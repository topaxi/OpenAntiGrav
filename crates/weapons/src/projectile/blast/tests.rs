//! The blast rules on their own: the radius sweep, the single-target mine
//! trip, and the shield's refusal. Split from `projectile/tests.rs` under the
//! 1,000-line ceiling; the craft fixture stays there and is reached through
//! `super::super::tests`.

use super::super::tests::ships;
use super::*;
use oag_core::math::Vec3;

/// The blast, both halves: energy off the pool and velocity away from the
/// centre, for everything inside the radius and nothing outside it.
///
/// The craft outside is the assertion that matters - a blast that reached
/// every craft on the track would pass any test that only looked at the one
/// that was hit.
///
/// **And the two halves treat distance differently**, which is recovered and is
/// the thing most likely to be quietly undone by a later edit: the damage is
/// flat inside the radius and the impulse falls off linearly. The grid puts one
/// craft at the centre and one at four fifths of the radius precisely so the
/// two are distinguishable - equal damage, and a fivefold difference in push.
#[test]
fn a_blast_reaches_inside_the_radius_and_stops_at_it() {
    let mut grid = ships(&[
        (true, Vec3::ZERO),                // dead centre
        (true, Vec3::new(0.0, 0.0, 8.0)),  // inside a radius of 10
        (true, Vec3::new(0.0, 0.0, 40.0)), // well outside it
    ]);
    for ship in &mut grid {
        ship.handling.dimensions.shield = 100.0;
        ship.physics.shield = 100.0;
        ship.physics.body.mass = 2.0;
    }

    let reached = blast(
        &mut grid,
        Vec3::ZERO,
        &BlastStats {
            radius: 10.0,
            damage: 30.0,
            force: 100.0,
            slowdown_time: 0.0,
        },
        oag_physics::DamageRules::default(),
        &mut [],
    );
    assert_eq!(reached, 2, "the blast reached {reached} craft");

    assert_eq!(grid[0].physics.shield, 70.0, "the craft at the centre");
    assert_eq!(grid[1].physics.shield, 70.0, "the craft inside the radius");
    assert_eq!(
        grid[2].physics.shield, 100.0,
        "a craft outside the radius took damage"
    );

    // `dv = J / m`. The craft at `z = 8` is four fifths of the way out, so
    // `falloff = 1 - 8/10 = 0.2`: 20 units of impulse on a mass of 2, or 10
    // units of velocity, directed away from the centre. Flat force would give
    // 50 here, so this is the assertion the falloff lives or dies on.
    let pushed = grid[1].physics.body.linear_velocity;
    assert!((pushed.z - 10.0).abs() < 1e-3, "pushed {pushed:?}");
    assert_eq!(
        grid[2].physics.body.linear_velocity,
        Vec3::ZERO,
        "a craft outside the radius was pushed"
    );
    // The craft exactly on the centre has no direction, and gets world up
    // rather than a NaN. It is also where `falloff` is `1.0`, so it takes the
    // whole authored force - the other end of the same rule.
    let centred = grid[0].physics.body.linear_velocity;
    assert!(centred.is_finite(), "a centred craft got {centred:?}");
    assert!((centred.y - 50.0).abs() < 1e-3, "{centred:?}");
    // Damage, by contrast, does not fall off: both craft lost the same 30.
    // Asserted as a relation rather than twice as a number, so it survives the
    // figures above changing.
    assert_eq!(
        grid[0].physics.shield, grid[1].physics.shield,
        "the damage fell off with distance - it is the impulse that does, not \
         the damage"
    );
}

/// A shielded craft inside the radius takes neither half. The damage gate is
/// `oag_physics::damage`'s, but the *impulse* is applied here and has no
/// gate of its own - so this is the test that says whether a shield stops a
/// rocket shoving a craft off the racing line.
///
/// **It does not**, and that is deliberate: the shield refuses damage, which
/// is the one thing about it with a duration behind it. Extending it to
/// refuse momentum would be a second invented rule stacked on the first.
#[test]
fn a_shielded_craft_keeps_its_energy_and_still_gets_shoved() {
    let mut grid = ships(&[(true, Vec3::new(0.0, 0.0, 5.0))]);
    grid[0].handling.dimensions.shield = 100.0;
    grid[0].physics.shield = 100.0;
    grid[0].physics.body.mass = 1.0;
    grid[0].physics.shield_pickup_timer = 1.0;

    blast(
        &mut grid,
        Vec3::ZERO,
        &BlastStats {
            radius: 10.0,
            damage: 30.0,
            force: 10.0,
            slowdown_time: 1.0,
        },
        oag_physics::DamageRules::default(),
        &mut [],
    );
    assert_eq!(grid[0].physics.shield, 100.0, "the shield let damage in");
    // **The slowdown credit is not gated here.** A shielded craft is credited
    // like any other and the gate is at the drain, which discards it - see
    // `crate::slowdown::drain`, which asserts the other half.
    assert_eq!(grid[0].pending_slowdown, 1.0);
    assert!(
        grid[0].physics.body.linear_velocity.length() > 0.0,
        "the shield also stopped the shove, which it should not"
    );
}

/// `Mine_SweepCraftTrigger`'s credit goes to the tripping craft alone, and only
/// inside `blastradius`; a bystander inside the radius takes nothing.
#[test]
fn a_tripped_mine_spends_itself_on_the_tripping_craft_only() {
    let mut grid = ships(&[
        (true, Vec3::new(0.0, 0.0, 4.0)), // tripped it, inside the blast radius
        (true, Vec3::new(0.0, 0.0, 6.0)), // bystander, also inside the radius
        (true, Vec3::new(0.0, 0.0, 30.0)), // tripped it from outside the radius
    ]);
    for ship in &mut grid {
        ship.handling.dimensions.shield = 100.0;
        ship.physics.shield = 100.0;
        ship.physics.body.mass = 2.0;
    }
    let stats = BlastStats {
        radius: 10.0,
        damage: 30.0,
        force: 100.0,
        slowdown_time: 1.5,
    };
    blast_mine_trip(
        &mut grid,
        Vec3::ZERO,
        &stats,
        0,
        oag_physics::DamageRules::default(),
        &mut [],
    );
    assert_eq!(grid[0].physics.shield, 70.0, "the craft that tripped it");
    assert_eq!(grid[0].pending_slowdown, 1.5, "and it owes the slowdown");
    assert!(
        grid[0].physics.body.linear_velocity.length() > 0.0,
        "and it was pushed"
    );
    assert_eq!(
        grid[1].physics.shield, 100.0,
        "a bystander inside the radius took damage"
    );
    assert_eq!(
        grid[1].physics.body.linear_velocity.length(),
        0.0,
        "a bystander inside the radius was pushed"
    );

    blast_mine_trip(
        &mut grid,
        Vec3::ZERO,
        &stats,
        2,
        oag_physics::DamageRules::default(),
        &mut [],
    );
    assert_eq!(
        grid[2].physics.shield, 100.0,
        "a craft that tripped the mine from outside blastradius took damage"
    );
}
