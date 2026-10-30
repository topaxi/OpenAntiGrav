//! Wall Penalty on a real circuit: a craft steered into a wall is slower a second
//! later with the curse than without it.
//!
//! **`#[ignore]`d and never run in CI**: it needs `data/images/pulse-psp-eu.chd`, the
//! `assets/mods/examples` submodule and the `extensions` feature:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --features extensions \
//!     --run-ignored only -E 'test(/wall_penalty_ground_truth::/)' --no-capture
//! ```
//!
//! The module is the committed build of the example mod
//! (`assets/mods/examples/wall-penalty/ext/wall_penalty.wasm`), loaded as the game loads
//! it: its manifest through `oag-mod`, its bytes through the race load. Two races,
//! identical inputs (throttle held from the start, the stick centred), one with the
//! mod. The craft goes straight on into the wall of the first bend at about 405 km/h
//! and scrapes along it. The races are the same until that first contact; from the
//! tick after, the cursed craft has a quarter of its thrust for 1.5 s, renewed while
//! it stays on the wall. Measured 2026-10-29 (Pulse PSP EU, Venom, Time Trial):
//!
//! | After the first contact | Cursed | Plain |
//! | --- | --- | --- |
//! | 0.25 s | 207.9 km/h | 255.7 km/h |
//! | 1 s | 50.7 km/h | 115.6 km/h |
//! | 2 s | 15.6 km/h | 81.7 km/h |
//! | 5 s | 46.8 km/h | 83.3 km/h |
//!
//! The AI is cursed by the same law: in a Single Race with the player idle on the
//! grid, the two races split at the first AI contact (slot 1, tick 2676, about
//! 400 km/h); a second later that craft is at 255.3 km/h against 410.7 km/h.
#![cfg(feature = "extensions")]

use std::sync::Arc;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

const TRACK: &str = "Data\\Environments\\16_Track\\track.vex";
const MOD: &str = "assets/mods/examples/wall-penalty";

fn held(stick_x: f32) -> oag_gameplay::InputSnapshot {
    let mut buttons = oag_gameplay::input::Input::new();
    buttons.begin_frame(oag_gameplay::input::Button::Cross.bit());
    buttons.begin_frame(oag_gameplay::input::Button::Cross.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        stick_x,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// The example mod's rules module, as `oag_game::mod_extensions::sources` builds it.
fn wall_penalty() -> Option<race::extensions::Source> {
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
        .expect("the committed module is in the mod");
    Some(race::extensions::Source {
        mod_id: module.manifest.id.clone(),
        module: rules.module,
        titles: module.manifest.titles.clone(),
        api: extension.api,
        memory_pages: extension.memory_pages,
        state_bytes: extension.state_bytes,
        scope: race::extensions::Scope::Rules,
        read: Arc::new(move || Ok(wasm.clone())),
    })
}

fn race_with(extensions: Vec<race::extensions::Source>) -> Option<race::Race> {
    race_in(oag_race::Mode::TimeTrial, extensions)
}

fn race_in(mode: oag_race::Mode, extensions: Vec<race::extensions::Source>) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(TRACK.to_string()),
        class: "VENOM".to_string(),
        mode,
        extensions,
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup))
}

/// The player's speed, km/h, every tick for `ticks` ticks, throttle held from the start.
fn speeds(mut race: race::Race, ticks: usize) -> Vec<f32> {
    (0..ticks)
        .map(|_| {
            race.tick(&PlayerInputs::single(held(0.0)));
            race.ship().physics.body.linear_velocity.length() * oag_core::math::SPEED_TO_KMH
        })
        .collect()
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn a_craft_scraping_a_wall_is_slower_with_the_curse() {
    let Some(source) = wall_penalty() else { return };
    let Some(cursed) = race_with(vec![source]) else {
        return;
    };
    assert!(cursed.is_modded(), "the module loaded");
    let plain = race_with(Vec::new()).unwrap();
    const TICKS: usize = 900;
    let (cursed, plain) = (speeds(cursed, TICKS), speeds(plain, TICKS));
    let split = (0..TICKS)
        .find(|&t| cursed[t] != plain[t])
        .expect("the craft never touched a wall");
    for after in [0, 15, 30, 60, 90, 120, 180, 240, 300] {
        let t = (split + after).min(TICKS - 1);
        println!(
            "tick {t} (split +{after}): cursed {:.1} km/h, plain {:.1} km/h",
            cursed[t], plain[t]
        );
    }
    let t = split + 60;
    assert!(
        cursed[t] < plain[t] * 0.6,
        "a second after the first contact the cursed craft must be well down: {} against {}",
        cursed[t],
        plain[t]
    );
}

/// Every craft's speed, km/h, every tick for `ticks` ticks, the player idle on the grid.
fn field_speeds(mut race: race::Race, ticks: usize) -> Vec<[f32; oag_gameplay::MAX_SHIPS]> {
    (0..ticks)
        .map(|_| {
            race.tick(&PlayerInputs::none());
            std::array::from_fn(|slot| {
                race.sim.world.ships[slot]
                    .physics
                    .body
                    .linear_velocity
                    .length()
                    * oag_core::math::SPEED_TO_KMH
            })
        })
        .collect()
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn an_ai_craft_touching_a_wall_is_cursed_too() {
    let Some(source) = wall_penalty() else { return };
    let Some(cursed) = race_in(oag_race::Mode::SingleRace, vec![source]) else {
        return;
    };
    let plain = race_in(oag_race::Mode::SingleRace, Vec::new()).unwrap();
    const TICKS: usize = 3600;
    let (cursed, plain) = (field_speeds(cursed, TICKS), field_speeds(plain, TICKS));
    let (split, slot) = (0..TICKS)
        .find_map(|t| {
            (0..oag_gameplay::MAX_SHIPS)
                .find(|&s| cursed[t][s] != plain[t][s])
                .map(|s| (t, s))
        })
        .expect("no AI craft touched a wall");
    for after in [0, 15, 30, 60, 90] {
        let t = (split + after).min(TICKS - 1);
        println!(
            "slot {slot} tick {t} (split +{after}): cursed {:.1} km/h, plain {:.1} km/h",
            cursed[t][slot], plain[t][slot]
        );
    }
    assert_ne!(
        slot, 0,
        "the player sat on the grid; the first contact is an AI craft's"
    );
    let t = split + 60;
    assert!(
        cursed[t][slot] < plain[t][slot],
        "the AI craft is slower a second after its first contact: {} against {}",
        cursed[t][slot],
        plain[t][slot]
    );
}
