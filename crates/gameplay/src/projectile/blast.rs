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
use oag_formats::weapons::Weapon;

/// One weapon's `blastradius`, `damage` and `blastforce`, or `None`.
///
/// `None` for a table that did not load, for a weapon it does not author, and for
/// a weapon whose block this crate does not decode - all three are the same
/// answer to the caller, which is "spend no blast". A weapon that reaches an
/// impact with no authored numbers is a bug upstream in
/// [`crate::pickup::IMPLEMENTED`], not something to paper over with a default.
pub(super) fn blast_stats(
    weapons: Option<&oag_formats::weapons::WeaponStats>,
    kind: Weapon,
) -> Option<(f32, f32, f32)> {
    let weapons = weapons?;
    match kind {
        Weapon::Rocket => weapons
            .rocket()
            .map(|s| (s.blastradius, s.damage, s.blastforce)),
        Weapon::Missile => weapons
            .missile()
            .map(|s| (s.blastradius, s.damage, s.blastforce)),
        Weapon::Plasma => weapons
            .plasma()
            .map(|s| (s.blastradius, s.damage, s.blastforce)),
        Weapon::Mine => weapons
            .mine()
            .map(|s| (s.blastradius, s.damage, s.blastforce)),
        // **`blastradius`, not `damageradius`.** The Bomb is the only weapon
        // that authors a second radius and the only blast path read at
        // instruction level spends `blastradius` for both halves; see
        // `oag_formats::weapons::BombStats`, which is explicit about the one
        // authored attribute this engine leaves unspent.
        Weapon::Bomb => weapons
            .bomb()
            .map(|s| (s.blastradius, s.damage, s.blastforce)),
        _ => None,
    }
}

/// Spends one blast against every craft inside its radius.
///
/// Full `damage` and a `force` impulse directed away from `point` and **scaled
/// linearly by distance**, with the firing craft included - the split
/// [`Impact`] records and defends. Damage goes through [`oag_physics::damage::apply_weapon`], so
/// the state gate, the weapons-off halving and the clamp are the recovered ones.
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
    radius: f32,
    damage: f32,
    force: f32,
    rules: oag_physics::DamageRules,
    absorbed: &mut [bool],
) -> usize {
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

        let dimensions = ship.handling.dimensions;
        let report =
            oag_physics::damage::apply_weapon(&mut ship.physics, &dimensions, damage, rules);
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
            .apply_impulse(direction * (falloff * force));
    }
    reached
}
