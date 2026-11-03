//! Deterministic side-on locomotion, independent of Bevy and story input.
//!
//! This module deliberately has no jumping, collisions, physics engine,
//! procedural quests, hidden movement triggers, or forced travel.
use crate::experience::{Facing, WorldPosition};

/// A bounded movement step in logical game-space pixels.
/// Time spikes are capped: losing focus or opening a menu cannot teleport.
pub fn walk(position: &mut WorldPosition, axis: i8, delta_seconds: f32,
    speed: f32, width: f32) -> bool {
    if !delta_seconds.is_finite() || delta_seconds <= 0.0
        || !speed.is_finite() || speed <= 0.0
        || !width.is_finite() || width <= 0.0
        || !position.x.is_finite()
    {
        return false;
    }
    let direction = axis.signum();
    if direction == 0 { return false; }
    let before = position.x;
    position.x = (before + f32::from(direction) * speed * delta_seconds.min(0.05))
        .clamp(0.0, width);
    // Facing remains stable when pushing against the end of a street.
    if position.x != before {
        position.facing = if direction < 0 { Facing::Left } else { Facing::Right };
        true
    } else {
        false
    }
}

/// Render projection: authored world coordinates to a centered camera offset,
/// with a single-screen location never forced to scroll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub center_x: f32,
    pub world_to_screen: f32,
}
impl View {
    pub fn for_area(width: f32, height: f32, viewport_width: f32,
        viewport_height: f32, zoom: f32, player_x: f32, follow: bool) -> Self {
        let scale = (viewport_height / height.max(1.0)).clamp(0.05, 8.0)
            * zoom.clamp(0.5, 2.0);
        let visible = viewport_width.max(1.0) / scale;
        let center = if !follow || width <= visible {
            width / 2.0
        } else {
            player_x.clamp(visible / 2.0, width - visible / 2.0)
        };
        Self { center_x: center, world_to_screen: scale }
    }

    pub fn x(self, world_x: f32) -> f32 {
        (world_x - self.center_x) * self.world_to_screen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor() -> WorldPosition {
        WorldPosition::new("main_street", 100.0, Facing::Right).unwrap()
    }

    #[test]
    fn deterministic_walk_and_reverse_are_bounded() {
        let mut a = actor();
        assert!(walk(&mut a, 1, 0.02, 200.0, 1280.0));
        assert_eq!(a.x, 104.0);
        assert!(walk(&mut a, -1, 0.02, 200.0, 1280.0));
        assert_eq!(a.x, 100.0);
        assert_eq!(a.facing, Facing::Left);
        assert!(!walk(&mut a, 0, 1.0, 200.0, 1280.0));
    }

    #[test]
    fn focus_loss_cannot_jump_player_to_end_of_location() {
        let mut a = actor();
        assert!(walk(&mut a, 1, 20.0, 200.0, 1280.0));
        assert_eq!(a.x, 110.0);
        for _ in 0..500 { walk(&mut a, 1, 0.05, 200.0, 1280.0); }
        assert_eq!(a.x, 1280.0);
        assert!(!walk(&mut a, 1, 0.01, 200.0, 1280.0));
        assert_eq!(a.facing, Facing::Right);
    }

    #[test]
    fn invalid_time_or_speed_never_changes_position() {
        for dt in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
            let mut a = actor();
            assert!(!walk(&mut a, 1, dt, 200.0, 1280.0));
            assert_eq!(a.x, 100.0);
        }
        let mut a = actor();
        assert!(!walk(&mut a, 1, 0.016, -30.0, 1280.0));
        assert!(!walk(&mut a, 1, 0.016, 230.0, f32::NAN));
    }

    #[test]
    fn camera_remains_stationary_on_one_screen_and_clamps_long_streets() {
        let single = View::for_area(640.0, 720.0, 1280.0, 720.0, 1.0, 20.0, true);
        assert_eq!(single.center_x, 320.0);
        let long = View::for_area(2500.0, 720.0, 1280.0, 720.0, 1.0, 2200.0, true);
        assert_eq!(long.center_x, 1860.0);
        assert_eq!(long.x(2500.0), 640.0);
        let lock = View::for_area(2500.0, 720.0, 1280.0, 720.0, 1.0, 2200.0, false);
        assert_eq!(lock.center_x, 1250.0);
    }

    #[test]
    fn viewport_scaling_changes_projection_not_world_position() {
        let a = actor();
        let small = View::for_area(1280.0, 720.0, 946.0, 511.0, 1.0, a.x, true);
        let full = View::for_area(1280.0, 720.0, 1280.0, 720.0, 1.0, a.x, true);
        assert!(small.world_to_screen < full.world_to_screen);
        assert_eq!(a.x, 100.0);
        assert!(small.x(1280.0).is_finite());
    }
}
