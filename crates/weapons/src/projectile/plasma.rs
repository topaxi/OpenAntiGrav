//! The Plasma: one bolt, out of the nose, at the class's own speed.
//!
//! It shares the flight model with [`super::rocket`]; only the count differs (one
//! where the Rocket fires three), and that is read, not assumed. See
//! `docs/ghidra/functions/psp-pulse-usa/plasma.md`. The flight stays in
//! [`super::Projectiles::advance`]: `Plasma_Update` (`0x0885c6cc`) is the Rocket's
//! floor follower instruction for instruction (12-unit probe along the carried
//! normal, speed-preserving redirect, fall, detonate on a wall).

use oag_core::math::Vec3;
use oag_tables::weapons::PlasmaStats;

use super::KMH_PER_UNIT_PER_SECOND;
use crate::Craft;

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
/// # The launch point
///
/// The craft's weapon node, [`muzzle`].
#[must_use]
pub fn launch<C: Craft>(craft: &C, stats: &PlasmaStats, class: &str) -> Option<(Vec3, Vec3)> {
    let (nose, forward) = muzzle(craft);
    let speed = (stats.speed_for_named(class)? + stats.launch_speed) / KMH_PER_UNIT_PER_SECOND;
    Some((nose, forward * speed))
}

/// Where a bolt sits on the craft, and which way it points, right now. Split from
/// [`launch`] because a charging bolt needs both every wind-up tick without
/// re-resolving a speed (see [`CHARGE_SECONDS`]).
///
/// **The hull's `Ship Muzzle` locator, through the craft's display scale
/// (confidence 85, Assegai probed; `plasma.md`, "the head's matrix and node" and
/// "the head after the release").** `Plasma_Init` seeds the bolt's own matrix from
/// the craft's weapon node and `Plasma_Launch` releases it there: read at the draw
/// hook, the bolt is 4.24 ahead of the craft's body on the last charging frame and
/// 4.43, 4.62, 4.81 ... on the frames after, with no step back, while the craft's
/// speed rose from 106 to 120 u/s. The node is the hull's authored locator
/// (`(0, 0, 5.653)` model units on Assegai, per team) carried by the craft's
/// world rows, which are [`LAUNCH_SPEED_SCALE`] long. A `.vex` hull is authored
/// nose along model `+Z` and drawn half a turn about its up axis, so model `+Z`
/// is the body's forward and model `+X` its left. A hull with no locator keeps
/// the measured hold point [`MUZZLE_FORWARD`].
#[must_use]
pub fn muzzle<C: Craft>(craft: &C) -> (Vec3, Vec3) {
    let body = &craft.physics().body;
    let forward = body.forward();
    let offset = match craft.muzzle() {
        Some(local) => {
            let scaled = local * LAUNCH_SPEED_SCALE;
            forward * scaled.z + body.up() * scaled.y - body.right() * scaled.x
        }
        None => forward * MUZZLE_FORWARD,
    };
    (body.position + offset, forward)
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

/// How far ahead of the craft's body a bolt is held and released on a hull with
/// no `Ship Muzzle` locator (HD's hulls).
///
/// **Measured 2026-10-31, confidence 75, Assegai Venom only, at one hook.**
/// `--probe plasma_fly` records the firing craft beside the bolt at the entry of
/// `Plasma_Update`: there the bolt, not yet moved this frame, reads 2.23 units
/// ahead of a craft that has (three boots agree to 0.05). At one instant the
/// release point is the locator itself, 4.24 ahead on Assegai ([`muzzle`]); the
/// 2.23 is that minus one craft step at 120 u/s, kept only as the stand-in for a
/// hull that authors no node. This engine once pushed the bolt out by the hull's
/// extent, 4.87 on that craft.
pub const MUZZLE_FORWARD: f32 = 2.23;

/// How much slower than its release speed a bolt leaves, until its first floor ride.
///
/// **Measured 2026-10-30, confidence 85** (`--probe plasma_fly`, Venom, Talon's
/// Junction, Time Trial): released at `launch_kmh` 634.61, the first four records read
/// 132.2 units/s = 475.9 km/h = 0.75 x that. It is the Rocket's law: the weapon-node
/// matrix `Plasma_Launch` (`0x0885bf84`) takes carries the craft's display scale, so
/// the direction row is 0.75 long. Not backed out of the matrix (the 16 floats are
/// unread); the scale is [`super::rocket::LAUNCH_SPEED_SCALE`], same global.
pub const LAUNCH_SPEED_SCALE: f32 = super::rocket::LAUNCH_SPEED_SCALE;

/// How far above the floor a Plasma bolt rides: `g_ride_height` (`_DAT_002acf08`).
///
/// **Measured 2026-10-31, confidence 80, not read from Pulse's constant** (its value was
/// never sampled; `plasma.md`). HD's and Omega's own `Plasma_Update` read a literal 4.0
/// (`ps3-hdfury-eu/plasma.md`, the TOC constant `0x008a9fd8`), which agrees. `--probe plasma_fly` on the original puts the bolt's
/// riding position 4.04 units along the floor normal above the floor this engine's
/// collision world finds under it (five records, 4.04 to 4.05), where the Rocket's
/// own literal is `3.0` ([`super::RIDE_HEIGHT`]). On a banked track the extra unit is
/// 0.2 units sideways, which decides where a bolt meets the wall beside it. Missile and
/// Shuriken read the same global and keep `3.0` until each is measured.
pub const RIDE_HEIGHT: f32 = 4.0;

#[cfg(test)]
mod tests;
