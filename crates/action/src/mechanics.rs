//! Deterministic, time-step-independent movement and collision rules.
//! Pure helpers allow testing action mechanics without booting a display.

use bevy::prelude::*;

pub const HALF_ROOM: Vec2 = Vec2::new(308.0, 166.0);
pub const PLAY_LIMITS: Vec2 = Vec2::new(289.0, 116.0);
pub const HERO_RADIUS: f32 = 12.0;

#[derive(Debug, Clone, Copy)]
pub struct Solid {
    pub center: Vec2,
    pub half: Vec2,
}

pub fn room_solids() -> [Solid; 2] {
    [
        Solid {
            center: Vec2::new(-205.0, 26.0),
            half: Vec2::new(48.0, 41.0),
        },
        Solid {
            center: Vec2::new(179.0, 20.0),
            half: Vec2::new(29.0, 36.0),
        },
    ]
}

fn intersects_circle_aabb(point: Vec2, radius: f32, wall: Solid) -> bool {
    let nearest = point.clamp(wall.center - wall.half, wall.center + wall.half);
    point.distance_squared(nearest) < radius * radius
}

fn legal(point: Vec2, radius: f32, solids: &[Solid]) -> bool {
    point.x >= -PLAY_LIMITS.x + radius
        && point.x <= PLAY_LIMITS.x - radius
        && point.y >= -PLAY_LIMITS.y + radius
        && point.y <= PLAY_LIMITS.y - radius
        && solids.iter().all(|wall| !intersects_circle_aabb(point, radius, *wall))
}

/// Separate axes so the actor slides along a cabinet rather than freezing.
pub fn slide_move(point: Vec2, desired_delta: Vec2, radius: f32, solids: &[Solid]) -> Vec2 {
    let mut next = point;
    let along_x = Vec2::new(point.x + desired_delta.x, point.y);
    if legal(along_x, radius, solids) {
        next.x = along_x.x;
    }
    let along_y = Vec2::new(next.x, next.y + desired_delta.y);
    if legal(along_y, radius, solids) {
        next.y = along_y.y;
    }
    next
}

pub fn movement_axis(left: bool, right: bool, down: bool, up: bool) -> Vec2 {
    Vec2::new((right as i8 - left as i8) as f32, (up as i8 - down as i8) as f32)
        .normalize_or_zero()
}

/// Melee attacks must actually face the target; distance alone is not enough.
pub fn directional_hit(origin: Vec2, facing: Vec2, target: Vec2) -> bool {
    let offset = target - origin;
    offset.length_squared() <= 58.0_f32.powi(2)
        && offset.length_squared() >= 1.0
        && offset.normalize_or_zero().dot(facing.normalize_or_zero()) >= 0.38
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_movement_is_not_faster() {
        let diagonal = movement_axis(false, true, false, true);
        assert!((diagonal.length() - 1.0).abs() < 0.0001);
        assert_eq!(movement_axis(true, true, false, false), Vec2::ZERO);
    }

    #[test]
    fn cabinet_blocks_walking_but_actor_slides_past_it() {
        let wall = Solid {
            center: Vec2::ZERO,
            half: Vec2::splat(20.0),
        };
        let new_pos = slide_move(
            Vec2::new(-33.0, -15.0),
            Vec2::new(14.0, -12.0),
            9.0,
            &[wall],
        );
        assert_eq!(new_pos.x, -33.0);
        assert_eq!(new_pos.y, -27.0);
    }

    #[test]
    fn facing_matters_for_attack_hits() {
        let origin = Vec2::ZERO;
        let right = Vec2::X;
        assert!(directional_hit(origin, right, Vec2::new(40.0, 4.0)));
        assert!(!directional_hit(origin, right, Vec2::new(-22.0, 0.0)));
        assert!(!directional_hit(origin, right, Vec2::new(90.0, 0.0)));
    }
}
