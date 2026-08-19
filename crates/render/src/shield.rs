//! The shell a fired Shield pickup wraps around the craft.
//!
//! Recovered from the PSP executable's shield visual object - its constructor,
//! its activate, its per-frame update and its hit arm. Addresses, evidence and a
//! confidence score per claim are in
//! `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`; this module
//! implements what that page describes and cites it rather than restating it.
//!
//! # What the original does, in one paragraph
//!
//! `ShipShield_Construct` (`0x0885db38`) loads **two** authored models per
//! craft: `Data\Ships\<Team>\shipshield.vex`, a single additive shell the shape
//! of the hull, and `Data\Weapons\vr_shield_cockpit.vex`, a noise-textured
//! sphere. `ShipShield_Update` (`0x0885e254`) draws exactly one of them - the
//! shell from an external camera, the sphere from the cockpit - at a scale that
//! swells on impact and settles, with an alpha that flickers. `ShipShield_Hit`
//! (`0x0885eb04`) is what an absorbed hit calls, and it is the only thing that
//! makes the shell visibly react.
//!
//! # Two halves, deliberately
//!
//! This module is the state and the maths, with no `wgpu` in it at all, the same
//! split [`crate::exhaust`] uses: the recovered constants are testable on a
//! machine with no graphics driver. The models themselves are ordinary `.vex`
//! meshes and go through [`crate::mesh`] like the hull and the boost plume, so
//! there is no pipeline here to own.
//!
//! # What is recovered and what is ours
//!
//! **Recovered**, all from instruction immediates: the two lerp rates
//! ([`COLOUR_RATE`], [`SWELL_RATE`]), the swell's rest value and its hit value
//! ([`SWELL_REST`], [`SWELL_ON_HIT`]), the `0.012` base and flicker of the scale
//! ([`SCALE_BASE`], [`SCALE_FLICKER`]), the alpha flicker's `0.25`/`0.75` split
//! ([`ALPHA_FLICKER`]), the cockpit sphere's [`COCKPIT_SCALE`], the fixed
//! 60 Hz substep, and the `0.1` alpha at which a fading shell stops being drawn
//! ([`FADE_OUT_ALPHA`]).
//!
//! **Ours, and there is no way for it not to be yet.**
//!
//! - **The flicker's own curve.** The original samples one unread function of
//!   the effect's accumulated time and uses its result for both the alpha and
//!   the scale wobble. Its *range* is pinned - the two consumers only make sense
//!   for `0..=1` - and its role is recovered; the function is not. See
//!   [`flicker`], which is a stand-in for a missing *function* rather than for
//!   missing data, and which nothing else in this module depends on the shape
//!   of.
//! - **The tint.** `ShipShield_Activate` and `ShipShield_Hit` load their colours
//!   from a constant pool this project cannot yet address - see the docs page's
//!   "the three colour vec4s are not resolved". So [`ShipShield::rgba`] starts
//!   and stays at the model's own authored white, and the colour lerp runs
//!   against a target that never differs from it. **Painting a plausible blue
//!   here is exactly the invention `CLAUDE.md` forbids**, and it would also
//!   remove the pressure to resolve the constants. The lerp is implemented
//!   anyway, so the day they resolve, this is two constants and no new code.
//!
//! With no tint, what a player sees is the authored shell in its authored
//! colour, appearing on activation, bulging on every absorbed hit and fading
//! out. That is the whole recovered mechanism minus one unread colour.

/// How far the colour moves toward its target per 60 Hz substep.
///
/// `ShipShield_Activate` writes `0.15` to the object's `+0x60` and
/// `ShipShield_Update` multiplies the colour delta by it once per substep.
pub const COLOUR_RATE: f32 = 0.15;

/// How far the swell moves toward [`SWELL_REST`] per 60 Hz substep.
///
/// `+0x6c`, `0.2`. About a fifth of a second to settle a hit, which is what
/// makes the bulge read as an impact rather than a pulse.
pub const SWELL_REST_RATE: f32 = 0.2;

/// What the swell settles to. `+0x68`, `1.0`.
pub const SWELL_REST: f32 = 1.0;

/// What an absorbed hit sets the swell to. `ShipShield_Hit` stores `1.1`.
///
/// A tenth over rest, pulled back at [`SWELL_REST_RATE`]: the shell jumps out by
/// 10 % of its size and shrinks back. This is the whole of the impact response,
/// and it is why [`crate::shield::ShipShield::hit`] has to be called from the
/// tick that absorbed rather than inferred from the timer.
pub const SWELL_ON_HIT: f32 = 1.1;

/// Added to the swell to get the drawn scale. `0.012`.
pub const SCALE_BASE: f32 = 0.012;

/// How much of the drawn scale the flicker moves. `0.012`, the same magnitude
/// as [`SCALE_BASE`], so the shell breathes by about a percent.
pub const SCALE_FLICKER: f32 = 0.012;

/// The flicker's share of the alpha: `alpha * (n * 0.25 + 0.75)`.
///
/// So the shell never dims below three quarters of its colour's alpha, and the
/// flicker is a quarter of it.
pub const ALPHA_FLICKER: f32 = 0.25;

/// The floor of the alpha flicker, `1.0 - ALPHA_FLICKER`.
pub const ALPHA_FLOOR: f32 = 1.0 - ALPHA_FLICKER;

/// What the cockpit sphere's scale is multiplied by. `1.8`.
///
/// Only reachable from an internal camera, which this engine does not have -
/// kept because it is recovered and because the constant is the whole of the
/// difference between the two branches.
pub const COCKPIT_SCALE: f32 = 1.8;

/// Alpha at or below which a **fading** shell stops being drawn at all. `0.1`.
///
/// Only consulted while fading: an active shield whose colour happens to be dim
/// keeps drawing.
pub const FADE_OUT_ALPHA: f32 = 0.1;

/// The substep the update runs its two lerps at, `1.0 / 60.0` as the original
/// spells it.
///
/// **`(int)(dt / 0.016666668)` is the original's own loop bound**, not a fixed
/// timestep this project imposed: both approaches run that many times per frame
/// whatever the frame took. At this engine's 60 Hz that is exactly one, so the
/// substepping is invisible today and is implemented anyway - a variable-rate
/// caller would otherwise get a shield that settles at a different speed.
pub const SUBSTEP: f32 = 0.016_666_668;

/// A shield shell's animation state, one per craft.
///
/// Not simulation state: nothing here feeds a force, a hash or a replay. It is
/// driven *by* the simulation - armed when the pickup fires, bumped when
/// [`oag_physics::damage::Shield::absorbed`] says a hit was swallowed, faded
/// when the timer runs out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShipShield {
    /// The current colour, `0..=1` per channel.
    rgba: [f32; 4],
    /// What the colour approaches at [`COLOUR_RATE`].
    ///
    /// Equal to [`Self::rgba`]'s resting value until the activation and hit
    /// tints are recovered - see the module docs.
    rgba_target: [f32; 4],
    /// The impact swell, resting at [`SWELL_REST`].
    swell: f32,
    /// Seconds since activation, the flicker's argument.
    time: f32,
    /// Whether anything is drawn at all.
    active: bool,
    /// Whether the shell is on its way out.
    fading: bool,
}

impl Default for ShipShield {
    fn default() -> Self {
        Self::new()
    }
}

impl ShipShield {
    /// The colour a shell rests at: the model's own authored white.
    ///
    /// **A placeholder for one unread constant, and deliberately the identity.**
    /// The original loads a tint here; multiplying the authored vertex colours by
    /// white is the one choice that cannot misrepresent them. See the module
    /// docs.
    pub const UNTINTED: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

    /// A shield that is not up.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rgba: Self::UNTINTED,
            rgba_target: Self::UNTINTED,
            swell: SWELL_REST,
            time: 0.0,
            active: false,
            fading: false,
        }
    }

    /// Raise the shell, `ShipShield_Activate` (`0x0885de4c`).
    ///
    /// Resets the clock, which is what makes two shields in one race flicker the
    /// same way rather than continuing one waveform - the original stores `0` to
    /// the same field in its constructor and never elsewhere, so this is a
    /// reading of the activate rather than of a reset.
    pub fn activate(&mut self) {
        self.rgba = Self::UNTINTED;
        self.rgba_target = Self::UNTINTED;
        self.swell = SWELL_REST;
        self.time = 0.0;
        self.active = true;
        self.fading = false;
    }

    /// Bulge the shell, `ShipShield_Hit` (`0x0885eb04`).
    ///
    /// A no-op on a shell that is not up, which is the original's own guard: its
    /// two callers both test the object's active flag before calling.
    pub fn hit(&mut self) {
        if !self.active {
            return;
        }
        self.swell = SWELL_ON_HIT;
    }

    /// Start the shell fading, `ShipShield_Deactivate` (`0x0885e1c0`).
    ///
    /// **The colour target goes to zero on all four channels**, which is the
    /// original's own four stores and not just an alpha ramp: the shell darkens
    /// as well as thinning. The existing [`COLOUR_RATE`] then carries it there
    /// over about half a second.
    ///
    /// The shell is **not** hidden here, and this is the one place where saying
    /// so matters. The original does clear both models' draw flags in this
    /// function - and then [`Self::advance`] sets the right one again on the
    /// very next frame, because the update re-shows whichever model the camera
    /// calls for every time it runs. So that clear is per-frame bookkeeping
    /// rather than a stop, and what actually ends the shell is the
    /// [`FADE_OUT_ALPHA`] test at the bottom of the update. Reproducing the
    /// clear here would end the shell a frame early and delete the fade.
    ///
    /// The original also writes an unresolved constant over the swell's target
    /// at the same moment. It is in the same constant pool as the three tints -
    /// see the module docs - and is left alone here for the same reason.
    pub fn deactivate(&mut self) {
        if !self.active {
            return;
        }
        self.fading = true;
        self.rgba_target = [0.0; 4];
    }

    /// One frame of `ShipShield_Update` (`0x0885e254`).
    ///
    /// Runs both lerps `(int)(dt / SUBSTEP)` times, exactly as the original
    /// does - a `dt` shorter than a substep advances neither, which is a real
    /// property of the original and not a rounding artefact to smooth over.
    pub fn advance(&mut self, dt: f32) {
        if !self.active {
            return;
        }
        let steps = (dt / SUBSTEP) as i32;
        for _ in 0..steps {
            for channel in 0..4 {
                self.rgba[channel] +=
                    (self.rgba_target[channel] - self.rgba[channel]) * COLOUR_RATE;
            }
            self.swell += (SWELL_REST - self.swell) * SWELL_REST_RATE;
        }
        self.time += dt;
        if self.fading && self.rgba[3] <= FADE_OUT_ALPHA {
            self.active = false;
            self.fading = false;
        }
    }

    /// Whether anything should be drawn this frame.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.active
    }

    /// The uniform scale the shell is drawn at, on top of the craft's own matrix.
    ///
    /// `swell + 0.012 + n * 0.012`. Near `1.024` at rest and near `1.124` on the
    /// frame a hit lands.
    #[must_use]
    pub fn scale(&self) -> f32 {
        self.swell + SCALE_BASE + flicker(self.time) * SCALE_FLICKER
    }

    /// The scale the **cockpit** sphere is drawn at: [`Self::scale`] times
    /// [`COCKPIT_SCALE`].
    ///
    /// Unreachable until this engine has an internal camera; see
    /// [`COCKPIT_SCALE`].
    #[must_use]
    pub fn cockpit_scale(&self) -> f32 {
        self.scale() * COCKPIT_SCALE
    }

    /// The colour the shell multiplies its authored vertices by, `0..=1`.
    ///
    /// Only the alpha flickers: the original modulates the packed colour's alpha
    /// byte alone and passes the other three straight through.
    #[must_use]
    pub fn colour(&self) -> [f32; 4] {
        let alpha = self.rgba[3] * (flicker(self.time) * ALPHA_FLICKER + ALPHA_FLOOR);
        [self.rgba[0], self.rgba[1], self.rgba[2], alpha]
    }
}

/// The shell's flicker, `0..=1`, as a function of its own clock.
///
/// **Ours.** The original calls one unread function with the object's
/// accumulated time and spends the result on both the alpha and the scale. What
/// is recovered is that there is a single such value, that both consumers need
/// it in `0..=1`, and that its argument is a clock that resets on activation -
/// not the curve.
///
/// This is a sum of two incommensurable sines, which gives a non-repeating
/// shimmer without a table and without a generator: a pure function of `t`, so a
/// screenshot at a given time is reproducible and two craft whose shields went
/// up together flicker together, the way one clock feeding one function must.
///
/// Not in the simulation and so not bound by
/// `docs/architecture/determinism.md` - `sin` is a platform transcendental and
/// would be a violation on the other side of the line.
#[must_use]
pub fn flicker(t: f32) -> f32 {
    let a = (t * 17.0).sin();
    let b = (t * 6.3).sin();
    (a + b).mul_add(0.25, 0.5)
}

#[cfg(test)]
mod tests;
