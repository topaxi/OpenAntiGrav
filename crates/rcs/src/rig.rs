//! Evaluating a `.rcsskeleton` node under its `.rcsanimclip` track: the
//! local matrix at a given second.
//!
//! One [`NodeMotion`] per node; a model's renderer composes them through
//! the hierarchy the same way it composes Pulse's `Anim Transform`s. The
//! evaluation rules were measured against Wipeout HD's own evaluator on the
//! circuits 2048 re-ships from it (`docs/formats/2048-animation.md`):
//!
//! - time wraps on the **track's** duration, not the clip's;
//! - between two keys a vector blends linearly and a quaternion blends
//!   linearly and renormalises, with the shorter arc taken - slerp gives
//!   the same matrix to three decimals at 0.2 s key spacing, so the
//!   cheaper one is used and the distinction is not claimed;
//! - the segment past the last key blends toward key 0, so a loop closes
//!   without a snap;
//! - visibility holds its key rather than blending, and a hidden node
//!   collapses to the zero matrix so its geometry draws nothing.

use crate::rcsanimclip::{Channel, Track};
use crate::rcsskeleton::{
    Kind, Node, SLOT_PIVOT, SLOT_PIVOT_TRANSLATE, SLOT_ROTATION, SLOT_SCALE, SLOT_TRANSLATION,
    SLOT_VISIBILITY, local_matrix,
};

/// One node with the keys that move it, if any.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeMotion {
    /// The bind pose and the pivots.
    pub node: Node,
    /// The track animating it, or `None` for a node the clip leaves at bind.
    pub track: Option<Track>,
}

impl NodeMotion {
    /// Whether anything about this node changes with time.
    #[must_use]
    pub fn is_animated(&self) -> bool {
        self.track
            .as_ref()
            .is_some_and(|t| t.channels.iter().any(Option::is_some))
    }

    /// Whether the node is visible at `seconds`.
    #[must_use]
    pub fn visible_at(&self, seconds: f32) -> bool {
        match self.channel(SLOT_VISIBILITY) {
            Some((channel, duration)) if channel.kind == Kind::Bool && channel.count > 0 => {
                let (i, _, _) = channel_at(channel, wrap(seconds, duration));
                channel.key(i)[0] != 0.0
            }
            _ => self.node.visible,
        }
    }

    /// The node's local matrix at `seconds`, or all zeros while hidden.
    #[must_use]
    pub fn sample(&self, seconds: f32) -> [f32; 16] {
        if !self.visible_at(seconds) {
            return [0.0; 16];
        }
        let scale = self.vec3_at(SLOT_SCALE, seconds).unwrap_or(self.node.scale);
        let translation = self
            .vec3_at(SLOT_TRANSLATION, seconds)
            .unwrap_or(self.node.translation);
        let pivot = self.vec3_at(SLOT_PIVOT, seconds).unwrap_or(self.node.pivot);
        let pivot_translate = self
            .vec3_at(SLOT_PIVOT_TRANSLATE, seconds)
            .unwrap_or(self.node.pivot_translate);
        let rotation = self.quat_at(seconds).unwrap_or(self.node.rotation);
        local_matrix(scale, rotation, translation, pivot, pivot_translate)
    }

    fn channel(&self, slot: usize) -> Option<(&Channel, f32)> {
        let track = self.track.as_ref()?;
        let channel = track.channels[slot].as_ref()?;
        Some((channel, track.duration))
    }

    fn vec3_at(&self, slot: usize, seconds: f32) -> Option<[f32; 3]> {
        let (channel, duration) = self.channel(slot)?;
        if channel.kind != Kind::Vec3 || channel.count == 0 {
            return None;
        }
        let (i, j, f) = channel_at(channel, wrap(seconds, duration));
        let (a, b) = (channel.key(i), channel.key(j));
        Some(std::array::from_fn(|k| a[k] + (b[k] - a[k]) * f))
    }

    fn quat_at(&self, seconds: f32) -> Option<[f32; 4]> {
        let (channel, duration) = self.channel(SLOT_ROTATION)?;
        if channel.kind != Kind::Quat || channel.count == 0 {
            return None;
        }
        let (i, j, f) = channel_at(channel, wrap(seconds, duration));
        let a = channel.key(i);
        let b = channel.key(j);
        // The shorter arc: `q` and `-q` are one rotation, and blending
        // toward the far one swings through the identity.
        let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
        let sign = if dot < 0.0 { -1.0 } else { 1.0 };
        let mut q: [f32; 4] = std::array::from_fn(|k| a[k] + (sign * b[k] - a[k]) * f);
        let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        if len > 0.0 {
            for v in &mut q {
                *v /= len;
            }
        }
        Some(q)
    }
}

/// `seconds` wrapped onto `[0, duration)`, or `0` for a track with no
/// length.
fn wrap(seconds: f32, duration: f32) -> f32 {
    if duration > 0.0 {
        seconds.rem_euclid(duration)
    } else {
        0.0
    }
}

/// The two key indices straddling wrapped time `t` and the blend between
/// them; the key after the last is key 0.
fn channel_at(channel: &Channel, t: f32) -> (usize, usize, f32) {
    if channel.count == 0 || channel.seconds_per_key <= 0.0 {
        return (0, 0, 0.0);
    }
    let x = t / channel.seconds_per_key;
    let i = (x.floor().max(0.0) as usize) % channel.count;
    let j = (i + 1) % channel.count;
    (i, j, x - x.floor())
}

#[cfg(test)]
mod tests;
