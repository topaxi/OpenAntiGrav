//! Movie playback: demux here, transcode out of process, decode in process.
//!
//! A `.PMF` holds H.264 video and ATRAC3+ audio. Demuxing it is ours to do and
//! lives in [`oag_video::pmf`]. Decoding **H.264** is not: per
//! `docs/architecture/adr/0004-asset-pipeline.md`, the original is converted
//! once and cached, and the conversion runs out of process through `ffmpeg`, so
//! no H.264 decoder ships in the workspace.
//!
//! What the cache holds is **lossless AV1 in an IVF container**, not raw
//! frames: see `docs/architecture/adr/0008-av1-movie-cache.md`. Lossless, so
//! the picture is bit-for-bit what the raw cache used to hold and ADR-0004's
//! fidelity rule is untouched; AV1, because [`oag_video::av1`] decodes it in
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
//! | `oag_ui::frontend::Player` | A frame counter and the pacing rules. No pixels. | The one that ticks |
//! | [`FrameStore`] | One synchronous `read_frame`, straight off the decoder. | Whoever calls it |
//! | [`Feed`] | A [`FrameStore`] on a worker thread, decoding ahead into a ring. | Its own |
//!
//! `Player` and [`Feed`] are deliberately separate and talk only through a
//! **position** - see `Player::position`. The player says where playback has
//! got to; the feed says which decoded frames it has. Nothing about the feed
//! feeds back into the player, which is what makes moving the decode off the
//! render thread safe - see [ADR-0010](../../../docs/architecture/adr/0010-movie-decode-thread.md).
//!
//! See `docs/architecture/frontend-boot.md`.

mod bink;
mod container_audio;
#[cfg(all(target_os = "linux", feature = "native-video"))]
mod gst;
mod mp4;
mod mpeg2_ps;
mod track;
#[cfg(target_arch = "wasm32")]
mod webcodecs;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};

use anyhow::{Context, Result, anyhow, bail};
use log::{info, warn};
use oag_video::{av1, bik, ipf, pmf};

pub use track::MovieAudio;
use track::movie_audio;
// `FRAME_RATE`/`PS2_DISPLAY_ASPECT` moved to `oag_ui::frontend::player` with
// `Player`: both are timing/shape constants a movie is *played* at, not part
// of decoding one, and `Player` is the part of this file the front end talks
// to. See that module's own doc for why the split falls there.
use oag_ui::frontend::{FRAME_RATE, PS2_DISPLAY_ASPECT};

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
    /// changes nothing. The PS2's containers are neither their own shape nor
    /// their tag's - `INTRO512.PSS` is a square 512x512 declaring 4:3 - and are
    /// drawn at [`PS2_DISPLAY_ASPECT`], the frame they were cut to fill, which
    /// is measured off the picture rather than read off the stream.
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
/// `video.wesl` at one format.
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
    /// [`oag_present::perf`]), so a player or a screenshot can tell which tier
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

/// What [`VideoDecoder::frame`] fails with when the frame is not decoded *yet*.
///
/// Only an asynchronous decoder returns it (the browser's WebCodecs, in
/// `webcodecs.rs`), and only the web's inline [`Feed`] sees it: that feed polls
/// once a frame instead of blocking a thread, and retries the same position on
/// its next poll. It is not a failure, so it never marks a feed failed.
#[derive(Debug)]
pub struct Pending;

impl std::fmt::Display for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the frame is not decoded yet")
    }
}

impl std::error::Error for Pending {}

/// Decodes the AV1 movie cache through [`VideoDecoder`].
///
/// A wrapper rather than an inherent impl so [`av1::FrameSource`] stays free
/// of anything `oag-game` specific - `oag-formats` builds without `oag-game`
/// in the tree.
#[derive(Debug)]
#[cfg(not(target_arch = "wasm32"))]
struct Av1CacheDecoder(av1::FrameSource);

#[cfg(not(target_arch = "wasm32"))]
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
    #[cfg(not(target_arch = "wasm32"))]
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

    /// The web build has no AV1 decoder (`re_rav1d` does not build for
    /// `wasm32`), so a cached movie is absent there rather than played.
    #[cfg(target_arch = "wasm32")]
    fn open(path: PathBuf, _width: u32, _height: u32) -> Result<Self> {
        bail!(
            "{}: the web build has no AV1 decoder for the movie cache",
            path.display()
        )
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
/// `position` is the [`oag_ui::frontend::Player::position`] this frame is the picture for, and it
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
/// [`oag_ui::frontend::Player`] has reached, uploads it if there is one, and keeps the frame it
/// already has if there is not.
///
/// # Why it is safe against the determinism rules
///
/// `docs/architecture/determinism.md` requires the simulation to be
/// single-threaded, and this does not touch it. The feed is **write-only toward
/// the GPU**: a [`oag_ui::frontend::Player`] hands it a position and it hands back pixels. Nothing
/// it produces is ever read back into sequencing - the intro's state machine
/// compares [`oag_ui::frontend::Player::frames_produced`] against its own 144, 231 and 260, and
/// that number comes from the player whether a picture ever arrives or not. So
/// no simulation state, and no state the simulation reads, can depend on when a
/// decode finished. See
/// [ADR-0010](../../../docs/architecture/adr/0010-movie-decode-thread.md).
///
/// **What ADR-0019 changed, and what it did not.** That number no longer always
/// comes from a *fixed-timestep* `update`: a movie with a sounding track is
/// paced by [`oag_ui::frontend::Player::follow`] instead, so the sequencing of that movie now
/// depends on the audio device's clock. ADR-0010's argument survives unchanged,
/// because it is about the **decode** thread and this is not it - the feed still
/// cannot influence the player, and a decode that ran long still cannot move a
/// state transition. What is new is a second clock, not a second writer, and it
/// is not the renderer's. Headless runs are unaffected either way: with no
/// device the mixer is advanced by [`oag_sound::Audio::tick`] at exactly the
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
    /// The store and `repeat`, on the web, where there is no worker: the
    /// browser decodes asynchronously, and [`Feed::take_upto`] polls it.
    #[cfg(target_arch = "wasm32")]
    inline: (FrameStore, bool),
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
    /// backdrop loops and the intro does not, exactly as [`oag_ui::frontend::Player::new`] takes
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
        #[cfg(not(target_arch = "wasm32"))]
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
            #[cfg(not(target_arch = "wasm32"))]
            worker: Some(worker),
            #[cfg(target_arch = "wasm32")]
            worker: None,
            len,
            decoder_label,
            chroma_width,
            chroma_height,
            width,
            height,
            #[cfg(target_arch = "wasm32")]
            inline: (store, repeat),
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
        #[cfg(target_arch = "wasm32")]
        {
            let (store, repeat) = &mut self.inline;
            while claim(&self.shared).is_some_and(|(epoch, position)| {
                decode_step(&self.shared, store, self.len, *repeat, epoch, position)
            }) {}
        }
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
    /// Called when a [`oag_ui::frontend::Player`] is rebuilt at frame zero - opening the menus a
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
#[cfg(not(target_arch = "wasm32"))]
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
        decode_step(shared, &mut store, len, repeat, epoch, position);
    }
}

/// The position to decode next and its epoch, or `None` when there is nothing
/// to do: [`decode_loop`]'s wait condition, for the web's feed, which polls
/// rather than waits.
#[cfg(target_arch = "wasm32")]
fn claim(shared: &Shared) -> Option<(u64, u64)> {
    let state = shared
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let idle = state.stop || state.failed || state.done || state.ring.is_full();
    (!idle).then_some((state.epoch, state.next))
}

/// Decodes the claimed `position` and pushes it; `false` when the decoder had
/// it [`Pending`], which is retried on the next poll.
fn decode_step(
    shared: &Shared,
    store: &mut FrameStore,
    len: usize,
    repeat: bool,
    epoch: u64,
    position: u64,
) -> bool {
    let lock = || {
        shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    };
    let Some(index) = frame_at(position, len, repeat) else {
        // A movie that does not repeat, decoded to its end. Its last frame
        // is still in the ring for whoever wants it.
        let mut state = lock();
        if state.epoch == epoch {
            state.done = true;
        }
        return true;
    };

    let mut picture = VideoFrame::default();
    let decoded = store.read_frame(index, &mut picture);

    let mut state = lock();
    // Restarted while this was decoding: the frame belongs to playback that
    // no longer exists, so it is dropped rather than pushed. The next pass
    // round claims position 0 and `read_frame` rewinds by itself.
    if state.epoch != epoch {
        return true;
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
        Err(e) if e.downcast_ref::<Pending>().is_some() => return false,
        Err(e) => {
            state.error = Some(format!("{e:#}"));
            state.failed = true;
        }
    }
    true
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
/// code, an `.IPF`'s `IPUF`, a `.bik`'s `BIK`, or a Wipeout 2048 `.mp4`'s
/// `ftyp` - because what a decode path needs to know is what the file is, not
/// what disc it happened to come off. Wipeout HD is what makes that more than
/// a tidy principle: its 37 `.bik` files are the first video this project
/// reads off a console that is not the one the container was written on, and
/// 2048's 26 `.mp4` files are the second.
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
    // Only a `.PMF`'s H.264 has a decoder in a browser (WebCodecs); the rest
    // are opened for their shape and sound alone, and say why.
    #[cfg(target_arch = "wasm32")]
    if !blob.starts_with(pmf::MAGIC) && !how.no_video {
        let only_shape = Decode {
            no_video: true,
            ..how
        };
        let mut movie =
            open(blob, key, cache_dir, extent, only_shape, watch).context(webcodecs::NO_DECODER)?;
        movie.no_picture_reason = Some(webcodecs::NO_DECODER.to_string());
        return Ok(movie);
    }
    if blob.starts_with(pmf::MAGIC) {
        open_psmf(blob, key, cache_dir, extent, how, watch)
    } else if blob.starts_with(&mpeg2_ps::START_CODE) {
        mpeg2_ps::open(blob, key, cache_dir, extent, how, watch)
    } else if blob.starts_with(&ipf::MAGIC) {
        open_ipuf(blob, key, cache_dir, extent, how, watch)
    } else if bik::is_bink(blob) {
        bink::open(blob, key, cache_dir, extent, how, watch)
    } else if oag_video::mp4::is_mp4(blob) {
        mp4::open(blob, key, cache_dir, extent, how, watch)
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
        warn!(
            "{} byte(s) of {key} were not part of any pack or PES packet",
            demuxed.stray_bytes
        );
    }

    let frame_count = pmf::frame_count(&demuxed.video);
    let expected = header.expected_frame_count() as usize;
    // The header's duration and the elementary stream's access units are two
    // independent measurements of the same thing. Disagreeing by more than a
    // frame means one of them is being read wrong.
    if frame_count.abs_diff(expected) > 1 {
        warn!("{key} has {frame_count} access units but its duration implies {expected}");
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

    // The web has no cache and cannot run `ffmpeg`, but its browser decodes
    // H.264 itself: see `webcodecs.rs`.
    #[cfg(target_arch = "wasm32")]
    let made = webcodecs::frame_store(&demuxed.video, key, width, height, wanted);
    #[cfg(not(target_arch = "wasm32"))]
    let made = transcode(&demuxed.video, conversion, wanted);
    match made {
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
            warn!("platform-native H.264 decode unavailable for {key}: {e:#}");
            return None;
        }
    };

    let geometry = decoder.geometry();
    if (geometry.width, geometry.height) != (width, height) {
        warn!(
            "{key}'s GStreamer decode is {}x{}, but the movie declares {width}x{height}",
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

/// The rate a PS2 `.IPF` backdrop is presented at, by declared width.
///
/// **An `IPUF` container declares no frame rate at all**, so this is inferred
/// rather than read, and the inference is the disc's own pairing:
/// `SCES_547.48`'s `FUN_0019b168` rewrites the front-end XML's
/// `Data\Movies\Backdrop.ipf` to `Data\Movies\bg512.ipf` or
/// `Data\Movies\bg640.ipf` on exactly the same global (`0x0027a85c`) that picks
/// `Intro512.pss` against `Intro640.pss` - and those two *do* declare their
/// rates, measured at 25/1 and 30000/1001. So `bg512` is the 50 Hz cut and
/// `bg640` the 60 Hz one - a player's answer, not a region: `refresh-mode.md`.
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

/// Makes a PS2 `.IPF` backdrop's frames available, transcoding if it must.
///
/// The container is [`oag_video::ipf`]: fixed-size slots around one IPU
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
            display_aspect: PS2_DISPLAY_ASPECT,
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
            display_aspect: PS2_DISPLAY_ASPECT,
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
            display_aspect: PS2_DISPLAY_ASPECT,
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
#[cfg(not(target_arch = "wasm32"))]
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

/// Whether `ffmpeg` is absent, probed once per process; the first miss logs
/// the one line a player needs, and every later movie skips quietly.
///
/// Without it every conversion would fail on its own, and the reason would sit
/// in a debug-level loader report while the screen shows a black
/// `INTRO FRAME n / N (NO PICTURE)`. One `warn` names the cause and the fix.
fn ffmpeg_missing() -> bool {
    static MISSING: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *MISSING.get_or_init(|| {
        let missing = matches!(
            std::process::Command::new("ffmpeg").arg("-version").output(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound
        );
        if missing {
            warn!(
                "ffmpeg is not installed, so the intro is skipped and movies show no \
                 picture (the game is otherwise fine). Install ffmpeg to see them; \
                 `--no-video` hides this message"
            );
        }
        missing
    })
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
    if ffmpeg_missing() {
        bail!(
            "ffmpeg is not on PATH. Install it to see the intro video; \
             without it the intro is skipped"
        );
    }
    // Said before rather than after, because the whole 1200-frame intro takes
    // about 80 seconds and silence for that long reads as a hang. It happens
    // once per movie: the result is cached.
    info!(
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
             without it the intro is skipped"
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

#[cfg(test)]
mod tests;
