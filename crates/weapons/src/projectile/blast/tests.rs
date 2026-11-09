//! The blast rules on their own: the radius sweep, the single-target mine trip and
//! the shield's refusal. Split from `projectile/tests.rs` under the 1,000-line
//! ceiling; the craft fixture stays there (`super::super::tests`).

use super::super::tests::ships;
use super::*;
use oag_core::math::Vec3;

/// [`oag_physics::wall::NOSE_OFFSET`], short for the expected-value arithmetic below.
const NOSE: f32 = oag_physics::wall::NOSE_OFFSET;

/// The blast, both halves: energy off the pool and a posted impulse, for everything
/// inside the radius and nothing outside (a blast reaching every craft would pass a
/// test looking only at the one hit).
///
/// The halves treat distance differently (recovered, easily undone): damage is
/// flat inside the radius, impulse falls off linearly from the craft's nose point.
/// Two craft at different distances: equal damage, different push.
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
            weapon: oag_tables::weapons::Weapon::Rocket,
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

    // The push is posted, not applied: the craft's next physics step drains it
    // (`apply_pending_impulse`), so the velocity has not moved yet.
    for ship in &grid {
        assert_eq!(ship.physics.body.linear_velocity, Vec3::ZERO);
    }
    // Measured from each nose point, `4.875` along the forward axis `-Z`: the craft
    // at the centre has its nose at `z = -4.875` (`falloff = 0.5125`, pushed `-Z`),
    // the one at `z = 8` at `z = 3.125` (`falloff = 0.6875`, pushed `+Z`).
    let expected_centre = Vec3::new(0.0, 0.0, -(1.0 - NOSE / 10.0) * 100.0);
    let expected_inside = Vec3::new(0.0, 0.0, (1.0 - (8.0 - NOSE) / 10.0) * 100.0);
    assert!(
        (grid[0].physics.pending_impulse - expected_centre).length() < 1e-3,
        "centre craft {:?}",
        grid[0].physics.pending_impulse
    );
    assert!(
        (grid[1].physics.pending_impulse - expected_inside).length() < 1e-3,
        "inside craft {:?}",
        grid[1].physics.pending_impulse
    );
    assert_eq!(
        grid[2].physics.pending_impulse,
        Vec3::ZERO,
        "a craft outside the radius was pushed"
    );
    // Damage does not fall off: both craft lost the same 30 (a relation, so it
    // survives the figures changing).
    assert_eq!(
        grid[0].physics.shield, grid[1].physics.shield,
        "the damage fell off with distance - it is the impulse that does, not \
         the damage"
    );
}

/// A shielded craft inside the radius keeps its energy, and the shove is posted
/// like anyone's but the craft's own step discards it: `Ship_ApplyCollisionImpulse`
/// wraps the projection and the stun in `(craft->0x1b8 & 0x10) == 0` and zeroes
/// the pending vector either way (`oag_physics::wall::apply_pending_impulse`).
#[test]
fn a_shielded_craft_keeps_its_energy_and_banks_no_shove() {
    let mut grid = ships(&[(true, Vec3::new(0.0, 0.0, 5.0))]);
    grid[0].handling.dimensions.shield = 100.0;
    grid[0].physics.shield = 100.0;
    grid[0].physics.body.mass = 1.0;
    grid[0].physics.shield_pickup_timer = 1.0;

    blast(
        &mut grid,
        Vec3::ZERO,
        &BlastStats {
            weapon: oag_tables::weapons::Weapon::Rocket,
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
    assert_ne!(
        grid[0].physics.pending_impulse,
        Vec3::ZERO,
        "the shove was not posted"
    );
    oag_physics::wall::apply_pending_impulse(&mut grid[0].physics);
    assert_eq!(
        grid[0].physics.body.linear_velocity,
        Vec3::ZERO,
        "the shield let the shove through"
    );
    assert_eq!(grid[0].physics.stun_timer, 0.0, "the shield let the stun in");
    assert_eq!(grid[0].physics.pending_impulse, Vec3::ZERO);
}

/// An unshielded craft takes the shove as `apply_pending_impulse` makes it: a push
/// along its own forward axis, in the direction the blast pointed relative to it,
/// and the collision stun.
#[test]
fn a_blast_shoves_along_the_forward_axis_and_stuns() {
    let mut grid = ships(&[(true, Vec3::new(0.0, 0.0, 5.0))]);
    grid[0].physics.body.mass = 2.0;

    blast(
        &mut grid,
        Vec3::ZERO,
        &BlastStats {
            weapon: oag_tables::weapons::Weapon::Rocket,
            radius: 10.0,
            damage: 0.0,
            force: 100.0,
            slowdown_time: 0.0,
        },
        oag_physics::DamageRules::default(),
        &mut [],
    );
    oag_physics::wall::apply_pending_impulse(&mut grid[0].physics);

    // Nose at `z = 0.125`: falloff `0.9875`, directed `+Z`, against a forward axis
    // of `-Z`, so the sign flips and the push is `forward * -magnitude`, which is
    // `+Z` again, over mass 2.
    let expected = 0.5 * (1.0 - (5.0 - NOSE) / 10.0) * 100.0;
    let velocity = grid[0].physics.body.linear_velocity;
    assert!((velocity.z - expected).abs() < 1e-3, "{velocity:?}");
    assert_eq!(velocity.x, 0.0);
    assert_eq!(velocity.y, 0.0);
    assert_eq!(
        grid[0].physics.stun_timer,
        oag_physics::wall::STUN_PER_CONTACT
    );
}

/// A direct hit's sweep reaches by the nose point, like `Plasma_ApplyBlastForce`
/// (`d < blastradius` off `T+0x50`): a craft whose centre is outside the radius
/// but whose nose is inside is pushed, and one whose centre is inside but whose
/// nose is outside is not. A centre test would get both wrong.
#[test]
fn a_direct_hit_sweep_reaches_by_the_nose_point_not_the_centre() {
    let mut grid = ships(&[
        (true, Vec3::new(0.0, 0.0, 0.0)),  // the struck craft
        (true, Vec3::new(0.0, 0.0, 12.0)), // centre outside 10, nose at 7.125 inside
        (true, Vec3::new(0.0, 0.0, -8.0)), // centre inside 10, nose at -12.875 outside
        (true, Vec3::new(0.0, 0.0, 9.0)),  // the firer, spared
    ]);
    for ship in &mut grid {
        ship.physics.body.mass = 1.0;
    }
    blast_direct_hit(
        &mut grid,
        Vec3::ZERO,
        &BlastStats {
            weapon: oag_tables::weapons::Weapon::Plasma,
            radius: 10.0,
            damage: 0.0,
            force: 100.0,
            slowdown_time: 0.0,
        },
        0,
        3,
        oag_physics::DamageRules::default(),
        &mut [],
    );
    let reached = grid[1].physics.pending_impulse;
    let expected = Vec3::new(0.0, 0.0, (1.0 - (12.0 - NOSE) / 10.0) * 100.0);
    assert!((reached - expected).length() < 1e-3, "{reached:?}");
    assert_eq!(grid[2].physics.pending_impulse, Vec3::ZERO);
    assert_eq!(grid[3].physics.pending_impulse, Vec3::ZERO, "the firer");
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
        weapon: oag_tables::weapons::Weapon::Rocket,
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
    assert_ne!(
        grid[0].physics.pending_impulse,
        Vec3::ZERO,
        "and it was pushed"
    );
    assert_eq!(
        grid[1].physics.shield, 100.0,
        "a bystander inside the radius took damage"
    );
    assert_eq!(
        grid[1].physics.pending_impulse,
        Vec3::ZERO,
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
