//! What a weapon hit costs a craft in speed: the timer, its clamp, its effects.
//!
//! A craft hit by a mine, rocket or missile slows down. The whole mechanic is one `f32`,
//! [`ShipState::slowdown_timer`] (the original's `craft+0x2e0`), and this module is the single
//! function that raises it. Every claim below comes from the disassembly in
//! `docs/ghidra/functions/psp-pulse-usa/engine.md`, "The slowdown mechanic, recovered end to
//! end".
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
//! **This crate owns the timer and the four effects, not the pending slot**: that is
//! `entity+0x130`, credited from a weapon's `slowdown_time` (`<WeaponStats>`), and `oag-physics`
//! depends on `oag-core` only, so it lives on `oag_gameplay::world::Ship` beside the pickup and
//! `oag_weapons::slowdown` runs the drain that calls [`add`].
//!
//! Three effects predate the mechanic's identification, under the name "leap timer":
//! [`crate::engine::engine`]'s early return, [`crate::forces::evaluate`]'s lateral-grip skip and
//! [`crate::hover::target_height`]'s subtraction. They did not change; something now arms the
//! field.
//!
//! # Confidence, and what has never been watched run
//!
//! 85 to 92 per claim, **all static**: nothing has been observed in an emulator with a craft
//! taking a hit. A PPSSPP capture of `craft+0x2e0` through a real impact would confirm the decay
//! rate and saturation in one run (`scripts/psp-trace.py` already carries the field). Until
//! then this is a careful reading of fifteen instructions and their callers.

use crate::ship::ShipState;

/// `Ship_AddSlowdown` (`0x08848690`): add seconds of slowdown, clamped to the authored ceiling.
///
/// Fifteen instructions, one clamp branch, one caller: the drain in `FUN_088418e0` at
/// `0x08842104`. Confidence 90; Ghidra defines no function there, so it was read from the
/// disassembly.
/// ```text
/// craft->timer_2e0 += seconds;
/// float limit = ActiveWeaponStats()->slowdown_limit;   // block + 0x00
/// if (craft->timer_2e0 > limit) craft->timer_2e0 = limit;
/// ```
///
/// # `limit` is a ceiling on seconds outstanding
///
/// Not a speed floor, not a cap on what one weapon may take off, and not a maximum duration from
/// the first hit. Sustained fire never slows a craft for more than `limit` *at any one moment*,
/// but every impact refills the timer to that ceiling: a Plasma, whose `slowdown_time` equals
/// the global cap on both shipped tables, saturates the mechanic alone and a second hit inside
/// the window adds nothing. `limit` is `<Weapon type="Global"><Stats slowdown_limit/>`
/// (`oag_tables::weapons::WeaponStats::slowdown_limit`), a parameter because the player's disc
/// says what it is.
///
/// # The clamp is one-sided
///
/// The original clamps above and never below, so a negative `seconds` would lower the timer.
/// Reproduced, not guarded: no caller passes one (`slowdown_time` is positive on all four
/// shipped tables) and a `max(0.0)` would invent a rule. The *timer* slightly negative on entry
/// is normal: [`crate::forces::evaluate`] leaves it one `dt` below zero on the tick it expires,
/// as the original does, and that residue is subtracted from the next hit's slowdown, which is
/// why the decrement is not clamped.
pub fn add(state: &mut ShipState, seconds: f32, limit: f32) {
    state.slowdown_timer += seconds;
    if state.slowdown_timer > limit {
        state.slowdown_timer = limit;
    }
}

#[cfg(test)]
mod tests;
