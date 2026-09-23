//! [`Replay`] and its file: a magic, a version, and tagged chunks.
//!
//! ```text
//! "OAGR"  u16 version  u16 reserved (0)
//! chunk*: [u8; 4] tag, u32 length, payload
//!   HEAD  the Header, as TOML text
//!   INPT  one per recorded slot: u8 slot, then the codec's input stream
//!   HASH  u64 initial hash, varint count, count x u64
//!   GHST  optional: the ghost lap's pose track
//!   END_  u64: FNV-1a over every byte before this chunk
//! ```
//!
//! All integers little-endian. **A chunk this version does not know is
//! skipped**, which is how a later version adds a section without making an
//! older build refuse the whole file. The trailing checksum is there for the
//! one failure a state hash cannot see: a file cut short by a crash while it
//! was being written, whose header and first minutes still parse. Writers
//! should still write to a temporary name and rename.

use oag_core::StateHasher;
use oag_gameplay::{InputSnapshot, PlayerInputs};

use crate::codec::{Cursor, Truncated, decode_inputs, encode_inputs, put_varint};
use crate::header::Header;
use crate::pose::GhostLap;

/// The version this build writes, and the only one it reads.
///
/// Bumped when a chunk this version *does* know changes meaning. Adding a
/// chunk or a `#[serde(default)]` header field is not a bump.
pub const FORMAT_VERSION: u16 = 1;

const MAGIC: &[u8; 4] = b"OAGR";

/// Why a file could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// Not a replay at all.
    #[error("not a replay file (bad magic)")]
    BadMagic,
    /// A replay from a format this build does not read.
    #[error("replay format version {0}, this build reads {FORMAT_VERSION}")]
    Version(u16),
    /// Cut short or corrupted.
    #[error(transparent)]
    Truncated(#[from] Truncated),
    /// The checksum does not match: a partial write, or a changed byte.
    #[error("replay checksum mismatch - the file is damaged")]
    Checksum,
    /// The header is not valid TOML for a [`Header`].
    #[error("replay header: {0}")]
    Header(String),
    /// The chunks parse but disagree with each other.
    #[error("replay is inconsistent: {0}")]
    Inconsistent(String),
}

/// A recorded run: its header, its inputs, its hashes, and its ghost.
#[derive(Debug, Clone, PartialEq)]
pub struct Replay {
    /// What the run was of.
    pub header: Header,
    /// One stream per entry of [`Header::slots`], in that order, one snapshot
    /// per tick. All the same length.
    pub inputs: Vec<Vec<InputSnapshot>>,
    /// The race's state hash before the first tick.
    ///
    /// **Checked before anything is stepped**, so a replay handed the wrong
    /// track or the wrong class is refused at tick zero rather than a second
    /// in.
    pub initial_hash: u64,
    /// `hashes[i]` is the state hash after tick `(i + 1) * hash_interval`.
    pub hashes: Vec<u64>,
    /// The lap a ghost is drawn from, if this file carries one.
    pub ghost: Option<GhostLap>,
}

impl Replay {
    /// How many ticks the run recorded.
    #[must_use]
    pub fn ticks(&self) -> u64 {
        self.inputs.first().map_or(0, |stream| stream.len() as u64)
    }

    /// What the tick that advanced the world from `tick` to `tick + 1` was
    /// handed: the recorded slots' snapshots, and nothing in the rest.
    #[must_use]
    pub fn inputs_at(&self, tick: u64) -> PlayerInputs {
        let mut inputs = PlayerInputs::none();
        let Ok(index) = usize::try_from(tick) else {
            return inputs;
        };
        for (&slot, stream) in self.header.slots.iter().zip(&self.inputs) {
            if let Some(snapshot) = stream.get(index) {
                inputs.set(usize::from(slot), *snapshot);
            }
        }
        inputs
    }

    /// Drops everything after `ticks`: the inputs, and the hashes of ticks
    /// that are no longer recorded.
    ///
    /// What a ghost file is cut to - a Speed Lap session runs for as many laps
    /// as the player likes, and only the ones up to the end of the ghost's lap
    /// are needed to reproduce it.
    pub fn truncate(&mut self, ticks: u64) {
        let keep = usize::try_from(ticks).unwrap_or(usize::MAX);
        for stream in &mut self.inputs {
            stream.truncate(keep);
        }
        let interval = u64::from(self.header.hash_interval.max(1));
        let hashes = usize::try_from(ticks / interval).unwrap_or(usize::MAX);
        self.hashes.truncate(hashes);
    }

    /// The file's bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());

        let header = toml::to_string(&self.header).expect("a Header always serialises");
        chunk(&mut out, *b"HEAD", header.as_bytes());
        for (&slot, stream) in self.header.slots.iter().zip(&self.inputs) {
            let mut payload = vec![slot];
            payload.extend(encode_inputs(stream));
            chunk(&mut out, *b"INPT", &payload);
        }
        let mut hashes = Vec::with_capacity(16 + self.hashes.len() * 8);
        hashes.extend_from_slice(&self.initial_hash.to_le_bytes());
        put_varint(&mut hashes, self.hashes.len() as u64);
        for hash in &self.hashes {
            hashes.extend_from_slice(&hash.to_le_bytes());
        }
        chunk(&mut out, *b"HASH", &hashes);
        if let Some(ghost) = &self.ghost {
            chunk(&mut out, *b"GHST", &ghost.encode());
        }
        let checksum = checksum(&out);
        chunk(&mut out, *b"END_", &checksum.to_le_bytes());
        out
    }

    /// Reads a file [`Self::to_bytes`] wrote.
    ///
    /// # Errors
    ///
    /// [`ReadError`] for anything that is not a complete, self-consistent
    /// replay of [`FORMAT_VERSION`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ReadError> {
        let mut cursor = Cursor::new(bytes);
        if cursor.take(4).map_err(|_| ReadError::BadMagic)? != MAGIC {
            return Err(ReadError::BadMagic);
        }
        let version = cursor.u16()?;
        if version != FORMAT_VERSION {
            return Err(ReadError::Version(version));
        }
        let _reserved = cursor.u16()?;

        let mut header = None;
        let mut inputs = Vec::new();
        let mut hashes = None;
        let mut ghost = None;
        let mut ended = false;
        while !cursor.is_empty() {
            let start = cursor.position();
            let tag = cursor.take(4)?;
            let len = cursor.u32()? as usize;
            let payload = cursor.take(len)?;
            match tag {
                b"HEAD" => {
                    let text = std::str::from_utf8(payload)
                        .map_err(|e| ReadError::Header(e.to_string()))?;
                    header = Some(
                        toml::from_str::<Header>(text)
                            .map_err(|e| ReadError::Header(e.to_string()))?,
                    );
                }
                b"INPT" => {
                    let (&slot, stream) =
                        payload.split_first().ok_or(Truncated(cursor.position()))?;
                    inputs.push((slot, decode_inputs(stream)?));
                }
                b"HASH" => {
                    let mut chunk = Cursor::new(payload);
                    let initial = chunk.u64()?;
                    let count = usize::try_from(chunk.varint()?).map_err(|_| Truncated(start))?;
                    let list = (0..count.min(payload.len() / 8))
                        .map(|_| chunk.u64())
                        .collect::<Result<Vec<_>, _>>()?;
                    if list.len() != count || !chunk.is_empty() {
                        return Err(Truncated(start).into());
                    }
                    hashes = Some((initial, list));
                }
                b"GHST" => ghost = Some(GhostLap::decode(payload)?),
                b"END_" => {
                    let stored = Cursor::new(payload).u64()?;
                    if stored != checksum(&bytes[..start]) {
                        return Err(ReadError::Checksum);
                    }
                    ended = true;
                    break;
                }
                _ => {}
            }
        }
        if !ended || !cursor.is_empty() {
            return Err(ReadError::Checksum);
        }

        let header = header.ok_or_else(|| ReadError::Inconsistent("no header".into()))?;
        let (initial_hash, hashes) =
            hashes.ok_or_else(|| ReadError::Inconsistent("no hash track".into()))?;
        let mut streams = Vec::with_capacity(header.slots.len());
        for &slot in &header.slots {
            let stream = inputs
                .iter()
                .position(|(s, _)| *s == slot)
                .map(|at| inputs.swap_remove(at).1)
                .ok_or_else(|| {
                    ReadError::Inconsistent(format!("no input stream for slot {slot}"))
                })?;
            streams.push(stream);
        }
        let replay = Self {
            header,
            inputs: streams,
            initial_hash,
            hashes,
            ghost,
        };
        replay.check()?;
        Ok(replay)
    }

    /// The cross-chunk invariants: equal stream lengths, and no more hashes
    /// than the recorded ticks can have produced.
    fn check(&self) -> Result<(), ReadError> {
        let ticks = self.ticks();
        if self
            .inputs
            .iter()
            .any(|stream| stream.len() as u64 != ticks)
        {
            return Err(ReadError::Inconsistent(
                "input streams differ in length".into(),
            ));
        }
        if self.header.hash_interval == 0 {
            return Err(ReadError::Inconsistent("hash interval of zero".into()));
        }
        if self.hashes.len() as u64 > ticks / u64::from(self.header.hash_interval) {
            return Err(ReadError::Inconsistent(
                "more hashes than recorded ticks".into(),
            ));
        }
        Ok(())
    }
}

fn chunk(out: &mut Vec<u8>, tag: [u8; 4], payload: &[u8]) {
    out.extend_from_slice(&tag);
    let len = u32::try_from(payload.len()).expect("a replay chunk under 4 GiB");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(payload);
}

fn checksum(bytes: &[u8]) -> u64 {
    let mut hasher = StateHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

#[cfg(test)]
mod tests;
