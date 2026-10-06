//! The four control states, and the ramps that drive them.
//!
//! The original keeps throttle, brake, steering and the two airbrake sides as floats in adjacent
//! craft slots (`craft+0x2b8` to `+0x2c8`) and ramps them at parameter rates. Three of the five
//! ramps are live; the throttle's is not ([`crate::params::Engine::falloff`]). Evidence:
//! `docs/ghidra/functions/psp-pulse-usa/engine.md`, "Engine", "Brakes" and "Steering".

use crate::params::Handling;
use crate::ship::{ShipControls, ShipState};

/// The scale the original's control states use.
///
/// [`ShipControls`] is normalised (`-1..=1` axes, `0..=1` triggers; the contract `oag-gameplay`
/// owns), but **the original's states are percentages**, and three pieces of arithmetic only
/// come out right on `0..=100`:
///
/// - The brake ramp clamps at `100.0`.
/// - The engine compares `throttle > 100.0` to detect an over-unity throttle.
/// - The lateral grip coefficient `max(L, R) * (0.01 - slidegrip) - 1.0` reaches exactly `0` at
///   full airbrake with `slidegrip = 0`, needing `max(L, R)` to reach `100`.
///
/// So the conversion happens here, once, where a normalised input becomes a state.
///
/// **The steering axis is measured on this scale**: both captures in `data/traces/` show
/// `craft+0x2c0` ramping at 498/s and falling at 924/s under full deflection, reproducing the
/// shipped `Turning.gain` and `Turning.falloff` to under 1 % only on the `0..=100` reading.
/// `stick_y` is untested (the craft control block holds no second axis to capture).
///
/// **Do not "fix" the yaw authority by dividing it by `CONTROL_RANGE`.** An older version of
/// this comment invited exactly that and is falsified: the axis scale is right,
/// `HandlingXml_ParseTurning` stores `amount` verbatim (`swc1 f0,0xd0(a0)`), and the yaw error
/// is **22x, not 100x**. Dividing by 100 lands 4.7x too *weak* (RMS error 1.05 rad/s against a
/// signal of 1.5). The real discrepancy and its recovered constant are at
/// [`crate::forces::YAW_INVERSE_INERTIA`].
pub const CONTROL_RANGE: f32 = 100.0;

/// The brake state's ceiling, and so every control state's nominal maximum.
pub const CONTROL_MAX: f32 = 100.0;

/// Ramps one control toward a non-negative target at `gain` up and `falloff` down.
///
/// Both rates are per-second and the result never overshoots. A per-second *exponential* decay
/// would also fit "per-second rate"; the linear reading is a **guess awaiting M3**, chosen
/// because it cannot overshoot and so cannot introduce an oscillation mistaken for a physics
/// bug.
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
/// Unlike [`ramp`], `gain` is the rate while moving toward a **larger-magnitude** target and
/// `falloff` while returning toward centre, on both sides of zero, and the centre is reached
/// exactly. The same parameters give a fast bite with a slow return or the reverse, intended
/// and not a sign artefact. From `Ship_UpdateSteering` (`0x08848788`), confidence 84.
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

/// Advances every control state by one frame, before any force is evaluated, as the original
/// does: engine, brakes, airbrakes and steering each ramp their own state at the top of their
/// own update, and all five precede every consumer.
pub fn update(state: &mut ShipState, controls: &ShipControls, handling: &Handling, dt: f32) {
    // The throttle is **not** ramped: the original computes a ramp, stores it and overwrites it
    // with the raw input before reading it back, so a ramp would add throttle lag the game lacks.
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
    update_camera_lean(state, controls.steer_x, dt);
}

/// The follower's per-second gain, the `5.4` literal at `0x0883fac0`.
pub const CAMERA_LEAN_FOLLOW_GAIN: f32 = 5.4;
/// The most the follower may be behind its target, in lean units, while the stick
/// pushes it further out (`0x3f19999a`).
pub const CAMERA_LEAN_OUT_LIMIT: f32 = 0.6;
/// The same bound while the stick is centred or opposes the move (`0x3e99999a`).
pub const CAMERA_LEAN_RETURN_LIMIT: f32 = 0.3;
/// The first-order filter's rate between follower and lean, per second (`4.0`).
pub const CAMERA_LEAN_SMOOTH_RATE: f32 = 4.0;

/// Advances the steering lean the cockpit camera rolls by, one frame.
///
/// `FUN_0883fab4` (`craft+0x848`, `craft+0x844`), arithmetic read at instruction level,
/// confidence 80. The input is the **raw** stick on the `+/-100` record `PlayerInput_Update`
/// writes, not the ramped [`ShipState::steer`]; `stick_x` is that stick on `-1..=1`. The
/// follower's `delta` is clamped, not the lean, which saturates at `+/-1` where the stick does.
///
/// **Chosen, not measured: omitted.** The original also folds in `+/-10.0` from two flags
/// (`craft+0x8a4`, `craft+0x8a8`) and negates on `craft+0x860 & 2`; none is identified, so they
/// are treated as clear.
pub fn update_camera_lean(state: &mut ShipState, stick_x: f32, dt: f32) {
    let target = stick_x * CONTROL_RANGE * 0.01;
    let delta = target - state.camera_lean_follower;
    let limit = if (delta > 0.0 && stick_x > 0.0) || (delta <= 0.0 && stick_x < 0.0) {
        CAMERA_LEAN_OUT_LIMIT
    } else {
        CAMERA_LEAN_RETURN_LIMIT
    };
    let delta = delta.clamp(-limit, limit);
    state.camera_lean_follower += delta * CAMERA_LEAN_FOLLOW_GAIN * dt;
    state.camera_lean +=
        (state.camera_lean_follower - state.camera_lean) * dt * CAMERA_LEAN_SMOOTH_RATE;
}

#[cfg(test)]
mod tests;
