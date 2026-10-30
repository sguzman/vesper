//! Optional external notebook/diary/banner dialogue backdrops. Only safe,
 //! already validated game-relative images are loaded via Bevy's async assets.
use super::*;
use bevy::ui::widget::NodeImageMode;

#[derive(Component)]
pub struct StoryPanelArt;

/// The active per-scene override beats the authored theme default. A game
/// without panel skins retains the original solid-color panel and dimensions.
pub fn active_skin(session: &Session) -> Option<&vesper_game::PanelSkin> {
    let id = session.presentation.panel_skin.as_deref()
        .or(session.game.theme.panel.skin.as_deref())?;
    session.game.manifest.panel_skins.get(id)
}

/// The left/right/top/bottom writing region is additive to the ordinary
/// theme padding. Compress it at compact viewports, cap at 20% per edge, and
/// never turn a narrow panel into negative available space.
pub fn skin_padding(session: &Session, viewport: Vec2) -> Option<UiRect> {
    // High contrast intentionally hides the authored paper illustration;
    // remove its decorative writing insets at the same time. Otherwise the
    // invisible notebook spine wastes the first 160px of the solid panel.
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

pub fn sync(
    session: Res<Session>,
    assets: Res<AssetServer>,
    mut artwork: Query<(&mut ImageNode, &mut Node), With<StoryPanelArt>>,
    mut previous: Local<Option<(String, bool, u32, u32)>>,
) {
    let skin = active_skin(&session);
    let high_contrast = session.preferences.high_contrast.unwrap_or(false);
    let opacity = session.preferences.panel_opacity
        .unwrap_or(session.game.theme.panel.background[3]);
    let key = skin.map_or_else(
        || String::new(),
        |skin| skin.image.clone()
    );
    let slice = skin.map_or(0.0, |skin| skin.slice_border);
    let now = (key.clone(), high_contrast, opacity.to_bits(), slice.to_bits());
    if previous.as_ref() == Some(&now) { return; }
    for (mut image, mut node) in &mut artwork {
        if let Some(skin) = skin.filter(|_| !high_contrast) {
            // Cached handles and nine-slice drawing; no disk reads in Update.
            image.image = assets.load(skin.image.clone());
            image.image_mode = NodeImageMode::Sliced(TextureSlicer {
                border: BorderRect::all(skin.slice_border),
                ..default()
            });
            image.color = Color::srgba(1.0, 1.0, 1.0, opacity);
            node.display = Display::Flex;
        } else {
            node.display = Display::None;
        }
    }
    *previous = Some(now);
}
