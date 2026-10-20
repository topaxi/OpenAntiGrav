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
        sum_escapes: true,
        probe_escape: true,
        spring: SpringLaw::Racing,
        align_projects_pitch_out: true,
        downforce_scaled: true,
        snap: 0.0,
        brakes_off: false,
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

/// **HD sums its escapes.** A level craft whose four probes sit `0.6` above the floor (the body
/// at `1.725`) escapes `0.4` per probe: `1.6` in all on HD's rig, `0.4` taking the largest alone.
#[test]
fn a_rig_that_sums_escapes_moves_the_body_by_every_penetrating_probe() {
    let escape = |rig: Rig| {
        evaluate(
            &craft(rig, 1.725, 0.0, 0.0),
            &handling(),
            &Environment::default(),
            &flat_floor(),
            TARGET,
        )
        .escape
        .y
    };
    let summed = escape(four_point());
    let largest = escape(Rig {
        sum_escapes: false,
        ..four_point()
    });
    assert!((summed - 1.6).abs() < 1e-4, "{summed}");
    assert!((largest - 0.4).abs() < 1e-4, "{largest}");
}

/// A rig shaped like Pulse's Zone hover, `Ship_HoverFourCorner`: four level probes, `0.15`
/// each, all cast, the springs along up, and no escape in the probe loop.
fn four_corner() -> Rig {
    Rig {
        offsets: [
            Vec3::new(-1.875, 0.0, -3.75),
            Vec3::new(1.875, 0.0, -3.75),
            Vec3::new(-1.875, 0.0, 3.75),
            Vec3::new(1.875, 0.0, 3.75),
        ],
        count: 4,
        spring_share: 0.15,
        derive_rear: false,
        along_normal: false,
        normal_mean: NormalMean::Mean,
        sum_escapes: false,
        probe_escape: false,
        spring: SpringLaw::FourCorner,
        align_projects_pitch_out: false,
        downforce_scaled: false,
        snap: crate::hover::SNAP_GAIN,
        brakes_off: true,
    }
}

/// **Zone's probes do not escape.** The same penetrating craft that the racing law's rig pushes
/// out by `0.4` stays where it is under the four-corner law, whose loop moves nothing.
#[test]
fn the_four_corner_rig_has_no_escape_in_its_probe_loop() {
    let escape = |rig: Rig| {
        evaluate(
            &craft(rig, 0.6, 0.0, 0.0),
            &handling(),
            &Environment::default(),
            &flat_floor(),
            TARGET,
        )
        .escape
    };
    assert_eq!(escape(four_corner()), Vec3::ZERO);
    let with_escape = escape(Rig {
        probe_escape: true,
        ..four_corner()
    });
    assert!((with_escape.y - 0.4).abs() < 1e-4, "{with_escape:?}");
}

/// **Zone's spring carries no load factor, no rebound and no magnet-lock fade**, and its damper
/// is a fixed `0.8` clamped at `1`: `((((t - h) * K) * g) * 0.15) * m * (1 + 0.8 * clamp(-0.1 vn,
/// -1, 1))`. The same probe under the racing law moves with every one of those.
#[test]
fn the_four_corner_spring_is_its_own_law() {
    let probe = |rig: Rig, grounded_prev: f32, mag_lock: f32, sink: f32| {
        let mut state = craft(rig, 3.0, 0.0, 0.0);
        state.grounded_prev = grounded_prev;
        state.mag_lock_blend = mag_lock;
        state.body.linear_velocity = Vec3::new(0.0, -sink, 0.0);
        evaluate(
            &state,
            &handling(),
            &Environment::default(),
            &flat_floor(),
            TARGET,
        )
        .probes[0]
            .force
    };
    // h = 3.0 with level probes: (4.125 - 3.0) * 4/3 * 80 * 0.15 * 1.0.
    let spring = (TARGET - 3.0) * crate::hover::HOVER_K * 80.0 * 0.15 * 1.0;
    let at_rest = probe(four_corner(), 1.0, 0.0, 0.0);
    assert_eq!(at_rest, Vec3::new(0.0, spring, 0.0));
    assert_eq!(
        probe(four_corner(), 0.0, 0.0, 0.0),
        at_rest,
        "no load factor"
    );
    assert_eq!(
        probe(four_corner(), 1.0, 0.5, 0.0),
        at_rest,
        "no magnet-lock fade"
    );
    // Closing at 5 units/s: 1 + 0.8 * 0.5; at 30 units/s the clamp holds it at 1 + 0.8.
    let closing = probe(four_corner(), 1.0, 0.0, 5.0).y;
    assert!(
        (closing / spring - 1.4).abs() < 1e-5,
        "{}",
        closing / spring
    );
    let fast = probe(four_corner(), 1.0, 0.0, 30.0).y;
    assert!((fast / spring - 1.8).abs() < 1e-5, "{}", fast / spring);

    let racing = Rig {
        spring: SpringLaw::Racing,
        ..four_corner()
    };
    assert_ne!(probe(racing, 0.0, 0.0, 0.0), probe(racing, 1.0, 0.0, 0.0));
}

/// **Zone's alignment torque pitches.** A craft pitched over a flat floor gets
/// `-400 * (up x n)`, whose component about the right axis the racing law projects out and the
/// four-corner law keeps. Its normal is `sum * (1 / n)`, here the floor's own.
#[test]
fn the_four_corner_alignment_torque_keeps_its_pitch_component() {
    let torque = |rig: Rig| {
        let state = craft(rig, 3.0, 0.05, 0.0);
        let hover = evaluate(
            &state,
            &handling(),
            &Environment::default(),
            &flat_floor(),
            TARGET,
        );
        assert_eq!(hover.contacts, 4);
        assert_eq!(hover.average_normal, Vec3::new(0.0, 1.0, 0.0));
        let up = state.body.up();
        let pitch = hover.alignment_torque.dot(state.body.right()).abs();
        if rig.align_projects_pitch_out {
            assert!(pitch < 1e-4, "{pitch}");
        } else {
            assert!(pitch > 1.0, "{pitch}");
        }
        (hover.alignment_torque, up)
    };
    let (zone, up) = torque(four_corner());
    assert_eq!(
        zone,
        up.cross(Vec3::new(0.0, 1.0, 0.0)) * crate::hover::ALIGNMENT_GAIN
    );
    torque(Rig {
        align_projects_pitch_out: true,
        ..four_corner()
    });
}

/// `sum * (1 / n)` is not normalised: two unit normals 90 degrees apart average to length
/// `sqrt(0.5)`, where the racing law's mean is unit length.
#[test]
fn the_plain_mean_normal_is_not_normalised() {
    let two = Vec3::new(1.0, 0.0, 0.0) + Vec3::new(0.0, 1.0, 0.0);
    let mean = two * (1.0 / 2.0_f32);
    assert!((mean.length() - 0.5_f32.sqrt()).abs() < 1e-6);
    assert_eq!(NormalMean::Mean, four_corner().normal_mean);
}

/// **Zone's downforce is the whole `track_gravity * mass` on any contact.** With two of four
/// probes down (a craft lifted and pitched so its rear pair touches) the racing law's downforce
/// is half, the four-corner law's is all of it: read live on two-contact frames,
/// `-80 * 1 * n` with `craft+0x2b0 = 0.5`.
#[test]
fn the_four_corner_downforce_takes_no_grounded_factor() {
    let downforce = |rig: Rig| {
        let mut state = craft(rig, 3.8, 0.3, 0.0);
        state.mag_lock_blend = 0.5;
        let hover = evaluate(
            &state,
            &handling(),
            &Environment::default(),
            &flat_floor(),
            TARGET,
        );
        assert_eq!(hover.contacts, 2);
        hover.downforce.y
    };
    assert_eq!(downforce(four_corner()), -70.0);
    let scaled = downforce(Rig {
        downforce_scaled: true,
        ..four_corner()
    });
    assert_eq!(scaled, -70.0 * 0.5 * 0.5);
}

/// **Zone pulls a high craft down.** Its centre `5.0` above a flat floor, every level probe out
/// of reach, the four-corner law moves the body `(3 - 5) * 0.25 = -0.5` along the floor's normal
/// that frame; at `2.9` (inside `3.0`) or `10.5` (past the ray) it moves nothing.
#[test]
fn the_four_corner_snap_pulls_a_quarter_of_the_way_to_three() {
    let escape = |rig: Rig, height: f32| {
        let hover = evaluate(
            &craft(rig, height, 0.0, 0.0),
            &handling(),
            &Environment::default(),
            &flat_floor(),
            TARGET,
        );
        hover.escape
    };
    assert_eq!(escape(four_corner(), 5.0), Vec3::new(0.0, -0.5, 0.0));
    assert_eq!(escape(four_corner(), 2.9), Vec3::ZERO);
    assert_eq!(escape(four_corner(), 10.5), Vec3::ZERO);
    let off = Rig {
        snap: 0.0,
        ..four_corner()
    };
    assert_eq!(escape(off, 5.0), Vec3::ZERO);
}

/// **HD's Zone ray closes half the gap.** The same ray, reach and floor with `0.5` for Pulse's
/// `0.25` (`0x008a918c`): from `5.0` above a flat floor the body moves `(3 - 5) * 0.5 = -1.0`.
#[test]
fn hds_zone_ray_pulls_half_of_the_way_to_three() {
    let escape = |rig: Rig, height: f32| {
        evaluate(
            &craft(rig, height, 0.0, 0.0),
            &handling(),
            &Environment::default(),
            &flat_floor(),
            TARGET,
        )
        .escape
    };
    let half = Rig {
        snap: 0.5,
        ..four_corner()
    };
    assert_eq!(escape(half, 5.0), Vec3::new(0.0, -1.0, 0.0));
    assert_eq!(escape(half, 2.9), Vec3::ZERO);
}
