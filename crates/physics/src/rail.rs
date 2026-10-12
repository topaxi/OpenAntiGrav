//! The one write to a craft's body from outside the step: the original
//! opponent's rail, `Ai_SnapToLane` (`0x088542e4`), which sets the body's
//! linear velocity and position every tick it fires and never its
//! orientation (`docs/ghidra/functions/psp-pulse-usa/ai-controls.md`,
//! confidence 90 for the mechanism, measured firing every tick).
//!
//! Kept to its own function so the bypass is named where it happens and found
//! by a search: nothing else in the simulation moves a body this way except a
//! respawn.

use crate::ship::Body;
use oag_core::math::Vec3;

/// Sets `body`'s linear velocity and position, as the rail asks. Orientation,
/// angular velocity and the force accumulators are left alone.
pub fn apply(body: &mut Body, velocity: Vec3, position: Vec3) {
    body.linear_velocity = velocity;
    body.position = position;
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::math::Quat;

    #[test]
    fn only_velocity_and_position_are_written() {
        let mut body = Body {
            orientation: Quat::from_xyzw(0.0, 0.6, 0.0, 0.8),
            angular_velocity: Vec3::new(0.1, 0.2, 0.3),
            ..Body::default()
        };
        let before = body;
        apply(
            &mut body,
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
        );
        assert_eq!(
            (body.linear_velocity, body.position),
            (Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0))
        );
        assert_eq!(body.orientation, before.orientation);
        assert_eq!(body.angular_velocity, before.angular_velocity);
        assert_eq!((body.force, body.torque), (before.force, before.torque));
    }
}
