//! Two LeachBeams live at once on a real disc, each draining its own victim.
//!
//! `#[ignore]`d and never run in CI: it needs a disc image (`just test-data`).
//! Every title that ships a LeachBeam runs a pool of eight
//! (`oag_title::weapons::Weapons::leach_beams`; 2048 and Omega measure eight,
//! Pulse and HD measure one and take eight as a chosen divergence), so two
//! beams from two different shooters stay up together and both victims lose
//! shield. Dropping the pool (a single `Option`, or a limit of one) arms only
//! the first and fails here.

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;

const WARM_UP_TICKS: u64 = oag_race::COUNTDOWN_TICKS + 60;

fn throttle() -> oag_gameplay::InputSnapshot {
    let mut buttons = Input::new();
    buttons.begin_frame(Button::Cross.bit());
    buttons.begin_frame(Button::Cross.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

fn two_beams(options: race::Options) {
    let loaded = race::load(&options).expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(throttle()));
    }
    let shield = |race: &race::Race, slot: usize| race.sim.world.ships[slot].physics.shield;
    let before = (shield(&race, 1), shield(&race, 3));
    assert!(race.force_leach_lock(0, 1), "the title authors a LeachBeam");
    assert!(
        race.force_leach_lock(2, 3),
        "a second beam must arm beside the first"
    );
    assert_eq!(race.sim.world.leach_beams_live(), 2);
    for _ in 0..120 {
        race.tick(&PlayerInputs::single(throttle()));
    }
    println!(
        "victim shields {before:?} -> ({}, {})",
        shield(&race, 1),
        shield(&race, 3)
    );
    assert!(shield(&race, 1) < before.0, "victim 1 was not drained");
    assert!(shield(&race, 3) < before.1, "victim 3 was not drained");
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulse_holds_two_beams_and_drains_both_victims() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    two_beams(race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        team: Some("Assegai".to_string()),
        ..race::Options::default()
    });
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_holds_two_beams_and_drains_both_victims() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    two_beams(race::Options {
        source: image.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    });
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_2048_holds_two_beams_and_drains_both_victims() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    two_beams(race::Options {
        source: path.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    });
}
