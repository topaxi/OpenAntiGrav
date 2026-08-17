//! Reading the boot movie off whichever source is open, and naming the entry
//! it came from.
//!
//! Split out of `boot.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. What holds it
//! together is that a movie is the one boot asset that is **not** an archive
//! entry on every release: the PS2 keeps its loose in the ISO filesystem, and
//! three of the PSP's have no recovered name at all, so both of the ways around
//! that - [`EntryRef`] and the loose-file fallback - are here.

use super::*;

/// How an archive entry was asked for.
///
/// A WAD directory stores only the hash of each name, and three of the disc's
/// movies - including the 260-frame reel `Intro Screen->IntroMovie1`'s counters
/// describe - have no name anyone has recovered. Addressing one by hash is the
/// only way to name it at all, so `hash:3d2c85f8` is accepted anywhere an entry
/// name is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryRef {
    /// A real name, which is hashed to find the entry.
    Name(String),
    /// A name hash, for an entry whose name is not known.
    Hash(u32),
}

impl EntryRef {
    /// Reads the `hash:` prefix, and treats anything else as a name.
    #[must_use]
    pub fn parse(spec: &str) -> Self {
        match spec.strip_prefix("hash:") {
            Some(digits) => match u32::from_str_radix(digits.trim_start_matches("0x"), 16) {
                Ok(hash) => Self::Hash(hash),
                // Not a hash after all. Falling through to a name keeps a
                // mistyped digit an honest "not in Data.wad" rather than a
                // silent match on something else.
                Err(_) => Self::Name(spec.to_string()),
            },
            None => Self::Name(spec.to_string()),
        }
    }

    /// The hash this reference resolves to.
    #[must_use]
    pub fn hash(&self) -> u32 {
        match self {
            Self::Name(name) => oag_formats::wad::hash_name(name),
            Self::Hash(hash) => *hash,
        }
    }
}

impl std::fmt::Display for EntryRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Name(name) => write!(f, "{name}"),
            Self::Hash(hash) => write!(f, "hash:{hash:08x}"),
        }
    }
}

/// Reads the boot movie, or reports why there is none.
///
/// **A source with no reel of its own is not an error.** The PS2 release ships no
/// `.PMF` in any archive: its movies are loose in the ISO filesystem instead -
/// `DATA/MOVIES/INTRO512.PSS`, an MPEG-2 program stream, and
/// `DATA/MOVIES/BG512.IPF`, IPU video for the PS2's Image Processing Unit. Both
/// decode, but neither is addressable as an archive entry, so
/// [`LOOSE_MOVIES`] is consulted before giving up. A source with neither still
/// gets the sequence without a picture rather than a failure - the same path
/// `--no-video` already takes - because stopping the whole front end over the
/// one piece of it that is genuinely absent helps nobody.
///
/// **A movie the caller actually named is still an error**, and the
/// discriminator is that rather than the platform: [`DEFAULT_BOOT_MOVIE`] and
/// [`DEVPUB_REEL`] are defaults, so a source that does not have one is answering
/// a default, while `--movie` is a request and a request that cannot be met
/// should say so instead of quietly drawing nothing.
///
/// See `docs/ps2/pulse-disc-layout.md`.
pub(super) fn load_movie(
    archives: &mut oag_assets::Archives,
    movie_name: &str,
    options: &Options,
    report: &mut Vec<String>,
    watch: crate::movie::Watch<'_>,
) -> Result<Option<Movie>> {
    let entry = EntryRef::parse(movie_name);
    let hash = entry.hash();
    let data = &mut archives.data;
    let index = match data.index_of_hash(hash) {
        Some(index) => index,
        // Only the default boot movie has a loose-file fallback worth trying:
        // it is the PS2's own intro, a plain file outside every WAD. See
        // `load_loose_intro`.
        None if loose_candidates(movie_name).is_some() => {
            if let Some(movie) = load_loose_movie(movie_name, options, report, watch)? {
                return Ok(Some(movie));
            }
            report.push(format!(
                "{entry} is not in {}, and the source has no loose copy of it either, \
                 so the sequence plays with no picture",
                data.label()
            ));
            return Ok(None);
        }
        None if movie_name == DEVPUB_REEL => {
            report.push(format!(
                "{entry} is not in {}, so the sequence plays with no picture. The dev/pub \
                 reel has no PS2 equivalent",
                data.label()
            ));
            return Ok(None);
        }
        None => anyhow::bail!("{entry} is not in {}", data.label()),
    };
    let size = data.entry_len(index)?;
    let movie_name = entry.to_string();

    if options.no_video {
        // The header alone is 2048 bytes, so this reads kilobytes rather than
        // megabytes when there is no picture to make.
        let head = data.peek(index, oag_formats::pmf::HEADER_LEN as u64)?;
        let header = oag_formats::pmf::Header::parse(&head)
            .map_err(|e| anyhow::anyhow!("parsing {movie_name}: {e}"))?;
        let video = header.video.context("the movie declares no video stream")?;
        report.push(format!(
            "{movie_name}: {}x{}, {:.2}s, video disabled",
            video.width,
            video.height,
            header.duration_seconds()
        ));
        return Ok(Some(Movie {
            frame_count: header.expected_frame_count() as usize,
            width: u32::from(video.width),
            height: u32::from(video.height),
            frame_rate: movie::FRAME_RATE,
            display_aspect: (u32::from(video.width), u32::from(video.height)),
            header: Some(header),
            frames: None,
            no_picture_reason: Some("--no-video was given".to_string()),
            // `--no-video` costs the movie its sound as well, because this path
            // never reads the movie at all - it peeks the header and stops, and
            // the ATRAC3+ frames are in the program stream behind it. Worth
            // knowing when reaching for the flag to isolate the audio: it
            // removes both.
            audio: None,
        }));
    }

    let blob = data
        .read(index)
        .with_context(|| format!("reading {movie_name} out of {}", data.label()))?;
    let key = format!("{hash:08x}-{size}");
    let movie = movie::open(
        &blob,
        &key,
        &options.cache,
        options.extent,
        options.decode(),
        watch,
    )?;

    if let Some(header) = &movie.header {
        report.push(format!(
            "{movie_name}: {}x{}, {:.2}s, {} frames, PSMF{}",
            movie.width,
            movie.height,
            header.duration_seconds(),
            movie.frame_count,
            String::from_utf8_lossy(&header.version)
        ));
        // What the audio stream *is* gets reported by [`load_movie_sound`],
        // which says what became of it as well. This used to say "ATRAC3+, not
        // decoded" here, and that was true right up until ADR-0019 landed.
    }
    match (&movie.frames, &movie.no_picture_reason) {
        (Some(frames), _) => report.push(format!(
            "  {} frame(s) cached in {}",
            frames.len,
            frames.path().display()
        )),
        (None, Some(reason)) => report.push(format!("  no picture: {reason}")),
        (None, None) => {}
    }
    Ok(Some(movie))
}

// The loose-file table `loose_candidates` searches. Which movie a name resolves
// to on which pressing is a fact about what Pulse shipped, so it lives in the
// title package under ADR-0022 rather than here.
use oag_pulse::movies::LOOSE_MOVIES;

/// The loose files a movie name may be answered by, if any.
///
/// Case-insensitive because the front-end XML and the ISO 9660 directory
/// disagree on it: the XML says `Backdrop.ipf`, the disc says `BG512.IPF`.
pub(super) fn loose_candidates(name: &str) -> Option<&'static [&'static str; 2]> {
    LOOSE_MOVIES
        .iter()
        .find(|(asked, _)| asked.eq_ignore_ascii_case(name))
        .map(|(_, candidates)| candidates)
}

/// Tries a movie that sits loose on the disc's filesystem rather than in a WAD.
///
/// This is every movie the PS2 release has: the intro as an MPEG-2 program
/// stream (`INTRO512.PSS`, 512x512, 25 fps PAL; `INTRO640.PSS`, 640x448,
/// 29.97 fps NTSC) and the menu backdrop as IPU video (`BG512.IPF`,
/// `BG640.IPF`). See [`LOOSE_MOVIES`] for which name resolves to which.
///
/// `Ok(None)` when the source has neither cut - a PSP source, or a PS2 one
/// missing both, which is not expected but is not this function's problem to
/// diagnose.
pub(super) fn load_loose_movie(
    name: &str,
    options: &Options,
    report: &mut Vec<String>,
    watch: crate::movie::Watch<'_>,
) -> Result<Option<Movie>> {
    let Some(candidates) = loose_candidates(name) else {
        return Ok(None);
    };
    let Some((path, blob)) = oag_assets::read_loose_file(&options.source, candidates)? else {
        return Ok(None);
    };

    let key = format!("{}-{}", path.replace(['/', '\\'], "_"), blob.len());
    let movie = movie::open(
        &blob,
        &key,
        &options.cache,
        options.extent,
        options.decode(),
        watch,
    )?;

    // Named from the blob's own magic, the same way `movie::open` dispatches,
    // so the report cannot claim a container the decoder did not take.
    let container = if blob.starts_with(&oag_formats::ipf::MAGIC) {
        "IPU video"
    } else {
        "MPEG-2 program stream"
    };
    report.push(format!(
        "{path}: {}x{}, {}/{} fps, {} frames, {container}",
        movie.width, movie.height, movie.frame_rate.0, movie.frame_rate.1, movie.frame_count
    ));
    match (&movie.frames, &movie.no_picture_reason) {
        (Some(frames), _) => report.push(format!(
            "  {} frame(s) cached in {}",
            frames.len,
            frames.path().display()
        )),
        (None, Some(reason)) => report.push(format!("  no picture: {reason}")),
        (None, None) => {}
    }
    Ok(Some(movie))
}
