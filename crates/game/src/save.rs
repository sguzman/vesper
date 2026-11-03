//! Versioned, game-bound saves. The Bevy player handles UI and async I/O.
//! Saves never serialize transient ECS entity identifiers.
use crate::{Game, experience::{DialogueSurface, ExperienceMode, ExperienceState, WorldPosition}};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::{self, Write}, path::Path,
    sync::atomic::{AtomicU64, Ordering}};
use vesper_story::{Machine, MachineSnapshot};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterPose {
    pub expression: String,
    pub position: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationState {
    pub background: Option<String>,
    /// Active per-scene panel override; None means game/theme default.
    #[serde(default)]
    pub panel_skin: Option<String>,
    /// Optional author surface override. Missing in version-one saved games:
    /// default to the current game's panel or bubble author preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialogue_surface: Option<DialogueSurface>,
    #[serde(default)]
    pub music: Option<String>,
    #[serde(default)]
    pub selected_choice: usize,
    pub characters: BTreeMap<String, CharacterPose>,
    /// Optional author-command overrides, keyed by external ambient actor ID.
    /// Legacy format-version 1 saves have no key and retain manifest defaults.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ambient_visibility: BTreeMap<String, bool>,
    /// Optional elapsed seconds for moving actors; missing in legacy saves.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ambient_motion_elapsed: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveData {
    pub format_version: u32,
    pub game_id: String,
    pub machine: MachineSnapshot,
    pub presentation: PresentationState,
    /// Version-two logical world state; absent in legacy visual-novel saves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<ExperienceState>,
}

/// Validate against the currently authored world before restoring a save or
/// accepting a live reload. World state never contains ECS entity IDs.
pub fn validate_experience(game: &Game, state: &ExperienceState) -> Result<(), String> {
    let position = match &state.mode {
        ExperienceMode::VisualNovel => return Ok(()),
        ExperienceMode::Exploring(position) => position,
        ExperienceMode::Conversation { scene, return_to } => {
            Machine::new(&game.story, scene)
                .map_err(|error| format!("saved interaction scene '{scene}' is invalid: {error}"))?;
            return_to
        }
    };
    WorldPosition::new(position.location.clone(), position.x, position.facing)?;
    let world = game.manifest.exploration.as_ref()
        .ok_or("saved exploration state requires authored locations")?;
    let area = world.locations.get(&position.location)
        .ok_or_else(|| format!("saved exploration location '{}' no longer exists",
            position.location))?;
    if position.x > area.width {
        return Err(format!("saved exploration position in '{}' exceeds its width",
            position.location));
    }
    Ok(())
}

impl SaveData {
    pub fn new(game: &Game, machine: &Machine, presentation: PresentationState) -> Self {
        Self {
            format_version: 1,
            game_id: game.manifest.id.clone(),
            machine: machine.snapshot(),
            presentation,
            experience: None,
        }
    }

    /// Pure VN games continue writing v1; optional exploration games opt in.
    pub fn with_experience(mut self, state: &ExperienceState) -> Self {
        self.format_version = 2;
        self.experience = Some(state.clone());
        self
    }

    pub fn restore_experience(&self, game: &Game) -> Result<ExperienceState, String> {
        if self.game_id != game.manifest.id {
            return Err("save belongs to a different game".into());
        }
        let state = match (self.format_version, &self.experience) {
            (1, None) => ExperienceState::default(),
            (2, Some(state)) => state.clone(),
            (1, Some(_)) => return Err("version-one save contains unexpected world state".into()),
            (2, None) => return Err("version-two save is missing world state".into()),
            (version, _) => return Err(format!("unsupported game save version {version}")),
        };
        validate_experience(game, &state)?;
        Ok(state)
    }

    pub fn restore(&self, game: &Game) -> Result<Machine, String> {
        self.restore_experience(game)?;
        if let Some(music) = &self.presentation.music {
            game.music(music)?;
        }
        if let Some(background) = &self.presentation.background {
            game.background(background)?;
        }
        if let Some(id) = &self.presentation.panel_skin {
            game.panel_skin(id)?;
        }
        for (id, elapsed) in &self.presentation.ambient_motion_elapsed {
            if !elapsed.is_finite() || !(0.0..=240.0).contains(elapsed)
                || !game.manifest.ambient_actors.get(id)
                    .is_some_and(|actor| actor.motion.is_some())
            {
                return Err(format!("invalid saved ambient motion clock '{id}'"));
            }
        }
        for id in self.presentation.ambient_visibility.keys() {
            if !game.manifest.ambient_actors.contains_key(id) {
                return Err(format!("saved ambient actor '{id}' no longer exists"));
            }
        }
        for (id, pose) in &self.presentation.characters {
            game.character_expression(id, &pose.expression)?;
            if !matches!(pose.position.as_str(), "left" | "center" | "right") {
                return Err(format!("invalid saved character position: {}", pose.position));
            }
        }
        let machine = Machine::restore(&game.story, self.machine.clone())?;
        if self.presentation.selected_choice != 0 {
            match machine.last_output() {
                Some(vesper_story::Output::Choices(choices))
                    if self.presentation.selected_choice < choices.len() => {},
                _ => return Err("saved choice selection is invalid".into()),
            }
        }
        Ok(machine)
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec_pretty(self).map_err(|error| error.to_string())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        const MAX_BYTES: usize = 8 * 1024 * 1024;
        if bytes.len() > MAX_BYTES {
            return Err("save file exceeds 8 MiB limit".into());
        }
        serde_json::from_slice(bytes).map_err(|error| format!("invalid save file: {error}"))
    }

    pub fn read(path: &Path) -> Result<Self, String> {
        let metadata = fs::metadata(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if metadata.len() > 8 * 1024 * 1024 {
            return Err("save file exceeds 8 MiB limit".into());
        }
        let bytes = fs::read(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Self::decode(&bytes)
    }

    pub fn write_atomic(&self, path: &Path) -> Result<(), String> {
        let parent = path.parent().ok_or("save path has no parent")?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
        let bytes = self.encode()?;
        let name = path.file_name().ok_or("save path has no file name")?;
        let mut temp = name.to_os_string();
        static TEMP_ID: AtomicU64 = AtomicU64::new(0);
        temp.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed),
        ));
        let tmp = parent.join(temp);
        let result: Result<(), io::Error> = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true).create_new(true).open(&tmp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&tmp, path)?;
            #[cfg(unix)]
            fs::File::open(parent)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result.map_err(|error| format!("{}: {error}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use vesper_story::Output;

    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let root = std::env::temp_dir().join(format!(
                "vesper-save-test-{}-{nonce}", std::process::id()
            ));
            fs::create_dir_all(root.join("scripts")).unwrap();
            fs::write(root.join("game.toml"),
                "format_version = 1\nid = \"demo\"\ntitle = \"Demo\"\nstory = \"scripts/a.vn\"\nentry = \"start\"\n"
            ).unwrap();
            fs::write(root.join("scripts/a.vn"),
                "@scene start\nset trust = 4\n\"Hello\"\n* \"Continue\" -> ending\n@scene ending\n\"End\"\n"
            ).unwrap();
            Self(root)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) { fs::remove_dir_all(&self.0).unwrap(); }
    }

    #[test]
    fn save_reopens_and_restores_choices_across_game_reload() {
        let fixture = Fixture::new();
        let game = Game::load(&fixture.0).unwrap();
        let mut machine = Machine::new(&game.story, &game.manifest.entry).unwrap();
        machine.advance().unwrap();
        assert!(matches!(machine.advance().unwrap(), Output::Choices(_)));
        let path = fixture.0.join("saves/slot.json");
        SaveData::new(&game, &machine, PresentationState::default())
            .write_atomic(&path).unwrap();
        let second_game = Game::load(&fixture.0).unwrap();
        let file = SaveData::read(&path).unwrap();
        let mut restored = file.restore(&second_game).unwrap();
        assert_eq!(restored.variables()["trust"], vesper_story::Value::Number(4));
        restored.choose(0).unwrap();
        assert_eq!(restored.advance().unwrap(), Output::Dialogue {
            speaker: None, text: "End".into(),
        });
    }

    #[test]
    fn old_save_schema_without_skin_stays_compatible() {
        let fixture = Fixture::new();
        let game = Game::load(&fixture.0).unwrap();
        let machine = Machine::new(&game.story, &game.manifest.entry).unwrap();
        let current = SaveData::new(&game, &machine, PresentationState::default());
        let json = String::from_utf8(current.encode().unwrap()).unwrap();
        assert!(json.contains("panel_skin"));
        let old = json.replace("    \"panel_skin\": null,\n", "");
        let decoded = SaveData::decode(old.as_bytes()).unwrap();
        assert_eq!(decoded.presentation.panel_skin, None);
    }

    #[test]
    fn optional_surface_roundtrip_and_old_save_default_compatibility() {
        let fixture = Fixture::new();
        let game = Game::load(&fixture.0).unwrap();
        let machine = Machine::new(&game.story, &game.manifest.entry).unwrap();
        let state = PresentationState {
            dialogue_surface: Some(DialogueSurface::Bubble),
            ..Default::default()
        };
        let save = SaveData::new(&game, &machine, state);
        let bytes = save.encode().unwrap();
        let decoded = SaveData::decode(&bytes).unwrap();
        assert_eq!(decoded.presentation.dialogue_surface, Some(DialogueSurface::Bubble));
        decoded.restore(&game).unwrap();
        let mut legacy = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap();
        legacy["presentation"].as_object_mut().unwrap().remove("dialogue_surface");
        let older = SaveData::decode(&serde_json::to_vec(&legacy).unwrap()).unwrap();
        assert_eq!(older.presentation.dialogue_surface, None);
        older.restore(&game).unwrap();
    }

    #[test]
    fn ambient_motion_clock_persists_and_legacy_saves_remain_compatible() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("bird.png"), b"external test art").unwrap();
        let manifest = fixture.0.join("game.toml");
        let mut source = fs::read_to_string(&manifest).unwrap();
        source.push_str(
            "\n[ambient_actors.bird]\nimage = \"bird.png\"\n\
             [ambient_actors.bird.motion]\nwaypoints = [[0.1,0.5],[0.9,0.5]]\n\
             period_seconds = 5.0\nmode = \"loop\"\n"
        );
        fs::write(&manifest, source).unwrap();
        let game = Game::load(&fixture.0).unwrap();
        let machine = Machine::new(&game.story, &game.manifest.entry).unwrap();
        let mut state = PresentationState::default();
        state.ambient_motion_elapsed.insert("bird".into(), 2.25);
        let current = SaveData::new(&game, &machine, state);
        let json = current.encode().unwrap();
        let decoded = SaveData::decode(&json).unwrap();
        assert_eq!(decoded.presentation.ambient_motion_elapsed["bird"], 2.25);
        assert!(decoded.restore(&game).is_ok());

        // Old version-one saves omit the clock field entirely.
        let mut legacy = serde_json::from_slice::<serde_json::Value>(&json).unwrap();
        legacy["presentation"].as_object_mut().unwrap()
            .remove("ambient_motion_elapsed");
        let legacy_bytes = serde_json::to_vec(&legacy).unwrap();
        let old = SaveData::decode(&legacy_bytes).unwrap();
        assert!(old.presentation.ambient_motion_elapsed.is_empty());
        assert!(old.restore(&game).is_ok());

        for invalid in [-0.01, 240.1, f32::NAN, f32::INFINITY] {
            let mut tampered = decoded.clone();
            tampered.presentation.ambient_motion_elapsed.insert("bird".into(), invalid);
            assert!(tampered.restore(&game).unwrap_err().contains("ambient motion clock"));
        }
        let mut missing_actor = decoded.clone();
        missing_actor.presentation.ambient_motion_elapsed
            .insert("unknown".into(), 1.0);
        assert!(missing_actor.restore(&game).is_err());
    }

    #[test]
    fn version_two_world_state_roundtrips_and_rejects_stale_positions() {
        use crate::experience::{ExperienceMode, ExperienceState, WorldPosition};
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/exploration-schema");
        let game = Game::load(&root).unwrap();
        let machine = Machine::new(&game.story, &game.manifest.entry).unwrap();
        let mut state = ExperienceState::default();
        let start = game.manifest.exploration.as_ref().unwrap()
            .start_position().unwrap();
        state.enter_exploration(start.clone());
        let saved = SaveData::new(&game, &machine, PresentationState::default())
            .with_experience(&state);
        let encoded = saved.encode().unwrap();
        let loaded = SaveData::decode(&encoded).unwrap();
        assert_eq!(loaded.format_version, 2);
        assert_eq!(loaded.restore_experience(&game).unwrap(), state);
        loaded.restore(&game).unwrap();
        assert!(!String::from_utf8_lossy(&encoded).contains("entity_id"));

        state.begin_interaction("neighbor_chat").unwrap();
        let modal = SaveData::new(&game, &machine, PresentationState::default())
            .with_experience(&state);
        assert_eq!(modal.restore_experience(&game).unwrap(), state);

        let mut invalid = modal.clone();
        invalid.experience.as_mut().unwrap().mode =
            ExperienceMode::Exploring(WorldPosition {
                location: "unknown".into(), ..start.clone()
            });
        assert!(invalid.restore(&game).unwrap_err().contains("no longer exists"));
        invalid.experience.as_mut().unwrap().mode =
            ExperienceMode::Exploring(WorldPosition {
                x: 1_000_000.0, ..start.clone()
            });
        assert!(invalid.restore(&game).unwrap_err().contains("exceeds its width"));
        invalid.experience.as_mut().unwrap().mode =
            ExperienceMode::Exploring(WorldPosition {
                x: f32::INFINITY, ..start.clone()
            });
        assert!(invalid.restore(&game).is_err());
        invalid.experience.as_mut().unwrap().mode =
            ExperienceMode::Conversation {
                scene: "missing_scene".into(), return_to: start
            };
        assert!(invalid.restore(&game).unwrap_err().contains("interaction scene"));
        let mut absent = modal.clone();
        absent.experience = None;
        assert!(absent.restore(&game).unwrap_err().contains("missing world state"));
        let mut wrong_version = modal;
        wrong_version.format_version = 1;
        assert!(wrong_version.restore(&game).unwrap_err().contains("unexpected world state"));
    }

    #[test]
    fn original_save_schema_remains_a_pure_vn_without_migration() {
        let fixture = Fixture::new();
        let game = Game::load(&fixture.0).unwrap();
        let machine = Machine::new(&game.story, &game.manifest.entry).unwrap();
        let original = SaveData::new(&game, &machine, PresentationState::default());
        let encoded = original.encode().unwrap();
        assert!(!String::from_utf8_lossy(&encoded).contains("\"experience\""));
        let restored = SaveData::decode(&encoded).unwrap();
        assert_eq!(restored.format_version, 1);
        assert_eq!(restored.restore_experience(&game).unwrap().mode,
            ExperienceMode::VisualNovel);
        restored.restore(&game).unwrap();
    }

    #[test]
    fn rejects_incompatible_or_invalid_saves() {
        let fixture = Fixture::new();
        let game = Game::load(&fixture.0).unwrap();
        let machine = Machine::new(&game.story, &game.manifest.entry).unwrap();
        let mut save = SaveData::new(&game, &machine, PresentationState::default());
        save.game_id = "other".into();
        assert!(save.restore(&game).unwrap_err().contains("different game"));
        save.game_id = game.manifest.id.clone();
        save.presentation.background = Some("unregistered".into());
        assert!(save.restore(&game).unwrap_err().contains("background"));
        assert!(SaveData::decode(&vec![0; 8 * 1024 * 1024 + 1]).is_err());
    }
}
