//! PSP movie files (`.PMF`): a PSMF wrapper around an MPEG program stream.
//!
//! ```text
//! +0x00  u8[4]   "PSMF"
//! +0x04  u8[4]   version, ASCII: "0012" or "0014"
//! +0x08  u32be   stream offset, always 0x800 in shipped files
//! +0x0c  u32be   stream size
//! +0x54  u48be   presentation start time, 90 kHz
//! +0x5a  u48be   presentation end time, 90 kHz
//! +0x80  u16be   stream count
//! +0x82  16 bytes per stream descriptor
//! ```
//!
//! A stream descriptor is keyed by its MPEG stream id at `+0x00`: `0xe0` for
//! AVC video, `0xbd` for ATRAC3+ audio in `private_stream_1`.
//!
//! ```text
//! video  +0x04 u32be EP map offset   +0x08 u32be EP map entries
//!        +0x0c u8    width / 16      +0x0d u8    height / 16
//! audio  +0x0e u8    channels        +0x0f u8    frequency code
//! ```
//!
//! See `docs/formats/pmf.md` for the evidence and the checks that pin it down.
//!
//! # What this module does and does not do
//!
//! It parses the header and **demuxes** the program stream into elementary
//! streams. It does not decode them: the video is H.264 and the audio is
//! ATRAC3+, and neither decoder lives here. [`frame_count`] counts H.264 access
//! units, which is enough to pace playback against the original's own frame
//! counters without decoding a single macroblock.

/// The four bytes a PSMF header begins with.
pub const MAGIC: &[u8; 4] = b"PSMF";

/// Bytes of PSMF header before the program stream. Also the observed value of
/// [`Header::stream_offset`] in every shipped file.
pub const HEADER_LEN: usize = 0x800;

/// Ticks per second of an MPEG presentation timestamp.
pub const TIMESTAMP_HZ: u64 = 90_000;

/// MPEG stream id of the AVC video stream.
pub const VIDEO_STREAM_ID: u8 = 0xe0;

/// MPEG stream id carrying ATRAC3+ audio (`private_stream_1`).
pub const AUDIO_STREAM_ID: u8 = 0xbd;

/// Something wrong with a PMF blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The blob does not start with `PSMF`.
    NotPsmf,
    /// The version field is not four ASCII digits.
    BadVersion {
        /// The four bytes found.
        found: [u8; 4],
    },
    /// The stream offset is outside the blob, or before the header.
    BadStreamOffset {
        /// The offset found.
        offset: u32,
    },
    /// `stream_offset + stream_size` does not match the blob length.
    ///
    /// The strongest structural check available: the two fields fully determine
    /// the file size, so a mismatch means the layout is being misread.
    SizeMismatch {
        /// What the header implies.
        expected: u64,
        /// What was supplied.
        got: u64,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "need at least {HEADER_LEN} bytes, got {got}"),
            Self::NotPsmf => f.write_str("not a PSMF blob"),
            Self::BadVersion { found } => {
                write!(f, "version {:?} is not four ASCII digits", *found)
            }
            Self::BadStreamOffset { offset } => write!(f, "stream offset {offset} is out of range"),
            Self::SizeMismatch { expected, got } => {
                write!(f, "header implies {expected} bytes, blob is {got}")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// The video stream's parameters, from its 16-byte descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoStream {
    /// Width in pixels. Stored as sixteenths.
    pub width: u16,
    /// Height in pixels. Stored as sixteenths.
    pub height: u16,
    /// Byte offset of the entry-point map, relative to the start of the blob.
    pub ep_map_offset: u32,
    /// Number of entry-point map entries.
    pub ep_map_entries: u32,
}

/// The audio stream's parameters, from its 16-byte descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioStream {
    /// Channel count.
    pub channels: u8,
    /// Sample-rate code. Only `2`, meaning 44,100 Hz, appears in shipped files.
    pub frequency_code: u8,
}

impl AudioStream {
    /// Sample rate in hertz, or `None` for a code this build does not know.
    ///
    /// Only code 2 was ever observed, so anything else is reported as unknown
    /// rather than guessed at.
    #[must_use]
    pub fn frequency_hz(self) -> Option<u32> {
        match self.frequency_code {
            2 => Some(44_100),
            _ => None,
        }
    }
}

/// A parsed PSMF header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The four version digits, for example `*b"0014"`.
    pub version: [u8; 4],
    /// Byte offset of the program stream.
    pub stream_offset: u32,
    /// Length of the program stream in bytes.
    pub stream_size: u32,
    /// Presentation start time in 90 kHz ticks.
    pub presentation_start: u64,
    /// Presentation end time in 90 kHz ticks.
    pub presentation_end: u64,
    /// How many stream descriptors the header declares.
    pub stream_count: u16,
    /// The video stream, if one is declared.
    pub video: Option<VideoStream>,
    /// The audio stream, if one is declared.
    pub audio: Option<AudioStream>,
}

impl Header {
    /// Parses the PSMF header at the start of `data`.
    ///
    /// `data` may be the whole file or just its first [`HEADER_LEN`] bytes; the
    /// size check is only applied when the whole file is supplied, so a header
    /// can be inspected without reading megabytes off a disc.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let head = data
            .get(..HEADER_LEN)
            .ok_or(Error::TooShort { got: data.len() })?;

        if &head[..4] != MAGIC {
            return Err(Error::NotPsmf);
        }

        let mut version = [0u8; 4];
        version.copy_from_slice(&head[4..8]);
        if !version.iter().all(u8::is_ascii_digit) {
            return Err(Error::BadVersion { found: version });
        }

        let stream_offset = be32(head, 8);
        let stream_size = be32(head, 12);

        if (stream_offset as usize) < HEADER_LEN {
            return Err(Error::BadStreamOffset {
                offset: stream_offset,
            });
        }

        // Only enforced when the caller handed over the whole file. Header-only
        // inspection is a legitimate and much cheaper thing to want.
        let implied = u64::from(stream_offset) + u64::from(stream_size);
        if data.len() > HEADER_LEN && data.len() as u64 != implied {
            return Err(Error::SizeMismatch {
                expected: implied,
                got: data.len() as u64,
            });
        }

        let stream_count = be16(head, 0x80);
        let mut video = None;
        let mut audio = None;

        for index in 0..usize::from(stream_count) {
            let at = 0x82 + index * 16;
            let Some(desc) = head.get(at..at + 16) else {
                break;
            };
            match desc[0] {
                AUDIO_STREAM_ID => {
                    audio = Some(AudioStream {
                        channels: desc[14],
                        frequency_code: desc[15],
                    });
                }
                // Video ids are a range: the low bits select the stream number.
                id if id & VIDEO_STREAM_ID == VIDEO_STREAM_ID => {
                    video = Some(VideoStream {
                        width: u16::from(desc[12]) * 16,
                        height: u16::from(desc[13]) * 16,
                        ep_map_offset: be32(desc, 4),
                        ep_map_entries: be32(desc, 8),
                    });
                }
                _ => {}
            }
        }

        Ok(Self {
            version,
            stream_offset,
            stream_size,
            presentation_start: be48(head, 0x54),
            presentation_end: be48(head, 0x5a),
            stream_count,
            video,
            audio,
        })
    }

    /// Length the header implies the whole file has.
    #[must_use]
    pub fn total_len(&self) -> u64 {
        u64::from(self.stream_offset) + u64::from(self.stream_size)
    }

    /// Presentation duration in seconds.
    #[must_use]
    pub fn duration_seconds(&self) -> f64 {
        let ticks = self
            .presentation_end
            .saturating_sub(self.presentation_start);
        ticks as f64 / TIMESTAMP_HZ as f64
    }

    /// Frame count implied by the duration at the PSP's video rate.
    ///
    /// The PSP plays movies at 30000/1001 Hz. This is a prediction, and
    /// [`frame_count`] over the demuxed video is the measurement; the two
    /// agreeing is what validates the header.
    #[must_use]
    pub fn expected_frame_count(&self) -> u64 {
        let ticks = self
            .presentation_end
            .saturating_sub(self.presentation_start);
        // 90000 ticks per second, 30000/1001 frames per second.
        ticks * 30_000 / (1001 * TIMESTAMP_HZ)
    }
}

/// Elementary streams recovered from the program stream.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Demuxed {
    /// H.264 video in Annex B form, start codes included.
    pub video: Vec<u8>,
    /// One `private_stream_1` PES payload per packet, in presentation order.
    ///
    /// **Not one ATRAC3+ frame each**, which this said until the frames were
    /// actually decoded. Two layers of framing are still on them, and both are
    /// the container's rather than the codec's: four bytes of sub-header on each
    /// payload, whose third and fourth are a big-endian offset to the first
    /// frame that *starts* in it, and then eight bytes on each frame - the sync
    /// word `0f d0`, a codec config word, and four zero bytes. Frames straddle
    /// packet boundaries, so a payload is a slice of a byte stream and not a
    /// unit. `oag_game::movie` unwraps both; see its `movie_audio`.
    pub audio: Vec<Vec<u8>>,
    /// Pack headers seen. Every pack is 2048 bytes in shipped files.
    pub packs: usize,
    /// Bytes that were not part of any recognised pack or PES packet.
    ///
    /// **Zero in a correct demux.** A non-zero count means the walk lost sync,
    /// which is the failure this field exists to make visible.
    pub stray_bytes: usize,
}

/// Splits a PMF's program stream into elementary streams.
///
/// Takes the whole blob, not just the payload: the header says where the
/// payload starts.
pub fn demux(data: &[u8]) -> Result<Demuxed> {
    let header = Header::parse(data)?;
    let payload = data
        .get(header.stream_offset as usize..)
        .ok_or(Error::BadStreamOffset {
            offset: header.stream_offset,
        })?;
    Ok(demux_payload(payload))
}

/// Splits a bare MPEG program stream, with no PSMF header in front of it.
#[must_use]
pub fn demux_payload(payload: &[u8]) -> Demuxed {
    let mut out = Demuxed::default();
    let mut at = 0usize;

    while at + 4 <= payload.len() {
        if payload[at..at + 3] != [0, 0, 1] {
            // Losing sync is not fatal: count the byte and keep looking, so a
            // truncated file still yields whatever it does contain.
            out.stray_bytes += 1;
            at += 1;
            continue;
        }

        match payload[at + 3] {
            // Pack header: 14 bytes plus a stuffing length in the low 3 bits of
            // the last one.
            0xba => {
                let Some(&stuffing) = payload.get(at + 13) else {
                    break;
                };
                out.packs += 1;
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

                if id == VIDEO_STREAM_ID || id == AUDIO_STREAM_ID {
                    // PES header: two flag bytes then a length, then that many
                    // bytes of optional fields before the payload.
                    if let Some(&extra) = body.get(2)
                        && let Some(pes) = body.get(3 + usize::from(extra)..)
                    {
                        if id == VIDEO_STREAM_ID {
                            out.video.extend_from_slice(pes);
                        } else {
                            out.audio.push(pes.to_vec());
                        }
                    }
                }

                at = body_at + len;
            }
        }
    }

    // A trailing partial start code is stray, same as any other lost byte.
    out.stray_bytes += payload.len().saturating_sub(at);
    out
}

/// Counts frames in an H.264 Annex B elementary stream.
///
/// Counts **access unit delimiters** (NAL type 9), which the PSP encoder emits
/// once per frame. Falls back to counting the first slice of each picture when
/// a stream has no delimiters.
#[must_use]
pub fn frame_count(video: &[u8]) -> usize {
    let mut delimiters = 0usize;
    let mut first_slices = 0usize;

    for nal in nal_units(video) {
        let Some(&first) = nal.first() else { continue };
        match first & 0x1f {
            9 => delimiters += 1,
            // A slice whose `first_mb_in_slice` is 0 starts a picture. That
            // field is the first `ue(v)` of the header, so it is zero exactly
            // when the top bit of the next byte is set.
            1 | 5 if nal.get(1).is_some_and(|b| b & 0x80 != 0) => first_slices += 1,
            _ => {}
        }
    }

    if delimiters > 0 {
        delimiters
    } else {
        first_slices
    }
}

/// Splits an H.264 Annex B elementary stream into access units, as byte
/// ranges of `video` with their start codes.
///
/// Cut where [`frame_count`] counts, so the two agree: at each access unit
/// delimiter (NAL type 9), or at the first slice of each picture when a stream
/// has none. Whatever precedes the first cut (a parameter set ahead of the
/// first delimiter) stays on the first unit, which is the one a decoder starts
/// on and needs it. A decoder that takes one picture per chunk (the browser's
/// WebCodecs) is fed these.
#[must_use]
pub fn access_units(video: &[u8]) -> Vec<std::ops::Range<usize>> {
    let delimited = nal_units(video).any(|nal| nal.first().is_some_and(|b| b & 0x1f == 9));
    let mut cuts = Vec::new();
    let mut at = 0;
    while let Some(start) = find_start_code(video, at) {
        let body = start + 3;
        let starts_picture = video.get(body).is_some_and(|&first| match first & 0x1f {
            9 => delimited,
            1 | 5 => !delimited && video.get(body + 1).is_some_and(|b| b & 0x80 != 0),
            _ => false,
        });
        if starts_picture {
            // A four-byte start code's leading zero belongs to this unit.
            let cut = if start > 0 && video[start - 1] == 0 {
                start - 1
            } else {
                start
            };
            cuts.push(cut);
        }
        at = body;
    }
    if let Some(first) = cuts.first_mut() {
        *first = 0;
    }
    let mut ends: Vec<usize> = cuts.iter().skip(1).copied().collect();
    ends.push(video.len());
    cuts.into_iter()
        .zip(ends)
        .map(|(start, end)| start..end)
        .collect()
}

/// The WebCodecs codec string (`avc1.PPCCLL`) for a stream's first sequence
/// parameter set: its profile, constraint flags and level, in hex.
#[must_use]
pub fn avc_codec(video: &[u8]) -> Option<String> {
    nal_units(video)
        .find(|nal| nal.first().is_some_and(|b| b & 0x1f == 7))
        .and_then(|sps| sps.get(1..4))
        .map(|p| format!("avc1.{:02x}{:02x}{:02x}", p[0], p[1], p[2]))
}

/// Iterates the NAL units of an Annex B stream, without their start codes.
pub fn nal_units(video: &[u8]) -> impl Iterator<Item = &[u8]> {
    NalUnits { rest: video }
}

struct NalUnits<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for NalUnits<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let start = find_start_code(self.rest, 0)?;
        let body_at = start + 3;
        let end = find_start_code(self.rest, body_at).unwrap_or(self.rest.len());
        let nal = self.rest.get(body_at..end)?;
        self.rest = &self.rest[end.min(self.rest.len())..];
        Some(trim_trailing_zero(nal))
    }
}

/// Index of the next `00 00 01` at or after `from`.
fn find_start_code(data: &[u8], from: usize) -> Option<usize> {
    let mut at = from;
    while at + 3 <= data.len() {
        if data[at] == 0 && data[at + 1] == 0 && data[at + 2] == 1 {
            return Some(at);
        }
        at += 1;
    }
    None
}

/// Drops the zero byte a four-byte start code leaves on the previous unit.
fn trim_trailing_zero(nal: &[u8]) -> &[u8] {
    match nal {
        [rest @ .., 0] => rest,
        other => other,
    }
}

fn be16(data: &[u8], at: usize) -> u16 {
    data.get(at..at + 2)
        .map_or(0, |b| u16::from_be_bytes([b[0], b[1]]))
}

fn be32(data: &[u8], at: usize) -> u32 {
    data.get(at..at + 4)
        .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// A 48-bit big-endian MPEG timestamp.
fn be48(data: &[u8], at: usize) -> u64 {
    data.get(at..at + 6).map_or(0, |b| {
        b.iter()
            .fold(0u64, |acc, &byte| (acc << 8) | u64::from(byte))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal but structurally valid PSMF header, with both streams.
    fn header_bytes(payload_len: u32) -> Vec<u8> {
        let mut data = vec![0u8; HEADER_LEN];
        data[..4].copy_from_slice(b"PSMF");
        data[4..8].copy_from_slice(b"0014");
        data[8..12].copy_from_slice(&(HEADER_LEN as u32).to_be_bytes());
        data[12..16].copy_from_slice(&payload_len.to_be_bytes());
        // Start at one second, end at three: two seconds of presentation.
        data[0x54..0x5a].copy_from_slice(&90_000u64.to_be_bytes()[2..]);
        data[0x5a..0x60].copy_from_slice(&270_000u64.to_be_bytes()[2..]);
        data[0x80..0x82].copy_from_slice(&2u16.to_be_bytes());

        let video = 0x82;
        data[video] = VIDEO_STREAM_ID;
        data[video + 4..video + 8].copy_from_slice(&0xa2u32.to_be_bytes());
        data[video + 8..video + 12].copy_from_slice(&26u32.to_be_bytes());
        data[video + 12] = 30; // 480
        data[video + 13] = 17; // 272

        let audio = 0x92;
        data[audio] = AUDIO_STREAM_ID;
        data[audio + 14] = 2;
        data[audio + 15] = 2;

        data
    }

    #[test]
    fn parses_a_header() {
        let header = Header::parse(&header_bytes(0)).unwrap();
        assert_eq!(&header.version, b"0014");
        assert_eq!(header.stream_offset, HEADER_LEN as u32);
        assert_eq!(header.stream_count, 2);

        let video = header.video.unwrap();
        assert_eq!((video.width, video.height), (480, 272));
        assert_eq!(video.ep_map_entries, 26);

        let audio = header.audio.unwrap();
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.frequency_hz(), Some(44_100));

        assert!((header.duration_seconds() - 2.0).abs() < 1e-9);
        assert_eq!(header.expected_frame_count(), 59);
    }

    #[test]
    fn rejects_a_blob_that_is_not_psmf() {
        let mut data = header_bytes(0);
        data[..4].copy_from_slice(b"RIFF");
        assert_eq!(Header::parse(&data), Err(Error::NotPsmf));
    }

    #[test]
    fn rejects_a_non_numeric_version() {
        let mut data = header_bytes(0);
        data[4..8].copy_from_slice(b"beta");
        assert!(matches!(
            Header::parse(&data),
            Err(Error::BadVersion { .. })
        ));
    }

    #[test]
    fn rejects_a_length_the_header_disagrees_with() {
        // The header claims a 64-byte payload but only 32 bytes follow.
        let mut data = header_bytes(64);
        data.extend_from_slice(&[0u8; 32]);
        assert_eq!(
            Header::parse(&data),
            Err(Error::SizeMismatch {
                expected: HEADER_LEN as u64 + 64,
                got: HEADER_LEN as u64 + 32,
            })
        );
    }

    #[test]
    fn a_short_blob_is_an_error_not_a_panic() {
        assert_eq!(Header::parse(b"PSMF0014"), Err(Error::TooShort { got: 8 }));
        assert_eq!(Header::parse(&[]), Err(Error::TooShort { got: 0 }));
    }

    /// Builds one 2048-byte pack holding a video and an audio PES packet.
    fn pack(video: &[u8], audio: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&[0, 0, 1, 0xba]);
        out.extend_from_slice(&[0u8; 9]);
        out.push(0); // no stuffing

        for (id, payload) in [(VIDEO_STREAM_ID, video), (AUDIO_STREAM_ID, audio)] {
            if payload.is_empty() {
                continue;
            }
            let body_len = 3 + payload.len();
            out.extend_from_slice(&[0, 0, 1, id]);
            out.extend_from_slice(&(body_len as u16).to_be_bytes());
            out.extend_from_slice(&[0x80, 0x00, 0x00]); // flags, flags, no extra
            out.extend_from_slice(payload);
        }
        out
    }

    #[test]
    fn demuxes_video_and_audio() {
        let video = [0, 0, 0, 1, 0x09, 0x10, 0, 0, 0, 1, 0x65, 0x88];
        let audio = [1u8, 2, 3, 4];
        let body = pack(&video, &audio);

        let mut data = header_bytes(body.len() as u32);
        data.extend_from_slice(&body);

        let out = demux(&data).unwrap();
        assert_eq!(out.packs, 1);
        assert_eq!(out.stray_bytes, 0, "every byte must be accounted for");
        assert_eq!(out.video, video);
        assert_eq!(out.audio, vec![audio.to_vec()]);
    }

    #[test]
    fn counts_frames_from_access_unit_delimiters() {
        let mut es = Vec::new();
        for _ in 0..3 {
            es.extend_from_slice(&[0, 0, 0, 1, 0x09, 0x10]);
            es.extend_from_slice(&[0, 0, 0, 1, 0x21, 0x88, 0x84]);
        }
        assert_eq!(frame_count(&es), 3);
    }

    #[test]
    fn counts_frames_from_slices_when_there_are_no_delimiters() {
        let mut es = Vec::new();
        // Two pictures, the second split into two slices. Only the slice whose
        // `first_mb_in_slice` is zero starts a picture.
        es.extend_from_slice(&[0, 0, 0, 1, 0x25, 0x88]);
        es.extend_from_slice(&[0, 0, 0, 1, 0x21, 0x88]);
        es.extend_from_slice(&[0, 0, 0, 1, 0x21, 0x0a]);
        assert_eq!(frame_count(&es), 2);
        assert_eq!(access_units(&es), vec![0..6, 6..18]);
    }

    #[test]
    fn access_units_cut_at_delimiters_and_keep_the_parameter_sets_on_the_first() {
        let mut es = vec![0, 0, 0, 1, 0x67, 0x4d, 0x40, 0x1e];
        for _ in 0..2 {
            es.extend_from_slice(&[0, 0, 0, 1, 0x09, 0x10]);
            es.extend_from_slice(&[0, 0, 1, 0x21, 0x88, 0x84]);
        }
        assert_eq!(access_units(&es), vec![0..20, 20..32]);
        assert_eq!(access_units(&es).len(), frame_count(&es));
        assert_eq!(avc_codec(&es).as_deref(), Some("avc1.4d401e"));
    }

    #[test]
    fn walks_nal_units_of_both_start_code_lengths() {
        let es = [0, 0, 1, 0x09, 0x10, 0, 0, 0, 1, 0x67, 0x4d];
        let nals: Vec<&[u8]> = nal_units(&es).collect();
        assert_eq!(nals.len(), 2);
        assert_eq!(nals[0], &[0x09, 0x10]);
        assert_eq!(nals[1], &[0x67, 0x4d]);
    }

    #[test]
    fn a_stream_with_no_start_codes_yields_nothing() {
        assert_eq!(nal_units(&[1, 2, 3, 4]).count(), 0);
        assert_eq!(frame_count(&[1, 2, 3, 4]), 0);
    }

    #[test]
    fn stray_bytes_are_counted_rather_than_panicked_on() {
        let out = demux_payload(&[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(out.packs, 0);
        assert_eq!(out.stray_bytes, 4);
    }

    #[test]
    fn a_truncated_pes_packet_stops_the_walk_cleanly() {
        // Claims 4096 bytes of body but supplies none.
        let payload = [0, 0, 1, VIDEO_STREAM_ID, 0x10, 0x00];
        let out = demux_payload(&payload);
        assert!(out.video.is_empty());
        assert_eq!(out.stray_bytes, payload.len());
    }
}
