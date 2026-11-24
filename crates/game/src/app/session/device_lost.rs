//! What the frame loop does when the GPU device is lost (`crate::app::device_loss`): save, say
//! so, and stop drawing. `docs/architecture/device-loss.md`.

use log::error;

use crate::app::device_loss;
use crate::app::stage::Stage;

use super::Session;

/// The ids of the words, in `assets/ui/strings/english.toml`.
mod words {
    /// The line the next run shows over its first screen.
    pub(crate) const LAST_RUN: &str = "OAG_DEVICE_LOST_LAST_RUN";
    /// The title of the window left open, where nothing can be drawn.
    pub(crate) const TITLE: &str = "OAG_DEVICE_LOST_TITLE";
}

impl Session {
    /// The top of a frame: whether the device went since the last one. If so, everything that
    /// can be kept is kept (the campaign position, the replays, the records, the playlist), a
    /// note is left for the next run, the window says what happened in the one place that needs
    /// no GPU, and the run is told to end. Returns whether it did, and the frame draws nothing.
    pub(crate) fn device_check(&mut self) -> bool {
        let Some(lost) = self.gpu.loss.take() else {
            return false;
        };
        error!(
            "the graphics device was lost ({:?}): {}. Progress and the replay were saved; start the game again",
            lost.reason, lost.message
        );
        self.wind_down();
        device_loss::leave_marker(&lost);
        let title = self.project_word(words::TITLE);
        self.gpu.output.window.set_title(&title);
        self.lost = Some(lost);
        self.quit = true;
        true
    }

    /// The note over the first screen of a run whose predecessor ended on a lost device.
    pub(crate) fn say_last_run_lost_its_device(&mut self) {
        self.notice = Some(super::shot_open::Notice::new(
            self.project_word(words::LAST_RUN),
        ));
    }

    /// Whether a game that lost its device may start itself again: on the front end, with
    /// nothing that would run twice. A race has been saved and ends.
    pub(crate) fn restartable(&self) -> bool {
        matches!(
            self.stage,
            Stage::Menu(_) | Stage::Frontend(_) | Stage::Launcher(_)
        ) && self.suspended_race.is_none()
            && self.net.is_none()
    }
}
