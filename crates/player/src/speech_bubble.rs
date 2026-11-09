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
#[derive(Component)]
pub struct BubbleScrollCue;

#[derive(Clone, Copy, Debug)]
struct BubbleGeometry {
    left: f32,
    top: f32,
    width: f32,
    /// Maximum on-screen bubble content height; longer passages scroll.
    max_height: f32,
    /// Horizontal position of the bubble-tail center within the bubble.
    tail_x: Option<f32>,
}

/// Pure geometry, in logical Bevy UI pixels. The viewport is already divided
/// by UiScale, so physical HiDPI/windowed displays are treated consistently.
fn bubble_geometry(
    viewport: Vec2, width_percent: f32, anchor: Option<Vec2>,
) -> BubbleGeometry {
    let viewport = viewport.max(Vec2::splat(1.0));
    // A clipped/offscreen staged character or a world NPC outside the
    // current camera is not a valid bubble anchor. Prefer the same centered
    // readable fallback used by narration, never a tail toward empty space.
    // This check is in logical UI coordinates for both portrait and world NPCs.
    let safe_x = 12.0_f32.min(viewport.x * 0.05);
    let safe_y = 12.0_f32.min(viewport.y * 0.05);
    // The bubble needs at least 90 logical pixels of text area above
    // the head plus its 14px tail gap. A speaker at the top edge can be
    // onscreen yet have no such headroom: use the narration fallback
    // rather than covering their face with a forced minimum-height panel.
    let min_head_y = (8.0 + 90.0 + 14.0_f32).max(safe_y);
    let visible_head = anchor.filter(|head| head.is_finite()
        && head.x >= safe_x && head.x <= viewport.x - safe_x
        && head.y >= safe_y && head.y <= viewport.y - safe_y);
    let low_head = visible_head.filter(|head| head.y < min_head_y);
    // Above-head anchoring needs enough room for ordinary speaker/nameplate
    // layout, not merely the 90px minimum scroll viewport. A high staged
    // portrait can have a visible head but only 118px above it, which would
    // incorrectly force PgDn even for a single short sentence.
    let comfortable_head_y = min_head_y.max(8.0 + 150.0 + 14.0);
    let side_head = visible_head.filter(|head|
        head.y >= min_head_y && head.y < comfortable_head_y);
    let anchor = visible_head.filter(|head| head.y >= comfortable_head_y);
    let gutter = safe_x;
    let available = (viewport.x - gutter * 2.0).max(1.0);
    let authored_width = (viewport.x * width_percent / 100.0).min(available);
    // For a high portrait with inadequate headroom, use the wider available
    // side instead of scrolling a one-line bubble or covering the face.
    // Side placement deliberately has no downward-pointing tail.
    let (width, left) = if let Some(head) = side_head {
        let face_gap = 76.0_f32.min(viewport.x * 0.15);
        let left_room = (head.x - face_gap - gutter).max(1.0);
        let right_room = (viewport.x - head.x - face_gap - gutter).max(1.0);
        let right = right_room >= left_room;
        let width = authored_width.min(if right { right_room } else { left_room });
        let left = if right { head.x + face_gap } else { head.x - face_gap - width };
        (width, left.clamp(gutter, (viewport.x - width - gutter).max(gutter)))
    } else {
        let left = visible_head.map_or((viewport.x - authored_width) * 0.5, |head| {
            (head.x - authored_width * 0.5)
                .clamp(gutter, (viewport.x - authored_width - gutter).max(gutter))
        });
        (authored_width, left)
    };
    // With inadequate headroom, a tail-free bubble goes *below* the
    // visible speaker. Centering the fallback near the top would otherwise
    // cover the very face this geometry is supposed to protect.
    let top = if let Some(head) = low_head {
        head.y + 18.0
    } else if side_head.is_some() {
        viewport.y * 0.09
    } else {
        anchor.map_or(viewport.y * 0.09, |head| {
            (head.y - 172.0).clamp(8.0, (viewport.y * 0.50).max(8.0))
        })
    };
    // Long passages are scrollable, not full-screen veils. Limit the panel
    // to the upper 43% of the scene, leaving the central character's face
    // and the exploration focus area visible. Anchored dialogue additionally
    // stops above the speaker's head. Never exceed the physical viewport.
    let viewport_cap = (viewport.y - top - 12.0).max(90.0);
    let face_safe_bottom = viewport.y * 0.43;
    // Narration and top-edge speaker fallbacks use the upper safe band;
    // an above-head bubble instead follows the speaker, stopping above
    // their face so the tail connects visually. The top-edge fallback
    // has no tail and begins below the head, preserving their face.
    let max_bottom = anchor.map_or(face_safe_bottom, |head| head.y - 14.0);
    let max_height = (max_bottom - top).clamp(90.0, viewport_cap);
    BubbleGeometry {
        left, top, width, max_height,
        tail_x: anchor.map(|head| {
            (head.x - left).clamp(22.0, (width - 22.0).max(22.0))
        }),
    }
}

/// Return an NPC only if one authored interaction owns this scene.
fn unique_modal_npc<'a>(
    npcs: &'a std::collections::BTreeMap<String, vesper_game::exploration::Npc>,
    scene: &str,
) -> Option<&'a vesper_game::exploration::Npc> {
    let mut candidates = npcs.values().filter(|npc| npc.interact_scene == scene);
    let only = candidates.next()?;
    candidates.next().is_none().then_some(only)
}

/// Project an authored world NPC to logical UI coordinates. This same
/// projected head works for both speaker lines and contextual narration.
fn world_npc_head(
    npc: &vesper_game::exploration::Npc,
    area: &vesper_game::exploration::Location,
    world_x: f32,
    viewport: Vec2,
    scale: f32,
) -> Vec2 {
    let view = vesper_game::walk::View::for_area(
        area.width, area.height, viewport.x, viewport.y,
        area.camera.zoom, world_x, area.camera.follow_x,
    );
    let ground_y = (0.25 - area.camera.horizon) * viewport.y;
    Vec2::new(
        viewport.x * 0.5 + view.x(npc.x),
        // The renderer places the sprite at ground + authored elevation,
        // then scales its 85px nominal height by the NPC's own scale.
        // Anchor the bubble above the *visible* head, not the old ground.
        viewport.y * 0.5 - ground_y
            - (npc.elevation + 42.5 * npc.scale) * view.world_to_screen,
    ) / scale.max(0.05)
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
        IgnoreScroll(BVec2::TRUE),
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
        Name::new("Long bubble scroll hint"),
        BubbleScrollCue,
        IgnoreScroll(BVec2::TRUE),
        OverrideClip,
        Text::new("PgDn ↓"),
        TextFont { font_size: FontSize::Px(13.0), ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            right: px(9.0), bottom: px(3.0),
            display: Display::None, ..default()
        },
    ));
    parent.spawn((
        Name::new("Native speech bubble tail"),
        BubbleTail,
        IgnoreScroll(BVec2::TRUE),
        OverrideClip,
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
        &mut Node, &mut BackgroundColor, &mut BorderColor, &mut ScrollPosition,
        &ComputedNode
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
        Has<BubbleScrollCue>,
        Option<&mut BackgroundColor>,
    ), (
        Without<StoryPanel>, Without<BubbleFrame>,
        Or<(
            With<StoryText>, With<panel_art::StoryPanelArt>,
            With<StoryAccent>, With<StorySeparator>, With<NameplateAnchor>,
            With<HintAnchor>, With<BubbleTail>, With<BubbleScrollCue>,
        )>,
    )>,
    mut previous: Local<(bool, Option<String>)>,
    mut previous_line: Local<Option<(String, usize, u64)>>,
) {
    let active = active_bubble(&session);
    // With no bubble configured, legacy panels don't acquire additional
    // per-frame UI work or lose their custom artwork.
    if !active && !previous.0 { return; }
    let Ok(window) = windows.single() else { return; };
    let viewport = Vec2::new(window.resolution.width(), window.resolution.height());
    if viewport.x <= 0.0 || viewport.y <= 0.0 { return; }
    let logical = viewport / ui_scale.0.max(0.05);
    let Ok((mut node, mut fill, mut border, mut scroll, computed)) = roots.single_mut()
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
            // A modal world conversation can use the same speech-bubble
            // surface as a pure VN, anchored to its authored NPC rather
            // than requiring a staged VN character or baked text artwork.
            if position.is_none()
                && let vesper_game::experience::ExperienceMode::Conversation {
                    return_to, ..
                } = &session.experience.mode
                && let Some(area) = session.game.manifest.exploration.as_ref()
                    .and_then(|world| world.locations.get(&return_to.location))
                && let Some(npc) = area.npcs.get(name)
                && npc.visible_if.as_ref().is_none_or(|condition|
                    condition.matches(session.machine.variables()))
            {
                position = Some(world_npc_head(
                    npc, area, return_to.x, viewport, ui_scale.0,
                ));
            }
        }
        // Unnamed narration inside a modal NPC conversation must not lose
        // its visual speaker context. The original game's Mateo interaction
        // begins with a bell/narration line: a top-of-screen fallback sat
        // directly over his painted face despite a validated NPC target.
        // Derive the anchor from the *existing authored interaction scene*;
        // do not invent a speaker or change story machine/save state.
        if speaker.is_none() && position.is_none()
            && let vesper_game::experience::ExperienceMode::Conversation {
                scene, return_to
            } = &session.experience.mode
            && let Some(area) = session.game.manifest.exploration.as_ref()
                .and_then(|world| world.locations.get(&return_to.location))
            && let Some(npc) = unique_modal_npc(&area.npcs, scene)
            && npc.visible_if.as_ref().is_none_or(|condition|
                condition.matches(session.machine.variables()))
        {
            position = Some(world_npc_head(
                npc, area, return_to.x, viewport, ui_scale.0,
            ));
        }
        let geometry = bubble_geometry(logical, skin.max_width_percent, position);
        // Each distinct *authored position*, including identical consecutive
        // dialogue strings, starts at the top. An F9 load must also reset a
        // previously scrolled bubble at the same scene/cursor. Do not clone
        // the complete narrative snapshot or hash the long text each frame.
        let scene = session.machine.scene();
        let cursor = session.machine.cursor();
        let epoch = session.ambient_restore_epoch;
        if !previous_line.as_ref().is_some_and(|(old_scene, old_cursor, old_epoch)|
            old_scene == scene && *old_cursor == cursor && *old_epoch == epoch)
        {
            scroll.0 = Vec2::ZERO;
            *previous_line = Some((scene.to_owned(), cursor, epoch));
        }
        // Mut<Node> projects through DerefMut: update sequentially rather
        // than borrowing several fields of the wrapper simultaneously.
        if node.left != px(geometry.left) { node.left = px(geometry.left); }
        if node.top != px(geometry.top) { node.top = px(geometry.top); }
        if node.bottom != Val::Auto { node.bottom = Val::Auto; }
        if node.width != px(geometry.width) { node.width = px(geometry.width); }
        if node.height != Val::Auto { node.height = Val::Auto; }
        if node.max_height != px(geometry.max_height) {
            node.max_height = px(geometry.max_height);
        }
        if node.min_height != px(90.0) { node.min_height = px(90.0); }
        // Bevy's Mut<Node> marks the layout dirty on any field write.
        // A typical dialogue lasts hundreds of frames, so do not write
        // identical padding, border and scroll mode every Update.
        let next_edge = px(2.0).all();
        if node.border != next_edge { node.border = next_edge; }
        let next_padding = px(skin.padding).all();
        if node.padding != next_padding { node.padding = next_padding; }
        if node.row_gap != px(6.0) { node.row_gap = px(6.0); }
        // Scrolling prevents long dialogue or reader zoom clipping.
        let next_overflow = Overflow::scroll_y();
        if node.overflow != next_overflow { node.overflow = next_overflow; }
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
        // A short line can have a tiny layout/content discrepancy from
        // padding and decorative nodes. It is not genuinely scrollable
        // until the bubble actually reaches its viewport-height cap.
        let cap_in_layout_pixels = geometry.max_height
            / computed.inverse_scale_factor().max(0.001);
        let has_more = computed.size().y >= cap_in_layout_pixels - 4.0
            && computed.content_size().y > computed.size().y + 2.0;
        for (mut part, text, art, accent, divider, nameplate, hint_slot, tail, cue, color) in &mut details {
            if cue {
                let display = if has_more { Display::Flex } else { Display::None };
                if part.display != display {
                    part.display = display;
                    info!("Vesper bubble scroll hint {} (panel {:.0}, content {:.0}, cap {:.0})",
                        if has_more { "visible" } else { "hidden" },
                        computed.size().y, computed.content_size().y,
                        cap_in_layout_pixels);
                }
            } else if art || accent || divider || hint_slot
                || matches!(text, Some(StoryText::Hint))
            {
                if part.display != Display::None { part.display = Display::None; }
            } else if nameplate {
                if part.display != Display::Flex { part.display = Display::Flex; }
                if part.position_type != PositionType::Relative {
                    part.position_type = PositionType::Relative;
                }
                if part.left != Val::Auto { part.left = Val::Auto; }
                if part.top != Val::Auto { part.top = Val::Auto; }
                if part.width != percent(100.0) { part.width = percent(100.0); }
                if part.height != Val::Auto { part.height = Val::Auto; }
            } else if tail {
                let show = geometry.tail_x.is_some();
                let display = if show { Display::Flex } else { Display::None };
                if part.display != display { part.display = display; }
                if let Some(x) = geometry.tail_x {
                    if part.left != px(x - 8.0) { part.left = px(x - 8.0); }
                }
                if let Some(mut color) = color {
                    if color.0 != background { color.0 = background; }
                }
            }
        }
        previous.0 = true;
        return;
    }

    // Restore the legacy illustrated-panel geometry after a bubble line
    // changes into choices or an ending. The original text/choice entities
    // never moved or re-created, so reading state and focus are untouched.
    if previous.0 { scroll.0 = Vec2::ZERO; }
    *previous_line = None;
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
    for (mut part, text, art, accent, divider, nameplate, hint_slot, tail, cue, _) in &mut details {
        if tail || cue {
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


/// The scroll host is the existing StoryPanel, not a second text renderer.
/// Mouse wheel and PageUp/PageDown reach the complete content without
/// intercepting Enter/Space, reader reveal pacing or authored story choices.
pub fn scroll(
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    keyboard: Res<ButtonInput<KeyCode>>,
    session: Res<Session>,
    settings: Res<settings::SettingsState>,
    font_browser: Res<font_browser::State>,
    slots: Res<save_slots::SaveUi>,
    mut panels: Query<(&mut ScrollPosition, &ComputedNode), With<StoryPanel>>,
) {
    use bevy::input::mouse::MouseScrollUnit;
    let mut wheel_delta = 0.0;
    for event in wheel.read() {
        wheel_delta -= event.y * match event.unit {
            MouseScrollUnit::Line => 60.0,
            MouseScrollUnit::Pixel => 1.0,
        };
    }
    if !active_bubble(&session) || session.paused
        || session.menu_overlay.is_some() || session.history.visible()
        || settings.overlay.is_some() || font_browser.overlay.is_some()
        || slots.visible()
    {
        return;
    }
    let Ok((mut position, computed)) = panels.single_mut() else { return; };
    let page = (computed.size().y * 0.75).max(90.0);
    let mut requested = wheel_delta;
    if keyboard.just_pressed(KeyCode::PageDown) { requested += page; }
    if keyboard.just_pressed(KeyCode::PageUp) { requested -= page; }
    let to_top = keyboard.just_pressed(KeyCode::Home);
    let to_end = keyboard.just_pressed(KeyCode::End);
    if requested == 0.0 && !to_top && !to_end { return; }

    let max_scroll = ((computed.content_size().y - computed.size().y)
        * computed.inverse_scale_factor()).max(0.0);
    let next = if to_top { 0.0 }
        else if to_end { max_scroll }
        else { (position.y + requested).clamp(0.0, max_scroll) };
    if (next - position.y).abs() > 0.1 {
        position.y = next;
        info!("Vesper bubble scroll: offset {:.0}/{:.0}", next, max_scroll);
    }
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
                assert!(g.top + g.max_height <= viewport.y - 11.0);
                assert!(g.max_height >= 90.0);
                if let Some(tail) = g.tail_x {
                    assert!(tail >= 0.0 && tail <= g.width);
                }
            }
        }
    }

    #[test]
    fn long_dialogue_does_not_cover_the_middle_of_the_scene() {
        for viewport in [
            Vec2::new(946.0, 511.0), Vec2::new(1280.0, 720.0),
            Vec2::new(1920.0, 1080.0),
        ] {
            let narrator = bubble_geometry(viewport, 58.0, None);
            assert!(narrator.top + narrator.max_height <= viewport.y * 0.43 + 0.1);
            assert!(narrator.max_height >= 90.0);
            let speaker = Vec2::new(viewport.x / 2.0, viewport.y * 0.65);
            let anchored = bubble_geometry(viewport, 58.0, Some(speaker));
            assert!(anchored.tail_x.is_some());
            assert!(anchored.top + anchored.max_height < speaker.y - 12.0);
            assert!(anchored.max_height >= 90.0);
        }
    }

    #[test]
    fn high_staged_speakers_use_face_safe_side_bubbles_with_real_text_height() {
        for viewport in [Vec2::new(946.0, 511.0), Vec2::new(1280.0, 720.0)] {
            for x_fraction in [0.24, 0.76] {
                let head = Vec2::new(viewport.x * x_fraction, 140.0);
                let bubble = bubble_geometry(viewport, 57.0, Some(head));
                assert!(bubble.tail_x.is_none());
                assert!(bubble.width >= 300.0);
                assert!(bubble.max_height >= 140.0);
                assert!(bubble.top + bubble.max_height <= viewport.y * 0.43 + 0.1);
                if x_fraction < 0.5 {
                    assert!(bubble.left >= head.x + 70.0);
                } else {
                    assert!(bubble.left + bubble.width <= head.x - 70.0);
                }
            }
        }
    }

    #[test]
    fn offscreen_and_invalid_speakers_use_centered_tail_free_fallback() {
        for viewport in [Vec2::new(946.0, 511.0), Vec2::new(1280.0, 720.0),
                         Vec2::new(1920.0, 1080.0)] {
            let fallback = bubble_geometry(viewport, 58.0, None);
            for outside in [
                Vec2::new(-1.0, 220.0),
                Vec2::new(viewport.x + 1.0, 220.0),
                Vec2::new(320.0, -20.0),
                Vec2::new(320.0, viewport.y + 20.0),
                Vec2::new(f32::NAN, 200.0),
                Vec2::new(200.0, f32::INFINITY),
            ] {
                let g = bubble_geometry(viewport, 58.0, Some(outside));
                assert!(g.tail_x.is_none(), "unexpected tail for {outside:?}");
                assert_eq!(g.left, fallback.left);
                assert_eq!(g.top, fallback.top);
                assert_eq!(g.width, fallback.width);
                assert_eq!(g.max_height, fallback.max_height);
            }
            // A speaker visible near the top has no room for a bubble
            // above their face. The readable tail-free fallback goes
            // below their head, never covering it.
            for head_y in [60.0, 110.0] {
                let below = bubble_geometry(viewport, 58.0,
                    Some(Vec2::new(viewport.x * 0.5, head_y)));
                assert!(below.tail_x.is_none());
                assert!(below.top >= head_y + 17.0);
                assert!(below.top + below.max_height < viewport.y - 11.0);
            }
            // Fully onscreen speakers with headroom retain the tail.
            let visible = bubble_geometry(viewport, 58.0,
                Some(Vec2::new(viewport.x - 24.0, viewport.y * 0.5)));
            assert!(visible.tail_x.is_some());
            assert!(visible.left >= 0.0 && visible.left + visible.width <= viewport.x);
        }
    }

    #[test]
    fn modal_narration_uses_only_unambiguous_authored_npc() {
        use vesper_game::exploration::Npc;
        let mut npcs = std::collections::BTreeMap::new();
        let mateo = Npc {
            x: 820.0, interact_scene: "mateo_chat".into(), image: None,
            scale: 1.0, elevation: 0.0, interaction_radius: 140.0, visible_if: None,
        };
        npcs.insert("mateo".to_owned(), mateo);
        assert_eq!(unique_modal_npc(&npcs, "mateo_chat").map(|npc| npc.x),
            Some(820.0));
        assert!(unique_modal_npc(&npcs, "shop_cat").is_none());
        // Two NPCs sharing a scene are ambiguous; do not fabricate a tail.
        npcs.insert("second".to_owned(), Npc {
            x: 200.0, interact_scene: "mateo_chat".into(), image: None,
            scale: 1.0, elevation: 0.0, interaction_radius: 140.0, visible_if: None,
        });
        assert!(unique_modal_npc(&npcs, "mateo_chat").is_none());
    }

    #[test]
    fn modal_npc_bubble_head_tracks_authored_elevation_and_scale() {
        use vesper_game::exploration::{Location, Npc};
        let area = Location {
            background: "test.png".into(), width: 1280.0, height: 720.0,
            camera: Default::default(), quick_interact: None,
            spawns: Default::default(), npcs: Default::default(),
            props: Default::default(), exits: Default::default(),
            layers: Default::default(),
        };
        let mut npc = Npc {
            x: 640.0, interact_scene: "chat".into(), image: None,
            scale: 1.0, elevation: 0.0,
            interaction_radius: 95.0, visible_if: None,
        };
        let viewport = Vec2::new(1280.0, 720.0);
        let legacy = world_npc_head(&npc, &area, 640.0, viewport, 1.0);
        assert!((legacy.x - 640.0).abs() < 0.1);
        assert!((legacy.y - 497.5).abs() < 0.1);
        npc.scale = 2.0;
        npc.elevation = 180.0;
        let elevated = world_npc_head(&npc, &area, 640.0, viewport, 1.0);
        assert!((elevated.x - legacy.x).abs() < 0.1);
        assert!((legacy.y - elevated.y - 222.5).abs() < 0.1);
        let half_ui = world_npc_head(&npc, &area, 640.0, viewport, 2.0);
        assert!((half_ui.y * 2.0 - elevated.y).abs() < 0.1);

        // A compact window scales the elevation and apparent head size
        // together, without moving the authored world interaction point.
        let compact = Vec2::new(946.0, 511.0);
        let raised_compact = world_npc_head(&npc, &area, 640.0, compact, 1.0);
        npc.scale = 1.0;
        npc.elevation = 0.0;
        let default_compact = world_npc_head(&npc, &area, 640.0, compact, 1.0);
        let compact_scale = 511.0 / 720.0;
        assert!((raised_compact.x - 473.0).abs() < 0.1);
        assert!((default_compact.x - raised_compact.x).abs() < 0.1);
        assert!((default_compact.y - raised_compact.y
            - 222.5 * compact_scale).abs() < 0.1);
    }

    #[test]
    fn no_character_anchors_to_readable_center_without_fabricating_world_state() {
        let g = bubble_geometry(Vec2::new(1280.0, 720.0), 58.0, None);
        assert!((g.left + g.width * 0.5 - 640.0).abs() < 0.1);
        assert!(g.tail_x.is_none());
    }
}
