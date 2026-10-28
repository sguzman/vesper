//! Engine-owned, externally styled character staging; never runs asset I/O
//! on the Bevy update thread. The .vn show/hide commands remain unchanged.
use super::*;
use vesper_game::CharacterStage;

#[derive(Component)]
pub struct ActorEntrance {
    pub previous: Option<Entity>,
    pub elapsed: f32,
    pub duration: f32,
    /// Keep fast-forward requests made before the async image finishes loading.
    pub skip_requested: bool,
}

#[derive(Component)]
pub struct ActorExit {
    pub elapsed: f32,
    pub duration: f32,
}

#[derive(Component, Default)]
pub struct ActorIdle {
    pub elapsed: f32,
}

fn ease(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Fit an authored 320x500 character within the current logical viewport.
/// Stage coordinates are relative to the existing left/center/right slots;
/// old manifests with default stage values retain their exact geometry.
pub fn placement(
    viewport: Vec2,
    factor: f32,
    position: &str,
    stage: &CharacterStage,
    entrance: Option<&ActorEntrance>,
    exit: Option<&ActorExit>,
    idle: Option<&ActorIdle>,
    reduced_motion: bool,
) -> Transform {
    let base_x = match position {
        "left" => -350.0,
        "right" => 350.0,
        _ => 0.0,
    };
    let scale = factor * stage.scale;
    let max_x = (viewport.x * 0.5 - 160.0 * scale - 6.0).max(0.0);
    let max_y = (viewport.y * 0.5 - 250.0 * scale - 6.0).max(0.0);
    let x = ((base_x + stage.offset_x) * factor).clamp(-max_x, max_x);
    let mut y = (-30.0 + stage.offset_y) * factor;
    if !reduced_motion && stage.idle_pixels > 0.0 {
        if let Some(phase) = idle {
            y += (phase.elapsed * std::f32::consts::TAU
                / stage.idle_period_seconds).sin() * stage.idle_pixels * factor;
        }
    }
    if !reduced_motion {
        if let Some(entrance) = entrance {
            let progress = ease(entrance.elapsed / entrance.duration);
            y -= stage.slide_pixels * (1.0 - progress) * factor;
        }
        if let Some(exit) = exit {
            let progress = ease(exit.elapsed / exit.duration);
            y += stage.slide_pixels * progress * factor;
        }
    }
    y = y.clamp(-max_y, max_y);
    // An incoming expression must composite above the retained opaque
    // outgoing sprite. Equal-Z transparent sprites have an unspecified draw
    // order, which can otherwise collapse a crossfade into an abrupt swap.
    let front = entrance.is_some_and(|effect| effect.previous.is_some());
    let z = stage.depth + if front { 0.02 } else { 0.0 };
    Transform::from_xyz(x, y, z).with_scale(Vec3::splat(scale))
}

/// Finish presentation-only animation without advancing the narrative. Pauses
/// freeze animation clocks; the reduced-motion switch completes them instantly.
pub fn tick(
    time: Res<Time>,
    session: Res<Session>,
    settings: Res<settings::SettingsState>,
    server: Res<AssetServer>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut sprites: Query<(
        Entity, &CharacterVisual, &mut Sprite,
        Option<&mut ActorEntrance>, Option<&mut ActorExit>, &mut ActorIdle,
    )>,
) {
    let active = !session.paused && session.menu_overlay.is_none()
        && settings.overlay.is_none();
    let reduced = session.preferences.reduced_motion.unwrap_or(false);
    let dt = if active { time.delta_secs().clamp(0.0, 0.1) } else { 0.0 };
    let skip = reduced || (active && keys.just_pressed(KeyCode::KeyK));
    let mut retained = Vec::new();
    for (entity, visual, _, entrance, _, _) in &mut sprites {
        if session.characters.get(&visual.character) == Some(&entity) {
            if let Some(entrance) = entrance {
                if let Some(previous) = entrance.previous {
                    retained.push(previous);
                }
            }
        }
    }
    for (entity, visual, mut sprite, entrance, exiting, mut idle) in &mut sprites {
        let current = session.characters.get(&visual.character) == Some(&entity);
        if let Some(mut exit) = exiting {
            if skip { exit.elapsed = exit.duration; }
            else { exit.elapsed = (exit.elapsed + dt).min(exit.duration); }
            let opacity = 1.0 - ease(exit.elapsed / exit.duration);
            sprite.color = Color::srgba(1.0, 1.0, 1.0, opacity);
            if exit.elapsed >= exit.duration {
                commands.entity(entity).despawn();
                info!("Vesper character exit complete: {}", visual.character);
            }
            continue;
        }
        if !current {
            if retained.contains(&entity) {
                // Interrupted incoming expressions may have been transparent.
                sprite.color = Color::WHITE;
            } else {
                commands.entity(entity).despawn();
            }
            continue;
        }
        if active && !reduced {
            // Bounded periodic clock avoids accumulating unbounded animation
            // time and is deterministic for a given authored period and delta.
            let period = session.game.manifest.characters
                .get(&visual.character)
                .map_or(4.0, |c| c.stage.idle_period_seconds);
            idle.elapsed = (idle.elapsed + dt).rem_euclid(period);
        }
        let Some(mut entering) = entrance else { continue; };
        if skip { entering.skip_requested = true; }
        match server.load_state(sprite.image.id()) {
            LoadState::Loaded => {}
            LoadState::Failed(error) => {
                warn!("Vesper staged character image unavailable: {error}");
                continue;
            }
            _ => continue,
        }
        if entering.skip_requested { entering.elapsed = entering.duration; }
        else { entering.elapsed = (entering.elapsed + dt).min(entering.duration); }
        let alpha = ease(entering.elapsed / entering.duration);
        sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
        if entering.elapsed >= entering.duration {
            if let Some(previous) = entering.previous {
                commands.entity(previous).despawn();
            }
            commands.entity(entity).remove::<ActorEntrance>();
            info!("Vesper character entrance complete: {}", visual.character);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placements_stay_in_viewport_and_depth_is_authored() {
        let mut stage = CharacterStage {
            offset_x: 250.0, offset_y: 100.0, scale: 1.35,
            depth: 6.0, ..default()
        };
        for size in [Vec2::new(946.0,511.0), Vec2::new(955.0,520.0),
            Vec2::new(800.0,600.0), Vec2::new(1920.0,1080.0)] {
            let factor = (size.x / 1280.0).min(size.y / 720.0);
            let t = placement(size, factor, "right", &stage, None, None, None, false);
            assert_eq!(t.translation.z, 6.0);
            assert!(t.translation.x.abs() + 160.0*t.scale.x <= size.x*0.5 + 0.01);
            assert!(t.translation.y.abs() + 250.0*t.scale.y <= size.y*0.5 + 0.01);
        }
        stage.offset_x = 0.0;
        stage.offset_y = 0.0;
        stage.scale = 1.0;
        let t = placement(Vec2::new(1280.0,720.0), 1.0, "left", &stage,
            None, None, None, false);
        assert_eq!(t.translation.x, -350.0);
        assert_eq!(t.translation.y, -30.0);
    }

    #[test]
    fn crossfade_keeps_incoming_sprite_ahead_of_retained_expression() {
        let stage = CharacterStage { depth: 4.0, ..default() };
        let entering = ActorEntrance {
            previous: Some(Entity::PLACEHOLDER),
            elapsed: 0.3,
            duration: 1.0,
            skip_requested: false,
        };
        let size = Vec2::new(1280.0, 720.0);
        let old = placement(size, 1.0, "center", &stage, None, None, None, false);
        let new = placement(size, 1.0, "center", &stage, Some(&entering), None, None, false);
        assert!(new.translation.z > old.translation.z);
        assert!(new.translation.z - old.translation.z < 0.05);
    }

    #[test]
    fn reduced_motion_skips_idle_and_slide_displacement() {
        let stage = CharacterStage {
            slide_pixels: 30.0, idle_pixels: 5.0, ..default()
        };
        let entering = ActorEntrance {
            previous: None, elapsed: 0.0, duration: 1.0,
            skip_requested: false,
        };
        let idle = ActorIdle {elapsed: 1.0};
        let full = placement(Vec2::new(1280.,720.),1., "center", &stage,
            Some(&entering),None,Some(&idle),false);
        let still = placement(Vec2::new(1280.,720.),1., "center", &stage,
            Some(&entering),None,Some(&idle),true);
        assert_ne!(full.translation.y,still.translation.y);
        assert_eq!(still.translation.y,-30.0);
    }
}
