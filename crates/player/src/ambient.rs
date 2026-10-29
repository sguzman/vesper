//! Independent, optionally scene-scoped ambient scenery. QOL06 owns stable
//! spawning and placement; QOL07 supplies authored motion/flipbook updates.
//! No filesystem operations, motion path parsing or image decoding in Update.
use super::*;

#[derive(Component)]
pub struct AmbientVisual { pub id: String, pub image_path: String }

/// Normalized [0,1] stage anchors and authored depth, independent of spoken
/// character position and cinematic background-cover cropping.
pub fn placement(size: Vec2, config: &vesper_game::AmbientActor) -> Transform {
    let factor = (size.x / 1280.0).min(size.y / 720.0).max(0.01);
    Transform {
        translation: Vec3::new(
            (config.anchor[0] - 0.5) * 1280.0 * factor,
            (0.5 - config.anchor[1]) * 720.0 * factor,
            -7.0 + config.depth * 0.25, // behind actors (>=0), above backgrounds (-10)
        ),
        scale: Vec3::splat(config.scale * factor),
        ..default()
    }
}

/// Scene-specific sprites are shown only inside their named scene; unscoped
/// sprites participate everywhere. Visibility flags are not actor expressions.
pub fn scene_visible(
    config: &vesper_game::AmbientActor, current_scene: &str,
    override_visible: Option<bool>,
) -> bool {
    override_visible.unwrap_or(config.initially_visible)
        && config.scene.as_deref().is_none_or(|name| name == current_scene)
}

pub fn spawn(
    mut commands: Commands,
    session: Res<Session>,
    assets: Res<AssetServer>,
) {
    for (id, config) in &session.game.manifest.ambient_actors {
        commands.spawn((
            Name::new(format!("Ambient scene actor: {id}")),
            AmbientVisual { id: id.clone(), image_path: config.image.clone() },
            Sprite {
                image: assets.load(config.image.clone()),
                color: Color::srgba(1.0, 1.0, 1.0, config.opacity),
                ..default()
            },
            placement(Vec2::new(1280.0, 720.0), config),
            if scene_visible(config, session.machine.scene(),
                session.presentation.ambient_visibility.get(id).copied()) {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
        ));
    }
    if !session.game.manifest.ambient_actors.is_empty() {
        info!("Vesper ambient actors spawned: {}", session.game.manifest.ambient_actors.len());
    }
}

/// Reconcile runtime entities against worker-validated external assets.
pub fn sync(
    mut commands: Commands,
    session: Res<Session>,
    assets: Res<AssetServer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut visuals: Query<(Entity, &mut AmbientVisual, &mut Sprite, &mut Transform, &mut Visibility)>,
    mut prior: Local<(Vec2, String)>,
) {
    let Ok(window) = windows.single() else { return };
    let size = Vec2::new(window.resolution.width(), window.resolution.height());
    if size.x <= 0.0 || size.y <= 0.0 { return; }
    let scene = session.machine.scene();
    if !session.is_changed() && prior.0 == size && prior.1 == scene {
        return;
    }
    let mut present = std::collections::BTreeSet::new();
    for (entity, mut visual, mut sprite, mut transform, mut visibility) in &mut visuals {
        let Some(config) = session.game.manifest.ambient_actors.get(&visual.id) else {
            commands.entity(entity).despawn();
            continue;
        };
        present.insert(visual.id.clone());
        if visual.image_path != config.image {
            sprite.image = assets.load(config.image.clone());
            visual.image_path.clone_from(&config.image);
        }
        let color = Color::srgba(1.0, 1.0, 1.0, config.opacity);
        if sprite.color != color { sprite.color = color; }
        let target = placement(size, config);
        if transform.translation != target.translation { transform.translation = target.translation; }
        if transform.scale != target.scale { transform.scale = target.scale; }
        let next = if scene_visible(config, scene,
            session.presentation.ambient_visibility.get(&visual.id).copied()) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != next { *visibility = next; }
    }
    for (id, config) in &session.game.manifest.ambient_actors {
        if present.contains(id) { continue; }
        let visible = scene_visible(config, scene,
            session.presentation.ambient_visibility.get(id).copied());
        commands.spawn((
            Name::new(format!("Ambient scene actor: {id}")),
            AmbientVisual { id: id.clone(), image_path: config.image.clone() },
            Sprite {
                image: assets.load(config.image.clone()),
                color: Color::srgba(1.0, 1.0, 1.0, config.opacity),
                ..default()
            },
            placement(size, config),
            if visible { Visibility::Visible } else { Visibility::Hidden },
        ));
    }
    prior.0 = size;
    prior.1 = scene.to_owned();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_ambient_placement_is_bounded_and_background_independent() {
        let mut actor = vesper_game::AmbientActor::default();
        actor.anchor = [0.75, 0.25];
        actor.scale = 0.5;
        actor.depth = 2.0;
        let baseline = placement(Vec2::new(1280., 720.), &actor);
        assert_eq!(baseline.translation, Vec3::new(320., 180., -6.5));
        assert_eq!(baseline.scale, Vec3::splat(0.5));
        let small = placement(Vec2::new(640., 360.), &actor);
        assert_eq!(small.translation, Vec3::new(160., 90., -6.5));
        assert_eq!(small.scale, Vec3::splat(0.25));
        assert!(baseline.translation.z < 0.0 && baseline.translation.z > -10.0);
    }
    #[test]
    fn scene_and_visibility_are_independent_of_speaking_characters() {
        let mut actor = vesper_game::AmbientActor::default();
        assert!(scene_visible(&actor, "arrival", None));
        actor.scene = Some("arrival".into());
        assert!(scene_visible(&actor, "arrival", None));
        assert!(!scene_visible(&actor, "departure", None));
        actor.initially_visible = false;
        assert!(!scene_visible(&actor, "arrival", None));
        assert!(scene_visible(&actor, "arrival", Some(true)));
        assert!(!scene_visible(&actor, "arrival", Some(false)));
        assert!(!scene_visible(&actor, "departure", Some(true)));
    }
}
