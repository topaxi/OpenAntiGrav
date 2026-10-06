//! Audiokinetic Wwise sound banks: `BKHD`, `DIDX`/`DATA` and `HIRC`.
//!
//! **Not the PSP's `.bnk`.** [`sblk`](crate::sblk) is Studio Liverpool's own
//! bank; its reader answers a Wwise file with "version 1145588546, expected 3"
//! (the ASCII `BKHD` as a `u32`). Omega Collection (PS4) is built on Wwise:
//! its `Data/audio/sound/*.bnk` are **bank generator version 118** on all 107
//! banks in its base and patch archives (`data00` 53, `data05` 1, `data08`
//! 53). The ten `.bnk` files in `data02` (all under `environments/`, six
//! little-endian and four byte-swapped) are **not** Wwise: they open with the
//! `SBlk` container's `version 3, 2 sections`, and [`Bank::parse`] refuses
//! them with [`Error::NoHeader`].
//!
//! # The container
//!
//! A bank is a run of chunks, each `{ char[4] tag, u32 size, body }`,
//! little-endian, tiling the file exactly (107 of 107 do):
//!
//! ```text
//! BKHD  the header             every bank, always first
//! INIT  plugin list            Init.bnk only
//! STMG  global settings        Init.bnk only
//! ENVS  environment settings   Init.bnk only
//! PLAT  platform name          Init.bnk only
//! DIDX  media index            33 of 53 base banks
//! DATA  the media it indexes   with DIDX, never one without the other
//! HIRC  the object hierarchy   37 of 53 base banks
//! ```
//!
//! The 16 base banks with a `BKHD` and nothing else (`weapons`, `shipHD`, every
//! `speech_*` but two) are 32 bytes: the names exist, the content is elsewhere,
//! not established; see [`Library`], which counts the references leading nowhere.
//!
//! ## `BKHD`
//!
//! ```text
//! +0x00  u32  bank generator version   118
//! +0x04  u32  bank id                  the FNV-1 hash of the bank's name
//! +0x08  u32  language id              0 for SFX, a hash for a voice bank
//! +0x0c  u16  alignment
//! +0x0e  u16  device allocated
//! +0x10  u32  project id
//! +0x14  ...  zero padding to the chunk's size (0 to 232 bytes)
//! ```
//!
//! The padding is zero in all 107 banks, so the fields above are the whole
//! header in this version.
//!
//! ## `DIDX` and `DATA`
//!
//! `DIDX` is 12-byte `{ u32 media id, u32 offset, u32 size }` records; `offset`
//! is into the `DATA` body. Every range lies inside `DATA` (all banks), holding
//! a complete RIFF/WAVE `.wem` from its first byte. A `.wem` is the same file
//! embedded or loose as `<media id>.wem`; the bank chooses per sound, see
//! [`hirc::StreamType`].
//!
//! ## `HIRC`
//!
//! `u32 count`, then `count` objects `{ u8 type, u32 size, u32 id, body }`
//! where `size` counts the `id` and the body. They tile the chunk and the count
//! is the header's (75 of 75 banks that have one: 37 in `data00`, 1 in `data05`,
//! 37 in `data08`). Types seen: 1 settings, 2 sound, 3 action, 4 event, 5
//! random/sequence container, 6 switch container, 7 actor-mixer, 8 bus, 9
//! layer container, 10 music segment, 11 music track, 12 music switch, 13
//! music random/sequence, 14 attenuation, 18-22 effects and modulators.
//! [`Bank::objects`] indexes every one; [`hirc`] reads the ones the sound path
//! needs.
//!
//! An **event** is a list of action ids, an **action** names a target object
//! (`Play` is `0x0403`), a **sound** names a media id and a codec, and a
//! container or actor-mixer is a node with a parent. An event resolves to media
//! by `event -> action -> target -> descendants`, [`Library::resolve_event`].
//! Nothing here is a *name*: ids are hashes and the banks carry no event names.
//!
//! # What is and is not read
//!
//! Read and checked against every bank: the chunk walk, `BKHD`, `DIDX`/`DATA`,
//! the `HIRC` framing, events, `Play` actions, sounds, music tracks' source
//! lists and the **parent id** at the head of each node's base parameters.
//! **Not read:** the rest of the base parameters (positioning, auxiliary sends,
//! RTPCs) and a sound container's playlist. **Music is read** in [`music`],
//! field by field, refusing by name what was not measured. `STID` (bank names)
//! is absent from every bank here. Evidence and counts: `docs/formats/wwise.md`.

use std::ops::Range;

pub mod hirc;
mod library;
pub mod music;
#[cfg(test)]
mod tests;
pub mod wem;

pub use hirc::{Kind, Object};
pub use library::{EventPlan, Library, MediaRef, SegmentChain, SongChain, TrackChain, WalkError};

/// The bank generator version every bank here carries, the only one the layouts
/// were measured against.
pub const BANK_VERSION: u32 = 118;

/// Wwise's name hash: FNV-1 (32-bit) over the lower-cased name. It is the id of
/// an event, state group, state or switch; the banks carry only the result.
/// Checked against every name `Music.txt` lists.
#[must_use]
pub fn name_hash(name: &str) -> u32 {
    name.bytes().fold(2_166_136_261u32, |hash, byte| {
        hash.wrapping_mul(16_777_619) ^ u32::from(byte.to_ascii_lowercase())
    })
}

const CHUNK_HEADER: usize = 8;

const HEADER_FIELDS: usize = 20;

const MEDIA_RECORD: usize = 12;

/// Why a bank could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A chunk header or body runs past the end of the file, at this offset.
    Truncated { at: usize },
    /// The first chunk is not `BKHD`.
    NoHeader,
    /// The `BKHD` chunk is shorter than its fields.
    ShortHeader { size: usize },
    /// A bank generator version other than [`BANK_VERSION`].
    UnsupportedVersion { version: u32 },
    /// A `DIDX` chunk whose size is not a whole number of records.
    BadMediaIndex { size: usize },
    /// A `HIRC` chunk whose objects do not tile it or whose count is wrong.
    BadHierarchy { at: usize },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated { at } => write!(f, "a chunk runs past the end of the file at {at:#x}"),
            Self::NoHeader => write!(f, "the first chunk is not BKHD"),
            Self::ShortHeader { size } => {
                write!(f, "a BKHD of {size} bytes cannot hold its fields")
            }
            Self::UnsupportedVersion { version } => write!(
                f,
                "bank generator version {version}, not {BANK_VERSION}; the layouts here were \
                 measured on {BANK_VERSION} only"
            ),
            Self::BadMediaIndex { size } => {
                write!(f, "a DIDX of {size} bytes is not whole 12-byte records")
            }
            Self::BadHierarchy { at } => {
                write!(f, "the HIRC objects do not tile the chunk at {at:#x}")
            }
        }
    }
}

impl std::error::Error for Error {}

/// One chunk of a bank: its tag and where its body is in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub tag: [u8; 4],
    pub body: Range<usize>,
}

/// The `BKHD` fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// The bank generator version, [`BANK_VERSION`].
    pub version: u32,
    /// The bank's id: the FNV-1 hash of its lower-case name.
    pub id: u32,
    /// `0` for a language-neutral bank.
    pub language_id: u32,
    pub alignment: u16,
    pub device_allocated: u16,
    pub project_id: u32,
    /// Bytes of padding after the fields; all zero in every shipped bank.
    pub padding: usize,
}

/// One `DIDX` record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Media {
    /// The media id - also the file name of the loose `.wem`, in decimal.
    pub id: u32,
    /// Where the media starts in the `DATA` chunk's body.
    pub offset: u32,
    pub size: u32,
}

/// A parsed bank, borrowing the file it came from.
#[derive(Debug, Clone)]
pub struct Bank<'a> {
    data: &'a [u8],
    header: Header,
    chunks: Vec<Chunk>,
    media: Vec<Media>,
    media_data: Option<Range<usize>>,
    objects: Vec<Object>,
    /// `(id, index into objects)`, sorted by id then index.
    by_id: Vec<(u32, usize)>,
}

impl<'a> Bank<'a> {
    /// Reads a whole `.bnk`.
    ///
    /// # Errors
    ///
    /// [`Error`] when the chunks do not tile the file, the first is not a
    /// `BKHD` of a supported version, or a `DIDX` or `HIRC` chunk is malformed.
    pub fn parse(data: &'a [u8]) -> Result<Self, Error> {
        // Checked before the walk: another container (the `SBlk` banks in
        // `data02`) would otherwise fail somewhere arbitrary as chunks.
        if data.get(..4) != Some(b"BKHD") {
            return Err(Error::NoHeader);
        }
        let mut chunks = Vec::new();
        let mut at = 0;
        while at < data.len() {
            let header_end = at + CHUNK_HEADER;
            if header_end > data.len() {
                return Err(Error::Truncated { at });
            }
            let tag: [u8; 4] = data[at..at + 4].try_into().expect("four bytes");
            let size = u32::from_le_bytes(data[at + 4..at + 8].try_into().expect("four bytes"));
            let end = header_end
                .checked_add(size as usize)
                .filter(|&end| end <= data.len())
                .ok_or(Error::Truncated { at })?;
            chunks.push(Chunk {
                tag,
                body: header_end..end,
            });
            at = end;
        }
        let first = chunks
            .first()
            .filter(|c| &c.tag == b"BKHD")
            .ok_or(Error::NoHeader)?;
        let body = &data[first.body.clone()];
        if body.len() < HEADER_FIELDS {
            return Err(Error::ShortHeader { size: body.len() });
        }
        let word = |at: usize| u32::from_le_bytes(body[at..at + 4].try_into().expect("four bytes"));
        let half = |at: usize| u16::from_le_bytes(body[at..at + 2].try_into().expect("two bytes"));
        let header = Header {
            version: word(0),
            id: word(4),
            language_id: word(8),
            alignment: half(12),
            device_allocated: half(14),
            project_id: word(16),
            padding: body.len() - HEADER_FIELDS,
        };
        if header.version != BANK_VERSION {
            return Err(Error::UnsupportedVersion {
                version: header.version,
            });
        }
        let mut media = Vec::new();
        if let Some(didx) = chunks.iter().find(|c| &c.tag == b"DIDX") {
            let body = &data[didx.body.clone()];
            if !body.len().is_multiple_of(MEDIA_RECORD) {
                return Err(Error::BadMediaIndex { size: body.len() });
            }
            for record in body.as_chunks::<MEDIA_RECORD>().0 {
                let word = |at: usize| {
                    u32::from_le_bytes(record[at..at + 4].try_into().expect("four bytes"))
                };
                media.push(Media {
                    id: word(0),
                    offset: word(4),
                    size: word(8),
                });
            }
        }
        let media_data = chunks
            .iter()
            .find(|c| &c.tag == b"DATA")
            .map(|c| c.body.clone());
        let objects = match chunks.iter().find(|c| &c.tag == b"HIRC") {
            Some(hirc) => hirc::index(data, hirc.body.clone())?,
            None => Vec::new(),
        };
        let mut by_id: Vec<(u32, usize)> = objects
            .iter()
            .enumerate()
            .map(|(index, object)| (object.id, index))
            .collect();
        by_id.sort_unstable();
        Ok(Self {
            data,
            header,
            chunks,
            media,
            media_data,
            objects,
            by_id,
        })
    }

    /// The `BKHD` fields.
    #[must_use]
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Every chunk, in file order.
    #[must_use]
    pub fn chunks(&self) -> &[Chunk] {
        &self.chunks
    }

    /// The `DIDX` records, in file order; empty when the bank has none.
    #[must_use]
    pub fn media(&self) -> &[Media] {
        &self.media
    }

    /// The bytes of the media a `DIDX` record names, or `None` when this bank
    /// has no such record or the record's range is outside `DATA`.
    #[must_use]
    pub fn embedded(&self, id: u32) -> Option<&'a [u8]> {
        let record = self.media.iter().find(|m| m.id == id)?;
        let data = self.media_data.as_ref()?;
        let start = data.start.checked_add(record.offset as usize)?;
        let end = start.checked_add(record.size as usize)?;
        (end <= data.end).then(|| &self.data[start..end])
    }

    /// Every `HIRC` object, in file order; empty when the bank has none.
    #[must_use]
    pub fn objects(&self) -> &[Object] {
        &self.objects
    }

    /// The objects with this id: usually one.
    pub fn objects_with_id(&self, id: u32) -> impl Iterator<Item = &Object> + '_ {
        let first = self.by_id.partition_point(|&(i, _)| i < id);
        self.by_id[first..]
            .iter()
            .take_while(move |&&(i, _)| i == id)
            .map(|&(_, index)| &self.objects[index])
    }

    /// The first object with this id.
    #[must_use]
    pub fn object(&self, id: u32) -> Option<&Object> {
        self.objects_with_id(id).next()
    }

    /// An object's body: the bytes after its id.
    #[must_use]
    pub fn body(&self, object: &Object) -> &'a [u8] {
        &self.data[object.body.clone()]
    }
}
