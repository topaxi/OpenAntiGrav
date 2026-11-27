//! A projectile's two floor arms, the probe's and the travel sweep's, per title
//! (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`, "By-catch in the flight arms";
//! HD's in `docs/ghidra/functions/ps3-hdfury-eu/projectile-floor-arms.md`).
//!
//! The floor is flat at `y = 0`, so every expected velocity is a line of arithmetic.

use super::*;
use crate::projectile::{FlightLaws, PlasmaFall};

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
        crate::projectile::LaunchScale::DisplayScaled,
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
fn a_shuriken_probe_floor_arm_writes_the_step_to_the_ride_point_raw_on_pulse_and_on_hd() {
    for floor_arms in [FloorArms::SteerToRidePoint, FloorArms::ParallelToFloor] {
        let velocity = Vec3::new(300.0, 0.0, 0.0);
        let after = once(Weapon::Shuriken, floor_arms, ABOVE, velocity, Vec3::Y);
        // `(hit + normal * 3.0 - from) / dt` = `((5, 3, 0) - (0, 4.5, 0)) * 60`: longer than it was.
        assert!(
            near(after.velocity, Vec3::new(300.0, -90.0, 0.0)),
            "{floor_arms:?} {:?}",
            after.velocity
        );
        assert!(
            near(after.position, Vec3::new(5.0, 3.0, 0.0)),
            "{floor_arms:?} {:?}",
            after.position
        );
    }
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

/// Where `CROSSING`'s step ends: past the floor it crosses.
const CROSSED: Vec3 = Vec3::new(2.0, -1.0, 0.0);

#[test]
fn a_rocket_and_a_shuriken_sweep_floor_arm_writes_the_step_to_the_ride_point_raw() {
    for (kind, floor_arms) in [
        (Weapon::Rocket, FloorArms::SteerToRidePoint),
        (Weapon::Shuriken, FloorArms::SteerToRidePoint),
        (Weapon::Shuriken, FloorArms::ParallelToFloor),
    ] {
        let after = once(kind, floor_arms, CROSSING, CROSSING_VELOCITY, Vec3::X);
        // `(hit + normal * 3.0 - from) / dt` = `((1.5, 3, 0) - (0, 3, 0)) * 60`, whatever the speed
        // was, and the floor's normal adopted.
        assert!(
            near(after.velocity, Vec3::new(90.0, 0.0, 0.0)),
            "{kind:?} {floor_arms:?} {:?}",
            after.velocity
        );
        assert!(
            near(after.position, Vec3::new(1.5, 3.0, 0.0)),
            "{kind:?} {floor_arms:?} {:?}",
            after.position
        );
        assert_eq!(after.surface, Vec3::Y, "{kind:?} {floor_arms:?}");
    }
}

#[test]
fn hds_rocket_runs_no_sweep_floor_arm() {
    let after = once(
        Weapon::Rocket,
        FloorArms::ParallelToFloor,
        CROSSING,
        CROSSING_VELOCITY,
        Vec3::X,
    );
    // The step's own end, the carried normal and the velocity the probe's miss left.
    assert!(near(after.position, CROSSED), "{:?}", after.position);
    assert_eq!(after.surface, Vec3::X);
    assert!(
        near(
            after.velocity,
            CROSSING_VELOCITY - Vec3::Y * (FALL_ACCELERATION * DT)
        ),
        "{:?}",
        after.velocity
    );
}

#[test]
fn a_pulse_plasma_sweep_floor_arm_only_moves_and_hds_runs_none() {
    // Carried normal tilted, so the fall along it is distinguishable from world down.
    let carried = Vec3::new(0.6, 0.8, 0.0);
    // Seated at the Plasma's own ride height on Pulse, at the step's end on HD; the normal and
    // the velocity are the probe's miss (`velocity -= surface * 50 dt`) and nothing more.
    for (floor_arms, position) in [
        (
            FloorArms::SteerToRidePoint,
            Vec3::new(1.5, plasma::RIDE_HEIGHT, 0.0),
        ),
        (FloorArms::ParallelToFloor, CROSSED),
    ] {
        let after = once(
            Weapon::Plasma,
            floor_arms,
            CROSSING,
            CROSSING_VELOCITY,
            carried,
        );
        assert!(
            near(after.position, position),
            "{floor_arms:?} {:?}",
            after.position
        );
        assert_eq!(after.surface, carried, "{floor_arms:?}");
        assert!(
            near(
                after.velocity,
                CROSSING_VELOCITY - carried * (FALL_ACCELERATION * DT)
            ),
            "{floor_arms:?} {:?}",
            after.velocity
        );
    }
}

#[test]
fn a_craft_past_a_floor_hd_passes_through_is_still_struck() {
    for kind in [Weapon::Rocket, Weapon::Plasma] {
        let mut projectiles = Projectiles::new();
        projectiles.slots[0] = Projectile {
            kind: Some(kind),
            position: CROSSING,
            // Twice `CROSSING_VELOCITY`: the floor is met 3.4 along the step, the hull 5.8.
            velocity: CROSSING_VELOCITY * 2.0,
            surface: Vec3::X,
            lifetime: MAX_FLIGHT_SECONDS - 0.1,
            ..Projectile::default()
        };
        let impacts = projectiles.advance(
            DT,
            &floor_at_y(0.0),
            None,
            // Slot 0 fired it.
            &ships(&[(false, Vec3::ZERO), (true, Vec3::new(3.5, -4.0, 0.0))]),
            None,
            None,
            None,
            TriggerRadii::default(),
            "VENOM",
            FloorArms::ParallelToFloor,
            crate::projectile::OwnCraftProbe::PlasmaWindow(0.04),
            crate::projectile::LaunchScale::DisplayScaled,
        );
        assert_eq!(
            impacts[0].and_then(|impact| impact.struck),
            Some(1),
            "{kind:?}"
        );
    }
}

/// A Plasma bolt over a floor too far below to probe, one update under a title's fall.
fn open_air_plasma(floor_arms: FloorArms, plasma_fall: PlasmaFall, carried: Vec3) -> Projectile {
    let mut projectiles = Projectiles::new();
    projectiles.slots[0] = Projectile {
        kind: Some(Weapon::Plasma),
        position: Vec3::new(0.0, 50.0, 0.0),
        velocity: Vec3::new(0.0, 0.0, 300.0),
        surface: carried,
        launch_speed_kmh: 600.0,
        lifetime: MAX_FLIGHT_SECONDS - 0.1,
        ..Projectile::default()
    };
    let table = oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Plasma"><Stats absorb="1" blastforce="10" blastradius="12"
               damage="25" slowdown_time="1" speed="600" launchSpeed="0"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture parses");
    let impacts = projectiles.advance(
        DT,
        &floor_at_y(0.0),
        None,
        &ships(&[]),
        None,
        table.plasma().as_ref(),
        None,
        TriggerRadii::default(),
        "VENOM",
        FlightLaws {
            plasma_fall,
            ..FlightLaws::from(floor_arms)
        },
        crate::projectile::OwnCraftProbe::NoBox,
        crate::projectile::LaunchScale::Unscaled,
    );
    assert!(impacts[0].is_none());
    projectiles.slots[0]
}

#[test]
fn hds_plasma_falls_on_world_y_and_is_pinned_back_to_its_speed() {
    let carried = Vec3::new(0.6, 0.8, 0.0);
    let hd = open_air_plasma(FloorArms::ParallelToFloor, PlasmaFall::WorldY, carried);
    let fallen = Vec3::new(0.0, -FALL_ACCELERATION * DT, 300.0);
    assert!(
        near(hd.velocity, fallen.normalize() * (600.0 / 3.6)),
        "{:?}",
        hd.velocity
    );
    assert_eq!(hd.surface, carried);

    // Pulse pulls along the carried normal and keeps whatever speed that leaves.
    let pulse = open_air_plasma(
        FloorArms::SteerToRidePoint,
        PlasmaFall::AlongCarriedNormal,
        carried,
    );
    assert!(
        near(
            pulse.velocity,
            Vec3::new(0.0, 0.0, 300.0) - carried * (FALL_ACCELERATION * DT)
        ),
        "{:?}",
        pulse.velocity
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

/// Pure's carried normal, 15 degrees off the floor's (the Sebenco Climb shot,
/// `docs/ghidra/functions/psp-pure-usa/floor-follow.md`).
fn tilted() -> Vec3 {
    let angle = 15.0_f32.to_radians();
    Vec3::new(angle.sin(), angle.cos(), 0.0)
}

#[test]
fn under_the_launch_normal_a_rocket_and_a_plasma_ride_along_their_own_normal() {
    let surface = tilted();
    for (kind, ride) in [(Weapon::Rocket, 2.25), (Weapon::Plasma, 4.5)] {
        let velocity = Vec3::new(300.0, 0.0, 0.0);
        let after = once(kind, FloorArms::LaunchNormal, ABOVE, velocity, surface);
        // The probe leaves `(5, 4.5, 0)` along `-surface` and meets `y = 0`; the ride point is
        // `ride` along the carried normal from there, and the step to it is written raw.
        let to = ABOVE + velocity * DT;
        let hit = to - surface * (to.y / surface.y);
        let point = hit + surface * ride;
        assert!(near(after.position, point), "{kind:?} {:?}", after.position);
        assert!(
            near(after.velocity, (point - ABOVE) / DT),
            "{kind:?} {:?}",
            after.velocity
        );
        assert!(near(after.surface, surface), "{kind:?} {:?}", after.surface);

        // Pulse's arm adopts the floor's normal and rides 3 (4 for the Plasma) straight above it.
        let pulse = once(kind, FloorArms::SteerToRidePoint, ABOVE, velocity, surface);
        assert!(near(pulse.surface, Vec3::Y), "{kind:?} {:?}", pulse.surface);
        assert!(
            (pulse.position - after.position).length() > 0.3,
            "{kind:?}: {:?} against {:?}",
            pulse.position,
            after.position
        );
    }
}

#[test]
fn under_the_launch_normal_the_travel_sweep_runs_before_the_probe() {
    // The step crosses the floor at `(1.5, 0, 0)`: the sweep pushes the end to `(1.5, 3, 0)`
    // along the hit's normal, the probe from there finds the floor and the Rocket rides 2.25
    // above it. Probing first, from below the floor, Pulse's falls and lands on the sweep's
    // `(1.5, 3, 0)` instead.
    let after = once(
        Weapon::Rocket,
        FloorArms::LaunchNormal,
        CROSSING,
        CROSSING_VELOCITY,
        Vec3::Y,
    );
    assert!(
        near(after.position, Vec3::new(1.5, 2.25, 0.0)),
        "{:?}",
        after.position
    );
    assert!(near(after.surface, Vec3::Y), "{:?}", after.surface);
    let pulse = once(
        Weapon::Rocket,
        FloorArms::SteerToRidePoint,
        CROSSING,
        CROSSING_VELOCITY,
        Vec3::Y,
    );
    assert!(
        near(pulse.position, Vec3::new(1.5, 3.0, 0.0)),
        "{:?}",
        pulse.position
    );
}

#[test]
fn under_the_launch_normal_a_plasma_with_nothing_below_falls_along_world_down() {
    let surface = tilted();
    let velocity = Vec3::new(300.0, 0.0, 0.0);
    let high = Vec3::new(0.0, 40.0, 0.0);
    let after = once(
        Weapon::Plasma,
        FloorArms::LaunchNormal,
        high,
        velocity,
        surface,
    );
    assert!(
        near(
            after.velocity,
            velocity - Vec3::Y * (FALL_ACCELERATION * DT)
        ),
        "{:?}",
        after.velocity
    );
    let pulse = once(
        Weapon::Plasma,
        FloorArms::SteerToRidePoint,
        high,
        velocity,
        surface,
    );
    assert!(
        near(
            pulse.velocity,
            velocity - surface * (FALL_ACCELERATION * DT)
        ),
        "{:?}",
        pulse.velocity
    );
}

#[test]
fn under_the_launch_normal_a_missile_keeps_pulses_arms() {
    let velocity = Vec3::new(100.0, 0.0, 0.0);
    let pure = once(
        Weapon::Missile,
        FloorArms::LaunchNormal,
        ABOVE,
        velocity,
        Vec3::Y,
    );
    let pulse = once(
        Weapon::Missile,
        FloorArms::SteerToRidePoint,
        ABOVE,
        velocity,
        Vec3::Y,
    );
    assert_eq!(pure.position, pulse.position);
    assert_eq!(pure.velocity, pulse.velocity);
}
