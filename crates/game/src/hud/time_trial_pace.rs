//! [`super::Readout::time_trial_pace`]: the campaign Time Trial/Speed Lap
//! "pacing" indicator - `Hud_UpdateTimeCluster`'s tier/target/redden
//! triple (`*(hud+0x3c)+0x34`/`+0x30`/`+0x38`), for the gold/silver/bronze
//! branch of the campaign case alone. Split out of `hud.rs` for
//! `scripts/check-file-size.py`'s 1,000-line ratchet, the same reason
//! [`super::lap_splits`] has its own file.
//!
//! **Deliberately incomplete, and the gap is not hidden.**
//! `PlayerStatus_Update`'s own decompile
//! (`docs/ghidra/functions/psp-pulse-usa/race-progress.md`'s "The
//! target-time readout" section) has a second `RECORD` path even on a
//! campaign cell: if the player's own stored personal best for this track
//! (`FUN_088091a0`, keyed by team) beats the cell's own gold target *and*
//! the live pace is currently beating that personal best too, the original
//! shows `RECORD` instead of `GOLD`. `FUN_088091a0`'s own record format is
//! unread - this project has nothing to feed that branch - so
//! [`TimeTrialPace::from_elapsed`] never produces [`Medal::Gold`]'s
//! `RECORD` upgrade. A campaign cell run on a personal best already faster
//! than gold shows `GOLD` here where the original would show `RECORD`;
//! everything else the original's own ladder does for a campaign cell is
//! reproduced.

use oag_tables::race_campaign::Medal;

/// [`super::Readout::time_trial_pace`]'s own value: which medal the current
/// pace is chasing, and how far off it the clock reads right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeTrialPace {
    /// The medal this pace would earn if the race ended this tick.
    pub medal: Medal,
    /// Ticks left to beat [`Self::medal`]'s own target, or `0` once
    /// [`Self::missed`].
    pub remaining_ticks: u32,
    /// Whether the clock has already passed the cell's own bronze target -
    /// `Hud_UpdateTimeCluster_q`'s literal `0xffff0000` (pure red) tint,
    /// applied to `TotalTime` alone.
    pub missed: bool,
}

impl TimeTrialPace {
    /// `Hud_UpdateTimeCluster`'s tier ladder (`0x0881c9d0`), fed by
    /// `PlayerStatus_Update`'s own campaign branch (`0x0883b3b8`),
    /// collapsed to a pure function of the elapsed tick count - the
    /// gold/silver/bronze ladder only; see this module's own doc for the
    /// `RECORD`-via-personal-best branch this deliberately omits.
    ///
    /// **The original's own tier field is stateful**: written once a tick,
    /// and left untouched once the elapsed value has passed every target,
    /// so the tier keeps showing whichever medal was last in reach rather
    /// than resetting to "none". That statefulness is provably redundant
    /// here. `PlayerStatus_Update`'s own elapsed value only ever grows
    /// across a race - a race clock never runs backwards - and every
    /// branch that freshly assigns the tier does so from a comparison
    /// against that same, monotonically growing value: gold's branch is
    /// reached exactly when `elapsed <= gold`, silver's exactly when
    /// `gold < elapsed <= silver`, bronze's exactly when
    /// `silver < elapsed <= bronze`, and the one branch that leaves the
    /// tier untouched is reached exactly when `elapsed > bronze` - at
    /// which point the last real assignment before that tick was
    /// necessarily bronze, since elapsed cannot have been smaller a tick
    /// ago than it is now. So "the tier last assigned before elapsed
    /// passed bronze" and "bronze, evaluated fresh against the current
    /// elapsed value" name the same tick for a monotonic clock, and
    /// re-deriving the tier here from `elapsed_ticks` alone avoids a
    /// second piece of per-race state to keep in sync with `Race`'s own
    /// tick count, at no cost in fidelity. The original's own redden bit
    /// carries the same proof: every freshly-assigned branch computes it
    /// as the *negation* of the condition that reached that branch, so it
    /// is always `false` on a fresh assignment and `true` only in the
    /// untouched branch, i.e. exactly `elapsed > bronze`. See
    /// `docs/ghidra/functions/psp-pulse-usa/hud-time-caption-substitution.md`'s
    /// "What the tier itself is, closed 2026-09-28" section for the full
    /// decompile this argument is checked against.
    ///
    /// **`elapsed_ticks` inherits `TotalTime`/`CurrentTime`'s own existing
    /// clock-start conventions, not newly-checked ones, and the two are not
    /// the same convention.** `Race_ticks` (Time Trial) is `World::tick`,
    /// counting from the standing start (`tick == 0`), before the green
    /// flag - matching `race-progress.md`'s own reading of the original's
    /// `craft+0x920`, which "is only reset by a *lap* completion" and so
    /// "is timed from whenever `Race_UpdatePositions` starts stepping
    /// crafts", i.e. the same standing start, not the green flag either.
    /// `Readout::lap_ticks` (Speed Lap) is different: `RaceState`
    /// deliberately resets `lap_start_tick` at the *first* line crossing,
    /// not at the standing start, to compensate for this build's own grid
    /// spawn sitting further back from the line than the original's
    /// (`crates/race/src/state.rs`'s own `advance_progress`, "Start lap 1's
    /// clock here rather than at the standing start... the original does
    /// *not* restart its lap clock at the first crossing"). So a Speed Lap
    /// pace here starts its own clock at the first crossing while the
    /// original's `craft+0x920` keeps running from the standing start
    /// through it - a real, if usually small (`Course::START_LINE_ADVANCE`
    /// at Speed Lap velocity, a few tenths of a second on most circuits),
    /// divergence this function inherits rather than introduces. Whether
    /// the original's own clock is also offset from the *countdown's* own
    /// start (before `RaceState::thrust_gated` releases thrust) is a
    /// separate, still-open question neither `race_ticks` nor `lap_ticks`
    /// answers - see
    /// `docs/ghidra/functions/psp-pulse-usa/race-progress.md`'s own "The
    /// first crossing starts the race but not the clock" bullet, open since
    /// before this pass.
    #[must_use]
    pub fn from_elapsed(elapsed_ticks: u64, cell: &oag_tables::race_campaign::Cell) -> Self {
        let elapsed_centis = i64::try_from(elapsed_ticks * 100 / 60).unwrap_or(i64::MAX);
        let (medal, target_centis) = if elapsed_centis <= cell.gold {
            (Medal::Gold, cell.gold)
        } else if elapsed_centis <= cell.silver {
            (Medal::Silver, cell.silver)
        } else {
            (Medal::Bronze, cell.bronze)
        };
        let missed = elapsed_centis > cell.bronze;
        let remaining_ticks = if missed {
            0
        } else {
            u32::try_from((target_centis - elapsed_centis).max(0) * 60 / 100).unwrap_or(u32::MAX)
        };
        Self {
            medal,
            remaining_ticks,
            missed,
        }
    }
}

/// [`super::Readout::time_trial_pace`]'s own caption -
/// `Hud_UpdateTimeCluster`'s `IG_HUD_GOLD`/`SILVER`/`BRONZE` arm of its
/// five-way table. `RECORD` **is** reachable on a campaign cell too (see
/// this module's own top-level doc) but is not produced by
/// [`TimeTrialPace::from_elapsed`], so there is nothing here to map it
/// from; the layout's own default `TOTAL` likewise has no [`Medal`] value
/// to represent "no pace" or "not a Time Trial/Speed Lap cell" - both stay
/// `None` upstream instead. See
/// `docs/ghidra/functions/psp-pulse-usa/hud-time-caption-substitution.md`.
pub(super) fn medal_caption(medal: Medal, strings: &oag_ui::language::StringTable) -> String {
    let id = match medal {
        Medal::Gold => "IG_HUD_GOLD",
        Medal::Silver => "IG_HUD_SILVER",
        Medal::Bronze => "IG_HUD_BRONZE",
    };
    strings.get_or_id(id).to_string()
}

/// `TotalTime`'s own colour override - `Hud_UpdateTimeCluster`'s literal
/// `0xffff0000` (pure red) when [`super::Readout::time_trial_pace`] reads
/// [`TimeTrialPace::missed`]. Never `TotalTimeTxt`: the original's own
/// `FUN_088cd290` recolour call is issued once, against the numeric widget
/// alone - the caption keeps its layout-authored colour throughout.
pub(super) fn time_trial_colour(readout: &super::Readout, name: &str) -> Option<[f32; 4]> {
    let pace = readout.time_trial_pace?;
    (name == "TotalTime" && pace.missed).then(|| oag_ui::screen::argb_to_rgba(0xFFFF_0000))
}

#[cfg(test)]
mod tests;
