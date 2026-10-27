//! Filesystem-based game format. Never interpret author-controlled asset paths
//! as unrestricted paths outside the chosen game directory.

use serde::Deserialize;
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
    pub backgrounds: BTreeMap<String, String>,
    #[serde(default)]
    pub characters: BTreeMap<String, Character>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Character {
    pub expressions: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub story: Story,
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
        for (name, filename) in &manifest.backgrounds {
            if !id_valid(name) {
                return Err(format!("invalid background id '{name}'"));
            }
            checked_file(&root, filename)?;
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
                _ => None,
            };
            if let Some(reference) = missing {
                return Err(format!(
                    "{}:{line}: unknown {reference} in game manifest",
                    script_path.display()
                ));
            }
        }
        Ok(Self { root, manifest, story })
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
