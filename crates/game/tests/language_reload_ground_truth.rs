//! Proves `boot::load_shell` actually reads the *requested* language, not
//! whichever plugin a source lists first.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(language_reload_ground_truth)'
//! ```
//!
//! # What this is for
//!
//! A maintainer playing Pulse on the PSP reported two bugs on 2026-09-27:
//! the in-race HUD did not follow the front end's saved language, and picking
//! a language in OPTIONS took a restart to show anywhere. Both traced back to
//! the same shape of mistake - a call site that had a player's chosen
//! language in hand and threw it away, either passing `None` outright
//! (`race::hud::load_hud`'s one caller) or never being asked to reload at all
//! (`Session::apply_setting`'s `"language"` row, before
//! `Session::resupply_language` existed).
//!
//! `Session::resupply_language` is the fix for the second bug, and it works by
//! calling `boot::load_shell` again with the newly picked language and folding
//! the result through `main::session::Shell::from_boot` - the same function a
//! fresh boot uses. Both `Session` and `Shell` live in the `oag-game` binary
//! crate, which this integration test (built against the `oag-game` **library**
//! crate) cannot reach - so what is asserted here is the shared, GPU-free half
//! both a boot and a live reload depend on: that `boot::load_shell`'s own
//! output genuinely differs by `Options::language`, on the exact fields
//! `Shell::from_boot` reads to build `Teams`/`RaceModes`'s row labels. A
//! player-visible before/after screenshot of the HUD case, and of the OPTIONS
//! page's own live switch, are in
//! `data/scratch/drive-2026-09-27/live-language/`.
//!
//! **The chain default matters here.** The PSP EU pressing's English is `PI000`
//! (it has no `PI012`). `oag_pulse::LANGUAGE_PLUGINS` once ended in `PI012`, so
//! this pressing loaded no English and `chosen_language`'s fallback landed on
//! French - the trap the maintainer's report named; the release's manifest
//! (`oag_pulse::LANGUAGE_MANIFESTS`) fixed that. Every assertion below compares
//! two languages that are each other's non-default: German and Italian, neither
//! of them English, so a passing run proves the preference is honoured rather
//! than merely coinciding with the chain's own answer.

use oag_game::boot;

fn shell_for(image: &std::path::Path, language: &str) -> boot::Shell {
    let options = boot::Options {
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::default(),
        language: Some(language.to_string()),
        movie: None,
        cache: std::env::temp_dir(),
        audio_cache: std::env::temp_dir(),
        extent: oag_game::movie::Extent::Whole,
        // No video decoded here - this is the "cheap half" of the boot, the
        // same half `Session::resupply_language` pays again on a LANGUAGE
        // row pick, and this test is about its data, not its movies.
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    let (shell, _archives, _title) =
        boot::load_shell(&options).unwrap_or_else(|e| panic!("{}: {e:#}", image.display()));
    shell
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_chain_default_on_pulse_psp_eu_is_english() {
    // The falsification this whole file exists to guard: every assertion
    // below picks two *non-default* languages on purpose, and this is the
    // measurement that says why. If this ever starts asserting "English",
    // the EU pressing shipped an English plugin at some point after this was
    // written and the rest of the file's language choices should be revisited
    // - not because they would be wrong, but because they would no longer be
    // proving what their own doc comments say they prove.
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        println!("skipping: pulse-psp-eu.chd not present");
        return;
    };
    let options = boot::Options {
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::default(),
        // The one thing this test asks for: nothing. A fresh install with no
        // settings.toml yet reads exactly this - see `chosen_language`'s own
        // doc for the English-then-first fallback that lands here.
        language: None,
        movie: None,
        cache: std::env::temp_dir(),
        audio_cache: std::env::temp_dir(),
        extent: oag_game::movie::Extent::Whole,
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    let (shell, _archives, _title) = boot::load_shell(&options).expect("opening the EU pressing");
    assert_eq!(
        shell.entries.as_deref(),
        Some("Data\\Plugins\\PI000\\entries.xml"),
        "the EU pressing's chain default moved off PI000 (English) - re-read \
         this file's own module doc before trusting any assertion below that \
         assumes English is the default"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn picking_a_language_changes_which_plugin_load_shell_reads() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        println!("skipping: pulse-psp-eu.chd not present");
        return;
    };
    let german = shell_for(&image, "German");
    let italian = shell_for(&image, "Italian");
    assert_eq!(
        german.entries.as_deref(),
        Some("Data\\Plugins\\PI009\\entries.xml"),
        "German did not resolve to its own plugin"
    );
    assert_eq!(
        italian.entries.as_deref(),
        Some("Data\\Plugins\\PI011\\entries.xml"),
        "Italian did not resolve to its own plugin"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn picking_a_language_changes_the_labels_shell_from_boot_builds_rows_from() {
    // `main::session::Shell::from_boot` - the function both a fresh boot and
    // `Session::resupply_language` build a menu's `Teams`/`RaceModes` rows
    // from - reads exactly two things off a `boot::Shell` for those: its own
    // `strings` table (through `oag_ui::menu::mode_choices`) and each
    // `catalogue::Team::label`. Both are asserted directly here since
    // `Shell::from_boot` itself is not reachable from this crate.
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        println!("skipping: pulse-psp-eu.chd not present");
        return;
    };
    let german = shell_for(&image, "German");
    let italian = shell_for(&image, "Italian");

    let german_modes: Vec<String> = oag_ui::menu::mode_choices(&german.strings)
        .into_iter()
        .map(|choice| choice.label)
        .collect();
    let italian_modes: Vec<String> = oag_ui::menu::mode_choices(&italian.strings)
        .into_iter()
        .map(|choice| choice.label)
        .collect();
    assert_ne!(
        german_modes, italian_modes,
        "RACE MODE row labels did not move between German and Italian - \
         `Session::resupply_language`'s own re-`supply` of \
         `menu::ValueSource::RaceModes` would have nothing to show"
    );
    assert!(
        !german_modes.is_empty() && !italian_modes.is_empty(),
        "no modes resolved at all on either language"
    );

    // Time Trial's own row, spelled the way each language's disc text does -
    // a fixed pair to fail loudly on, rather than only on the weaker
    // not-equal check above, if a future change reorders `oag_race::Mode::ALL`.
    assert!(
        german_modes.iter().any(|label| label == "Zeitrennen"),
        "German RACE MODE rows did not include \"Zeitrennen\": {german_modes:?}"
    );
    assert!(
        italian_modes.iter().any(|label| label == "Prova a tempo"),
        "Italian RACE MODE rows did not include \"Prova a tempo\": {italian_modes:?}"
    );
}
