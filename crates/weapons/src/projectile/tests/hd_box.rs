//! HD's own-craft box, probe lengths and open-air Rocket against records read off the original
//! (`docs/ghidra/functions/ps3-hdfury-eu/own-craft-box.md`, "Measured live").

use super::craft_box::body_from_rows;
use super::*;
use crate::projectile::{
    FlightLaws, OwnCraftProbe, PlasmaFall, ProbeLengths, RocketOpenAir, own_box_blocks,
};

const DT: f32 = 1.0 / 60.0;

const HD: FlightLaws = FlightLaws {
    floor_arms: FloorArms::ParallelToFloor,
    probes: ProbeLengths {
        rocket: 6.0,
        missile: 6.0,
        plasma: 6.0,
        shuriken: 12.0,
    },
    rocket_open_air: RocketOpenAir::TurnsAlongNormal,
    plasma_fall: PlasmaFall::WorldY,
};

/// A Feisar's `<Misc>` before the 0.75 scale: HD's live collider is `(3.75, 2.625, 9.75)`.
fn feisar() -> Dimensions {
    Dimensions {
        width: 5.0,
        height: 3.5,
        length: 13.0,
        ..Dimensions::default()
    }
}

/// A segment the original ran against the firing craft's box, and whether it answered.
struct Record {
    name: &'static str,
    rows: [[f32; 3]; 3],
    centre: [f32; 3],
    start: [f32; 3],
    end: [f32; 3],
    box_met: bool,
}

/// Talon's Junction rows for the first parked Rocket volley.
const GRID: [[f32; 3]; 3] = [
    [-0.2220, -0.0076, -0.9750],
    [0.0016, 1.0000, -0.0082],
    [0.9751, -0.0034, -0.2219],
];

const RECORDS: [Record; 3] = [
    Record {
        name: "Rocket volley 1, call 5: the probe the box answered",
        rows: GRID,
        centre: [-131.8576, -49.6018, -175.5146],
        start: [-130.2804, -50.1616, -173.9537],
        end: [-130.2996, -56.1614, -173.9047],
        box_met: true,
    },
    Record {
        name: "Rocket volley 1, call 3: the probe on the other side, clear",
        rows: GRID,
        centre: [-131.8576, -49.6018, -175.5146],
        start: [-131.0865, -50.1153, -177.6264],
        end: [-131.1057, -56.1151, -177.5774],
        box_met: false,
    },
    Record {
        name: "Plasma sweep, yaw 76: clear only through the transposed gate",
        rows: [
            [0.9519, 0.2181, -0.2151],
            [-0.1881, 0.9704, 0.1512],
            [0.2417, -0.1035, 0.9648],
        ],
        centre: [379.2689, -33.2277, -143.0654],
        start: [380.5079, -33.8264, -138.5839],
        end: [380.715, -33.7689, -137.6788],
        box_met: false,
    },
];

/// The box the engine builds from a body and `<Misc>` answers HD's recorded segments as the
/// original's did.
#[test]
fn the_box_answers_hds_recorded_segments_as_the_original_did() {
    for record in &RECORDS {
        let body = body_from_rows(record.rows, Vec3::from(record.centre));
        let blocked = own_box_blocks(
            &body,
            &feisar(),
            Vec3::from(record.start),
            Vec3::from(record.end),
            None,
        );
        assert_eq!(blocked, record.box_met, "{}", record.name);
    }
}

/// One update of one projectile over a flat floor at `y = 0`, no craft to hide it.
fn once(laws: FlightLaws, kind: Weapon, position: Vec3, velocity: Vec3, surface: Vec3) -> Vec3 {
    let mut projectiles = Projectiles::new();
    projectiles.slots[0] = Projectile {
        kind: Some(kind),
        position,
        velocity,
        surface,
        lifetime: MAX_FLIGHT_SECONDS - 0.1,
        launch_speed_kmh: 800.0,
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
        laws,
        OwnCraftProbe::NoBox,
        crate::projectile::LaunchScale::Unscaled,
    );
    assert!(impacts[0].is_none(), "{:?}", impacts[0]);
    projectiles.slots[0].velocity
}

/// HD's `Rocket_Update` on an update the box held: the velocity read at its probe and at its
/// sweep. It turned parallel to the carried normal and did not fall.
#[test]
fn an_hd_rocket_in_open_air_turns_along_its_carried_normal_and_does_not_fall() {
    let before = Vec3::new(-176.9259, -69.6160, 115.0372);
    let normal = Vec3::new(-0.3418, 0.9397, -0.0095);
    let after = Vec3::new(-179.0093, -63.9380, 114.9898);
    let high = Vec3::new(0.0, 500.0, 0.0);
    let turned = once(HD, Weapon::Rocket, high, before, normal);
    assert!(
        turned.distance(after) < 0.1,
        "ours {turned}, original {after}"
    );
    let pulse = once(
        FloorArms::ParallelToFloor.into(),
        Weapon::Rocket,
        high,
        before,
        normal,
    );
    assert!(
        (pulse.y - (before.y - 50.0 * DT)).abs() < 0.01,
        "Pulse's arm falls: {pulse}"
    );
}

/// HD's `Missile_Update` on an update its probe found nothing: the velocity read at its probe and
/// at its sweep. It fell on world Y, not along the carried normal half a degree off it
/// (`docs/ghidra/functions/ps3-hdfury-eu/open-air-arms.md`, "Measured live").
#[test]
fn an_hd_missile_in_open_air_falls_on_world_y_as_recorded() {
    let before = Vec3::new(64.4875, -2.7333, -2.2110);
    let normal = Vec3::new(0.003193, 0.999962, -0.008093);
    let after = Vec3::new(64.4875, -3.5666, -2.2110);
    let high = Vec3::new(0.0, 500.0, 0.0);
    let ours = once(HD, Weapon::Missile, high, before, normal);
    assert!(
        ours.distance(after) < 0.002,
        "ours {ours}, original {after}"
    );
}

/// HD's `Shuriken_Update` reads as the same fall on world Y. It never runs, as HD builds no
/// Shuriken, so this holds ours to the static read (`open-air-arms.md`).
#[test]
fn an_hd_shuriken_in_open_air_falls_on_world_y() {
    let before = Vec3::new(0.0, 0.0, -40.0);
    let banked = Vec3::new(0.6, 0.8, 0.0);
    let ours = once(
        HD,
        Weapon::Shuriken,
        Vec3::new(0.0, 500.0, 0.0),
        before,
        banked,
    );
    let fallen = before - Vec3::Y * (FALL_ACCELERATION * DT);
    assert!(ours.distance(fallen) < 0.002, "ours {ours}, read {fallen}");
}

/// The Missile and Plasma probe `6.0` on HD and `12.0` on Pulse: eight above the floor, HD's
/// finds nothing and the bolt falls, Pulse's rides.
#[test]
fn an_hd_probe_reaches_six_where_pulses_reaches_twelve() {
    for kind in [Weapon::Missile, Weapon::Plasma] {
        let start = Vec3::new(0.0, 8.0, 0.0);
        let level = Vec3::new(0.0, 0.0, -40.0);
        let hd = once(HD, kind, start, level, Vec3::Y);
        assert!(hd.y < -0.5, "{kind:?} past HD's probe: {hd}");
        let pulse = once(
            FloorArms::ParallelToFloor.into(),
            kind,
            start,
            level,
            Vec3::Y,
        );
        assert!(
            pulse.y.abs() < 1e-3,
            "{kind:?} within Pulse's probe: {pulse}"
        );
    }
}
