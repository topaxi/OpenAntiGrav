//! The barrel roll's drawing recipe: the ease that turns
//! `oag_physics::ShipState::roll_phase` into an angle, and the rotation built
//! from it.
//!
//! Recovered at confidence 88 - see
//! `docs/ghidra/functions/psp-pulse-usa/input-bindings.md#the-roll-is-drawn-0x87c-eases-into-0x880-which-rolls-the-ship-about-its-nose`.
//! Two call sites use it: the ship's own transform
//! (about its forward axis) and the internal camera's up vector (about the
//! view axis) - both documented at their own call site, not here, since this
//! module owns only the shared angle.
//!
//! `roll_phase` is taken as a plain `f32` everywhere in this module rather
//! than `oag_physics::ShipState`, the same way [`super::camera::Target`] is
//! three vectors rather than a `Ship`: `oag-render` does not depend on
//! `oag-physics` and this crate's own convention is a plain value at the
//! boundary, not a gameplay type.

use oag_core::math::{Quat, Vec3};

/// The angle a completed roll (`roll_phase` at `+/-1.0`, fully eased) turns
/// through, in radians.
///
/// A **code literal** in the original (`FUN_08841f88`'s `0x08841f88` store),
/// not `2.0 * PI`: a completed barrel roll is `359.8` degrees, not a full
/// turn. Carried across as-is - see the module this constant is documented
/// against. `allow`ed rather than replaced by `TAU`: the point of the
/// constant is that it is the disc's own literal, and substituting the exact
/// mathematical one would silently complete a full turn where the original
/// does not.
#[allow(clippy::approx_constant)]
pub const FULL_TURN: f32 = 6.28;

/// The roll's direction about the forward axis, for a positive `roll_phase`.
///
/// `+1.0`: a positive phase (and a positive steering lean) rotates the hull about its
/// nose so the **left** wingtip rises and the right one drops - a right bank for a
/// right stick, a left roll (left wing down) for a left-first gesture, whose phase
/// ramps negative.
///
/// **Measured, confidence 92, and read the other way round until 2026-10-21.** The
/// 2026-09-08 capture pinned `entity+0x880` at `0.0`, `+/-0.2`, `+/-0.35` and read the
/// drawn matrix back: `row0 = scale*cos(angle)*body_row0 + scale*sin(angle)*body_row1`,
/// scale `0.75`. Row 0 of the original's basis is the hull's **left**, not its right
/// (`oag_trace::replay::Basis::LeftUpForward`, measured; `right x up = forward` in the
/// capture), so a positive angle lifts the left wingtip. The test that carried the sign
/// across mapped that row onto our `+X`, the right, and shipped `-1.0`: both the lean and
/// the roll drew mirrored. `crates/raceplay/src/drawable.rs`'s
/// `roll_direction_matches_the_original_captured_display_matrix` now builds our body from
/// the captured rows the way `Basis::LeftUpForward` does and compares the left wingtip.
pub const ROLL_DIRECTION: f32 = 1.0;

/// The symmetric quadratic ease-in-out `FUN_08841f88` applies to
/// `roll_phase` before turning it into an angle.
///
/// ```text
/// ease(p) = 0                          p == 0
///         =  2p^2                      0    <  p <= 0.5
///         =  1 - 2(1-p)^2              0.5  <  p
///         = -2p^2                     -0.5 <=  p <  0
///         = -1 + 2(-1-p)^2                    p < -0.5
/// ```
///
/// Odd in `p` and continuous at `+/-0.5`, where it takes `+/-0.5` - this is
/// why the roll does not snap in and out of level. Exact arithmetic, not an
/// approximation of a smoothstep: ported as the four-branch piecewise
/// quadratic it is, rather than as a library ease function that merely looks
/// similar.
#[must_use]
pub fn ease(p: f32) -> f32 {
    if p == 0.0 {
        0.0
    } else if p > 0.5 {
        1.0 - 2.0 * (1.0 - p) * (1.0 - p)
    } else if p > 0.0 {
        2.0 * p * p
    } else if p >= -0.5 {
        -2.0 * p * p
    } else {
        -1.0 + 2.0 * (-1.0 - p) * (-1.0 - p)
    }
}

/// The angle to roll by, in radians: the eased phase times [`FULL_TURN`]
/// times [`ROLL_DIRECTION`].
///
/// The roll term alone, which is what the internal camera's up vector turns by
/// (`FUN_088455ec` reads `craft[0x880]` and not the steering lean). The hull's
/// display matrix adds the lean: [`display_angle`].
#[must_use]
pub fn angle(roll_phase: f32) -> f32 {
    ease(roll_phase) * FULL_TURN * ROLL_DIRECTION
}

/// What the steering lean is scaled by before it joins the roll's angle: the
/// `0.5` at `0x08842148` in `FUN_088418e0`.
pub const LEAN_SCALE: f32 = 0.5;

/// The angle the hull is drawn turned by about its nose, in radians.
///
/// `FUN_088418e0` builds one angle, `entity[0x854] * 0.5 + entity[0x880] * 6.28`
/// (`0x08842140`), and rotates the display matrix by it: the steering lean
/// (`oag_physics::controls::update_steer_lean`, confidence 95) and the eased
/// barrel roll share the rotation, so they share [`ROLL_DIRECTION`] too.
#[must_use]
pub fn display_angle(roll_phase: f32, steer_lean: f32) -> f32 {
    (steer_lean * LEAN_SCALE + ease(roll_phase) * FULL_TURN) * ROLL_DIRECTION
}

/// The rotation [`angle`] describes, about `axis`.
///
/// `axis` is the caller's own forward: the internal camera's view direction
/// for the up vector. The hull's own rotation is [`display_rotation`].
#[must_use]
pub fn rotation(axis: Vec3, roll_phase: f32) -> Quat {
    Quat::from_axis_angle(axis, angle(roll_phase))
}

/// The rotation [`display_angle`] describes, about `axis`, the ship's local
/// nose for the model matrix and everything placed on the drawn hull.
#[must_use]
pub fn display_rotation(axis: Vec3, roll_phase: f32, steer_lean: f32) -> Quat {
    Quat::from_axis_angle(axis, display_angle(roll_phase, steer_lean))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lean_turns_the_hull_the_way_a_positive_roll_does_at_half_scale() {
        assert_eq!(display_angle(0.0, 0.0), 0.0);
        assert_eq!(display_angle(0.0, 1.0), LEAN_SCALE * ROLL_DIRECTION);
        assert_eq!(display_angle(0.3, 0.0), angle(0.3));
        assert!((display_angle(0.3, 0.8) - (angle(0.3) + 0.4 * ROLL_DIRECTION)).abs() < 1e-6);
    }

    #[test]
    fn ease_of_zero_is_zero() {
        assert_eq!(ease(0.0), 0.0);
    }

    #[test]
    fn ease_of_a_half_is_a_half_either_side() {
        assert_eq!(ease(0.5), 0.5);
        assert_eq!(ease(-0.5), -0.5);
    }

    #[test]
    fn ease_of_one_is_one_either_side() {
        assert_eq!(ease(1.0), 1.0);
        assert_eq!(ease(-1.0), -1.0);
    }

    /// Odd in `p`: the shape the module documentation states, pinned rather
    /// than assumed. A sweep, not one sample, because the function is
    /// piecewise and a single point could pass by landing on a branch
    /// boundary that happens to be symmetric by construction.
    #[test]
    fn ease_is_odd() {
        let samples = [0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 1.0];
        for p in samples {
            assert_eq!(ease(-p), -ease(p), "p = {p}");
        }
    }

    /// Continuous at the branch seam the module documentation calls out -
    /// approaching `0.5` from either side lands on the same value ease(0.5)
    /// already pins, so this is the "does not snap" property, not a repeat of
    /// the boundary test above.
    #[test]
    fn ease_is_continuous_at_the_half_seam() {
        let just_below = ease(0.5 - f32::EPSILON * 4.0);
        let just_above = ease(0.5 + f32::EPSILON * 4.0);
        assert!((just_below - 0.5).abs() < 1e-6, "{just_below}");
        assert!((just_above - 0.5).abs() < 1e-6, "{just_above}");
    }

    /// The relationship the sign ambiguity leaves intact regardless of
    /// [`ROLL_DIRECTION`]'s value: a full roll started either way turns
    /// through the same angle in opposite senses. Sign-agnostic on purpose -
    /// see [`ROLL_DIRECTION`]'s own documentation - so this catches a
    /// regression without asserting which direction is correct.
    #[test]
    fn opposite_phases_rotate_oppositely_by_equal_magnitude() {
        for p in [0.1, 0.5, 0.75, 1.0] {
            assert_eq!(angle(-p), -angle(p), "p = {p}");
        }
    }

    /// The sense, on a hull and on the cockpit camera's up vector alike: a roll started
    /// leftwards (`roll_phase < 0`) tips the up vector towards the craft's left, and
    /// a steer held right leans the hull's up towards its right. Derived from the
    /// handedness (`right = forward x up`), so it fails if the constant flips.
    #[test]
    fn a_left_roll_and_a_right_steer_tip_up_the_way_they_name() {
        let forward = Vec3::X;
        let up = Vec3::Y;
        let right = forward.cross(up);
        let left_roll = rotation(forward, -0.25) * up;
        assert!(left_roll.dot(right) < -0.1, "{left_roll}");
        let right_roll = rotation(forward, 0.25) * up;
        assert!(right_roll.dot(right) > 0.1, "{right_roll}");
        let right_steer = display_rotation(forward, 0.0, 1.0) * up;
        assert!(right_steer.dot(right) > 0.1, "{right_steer}");
    }

    #[test]
    fn a_full_roll_turns_through_full_turn_radians() {
        assert_eq!(angle(1.0).abs(), FULL_TURN);
    }
}
