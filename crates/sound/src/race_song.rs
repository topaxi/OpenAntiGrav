//! The race song, held back until the race leaves its countdown.
//!
//! The originals keep the music silent through the fly-over and the countdown and start the song
//! when the race is released (`oag_title::race_song`). The composition root arms the hold for a
//! race from the title's [`oag_title::Music::song_waits_for_go`]; a track that lands while it is
//! armed is parked, decoded and ready, and [`Audio::race_tick`] starts it on the first step the
//! race is no longer gated. Starting is synchronous (a voice on an `Arc<Sound>`), so the song
//! begins on the same tick whichever backend runs and a capture stays a function of the tick
//! count.

use super::*;

/// The hold and the track waiting under it.
#[derive(Default)]
pub(super) struct Hold {
    armed: bool,
    parked: Option<Parked>,
}

struct Parked {
    index: usize,
    loaded: Loaded,
    seek: Option<f64>,
}

impl Hold {
    /// Whether a landed race track waits for the release.
    pub(super) fn holding(&self) -> bool {
        self.armed
    }

    pub(super) fn park(&mut self, index: usize, loaded: Loaded, seek: Option<f64>) {
        self.parked = Some(Parked {
            index,
            loaded,
            seek,
        });
    }

    pub(super) fn drop_parked(&mut self) {
        self.parked = None;
    }

    /// The release the parked track was read off, when a track waits for the release.
    pub(super) fn parked_from(&self) -> Option<Option<Platform>> {
        self.parked.as_ref().map(|parked| parked.loaded.from)
    }

    /// Swaps the parked track `index` for `loaded`, keeping the place it resumes from. `false`
    /// when nothing is parked under that index (the song started, or the race moved on).
    pub(super) fn replace_parked(&mut self, index: usize, loaded: Loaded) -> bool {
        match &mut self.parked {
            Some(parked) if parked.index == index => {
                parked.loaded = loaded;
                true
            }
            _ => false,
        }
    }
}

impl Audio {
    /// Arms the hold for the race about to start (`until_go`), or clears a stale one.
    ///
    /// Called by the race hand-off before it starts the race music, with the title's rule. While
    /// armed no race voice starts and [`Self::now_playing`] reports nothing; the first
    /// [`Self::race_tick`] whose frame is no longer gated releases it. A race that never ticks
    /// (a menu visit) leaves the next arming to replace it.
    pub fn hold_race_song(&mut self, until_go: bool) {
        self.song_hold.armed = until_go;
        self.song_hold.parked = None;
    }

    /// The race left its countdown: starts the parked track, or lets one still being read start
    /// the moment it lands.
    pub(super) fn release_race_song(&mut self) {
        if !self.song_hold.armed {
            return;
        }
        self.song_hold.armed = false;
        let Some(Parked {
            index,
            loaded,
            seek,
        }) = self.song_hold.parked.take()
        else {
            return;
        };
        self.start_race_voice(index, loaded, seek);
        if self.race_voice.is_some() {
            self.race_position = 0.0;
        }
    }
}
