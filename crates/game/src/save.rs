//! Versioned, game-bound saves. The Bevy player handles UI and async I/O.
//! Saves never serialize transient ECS entity identifiers.
use crate::Game;
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

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationState {
    pub background: Option<String>,
    /// Active per-scene panel override; None means game/theme default.
    #[serde(default)]
    pub panel_skin: Option<String>,
    #[serde(default)]
    pub music: Option<String>,
    #[serde(default)]
    pub selected_choice: usize,
    pub characters: BTreeMap<String, CharacterPose>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveData {
    pub format_version: u32,
    pub game_id: String,
    pub machine: MachineSnapshot,
    pub presentation: PresentationState,
}

impl SaveData {
    pub fn new(game: &Game, machine: &Machine, presentation: PresentationState) -> Self {
        Self {
            format_version: 1,
            game_id: game.manifest.id.clone(),
            machine: machine.snapshot(),
            presentation,
        }
    }

    pub fn restore(&self, game: &Game) -> Result<Machine, String> {
        if self.format_version != 1 {
            return Err(format!("unsupported game save version {}", self.format_version));
        }
        if self.game_id != game.manifest.id {
            return Err("save belongs to a different game".into());
        }
        if let Some(music) = &self.presentation.music {
            game.music(music)?;
        }
        if let Some(background) = &self.presentation.background {
            game.background(background)?;
        }
        if let Some(id) = &self.presentation.panel_skin {
            game.panel_skin(id)?;
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
