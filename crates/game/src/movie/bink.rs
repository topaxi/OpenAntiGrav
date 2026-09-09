//! Wipeout HD's `.bik` movies: RAD's Bink container, handed to `ffmpeg` whole.
//!
//! The third console and the fourth container, and the one that arrives with
//! the least new machinery behind it: the header is read by
//! [`oag_video::bik`] and everything after that is the cache
//! [ADR-0008](../../../../docs/architecture/adr/0008-av1-movie-cache.md) already
//! built. Like [`super::mpeg2_ps`] and unlike the `.PMF` and `.IPF` paths,
//! nothing here demuxes: `ffmpeg` reads the container itself, so the transcode
//! input is the disc's own bytes unaltered.
//!
//! # Why `ffmpeg` and not a decoder in the workspace
//!
//! [ADR-0024](../../../../docs/architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md)
//! asks for the check to be *made* rather than inherited from "video means
//! `ffmpeg`", so it was made on 2026-08-17 and this is the result:
//!
//! - **Category 1, a pure-Rust library: one candidate, and its licence closes
//!   it.** `infinitier_bik_decoder` 0.0.14 is the only Bink decoder published on
//!   crates.io, it does target Bink 1 `BIKi` - which is what all 37 files are -
//!   and it is **GPL-3.0-or-later**. That is the strong copyleft the ADR's
//!   licence bar names as the one kind that is not acceptable, because it would
//!   reach this project's own sources. Nothing else exists: `nihav` carries Bink
//!   decoders and is not published to crates.io at all.
//! - **Category 2, our own decoder: closed by the rule, not by effort.** Bink is
//!   RAD's format, not Wipeout's. It is exactly the "published standard we did
//!   not invent" case the ADR says must never land here, and a wrong Bink
//!   decoder fails the same way a wrong MP3 decoder does - plausible-looking
//!   noise.
//! - **Category 3 it is**, which is where the ADR already puts the video path,
//!   and this is the version of that sentence that was derived rather than
//!   assumed.
//!
//! The container header is a different question from the codec and is read
//! here, the way `.PMF`'s and `.IPF`'s are: see [`oag_video::bik`]'s own docs
//! for why that is not the rejected category-2 case, and for what a file that
//! has no `ffmpeg` to hand still knows about itself.

use super::*;
use log::warn;

/// Makes a Bink file's frames available, transcoding if it must.
///
/// Every number in the returned [`Movie`] comes off the container's own header
/// rather than from `ffprobe`, which is the one way this differs from
/// [`super::mpeg2_ps::open`] and it is worth the difference: a machine with no
/// `ffmpeg` still gets a correct frame count, size and rate, so the sequencing
/// stays faithful and only the picture is missing. That is the degradation the
/// module docs promise for `.PMF`, and a program stream cannot offer it because
/// it carries no header to read.
pub(super) fn open(
    blob: &[u8],
    key: &str,
    cache_dir: &Path,
    extent: Extent,
    how: Decode,
    watch: Watch<'_>,
) -> Result<Movie> {
    let header = bik::parse(blob)
        .map_err(|e| anyhow!("{e}"))
        .context("parsing the Bink header")?;

    // Not fatal, and reported for the same reason a `.PMF`'s stray bytes are:
    // it means the entry was read short or long, and every count below is then
    // describing a file this is not holding all of.
    if !header.declares_length_of(blob.len()) {
        warn!(
            "{key} declares {} byte(s) and {} were read",
            header.declared_len,
            blob.len()
        );
    }
    // The invariant that validates the header's variable middle - see
    // `oag_video::bik::first_frame_offset`. Reported rather than fatal:
    // a file this project has not seen is worth looking at, not refusing.
    if let Some(first) = oag_video::bik::first_frame_offset(blob, &header)
        && first != header.header_len
    {
        warn!(
            "{key}'s first frame is at {first} where its header ends at {}",
            header.header_len
        );
    }

    let width = header.width;
    let height = header.height;
    let frame_rate = (
        u64::from(header.frame_rate.0),
        u64::from(header.frame_rate.1),
    );

    let movie = |frames: Option<FrameStore>, no_picture_reason: Option<String>| Movie {
        // A `.PMF`'s PSMF header, and there is no such thing here: the field is
        // that container's, not "the movie's header". What this file declares
        // is on the `Movie` itself.
        header: None,
        frame_count: header.frame_count,
        width,
        height,
        frame_rate,
        // Square pixels. Bink declares no aspect of its own, and both logo
        // reels are 1920x1080 - already 16:9 as a pixel grid - where the PSP's
        // and PS2's movies are authored at sizes that are not their own shape.
        display_aspect: (width, height),
        frames,
        no_picture_reason,
        // **Bink's audio is inside the video file**, where ATRAC3+ sits beside
        // it in a `.PMF` and `MovieAudio` is shaped for that: it holds codec
        // blocks and an `at3::Format` and decodes through `crate::at3`. Six of
        // the disc's 37 files have a track - the two logo reels carry four each
        // - and playing one needs a second route through `ffmpeg` and a widened
        // `Movie::audio`, which is its own change. Until then a `.bik` is
        // silent and says so here rather than looking like a file with no
        // track.
        audio: None,
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

/// Converts a Bink file into lossless AV1 under the cache.
///
/// The input is the **whole file**, as [`super::mpeg2_ps::transcode`] hands over
/// a whole program stream: `ffmpeg`'s Bink demuxer reads the container, picks
/// the video stream out of it, and the audio tracks - where there are any - go
/// nowhere, an IVF holding video and nothing else.
fn transcode(blob: &[u8], to: Conversion<'_>, frames: Frames) -> Result<FrameStore> {
    let Conversion {
        key,
        cache_dir,
        width,
        height,
        // Read by `cached` alone, as in every other transcode here: whether a
        // conversion is skipped is that function's whole question.
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

    let source = cache_dir.join(format!("{key}.bik"));
    std::fs::write(&source, blob).with_context(|| format!("writing {}", source.display()))?;

    run_ffmpeg(&source, &out, None, frames, watch)?;

    let store = FrameStore::open(out, width, height)?;
    if store.len == 0 {
        bail!("{} holds no frames", store.path().display());
    }
    Ok(store)
}
