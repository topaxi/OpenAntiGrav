//! Two mods that each hand out a pickup of their own in one race: the official mod
//! Slipstream (`oag.slipstream`) and the example Energy Turbo (`oag.energy-turbo`), both on
//! Pulse PSP EU's `16_Track` in a full field. Kinds follow module ids, so the energy turbo is
//! kind 0 and the slipstream kind 1. The pads hand out both (the slipstream by its odds, the
//! energy turbo in place of every second Turbo), each is used by its own mod, and neither
//! faults.
//!
//! **`#[ignore]`d and never run in CI**: it needs `data/images/pulse-psp-eu.chd`, both
//! submodules and the `extensions` feature:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --features extensions \
//!     --run-ignored only -E 'test(/slipstream_energy_turbo_ground_truth::/)' --no-capture
//! ```
#![cfg(feature = "extensions")]

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::Button;
use oag_raceplay as race;
use oag_tables::weapons::{ModKind, Weapon};

use crate::slipstream_ground_truth::source_of;

const TRACK: &str = "Data\\Environments\\16_Track\\track.vex";
const ENERGY_TURBO: &str = "assets/mods/examples/energy-turbo";
const SLIPSTREAM: &str = "assets/mods/official/slipstream";

fn throttle() -> PlayerInputs {
    let mut input = oag_gameplay::input::Input::new();
    input.begin_frame(Button::Cross.bit());
    input.begin_frame(Button::Cross.bit());
    PlayerInputs::single(oag_gameplay::InputSnapshot {
        buttons: input,
        ..oag_gameplay::InputSnapshot::new()
    })
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd, both mod submodules and --features extensions"]
fn both_mods_hand_out_and_use_their_own_pickups_in_one_race_with_no_fault() {
    let (Some(energy), Some(slip)) = (source_of(ENERGY_TURBO), source_of(SLIPSTREAM)) else {
        return;
    };
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(TRACK.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        extensions: vec![slip, energy],
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    assert_eq!(
        race.extension_reports().len(),
        2,
        "both mods are on: {:?}",
        loaded.report
    );
    let ships = usize::from(race.sim.world.ship_count);
    let mut carried: Vec<Option<Weapon>> = vec![None; ships];
    let mut handed = [0_u32; 2];
    let mut transits = 0_u32;
    let mut was_out = vec![false; ships];
    let drain_share = 0.008;
    let mut boost_ticks = 0_u32;
    let mut prior: Vec<f32> = race.sim.world.ships[..ships]
        .iter()
        .map(|ship| ship.physics.shield)
        .collect();
    for _ in 0..10_800 {
        race.tick(&throttle());
        for slot in 0..ships {
            let ship = &race.sim.world.ships[slot];
            let now = ship.pickup.weapon;
            if carried[slot].is_none()
                && let Some(Weapon::Extension(kind)) = now
            {
                handed[kind.index()] += 1;
            }
            let out = ship.transit.is_some();
            if out && !was_out[slot] {
                transits += 1;
            }
            was_out[slot] = out;
            let drain = ship.handling.dimensions.shield * drain_share;
            if now == Some(Weapon::Extension(ModKind::FIRST))
                && ((prior[slot] - ship.physics.shield) - drain).abs() < 1e-4
            {
                boost_ticks += 1;
            }
            carried[slot] = now;
            prior[slot] = ship.physics.shield;
        }
    }
    println!(
        "energy turbos handed {}, slipstreams handed {}; {transits} portal transits, {boost_ticks} energy boost ticks",
        handed[0], handed[1]
    );
    assert!(handed[0] > 0, "the energy turbo was handed out");
    assert!(handed[1] > 0, "the slipstream was handed out");
    assert!(transits > 0, "a slipstream was used by its mod");
    assert!(boost_ticks > 0, "an energy turbo was used by its mod");
    let reports = race.extension_reports();
    assert!(
        reports.iter().all(|r| r.faulted.is_none()),
        "no fault: {reports:?}"
    );
}
