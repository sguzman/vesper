//! Bevy 0.20 presentation shell. All authored content and artwork is external.

use bevy::{asset::AssetPlugin, prelude::*};
use std::{collections::BTreeMap, env, process};
use vesper_game::Game;
use vesper_story::{Machine, Output, Visual};

#[derive(Resource)]
struct Session {
    game: Game,
    machine: Machine,
    choosing: bool,
    finished: bool,
    background: Option<Entity>,
    characters: BTreeMap<String, Entity>,
}

#[derive(Component)]
struct StoryText;

fn main() {
    let directory = env::args().nth(1).unwrap_or_else(|| "game".into());
    let game = Game::load(&directory).unwrap_or_else(|error| {
        eprintln!("{directory}: {error}");
        process::exit(2);
    });
    let machine = Machine::new(&game.story, &game.manifest.entry).unwrap_or_else(|error| {
        eprintln!("{directory}: {error}");
        process::exit(2);
    });
    let asset_folder = game.root.to_string_lossy().into_owned();
    let title = game.manifest.title.clone();
    App::new()
        .add_plugins(DefaultPlugins
            .set(AssetPlugin { file_path: asset_folder, ..default() })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title,
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                ..default()
            }))
        .insert_resource(Session {
            game, machine, choosing: false, finished: false,
            background: None, characters: BTreeMap::new(),
        })
        .add_systems(Startup, setup)
        .add_systems(Update, advance_from_keyboard)
        .run();
}

fn apply_visual(
    session: &mut Session,
    commands: &mut Commands,
    assets: &AssetServer,
    visual: Visual,
) -> Result<(), String> {
    match visual {
        Visual::Background(id) => {
            let path = session.game.background(&id)?.to_owned();
            if let Some(previous) = session.background.take() {
                commands.entity(previous).despawn();
            }
            let entity = commands.spawn((
                Name::new(format!("Background: {id}")),
                Sprite {
                    image: assets.load(path),
                    custom_size: Some(Vec2::new(1280., 720.)),
                    ..default()
                },
                Transform::from_xyz(0., 0., -10.),
            )).id();
            session.background = Some(entity);
        }
        Visual::Show { character, expression, position } => {
            let path = session.game.character_expression(&character, &expression)?.to_owned();
            let x = match position.as_str() {
                "left" => -350.,
                "center" => 0.,
                "right" => 350.,
                _ => return Err(format!("unknown character position '{position}'")),
            };
            if let Some(previous) = session.characters.remove(&character) {
                commands.entity(previous).despawn();
            }
            let entity = commands.spawn((
                Name::new(format!("Character: {character} ({expression})")),
                Sprite {
                    image: assets.load(path),
                    custom_size: Some(Vec2::new(320., 500.)),
                    ..default()
                },
                Transform::from_xyz(x, -30., 0.),
            )).id();
            session.characters.insert(character, entity);
        }
        Visual::Hide(character) => {
            if let Some(previous) = session.characters.remove(&character) {
                commands.entity(previous).despawn();
            }
        }
        Visual::Music(_) | Visual::Sound(_) => {
            // Audio playback belongs to milestone P03.
        }
    }
    Ok(())
}

/// Execute visual actions once and yield at dialogue, choices, or ending.
fn next_screen(session: &mut Session, commands: &mut Commands, assets: &AssetServer) -> String {
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
                for (index, option) in options.iter().enumerate() {
                    display.push_str(&format!("{}. {}\n", index + 1, option.label));
                }
                return display;
            }
            Ok(Output::Visual(visual)) => {
                if let Err(error) = apply_visual(session, commands, assets, visual) {
                    session.finished = true;
                    return format!("Story asset error:\n{error}");
                }
            }
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

fn setup(
    mut commands: Commands,
    mut session: ResMut<Session>,
    assets: Res<AssetServer>,
) {
    commands.spawn(Camera2d);
    let initial = next_screen(&mut session, &mut commands, &assets);
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
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut text: Single<&mut Text, With<StoryText>>,
) {
    if session.finished { return; }
    if session.choosing {
        let number = [
            KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
            KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
            KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
        ].iter().position(|key| input.just_pressed(*key));
        if let Some(number) = number {
            if session.machine.choose(number).is_ok() {
                session.choosing = false;
                text.0 = next_screen(&mut session, &mut commands, &assets);
            }
        }
    } else if input.just_pressed(KeyCode::Enter) || input.just_pressed(KeyCode::Space) {
        text.0 = next_screen(&mut session, &mut commands, &assets);
    }
}
