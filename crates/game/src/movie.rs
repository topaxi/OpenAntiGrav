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
//! See `docs/architecture/frontend-boot.md`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use oag_formats::{av1, pmf};

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
    /// The PSMF header, for a `.PMF`. `None` for a raw MPEG-2 program
    /// stream (the PS2's loose `.PSS` files), which carries no such header -
    /// see [`open_mpeg2_ps`].
    pub header: Option<pmf::Header>,
    /// Frames measured by counting H.264 access units for a `.PMF`, or
    /// decoded by `ffmpeg` for a raw MPEG-2 program stream.
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
/// from - a `.PMF`'s `PSMF` header, or a raw MPEG-2 program stream's pack
/// start code - because what a decode path needs to know is what the file is,
/// not what disc it happened to come off.
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

            if self.frame >= self.frames {
                if self.repeat {
                    self.frame = 0;
                } else {
                    self.frame = self.frames.saturating_sub(1);
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
}
