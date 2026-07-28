//! `IPUF`: the PS2's fixed-slot wrapper around an IPU video bitstream.
//!
//! Wipeout Pulse's PS2 release ships its looping menu backdrop as
//! `DATA/MOVIES/BG512.IPF` and `DATA/MOVIES/BG640.IPF`, loose in the ISO
//! filesystem beside the two `.PSS` intro cuts. Both begin `IPUF`, and what
//! they hold is **not** a program stream: it is the raw intra-only MPEG-2
//! variant the PS2's Image Processing Unit decodes, in Sony's `.IPU` shape,
//! cut into equal-sized slots so the streamer can seek by multiplying.
//!
//! ```text
//! +0x00  u8[4]   "IPUF"
//! +0x04  u16le   width
//! +0x06  u16le   height
//! +0x08  u32le   frame count
//! +0x0c  u32le   frame stride, bytes per slot
//! +0x10  u32le   alignment, 16 in both shipped files
//! +0x14  u32[3]  zero
//! +0x20          the first frame slot
//! ```
//!
//! Every slot is `frame stride` bytes and starts with its own 64-byte header,
//! of which only the first field is used:
//!
//! ```text
//! +0x00  u32le   payload length
//! +0x04  60      zero
//! +0x40          payload, then zero padding out to the stride
//! ```
//!
//! The payload is a single IPU frame: a four-byte IPU picture header, the
//! bitstream, and the `00 00 01 B0` end marker every frame closes on. This
//! module does not decode it - see `docs/formats/ipf.md` for the route, which
//! is the same out-of-process transcode `.PMF` and `.PSS` video already take.
//!
//! Nothing here reads the file's frame rate, because the container declares
//! none. That is the decoder's problem and it is discussed on the format page.

/// The four bytes an IPF file starts with.
pub const MAGIC: [u8; 4] = *b"IPUF";

/// Bytes of file header before the first frame slot.
pub const HEADER_LEN: usize = 32;

/// Bytes of per-slot header before the payload.
pub const FRAME_HEADER_LEN: usize = 64;

/// Something wrong with an IPF blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the file header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The blob does not start with `IPUF`.
    NotIpf,
    /// A zero width or height, which no decoder can use.
    ZeroDimension {
        /// Declared width.
        width: u32,
        /// Declared height.
        height: u32,
    },
    /// A stride too small to hold even a slot header.
    StrideTooShort {
        /// The stride the header claimed.
        stride: usize,
    },
    /// The slots the header declares do not fit in the blob.
    Truncated {
        /// Bytes the header implies.
        want: usize,
        /// Bytes supplied.
        got: usize,
    },
    /// A slot claims a payload longer than the slot itself.
    PayloadTooLong {
        /// Index of the frame, counting from zero.
        index: usize,
        /// Payload length the slot header claimed.
        claimed: usize,
        /// Bytes the slot has after its own header.
        available: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => {
                write!(f, "{got} byte(s) is shorter than IPF's {HEADER_LEN}")
            }
            Self::NotIpf => write!(f, "does not start with IPUF"),
            Self::ZeroDimension { width, height } => {
                write!(f, "declares a {width}x{height} picture")
            }
            Self::StrideTooShort { stride } => write!(
                f,
                "frame stride {stride} is under the {FRAME_HEADER_LEN}-byte slot header"
            ),
            Self::Truncated { want, got } => {
                write!(f, "declares {want} byte(s) of frames but has {got}")
            }
            Self::PayloadTooLong {
                index,
                claimed,
                available,
            } => write!(
                f,
                "frame {index} claims {claimed} byte(s) in a slot holding {available}"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// What an IPF's file header declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Picture width in pixels.
    pub width: u32,
    /// Picture height in pixels.
    pub height: u32,
    /// How many frame slots follow.
    pub frame_count: usize,
    /// Bytes per frame slot, including the slot's own 64-byte header.
    pub frame_stride: usize,
    /// The field at `+0x10`, `16` in both shipped files.
    ///
    /// Read as an alignment because the stride is exactly
    /// `64 + longest payload` rounded up to it in both files, but two files
    /// cannot separate that from a constant. See `docs/formats/ipf.md`.
    pub alignment: u32,
}

/// A parsed IPF: the header, and one slice per frame payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipf<'a> {
    /// What the file header declared.
    pub header: Header,
    /// One IPU frame each, with the slot header and the zero padding removed.
    pub frames: Vec<&'a [u8]>,
}

impl Ipf<'_> {
    /// How many frames the file holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether the file holds no frames at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Every frame payload end to end, which is what an IPU decoder wants.
    ///
    /// The slot framing exists so the streamer can seek by multiplying; the
    /// bitstream itself is self-delimiting, each frame ending on `00 00 01 B0`.
    /// So dropping the slots loses nothing - and is checkable, since a decoder
    /// splitting the result on its own must land on exactly the boundaries the
    /// slot headers declared.
    #[must_use]
    pub fn bitstream(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.frames.iter().map(|f| f.len()).sum());
        for frame in &self.frames {
            out.extend_from_slice(frame);
        }
        out
    }
}

fn u16_at(blob: &[u8], offset: usize) -> u32 {
    u32::from(u16::from_le_bytes([blob[offset], blob[offset + 1]]))
}

fn u32_at(blob: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        blob[offset],
        blob[offset + 1],
        blob[offset + 2],
        blob[offset + 3],
    ])
}

/// Reads an IPF's header and cuts out every frame's payload.
///
/// # Errors
///
/// When the blob is not an IPF, or when the slots the header declares do not
/// fit in the bytes supplied.
pub fn parse(blob: &[u8]) -> Result<Ipf<'_>, Error> {
    if blob.len() < HEADER_LEN {
        return Err(Error::TooShort { got: blob.len() });
    }
    if blob[..4] != MAGIC {
        return Err(Error::NotIpf);
    }

    let width = u16_at(blob, 4);
    let height = u16_at(blob, 6);
    if width == 0 || height == 0 {
        return Err(Error::ZeroDimension { width, height });
    }

    let frame_count = u32_at(blob, 8) as usize;
    let frame_stride = u32_at(blob, 12) as usize;
    let alignment = u32_at(blob, 16);

    if frame_stride <= FRAME_HEADER_LEN {
        return Err(Error::StrideTooShort {
            stride: frame_stride,
        });
    }

    let want = HEADER_LEN
        .checked_add(
            frame_count
                .checked_mul(frame_stride)
                .ok_or(Error::Truncated {
                    want: usize::MAX,
                    got: blob.len(),
                })?,
        )
        .ok_or(Error::Truncated {
            want: usize::MAX,
            got: blob.len(),
        })?;
    if blob.len() < want {
        return Err(Error::Truncated {
            want,
            got: blob.len(),
        });
    }

    let available = frame_stride - FRAME_HEADER_LEN;
    let mut frames = Vec::with_capacity(frame_count);
    for index in 0..frame_count {
        let slot = HEADER_LEN + index * frame_stride;
        let claimed = u32_at(blob, slot) as usize;
        if claimed > available {
            return Err(Error::PayloadTooLong {
                index,
                claimed,
                available,
            });
        }
        let start = slot + FRAME_HEADER_LEN;
        frames.push(&blob[start..start + claimed]);
    }

    Ok(Ipf {
        header: Header {
            width,
            height,
            frame_count,
            frame_stride,
            alignment,
        },
        frames,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an IPF from payloads, the way the shipped files are laid out:
    /// a 32-byte file header, then one `stride`-byte slot per frame, each
    /// opening with a 64-byte header whose first field is the payload length
    /// and whose payload is zero-padded out to the stride.
    ///
    /// Authored rather than extracted, per
    /// `docs/architecture/adr/0006-no-copyrighted-content.md`.
    fn build(width: u16, height: u16, payloads: &[&[u8]]) -> Vec<u8> {
        let longest = payloads.iter().map(|p| p.len()).max().unwrap_or(0);
        let stride = (FRAME_HEADER_LEN + longest).next_multiple_of(16);

        let mut out = Vec::new();
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.extend_from_slice(&(payloads.len() as u32).to_le_bytes());
        out.extend_from_slice(&(stride as u32).to_le_bytes());
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&[0; 12]);
        assert_eq!(out.len(), HEADER_LEN);

        for payload in payloads {
            let slot = out.len();
            out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            out.resize(slot + FRAME_HEADER_LEN, 0);
            out.extend_from_slice(payload);
            out.resize(slot + stride, 0);
        }
        out
    }

    /// A stand-in for one IPU frame: a picture header, some bytes, and the
    /// `00 00 01 B0` end marker every real frame closes on.
    fn frame(fill: u8, len: usize) -> Vec<u8> {
        let mut out = vec![0x00, 0x43, 0xf0, 0x0a];
        out.resize(len - 4, fill);
        out.extend_from_slice(&[0x00, 0x00, 0x01, 0xb0]);
        out
    }

    #[test]
    fn a_three_frame_file_round_trips() {
        let payloads = [frame(0x11, 100), frame(0x22, 260), frame(0x33, 40)];
        let refs: Vec<&[u8]> = payloads.iter().map(Vec::as_slice).collect();
        let blob = build(512, 512, &refs);

        let ipf = parse(&blob).expect("parsing");
        assert_eq!(ipf.header.width, 512);
        assert_eq!(ipf.header.height, 512);
        assert_eq!(ipf.header.frame_count, 3);
        assert_eq!(ipf.header.alignment, 16);
        assert_eq!(ipf.len(), 3);
        assert_eq!(ipf.frames, refs);
    }

    #[test]
    fn the_slot_stride_is_the_longest_payload_rounded_up() {
        // What the shipped files show: every slot is the size of the worst
        // frame, so a seek is a multiply. The short frames are mostly padding,
        // and the parser must not hand that padding to a decoder.
        let payloads = [frame(0x11, 40), frame(0x22, 260)];
        let refs: Vec<&[u8]> = payloads.iter().map(Vec::as_slice).collect();
        let blob = build(640, 448, &refs);

        let ipf = parse(&blob).expect("parsing");
        assert_eq!(
            ipf.header.frame_stride,
            (FRAME_HEADER_LEN + 260).next_multiple_of(16)
        );
        assert_eq!(
            ipf.frames[0].len(),
            40,
            "the padding is not part of the frame"
        );
        assert_eq!(ipf.frames[1].len(), 260);
        assert_eq!(blob.len(), HEADER_LEN + 2 * ipf.header.frame_stride);
    }

    #[test]
    fn concatenating_the_frames_drops_every_padding_byte() {
        let payloads = [frame(0x11, 40), frame(0x22, 260)];
        let refs: Vec<&[u8]> = payloads.iter().map(Vec::as_slice).collect();
        let blob = build(512, 512, &refs);

        let ipf = parse(&blob).expect("parsing");
        let bitstream = ipf.bitstream();
        assert_eq!(bitstream.len(), 300);
        assert_eq!(&bitstream[..40], payloads[0].as_slice());
        assert_eq!(&bitstream[40..], payloads[1].as_slice());
    }

    #[test]
    fn a_short_blob_is_refused() {
        assert_eq!(parse(b"IPU").unwrap_err(), Error::TooShort { got: 3 });
    }

    #[test]
    fn another_container_is_refused() {
        let mut blob = vec![0; HEADER_LEN];
        blob[..4].copy_from_slice(b"PSMF");
        assert_eq!(parse(&blob).unwrap_err(), Error::NotIpf);
    }

    #[test]
    fn a_truncated_file_says_how_much_is_missing() {
        let payloads = [frame(0x11, 100), frame(0x22, 100)];
        let refs: Vec<&[u8]> = payloads.iter().map(Vec::as_slice).collect();
        let mut blob = build(512, 512, &refs);
        let full = blob.len();
        blob.truncate(full - 1);

        assert_eq!(
            parse(&blob).unwrap_err(),
            Error::Truncated {
                want: full,
                got: full - 1
            }
        );
    }

    #[test]
    fn a_slot_claiming_more_than_it_holds_is_refused() {
        let payloads = [frame(0x11, 100)];
        let refs: Vec<&[u8]> = payloads.iter().map(Vec::as_slice).collect();
        let mut blob = build(512, 512, &refs);
        let claimed = 0xffffu32;
        blob[HEADER_LEN..HEADER_LEN + 4].copy_from_slice(&claimed.to_le_bytes());

        let stride = u32_at(&blob, 12) as usize;
        assert_eq!(
            parse(&blob).unwrap_err(),
            Error::PayloadTooLong {
                index: 0,
                claimed: claimed as usize,
                available: stride - FRAME_HEADER_LEN
            }
        );
    }

    #[test]
    fn a_zero_sized_picture_is_refused() {
        let payloads = [frame(0x11, 100)];
        let refs: Vec<&[u8]> = payloads.iter().map(Vec::as_slice).collect();
        let mut blob = build(512, 512, &refs);
        blob[6..8].copy_from_slice(&0u16.to_le_bytes());

        assert_eq!(
            parse(&blob).unwrap_err(),
            Error::ZeroDimension {
                width: 512,
                height: 0
            }
        );
    }

    #[test]
    fn a_stride_with_no_room_for_a_payload_is_refused() {
        let payloads = [frame(0x11, 100)];
        let refs: Vec<&[u8]> = payloads.iter().map(Vec::as_slice).collect();
        let mut blob = build(512, 512, &refs);
        blob[12..16].copy_from_slice(&(FRAME_HEADER_LEN as u32).to_le_bytes());

        assert_eq!(
            parse(&blob).unwrap_err(),
            Error::StrideTooShort {
                stride: FRAME_HEADER_LEN
            }
        );
    }
}
