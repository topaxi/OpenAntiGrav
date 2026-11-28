//! Whether the pad's L1 is a flip or EMP button this tick, or the left airbrake.

use crate::app::stage::Stage;

/// Whether L1 reads as the race's flip or EMP button and not an airbrake.
///
/// Only for a race somebody is flying (`live_race`) whose rules put a flip or an EMP on L1
/// (`armed`, `oag_raceplay::RaceSim::l1_is_flip`): photo mode and the replay viewer page on L1,
/// and a finished race has no flip.
pub(super) fn l1_reads_as_flip(
    photo_open: bool,
    viewer_open: bool,
    live_race: bool,
    armed: bool,
) -> bool {
    !photo_open && !viewer_open && live_race && armed
}

impl super::Session {
    /// [`l1_reads_as_flip`] for the stage on screen.
    pub(super) fn l1_is_flip(&self) -> bool {
        let (live_race, armed) = match self.stack.top() {
            Stage::Race(stage) => (
                !stage.race.core.sim.finished(),
                stage.race.core.sim.l1_is_flip(),
            ),
            _ => (false, false),
        };
        l1_reads_as_flip(
            self.photo.is_some(),
            self.viewer.is_some(),
            live_race,
            armed,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::l1_reads_as_flip;

    #[test]
    fn l1_is_the_flip_in_a_live_race_that_arms_one() {
        assert!(l1_reads_as_flip(false, false, true, true));
    }

    #[test]
    fn l1_stays_an_airbrake_where_the_race_arms_no_flip() {
        assert!(!l1_reads_as_flip(false, false, true, false));
    }

    #[test]
    fn l1_pages_in_photo_mode_the_replay_viewer_a_finished_race_and_the_menus() {
        assert!(!l1_reads_as_flip(true, false, true, true), "photo mode");
        assert!(!l1_reads_as_flip(false, true, true, true), "replay viewer");
        assert!(!l1_reads_as_flip(false, false, false, true), "no live race");
    }
}
