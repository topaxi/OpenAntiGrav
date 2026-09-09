//! Scratch: does `opponent_weapons_ground_truth.rs`'s worst-craft floor
//! separate "one craft got unlucky" from "an AI change wrecked the field",
//! once sampled over several seeds rather than the one the ground-truth test
//! itself is pinned to?
//!
//! `#[ignore]`d, gated on an env var, never run in CI or in `just test-data` -
//! the same shape as `ai_look_sweep.rs`. Uses `race::Options::seed`, the
//! existing verification aid, to vary the world generator across otherwise
//! identical setups, always on `16_Track` (`race::DEFAULT_TRACK`) - the same
//! circuit the ground-truth test and `ai_look_sweep.rs`'s own field
//! measurement both use. A pooled multi-track sweep was tried first and
//! dropped: each circuit has its own shield economy (03_Track and 05_Track
//! differ by 0.49 of a pool at the same tuning), so pooling across tracks
//! buries a tuning effect in between-track variance. A fixed track with the
//! seed varied is the paired design - same track, same tuning, only the
//! world's RNG stream differs seed to seed.
//!
//! **`Tuning` is degraded through `Difficulty::tune`, exactly as
//! `Race::start` and `ai_look_sweep.rs::field_run` both apply it** - the same
//! step `Tuning::look_speed`'s own doc comment's table was measured through.
//! Skipping this step drives every craft with `mistake_rate: 0.0`, which is a
//! different simulation from the one being compared against.
//!
//! ```sh
//! OAG_SWEEP_SEEDS=16 OAG_SWEEP_LOOK=0.30,0.22,0.20 OAG_REQUIRE_GAME_DATA=1 \
//!   cargo nextest run -p oag-game --run-ignored all \
//!   sweep_worst_shield_over_seeds --no-capture
//! ```

use std::path::PathBuf;

use oag_game::race;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

#[derive(Debug, Clone, Copy)]
struct FieldRun {
    mean: f32,
    worst: f32,
    /// Opponents that ended the run with no shield left (`shield <= 0.0`).
    ///
    /// **Not `!Ship::active`.** Nothing in `crates/game` consumes
    /// `Shield::depleted` yet, so a craft's `active` flag never flips off
    /// depletion in this build - reading it here would print a column that is
    /// structurally always zero.
    depleted: usize,
}

fn field_run(seed: u64, tuning: oag_ai::Tuning) -> Option<FieldRun> {
    let image = image()?;
    let options = race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        seed: Some(seed),
        ..race::Options::default()
    };
    // Degrade through the race's own difficulty, the same step `Race::start`
    // and `ai_look_sweep.rs::field_run` both apply - see this file's module
    // doc for why skipping it drives a different simulation.
    let level = options.difficulty;
    let loaded = race::load(&options).ok()?;
    let mut race = race::Race::start(loaded.setup);
    race.set_ai_tuning(level.tune(&tuning));
    let opponents = 1..usize::from(race.ship_count());
    let full = race.world.ships[1].handling.dimensions.shield;
    for _ in 0..3_600 {
        race.tick(&oag_gameplay::InputSnapshot::default());
    }
    let depleted = opponents
        .clone()
        .filter(|&slot| race.world.ships[slot].physics.shield <= 0.0)
        .count();
    let mean: f32 = opponents
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
        mean,
        worst,
        depleted,
    })
}

/// The other candidate metric: not shield at all, but whether a craft gets
/// round the circuit twice inside a real time budget - `ai_look_sweep.rs`'s
/// own `lap_times` scenario, seed-varied the same way [`field_run`] is.
///
/// **Ace, not Elite** - the same difficulty `ai_look_sweep.rs::field_run`
/// times laps at, so this is directly comparable to that file's own
/// `lapped`/`untimed` columns rather than a new scenario. 9,000 ticks (150s),
/// also unchanged from there.
///
/// Returns `(lapped, untimed)`: how many opponents reached lap 2 at all, and
/// of those, how many have no recorded `best_lap_ticks` - a lap that was
/// crossed without ever being timed, which is the failure
/// `lap_times_ground_truth::every_opponent_that_laps_has_a_lap_time` exists
/// for. Neither number touches weapons or shield, so this is the candidate
/// for a metric a wrecked *steering* tuning would move without the weapon
/// RNG this file's other sweep found to be the dominant noise source.
fn lap_completion_run(seed: u64, tuning: oag_ai::Tuning) -> Option<(usize, usize)> {
    let image = image()?;
    let options = race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(race::DEFAULT_TRACK.to_string()),
        seed: Some(seed),
        ..race::Options::default()
    };
    let level = options.difficulty;
    let loaded = race::load(&options).ok()?;
    let mut race = race::Race::start(loaded.setup);
    race.set_ai_tuning(level.tune(&tuning));
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
    Some((lapped.len(), untimed))
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP_SEEDS and OAG_SWEEP_LOOK, read the table"]
fn sweep_lap_completion_over_seeds() {
    let Some(seed_count) = std::env::var_os("OAG_SWEEP_SEEDS") else {
        return;
    };
    let seed_count: u64 = seed_count.to_string_lossy().parse().expect("a seed count");
    let Some(look_list) = std::env::var_os("OAG_SWEEP_LOOK") else {
        return;
    };
    let variants: Vec<(String, oag_ai::Tuning)> = look_list
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

    if image().is_none() {
        return;
    }

    for (name, tuning) in variants {
        let mut report = format!(
            "\n=== lap_completion look_speed {name} over {seed_count} seeds, 16_Track ===\n"
        );
        report.push_str("seed  lapped  untimed\n");
        let mut short_seeds = 0usize;
        let mut untimed_seeds = 0usize;
        for seed in 0..seed_count {
            let Some((lapped, untimed)) = lap_completion_run(seed, tuning) else {
                continue;
            };
            report.push_str(&format!("{seed:<5} {lapped:<7} {untimed}\n"));
            if lapped < 7 {
                short_seeds += 1;
            }
            if untimed > 0 {
                untimed_seeds += 1;
            }
        }
        report.push_str(&format!(
            "seeds with fewer than 7/7 lapped: {short_seeds}/{seed_count}  \
             seeds with an untimed lap: {untimed_seeds}/{seed_count}\n"
        ));
        println!("{report}");
    }
}

#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP_SEEDS and OAG_SWEEP_LOOK, read the table"]
fn sweep_worst_shield_over_seeds() {
    let Some(seed_count) = std::env::var_os("OAG_SWEEP_SEEDS") else {
        return;
    };
    let seed_count: u64 = seed_count.to_string_lossy().parse().expect("a seed count");
    let Some(look_list) = std::env::var_os("OAG_SWEEP_LOOK") else {
        return;
    };
    let variants: Vec<(String, oag_ai::Tuning)> = look_list
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

    if image().is_none() {
        return;
    }

    // The first variant is the baseline every later one is paired against,
    // seed for seed - the comparison a plain mean-of-means cannot make, since
    // the per-seed spread here (commonly > 0.2) swamps it.
    let mut baseline_worsts: Option<Vec<f32>> = None;

    for (name, tuning) in variants {
        let mut report = format!("\n=== look_speed {name} over {seed_count} seeds, 16_Track ===\n");
        report.push_str("seed  mean   worst  depleted\n");
        let mut worsts = Vec::new();
        let mut means = Vec::new();
        let mut depleted_seeds = 0usize;
        for seed in 0..seed_count {
            let Some(run) = field_run(seed, tuning) else {
                continue;
            };
            report.push_str(&format!(
                "{seed:<5} {:<6.2} {:<6.2} {}\n",
                run.mean, run.worst, run.depleted
            ));
            worsts.push(run.worst);
            means.push(run.mean);
            if run.depleted > 0 {
                depleted_seeds += 1;
            }
        }
        let mut sorted = worsts.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_worst = sorted[sorted.len() / 2];
        let mean_of_means = means.iter().sum::<f32>() / means.len() as f32;
        let mean_of_worsts = worsts.iter().sum::<f32>() / worsts.len() as f32;
        let below_045 = worsts.iter().filter(|&&w| w <= 0.45).count();
        report.push_str(&format!(
            "median worst {median_worst:.2}  mean-of-means {mean_of_means:.2}  \
             mean-of-worsts {mean_of_worsts:.2}  seeds<=0.45 {below_045}/{}  \
             seeds with a depleted craft {depleted_seeds}/{}\n",
            worsts.len(),
            worsts.len()
        ));
        if let Some(baseline) = &baseline_worsts {
            let worse_than_baseline = worsts.iter().zip(baseline).filter(|(w, b)| w < b).count();
            report.push_str(&format!(
                "paired against the baseline: {worse_than_baseline}/{} seeds have a \
                 strictly lower worst here than the baseline had on that same seed\n",
                worsts.len()
            ));
        } else {
            baseline_worsts = Some(worsts);
        }
        println!("{report}");
    }
}
