//! [`ShotRecord`]: what a screenshot carries so its frame can be made again.
//!
//! A shot's PNG holds this in one private ancillary chunk, `oaGs` ([`CHUNK`]): the run's
//! [`Replay`] cut at the tick of the shot (the human slots' inputs, the launch request and the
//! seed in its header - the file ADR-0055 already defines, not a second format), the tick, and
//! the race's state hash at that tick. A replay stores a hash a second; a shot is rarely on one,
//! so the hash is its own field. The AI needs no stream: it reproduces itself from the seed.
//!
//! ```text
//! u8 version (1), u64 tick, u64 state hash, then the replay file raw-deflated
//! ```
//!
//! All integers little-endian. The frame is an ancillary chunk's data: a reader that does not
//! know it skips it.

use std::io::{Read as _, Write as _};

use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;

use crate::file::{ReadError, Replay};

/// The PNG chunk type: ancillary (`o`), private (`a`), reserved bit set (`G`), safe to copy
/// (`s`).
pub const CHUNK: [u8; 4] = *b"oaGs";

/// The version this build writes and the only one it reads.
pub const VERSION: u8 = 1;

/// Why a chunk could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ShotError {
    /// Shorter than the frame, or another version.
    #[error("not a shot record this build reads (version {VERSION})")]
    Frame,
    /// The deflate stream is damaged.
    #[error("the shot record's stream is damaged: {0}")]
    Inflate(std::io::Error),
    /// The replay inside does not read.
    #[error(transparent)]
    Replay(#[from] ReadError),
    /// The replay holds fewer ticks than the shot names.
    #[error("the shot is at tick {tick} but its replay holds {held}")]
    Short {
        /// The shot's tick.
        tick: u64,
        /// What the replay holds.
        held: u64,
    },
}

/// A shot's setup, input stream and state hash.
#[derive(Debug, Clone, PartialEq)]
pub struct ShotRecord {
    /// The world tick the frame was drawn at: the race has run this many ticks.
    pub tick: u64,
    /// The race's state hash at [`Self::tick`].
    pub hash: u64,
    /// The run to that tick: [`Replay::ticks`] equals [`Self::tick`]. No ghost, no pose track.
    pub replay: Replay,
}

impl ShotRecord {
    /// The record of a run `replay` that stood at `hash` after `tick` ticks: the replay is cut
    /// to `tick` and stripped of what a re-simulation never reads.
    #[must_use]
    pub fn new(mut replay: Replay, tick: u64, hash: u64) -> Self {
        replay.truncate(tick);
        replay.ghost = None;
        replay.pose_track = None;
        replay.header.ghost = None;
        Self { tick, hash, replay }
    }

    /// The chunk's data.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![VERSION];
        out.extend(self.tick.to_le_bytes());
        out.extend(self.hash.to_le_bytes());
        let mut deflate = DeflateEncoder::new(out, Compression::best());
        deflate
            .write_all(&self.replay.to_bytes())
            .expect("deflating into memory cannot fail");
        deflate.finish().expect("deflating into memory cannot fail")
    }

    /// Reads a chunk [`Self::to_bytes`] wrote.
    ///
    /// # Errors
    ///
    /// [`ShotError`] for a frame this build does not read, a damaged stream, a replay that does
    /// not parse, or one shorter than the tick it claims.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ShotError> {
        let (&[version], rest) = bytes.split_first_chunk::<1>().ok_or(ShotError::Frame)?;
        let (tick, rest) = rest.split_first_chunk::<8>().ok_or(ShotError::Frame)?;
        let (hash, rest) = rest.split_first_chunk::<8>().ok_or(ShotError::Frame)?;
        if version != VERSION {
            return Err(ShotError::Frame);
        }
        let mut inflated = Vec::new();
        DeflateDecoder::new(rest)
            .read_to_end(&mut inflated)
            .map_err(ShotError::Inflate)?;
        let replay = Replay::from_bytes(&inflated)?;
        let tick = u64::from_le_bytes(*tick);
        let held = replay.ticks();
        if held < tick {
            return Err(ShotError::Short { tick, held });
        }
        Ok(Self {
            tick,
            hash: u64::from_le_bytes(*hash),
            replay,
        })
    }
}

#[cfg(test)]
mod tests;
