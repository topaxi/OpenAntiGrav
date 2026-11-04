//! A disc's rumble pattern, read by the probe's own reader and played by the
//! player, on HD/Fury's real image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game rumble_ground_truth:: --run-ignored all
//! ```
//!
//! What only real data says: the path spellings `oag_raceplay::rumble_library::disc_pattern`
//! tries resolve a pattern in HD's archive, and the player plays the file's own
//! bytes out under HD's small-motor law.

use oag_input::rumble::{Mode, Player, StepContext};
use oag_tables::rumble::{Kind, Library};

fn hd() -> Option<oag_assets::Archives> {
    let image = oag_testdata::image("hdfury-ps3-eu.iso")?;
    Some(
        oag_source::title::open_source(&image.to_string_lossy(), Vec::new(), Vec::new())
            .expect("HD opens")
            .archives,
    )
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu.iso"]
fn hd_plays_its_own_missile_pattern_under_the_ps3_law() {
    let Some(mut archives) = hd() else { return };
    let pattern = oag_raceplay::rumble_library::disc_pattern(&mut archives, "Missile_Fired_Rumble")
        .expect("the disc has it");
    assert_eq!(pattern.kind, Kind::OneShot);
    let (large, small) = (pattern.large.clone(), pattern.small.clone());
    assert!(!large.is_empty() && small.iter().any(|&b| b != 0));

    let law = Mode::Original
        .law(oag_game::rumble::original_law(oag_hd::TITLE.rumble))
        .expect("HD's original plays patterns");
    let mut library = Library::new();
    library.insert("missile", pattern);
    let mut player = Player::new(library, law);
    assert!(player.one_shot(0, None));
    let ctx = StepContext {
        race: true,
        ..StepContext::default()
    };
    for (frame, &want) in large.iter().enumerate() {
        let motors = player.step(ctx);
        assert_eq!(motors.large, want, "large, frame {frame}");
        let on = small.get(frame).copied().unwrap_or(0) != 0;
        assert_eq!(
            motors.small,
            if on { 255 } else { 0 },
            "small, frame {frame}"
        );
    }
    assert_eq!(player.step(ctx), oag_input::rumble::Motors::OFF);
    assert_eq!(player.active_voices(), 0);
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu.iso"]
fn a_name_the_disc_does_not_have_is_refused_by_name() {
    let Some(mut archives) = hd() else { return };
    let err = oag_raceplay::rumble_library::disc_pattern(&mut archives, "No_Such_Pattern")
        .expect_err("not on the disc");
    assert!(err.to_string().contains("No_Such_Pattern"), "{err}");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_has_no_patterns_and_its_original_offers_none() {
    let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let mut opened =
        oag_source::title::open_source(&image.to_string_lossy(), Vec::new(), Vec::new())
            .expect("Pulse opens");
    assert!(
        oag_raceplay::rumble_library::disc_pattern(&mut opened.archives, "Missile_Fired_Rumble")
            .is_err()
    );
    assert_eq!(
        Mode::Original.law(oag_game::rumble::original_law(opened.title.rumble)),
        None
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu.iso"]
fn every_pattern_the_disc_numbers_is_fired_or_unfired_with_a_reason() {
    use oag_input::rumble::map::{UNFIRED, pattern_names};
    let Some(mut archives) = hd() else { return };
    let mut names: Vec<&str> = pattern_names()
        .chain(UNFIRED.iter().map(|(n, _)| *n))
        .collect();
    names.sort_unstable();
    let total = names.len();
    names.dedup();
    assert_eq!(names.len(), total, "a pattern is both fired and unfired");
    for name in &names {
        oag_raceplay::rumble_library::disc_pattern(&mut archives, name)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
    assert_eq!(
        names.len(),
        64,
        "ids 0..=64 span 65 slots with 64 files (id 11 has none)"
    );
}

/// 2048 ships HD's 69 files and Omega reads them; both resolve every mapped
/// pattern through the same loader, so the map plays 2048's own files
/// (inherited from Omega) and Omega's (`docs/gameplay/rumble.md`, "Provenance
/// per title").
#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg and data/extracted/vita/PCSF00007"]
fn omega_and_2048_resolve_every_mapped_pattern_through_the_same_loader() {
    let extracted = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    let omega = oag_testdata::image("omega-ps4-eu.pkg");
    let vita = extracted.join("vita/PCSF00007");
    let sources = [
        ("omega-ps4-eu.pkg", omega),
        ("2048 extracted", vita.exists().then_some(vita)),
    ];
    for (image, path) in sources {
        let Some(path) = path else {
            continue;
        };
        let mut opened =
            oag_source::title::open_source(&path.to_string_lossy(), Vec::new(), Vec::new())
                .unwrap_or_else(|e| panic!("{image}: {e:#}"));
        let mut missing = Vec::new();
        for name in oag_input::rumble::map::pattern_names() {
            if oag_raceplay::rumble_library::disc_pattern(&mut opened.archives, name).is_err() {
                missing.push(name);
            }
        }
        assert!(missing.is_empty(), "{image}: not found: {missing:?}");
        println!("{image}: all mapped patterns resolve");
    }
}
