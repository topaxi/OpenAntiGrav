//! What a weapon hit costs a craft in speed: the timer, its clamp, its effects.
//!
//! A craft hit by a mine, a rocket or a missile slows down. The whole mechanic
//! is one `f32` on the craft - [`ShipState::slowdown_timer`], the original's
//! `craft+0x2e0` - and this module is the single function that raises it. See
//! `docs/ghidra/functions/psp-pulse-usa/engine.md`, "The slowdown mechanic,
//! recovered end to end", for the disassembly every claim below comes from.
//!
//! # The law
//!
//! ```text
//! weapon impact    victim.pending_slowdown += weapon.slowdown_time
//!    |
//! once per tick    if pending > 0 {
//!    |                 if !shielded { add(timer, pending, slowdown_limit) }
//!    |                 pending = 0
//!    |             }
//!    v
//! add(t)           timer = min(timer + t, slowdown_limit)
//!    |
//!    +--> engine   timer > 0  ->  no thrust, no lift, throttle state zeroed
//!    +--> hover    hover target -= min(timer, 4.0)
//!    +--> grip     timer > 0  ->  the lateral-grip term is skipped entirely
//!    +--> decay    timer > 0  ->  timer -= dt      (no clamp to zero)
//! ```
//!
//! **This crate owns the timer and the four effects; it does not own the
//! pending slot.** That slot is `entity+0x130` rather than a craft field, it is
//! credited from a weapon's `slowdown_time` out of `<WeaponStats>`, and
//! `oag-physics` depends on `oag-core` and nothing else - so it lives on
//! `oag_gameplay::world::Ship` beside the pickup that fired the weapon, and
//! `oag_gameplay::slowdown` runs the drain that calls [`add`].
//!
//! Three of the four effects were implemented here long before the mechanic was
//! understood, under the name "leap timer": [`crate::engine::engine`]'s early
//! return, [`crate::forces::evaluate`]'s lateral-grip skip and
//! [`crate::hover::target_height`]'s subtraction. Nothing about them changed
//! when the field was identified - what changed is that something arms it.
//!
//! # Confidence, and what has never been watched run
//!
//! 85 to 92 per claim, and **all of it static**. Nothing has been observed in
//! an emulator with a craft actually taking a hit; a PPSSPP capture of
//! `craft+0x2e0` through a real impact would confirm the decay rate and the
//! saturation behaviour in one run, and `scripts/psp-trace.py` already carries
//! the field. Until then this is a careful reading of fifteen instructions and
//! their callers, not a measurement.

use crate::ship::ShipState;

/// `Ship_AddSlowdown` (`0x08848690`): add seconds of slowdown, clamped to the
/// authored ceiling.
///
/// Fifteen instructions, no branches but the one clamp, and exactly one caller:
/// the drain in `FUN_088418e0` at `0x08842104`. Confidence 90; the function is
/// not one Ghidra defines, so it was read straight out of the disassembly.
///
/// ```text
/// craft->timer_2e0 += seconds;
/// float limit = ActiveWeaponStats()->slowdown_limit;   // block + 0x00
/// if (craft->timer_2e0 > limit) craft->timer_2e0 = limit;
/// ```
///
/// # `limit` is a ceiling on seconds outstanding
///
/// Not a speed floor, not a cap on what a single weapon may take off, and not a
/// maximum duration counted from the first hit. A craft under sustained fire is
/// never slowed for longer than `limit` *at any one moment*, but every new
/// impact refills the timer back up to that ceiling - so a Plasma, which
/// authors a `slowdown_time` equal to the global cap on both shipped tables,
/// saturates the mechanic on its own and a second one inside the window adds
/// nothing.
///
/// `limit` comes from `<Weapon type="Global"><Stats slowdown_limit/>` by way of
/// `oag_formats::weapons::WeaponStats::slowdown_limit`. It is a parameter rather
/// than a constant here for the reason this crate takes every authored number as
/// one: it is the player's disc that says what it is.
///
/// # The clamp is one-sided
///
/// The original clamps above and never below, so a negative `seconds` would
/// lower the timer and nothing stops it. Reproduced rather than guarded: no
/// caller passes one - `slowdown_time` is authored positive on all four shipped
/// tables - and a `max(0.0)` here would be this crate inventing a rule.
///
/// The *timer* being slightly negative when this is called is normal, not an
/// error: [`crate::forces::evaluate`] leaves it one `dt` below zero on the tick
/// it expires, exactly as the original does, and that residue is subtracted
/// from the next hit's worth of slowdown. It is the reason that decrement is
/// not clamped.
pub fn add(state: &mut ShipState, seconds: f32, limit: f32) {
    state.slowdown_timer += seconds;
    if state.slowdown_timer > limit {
        state.slowdown_timer = limit;
    }
}

#[cfg(test)]
mod tests;
