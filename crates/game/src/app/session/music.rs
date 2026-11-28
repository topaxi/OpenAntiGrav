//! The race playlist's place across a restart: put back before a race loads,
//! written on the way out of a race, on quit, and every few seconds while racing.
//! The file, the rules and the bookkeeping are `crate::music_state`; this is
//! only which title the session is keyed on.

use super::Session;
use crate::app::stage::Stage;

/// The song title to count under `played`, when a song has started since `last` and `played`
/// authored its name (see [`Session::count_heard_song`]).
fn song_to_count(last: u32, song: &oag_sound::NowPlaying, played: &str) -> Option<String> {
    if last == song.started || song.author != Some(played) {
        return None;
    }
    song.name.as_ref().map(|name| name.title.clone())
}

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

    /// Counts the race song that started since the last call in the title's lifetime statistics
    /// (`TKR_SONGS`, `TKR_FAVMUS`), once per song started: a `MUSIC SOURCE` switch renames the
    /// song on the marquee and does not count again. Only a name the title being played authored
    /// from its own listing is counted; a pool song another title names, a file of the player's
    /// own or a disc track nothing names is shown and left out, as the ticker has no row of this
    /// title's to quote it under.
    pub(super) fn count_heard_song(&mut self) {
        let Some(song) = self.audio.now_playing() else {
            return;
        };
        let last = std::mem::replace(&mut self.heard_song, song.started);
        if let Some(title) = self.music_title()
            && let Some(counted) = song_to_count(last, &song, title)
        {
            self.records.count_song(title, &counted);
        }
    }

    /// The title this session races: the open source's.
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
        if !matches!(self.stack.top(), Stage::Race(_)) {
            return;
        }
        if let Some(title) = self.music_title() {
            self.music_place.racing_tick(&self.audio, title);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::song_to_count;

    fn song(started: u32, serial: u32, author: Option<&'static str>) -> oag_sound::NowPlaying {
        oag_sound::NowPlaying {
            serial,
            started,
            name: Some(oag_music::song_names::SongName {
                artist: None,
                title: "Song".to_string(),
            }),
            author,
            track: None,
        }
    }

    #[test]
    fn a_song_the_played_title_authored_is_counted_once() {
        let heard = song(1, 1, Some("Wipeout Pulse"));
        assert_eq!(
            song_to_count(0, &heard, "Wipeout Pulse").as_deref(),
            Some("Song")
        );
        assert_eq!(song_to_count(1, &heard, "Wipeout Pulse"), None);
    }

    #[test]
    fn a_foreign_or_unauthored_name_is_shown_and_not_counted() {
        assert_eq!(
            song_to_count(0, &song(1, 1, Some("Wipeout HD")), "Wipeout Pulse"),
            None
        );
        assert_eq!(song_to_count(0, &song(1, 1, None), "Wipeout Pulse"), None);
    }

    #[test]
    fn a_source_switch_rename_is_not_counted_again() {
        // The switch moved the serial on (the marquee restarts) and not the started count.
        let renamed = song(1, 2, Some("Wipeout Pulse"));
        assert_eq!(song_to_count(1, &renamed, "Wipeout Pulse"), None);
    }
}
