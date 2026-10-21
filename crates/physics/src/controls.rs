//! The four control states, and the ramps that drive them.
//!
//! The original keeps throttle, brake, steering and the two airbrake sides as floats in adjacent
//! craft slots (`craft+0x2b8` to `+0x2c8`) and ramps them at parameter rates. Three of the five
//! ramps are live; the throttle's is not ([`crate::params::Engine::falloff`]). Evidence:
//! `docs/ghidra/functions/psp-pulse-usa/engine.md`, "Engine", "Brakes" and "Steering".

use crate::params::{CameraClamp, Handling};
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

/// [`ramp_steering`] as Wipeout HD's `Craft_UpdateSteering` (`0x000edb50`) runs it: the same two
/// rates, but each step stops at the target, `min(cur + gain * dt, target)` toward a larger
/// magnitude (`0x000edc50`-`0x000edc68`) and `max(cur - falloff * dt, target)` back
/// (`0x000edb9c`-`0x000edba4`, an `fsel`), mirrored below zero.
///
/// Pulse's ramp (`0x08848788`) has no clamp, so a held stick overshoots by `f32` rounding
/// (twelve `8.333334` steps make `100.00002`), falls by `falloff * dt` the next frame and cycles
/// `83.3 / 91.7 / 100`: a full lock averages `91.7`. HD's parks at `100`, read live at
/// `craft+0x314` on RPCS3. See `docs/ghidra/functions/ps3-hdfury-eu/craft-inertia.md`.
#[must_use]
pub fn ramp_steering_clamped(current: f32, target: f32, gain: f32, falloff: f32, dt: f32) -> f32 {
    if target > 0.0 {
        if target > current {
            (current + gain * dt).min(target)
        } else {
            (current - falloff * dt).max(target)
        }
    } else if target < 0.0 {
        if target < current {
            (current - gain * dt).max(target)
        } else {
            (current + falloff * dt).min(target)
        }
    } else {
        ramp_steering(current, target, gain, falloff, dt)
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

    let steer_ramp = if state.steer_ramp_clamped {
        ramp_steering_clamped
    } else {
        ramp_steering
    };
    state.steer = steer_ramp(
        state.steer,
        controls.steer_x * CONTROL_RANGE,
        handling.turning.gain,
        handling.turning.falloff,
        dt,
    );
    update_camera_lean(state, controls.steer_x, &handling.camera_lean, dt);
    update_steer_lean(state, controls, &handling.steer_lean, dt);
}

/// The most the follower may be behind its target, in lean units, while the stick
/// pushes it further out (`0x3f19999a`).
pub const CAMERA_LEAN_OUT_LIMIT: f32 = 0.6;
/// The same bound while the stick is centred or opposes the move (`0x3e99999a`).
pub const CAMERA_LEAN_RETURN_LIMIT: f32 = 0.3;

/// Advances the steering lean the cockpit camera rolls by, one frame.
///
/// `FUN_0883fab4` (`craft+0x848`, `craft+0x844`), arithmetic read at instruction level,
/// confidence 80. The input is the **raw** stick on the `+/-100` record `PlayerInput_Update`
/// writes, not the ramped [`ShipState::steer`]; `stick_x` is that stick on `-1..=1`. The
/// follower's `delta` is clamped, not the lean, which saturates at `+/-1` where the stick does.
///
/// `lean` carries the gains a title may change ([`crate::params::CameraLean`]): Wipeout HD's
/// `FUN_000eadb8` (`0x000eb528`-`0x000eb5d0`) runs this filter with the stick term `0.015 *
/// <Tilt internalAmount>`, the follower at `<Tilt internalSpeed>`, the smooth at `6.0`, and
/// `+/-0.5` from the two sideshift timers ([`ShipState::sideshift_timers`]).
///
/// **Chosen, not measured: omitted on Pulse.** The original also folds in `+/-10.0` from two
/// flags (`craft+0x8a4`, `craft+0x8a8`) and negates on `craft+0x860 & 2`; none is identified,
/// so they are treated as clear.
pub fn update_camera_lean(
    state: &mut ShipState,
    stick_x: f32,
    lean: &crate::params::CameraLean,
    dt: f32,
) {
    let mut target = stick_x * CONTROL_RANGE * lean.stick_scale;
    if state.sideshift_timers[0] > 0.0 {
        target -= lean.sideshift_offset;
    }
    if state.sideshift_timers[1] > 0.0 {
        target += lean.sideshift_offset;
    }
    let delta = target - state.camera_lean_follower;
    let delta = match lean.clamp {
        CameraClamp::Pulse => {
            let limit = if (delta > 0.0 && stick_x > 0.0) || (delta <= 0.0 && stick_x < 0.0) {
                CAMERA_LEAN_OUT_LIMIT
            } else {
                CAMERA_LEAN_RETURN_LIMIT
            };
            delta.clamp(-limit, limit)
        }
        CameraClamp::Hd => {
            if delta > 0.0 && stick_x > 0.0 {
                if delta > CAMERA_LEAN_OUT_LIMIT {
                    CAMERA_LEAN_OUT_LIMIT
                } else {
                    delta.min(CAMERA_LEAN_RETURN_LIMIT)
                }
            } else if delta <= 0.0 && stick_x < 0.0 {
                if delta < -CAMERA_LEAN_OUT_LIMIT {
                    -CAMERA_LEAN_OUT_LIMIT
                } else {
                    delta.max(-CAMERA_LEAN_RETURN_LIMIT)
                }
            } else {
                delta
            }
        }
    };
    state.camera_lean_follower += delta * lean.follower_rate * dt;
    state.camera_lean += (state.camera_lean_follower - state.camera_lean) * dt * lean.smooth_rate;
}

/// The most the steer lean follower may trail its target while the stick pushes the same way
/// (`0.5`, `0x0883fd08`).
pub const STEER_LEAN_OUT_LIMIT: f32 = 0.5;
/// The same bound otherwise, airbrakes alone included (`0x3e99999a`, `0x0883fd28`).
pub const STEER_LEAN_RETURN_LIMIT: f32 = 0.3;
/// What one percent of airbrake adds to the steer lean's target (`0x3ba3d70a`): a full right
/// airbrake leans like half a right stick, a full left one like half a left stick.
pub const STEER_LEAN_AIRBRAKE: f32 = 0.005;

/// Advances the lean the ship's model is drawn with, one frame.
///
/// `FUN_0883fab4`'s second filter (`entity+0x858` follower, `entity+0x854` lean), read at
/// instruction level and measured: replaying it over a PPSSPP capture of the stick, each
/// airbrake alone, the stick with either airbrake and both airbrakes
/// (`data/traces/talons-junction-steer-lean-airbrakes.csv`) reproduces `+0x854` to `8e-7`.
/// Confidence 95. The target is `stick * 0.01 - left * 0.005 + right * 0.005` on the `+/-100`
/// control record; the follower's step is clamped to `0.5` when the **raw stick** points the
/// way it moves and to `0.3` otherwise, so an airbrake alone always takes the slower limit.
/// The display matrix turns the hull by `lean * 0.5` radians about its nose (`FUN_088418e0`,
/// `0x08842140`); the cockpit camera does not read it.
///
/// While the craft is out of the race (state 5, `entity+0x860 & 0x1000`) the target is zero,
/// so the lean eases back level through both rates rather than snapping.
///
/// **Chosen, not measured: omitted.** The stick term is negated on `entity+0x860 & 2`, a bit
/// with no identified writer; it is treated as clear, as it read throughout the capture.
///
/// `lean` carries the three gains a title may change ([`crate::params::SteerLean`]): Wipeout
/// HD's `FUN_000eadb8` (`0x000eb5d4`-`0x000eb650`) runs this same filter with the stick term
/// scaled by `<Tilt shipAmount>`, the follower at `<Tilt shipSpeed>` and the smooth at `10.0`,
/// measured on RPCS3 (`crates/trace/tests/steer_lean_ground_truth.rs`).
pub fn update_steer_lean(
    state: &mut ShipState,
    controls: &ShipControls,
    lean: &crate::params::SteerLean,
    dt: f32,
) {
    let stick = controls.steer_x * CONTROL_RANGE;
    let target = if state.craft_state == crate::damage::CraftState::Eliminated {
        0.0
    } else {
        stick * 0.01 * lean.stick_amount
            - controls.airbrake_left * CONTROL_RANGE * STEER_LEAN_AIRBRAKE
            + controls.airbrake_right * CONTROL_RANGE * STEER_LEAN_AIRBRAKE
    };
    let delta = target - state.steer_lean_follower;
    let limit = if (delta > 0.0 && stick > 0.0) || (delta <= 0.0 && stick < 0.0) {
        STEER_LEAN_OUT_LIMIT
    } else {
        STEER_LEAN_RETURN_LIMIT
    };
    let delta = delta.clamp(-limit, limit);
    state.steer_lean_follower += delta * dt * lean.follower_rate;
    state.steer_lean += (state.steer_lean_follower - state.steer_lean) * lean.smooth_rate * dt;
}

#[cfg(test)]
mod tests;
