//! The firing craft's box collider as a projectile's probe sees it
//! (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`, "The own-craft box is a rotated
//! frame"): the records the page prints, the reach by yaw, and the probe's fall and ride through
//! [`Projectiles::advance`].

use super::*;
use crate::projectile::{OwnCraftProbe, own_box_blocks};
use oag_core::math::{Mat3, Quat};
use oag_physics::Body;

const DT: f32 = 1.0 / 60.0;

/// A Venom's `<Misc>` before the 0.75 scale: the box is `(4.125, 2.625, 9.75)`.
fn venom() -> Dimensions {
    Dimensions {
        width: 5.5,
        height: 3.5,
        length: 13.0,
        ..Dimensions::default()
    }
}

/// A body whose recorded rows are `(-right, up, forward)` (`Basis::LeftUpForward`).
fn body_from_rows(rows: [[f32; 3]; 3], position: Vec3) -> Body {
    let left = Vec3::from(rows[0]);
    let up = Vec3::from(rows[1]);
    let forward = Vec3::from(rows[2]);
    let orientation = Quat::from_mat3(&Mat3::from_cols(-left, up, -forward)).normalize();
    Body {
        position,
        orientation,
        ..Body::default()
    }
}

struct Record {
    name: &'static str,
    rows: [[f32; 3]; 3],
    centre: [f32; 3],
    start: [f32; 3],
    normal: [f32; 3],
    length: f32,
    /// Whether the original's probe met the box (`0x7f`) and not the mesh.
    box_met: bool,
}

/// The page's table of discriminating records, three decimals. The mesh rows are the ones the
/// frame decides: the gate in the rows' own frame would call them blocked.
const RECORDS: [Record; 7] = [
    Record {
        name: "Talons Junction parked, Missile 3",
        rows: [
            [-0.879, -0.012, -0.477],
            [-0.004, 1.000, -0.018],
            [0.477, -0.014, -0.879],
        ],
        centre: [5.643, -50.083, -196.670],
        start: [6.655, -50.155, -198.533],
        normal: [-0.003, 1.000, -0.019],
        length: 12.0,
        box_met: true,
    },
    Record {
        name: "Talons Junction parked, Missile 4",
        rows: [
            [-0.879, -0.012, -0.477],
            [-0.004, 1.000, -0.018],
            [0.477, -0.014, -0.879],
        ],
        centre: [5.642, -50.083, -196.670],
        start: [6.995, -50.208, -199.161],
        normal: [-0.003, 1.000, -0.019],
        length: 12.0,
        box_met: false,
    },
    Record {
        name: "De Konstruct White, 112 units/s, Rocket 1",
        rows: [
            [0.897, -0.101, -0.431],
            [0.127, 0.991, 0.033],
            [0.424, -0.085, 0.902],
        ],
        centre: [84.827, -26.447, 70.351],
        start: [85.978, -26.678, 72.801],
        normal: [0.128, 0.991, 0.016],
        length: 6.0,
        box_met: true,
    },
    Record {
        name: "De Konstruct White, 112 units/s, Rocket 4",
        rows: [
            [0.896, -0.107, -0.431],
            [0.133, 0.991, 0.030],
            [0.424, -0.084, 0.902],
        ],
        centre: [85.621, -26.590, 72.051],
        start: [87.166, -26.930, 75.332],
        normal: [0.128, 0.991, 0.016],
        length: 6.0,
        box_met: false,
    },
    Record {
        name: "Moa Therma White bank, 139 units/s, Rocket 1",
        rows: [
            [0.064, -0.997, -0.032],
            [0.397, 0.055, -0.916],
            [0.916, 0.046, 0.399],
        ],
        centre: [251.772, 124.247, -349.078],
        start: [254.322, 124.351, -347.969],
        normal: [0.393, 0.054, -0.918],
        length: 6.0,
        box_met: false,
    },
    Record {
        name: "Moa Therma White bank, Missile 2",
        rows: [
            [0.065, -0.997, -0.032],
            [0.394, 0.055, -0.917],
            [0.917, 0.047, 0.397],
        ],
        centre: [253.130, 124.310, -348.487],
        start: [252.295, 124.236, -348.851],
        normal: [0.394, 0.053, -0.917],
        length: 12.0,
        box_met: true,
    },
    Record {
        name: "Outpost 7, 160 units/s, Plasma 1",
        rows: [
            [-0.184, -0.060, -0.981],
            [-0.462, 0.886, 0.032],
            [0.868, 0.459, -0.191],
        ],
        centre: [-1031.705, 7.034, 347.084],
        start: [-1028.295, 9.891, 345.924],
        normal: [-0.377, 0.926, -0.000],
        length: 12.0,
        box_met: true,
    },
];

/// Every record the original answered `0x7f` to, because the box was met first, is blocked here,
/// and every one it answered with the mesh is not. The mesh rows are the ones only the
/// transposed frame gets right: the Talons, De Konstruct and mag-strip starts pass the gate
/// in the box's own frame and would be hidden by the box.
#[test]
fn the_recorded_probes_meet_the_box_exactly_where_the_original_did() {
    for record in &RECORDS {
        let body = body_from_rows(record.rows, Vec3::from(record.centre));
        let start = Vec3::from(record.start);
        let end = start - Vec3::from(record.normal) * record.length;
        let blocked = own_box_blocks(&body, &venom(), start, end, None);
        assert_eq!(blocked, record.box_met, "{}", record.name);
    }
}

/// How far ahead of the centre a start at the centre's height is still hidden by the box, for a
/// craft yawed `degrees` off the world axes, probing straight down 12 units.
fn reach_ahead(degrees: f32) -> f32 {
    let orientation = Quat::from_rotation_y(degrees.to_radians());
    let body = Body {
        orientation,
        ..Body::default()
    };
    let forward = body.forward();
    let blocks = |ahead: f32| {
        let start = forward * ahead;
        own_box_blocks(&body, &venom(), start, start - Vec3::Y * 12.0, None)
    };
    let (mut hit, mut miss) = (0.0_f32, 12.0_f32);
    assert!(blocks(hit));
    for _ in 0..40 {
        let mid = 0.5 * (hit + miss);
        if blocks(mid) {
            hit = mid;
        } else {
            miss = mid;
        }
    }
    hit
}

#[test]
fn an_axis_aligned_craft_hides_a_start_to_the_end_of_its_box() {
    let reach = reach_ahead(0.0);
    assert!((reach - 4.875).abs() < 0.01, "reach {reach}");
    assert!((reach_ahead(90.0) - 4.875).abs() < 0.01);
}

#[test]
fn the_reach_shrinks_to_two_units_at_forty_five_degrees_of_yaw() {
    let reach = reach_ahead(45.0);
    assert!((reach - 2.06).abs() < 0.02, "reach {reach}");
    assert!(reach_ahead(15.0) > reach && reach_ahead(60.0) > reach);
}

/// One Rocket update over a flat floor at `y = 0`, fired by a craft at `(0, 1, 0)` with
/// `orientation`, the Rocket `ahead` units in front of it on the craft's heading, 3 above the
/// floor. Returns the Rocket's velocity after the update: parallel to the floor if it rode,
/// pulled down if the probe fell.
fn rocket_velocity_after_one_update(
    orientation: Quat,
    ahead: f32,
    own_craft: OwnCraftProbe,
) -> Vec3 {
    let mut ships = ships(&[(true, Vec3::new(0.0, 1.0, 0.0))]);
    ships[0].physics.body.orientation = orientation;
    ships[0].handling.dimensions = venom();
    let heading = ships[0].physics.body.forward();
    let mut projectiles = Projectiles::new();
    projectiles.slots[0] = Projectile {
        kind: Some(Weapon::Rocket),
        owner: 0,
        position: Vec3::new(0.0, 3.0, 0.0) + heading * ahead,
        velocity: Vec3::ZERO,
        surface: Vec3::Y,
        lifetime: MAX_FLIGHT_SECONDS - 0.1,
        ..Projectile::default()
    };
    projectiles.advance(
        DT,
        &floor_at_y(0.0),
        None,
        &ships,
        None,
        None,
        None,
        TriggerRadii::default(),
        "VENOM",
        FloorArms::SteerToRidePoint,
        own_craft,
    );
    projectiles.slots[0].velocity
}

#[test]
fn a_rocket_inside_the_box_falls_and_one_clear_of_it_rides() {
    let falls = rocket_velocity_after_one_update(Quat::IDENTITY, 3.9, OwnCraftProbe::CraftBox);
    assert!(falls.y < -0.5, "inside the box: {falls}");
    let rides = rocket_velocity_after_one_update(Quat::IDENTITY, 5.02, OwnCraftProbe::CraftBox);
    assert!(rides.y.abs() < 0.5, "clear of the box: {rides}");
}

/// The same start is inside the box of an aligned craft and clear of one turned 45 degrees,
/// whose gate box is turned the other way.
#[test]
fn the_reach_follows_the_craft_heading_not_a_forward_distance() {
    let yawed = Quat::from_rotation_y(45f32.to_radians());
    let aligned = rocket_velocity_after_one_update(Quat::IDENTITY, 3.0, OwnCraftProbe::CraftBox);
    let turned = rocket_velocity_after_one_update(yawed, 3.0, OwnCraftProbe::CraftBox);
    assert!(aligned.y < -0.5, "aligned: {aligned}");
    assert!(turned.y.abs() < 0.5, "45 degrees: {turned}");
}

/// A craft rolled 85 degrees on a mag strip: the gate sees the start `2.5` above the box's
/// half height, so the bolt rides at once where the same start from a level craft is hidden.
#[test]
fn a_bolt_fired_from_a_craft_on_the_mag_strip_rides_at_once() {
    let rolled = Quat::from_rotation_z(85f32.to_radians());
    let level = rocket_velocity_after_one_update(Quat::IDENTITY, 2.0, OwnCraftProbe::CraftBox);
    assert!(level.y < -0.5, "level: {level}");
    let banked = rocket_velocity_after_one_update(rolled, 2.0, OwnCraftProbe::CraftBox);
    assert!(banked.y.abs() < 0.5, "rolled 85 degrees: {banked}");
}

#[test]
fn without_the_measured_box_a_rocket_rides_from_the_first_update() {
    let velocity =
        rocket_velocity_after_one_update(Quat::IDENTITY, 3.9, OwnCraftProbe::PlasmaWindow(0.04));
    assert!(velocity.y.abs() < 0.5, "{velocity}");
}
