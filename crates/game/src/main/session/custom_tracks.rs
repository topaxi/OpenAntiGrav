//! RACE REMIX's CUSTOM track title, on the session's side: what TRACK TITLE and
//! TRACK offer and where a launch finds the file. The scan itself is
//! [`oag_game::custom_tracks`], which `--menu-page` draws from too.

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

    /// Whether `track_title` is the CUSTOM entry. Never, in a build without the
    /// `ballisticng` feature, which offers none.
    #[allow(clippy::unused_self)]
    pub(super) fn custom_track_selected(&self, track_title: &str) -> bool {
        #[cfg(feature = "ballisticng")]
        return track_title == oag_game::custom_tracks::CUSTOM;
        #[cfg(not(feature = "ballisticng"))]
        return {
            let _ = track_title;
            false
        };
    }

    /// Sets what racing the TRACK row's custom file needs on `options`, or says
    /// why it cannot be raced.
    #[cfg(feature = "ballisticng")]
    pub(super) fn apply_custom_track(
        &self,
        options: &mut oag_raceplay::Options,
    ) -> Result<(), String> {
        let path = self.custom_tracks.raceable(&self.settings.remix.track)?;
        oag_game::custom_tracks::apply(options, path);
        Ok(())
    }

    #[cfg(not(feature = "ballisticng"))]
    #[allow(clippy::unused_self)]
    pub(super) fn apply_custom_track(
        &self,
        _options: &mut oag_raceplay::Options,
    ) -> Result<(), String> {
        Err("custom tracks need a build with the `ballisticng` feature".to_string())
    }

    /// CUSTOM, as TRACK TITLE offers it - none when the scan found no file.
    #[cfg(feature = "ballisticng")]
    pub(super) fn custom_title_choice(&self) -> Option<Choice> {
        use oag_game::custom_tracks::CUSTOM;
        let strings = oag_ui::strings::project_table(self.settings.language.as_deref());
        (!self.custom_tracks.is_empty())
            .then(|| Choice::labelled(CUSTOM, self.custom_tracks.title_label(&strings)))
    }

    /// No CUSTOM title without the `ballisticng` feature: no row, dead or not.
    #[cfg(not(feature = "ballisticng"))]
    #[allow(clippy::unused_self)]
    pub(super) fn custom_title_choice(&self) -> Option<Choice> {
        None
    }

    /// The TRACK row's options for whatever `remix.track_title` names: the
    /// scan's for CUSTOM, a title's own circuits otherwise. `None` when the
    /// title's source will not open, which supplies and settles nothing.
    pub(super) fn remix_track_options(&self) -> Option<Vec<Choice>> {
        let title = &self.settings.remix.track_title;
        #[cfg(feature = "ballisticng")]
        if title == oag_game::custom_tracks::CUSTOM {
            let strings = oag_ui::strings::project_table(self.settings.language.as_deref());
            return Some(self.custom_tracks.choices(&strings));
        }
        let catalogue = self.remix_catalogue_for(title)?;
        Some(remix_menu::remix_track_choices(&catalogue))
    }

    /// Called once a frame. Starts reading the custom tracks the first time the
    /// RACE REMIX page shows CUSTOM, and never while a race is running or
    /// suspended behind its pause menu; when the read has just finished and the
    /// TRACK row is showing it, hands the row its checked list.
    #[cfg(feature = "ballisticng")]
    pub(crate) fn poll_custom_tracks(&mut self) {
        use oag_game::custom_tracks::CUSTOM;
        let on_remix_page = matches!(
            &self.stage,
            crate::stage::Stage::Menu(stage) if stage.menu.page().id == "remix"
        );
        if on_remix_page
            && self.suspended_race.is_none()
            && self.settings.remix.track_title == CUSTOM
        {
            self.custom_tracks.begin();
        }
        if self.custom_tracks.poll() && self.settings.remix.track_title == CUSTOM {
            self.resupply_remix_tracks();
        }
    }

    #[cfg(not(feature = "ballisticng"))]
    #[allow(clippy::unused_self)]
    pub(crate) fn poll_custom_tracks(&mut self) {}
}
