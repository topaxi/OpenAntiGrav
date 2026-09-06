//! The camera shake a hard wall hit arms, decaying over a fixed duration.
//!
//! Recovered at instruction level from `Camera_ArmShake` and the shake-apply
//! block inside `Camera_SubmitScene` (both PS2 and PSP now share the name), cross-
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
//! # The rotation axis: two sequential rotations about the camera's own live basis
//!
//! **Settled 2026-09-05, implemented 2026-09-06.** The original does not rotate
//! about a fixed axis at all: each `shake_mode` reads two of the camera's own
//! *live* basis rows (`+0x50` then `+0x60` for `Ahead`/`Elsewhere`, `+0x50` then
//! `+0x40` for the third, unimplemented mode) and applies two sequential
//! rotations, one per row, composing on top of each other - confirmed
//! independently on both binaries by disassembly/decompile alone (no p-code
//! needed once the register origin was traced), see
//! `docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`'s "Settled
//! 2026-09-05" section. For the two modes the collision path actually arms,
//! the row order is **not** ambiguous: row1 first (`+0x50`), row2 second
//! (`+0x60`), in both `Ahead` and `Elsewhere` - they differ only in the first
//! rotation's angle, which `Ahead` negates. Confidence 92 (PS2 disassembly) /
//! 95 (cross-checked against the PSP's explicit pointer argument).
//!
//! [`Shake::rotation`] takes the caller's `row1`/`row2` - two [`Vec3`]s read
//! fresh from the camera's current basis every tick, never cached - and
//! applies two [`quat_from_axis_angle`] rotations in that order: `row1` with
//! the mode's own (possibly negated) envelope-plus-oscillator angle, then
//! `row2` with the oscillator alone. `row2`'s own vector is rotated by the
//! first quaternion before being used as the second rotation's axis, matching
//! the original's own fresh re-read of that offset *after* the first call has
//! already rotated the whole basis in place - the same "read back the row the
//! first call just rotated" mechanism the evidence page's "else" branch shows
//! explicitly and this page's general reasoning extends to `row2` here.
//!
//! # What is still a modeling choice, not a reading
//!
//! Two things this module cannot get from the evidence pages as written,
//! flagged here rather than silently assumed:
//!
//! - **Which concrete vectors are `row1`/`row2`.** `collision-shake.md`'s own
//!   "Not determined" section leaves what each basis row physically represents
//!   open at confidence ~55 (world right/up/forward directly, or the
//!   transposed "columns are world axes" reading `positional-audio.md`
//!   establishes for the camera elsewhere) - and says answering it is not
//!   needed to settle the axis-selection question above. `oag-game`'s own
//!   caller (`Race::view`, outside this crate - `oag-render` has no gameplay
//!   type to read a ship or a camera struct from) reads `row1`/`row2` off the
//!   camera's own already-built view matrix for the current tick (its rows 1
//!   and 2) rather than a persisted basis object like the original's camera
//!   struct - this engine does not keep one, recomputing the view fresh every
//!   frame instead. That is the closest live analogue this codebase has to
//!   "the camera's own current basis row", not a claim that it is numerically
//!   the same quantity the original's struct stores.
//! - **The handedness of the two rotations**, in two places: whether `row2`
//!   should be rotated *forward* by the first quaternion before being used as
//!   the second axis (what [`Shake::rotation`] does) or by its inverse, and
//!   whether each angle's sign matches the disc's own VU convention or its
//!   mirror. Neither is recoverable from the two evidence pages, which pin
//!   *which* offsets are read and in *what order*, not the exact sign
//!   convention the underlying `vmulabc`/`vmaddabc`-shaped instructions use.
//!   Carried the same way [`crate::roll::ROLL_DIRECTION`] carries its own
//!   unmeasured sign: a named choice, not a reading, with a one-line fix
//!   (negate the angle, or use `first.inverse() * row2`) if a future
//!   play-test or capture shows the shake twisting the wrong way.
//!
//! **What composing the result onto a view matrix does not need to guess
//! about: which side of the multiply to use.** [`Shake::rotation`] returns
//! one [`Quat`], and left-multiplying its inverse onto a view matrix - the
//! existing convention `Race::view` already uses, unchanged by this pass -
//! keeps the camera's eye at exactly its pre-shake world position for *any*
//! rotation, because a view matrix's translation column maps the eye to the
//! view-space origin and a zero-translation left factor maps that origin to
//! itself regardless of its own rotation. Right-multiplying instead would
//! orbit the eye around the world's origin by the rotation's inverse - a
//! large, visible bug for any camera not standing at `(0, 0, 0)`, which is
//! effectively always. So the multiply side is not a modeling choice at all;
//! only the two items above are.
//!
//! Also unconfirmed: whether the oscillator's phase
//! (`progress * `[`BASE_FREQUENCY`]`)` is plain radians or goes through the
//! same fixed-point turn wrap the rotation-matrix builder itself uses -
//! `FUN_0020cf50`, the `sin`-shaped call the shake-apply block drives it
//! through, was not decompiled. Read as plain radians here, the simpler of
//! the two readings; either way the shake's shape - a decaying wobble - does
//! not change.
//!
//! `Camera_SubmitScene`'s third combination (`shake_mode` neither `1` nor `3`, a
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

/// Shared scale on the oscillator term, the literal `Camera_SubmitScene` loads
/// alongside it.
const OSCILLATOR_SCALE: f32 = 0.1;

/// Range the phase offset is drawn from once per arm - `Rng_RangeF(0.2, 0.8)`.
pub const PHASE_RANGE: (f32, f32) = (0.2, 0.8);

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
    /// `row1`/`row2` are the camera's own current basis vectors for this
    /// tick, standing in for `Camera_SubmitScene`'s live re-reads of
    /// `+0x50`/`+0x60` - see the module documentation's "rotation axis"
    /// section for what a caller should pass and what is and is not
    /// confirmed about that choice. Two rotations are built and composed in
    /// the original's own order: `row1` first, with the mode's own
    /// (possibly negated) envelope-plus-oscillator angle; `row2` second,
    /// rotated by the first quaternion before it is used as an axis (the
    /// original re-reads that offset only after the first call has already
    /// rotated the whole basis in place), with the oscillator alone.
    ///
    /// [`Side::Ahead`]'s first angle - negate the envelope+oscillator sum -
    /// is the original's own arithmetic; both modes share the same second
    /// angle (the oscillator alone) and the same row order, so nothing about
    /// which row goes first is a per-mode choice here. Whether `row2` should
    /// be rotated forward by the first quaternion (as below) or by its
    /// inverse before becoming the second axis is not settled by the
    /// evidence - see the module documentation's "handedness" item.
    ///
    /// The composed result is one [`Quat`] representing "apply `row1`'s
    /// rotation, then `row2`'s" (`q2 * q1`, the right-hand factor applying
    /// first). A caller composes it onto a view matrix the same way the
    /// single-fixed-axis approximation this replaces did - left-multiplying
    /// its inverse (`rotation.inverse() * view`) - which is not itself a
    /// choice: for any rotation at all, a zero-translation *left* factor
    /// maps the view-space origin to itself, so the camera's eye stays
    /// exactly where it was and only the viewing direction turns. See the
    /// module documentation's closing paragraph.
    #[must_use]
    pub fn rotation(&self, row1: Vec3, row2: Vec3) -> Quat {
        if self.timer <= 0.0 || self.duration <= 0.0 {
            return Quat::IDENTITY;
        }
        let remaining = self.timer / self.duration;
        let progress = 1.0 - remaining;
        let envelope = self.envelope(progress);
        let (sin_a, _) = sin_cos(progress * BASE_FREQUENCY);
        let oscillator = sin_a * remaining * OSCILLATOR_SCALE * self.magnitude;
        let angle1 = match self.side {
            Side::Elsewhere => envelope + oscillator,
            Side::Ahead => -(envelope + oscillator),
        };
        let angle2 = oscillator;
        let first = quat_from_axis_angle(row1, angle1);
        let second = quat_from_axis_angle(first * row2, angle2);
        second * first
    }
}

#[cfg(test)]
mod tests;
