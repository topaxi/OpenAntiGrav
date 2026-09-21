//! Wipeout 2048's `.mp4` movies: ISOBMFF, handed to `ffmpeg` whole.
//!
//! The fourth console and the fifth container, and it arrives with the same
//! shape `.bik` did: the header is read by [`oag_video::mp4`] and everything
//! after that is the cache
//! [ADR-0008](../../../../docs/architecture/adr/0008-av1-movie-cache.md)
//! already built. Like [`super::bink`] and unlike the `.PMF` and `.IPF`
//! paths, nothing here demuxes: `ffmpeg` reads the container itself, so the
//! transcode input is the disc's own bytes unaltered.
//!
//! # Why `ffmpeg` and not a decoder in the workspace
//!
//! The codec here is H.264 (every file) and, on `intro.mp4` alone, AAC -
//! [ADR-0024](../../../../docs/architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md)'s
//! question, asked fresh rather than inherited from `.bik`'s answer:
//!
//! - **Category 1, a pure-Rust library.** No permissively-licensed H.264
//!   decoder exists on crates.io to speak of: `openh264` is an FFI binding to
//!   Cisco's C library (not a Rust decoder), and the actual Rust
//!   implementations (`nihav`'s H.264 support among them) are not published
//!   there. AAC has more candidates, but a second in-process decoder buys
//!   nothing when the video track next to it still needs `ffmpeg`.
//! - **Category 2, our own decoder.** Closed by the rule that already closed
//!   it for Bink: H.264 and AAC are published standards this project did not
//!   invent, and a wrong decoder for either fails as plausible-looking noise,
//!   the exact failure mode the rule exists to keep out of category 2.
//! - **Category 3 it is**, the same conclusion `.bik` reached, derived fresh
//!   for a different codec pair rather than assumed because the pipe already
//!   exists.
//!
//! The **container** is a different question from the codec and is read by
//! [`oag_video::mp4`] - see that module's own "Why hand-rolled, and why the
//! container is not the codec" section for why hand-reading eight box types
//! was chosen over a pure-Rust ISOBMFF demuxer that does exist to take
//! instead. That is what buys a machine with no `ffmpeg` a correct frame
//! count, size and rate off any of the 26 files, same as `.bik`.

use super::*;
use log::warn;

/// Makes an MP4 file's frames available, transcoding if it must.
///
/// Every number in the returned [`Movie`] comes off the container's own
/// header rather than from `ffprobe` - see [`bink::open`]'s own doc for why
/// that is worth stating: a machine with no `ffmpeg` still gets a correct
/// frame count, size and rate, so the sequencing stays faithful and only the
/// picture is missing.
pub(super) fn open(
    blob: &[u8],
    key: &str,
    cache_dir: &Path,
    extent: Extent,
    how: Decode,
    watch: Watch<'_>,
) -> Result<Movie> {
    let header = oag_video::mp4::parse(blob)
        .map_err(|e| anyhow!("{e}"))
        .context("parsing the MP4 header")?;

    // Not fatal, on the same terms `.bik`'s two checks are not: a
    // disagreement here means the entry was read short and every count below
    // is then describing a file this is not holding all of, but the file is
    // still worth playing as far as it goes.
    if header.frame_count != header.stts_sample_count {
        warn!(
            "{key}'s stsz sample count ({}) and stts sample count ({}) disagree",
            header.frame_count, header.stts_sample_count
        );
    }
    let expected_duration = header.frame_count as u64 * u64::from(header.frame_delta);
    if header.duration != expected_duration {
        warn!(
            "{key}'s mdhd duration ({}) does not equal frame_count * frame_delta ({expected_duration})",
            header.duration
        );
    }

    let width = header.width;
    let height = header.height;
    let frame_rate = (
        u64::from(header.frame_rate.0),
        u64::from(header.frame_rate.1),
    );

    // Computed before the `--no-video` branch, the same place `bink::open`
    // unwraps its own container-hosted track - see that function's comment on
    // why `--no-video` should not cost this file its sound.
    //
    // **AAC is inside the container**, the same shape `.bik`'s own audio is -
    // see `docs/formats/bik.md#the-audio-is-inside-the-video-file`. Only
    // `intro.mp4` of the 26 carries a track at all
    // ([`oag_video::mp4::Header::audio`]). `ensure_source` writes the same
    // `{key}.mp4` file `transcode` hands to `ffmpeg` for the picture -
    // written here too so a video-cache hit does not leave the audio route
    // with nothing to read.
    let audio = match &header.audio {
        None => None,
        Some(track) => match ensure_source(cache_dir, key, "mp4", blob) {
            Ok(source) => Some(track::mp4_audio(track, source, key)),
            Err(e) => {
                warn!("{key}'s audio track could not be cached for decoding: {e:#}");
                None
            }
        },
    };

    let movie = move |frames: Option<FrameStore>, no_picture_reason: Option<String>| Movie {
        // A `.PMF`'s PSMF header, and there is no such thing here - see
        // `bink::open`'s own comment on the same field.
        header: None,
        frame_count: header.frame_count,
        width,
        height,
        frame_rate,
        // Square pixels: `intro.mp4`'s `tkhd` declares a 960x544 track
        // presentation size identical to its `avc1` coded picture (measured
        // 2026-09-21), so there is no anamorphic scaling to carry - the same
        // reading `.bik`'s own `display_aspect` comment describes.
        display_aspect: (width, height),
        frames,
        no_picture_reason,
        audio,
    };

    if how.no_video {
        return Ok(movie(None, Some("--no-video was given".to_string())));
    }

    let frames = Frames::plan(extent, header.frame_count);
    let conversion = Conversion {
        key,
        cache_dir,
        width,
        height,
        refresh: how.refresh,
        watch,
    };

    match transcode(blob, conversion, frames) {
        Ok(frames) => Ok(movie(Some(frames), None)),
        Err(reason) => Ok(movie(None, Some(format!("{reason:#}")))),
    }
}

/// Writes `blob` under `cache_dir` as `{key}.{ext}`, unless a file already
/// there is already the right length - see `bink::ensure_source`, which this
/// mirrors exactly for the same reason.
fn ensure_source(cache_dir: &Path, key: &str, ext: &str, blob: &[u8]) -> Result<PathBuf> {
    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating {}", cache_dir.display()))?;
    let path = cache_dir.join(format!("{key}.{ext}"));
    let already_written = std::fs::metadata(&path).is_ok_and(|m| m.len() == blob.len() as u64);
    if !already_written {
        std::fs::write(&path, blob).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(path)
}

/// Converts an MP4 file into lossless AV1 under the cache.
///
/// The input is the **whole file**, exactly as [`bink::transcode`] hands over
/// a whole `.bik`: `ffmpeg`'s own MP4 demuxer reads the container, picks the
/// video stream out of it, and the one audio track any of the 26 files
/// carries (`intro.mp4`'s AAC) goes nowhere, an IVF holding video and
/// nothing else.
fn transcode(blob: &[u8], to: Conversion<'_>, frames: Frames) -> Result<FrameStore> {
    let Conversion {
        key,
        cache_dir,
        width,
        height,
        // Read by `cached` alone, as in every other transcode here.
        refresh: _,
        watch,
    } = to;
    if let Some(store) = cached(to, frames) {
        watched(watch, Step::Cached);
        return Ok(store);
    }
    let out = cache_dir.join(cache_name(key, width, height, frames.cap));

    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("creating {}", cache_dir.display()))?;

    let source = ensure_source(cache_dir, key, "mp4", blob)?;

    run_ffmpeg(&source, &out, None, frames, watch)?;

    let store = FrameStore::open(out, width, height)?;
    if store.len == 0 {
        bail!("{} holds no frames", store.path().display());
    }
    Ok(store)
}
