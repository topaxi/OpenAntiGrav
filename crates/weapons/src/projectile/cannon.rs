//! The Cannon: a round that fires itself, and hurts only what it hits.
//!
//! When a round leaves is [`crate::pickup::Held::advance_cannon_reload`]'s
//! (per-craft state across ticks, like a mine drop); the flight is
//! [`super::Projectiles::advance`]'s shared floor-follower. The original's
//! `Cannon_UpdateRound` (`0x0886593c`, EU `0x08865798`) is read
//! (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "What draws a
//! Cannon round" and the 2026-09-09 hit-path addition), but its own raycast
//! against the collision mesh is not reproduced instruction for instruction: a
//! round rides the track and detonates on a wall or craft as the Rocket, Plasma
//! and Shuriken do. This weapon's own parts are [`launch`] and [`direct_hit`].
//!
//! # The wall hit spawns a spark effect; the craft hit does not
//!
//! **Recovered, confidence 85.** `Cannon_UpdateRound` raycasts previous to new
//! position each tick (`FUN_0883198c`, shared with the other weapons and
//! `Camera_UpdatePlayerView`) and spawns `Data\Psys\WO_CANNON_SPARKS.POB`, at the
//! hit point oriented to the hit basis, only for hit-type `0` or `4` (world/track).
//! A craft hit is a separate test after it in `CannonPool_Update` (`0x088582b0`,
//! EU `0x0885813c`): a per-craft cylinder sweep (`FUN_088579a8`) that within `6.0`
//! units calls `FUN_08857f2c`, which ORs the round's `+0x3c` flags with `0x24`
//! (general-hit and *ship* bits; the ship bit picks the `CANNONEXPLSHIP` cue over
//! `CANNONEXPLWALL`) and, if no hit was recorded, calls `FUN_08857e90`, the damage
//! handler [`direct_hit`] is confirmed against. None of the three calls
//! `Psys_Spawn_q`: a craft-hitting round plays a sound and applies damage but
//! throws no spark. The struck hull sparks from its side: `Ship_Damage`'s weapon
//! branch throws `WO_SHIP_COLL_SPARK_DAMAGE` (see [`super::WeaponHit::landed`]).
//! Wired at `oag_raceplay::CANNON_SPARKS_EFFECT`;
//! `crates/game/tests/psys_inventory_ground_truth.rs`'s `WO_CANNON_HIT_SHIP` entry
//! is a PS2-only asset this does not settle.

use crate::Craft;
use oag_core::math::Vec3;
use oag_physics::ShipState;
use oag_physics::params::Dimensions;
use oag_tables::weapons::CannonStats;

use super::KMH_PER_UNIT_PER_SECOND;

/// Added to the firing craft's current speed to get a round's muzzle speed.
///
/// **Recovered, confidence 90.** `func_0x00060af4` (`0x08864af4`, EU `0x08864950`)
/// is `lui a0,0x43fa; mtc1 a0,f0; jr ra`, an unconditional `return 500.0f`
/// ignoring its argument; `Cannon_Init` (`0x088648ec`) adds it to the caller's
/// `speed_kmh`. The earlier "per-class" reading was wrong. The Cannon's `<Stats>`
/// authors no speed (`absorb rounds rate damage_per_bullet slowdown_time`), so
/// the base is baked into the executable.
pub const BASE_SPEED_KMH: f32 = 500.0;

/// How long a Cannon round that hits nothing stays in the air, in seconds.
///
/// **Recovered, confidence 90.** `Cannon_Init` (`0x088648ec`) zeroes the age at
/// `+0x48`, `Cannon_UpdateRound` (`0x0886593c`) adds `dt` each tick, and
/// `CannonPool_Update` (`0x088582b0`) tests `1.0 < round+0x48` as the first half
/// of the `||` opening its teardown. A timed-out round sets neither the wall
/// (`0x10`) nor craft (`0x20`) bit: no sound, no impact. Tested after the tick,
/// strictly greater; the Rocket's is [`super::rocket::LIFETIME_SECONDS`].
pub const LIFETIME_SECONDS: f32 = 1.0;

/// How far a round is pushed off a floor it glances, along the surface normal.
///
/// **Recovered, confidence 82.** `Cannon_UpdateRound`'s third arm reflects the
/// velocity about the normal and moves the round to `hit + normal * 3.0`
/// (`local_2b0 = 0x40400000`).
pub const BOUNCE_PUSH_OFF: f32 = 3.0;

/// How far to each side of the nose the two muzzles sit, as a fraction of the
/// hull's width.
///
/// **Chosen, not measured; no confidence score.** `Weapon_FireCannon` reads two
/// anchors, `craft->entity->barrel[0]`/`barrel[1]`, off the craft's model. This
/// engine has no mesh attachment points, so the muzzles sit a quarter of the
/// width either side of the nose.
pub const MUZZLE_SPACING: f32 = 0.25;

/// Where one round leaves from, and how fast.
///
/// `left` picks the muzzle side, mirroring `Weapon_FireCannon`'s `craft->shots &
/// 1`; the caller reads it off the post-decrement count
/// [`crate::pickup::Held::advance_cannon_reload`] returns, as the original does.
///
/// **The speed is the firing craft's own, not the class's. Recovered.**
/// `Weapon_FireCannon` reads `craft->entity->body->speed * 3.6` and `Cannon_Init`
/// adds [`BASE_SPEED_KMH`]: a stationary craft's round leaves at a fixed rate and
/// a moving one adds its motion, as the Shuriken's throw does.
#[must_use]
pub fn launch(state: &ShipState, dimensions: &Dimensions, left: bool) -> (Vec3, Vec3) {
    let forward = state.body.forward();
    let nose = state.body.position
        + forward * oag_physics::wall::hull_extent(&state.body, dimensions, forward);
    let side = if left { -1.0 } else { 1.0 };
    let position = nose + state.body.right() * (dimensions.width * MUZZLE_SPACING * side);
    let craft_speed_kmh = state.body.linear_velocity.length() * KMH_PER_UNIT_PER_SECOND;
    let speed = (craft_speed_kmh + BASE_SPEED_KMH) / KMH_PER_UNIT_PER_SECOND;
    (position, forward * speed)
}

/// What one round costs the craft it struck, and nobody else.
///
/// Direct-hit only, confirmed against the original's handler: `FUN_08857e90`
/// (reached via `FUN_088579a8` and `FUN_08857f2c`'s `+0x24`, see the module
/// docs) reads `stats+0x7c` (`damage_per_bullet`, `WeaponStats_ParseCannon`
/// `0x0880c774`) into the struck craft's damage accumulator and `stats+0x80`
/// (`slowdown_time`) into a second, unconditionally. A geometry hit
/// (`struck.is_none()`) never reaches this; [`apply_impact`] is the only caller.
///
/// Damage goes through [`oag_physics::damage::apply_weapon`] (recovered state
/// gate, weapons-off halving, clamp). `slowdown_time` is credited as
/// [`super::blast`] credits it, the shield gate living at the drain. `hits` is
/// threaded as `blast` does, so a shielded shell bulges and an unshielded hull
/// sparks; see [`super::WeaponHit`].
pub fn direct_hit<S: Craft>(
    ships: &mut [S],
    slot: usize,
    stats: &CannonStats,
    rules: oag_physics::DamageRules,
    hits: &mut [super::WeaponHit],
) {
    let Some(ship) = ships.get_mut(slot).filter(|s| s.active()) else {
        return;
    };
    *ship.pending_slowdown_mut() += stats.slowdown_time;
    let dimensions = ship.dimensions();
    let report = oag_physics::damage::apply_weapon(
        ship.physics_mut(),
        &dimensions,
        stats.damage_per_bullet,
        rules,
    );
    super::hit::record(hits, slot, &report);
}

/// What one Cannon round's impact does, called from
/// `oag_weapons::projectile::blast::apply_impacts` on the arm that would
/// otherwise invent a blast radius for a weapon that authors none. A direct hit if
/// it struck a craft; nothing for geometry or a table with no Cannon block.
pub fn apply_impact<S: Craft>(
    ships: &mut [S],
    weapons: Option<&oag_tables::weapons::WeaponStats>,
    impact: &super::Impact,
    rules: oag_physics::DamageRules,
    hits: &mut [super::WeaponHit],
) {
    let Some(struck) = impact.struck else {
        return;
    };
    let Some(cannon) = weapons.and_then(oag_tables::weapons::WeaponStats::cannon) else {
        return;
    };
    direct_hit(ships, struck as usize, &cannon, rules, hits);
}

#[cfg(test)]
mod tests;
