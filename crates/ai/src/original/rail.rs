//! `Ai_SnapToLane` (`0x088542e4`): the rail. Measured firing every tick
//! (confidence 90); its share of the line-holding is open.
//! `docs/ghidra/functions/psp-pulse-usa/ai-controls.md`, "the rail".
//!
//! The original writes the body's velocity and position and never its
//! orientation; Pure's writes the position alone ([`Laws::rail_turns_velocity`]).
//!
//! [`Laws::rail_turns_velocity`]: super::Laws::rail_turns_velocity This returns the write as a [`RailCorrection`] for the
//! caller to apply with [`oag_physics::rail::apply`].

use super::track::{Pose, TrackPoint};
use oag_core::math::Vec3;

/// The body write the rail asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RailCorrection {
    /// The new linear velocity: the old one's speed, turned toward the track;
    /// `None` where the title's rail leaves the velocity alone.
    pub velocity: Option<Vec3>,
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
    /// The signed offset across the track from the lane point on the last
    /// call, `dot(d, lateral)`: read by measurements, never by the rail.
    pub lateral: f32,
}

impl Rail {
    /// One tick. `touching` is the body having any contact (`body+0x370`),
    /// which drops the rail at once for an opponent; `finished` is ship state
    /// `2`, which blends it in at `1.5` a second instead of `0.6`;
    /// `turns_velocity` is [`super::Laws::rail_turns_velocity`].
    #[allow(clippy::too_many_arguments)]
    pub fn snap(
        &mut self,
        pose: &Pose,
        own: &TrackPoint,
        own_lane: f32,
        touching: bool,
        finished: bool,
        turns_velocity: bool,
        dt: f32,
    ) -> Option<RailCorrection> {
        let lane = own.lane_point(own_lane);
        let d = pose.position - lane;
        self.lateral = d.dot(own.lateral);
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
        let velocity = turns_velocity.then(|| {
            let turned = pose.velocity - lateral * (pose.velocity.dot(lateral) * f * 0.25);
            turned.normalize_or_zero() * speed
        });
        Some(RailCorrection {
            velocity,
            position: pose.position - lateral * (d.dot(lateral) * f),
        })
    }
}

#[cfg(test)]
mod tests;
