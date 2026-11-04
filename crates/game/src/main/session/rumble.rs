//! The race's pad rumble: armed when a race goes live, stopped when it is left.

use super::*;

impl Session {
    /// Builds the live race's rumble from the RUMBLE setting and the title
    /// raced. None where the setting plays nothing or the title ships no
    /// patterns.
    pub(crate) fn arm_rumble(&mut self) {
        self.stop_rumble();
        let (Stage::Race(stage), Some(title)) = (&self.stage, self.title_raced()) else {
            return;
        };
        self.rumbler =
            oag_game::rumble::Rumbler::new(&stage.race, &self.settings.controls.rumble, title.rumble);
    }

    /// Ends the race's voices and silences every pad.
    pub(crate) fn stop_rumble(&mut self) {
        if let Some(mut rumbler) = self.rumbler.take() {
            rumbler.stop(self.controls.pad_mut());
        }
    }
}
