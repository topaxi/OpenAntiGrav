//! Wipeout HD's weapon-absorb shell: each team's `AbsorbEffect.vex` drawn over
//! the hull, faded in while an absorb's one-second timer runs and out after.
//!
//! This is HD's counterpart to Pulse's [`crate::hull_overlay`], and it shares
//! nothing with it but the trigger. Pulse projects a texture over the hull's
//! own batches; HD authors a separate shell model per team and draws that. The
//! evidence is on `docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md`,
//! "The absorb shell"; this module implements that reading and cites it.
//!
//! # What the original does
//!
//! - `ShipAbsorbShell_Load` (`0x000dba30`) loads two scene nodes per craft and
//!   links them under the craft: `<team dir>\AbsorbEffect.vex` at
//!   `craft+0x7a44`, and `Data\Weapons\vr_absorbinternal_cockpit.vex` at
//!   `craft+0x7a40`. Both start hidden, and both have their material parameter
//!   `ShieldColour` (`0xaaf53119` = `~crc32("ShieldColour")`) bound to the
//!   float at `craft+0x7a50`.
//! - `Ship_PlayAbsorbFeedback` (`0x000d9398`) stores `1.0` into
//!   `craft+0x7a5c` on every call - [`TIMER`].
//! - Each tick, the craft update (inlined in `FUN_000eadb8` at
//!   `0x000ebff0..0x000ec05c`; the stand-alone copy is `ShipAbsorbShell_Step`,
//!   `0x000cef50`) relaxes `+0x7a50` toward `+0x7a54` by `+0x7a58` of the gap,
//!   `trunc(dt * 59.999996)` times, then subtracts `dt` from the timer.
//! - While the timer is positive, the target is [`PEAK`] and the rate
//!   [`RATE`]; after, the target is `0`. The shell is shown while the timer is
//!   positive or the fade is above [`HIDE_BELOW`], and hidden after that
//!   (`FUN_000e41b0` and `ShipAbsorbShell_Show`, `0x000db6f8`).
//!
//! The material, `hd_absorbinternal.rcsmaterial`, multiplies the shell's own
//! `VertexColour1` alpha by `ShieldColour` in its vertex program and draws
//! additively (`SrcAlpha`/`One`). So [`AbsorbShell::fader`] is the value the
//! vertex alpha is scaled by, and nothing else about the picture moves with it.
//!
//! # What is ours
//!
//! - **The order inside one tick**: here the stamp, the target, the relaxation
//!   and the timer's decrement all happen in the same tick, in that order. In
//!   the original the target is chosen in a later stage of the frame than the
//!   relaxation, so this may lead it by one tick. Chosen, not measured.
//! - **The cockpit shell is not drawn.** Its local transform is the identity
//!   with a `7.0` in a lane selected by a mask in `.bss` (`*(0x008a8c20) +
//!   0x20`), which no static read gives. See the doc page.

/// What `Ship_PlayAbsorbFeedback` stores into the timer, `settings+0x58` at
/// `0x008c1638`. Seconds.
pub const TIMER: f32 = 1.0;

/// The fade's target while the timer runs, `settings+0x6c` at `0x008c164c`.
pub const PEAK: f32 = 1.0;

/// The share of the gap to the target closed per step, `DAT_008a8abc`.
pub const RATE: f32 = 0.1;

/// The fade at or below which a shell whose timer has run out is hidden,
/// `DAT_008a8afc`.
pub const HIDE_BELOW: f32 = 0.01;

/// Relaxation steps per second of `dt`, `DAT_008a8ab8` (`0x426fffff`). The
/// step count is this times `dt`, truncated toward zero by `fctiwz`.
pub const STEPS_PER_SECOND: f32 = f32::from_bits(0x426f_ffff);

/// `~crc32("ShieldColour")`, the one parameter `hd_absorbinternal` declares.
pub const SHIELD_COLOUR: u32 = 0xaaf5_3119;

/// One craft's absorb shell fade: `craft+0x7a50..+0x7a5c`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AbsorbShell {
    /// `+0x7a50`, the value `ShieldColour` reads.
    fade: f32,
    /// `+0x7a5c`, seconds left of the absorb.
    timer: f32,
}

impl AbsorbShell {
    /// `Ship_PlayAbsorbFeedback`'s store: the timer restarts at [`TIMER`].
    /// The fade carries on from wherever it is.
    pub fn stamp(&mut self) {
        self.timer = TIMER;
    }

    /// One tick of `dt` seconds. See the module doc for the order.
    pub fn advance(&mut self, dt: f32) {
        let target = if self.timer > 0.0 { PEAK } else { 0.0 };
        for _ in 0..steps(dt) {
            self.fade += (target - self.fade) * RATE;
        }
        self.timer -= dt;
    }

    /// The `ShieldColour` the shell draws with this tick, or `None` while the
    /// original hides it.
    #[must_use]
    pub fn fader(&self) -> Option<f32> {
        (self.timer > 0.0 || self.fade > HIDE_BELOW).then_some(self.fade)
    }
}

/// How many relaxation steps a tick of `dt` seconds takes.
///
/// One at the port's fixed 60 Hz. The constant sits just under 60 so that a
/// frame a hair short of 1/60 s still rounds to one step in `f32`; the product
/// at exactly `1.0 / 60.0` rounds up to `1.0`.
#[must_use]
pub fn steps(dt: f32) -> u32 {
    let product = dt * STEPS_PER_SECOND;
    if product > 0.0 { product as u32 } else { 0 }
}

#[cfg(test)]
mod tests;
