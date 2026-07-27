//! The four control states, and the ramps that drive them.
//!
//! The original keeps throttle, brake, steering and the two airbrake sides as
//! floats in adjacent craft slots (`craft+0x2b8` through `+0x2c8`) and ramps them
//! at rates from the parameter set. Three of the five ramps are live. The
//! throttle's is not: see [`crate::params::Engine::falloff`].
//!
//! Evidence: `docs/ghidra/functions/psp-pulse/engine.md`, sections "Engine",
//! "Brakes" and "Steering".

use crate::params::Handling;
use crate::ship::{ShipControls, ShipState};

/// The scale the original's control states use.
///
/// [`ShipControls`] is normalised, `-1..=1` on the axes and `0..=1` on the
/// triggers, because that is the contract `oag-gameplay` owns. **The original's
/// internal states are percentages**, and the difference is not cosmetic: three
/// separate pieces of arithmetic only come out right on the `0..=100` scale.
///
/// - The brake ramp clamps at `100.0`.
/// - The engine compares `throttle > 100.0` to detect an over-unity throttle.
/// - The lateral grip coefficient `max(L, R) * (0.01 - slidegrip) - 1.0` reaches
///   exactly `0` at full airbrake with `slidegrip = 0`, which needs `max(L, R)` to
///   reach `100` while `slidegrip` runs `0..0.01`.
///
/// So the conversion happens here, once, where a normalised input becomes a state.
///
/// **The steering axis is now measured, and it is on this scale.** Both captures in
/// `data/traces/` show `craft+0x2c0` ramping at 498/s and falling at 924/s under
/// full deflection, which reproduces the shipped `Turning.gain` and
/// `Turning.falloff` to under 1 % only on the `0..=100` reading. That retires the
/// inference for this axis; `stick_y` is still untested, since the craft control
/// block holds no second axis to capture.
///
/// **Do not "fix" the yaw authority by dividing it by `CONTROL_RANGE`.** An earlier
/// version of this comment warned that an unscaled `Turning.amount` would put yaw
/// out by whatever factor the axis scale was wrong by, and invited exactly that
/// edit. It is falsified: the axis scale is right, `HandlingXml_ParseTurning`
/// really does store `amount` verbatim (`swc1 f0,0xd0(a0)`), and the yaw error is
/// **22x, not 100x**. Dividing by 100 here lands 4.7x too *weak* - measured, RMS
/// error 1.05 rad/s against captures whose signal is 1.5. The real discrepancy and
/// what stands in for it are documented at
/// [`crate::forces::YAW_DRIVE_CALIBRATION`].
pub const CONTROL_RANGE: f32 = 100.0;

/// The brake state's ceiling, and so every control state's nominal maximum.
pub const CONTROL_MAX: f32 = 100.0;

/// Ramps one control toward a non-negative target at `gain` up and `falloff` down.
///
/// Both rates are per-second, and the result never overshoots the target. A
/// per-second *exponential* decay would also fit "per-second rate" and is not what
/// is implemented; the linear reading is a **guess awaiting M3**, chosen because it
/// cannot overshoot and so cannot introduce an oscillation that would be mistaken
/// for a physics bug.
#[must_use]
pub fn ramp(current: f32, target: f32, gain: f32, falloff: f32, dt: f32) -> f32 {
    if target > current {
        (current + gain * dt).min(target)
    } else {
        (current - falloff * dt).max(target)
    }
}

/// Ramps the steering state toward a signed target.
///
/// Not the same shape as [`ramp`]: `gain` is the rate while moving toward a
/// **larger-magnitude** target and `falloff` the rate while returning toward
/// centre, on both sides of zero, and the centre is reached exactly rather than
/// asymptotically. So the same pair of parameters gives a fast bite with a slow
/// return, or the reverse, and the asymmetry is intended rather than a sign
/// artefact. Transcribed from `Ship_UpdateSteering` (`0x08848788`), confidence 84.
#[must_use]
pub fn ramp_steering(current: f32, target: f32, gain: f32, falloff: f32, dt: f32) -> f32 {
    if target > 0.0 {
        if current < target {
            current + gain * dt
        } else {
            current - falloff * dt
        }
    } else if target < 0.0 {
        if current > target {
            current - gain * dt
        } else {
            current + falloff * dt
        }
    } else {
        // Decay to exactly zero from whichever side, rather than crossing it.
        if current > 0.0 {
            let next = current - falloff * dt;
            if next < 0.0 { 0.0 } else { next }
        } else if current < 0.0 {
            let next = current + falloff * dt;
            if next > 0.0 { 0.0 } else { next }
        } else {
            current
        }
    }
}

/// Advances every control state by one frame.
///
/// Ordering note: the states are advanced before any force is evaluated, which is
/// what the original does - the engine, brakes, airbrakes and steering each ramp
/// their own state at the top of their own update, and all five updates precede
/// every consumer of those states.
pub fn update(state: &mut ShipState, controls: &ShipControls, handling: &Handling, dt: f32) {
    // The throttle is **not** ramped. The original computes a ramp, stores it, and
    // overwrites it with the raw input before reading it back; reproducing the
    // ramp would add throttle lag that is not in the game.
    state.thrust = controls.thrust * CONTROL_RANGE;

    state.airbrake_left = ramp(
        state.airbrake_left,
        controls.airbrake_left * CONTROL_RANGE,
        handling.airbrake.gain,
        handling.airbrake.falloff,
        dt,
    );
    state.airbrake_right = ramp(
        state.airbrake_right,
        controls.airbrake_right * CONTROL_RANGE,
        handling.airbrake.gain,
        handling.airbrake.falloff,
        dt,
    );

    // The brake has no axis of its own: both airbrakes at once is the brake.
    if controls.airbrake_left > 0.0 && controls.airbrake_right > 0.0 {
        state.brake = (state.brake + handling.brakes.gain * dt).min(CONTROL_MAX);
    } else {
        state.brake = (state.brake - handling.brakes.falloff * dt).max(0.0);
    }

    state.steer = ramp_steering(
        state.steer,
        controls.steer_x * CONTROL_RANGE,
        handling.turning.gain,
        handling.turning.falloff,
        dt,
    );

    state.leap_timer = (state.leap_timer - dt).max(0.0);
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn the_leap_timer_counts_down_and_stops_at_zero() {
        let handling = test_handling();
        let mut state = ShipState {
            leap_timer: 0.1,
            ..ShipState::default()
        };

        update(&mut state, &ShipControls::default(), &handling, 0.0625);
        assert_eq!(state.leap_timer, 0.1 - 0.0625);

        for _ in 0..10 {
            update(&mut state, &ShipControls::default(), &handling, 0.0625);
        }
        assert_eq!(state.leap_timer, 0.0);
    }

    #[test]
    fn a_zero_delta_leaves_every_ramped_state_where_it_was() {
        let handling = test_handling();
        let mut state = ShipState {
            airbrake_left: 30.0,
            airbrake_right: 70.0,
            brake: 25.0,
            steer: -40.0,
            leap_timer: 2.0,
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
        assert_eq!(state.leap_timer, before.leap_timer);
    }
}
