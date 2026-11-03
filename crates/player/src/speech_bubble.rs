//! Optional speech-bubble presentation for ordinary VN dialogue.
//!
//! The bubble reuses the **existing** StoryPanel text entities and reading
//! state: reveal pacing, rich text spans, history, fonts, saves and input are
//! unchanged. We only restyle/reanchor the surface for authored bubble lines.
//! Exploration is a separate optional subsystem.
use super::*;
use vesper_game::experience::DialogueSurface;

#[derive(Component)]
pub struct BubbleFrame;
#[derive(Component)]
pub struct BubbleTail;

#[derive(Clone, Copy, Debug)]
struct BubbleGeometry {
    left: f32,
    top: f32,
    width: f32,
    /// Horizontal position of the bubble-tail center within the bubble.
    tail_x: Option<f32>,
}

/// Pure geometry, in logical Bevy UI pixels. The viewport is already divided
/// by UiScale, so physical HiDPI/windowed displays are treated consistently.
fn bubble_geometry(
    viewport: Vec2, width_percent: f32, anchor: Option<Vec2>,
) -> BubbleGeometry {
    let viewport = viewport.max(Vec2::splat(1.0));
    let gutter = 12.0_f32.min(viewport.x * 0.05);
    let available = (viewport.x - gutter * 2.0).max(1.0);
    let width = (viewport.x * width_percent / 100.0).min(available);
    let left = anchor.map_or((viewport.x - width) * 0.5, |head| {
        (head.x - width * 0.5).clamp(gutter, (viewport.x - width - gutter).max(gutter))
    });
    // A character's head can already be near the top of the viewport.
    // Clamp the bubble above / near it without pushing text offscreen.
    let top = anchor.map_or(viewport.y * 0.09, |head| {
        (head.y - 172.0).clamp(8.0, (viewport.y * 0.50).max(8.0))
    });
    BubbleGeometry {
        left, top, width,
        tail_x: anchor.map(|head| {
            (head.x - left).clamp(22.0, (width - 22.0).max(22.0))
        }),
    }
}

fn active_bubble(session: &Session) -> bool {
    !session.choosing && !session.finished
        && session.experience.dialogue_surface == DialogueSurface::Bubble
        && matches!(session.machine.last_output(),
            Some(Output::Dialogue { .. } | Output::RichDialogue { .. }))
}

/// Build only two lightweight art nodes. They are never regenerated during
/// per-line text changes or animation frames.
pub fn spawn(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Name::new("Optional native speech bubble image"),
        BubbleFrame,
        ImageNode::default(),
        Node {
            position_type: PositionType::Absolute,
            top: px(0.0), left: px(0.0),
            width: percent(100.0), height: percent(100.0),
            display: Display::None,
            ..default()
        },
    ));
    parent.spawn((
        Name::new("Native speech bubble tail"),
        BubbleTail,
        Node {
            position_type: PositionType::Absolute,
            width: px(16.0), height: px(16.0),
            bottom: px(-8.0), left: px(30.0),
            display: Display::None,
            border_radius: BorderRadius::all(px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.035, 0.065, 0.12, 0.98)),
        UiTransform {
            rotation: Rot2::degrees(45.0),
            ..default()
        },
    ));
}

/// Runs last in the reader pipeline, after viewport fitting, panel-art sync
/// and preference changes. No files are parsed; the AssetServer loads game
/// artwork once per authored image path, off the render thread.
pub fn sync(
    session: Res<Session>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    characters: Query<(&CharacterVisual, &Transform)>,
    assets: Res<AssetServer>,
    mut roots: Query<(
        &mut Node, &mut BackgroundColor, &mut BorderColor
    ), (With<StoryPanel>, Without<BubbleFrame>)>,
    mut frames: Query<(&mut Node, &mut ImageNode),
        (With<BubbleFrame>, Without<StoryPanel>)>,
    mut details: Query<(
        &mut Node,
        Option<&StoryText>,
        Has<panel_art::StoryPanelArt>,
        Has<StoryAccent>,
        Has<StorySeparator>,
        Has<NameplateAnchor>,
        Has<HintAnchor>,
        Has<BubbleTail>,
        Option<&mut BackgroundColor>,
    ), (
        Without<StoryPanel>, Without<BubbleFrame>,
        Or<(
            With<StoryText>, With<panel_art::StoryPanelArt>,
            With<StoryAccent>, With<StorySeparator>, With<NameplateAnchor>,
            With<HintAnchor>, With<BubbleTail>,
        )>,
    )>,
    mut previous: Local<(bool, Option<String>)>,
) {
    let active = active_bubble(&session);
    // With no bubble configured, legacy panels don't acquire additional
    // per-frame UI work or lose their custom artwork.
    if !active && !previous.0 { return; }
    let Ok(window) = windows.single() else { return; };
    let viewport = Vec2::new(window.resolution.width(), window.resolution.height());
    if viewport.x <= 0.0 || viewport.y <= 0.0 { return; }
    let logical = viewport / ui_scale.0.max(0.05);
    let Ok((mut node, mut fill, mut border)) = roots.single_mut()
        else { return; };
    let skin = &session.game.manifest.dialogue.bubble;
    let contrast = session.preferences.high_contrast.unwrap_or(false);
    let (background, edge) = if contrast {
        (Color::BLACK, Color::WHITE)
    } else {
        (rgba(skin.background), rgba(skin.border_color))
    };

    let mut position = None;
    if active {
        let speaker = match session.machine.last_output() {
            Some(Output::Dialogue { speaker, .. } | Output::RichDialogue { speaker, .. }) =>
                speaker.as_deref(),
            _ => None,
        };
        if let Some(name) = speaker {
            for (actor, transform) in &characters {
                if actor.character == name && session.characters
                    .get(&actor.character).is_some()
                {
                    // World sprite origin is centered. 500px source height;
                    // the authored stage scale is already in Transform.
                    let head = Vec2::new(
                        viewport.x * 0.5 + transform.translation.x,
                        viewport.y * 0.5 - transform.translation.y
                            - 250.0 * transform.scale.y,
                    ) / ui_scale.0.max(0.05);
                    position = Some(head);
                    break;
                }
            }
        }
        let geometry = bubble_geometry(logical, skin.max_width_percent, position);
        // Mut<Node> projects through DerefMut: update sequentially rather
        // than borrowing several fields of the wrapper simultaneously.
        if node.left != px(geometry.left) { node.left = px(geometry.left); }
        if node.top != px(geometry.top) { node.top = px(geometry.top); }
        if node.bottom != Val::Auto { node.bottom = Val::Auto; }
        if node.width != px(geometry.width) { node.width = px(geometry.width); }
        if node.height != Val::Auto { node.height = Val::Auto; }
        if node.max_height != Val::Auto { node.max_height = Val::Auto; }
        if node.min_height != px(90.0) { node.min_height = px(90.0); }
        node.border = px(2.0).all();
        node.padding = px(skin.padding).all();
        node.row_gap = px(6.0);
        node.overflow = Overflow::visible();
        let next_rounding = BorderRadius::all(px(skin.corner_radius));
        if node.border_radius != next_rounding { node.border_radius = next_rounding; }
        if fill.0 != background { fill.0 = background; }
        let next_border = BorderColor::all(edge);
        if *border != next_border { *border = next_border; }

        for (mut frame, mut image) in &mut frames {
            let filename = if contrast { None } else { skin.image.as_ref() };
            if previous.1.as_ref() != filename {
                if let Some(filename) = filename {
                    image.image = assets.load(filename.clone());
                    image.image_mode = bevy::ui::widget::NodeImageMode::Sliced(
                        menu_button_slicer([skin.slice_border; 4])
                    );
                }
            }
            let display = if filename.is_some() { Display::Flex } else { Display::None };
            if frame.display != display { frame.display = display; }
        }
        previous.1 = if contrast { None } else { skin.image.clone() };
        // A bubble is a dialogue surface, never an instruction banner.
        // Hidden native hints remain in the accessibility/reader model.
        for (mut part, text, art, accent, divider, nameplate, hint_slot, tail, color) in &mut details {
            if art || accent || divider || hint_slot
                || matches!(text, Some(StoryText::Hint))
            {
                if part.display != Display::None { part.display = Display::None; }
            } else if nameplate {
                part.display = Display::Flex;
                part.position_type = PositionType::Relative;
                part.left = Val::Auto;
                part.top = Val::Auto;
                part.width = percent(100.0);
                part.height = Val::Auto;
            } else if tail {
                let show = geometry.tail_x.is_some();
                part.display = if show { Display::Flex } else { Display::None };
                if let Some(x) = geometry.tail_x { part.left = px(x - 8.0); }
                if let Some(mut color) = color { color.0 = background; }
            }
        }
        previous.0 = true;
        return;
    }

    // Restore the legacy illustrated-panel geometry after a bubble line
    // changes into choices or an ending. The original text/choice entities
    // never moved or re-created, so reading state and focus are untouched.
    let theme = effective_theme(&session);
    let p = &theme.panel;
    let panel_skin = if contrast { None } else { panel_art::active_skin(&session) };
    let mut geometry = illustrated_panel_metrics(
        viewport, p, panel_skin, session.preferences.panel_width_percent,
    );
    geometry.bottom = viewport.y * session.preferences.panel_bottom_percent
        .unwrap_or(p.bottom_percent) / 100.0;
    // A bubble uses absolute top-positioning, while a traditional panel is
    // bottom-anchored. Clear the old top constraint before restoring layout:
    // leaving both top and bottom set stretches/shifts the panel after F9.
    if node.top != Val::Auto { node.top = Val::Auto; }
    apply_panel_metrics(
        &mut node, p, geometry, viewport, session.preferences.panel_width_percent,
    );
    if let Some(padding) = panel_art::skin_padding(&session, viewport) {
        node.padding = padding;
    }
    node.border = UiRect::default();
    node.border_radius = BorderRadius::all(px(0.0));
    *border = BorderColor::all(Color::NONE);
    let mut panel_color = p.background;
    if panel_skin.is_some_and(|skin| skin.transparent_background) {
        panel_color[3] = 0.0;
    }
    fill.0 = rgba(panel_color);
    for (mut frame, _) in &mut frames { frame.display = Display::None; }

    let width = panel_width(viewport.x, p, session.preferences.panel_width_percent);
    let nameplate_width = panel_art::speaker_anchor_width(
        panel_skin, Vec2::new(width, geometry.min_height),
    );
    let (name_left, name_top, name_width, name_height) = speaker_bounds(
        panel_skin.and_then(|skin| skin.speaker_area),
        nameplate_width, geometry.min_height,
    );
    for (mut part, text, art, accent, divider, nameplate, hint_slot, tail, _) in &mut details {
        if tail {
            part.display = Display::None;
        } else if art {
            // panel_art::sync owns restored illustrated-cell visibility.
        } else if accent || divider {
            part.display = if p.hint_inside { Display::None } else { Display::Flex };
        } else if nameplate {
            part.position_type = PositionType::Absolute;
            part.left = name_left;
            part.top = name_top;
            part.width = name_width;
            part.height = name_height;
            part.display = Display::Flex;
        } else if hint_slot {
            part.display = if panel_skin.and_then(|skin| skin.hint_region).is_some() {
                Display::Flex
            } else { Display::None };
        } else if matches!(text, Some(StoryText::Hint)) {
            part.display = Display::Flex;
        }
    }
    previous.0 = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bubble_geometry_stays_inside_compact_and_large_viewports() {
        for viewport in [Vec2::new(946.0, 511.0), Vec2::new(1280.0, 720.0),
                         Vec2::new(1900.0, 1035.0)] {
            for anchor in [None, Some(Vec2::new(22.0, 160.0)),
                           Some(Vec2::new(viewport.x - 22.0, 180.0))] {
                let g = bubble_geometry(viewport, 58.0, anchor);
                assert!(g.left >= 0.0 && g.left + g.width <= viewport.x);
                assert!(g.top >= 0.0 && g.top <= viewport.y * 0.5);
                if let Some(tail) = g.tail_x {
                    assert!(tail >= 0.0 && tail <= g.width);
                }
            }
        }
    }

    #[test]
    fn no_character_anchors_to_readable_center_without_fabricating_world_state() {
        let g = bubble_geometry(Vec2::new(1280.0, 720.0), 58.0, None);
        assert!((g.left + g.width * 0.5 - 640.0).abs() < 0.1);
        assert!(g.tail_x.is_none());
    }
}
