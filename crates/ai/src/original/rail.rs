//! `Ai_SnapToLane` (`0x088542e4`): the rail. Measured firing every tick
//! (confidence 90); its share of the line-holding is open.
//! `docs/ghidra/functions/psp-pulse-usa/ai-controls.md`, "the rail".
//!
//! The original writes the body's velocity and position and never its
//! orientation. This returns the write as a [`RailCorrection`] for the
//! caller to apply with [`oag_physics::rail::apply`].

use super::track::{Pose, TrackPoint};
use oag_core::math::Vec3;

/// The body write the rail asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RailCorrection {
    /// The new linear velocity: the old one's speed, turned toward the track.
    pub velocity: Vec3,
    /// The new position.
    pub position: Vec3,
}

/// The rail's state on the driver (`+0x48` blend, `+0x4c` time on the rail).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rail {
    /// `+0x48`.
    pub blend: f32,
    /// `+0x4c`.
    pub time: f32,
}

impl Rail {
    /// One tick. `touching` is the body having any contact (`body+0x370`),
    /// which drops the rail at once for an opponent; `finished` is ship state
    /// `2`, which blends it in at `1.5` a second instead of `0.6`.
    pub fn snap(
        &mut self,
        pose: &Pose,
        own: &TrackPoint,
        own_lane: f32,
        touching: bool,
        finished: bool,
        dt: f32,
    ) -> Option<RailCorrection> {
        let lane = own.lane_point(own_lane);
        let d = pose.position - lane;
        let speed = pose.velocity.length();
        let a = ((speed - 50.0) * 0.01).clamp(0.0, 1.0);
        let b = (1.0 - (d.dot(own.down).abs() - 7.0) / 7.0).clamp(0.0, 1.0);
        let w = if touching { 0.0 } else { a * b };
        self.time = if w != 0.0 { self.time + dt } else { 0.0 };
        self.blend = if w < self.blend {
            w
        } else {
            let rate = if finished { 1.5 } else { 0.6 };
            (self.blend + dt * rate).min(w)
        };
        let f = self.blend * self.blend * 0.7;
        if f < 1.0e-6 {
            return None;
        }
        let lateral = own.lateral;
        let turned = pose.velocity - lateral * (pose.velocity.dot(lateral) * f * 0.25);
        Some(RailCorrection {
            velocity: turned.normalize_or_zero() * speed,
            position: pose.position - lateral * (d.dot(lateral) * f),
        })
    }
}

#[cfg(test)]
mod tests;
