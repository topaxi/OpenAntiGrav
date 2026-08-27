//! Loading the boot sequence out of a disc image.
//!
//! Everything the front end needs, in the order the original needs it: the
//! front-end root XML, the language plugins, one language's string table, and
//! the intro movie. Kept apart from `main.rs` so the load can be exercised
//! without a window, and apart from [`crate::frontend`] so the sequence itself
//! stays free of file I/O.

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use oag_formats::fexml;
use oag_pulse as pulse;

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
    /// The second movie this title's chain names, if it names one - see
    /// [`load_second_movie`]. `None` for a title whose boot plays only one movie,
    /// the same way `backdrop` is `None` for a source with no menu backdrop.
    pub after_language_movie: Option<Movie>,
    /// That movie's own sound, on exactly the same terms as [`Self::movie_sound`].
    ///
    /// Kept apart because the two are played at different moments - each on the
    /// tick its own screen is entered - and one field would have to be reloaded
    /// rather than handed over.
    pub after_language_movie_sound: Option<crate::at3::Pcm>,
    /// Every front-end image the screens reference, in one texture.
    pub sprites: crate::sprite::Sheet,
    /// How this title lays its menus out and colours them. See
    /// [`Shell::menu_skin`].
    pub menu_skin: &'static oag_title::MenuSkin,
    /// The frame its menus are drawn inside. See [`Shell::frame`].
    pub frame: crate::menu::Frame,
    /// The face menu rows are drawn in. See [`Shell::menu_font`].
    pub menu_font: Option<crate::font::Atlas>,
    /// The text atlas: the disc's own font when it decodes, ours when it does
    /// not.
    pub font: crate::font::Atlas,
    /// The chosen language's string-table entry, carried through from
    /// [`Shell::entries`] for the one caller that needs the other copies of it.
    pub entries: Option<String>,
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

impl Boot {
    /// The plane geometry the front end's one video pipeline is built for.
    ///
    /// The first movie when there is one, and the second when there is not. That
    /// fallback is the whole point of the method: a title whose boot screen plays
    /// nothing still draws a movie later, and taking the format from the first
    /// alone would build a renderer with **no video pipeline at all** - so that
    /// movie's first frame would not merely be skipped, it would fail the run,
    /// `upload_frame` returning "no video pipeline".
    ///
    /// A method rather than a line at each call site because there are two of
    /// them - the window and the headless capture - and they must not be able to
    /// size the same pipeline differently.
    #[must_use]
    pub fn video_format(&self) -> Option<crate::render::VideoFormat> {
        self.movie
            .as_ref()
            .or(self.after_language_movie.as_ref())
            .and_then(crate::render::VideoFormat::of)
    }
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
    /// Convert every movie again even when the cache already holds it, and
    /// overwrite what is there. See [`crate::movie::Decode::refresh`].
    pub refresh_video: bool,
    /// Take the AV1 cache path even where a platform decoder is available. See
    /// [`crate::movie::Decode::prefer_cache`].
    pub prefer_av1_cache: bool,
}

impl Options {
    /// How the movie loaders should open a picture, from these options.
    fn decode(&self) -> crate::movie::Decode {
        crate::movie::Decode {
            no_video: self.no_video,
            refresh: self.refresh_video,
            prefer_cache: self.prefer_av1_cache,
        }
    }
}

/// A step the load has to be quick enough at for nobody to notice. Anything
/// slower gets named in the report; anything faster would only be noise there.
const FELT: std::time::Duration = std::time::Duration::from_millis(20);

/// A stopwatch that names each step of the load as it passes.
///
/// The boot is the longest wait this build asks anyone to sit through and it
/// used to be one opaque call, so "which part of it" was a question nobody
/// could answer without a profiler. One [`Steps::lap`] per step answers it on
/// every boot, in the report the load already prints, which is also what keeps
/// the answer current: a step that gets slower says so rather than waiting to
/// be re-measured.
///
/// Wall clock, deliberately, and this is the one place in the codebase that is
/// allowed to be - see `docs/architecture/determinism.md`. Nothing here reaches
/// the simulation: these are strings for a human.
struct Steps {
    at: std::time::Instant,
    steps: Vec<(&'static str, std::time::Duration)>,
}

impl Steps {
    fn new() -> Self {
        Self {
            at: std::time::Instant::now(),
            steps: Vec::new(),
        }
    }

    /// Closes the step that has been running since the last lap.
    fn lap(&mut self, what: &'static str) {
        let now = std::time::Instant::now();
        self.steps.push((what, now - self.at));
        self.at = now;
    }

    /// One line: the total, then the steps that were felt, slowest first.
    ///
    /// Slowest first rather than in load order because the reason to read this
    /// line at all is "what am I waiting for", and that is the first name on it.
    fn describe(&self, what: &str) -> String {
        let total: std::time::Duration = self.steps.iter().map(|(_, took)| *took).sum();
        let mut felt: Vec<_> = self
            .steps
            .iter()
            .filter(|(_, took)| *took >= FELT)
            .collect();
        felt.sort_by_key(|(_, took)| std::cmp::Reverse(*took));
        let named = felt
            .iter()
            .map(|(name, took)| format!("{name} {:.2}", took.as_secs_f32()))
            .collect::<Vec<_>>()
            .join(", ");
        if named.is_empty() {
            return format!("{what} took {:.2} s", total.as_secs_f32());
        }
        format!("{what} took {:.2} s - {named}", total.as_secs_f32())
    }
}

/// Loads everything and builds the sequence.
///
/// **The blocking whole**, for callers with nothing to draw while they wait: a
/// headless capture, the ground-truth tests. A window uses the three phases
/// this is made of - [`load_shell`], [`load_media`] and [`assemble`] - so that
/// it can put the loading screen up between them. See [`Shell`] for why the
/// line falls where it does.
pub fn load(options: &Options) -> Result<Boot> {
    let (mut shell, archives) = load_shell(options)?;
    // A throwaway tally: this caller is the one that blocks, so there is nothing
    // on screen to read it.
    let media = load_media(
        archives,
        &shell.screens,
        &shell.media_plan(),
        options,
        &Mutex::new(MediaProgress::default()),
    );
    shell.report.extend(media.report.iter().cloned());
    Ok(assemble(shell, media))
}

/// Everything a window needs before it can draw anything at all.
///
/// **The cheap half of the boot**, and cheap is measured rather than hoped:
/// 0.05 s of the 4.93 s a `native-video` boot of the EU disc takes, the rest
/// of it being the two movies in [`load_media`]. That ratio is the whole
/// reason the line is here - a phase this short can run before the event loop
/// with nobody noticing, and everything after it can run under a loading
/// screen.
///
/// Carries no [`oag_assets::Archives`]: [`load_shell`] hands those back
/// separately, because the next phase takes them onto a worker thread and
/// this half stays on the one that owns the window.
pub struct Shell {
    /// The front-end XML. Cloned for the media worker, moved into
    /// [`Frontend::booting`] by [`assemble`].
    pub screens: Screens,
    /// Every language this source offers.
    pub languages: Vec<Language>,
    /// The chosen language's strings. Also what
    /// [`crate::loading::Assets::load`] needs to put a tip on the loading
    /// screen, which is the other reason this phase exists.
    pub strings: StringTable,
    /// The raceable circuits, for the menus.
    pub tracks: Vec<crate::catalogue::Track>,
    /// The chosen language's string-table entry, when it names one.
    ///
    /// Carried because the loading screen needs the *other copies* of it: two
    /// of Wipeout HD's five feature descriptions are only in the copy that also
    /// carries all 28 circuit names, and picking that copy needs the path. See
    /// `crate::loading::Assets::load`.
    pub entries: Option<String>,
    /// This title's own loading-screen table, carried for the same reason
    /// [`Self::menu_skin`] is: it is a property of the source, settled while the
    /// serial was in hand, and the menus need its style names to offer a row.
    /// `None` on a title that authors no loading screen. See
    /// [`oag_title::Loading`].
    pub loading: Option<&'static oag_title::Loading>,
    /// What to call each of them, resolved beside them.
    ///
    /// Not folded into [`Self::strings`] because it may come out of a
    /// **different copy** of the string table than the rest of the front end
    /// does; see [`crate::language::CircuitNames`]. Empty on a source where no
    /// copy names every circuit, which shows each one its id.
    pub circuit_names: crate::language::CircuitNames,
    /// The raceable teams, for the menus.
    pub teams: Vec<crate::catalogue::Team>,
    /// The text atlas.
    pub font: crate::font::Atlas,
    /// The front-end sprite sheet.
    pub sprites: crate::sprite::Sheet,
    /// The grid this source authors its widgets in, read off the archives'
    /// own platform while they are still in hand - [`assemble`] has no
    /// archives to ask by the time it needs this.
    pub space: crate::frontend::Space,
    /// This title's own boot table, selected from the serial before any XML was
    /// parsed. Carried so the later phases ask it rather than the screens.
    pub profile: &'static oag_title::BootProfile,
    /// This title's own menu layout and colours, carried the same way and for
    /// the same reason as [`Self::profile`]: the serial settled which title
    /// this is, so nothing downstream has to ask again.
    ///
    /// Presentation only. The menu *tree* is this build's own - see
    /// `docs/architecture/menus.md`.
    pub menu_skin: &'static oag_title::MenuSkin,
    /// The widgets of the screen this title frames its menus with: what they
    /// clear to and the marks they are drawn between.
    ///
    /// Which screen that was is [`oag_title::FrontEnd::menu_frame`] and is not
    /// carried here - by this point it has been read, and a name nothing reads
    /// is the inert-field smell this file avoids elsewhere. See
    /// [`crate::menu::read_frame`].
    ///
    /// Built here rather than by whoever draws, because it needs the parsed
    /// screens *and* the sprite sheet *and* the grid, and this is the only place
    /// all three are in hand. Empty for a title whose frame is unread, which
    /// draws the menus exactly as they were drawn before this existed.
    pub frame: crate::menu::Frame,
    /// The face menu rows are drawn in, when the title names one and it
    /// reads. `None` falls the menus back to [`Self::font`].
    pub menu_font: Option<crate::font::Atlas>,
    /// The screens this boot walks, in the title's own order, already filtered
    /// to the ones this pressing carries and this build can drive.
    ///
    /// Resolved in this phase because it depends on [`Self::screens`], and the
    /// media phase needs the answer to know which movies to read. See
    /// [ADR-0023](../../../docs/architecture/adr/0023-boot-sequence-as-title-data.md).
    pub walked: Vec<&'static oag_title::BootStep>,
    /// Which entry the first movie in that chain is, if it has one.
    ///
    /// `None` for a boot that plays nothing anywhere - and **not** the same
    /// question as "does the boot screen play something": on Pure neither movie is
    /// on the boot step, so this names the reel that plays one screen later.
    pub movie_name: Option<String>,
    /// The chain's second movie, on the same terms.
    pub second_movie_name: Option<&'static str>,
    /// Which leg the sequence boots into, carried for [`assemble`].
    pub leg: crate::frontend::Leg,
    /// What to print. [`assemble`] appends its own.
    pub report: Vec<String>,
}

impl Shell {
    /// What the media phase should read, from what this phase resolved.
    #[must_use]
    pub fn media_plan(&self) -> MediaPlan {
        MediaPlan {
            movie_name: self.movie_name.clone(),
            second_movie_name: self.second_movie_name,
            menu_backdrop: self.profile.menu_backdrop,
        }
    }
}

// Written out rather than derived, for the reason [`Boot`]'s is: the sprite
// sheet and the string table are megabytes between them, and a `{:?}` of this
// should say which source it came off rather than print the disc.
impl std::fmt::Debug for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shell")
            .field("movie_name", &self.movie_name)
            .field("leg", &self.leg)
            .finish_non_exhaustive()
    }
}

/// The slow half: the movies, and the intro's own sound.
///
/// Every field is what it is on a source that has none of it - `None` is an
/// ordinary outcome throughout, exactly as it is on [`Boot`]'s own fields.
/// [`Default`] is therefore a *meaningful* value here rather than a filler: it
/// is the boot of a source that carries no movie at all, which the sequence
/// plays out on black.
#[derive(Default)]
pub struct Media {
    /// The intro reel.
    pub movie: Option<Movie>,
    /// Its ATRAC3+ track, decoded.
    pub movie_sound: Option<crate::at3::Pcm>,
    /// The looping menu backdrop.
    pub backdrop: Option<Movie>,
    /// The chain's second movie, and its own track.
    pub after_language_movie: Option<Movie>,
    pub after_language_movie_sound: Option<crate::at3::Pcm>,
    /// What to print, kept separate because this half may finish on another
    /// thread and its lines must not interleave with the shell's.
    pub report: Vec<String>,
}

// Same reasoning as [`Shell`]'s and [`Boot`]'s: the three movies are hundreds
// of megabytes of decoded picture on a `native-video` build.
impl std::fmt::Debug for Media {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Media")
            .field("movie", &self.movie)
            .finish_non_exhaustive()
    }
}

/// Opens the source and loads everything that is not a movie.
///
/// Returns the archives alongside, still open, for [`load_media`] to take.
///
/// # Errors
///
/// A source whose archives will not open, or which carries no front-end XML.
/// Everything else degrades into a report line.
pub fn load_shell(options: &Options) -> Result<(Shell, oag_assets::Archives)> {
    let mut report = Vec::new();
    let mut steps = Steps::new();
    let (packs, problems) = crate::dlc::packs(&options.dlc, &default_dlc_cache_dir());
    let crate::title::Opened {
        mut archives,
        title,
    } = open_source(&options.source, packs)
        .with_context(|| format!("opening the archives in {}", options.source))?;
    // Which title this is was settled by the serial, here, and everything below
    // asks `profile` rather than asking the XML again. See ADR-0023.
    //
    // **A title with no recovered front end is still refused by name**, and no
    // title is in that state today. What used to be refused here as well was a
    // front end whose *chain* was only declared - Wipeout HD's - and that is now
    // expressed rather than withheld: see `oag_title::Provenance` and ADR-0025.
    // Substituting a sibling title's chain would still be wrong, and is still
    // not done.
    let front_end = title.front_end.ok_or_else(|| {
        anyhow::anyhow!(
            "{}'s front end is not recovered, so there is no sequence to walk: \
             race on it with --race instead",
            title.name
        )
    })?;
    let profile = front_end.boot;
    report.push(archives.layout.describe());
    report.push(format!("{}: boot sequence", title.name));
    // Said on every boot of such a title, and near the top where the reader is
    // still looking: a sequence read out of the disc's XML is not a sequence
    // anyone has watched, and a screenshot of it must not be filed as evidence
    // of what the original does.
    if !profile.provenance.is_measured() {
        report.push(format!(
            "{}: this order is what its front-end XML declares, not a boot \
             anyone has watched",
            title.name
        ));
    }
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
        archives.data.entry_count()
    ));
    if let Some(fe) = &archives.fe {
        report.push(format!("{}: {} entries", fe.label(), fe.entry_count()));
    }

    steps.lap("open");
    // **Languages before fonts**, because every face this source draws with is
    // named by a language plugin's `<Font>` slots - see [`load_font`], which
    // used to reach for a constant here and so did not need them.
    let languages = load_languages(&mut archives, front_end.language_plugins, &mut report);
    let offered = languages.clone();
    steps.lap("languages");
    let font = load_font(&mut archives, &languages, &mut report);
    steps.lap("font");
    let screens = load_screens(
        &mut archives,
        front_end.root,
        profile.fallback_globals,
        &mut report,
    )?;
    steps.lap("screens");
    let strings = load_strings(
        &mut archives,
        &languages,
        options.language.as_deref(),
        &mut report,
    );
    steps.lap("strings");
    let menu_font = load_menu_font(&mut archives, &languages, front_end.menu, &mut report);
    steps.lap("menu font");
    let definition = title.plugin_definition;
    let documents = definitions(&mut archives, definition, &mut report);
    let tracks = load_tracks(&mut archives, definition, &documents, &mut report);
    let teams = load_teams(
        &mut archives,
        (title.race.ship_dir, title.race.handling_dir),
        definition,
        &documents,
        &mut report,
    );
    // After the circuits, because which copy of the string table names them all
    // is a question about the list this just produced.
    let circuit_names = load_circuit_names(
        &mut archives,
        chosen_language(&languages, options.language.as_deref()),
        &strings,
        &tracks,
        &mut report,
    );
    steps.lap("catalogue");
    let sprites = load_sprites(&mut archives, &screens, &mut report);
    steps.lap("sprites");
    // Which step the boot opens on, from this title's own chain. `--reel`
    // replaces the boot step rather than preceding it, and is refused by name on
    // a title with no evidenced reel state rather than pointed at another
    // title's screen.
    let start_step = match options.leg {
        crate::frontend::Leg::LogoFmv => profile.start(),
        crate::frontend::Leg::DevPubReel => profile.reel.as_ref().with_context(|| {
            format!(
                "--reel is an off-path dev/pub reel state; {} has no equivalent anyone \
                 has found",
                title.name
            )
        })?,
    };
    // The rest of the chain, filtered to what this pressing carries *and* this
    // build can drive. A step failing either is stepped over **and reported**: a
    // silently shortened sequence is exactly the kind of thing that reads as
    // finished work, which is how this build once shipped Pure's order wrong.
    //
    // **The chain's first step goes through the same two checks**, which it did
    // not until finding G3 of the 2026-08-18 review: `walked` started with
    // `profile.start()` unconditionally, so a pressing lacking the boot screen -
    // a demo or trial disc - walked an unbacked state that any *later* step
    // would have been skipped and reported for. The report is the point: a boot
    // that opens on a screen this source does not carry should say so, not
    // present as a blank one.
    //
    // **`--reel` is exempt, and that is the whole nature of the flag.** The
    // dev/pub reel is an *off-path* state: it is real and evidenced (its
    // `OnEnter` caches `"DevPubRedirect"` at `0x088d7d80`) and the disc's own
    // boot never enters it, so it has no screen in the set to be found by. It
    // is reached because the operator named it, not because a chain led there,
    // and checking a screen set that by construction does not carry it turns
    // `--reel` into a no-op - which is what this check did on its first pass,
    // caught by `boot_ground_truth::the_reel_leg_still_runs_its_frame_holds`.
    let mut walked: Vec<&'static oag_title::BootStep> = Vec::new();
    let off_path = options.leg == crate::frontend::Leg::DevPubReel;
    if !off_path && screens.by_name(start_step.state).is_none() {
        report.push(format!(
            "the chain opens on {:?}, which this pressing does not carry",
            start_step.state
        ));
    } else if off_path || crate::frontend::can_drive(start_step.state) {
        walked.push(start_step);
    } else {
        report.push(format!(
            "the chain opens on {:?}, which this build has no behaviour for yet",
            start_step.state
        ));
    }
    for step in profile.chain.iter().skip(1) {
        // A reel step that is *also* a chain step - Pure's, which is on its boot
        // path - would otherwise be walked twice and its movie read twice.
        if step.state == start_step.state {
            continue;
        }
        if screens.by_name(step.state).is_none() {
            report.push(format!(
                "the chain's {:?} is skipped: this pressing does not carry it",
                step.state
            ));
        } else if crate::frontend::can_drive(step.state) {
            walked.push(step);
        } else {
            report.push(format!(
                "the disc's {:?} is skipped: this build has no behaviour for it yet",
                step.state
            ));
        }
    }
    // Every step that plays something, in order. Pulse has one (its boot step);
    // Pure has two, and **neither is its boot step** - the reel plays on the
    // developer/publisher screen and the FMV two steps later. Keying either off
    // "the boot step" or "the step after the picker" gets one of the two titles
    // wrong.
    let playing: Vec<&'static oag_title::BootStep> = walked
        .iter()
        .copied()
        .filter(|step| step.movie.is_some())
        .collect();
    // `--movie` overrides whichever movie the sequence draws first, so the flag
    // stays a preview tool on a title whose own boot screen is silent.
    let movie_name = options.movie.clone().or_else(|| {
        playing
            .first()
            .and_then(|step| step.movie)
            .map(str::to_string)
    });
    let second_movie_name = playing.get(1).and_then(|step| step.movie);
    // The grid this source authors in, needed before the front end is built so
    // that a boot with no movie falls back to the *source's* shape rather than
    // to the PSP's. `frontend.set_space` takes the same value.
    let space = crate::frontend::Space::of(archives.layout.platform);
    // The frame the menus are drawn inside, off the screen this title names -
    // built here because this is where the parsed XML, the sheet and the grid
    // are all in hand, and **reported** because which archive served the root
    // decides what colour it comes out in: Wipeout HD's `HD_*` palette is its FE
    // style, black and red in `DATA00` against white and teal in `DATA06`. A
    // menu that looks like the wrong game is then a line in the boot report
    // rather than a mystery. See [`crate::menu::frame`].
    let frame = crate::menu::read_frame(&screens, &sprites, space, front_end.menu_frame);
    if let Some(name) = front_end.menu_frame {
        report.push(format!("menu frame {name}: {}", describe_frame(&frame)));
    }
    report.push(steps.describe("the boot's first half"));

    Ok((
        Shell {
            screens,
            entries: chosen_language(&languages, options.language.as_deref())
                .and_then(|language| language.entries.clone()),
            loading: title.loading,
            languages: offered,
            strings,
            tracks,
            circuit_names,
            teams,
            font,
            sprites,
            space,
            profile,
            menu_skin: front_end.menu,
            frame,
            menu_font,
            walked,
            movie_name,
            second_movie_name,
            leg: options.leg,
            report,
        },
        archives,
    ))
}

/// Loads the movies and the intro's sound. **The slow half**, by a factor of a
/// hundred on a `native-video` build - see [`Shell`].
///
/// Takes the archives by value because this is what runs on a worker thread
/// while a window is already up, and a half-loaded boot is not something two
/// threads should be reaching into. Never fails: every movie that will not load
/// degrades into a report line and a `None`, which is what the sequence already
/// copes with everywhere.
///
/// `progress` is written before and after each load so the loading screen can
/// draw a bar over the seconds this takes; a caller with nothing to draw passes
/// a throwaway, the way [`load`] does.
pub fn load_media(
    mut archives: oag_assets::Archives,
    screens: &Screens,
    plan: &MediaPlan,
    options: &Options,
    progress: &Mutex<MediaProgress>,
) -> Media {
    let mut report = Vec::new();
    let mut steps = Steps::new();
    // The denominator before the first load rather than after it: a bar that
    // appeared one movie in would be up for the shortest part of the wait.
    lock_media(progress).total = plan.loads();
    // One watch for all three movie loads: each `starting` clears the step, so
    // there is no state here to carry between them.
    let watch = watching(progress);
    let watch: crate::movie::Watch<'_> = Some(&watch);
    // Kept as a report line rather than propagated: by the time this runs the
    // window is up and the loading screen is drawing, so a movie that will not
    // open has to be survivable. `load` above is the caller that used to be
    // able to fail here, and its `?` only ever fired on a *named* entry being
    // absent - which `Movie: None` already describes.
    let movie = match &plan.movie_name {
        Some(name) => {
            starting(progress, name);
            let movie = match load_movie(&mut archives, name, options, &mut report, watch) {
                Ok(movie) => movie,
                Err(error) => {
                    report.push(format!("{name}: {error:#}"));
                    None
                }
            };
            loaded(progress);
            movie
        }
        None => None,
    };
    steps.lap("intro");
    // Straight after the movie, so its report lines stay together, and while
    // `screens` is still in hand: the widget that decides whether this movie is
    // heard at all is in that XML.
    let movie_sound = match &plan.movie_name {
        Some(name) => {
            starting(progress, &format!("{name} (sound)"));
            let sound = load_movie_sound(movie.as_ref(), screens, name, options, &mut report);
            loaded(progress);
            sound
        }
        None => None,
    };
    steps.lap("intro sound");
    let backdrop = match plan.menu_backdrop {
        Some(name) => {
            starting(progress, name);
            let backdrop = load_backdrop(&mut archives, name, options, &mut report, watch);
            loaded(progress);
            backdrop
        }
        None => None,
    };
    steps.lap("backdrop");
    let mut after_language_movie = match plan.second_movie_name {
        Some(name) => {
            starting(progress, name);
            let second = load_second_movie(&mut archives, name, options, &mut report, watch);
            loaded(progress);
            second
        }
        None => None,
    };
    // The same one-pipeline guard the backdrop goes through: `upload_frame`
    // slices by the pipeline's dimensions, so a movie of another shape is a
    // garbled picture rather than an error.
    if let (Some(first), Some(second)) = (movie.as_ref(), after_language_movie.as_ref())
        && !same_planes(second, first)
    {
        report.push(
            "the second boot movie is skipped: its planes are not the first movie's, \
             and the front end has one video pipeline"
                .to_string(),
        );
        after_language_movie = None;
    }
    let after_language_movie_sound = match plan.second_movie_name {
        Some(name) => {
            starting(progress, &format!("{name} (sound)"));
            let sound = load_movie_sound(
                after_language_movie.as_ref(),
                screens,
                name,
                options,
                &mut report,
            );
            loaded(progress);
            sound
        }
        None => None,
    };
    steps.lap("second movie");
    report.push(steps.describe("the boot's movies"));
    // Nothing is loading any more, and the name of the last thing that was
    // would otherwise stay under the bar for the whole fade.
    lock_media(progress).current = None;

    Media {
        movie,
        movie_sound,
        backdrop,
        after_language_movie,
        after_language_movie_sound,
        report,
    }
}

/// What the media phase is being asked to read, resolved by the shell phase.
///
/// A struct rather than three arguments because the three travel together across
/// a thread boundary and are all answers to the same question - which movies this
/// title's chain names - decided where the chain is.
#[derive(Debug, Clone)]
pub struct MediaPlan {
    pub movie_name: Option<String>,
    pub second_movie_name: Option<&'static str>,
    pub menu_backdrop: Option<&'static str>,
}

impl MediaPlan {
    /// How many loads [`load_media`] will attempt for this plan.
    ///
    /// Counted from the names rather than from what succeeds, because this is
    /// the denominator of a bar that goes up while the loads are still running:
    /// a total that shrank when a movie failed would make the fraction jump
    /// backwards. A load that degrades to `None` still counts as done - the wait
    /// it represents happened either way, which is the only thing the bar
    /// measures.
    ///
    /// Each named movie is **two** loads, its picture and its ATRAC3+ track, and
    /// they are counted apart because they are two out-of-process decodes with
    /// nothing but the name in common. The backdrop has no track and is one.
    #[must_use]
    pub fn loads(&self) -> usize {
        usize::from(self.movie_name.is_some()) * 2
            + usize::from(self.second_movie_name.is_some()) * 2
            + usize::from(self.menu_backdrop.is_some())
    }
}

/// How far the media phase has got.
///
/// A snapshot handed out by value, on the same terms as
/// [`crate::prefetch::Progress`]: the loading screen polls it from the frame
/// loop it already has and holds no lock while it draws.
///
/// There is no `finished` here because [`MediaWorker::is_finished`] already
/// answers that, and the thread is what knows - `done == total` is true for the
/// moment between the last load returning and the thread handing back its
/// [`Media`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaProgress {
    /// What [`MediaPlan::loads`] counted, or `0` before the phase starts.
    pub total: usize,
    /// Loads that have returned, successfully or not.
    pub done: usize,
    /// What is loading right now, by the entry name the plan gave.
    pub current: Option<String>,
    /// What that load is actually doing - see [`crate::movie::Step`].
    ///
    /// **This is the difference between a wait nobody notices and eighty
    /// seconds of one.** `Some(Step::Cached)` and `Some(Step::Transcoding)` sit
    /// under the same entry name and mean entirely different things to whoever
    /// is looking at the screen. `None` before a load has said anything, which
    /// includes the sound loads: [`crate::at3`] has its own cache and does not
    /// report through this.
    pub step: Option<crate::movie::Step>,
}

/// Names what is about to load. Overwrites rather than clears, so the label
/// under the bar never blinks empty between two loads.
///
/// The step is cleared, though, and must be: it described the *previous* load,
/// and carrying it over would caption a cache hit with the last transcode's
/// frame counter.
fn starting(progress: &Mutex<MediaProgress>, what: &str) {
    let mut at = lock_media(progress);
    at.current = Some(what.to_string());
    at.step = None;
}

/// Counts a load that has returned, however it returned. See [`MediaPlan::loads`].
fn loaded(progress: &Mutex<MediaProgress>) {
    lock_media(progress).done += 1;
}

/// The callback the movie loaders report their [`crate::movie::Step`] through.
fn watching(progress: &Mutex<MediaProgress>) -> impl Fn(crate::movie::Step) + Sync {
    move |step| lock_media(progress).step = Some(step)
}

/// The same rule [`crate::prefetch`]'s own `lock` follows: a poisoned lock is a
/// worker that panicked, and the last snapshot it wrote is still a true
/// statement about what got done. Bringing the window down over it would swap a
/// boot with no movies for no boot at all.
fn lock_media(progress: &Mutex<MediaProgress>) -> std::sync::MutexGuard<'_, MediaProgress> {
    progress
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// [`load_media`] running on a thread of its own, so a window can open first.
///
/// **This is the whole reason the boot is in halves.** On a `native-video`
/// build the movies are 4.9 of a 5.0-second boot of the EU disc - `GstDecoder`
/// decodes every frame of both reels into memory before it returns - and all
/// of that used to happen before winit had been told to make a window. There
/// was nothing on screen to say the game had started because there was no
/// screen.
#[derive(Debug)]
pub struct MediaWorker {
    /// `None` once joined, which is what makes [`Self::join`] idempotent.
    handle: Option<std::thread::JoinHandle<Media>>,
    /// What the loading screen draws its bar from while the thread runs.
    progress: Arc<Mutex<MediaProgress>>,
}

impl MediaWorker {
    /// Starts the media phase in the background.
    ///
    /// Takes copies rather than borrows because the thread outlives this call
    /// by seconds; `Screens` is a dozen parsed widget trees and `Options` a
    /// handful of paths, which is nothing beside what the thread then decodes.
    #[must_use]
    pub fn spawn(archives: oag_assets::Archives, shell: &Shell, options: &Options) -> Self {
        let screens = shell.screens.clone();
        let plan = shell.media_plan();
        let options = options.clone();
        // The total is filled in on the thread, by `load_media` - but it is
        // known here, and a bar that only appeared once the thread had been
        // scheduled would flicker in on the first frame. So it starts correct.
        let progress = Arc::new(Mutex::new(MediaProgress {
            total: plan.loads(),
            ..MediaProgress::default()
        }));
        let handle = std::thread::Builder::new()
            // Named so it is obvious in a debugger and in `top` which thread
            // the boot is waiting on, the same way `movie-decode` is.
            .name("boot-media".to_string())
            .spawn({
                let progress = Arc::clone(&progress);
                move || load_media(archives, &screens, &plan, &options, &progress)
            })
            .expect("spawning the boot's media thread");
        Self {
            handle: Some(handle),
            progress,
        }
    }

    /// Whether the movies have arrived, so the loading screen may start fading.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.handle
            .as_ref()
            .is_none_or(std::thread::JoinHandle::is_finished)
    }

    /// How far the movies have got, right now.
    #[must_use]
    pub fn progress(&self) -> MediaProgress {
        lock_media(&self.progress).clone()
    }

    /// Waits for the movies and takes them.
    ///
    /// **A worker that panicked yields a boot with no movies rather than a
    /// panic here**, because the alternative is a loading screen that never
    /// fades: the sequence copes with `None` on every one of these fields
    /// already - that is what a source with no backdrop, or a machine with no
    /// `ffmpeg`, has always produced - so the game reaches its menus and says
    /// what happened. A second call yields the same empty `Media`, which
    /// cannot happen today and would otherwise be a second panic.
    pub fn join(&mut self) -> Media {
        let Some(handle) = self.handle.take() else {
            return Self::nothing("the boot's movies were already taken");
        };
        match handle.join() {
            Ok(media) => media,
            Err(_) => Self::nothing(
                "the thread loading the movies panicked, so the sequence plays with no picture",
            ),
        }
    }

    fn nothing(why: &str) -> Media {
        Media {
            report: vec![why.to_string()],
            ..Media::default()
        }
    }
}

/// Builds the sequence out of the two halves.
///
/// Everything here needs a frame count or a plane size, which is why it is a
/// third phase rather than the tail of either: the front end cannot be
/// constructed until the movies have been measured.
#[must_use]
pub fn assemble(shell: Shell, media: Media) -> Boot {
    let Shell {
        screens,
        entries,
        loading: _,
        languages,
        strings,
        tracks,
        circuit_names: _,
        teams,
        font,
        sprites,
        space,
        profile,
        menu_skin,
        frame,
        menu_font,
        walked,
        movie_name: _,
        second_movie_name: _,
        leg: _,
        mut report,
    } = shell;
    let Media {
        movie,
        movie_sound,
        backdrop,
        after_language_movie,
        after_language_movie_sound,
        report: _,
    } = media;
    // The picker inside the sequence and the menus' own row are two lists, and
    // the sequence takes one of them by value. See `Boot::languages`.
    let offered = languages.clone();

    let placements = screens
        .screens
        .iter()
        .flat_map(|s| s.images.iter())
        .filter_map(|image| sprites.get(&image.src).map(|p| (image.src.clone(), p)))
        .collect();
    // Both movies described the same way, each from its own container: the cached
    // frame count when there is a cache and the demuxed one when there is not,
    // because `--no-video` and a missing `ffmpeg` still have to play a leg out
    // over its movie's real duration rather than the reel's.
    let plan = |movie: Option<&Movie>| {
        movie.map_or(
            crate::frontend::MoviePlan::none((space.size.0 as u32, space.size.1 as u32)),
            |movie| crate::frontend::MoviePlan {
                frames: movie.frames.as_ref().map_or(movie.frame_count, |f| f.len),
                frame_rate: movie.frame_rate,
                aspect: movie.display_aspect,
                has_picture: movie.frames.is_some(),
            },
        )
    };
    // Each walked step with whichever movie was read for it, matched by position:
    // at most two steps in either title's chain name a movie, and the two the
    // media phase read are those two in order.
    let playing: Vec<&'static str> = walked
        .iter()
        .filter(|step| step.movie.is_some())
        .map(|step| step.state)
        .collect();
    let steps: Vec<crate::frontend::Step> = walked
        .iter()
        .map(|step| crate::frontend::Step {
            state: step.state,
            movie: if playing.first() == Some(&step.state) {
                plan(movie.as_ref())
            } else if playing.get(1) == Some(&step.state) {
                plan(after_language_movie.as_ref())
            } else {
                plan(None)
            },
        })
        .collect();
    let mut frontend = Frontend::booting(
        crate::frontend::Sequence {
            steps,
            backdrop_parent: profile.picker_backdrop_parent,
        },
        screens,
        strings.clone(),
        languages,
        placements,
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

    // The disc's own next-after-the-picker against this build's, so the
    // difference is visible at startup rather than buried. Both halves are
    // resolved rather than named: the target from the sequence itself, and
    // whether a movie ran first from whether the *boot screen* played one. This
    // line used to assert `Show Logo` and "having played it first" on every
    // source, which was two false claims at once on a title whose picker comes
    // first and whose boot screen is silent.
    if let Some(goto) = frontend.language_auto_redirect() {
        let target = frontend.language_confirm_target();
        let order = if walked.first().is_some_and(|step| step.movie.is_some()) {
            ", having played the boot movie first"
        } else {
            ", with no movie before it"
        };
        if goto == target {
            report.push(format!(
                "the disc's Language Selection redirects to {goto}, and so does this build{order}"
            ));
        } else {
            report.push(format!(
                "the disc's Language Selection redirects to {goto}; this build goes to \
                 {target}{order}"
            ));
        }
    }

    Boot {
        entries,
        menu_skin,
        frame,
        menu_font,
        languages: offered,
        strings,
        tracks,
        teams,
        font,
        frontend,
        movie,
        movie_sound,
        backdrop,
        after_language_movie,
        after_language_movie_sound,
        sprites,
        report,
    }
}

/// Whether two movies can share one set of I420 planes.
///
/// The front end is drawn by one renderer with one plane set, and
/// [`crate::render::Renderer::upload_frame`] slices a frame by *those*
/// dimensions rather than by the frame's own - so handing it a
/// differently-shaped picture is a garbled image rather than an error.
///
/// The display aspect is deliberately **not** compared: that differs per movie on
/// the PS2 and is handled by the pillarbox rect, not by the plane size.
///
/// **Unknown geometry is not disagreeing geometry.** `VideoFormat::of` answers
/// `None` for a movie with no decoded frames - `--no-video`, no `ffmpeg`, a
/// failed transcode - and there are then no planes to conflict over, because
/// nothing will be uploaded. Reading that as a mismatch dropped the second movie
/// on every `--no-video` load, which is the configuration the ground-truth tests
/// use.
fn same_planes(a: &Movie, b: &Movie) -> bool {
    match (
        crate::render::VideoFormat::of(a),
        crate::render::VideoFormat::of(b),
    ) {
        (Some(a), Some(b)) => {
            (a.width, a.height, a.chroma_width, a.chroma_height)
                == (b.width, b.height, b.chroma_width, b.chroma_height)
        }
        _ => true,
    }
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
        .flat_map(|screen| screen.movies.iter())
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
    name: &str,
    options: &Options,
    report: &mut Vec<String>,
    watch: crate::movie::Watch<'_>,
) -> Option<Movie> {
    if options.no_video {
        return None;
    }
    let wanted = Options {
        extent: Extent::Whole,
        ..options.clone()
    };
    match load_movie(archives, name, &wanted, report, watch) {
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
fn load_second_movie(
    archives: &mut oag_assets::Archives,
    name: &str,
    options: &Options,
    report: &mut Vec<String>,
    watch: crate::movie::Watch<'_>,
) -> Option<Movie> {
    let wanted = Options {
        extent: Extent::Whole,
        ..options.clone()
    };
    match load_movie(archives, name, &wanted, report, watch) {
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
/// One line saying what a title's menu frame came out as.
///
/// The colour is the point: it is the only thing in the frame that a *different
/// archive* would have made different, so a report that names it is what tells
/// a reader which FE style they are looking at without opening the disc.
fn describe_frame(frame: &crate::menu::Frame) -> String {
    let hex = |color: [f32; 4]| {
        let byte = |channel: f32| (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!(
            "#{:02x}{:02x}{:02x}",
            byte(color[0]),
            byte(color[1]),
            byte(color[2])
        )
    };
    let clear = match &frame.clear {
        Some(crate::frontend::Draw::Fill { color, .. }) => format!("clears to {}", hex(*color)),
        _ => "no clear".to_string(),
    };
    let ink = frame.ink.map_or_else(
        || String::from("no single ink"),
        |ink| format!("ink {}", hex(ink)),
    );
    format!("{clear}, {} mark(s), {ink}", frame.marks.len())
}

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
        && let Ok(blob) = fe.read_entry(name)
    {
        return Ok(blob);
    }
    oag_pulse::read_image(archives, name)
}

fn load_screens(
    archives: &mut oag_assets::Archives,
    root: &str,
    fallback_globals: &[(&str, &str)],
    report: &mut Vec<String>,
) -> Result<Screens> {
    // The one piece with no degraded form: a front end with no screens is not a
    // front end. The message names the archives searched, because on a source
    // whose front-end root has never been located that is the useful half.
    let blob = archives
        .read_name(root)
        .with_context(|| format!("reading {} out of {}", root, archives.layout.describe()))?;

    // Front-end XML is stored with its element and attribute names shortened
    // through a per-file dictionary. Files that begin `<?xml` are already plain.
    let xml = if fexml::is_fexml(&blob) {
        fexml::expand(&blob).map_err(|e| anyhow::anyhow!("expanding the front-end XML: {e}"))?
    } else {
        String::from_utf8(blob).context("the front-end XML is not text")?
    };

    // This title's own measured stand-ins, not every title's. Pure's table used
    // to be handed to every source on the grounds that `or_insert` made it a
    // no-op on Pulse - true, and true only for as long as no two titles measure a
    // *different* value for one name. Pulse's own table is empty.
    let screens = Screens::from_xml_with_fallback_globals(&xml, fallback_globals);
    report.push(format!(
        "{}: {} screens, {} globals, {} LoadXML includes",
        root,
        screens.screens.len(),
        screens.globals.len(),
        screens.load_xml.len()
    ));
    for screen in screens.with_movies() {
        // One line per widget. A screen with four of them used to report one,
        // whichever the parser happened to keep last, which made the other three
        // look absent from the disc.
        for movie in &screen.movies {
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
    plugins: &[&str],
    report: &mut Vec<String>,
) -> Vec<Language> {
    let mut out = Vec::new();
    for plugin in plugins {
        // **Pulse's path convention, applied to every title.** It is genuinely
        // shared today - HD's plugins resolve through it - but it lives in the
        // Pulse crate rather than on `oag_title`, so a title that keeps its
        // plugins elsewhere would load zero languages. That used to happen
        // *silently*, one `continue` per miss (finding G4 of the 2026-08-18
        // review); each miss now names the entry it asked for, so the shape of
        // the failure is legible from the load report rather than only from an
        // empty picker. Promoting the convention to an axis waits for the title
        // that disagrees, which is ADR-0022's rule and the same call S6 makes.
        let name = pulse::names::language_definition(plugin);
        let Ok(blob) = archives.read_name(&name) else {
            report.push(format!(
                "language plugin {plugin}: no {name} in this source"
            ));
            continue;
        };
        let Ok(xml) = expand(&blob) else {
            report.push(format!(
                "language plugin {plugin}: {name} is not readable XML"
            ));
            continue;
        };
        match Language::from_definition(plugin, &xml) {
            Some(language) => out.push(language),
            None => report.push(format!(
                "language plugin {plugin}: {name} declares no language"
            )),
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

/// Which language a boot reads its text in.
///
/// The saved language first, then English, then whatever comes first. The
/// fallback chain used to end at English with a note that there was nothing
/// saved to prefer; there is now. A saved name this source does not carry falls
/// through rather than failing - the same rule the picker's own preselection
/// follows, and for the same reason: a settings file written against the EU
/// disc must not stop the USA one booting.
///
/// Its own function so that [`load_strings`] and
/// [`roster::load_circuit_names`] cannot answer it differently and put half the
/// front end in one language and the circuit list in another.
#[must_use]
pub fn chosen_language<'a>(
    languages: &'a [Language],
    preferred: Option<&str>,
) -> Option<&'a Language> {
    preferred
        .and_then(|name| languages.iter().find(|l| l.name.eq_ignore_ascii_case(name)))
        .or_else(|| languages.iter().find(|l| l.name == "English"))
        .or_else(|| languages.first())
}

/// The chosen language's string table.
///
/// Public for the same reason [`load_languages`] is: the HUD resolves its own
/// `idstring` keys through this (`IG_HUD_LAP` on Pulse, `HUD_Lap` on Pure),
/// and a race reaches it without booting the front end.
pub fn load_strings(
    archives: &mut oag_assets::Archives,
    languages: &[Language],
    preferred: Option<&str>,
    report: &mut Vec<String>,
) -> StringTable {
    let Some(language) = chosen_language(languages, preferred) else {
        return StringTable::default();
    };
    // No `Dynamic Entry File Source` does not always mean no strings: Pure's
    // `PI000` (English) states every string inline in `Definition.xml`
    // instead of naming a separate file - see `docs/formats/pure-status.md`.
    // Re-parsing the definition is safe for a plugin that really names
    // nothing too: `<Font>`/`<Values>` carry no `<Entry>` tag.
    let name = match language.entries.as_deref() {
        Some(entries) => entries.to_string(),
        None => pulse::names::language_definition(&language.plugin),
    };

    match archives
        .read_name(&name)
        .and_then(|blob| expand(&blob).map_err(|e| oag_assets::Error::BadSpec(e.to_string())))
    {
        Ok(xml) => {
            let table = StringTable::from_xml(&xml);
            if table.is_empty() {
                report.push(format!("{} names no string table", language.name));
            } else {
                report.push(format!(
                    "{name}: {} strings for {}",
                    table.len(),
                    language.name
                ));
            }
            table
        }
        Err(e) => {
            report.push(format!("{name}: {e}"));
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
pub const DEVPUB_REEL: &str = pulse::names::DEVPUB_REEL;

mod fonts;
mod movies;
mod roster;
pub(crate) mod xml;

use fonts::{load_font, load_menu_font};
pub use movies::EntryRef;
use movies::load_movie;
use roster::{definitions, load_circuit_names, load_teams, load_tracks};
use xml::expand;

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
mod tests;
