//! The Vita's `.rcsanimclip`: keyframes for the nodes a `.rcsskeleton`
//! declares.
//!
//! Same [`container`](crate::rcsmodel::psp2::container) as the model and
//! the skeleton, one section:
//!
//! ```text
//! section, from its own start:
//!   +0x00  u32   2 - unread, constant on all 49 files
//!   +0x04  u32   bound node count N
//!   +0x08  u32   track count T, at most N
//!   +0x0c  u32   unread: 0 on 44 files, 0x30/0x22/0xc/0x37 on the rest
//!   +0x10  u32   offset of u32[N]: node ids, every one a skeleton node
//!   +0x14  u32   offset of track[T], 20 bytes each - track i animates
//!                node id[i]; ids T..N are bound with no keys
//!   +0x18  f32   duration in seconds, the longest track's on all 49
//!
//! one track:
//!   +0x00  u32   0
//!   +0x04  u32   slot count, 9
//!   +0x08  u32   offset of u32[9]: channel offsets by slot, 0 for a slot
//!                the track leaves at its bind value
//!   +0x0c  u32   0 (1 on one track of `data/StartAnim/model/start`)
//!   +0x10  f32   this track's own loop length in seconds
//!
//! one channel:
//!   +0x00  u32   0x10000 | slot
//!   +0x04  f32   duration, always the track's
//!   +0x08  u32   key count, always duration * rate to within one
//!   +0x0c  u32   (type << 16) | 1, type as the skeleton's property tags
//!   +0x10  u32   offset of the keys, packed by type: 16 bytes a
//!                quaternion, 12 a vec3, 4 a scalar, 1 a bool
//!   +0x14  f32   rate in keys per second: 5 on 6,554 of the corpus's
//!                6,986 channels, 30 on 428, 10 on 4
//!   +0x18  f32   seconds per key, 1 / rate
//! ```
//!
//! **A key is the node's whole local value at that instant, not a delta**,
//! and the keys are Wipeout HD's own animation resampled: on Anulpha Pass,
//! which 2048 re-ships from HD, every 5 Hz rotation key equals HD's
//! `Anim Transform` evaluated at the same frame - see
//! `docs/formats/2048-animation.md`, which is also where each field's
//! confidence lives. Evaluating one is [`crate::rig`]'s job.

use crate::rcsmodel::psp2::container::{self, u32_at};
use crate::rcsmodel::psp2::{Error, Result};
use crate::rcsskeleton::{Kind, SLOTS};

/// One channel's keys, evenly spaced from time zero.
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    /// What each key is.
    pub kind: Kind,
    /// The keys, flattened: `kind.key_len() / 4` floats per key, or one
    /// `0.0`/`1.0` per key for a [`Kind::Bool`].
    pub keys: Vec<f32>,
    /// Seconds between keys.
    pub seconds_per_key: f32,
    /// How many keys.
    pub count: usize,
}

impl Channel {
    /// Floats per key.
    #[must_use]
    pub fn width(&self) -> usize {
        match self.kind {
            Kind::Scalar | Kind::Bool => 1,
            Kind::Vec3 => 3,
            Kind::Quat => 4,
        }
    }

    /// Key `i`'s floats.
    #[must_use]
    pub fn key(&self, i: usize) -> &[f32] {
        let w = self.width();
        let i = i.min(self.count.saturating_sub(1));
        &self.keys[i * w..(i + 1) * w]
    }
}

/// One node's tracks: a channel per animated slot.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    /// The node id this track animates.
    pub id: u32,
    /// The track's own loop length, in seconds.
    pub duration: f32,
    /// A channel per slot, `None` where the node keeps its bind value.
    pub channels: [Option<Channel>; SLOTS],
}

/// A decoded `.rcsanimclip`.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    /// The longest track's duration.
    pub duration: f32,
    /// Every node id the clip binds, tracks first; ids past
    /// [`Self::tracks`]' length carry no keys.
    pub bound: Vec<u32>,
    /// One track per animated node, in file order.
    pub tracks: Vec<Track>,
}

impl Clip {
    /// The track animating node `id`, if any.
    #[must_use]
    pub fn track(&self, id: u32) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }
}

/// Decodes a `.rcsanimclip`.
///
/// # Errors
///
/// The container's own refusals, any offset that runs past the section, and
/// a channel whose type byte names a kind not seen on the disc.
pub fn parse(file: &[u8]) -> Result<Clip> {
    let header = container::read(file)?;
    let section = header.section(file, 0).ok_or(Error::OutOfBounds {
        what: "clip section",
        end: 0,
        len: file.len(),
    })?;
    let bound_count = u32_at(section, 0x04, "bound node count")? as usize;
    let track_count = u32_at(section, 0x08, "track count")? as usize;
    let ids_at = u32_at(section, 0x10, "node ids")? as usize;
    let tracks_at = u32_at(section, 0x14, "tracks")? as usize;
    let duration = f32::from_bits(u32_at(section, 0x18, "duration")?);
    if track_count > bound_count {
        return Err(Error::OutOfBounds {
            what: "tracks past the bound node list",
            end: track_count,
            len: bound_count,
        });
    }
    let bound = (0..bound_count)
        .map(|i| u32_at(section, ids_at + i * 4, "node id"))
        .collect::<Result<Vec<u32>>>()?;

    let mut tracks = Vec::with_capacity(track_count);
    for (i, &id) in bound.iter().enumerate().take(track_count) {
        let at = tracks_at + i * 20;
        let slots = u32_at(section, at + 0x04, "slot count")? as usize;
        let table = u32_at(section, at + 0x08, "channel table")? as usize;
        let track_duration = f32::from_bits(u32_at(section, at + 0x10, "track duration")?);
        let mut channels: [Option<Channel>; SLOTS] = Default::default();
        for (slot, out) in channels.iter_mut().enumerate().take(slots.min(SLOTS)) {
            let channel = u32_at(section, table + slot * 4, "channel")? as usize;
            if channel == 0 {
                continue;
            }
            *out = Some(read_channel(section, channel)?);
        }
        tracks.push(Track {
            id,
            duration: track_duration,
            channels,
        });
    }
    Ok(Clip {
        duration,
        bound,
        tracks,
    })
}

fn read_channel(section: &[u8], at: usize) -> Result<Channel> {
    let count = u32_at(section, at + 0x08, "key count")? as usize;
    let type_word = u32_at(section, at + 0x0c, "channel type")?;
    let keys_at = u32_at(section, at + 0x10, "keys")? as usize;
    let seconds_per_key = f32::from_bits(u32_at(section, at + 0x18, "seconds per key")?);
    let kind = Kind::from_type(((type_word >> 16) & 0xff) as u8).ok_or(Error::OutOfBounds {
        what: "channel type byte not seen on the disc",
        end: (type_word >> 16) as usize,
        len: 4,
    })?;
    let bytes = section
        .get(keys_at..keys_at + count * kind.key_len())
        .ok_or(Error::OutOfBounds {
            what: "channel keys",
            end: keys_at + count * kind.key_len(),
            len: section.len(),
        })?;
    let keys: Vec<f32> = match kind {
        Kind::Bool => bytes.iter().map(|&b| f32::from(u8::from(b != 0))).collect(),
        _ => bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect(),
    };
    Ok(Channel {
        kind,
        keys,
        seconds_per_key,
        count,
    })
}

#[cfg(test)]
pub(crate) mod tests;
