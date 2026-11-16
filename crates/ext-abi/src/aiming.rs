//! Where each other craft sits in a craft's own frame: the third per-craft record, after the
//! title key. A mod that decides when to fire a pickup at something (a lock, a cone in front
//! within some range) reads it for the craft holding the pickup.
//!
//! One matrix, `AIM_SLOTS` observers by `AIM_SLOTS` targets, three bytes each: how far the
//! target is ahead of, to the right of and above the observer, along the observer's own
//! forward, right and up axes, in [`AIM_STEP`] units, saturating at the byte's range
//! (a target past `127 * AIM_STEP` units on an axis reads as that). A pair of a craft with
//! itself, or with an empty slot, is zero: check the active mask first. Slots at or past
//! `AIM_SLOTS` have no row and no column.

use crate::{IO_LEN, TITLE_AT};

/// Where the matrix starts: after the title key.
pub const AIM_AT: usize = TITLE_AT + 4;
/// Observers and targets the matrix covers: a grid's slots (the world holds at most this
/// many crafts).
pub const AIM_SLOTS: usize = 8;
/// One pair: forward, right and up as signed bytes.
pub const AIM_LEN: usize = 3;
/// One byte's step, world units. Chosen: a weapon range of about 200 units fits in a byte at
/// half that resolution, and a hull is a few units across.
pub const AIM_STEP: f32 = 2.0;
/// The matrix's whole size.
pub const AIM_TOTAL: usize = AIM_SLOTS * AIM_SLOTS * AIM_LEN;

const _: () = assert!(AIM_AT + AIM_TOTAL <= IO_LEN);

/// Where one target sits in one observer's frame, quantised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Aim {
    /// Along the observer's forward axis, in [`AIM_STEP`]s: positive is ahead.
    pub forward: i8,
    /// Along its right axis: positive is to the right.
    pub right: i8,
    /// Along its up axis: positive is above.
    pub up: i8,
}

impl Aim {
    /// The pair from an offset in the observer's frame, in world units.
    #[must_use]
    pub fn from_offset(forward: f32, right: f32, up: f32) -> Self {
        // Rounded half away from zero by hand: `core` has no `round`.
        let q = |v: f32| {
            let steps = v / AIM_STEP;
            if steps.is_nan() {
                return 0;
            }
            let steps = steps.clamp(-127.0, 127.0);
            (if steps >= 0.0 {
                steps + 0.5
            } else {
                steps - 0.5
            }) as i8
        };
        Self {
            forward: q(forward),
            right: q(right),
            up: q(up),
        }
    }

    /// Units ahead of the observer; negative behind.
    #[must_use]
    pub fn ahead(self) -> f32 {
        f32::from(self.forward) * AIM_STEP
    }

    /// Units to the observer's right; negative to its left.
    #[must_use]
    pub fn across(self) -> f32 {
        f32::from(self.right) * AIM_STEP
    }

    /// Units above the observer; negative below.
    #[must_use]
    pub fn above(self) -> f32 {
        f32::from(self.up) * AIM_STEP
    }

    /// Inside a cone in front of the observer: ahead by more than nothing and at most
    /// `range` units, and no further off the forward axis (sideways and up taken
    /// together) than `spread` times the distance ahead. `spread` is the tangent of the
    /// cone's half-angle: 1.0 is 45 degrees, 0.2 about 11.
    #[must_use]
    pub fn in_cone(self, range: f32, spread: f32) -> bool {
        let (ahead, across, above) = (self.ahead(), self.across(), self.above());
        ahead > 0.0
            && ahead <= range
            && across * across + above * above <= spread * spread * ahead * ahead
    }

    /// The three bytes.
    #[must_use]
    pub fn encode(self) -> [u8; AIM_LEN] {
        [
            self.forward.to_le_bytes()[0],
            self.right.to_le_bytes()[0],
            self.up.to_le_bytes()[0],
        ]
    }

    /// From three bytes.
    #[must_use]
    pub fn decode(bytes: &[u8; AIM_LEN]) -> Self {
        Self {
            forward: i8::from_le_bytes([bytes[0]]),
            right: i8::from_le_bytes([bytes[1]]),
            up: i8::from_le_bytes([bytes[2]]),
        }
    }
}

/// Every observer's view of every target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aiming([Aim; AIM_SLOTS * AIM_SLOTS]);

impl Default for Aiming {
    fn default() -> Self {
        Self([Aim::default(); AIM_SLOTS * AIM_SLOTS])
    }
}

impl Aiming {
    /// Sets where `target` sits for `observer`; slots past [`AIM_SLOTS`] are ignored.
    pub fn set(&mut self, observer: usize, target: usize, aim: Aim) {
        if observer < AIM_SLOTS && target < AIM_SLOTS {
            self.0[observer * AIM_SLOTS + target] = aim;
        }
    }

    /// Where `target` sits for `observer`; zero past [`AIM_SLOTS`].
    #[must_use]
    pub fn get(&self, observer: usize, target: usize) -> Aim {
        if observer < AIM_SLOTS && target < AIM_SLOTS {
            self.0[observer * AIM_SLOTS + target]
        } else {
            Aim::default()
        }
    }

    /// Writes the matrix into a whole buffer.
    pub fn write(&self, io: &mut [u8; IO_LEN]) {
        for (i, aim) in self.0.iter().enumerate() {
            let at = AIM_AT + i * AIM_LEN;
            io[at..at + AIM_LEN].copy_from_slice(&aim.encode());
        }
    }
}

/// The pair's bytes in the section a guest reads from [`crate::CRAFTS_AT`] on, for
/// `observer` looking at `target`.
#[must_use]
pub fn read(crafts: &[u8], observer: usize, target: usize) -> Aim {
    if observer >= AIM_SLOTS || target >= AIM_SLOTS {
        return Aim::default();
    }
    let at = AIM_AT - crate::CRAFTS_AT + (observer * AIM_SLOTS + target) * AIM_LEN;
    match crafts.get(at..at + AIM_LEN) {
        Some(&[f, r, u]) => Aim::decode(&[f, r, u]),
        _ => Aim::default(),
    }
}
