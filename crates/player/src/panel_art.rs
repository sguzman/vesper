//! Optional image-backed dialogue art, including stretch-free end ornaments.
//! Images remain external to the engine and are loaded with the asset server.
use super::*;
use bevy::ui::widget::NodeImageMode;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoryPanelArt {
    Full,
    Left,
    Middle,
    Right,
    Cell(usize, usize),
}

/// Per-scene panel choice overrides the authored theme default.
pub fn active_skin(session: &Session) -> Option<&vesper_game::PanelSkin> {
    let id = session.presentation.panel_skin.as_deref()
        .or(session.game.theme.panel.skin.as_deref())?;
    session.game.manifest.panel_skins.get(id)
}

/// Decorative writing insets stay inside the viewport. High-contrast mode
/// discards both illustration and its reserved space.
/// A transparent game-panel backdrop must not make the illustrated skin
/// invisible. The reader opacity slider still explicitly controls the art.
pub fn default_art_opacity(session: &Session) -> f32 {
    if active_skin(session).is_some_and(|skin| skin.transparent_background) {
        1.0
    } else {
        session.game.theme.panel.background[3]
    }
}

pub fn skin_padding(session: &Session, viewport: Vec2) -> Option<UiRect> {
    if session.preferences.high_contrast.unwrap_or(false) { return None; }
    let skin = active_skin(session)?;
    let compact = (viewport.x / 1280.0).min(viewport.y / 720.0).clamp(0.6, 1.0);
    let base = session.game.theme.panel.padding * compact;
    if let Some(area) = skin.safe_area {
        // Source painting declares the exact safe writing region as proportions
        // of the *actual panel*, so changing reader width cannot move text onto
        // the compass, ivy or ornamental corners.
        let width = super::panel_width(
            viewport.x, &session.game.theme.panel, session.preferences.panel_width_percent,
        );
        let height = super::illustrated_panel_metrics(
            viewport, &session.game.theme.panel, Some(skin),
            session.preferences.panel_width_percent,
        ).min_height;
        return Some(UiRect {
            left: px((width * area[0]).ceil()),
            right: px((width * area[1]).ceil()),
            top: px((height * area[2]).ceil()),
            bottom: px((height * area[3]).ceil()),
        });
    }
    let i = skin.insets;
    Some(UiRect {
        left: px((base + i[0] * compact).min(viewport.x * 0.2)),
        right: px((base + i[1] * compact).min(viewport.x * 0.2)),
        // Ornamental top bars lose proportionally more content clearance
        // on compact screens. An opt-in authored boost keeps glyphs below
        // the paper border rather than touching the dark gold trim.
        top: px((base + i[2] * compact
            + skin.compact_top_boost * (1.0 - compact)).min(viewport.y * 0.2)),
        bottom: px((base + i[3] * compact).min(viewport.y * 0.2)),
    })
}

/// Source pixels for the three images, and destination logical pixel widths
/// for the two untouched end caps. The middle crop alone may stretch.
fn horizontal_geometry(
    spec: &vesper_game::PanelHorizontalSlices,
    size: Vec2,
) -> (f32, f32, [Rect; 3]) {
    let [source_width, source_height] = spec.image_size;
    let left_source = spec.stretch_start;
    let right_source = source_width - spec.stretch_end;
    // Never allow the ornament caps to consume all available width.
    let scale = (size.y / source_height)
        .min(size.x * 0.95 / (left_source + right_source))
        .max(0.0);
    let left = left_source * scale;
    let right = right_source * scale;
    (left, right, [
        Rect::new(0.0, 0.0, spec.stretch_start, source_height),
        Rect::new(spec.stretch_start, 0.0, spec.stretch_end, source_height),
        Rect::new(spec.stretch_end, 0.0, source_width, source_height),
    ])
}

/// The painted speaker plate belongs to the aspect-preserved left endcap.
/// Using the full panel width instead makes the name drift as paper stretches.
pub fn speaker_anchor_width(skin: Option<&vesper_game::PanelSkin>, size: Vec2) -> f32 {
    skin.and_then(|skin| skin.horizontal_slices.as_ref())
        .map_or(size.x, |spec| {
            if spec.vertical_slices.is_some() {
                spec.stretch_start * grid_scale(spec, size)
            } else {
                horizontal_geometry(spec, size).0
            }
        })
}

/// Optional nine-region geometry: decorations retain their original aspect
/// ratio regardless of the extra paper added by longer prose or window width.
fn grid_scale(spec: &vesper_game::PanelHorizontalSlices, size: Vec2) -> f32 {
    let Some([top, bottom]) = spec.vertical_slices else { return 0.0; };
    let [width, height] = spec.image_size;
    let cap_width = spec.stretch_start + width - spec.stretch_end;
    let cap_height = top + height - bottom;
    (spec.reference_height.unwrap_or(height) / height)
        .min(size.x * 0.96 / cap_width)
        .min(size.y * 0.99 / cap_height)
        .max(0.0)
}

pub fn sync(
    session: Res<Session>,
    assets: Res<AssetServer>,
    panel: Query<&ComputedNode, With<StoryPanel>>,
    mut artwork: Query<(&StoryPanelArt, &mut ImageNode, &mut Node)>,
    mut previous: Local<Option<(String, bool, u32, u32, u32, u32)>>,
) {
    let skin = active_skin(&session);
    let high_contrast = session.preferences.high_contrast.unwrap_or(false);
    let opacity = session.preferences.panel_opacity
        .unwrap_or_else(|| default_art_opacity(&session));
    let key = skin.map_or_else(String::new, |skin| skin.image.clone());
    let slices = skin.and_then(|skin| skin.horizontal_slices.as_ref());
    // ComputedNode is in physical pixels. Convert to UI logical pixels so
    // one source pixel has the same height/width scale on HiDPI monitors.
    let size = panel.single()
        .map(|computed| computed.size() * computed.inverse_scale_factor)
        .unwrap_or(Vec2::new(1178.0, 320.0));
    let cache_key = (
        key.clone(), high_contrast, opacity.to_bits(),
        skin.map_or(0.0, |skin| skin.slice_border).to_bits(),
        size.x.to_bits(), size.y.to_bits(),
    );
    if previous.as_ref() == Some(&cache_key) && !session.is_changed() { return; }
    let geometry = slices.map(|spec| horizontal_geometry(spec, size));
    let grid = slices.filter(|spec| spec.vertical_slices.is_some());

    for (part, mut image, mut node) in &mut artwork {
        let Some(skin) = skin.filter(|_| !high_contrast) else {
            node.display = Display::None;
            continue;
        };
        let enabled = if grid.is_some() {
            matches!(part, StoryPanelArt::Cell(_, _))
        } else if geometry.is_some() {
            matches!(part, StoryPanelArt::Left | StoryPanelArt::Middle | StoryPanelArt::Right)
        } else {
            *part == StoryPanelArt::Full
        };
        if !enabled {
            node.display = Display::None;
            continue;
        }
        let handle = assets.load(skin.image.clone());
        image.image = handle;
        image.color = Color::srgba(1.0, 1.0, 1.0, opacity);
        node.display = Display::Flex;
        if let Some(spec) = grid {
            let [sw, sh] = spec.image_size;
            let [top, bottom] = spec.vertical_slices.expect("grid has vertical slices");
            let scale = grid_scale(spec, size);
            let xs = [0.0, spec.stretch_start, spec.stretch_end, sw];
            let ys = [0.0, top, bottom, sh];
            let widths = [spec.stretch_start * scale, 0.0, (sw - spec.stretch_end) * scale];
            let heights = [top * scale, 0.0, (sh - bottom) * scale];
            let StoryPanelArt::Cell(column, row) = *part else { unreachable!() };
            image.rect = Some(Rect::new(xs[column], ys[row], xs[column+1], ys[row+1]));
            image.image_mode = NodeImageMode::Stretch;
            // Infill underlaps edge ornaments; corners are painted last.
            let seam_x = 2.0_f32.min((size.x - widths[0] - widths[2]).max(0.0) * 0.2);
            let seam_y = 2.0_f32.min((size.y - heights[0] - heights[2]).max(0.0) * 0.2);
            node.left = if column == 0 { px(0.0) } else if column == 1 { px(widths[0] - seam_x) } else { Val::Auto };
            node.right = if column == 2 { px(0.0) } else if column == 1 { px(widths[2] - seam_x) } else { Val::Auto };
            node.top = if row == 0 { px(0.0) } else if row == 1 { px(heights[0] - seam_y) } else { Val::Auto };
            node.bottom = if row == 2 { px(0.0) } else if row == 1 { px(heights[2] - seam_y) } else { Val::Auto };
            node.width = if column == 1 { Val::Auto } else { px(widths[column]) };
            node.height = if row == 1 { Val::Auto } else { px(heights[row]) };
        } else if let Some((left, right, rects)) = geometry {
            // Avoid a 1px black line from bilinear sampling the transparent
            // edges of adjacent cropped quads at fractional display scales.
            // The plain middle paper underlaps the two opaque decorative caps.
            let seam = 2.0_f32.min((size.x - left - right).max(0.0) * 0.2);
            let (rect, left_edge, right_edge, width) = match part {
                StoryPanelArt::Left => (rects[0], px(0.0), Val::Auto, px(left)),
                StoryPanelArt::Middle => (rects[1], px(left - seam), px(right - seam), Val::Auto),
                StoryPanelArt::Right => (rects[2], Val::Auto, px(0.0), px(right)),
                StoryPanelArt::Full | StoryPanelArt::Cell(_, _) => unreachable!(),
            };
            image.rect = Some(rect);
            image.image_mode = NodeImageMode::Stretch;
            node.left = left_edge;
            node.right = right_edge;
            node.width = width;
            node.height = percent(100.0);
            node.top = px(0.0);
            node.bottom = px(0.0);
        } else {
            image.rect = None;
            image.image_mode = NodeImageMode::Sliced(TextureSlicer {
                border: BorderRect::all(skin.slice_border),
                ..default()
            });
            node.left = px(0.0);
            node.right = px(0.0);
            node.width = percent(100.0);
            node.height = percent(100.0);
            node.top = px(0.0);
            node.bottom = px(0.0);
        }
    }
    *previous = Some(cache_key);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vertical_paper_growth_leaves_ornamental_cap_sizes_unchanged() {
        let spec = vesper_game::PanelHorizontalSlices {
            image_size: [1914.0, 820.0],
            stretch_start: 1065.0, stretch_end: 1095.0,
            vertical_slices: Some([395.0, 420.0]),
            reference_height: Some(320.0),
        };
        let standard = grid_scale(&spec, Vec2::new(1180.0, 320.0));
        let expanded = grid_scale(&spec, Vec2::new(1180.0, 400.0));
        assert!((standard - expanded).abs() < 0.0001);
        let source_height = spec.image_size[1];
        let [top, bottom] = spec.vertical_slices.unwrap();
        let extra_paper = (400.0 - (top + source_height - bottom) * expanded)
            - (320.0 - (top + source_height - bottom) * standard);
        assert!((extra_paper - 80.0).abs() < 0.01);
    }

    #[test]
    fn nameplate_tracks_fixed_illustrated_cap_across_reader_widths() {
        let skin = vesper_game::PanelSkin {
            horizontal_slices: Some(vesper_game::PanelHorizontalSlices {
                image_size: [1914.0, 820.0],
                stretch_start: 1065.0, stretch_end: 1095.0,
                vertical_slices: None, reference_height: None,
            }),
            ..Default::default()
        };
        for size in [Vec2::new(950.0, 430.0), Vec2::new(1180.0, 430.0)] {
            let cap = speaker_anchor_width(Some(&skin), size);
            assert!(cap > 0.0 && cap < size.x);
            assert!(cap * (0.76 + 0.22) < cap);
        }
        assert_eq!(speaker_anchor_width(None, Vec2::new(900.0, 400.0)), 900.0);
    }

    #[test]
    fn atlas_two_axis_caps_are_uniformly_scaled_and_anchor_follows_them() {
        let spec = vesper_game::PanelHorizontalSlices {
            image_size: [1914.0, 820.0],
            stretch_start: 1240.0, stretch_end: 1244.0,
            vertical_slices: Some([395.0, 420.0]),
            reference_height: Some(430.0),
        };
        let skin = vesper_game::PanelSkin {
            horizontal_slices: Some(spec.clone()), ..Default::default()
        };
        for size in [Vec2::new(770.0, 323.0), Vec2::new(810.0, 177.0),
                     Vec2::new(946.0, 306.0), Vec2::new(1180.0, 430.0)] {
            let scale = grid_scale(&spec, size);
            let x_caps = (spec.stretch_start + spec.image_size[0] - spec.stretch_end) * scale;
            let y_caps = (395.0 + 820.0 - 420.0) * scale;
            assert!(x_caps < size.x && y_caps < size.y);
            assert!((speaker_anchor_width(Some(&skin), size) -
                spec.stretch_start * scale).abs() < 0.001);
            assert!((spec.stretch_start * scale / spec.stretch_start -
                (spec.image_size[0] - spec.stretch_end) * scale /
                (spec.image_size[0] - spec.stretch_end)).abs() < 0.0001);
        }
    }

    #[test]
    fn wide_and_narrow_panels_keep_illustrated_caps_proportional() {
        let spec = vesper_game::PanelHorizontalSlices {
            image_size: [1914.0, 820.0],
            stretch_start: 1065.0,
            stretch_end: 1095.0,
            vertical_slices: None,
            reference_height: None,
        };
        for size in [Vec2::new(1178.0, 320.0), Vec2::new(1764.0, 325.0),
                     Vec2::new(946.0, 245.0), Vec2::new(1900.0, 360.0)] {
            let (left, right, rects) = horizontal_geometry(&spec, size);
            assert!(left > 0.0 && right > 0.0 && size.x - left - right > 0.0);
            assert!((left / spec.stretch_start -
                right / (spec.image_size[0] - spec.stretch_end)).abs() < 0.0001);
            assert_eq!(rects[1].width(), 30.0);
        }
    }
}
