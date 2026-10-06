//! The pending slot's one consumer: what turns a weapon hit into a slow craft.
//!
//! `oag_physics::slowdown` holds the timer, clamp and four effects and the whole
//! law. This is the half that reads a figure out of `<WeaponStats>`, which
//! `oag-physics` (depending on `oag-core` only) cannot. The original's only reader
//! of `entity+0x130` (nine writers) is `FUN_088418e0` at
//! `0x088420cc`-`0x08842110`:
//!
//! ```c
//! if (T != NULL && T->pending_0x130 > 0.0f) {
//!     if ((T->flags_0x1b8 & 0x10) == 0)        /* the pickup/shield word */
//!         Ship_AddSlowdown(T->pending_0x130, craft);
//!     T->pending_0x130 = 0.0f;
//! }
//! ```
//!
//! Confidence 88 for the drain, 85 for the shield gate; see
//! `docs/ghidra/functions/psp-pulse-usa/engine.md`, "The pending slot and its one
//! consumer".

use crate::Craft;

/// Drains every craft's pending slowdown into its timer, once per tick.
///
/// Reproduced exactly, each of which a port gets wrong by default:
///
/// 1. **The shield gate discards rather than defers**: a craft under a fired
///    Shield takes no slowdown and has its pending slot cleared, so a hit one
///    tick before the shield expires is lost.
/// 2. **It is `entity+0x1b8 & 0x10`**, the pickup/shield word, not the dynamics
///    flag word at `craft+0x1c0` that the engine's early return tests: two
///    `& 0x10`s that must not be merged.
///    `oag_physics::ShipState::shield_pickup_timer` is the reading of the first,
///    which `oag_physics::damage` also gates on.
/// 3. **Credit and drain are different steps of the tick**: a blast lands after
///    the craft is stepped, so its slot is drained at the top of the next tick and
///    the timer is armed before that tick's engine reads it. A hit costs its full
///    first tick (recovered ordering).
///
/// `limit` is `<Weapon type="Global"><Stats slowdown_limit>`, the ceiling for
/// `oag_physics::slowdown::add`. `None` (table did not load) does nothing, safe as
/// `projectile::blast_stats` credits nothing without a table.
///
/// Slot order, as everywhere in `oag-gameplay`; each craft touches only its own
/// fields, so this is about determinism, not physics. `ships` is the occupied part
/// of the field.
pub fn drain<S: Craft>(ships: &mut [S], limit: Option<f32>) {
    let Some(limit) = limit else {
        return;
    };
    for ship in ships {
        if !ship.active() || ship.pending_slowdown() <= 0.0 {
            continue;
        }
        let pending = ship.pending_slowdown();
        // The gate is on the fired Shield pickup, not the energy pool: a craft
        // with a full shield bar still slows down.
        if ship.physics().shield_pickup_timer <= 0.0 {
            oag_physics::slowdown::add(ship.physics_mut(), pending, limit);
        }
        // Unconditional, on both arms of the gate above.
        *ship.pending_slowdown_mut() = 0.0;
    }
}

#[cfg(test)]
mod tests;
