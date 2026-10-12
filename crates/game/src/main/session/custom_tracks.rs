//! RACE REMIX's CUSTOM track title, on the session's side: what TRACK TITLE and
//! TRACK offer and where a launch finds the file. The scan itself is
//! [`oag_game::custom_tracks`], which `--menu-page` draws from too.

use oag_game::custom_tracks::CUSTOM;
use oag_ui::menu::Choice;

use super::Session;
use super::remix_menu;

impl Session {
    /// Sets `options.foreign_track` to what the command line named, or none. Run
    /// by every launch that picks its own circuit (RACE, a campaign cell, a
    /// remix) so a CUSTOM pick never carries into the next one, while RACE AGAIN
    /// relaunches `race_options` untouched and so races the same track.
    pub(super) fn inherit_cli_foreign_track(&self, options: &mut oag_raceplay::Options) {
        options.foreign_track.clone_from(&self.cli_foreign_track);
    }

    /// CUSTOM, as TRACK TITLE offers it - none when the scan found no file.
    pub(super) fn custom_title_choice(&self) -> Option<Choice> {
        let strings = oag_ui::strings::project_table(self.settings.language.as_deref());
        (!self.custom_tracks.is_empty())
            .then(|| Choice::labelled(CUSTOM, self.custom_tracks.title_label(&strings)))
    }

    /// The TRACK row's options for whatever `remix.track_title` names: the
    /// scan's for CUSTOM, a title's own circuits otherwise. `None` when the
    /// title's source will not open, which supplies and settles nothing.
    pub(super) fn remix_track_options(&self) -> Option<Vec<Choice>> {
        let title = &self.settings.remix.track_title;
        if title == CUSTOM {
            let strings = oag_ui::strings::project_table(self.settings.language.as_deref());
            return Some(self.custom_tracks.choices(&strings));
        }
        let catalogue = self.remix_catalogue_for(title)?;
        Some(remix_menu::remix_track_choices(&catalogue))
    }

    /// Called once a frame: when the scan has just finished and the TRACK row
    /// is showing it, hands the row its checked list.
    pub(crate) fn poll_custom_tracks(&mut self) {
        if self.custom_tracks.poll() && self.settings.remix.track_title == CUSTOM {
            self.resupply_remix_tracks();
        }
    }
}
