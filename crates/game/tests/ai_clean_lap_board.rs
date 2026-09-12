//! The board behind "an Ace should lap every circuit, in every speed class,
//! without touching a wall".
//!
//! **`#[ignore]`d, gated on `OAG_SWEEP`, printing rather than asserting**, for
//! the reason `race_ground_truth.rs`'s own `sweep_grip` and
//! `ai_span_sweep.rs` both give: it needs game content this project does not
//! ship, and `#[ignore]` alone does not keep a run of this length out of `just
//! test-data`, which runs ignored tests. Forty-eight rows of five simulated
//! minutes each is how a suite stops being run.
//!
//! ```sh
//! OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 \
//!   cargo nextest run --release -p oag-game --run-ignored all \
//!   --no-capture clean_lap_board
//! ```
//!
//! # What this measures that nothing else did
//!
//! Three axes, none of which the existing harness could see.
//!
//! - **Speed class.** Every solo measurement this project had taken was at
//!   `VENOM`; `solo_lap_tuned` hardcoded it. `FLASH`, `RAPIER` and `PHANTOM`
//!   were unmeasured.
//! - **Wall contact, as its own quantity.** `Solo` reported clean lap,
//!   respawns, laps and `lost_at`; shield was not in it, and where shield *has*
//!   been read by hand it mixes barrel-roll charge and shield-pad pickups into
//!   the wall term. That is why `07_Track` was the only circuit whose wall
//!   attrition could be read at all. [`Race::wall_contact_ticks_of`] and
//!   [`Race::wall_shield_charged_of`] separate the two structurally rather
//!   than by subtraction - see their doc comments.
//! - **Team.** [`yaw_ceiling_by_team_and_class`] below, which is a table read
//!   rather than a sweep.
//!
//! [`Race::wall_contact_ticks_of`]: oag_game::race::Race::wall_contact_ticks_of
//! [`Race::wall_shield_charged_of`]: oag_game::race::Race::wall_shield_charged_of

use std::path::PathBuf;

use oag_game::{catalogue, race};

/// How long a measured run is: five minutes at 60 Hz, the same window
/// `race_ground_truth.rs`'s solo benchmark and `ai_span_sweep.rs` use, so all
/// three tables compare.
///
/// **It is not enough to hold every class inside the race.** `SingleRace` runs
/// `Mode::SINGLE_RACE_LAPS_BY_CLASS` = `[3, 4, 4, 5]` laps, and a craft that
/// crosses the last one stops being `Racing` - `Race::step_opponents` releases
/// it and it coasts. [`Solo::finished`] records whether that happened, so a row
/// whose end-of-run pool was taken off a coasting wreck is visible as such
/// rather than silently compared against a row still racing.
const TICKS: u64 = 18_000;

/// Which slot is measured. Slot 0 is switched off and 2..8 with it, so what is
/// left is one opponent alone on the circuit.
const LONE: usize = 1;

/// How wide a driver-index bucket is when locating the worst cluster.
///
/// Fifty samples is the granularity the Outpost 7 thread reported its own
/// clusters at (`2,100-2,149`, `2,350-2,399`), so a bucket here names the same
/// stretch of road that thread's evidence pages do.
const CLUSTER: u32 = 50;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// What one lone craft managed on one circuit at one speed class.
#[derive(Debug, Default, Clone)]
struct Solo {
    /// The quickest lap with no recovery in it, in ticks.
    best: Option<u64>,
    /// How many times the craft had to be put back on the track.
    respawns: u32,
    /// How far round it got, in laps. `standing.lap` counts from 1, so a row
    /// reading `4` completed three.
    laps: u32,
    /// Whether the craft completed its class's full lap target inside
    /// [`TICKS`]. A row that did not is not comparable on end-of-run pool with
    /// one that did, and neither is one that did: past the flag the craft stops
    /// being `Racing` and coasts.
    finished: bool,
    /// What state the craft ended in. `Destroyed` is the row that matters -
    /// that is a craft the circuit killed.
    state: oag_physics::CraftState,
    /// What was left in the shield pool when the run ended.
    end_shield: f32,
    /// What each completed lap cost the pool, in order. **A different scale
    /// from [`Self::end_shield`]** and the trap this table is laid out to
    /// avoid: `95.00` end is a circuit that finished at capacity, `33.46 35.11`
    /// per lap is a circuit shedding that much *each* lap.
    per_lap: Vec<f32>,
    /// Ticks spent in contact with a wall.
    contact_ticks: u32,
    /// The inbound subset - arrivals rather than scrapes.
    inbound_ticks: u32,
    /// What those contacts charged, in pool units. Charged, not lost.
    wall_charged: f32,
    /// The worst [`CLUSTER`]-wide driver-index bucket, as
    /// `(first index, charge)`, so a reader can go straight to it.
    worst_cluster: Option<(u32, f32)>,
    /// The racing line's length, for reading [`Self::worst_cluster`] against.
    line_len: u32,
}

/// One lone craft, one circuit, one class, one tuning.
///
/// **`track` is a WAD entry name, not a circuit id** - the same trap
/// `ai_span_sweep.rs` records: `catalogue::Track::id` spells the circuit
/// `10_Track` and `Options::track` wants `Data\Tracks\...\track.vex`, and
/// passing the first silently fails to load.
fn solo_on(track: &str, class: &str, tuning: Option<oag_ai::Tuning>) -> Option<Solo> {
    let image = image()?;
    // The class's own lap target, so a row can say whether five minutes held
    // the whole race - `[3, 4, 4, 5]`, so `PHANTOM` is two laps longer than
    // `VENOM` at a higher speed and the two do not automatically both fit.
    let target = oag_tables::handling::SpeedClass::from_name(class).map_or(0, |rung| {
        oag_race::Mode::SINGLE_RACE_LAPS_BY_CLASS[rung as usize]
    });
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .ok()?;
    let mut race = race::Race::start(loaded.setup);
    if let Some(tuning) = tuning {
        race.set_ai_tuning(tuning);
    }
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;

    let mut best: Option<u64> = None;
    let mut lap = race.sim.world.ships[LONE].standing.lap;
    let mut started = 0u64;
    let mut recovered_this_lap = false;
    let mut per_lap = Vec::new();
    let mut shield_at_lap = race.sim.world.ships[LONE].physics.shield;
    // Buckets rather than a full per-index table: the whole point is to name
    // one stretch of road, and a sparse map would be a `HashMap` iteration
    // order feeding a printed report.
    let line_len = race.racing_line().len() as u32;
    let mut clusters = vec![0.0f32; (line_len / CLUSTER + 2) as usize];
    let mut charged = 0.0f32;
    for tick in 0..TICKS {
        let before = race.respawns_of(LONE);
        let was_at = race.sim.world.ships[LONE].driver.index;
        race.tick(&oag_gameplay::InputSnapshot::default());
        if race.respawns_of(LONE) != before {
            recovered_this_lap = true;
        }
        let now_charged = race.wall_shield_charged_of(LONE);
        if now_charged > charged {
            // Charged against where the craft *entered* the tick, which is the
            // index a reader casts from when they go looking - the same
            // convention `Solo::lost_at` uses in `race_ground_truth.rs`.
            let bucket = (was_at / CLUSTER) as usize;
            if let Some(slot) = clusters.get_mut(bucket) {
                *slot += now_charged - charged;
            }
            charged = now_charged;
        }
        let now = race.sim.world.ships[LONE].standing.lap;
        if now != lap {
            let shield = race.sim.world.ships[LONE].physics.shield;
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

    let worst_cluster = clusters
        .iter()
        .enumerate()
        .filter(|(_, charge)| **charge > 0.0)
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(bucket, charge)| (bucket as u32 * CLUSTER, *charge));

    Some(Solo {
        best,
        respawns: race.respawns_of(LONE),
        laps: lap,
        finished: lap.saturating_sub(1) >= target,
        state: race.sim.world.ships[LONE].physics.craft_state,
        end_shield: race.sim.world.ships[LONE].physics.shield,
        per_lap,
        contact_ticks: race.wall_contact_ticks_of(LONE),
        inbound_ticks: race.wall_inbound_ticks_of(LONE),
        wall_charged: charged,
        worst_cluster,
        line_len,
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
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");

    catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| !track.reversed)
        .map(|track| (track.id.clone(), track.entry_name()))
        .collect()
}

/// The four rungs, as `Options::class` spells them.
const CLASSES: [&str; 4] = ["VENOM", "FLASH", "RAPIER", "PHANTOM"];

/// Which classes a run covers. `OAG_SWEEP_CLASS` narrows it; the default is all
/// four, and a row is fifteen seconds in release.
fn classes() -> Vec<String> {
    match std::env::var("OAG_SWEEP_CLASS") {
        Ok(list) => list
            .split(',')
            .map(|word| word.trim().to_uppercase())
            .filter(|word| !word.is_empty())
            .collect(),
        Err(_) => CLASSES.iter().map(|class| (*class).to_string()).collect(),
    }
}

fn row(id: &str, class: &str, solo: &Solo) -> String {
    let best = solo
        .best
        .map_or_else(|| "-".to_string(), |b| format!("{:.1}s", b as f32 / 60.0));
    let per_lap: Vec<String> = solo
        .per_lap
        .iter()
        .map(|cost| format!("{cost:.1}"))
        .collect();
    let cluster = solo.worst_cluster.map_or_else(
        || "-".to_string(),
        |(index, charge)| format!("{index}-{}@{charge:.1}", index + CLUSTER - 1),
    );
    format!(
        "{id:<10} {class:<8} {best:<8} {:<5} {:<4} {:<4} {:<10} {:<7} {:<7.2} {cluster:<16} \
         {:<7.2} {}\n",
        solo.respawns,
        solo.laps,
        if solo.finished { "yes" } else { "NO" },
        format!("{:?}", solo.state),
        solo.contact_ticks,
        solo.wall_charged,
        solo.end_shield,
        per_lap.join(" "),
    )
}

fn header() -> String {
    "circuit    class    best     resp  laps fin  state      ticks   charged worst cluster    \
     end     per-lap shield\n"
        .to_string()
}

/// The board: every forward circuit, every speed class, one lone Ace.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP, read the board"]
fn clean_lap_board() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let circuits = circuits();
    if circuits.is_empty() {
        return;
    }
    let mut report = format!("\n=== clean-Ace board, Pulse PSP USA ===\n{}", header());
    for class in classes() {
        for (id, entry) in &circuits {
            let Some(solo) = solo_on(entry, &class, None) else {
                continue;
            };
            report.push_str(&row(id, &class, &solo));
        }
        report.push('\n');
    }
    println!("{report}");
}

/// What a craft's hull can actually yaw at, per team and per class, against the
/// AI's own global belief.
///
/// **A table read, not a sweep** - and it is the measurement that decides
/// whether the board needs a team column at all. `oag_ai::Tuning::max_turn_rate`
/// is one global constant (1.8) while the rate a hull *achieves* comes out of
/// the authored `<Turning amount>`; if the achievable ceiling clusters, the team
/// axis is dead and the board is a circuit-by-class grid.
///
/// The steady state is `omega = steer * Turning.amount / (5 * I_yy)`:
/// `oag_physics::engine::steering` is `steer * Turning.amount` as a body-local
/// yaw drive, `oag_physics::passive::angular_damping` damps yaw momentum at
/// `-5`, and `I_yy` is `1 / oag_physics::forces::YAW_INVERSE_INERTIA`. `steer`
/// runs to `oag_physics::controls::CONTROL_RANGE`.
///
/// **`I_yy` is not per-craft, and the brief this was opened under assumed it
/// was.** `YAW_INVERSE_INERTIA`'s own doc settles it: the inertia box `(12, 8,
/// 12)` and the mass `0.9` are *code literals at a single call site* in the
/// ship-entity constructor, and `Misc` `width`/`length`/`height` reach the
/// collider, not the tensor. So every craft in the game has the same tensor and
/// the only per-craft term in the ceiling is `Turning.amount`.
#[test]
#[ignore = "a scratch table read: set OAG_SWEEP, read the table"]
fn yaw_ceiling_by_team_and_class() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(image) = image() else {
        return;
    };
    let source = image.display().to_string();
    let mut archives = oag_pulse::open(&source).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");

    let i_yy = 1.0 / oag_physics::forces::YAW_INVERSE_INERTIA;
    let damping = 5.0f32;
    let steer = oag_physics::controls::CONTROL_RANGE;
    let mut report = format!(
        "\n=== achievable yaw ceiling, steer * Turning.amount / ({damping} * I_yy), \
         I_yy = {i_yy:.3} ===\nteam           class    amount   ceiling  vs Tuning::max_turn_rate \
         {:.2}\n",
        oag_ai::Tuning::default().max_turn_rate
    );
    let mut lowest = f32::INFINITY;
    let mut highest = 0.0f32;
    for team in catalogue::teams(&definition) {
        let title = oag_pulse::TITLE;
        let name =
            oag_tables::handling::entry_name_in(title.race.handling_dir_for(&team.id), &team.id);
        let Ok(stats_blob) = archives.read_name(&name) else {
            report.push_str(&format!("{:<14} (no {name})\n", team.id));
            continue;
        };
        let Ok(stats) = oag_tables::handling::from_blob(&stats_blob) else {
            report.push_str(&format!("{:<14} ({name} does not parse)\n", team.id));
            continue;
        };
        for class in CLASSES {
            let Some(block) = stats.class_named(class) else {
                continue;
            };
            let amount = block.turning.amount;
            let ceiling = steer * amount / (damping * i_yy);
            lowest = lowest.min(ceiling);
            highest = highest.max(ceiling);
            report.push_str(&format!(
                "{:<14} {class:<8} {amount:<8.4} {ceiling:<8.4} {:+.1}%\n",
                team.id,
                (ceiling / oag_ai::Tuning::default().max_turn_rate - 1.0) * 100.0,
            ));
        }
    }
    report.push_str(&format!(
        "spread: {lowest:.4} to {highest:.4} rad/s, {:.1}% of the lowest\n",
        (highest / lowest - 1.0) * 100.0
    ));
    println!("{report}");
}

/// Which quantity the board's wall column should be built on.
///
/// **The check that had to come before the board**, because the two candidates
/// are not the same count and choosing the wrong one understates every row.
/// `oag_physics::ShipState::wall_contact_prev` is `WallResponse::impact`, an
/// *inbound* test (`normal_speed < 0.0`); a craft grinding along a wall stops
/// being inbound long before it stops being in contact, and the Outpost 7
/// thread's own decomposition is "a 4.36 shield impact **and ~180 ticks of
/// grinding** at 0.02-0.05 shield a tick".
///
/// Prints all three on one circuit so the ordering is on the record:
/// inbound ticks, contact ticks, and what the contacts charged.
#[test]
#[ignore = "a scratch probe: set OAG_SWEEP, read the counts"]
fn inbound_ticks_are_not_contact_ticks() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let Some(solo) = solo_on("Data\\Environments\\07_Track\\track.vex", "VENOM", None) else {
        return;
    };
    println!(
        "\n07_Track VENOM lone Ace, {TICKS} ticks:\n  contact ticks {}\n  inbound ticks {}\n  \
         charged {:.2}\n  end shield {:.2}\n  per-lap {:?}\n  worst cluster {:?} of {}\n",
        solo.contact_ticks,
        solo.inbound_ticks,
        solo.wall_charged,
        solo.end_shield,
        solo.per_lap,
        solo.worst_cluster,
        solo.line_len,
    );
}
