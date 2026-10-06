//! What a hit is worth: which weapon's numbers to spend, and on whom.
//!
//! Split from `projectile.rs` under the 1,000-line rule; re-exported from
//! [`super`]. This runs after a projectile has stopped and reads a weapon table;
//! the parent runs while one moves and reads only geometry.

use crate::Craft;
use oag_core::math::Vec3;
use oag_tables::weapons::{Weapon, WeaponStats};

use super::cannon;

/// What one weapon's blast is worth, read off the table the player's disc
/// authors. A struct so [`blast`] stays under `clippy::too_many_arguments` and
/// same-typed numbers cannot be swapped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastStats {
    /// `<Stats blastradius>`: how far the blast reaches.
    pub radius: f32,
    /// `<Stats damage>`: what it costs a craft's energy, with no falloff.
    pub damage: f32,
    /// `<Stats blastforce>`: the impulse at the centre, falling off linearly.
    pub force: f32,
    /// `<Stats slowdown_time>`: seconds of slowdown credited to a victim's
    /// pending slot. Not a blast term like the others: it is spent on the
    /// victim's timer, credited whether or not a shield is up (the gate is at the
    /// drain), and the law is in `oag_physics::slowdown`. Authored on all four
    /// shipped PSP tables; see `oag_tables::weapons`.
    pub slowdown_time: f32,
}

/// One weapon's [`BlastStats`], or `None` for a table that did not load, a weapon
/// it does not author, or one this crate does not decode: all mean "spend no
/// blast". An impact with no authored numbers is a bug upstream in
/// [`crate::pickup::IMPLEMENTED`], not something to default over.
pub fn blast_stats(
    weapons: Option<&oag_tables::weapons::WeaponStats>,
    kind: Weapon,
) -> Option<BlastStats> {
    let weapons = weapons?;
    match kind {
        Weapon::Rocket => weapons.rocket().map(|s| BlastStats {
            radius: s.blastradius,
            damage: s.damage,
            force: s.blastforce,
            slowdown_time: s.slowdown_time,
        }),
        Weapon::Missile => weapons.missile().map(|s| BlastStats {
            radius: s.blastradius,
            damage: s.damage,
            force: s.blastforce,
            slowdown_time: s.slowdown_time,
        }),
        Weapon::Plasma => weapons.plasma().map(|s| BlastStats {
            radius: s.blastradius,
            damage: s.damage,
            force: s.blastforce,
            slowdown_time: s.slowdown_time,
        }),
        // `blastdamage` and `blastForce`, not the ricochet pair: the Shuriken
        // authors a second damage and force and nothing read says when those are
        // spent; see `oag_tables::weapons::ShurikenStats`.
        Weapon::Shuriken => weapons.shuriken().map(|s| BlastStats {
            radius: s.blastradius,
            damage: s.blastdamage,
            force: s.blastforce,
            slowdown_time: s.slowdown_time,
        }),
        Weapon::Mine => weapons.mine().map(|s| BlastStats {
            radius: s.blastradius,
            damage: s.damage,
            force: s.blastforce,
            slowdown_time: s.slowdown_time,
        }),
        // `blastradius`, not `damageradius`: the only blast path read at
        // instruction level spends `blastradius` for both halves; see
        // `oag_tables::weapons::BombStats`.
        Weapon::Bomb => weapons.bomb().map(|s| BlastStats {
            radius: s.blastradius,
            damage: s.damage,
            force: s.blastforce,
            slowdown_time: s.slowdown_time,
        }),
        _ => None,
    }
}

/// Spends one blast against every craft inside its radius.
///
/// Full [`BlastStats::damage`] and a [`BlastStats::force`] impulse away from
/// `point`, scaled linearly by distance, firing craft included (the split
/// [`Impact`] records). Damage goes through
/// [`oag_physics::damage::apply_weapon`], so the state gate, weapons-off halving
/// and clamp are the recovered ones. Each craft reached is also credited
/// [`BlastStats::slowdown_time`] on `oag_gameplay::world::Ship::pending_slowdown`
/// (see [`crate::slowdown`]). Returns how many craft it reached.
///
/// `hits` has one [`super::WeaponHit`] per ship slot, set and never cleared:
/// whether a fired Shield swallowed this blast, and whether it landed. An
/// out-parameter because the caller is two layers up and [`step`] already returns
/// the other thing callers want.
pub fn blast<S: Craft>(
    ships: &mut [S],
    point: Vec3,
    stats: &BlastStats,
    rules: oag_physics::DamageRules,
    hits: &mut [super::WeaponHit],
) -> usize {
    let radius = stats.radius;
    let mut reached = 0;
    for (slot, ship) in ships.iter_mut().enumerate() {
        if !ship.active() {
            continue;
        }
        let offset = ship.physics().body.position - point;
        let distance = offset.length();
        if distance > radius {
            continue;
        }
        reached += 1;

        // The slowdown credit is not gated on the shield here: all nine of the
        // original's writers of `entity+0x130` simply add; the shield gate is at
        // the drain (`crate::slowdown`), which discards the pending figure, so a
        // hit one tick before a shield expires is lost (recovered). Full
        // `slowdown_time` inside the radius with no falloff, unlike the impulse:
        // the writers add it three instructions from the `Ship_Damage` call. See
        // `oag_physics::slowdown`.
        *ship.pending_slowdown_mut() += stats.slowdown_time;

        let dimensions = ship.dimensions();
        let report =
            oag_physics::damage::apply_weapon(ship.physics_mut(), &dimensions, stats.damage, rules);
        super::hit::record(hits, slot, &report);

        // A craft on the blast centre has no direction: push up, which depends on
        // no axis of the craft or track.
        let direction = if distance > 1e-4 {
            offset.normalize()
        } else {
            Vec3::Y
        };
        // Falloff `1.0 - d / blastradius`, as `Weapon_PostBlastImpulse_q`
        // (`0x0886794c`) computes it; the damage above has none. The original does
        // not clamp it, but the range test already skips anything further out. A
        // zero `radius` is guarded: it admits only `d <= 0`.
        let falloff = if radius > 0.0 {
            1.0 - distance / radius
        } else {
            1.0
        };
        ship.physics_mut()
            .body
            .apply_impulse(direction * (falloff * stats.force));
    }
    reached
}

/// Spends a tripped mine on the one craft that tripped it.
///
/// **Recovered.** `Mine_SweepCraftTrigger` (`0x08867b50`) raises the mine's
/// destroy bit for the first craft inside `trigger_radius` and, only if that
/// craft is also inside `blastradius` (`stats->0xec`), calls
/// `Weapon_PostBlastImpulse(pool, mine, craft)` (`0x0886794c`,
/// `contact-response.md`): `damage` (`+0xe8`), `slowdown_time` (`+0xfc`) and a
/// `(1 - d/blastradius) * blastforce` impulse into `+0x110`, for that craft
/// alone. Teardown only spawns the explosion. So a craft tripping from between
/// the two radii takes nothing, and a bystander inside `blastradius` neither.
pub(super) fn blast_mine_trip<S: Craft>(
    ships: &mut [S],
    point: Vec3,
    stats: &BlastStats,
    struck: u8,
    rules: oag_physics::DamageRules,
    hits: &mut [super::WeaponHit],
) {
    let Some(ship) = ships.get_mut(struck as usize).filter(|s| s.active()) else {
        return;
    };
    let offset = ship.physics().body.position - point;
    let distance = offset.length();
    let radius = stats.radius;
    if distance >= radius {
        return;
    }
    *ship.pending_slowdown_mut() += stats.slowdown_time;
    let dimensions = ship.dimensions();
    let report =
        oag_physics::damage::apply_weapon(ship.physics_mut(), &dimensions, stats.damage, rules);
    super::hit::record(hits, struck as usize, &report);
    let direction = if distance > 1e-4 {
        offset.normalize()
    } else {
        Vec3::Y
    };
    let falloff = if radius > 0.0 {
        1.0 - distance / radius
    } else {
        1.0
    };
    ship.physics_mut()
        .body
        .apply_impulse(direction * (falloff * stats.force));
}

/// Spends a Plasma or Rocket direct craft hit: full [`BlastStats::damage`] and
/// [`BlastStats::slowdown_time`] to `struck` alone, plus a [`BlastStats::force`]
/// impulse, falling off as [`blast`]'s does, to every other active craft within
/// [`BlastStats::radius`] except `owner`.
///
/// **Recovered.** `Plasma_HitCraft` (`0x0886ad60`, confidence 88) credits `struck`
/// unconditionally (no distance test), and `Plasma_ApplyBlastForce`
/// (`0x0886ae08`, confidence 88) sweeps every craft but the firer for the
/// impulse alone. See `docs/ghidra/functions/psp-pulse-usa/plasma.md`, "a craft
/// hit is the third ending". The Rocket's pair is the same shape (read
/// 2026-09-16): `Rocket_HitCraft` (`0x0886ebdc`, `damage` `+0x04`, `slowdown_time`
/// `+0x2c`) and `Rocket_ApplyBlastForce` (`0x0886ee88`, into `entity+0x110`).
/// Neither weapon's wall path touches a craft.
///
/// `struck` is not excluded from the impulse sweep, the original's choice: it
/// takes a near-full falloff on top of the direct credit, and only `owner` is
/// excluded (a bolt cannot hit its own firer's hull, see
/// [`super::Projectiles::advance`], but a bystander near the firer can be inside
/// `radius`). Called from [`apply_impacts`] for these two with `struck: Some(_)`.
pub(super) fn blast_direct_hit<S: Craft>(
    ships: &mut [S],
    point: Vec3,
    stats: &BlastStats,
    struck: u8,
    owner: u8,
    rules: oag_physics::DamageRules,
    hits: &mut [super::WeaponHit],
) {
    if let Some(ship) = ships.get_mut(struck as usize).filter(|s| s.active()) {
        *ship.pending_slowdown_mut() += stats.slowdown_time;
        let dimensions = ship.dimensions();
        let report =
            oag_physics::damage::apply_weapon(ship.physics_mut(), &dimensions, stats.damage, rules);
        super::hit::record(hits, struck as usize, &report);
    }

    let radius = stats.radius;
    for (slot, ship) in ships.iter_mut().enumerate() {
        if !ship.active() || slot as u8 == owner {
            continue;
        }
        let offset = ship.physics().body.position - point;
        let distance = offset.length();
        if distance > radius {
            continue;
        }
        let direction = if distance > 1e-4 {
            offset.normalize()
        } else {
            Vec3::Y
        };
        let falloff = if radius > 0.0 {
            1.0 - distance / radius
        } else {
            1.0
        };
        ship.physics_mut()
            .body
            .apply_impulse(direction * (falloff * stats.force));
    }
}

/// What [`super::step`] does with every impact one tick produced. The Cannon
/// takes its own arm because it has no radius to sweep; see [`cannon::apply_impact`].
pub(super) fn apply_impacts<S: Craft>(
    ships: &mut [S],
    weapons: Option<&WeaponStats>,
    impacts: &[Option<super::Impact>],
    rules: oag_physics::DamageRules,
    hits: &mut [super::WeaponHit],
) {
    for impact in impacts.iter().flatten() {
        // A detonation that only shows an explosion: [`super::Impact::blast`].
        if !impact.blast {
            continue;
        }
        if impact.kind == Weapon::Cannon {
            cannon::apply_impact(ships, weapons, impact, rules, hits);
            continue;
        }
        // A Disruptor has no blast: its effect lands on the struck craft alone,
        // gated as `Disruptor_ApplyEffect` gates it (`crate::disruption::land`),
        // with no damage or slowdown.
        if impact.kind == Weapon::Disruptor {
            if let (Some(struck), Some(kind), Some(stats)) = (
                impact.struck,
                impact.effect,
                weapons.and_then(WeaponStats::disruptor),
            ) && let Some(ship) = ships.get_mut(struck as usize)
            {
                crate::disruption::land(ship, kind, &stats);
            }
            continue;
        }
        let Some(stats) = blast_stats(weapons, impact.kind) else {
            continue;
        };
        // A tripped mine spends itself on the tripper alone:
        // `Mine_SweepCraftTrigger` (`0x08867b50`, read 2026-09-16) calls
        // `Weapon_PostBlastImpulse` for that craft only, and only inside
        // `blastradius`. See [`blast_mine_trip`].
        //
        // A Plasma or Rocket direct hit (`struck` set) spends its blast
        // differently, see [`blast_direct_hit`]. Others use the full-radius
        // [`blast`].
        if impact.kind == Weapon::Mine
            && let Some(struck) = impact.struck
        {
            blast_mine_trip(ships, impact.point, &stats, struck, rules, hits);
            continue;
        }
        if matches!(impact.kind, Weapon::Plasma | Weapon::Rocket)
            && let Some(struck) = impact.struck
        {
            blast_direct_hit(
                ships,
                impact.point,
                &stats,
                struck,
                impact.owner,
                rules,
                hits,
            );
            continue;
        }
        blast(ships, impact.point, &stats, rules, hits);
    }
}

#[cfg(test)]
mod tests;
