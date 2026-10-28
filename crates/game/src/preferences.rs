//! Player-owned visual preferences, separate from authored themes and story saves.
//! Call read/write methods outside Bevy's Update/Render schedules.
use crate::Theme;
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub const CURRENT_VERSION: u32 = 1;
pub const MAX_PREFERENCES_BYTES: usize = 64 * 1024;

/// None means "follow the current game's theme". Overrides never alter TOML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlayerPreferences {
    /// Version 0 (or a missing version) is the initial compatible draft.
    #[serde(default)]
    pub format_version: u32,
    pub dialogue_opacity: Option<f32>,
    pub dialogue_font_scale: Option<f32>,
    /// A system font family, never a user-supplied filesystem path.
    pub font_family: Option<String>,
}

impl Default for PlayerPreferences {
    fn default() -> Self {
        Self {
            format_version: CURRENT_VERSION,
            dialogue_opacity: None,
            dialogue_font_scale: None,
            font_family: None,
        }
    }
}

impl PlayerPreferences {
    pub fn reset_to_game_defaults(&mut self) {
        self.dialogue_opacity = None;
        self.dialogue_font_scale = None;
        self.font_family = None;
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != CURRENT_VERSION {
            return Err(format!("unsupported preference version {}", self.format_version));
        }
        if self.dialogue_opacity.is_some_and(|value| {
            !value.is_finite() || !(0.60..=1.0).contains(&value)
        }) {
            return Err("dialogue_opacity must be in 0.60..=1.00".into());
        }
        if self.dialogue_font_scale.is_some_and(|value| {
            !value.is_finite() || !(0.80..=1.60).contains(&value)
        }) {
            return Err("dialogue_font_scale must be in 0.80..=1.60".into());
        }
        if self.font_family.as_ref().is_some_and(|name| {
            name.trim() != name
                || name.is_empty()
                || name.len() > 128
                || name.chars().any(|c| c.is_control() || c == '/' || c == '\\')
        }) {
            return Err("font_family must be a simple nonempty system family name".into());
        }
        Ok(())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_PREFERENCES_BYTES {
            return Err("preferences exceed the 64 KiB limit".into());
        }
        let mut prefs: Self = serde_json::from_slice(bytes)
            .map_err(|error| format!("invalid preferences JSON: {error}"))?;
        match prefs.format_version {
            0 => prefs.format_version = CURRENT_VERSION,
            CURRENT_VERSION => {}
            other => return Err(format!("unsupported preference version {other}")),
        }
        prefs.validate()?;
        Ok(prefs)
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        if bytes.len() > MAX_PREFERENCES_BYTES {
            return Err("preferences exceed the 64 KiB limit".into());
        }
        Ok(bytes)
    }

    pub fn read(path: &Path) -> Result<Self, String> {
        let metadata = fs::metadata(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if metadata.len() > MAX_PREFERENCES_BYTES as u64 {
            return Err("preferences exceed the 64 KiB limit".into());
        }
        let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        Self::decode(&bytes)
    }

    /// Invalid, outdated or unreadable files do not prevent the game opening.
    /// The diagnostic is returned for logging; the old file is never deleted.
    pub fn read_or_default(path: &Path) -> (Self, Option<String>) {
        match Self::read(path) {
            Ok(prefs) => (prefs, None),
            Err(_error) if !path.exists() => (Self::default(), None),
            Err(error) => (Self::default(), Some(error)),
        }
    }

    /// For a settings event, copy the validated snapshot to an I/O worker.
    /// This function does synchronous I/O and MUST NOT run in Bevy Update.
    pub fn write_atomic(&self, path: &Path) -> Result<(), String> {
        let bytes = self.encode()?;
        let parent = path.parent().ok_or("preferences path has no parent")?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
        let file_name = path.file_name().ok_or("preferences path has no file name")?;
        static NEXT_TMP: AtomicU64 = AtomicU64::new(0);
        let mut temp_name = file_name.to_os_string();
        temp_name.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            NEXT_TMP.fetch_add(1, Ordering::Relaxed),
        ));
        let temporary = parent.join(temp_name);
        let result: io::Result<()> = (|| {
            let mut file = fs::OpenOptions::new()
                .create_new(true).write(true).open(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, path)?;
            #[cfg(unix)]
            fs::File::open(parent)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(|error| format!("{}: {error}", path.display()))
    }

    /// Returns a new effective theme each time preferences or authored TOML change.
    /// The caller supplies the font-discovery result; a missing family keeps
    /// the game's own font (or the engine's sans-serif default).
    pub fn resolve_theme(
        &self,
        authored: &Theme,
        system_font_exists: impl Fn(&str) -> bool,
    ) -> Theme {
        let mut effective = authored.clone();
        // Reject in-memory edits too, not only preferences loaded from JSON.
        if self.validate().is_err() {
            return effective;
        }
        if let Some(alpha) = self.dialogue_opacity {
            effective.panel.background[3] = alpha;
        }
        if let Some(scale) = self.dialogue_font_scale {
            // Authored values are validated to 8..=96; never exceed their cap.
            for text in [
                &mut effective.speaker,
                &mut effective.dialogue,
                &mut effective.choices,
            ] {
                text.size = (text.size * scale).clamp(8.0, 96.0);
            }
        }
        if let Some(family) = self.font_family.as_deref()
            && system_font_exists(family)
        {
            effective.font.file = None;
            effective.font.family = Some(family.to_owned());
        }
        effective
    }
}

/// Pure path selection for consistent XDG behavior and independently testable
/// override priority. Preferences never share a directory with story saves.
pub fn preferences_path_for(
    override_file: Option<&Path>,
    xdg_config_home: Option<&Path>,
    home: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(path) = override_file.filter(|path| !path.as_os_str().is_empty()) {
        return Ok(path.to_path_buf());
    }
    if let Some(xdg) = xdg_config_home.filter(|path| path.is_absolute()) {
        return Ok(xdg.join("vesper/preferences.json"));
    }
    if let Some(home) = home.filter(|path| !path.as_os_str().is_empty()) {
        return Ok(home.join(".config/vesper/preferences.json"));
    }
    Err("preferences location unavailable (set VESPER_PREFERENCES_FILE)".into())
}

pub fn preferences_path() -> Result<PathBuf, String> {
    let override_file = env::var_os("VESPER_PREFERENCES_FILE").map(PathBuf::from);
    let xdg = env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    let home = env::var_os("HOME").map(PathBuf::from);
    preferences_path_for(override_file.as_deref(), xdg.as_deref(), home.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let root = env::temp_dir()
                .join(format!("vesper-prefs-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) { fs::remove_dir_all(&self.0).unwrap(); }
    }

    #[test]
    fn round_trip_is_atomic_and_keeps_saves_separate() {
        let fixture = Fixture::new();
        let path = fixture.0.join("config/vesper/preferences.json");
        let save = fixture.0.join("data/vesper/saves/demo.json");
        fs::create_dir_all(save.parent().unwrap()).unwrap();
        fs::write(&save, b"existing story save").unwrap();
        let prefs = PlayerPreferences {
            dialogue_opacity: Some(0.72),
            dialogue_font_scale: Some(1.30),
            font_family: Some("DejaVu Sans".into()),
            ..Default::default()
        };
        prefs.write_atomic(&path).unwrap();
        assert_eq!(PlayerPreferences::read(&path).unwrap(), prefs);
        assert_eq!(fs::read(save).unwrap().as_slice(), &b"existing story save"[..]);
        assert!(fs::read_dir(path.parent().unwrap()).unwrap().all(|item| {
            item.unwrap().path().extension().is_some_and(|extension| extension == "json")
        }));
    }

    #[test]
    fn malformed_missing_and_unsupported_files_fall_back_without_deletion() {
        let fixture = Fixture::new();
        let path = fixture.0.join("prefs.json");
        assert_eq!(PlayerPreferences::read_or_default(&path), (PlayerPreferences::default(), None));
        fs::write(&path, b"{bad json").unwrap();
        let (prefs, diagnostic) = PlayerPreferences::read_or_default(&path);
        assert_eq!(prefs, PlayerPreferences::default());
        assert!(diagnostic.unwrap().contains("invalid preferences JSON"));
        assert_eq!(fs::read(&path).unwrap().as_slice(), &b"{bad json"[..]);
        assert!(PlayerPreferences::decode(br#"{"format_version":99}"#).is_err());
        assert!(PlayerPreferences::decode(&vec![b' '; MAX_PREFERENCES_BYTES + 1]).is_err());
    }

    #[test]
    fn migration_from_unversioned_draft_is_explicit() {
        let prefs = PlayerPreferences::decode(
            br#"{"dialogue_opacity":0.8,"dialogue_font_scale":1.1}"#
        ).unwrap();
        assert_eq!(prefs.format_version, CURRENT_VERSION);
        assert_eq!(prefs.dialogue_opacity, Some(0.8));
        assert_eq!(PlayerPreferences::decode(&prefs.encode().unwrap()).unwrap(), prefs);
    }

    #[test]
    fn invalid_numeric_values_or_untrusted_font_paths_fail_validation() {
        for json in [
            r#"{"format_version":1,"dialogue_opacity":0.3}"#,
            r#"{"format_version":1,"dialogue_font_scale":9}"#,
            r#"{"format_version":1,"font_family":"../outside.ttf"}"#,
            r#"{"format_version":1,"font_family":""}"#,
        ] {
            assert!(PlayerPreferences::decode(json.as_bytes()).is_err(), "{json}");
        }
        let mut prefs = PlayerPreferences {
            dialogue_opacity: Some(f32::NAN),
            ..Default::default()
        };
        assert!(prefs.encode().is_err());
        prefs.dialogue_opacity = None;
        prefs.dialogue_font_scale = Some(f32::INFINITY);
        assert!(prefs.encode().is_err());
    }

    #[test]
    fn player_overrides_follow_the_active_game_and_reset_releases_them() {
        let mut lantern = Theme::default();
        lantern.panel.background[3] = 0.90;
        lantern.dialogue.size = 25.0;
        lantern.font.family = Some("Lantern Game Font".into());
        let mut moon = Theme::default();
        moon.panel.background[3] = 0.98;
        moon.dialogue.size = 31.0;
        moon.font.family = Some("Moon Game Font".into());
        let mut prefs = PlayerPreferences {
            dialogue_opacity: Some(0.75),
            dialogue_font_scale: Some(1.20),
            font_family: Some("Reader Font".into()),
            ..Default::default()
        };
        let l = prefs.resolve_theme(&lantern, |name| name == "Reader Font");
        let m = prefs.resolve_theme(&moon, |name| name == "Reader Font");
        assert_eq!(l.panel.background[3], 0.75);
        assert_eq!(m.panel.background[3], 0.75);
        assert!((l.dialogue.size - 30.0).abs() < 0.001);
        assert!((m.dialogue.size - 37.2).abs() < 0.001);
        assert_eq!(l.font.family.as_deref(), Some("Reader Font"));
        assert_eq!(m.font.family.as_deref(), Some("Reader Font"));
        assert_eq!(lantern.dialogue.size, 25.0);
        assert_eq!(moon.dialogue.size, 31.0);

        // A missing system font must retain the game font; not an empty handle.
        let fallback = prefs.resolve_theme(&lantern, |_| false);
        assert_eq!(fallback.font.family.as_deref(), Some("Lantern Game Font"));
        prefs.reset_to_game_defaults();
        assert_eq!(prefs.resolve_theme(&lantern, |_| true).dialogue.size, 25.0);
        assert_eq!(prefs.resolve_theme(&moon, |_| true).panel.background[3], 0.98);
    }

    #[test]
    fn xdg_config_path_and_explicit_override_are_isolated_from_saves() {
        let xdg = Path::new("/tmp/xdg");
        let home = Path::new("/home/test");
        assert_eq!(preferences_path_for(None, Some(xdg), Some(home)).unwrap(),
            xdg.join("vesper/preferences.json"));
        assert_eq!(preferences_path_for(None, Some(Path::new("relative")), Some(home)).unwrap(),
            home.join(".config/vesper/preferences.json"));
        assert_eq!(preferences_path_for(Some(Path::new("/tmp/custom.json")), Some(xdg), Some(home)).unwrap(),
            PathBuf::from("/tmp/custom.json"));
        assert!(preferences_path_for(None, None, None).is_err());
    }
}
