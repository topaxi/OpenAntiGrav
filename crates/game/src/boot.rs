//! Loading the boot sequence out of a disc image.
//!
//! Everything the front end needs, in the order the original needs it: the
//! front-end root XML, the language plugins, one language's string table, and
//! the intro movie. Kept apart from `main.rs` so the load can be exercised
//! without a window, and apart from [`crate::frontend`] so the sequence itself
//! stays free of file I/O.

use std::path::Path;

use anyhow::{Context, Result};
use oag_formats::fexml;
use oag_pulse as pulse;
use oag_pure::frontend::states as pure_states;

use crate::title::open_source;

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
    /// Pure's second boot movie, played by the `FMV Intro` screen state - see
    /// [`load_fmv_intro`]. `None` for any source that has no such state
    /// (every Pulse source), the same way `backdrop` is `None` for a source
    /// with no menu backdrop.
    pub fmv_intro: Option<Movie>,
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
    /// Every circuit this source offers to race on, plus every one a mounted
    /// pack adds. See [`crate::catalogue`].
    pub tracks: Vec<crate::catalogue::Track>,
    /// Every team this source offers, plus every one a mounted pack adds, in
    /// the order the definitions declare them.
    pub teams: Vec<crate::catalogue::Team>,
    /// The intro movie's own sound, decoded and ready to play.
    ///
    /// `None` whenever the movie should be silent, and every reason funnels
    /// into that one word rather than being re-decided downstream: a movie with
    /// no audio stream, a `Movie` widget carrying `sound="false"`, an
    /// `ffmpeg` that is missing or failed, or `--no-video`, which never reads
    /// the movie at all. See [`load_movie_sound`].
    pub movie_sound: Option<crate::at3::Pcm>,
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
    /// Directories to look in for [downloadable content](crate::dlc), mounted
    /// behind `source`'s own archives and independent of which release it is.
    pub dlc: Vec<std::path::PathBuf>,
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
    ///
    /// `None` is not "no movie" - it means "this leg's own default", resolved
    /// in [`load`] once the source's title is known (`screens` is parsed by
    /// then). Before this existed the default was baked into `Options` at
    /// construction, in `main.rs`, which is exactly where the title is *not*
    /// yet known - that was fine while every source was Pulse and wrong the
    /// moment Pure could open at all, since Pulse's default names an entry
    /// Pure does not have. `Some(name)` is a request, same as always, and is
    /// used verbatim regardless of title.
    pub movie: Option<String>,
    /// Where converted frames are cached.
    pub cache: std::path::PathBuf,
    /// Where decoded PCM is cached - see [`default_audio_cache_dir`].
    ///
    /// A second directory rather than the one above, and deliberately: the two
    /// hold different things with different lifetimes, and a player clearing
    /// one should not lose the other.
    pub audio_cache: std::path::PathBuf,
    /// How much of the movie to convert.
    pub extent: Extent,
    /// Skip conversion entirely.
    pub no_video: bool,
}

/// Loads everything and builds the sequence.
pub fn load(options: &Options) -> Result<Boot> {
    let mut report = Vec::new();
    let (packs, problems) = crate::dlc::packs(&options.dlc, &default_dlc_cache_dir());
    let mut archives = open_source(&options.source, packs)
        .with_context(|| format!("opening the archives in {}", options.source))?;
    report.push(archives.layout.describe());
    if !archives.packs.is_empty() {
        report.push(format!(
            "dlc: {} archive(s) mounted behind this source",
            archives.packs.len()
        ));
    }
    report.extend(problems.into_iter().map(|p| format!("dlc: {p}")));
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
    let documents = definitions(&mut archives, &mut report);
    let tracks = load_tracks(&mut archives, &documents, &mut report);
    let teams = load_teams(&mut archives, &documents, &mut report);
    let movie_name = options
        .movie
        .clone()
        .unwrap_or_else(|| default_boot_movie(options.leg, &screens));
    let movie = load_movie(&mut archives, &movie_name, options, &mut report)?;
    // Straight after the movie, so its report lines stay together, and while
    // `screens` is still in hand: the widget that decides whether this movie is
    // heard at all is in that XML.
    let movie_sound = load_movie_sound(movie.as_ref(), &screens, &movie_name, options, &mut report);
    let backdrop = load_backdrop(&mut archives, options, &mut report);
    let fmv_intro = load_fmv_intro(&mut archives, &screens, options, &mut report);
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
    // The grid this source authors in, needed before the front end is built so
    // that a boot with no movie falls back to the *source's* shape rather than
    // to the PSP's. `frontend.set_space` below takes the same value.
    let space = crate::frontend::Space::of(archives.layout.platform);
    let video_aspect = movie
        .as_ref()
        .map_or((space.size.0 as u32, space.size.1 as u32), |movie| {
            movie.display_aspect
        });
    // Same shape as `frames`/`frame_rate` above, for the second boot movie.
    // Both `None` on any source `load_fmv_intro` never attempted.
    let fmv_intro_frames = fmv_intro.as_ref().map_or(0, |movie| {
        movie.frames.as_ref().map_or(movie.frame_count, |f| f.len)
    });
    let fmv_intro_frame_rate = fmv_intro
        .as_ref()
        .map_or(movie::FRAME_RATE, |movie| movie.frame_rate);
    let mut frontend = Frontend::booting(
        options.leg,
        screens,
        strings.clone(),
        languages,
        placements,
        frames,
        frame_rate,
        video_aspect,
        movie.as_ref().is_some_and(|movie| movie.frames.is_some()),
        fmv_intro_frames,
        fmv_intro_frame_rate,
        fmv_intro
            .as_ref()
            .is_some_and(|movie| movie.frames.is_some()),
    );

    // **Before `set_backdrop`, which bakes a rect out of it.** The PS2's
    // `Skin.xml` places widgets in a 640x448 grid rather than the PSP's
    // 480x272, so every widget on that disc landed off the bottom-right of a
    // screen a third too small - which is why its `Show Logo` drew nothing at
    // all. Set from the archives' own platform rather than sniffed: the layout
    // resolved one to open them. See `crate::frontend::Space`.
    frontend.set_space(space);
    report.push(format!(
        "front-end grid {}x{}, shown as {:.3}",
        space.size.0, space.size.1, space.display_aspect
    ));

    // `Show Logo` sits on the moving backdrop, because it is a child of the
    // `FE Screen` that owns that movie. Set here rather than at either call site
    // because this is the one place the frontend and the backdrop are both in
    // hand, and because the window, the offscreen capture and the ground-truth
    // tests all have to agree about it - `main.rs` takes the backdrop's frames
    // away onto a decode thread immediately after this, so there is no second
    // moment where both halves exist.
    //
    // **Only when the two movies have the same plane geometry**, which is a
    // guard rather than a switch: the front end is drawn by one renderer with
    // one set of I420 planes, sized once from the intro, and `upload_frame`
    // slices a frame by *those* dimensions rather than by the frame's own - so
    // handing it a differently-shaped picture is a garbled image rather than an
    // error.
    //
    // **Both current sources pass it**, and that was worth checking rather than
    // assuming: the PSP's `Intro.PMF` and `Backdrop.PMF` are both 480x272, and
    // the PS2's `INTRO512.PSS` and `BG512.IPF` are both 512x512. What differs on
    // the PS2 is the *display aspect*, not the plane size, and that is already
    // handled per-movie by the pillarbox rect rather than here. So this branch
    // does not currently fire on any disc anyone has - it is what stops a future
    // source, or a `--movie` override pointing the intro at something else, from
    // drawing garbage instead of saying so.
    // See `docs/architecture/frontend-boot.md`.
    if let Some(backdrop) = &backdrop {
        let same_planes = crate::render::VideoFormat::of(backdrop)
            .zip(movie.as_ref().and_then(crate::render::VideoFormat::of))
            .is_some_and(|(back, intro)| {
                (
                    back.width,
                    back.height,
                    back.chroma_width,
                    back.chroma_height,
                ) == (
                    intro.width,
                    intro.height,
                    intro.chroma_width,
                    intro.chroma_height,
                )
            });
        if same_planes {
            // **The cache's length, not the container's frame count**, when
            // there is a cache. The playhead set here is the one the menus go
            // on running after the boot sequence hands it over, and it is
            // compared against a `movie::Feed` built from these same decoded
            // frames - so it has to wrap where the feed wraps. The two agree on
            // every source measured so far; taking the count off the container
            // would only be right by luck.
            let frames = backdrop
                .frames
                .as_ref()
                .map_or(backdrop.frame_count, |frames| frames.len);
            frontend.set_backdrop(frames, backdrop.frame_rate, backdrop.display_aspect);
        } else {
            report.push(
                "Show Logo draws on black: the backdrop's planes are not the intro's, \
                 and the front end has one video pipeline"
                    .to_string(),
            );
        }
    }

    if let Some(goto) = frontend.language_auto_redirect() {
        report.push(format!(
            "the disc's Language Selection redirects to {goto}; this build goes to {}, \
             having played it first",
            crate::frontend::states::SHOW_LOGO
        ));
    }

    Ok(Boot {
        languages: offered,
        strings,
        tracks,
        teams,
        font,
        frontend,
        movie,
        movie_sound,
        backdrop,
        fmv_intro,
        sprites,
        report,
    })
}

/// Decodes the intro movie's ATRAC3+ track, if it should be heard at all.
///
/// # The widget decides, not the container
///
/// A `.PMF` having an audio stream is not the same question as whether the
/// front end plays it. `Data\Movies\Backdrop` is `sound="false"` **and** has no
/// track, so it is silent twice over; a movie that had a track and a widget
/// saying `sound="false"` would still have to be silent, and this is what makes
/// that true rather than the container's own contents. `Data\Movies\Intro` is
/// `sound="true"`.
///
/// A movie no widget names - `--movie` pointing at an entry by hash, which is
/// how the three unnamed reels are addressed - is played **with** its sound.
/// The widget is the authority when there is one, and its absence is an absence
/// of instruction rather than an instruction to be silent.
///
/// # Never fatal
///
/// A decode that fails names the reason on the report and leaves the movie
/// silent, exactly as a missing `ffmpeg` leaves it with a black picture. This
/// is the degradation ADR-0019 asks for, and it is the only one available:
/// there is no second decoder to fall back to.
fn load_movie_sound(
    movie: Option<&Movie>,
    screens: &Screens,
    movie_name: &str,
    options: &Options,
    report: &mut Vec<String>,
) -> Option<crate::at3::Pcm> {
    let movie = movie?;
    if options.no_video {
        // Said here rather than left to the branch below, which would report a
        // track that could not be unwrapped: `--no-video` never reads the movie
        // at all, so there was nothing to unwrap. Worth stating, because the
        // flag names only the picture and takes the sound with it.
        report.push(
            "  audio: --no-video skips reading the movie, so it has no sound either".to_string(),
        );
        return None;
    }
    let Some(audio) = movie.audio.as_ref() else {
        // A header that declares a track and a demux that could not recover one
        // has already said why on stderr; this is the line that says the movie
        // is silent as a result, so the two are not read as unrelated.
        if movie
            .header
            .as_ref()
            .is_some_and(|header| header.audio.is_some())
        {
            report.push("  audio: declared but not recovered, so it stays silent".to_string());
        }
        return None;
    };

    let silent = screens
        .with_movies()
        .filter_map(|screen| screen.movie.as_ref())
        .any(|widget| widget.entry_name().eq_ignore_ascii_case(movie_name) && !widget.sound);
    if silent {
        report.push(format!(
            "  audio: {} channel(s) at {} Hz, muted - the widget playing it is sound=\"false\"",
            audio.format.channels, audio.format.sample_rate
        ));
        return None;
    }

    match audio.decode(&options.audio_cache) {
        Ok(pcm) => {
            report.push(format!(
                "  audio: {} channel(s) at {} Hz, {} ATRAC3+ block(s) of {} bytes, decoded to \
                 {:.2}s",
                pcm.channels,
                pcm.sample_rate,
                audio.block_count(),
                audio.format.block_align,
                pcm.samples.len() as f64
                    / f64::from(pcm.channels.max(1))
                    / f64::from(pcm.sample_rate.max(1))
            ));
            Some(pcm)
        }
        Err(error) => {
            report.push(format!("  audio: not decoded ({error:#})"));
            None
        }
    }
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
    archives: &mut oag_assets::Archives,
    options: &Options,
    report: &mut Vec<String>,
) -> Option<Movie> {
    if options.no_video {
        return None;
    }
    let wanted = Options {
        extent: Extent::Whole,
        ..options.clone()
    };
    match load_movie(archives, pulse::names::BACKDROP_MOVIE, &wanted, report) {
        Ok(movie) => movie,
        // Reported and dropped. `load_movie` only errors here on an entry it
        // cannot read, and the backdrop is not worth failing a boot over.
        Err(e) => {
            report.push(format!("no menu backdrop: {e:#}"));
            None
        }
    }
}

/// Decodes Pure's second boot movie - see
/// [`oag_pure::names::FMV_INTRO_MOVIE`]'s own doc comment for what it is, how
/// its name was found, and what it takes to actually draw it (the other half
/// of that work, done alongside this function: see `crate::frontend::Frontend`
/// and `crate::main::FrontendStage` for the rest).
///
/// `None`, and nothing attempted at all, for a source whose `screens` has no
/// `FMV Intro` state - every Pulse source, and any future title this build
/// has not seen. Checked against `screens` rather than tried unconditionally
/// the way [`load_backdrop`] tries `BACKDROP_MOVIE` for every source: a miss
/// there is silent by construction (no report line), but `FMV_INTRO_MOVIE` is
/// a Pure-specific literal, and trying it against a Pulse source would add a
/// "no second boot movie" line nobody asked about.
fn load_fmv_intro(
    archives: &mut oag_assets::Archives,
    screens: &Screens,
    options: &Options,
    report: &mut Vec<String>,
) -> Option<Movie> {
    screens.by_name(pure_states::FMV_INTRO)?;
    let wanted = Options {
        extent: Extent::Whole,
        ..options.clone()
    };
    match load_movie(archives, oag_pure::names::FMV_INTRO_MOVIE, &wanted, report) {
        Ok(movie) => movie,
        Err(e) => {
            report.push(format!("no second boot movie: {e:#}"));
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
/// [`oag_assets::Archives::read_name`], which searches the *bulk* archive
/// first because that is the right default for a race: same size is not same
/// bytes, and a front-end image should come off the front end's own archive.
fn load_sprites(
    archives: &mut oag_assets::Archives,
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
/// The mirror image of [`oag_assets::Archives::read_name`]'s order, for the
/// callers that want a front-end asset specifically. See [`load_sprites`] for the
/// two entries that decide it.
///
/// A name neither archive has falls through to
/// [`oag_assets::Archives::read_image`], which knows the handful of
/// images the PS2 keeps under an entry its own XML's name does not hash to -
/// `pulse_logo.mip` among them.
fn read_front_end_first(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> oag_assets::Result<Vec<u8>> {
    if let Some(fe) = archives.fe.as_mut()
        && let Ok(blob) = fe.read_name(name)
    {
        return Ok(blob);
    }
    oag_pulse::read_image(archives, name)
}

/// Reads the front end's own font, falling back to the built-in glyphs.
///
/// A missing or undecodable font is not fatal: the menu still draws, in the 5x7
/// approximation, and the report says which one is on screen. That matters more
/// than it sounds - the two look very different, and a silent fallback would
/// make a rendering bug indistinguishable from a loading one. The PS2 spent a
/// while in exactly that state: its `.fnt` decodes as far as the metrics and
/// then stops, because the glyph sheet is a separate archive entry.
///
/// Which is why this goes through [`oag_assets::Archives::read_font`]
/// rather than [`read_front_end_first`] plus a parse: locating the atlas is the
/// archive's problem, not the front end's. The two archives hold byte-identical
/// copies of all five fonts on the PSP, checked, so reading the bulk one costs
/// nothing there.
fn load_font(archives: &mut oag_assets::Archives, report: &mut Vec<String>) -> crate::font::Atlas {
    let name = oag_pulse::names::DEFAULT_FONT;
    match archives.read_font(name).map_err(|e| e.to_string()) {
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

fn load_screens(archives: &mut oag_assets::Archives, report: &mut Vec<String>) -> Result<Screens> {
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

    // Unconditional rather than title-gated: `or_insert` only fills a name
    // Pulse's own `Skin.xml` leaves undeclared, and Pulse declares its own
    // `TitleColor`/`DesignColor` (see `oag_pure::frontend::FALLBACK_GLOBALS`'s
    // own doc comment), so this is a no-op there.
    let screens =
        Screens::from_xml_with_fallback_globals(&xml, oag_pure::frontend::FALLBACK_GLOBALS);
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
    archives: &mut oag_assets::Archives,
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

/// The game plugin's own definition, followed by every mounted pack's
/// manifest.
///
/// One list because they are one schema: a pack declares its additions as a
/// fragment of the very file the disc ships, so both go through
/// [`crate::catalogue`] unchanged. The disc's own definition is first, which is
/// what makes it win a collision.
///
/// A definition that will not read is reported and not fatal: a source whose
/// plugin is unreadable still boots and still races the default track.
fn definitions(archives: &mut oag_assets::Archives, report: &mut Vec<String>) -> Vec<String> {
    let name = pulse::names::GAME_PLUGIN_DEFINITION;
    let mut out = Vec::new();
    match archives
        .read_name(name)
        .and_then(|blob| expand(&blob).map_err(|e| oag_assets::Error::BadSpec(e.to_string())))
    {
        Ok(xml) => out.push(xml),
        Err(e) => report.push(format!("{name}: {e}")),
    }
    out.extend(archives.manifests.iter().cloned());
    out
}

/// Reads the raceable circuits out of [`definitions`].
///
/// A pack's circuit is dropped when the geometry it names is in none of the
/// mounted archives, which is what a partial set of packs looks like: two of
/// the four packs cross-declare each other's circuits, so owning one means
/// holding a declaration for a track whose `.vex` is in a pack you did not buy.
/// Skipped and reported, rather than offered and then failing at the archive
/// with a message about a missing entry.
fn load_tracks(
    archives: &mut oag_assets::Archives,
    documents: &[String],
    report: &mut Vec<String>,
) -> Vec<crate::catalogue::Track> {
    let declared = crate::catalogue::all_tracks(documents);
    let declared_count = declared.len();
    let tracks: Vec<_> = declared
        .into_iter()
        .filter(|track| archives.locate(&track.entry_name()).is_some())
        .collect();

    if tracks.len() != declared_count {
        report.push(format!(
            "dlc: {} declared circuit(s) have no geometry on this source; a pack \
             they belong to is not mounted",
            declared_count - tracks.len()
        ));
    }
    report.push(format!(
        "{}: {} raceable circuit(s) over {} definition(s)",
        pulse::names::GAME_PLUGIN_DEFINITION,
        tracks.len(),
        documents.len()
    ));
    tracks
}

/// Reads the roster out of [`definitions`], the same way [`load_tracks`] reads
/// the circuits.
///
/// Filtered the same way too, and for the same reason - but against **both**
/// files a race needs, not just the model.
///
/// A pack splits the two across archives: the ship is in `PACKn.edat` and the
/// handling stats are in `PACKn_UI1.edat`. A hand-copied or repacked pack
/// holding only the first would pass a model-only filter, appear in the menu,
/// and then fail at the stats read the moment it was picked. Checking both is
/// what keeps "offered" and "raceable" the same set - the job the deleted
/// menu.rs assertion used to do when the roster was a fixed list.
///
/// Both names are composed by the same functions `race::load` will call, so the
/// filter cannot disagree with the loader about how a path is spelled.
fn load_teams(
    archives: &mut oag_assets::Archives,
    documents: &[String],
    report: &mut Vec<String>,
) -> Vec<crate::catalogue::Team> {
    let declared = crate::catalogue::all_teams(documents);
    let declared_count = declared.len();
    let mut teams: Vec<_> = declared
        .into_iter()
        .filter(|team| raceable(archives, &team.id))
        .collect();

    if teams.len() != declared_count {
        report.push(format!(
            "{} declared team(s) have no ship or no handling stats on this \
             source; a pack they belong to may be only half mounted",
            declared_count - teams.len()
        ));
    }

    // A source whose plugin definition will not read is reported and not fatal
    // - see `definitions` - but a TEAM row with nothing in it would leave the
    // player unable to start a race at all, where the circuits row still has a
    // default to fall back on. So the eight teams the PSP disc always ships
    // stand in, still filtered against what is really there. This is the list
    // the menu definition itself carried before the roster became data.
    if teams.is_empty() {
        teams = oag_formats::handling::TEAMS
            .iter()
            .filter(|id| raceable(archives, id))
            .map(|id| crate::catalogue::Team {
                id: (*id).to_string(),
                location: format!(r"Data\Ships\{id}"),
                help_text: None,
            })
            .collect();
        if !teams.is_empty() {
            report.push(format!(
                "no team was declared; falling back to the {} shipped team(s) \
                 this source actually carries",
                teams.len()
            ));
        }
    }

    report.push(format!(
        "{}: {} team(s) over {} definition(s)",
        pulse::names::GAME_PLUGIN_DEFINITION,
        teams.len(),
        documents.len()
    ));
    teams
}

/// Whether both files a race reads for a team are on this source.
fn raceable(archives: &oag_assets::Archives, id: &str) -> bool {
    archives
        .locate(&crate::race::ship_entry_name(id, oag_race::Mode::default()))
        .is_some()
        && archives
            .locate(&oag_formats::handling::entry_name(id))
            .is_some()
}

/// The chosen language's string table.
///
/// Public for the same reason [`load_languages`] is: the HUD resolves `IG_HUD_*`
/// keys through this, and a race reaches it without booting the front end.
pub fn load_strings(
    archives: &mut oag_assets::Archives,
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

/// [`DEFAULT_BOOT_MOVIE`], generalised over which title `screens` came from.
///
/// `--reel` (`Leg::DevPubReel`) always wants [`DEVPUB_REEL`] - that leg is
/// reached explicitly, by a flag, on Pulse alone, so the title makes no
/// difference to it. `Leg::LogoFmv` is the one this build boots into by
/// default for any source, and `DEFAULT_BOOT_MOVIE` names an entry only
/// Pulse has - so a Pure source (detected the same way
/// `Frontend::language_confirm_target` is: by whether its own `Title Screen`
/// exists) gets [`oag_pure::names::INTRO_MOVIE`] instead.
fn default_boot_movie(leg: crate::frontend::Leg, screens: &Screens) -> String {
    match leg {
        crate::frontend::Leg::DevPubReel => DEVPUB_REEL.to_string(),
        crate::frontend::Leg::LogoFmv => {
            if screens.by_name(pure_states::TITLE_SCREEN).is_some() {
                oag_pure::names::INTRO_MOVIE.to_string()
            } else {
                DEFAULT_BOOT_MOVIE.to_string()
            }
        }
    }
}

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
    archives: &mut oag_assets::Archives,
    movie_name: &str,
    options: &Options,
    report: &mut Vec<String>,
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
            if let Some(movie) = load_loose_movie(movie_name, options, report)? {
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
    let movie = movie::open(&blob, &key, &options.cache, options.extent, false)?;

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
    let Some((path, blob)) = oag_assets::read_loose_file(&options.source, candidates)? else {
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

/// The default movie cache directory: `data/cache/movies` in a repository
/// checkout, and `<cache dir>/oag/movies` anywhere else - see
/// [`cache_dir_named`] for how the two are told apart.
///
/// Deleting either directory is always safe; see
/// `docs/architecture/adr/0004-asset-pipeline.md`.
#[must_use]
pub fn default_cache_dir() -> std::path::PathBuf {
    cache_dir_named("movies")
}

/// The default decoded-audio directory: `data/cache/audio` in a repository
/// checkout, and `<cache dir>/oag/audio` anywhere else.
///
/// A sibling of [`default_cache_dir`] rather than the same directory, because
/// the two hold different things with different lifetimes - lossless AV1 of a
/// movie, and PCM decoded out of ATRAC3+ - and a player clearing one should not
/// have to re-transcode the other. `--cache` names the movie directory only,
/// which is what its help text has always said. See
/// `docs/architecture/adr/0019-atrac3plus-out-of-process.md`.
#[must_use]
pub fn default_audio_cache_dir() -> std::path::PathBuf {
    cache_dir_named("audio")
}

/// The default unpacked-DLC directory: `data/cache/dlc` in a repository
/// checkout, and `<cache dir>/oag/dlc` anywhere else.
///
/// A third sibling for the same reason the second one exists: what lands here
/// is `PACKn.edat` copied out of a downloaded zip, which is derived from a file
/// the player already has and is therefore always safe to delete. Keeping it
/// out of the movie and audio directories means clearing one cache never costs
/// the others - and, unlike those two, nothing here is transcoded, so a stale
/// entry is cheap to spot: it is a byte copy or it is wrong.
///
/// See [`crate::dlc_cache`].
#[must_use]
pub fn default_dlc_cache_dir() -> std::path::PathBuf {
    cache_dir_named("dlc")
}

/// `data/cache/<what>` in a checkout, `<cache dir>/oag/<what>` anywhere else.
///
/// A checkout is recognised by having a `data/` directory, which is where
/// everything user-supplied already lives and what `just` recipes and
/// `data/README.md` document. A packaged build has no checkout around it, and
/// writing beside wherever it happens to have been run from would scatter a
/// cache through a player's folders - or fail outright, if that is a read-only
/// mount.
fn cache_dir_named(what: &str) -> std::path::PathBuf {
    let checkout = Path::new("data/cache").join(what);
    if Path::new("data").is_dir() {
        return checkout;
    }
    dirs::cache_dir().map_or_else(|| checkout, |cache| cache.join("oag").join(what))
}

#[cfg(test)]
mod tests {
    use super::{DEVPUB_REEL, EntryRef, pulse};

    /// The CLI default spells the reel's hash as text, and the title package
    /// holds it as a number. Two spellings of one fact can drift silently -
    /// nothing would fail, `--reel` would simply address an entry that is not
    /// there - so the text form is checked against the number rather than
    /// trusted to stay in step.
    #[test]
    fn the_reel_default_spells_the_hash_the_title_package_holds() {
        assert_eq!(
            DEVPUB_REEL,
            format!("hash:{:08x}", pulse::hashes::DEVPUB_REEL_SCEE)
        );
        assert_eq!(
            EntryRef::parse(DEVPUB_REEL).hash(),
            pulse::hashes::DEVPUB_REEL_SCEE
        );
    }

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
