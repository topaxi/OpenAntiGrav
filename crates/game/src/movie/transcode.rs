//! The `ffmpeg` conversions and the cache lookup in front of them: what a
//! conversion is asked for, where it writes, and the process that does it.

use super::*;

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
pub(super) struct Frames {
    pub(super) cap: Option<usize>,
    pub(super) total: Option<usize>,
}

impl Frames {
    /// Encode `n` and stop. The cap is also the count, by definition.
    pub(super) fn capped(n: usize) -> Self {
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
    pub(super) fn whole(available: usize) -> Self {
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
    pub(super) fn plan(extent: Extent, available: usize) -> Self {
        match extent {
            Extent::Whole => Self::whole(available),
            Extent::Frames(n) => Self::capped(n),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Conversion<'a> {
    /// Identifies the source for caching - see [`open`].
    pub(super) key: &'a str,
    pub(super) cache_dir: &'a Path,
    pub(super) width: u32,
    pub(super) height: u32,
    /// Ignore whatever is already cached - see [`Decode::refresh`].
    pub(super) refresh: bool,
    pub(super) watch: Watch<'a>,
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
pub(super) fn cached(to: Conversion<'_>, frames: Frames) -> Option<FrameStore> {
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
pub(super) fn transcode_ipu(
    parsed: &ipf::Ipf<'_>,
    to: Conversion<'_>,
    frames: usize,
) -> Result<FrameStore> {
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
pub(super) fn transcode(video: &[u8], to: Conversion<'_>, frames: usize) -> Result<FrameStore> {
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
pub(super) fn ffmpeg_missing() -> bool {
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
pub(super) fn run_ffmpeg(
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
pub(super) fn ffmpeg_progress(
    stdout: std::process::ChildStdout,
    total: Option<usize>,
    watch: Watch<'_>,
) {
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
