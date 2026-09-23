//! Carries a Tournament cell's own leg list and running standings across a
//! leg boundary - a genuinely new shape, not an extension of an existing
//! field: every other mode resets fresh per race because every one of them
//! ends there, and Tournament is the first that does not. The law it carries
//! is `docs/ghidra/functions/psp-pulse-usa/tournament.md`'s.
//!
//! The points law itself is [`oag_race::tournament`]'s; this module is only
//! the game-layer bookkeeping that owns it across the several separate
//! [`crate::race::Race`] instances one tournament's legs actually are - see
//! `docs/gameplay/race-modes.md#tournament`.

use oag_gameplay::MAX_SHIPS;

use crate::scoreboard::Board;

/// One tournament's own progress: which leg is current, every leg's own
/// track (already resolved to an archive entry name, exactly as
/// `race::Options::track` wants it), and the running points standings.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    /// Archive entry names, one per leg, in the order
    /// `oag_tables::race_campaign::Cell::tournament_tracks` authors them (or
    /// `TournamentSelection_CommitSelection`'s own twelve-slot picker, for a
    /// Custom Race `Tournament C` launch, once one exists to read from).
    legs: Vec<String>,
    /// The 0-based leg currently racing, or just finished.
    current_leg: usize,
    standings: oag_race::tournament::Standings<{ MAX_SHIPS }>,
}

impl Progress {
    /// A fresh tournament on its first leg, with nobody having scored yet.
    ///
    /// **Zeroing every craft's total here is chosen, not measured.**
    /// `tournament.md`'s own "what is not determined" section names exactly
    /// this gap: `DAT_08b34320`'s one write site (`FUN_08820d78`) was found
    /// but never decompiled, so whether the original truly zeroes a fresh
    /// tournament's own running total (rather than, say, carrying one over
    /// from an unrelated prior race) is still open. Starting empty is the
    /// only reading that does not require guessing at unread code.
    #[must_use]
    pub fn new(legs: Vec<String>) -> Self {
        Self {
            legs,
            current_leg: 0,
            standings: oag_race::tournament::Standings::new(),
        }
    }

    /// The track this leg races, or `None` on a malformed (empty) leg list -
    /// never observed, but `Session::launch_campaign_cell` checks before
    /// this is ever constructed rather than relying on this alone.
    #[must_use]
    pub fn current_track(&self) -> Option<&str> {
        self.legs.get(self.current_leg).map(String::as_str)
    }

    /// Whether the leg currently racing is the last one - the exact guard
    /// `Tournament_AdvanceLeg` reads (`DAT_08b30fa4 < DAT_08b30fa0 - 1`) to
    /// decide whether `EndRace Menu` offers `ER_NEXT_RACE` at all.
    #[must_use]
    pub fn is_last_leg(&self) -> bool {
        self.current_leg + 1 >= self.legs.len()
    }

    /// Moves to the next leg. Does nothing and answers `false` on the last
    /// leg - the same guard [`Self::is_last_leg`] reports.
    pub fn advance(&mut self) -> bool {
        if self.is_last_leg() {
            return false;
        }
        self.current_leg += 1;
        true
    }

    /// Folds the leg that just finished into the running standings -
    /// **idempotent**, so a caller that lands here twice for the same leg
    /// (the finish transition can re-enter a tick) overwrites rather than
    /// double-counts. See [`oag_race::tournament::Standings::record_leg`].
    ///
    /// A row's own slot past [`MAX_SHIPS`] is silently dropped rather than
    /// panicking - never happens, `Board` is built off `World::ship_count`
    /// which cannot exceed it, but this stays defensive rather than
    /// asserting a fact this module has no way to check.
    pub fn record_leg(&mut self, board: &Board) {
        let mut places = [0u8; MAX_SHIPS];
        let mut finished = [false; MAX_SHIPS];
        for row in &board.rows {
            let slot = usize::from(row.slot);
            if slot < MAX_SHIPS {
                places[slot] = row.place;
                finished[slot] = row.finished();
            }
        }
        self.standings
            .record_leg(self.current_leg, &places, &finished);
    }

    /// `slot`'s own points total so far, across every leg recorded.
    #[must_use]
    pub fn points(&self, slot: usize) -> u32 {
        self.standings.totals().get(slot).copied().unwrap_or(0)
    }

    /// `slot`'s own standings rank, 1-based - the value
    /// `Race_BuildEndRaceResult`'s Tournament block feeds `Cell_EvaluateMedal`
    /// through `Race_RecordResult`, meaningful only once every leg has run.
    #[must_use]
    pub fn rank(&self, slot: usize) -> u8 {
        self.standings.ranks().get(slot).copied().unwrap_or(u8::MAX)
    }
}

#[cfg(test)]
mod tests;
