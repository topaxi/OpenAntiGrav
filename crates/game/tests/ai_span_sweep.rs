//! What a ceiling on the AI's curvature estimator is worth, on the disc's own
//! circuits.
//!
//! **`#[ignore]`d, gated on `OAG_SWEEP_SPAN`, and never run in CI.** It needs
//! game content, which this project does not ship - see
//! `docs/architecture/adr/0006-no-copyrighted-content.md` - and `#[ignore]`
//! alone would not keep it out of `just test-data`, which runs ignored tests. A
//! run of this length in that suite is how a suite stops being run; that is the
//! same reasoning `race_ground_truth.rs`'s own `sweep_grip` gives.
//!
//! ```sh
//! OAG_SWEEP_SPAN=none,6,10,16,25 OAG_REQUIRE_GAME_DATA=1 \
//!   cargo nextest run --release -p oag-game --run-ignored all \
//!   sweep_curvature_span --no-capture
//! ```
//!
//! About 15 s a row in release.
//!
//! # Why this needs its own file, and its own metric
//!
//! `sweep_grip` reports clean laps, laps completed, recoveries and lap time.
//! **None of those four can see the failure this knob reaches.** `07_Track`
//! banks a clean 49.8s lap while shedding 34-35 shield doing it, grinding down
//! the wall outside a corner its driver never saw, and every one of those
//! columns reads that as a pass right up to the tick the craft is destroyed on
//! lap 3. So this measures **shield**, and it is the reason a knob that is worth
//! 164 shield across the board is worth nothing at all on `sweep_grip`'s table.
//!
//! Two shield figures, and they are **different scales**. [`Solo::shield`] is
//! what was left in the pool after 18,000 ticks; [`Solo::per_lap`] is what each
//! completed lap cost. Reading one against the other is the trap this file's
//! output is laid out to avoid: `03 95.00` is a circuit that finished at full
//! capacity, `07 33.46 35.11` is a circuit that lost that much *each lap*.
//!
//! The table this produced, the criterion it was read with and the value chosen
//! are on `oag_ai::Tuning::curvature_span`, next to the constant they set.

use std::path::PathBuf;

use oag_game::{catalogue, race};

/// How long a measured run is: five minutes at 60 Hz, the same window
/// `race_ground_truth.rs`'s solo benchmark uses, so the two are comparable.
const TICKS: u64 = 18_000;

/// Which slot is measured. Slot 0 is switched off and 2..8 with it, so what is
/// left is one opponent alone on the circuit - the driving, with no traffic
/// mixed into it.
const LONE: usize = 1;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// What one lone craft managed on one circuit.
#[derive(Debug, Default, Clone)]
struct Solo {
    /// The quickest lap with no recovery in it, in ticks.
    best: Option<u64>,
    /// How many times the craft had to be put back on the track.
    respawns: u32,
    /// How far round it got, in laps.
    laps: u32,
    /// What was left in the shield pool when the run ended.
    shield: f32,
    /// What each completed lap cost the pool, in order.
    ///
    /// A respawn does not refill it, so a lap the craft was recovered during
    /// still reports what it shed; read it beside [`Self::respawns`].
    per_lap: Vec<f32>,
}

/// One lone craft, one circuit, one tuning.
///
/// **`track` is a WAD entry name, not a circuit id** - the same trap
/// `ai_roll_ground_truth.rs` records: `catalogue::Track::id` spells the circuit
/// `10_Track` and `Options::track` wants `Data\Tracks\...\track.vex`, and
/// passing the first silently fails to load.
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
            // The first lap is the standing start, and a lap the craft had to
            // be recovered during is not a lap it drove.
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

/// Every forward circuit on the disc, as `(id, entry name)`.
///
/// Forward only: a reversed circuit is the same geometry and doubles the wall
/// clock for nothing this measurement can see.
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

/// What a full grid did, on the fixture the two field ground-truth tests use.
#[derive(Debug, Default, Clone, Copy)]
struct FieldRun {
    /// How many of the seven opponents completed a **timed** lap - the thing
    /// `lap_times_ground_truth::every_opponent_that_laps_has_a_lap_time`
    /// asserts. A craft that crosses the line once and then wedges reaches
    /// `lap >= 2` with no time against it, which is exactly what that test
    /// catches.
    lapped: usize,
    /// And how many of those have **no time** against them, which is the exact
    /// thing that test panics on: a craft that crosses the line once and then
    /// wedges reaches `lap >= 2` with nothing recorded. Anything but zero here
    /// is that test red.
    untimed: usize,
    /// The mean opponent shield after a minute, and the **worst** one, as
    /// fractions of the pool. `opponent_weapons_ground_truth`'s floors are 0.70
    /// on the mean and 0.45 on the worst, and it is the worst that moves: blind
    /// to mines the mean sat at 0.80 while one craft was ground to 0.36.
    mean: f32,
    worst: f32,
}

/// A full grid, on **the two committed field fixtures exactly as they are set
/// up**, not on a third one of this file's own.
///
/// **The half of the board the per-circuit sweep above cannot see, and it is
/// not optional.** Every row of that table is one craft alone. A span that
/// reads well there can still wedge a craft that is being shoved, and a span
/// whose solo board was clean on all twelve turned both of these red. Judging a
/// span without this is how a number chosen on twelve clean rows becomes
/// invisible damage in a race.
///
/// Two fixtures because the two tests use two, and copying either one onto the
/// other is how a sweep stops predicting the thing it is meant to gate:
///
/// - `lap_times_ground_truth::clocks` - `16_Track`, **Ace**, 9,000 ticks.
/// - `opponent_weapons_ground_truth::single_race` - the **default** circuit and
///   the **default** difficulty, 3,600 ticks.
///
/// And the tuning goes through [`oag_ai::Difficulty::tune`] rather than being
/// set raw, because that is what `Race::start` does. Setting a bare
/// `Tuning::default()` here would also wipe the difficulty's own grip belief,
/// mistake rate and reaction delay, and the row would be measuring those
/// instead of the span. (The solo rows above set it raw on purpose: that is
/// what `race_ground_truth.rs`'s own sweeps do, so the two tables compare.)
fn field_run(cap: Option<f32>) -> Option<FieldRun> {
    let image = image()?;
    let tuned = |level: oag_ai::Difficulty| {
        level.tune(&oag_ai::Tuning {
            curvature_span: cap,
            ..oag_ai::Tuning::default()
        })
    };

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

/// The sweep itself. `OAG_SWEEP_SPAN` takes a comma-separated list of caps in
/// units, with `none` for the uncapped row.
///
/// **The `none` row is the harness's own check on itself**: it reproduces the
/// behaviour from before the cap existed, so a table whose `none` row does not
/// match the recorded baseline is a table measuring a different harness rather
/// than a different span.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP_SPAN, read the table, choose"]
fn sweep_curvature_span() {
    let Some(list) = std::env::var_os("OAG_SWEEP_SPAN") else {
        return;
    };
    let caps: Vec<Option<f32>> = list
        .to_string_lossy()
        .split(',')
        .map(|word| match word.trim() {
            "none" => None,
            value => Some(value.parse().expect("a span cap in units, or `none`")),
        })
        .collect();

    let circuits = circuits();
    if circuits.is_empty() {
        return;
    }

    for cap in caps {
        let tuning = oag_ai::Tuning {
            curvature_span: cap,
            ..oag_ai::Tuning::default()
        };
        let laps: Vec<(String, Solo)> = circuits
            .iter()
            .filter_map(|(id, entry)| solo_on(entry, tuning).map(|solo| (id.clone(), solo)))
            .collect();

        let label = cap.map_or_else(|| "none".to_string(), |cap| format!("{cap}"));
        let mut report = format!(
            "\n=== span {label} ===\ncircuit    end     respawns  laps  best     per-lap shield\n"
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
        if let Some(field) = field_run(cap) {
            report.push_str(&format!(
                "FIELD      lapped {}/7  untimed {} (must be 0)  mean {:.2} (floor 0.70)  \
                 worst {:.2} (floor 0.45)\n",
                field.lapped, field.untimed, field.mean, field.worst,
            ));
        }
        println!("{report}");
    }
}
