//! What the screens that only look at a race read off the devices: the right stick, the
//! analogue triggers and a pad's own left stick, none of which reach
//! [`oag_gameplay::InputSnapshot`].
//!
//! The replay viewer and photo mode are view-side (ADR-0088, "The binding plan"). A trigger
//! there is an analogue read taken **before** [`crate::pad::resolve`] turns a pull into a button,
//! whatever the trigger mode says, and the right stick has no meaning to a craft at all. The
//! snapshot a race is ticked from is built exactly as before, so a recording and its hashes
//! cannot move.

use crate::pad::{TriggerConfig, condition, deadzone, larger};

/// A conditioned trigger at or past this is a full pull: in photo mode it starts playback.
pub const PLAY_PULL: f32 = 0.9;

/// What the devices said this tick, before any deadzone or curve.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ViewRaw {
    /// Right stick, right and up positive.
    pub right: (f32, f32),
    /// A pad's own left stick, apart from the arrow keys and the d-pad the snapshot's stick merges
    /// it with.
    pub left: (f32, f32),
    /// L2, 0 to 1.
    pub l2: f32,
    /// R2, 0 to 1.
    pub r2: f32,
    /// A real pad (or an injected one) is speaking, so `left` is the left stick and the arrow
    /// keys are not.
    pub pad: bool,
}

impl ViewRaw {
    /// Two devices as one: each axis the larger, a trigger the deeper pull.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            right: (
                larger(self.right.0, other.right.0),
                larger(self.right.1, other.right.1),
            ),
            left: (
                larger(self.left.0, other.left.0),
                larger(self.left.1, other.left.1),
            ),
            l2: self.l2.max(other.l2),
            r2: self.r2.max(other.r2),
            pad: self.pad || other.pad,
        }
    }

    /// The reading the screens act on: sticks deadzoned, triggers conditioned with the pilot's
    /// own curve, whatever [`crate::pad::TriggerMode`] is live.
    #[must_use]
    pub fn read(self, config: TriggerConfig) -> ViewReading {
        ViewReading {
            right: (deadzone(self.right.0), deadzone(self.right.1)),
            left: (deadzone(self.left.0), deadzone(self.left.1)),
            l2: condition(self.l2, config.curve()),
            r2: condition(self.r2, config.curve()),
            pad: self.pad,
        }
    }
}

/// [`ViewRaw`] after the deadzones and the trigger curve.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ViewReading {
    /// Right stick.
    pub right: (f32, f32),
    /// The pad's left stick.
    pub left: (f32, f32),
    /// L2, 0 to 1.
    pub l2: f32,
    /// R2, 0 to 1.
    pub r2: f32,
    /// See [`ViewRaw::pad`].
    pub pad: bool,
}

#[cfg(test)]
mod tests;
