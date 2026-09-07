//! The magstrip hold's per-tick locator: a track's spline, resampled into the
//! `oag_formats::track::Sample` table `oag_trace::replay`'s `replay`/`drive`/
//! `drive_with` and `oag_trace::plan::to_gate` all take now.
//!
//! Its own file rather than another `main.rs` function: that file already sits
//! at its frozen `scripts/check-file-size.py` ceiling, and this is a new
//! feature rather than a fix to something already there. See Task #33.

use anyhow::Result;
use oag_formats::track;
use oag_race::Course;

use crate::{load_ai, resample};

/// A track's spline, resampled the same way `resample` always is - dropping
/// which path each sample came from, which the locator has no use for.
///
/// `None` reproduces the tool's pre-Task-#33 behaviour exactly: no disc to
/// read a `.vex` from means an empty table, which makes every locator lookup
/// come back `None` and the magstrip hold's blend stay pinned at zero.
pub(crate) fn load_samples(source: Option<&str>, name: &str) -> Result<Vec<track::Sample>> {
    let Some(source) = source else {
        return Ok(Vec::new());
    };
    Ok(resample(&load_ai(source, name)?, Course::STEPS_PER_SEGMENT)
        .into_iter()
        .map(|(_, sample)| sample)
        .collect())
}
