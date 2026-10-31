//! Player-owned visual preferences. Authored game themes remain immutable inputs.
//! No file I/O belongs in Bevy Update/Render systems; callers read at startup
//! and dispatch writes to the existing asynchronous I/O worker model.

use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{TextRevealSpeed, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextInk { Black, DarkBrown, Navy, Sepia }
impl TextInk {
    pub fn rgb(self) -> [f32; 3] {
        match self {
            Self::Black => [0.015, 0.015, 0.015],
            Self::DarkBrown => [0.085, 0.050, 0.030],
            Self::Navy => [0.045, 0.075, 0.125],
            Self::Sepia => [0.23, 0.13, 0.060],
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Black => "Black", Self::DarkBrown => "Dark brown",
            Self::Navy => "Navy", Self::Sepia => "Sepia",
        }
    }
}

pub const VERSION: u32 = 1;
const MAX_BYTES: u64 = 16 * 1024;
static NEXT_WRITE: AtomicU64 = AtomicU64::new(0);

/// Explicit overrides only. None means "follow the author's live theme".
/// This file never contains narrative, scene positions, or game save slots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerPreferences {
    #[serde(default)]
    pub format_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel_opacity: Option<f32>,
    /// Reader-selected dialogue width; never edits the game's TOML.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel_width_percent: Option<f32>,
    /// Percentage of viewport height from its bottom edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel_bottom_percent: Option<f32>,
    /// Reader's chosen dialogue ink; None keeps the authored palette.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_ink: Option<TextInk>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reveal_speed: Option<TextRevealSpeed>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduced_motion: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_advance: Option<bool>,
    /// Seconds to hold completed dialogue when auto-advance is enabled.
    /// None retains the original 1.5-second player default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_advance_delay: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_contrast: Option<bool>,
}

impl Default for PlayerPreferences {
    fn default() -> Self {
        Self {
            format_version: VERSION,
            panel_opacity: None,
            panel_width_percent: None,
            panel_bottom_percent: None,
            text_ink: None,
            text_scale: None,
            font_family: None,
            reveal_speed: None,
            reduced_motion: None,
            auto_advance: None,
            auto_advance_delay: None,
            high_contrast: None,
        }
    }
}

/// A user-supplied family must be checked against the host's available font
/// choices before applying. Generic families always provide a safe fallback;
/// other families may be supplied by an explicit, verified font picker later.
pub fn generic_font_available(name: &str) -> bool {
    matches!(name, "sans-serif" | "serif" | "monospace")
}

impl PlayerPreferences {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != VERSION {
            return Err(format!("unsupported player preference version {}", self.format_version));
        }
        if self.panel_opacity.is_some_and(|v| !v.is_finite() || !(0.60..=1.0).contains(&v)) {
            return Err("panel_opacity must be within 0.60..=1.00".into());
        }
        if self.panel_width_percent.is_some_and(|v| !v.is_finite() || !(50.0..=100.0).contains(&v)) {
            return Err("panel_width_percent must be within 50..=100".into());
        }
        if self.panel_bottom_percent.is_some_and(|v| !v.is_finite() || !(0.0..=20.0).contains(&v)) {
            return Err("panel_bottom_percent must be within 0..=20".into());
        }
        if self.text_scale.is_some_and(|v| !v.is_finite() || !(0.80..=1.60).contains(&v)) {
            return Err("text_scale must be within 0.80..=1.60".into());
        }
        if self.auto_advance_delay.is_some_and(|delay|
            !delay.is_finite() || !(0.5..=20.0).contains(&delay)) {
            return Err("auto_advance_delay must be within 0.5..=20.0 seconds".into());
        }
        if self.font_family.as_ref().is_some_and(|font| {
            font.trim() != font || font.is_empty() || font.len() > 128
                || font.chars().any(char::is_control)
        }) {
            return Err("font_family must be a nonempty, safe name up to 128 bytes".into());
        }
        Ok(())
    }

    /// V0 is the original pre-versioned JSON shape with the same optional
    /// properties. This migration adds a schema version but retains overrides.
    /// Future unknown versions are never silently interpreted as V1.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() as u64 > MAX_BYTES {
            return Err("player preferences exceed 16 KiB limit".into());
        }
        let mut prefs: Self =
            serde_json::from_slice(bytes).map_err(|err| format!("invalid preferences: {err}"))?;
        if prefs.format_version == 0 {
            prefs.format_version = VERSION;
        }
        prefs.validate()?;
        Ok(prefs)
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|err| err.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("player preferences exceed 16 KiB limit".into());
        }
        Ok(bytes)
    }

    pub fn read(path: &Path) -> Result<Self, String> {
        let meta = fs::metadata(path).map_err(|err| format!("{}: {err}", path.display()))?;
        if !meta.is_file() || meta.len() > MAX_BYTES {
            return Err("player preference file is not regular or exceeds 16 KiB".into());
        }
        let bytes = fs::read(path).map_err(|err| format!("{}: {err}", path.display()))?;
        Self::decode(&bytes)
    }

    /// Missing/unsupported/corrupt files never prevent opening a game.
    /// Return the diagnostic so callers can log it without altering the file.
    pub fn load_or_default(path: &Path) -> (Self, Option<String>) {
        match Self::read(path) {
            Ok(value) => (value, None),
            Err(error) if error.contains("No such file or directory") => (Self::default(), None),
            Err(error) => (Self::default(), Some(error)),
        }
    }

    /// Call only from an I/O worker or when Bevy is not running. Uses a fresh
    /// exclusive temporary file, fsync, atomic replace and directory sync.
    pub fn write_atomic(&self, path: &Path) -> Result<(), String> {
        let bytes = self.encode()?;
        let parent = path.parent().ok_or("invalid preferences path")?;
        fs::create_dir_all(parent).map_err(|err| format!("{}: {err}", parent.display()))?;
        let filename = path.file_name().ok_or("invalid preferences filename")?;
        let mut tmp_name = filename.to_os_string();
        tmp_name.push(format!(
            ".{}.{}.tmp", std::process::id(),
            NEXT_WRITE.fetch_add(1, Ordering::Relaxed)
        ));
        let temp = parent.join(tmp_name);
        let result: io::Result<()> = (|| {
            let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp, path)?;
            #[cfg(unix)]
            fs::File::open(parent)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() { let _ = fs::remove_file(&temp); }
        result.map_err(|err| format!("{}: {err}", path.display()))
    }

    /// Resolves player > author > engine defaults without changing the author.
    /// Unavailable user fonts are ignored; authored font continues to work.
    pub fn resolve_theme(&self, authored: &Theme, available: impl Fn(&str) -> bool) -> Theme {
        let mut result = authored.clone();
        if self.validate().is_err() { return result; }
        if let Some(opacity) = self.panel_opacity {
            result.panel.background[3] = opacity;
        }
        if let Some(width) = self.panel_width_percent {
            result.panel.width_percent = width;
        }
        if let Some(position) = self.panel_bottom_percent {
            result.panel.bottom_percent = position;
        }
        if let Some(ink) = self.text_ink {
            let rgb = ink.rgb();
            // Speaker names are on illustrated title plates; preserve their
            // authored contrasting gold instead of making black ink invisible
            // on a navy nameplate.
            for style in [&mut result.dialogue, &mut result.choices, &mut result.hint] {
                style.color = [rgb[0], rgb[1], rgb[2], 1.0];
            }
        }
        if let Some(scale) = self.text_scale {
            for style in [
                &mut result.speaker, &mut result.dialogue, &mut result.choices, &mut result.hint,
            ] {
                style.size = (style.size * scale).clamp(8.0, 96.0);
            }
        }
        if let Some(family) = self.font_family.as_deref().filter(|family| available(family)) {
            result.font.file = None;
            result.font.family = Some(family.to_owned());
        }
        if self.high_contrast.unwrap_or(false) {
            // Player accessibility wins over authored transparency and color,
            // without ever changing or writing the author's theme.
            result.panel.background = [0.015, 0.020, 0.035, 1.0];
            result.panel.divider = [1.0, 1.0, 1.0, 0.85];
            result.panel.accent = [1.0, 0.85, 0.22, 1.0];
            for style in [
                &mut result.speaker, &mut result.dialogue,
                &mut result.choices, &mut result.hint,
            ] {
                style.color = [1.0, 1.0, 1.0, 1.0];
            }
        }
        result
    }
}

/// VESPER_PREFERENCES_DIR is for isolated testing/portable installations.
/// Otherwise use XDG_CONFIG_HOME; importantly this is NOT the save directory.
pub fn default_file() -> Result<PathBuf, String> {
    let parent = if let Some(dir) = env::var_os("VESPER_PREFERENCES_DIR") {
        PathBuf::from(dir)
    } else if let Some(dir) = env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(dir).join("vesper")
    } else if let Some(dir) = env::var_os("HOME") {
        PathBuf::from(dir).join(".config/vesper")
    } else {
        return Err("preferences location unavailable (set VESPER_PREFERENCES_DIR)".into());
    };
    Ok(parent.join("player-preferences.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auto_advance_delay_is_optional_bounded_and_backward_compatible() {
        let previous = PlayerPreferences::decode(
            br#"{"format_version":1,"auto_advance":true}"#,
        ).unwrap();
        assert_eq!(previous.auto_advance_delay, None);
        for invalid in [-1.0, 0.0, 0.49, 20.5, f32::NAN, f32::INFINITY] {
            let prefs = PlayerPreferences {
                auto_advance_delay: Some(invalid), ..Default::default()
            };
            assert!(prefs.validate().is_err(), "accepted invalid delay {invalid}");
        }
        for valid in [0.5, 1.5, 4.5, 20.0] {
            let prefs = PlayerPreferences {
                auto_advance_delay: Some(valid), ..Default::default()
            };
            let bytes = prefs.encode().unwrap();
            let restored = PlayerPreferences::decode(&bytes).unwrap();
            assert_eq!(restored.auto_advance_delay, Some(valid));
        }
    }

    use crate::Theme;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let path = env::temp_dir().join(format!(
                "vesper-preferences-{}-{nanos}", std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn file(&self) -> PathBuf { self.0.join("prefs.json") }
    }
    impl Drop for Fixture {
        fn drop(&mut self) { fs::remove_dir_all(&self.0).unwrap(); }
    }

    #[test]
    fn atomic_round_trip_and_reset() {
        let fixture = Fixture::new();
        let path = fixture.file();
        let mut p = PlayerPreferences {
            panel_opacity: Some(0.78), text_scale: Some(1.3),
            font_family: Some("monospace".into()), ..Default::default()
        };
        p.write_atomic(&path).unwrap();
        assert_eq!(PlayerPreferences::read(&path).unwrap(), p);
        assert!(!fs::read_dir(&fixture.0).unwrap()
            .any(|entry| entry.unwrap().file_name().to_string_lossy().contains(".tmp")));
        p.reset();
        assert_eq!(p, PlayerPreferences::default());
        p.write_atomic(&path).unwrap();
        assert_eq!(PlayerPreferences::read(&path).unwrap(), p);
    }

    #[test]
    fn limits_corruption_unknown_version_and_legacy_migration() {
        for json in [
            r#"{"format_version":7}"#, r#"{"format_version":1,"panel_opacity":0.2}"#,
            r#"{"format_version":1,"text_scale":99}"#,
            r#"{"format_version":1,"font_family":"bad\nname"}"#,
            r#"{"format_version":1,"unexpected":0}"#, r#"not json"#,
        ] {
            assert!(PlayerPreferences::decode(json.as_bytes()).is_err(), "{json}");
        }
        assert!(PlayerPreferences::decode(&vec![b' '; 16 * 1024 + 1]).is_err());
        let p = PlayerPreferences::decode(br#"{"panel_opacity":0.85,"text_scale":1.2}"#).unwrap();
        assert_eq!(p.format_version, VERSION);
        assert_eq!(p.panel_opacity, Some(0.85));
        assert_eq!(p.text_scale, Some(1.2));
        assert_eq!(PlayerPreferences::decode(&p.encode().unwrap()).unwrap(), p);

        let fixture = Fixture::new();
        let path = fixture.file();
        let (defaults, warning) = PlayerPreferences::load_or_default(&path);
        assert_eq!(defaults, PlayerPreferences::default());
        assert!(warning.is_none());
        fs::write(&path, b"bad").unwrap();
        let (defaults, warning) = PlayerPreferences::load_or_default(&path);
        assert_eq!(defaults, PlayerPreferences::default());
        assert!(warning.is_some());
        assert_eq!(fs::read(&path).unwrap(), b"bad");
    }

    #[test]
    fn player_precedence_font_fallback_theme_reloading_and_game_switches() {
        let mut warm = Theme::default();
        warm.panel.background[3] = 0.94;
        warm.dialogue.size = 28.0;
        warm.font.family = Some("DejaVu Sans".into());
        let mut cool = warm.clone();
        cool.dialogue.size = 25.0;
        cool.panel.background[3] = 0.91;
        let prefs = PlayerPreferences {
            panel_opacity: Some(0.70), text_scale: Some(1.25),
            font_family: Some("Missing Font".into()), ..Default::default()
        };
        // Missing font falls back to the authored font (not a crash).
        let warm_effective = prefs.resolve_theme(&warm, generic_font_available);
        assert_eq!(warm_effective.panel.background[3], 0.70);
        assert_eq!(warm_effective.dialogue.size, 35.0);
        assert_eq!(warm_effective.font.family.as_deref(), Some("DejaVu Sans"));
        assert_eq!(warm.panel.background[3], 0.94);

        // Same player preferences apply to another game; no per-game edits.
        let cool_effective = prefs.resolve_theme(&cool, generic_font_available);
        assert_eq!(cool_effective.dialogue.size, 31.25);
        assert_eq!(cool_effective.panel.background[3], 0.70);
        assert_eq!(cool.panel.background[3], 0.91);

        let pref_font = PlayerPreferences {
            font_family: Some("monospace".into()), ..Default::default()
        };
        let named = pref_font.resolve_theme(&warm, generic_font_available);
        assert_eq!(named.font.family.as_deref(), Some("monospace"));

        // If no override exists, an authored theme hot-reload takes effect.
        let plain = PlayerPreferences::default();
        let mut updated_author = warm.clone();
        updated_author.dialogue.size = 44.0;
        assert_eq!(plain.resolve_theme(&updated_author, generic_font_available).dialogue.size, 44.0);
        assert_eq!(plain.resolve_theme(&warm, generic_font_available).dialogue.size, 28.0);
    }

    #[test]
    fn high_contrast_is_player_owned_and_compatible_with_old_preference_files() {
        let authored = Theme::default();
        let original = authored.clone();
        let old = PlayerPreferences::decode(br#"{"format_version":1,"panel_opacity":0.7}"#)
            .unwrap();
        assert_eq!(old.high_contrast, None);
        let p = PlayerPreferences {
            panel_opacity: Some(0.60),
            high_contrast: Some(true),
            ..Default::default()
        };
        let resolved = p.resolve_theme(&authored, generic_font_available);
        assert_eq!(resolved.panel.background[3], 1.0);
        assert_eq!(resolved.dialogue.color, [1.0; 4]);
        assert_eq!(resolved.hint.color, [1.0; 4]);
        assert_eq!(resolved.panel.accent[3], 1.0);
        assert_eq!(authored.panel.background, original.panel.background);
        assert_eq!(PlayerPreferences::decode(&p.encode().unwrap()).unwrap(), p);
    }

    #[test]
    fn atomic_validation_failure_leaves_existing_file_intact() {
        let fixture = Fixture::new();
        let path = fixture.file();
        let initial = PlayerPreferences::default();
        initial.write_atomic(&path).unwrap();
        let original = fs::read(&path).unwrap();
        let invalid = PlayerPreferences {
            panel_opacity: Some(f32::NAN), ..Default::default()
        };
        assert!(invalid.write_atomic(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
    }

    #[test]
    fn reader_width_preference_is_validated_saved_and_resolved() {
        let author = Theme::default();
        let reader = PlayerPreferences {
            panel_width_percent: Some(65.0), ..Default::default()
        };
        assert_eq!(reader.resolve_theme(&author, generic_font_available).panel.width_percent, 65.0);
        assert_eq!(author.panel.width_percent, 90.0);
        assert_eq!(PlayerPreferences::decode(&reader.encode().unwrap()).unwrap(), reader);
        for bad in [f32::NAN, 49.0, 101.0] {
            let invalid = PlayerPreferences {
                panel_width_percent: Some(bad), ..Default::default()
            };
            assert!(invalid.validate().is_err());
        }
    }

    #[test]
    fn panel_position_and_ink_are_reader_owned_and_validated() {
        let authored = Theme::default();
        let reader = PlayerPreferences {
            panel_bottom_percent: Some(1.0), text_ink: Some(TextInk::Black),
            ..Default::default()
        };
        let resolved = reader.resolve_theme(&authored, generic_font_available);
        assert_eq!(resolved.panel.bottom_percent, 1.0);
        assert_eq!(resolved.dialogue.color[0], 0.015);
        assert_eq!(authored.panel.bottom_percent, 3.3);
        assert_eq!(PlayerPreferences::decode(&reader.encode().unwrap()).unwrap(), reader);
        let bad = PlayerPreferences { panel_bottom_percent: Some(21.0), ..Default::default() };
        assert!(bad.validate().is_err());
        let contrast = PlayerPreferences { high_contrast: Some(true), ..reader };
        assert_eq!(contrast.resolve_theme(&authored, generic_font_available).dialogue.color,
                   [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn stored_preferences_do_not_leak_into_narrative_saves() {
        let encoded = PlayerPreferences {
            text_scale: Some(1.2), ..Default::default()
        }.encode().unwrap();
        let string = std::str::from_utf8(&encoded).unwrap();
        assert!(!string.contains("game_id"));
        assert!(!string.contains("machine"));
        assert!(!string.contains("presentation"));
    }
}
