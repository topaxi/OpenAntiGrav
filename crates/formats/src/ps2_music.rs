//! `PS2MUSIC.WAD`: a flat archive of uncompressed PCM music.
//!
//! ```text
//! +0x00  u32  entry_count
//! +0x04  entry[entry_count], 12 bytes each
//!
//! entry:
//!   +0x00  u32  name_hash    the same hash as [`crate::wad`]
//!   +0x04  u32  size
//!   +0x08  u32  offset       from the start of the file
//! ```
//!
//! See `docs/formats/ps2-audio.md` for the evidence.
//!
//! # This is not the WAD container
//!
//! Despite the extension. The ordinary [WAD](crate::wad) stores
//! `{hash, offset, size_uncompressed, size}` in 16 bytes; this stores
//! `{hash, size, offset}` in 12, with no compression field. Read as a WAD
//! directory, the first track lands at offset 36,018,740.
//!
//! # The payload is raw PCM
//!
//! Signed 16-bit little-endian, two channels interleaved left first, 48,000 Hz,
//! none of it stated in the file. The frame size follows from the entry sizes
//! (all divisible by 4, only 6 of 16 by 8); the rate and channel order from the
//! PSP disc, which carries the same sixteen tracks as ATRAC3plus: durations
//! match the PS2 byte counts at 48 kHz to within 10 ms each, and correlating
//! the discs' *side* signals (the one statistic that changes sign when channels
//! swap) gives +0.98. `docs/formats/ps2-audio.md` has both tables.
//! `docs/formats/ps2-audio.md` has both tables.

pub const HEADER_LEN: usize = 4;

pub const ENTRY_LEN: usize = 12;

pub const FRAME_LEN: usize = 4;

/// Sample rate of the PCM payload, in hertz.
///
/// Not read from the file; see the module docs.
pub const SAMPLE_RATE: u32 = 48_000;

/// Channels in the PCM payload.
pub const CHANNELS: u16 = 2;

/// An implausible entry count, used to reject a file that is not this format.
const MAX_ENTRIES: u32 = 4096;

/// Something wrong with a music archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    TooShort {
        got: usize,
    },
    /// The entry count is missing or implausible.
    BadEntryCount {
        count: u32,
    },
    /// An entry runs past the end of the archive.
    OutOfRange {
        index: usize,
        offset: u32,
        size: u32,
    },
    /// An entry's size is not a whole number of sample frames.
    PartialFrame {
        index: usize,
        size: u32,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "need at least {HEADER_LEN} bytes, got {got}"),
            Self::BadEntryCount { count } => write!(f, "implausible entry count {count}"),
            Self::OutOfRange {
                index,
                offset,
                size,
            } => write!(
                f,
                "entry {index} claims {size} bytes at {offset}, past the end"
            ),
            Self::PartialFrame { index, size } => write!(
                f,
                "entry {index} is {size} bytes, not a whole number of {FRAME_LEN}-byte frames"
            ),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// One track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// CRC-32 of the name, as [`crate::wad::hash_name`] computes it.
    pub name_hash: u32,
    /// Bytes of PCM.
    pub size: u32,
    /// Where they start, from the beginning of the archive.
    pub offset: u32,
}

impl Entry {
    /// Sample frames, one per instant across both channels.
    #[must_use]
    pub fn frames(&self) -> u32 {
        self.size / FRAME_LEN as u32
    }

    /// Playing time in seconds at [`SAMPLE_RATE`].
    #[must_use]
    pub fn seconds(&self) -> f32 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a duration for display; the exact frame count is `frames`"
        )]
        let frames = self.frames() as f32;
        frames / SAMPLE_RATE as f32
    }
}

/// The archive directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    pub entries: Vec<Entry>,
}

/// Bytes the directory occupies for `count` entries.
#[must_use]
pub fn directory_len(count: u32) -> u64 {
    HEADER_LEN as u64 + u64::from(count) * ENTRY_LEN as u64
}

/// Reads the entry count without reading the directory.
///
/// # Errors
///
/// Fails when `data` is shorter than the header or the count is implausible.
pub fn peek_entry_count(data: &[u8]) -> Result<u32> {
    if data.len() < HEADER_LEN {
        return Err(Error::TooShort { got: data.len() });
    }
    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if count == 0 || count > MAX_ENTRIES {
        return Err(Error::BadEntryCount { count });
    }
    Ok(count)
}

impl Directory {
    /// Parses a directory.
    ///
    /// `archive_len`, when known, bounds every entry, turning a plausible read
    /// into a checked one: entries chain end-to-end with no padding, so the last
    /// finishes exactly at the archive's length.
    ///
    /// # Errors
    ///
    /// Fails when the header is short, the count is implausible, an entry runs
    /// past `archive_len`, or an entry is not a whole number of sample frames.
    pub fn parse(data: &[u8], archive_len: Option<u64>) -> Result<Self> {
        let count = peek_entry_count(data)?;
        let needed = directory_len(count);
        if (data.len() as u64) < needed {
            return Err(Error::TooShort { got: data.len() });
        }

        let mut entries = Vec::with_capacity(count as usize);
        for index in 0..count as usize {
            let at = HEADER_LEN + index * ENTRY_LEN;
            let word = |i: usize| {
                let o = at + i * 4;
                u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]])
            };
            let entry = Entry {
                name_hash: word(0),
                size: word(1),
                offset: word(2),
            };
            if !(entry.size as usize).is_multiple_of(FRAME_LEN) {
                return Err(Error::PartialFrame {
                    index,
                    size: entry.size,
                });
            }
            if let Some(len) = archive_len
                && u64::from(entry.offset) + u64::from(entry.size) > len
            {
                return Err(Error::OutOfRange {
                    index,
                    offset: entry.offset,
                    size: entry.size,
                });
            }
            entries.push(entry);
        }
        Ok(Self { entries })
    }

    /// Whether the entries chain end to end with no gaps, from just after the
    /// directory to `archive_len`. Exact on the shipped archive; it says the
    /// second word is the size and the third the offset, not the reverse.
    #[must_use]
    pub fn chains_exactly(&self, archive_len: u64) -> bool {
        let mut cursor = directory_len(self.entries.len() as u32);
        for entry in &self.entries {
            if u64::from(entry.offset) != cursor {
                return false;
            }
            cursor += u64::from(entry.size);
        }
        cursor == archive_len
    }

    /// Total sample frames across every track.
    #[must_use]
    pub fn frames(&self) -> u64 {
        self.entries.iter().map(|e| u64::from(e.frames())).sum()
    }
}

/// Wraps raw PCM in a canonical WAV header, at this archive's own rate and
/// channel count. Anything else wants [`wav_with`].
#[must_use]
pub fn wav(pcm: &[u8]) -> Vec<u8> {
    wav_with(pcm, SAMPLE_RATE, CHANNELS)
}

/// Wraps raw 16-bit little-endian PCM in a canonical WAV header.
///
/// The payload is copied verbatim: it is already the sample format a WAV
/// `data` chunk wants. It lives here so the workspace has one WAV writer;
/// `oag-audio` uses it for `--dump-audio`.
#[must_use]
pub fn wav_with(pcm: &[u8], sample_rate: u32, channels: u16) -> Vec<u8> {
    let data_len = pcm.len() as u32;
    let byte_rate = sample_rate * u32::from(channels) * 2;
    let block_align = channels * 2;

    let mut out = Vec::with_capacity(44 + pcm.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an archive directory by hand. No game data in any test.
    fn directory(sizes: &[u32]) -> Vec<u8> {
        let count = sizes.len() as u32;
        let mut out = count.to_le_bytes().to_vec();
        let mut offset = directory_len(count) as u32;
        for (index, &size) in sizes.iter().enumerate() {
            out.extend_from_slice(&(0xdead_0000 + index as u32).to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&offset.to_le_bytes());
            offset += size;
        }
        out
    }

    #[test]
    fn a_directory_chains_end_to_end() {
        let sizes = [4u32, 400, 40];
        let data = directory(&sizes);
        let total = directory_len(3) + u64::from(sizes.iter().sum::<u32>());
        let dir = Directory::parse(&data, Some(total)).expect("parse");
        assert_eq!(dir.entries.len(), 3);
        assert_eq!(dir.entries[0].offset, 40);
        assert!(dir.chains_exactly(total));
        assert_eq!(dir.frames(), (4 + 400 + 40) / 4);
    }

    #[test]
    fn a_gap_in_the_chain_is_reported() {
        let mut data = directory(&[4u32, 400, 40]);
        // Push the last entry 4 bytes further out.
        let at = HEADER_LEN + 2 * ENTRY_LEN + 8;
        let offset = u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
        data[at..at + 4].copy_from_slice(&(offset + 4).to_le_bytes());
        let dir = Directory::parse(&data, Some(10_000)).expect("parse");
        assert!(!dir.chains_exactly(directory_len(3) + 444));
    }

    #[test]
    fn a_size_that_is_not_a_whole_frame_is_refused() {
        let data = directory(&[6u32]);
        assert_eq!(
            Directory::parse(&data, None),
            Err(Error::PartialFrame { index: 0, size: 6 })
        );
    }

    #[test]
    fn an_entry_past_the_end_is_refused() {
        let data = directory(&[400u32]);
        assert!(matches!(
            Directory::parse(&data, Some(64)),
            Err(Error::OutOfRange { index: 0, .. })
        ));
    }

    #[test]
    fn a_wad_directory_is_not_mistaken_for_this() {
        // The ordinary container's header is `{version = 1, entry_count}`, so a
        // count of 1 and a 16-byte stride reads here as one entry whose size is
        // the WAD's entry count. Bounding rejects that.
        let mut data = 1u32.to_le_bytes().to_vec();
        data.extend_from_slice(&7u32.to_le_bytes());
        data.extend_from_slice(&0x1234_5678u32.to_le_bytes());
        data.extend_from_slice(&0x8000_0000u32.to_le_bytes());
        assert!(Directory::parse(&data, Some(1024)).is_err());
    }

    #[test]
    fn seconds_uses_the_documented_rate() {
        let entry = Entry {
            name_hash: 0,
            size: SAMPLE_RATE * u32::from(CHANNELS) * 2,
            offset: 0,
        };
        assert!((entry.seconds() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_wav_header_describes_the_payload_it_wraps() {
        let pcm = vec![0u8; 400];
        let file = wav(&pcm);
        assert_eq!(&file[..4], b"RIFF");
        assert_eq!(&file[8..12], b"WAVE");
        assert_eq!(file.len(), 44 + pcm.len());
        assert_eq!(
            u32::from_le_bytes([file[4], file[5], file[6], file[7]]) as usize,
            file.len() - 8
        );
        assert_eq!(
            u32::from_le_bytes([file[40], file[41], file[42], file[43]]) as usize,
            pcm.len()
        );
    }
}
