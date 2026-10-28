//! Engine-owned choice rows. Narrative option labels and destinations remain
//! externally authored; only rendering, focus and pointer activation live here.
use super::*;
use vesper_story::{Choice, Output};

#[derive(Component)]
pub struct ChoiceList;

#[derive(Component, Debug, Clone, Copy)]
pub struct ChoiceRow(pub usize);

#[derive(Resource, Default)]
pub struct ChoiceUi {
    pub container: Option<Entity>,
    labels: Vec<String>,
    rows: Vec<Entity>,
}

#[derive(Resource, Default)]
pub struct ChoiceClick(pub Option<usize>);

/// A bounded scroll viewport prevents up to nine (potentially multi-line)
/// options from extending beyond compact Linux/Wayland windows.
pub fn choice_viewport_height(window_height: f32) -> f32 {
    (window_height * 0.43).clamp(120.0, 480.0)
}

pub fn spawn_container(parent: &mut ChildSpawnerCommands) -> Entity {
    parent.spawn((
        Name::new("Story choices — engine-owned list"),
        ChoiceList,
        Node {
            display: Display::None,
            width: percent(100.0),
            max_height: px(choice_viewport_height(720.0)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: px(4.0),
            overflow: Overflow::scroll_y(),
            flex_shrink: 0.0,
            ..default()
        },
        ScrollPosition::default(),
    )).id()
}

fn active_options(session: &Session) -> Option<&[Choice]> {
    if !session.choosing || session.finished { return None; }
    match session.machine.last_output() {
        Some(Output::Choices(options)) if !options.is_empty() => Some(options.as_slice()),
        _ => None,
    }
}

fn choice_colors(theme: &vesper_game::Theme, selected: bool) -> (Color, TextColor) {
    if selected {
        let accent = theme.panel.accent;
        (
            Color::srgba(accent[0] * 0.28, accent[1] * 0.28, accent[2] * 0.28, 0.94),
            TextColor(Color::srgba(1.0, 0.98, 0.91, 1.0)),
        )
    } else {
        (Color::NONE, text_color(theme.choices.color))
    }
}

/// Only rebuild when authored option labels change. Focus movement merely
/// recolors existing rows; no entity churn or string regeneration per keypress.
pub fn sync_rows(
    session: Res<Session>,
    mut ui: ResMut<ChoiceUi>,
    assets: Res<AssetServer>,
    mut commands: Commands,
    mut lists: Query<&mut Node, With<ChoiceList>>,
) {
    if !session.is_changed() { return; }
    let Some(parent) = ui.container else { return; };
    let Ok(mut list) = lists.get_mut(parent) else { return; };
    let Some(options) = active_options(&session) else {
        if list.display != Display::None { list.display = Display::None; }
        return;
    };
    if list.display != Display::Flex { list.display = Display::Flex; }
    let same_labels = options.len() == ui.labels.len()
        && options.iter().zip(&ui.labels).all(|(option, old)| &option.label == old);
    if same_labels { return; }

    for old in ui.rows.drain(..) {
        commands.entity(old).despawn();
    }
    ui.labels = options.iter().map(|option| option.label.clone()).collect();
    let theme = effective_theme(&session);
    let font = text_font(&theme, &assets);
    let selected = session.presentation.selected_choice;
    let mut new_rows = Vec::with_capacity(options.len());
    commands.entity(parent).with_children(|children| {
        for (index, option) in options.iter().enumerate() {
            let (background, foreground) = choice_colors(&theme, selected == index);
            let row = children.spawn((
                Name::new(format!("Story choice {}", index + 1)),
                ChoiceRow(index),
                Button,
                Hovered::default(),
                observe(move |_activate: On<Activate>, mut clicks: ResMut<ChoiceClick>| {
                    clicks.0 = Some(index);
                }),
                Text::new(format!("{}. {}", index + 1, option.label)),
                TextFont {
                    font: font.clone(), font_size: FontSize::Px(theme.choices.size),
                    ..default()
                },
                foreground,
                TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                Node {
                    width: percent(100.0), flex_shrink: 0.0,
                    padding: UiRect::axes(px(7.0), px(5.0)),
                    ..default()
                },
                BackgroundColor(background),
            )).id();
            new_rows.push(row);
        }
    });
    ui.rows = new_rows;
    info!("Vesper choice UI mounted {} independent rows", options.len());
}

/// Style changes and keyboard selection update the existing rows in-place.
pub fn sync_row_style(
    session: Res<Session>,
    assets: Res<AssetServer>,
    mut rows: Query<(&ChoiceRow, &mut BackgroundColor, &mut TextColor, &mut TextFont)>,
) {
    if !session.is_changed() { return; }
    let Some(_options) = active_options(&session) else { return; };
    let theme = effective_theme(&session);
    let font = text_font(&theme, &assets);
    for (index, mut background, mut foreground, mut text_font) in &mut rows {
        let (bg, fg) = choice_colors(&theme, index.0 == session.presentation.selected_choice);
        if background.0 != bg { background.0 = bg; }
        if *foreground != fg { *foreground = fg; }
        if text_font.font_size != FontSize::Px(theme.choices.size) {
            text_font.font_size = FontSize::Px(theme.choices.size);
        }
        text_font.font = font.clone();
    }
}

/// Reserve the actual shaped text height, not an estimated row count.
pub fn reserve_row_height(
    mut rows: Query<(&TextLayoutInfo, &mut Node), With<ChoiceRow>>,
) {
    for (layout, mut node) in &mut rows {
        let Some(required) = text_height_reservation(layout.size.y) else { continue; };
        let required = required + 10.0;
        if node.min_height != px(required) { node.min_height = px(required); }
    }
}

pub fn sync_viewport(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut list: Query<&mut Node, With<ChoiceList>>,
    mut previous: Local<f32>,
) {
    let Ok(window) = windows.single() else { return; };
    let height = choice_viewport_height(window.resolution.height());
    if (*previous - height).abs() < 0.01 { return; }
    for mut node in &mut list {
        node.max_height = px(height);
    }
    *previous = height;
}

/// Keep the focused row visible when a long choice list is scrolled.
pub fn scroll_selected(
    session: Res<Session>,
    ui: Res<ChoiceUi>,
    mut containers: Query<(&ComputedNode, &mut ScrollPosition), With<ChoiceList>>,
    row_nodes: Query<&ComputedNode, With<ChoiceRow>>,
) {
    if !session.is_changed() && !ui.is_changed() { return; }
    if active_options(&session).is_none() { return; }
    let Some(container) = ui.container else { return; };
    let Ok((computed, mut scroll)) = containers.get_mut(container) else { return; };
    let scale = computed.inverse_scale_factor();
    let viewport_height = computed.size().y * scale;
    if viewport_height <= 0.0 { return; }
    let mut offset = 0.0;
    for (index, row) in ui.rows.iter().copied().enumerate() {
        let Ok(node) = row_nodes.get(row) else { return; };
        let row_height = node.size().y * scale + 4.0;
        if index == session.presentation.selected_choice {
            let bottom = offset + row_height;
            let current = scroll.0.y;
            if offset < current {
                scroll.0.y = offset;
            } else if bottom > current + viewport_height {
                scroll.0.y = (bottom - viewport_height).max(0.0);
            }
            return;
        }
        offset += row_height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn viewport_height_is_finite_and_bounded() {
        for (height, expected) in [
            (511.0, 219.73),
            (720.0, 309.6),
            (1080.0, 464.4),
            (10.0, 120.0),
            (3000.0, 480.0),
        ] {
            let computed = choice_viewport_height(height);
            assert!((computed - expected).abs() < 0.001, "{height}: {computed}");
        }
    }
}
