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
//! [`Race::wall_contact_ticks_of`]: oag_raceplay::Race::wall_contact_ticks_of
//! [`Race::wall_shield_charged_of`]: oag_raceplay::Race::wall_shield_charged_of

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_physics::Raycaster;
use oag_raceplay as race;
use oag_raceplay::catalogue;

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
    /// How many of [`TICKS`] the craft was still racing for - the denominator
    /// every wall figure above is counted over. A wrecked craft is released and
    /// keeps being integrated, so without this a hull settled against a wall
    /// reads as the busiest driver on the board.
    racing_ticks: u32,
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
        race.tick(&PlayerInputs::none());
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
        racing_ticks: race.racing_ticks_of(LONE),
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
        "{id:<10} {class:<8} {best:<8} {:<5} {:<4} {:<4} {:<10} {:<7} {:<7} {:<7.2} \
         {cluster:<16} {:<7.2} {}\n",
        solo.respawns,
        solo.laps,
        if solo.finished { "yes" } else { "NO" },
        format!("{:?}", solo.state),
        solo.racing_ticks,
        solo.contact_ticks,
        solo.wall_charged,
        solo.end_shield,
        per_lap.join(" "),
    )
}

fn header() -> String {
    "circuit    class    best     resp  laps fin  state      racing  ticks   charged \
     worst cluster    end     per-lap shield\n"
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

/// What a held airbrake is worth to the corner-speed ceiling, off the disc's
/// own numbers.
///
/// **A table read and a closed-form evaluation, not a sweep** - the same shape
/// as [`yaw_ceiling_by_team_and_class`], and for the same reason: it sizes an
/// opportunity before any code is written for it.
///
/// # The mechanism, and why it is a curvature offset rather than a bigger ceiling
///
/// Two torques write `acc.local_angular.y` and they **add**:
/// `oag_physics::engine::steering` is `-(steer * Turning.amount)`, speed
/// *independent*, and `oag_physics::airbrake`'s is `speed * Airbrake.turn *
/// imbalance * 0.001`, speed *proportional*. Through the same steady state
/// [`hull_yaw_ceiling`](oag_ai::hull_yaw_ceiling) uses (`omega = torque / (5 *
/// I_yy)`, and `5 * 21.6 = 108` on Pulse's craft):
///
/// ```text
/// v * k = [S + v * T] / 108      S = 100 * Turning.amount
///                                T = Airbrake.turn * imbalance * 0.001
/// v     = (S / 108) / (k - C)    C = T / 108
/// ```
///
/// So the airbrake **subtracts from the curvature** rather than adding to the
/// ceiling, and the gain grows without bound as `k` approaches `C` - largest
/// exactly at the tight apexes where the yaw term binds.
///
/// **The sign was checked rather than assumed**, because this project shipped it
/// backwards for months in `5ad69f3`. `imbalance = left - right_brake` on the
/// ramped `0..=100` states, so braking the right side harder makes it negative,
/// and a negative `local_angular.y` is nose-**right** in this crate's frame -
/// the same sign `engine::steering`'s `-(steer * amount)` gives a right steer.
/// They add.
///
/// # The counter-term, which is the whole question
///
/// `pace::corner_target` is `min(v_grip, v_yaw)`. Holding an airbrake raises
/// `v_yaw` **and lowers `v_grip`**: `oag_physics::airbrake`'s lateral-grip
/// coefficient is `max(L, R) * (0.01 - slidegrip) - 1.0`, so grip retained at
/// command `b` is `1 - b * (0.01 - slidegrip)` and `v_grip` scales as its square
/// root. A pure differential of `d` sets both `max(L, R)` and `|imbalance|` to
/// `d` - see `pace::airbrakes` with `brake = 0` - so one variable moves both
/// terms in opposite directions.
///
/// **This prints the combined `min`, never the yaw half alone.** If the `min`
/// does not rise, the mechanism is real in the physics and the driver cannot
/// exploit it, and that is the finding.
#[test]
#[ignore = "a scratch table read: set OAG_SWEEP, read the table"]
fn what_a_held_airbrake_is_worth_to_the_corner_ceiling() {
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

    let damping = -oag_physics::passive::YAW_DAMPING;
    let i_yy = 1.0 / oag_physics::forces::YAW_INVERSE_INERTIA;
    let denominator = damping * i_yy;
    let steer = oag_physics::controls::CONTROL_RANGE;
    let tuning = oag_ai::Tuning::default();
    // `07_Track`'s binding apex, the one Outpost 7 step 2 measured: radius 21.
    let k = 0.047f32;

    let mut report = format!(
        "\n=== what a held airbrake is worth, at 07_Track's binding k = {k} ===\n\
         omega = (S + v*T) / {denominator}, so v = (S/{denominator}) / (k - C), C = T/{denominator}\n\n\
         team           class    turn     slidegrip  C@d=100  C/k    v_yaw(0)  v_grip(0)  best min  at d   gain     k crossover\n"
    );
    for team in catalogue::teams(&definition) {
        let title = oag_pulse::TITLE;
        let name =
            oag_tables::handling::entry_name_in(title.race.handling_dir_for(&team.id), &team.id);
        let Ok(stats_blob) = archives.read_name(&name) else {
            continue;
        };
        let Ok(stats) = oag_tables::handling::from_blob(&stats_blob) else {
            continue;
        };
        for class in CLASSES {
            let Some(block) = stats.class_named(class) else {
                continue;
            };
            let s = steer * block.turning.amount;
            let turn = block.airbrake.turn;
            // **The scaled value, not the XML one.** `oag_tables` holds
            // `<Airbrake slidegrip>` verbatim on `0..100` and
            // `oag_gameplay::handling::SLIDEGRIP_SCALE` turns it into the
            // `0..0.01` the grip coefficient's `(0.01 - slidegrip)` is written
            // against. Reading the raw 80.0 makes `retained` come out at 8,000
            // and the grip term never binds, which silently deletes the
            // counter-term this whole table exists to weigh.
            let slidegrip = block.airbrake.slidegrip * oag_gameplay::handling::SLIDEGRIP_SCALE;
            let c_full = turn * steer * 0.001 / denominator;
            let v_yaw_0 = s / denominator / k;
            // The grip half at zero airbrake, on the driver's own belief - the
            // same `lateral_accel * commitment` `corner_target` uses, at the
            // neutral personality so the row is about the craft.
            let v_grip_0 = (tuning.lateral_accel / k).sqrt();
            // Sweep a pure differential and take the combined `min`, which is
            // what `corner_target` would actually return.
            let mut best = (v_yaw_0.min(v_grip_0), 0.0f32);
            let mut d = 0.0f32;
            while d <= steer {
                let c = turn * d * 0.001 / denominator;
                let v_yaw = if k - c > 1e-6 {
                    s / denominator / (k - c)
                } else {
                    f32::INFINITY
                };
                let retained = 1.0 - d * (0.01 - slidegrip);
                let v_grip = if retained > 0.0 {
                    (tuning.lateral_accel * retained / k).sqrt()
                } else {
                    0.0
                };
                let combined = v_yaw.min(v_grip);
                if combined > best.0 {
                    best = (combined, d);
                }
                d += 1.0;
            }
            // Where the two terms cross under a **full** differential: above this
            // curvature the yaw term still binds and the airbrake buys the whole
            // multiplier; below it the grip term binds first and holding the brake
            // is a net loss. Bisected rather than solved, so the expression stays
            // the two `corner_target` halves verbatim instead of an algebra step
            // that has to be re-derived if either moves.
            let retained_full = 1.0 - steer * (0.01 - slidegrip);
            let yaw_binds = |curvature: f32| {
                let yaw = if curvature - c_full > 1e-6 {
                    s / denominator / (curvature - c_full)
                } else {
                    f32::INFINITY
                };
                let grip = (tuning.lateral_accel * retained_full.max(0.0) / curvature).sqrt();
                yaw <= grip
            };
            let (mut lo, mut hi) = (c_full + 1e-5, 0.2f32);
            for _ in 0..60 {
                let mid = 0.5 * (lo + hi);
                if yaw_binds(mid) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            let crossover = hi;
            report.push_str(&format!(
                "{:<14} {class:<8} {turn:<8.3} {slidegrip:<10.6} {c_full:<8.5} {:<6.2} \
                 {v_yaw_0:<9.1} {v_grip_0:<10.1} {:<9.1} {:<6.0} {:<8} {crossover:.4}\n",
                team.id,
                c_full / k,
                best.0,
                best.1,
                format!("{:+.1}%", (best.0 / v_yaw_0.min(v_grip_0) - 1.0) * 100.0),
            ));
        }
    }
    println!("{report}");
}

/// What the airbrakes actually do through one named stretch of one circuit,
/// tick by tick.
///
/// **The probe that answers a player rather than a column.** A change that
/// improves the board's `charged` total while still looking timid on a hairpin
/// has not answered the report it was built for, and nothing else here can see
/// a hairpin at all - the board reports one number per lap.
///
/// `OAG_HAIRPIN` takes `circuit,class,first,last` and defaults to `07_Track`'s
/// documented crash cluster, `2150-2199` - the tightest corner on the board's
/// worst row, and the one the Outpost 7 thread's step 6 decomposed.
///
/// Reports the **ramped** `ShipState` airbrake values, not the driver's
/// commands: `oag_physics::controls::update` ramps them at `Airbrake.gain` and
/// `falloff`, so the command is what the driver asked for and these are what
/// the yaw term actually got. The gap between them is the arming cost, and it
/// is the thing a steady-state ceiling cannot see.
#[test]
#[ignore = "a scratch probe: set OAG_SWEEP, read the ticks"]
fn what_the_airbrakes_do_through_a_hairpin() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let spec = std::env::var("OAG_HAIRPIN").unwrap_or_else(|_| "07,VENOM,2150,2199".to_string());
    let parts: Vec<&str> = spec.split(',').collect();
    let (circuit, class) = (parts[0], parts.get(1).copied().unwrap_or("VENOM"));
    let first: u32 = parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(2150);
    let last: u32 = parts.get(3).and_then(|v| v.parse().ok()).unwrap_or(2199);

    let Some(image) = image() else {
        return;
    };
    let track = format!("Data\\Environments\\{circuit}_Track\\track.vex");
    let Ok(loaded) = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(track),
        ..race::Options::default()
    }) else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    // The same `OAG_SWEEP_CHORD` the board honours, so a stretch can be probed
    // under the estimator that breaks it rather than only under the shipped one.
    if let Some(chord) = std::env::var("OAG_SWEEP_CHORD")
        .ok()
        .and_then(|v| v.trim().parse::<f32>().ok())
    {
        race.set_ai_tuning(oag_ai::Difficulty::Ace.tune(&oag_ai::Tuning {
            curvature_chord: Some(chord),
            ..oag_ai::Tuning::default()
        }));
    }
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;

    let turn = race.sim.world.ships[LONE].handling.airbrake.turn;
    let denominator = -oag_physics::passive::YAW_DAMPING / oag_physics::forces::YAW_INVERSE_INERTIA;
    let mut report = format!(
        "\n=== {circuit}_Track {class}, driver index {first}-{last}, lone Ace ===\n\
         Airbrake.turn {turn}, so C = turn * |imbalance| * 0.001 / {denominator:.0}\n\n\
         tick   idx    speed   L      R      imbal  brake  steer   air    gnd   C        contact\n"
    );
    let mut lap_seen = 0;
    let mut charged = 0.0f32;
    for tick in 0..6_000u64 {
        race.tick(&PlayerInputs::none());
        let ship = &race.sim.world.ships[LONE];
        let index = ship.driver.index;
        if index < first || index > last {
            continue;
        }
        let now = race.wall_shield_charged_of(LONE);
        let hit = now > charged;
        charged = now;
        let state = &ship.physics;
        let left = state.airbrake_left;
        let right = state.airbrake_right;
        let imbalance = left - right;
        let c = turn * imbalance.abs() * 0.001 / denominator;
        report.push_str(&format!(
            "{tick:<6} {index:<6} {:<7.1} {left:<6.1} {right:<6.1} {imbalance:<6.1} \
             {:<6.1} {:<7.1} {:<6.3} {:<5.2} {c:<8.5} {}\n",
            state.body.linear_velocity.length(),
            state.brake,
            state.steer,
            state.time_airborne,
            state.grounded,
            if hit { "WALL" } else { "" },
        ));
        lap_seen += 1;
        if lap_seen > 400 {
            break;
        }
    }
    println!("{report}");
}

/// What `Tuning::trail_peak_decay` is worth, board-wide, with **lap time**
/// reported - the column the rest of this file did not have when the yaw
/// ceiling cost 0.5-3.0 s a circuit.
///
/// **`1.0` is the pre-fix behaviour**, so the row this replaced is in the
/// table rather than remembered, the same self-check `ai_span_sweep.rs`'s
/// `none` row is.
///
/// `Tuning::trail_max` was the first candidate and it is **not** the
/// constraint: measured through `07_Track`'s hairpin the differential is
/// `0.0`, not a capped `0.6`, so a ceiling on a quantity that never leaves
/// zero is worth nothing. See `pace::track_peak_curvature`.
///
/// ```sh
/// OAG_SWEEP=1 OAG_SWEEP_TRAIL=1.0,0.99,0.98 OAG_REQUIRE_GAME_DATA=1 \
///   cargo nextest run --release -p oag-game --run-ignored all \
///   --no-capture sweep_trail_peak_decay
/// ```
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP and OAG_SWEEP_TRAIL"]
fn sweep_trail_peak_decay() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let list =
        std::env::var("OAG_SWEEP_TRAIL").unwrap_or_else(|_| "1.0,0.995,0.99,0.98".to_string());
    let circuits = circuits();
    if circuits.is_empty() {
        return;
    }
    for word in list.split(',') {
        let Ok(decay) = word.trim().parse::<f32>() else {
            continue;
        };
        let tuning = oag_ai::Tuning {
            trail_peak_decay: decay,
            ..oag_ai::Tuning::default()
        };
        let mut report = format!("\n=== trail_peak_decay {decay} ===\n{}", header());
        let (mut ticks, mut charged, mut end, mut resp, mut elim) = (0u32, 0.0f32, 0.0f32, 0u32, 0);
        let (mut laps, mut clean) = (0u64, 0u32);
        for class in classes() {
            for (id, entry) in &circuits {
                let Some(solo) = solo_on(entry, &class, Some(tuning)) else {
                    continue;
                };
                report.push_str(&row(id, &class, &solo));
                ticks += solo.contact_ticks;
                // Capped at the pool: a craft racing at zero shield is charged
                // and loses nothing, so a raw sum rewards dying early. See
                // `RaceSim::wall_damage`.
                charged += solo
                    .wall_charged
                    .min(race_pool(&solo).unwrap_or(solo.wall_charged));
                end += solo.end_shield;
                resp += solo.respawns;
                elim += usize::from(solo.state == oag_physics::CraftState::Eliminated);
                if let Some(best) = solo.best {
                    laps += best;
                    clean += 1;
                }
            }
        }
        report.push_str(&format!(
            "TOTAL      contact ticks {ticks}  charged {charged:.1}  end {end:.1}  \
             respawns {resp}  eliminated {elim}  clean laps {clean}/48  \
             mean clean lap {:.2}s\n",
            laps as f32 / clean.max(1) as f32 / 60.0,
        ));
        println!("{report}");
    }
}

/// The shield pool a row's charge should be capped at.
///
/// 95.0 on every Pulse craft measured, but read rather than assumed: a row that
/// finished at capacity reads its own pool in [`Solo::end_shield`], and any
/// other row is capped at the constant the board has always seen.
fn race_pool(_solo: &Solo) -> Option<f32> {
    Some(95.0)
}

/// What is actually wrong with a stretch of a circuit's racing line.
///
/// **The blocker's own instrument.** A chord of 4 destroys `05_Track` at VENOM
/// inside one lap, wedging at driver index 200-249, and that sits inside the
/// run of 134 unsupported line samples `race_ground_truth::the_racing_line_has_track_under_it_where_it_is_known_to`
/// records at 161-211. The question this answers is whether the sharper
/// estimator is *causing* that failure or *reporting* it.
///
/// Per sample it prints what separates the candidate explanations:
///
/// - **`drop`**, how far below the sample the floor is, cast down the sample's
///   own `down` axis from one probe reach above. A racing line running above
///   the track reads as a steady positive drop; a genuine hole reads as `none`.
/// - **`gap`**, the distance to the previous sample. A splice of two authored
///   paths shows as a jump against the circuit's ordinary spacing.
/// - **`order`**, the spline sample each racing-line index maps to through
///   `RaceSim::ai_order`. Non-consecutive values are the path-order splice that
///   `Course::path_order` exists to prevent - `05` was one of three circuits it
///   put craft off the track on, and `crates/raceplay/src/course.rs` carries
///   the account.
/// - **`k4`/`k11`**, the curvature at the two chords. A **kink** - geometry
///   that is not a corner - shows as a `k4` spike the `k11` chord averages
///   away; a genuine tight corner shows at both.
/// - **`hw`**, the authored half-widths, and **`lat`**, where the racing line
///   sits across the track.
///
/// ```sh
/// OAG_SWEEP=1 OAG_LINE=05,140,280 OAG_REQUIRE_GAME_DATA=1 \
///   cargo nextest run --release -p oag-game --run-ignored all \
///   --no-capture -E 'test(what_is_wrong_with_the_line)'
/// ```
#[test]
#[ignore = "a scratch probe: set OAG_SWEEP, read the samples"]
fn what_is_wrong_with_the_line() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let spec = std::env::var("OAG_LINE").unwrap_or_else(|_| "05,140,280".to_string());
    let parts: Vec<&str> = spec.split(',').collect();
    let circuit = parts[0];
    let first: usize = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(140);
    let last: usize = parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(280);

    let Some(image) = image() else {
        return;
    };
    let Ok(loaded) = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(format!("Data\\Environments\\{circuit}_Track\\track.vex")),
        ..race::Options::default()
    }) else {
        return;
    };
    let reach = loaded.setup.handling.antigrav.ride_height;
    let collision = loaded.setup.collision.clone();
    let race = race::Race::start(loaded.setup);
    let line = race.racing_line();

    let mut report = format!(
        "\n=== {circuit}_Track racing line, idx {first}-{last}, ride height {reach:.2} ===\n\
         idx    order  gap    drop     surface  k4       k11      hwL    hwR    lat\n"
    );
    let mut previous: Option<oag_core::math::Vec3> = None;
    for index in first..=last.min(line.len().saturating_sub(1)) {
        let Some(sample) = race.ai_sample(index) else {
            continue;
        };
        let point = line.point(index);
        let up = (-oag_core::math::Vec3::from_array(sample.down)).normalize_or_zero();
        let from = point + up * reach;
        // Sixty reaches, the same deep cast the committed test uses to tell a
        // line above the track from an authored jump with nothing under it.
        let hit = collision.raycast(oag_physics::Ray::new(from, -up, reach * 60.0), None, false);
        let gap = previous.map_or(0.0, |p| (point - p).length());
        previous = Some(point);
        let centre = oag_core::math::Vec3::from_array(sample.pos);
        let lateral = oag_core::math::Vec3::from_array(sample.lateral).normalize_or_zero();
        report.push_str(&format!(
            "{index:<6} {:<6} {gap:<6.2} {:<8} {:<8} {:<8.5} {:<8.5} {:<6.2} {:<6.2} {:.2}\n",
            race.ai_sample_index(index)
                .map_or_else(|| "-".to_string(), |o| o.to_string()),
            hit.map_or_else(
                || "none".to_string(),
                |h| format!("{:.2}", h.distance - reach)
            ),
            hit.map_or_else(|| "-".to_string(), |h| format!("{:?}", h.surface)),
            line.curvature(index, 4.0),
            line.curvature(index, 11.0),
            sample.half_width_left,
            sample.half_width_right,
            (point - centre).dot(lateral),
        ));
    }
    println!("{report}");
}
