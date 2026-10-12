//! `Ai_ComputeControls` (`0x08855dc4`)'s steering half and `Ai_ComputePitch`
//! (`0x08853148`). Formulas: `docs/ghidra/functions/psp-pulse-usa/ai-controls.md`,
//! read at instruction level (85), the `0.7/0.3` cross-track split and the yaw
//! prediction from the decompile (75). The steering has no live leg.

use super::track::{Pose, TrackPoint};
use oag_core::math::{Vec3, sin_cos};

/// `AiStats_SetDefaults`' header (`RM+0x2c4..`): executable constants.
const STEER_GAIN: f32 = 950.0;
const AIRBRAKE_GAIN: f32 = 80.0;
const AIRBRAKE_DEAD: f32 = 10.0;

/// One class's controller, the disc's `AIControlStats.xml` with the
/// executable's per-class defaults (`AiStats_SetDefaults`) where a file is silent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Controller {
    /// `SteerMul`, default `1`.
    pub steer_mul: f32,
    /// `SteerDamp`, default `0`.
    pub steer_damp: f32,
    /// `LookAheadSecs`, default `0.5`.
    pub look_ahead_secs: f32,
    /// `xtrackMul`, default `0`.
    pub xtrack_mul: f32,
}

impl Controller {
    /// The disc's row over the executable's defaults. `xtrackMax` and
    /// `xtrackDamp` have no reader in the original and are not carried.
    #[must_use]
    pub fn from_disc(row: &oag_tables::ai_control_stats::Controller) -> Self {
        Self {
            steer_mul: row.steer_mul.unwrap_or(1.0),
            steer_damp: row.steer_damp.unwrap_or(0.0),
            look_ahead_secs: row.look_ahead_secs.unwrap_or(0.5),
            xtrack_mul: row.xtrack_mul.unwrap_or(0.0),
        }
    }
}

/// Steer and the two airbrakes on the original's `0..=100` scale, before the
/// authority multiplies them.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Steering {
    /// Positive to the right.
    pub steer: f32,
    /// Left airbrake.
    pub left: f32,
    /// Right airbrake.
    pub right: f32,
}

/// How far ahead `Ai_SampleLookahead` samples: `max(speed * t, 15)` units,
/// at `1.5` times that in progress (`0x088530a0`). Returned as the distance
/// the caller walks along the track.
#[must_use]
pub fn lookahead_distance(smoothed_speed: f32, seconds: f32) -> f32 {
    let d = smoothed_speed * seconds;
    let d = if d < 15.0 { 15.0 } else { d };
    d * 1.5
}

/// The height term: how far the craft is airborne over its own sample,
/// `clamp((|h| - 15) * 0.025, 0, 1)`.
#[must_use]
pub fn airborne(pose: &Pose, own: &TrackPoint) -> f32 {
    let h = (pose.position - own.position).dot(pose.up);
    ((h.abs() - 15.0) * 0.025).clamp(0.0, 1.0)
}

/// What the steering reads besides the pose.
#[derive(Debug, Clone, Copy)]
pub struct SteerInput<'a> {
    /// The craft's own sample.
    pub own: &'a TrackPoint,
    /// The planned lane at the own sample's progress.
    pub own_lane: f32,
    /// The lookahead sample.
    pub ahead: &'a TrackPoint,
    /// The planned lane at the lookahead's progress.
    pub ahead_lane: f32,
    /// [`airborne`].
    pub air: f32,
}

/// The steering controller. `last_error` is `driver+0x174[0]`.
///
/// `dt * 60` is exactly `1` at the fixed 60 Hz tick, so the derivative is the
/// difference itself.
pub fn steer(
    pose: &Pose,
    input: &SteerInput<'_>,
    controller: &Controller,
    last_error: &mut f32,
) -> Steering {
    let (own, ahead) = (input.own, input.ahead);
    let x = own
        .lateral
        .dot(pose.position - own.lane_point_from_centre(input.own_lane))
        * (1.0 - input.air);
    let target = ahead.lane_point(input.ahead_lane);
    let dist = (target - pose.position).length();
    let c = controller.xtrack_mul * x;
    let shift = c * dist;
    let target = target - ahead.lateral * (shift * 0.7) + ahead.tangent * (shift.abs() * 0.7)
        - own.lateral * (shift * 0.3)
        + own.tangent * (shift.abs() * 0.3);
    let dir = (target - (pose.position + pose.velocity * 0.0676)).normalize_or_zero();
    let yaw = pose.angular_velocity.dot(pose.up) * -0.0206;
    let (sin, cos) = sin_cos(yaw);
    let f = pose.forward;
    let predicted =
        Vec3::new(f.x * cos + f.z * sin, f.y, f.z * cos - f.x * sin).normalize_or_zero();
    let error = -predicted.cross(dir).dot(pose.up);
    let derivative = error - *last_error;
    *last_error = error;
    let raw = (error + derivative * controller.steer_damp) * STEER_GAIN * controller.steer_mul;
    let mut out = Steering {
        steer: raw,
        ..Steering::default()
    };
    if raw > 100.0 {
        out.right = ((raw - 100.0) * AIRBRAKE_GAIN - AIRBRAKE_DEAD).clamp(0.0, 100.0);
        out.steer = 100.0;
    } else if raw < -100.0 {
        out.left = ((-raw - 100.0) * AIRBRAKE_GAIN - AIRBRAKE_DEAD).clamp(0.0, 100.0);
        out.steer = -100.0;
    }
    out
}

/// `Ai_ComputePitch`: toward the track tangent while the craft's ground
/// fraction is at most `0.9`, else `0`. `last_forward` (`driver+0x190`) is
/// only refreshed on the airborne branch, as the original does.
pub fn pitch(pose: &Pose, own: &TrackPoint, grounded: f32, last_forward: &mut Vec3) -> f32 {
    if grounded > 0.9 {
        return 0.0;
    }
    let rate = pose.forward.cross(*last_forward).dot(pose.left);
    *last_forward = pose.forward;
    let error = pose.forward.cross(own.tangent).dot(pose.left);
    ((error + rate * 40.0) * -2500.0).clamp(-100.0, 100.0)
}

#[cfg(test)]
mod tests;
