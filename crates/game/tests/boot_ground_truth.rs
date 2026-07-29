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
//! - the whole sequence runs from boot to `Launch Game` with no picture at all.
//!
//! It runs with `no_video`, so it never invokes `ffmpeg` and never writes a
//! cache. What it is testing is the sequencing and the data, not the transcode.

use std::path::{Path, PathBuf};

use oag_assets::pulse;
use oag_game::boot;
use oag_game::frontend::states;
use oag_game::input::{Input, button};

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn load() -> Option<boot::Boot> {
    load_leg(oag_game::frontend::Leg::LogoFmv, boot::DEFAULT_BOOT_MOVIE)
}

fn load_leg(leg: oag_game::frontend::Leg, movie: &str) -> Option<boot::Boot> {
    let image = image()?;
    let options = boot::Options {
        // No saved language: these boot a fresh install every time.
        language: None,
        source: image.display().to_string(),
        leg,
        movie: movie.to_string(),
        // Nothing is written and ffmpeg is never run.
        cache: std::env::temp_dir().join("oag-boot-ground-truth"),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
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
    let Some(loaded) = load_leg(oag_game::frontend::Leg::DevPubReel, boot::DEVPUB_REEL) else {
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
        frontend.update(dt, &mut input);
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

    let movies: Vec<String> = screens
        .with_movies()
        .filter_map(|s| s.movie.as_ref().map(|m| m.entry_name()))
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
            ("PI008", "French", "Français"),
            ("PI009", "German", "Deutsch"),
            ("PI010", "Spanish", "Español"),
            ("PI011", "Italian", "Italiano"),
            ("PI012", "English", "English"),
        ],
        "the USA disc's language plugins"
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
        frontend.update(dt, &mut input);
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

    // Move to English, the last of the five, and pick it.
    for _ in 0..4 {
        input.begin_frame(1 << button::DOWN);
        frontend.update(dt, &mut input);
        input.begin_frame(0);
        frontend.update(dt, &mut input);
    }
    assert_eq!(frontend.selected(), 4);

    input.begin_frame(1 << button::CROSS);
    frontend.update(dt, &mut input);
    assert_eq!(frontend.chosen(), Some("English"));
    assert!(frontend.is_finished());
    assert!(frontend.machine().is(states::LAUNCH_GAME));
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_picker_draws_every_language_in_its_own_name() {
    use oag_game::frontend::Draw;

    let Some(loaded) = load() else { return };
    let mut frontend = loaded.frontend;
    let mut input = Input::new();

    // START skips straight there.
    input.begin_frame(1 << button::START);
    frontend.update(1.0 / 60.0, &mut input);
    input.begin_frame(0);
    frontend.update(1.0 / 60.0, &mut input);
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
