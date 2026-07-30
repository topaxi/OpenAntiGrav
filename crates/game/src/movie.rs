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

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};

use anyhow::{Context, Result, anyhow, bail};
use oag_formats::{av1, ipf, pmf};

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
    source: av1::FrameSource,
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
            source,
            path,
        })
    }

    /// Decodes frame `index` into `out`, which is resized to one frame.
    pub fn read_frame(&mut self, index: usize, out: &mut Vec<u8>) -> Result<()> {
        self.source
            .frame(index, out)
            .map_err(|e| anyhow!("decoding frame {index} of {}: {e}", self.path.display()))
    }

    /// Where the cache file lives.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
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
    /// Tightly packed I420: luma, then both chroma planes.
    pub bytes: Vec<u8>,
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
/// that number comes from the player's fixed-timestep `update` whether a picture
/// ever arrives or not. So no simulation state, and no state the simulation
/// reads, can depend on when a decode finished. See
/// [ADR-0010](../../../docs/architecture/adr/0010-movie-decode-thread.md).
#[derive(Debug)]
pub struct Feed {
    shared: Arc<Shared>,
    /// `None` only between [`Feed::drop`] taking it and the join finishing.
    worker: Option<std::thread::JoinHandle<()>>,
    /// How many frames the cache holds. Copied out of the [`FrameStore`] before
    /// it moved onto the worker, because the player needs it and the store is no
    /// longer reachable from here.
    len: usize,
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

        let mut bytes = Vec::new();
        let decoded = store.read_frame(index, &mut bytes);

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
                    bytes,
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
    fn key(self) -> String {
        match self {
            Self::Whole => "all".to_string(),
            Self::Frames(n) => n.to_string(),
        }
    }

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
///
/// `no_video` skips the transcode and reports a picture-less movie instead -
/// still with a correct frame count, width, height and frame rate, since
/// those come from parsing the container rather than decoding it.
pub fn open(
    blob: &[u8],
    key: &str,
    cache_dir: &Path,
    extent: Extent,
    no_video: bool,
) -> Result<Movie> {
    if blob.starts_with(pmf::MAGIC) {
        open_psmf(blob, key, cache_dir, extent, no_video)
    } else if blob.starts_with(&MPEG_PS_START_CODE) {
        open_mpeg2_ps(blob, key, cache_dir, extent, no_video)
    } else if blob.starts_with(&ipf::MAGIC) {
        open_ipuf(blob, key, cache_dir, extent, no_video)
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
    no_video: bool,
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

    if no_video {
        return Ok(Movie {
            header: Some(header),
            frame_count,
            width,
            height,
            frame_rate: FRAME_RATE,
            display_aspect: (width, height),
            frames: None,
            no_picture_reason: Some("--no-video was given".to_string()),
        });
    }

    let wanted = extent.limit(frame_count);

    match transcode(&demuxed.video, key, cache_dir, width, height, wanted) {
        Ok(frames) => Ok(Movie {
            header: Some(header),
            frame_count,
            width,
            height,
            frame_rate: FRAME_RATE,
            display_aspect: (width, height),
            frames: Some(frames),
            no_picture_reason: None,
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
        }),
    }
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
    no_video: bool,
) -> Result<Movie> {
    let probed = probe(blob, key, cache_dir)?;

    if no_video {
        return Ok(Movie {
            header: None,
            frame_count: probed.frame_count,
            width: probed.width,
            height: probed.height,
            frame_rate: probed.frame_rate,
            display_aspect: probed.display_aspect,
            frames: None,
            no_picture_reason: Some("--no-video was given".to_string()),
        });
    }

    let wanted = match extent {
        Extent::Whole => None,
        Extent::Frames(n) => Some(n),
    };

    match transcode_mpeg2_ps(blob, key, cache_dir, probed.width, probed.height, wanted) {
        Ok(frames) => Ok(Movie {
            header: None,
            frame_count: frames.len,
            width: probed.width,
            height: probed.height,
            frame_rate: probed.frame_rate,
            display_aspect: probed.display_aspect,
            frames: Some(frames),
            no_picture_reason: None,
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
    no_video: bool,
) -> Result<Movie> {
    let parsed = ipf::parse(blob).map_err(|e| anyhow!("parsing {key} as an IPF: {e}"))?;
    let width = parsed.header.width;
    let height = parsed.header.height;
    let frame_count = parsed.len();
    let frame_rate = backdrop_frame_rate(width);

    if no_video {
        return Ok(Movie {
            header: None,
            frame_count,
            width,
            height,
            frame_rate,
            display_aspect: BACKDROP_DISPLAY_ASPECT,
            frames: None,
            no_picture_reason: Some("--no-video was given".to_string()),
        });
    }

    let wanted = extent.limit(frame_count);

    match transcode_ipu(&parsed, key, cache_dir, width, height, wanted) {
        Ok(frames) => Ok(Movie {
            header: None,
            frame_count,
            width,
            height,
            frame_rate,
            display_aspect: BACKDROP_DISPLAY_ASPECT,
            frames: Some(frames),
            no_picture_reason: None,
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

/// Converts a PS2 `.IPF`'s IPU bitstream into lossless AV1 under `cache_dir`.
fn transcode_ipu(
    parsed: &ipf::Ipf<'_>,
    key: &str,
    cache_dir: &Path,
    width: u32,
    height: u32,
    frames: usize,
) -> Result<FrameStore> {
    let name = format!(
        "{key}-{width}x{height}-{CACHE_CODEC}-{}.ivf",
        Extent::Frames(frames).key()
    );
    let out = cache_dir.join(&name);

    let ready = FrameStore::open(out.clone(), width, height)
        .ok()
        .filter(|store| store.len == frames);

    if let Some(store) = ready {
        return Ok(store);
    }

    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating {}", cache_dir.display()))?;

    let source = cache_dir.join(format!("{key}.ipu"));
    let wrapped = ipum(&parsed.bitstream(), width, height, parsed.len());
    std::fs::write(&source, &wrapped).with_context(|| format!("writing {}", source.display()))?;

    run_ffmpeg(&source, &out, Some("ipu"), Some(frames))?;

    let store = FrameStore::open(out, width, height)?;
    if store.len == 0 {
        bail!("{} holds no frames", store.path().display());
    }
    Ok(store)
}

/// Converts an H.264 elementary stream into lossless AV1 under `cache_dir`.
///
/// Returns the reason as an error when conversion is impossible, which the
/// caller turns into a fallback rather than a failure.
fn transcode(
    video: &[u8],
    key: &str,
    cache_dir: &Path,
    width: u32,
    height: u32,
    frames: usize,
) -> Result<FrameStore> {
    let name = format!(
        "{key}-{width}x{height}-{CACHE_CODEC}-{}.ivf",
        Extent::Frames(frames).key()
    );
    let out = cache_dir.join(&name);

    // A previous run's file is reused only if it opens *and* holds the frames
    // asked for. Unlike the raw cache this cannot be checked by file length, so
    // it is checked by decoding the container - cheap, since that is a parse of
    // the frame headers and not of the pictures.
    let ready = FrameStore::open(out.clone(), width, height)
        .ok()
        .filter(|store| store.len == frames);

    if let Some(store) = ready {
        return Ok(store);
    }

    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating {}", cache_dir.display()))?;

    // Written beside the cache because it is the exact input ffmpeg saw, so a
    // mismatch can be reproduced by hand.
    let es = cache_dir.join(format!("{key}.h264"));
    std::fs::write(&es, video).with_context(|| format!("writing {}", es.display()))?;

    // The elementary stream carries no container timing, so the demuxer has to
    // be named explicitly.
    run_ffmpeg(&es, &out, Some("h264"), Some(frames))?;

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
fn transcode_mpeg2_ps(
    video: &[u8],
    key: &str,
    cache_dir: &Path,
    width: u32,
    height: u32,
    frames: Option<usize>,
) -> Result<FrameStore> {
    let name = format!(
        "{key}-{width}x{height}-{CACHE_CODEC}-{}.ivf",
        frames.map_or_else(|| "all".to_string(), |n| n.to_string())
    );
    let out = cache_dir.join(&name);

    let ready = FrameStore::open(out.clone(), width, height)
        .ok()
        .filter(|store| frames.is_none_or(|n| store.len == n));

    if let Some(store) = ready {
        return Ok(store);
    }

    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating {}", cache_dir.display()))?;

    let source = cache_dir.join(format!("{key}.pss"));
    std::fs::write(&source, video).with_context(|| format!("writing {}", source.display()))?;

    run_ffmpeg(&source, &out, None, frames)?;

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
/// a raw MPEG-2 program stream carries. `frames` caps how many pictures are
/// encoded; `None` converts the whole input.
fn run_ffmpeg(
    input: &Path,
    output: &Path,
    input_format: Option<&str>,
    frames: Option<usize>,
) -> Result<()> {
    // Said before rather than after, because the whole 1200-frame intro takes
    // about 80 seconds and silence for that long reads as a hang. It happens
    // once per movie: the result is cached.
    eprintln!(
        "transcoding {} into {} (once; cached after this)",
        frames.map_or_else(|| "every frame".to_string(), |n| format!("{n} frame(s)")),
        output.display()
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
    if let Some(frames) = frames {
        command.args(["-frames:v", &frames.to_string()]);
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
        .status();

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
            bytes: vec![position as u8],
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
