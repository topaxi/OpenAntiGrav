//! Ending a race: the finish condition, and the table it leaves behind.
//!
//! The rule itself is `oag_race`'s - [`oag_race::RaceState::finished`] is set by
//! the lap counter when the player crosses for the last time, and
//! [`oag_race::places`] orders the field. Nothing here re-derives either. What
//! this adds is the *moment*: the tick that flag flips is when the board is
//! taken, and from then on the race has a result.
//!
//! **The simulation is not stopped here.** [`Race::tick`] goes on doing exactly
//! what it did before, byte for byte, so nothing that ticks a race for a fixed
//! count - the trace harness, a headless capture, the disc-backed AI tests -
//! sees a different world than it did. Standing still once the race is over is
//! the *composition root's* call, on the same argument that keeps the camera
//! cycle outside the tick: see `Session::frame`, which stops calling `tick` once
//! [`Race::finished`] answers yes.

use super::*;

use crate::scoreboard::{self, Board, Craft};

impl Race {
    /// Whether the race has reached its finish condition.
    ///
    /// **The player's own condition**, which is what ends the event: the lap
    /// counter sets it when they cross the line for the last time. An opponent
    /// crossing ahead of them finishes *that craft* - it takes a
    /// [`oag_race::Standing::finish_tick`] and stops being ordered by distance -
    /// and does not end the race.
    ///
    /// A speed lap and a Zone run have no lap target, so this stays `false` for
    /// them however long they run. See [`oag_race::Mode::laps_target`].
    #[must_use]
    pub fn finished(&self) -> bool {
        self.sim.world.primary_race().finished
    }

    /// The results, once there are any.
    ///
    /// `None` for the whole race and `Some` from the finishing tick on. The
    /// table is taken at that instant and never revised - see [`RaceView::results`]'s
    /// field for why that matters.
    #[must_use]
    pub fn results(&self) -> Option<&Board> {
        self.view.results.as_ref()
    }

    /// Takes the board, on the tick the race finishes and only then.
    ///
    /// Called at the end of [`Race::tick`], after the standings have been
    /// advanced, so the last crossing is already in the table it reads.
    pub(super) fn capture_results(&mut self) {
        if !self.finished() || self.view.results.is_some() {
            return;
        }
        let places = self.places();
        // Every slot the grid has, whether or not the mode fielded a full one:
        // `ship_count` is 8 for a single race and 1 for the three single-ship
        // modes, and a slot beyond it holds no craft at all.
        //
        // The places are used exactly as `oag_race::places` assigned them,
        // gaps included, rather than renumbered over the rows that survive the
        // cut. That table is what the HUD's own position readout comes from,
        // and a scoreboard that renumbered would contradict the number the
        // player was just looking at.
        let crafts: Vec<Craft> = (0..usize::from(self.sim.world.ship_count))
            .map(|slot| {
                let ship = &self.sim.world.ships[slot];
                Craft {
                    slot: u8::try_from(slot).unwrap_or(u8::MAX),
                    place: places[slot],
                    lap: ship.standing.lap,
                    // The racing clock: the board prints this as the craft's
                    // race time, and the original's own clock starts on the
                    // release, not on the grid.
                    finish_tick: ship.standing.finish_tick.map(oag_race::race_clock_ticks),
                    // The craft's own clock rather than `RaceState`'s, which
                    // only times slot 0 - the two agree there, which is what
                    // `crates/game/tests/lap_times_ground_truth.rs` asserts.
                    best_lap_ticks: ship.standing.best_lap_ticks,
                }
            })
            .collect();
        self.view.results = Some(scoreboard::build(
            &crafts,
            self.sim.world.laps_target(),
            oag_race::race_clock_ticks(self.sim.world.tick),
        ));
    }
}

#[cfg(test)]
mod tests;
