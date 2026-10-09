//! AV1 decoding, for the movie cache and for any video the project ships.
//!
//! Wraps [`re_rav1d`], a Rust port of `dav1d`, and turns an [`crate::ivf`]
//! stream into tightly packed 8-bit I420 frames - the layout
//! `oag-render`'s video pipeline uploads as three `R8` planes.
//!
//! The decoder, [`FrameSource`], is behind the `av1` cargo feature, off by
//! default, so that `oag-formats` stays dependency-free for the tools that only
//! parse Wipeout containers. `oag-game` turns it on everywhere but
//! `wasm32`, where `re_rav1d` does not build (it needs `libc` types the
//! target lacks); [`Geometry`] and [`Error`] are always here, because every
//! movie decoder describes its frames with them.
//!
//! # Why this decoder
//!
//! [ADR-0008](../../../docs/architecture/adr/0008-av1-movie-cache.md) has the
//! reasoning. The short version: it is pure Rust, so no C toolchain enters the
//! build, and AV1 is royalty-free, which H.264 is not.
//!
//! The `asm` feature of `re_rav1d` is **off**: enabling it requires `nasm` at
//! build time, and a 480x272 movie decodes at roughly 900 frames per second
//! without it, thirty times faster than it is played back.
//!
//! # Sequential by design
//!
//! Frames are decoded in order. [`FrameSource::frame`] accepts any index, but
//! reaching a frame means decoding every frame before it, and going backwards
//! flushes the decoder and starts again from the beginning. That matches how
//! movies are actually played here - forwards, or looped back to the start -
//! and it is why no frame index is stored beyond the IVF's own frame list.

#[cfg(feature = "av1")]
use std::ops::Range;

#[cfg(feature = "av1")]
use re_rav1d::{Decoder, Picture, PixelLayout, PlanarImageComponent, Settings};

use crate::ivf;

/// Something wrong with an AV1 stream, or with decoding it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The container did not parse.
    Container(ivf::Error),
    /// The container holds something other than AV1.
    NotAv1 {
        /// The fourcc found.
        fourcc: [u8; 4],
    },
    /// The decoder could not be created or driven.
    Decoder {
        /// What the decoder said.
        message: String,
    },
    /// A frame was asked for that the stream does not have.
    OutOfRange {
        /// The index asked for.
        index: usize,
        /// How many frames there are.
        len: usize,
    },
    /// The stream ran out of pictures before the frame that was asked for.
    ///
    /// The frame list and the decoder disagreeing means the file is damaged, or
    /// holds frames that decode to nothing.
    UnexpectedEnd {
        /// The index asked for.
        index: usize,
        /// How many pictures came out.
        decoded: usize,
    },
    /// The pictures are not the 8-bit I420 the renderer expects.
    UnsupportedFormat {
        /// What the picture actually is.
        description: String,
    },
    /// A picture's dimensions are not the ones the container declared.
    SizeChanged {
        /// What the header said.
        expected: (u32, u32),
        /// What the picture was.
        found: (u32, u32),
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Container(e) => write!(f, "the IVF container: {e}"),
            Self::NotAv1 { fourcc } => {
                write!(f, "fourcc {} is not AV01", String::from_utf8_lossy(fourcc))
            }
            Self::Decoder { message } => write!(f, "the AV1 decoder: {message}"),
            Self::OutOfRange { index, len } => {
                write!(f, "frame {index} asked for, but the stream holds {len}")
            }
            Self::UnexpectedEnd { index, decoded } => write!(
                f,
                "the stream ended after {decoded} picture(s), before frame {index}"
            ),
            Self::UnsupportedFormat { description } => {
                write!(f, "expected 8-bit I420, found {description}")
            }
            Self::SizeChanged { expected, found } => write!(
                f,
                "expected {}x{} frames, found {}x{}",
                expected.0, expected.1, found.0, found.1
            ),
        }
    }
}

impl std::error::Error for Error {}

impl From<ivf::Error> for Error {
    fn from(e: ivf::Error) -> Self {
        Self::Container(e)
    }
}

/// The shape of one decoded frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    /// Luma width in samples, and the frame width in pixels.
    pub width: u32,
    /// Luma height in samples, and the frame height in pixels.
    pub height: u32,
    /// Chroma plane width in samples: half the luma width, rounded up.
    pub chroma_width: u32,
    /// Chroma plane height in samples: half the luma height, rounded up.
    pub chroma_height: u32,
}

impl Geometry {
    /// The geometry implied by a frame size, for 4:2:0 chroma.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            chroma_width: width.div_ceil(2),
            chroma_height: height.div_ceil(2),
        }
    }

    /// Bytes in the luma plane.
    #[must_use]
    pub fn luma_len(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// Bytes in each chroma plane.
    #[must_use]
    pub fn chroma_len(&self) -> usize {
        (self.chroma_width * self.chroma_height) as usize
    }

    /// Bytes in a whole frame: luma followed by both chroma planes.
    #[must_use]
    pub fn frame_len(&self) -> usize {
        self.luma_len() + 2 * self.chroma_len()
    }
}

/// Decodes an AV1-in-IVF stream, one frame at a time.
#[cfg(feature = "av1")]
pub struct FrameSource {
    blob: Vec<u8>,
    /// Payload ranges into `blob`, in stream order.
    payloads: Vec<Range<usize>>,
    decoder: Decoder,
    geometry: Geometry,
    /// The frame index the next decoded picture will be.
    next: usize,
    /// How many payloads have been handed to the decoder.
    sent: usize,
    /// Whether the decoder has been told there is no more input.
    flushed: bool,
}

// `re_rav1d::Decoder` is not `Debug`, and the workspace warns on types that are
// not. Everything worth printing is ours anyway.
#[cfg(feature = "av1")]
impl std::fmt::Debug for FrameSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameSource")
            .field("frames", &self.payloads.len())
            .field("bytes", &self.blob.len())
            .field("geometry", &self.geometry)
            .field("next", &self.next)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "av1")]
impl FrameSource {
    /// Takes ownership of an IVF blob and prepares to decode it.
    ///
    /// Nothing is decoded here: the frame list comes from the container, so the
    /// length is known without touching the decoder.
    ///
    /// # Errors
    ///
    /// If the container does not parse, does not hold AV1, or the decoder
    /// cannot be created.
    pub fn new(blob: Vec<u8>) -> Result<Self, Error> {
        let (payloads, geometry) = {
            let parsed = ivf::parse(&blob)?;
            if !parsed.header.is_av1() {
                return Err(Error::NotAv1 {
                    fourcc: parsed.header.fourcc,
                });
            }
            let payloads = parsed
                .frames
                .iter()
                .map(|f| f.offset..f.offset + f.data.len())
                .collect::<Vec<_>>();
            (
                payloads,
                Geometry::new(
                    u32::from(parsed.header.width),
                    u32::from(parsed.header.height),
                ),
            )
        };

        // One thread and the shortest frame delay: this decodes far faster than
        // playback needs, and a low delay keeps "decode one more frame" from
        // having to pump a pipeline.
        let mut settings = Settings::new();
        settings.set_n_threads(1);
        settings.set_max_frame_delay(1);

        let decoder = Decoder::with_settings(&settings).map_err(|e| Error::Decoder {
            message: e.to_string(),
        })?;

        Ok(Self {
            blob,
            payloads,
            decoder,
            geometry,
            next: 0,
            sent: 0,
            flushed: false,
        })
    }

    /// How many frames the container holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.payloads.len()
    }

    /// Whether the stream holds no frames at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.payloads.is_empty()
    }

    /// The frame shape, from the container header.
    #[must_use]
    pub fn geometry(&self) -> Geometry {
        self.geometry
    }

    /// Decodes frame `index` into `out`, which is resized to one whole frame.
    ///
    /// Going forwards decodes only the frames in between. Going backwards
    /// flushes and restarts from frame zero, which is the cost of a stream with
    /// inter-frame prediction and no index.
    ///
    /// # Errors
    ///
    /// If `index` is past the end, if the decoder fails, or if a picture is not
    /// the 8-bit I420 the container's geometry describes.
    pub fn frame(&mut self, index: usize, out: &mut Vec<u8>) -> Result<(), Error> {
        if index >= self.len() {
            return Err(Error::OutOfRange {
                index,
                len: self.len(),
            });
        }
        if index < self.next {
            self.rewind();
        }

        while self.next <= index {
            let picture = self.decode_one()?.ok_or(Error::UnexpectedEnd {
                index,
                decoded: self.next,
            })?;
            if self.next == index {
                self.write_frame(&picture, out)?;
            }
            self.next += 1;
        }
        Ok(())
    }

    /// Drops decoder state and starts again from frame zero.
    pub fn rewind(&mut self) {
        self.decoder.flush();
        self.next = 0;
        self.sent = 0;
        self.flushed = false;
    }

    /// Pulls the next picture out of the decoder, feeding it as needed.
    ///
    /// Returns `None` once the stream is exhausted.
    fn decode_one(&mut self) -> Result<Option<Picture>, Error> {
        loop {
            match self.decoder.get_picture() {
                Ok(picture) => return Ok(Some(picture)),
                Err(re_rav1d::Error::Again) => {}
                Err(e) => {
                    return Err(Error::Decoder {
                        message: e.to_string(),
                    });
                }
            }

            if self.sent >= self.payloads.len() {
                // No input left. Tell the decoder so, then let the next
                // `get_picture` either drain a straggler or report the end.
                if self.flushed {
                    return Ok(None);
                }
                match self.decoder.send_pending_data() {
                    Ok(()) => self.flushed = true,
                    Err(re_rav1d::Error::Again) => {}
                    Err(e) => {
                        return Err(Error::Decoder {
                            message: e.to_string(),
                        });
                    }
                }
                continue;
            }

            let payload = self.blob[self.payloads[self.sent].clone()].to_vec();
            match self.decoder.send_data(payload, None, None, None) {
                Ok(()) => self.sent += 1,
                // Pictures are waiting; the next loop collects one.
                Err(re_rav1d::Error::Again) => {}
                Err(e) => {
                    return Err(Error::Decoder {
                        message: e.to_string(),
                    });
                }
            }
        }
    }

    /// Copies a picture into `out` as tightly packed I420.
    fn write_frame(&self, picture: &Picture, out: &mut Vec<u8>) -> Result<(), Error> {
        if picture.pixel_layout() != PixelLayout::I420 || picture.bit_depth() != 8 {
            return Err(Error::UnsupportedFormat {
                description: format!(
                    "{:?} at {} bit(s)",
                    picture.pixel_layout(),
                    picture.bit_depth()
                ),
            });
        }
        let found = (picture.width(), picture.height());
        if found != (self.geometry.width, self.geometry.height) {
            return Err(Error::SizeChanged {
                expected: (self.geometry.width, self.geometry.height),
                found,
            });
        }

        out.clear();
        out.reserve(self.geometry.frame_len());
        for component in [
            PlanarImageComponent::Y,
            PlanarImageComponent::U,
            PlanarImageComponent::V,
        ] {
            let (width, height) = match component {
                PlanarImageComponent::Y => (self.geometry.width, self.geometry.height),
                _ => (self.geometry.chroma_width, self.geometry.chroma_height),
            };
            // `Picture::plane_data_geometry` returns the *stride*, not the
            // width. Copying stride-wide rows would silently pad every frame,
            // so the width comes from the geometry and the stride only from
            // `stride`.
            let stride = picture.stride(component) as usize;
            let plane = picture.plane(component);
            let bytes: &[u8] = plane.as_ref();
            for row in 0..height as usize {
                let start = row * stride;
                let end = start + width as usize;
                let Some(slice) = bytes.get(start..end) else {
                    return Err(Error::UnsupportedFormat {
                        description: format!(
                            "a {width}x{height} plane with stride {stride} in {} byte(s)",
                            bytes.len()
                        ),
                    });
                };
                out.extend_from_slice(slice);
            }
        }
        Ok(())
    }
}

/// A decoder must be movable to another thread, and this fails the build if it
/// stops being.
///
/// Not idle pedantry: `oag_game::movie::Feed` moves a whole [`FrameSource`] onto
/// a worker thread so that decoding a frame - up to 30 ms - does not happen on
/// the thread that draws. See
/// [ADR-0010](../../../docs/architecture/adr/0010-movie-decode-thread.md). If a
/// future `re_rav1d` made `Decoder` thread-bound, the error would otherwise land
/// in `oag-game` as a confusing `Send` failure inside a closure rather than here,
/// next to the decoder it is a fact about.
#[cfg(feature = "av1")]
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<FrameSource>();
};

#[cfg(all(test, feature = "av1"))]
mod tests {
    use super::*;

    /// Twelve frames of 64x64 `ffmpeg` test pattern, encoded losslessly with the
    /// same flags the movie cache uses.
    ///
    /// Synthetic and ours: it is generated from `testsrc`, so it carries no game
    /// content and can be committed. It exists so the decoder, and above all the
    /// rewind path, are covered by tests that run in CI without a disc image.
    const TESTSRC: &[u8] = include_bytes!("../tests/data/testsrc-64x64.ivf");

    fn source() -> FrameSource {
        FrameSource::new(TESTSRC.to_vec()).expect("the fixture decodes")
    }

    #[test]
    fn reads_the_stream_geometry_without_decoding() {
        let src = source();
        assert_eq!(src.len(), 12);
        assert!(!src.is_empty());
        assert_eq!(src.geometry(), Geometry::new(64, 64));
    }

    #[test]
    fn decodes_frames_in_order() {
        let mut src = source();
        let mut frame = Vec::new();
        for index in 0..src.len() {
            src.frame(index, &mut frame).expect("decoding forwards");
            assert_eq!(
                frame.len(),
                src.geometry().frame_len(),
                "frame {index} is the wrong size"
            );
        }
    }

    /// Frames must not all be identical, or every other test here would pass on
    /// a decoder that returned the same picture forever.
    #[test]
    fn frames_differ_from_one_another() {
        let mut src = source();
        let (mut first, mut last) = (Vec::new(), Vec::new());
        src.frame(0, &mut first).unwrap();
        src.frame(11, &mut last).unwrap();
        assert_ne!(first, last);
    }

    /// Going backwards flushes the decoder and restarts. This is what a looping
    /// movie does at every wrap, and it must return the same bytes it did the
    /// first time round.
    #[test]
    fn rewinding_reproduces_earlier_frames() {
        let mut src = source();
        let (mut forwards, mut backwards) = (Vec::new(), Vec::new());

        src.frame(2, &mut forwards).unwrap();
        src.frame(9, &mut Vec::new()).unwrap();
        // Now behind the cursor: this is the rewind path.
        src.frame(2, &mut backwards).unwrap();
        assert_eq!(forwards, backwards);

        // And the stream still runs forwards afterwards.
        src.frame(11, &mut backwards).unwrap();
        assert_eq!(backwards.len(), src.geometry().frame_len());
    }

    /// Reading the same frame twice must not advance anything.
    #[test]
    fn rereading_a_frame_is_stable() {
        let mut src = source();
        let (mut once, mut twice) = (Vec::new(), Vec::new());
        src.frame(5, &mut once).unwrap();
        src.frame(5, &mut twice).unwrap();
        assert_eq!(once, twice);
    }

    /// Skipping forwards must land on the same frame a walk would.
    #[test]
    fn seeking_forwards_matches_walking() {
        let mut walked = Vec::new();
        let mut src = source();
        for index in 0..=7 {
            src.frame(index, &mut walked).unwrap();
        }

        let mut jumped = Vec::new();
        source().frame(7, &mut jumped).unwrap();
        assert_eq!(walked, jumped);
    }

    #[test]
    fn a_frame_past_the_end_is_an_error() {
        let mut src = source();
        assert_eq!(
            src.frame(12, &mut Vec::new()),
            Err(Error::OutOfRange { index: 12, len: 12 })
        );
    }

    #[test]
    fn geometry_halves_chroma_rounding_up() {
        let g = Geometry::new(480, 272);
        assert_eq!((g.chroma_width, g.chroma_height), (240, 136));
        assert_eq!(g.luma_len(), 130_560);
        assert_eq!(g.chroma_len(), 32_640);
        assert_eq!(g.frame_len(), 195_840);

        let odd = Geometry::new(15, 9);
        assert_eq!((odd.chroma_width, odd.chroma_height), (8, 5));
    }

    #[test]
    fn a_non_av1_container_is_rejected() {
        let mut blob = Vec::new();
        blob.extend_from_slice(&ivf::SIGNATURE);
        blob.extend_from_slice(&0u16.to_le_bytes());
        blob.extend_from_slice(&(ivf::FILE_HEADER_LEN as u16).to_le_bytes());
        blob.extend_from_slice(b"VP80");
        blob.extend_from_slice(&320u16.to_le_bytes());
        blob.extend_from_slice(&240u16.to_le_bytes());
        blob.extend_from_slice(&30u32.to_le_bytes());
        blob.extend_from_slice(&1u32.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());
        blob.extend_from_slice(&0u32.to_le_bytes());

        assert!(matches!(
            FrameSource::new(blob),
            Err(Error::NotAv1 { fourcc }) if fourcc == *b"VP80"
        ));
    }

    #[test]
    fn a_damaged_container_is_reported_as_such() {
        let blob = b"not an ivf file at all, not even close!!".to_vec();
        assert!(matches!(
            FrameSource::new(blob),
            Err(Error::Container(ivf::Error::NotIvf))
        ));
    }
}
