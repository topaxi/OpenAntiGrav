//! Restart, next and previous for the race song, on the pad's music actions.
//!
//! The race playlist is one numbered list walked in order (`pool`): next is the index after
//! the playing one, previous the index before it, both wrapping. There is no shuffle, so
//! previous is exactly the song that played before when the player has only gone forward.
//! A playlist of one song, no race music, a song still held for the green light, or a track
//! still being fetched does nothing.

use crate::*;

impl Audio {
    /// Whether a skip has anywhere to go: a race voice is sounding, nothing is being fetched,
    /// the song is not held for the light, and the playlist has more than one song.
    fn can_skip_race_song(&self) -> bool {
        let Some((discs, _, _)) = &self.race_context else {
            return false;
        };
        self.race_voice.is_some()
            && self.race_job.is_none()
            && !self.song_hold.holding()
            && pool::playlist_len(&discs.pool, self.race_tracks) > 1
    }

    /// Starts the playing race song again from its beginning. Not a new song for the
    /// now-playing display. Returns whether a song was restarted.
    pub fn restart_race_song(&mut self) -> bool {
        let Some(id) = self.race_voice else {
            return false;
        };
        self.output.with_mixer(|mixer| mixer.seek(id, 0.0))
    }

    /// Moves the race playlist to the next song. Returns whether it moved.
    pub fn next_race_song(&mut self) -> bool {
        if !self.can_skip_race_song() {
            return false;
        }
        self.advance_race_track();
        true
    }

    /// Moves the race playlist to the previous song. Returns whether it moved.
    pub fn previous_race_song(&mut self) -> bool {
        if !self.can_skip_race_song() {
            return false;
        }
        let (Some((discs, choice, cache_dir)), Some(index)) =
            (self.race_context.clone(), self.race_index)
        else {
            return false;
        };
        if let Some(id) = self.race_voice.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
        // Prefetched for the song after the one that was playing.
        self.race_prefetch = None;
        self.race_job = Some(MusicFetchWorker::spawn(
            discs,
            choice,
            cache_dir,
            race_music::RacePick::before(index),
            "race-music",
        ));
        self.poll_race_job();
        self.settle_if_dumping();
        true
    }
}

#[cfg(test)]
mod tests;
