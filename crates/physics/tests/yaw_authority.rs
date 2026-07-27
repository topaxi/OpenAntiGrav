//! What the steering term is worth, in rad/s.
//!
//! This file exists because the recovered steering law is *verified* and still
//! wrong by about 22x: see [`oag_physics::forces::YAW_DRIVE_CALIBRATION`] for
//! the instruction-level reading and the measurements it disagrees with. The
//! calibration that closes the gap is fitted to real captures, so it needs a test
//! that fails when someone changes it - the usual "don't pin this crate's own
//! arithmetic" rule in `ship_dynamics.rs` is suspended here precisely because
//! there *is* an external measurement to pin against.
//!
//! Both tests here run in CI on arbitrary round numbers, and pin the *shape* -
//! that yaw settles at `steer * amount * calibration / |YAW_DAMPING|` - without
//! reproducing any shipped handling value. One does it against a two-line ODE, so
//! a failure points at the constant; the other drives the real `step`, so a
//! failure points at the wiring.
//!
//! What pins the *number* is `crates/trace/tests/yaw_authority_ground_truth.rs`,
//! which replays the original's own recorded captures. It lives over there rather
//! than here because it has to read the player's disc, and `oag-physics` depends
//! on `oag-core` and nothing else, deliberately.
//!
//! No handling data is reproduced here; see
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.

use oag_core::math::Vec3;
use oag_physics::collide::{Surface, TriangleSoup};
use oag_physics::engine::steering;
use oag_physics::forces::YAW_DRIVE_CALIBRATION;
use oag_physics::params::{Antigrav, Dimensions, Physical, Turning};
use oag_physics::passive::YAW_DAMPING;
use oag_physics::{Body, CollisionWorld, Environment, Handling, ShipControls, ShipState, step};

/// One step of the isolated yaw equation the captures were fitted against:
///
/// ```text
/// omega' = -steer * amount * calibration - YAW_DAMPING_MAGNITUDE * omega
/// ```
///
/// Isolated on purpose. Running the full `step` would fold in hover, grip and
/// wall response, none of which is what the calibration was fitted to, and all of
/// which carry their own open questions.
///
/// The calibration is applied here rather than inside `steering` because that is
/// where [`oag_physics::forces::evaluate`] applies it: to the summed yaw drive,
/// before the damping. Mirroring the pipeline's shape is the point.
fn advance_yaw(omega: f32, steer: f32, handling: &Handling, dt: f32) -> f32 {
    let state = ShipState {
        steer,
        ..ShipState::default()
    };
    let drive = steering(&state, handling) * YAW_DRIVE_CALIBRATION;

    omega + (drive + YAW_DAMPING * omega) * dt
}

const TICK: f32 = 1.0 / 60.0;

fn handling_with_turning_amount(amount: f32) -> Handling {
    Handling {
        turning: Turning {
            amount,
            gain: 500.0,
            falloff: 900.0,
        },
        ..Handling::ZERO
    }
}

/// Steering is an angular *acceleration* damped by [`YAW_DAMPING`], so holding a
/// deflection settles at a rate rather than spinning up without bound.
///
/// Both terms land in the same accumulator in the original
/// (`Ship_UpdateSteering` and `Ship_ApplyAngularDamping` each add to
/// `craft+0x340`), which is why every common factor - the inertia tensor
/// included - cancels out of this equilibrium.
#[test]
fn terminal_yaw_rate_is_the_damped_equilibrium_of_the_steering_term() {
    // Arbitrary round numbers. **Not values from any ship.**
    let handling = handling_with_turning_amount(2.0);
    let steer = 100.0;
    let dt = 1.0 / 60.0;

    let expected = -(steer * 2.0 * YAW_DRIVE_CALIBRATION) / -YAW_DAMPING;

    let mut omega = 0.0;
    for _ in 0..2000 {
        omega = advance_yaw(omega, steer, &handling, dt);
    }

    assert!(
        (omega - expected).abs() < 1e-3,
        "yaw settled at {omega} rad/s, expected {expected} rad/s"
    );

    // The sign convention this crate needs: positive steer is to the right, and a
    // right turn is a negative yaw about the up axis. See `engine::steering`.
    assert!(omega < 0.0, "holding right must yaw negative, got {omega}");

    let mut mirrored = 0.0;
    for _ in 0..2000 {
        mirrored = advance_yaw(mirrored, -steer, &handling, dt);
    }
    assert!(
        (mirrored + omega).abs() < 1e-3,
        "left and right must mirror: {mirrored} vs {omega}"
    );
}

/// Full lock has to beat the camber, or a banked corner steers the ship for you.
///
/// Reported from play: on tilted track "I'm not able to steer in the opposite
/// direction". [`oag_physics::hover::BANK_TO_YAW_GAIN`] writes `30 * right.y` into
/// the *same* body-local yaw accumulator that steering writes to, so the two are
/// in direct competition and their **ratio** is what decides whether a corner can
/// be fought. At a bank of 30 degrees the camber asks for `30 * sin(30) = 15`,
/// which full lock has to exceed.
///
/// This is a ratio, so it is invariant to
/// [`oag_physics::forces::YAW_DRIVE_CALIBRATION`] - which is the whole point.
/// Scaling *only* steering by that factor, and leaving bank-to-yaw at full
/// strength, put the break-even bank at about 14 degrees and is exactly the bug
/// this reproduces.
#[test]
fn full_lock_out_yaws_the_bank_on_a_steeply_cambered_track() {
    // Arbitrary round numbers. **Not values from any ship.**
    let amount = 2.0;
    let handling = handling_with_turning_amount(amount);
    let state = ShipState {
        steer: 100.0,
        ..ShipState::default()
    };

    let steering_drive = steering(&state, &handling).abs();

    for bank_degrees in [10.0_f32, 20.0, 30.0] {
        let bank_drive = oag_physics::hover::BANK_TO_YAW_GAIN * bank_degrees.to_radians().sin();

        assert!(
            steering_drive > bank_drive,
            "at a {bank_degrees} degree bank the camber asks for {bank_drive} rad/s^2 \
             and full lock only answers with {steering_drive}: the ship cannot be \
             steered out of the corner"
        );
    }
}

/// The same equilibrium, but reached through the real [`step`] rather than through
/// this file's own two-line ODE.
///
/// [`terminal_yaw_rate_is_the_damped_equilibrium_of_the_steering_term`] would still
/// pass if someone dropped `passive::angular_damping` out of `forces::evaluate`, or
/// stopped routing `engine::steering` into the accumulator at all - it never calls
/// the pipeline. This one does, on a flat floor with a ship held at its hover
/// height, so the wiring is pinned as well as the arithmetic.
///
/// The tolerance is deliberately loose (30 %). Running the whole force law brings
/// in the weathervane, lateral grip and the hover probes' own torques, none of
/// which the calibration was fitted to; the assertion is that yaw lands *at the
/// right rate*, not that the two models agree to the last digit.
///
/// **This does not pin the constant's value**, and neither does the test above:
/// both derive `expected` from [`YAW_DRIVE_CALIBRATION`], so editing it moves
/// both sides and they stay green. That is deliberate - the number's only honest
/// judge is the original's own behaviour, which is
/// `crates/trace/tests/yaw_authority_ground_truth.rs` under `just test-data`.
/// What this one pins is the *wiring*: unhooking `engine::steering` from
/// `forces::evaluate` drops it to `0 rad/s` and fails here, where the ODE test
/// above never notices.
#[test]
fn steering_reaches_that_same_rate_through_the_real_force_law() {
    // Arbitrary round numbers. **Not values from any ship.**
    let amount = 2.0;
    let handling = Handling {
        physical: Physical {
            mass: 1.0,
            ..Handling::ZERO.physical
        },
        antigrav: Antigrav {
            ride_height: 4.0,
            ..Handling::ZERO.antigrav
        },
        dimensions: Dimensions {
            height: 1.0,
            length: 4.0,
            width: 2.0,
            ..Dimensions::default()
        },
        ..handling_with_turning_amount(amount)
    };

    let mut floor = CollisionWorld::new();
    floor.push(TriangleSoup::new(
        vec![
            [-2000.0, 0.0, -2000.0],
            [-2000.0, 0.0, 2000.0],
            [2000.0, 0.0, 0.0],
        ],
        vec![[0, 1, 2]],
        Vec::new(),
        Surface::Floor,
        0,
    ));

    let mut state = ShipState {
        body: Body {
            position: Vec3::new(0.0, handling.antigrav.ride_height, 0.0),
            mass: handling.physical.mass,
            ..Body::default()
        },
        // Seeded already at full deflection, so the ramp's own time constant does
        // not have to be waited out on top of the yaw's.
        steer: 100.0,
        ..ShipState::default()
    };

    let controls = ShipControls {
        steer_x: 1.0,
        ..ShipControls::default()
    };

    let mut previous_forward = state.body.forward();
    let mut yaw = 0.0;
    for tick in 0..600 {
        step(
            &mut state,
            &controls,
            &handling,
            &Environment::default(),
            &floor,
            TICK,
        );

        // Measured the way the captures are, rather than read out of the body, so
        // this asserts against observable rotation.
        let forward = state.body.forward();
        if tick >= 400 {
            yaw += Vec3::cross(previous_forward, forward).dot(state.body.up()) / TICK / 200.0;
        }
        previous_forward = forward;
    }

    let expected = -(100.0 * amount * YAW_DRIVE_CALIBRATION) / -YAW_DAMPING;
    assert!(
        (yaw - expected).abs() < 0.3 * expected.abs(),
        "yaw through the force law settled at {yaw} rad/s, expected about {expected}"
    );
}
