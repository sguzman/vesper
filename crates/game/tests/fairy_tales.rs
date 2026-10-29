//! Verify that two external fairytales are complete games, not just parseable
//! files. The longer story is exhaustively exercised over its choice graph.
use std::{
    collections::{BTreeSet, VecDeque},
    path::PathBuf,
};
use vesper_game::Game;
use vesper_story::{Machine, Output, Value, Visual};

fn game(name: &str) -> Game {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    Game::load(path).expect("external game and all referenced assets are valid")
}

#[derive(Default)]
struct Trace {
    scenes: BTreeSet<String>,
    backgrounds: BTreeSet<String>,
    expressions: BTreeSet<String>,
    music: BTreeSet<String>,
    effects: BTreeSet<String>,
    dialogues: Vec<String>,
    variables: std::collections::BTreeMap<String, Value>,
    ending: String,
}
enum ResultState {
    MoreChoices(usize),
    End(Trace),
}

fn play(game: &Game, decisions: &[usize]) -> ResultState {
    let mut machine = Machine::new(&game.story, &game.manifest.entry).unwrap();
    let mut trace = Trace::default();
    let mut chosen = 0;
    for _ in 0..512 {
        trace.scenes.insert(machine.scene().to_owned());
        match machine.advance().expect("runtime story must execute without an error") {
            Output::Dialogue { text, .. } => {
                assert!(!text.trim().is_empty());
                trace.dialogues.push(text);
            }
            Output::Visual(visual) => match visual {
                Visual::Background(id) => {
                    game.background(&id).unwrap();
                    trace.backgrounds.insert(id);
                }
                Visual::Show { character, expression, position } => {
                    game.character_expression(&character, &expression).unwrap();
                    assert!(["left", "center", "right"].contains(&position.as_str()));
                    trace.expressions.insert(format!("{character}.{expression}"));
                }
                Visual::Panel(id) => {
                    if id != "default" {
                        game.panel_skin(&id).expect("panel event uses a declared skin");
                    }
                }
                Visual::Hide(_) => {}
                Visual::AmbientShow(id) | Visual::AmbientHide(id) => {
                    assert!(game.manifest.ambient_actors.contains_key(&id));
                }
                Visual::Music(id) => {
                    game.music(&id).unwrap();
                    trace.music.insert(id);
                }
                Visual::Sound(id) => {
                    game.sound(&id).unwrap();
                    trace.effects.insert(id);
                }
            },
            Output::Choices(options) => {
                assert!(!options.is_empty() && options.len() <= 9);
                if chosen == decisions.len() {
                    return ResultState::MoreChoices(options.len());
                }
                machine.choose(decisions[chosen]).expect("valid choice index");
                chosen += 1;
            }
            Output::End => {
                assert_eq!(chosen, decisions.len(), "unused selections remain");
                trace.ending = machine.scene().to_owned();
                trace.variables = machine.variables().clone();
                assert_eq!(trace.dialogues.last().map(String::as_str), Some("The End."));
                return ResultState::End(trace);
            }
        }
    }
    panic!("story execution failed to end within 512 narrative events");
}

#[test]
fn the_little_lantern_is_a_complete_minimal_novel() {
    let game = game("little-lantern");
    assert_eq!(game.manifest.theme.as_deref(), Some("styles/lantern.toml"));
    assert!(game.theme.panel.accent[0] > game.theme.panel.accent[2],
        "Lantern should load its warm external accent");
    assert!(game.manifest.music.is_empty() && game.manifest.sounds.is_empty());
    assert_eq!(game.manifest.backgrounds.len(), 1);
    assert_eq!(game.manifest.characters.len(), 1);
    for choice in 0..2 {
        assert!(matches!(play(&game, &[]), ResultState::MoreChoices(2)));
        let ResultState::End(trace) = play(&game, &[choice]) else {
            panic!("unexpected second choice");
        };
        assert_eq!(trace.ending, "goodnight");
        assert_eq!(trace.backgrounds.len(), 1);
        assert!(trace.expressions.contains("pip.cheerful"));
        assert_eq!(trace.dialogues.len(), 8);
    }
}

#[test]
fn the_clockwork_moon_explores_all_36_branch_combinations() {
    let game = game("clockwork-moon");
    assert!(game.manifest.theme.is_some());
    assert_eq!(game.manifest.backgrounds.len(), 7);
    assert_eq!(game.manifest.music.len(), 3);
    assert_eq!(game.manifest.sounds.len(), 5);
    let mut queue = VecDeque::from([Vec::<usize>::new()]);
    let mut completed = 0;
    let mut all_scenes = BTreeSet::new();
    let mut all_backgrounds = BTreeSet::new();
    let mut all_expressions = BTreeSet::new();
    let mut all_music = BTreeSet::new();
    let mut all_effects = BTreeSet::new();
    let mut all_endings = BTreeSet::new();
    let mut passages = Vec::new();

    while let Some(path) = queue.pop_front() {
        assert!(path.len() <= 3, "unexpected extra choice in the story");
        match play(&game, &path) {
            ResultState::MoreChoices(n) => {
                for i in 0..n {
                    let mut next = path.clone();
                    next.push(i);
                    queue.push_back(next);
                }
            }
            ResultState::End(trace) => {
                assert_eq!(path.len(), 3, "unexpected early ending");
                assert!(trace.dialogues.len() >= 20, "ambitious game should have real story depth");
                assert!(trace.variables.contains_key("kindness"));
                assert!(trace.variables.contains_key("courage"));
                assert!(trace.variables.contains_key("starlight"));
                assert!(trace.variables.contains_key("pledge"));
                assert!(trace.variables.contains_key("road"));
                completed += 1;
                all_scenes.extend(trace.scenes);
                all_backgrounds.extend(trace.backgrounds);
                all_expressions.extend(trace.expressions);
                all_music.extend(trace.music);
                all_effects.extend(trace.effects);
                all_endings.insert(trace.ending);
                passages.extend(trace.dialogues);
            }
        }
        assert!(completed <= 36, "unexpected choice explosion");
    }

    assert_eq!(completed, 36);
    assert_eq!(all_scenes.len(), 18, "a narrative scene is unreachable");
    assert_eq!(all_endings,
        ["ending_kind", "ending_bold", "ending_quiet", "ending_home"]
            .into_iter().map(str::to_owned).collect());
    assert_eq!(all_backgrounds.len(), game.manifest.backgrounds.len());
    assert_eq!(all_music.len(), game.manifest.music.len());
    assert_eq!(all_effects.len(), game.manifest.sounds.len());
    assert_eq!(all_expressions.len(), 5);
    assert!(passages.iter().any(|text| text.contains("Buenas noches")));
    assert!(passages.iter().any(|text| text.contains("おやすみ")));
    assert!(passages.iter().any(|text| text.len() > 140));
}


#[test]
fn dawn_post_office_explores_all_27_combinations_without_skipped_scenes() {
    let game = game("dawn-post-office");
    assert_eq!(game.manifest.backgrounds.len(), 7);
    assert_eq!(game.manifest.characters.len(), 3);
    assert_eq!(game.manifest.music.len(), 4);
    assert_eq!(game.manifest.sounds.len(), 3);

    let mut queue = VecDeque::from([Vec::<usize>::new()]);
    let mut completed = 0;
    let mut scenes = BTreeSet::new();
    let mut backgrounds = BTreeSet::new();
    let mut expressions = BTreeSet::new();
    let mut music = BTreeSet::new();
    let mut sounds = BTreeSet::new();
    let mut endings = BTreeSet::new();
    let mut best_path_lines = 0;

    while let Some(decisions) = queue.pop_front() {
        assert!(decisions.len() <= 3, "unexpected fourth choice");
        match play(&game, &decisions) {
            ResultState::MoreChoices(count) => {
                assert_eq!(count, 3, "all three authored decisions offer three choices");
                for selected in 0..count {
                    let mut next = decisions.clone();
                    next.push(selected);
                    queue.push_back(next);
                }
            }
            ResultState::End(trace) => {
                assert_eq!(decisions.len(), 3, "premature ending");
                assert!(trace.dialogues.len() >= 55, "showcase route is too short");
                best_path_lines = best_path_lines.max(trace.dialogues.len());
                for field in ["kindness", "courage", "wonder", "promise", "route"] {
                    assert!(trace.variables.contains_key(field), "missing narrative variable {field}");
                }
                completed += 1;
                scenes.extend(trace.scenes);
                backgrounds.extend(trace.backgrounds);
                expressions.extend(trace.expressions);
                music.extend(trace.music);
                sounds.extend(trace.effects);
                endings.insert(trace.ending);
            }
        }
        assert!(completed <= 27, "choice expansion exceeded intended story graph");
    }

    assert_eq!(completed, 27);
    assert_eq!(scenes.len(), 15, "a designed scene is unreachable");
    assert_eq!(backgrounds.len(), 7, "not all authored locations are visited");
    assert_eq!(expressions.len(), 9, "not all generated character expressions are used");
    assert_eq!(music.len(), 4, "not all original music tracks are used");
    assert_eq!(sounds.len(), 3, "not all original sound effects are used");
    assert_eq!(endings, [
        "ending_homecoming", "ending_questions", "ending_lanterns",
    ].into_iter().map(str::to_owned).collect());
    assert!(best_path_lines >= 60, "showcase should have extended reading depth");
}
