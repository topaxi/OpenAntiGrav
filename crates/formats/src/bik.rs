//! `BIK`: RAD Game Tools' Bink container, which is what Wipeout HD's video is.
//!
//! Thirty-seven files on the disc, from the studio logo reel down to the
//! animated mode icons in the front end. This module reads the **container
//! header** and nothing else: what a frame holds is Bink's own video codec, and
//! decoding it happens out of process exactly as `.PMF` and `.PSS` video does -
//! see `docs/formats/bik.md` for that route and
//! [ADR-0024](https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md)
//! for why it is not decoded here.
//!
//! ```text
//! +0x00  u8[3]   "BIK"
//! +0x03  u8      revision; 'i' on all 37 shipped files
//! +0x04  u32le   file length minus 8
//! +0x08  u32le   frame count
//! +0x0c  u32le   largest frame, in bytes
//! +0x10  u32le   frame count, again
//! +0x14  u32le   width
//! +0x18  u32le   height
//! +0x1c  u32le   frame rate numerator
//! +0x20  u32le   frame rate denominator
//! +0x24  u32le   video flags
//! +0x28  u32le   audio track count, `n`
//! +0x2c  u32[n]  largest decoded audio frame, per track
//!        u32[n]  audio flags, per track: sample rate in the low half
//!        u32[n]  track id, per track
//!        u32[f+1] offset of each frame, and the end; bit 0 is a keyframe flag
//! ```
//!
//! # It is little-endian on a big-endian console
//!
//! **This is the one thing about the file a reader of this repository will
//! expect to be wrong.** Every other HD payload is Pulse's format
//! byte-swapped - `XXEV` against `VEXX`, `WOtd` against `dtOW` - and
//! [`crate::byte_order`] exists to carry that. A `.bik` is not: `+0x14` reads
//! `80 07 00 00` and the picture really is 1920 pixels wide, not 0x80070000.
//! The container is the authoring tool's, written once on a PC and shipped
//! unchanged, so this module takes no [`crate::ByteOrder`] and never will.
//!
//! # What is measured
//!
//! All 37 `.bik` entries of `hdfury-ps3-eu-dec.iso`, three invariants each and
//! **37 of 37 on every one**: the length at `+0x04` plus 8 is the entry's real
//! size, the count at `+0x10` repeats the count at `+0x08`, and the first entry
//! of the offset table - with its keyframe bit masked off - lands exactly on the
//! end of the header this layout computes. The third is the load-bearing one:
//! it can only agree if the audio-track arrays were sized and ordered right, so
//! it validates the variable-length middle of the header rather than its ends.
//! Confidence **92**; see `docs/formats/bik.md`.

/// The three bytes every Bink 1 file starts with, before its revision byte.
///
/// Bink 2 spells its magic `KB2` and is a different container; no file on any
/// disc this project reads is one, and [`parse`] rejects it as not-Bink rather
/// than pretending the layout above applies.
pub const MAGIC: [u8; 3] = *b"BIK";

/// The revision byte all 37 shipped files carry.
///
/// Recorded rather than required: [`parse`] accepts any revision, because the
/// header layout above is common to the whole of Bink 1 and a file this project
/// has not seen is better read than refused.
pub const SHIPPED_REVISION: u8 = b'i';

/// Bytes of fixed header before the per-track arrays.
pub const FIXED_HEADER_LEN: usize = 44;

/// Audio flag: samples are 16-bit rather than 8-bit.
pub const AUD_16BITS: u16 = 0x4000;

/// Audio flag: two channels rather than one.
pub const AUD_STEREO: u16 = 0x2000;

/// Audio flag: the DCT-based codec rather than the RDFT one.
pub const AUD_USEDCT: u16 = 0x1000;

/// Something wrong with a Bink blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the fixed header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The blob does not start with `BIK`.
    NotBink,
    /// A zero width or height, which no decoder can use.
    ZeroDimension {
        /// Declared width.
        width: u32,
        /// Declared height.
        height: u32,
    },
    /// A zero frame-rate denominator, which no player can pace against.
    ZeroFrameRate,
    /// The header's own arrays run past the end of the blob.
    ///
    /// Reported for a truncated file and for a track count so large the arrays
    /// could not fit whatever the file held.
    HeaderTruncated {
        /// Bytes the header layout implies.
        want: usize,
        /// Bytes supplied.
        got: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => {
                write!(f, "{got} byte(s) is shorter than Bink's {FIXED_HEADER_LEN}")
            }
            Self::NotBink => write!(f, "does not start with BIK"),
            Self::ZeroDimension { width, height } => {
                write!(f, "declares a {width}x{height} picture")
            }
            Self::ZeroFrameRate => write!(f, "declares a frame rate with a zero denominator"),
            Self::HeaderTruncated { want, got } => {
                write!(f, "declares a {want}-byte header in {got} byte(s)")
            }
        }
    }
}

impl std::error::Error for Error {}

/// One of a Bink file's audio tracks.
///
/// **Six of the disc's 37 files have any**: the four Zone circuit previews
/// carry one each, and the two studio logo reels carry four each. The other 31
/// are silent by construction, which is why a caller must treat an empty
/// [`Header::audio`] as ordinary rather than as a failure to read one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioTrack {
    /// The track's own id, as the file numbers it. `0..n` on every shipped file.
    pub id: u32,
    /// Sampling rate in Hz - the low half of the flags word. 48,000 throughout.
    pub sample_rate: u32,
    /// The high half of the flags word; see [`AUD_STEREO`] and friends.
    pub flags: u16,
    /// The largest this track decodes a single frame to, in bytes.
    pub max_decoded_len: usize,
}

impl AudioTrack {
    /// How many channels the track carries.
    #[must_use]
    pub fn channels(&self) -> u16 {
        if self.flags & AUD_STEREO != 0 { 2 } else { 1 }
    }

    /// Whether the track is the DCT codec rather than the RDFT one.
    ///
    /// Named because the two are different decoders with the same container
    /// framing, so a caller that hands the file to something else needs to be
    /// able to say which it is. All six shipped tracks are DCT.
    #[must_use]
    pub fn is_dct(&self) -> bool {
        self.flags & AUD_USEDCT != 0
    }
}

/// What a Bink file's header declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The byte after `BIK`; see [`SHIPPED_REVISION`].
    pub revision: u8,
    /// The file's own length, which is the field at `+0x04` plus the eight
    /// bytes that field does not count.
    ///
    /// Checkable against the blob, and [`parse`] does not check it: a caller
    /// holding a short read of a long file has a useful header and no file
    /// length, and refusing it would make the two inseparable. See
    /// [`Header::declares_length_of`].
    pub declared_len: u64,
    /// How many frames the file holds.
    pub frame_count: usize,
    /// The largest single frame, in bytes.
    pub largest_frame: usize,
    /// Picture width in pixels.
    pub width: u32,
    /// Picture height in pixels.
    pub height: u32,
    /// Presentation rate as the file states it, numerator first.
    ///
    /// Left as the file's own fraction rather than reduced or turned into a
    /// float: the logo reel's `10000000/166833` is 59.94 Hz and the icons'
    /// `60/1` is exactly 60, and a player pacing against a rounded number
    /// accumulates the difference over the reel.
    pub frame_rate: (u32, u32),
    /// The word at `+0x24`. Zero on all 37 shipped files.
    pub video_flags: u32,
    /// One entry per audio track, in the file's own order. Usually empty.
    pub audio: Vec<AudioTrack>,
    /// Bytes of header, through the end of the frame offset table.
    ///
    /// Which is also where the first frame starts, and checking that those two
    /// agree is what validates the variable-length middle - see [`parse`].
    pub header_len: usize,
}

impl Header {
    /// Whether the length at `+0x04` describes a blob of `len` bytes.
    ///
    /// Separate from [`parse`] so that a caller reading a header out of a short
    /// prefix - a survey over a whole archive, say - is not forced to read every
    /// file whole to get one.
    #[must_use]
    pub fn declares_length_of(&self, len: usize) -> bool {
        self.declared_len == len as u64
    }

    /// The reel's running time in seconds, from its own count and rate.
    #[must_use]
    pub fn seconds(&self) -> f64 {
        f64::from(self.frame_rate.1) * self.frame_count as f64 / f64::from(self.frame_rate.0)
    }
}

fn u32_at(blob: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        blob[offset],
        blob[offset + 1],
        blob[offset + 2],
        blob[offset + 3],
    ])
}

/// Whether `blob` starts like a Bink file.
///
/// The dispatch test, and it is deliberately three bytes rather than four: the
/// revision is part of the magic in every other reader's telling, and treating
/// it as such would make a `BIKb` file "not a movie container this build
/// recognises" when the only thing this project does with either is hand it to
/// the same decoder.
#[must_use]
pub fn is_bink(blob: &[u8]) -> bool {
    blob.len() >= 4 && blob[..3] == MAGIC
}

/// Reads a Bink file's header.
///
/// The frame offset table is **validated and not returned**: its first entry is
/// checked against the header length this layout computes, which is what proves
/// the audio arrays were sized right, and nothing in this project seeks a Bink
/// file by frame. See `docs/formats/bik.md`.
///
/// # Errors
///
/// When the blob is not Bink, when it declares a picture or a rate nothing can
/// use, or when its own header does not fit in the bytes supplied.
pub fn parse(blob: &[u8]) -> Result<Header, Error> {
    if blob.len() < FIXED_HEADER_LEN {
        return Err(Error::TooShort { got: blob.len() });
    }
    if !is_bink(blob) {
        return Err(Error::NotBink);
    }

    let width = u32_at(blob, 0x14);
    let height = u32_at(blob, 0x18);
    if width == 0 || height == 0 {
        return Err(Error::ZeroDimension { width, height });
    }

    let frame_rate = (u32_at(blob, 0x1c), u32_at(blob, 0x20));
    if frame_rate.1 == 0 {
        return Err(Error::ZeroFrameRate);
    }

    let frame_count = u32_at(blob, 0x08) as usize;
    let track_count = u32_at(blob, 0x28) as usize;

    // Computed before anything reads at these offsets, and in `usize` with
    // checked arithmetic throughout: a track count of 0xffffffff off a corrupt
    // or misidentified blob would otherwise index past the end.
    let truncated = |want: usize| Error::HeaderTruncated {
        want,
        got: blob.len(),
    };
    let arrays = track_count.checked_mul(12).ok_or(truncated(usize::MAX))?;
    let offsets = frame_count
        .checked_add(1)
        .and_then(|n| n.checked_mul(4))
        .ok_or(truncated(usize::MAX))?;
    let header_len = FIXED_HEADER_LEN
        .checked_add(arrays)
        .and_then(|n| n.checked_add(offsets))
        .ok_or(truncated(usize::MAX))?;
    if blob.len() < header_len {
        return Err(truncated(header_len));
    }

    // Three parallel arrays rather than a table of structs, which is the file's
    // shape and not a choice here: every track's maximum decoded length comes
    // first, then every track's flags, then every track's id.
    let flags_at = FIXED_HEADER_LEN + 4 * track_count;
    let ids_at = flags_at + 4 * track_count;
    let audio = (0..track_count)
        .map(|i| {
            // Read as two `u16`s rather than one `u32` masked and shifted,
            // because that is what the field is: a rate in the low half and a
            // flag word in the high one, little-endian like everything else
            // here.
            let at = flags_at + 4 * i;
            AudioTrack {
                id: u32_at(blob, ids_at + 4 * i),
                sample_rate: u32::from(u16::from_le_bytes([blob[at], blob[at + 1]])),
                flags: u16::from_le_bytes([blob[at + 2], blob[at + 3]]),
                max_decoded_len: u32_at(blob, FIXED_HEADER_LEN + 4 * i) as usize,
            }
        })
        .collect();

    Ok(Header {
        revision: blob[3],
        declared_len: u64::from(u32_at(blob, 0x04)) + 8,
        frame_count,
        largest_frame: u32_at(blob, 0x0c) as usize,
        width,
        height,
        frame_rate,
        video_flags: u32_at(blob, 0x24),
        audio,
        header_len,
    })
}

/// Where the file says its first frame begins, with the keyframe bit masked off.
///
/// The check [`parse`] documents but cannot make part of its own result: a
/// header that agrees with this has had its audio arrays read at the right
/// width, and one that does not has not. Kept separate so the disagreement is
/// reportable rather than fatal - a file this project has not seen is worth
/// looking at, not refusing.
///
/// `None` when the blob is too short to hold the offset table at all.
#[must_use]
pub fn first_frame_offset(blob: &[u8], header: &Header) -> Option<usize> {
    let at = FIXED_HEADER_LEN + 12 * header.audio.len();
    if blob.len() < at + 4 {
        return None;
    }
    Some(u32_at(blob, at) as usize & !1)
}

#[cfg(test)]
mod tests;
