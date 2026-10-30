//! Energy Turbo on a real circuit: holding fire with the turbo boosts and costs shield,
//! an absorb gives back at most half of what was spent, a speed pad fills the pool
//! instead of boosting, a craft without the turbo is untouched, and in a full field the
//! weapon pads hand out the mod's pickup in place of every second Turbo and the AI
//! boosts with it.
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
//! the player's craft. The mod's pickup (`Weapon::Extension`) is put in the craft's hands
//! at the start, which is what a weapon pad does under the mod's `pickup-roll`. This project's driver flies the craft (`Race::set_autopilot`);
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
        scope: race::extensions::Scope::Rules,
        read: Arc::new(move || Ok(wasm.clone())),
        strings: Arc::new(|_| Default::default()),
        icons: Vec::new(),
    })
}

/// A Single Race with nobody else on the grid, or a full one (`field`); `turbo` hands
/// the player the energy turbo and starts it at half a pool, so a gain shows.
fn race_on(
    module: Option<&race::extensions::Source>,
    turbo: bool,
    field: bool,
) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(TRACK.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        no_driver_for_class: !field,
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
        ship.pickup.weapon = Some(Weapon::Extension);
    }
    Some(race)
}

fn race_with(module: Option<&race::extensions::Source>, turbo: bool) -> Option<race::Race> {
    race_on(module, turbo, false)
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
    for t in [359, 380, 400, 420, 479] {
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
        Some(Weapon::Extension),
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

/// The mod's own numbers (`guest/src/lib.rs`): every second Turbo rolled, the drain.
const ROLL_EVERY: u32 = 2;
const DRAIN_SHARE: f32 = 0.008;

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn in_a_full_field_every_second_turbo_is_the_mods_and_the_ai_boosts_on_it() {
    let Some(source) = energy_turbo() else { return };
    let Some(mut race) = race_on(Some(&source), false, true) else {
        return;
    };
    let ships = usize::from(race.sim.world.ship_count);
    let mut carried: Vec<Option<Weapon>> = vec![None; ships];
    let (mut turbos, mut energy) = (0_u32, 0_u32);
    let mut boost_ticks = 0_u32;
    // What the pads handed the player, whose pickup is spent the tick it arrives (the
    // driver flying it fires nothing), so it keeps drawing.
    let mut player: Vec<Weapon> = Vec::new();
    let mut prior: Vec<f32> = race.sim.world.ships[..ships]
        .iter()
        .map(|ship| ship.physics.shield)
        .collect();
    for _ in 0..10_800 {
        race.tick(&held(&[Button::Cross], &[]));
        if let Some(weapon) = race.sim.world.ships[0].pickup.take() {
            player.push(weapon);
            match weapon {
                Weapon::Turbo => turbos += 1,
                Weapon::Extension => energy += 1,
                _ => {}
            }
        }
        for slot in 1..ships {
            let ship = &race.sim.world.ships[slot];
            let now = ship.pickup.weapon;
            if carried[slot].is_none() {
                match now {
                    Some(Weapon::Turbo) => turbos += 1,
                    Some(Weapon::Extension) => energy += 1,
                    _ => {}
                }
            }
            // A boosting tick costs exactly the drain share and nothing else touches
            // the pool on most ticks: a drop of that size while carrying it is a boost.
            let drop = prior[slot] - ship.physics.shield;
            let drain = ship.handling.dimensions.shield * DRAIN_SHARE;
            if now == Some(Weapon::Extension) && (drop - drain).abs() < 1e-4 {
                boost_ticks += 1;
            }
            carried[slot] = now;
            prior[slot] = ship.physics.shield;
        }
    }
    println!(
        "in 180 s: {turbos} Turbos and {energy} energy turbos handed out; {boost_ticks} AI boost ticks; the player drew {player:?}"
    );
    assert!(
        turbos > 0 && energy > 0,
        "{turbos} Turbos, {energy} energy turbos"
    );
    // The count is race-wide over every weapon pad's Turbo draw, so the two can differ
    // by one at most.
    assert!(
        turbos.abs_diff(energy * (ROLL_EVERY - 1)) <= 1,
        "every second Turbo: {turbos} against {energy}"
    );
    assert!(boost_ticks > 0, "no AI craft boosted on the energy turbo");
    // The player's own draws are printed, not asserted: one grant path serves every
    // slot (`Race::test_weapon_pads`), and whether the human column's odds deal the
    // player a Turbo in one race is luck of the race (one energy turbo at x1.6, none
    // at x1.9).
}
