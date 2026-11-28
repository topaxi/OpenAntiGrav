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
    /// The top of a frame: the last run's note once the menu is up, then whether the device is
    /// lost. If it just was, everything that can be kept is kept (a race's result and best lap as
    /// leaving it would, the replays, the campaign position, the records, the playlist), a note is
    /// left for the next run, and the window says what happened in the one place that needs no
    /// GPU. From then on the frame draws nothing and ticks nothing: the caller (`App::after_loss`)
    /// restarts the game, leaves the window up as it is, or ends the run. Returns whether the
    /// device is lost.
    pub(crate) fn device_check(&mut self) -> bool {
        if self.last_run_lost && matches!(self.stack.top(), Stage::Menu(_)) {
            self.last_run_lost = false;
            self.notice = Some(super::shot_open::Notice::new(
                self.project_word(words::LAST_RUN),
            ));
            // Cleared as the line goes up, not when the run opened: a run that never reaches the
            // menu (a race, a closed window) leaves the note for the one that does.
            device_loss::clear_marker();
        }
        if self.lost.is_some() {
            return true;
        }
        let Some(lost) = self.gpu.loss.take() else {
            return false;
        };
        error!(
            "the graphics device was lost ({:?}){}. Saved what there was to save; start the game again",
            lost.reason,
            if lost.message.is_empty() {
                String::new()
            } else {
                format!(": {}", lost.message)
            }
        );
        self.capture_race_result();
        self.wind_down();
        device_loss::leave_marker(&lost);
        let title = self.project_word(words::TITLE);
        self.gpu.output.window.set_title(&title);
        self.lost = Some(lost);
        true
    }

    /// The run before this one ended on a lost device: the main menu says so when it first
    /// shows, which is the first screen a player reads (the boot sequence before it is a
    /// movie, and a notice there would run out unseen).
    pub(crate) fn last_run_lost_its_device(&mut self) {
        self.last_run_lost = true;
    }
}
