//! The session's half of the REPLAYS page: keeping a recorded race when the game ends under it,
//! and (below) opening and watching one.

use crate::stage::Stage;

use super::Session;

impl Session {
    /// Writes the recordings of the race on screen and of the one parked behind the menus: the
    /// process is ending, and a stage that is never dropped writes nothing else.
    pub(crate) fn keep_replays(&mut self) {
        if let Stage::Race(stage) = &mut self.stage {
            stage.write_auto_replay();
        }
        if let Some(stage) = self.suspended_race.as_mut() {
            stage.write_auto_replay();
        }
    }
}
