//! The PS2's loose `.PSS` movies: a raw MPEG-2 program stream, handed to
//! `ffmpeg` whole.
//!
//! Split out of `movie.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change, and the
//! renames are the two below. What holds it together is the thing that makes
//! this container different from the other three: **`ffmpeg` reads it by
//! itself**. A `.PMF` and an `.IPF` are demuxed here first - by
//! [`oag_formats::pmf`] and [`oag_formats::ipf`] - and only their elementary
//! streams reach the transcoder, so those paths stay in `movie.rs` beside the
//! machinery they share. This one and [`super::bink`] hand the file over
//! unaltered, container and all.
//!
//! Two names are shorter here than they were in `movie.rs`, and both shadow
//! something in the parent that `use super::*` brings in: [`open`] against
//! [`super::open`], which is the dispatcher that calls this one, and
//! [`transcode`] against [`super::transcode`], which is the H.264 path. The
//! module qualifies them at every call site, which is why they can afford to
//! be short.

use super::*;

/// The start code every MPEG program stream pack begins with: `00 00 01 BA`.
///
/// A raw MPEG-2 program stream - the PS2's loose `.PSS` files - has no header
/// of its own the way a `.PMF` does, so this is what tells the two apart. See
/// `docs/ps2/pulse-disc-layout.md`.
pub(super) const START_CODE: [u8; 4] = [0x00, 0x00, 0x01, 0xba];

/// Makes a raw MPEG-2 program stream's frames available, transcoding if it
/// must.
///
/// This is the PS2's loose `.PSS` movies: no PSMF wrapper, no separate demux
/// step - `ffmpeg` reads the whole program stream itself, container and all -
/// and no audio stream in the one measured so far (`INTRO512.PSS`), so this
/// never reports one. Width, height and frame rate come from `ffprobe`
/// (see [`probe`]) rather than a header, because there is no header to read
/// them from.
pub(super) fn open(
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
            display_aspect: PS2_DISPLAY_ASPECT,
            frames: None,
            no_picture_reason: Some("--no-video was given".to_string()),
            audio: None,
        });
    }

    let wanted = Frames::plan(extent, probed.frame_count);

    match transcode(
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
            display_aspect: PS2_DISPLAY_ASPECT,
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
            display_aspect: PS2_DISPLAY_ASPECT,
            frames: None,
            no_picture_reason: Some(format!("{reason:#}")),
            audio: None,
        }),
    }
}

/// Converts a raw MPEG-2 program stream into lossless AV1 under `cache_dir`.
///
/// Unlike [`super::transcode`], `video` is the *whole* container - `ffmpeg` demuxes
/// it itself, so there is no elementary stream to extract first, and no input
/// format to name: a program stream carries its own.
fn transcode(video: &[u8], to: Conversion<'_>, frames: Frames) -> Result<FrameStore> {
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

/// What `probe` reads off a raw MPEG-2 program stream with `ffprobe`.
///
/// **No display aspect.** This used to carry the stream's own
/// `display_aspect_ratio` - `4:3` on both cuts - and the picture does not
/// agree with it; a PS2 movie is drawn at [`PS2_DISPLAY_ASPECT`] instead, which
/// is measured rather than declared. Reading a field only to override it would
/// have been the confusing half of both.
struct Probed {
    width: u32,
    height: u32,
    frame_rate: (u64, u64),
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

    let stream = run_ffprobe(&source, "stream", "width,height,r_frame_rate")?;
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
