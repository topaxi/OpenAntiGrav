//! Where a craft is on the course and where it runs in the field: the second per-craft
//! record, after the [`crate::Craft`] records, and the course length in the header.
//!
//! A mod that drives its own AI decision reads it: the craft's distance along the lap,
//! its place, and the length of the lap to turn the distance into a share of it.

use crate::{CRAFT_LEN, CRAFTS_AT, MAX_CRAFTS, get_u32, put_u32};

/// Where the course length is written: `f32` units, 0 when the race has no course.
pub const COURSE_LENGTH_AT: usize = 4;
/// Where the tracking records start, after the craft records.
pub const TRACKING_AT: usize = CRAFTS_AT + CRAFT_LEN * MAX_CRAFTS;
/// One tracking record.
pub const TRACKING_LEN: usize = 8;
/// Within a tracking record: the distance along the lap, `f32` units, 0 without a fix.
pub const TRACKING_PROGRESS: usize = 0;
/// Within a tracking record: the place, 1-based, 0 when the slot holds no craft.
pub const TRACKING_PLACE: usize = 4;
/// Within a tracking record: the flags byte.
pub const TRACKING_FLAGS: usize = 5;
/// A flag: the craft has a course fix, so `progress` means something. Distance is
/// negative on the grid behind the line, so no number can stand for "no fix".
pub const TRACKING_FIXED: u8 = 1;

/// Where the title key is written, after the tracking records: [`crate::channel_key`] of the
/// title's lowercased hyphenated name (`wipeout-hd`), 0 when the race has none.
pub const TITLE_AT: usize = TRACKING_AT + TRACKING_LEN * MAX_CRAFTS;

const _: () = assert!(TITLE_AT + 4 <= crate::IO_LEN);

/// What a tick tells a guest about the course: its length, each slot's [`Tracking`]
/// in slot order, and the title raced.
#[derive(Debug, Clone, Copy, Default)]
pub struct Course<'a> {
    /// One lap's length, units; 0 when the race has no course.
    pub length: f32,
    /// Each grid slot, in slot order.
    pub tracking: &'a [Tracking],
    /// The title's key (`channel_key` of its slug); 0 when unknown.
    pub title: u32,
    /// Where each craft sits in each other's frame; zero when the host does not say.
    pub aiming: Option<&'a crate::Aiming>,
}

/// One slot's place on the course.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Tracking {
    /// Distance along the circuit, `None` before the craft's first fix.
    pub progress: Option<f32>,
    /// Place in the field, 1-based; 0 for an empty slot.
    pub place: u8,
}

impl Tracking {
    /// The record.
    #[must_use]
    pub fn encode(&self) -> [u8; TRACKING_LEN] {
        let mut record = [0; TRACKING_LEN];
        put_u32(
            &mut record,
            TRACKING_PROGRESS,
            self.progress.unwrap_or(0.0).to_bits(),
        );
        record[TRACKING_PLACE] = self.place;
        record[TRACKING_FLAGS] = if self.progress.is_some() {
            TRACKING_FIXED
        } else {
            0
        };
        record
    }

    /// The tracking a record describes.
    #[must_use]
    pub fn decode(record: &[u8; TRACKING_LEN]) -> Self {
        let fixed = record[TRACKING_FLAGS] & TRACKING_FIXED != 0;
        Self {
            progress: fixed.then(|| f32::from_bits(get_u32(record, TRACKING_PROGRESS))),
            place: record[TRACKING_PLACE],
        }
    }
}
