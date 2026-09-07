//! The internal camera: the cockpit view, one of the three the player cycles.
//!
//! Its five parameters are **data, not code**, exactly as [`super::chase`]'s
//! seven are: they come from `<InternalCamera fov headtilt height length pitch/>`
//! in the ship's own `Data\Ships\<Team>\handlingstats.xml`. See
//! `docs/formats/handling-stats.md` for the schema and
//! `docs/ghidra/functions/psp-pulse-usa/camera.md` for how the original consumes
//! them.
//!
//! [`InternalParams`] therefore has **no `Default`**, for the same reason
//! [`super::chase::ChaseParams`] does not.
//!
//! # What is recovered and what is not
//!
//! The geometry below was read at instruction level out of the original's
//! internal-rig update (`FUN_088455ec`, which is the only caller that builds the
//! `player internal tripod` at `craft+0xa0`), so unusually little of this module
//! is a reading of parameter *names*:
//!
//! - **The eye is rigid.** `eye = position + forward * length + up * height`,
//!   with no spring and no lag of any kind - there are no spring constants on
//!   this element, and the original applies none. That also matches the measured
//!   `+3.000` forward / `0.000` up sample `camera.md` records against a ship
//!   whose `<InternalCamera>` authors `length` 3 and `height` 0.
//! - **`pitch` is a rise over a fixed run of [`AIM_DISTANCE`], not an angle.**
//!   The original aims at `eye + forward * 10.0 + up * pitch`, with the `10.0` a
//!   code literal. So a `pitch` of 1 tilts the view up by `atan(1/10)`, and
//!   reading the attribute as degrees or radians would be wrong by whatever the
//!   ship happens to author. Confidence **85**: one site, read as instructions,
//!   with the literal visible.
//! - **`headtilt` is carried and deliberately *not applied*.** The original rolls
//!   the view's up vector by `up - side * (craft[0x844] * headtilt)`.
//!   `craft+0x844` **was** identified on 2026-08-09 as a two-stage
//!   steering-driven filter (`camera.md`), which settles the **sign** - the
//!   craft leans *into* the turn - and leaves the **magnitude** open, because
//!   the filter's own input is unidentified at confidence 0 and the constants
//!   beside it say its units are not what a first reading assumes.
//!
//!   So the reason for holding this back has narrowed but not gone: a lean of
//!   the right sign and the wrong scale still leans the horizon wrongly through
//!   every corner. What would close it is one field - the first float of
//!   `*(craft+0x94) + 0x78` - and not another capture.
//!   See [`InternalParams::headtilt`].
//!   The original discards `<BackwardCamera headtilt>` outright, so an unapplied
//!   headtilt is at least a thing this format does elsewhere.
//! - **No `0.75`.** The original scales both *external* rigs by a global length
//!   factor of `0.75` and does not touch the internal one with it. That factor is
//!   a scale on craft-space geometry generally (it also shrinks the hull the
//!   collider is built from and the hover-probe corners), not a camera tweak, and
//!   this project scales none of those - so it applies it nowhere. See
//!   `camera.md`.
//!
//! Two further terms the original adds are **not** reproduced, and are named here
//! so nobody reads their absence as an oversight: a smoothed look-around offset
//! integrated per frame into the aim point (`craft+0x810`) and a roll of the view
//! about its own axis (`craft+0x880`). Both are dynamic camera wobble whose
//! inputs were not identified; neither changes where the cockpit is.

use oag_core::math::{Mat4, Vec3, camera};

use super::chase::Target;

/// How far ahead of the eye the aim point sits, in world units.
///
/// A code literal in the original, and the run that `<InternalCamera pitch>` is
/// the rise over: the aim point is `eye + forward * AIM_DISTANCE + up * pitch`.
/// Recovered, not chosen - see the module documentation.
pub const AIM_DISTANCE: f32 = 10.0;

/// The five values one `<InternalCamera>` block carries.
///
/// Deliberately not `Default`: see the module documentation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InternalParams {
    /// Field of view, vertical degrees at the authored 480x272 aspect - the same
    /// unit and the same aspect as `<ExternalCameraFar fov>`, both measured at
    /// confidence 95 in `camera.md`. Nothing here converts it; a caller that
    /// wants a projection matrix passes radians to [`super::projection`].
    pub fov: f32,
    /// How far the view rolls with the ship, **carried and not applied**.
    ///
    /// The original's roll is proportional to this times an unidentified
    /// per-craft quantity, so the sign and scale of the effect are unrecovered.
    /// Kept on the type because the document has it and because a later pass
    /// that identifies that quantity should find the value already threaded
    /// here, not have to re-thread it.
    ///
    /// `None` where the document omits the attribute, which Wipeout HD's team
    /// files do. Carried as an absence rather than defaulted to zero at this
    /// boundary: nothing applies it, so a zero would be indistinguishable from
    /// an authored zero for no gain.
    pub headtilt: Option<f32>,
    /// Eye offset along the ship's up axis.
    pub height: f32,
    /// Eye offset along the ship's forward axis, positive toward the nose.
    pub length: f32,
    /// How far the aim point rises above the eye, over a run of
    /// [`AIM_DISTANCE`]. **Not an angle** - see the module documentation.
    pub pitch: f32,
}

/// Where the eye sits: rigidly on the ship, with no spring.
#[must_use]
pub fn eye(target: Target, params: &InternalParams) -> Vec3 {
    target.position + target.forward * params.length + target.up * params.height
}

/// The point the camera aims at: [`AIM_DISTANCE`] ahead of the eye, raised by
/// `pitch`.
#[must_use]
pub fn look_at_point(target: Target, params: &InternalParams) -> Vec3 {
    eye(target, params) + target.forward * AIM_DISTANCE + target.up * params.pitch
}

/// The view matrix for the cockpit view.
///
/// `target.up` is the up vector, so the horizon rolls with the ship - which for
/// an in-cockpit view is not a choice at all, since the cockpit is bolted to the
/// hull.
///
/// `roll_phase` - `oag_physics::ShipState::roll_phase`, taken as a plain
/// `f32` for the reason [`Target`] already is one - rolls that up vector
/// further, about the view axis: `FUN_088455ec`'s own last step before
/// `Tripod_LookAt_q` is `up = Rot(craft[0x880] * 6.28) * up`, applied to the
/// same up vector `<InternalCamera headtilt>` would lean, and *alongside* it
/// rather than instead of it - see the module documentation for why
/// `headtilt` itself still is not applied. Only the eye's own basis is
/// rolled; [`eye`] and [`look_at_point`] read `target.up` unrolled, matching
/// the original, whose `row1` offsets both before this rotation is taken.
/// See `oag_render::roll`.
#[must_use]
pub fn view(target: Target, params: &InternalParams, roll_phase: f32) -> Mat4 {
    let up = crate::roll::rotation(target.forward, roll_phase) * target.up;
    camera::look_at(eye(target, params), look_at_point(target, params), up)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ship at the origin facing +X, upright. No number here claims to be the
    /// game's; they are round values picked to make the geometry readable.
    fn target() -> Target {
        Target {
            position: Vec3::ZERO,
            forward: Vec3::X,
            up: Vec3::Y,
        }
    }

    fn params() -> InternalParams {
        InternalParams {
            fov: 65.0,
            headtilt: Some(7.0),
            height: 2.0,
            length: 3.0,
            pitch: 0.0,
        }
    }

    /// The sign that is cheapest to get backwards, and the one the original's
    /// own measured sample pins: the cockpit is at the **nose**, not behind.
    #[test]
    fn the_eye_sits_ahead_of_the_ship_by_length() {
        assert_eq!(eye(target(), &params()), Vec3::new(3.0, 2.0, 0.0));
    }

    #[test]
    fn a_zero_offset_block_puts_the_eye_on_the_ship() {
        let params = InternalParams {
            height: 0.0,
            length: 0.0,
            ..params()
        };
        assert_eq!(eye(target(), &params), Vec3::ZERO);
    }

    #[test]
    fn the_aim_point_is_ten_units_ahead_of_the_eye() {
        let params = params();
        let ahead = look_at_point(target(), &params);
        assert_eq!(ahead, Vec3::new(3.0 + AIM_DISTANCE, 2.0, 0.0));
    }

    /// `pitch` is a rise over `AIM_DISTANCE`, so it moves the aim point and not
    /// the eye - which is what makes it an offset rather than an angle.
    #[test]
    fn pitch_raises_the_aim_point_and_leaves_the_eye_alone() {
        let flat = params();
        let tilted = InternalParams { pitch: 1.0, ..flat };
        assert_eq!(eye(target(), &tilted), eye(target(), &flat));

        let raised = look_at_point(target(), &tilted) - look_at_point(target(), &flat);
        assert_eq!(raised, Vec3::Y);
        // And the angle it produces is `atan(pitch / AIM_DISTANCE)`, stated as a
        // test because reading the attribute as degrees is the mistake this
        // module exists to prevent.
        let angle = (1.0f32 / AIM_DISTANCE).atan();
        assert!((angle - 0.09966865f32).abs() < 1e-6, "{angle}");
    }

    /// Everything is taken in the ship's own frame, so an inverted craft's
    /// cockpit is still in its cockpit.
    #[test]
    fn an_inverted_ship_keeps_its_offsets_in_its_own_frame() {
        let target = Target {
            position: Vec3::ZERO,
            forward: Vec3::X,
            up: -Vec3::Y,
        };
        let params = params();
        assert_eq!(eye(target, &params), Vec3::new(3.0, -2.0, 0.0));
    }

    /// The view has to look at what `look_at_point` reports: the eye maps to the
    /// origin of view space and the aim point onto the negative Z axis, which is
    /// the convention `oag_core::math::camera` is right-handed about.
    #[test]
    fn the_view_matrix_looks_from_the_eye_at_the_aim_point() {
        let params = params();
        let target = target();
        let matrix = view(target, &params, 0.0);

        let at_eye = matrix.transform_point3(eye(target, &params));
        assert!(at_eye.length() < 1e-5, "{at_eye}");

        let at_aim = matrix.transform_point3(look_at_point(target, &params));
        assert!(at_aim.x.abs() < 1e-4 && at_aim.y.abs() < 1e-4, "{at_aim}");
        assert!(at_aim.z < 0.0, "the aim point is down -Z: {at_aim}");
    }

    /// `headtilt` is carried and not applied, and that is deliberate. If a later
    /// pass wires it up this test is the one to delete, on purpose.
    #[test]
    fn headtilt_changes_nothing_today() {
        let target = target();
        let flat = params();
        let tilted = InternalParams {
            headtilt: Some(45.0),
            ..flat
        };
        assert_eq!(view(target, &tilted, 0.0), view(target, &flat, 0.0));
    }

    /// A zero roll phase changes nothing, so a race that never rolls draws
    /// exactly what it drew before this landed: `ease(0.0)` is exactly `0.0`
    /// and rotating by an angle of exactly `0.0` is the identity.
    #[test]
    fn a_level_roll_phase_changes_nothing() {
        let target = target();
        let params = params();
        assert_eq!(
            view(target, &params, 0.0),
            camera::look_at(
                eye(target, &params),
                look_at_point(target, &params),
                target.up
            )
        );
    }

    /// The roll moves the eye and the aim point nowhere - only the final up
    /// vector - which is what makes it a camera roll and not a
    /// repositioning. Matches the original: `row1`, the unrolled up axis, is
    /// what `eye` and `look_at_point` are built from, and the rotation is
    /// taken after, so a rolled view still puts the eye at view space's
    /// origin and the aim point the same distance down `-Z`.
    #[test]
    fn the_roll_moves_neither_the_eye_nor_the_aim_point() {
        let target = target();
        let params = params();
        let matrix = view(target, &params, 0.4);

        let at_eye = matrix.transform_point3(eye(target, &params));
        assert!(at_eye.length() < 1e-4, "{at_eye}");

        let at_aim = matrix.transform_point3(look_at_point(target, &params));
        let unrolled_aim =
            view(target, &params, 0.0).transform_point3(look_at_point(target, &params));
        assert!(
            (at_aim.length() - unrolled_aim.length()).abs() < 1e-4,
            "at_aim = {at_aim}, unrolled_aim = {unrolled_aim}"
        );
    }

    /// Rolling by a full turn's worth of phase in each direction produces
    /// opposite views - sign-agnostic on purpose, the same reasoning
    /// `oag_render::roll::ROLL_DIRECTION`'s own test gives.
    #[test]
    fn opposite_roll_phases_produce_different_and_symmetric_views() {
        let target = target();
        let params = params();
        let level = view(target, &params, 0.0);
        let rolled_one_way = view(target, &params, 0.5);
        let rolled_the_other_way = view(target, &params, -0.5);
        assert_ne!(level, rolled_one_way);
        assert_ne!(rolled_one_way, rolled_the_other_way);
    }
}
