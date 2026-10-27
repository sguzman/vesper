//! Bevy 0.20 presentation shell. All authored content and artwork is external.

use bevy::{asset::{AssetPlugin, LoadState}, prelude::*};
mod reload;
use std::{collections::BTreeMap, env, path::PathBuf, process, sync::{Arc, Mutex}};
use vesper_game::{Game, save::{CharacterPose, PresentationState, SaveData}};
use vesper_story::{Machine, Output, Visual};

#[derive(Resource)]
struct Session {
    game: Game,
    machine: Machine,
    choosing: bool,
    finished: bool,
    background: Option<Entity>,
    music: Option<Entity>,
    characters: BTreeMap<String, Entity>,
    presentation: PresentationState,
    pending_audio: Vec<(String, Handle<AudioSource>)>,
}

#[derive(Resource, Default)]
struct CompletedIo(Arc<Mutex<Vec<IoResult>>>);

/// Keep asynchronous save workers alive through application shutdown.
#[derive(Resource, Default)]
struct IoWorkers(Arc<Mutex<Vec<std::thread::JoinHandle<()>>>>);

enum IoResult {
    Saved(Result<(), String>),
    Loaded(Result<(Machine, PresentationState), String>),
}


#[derive(Component)]
struct FadeIn { elapsed: f32, duration: f32 }

fn fade_fraction(elapsed: f32, duration: f32) -> f32 {
    (elapsed / duration).clamp(0.0, 1.0)
}

fn animate_fades(
    time: Res<Time>,
    mut commands: Commands,
    mut sprites: Query<(Entity, &Name, &mut Sprite, &mut FadeIn)>,
) {
    for (entity, name, mut sprite, mut fade) in &mut sprites {
        fade.elapsed = (fade.elapsed + time.delta_secs()).min(fade.duration);
        let alpha = fade_fraction(fade.elapsed, fade.duration);
        sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
        if fade.elapsed >= fade.duration {
            info!("Vesper fade complete: {}", name.as_str());
            commands.entity(entity).remove::<FadeIn>();
        }
    }
}

#[derive(Component)]
struct StoryPanel;

#[derive(Component)]
enum StoryText {
    Speaker,
    Body,
    Hint,
}

struct Screen {
    speaker: String,
    body: String,
    hint: String,
    choices: bool,
}

impl Screen {
    fn normal(speaker: String, body: String) -> Self {
        Self {
            speaker,
            body,
            hint: "Enter / Space: continue   F5: save   F9: load".into(),
            choices: false,
        }
    }

    fn choices(options: &[vesper_story::Choice], selected: usize) -> Self {
        let mut lines = String::new();
        for (index, option) in options.iter().enumerate() {
            lines.push_str(&format!("{} {}. {}\n",
                if index == selected { "▶" } else { " " },
                index + 1, option.label,
            ));
        }
        Self {
            speaker: String::new(),
            body: lines,
            hint: "Arrows: move   Enter: select   Number: choose   F5: save   F9: load".into(),
            choices: true,
        }
    }

    fn ending(message: String) -> Self {
        Self { speaker: String::new(), body: message, hint: String::new(), choices: false }
    }

    fn part(&self, slot: &StoryText) -> &str {
        match slot {
            StoryText::Speaker => &self.speaker,
            StoryText::Body => &self.body,
            StoryText::Hint => &self.hint,
        }
    }
}

fn text_font(theme: &vesper_game::Theme, assets: &AssetServer) -> FontSource {
    if let Some(file) = &theme.font.file {
        assets.load::<Font>(file.clone()).into()
    } else if let Some(family) = &theme.font.family {
        FontSource::Family(family.clone().into())
    } else {
        FontSource::sans_serif()
    }
}

fn text_color(components: [f32; 4]) -> TextColor {
    TextColor(Color::srgba(
        components[0], components[1], components[2], components[3]
    ))
}

fn add_text(
    parent: &mut ChildSpawnerCommands,
    slot: StoryText,
    content: &str,
    style: &vesper_game::TextStyle,
    font: &FontSource,
) {
    parent.spawn((
        slot,
        Text::new(content),
        TextFont {
            font: font.clone(),
            font_size: FontSize::Px(style.size),
            ..default()
        },
        text_color(style.color),
        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
        Node {
            width: percent(100),
            ..default()
        },
    ));
}

fn main() {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let watch = arguments.iter().any(|arg| arg == "--watch");
    let directory = arguments.iter().find(|arg| arg.as_str() != "--watch")
        .cloned().unwrap_or_else(|| "game".into());
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
    let ui_scale = game.theme.ui_scale;
    let inbox = watch.then(|| reload::start(game.root.clone()));
    let io_workers = IoWorkers::default();
    let outstanding_workers = io_workers.0.clone();
    let mut app = App::new();
    app
        .add_plugins(DefaultPlugins
            .set(AssetPlugin {
                file_path: asset_folder,
                watch_for_changes_override: Some(watch),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title,
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                ..default()
            }))
        .insert_resource(UiScale(ui_scale))
        .insert_resource(CompletedIo::default())
        .insert_resource(io_workers)
        .insert_resource(Session {
            game, machine, choosing: false, finished: false,
            background: None, music: None, characters: BTreeMap::new(),
            presentation: PresentationState::default(),
            pending_audio: Vec::new(),
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (advance_from_keyboard, trigger_save_load, poll_completed_io, animate_fades, monitor_audio_assets).chain());
    if let Some(inbox) = inbox {
        app.insert_resource(inbox).add_systems(Update, reload::apply_reload);
    }
    app.run();
    // This is after Bevy's UI/render loop has exited, so joining cannot
    // block a live frame. F5 immediately followed by closing the window is safe.
    if let Ok(mut jobs) = outstanding_workers.lock() {
        for job in std::mem::take(&mut *jobs) {
            if job.join().is_err() {
                eprintln!("A Vesper save/load worker panicked during shutdown");
            }
        }
    }
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
            let fade = session.game.theme.fade_seconds;
            let entity = commands.spawn((
                Name::new(format!("Background: {id}")),
                Sprite {
                    image: assets.load(path),
                    color: if fade > 0.0 { Color::srgba(1.0, 1.0, 1.0, 0.0) } else { Color::WHITE },
                    custom_size: Some(Vec2::new(1280., 720.)),
                    ..default()
                },
                Transform::from_xyz(0., 0., -10.),
            )).id();
            if fade > 0.0 {
                commands.entity(entity).insert(FadeIn { elapsed: 0.0, duration: fade });
            }
            session.background = Some(entity);
            session.presentation.background = Some(id);
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
            let fade = session.game.theme.fade_seconds;
            let entity = commands.spawn((
                Name::new(format!("Character: {character} ({expression})")),
                Sprite {
                    image: assets.load(path),
                    color: if fade > 0.0 { Color::srgba(1.0, 1.0, 1.0, 0.0) } else { Color::WHITE },
                    custom_size: Some(Vec2::new(320., 500.)),
                    ..default()
                },
                Transform::from_xyz(x, -30., 0.),
            )).id();
            if fade > 0.0 {
                commands.entity(entity).insert(FadeIn { elapsed: 0.0, duration: fade });
            }
            session.presentation.characters.insert(
                character.clone(),
                CharacterPose { expression, position },
            );
            session.characters.insert(character, entity);
        }
        Visual::Hide(character) => {
            if let Some(previous) = session.characters.remove(&character) {
                commands.entity(previous).despawn();
            }
            session.presentation.characters.remove(&character);
        }
        Visual::Music(id) => {
            let path = session.game.music(&id)?.to_owned();
            if let Some(previous) = session.music.take() {
                commands.entity(previous).despawn();
            }
            let entity = commands.spawn((
                Name::new(format!("Music: {id}")),
                AudioPlayer::new({
                    let handle = assets.load::<AudioSource>(path);
                    session.pending_audio.push((format!("music:{id}"), handle.clone()));
                    handle
                }),
                PlaybackSettings::LOOP,
            )).id();
            session.music = Some(entity);
            session.presentation.music = Some(id.clone());
            info!("Vesper queued music: {id}");
        }
        Visual::Sound(id) => {
            let path = session.game.sound(&id)?.to_owned();
            commands.spawn((
                Name::new(format!("Sound: {id}")),
                AudioPlayer::new({
                    let handle = assets.load::<AudioSource>(path);
                    session.pending_audio.push((format!("sound:{id}"), handle.clone()));
                    handle
                }),
                PlaybackSettings::DESPAWN,
            ));
            info!("Vesper queued sound: {id}");
        }
    }
    Ok(())
}

/// Execute visual actions once and yield at dialogue, choices, or ending.
fn next_screen(session: &mut Session, commands: &mut Commands, assets: &AssetServer) -> Screen {
    for _ in 0..128 {
        match session.machine.advance() {
            Ok(Output::Dialogue { speaker, text }) => {
                if env::var_os("VESPER_TRACE_SCENES").is_some() {
                    info!("Vesper scene dialogue: {}", session.machine.scene());
                }
                return Screen::normal(speaker.unwrap_or_default(), text);
            }
            Ok(Output::Choices(options)) => {
                session.choosing = true;
                session.presentation.selected_choice = 0;
                return Screen::choices(&options, 0);
            }
            Ok(Output::Visual(visual)) => {
                if let Err(error) = apply_visual(session, commands, assets, visual) {
                    session.finished = true;
                    return Screen::ending(format!("Story asset error:\n{error}"));
                }
            }
            Ok(Output::End) => {
                info!("Vesper story reached ending scene: {}", session.machine.scene());
                session.finished = true;
                return Screen::ending("The End".into());
            }
            Err(error) => {
                session.finished = true;
                return Screen::ending(format!("Story error:\n{error}"));
            }
        }
    }
    session.finished = true;
    Screen::ending("Story did not reach dialogue within 128 presentation events.".into())
}

fn setup(
    mut commands: Commands,
    mut session: ResMut<Session>,
    assets: Res<AssetServer>,
) {
    commands.spawn(Camera2d);
    let theme = session.game.theme.clone();
    let font = text_font(&theme, &assets);
    let initial = next_screen(&mut session, &mut commands, &assets);
    let panel = &session.game.theme.panel;
    let background = panel.background;
    let speaker_style = theme.speaker.clone();
    let body_style = if initial.choices { &theme.choices } else { &theme.dialogue }.clone();
    let hint_style = theme.hint.clone();

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(panel.gap),
                bottom: px(24),
                left: percent((100.0 - panel.width_percent) / 2.0),
                width: percent(panel.width_percent),
                min_height: px(panel.min_height),
                padding: px(panel.padding).all(),
                ..default()
            },
            BackgroundColor(Color::srgba(
                background[0], background[1], background[2], background[3],
            )),
            StoryPanel,
        ))
        .with_children(|parent| {
            add_text(parent, StoryText::Speaker, &initial.speaker, &speaker_style, &font);
            add_text(parent, StoryText::Body, &initial.body, &body_style, &font);
            add_text(parent, StoryText::Hint, &initial.hint, &hint_style, &font);
        });
}

fn update_screen(
    screen: Screen,
    theme: &vesper_game::Theme,
    texts: &mut Query<(&mut Text, &mut TextFont, &mut TextColor, &StoryText)>,
) {
    for (mut text, mut font, mut color, slot) in texts.iter_mut() {
        text.0 = screen.part(slot).to_owned();
        if matches!(slot, StoryText::Body) {
            let style = if screen.choices { &theme.choices } else { &theme.dialogue };
            font.font_size = FontSize::Px(style.size);
            *color = text_color(style.color);
        }
    }
}

fn advance_from_keyboard(
    input: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<Session>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texts: Query<(&mut Text, &mut TextFont, &mut TextColor, &StoryText)>,
) {
    if session.finished { return; }
    if session.choosing {
        let options = match session.machine.last_output() {
            Some(Output::Choices(options)) => options.clone(),
            _ => return,
        };
        if options.is_empty() { return; }
        let current = session.presentation.selected_choice.min(options.len() - 1);
        let selected = if input.just_pressed(KeyCode::ArrowDown) {
            Some((current + 1) % options.len())
        } else if input.just_pressed(KeyCode::ArrowUp) {
            Some((current + options.len() - 1) % options.len())
        } else {
            None
        };
        if let Some(index) = selected {
            session.presentation.selected_choice = index;
            update_screen(Screen::choices(&options, index), &session.game.theme, &mut texts);
            return;
        }
        let digit = [
            KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
            KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
            KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
        ].iter().position(|key| input.just_pressed(*key));
        let confirmed = digit.or_else(|| {
            (input.just_pressed(KeyCode::Enter) || input.just_pressed(KeyCode::Space))
                .then_some(current)
        });
        if let Some(index) = confirmed {
            if session.machine.choose(index).is_ok() {
                session.choosing = false;
                session.presentation.selected_choice = 0;
                let screen = next_screen(&mut session, &mut commands, &assets);
                update_screen(screen, &session.game.theme, &mut texts);
            }
        }
    } else if input.just_pressed(KeyCode::Enter) || input.just_pressed(KeyCode::Space) {
        let screen = next_screen(&mut session, &mut commands, &assets);
        update_screen(screen, &session.game.theme, &mut texts);
    }
}

/// Data directory override is especially useful for test isolation.
/// A game id is already validated as a restricted logical identifier.
fn save_file(game_id: &str) -> Result<PathBuf, String> {
    let base = if let Some(path) = env::var_os("VESPER_SAVE_DIR") {
        PathBuf::from(path)
    } else if let Some(path) = env::var_os("XDG_DATA_HOME") {
        PathBuf::from(path).join("vesper/saves")
    } else if let Some(home) = env::var_os("HOME") {
        PathBuf::from(home).join(".local/share/vesper/saves")
    } else {
        return Err("save directory is unavailable (set VESPER_SAVE_DIR)".into());
    };
    Ok(base.join(format!("{game_id}.json")))
}

fn trigger_save_load(
    keyboard: Res<ButtonInput<KeyCode>>,
    session: Res<Session>,
    completed: Res<CompletedIo>,
    workers: Res<IoWorkers>,
) {
    if !(keyboard.just_pressed(KeyCode::F5) || keyboard.just_pressed(KeyCode::F9)) {
        return;
    }
    let path = match save_file(&session.game.manifest.id) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Save path error: {error}");
            return;
        }
    };
    let mut active = match workers.0.lock() {
        Ok(jobs) => jobs,
        Err(_) => { error!("Vesper save worker lock is unavailable"); return; }
    };
    active.retain(|job| !job.is_finished());
    if !active.is_empty() {
        warn!("Vesper save/load operation already in progress");
        return;
    }
    let queue = completed.0.clone();
    let worker = if keyboard.just_pressed(KeyCode::F5) {
        let save = SaveData::new(
            &session.game, &session.machine, session.presentation.clone(),
        );
        std::thread::spawn(move || {
            let result = save.write_atomic(&path);
            if let Ok(mut messages) = queue.lock() {
                messages.push(IoResult::Saved(result));
            }
        })
    } else {
        let game = session.game.clone();
        std::thread::spawn(move || {
            let result = SaveData::read(&path).and_then(|save| {
                let restored = save.restore(&game)?;
                Ok((restored, save.presentation))
            });
            if let Ok(mut messages) = queue.lock() {
                messages.push(IoResult::Loaded(result));
            }
        })
    };
    active.push(worker);
}

fn poll_completed_io(
    completed: Res<CompletedIo>,
    mut session: ResMut<Session>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texts: Query<(&mut Text, &mut TextFont, &mut TextColor, &StoryText)>,
) {
    let events = match completed.0.lock() {
        Ok(mut events) => std::mem::take(&mut *events),
        Err(_) => return,
    };
    for event in events {
        match event {
            IoResult::Saved(Ok(())) => info!("Vesper save written"),
            IoResult::Saved(Err(error)) => error!("Vesper save failed: {error}"),
            IoResult::Loaded(Err(error)) => error!("Vesper load failed: {error}"),
            IoResult::Loaded(Ok((machine, state))) => {
                if let Some(entity) = session.background.take() {
                    commands.entity(entity).despawn();
                }
                for (_, entity) in std::mem::take(&mut session.characters) {
                    commands.entity(entity).despawn();
                }
                session.presentation = PresentationState::default();
                session.presentation.selected_choice = state.selected_choice;
                if let Some(id) = state.music {
                    if let Err(error) = apply_visual(
                        &mut session, &mut commands, &assets, Visual::Music(id)
                    ) {
                        error!("Invalid saved music: {error}");
                        continue;
                    }
                } else if let Some(previous) = session.music.take() {
                    commands.entity(previous).despawn();
                }
                if let Some(id) = state.background {
                    if let Err(error) = apply_visual(
                        &mut session, &mut commands, &assets, Visual::Background(id)
                    ) {
                        error!("Invalid saved background: {error}");
                        continue;
                    }
                }
                for (id, pose) in state.characters {
                    if let Err(error) = apply_visual(
                        &mut session, &mut commands, &assets,
                        Visual::Show {
                            character: id,
                            expression: pose.expression,
                            position: pose.position,
                        },
                    ) {
                        error!("Invalid saved character: {error}");
                        continue;
                    }
                }
                let last = machine.last_output().cloned();
                session.machine = machine;
                session.choosing = matches!(last, Some(Output::Choices(_)));
                session.finished = matches!(last, Some(Output::End));
                let screen = match last {
                    Some(Output::Dialogue { speaker, text }) =>
                        Screen::normal(speaker.unwrap_or_default(), text),
                    Some(Output::Choices(choices)) =>
                        Screen::choices(&choices, session.presentation.selected_choice),
                    Some(Output::End) => Screen::ending("The End".into()),
                    _ => Screen::ending("Story restored".into()),
                };
                update_screen(screen, &session.game.theme, &mut texts);
                info!("Vesper save restored");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fade_fraction;
    #[test]
    fn fade_progress_is_bounded_and_reaches_full_opacity() {
        assert_eq!(fade_fraction(0.0, 2.0), 0.0);
        assert_eq!(fade_fraction(1.0, 2.0), 0.5);
        assert_eq!(fade_fraction(2.0, 2.0), 1.0);
        assert_eq!(fade_fraction(3.0, 2.0), 1.0);
    }
}


/// Verify that external audio has been decoded into Bevy assets. CI has no
/// physical audio sink, so this asserts decoding rather than audible output.
fn monitor_audio_assets(mut session: ResMut<Session>, server: Res<AssetServer>) {
    session.pending_audio.retain(|(name, handle)| {
        match server.load_state(handle.id()) {
            LoadState::Loaded => {
                info!("Vesper decoded audio asset: {name}");
                false
            }
            LoadState::Failed(error) => {
                error!("Vesper audio asset failed ({name}): {error}");
                false
            }
            _ => true,
        }
    });
}
