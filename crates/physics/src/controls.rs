//! The four control states, and the ramps that drive them.
//!
//! The original keeps throttle, brake, steering and the two airbrake sides as
//! floats in adjacent craft slots (`craft+0x2b8` through `+0x2c8`) and ramps them
//! at rates from the parameter set. Three of the five ramps are live. The
//! throttle's is not: see [`crate::params::Engine::falloff`].
//!
//! Evidence: `docs/ghidra/functions/psp-pulse-usa/engine.md`, sections "Engine",
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
/// error 1.05 rad/s against captures whose signal is 1.5. The real discrepancy
/// and the recovered constant that closes it are documented at
/// [`crate::forces::YAW_INVERSE_INERTIA`] - which is recovered rather than a
/// stand-in, the fitted `YAW_DRIVE_CALIBRATION` it replaced having been retired
/// when its writer was disassembled.
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
mod tests;
