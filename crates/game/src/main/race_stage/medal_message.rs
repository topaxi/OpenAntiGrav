//! The medal message and the standing medal line, driven off
//! [`oag_game::medal_watch`] - see there for what is chosen and what is not.

use oag_tables::race_campaign::Mode;

use super::RaceStage;

impl RaceStage {
    /// One tick of the medal watch for a campaign Zone or Speed Lap race.
    /// Called straight after `Race::tick`, so a lap or a zone that lands on
    /// this tick is announced on it. Time Trial earns its medal at the finish
    /// and stays silent here; its live pace caption already tells the player
    /// where they stand.
    pub(crate) fn tick_messages(&mut self) {
        let Some(cell) = self.campaign_cell.as_ref() else {
            return;
        };
        if !matches!(cell.mode, Mode::Zone | Mode::SpeedLap) {
            return;
        }
        let difficulty = self
            .campaign_difficulty
            .unwrap_or(oag_tables::race_campaign::Difficulty::Medium);
        oag_game::medal_watch::tick(cell, difficulty, &mut self.earned_medal, &mut self.race);
    }
}

#[cfg(test)]
mod tests {
    /// The windowed session and the headless capture are the two places a race
    /// ticks, and a GPU scene keeps either from running under a unit test, so
    /// this is the one guard that dropping the call from either fails a test:
    /// each source must still reach `oag_game::medal_watch::tick`, the function
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
            watcher.contains("oag_game::medal_watch::tick("),
            "RaceStage::tick_messages must run the shared medal watch"
        );
        let capture = include_str!("../../race_capture/tick.rs");
        assert!(
            capture.contains("crate::medal_watch::tick("),
            "the headless capture's tick must run the shared medal watch"
        );
    }
}
