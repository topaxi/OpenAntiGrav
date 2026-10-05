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
//! - A damped cosine term rides on top of the envelope, at [`BASE_FREQUENCY`],
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
//! # Measured against the running original, 2026-09-30
//!
//! Everything below was read off PPSSPP running Pulse PSP (USA): one
//! breakpoint at `Camera_SubmitScene`'s entry (`0x08878874`) for the camera's
//! basis rows and shake fields, one at `0x08878af0` (after the shake block,
//! before the copy-out) for the basis as shaken, swapped per call. Over 352
//! active frames on real wall scrapes (magnitude `0.0009` to `0.025`) and on
//! a forced arm at magnitude `0.3` (mode 1) and `0.15` (mode 3) running the
//! whole `0.6` s decay, this module's arithmetic reproduces the shaken basis
//! to `3e-7` (float noise). Method, numbers and the capture paths are in
//! `docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`'s "Measured
//! against the original" section; the constants that pin it are in
//! `crates/render/tests/shake_ground_truth.rs`. Three things that the
//! recovered disassembly alone had wrong or open are now settled:
//!
//! - **The oscillator is a cosine**, not a sine: at the moment of impact the
//!   second rotation is `0.1 * magnitude` and the first is
//!   `(0.25 + 0.1) * magnitude`, not `0.25 * magnitude` and nothing. The call
//!   the shake-apply block makes at `0x0897e030` is the cosine of its
//!   argument in plain radians (argument `progress * 30`). Fit error `4e-7`
//!   for cosine against `3e-4` to `1e-2` for sine.
//! - **The rotation sense is the opposite of the right-handed Rodrigues
//!   formula** on the original's own world numbers ([`SENSE`]): each basis row
//!   vector is rotated by `Rodrigues(axis, -angle)`. Fit error `4e-7` against
//!   `5e-2` for the other sign. The original's world is right-handed in these
//!   numbers (`cross(row0, row1) = row2` on every recorded tick), so this is a
//!   physical fact about which way the camera turns, not a convention.
//! - **The second axis is the row as the first rotation left it**: the
//!   original's `+0x60` is re-read after the first call has rotated the
//!   whole basis in place. The variant that keeps the pre-shake row is
//!   `3e-3` off at magnitude `0.3`, the variant that re-reads is `5e-7`.
//!
//! The basis rows are `(left, up, forward)` and the two axes are the *up*
//! row (`+0x50`) and then the *forward* row (`+0x60`): a yaw about the
//! camera's own up followed by a roll about its own forward, in **world**
//! coordinates, each applied as an active rotation of the camera. The basis
//! is **not** accumulated: the next frame's entry basis is rebuilt upstream
//! (identical to `1e-4` across a forced `0.3` shake), so the shake is a pure
//! per-frame perturbation of what the camera update produced.
//!
//! The decrement is the frame's measured `dt` (`+0x124`, `0.0167 +- 0.0003`
//! on the capture), not a constant, and the shake is armed *before* the
//! frame's first submit, so the first frame after an arm sees the full
//! `0.6` s timer. A caller must therefore advance the shake at the start of
//! the next tick, before anything can arm it, not after it.
//!
//! [`Shake::apply`] is the whole composition onto a view matrix:
//! [`Shake::rotation`] builds the world-space rotation from the camera's own
//! up and forward, and `apply` turns it into a view matrix with the eye held
//! exactly where it was.
//!
//! `Camera_SubmitScene`'s third combination (`shake_mode` neither `1` nor `3`, a
//! second oscillator at `1.5x` frequency) is not implemented: nothing on the
//! collision path this module wires arms that mode, and drawing behaviour for
//! a caller nobody has found would be exactly the invented-stand-in this
//! project's own rule warns against. See `collision-shake.md`'s "Not
//! determined" if a second caller ever turns up.

use oag_core::Rng;
use oag_core::math::{Mat4, Quat, Vec3, quat_from_axis_angle, sin_cos};

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

/// The sense of rotation relative to the right-handed Rodrigues formula:
/// `-1.0`. Measured, see the module documentation.
pub const SENSE: f32 = -1.0;

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
/// Mirrors [`crate::camera::chase::Chase`] and [`oag_fx::exhaust::Exhaust`] in
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
    /// clamp [`oag_fx::sparks::severity`]'s own input is scaled from) and
    /// which side of the craft it hit - `Camera_ArmShake`'s own inputs.
    ///
    /// `rng` is the phase draw, seeded and never OS entropy - the same
    /// contract [`oag_fx::exhaust::Exhaust::advance`]'s own `rng` carries.
    pub fn arm(&mut self, severity: f32, side: Side, rng: &mut Rng) {
        self.arm_with(severity * MAGNITUDE_SCALE, DURATION_SECONDS, side, rng);
    }

    /// `Camera_ArmShake(magnitude, duration, camera, mode)` as a caller that
    /// passes its own two numbers makes it: the player's craft going out arms
    /// `(0.3, 0.4, mode 3)` at state 5 and `(0.8, 0.6, mode 1)` at state 6
    /// (`FUN_0883e064`, `FUN_088407b0`). `rng` draws the phase as for
    /// [`Self::arm`].
    pub fn arm_with(&mut self, magnitude: f32, duration: f32, side: Side, rng: &mut Rng) {
        self.magnitude = magnitude;
        self.duration = duration;
        self.timer = duration;
        self.side = side;
        self.phase = PHASE_RANGE.0 + (PHASE_RANGE.1 - PHASE_RANGE.0) * rng.next_f32();
    }

    /// Ends a running shake: `Ship_SetState` case 4 writes `0.0` to the
    /// camera's duration field (`+0xe4`) for the local player.
    pub fn cancel(&mut self) {
        self.timer = 0.0;
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

    /// The magnitude the shake was armed with, before the envelope.
    #[must_use]
    pub fn magnitude(&self) -> f32 {
        self.magnitude
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

    /// This tick's rotation of the camera, as a world-space active rotation of
    /// its basis: [`Quat::IDENTITY`] once the shake has decayed away or was
    /// never armed.
    ///
    /// `up` and `forward` are the camera's own up and forward vectors in world
    /// coordinates, the original's basis rows `+0x50` and `+0x60`. Two
    /// rotations are built and composed in the original's order: about `up`
    /// by the mode's envelope-plus-oscillator angle ([`Side::Ahead`] negates
    /// it), then about `forward` as the first rotation left it (the original
    /// re-reads that row after the first call has rotated the basis in place)
    /// by the oscillator alone. Both angles are taken in the original's own
    /// sense, [`SENSE`].
    ///
    /// Apply the result to each basis vector (`R * v`) to get the shaken
    /// basis, or use [`Self::apply`] for a view matrix.
    #[must_use]
    pub fn rotation(&self, up: Vec3, forward: Vec3) -> Quat {
        if self.timer <= 0.0 || self.duration <= 0.0 {
            return Quat::IDENTITY;
        }
        let remaining = self.timer / self.duration;
        let progress = 1.0 - remaining;
        let envelope = self.envelope(progress);
        let (_, cos_a) = sin_cos(progress * BASE_FREQUENCY);
        let oscillator = cos_a * remaining * OSCILLATOR_SCALE * self.magnitude;
        let angle1 = match self.side {
            Side::Elsewhere => envelope + oscillator,
            Side::Ahead => -(envelope + oscillator),
        };
        let angle2 = oscillator;
        let first = quat_from_axis_angle(up, SENSE * angle1);
        let second = quat_from_axis_angle(first * forward, SENSE * angle2);
        second * first
    }

    /// The shake as a world-space transform on the camera: `view * matrix(view)`
    /// is `view` with this tick's shake applied - the camera turned about its
    /// own up and forward by [`Self::rotation`], its eye exactly where it was.
    /// [`Mat4::IDENTITY`] when no shake is active.
    ///
    /// The camera's up and forward are read off `view` itself (a right-handed
    /// view matrix's rows are right, up and back in world coordinates), so
    /// the rotation is always about the axes the camera has *this* frame, as
    /// the original re-reads them every frame.
    ///
    /// A world-space factor on the right of the view is what lets the motion
    /// blur take the shake out of its velocity: the previous tick's unshaken
    /// view times this tick's matrix measures the same scene through the same
    /// shake, so a camera that holds still reads as still however hard it is
    /// shaking. See `oag_game::race::Race::shake_matrix`.
    #[must_use]
    pub fn matrix(&self, view: Mat4) -> Mat4 {
        if !self.active() {
            return Mat4::IDENTITY;
        }
        let up = view.row(1).truncate();
        let forward = -view.row(2).truncate();
        let world = self.rotation(up, forward);
        let eye = view.inverse().w_axis.truncate();
        // The camera's world transform `C` becomes `T(eye) * R * T(-eye) * C`,
        // so the view, its inverse, becomes `V * T(eye) * R^-1 * T(-eye)`.
        Mat4::from_translation(eye)
            * Mat4::from_quat(world.inverse())
            * Mat4::from_translation(-eye)
    }

    /// `view` with this tick's shake applied: see [`Self::matrix`].
    #[must_use]
    pub fn apply(&self, view: Mat4) -> Mat4 {
        view * self.matrix(view)
    }
}

#[cfg(test)]
mod tests;
