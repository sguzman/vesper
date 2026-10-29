//! Exercise every authored branch in the external reference visual novel.
//! Tests load game content from disk, never from compiled runtime constants.
use std::fs;
use vesper_story::{Machine, Output, Story, Value};

fn execute(choice: usize) -> (Vec<String>, Option<Value>) {
    let source = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"), "/../../game/scripts/demo.vn"
    )).unwrap();
    let story = Story::parse(&source).expect("reference story must be valid");
    let mut machine = Machine::new(&story, story.entry_scene()).unwrap();
    let mut dialogue = Vec::new();
    let mut reached_end = false;
    let mut count_choices = 0;
    for _ in 0..128 {
        match machine.advance().unwrap() {
            Output::Visual(_) => {}
            Output::Dialogue { text, .. } | Output::RichDialogue { text, .. } =>
                dialogue.push(text),
            Output::Choices(options) => {
                assert_eq!(options.len(), 2);
                assert_eq!(count_choices, 0, "unexpected second choice node");
                machine.choose(choice).unwrap();
                count_choices += 1;
            }
            Output::End => {
                reached_end = true;
                break;
            }
        }
    }
    assert!(reached_end, "reference story did not terminate within 128 events");
    assert_eq!(count_choices, 1);
    assert_eq!(dialogue.last().map(String::as_str), Some("The evening fades."));
    (dialogue, machine.variables().get("affection").cloned())
}

#[test]
fn reference_story_every_choice_reaches_ending() {
    let (friendly, affection) = execute(0);
    let (confused, absent_affection) = execute(1);
    assert!(friendly.iter().any(|s| s == "I'm glad you're here."));
    assert!(friendly.iter().any(|s| s == "I was hoping you'd return."));
    assert!(confused.iter().any(|s| s == "You're safe."));
    assert_eq!(affection, Some(Value::Number(1)));
    assert_eq!(absent_affection, None);
    assert_ne!(friendly, confused);
}
