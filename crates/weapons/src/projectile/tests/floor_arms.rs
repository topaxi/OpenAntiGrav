//! A projectile's two floor arms, the probe's and the travel sweep's, per title
//! (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`, "By-catch in the flight arms").
//!
//! The floor is flat at `y = 0`, so every expected velocity is a line of arithmetic.

use super::*;

const DT: f32 = 1.0 / 60.0;

/// One update of one projectile over the floor, the other fields as a fresh launch has them.
fn once(
    kind: Weapon,
    floor_arms: FloorArms,
    position: Vec3,
    velocity: Vec3,
    surface: Vec3,
) -> Projectile {
    let mut projectiles = Projectiles::new();
    projectiles.slots[0] = Projectile {
        kind: Some(kind),
        position,
        velocity,
        surface,
        // Past the Plasma probe window, so a Plasma probes.
        lifetime: MAX_FLIGHT_SECONDS - 0.1,
        ..Projectile::default()
    };
    let impacts = projectiles.advance(
        DT,
        &floor_at_y(0.0),
        None,
        &ships(&[]),
        None,
        None,
        None,
        TriggerRadii::default(),
        "VENOM",
        floor_arms,
        crate::projectile::OwnCraftProbe::PlasmaWindow(0.04),
    );
    assert!(impacts[0].is_none(), "{kind:?} rode the floor");
    projectiles.slots[0]
}

fn near(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < 1e-3
}

/// Starts 1.5 above the ride height, moving flat: the probe finds the floor and the step to the
/// ride point is `(5, -1.5)`.
const ABOVE: Vec3 = Vec3::new(0.0, 4.5, 0.0);

/// Starts above the floor and ends below it, so the probe from the end finds nothing and the
/// sweep crosses the floor at `(1.5, 0, 0)`.
const CROSSING: Vec3 = Vec3::new(0.0, 3.0, 0.0);
const CROSSING_VELOCITY: Vec3 = Vec3::new(120.0, -240.0, 0.0);

#[test]
fn a_shuriken_probe_floor_arm_writes_the_step_to_the_ride_point_raw() {
    let velocity = Vec3::new(300.0, 0.0, 0.0);
    let after = once(
        Weapon::Shuriken,
        FloorArms::SteerToRidePoint,
        ABOVE,
        velocity,
        Vec3::Y,
    );
    // `(hit + normal * 3.0 - from) / dt` = `((5, 3, 0) - (0, 4.5, 0)) * 60`: longer than it was.
    assert!(
        near(after.velocity, Vec3::new(300.0, -90.0, 0.0)),
        "{:?}",
        after.velocity
    );
    assert!(
        near(after.position, Vec3::new(5.0, 3.0, 0.0)),
        "{:?}",
        after.position
    );

    let after = once(
        Weapon::Shuriken,
        FloorArms::ParallelToFloor,
        ABOVE,
        velocity,
        Vec3::Y,
    );
    assert!(near(after.velocity, velocity), "{:?}", after.velocity);
    assert!(
        near(after.position, Vec3::new(5.0, 3.0, 0.0)),
        "{:?}",
        after.position
    );
}

#[test]
fn a_missile_probe_floor_arm_steers_to_the_ride_point_where_a_parallel_turn_does_not() {
    let velocity = Vec3::new(100.0, 0.0, 0.0);
    let steered = once(
        Weapon::Missile,
        FloorArms::SteerToRidePoint,
        ABOVE,
        velocity,
        Vec3::Y,
    );
    // The direction of `(5/3, -1.5)` at the speed it had: `Missile_Update`'s probe arm is
    // `Rocket_Update`'s.
    let toward = Vec3::new(100.0 / 60.0, -1.5, 0.0).normalize() * 100.0;
    assert!(near(steered.velocity, toward), "{:?}", steered.velocity);
    assert!(
        near(steered.position, ABOVE + toward * DT),
        "{:?}",
        steered.position
    );

    let parallel = once(
        Weapon::Missile,
        FloorArms::ParallelToFloor,
        ABOVE,
        velocity,
        Vec3::Y,
    );
    assert!(near(parallel.velocity, velocity), "{:?}", parallel.velocity);
    assert!(
        near(parallel.position, Vec3::new(100.0 / 60.0, 3.0, 0.0)),
        "{:?}",
        parallel.position
    );
}

#[test]
fn a_rocket_and_a_shuriken_sweep_floor_arm_writes_the_step_to_the_ride_point_raw() {
    for kind in [Weapon::Rocket, Weapon::Shuriken] {
        let after = once(
            kind,
            FloorArms::SteerToRidePoint,
            CROSSING,
            CROSSING_VELOCITY,
            Vec3::X,
        );
        // `(hit + normal * 3.0 - from) / dt` = `((1.5, 3, 0) - (0, 3, 0)) * 60`, whatever the speed
        // was, and the floor's normal adopted.
        assert!(
            near(after.velocity, Vec3::new(90.0, 0.0, 0.0)),
            "{kind:?} {:?}",
            after.velocity
        );
        assert!(
            near(after.position, Vec3::new(1.5, 3.0, 0.0)),
            "{kind:?} {:?}",
            after.position
        );
        assert_eq!(after.surface, Vec3::Y, "{kind:?}");

        let after = once(
            kind,
            FloorArms::ParallelToFloor,
            CROSSING,
            CROSSING_VELOCITY,
            Vec3::X,
        );
        // Turned parallel to the floor at the speed it had (the fall has run: a rocket and a blade
        // find nothing below their end, so `y` loses `50 dt`).
        let speed = Vec3::new(120.0, -240.0 - 50.0 * DT, 0.0).length();
        assert!(
            near(after.velocity, Vec3::new(speed, 0.0, 0.0)),
            "{kind:?} {:?}",
            after.velocity
        );
        assert_eq!(after.surface, Vec3::Y, "{kind:?}");
    }
}

#[test]
fn a_plasma_sweep_floor_arm_only_moves() {
    // Carried normal tilted, so the fall along it is distinguishable from world down.
    let carried = Vec3::new(0.6, 0.8, 0.0);
    let after = once(
        Weapon::Plasma,
        FloorArms::SteerToRidePoint,
        CROSSING,
        CROSSING_VELOCITY,
        carried,
    );
    // Seated at the Plasma's own ride height; the normal and the velocity are the probe's miss
    // (`velocity -= surface * 50 dt`) and nothing more.
    assert!(
        near(after.position, Vec3::new(1.5, plasma::RIDE_HEIGHT, 0.0)),
        "{:?}",
        after.position
    );
    assert_eq!(after.surface, carried);
    assert!(
        near(
            after.velocity,
            CROSSING_VELOCITY - carried * (FALL_ACCELERATION * DT)
        ),
        "{:?}",
        after.velocity
    );

    let after = once(
        Weapon::Plasma,
        FloorArms::ParallelToFloor,
        CROSSING,
        CROSSING_VELOCITY,
        carried,
    );
    assert_eq!(
        after.surface,
        Vec3::Y,
        "a title without the law adopts the floor's normal"
    );
}

#[test]
fn a_missile_sweep_floor_arm_only_moves_on_every_title() {
    for floor_arms in [FloorArms::SteerToRidePoint, FloorArms::ParallelToFloor] {
        let after = once(
            Weapon::Missile,
            floor_arms,
            CROSSING,
            CROSSING_VELOCITY,
            Vec3::X,
        );
        assert!(
            near(after.position, Vec3::new(1.5, 3.0, 0.0)),
            "{:?}",
            after.position
        );
        assert_eq!(after.surface, Vec3::X);
    }
}
