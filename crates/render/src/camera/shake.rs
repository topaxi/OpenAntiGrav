//! The camera shake a hard wall hit arms, decaying over a fixed duration.
//!
//! Recovered at instruction level from `Camera_ArmShake` and the shake-apply
//! block inside `FUN_0013e280` (PS2) / `Camera_SubmitScene` (PSP), cross-
//! corroborated between the two binaries. Evidence, addresses and confidence
//! scores are in
//! `docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`; this module
//! implements what that page confirms and flags the one thing it does not.
//!
//! # What is confirmed, cross-platform
//!
//! - A collision arms the shake with `magnitude = severity * `[`MAGNITUDE_SCALE`]
//!   and `duration = `[`DURATION_SECONDS`]` seconds flat - `severity` alone
//!   varies shake to shake, read directly off both binaries' `.data`
//!   (`DAT_0027e8dc`/`DAT_0027e8e0` on PS2, `DAT_08ab0dfc`/`DAT_08ab0e00` on
//!   PSP, identical bytes).
//! - The strength follows a fixed three-key envelope over elapsed progress:
//!   `magnitude * 0.25` at the moment of impact, down to `magnitude * 0.125`
//!   by 30% of the way through the shake's own duration, down to `0.0` at the
//!   end - authored by `Camera_ArmShake` itself, not a separate table.
//! - A damped sine term rides on top of the envelope, at [`BASE_FREQUENCY`],
//!   scaled by the remaining fraction of the timer and [`OSCILLATOR_SCALE`].
//! - `Camera_ArmShake`'s `mode` is `3` when the contact point is ahead of the
//!   craft and `1` otherwise ([`Side`]); the two combine the envelope and the
//!   oscillator differently - see [`Shake::rotation`].
//! - The apply side is a **rotation of the camera's own basis matrix**,
//!   confirmed independently on both binaries: `FUN_0025cbb0`/`FUN_0025ca48`
//!   on PS2 and `func_0x002676b4`/`func_0x00267820` on PSP both build a
//!   Rodrigues axis-angle rotation matrix from the oscillator output and
//!   matrix-multiply it into the camera's basis rows. **Not** a translation
//!   of the eye, which is what an earlier pass of the thread this module
//!   implements had assumed while the apply side was still unread.
//!
//! # What is not confirmed, and is this module's own choice
//!
//! **Which axis the rotation happens around.** Ghidra drops the carrying
//! argument from the decompiled C signature at every call level the original's
//! rotation-matrix builder is reached through (`FUN_0013e280` ->
//! `FUN_0025cbb0` -> `FUN_0025ca48`), so nothing in either binary's decompile
//! names it. [`AXIS`] rotates about the camera's own local right - the
//! common "impact nod" choice - and is flagged as this module's own stand-in
//! rather than presented as a reading, per this project's own rule against
//! inventing what the disc's data does not say. Everything else here - the
//! envelope, the oscillator term, the per-mode combination, the magnitude and
//! the duration - is the original's own numbers.
//!
//! Also unconfirmed: whether the oscillator's phase
//! (`progress * `[`BASE_FREQUENCY`]`)` is plain radians or goes through the
//! same fixed-point turn wrap the rotation-matrix builder itself uses -
//! `FUN_0020cf50`, the `sin`-shaped call the shake-apply block drives it
//! through, was not decompiled. Read as plain radians here, the simpler of
//! the two readings; either way the shake's shape - a decaying wobble - does
//! not change.
//!
//! `FUN_0013e280`'s third combination (`shake_mode` neither `1` nor `3`, a
//! second oscillator at `1.5x` frequency) is not implemented: nothing on the
//! collision path this module wires arms that mode, and drawing behaviour for
//! a caller nobody has found would be exactly the invented-stand-in this
//! project's own rule warns against. See `collision-shake.md`'s "Not
//! determined" if a second caller ever turns up.

use oag_core::Rng;
use oag_core::math::{Quat, Vec3, quat_from_axis_angle, sin_cos};

/// Magnitude scale on the arming severity - `DAT_0027e8dc`/`DAT_08ab0dfc`,
/// read identically off both binaries' `.data`.
pub const MAGNITUDE_SCALE: f32 = 0.3;

/// Duration in seconds, flat regardless of severity -
/// `DAT_0027e8e0`/`DAT_08ab0e00`.
pub const DURATION_SECONDS: f32 = 0.6;

/// Envelope keyframe positions, as a fraction of elapsed duration - the
/// literals `Camera_ArmShake` writes to `falloff_pos[0..3]`.
const ENVELOPE_POS: [f32; 3] = [0.0, 0.3, 1.0];

/// Envelope keyframe values, as a fraction of `magnitude` -
/// `Camera_ArmShake`'s `falloff_value[0..3]` divided back out.
const ENVELOPE_VALUE: [f32; 3] = [0.25, 0.125, 0.0];

/// The oscillator's base frequency - `DAT_0027e7e0`, read off `.data`.
pub const BASE_FREQUENCY: f32 = 30.0;

/// Shared scale on the oscillator term, the literal `FUN_0013e280` loads
/// alongside it.
const OSCILLATOR_SCALE: f32 = 0.1;

/// Range the phase offset is drawn from once per arm - `Rng_RangeF(0.2, 0.8)`.
pub const PHASE_RANGE: (f32, f32) = (0.2, 0.8);

/// The rotation axis - **this module's own choice, not a reading**. See the
/// module documentation's "not confirmed" section.
pub const AXIS: Vec3 = Vec3::X;

/// Which side of the craft a collision arming a shake landed on -
/// `Camera_ArmShake`'s `mode` argument, which the collision path only ever
/// arms as one of these two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// `mode = 3`: the contact point was ahead of the craft.
    Ahead,
    /// `mode = 1`: everywhere else.
    Elsewhere,
}

/// Per-frame state of one camera's impact shake.
///
/// Mirrors [`crate::camera::chase::Chase`] and [`crate::exhaust::Exhaust`] in
/// shape and for the same reason: render-only state the game crate owns and
/// advances on the simulation's fixed tick, so it never enters `World` and
/// never touches a determinism hash. `Copy`, so a caller holding one per
/// camera stays memcpy-shaped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shake {
    timer: f32,
    duration: f32,
    magnitude: f32,
    side: Side,
    phase: f32,
}

impl Default for Shake {
    fn default() -> Self {
        Self::new()
    }
}

impl Shake {
    /// No shake armed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            timer: 0.0,
            duration: DURATION_SECONDS,
            magnitude: 0.0,
            side: Side::Elsewhere,
            phase: 0.5,
        }
    }

    /// Arms the shake from a collision's clamped severity (`0..1`, the same
    /// clamp [`crate::sparks::severity`]'s own input is scaled from) and
    /// which side of the craft it hit - `Camera_ArmShake`'s own inputs.
    ///
    /// `rng` is the phase draw, seeded and never OS entropy - the same
    /// contract [`crate::exhaust::Exhaust::advance`]'s own `rng` carries.
    pub fn arm(&mut self, severity: f32, side: Side, rng: &mut Rng) {
        self.magnitude = severity * MAGNITUDE_SCALE;
        self.duration = DURATION_SECONDS;
        self.timer = DURATION_SECONDS;
        self.side = side;
        self.phase = PHASE_RANGE.0 + (PHASE_RANGE.1 - PHASE_RANGE.0) * rng.next_f32();
    }

    /// Decays one tick toward inactive.
    pub fn advance(&mut self, dt: f32) {
        self.timer = (self.timer - dt).max(0.0);
    }

    /// Whether a shake is still running.
    #[must_use]
    pub fn active(&self) -> bool {
        self.timer > 0.0
    }

    /// The envelope's value at `progress` (`0..1` through the shake's
    /// duration), piecewise-linear between [`ENVELOPE_POS`]'s three keys.
    fn envelope(&self, progress: f32) -> f32 {
        if progress <= ENVELOPE_POS[0] {
            return ENVELOPE_VALUE[0] * self.magnitude;
        }
        for i in 1..ENVELOPE_POS.len() {
            if progress <= ENVELOPE_POS[i] {
                let t = (progress - ENVELOPE_POS[i - 1]) / (ENVELOPE_POS[i] - ENVELOPE_POS[i - 1]);
                let v0 = ENVELOPE_VALUE[i - 1] * self.magnitude;
                let v1 = ENVELOPE_VALUE[i] * self.magnitude;
                return v0 + (v1 - v0) * t;
            }
        }
        ENVELOPE_VALUE[ENVELOPE_VALUE.len() - 1] * self.magnitude
    }

    /// This tick's rotation of the camera basis, [`Quat::IDENTITY`] once the
    /// shake has decayed away or was never armed.
    ///
    /// Reproduces `FUN_0013e280`'s two-call combination for `shake_mode`
    /// `1`/`3` exactly: both sides add the envelope and one oscillator term,
    /// but [`Side::Ahead`] negates the envelope+oscillator sum before adding
    /// the oscillator term back unsigned, so its two calls' oscillations
    /// cancel and only the (negated) envelope survives - a sharp, mostly
    /// non-oscillating kick for a head-on hit against a wobblier one
    /// everywhere else. That asymmetry falls straight out of the original's
    /// own arithmetic; it is not tuned here.
    #[must_use]
    pub fn rotation(&self) -> Quat {
        if self.timer <= 0.0 || self.duration <= 0.0 {
            return Quat::IDENTITY;
        }
        let remaining = self.timer / self.duration;
        let progress = 1.0 - remaining;
        let envelope = self.envelope(progress);
        let (sin_a, _) = sin_cos(progress * BASE_FREQUENCY);
        let oscillator = sin_a * remaining * OSCILLATOR_SCALE * self.magnitude;
        let angle = match self.side {
            Side::Elsewhere => envelope + oscillator + oscillator,
            Side::Ahead => -(envelope + oscillator) + oscillator,
        };
        quat_from_axis_angle(AXIS, angle)
    }
}

#[cfg(test)]
mod tests;
