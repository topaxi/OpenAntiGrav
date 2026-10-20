//! Scratch sweep: what the collision cage does to a de Konstruct Black field.
//!
//! `#[ignore]`d and gated on `OAG_SWEEP`, printing rather than asserting. It
//! races `ai_dekonstruct_black_ground_truth`'s forward cells (four classes,
//! seeds 1 to 3) over `perturb::SCALES`, once with the circuit's cage and once
//! without it, and prints for each race the craft destroyed, the shield each
//! lost to walls, rolls and the rest (weapons), and every rescue by cause.
//!
//! ```sh
//! OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay \
//!     --run-ignored all cage_effect_sweep --no-capture
//! ```

use std::collections::BTreeMap;

use oag_gameplay::PlayerInputs;
use oag_physics::CraftState;
use oag_raceplay as race;
use oag_raceplay::catalogue;

use crate::in_parallel;
use crate::perturb;

const TICKS: u64 = 18_000;

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn entry(reversed: bool) -> Option<String> {
    let image = image()?;
    let mut archives = oag_pulse::open(&image.display().to_string()).ok()?;
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .ok()?;
    let definition = oag_tables::fexml::expand(&blob).ok()?;
    catalogue::tracks(&definition)
        .into_iter()
        .find(|track| track.location.ends_with("05_Track") && track.reversed == reversed)
        .map(|track| track.entry_name())
}

/// One craft's race.
#[derive(Debug, Default, Clone)]
struct Craft {
    dead_tick: Option<u64>,
    wall: f32,
    roll: f32,
    other: f32,
    rescues: BTreeMap<&'static str, u32>,
    /// Ticks spent in `Eliminated` with the state timer run out (the
    /// original's state 6), and how many of those the cage touched.
    state6_ticks: u32,
    /// The tick and cause of the latest rescue.
    last_rescue: Option<(u64, &'static str)>,
    /// At death: ticks since the latest rescue and its cause, and the
    /// driver's line index.
    death: Option<(Option<(u64, &'static str)>, u32)>,
}

#[derive(Debug, Clone)]
struct Cell {
    class: &'static str,
    seed: u64,
    eps: f32,
    cage: bool,
}

fn run(cell: &Cell) -> Vec<Craft> {
    let image = image().expect("image");
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: cell.class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry(false).expect("05_Track")),
        seed: Some(cell.seed),
        ..race::Options::default()
    })
    .expect("loading");
    let mut setup = loaded.setup;
    perturb::apply(&mut setup, cell.eps);
    if !cell.cage {
        setup.spline = setup.spline.clone().with_cage(None);
    }
    let mut race = race::Race::start(setup);
    race.sim.world.ships[0].active = false;
    let count = race.ship_count() as usize;
    let mut crafts = vec![Craft::default(); count];
    let mut prev_wall = vec![0.0f32; count];
    let mut prev_roll = vec![0.0f32; count];
    let mut prev_resp = vec![0u32; count];
    for tick in 1..=TICKS {
        let before: Vec<f32> = (0..count)
            .map(|s| race.sim.world.ships[s].physics.shield)
            .collect();
        race.tick(&PlayerInputs::none());
        for s in 1..count {
            let ship = &race.sim.world.ships[s];
            let wall = race.wall_shield_charged_of(s);
            let roll = race.roll_shield_spent_of(s);
            let resp = race.respawns_of(s);
            let (dwall, droll) = (wall - prev_wall[s], roll - prev_roll[s]);
            if resp != prev_resp[s] {
                let cause = race
                    .last_respawn_cause_of(s)
                    .map_or("unknown", |cause| cause.label());
                *crafts[s].rescues.entry(cause).or_default() += resp - prev_resp[s];
                crafts[s].last_rescue = Some((tick, cause));
            }
            prev_wall[s] = wall;
            prev_roll[s] = roll;
            prev_resp[s] = resp;
            let c = &mut crafts[s];
            let drop = before[s] - ship.physics.shield;
            if c.dead_tick.is_none() && drop > 0.0 {
                c.wall += dwall.min(drop);
                c.roll += droll.min(drop);
                c.other += (drop - dwall - droll).max(0.0);
            }
            if ship.physics.craft_state != CraftState::Racing && c.dead_tick.is_none() {
                c.dead_tick = Some(tick);
                c.death = Some((
                    c.last_rescue.map(|(at, cause)| (tick - at, cause)),
                    ship.driver.index,
                ));
            }
            if ship.physics.craft_state == CraftState::Eliminated && ship.physics.state_timer <= 0.0
            {
                c.state6_ticks += 1;
            }
        }
    }
    crafts
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn cage_effect_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() || image().is_none() {
        return;
    }
    let mut cells = Vec::new();
    for class in ["VENOM", "FLASH", "RAPIER", "PHANTOM"] {
        for seed in 1..=3 {
            for eps in perturb::SCALES {
                for cage in [false, true] {
                    cells.push(Cell {
                        class,
                        seed,
                        eps,
                        cage,
                    });
                }
            }
        }
    }
    let width = std::env::var("OAG_SWEEP_WIDTH")
        .ok()
        .and_then(|w| w.parse().ok())
        .unwrap_or(12);
    let results = in_parallel::map(&cells, width, run);
    for (cell, crafts) in cells.iter().zip(&results) {
        let dead: Vec<String> = crafts
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(s, c)| {
                c.dead_tick.map(|t| {
                    format!(
                        "s{s}@{t} wall {:.0} roll {:.0} other {:.0} resc {:?} death {:?}",
                        c.wall, c.roll, c.other, c.rescues, c.death
                    )
                })
            })
            .collect();
        let mut rescues: BTreeMap<&str, u32> = BTreeMap::new();
        for c in crafts.iter().skip(1) {
            for (k, v) in &c.rescues {
                *rescues.entry(k).or_default() += v;
            }
        }
        let wall: f32 = crafts.iter().skip(1).map(|c| c.wall).sum();
        let other: f32 = crafts.iter().skip(1).map(|c| c.other).sum();
        println!(
            "ROW {} {} {:e} cage={} dead {} wall {:.0} other {:.0} rescues {:?} | {}",
            cell.class,
            cell.seed,
            cell.eps,
            cell.cage,
            dead.len(),
            wall,
            other,
            rescues,
            dead.join("; ")
        );
    }
}
