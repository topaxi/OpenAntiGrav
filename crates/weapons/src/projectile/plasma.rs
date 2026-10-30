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
/// # The launch point
///
/// [`MUZZLE_FORWARD`] ahead of the craft's body (measured, Assegai Venom).
#[must_use]
pub fn launch(state: &ShipState, stats: &PlasmaStats, class: &str) -> Option<(Vec3, Vec3)> {
    let (nose, forward) = muzzle(state);
    let speed = (stats.speed_for_named(class)? + stats.launch_speed) / KMH_PER_UNIT_PER_SECOND;
    Some((nose, forward * speed))
}

/// Where a bolt sits on the craft, and which way it points, right now. Split from
/// [`launch`] because a charging bolt needs both every wind-up tick without
/// re-resolving a speed (see [`CHARGE_SECONDS`]). The offset is [`MUZZLE_FORWARD`].
#[must_use]
pub fn muzzle(state: &ShipState) -> (Vec3, Vec3) {
    let forward = state.body.forward();
    (state.body.position + forward * MUZZLE_FORWARD, forward)
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

/// How far ahead of the craft's body the weapon node sits, which is where a bolt is
/// held while it charges and where it leaves from.
///
/// **Measured 2026-10-31, confidence 75, Assegai Venom only.** `--probe plasma_fly`
/// now records the firing craft beside the bolt: at release the craft is at
/// `(239.54, -42.22, -196.91)` and the bolt at `(241.77, -42.11, -196.86)`, 2.23 units
/// ahead along the heading (three boots agree to 0.05). This engine pushed the bolt out
/// by the hull's extent, 4.87 on that craft, the Rocket's choice before it was measured
/// to leave from the craft's own position. Other crafts and classes are unmeasured: the
/// node is a point in the model, so 2.23 is probably per craft; read as a constant
/// until each is probed. 3.0 x the display scale 0.75 is 2.25, a coincidence not a
/// finding.
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
/// **Measured 2026-10-31, confidence 70, not read from the constant** (its value was
/// never sampled; `plasma.md`). `--probe plasma_fly` on the original puts the bolt's
/// riding position 4.04 units along the floor normal above the floor this engine's
/// collision world finds under it (five records, 4.04 to 4.05), where the Rocket's
/// own literal is `3.0` ([`super::RIDE_HEIGHT`]). On a banked track the extra unit is
/// 0.2 units sideways, which decides where a bolt meets the wall beside it. Missile and
/// Shuriken read the same global and keep `3.0` until each is measured.
pub const RIDE_HEIGHT: f32 = 4.0;

/// How long after release a bolt does not ride the floor, in seconds.
///
/// **Chosen inside a measured bracket, not recovered.** The original's first three
/// `Plasma_Update`s fall (`velocity -= surface * dt * 50`: y drops 0.82 a tick) and keep
/// the 0.75 launch speed; the fourth finds the floor and pins to the ramp (records at
/// entry age 0, 0.0167, 0.0334 slow, 0.0501 the first ride). The Rocket shows the same
/// four updates at a different place on the track, so this reads as age, not position.
/// What gates it (the probe's reach, a collision kind, the bounds the query takes) is
/// unread; the Rocket thread has the same open question. 0.04 is between the last slow
/// age (0.0334) and the first riding one (0.0501), so a tick of rounding cannot move it.
pub const UNPINNED_SECONDS: f32 = 0.04;

/// How many updates that is at the fixed 60 Hz step: ages 0, 1/60 and 2/60 are below
/// [`UNPINNED_SECONDS`], 3/60 is not.
#[cfg(test)]
const UNPINNED_UPDATES: usize = 3;

#[cfg(test)]
mod tests;
