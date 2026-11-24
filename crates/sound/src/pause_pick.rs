//! The race playlist as a list to pick from, for a pause screen's music list, and the pick it
//! hands back when the race resumes (`docs/ghidra/functions/psp-pulse-usa/music-list.md`).
//!
//! Reads only what is already listed: the frame thread never reads a listing here.

use crate::*;

/// The race playlist's song titles in playlist order, and the one playing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RacePlaylist {
    /// One title per song, in the order [`Audio::pick_race_song`] counts them.
    pub titles: Vec<String>,
    /// The index of the song the race was playing.
    pub playing: usize,
}

impl Audio {
    /// The playlist the race music plays from, worded with each song's title alone (no artist),
    /// or `None` before a race song has played.
    ///
    /// The booted soundtrack's songs first, in its own order, then every other enabled source's.
    /// A disc track is named by its listing (or its other pressing's, already listed); a folder
    /// file by its file name, **chosen**: its tags are read only when it plays.
    #[must_use]
    pub fn race_playlist(&self, discs: &MusicDiscs) -> Option<RacePlaylist> {
        let playing = self.race_index?;
        let mut titles = Vec::new();
        if discs.pool.booted_enabled()
            && let Some((source, platform)) = discs.pick(MusicSource::Auto)
            && let Some(listing) = discs.listings.known(source, platform)
        {
            let author = discs.pool.booted_title();
            let counterpart = discs.counterpart(platform);
            for position in 0..listing.tracks.len() {
                let song = song::name_song(
                    &discs.listings,
                    source,
                    platform,
                    position,
                    author,
                    counterpart,
                );
                titles.push(
                    song.map_or_else(|| format!("Track {}", position + 1), |song| song.name.title),
                );
            }
        }
        for extra in discs.pool.extras() {
            titles.push(match extra {
                pool::Extra::Disc {
                    source,
                    platform,
                    position,
                    title,
                    ..
                } => song::name_song(
                    &discs.listings,
                    &source,
                    platform,
                    position,
                    Some(title),
                    None,
                )
                .map_or_else(|| format!("Track {}", position + 1), |song| song.name.title),
                pool::Extra::File(path) => path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or_default()
                    .to_string(),
            });
        }
        (playing < titles.len()).then_some(RacePlaylist { titles, playing })
    }

    /// Moves the race playlist to song `index`, to start from its beginning when the race music
    /// starts again ([`Self::start_race_music`]), as a new song for the now-playing display.
    /// Picking the song that was playing changes nothing, so it resumes where it was. Returns
    /// whether the playlist moved.
    pub fn pick_race_song(&mut self, index: usize) -> bool {
        if self.race_index == Some(index) {
            return false;
        }
        self.set_race_index(index);
        self.race_position = 0.0;
        // Fetched for the song that was playing or the one after it: landing, either would
        // move the playlist off the pick.
        self.race_prefetch = None;
        self.race_job = None;
        true
    }
}

#[cfg(test)]
mod tests;
