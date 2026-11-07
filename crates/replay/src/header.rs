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
    /// The launch request the race was built from, resolved: what a build needs to rebuild the
    /// race without the menu. `None` in a file written before the table existed, and in a file
    /// whose writer had no request to record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup: Option<Setup>,
    /// Set when this file is a fork of another replay (ADR-0087, decision 9). Read, never yet
    /// written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork: Option<Fork>,
}

/// The schema kind of a replay header's `toml-schema` line.
pub const SCHEMA_KIND: &str = "replay-header";
/// The one major of the schema this build reads and writes.
pub const SCHEMA_MAJOR: u32 = 1;

/// The resolved launch request of a race: every input to building it that is not the disc, the
/// seed or an [`oag_gameplay::InputSnapshot`].
///
/// Names and tokens, never a path: the source is whatever the reader has of the header's title.
/// **The tick-zero hash is the check that this is enough** (ADR-0087, decision 7): a rebuild that
/// lacks something disagrees at tick 0 and is refused by name. Plain data so the header stays
/// text a person can read; the crate that builds races (`oag-raceplay`) owns the mapping to and
/// from its own options.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Setup {
    /// The race mode's token (`time_trial`, `single_race`, ...).
    pub mode: String,
    /// The speed class, spelled the way the disc spells it.
    pub class: String,
    /// The circuit's archive entry, or none for the title's own default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<String>,
    /// The player's team id, or none for the title's own default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    /// The player's alternate hull file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hull_variant: Option<String>,
    /// The player's `PI_ModelSkin`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skin: Option<String>,
    /// The team ids the opponents may fly, in the caller's order.
    pub opponent_teams: Vec<String>,
    /// The grid is filled although the mode would not.
    pub opponents: bool,
    /// The class has no controller, so the race fields no opponents.
    pub no_driver_for_class: bool,
    /// How many people fly the race, across every window (0 is one per view).
    pub people: u8,
    /// How many split-screen views the race is drawn in.
    pub views: u8,
    /// Laps the race ends after, over the mode's own table.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub laps_override: Option<u32>,
    /// The weapons switch, over the mode's own answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weapons_override: Option<bool>,
    /// The Eliminator kill count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eliminator_kill_target: Option<u32>,
    /// The Eliminator score target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eliminator_score_target: Option<u32>,
    /// The `Campaign=` value of a downloadable campaign's event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_on_campaign: Option<String>,
    /// A 2048 Zone race draws its own track model.
    pub zone_model: bool,
}

/// What marks a replay as a fork of another (ADR-0087, decision 9): the tick the live pad first
/// flew, and which recording it came from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fork {
    /// The first tick the live pad flew.
    pub tick: u64,
    /// The tick the player picked TAKE OVER on, which the hold of at most a second follows.
    pub offered_at: u64,
    /// The slots a person took.
    pub seats: Vec<u8>,
    /// The parent's [`Header::build`], informational.
    pub parent_build: String,
    /// The parent's stored hash nearest before the tick, as `0x` and sixteen hex digits.
    pub parent_hash: String,
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
            setup: None,
            fork: None,
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
