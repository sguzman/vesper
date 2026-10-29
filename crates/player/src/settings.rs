//! Live player-owned appearance controls. The authored game is never edited.
//! Settings writes go to a single sequential background worker, not Bevy Update.
use super::*;
use std::sync::mpsc::{self, Sender};

pub const ITEMS: [&str; 10] = [
    "Dialogue opacity", "Text size", "Font family",
    "Reveal speed", "Reduced motion", "Auto advance",
    "Auto advance delay", "High contrast", "Reset to game defaults", "Back",
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

/// Completed pointer/button activations enter the normal validated settings
/// input path. Transient Pressed markers are not reliable click events.
#[derive(Resource, Default)]
pub struct SettingsClicks(pub Option<usize>);
#[derive(Component)]
pub struct SettingsRowLabel(usize);
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
        8 => "Reset to game defaults".into(),
        9 => "Back to menu".into(),
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
            let base = next.panel_opacity.unwrap_or(session.game.theme.panel.background[3]);
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
        8 => next.reset(),
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
    if index == 9 || index >= ITEMS.len() { return; }
    let next = edited(session, index, delta);
    if next == session.preferences { return; }
    if let Err(error) = next.validate() {
        warn!("Rejected invalid player setting: {error}");
        return;
    }
    session.preferences = next;
    persist(session, writer);
    info!("Vesper settings applied: {}", ITEMS[index]);
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
    let bg = theme.panel.background;
    let overlay = commands.spawn((
        Name::new("Player settings overlay"),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.0), height: percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.07, 0.88)),
    )).with_children(|screen| {
        screen.spawn((
            Name::new("Appearance settings panel"),
            Node {
                width: percent(88.0), max_width: px(720.0),
                max_height: percent(94.0),
                overflow: Overflow::scroll_y(),
                padding: px(8.0).all(), row_gap: px(2.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.07, 0.12, 0.98)),
            SettingsScroll,
            ScrollPosition(Vec2::ZERO),
        )).with_children(|panel| {
            panel.spawn((
                Text::new("Appearance settings"),
                TextFont { font: font.clone(), font_size: FontSize::Px(23.0), ..default() },
                TextColor(Color::WHITE),
            ));
            panel.spawn((
                Node { width: percent(100.0), height: px(2.0), ..default() },
                BackgroundColor(Color::srgba(accent[0], accent[1], accent[2], accent[3])),
            ));
            for i in 0..ITEMS.len() {
                panel.spawn((
                    Name::new(format!("Settings option {}", i + 1)),
                    Button,
                    Hovered::default(),
                    SettingsRow(i),
                    observe(move |_activate: On<Activate>, mut clicks: ResMut<SettingsClicks>| {
                        clicks.0 = Some(i);
                    }),
                    Node {
                        width: percent(100.0), padding: px(3.0).all(),
                        flex_shrink: 0.0, ..default()
                    },
                    BackgroundColor(Color::NONE),
                )).with_children(|button| {
                    button.spawn((
                        SettingsRowLabel(i),
                        Text::new(label(session, i)),
                        TextFont {
                            font: font.clone(), font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            }
            panel.spawn((
                Text::new("LIVE PREVIEW"),
                TextFont { font: font.clone(), font_size: FontSize::Px(14.0), ..default() },
                TextColor(Color::srgba(0.8, 0.82, 0.9, 1.0)),
            ));
            panel.spawn((
                SettingsPreviewPanel,
                Node {
                    width: percent(100.0), flex_direction: FlexDirection::Column,
                    row_gap: px(2.0), padding: px(8.0).all(),
                    ..default()
                },
                BackgroundColor(Color::srgba(bg[0], bg[1], bg[2], bg[3])),
            )).with_children(|preview| {
                for (slot, content, style) in [
                    (Preview::Speaker, "Sample Speaker", &theme.speaker),
                    (Preview::Body, "A long sample dialogue line wraps naturally at different window sizes, keeping every word comfortable to read.", &theme.dialogue),
                    (Preview::Choice, "1. Continue the sample story", &theme.choices),
                    (Preview::Hint, "Enter: continue    P: pause", &theme.hint),
                ] {
                    preview.spawn((
                        slot, Text::new(content),
                        TextFont {
                            font: font.clone(), font_size: FontSize::Px(style.size),
                            ..default()
                        },
                        text_color(style.color),
                        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                        Node { width: percent(100.0), flex_shrink: 0.0, ..default() },
                    ));
                }
            });
            panel.spawn((
                Text::new("↑↓ select    ←→ adjust    Enter activate    Esc back"),
                TextFont { font, font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::srgba(0.76, 0.8, 0.86, 1.0)),
                TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
            ));
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
    } else if input.just_pressed(KeyCode::ArrowUp) {
        state.selection = (state.selection + ITEMS.len() - 1) % ITEMS.len();
    } else if input.just_pressed(KeyCode::ArrowLeft) {
        if state.selection == 2 { font_browser::open(session, browser, commands); }
        else { change(session, state.selection, -1, writer); }
    } else if input.just_pressed(KeyCode::ArrowRight) {
        if state.selection == 2 { font_browser::open(session, browser, commands); }
        else { change(session, state.selection, 1, writer); }
    } else if input.just_pressed(KeyCode::Enter) || input.just_pressed(KeyCode::Space) {
        if state.selection == 9 { close(session, state, commands, assets); }
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
    mut browser: ResMut<font_browser::State>,
) {
    if state.overlay.is_none() || browser.overlay.is_some() {
        clicks.0 = None;
        return;
    }
    if let Some(index) = clicks.0.take() {
        state.selection = index;
        if index == 9 { close(&mut session, &mut state, &mut commands, &assets); }
        else if index == 2 { font_browser::open(&session, &mut browser, &mut commands); }
        else { change(&mut session, index, 1, &writer); }
    }
}

/// Updates existing ECS entities in place. The story position and choices are
/// never reset. Not scheduled on a worker because only UI component values change.
pub fn sync_visuals(
    session: Res<Session>, state: Res<SettingsState>,
    assets: Res<AssetServer>,
    mut panels: Query<&mut BackgroundColor, (With<StoryPanel>, Without<SettingsPreviewPanel>)>,
    mut texts: Query<(&StoryText, &mut TextFont, &mut TextColor), (Without<Preview>, Without<SettingsRowLabel>)>,
    mut preview_panels: Query<&mut BackgroundColor, (With<SettingsPreviewPanel>, Without<StoryPanel>, Without<SettingsRow>)>,
    mut previews: Query<(&Preview, &mut TextFont, &mut TextColor), (Without<StoryText>, Without<SettingsRowLabel>)>,
    mut labels: Query<(&SettingsRowLabel, &mut Text), (Without<Preview>, Without<StoryText>)>,
    mut rows: Query<(&SettingsRow, &mut BackgroundColor), (Without<StoryPanel>, Without<SettingsPreviewPanel>)>,
    mut scrolling: Query<&mut ScrollPosition, With<SettingsScroll>>,
    mut last: Local<Option<PlayerPreferences>>,
) {
    // Never rediscover fonts or touch Bevy text entities every frame.
    if !session.is_changed() && !state.is_changed() { return; }
    let theme = effective_theme(&session);
    if last.as_ref() != Some(&session.preferences) {
        let font = text_font(&theme, &assets);
        let p = theme.panel.background;
        for mut color in &mut panels {
            color.0 = Color::srgba(p[0], p[1], p[2], p[3]);
        }
        for (slot, mut text_font, mut color) in &mut texts {
            let style = match slot {
                StoryText::Speaker => &theme.speaker,
                StoryText::Body => if session.choosing { &theme.choices } else { &theme.dialogue },
                StoryText::Hint => &theme.hint,
            };
            text_font.font = font.clone();
            text_font.font_size = FontSize::Px(style.size);
            *color = text_color(style.color);
        }
        *last = Some(session.preferences.clone());
    }
    if state.overlay.is_none() { return; }
    for mut position in &mut scrolling {
        position.0.y = state.selection.saturating_sub(4) as f32 * 31.0;
    }
    let font = text_font(&theme, &assets);
    let p = theme.panel.background;
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
    for (label_slot, mut text) in &mut labels {
        let value = label(&session, label_slot.0);
        if text.0 != value { text.0 = value; }
    }
    for (row, mut background) in &mut rows {
        let selected = row.0 == state.selection;
        let a = theme.panel.accent;
        let color = if selected {
            Color::srgba(a[0] * 0.27, a[1] * 0.27, a[2] * 0.27, 0.92)
        } else { Color::NONE };
        if background.0 != color { background.0 = color; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stepping_is_bounded() {
        assert!((step_scaled(0.60, -1, 0.60, 1., 0.05) - 0.60).abs() < 0.001);
        assert!((step_scaled(0.95, 1, 0.60, 1., 0.05) - 1.).abs() < 0.001);
        assert!((step_scaled(10.60, 1, 0.80, 10.60, 0.10) - 10.60).abs() < 0.001);
        assert_eq!(ITEMS.len(), 10);
        assert_eq!(step_scaled(1.5, 1, 0.5, 20.0, 0.5), 2.0);
        assert_eq!(step_scaled(20.0, 1, 0.5, 20.0, 0.5), 20.0);
    }
}
