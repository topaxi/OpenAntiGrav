//! The Cannon: a round that fires itself, and hurts only what it hits.
//!
//! Everything about *when* a round leaves lives on
//! [`crate::pickup::Held::advance_cannon_reload`] - it is per-craft state that
//! spans ticks, the same shape a mine drop is - and everything about the
//! flight itself is [`super::Projectiles::advance`]'s shared floor-follower,
//! unmodified: the original's own per-round update, `Cannon_UpdateRound`
//! (`0x0886593c`, EU `0x08865798`), is now read
//! (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`'s "What
//! draws a Cannon round" and the 2026-09-09 hit-path addition below it), but
//! its own raycast against the track's collision mesh is not something this
//! engine's shared floor-follower reproduces instruction-for-instruction -
//! so a round here still rides the track and detonates on a wall or a craft
//! exactly the way the Rocket, the Plasma and the Shuriken already do. What
//! is genuinely this weapon's own is in [`launch`] and [`direct_hit`].
//!
//! # The wall hit spawns a spark effect; the craft hit does not
//!
//! **Recovered, confidence 85.** `Cannon_UpdateRound` raycasts each round
//! from its previous position to its new one every tick
//! (`FUN_0883198c`, the same track-collision query `Rocket_Update`,
//! `Missile_Update`, `Plasma_Update`, `Shuriken_Update` and
//! `Camera_UpdatePlayerView` all share) and spawns
//! `Data\Psys\WO_CANNON_SPARKS.POB` - oriented to the hit basis, at the hit
//! point - only when that raycast's own hit-type code is `0` or `4`, a
//! *world/track* hit. A craft hit is a wholly separate test, run
//! immediately after in `CannonPool_Update` (`0x088582b0`, EU `0x0885813c`):
//! a per-craft cylinder-distance sweep (`FUN_088579a8`) that, within `6.0`
//! units, calls `FUN_08857f2c`, which ORs the round's own `+0x3c` flags with
//! `0x24` (the general-hit bit and the *ship* bit, the same bit
//! `CannonPool_Update`'s own despawn pass reads to choose the
//! `CANNONEXPLSHIP` cue over `CANNONEXPLWALL`) and, if the round had not
//! already recorded a hit, calls `FUN_08857e90` - the damage handler [`direct_hit`]
//! is now confirmed against. **Neither of those three functions calls
//! `Psys_Spawn_q`.** So on the PSP, a round that hits a craft plays a sound
//! and applies damage but throws no spark; only a round that hits the track
//! does. See `oag_game::race::CANNON_SPARKS_EFFECT` for where this is wired
//! and `crates/game/tests/psys_inventory_ground_truth.rs`'s
//! `WO_CANNON_HIT_SHIP` entry for the PS2-only asset this does not settle.

use oag_core::math::Vec3;
use oag_formats::weapons::CannonStats;
use oag_physics::ShipState;
use oag_physics::params::Dimensions;

use super::KMH_PER_UNIT_PER_SECOND;

/// Added to the firing craft's own current speed to get a round's muzzle speed.
///
/// **Recovered, confidence 90.** `func_0x00060af4` (`0x08864af4`, EU
/// `0x08864950`) is three instructions on both pressings - `lui a0,0x43fa`;
/// `mtc1 a0,f0`; `jr ra` - an unconditional `return 500.0f` that never
/// touches its own argument. `Cannon_Init` (`0x088648ec`) calls it as
/// `FUN_08864af4(param_2)` and adds the result to the caller's
/// `speed_kmh`, so the value is real, but **the "per-class" premise this
/// constant's name and this page's own earlier reading carried was wrong**:
/// the function ignores the class pointer it is handed and returns the same
/// constant regardless. The Cannon's own `<Stats>` still authors no speed at
/// all (`absorb rounds rate damage_per_bullet slowdown_time`), so this base
/// is baked into the executable rather than authored - just not per class.
pub const BASE_SPEED_KMH: f32 = 500.0;

/// How far to each side of the nose the two muzzles sit, as a fraction of the
/// hull's own width.
///
/// **Chosen, not measured; no confidence score.** `Weapon_FireCannon` reads
/// two distinct anchors, `craft->entity->barrel[0]`/`barrel[1]`, off the
/// craft's own model - a twin-barrel cannon. This engine has no
/// mesh-attachment-point system at all - no weapon here reads one - so the
/// two muzzles are placed a quarter of the hull's own width either side of
/// the nose instead of at the model's real anchors.
pub const MUZZLE_SPACING: f32 = 0.25;

/// Where one round leaves from, and how fast.
///
/// `left` picks the muzzle side, mirroring `Weapon_FireCannon`'s own
/// `craft->shots & 1` test - the caller reads that bit off the **post-decrement**
/// round count [`crate::pickup::Held::advance_cannon_reload`] returns, exactly
/// as the original reads it after its own decrement.
///
/// # The speed is the firing craft's own, not the class's
///
/// **Recovered.** `Weapon_FireCannon` reads
/// `craft->entity->body->speed * 3.6` - the firing craft's *own* current
/// speed, not a per-class figure the way every other weapon's launch is - and
/// `Cannon_Init` adds [`BASE_SPEED_KMH`] to it. A stationary craft therefore
/// fires a round that leaves at a fixed rate however fast the craft itself
/// was going; one already at speed adds its own motion on top, the same
/// "carry the shooter's own speed" shape the Shuriken's throw already has.
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
/// **Direct-hit only, and that shape is now confirmed against the original's
/// own handler, not just the schema's.** `FUN_08857e90` - reached from the
/// craft-proximity test `FUN_088579a8` through `FUN_08857f2c`'s `+0x24` flag
/// set, see the module doc's "The wall hit spawns a spark effect; the craft
/// hit does not" - reads exactly `stats+0x7c` (`damage_per_bullet`, per
/// `WeaponStats_ParseCannon`, `0x0880c774`) into the struck craft's own
/// damage accumulator and `stats+0x80` (`slowdown_time`) into a second one,
/// unconditionally, both in the same call. That is [`CannonStats::damage_per_bullet`]
/// and `slowdown_time` below, applied exactly as this function already did
/// before the handler was read. A round that hits geometry
/// (`struck.is_none()`) reaches no call to this at all; see [`apply_impact`],
/// which is the only caller.
///
/// Damage goes through [`oag_physics::damage::apply_weapon`], the same
/// recovered state gate, weapons-off halving and clamp every other weapon's
/// hit spends - a second path that bypassed it would be a real bug, not a
/// shortcut. `slowdown_time` is credited exactly as [`super::blast`] credits
/// it: unconditionally, with the shield gate living at the drain rather than
/// here. `absorbed` is threaded through for the same reason `blast` takes
/// it - a shielded craft's shell still has to bulge for the hit to read as
/// having landed at all.
pub fn direct_hit(
    ships: &mut [crate::world::Ship],
    slot: usize,
    stats: &CannonStats,
    rules: oag_physics::DamageRules,
    absorbed: &mut [bool],
) {
    let Some(ship) = ships.get_mut(slot).filter(|s| s.active) else {
        return;
    };
    ship.pending_slowdown += stats.slowdown_time;
    let dimensions = ship.handling.dimensions;
    let report = oag_physics::damage::apply_weapon(
        &mut ship.physics,
        &dimensions,
        stats.damage_per_bullet,
        rules,
    );
    if let Some(flag) = absorbed.get_mut(slot) {
        *flag |= report.absorbed;
    }
}

/// What one Cannon round's impact does, called from
/// `oag_gameplay::projectile::blast::apply_impacts` - [`super::step`]'s own
/// per-impact dispatcher - on the arm that would otherwise have to invent a
/// blast radius for a weapon that authors none.
///
/// A direct hit if it struck a craft, and nothing at all if it struck
/// geometry or the table carries no Cannon block - see [`direct_hit`] for why
/// there is no radius to fall back to sweeping.
pub fn apply_impact(
    ships: &mut [crate::world::Ship],
    weapons: Option<&oag_formats::weapons::WeaponStats>,
    impact: &super::Impact,
    rules: oag_physics::DamageRules,
    absorbed: &mut [bool],
) {
    let Some(struck) = impact.struck else {
        return;
    };
    let Some(cannon) = weapons.and_then(oag_formats::weapons::WeaponStats::cannon) else {
        return;
    };
    direct_hit(ships, struck as usize, &cannon, rules, absorbed);
}
