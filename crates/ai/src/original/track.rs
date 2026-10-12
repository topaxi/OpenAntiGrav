//! What the original driver sees of the track and of its own craft.
//!
//! A [`TrackPoint`] is the `SplinePt`-shaped record the original evaluates
//! (`docs/ghidra/functions/psp-pulse-usa/ai-controls.md`, "Track samples"):
//! position, tangent, down, lateral, the AI corridor and the racing line, and
//! the normalised lap progress. The caller builds it from the disc's spline.

use oag_core::math::Vec3;

/// One evaluated track sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackPoint {
    /// The point as the running game holds it: the disc's position lifted
    /// off the surface (`AiTrack_LoadPathPoints`'s `pos -= 3 * down`).
    pub position: Vec3,
    /// Unit vector along the path.
    pub tangent: Vec3,
    /// Unit vector into the surface.
    pub down: Vec3,
    /// Unit vector across the path, to the driver's right.
    pub lateral: Vec3,
    /// `ai_bound_left`, a lateral offset.
    pub bound_left: f32,
    /// `ai_bound_right`, a lateral offset.
    pub bound_right: f32,
    /// `racing_line`, a lateral offset.
    pub racing_line: f32,
    /// The lap progress, `0.0..1.0`.
    pub progress: f32,
}

impl TrackPoint {
    /// `span(l)`: the corridor's half on lane `l`'s side of the racing line.
    fn span(&self, lane: f32) -> f32 {
        if lane < 0.0 {
            self.racing_line - self.bound_left
        } else {
            self.bound_right - self.racing_line
        }
    }

    /// `Ai_LanePoint` (`0x08852dac`): lane `l`'s world point, racing line included.
    #[must_use]
    pub fn lane_point(&self, lane: f32) -> Vec3 {
        self.position + self.lateral * (self.racing_line + self.span(lane) * lane)
    }

    /// `Ai_LanePointFromCentre` (`0x08852364`): the same without the
    /// racing-line term, as `Ai_ComputeControls` measures its cross-track error.
    #[must_use]
    pub fn lane_point_from_centre(&self, lane: f32) -> Vec3 {
        self.position + self.lateral * (self.span(lane) * lane)
    }

    /// `FUN_08852c98`/`FUN_08852c08`: the lane coordinate of `position`
    /// across this sample's corridor, unclamped; `0` where a corridor half is
    /// narrower than `1e-4`.
    #[must_use]
    pub fn lane_of(&self, position: Vec3) -> f32 {
        let e = self.lateral.dot(position - self.position);
        let rl = self.racing_line;
        if rl <= e {
            let span = self.bound_right - rl;
            if span.abs() < 1.0e-4 {
                0.0
            } else {
                (e - rl) / span
            }
        } else {
            let span = self.bound_left - rl;
            if span.abs() < 1.0e-4 {
                0.0
            } else {
                -((e - rl) / span)
            }
        }
    }
}

/// The craft as `AiManager_SnapshotRacers` copies it into the racer record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// Body position.
    pub position: Vec3,
    /// The body's own up axis.
    pub up: Vec3,
    /// The body's own forward axis.
    pub forward: Vec3,
    /// The body's own left axis (`racer+0x40`): this engine's `-right`.
    pub left: Vec3,
    /// Linear velocity.
    pub velocity: Vec3,
    /// Angular velocity, radians a second.
    pub angular_velocity: Vec3,
}
