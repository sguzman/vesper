//! Initial Bevy 0.20 presentation bridge for externally authored story scripts.
//! Assets and rich typography will be integrated in subsequent milestones.

use std::{env, fs, process};
use bevy::prelude::*;
use vesper_story::{Machine, Output, Story};

#[derive(Resource)]
struct Session {
    machine: Machine,
    choosing: bool,
    finished: bool,
}

#[derive(Component)]
struct StoryText;

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "game/scripts/demo.vn".to_owned());
    let source = fs::read_to_string(&path).unwrap_or_else(|err| {
        eprintln!("{path}: {err}");
        process::exit(2);
    });
    let story = Story::parse(&source).unwrap_or_else(|diagnostics| {
        for diagnostic in diagnostics {
            eprintln!("{path}:{}: {}", diagnostic.line, diagnostic.message);
        }
        process::exit(2);
    });
    let machine = Machine::new(&story, story.entry_scene()).unwrap_or_else(|err| {
        eprintln!("{path}: {err}");
        process::exit(2);
    });
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Session {
            machine,
            choosing: false,
            finished: false,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, advance_from_keyboard)
        .run();
}

/// Yields a blocking narrative event. Visual events are not yet rendered by the
/// initial player: asset-backed ECS presentation is a later milestone.
fn next_screen(session: &mut Session) -> String {
    for _ in 0..128 {
        match session.machine.advance() {
            Ok(Output::Dialogue { speaker, text }) => {
                return match speaker {
                    Some(name) => format!("{name}\n{text}\n\n[Enter or Space]"),
                    None => format!("{text}\n\n[Enter or Space]"),
                };
            }
            Ok(Output::Choices(options)) => {
                session.choosing = true;
                let mut display = String::from("Choose:\n");
                for (number, option) in options.iter().enumerate() {
                    display.push_str(&format!("{}. {}\n", number + 1, option.label));
                }
                return display;
            }
            Ok(Output::Visual(_)) => {}
            Ok(Output::End) => {
                session.finished = true;
                return "The End".into();
            }
            Err(error) => {
                session.finished = true;
                return format!("Story error:\n{error}");
            }
        }
    }
    session.finished = true;
    "Story did not reach dialogue within 128 presentation events.".into()
}

fn setup(mut commands: Commands, mut session: ResMut<Session>) {
    let initial = next_screen(&mut session);
    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(24),
                left: percent(5),
                width: percent(90),
                min_height: px(170),
                padding: px(24).all(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.05, 0.09, 0.96)),
        ))
        .with_child((
            Text::new(initial),
            TextFont::from_font_size(px(28)),
            TextColor(Color::WHITE),
            StoryText,
        ));
}

fn advance_from_keyboard(
    input: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<Session>,
    mut text: Single<&mut Text, With<StoryText>>,
) {
    if session.finished {
        return;
    }

    if session.choosing {
        let number = [
            KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
            KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
            KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
        ]
        .iter()
        .position(|key| input.just_pressed(*key));
        if let Some(number) = number {
            if session.machine.choose(number).is_ok() {
                session.choosing = false;
                text.0 = next_screen(&mut session);
            }
        }
    } else if input.just_pressed(KeyCode::Enter) || input.just_pressed(KeyCode::Space) {
        text.0 = next_screen(&mut session);
    }
}
