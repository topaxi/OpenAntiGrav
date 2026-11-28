//! The medal message and the standing medal line, driven off
//! [`crate::medal_watch`] - see there for what is chosen and what is not.

use oag_tables::race_campaign::Mode;

use super::RaceStage;

impl RaceStage {
    /// One tick of the medal watch for a campaign Zone or Speed Lap race.
    /// Called straight after `Race::tick`, so a lap or a zone that lands on
    /// this tick is announced on it. Time Trial earns its medal at the finish
    /// and stays silent here; its live pace caption already tells the player
    /// where they stand.
    pub(crate) fn tick_messages(&mut self) {
        let Some(cell) = self.run.campaign_cell.as_ref() else {
            return;
        };
        let detonator = cell.mode == Mode::Other(crate::campaign::DETONATOR_CELL_MODE.to_string());
        if !detonator && !matches!(cell.mode, Mode::Zone | Mode::SpeedLap) {
            return;
        }
        let difficulty = self
            .run
            .campaign_difficulty
            .unwrap_or(oag_tables::race_campaign::Difficulty::Medium);
        crate::medal_watch::tick(
            cell,
            difficulty,
            self.title.campaign.counting_modes,
            &mut self.earned_medal,
            &mut self.race,
        );
    }
}

#[cfg(test)]
mod tests {
    /// The windowed session and the headless capture are the two places a race
    /// ticks, and a GPU scene keeps either from running under a unit test, so
    /// this is the one guard that dropping the call from either fails a test:
    /// each source must still reach `crate::medal_watch::tick`, the function
    /// the disc-backed `medal_message_ground_truth` drives through a real lap.
    #[test]
    fn both_tick_loops_still_call_the_medal_watch() {
        let session = include_str!("../session/frame.rs");
        assert!(
            session.contains("stage.tick_messages()"),
            "the windowed loop must call RaceStage::tick_messages after Race::tick"
        );
        let watcher = include_str!("medal_message.rs");
        assert!(
            watcher.contains("crate::medal_watch::tick("),
            "RaceStage::tick_messages must run the shared medal watch"
        );
        let capture = include_str!("../../race_capture/tick.rs");
        assert!(
            capture.contains("crate::medal_watch::tick("),
            "the headless capture's tick must run the shared medal watch"
        );
    }

    /// Detonator's medal models: a cell's targets reach the race through the shared watch, from
    /// both tick loops, and both draw sites (the window and the headless capture) draw the model.
    /// Source-level for the same reason as above; the disc-backed
    /// `detonator_hud_ground_truth` holds the readout and the decode.
    #[test]
    fn the_detonator_medal_is_still_fed_and_drawn_in_both_places() {
        let watch = include_str!("../../medal_watch.rs");
        assert!(
            watch.contains("race.set_medal_targets("),
            "medal_watch::tick must hand a Detonator cell's targets to the race"
        );
        let watcher = include_str!("medal_message.rs");
        assert!(
            watcher.contains("DETONATOR_CELL_MODE"),
            "RaceStage::tick_messages must run the watch for a Detonator cell"
        );
        let stage = include_str!("../race_stage.rs");
        assert!(
            stage.contains("medals.draw("),
            "RaceStage::draw_window_hud must draw the medal"
        );
        let capture = include_str!("../../race_capture/overlays.rs");
        assert!(
            capture.contains("race.detonator_medal()"),
            "the headless capture must draw the medal"
        );
    }

    /// An Elimination cell's medal value is `medal_watch::elimination_value`'s, which reads
    /// the place on HD and Omega: the race stage can not be built under a unit test (it owns a
    /// GPU scene), so this fails when `campaign_tier` stops asking it.
    #[test]
    fn the_campaign_tier_asks_for_the_elimination_value() {
        let stage = include_str!("../race_stage.rs");
        assert!(
            stage.contains("crate::medal_watch::elimination_value("),
            "campaign_tier must take an Elimination cell's value from medal_watch"
        );
        assert!(
            stage.contains("self.race.core.eliminator_place()"),
            "the place is the end-of-race board's, SimRace::eliminator_place"
        );
    }

    /// A Detonator cell's medal is the cannon's score: the race stage can not be built under a
    /// unit test, so this fails when `campaign_tier` stops asking `medal_watch` for it or stops
    /// reading the score off the race.
    #[test]
    fn the_campaign_tier_asks_for_the_detonator_score() {
        let stage = include_str!("../race_stage.rs");
        assert!(
            stage.contains("crate::medal_watch::detonator_value("),
            "campaign_tier must take a Detonator cell's value from medal_watch"
        );
        assert!(
            stage.contains("self.race.core.sim.detonator_score()"),
            "the score is the cannon's, read through the race's accessor"
        );
    }
}
