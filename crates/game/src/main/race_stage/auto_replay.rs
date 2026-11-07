//! A race recorded for the REPLAYS page: armed when the session hands the race over, written to
//! `<data dir>/oag/replays/auto/` when the stage is dropped (ADR-0087, decision 1).
//!
//! **Armed only by the session's hand-over**, never by [`super::RaceStage`]'s own build: a race a
//! test, a trace, a capture or a server builds records nothing unless it asks, so every golden
//! and trace runs the code it ran before.
//!
//! **Written when the stage is dropped**, which every way out of a race goes through: the finish,
//! quitting to the menus, restarting, leaving the game. The recording stops at the finish line
//! (the ticks after it run through `Race::tick_finished`, which records none), so a race left on
//! its results table writes the same file as one left a second after the line. A stage dropped
//! off the frame thread writes there.

use log::{debug, warn};
use oag_replay::header::Finish;

use super::RaceStage;

/// A race shorter than this many ticks (five seconds) is not worth a file: a restart pressed on
/// the grid. Chosen, not measured.
pub(crate) const MIN_TICKS: u64 = 300;

impl RaceStage {
    /// Records this race for the REPLAYS page, from its first tick. Called once the session has
    /// finished writing to the race (seats, windows, AI), so the tick-zero hash the recording
    /// opens with is the one a rebuild from the header must reproduce.
    ///
    /// A race that already records for a ghost starts afresh: the same header, seats now right.
    pub(crate) fn arm_auto_replay(&mut self) {
        self.race.start_recording(self.replay_header.clone());
        self.auto_replay = self.race.recording();
        debug!("replays: recording this race for the REPLAYS page: {}", self.auto_replay);
    }

    /// The file this race leaves behind, if it is armed and long enough to be worth one.
    fn finished_recording(&self) -> Option<oag_replay::Replay> {
        if !self.auto_replay {
            return None;
        }
        let mut replay = self.race.recorded()?;
        let ticks = replay.ticks();
        if ticks < MIN_TICKS {
            debug!("replays: not kept, the race ran {ticks} ticks");
            return None;
        }
        replay.header.ticks = ticks;
        replay.header.finish = self
            .race
            .results()
            .and_then(race_board_player)
            .map(|(place, finish_ticks)| Finish {
                place,
                ticks: finish_ticks,
            });
        Some(replay)
    }

    /// Writes this race's recording into the ring, once.
    pub(crate) fn write_auto_replay(&mut self) {
        let Some(replay) = self.finished_recording() else {
            return;
        };
        self.auto_replay = false;
        let Some(root) = oag_game::replays::dir() else {
            return;
        };
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        match oag_game::replays::write_auto_in(&root, &self.result_key.title, unix, &replay) {
            Ok(path) => debug!("replays: kept {} ({} ticks)", path.display(), replay.ticks()),
            Err(e) => warn!("replays: could not keep this race: {e:#}"),
        }
    }
}

/// The player's place and finishing time on a results board.
fn race_board_player(board: &oag_raceplay::scoreboard::Board) -> Option<(u8, u32)> {
    let row = board.player()?;
    let ticks = u32::try_from(row.finish_tick?).ok()?;
    Some((row.place, ticks))
}
