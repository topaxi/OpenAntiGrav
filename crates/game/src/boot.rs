//! Loading the boot sequence out of a disc image.
//!
//! Everything the front end needs, in the order the original needs it: the
//! front-end root XML, the language plugins, one language's string table, and
//! the intro movie. Kept apart from `main.rs` so the load can be exercised
//! without a window, and apart from [`crate::frontend`] so the sequence itself
//! stays free of file I/O.

use std::path::Path;

use anyhow::{Context, Result};
use oag_assets::pulse;
use oag_formats::fexml;

use crate::frontend::Frontend;
use crate::language::{Language, StringTable};
use crate::movie::{self, Extent, Movie};
use crate::screen::Screens;

/// A loaded boot sequence and the pieces the renderer needs alongside it.
pub struct Boot {
    /// The sequence itself.
    pub frontend: Frontend,
    /// The intro movie, whether or not it has a picture.
    ///
    /// `None` when the source has no such movie **at all**. That was the PS2's
    /// standing case and is no longer: the PS2 release ships no `.PMF` in any
    /// archive, but both its containers now decode - `DATA/MOVIES/INTRO512.PSS`
    /// (an MPEG-2 program stream) and `DATA/MOVIES/BG512.IPF` (IPU video), both
    /// loose in the filesystem, reached through [`LOOSE_MOVIES`]. `None` still
    /// means no movie rather than a movie with no picture, which is what
    /// [`Movie::no_picture_reason`] means and is a different thing.
    pub movie: Option<Movie>,
    /// The looping backdrop the disc's own `FE Screen` plays behind its menus,
    /// for ours to sit on. See [`load_backdrop`].
    ///
    /// `None` is an ordinary outcome and not an error: a source that does not
    /// carry it, `--no-video`, or a missing `ffmpeg`. The menus draw on black
    /// there, which is what they did before this existed.
    pub backdrop: Option<Movie>,
    /// Every front-end image the screens reference, in one texture.
    pub sprites: crate::sprite::Sheet,
    /// The text atlas: the disc's own font when it decodes, ours when it does
    /// not.
    pub font: crate::font::Atlas,
    /// Every language this source offers, for the menus' own language row.
    ///
    /// The picker inside [`Self::frontend`] has the same list; this is the copy
    /// the menus read, so they do not have to reach into a boot sequence that
    /// has already handed off.
    pub languages: Vec<Language>,
    /// The chosen language's string table, for turning a plugin id into
    /// something a player can read.
    pub strings: StringTable,
    /// Every circuit this source offers to race on. See [`crate::catalogue`].
    pub tracks: Vec<crate::catalogue::Track>,
    /// Lines worth printing once, describing what was found.
    pub report: Vec<String>,
}

impl std::fmt::Debug for Boot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Boot")
            .field("movie", &self.movie)
            .finish_non_exhaustive()
    }
}

/// What to load and how much of the movie to convert.
#[derive(Debug, Clone)]
pub struct Options {
    /// A disc image, or a directory extracted with `oag-unpack`.
    pub source: String,
    /// Which movie leg the sequence boots into.
    pub leg: crate::frontend::Leg,
    /// The language to load the string table for, by the XML's own English
    /// name, when one has been chosen on an earlier run.
    ///
    /// `None` falls back to English and then to whatever the source lists
    /// first, which is what a first run gets. A name this source does not carry
    /// falls back the same way rather than failing.
    pub language: Option<String>,
    /// Which `Data.wad` entry that leg plays, as a name or a name hash.
    ///
    /// Three of the disc's movies have no recovered name, and one of them is
    /// the reel `Intro Screen->IntroMovie1`'s frame counters describe, so a name
    /// is not enough to address every candidate. See [`EntryRef`].
    pub movie: String,
    /// Where converted frames are cached.
    pub cache: std::path::PathBuf,
    /// How much of the movie to convert.
    pub extent: Extent,
    /// Skip conversion entirely.
    pub no_video: bool,
}

/// Loads everything and builds the sequence.
pub fn load(options: &Options) -> Result<Boot> {
    let mut report = Vec::new();
    let mut archives = pulse::Archives::open(&options.source)
        .with_context(|| format!("opening the archives in {}", options.source))?;
    report.push(archives.layout.describe());
    report.push(format!(
        "{}: {} entries",
        archives.data.label(),
        archives.data.directory().entries.len()
    ));
    if let Some(fe) = &archives.fe {
        report.push(format!(
            "{}: {} entries",
            fe.label(),
            fe.directory().entries.len()
        ));
    }

    let font = load_font(&mut archives, &mut report);
    let screens = load_screens(&mut archives, &mut report)?;
    let languages = load_languages(&mut archives, &mut report);
    let offered = languages.clone();
    let strings = load_strings(
        &mut archives,
        &languages,
        options.language.as_deref(),
        &mut report,
    );
    let tracks = load_tracks(&mut archives, &mut report);
    let movie = load_movie(&mut archives, options, &mut report)?;
    let backdrop = load_backdrop(&mut archives, options, &mut report);
    let sprites = load_sprites(&mut archives, &screens, &mut report);

    // The cached frames when there are any, and the demuxed count when there
    // are not: `--no-video` and a missing `ffmpeg` still have to play the
    // sequence out over the movie's real duration rather than the reel's.
    let frames = movie.as_ref().map_or(0, |movie| {
        movie.frames.as_ref().map_or(movie.frame_count, |f| f.len)
    });
    let placements = screens
        .screens
        .iter()
        .flat_map(|s| s.images.iter())
        .filter_map(|image| sprites.get(&image.src).map(|p| (image.src.clone(), p)))
        .collect();
    let frame_rate = movie
        .as_ref()
        .map_or(movie::FRAME_RATE, |movie| movie.frame_rate);
    let video_aspect = movie.as_ref().map_or(
        (
            crate::frontend::SCREEN.0 as u32,
            crate::frontend::SCREEN.1 as u32,
        ),
        |movie| movie.display_aspect,
    );
    let frontend = Frontend::booting(
        options.leg,
        screens,
        strings.clone(),
        languages,
        placements,
        frames,
        frame_rate,
        video_aspect,
        movie.as_ref().is_some_and(|movie| movie.frames.is_some()),
    );

    if let Some(goto) = frontend.language_auto_redirect() {
        report.push(format!(
            "the disc's Language Selection redirects to {goto}; this build goes to Launch Game, \
             which starts a race"
        ));
    }

    Ok(Boot {
        languages: offered,
        strings,
        tracks,
        font,
        frontend,
        movie,
        backdrop,
        sprites,
        report,
    })
}

/// Loads the looping backdrop the disc plays behind its menus.
///
/// **The disc's own arrangement, not decoration we invented.** `FE Screen` -
/// the original's main menu - carries a `Movie` widget naming
/// `Data\Movies\Backdrop.PMF`, and a cold boot under PPSSPP with
/// `MoviePlayer_Open` armed from reset opens exactly two movies in ten minutes:
/// the intro, and this. See [`DEFAULT_BOOT_MOVIE`] and
/// `docs/ghidra/functions/psp-pulse-usa/frontend-video.md`. Our menu tree is ours;
/// what it sits on is the disc's.
///
/// Three differences from [`load_movie`], each of which is why this is its own
/// function rather than a second call:
///
/// - **Absence is never an error.** `load_movie` bails on a movie the command
///   line named and the source does not have, because a run that cannot play
///   the movie it was asked for has failed. Nothing asked for this one, so
///   every way of not getting it - no such entry, no `ffmpeg`, `--no-video` -
///   ends as `None` and a note, and the menus draw on black.
/// - **Always the whole movie**, whatever `--movie-frames` says. That flag
///   exists so a first run need not transcode 1200 intro frames; capping a
///   *loop* at four would make the backdrop stutter round every seventh of a
///   second, which is not a shorter version of the same thing.
/// - **It is loaded here, at boot, rather than when the menus first open.** The
///   270 frames cost about thirteen seconds to transcode once and nothing on
///   every run after, and boot is already paying that for the intro's 1200 -
///   whereas the menus open on a keypress out of a race, where a
///   thirteen-second freeze would read as a hang.
fn load_backdrop(
    archives: &mut oag_assets::pulse::Archives,
    options: &Options,
    report: &mut Vec<String>,
) -> Option<Movie> {
    if options.no_video {
        return None;
    }
    let wanted = Options {
        movie: pulse::names::BACKDROP_MOVIE.to_string(),
        extent: Extent::Whole,
        ..options.clone()
    };
    match load_movie(archives, &wanted, report) {
        Ok(movie) => movie,
        // Reported and dropped. `load_movie` only errors here on an entry it
        // cannot read, and the backdrop is not worth failing a boot over.
        Err(e) => {
            report.push(format!("no menu backdrop: {e:#}"));
            None
        }
    }
}

/// Decodes every image the screens name.
///
/// The names come from the screens rather than from a list here, so a screen
/// that gains an `Image` gains its texture without this function changing.
///
/// **Every archive the source has is searched, `FE.wad` first, and both parts of
/// that matter.** `pulse_logo.mip` is in `FE.wad` *and* `Data.wad` at the same
/// size, which makes `FE.wad` look sufficient; `gameshare_backdrop.mip` is in
/// `Data.wad` only, which proves it is not. The order is deliberate and is
/// therefore written here rather than taken from
/// [`oag_assets::pulse::Archives::read_name`], which searches the *bulk* archive
/// first because that is the right default for a race: same size is not same
/// bytes, and a front-end image should come off the front end's own archive.
fn load_sprites(
    archives: &mut oag_assets::pulse::Archives,
    screens: &Screens,
    report: &mut Vec<String>,
) -> crate::sprite::Sheet {
    let mut srcs: Vec<String> = Vec::new();
    for screen in &screens.screens {
        for image in &screen.images {
            if !srcs.contains(&image.src) {
                srcs.push(image.src.clone());
            }
        }
    }

    if srcs.is_empty() {
        return crate::sprite::Sheet::default();
    }

    let mut blobs: Vec<(String, Vec<u8>)> = Vec::new();
    for src in &srcs {
        match read_front_end_first(archives, src) {
            Ok(blob) => blobs.push((src.clone(), blob)),
            Err(e) => report.push(format!("image {src}: {e}")),
        }
    }

    let sheet = crate::sprite::Sheet::build(&blobs, report);
    report.push(format!(
        "{} of {} front-end image(s) decoded into a {}x{} sheet",
        sheet.len(),
        srcs.len(),
        sheet.width,
        sheet.height
    ));
    sheet
}

/// Reads a front-end asset, preferring the companion archive over the bulk one.
///
/// The mirror image of [`oag_assets::pulse::Archives::read_name`]'s order, for the
/// callers that want a front-end asset specifically. See [`load_sprites`] for the
/// two entries that decide it.
fn read_front_end_first(
    archives: &mut oag_assets::pulse::Archives,
    name: &str,
) -> oag_assets::Result<Vec<u8>> {
    if let Some(fe) = archives.fe.as_mut()
        && let Ok(blob) = fe.read_name(name)
    {
        return Ok(blob);
    }
    archives.data.read_name(name)
}

/// Reads the front end's own font, falling back to the built-in glyphs.
///
/// A missing or undecodable font is not fatal: the menu still draws, in the 5x7
/// approximation, and the report says which one is on screen. That matters more
/// than it sounds - the two look very different, and a silent fallback would
/// make a rendering bug indistinguishable from a loading one.
fn load_font(
    archives: &mut oag_assets::pulse::Archives,
    report: &mut Vec<String>,
) -> crate::font::Atlas {
    let name = oag_assets::pulse::names::DEFAULT_FONT;
    match read_front_end_first(archives, name)
        .map_err(|e| e.to_string())
        .and_then(|blob| oag_formats::fnt::Font::parse(&blob).map_err(|e| e.to_string()))
    {
        Ok(font) => {
            let atlas = crate::font::Atlas::from_font(&font);
            report.push(format!(
                "font {name}: {}x{} atlas, {} glyphs, line height {}",
                font.width,
                font.height,
                font.glyphs.len(),
                font.line_height
            ));
            atlas
        }
        Err(why) => {
            report.push(format!("font {name} unavailable ({why}); drawing with 5x7"));
            crate::font::Atlas::build()
        }
    }
}

fn load_screens(
    archives: &mut oag_assets::pulse::Archives,
    report: &mut Vec<String>,
) -> Result<Screens> {
    // The one piece with no degraded form: a front end with no screens is not a
    // front end. The message names the archives searched, because on a source
    // whose front-end root has never been located that is the useful half.
    let blob = archives
        .read_name(pulse::names::FRONTEND_ROOT)
        .with_context(|| {
            format!(
                "reading {} out of {}",
                pulse::names::FRONTEND_ROOT,
                archives.layout.describe()
            )
        })?;

    // Front-end XML is stored with its element and attribute names shortened
    // through a per-file dictionary. Files that begin `<?xml` are already plain.
    let xml = if fexml::is_fexml(&blob) {
        fexml::expand(&blob).map_err(|e| anyhow::anyhow!("expanding the front-end XML: {e}"))?
    } else {
        String::from_utf8(blob).context("the front-end XML is not text")?
    };

    let screens = Screens::from_xml(&xml);
    report.push(format!(
        "{}: {} screens, {} globals, {} LoadXML includes",
        pulse::names::FRONTEND_ROOT,
        screens.screens.len(),
        screens.globals.len(),
        screens.load_xml.len()
    ));
    for screen in screens.with_movies() {
        if let Some(movie) = &screen.movie {
            report.push(format!(
                "  screen {:?} plays {}",
                screen.name,
                movie.entry_name()
            ));
        }
    }
    Ok(screens)
}

/// Every language plugin this source carries.
///
/// Public because a race needs the string table too, for the HUD's `idstring`
/// captions, and it does not go through the boot path that used to be the only
/// caller. See [`load_strings`].
pub fn load_languages(
    archives: &mut oag_assets::pulse::Archives,
    report: &mut Vec<String>,
) -> Vec<Language> {
    let mut out = Vec::new();
    for plugin in pulse::LANGUAGE_PLUGINS {
        let name = pulse::names::language_definition(plugin);
        let Ok(blob) = archives.read_name(&name) else {
            continue;
        };
        let Ok(xml) = expand(&blob) else { continue };
        if let Some(language) = Language::from_definition(plugin, &xml) {
            out.push(language);
        }
    }

    if out.is_empty() {
        report.push("no language plugins resolved; the picker will be empty".to_string());
    } else {
        report.push(format!(
            "{} language(s): {}",
            out.len(),
            out.iter()
                .map(|l| format!("{} ({}, {})", l.name, l.native_name, l.plugin))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out
}

/// Reads the raceable circuits out of the game plugin's own definition.
///
/// An empty list is reported and not fatal: a source whose plugin will not read
/// still boots, still races the default track, and the menus' circuit row draws
/// as one with nothing to offer rather than the game refusing to start.
fn load_tracks(
    archives: &mut pulse::Archives,
    report: &mut Vec<String>,
) -> Vec<crate::catalogue::Track> {
    let name = pulse::names::GAME_PLUGIN_DEFINITION;
    let tracks = match archives
        .read_name(name)
        .and_then(|blob| expand(&blob).map_err(|e| oag_assets::Error::BadSpec(e.to_string())))
    {
        Ok(xml) => crate::catalogue::tracks(&xml),
        Err(e) => {
            report.push(format!("{name}: {e}"));
            Vec::new()
        }
    };
    report.push(format!("{name}: {} raceable circuit(s)", tracks.len()));
    tracks
}

/// The chosen language's string table.
///
/// Public for the same reason [`load_languages`] is: the HUD resolves `IG_HUD_*`
/// keys through this, and a race reaches it without booting the front end.
pub fn load_strings(
    archives: &mut oag_assets::pulse::Archives,
    languages: &[Language],
    preferred: Option<&str>,
    report: &mut Vec<String>,
) -> StringTable {
    // The saved language first, then English, then whatever comes first. The
    // fallback chain used to end at English with a note that there was nothing
    // saved to prefer; there is now. A saved name this source does not carry
    // falls through rather than failing - the same rule the picker's own
    // preselection follows, and for the same reason: a settings file written
    // against the EU disc must not stop the USA one booting.
    let chosen = preferred
        .and_then(|name| languages.iter().find(|l| l.name.eq_ignore_ascii_case(name)))
        .or_else(|| languages.iter().find(|l| l.name == "English"))
        .or_else(|| languages.first());

    let Some(language) = chosen else {
        return StringTable::default();
    };
    let Some(entries) = language.entries.as_deref() else {
        report.push(format!("{} names no string table", language.name));
        return StringTable::default();
    };

    match archives
        .read_name(entries)
        .and_then(|blob| expand(&blob).map_err(|e| oag_assets::Error::BadSpec(e.to_string())))
    {
        Ok(xml) => {
            let table = StringTable::from_xml(&xml);
            report.push(format!(
                "{entries}: {} strings for {}",
                table.len(),
                language.name
            ));
            table
        }
        Err(e) => {
            report.push(format!("{entries}: {e}"));
            StringTable::default()
        }
    }
}

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
pub const DEVPUB_REEL: &str = "hash:b1ba72c3";

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
fn load_movie(
    archives: &mut oag_assets::pulse::Archives,
    options: &Options,
    report: &mut Vec<String>,
) -> Result<Option<Movie>> {
    let entry = EntryRef::parse(&options.movie);
    let hash = entry.hash();
    let data = &mut archives.data;
    let index = match data.index_of_hash(hash) {
        Some(index) => index,
        // Only the default boot movie has a loose-file fallback worth trying:
        // it is the PS2's own intro, a plain file outside every WAD. See
        // `load_loose_intro`.
        None if loose_candidates(&options.movie).is_some() => {
            if let Some(movie) = load_loose_movie(&options.movie, options, report)? {
                return Ok(Some(movie));
            }
            report.push(format!(
                "{entry} is not in {}, and the source has no loose copy of it either, \
                 so the sequence plays with no picture",
                data.label()
            ));
            return Ok(None);
        }
        None if options.movie == DEVPUB_REEL => {
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
    let options = &Options {
        movie: entry.to_string(),
        ..options.clone()
    };

    if options.no_video {
        // The header alone is 2048 bytes, so this reads kilobytes rather than
        // megabytes when there is no picture to make.
        let head = data.peek(index, oag_formats::pmf::HEADER_LEN as u64)?;
        let header = oag_formats::pmf::Header::parse(&head)
            .map_err(|e| anyhow::anyhow!("parsing {}: {e}", options.movie))?;
        let video = header.video.context("the movie declares no video stream")?;
        report.push(format!(
            "{}: {}x{}, {:.2}s, video disabled",
            options.movie,
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
        }));
    }

    let blob = data
        .read(index)
        .with_context(|| format!("reading {} out of {}", options.movie, data.label()))?;
    let key = format!("{hash:08x}-{size}");
    let movie = movie::open(&blob, &key, &options.cache, options.extent, false)?;

    if let Some(header) = &movie.header {
        report.push(format!(
            "{}: {}x{}, {:.2}s, {} frames, PSMF{}",
            options.movie,
            movie.width,
            movie.height,
            header.duration_seconds(),
            movie.frame_count,
            String::from_utf8_lossy(&header.version)
        ));
        if let Some(audio) = header.audio {
            report.push(format!(
                "  audio: {} channel(s) at {}, ATRAC3+, not decoded",
                audio.channels,
                audio.frequency_hz().map_or_else(
                    || format!("code {}", audio.frequency_code),
                    |hz| format!("{hz} Hz")
                )
            ));
        }
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

/// The PS2's movies, which are loose on the disc's own filesystem rather than
/// in any WAD, keyed by the name that asks for them.
///
/// **The two-cut split and this table are the original's, read out of
/// `SCES_547.48`.** `FUN_0019b168` tests the `Movie` widget's `src` for
/// `\Intro.pss` or `\Backdrop.ipf` and rewrites it to
/// `Data\Movies\Intro512.pss` / `Intro640.pss` or `Data\Movies\bg512.ipf` /
/// `bg640.ipf`, choosing on one global (`0x0027a85c`) whose sole writer sets
/// PAL pixel-aspect constants for the `512` value and NTSC ones for `640`. So
/// the rewrite, the pairing and the region split are all the disc's, and the
/// XML's names are never filenames on PS2.
///
/// `Data\Movies\Intro.PMF` is here because it is [`DEFAULT_BOOT_MOVIE`]: the
/// PSP's name for the same reel, which a PS2 source answers with its own cut.
/// `Data\Movies\Backdrop.PMF` is here for exactly the same reason, and so that
/// [`load_backdrop`] can ask for one name on both platforms.
///
/// 512 is listed first because this disc is EU/PAL; the ultimate trigger that
/// decides which value `0x0027a85c` takes is not traced. See
/// `docs/ps2/pulse-disc-layout.md` and `docs/formats/ipf.md`.
const LOOSE_MOVIES: [(&str, [&str; 2]); 4] = [
    (
        DEFAULT_BOOT_MOVIE,
        ["DATA/MOVIES/INTRO512.PSS", "DATA/MOVIES/INTRO640.PSS"],
    ),
    (
        r"Data\Movies\Intro.pss",
        ["DATA/MOVIES/INTRO512.PSS", "DATA/MOVIES/INTRO640.PSS"],
    ),
    (
        r"Data\Movies\Backdrop.ipf",
        ["DATA/MOVIES/BG512.IPF", "DATA/MOVIES/BG640.IPF"],
    ),
    (
        pulse::names::BACKDROP_MOVIE,
        ["DATA/MOVIES/BG512.IPF", "DATA/MOVIES/BG640.IPF"],
    ),
];

/// The loose files a movie name may be answered by, if any.
///
/// Case-insensitive because the front-end XML and the ISO 9660 directory
/// disagree on it: the XML says `Backdrop.ipf`, the disc says `BG512.IPF`.
fn loose_candidates(name: &str) -> Option<&'static [&'static str; 2]> {
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
fn load_loose_movie(
    name: &str,
    options: &Options,
    report: &mut Vec<String>,
) -> Result<Option<Movie>> {
    let Some(candidates) = loose_candidates(name) else {
        return Ok(None);
    };
    let Some((path, blob)) = pulse::read_loose_file(&options.source, candidates)? else {
        return Ok(None);
    };

    let key = format!("{}-{}", path.replace(['/', '\\'], "_"), blob.len());
    let movie = movie::open(
        &blob,
        &key,
        &options.cache,
        options.extent,
        options.no_video,
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

fn expand(blob: &[u8]) -> Result<String> {
    if fexml::is_fexml(blob) {
        fexml::expand(blob).map_err(|e| anyhow::anyhow!("{e}"))
    } else {
        String::from_utf8(blob.to_vec()).context("not text")
    }
}

/// The default cache directory: `data/cache/movies` in a repository checkout,
/// and `<cache dir>/oag/movies` anywhere else.
///
/// A checkout is recognised by having a `data/` directory, which is where
/// everything user-supplied already lives and what `just` recipes and
/// `data/README.md` document. A packaged build has no checkout around it, and
/// writing beside wherever it happens to have been run from would scatter a
/// cache through a player's folders - or fail outright, if that is a read-only
/// mount. Deleting either directory is always safe; see
/// `docs/architecture/adr/0004-asset-pipeline.md`.
#[must_use]
pub fn default_cache_dir() -> std::path::PathBuf {
    let checkout = Path::new("data/cache/movies");
    if Path::new("data").is_dir() {
        return checkout.to_path_buf();
    }
    dirs::cache_dir().map_or_else(
        || checkout.to_path_buf(),
        |cache| cache.join("oag").join("movies"),
    )
}

#[cfg(test)]
mod tests {
    use super::{EntryRef, pulse};

    #[test]
    fn a_name_resolves_through_the_wad_hash() {
        let entry = EntryRef::parse(pulse::names::INTRO_MOVIE);
        assert_eq!(entry, EntryRef::Name(pulse::names::INTRO_MOVIE.to_string()));
        assert_eq!(entry.hash(), 0x71d3_c1ec);
    }

    #[test]
    fn the_default_boot_movie_is_the_one_the_disc_opens() {
        // The regression guard for the reported bug: booting into the dev/pub
        // reel showed Pure-era logo cards, because those three reels ship
        // byte-identically on Wipeout Pure's disc. What LogoFMV names is this.
        assert_eq!(super::DEFAULT_BOOT_MOVIE, r"Data\Movies\Intro.PMF");
        assert_eq!(
            EntryRef::parse(super::DEFAULT_BOOT_MOVIE).hash(),
            0x71d3_c1ec
        );
    }

    #[test]
    fn the_reel_default_is_the_european_cut() {
        assert_eq!(
            EntryRef::parse(super::DEVPUB_REEL).hash(),
            pulse::hashes::DEVPUB_REEL_SCEE
        );
    }

    #[test]
    fn a_hash_addresses_an_entry_with_no_recovered_name() {
        // The SCEA dev/pub reel, which has no name to ask for.
        let entry = EntryRef::parse("hash:3d2c85f8");
        assert_eq!(entry, EntryRef::Hash(0x3d2c_85f8));
        assert_eq!(entry.hash(), 0x3d2c_85f8);
        assert_eq!(entry.to_string(), "hash:3d2c85f8");
        assert_eq!(EntryRef::parse("hash:0x3d2c85f8").hash(), 0x3d2c_85f8);
    }

    #[test]
    fn a_mistyped_hash_stays_a_name_rather_than_matching_something_else() {
        let entry = EntryRef::parse("hash:zzz");
        assert_eq!(entry, EntryRef::Name("hash:zzz".to_string()));
    }

    #[test]
    fn a_name_that_looks_like_a_hash_is_still_a_name() {
        // No prefix, so no hash. Names are never bare hex on these discs, but
        // the rule has to be the prefix rather than the shape.
        assert_eq!(
            EntryRef::parse("3d2c85f8"),
            EntryRef::Name("3d2c85f8".to_string())
        );
    }
}
