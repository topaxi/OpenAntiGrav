//! `.wem`: the RIFF/WAVE a Wwise bank's sources are stored as.
//!
//! A `.wem` is one file whether it sits in a bank's `DATA` chunk or loose as
//! `<media id>.wem`, and on the Omega Collection **every one of the 1,448 loose
//! files (`data00` 714, `data08` 734) and the embedded media of every bank is a
//! RIFF/WAVE with the chunks `fmt `, `JUNK`, `data` in that order**. `JUNK` is
//! Wwise's alignment padding and is zeros.
//!
//! # The codec is ATRAC9
//!
//! `fmt ` is 36 bytes, `WAVEFORMATEX` plus 18 bytes of extra data, and its
//! `wFormatTag` is **`0xFFFC`** on every ATRAC9 file - Wwise's own tag, not a
//! registered one. Four independent statements agree that it is Sony's ATRAC9
//! and not something Wwise-specific:
//!
//! 1. **The bank says so.** A sound's `HIRC` source carries a plugin id whose
//!    class is *codec* and whose number is 12, on all 6,858 (`data00`) and
//!    7,510 (`data08`) sounds whose media has tag `0xFFFC`. Wwise's codec
//!    numbering has `AKCODECID_ATRAC9` at 12 (recalled from the SDK's header
//!    rather than found on the disc; the three statements below do not depend
//!    on it).
//! 2. **The extra data contains a valid ATRAC9 configuration word.** Bytes
//!    6-9 of it are four bytes that open `0xFE` - ATRAC9's sync byte - then a
//!    4-bit sample-rate index (7, which is 48 kHz), a 3-bit channel-config
//!    index, a validation bit (0), an 11-bit `frame bytes - 1` and a 2-bit
//!    superframe index. Over the 400 files sampled: sync `0xFE` 400 of 400;
//!    rate index 7 and `fmt` rate 48,000, 400; validation bit clear, 400;
//!    **`frame bytes` equals `nBlockAlign`, 400**; superframe index 0 (one
//!    frame per block) 400; and the channel-config index tracks the channel
//!    count - index 2 is 2 channels (371 files), 5 is 4 (24), 4 is 8 (5) -
//!    which is LibAtrac9's own table. Four unrelated fields agreeing with
//!    `fmt` is not a coincidence a different codec would produce.
//! 3. **Two independent ATRAC9 decoders accept every frame and agree.**
//!    FFmpeg's decoder decodes each of the 60 files tried with `-xerror`,
//!    two, four and eight channels, with no error, where random bytes in the
//!    same container stop at "Invalid scalefactor coding mode!". The pure-Rust
//!    port of LibAtrac9 (`atrac9dec`) and FFmpeg produce the same PCM to within
//!    one least-significant bit on a stereo voice clip (99.75% of samples
//!    identical), **up to polarity**: one is the negation of the other.
//! 4. **The frame size is ATRAC9's.** 256 samples a frame at 48 kHz, and the
//!    file's sample count is `frames * 256 - delay - pad` with `0 <= pad <
//!    256` on all 400.
//!
//! # The extra data
//!
//! ```text
//! +0x00  u16  samples per frame         256 at 48 kHz, 128 at 24 kHz
//! +0x02  u32  channel config            channels | 1 << 8 | mask << 12
//! +0x06  u8[4]  ATRAC9 config word      see above; read big-endian
//! +0x0a  u32  samples                   after the delay, before the pad
//! +0x0e  u32  delay                     one frame's worth of samples: skip them
//! ```
//!
//! The channel config is Wwise's `AkChannelConfig`: the low byte is the
//! channel count, bits 8-11 the config type (1, standard) and the rest the
//! speaker mask - `0x3` for stereo, `0x603` for four channels (front and
//! side), `0x63f` for eight (7.1).
//!
//! # Not the RIFF size
//!
//! The RIFF size field is **not** the file's length minus eight: it is short by
//! 28 bytes on a stereo file, 92 on four channels and 236 on eight. A reader
//! that trusts it clips the last chunk, so this one walks the chunks and never
//! consults it.

use std::ops::Range;

use super::hirc::Codec;

/// Wwise's `wFormatTag` for ATRAC9.
pub const TAG_ATRAC9: u16 = 0xFFFC;

/// The tag Wwise writes for PCM: `WAVE_FORMAT_EXTENSIBLE`, not the plain `0x0001`.
/// The ten PCM sounds on the disc are mono, 16-bit, at 32 or 44.1 kHz, `fmt ` of
/// 24 bytes with six bytes of extra data (a zero `u16` and the channel config).
pub const TAG_PCM: u16 = 0xFFFE;

/// Why a `.wem` could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not a RIFF/WAVE.
    NotWave,
    /// A chunk runs past the end of the file.
    Truncated { at: usize },
    /// No `fmt ` chunk, or one shorter than `WAVEFORMATEX`.
    NoFormat,
    /// No `data` chunk.
    NoData,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotWave => write!(f, "not a RIFF/WAVE"),
            Self::Truncated { at } => write!(f, "a chunk runs past the end of the file at {at:#x}"),
            Self::NoFormat => write!(f, "no usable fmt chunk"),
            Self::NoData => write!(f, "no data chunk"),
        }
    }
}

impl std::error::Error for Error {}

/// The ATRAC9 extra data of a `fmt ` chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Atrac9 {
    /// Samples a frame decodes to: 256 at 48 kHz, 128 at 24 kHz.
    pub frame_samples: u16,
    /// Wwise's `AkChannelConfig`.
    pub channel_config: u32,
    /// The four-byte ATRAC9 configuration word, in file order.
    pub config: [u8; 4],
    /// Samples after the delay, before the pad.
    pub samples: u32,
    /// Samples to skip at the start.
    pub delay: u32,
}

/// The `fmt ` chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    pub tag: u16,
    pub channels: u16,
    pub sample_rate: u32,
    pub avg_bytes_per_sec: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
    /// The extra data after `WAVEFORMATEX`.
    pub extra: Vec<u8>,
}

impl Format {
    /// The codec, when the tag is one this crate names.
    #[must_use]
    pub fn codec(&self) -> Option<Codec> {
        match self.tag {
            TAG_ATRAC9 => Some(Codec::Atrac9),
            TAG_PCM => Some(Codec::Pcm),
            _ => None,
        }
    }

    /// The ATRAC9 extra data, when this is ATRAC9 and the extra data is the
    /// 18 bytes every shipped file carries.
    #[must_use]
    pub fn atrac9(&self) -> Option<Atrac9> {
        if self.tag != TAG_ATRAC9 || self.extra.len() != 18 {
            return None;
        }
        let word =
            |at: usize| u32::from_le_bytes(self.extra[at..at + 4].try_into().expect("four bytes"));
        Some(Atrac9 {
            frame_samples: u16::from_le_bytes([self.extra[0], self.extra[1]]),
            channel_config: word(2),
            config: self.extra[6..10].try_into().expect("four bytes"),
            samples: word(10),
            delay: word(14),
        })
    }
}

/// A parsed `.wem`, borrowing the file.
#[derive(Debug, Clone)]
pub struct Wem<'a> {
    data: &'a [u8],
    format: Format,
    chunks: Vec<([u8; 4], Range<usize>)>,
    payload: Range<usize>,
}

impl<'a> Wem<'a> {
    /// Reads a `.wem`'s chunks. The RIFF size field is not consulted.
    ///
    /// # Errors
    ///
    /// [`Error`] when it is not RIFF/WAVE, a chunk overruns the file, or `fmt `
    /// or `data` is missing.
    pub fn parse(data: &'a [u8]) -> Result<Self, Error> {
        if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WAVE" {
            return Err(Error::NotWave);
        }
        let mut chunks = Vec::new();
        let mut at = 12;
        while at + 8 <= data.len() {
            let tag: [u8; 4] = data[at..at + 4].try_into().expect("four bytes");
            let size = u32::from_le_bytes(data[at + 4..at + 8].try_into().expect("four bytes"));
            let end = (at + 8)
                .checked_add(size as usize)
                .filter(|&end| end <= data.len())
                .ok_or(Error::Truncated { at })?;
            chunks.push((tag, at + 8..end));
            at = end + (size as usize & 1);
        }
        let fmt = chunks
            .iter()
            .find(|(t, _)| t == b"fmt ")
            .map(|(_, r)| &data[r.clone()])
            .filter(|f| f.len() >= 16)
            .ok_or(Error::NoFormat)?;
        let half = |at: usize| u16::from_le_bytes(fmt[at..at + 2].try_into().expect("two bytes"));
        let word = |at: usize| u32::from_le_bytes(fmt[at..at + 4].try_into().expect("four bytes"));
        let extra_len = if fmt.len() >= 18 {
            usize::from(half(16)).min(fmt.len() - 18)
        } else {
            0
        };
        let format = Format {
            tag: half(0),
            channels: half(2),
            sample_rate: word(4),
            avg_bytes_per_sec: word(8),
            block_align: half(12),
            bits_per_sample: half(14),
            extra: fmt.get(18..18 + extra_len).unwrap_or_default().to_vec(),
        };
        let payload = chunks
            .iter()
            .find(|(t, _)| t == b"data")
            .map(|(_, r)| r.clone())
            .ok_or(Error::NoData)?;
        Ok(Self {
            data,
            format,
            chunks,
            payload,
        })
    }

    /// The `fmt ` chunk.
    #[must_use]
    pub fn format(&self) -> &Format {
        &self.format
    }

    /// Chunk tags in file order.
    #[must_use]
    pub fn tags(&self) -> Vec<[u8; 4]> {
        self.chunks.iter().map(|(t, _)| *t).collect()
    }

    /// The `data` chunk's bytes: for ATRAC9, whole frames of `block_align`
    /// bytes each.
    #[must_use]
    pub fn payload(&self) -> &'a [u8] {
        &self.data[self.payload.clone()]
    }

    /// Whole frames in the payload, for a block-aligned codec.
    #[must_use]
    pub fn frames(&self) -> usize {
        match self.format.block_align {
            0 => 0,
            align => self.payload().len() / usize::from(align),
        }
    }
}
