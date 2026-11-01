//! Live player-owned appearance controls. The authored game is never edited.
//! Settings writes go to a single sequential background worker, not Bevy Update.
use super::*;
use bevy::ui_widgets::{Slider, SliderRange, SliderThumb, SliderValue, TrackClick, ValueChange};
use std::sync::mpsc::{self, Sender};

pub const ITEMS: [&str; 13] = [
    "Dialogue opacity", "Text size", "Font family",
    "Reveal speed", "Reduced motion", "Auto advance",
    "Auto advance delay", "High contrast", "Dialogue width",
    "Dialogue position", "Text ink", "Reset to game defaults", "Back",
];

#[derive(Resource)]
pub struct SettingsState {
    pub overlay: Option<Entity>,
    pub selection: usize,
    pub origin: MenuKind,
    pub just_closed: bool,
}
impl Default for SettingsState {
    fn default() -> Self {
        Self { overlay: None, selection: 0, origin: MenuKind::Pause, just_closed: false }
    }
}

#[derive(Component)]
pub struct SettingsRow(pub usize);
#[derive(Component)]
pub struct SettingsScroll;
#[derive(Component)]
pub struct SettingsLayoutPanel;
#[derive(Component)]
pub struct SettingsContentGrid;
#[derive(Component)]
pub struct SettingsOptionsColumn;
#[derive(Component)]
pub struct SettingsPreviewColumn;

/// Completed pointer/button activations enter the normal validated settings
/// input path. Transient Pressed markers are not reliable click events.
#[derive(Resource, Default)]
pub struct SettingsClicks(pub Option<usize>);
/// Pointer-activated left/right controls use exactly the keyboard edit path.
#[derive(Resource, Default)]
pub struct SettingsAdjustClicks(pub Option<(usize, i32)>);
/// Sliders emit normalized absolute positions while dragging or clicking the rail.
#[derive(Resource, Default)]
pub struct SettingsSliderClicks(pub Option<(usize, f32)>);
#[derive(Resource, Default)]
pub struct SettingsTabClicks(pub Option<usize>);
#[derive(Component)]
pub struct SettingsTab(pub usize);
#[derive(Component)]
pub struct SettingsTabArt(pub usize);
#[derive(Component)]
pub struct SettingsTabLabel(pub usize);
#[derive(Component)]
pub struct SettingsSectionHeading(pub usize);

const SECTION_START: [usize; 4] = [0, 4, 8, 11];
const SECTION_LABEL: [&str; 4] = ["Reading", "Accessibility", "Layout", "Actions"];
fn section(index: usize) -> usize {
    if index < 4 { 0 } else if index < 8 { 1 } else if index < 11 { 2 } else { 3 }
}
fn section_label(tab: usize, _active: bool) -> String {
    // The illuminated blue-and-brass plaque already indicates selection.
    // Avoid font-dependent diamond characters in the tab label itself.
    SECTION_LABEL[tab].to_owned()
}
#[derive(Component)]
pub struct SettingsRowLabel(usize);
#[derive(Component)]
pub struct SettingsRowValue(usize);
#[derive(Component)]
pub struct SettingsSliderFill(usize);
#[derive(Component)]
pub struct SettingsSliderThumb(usize);
#[derive(Component)]
pub struct SettingsSliderControl(usize);
#[derive(Component)]
pub struct SettingsPreviewPanel;
#[derive(Component, Clone, Copy)]
pub enum Preview { Speaker, Body, Choice, Hint }

#[derive(Resource)]
pub struct PreferencesWriter(Option<Sender<Option<PlayerPreferences>>>);
pub struct WriterShutdown {
    sender: Option<Sender<Option<PlayerPreferences>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl WriterShutdown {
    pub fn finish(mut self) {
        // The stop sentinel is ordered after any edits sent from Bevy.
        if let Some(sender) = self.sender.take() { let _ = sender.send(None); }
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() { eprintln!("Vesper preference worker panicked"); }
        }
    }
}
pub fn start_writer() -> (PreferencesWriter, WriterShutdown) {
    let Ok(path) = default_file() else {
        return (PreferencesWriter(None), WriterShutdown { sender: None, thread: None });
    };
    let (tx, rx) = mpsc::channel::<Option<PlayerPreferences>>();
    let thread = std::thread::spawn(move || {
        while let Ok(Some(mut current)) = rx.recv() {
            // Coalesce rapid keyboard clicks, retaining strict last-change wins.
            let mut should_stop = false;
            while let Ok(next) = rx.try_recv() {
                if let Some(prefs) = next { current = prefs; }
                else { should_stop = true; break; }
            }
            if let Err(error) = current.write_atomic(&path) {
                eprintln!("Vesper preference write failed: {error}");
            } else {
                println!("Vesper player preferences saved");
            }
            if should_stop { break; }
        }
    });
    (PreferencesWriter(Some(tx.clone())), WriterShutdown {
        sender: Some(tx), thread: Some(thread),
    })
}
pub(super) fn persist(session: &Session, writer: &PreferencesWriter) {
    if let Some(tx) = &writer.0 {
        if tx.send(Some(session.preferences.clone())).is_err() {
            error!("Vesper preference writer stopped unexpectedly");
        }
    } else {
        warn!("Vesper preference storage unavailable; changes apply only this session");
    }
}

fn label(session: &Session, index: usize) -> String {
    match index {
        0 => {
            let value = session.preferences.panel_opacity.map_or_else(
                || "Game default".to_owned(), |v| format!("{:.0}%", v * 100.0));
            format!("Opacity   <  {value}  >")
        }
        1 => {
            let value = session.preferences.text_scale.map_or_else(
                || "Game default".to_owned(), |v| format!("{:.0}%", v * 100.0));
            format!("Text size   <  {value}  >")
        }
        2 => format!("Font browser   [ {} ]", session.preferences.font_family.as_deref().unwrap_or("Game default")),
        6 => format!("Auto delay   <  {:.1} s  >",
            session.preferences.auto_advance_delay.unwrap_or(1.5)),
        7 => format!("High contrast   <  {}  >",
            if session.preferences.high_contrast.unwrap_or(false) { "On" } else { "Off" }),
        8 => {
            let label = session.preferences.panel_width_percent.map_or_else(
                || "Game default".to_owned(),
                |v| format!("{v:.0}%"));
            format!("Dialogue width   <  {label}  >")
        },
        9 => format!("Dialogue position   <  {:.0}% above bottom  >",
            session.preferences.panel_bottom_percent.unwrap_or(session.game.theme.panel.bottom_percent)),
        10 => format!("Text ink   <  {}  >",
            session.preferences.text_ink.map_or("Game default", |ink| ink.label())),
        11 => "Reset to game defaults".into(),
        12 => "Back to menu".into(),
        3 => {
            let speed = session.preferences.reveal_speed
                .unwrap_or(session.game.theme.reveal_speed);
            let setting = match speed {
                vesper_game::TextRevealSpeed::Instant => "Instant",
                vesper_game::TextRevealSpeed::Slow => "Slow",
                vesper_game::TextRevealSpeed::Normal => "Normal",
                vesper_game::TextRevealSpeed::Fast => "Fast",
            };
            format!("Text reveal   <  {setting}  >")
        }
        4 => format!("Reduced motion   <  {}  >",
            if session.preferences.reduced_motion.unwrap_or(false) { "On" } else { "Off" }),
        5 => format!("Auto advance   <  {}  >",
            if session.preferences.auto_advance.unwrap_or(false) { "On" } else { "Off" }),
        _ => String::new(),
    }
}

/// Data model for skinned controls. The widgets share the same preferences
/// and directional editing logic as the legacy rows; no duplicate settings.
fn slider_fraction(session: &Session, index: usize) -> Option<f32> {
    let (value, low, high) = match index {
        0 => (session.preferences.panel_opacity.unwrap_or_else(|| {
            panel_art::default_art_opacity(session)
        }), 0.60, 1.0),
        1 => (session.preferences.text_scale.unwrap_or(1.0), 0.8, 1.6),
        6 => (session.preferences.auto_advance_delay.unwrap_or(1.5), 0.5, 20.0),
        8 => (session.preferences.panel_width_percent.unwrap_or(
            session.game.theme.panel.width_percent
        ), 50.0, 100.0),
        9 => (session.preferences.panel_bottom_percent.unwrap_or(
            session.game.theme.panel.bottom_percent
        ), 0.0, 20.0),
        _ => return None,
    };
    Some(((value - low) / (high - low)).clamp(0.0, 1.0))
}

/// Snap a normalized drag to the same legal steps used by keyboard arrows.
/// Dragging does not accumulate rounding error across repeated updates.
fn slider_preferences(session: &Session, index: usize, fraction: f32) -> Option<PlayerPreferences> {
    if !fraction.is_finite() { return None; }
    let fraction = fraction.clamp(0.0, 1.0);
    let mut prefs = session.preferences.clone();
    match index {
        0 => prefs.panel_opacity = Some(0.60 + (fraction * 8.0).round() * 0.05),
        1 => prefs.text_scale = Some(0.80 + (fraction * 8.0).round() * 0.10),
        6 => prefs.auto_advance_delay = Some(0.5 + (fraction * 39.0).round() * 0.5),
        8 => prefs.panel_width_percent = Some(50.0 + (fraction * 10.0).round() * 5.0),
        9 => prefs.panel_bottom_percent = Some((fraction * 20.0).round()),
        _ => return None,
    }
    Some(prefs)
}

fn apply_preferences(session: &mut Session, next: PlayerPreferences, writer: &PreferencesWriter, name: &str) {
    if next == session.preferences { return; }
    if let Err(error) = next.validate() {
        warn!("Rejected invalid player setting: {error}");
        return;
    }
    session.preferences = next;
    persist(session, writer);
    info!("Vesper settings applied: {name}");
}

fn control_value(session: &Session, index: usize) -> String {
    let string = label(session, index);
    if let Some((_, tail)) = string.split_once("   <  ") {
        return tail.trim_end_matches("  >").to_owned();
    }
    if let Some((_, tail)) = string.split_once("   [ ") {
        return tail.trim_end_matches(" ]").to_owned();
    }
    if matches!(index, 11 | 12) { return "↵".into(); }
    string
}

fn step_scaled(value: f32, delta: i32, minimum: f32, maximum: f32, tick: f32) -> f32 {
    let steps = ((value - minimum) / tick).round() as i32 + delta;
    let limit = ((maximum - minimum) / tick).round() as i32;
    minimum + tick * steps.clamp(0, limit) as f32
}

/// Pure editing logic; no filesystem or ECS mutations.
fn edited(session: &Session, index: usize, delta: i32) -> PlayerPreferences {
    let mut next = session.preferences.clone();
    match index {
        0 => {
            let base = next.panel_opacity.unwrap_or_else(|| panel_art::default_art_opacity(session));
            next.panel_opacity = Some(step_scaled(base, delta, 0.60, 1.0, 0.05));
        }
        1 => {
            let base = next.text_scale.unwrap_or(1.0);
            next.text_scale = Some(step_scaled(base, delta, 0.80, 1.60, 0.10));
        }
        // QOL01: font changes go through the searchable installed-font picker.
        2 => {},
        6 => {
            let base = next.auto_advance_delay.unwrap_or(1.5);
            next.auto_advance_delay = Some(step_scaled(base, delta, 0.5, 20.0, 0.5));
        }
        7 => next.high_contrast = Some(!next.high_contrast.unwrap_or(false)),
        8 => {
            let base = next.panel_width_percent.unwrap_or(session.game.theme.panel.width_percent);
            next.panel_width_percent = Some(step_scaled(base, delta, 50.0, 100.0, 5.0));
        },
        9 => {
            let base = next.panel_bottom_percent.unwrap_or(session.game.theme.panel.bottom_percent);
            next.panel_bottom_percent = Some(step_scaled(base, delta, 0.0, 20.0, 1.0));
        }
        10 => {
            use vesper_game::preferences::TextInk as Ink;
            let choices = [None, Some(Ink::Black), Some(Ink::DarkBrown), Some(Ink::Navy), Some(Ink::Sepia)];
            let index = choices.iter().position(|ink| *ink == next.text_ink).unwrap_or(0);
            next.text_ink = choices[(index as i32 + delta).rem_euclid(choices.len() as i32) as usize];
        }
        11 => next.reset(),
        3 => {
            use vesper_game::TextRevealSpeed as Speed;
            let speeds = [Speed::Instant, Speed::Slow, Speed::Normal, Speed::Fast];
            let current = next.reveal_speed.unwrap_or(session.game.theme.reveal_speed);
            let index = speeds.iter().position(|s| *s == current).unwrap_or(0);
            let next_index = (index as i32 + delta).rem_euclid(speeds.len() as i32) as usize;
            next.reveal_speed = Some(speeds[next_index]);
        }
        4 => next.reduced_motion = Some(!next.reduced_motion.unwrap_or(false)),
        5 => next.auto_advance = Some(!next.auto_advance.unwrap_or(false)),
        _ => {}
    }
    next
}

pub fn change(session: &mut Session, index: usize, delta: i32, writer: &PreferencesWriter) {
    if index == 12 || index >= ITEMS.len() { return; }
    let next = edited(session, index, delta);
    apply_preferences(session, next, writer, ITEMS[index]);
}

pub fn open(
    session: &Session, state: &mut SettingsState,
    commands: &mut Commands, assets: &AssetServer, origin: MenuKind,
) {
    if state.overlay.is_some() { return; }
    state.selection = 0;
    state.origin = origin;
    let theme = effective_theme(session);
    let font = text_font(&theme, assets);
    let accent = theme.panel.accent;
    let mut bg = theme.panel.background;
    // The Settings sample needs an opaque paper swatch even when the story
    // renders an illustrated panel over a transparent underlying node.
    if bg[3] < 0.05 { bg[3] = 1.0; }
    let illustrated = session.game.manifest.menu_backgrounds.settings.is_some();
    let skin = session.game.manifest.menu_skins.settings.as_ref();
    let overlay = commands.spawn((
        Name::new("Player settings overlay"),
        MenuOverlayRoot,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.0), height: percent(100.0),
            align_items: AlignItems::Center,
            justify_content: if illustrated && skin.is_none() {
                JustifyContent::FlexStart
            } else { JustifyContent::Center },
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.07,
            if illustrated { 0.08 } else { 0.88 })),
    )).with_children(|screen| {
        if let Some(path) = session.game.manifest.menu_backgrounds.settings.as_deref() {
            info!("Vesper settings menu backdrop loaded: {path}");
            screen.spawn((
                Name::new("Settings custom backdrop"),
                MenuBackdrop,
                ImageNode {
                    image: assets.load(path.to_owned()),
                    image_mode: bevy::ui::widget::NodeImageMode::Stretch,
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100.0), height: percent(100.0),
                    left: px(0.0), top: px(0.0), ..default()
                },
            ));
            screen.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100.0), height: percent(100.0),
                    left: px(0.0), top: px(0.0), ..default()
                },
                BackgroundColor(Color::srgba(0.01, 0.018, 0.04, 0.18)),
            ));
        }
        if let Some(skin) = skin
            && let Some(path) = skin.decor_image.as_deref()
        {
            info!("Vesper settings menu decoration loaded: {path}");
            screen.spawn((
                Name::new("Settings independent decorative art"),
                MenuBackdrop,
                ImageNode {
                    image: assets.load(path.to_owned()),
                    color: Color::srgba(1.0, 1.0, 1.0, skin.decor_opacity),
                    image_mode: bevy::ui::widget::NodeImageMode::Stretch,
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0), top: px(0.0),
                    width: percent(100.0), height: percent(100.0),
                    ..default()
                },
            ));
        }
        screen.spawn((
            Name::new("Settings panel"),
            SettingsLayoutPanel,
            Node {
                width: if let Some(skin) = skin { percent(skin.panel_width_percent) }
                    else if illustrated { percent(70.0) } else { percent(88.0) },
                // The four illustrated tabs must never force the panel's
                // intrinsic min-content width beyond the compact viewport.
                min_width: if illustrated && skin.is_none() { px(300.0) }
                    else { px(0.0) },
                max_width: px(if skin.is_some() { 1050.0 } else { 720.0 }),
                max_height: percent(94.0),
                margin: if let Some(skin) = skin {
                    if skin.centered { UiRect::default() }
                    else { UiRect::left(percent(skin.panel_offset_percent)) }
                } else if illustrated { UiRect::left(percent(5.0)) }
                    else { UiRect::default() },
                border: if illustrated { px(if skin.is_some() { 3.0 } else { 2.0 }).all() }
                    else { UiRect::default() },
                overflow: Overflow::scroll_y(),
                padding: px(if skin.is_some() { 15.0 } else { 8.0 }).all(),
                row_gap: px(if skin.is_some() { 4.0 } else { 2.0 }),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BorderColor::all(skin.map_or(
                Color::srgba(0.86, 0.68, 0.38, 0.95), |skin| rgba(skin.border_color),
            )),
            BackgroundColor(skin.map_or(
                Color::srgba(0.05, 0.07, 0.12, if illustrated { 0.86 } else { 0.98 }),
                |skin| {
                    let mut p = skin.panel_color;
                    p[3] *= skin.panel_opacity;
                    rgba(p)
                },
            )),
            SettingsScroll,
            ScrollPosition(Vec2::ZERO),
        )).with_children(|panel| {
            if skin.is_some_and(|s| s.centered) {
                spawn_gold_ornament(panel);
            }
            if let Some(skin) = skin.filter(|s| s.centered) {
                let title_font = FontSource::Family("DejaVu Serif".into());
                spawn_ornate_menu_heading(panel,
                    skin.title.as_deref().unwrap_or("Settings"), &title_font, assets, skin);
            } else {
                panel.spawn((
                    Text::new(skin.and_then(|skin| skin.title.as_deref()).unwrap_or("Settings")),
                    TextFont { font: font.clone(),
                        font_size: FontSize::Px(skin.map_or(23.0, |skin| skin.title_size)), ..default() },
                    TextColor(skin.map_or(Color::WHITE, |skin| rgba(skin.title_color))),
                    TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                    Node { width: percent(100.0), ..default() },
                ));
            }
            panel.spawn((
                Node { width: percent(100.0), height: px(2.0), ..default() },
                BackgroundColor(Color::srgba(accent[0], accent[1], accent[2], accent[3])),
            ));
            if let Some(skin) = skin {
                panel.spawn((
                    Name::new("Atlas settings category tabs"),
                    Node {
                        width: percent(100.0), min_width: px(0.0),
                        flex_direction: FlexDirection::Row,
                        column_gap: px(5.0), flex_shrink: 0.0, ..default()
                    },
                )).with_children(|tabs| {
                    for group in 0..SECTION_LABEL.len() {
                        tabs.spawn((
                            SettingsTab(group),
                            Button, Hovered::default(),
                            Node {
                                // Equal-size tab columns, rather than four unbounded
                                // text/image min-content boxes that clip at 946px.
                                flex_grow: 1.0, flex_shrink: 0.0,
                                flex_basis: px(0.0), min_width: px(0.0),
                                // Preserve a full-height navy brass plaque even
                                // without the obsolete paper-colored padding.
                                height: px(31.0), min_height: px(31.0),
                                max_height: px(31.0),
                                padding: UiRect::default(),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                border: UiRect::default(), ..default()
                            },
                            // The 9-slice plaque fills the entire hit target;
                            // no paper-colored rectangle or inset white rim.
                            BackgroundColor(Color::NONE),
                            observe(move |_activate: On<Activate>,
                                mut tabs: ResMut<SettingsTabClicks>| {
                                tabs.0 = Some(group);
                            }),
                        )).with_children(|button| {
                            if let (Some(idle), Some(selected)) = (
                                &skin.button_idle_image, &skin.button_selected_image,
                            ) {
                                button.spawn((
                                    Name::new(format!("Atlas illustrated category tab {group}")),
                                    SettingsTabArt(group),
                                    ImageNode {
                                        image: assets.load(if group == section(state.selection) {
                                            selected.clone()
                                        } else { idle.clone() }),
                                        color: if group == section(state.selection) {
                                            Color::WHITE
                                        } else { Color::srgba(0.68, 0.75, 0.83, 1.0) },
                                        image_mode: bevy::ui::widget::NodeImageMode::Sliced(
                                            menu_button_slicer(skin.button_slice_border)),
                                        ..default()
                                    },
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: px(0.0), top: px(0.0),
                                        width: percent(100.0), height: percent(100.0),
                                        ..default()
                                    },
                                ));
                            }
                            button.spawn((
                                SettingsTabLabel(group),
                                Text::new(section_label(group, group == section(state.selection))),
                                TextFont { font: font.clone(), font_size: FontSize::Px(13.0), ..default() },
                                TextColor(rgba(skin.title_color)),
                                TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
                                Node { width: percent(100.0), min_width: px(0.0),
                                    flex_shrink: 1.0, ..default() },
                            ));
                        });
                    }
                });
            }
            panel.spawn((
                SettingsContentGrid,
                Node { width: percent(100.0), min_width: px(0.0),
                    // Start in the safe compact layout. Only a measured viewport
                    // may opt in to the desktop two-column arrangement.
                    flex_direction: FlexDirection::Column,
                    column_gap: px(12.0), row_gap: px(5.0), ..default() },
            )).with_children(|grid| {
                grid.spawn((
                    SettingsOptionsColumn,
                    Node { width: percent(100.0),
                        min_width: px(0.0), flex_shrink: 0.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4.0), ..default() },
                )).with_children(|options| {
            for i in 0..ITEMS.len() {
                if let Some(skin) = skin
                    && matches!(i, 0 | 4 | 8 | 11)
                {
                    let heading = match i {
                        0 => "READING & DISPLAY",
                        4 => "MOTION & ACCESSIBILITY",
                        8 => "PAGE COMPOSITION",
                        _ => "ACTIONS",
                    };
                    options.spawn((
                        SettingsSectionHeading(section(i)),
                        Text::new(heading),
                        TextFont { font: font.clone(), font_size: FontSize::Px(12.0), ..default() },
                        TextColor(rgba(skin.title_color)),
                        Node {
                            display: if section(i) == section(state.selection) {
                                Display::Flex
                            } else { Display::None },
                            margin: UiRect::top(px(6.0)), ..default()
                        },
                    ));
                }
                options.spawn((
                    Name::new(format!("Settings option {}", i + 1)),
                    Button,
                    Hovered::default(),
                    SettingsRow(i),
                    observe(move |_activate: On<Activate>, mut clicks: ResMut<SettingsClicks>| {
                        clicks.0 = Some(i);
                    }),
                    Node {
                        display: if skin.is_some() && section(i) != section(state.selection) {
                            Display::None
                        } else { Display::Flex },
                        width: percent(100.0), min_width: px(0.0),
                        min_height: skin.map_or(Val::Auto, |skin| px(skin.button_height.min(45.0))),
                        padding: if skin.is_some() { px(6.0).all() } else { px(3.0).all() },
                        border: if skin.is_some() { px(1.5).all() } else { UiRect::default() },
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        flex_shrink: 0.0, ..default()
                    },
                    BorderColor::all(skin.map_or(Color::NONE, |skin| rgba(skin.border_color))),
                    BackgroundColor(skin.map_or(Color::NONE, |skin| rgba(skin.normal_fill))),
                )).with_children(|button| {
                    button.spawn((
                        SettingsRowLabel(i),
                        Text::new(if skin.is_some() { ITEMS[i].to_owned() } else { label(session, i) }),
                        TextFont {
                            font: font.clone(),
                            font_size: FontSize::Px(skin.map_or(16.0, |skin| skin.button_text_size.min(17.0))),
                            ..default()
                        },
                        TextColor(skin.map_or(Color::WHITE, |skin| rgba(skin.normal_text))),
                    ));
                    if let Some(skin) = skin {
                        button.spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: px(6.0), min_width: px(0.0),
                                flex_shrink: 1.0,
                                ..default()
                            },
                        )).with_children(|controls| {
                            if i < 11 {
                                controls.spawn((
                                    Name::new(format!("Settings decrease {}", i)),
                                    Button, Hovered::default(),
                                    Node {
                                        width: px(22.0), min_height: px(27.0),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center, ..default()
                                    },
                                    BackgroundColor(rgba(skin.normal_fill)),
                                    BorderColor::all(rgba(skin.border_color)),
                                    observe(move |_activate: On<Activate>,
                                        mut adjust: ResMut<SettingsAdjustClicks>| {
                                        adjust.0 = Some((i, -1));
                                    }),
                                )).with_children(|arrow| {
                                    arrow.spawn((
                                        Text::new("‹"),
                                        TextFont { font: font.clone(), font_size: FontSize::Px(20.0), ..default() },
                                        TextColor(rgba(skin.selected_text)),
                                    ));
                                });
                            }
                            if let Some(frac) = slider_fraction(session, i) {
                                // A real Bevy slider: drag the thumb or click the track.
                                // Its model is player preferences, also edited by arrows.
                                controls.spawn((
                                    Name::new(format!("Settings draggable slider {i}")),
                                    SettingsSliderControl(i),
                                    // Clicking the rail must snap to the indicated value;
                                    // Drag remains available from the real thumb.
                                    Slider { track_click: TrackClick::Snap, ..default() },
                                    SliderRange::new(0.0, 1.0),
                                    SliderValue(frac), Hovered::default(),
                                    Node { width: px(170.0), height: px(22.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center, ..default() },
                                    observe(move |value: On<ValueChange<f32>>,
                                        mut sliders: ResMut<SettingsSliderClicks>| {
                                        sliders.0 = Some((i, value.value));
                                    }),
                                )).with_children(|bar| {
                                    bar.spawn((
                                        // The headless Slider (not its decorative
                                        // descendants) must own pointer drag events.
                                        Pickable::IGNORE,
                                        Node { width: percent(100.0), height: px(7.0), ..default() },
                                        BackgroundColor(Color::srgba(0.12, 0.16, 0.22, 1.0)),
                                    )).with_children(|rail| {
                                        rail.spawn((
                                            SettingsSliderFill(i), Pickable::IGNORE,
                                            Node { width: percent(frac * 100.0),
                                                height: percent(100.0), ..default() },
                                            BackgroundColor(rgba(skin.border_color)),
                                        ));
                                    });
                                    bar.spawn((
                                        SliderThumb, SettingsSliderThumb(i), Pickable::IGNORE,
                                        Node { position_type: PositionType::Absolute,
                                            left: px(frac * 158.0), top: px(2.0),
                                            width: px(12.0), height: px(18.0),
                                            border: px(1.0).all(), ..default() },
                                        // Gold on a dark track remains visible even when
                                        // the Settings row itself uses dark ink on paper.
                                        BackgroundColor(rgba(skin.border_color)),
                                        BorderColor::all(rgba(skin.selected_text)),
                                    ));
                                });
                            }
                            controls.spawn((
                                SettingsRowValue(i),
                                Text::new(control_value(session, i)),
                                TextFont {
                                    font: font.clone(),
                                    font_size: FontSize::Px(skin.button_text_size.min(15.0)),
                                    ..default()
                                },
                                TextColor(rgba(skin.selected_text)),
                                // Fixed-width live readouts keep the slider/arrow
                                // geometry stable when "Game default" becomes
                                // "65%" mid-pointer-drag. Font names have their
                                // own wider selector slot, without a slider.
                                TextLayout::new(Justify::Right, LineBreak::NoWrap),
                                Node {
                                    width: px(if i == 2 { 168.0 } else { 112.0 }),
                                    min_width: px(if i == 2 { 168.0 } else { 112.0 }),
                                    max_width: px(if i == 2 { 168.0 } else { 112.0 }),
                                    flex_shrink: 0.0, ..default()
                                },
                            ));
                            if i < 11 {
                                controls.spawn((
                                    Name::new(format!("Settings increase {}", i)),
                                    Button, Hovered::default(),
                                    Node {
                                        width: px(22.0), min_height: px(27.0),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center, ..default()
                                    },
                                    BackgroundColor(rgba(skin.normal_fill)),
                                    BorderColor::all(rgba(skin.border_color)),
                                    observe(move |_activate: On<Activate>,
                                        mut adjust: ResMut<SettingsAdjustClicks>| {
                                        adjust.0 = Some((i, 1));
                                    }),
                                )).with_children(|arrow| {
                                    arrow.spawn((
                                        Text::new("›"),
                                        TextFont { font: font.clone(), font_size: FontSize::Px(20.0), ..default() },
                                        TextColor(rgba(skin.selected_text)),
                                    ));
                                });
                            }
                        });
                    }
                });
            }
                });
                grid.spawn((
                    SettingsPreviewColumn,
                    Node { width: percent(100.0),
                        min_width: px(0.0), flex_shrink: 0.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: px(5.0), ..default() },
                )).with_children(|preview_column| {
            preview_column.spawn((
                Text::new("LIVE PREVIEW"),
                TextFont { font: font.clone(), font_size: FontSize::Px(14.0), ..default() },
                TextColor(Color::srgba(0.8, 0.82, 0.9, 1.0)),
            ));
            preview_column.spawn((
                SettingsPreviewPanel,
                Node {
                    width: percent(100.0), flex_direction: FlexDirection::Column,
                    row_gap: px(4.0), padding: px(if skin.is_some() { 13.0 } else { 8.0 }).all(),
                    border: if skin.is_some() { px(3.0).all() } else { UiRect::default() },
                    ..default()
                },
                BorderColor::all(skin.map_or(Color::NONE, |skin| rgba(skin.border_color))),
                BackgroundColor(Color::srgba(bg[0], bg[1], bg[2], bg[3])),
            )).with_children(|preview| {
                for (slot, content, style) in [
                    (Preview::Speaker, "Sample Speaker", &theme.speaker),
                    (Preview::Body, "A long sample dialogue line wraps naturally at different window sizes, keeping every word comfortable to read.", &theme.dialogue),
                    (Preview::Choice, "1. Continue the sample story", &theme.choices),
                    (Preview::Hint, "Enter: continue    P: pause", &theme.hint),
                ] {
                    if matches!(slot, Preview::Hint) && skin.is_some() {
                        // Keep the model slot for live preview/font sync, but
                        // render the example help as recognizable physical keys.
                        preview.spawn((
                            slot, Text::new(content),
                            TextFont { font: font.clone(), font_size: FontSize::Px(style.size), ..default() },
                            text_color(style.color),
                            Node { display: Display::None, ..default() },
                        ));
                        spawn_keyboard_legend(preview,
                            "Enter: Continue    P: Pause", &font, true);
                        continue;
                    }
                    let content_node = (
                        slot, Text::new(content),
                        TextFont {
                            font: font.clone(), font_size: FontSize::Px(style.size),
                            ..default()
                        },
                        text_color(style.color),
                        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                        Node { width: percent(100.0), flex_shrink: 0.0, ..default() },
                    );
                    if matches!(slot, Preview::Speaker) && let Some(skin) = skin {
                        // The actual gold speaker label sits on an opaque
                        // navy titleplate, not on parchment. Match that
                        // contrast in the live sample instead of painting
                        // pale gold directly over the light paper background.
                        preview.spawn((
                            Name::new("Illustrated speaker preview titleplate"),
                            Node {
                                width: percent(100.0),
                                padding: px(8.0).all(),
                                border: px(1.0).all(),
                                ..default()
                            },
                            BorderColor::all(rgba(skin.border_color)),
                            BackgroundColor(rgba(skin.panel_color)),
                        )).with_children(|nameplate| {
                            nameplate.spawn(content_node);
                        });
                    } else {
                        preview.spawn(content_node);
                    }
                }
            });
                });
            });
            spawn_keyboard_legend(panel,
                "↑↓: select    ←→: adjust    Enter: activate    Esc: back", &font, false);
        });
    }).id();
    state.overlay = Some(overlay);
    info!("Vesper appearance settings opened");
}

pub fn close(
    session: &mut Session, state: &mut SettingsState,
    commands: &mut Commands, assets: &AssetServer,
) {
    if let Some(entity) = state.overlay.take() { commands.entity(entity).despawn(); }
    state.just_closed = true;
    match state.origin {
        MenuKind::Pause => show_pause_menu(session, commands, assets, PAUSE_ITEMS.len() - 1),
        MenuKind::Main | MenuKind::Ending => show_main_menu(session, commands, assets, MAIN_ITEMS.len() - 1),
    }
    info!("Vesper appearance settings closed");
}

pub fn keyboard(
    input: &ButtonInput<KeyCode>, session: &mut Session,
    state: &mut SettingsState, browser: &mut font_browser::State,
    commands: &mut Commands, assets: &AssetServer,
    writer: &PreferencesWriter,
) {
    if input.just_pressed(KeyCode::Escape) || input.just_pressed(KeyCode::KeyP)
        || input.just_pressed(KeyCode::KeyM)
    {
        close(session, state, commands, assets);
    } else if input.just_pressed(KeyCode::ArrowDown) {
        state.selection = (state.selection + 1) % ITEMS.len();
        info!("Vesper appearance selection: {}", state.selection);
    } else if input.just_pressed(KeyCode::ArrowUp) {
        state.selection = (state.selection + ITEMS.len() - 1) % ITEMS.len();
        info!("Vesper appearance selection: {}", state.selection);
    } else if input.just_pressed(KeyCode::ArrowLeft) {
        if state.selection == 2 { font_browser::open(session, browser, commands); }
        else { change(session, state.selection, -1, writer); }
    } else if input.just_pressed(KeyCode::ArrowRight) {
        if state.selection == 2 { font_browser::open(session, browser, commands); }
        else { change(session, state.selection, 1, writer); }
    } else if input.just_pressed(KeyCode::Enter) || input.just_pressed(KeyCode::Space) {
        if state.selection == 12 { close(session, state, commands, assets); }
        else if state.selection == 2 { font_browser::open(session, browser, commands); }
        else { change(session, state.selection, 1, writer); }
    }
}

/// Pointer presses use the same action and validation as keyboard presses.
pub fn pointer(
    mut session: ResMut<Session>, mut state: ResMut<SettingsState>,
    writer: Res<PreferencesWriter>, mut commands: Commands,
    assets: Res<AssetServer>,
    mut clicks: ResMut<SettingsClicks>,
    mut adjust: ResMut<SettingsAdjustClicks>,
    mut sliders: ResMut<SettingsSliderClicks>,
    mut tabs: ResMut<SettingsTabClicks>,
    mut browser: ResMut<font_browser::State>,
) {
    if state.overlay.is_none() || browser.overlay.is_some() {
        clicks.0 = None;
        adjust.0 = None;
        sliders.0 = None;
        tabs.0 = None;
        return;
    }
    if let Some(group) = tabs.0.take() {
        clicks.0 = None;
        adjust.0 = None;
        sliders.0 = None;
        state.selection = SECTION_START[group];
        return;
    }
    if let Some((index, fraction)) = sliders.0.take() {
        clicks.0 = None;
        adjust.0 = None;
        state.selection = index;
        if let Some(next) = slider_preferences(&session, index, fraction) {
            apply_preferences(&mut session, next, &writer, ITEMS[index]);
        }
        return;
    }
    if let Some((index, direction)) = adjust.0.take() {
        // Child controls stop propagation to the row. Ignore any row click
        // captured in the same frame to avoid duplicate adjustments.
        clicks.0 = None;
        state.selection = index;
        if index == 2 {
            font_browser::open(&session, &mut browser, &mut commands);
        } else {
            change(&mut session, index, direction, &writer);
        }
        return;
    }
    if let Some(index) = clicks.0.take() {
        state.selection = index;
        if index == 12 { close(&mut session, &mut state, &mut commands, &assets); }
        else if index == 2 { font_browser::open(&session, &mut browser, &mut commands); }
        else { change(&mut session, index, 1, &writer); }
    }
}

/// A hard viewport bound is necessary for Bevy's scrollable column:
/// its nested button/image intrinsic widths otherwise keep the 1050px desktop
/// panel after the window shrinks to 946px, clipping all controls on the right.
/// Apply the current physical window to the actual live ECS panel.
fn responsive_panel_width(physical_width: f32, authored_percent: f32) -> f32 {
    // UI-scale-adjusted logical dimensions can exceed the actual X11 window;
    // never let that compensation defeat the *visible* compact frame bound.
    let width = (physical_width.max(0.0) * authored_percent / 100.0)
        .min(1050.0).max(0.0);
    if physical_width < 1100.0 { width.min(physical_width * 0.76) }
    else { width }
}

pub fn fit_panel_to_viewport(
    session: Res<Session>, state: Res<SettingsState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut panels: Query<(&mut Node, &ComputedNode, &UiGlobalTransform), With<SettingsLayoutPanel>>,
    mut last: Local<Option<(u32, u32, u32)>>,
) {
    if state.overlay.is_none() { return; }
    let Some(skin) = session.game.manifest.menu_skins.settings.as_ref() else { return; };
    if !skin.centered { return; }
    let Ok(window) = windows.single() else { return; };
    let physical = Vec2::new(window.resolution.width(), window.resolution.height());
    let width = px(responsive_panel_width(physical.x, skin.panel_width_percent));
    let key = (physical.x as u32, physical.y as u32);
    let sample = match *last {
        Some((x, y, frames)) if (x, y) == key => frames.saturating_add(1),
        _ => 0,
    };
    *last = Some((key.0, key.1, sample));
    for (mut node, actual, transform) in &mut panels {
        // A scrollable column's min-content contribution may otherwise
        // override a width or a max-width by itself; set both fixed bounds.
        if node.width != width { node.width = width; }
        if node.min_width != width { node.min_width = width; }
        if node.max_width != width { node.max_width = width; }
        if matches!(sample, 1 | 30) {
            info!(
                "Vesper settings geometry: window={}x{} ui_scale={:.3} target={:?} computed_width={:.1} center_x={:.1}",
                key.0, key.1, ui_scale.0, width, actual.size().x, transform.translation.x,
            );
        }
    }
}

/// Native Settings options and live sample share a spacious two-column row,
/// but stack inside the wheel-scrollable viewport on compact windows.
fn settings_side_by_side(width: f32, height: f32) -> bool {
    width >= 1150.0 && height >= 650.0
}

pub fn sync_content_layout(
    windows: Query<&Window, With<PrimaryWindow>>,
    session: Res<Session>, state: Res<SettingsState>,
    mut grids: Query<&mut Node, (With<SettingsContentGrid>, Without<SettingsOptionsColumn>, Without<SettingsPreviewColumn>)>,
    mut options: Query<&mut Node, (With<SettingsOptionsColumn>, Without<SettingsContentGrid>, Without<SettingsPreviewColumn>)>,
    mut previews: Query<&mut Node, (With<SettingsPreviewColumn>, Without<SettingsContentGrid>, Without<SettingsOptionsColumn>)>,
) {
    if state.overlay.is_none() { return; }
    if !session.game.manifest.menu_skins.settings.as_ref().is_some_and(|s| s.centered) { return; }
    let Ok(window) = windows.single() else { return; };
    let side = settings_side_by_side(window.resolution.width(), window.resolution.height());
    let desired = if side { FlexDirection::Row } else { FlexDirection::Column };
    for mut node in &mut grids {
        if node.flex_direction != desired {
            info!("Vesper Settings layout: window={:.0}x{:.0}, two_columns={side}",
                window.resolution.width(), window.resolution.height());
            node.flex_direction = desired;
        }
    }
    for mut node in &mut options {
        let width = percent(if side { 57.0 } else { 100.0 });
        if node.width != width { node.width = width; }
    }
    for mut node in &mut previews {
        let width = percent(if side { 40.0 } else { 100.0 });
        if node.width != width { node.width = width; }
    }
}

/// Measured, bounded Settings scrolling keeps the lower preview and shortcut
/// hints reachable on compact displays without touching player preferences.
fn settings_scroll_max(node: &ComputedNode) -> f32 {
    ((node.content_size().y - node.size().y).max(0.0)
        * node.inverse_scale_factor).max(0.0)
}

pub fn wheel(
    mut events: MessageReader<bevy::input::mouse::MouseWheel>,
    state: Res<SettingsState>,
    mut scrolling: Query<(&mut ScrollPosition, &ComputedNode), With<SettingsScroll>>,
) {
    use bevy::input::mouse::MouseScrollUnit;
    let mut delta = 0.0;
    for event in events.read() {
        delta -= event.y * match event.unit {
            MouseScrollUnit::Line => 65.0,
            MouseScrollUnit::Pixel => 1.0,
        };
    }
    if state.overlay.is_none() || delta == 0.0 { return; }
    for (mut pos, node) in &mut scrolling {
        let max = settings_scroll_max(node);
        pos.0.y = (pos.0.y + delta).clamp(0.0, max);
    }
}

/// Category plaques use authored art while pointer and keyboard focus
/// remain owned by the same native Bevy SettingsTab entities.
pub fn sync_tab_art(
    session: Res<Session>, state: Res<SettingsState>, assets: Res<AssetServer>,
    mut art: Query<(&SettingsTabArt, &mut ImageNode)>,
) {
    if !state.is_changed() || state.overlay.is_none() { return; }
    let Some(skin) = &session.game.manifest.menu_skins.settings else { return; };
    for (tab, mut image) in &mut art {
        let active = tab.0 == section(state.selection);
        let path = if active { &skin.button_selected_image }
            else { &skin.button_idle_image };
        if let Some(path) = path {
            let handle = assets.load(path.clone());
            if image.image != handle { image.image = handle; }
            let tint = if active { Color::WHITE }
                else { Color::srgba(0.68, 0.75, 0.83, 1.0) };
            if image.color != tint { image.color = tint; }
        }
    }
}

/// Keep native immutable Bevy SliderValue state consistent with edits from
/// keyboard, arrow buttons and slider track events. This is a separate small
/// system to keep the general Settings synchronizer within ECS parameter limits.
pub fn sync_slider_visuals(
    session: Res<Session>, mut commands: Commands,
    mut gauges: Query<(&SettingsSliderFill, &mut Node), (Without<SettingsRow>, Without<SettingsSliderThumb>)>,
    mut thumbs: Query<(&SettingsSliderThumb, &mut Node), (Without<SettingsRow>, Without<SettingsSliderFill>)>,
    sliders: Query<(Entity, &SettingsSliderControl, &SliderValue)>,
) {
    if !session.is_changed() { return; }
    for (slot, mut node) in &mut gauges {
        if let Some(frac) = slider_fraction(&session, slot.0) {
            node.width = percent(frac * 100.0);
        }
    }
    for (slot, mut node) in &mut thumbs {
        if let Some(frac) = slider_fraction(&session, slot.0) {
            node.left = px(frac * 158.0);
        }
    }
    for (entity, slot, value) in &sliders {
        if let Some(frac) = slider_fraction(&session, slot.0) {
            if (value.0 - frac).abs() > 0.0001 {
                commands.entity(entity).insert(SliderValue(frac));
            }
        }
    }
}

/// Updates existing ECS entities in place. The story position and choices are
/// never reset. Not scheduled on a worker because only UI component values change.
pub fn sync_visuals(
    session: Res<Session>, state: Res<SettingsState>,
    assets: Res<AssetServer>,
    mut panels: Query<&mut BackgroundColor, (With<StoryPanel>, Without<SettingsPreviewPanel>)>,
    mut texts: Query<(&StoryText, &mut TextFont, &mut TextColor, Has<HiddenHintText>), (Without<Preview>, Without<SettingsRowLabel>)>,
    mut preview_panels: Query<&mut BackgroundColor, (With<SettingsPreviewPanel>, Without<StoryPanel>, Without<SettingsRow>)>,
    mut previews: Query<(&Preview, &mut TextFont, &mut TextColor), (Without<StoryText>, Without<SettingsRowLabel>)>,
    mut labels: Query<(&SettingsRowLabel, &mut Text), (Without<Preview>, Without<StoryText>, Without<SettingsRowValue>)>,
    mut values: Query<(&SettingsRowValue, &mut Text), (Without<Preview>, Without<StoryText>, Without<SettingsRowLabel>)>,
    mut rows: Query<(&SettingsRow, &mut BackgroundColor, &mut BorderColor, &mut Node),
        (Without<StoryPanel>, Without<SettingsPreviewPanel>, Without<SettingsSliderFill>, Without<SettingsSliderThumb>, Without<SettingsSectionHeading>, Without<SettingsTab>)>,
    mut headings: Query<(&SettingsSectionHeading, &mut Node),
        (Without<SettingsRow>, Without<SettingsSliderFill>, Without<SettingsSliderThumb>, Without<SettingsTab>)>,
    mut tab_labels: Query<(&SettingsTabLabel, &mut Text),
        (Without<SettingsRowValue>, Without<SettingsRowLabel>, Without<Preview>, Without<StoryText>)>,
    mut scrolling: Query<&mut ScrollPosition, With<SettingsScroll>>,
    mut last: Local<Option<PlayerPreferences>>,
) {
    // Never rediscover fonts or touch Bevy text entities every frame.
    if !session.is_changed() && !state.is_changed() { return; }
    let theme = effective_theme(&session);
    if last.as_ref() != Some(&session.preferences) {
        let font = text_font(&theme, &assets);
        let mut p = theme.panel.background;
        if !session.preferences.high_contrast.unwrap_or(false)
            && panel_art::active_skin(&session).is_some_and(|skin| skin.transparent_background)
        {
            // Keep the illustration floating over the scene even if the
            // reader adjusts panel opacity; only the artwork fades.
            p[3] = 0.0;
        }
        for mut color in &mut panels {
            color.0 = Color::srgba(p[0], p[1], p[2], p[3]);
        }
        for (slot, mut text_font, mut color, hidden) in &mut texts {
            let style = match slot {
                StoryText::Speaker => &theme.speaker,
                StoryText::Body => if session.choosing { &theme.choices } else { &theme.dialogue },
                StoryText::Hint => &theme.hint,
            };
            text_font.font = font.clone();
            text_font.font_size = FontSize::Px(style.size);
            let mut tint = style.color;
            if hidden { tint[3] = 0.0; }
            *color = text_color(tint);
        }
        *last = Some(session.preferences.clone());
    }
    if state.overlay.is_none() { return; }
    for mut position in &mut scrolling {
        // Themed controls are taller and separated by section headings.
        let styled = session.game.manifest.menu_skins.settings.is_some();
        position.0.y = if styled { 0.0 }
            else { state.selection.saturating_sub(4) as f32 * 31.0 };
    }
    let font = text_font(&theme, &assets);
    let mut p = theme.panel.background;
    if p[3] < 0.05 { p[3] = 1.0; }
    for mut background in &mut preview_panels {
        background.0 = Color::srgba(p[0], p[1], p[2], p[3]);
    }
    for (slot, mut text_font, mut color) in &mut previews {
        let style = match slot {
            Preview::Speaker => &theme.speaker,
            Preview::Body => &theme.dialogue,
            Preview::Choice => &theme.choices,
            Preview::Hint => &theme.hint,
        };
        if text_font.font_size != FontSize::Px(style.size) { text_font.font_size = FontSize::Px(style.size); }
        text_font.font = font.clone();
        *color = text_color(style.color);
    }
    let skin = session.game.manifest.menu_skins.settings.as_ref();
    for (label_slot, mut text) in &mut labels {
        let value = if skin.is_some() { ITEMS[label_slot.0].to_owned() }
            else { label(&session, label_slot.0) };
        if text.0 != value { text.0 = value; }
    }
    for (slot, mut text) in &mut values {
        let value = control_value(&session, slot.0);
        if text.0 != value { text.0 = value; }
    }
    for (group, mut node) in &mut headings {
        let show = skin.is_none() || group.0 == section(state.selection);
        node.display = if show { Display::Flex } else { Display::None };
    }
    for (tab, mut text) in &mut tab_labels {
        let value = section_label(tab.0, tab.0 == section(state.selection));
        if text.0 != value { text.0 = value; }
    }
    for (row, mut background, mut border, mut node) in &mut rows {
        let selected = row.0 == state.selection;
        let show = skin.is_none() || section(row.0) == section(state.selection);
        node.display = if show { Display::Flex } else { Display::None };
        if let Some(skin) = skin {
            let (fill, _, edge) = menu_skin_row_colors(skin, selected);
            if background.0 != fill { background.0 = fill; }
            if *border != BorderColor::all(edge) { *border = BorderColor::all(edge); }
        } else {
            let a = theme.panel.accent;
            let color = if selected {
                Color::srgba(a[0] * 0.27, a[1] * 0.27, a[2] * 0.27, 0.92)
            } else { Color::NONE };
            if background.0 != color { background.0 = color; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atlas_settings_columns_reflow_only_with_sufficient_window_room() {
        assert!(settings_side_by_side(1280.0, 720.0));
        assert!(settings_side_by_side(1900.0, 1035.0));
        assert!(!settings_side_by_side(946.0, 511.0));
        assert!(!settings_side_by_side(1100.0, 700.0));
        assert!(!settings_side_by_side(1280.0, 600.0));
    }

    #[test]
    fn settings_scroll_bounds_follow_measured_content_and_ui_scale() {
        let mut node = ComputedNode::default();
        node.size = Vec2::new(780.0, 480.0);
        node.content_size = Vec2::new(780.0, 760.0);
        node.inverse_scale_factor = 1.0;
        assert_eq!(settings_scroll_max(&node), 280.0);
        node.inverse_scale_factor = 0.75;
        assert_eq!(settings_scroll_max(&node), 210.0);
        node.content_size.y = 300.0;
        assert_eq!(settings_scroll_max(&node), 0.0);
    }

    #[test]
    fn atlas_settings_panel_never_exceeds_compact_logical_viewport() {
        assert_eq!(responsive_panel_width(1280.0, 87.0), 1050.0);
        assert!((responsive_panel_width(946.0, 87.0) - 718.96).abs() < 0.001);
        assert!((responsive_panel_width(810.0, 87.0) - 615.6).abs() < 0.001);
        assert!(responsive_panel_width(1900.0, 87.0) <= 1050.0);
    }

    #[test]
    fn section_navigation_is_total_and_stable() {
        for index in 0..ITEMS.len() {
            let group = section(index);
            assert!(group < SECTION_LABEL.len());
            assert!(index >= SECTION_START[group]);
        }
        assert_eq!(section(0), 0);
        assert_eq!(section(4), 1);
        assert_eq!(section(8), 2);
        assert_eq!(section(12), 3);
        assert_eq!(section_label(2, true), "Layout");
        assert_eq!(section_label(2, false), "Layout");
    }
    #[test]
    fn stepping_is_bounded() {
        assert!((step_scaled(0.60, -1, 0.60, 1., 0.05) - 0.60).abs() < 0.001);
        assert!((step_scaled(0.95, 1, 0.60, 1., 0.05) - 1.).abs() < 0.001);
        assert!((step_scaled(10.60, 1, 0.80, 10.60, 0.10) - 10.60).abs() < 0.001);
        assert_eq!(ITEMS.len(), 13);
        assert_eq!(step_scaled(1.5, 1, 0.5, 20.0, 0.5), 2.0);
        assert_eq!(step_scaled(20.0, 1, 0.5, 20.0, 0.5), 20.0);
    }
}
