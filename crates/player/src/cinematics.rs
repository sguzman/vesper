//! Generic, data-authored background cinematics. No filesystem I/O or
//! allocation proportional to authored artwork occurs on the update thread.
use super::*;
use vesper_game::{BackgroundTransitionKind as Kind, CinematicTheme};

#[derive(Component, Clone, Copy)]
pub struct BackdropTransition {
    pub outgoing: Entity,
    pub kind: Kind,
    pub duration: f32,
    pub elapsed: f32,
    pub curtain: Option<Entity>,
    pub slide_pixels: f32,
    /// A K press while an incoming image is still loading is not lost.
    pub skip_requested: bool,
}

#[derive(Component)]
pub struct BackdropCurtain;

#[derive(Component, Clone, Copy)]
pub struct BackdropMotion {
    pub elapsed: f32,
    pub seconds: f32,
    pub start_zoom: f32,
    pub end_zoom: f32,
    pub pan: Vec2,
}

fn fraction(elapsed: f32, duration: f32) -> f32 {
    if duration <= 0.0 { 1.0 } else { (elapsed / duration).clamp(0.0, 1.0) }
}

fn cubic(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

pub fn create_curtain(commands: &mut Commands, theme: &CinematicTheme) -> Entity {
    let color = theme.through_color;
    commands.spawn((
        Name::new("Backdrop cinematic color curtain"),
        BackdropCurtain,
        Sprite {
            color: Color::srgba(color[0], color[1], color[2], 0.0),
            custom_size: Some(Vec2::new(1280.0, 720.0)),
            ..default()
        },
        Transform::from_xyz(0., 0., -9.0),
    )).id()
}

pub fn motion(theme: &CinematicTheme) -> Option<BackdropMotion> {
    let sliding = theme.transition == Kind::SoftSlide;
    if theme.motion_seconds == 0.0 && !sliding { return None; }
    let minimum_zoom = if sliding { 1.12 } else { 1.04 };
    let end_zoom = if theme.motion_seconds > 0.0 {
        theme.motion_zoom.max(minimum_zoom)
    } else {
        minimum_zoom
    };
    Some(BackdropMotion {
        elapsed: 0.0,
        seconds: theme.motion_seconds,
        start_zoom: minimum_zoom,
        end_zoom,
        pan: if theme.motion_seconds > 0.0 {
            Vec2::from_array(theme.motion_pan)
        } else { Vec2::ZERO },
    })
}

/// Computes transform from logical window coordinates, clamping every pan to
/// actual overscan on both axes. Never exposes image edges at narrow/wide sizes.
pub fn background_transform(
    viewport: Vec2, base_cover: f32,
    motion: Option<&BackdropMotion>, transition: Option<&BackdropTransition>,
) -> (f32, Vec2) {
    let (zoom, pan) = if let Some(m) = motion {
        let t = cubic(fraction(m.elapsed, m.seconds));
        (m.start_zoom + (m.end_zoom - m.start_zoom) * t, m.pan * t)
    } else { (1.0, Vec2::ZERO) };
    let scale = base_cover * zoom;
    let excess = Vec2::new(
        ((1280.0 * scale - viewport.x) * 0.5).max(0.0),
        ((720.0 * scale - viewport.y) * 0.5).max(0.0),
    );
    let mut offset = Vec2::new(viewport.x * pan.x, viewport.y * pan.y);
    if let Some(t) = transition {
        if t.kind == Kind::SoftSlide {
            offset.x += t.slide_pixels * (1.0 - cubic(fraction(t.elapsed, t.duration))) * base_cover;
        }
    }
    offset = offset.clamp(-excess, excess);
    (scale, offset)
}

/// An incoming background stays transparent until its image has loaded. The
/// outgoing background is retained underneath; no black flash on slow I/O.
/// At most the current and outgoing background sprites survive each tick.
pub fn tick(
    time: Res<Time>,
    session: Res<Session>,
    settings: Res<settings::SettingsState>,
    server: Res<AssetServer>,
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut backdrops: Query<(Entity, &mut Sprite, Option<&mut BackdropTransition>), With<BackgroundVisual>>,
    mut curtains: Query<(Entity, &mut Sprite), (With<BackdropCurtain>, Without<BackgroundVisual>)>,
    mut motions: Query<&mut BackdropMotion>,
) {
    let active = !session.paused && session.menu_overlay.is_none()
        && settings.overlay.is_none() && !session.history.visible();
    let reduced_motion = session.preferences.reduced_motion.unwrap_or(false);
    let dt = if active { time.delta_secs().clamp(0.0, 0.1) } else { 0.0 };
    for mut motion in &mut motions {
        if !reduced_motion && motion.seconds > 0.0 && dt > 0.0 {
            motion.elapsed = (motion.elapsed + dt).min(motion.seconds);
        }
    }
    let mut outgoing = None;
    let mut finished = None;
    let mut active_curtain = None;
    for (entity, mut sprite, maybe_transition) in &mut backdrops {
        if Some(entity) != session.background { continue; }
        let Some(mut transition) = maybe_transition else { continue; };
        outgoing = Some(transition.outgoing);
        active_curtain = transition.curtain;
        if reduced_motion || (active && input.just_pressed(KeyCode::KeyK)) {
            // Respect an accessibility change even during an in-flight effect.
            transition.skip_requested = true;
        }
        match server.load_state(sprite.image.id()) {
            LoadState::Loaded => {}
            LoadState::Failed(error) => {
                warn!("Vesper cinematic image unavailable; keeping old background: {error}");
                continue;
            }
            _ => {
                continue;
            }
        }
        if transition.skip_requested {
            transition.elapsed = transition.duration;
            transition.skip_requested = false;
            info!("Vesper cinematic skipped");
        } else {
            transition.elapsed = (transition.elapsed + dt).min(transition.duration);
        }
        let t = cubic(fraction(transition.elapsed, transition.duration));
        let alpha = match transition.kind {
            Kind::FadeThroughColor if t < 0.5 => 0.0,
            Kind::FadeThroughColor => 1.0,
            _ => t,
        };
        let next = Color::srgba(1., 1., 1., alpha);
        if sprite.color != next { sprite.color = next; }
        if let Some(curtain) = transition.curtain {
            if let Ok((_, mut layer)) = curtains.get_mut(curtain) {
                let [r,g,b,_] = session.game.theme.cinematic.through_color;
                let curtain_alpha = if t < 0.5 { t * 2.0 } else { (1.0-t) * 2.0 };
                layer.color = Color::srgba(r,g,b,curtain_alpha.clamp(0.0, 1.0));
            }
        }
        if transition.elapsed >= transition.duration {
            info!("Vesper cinematic complete: {:?}", transition.kind);
            finished = Some((entity, transition.outgoing, transition.curtain));
        }
    }
    // Any interrupted transition may leave a formerly incoming sprite with
    // low alpha. The outgoing frame must always be fully opaque.
    for (entity, mut sprite, maybe_transition) in &mut backdrops {
        if Some(entity) == outgoing && sprite.color.alpha() != 1.0 {
            sprite.color = Color::WHITE;
        }
        if Some(entity) != session.background && Some(entity) != outgoing {
            commands.entity(entity).despawn();
        } else if Some(entity) != session.background && maybe_transition.is_some() {
            commands.entity(entity).remove::<BackdropTransition>();
        }
    }
    for (entity, _) in &mut curtains {
        if Some(entity) != active_curtain { commands.entity(entity).despawn(); }
    }
    if let Some((current, old, curtain)) = finished {
        commands.entity(old).despawn();
        commands.entity(current).remove::<BackdropTransition>();
        if let Some(curtain) = curtain { commands.entity(curtain).despawn(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn easing_is_bounded_and_hits_endpoints() {
        assert_eq!(fraction(0.0, 1.0), 0.0);
        assert_eq!(fraction(1.0, 1.0), 1.0);
        assert_eq!(cubic(0.0), 0.0);
        assert_eq!(cubic(1.0), 1.0);
        assert!((cubic(0.5) - 0.5).abs() < 0.001);
    }
    #[test]
    fn ken_burns_cannot_expose_raster_edges_even_at_extreme_aspect_ratios() {
        let mut theme = CinematicTheme::default();
        theme.motion_seconds = 20.0;
        theme.motion_zoom = 1.15;
        theme.motion_pan = [0.04, -0.04];
        let mut state = motion(&theme).unwrap();
        state.elapsed = 20.0;
        for size in [Vec2::new(946.,511.),Vec2::new(2560.,1080.),
            Vec2::new(800.,700.),Vec2::new(1920.,1080.)] {
            let base = (size.x/1280.).max(size.y/720.);
            let (scale,pos) = background_transform(size,base,Some(&state),None);
            let x = (1280. * scale - size.x) * 0.5;
            let y = (720. * scale - size.y) * 0.5;
            assert!(x + 0.01 >= pos.x.abs());
            assert!(y + 0.01 >= pos.y.abs());
            assert!(scale >= base);
        }
    }
}
