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
//! shell from an external camera, the sphere from the cockpit, on `craft+0x6d`
//! and hiding whichever it did not draw - at a scale that
//! swells on impact and settles, with an alpha that breathes. `ShipShield_Hit`
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
//! **Recovered**, from instruction immediates: the two lerp rates
//! ([`COLOUR_RATE`], [`SWELL_RATE`]), the swell's rest and hit values
//! ([`SWELL_REST`], [`SWELL_ON_HIT`]), the `0.012` base and flicker of the scale
//! ([`SCALE_BASE`], [`SCALE_FLICKER`]), the alpha flicker's `0.25`/`0.75` split
//! ([`ALPHA_FLICKER`], [`ALPHA_MID`]), the cockpit sphere's [`COCKPIT_SCALE`],
//! the fixed 60 Hz substep, and the `0.1` alpha at which a fading shell stops
//! being drawn ([`FADE_OUT_ALPHA`]); and from `.data`, the swell an activation
//! starts at and the one a fade expands to ([`SWELL_ON_ACTIVATE`],
//! [`SWELL_ON_FADE`]).
//!
//! **Also recovered, and this one nearly was not.** The flicker's curve is
//! `sinf` of the shell's own clock in **seconds** - see [`flicker`]. The call
//! reads as an unresolved `func_0x0017a300` in the decompiled update, and the
//! first pass here shipped an invented shimmer in its place; decompiling the
//! target settles it as the C library's `sinf`, so the shell breathes on a
//! `2*pi`-second cycle rather than flickering. The two are not subtly different
//! and the invented one was wrong.
//!
//! **And so are the colours**, as of the second pass. They looked unreachable
//! because they live in `.bss` at addends a `lui`/`addiu` pair reaches through
//! the PRX relocation table's `ADDR_BASE = 1`, which Ghidra applies to nothing -
//! so the obvious reading lands inside `.text` and reads as "no such constant".
//! Their one writer is a `.cplinit` static initialiser that no `jal` targets, so
//! it looks dead until you find it in the init list. See
//! [`ACTIVATION_COLOUR`], [`TARGET_COLOUR`] and [`HIT_COLOUR`], and the docs
//! page for how each was pinned.
//!
//! **Nothing here is ours any more.** What a player sees is the authored shell
//! fading up from nothing and growing from [`SWELL_ON_ACTIVATE`], flashing cyan
//! and bulging on every absorbed hit, then fading out while it expands.

/// How far the colour moves toward its target per 60 Hz substep.
///
/// `ShipShield_Activate` writes `0.15` to the object's `+0x60` and
/// `ShipShield_Update` multiplies the colour delta by it once per substep.
pub const COLOUR_RATE: f32 = 0.15;

/// How far the swell moves toward its target per 60 Hz substep.
///
/// `+0x6c`, `0.2`. About a fifth of a second to settle a hit, which is what
/// makes the bulge read as an impact rather than a pulse.
pub const SWELL_RATE: f32 = 0.2;

/// What the swell settles to while the shield is up. `+0x68`, `1.0`.
pub const SWELL_REST: f32 = 1.0;

/// What `ShipShield_Activate` starts the swell at. `0.7`, from `.data`.
///
/// **Below rest, so a raised shield grows into place** rather than appearing at
/// full size - which is the half of the activation the colour fade does not
/// carry. Read from `.data` (`0x08ab0f18`) rather than an immediate, because
/// this one is an initialised global.
pub const SWELL_ON_ACTIVATE: f32 = 0.7;

/// What `ShipShield_Deactivate` retargets the swell to. `1.2`, from `.data`.
///
/// **Above rest**: an expiring shell blows outward as it fades, rather than
/// shrinking away. `0x08ab0f1c`, the word after [`SWELL_ON_ACTIVATE`], which is
/// how the pair reads as one authored decision.
pub const SWELL_ON_FADE: f32 = 1.2;

/// What an absorbed hit sets the swell to. `ShipShield_Hit` stores `1.1`.
///
/// A tenth over rest, pulled back at [`SWELL_RATE`]: the shell jumps out by
/// 10 % of its size and shrinks back. This is the whole of the impact response,
/// and it is why [`crate::shield::ShipShield::hit`] has to be called from the
/// tick that absorbed rather than inferred from the timer.
pub const SWELL_ON_HIT: f32 = 1.1;

/// Added to the swell to get the drawn scale. `0.012`.
pub const SCALE_BASE: f32 = 0.012;

/// How much of the drawn scale the flicker moves. `0.012`, the same magnitude
/// as [`SCALE_BASE`], so the shell breathes by about a percent.
pub const SCALE_FLICKER: f32 = 0.012;

/// The flicker's share of the alpha: `alpha * (sin(t) * 0.25 + 0.75)`.
///
/// `sin` runs `-1..=1`, so the shell breathes between **half** and **all** of
/// its colour's alpha - not between three quarters and all, which is what this
/// constant reads like on its own and what the first pass here assumed while it
/// had the flicker's range wrong.
pub const ALPHA_FLICKER: f32 = 0.25;

/// The midpoint the alpha flicker swings about, `1.0 - ALPHA_FLICKER`.
pub const ALPHA_MID: f32 = 1.0 - ALPHA_FLICKER;

/// The dimmest the alpha flicker goes, as a fraction of the colour's own alpha.
///
/// `ALPHA_MID - ALPHA_FLICKER`, i.e. a half, at `sin(t) == -1`.
pub const ALPHA_FLOOR: f32 = ALPHA_MID - ALPHA_FLICKER;

/// What the cockpit sphere's scale is multiplied by. `1.8`.
///
/// **Not a decoration on the number**: `vr_shield_cockpit.vex` is authored at
/// radius `3.99` against a hull of about `6.97`, so at `1.8` it reaches `7.2`
/// and clears the hull the camera is sitting inside. Authored smaller and grown
/// past, which is what the constant is *for*.
pub const COCKPIT_SCALE: f32 = 1.8;

/// Alpha at or below which a **fading** shell stops being drawn at all. `0.1`.
///
/// Only consulted while fading: an active shield whose colour happens to be dim
/// keeps drawing.
pub const FADE_OUT_ALPHA: f32 = 0.1;

/// The colour `ShipShield_Activate` starts the shell at: transparent black.
///
/// `0x08b3bf60`, written `(0, 0, 0, 0)` by the `.cplinit` initialiser at
/// `0x0885eb54`. With [`TARGET_COLOUR`] as the destination, an activation is a
/// **fade up from nothing** - the shell does not pop in.
pub const ACTIVATION_COLOUR: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

/// The colour the shell settles to while the shield is up: full white.
///
/// `0x08b3bf40`, `(1, 1, 1, 1)`. White is the identity for the multiply the draw
/// does against the model's own authored vertex colours, so a settled shell is
/// exactly what the artists painted - `(0.55, 0.50, 0.91, 0.50)` on this mesh's
/// main batch, a blue-violet at half alpha.
pub const TARGET_COLOUR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// The colour an absorbed hit flashes the shell to: **cyan**.
///
/// `0x08b3bf50`, `(0, 1, 1, 1)`. The red channel alone goes to zero, so against
/// the authored blue-violet the flash reads as a hue shift to cyan rather than
/// as a brightening - the alpha is untouched. It then lerps back to
/// [`TARGET_COLOUR`] at [`COLOUR_RATE`], about a quarter of a second.
///
/// This is the pair with [`SWELL_ON_HIT`]: colour *and* size move together, and
/// either alone is a weaker read of the same event.
pub const HIT_COLOUR: [f32; 4] = [0.0, 1.0, 1.0, 1.0];

/// What `ShipShield_Deactivate` retargets the colour to: transparent black.
///
/// The same value [`ACTIVATION_COLOUR`] holds, and written by the same
/// initialiser into a fourth slot at `0x08b3bf70` that nothing reads - the
/// deactivate stores four literal zeros itself. Named here because the fade-out
/// is a colour target like any other, not a special case.
pub const FADE_COLOUR: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

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
    /// What the colour approaches at [`COLOUR_RATE`]: [`TARGET_COLOUR`] while
    /// the shield is up, [`FADE_COLOUR`] once it is expiring.
    rgba_target: [f32; 4],
    /// The impact swell, resting at [`SWELL_REST`].
    swell: f32,
    /// What the swell approaches at [`SWELL_RATE`]: [`SWELL_REST`] while the
    /// shield is up, [`SWELL_ON_FADE`] once it is expiring.
    ///
    /// **State rather than a constant**, because the deactivate moves it - which
    /// is what makes an expiring shell blow outward instead of settling.
    swell_target: f32,
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
    /// A shield that is not up.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rgba: ACTIVATION_COLOUR,
            rgba_target: TARGET_COLOUR,
            swell: SWELL_REST,
            swell_target: SWELL_REST,
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
        self.rgba = ACTIVATION_COLOUR;
        self.rgba_target = TARGET_COLOUR;
        self.swell = SWELL_ON_ACTIVATE;
        self.swell_target = SWELL_REST;
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
        self.rgba = HIT_COLOUR;
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
    /// It also retargets the swell to [`SWELL_ON_FADE`], above rest: the shell
    /// blows outward as it thins rather than shrinking away.
    pub fn deactivate(&mut self) {
        if !self.active {
            return;
        }
        self.fading = true;
        self.rgba_target = FADE_COLOUR;
        self.swell_target = SWELL_ON_FADE;
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
            self.swell += (self.swell_target - self.swell) * SWELL_RATE;
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
        let alpha = self.rgba[3] * (flicker(self.time) * ALPHA_FLICKER + ALPHA_MID);
        [self.rgba[0], self.rgba[1], self.rgba[2], alpha]
    }
}

/// The shell's flicker, `-1..=1`: **`sin` of its own clock, in seconds**.
///
/// Recovered. `ShipShield_Update` calls one function with the object's
/// accumulated time and spends the result on both the alpha and the scale, and
/// that function - printed as an unresolved `func_0x0017a300` because this
/// image relocates no call targets - decompiles as the C library's `sinf`
/// (`0x0897e300`).
///
/// So the argument is in **radians and the clock is in seconds**, giving a
/// `2*pi`-second period: the shell breathes about once every six and a third
/// seconds, over a whole shield's typical life. That matters because it is not
/// what "flicker" suggests, and the first version of this function was an
/// invented ~2.7 Hz shimmer that read as a completely different effect. Named
/// `flicker` still, because that is what the two consumers use it for.
///
/// A pure function of `t`, so a screenshot at a given time is reproducible and
/// two craft whose shields went up together breathe together - which they must,
/// one clock feeding one function.
///
/// Not in the simulation and so not bound by
/// `docs/architecture/determinism.md`; `sin` is a platform transcendental and
/// would be a violation on the other side of the line.
#[must_use]
pub fn flicker(t: f32) -> f32 {
    t.sin()
}

#[cfg(test)]
mod tests;
