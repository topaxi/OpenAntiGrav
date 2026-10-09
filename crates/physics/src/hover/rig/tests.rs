//! What a [`Rig`] changes: the load it carries, and the pitch stiffness the derived rear hit
//! takes away.

use super::*;
use crate::collide::{CollisionWorld, Surface, TriangleSoup};
use crate::forces::Environment;
use crate::hover::evaluate;
use crate::params::Handling;
use crate::ship::{Body, ShipState};
use oag_core::math::Quat;

/// A rig shaped like HD's: four probes, `0.15` each, all cast, along the hit normal.
fn four_point() -> Rig {
    Rig {
        offsets: [
            Vec3::new(-1.5, -1.125, -4.5),
            Vec3::new(-1.5, -1.125, 4.5),
            Vec3::new(1.5, -1.125, -4.5),
            Vec3::new(1.5, -1.125, 4.5),
        ],
        count: 4,
        spring_share: 0.15,
        derive_rear: false,
        along_normal: true,
        normal_mean: NormalMean::QuarterSum,
    }
}

fn flat_floor() -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-500.0, 0.0, -500.0],
            [-500.0, 0.0, 500.0],
            [500.0, 0.0, 0.0],
        ],
        vec![[0, 1, 2]],
        Vec::new(),
        Surface::Floor,
        0,
    ));
    world
}

/// Round numbers, not a ship's: no handling data is reproduced in this repository.
fn handling() -> Handling {
    Handling {
        antigrav: crate::params::Antigrav {
            ride_height: 5.5,
            ..crate::params::Antigrav::default()
        },
        physical: crate::params::Physical {
            mass: 1.0,
            normal_gravity: 10.0,
            track_gravity: 70.0,
            ..crate::params::Physical::default()
        },
        ..Handling::ZERO
    }
}

const TARGET: f32 = 4.125;

/// A craft at `height` above a flat floor, pitched `pitch` radians about its right axis, flying
/// level along `-z` at `speed`.
fn craft(rig: Rig, height: f32, pitch: f32, speed: f32) -> ShipState {
    ShipState {
        body: Body {
            position: Vec3::new(0.0, height, 0.0),
            orientation: Quat::from_rotation_x(pitch),
            linear_velocity: Vec3::new(0.0, 0.0, -speed),
            ..Body::default()
        },
        grounded_prev: 1.0,
        hover_rig: rig,
        ..ShipState::default()
    }
}

/// The springs' summed force and their torque about the body's right axis.
fn load_and_pitch_torque(state: &ShipState) -> (Vec3, f32) {
    let hover = evaluate(
        state,
        &handling(),
        &Environment::default(),
        &flat_floor(),
        TARGET,
    );
    let mut force = Vec3::ZERO;
    let mut torque = Vec3::ZERO;
    for probe in hover.active() {
        force += probe.force;
        torque += (probe.point - state.body.position).cross(probe.force);
    }
    (force, torque.dot(state.body.right()))
}

#[test]
fn four_springs_at_0_15_carry_what_two_at_0_3_carry_on_level_ground() {
    let (two, _) = load_and_pitch_torque(&craft(Rig::TWO_POINT, 4.0, 0.0, 0.0));
    let (four, _) = load_and_pitch_torque(&craft(four_point(), 4.0, 0.0, 0.0));
    assert!(two.y > 0.0, "{two:?}");
    assert!(
        (four.y - two.y).abs() < 1e-3 * two.y,
        "{four:?} against {two:?}"
    );
}

#[test]
fn every_rig_reports_full_contact_as_grounded_one() {
    assert_eq!(Rig::TWO_POINT.grounded(2), 1.0);
    assert_eq!(Rig::TWO_POINT.grounded(1), 0.5);
    assert_eq!(four_point().grounded(4), 1.0);
    assert_eq!(four_point().grounded(1), 0.25);
    assert_eq!(four_point().grounded(3), 0.75);
}

/// **The pitch law HD differs by.** Above 50 units/s Pulse derives the rear hit with a flat
/// `6.0` slope gain where the probes are `9` apart, so its restoring torque per degree of pitch
/// is two thirds of what casting both probes gives; HD casts all four. Measured on RPCS3, HD
/// settles about 30% shallower than the two-point law (`docs/physics/hd-handling-ground-truth.md`).
#[test]
fn the_derived_rear_hit_leaves_two_thirds_of_the_cast_pitch_stiffness() {
    let pitch = 0.03;
    let speed = 86.0;
    let cast_two = Rig {
        derive_rear: false,
        ..Rig::TWO_POINT
    };
    let torque = |rig: Rig| {
        let (_, level) = load_and_pitch_torque(&craft(rig, 4.0, 0.0, speed));
        let (_, pitched) = load_and_pitch_torque(&craft(rig, 4.0, pitch, speed));
        (pitched - level) / pitch
    };
    let derived = torque(Rig::TWO_POINT);
    let cast = torque(cast_two);
    let hd = torque(four_point());

    assert!(cast.abs() > 0.0, "the cast pair restores at all: {cast}");
    let ratio = derived / cast;
    assert!((ratio - 2.0 / 3.0).abs() < 0.03, "derived / cast = {ratio}");
    let hd_ratio = hd / cast;
    assert!(
        (hd_ratio - 1.0).abs() < 0.1,
        "four-point / cast = {hd_ratio}"
    );
}

/// Below 50 units/s Pulse casts both probes too, so the two laws agree in pitch there.
#[test]
fn below_the_fast_probe_speed_the_two_point_law_casts_both_probes() {
    let pitch = 0.03;
    let derived = load_and_pitch_torque(&craft(Rig::TWO_POINT, 4.0, pitch, 20.0)).1;
    let cast = load_and_pitch_torque(&craft(
        Rig {
            derive_rear: false,
            ..Rig::TWO_POINT
        },
        4.0,
        pitch,
        20.0,
    ))
    .1;
    assert_eq!(derived, cast);
}
