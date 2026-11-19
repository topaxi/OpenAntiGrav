//! The race playlist's place across a restart: put back before a race loads,
//! written on the way out of a race, on quit, and every few seconds while racing.
//! The file, the rules and the bookkeeping are `crate::music_state`; this is
//! only which title the session is keyed on.

use super::Session;
use crate::app::stage::Stage;

impl Session {
    /// The title the place is keyed on: the shell's on every front-end route,
    /// the one `--race` opened on the command-line route.
    pub(crate) fn music_title(&self) -> Option<&'static str> {
        self.shell
            .as_ref()
            .map(|shell| shell.title)
            .or(self.race_title)
            .map(|title| title.name)
    }

    /// Once per run, before the first race reserves its track.
    pub(super) fn resume_race_music(&mut self) {
        if let Some(title) = self.music_title() {
            self.music_place
                .resume(&mut self.audio, &self.music_discs, title);
        }
    }

    /// Arms the race song for the title this session races, before a fresh race starts its music:
    /// a title whose original starts the song at the release holds it until the countdown ends.
    pub(super) fn hold_race_song(&mut self) {
        if let Some(title) = self.title_raced() {
            crate::sound::hold_race_song(&mut self.audio, title);
        }
    }

    /// Counts the race song that started since the last call, as the disc names it, in the
    /// title's lifetime statistics (`TKR_SONGS`, `TKR_FAVMUS`). A song the disc does not name
    /// is not counted: the ticker would have no name to quote.
    pub(super) fn count_heard_song(&mut self) {
        let Some(song) = self.audio.now_playing() else {
            return;
        };
        if std::mem::replace(&mut self.heard_song, song.serial) == song.serial {
            return;
        }
        if let (Some(title), Some(name)) = (self.music_title(), song.name) {
            self.records.count_song(title, &name.title);
        }
    }

    /// The title this session races.
    pub(super) fn title_raced(&self) -> Option<&'static oag_title::Title> {
        self.shell
            .as_ref()
            .map(|shell| shell.title)
            .or(self.race_title)
    }

    /// Writes the place now if it moved. Cheap when it did not.
    pub(crate) fn save_race_music(&mut self) {
        if let Some(title) = self.music_title() {
            self.music_place.save(&self.audio, title);
        }
    }

    /// Called from the fixed-timestep loop: a race saves its place every
    /// `SAVE_EVERY_SECONDS` seconds of ticks.
    pub(super) fn save_race_music_periodically(&mut self) {
        if !matches!(self.stage, Stage::Race(_)) {
            return;
        }
        if let Some(title) = self.music_title() {
            self.music_place.racing_tick(&self.audio, title);
        }
    }
}
