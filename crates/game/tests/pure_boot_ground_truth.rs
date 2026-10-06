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
//! over **both** Pure pressings, because each pressing's own executable bakes a
//! different, region-suffixed literal into every `localised="true"` `<Movie>`
//! widget's resolved name - `Movie_ParseAttributes`, on the same evidence class
//! `docs/ghidra/functions/psp-pure-eu/title-screen.md` already found for
//! `TitleFrame`'s wordmark; see
//! `docs/ghidra/functions/psp-pure-eu/movie-localised-suffix.md`.
//! `oag_pure::frontend::localised_movie_region` resolves which suffix a
//! source's own serial takes, and [`each_pressing_resolves_its_own_cut`] is the
//! canary that keeps it - it fails with the name it could not find rather than
//! leaving a pressing silently picture-less.
//!
//! It runs with `no_video`, so it never invokes `ffmpeg` and never writes a
//! cache. What is tested is the sequencing and the data, not the transcode.

use std::path::{Path, PathBuf};

use oag_game::boot;
use oag_game::input::{Button, Input};
use oag_pure::frontend::states as pure_states;
use oag_ui::frontend::states;

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

/// Which region a labelled image resolves its `localised` movies to,
/// restated by label rather than read back off a loaded `Boot` -
/// `oag_pure::frontend::localised_movie_region`'s own match on the serial,
/// mirrored here because `images()` already names the pressing.
fn expected_region(label: &str) -> &'static str {
    if label == "pure-psp-usa" { "US" } else { "EU" }
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
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-pure-boot-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
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
        let named = format!("{}: ", oag_pure::names::intro_movie(expected_region(label)));
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
        input.begin_frame(Button::Cross.bit());
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
        input.begin_frame(Button::Cross.bit());
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

        // **`Title Screen` waits for START and for nothing else.** Its
        // `TitleRedirect` carries no `forward` at all, so the button is not
        // authored there - it comes from the screen's own `idstring="PRESS
        // START"` widget and from watching the real firmware sit on this screen
        // until START. Ten seconds of every other button the abstract layer
        // carries, alternated with none, must leave it exactly where it is; a
        // hidden timeout would show up here.
        for tick in 0..600 {
            let held = !Button::Start.bit();
            input.begin_frame(if tick % 2 == 0 { held } else { 0 });
            loaded.frontend.update(1.0 / 60.0, &mut input, None);
        }
        assert!(
            loaded.frontend.machine().is(pure_states::TITLE_SCREEN),
            "{label}: only START leaves Title Screen, and it does not time out; got {:?}",
            loaded.frontend.machine().current()
        );
        assert!(
            !loaded.frontend.is_finished(),
            "{label}: and nothing has been handed off yet"
        );

        // START does leave, for the menus. **The destination is this build's
        // divergence, not the disc's** - `TitleRedirect`'s `Default goto` names
        // `Profile Manager`, a screen this build does not have, exactly as
        // Pulse's `Show Logo` leads to four Memory Stick screens it does not
        // have either. Both titles reach the menus by the same edge; see
        // `docs/architecture/pure-boot.md`.
        input.begin_frame(Button::Start.bit());
        loaded.frontend.update(1.0 / 60.0, &mut input, None);
        assert!(
            loaded.frontend.machine().is(states::LAUNCH_GAME),
            "{label}: START on Title Screen hands off to the menus; got {:?}",
            loaded.frontend.machine().current()
        );
        assert!(
            loaded.frontend.is_finished(),
            "{label}: and the composition root is told, or the menus never open"
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
        let region = expected_region(label);
        assert!(
            loaded
                .report
                .iter()
                .any(|line| line.contains(oag_pure::names::fmv_intro_movie(region))),
            "{label}: the report names the second movie; was {:#?}",
            loaded.report
        );
        let movie = loaded.after_language_movie.as_ref().unwrap();
        // Measured 2026-08-10 by extracting `WoFMVNew_US.PMF` from both
        // pressings and reading its PSMF stream descriptor: 480x272,
        // byte-identical, 2848 frames - before the region-suffix fix, when
        // both discs loaded that one entry regardless of pressing.
        //
        // **2026-09-23, re-measured through each pressing's own resolved
        // cut**: the EU disc's `WoFMVNew_EU.PMF` is still 480x272 but decodes
        // to 2901 frames, not 2848 - the two cuts share a frame size but not a
        // duration, most likely the EU cut's own regional card holding the
        // screen longer. `frame_count` is asserted per region rather than
        // assumed shared for this reason.
        assert_eq!(
            (movie.width, movie.height),
            (480, 272),
            "{label}: measured geometry"
        );
        let expected_frames = match region {
            "US" => 2848,
            // Measured 2026-09-23 off `pure-psp-eu.chd`'s own `WoFMVNew_EU.PMF`.
            _ => 2901,
        };
        assert_eq!(
            movie.frame_count, expected_frames,
            "{label}: measured frame count for the {region} cut"
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
        let region = expected_region(label);
        let widget = loaded
            .frontend
            .screens()
            .with_movies()
            .flat_map(|screen| screen.movies.iter())
            .find(|movie| {
                movie
                    .entry_name(region)
                    .eq_ignore_ascii_case(oag_pure::names::fmv_intro_movie(region))
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
fn each_pressing_resolves_its_own_cut() {
    // The canary for the region-suffix fix: `Movie::entry_name` used to append
    // an unconditional `_US`, so the EU disc's own `_EU.PMF` entries were never
    // asked for at all. If a pressing's own resolved cut ever stops existing,
    // this fails with the name it could not find rather than leaving that
    // pressing's boot silently picture-less.
    let found = images();
    if found.len() < 2 {
        println!("skipping: needs both Pure pressings");
        return;
    }
    for (label, image) in found {
        let loaded = load(&image);
        let region = expected_region(label);
        assert!(
            loaded.after_language_movie.is_some(),
            "{label}: {} did not resolve on this pressing; the localisation suffix rule \
             is what to re-measure, not the boot sequence",
            oag_pure::names::fmv_intro_movie(region)
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
            leg: oag_ui::frontend::Leg::DevPubReel,
            movie: None,
            cache: std::env::temp_dir().join("oag-pure-boot-ground-truth"),
            audio_cache: oag_source::cache::default_audio_cache_dir(),
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

/// The regression guard for `oag_pure::FRONT_END::language_plugins`.
///
/// The old list was Pulse's USA set copied wholesale - `PI008`-`PI012` - which
/// resolved `PI012` as "English" (the first `<Entry Language="English">` in a
/// plugin with no `<Font>` block at all) while naming no `entries.xml`, so the
/// picker's English row drew with an empty string table. See
/// `docs/formats/pure-status.md#the-language-plugin-id-space-is-pures-own-not-pulses`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_picker_offers_its_own_manifest_english_included() {
    for (label, image) in images() {
        let loaded = load(&image);
        let languages = loaded.frontend.languages();

        let found: Vec<(&str, &str)> = languages
            .iter()
            .map(|l| (l.plugin.as_str(), l.name.as_str()))
            .collect();
        // Each pressing's own executable manifest, in order: the USA one
        // offers three of the five plugins its disc carries.
        let expected: &[(&str, &str)] = if label == "pure-psp-usa" {
            &[
                ("PI000", "English"),
                ("PI010", "Spanish"),
                ("PI008", "French"),
            ]
        } else {
            &[
                ("PI000", "English"),
                ("PI010", "Spanish"),
                ("PI008", "French"),
                ("PI009", "German"),
                ("PI011", "Italian"),
            ]
        };
        assert_eq!(
            found, expected,
            "{label}: Pure's own plugin ids, not Pulse's"
        );

        // English's string table lives inline in `PI000\Definition.xml` -
        // Pure names no `entries.xml` for it at all - so this is the one
        // language here with `entries: None` and it must still resolve a
        // real table via `load_strings`'s inline fallback.
        let english = languages
            .iter()
            .find(|l| l.name == "English")
            .expect("English is in the list asserted above");
        assert_eq!(english.entries, None, "{label}: PI000 names no entries.xml");
        assert!(
            loaded.strings.len() > 100,
            "{label}: English should resolve PI000's inline string table, got {} entries",
            loaded.strings.len()
        );

        // The HUD's own idstring convention on Pure, read straight off
        // `TimeTrial_HUD.xml`'s `idstring="HUD_Lap"` - not Pulse's `IG_HUD_LAP`.
        assert_eq!(loaded.strings.get_or_id("HUD_Lap"), "Lap", "{label}");
        assert_eq!(loaded.strings.get_or_id("HUD_best"), "best", "{label}");
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
        // And the ones it does not: those come from the *style* skin
        // `Data\Plugins\PI001\Definition.xml` activates beside it, whose own
        // `Data\Skins\Default\Skin.xml` declares 41 globals - identically on
        // both pressings. They were pixel-sampled into
        // `oag_pure::frontend::FALLBACK_GLOBALS` at confidence 65 until
        // 2026-09-10, and three of the four sampled values were wrong.
        assert!(oag_pure::frontend::FALLBACK_GLOBALS.is_empty());
        for (name, value) in [
            ("TitleColor", "0xFFED4796"),
            ("DesignColor", "0xFF5FDBF6"),
            ("TextColor", "0xFF11ACD0"),
            ("FrameLineColor", "0xFE99C9D8"),
            // The two the selection screens tint their stills with.
            ("ShipColor", "0xFF99D9E8"),
            ("TrackColor", "0xFF99D9E8"),
        ] {
            assert_eq!(
                globals.get(name).map(String::as_str),
                Some(value),
                "{label}: {name} comes from the activated style skin"
            );
        }
    }
}

/// `MemoryStickWarning`'s two `Image`s carry `x`/`y`/`width`/`height` and a
/// `Color="FEGlobals->MSWarningColour1"` indirection - not full-screen, and
/// not a literal colour. Both have to hold for the screen to draw a warning
/// stripe rather than a full-screen wash or nothing at all.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_memory_stick_warnings_two_stripes_keep_their_own_rects() {
    for (label, image) in images() {
        let loaded = load(&image);
        let screen = loaded
            .frontend
            .screens()
            .by_name("MemoryStickWarning")
            .unwrap_or_else(|| panic!("{label}: MemoryStickWarning"));
        let color = loaded
            .frontend
            .screens()
            .globals
            .get("MSWarningColour1")
            .and_then(|v| oag_ui::screen::parse_argb(v))
            .unwrap_or_else(|| panic!("{label}: MSWarningColour1 must resolve to a real colour"));
        assert_eq!(
            screen.fills,
            vec![
                oag_ui::screen::Fill {
                    name: None,
                    x: 0.0,
                    y: 10.0,
                    width: Some(480.0),
                    height: Some(1.0),
                    color,
                    gradient: None,
                    reveal: Vec::new(),
                    transition: 0.0,
                },
                oag_ui::screen::Fill {
                    name: None,
                    x: 0.0,
                    y: 240.0,
                    width: Some(480.0),
                    height: Some(1.0),
                    color,
                    gradient: None,
                    reveal: Vec::new(),
                    transition: 0.0,
                },
            ],
            "{label}: two thin stripes, not a 480x272 wash"
        );
    }
}

/// `Title Screen`'s seventeen frame-line and corner-bracket widgets, every one
/// `Color="FEGlobals->FrameLineColor"` and none of them full-screen - the
/// widget this thread's own `Animation`/`Fill` work was for. `FrameLineColor`
/// is undeclared in the front-end root and **declared by the activated style
/// skin**, so this also pins that the skin's own value is what draws.
/// Rects read off `pure-psp-usa.chd`'s own `Skin.xml`, in document order,
/// behind the white background `Fill` `Show Logo`-style backdrops already
/// cover.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn title_screens_frame_lines_keep_their_own_rects_and_share_one_colour() {
    for (label, image) in images() {
        let loaded = load(&image);
        let screen = loaded
            .frontend
            .screens()
            .by_name("Title Screen")
            .unwrap_or_else(|| panic!("{label}: Title Screen"));
        let color = loaded
            .frontend
            .screens()
            .globals
            .get("FrameLineColor")
            .and_then(|v| oag_ui::screen::parse_argb(v))
            .unwrap_or_else(|| panic!("{label}: FrameLineColor must resolve to a real colour"));
        assert_eq!(
            color,
            oag_ui::screen::parse_argb("0xFE99C9D8").unwrap(),
            "{label}: Data\\Skins\\Default\\Skin.xml declares FrameLineColor"
        );
        // `reveal` keys read straight off this same parse, not hand-typed:
        // every one of the sixteen brackets `TextureWidth` between its own
        // negated width (fully hidden) and `0` (fully shown) - the reveal
        // convention `docs/ghidra/functions/psp-pure-eu/title-screen.md`
        // documents, now pinned per-widget rather than only described.
        let key = |time: f32, texture_width: f32| oag_ui::screen::RevealKey {
            time,
            texture_width,
        };
        let rect =
            |x: f32, y: f32, width: f32, height: f32, reveal: Vec<oag_ui::screen::RevealKey>| {
                oag_ui::screen::Fill {
                    name: None,
                    x,
                    y,
                    width: Some(width),
                    height: Some(height),
                    color,
                    gradient: None,
                    reveal,
                    transition: 0.0,
                }
            };
        let white_background = oag_ui::screen::Fill {
            name: None,
            x: 0.0,
            y: 0.0,
            width: Some(480.0),
            height: Some(272.0),
            color: 0xffff_ffff,
            gradient: None,
            reveal: Vec::new(),
            transition: 0.0,
        };
        assert_eq!(
            screen.fills,
            vec![
                white_background,
                rect(14.0, 240.0, 1.0, 16.0, vec![key(0.0, -1.0), key(0.1, 0.0)]),
                rect(
                    14.0,
                    240.0,
                    235.0,
                    1.0,
                    vec![key(0.0, -235.0), key(0.2, 0.0)]
                ),
                rect(
                    184.0,
                    240.0,
                    1.0,
                    8.0,
                    vec![key(0.0, -1.0), key(0.1, -1.0), key(0.15, 0.0)],
                ),
                rect(
                    241.0,
                    240.0,
                    1.0,
                    11.0,
                    vec![key(0.0, -1.0), key(0.16, -1.0), key(0.2, 0.0)],
                ),
                rect(
                    249.0,
                    232.0,
                    3.0,
                    1.0,
                    vec![key(0.0, -3.0), key(0.63, -3.0), key(0.65, 0.0)],
                ),
                rect(
                    249.0,
                    232.0,
                    1.0,
                    3.0,
                    vec![key(0.0, -1.0), key(0.63, -1.0), key(0.65, 0.0)],
                ),
                rect(
                    249.0,
                    250.0,
                    3.0,
                    1.0,
                    vec![key(0.0, -3.0), key(0.63, -3.0), key(0.65, 0.0)],
                ),
                rect(
                    249.0,
                    247.0,
                    1.0,
                    3.0,
                    vec![key(0.0, -1.0), key(0.63, -1.0), key(0.65, 0.0)],
                ),
                rect(
                    249.0,
                    237.0,
                    1.0,
                    8.0,
                    vec![key(0.0, -1.0), key(0.63, -1.0), key(0.65, 0.0)],
                ),
                rect(
                    283.0,
                    237.0,
                    1.0,
                    8.0,
                    vec![key(0.0, -1.0), key(0.63, -1.0), key(0.65, 0.0)],
                ),
                rect(
                    280.0,
                    232.0,
                    3.0,
                    1.0,
                    vec![key(0.0, -3.0), key(0.63, -3.0), key(0.65, 0.0)],
                ),
                rect(
                    283.0,
                    232.0,
                    1.0,
                    3.0,
                    vec![key(0.0, -1.0), key(0.63, -1.0), key(0.65, 0.0)],
                ),
                rect(
                    280.0,
                    250.0,
                    4.0,
                    1.0,
                    vec![key(0.0, -4.0), key(0.63, -4.0), key(0.65, 0.0)],
                ),
                rect(
                    283.0,
                    247.0,
                    1.0,
                    3.0,
                    vec![key(0.0, -1.0), key(0.63, -1.0), key(0.65, 0.0)],
                ),
                rect(
                    419.0,
                    240.0,
                    1.0,
                    10.0,
                    vec![key(0.0, -1.0), key(0.35, -1.0), key(0.36, 0.0)],
                ),
                rect(
                    283.0,
                    240.0,
                    136.0,
                    1.0,
                    vec![key(0.0, -136.0), key(0.25, -136.0), key(0.35, 0.0)],
                ),
                rect(
                    286.0,
                    250.0,
                    133.0,
                    1.0,
                    vec![key(0.0, -133.0), key(0.25, -133.0), key(0.35, 0.0)],
                ),
            ],
            "{label}: every frame line at its own rect, none of them a wash, and each \
             carries the reveal timeline its own <Animation><Key> authors"
        );
    }
}

/// `TitleFrame`, the "wipEout pure" wordmark - see
/// `docs/formats/pure-status.md#the-title-screen-wordmark-titleframe` for how
/// entries 535 (EU) and 537 (USA) of `Data.wad` were found and matched
/// against real captured frames of each pressing.
///
/// **The disc's own XML gives this widget no `src` at all**, so this is
/// exactly the case [`oag_pure::frontend::title_frame_src`] exists for: this
/// test pins that the fallback resolves on both pressings, at the widget's own
/// authored rect, to a texture that actually decodes - **and that the two
/// pressings resolve to their own, different, hash** rather than one pressing
/// silently drawing the other's wordmark colourway. That is the bug
/// `Screen_ConstructTitleScreen`'s own reading found:
/// `docs/ghidra/functions/psp-pure-eu/title-screen.md`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn title_screens_own_wordmark_gets_the_measured_texture() {
    for (label, image) in images() {
        // Each pressing's executable bakes in its own literal, region-suffixed
        // texture name - see `oag_pure::frontend::title_frame_src`'s own doc
        // comment. A pressing this match does not name would silently fall
        // through to the EU default and this assertion would catch it.
        let expected_src = match label {
            "pure-psp-usa" => "hash:3af18d90",
            "pure-psp-eu" => "hash:b6677aab",
            other => panic!("no expected TitleFrame src recorded for {other}"),
        };
        let loaded = load(&image);
        let screen = loaded
            .frontend
            .screens()
            .by_name("Title Screen")
            .unwrap_or_else(|| panic!("{label}: Title Screen"));
        let title_frame = screen
            .images
            .iter()
            .find(|image| image.name.as_deref() == Some("TitleFrame"))
            .unwrap_or_else(|| {
                panic!("{label}: TitleFrame did not resolve through the fallback table")
            });
        assert_eq!(title_frame.src, expected_src, "{label}");
        assert_eq!((title_frame.x, title_frame.y), (0.0, 76.0));
        assert_eq!(
            (title_frame.width, title_frame.height),
            (Some(480.0), Some(128.0))
        );
        // `StartEnabled="false"` in the XML, and no `EnableTransition`/
        // `Transition` authored anywhere on it - so it resolves to the
        // measured class-wide default, not `0.0`. See
        // `oag_ui::screen::resolve_fade_in`'s own doc and
        // `docs/ghidra/functions/psp-pure-eu/title-screen.md`'s
        // `Element_UpdateFade` section, which reads this widget's own
        // `+0x70`/`+0x74` as `0.1`/`0.1` live on `pure-psp-eu.chd`.
        assert!(!title_frame.start_enabled, "{label}");
        assert_eq!(
            title_frame.transition,
            oag_ui::screen::MEASURED_HIDDEN_WIDGET_FADE_IN_SECONDS,
            "{label}: TitleFrame's own fade-in duration"
        );
        let placed = loaded
            .sprites
            .get(&title_frame.src)
            .unwrap_or_else(|| panic!("{label}: {expected_src} must decode as a texture"));
        assert_eq!(
            (placed.width, placed.height),
            (512, 128),
            "{label}: the physical texture is padded wider than the widget's own 480 TxtrWidth"
        );
    }
}

/// `BackgroundTopRightImage`, the "ワイプアウト" wordmark beside the swoosh
/// logo - see
/// `docs/formats/pure-status.md#fe-screens-corner-logo-backgroundtoprightimage`
/// for how entry 27 of `Data.wad` was found and matched against a real
/// captured `Main Menu` frame, and
/// `docs/ghidra/functions/psp-pure-eu/title-screen.md` for the runtime
/// mechanism read since: a declared `FEGlobals->BackgroundTopRightTexture`
/// global, not a hard-coded hash.
///
/// The same shape of test as [`title_screens_own_wordmark_gets_the_measured_texture`]:
/// this widget's own `Skin.xml` gives it no `src` either, so
/// [`oag_pure::frontend::FALLBACK_IMAGES`] is what fills it, and this test
/// pins that the fallback resolves on both pressings, at the widget's own
/// authored rect, to a texture that actually decodes. **The resolved `src` is
/// now the literal WAD path `Data\Skins\Default\Skin.xml` itself declares**,
/// not a `hash:` spec - `oag_pure::hashes::MENU_TOPRIGHT_LOGO` records the
/// hash that literal name resolves to, for the evidence trail alone.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn fe_screens_own_corner_logo_gets_the_measured_texture() {
    const EXPECTED_SRC: &str = r"Data\Skins\Default\Images\default_texture.mip";
    for (label, image) in images() {
        let loaded = load(&image);
        let screen = loaded
            .frontend
            .screens()
            .by_name("FE Screen")
            .unwrap_or_else(|| panic!("{label}: FE Screen"));
        let corner_logo = screen
            .images
            .iter()
            .find(|image| image.name.as_deref() == Some("BackgroundTopRightImage"))
            .unwrap_or_else(|| {
                panic!(
                    "{label}: BackgroundTopRightImage did not resolve through the fallback table"
                )
            });
        assert_eq!(corner_logo.src, EXPECTED_SRC, "{label}");
        assert_eq!((corner_logo.x, corner_logo.y), (252.0, 3.0));
        assert_eq!(
            (corner_logo.width, corner_logo.height),
            (Some(256.0), Some(32.0))
        );
        let placed = loaded
            .sprites
            .get(&corner_logo.src)
            .unwrap_or_else(|| panic!("{label}: {EXPECTED_SRC} must decode as a texture"));
        assert_eq!(
            (placed.width, placed.height),
            (256, 32),
            "{label}: already a power of two on both axes, unlike TitleFrame's padded crop"
        );
    }
}

/// `BackgroundImage` resolves through the same `FEGlobals->` mechanism as
/// [`fe_screens_own_corner_logo_gets_the_measured_texture`], to the empty
/// string `Data\Skins\Default\Skin.xml` declares for `BackgroundTexture` -
/// genuinely no texture, not an unresolved fallback. Pins that the widget
/// carries no `src` at all (so nothing downstream tries to decode `""` as a
/// texture) rather than silently dropping the whole widget.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn fe_screens_backdrop_resolves_its_declared_empty_global() {
    for (label, image) in images() {
        let loaded = load(&image);
        let screen = loaded
            .frontend
            .screens()
            .by_name("FE Screen")
            .unwrap_or_else(|| panic!("{label}: FE Screen"));
        assert!(
            screen
                .images
                .iter()
                .all(|image| image.name.as_deref() != Some("BackgroundImage")),
            "{label}: BackgroundImage should resolve to no `src` at all, not a texture"
        );
    }
}
