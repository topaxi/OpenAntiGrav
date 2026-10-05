//! Session-level Tournament bookkeeping: folding a finished leg into
//! [`Session::tournament`]'s own running standings, continuing to the next
//! one, and discarding it once its `EndRace Menu` is left. The points law
//! and the leg list themselves are `oag_raceplay::tournament::Progress`'s -
//! this module only drives it against the rest of the session, the same
//! split `crate::main::session::campaign`/`crate::main::session::endrace`
//! already make for the screens either owns.

use log::warn;

use crate::race_stage::RaceStage;
use crate::stage::Stage;
use oag_raceplay::tournament::Progress;

use super::Session;

/// Folds the leg that just finished into `tournament`'s own running
/// standings, and - only once every leg has run - carries the resulting
/// standings rank into `stage.tournament_final_rank` for
/// `RaceStage::campaign_medal` to read.
///
/// **A plain function, not a `Session` method**, because its one caller
/// (`Session::frame`'s finish-transition guard) already holds `stage` as a
/// `&mut` borrow of `self.stage` at the point this needs to run - a method
/// taking `&mut self` there would conflict with that live borrow even
/// though the two touch disjoint fields, since the compiler cannot see
/// through the method call to know `Session::record_tournament_leg` never
/// touches `self.stage` itself. Two plain, disjoint references sidestep
/// that entirely.
///
/// A no-op outside a Tournament cell (`tournament` is `None`) and before
/// the leg has a board to read. Idempotent by construction:
/// `Progress::record_leg` overwrites rather than accumulates, so a caller
/// that lands here twice for the same leg - the finish transition this
/// runs from is itself guarded by `RaceStage::result_saved`, but nothing
/// here relies on that guard holding - scores it once either way.
pub(crate) fn record_finished_leg(tournament: &mut Option<Progress>, stage: &mut RaceStage) {
    let Some(board) = stage.race.results() else {
        return;
    };
    let Some(progress) = tournament.as_mut() else {
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
        self.tournament
            .as_ref()
            .is_some_and(|progress| !progress.is_last_leg())
    }

    /// `ER_NEXT_RACE`: advances `self.tournament` to its next leg and
    /// relaunches - `Tournament_AdvanceLeg`'s own effect, reproduced through
    /// this project's existing relaunch machinery rather than a parallel
    /// path of its own, the same choice `MenuOption::RaceAgain` already
    /// makes for its own redirect.
    pub(crate) fn advance_tournament_leg(&mut self) {
        // The same re-arm `RaceAgain` needs, for the same reason:
        // `Session::launch_campaign_cell`'s own drain already cleared
        // `self.campaign_cell` into the *first* leg's `RaceStage`, so a
        // relaunch with nothing set here would lose the medal evaluation
        // and `EndRace Rewards`' own `campaign` flag on the next leg.
        (self.campaign_cell, self.campaign_difficulty) = match &self.stage {
            Stage::Race(stage) => (stage.campaign_cell.clone(), stage.campaign_difficulty),
            _ => (None, None),
        };
        let Some(progress) = self.tournament.as_mut() else {
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
        let Some(options) = self.race_options.as_mut() else {
            return;
        };
        options.track = Some(track);
        self.finish_launch();
    }

    /// Discards `self.tournament` - called wherever an EndRace `Return`
    /// option leaves the flow, whether the tournament finished or was
    /// abandoned mid-way. See [`Session::tournament`]'s own doc for why
    /// there is nothing to save.
    pub(crate) fn abandon_tournament(&mut self) {
        self.tournament = None;
    }
}
