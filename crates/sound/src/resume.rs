//! The race playlist's place, kept across a restart.
//!
//! The playlist already survives a visit to the menus in memory
//! ([`Audio::pause_race_music`] saves [`Audio::race_position`] and leaves
//! [`Audio::race_index`] alone). This module is the two seams that let a host
//! carry the same two numbers across a process: [`Audio::race_music_state`] reads
//! them out, [`Audio::restore_race_music`] puts them back before the first race
//! of a run reserves its track.
//!
//! **Chosen, not measured.** The original's race music is not recovered (see
//! [`Audio::race_voice`]), so what is resumed is this engine's own playlist: a
//! plain cycle of the booted disc's soundtrack order. There is no shuffle, so the
//! "order" is the index and nothing else needs saving.
//!
//! No file is read or written here; `oag_game::music_state` owns the TOML.

use super::*;

/// Where the race playlist stood, at whole-second resolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceMusicState {
    /// The track, in the **booted** disc's soundtrack order ([`Audio::race_index`]).
    pub index: usize,
    /// How far into it, in whole seconds, rounded to the nearest.
    pub second: u32,
    /// How many tracks the soundtrack listed, to notice a changed playlist.
    pub tracks: usize,
    /// The track's own length in seconds as the decoded sound measured it, to
    /// notice a changed playlist the count alone would miss (a swapped track).
    pub track_seconds: f64,
}

/// Why a saved [`RaceMusicState`] was not used; the playlist then starts where it
/// normally does. `Display` is the load-report line.
#[derive(Debug, Clone, PartialEq)]
pub enum Unusable {
    /// The booted soundtrack cannot be listed (no soundtrack, or an unreadable image).
    Unlisted,
    /// The soundtrack now lists a different number of tracks.
    CountChanged { saved: usize, now: usize },
    /// The saved track is past the end of the list.
    OutOfRange { index: usize, now: usize },
    /// The track at that index is not the same length any more.
    TrackChanged { index: usize, saved: f64, now: f64 },
    /// The saved second is not inside the track.
    PastEnd { second: u32, length: f64 },
}

impl std::fmt::Display for Unusable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unlisted => write!(f, "the booted soundtrack cannot be listed"),
            Self::CountChanged { saved, now } => {
                write!(
                    f,
                    "the soundtrack lists {now} tracks, it listed {saved} when saved"
                )
            }
            Self::OutOfRange { index, now } => {
                write!(
                    f,
                    "saved track {index} is not in the {now}-track soundtrack"
                )
            }
            Self::TrackChanged { index, saved, now } => {
                write!(
                    f,
                    "track {index} is {now:.1} s, it was {saved:.1} s when saved"
                )
            }
            Self::PastEnd { second, length } => {
                write!(
                    f,
                    "saved second {second} is past the end of a {length:.1} s track"
                )
            }
        }
    }
}

/// How far a track's length may differ from the saved one and still be the same
/// recording: the two PSP/PS2 releases of one track agree to 11.4 ms, a
/// different recording is at least 130 ms away (see [`Soundtrack::pairs_with`]).
const SAME_TRACK_SECONDS: f64 = 0.05;

impl RaceMusicState {
    /// Whether this state still names the same place in `listing`, the booted
    /// soundtrack's track lengths in order.
    ///
    /// # Errors
    /// The first reason it does not.
    pub fn check(&self, listing: &[f64]) -> Result<(), Unusable> {
        if listing.is_empty() {
            return Err(Unusable::Unlisted);
        }
        if listing.len() != self.tracks {
            return Err(Unusable::CountChanged {
                saved: self.tracks,
                now: listing.len(),
            });
        }
        let Some(&length) = listing.get(self.index) else {
            return Err(Unusable::OutOfRange {
                index: self.index,
                now: listing.len(),
            });
        };
        if (length - self.track_seconds).abs() > SAME_TRACK_SECONDS {
            return Err(Unusable::TrackChanged {
                index: self.index,
                saved: self.track_seconds,
                now: length,
            });
        }
        if f64::from(self.second) >= length {
            return Err(Unusable::PastEnd {
                second: self.second,
                length,
            });
        }
        Ok(())
    }
}

impl Audio {
    /// The race playlist's place right now, or [`None`] before a race has
    /// started one (nothing to save) or when the track's length is not known.
    ///
    /// Reads the live voice while a race plays and the position
    /// [`Self::pause_race_music`] saved otherwise, so it is right in a race, in
    /// the menus after one and at a quit from either.
    #[must_use]
    pub fn race_music_state(&self) -> Option<RaceMusicState> {
        let index = self.race_index?;
        if self.race_tracks == 0 {
            return None;
        }
        let (_, _, sound) = self
            .race_cache
            .as_ref()
            .filter(|(_, held, _)| *held == index)?;
        let seconds = self
            .race_voice
            .and_then(|id| self.output.with_mixer(|mixer| mixer.position(id)))
            .unwrap_or(self.race_position);
        Some(RaceMusicState {
            index,
            // Rounded to the nearest second; a negative or NaN position saturates to the start.
            second: seconds.max(0.0).round() as u32,
            tracks: self.race_tracks,
            track_seconds: f64::from(sound.seconds()),
        })
    }

    /// Puts a saved place back, so the next race resumes there instead of at the
    /// playlist's normal start. Does nothing once a race has started the playlist
    /// this run.
    ///
    /// # Errors
    /// [`Unusable`] when the booted soundtrack no longer matches what was saved;
    /// nothing is changed then.
    pub fn restore_race_music(
        &mut self,
        discs: &MusicDiscs,
        saved: &RaceMusicState,
    ) -> Result<(), Unusable> {
        if self.race_index.is_some() {
            return Ok(());
        }
        self.apply_restored(saved, &Self::booted_listing(discs))
    }

    /// [`Self::restore_race_music`] against a listing already read.
    pub(super) fn apply_restored(
        &mut self,
        saved: &RaceMusicState,
        listing: &[f64],
    ) -> Result<(), Unusable> {
        saved.check(listing)?;
        self.race_index = Some(saved.index);
        self.race_position = f64::from(saved.second);
        self.race_tracks = saved.tracks;
        Ok(())
    }

    /// The booted soundtrack's track lengths in order; empty when unreadable.
    fn booted_listing(discs: &MusicDiscs) -> Vec<f64> {
        let Some((source, platform)) = discs.pick(MusicSource::Auto) else {
            return Vec::new();
        };
        Soundtrack::read(discs.library, source, platform)
            .ok()
            .flatten()
            .map_or_else(Vec::new, |soundtrack| {
                soundtrack
                    .tracks
                    .iter()
                    .map(|track| track.seconds)
                    .collect()
            })
    }
}
