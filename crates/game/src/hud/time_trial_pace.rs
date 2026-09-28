//! [`super::Readout::time_trial_pace`]: the campaign Time Trial/Speed Lap
//! "pacing" indicator - `Hud_UpdateTimeCluster_q`'s tier/target/redden
//! triple (`*(hud+0x3c)+0x34`/`+0x30`/`+0x38`), for the campaign branch
//! alone. Split out of `hud.rs` for `scripts/check-file-size.py`'s 1,000-line
//! ratchet, the same reason [`super::lap_splits`] has its own file.

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
    /// `Hud_UpdateTimeCluster_q`'s tier ladder (`0x0881c9d0`), fed by
    /// `PlayerStatus_Update`'s own campaign branch (`0x0883b3b8`),
    /// collapsed to a pure function of the elapsed tick count.
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
    /// "The live tier is a pure function of elapsed time" section for the
    /// full decompile this argument is checked against.
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
/// `Hud_UpdateTimeCluster_q`'s `IG_HUD_GOLD`/`SILVER`/`BRONZE` arm of its
/// five-way table. `RECORD` and the layout's own default `TOTAL` are the
/// other two branches of that table and are not reachable from a campaign
/// cell's gold/silver/bronze targets, so [`Medal`] has no case for either -
/// see
/// `docs/ghidra/functions/psp-pulse-usa/hud-time-caption-substitution.md`.
pub(super) fn medal_caption(medal: Medal, strings: &oag_ui::language::StringTable) -> String {
    let id = match medal {
        Medal::Gold => "IG_HUD_GOLD",
        Medal::Silver => "IG_HUD_SILVER",
        Medal::Bronze => "IG_HUD_BRONZE",
    };
    strings.get_or_id(id).to_string()
}

/// `TotalTime`'s own colour override - `Hud_UpdateTimeCluster_q`'s literal
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
