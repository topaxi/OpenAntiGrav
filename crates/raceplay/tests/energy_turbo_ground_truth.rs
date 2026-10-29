//! Energy Turbo on a real circuit: holding fire with the turbo boosts and costs shield,
//! an absorb gives back at most half of what was spent, a speed pad fills the pool
//! instead of boosting, and a craft without the turbo is untouched.
//!
//! **`#[ignore]`d and never run in CI**: it needs `data/images/pulse-psp-eu.chd`, the
//! `assets/mods/examples` submodule and the `extensions` feature:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --features extensions \
//!     --run-ignored only -E 'test(/energy_turbo_ground_truth::/)' --no-capture
//! ```
//!
//! Single Race (weapons and damage on) with an empty grid, so two races differ only by
//! the player's craft. The Turbo is put in the craft's hands at the start, which is what a
//! weapon pad would do. This project's driver flies the craft (`Race::set_autopilot`);
//! fire and absorb are pressed by the script.
#![cfg(feature = "extensions")]

use std::sync::Arc;

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::Button;
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

const TRACK: &str = "Data\\Environments\\16_Track\\track.vex";
const MOD: &str = "assets/mods/examples/energy-turbo";
/// The mod's own payback share (`guest/src/lib.rs`).
const PAYBACK: f32 = 0.5;

fn energy_turbo() -> Option<race::extensions::Source> {
    let manifest = oag_testdata::exact(&format!("{MOD}/oag-mod.toml"))?;
    let module = oag_mod::Mod::open(manifest.parent().unwrap()).expect("the example mod opens");
    let extension = module
        .manifest
        .extension
        .clone()
        .expect("it declares [extension]");
    let rules = extension.rules.expect("a rules module");
    let wasm = module
        .read(&rules.module)
        .unwrap()
        .expect("the committed module");
    Some(race::extensions::Source {
        mod_id: module.manifest.id.clone(),
        module: rules.module,
        titles: module.manifest.titles.clone(),
        api: extension.api,
        memory_pages: extension.memory_pages,
        state_bytes: extension.state_bytes,
        read: Arc::new(move || Ok(wasm.clone())),
    })
}

/// A Single Race with nobody else on the grid; `turbo` hands the player a Turbo and
/// starts it at half a pool, so a gain shows.
fn race_with(module: Option<&race::extensions::Source>, turbo: bool) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(TRACK.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        no_driver_for_class: true,
        extensions: module.into_iter().cloned().collect(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    // This project's driver steers (a scripted stick finds the first bend's wall); the
    // buttons below still reach the pickup and the mod.
    race.set_autopilot(true);
    let ship = &mut race.sim.world.ships[0];
    ship.physics.shield = ship.handling.dimensions.shield * 0.5;
    if turbo {
        ship.pickup.weapon = Some(Weapon::Turbo);
    }
    Some(race)
}

fn held(buttons: &[Button], pressed: &[Button]) -> PlayerInputs {
    let mut input = oag_gameplay::input::Input::new();
    let held = buttons.iter().fold(0, |bits, b| bits | b.bit());
    let pressed_bits = pressed.iter().fold(0, |bits, b| bits | b.bit());
    input.begin_frame(held & !pressed_bits);
    input.begin_frame(held);
    PlayerInputs::single(oag_gameplay::InputSnapshot {
        buttons: input,
        ..oag_gameplay::InputSnapshot::new()
    })
}

struct Sample {
    kmh: f32,
    shield: f32,
}

fn sample(race: &race::Race) -> Sample {
    let ship = &race.sim.world.ships[0];
    Sample {
        kmh: ship.physics.body.linear_velocity.length() * oag_core::math::SPEED_TO_KMH,
        shield: ship.physics.shield,
    }
}

/// Throttle every tick; fire held over `fire`; an absorb pressed on `absorb`.
fn drive(
    race: &mut race::Race,
    ticks: std::ops::Range<u32>,
    fire: std::ops::Range<u32>,
    absorb: Option<u32>,
) -> Vec<Sample> {
    ticks
        .map(|t| {
            let mut buttons = vec![Button::Cross];
            let mut pressed = Vec::new();
            if fire.contains(&t) {
                buttons.push(Button::Square);
                if t == fire.start {
                    pressed.push(Button::Square);
                }
            }
            if absorb == Some(t) {
                buttons.push(Button::Circle);
                pressed.push(Button::Circle);
            }
            race.tick(&held(&buttons, &pressed));
            sample(race)
        })
        .collect()
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn holding_fire_boosts_on_shield_and_an_absorb_pays_back_at_most_half() {
    let Some(source) = energy_turbo() else { return };
    let Some(mut boosted) = race_with(Some(&source), true) else {
        return;
    };
    let mut carried = race_with(Some(&source), true).unwrap();
    assert!(boosted.is_modded());
    const FIRE: std::ops::Range<u32> = 360..480;
    let a = drive(&mut boosted, 0..480, FIRE, None);
    let b = drive(&mut carried, 0..480, 0..0, None);
    for t in [359, 390, 420, 479] {
        let t = t as usize;
        println!(
            "tick {t}: fire held {:.1} km/h shield {:.2} / carried {:.1} km/h shield {:.2}",
            a[t].kmh, a[t].shield, b[t].kmh, b[t].shield
        );
    }
    assert_eq!(a[359].kmh, b[359].kmh, "the same race until fire is held");
    assert!(a[479].kmh > b[479].kmh, "the boost is faster");
    assert!(a[479].shield < b[479].shield, "and paid for in shield");
    assert_eq!(
        boosted.sim.world.ships[0].pickup.weapon,
        Some(Weapon::Turbo),
        "firing keeps it"
    );

    // What the boost cost, read off the pool: nothing else touched it (no hit, no
    // damaging contact on this stretch). The drain stops at the mod's floor, before the
    // 120 held ticks are up.
    let spent = a[359].shield - a[479].shield;
    let before = sample(&boosted).shield;
    let after = drive(&mut boosted, 480..481, 0..0, Some(480))[0].shield;
    let gained = after - before;
    println!("spent {spent:.2}, absorb gave back {gained:.2} (pool {before:.2} -> {after:.2})");
    assert_eq!(
        boosted.sim.world.ships[0].pickup.weapon, None,
        "the absorb takes it"
    );
    assert!(
        gained > 0.0 && gained <= spent * PAYBACK + 1e-3,
        "{gained} against {spent}"
    );
    assert!(
        (gained - spent * PAYBACK).abs() < 1e-3,
        "exactly half: {gained} of {spent}"
    );
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn a_carried_turbo_turns_a_speed_pad_into_energy_and_no_turbo_is_untouched() {
    let Some(source) = energy_turbo() else { return };
    let Some(mut modded) = race_with(Some(&source), true) else {
        return;
    };
    let mut vanilla = race_with(None, true).unwrap();
    let a = drive(&mut modded, 0..1200, 0..0, None);
    let b = drive(&mut vanilla, 0..1200, 0..0, None);
    let split = (0..a.len())
        .find(|&t| a[t].kmh != b[t].kmh || a[t].shield != b[t].shield)
        .expect("the craft crossed no speed pad");
    for after in [0, 30, 60] {
        let t = split + after;
        println!(
            "tick {t} (pad +{after}): carried {:.1} km/h shield {:.2} / vanilla {:.1} km/h shield {:.2}",
            a[t].kmh, a[t].shield, b[t].kmh, b[t].shield
        );
    }
    let t = split + 30;
    assert!(a[t].kmh < b[t].kmh, "no boost from the pad");
    assert!(a[t].shield > b[t].shield, "energy instead");

    // Without the turbo the mod does nothing to the craft at all.
    let mut bare = race_with(Some(&source), false).unwrap();
    let mut plain = race_with(None, false).unwrap();
    let c = drive(&mut bare, 0..1200, 0..0, None);
    let d = drive(&mut plain, 0..1200, 0..0, None);
    assert!(
        c.iter()
            .zip(&d)
            .all(|(c, d)| c.kmh == d.kmh && c.shield == d.shield),
        "a craft without the turbo flies the vanilla race"
    );
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn an_energy_turbo_race_run_twice_hashes_the_same_every_tick() {
    let Some(source) = energy_turbo() else { return };
    let Some(mut first) = race_with(Some(&source), true) else {
        return;
    };
    let mut second = race_with(Some(&source), true).unwrap();
    for t in 0..900 {
        let mut buttons = vec![Button::Cross];
        let mut pressed = Vec::new();
        if (360..480).contains(&t) {
            buttons.push(Button::Square);
        }
        if t == 360 {
            pressed.push(Button::Square);
        }
        if t == 600 {
            buttons.push(Button::Circle);
            pressed.push(Button::Circle);
        }
        let inputs = held(&buttons, &pressed);
        first.tick(&inputs);
        second.tick(&inputs);
        assert_eq!(first.sim.state_hash(), second.sim.state_hash(), "tick {t}");
    }
}
