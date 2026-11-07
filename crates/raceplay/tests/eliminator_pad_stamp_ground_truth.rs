//! The stamp a weapon pad takes in an Eliminator, per title, off each real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^eliminator_pad_stamp_ground_truth::/)'
//! ```
//!
//! Pulse stamps `elimination_refresh_time` (`0.05` s). HD, Fury, 2048 and Omega author `0.15` and
//! never stamp in Eliminator, so the race carries `0.0` (`docs/gameplay/pickups.md`, "Eliminator
//! and the pads, per title"). Every other mode stamps `refresh_time` on all of them.

use oag_race::Mode;
use oag_raceplay as race;

fn refresh(source: &std::path::Path, track: Option<&str>, mode: Mode) -> f32 {
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        mode,
        track: track.map(str::to_string),
        seed: Some(1),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("loading {mode:?}: {e:#}"));
    let refresh = loaded.setup.weapon_pad_refresh;
    if mode == Mode::Eliminator {
        assert!(
            !race::Race::start(loaded.setup).weapon_pads().is_empty(),
            "the circuit has pads, so the stamp is live"
        );
    }
    refresh
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_stamps_the_elimination_figure() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    assert_eq!(refresh(&image, None, Mode::Eliminator), 0.05);
    assert_eq!(refresh(&image, None, Mode::SingleRace), 0.55);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_never_stamps_a_pad_in_an_eliminator() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    assert_eq!(refresh(&image, None, Mode::Eliminator), 0.0);
    assert_eq!(refresh(&image, None, Mode::SingleRace), 0.75);
}

#[test]
#[ignore = "needs the extracted package under data/extracted/vita/"]
fn vita_2048_never_stamps_a_pad_in_an_eliminator() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let track = r"Data\art\published\environments\sol\track.vex";
    assert_eq!(refresh(&source, Some(track), Mode::Eliminator), 0.0);
    assert_eq!(refresh(&source, Some(track), Mode::SingleRace), 0.75);
}

#[test]
#[ignore = "needs the extracted package under data/extracted/ps4/"]
fn omega_never_stamps_a_pad_in_an_eliminator() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let track = r"data\environments2048\sol\track.vex";
    assert_eq!(refresh(&source, Some(track), Mode::Eliminator), 0.0);
    assert_eq!(refresh(&source, Some(track), Mode::SingleRace), 0.75);
}
