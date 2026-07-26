//! Movie playback: demux here, decode elsewhere, cache the result.
//!
//! A `.PMF` holds H.264 video and ATRAC3+ audio. Demuxing it is ours to do and
//! lives in [`oag_formats::pmf`]. Decoding it is not: per
//! `docs/architecture/adr/0004-asset-pipeline.md`, the original is **converted
//! once into a convenient intermediate and cached**, and playback reads the
//! cache. The conversion runs out of process through `ffmpeg`, so no decoder
//! ships in the workspace.
//!
//! When `ffmpeg` is absent, playback still happens: the player advances its
//! frame counter over the movie's real duration and the renderer draws the black
//! backdrop the `LogoFMV` screen puts behind the movie anyway. That keeps the
//! **sequencing** faithful, which is what the intro state machine actually
//! depends on, and it is honest about having no picture.
//!
//! See `docs/architecture/frontend-boot.md`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use oag_formats::pmf;

/// The rate the PSP presents movie frames at, as a rational.
pub const FRAME_RATE: (u64, u64) = (30_000, 1001);

/// Colour format of the cached frames: planar 8-bit YUV, chroma at half
/// resolution in both axes.
///
/// Chosen because it is what the H.264 decoder produces natively, so the
/// transcode does no colour conversion, and it is 1.5 bytes per pixel rather
/// than 4. The conversion to RGB happens in the fragment shader.
pub const PIXEL_FORMAT: &str = "yuv420p";

/// A movie that has been demuxed, and possibly transcoded.
#[derive(Debug)]
pub struct Movie {
    /// The PSMF header.
    pub header: pmf::Header,
    /// Frames measured by counting H.264 access units.
    pub frame_count: usize,
    /// Video width in pixels.
    pub width: u32,
    /// Video height in pixels.
    pub height: u32,
    /// Where the decoded frames are, if a transcode happened.
    pub frames: Option<FrameStore>,
    /// Why there is no picture, when there is none.
    pub no_picture_reason: Option<String>,
}

/// Decoded frames on disk, read one at a time.
#[derive(Debug)]
pub struct FrameStore {
    path: PathBuf,
    file: std::fs::File,
    frame_len: usize,
    /// How many whole frames the file holds.
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
    /// Reads frame `index` into `out`, which is resized to one frame.
    pub fn read_frame(&mut self, index: usize, out: &mut Vec<u8>) -> Result<()> {
        use std::io::{Read, Seek, SeekFrom};

        if index >= self.len {
            bail!(
                "{} holds {} frames, asked for {index}",
                self.path.display(),
                self.len
            );
        }
        out.resize(self.frame_len, 0);
        self.file
            .seek(SeekFrom::Start((index * self.frame_len) as u64))?;
        self.file.read_exact(out)?;
        Ok(())
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

/// Demuxes a `.PMF` and makes its frames available, transcoding if it must.
///
/// `key` identifies the source for caching. It must change when the bytes do:
/// the callers pass the WAD name hash and the entry size, so a different disc
/// image cannot collide.
pub fn open(blob: &[u8], key: &str, cache_dir: &Path, extent: Extent) -> Result<Movie> {
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
    let wanted = extent.limit(frame_count);

    match transcode(&demuxed.video, key, cache_dir, width, height, wanted) {
        Ok(frames) => Ok(Movie {
            header,
            frame_count,
            width,
            height,
            frames: Some(frames),
            no_picture_reason: None,
        }),
        Err(reason) => Ok(Movie {
            header,
            frame_count,
            width,
            height,
            frames: None,
            no_picture_reason: Some(format!("{reason:#}")),
        }),
    }
}

/// Converts an H.264 elementary stream into raw frames under `cache_dir`.
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
    let luma_len = (width * height) as usize;
    let chroma_width = width.div_ceil(2);
    let chroma_height = height.div_ceil(2);
    let chroma_len = (chroma_width * chroma_height) as usize;
    let frame_len = luma_len + 2 * chroma_len;

    let name = format!(
        "{key}-{width}x{height}-{PIXEL_FORMAT}-{}.raw",
        Extent::Frames(frames).key()
    );
    let out = cache_dir.join(&name);

    let ready = std::fs::metadata(&out).is_ok_and(|m| m.len() == (frame_len * frames) as u64);

    if !ready {
        std::fs::create_dir_all(cache_dir)
            .with_context(|| format!("creating {}", cache_dir.display()))?;

        // Written beside the frames because it is the exact input ffmpeg saw, so
        // a mismatch can be reproduced by hand.
        let es = cache_dir.join(format!("{key}.h264"));
        std::fs::write(&es, video).with_context(|| format!("writing {}", es.display()))?;

        run_ffmpeg(&es, &out, frames)?;
    }

    let len = std::fs::metadata(&out)
        .with_context(|| format!("stat {}", out.display()))?
        .len() as usize
        / frame_len;

    if len == 0 {
        bail!("{} holds no whole frames", out.display());
    }

    Ok(FrameStore {
        file: std::fs::File::open(&out).with_context(|| format!("opening {}", out.display()))?,
        path: out,
        frame_len,
        len,
        luma_len,
        chroma_len,
        chroma_width,
        chroma_height,
    })
}

fn run_ffmpeg(input: &Path, output: &Path, frames: usize) -> Result<()> {
    let status = std::process::Command::new("ffmpeg")
        .arg("-hide_banner")
        .args(["-loglevel", "error"])
        .arg("-y")
        // The elementary stream carries no container timing, so the demuxer has
        // to be named explicitly.
        .args(["-f", "h264"])
        .arg("-i")
        .arg(input)
        .args(["-frames:v", &frames.to_string()])
        .args(["-f", "rawvideo"])
        .args(["-pix_fmt", PIXEL_FORMAT])
        .arg(output)
        .status();

    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!("ffmpeg exited with {status}"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!(
            "ffmpeg is not on PATH. Install it to see the intro video; \
             without it the sequence still plays, with a black picture"
        ),
        Err(e) => Err(e).context("running ffmpeg"),
    }
}

/// Plays a movie: a frame counter, a pause flag, and the pacing rules.
///
/// This is the part the intro state machine talks to, and it is deliberately
/// free of any decoding or rendering. The original's player never reads the pad
/// either; skipping is the *state's* job.
#[derive(Debug)]
pub struct Player {
    frames: usize,
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
    pub fn new(frames: usize, repeat: bool) -> Self {
        Self {
            frames,
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

        let (num, den) = FRAME_RATE;
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
        let mut player = Player::new(100, false);
        assert_eq!(player.frame(), 0);
        player.update(FRAME);
        assert_eq!(player.frame(), 1);
        player.update(FRAME * 2.0);
        assert_eq!(player.frame(), 3);
    }

    #[test]
    fn a_short_step_does_not_advance_but_is_not_lost() {
        let mut player = Player::new(100, false);
        player.update(FRAME / 2.0);
        assert_eq!(player.frame(), 0);
        player.update(FRAME / 2.0);
        assert_eq!(player.frame(), 1, "the remainder carries");
    }

    #[test]
    fn pausing_holds_the_frame() {
        let mut player = Player::new(100, false);
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
        let mut player = Player::new(5, false);
        player.update(FRAME * 100.0);
        assert!(player.is_finished());
        assert_eq!(player.frame(), 4);
    }

    #[test]
    fn repeating_wraps_instead_of_finishing() {
        let mut player = Player::new(5, true);
        player.update(FRAME * 5.0);
        assert!(!player.is_finished());
        assert_eq!(player.frame(), 0);
    }

    #[test]
    fn an_empty_movie_is_finished_immediately() {
        let player = Player::new(0, false);
        assert!(player.is_finished());
    }

    #[test]
    fn frames_produced_counts_from_one() {
        let mut player = Player::new(300, false);
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
