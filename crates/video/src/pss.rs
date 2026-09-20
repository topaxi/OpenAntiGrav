//! `.PSS`: the PS2's loose MPEG-2 program stream movies - specifically the
//! audio carried in `private_stream_1` (MPEG stream id `0xbd`).
//!
//! The video elementary stream is handed to `ffmpeg` whole (see
//! `oag_game::movie::mpeg2_ps`, which demuxes nothing itself: `ffmpeg`'s own
//! `mpegps` demuxer reads a `.PSS` directly). This module exists only for the
//! audio, which `ffmpeg`'s probe never reports on this container - so nothing
//! here reads or copies out a single byte of the `0xe0` video stream, on
//! purpose: a boot must not pay to copy the ~19 MB of it a `.PSS` this size
//! carries just to reach the audio sitting beside it.
//!
//! # What `private_stream_1` carries
//!
//! Every `0xbd` PES payload on `DATA/MOVIES/INTRO640.PSS` and
//! `DATA/MOVIES/INTRO512.PSS` (measured on `pulse-ps2-eu.chd`) opens with the
//! same 4-byte sub-header - [`SUBSTREAM_PREFIX`], `ff a0 00 00` - a Sony
//! private-stream convention distinct from the generic PES header the pack
//! walk below has already stripped. The **first** packet's payload, past that
//! prefix, carries a small metadata block before the audio body starts:
//!
//! ```text
//! "SShd"          4 bytes, ASCII tag
//! header_len      u32le, 24 in both files measured
//! format          u32le, 1 in both files measured - see "format 1" below
//! sample_rate     u32le, 48000 in both files measured
//! channels        u32le, 2 in both files measured
//! interleave      u32le, 512 in both files measured
//! (gap)           header_len says the SShd chunk is over at this point; the
//!                 gap to "SSbd" is 8 bytes of 0xff in both files measured,
//!                 not assumed - this walk finds "SSbd" instead of hardcoding
//!                 the gap's length
//! "SSbd"          4 bytes, ASCII tag
//! body_len        u32le - the exact byte count of what follows, across every
//!                 packet including this one
//! ```
//!
//! No packet after the first repeats `SShd`: the framing on every later
//! packet is [`SUBSTREAM_PREFIX`] alone, and everything past it is a straight
//! continuation of the body. Concatenating (packet 0's payload, past its
//! [`SUBSTREAM_PREFIX`] and metadata block) with (every later packet's
//! payload, past its own [`SUBSTREAM_PREFIX`]) reproduces `body_len` exactly -
//! 7,303,168 bytes over 1,795 packets for `INTRO640.PSS` - which is the
//! strongest evidence the framing above is read right rather than guessed.
//!
//! # `format 1` is 16-bit PCM, not PS-ADPCM
//!
//! The investigation that found the framing above first read
//! `private_stream_1`'s payload as PS-ADPCM, on the reasonable assumption that
//! PS2 movie audio usually is. It measured the framing right and the codec
//! wrong: decoding `INTRO640.PSS`'s body as 16-byte PS-ADPCM
//! blocks - with or without a 512-byte channel split - leaves under 6% of
//! blocks in spec (predictor under 5, shift at most 12) at any of the 16
//! possible byte alignments, which is what unrelated bytes give by chance,
//! not a four-value predictor codec.
//!
//! Reading the same bytes as **16-bit signed little-endian PCM**, 512 bytes
//! (256 samples) per channel at a time, gives a track that:
//!
//! - runs 38.037 s against `ffprobe`'s own 38.071367 s for the container;
//! - has a mean sample-to-sample step of about 946 against roughly 21,845 for
//!   uniform noise at 16 bits - the actual data is over 20 times smoother;
//! - shows no seam at the 256-sample block boundary: the mean step across a
//!   join (959) is the same as the mean step everywhere else (946), which is
//!   what a real per-channel waveform assembled correctly looks like, not
//!   what an accidentally-scrambled interleave would.
//!
//! All three agree on PCM and none agree on ADPCM. Both `.PSS` files on this
//! disc declare `format 1`; [`Format::is_pcm16`] is the gate a caller checks
//! before trusting that reading - a build that saw a different value has no
//! decoder for it and must say so rather than assume PCM again. See
//! `docs/formats/pss.md` for the full evidence and the byte dumps behind it.

/// MPEG stream id carrying this container's audio, `private_stream_1`.
///
/// The same id as `.PMF`'s `oag_video::pmf::AUDIO_STREAM_ID` - it is the
/// generic MPEG convention for a privately-defined stream, not something
/// specific to either container - given its own constant here because what
/// actually rides inside it is a completely different codec; see the module
/// docs.
pub const PRIVATE_STREAM_1_ID: u8 = 0xbd;

/// The 4-byte sub-header every `private_stream_1` payload in a `.PSS` opens
/// with, measured constant across both files this checkout has (1,795
/// packets in `INTRO640.PSS`, 1,794 in `INTRO512.PSS`, every one of them).
///
/// The first byte reads as a Sony private-stream substream id (`0xff`); the
/// other three are not decoded further here because they never varied across
/// either file measured.
pub const SUBSTREAM_PREFIX: [u8; 4] = [0xff, 0xa0, 0x00, 0x00];

/// The ASCII tag the first `private_stream_1` payload's metadata block opens
/// with, past [`SUBSTREAM_PREFIX`].
pub const HEADER_TAG: [u8; 4] = *b"SShd";

/// The ASCII tag that follows the header fields (at `header_len` bytes past
/// [`HEADER_TAG`]) and precedes the `u32le` body length.
pub const BODY_TAG: [u8; 4] = *b"SSbd";

/// The `format` code this build has evidence for: 16-bit signed
/// little-endian PCM, [`Format::interleave`] bytes at a time per channel.
/// See the module docs for how this was told apart from PS-ADPCM.
pub const FORMAT_PCM16: u32 = 1;

/// Something wrong with a `.PSS`'s `private_stream_1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The pack/PES walk found no `private_stream_1` packets at all.
    NoPrivateStream1Packets,
    /// A packet's payload is shorter than [`SUBSTREAM_PREFIX`].
    PacketTooShort {
        /// Index of the packet, counting from zero.
        packet: usize,
        /// Bytes the payload actually had.
        got: usize,
    },
    /// A packet's payload does not open with [`SUBSTREAM_PREFIX`].
    BadSubstreamPrefix {
        /// Index of the packet, counting from zero.
        packet: usize,
        /// The four bytes found instead.
        found: [u8; 4],
    },
    /// The first packet, past its prefix, is too short to hold [`HEADER_TAG`].
    TooShortForHeaderTag {
        /// Bytes available past the prefix.
        got: usize,
    },
    /// The first packet's metadata block does not open with [`HEADER_TAG`].
    MissingHeaderTag {
        /// The four bytes found instead.
        found: [u8; 4],
    },
    /// `header_len` runs past the end of the first packet's payload.
    HeaderLenOutOfRange {
        /// `header_len` as declared.
        header_len: usize,
        /// Bytes available past [`HEADER_TAG`].
        available: usize,
    },
    /// [`BODY_TAG`] does not appear where `header_len` says the metadata
    /// block ends.
    MissingBodyTag,
    /// Fewer bytes were recovered, across every packet, than `body_len`
    /// declared.
    BodyShorterThanDeclared {
        /// What `body_len` declared.
        declared: usize,
        /// What the walk actually recovered.
        got: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPrivateStream1Packets => {
                f.write_str("no private_stream_1 (0xbd) packets in this program stream")
            }
            Self::PacketTooShort { packet, got } => write!(
                f,
                "packet {packet} is {got} byte(s), shorter than the 4-byte substream prefix"
            ),
            Self::BadSubstreamPrefix { packet, found } => write!(
                f,
                "packet {packet} opens with {found:02x?}, not the measured {SUBSTREAM_PREFIX:02x?}"
            ),
            Self::TooShortForHeaderTag { got } => write!(
                f,
                "the first packet has {got} byte(s) past its prefix, too short for \"SShd\""
            ),
            Self::MissingHeaderTag { found } => {
                write!(f, "the first packet opens with {found:02x?}, not \"SShd\"")
            }
            Self::HeaderLenOutOfRange {
                header_len,
                available,
            } => write!(
                f,
                "header_len {header_len} runs past the {available} byte(s) available"
            ),
            Self::MissingBodyTag => f.write_str("no \"SSbd\" tag found after the SShd header"),
            Self::BodyShorterThanDeclared { declared, got } => write!(
                f,
                "body_len declared {declared} byte(s) but only {got} were recovered"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// The stream parameters `SShd` declares, raw.
///
/// Nothing here is interpreted beyond [`Format::is_pcm16`] - a `format`,
/// `sample_rate` or `channels` this build has not measured is reported as
/// exactly what the file said, not translated into a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    /// The codec code. [`FORMAT_PCM16`] is the only value measured so far.
    pub format: u32,
    /// Frames per second.
    pub sample_rate: u32,
    /// Channels per frame.
    pub channels: u32,
    /// Bytes of one channel's data before the stream switches to the next -
    /// 512 in both files measured. See the module docs for what a channel's
    /// 512 bytes hold under [`FORMAT_PCM16`].
    pub interleave: usize,
}

impl Format {
    /// Whether this is the one codec this build knows how to play: 16-bit
    /// signed little-endian PCM. See the module docs for the evidence.
    #[must_use]
    pub fn is_pcm16(&self) -> bool {
        self.format == FORMAT_PCM16
    }
}

/// A demuxed `.PSS` audio track: the stream's own parameters and its raw
/// body, channel-interleaved exactly as authored.
///
/// Nothing here decodes it - not even for [`FORMAT_PCM16`], where "decode" is
/// just reading the bytes as samples. That reading is the caller's job, the
/// same split `oag_video::pmf` draws for ATRAC3+: this module's job ends at
/// the container's own framing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Demuxed {
    /// What `SShd` declared.
    pub format: Format,
    /// Every byte `SSbd` declared, in channel-interleave order, with every
    /// packet's [`SUBSTREAM_PREFIX`] and the first packet's metadata block
    /// removed.
    pub body: Vec<u8>,
    /// How many `private_stream_1` packets the walk found.
    pub packet_count: usize,
}

/// Demuxes a raw MPEG-2 program stream's `private_stream_1` into its
/// `SShd`/`SSbd`-framed audio body.
///
/// `data` is the whole `.PSS` file (or any raw MPEG-2 program stream carrying
/// the same framing) - there is no header in front of it to skip, unlike a
/// `.PMF`.
///
/// # Errors
///
/// When there are no `private_stream_1` packets at all, when a packet does
/// not open with the measured [`SUBSTREAM_PREFIX`], or when the first
/// packet's metadata block does not parse as `SShd`/`SSbd` framing. Never
/// panics on a truncated or malformed file.
pub fn demux(data: &[u8]) -> Result<Demuxed> {
    let packets = private_stream_1_packets(data);
    let first = packets.first().ok_or(Error::NoPrivateStream1Packets)?;
    let rest0 = strip_prefix(first, 0)?;

    let Some(tag) = rest0.get(..4) else {
        return Err(Error::TooShortForHeaderTag { got: rest0.len() });
    };
    if tag != HEADER_TAG {
        let mut found = [0u8; 4];
        found.copy_from_slice(tag);
        return Err(Error::MissingHeaderTag { found });
    }

    let header_len = u32_le(rest0, 4) as usize;
    let format = u32_le(rest0, 8);
    let sample_rate = u32_le(rest0, 12);
    let channels = u32_le(rest0, 16);
    let interleave = u32_le(rest0, 20) as usize;

    let after_header = rest0.get(header_len..).ok_or(Error::HeaderLenOutOfRange {
        header_len,
        available: rest0.len(),
    })?;
    let ssbd_at = find_tag(after_header, &BODY_TAG).ok_or(Error::MissingBodyTag)?;
    let body_len = u32_le(after_header, ssbd_at + 4) as usize;
    let body_start = header_len + ssbd_at + 8;

    let mut body = Vec::with_capacity(body_len);
    body.extend_from_slice(rest0.get(body_start..).unwrap_or_default());
    for (index, packet) in packets.iter().enumerate().skip(1) {
        body.extend_from_slice(strip_prefix(packet, index)?);
    }

    if body.len() < body_len {
        return Err(Error::BodyShorterThanDeclared {
            declared: body_len,
            got: body.len(),
        });
    }
    body.truncate(body_len);

    Ok(Demuxed {
        format: Format {
            format,
            sample_rate,
            channels,
            interleave,
        },
        body,
        packet_count: packets.len(),
    })
}

/// Strips [`SUBSTREAM_PREFIX`] off one `private_stream_1` payload.
fn strip_prefix(packet: &[u8], index: usize) -> Result<&[u8]> {
    let Some(prefix) = packet.get(..4) else {
        return Err(Error::PacketTooShort {
            packet: index,
            got: packet.len(),
        });
    };
    if prefix != SUBSTREAM_PREFIX {
        let mut found = [0u8; 4];
        found.copy_from_slice(prefix);
        return Err(Error::BadSubstreamPrefix {
            packet: index,
            found,
        });
    }
    Ok(&packet[4..])
}

/// The first index at which `data` starts with `tag`, if any.
fn find_tag(data: &[u8], tag: &[u8; 4]) -> Option<usize> {
    data.windows(4).position(|w| w == tag)
}

fn u32_le(data: &[u8], at: usize) -> u32 {
    data.get(at..at + 4)
        .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Walks a raw MPEG-2 program stream's packs and PES packets, collecting only
/// [`PRIVATE_STREAM_1_ID`] payloads.
///
/// The same pack/PES framing `oag_video::pmf::demux_payload` walks, but
/// deliberately not that function: it also collects the video elementary
/// stream, and a `.PSS` this size carries tens of megabytes of it that
/// nothing here would ever read. Losing sync is not fatal, the same as that
/// walk: a truncated or malformed file yields whatever packets it does
/// contain rather than panicking.
fn private_stream_1_packets(payload: &[u8]) -> Vec<&[u8]> {
    let mut packets = Vec::new();
    let mut at = 0usize;

    while at + 4 <= payload.len() {
        if payload[at..at + 3] != [0, 0, 1] {
            at += 1;
            continue;
        }

        match payload[at + 3] {
            // Pack header: 14 bytes plus a stuffing length in the low 3 bits
            // of the last one.
            0xba => {
                let Some(&stuffing) = payload.get(at + 13) else {
                    break;
                };
                at += 14 + usize::from(stuffing & 7);
            }
            // Program end.
            0xb9 => at += 4,
            id => {
                let Some(len) = payload
                    .get(at + 4..at + 6)
                    .map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])))
                else {
                    break;
                };
                let body_at = at + 6;
                let Some(body) = payload.get(body_at..body_at + len) else {
                    break;
                };

                if id == PRIVATE_STREAM_1_ID
                    && let Some(&extra) = body.get(2)
                    && let Some(pes) = body.get(3 + usize::from(extra)..)
                {
                    packets.push(pes);
                }

                at = body_at + len;
            }
        }
    }

    packets
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds one 2048-byte pack holding a `private_stream_1` PES packet,
    /// the way the pack/PES walk expects: a pack header, then a PES packet
    /// with a two-flag-byte, one-length-byte, zero-extra-bytes header.
    fn pack(payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&[0, 0, 1, 0xba]);
        out.extend_from_slice(&[0u8; 9]);
        out.push(0); // no stuffing

        let body_len = 3 + payload.len();
        out.extend_from_slice(&[0, 0, 1, PRIVATE_STREAM_1_ID]);
        out.extend_from_slice(&(body_len as u16).to_be_bytes());
        out.extend_from_slice(&[0x80, 0x00, 0x00]); // flags, flags, no extra
        out.extend_from_slice(payload);
        out
    }

    /// A minimal but structurally valid first packet: the substream prefix,
    /// then an `SShd`/`SSbd` metadata block with an 8-byte gap between them
    /// (measured, not assumed to be zero-length), then `own_body` - which may
    /// be shorter than `declared_body_len` when later packets carry the rest.
    fn first_packet(
        sample_rate: u32,
        channels: u32,
        interleave: u32,
        declared_body_len: usize,
        own_body: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&SUBSTREAM_PREFIX);
        out.extend_from_slice(&HEADER_TAG);
        out.extend_from_slice(&24u32.to_le_bytes()); // header_len
        out.extend_from_slice(&FORMAT_PCM16.to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&interleave.to_le_bytes());
        out.extend_from_slice(&[0xff; 8]); // the measured gap
        out.extend_from_slice(&BODY_TAG);
        out.extend_from_slice(&(declared_body_len as u32).to_le_bytes());
        out.extend_from_slice(own_body);
        out
    }

    /// A later packet: just the substream prefix and a body continuation.
    fn later_packet(body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&SUBSTREAM_PREFIX);
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn demuxes_a_single_packet_track() {
        let body = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let data = pack(&first_packet(48_000, 2, 512, body.len(), &body));

        let demuxed = demux(&data).expect("demuxing");
        assert_eq!(demuxed.packet_count, 1);
        assert_eq!(demuxed.body, body);
        assert_eq!(demuxed.format.format, FORMAT_PCM16);
        assert_eq!(demuxed.format.sample_rate, 48_000);
        assert_eq!(demuxed.format.channels, 2);
        assert_eq!(demuxed.format.interleave, 512);
        assert!(demuxed.format.is_pcm16());
    }

    #[test]
    fn concatenates_the_body_across_packets_in_order() {
        let mut data = pack(&first_packet(48_000, 2, 512, 12, &[0xaa; 4]));
        data.extend_from_slice(&pack(&later_packet(&[0xbb; 4])));
        data.extend_from_slice(&pack(&later_packet(&[0xcc; 4])));

        let demuxed = demux(&data).expect("demuxing");
        assert_eq!(demuxed.packet_count, 3);
        assert_eq!(
            demuxed.body,
            [
                0xaa, 0xaa, 0xaa, 0xaa, 0xbb, 0xbb, 0xbb, 0xbb, 0xcc, 0xcc, 0xcc, 0xcc
            ]
        );
    }

    #[test]
    fn a_trailing_partial_body_is_dropped_rather_than_kept() {
        // body_len declares 8 but only 4 are actually supplied - a truncated
        // capture, or a file cut for a test fixture.
        let first = first_packet(48_000, 2, 512, 8, &[0xaa; 4]);
        let data = pack(&first);

        assert_eq!(
            demux(&data).unwrap_err(),
            Error::BodyShorterThanDeclared {
                declared: 8,
                got: 4
            }
        );
    }

    #[test]
    fn a_stream_with_no_private_stream_1_packets_is_refused() {
        // Just a pack header and program end, no PES packets at all.
        let mut data = Vec::new();
        data.extend_from_slice(&[0, 0, 1, 0xba]);
        data.extend_from_slice(&[0u8; 9]);
        data.push(0);
        data.extend_from_slice(&[0, 0, 1, 0xb9]);

        assert_eq!(demux(&data), Err(Error::NoPrivateStream1Packets));
    }

    #[test]
    fn a_packet_not_opening_with_the_measured_prefix_is_refused() {
        let mut first = first_packet(48_000, 2, 512, 4, &[0; 4]);
        first[0] = 0x00; // corrupt the substream prefix
        let data = pack(&first);

        assert_eq!(
            demux(&data).unwrap_err(),
            Error::BadSubstreamPrefix {
                packet: 0,
                found: [0x00, 0xa0, 0x00, 0x00]
            }
        );
    }

    #[test]
    fn a_missing_header_tag_is_refused() {
        let mut first = first_packet(48_000, 2, 512, 4, &[0; 4]);
        first[4] = b'X'; // corrupt "SShd"
        let data = pack(&first);

        assert!(matches!(
            demux(&data).unwrap_err(),
            Error::MissingHeaderTag { .. }
        ));
    }

    #[test]
    fn a_missing_body_tag_is_refused() {
        // Valid SShd header, but the bytes at the expected SSbd offset are
        // not "SSbd" and never become it anywhere in the packet.
        let mut out = Vec::new();
        out.extend_from_slice(&SUBSTREAM_PREFIX);
        out.extend_from_slice(&HEADER_TAG);
        out.extend_from_slice(&24u32.to_le_bytes());
        out.extend_from_slice(&FORMAT_PCM16.to_le_bytes());
        out.extend_from_slice(&48_000u32.to_le_bytes());
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&512u32.to_le_bytes());
        out.extend_from_slice(&[0u8; 16]); // no "SSbd" anywhere in here
        let data = pack(&out);

        assert_eq!(demux(&data), Err(Error::MissingBodyTag));
    }
}
