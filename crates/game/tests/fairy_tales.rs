//! External visual-novel examples: exhaustive choice-graph testing.
//! Neither game's story or assets are embedded in Vesper's executable.
use std::{collections::BTreeSet, path::PathBuf};
use vesper_game::Game;
use vesper_story::{Machine, MachineSnapshot, Output, Visual};

fn root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[derive(Default, Debug)]
struct Coverage {
    routes: usize,
    endings: BTreeSet<String>,
    reached: BTreeSet<String>,
    backgrounds: BTreeSet<String>,
    characters: BTreeSet<String>,
    music: BTreeSet<String>,
    sounds: BTreeSet<String>,
    choices: usize,
    dialogue: usize,
    hides: usize,
}

fn explore(game: &Game) -> Coverage {
    let mut result = Coverage::default();
    let start = Machine::new(&game.story, &game.manifest.entry).unwrap();
    let mut branches: Vec<(MachineSnapshot, usize)> = vec![(start.snapshot(), 0)];
    let mut states = 0;

    while let Some((snapshot, steps_so_far)) = branches.pop() {
        let mut machine = Machine::restore(&game.story, snapshot).unwrap();
        let mut steps = steps_so_far;
        loop {
            steps += 1;
            assert!(steps < 256, "story loop detected near scene {}", machine.scene());
            states += 1;
            assert!(states < 20_000, "too many states in test traversal");
            result.reached.insert(machine.scene().to_string());
            match machine.advance().expect("interpreter must not error") {
                Output::Dialogue { .. } => result.dialogue += 1,
                Output::Visual(event) => match event {
                    Visual::Background(id) => { result.backgrounds.insert(id); }
                    Visual::Show { character, expression, .. } => {
                        result.characters.insert(format!("{character}.{expression}"));
                    }
                    Visual::Hide(_) => result.hides += 1,
                    Visual::Music(id) => { result.music.insert(id); }
                    Visual::Sound(id) => { result.sounds.insert(id); }
                },
                Output::Choices(options) => {
                    assert!(!options.is_empty());
                    assert!(options.len() <= 9, "number-key choice limit");
                    result.choices += 1;
                    let paused = machine.snapshot();
                    for index in 0..options.len() {
                        let mut fork = Machine::restore(&game.story, paused.clone())
                            .expect("snapshot round-trip at branching point");
                        fork.choose(index).unwrap();
                        branches.push((fork.snapshot(), steps));
                    }
                    break;
                }
                Output::End => {
                    result.routes += 1;
                    result.endings.insert(machine.scene().to_string());
                    break;
                }
            }
        }
    }
    result
}

#[test]
fn little_lantern_has_exactly_two_complete_paths() {
    let game = Game::load(root("little-lantern")).unwrap();
    assert_eq!(game.manifest.id, "little-lantern");
    assert!(game.manifest.music.is_empty(), "small story must stay minimal");
    let covered = explore(&game);
    assert_eq!(covered.routes, 2, "{covered:?}");
    assert_eq!(covered.endings, BTreeSet::from(["goodnight".to_owned()]));
    assert_eq!(covered.backgrounds, BTreeSet::from(["sunrise".to_owned()]));
    assert_eq!(covered.characters, BTreeSet::from(["pip.cheerful".to_owned()]));
    assert!(covered.hides > 0);
}

#[test]
fn clockwork_moon_exercises_all_thirty_six_branch_combinations() {
    let game = Game::load(root("clockwork-moon")).unwrap();
    assert_eq!(game.manifest.id, "clockwork-moon");
    let covered = explore(&game);
    assert_eq!(covered.routes, 36, "{covered:?}");
    assert_eq!(covered.endings, BTreeSet::from([
        "ending_bold".to_owned(), "ending_home".to_owned(),
        "ending_kind".to_owned(), "ending_quiet".to_owned(),
    ]), "{covered:?}");
    assert_eq!(covered.backgrounds.len(), 7, "{covered:?}");
    assert_eq!(covered.characters.len(), 5, "{covered:?}");
    assert_eq!(covered.music.len(), 3, "{covered:?}");
    assert_eq!(covered.sounds.len(), 5, "{covered:?}");
    assert!(covered.choices >= 15, "{covered:?}");
    assert!(covered.dialogue >= 250, "{covered:?}");
    assert!(covered.hides > 0);
    assert!(covered.reached.len() >= 17);
}

#[test]
fn each_story_has_independent_save_identity() {
    let first = Game::load(root("little-lantern")).unwrap();
    let second = Game::load(root("clockwork-moon")).unwrap();
    assert_ne!(first.manifest.id, second.manifest.id);
    let left = Machine::new(&first.story, &first.manifest.entry).unwrap().snapshot();
    let right = Machine::new(&second.story, &second.manifest.entry).unwrap().snapshot();
    assert_ne!(left.story_fingerprint, right.story_fingerprint);
}
