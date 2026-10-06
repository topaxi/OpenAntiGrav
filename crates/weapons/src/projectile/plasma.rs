//! The Plasma: one bolt, out of the nose, at the class's own speed.
//!
//! It shares the flight model with [`super::rocket`]; only the count differs (one
//! where the Rocket fires three), and that is read, not assumed. See
//! `docs/ghidra/functions/psp-pulse-usa/plasma.md`. The flight stays in
//! [`super::Projectiles::advance`]: `Plasma_Update` (`0x0885c6cc`) is the Rocket's
//! floor follower instruction for instruction (12-unit probe along the carried
//! normal, speed-preserving redirect, fall, detonate on a wall).

use oag_core::math::Vec3;
use oag_physics::ShipState;
use oag_physics::params::Dimensions;
use oag_tables::weapons::PlasmaStats;

use super::KMH_PER_UNIT_PER_SECOND;

/// Where a craft launches its plasma bolt from, and the speed it holds while
/// charging.
///
/// **The returned speed is not the flight speed**: it is the magnitude
/// [`super::Projectiles::charge_up`] reseats each wind-up tick, a hold-time visual.
/// When the charge ends, [`super::Projectiles::advance`] replaces direction and
/// magnitude with values read at release (the craft's speed then plus
/// `launchSpeed`, blending to the class speed over the first second), per
/// `Plasma_Launch` (`0x0885bf84`). See [`super::missile::SPEED_RAMP_SECONDS`] and
/// `Plasma_SpeedForClass` (`0x0885c5a4`, `plasma.md`, "the launch ramp"). Speeds
/// are km/h, for the Rocket's measured reason.
///
/// # One shot, read twice
///
/// **The handler.** `Weapon_FirePlasma` (`0x0886a868`), the bit-`0x4` handler in
/// `Weapons_DispatchFire`, takes one slot of a 16-entry pool, calls `Plasma_Init`
/// once and clears its request bit: no three spawn calls (Rocket), no reload
/// timer and round counter (Mine). **The schema.** The `<Stats>` carries no
/// `spread`, the same absence as on the one-shot Missile.
///
/// # Shared with the Rocket, so equally ours
///
/// Only the launch offset: the origin pushed forward by the hull's extent
/// ([`super::launch`]'s choice, so the two cannot drift).
#[must_use]
pub fn launch(
    state: &ShipState,
    dimensions: &Dimensions,
    stats: &PlasmaStats,
    class: &str,
) -> Option<(Vec3, Vec3)> {
    let (nose, forward) = muzzle(state, dimensions);
    let speed = (stats.speed_for_named(class)? + stats.launch_speed) / KMH_PER_UNIT_PER_SECOND;
    Some((nose, forward * speed))
}

/// Where a bolt sits on the craft, and which way it points, right now. Split from
/// [`launch`] because a charging bolt needs both every wind-up tick without
/// re-resolving a speed (see [`CHARGE_SECONDS`]). The offset is [`super::launch`]'s.
#[must_use]
pub fn muzzle(state: &ShipState, dimensions: &Dimensions) -> (Vec3, Vec3) {
    let forward = state.body.forward();
    let nose = state.body.position
        + forward * oag_physics::wall::hull_extent(&state.body, dimensions, forward);
    (nose, forward)
}

/// How long a plasma bolt winds up on the nose before it flies, in seconds.
///
/// **Recovered, confidence 90, a literal.** `Plasma_Init` (`0x0885bd18`) marks the
/// entry charging and seeds its countdown:
///
/// ```text
/// *(undefined1 *)(entity + 0x4c) = 1;              // charging
/// *(undefined4 *)(entity + 0x50) = _DAT_08a7c098;  // 0x3f800000 == 1.0f
/// ```
///
/// `0x08a7c098` was read from `.rodata` as `0x3f800000`. `Plasmas_Update`
/// (`0x0886b490`) calls `Plasma_UpdateCharge` (`0x0885c170`) for a charging entity
/// (copies the craft's weapon-node matrix onto the bolt), subtracts `dt` from
/// `+0x50`, and calls `Plasma_Launch` (`0x0885bf84`) at zero, which clears `+0x4c`
/// and builds the velocity from the current forward.
///
/// **Wipeout Pure hard-codes the same second**: `Plasma_Init` there is
/// `FUN_0885df98`, an immediate `0x3f800000`, allocation tag
/// `c:/Work/Wipeout/Code/Backend/Weapons/Plasma.cpp` line 63.
///
/// # This is not `<Plasma charge_time>`
///
/// The file authors `charge_time="3"` (Pulse's `WeaponStats_Race.xml` and
/// `WeaponStats_Elimination.xml`, Pure's `weaponstats.xml`; the only weapon to do
/// so) and **nothing reads it in either executable**. `WeaponStats_ParsePlasma`
/// (`0x0880cc2c`) stores it at `stats+0x9c`, never loaded again: across all 69
/// functions in `psp-pulse-usa` reaching the weapon-stats table
/// (`*(int *)(&DAT_08b32420 + DAT_08b32428 * 4)`), every load and store at `+0x9c`
/// finds zero, while the same sweep at the `venomspeed` offset `+0xac` finds
/// fourteen including `Plasma_SpeedForClass`. The wind-up is three times shorter
/// than the attribute suggests. See `plasma.md`.
pub const CHARGE_SECONDS: f32 = 1.0;

#[cfg(test)]
mod tests;
