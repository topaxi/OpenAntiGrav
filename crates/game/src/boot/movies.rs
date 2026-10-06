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

/// What `--movie` defaults to: the movie the disc's own boot plays.
///
/// `Data\Movies\Intro.PMF` is the 1200-frame, 40-second Pulse showcase, and it
/// is what the `LogoFMV` screen's `Movie` widget names - `src="Data\Movies\Intro"`,
/// `autostart`, `repeat="false"`, `autoredirect` - in the front-end XML on the
/// disc. A cold boot under PPSSPP with `MoviePlayer_Open` armed from reset opens
/// exactly two movies in ten minutes, this one and `Data\Movies\Backdrop.PMF`,
/// the latter being the looping backdrop of the `FE Screen` that comes after.
///
/// See `docs/architecture/frontend-boot.md` and
/// `docs/ghidra/functions/psp-pulse-usa/frontend-video.md`.
pub const DEFAULT_BOOT_MOVIE: &str = pulse::names::INTRO_MOVIE;

/// What `--reel` defaults to: the European cut of the dev/pub reel.
///
/// Spelled as a hash because the reel has no recovered name. It is the reel
/// whose contents fit `Intro Screen->IntroMovie1`'s constants - 260 frames,
/// static at 144 and 231, which is exactly where that state pauses for two
/// seconds, and those frames read `SONY COMPUTER ENTERTAINMENT EUROPE PRESENTS`
/// and `A STUDIO LIVERPOOL GAME`.
///
/// **It is not a boot movie.** The disc never opens it during boot, and it
/// carries no Pulse branding: the same three cuts ship byte-identically on
/// *Wipeout Pure*'s USA disc, which is why booting into it looked like the wrong
/// game. Where the reels *are* played is an open question.
///
/// European rather than American despite the disc's `UCUS-98712` serial: the
/// executable on this image is the EU build throughout - 18 `UCES00465` strings
/// and no `UCUS` string at all - and the ISO's volume id and publisher are both
/// `SCEE`. Confidence 75; the selection itself has not been read out of the
/// binary. `--movie hash:3d2c85f8` is the American cut.
///
/// See `docs/architecture/frontend-boot.md`.
pub const DEVPUB_REEL: &str = pulse::names::DEVPUB_REEL;

/// The region `oag_ui::screen::Movie::entry_name` resolves a `localised`
/// widget with - `oag_ui::screen::DEFAULT_REGION` off every title with no
/// [`oag_title::Title::pressings`], that table's row for the serial on one
/// that has them (Pure). See
/// `docs/ghidra/functions/psp-pure-eu/movie-localised-suffix.md`.
pub(super) fn resolve_movie_region(title: &oag_title::Title, serial: Option<&str>) -> &'static str {
    title
        .pressings
        .map_or(oag_ui::screen::DEFAULT_REGION, |pressings| {
            pressings.of(serial).movie_region
        })
}

/// Substitutes a title's pressing-correct cut for its two declared boot
/// movies (Pure's), leaving a title with no
/// [`oag_title::Title::pressings`] untouched. See [`resolve_movie_region`]'s
/// own doc for why one table cannot hold this.
///
/// Keyed on the region [`resolve_movie_region`] resolved, so a region no row
/// carries takes the unlisted (EU) row, as `oag_pure::names::intro_movie` did.
pub(super) fn resolve_pure_movie_region(
    title: &oag_title::Title,
    movie_region: &str,
    first: Option<&'static str>,
    second: Option<&'static str>,
) -> (Option<&'static str>, Option<&'static str>) {
    let Some(pressings) = title.pressings else {
        return (first, second);
    };
    let pressing = pressings
        .listed
        .iter()
        .find(|row| row.movie_region == movie_region)
        .unwrap_or(&pressings.unlisted);
    (
        first.map(|_| pressing.intro_movie),
        second.map(|_| pressing.fmv_intro_movie),
    )
}

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
/// `DATA/MOVIES/INTRO640.PSS`, an MPEG-2 program stream, and
/// `DATA/MOVIES/BG640.IPF`, IPU video for the PS2's Image Processing Unit. Both
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
    // **A PSARC stores real paths and no name hash**, so every WAD-shaped step
    // below - `index_of_hash`, `entry_len`, `peek` - is unavailable on a Wipeout
    // HD source, and `EntryRef` is meaningless there: `hash:` addressing exists
    // because a WAD directory holds only hashes, which is the opposite of this
    // container's problem. `read_name` is the route both understand, so the
    // branch is on the container rather than on the console.
    if !archives.data.is_wad() {
        return load_movie_by_path(archives, movie_name, options, report, watch);
    }
    let entry = EntryRef::parse(movie_name);
    let hash = entry.hash();
    // Every path below is WAD-shaped - by hash, by index, peeking a header -
    // and every source that reaches here is a PSP or PS2 one. See
    // `oag_assets::Error::NotAWad`.
    let data = archives.data.as_wad_mut("a movie by name hash")?;
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
        let head = data.peek(index, oag_video::pmf::HEADER_LEN as u64)?;
        let header = oag_video::pmf::Header::parse(&head)
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
            frame_rate: oag_ui::frontend::FRAME_RATE,
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

/// Reads the boot movie out of a path-addressed archive - Wipeout HD's PSARCs.
///
/// Shorter than [`load_movie`] because three of that function's four
/// complications do not exist here. There is no loose-file fallback (HD keeps
/// every `.bik` inside an archive), no `hash:` spelling to accept, and no
/// `peek`-the-header shortcut for `--no-video`: [`oag_video::bik`] reads a
/// header out of a blob rather than off a container, and a PSARC entry is
/// compressed in blocks, so there is no cheap prefix to peek at anyway. The
/// whole entry is read and `movie::open` is asked for a picture-less movie,
/// which costs one read and no transcode.
///
/// **A movie the caller named and this source does not have is still an
/// error**, exactly as in [`load_movie`]: `--movie` is a request.
fn load_movie_by_path(
    archives: &mut oag_assets::Archives,
    movie_name: &str,
    options: &Options,
    report: &mut Vec<String>,
    watch: crate::movie::Watch<'_>,
) -> Result<Option<Movie>> {
    let blob = archives
        .read_name(movie_name)
        .with_context(|| format!("reading {movie_name} out of {}", archives.layout.describe()))?;

    // **The entry's own length, not a directory field.** A WAD key is
    // `{hash:08x}-{size}` off the directory; here the bytes are already in hand
    // and their length is the same thing measured one step later. The name is
    // folded into the key as well, because unlike a hash it is not already
    // unique across archives - and it is sanitised because this key becomes a
    // filename in the movie cache.
    let key = format!("{}-{}", cache_key_for(movie_name), blob.len());
    let movie = movie::open(
        &blob,
        &key,
        &options.cache,
        options.extent,
        options.decode(),
        watch,
    )?;

    report.push(format!(
        "{movie_name}: {}x{}, {:.2}s, {} frames",
        movie.width,
        movie.height,
        movie.frame_count as f64 * movie.frame_rate.1 as f64 / movie.frame_rate.0 as f64,
        movie.frame_count,
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

/// Turns an archive path into something that can be part of a filename.
///
/// The movie cache writes `{key}-{w}x{h}-...` files, and HD's movie names are
/// paths - `Data/FE/Images/StudioLiverpool.bik` - so an unsanitised key would
/// ask for a file three directories deep that nothing created. Every character
/// that is not alphanumeric becomes `_`, which is lossy and does not matter: the
/// length is in the key beside it, and two different entries of the same length
/// whose paths differ only in punctuation is not a collision this disc can
/// produce.
fn cache_key_for(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
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
    let container = if blob.starts_with(&oag_video::ipf::MAGIC) {
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
