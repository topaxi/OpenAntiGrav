//! What each title actually puts on screen while it loads, read off its own
//! disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(loading_screen_ground_truth)'
//! ```
//!
//! The PS3 image has to be layer-1 decrypted first - `scripts/ps3iso.py
//! decrypt`, `docs/formats/ps3-disc.md`.
//!
//! # What is being established
//!
//! Three titles give three different answers, which is why
//! [`oag_title::Loading`] is an axis at all, and each answer is asserted
//! against the disc rather than against the table that states it:
//!
//! 1. **Pulse** ships the tips plugin and the glow strip, and no full-screen
//!    still.
//! 2. **Pure** ships none of it, and the loader says so instead of failing.
//! 3. **Wipeout HD** ships five illustrated features in two stylings and a
//!    caption, and **not** the tips plugin or the strip - which is the finding
//!    rather than a gap: its executable still carries `PI_LoadingScreen` and
//!    asks for `Data\Plugins\loading`, and the running game answers
//!    `FileSystem::Open FAILED` for it on every boot. See
//!    `docs/formats/hd-loading.md`.
//!
//! The third is the one worth pinning hardest, because it is the one a future
//! change could silently "fix" by pointing HD at Pulse's entry names and
//! getting a wave over a screen the disc never authored.

use std::path::{Path, PathBuf};

use oag_game::language::StringTable;
use oag_game::loading::Assets;

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);

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

/// The chosen language's table off one source, for the caption lookup.
fn strings(source: &Path) -> StringTable {
    let opened = oag_game::title::open_source(&source.display().to_string(), Vec::new())
        .expect("the source opens");
    let mut archives = opened.archives;
    let mut report = Vec::new();
    let plugins: &[&str] = opened.title.front_end.map_or(&[], |fe| fe.language_plugins);
    let languages = oag_game::boot::load_languages(&mut archives, plugins, &mut report);
    oag_game::boot::load_strings(&mut archives, &languages, Some("English"), &mut report)
}

/// Every feature illustration HD names is on the disc, in both stylings, and
/// decodes.
///
/// The list is `oag_hd::loading::FEATURES`, which is the order the executable
/// names the ten images in. All ten are asserted rather than the one this build
/// draws, because the styling axis is only real if both halves are shipped -
/// and it is the axis `settings.display.front_end_style` offers.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_feature_illustration_is_on_the_disc_in_both_stylings() {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("the PS3 source opens");

    let styles = oag_hd::loading::FEATURES;
    assert_eq!(styles.len(), 2, "HD and Fury");
    assert_eq!(styles[0].name, "HD");
    assert_eq!(styles[1].name, "FURY");
    for style in styles {
        assert_eq!(style.features.len(), 5, "{} has five", style.name);
        for feature in style.features {
            let blob = archives
                .read_name(feature.image)
                .unwrap_or_else(|e| panic!("{} {}: {e}", style.name, feature.image));
            let gtf = oag_formats::gtf::Gtf::parse(&blob)
                .unwrap_or_else(|e| panic!("{} {}: {e}", style.name, feature.image));
            let texture = gtf.only().expect("one texture per file");
            assert!(
                texture.width >= 256 && texture.height >= 256,
                "{} {} is {}x{}, too small to be an illustration",
                style.name,
                feature.image,
                texture.width,
                texture.height
            );
        }
    }

    // The two stylings are the same five features in the same order, differing
    // only by the `_fury` suffix - which is what makes swapping them a styling
    // rather than a different screen.
    for (hd, fury) in styles[0].features.iter().zip(styles[1].features) {
        assert_eq!(hd.title, fury.title);
        assert_eq!(hd.description, fury.description);
        assert_eq!(
            fury.image,
            hd.image.replace(".gtf", "_fury.gtf"),
            "the Fury art is the same name with a suffix"
        );
    }
}

/// HD's loading screen is a caption, an illustrated feature and no wave.
///
/// The absences are the finding, so they are asserted directly: a future change
/// that pointed HD at Pulse's entry names would draw a procedural wave over a
/// stand-in glow strip on a title whose own screen is right there in its
/// executable, and every other test here would still pass.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_draws_a_feature_and_a_caption_and_no_wave() {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let source = image.display().to_string();
    let table = strings(&image);
    let assets = Assets::load(&source, &table, None);
    for note in &assets.notes {
        println!("{note}");
    }

    assert!(!assets.wave, "HD ships no glow strip and no tips plugin");
    assert!(assets.tips.is_empty(), "{:?}", assets.tips);
    assert_eq!(
        assets.caption.as_deref(),
        Some("LOADING..."),
        "the caption is `FE_LOADINGDOT` out of the disc's own English table"
    );

    // The default styling is the base game's, and the feature is the one both
    // observations of the running game showed.
    let feature = assets.feature.as_ref().expect("HD ships features");
    let art = assets.art.as_ref().expect("HD ships art");
    assert!(
        art.illustration.is_some(),
        "the illustration is in the sheet"
    );
    assert!(
        art.sheet.get(r"Data\FE\Images\Pilot_Assist.gtf").is_some(),
        "and it is the one both observations of the running game showed"
    );
    // The marks the original frames the screen with are in the same sheet.
    assert!(art.title_arrow.is_some());
    assert!(art.subtitle_arrow.is_some());
    assert!(art.rule.is_some());
    assert!(art.corner.is_some());
    assert!(art.dot.is_some());
    assert_eq!(feature.title.as_deref(), Some("PILOT ASSIST"));
    assert!(
        feature
            .description
            .starts_with("Pilot Assist can aid your navigation"),
        "{:?}",
        feature.description
    );

    // Asking for Fury's styling gets Fury's art and the same words.
    let fury = Assets::load(&source, &table, Some("FURY"));
    let fury_feature = fury.feature.as_ref().expect("Fury ships features too");
    let fury_art = fury.art.as_ref().expect("Fury ships art");
    assert!(
        fury_art
            .sheet
            .get(r"Data\FE\Images\Pilot_Assist_fury.gtf")
            .is_some(),
        "asking for Fury's styling gets Fury's art"
    );
    assert_eq!(fury_feature.description, feature.description);

    // And the two entries a wave would need really are absent, so this reads as
    // "cut" rather than "not looked for". The running game agrees: its own
    // `TTY.log` carries `FileSystem::Open FAILED` for the first of them.
    let archives = oag_hd::open(&source).expect("the PS3 source opens");
    for entry in [
        oag_pulse::loading::TIPS_ENTRY,
        oag_pulse::loading::GLOW_STRIP_ENTRY,
    ] {
        assert!(
            archives.locate(entry).is_none(),
            "{entry} is on an HD archive after all; the cut finding needs re-reading"
        );
    }
}

/// Pulse still gets exactly what it always got: tips, a real strip, no still.
///
/// The regression guard for the title axis itself. Routing the loader through
/// `oag_title::Loading` was meant to leave the one title that had a loading
/// screen exactly where it was.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_still_gets_its_tips_and_its_own_strip() {
    let Some(image) = image("pulse-psp-eu.chd") else {
        return;
    };
    let source = image.display().to_string();
    let table = strings(&image);
    let assets = Assets::load(&source, &table, None);
    for note in &assets.notes {
        println!("{note}");
    }

    assert!(assets.wave, "Pulse authors the wave");
    assert!(
        assets.tips.len() >= 20,
        "the disc's own loading tips: {}",
        assets.tips.len()
    );
    assert!(
        assets.feature.is_none(),
        "Pulse ships no illustrated features"
    );
    assert!(assets.art.is_none(), "and so draws no illustration");
    assert!(
        assets.caption.is_none(),
        "Pulse names no caption id, so the heading stands"
    );
    // The disc's own strip rather than the stand-in, which `Assets::load`
    // reports by name when it has to fall back.
    assert!(
        !assets
            .notes
            .iter()
            .any(|note| note.contains("authored stand-in strip")),
        "{:?}",
        assets.notes
    );
}

/// Pure authors no loading screen, and the loader says which of the two that is.
///
/// "This title has none" and "this title's assets would not read" are different
/// facts and used to produce the same silence. The note is the difference.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_authors_no_loading_screen_and_says_so() {
    let Some(image) = image("pure-psp-eu.chd") else {
        return;
    };
    let source = image.display().to_string();
    let table = strings(&image);
    let assets = Assets::load(&source, &table, None);
    for note in &assets.notes {
        println!("{note}");
    }

    assert!(!assets.wave);
    assert!(assets.tips.is_empty());
    assert!(assets.feature.is_none());
    assert!(assets.art.is_none());
    assert!(
        assets
            .notes
            .iter()
            .any(|note| note.contains("authors no loading screen of its own")),
        "{:?}",
        assets.notes
    );
}

/// The circuit load really runs on a thread, and the screen can name it while
/// it does.
///
/// **What this covers and what it does not.** The worker itself is testable
/// headlessly and is tested here: that it comes back with a `Loaded`, that its
/// snapshot carries the label a caller handed it and counts nothing, and that
/// `join` is idempotent. The *stage transition* built on it - `launch_race`
/// putting the screen up, `finish_loading` swapping it for the grid - needs a
/// window and is not covered anywhere.
///
/// Wipeout HD rather than Pulse on purpose: HD's circuits are the slow end of
/// the 1.5-to-5-second range that made the blocking load worth moving off the
/// frame thread in the first place.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_circuit_load_runs_on_a_worker_and_names_what_it_is_reading() {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let options = oag_game::race::Options {
        source: image.display().to_string(),
        class: oag_physics::SpeedClass::Venom,
        track: Some(r"Data\Environments\Talons_Junction\track.vex".to_string()),
        ..oag_game::race::Options::default()
    };
    let label = "TALON'S JUNCTION".to_string();
    let mut worker = oag_game::race::LoadWorker::spawn(options, Some(label.clone()));

    // The screen's own view of it, before the load has necessarily finished.
    let progress = worker.progress();
    assert_eq!(progress.current.as_deref(), Some(label.as_str()));
    assert_eq!(progress.total, 0, "a race load counts nothing");
    assert_eq!(progress.done, 0);

    let loaded = worker
        .join()
        .expect("the thread started")
        .expect("the circuit reads");
    assert!(
        !loaded.setup.spline.is_empty(),
        "a real circuit came back, not an empty one"
    );
    assert!(worker.is_finished());
    assert!(
        worker.join().is_none(),
        "joining twice is a no-op rather than a panic"
    );
}
