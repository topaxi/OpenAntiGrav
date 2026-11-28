//! Session-level Tournament bookkeeping: folding a finished leg into its run's running
//! standings (`RunContext::tournament`), and continuing to the next one. The points law and the
//! leg list themselves are `oag_raceplay::tournament::Progress`'s; this module only drives it
//! against the rest of the session. A tournament ends with its run (ADR-0099).

use log::warn;

use crate::app::race_stage::RaceStage;

use super::Session;

/// Folds the leg that just finished into its run's running standings, and - only once every
/// leg has run - carries the resulting standings rank into `stage.tournament_final_rank` for
/// `RaceStage::campaign_medal` to read.
///
/// A no-op outside a Tournament cell (the run has no `tournament`) and before the leg has a
/// board to read. Idempotent by construction: `Progress::record_leg` overwrites rather than
/// accumulates, so a caller that lands here twice for the same leg scores it once either way.
pub(crate) fn record_finished_leg(stage: &mut RaceStage) {
    let Some(board) = stage.race.core.results() else {
        return;
    };
    let Some(progress) = stage.run.tournament.as_mut() else {
        return;
    };
    progress.record_leg(board);
    // Slot 0 is always the player - see `oag_gameplay::World::race`'s own
    // "per slot, not per human" doc.
    stage.tournament_final_rank = progress.is_last_leg().then(|| progress.rank(0));
}

impl Session {
    /// Whether `EndRace Menu` should offer `ER_NEXT_RACE` in place of
    /// `RACE AGAIN` - a tournament with a leg still to race.
    #[must_use]
    pub(crate) fn tournament_has_next_leg(&self) -> bool {
        self.current_run()
            .and_then(|run| run.tournament.as_ref())
            .is_some_and(|progress| !progress.is_last_leg())
    }

    /// `ER_NEXT_RACE`: advances the finished leg's run to its next leg and
    /// relaunches - `Tournament_AdvanceLeg`'s own effect, reproduced through
    /// this project's existing relaunch machinery rather than a parallel
    /// path of its own, the same choice `MenuOption::RaceAgain` already
    /// makes for its own redirect.
    pub(crate) fn advance_tournament_leg(&mut self) {
        // The finished leg's run again, as `RaceAgain` relaunches it (ADR-0099).
        self.rerun_race_on_screen();
        let Some(run) = self.run.as_mut() else {
            return;
        };
        let Some(progress) = run.tournament.as_mut() else {
            warn!("ER_NEXT_RACE pressed with no tournament in progress - ignoring");
            return;
        };
        if !progress.advance() {
            warn!("ER_NEXT_RACE pressed on a tournament's own last leg - ignoring");
            return;
        }
        let Some(track) = progress.current_track().map(str::to_string) else {
            warn!("the next leg names no track - cannot continue the tournament");
            return;
        };
        let Some(options) = run.options.as_mut() else {
            return;
        };
        options.track = Some(track);
        self.finish_launch();
    }
}
