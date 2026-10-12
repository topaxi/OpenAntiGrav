use super::*;
use crate::original::thrust::tests_table;

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

fn tick(z: f32) -> Tick {
    Tick {
        pose: Pose {
            position: Vec3::new(1.0, 3.0, z),
            up: Vec3::Y,
            forward: Vec3::NEG_Z,
            left: Vec3::NEG_X,
            velocity: Vec3::new(0.0, 0.0, -120.0),
            angular_velocity: Vec3::ZERO,
        },
        own: point(z),
        band: None,
        speed_floor: None,
        grounded: 1.0,
        contact_sides: 0,
        touching: false,
        finished: false,
        race_fraction: 0.1,
    }
}

fn field() -> Field {
    let controller = Controller {
        steer_mul: 0.01,
        steer_damp: 0.0,
        look_ahead_secs: 0.5,
        xtrack_mul: 0.0,
    };
    Field::new(tests_table(), controller, 1.0, 1, 42)
}

const RACERS: [(f32, u8); 3] = [(500.0, 3), (520.0, 2), (540.0, 1)];

fn standings() -> Standings<'static> {
    Standings {
        racers: &RACERS,
        player: 0,
        player_rubber_band_off: false,
        race_time: 30.0,
    }
}

fn run(field: &mut Field, ticks: usize) -> Vec<Output> {
    (0..ticks)
        .map(|_| {
            let lane = field.begin_tick(&[1, 2]);
            field.drive(1, lane == 1, &tick(-520.0), &standings(), 1.0 / 60.0, |d| {
                Some(point(-520.0 - d))
            })
        })
        .collect()
}

#[test]
fn the_round_robin_runs_one_slot_a_tick_with_period_eight() {
    let mut field = field();
    let slots: Vec<usize> = (0..17).map(|_| field.begin_tick(&[1, 2])).collect();
    assert_eq!(slots, [0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0]);
}

#[test]
fn the_shuffle_runs_once_and_arms_the_wander() {
    let mut field = field();
    field.begin_tick(&[1, 2]);
    assert_eq!(field.drivers[1].wander.segments, 2.0);
    assert_eq!(
        field.drivers[0].wander,
        Wander::constructed(0),
        "the player's is untouched"
    );
    let after = field.drivers[1].wander;
    field.begin_tick(&[1, 2]);
    assert_eq!(field.drivers[1].wander, after);
}

#[test]
fn thrust_is_set_on_the_drivers_own_tick_and_held_between() {
    let outputs = run(&mut field(), 20);
    assert_eq!(
        outputs[0].controls.thrust, 0.0,
        "nothing before its first turn"
    );
    let set = outputs[1].controls.thrust;
    assert!(set > 0.0);
    assert!(outputs[1..9].iter().all(|o| o.controls.thrust == set));
}

#[test]
fn the_steering_authority_ramps_in_over_forty_ticks() {
    let mut field = field();
    run(&mut field, 39);
    assert!(field.drivers[1].authority < 1.0);
    run(&mut field, 2);
    assert_eq!(field.drivers[1].authority, 1.0);
}

#[test]
fn a_speed_floor_forces_full_throttle_below_its_speed() {
    let mut field = field();
    field.begin_tick(&[1, 2]);
    field.begin_tick(&[1, 2]);
    let mut slow = Tick {
        speed_floor: Some(0),
        ..tick(-520.0)
    };
    slow.pose.velocity.z = -60.0;
    let out = field.drive(1, true, &slow, &standings(), 1.0 / 60.0, |d| {
        Some(point(-520.0 - d))
    });
    assert_eq!(out.controls.thrust, 1.0, "60 u/s is under the floor's 95");
    let out = field.drive(1, true, &tick(-520.0), &standings(), 1.0 / 60.0, |d| {
        Some(point(-520.0 - d))
    });
    assert!(
        out.controls.thrust < 1.0,
        "120 u/s is over it, the law's own throttle"
    );
}

#[test]
fn a_field_replays_identically_from_its_seed() {
    assert_eq!(run(&mut field(), 300), run(&mut field(), 300));
}
