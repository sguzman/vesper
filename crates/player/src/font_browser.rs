//! Cached host font discovery and a keyboard-first, paginated system font picker.
//! FontCx is queried once during Startup; there are no font scans in Update.
use super::*;
use bevy::{
    input::mouse::MouseWheel,
    input_focus::AutoFocus,
    text::{EditableText, FontCx, TextCursorStyle},
    ui_widgets::TextInput,
};

const PAGE_ROWS: usize = 8;
const MAX_FAMILIES: usize = 4096;
const SPECIMEN: &str = "Aa Bb Cc 0123456789  ¡Hola!  こんにちは  ✨";

#[derive(Resource, Default)]
pub struct State {
    pub overlay: Option<Entity>,
    pub selected: usize,
    pub results: Vec<String>,
    pub filter: String,
}

#[derive(Resource, Default)]
pub struct Click(pub Option<usize>);
#[derive(Component)]
pub(super) struct Search;
#[derive(Component)]
pub(super) struct Row(usize);
#[derive(Component)]
pub(super) struct RowLabel(usize);
#[derive(Component)]
pub(super) struct Specimen;
#[derive(Component)]
pub(super) struct Count;

/// Bevy's FontCx uses the same font database as FontSource::Family.
/// Do this at startup, before the player UI and per-frame update begins.
pub fn discover(mut session: ResMut<Session>, mut cx: ResMut<FontCx>) {
    let mut families: Vec<_> = cx.context.collection.family_names()
        .filter(|name| safe_family(name))
        .map(ToOwned::to_owned)
        .collect();
    // Plain lexical sorting also matches the allocation-free binary search
    // used by effective_theme when resolving a persisted family name.
    families.sort_unstable();
    families.dedup();
    if families.len() > MAX_FAMILIES {
        warn!("Vesper font catalog truncated to {MAX_FAMILIES} families");
        families.truncate(MAX_FAMILIES);
    }
    info!("Vesper discovered {} installed system font families", families.len());
    session.font_families = families;
}

fn safe_family(name: &str) -> bool {
    !name.is_empty() && name.len() <= 128
        && name.trim() == name && !name.chars().any(char::is_control)
}

fn choices(session: &Session) -> Vec<String> {
    let mut fonts = vec![
        "Game default".to_owned(), "sans-serif".to_owned(),
        "serif".to_owned(), "monospace".to_owned(),
    ];
    for family in &session.font_families {
        if !fonts.iter().any(|f| f.eq_ignore_ascii_case(family)) {
            fonts.push(family.clone());
        }
    }
    fonts
}

fn filtered(families: &[String], search: &str) -> Vec<String> {
    let needle = search.trim().to_lowercase();
    families.iter().filter(|family|
        family.to_lowercase().contains(&needle)
    ).cloned().collect()
}

fn start_index(state: &State) -> usize {
    state.selected.saturating_sub(3)
        .min(state.results.len().saturating_sub(PAGE_ROWS))
}

fn selected_font(state: &State) -> Option<&str> {
    state.results.get(state.selected).map(String::as_str)
}

pub fn open(session: &Session, state: &mut State, commands: &mut Commands) {
    if state.overlay.is_some() { return; }
    state.filter.clear();
    state.results = choices(session);
    let desired = session.preferences.font_family.as_deref().unwrap_or("Game default");
    state.selected = state.results.iter().position(|family| family == desired)
        .unwrap_or(0);
    let selected = selected_font(state).unwrap_or("Game default");
    let current_font = if selected == "Game default" {
        FontSource::sans_serif()
    } else {
        FontSource::Family(selected.to_owned().into())
    };
    let root = commands.spawn((
        Name::new("System font browser overlay"),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.0), height: percent(100.0),
            align_items: AlignItems::Center, justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.025, 0.05, 0.97)),
    )).with_children(|screen| {
        screen.spawn((
            Name::new("System font browser"),
            Node {
                width: percent(92.0), max_width: px(700.0),
                max_height: percent(94.0),
                padding: px(16.0).all(), row_gap: px(7.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.08, 0.14, 1.0)),
        )).with_children(|panel| {
            panel.spawn((
                Text::new("SYSTEM FONTS"),
                TextFont { font_size: FontSize::Px(22.0), ..default() },
                TextColor(Color::WHITE),
            ));
            panel.spawn((
                Text::new("Type to filter installed families. Up/Down: highlight; Enter: use; Esc: back."),
                TextFont { font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::srgba(0.82, 0.84, 0.92, 1.0)),
            ));
            panel.spawn((
                Search, TextInput, EditableText {
                    max_characters: Some(96), allow_newlines: false,
                    ..default()
                },
                TextCursorStyle::default(), AutoFocus,
                TextLayout::no_wrap(),
                TextFont { font_size: FontSize::Px(19.0),
                    font: FontSource::sans_serif(), ..default() },
                Node {
                    width: percent(100.0), min_height: px(33.0),
                    padding: px(5.0).all(), border: px(1.0).all(),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.70, 0.74, 0.92, 1.0)),
                BackgroundColor(Color::srgba(0.11, 0.14, 0.22, 1.0)),
            ));
            panel.spawn((
                Count,
                Text::new(format!("{} families · 1 selected", state.results.len())),
                TextFont { font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::srgba(0.8, 0.86, 0.97, 1.0)),
            ));
            // Only eight reusable rows are spawned even with thousands of fonts.
            panel.spawn((
                Node { flex_direction: FlexDirection::Column,
                    row_gap: px(3.0), width: percent(100.0), ..default() },
            )).with_children(|list| {
                for slot in 0..PAGE_ROWS {
                    list.spawn((
                        Row(slot), Button, Hovered::default(),
                        observe(move |_on: On<Activate>, mut click: ResMut<Click>| {
                            click.0 = Some(slot);
                        }),
                        Node {
                            width: percent(100.0), min_height: px(26.0),
                            padding: UiRect::axes(px(7.0), px(3.0)),
                            display: if slot < state.results.len() {
                                Display::Flex
                            } else { Display::None },
                            ..default()
                        },
                        BackgroundColor(if slot == state.selected {
                            Color::srgba(0.20, 0.27, 0.44, 1.0)
                        } else { Color::NONE }),
                    )).with_children(|button| {
                        button.spawn((
                            RowLabel(slot),
                            Text::new(state.results.get(slot).cloned().unwrap_or_default()),
                            TextFont { font_size: FontSize::Px(17.0),
                                font: FontSource::sans_serif(), ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
                }
            });
            panel.spawn((
                Text::new("LIVE FONT PREVIEW"),
                TextFont { font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::srgba(0.7, 0.8, 0.95, 1.0)),
            ));
            panel.spawn((
                Specimen, Text::new(SPECIMEN),
                TextFont {
                    font: current_font, font_size: FontSize::Px(24.0), ..default()
                },
                TextColor(Color::WHITE),
                TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                Node { width: percent(100.0), min_height: px(40.0),
                    flex_shrink: 0.0, ..default() },
            ));
        });
    }).id();
    state.overlay = Some(root);
    info!("Vesper system font browser opened: {} families", session.font_families.len());
}

pub fn close(state: &mut State, commands: &mut Commands) {
    if let Some(entity) = state.overlay.take() { commands.entity(entity).despawn(); }
    state.filter.clear();
    state.results.clear();
    info!("Vesper system font browser closed");
}

fn move_selection(state: &mut State, delta: i32) {
    if !state.results.is_empty() {
        let n = state.results.len() as i32;
        state.selected = (state.selected as i32 + delta).clamp(0, n - 1) as usize;
    }
}

fn apply(session: &mut Session, state: &mut State,
    commands: &mut Commands, writer: &settings::PreferencesWriter,
) {
    let Some(name) = selected_font(state) else { return; };
    let mut next = session.preferences.clone();
    next.font_family = (name != "Game default").then(|| name.to_owned());
    if let Err(error) = next.validate() {
        warn!("Vesper rejected font selection: {error}");
        return;
    }
    if next != session.preferences {
        session.preferences = next;
        settings::persist(session, writer);
    }
    info!("Vesper selected font: {name}");
    close(state, commands);
}

pub fn keyboard(input: &ButtonInput<KeyCode>,
    session: &mut Session, state: &mut State, commands: &mut Commands,
    writer: &settings::PreferencesWriter,
) {
    // P and M are ordinary characters when typing a font family query;
    // only Escape closes the browser without changing the user's selection.
    if input.just_pressed(KeyCode::Escape) {
        close(state, commands);
    } else if input.just_pressed(KeyCode::ArrowDown) {
        move_selection(state, 1);
    } else if input.just_pressed(KeyCode::ArrowUp) {
        move_selection(state, -1);
    } else if input.just_pressed(KeyCode::PageDown) {
        move_selection(state, PAGE_ROWS as i32);
    } else if input.just_pressed(KeyCode::PageUp) {
        move_selection(state, -(PAGE_ROWS as i32));
    } else if input.just_pressed(KeyCode::Enter) {
        apply(session, state, commands, writer);
    }
}

pub fn pointer(
    mut session: ResMut<Session>, mut state: ResMut<State>,
    mut clicks: ResMut<Click>,
    mut commands: Commands, writer: Res<settings::PreferencesWriter>,
) {
    if state.overlay.is_none() {
        clicks.0 = None;
        return;
    }
    if let Some(slot) = clicks.0.take() {
        let index = start_index(&state) + slot;
        if index < state.results.len() {
            state.selected = index;
            apply(&mut session, &mut state, &mut commands, &writer);
        }
    }
}

/// Keyboard paging works everywhere; wheel navigation is only consumed when
/// the browser is actually open. Event history is drained even when closed.
pub fn wheel(
    mut events: MessageReader<MouseWheel>,
    mut state: ResMut<State>,
) {
    let mut steps = 0;
    for event in events.read() {
        if event.y < 0.0 { steps += 3; }
        else if event.y > 0.0 { steps -= 3; }
    }
    if state.overlay.is_some() && steps != 0 {
        move_selection(&mut state, steps.clamp(-30, 30));
    }
}

/// Reading EditableText happens only when the font browser is open, and
/// matching/filtering happens only after text changed, never on every frame.
pub fn sync(
    mut state: ResMut<State>, search: Query<&EditableText, With<Search>>,
    mut rows: Query<(&Row, &mut Node, &mut BackgroundColor)>,
    mut labels: Query<(&RowLabel, &mut Text), (With<RowLabel>, Without<Specimen>, Without<Count>)>,
    mut specimen: Query<(&mut Text, &mut TextFont), (With<Specimen>, Without<RowLabel>, Without<Count>)>,
    mut counts: Query<&mut Text, (With<Count>, Without<Specimen>, Without<RowLabel>)>,
    session: Res<Session>,
) {
    if state.overlay.is_none() { return; }
    if let Ok(text) = search.single()
        && state.filter != text.value().to_string() {
        let old_selected = selected_font(&state).map(str::to_owned);
        state.filter = text.value().to_string();
        state.results = filtered(&choices(&session), &state.filter);
        state.selected = old_selected.and_then(|name|
            state.results.iter().position(|family| family == &name))
            .unwrap_or(0);
    }
    if !state.is_changed() { return; }
    let first = start_index(&state);
    for (slot, mut node, mut background) in &mut rows {
        let at = first + slot.0;
        node.display = if at < state.results.len() { Display::Flex } else { Display::None };
        background.0 = if at == state.selected && at < state.results.len() {
            Color::srgba(0.20, 0.27, 0.44, 1.0)
        } else { Color::NONE };
    }
    for (slot, mut text) in &mut labels {
        let value = state.results.get(first + slot.0)
            .map(String::as_str).unwrap_or("");
        if text.0 != value { text.0 = value.to_owned(); }
    }
    for mut text in &mut counts {
        let current = if state.results.is_empty() { 0 } else { state.selected + 1 };
        text.0 = format!("{} matching families · {current} highlighted ({} installed)",
            state.results.len(), session.font_families.len());
    }
    let selected = selected_font(&state).unwrap_or("Game default");
    for (mut text, mut font) in &mut specimen {
        text.0 = if state.results.is_empty() {
            "No matching installed font".to_owned()
        } else {
            SPECIMEN.to_owned()
        };
        font.font = if selected == "Game default" {
            FontSource::sans_serif()
        } else { FontSource::Family(selected.to_owned().into()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matching_is_case_insensitive_and_preserves_order() {
        let f = vec!["DejaVu Sans".into(), "Fira Code".into(), "Noto Sans".into()];
        assert_eq!(filtered(&f, "SANS"), vec!["DejaVu Sans", "Noto Sans"]);
        assert_eq!(filtered(&f, "  fIrA "), vec!["Fira Code"]);
        assert!(filtered(&f, "not-installed").is_empty());
    }
    #[test]
    fn families_reject_bad_persisted_values_and_page_indices_are_bounded() {
        assert!(!safe_family(" "));
        assert!(!safe_family("bad\nfont"));
        assert!(!safe_family(&"X".repeat(129)));
        assert!(safe_family("Noto Sans CJK JP"));
        let mut state = State { results: (0..30).map(|n| format!("Font {n}")).collect(),
            ..default() };
        move_selection(&mut state, 200);
        assert_eq!(state.selected, 29);
        assert_eq!(start_index(&state), 22);
        move_selection(&mut state, -200);
        assert_eq!(state.selected, 0);
        assert_eq!(start_index(&state), 0);
    }
}
