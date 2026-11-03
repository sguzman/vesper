//! Verify cross-manifest .vn travel references before the Bevy player starts.
use std::{fs, path::PathBuf, sync::atomic::{AtomicU64, Ordering}};
use vesper_game::Game;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/exploration-bridge")
}

struct FixtureCopy(PathBuf);
impl FixtureCopy {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir()
            .join(format!("vesper-authored-explore-{}-{id}", std::process::id()));
        fs::create_dir_all(dir.join("scripts")).unwrap();
        fs::create_dir_all(dir.join("assets/backgrounds")).unwrap();
        for name in [
            "game.toml", "scripts/story.vn", "assets/backgrounds/sunrise.png",
        ] {
            fs::copy(fixture().join(name), dir.join(name)).unwrap();
        }
        Self(dir)
    }
    fn write(&self, path: &str, text: &str) {
        fs::write(self.0.join(path), text).unwrap();
    }
    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.0.join(path)).unwrap()
    }
}
impl Drop for FixtureCopy {
    fn drop(&mut self) { fs::remove_dir_all(&self.0).unwrap(); }
}

#[test]
fn mixed_mode_game_opens_as_a_vn_and_has_two_valid_world_transfers() {
    let game = Game::load(fixture()).unwrap();
    let world = game.manifest.exploration.as_ref().unwrap();
    assert!(world.start.is_none());
    let transfers = game.story.exploration_transfers();
    assert_eq!(transfers.len(), 2);
    assert_eq!(transfers[0].1, "street");
    assert_eq!(transfers[0].2, "home");
    assert_eq!(transfers[1].1, "cafe");
    assert_eq!(transfers[1].2, "entry");
}

#[test]
fn invalid_authored_location_or_spawn_reports_actual_vn_line() {
    for (from, to, reason) in [
        ("explore street at home", "explore missing at home", "location"),
        ("explore cafe at entry", "explore cafe at missing", "spawn"),
    ] {
        let copy = FixtureCopy::new();
        copy.write("scripts/story.vn", &copy.read("scripts/story.vn").replace(from, to));
        let error = Game::load(&copy.0).unwrap_err();
        assert!(error.contains("scripts/story.vn:"), "{error}");
        assert!(error.contains(reason), "{error}");
    }
}

#[test]
fn pure_visual_novel_cannot_reference_an_undeclared_exploration_world() {
    let copy = FixtureCopy::new();
    copy.write("game.toml", r#"
format_version = 1
id = "no-world"
title = "Pure visual novel"
story = "scripts/story.vn"
entry = "intro"
"#);
    let error = Game::load(&copy.0).unwrap_err();
    assert!(error.contains("scripts/story.vn:"), "{error}");
    assert!(error.contains("not an authored location"), "{error}");
}

#[test]
fn bridge_fixture_uses_local_art_not_author_executable_code() {
    let root = fixture();
    assert!(root.join("assets/backgrounds/sunrise.png").exists());
    let source = fs::read_to_string(root.join("scripts/story.vn")).unwrap();
    assert!(source.contains("explore street at home"));
    assert!(source.contains("explore cafe at entry"));
}
