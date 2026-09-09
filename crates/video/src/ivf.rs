//! IVF: a container thin enough to be worth parsing by hand.
//!
//! ```text
//! +0x00  u8[4]   "DKIF"
//! +0x04  u16le   version, 0
//! +0x06  u16le   header length, 32
//! +0x08  u8[4]   fourcc, "AV01" for AV1
//! +0x0c  u16le   width
//! +0x0e  u16le   height
//! +0x10  u32le   time base denominator
//! +0x14  u32le   time base numerator
//! +0x18  u32le   frame count, advisory
//! +0x1c  u32le   unused
//! ```
//!
//! Then one length-prefixed frame after another, to the end of the file:
//!
//! ```text
//! +0x00  u32le   payload length
//! +0x04  u64le   presentation timestamp
//! +0x0c  payload
//! ```
//!
//! Unlike everything else in this crate, IVF is not a Wipeout format. It is the
//! container the project's **own** movie cache is written in, holding the AV1
//! that `.PMF` video is transcoded into: see
//! `docs/architecture/adr/0008-av1-movie-cache.md`. It lives here because the
//! rest of the video path does, and because it is small enough that pulling in
//! an MP4 or WebM demuxer to read it would cost more than it saves.
//!
//! The header's frame count is advisory: nothing rejects a file whose count
//! disagrees with the frames actually present, and [`parse`] reports what it
//! found rather than what the header claimed.

/// Bytes of file header, and the value the header's own length field carries in
/// every writer's output.
pub const FILE_HEADER_LEN: usize = 32;

/// Bytes of per-frame header before the payload.
pub const FRAME_HEADER_LEN: usize = 12;

/// The four bytes an IVF file starts with.
pub const SIGNATURE: [u8; 4] = *b"DKIF";

/// The fourcc of an AV1 stream.
pub const FOURCC_AV1: [u8; 4] = *b"AV01";

/// Something wrong with an IVF blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the file header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The blob does not start with `DKIF`.
    NotIvf,
    /// The header length field is smaller than the header it describes, so the
    /// first frame would start inside it.
    HeaderTooShort {
        /// The length the field claimed.
        claimed: usize,
    },
    /// A frame's payload runs off the end of the blob.
    TruncatedFrame {
        /// Index of the frame, counting from zero.
        index: usize,
        /// Payload length the frame header claimed.
        want: usize,
        /// Bytes actually left.
        got: usize,
    },
    /// Bytes left over that are too few to be another frame header.
    TrailingBytes {
        /// How many.
        count: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => {
                write!(f, "{got} byte(s) is shorter than IVF's {FILE_HEADER_LEN}")
            }
            Self::NotIvf => write!(f, "does not start with DKIF"),
            Self::HeaderTooShort { claimed } => write!(
                f,
                "header length {claimed} is under the {FILE_HEADER_LEN}-byte header"
            ),
            Self::TruncatedFrame { index, want, got } => {
                write!(f, "frame {index} wants {want} byte(s) but {got} remain")
            }
            Self::TrailingBytes { count } => {
                write!(f, "{count} trailing byte(s) after the last frame")
            }
        }
    }
}

impl std::error::Error for Error {}

/// An IVF file header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// Codec fourcc, [`FOURCC_AV1`] for AV1.
    pub fourcc: [u8; 4],
    /// Frame width in pixels.
    pub width: u16,
    /// Frame height in pixels.
    pub height: u16,
    /// Time base denominator: the frame rate's numerator, in practice.
    pub rate: u32,
    /// Time base numerator: the frame rate's denominator, in practice.
    pub scale: u32,
    /// The frame count the header claims. Advisory; see the module docs.
    pub declared_frames: u32,
    /// Bytes before the first frame header, from the header's own field.
    pub header_len: usize,
}

impl Header {
    /// Parses the file header.
    ///
    /// # Errors
    ///
    /// If the blob is too short, is not IVF, or declares a header shorter than
    /// the fields it must contain.
    pub fn parse(data: &[u8]) -> Result<Self, Error> {
        if data.len() < FILE_HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }
        if data[0..4] != SIGNATURE {
            return Err(Error::NotIvf);
        }
        let header_len = u16::from_le_bytes([data[6], data[7]]) as usize;
        if header_len < FILE_HEADER_LEN {
            return Err(Error::HeaderTooShort {
                claimed: header_len,
            });
        }
        Ok(Self {
            fourcc: [data[8], data[9], data[10], data[11]],
            width: u16::from_le_bytes([data[12], data[13]]),
            height: u16::from_le_bytes([data[14], data[15]]),
            rate: u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            scale: u32::from_le_bytes([data[20], data[21], data[22], data[23]]),
            declared_frames: u32::from_le_bytes([data[24], data[25], data[26], data[27]]),
            header_len,
        })
    }

    /// Whether the stream is AV1.
    #[must_use]
    pub fn is_av1(&self) -> bool {
        self.fourcc == FOURCC_AV1
    }
}

/// One frame's payload and its timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame<'a> {
    /// Presentation timestamp in the header's time base.
    pub pts: u64,
    /// Byte offset of the payload within the blob it was parsed from.
    ///
    /// Carried so a caller that owns the blob can keep ranges into it rather
    /// than borrowing, which a decoder holding the blob needs.
    pub offset: usize,
    /// The coded payload, one temporal unit of AV1 OBUs.
    pub data: &'a [u8],
}

/// A parsed IVF file: its header and every frame in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ivf<'a> {
    /// The file header.
    pub header: Header,
    /// Frames in stream order.
    pub frames: Vec<Frame<'a>>,
}

/// Parses a whole IVF blob.
///
/// Frames are borrowed from `data` rather than copied: the caller already holds
/// the file, and the payloads are the bulk of it.
///
/// # Errors
///
/// If the header does not parse, if a frame's payload runs off the end, or if
/// the file ends with a partial frame header. Every byte after the header must
/// belong to a frame, which is what makes a mis-sized frame detectable rather
/// than silently resynchronising.
pub fn parse(data: &[u8]) -> Result<Ivf<'_>, Error> {
    let header = Header::parse(data)?;
    let mut frames = Vec::new();
    let mut at = header.header_len.min(data.len());

    while at < data.len() {
        let left = data.len() - at;
        if left < FRAME_HEADER_LEN {
            return Err(Error::TrailingBytes { count: left });
        }
        let size =
            u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize;
        let pts = u64::from_le_bytes([
            data[at + 4],
            data[at + 5],
            data[at + 6],
            data[at + 7],
            data[at + 8],
            data[at + 9],
            data[at + 10],
            data[at + 11],
        ]);
        at += FRAME_HEADER_LEN;
        if data.len() - at < size {
            return Err(Error::TruncatedFrame {
                index: frames.len(),
                want: size,
                got: data.len() - at,
            });
        }
        frames.push(Frame {
            pts,
            offset: at,
            data: &data[at..at + size],
        });
        at += size;
    }

    Ok(Ivf { header, frames })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an IVF blob with the given frame payloads.
    fn build(fourcc: &[u8; 4], width: u16, height: u16, payloads: &[&[u8]]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&SIGNATURE);
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(FILE_HEADER_LEN as u16).to_le_bytes());
        out.extend_from_slice(fourcc);
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.extend_from_slice(&30_000u32.to_le_bytes());
        out.extend_from_slice(&1001u32.to_le_bytes());
        out.extend_from_slice(&(payloads.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        for (i, p) in payloads.iter().enumerate() {
            out.extend_from_slice(&(p.len() as u32).to_le_bytes());
            out.extend_from_slice(&(i as u64).to_le_bytes());
            out.extend_from_slice(p);
        }
        out
    }

    #[test]
    fn reads_the_header() {
        let blob = build(&FOURCC_AV1, 480, 272, &[]);
        let header = Header::parse(&blob).unwrap();
        assert_eq!(header.fourcc, FOURCC_AV1);
        assert!(header.is_av1());
        assert_eq!((header.width, header.height), (480, 272));
        assert_eq!((header.rate, header.scale), (30_000, 1001));
        assert_eq!(header.header_len, FILE_HEADER_LEN);
    }

    #[test]
    fn splits_frames_and_keeps_their_order() {
        let blob = build(&FOURCC_AV1, 480, 272, &[b"one", b"three33", b"22"]);
        let ivf = parse(&blob).unwrap();
        assert_eq!(ivf.frames.len(), 3);
        assert_eq!(ivf.frames[0].data, b"one");
        assert_eq!(ivf.frames[1].data, b"three33");
        assert_eq!(ivf.frames[2].data, b"22");
        assert_eq!(ivf.frames[2].pts, 2);
    }

    /// The offsets must address the payloads inside the original blob, since a
    /// decoder that owns the blob indexes it by them rather than by borrowing.
    #[test]
    fn frame_offsets_point_at_the_payloads() {
        let blob = build(&FOURCC_AV1, 480, 272, &[b"one", b"three33", b"22"]);
        let ivf = parse(&blob).unwrap();
        for frame in &ivf.frames {
            assert_eq!(
                &blob[frame.offset..frame.offset + frame.data.len()],
                frame.data
            );
        }
        assert_eq!(ivf.frames[0].offset, FILE_HEADER_LEN + FRAME_HEADER_LEN);
    }

    #[test]
    fn an_empty_stream_parses_to_no_frames() {
        let blob = build(&FOURCC_AV1, 480, 272, &[]);
        assert!(parse(&blob).unwrap().frames.is_empty());
    }

    /// Zero-length frames are legal and must not spin the walk.
    #[test]
    fn zero_length_frames_advance() {
        let blob = build(&FOURCC_AV1, 16, 16, &[b"", b"", b"x"]);
        let ivf = parse(&blob).unwrap();
        assert_eq!(ivf.frames.len(), 3);
        assert_eq!(ivf.frames[2].data, b"x");
    }

    #[test]
    fn a_longer_header_is_honoured() {
        let mut blob = build(&FOURCC_AV1, 480, 272, &[b"payload"]);
        // Claim eight extra bytes of header and insert them, the way a writer
        // with a private extension would.
        blob[6] = (FILE_HEADER_LEN + 8) as u8;
        let mut spliced = blob[..FILE_HEADER_LEN].to_vec();
        spliced.extend_from_slice(&[0; 8]);
        spliced.extend_from_slice(&blob[FILE_HEADER_LEN..]);
        let ivf = parse(&spliced).unwrap();
        assert_eq!(ivf.frames.len(), 1);
        assert_eq!(ivf.frames[0].data, b"payload");
    }

    #[test]
    fn rejects_a_non_ivf_blob() {
        let mut blob = build(&FOURCC_AV1, 480, 272, &[b"x"]);
        blob[0] = b'X';
        assert_eq!(Header::parse(&blob), Err(Error::NotIvf));
    }

    #[test]
    fn rejects_a_short_blob() {
        assert_eq!(Header::parse(&[0; 8]), Err(Error::TooShort { got: 8 }));
    }

    #[test]
    fn rejects_a_header_length_under_the_header() {
        let mut blob = build(&FOURCC_AV1, 480, 272, &[b"x"]);
        blob[6] = 16;
        assert_eq!(
            Header::parse(&blob),
            Err(Error::HeaderTooShort { claimed: 16 })
        );
    }

    /// A mis-sized frame must be an error rather than a resynchronisation: the
    /// walk accounting for every byte is what makes the parse trustworthy.
    #[test]
    fn rejects_a_frame_running_past_the_end() {
        let mut blob = build(&FOURCC_AV1, 480, 272, &[b"payload"]);
        blob[FILE_HEADER_LEN] = 0xff;
        assert_eq!(
            parse(&blob),
            Err(Error::TruncatedFrame {
                index: 0,
                want: 0xff,
                got: 7,
            })
        );
    }

    #[test]
    fn rejects_a_partial_frame_header() {
        let mut blob = build(&FOURCC_AV1, 480, 272, &[b"x"]);
        blob.extend_from_slice(&[0; 4]);
        assert_eq!(parse(&blob), Err(Error::TrailingBytes { count: 4 }));
    }
}
