//! Which song a race is playing, for the HUD's now-playing display.
//!
//! The music selection lives here and never depends on the device: the null backend has a
//! mixer too, so a `--no-audio` run lands the same race song and reports it.

use super::*;

/// The race song that last started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NowPlaying {
    /// Counts the race songs that have started; changes each time one does, so a caller restarts
    /// its display on a change and not on a value.
    pub serial: u32,
    /// Counts the songs that have started, not the renames: a `MUSIC SOURCE` switch gives the
    /// song another name (a new `serial`, so the display restarts) without starting another.
    pub started: u32,
    /// What the song is called, when its source says.
    pub name: Option<oag_music::song_names::SongName>,
    /// The title whose own listing authored `name`; `None` when no title did (a file's tags or
    /// name, a disc track nothing names).
    pub author: Option<&'static str>,
    /// For a disc track nothing names: its 1-based place in its disc's listing, which the display
    /// words through the string table (`name.title` is the English wording).
    pub track: Option<usize>,
}

impl Audio {
    /// The race song that last started, or `None` before the first one has - and while the song
    /// is held for the green light, so a race never shows the song the race before it started.
    ///
    /// Reads the name stored when the voice started, never a listing, so it is cheap to call
    /// every tick.
    #[must_use]
    pub fn now_playing(&self) -> Option<NowPlaying> {
        if self.song_serial == 0 || self.song_hold.holding() {
            return None;
        }
        let song = self.song.as_ref();
        Some(NowPlaying {
            serial: self.song_serial,
            started: self.song_starts,
            name: song.map(|song| song.name.clone()),
            author: song.and_then(|song| song.author),
            track: song.and_then(|song| song.track),
        })
    }
}
