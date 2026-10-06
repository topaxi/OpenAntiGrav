//! The blast rules on their own: the radius sweep, the single-target mine trip and
//! the shield's refusal. Split from `projectile/tests.rs` under the 1,000-line
//! ceiling; the craft fixture stays there (`super::super::tests`).

use super::super::tests::ships;
use super::*;
use oag_core::math::Vec3;

/// The blast, both halves: energy off the pool and velocity away from the centre,
/// for everything inside the radius and nothing outside (a blast reaching every
/// craft would pass a test looking only at the one hit).
///
/// The halves treat distance differently (recovered, easily undone): damage is
/// flat inside the radius, impulse falls off linearly. One craft at the centre and
/// one at four fifths of the radius: equal damage, a fivefold difference in push.
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

    // `dv = J / m`. The craft at `z = 8` is four fifths out, `falloff = 0.2`: 20
    // units of impulse on a mass of 2, 10 of velocity, away from the centre. Flat
    // force would give 50.
    let pushed = grid[1].physics.body.linear_velocity;
    assert!((pushed.z - 10.0).abs() < 1e-3, "pushed {pushed:?}");
    assert_eq!(
        grid[2].physics.body.linear_velocity,
        Vec3::ZERO,
        "a craft outside the radius was pushed"
    );
    // A craft on the centre has no direction and gets world up, not a NaN; its
    // `falloff` is `1.0`, the whole authored force.
    let centred = grid[0].physics.body.linear_velocity;
    assert!(centred.is_finite(), "a centred craft got {centred:?}");
    assert!((centred.y - 50.0).abs() < 1e-3, "{centred:?}");
    // Damage does not fall off: both craft lost the same 30 (a relation, so it
    // survives the figures changing).
    assert_eq!(
        grid[0].physics.shield, grid[1].physics.shield,
        "the damage fell off with distance - it is the impulse that does, not \
         the damage"
    );
}

/// A shielded craft inside the radius takes neither half. The damage gate is
/// `oag_physics::damage`'s but the impulse has no gate of its own; this says
/// whether a shield stops a rocket shoving a craft off the line. **It does not**,
/// deliberately: the shield refuses damage, the one thing with a duration behind
/// it, and refusing momentum would be a second invented rule.
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
    // The slowdown credit is not gated here: a shielded craft is credited and the
    // gate at the drain discards it (`crate::slowdown::drain` asserts that half).
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
