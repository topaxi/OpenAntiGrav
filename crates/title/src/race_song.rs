//! When a race's song begins to sound.
//!
//! The originals keep the music silent through the fly-over and the countdown and start the
//! song, and the now-playing display with it, when the race leaves its countdown. Pulse's
//! `RaceMode_SetState(1)` stops whatever the music player held and its
//! `RaceMode_UpdateCountdown` calls `MusicPlayer_StartNextSong` right after
//! `RaceMode_SetState(2)`; HD's `RaceManager_UpdateCountdown` ends in `MusicManager_ClearHold`.
//! A title says which rule it runs, and where that came from, as
//! [`crate::Music::race_song_start`].

/// When the race song begins to sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaceSongStart {
    /// As the race loads, over the fly-over and the countdown. What this build did before the
    /// originals were read; kept for a title whose original has not been, and for a test that
    /// needs the song at tick 0.
    AtLoad,
    /// When the race leaves its countdown: on the step that voices `go`, and silent before it.
    AtGo,
}
