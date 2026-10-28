//! Bevy 0.20 presentation shell. All authored content and artwork is external.

use bevy::{asset::{AssetPlugin, LoadState}, prelude::*, text::TextLayoutInfo, window::PrimaryWindow};
mod reload;
mod settings;
use std::{collections::BTreeMap, env, path::PathBuf, process, sync::{Arc, Mutex}};
use vesper_game::{Game, preferences::{default_file, generic_font_available, PlayerPreferences}, save::{CharacterPose, PresentationState, SaveData}};
use vesper_story::{Machine, Output, Visual};

#[derive(Resource)]
struct Session {
    game: Game,
    /// Player-owned overrides loaded once before entering Bevy's update loop.
    preferences: PlayerPreferences,
    machine: Machine,
    choosing: bool,
    finished: bool,
    ending_selection: usize,
    paused: bool,
    pause_overlay: Option<Entity>,
    pause_selection: usize,
    menu_overlay: Option<Entity>,
    menu_selection: usize,
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
struct BackgroundVisual;

#[derive(Component)]
struct CharacterVisual {
    /// Author-defined x coordinate in the 1280x720 design canvas.
    anchor_x: f32,
}

#[derive(Clone, Copy, Debug)]
struct ViewportMetrics {
    background_scale: f32,
    character_scale: f32,
    ui_scale: f32,
}

/// Background art uses aspect-preserving "cover" cropping, while characters
/// and the interface fit the full design canvas. Never stretch raster pixels.
fn viewport_metrics(size: Vec2, authored_ui_scale: f32) -> ViewportMetrics {
    let viewport = size.max(Vec2::ONE);
    let factors = viewport / Vec2::new(1280.0, 720.0);
    let fit = factors.x.min(factors.y);
    ViewportMetrics {
        background_scale: factors.x.max(factors.y),
        character_scale: fit,
        ui_scale: authored_ui_scale * fit.clamp(0.6, 2.5),
    }
}

#[derive(Clone, Copy, Debug)]
struct PanelMetrics {
    bottom: f32,
    min_height: f32,
    padding: f32,
    gap: f32,
}

/// Fit the authored panel within the usable part of a small window, while
/// keeping its external theme width and contrast. No layout work blocks UI.
fn panel_metrics(viewport: Vec2, panel: &vesper_game::PanelStyle) -> PanelMetrics {
    let width = viewport.x.max(1.0);
    let height = viewport.y.max(1.0);
    let compact = (width / 1280.0).min(height / 720.0).clamp(0.60, 1.0);
    PanelMetrics {
        bottom: (height * 0.033).clamp(8.0, 24.0),
        min_height: panel.min_height.min(height * 0.48),
        padding: panel.padding * compact,
        gap: (panel.gap * compact).max(3.0),
    }
}

fn apply_panel_metrics(node: &mut Node, panel: &vesper_game::PanelStyle, metrics: PanelMetrics) {
    node.bottom = px(metrics.bottom);
    node.left = percent((100.0 - panel.width_percent) / 2.0);
    node.width = percent(panel.width_percent);
    node.min_height = px(metrics.min_height);
    node.padding = px(metrics.padding).all();
    node.row_gap = px(metrics.gap);
}

/// Work in logical window dimensions: Wayland's per-monitor scale factor is
/// already handled by Bevy. Recalculate every frame so spawned/reloaded
/// sprites inherit the current size, even when the window did not just resize.
fn fit_viewport(
    windows: Query<&Window, With<PrimaryWindow>>,
    session: Res<Session>,
    mut ui_scale: ResMut<UiScale>,
    mut backgrounds: Query<&mut Transform, (With<BackgroundVisual>, Without<CharacterVisual>)>,
    mut characters: Query<(&CharacterVisual, &mut Transform), (With<CharacterVisual>, Without<BackgroundVisual>)>,
    mut panels: Query<&mut Node, With<StoryPanel>>,
    mut previous: Local<(Vec2, [u32; 4])>,
) {
    let Ok(window) = windows.single() else { return; };
    let size = Vec2::new(window.resolution.width(), window.resolution.height());
    if size.x < 1.0 || size.y < 1.0 { return; }
    let m = viewport_metrics(size, session.game.theme.ui_scale);
    let resolved = effective_theme(&session);
    let panel = &resolved.panel;
    let panel_key = [
        panel.width_percent.to_bits(),
        panel.min_height.to_bits(),
        panel.padding.to_bits(),
        panel.gap.to_bits(),
    ];
    // Avoid marking the Bevy UI layout dirty every frame. The panel is
    // reflowed only after a viewport or external theme-geometry change.
    if previous.0 != size || previous.1 != panel_key {
        let geometry = panel_metrics(size, panel);
        for mut node in &mut panels {
            apply_panel_metrics(&mut node, panel, geometry);
        }
        previous.1 = panel_key;
    }
    if (ui_scale.0 - m.ui_scale).abs() > 0.001 {
        ui_scale.0 = m.ui_scale;
    }
    for mut transform in &mut backgrounds {
        transform.scale = Vec3::splat(m.background_scale);
    }
    for (anchor, mut transform) in &mut characters {
        transform.scale = Vec3::splat(m.character_scale);
        transform.translation.x = anchor.anchor_x * m.character_scale;
        transform.translation.y = -30.0 * m.character_scale;
    }
    if previous.0 != size {
        previous.0 = size;
        info!(
            "Vesper viewport resized: {}x{}; scene scale {:.3}; background cover {:.3}",
            size.x.round(), size.y.round(), m.character_scale, m.background_scale
        );
    }
}

#[derive(Component)]
struct StoryPanel;

#[derive(Component)]
struct StorySeparator;

#[derive(Component)]
struct StoryAccent;

/// One selection = one UI entity. Labels are never rebuilt as a menu string.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MenuKind { Pause, Main, Ending }

#[derive(Component)]
struct MenuRow { kind: MenuKind, index: usize }

#[derive(Component)]
struct EndingActions;

#[derive(Resource, Default)]
struct MenuPointer { action: Option<(MenuKind, usize)> }

/// Preserve the previous Up-from-first = Quit navigation shortcut.
fn menu_step(index: usize, direction: i32, count: usize) -> usize {
    if direction < 0 && index == 0 && count == 4 { return 2; }
    (index as i32 + direction).rem_euclid(count as i32) as usize
}

fn pointer_menu_rows(
    mut session: ResMut<Session>,
    mut pointer: ResMut<MenuPointer>,
    rows: Query<(&Interaction, &MenuRow), Changed<Interaction>>,
) {
    for (interaction, row) in &rows {
        let visible = match row.kind {
            MenuKind::Pause => session.paused,
            MenuKind::Main => session.menu_overlay.is_some(),
            MenuKind::Ending => session.finished && !session.paused && session.menu_overlay.is_none(),
        };
        if !visible || *interaction == Interaction::None { continue; }
        match row.kind {
            MenuKind::Pause => session.pause_selection = row.index,
            MenuKind::Main => session.menu_selection = row.index,
            MenuKind::Ending => session.ending_selection = row.index,
        }
        if *interaction == Interaction::Pressed {
            pointer.action = Some((row.kind, row.index));
        }
    }
}

const PAUSE_ITEMS: [&str; 4] = ["Resume story", "Main menu", "Quit", "Appearance settings"];
const MAIN_ITEMS: [&str; 4] = ["Start new story", "Continue current story", "Quit", "Appearance settings"];
const ENDING_ITEMS: [&str; 3] = ["Play again", "Main menu", "Quit"];

/// Retain row identity while focus changes: only selected row colors update.
fn menu_row_colors(kind: MenuKind, selected: bool, accent: [f32; 4]) -> (Color, Color) {
    if selected {
        let alpha = if kind == MenuKind::Ending { 0.50 } else { 0.94 };
        (
            Color::srgba(accent[0] * 0.27, accent[1] * 0.27, accent[2] * 0.27, alpha),
            Color::srgba(1.0, 0.98, 0.91, 1.0),
        )
    } else {
        (Color::NONE, Color::srgba(0.80, 0.84, 0.90, 1.0))
    }
}

fn spawn_menu_rows(
    parent: &mut ChildSpawnerCommands,
    kind: MenuKind,
    options: &[&str],
    font: &FontSource,
) {
    for (index, label) in options.iter().enumerate() {
        parent.spawn((
            Name::new(format!("{kind:?} action {}", index + 1)),
            MenuRow { kind, index },
            Button,
            Text::new(format!("{}. {}", index + 1, label)),
            TextFont { font: font.clone(), font_size: FontSize::Px(22.0), ..default() },
            TextColor(Color::WHITE),
            TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            Node { width: percent(100.0), padding: px(10.0).all(), ..default() },
            BackgroundColor(Color::NONE),
        ));
    }
}

fn sync_menu_selection(
    session: Res<Session>,
    mut rows: Query<(&MenuRow, &mut BackgroundColor, &mut TextColor)>,
    mut ending: Query<&mut Node, With<EndingActions>>,
) {
    for mut node in &mut ending {
        let display = if session.finished { Display::Flex } else { Display::None };
        if node.display != display { node.display = display; }
    }
    for (row, mut bg, mut fg) in &mut rows {
        let active = match row.kind {
            MenuKind::Pause => session.paused && row.index == session.pause_selection,
            MenuKind::Main => session.menu_overlay.is_some() && row.index == session.menu_selection,
            MenuKind::Ending => session.finished && row.index == session.ending_selection,
        };
        let (background, foreground) =
            menu_row_colors(row.kind, active, session.game.theme.panel.accent);
        if bg.0 != background { bg.0 = background; }
        if fg.0 != foreground { fg.0 = foreground; }
    }
}


#[derive(Component, Debug)]
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
            hint: "Enter / Space: continue   P: pause   M: menu   F5: save   F9: load   Esc: quit".into(),
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
            hint: "Arrows: move   Enter: select   Number: choose   P: pause   M: menu   F5: save   F9: load   Esc: quit".into(),
            choices: true,
        }
    }

    fn ending(message: String) -> Self {
        Self {
            speaker: String::new(),
            body: message,
            hint: "Up/Down: move   Enter: select   1-3: choose   R: replay   M: menu   Esc: quit".into(),
            choices: false,
        }
    }

    fn part(&self, slot: &StoryText) -> &str {
        match slot {
            StoryText::Speaker => &self.speaker,
            StoryText::Body => &self.body,
            StoryText::Hint => &self.hint,
        }
    }
}

/// Compute the presentation effective at this instant. The authored game
/// theme remains untouched, so --watch picks up unoverridden changes.
fn effective_theme(session: &Session) -> vesper_game::Theme {
    let authored = &session.game.theme;
    session.preferences.resolve_theme(authored, |family| {
        generic_font_available(family)
            || authored.font.family.as_deref() == Some(family)
    })
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

/// Text is intrinsic-height content in a vertical panel. In particular, a
/// wrapped second line must never be flex-shrunk underneath the hint footer.
fn story_text_node(slot: &StoryText) -> Node {
    Node {
        width: percent(100.0),
        flex_shrink: 0.0,
        margin: if matches!(slot, StoryText::Hint) {
            UiRect::top(px(9.0))
        } else {
            UiRect::default()
        },
        ..default()
    }
}

fn add_text(
    parent: &mut ChildSpawnerCommands,
    slot: StoryText,
    content: &str,
    style: &vesper_game::TextStyle,
    font: &FontSource,
) {
    let node = story_text_node(&slot);
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
        node,
    ));
}

/// Bevy's UI flex measurement can under-report the height of wrapped text
/// at narrow window aspect ratios. Use the *actual shaped glyph bounds* as
/// the minimum UI height so the next sibling cannot occupy the same pixels.
/// Text shaping happens in Bevy's normal layout pipeline, not in this system.
/// We only update geometry when that measured height changes.
fn text_height_reservation(actual: f32) -> Option<f32> {
    if !actual.is_finite() || actual <= 0.0 {
        None
    } else {
        Some((actual + 10.0).ceil())
    }
}

fn reserve_story_text_height(
    mut texts: Query<(&StoryText, &TextLayoutInfo, &mut Node)>,
) {
    for (slot, glyphs, mut node) in &mut texts {
        let actual = glyphs.size.y;
        // Extra measured-height breathing room covers subpixel rounding and
        // keeps the hint visibly separate at compact/fractional scales.
        // The number of wrapped lines is still never guessed.
        let Some(required) = text_height_reservation(actual) else { continue; };
        if node.min_height != px(required) {
            node.min_height = px(required);
            if env::var_os("VESPER_TRACE_LAYOUT").is_some() {
                info!("Vesper text {:?}: rendered {:.1}px, reserved {:.1}px",
                    slot, actual, required);
            }
        }
    }
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
    // This read takes place before app.run(), never on the render/UI thread.
    // No file is written until the player explicitly edits preferences (VX02).
    let preferences = match default_file() {
        Ok(path) => {
            let (value, warning) = PlayerPreferences::load_or_default(&path);
            if let Some(message) = warning {
                eprintln!("Vesper player preferences ignored: {message}");
            }
            value
        }
        Err(message) => {
            eprintln!("Vesper player preferences unavailable: {message}");
            PlayerPreferences::default()
        }
    };
    let asset_folder = game.root.to_string_lossy().into_owned();
    let title = game.manifest.title.clone();
    let ui_scale = game.theme.ui_scale;
    let inbox = watch.then(|| reload::start(game.root.clone()));
    let (preferences_writer, preferences_shutdown) = settings::start_writer();
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
        .insert_resource(settings::SettingsState::default())
        .insert_resource(MenuPointer::default())
        .insert_resource(preferences_writer)
        .insert_resource(Session {
            game, preferences, machine, choosing: false, finished: false,
            ending_selection: 0,
            paused: false, pause_overlay: None,
            pause_selection: 0,
            menu_overlay: None, menu_selection: 0,
            background: None, music: None, characters: BTreeMap::new(),
            presentation: PresentationState::default(),
            pending_audio: Vec::new(),
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (
            pointer_menu_rows, settings::pointer, advance_from_keyboard,
            settings::sync_visuals, reserve_story_text_height, trigger_save_load,
            poll_completed_io, sync_menu_selection, animate_fades,
            monitor_audio_assets, escape_to_quit, fit_viewport,
        ).chain());
    if let Some(inbox) = inbox {
        app.insert_resource(inbox).add_systems(Update, reload::apply_reload);
    }
    app.run();
    preferences_shutdown.finish();
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
                BackgroundVisual,
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
                CharacterVisual { anchor_x: x },
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

/// Store the visible end state for keyboard selection and hot-reload refreshes.
fn finish_story(session: &mut Session, message: String) -> Screen {
    session.finished = true;
    session.ending_selection = 0;
    Screen::ending(message)
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
                    return finish_story(session, format!("Story asset error:\n{error}"));
                }
            }
            Ok(Output::End) => {
                info!("Vesper story reached ending scene: {}", session.machine.scene());
                return finish_story(session, "The End".into());
            }
            Err(error) => {
                return finish_story(session, format!("Story error:\n{error}"));
            }
        }
    }
    finish_story(session, "Story did not reach dialogue within 128 presentation events.".into())
}

fn setup(
    mut commands: Commands,
    mut session: ResMut<Session>,
    assets: Res<AssetServer>,
) {
    commands.spawn(Camera2d);
    let theme = effective_theme(&session);
    let font = text_font(&theme, &assets);
    let initial = next_screen(&mut session, &mut commands, &assets);
    let panel = &theme.panel;
    let geometry = panel_metrics(Vec2::new(1280.0, 720.0), panel);
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
                row_gap: px(geometry.gap),
                bottom: px(geometry.bottom),
                left: percent((100.0 - panel.width_percent) / 2.0),
                width: percent(panel.width_percent),
                min_height: px(geometry.min_height),
                padding: px(geometry.padding).all(),
                ..default()
            },
            BackgroundColor(Color::srgba(
                background[0], background[1], background[2], background[3],
            )),
            StoryPanel,
        ))
        .with_children(|parent| {
            parent.spawn((
                StoryAccent,
                Node { width: percent(100), height: px(3.0), flex_shrink: 0.0, ..default() },
                BackgroundColor(Color::srgba(
                    panel.accent[0], panel.accent[1], panel.accent[2], panel.accent[3]
                )),
            ));
            add_text(parent, StoryText::Speaker, &initial.speaker, &speaker_style, &font);
            add_text(parent, StoryText::Body, &initial.body, &body_style, &font);
            parent.spawn((
                EndingActions,
                Node {
                    display: Display::None,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(6.0),
                    width: percent(100.0),
                    ..default()
                },
            )).with_children(|list| {
                spawn_menu_rows(list, MenuKind::Ending, &ENDING_ITEMS, &font);
            });
            parent.spawn((
                StorySeparator,
                Node { width: percent(100), height: px(1.0), flex_shrink: 0.0, ..default() },
                BackgroundColor(Color::srgba(
                    panel.divider[0], panel.divider[1], panel.divider[2], panel.divider[3]
                )),
            ));
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



/// The overlay layout stays mounted for its entire menu lifetime; moving focus
/// never despawns entities, rewrites a giant Text node, or resets the story.
fn spawn_center_menu(
    commands: &mut Commands,
    assets: &AssetServer,
    theme: &vesper_game::Theme,
    title: &str,
    kind: MenuKind,
    options: &[&str],
    instructions: &str,
) -> Entity {
    let font = text_font(theme, assets);
    let accent = theme.panel.accent;
    commands.spawn((
        Name::new(format!("{kind:?} overlay")),
        Node {
            position_type: PositionType::Absolute,
            left: percent(0.0), top: percent(0.0),
            width: percent(100.0), height: percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.07, 0.78)),
    )).with_children(|parent| {
        parent.spawn((
            Name::new(format!("{kind:?} panel")),
            Node {
                width: percent(85.0),
                max_width: px(660.0),
                flex_direction: FlexDirection::Column,
                padding: px(26.0).all(),
                row_gap: px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.055, 0.072, 0.12, 0.97)),
        )).with_children(|panel| {
            panel.spawn((
                Text::new(title),
                TextFont { font: font.clone(), font_size: FontSize::Px(28.0), ..default() },
                TextColor(Color::srgba(1.0, 0.98, 0.92, 1.0)),
                TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            ));
            panel.spawn((
                Node { width: percent(100.0), height: px(2.0), ..default() },
                BackgroundColor(Color::srgba(accent[0], accent[1], accent[2], accent[3])),
            ));
            spawn_menu_rows(panel, kind, options, &font);
            panel.spawn((
                Text::new(instructions),
                TextFont { font, font_size: FontSize::Px(15.0), ..default() },
                TextColor(Color::srgba(0.72, 0.77, 0.84, 1.0)),
                TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                Node { margin: UiRect::top(px(6.0)), ..default() },
            ));
        });
    }).id()
}

fn dismiss_pause(session: &mut Session, commands: &mut Commands) {
    if let Some(overlay) = session.pause_overlay.take() {
        commands.entity(overlay).despawn();
    }
    session.paused = false;
    info!("Vesper story resumed");
}

fn show_pause_menu(
    session: &mut Session, commands: &mut Commands, assets: &AssetServer, selected: usize,
) {
    if let Some(overlay) = session.pause_overlay.take() {
        commands.entity(overlay).despawn();
    }
    let overlay = spawn_center_menu(
        commands, assets, &effective_theme(session), "PAUSED", MenuKind::Pause,
        &PAUSE_ITEMS,
        "Up/Down: select    Enter: confirm    P: resume    Esc: quit",
    );
    session.pause_overlay = Some(overlay);
    session.paused = true;
    session.pause_selection = selected;
    info!("Vesper story paused");
}

fn hide_main_menu(session: &mut Session, commands: &mut Commands) {
    if let Some(overlay) = session.menu_overlay.take() {
        commands.entity(overlay).despawn();
        info!("Vesper main menu closed");
    }
}

fn show_main_menu(
    session: &mut Session, commands: &mut Commands, assets: &AssetServer, selected: usize,
) {
    hide_main_menu(session, commands);
    let title = format!("{} — Main menu", session.game.manifest.title);
    let overlay = spawn_center_menu(
        commands, assets, &effective_theme(session), &title, MenuKind::Main, &MAIN_ITEMS,
        "Up/Down: select    Enter: confirm    M: close    Esc: quit",
    );
    session.menu_overlay = Some(overlay);
    session.menu_selection = selected;
    info!("Vesper main menu opened");
}

fn restart_story(
    session: &mut Session,
    commands: &mut Commands,
    assets: &AssetServer,
    texts: &mut Query<(&mut Text, &mut TextFont, &mut TextColor, &StoryText)>,
    via_menu: bool,
) {
    match Machine::new(&session.game.story, &session.game.manifest.entry) {
        Ok(machine) => {
            if let Some(background) = session.background.take() {
                commands.entity(background).despawn();
            }
            for (_, character) in std::mem::take(&mut session.characters) {
                commands.entity(character).despawn();
            }
            if let Some(music) = session.music.take() {
                commands.entity(music).despawn();
            }
            session.pending_audio.clear();
            session.presentation = PresentationState::default();
            session.machine = machine;
            session.choosing = false;
            session.finished = false;
            session.ending_selection = 0;
            let screen = next_screen(session, commands, assets);
            update_screen(screen, &effective_theme(session), texts);
            if via_menu {
                info!("Vesper story restarted from its beginning via menu");
            } else {
                info!("Vesper story restarted from its beginning");
            }
        }
        Err(error) => error!("Vesper restart failed: {error}"),
    }
}

fn advance_from_keyboard(
    input: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<Session>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texts: Query<(&mut Text, &mut TextFont, &mut TextColor, &StoryText)>,
    mut menu_exit: MessageWriter<AppExit>,
    mut settings_state: ResMut<settings::SettingsState>,
    preference_writer: Res<settings::PreferencesWriter>,
    mut pointer: ResMut<MenuPointer>,
) {
    if settings_state.overlay.is_some() {
        settings::keyboard(&input, &mut session, &mut settings_state,
            &mut commands, &assets, &preference_writer);
        return;
    }
    if session.menu_overlay.is_some() {
        if input.just_pressed(KeyCode::KeyM) {
            hide_main_menu(&mut session, &mut commands);
            return;
        }
        let selected = session.menu_selection;
        if input.just_pressed(KeyCode::ArrowDown) {
            session.menu_selection = menu_step(selected, 1, MAIN_ITEMS.len());
            return;
        }
        if input.just_pressed(KeyCode::ArrowUp) {
            session.menu_selection = menu_step(selected, -1, MAIN_ITEMS.len());
            return;
        }
        let digit = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4]
            .iter().position(|key| input.just_pressed(*key));
        let from_pointer = pointer.action.take()
            .and_then(|(kind, index)| (kind == MenuKind::Main).then_some(index));
        let confirmed = from_pointer.or(digit).or_else(|| {
            (input.just_pressed(KeyCode::Enter) || input.just_pressed(KeyCode::Space))
                .then_some(selected)
        });
        if let Some(index) = confirmed {
            match index {
                0 => {
                    hide_main_menu(&mut session, &mut commands);
                    restart_story(&mut session, &mut commands, &assets, &mut texts, true);
                }
                1 => hide_main_menu(&mut session, &mut commands),
                2 => {
                    info!("Vesper quitting through menu");
                    menu_exit.write(AppExit::Success);
                }
                3 => {
                    hide_main_menu(&mut session, &mut commands);
                    settings::open(&session, &mut settings_state, &mut commands,
                        &assets, MenuKind::Main);
                }
                _ => {}
            }
        }
        return;
    }
    if input.just_pressed(KeyCode::KeyM) {
        if session.paused {
            dismiss_pause(&mut session, &mut commands);
        }
        show_main_menu(&mut session, &mut commands, &assets, 0);
        return;
    }
    // The pause menu retains the existing P-to-resume behavior, and Space
    // remains ignored while paused (POL01 regression guarantee).
    if input.just_pressed(KeyCode::KeyP) && !session.finished {
        if session.paused {
            dismiss_pause(&mut session, &mut commands);
        } else {
            show_pause_menu(&mut session, &mut commands, &assets, 0);
        }
        return;
    }
    if session.paused {
        let selected = session.pause_selection;
        if input.just_pressed(KeyCode::ArrowDown) {
            session.pause_selection = menu_step(selected, 1, PAUSE_ITEMS.len());
            return;
        }
        if input.just_pressed(KeyCode::ArrowUp) {
            session.pause_selection = menu_step(selected, -1, PAUSE_ITEMS.len());
            return;
        }
        let digit = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4]
            .iter().position(|key| input.just_pressed(*key));
        let from_pointer = pointer.action.take()
            .and_then(|(kind, index)| (kind == MenuKind::Pause).then_some(index));
        let confirmed = from_pointer.or(digit)
            .or_else(|| input.just_pressed(KeyCode::Enter).then_some(selected));
        if let Some(index) = confirmed {
            match index {
                0 => dismiss_pause(&mut session, &mut commands),
                1 => {
                    dismiss_pause(&mut session, &mut commands);
                    show_main_menu(&mut session, &mut commands, &assets, 0);
                }
                2 => {
                    info!("Vesper quitting through pause menu");
                    menu_exit.write(AppExit::Success);
                }
                3 => {
                    dismiss_pause(&mut session, &mut commands);
                    settings::open(&session, &mut settings_state, &mut commands,
                        &assets, MenuKind::Pause);
                }
                _ => {}
            }
        }
        return;
    }
    if session.finished {
        let selected = session.ending_selection;
        let next = if input.just_pressed(KeyCode::ArrowDown) {
            Some((selected + 1) % 3)
        } else if input.just_pressed(KeyCode::ArrowUp) {
            Some((selected + 2) % 3)
        } else {
            None
        };
        if let Some(next) = next {
            session.ending_selection = next;
            return;
        }
        if input.just_pressed(KeyCode::KeyR) {
            restart_story(&mut session, &mut commands, &assets, &mut texts, false);
            return;
        }
        let digit = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
            .iter().position(|key| input.just_pressed(*key));
        let from_pointer = pointer.action.take()
            .and_then(|(kind, index)| (kind == MenuKind::Ending).then_some(index));
        let confirmed = from_pointer.or(digit).or_else(|| {
            (input.just_pressed(KeyCode::Enter) || input.just_pressed(KeyCode::Space))
                .then_some(selected)
        });
        if let Some(index) = confirmed {
            match index {
                0 => restart_story(&mut session, &mut commands, &assets, &mut texts, false),
                1 => show_main_menu(&mut session, &mut commands, &assets, 0),
                2 => {
                    info!("Vesper quitting through ending menu");
                    menu_exit.write(AppExit::Success);
                }
                _ => {}
            }
        }
        return;
    }
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
            update_screen(Screen::choices(&options, index), &effective_theme(&session), &mut texts);
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
                update_screen(screen, &effective_theme(&session), &mut texts);
            }
        }
    } else if input.just_pressed(KeyCode::Enter) || input.just_pressed(KeyCode::Space) {
        let screen = next_screen(&mut session, &mut commands, &assets);
        update_screen(screen, &effective_theme(&session), &mut texts);
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
    if (session.paused || session.menu_overlay.is_some()) || !(keyboard.just_pressed(KeyCode::F5) || keyboard.just_pressed(KeyCode::F9)) {
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
                session.ending_selection = 0;
                let screen = match last {
                    Some(Output::Dialogue { speaker, text }) =>
                        Screen::normal(speaker.unwrap_or_default(), text),
                    Some(Output::Choices(choices)) =>
                        Screen::choices(&choices, session.presentation.selected_choice),
                    Some(Output::End) => Screen::ending("The End".into()),
                    _ => Screen::ending("Story restored".into()),
                };
                update_screen(screen, &effective_theme(&session), &mut texts);
                info!("Vesper save restored");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{fade_fraction, viewport_metrics, panel_metrics};
    use bevy::{audio::Decodable, prelude::{AudioSource, Vec2}};
    use std::{fs, path::Path, sync::Arc};

    #[test]
    fn ending_actions_are_separate_widgets() {
        let screen = super::Screen::ending("The End".into());
        assert_eq!(screen.body, "The End");
        assert_eq!(super::ENDING_ITEMS, ["Play again", "Main menu", "Quit"]);
        assert_eq!(super::PAUSE_ITEMS[0], "Resume story");
        assert_eq!(super::MAIN_ITEMS[1], "Continue current story");
    }

    #[test]
    fn menu_focus_uses_independent_row_colors() {
        use super::MenuKind;
        let accent = [0.45, 0.75, 1.0, 1.0];
        let (idle_bg, idle_fg) = super::menu_row_colors(MenuKind::Pause, false, accent);
        let (active_bg, active_fg) = super::menu_row_colors(MenuKind::Pause, true, accent);
        assert_ne!(idle_bg, active_bg);
        assert_ne!(idle_fg, active_fg);
        assert_eq!(super::MAIN_ITEMS.len(), 4);
        assert_eq!(super::menu_step(0, -1, 4), 2);
    }

    #[test]
    fn decode_external_audio_through_the_actual_bevy_playback_decoder() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut decoded = 0;
        for game in ["game", "examples/clockwork-moon"] {
            let dir = workspace.join(game).join("assets/audio");
            if !dir.is_dir() { continue; }
            for entry in fs::read_dir(&dir).expect("list authored audio").flatten() {
                let path = entry.path();
                if !matches!(path.extension().and_then(|ext| ext.to_str()), Some("wav" | "ogg")) {
                    continue;
                }
                let bytes: Arc<[u8]> = fs::read(&path)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
                    .into();
                let source = AudioSource { bytes };
                let sample = source.decoder().next();
                assert!(sample.is_some(), "decoder returned no samples for {}", path.display());
                decoded += 1;
            }
        }
        assert_eq!(decoded, 10, "all 10 authored external WAVs must decode");
    }

    #[test]
    fn viewport_scales_equally_with_window_and_preserves_aspect() {
        let baseline = viewport_metrics(Vec2::new(1280.0, 720.0), 1.0);
        assert_eq!(baseline.background_scale, 1.0);
        assert_eq!(baseline.character_scale, 1.0);
        assert_eq!(baseline.ui_scale, 1.0);

        let full = viewport_metrics(Vec2::new(1920.0, 1080.0), 1.0);
        assert_eq!(full.background_scale, 1.5);
        assert_eq!(full.character_scale, 1.5);
        assert_eq!(full.ui_scale, 1.5);

        // Wide monitor: fill scene with cropping, retain character proportions.
        let wide = viewport_metrics(Vec2::new(2560.0, 1080.0), 1.0);
        assert_eq!(wide.background_scale, 2.0);
        assert_eq!(wide.character_scale, 1.5);
        assert_eq!(wide.ui_scale, 1.5);

        let narrow = viewport_metrics(Vec2::new(800.0, 700.0), 1.2);
        assert!(narrow.background_scale > narrow.character_scale);
        assert_eq!(narrow.character_scale, 0.625);
        assert_eq!(narrow.ui_scale, 0.75);
    }

    #[test]
    fn measured_text_geometry_has_a_height_allowance() {
        // Rendering-height reserve must be independent of a fixed number of
        // lines; glyph measurement controls it even if text changes.
        assert_eq!(super::text_height_reservation(32.3), Some(43.0));
        assert_eq!(super::text_height_reservation(0.0), None);
        assert_eq!(super::text_height_reservation(f32::NAN), None);
    }

    #[test]
    fn text_blocks_do_not_collapse_under_wrapping() {
        use super::StoryText;
        assert_eq!(super::story_text_node(&StoryText::Speaker).flex_shrink, 0.0);
        assert_eq!(super::story_text_node(&StoryText::Body).flex_shrink, 0.0);
        let footer = super::story_text_node(&StoryText::Hint);
        assert_eq!(footer.flex_shrink, 0.0);
        assert!(matches!(footer.margin.top, bevy::ui::Val::Px(value) if value >= 8.0));
    }

    #[test]
    fn compact_panel_respects_window_height_and_authored_theme() {
        let panel = vesper_game::PanelStyle {
            background: [0.04, 0.05, 0.09, 0.94],
            width_percent: 90.0,
            min_height: 226.0,
            padding: 22.0,
            gap: 8.0,
            ..Default::default()
        };
        let compact = panel_metrics(Vec2::new(640.0, 360.0), &panel);
        let standard = panel_metrics(Vec2::new(1280.0, 720.0), &panel);
        assert!(compact.padding < standard.padding);
        assert!(compact.min_height <= 360.0 * 0.48);
        assert!(compact.bottom > 0.0 && compact.bottom < 24.0);
        assert_eq!(standard.padding, panel.padding);
        assert_eq!(standard.min_height, panel.min_height);
    }

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


fn escape_to_quit(
    input: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
    mut settings: ResMut<settings::SettingsState>,
) {
    let just_closed = std::mem::take(&mut settings.just_closed);
    if input.just_pressed(KeyCode::Escape) && settings.overlay.is_none() && !just_closed {
        info!("Vesper exiting by Escape");
        exit.write(AppExit::Success);
    }
}
