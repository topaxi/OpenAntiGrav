//! Loading the boot sequence out of a disc image.
//!
//! Everything the front end needs, in the order the original needs it: the
//! front-end root XML, the language plugins, one language's string table, and
//! the intro movie. Kept apart from `main.rs` so the load can be exercised
//! without a window, and apart from [`oag_ui::frontend`] so the sequence itself
//! stays free of file I/O.

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use oag_pulse as pulse;

use oag_source::title::open_source;

use crate::movie::{self, Extent, Movie};
use oag_ui::frontend::Frontend;
use oag_ui::language::{Language, StringTable};
use oag_ui::screen::Screens;

/// A loaded boot sequence and the pieces the renderer needs alongside it.
pub struct Boot {
    /// Which title this is, carried from [`Shell::title`] by [`assemble`]
    /// rather than re-derived. `--menu-page` reads its `name` to resolve
    /// `crate::settings::menu_seeds`' five per-title render-profile rows.
    pub title: &'static oag_title::Title,
    /// Which console this source is for, carried from [`Shell::platform`].
    pub platform: oag_disc::Platform,
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
    pub after_language_movie_sound: Option<oag_music::at3::Pcm>,
    /// Every front-end image the screens reference, in one texture.
    pub sprites: oag_hud::sprite::Sheet,
    /// How this title lays its menus out and colours them. See
    /// [`Shell::menu_skin`].
    pub menu_skin: &'static oag_title::MenuSkin,
    /// The frame its menus are drawn inside. See [`Shell::frame`].
    pub frame: oag_ui::menu::Frame,
    /// Its `Confirm`/`Back` legend - see [`Shell::nav_legend`].
    pub nav_legend: Option<oag_ui_screens::campaign::footer::NavigationLegend>,
    /// Its footer's scrolling tip ticker layout - see [`Shell::ticker`].
    pub ticker: Option<oag_ui_screens::campaign::footer::TickerLayout>,
    /// The style's animated menu backdrop - see [`backdrop::load`]. `None` where none.
    pub fury_backdrop: Option<Arc<backdrop::MenuBackdrop>>,
    /// The race box's selection screens. See [`Shell::track_select`].
    pub track_select: Option<oag_ui_screens::picker::Layout>,
    pub ship_select: Option<oag_ui_screens::picker::Layout>,
    /// The `KILLS` and `WEAPONS` rows' values - see [`Shell::race_setup`].
    pub race_setup: RaceSetup,
    /// The circuit names, for a capture's circuit labels.
    pub circuit_names: oag_ui::language::CircuitNames,
    /// The row face and the title's own - see [`Shell::menu_font`]/[`Shell::title_font`].
    pub menu_font: Option<oag_ui::font::Atlas>,
    /// See [`Shell::face_scales`].
    pub face_scales: Vec<(String, f32)>,
    pub title_font: Option<oag_ui::font::Atlas>,
    /// The PlayStation button-glyph face, when this source's language
    /// plugins name one - see [`fonts::load_buttons_font`] and
    /// [`crate::render::Renderer::set_buttons_atlas`].
    pub buttons_font: Option<oag_ui::font::Atlas>,
    /// The text atlas: the disc's own font when it decodes, ours when it does
    /// not.
    pub font: oag_ui::font::Atlas,
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
    /// pack adds. See [`oag_raceplay::catalogue`].
    pub tracks: Vec<oag_raceplay::catalogue::Track>,
    /// Every team this source offers, plus every one a mounted pack adds, in
    /// the order the definitions declare them.
    pub teams: Vec<oag_raceplay::catalogue::Team>,
    /// The intro movie's own sound, decoded and ready to play.
    ///
    /// `None` whenever the movie should be silent, and every reason funnels
    /// into that one word rather than being re-decided downstream: a movie with
    /// no audio stream, a `Movie` widget carrying `sound="false"`, an
    /// `ffmpeg` that is missing or failed, or `--no-video`, which never reads
    /// the movie at all. See [`load_movie_sound`].
    pub movie_sound: Option<oag_music::at3::Pcm>,
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

pub use options::Options;

/// Loads everything and builds the sequence.
///
/// **The blocking whole**, for callers with nothing to draw while they wait: a
/// headless capture, the ground-truth tests. A window uses the three phases
/// this is made of - [`load_shell`], [`load_media`] and [`assemble`] - so that
/// it can put the loading screen up between them. See [`Shell`] for why the
/// line falls where it does.
pub fn load(options: &Options) -> Result<Boot> {
    let (mut shell, archives, _title) = load_shell(options)?;
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
    /// Which title this is, settled by the serial before any XML was parsed.
    /// Also returned alongside this struct by [`load_shell`] for a caller
    /// that needs it before a `Shell` exists at all; carried here too so
    /// [`assemble`] does not have to be handed a third, easy-to-mismatch
    /// argument for a value this struct was already built from.
    pub title: &'static oag_title::Title,
    /// Which console this source is for - see `crate::settings::profile_key`.
    pub platform: oag_disc::Platform,
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
    pub tracks: Vec<oag_raceplay::catalogue::Track>,
    /// The circuits a Zone race can be picked from. See [`oag_title::ZoneCircuit::menu_tracks`].
    pub zone_tracks: Vec<oag_raceplay::catalogue::Track>,
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
    /// does; see [`oag_ui::language::CircuitNames`]. Empty on a source where no
    /// copy names every circuit, which shows each one its id.
    pub circuit_names: oag_ui::language::CircuitNames,
    /// The raceable teams, for the menus.
    pub teams: Vec<oag_raceplay::catalogue::Team>,
    /// The text atlas.
    pub font: oag_ui::font::Atlas,
    /// The front-end sprite sheet.
    pub sprites: oag_hud::sprite::Sheet,
    /// The grid this source authors its widgets in, read off the archives'
    /// own platform while they are still in hand - [`assemble`] has no
    /// archives to ask by the time it needs this.
    pub space: oag_display::space::Space,
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
    /// [`oag_ui::menu::read_frame`].
    ///
    /// Built here rather than by whoever draws, because it needs the parsed
    /// screens *and* the sprite sheet *and* the grid, and this is the only place
    /// all three are in hand. Empty for a title whose frame is unread, which
    /// draws the menus exactly as they were drawn before this existed.
    pub frame: oag_ui::menu::Frame,
    /// Its `Confirm`/`Back` legend - see `screens::read_nav_legend`.
    pub nav_legend: Option<oag_ui_screens::campaign::footer::NavigationLegend>,
    /// Its footer's scrolling tip ticker layout, off the same root -
    /// `None` when this source's front-end root authors no
    /// `TextInfoIsAlwaysLast` viewport, which is every title but Pulse
    /// today. See `screens::read_ticker`. Carries no content of its own:
    /// `MenuStage`'s own per-tick clock supplies which tip is showing, off
    /// `oag_game::records::ticker_tips` - the same rotation the Race
    /// Campaign's own footer already reads, so a live session's two footers
    /// cannot disagree about what a save file honestly has to show.
    pub ticker: Option<oag_ui_screens::campaign::footer::TickerLayout>,
    /// The style's menu backdrop, read here for the same reason the frame is.
    pub fury_backdrop: Option<Arc<backdrop::MenuBackdrop>>,
    /// The race box's two selection screens, read off the same XML the
    /// frame was - `Track Creation` and `Team Selection` on Pulse - with
    /// every string resolved. `None` on a title that authors neither, which
    /// launches straight from the RACE page as before. See
    /// [`oag_ui_screens::picker`].
    pub track_select: Option<oag_ui_screens::picker::Layout>,
    pub ship_select: Option<oag_ui_screens::picker::Layout>,
    /// The `KILLS`/`WEAPONS` rows' values, off the disc's own lists.
    pub race_setup: RaceSetup,
    /// The face menu rows are drawn in, when the title names one and it
    /// reads. `None` falls the menus back to [`Self::font`].
    pub menu_font: Option<oag_ui::font::Atlas>,
    pub title_font: Option<oag_ui::font::Atlas>,
    /// The PlayStation button-glyph face (`PS_BUTTONS.fnt` on HD and Omega).
    pub buttons_font: Option<oag_ui::font::Atlas>,
    /// Each font role's face against `Default`, see [`fonts::face_scales`].
    pub face_scales: Vec<(String, f32)>,
    /// Wipeout 2048's campaign map, see [`campaign2048::map_events`]. Empty
    /// on every other title.
    pub campaign_events: Vec<oag_ui::frontend::MapEvent>,
    /// This build's own tiles on 2048's mode grid, see
    /// [`campaign2048::extra_tiles`]. Empty on every other title.
    pub extra_tiles: Vec<oag_ui::frontend::ExtraTile>,
    /// The screens this boot walks, in the title's own order, already filtered
    /// to the ones this pressing carries and this build can drive.
    ///
    /// Resolved in this phase because it depends on [`Self::screens`], and the
    /// media phase needs the answer to know which movies to read. See
    /// [ADR-0023](../../../docs/architecture/adr/0023-boot-sequence-as-title-data.md).
    pub walked: Vec<&'static oag_title::BootStep>,
    /// Which entry the first movie in that chain is, if it has one. `None`
    /// for a boot that plays nothing anywhere - **not** the same question as
    /// "does the boot screen play something": on Pure neither movie is on the
    /// boot step, so this names the reel that plays one screen later.
    pub movie_name: Option<String>,
    /// The chain's second movie, on the same terms.
    pub second_movie_name: Option<&'static str>,
    /// See [`resolve_movie_region`].
    pub movie_region: &'static str,
    /// Which leg the sequence boots into, carried for [`assemble`].
    pub leg: oag_ui::frontend::Leg,
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
            movie_region: self.movie_region,
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
    pub movie_sound: Option<oag_music::at3::Pcm>,
    /// The looping menu backdrop.
    pub backdrop: Option<Movie>,
    /// The chain's second movie, and its own track.
    pub after_language_movie: Option<Movie>,
    pub after_language_movie_sound: Option<oag_music::at3::Pcm>,
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
/// Returns the archives alongside, still open, and the title, both for later.
///
/// # Errors
///
/// A source whose archives will not open, or which carries no front-end XML.
/// Everything else degrades into a report line.
pub fn load_shell(
    options: &Options,
) -> Result<(Shell, oag_assets::Archives, &'static oag_title::Title)> {
    let mut report = Vec::new();
    let mut steps = Steps::new();
    let (packs, pure_packs, problems) = oag_source::dlc::packs_from_defaults(
        &options.dlc,
        &oag_source::cache::default_dlc_cache_dir(),
    );
    let oag_source::title::Opened {
        mut archives,
        title,
    } = open_source(&options.source, packs, pure_packs)
        .with_context(|| format!("opening the archives in {}", options.source))?;
    // Which title this is was settled by the serial, here, and everything below
    // asks `profile` rather than asking the XML again. See ADR-0023.
    // **A title with no recovered front end is still refused by name**, and no
    // title is in that state today. What used to be refused here as well was a
    // front end whose *chain* was only declared - Wipeout HD's - and that is now
    // expressed rather than withheld: see `oag_title::Provenance` and ADR-0025.
    let front_end = title.front_end.ok_or_else(|| {
        anyhow::anyhow!(
            "{}'s front end is not recovered, so there is no sequence to walk: \
             race on it with --race instead",
            title.name
        )
    })?;
    // **A front end with no `MenuSkin`-shaped menu is not refused.** Wipeout
    // 2048 is the one title in this state (ADR-0054): its boot chain walks
    // and its screens are touch-icon grids drawn off `front_end.touch`, so
    // the skin below is only what this build's *own* menus would draw in if
    // they were ever opened on it - `oag_ui::placeholder`'s numbers, which
    // belong to no disc and are never mistaken for a measurement.
    //
    // **A PS2 source prefers `front_end.menu_ps2` over `front_end.menu`.**
    // Only Wipeout Pulse fills the former today - see its own doc for why a
    // title's PSP-authored `MenuSkin` scaled by grid ratio alone is an
    // approximation `oag_ui::menu::Skin::new` already documents as one - and
    // every other title's `None` falls straight through to `front_end.menu`
    // exactly as before this axis existed, so this changes no title but
    // Pulse's PS2 pressing.
    let menu_skin = match archives.layout.platform {
        oag_title::Platform::Ps2 => front_end.menu_ps2.or(front_end.menu),
        _ => front_end.menu,
    }
    .unwrap_or(&oag_ui::placeholder::MENU_SKIN);
    let profile = front_end.boot;
    report.push(archives.layout.describe());
    report.push(format!("{}: boot sequence", title.name));
    report.extend(provenance::caveat(title.name, profile.provenance));
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
    let offered_plugins = front_end.offered_languages(archives.layout.serial.as_deref());
    let languages = load_languages(
        &mut archives,
        offered_plugins,
        front_end.disc_strings,
        &mut report,
    );
    let offered = languages.clone();
    steps.lap("languages");
    // Resolved once and threaded through every font/table read below rather
    // than re-asked with `options.language.as_deref()` at each one: the
    // fonts used to skip this entirely and take whichever plugin's `<Font>`
    // slot came first in source order, which is the same bug
    // `race::hud::hud_font`'s own doc records - see `fonts::role_font`.
    let preferred_language = chosen_language(&languages, options.language.as_deref());
    // Omega's faces are HD's at twice the pixel size; see
    // `oag_display::space::Space::font_texel_scale`. `1.0` for every other source.
    let texel_scale = oag_display::space::Space::font_texel_scale(archives.layout.platform);
    let font = load_font(&mut archives, &languages, preferred_language, &mut report)
        .with_prompts(title.prompts)
        .with_texel_scale(texel_scale);
    steps.lap("font");
    // **`TitleFrame` is appended here, not carried in `profile.fallback_images`
    // itself.** Its own real `src` differs by pressing - `Screen_
    // ConstructTitleScreen` bakes a different, region-suffixed literal into
    // each disc's own executable (`docs/ghidra/functions/psp-pure-eu/
    // title-screen.md`) - and `BootProfile.fallback_images` is one `&'static`
    // table shared by every source a title opens, with no room for that axis.
    // Every other title's own table is empty, so this is additive rather than
    // a behaviour change for them; see `oag_pure::frontend::title_frame_src`'s
    // own doc comment for why the fallback is EU rather than a guess when the
    // serial cannot be read at all.
    let mut fallback_images = profile.fallback_images.to_vec();
    if let Some(pressings) = title.pressings {
        fallback_images.push(pressings.of(archives.layout.serial.as_deref()).title_frame);
    }
    let movie_region = resolve_movie_region(title, archives.layout.serial.as_deref());
    let mut screens = load_screens(
        &mut archives,
        front_end.root,
        title.plugin_definition,
        profile.fallback_globals,
        &fallback_images,
        movie_region,
        &mut report,
    )?;
    // A touch front end's screens are in the root's includes - see
    // `boot::includes` for why only that idiom is followed.
    if front_end.touch.is_some() {
        includes::follow(
            &mut archives,
            front_end.root,
            &mut screens,
            oag_2048::frontend::includes::FOLLOWED,
            oag_2048::frontend::includes::LOCALISED_SUFFIX,
            &mut report,
        );
    }
    steps.lap("screens");
    // The race box's own definition, when the title names one - parsed here,
    // ahead of the sprite sheet, so its images (`hex_bg.mip`, the bars) go
    // into the same sheet the skin's do. Its layouts are read further down,
    // once the strings are in hand.
    let race_box = front_end.race_box.and_then(|name| {
        match load_included_screens(&mut archives, name, &screens) {
            Ok(included) => Some(included),
            Err(error) => {
                report.push(format!("{name}: {error:#} - no selection screens"));
                None
            }
        }
    });
    let team_box = screens::load_team_select(
        &mut archives,
        front_end.team_select,
        preferred_language.and_then(|language| language.entries.as_deref()),
        &screens,
        &mut report,
    );
    let track_box = screens::load_track_select(
        &mut archives,
        front_end.track_select,
        preferred_language.and_then(|language| language.entries.as_deref()),
        &screens,
        &mut report,
    );
    let race_setup = screens::read_race_setup(&mut archives, front_end.race_setup, &mut report);
    let strings = load_strings(
        &mut archives,
        &languages,
        options.language.as_deref(),
        &mut report,
    );
    steps.lap("strings");
    let (menu_font, title_font, buttons_font) = fonts::load_role_fonts(
        &mut archives,
        &languages,
        preferred_language,
        menu_skin,
        texel_scale,
        title.prompts,
        &mut report,
    );
    // A touch front end's screens author more than one role and this build
    // has one atlas; the ratio each face's line height stands to `Default`
    // is what keeps the others the right size. Not measured for the other
    // titles here, whose screens were matched against captures without it.
    let face_scales = if front_end.touch.is_some() {
        fonts::face_scales(
            &mut archives,
            preferred_language,
            font.line_height,
            &mut report,
        )
    } else {
        Vec::new()
    };
    steps.lap("menu and title fonts");
    // The map the touch front end's campaign tile leads to, and the two
    // tiles this build adds beside the authored four.
    let (campaign_events, extra_tiles) = if front_end.touch.is_some() {
        (
            campaign2048::map_events(&mut archives, &strings, &mut report),
            campaign2048::extra_tiles(options.language.as_deref()),
        )
    } else {
        (Vec::new(), Vec::new())
    };
    steps.lap("campaign map");
    let definition = title.plugin_definition;
    let documents = definitions(
        &mut archives,
        &[
            definition,
            title.track_plugin_definition.unwrap_or(definition),
        ],
        &mut report,
    );
    let tracks = load_tracks(&mut archives, definition, &documents, &mut report);
    let zone_tracks = load_zone_tracks(&mut archives, title.race.zone, &documents, &mut report);
    let teams = load_teams(
        &mut archives,
        (title.race.ship_dir, title.race.handling_dir),
        definition,
        title.race.team,
        &documents,
        &mut report,
    );
    // After the circuits, because which copy of the string table names them all
    // is a question about the list this just produced.
    let circuit_names = load_circuit_names(
        &mut archives,
        preferred_language,
        &strings,
        &tracks,
        &mut report,
    );
    steps.lap("catalogue");
    // The menu blocks' three textures are the executable's own, named by no
    // screen - see `oag_title::MenuBlocks` - so they are asked for by name
    // alongside everything the screens name.
    let mut block_textures: Vec<&str> = menu_skin
        .blocks
        .map(|blocks| {
            vec![
                blocks.frame_texture,
                blocks.cursor_texture,
                blocks.arrow_texture,
            ]
        })
        .unwrap_or_default();
    // The campaign map's own hex tile art and its four mode icons - named
    // by no screen either, the same "executable draws it, not a widget"
    // shape the menu blocks are already in. See
    // `oag_ui::frontend::campaign_map::HEX_FILLED`'s own doc comment for why
    // these decode but are not authored anywhere in `NEWGUI/*.xml`.
    if front_end.touch.is_some() {
        block_textures.extend([
            oag_ui::frontend::HEX_FILLED,
            oag_ui::frontend::HEX_OUTLINE,
            oag_ui::frontend::HEX_SELECT,
            oag_ui::frontend::EventIcon::Race.texture_name(),
            oag_ui::frontend::EventIcon::SpeedLap.texture_name(),
            oag_ui::frontend::EventIcon::Zone.texture_name(),
            oag_ui::frontend::EventIcon::Elimination.texture_name(),
        ]);
        block_textures.extend(campaign2048::card_textures(&campaign_events));
    }
    let logos = screens::team_logos(team_box.is_some(), &teams);
    block_textures.extend(logos.iter().map(String::as_str));
    let emblems = screens::track_emblems(track_box.is_some(), &tracks);
    block_textures.extend(emblems.iter().map(String::as_str));
    let sprites = sprites::load(
        &mut archives,
        &std::iter::once(&screens)
            .chain(race_box.as_ref())
            .chain(team_box.as_ref().map(|team_box| &team_box.screens))
            .chain(track_box.as_ref().map(|track_box| &track_box.screens))
            .collect::<Vec<_>>(),
        &block_textures,
        front_end.bottom_up_gnf,
        &mut report,
    );
    steps.lap("sprites");
    // Which step the boot opens on, from this title's own chain. `--reel`
    // replaces the boot step rather than preceding it, and is refused by name on
    // a title with no evidenced reel state rather than pointed at another
    // title's screen.
    let start_step = match options.leg {
        oag_ui::frontend::Leg::LogoFmv => profile.start(),
        oag_ui::frontend::Leg::DevPubReel => profile.reel.as_ref().with_context(|| {
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
    let off_path = options.leg == oag_ui::frontend::Leg::DevPubReel;
    if !off_path && screens.by_name(start_step.state).is_none() {
        report.push(format!(
            "the chain opens on {:?}, which this pressing does not carry",
            start_step.state
        ));
    } else if off_path || oag_ui::frontend::can_drive(start_step.state) {
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
        } else if oag_ui::frontend::can_drive(step.state) {
            walked.push(step);
        } else {
            report.push(format!(
                "the disc's {:?} is skipped: this build has no behaviour for it yet",
                step.state
            ));
        }
    }
    // Nothing to walk is a failed boot, said by name: `Frontend::booting`
    // indexes the first step, and a chain whose every screen is missing
    // (a root whose includes did not read) must not get that far.
    anyhow::ensure!(
        !walked.is_empty(),
        "{}: none of the boot chain's {} screens is in the front end this source \
         served, so there is no sequence to walk - {}",
        title.name,
        profile.chain.len(),
        report.join("; ")
    );
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
    let (default_movie_name, second_movie_name) = resolve_pure_movie_region(
        title,
        movie_region,
        playing.first().and_then(|step| step.movie),
        playing.get(1).and_then(|step| step.movie),
    );
    // `--movie` overrides whichever movie the sequence draws first, so the flag
    // stays a preview tool on a title whose own boot screen is silent.
    let movie_name = options
        .movie
        .clone()
        .or_else(|| default_movie_name.map(str::to_string));
    // The grid this source authors in, needed before the front end is built so
    // that a boot with no movie falls back to the *source's* shape rather than
    // to the PSP's. `frontend.set_space` takes the same value.
    let space = oag_display::space::Space::of(archives.layout.platform);
    // The frame the menus are drawn inside, off the screen this title names -
    // built here because this is where the parsed XML, the sheet and the grid
    // are all in hand, and **reported** because which archive served the root
    // decides what colour it comes out in: Wipeout HD's `HD_*` palette is its FE
    // style, black and red in `DATA00` against white and teal in `DATA06`. A
    // menu that looks like the wrong game is then a line in the boot report
    // rather than a mystery. See [`oag_ui::menu::frame`].
    let blocks = sprites::block_art(menu_skin.blocks, &sprites, &screens, &mut report);
    let skin_xml = archives
        .read_name(front_end.root)
        .ok()
        .and_then(|b| expand(&b).ok());
    let fury_backdrop = backdrop::load(
        &mut archives,
        (sprites::fury_style(&screens), profile.menu_scene),
        skin_xml.as_deref(),
        &mut report,
    );
    let mut frame = oag_ui::menu::read_frame(
        &screens,
        sprites.entries(),
        space,
        front_end.menu_frame,
        menu_skin.strip.and_then(|strip| strip.selected_fill),
        blocks,
    );
    if let Some(name) = front_end.menu_frame {
        report.push(format!("menu frame {name}: {}", frame.describe()));
    }
    frame.settings = screens::read_settings_layout(
        &mut archives,
        menu_skin,
        &screens.globals,
        sprites.entries(),
        &mut report,
    );
    // See [`screens::read_nav_legend`]'s own doc.
    let nav_legend = read_nav_legend(skin_xml.as_deref(), &screens.globals, &strings);
    if let Some(legend) = &nav_legend {
        report.push(format!("nav legend: {}", legend.describe()));
    }
    // See [`screens::read_ticker`]'s own doc.
    let ticker = read_ticker(skin_xml.as_deref(), &screens.globals);
    if let Some(ticker) = &ticker {
        report.push(format!("footer ticker: viewport {:?}", ticker.viewport));
    }
    let (track_select, ship_select) = selection_layouts(
        race_box.as_ref(),
        team_box.as_ref(),
        track_box.as_ref(),
        &strings,
        &font,
        menu_font.as_ref(),
        fonts::default_in_own_slot(menu_skin),
        space,
        &mut report,
    );
    report.push(steps.describe("the boot's first half"));

    Ok((
        Shell {
            title,
            platform: archives.layout.platform,
            screens,
            entries: preferred_language.and_then(|language| language.entries.clone()),
            loading: title.loading,
            languages: offered,
            strings,
            tracks,
            zone_tracks,
            circuit_names,
            teams,
            font,
            sprites,
            space,
            profile,
            menu_skin,
            frame,
            nav_legend,
            ticker,
            fury_backdrop,
            track_select,
            ship_select,
            race_setup,
            menu_font,
            title_font,
            buttons_font,
            face_scales,
            campaign_events,
            extra_tiles,
            walked,
            movie_name,
            second_movie_name,
            movie_region,
            leg: options.leg,
            report,
        },
        archives,
        title,
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
    let region = plan.movie_region;
    let movie_sound = match &plan.movie_name {
        Some(name) => {
            starting(progress, &format!("{name} (sound)"));
            let sound =
                load_movie_sound(movie.as_ref(), screens, name, region, options, &mut report);
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
                region,
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
    /// See [`Shell::movie_region`].
    pub movie_region: &'static str,
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
    #[cfg(not(target_arch = "wasm32"))]
    handle: Option<std::thread::JoinHandle<Media>>,
    /// The web build has no threads, so [`Self::spawn`] loads the media
    /// inline and the result waits here. See docs/tools/web.md.
    #[cfg(target_arch = "wasm32")]
    handle: Option<Media>,
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
        #[cfg(target_arch = "wasm32")]
        let handle = load_media(archives, &screens, &plan, &options, &progress);
        #[cfg(not(target_arch = "wasm32"))]
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
        #[cfg(target_arch = "wasm32")]
        return true;
        #[cfg(not(target_arch = "wasm32"))]
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
        #[cfg(target_arch = "wasm32")]
        return handle;
        #[cfg(not(target_arch = "wasm32"))]
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
        title,
        platform,
        screens,
        entries,
        loading: _,
        languages,
        strings,
        tracks,
        zone_tracks: _,
        circuit_names,
        teams,
        font,
        sprites,
        space,
        profile,
        menu_skin,
        frame,
        nav_legend,
        ticker,
        fury_backdrop,
        track_select,
        ship_select,
        race_setup,
        menu_font,
        title_font,
        buttons_font,
        face_scales,
        campaign_events,
        extra_tiles,
        walked,
        movie_name: _,
        second_movie_name: _,
        movie_region: _,
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

    // Every image `sprites` actually decoded - every `Image`/`TouchButton`
    // icon a screen spells, **plus** the menu blocks' own nine-patch and the
    // campaign map's hex tile/mode-icon art, neither named by any screen
    // (`sprites::load`'s own `extra` list). `Sheet::entries` is already
    // exactly this set, so reading it directly is both simpler and more
    // complete than the screen-only walk this replaced: that walk never
    // surfaced an `extra` name at all, which stayed harmless while nothing
    // read one back out of `Frontend::placements` - `sprites::block_art`
    // gets its own `BlockArt` straight from `sprites` instead - until
    // `oag_ui::frontend::campaign_map::Frontend::placed` became the first
    // caller that does (2026-09-27).
    let placements = sprites.entries().to_vec();
    // Both movies described the same way, each from its own container: the cached
    // frame count when there is a cache and the demuxed one when there is not,
    // because `--no-video` and a missing `ffmpeg` still have to play a leg out
    // over its movie's real duration rather than the reel's.
    let plan = |movie: Option<&Movie>| {
        movie.map_or(
            oag_ui::frontend::MoviePlan::none((space.size.0 as u32, space.size.1 as u32)),
            |movie| oag_ui::frontend::MoviePlan {
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
    let steps: Vec<oag_ui::frontend::Step> = walked
        .iter()
        .map(|step| oag_ui::frontend::Step {
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
        oag_ui::frontend::Sequence {
            steps,
            backdrop_parent: profile.picker_backdrop_parent,
        },
        screens,
        strings.clone(),
        languages,
        placements,
    );
    frontend.set_menu_skin(menu_skin);
    // Only where they were measured - see `load_shell`'s own gate.
    if !face_scales.is_empty() {
        frontend.set_face_scales(face_scales.clone(), font.line_height);
    }
    if profile.picker_from_loaded_faces {
        frontend.set_picker_line_height(font.line_height);
    }
    if !campaign_events.is_empty() {
        frontend.set_campaign(campaign_events);
    }
    if !extra_tiles.is_empty() {
        frontend.set_extra_tiles(extra_tiles);
    }

    // **Before `set_backdrop`, which bakes a rect out of it.** The PS2's `Skin.xml`
    // places widgets in a 640x448 grid rather than the PSP's 480x272, so every widget
    // on that disc landed off the bottom-right of a screen a third too small, leaving
    // `Show Logo` blank. Set from the archives' own platform, not sniffed - the layout
    // resolved one to open them. See `oag_display::space::Space`.
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
    // the PS2's `INTRO640.PSS` and `BG640.IPF` are both 640x448. What differs on
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
        title,
        platform,
        entries,
        menu_skin,
        frame,
        nav_legend,
        ticker,
        fury_backdrop,
        track_select,
        ship_select,
        race_setup,
        circuit_names,
        menu_font,
        face_scales,
        title_font,
        buttons_font,
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
    movie_region: &str,
    options: &Options,
    report: &mut Vec<String>,
) -> Option<oag_music::at3::Pcm> {
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
        .any(|widget| {
            widget
                .entry_name(movie_region)
                .eq_ignore_ascii_case(movie_name)
                && !widget.sound
        });
    if silent {
        report.push(format!(
            "  audio: {} channel(s) at {} Hz, muted - the widget playing it is sound=\"false\"",
            audio.channels(),
            audio.sample_rate()
        ));
        return None;
    }

    match audio.decode(&options.audio_cache) {
        Ok(pcm) => {
            report.push(format!(
                "  audio: {} channel(s) at {} Hz, {}, decoded to {:.2}s",
                pcm.channels,
                pcm.sample_rate,
                audio.codec_clause(),
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
/// of that work, done alongside this function: see `oag_ui::frontend::Frontend`
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

pub mod backdrop;
pub(crate) mod campaign2048;
pub mod fonts;
pub mod fury;
mod images;
mod includes;
mod movies;
mod options;
mod progress;
mod provenance;
pub(crate) mod roster;
pub mod scene;
mod screens;
pub use screens::RaceSetup;
pub(crate) mod sprites;
mod steps;

use fonts::load_font;
pub use movies::{DEFAULT_BOOT_MOVIE, DEVPUB_REEL, EntryRef};
use movies::{load_movie, resolve_movie_region, resolve_pure_movie_region};
use oag_ui::language::load::{chosen_language, load_languages, load_strings};
use oag_ui::xml::expand;
pub use progress::MediaProgress;
use progress::{loaded, lock_media, starting, watching};
use roster::{definitions, load_circuit_names, load_teams, load_tracks, load_zone_tracks};
use screens::{
    load_included_screens, load_screens, read_nav_legend, read_ticker, selection_layouts,
};
use steps::Steps;

#[cfg(test)]
mod tests;
