//! Filesystem-based game format. Never interpret author-controlled asset paths
//! as unrestricted paths outside the chosen game directory.

use serde::{Deserialize, Serialize};
pub mod save;
pub mod preferences;
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};
use vesper_story::Story;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format_version: u32,
    pub id: String,
    pub title: String,
    pub story: String,
    pub entry: String,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub backgrounds: BTreeMap<String, String>,
    #[serde(default)]
    pub music: BTreeMap<String, String>,
    #[serde(default)]
    pub sounds: BTreeMap<String, String>,
    #[serde(default)]
    pub characters: BTreeMap<String, Character>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Character {
    pub expressions: BTreeMap<String, String>,
}

/// An externally editable visual theme; values are CSS-like logical UI sizes.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Theme {
    pub ui_scale: f32,
    pub fade_seconds: f32,
    pub reveal_speed: TextRevealSpeed,
    pub font: FontSelection,
    pub panel: PanelStyle,
    pub speaker: TextStyle,
    pub dialogue: TextStyle,
    pub choices: TextStyle,
    pub hint: TextStyle,
}

/// Text reveal speed is authorable in theme TOML and can be overridden locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextRevealSpeed {
    #[default]
    Instant,
    Slow,
    Normal,
    Fast,
}
impl TextRevealSpeed {
    pub const fn rate(self) -> Option<f64> {
        match self {
            Self::Instant => None,
            Self::Slow => Some(14.0),
            Self::Normal => Some(34.0),
            Self::Fast => Some(80.0),
        }
    }
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FontSelection {
    /// Optional game-relative .ttf or .otf path; do not embed font bytes in Rust.
    pub file: Option<String>,
    /// System font family alternative when no font file is specified.
    pub family: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PanelStyle {
    pub background: [f32; 4],
    /// Game-authored highlight bar above dialogue.
    pub accent: [f32; 4],
    /// Game-authored visual division between dialogue and hints.
    pub divider: [f32; 4],
    pub width_percent: f32,
    pub min_height: f32,
    pub padding: f32,
    pub gap: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TextStyle {
    pub size: f32,
    pub color: [f32; 4],
}

impl Default for PanelStyle {
    fn default() -> Self {
        Self {
            background: [0.04, 0.05, 0.09, 0.96],
            accent: [0.96, 0.75, 0.40, 1.0],
            divider: [0.70, 0.76, 0.86, 0.34],
            width_percent: 90.0,
            min_height: 180.0,
            padding: 20.0,
            gap: 8.0,
        }
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self { size: 28.0, color: [1.0, 1.0, 1.0, 1.0] }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            ui_scale: 1.0,
            fade_seconds: 0.25,
            reveal_speed: TextRevealSpeed::Instant,
            font: FontSelection::default(),
            panel: PanelStyle::default(),
            speaker: TextStyle { size: 25.0, color: [0.96, 0.75, 0.40, 1.0] },
            dialogue: TextStyle { size: 28.0, color: [1.0, 1.0, 1.0, 1.0] },
            choices: TextStyle { size: 26.0, color: [0.88, 0.94, 1.0, 1.0] },
            hint: TextStyle { size: 16.0, color: [0.75, 0.78, 0.83, 1.0] },
        }
    }
}

fn color_valid(color: [f32; 4]) -> bool {
    color.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v))
}

impl Theme {
    pub fn validate(&self) -> Result<(), String> {
        if !self.ui_scale.is_finite() || !(0.5..=2.5).contains(&self.ui_scale) {
            return Err("theme.ui_scale must be between 0.5 and 2.5".into());
        }
        if !self.fade_seconds.is_finite() || !(0.0..=3.0).contains(&self.fade_seconds) {
            return Err("theme.fade_seconds must be between 0 and 3".into());
        }
        let panel = &self.panel;
        if !color_valid(panel.background)
            || !color_valid(panel.accent)
            || !color_valid(panel.divider)
            || !panel.width_percent.is_finite()
            || !(20.0..=100.0).contains(&panel.width_percent)
            || !panel.min_height.is_finite()
            || !(80.0..=600.0).contains(&panel.min_height)
            || !panel.padding.is_finite()
            || !(0.0..=80.0).contains(&panel.padding)
            || !panel.gap.is_finite()
            || !(0.0..=48.0).contains(&panel.gap)
        {
            return Err("theme.panel contains an invalid color or dimension".into());
        }
        for (name, style) in [
            ("speaker", &self.speaker),
            ("dialogue", &self.dialogue),
            ("choices", &self.choices),
            ("hint", &self.hint),
        ] {
            if !style.size.is_finite()
                || !(8.0..=96.0).contains(&style.size)
                || !color_valid(style.color)
            {
                return Err(format!("theme.{name} has invalid text size or color"));
            }
        }
        if self.font.file.is_some() && self.font.family.is_some() {
            return Err("theme.font specifies both file and family; select one".into());
        }
        if self.font.family.as_ref().is_some_and(|name| name.trim().is_empty()) {
            return Err("theme.font.family cannot be empty".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Game {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub story: Story,
    pub theme: Theme,
}

fn id_valid(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// Resolve and validate paths, including symlink targets, before Bevy receives
/// an author-controlled asset path. Reject absolute and traversal components.
fn checked_file(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative.is_empty()
        || !relative_path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(format!("unsafe or invalid relative path: '{relative}'"));
    }
    let full = root.join(relative_path);
    let canonical = full
        .canonicalize()
        .map_err(|error| format!("{}: {error}", full.display()))?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err(format!("path is not a regular file inside game directory: '{relative}'"));
    }
    Ok(canonical)
}

impl Game {
    pub fn load(directory: impl AsRef<Path>) -> Result<Self, String> {
        let root = directory.as_ref().canonicalize().map_err(|error| {
            format!("{}: {error}", directory.as_ref().display())
        })?;
        if !root.is_dir() {
            return Err(format!("not a game directory: {}", root.display()));
        }
        let manifest_path = checked_file(&root, "game.toml")?;
        let text = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
        let manifest: Manifest = toml::from_str(&text)
            .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
        if manifest.format_version != 1 {
            return Err(format!(
                "unsupported game format version {}; expected 1",
                manifest.format_version
            ));
        }
        if !id_valid(&manifest.id) || manifest.title.trim().is_empty() {
            return Err("manifest requires valid 'id' and nonempty 'title'".into());
        }
        let script_path = checked_file(&root, &manifest.story)?;
        let source = fs::read_to_string(&script_path)
            .map_err(|error| format!("{}: {error}", script_path.display()))?;
        let story = Story::parse(&source).map_err(|diagnostics| {
            diagnostics
                .into_iter()
                .map(|diag| format!("{}:{}: {}", script_path.display(), diag.line, diag.message))
                .collect::<Vec<_>>()
                .join("\n")
        })?;
        if !story.has_scene(&manifest.entry) {
            return Err(format!("entry scene '{}' does not exist", manifest.entry));
        }
        let theme = if let Some(path) = &manifest.theme {
            let path = checked_file(&root, path)?;
            let content = fs::read_to_string(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            toml::from_str::<Theme>(&content)
                .map_err(|error| format!("{}: {error}", path.display()))?
        } else {
            Theme::default()
        };
        theme.validate()?;
        if let Some(file) = &theme.font.file {
            let path = checked_file(&root, file)?;
            if !matches!(
                path.extension().and_then(|ext| ext.to_str()).map(str::to_ascii_lowercase).as_deref(),
                Some("ttf" | "otf")
            ) {
                return Err("theme.font.file must be a .ttf or .otf file".into());
            }
        }
        for (name, filename) in &manifest.backgrounds {
            if !id_valid(name) {
                return Err(format!("invalid background id '{name}'"));
            }
            checked_file(&root, filename)?;
        }
        for (kind, files) in [("music", &manifest.music), ("sound", &manifest.sounds)] {
            for (name, filename) in files {
                if !id_valid(name) {
                    return Err(format!("invalid {kind} identifier '{name}'"));
                }
                let file = checked_file(&root, filename)?;
                if !matches!(file.extension().and_then(|ext| ext.to_str())
                    .map(str::to_ascii_lowercase).as_deref(), Some("wav" | "ogg")) {
                    return Err(format!("{kind} '{name}' must be a .wav or .ogg file"));
                }
            }
        }
        for (name, character) in &manifest.characters {
            if !id_valid(name) || character.expressions.is_empty() {
                return Err(format!("invalid character '{name}' or missing expressions"));
            }
            for (expression, filename) in &character.expressions {
                if !id_valid(expression) {
                    return Err(format!("invalid expression '{name}.{expression}'"));
                }
                checked_file(&root, filename)?;
            }
        }
        for (line, visual) in story.visual_references() {
            let missing = match visual {
                vesper_story::Visual::Background(id) => {
                    if manifest.backgrounds.contains_key(&id) { None }
                    else { Some(format!("background '{id}'")) }
                }
                vesper_story::Visual::Show { character, expression, .. } => {
                    if manifest.characters.get(&character)
                        .is_some_and(|entry| entry.expressions.contains_key(&expression)) {
                        None
                    } else { Some(format!("character expression '{character}.{expression}'")) }
                }
                vesper_story::Visual::Music(id) => {
                    if manifest.music.contains_key(&id) { None }
                    else { Some(format!("music '{id}'")) }
                }
                vesper_story::Visual::Sound(id) => {
                    if manifest.sounds.contains_key(&id) { None }
                    else { Some(format!("sound '{id}'")) }
                }
                _ => None,
            };
            if let Some(reference) = missing {
                return Err(format!(
                    "{}:{line}: unknown {reference} in game manifest",
                    script_path.display()
                ));
            }
        }
        Ok(Self { root, manifest, story, theme })
    }

    pub fn music(&self, id: &str) -> Result<&str, String> {
        self.manifest.music.get(id).map(String::as_str)
            .ok_or_else(|| format!("unknown music '{id}'"))
    }

    pub fn sound(&self, id: &str) -> Result<&str, String> {
        self.manifest.sounds.get(id).map(String::as_str)
            .ok_or_else(|| format!("unknown sound '{id}'"))
    }

    pub fn background(&self, id: &str) -> Result<&str, String> {
        self.manifest.backgrounds.get(id).map(String::as_str)
            .ok_or_else(|| format!("unknown background '{id}'"))
    }

    pub fn character_expression(&self, id: &str, expression: &str) -> Result<&str, String> {
        self.manifest.characters.get(id)
            .ok_or_else(|| format!("unknown character '{id}'"))?
            .expressions.get(expression).map(String::as_str)
            .ok_or_else(|| format!("unknown expression '{id}.{expression}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::atomic::{AtomicU64, Ordering}, time::{SystemTime, UNIX_EPOCH}};
    static NONCE: AtomicU64 = AtomicU64::new(0);

    struct Fixture { root: PathBuf }

    impl Fixture {
        fn new(id: &str) -> Self {
            let number = NONCE.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let root = std::env::temp_dir().join(format!(
                "vesper-{id}-{}-{nanos}-{number}", std::process::id()
            ));
            fs::create_dir_all(root.join("scripts")).unwrap();
            fs::create_dir_all(root.join("assets")).unwrap();
            fs::write(root.join("scripts/story.vn"), "@scene opening\n\"Hello\"\n").unwrap();
            fs::write(root.join("assets/room.png"), b"fixture").unwrap();
            fs::write(root.join("game.toml"), format!(r#"
format_version = 1
id = "{id}"
title = "Example"
story = "scripts/story.vn"
entry = "opening"
[backgrounds]
room = "assets/room.png"
"#)).unwrap();
            Self { root }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) { fs::remove_dir_all(&self.root).unwrap(); }
    }

    #[test]
    fn independent_game_folders_load_without_compilation() {
        let first = Fixture::new("first");
        let second = Fixture::new("second");
        assert_eq!(Game::load(&first.root).unwrap().manifest.id, "first");
        assert_eq!(Game::load(&second.root).unwrap().manifest.id, "second");
    }

    #[test]
    fn rejects_parent_path_escape() {
        let sample = Fixture::new("traversal");
        let manifest = fs::read_to_string(sample.root.join("game.toml")).unwrap();
        fs::write(sample.root.join("game.toml"), manifest.replace(
            "scripts/story.vn", "../outside.vn")).unwrap();
        let error = Game::load(&sample.root).unwrap_err();
        assert!(error.contains("unsafe"), "{error}");
    }

    #[test]
    fn rejects_missing_assets_and_unknown_entry_scene() {
        let sample = Fixture::new("missing");
        fs::remove_file(sample.root.join("assets/room.png")).unwrap();
        assert!(Game::load(&sample.root).unwrap_err().contains("room.png"));
        fs::write(sample.root.join("assets/room.png"), b"fixture").unwrap();
        let manifest = fs::read_to_string(sample.root.join("game.toml")).unwrap();
        fs::write(sample.root.join("game.toml"), manifest.replace(
            "entry = \"opening\"", "entry = \"missing\"")).unwrap();
        assert!(Game::load(&sample.root).unwrap_err().contains("entry scene"));
    }

    #[test]
    fn rejects_undeclared_visual_references() {
        let sample = Fixture::new("visual-ref");
        fs::write(
            sample.root.join("scripts/story.vn"),
            "@scene opening\nbackground missing\n",
        ).unwrap();
        let err = Game::load(&sample.root).unwrap_err();
        assert!(err.contains("story.vn:2:"), "{err}");
        assert!(err.contains("background 'missing'"), "{err}");
        fs::write(
            sample.root.join("scripts/story.vn"),
            "@scene opening\nshow elena happy at center\n",
        ).unwrap();
        let err = Game::load(&sample.root).unwrap_err();
        assert!(err.contains("character expression 'elena.happy'"), "{err}");
    }

    #[test]
    fn external_theme_and_font_path_are_validated() {
        let fixture = Fixture::new("theme");
        fs::create_dir_all(fixture.root.join("styles")).unwrap();
        let config = fixture.root.join("game.toml");
        let initial = fs::read_to_string(&config).unwrap();
        fs::write(&config, initial.replace(
            "entry = \"opening\"",
            "entry = \"opening\"\ntheme = \"styles/visual.toml\"",
        )).unwrap();
        fs::write(fixture.root.join("styles/visual.toml"),
            "ui_scale = 1.4\n[font]\nfile = \"../outside.ttf\"\n").unwrap();
        let err = Game::load(&fixture.root).unwrap_err();
        assert!(err.contains("unsafe") || err.contains("font"), "{err}");
        fs::write(fixture.root.join("styles/visual.toml"),
            "ui_scale = 1.4\n[dialogue]\nsize = 36.0\ncolor = [0.7, 0.8, 0.9, 1.0]\n"
        ).unwrap();
        let loaded = Game::load(&fixture.root).unwrap();
        assert_eq!(loaded.theme.ui_scale, 1.4);
        assert_eq!(loaded.theme.dialogue.size, 36.0);
        fs::write(fixture.root.join("styles/visual.toml"), "ui_scale = 0.1\n").unwrap();
        assert!(Game::load(&fixture.root).unwrap_err().contains("ui_scale"));
    }

    #[test]
    fn panel_palette_defaults_and_validation() {
        let theme: Theme = toml::from_str("[panel]\naccent = [0.72, 0.84, 1.0, 1.0]\ndivider = [0.6, 0.8, 1.0, 0.4]").unwrap();
        assert_eq!(theme.panel.accent[0], 0.72);
        assert!(theme.validate().is_ok());
        let invalid: Theme = toml::from_str("[panel]\naccent = [1.2, 0.5, 0.5, 1.0]").unwrap();
        assert!(invalid.validate().is_err());
        // Existing themes without optional tokens remain backwards compatible.
        assert!(toml::from_str::<Theme>("[panel]\nwidth_percent = 90.0")
            .unwrap().validate().is_ok());
    }

    #[test]
    fn rejects_undeclared_sound_and_invalid_transition() {
        let sample = Fixture::new("sounds");
        fs::write(sample.root.join("scripts/story.vn"), "@scene opening\nmusic night\n").unwrap();
        let err = Game::load(&sample.root).unwrap_err();
        assert!(err.contains("story.vn:2:") && err.contains("music 'night'"), "{err}");
        fs::write(sample.root.join("scripts/story.vn"), "@scene opening\nsound hello\n").unwrap();
        let err = Game::load(&sample.root).unwrap_err();
        assert!(err.contains("sound 'hello'"), "{err}");
        assert!(toml::from_str::<Theme>("fade_seconds = -1.0").unwrap().validate().is_err());
        assert!(toml::from_str::<Theme>("fade_seconds = 0.0").unwrap().validate().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_outside_game_folder() {
        use std::os::unix::fs::symlink;
        let sample = Fixture::new("symlink");
        fs::remove_file(sample.root.join("assets/room.png")).unwrap();
        symlink("/etc/hosts", sample.root.join("assets/room.png")).unwrap();
        let err = Game::load(&sample.root).unwrap_err();
        assert!(err.contains("inside game directory"), "{err}");
    }
}
