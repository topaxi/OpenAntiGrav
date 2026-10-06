//! What the airbrake, lateral-grip and sideshift laws in [`super`] are asserted to do. Its own
//! file because the tests are 650 lines against the laws' 490 (`scripts/check-file-size.py`).

use super::*;
use crate::params::{Airbrake, Antigrav};
use crate::ship::Body;

/// Arbitrary round numbers, in the scaled in-memory form: `amount` and
/// `slidegrip` are what the loader would have stored, not what the XML holds.
/// **Not recovered values.**
fn test_handling() -> Handling {
    Handling {
        airbrake: Airbrake {
            // An XML `amount` of 20 would be stored as 0.002.
            amount: 0.002,
            drag: 5.0,
            falloff: 400.0,
            gain: 800.0,
            turn: 3.0,
            // An XML `slidegrip` of 50 is stored as 0.005.
            slidegrip: 0.005,
            sideshift: 6.0,
        },
        antigrav: Antigrav {
            grip_air: 1.0,
            grip_ground: 2.0,
            ..Antigrav::default()
        },
        ..Handling::ZERO
    }
}

/// Forty units per second forwards and three to the right. Forward is `-Z`.
fn moving_ship() -> ShipState {
    ShipState {
        body: Body {
            linear_velocity: Vec3::new(3.0, 0.0, -40.0),
            ..Body::default()
        },
        ..ShipState::default()
    }
}

/// Braking one side must turn the nose **toward** that side: a **positive** `local_angular.y` for
/// the left brake in this crate's frame, the convention [`crate::engine::steering`] satisfies by
/// negating its literal law (and `crate::forces`'s
/// `holding_right_turns_the_ship_toward_its_own_right_axis` checks for steering). A test asserting
/// only `left == -right` would pass with the term inverted, the bug this pins: from `5ad69f3`
/// until Task #34 the airbrake yaw ran the other way, as the steering fix was applied in
/// `engine::steering` alone.
#[test]
fn braking_the_left_side_yaws_the_nose_left_and_pushes_the_body_right() {
    let handling = test_handling();
    let mut state = moving_ship();
    state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
    let input = ShipControls::default();

    state.airbrake_left = 100.0;
    state.airbrake_right = 0.0;
    let left_brake = evaluate(&state, &input, &handling, 40.0);

    state.airbrake_left = 0.0;
    state.airbrake_right = 100.0;
    let right_brake = evaluate(&state, &input, &handling, 40.0);

    assert!(
        left_brake.local_angular.y > 0.0,
        "the left brake yawed {}, and nose-left is positive here",
        left_brake.local_angular.y
    );
    assert!(
        right_brake.local_angular.y < 0.0,
        "the right brake yawed {}, and nose-right is negative here",
        right_brake.local_angular.y
    );

    // The original writes the lateral term along `craft+0x170`, the ship's left, so braking left
    // pushes the body right (the craft rotates into the corner while its mass runs wide). `+X` is
    // right and the ship is aimed along `-Z`, so `.x` isolates it.
    assert!(
        left_brake.world_force.x > 0.0,
        "the left brake pushed {} laterally, expected +X (right)",
        left_brake.world_force.x
    );
    assert!(right_brake.world_force.x < 0.0);
}

/// The confidence-85 negative, pinned as a test because it is the exact thing a
/// reimplementation is tempted to "fix".
#[test]
fn the_airbrake_path_never_writes_a_roll_torque() {
    let handling = test_handling();
    let mut state = moving_ship();

    for (left, right) in [(100.0, 0.0), (0.0, 100.0), (100.0, 100.0), (35.0, 75.0)] {
        state.airbrake_left = left;
        state.airbrake_right = right;
        for steer in [-1.0, 0.0, 0.6] {
            let input = ShipControls {
                steer_x: steer,
                ..ShipControls::default()
            };
            let forces = evaluate(&state, &input, &handling, 40.0);
            assert_eq!(
                forces.local_angular.z, 0.0,
                "left {left} right {right} steer {steer}"
            );
            assert_eq!(forces.local_angular.x, 0.0);
        }
    }
}

/// Symmetric input, symmetric result: with the two sides equal there is no
/// imbalance, so no lateral force and no yaw, whatever else is going on.
#[test]
fn equal_airbrakes_produce_no_lateral_force_and_no_yaw() {
    let handling = test_handling();
    let mut state = moving_ship();
    state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
    state.airbrake_left = 60.0;
    state.airbrake_right = 60.0;

    let input = ShipControls {
        steer_x: 0.5,
        ..ShipControls::default()
    };
    let forces = evaluate(&state, &input, &handling, 40.0);

    assert_eq!(forces.local_angular, Vec3::ZERO);
    assert_eq!(forces.world_force.x, 0.0);
}

/// Mirroring the two sides mirrors the result exactly, which pins that no term
/// treats one side preferentially.
#[test]
fn mirroring_the_airbrakes_negates_the_lateral_force_and_the_yaw() {
    let handling = test_handling();
    let input = ShipControls {
        steer_x: 0.5,
        ..ShipControls::default()
    };

    let mut state = moving_ship();
    state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);

    state.airbrake_left = 100.0;
    state.airbrake_right = 0.0;
    let left_heavy = evaluate(&state, &input, &handling, 40.0);

    state.airbrake_left = 0.0;
    state.airbrake_right = 100.0;
    let right_heavy = evaluate(&state, &input, &handling, 40.0);

    assert_eq!(left_heavy.local_angular, -right_heavy.local_angular);
    assert_eq!(left_heavy.world_force.x, -right_heavy.world_force.x);
    // The forward slide term uses |L - R| and so is unchanged by mirroring.
    assert_eq!(left_heavy.world_force.z, right_heavy.world_force.z);
}

#[test]
fn the_slide_term_needs_both_an_imbalance_and_steering() {
    let handling = test_handling();
    let mut state = moving_ship();
    state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
    state.airbrake_left = 100.0;
    state.airbrake_right = 0.0;

    let straight = evaluate(&state, &ShipControls::default(), &handling, 40.0);
    assert_eq!(straight.world_force.z, 0.0);

    let turning = evaluate(
        &state,
        &ShipControls {
            steer_x: 1.0,
            ..ShipControls::default()
        },
        &handling,
        40.0,
    );
    assert_ne!(turning.world_force.z, 0.0);
}

/// The `drag` term's exact magnitude, in the binary's own association order. Asserted against a
/// recomputed product, not a decimal literal: `0.01f32` and `0.001f32` are inexact, so a literal
/// would fail or force a tolerance that pins nothing.
#[test]
fn the_airbrake_drag_term_has_the_magnitude_the_instruction_stream_forms() {
    let handling = test_handling();
    let mut state = moving_ship();
    state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
    state.airbrake_left = 100.0;
    state.airbrake_right = 0.0;
    let input = ShipControls {
        steer_x: 1.0,
        ..ShipControls::default()
    };

    let forces = evaluate(&state, &input, &handling, 40.0);

    // `slide` first, then `(forward * speed) * slide * 0.001` (the two `vscl.q`s and two literals,
    // in order). A full `steer_x` of `1.0` is the snapshot's `100.0`: `Ship_UpdateSteering`
    // compares that field with the `+/-100` ramped state, so dropping the scale runs this term
    // 100x weak, as it did until 2026-09-30.
    let slide = 100.0f32 * handling.airbrake.drag * 100.0 * 0.01;
    let expected = -(40.0f32 * slide * 0.001);

    // Forward is `-Z`, and the lateral term is along `+X`, so `.z` isolates
    // the drag term.
    assert_eq!(forces.world_force.z, expected);
    assert_eq!(forces.world_force.y, 0.0);
    assert!(expected < 0.0, "the term must point along +forward");
}

/// Both factors are needed, and "zero" means exactly zero rather than small.
#[test]
fn the_drag_term_is_exactly_zero_without_both_an_imbalance_and_steering() {
    let handling = test_handling();
    let mut state = moving_ship();
    state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);

    // Equal airbrakes, full steering.
    state.airbrake_left = 70.0;
    state.airbrake_right = 70.0;
    let equal = evaluate(
        &state,
        &ShipControls {
            steer_x: 1.0,
            ..ShipControls::default()
        },
        &handling,
        40.0,
    );
    assert_eq!(equal.world_force.z, 0.0);

    // Full imbalance, no steering.
    state.airbrake_left = 100.0;
    state.airbrake_right = 0.0;
    let straight = evaluate(&state, &ShipControls::default(), &handling, 40.0);
    assert_eq!(straight.world_force.z, 0.0);
}

/// The gate is on `craft+0x2ec`, the **absolute** cached speed, so a
/// reversing ship gets the term too - pointed along `+forward`, which for it
/// is a deceleration. A gate on a signed forward speed would zero this.
#[test]
fn the_drag_term_points_along_forward_in_both_directions_of_travel() {
    let handling = test_handling();
    let mut state = moving_ship();
    state.airbrake_left = 100.0;
    state.airbrake_right = 0.0;
    let input = ShipControls {
        steer_x: 1.0,
        ..ShipControls::default()
    };

    state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
    let forwards = evaluate(&state, &input, &handling, 40.0);

    state.body.linear_velocity = Vec3::new(0.0, 0.0, 40.0);
    let reversing = evaluate(&state, &input, &handling, -40.0);

    assert_eq!(reversing.world_force.z, forwards.world_force.z);
    assert!(forwards.world_force.z < 0.0);
}

/// The slide term reads the **raw** `steerX` from the input snapshot, while
/// the airbrake sides are the ramped states. Pinned because unifying the two
/// is a natural-looking tidy-up that would diverge.
#[test]
fn the_drag_term_reads_the_raw_steering_input_not_a_ramped_state() {
    let handling = test_handling();
    let mut state = moving_ship();
    state.body.linear_velocity = Vec3::new(0.0, 0.0, -40.0);
    state.airbrake_left = 100.0;
    state.airbrake_right = 0.0;

    let half = evaluate(
        &state,
        &ShipControls {
            steer_x: 0.5,
            ..ShipControls::default()
        },
        &handling,
        40.0,
    );
    let full = evaluate(
        &state,
        &ShipControls {
            steer_x: 1.0,
            ..ShipControls::default()
        },
        &handling,
        40.0,
    );

    // Linear in the raw input, with no ramp state anywhere in between.
    assert_eq!(half.world_force.z, full.world_force.z * 0.5);
    // And it is `|steerX|`, so the opposite lock gives the same thing.
    let mirrored = evaluate(
        &state,
        &ShipControls {
            steer_x: -1.0,
            ..ShipControls::default()
        },
        &handling,
        40.0,
    );
    assert_eq!(mirrored.world_force.z, full.world_force.z);
}

#[test]
fn a_stationary_ship_gets_nothing_from_the_airbrakes() {
    let handling = test_handling();
    let state = ShipState {
        airbrake_left: 100.0,
        ..ShipState::default()
    };
    let forces = evaluate(
        &state,
        &ShipControls {
            steer_x: 1.0,
            ..ShipControls::default()
        },
        &handling,
        0.0,
    );
    assert_eq!(forces.world_force, Vec3::ZERO);
    assert_eq!(forces.local_angular, Vec3::ZERO);
}

#[test]
fn lateral_grip_opposes_sideways_velocity() {
    let handling = test_handling();
    let state = moving_ship();
    // Sliding right, so the grip term must push left, along local -X.
    assert!(lateral_grip(&state, &handling, 1.0).x < 0.0);
}

/// The two endpoints of the resolved `slidegrip` reading, on the scaled value.
#[test]
fn grip_is_unchanged_by_the_airbrakes_at_a_slidegrip_of_one_hundred() {
    let handling = Handling {
        airbrake: Airbrake {
            // XML 100, scaled by 1e-4.
            slidegrip: 0.01,
            ..test_handling().airbrake
        },
        ..test_handling()
    };
    assert_eq!(lateral_grip_coefficient(&handling, 0.0, 0.0), -1.0);
    assert_eq!(lateral_grip_coefficient(&handling, 100.0, 100.0), -1.0);
}

#[test]
fn grip_vanishes_at_full_airbrake_with_a_slidegrip_of_zero() {
    let handling = Handling {
        airbrake: Airbrake {
            slidegrip: 0.0,
            ..test_handling().airbrake
        },
        ..test_handling()
    };
    assert_eq!(lateral_grip_coefficient(&handling, 0.0, 0.0), -1.0);
    assert_eq!(lateral_grip_coefficient(&handling, 100.0, 0.0), 0.0);
}

/// Half airbrake lands between the two endpoints, which is what makes this a
/// slide control rather than a switch.
#[test]
fn grip_falls_off_progressively_between_those_endpoints() {
    let handling = Handling {
        airbrake: Airbrake {
            slidegrip: 0.0,
            ..test_handling().airbrake
        },
        ..test_handling()
    };
    let none = lateral_grip_coefficient(&handling, 0.0, 0.0);
    let half = lateral_grip_coefficient(&handling, 50.0, 0.0);
    let full = lateral_grip_coefficient(&handling, 100.0, 0.0);

    assert_eq!(none, -1.0);
    assert_eq!(full, 0.0);
    assert!(half > none && half < full);
}

#[test]
fn groundedness_selects_between_the_two_grip_coefficients() {
    let handling = test_handling();
    let state = moving_ship();

    let ground = lateral_grip(&state, &handling, 1.0).x;
    let air = lateral_grip(&state, &handling, 0.0).x;
    let half = lateral_grip(&state, &handling, 0.5).x;

    // `grip_ground` is twice `grip_air` in the fixture, so grounded grip is
    // stronger and half-grounded sits between the two.
    assert!(ground < air);
    assert!(half < air && half > ground);
}

/// A sideshift pushes toward the side it was fired at, for as long as its timer runs, and only
/// while grounded. The direction is the recovered one ([`sideshift_force`]), asserted both ways
/// round so an unconditional push cannot pass.
#[test]
fn a_sideshift_pushes_toward_the_side_it_was_fired_at_while_grounded() {
    let handling = test_handling();
    let mut state = ShipState::default();

    // Nothing fired: no force, and the timers stay down.
    advance_sideshift(&mut state, &ShipControls::default(), 1.0 / 60.0);
    assert_eq!(sideshift_force(&state, &handling, 1.0), Vec3::ZERO);

    let fire = |side| ShipControls {
        sideshift: side,
        ..ShipControls::default()
    };

    let mut right = ShipState::default();
    advance_sideshift(&mut right, &fire(Sideshift::Right), 1.0 / 60.0);
    assert_eq!(
        sideshift_force(&right, &handling, 1.0),
        Vec3::new(6.0, 0.0, 0.0)
    );
    // Switched off in the air, which the old velocity-change shape was not.
    assert_eq!(sideshift_force(&right, &handling, 0.0), Vec3::ZERO);

    let mut left = ShipState::default();
    advance_sideshift(&mut left, &fire(Sideshift::Left), 1.0 / 60.0);
    assert_eq!(
        sideshift_force(&left, &handling, 1.0),
        Vec3::new(-6.0, 0.0, 0.0)
    );

    // Both at once cancel, as the original's two flags do.
    let mut both = ShipState::default();
    advance_sideshift(&mut both, &fire(Sideshift::Left), 1.0 / 60.0);
    advance_sideshift(&mut both, &fire(Sideshift::Right), 1.0 / 60.0);
    assert_eq!(sideshift_force(&both, &handling, 1.0), Vec3::ZERO);
}

/// It lasts [`SIDESHIFT_DURATION`] and then stops - it is not one frame, and
/// it is not forever.
#[test]
fn a_sideshift_pushes_for_its_whole_timer_and_then_stops() {
    let handling = test_handling();
    let mut state = ShipState::default();
    let dt = 1.0 / 60.0;

    advance_sideshift(
        &mut state,
        &ShipControls {
            sideshift: Sideshift::Right,
            ..ShipControls::default()
        },
        dt,
    );

    let mut pushing = 0;
    for _ in 0..60 {
        if sideshift_force(&state, &handling, 1.0) != Vec3::ZERO {
            pushing += 1;
        }
        advance_sideshift(&mut state, &ShipControls::default(), dt);
    }

    // 0.2 s at 60 Hz, plus the frame it fired on: the original arms the timer
    // after the countdown, in the same order this does, so a fired sideshift
    // always gets its whole duration and never a partial first frame.
    assert_eq!(pushing, (SIDESHIFT_DURATION / dt).round() as u32 + 1);
}

const DT: f32 = 1.0 / 60.0;

/// Novice: hold the sideshift button, centre the stick, flick.
fn flick(state: &mut ShipState, steer_x: f32) {
    advance_sideshift(
        state,
        &ShipControls {
            shift_modifier: true,
            steer_x,
            ..ShipControls::default()
        },
        DT,
    );
}

/// Veteran: one press of an airbrake, then `gap` ticks of nothing.
fn tap(state: &mut ShipState, left: bool, gap: u32) {
    advance_sideshift(
        state,
        &ShipControls {
            shift_tap_left: left,
            shift_tap_right: !left,
            ..ShipControls::default()
        },
        DT,
    );
    for _ in 0..gap {
        advance_sideshift(state, &ShipControls::default(), DT);
    }
}

/// The flick has to arm before it can fire, and fires toward the flick. The latch is the point: a
/// pilot already holding the stick over when they press the button must not get a sideshift for
/// free, nor one every tick after.
#[test]
fn a_novice_flick_arms_inside_the_threshold_and_fires_outside_it() {
    let mut state = ShipState::default();

    // Stick already over: the modifier is held but nothing arms, so nothing
    // fires however long it is held.
    for _ in 0..30 {
        flick(&mut state, 1.0);
    }
    assert!(!state.shift_armed, "an off-centre stick never arms");
    assert_eq!(state.sideshift_timers, [0.0, 0.0]);

    // Centre the stick to arm, then flick right.
    flick(&mut state, 0.0);
    assert!(state.shift_armed);
    flick(&mut state, 1.0);
    assert_eq!(state.sideshift_timers[1], SIDESHIFT_DURATION);
    assert_eq!(state.sideshift_timers[0], 0.0, "the flick went right");
    assert!(!state.shift_armed, "firing disarms");
}

/// A left flick is the mirror image, and the axis sign is the binding.
#[test]
fn a_novice_flick_left_shifts_left() {
    let mut state = ShipState::default();
    flick(&mut state, 0.0);
    flick(&mut state, -1.0);
    assert_eq!(state.sideshift_timers[0], SIDESHIFT_DURATION);
    assert_eq!(state.sideshift_timers[1], 0.0);
}

/// Veteran: two taps inside the window shift, one tap does not. A single tap is an ordinary
/// airbrake press; this catches a window left permanently open.
#[test]
fn a_veteran_double_tap_shifts_and_a_single_tap_does_not() {
    let mut state = ShipState::default();

    tap(&mut state, true, 4);
    assert_eq!(state.sideshift_timers[0], 0.0, "one tap is not a shift");
    assert!(state.shift_tap_windows[0] > 0.0, "the window is open");

    tap(&mut state, true, 0);
    assert_eq!(state.sideshift_timers[0], SIDESHIFT_DURATION);
}

/// The window closes, and a slow second tap opens a new one instead.
#[test]
fn a_veteran_tap_outside_the_window_is_a_first_tap_again() {
    let mut state = ShipState::default();

    let gap = (SIDESHIFT_TAP_WINDOW / DT).ceil() as u32 + 1;
    tap(&mut state, false, gap);
    assert_eq!(state.shift_tap_windows[1], 0.0, "the window expired");

    tap(&mut state, false, 0);
    assert_eq!(state.sideshift_timers[1], 0.0, "and this is tap one again");
    assert!(state.shift_tap_windows[1] > 0.0);
}

/// One second between shifts, whichever gesture asked for the second. The lockout is refreshed
/// while the shift runs, so it is measured from the shift's end: total gap is duration plus lockout.
#[test]
fn the_lockout_holds_off_the_next_gesture_for_a_second() {
    let mut state = ShipState::default();
    flick(&mut state, 0.0);
    flick(&mut state, 1.0);
    assert_eq!(state.sideshift_timers[1], SIDESHIFT_DURATION);

    // Immediately try the other gesture, twice, well inside the lockout.
    tap(&mut state, true, 2);
    tap(&mut state, true, 0);
    assert_eq!(state.sideshift_timers[0], 0.0, "locked out");

    // Run out the shift and the lockout, then the same double tap works.
    let ticks = ((SIDESHIFT_DURATION + SIDESHIFT_LOCKOUT) / DT).ceil() as u32 + 1;
    for _ in 0..ticks {
        advance_sideshift(&mut state, &ShipControls::default(), DT);
    }
    assert_eq!(state.shift_lockout, 0.0);
    tap(&mut state, true, 2);
    tap(&mut state, true, 0);
    assert_eq!(state.sideshift_timers[0], SIDESHIFT_DURATION);
}

/// The dormant scheme's fields are inert, which lets both machines run side by side.
/// `oag_gameplay::controls::ship_controls` sets only one scheme's fields; this pins that the
/// other machine does nothing without them.
#[test]
fn a_scheme_that_sends_nothing_produces_nothing() {
    let mut state = ShipState::default();
    for _ in 0..120 {
        advance_sideshift(
            &mut state,
            &ShipControls {
                steer_x: 1.0,
                airbrake_left: 1.0,
                airbrake_right: 1.0,
                ..ShipControls::default()
            },
            DT,
        );
    }
    assert_eq!(state.sideshift_timers, [0.0, 0.0]);
    assert_eq!(state.shift_tap_windows, [0.0, 0.0]);
    assert!(!state.shift_armed);
}

#[test]
fn a_zero_handling_ship_gets_nothing_from_the_airbrake_path() {
    let state = moving_ship();
    let input = ShipControls {
        steer_x: 1.0,
        steer_y: 0.0,
        thrust: 1.0,
        airbrake_left: 1.0,
        airbrake_right: 0.0,
        sideshift: Sideshift::Right,
        // Every gesture input on as well, so this asserts the airbrake path
        // is inert on zero parameters rather than merely untriggered.
        shift_modifier: true,
        shift_tap_left: true,
        shift_tap_right: true,
        roll_tap_left: true,
        roll_tap_right: true,
        // The barrel roll's direct request and its invented shield floor too,
        // so this covers the AI's path and not only a pad's.
        roll_request: Some(crate::barrel_roll::TapDirection::Right),
        roll_shield_floor: 0.2,
    };
    let forces = evaluate(&state, &input, &Handling::ZERO, 40.0);
    assert_eq!(forces.world_force, Vec3::ZERO);
    assert_eq!(forces.local_angular, Vec3::ZERO);
    assert_eq!(lateral_grip(&state, &Handling::ZERO, 1.0), Vec3::ZERO);
    let mut fired = state;
    advance_sideshift(&mut fired, &input, 1.0 / 60.0);
    assert_eq!(sideshift_force(&fired, &Handling::ZERO, 1.0), Vec3::ZERO);
}
