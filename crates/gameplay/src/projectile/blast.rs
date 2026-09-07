//! What a hit is worth: which weapon's numbers to spend, and on whom.
//!
//! Split out of `projectile.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Both items
//! are re-exported from [`super`], so `projectile::blast` and
//! `projectile::blast_stats` still resolve and no call site moved.
//!
//! The seam is real: everything here runs **after** a projectile has stopped and
//! reads a weapon table, where everything left in the parent runs while one is
//! still moving and reads only geometry. They are two different questions about
//! the same tick.

use oag_core::math::Vec3;
use oag_formats::weapons::{Weapon, WeaponStats};

use super::cannon;

/// What one weapon's blast is worth, read off the table the player's disc
/// authors.
///
/// A struct rather than the tuple this was until 2026-09-06, for two reasons
/// that arrived together: `slowdown_time` made it a fourth field, and passing
/// four loose `f32`s on into [`blast`] would have put that function at eight
/// arguments and past `clippy::too_many_arguments`. Naming them also removes
/// the one way a tuple of same-typed numbers goes wrong.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastStats {
    /// `<Stats blastradius>`: how far the blast reaches.
    pub radius: f32,
    /// `<Stats damage>`: what it costs a craft's energy, with no falloff.
    pub damage: f32,
    /// `<Stats blastforce>`: the impulse at the centre, falling off linearly.
    pub force: f32,
    /// `<Stats slowdown_time>`: the seconds of slowdown it credits to a
    /// victim's pending slot.
    ///
    /// **Not a blast term the way the other three are.** It is spent on the
    /// victim's timer rather than on its body, it is credited whether or not a
    /// shield is up (the gate is at the drain), and the whole law is in
    /// `oag_physics::slowdown`. Decoded on all six blocks and authored on all
    /// four shipped PSP tables - see `oag_formats::weapons`.
    pub slowdown_time: f32,
}

/// One weapon's [`BlastStats`], or `None`.
///
/// `None` for a table that did not load, for a weapon it does not author, and for
/// a weapon whose block this crate does not decode - all three are the same
/// answer to the caller, which is "spend no blast". A weapon that reaches an
/// impact with no authored numbers is a bug upstream in
/// [`crate::pickup::IMPLEMENTED`], not something to paper over with a default.
pub(super) fn blast_stats(
    weapons: Option<&oag_formats::weapons::WeaponStats>,
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
        // **`blastdamage` and `blastForce`, not the ricochet pair.** The
        // Shuriken is the only weapon authoring a second damage and a second
        // force, and nothing read says when those are spent - see
        // `oag_formats::weapons::ShurikenStats`, which leaves both undecoded
        // rather than picking one.
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
        // **`blastradius`, not `damageradius`.** The Bomb is the only weapon
        // that authors a second radius and the only blast path read at
        // instruction level spends `blastradius` for both halves; see
        // `oag_formats::weapons::BombStats`, which is explicit about the one
        // authored attribute this engine leaves unspent.
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
/// Full [`BlastStats::damage`] and a [`BlastStats::force`] impulse directed away
/// from `point` and **scaled linearly by distance**, with the firing craft
/// included - the split [`Impact`] records and defends. Damage goes through
/// [`oag_physics::damage::apply_weapon`], so the state gate, the weapons-off
/// halving and the clamp are the recovered ones.
///
/// Every craft it reaches is also credited [`BlastStats::slowdown_time`] on
/// [`crate::world::Ship::pending_slowdown`], which is what makes a craft hit by
/// a rocket lose its engine - see [`crate::slowdown`].
///
/// Returns how many craft it reached, which is what a caller asserting "the
/// blast did something" wants and what a test asserting "and nothing outside the
/// radius" needs the other half of.
///
/// # `absorbed`
///
/// One flag per ship slot, **set and never cleared**, marking a craft whose
/// fired Shield swallowed this blast. The original's weapon-damage drain
/// (`Ship_ApplyPendingWeaponDamage`, `0x0883f13c`) takes a shield branch that
/// discards the amount and calls `ShipShield_Hit` instead, so a swallowed hit is
/// the *only* thing that makes the shell visibly react - see
/// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
///
/// An out-parameter rather than a second return value, because the caller that
/// wants it is two layers up and the intermediate ([`step`]) already returns the
/// thing every other caller asks for. A caller with nothing to draw passes a
/// scratch array; a short slice is written as far as it goes rather than
/// panicking, so `&mut []` is a legal "do not tell me".
pub fn blast(
    ships: &mut [crate::world::Ship],
    point: Vec3,
    stats: &BlastStats,
    rules: oag_physics::DamageRules,
    absorbed: &mut [bool],
) -> usize {
    let radius = stats.radius;
    let mut reached = 0;
    for (slot, ship) in ships.iter_mut().enumerate() {
        if !ship.active {
            continue;
        }
        let offset = ship.physics.body.position - point;
        let distance = offset.length();
        if distance > radius {
            continue;
        }
        reached += 1;

        // **The slowdown credit, and it is not gated on the shield here.** Every
        // one of the original's nine writers of `entity+0x130` simply adds to
        // it; the shield gate lives at the *drain*, in `crate::slowdown`, and it
        // discards the pending figure rather than banking it. Splitting the two
        // that way is what makes a hit landed one tick before a shield expires
        // simply lost, which is the recovered behaviour. See
        // `oag_physics::slowdown` for the whole law.
        //
        // Full `slowdown_time` everywhere inside the radius, with no falloff -
        // the same shape as the damage below and unlike the impulse. Nothing
        // read suggests otherwise: the writers add the block's figure straight
        // in, three instructions from the `Ship_Damage` call that spends
        // `damage`.
        ship.pending_slowdown += stats.slowdown_time;

        let dimensions = ship.handling.dimensions;
        let report =
            oag_physics::damage::apply_weapon(&mut ship.physics, &dimensions, stats.damage, rules);
        // `|=` rather than `=`: two blasts in one tick against one shielded
        // craft are two absorbs, and the second must not clear the first.
        if let Some(flag) = absorbed.get_mut(slot) {
            *flag |= report.absorbed;
        }

        // A craft exactly on the blast centre has no direction to be pushed in.
        // World up rather than a zero push or a normalised NaN: something has to
        // happen, and up is the one direction that does not depend on an
        // arbitrary axis of the craft or of the track.
        let direction = if distance > 1e-4 {
            offset.normalize()
        } else {
            Vec3::Y
        };
        // **The falloff is recovered and the damage above deliberately has
        // none.** `1.0 - d / blastradius`, exactly as `Weapon_PostBlastImpulse_q`
        // (`0x0886794c`) computes it, so a craft on the rim is nudged and one at
        // the centre is thrown. The original does **not** clamp this - a hit
        // outside the radius drives the term negative there and nothing in that
        // function stops it - which cannot happen here because the range test
        // above has already skipped anything further out. A `radius` of zero
        // would divide by zero, so it is guarded: a weapon with no radius
        // reaches nobody anyway, since the test above admits only `d <= 0`.
        let falloff = if radius > 0.0 {
            1.0 - distance / radius
        } else {
            1.0
        };
        ship.physics
            .body
            .apply_impulse(direction * (falloff * stats.force));
    }
    reached
}

/// What [`super::step`] does with every impact one tick produced.
///
/// Split out of `step` itself under the parent module's own 1,000-line
/// ceiling - a move, with no behaviour change. The Cannon takes its own arm
/// because it has no radius to sweep at all; see [`cannon::apply_impact`].
pub(super) fn apply_impacts(
    ships: &mut [crate::world::Ship],
    weapons: Option<&WeaponStats>,
    impacts: &[Option<super::Impact>],
    rules: oag_physics::DamageRules,
    absorbed: &mut [bool],
) {
    for impact in impacts.iter().flatten() {
        // A detonation that only shows an explosion - see [`super::Impact::blast`].
        if !impact.blast {
            continue;
        }
        if impact.kind == Weapon::Cannon {
            cannon::apply_impact(ships, weapons, impact, rules, absorbed);
            continue;
        }
        let Some(stats) = blast_stats(weapons, impact.kind) else {
            continue;
        };
        blast(ships, impact.point, &stats, rules, absorbed);
    }
}
