//! What the replay harness in [`super`] is asserted to do: seeding a run from
//! a recording's first row, the three input sources, the readings of a
//! recorded basis, and the reseeding windows.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `replay.rs`: the tests are 808 lines, past the 200 an inline test module
//! may hold. See `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;
use crate::compare::{Field, Tolerances, compare};
use oag_physics::CollisionWorld;

/// How heavy a test ship is, and why.
///
/// **There is no force-free configuration of the recovered force law.** Two of
/// its terms are constants rather than handling parameters - rolling
/// resistance is a flat `2.0` opposing motion
/// ([`oag_physics::passive::ROLLING_RESISTANCE`]) and the airborne quadratic
/// drag coefficient is `-0.002` - so an all-zero parameter set still
/// decelerates: at mass 1 that is 2 units/s^2, which is a third of a slow
/// ship's velocity every tick. They are *forces*, so mass is the only dial
/// that makes them negligible, and a ship this heavy accelerates at 2e-6
/// units/s^2: nothing over a two-second run.
///
/// This is a property of the harness's fixtures, not a claim about any ship.
const TEST_MASS: f32 = 1e6;

/// A parameter set that is inert except for mass, so a replay is as close to a
/// free body as the force law allows. The real values are read off the
/// player's own disc.
fn inert_handling() -> Handling {
    let mut handling = Handling::ZERO;
    handling.physical.mass = TEST_MASS;
    handling
}

fn frame(position: Vec3, velocity: Vec3) -> Frame {
    Frame {
        position,
        velocity,
        speed: velocity.length(),
        speed_cached: velocity.length(),
        ..Frame::default()
    }
}

/// A straight coast at constant velocity, in the recording's own conventions.
///
/// A recorded basis is positively oriented - `cross(row0, up) = forward` - so
/// a ship aimed along `+z` with up `+y` records row 0 along `+x`, and under
/// the measured reading that row 0 is the ship's **left**, making the body's
/// right `-x`. Getting this fixture wrong is the same mistake as getting the
/// reading wrong, so it is spelled out rather than eyeballed.
fn coasting(ticks: usize, dt: f32, velocity: Vec3) -> Trace {
    Trace {
        frames: (0..ticks)
            .map(|tick| Frame {
                tick: tick as u64,
                dt,
                row0: Vec3::X,
                up: Vec3::Y,
                forward: Vec3::Z,
                ..frame(velocity * (tick as f32 * dt), velocity)
            })
            .collect(),
    }
}

#[test]
fn the_fixture_basis_is_positively_oriented_like_a_real_one() {
    let frame = coasting(1, 1.0 / 60.0, Vec3::ZERO).frames[0];
    assert!(frame.basis_is_positively_oriented(1e-6));
}

#[test]
fn the_recorded_basis_becomes_the_body_s_orientation() {
    let recorded = coasting(1, 1.0 / 60.0, Vec3::ZERO).frames[0];
    let orientation = orientation_of(&recorded, Basis::LeftUpForward);
    // Row 0 is the ship's left, so the body's right is -x; row 2 is the nose,
    // so the body's forward - which is its own -z - is +z.
    assert!((orientation * Vec3::X - Vec3::NEG_X).length() < 1e-6);
    assert!((orientation * Vec3::Y - Vec3::Y).length() < 1e-6);
    assert!((orientation * Vec3::NEG_Z - Vec3::Z).length() < 1e-6);
}

/// Both readings of the basis must be rotations. One flipped row would be a
/// reflection, which `Quat::from_mat3` turns into silent nonsense rather than
/// an error - so the alternative reading flips two rows, and this is what says
/// so in arithmetic.
#[test]
fn both_readings_of_the_basis_are_proper_rotations() {
    let recorded = coasting(1, 1.0 / 60.0, Vec3::ZERO).frames[0];
    for basis in [Basis::LeftUpForward, Basis::RightUpBack] {
        let q = orientation_of(&recorded, basis);
        let (x, y, z) = (q * Vec3::X, q * Vec3::Y, q * Vec3::Z);
        assert!(
            (x.cross(y) - z).length() < 1e-5,
            "{basis:?} is a reflection"
        );
        assert!((q.length() - 1.0).abs() < 1e-6);
    }
}

/// The two readings disagree about which way the ship faces, which is the
/// whole point of being able to run a comparison both ways.
#[test]
fn the_two_readings_of_the_basis_face_opposite_ways() {
    let recorded = coasting(1, 1.0 / 60.0, Vec3::ZERO).frames[0];
    let measured = orientation_of(&recorded, Basis::LeftUpForward) * Vec3::NEG_Z;
    let other = orientation_of(&recorded, Basis::RightUpBack) * Vec3::NEG_Z;
    assert!((measured + other).length() < 1e-6, "{measured} vs {other}");
}

/// The node stores the camera's rotation transposed - its world axes are
/// the recorded matrix's columns, look along the negated third one. This
/// fixture builds the stored rows from a known camera the way the
/// measured convention says the game does, and the reading must recover
/// that camera's look and up exactly.
#[test]
fn a_camera_pose_reads_the_transposed_store_back_into_look_and_up() {
    let yaw = 0.37f32;
    let (sin, cos) = yaw.sin_cos();
    let look = Vec3::new(sin, -0.2, cos).normalize();
    let back = -look;
    let up = (Vec3::Y - back * Vec3::Y.dot(back)).normalize();
    let right = up.cross(back);
    // Stored rows are the transpose of the camera's world-axis columns
    // (right, up, back).
    let frame = Frame {
        camera_row0: Some(Vec3::new(right.x, up.x, back.x)),
        camera_up: Some(Vec3::new(right.y, up.y, back.y)),
        camera_forward: Some(Vec3::new(right.z, up.z, back.z)),
        camera_position: Some(Vec3::new(1.0, 2.0, 3.0)),
        ..Frame::default()
    };
    let q = camera_orientation_of(&frame).expect("a full pose");
    assert!((q * Vec3::NEG_Z - look).length() < 1e-6);
    assert!((q * Vec3::Y - up).length() < 1e-6);
    let (x, y, z) = (q * Vec3::X, q * Vec3::Y, q * Vec3::Z);
    assert!(
        (x.cross(y) - z).length() < 1e-5,
        "a reflection, not a rotation"
    );
}

#[test]
fn a_capture_without_a_camera_has_no_camera_orientation() {
    assert_eq!(camera_orientation_of(&Frame::default()), None);
}

#[test]
fn a_frame_round_trips_through_a_reading_of_the_basis() {
    for basis in [Basis::LeftUpForward, Basis::RightUpBack] {
        let (right, up, forward) = basis.to_body(Vec3::X, Vec3::Y, Vec3::Z);
        assert_eq!(
            basis.to_rows(right, up, forward),
            (Vec3::X, Vec3::Y, Vec3::Z)
        );
    }
}

#[test]
fn a_replay_starts_exactly_where_the_recording_starts() {
    let recorded = coasting(8, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
    let simulated = replay(
        &recorded,
        &inert_handling(),
        &Environment::default(),
        &CollisionWorld::new(),
        &Options::default(),
    );
    assert_eq!(simulated.len(), recorded.len());
    assert_eq!(simulated.frames[0].position, recorded.frames[0].position);
    assert_eq!(simulated.frames[0].velocity, recorded.frames[0].velocity);
    assert_eq!(simulated.frames[0].tick, recorded.frames[0].tick);
    assert!((simulated.frames[0].row0 - recorded.frames[0].row0).length() < 1e-6);
}

/// A near-free body is the harness's own self-check: if *this* diverges, the
/// bug is in the harness rather than in the physics. See [`TEST_MASS`] for why
/// "near".
#[test]
fn a_coasting_ship_tracks_a_straight_line_recording() {
    let recorded = coasting(120, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
    let simulated = replay(
        &recorded,
        &inert_handling(),
        &Environment::default(),
        &CollisionWorld::new(),
        &Options::default(),
    );
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    assert!(!comparison.diverged(), "{comparison}");
}

/// `speed_cached` is the previous tick's *forward-projected* speed, not the
/// previous tick's speed. A straight-line capture cannot tell those apart, so
/// this fixture puts the velocity across the nose, where the magnitude reading
/// would emit 22 and the projection emits nothing.
#[test]
fn speed_cached_is_the_forward_projection_and_not_the_magnitude() {
    let across = Vec3::new(22.0, 0.0, 0.0);
    // Aimed along +z (row 2 is the nose) while travelling along +x.
    let recorded = Trace {
        frames: (0..4)
            .map(|tick| Frame {
                tick,
                dt: 1.0 / 60.0,
                row0: Vec3::X,
                up: Vec3::Y,
                forward: Vec3::Z,
                ..frame(across * (tick as f32 / 60.0), across)
            })
            .collect(),
    };
    let simulated = replay(
        &recorded,
        &inert_handling(),
        &Environment::default(),
        &CollisionWorld::new(),
        &Options::default(),
    );
    // Tick 0 is seeded from the recording, so the first tick the replay
    // computes for itself is tick 1.
    assert!(
        simulated.frames[1].speed_cached.abs() < 1e-3,
        "{} is the magnitude, not the projection",
        simulated.frames[1].speed_cached
    );
    // And the magnitude really is 22 here, so the two readings are separated
    // by this fixture rather than merely agreeing on a small number.
    assert!((simulated.frames[1].speed - 22.0).abs() < 1e-3);
}

/// Replaying the same recording twice must produce the same trace, or nothing
/// the harness reports means anything. `docs/architecture/determinism.md`.
#[test]
fn a_replay_is_reproducible() {
    let recorded = coasting(60, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
    let run = || {
        replay(
            &recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &Options::default(),
        )
    };
    assert_eq!(run(), run());
}

/// Sampling after the step instead of before it is the mistake this pins: it
/// shifts every row by one tick, which reads as a lag in the physics.
#[test]
fn rows_are_sampled_before_the_step_not_after() {
    let dt = 1.0 / 60.0;
    let velocity = Vec3::new(0.0, 0.0, 22.0);
    let recorded = coasting(4, dt, velocity);
    let simulated = replay(
        &recorded,
        &inert_handling(),
        &Environment::default(),
        &CollisionWorld::new(),
        &Options::default(),
    );
    // Row 1 must hold one tick of travel, not two and not none.
    let travelled = simulated.frames[1].position - simulated.frames[0].position;
    assert!((travelled - velocity * dt).length() < 1e-5, "{travelled}");
}

#[test]
fn a_held_input_ignores_what_the_recording_says_the_controls_were() {
    let mut recorded = coasting(4, 1.0 / 60.0, Vec3::ZERO);
    for frame in &mut recorded.frames {
        frame.throttle = 100.0;
    }
    let options = Options {
        inputs: Inputs::Held(Held::default()),
        ..Options::default()
    };
    let simulated = replay(
        &recorded,
        &inert_handling(),
        &Environment::default(),
        &CollisionWorld::new(),
        &options,
    );
    // Tick 0 is the seeded initial condition, so the recording's throttle is
    // still there; from tick 1 the held input - nothing - is what drives it.
    assert_eq!(simulated.frames[0].throttle, 100.0);
    assert_eq!(simulated.frames[1].throttle, 0.0);
}

#[test]
fn a_trace_derived_input_carries_the_recorded_throttle_through() {
    let mut recorded = coasting(4, 1.0 / 60.0, Vec3::ZERO);
    for frame in &mut recorded.frames {
        frame.throttle = 100.0;
    }
    let simulated = replay(
        &recorded,
        &inert_handling(),
        &Environment::default(),
        &CollisionWorld::new(),
        &Options::default(),
    );
    assert!(simulated.frames.iter().all(|f| f.throttle == 100.0));
}

/// The point of a script: the input *changes* part-way through a run, and it
/// changes on the tick the file says rather than on whatever tick the
/// recording happened to.
#[test]
fn a_scripted_input_changes_on_the_tick_the_script_says() {
    let mut recorded = coasting(6, 1.0 / 60.0, Vec3::ZERO);
    for frame in &mut recorded.frames {
        // The recording says thrust throughout, and the script must win.
        frame.throttle = 100.0;
    }
    let script = crate::script::Script::parse("3 none\n3 cross\n").expect("parses");
    let options = Options {
        inputs: Inputs::Scripted(script.states.clone()),
        ..Options::default()
    };
    let simulated = replay(
        &recorded,
        &inert_handling(),
        &Environment::default(),
        &CollisionWorld::new(),
        &options,
    );
    // Tick 0 is the seeded initial condition, so it still carries the
    // recording's throttle; ticks 1 and 2 are the script's `none`, and the
    // thrust the script asks for at tick 3 shows in the row *after* it,
    // because a row is sampled before its own step.
    assert_eq!(simulated.frames[1].throttle, 0.0);
    assert_eq!(simulated.frames[3].throttle, 0.0);
    assert!(simulated.frames[4].throttle > 0.0);
}

/// A script's steering must reach the force law through the same axis a pad's
/// would, or a scripted turn is not the turn a player would take. Asserted on
/// the snapshot rather than on the ship's own `steer`, which is downstream of
/// a ramp whose rate the inert test parameter set holds at zero.
#[test]
fn a_scripted_dpad_reaches_the_stick_axis_and_the_cross_bit() {
    let script = crate::script::Script::parse("2 cross left\n").expect("parses");
    let inputs = Inputs::Scripted(script.states.clone());
    let mut buttons = Input::new();
    let snapshot = snapshot_for(&Frame::default(), &mut buttons, &inputs, 0);
    assert_eq!(snapshot.stick_x, -1.0, "left is negative stick x");
    assert!(snapshot.buttons.is_held(button::CROSS));
    assert!(
        snapshot.buttons.is_pressed(button::LEFT),
        "tick 0 is an edge"
    );

    let snapshot = snapshot_for(&Frame::default(), &mut buttons, &inputs, 1);
    assert!(
        !snapshot.buttons.is_pressed(button::LEFT),
        "a button held across two scripted ticks is not pressed twice"
    );
}

/// A script shorter than the recording holds its last state rather than
/// releasing everything, which would put a deceleration into the comparison
/// that nobody asked for.
#[test]
fn a_short_script_holds_its_last_state_for_the_rest_of_the_run() {
    let recorded = coasting(8, 1.0 / 60.0, Vec3::ZERO);
    let script = crate::script::Script::parse("2 cross\n").expect("parses");
    let options = Options {
        inputs: Inputs::Scripted(script.states.clone()),
        ..Options::default()
    };
    let simulated = replay(
        &recorded,
        &inert_handling(),
        &Environment::default(),
        &CollisionWorld::new(),
        &options,
    );
    assert!(simulated.frames[7].throttle > 0.0);
}

/// A one-state script and the equivalent held input are the same run. They
/// share a type for exactly this reason, and this is what says so.
#[test]
fn a_one_state_script_is_the_same_run_as_the_equivalent_held_input() {
    let recorded = coasting(30, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
    let script = crate::script::Script::parse("30 cross\n").expect("parses");
    let run = |inputs| {
        replay(
            &recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &Options {
                inputs,
                ..Options::default()
            },
        )
    };
    assert_eq!(
        run(Inputs::Scripted(script.states.clone())),
        run(Inputs::Held(script.states[0]))
    );
}

/// The recorded yaw rate the captures show with the stick held over, and the
/// signal the angular-velocity column was added to compare against:
/// `+1.51 rad/s`, `docs/ghidra/functions/psp-pulse-usa/engine.md`.
const RECORDED_YAW_RATE: f32 = 1.51;

/// Tick 0 is an initial condition, not a measurement - the same thing that is
/// already true of position, velocity and the basis. So a recorded angular
/// velocity must come back out of tick 0 unchanged under **every** reading:
/// the seeding and the writing-back are one transformation and its inverse,
/// and a reading where they are not would put a tick-0 error into the
/// comparison that looks exactly like a physics bug.
#[test]
fn a_recorded_rotation_survives_tick_zero_under_every_reading() {
    for angular in AngularReading::ALL {
        let mut recorded = coasting(4, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
        let recorded_rate = Vec3::new(0.1, RECORDED_YAW_RATE, -0.05);
        for frame in &mut recorded.frames {
            frame.angular_velocity = Some(recorded_rate);
        }
        let simulated = replay(
            &recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &Options {
                angular,
                ..Options::default()
            },
        );
        let written = simulated.frames[0]
            .angular_velocity
            .expect("a replay always writes one");
        assert!(
            (written - recorded_rate).length() < 1e-5,
            "{angular:?}: {written} is not {recorded_rate}"
        );
    }
}

/// A capture taken before the column existed leaves the rotation at rest,
/// which is what every run did before it existed. Nothing is invented for the
/// older file - see `crate::trace::REQUIRED_COLUMNS`.
#[test]
fn a_recording_without_an_angular_velocity_starts_at_rest() {
    let recorded = coasting(4, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
    assert_eq!(recorded.frames[0].angular_velocity, None);
    let state = initial_state(
        &recorded.frames[0],
        &inert_handling(),
        Basis::default(),
        AngularReading::default(),
    );
    assert_eq!(state.body.angular_velocity, Vec3::ZERO);
}

/// And the seeding is not decorative: a ship handed the recorded turn rate
/// must actually be turning, or the column would be read and then ignored.
///
/// **Both angular columns seed, and they are not the same quantity.**
/// `omega_*` (`body+0x150`) is the rate and seeds it directly; `avel_*`
/// (`body+0x160`) is `I * omega` and has to be divided by the tensor on the
/// way in. This test drives both and asserts they agree when handed the same
/// physical rotation - the version of it that seeded a body's angular
/// velocity straight from the momentum column started every run spinning
/// `21.6x` too fast about the up axis, and passed, because it only asked
/// whether the nose had moved.
#[test]
fn a_seeded_rotation_actually_turns_the_ship() {
    let rate = Vec3::new(0.0, RECORDED_YAW_RATE, 0.0);
    let momentum = rate * oag_physics::forces::ship_inertia();

    let mut by_rate = coasting(4, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
    for frame in &mut by_rate.frames {
        frame.angular_rate = Some(rate);
    }
    let mut by_momentum = coasting(4, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
    for frame in &mut by_momentum.frames {
        frame.angular_velocity = Some(momentum);
    }
    let still = coasting(4, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));

    let run = |recorded| {
        replay(
            recorded,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &Options::default(),
        )
    };
    let (turned, from_momentum, stayed) = (run(&by_rate), run(&by_momentum), run(&still));

    let sweep = |a: &crate::trace::Frame, b: &crate::trace::Frame| {
        crate::compare::checks(a, b, &crate::compare::Tolerances::default())
            .iter()
            .flatten()
            .find(|check| check.field == Field::Forward)
            .expect("the forward axis is always compared")
            .error
    };

    // A tick of 1.51 rad/s is about 0.025 rad, so tick 1's noses are that far
    // apart; the ship left at rest has not moved its at all.
    assert!(
        sweep(&turned.frames[1], &stayed.frames[1]) > 0.01,
        "the rate column did not turn the ship"
    );
    // And the momentum column, divided by the tensor, is the same rotation.
    assert!(
        sweep(&turned.frames[1], &from_momentum.frames[1]) < 1e-5,
        "the two columns disagree about the same physical rotation"
    );
}

/// The simulated side always carries the timers it models and never the one
/// it does not, so `timer_2e0` is reported as not compared rather than as
/// agreement. `craft+0x2e0`'s arming condition has never been read.
#[test]
fn a_replay_carries_the_stun_timer_and_not_the_gate_it_does_not_model() {
    let recorded = coasting(4, 1.0 / 60.0, Vec3::ZERO);
    let simulated = replay(
        &recorded,
        &inert_handling(),
        &Environment::default(),
        &CollisionWorld::new(),
        &Options::default(),
    );
    assert_eq!(simulated.frames[0].stun_timer, Some(0.0));
    assert_eq!(simulated.frames[0].timer_2e0, None);
}

/// A driven run has no recording behind it, so nothing can be seeded from
/// one: `drive` must produce exactly the ticks it was asked for, from the
/// state it was handed.
#[test]
fn a_driven_run_is_as_long_as_it_was_asked_for_and_starts_where_it_was_put() {
    let mut initial = ShipState::default();
    initial.body.mass = TEST_MASS;
    initial.body.position = Vec3::new(1.0, 2.0, 3.0);
    let script = crate::script::Script::parse("40 cross\n").expect("parses");
    let run = drive(
        initial,
        &inert_handling(),
        &Environment::default(),
        &CollisionWorld::new(),
        &script.states,
        &DriveOptions {
            ticks: 40,
            ..DriveOptions::default()
        },
    );
    assert_eq!(run.len(), 40);
    assert_eq!(run.frames[0].position, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(run.frames[0].tick, 0);
    assert_eq!(run.frames[39].tick, 39);
}

/// A script shorter than the run holds its last state here too, which is the
/// same rule [`Script::at`](crate::script::Script::at) argues for and the
/// reason `--ticks` may exceed a scenario's own length.
#[test]
fn a_driven_run_holds_a_short_script_s_last_state() {
    let mut initial = ShipState::default();
    initial.body.mass = TEST_MASS;
    let script = crate::script::Script::parse("2 none\n2 cross\n").expect("parses");
    let run = drive(
        initial,
        &inert_handling(),
        &Environment::default(),
        &CollisionWorld::new(),
        &script.states,
        &DriveOptions {
            ticks: 20,
            ..DriveOptions::default()
        },
    );
    assert_eq!(run.frames[1].throttle, 0.0);
    assert!(run.frames[19].throttle > 0.0, "the last state is held");
}

/// Driving the same scenario twice must give the same trace, for the same
/// reason replaying one twice must: `docs/architecture/determinism.md`. This
/// is the property `just scripted-sim` rests on - it is the only side of the
/// comparison that *can* be reproduced exactly, the emulator's start pose
/// being repeatable only to about 2.5 degrees.
#[test]
fn a_driven_run_is_reproducible() {
    let script = crate::script::Script::parse("10 cross\n10 cross left\n").expect("parses");
    let run = || {
        let mut initial = ShipState::default();
        initial.body.mass = TEST_MASS;
        drive(
            initial,
            &inert_handling(),
            &Environment::default(),
            &CollisionWorld::new(),
            &script.states,
            &DriveOptions {
                ticks: 20,
                ..DriveOptions::default()
            },
        )
    };
    assert_eq!(run(), run());
}

/// `drive` and `replay` are the same stepping loop with different sources for
/// the length, the delta and the seed. Handed the same three, they must agree
/// tick for tick - otherwise a scenario run and a comparison run would be
/// measuring two different simulations and nobody would notice.
#[test]
fn driving_and_replaying_the_same_scenario_agree() {
    let dt = 1.0 / 60.0;
    let recorded = coasting(30, dt, Vec3::new(0.0, 0.0, 22.0));
    let script = crate::script::Script::parse("30 cross\n").expect("parses");
    let handling = inert_handling();
    let replayed = replay(
        &recorded,
        &handling,
        &Environment::default(),
        &CollisionWorld::new(),
        &Options {
            inputs: Inputs::Scripted(script.states.clone()),
            dt: DeltaSource::Fixed(dt),
            ..Options::default()
        },
    );
    let driven = drive(
        initial_state(
            &recorded.frames[0],
            &handling,
            Basis::default(),
            AngularReading::default(),
        ),
        &handling,
        &Environment::default(),
        &CollisionWorld::new(),
        &script.states,
        &DriveOptions {
            ticks: 30,
            dt,
            ..DriveOptions::default()
        },
    );
    assert_eq!(driven.len(), replayed.len());
    for (a, b) in driven.frames.iter().zip(replayed.frames.iter()) {
        assert_eq!(a.position, b.position, "tick {}", a.tick);
        assert_eq!(a.velocity, b.velocity, "tick {}", a.tick);
        assert_eq!(a.throttle, b.throttle, "tick {}", a.tick);
    }
}

#[test]
fn an_empty_recording_replays_to_an_empty_trace() {
    let simulated = replay(
        &Trace::default(),
        &inert_handling(),
        &Environment::default(),
        &CollisionWorld::new(),
        &Options::default(),
    );
    assert!(simulated.is_empty());
}

/// The whole harness is worthless if it cannot say *when* a run went wrong, so
/// this drives a real divergence through it: gravity in the parameter set that
/// the recording's straight line does not have.
#[test]
fn a_physics_difference_shows_up_as_a_dated_divergence() {
    let recorded = coasting(120, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
    let mut handling = inert_handling();
    handling.physical.normal_gravity = 9.8;
    handling.physical.flight_gravity = 9.8;
    let simulated = replay(
        &recorded,
        &handling,
        &Environment::default(),
        &CollisionWorld::new(),
        &Options::default(),
    );
    let comparison = compare(&recorded, &simulated, &Tolerances::default());
    let divergence = comparison
        .first_divergence
        .clone()
        .expect("gravity must show");
    assert!(
        divergence.tick > 0,
        "tick 0 is the seeded initial condition"
    );
    assert_eq!(
        comparison.field(Field::Position).trend.verdict,
        crate::compare::TrendVerdict::Growing,
        "a constant acceleration is a systematic error, not a bounded one"
    );
}

fn reseeding(every: usize) -> Options {
    Options {
        reseed: NonZeroUsize::new(every),
        ..Options::default()
    }
}

/// The default must stay exactly what it was, or every existing invocation
/// silently changes meaning.
#[test]
fn a_run_is_seeded_once_unless_asked_otherwise() {
    assert_eq!(Options::default().reseed, None);

    let recorded = coasting(120, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
    let mut handling = inert_handling();
    handling.physical.normal_gravity = 9.8;
    handling.physical.flight_gravity = 9.8;
    let once = replay(
        &recorded,
        &handling,
        &Environment::default(),
        &CollisionWorld::new(),
        &Options::default(),
    );
    let explicit = replay(
        &recorded,
        &handling,
        &Environment::default(),
        &CollisionWorld::new(),
        &Options {
            reseed: None,
            ..Options::default()
        },
    );
    assert_eq!(once.to_csv(), explicit.to_csv());
}

/// The property the whole option exists for: error stops compounding.
///
/// The same falling-under-gravity divergence as
/// [`a_physics_difference_shows_up_as_a_dated_divergence`], which grows as
/// `t^2` when the run is seeded once. Re-seeded every twenty ticks it cannot
/// exceed what twenty ticks of that acceleration produce, however long the
/// recording is - so the max error stops being a statement about the run's
/// length and becomes one about the window's.
#[test]
fn reseeding_bounds_the_error_by_the_window_rather_than_the_run() {
    let recorded = coasting(600, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
    let mut handling = inert_handling();
    handling.physical.normal_gravity = 9.8;
    handling.physical.flight_gravity = 9.8;

    let seeded_once = replay(
        &recorded,
        &handling,
        &Environment::default(),
        &CollisionWorld::new(),
        &Options::default(),
    );
    let reseeded = replay(
        &recorded,
        &handling,
        &Environment::default(),
        &CollisionWorld::new(),
        &reseeding(20),
    );

    let once = compare(&recorded, &seeded_once, &Tolerances::default());
    let windowed = compare(&recorded, &reseeded, &Tolerances::default());
    let (once, windowed) = (
        once.field(Field::Position).max_error,
        windowed.field(Field::Position).max_error,
    );
    // Free fall over a window of `n` ticks goes as `n^2`, so twenty ticks of
    // it against six hundred is a factor of nine hundred. An order of
    // magnitude is the assertion; the exact ratio is arithmetic nobody should
    // have to keep true.
    assert!(
        windowed * 10.0 < once,
        "reseeded error {windowed} is not bounded well below the compounded {once}"
    );
}

/// A window's first row is the recording's own state, so it is an initial
/// condition and not a measurement - the same property tick 0 has, and the
/// reason the seed happens before the row is emitted rather than after.
#[test]
fn every_window_starts_exactly_on_the_recording() {
    let recorded = coasting(100, 1.0 / 60.0, Vec3::new(0.0, 0.0, 22.0));
    let mut handling = inert_handling();
    handling.physical.normal_gravity = 9.8;
    handling.physical.flight_gravity = 9.8;
    let simulated = replay(
        &recorded,
        &handling,
        &Environment::default(),
        &CollisionWorld::new(),
        &reseeding(25),
    );

    for tick in [0, 25, 50, 75] {
        assert_eq!(
            simulated.frames[tick].position, recorded.frames[tick].position,
            "tick {tick} is a seeded row and must agree exactly"
        );
    }
    assert_ne!(
        simulated.frames[24].position, recorded.frames[24].position,
        "the tick before a seed is a measurement and must be free to diverge"
    );
}
