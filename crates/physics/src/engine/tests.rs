//! What the engine, brake, steering and pitch laws in [`super`] are asserted to do. Split out of
//! `engine.rs` under the 200-line cap on inline `#[cfg(test)]` modules (`scripts/check-file-size.py`).

use super::*;
use crate::params::{Brakes, Engine, Pitch, Turning};
use crate::ship::Body;

/// Arbitrary round numbers, already in the scaled in-memory form.
/// **Not recovered values.**
fn test_handling() -> Handling {
    Handling {
        engine: Engine {
            accelcap: 20.0,
            amount: 0.4,
            falloff: 1.0,
            gain: 1.0,
            turbo: 5.0,
        },
        brakes: Brakes {
            amount: -0.5,
            falloff: 200.0,
            gain: 400.0,
        },
        turning: Turning {
            amount: 0.01,
            falloff: 200.0,
            gain: 400.0,
        },
        pitch: Pitch {
            pitch_air: 0.02,
            pitch_ground: 0.005,
            pitch_damping: 3.0,
            antigrav_height_adjust: 0.0,
        },
        ..Handling::ZERO
    }
}

fn ship_moving_forward(speed: f32) -> ShipState {
    ShipState {
        body: Body {
            linear_velocity: Vec3::new(0.0, 0.0, -speed),
            ..Body::default()
        },
        ..ShipState::default()
    }
}

#[test]
fn full_throttle_pushes_the_ship_along_its_forward_axis() {
    let handling = test_handling();
    let mut state = ship_moving_forward(40.0);
    state.thrust = 100.0;

    let force = engine(
        &state,
        &handling,
        1.0,
        40.0,
        None,
        1.0,
        EngineFlags::default(),
    )
    .as_local_force();
    // Body forward is -Z.
    assert!(force.z < 0.0, "force was {force:?}");
    assert_eq!(force.x, 0.0);
    assert_eq!(force.y, 0.0);
}

/// The whole point of the gate: a stunned ship at full throttle produces
/// nothing. This is what stopped this crate accelerating to four times the
/// original's speed, so it pins the behaviour rather than just the field.
#[test]
fn a_stunned_ship_produces_no_thrust_at_full_throttle() {
    let handling = test_handling();
    let mut state = ship_moving_forward(24.0);
    state.thrust = 100.0;

    let running = engine(
        &state,
        &handling,
        1.0,
        24.0,
        None,
        1.0,
        EngineFlags::default(),
    );
    assert!(
        running.thrust > 0.0,
        "the un-stunned baseline must be non-zero"
    );

    state.stun_timer = 0.5;
    assert_eq!(
        engine(
            &state,
            &handling,
            1.0,
            24.0,
            None,
            1.0,
            EngineFlags::default()
        ),
        EngineForce::default()
    );
}

/// The gate is `> 0`, so a timer that has counted exactly to zero is running
/// again. An `>=` here would leave every ship permanently dead.
#[test]
fn a_stun_timer_at_exactly_zero_does_not_gate() {
    let handling = test_handling();
    let mut state = ship_moving_forward(24.0);
    state.thrust = 100.0;
    state.stun_timer = 0.0;

    assert!(
        engine(
            &state,
            &handling,
            1.0,
            24.0,
            None,
            1.0,
            EngineFlags::default()
        )
        .thrust
            > 0.0
    );
}

/// The same early return has a second arm, on the weapon slowdown timer.
#[test]
fn a_weapon_slowed_ship_produces_no_thrust_either() {
    let handling = test_handling();
    let mut state = ship_moving_forward(24.0);
    state.thrust = 100.0;
    state.slowdown_timer = 1.0;

    assert_eq!(
        engine(
            &state,
            &handling,
            1.0,
            24.0,
            None,
            1.0,
            EngineFlags::default()
        ),
        EngineForce::default()
    );
}

/// A running Turbo pickup escapes both arms of the early return (`craft+0x1c0 & 0x200`,
/// `0x0884c678`), and the barrel roll's payout does not. Fails if the escape is dropped or
/// widened to the roll.
#[test]
fn a_running_turbo_keeps_the_engine_through_a_stun_and_a_slowdown() {
    let handling = test_handling();
    let run = |state: &ShipState| {
        engine(
            state,
            &handling,
            1.0,
            24.0,
            None,
            1.0,
            EngineFlags::default(),
        )
        .thrust
    };
    for (stun, slow) in [(0.5, 0.0), (0.0, 1.0)] {
        let mut state = ship_moving_forward(24.0);
        state.thrust = 100.0;
        state.stun_timer = stun;
        state.slowdown_timer = slow;
        assert_eq!(run(&state), 0.0, "gated without a turbo");

        state.roll_payout_timer = 0.5;
        assert_eq!(run(&state), 0.0, "the roll payout is not the escape");

        state.roll_payout_timer = 0.0;
        state.turbo_timer = 0.5;
        assert!(run(&state) > 0.0, "a running turbo escapes the gate");
    }
}

/// The Eliminator wrong-way kill zeroes the engine whatever else is running, a Turbo included
/// (`0x0884c954` is the last thing the original does to `T`).
#[test]
fn the_wrong_way_kill_zeroes_thrust_even_under_a_turbo() {
    let handling = test_handling();
    let mut state = ship_moving_forward(24.0);
    state.thrust = 100.0;
    state.turbo_timer = 0.5;
    let run = |state: &ShipState| {
        engine(
            state,
            &handling,
            1.0,
            24.0,
            None,
            1.0,
            EngineFlags::default(),
        )
    };
    assert!(run(&state).thrust > 0.0);
    state.wrong_way_kill = true;
    assert_eq!(run(&state), EngineForce::default());
}

/// The boost lift: `<Special turbo_jump> * T` on the thrust just after the turbo add, under a
/// Turbo or a roll payout, and nothing otherwise (`0x0884c8d8`-`0x0884c910`). Fails if the lift
/// is dropped, ungated, or taken from the doubled thrust.
#[test]
fn a_turbo_lifts_by_turbo_jump_times_the_thrust_after_the_add() {
    let handling = Handling {
        turbo_jump: 0.1,
        ..test_handling()
    };
    let mut state = ship_moving_forward(24.0);
    state.thrust = 100.0;
    let run = |state: &ShipState| {
        engine(
            state,
            &handling,
            1.0,
            24.0,
            None,
            1.0,
            EngineFlags::default(),
        )
    };
    let plain = run(&state);
    assert_eq!(plain.lift, 0.0, "no lift without a turbo");

    let before_double = plain.thrust / ENGINE_OUTPUT_DOUBLE + handling.engine.turbo;
    for roll in [false, true] {
        let mut boosted = state;
        if roll {
            boosted.roll_payout_timer = 0.5;
        } else {
            boosted.turbo_timer = 0.5;
        }
        let lift = run(&boosted).lift;
        assert!((lift - 0.1 * before_double).abs() < 1e-3, "{lift}");
    }
}

#[test]
fn no_throttle_is_no_thrust() {
    let handling = test_handling();
    let state = ship_moving_forward(40.0);
    assert_eq!(
        engine(
            &state,
            &handling,
            1.0,
            40.0,
            None,
            1.0,
            EngineFlags::default()
        )
        .thrust,
        0.0
    );
}

/// An airborne ship keeps a fifth of its thrust, and a half-grounded one lands
/// between the two rather than snapping.
#[test]
fn thrust_is_reduced_but_not_removed_with_no_contact() {
    let handling = Handling {
        engine: Engine {
            // A cap high enough not to bind, so this test sees only the blend.
            accelcap: 1000.0,
            ..test_handling().engine
        },
        ..test_handling()
    };
    let mut state = ship_moving_forward(0.0);
    state.thrust = 100.0;

    let grounded = engine(
        &state,
        &handling,
        1.0,
        0.0,
        None,
        1.0,
        EngineFlags::default(),
    )
    .thrust;
    let airborne = engine(
        &state,
        &handling,
        0.0,
        0.0,
        None,
        1.0,
        EngineFlags::default(),
    )
    .thrust;
    let half = engine(
        &state,
        &handling,
        0.5,
        0.0,
        None,
        1.0,
        EngineFlags::default(),
    )
    .thrust;

    assert_eq!(airborne, grounded * ENGINE_AIR_THRUST);
    assert!(half > airborne && half < grounded);
}

/// The cap rises with speed, which is what makes `accelcap` a floor on the
/// ceiling rather than a top speed.
#[test]
fn the_thrust_cap_grows_with_forward_speed() {
    let handling = test_handling();
    let mut state = ship_moving_forward(0.0);
    state.thrust = 100.0;

    let stationary = engine(
        &state,
        &handling,
        1.0,
        0.0,
        None,
        1.0,
        EngineFlags::default(),
    )
    .thrust;
    let fast = engine(
        &state,
        &handling,
        1.0,
        200.0,
        None,
        1.0,
        EngineFlags::default(),
    )
    .thrust;
    assert!(fast > stationary, "{fast} was not above {stationary}");
}

/// The cap reads the *magnitude* of the forward speed, so reversing does not
/// give a ship a smaller ceiling than standing still.
#[test]
fn the_thrust_cap_uses_the_magnitude_of_the_forward_speed() {
    let handling = test_handling();
    let mut state = ship_moving_forward(0.0);
    state.thrust = 100.0;

    assert_eq!(
        engine(
            &state,
            &handling,
            1.0,
            60.0,
            None,
            1.0,
            EngineFlags::default()
        )
        .thrust,
        engine(
            &state,
            &handling,
            1.0,
            -60.0,
            None,
            1.0,
            EngineFlags::default()
        )
        .thrust
    );
}

#[test]
fn a_zero_parameter_engine_produces_nothing() {
    let mut state = ship_moving_forward(40.0);
    state.thrust = 100.0;
    assert_eq!(
        engine(
            &state,
            &Handling::ZERO,
            1.0,
            40.0,
            None,
            1.0,
            EngineFlags::default()
        )
        .thrust,
        0.0
    );
}

#[test]
fn the_brake_opposes_travel_rather_than_accelerating_it() {
    let handling = test_handling();
    let mut state = ship_moving_forward(40.0);
    state.brake = 100.0;

    let force = brakes(&state, &handling);
    // Travelling along -Z, so the brake must push along +Z.
    assert!(force.z > 0.0, "force was {force:?}");
}

/// `Brakes.amount` is negative in memory. A reimplementation that negated here
/// as well would accelerate under braking, which this pins.
#[test]
fn a_positive_brake_amount_would_accelerate_and_is_therefore_not_what_the_data_holds() {
    let mut handling = test_handling();
    handling.brakes.amount = 0.5;
    let mut state = ship_moving_forward(40.0);
    state.brake = 100.0;

    // Documenting the trap: with the sign flipped the force is along travel.
    assert!(brakes(&state, &handling).z < 0.0);
}

#[test]
fn no_brake_state_is_no_brake_force() {
    let handling = test_handling();
    let state = ship_moving_forward(40.0);
    assert_eq!(state.brake, 0.0);
    assert_eq!(brakes(&state, &handling), Vec3::ZERO);
}

#[test]
fn the_brake_fades_out_at_low_speed_and_is_exactly_zero_at_rest() {
    let handling = test_handling();

    let mut at_rest = ship_moving_forward(0.0);
    at_rest.brake = 100.0;
    assert_eq!(brakes(&at_rest, &handling), Vec3::ZERO);

    let mut crawling = ship_moving_forward(1.0);
    crawling.brake = 100.0;
    let mut fast = ship_moving_forward(100.0);
    fast.brake = 100.0;

    assert!(brakes(&crawling, &handling).length() < brakes(&fast, &handling).length());
}

/// Holding right (`steer > 0`) must turn the ship right, which in this crate's convention needs a
/// **negative** `local_angular.y` contribution (the weathervane direction test in `crate::passive`
/// applies the same identity to another term; the measured law is in
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`). A test checking only `right == -left` would
/// pass with the whole thing inverted, the bug this pins.
#[test]
fn steering_right_yaws_negative_and_is_symmetric_with_left() {
    let handling = test_handling();
    let mut state = ShipState {
        steer: 100.0,
        ..ShipState::default()
    };

    let right = steering(&state, &handling);
    state.steer = -100.0;
    let left = steering(&state, &handling);

    assert!(
        right < 0.0,
        "steering right yawed {right}, expected negative"
    );
    assert_eq!(right, -left);
}

/// Steering authority does not vanish at a standstill: `Turning.amount` feeds
/// yaw with no speed factor at all.
#[test]
fn steering_authority_is_independent_of_speed() {
    let handling = test_handling();
    let stationary = ShipState {
        steer: 50.0,
        ..ShipState::default()
    };
    let mut fast = ship_moving_forward(200.0);
    fast.steer = 50.0;

    assert_eq!(steering(&stationary, &handling), steering(&fast, &handling));
}

/// The reverse-controls pickup is a blend: normal at 0, dead at 0.5, inverted
/// at 1. A boolean would snap where the original ramps.
#[test]
fn reversed_controls_blend_through_dead_rather_than_snapping() {
    let handling = test_handling();
    let mut state = ShipState {
        steer: 100.0,
        ..ShipState::default()
    };

    let normal = steering(&state, &handling);

    state.reverse_controls = 0.0;
    assert_eq!(steering(&state, &handling), normal);

    state.reverse_controls = 0.25;
    assert_eq!(steering(&state, &handling), normal * 0.5);

    state.reverse_controls = 0.5;
    assert_eq!(steering(&state, &handling), 0.0);

    state.reverse_controls = 1.0;
    assert_eq!(steering(&state, &handling), -normal);

    state.reverse_controls = 2.0;
    assert_eq!(steering(&state, &handling), -normal);
}

/// The gains, on the control scale the axis was measured to be on. The expected value carries
/// [`crate::controls::CONTROL_RANGE`], a measurement not bookkeeping: the original's pitch axis
/// reads `+/-100`, not `-1..=1` ([`pitch`]). This once asserted the bare gain and passed while
/// the term was 100x weak.
#[test]
fn the_pitch_axis_uses_a_different_gain_on_the_ground_than_in_the_air() {
    let handling = test_handling();
    let nose_up = ShipControls {
        steer_y: 1.0,
        ..ShipControls::default()
    };
    let scale = crate::controls::CONTROL_RANGE;

    assert_eq!(
        pitch(&nose_up, &handling, true, true, true),
        scale * handling.pitch.pitch_ground
    );
    assert_eq!(
        pitch(&nose_up, &handling, false, true, true),
        scale * handling.pitch.pitch_air
    );
}

/// A nose-up request must produce a positive body-local pitch torque. `oag_physics`' body frame is
/// right-handed on `(x right, y up, z back)` with forward `-Z`, so `omega = +k * x` swings the
/// nose toward `+up`. The original agrees through another convention: d-pad `down` writes `+100`
/// to the pitch axis, into the game-side accumulator, and the basis advance (`e' = e x omega`)
/// tips its forward row up. Both say this term's sign is the sign of "nose up".
#[test]
fn a_nose_up_request_pitches_the_nose_up() {
    let handling = test_handling();
    let nose_up = ShipControls {
        steer_y: 1.0,
        ..ShipControls::default()
    };

    assert!(
        pitch(&nose_up, &handling, false, true, true) > 0.0,
        "steer_y is documented positive nose up, so this must be a positive \
         body-local pitch torque; got {}",
        pitch(&nose_up, &handling, false, true, true)
    );
}

#[test]
fn pitch_is_a_plain_gain_with_no_state_and_is_symmetric() {
    let handling = test_handling();
    let up = ShipControls {
        steer_y: 1.0,
        ..ShipControls::default()
    };
    let down = ShipControls {
        steer_y: -1.0,
        ..ShipControls::default()
    };

    assert_eq!(
        pitch(&up, &handling, false, true, true),
        -pitch(&down, &handling, false, true, true)
    );
    assert_eq!(
        pitch(&ShipControls::default(), &handling, false, true, true),
        0.0
    );
}

/// `Ship_UpdatePitch`: a closed gate drops the input on the ground and in the air.
#[test]
fn a_closed_gate_drops_the_pitch_input() {
    let handling = test_handling();
    let nose_up = ShipControls {
        steer_y: 1.0,
        ..ShipControls::default()
    };
    let scale = crate::controls::CONTROL_RANGE;
    assert_eq!(
        pitch(&nose_up, &handling, false, true, true),
        scale * handling.pitch.pitch_air
    );
    assert_eq!(pitch(&nose_up, &handling, false, false, true), 0.0);
    assert_eq!(pitch(&nose_up, &handling, true, false, true), 0.0);
    assert_eq!(
        pitch(&nose_up, &handling, false, false, false),
        0.0,
        "a title without it"
    );
}

/// The in-air bias is added whether or not the gate passed, and never on the ground.
#[test]
fn the_weight_distribution_bias_is_added_in_the_air_only() {
    let mut handling = test_handling();
    handling.dimensions.weight_distribution = 8.0;
    let nose_up = ShipControls {
        steer_y: 1.0,
        ..ShipControls::default()
    };
    let scale = crate::controls::CONTROL_RANGE;
    assert_eq!(
        pitch(&nose_up, &handling, false, true, true),
        scale * handling.pitch.pitch_air + 8.0
    );
    assert_eq!(pitch(&nose_up, &handling, false, false, true), 8.0);
    assert_eq!(
        pitch(&nose_up, &handling, true, true, true),
        scale * handling.pitch.pitch_ground
    );
    assert_eq!(pitch(&nose_up, &handling, true, false, true), 0.0);
    assert_eq!(
        pitch(&nose_up, &handling, false, false, false),
        0.0,
        "a title without the law adds nothing"
    );
}

/// `dot(hull_up, -down) > 0.9`, strict, and open with no located sample.
#[test]
fn the_pitch_gate_opens_within_its_cone_of_the_track_up() {
    let sample = |down: Vec3| crate::maglock::TrackSample {
        position: Vec3::ZERO,
        down,
    };
    let level = sample(Vec3::new(0.0, -1.0, 0.0));
    assert!(pitch_aligned(Vec3::Y, Some(&level)));
    assert!(
        !pitch_aligned(Vec3::new(0.0, 0.9, 0.0), Some(&level)),
        "0.9 is closed"
    );
    assert!(pitch_aligned(Vec3::new(0.0, 0.9001, 0.0), Some(&level)));
    assert!(!pitch_aligned(Vec3::X, Some(&level)));
    assert!(pitch_aligned(Vec3::X, None));
}

/// `pitch_damping` is consumed by the angular damping term, not by the pitch
/// input, so changing it must not change the pitch response.
#[test]
fn pitch_damping_does_not_affect_the_pitch_input() {
    let controls = ShipControls {
        steer_y: 1.0,
        ..ShipControls::default()
    };
    let mut handling = test_handling();
    let before = pitch(&controls, &handling, true, true, true);
    handling.pitch.pitch_damping *= 10.0;
    assert_eq!(pitch(&controls, &handling, true, true, true), before);
}

/// `if (craft+0x31c < 1.0) T *= craft+0x31c`: a beam's `slowShipFactor` cuts
/// the doubled thrust and nothing else, and the neutral value is a no-op.
#[test]
fn a_thrust_scale_below_one_cuts_the_thrust_and_leaves_the_lift_alone() {
    let state = ShipState {
        thrust: 100.0,
        turbo_timer: 1.0,
        ..ShipState::default()
    };
    let handling = Handling {
        engine: crate::params::Engine {
            amount: 1.0,
            accelcap: 100.0,
            turbo: 10.0,
            ..crate::params::Engine::default()
        },
        ..Handling::ZERO
    };
    let full = engine(
        &state,
        &handling,
        1.0,
        40.0,
        None,
        1.0,
        EngineFlags::default(),
    );
    let throttled = engine(
        &state,
        &handling,
        1.0,
        40.0,
        None,
        0.8,
        EngineFlags::default(),
    );
    assert!(full.thrust > 0.0);
    assert_eq!(
        throttled.thrust,
        full.thrust * 0.8,
        "the scale lands after the doubling"
    );
    assert_eq!(throttled.lift, full.lift, "the lift is not scaled");
    // At or above one it is the neutral value, not a boost.
    assert_eq!(
        engine(
            &state,
            &handling,
            1.0,
            40.0,
            None,
            1.5,
            EngineFlags::default()
        ),
        full
    );
    // And the four-corner branch is scaled too: the original applies it after
    // the `if`/`else`, not inside the throttle arm.
    let zone = engine(
        &state,
        &handling,
        1.0,
        40.0,
        Some(50.0),
        0.5,
        EngineFlags::default(),
    );
    assert_eq!(
        zone.thrust,
        engine(
            &state,
            &handling,
            1.0,
            40.0,
            Some(50.0),
            1.0,
            EngineFlags::default()
        )
        .thrust
            * 0.5
    );
}

/// The Autopilot's `0x4` bit: `1.2` times the thrust, after the cap. At speed 100 the cap
/// (`0.5 * 100 + 20`) is above `100 * 0.4`, so the multiplier is all that moves.
#[test]
fn the_autopilot_speedup_multiplies_thrust_by_one_point_two() {
    let handling = test_handling();
    let mut state = ship_moving_forward(100.0);
    state.thrust = 100.0;
    let plain = engine(
        &state,
        &handling,
        1.0,
        100.0,
        None,
        1.0,
        EngineFlags::default(),
    );
    let flags = EngineFlags {
        speedup: Some(ENGINE_AUTOPILOT_SPEEDUP),
        ..EngineFlags::default()
    };
    let sped = engine(&state, &handling, 1.0, 100.0, None, 1.0, flags);
    assert_eq!(sped.thrust, plain.thrust * ENGINE_AUTOPILOT_SPEEDUP);
}

/// The `0x8` bit: no `0.5 * speed + accelcap` clamp. From a standstill the cap is `20`
/// against the throttle's `40`, so uncapped doubles it; the multiplier applies on top,
/// after where the cap would have been.
#[test]
fn the_uncapped_bit_lifts_the_acceleration_cap() {
    let handling = test_handling();
    let mut state = ship_moving_forward(0.0);
    state.thrust = 100.0;
    let capped = engine(
        &state,
        &handling,
        1.0,
        0.0,
        None,
        1.0,
        EngineFlags::default(),
    )
    .thrust;
    let uncapped = EngineFlags {
        uncapped: true,
        speedup: None,
    };
    let free = engine(&state, &handling, 1.0, 0.0, None, 1.0, uncapped).thrust;
    assert_eq!(free, capped * 2.0, "40 against a cap of 20");
    let both = EngineFlags {
        uncapped: true,
        speedup: Some(ENGINE_AUTOPILOT_SPEEDUP),
    };
    let autopilot = engine(&state, &handling, 1.0, 0.0, None, 1.0, both).thrust;
    assert_eq!(autopilot, free * ENGINE_AUTOPILOT_SPEEDUP);
}

/// Wipeout Pure's engine has no doubling (`oag_physics::laws::Laws::engine_output_double`),
/// measured live on 374 frames: `T = min(throttle * amount, 0.5 * speed + accelcap)`
/// exactly. Undoing the doubling is exact, so Pure gets its own law to the bit.
#[test]
fn undoubling_the_engine_gives_the_capped_thrust_exactly() {
    let handling = test_handling();
    let mut state = ship_moving_forward(10.0);
    state.thrust = 100.0;
    let force = engine(
        &state,
        &handling,
        1.0,
        10.0,
        None,
        1.0,
        EngineFlags::default(),
    );
    // Throttle-bound: 100 * 0.4 = 40 against a cap of 0.5 * 10 + 20 = 25, so the cap.
    assert_eq!(force.thrust, 50.0);
    assert_eq!(force.undoubled().thrust, 25.0);
}
