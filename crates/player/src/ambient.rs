//! Independent, optionally scene-scoped ambient scenery. QOL06 owns stable
//! spawning and placement; QOL07 supplies authored motion/flipbook updates.
//! No filesystem operations, motion path parsing or image decoding in Update.
use super::*;

#[derive(Component)]
pub struct AmbientVisual { pub id: String, pub image_path: String }

/// Bounded clock and resolved frame handles; no parsing or file IO per frame.
#[derive(Component)]
pub struct MotionClock {
    pub elapsed: f32,
    pub configured: Option<vesper_game::AmbientMotion>,
    pub handles: Vec<Handle<Image>>,
    pub initial_phase: f32,
    pub restored_elapsed: Option<f32>,
}
impl MotionClock {
    fn new(config: &vesper_game::AmbientActor, assets: &AssetServer, saved: Option<f32>) -> Self {
        let handles = config.motion.as_ref().map_or_else(Vec::new, |motion|
            motion.frames.iter().map(|path| assets.load::<Image>(path.clone())).collect());
        Self {
            elapsed: saved.unwrap_or(config.start_phase_seconds),
            configured: config.motion.clone(), handles,
            initial_phase: config.start_phase_seconds,
            restored_elapsed: saved,
        }
    }
}

/// Snapshot a clock only on an explicit save request.
pub fn snapshot_presentation(
    session: &Session,
    clocks: &Query<(&AmbientVisual, &MotionClock)>,
) -> PresentationState {
    let mut state = session.presentation.clone();
    state.ambient_motion_elapsed.clear();
    for (actor, clock) in clocks {
        if clock.configured.is_some() {
            state.ambient_motion_elapsed.insert(actor.id.clone(), clock.elapsed);
        }
    }
    state
}

/// Normalized [0,1] stage anchors and authored depth, independent of spoken
/// character position and cinematic background-cover cropping.
pub fn placement(size: Vec2, config: &vesper_game::AmbientActor) -> Transform {
    placement_at(size, config, config.anchor)
}
pub fn placement_at(size: Vec2, config: &vesper_game::AmbientActor, anchor: [f32; 2]) -> Transform {
    let factor = (size.x / 1280.0).min(size.y / 720.0).max(0.01);
    Transform {
        translation: Vec3::new(
            (anchor[0] - 0.5) * 1280.0 * factor,
            (0.5 - anchor[1]) * 720.0 * factor,
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
            MotionClock::new(config, &assets,
                session.presentation.ambient_motion_elapsed.get(id).copied()),
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
    mut visuals: Query<(Entity, &mut AmbientVisual, &mut MotionClock, &mut Sprite, &mut Transform, &mut Visibility)>,
    mut prior: Local<(Vec2, String)>,
    mut last_restore_epoch: Local<u64>,
) {
    let Ok(window) = windows.single() else { return };
    let size = Vec2::new(window.resolution.width(), window.resolution.height());
    if size.x <= 0.0 || size.y <= 0.0 { return; }
    let scene = session.machine.scene();
    let loaded = *last_restore_epoch != session.ambient_restore_epoch;
    if !loaded && !session.is_changed() && prior.0 == size && prior.1 == scene {
        return;
    }
    let mut present = std::collections::BTreeSet::new();
    for (entity, mut visual, mut clock, mut sprite, mut transform, mut visibility) in &mut visuals {
        let Some(config) = session.game.manifest.ambient_actors.get(&visual.id) else {
            info!("Vesper ambient actor reconciled removed entity: {}", visual.id);
            commands.entity(entity).despawn();
            continue;
        };
        present.insert(visual.id.clone());
        if visual.image_path != config.image {
            sprite.image = assets.load(config.image.clone());
            visual.image_path.clone_from(&config.image);
        }
        let saved = session.presentation.ambient_motion_elapsed.get(&visual.id).copied();
        if clock.configured != config.motion || clock.initial_phase != config.start_phase_seconds {
            *clock = MotionClock::new(config, &assets, saved);
            // On removal or replacement of a flipbook, restore its base art.
            // Otherwise the previous animated frame can remain stuck forever
            // after a hot reload switches an actor back to static scenery.
            sprite.image = assets.load(config.image.clone());
            sprite.flip_x = false;
            info!("Vesper ambient motion reconciled: {}", visual.id);
        } else if loaded || clock.restored_elapsed != saved {
            clock.elapsed = saved.unwrap_or(config.start_phase_seconds);
            clock.restored_elapsed = saved;
            if loaded {
                info!("Vesper ambient motion clock restored: {} to {:.3}s",
                    visual.id, clock.elapsed);
            }
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
            MotionClock::new(config, &assets,
                session.presentation.ambient_motion_elapsed.get(id).copied()),
            Sprite {
                image: assets.load(config.image.clone()),
                color: Color::srgba(1.0, 1.0, 1.0, config.opacity),
                ..default()
            },
            placement(size, config),
            if visible { Visibility::Visible } else { Visibility::Hidden },
        ));
        info!("Vesper ambient actor reconciled new entity: {id}");
    }
    prior.0 = size;
    prior.1 = scene.to_owned();
    *last_restore_epoch = session.ambient_restore_epoch;
}

/// Frame-by-frame path position from a cached, bounded clock.
pub fn tick_motion(
    time: Res<Time>,
    session: Res<Session>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut actors: Query<(&AmbientVisual, &mut MotionClock, &mut Transform,
        &mut Sprite, &Visibility)>,
) {
    let Ok(window) = windows.single() else { return };
    let size = Vec2::new(window.resolution.width(), window.resolution.height());
    if size.x <= 0.0 || size.y <= 0.0 { return };
    let reduced = session.preferences.reduced_motion.unwrap_or(false);
    let freeze = reduced || session.paused || session.menu_overlay.is_some()
        || session.history.visible() || session.finished;
    for (visual, mut clock, mut transform, mut sprite, visibility) in &mut actors {
        let Some(config) = session.game.manifest.ambient_actors.get(&visual.id) else { continue };
        let Some(motion) = &config.motion else { continue };
        if *visibility == Visibility::Hidden { continue; }
        if !freeze {
            let cycle = match motion.mode {
                vesper_game::AmbientMotionMode::Once => motion.period_seconds,
                vesper_game::AmbientMotionMode::PingPong => motion.period_seconds * 2.0,
                vesper_game::AmbientMotionMode::Loop => motion.period_seconds,
            };
            clock.elapsed = if matches!(motion.mode, vesper_game::AmbientMotionMode::Once) {
                (clock.elapsed + time.delta_secs()).min(cycle)
            } else {
                (clock.elapsed + time.delta_secs()).rem_euclid(cycle)
            };
        }
        let position = if reduced { 0.0 } else { clock.elapsed };
        let sample = vesper_game::ambient_motion::sample(motion, position);
        let target = placement_at(size, config, sample.anchor);
        if transform.translation != target.translation { transform.translation = target.translation; }
        if transform.scale != target.scale { transform.scale = target.scale; }
        if !clock.handles.is_empty() {
            let handle = &clock.handles[sample.frame_index];
            if sprite.image != *handle { sprite.image = handle.clone(); }
        }
        if sprite.flip_x != sample.facing_left { sprite.flip_x = sample.facing_left; }
    }
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
