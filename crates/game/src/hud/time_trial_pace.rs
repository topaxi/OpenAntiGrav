//! [`super::Readout::time_trial_pace`]: the campaign Time Trial/Speed Lap
//! "pacing" indicator - `Hud_UpdateTimeCluster`'s tier/target/redden
//! triple (`*(hud+0x3c)+0x34`/`+0x30`/`+0x38`), for the gold/silver/bronze
//! branch of the campaign case alone. Split out of `hud.rs` for
//! `scripts/check-file-size.py`'s 1,000-line ratchet, the same reason
//! [`super::lap_splits`] has its own file.
//!
//! **What this reproduces, and the one thing it does not.**
//! `PlayerStatus_Update`'s own decompile
//! (`docs/ghidra/functions/psp-pulse-usa/race-progress.md`'s "The
//! target-time readout" section) picks the tier in two ways. A campaign cell
//! races its own gold/silver/bronze ladder, and shows `RECORD` instead when
//! the player's stored best already beats gold and the run is ahead of it. A
//! plain Time Trial or Speed Lap has no ladder, so it is `RECORD` throughout,
//! counting down to the smaller of the player's stored best and the track's
//! authored `<RaceTimes>` (Time Trial) or `<LapTimes>` (Speed Lap). Both come
//! from [`RecordTarget`].
//!
//! **The stored best is this build's own, keyed by circuit, mode and class -
//! chosen, not measured.** The original keys its record store by team
//! (`FUN_088091a0`, `param_1+0x460`), and this project's
//! [`crate::records::Key`] has no team, so a run flown in one team's ship
//! races the best of any team's. The authored figure is unaffected.

use oag_tables::race_campaign::Cell;
use oag_tables::track_stats::TrackStats;

/// The tier `Hud_UpdateTimeCluster` reads (`*(hud+0x3c)+0x34`): `0` to `3`.
///
/// Its own type rather than [`oag_tables::race_campaign::Medal`], which
/// [`crate::records`] persists as a campaign award and which has no
/// `Record`: a pace is a caption, not something earned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaceTier {
    /// Tier `0`, `IG_HUD_BRONZE`.
    Bronze,
    /// Tier `1`, `IG_HUD_SILVER`.
    Silver,
    /// Tier `2`, `IG_HUD_GOLD`.
    Gold,
    /// Tier `3`, `IG_HUD_RECORD`.
    Record,
}

/// The time a plain (or personal-best-beating) run races: what
/// `PlayerStatus_Update` calls the ghost.
///
/// Both figures are centiseconds, the unit the original compares in. See the
/// module doc for what is measured and what is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RecordTarget {
    /// The stored best for this circuit, mode and class - `FUN_088091a0`'s
    /// answer. `None` until a race here has set one.
    pub personal_best_centis: Option<i64>,
    /// The track's own `stats.xml` figure for this class: `<RaceTimes>` for a
    /// Time Trial, `<LapTimes>` for a Speed Lap. Truncated, as the
    /// original's `(uint)(float * 100.0)` is. `None` where the file did not
    /// read: a campaign cell still races its ladder without it, a plain race
    /// has nothing to race.
    pub authored_centis: Option<i64>,
}

impl RecordTarget {
    /// The target a `mode` race on `class` chases, or `None` for a mode the
    /// clock cluster is not shown in. `best` is the [`crate::records::Record`]
    /// this race saves to; `stats` is `None` where the track's `stats.xml`
    /// did not read, or the class is not one of the four.
    #[must_use]
    pub fn new(
        mode: oag_race::Mode,
        class: &str,
        stats: Option<&TrackStats>,
        best: Option<&crate::records::Record>,
    ) -> Option<Self> {
        let class = oag_tables::handling::SpeedClass::from_name(class);
        let authored = |figures: fn(&TrackStats) -> [f32; 4]| {
            let (stats, class) = (stats?, class?);
            #[allow(clippy::cast_possible_truncation)]
            Some((figures(stats)[class as usize] * 100.0) as i64)
        };
        let (authored, best_ticks) = match mode {
            oag_race::Mode::TimeTrial => (
                authored(|stats| stats.race_times),
                best.and_then(|record| record.best_total_ticks),
            ),
            oag_race::Mode::SpeedLap => (
                authored(|stats| stats.lap_times),
                best.and_then(|record| record.best_lap_ticks.map(u64::from)),
            ),
            _ => return None,
        };
        Some(Self {
            personal_best_centis: best_ticks.map(ticks_to_centis),
            authored_centis: authored,
        })
    }
}

fn ticks_to_centis(ticks: u64) -> i64 {
    i64::try_from(ticks * 100 / 60).unwrap_or(i64::MAX)
}

/// [`super::Readout::time_trial_pace`]'s own value: which tier the current
/// pace is in, and how far off it the clock reads right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeTrialPace {
    /// The tier this pace is in this tick.
    pub tier: PaceTier,
    /// Ticks left to beat [`Self::tier`]'s own target, or `0` once
    /// [`Self::missed`].
    pub remaining_ticks: u32,
    /// Whether the clock has already passed the target - `Hud_UpdateTimeCluster_q`'s
    /// literal `0xffff0000` (pure red) tint, applied to `TotalTime` alone.
    pub missed: bool,
}

impl TimeTrialPace {
    /// `Hud_UpdateTimeCluster`'s tier (`0x0881c9d0`), fed by
    /// `PlayerStatus_Update`'s own target-time block (`0x0883b3b8`),
    /// collapsed to a pure function of the elapsed tick count. `ladder` is a
    /// campaign cell's own gold/silver/bronze, `None` for a plain race.
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
    /// **`elapsed_ticks` inherits `TotalTime`/`CurrentTime`'s own clock-start
    /// conventions, and the two are not the same convention.** `race_ticks`
    /// (Time Trial) is [`oag_race::race_clock_ticks`]: zero through the
    /// start-line countdown and counting from the release, which is when the
    /// original's `craft+0x920` starts (measured 2026-09-29: it reads `0.0`
    /// through the countdown and takes its first `dt` on the tick thrust is
    /// released - see `docs/gameplay/race-modes.md#the-race-clock-starts-at-the-release`).
    /// Until that change this was `World::tick` from the grid, which charged
    /// every medal comparison the whole 4.5 s countdown. It is still not
    /// reset at the first crossing, the same *reset convention*
    /// `race-progress.md`'s reading of `craft+0x920` describes ("is only
    /// reset by a *lap* completion"). **Not the same *value*, though**: this
    /// build's own grid spawn sits further back from the line than the
    /// original's (`crates/race/src/state.rs`'s own `advance_progress`
    /// comment), so `race_ticks` on lap 1 carries that extra run-up -
    /// unmeasured how much.
    ///
    /// `Readout::lap_ticks` (Speed Lap) diverges further still: `RaceState`
    /// deliberately resets `lap_start_tick` at the *first* line crossing,
    /// not at the standing start, to compensate for the same spawn
    /// difference ("Start lap 1's clock here rather than at the standing
    /// start... the original does *not* restart its lap clock at the first
    /// crossing"). So a Speed Lap pace here starts its own clock at the
    /// first crossing while the original's `craft+0x920` keeps running
    /// from the standing start through it - lap 1 only (every lap after
    /// resets on completion in both), magnitude unmeasured, and possibly
    /// larger than a Speed Lap cell's own gold/silver gap (`grid0_2_2`
    /// authors `4000`/`4200`/`4500`, a 2 s gold-to-silver window) - large
    /// enough to flip the tier on a lap-1 Speed Lap reading. A divergence
    /// this function inherits rather than introduces, not yet measured
    /// closely enough to correct for.
    #[must_use]
    pub fn evaluate(elapsed_ticks: u64, ladder: Option<&Cell>, target: &RecordTarget) -> Self {
        let elapsed = ticks_to_centis(elapsed_ticks);
        // The original's own `0xffffffff` and `0` both mean "no stored best".
        let best = target.personal_best_centis.filter(|&best| best > 0);
        let (tier, target_centis) = match ladder {
            // No ladder: always `RECORD`, chasing the smaller of the stored
            // best and the authored figure.
            None => (
                PaceTier::Record,
                match (best, target.authored_centis) {
                    (Some(best), Some(authored)) => best.min(authored),
                    (best, authored) => best.or(authored).unwrap_or(0),
                },
            ),
            // A stored best already faster than gold, and still ahead of.
            Some(cell) if best.is_some_and(|b| b < cell.gold && elapsed < b) => {
                (PaceTier::Record, best.unwrap_or(0))
            }
            Some(cell) if elapsed <= cell.gold => (PaceTier::Gold, cell.gold),
            Some(cell) if elapsed <= cell.silver => (PaceTier::Silver, cell.silver),
            Some(cell) => (PaceTier::Bronze, cell.bronze),
        };
        let missed = elapsed > target_centis;
        let remaining_ticks = if missed {
            0
        } else {
            u32::try_from((target_centis - elapsed).max(0) * 60 / 100).unwrap_or(u32::MAX)
        };
        Self {
            tier,
            remaining_ticks,
            missed,
        }
    }
}

/// [`super::Readout::time_trial_pace`] for a race in `mode`, or `None` when
/// the clock keeps showing the plain elapsed time.
///
/// Without a campaign ladder the readout races the authored figure, and where
/// the track's `stats.xml` did not load there is none: `PlayerStatus_Update`
/// fills the target block only under `DAT_08b310b4 != 0`, the loaded track
/// record, so a plain race keeps the plain clock. A campaign cell races its
/// ladder either way. Only Time Trial counts the whole race; Speed Lap reads
/// the current lap.
#[must_use]
pub fn pace_for(
    mode: oag_race::Mode,
    race_ticks: u64,
    lap_ticks: u64,
    ladder: Option<&Cell>,
    target: Option<&RecordTarget>,
) -> Option<TimeTrialPace> {
    let elapsed = match mode {
        oag_race::Mode::TimeTrial => race_ticks,
        oag_race::Mode::SpeedLap => lap_ticks,
        _ => return None,
    };
    let default = RecordTarget::default();
    let target = target.unwrap_or(&default);
    if ladder.is_none() && target.authored_centis.is_none() {
        return None;
    }
    Some(TimeTrialPace::evaluate(elapsed, ladder, target))
}

/// [`super::Readout::time_trial_pace`]'s own caption -
/// `Hud_UpdateTimeCluster`'s `IG_HUD_BRONZE`/`SILVER`/`GOLD`/`RECORD` arms of
/// its five-way table. The layout's own default `TOTAL` has no tier to map
/// from ("not a Time Trial/Speed Lap race"), which stays `None` upstream. See
/// `docs/ghidra/functions/psp-pulse-usa/hud-time-caption-substitution.md`.
pub(super) fn tier_caption(tier: PaceTier, strings: &oag_ui::language::StringTable) -> String {
    let id = match tier {
        PaceTier::Gold => "IG_HUD_GOLD",
        PaceTier::Silver => "IG_HUD_SILVER",
        PaceTier::Bronze => "IG_HUD_BRONZE",
        PaceTier::Record => "IG_HUD_RECORD",
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

/// Whether this title's HUD update hides `TotalTime` and `TotalTimeTxt` for
/// the readout's mode - [`oag_title::HudArt::total_time_timed_modes_only`].
///
/// **Pulse hides them in every mode but Time Trial and Speed Lap.**
/// `PlayerStatus_Update` (`0x0883b3b8`) writes the clock's target field to
/// `-1` each tick and overwrites it only for `g_game_mode` 5 (Time Trial),
/// `0x11` (Multiplayer Time Trial), 10 (Speed Lap) and 7 (Free Play);
/// `Hud_UpdateTimeCluster` clears both widgets' visible bit while it reads
/// `-1`. Free Play and the multiplayer modes are not modes this build races,
/// so the two it does are the whole set. Confidence **95**: the decompile,
/// plus a PPSSPP frame of an Eliminator race (`g_game_mode` 8) with no `TOTAL`
/// on screen and both widgets' flag words read live with the bit clear - see
/// `docs/ghidra/functions/psp-pulse-usa/hud-time-caption-substitution.md`.
///
/// This is a fact about the mode, not the layout, which is why it is
/// separate from `place_owns_the_anchor`: a single race with no place yet
/// (`Readout::place` zero) hides the clock too, and Zone's layout authors no
/// `TotalTime` at all so it never reaches this.
pub(super) fn mode_hides_total_time(art: &oag_title::HudArt, readout: &super::Readout) -> bool {
    art.total_time_timed_modes_only
        && !matches!(
            readout.mode,
            oag_race::Mode::TimeTrial | oag_race::Mode::SpeedLap
        )
}

#[cfg(test)]
mod tests;
