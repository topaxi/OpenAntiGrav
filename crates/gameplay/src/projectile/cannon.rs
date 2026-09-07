//! The Cannon: a round that fires itself, and hurts only what it hits.
//!
//! Everything about *when* a round leaves lives on
//! [`crate::pickup::Held::advance_cannon_reload`] - it is per-craft state that
//! spans ticks, the same shape a mine drop is - and everything about the
//! flight itself is [`super::Projectiles::advance`]'s shared floor-follower,
//! unmodified: no `Cannon_Update` was found on
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, so a round
//! here rides the track and detonates on a wall or a craft exactly the way
//! the Rocket, the Plasma and the Shuriken already do, as the placeholder that
//! page's own "What is buildable now" section argues every unread weapon here
//! gets. What is genuinely this weapon's own is in [`launch`] and
//! [`direct_hit`].

use oag_core::math::Vec3;
use oag_formats::weapons::CannonStats;
use oag_physics::ShipState;
use oag_physics::params::Dimensions;

use super::KMH_PER_UNIT_PER_SECOND;

/// Added to the firing craft's own current speed to get a round's muzzle speed.
///
/// **Chosen, not measured; no confidence score.** `Cannon_Init` (`0x088648ec`)
/// computes `speed = craft_speed_kmh + <a per-class base speed read off
/// func_0x00060af4, i.e. 0x08864af4, not decompiled this pass>` - the Cannon's
/// own `<Stats>` authors no speed at all
/// (`absorb rounds rate damage_per_bullet slowdown_time`), so that base is
/// baked into the executable rather than authored, and it genuinely cannot be
/// read off the disc. **This is the one place this weapon's evidence page is
/// not sufficient to build from without a further Ghidra session** - see
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`'s own "Not
/// chased" note on this exact function.
///
/// Zero would leave a round drifting beside the craft that fired it rather
/// than visibly leaving the barrel, which is not a legible weapon at all, so
/// this picks a value in the range the Rocket's own authored `launchSpeed`
/// (200 km/h) occupies - doubled, on the reasoning that a cannon round should
/// read faster relative to the craft than a rocket's own added component
/// does. Revisit once `0x08864af4` is decompiled.
pub const BASE_SPEED_KMH: f32 = 400.0;

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
/// **Direct-hit only, and that is the schema's own shape rather than a
/// choice** - see [`CannonStats::damage_per_bullet`]. A round that hits
/// geometry (`struck.is_none()`) reaches no call to this at all; see
/// [`apply_impact`], which is the only caller.
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
