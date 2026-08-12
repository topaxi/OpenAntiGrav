//! Validates Wipeout Pure's boot sequence against its real discs.
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
//! Every claim in `docs/architecture/pure-boot.md` that came from reading a real
//! disc is asserted here, so the page cannot quietly go stale. The load-bearing
//! one is the first: **Pure's boot plays no movie before its picker.** That was
//! got wrong once already, implemented from a stale emulator save profile that
//! was skipping the real boot, and a passing unit test suite did not notice -
//! which is why this file exists at all.
//!
//! # Both pressings, not one
//!
//! `boot_ground_truth.rs` reads a single Pulse image. This one runs everything
//! over **both** Pure pressings, because the movie entry names are USA-shaped by
//! construction: `Movie::entry_name` appends `_US.PMF` for every `localised`
//! widget on every source. Whether the EU disc really carries `_US`-suffixed
//! movies is a measurement, and
//! [`the_two_pressings_name_their_movies_the_same_way`] is the canary that keeps
//! it one - it fails with the name it could not find.
//!
//! It runs with `no_video`, so it never invokes `ffmpeg` and never writes a
//! cache. What is tested is the sequencing and the data, not the transcode.

use std::path::{Path, PathBuf};

use oag_game::boot;
use oag_game::frontend::states;
use oag_game::input::{Input, button};
use oag_pure::frontend::states as pure_states;

/// Every Pure pressing present, as `(label, path)`.
///
/// A missing image is skipped rather than failed, the way `boot_ground_truth.rs`
/// does it, unless `OAG_REQUIRE_GAME_DATA` says the caller expects them.
fn images() -> Vec<(&'static str, PathBuf)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    for (label, name) in [
        ("pure-psp-eu", "data/images/pure-psp-eu.chd"),
        ("pure-psp-usa", "data/images/pure-psp-usa.chd"),
    ] {
        let path = root.join(name);
        if path.exists() {
            found.push((label, path));
        } else {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "OAG_REQUIRE_GAME_DATA is set but {} is missing",
                path.display()
            );
            println!("skipping: {} not present", path.display());
        }
    }
    found
}

/// A default boot of one pressing, with **no `movie` override**.
///
/// `movie: None` is the whole point: it lets `boot::load` resolve the leg's own
/// movie, which on Pure is supposed to be nothing at all. Passing a name here -
/// as `boot_ground_truth.rs` does - would defeat every assertion below.
fn load(image: &Path) -> boot::Boot {
    let options = boot::Options {
        // No saved language: these boot a fresh install every time.
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: oag_game::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-pure-boot-ground-truth"),
        audio_cache: oag_game::boot::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    boot::load(&options).expect("loading the boot sequence")
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_boot_plays_no_movie_before_the_picker() {
    for (label, image) in images() {
        let loaded = load(&image);
        // The picker is the boot screen and plays nothing, which is the narrow
        // claim a cold boot established. The reel *is* loaded - it plays one step
        // later, on `Developer Publisher Screen` - so this asserts the sequence's
        // first *screen*, not the absence of the movie.
        assert_eq!(
            loaded.frontend.machine().current(),
            Some(states::LANGUAGE_SELECTION),
            "{label}: the first frame after power-on is the picker itself"
        );
        assert_eq!(
            loaded.frontend.movie_states().first().copied(),
            Some(pure_states::DEVELOPER_PUBLISHER),
            "{label}: the first movie plays after the picker, not before it"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_developer_publisher_screen_plays_the_reel() {
    // The correction this file's own history turns on. The screen declares no
    // widgets, and this build read that as "its cards are drawn by engine code" -
    // wrong, because a Pure child screen inherits its parent's widgets. It plays
    // `IntroMovie1` off `Intro Screen`, and the cards are frames 144 and 231.
    for (label, image) in images() {
        let loaded = load(&image);
        assert!(
            loaded.movie.is_some(),
            "{label}: the reel is on the boot path and has to load"
        );
        let named = format!("{}: ", oag_pure::names::INTRO_MOVIE);
        assert!(
            loaded.report.iter().any(|line| line.starts_with(&named)),
            "{label}: the report names the reel it opened; report was {:#?}",
            loaded.report
        );
        assert_eq!(
            loaded.frontend.movie_states(),
            vec![pure_states::DEVELOPER_PUBLISHER, pure_states::FMV_INTRO,],
            "{label}: two movies, on the two screens the disc plays them on"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_sequence_starts_on_the_language_picker() {
    for (label, image) in images() {
        let loaded = load(&image);
        assert_eq!(
            loaded.frontend.machine().current(),
            Some(states::LANGUAGE_SELECTION),
            "{label}: cold-boot-observed on both pressings, first frame after power-on"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_picker_leads_to_its_second_boot_movie_then_the_title_screen() {
    for (label, image) in images() {
        let mut loaded = load(&image);
        let mut input = Input::new();

        // Confirm whatever is highlighted. The picker is the boot's first screen
        // here, so there is nothing to run through to reach it.
        input.begin_frame(1 << button::CROSS);
        loaded.frontend.update(1.0 / 60.0, &mut input, None);
        assert!(
            loaded.frontend.chosen().is_some(),
            "{label}: cross on the picker takes the highlighted language"
        );
        assert!(
            loaded
                .frontend
                .machine()
                .is(pure_states::DEVELOPER_PUBLISHER),
            "{label}: the picker goes to the developer and publisher cards, which is what \
             the disc does and what this build got wrong; got {:?}",
            loaded.frontend.machine().current()
        );

        // The cards leave on a timer; the storage warning then waits for cross.
        for _ in 0..1200 {
            if loaded
                .frontend
                .machine()
                .is(pure_states::MEMORY_STICK_WARNING)
            {
                break;
            }
            input.begin_frame(0);
            loaded.frontend.update(1.0 / 60.0, &mut input, None);
        }
        assert!(
            loaded
                .frontend
                .machine()
                .is(pure_states::MEMORY_STICK_WARNING),
            "{label}: the cards advance themselves; got {:?}",
            loaded.frontend.machine().current()
        );
        input.begin_frame(1 << button::CROSS);
        loaded.frontend.update(1.0 / 60.0, &mut input, None);
        assert!(
            loaded.frontend.machine().is(pure_states::FMV_INTRO),
            "{label}: acknowledging the warning reaches the second boot movie; got {:?}",
            loaded.frontend.machine().current()
        );
        assert!(
            loaded.frontend.is_playing_movie(),
            "{label}: FMV Intro is a movie screen - saying otherwise stops its sound on \
             the first tick of the leg"
        );

        // With `no_video` there is no picture, but the leg still runs out over
        // the movie's own declared duration and leaves for the title screen.
        for _ in 0..12_000 {
            if loaded.frontend.machine().is(pure_states::TITLE_SCREEN) {
                break;
            }
            input.begin_frame(0);
            loaded.frontend.update(1.0 / 60.0, &mut input, None);
        }
        assert!(
            loaded.frontend.machine().is(pure_states::TITLE_SCREEN),
            "{label}: the movie running out reaches Title Screen; got {:?}",
            loaded.frontend.machine().current()
        );
        assert!(
            !loaded.frontend.is_playing_movie(),
            "{label}: the movie is over by Title Screen, so its sound has to stop"
        );

        // **What advances past `Title Screen` is not established.** Its
        // `TitleRedirect` carries no `forward` and its `Default goto` names a
        // screen this build does not have, so the screen is inert here on
        // purpose. This pins that rather than asserting a guess.
        for tick in 0..600 {
            input.begin_frame(if tick % 2 == 0 { u32::MAX } else { 0 });
            loaded.frontend.update(1.0 / 60.0, &mut input, None);
        }
        assert!(
            loaded.frontend.machine().is(pure_states::TITLE_SCREEN),
            "{label}: nothing evidenced advances past Title Screen yet"
        );
        assert!(
            !loaded.frontend.is_finished(),
            "{label}: Title Screen does not hand off to a race"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_second_boot_movie_is_the_one_the_disc_plays() {
    for (label, image) in images() {
        let loaded = load(&image);
        assert!(
            loaded.after_language_movie.is_some(),
            "{label}: the FMV Intro screen exists, so its movie has to load"
        );
        assert!(
            loaded
                .report
                .iter()
                .any(|line| line.contains("WoFMVNew_US.PMF")),
            "{label}: the report names the second movie; was {:#?}",
            loaded.report
        );
        let movie = loaded.after_language_movie.as_ref().unwrap();
        // Measured 2026-08-10 by extracting both movies from both pressings and
        // reading their PSMF stream descriptors: 480x272, byte-identical
        // descriptors, and `WoFMVNew_US.PMF` byte-identical across regions.
        assert_eq!(
            (movie.width, movie.height),
            (480, 272),
            "{label}: measured geometry"
        );
        assert_eq!(
            movie.frame_count, 2848,
            "{label}: 2848 frames, which the disc's own FMVFrameCount global calls 2847"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_second_movie_declares_sound() {
    for (label, image) in images() {
        let loaded = load(&image);
        // A data assertion, so it needs no transcode: the widget playing this
        // movie carries `sound="true"`, which is what makes wiring its own track
        // correct rather than a guess. `no_video` means nothing was decoded, so
        // the PCM itself is absent here by construction.
        let widget = loaded
            .frontend
            .screens()
            .with_movies()
            .flat_map(|screen| screen.movies.iter())
            .find(|movie| {
                movie
                    .entry_name()
                    .eq_ignore_ascii_case(oag_pure::names::FMV_INTRO_MOVIE)
            })
            .unwrap_or_else(|| panic!("{label}: no widget names the second boot movie"));
        assert!(
            widget.sound,
            "{label}: the widget is sound=\"true\", so the movie is heard"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_ships_no_menu_backdrop() {
    for (label, image) in images() {
        let loaded = load(&image);
        assert!(
            loaded.backdrop.is_none(),
            "{label}: neither Pure pressing carries Data\\Movies\\Backdrop.PMF, which is \
             what makes a two-video draw list unreachable on these discs"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_two_pressings_name_their_movies_the_same_way() {
    // The canary for `Movie::entry_name`'s unconditional `_US.PMF`. If the EU
    // pressing ever spells its movies differently, this fails with the name it
    // could not find rather than leaving the EU boot silently picture-less.
    let found = images();
    if found.len() < 2 {
        println!("skipping: needs both Pure pressings");
        return;
    }
    for (label, image) in found {
        let loaded = load(&image);
        assert!(
            loaded.after_language_movie.is_some(),
            "{label}: {} did not resolve on this pressing; the localisation suffix rule \
             is what to re-measure, not the boot sequence",
            oag_pure::names::FMV_INTRO_MOVIE
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_reel_flag_opens_the_screen_the_disc_itself_plays_it_on() {
    // `--reel` was refused on Pure while this build believed the title had no reel
    // state. It has a better-evidenced one than Pulse: on the boot path, one step
    // after the picker. So the flag now opens that screen rather than erroring.
    for (label, image) in images() {
        let options = boot::Options {
            language: None,
            source: image.display().to_string(),
            dlc: Vec::new(),
            leg: oag_game::frontend::Leg::DevPubReel,
            movie: None,
            cache: std::env::temp_dir().join("oag-pure-boot-ground-truth"),
            audio_cache: oag_game::boot::default_audio_cache_dir(),
            extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
            no_video: true,
            refresh_video: false,
            prefer_av1_cache: false,
        };
        let loaded = boot::load(&options).expect("Pure's reel state is on its boot path");
        assert_eq!(
            loaded.frontend.machine().current(),
            Some(pure_states::DEVELOPER_PUBLISHER),
            "{label}: --reel opens the screen the reel plays on"
        );
        assert!(
            loaded.movie.is_some(),
            "{label}: and that screen's movie is the reel"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_whole_chain_is_walked_and_nothing_in_it_is_skipped() {
    // The regression guard for the order defect. Descoping the two screens between
    // the picker and the movie left the *order* wrong, not just their content, and
    // a passing suite did not notice - the same shape of mistake as the stale-save
    // profile this whole file exists for.
    for (label, image) in images() {
        let loaded = load(&image);
        assert!(
            !loaded.report.iter().any(|line| line.contains("skipped")),
            "{label}: every screen in Pure's chain is drivable now, so nothing should \
             report as skipped; report was {:#?}",
            loaded.report
        );
        for state in [
            pure_states::DEVELOPER_PUBLISHER,
            pure_states::MEMORY_STICK_WARNING,
        ] {
            assert!(
                loaded.frontend.screens().by_name(state).is_some(),
                "{label}: the disc declares {state:?}"
            );
        }
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_own_globals_are_merged_only_where_the_disc_leaves_them_out() {
    for (label, image) in images() {
        let loaded = load(&image);
        let globals = &loaded.frontend.screens().globals;
        // The nine the disc's own Skin.xml authors, read directly from the file.
        for name in [
            "TitleScale",
            "TitleXOffset",
            "TitleYOffset",
            "MenuScale",
            "MenuXOffset",
            "MSWarningScale",
            "MSWarningColour1",
            "MSWarningColour2",
            "FMVFrameCount",
        ] {
            assert!(
                globals.contains_key(name),
                "{label}: the disc declares {name}"
            );
        }
        // And the three it does not, which `FALLBACK_GLOBALS` measures by
        // sampling pixels - confidence 65, of the effect rather than the source.
        for (name, value) in oag_pure::frontend::FALLBACK_GLOBALS {
            assert_eq!(
                globals.get(*name).map(String::as_str),
                Some(*value),
                "{label}: {name} comes from the measured fallback table"
            );
        }
    }
}
