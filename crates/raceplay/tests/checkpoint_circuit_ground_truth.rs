//! Checkpoint Circuit on a real race of its track file: the clock to the next checkpoint,
//! the drain past it, the pit lane's recharge, and an AI craft low on energy taking the
//! pit lane through `prefer-route` while one with energy to spare follows its coin.
//!
//! **`#[ignore]`d and never run in CI**: it needs `data/images/pulse-psp-eu.chd`, the
//! `assets/mods/examples` submodule and the `extensions` feature:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --features extensions \
//!     --run-ignored only -E 'test(/checkpoint_circuit_ground_truth::/)' --no-capture
//! ```
//!
//! The module is the committed build (`checkpoint-circuit/ext/checkpoint_circuit.wasm`),
//! loaded as the game loads it: its manifest through `oag-mod`, bound to its track file by
//! `[extension.track]`. Its numbers are the guest's own constants (chosen, not measured),
//! repeated here: a change there fails here by name.
#![cfg(feature = "extensions")]

use std::sync::Arc;

use oag_gameplay::PlayerInputs;
use oag_physics::CraftState;
use oag_raceplay as race;

const MOD: &str = "assets/mods/examples/checkpoint-circuit";
/// `CHECKPOINT_TICKS`, `DRAIN_SHARE`, `RECHARGE_SHARE`, `AI_LOW_SHARE` in the guest.
const CHECKPOINT_TICKS: u16 = 1200;
const DRAIN_SHARE: f32 = 1.0 / 240.0;
const RECHARGE_SHARE: f32 = 1.0 / 300.0;
const AI_LOW_SHARE: f32 = 0.4;
/// The guest's blob: `left: [u16; 16]`, `next: [u8; 16]`, `pit: [u8; 16]`.
const NEXT_AT: usize = 32;
const PIT_AT: usize = 48;
/// The pit lane's route as the race numbers it (`branching.route`, route index + 1).
const PIT_ROUTE: u16 = 1;

fn checkpoint_circuit() -> Option<(race::extensions::Source, std::path::PathBuf)> {
    let manifest = oag_testdata::exact(&format!("{MOD}/oag-mod.toml"))?;
    let module = oag_mod::Mod::open(manifest.parent().unwrap()).expect("the example mod opens");
    let extension = module.manifest.extension.clone().expect("[extension]");
    let track = extension.track.expect("an [extension.track] module");
    let wasm = module.read(&track.module).unwrap().expect("the module");
    let file = module.local_path(&track.track).expect("a folder mod");
    let source = race::extensions::Source {
        mod_id: module.manifest.id.clone(),
        module: track.module,
        titles: module.manifest.titles.clone(),
        api: extension.api,
        memory_pages: extension.memory_pages,
        state_bytes: extension.state_bytes,
        scope: race::extensions::Scope::Track(Some(file.clone())),
        read: Arc::new(move || Ok(wasm.clone())),
        strings: Arc::new(|_| Default::default()),
        icons: Vec::new(),
        effects: Vec::new(),
        files: Arc::new(|_| Err("no files".into())),
    };
    Some((source, file))
}

/// A Single Race of the track file: alone on the grid, or a full field with weapons off
/// (so nothing but the mod moves an AI craft's energy).
fn start(
    file: &std::path::Path,
    module: Option<&race::extensions::Source>,
    field: bool,
) -> race::Race {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd").expect("checked by the caller");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        foreign_track: Some(file.to_path_buf()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponents: field,
        no_driver_for_class: !field,
        weapons_override: field.then_some(false),
        extensions: module.into_iter().cloned().collect(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let race = race::Race::start(loaded.setup);
    assert_eq!(
        race.route_count(),
        1,
        "the pit lane is the track's one route"
    );
    race
}

fn ready() -> Option<(race::extensions::Source, std::path::PathBuf)> {
    oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    checkpoint_circuit()
}

fn blob(race: &race::Race) -> Vec<u8> {
    race.extension_state(0)
        .expect("the module's state")
        .to_vec()
}

fn left(blob: &[u8], slot: usize) -> u16 {
    u16::from_le_bytes([blob[2 * slot], blob[2 * slot + 1]])
}

fn shield(race: &race::Race, slot: usize) -> f32 {
    race.sim.world.ships[slot].physics.shield
}

fn shield_max(race: &race::Race, slot: usize) -> f32 {
    race.sim.world.ships[slot].handling.dimensions.shield
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn a_craft_that_never_reaches_its_checkpoint_drains_every_tick_past_zero_and_is_wrecked() {
    let Some((source, file)) = ready() else {
        return;
    };
    // Nobody flies the craft: it sits on the grid and never reaches checkpoint 1.
    let mut modded = start(&file, Some(&source), false);
    let mut vanilla = start(&file, None, false);
    assert!(modded.is_modded() && !vanilla.is_modded());
    let max = shield_max(&modded, 0);
    let step = max * DRAIN_SHARE;
    let mut zero_at = None;
    let mut wrecked_at = None;
    let mut drained = 0;
    for t in 0..4000_u32 {
        let before = shield(&modded, 0);
        modded.tick(&PlayerInputs::none());
        vanilla.tick(&PlayerInputs::none());
        assert_eq!(
            shield(&vanilla, 0),
            max,
            "tick {t}: nothing drains the vanilla craft"
        );
        let (b, after) = (blob(&modded), shield(&modded, 0));
        if zero_at.is_none() && b[NEXT_AT] != 0 && left(&b, 0) == 0 {
            zero_at = Some(t);
        }
        if modded.sim.world.ships[0].physics.craft_state != CraftState::Racing {
            wrecked_at = Some(t);
            println!("tick {t}: wrecked, energy {after:.3} of {max}");
            break;
        }
        match zero_at {
            None => assert_eq!(after, max, "tick {t}: no drain while the clock runs"),
            Some(_) => {
                assert!(
                    (before - after - step).abs() < 1e-3,
                    "tick {t}: lost {} past zero, the drain is {step}",
                    before - after
                );
                drained += 1;
            }
        }
    }
    let zero_at = zero_at.expect("the clock ran out");
    let wrecked_at = wrecked_at.expect("the drain wrecked the craft");
    println!(
        "clock out at tick {zero_at}, wrecked at {wrecked_at}: {drained} draining ticks of {step:.3} \
         from {max}"
    );
    let racing_from = zero_at + 1 - u32::from(CHECKPOINT_TICKS);
    assert!(racing_from > 0, "the clock starts at GO, not at load");
    // A full pool lasts 1 / DRAIN_SHARE ticks; the tick that empties it wrecks the craft.
    assert!(
        (wrecked_at - zero_at).abs_diff((1.0 / DRAIN_SHARE) as u32) <= 2,
        "{} ticks from zero to wrecked",
        wrecked_at - zero_at
    );
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn a_craft_that_reaches_its_checkpoint_in_time_has_its_clock_reset_and_loses_nothing() {
    let Some((source, file)) = ready() else {
        return;
    };
    // This project's driver flies the craft round: every checkpoint well inside 20 s.
    let mut modded = start(&file, Some(&source), false);
    let mut vanilla = start(&file, None, false);
    modded.set_autopilot(true);
    vanilla.set_autopilot(true);
    let mut reached = Vec::new();
    let mut lowest = CHECKPOINT_TICKS;
    let mut previous = blob(&modded);
    for t in 0..5000_u32 {
        modded.tick(&PlayerInputs::none());
        vanilla.tick(&PlayerInputs::none());
        assert_eq!(
            shield(&modded, 0).to_bits(),
            shield(&vanilla, 0).to_bits(),
            "tick {t}: the mod took nothing from a craft on time"
        );
        let b = blob(&modded);
        if b[NEXT_AT] != previous[NEXT_AT] && previous[NEXT_AT] != 0 {
            assert_eq!(
                left(&b, 0),
                CHECKPOINT_TICKS - 1,
                "tick {t}: checkpoint {} resets the clock",
                previous[NEXT_AT]
            );
            reached.push((t, previous[NEXT_AT], left(&previous, 0)));
        }
        if b[NEXT_AT] != 0 {
            lowest = lowest.min(left(&b, 0));
        }
        previous = b;
    }
    println!("checkpoints (tick, number, ticks left before): {reached:?}; lowest clock {lowest}");
    let numbers: Vec<u8> = reached.iter().map(|r| r.1).collect();
    assert!(
        numbers.starts_with(&[1, 2, 3, 4, 1]),
        "the gates in lap order, wrapping: {numbers:?}"
    );
    assert!(lowest > 0, "the clock never ran out");
}

/// A full field, weapons off, the person's craft flown by this project's driver; each
/// slot in `low` starts at just under the AI's pit threshold. Every AI slot's route each
/// tick, every slot's energy, and the guest's pit flags.
struct Run {
    routes: Vec<[u16; 8]>,
    visits: Vec<[u32; 8]>,
    shields: Vec<[f32; 8]>,
    pits: Vec<[u8; 8]>,
    max: [f32; 8],
}

fn run(
    file: &std::path::Path,
    module: Option<&race::extensions::Source>,
    low: &[usize],
    ticks: u32,
) -> Run {
    let mut race = start(file, module, true);
    race.set_autopilot(true);
    let max = std::array::from_fn(|slot| shield_max(&race, slot));
    for &slot in low {
        race.sim.world.ships[slot].physics.shield = max[slot] * (AI_LOW_SHARE - 0.01);
    }
    let mut out = Run {
        routes: Vec::new(),
        visits: Vec::new(),
        shields: Vec::new(),
        pits: Vec::new(),
        max,
    };
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
        let ships = &race.sim.world.ships;
        out.routes
            .push(std::array::from_fn(|s| ships[s].driver.branching.route));
        out.visits
            .push(std::array::from_fn(|s| ships[s].driver.branching.visits));
        out.shields
            .push(std::array::from_fn(|s| ships[s].physics.shield));
        let b = race.extension_state(0).map(<[u8]>::to_vec);
        out.pits.push(std::array::from_fn(|s| {
            b.as_ref().map_or(0, |b| b[PIT_AT + s])
        }));
    }
    out
}

/// Whether slot `slot` drove the pit route at any tick of `run` before `until`.
fn took_pit(run: &Run, slot: usize, until: usize) -> bool {
    run.routes[..until].iter().any(|r| r[slot] == PIT_ROUTE)
}

/// One lap and a bit: every craft meets the fork once.
const FIRST_FORK: usize = 2400;

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn a_craft_through_the_pit_lane_gains_energy_up_to_its_maximum() {
    let Some((source, file)) = ready() else {
        return;
    };
    let slot = 1;
    let run = run(&file, Some(&source), &[slot], FIRST_FORK as u32);
    let inside: Vec<usize> = (0..run.pits.len())
        .filter(|&t| run.pits[t][slot] != 0)
        .collect();
    assert!(
        !inside.is_empty(),
        "slot {slot} never entered the pit trigger"
    );
    let max = run.max[slot];
    let step = max * RECHARGE_SHARE;
    let (first, last) = (inside[0], *inside.last().unwrap());
    println!(
        "slot {slot} in the pit ticks {first}..={last}: energy {:.2} -> {:.2} of {max}",
        run.shields[first - 1][slot],
        run.shields[last][slot]
    );
    let mut gained = 0;
    for t in first + 1..=last {
        let (before, after) = (run.shields[t - 1][slot], run.shields[t][slot]);
        assert!(after <= max, "tick {t}: {after} past the maximum {max}");
        if before + step < max {
            assert!(
                (after - before - step).abs() < 1e-3,
                "tick {t}: gained {} in the pit, the recharge is {step}",
                after - before
            );
            gained += 1;
        }
    }
    assert!(gained > 30, "{gained} recharging ticks");
    assert_eq!(
        run.shields[last][slot], max,
        "capped at the maximum, exactly"
    );
    let out = (last + 1..run.shields.len()).find(|&t| run.pits[t][slot] == 0);
    if let Some(t) = out {
        assert!(run.shields[t][slot] <= max);
    }
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn a_low_ai_craft_takes_the_pit_route_and_a_full_one_follows_its_coin() {
    let Some((source, file)) = ready() else {
        return;
    };
    // Full energy: the module sends no prefer-route, so every AI craft's fork is its
    // coin's, tick for tick the vanilla race's.
    let vanilla = run(&file, None, &[], FIRST_FORK as u32);
    let full = run(&file, Some(&source), &[], FIRST_FORK as u32);
    for t in 0..FIRST_FORK {
        assert_eq!(full.routes[t], vanilla.routes[t], "tick {t}: routes");
        assert_eq!(full.visits[t], vanilla.visits[t], "tick {t}: coin draws");
    }
    let by_coin: Vec<bool> = (1..8).map(|s| took_pit(&vanilla, s, FIRST_FORK)).collect();
    println!("full energy, the coin's pit lane per AI slot 1..7: {by_coin:?}");
    let ring = (1..8)
        .find(|&s| !took_pit(&vanilla, s, FIRST_FORK))
        .expect("a craft whose coin keeps it on the main line");

    // The same slot low on energy: the module asks for the pit route and it is taken.
    let low = run(&file, Some(&source), &[ring], FIRST_FORK as u32);
    assert!(
        took_pit(&low, ring, FIRST_FORK),
        "slot {ring} low on energy stayed on the main line"
    );
    let entered = (0..FIRST_FORK)
        .find(|&t| low.routes[t][ring] == PIT_ROUTE)
        .unwrap();
    println!(
        "slot {ring} at {:.1} of {} energy: onto the pit route at tick {entered}",
        low.shields[0][ring], low.max[ring]
    );
    // The coin was still thrown: the same draws as the vanilla race up to the switch.
    assert_eq!(
        low.visits[entered][ring], vanilla.visits[entered][ring],
        "the coin's stream is unchanged"
    );
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn a_checkpoint_race_run_twice_hashes_the_same_every_tick() {
    let Some((source, file)) = ready() else {
        return;
    };
    let mut first = start(&file, Some(&source), true);
    let mut second = start(&file, Some(&source), true);
    for race in [&mut first, &mut second] {
        let max = shield_max(race, 2);
        race.sim.world.ships[2].physics.shield = max * 0.2;
    }
    for t in 0..1800 {
        first.tick(&PlayerInputs::none());
        second.tick(&PlayerInputs::none());
        assert_eq!(first.sim.state_hash(), second.sim.state_hash(), "tick {t}");
    }
}
