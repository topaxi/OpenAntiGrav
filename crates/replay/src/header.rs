//! [`Header`]: what a replay was recorded against, in text.
//!
//! **TOML rather than packed binary**, because the header is the part a person
//! reads when a replay will not play - "which build recorded this, on which
//! track, at which class" - and the part most likely to grow a field. A
//! `#[serde(default)]` field is how it grows without a format bump, the same
//! rule `records.toml` follows. The bulk of a file is the input stream and the
//! pose track, which are binary; the header is a few hundred bytes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Everything needed to know what a replay is of, and to rebuild its race.
///
/// The first five fields are the **key** a composition root files a replay
/// under, spelled the way that composition root spells its records key - this
/// crate compares them as strings and never interprets them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    /// The title the run was raced on, as the composition root names it.
    pub title: String,
    /// The circuit, as the composition root names it (Pulse's is the `.vex`
    /// entry path).
    pub track: String,
    /// The race mode.
    pub mode: String,
    /// The speed class.
    pub class: String,
    /// The team whose craft was flown, which is also the hull a ghost of the
    /// run is drawn as.
    pub team: String,
    /// The world generator's seed.
    pub seed: u64,
    /// Simulation ticks per second. Every tick index in the file is at this
    /// rate.
    pub tick_rate: u32,
    /// The build that recorded the run, informational only.
    ///
    /// **Never compared to decide whether a replay is still valid.** A commit
    /// hash changes on a docs-only commit and a hand-bumped version number is
    /// forgotten; the stored state hashes answer the actual question - "does
    /// this build still reproduce this run" - and [`crate::Verifier`] asks it.
    /// See ADR-0055, "when the simulation changes".
    #[serde(default)]
    pub build: String,
    /// Ticks between two stored state hashes.
    pub hash_interval: u32,
    /// The grid slots whose inputs are recorded: the human ones.
    ///
    /// Every other slot is flown by the AI, which reproduces itself from the
    /// seed and needs nothing stored.
    pub slots: Vec<u8>,
    /// Whatever else the composition root needs to rebuild the same race: the
    /// control scheme, the AI difficulty, a lap-count override, the operator's
    /// autopilot. Opaque to this crate, ordered so the text is stable.
    #[serde(default)]
    pub options: BTreeMap<String, String>,
    /// The lap this file's ghost is, if it carries one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ghost: Option<GhostInfo>,
}

/// Where a file's ghost lap sits in the recorded run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhostInfo {
    /// Which lap it was, 1-based.
    pub lap: u32,
    /// The world tick its lap clock started on: the tick the craft crossed
    /// the line to begin it.
    pub start_tick: u64,
    /// Its time, in ticks - the same unit and the same edge the lap timer and
    /// the records file use.
    pub lap_ticks: u32,
}

impl Header {
    /// A header with no options, no ghost, and a hash every second.
    #[must_use]
    pub fn new(
        title: impl Into<String>,
        track: impl Into<String>,
        mode: impl Into<String>,
        class: impl Into<String>,
        team: impl Into<String>,
        seed: u64,
    ) -> Self {
        Self {
            title: title.into(),
            track: track.into(),
            mode: mode.into(),
            class: class.into(),
            team: team.into(),
            seed,
            tick_rate: 60,
            build: String::new(),
            hash_interval: DEFAULT_HASH_INTERVAL,
            slots: vec![0],
            options: BTreeMap::new(),
            ghost: None,
        }
    }
}

/// One stored hash a second at 60 Hz.
///
/// **A second is the resolution a desync is reported at**, and that is fine: the
/// report's job is to say *that* a run diverged and roughly where, and a
/// debugging session then bisects inside the second with the per-tick hash the
/// determinism tests already use. Eight bytes a second is 480 bytes a minute,
/// against an input stream that is usually larger.
pub const DEFAULT_HASH_INTERVAL: u32 = 60;
