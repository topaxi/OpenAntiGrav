//! The CUSTOM track title, on the session's side: what a `titles_available` row and a
//! `tracks_of` row on it offer and where a launch finds the file. The scan itself is
//! [`oag_game::custom_tracks`], which `--menu-page` draws from too.

use oag_ui::menu::{self, Choice};

use super::Session;

impl Session {
    /// Sets `options.foreign_track` to what the command line named, or none. Run
    /// by every launch that picks its own circuit (RACE, a campaign cell, a
    /// remix) so a CUSTOM pick never carries into the next one, while RACE AGAIN
    /// relaunches `race_options` untouched and so races the same track.
    pub(super) fn inherit_cli_foreign_track(&self, options: &mut oag_raceplay::Options) {
        options.foreign_track.clone_from(&self.cli_foreign_track);
    }

    /// Whether `track_title` is the CUSTOM entry.
    #[allow(clippy::unused_self)]
    pub(super) fn custom_track_selected(&self, track_title: &str) -> bool {
        track_title == oag_game::custom_tracks::CUSTOM
    }

    /// The source a CUSTOM file's track side loads from: the first disc this
    /// machine opens of the title the file names as its host (ADR-0076,
    /// decision 13), never a guest standing in for it, since only that disc holds
    /// what the file borrows by name. `Ok(None)` for a file that names no host
    /// (a Workshop track), which races on the CRAFT TITLE's source.
    pub(super) fn custom_track_host(
        &self,
        value: &str,
    ) -> Result<Option<&oag_game::launcher::Candidate>, String> {
        let Some(host) = self
            .custom_tracks
            .listing(value)
            .and_then(|l| l.host.as_deref())
        else {
            return Ok(None);
        };
        oag_game::custom_tracks::host_disc(&self.titles, host).map(Some)
    }

    /// CUSTOM, as a `titles_available` row offers it - none when the scan found no file.
    pub(super) fn custom_title_choice(&self) -> Option<Choice> {
        use oag_game::custom_tracks::CUSTOM;
        let strings = oag_ui::strings::project_table(self.settings.language.as_deref());
        (!self.custom_tracks.is_empty())
            .then(|| Choice::labelled(CUSTOM, self.custom_tracks.title_label(&strings)))
    }

    /// Called once a frame. Starts reading the custom tracks the first time a page on screen
    /// has a `titles_available` row on CUSTOM, and never while a race is running or
    /// suspended behind its pause menu (a survey under way waits until the menus are back);
    /// when the read has just finished, re-lists the rows that read that row's setting.
    pub(crate) fn poll_custom_tracks(&mut self) {
        let (on_screen, anywhere) = match &self.stage {
            crate::stage::Stage::Menu(stage) => (
                custom_title_row(stage.menu.page().entries.iter()),
                custom_title_row(
                    stage
                        .menu
                        .definition()
                        .pages
                        .iter()
                        .flat_map(|page| &page.entries),
                ),
            ),
            _ => (None, None),
        };
        let in_menus = matches!(&self.stage, crate::stage::Stage::Menu(_));
        let racing = !in_menus || self.suspended_race.is_some();
        self.custom_tracks.hold(racing);
        if on_screen.is_some() && !racing {
            self.custom_tracks.begin();
        }
        if self.custom_tracks.poll()
            && let Some(setting) = anywhere
        {
            self.resupply_queries(Some(&setting));
        }
    }
}

/// The setting of the first `titles_available` row among `entries` that is on CUSTOM.
fn custom_title_row<'a>(entries: impl Iterator<Item = &'a menu::Entry>) -> Option<String> {
    entries.into_iter().find_map(|entry| match entry {
        menu::Entry::Choice {
            setting,
            source: Some(menu::ValueSource::Titles),
            values,
            current,
            ..
        } if values
            .get(*current)
            .is_some_and(|choice| choice.value == oag_game::custom_tracks::CUSTOM) =>
        {
            Some(setting.clone())
        }
        _ => None,
    })
}
