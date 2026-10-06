//! Validates the boot sequence against a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! Every claim in `docs/architecture/frontend-boot.md` that came from reading a
//! real disc is asserted here, so the page cannot quietly go stale:
//!
//! - the front-end root XML is at `Data\Plugins\PI001\GUI\Skin.xml`;
//! - its `Movie` widget's `src` plus `.PMF` names an entry that exists;
//! - the disc offers five languages, each naming itself;
//! - `Language Selection` has a `DisplayLanguages` widget and redirects to
//!   `LogoFMV`;
//! - the whole sequence runs from boot to `Launch Game` with no picture at all,
//!   through `Show Logo`, which waits for START and has no timeout.
//!
//! It runs with `no_video`, so it never invokes `ffmpeg` and never writes a
//! cache. What it is testing is the sequencing and the data, not the transcode.

use std::path::PathBuf;

use oag_game::boot;
use oag_game::input::{Button, Input};
use oag_pulse as pulse;
use oag_ui::frontend::states;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn load() -> Option<boot::Boot> {
    load_leg(oag_ui::frontend::Leg::LogoFmv, boot::DEFAULT_BOOT_MOVIE)
}

fn load_leg(leg: oag_ui::frontend::Leg, movie: &str) -> Option<boot::Boot> {
    let image = image()?;
    let options = boot::Options {
        // No saved language: these boot a fresh install every time.
        language: None,
        source: image.display().to_string(),
        // And no downloadable content: what is asserted below is what the disc
        // alone offers, which is the thing that must not move when a pack is
        // mounted elsewhere.
        dlc: Vec::new(),
        leg,
        movie: Some(movie.to_string()),
        // Nothing is written and ffmpeg is never run.
        cache: std::env::temp_dir().join("oag-boot-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    Some(boot::load(&options).expect("loading the boot sequence"))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_default_boot_opens_the_movie_the_disc_opens() {
    // The regression guard for "launching the game plays what looks like Wipeout
    // Pure": the boot used to default to one of the three 260-frame dev/pub
    // reels, which carry no Pulse branding and ship byte-identically on Pure's
    // own disc. What the disc opens - and all it opens, over ten minutes of a
    // cold boot with `MoviePlayer_Open` armed - is this.
    let Some(loaded) = load() else { return };

    let named = loaded
        .report
        .iter()
        .any(|line| line.starts_with(&format!("{}: ", pulse::names::INTRO_MOVIE)));
    assert!(
        named,
        "the boot report must say it opened {}; got {:#?}",
        pulse::names::INTRO_MOVIE,
        loaded.report
    );

    let movie = loaded.movie.expect("the PSP disc has the movie");
    assert_eq!(movie.frame_count, 1200, "the 40-second Pulse showcase");
    assert_eq!((movie.width, movie.height), (480, 272));
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_reel_leg_still_runs_its_frame_holds() {
    // `--reel`. The state is real and evidenced - its `OnEnter` caches
    // `"DevPubRedirect"` at `0x088d7d80` - but the disc's boot never enters it,
    // so this is the only thing keeping the path from rotting.
    let Some(loaded) = load_leg(oag_ui::frontend::Leg::DevPubReel, boot::DEVPUB_REEL) else {
        return;
    };
    let mut frontend = loaded.frontend;
    let mut input = Input::new();
    let dt = 1.0 / 60.0;

    assert_eq!(frontend.machine().current(), Some(states::INTRO_MOVIE));

    let mut held_at = Vec::new();
    for _ in 0..3600 {
        if frontend.machine().is(states::LANGUAGE_SELECTION) {
            break;
        }
        input.begin_frame(0);
        frontend.update(dt, &mut input, None);
        if frontend.player().is_paused() {
            let frame = frontend.player().frames_produced();
            if !held_at.contains(&frame) {
                held_at.push(frame);
            }
        }
    }

    assert_eq!(held_at, [144, 231, 260], "the holds at 0x088d7e1c");
    assert_eq!(
        frontend.machine().history(),
        [
            states::INTRO,
            states::INTRO_MOVIE,
            states::DEV_PUB_REDIRECT,
            states::LANGUAGE_SELECTION,
        ]
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_front_end_root_names_the_intro_movie() {
    let Some(loaded) = load() else { return };
    let screens = loaded.frontend.screens();

    // Pulse's own widgets are never `localised`, so the region argument is
    // never consulted here.
    let movies: Vec<String> = screens
        .with_movies()
        .flat_map(|s| {
            s.movies
                .iter()
                .map(|m| m.entry_name(oag_ui::screen::DEFAULT_REGION))
        })
        .collect();

    assert!(
        movies.contains(&pulse::names::INTRO_MOVIE.to_string()),
        "the front-end XML must name the intro movie; got {movies:?}"
    );
    assert!(
        movies.contains(&pulse::names::BACKDROP_MOVIE.to_string()),
        "and the menu backdrop; got {movies:?}"
    );

    // `LogoFMV` and `Play Intro` both play the intro; `FE Screen` loops the
    // backdrop. Three movie widgets, two distinct files.
    assert_eq!(screens.with_movies().count(), 3, "{movies:?}");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_backdrop_and_the_intro_share_one_plane_geometry() {
    // What lets `Show Logo` sit on the moving backdrop at all. The front end is
    // drawn by one renderer with one set of I420 planes, sized once from the
    // intro, and `upload_frame` slices by *those* dimensions rather than by the
    // frame's own - so `boot::load` only hands the sequence a backdrop when the
    // two agree, and on the PSP they do. If this ever fails the guard fires and
    // `Show Logo` quietly loses its backdrop; if the guard were removed it would
    // draw a garbled picture rather than error, which is why it exists.
    // (The PS2's pair agree too, at 512x512 - see the notes in `boot::load`.)
    let Some(intro) = load() else { return };
    let Some(backdrop) = load_leg(oag_ui::frontend::Leg::LogoFmv, pulse::names::BACKDROP_MOVIE)
    else {
        return;
    };

    let intro = intro.movie.expect("the PSP disc has the intro");
    let backdrop = backdrop.movie.expect("the PSP disc has the backdrop");

    assert_eq!((intro.width, intro.height), (480, 272));
    assert_eq!(
        (backdrop.width, backdrop.height),
        (intro.width, intro.height),
        "both are 480x272 .PMFs, which is what makes one video pipeline enough"
    );
    assert_eq!(backdrop.frame_count, 270, "9 seconds of looping backdrop");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_disc_offers_five_languages_each_naming_itself() {
    let Some(loaded) = load() else { return };
    let languages = loaded.frontend.languages();

    let found: Vec<(&str, &str, &str)> = languages
        .iter()
        .map(|l| (l.plugin.as_str(), l.name.as_str(), l.native_name.as_str()))
        .collect();

    assert_eq!(
        found,
        [
            ("PI012", "English", "English"),
            ("PI010", "Spanish", "Español"),
            ("PI008", "French", "Français"),
            ("PI009", "German", "Deutsch"),
            ("PI011", "Italian", "Italiano"),
            // The project's own language, after the disc's, standing on the
            // disc's English plugin (2026-10-06).
            ("PI012", "PortugueseBR", "Português (Brasil)"),
        ],
        "the USA disc's language plugins, in its executable's manifest order, then the project's"
    );

    for language in languages {
        let entries = language
            .entries
            .as_deref()
            .unwrap_or_else(|| panic!("{} names no string table", language.name));
        assert_eq!(entries, pulse::names::language_entries(&language.plugin));
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_language_screen_is_data_driven() {
    let Some(loaded) = load() else { return };
    let screen = loaded
        .frontend
        .language_screen()
        .expect("the disc must declare a Language Selection screen");

    assert_eq!(screen.name, "Language Selection");
    assert_eq!(screen.kind.as_deref(), Some("Language Selection"));
    assert!(
        screen.display_languages,
        "it is the DisplayLanguages widget that makes it the picker"
    );
    let menu = screen.menu.as_ref().expect("the picker has a Menu widget");
    assert_eq!(menu.name, "Language");
    assert_eq!(menu.x, 50.0, "x came from FEGlobals->MenuXOffset");
    assert_eq!(menu.y, 46.0);
    assert_eq!(menu.scale, 1.0, "scale came from FEGlobals->MenuScale");
    assert_eq!(
        menu.color, 0xff33_a6b9,
        "color came from FEGlobals->TextColor"
    );
    assert_eq!(menu.align, "left");
    assert!(
        !screen.texts.is_empty(),
        "the picker has its own Text widgets"
    );

    // The disc's own order is picker first, intro second. This build runs them
    // the other way round on purpose, and the page says so.
    assert_eq!(loaded.frontend.language_auto_redirect(), Some("LogoFMV"));
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_sequence_runs_from_boot_to_launch_game() {
    let Some(loaded) = load() else { return };
    let mut frontend = loaded.frontend;
    let mut input = Input::new();
    let dt = 1.0 / 60.0;

    assert_eq!(frontend.machine().current(), Some(states::LOGO_FMV));

    // Let the movie play itself out. Sixty seconds of simulated time is well
    // past the forty the intro runs for; `LogoFMV` has no frame holds, because
    // its `Movie` widget has no frame counters.
    for _ in 0..3600 {
        if frontend.machine().is(states::LANGUAGE_SELECTION) {
            break;
        }
        input.begin_frame(0);
        frontend.update(dt, &mut input, None);
    }
    assert!(
        frontend.machine().is(states::LANGUAGE_SELECTION),
        "the intro must end at the picker, got {:?}",
        frontend.machine().current()
    );

    assert_eq!(
        frontend.machine().history(),
        [states::LOGO_FMV, states::LANGUAGE_SELECTION],
        "no DevPubRedirect on this leg: it belongs to Intro Screen->IntroMovie1, \
         which the disc's own boot never enters"
    );

    // English is the manifest's first entry, so it is already selected.
    assert_eq!(frontend.selected(), 0);

    input.begin_frame(Button::Cross.bit());
    frontend.update(dt, &mut input, None);
    assert_eq!(frontend.chosen(), Some("English"));

    // The picker's exit is `Show Logo`, not the menus: the disc's own boot puts
    // the Pulse logo and PRESS START between the front end and everything after
    // it, and this build keeps that screen even though it skips the four Memory
    // Stick screens the disc has on either side of it.
    assert!(
        frontend.machine().is(states::SHOW_LOGO),
        "got as far as {:?}",
        frontend.machine().current()
    );
    assert!(
        !frontend.is_finished(),
        "Show Logo waits: nothing has been pressed through it yet"
    );

    // Its own XML gives it one redirect with a button and that button is START.
    // Ten seconds of doing nothing must not advance it, because nothing on the
    // screen is a timer.
    for _ in 0..600 {
        input.begin_frame(0);
        frontend.update(dt, &mut input, None);
    }
    assert!(
        frontend.machine().is(states::SHOW_LOGO),
        "and it has no timeout"
    );

    input.begin_frame(Button::Start.bit());
    frontend.update(dt, &mut input, None);
    assert!(frontend.is_finished());
    assert!(frontend.machine().is(states::LAUNCH_GAME));

    assert_eq!(
        frontend.machine().history(),
        [
            states::LOGO_FMV,
            states::LANGUAGE_SELECTION,
            states::SHOW_LOGO,
            states::LAUNCH_GAME,
        ]
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_picker_draws_every_language_in_its_own_name() {
    use oag_ui::frontend::Draw;

    let Some(loaded) = load() else { return };
    let mut frontend = loaded.frontend;
    let mut input = Input::new();

    // START skips straight there.
    input.begin_frame(Button::Start.bit());
    frontend.update(1.0 / 60.0, &mut input, None);
    input.begin_frame(0);
    frontend.update(1.0 / 60.0, &mut input, None);
    assert!(frontend.machine().is(states::LANGUAGE_SELECTION));

    let drawn: Vec<String> = frontend
        .draw_list()
        .iter()
        .filter_map(|d| match d {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();

    for expected in ["Français", "Deutsch", "Español", "Italiano", "English"] {
        assert!(
            drawn.iter().any(|t| t == expected),
            "{expected:?} missing from {drawn:?}"
        );
    }
}

/// Which string-table entries name a race mode.
///
/// The menu shows the player a mode name, and that name must come off the disc
/// rather than out of this repository - the same rule the circuit rows already
/// follow. This finds the ids, so the menu can ask for them by name.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_string_table_names_the_race_modes() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("archives");
    let entries = oag_pulse::names::language_entries("PI012");
    let blob = archives
        .read_name(&entries)
        .expect("the English string table");
    let xml = oag_tables::fexml::expand(&blob).expect("expands");

    let wanted = [
        "time trial",
        "speed lap",
        "zone",
        "single race",
        "eliminator",
    ];
    let mut hits = 0;
    for line in xml.split("<Entry").skip(1) {
        let Some(id) = line.split("ID=\"").nth(1).and_then(|r| r.split('"').next()) else {
            continue;
        };
        let Some(text) = line
            .split("String=\"")
            .nth(1)
            .and_then(|r| r.split('"').next())
        else {
            continue;
        };
        let lower = text.to_ascii_lowercase();
        if wanted.iter().any(|w| lower == *w || lower.contains(w)) {
            println!("  {id}  =  {text:?}");
            hits += 1;
        }
    }
    println!("{hits} mode-ish entries in {entries}");
    assert!(hits > 0, "the string table names no race mode at all");
}
