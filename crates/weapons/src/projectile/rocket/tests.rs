//! The Rocket's launch and its two speeds, as measured on Pulse PSP 2026-10-01.

use super::*;
use crate::projectile::{Projectiles, TriggerRadii};
use oag_physics::{CollisionWorld, Surface, TriangleSoup};

const DT: f32 = 1.0 / 60.0;

fn stats() -> RocketStats {
    oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Rocket"><Stats absorb="1" blastforce="2" blastradius="3"
               damage="4" slowdown_time="5" venomspeed="800" flashspeed="900"
               rapierspeed="1000" phantomspeed="1100" launchSpeed="200" spread="0"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture parses")
    .rocket()
    .expect("a Rocket")
}

fn floor_at_y(y: f32) -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-5000.0, y, -5000.0],
            [-5000.0, y, 5000.0],
            [5000.0, y, 5000.0],
            [5000.0, y, -5000.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Vec::new(),
        Surface::Floor,
        0,
    ));
    world
}

fn tick(projectiles: &mut Projectiles, world: &CollisionWorld) {
    let _ = projectiles.advance(
        DT,
        world,
        &[] as &[crate::test_craft::Ship],
        None,
        None,
        None,
        TriggerRadii::default(),
        "VENOM",
    );
}

/// Venom authors 800 km/h, which is 222.22 units/s; the launch is 0.75 of it.
const CLASS: f32 = 800.0 / KMH_PER_UNIT_PER_SECOND;

/// Out of the surface probe's reach the rocket never renormalises, so it keeps
/// the launch speed. `launchSpeed="200"` is authored and must play no part.
#[test]
fn a_rocket_with_no_floor_in_reach_keeps_the_launch_speed() {
    let mut projectiles = Projectiles::new();
    let fired = fire(
        &mut projectiles,
        &ShipState::default(),
        &stats(),
        "VENOM",
        0,
    )
    .expect("Venom is authored");
    assert_eq!(fired, ROCKET_SHOTS);
    let world = floor_at_y(-100.0);
    for _ in 0..4 {
        tick(&mut projectiles, &world);
    }
    let velocity = projectiles.slots[0].velocity;
    let along = (velocity - Vec3::Y * velocity.y).length();
    assert!(
        (along - CLASS * LAUNCH_SPEED_SCALE).abs() < 1e-2,
        "expected {} (0.75 x class), got {along}",
        CLASS * LAUNCH_SPEED_SCALE
    );
}

/// The first surface hit pins the speed to the class's, the step from 166.67 to
/// 222.22 the original's probe reads. Drops if `fire` stops carrying the class
/// speed or the flight stops pinning to it.
#[test]
fn the_first_surface_hit_sets_the_class_speed() {
    let mut projectiles = Projectiles::new();
    fire(
        &mut projectiles,
        &ShipState::default(),
        &stats(),
        "VENOM",
        0,
    )
    .expect("Venom is authored");
    let before = projectiles.slots[0].velocity.length();
    assert!((before - CLASS * LAUNCH_SPEED_SCALE).abs() < 1e-2);
    // Floor 4 units under the craft: inside the 6-unit probe from the first tick.
    tick(&mut projectiles, &floor_at_y(-4.0));
    let after = projectiles.slots[0].velocity.length();
    assert!(
        (after - CLASS).abs() < 1e-2,
        "expected the class speed {CLASS}, got {after}"
    );
}

/// The first probe goes along the craft's own normal, not world up: `Rocket_Init`
/// seeds the stored normal from the craft. Drops if `fire` goes back to
/// `spawn_guided`'s world-up seed.
#[test]
fn a_rocket_is_seeded_with_the_craft_up() {
    let mut state = ShipState::default();
    state.body.orientation = oag_core::math::Quat::from_rotation_z(0.2);
    let mut projectiles = Projectiles::new();
    fire(&mut projectiles, &state, &stats(), "VENOM", 0).expect("Venom is authored");
    for slot in &projectiles.slots[..ROCKET_SHOTS] {
        assert_eq!(slot.surface, state.body.up());
    }
    assert_ne!(
        state.body.up(),
        Vec3::Y,
        "the craft must be tilted for this to mean anything"
    );
}

/// On a floor hit the Rocket steers toward the ride point and re-integrates from
/// where it was: `velocity = normalize((hit + 3n) - from) * class speed`, and
/// the new position is `from + velocity * dt`, not the ride point itself and not
/// the old velocity turned parallel to the floor (which would leave `velocity.y`
/// at zero over a flat floor). `Rocket_Update`'s probe arm, measured to land
/// the volley within a few units of the original's ranges.
#[test]
fn a_rocket_steers_toward_the_ride_point_on_a_floor_hit() {
    let mut projectiles = Projectiles::new();
    let launch_kmh = 800.0;
    assert!(projectiles.spawn_riding(
        oag_tables::weapons::Weapon::Rocket,
        Vec3::ZERO,
        Vec3::Z * (launch_kmh / KMH_PER_UNIT_PER_SECOND * LAUNCH_SPEED_SCALE),
        0,
        Vec3::Y,
        launch_kmh,
    ));
    // The projected position is (0, 0, 0.75 * 222.22 * dt); the probe from it
    // reaches the floor 4 units down, so the ride point is 1 unit above zero.
    let projected = Vec3::Z * (launch_kmh / KMH_PER_UNIT_PER_SECOND * LAUNCH_SPEED_SCALE * DT);
    let ride_point = Vec3::new(0.0, -1.0, projected.z);
    let direction = ride_point.normalize();
    tick(&mut projectiles, &floor_at_y(-4.0));
    let rocket = projectiles.slots[0];
    let speed = launch_kmh / KMH_PER_UNIT_PER_SECOND;
    assert!(
        (rocket.velocity - direction * speed).length() < 1e-2,
        "velocity {:?}, wanted {:?}",
        rocket.velocity,
        direction * speed
    );
    assert!(
        (rocket.position - direction * speed * DT).length() < 1e-3,
        "position {:?} is not from + velocity * dt",
        rocket.position
    );
    assert!(
        rocket.velocity.y < -50.0,
        "it did not steer down: {:?}",
        rocket.velocity
    );
}
