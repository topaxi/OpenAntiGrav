//! The barrel roll's drawing recipe: the ease that turns
//! `oag_physics::ShipState::roll_phase` into an angle, and the rotation built
//! from it.
//!
//! Recovered at confidence 88 - see
//! `docs/ghidra/functions/psp-pulse-usa/input-bindings.md#the-roll-is-drawn-0x87c-eases-into-0x880-which-rolls-the-ship-about-its-nose`
//! and `handover/the-barrel-roll-has-a-drawing-job.md`, which this module is
//! the drawing half of. Two call sites use it: the ship's own transform
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
/// **Chosen, not measured.** The VFPU register-name row/column transpose that
/// recovered this recipe settles the rotation's *axis* - the ship's own
/// nose - because both literal `1.0`s of the rotation matrix sit on the
/// diagonal, which survives a transpose; it does not settle the *sign*,
/// because that reaches exactly the `sin`/`-sin` placement the transpose
/// does move. See
/// `handover/the-barrel-roll-has-a-drawing-job.md`.
///
/// `1.0` is picked so that a `[1, 2, 1]` gesture - tap LEFT first, which ramps
/// `roll_phase` toward `-1.0` - drops the **left** wing (anticlockwise seen
/// from behind): the mapping a player's gesture would suggest, and so the one
/// least likely to need flipping. It is a choice pending a play-test or a
/// capture, not a confidence-scored finding - do not cite it as one.
/// Flipping it to `-1.0` is the entire fix if it reads backwards; this is the
/// one place that has to change.
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
/// This is the roll term alone. The original's angle is
/// `craft[0x854] * 0.5 + craft[0x880] * 6.28`, and the first term -
/// `FUN_0883fab4`'s steering lean, confidence **0** for what it physically
/// means - is deliberately **not** transcribed here; see
/// `handover/the-barrel-roll-has-a-drawing-job.md`. So the reimplementation's
/// total roll angle differs from the original's by whatever that term
/// contributes - an honest, documented gap rather than a guessed constant.
#[must_use]
pub fn angle(roll_phase: f32) -> f32 {
    ease(roll_phase) * FULL_TURN * ROLL_DIRECTION
}

/// The rotation [`angle`] describes, about `axis`.
///
/// `axis` is the caller's own forward: the ship's local nose for the model
/// matrix, the internal camera's view direction for the up vector. Neither
/// call site lives here - see them for the axis each one passes.
#[must_use]
pub fn rotation(axis: Vec3, roll_phase: f32) -> Quat {
    Quat::from_axis_angle(axis, angle(roll_phase))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn a_full_roll_turns_through_full_turn_radians() {
        assert_eq!(angle(1.0).abs(), FULL_TURN);
    }
}
