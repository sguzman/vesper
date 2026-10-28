//! Optional shader-free atmospheric layers. All entities are bounded and
//! rendered from procedural solid-color sprites; no runtime file I/O,
//! image generation, font lookup or shader compilation is performed here.
use super::*;
use vesper_game::{AtmospherePreset, AtmosphereTheme};

#[derive(Clone, Copy, Debug)]
enum Part {
    Mote(u8),
    Glow(u8),
    Edge { side: u8, layer: u8 },
}

#[derive(Component)]
pub struct AtmospherePart(Part);

#[derive(Resource, Default)]
pub struct AtmosphereState {
    authored: Option<AtmosphereTheme>,
    elapsed: f32,
    entities: Vec<Entity>,
}

/// Maximum across presets: 32 motes or 16 motes + 3 glow + 12 edge strips.
pub const MAX_ENTITIES: usize = 32;

fn hash(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

fn unit(seed: u64) -> f32 {
    ((hash(seed) >> 40) as u32 as f32) / 16_777_215.0
}

fn mote_position(seed: u64, index: u8, size: Vec2, travel: f32) -> Vec2 {
    let salt = (index as u64).wrapping_mul(0x9e3779b97f4a7c15);
    let x = unit(seed ^ salt) * size.x - size.x * 0.5;
    let y = (unit(seed ^ salt.rotate_left(21)) * size.y + travel)
        .rem_euclid(size.y.max(1.0)) - size.y * 0.5;
    Vec2::new(x, y)
}

fn parts(theme: &AtmosphereTheme) -> Vec<Part> {
    let mut list = Vec::with_capacity(MAX_ENTITIES);
    let motes = match theme.preset {
        AtmospherePreset::Motes => theme.count,
        AtmospherePreset::Moonlit => theme.count.min(16),
        _ => 0,
    };
    for i in 0..motes { list.push(Part::Mote(i)); }
    if matches!(theme.preset, AtmospherePreset::Glow | AtmospherePreset::Moonlit) {
        for layer in 0..3 { list.push(Part::Glow(layer)); }
    }
    if matches!(theme.preset, AtmospherePreset::Vignette | AtmospherePreset::Moonlit) {
        for side in 0..4 {
            for layer in 0..3 { list.push(Part::Edge {side, layer}); }
        }
    }
    debug_assert!(list.len() <= MAX_ENTITIES);
    list
}

/// Geometry is recomputed only from a tiny fixed set of deterministic values.
/// It remains responsive under window resize and preserves aspect while motes
/// wrap inside the camera frame.
fn appearance(
    part: Part,
    config: &AtmosphereTheme,
    size: Vec2,
    elapsed: f32,
    reduced: bool,
) -> (Transform, Vec2, Color) {
    let size = size.max(Vec2::ONE);
    let opacity = config.opacity * config.color[3];
    let tint = config.color;
    match part {
        Part::Mote(i) => {
            let pos = mote_position(config.seed, i, size, elapsed * config.speed);
            let diameter = config.radius * 2.0;
            let alpha = if reduced { 0.0 } else { opacity };
            (Transform::from_xyz(pos.x, pos.y, -5.8),
                Vec2::splat(diameter),
                Color::srgba(tint[0], tint[1], tint[2], alpha))
        }
        Part::Glow(layer) => {
            // Softly nested, very low-opacity solid-color sprites avoid both
            // expensive shader materials and hard image dependencies.
            let factor = [0.78, 0.60, 0.43][layer as usize];
            let alpha = opacity * [0.06, 0.10, 0.12][layer as usize];
            (Transform::from_xyz(0.0, size.y * 0.07, -6.0 + layer as f32 * 0.01),
                size * factor,
                Color::srgba(tint[0], tint[1], tint[2], alpha))
        }
        Part::Edge {side, layer} => {
            // Three restrained bands per edge form a shader-free vignette.
            let layer_factor = layer as f32;
            let width = (size.x * 0.045).max(3.0);
            let height = (size.y * 0.045).max(3.0);
            let z = -5.9 + layer_factor * 0.01;
            let alpha = opacity * [0.28, 0.19, 0.09][layer as usize];
            let (x, y, shape) = match side {
                0 => (-size.x * 0.5 + width * (layer_factor + 0.5), 0.0,
                    Vec2::new(width + 1.0, size.y)),
                1 => (size.x * 0.5 - width * (layer_factor + 0.5), 0.0,
                    Vec2::new(width + 1.0, size.y)),
                2 => (0.0, size.y * 0.5 - height * (layer_factor + 0.5),
                    Vec2::new(size.x, height + 1.0)),
                _ => (0.0, -size.y * 0.5 + height * (layer_factor + 0.5),
                    Vec2::new(size.x, height + 1.0)),
            };
            (Transform::from_xyz(x, y, z), shape,
                Color::srgba(0.0, 0.0, 0.0, alpha))
        }
    }
}

pub fn tick(
    time: Res<Time>,
    session: Res<Session>,
    settings: Res<settings::SettingsState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<AtmosphereState>,
    mut commands: Commands,
    mut sprites: Query<(&AtmospherePart, &mut Transform, &mut Sprite)>,
) {
    let Ok(window) = windows.single() else { return; };
    let size = Vec2::new(window.resolution.width(), window.resolution.height());
    if size.x <= 0.0 || size.y <= 0.0 { return; }
    let authored = &session.game.theme.atmosphere;
    if state.authored.as_ref() != Some(authored) {
        for entity in state.entities.drain(..) {
            commands.entity(entity).despawn();
        }
        state.elapsed = 0.0;
        let description = parts(authored);
        for part in description {
            let entity = commands.spawn((
                Name::new(format!("Atmosphere: {part:?}")),
                AtmospherePart(part),
                Sprite::from_color(Color::WHITE, Vec2::ONE),
                Transform::from_xyz(0.0, 0.0, -6.0),
            )).id();
            state.entities.push(entity);
        }
        state.authored = Some(authored.clone());
        info!("Vesper atmosphere configured: {:?}, {} entities",
            authored.preset, state.entities.len());
    }
    let active = !session.paused && !session.finished
        && session.menu_overlay.is_none() && settings.overlay.is_none();
    let reduced = session.preferences.reduced_motion.unwrap_or(false);
    if active && !reduced {
        state.elapsed = (state.elapsed + time.delta_secs().clamp(0.0, 0.1))
            .rem_euclid(10000.0);
    }
    for (part, mut transform, mut sprite) in &mut sprites {
        let (placement, size, tint) =
            appearance(part.0, authored, size, state.elapsed, reduced);
        if transform.translation != placement.translation {
            transform.translation = placement.translation;
        }
        if sprite.custom_size != Some(size) { sprite.custom_size = Some(size); }
        if sprite.color != tint { sprite.color = tint; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_never_exceed_entity_cap_and_off_is_empty() {
        let mut theme = AtmosphereTheme::default();
        assert!(parts(&theme).is_empty());
        theme.count = 32;
        for preset in [AtmospherePreset::Motes, AtmospherePreset::Glow,
            AtmospherePreset::Vignette, AtmospherePreset::Moonlit] {
            theme.preset = preset;
            assert!(parts(&theme).len() <= MAX_ENTITIES);
        }
    }

    #[test]
    fn seeded_motes_are_deterministic_and_wrap_in_bounds() {
        let size = Vec2::new(946.0, 511.0);
        for i in 0..32 {
            let a = mote_position(882, i, size, 45.0);
            let b = mote_position(882, i, size, 45.0);
            assert_eq!(a, b);
            assert!(a.x >= -size.x * 0.5 && a.x <= size.x * 0.5);
            assert!(a.y >= -size.y * 0.5 && a.y <= size.y * 0.5);
        }
    }

    #[test]
    fn reduced_motion_hides_motes_but_keeps_static_vignette() {
        let theme = AtmosphereTheme {
            preset: AtmospherePreset::Motes, opacity: 0.25, ..default()
        };
        let (_, _, animated) =
            appearance(Part::Mote(0), &theme, Vec2::new(1280.,720.), 2.0, false);
        let (_, _, reduced) =
            appearance(Part::Mote(0), &theme, Vec2::new(1280.,720.), 2.0, true);
        assert!(animated.alpha() > 0.0);
        assert_eq!(reduced.alpha(), 0.0);
        let (_, _, vignette) =
            appearance(Part::Edge {side: 0, layer: 0}, &theme,
                Vec2::new(1280.,720.), 2.0, true);
        assert!(vignette.alpha() > 0.0);
    }
}
