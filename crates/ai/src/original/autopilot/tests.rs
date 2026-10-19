use super::*;
use crate::original::thrust::tests_table;
use crate::original::{Field, Pose, Standings};
use oag_core::math::Vec3;

fn point(z: f32) -> TrackPoint {
    TrackPoint {
        position: Vec3::new(0.0, 0.0, z),
        tangent: Vec3::NEG_Z,
        down: Vec3::NEG_Y,
        lateral: Vec3::X,
        bound_left: -10.0,
        bound_right: 10.0,
        racing_line: 0.0,
        progress: (-z / 5000.0).rem_euclid(1.0),
    }
}

/// A craft `x` units right of the racing line, flying straight down the track.
fn tick(x: f32, touching: bool, contact_sides: i32) -> Tick {
    Tick {
        pose: Pose {
            position: Vec3::new(x, 3.0, -520.0),
            up: Vec3::Y,
            forward: Vec3::NEG_Z,
            left: Vec3::NEG_X,
            velocity: Vec3::new(0.0, 0.0, -120.0),
            angular_velocity: Vec3::ZERO,
        },
        own: point(-520.0),
        band: None,
        speed_floor: None,
        grounded: 1.0,
        contact_sides,
        touching,
        finished: false,
        race_fraction: 0.1,
        lap_distance: 520.0,
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

fn autopilot(ringless: bool) -> Autopilot {
    Autopilot::new(controller(), Laws::PULSE, ringless, 7)
}

fn drive(autopilot: &mut Autopilot, tick: &Tick) -> Output {
    autopilot.drive(tick, 1.0 / 60.0, |d| Some(point(-520.0 - d)))
}

#[test]
fn the_player_holds_full_throttle_where_an_opponent_flies_the_law() {
    let mut player = autopilot(false);
    let mut field = Field::new(
        tests_table(),
        controller(),
        Laws::PULSE,
        1.0,
        1,
        [8, 1, 2, 3, 4, 5, 6, 7],
        42,
    );
    let racers = [(500.0, 3), (520.0, 2), (540.0, 1)];
    let standings = Standings {
        racers: &racers,
        player: 0,
        player_rubber_band_off: false,
        race_time: 30.0,
        eliminator: false,
        lap_length: 5000.0,
    };
    let mut opponent = 0.0;
    for _ in 0..16 {
        let lane = field.begin_tick(&[1, 2]);
        let out = field.drive(
            1,
            lane == 1,
            &tick(0.0, false, 0),
            &standings,
            1.0 / 60.0,
            |d| Some(point(-520.0 - d)),
        );
        opponent = out.controls.thrust;
        let out = drive(&mut player, &tick(0.0, false, 0));
        assert_eq!(out.controls.thrust, 1.0);
    }
    assert!(
        (opponent - 1.0).abs() > 1e-3,
        "the opponent flies the law, {opponent}"
    );
}

#[test]
fn the_player_lane_stays_on_the_racing_line_under_a_contact_push() {
    let mut player = autopilot(false);
    for _ in 0..200 {
        drive(&mut player, &tick(4.0, true, 3));
        assert_eq!(player.driver.planner.target, 0.0);
    }
    assert_eq!(player.driver.planner.ring.at(0.104), 0.0);
}

#[test]
fn the_player_rail_keeps_its_hold_through_a_contact() {
    let mut player = autopilot(false);
    let mut last = None;
    for _ in 0..120 {
        last = drive(&mut player, &tick(4.0, true, 0)).rail;
    }
    let rail = last.expect("the rail fires through a contact for the player");
    assert!(rail.position.x < 4.0, "pulled toward the line: {rail:?}");
}

#[test]
fn a_stint_starts_warm() {
    let mut player = autopilot(false);
    let out = drive(&mut player, &tick(0.0, false, 0));
    assert_eq!(player.driver.authority, 1.0);
    assert!((player.driver.smoothed_speed - 120.0).abs() < 1e-3);
    // Straight down the line: no derivative kick on the first tick.
    assert!(out.controls.steer_x.abs() < 1e-3, "{:?}", out.controls);
}

#[test]
fn pures_player_lane_closes_on_the_racing_line() {
    let mut player = autopilot(true);
    drive(&mut player, &tick(4.0, false, 0));
    assert_eq!(player.driver.single_lane.target, 0.0);
    let lane = player.driver.single_lane.lane;
    assert!(lane.abs() < 1e-3, "{lane}");
}

#[test]
fn a_release_rewarms_the_next_stint() {
    let mut player = autopilot(false);
    drive(&mut player, &tick(0.0, false, 0));
    player.driver.authority = 0.2;
    player.release();
    drive(&mut player, &tick(0.0, false, 0));
    assert_eq!(player.driver.authority, 1.0);
}
