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
}

/// Per-scene panel choice overrides the authored theme default.
pub fn active_skin(session: &Session) -> Option<&vesper_game::PanelSkin> {
    let id = session.presentation.panel_skin.as_deref()
        .or(session.game.theme.panel.skin.as_deref())?;
    session.game.manifest.panel_skins.get(id)
}

/// Decorative writing insets stay inside the viewport. High-contrast mode
/// discards both illustration and its reserved space.
pub fn skin_padding(session: &Session, viewport: Vec2) -> Option<UiRect> {
    if session.preferences.high_contrast.unwrap_or(false) { return None; }
    let skin = active_skin(session)?;
    let compact = (viewport.x / 1280.0).min(viewport.y / 720.0).clamp(0.6, 1.0);
    let base = session.game.theme.panel.padding * compact;
    let i = skin.insets;
    Some(UiRect {
        left: px((base + i[0] * compact).min(viewport.x * 0.2)),
        right: px((base + i[1] * compact).min(viewport.x * 0.2)),
        top: px((base + i[2] * compact).min(viewport.y * 0.2)),
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
        .unwrap_or(session.game.theme.panel.background[3]);
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
    if previous.as_ref() == Some(&cache_key) { return; }
    let geometry = slices.map(|spec| horizontal_geometry(spec, size));

    for (part, mut image, mut node) in &mut artwork {
        let Some(skin) = skin.filter(|_| !high_contrast) else {
            node.display = Display::None;
            continue;
        };
        let is_full = geometry.is_none();
        if is_full != (*part == StoryPanelArt::Full) {
            node.display = Display::None;
            continue;
        }
        let handle = assets.load(skin.image.clone());
        image.image = handle;
        image.color = Color::srgba(1.0, 1.0, 1.0, opacity);
        node.display = Display::Flex;
        if let Some((left, right, rects)) = geometry {
            let (rect, left_edge, right_edge, width) = match part {
                StoryPanelArt::Left => (rects[0], px(0.0), Val::Auto, px(left)),
                StoryPanelArt::Middle => (rects[1], px(left), px(right), Val::Auto),
                StoryPanelArt::Right => (rects[2], Val::Auto, px(0.0), px(right)),
                StoryPanelArt::Full => unreachable!(),
            };
            image.rect = Some(rect);
            image.image_mode = NodeImageMode::Stretch;
            node.left = left_edge;
            node.right = right_edge;
            node.width = width;
        } else {
            image.rect = None;
            image.image_mode = NodeImageMode::Sliced(TextureSlicer {
                border: BorderRect::all(skin.slice_border),
                ..default()
            });
            node.left = px(0.0);
            node.right = px(0.0);
            node.width = percent(100.0);
        }
    }
    *previous = Some(cache_key);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wide_and_narrow_panels_keep_illustrated_caps_proportional() {
        let spec = vesper_game::PanelHorizontalSlices {
            image_size: [1914.0, 820.0],
            stretch_start: 1065.0,
            stretch_end: 1095.0,
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
