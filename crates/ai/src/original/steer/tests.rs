use super::*;

/// A straight, level track along `-Z`, lateral `+X`, corridor `+/-10`.
fn point(z: f32) -> TrackPoint {
    TrackPoint {
        position: Vec3::new(0.0, 0.0, z),
        tangent: Vec3::NEG_Z,
        down: Vec3::NEG_Y,
        lateral: Vec3::X,
        bound_left: -10.0,
        bound_right: 10.0,
        racing_line: 0.0,
        progress: 0.0,
    }
}

fn pose(x: f32) -> Pose {
    Pose {
        position: Vec3::new(x, 2.0, 0.0),
        up: Vec3::Y,
        forward: Vec3::NEG_Z,
        left: Vec3::NEG_X,
        velocity: Vec3::new(0.0, 0.0, -100.0),
        angular_velocity: Vec3::ZERO,
    }
}

fn controller() -> Controller {
    Controller {
        steer_mul: 0.01,
        steer_damp: 0.0,
        look_ahead_secs: 0.5,
        xtrack_mul: 0.0,
    }
}

fn steer_at(x: f32, lane: f32, controller: &Controller, last: &mut f32) -> Steering {
    let own = point(0.0);
    let ahead = point(-60.0);
    steer(
        &pose(x),
        &SteerInput {
            own: &own,
            own_lane: lane,
            ahead: &ahead,
            ahead_lane: lane,
            air: 0.0,
        },
        controller,
        last,
    )
}

#[test]
fn a_lane_to_the_right_steers_right() {
    let out = steer_at(0.0, 0.3, &controller(), &mut 0.0);
    assert!(out.steer > 0.0, "{out:?}");
    let out = steer_at(0.0, -0.3, &controller(), &mut 0.0);
    assert!(out.steer < 0.0, "{out:?}");
}

#[test]
fn on_the_lane_and_straight_there_is_nothing_to_do() {
    assert!(steer_at(3.0, 0.3, &controller(), &mut 0.0).steer.abs() < 1e-3);
}

#[test]
fn steer_past_full_lock_becomes_the_airbrake_on_that_side() {
    let hard = Controller {
        steer_mul: 1.0,
        ..controller()
    };
    let right = steer_at(0.0, 0.8, &hard, &mut 0.0);
    assert_eq!(right.steer, 100.0);
    assert!(right.right > 0.0 && right.left == 0.0, "{right:?}");
    let left = steer_at(0.0, -0.8, &hard, &mut 0.0);
    assert_eq!(left.steer, -100.0);
    assert!(left.left > 0.0 && left.right == 0.0, "{left:?}");
}

#[test]
fn the_damping_reads_the_change_in_heading_error() {
    let damped = Controller {
        steer_damp: 2.0,
        ..controller()
    };
    let mut last = 0.0;
    let first = steer_at(0.0, 0.3, &damped, &mut last);
    let second = steer_at(0.0, 0.3, &damped, &mut last);
    assert!(first.steer > second.steer, "the step only kicks once");
    let undamped = steer_at(0.0, 0.3, &controller(), &mut 0.0);
    assert!((second.steer - undamped.steer).abs() < 1e-4);
}

#[test]
fn the_lookahead_is_at_least_fifteen_units_and_half_again_in_progress() {
    assert_eq!(lookahead_distance(0.0, 0.5), 22.5);
    assert_eq!(lookahead_distance(100.0, 0.5), 75.0);
}

#[test]
fn airborne_ramps_in_from_fifteen_units_over_the_surface() {
    let own = point(0.0);
    let mut high = pose(0.0);
    high.position.y = 20.0;
    assert_eq!(airborne(&pose(0.0), &own), 0.0);
    assert_eq!(airborne(&high, &own), 0.125);
}

#[test]
fn airborne_pitch_turns_the_nose_toward_the_tangent() {
    let mut own = point(0.0);
    own.tangent = Vec3::new(0.0, 0.2, -1.0).normalize();
    let mut last = Vec3::NEG_Z;
    let up = pitch(&pose(0.0), &own, 0.5, &mut last);
    assert!(up > 0.0, "nose below the tangent pitches up: {up}");
    own.tangent = Vec3::new(0.0, -0.2, -1.0).normalize();
    assert!(pitch(&pose(0.0), &own, 0.0, &mut last) < 0.0);
    assert_eq!(pitch(&pose(0.0), &own, 1.0, &mut last), 0.0, "grounded");
}

#[test]
fn a_silent_file_takes_the_executables_defaults() {
    let row = oag_tables::ai_control_stats::Controller {
        steer_mul: None,
        steer_damp: Some(0.3),
        look_ahead_secs: None,
        xtrack_mul: None,
        xtrack_max: None,
        xtrack_damp: None,
    };
    assert_eq!(
        Controller::from_disc(&row),
        Controller {
            steer_mul: 1.0,
            steer_damp: 0.3,
            look_ahead_secs: 0.5,
            xtrack_mul: 0.0
        }
    );
}
