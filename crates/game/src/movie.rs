//! Movie playback: demux here, transcode out of process, decode in process.
//!
//! A `.PMF` holds H.264 video and ATRAC3+ audio. Demuxing it is ours to do and
//! lives in [`oag_formats::pmf`]. Decoding **H.264** is not: per
//! `docs/architecture/adr/0004-asset-pipeline.md`, the original is converted
//! once and cached, and the conversion runs out of process through `ffmpeg`, so
//! no H.264 decoder ships in the workspace.
//!
//! What the cache holds is **lossless AV1 in an IVF container**, not raw
//! frames: see `docs/architecture/adr/0008-av1-movie-cache.md`. Lossless, so
//! the picture is bit-for-bit what the raw cache used to hold and ADR-0004's
//! fidelity rule is untouched; AV1, because [`oag_formats::av1`] decodes it in
//! process with no C toolchain. The intro's cache goes from 48.8 MiB to
//! 1.17 MiB that way.
//!
//! When `ffmpeg` is absent, playback still happens: the player advances its
//! frame counter over the movie's real duration and the renderer draws the black
//! backdrop the `LogoFMV` screen puts behind the movie anyway. That keeps the
//! **sequencing** faithful, which is what the intro state machine actually
//! depends on, and it is honest about having no picture.
//!
//! # Three pieces, and which thread each runs on
//!
//! | | What it is | Thread |
//! | --- | --- | --- |
//! | [`Player`] | A frame counter and the pacing rules. No pixels. | The one that ticks |
//! | [`FrameStore`] | One synchronous `read_frame`, straight off the decoder. | Whoever calls it |
//! | [`Feed`] | A [`FrameStore`] on a worker thread, decoding ahead into a ring. | Its own |
//!
//! [`Player`] and [`Feed`] are deliberately separate and talk only through a
//! **position** - see [`Player::position`]. The player says where playback has
//! got to; the feed says which decoded frames it has. Nothing about the feed
//! feeds back into the player, which is what makes moving the decode off the
//! render thread safe - see [ADR-0010](../../../docs/architecture/adr/0010-movie-decode-thread.md).
//!
//! See `docs/architecture/frontend-boot.md`.

#[cfg(all(target_os = "linux", feature = "native-video"))]
mod gst;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};

use anyhow::{Context, Result, anyhow, bail};
use oag_formats::{av1, ipf, pmf};

use crate::at3;

/// The rate the PSP presents `.PMF` frames at, as a rational.
///
/// PS2 movies are not this rate: a raw MPEG-2 program stream carries its own,
/// read with `ffprobe` in [`open_mpeg2_ps`] rather than assumed. [`Movie`]
/// carries whichever rate is correct for the file it came from, and
/// [`Player`] paces against that rather than a single constant.
pub const FRAME_RATE: (u64, u64) = (30_000, 1001);

/// Colour format the cache decodes to: planar 8-bit YUV, chroma at half
/// resolution in both axes.
///
/// Chosen because it is what the H.264 decoder produces natively, so neither
/// the transcode nor the AV1 decode does any colour conversion, and it is
/// 1.5 bytes per pixel rather than 4. The conversion to RGB happens in the
/// fragment shader.
pub const PIXEL_FORMAT: &str = "yuv420p";

/// Identifies the cache encoding in a cache file's name.
///
/// It is in the **filename**, not just the contents, so that changing the
/// encoder or its settings invalidates by name and cannot silently reuse an
/// older file. That matters more than it looks: `ffmpeg`'s `-lossless 1` on its
/// own is silently ignored and produces a lossy encode, so a cache written
/// before the flags were right must never be mistaken for a good one.
const CACHE_CODEC: &str = "av1ll";

/// The name a converted movie takes under the cache directory.
///
/// One function rather than a `format!` at each of the three transcode paths,
/// because [`crate::prefetch`] has to be able to ask "is this one already
/// done?" without transcoding it, and a fourth copy of the pattern would be a
/// fourth chance to disagree with the three that write it.
///
/// `frames` is how many pictures the file holds; `None` is the whole input,
/// which only [`transcode_mpeg2_ps`] ever writes - the other two resolve
/// [`Extent::Whole`] against a frame count they already know.
#[must_use]
pub fn cache_name(key: &str, width: u32, height: u32, frames: Option<usize>) -> String {
    format!(
        "{key}-{width}x{height}-{CACHE_CODEC}-{}.ivf",
        frames.map_or_else(|| "all".to_string(), |n| n.to_string())
    )
}

/// Bytes of sub-header on every audio PES payload in a `.PMF`.
///
/// `pmf::demux` strips the PES header itself and hands over what follows, and
/// what follows is **not** an ATRAC3+ frame: it is four bytes and then a slice
/// of the frame stream. The first two are zero on every packet of every movie
/// on the disc; the third and fourth are a big-endian offset from the end of
/// this header to the first frame that *starts* inside the packet, the bytes
/// before it being the tail of the frame the previous packet began.
///
/// **Measured, on all 323 packets of `Intro.PMF` and every other movie with a
/// track.** Reading that offset lands on the ATRAC3+ sync word 323 times out of
/// 323, and it is what confirms the field is a pointer rather than a counter:
/// packet 0 carries two whole 752-byte frames and 509 bytes of a third, and
/// packet 1's offset is the 243 bytes that complete it.
///
/// Nothing here needs the pointer to *reassemble* the stream - concatenating
/// every packet's payload in order gives the frames back contiguously - but the
/// first packet's is used, because a movie whose first frame does not begin at
/// offset zero would otherwise be decoded half a frame out.
const AUDIO_PES_HEADER_LEN: usize = 4;

/// The sync word every ATRAC3+ frame inside a `.PMF` begins with.
///
/// **A RIFF-wrapped `.at3` has no such word**: `PSP_GAME/SND0.AT3`'s data chunk
/// starts straight in on the codec payload, and a scan of all 25,760 bytes of
/// it finds `0f d0` nowhere. So this is the `.PMF`'s framing rather than the
/// codec's, and it has to come off before `ffmpeg` will read a block.
const ATRAC3PLUS_SYNC: [u8; 2] = [0x0f, 0xd0];

/// Bytes of header on every ATRAC3+ frame inside a `.PMF`, sync word included.
///
/// `0f d0` then the codec config word - see `at3::codec_config`, which the
/// disc's own `.at3` entries carry in the same shape - then four zero bytes.
/// Constant across all 865 frames of `Intro.PMF` and every frame of every other
/// movie with a track.
///
/// The evidence that it is exactly eight is that stripping eight leaves a block
/// that decodes: `Intro.PMF`'s frames are 752 bytes apart, 752 - 8 is 744, and
/// 744-byte blocks decode to 865 whole blocks of 2,048 samples with no
/// remainder. It also lines the payload up with what a `.at3` stores - both
/// begin `3a` - where stripping only the two-byte sync word does not, and
/// `ffmpeg` rejects that with "frame data doesn't match channel configuration"
/// rather than decoding noise.
const ATRAC3PLUS_FRAME_HEADER_LEN: usize = 8;

/// A movie's ATRAC3+ track, unwrapped from the container but not yet decoded.
///
/// Held rather than decoded on the spot because decoding shells out to `ffmpeg`
/// and lands in a **different** cache from the one the pictures use - see
/// [`crate::boot::default_audio_cache_dir`] - and [`open`] is given only the
/// movie cache. Keeping the two apart is also what lets `--prefetch` and the
/// viewer open a movie without ever paying for its sound.
pub struct MovieAudio {
    /// Every ATRAC3+ block, headers off, back to back. Exactly what
    /// [`crate::at3::riff`] wants for its `data` chunk.
    blocks: Vec<u8>,
    /// What those blocks are, for the RIFF wrapper.
    pub format: crate::at3::Format,
}

// Written out rather than derived for the reason `crate::audio::Dump` gives:
// this is most of a megabyte of codec payload, and a `{:?}` of a `Movie` should
// say how much there is rather than print it.
impl std::fmt::Debug for MovieAudio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MovieAudio")
            .field("blocks", &self.blocks.len())
            .field("format", &self.format)
            .finish()
    }
}

impl MovieAudio {
    /// How many whole ATRAC3+ blocks the track holds.
    #[must_use]
    pub fn block_count(&self) -> usize {
        self.blocks.len() / usize::from(self.format.block_align).max(1)
    }

    /// How long the track is, in seconds, at block granularity.
    ///
    /// **This is not the movie's own duration and should not be expected to
    /// match it.** A block is 2,048 samples whatever the encoder had left to
    /// put in it, so a track always runs to the end of a whole number of them:
    /// `Intro.PMF` declares 40.04 s and its 865 blocks are 40.17 s. The
    /// difference is padding, not a demux that ran long.
    #[must_use]
    pub fn seconds(&self) -> f64 {
        let samples = self.block_count() as f64 * f64::from(at3::SAMPLES_PER_BLOCK);
        samples / f64::from(self.format.sample_rate.max(1))
    }

    /// Decodes the track through `ffmpeg` and the cache.
    ///
    /// # Errors
    ///
    /// As [`crate::at3::decode_frames`]: an `ffmpeg` that is absent or fails,
    /// or a cache file that will not write or read back.
    pub fn decode(&self, cache_dir: &Path) -> Result<at3::Pcm> {
        at3::decode_frames(&self.blocks, self.format, cache_dir)
    }
}

/// Unwraps a demuxed `.PMF`'s audio packets into ATRAC3+ blocks.
///
/// Two layers come off, and both are the container's rather than the codec's:
/// [`AUDIO_PES_HEADER_LEN`] bytes on each packet, then
/// [`ATRAC3PLUS_FRAME_HEADER_LEN`] on each frame.
///
/// `block_align` is **measured rather than assumed**, because the disc uses two
/// values: the two 40-second reels are 744 bytes a block and the other seven
/// tracks are 560. It is the distance between the first two sync words, which
/// is then checked against every remaining frame - a stream whose sync words are
/// not evenly spaced is one this has read wrong, and saying so is better than
/// handing `ffmpeg` a block size that decodes into noise.
///
/// `None` means there is no track to play: a header that declares audio but a
/// demux that found no packets, a sample-rate code this build does not know, or
/// a stream whose framing does not check out. Never an error, because none of
/// those should cost a movie its picture.
fn movie_audio(demuxed: &pmf::Demuxed, stream: pmf::AudioStream, key: &str) -> Option<MovieAudio> {
    let first = demuxed.audio.first()?;
    let Some(sample_rate) = stream.frequency_hz() else {
        eprintln!(
            "warning: {key}'s audio is sample-rate code {}, which this build does not know, so \
             it stays silent",
            stream.frequency_code
        );
        return None;
    };

    // Where the first whole frame starts, past whatever tail of a previous one
    // the packet opens with. Zero on every movie on the disc, and read anyway
    // rather than assumed: it is the one field that tells us.
    //
    // Read through `get`, because `pmf::demux` hands over whatever followed the
    // PES header and that can be fewer than four bytes on a truncated stream -
    // and a movie that loses its sound must not also lose its picture to a
    // panic.
    let pointer: [u8; 2] = first.get(2..4).and_then(|b| b.try_into().ok())?;
    let start = usize::from(u16::from_be_bytes(pointer));

    let mut stream_bytes = Vec::new();
    for packet in &demuxed.audio {
        stream_bytes.extend_from_slice(&packet[AUDIO_PES_HEADER_LEN.min(packet.len())..]);
    }
    let body = stream_bytes.get(start..).unwrap_or_default();

    if !body.starts_with(&ATRAC3PLUS_SYNC) {
        eprintln!("warning: {key}'s first audio frame carries no sync word, so it stays silent");
        return None;
    }

    // Stepped by eight because a frame is always a whole number of eight-byte
    // groups - `block_align / 8 - 1` is what the config word stores - which
    // keeps a `0f d0` that happens to fall inside codec payload from being
    // mistaken for the next frame.
    let stride = (ATRAC3PLUS_FRAME_HEADER_LEN..body.len().saturating_sub(1))
        .step_by(8)
        .find(|&at| body[at..at + 2] == ATRAC3PLUS_SYNC)?;
    let Ok(block_align) = u16::try_from(stride - ATRAC3PLUS_FRAME_HEADER_LEN) else {
        return None;
    };

    // A trailing partial frame is dropped rather than padded: the movie's own
    // duration is carried by its PTS range, so a fragment of a block would add
    // noise at the end and nothing else.
    let frames = body.len() / stride;
    let mut blocks = Vec::with_capacity(frames * usize::from(block_align));
    for index in 0..frames {
        let at = index * stride;
        if body[at..at + 2] != ATRAC3PLUS_SYNC {
            eprintln!(
                "warning: {key}'s audio loses framing at frame {index} of {frames}, so it stays \
                 silent"
            );
            return None;
        }
        blocks.extend_from_slice(&body[at + ATRAC3PLUS_FRAME_HEADER_LEN..at + stride]);
    }

    Some(MovieAudio {
        blocks,
        format: crate::at3::Format {
            channels: u16::from(stream.channels),
            sample_rate,
            block_align,
        },
    })
}

/// A movie that has been demuxed, and possibly transcoded.
#[derive(Debug)]
pub struct Movie {
    /// The PSMF header, for a `.PMF`. `None` for the PS2's loose files: a raw
    /// MPEG-2 program stream (`.PSS`, see [`open_mpeg2_ps`]) carries no such
    /// header, and an `.IPF` has one of its own shape (see [`open_ipuf`]).
    pub header: Option<pmf::Header>,
    /// Frames measured by counting H.264 access units for a `.PMF`, decoded
    /// by `ffmpeg` for a raw MPEG-2 program stream, or read straight off the
    /// container for an `.IPF`.
    pub frame_count: usize,
    /// Video width in pixels.
    pub width: u32,
    /// Video height in pixels.
    pub height: u32,
    /// The rate this movie presents frames at. A `.PMF` is always
    /// [`FRAME_RATE`]; a PS2 `.PSS` carries its own, read by `ffprobe`.
    pub frame_rate: (u64, u64),
    /// The picture's intended display aspect ratio, as `(width, height)`.
    ///
    /// A `.PMF`'s pixels are square, so this is just `(width, height)` and
    /// changes nothing. A PS2 `.PSS` is not: `INTRO512.PSS` decodes to a
    /// square 512x512 but samples are non-square, and its own display aspect
    /// is 4:3 - drawing it stretched into a 480x272 (~16:9) box would squash
    /// it. Read by `ffprobe` alongside width and height; see [`probe`].
    pub display_aspect: (u32, u32),
    /// Where the decoded frames are, if a transcode happened.
    pub frames: Option<FrameStore>,
    /// Why there is no picture, when there is none.
    pub no_picture_reason: Option<String>,
    /// The movie's own ATRAC3+ track, undecoded.
    ///
    /// `None` on a movie with no audio stream - which `Backdrop.PMF` is, and it
    /// is the movie that plays most - and on the PS2's two containers, neither
    /// of which carries one. Decoding it is [`MovieAudio::decode`]'s job and
    /// deliberately not this module's: see [`MovieAudio`].
    pub audio: Option<MovieAudio>,
}

/// Pixel format a decoded [`VideoFrame`]'s planes are laid out in.
///
/// `I420` is the only variant produced by anything in this file today - see
/// [`PIXEL_FORMAT`] - but a hardware decoder's native output is commonly
/// `Nv12` (interleaved chroma), and this exists so a future [`VideoDecoder`]
/// can report what it actually decoded instead of being forced to
/// deinterleave before it can report anything. Nothing here converts one into
/// the other: that is each implementation's own job, which is what keeps
/// `video.wgsl` at one format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    /// Planar 4:2:0: Y, then U, then V, each tightly packed.
    #[default]
    I420,
    /// Semi-planar 4:2:0: Y, then interleaved UV, tightly packed. Not
    /// produced by anything in this file yet.
    Nv12,
}

/// One decoded frame: its pixel format, its shape, and its bytes.
///
/// Tightly packed in format order (`I420`: Y, U, V; `Nv12`: Y, UV) - no
/// stride padding, matching what [`av1::FrameSource::frame`] has always
/// produced. `Renderer::upload_frame` is the only reader.
#[derive(Debug, Clone, Default)]
pub struct VideoFrame {
    /// How `bytes` is laid out.
    pub format: PixelFormat,
    /// Luma width in samples, and the frame width in pixels.
    pub width: u32,
    /// Luma height in samples, and the frame height in pixels.
    pub height: u32,
    /// Chroma plane width in samples.
    pub chroma_width: u32,
    /// Chroma plane height in samples.
    pub chroma_height: u32,
    /// The plane data.
    pub bytes: Vec<u8>,
}

/// Something that can decode a movie's frames, in order, by index.
///
/// One implementation exists today, [`Av1CacheDecoder`] - a thin wrapper over
/// [`av1::FrameSource`], so the AV1 cache pipeline's behaviour is unchanged.
/// The trait exists so [`FrameStore`] can hold a decoder without knowing
/// which one, the same way `oag-disc`'s `SectorSource` trait lets a
/// `DiscImage` hold a sector source without knowing whether it is a CHD or a
/// raw ISO.
///
/// `Debug` is a supertrait for the same reason `SectorSource` makes it one: a
/// blanket `Debug` for `Box<dyn VideoDecoder>` would overlap the standard
/// library's impl for `Box<T: Debug>`, so every implementor derives or writes
/// its own instead. `Send` is a supertrait because [`Feed::spawn`] moves the
/// decoder onto a worker thread.
pub trait VideoDecoder: std::fmt::Debug + Send {
    /// A short, human-readable name for what is actually decoding - `"av1
    /// cache"`, `"gstreamer"`. Exists for the `dev` performance overlay (see
    /// [`crate::perf`]), so a player or a screenshot can tell which tier
    /// served a given movie without reading a log.
    fn label(&self) -> &'static str;

    /// The frame shape this decoder produces.
    fn geometry(&self) -> av1::Geometry;

    /// How many frames the movie holds.
    fn len(&self) -> usize;

    /// Whether the movie holds no frames at all.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Decodes frame `index` into `out`.
    ///
    /// `out`'s format and geometry are set as soon as decoding starts, before
    /// this can fail, so a caller that reuses one `VideoFrame` across many
    /// calls never sees stale metadata beside a fresh error - only `out.bytes`
    /// is unspecified when this returns `Err`.
    ///
    /// Playback here is always forwards, or a rewind to frame zero on a loop
    /// (see [`frame_at`]), so an implementation is free to treat `index`
    /// sequentially rather than supporting true random access.
    fn frame(&mut self, index: usize, out: &mut VideoFrame) -> Result<()>;

    /// Drops decoder state and starts again from frame zero.
    fn rewind(&mut self);
}

/// Decodes the AV1 movie cache through [`VideoDecoder`].
///
/// A wrapper rather than an inherent impl so [`av1::FrameSource`] stays free
/// of anything `oag-game` specific - `oag-formats` builds without `oag-game`
/// in the tree.
#[derive(Debug)]
struct Av1CacheDecoder(av1::FrameSource);

impl VideoDecoder for Av1CacheDecoder {
    fn label(&self) -> &'static str {
        "av1 cache"
    }

    fn geometry(&self) -> av1::Geometry {
        self.0.geometry()
    }

    fn len(&self) -> usize {
        self.0.len()
    }

    fn frame(&mut self, index: usize, out: &mut VideoFrame) -> Result<()> {
        // Set before the decode, which is what keeps this call's own error
        // from leaving a caller that reuses `out` holding a previous frame's
        // metadata beside whatever `bytes` was left in.
        let geometry = self.0.geometry();
        out.format = PixelFormat::I420;
        out.width = geometry.width;
        out.height = geometry.height;
        out.chroma_width = geometry.chroma_width;
        out.chroma_height = geometry.chroma_height;
        self.0.frame(index, &mut out.bytes)?;
        Ok(())
    }

    fn rewind(&mut self) {
        self.0.rewind();
    }
}

/// The cached movie, decoded a frame at a time.
///
/// Playback is sequential, so this decodes forward and only rewinds when asked
/// for a frame it has already passed - which is what a looping movie like the
/// menu backdrop does at every wrap.
///
/// **Every call here blocks for as long as the decode takes**, which on the PSP
/// backdrop is anything from 0.03 ms to 30 ms. That is fine off the render
/// thread - a headless capture wanting one exact frame, or the worker inside a
/// [`Feed`] - and is why anything drawing in a loop uses a [`Feed`] instead.
#[derive(Debug)]
pub struct FrameStore {
    path: PathBuf,
    source: Box<dyn VideoDecoder>,
    /// How many frames the cache holds.
    pub len: usize,
    /// Bytes in the luma plane.
    pub luma_len: usize,
    /// Bytes in each chroma plane.
    pub chroma_len: usize,
    /// Chroma plane width in samples.
    pub chroma_width: u32,
    /// Chroma plane height in samples.
    pub chroma_height: u32,
}

impl FrameStore {
    /// Opens a cached AV1 movie and checks it is the size the caller expects.
    fn open(path: PathBuf, width: u32, height: u32) -> Result<Self> {
        let blob = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let source = av1::FrameSource::new(blob)
            .map_err(|e| anyhow!("{} is not usable: {e}", path.display()))?;

        let geometry = source.geometry();
        if (geometry.width, geometry.height) != (width, height) {
            bail!(
                "{} holds {}x{} frames, but the movie declares {width}x{height}",
                path.display(),
                geometry.width,
                geometry.height
            );
        }

        Ok(Self {
            len: source.len(),
            luma_len: geometry.luma_len(),
            chroma_len: geometry.chroma_len(),
            chroma_width: geometry.chroma_width,
            chroma_height: geometry.chroma_height,
            source: Box::new(Av1CacheDecoder(source)),
            path,
        })
    }

    /// Decodes frame `index` into `out`.
    pub fn read_frame(&mut self, index: usize, out: &mut VideoFrame) -> Result<()> {
        self.source
            .frame(index, out)
            .with_context(|| format!("decoding frame {index} of {}", self.path.display()))
    }

    /// Where the cache file lives.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// What is actually decoding this movie - `"av1 cache"` or `"gstreamer"`.
    /// See [`VideoDecoder::label`].
    #[must_use]
    pub fn decoder_label(&self) -> &'static str {
        self.source.label()
    }
}

/// One decoded frame, and where in the playback order it belongs.
///
/// `position` is the [`Player::position`] this frame is the picture for, and it
/// is what a consumer compares against. `index` is the frame's index *within the
/// movie* and is only the same number until a loop wraps.
#[derive(Debug)]
pub struct Frame {
    /// How many frames into playback this one is, counting every loop.
    pub position: u64,
    /// Which frame of the movie it is, counting from zero.
    pub index: usize,
    /// The decoded picture.
    pub picture: VideoFrame,
}

/// Which frame of a `len`-frame movie is the picture at `position`.
///
/// A loop wraps: position 270 of a 270-frame movie is frame 0 again, which is
/// exactly what makes the wrap cheap. [`av1::FrameSource`] rewinding to frame 0
/// costs a decoder flush plus **one** frame's decode, because frame 0 is the
/// very next frame wanted - unlike a seek backwards into the middle of a movie,
/// which has to re-decode everything before it.
///
/// `None` means a movie that does not repeat has run out: the intro ends rather
/// than starting again.
fn frame_at(position: u64, len: usize, repeat: bool) -> Option<usize> {
    if len == 0 {
        return None;
    }
    if repeat {
        // `len` came from a `usize`, so the remainder fits one.
        Some((position % len as u64) as usize)
    } else {
        usize::try_from(position).ok().filter(|&index| index < len)
    }
}

/// How many decoded frames a [`Feed`] keeps ahead of the playhead.
///
/// Four, which at 30 Hz is 133 ms of slack against a worst case of 30 ms - so a
/// frame is essentially always waiting - and at 480x272 costs 4 x 196 KB, or
/// 784 KB. The PS2's 512x512 backdrop is 393 KB a frame and so 1.5 MB. Both are
/// noise next to the 52 MB and 2.7 s stall that decoding the whole movie into
/// RAM up front would cost.
///
/// It is a **bound**, not a target: the worker fills the ring and then parks, so
/// this is also what stops a decoder that runs at 900 frames a second from
/// racing thirty seconds ahead of a movie played at 30.
const LOOKAHEAD: usize = 4;

/// The decoded frames a [`Feed`]'s worker has run ahead by, oldest first.
///
/// Pure and threading-free on purpose: everything that decides *which* frame is
/// shown lives here and is unit-tested, and the worker around it is a thin shell
/// - the same split `display::viewport` and `upscale::target_size` use.
#[derive(Debug)]
struct Ring {
    slots: VecDeque<Frame>,
    capacity: usize,
}

impl Ring {
    fn new(capacity: usize) -> Self {
        Self {
            slots: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Whether the worker should park rather than decode another frame.
    fn is_full(&self) -> bool {
        self.slots.len() >= self.capacity
    }

    fn push(&mut self, frame: Frame) {
        self.slots.push_back(frame);
    }

    fn clear(&mut self) {
        self.slots.clear();
    }

    /// The newest frame at or before `position`, dropping anything older.
    ///
    /// Three cases, and each is a policy rather than an accident:
    ///
    /// - **Nothing decoded that far yet** - `None`. The caller keeps the picture
    ///   it last uploaded. A held frame is a frame late; an empty one is a green
    ///   rectangle, because zeroed I420 planes are not black.
    /// - **Exactly the frame asked for** - it comes back, and the ring is that
    ///   much emptier, which is what lets the worker decode another.
    /// - **The playhead jumped** - a slow frame can move it several frames on.
    ///   Everything skipped over is dropped and the *newest* frame at or before
    ///   the playhead comes back, so playback never runs backwards and never
    ///   drifts behind by more than it has to.
    fn take_upto(&mut self, position: u64) -> Option<Frame> {
        let mut newest = None;
        while self
            .slots
            .front()
            .is_some_and(|frame| frame.position <= position)
        {
            newest = self.slots.pop_front();
        }
        newest
    }
}

/// What a [`Feed`]'s worker and its consumer share.
#[derive(Debug)]
struct State {
    ring: Ring,
    /// The position the worker will decode next.
    next: u64,
    /// Bumped by [`Feed::restart`].
    ///
    /// A restart can land while the worker is up to 30 ms into a decode, and the
    /// frame that decode produces belongs to the playback that was abandoned. The
    /// worker re-reads the epoch after decoding and **throws the frame away** if
    /// it moved, so a stale frame can never be pushed at a position the new
    /// playback will later ask for. Without this the *second* time the menus
    /// open is wrong and the first time is fine, which is the worst shape a bug
    /// can have.
    epoch: u64,
    /// Why decoding stopped, when it did. Taken by the consumer, reported once.
    error: Option<String>,
    /// That decoding stopped, which outlives [`State::error`] being taken.
    ///
    /// Two fields for one event because they answer different questions and have
    /// different lifetimes: `error` is a message to report **once**, and this is
    /// the fact the worker parks on **forever**. Folding them into one would mean
    /// `take_error` silently put the worker back to work on a decoder that had
    /// already failed - which happens to be harmless today only because nothing
    /// notifies the condvar there, and a correctness argument that rests on a
    /// missing `notify` is not one.
    failed: bool,
    /// Set when a movie that does not repeat has been decoded to its end.
    done: bool,
    /// Set by [`Feed::drop`], so the worker returns instead of waiting.
    stop: bool,
}

/// A [`FrameStore`] on a worker thread, decoding ahead of the playhead.
///
/// # Why this exists
///
/// Decoding a frame takes up to 30 ms and drawing one has 4 ms to spare at 240
/// frames a second, so the two cannot be the same thread. The worker decodes
/// forward into a ring of [`LOOKAHEAD`] frames and parks when it is full;
/// whoever is drawing asks for *the newest frame at or before* the position the
/// [`Player`] has reached, uploads it if there is one, and keeps the frame it
/// already has if there is not.
///
/// # Why it is safe against the determinism rules
///
/// `docs/architecture/determinism.md` requires the simulation to be
/// single-threaded, and this does not touch it. The feed is **write-only toward
/// the GPU**: a [`Player`] hands it a position and it hands back pixels. Nothing
/// it produces is ever read back into sequencing - the intro's state machine
/// compares [`Player::frames_produced`] against its own 144, 231 and 260, and
/// that number comes from the player whether a picture ever arrives or not. So
/// no simulation state, and no state the simulation reads, can depend on when a
/// decode finished. See
/// [ADR-0010](../../../docs/architecture/adr/0010-movie-decode-thread.md).
///
/// **What ADR-0019 changed, and what it did not.** That number no longer always
/// comes from a *fixed-timestep* `update`: a movie with a sounding track is
/// paced by [`Player::follow`] instead, so the sequencing of that movie now
/// depends on the audio device's clock. ADR-0010's argument survives unchanged,
/// because it is about the **decode** thread and this is not it - the feed still
/// cannot influence the player, and a decode that ran long still cannot move a
/// state transition. What is new is a second clock, not a second writer, and it
/// is not the renderer's. Headless runs are unaffected either way: with no
/// device the mixer is advanced by [`crate::audio::Audio::tick`] at exactly the
/// tick rate, so the two clocks are the same number.
#[derive(Debug)]
pub struct Feed {
    shared: Arc<Shared>,
    /// `None` only between [`Feed::drop`] taking it and the join finishing.
    worker: Option<std::thread::JoinHandle<()>>,
    /// How many frames the cache holds. Copied out of the [`FrameStore`] before
    /// it moved onto the worker, because the player needs it and the store is no
    /// longer reachable from here.
    len: usize,
    /// The store's [`FrameStore::decoder_label`], copied out for the same
    /// reason as `len` - the store itself is not reachable from here once
    /// the worker owns it.
    decoder_label: &'static str,
    /// Chroma plane width in samples, for the renderer's plane geometry.
    pub chroma_width: u32,
    /// Chroma plane height in samples, for the renderer's plane geometry.
    pub chroma_height: u32,
    /// Luma width in samples, and the frame width in pixels.
    pub width: u32,
    /// Luma height in samples, and the frame height in pixels.
    pub height: u32,
}

/// The mutex and the condition the worker parks on.
#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
    /// Signalled when the ring gains room, when a restart happens, and when the
    /// worker should stop. One condition rather than three: the worker re-checks
    /// all of them on every wake, so telling them apart would only be a way to
    /// miss one.
    wake: Condvar,
}

impl Feed {
    /// Moves `store` onto a worker thread and starts decoding.
    ///
    /// `repeat` is the movie's own nature rather than a setting: the menu
    /// backdrop loops and the intro does not, exactly as [`Player::new`] takes
    /// it. A feed that repeats never finishes; one that does not parks on the
    /// last frame with it still in the ring.
    ///
    /// `width` and `height` are the movie's, for the renderer's plane geometry -
    /// [`FrameStore::open`] has already checked they are the cache's too.
    #[must_use]
    pub fn spawn(store: FrameStore, repeat: bool, width: u32, height: u32) -> Self {
        let len = store.len;
        let decoder_label = store.decoder_label();
        let (chroma_width, chroma_height) = (store.chroma_width, store.chroma_height);
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                ring: Ring::new(LOOKAHEAD),
                next: 0,
                epoch: 0,
                error: None,
                failed: false,
                done: false,
                stop: false,
            }),
            wake: Condvar::new(),
        });
        let worker = std::thread::Builder::new()
            // Named so it is obvious in a debugger and in `top` which thread the
            // decode is on, this being the whole point of it existing.
            .name("movie-decode".to_string())
            .spawn({
                let shared = Arc::clone(&shared);
                move || decode_loop(&shared, store, len, repeat)
            })
            // A machine that cannot start a thread cannot run the game either,
            // and the alternative is threading a fallible constructor through
            // every caller for a case that does not happen.
            .expect("spawning the movie decode thread");

        Self {
            shared,
            worker: Some(worker),
            len,
            decoder_label,
            chroma_width,
            chroma_height,
            width,
            height,
        }
    }

    /// How many frames the cache holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// What is actually decoding this movie - `"av1 cache"` or `"gstreamer"`.
    /// See [`VideoDecoder::label`].
    #[must_use]
    pub fn decoder_label(&self) -> &'static str {
        self.decoder_label
    }

    /// Whether the cache holds no frames at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The newest decoded frame at or before `position`, or `None` for none yet.
    ///
    /// `None` is an ordinary answer and means *keep showing what you have*: see
    /// [`Ring::take_upto`] for why that is the only safe fallback.
    pub fn take_upto(&mut self, position: u64) -> Option<Frame> {
        let taken = {
            let mut state = self.lock();
            state.ring.take_upto(position)
        };
        // Only when something came out, because only then did the ring gain the
        // room the worker is parked waiting for.
        if taken.is_some() {
            self.shared.wake.notify_all();
        }
        taken
    }

    /// Puts playback back at position zero.
    ///
    /// Called when a [`Player`] is rebuilt at frame zero - opening the menus a
    /// second time - because the feed's positions and the player's have to share
    /// an origin or every comparison after the first open is meaningless.
    ///
    /// Cheap: the ring is dropped and the worker decodes frame 0, which is one
    /// decoder flush and one frame, not a walk through the movie.
    pub fn restart(&mut self) {
        {
            let mut state = self.lock();
            state.ring.clear();
            state.next = 0;
            state.epoch += 1;
            state.done = false;
            // `failed` is deliberately not cleared. A decoder that failed is not
            // retried: the failure is in the file, not in the moment, so a
            // restarted feed on a broken cache stays parked and the menus keep
            // whatever picture they had, rather than failing once per open.
        }
        self.shared.wake.notify_all();
    }

    /// Why decoding stopped, the once.
    ///
    /// Taken rather than borrowed: a decode failure is reported by whoever is
    /// drawing, and a frame loop that reported the same one sixty times a second
    /// would bury it.
    pub fn take_error(&mut self) -> Option<String> {
        self.lock().error.take()
    }

    /// The shared state, with a poisoned lock treated as a lost worker.
    ///
    /// The worker only panics if the decoder does, and then there is nothing
    /// left to decode with - so recovering the guard and carrying on with a ring
    /// that will never be refilled is strictly better than bringing the drawing
    /// thread down with it. The picture freezes; the game does not stop.
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Drop for Feed {
    fn drop(&mut self) {
        {
            let mut state = self.lock();
            state.stop = true;
        }
        self.shared.wake.notify_all();
        // Joined rather than detached so the `FrameStore` - a decoder and a
        // multi-megabyte blob - is definitely gone before the next feed opens
        // one, and so a worker cannot outlive the process's teardown.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Decodes forward forever, parking whenever the ring is full.
///
/// Split out of [`Feed::spawn`] so the loop reads as a loop. It owns `store`
/// outright, which is the point: the only decoder is on this thread, and the
/// drawing thread has no way to reach it.
fn decode_loop(shared: &Shared, mut store: FrameStore, len: usize, repeat: bool) {
    let lock = || {
        shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    };

    loop {
        // Claim the next position to decode, waiting while there is nothing to
        // do. The lock is released before decoding, so a 30 ms decode never
        // holds up the drawing thread's `take_upto`.
        let (epoch, position) = {
            let mut state = lock();
            loop {
                if state.stop {
                    return;
                }
                if !state.failed && !state.done && !state.ring.is_full() {
                    break (state.epoch, state.next);
                }
                state = shared
                    .wake
                    .wait(state)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        };

        let Some(index) = frame_at(position, len, repeat) else {
            // A movie that does not repeat, decoded to its end. Its last frame
            // is still in the ring for whoever wants it.
            let mut state = lock();
            if state.epoch == epoch {
                state.done = true;
            }
            continue;
        };

        let mut picture = VideoFrame::default();
        let decoded = store.read_frame(index, &mut picture);

        let mut state = lock();
        // Restarted while this was decoding: the frame belongs to playback that
        // no longer exists, so it is dropped rather than pushed. The next pass
        // round claims position 0 and `read_frame` rewinds by itself.
        if state.epoch != epoch {
            continue;
        }
        match decoded {
            Ok(()) => {
                state.ring.push(Frame {
                    position,
                    index,
                    picture,
                });
                state.next = position + 1;
            }
            Err(e) => {
                state.error = Some(format!("{e:#}"));
                state.failed = true;
            }
        }
    }
}

/// How much of a movie to convert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extent {
    /// Every frame.
    Whole,
    /// Only the first `n` frames.
    ///
    /// The intro state stops at frame 260 whatever the movie's length, so
    /// converting the whole 40-second intro would be 235 MiB of cache for eight
    /// seconds of screen time.
    Frames(usize),
}

impl Extent {
    fn limit(self, available: usize) -> usize {
        match self {
            Self::Whole => available,
            Self::Frames(n) => n.min(available),
        }
    }
}

/// The start code every MPEG program stream pack begins with: `00 00 01 BA`.
///
/// A raw MPEG-2 program stream - the PS2's loose `.PSS` files - has no header
/// of its own the way a `.PMF` does, so this is what tells the two apart. See
/// `docs/ps2/pulse-disc-layout.md`.
const MPEG_PS_START_CODE: [u8; 4] = [0x00, 0x00, 0x01, 0xba];

/// How a caller wants a movie opened, beyond which frames of it.
///
/// A struct rather than two trailing `bool`s because that is exactly what it
/// was becoming: `open(.., extent, false, false)` at a call site says nothing
/// about which flag is which, and the two mean opposite kinds of thing.
/// [`Default`] is the ordinary case - decode the picture, reuse whatever the
/// cache already holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Decode {
    /// Skip the picture entirely and report a picture-less movie instead -
    /// still with a correct frame count, width, height and frame rate, since
    /// those come from parsing the container rather than decoding it.
    pub no_video: bool,
    /// Treat every cache file as a miss, so a transcode runs and overwrites it.
    ///
    /// **Only reaches the paths that transcode.** A `native-video` build
    /// decoding H.264 through GStreamer writes no cache file, so there is
    /// nothing there to ignore and this changes nothing for it; what it forces
    /// is a re-run of the `ffmpeg` conversions in [`transcode`],
    /// [`transcode_ipu`] and [`transcode_mpeg2_ps`]. This is the flag for a
    /// cache written by a build whose conversion has since been fixed - the
    /// cache is keyed by *content*, so a changed encoder produces the same key
    /// and the stale file would otherwise be reused forever.
    pub refresh: bool,
    /// **Build** the AV1 cache rather than using a platform decoder, when there
    /// is no cache file yet.
    ///
    /// **This is not what decides whether a cache file is used.** A cache file
    /// that already exists is preferred over the platform decoder always, with
    /// no flag - see the [`cached`] call in `open_psmf`, which is the ordinary
    /// path on every run after the one that built it. What this decides is the
    /// *other* case: with nothing cached, whether to transcode now or let the
    /// platform decode and leave the cache empty.
    ///
    /// **Only the `.PMF` path has a choice to make.** A PS2 `.PSS` or `.IPF`
    /// goes through the cache whatever this says, because there is no platform
    /// decoder for either; this is about H.264 on a `native-video` Linux build,
    /// where [ADR-0017] made GStreamer the default for a first, uncached run.
    /// That default stands - see its own reasoning - and this is the way past it.
    ///
    /// Two callers want it, for different reasons:
    ///
    /// - [`crate::prefetch`] sets it **always**, and must. Its whole job is to
    ///   fill the cache, and the GStreamer path writes no cache file at all - it
    ///   decodes into memory and hands back frames. A prefetch that took that
    ///   path would report every movie converted, leave the cache empty, and
    ///   plan the same 22 movies again on the next run.
    /// - `--prefer-av1-cache` sets it for the boot's own reels, so that one run
    ///   pays the transcode and every run afterwards picks the file up by
    ///   itself. [`Self::refresh`] implies it, because a re-conversion is what
    ///   that flag asks for and the platform decoder writes nothing to
    ///   re-convert.
    ///
    /// [ADR-0017]: ../../../docs/architecture/adr/0017-gstreamer-native-video.md
    pub prefer_cache: bool,
}

/// What a movie load is doing, for a caller that is drawing a wait.
///
/// **The three differ by three orders of magnitude**, which is the whole reason
/// this is reported rather than left as one word: a cache hit is milliseconds, a
/// GStreamer decode is seconds, and an `ffmpeg` transcode of the 1200-frame
/// intro is about eighty of them. A screen that called all three "loading" would
/// be asking a player to sit through eighty seconds in the same voice it uses
/// for a wait they will never see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Reading a cache file that already holds the frames asked for.
    Cached,
    /// Decoding through the platform's own decoder - see `gst_frame_store`.
    /// No frame count: that decode is one call which returns when it is done.
    Decoding,
    /// `ffmpeg` is converting, and has encoded `done` of `total` frames.
    ///
    /// `total` is `None` when the conversion was not capped and the input's own
    /// length is not known here, which is the raw program stream's case.
    Transcoding { done: usize, total: Option<usize> },
}

/// Where a movie load reports its [`Step`], for a caller that is drawing a wait.
///
/// `None` for the callers with nowhere to show one - a headless capture, the
/// ground-truth tests, `--dry-run` - which is also why this is a borrowed
/// callback rather than a channel: nothing is spawned to service it, and a
/// caller that passes `None` costs nothing at all.
pub type Watch<'a> = Option<&'a (dyn Fn(Step) + Sync)>;

/// Reports `step` if anyone is listening.
fn watched(watch: Watch<'_>, step: Step) {
    if let Some(watch) = watch {
        watch(step);
    }
}

/// Makes a movie's frames available, transcoding if it must.
///
/// Dispatches on the blob's own magic rather than on which platform it came
/// from - a `.PMF`'s `PSMF` header, a raw MPEG-2 program stream's pack start
/// code, or an `.IPF`'s `IPUF` - because what a decode path needs to know is
/// what the file is, not what disc it happened to come off.
///
/// `key` identifies the source for caching. It must change when the bytes do:
/// the callers pass the WAD name hash and the entry size (or, for a loose PS2
/// file, its own path and length), so a different disc image cannot collide.
pub fn open(
    blob: &[u8],
    key: &str,
    cache_dir: &Path,
    extent: Extent,
    how: Decode,
    watch: Watch<'_>,
) -> Result<Movie> {
    if blob.starts_with(pmf::MAGIC) {
        open_psmf(blob, key, cache_dir, extent, how, watch)
    } else if blob.starts_with(&MPEG_PS_START_CODE) {
        open_mpeg2_ps(blob, key, cache_dir, extent, how, watch)
    } else if blob.starts_with(&ipf::MAGIC) {
        open_ipuf(blob, key, cache_dir, extent, how, watch)
    } else {
        let head = &blob[..blob.len().min(4)];
        bail!("{key} is not a movie container this build recognises (starts with {head:02x?})")
    }
}

/// Demuxes a `.PMF` and makes its frames available, transcoding if it must.
fn open_psmf(
    blob: &[u8],
    key: &str,
    cache_dir: &Path,
    extent: Extent,
    how: Decode,
    watch: Watch<'_>,
) -> Result<Movie> {
    let header = pmf::Header::parse(blob).context("parsing the PSMF header")?;
    let video = header
        .video
        .context("this movie declares no video stream")?;

    let demuxed = pmf::demux(blob).context("demuxing the program stream")?;
    if demuxed.stray_bytes != 0 {
        // Not fatal, but it means the walk lost sync and the frame count below
        // is suspect, so it must be visible rather than swallowed.
        eprintln!(
            "warning: {} byte(s) of {key} were not part of any pack or PES packet",
            demuxed.stray_bytes
        );
    }

    let frame_count = pmf::frame_count(&demuxed.video);
    let expected = header.expected_frame_count() as usize;
    // The header's duration and the elementary stream's access units are two
    // independent measurements of the same thing. Disagreeing by more than a
    // frame means one of them is being read wrong.
    if frame_count.abs_diff(expected) > 1 {
        eprintln!(
            "warning: {key} has {frame_count} access units but its duration implies {expected}"
        );
    }

    let width = u32::from(video.width);
    let height = u32::from(video.height);

    // Unwrapped here rather than where it is played, because this is the only
    // place the demuxed packets exist: `pmf::demux` has been handing them over
    // to nothing since it was written, which is what ADR-0019 exists to end.
    // Undecoded, though - see `MovieAudio`.
    let audio = header
        .audio
        .and_then(|stream| movie_audio(&demuxed, stream, key));

    if how.no_video {
        return Ok(Movie {
            header: Some(header),
            frame_count,
            width,
            height,
            frame_rate: FRAME_RATE,
            display_aspect: (width, height),
            frames: None,
            no_picture_reason: Some("--no-video was given".to_string()),
            audio,
        });
    }

    let wanted = extent.limit(frame_count);

    let conversion = Conversion {
        key,
        cache_dir,
        width,
        height,
        refresh: how.refresh,
        watch,
    };

    // **A cache file that is already there beats the platform decoder**, and
    // this is the ordinary path on any run after the first that built one.
    // `GstDecoder::open` decodes every wanted frame into memory before it
    // returns - 4.80 s for the EU disc's two reels, every boot - where the cache
    // is opened lazily and decoded a frame at a time, which is 0.05 s. The
    // pictures are the same either way: the cache is lossless (ADR-0008), which
    // `movie_ground_truth.rs` checks byte for byte.
    //
    // Asking costs a parse of the frame headers, so a miss is not worth
    // avoiding. `refresh` makes `cached` answer `None`, which is what sends
    // `--refresh-video` past both this and GStreamer to the transcode it asked
    // for.
    if let Some(frames) = cached(conversion, Frames::capped(wanted)) {
        watched(watch, Step::Cached);
        return Ok(Movie {
            header: Some(header),
            frame_count,
            width,
            height,
            frame_rate: FRAME_RATE,
            display_aspect: (width, height),
            frames: Some(frames),
            no_picture_reason: None,
            audio,
        });
    }

    // Nothing cached, so the choice is the platform decoder or building one.
    // [ADR-0017] makes the decoder the default; `prefer_cache` is the opt-in
    // past it, and `refresh` implies it because a re-conversion is exactly what
    // that flag asks for and the decoder writes no file to re-convert.
    //
    // Said before the attempt, not after: the decode is one call that returns
    // when it is done, so the word has to be on screen while it happens.
    //
    // [ADR-0017]: ../../../docs/architecture/adr/0017-gstreamer-native-video.md
    #[cfg(all(target_os = "linux", feature = "native-video"))]
    if !how.prefer_cache && !how.refresh {
        watched(watch, Step::Decoding);
        if let Some(frames) = gst_frame_store(&demuxed.video, key, width, height, wanted) {
            return Ok(Movie {
                header: Some(header),
                frame_count,
                width,
                height,
                frame_rate: FRAME_RATE,
                display_aspect: (width, height),
                frames: Some(frames),
                no_picture_reason: None,
                audio,
            });
        }
    }

    match transcode(&demuxed.video, conversion, wanted) {
        Ok(frames) => Ok(Movie {
            header: Some(header),
            frame_count,
            width,
            height,
            frame_rate: FRAME_RATE,
            display_aspect: (width, height),
            frames: Some(frames),
            no_picture_reason: None,
            audio,
        }),
        Err(reason) => Ok(Movie {
            header: Some(header),
            frame_count,
            width,
            height,
            frame_rate: FRAME_RATE,
            display_aspect: (width, height),
            frames: None,
            no_picture_reason: Some(format!("{reason:#}")),
            audio,
        }),
    }
}

/// Tries the platform-native H.264 path for a demuxed `.PMF` elementary
/// stream, on Linux with the `native-video` feature. `None` means "use the
/// AV1 cache instead" - for any reason: no usable GStreamer H.264 decoder
/// element, or an outright error partway through, which is printed as a
/// warning rather than surfaced, matching how a missing `ffmpeg` is handled
/// a few lines down. See
/// [ADR-0017](../../../docs/architecture/adr/0017-gstreamer-native-video.md).
#[cfg(all(target_os = "linux", feature = "native-video"))]
fn gst_frame_store(
    video: &[u8],
    key: &str,
    width: u32,
    height: u32,
    wanted: usize,
) -> Option<FrameStore> {
    let decoder = match gst::GstDecoder::open(video.to_vec(), wanted) {
        Ok(Some(decoder)) => decoder,
        Ok(None) => return None,
        Err(e) => {
            eprintln!("warning: platform-native H.264 decode unavailable for {key}: {e:#}");
            return None;
        }
    };

    let geometry = decoder.geometry();
    if (geometry.width, geometry.height) != (width, height) {
        eprintln!(
            "warning: {key}'s GStreamer decode is {}x{}, but the movie declares {width}x{height}",
            geometry.width, geometry.height
        );
        return None;
    }

    Some(FrameStore {
        // Keeps the `{hash}-{size}` prefix the AV1 cache's own filenames use
        // (see `transcode`), so a caller that parses `path()` for the entry
        // key - `movie_ground_truth.rs`'s `the_cache_is_lossless` does - gets
        // the same answer regardless of which decoder actually served the
        // movie.
        path: PathBuf::from(format!("{key}-gst")),
        len: decoder.len(),
        luma_len: geometry.luma_len(),
        chroma_len: geometry.chroma_len(),
        chroma_width: geometry.chroma_width,
        chroma_height: geometry.chroma_height,
        source: Box::new(decoder),
    })
}

/// Makes a raw MPEG-2 program stream's frames available, transcoding if it
/// must.
///
/// This is the PS2's loose `.PSS` movies: no PSMF wrapper, no separate demux
/// step - `ffmpeg` reads the whole program stream itself, container and all -
/// and no audio stream in the one measured so far (`INTRO512.PSS`), so this
/// never reports one. Width, height and frame rate come from `ffprobe`
/// (see [`probe`]) rather than a header, because there is no header to read
/// them from.
fn open_mpeg2_ps(
    blob: &[u8],
    key: &str,
    cache_dir: &Path,
    extent: Extent,
    how: Decode,
    watch: Watch<'_>,
) -> Result<Movie> {
    let probed = probe(blob, key, cache_dir)?;

    if how.no_video {
        return Ok(Movie {
            header: None,
            frame_count: probed.frame_count,
            width: probed.width,
            height: probed.height,
            frame_rate: probed.frame_rate,
            display_aspect: probed.display_aspect,
            frames: None,
            no_picture_reason: Some("--no-video was given".to_string()),
            audio: None,
        });
    }

    let wanted = Frames::plan(extent, probed.frame_count);

    match transcode_mpeg2_ps(
        blob,
        Conversion {
            key,
            cache_dir,
            width: probed.width,
            height: probed.height,
            refresh: how.refresh,
            watch,
        },
        wanted,
    ) {
        Ok(frames) => Ok(Movie {
            header: None,
            frame_count: frames.len,
            width: probed.width,
            height: probed.height,
            frame_rate: probed.frame_rate,
            display_aspect: probed.display_aspect,
            frames: Some(frames),
            no_picture_reason: None,
            audio: None,
        }),
        Err(reason) => Ok(Movie {
            header: None,
            frame_count: probed.frame_count,
            width: probed.width,
            height: probed.height,
            frame_rate: probed.frame_rate,
            display_aspect: probed.display_aspect,
            frames: None,
            no_picture_reason: Some(format!("{reason:#}")),
            audio: None,
        }),
    }
}

/// The rate a PS2 `.IPF` backdrop is presented at, by declared width.
///
/// **An `IPUF` container declares no frame rate at all**, so this is inferred
/// rather than read, and the inference is the disc's own pairing:
/// `SCES_547.48`'s `FUN_0019b168` rewrites the front-end XML's
/// `Data\Movies\Backdrop.ipf` to `Data\Movies\bg512.ipf` or
/// `Data\Movies\bg640.ipf` on exactly the same global (`0x0027a85c`) that picks
/// `Intro512.pss` against `Intro640.pss` - and those two *do* declare their
/// rates, measured at 25/1 and 30000/1001. So `bg512` is the PAL cut and
/// `bg640` the NTSC one.
///
/// It checks out arithmetically as well: 225 frames at 25 Hz is 9.000 s and
/// 270 at 30000/1001 is 9.009 s, so the two cuts are the same nine-second loop.
/// Swapping the pairing would make one of them 10.8 s and the other 7.5 s.
///
/// See `docs/formats/ipf.md`.
fn backdrop_frame_rate(width: u32) -> (u64, u64) {
    if width == 640 {
        (30_000, 1001)
    } else {
        (25, 1)
    }
}

/// The display aspect both PS2 backdrop cuts are drawn at.
///
/// Same as the `.PSS` intro cuts of the same two sizes, which declare `4:3` in
/// their own sequence headers - neither `512x512` nor `640x448` is 4:3 as a
/// pixel grid, and the PS2's front end draws its movie over a `640x448` black
/// `Image` filling the screen. An `IPUF` carries no aspect field, so this is
/// taken from the paired cut rather than read.
const BACKDROP_DISPLAY_ASPECT: (u32, u32) = (4, 3);

/// Makes a PS2 `.IPF` backdrop's frames available, transcoding if it must.
///
/// The container is [`oag_formats::ipf`]: fixed-size slots around one IPU
/// frame each. `ffmpeg` has an IPU decoder and a demuxer for Sony's own `ipum`
/// wrapper, so the transcode input is the slots stripped off and that wrapper
/// put on - see [`ipum`].
///
/// Width, height and frame count come from the container and are exact. The
/// frame rate does not, because there is none to read: see
/// [`backdrop_frame_rate`].
fn open_ipuf(
    blob: &[u8],
    key: &str,
    cache_dir: &Path,
    extent: Extent,
    how: Decode,
    watch: Watch<'_>,
) -> Result<Movie> {
    let parsed = ipf::parse(blob).map_err(|e| anyhow!("parsing {key} as an IPF: {e}"))?;
    let width = parsed.header.width;
    let height = parsed.header.height;
    let frame_count = parsed.len();
    let frame_rate = backdrop_frame_rate(width);

    if how.no_video {
        return Ok(Movie {
            header: None,
            frame_count,
            width,
            height,
            frame_rate,
            display_aspect: BACKDROP_DISPLAY_ASPECT,
            frames: None,
            no_picture_reason: Some("--no-video was given".to_string()),
            audio: None,
        });
    }

    let wanted = extent.limit(frame_count);

    match transcode_ipu(
        &parsed,
        Conversion {
            key,
            cache_dir,
            width,
            height,
            refresh: how.refresh,
            watch,
        },
        wanted,
    ) {
        Ok(frames) => Ok(Movie {
            header: None,
            frame_count,
            width,
            height,
            frame_rate,
            display_aspect: BACKDROP_DISPLAY_ASPECT,
            frames: Some(frames),
            no_picture_reason: None,
            audio: None,
        }),
        Err(reason) => Ok(Movie {
            header: None,
            frame_count,
            width,
            height,
            frame_rate,
            display_aspect: BACKDROP_DISPLAY_ASPECT,
            frames: None,
            no_picture_reason: Some(format!("{reason:#}")),
            audio: None,
        }),
    }
}

/// Wraps a bare IPU bitstream in the 16-byte `ipum` header `ffmpeg`'s demuxer
/// reads.
///
/// ```text
/// +0x00  u8[4]   "ipum"
/// +0x04  u32le   payload length
/// +0x08  u16le   width
/// +0x0a  u16le   height
/// +0x0c  u32le   frame count
/// +0x10          the bitstream
/// ```
///
/// This is `ffmpeg`'s container, not Wipeout's, which is why it is built here
/// rather than in `oag-formats`: it exists only to hand the transcoder the
/// dimensions, which the IPU bitstream itself does not carry. Everything after
/// the header is the `.IPF`'s own bytes, unaltered.
fn ipum(bitstream: &[u8], width: u32, height: u32, frames: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + bitstream.len());
    out.extend_from_slice(b"ipum");
    out.extend_from_slice(&(bitstream.len() as u32).to_le_bytes());
    out.extend_from_slice(&(width as u16).to_le_bytes());
    out.extend_from_slice(&(height as u16).to_le_bytes());
    out.extend_from_slice(&(frames as u32).to_le_bytes());
    out.extend_from_slice(bitstream);
    out
}

/// Everything a conversion needs beyond the bytes it is converting.
///
/// The six travel together through all three `transcode_*` functions and are
/// answers to one question - where this conversion writes, at what size, and who
/// is watching - so they are one parameter rather than six. `frames` is
/// deliberately *not* in here: it is a `usize` for the two containers that carry
/// their own length and an `Option<usize>` for the raw program stream, which has
/// no header to read one from.
/// How many frames a conversion encodes, and how many that will turn out to be.
///
/// **Two numbers because they answer different questions**, and they are only
/// sometimes the same one:
///
/// - `cap` is `ffmpeg`'s `-frames:v`, which stops the encode early. `None`
///   converts the whole input, and it is also what names the cache file (see
///   [`cache_name`]), so it must stay exactly what the caller asked for.
/// - `total` is the denominator of the progress report. It is known even when
///   there is no cap, because the input's own length was measured before any of
///   this started.
///
/// One `Option<usize>` used to serve both, which was fine until something drew a
/// bar with it: the PS2's uncapped `.PSS` transcode reported a frame number with
/// nothing to divide it by, so a minute of work moved the bar not at all. The
/// cap was `None` and the length was known the whole time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Frames {
    cap: Option<usize>,
    total: Option<usize>,
}

impl Frames {
    /// Encode `n` and stop. The cap is also the count, by definition.
    fn capped(n: usize) -> Self {
        Self {
            cap: Some(n),
            total: Some(n),
        }
    }

    /// Encode all of it. `available` is what the container was measured at.
    ///
    /// **The measurement does not have to be exact and is not treated as if it
    /// were.** A raw program stream carries no frame count, so [`probe`] derives
    /// one from its duration and frame rate - 38.0 s at 25/1 gives 950 for
    /// `INTRO512.PSS`, which is what that file really encodes to, but an
    /// approximation is all it can be. It is the denominator of a picture of a
    /// wait, not of a decision: a low estimate saturates the bar at the end of
    /// the slice it was already heading for, which is where the next load puts
    /// it anyway.
    fn whole(available: usize) -> Self {
        Self {
            cap: None,
            total: Some(available),
        }
    }

    /// What `extent` asks for, against a container measured at `available`.
    ///
    /// The one place the two questions above are answered together, so that a
    /// caller cannot answer only the first - which is exactly how the bar came
    /// to stand still through the longest wait the boot has.
    fn plan(extent: Extent, available: usize) -> Self {
        match extent {
            Extent::Whole => Self::whole(available),
            Extent::Frames(n) => Self::capped(n),
        }
    }
}

#[derive(Clone, Copy)]
struct Conversion<'a> {
    /// Identifies the source for caching - see [`open`].
    key: &'a str,
    cache_dir: &'a Path,
    width: u32,
    height: u32,
    /// Ignore whatever is already cached - see [`Decode::refresh`].
    refresh: bool,
    watch: Watch<'a>,
}

/// The cache file for this conversion, if it is there and holds what was asked
/// for. **Never converts anything.**
///
/// Asking without converting is the point: it is what lets the `.PMF` path
/// prefer a cache file that already exists over the platform decoder without
/// committing to a transcode when there is none. The three `transcode_*`
/// functions ask the same question through the same code, so "is it cached?"
/// cannot come to mean two things.
///
/// A previous run's file is reused only if it opens **and** holds the frames
/// asked for. Unlike a raw frame cache this cannot be checked by file length, so
/// it is checked by decoding the container - cheap, since that is a parse of the
/// frame headers and not of the pictures. An uncapped conversion asks only that
/// it opens: it was written as "all of it", and how many that turned out to be
/// is what the file itself says.
///
/// `refresh` answers `None` to everything, which is the whole of what that flag
/// does: the file is there and perfectly readable, and the point is to overwrite
/// it. See [`Decode::refresh`].
fn cached(to: Conversion<'_>, frames: Frames) -> Option<FrameStore> {
    if to.refresh {
        return None;
    }
    let out = to
        .cache_dir
        .join(cache_name(to.key, to.width, to.height, frames.cap));
    FrameStore::open(out, to.width, to.height)
        .ok()
        .filter(|store| frames.cap.is_none_or(|n| store.len == n))
}

/// Converts a PS2 `.IPF`'s IPU bitstream into lossless AV1 under the cache.
fn transcode_ipu(parsed: &ipf::Ipf<'_>, to: Conversion<'_>, frames: usize) -> Result<FrameStore> {
    let Conversion {
        key,
        cache_dir,
        width,
        height,
        // Read by `cached` alone now: whether a conversion is skipped is that
        // function's whole question, and asking it twice is how the two answers
        // would drift.
        refresh: _,
        watch,
    } = to;
    if let Some(store) = cached(to, Frames::capped(frames)) {
        watched(watch, Step::Cached);
        return Ok(store);
    }
    let out = cache_dir.join(cache_name(key, width, height, Some(frames)));

    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating {}", cache_dir.display()))?;

    let source = cache_dir.join(format!("{key}.ipu"));
    let wrapped = ipum(&parsed.bitstream(), width, height, parsed.len());
    std::fs::write(&source, &wrapped).with_context(|| format!("writing {}", source.display()))?;

    run_ffmpeg(&source, &out, Some("ipu"), Frames::capped(frames), watch)?;

    let store = FrameStore::open(out, width, height)?;
    if store.len == 0 {
        bail!("{} holds no frames", store.path().display());
    }
    Ok(store)
}

/// Converts an H.264 elementary stream into lossless AV1 under the cache.
///
/// Returns the reason as an error when conversion is impossible, which the
/// caller turns into a fallback rather than a failure.
fn transcode(video: &[u8], to: Conversion<'_>, frames: usize) -> Result<FrameStore> {
    let Conversion {
        key,
        cache_dir,
        width,
        height,
        // Read by `cached` alone now: whether a conversion is skipped is that
        // function's whole question, and asking it twice is how the two answers
        // would drift.
        refresh: _,
        watch,
    } = to;
    if let Some(store) = cached(to, Frames::capped(frames)) {
        watched(watch, Step::Cached);
        return Ok(store);
    }
    let out = cache_dir.join(cache_name(key, width, height, Some(frames)));

    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating {}", cache_dir.display()))?;

    // Written beside the cache because it is the exact input ffmpeg saw, so a
    // mismatch can be reproduced by hand.
    let es = cache_dir.join(format!("{key}.h264"));
    std::fs::write(&es, video).with_context(|| format!("writing {}", es.display()))?;

    // The elementary stream carries no container timing, so the demuxer has to
    // be named explicitly.
    run_ffmpeg(&es, &out, Some("h264"), Frames::capped(frames), watch)?;

    let store = FrameStore::open(out, width, height)?;
    if store.len == 0 {
        bail!("{} holds no frames", store.path().display());
    }
    Ok(store)
}

/// Converts a raw MPEG-2 program stream into lossless AV1 under `cache_dir`.
///
/// Unlike [`transcode`], `video` is the *whole* container - `ffmpeg` demuxes
/// it itself, so there is no elementary stream to extract first, and no input
/// format to name: a program stream carries its own.
fn transcode_mpeg2_ps(video: &[u8], to: Conversion<'_>, frames: Frames) -> Result<FrameStore> {
    let Conversion {
        key,
        cache_dir,
        width,
        height,
        // Read by `cached` alone now: whether a conversion is skipped is that
        // function's whole question, and asking it twice is how the two answers
        // would drift.
        refresh: _,
        watch,
    } = to;
    if let Some(store) = cached(to, frames) {
        watched(watch, Step::Cached);
        return Ok(store);
    }
    // `cap` rather than `total`: the cache file is named for what was asked for,
    // and an uncapped conversion is spelled `-all` whatever the container turned
    // out to measure. Naming it for the count would orphan every file already in
    // the cache and make the name depend on `ffprobe`.
    let out = cache_dir.join(cache_name(key, width, height, frames.cap));

    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating {}", cache_dir.display()))?;

    let source = cache_dir.join(format!("{key}.pss"));
    std::fs::write(&source, video).with_context(|| format!("writing {}", source.display()))?;

    run_ffmpeg(&source, &out, None, frames, watch)?;

    let store = FrameStore::open(out, width, height)?;
    if store.len == 0 {
        bail!("{} holds no frames", store.path().display());
    }
    Ok(store)
}

/// Runs `ffmpeg` to transcode `input` into lossless AV1 `output`.
///
/// `input_format` names the demuxer explicitly when `input` is a bare
/// elementary stream with no container of its own (`Some("h264")` for a
/// `.PMF`'s video); `None` lets `ffmpeg` recognise the container itself, which
/// a raw MPEG-2 program stream carries. `frames` says both how many pictures to
/// encode and how many that will be - see [`Frames`], which exists because those
/// are two questions.
/// `watch` is called as each frame is encoded - see [`ffmpeg_progress`].
fn run_ffmpeg(
    input: &Path,
    output: &Path,
    input_format: Option<&str>,
    frames: Frames,
    watch: Watch<'_>,
) -> Result<()> {
    // Said before rather than after, because the whole 1200-frame intro takes
    // about 80 seconds and silence for that long reads as a hang. It happens
    // once per movie: the result is cached.
    eprintln!(
        "transcoding {} into {} (once; cached after this)",
        frames.cap.map_or_else(
            || frames.total.map_or_else(
                || "every frame".to_string(),
                |n| format!("all {n} frame(s)")
            ),
            |n| format!("{n} frame(s)")
        ),
        output.display()
    );
    // Reported before the process is even spawned, so a screen watching this
    // says "transcoding" for the whole of the wait rather than from whenever
    // libaom finishes its first frame. Encoding one frame of the intro is a
    // tenth of a second, but `ffmpeg` opening and probing its input is not.
    watched(
        watch,
        Step::Transcoding {
            done: 0,
            total: frames.total,
        },
    );
    let mut command = std::process::Command::new("ffmpeg");
    command
        .arg("-hide_banner")
        .args(["-loglevel", "error"])
        .arg("-y");
    if let Some(format) = input_format {
        command.args(["-f", format]);
    }
    command.arg("-i").arg(input);
    if let Some(cap) = frames.cap {
        command.args(["-frames:v", &cap.to_string()]);
    }
    if watch.is_some() {
        // Only when somebody is reading it: `-progress` writes a block of
        // key=value lines every second, and a run nobody is watching should not
        // pay for a pipe and a reader thread to throw them away. `-nostats`
        // turns off the human-readable status line that would otherwise share
        // the same stream.
        command.args(["-progress", "pipe:1", "-nostats"]);
        command.stdout(std::process::Stdio::piped());
    }
    let status = command
        .args(["-c:v", "libaom-av1"])
        // All five of these are needed together. `-lossless 1` alone is
        // silently ignored and yields a ~200 kbit/s lossy encode that looks
        // like a spectacular compression win; the quantiser has to be pinned to
        // zero and the rate control disabled as well. Verified by decoding the
        // result and comparing it byte for byte with the raw frames.
        .args(["-b:v", "0"])
        .args(["-crf", "0"])
        .args(["-qmin", "0"])
        .args(["-qmax", "0"])
        .args(["-aom-params", "lossless=1"])
        // `-cpu-used` below 6 is *also* not bit-exact in this mode, reproducibly
        // so, as well as slower. 6 is both correct and quick: the whole 1200
        // frame intro encodes in about 80 seconds.
        .args(["-cpu-used", "6"])
        .args(["-row-mt", "1"])
        .args(["-pix_fmt", PIXEL_FORMAT])
        .args(["-f", "ivf"])
        .arg(output)
        .spawn()
        .and_then(|mut child| {
            // Drained on *this* thread rather than a spawned one, which is what
            // keeps `watch` a borrowed `&dyn Fn` with no `'static` bound: the
            // read runs entirely inside this call. It is also why the pipe must
            // be drained at all - a progress block a second would eventually
            // fill the pipe buffer and block `ffmpeg` itself.
            if let Some(stdout) = child.stdout.take() {
                ffmpeg_progress(stdout, frames.total, watch);
            }
            child.wait()
        });

    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!(
            "ffmpeg exited with {status}. If it reports an unknown encoder, this \
             build of ffmpeg lacks libaom-av1"
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!(
            "ffmpeg is not on PATH. Install it to see the intro video; \
             without it the sequence still plays, with a black picture"
        ),
        Err(e) => Err(e).context("running ffmpeg"),
    }
}

/// Turns `ffmpeg -progress pipe:1` into [`Step::Transcoding`] reports.
///
/// The format is `ffmpeg`'s own and is deliberately machine-readable: one
/// `key=value` per line, a block of them about once a second, each block ended
/// by a `progress=continue` (or `progress=end` for the last). Only `frame=` is
/// read here - the rest is bitrate, speed and timing, none of which a loading
/// screen has anywhere to put.
///
/// **Every parse failure is silence rather than an error.** This is a picture of
/// a wait, not a measurement: a line that will not parse, a build of `ffmpeg`
/// that spells its keys differently, a pipe that closes early - all of them mean
/// the count stops moving while the conversion carries on, which is what the
/// caller already copes with when nobody is watching at all. Returning an error
/// from here would fail a transcode that is going perfectly well.
fn ffmpeg_progress(stdout: std::process::ChildStdout, total: Option<usize>, watch: Watch<'_>) {
    use std::io::BufRead;

    for line in std::io::BufReader::new(stdout).lines() {
        let Ok(line) = line else { return };
        if let Some(done) = line
            .strip_prefix("frame=")
            .and_then(|n| n.trim().parse().ok())
        {
            watched(watch, Step::Transcoding { done, total });
        }
    }
}

/// What `probe` reads off a raw MPEG-2 program stream with `ffprobe`.
struct Probed {
    width: u32,
    height: u32,
    frame_rate: (u64, u64),
    display_aspect: (u32, u32),
    frame_count: usize,
}

/// Reads a raw MPEG-2 program stream's video parameters with `ffprobe`,
/// without decoding a single picture.
///
/// A `.PMF`'s PSMF header gives this for free; a PS2 `.PSS` has no header at
/// all, so this is the substitute, and it is why a raw program stream needs
/// `ffprobe` on `PATH` even to report `--no-video`, where a `.PMF` does not.
fn probe(blob: &[u8], key: &str, cache_dir: &Path) -> Result<Probed> {
    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating {}", cache_dir.display()))?;
    let source = cache_dir.join(format!("{key}.pss"));
    std::fs::write(&source, blob).with_context(|| format!("writing {}", source.display()))?;

    let stream = run_ffprobe(
        &source,
        "stream",
        "width,height,r_frame_rate,display_aspect_ratio",
    )?;
    let width: u32 = field(&stream, "width")
        .context("ffprobe printed no width")?
        .parse()
        .context("parsing ffprobe's width")?;
    let height: u32 = field(&stream, "height")
        .context("ffprobe printed no height")?
        .parse()
        .context("parsing ffprobe's height")?;
    let rate = field(&stream, "r_frame_rate").context("ffprobe printed no frame rate")?;
    let (num, den) = rate
        .split_once('/')
        .context("ffprobe's frame rate was not a fraction")?;
    let frame_rate = (
        num.parse().context("parsing ffprobe's frame rate")?,
        den.parse().context("parsing ffprobe's frame rate")?,
    );
    // Square pixels (no `display_aspect_ratio` line, or one ffprobe could not
    // work out) fall back to the decoded size itself, same as a `.PMF`.
    let display_aspect = field(&stream, "display_aspect_ratio")
        .and_then(|dar| dar.split_once(':'))
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .unwrap_or((width, height));

    let format = run_ffprobe(&source, "format", "duration")?;
    let duration: f64 = field(&format, "duration")
        .context("ffprobe printed no duration")?
        .parse()
        .context("parsing ffprobe's duration")?;
    let frame_count = (duration * frame_rate.0 as f64 / frame_rate.1 as f64).round() as usize;

    Ok(Probed {
        width,
        height,
        frame_rate,
        display_aspect,
        frame_count,
    })
}

/// Runs `ffprobe`, requesting `fields` (comma-separated) out of `section`
/// (`stream` or `format`), keyed so [`field`] can pick one out regardless of
/// the order `ffprobe` prints them in - which is its own internal field
/// order, not necessarily the order requested.
fn run_ffprobe(input: &Path, section: &str, fields: &str) -> Result<String> {
    let output = std::process::Command::new("ffprobe")
        .args(["-v", "error"])
        .args(["-select_streams", "v:0"])
        .args(["-show_entries", &format!("{section}={fields}")])
        .args(["-of", "default=noprint_wrappers=1"])
        .arg(input)
        .output();

    let output = match output {
        Ok(output) => output,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!(
            "ffprobe is not on PATH. It ships beside ffmpeg and is needed to read \
             this movie's own width, height and frame rate, which it carries no \
             header for"
        ),
        Err(e) => return Err(e).context("running ffprobe"),
    };

    if !output.status.success() {
        bail!(
            "ffprobe exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Picks `key=value`'s value out of [`run_ffprobe`]'s output.
fn field<'a>(output: &'a str, key: &str) -> Option<&'a str> {
    output
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
}

/// Plays a movie: a frame counter, a pause flag, and the pacing rules.
///
/// This is the part the intro state machine talks to, and it is deliberately
/// free of any decoding or rendering. The original's player never reads the pad
/// either; skipping is the *state's* job.
#[derive(Debug)]
pub struct Player {
    frames: usize,
    /// The rate this movie presents frames at. A `.PMF` is always
    /// [`FRAME_RATE`]; a PS2 `.PSS` carries its own - see [`Movie::frame_rate`].
    frame_rate: (u64, u64),
    /// Seconds accumulated toward the next frame.
    accumulator: f64,
    frame: usize,
    /// How many frames have gone by, counting every loop - see
    /// [`Player::position`].
    position: u64,
    paused: bool,
    finished: bool,
    repeat: bool,
    /// Seconds of sound that went by while the picture was held, which the
    /// audio clock has to be read net of.
    ///
    /// Only the `--reel` leg ever holds a movie - see
    /// [`crate::frontend::PAUSE_FRAMES`] - and it holds for two seconds at a
    /// time. A sound card cannot be held with it, so without this the picture
    /// would jump sixty frames the moment it resumed. Zero on the disc's own
    /// leg, which never pauses.
    held_seconds: f64,
    /// Where the audio clock was when the current hold started, if one is on.
    held_from: Option<f64>,
}

impl Player {
    /// A player positioned at frame zero.
    #[must_use]
    pub fn new(frames: usize, repeat: bool, frame_rate: (u64, u64)) -> Self {
        Self {
            frames,
            frame_rate,
            accumulator: 0.0,
            frame: 0,
            position: 0,
            paused: false,
            finished: frames == 0,
            repeat,
            held_seconds: 0.0,
            held_from: None,
        }
    }

    /// Advances by `dt` seconds.
    pub fn update(&mut self, dt: f64) {
        if self.paused || self.finished {
            return;
        }

        let (num, den) = self.frame_rate;
        let per_frame = den as f64 / num as f64;
        self.accumulator += dt;

        while self.accumulator >= per_frame {
            self.accumulator -= per_frame;
            self.frame += 1;
            self.position += 1;

            if self.frame >= self.frames {
                if self.repeat {
                    self.frame = 0;
                } else {
                    self.frame = self.frames.saturating_sub(1);
                    // Clamped with the frame it clamps to, so a finished player
                    // does not keep counting positions no frame exists for.
                    self.position = self.frame as u64;
                    self.finished = true;
                    return;
                }
            }
        }
    }

    /// Advances to wherever the movie's own audio has got to, in seconds.
    ///
    /// **This is the audio clock ADR-0019 requires**, and the alternative to
    /// [`Player::update`] rather than an addition to it: a movie with a track
    /// is paced by one or the other on any given tick, never both, because two
    /// clocks on one playhead is the two-playhead bug this file has already
    /// had once.
    ///
    /// # Why audio leads and video follows
    ///
    /// A sound card consumes samples at its own rate and cannot be asked to
    /// wait. Stretching the picture to fit is invisible - a frame held or
    /// dropped at 30 Hz is 33 ms - where stretching the sound is a click. So
    /// the playhead is read off the mixer and the frame number is derived from
    /// it, which makes drift structurally impossible rather than merely small:
    /// there is no second accumulator to disagree with.
    ///
    /// # It only ever goes forwards
    ///
    /// Clamped against the position already reached, because a [`Feed`] hands
    /// each frame over exactly once and compares positions to do it - see
    /// [`Ring::take_upto`]. A playhead that went backwards would ask for a
    /// frame the ring had already dropped and get nothing, freezing the
    /// picture. Nothing should make it go backwards; the clamp is what stops a
    /// resampler's rounding from mattering if it did.
    /// # A held picture is discounted rather than skipped over
    ///
    /// The `--reel` leg holds the picture for two seconds at three points and
    /// the sound runs on underneath, so the seconds spent held are subtracted
    /// rather than treated as playback. This only works because the caller
    /// keeps calling while the hold is on - see
    /// [`crate::frontend::Frontend::update_intro`] - which is what lets the
    /// hold's start and end both be observed.
    pub fn follow(&mut self, seconds: f64) {
        if self.paused {
            // Where the sound was when the hold began, so its length can be
            // measured when it ends. `get_or_insert` because every tick of the
            // hold arrives here and only the first one is the start of it.
            self.held_from.get_or_insert(seconds);
            return;
        }
        if let Some(from) = self.held_from.take() {
            self.held_seconds += (seconds - from).max(0.0);
        }
        if self.finished || self.frames == 0 {
            return;
        }

        let (num, den) = self.frame_rate;
        // The movie's own rate, not the audio's: a frame index is what the feed
        // is addressed by, and `seconds` is only how far along we are.
        let elapsed = ((seconds - self.held_seconds).max(0.0) * num as f64 / den as f64) as u64;
        let position = elapsed.max(self.position);

        if self.repeat {
            self.position = position;
            // `frames` came from a `usize`, so the remainder fits one.
            self.frame = (position % self.frames as u64) as usize;
        } else if position >= self.frames as u64 {
            // Clamped with the frame it clamps to, exactly as `update` does.
            self.frame = self.frames - 1;
            self.position = self.frame as u64;
            self.finished = true;
        } else {
            self.position = position;
            self.frame = position as usize;
        }
    }

    /// The frame that should be on screen, counting from zero.
    #[must_use]
    pub fn frame(&self) -> usize {
        self.frame
    }

    /// How many frames have gone by since playback started, counting loops.
    ///
    /// [`Player::frame`] wraps and this does not, which is the whole reason it
    /// exists: a [`Feed`] decodes forward forever and needs a number that only
    /// ever goes up to compare against. Frame 12 of the third time round a
    /// 270-frame loop is position 552, and there is no confusing it with frame 12
    /// of the first.
    ///
    /// The invariant, and it is tested: `position % frames == frame` for a movie
    /// that repeats, and `position == frame` for one that does not.
    #[must_use]
    pub fn position(&self) -> u64 {
        self.position
    }

    /// The frame number as the original counts it, from one.
    ///
    /// The intro state compares against 144, 231 and 260, and those are counts
    /// of frames produced rather than a zero-based index.
    #[must_use]
    pub fn frames_produced(&self) -> usize {
        self.frame + 1
    }

    /// Whether playback has run out of frames.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Whether playback is paused.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Holds the current frame.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resumes from the current frame.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Total frames available.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One frame at 30000/1001 Hz, to the nanosecond.
    const FRAME: f64 = 1001.0 / 30_000.0;

    #[test]
    fn advances_one_frame_per_period() {
        let mut player = Player::new(100, false, FRAME_RATE);
        assert_eq!(player.frame(), 0);
        player.update(FRAME);
        assert_eq!(player.frame(), 1);
        player.update(FRAME * 2.0);
        assert_eq!(player.frame(), 3);
    }

    #[test]
    fn a_short_step_does_not_advance_but_is_not_lost() {
        let mut player = Player::new(100, false, FRAME_RATE);
        player.update(FRAME / 2.0);
        assert_eq!(player.frame(), 0);
        player.update(FRAME / 2.0);
        assert_eq!(player.frame(), 1, "the remainder carries");
    }

    #[test]
    fn pausing_holds_the_frame() {
        let mut player = Player::new(100, false, FRAME_RATE);
        player.update(FRAME * 10.0);
        player.pause();
        player.update(FRAME * 10.0);
        assert_eq!(player.frame(), 10);
        player.resume();
        player.update(FRAME);
        assert_eq!(player.frame(), 11);
    }

    #[test]
    fn finishing_clamps_to_the_last_frame() {
        let mut player = Player::new(5, false, FRAME_RATE);
        player.update(FRAME * 100.0);
        assert!(player.is_finished());
        assert_eq!(player.frame(), 4);
    }

    #[test]
    fn repeating_wraps_instead_of_finishing() {
        let mut player = Player::new(5, true, FRAME_RATE);
        player.update(FRAME * 5.0);
        assert!(!player.is_finished());
        assert_eq!(player.frame(), 0);
    }

    #[test]
    fn an_empty_movie_is_finished_immediately() {
        let player = Player::new(0, false, FRAME_RATE);
        assert!(player.is_finished());
    }

    #[test]
    fn frames_produced_counts_from_one() {
        let mut player = Player::new(300, false, FRAME_RATE);
        // Stepped one frame at a time, the way the game does, rather than in one
        // big jump: 259 steps from frame 0 lands on frame 259, the 260th frame.
        // This is what the intro's `260` counter compares against.
        for _ in 0..259 {
            player.update(FRAME);
        }
        assert_eq!(player.frame(), 259);
        assert_eq!(player.frames_produced(), 260);
    }

    /// The audio clock names the frame the picture should be on, which is the
    /// whole of what [`Player::follow`] promises.
    #[test]
    fn following_the_audio_names_the_frame_that_second_belongs_to() {
        let mut player = Player::new(1200, false, FRAME_RATE);
        player.follow(0.0);
        assert_eq!(player.frame(), 0);
        // One second of sound is 29.97 frames of picture, so frame 29.
        player.follow(1.0);
        assert_eq!(player.frame(), 29);
        player.follow(10.0);
        assert_eq!(player.frame(), 299);
    }

    /// A slow tick moves the playhead several frames at once and the picture
    /// has to jump rather than crawl - the opposite of an accumulator, and the
    /// reason audio clocking cannot drift.
    #[test]
    fn a_jump_in_the_audio_takes_the_picture_with_it() {
        let mut player = Player::new(1200, false, FRAME_RATE);
        player.follow(0.5);
        player.follow(20.0);
        assert_eq!(player.frame(), 599);
        assert_eq!(player.position(), 599);
    }

    /// A [`Feed`] hands each position over once, so a playhead that went
    /// backwards would ask for a frame the ring had already dropped and freeze
    /// the picture.
    #[test]
    fn the_audio_clock_never_runs_the_picture_backwards() {
        let mut player = Player::new(1200, false, FRAME_RATE);
        player.follow(5.0);
        let position = player.position();
        player.follow(4.0);
        assert_eq!(player.position(), position, "a rewind is ignored");
        assert_eq!(player.frame(), position as usize);
    }

    /// A movie that does not repeat ends on the audio clock too, and clamps to
    /// its last frame exactly as `update` does - the intro's `AutoRedirect`
    /// depends on `is_finished` becoming true whichever clock got it there.
    #[test]
    fn following_past_the_end_finishes_and_clamps() {
        let mut player = Player::new(5, false, FRAME_RATE);
        player.follow(100.0);
        assert!(player.is_finished());
        assert_eq!(player.frame(), 4);
        assert_eq!(player.position(), 4);
    }

    /// The invariant a [`Feed`] shares with the player, on the other clock:
    /// `position % frames == frame`, however the position was arrived at.
    #[test]
    fn following_wraps_a_repeating_movie_where_the_feed_wraps() {
        let mut player = Player::new(30, true, FRAME_RATE);
        for seconds in [0.0, 0.5, 1.0, 2.0, 3.5] {
            player.follow(seconds);
            assert_eq!(
                frame_at(player.position(), 30, true),
                Some(player.frame()),
                "the feed and the player disagree at {seconds}s"
            );
        }
        assert!(!player.is_finished());
    }

    /// The `--reel` leg holds the picture for two seconds at a time and the
    /// sound cannot be held with it, so those seconds are discounted. Without
    /// this the picture jumps sixty frames the instant the hold ends - which is
    /// the whole of what audio clocking is supposed to prevent, arriving by a
    /// different door.
    #[test]
    fn seconds_spent_holding_the_picture_are_discounted_from_the_audio_clock() {
        let mut player = Player::new(1200, false, FRAME_RATE);
        player.follow(1.0);
        assert_eq!(player.frame(), 29);

        // The hold: the caller keeps calling while the sound runs on.
        player.pause();
        for tick in 0..120 {
            player.follow(1.0 + f64::from(tick) / 60.0);
        }
        assert_eq!(player.frame(), 29, "the picture is held");

        player.resume();
        // Two seconds of sound went by, so the next frame is the next frame -
        // not the one sixty frames further on.
        player.follow(3.0 + 1.0 / 60.0);
        assert_eq!(
            player.frame(),
            29,
            "the hold is discounted rather than played through"
        );
        player.follow(4.0);
        assert_eq!(player.frame(), 59, "and a second on from there is a second");
    }

    #[test]
    fn a_paused_player_ignores_the_audio_clock_too() {
        let mut player = Player::new(1200, false, FRAME_RATE);
        player.follow(1.0);
        player.pause();
        player.follow(20.0);
        assert_eq!(player.frame(), 29, "paused is paused on either clock");
        player.resume();
        player.follow(20.0);
        assert_eq!(player.frame(), 599);
    }

    /// One audio packet, built the way a `.PMF` builds them: a four-byte
    /// sub-header, then frames that are an eight-byte header and a block.
    fn audio_packet(pointer: u16, frames: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![0, 0];
        out.extend_from_slice(&pointer.to_be_bytes());
        for frame in frames {
            out.extend_from_slice(frame);
        }
        out
    }

    fn atrac_frame(block_align: usize, fill: u8) -> Vec<u8> {
        let mut out = Vec::from(ATRAC3PLUS_SYNC);
        out.extend_from_slice(&[0x28, 0x5c, 0, 0, 0, 0]);
        out.extend_from_slice(&vec![fill; block_align]);
        out
    }

    fn stereo_44k() -> pmf::AudioStream {
        pmf::AudioStream {
            channels: 2,
            frequency_code: 2,
        }
    }

    /// Both layers of framing come off and the block size is measured rather
    /// than assumed - the disc uses 744 and 560, so a constant would decode one
    /// of them into noise.
    #[test]
    fn the_audio_framing_comes_off_and_the_block_size_is_measured() {
        let demuxed = pmf::Demuxed {
            audio: vec![audio_packet(
                0,
                &[atrac_frame(744, 0xab), atrac_frame(744, 0xcd)],
            )],
            ..pmf::Demuxed::default()
        };
        let audio = movie_audio(&demuxed, stereo_44k(), "test").expect("a track");

        assert_eq!(audio.format.block_align, 744, "measured, not assumed");
        assert_eq!(audio.format.channels, 2);
        assert_eq!(audio.format.sample_rate, 44_100);
        assert_eq!(audio.block_count(), 2);
        assert_eq!(audio.blocks.len(), 744 * 2);
        assert!(
            audio.blocks[..744].iter().all(|&b| b == 0xab),
            "the sync word and its header should be gone"
        );
        assert!(audio.blocks[744..].iter().all(|&b| b == 0xcd));
    }

    /// A frame straddles the packet boundary in a real `.PMF` - `Intro.PMF`'s
    /// first packet ends 243 bytes into its third frame - so reassembly has to
    /// be a concatenation rather than a frame per packet.
    #[test]
    fn a_frame_split_across_two_packets_is_put_back_together() {
        let whole = atrac_frame(560, 0x11);
        let (head, tail) = whole.split_at(200);
        let demuxed = pmf::Demuxed {
            audio: vec![
                audio_packet(0, &[atrac_frame(560, 0x22), head.to_vec()]),
                // 368 bytes of the previous frame before the next one starts.
                audio_packet(368, &[tail.to_vec(), atrac_frame(560, 0x33)]),
            ],
            ..pmf::Demuxed::default()
        };
        let audio = movie_audio(&demuxed, stereo_44k(), "test").expect("a track");
        assert_eq!(audio.format.block_align, 560);
        assert_eq!(audio.block_count(), 3);
        assert!(audio.blocks[560..1120].iter().all(|&b| b == 0x11));
    }

    /// A movie whose first packet opens partway into a frame skips to the
    /// frame boundary rather than handing `ffmpeg` a fragment. No movie on the
    /// disc does this, and a decode half a block out is silent noise rather
    /// than an error, so it is pinned rather than left to chance.
    #[test]
    fn a_first_packet_that_opens_mid_frame_is_skipped_to_the_boundary() {
        let mut packet = audio_packet(16, &[]);
        packet.extend_from_slice(&[0xff; 16]);
        packet.extend_from_slice(&atrac_frame(560, 0x44));
        packet.extend_from_slice(&atrac_frame(560, 0x44));
        let demuxed = pmf::Demuxed {
            audio: vec![packet],
            ..pmf::Demuxed::default()
        };
        let audio = movie_audio(&demuxed, stereo_44k(), "test").expect("a track");
        assert_eq!(audio.block_count(), 2);
        assert!(audio.blocks.iter().all(|&b| b == 0x44));
    }

    /// Every way a track can be unreadable is silence rather than a failure,
    /// because none of them should cost the movie its picture.
    #[test]
    fn an_unreadable_track_is_silence_rather_than_an_error() {
        let good = audio_packet(0, &[atrac_frame(560, 0x55), atrac_frame(560, 0x66)]);

        // A header that declares audio and a demux that found none.
        assert!(movie_audio(&pmf::Demuxed::default(), stereo_44k(), "test").is_none());

        // A sample-rate code this build does not know.
        let unknown = pmf::AudioStream {
            channels: 2,
            frequency_code: 7,
        };
        let demuxed = pmf::Demuxed {
            audio: vec![good.clone()],
            ..pmf::Demuxed::default()
        };
        assert!(movie_audio(&demuxed, unknown, "test").is_none());

        // A first frame with no sync word at all.
        let mut wrong = good.clone();
        wrong[4] = 0x00;
        let demuxed = pmf::Demuxed {
            audio: vec![wrong],
            ..pmf::Demuxed::default()
        };
        assert!(movie_audio(&demuxed, stereo_44k(), "test").is_none());

        // One sync word and never a second, so no stride can be measured.
        let demuxed = pmf::Demuxed {
            audio: vec![audio_packet(0, &[atrac_frame(560, 0x77)])],
            ..pmf::Demuxed::default()
        };
        assert!(movie_audio(&demuxed, stereo_44k(), "test").is_none());
    }

    /// Block granularity, stated as such: a track is always a whole number of
    /// 2,048-sample blocks and so runs slightly past the movie's own duration.
    #[test]
    fn a_tracks_length_is_a_whole_number_of_blocks() {
        let demuxed = pmf::Demuxed {
            audio: vec![audio_packet(
                0,
                &[
                    atrac_frame(744, 0),
                    atrac_frame(744, 0),
                    atrac_frame(744, 0),
                ],
            )],
            ..pmf::Demuxed::default()
        };
        let audio = movie_audio(&demuxed, stereo_44k(), "test").expect("a track");
        assert_eq!(audio.block_count(), 3);
        let expected = 3.0 * 2048.0 / 44_100.0;
        assert!((audio.seconds() - expected).abs() < 1e-9);
    }

    /// **An uncapped conversion still knows how many frames it will make.**
    ///
    /// The regression this pins: `Extent::Whole` used to become a bare `None`
    /// that meant both "do not pass `-frames:v`" and "there is no total", so the
    /// PS2's `.PSS` transcode reported frame numbers with no denominator and the
    /// loading screen's bar stood still through the longest wait in the boot.
    /// The container was measured before any of it started.
    #[test]
    fn an_uncapped_conversion_keeps_the_count_it_was_measured_at() {
        let whole = Frames::plan(Extent::Whole, 950);
        assert_eq!(
            whole.cap, None,
            "`-frames:v` must stay off, or the cache file is renamed and the \
             encode is truncated"
        );
        assert_eq!(
            whole.total,
            Some(950),
            "the denominator the progress report divides by"
        );

        let capped = Frames::plan(Extent::Frames(30), 950);
        assert_eq!(capped.cap, Some(30));
        assert_eq!(
            capped.total,
            Some(30),
            "a capped encode makes exactly its cap, so the two agree"
        );
    }

    /// **A cache file that is there is used; one that is not is not invented.**
    ///
    /// [`cached`] is what makes an existing cache file beat the platform decoder
    /// with no flag, so the two ways it can be wrong both matter: a false miss
    /// sends every boot back through GStreamer for 4.80 s, and a false hit hands
    /// back a file that is not this movie. It answers from the name
    /// [`cache_name`] builds and from opening the container, and never converts.
    #[test]
    fn a_cache_lookup_finds_only_a_file_that_is_really_there() {
        let dir = std::env::temp_dir().join("oag-cached-unit");
        std::fs::create_dir_all(&dir).expect("the temp directory");
        let to = Conversion {
            key: "0badcafe-1234",
            cache_dir: &dir,
            width: 480,
            height: 272,
            refresh: false,
            watch: None,
        };

        assert!(
            cached(to, Frames::capped(1200)).is_none(),
            "nothing is cached under a key nothing has ever written"
        );
        assert!(
            cached(to, Frames::whole(950)).is_none(),
            "and an uncapped lookup does not find one either"
        );

        // A file at the right name that is not a decodable container is a miss,
        // not a hit and not an error: `FrameStore::open` is the check.
        let name = cache_name(to.key, to.width, to.height, Some(1200));
        std::fs::write(dir.join(&name), b"not an IVF file").expect("writing the decoy");
        assert!(
            cached(to, Frames::capped(1200)).is_none(),
            "{name} opened as a movie, which it is not"
        );

        assert!(
            cached(
                Conversion {
                    refresh: true,
                    ..to
                },
                Frames::capped(1200)
            )
            .is_none(),
            "refresh answers None to everything - that is the whole of the flag"
        );

        std::fs::remove_file(dir.join(&name)).expect("cleaning up");
    }

    #[test]
    fn a_frame_limit_caps_at_what_exists() {
        assert_eq!(Extent::Frames(261).limit(1200), 261);
        assert_eq!(Extent::Frames(261).limit(100), 100);
        assert_eq!(Extent::Whole.limit(1200), 1200);
    }

    /// The invariant [`Feed`] depends on: the player's position and the feed's
    /// agree about which frame is which, across as many wraps as you like.
    #[test]
    fn position_counts_through_a_wrap_and_the_frame_follows_it() {
        let mut player = Player::new(5, true, FRAME_RATE);
        for expected in 1..=12u64 {
            player.update(FRAME);
            assert_eq!(player.position(), expected);
            assert_eq!(
                player.frame(),
                (expected % 5) as usize,
                "position {expected} names the wrong frame"
            );
            assert_eq!(
                frame_at(player.position(), 5, true),
                Some(player.frame()),
                "the feed and the player disagree at position {expected}"
            );
        }
    }

    #[test]
    fn a_movie_that_does_not_repeat_has_position_equal_to_frame() {
        let mut player = Player::new(5, false, FRAME_RATE);
        for _ in 0..20 {
            player.update(FRAME);
            assert_eq!(player.position(), player.frame() as u64);
        }
        // Clamped with the frame, rather than counting on past the end.
        assert!(player.is_finished());
        assert_eq!(player.position(), 4);
    }

    #[test]
    fn pausing_holds_the_position_too() {
        let mut player = Player::new(100, true, FRAME_RATE);
        player.update(FRAME * 3.0);
        player.pause();
        player.update(FRAME * 50.0);
        assert_eq!(player.position(), 3);
    }

    #[test]
    fn a_loop_maps_positions_round_and_round() {
        assert_eq!(frame_at(0, 270, true), Some(0));
        assert_eq!(frame_at(269, 270, true), Some(269));
        // The wrap: one past the last frame is the first one again.
        assert_eq!(frame_at(270, 270, true), Some(0));
        assert_eq!(frame_at(271, 270, true), Some(1));
        assert_eq!(frame_at(270 * 4 + 7, 270, true), Some(7));
    }

    #[test]
    fn a_movie_that_does_not_repeat_runs_out() {
        assert_eq!(frame_at(0, 3, false), Some(0));
        assert_eq!(frame_at(2, 3, false), Some(2));
        assert_eq!(frame_at(3, 3, false), None);
        assert_eq!(frame_at(9_999, 3, false), None);
    }

    /// An empty cache has no frame at any position, whichever way it is played.
    /// Without this the modulo below would divide by zero.
    #[test]
    fn an_empty_movie_has_no_frame_anywhere() {
        assert_eq!(frame_at(0, 0, true), None);
        assert_eq!(frame_at(0, 0, false), None);
    }

    fn frame(position: u64) -> Frame {
        Frame {
            position,
            index: position as usize,
            // One byte, standing in for a picture: the ring does not read them.
            picture: VideoFrame {
                bytes: vec![position as u8],
                ..VideoFrame::default()
            },
        }
    }

    fn ring(positions: impl IntoIterator<Item = u64>) -> Ring {
        let mut ring = Ring::new(LOOKAHEAD);
        for position in positions {
            ring.push(frame(position));
        }
        ring
    }

    #[test]
    fn an_empty_ring_has_nothing_to_show() {
        assert!(Ring::new(LOOKAHEAD).take_upto(0).is_none());
        assert!(Ring::new(LOOKAHEAD).take_upto(9_999).is_none());
    }

    #[test]
    fn the_ring_hands_frames_over_in_order() {
        let mut ring = ring(0..3);
        for expected in 0..3 {
            assert_eq!(ring.take_upto(expected).map(|f| f.position), Some(expected));
        }
        assert!(ring.take_upto(2).is_none(), "and only once each");
    }

    /// Taking is a **pop**, and this is the invariant both front-end transition
    /// defects rested on.
    ///
    /// A consumer that asks for a position, uses the frame or throws it away,
    /// and then asks for the same position again gets nothing the second time:
    /// the ring has no memory of what it handed over. That is correct here -
    /// popping is what frees a slot for the worker to decode into - so the
    /// caller is the one that has to keep the picture. `FrontendStage` does,
    /// in `held_backdrop`; before it did, the frame popped on a frame that drew
    /// the intro was gone by the time `Show Logo` wanted to draw it.
    #[test]
    fn a_frame_taken_is_a_frame_gone_even_at_the_same_position() {
        let mut ring = ring(0..3);
        assert_eq!(ring.take_upto(1).map(|f| f.position), Some(1));
        assert!(
            ring.take_upto(1).is_none(),
            "the same position asked twice hands nothing over the second time"
        );
        assert_eq!(
            ring.take_upto(2).map(|f| f.position),
            Some(2),
            "and the ring has moved on rather than been emptied"
        );
    }

    /// The playhead being ahead of the decoder is the ordinary underrun, and the
    /// answer is *nothing* rather than a stale frame or a wrong one: the caller
    /// keeps the picture it has.
    #[test]
    fn a_playhead_past_everything_decoded_gets_the_newest_there_is() {
        let mut ring = ring(0..3);
        assert_eq!(ring.take_upto(100).map(|f| f.position), Some(2));
        assert!(ring.take_upto(100).is_none());
    }

    /// A frame slow enough to move the playhead several frames on must not
    /// replay the frames it skipped. This is the case a plain channel `recv`
    /// gets wrong, and it is why the ring is a ring.
    #[test]
    fn a_jump_forwards_drops_what_it_skipped_and_keeps_the_newest() {
        let mut ring = ring(0..4);
        let taken = ring.take_upto(2).expect("frame 2 is there");
        assert_eq!(taken.position, 2, "the newest at or before, not the oldest");
        // 0 and 1 are gone rather than queued up behind it.
        assert_eq!(ring.take_upto(3).map(|f| f.position), Some(3));
        assert!(ring.take_upto(3).is_none());
    }

    /// The playhead sitting on a frame older than anything the ring holds - the
    /// shape a missed [`Feed::restart`] would produce. Nothing comes out, so the
    /// picture freezes rather than jumping to the wrong frame.
    #[test]
    fn a_playhead_behind_the_ring_gets_nothing() {
        let mut ring = ring(10..13);
        assert!(ring.take_upto(0).is_none());
        assert!(ring.take_upto(9).is_none());
        assert_eq!(ring.take_upto(10).map(|f| f.position), Some(10));
    }

    #[test]
    fn the_ring_is_full_at_the_lookahead_and_empties_as_it_is_read() {
        let mut ring = ring(0..LOOKAHEAD as u64);
        assert!(ring.is_full(), "the worker should park here");
        ring.take_upto(0);
        assert!(!ring.is_full(), "and wake once a frame is taken");
    }

    #[test]
    fn clearing_the_ring_leaves_nothing_behind() {
        let mut ring = ring(0..3);
        ring.clear();
        assert!(ring.take_upto(u64::MAX).is_none());
        assert!(!ring.is_full());
    }
}
