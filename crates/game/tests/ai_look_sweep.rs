//! What `Tuning::look_speed` and `Tuning::rate_gain` are worth on the disc's
//! own circuits, kept in the tree the way `ai_span_sweep.rs` keeps
//! `sweep_curvature_span`: `#[ignore]`d, gated on an env var, never run in
//! CI or in `just test-data`, and re-runnable the next time either constant
//! is worth re-checking against a real circuit.
//!
//! A lone Ace, twelve forward circuits, 18,000 ticks, shield (end and
//! per-lap) - plus the two committed field fixtures
//! (`lap_times_ground_truth::clocks`'s and
//! `opponent_weapons_ground_truth::single_race`'s own setups) so a value that
//! is clean on all twelve *lone* circuits cannot hide a craft wedging in
//! traffic. See `Tuning::look_speed`'s own doc comment for the table this
//! produced and the criterion it was chosen with.
//!
//! ```sh
//! OAG_SWEEP_LOOK=0.35,0.30,0.25,0.20 OAG_REQUIRE_GAME_DATA=1 \
//!   cargo nextest run --release -p oag-game --run-ignored all \
//!   sweep_look_speed --no-capture
//! OAG_SWEEP_GAIN=5.0,10.0,20.0 OAG_REQUIRE_GAME_DATA=1 \
//!   cargo nextest run --release -p oag-game --run-ignored all \
//!   sweep_rate_gain --no-capture
//! ```

use std::path::PathBuf;

use oag_game::{catalogue, race};

const TICKS: u64 = 18_000;
const LONE: usize = 1;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

#[derive(Debug, Default, Clone)]
struct Solo {
    best: Option<u64>,
    respawns: u32,
    laps: u32,
    shield: f32,
    per_lap: Vec<f32>,
}

fn solo_on(track: &str, tuning: oag_ai::Tuning) -> Option<Solo> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .ok()?;
    let mut race = race::Race::start(loaded.setup);
    race.set_ai_tuning(tuning);
    for slot in 2..8 {
        race.world.ships[slot].active = false;
    }
    race.world.ships[0].active = false;

    let mut best: Option<u64> = None;
    let mut lap = race.world.ships[LONE].standing.lap;
    let mut started = 0u64;
    let mut recovered_this_lap = false;
    let mut per_lap = Vec::new();
    let mut shield_at_lap = race.world.ships[LONE].physics.shield;
    for tick in 0..TICKS {
        let before = race.respawns_of(LONE);
        race.tick(&oag_gameplay::InputSnapshot::default());
        if race.respawns_of(LONE) != before {
            recovered_this_lap = true;
        }
        let now = race.world.ships[LONE].standing.lap;
        if now != lap {
            let shield = race.world.ships[LONE].physics.shield;
            per_lap.push(shield_at_lap - shield);
            shield_at_lap = shield;
            if lap > 1 && !recovered_this_lap {
                let taken = tick - started;
                best = Some(best.map_or(taken, |held: u64| held.min(taken)));
            }
            started = tick;
            lap = now;
            recovered_this_lap = false;
        }
    }

    Some(Solo {
        best,
        respawns: race.respawns_of(LONE),
        laps: lap,
        shield: race.world.ships[LONE].physics.shield,
        per_lap,
    })
}

fn circuits() -> Vec<(String, String)> {
    let Some(image) = image() else {
        return Vec::new();
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_formats::fexml::expand(&blob).expect("expanding it");

    catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| !track.reversed)
        .map(|track| (track.id.clone(), track.entry_name()))
        .collect()
}

#[derive(Debug, Default, Clone, Copy)]
struct FieldRun {
    lapped: usize,
    untimed: usize,
    mean: f32,
    worst: f32,
}

fn field_run(measured: oag_ai::Tuning) -> Option<FieldRun> {
    let image = image()?;
    let tuned = |level: oag_ai::Difficulty| level.tune(&measured);

    let lap_times = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some("Data\\Environments\\16_Track\\track.vex".to_string()),
        ..race::Options::default()
    })
    .ok()?;
    let mut race = race::Race::start(lap_times.setup);
    race.set_ai_tuning(tuned(oag_ai::Difficulty::Ace));
    for _ in 0..9_000 {
        race.tick(&oag_gameplay::InputSnapshot::default());
    }
    let lapped: Vec<usize> = (1..usize::from(race.ship_count()))
        .filter(|&slot| race.world.ships[slot].standing.lap >= 2)
        .collect();
    let untimed = lapped
        .iter()
        .filter(|&&slot| race.world.ships[slot].standing.best_lap_ticks.is_none())
        .count();
    let lapped = lapped.len();

    let options = race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    };
    let level = options.difficulty;
    let mines = race::load(&options).ok()?;
    let mut race = race::Race::start(mines.setup);
    race.set_ai_tuning(tuned(level));
    let opponents = 1..usize::from(race.ship_count());
    let full = race.world.ships[1].handling.dimensions.shield;
    for _ in 0..3_600 {
        race.tick(&oag_gameplay::InputSnapshot::default());
    }
    let mean = opponents
        .clone()
        .map(|slot| race.world.ships[slot].physics.shield)
        .sum::<f32>()
        / opponents.len() as f32
        / full;
    let worst = opponents
        .map(|slot| race.world.ships[slot].physics.shield)
        .fold(f32::INFINITY, f32::min)
        / full;

    Some(FieldRun {
        lapped,
        untimed,
        mean,
        worst,
    })
}

fn run_sweep(label: &str, variants: Vec<(String, oag_ai::Tuning)>) {
    let circuits = circuits();
    if circuits.is_empty() {
        return;
    }
    for (name, tuning) in variants {
        let laps: Vec<(String, Solo)> = circuits
            .iter()
            .filter_map(|(id, entry)| solo_on(entry, tuning).map(|solo| (id.clone(), solo)))
            .collect();

        let mut report = format!(
            "\n=== {label} {name} ===\ncircuit    end     respawns  laps  best     per-lap shield\n"
        );
        for (id, solo) in &laps {
            let best = solo
                .best
                .map_or_else(|| "-".to_string(), |b| format!("{:.1}s", b as f32 / 60.0));
            let per_lap: Vec<String> = solo
                .per_lap
                .iter()
                .map(|cost| format!("{cost:.2}"))
                .collect();
            report.push_str(&format!(
                "{id:<10} {:<7.2} {:<9} {:<5} {best:<8} {}\n",
                solo.shield,
                solo.respawns,
                solo.laps,
                per_lap.join(" ")
            ));
        }
        let total: f32 = laps.iter().map(|(_, solo)| solo.shield).sum();
        let clean: Vec<u64> = laps.iter().filter_map(|(_, solo)| solo.best).collect();
        let mean = clean.iter().sum::<u64>() as f32 / clean.len().max(1) as f32 / 60.0;
        report.push_str(&format!(
            "TOTAL      {total:<7.2} {:<9} clean {:<2}  mean {mean:.1}s\n",
            laps.iter().map(|(_, solo)| solo.respawns).sum::<u32>(),
            clean.len(),
        ));
        if let Some(field) = field_run(tuning) {
            report.push_str(&format!(
                "FIELD      lapped {}/7  untimed {} (must be 0)  mean {:.2} (floor 0.70)  \
                 worst {:.2} (floor 0.45)\n",
                field.lapped, field.untimed, field.mean, field.worst,
            ));
        }
        println!("{report}");
    }
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP_LOOK, read the table, choose"]
fn sweep_look_speed() {
    let Some(list) = std::env::var_os("OAG_SWEEP_LOOK") else {
        return;
    };
    let variants: Vec<(String, oag_ai::Tuning)> = list
        .to_string_lossy()
        .split(',')
        .map(|word| {
            let value: f32 = word.trim().parse().expect("a look_speed value");
            (
                format!("{value}"),
                oag_ai::Tuning {
                    look_speed: value,
                    ..oag_ai::Tuning::default()
                },
            )
        })
        .collect();
    run_sweep("look_speed", variants);
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP_GAIN, read the table, choose"]
fn sweep_rate_gain() {
    let Some(list) = std::env::var_os("OAG_SWEEP_GAIN") else {
        return;
    };
    let variants: Vec<(String, oag_ai::Tuning)> = list
        .to_string_lossy()
        .split(',')
        .map(|word| {
            let value: f32 = word.trim().parse().expect("a rate_gain value");
            (
                format!("{value}"),
                oag_ai::Tuning {
                    rate_gain: value,
                    ..oag_ai::Tuning::default()
                },
            )
        })
        .collect();
    run_sweep("rate_gain", variants);
}
