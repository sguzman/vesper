//! QOL07: deterministic, bounded path and flipbook sampling.
//! Pure math: authoring is validated once by Game::load, not in Update.
use crate::{AmbientMotion, AmbientMotionMode};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionSample {
    pub anchor: [f32; 2],
    /// True when movement should mirror a left-facing sprite.
    pub facing_left: bool,
    pub frame_index: usize,
    pub finished: bool,
}

/// Sample a path in normalized stage coordinates at an absolute, monotonic
/// clock. The one-way path takes one authored period, so a ping-pong completes
/// its round trip in two. Every segment takes equal fractions of its period;
/// QOL07 requires no physics, integration or mutable frame-to-frame drift.
pub fn sample(motion: &AmbientMotion, seconds: f32) -> MotionSample {
    debug_assert!(motion.waypoints.len() >= 2);
    let elapsed = if seconds.is_finite() { seconds.max(0.0) } else { 0.0 };
    let phase = elapsed / motion.period_seconds;
    let finished = matches!(motion.mode, AmbientMotionMode::Once) && phase >= 1.0;
    let (progress, returning) = match motion.mode {
        AmbientMotionMode::Once => (phase.clamp(0.0, 1.0), false),
        AmbientMotionMode::Loop => (phase.rem_euclid(1.0), false),
        AmbientMotionMode::PingPong => {
            let period = phase.rem_euclid(2.0);
            if period >= 1.0 { (2.0-period, true) }
            else { (period, false) }
        }
    };
    let n = motion.waypoints.len()-1;
    let scaled = progress * n as f32;
    let index = (scaled.floor() as usize).min(n-1);
    let fraction = (scaled-index as f32).clamp(0.0, 1.0);
    let from = motion.waypoints[index];
    let to = motion.waypoints[index+1];
    let anchor = [
        from[0] + (to[0]-from[0])*fraction,
        from[1] + (to[1]-from[1])*fraction,
    ];
    let dx = to[0] - from[0];
    let facing_left = motion.flip_on_reverse && if returning { dx > 0.0 } else { dx < 0.0 };
    let frames = motion.frames.len();
    let frame_index = if frames == 0 { 0 } else {
        let time = if finished { motion.period_seconds } else { elapsed };
        ((time*motion.frame_fps).floor() as usize) % frames
    };
    MotionSample { anchor, facing_left, frame_index, finished }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn motion(mode: AmbientMotionMode) -> AmbientMotion {
        AmbientMotion {
            waypoints: vec![[0.1,0.5],[0.5,0.2],[0.9,0.5]],
            period_seconds: 10.0, mode, flip_on_reverse:true,
            frames:vec!["a.png".into(),"b.png".into()],frame_fps:2.0,
        }
    }
    #[test]
    fn loop_path_is_periodic_and_normalized() {
        let m=motion(AmbientMotionMode::Loop);
        assert_eq!(sample(&m,0.0).anchor,[0.1,0.5]);
        assert_eq!(sample(&m,5.0).anchor,[0.5,0.2]);
        assert_eq!(sample(&m,10.0).anchor,[0.1,0.5]);
        assert_eq!(sample(&m,25.0).anchor,[0.5,0.2]);
        assert_eq!(sample(&m,5.0),sample(&m,25.0));
        assert!(!sample(&m,5.0).finished);
    }
    #[test]
    fn pingpong_reverses_without_coordinate_drift() {
        let m=motion(AmbientMotionMode::PingPong);
        let a=sample(&m,2.0);
        let b=sample(&m,18.0);
        assert_eq!(a.anchor,b.anchor);
        assert!(!a.facing_left);
        assert!(b.facing_left);
        assert_eq!(sample(&m,10.0).anchor,[0.9,0.5]);
        assert_eq!(sample(&m,20.0).anchor,[0.1,0.5]);
    }
    #[test]
    fn one_shot_stops_at_terminal_waypoint() {
        let m=motion(AmbientMotionMode::Once);
        assert!(!sample(&m,9.9).finished);
        assert!(sample(&m,10.0).finished);
        assert!(sample(&m,800.0).finished);
        assert_eq!(sample(&m,800.0).anchor,[0.9,0.5]);
        assert_eq!(sample(&m,10.0).frame_index,sample(&m,800.0).frame_index);
    }
    #[test]
    fn frame_sampling_is_repeatable_and_bounded() {
        let m=motion(AmbientMotionMode::Loop);
        assert_eq!(sample(&m,0.0).frame_index,0);
        assert_eq!(sample(&m,0.51).frame_index,1);
        assert_eq!(sample(&m,1.01).frame_index,0);
        for i in 0..200 { assert!(sample(&m,i as f32 / 7.0).frame_index<2); }
    }
}
