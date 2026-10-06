//! What the four control states and their ramps in [`super`] are asserted to do. Split out of
//! `controls.rs` under the 200-line cap on inline `#[cfg(test)]` modules (`scripts/check-file-size.py`).

use super::*;
use crate::params::{Airbrake, Brakes, Turning};

/// Arbitrary round numbers. **Not recovered values.**
fn test_handling() -> Handling {
    Handling {
        airbrake: Airbrake {
            gain: 800.0,
            falloff: 400.0,
            ..Airbrake::default()
        },
        brakes: Brakes {
            gain: 400.0,
            falloff: 200.0,
            amount: -0.5,
        },
        turning: Turning {
            gain: 400.0,
            falloff: 200.0,
            amount: 0.01,
        },
        ..Handling::ZERO
    }
}

#[test]
fn the_ramp_rises_at_gain_and_falls_at_falloff() {
    assert_eq!(ramp(0.0, 100.0, 8.0, 16.0, 0.125), 1.0);
    assert_eq!(ramp(4.0, 0.0, 8.0, 16.0, 0.125), 2.0);
}

#[test]
fn the_ramp_never_overshoots_its_target() {
    assert_eq!(ramp(0.0, 1.0, 1000.0, 1000.0, 0.5), 1.0);
    assert_eq!(ramp(1.0, 0.25, 1000.0, 1000.0, 0.5), 0.25);
    assert_eq!(ramp(0.5, 0.5, 1000.0, 1000.0, 0.5), 0.5);
}

/// The steering ramp's asymmetry is per side of centre, not per sign of the
/// state, so the same input mirrored gives the same rate.
#[test]
fn the_steering_ramp_uses_gain_outward_and_falloff_inward_on_both_sides() {
    // Outward, both directions, at `gain`.
    assert_eq!(ramp_steering(0.0, 100.0, 8.0, 16.0, 0.125), 1.0);
    assert_eq!(ramp_steering(0.0, -100.0, 8.0, 16.0, 0.125), -1.0);
    // Past the target, coming back, at `falloff`.
    assert_eq!(ramp_steering(4.0, 1.0, 8.0, 16.0, 0.125), 2.0);
    assert_eq!(ramp_steering(-4.0, -1.0, 8.0, 16.0, 0.125), -2.0);
}

#[test]
fn the_steering_ramp_reaches_centre_exactly_rather_than_crossing_it() {
    assert_eq!(ramp_steering(1.0, 0.0, 8.0, 16.0, 1.0), 0.0);
    assert_eq!(ramp_steering(-1.0, 0.0, 8.0, 16.0, 1.0), 0.0);
    assert_eq!(ramp_steering(0.0, 0.0, 8.0, 16.0, 1.0), 0.0);
}

/// The throttle is the raw input, scaled. Pinned because reintroducing the
/// dead `<Engine gain/>` ramp is the obvious mistake here.
#[test]
fn the_throttle_is_not_ramped() {
    let handling = Handling {
        engine: crate::params::Engine {
            gain: 1.0,
            falloff: 1.0,
            ..crate::params::Engine::default()
        },
        ..test_handling()
    };
    let mut state = ShipState::default();

    update(
        &mut state,
        &ShipControls {
            thrust: 1.0,
            ..ShipControls::default()
        },
        &handling,
        1.0 / 60.0,
    );
    assert_eq!(state.thrust, CONTROL_RANGE);

    update(&mut state, &ShipControls::default(), &handling, 1.0 / 60.0);
    assert_eq!(state.thrust, 0.0);
}

#[test]
fn the_brake_engages_only_when_both_airbrakes_are_held() {
    let handling = test_handling();
    let dt = 1.0 / 60.0;

    let mut one_side = ShipState::default();
    update(
        &mut one_side,
        &ShipControls {
            airbrake_left: 1.0,
            ..ShipControls::default()
        },
        &handling,
        dt,
    );
    assert_eq!(one_side.brake, 0.0);

    let mut both = ShipState::default();
    update(
        &mut both,
        &ShipControls {
            airbrake_left: 1.0,
            airbrake_right: 1.0,
            ..ShipControls::default()
        },
        &handling,
        dt,
    );
    assert!(both.brake > 0.0);
}

#[test]
fn the_brake_ramp_is_bounded_at_both_ends() {
    let handling = test_handling();
    let mut state = ShipState::default();
    let held = ShipControls {
        airbrake_left: 1.0,
        airbrake_right: 1.0,
        ..ShipControls::default()
    };

    for _ in 0..600 {
        update(&mut state, &held, &handling, 1.0 / 60.0);
    }
    assert_eq!(state.brake, CONTROL_MAX);

    for _ in 0..600 {
        update(&mut state, &ShipControls::default(), &handling, 1.0 / 60.0);
    }
    assert_eq!(state.brake, 0.0);
}

#[test]
fn the_airbrake_states_saturate_at_the_control_range() {
    let handling = test_handling();
    let mut state = ShipState::default();
    let held = ShipControls {
        airbrake_left: 1.0,
        airbrake_right: 0.5,
        ..ShipControls::default()
    };

    for _ in 0..600 {
        update(&mut state, &held, &handling, 1.0 / 60.0);
    }
    assert_eq!(state.airbrake_left, CONTROL_RANGE);
    assert_eq!(state.airbrake_right, CONTROL_RANGE * 0.5);
}

/// The weapon slowdown timer is **not** one of the ramps this function owns (it was, while the field
/// was believed to be a leap timer). Its decay is the original's `0x08849a24`, at the end of the
/// craft update, and lives in `crate::forces::evaluate`.
#[test]
fn the_slowdown_timer_is_not_touched_here() {
    let handling = test_handling();
    let mut state = ShipState {
        slowdown_timer: 0.1,
        ..ShipState::default()
    };

    update(&mut state, &ShipControls::default(), &handling, 0.0625);
    assert_eq!(state.slowdown_timer, 0.1);
}

#[test]
fn a_zero_delta_leaves_every_ramped_state_where_it_was() {
    let handling = test_handling();
    let mut state = ShipState {
        airbrake_left: 30.0,
        airbrake_right: 70.0,
        brake: 25.0,
        steer: -40.0,
        slowdown_timer: 2.0,
        ..ShipState::default()
    };
    let before = state;

    update(
        &mut state,
        &ShipControls {
            steer_x: 1.0,
            airbrake_left: 1.0,
            airbrake_right: 1.0,
            ..ShipControls::default()
        },
        &handling,
        0.0,
    );

    assert_eq!(state.airbrake_left, before.airbrake_left);
    assert_eq!(state.airbrake_right, before.airbrake_right);
    assert_eq!(state.brake, before.brake);
    assert_eq!(state.steer, before.steer);
    assert_eq!(state.slowdown_timer, before.slowdown_timer);
}

const DT: f32 = 1.0 / 60.0;

fn lean_after(ticks: usize, stick: f32) -> ShipState {
    let mut state = ShipState::default();
    for _ in 0..ticks {
        update_camera_lean(&mut state, stick, DT);
    }
    state
}

/// The lean leans with the stick's sign, never overshoots the stick, and
/// settles on it: the delta is clamped, not the value.
#[test]
fn the_camera_lean_settles_on_the_stick_and_never_overshoots() {
    let right = lean_after(600, 1.0);
    assert!(
        (right.camera_lean - 1.0).abs() < 1.0e-3,
        "{}",
        right.camera_lean
    );
    assert!(lean_after(600, -0.5).camera_lean < 0.0);
    for ticks in 1..200 {
        assert!(lean_after(ticks, 1.0).camera_lean <= 1.0);
    }
}

/// One tick from rest on full lock: the follower moves by `min(delta, 0.6) *
/// 5.4 * dt`, and the lean by `follower * 4 * dt` of that.
#[test]
fn the_camera_lean_first_tick_matches_the_decompiled_arithmetic() {
    let state = lean_after(1, 1.0);
    let follower = 0.6 * 5.4 * DT;
    assert_eq!(state.camera_lean_follower, follower);
    assert_eq!(state.camera_lean, follower * DT * 4.0);
}

/// Returning to centre is held to `0.3` against `0.6` going out, so the lean
/// lets go half as fast as it takes hold.
#[test]
fn the_camera_lean_returns_slower_than_it_leans_out() {
    let out = lean_after(1, 1.0).camera_lean_follower;
    let mut state = ShipState {
        camera_lean_follower: 1.0,
        ..ShipState::default()
    };
    update_camera_lean(&mut state, 0.0, DT);
    let back = 1.0 - state.camera_lean_follower;
    assert!((back * 2.0 - out).abs() < 1.0e-6, "{back} vs {out}");
}

/// `controls::update` feeds the lean from the raw stick, not the ramped steer.
#[test]
fn update_drives_the_camera_lean_from_the_raw_stick() {
    let mut state = ShipState::default();
    let controls = ShipControls {
        steer_x: 1.0,
        ..ShipControls::default()
    };
    update(&mut state, &controls, &test_handling(), DT);
    assert!(state.camera_lean_follower > 0.0);
    assert!(state.steer < 100.0);
    assert!(state.camera_lean_follower > 0.0 && state.camera_lean > 0.0);
}
