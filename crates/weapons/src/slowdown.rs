//! The pending slot's one consumer: what turns a weapon hit into a slow craft.
//!
//! `oag_physics::slowdown` holds the timer, the clamp and the four effects and
//! is where the whole law is written down. This module is the half of it that
//! cannot live there, because it reads a figure out of `<WeaponStats>` and
//! `oag-physics` depends on `oag-core` and nothing else.
//!
//! The original's own consumer is in `FUN_088418e0` at
//! `0x088420cc`-`0x08842110`, the craft's entity update, and it is the *only*
//! reader of `entity+0x130` against nine writers:
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
//! `docs/ghidra/functions/psp-pulse-usa/engine.md`, "The pending slot and its
//! one consumer".

use crate::Craft;

/// Drains every craft's pending slowdown into its timer, once per tick.
///
/// Three things this reproduces exactly, each of which a port gets wrong by
/// default:
///
/// 1. **The shield gate discards rather than defers.** A craft under a fired
///    Shield takes no slowdown *and* has its pending slot cleared, so a hit
///    landed one tick before the shield expires is simply lost. Zeroing on both
///    arms is the original's own shape, not a simplification.
/// 2. **It is `entity+0x1b8 & 0x10`**, the pickup/shield word, and *not* the
///    dynamics flag word at `craft+0x1c0` that the engine's early return tests.
///    Two different `& 0x10`s that must not be merged;
///    `oag_physics::ShipState::shield_pickup_timer` is this crate's reading of
///    the first, the same one `oag_physics::damage` gates on.
/// 3. **The credit and the drain are different steps of the same tick.** A
///    blast lands after the craft has been stepped, so the slot it credits is
///    drained at the top of the *next* tick and the timer is armed before that
///    tick's engine reads it. A hit therefore costs its full first tick, which
///    is the recovered ordering.
///
/// # `limit`
///
/// `<Weapon type="Global"><Stats slowdown_limit>`, passed straight to
/// `oag_physics::slowdown::add` as its ceiling. `None` for a race whose weapon
/// table did not load, in which case this does nothing at all - the same
/// "gated on the table" rule the pickup grant and the blast both follow, and
/// safe for the same reason: nothing can have credited the slot either, because
/// `projectile::blast_stats` returns `None` without a table.
///
/// # Order
///
/// Slot order, which is fixed and is the same order everything else in
/// `oag-gameplay` walks the array in. Nothing here is order-dependent - each
/// craft reads and writes only its own two fields - so this is a statement
/// about determinism rather than about physics.
///
/// `ships` is the occupied part of the field, which is what `World::ship_count`
/// slices.
pub fn drain<S: Craft>(ships: &mut [S], limit: Option<f32>) {
    let Some(limit) = limit else {
        return;
    };
    for ship in ships {
        if !ship.active() || ship.pending_slowdown() <= 0.0 {
            continue;
        }
        let pending = ship.pending_slowdown();
        // The gate is on the *fired Shield pickup*, not on the energy pool: a
        // craft with a full shield bar still slows down.
        if ship.physics().shield_pickup_timer <= 0.0 {
            oag_physics::slowdown::add(ship.physics_mut(), pending, limit);
        }
        // Unconditional, on both arms of the gate above.
        *ship.pending_slowdown_mut() = 0.0;
    }
}

#[cfg(test)]
mod tests;
