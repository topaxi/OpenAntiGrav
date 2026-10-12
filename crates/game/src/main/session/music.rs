//! The race playlist's place across a restart: put back before a race loads,
//! written on the way out of a race, on quit, and every few seconds while racing.
//! The file and its rules are `oag_game::music_state`.

use oag_game::music_state::{self, SAVE_EVERY_SECONDS, Saver};

use super::Session;
use crate::stage::Stage;

/// One run's bookkeeping for the saved place.
#[derive(Debug, Default)]
pub(crate) struct Place {
    restored: bool,
    saver: Saver,
    ticks: u32,
}

impl Session {
    /// Once per run, before the first race reserves its track.
    pub(super) fn resume_race_music(&mut self) {
        if std::mem::replace(&mut self.music_place.restored, true) {
            return;
        }
        if let Some(title) = self.shell.as_ref().map(|shell| shell.title.name) {
            music_state::resume(&mut self.audio, &self.music_discs, title);
        }
    }

    /// Writes the place now if it moved. Cheap when it did not.
    pub(crate) fn save_race_music(&mut self) {
        self.music_place.ticks = 0;
        // No shell is the `--race` route: a developer launch that neither
        // restores nor saves a place.
        if let Some(title) = self.shell.as_ref().map(|shell| shell.title.name) {
            self.music_place.saver.save(&self.audio, title);
        }
    }

    /// Called from the fixed-timestep loop: a race saves its place every
    /// [`SAVE_EVERY_SECONDS`] seconds of ticks.
    pub(super) fn save_race_music_periodically(&mut self) {
        if !matches!(self.stage, Stage::Race(_)) {
            return;
        }
        self.music_place.ticks += 1;
        if self.music_place.ticks >= SAVE_EVERY_SECONDS * 60 {
            self.save_race_music();
        }
    }
}
