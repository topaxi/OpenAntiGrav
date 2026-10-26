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
    /// What the booted disc calls the song, when it names it.
    pub name: Option<oag_music::song_names::SongName>,
}

impl Audio {
    /// The race song that last started, or `None` before the first one has.
    ///
    /// Reads the listing the music worker already kept, never the disc, so it is cheap to call
    /// every tick.
    #[must_use]
    pub fn now_playing(&self) -> Option<NowPlaying> {
        if self.song_serial == 0 {
            return None;
        }
        let index = self.race_index?;
        let name = self.race_context.as_ref().and_then(|(discs, _, _)| {
            let (source, platform) = discs.pick(MusicSource::Auto)?;
            let soundtrack = discs.listings.known(source, platform)?;
            soundtrack.names.get(index).cloned().flatten()
        });
        Some(NowPlaying {
            serial: self.song_serial,
            name,
        })
    }
}
