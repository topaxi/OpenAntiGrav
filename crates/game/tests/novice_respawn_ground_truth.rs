//! Why a Novice craft respawns 221-349 times on `13_Track`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-game \
//!   --run-ignored all novice_respawn_ground_truth
//! # the tier x circuit sweep is also OAG_SWEEP-gated, see sweep_novice_respawns
//! ```
//!
//! Flagged 2026-09-07 by the barrel-roll pass
//! (`docs/gameplay/ai.md`, "What it costs on the disc, measured") and out of
//! that pass's scope. This file is the follow-up: where the respawns happen,
//! whether the authored jump is the cause, and whether lowering
//! `Tuning::look_speed` from 0.35 to 0.30 (for the `13_Track` Ace S-bend, see
//! "The correction converges too slowly across a wide S-bend") made Novice
//! worse.

use std::path::{Path, PathBuf};

use oag_game::{catalogue, race};

/// Five minutes at 60 Hz, the window every other solo benchmark on this disc
/// uses.
const TICKS: u64 = 18_000;

/// Slot 0 and 2..8 are switched off in every run here, so slot 1 is a craft
/// alone with the circuit.
const LONE: usize = 1;

/// The circuit this whole investigation is about.
const TRACK_ID: &str = "13_Track";

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Every forward circuit on the disc, as `(id, entry name)`.
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

/// The WAD entry name for one circuit id, or `None` if the disc is absent or
/// the id is not on it.
fn entry_for(id: &str) -> Option<String> {
    circuits()
        .into_iter()
        .find(|(this_id, _)| this_id == id)
        .map(|(_, entry)| entry)
}

/// One airborne window: when it started and ended, in driver-line-index terms,
/// and how long it lasted.
#[derive(Debug, Clone, Copy)]
struct AirborneWindow {
    liftoff_index: u32,
    landing_index: u32,
    seconds: f32,
}

/// A full instrumented solo run: not just the pass/fail numbers
/// `race_ground_truth.rs`'s `Solo` carries, but where each respawn happened
/// and what the craft was doing airborne-wise.
#[derive(Debug, Default, Clone)]
struct Record {
    /// The driver's line index at the moment each respawn fired.
    respawn_indices: Vec<u32>,
    laps: u32,
    respawns: u32,
    /// Shield left at the end of the run, out of `capacity`.
    shield: f32,
    capacity: f32,
    /// The quickest lap with no recovery in it, in ticks.
    best: Option<u64>,
    airborne: Vec<AirborneWindow>,
    line_len: u32,
}

/// One lone craft, one circuit, one difficulty, with the *final* (already
/// tuned) shared tuning optionally overridden before the race starts - the
/// same pattern `ai_roll_ground_truth.rs`'s `rolls_on` uses, extended with
/// respawn-position and airborne-window instrumentation.
///
/// **`tuning` replaces `level.tune(&Tuning::default())` outright rather than
/// feeding through it** - a caller that wants "Novice, but with `look_speed`
/// at 0.35" has to build that final value itself
/// (`level.tune(&Tuning { look_speed: 0.35, ..Tuning::default() })`), and a
/// caller that wants to hold every Novice axis except one grip figure has to
/// be able to say so without that figure being re-scaled by `grip_believed`
/// a second time.
fn run(
    level: oag_ai::Difficulty,
    track_entry: &str,
    tuning: Option<oag_ai::Tuning>,
) -> Option<Record> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: level,
        track: Some(track_entry.to_string()),
        ..race::Options::default()
    })
    .ok()?;
    let mut race = race::Race::start(loaded.setup);
    if let Some(tuning) = tuning {
        race.set_ai_tuning(tuning);
    }
    for slot in 2..8 {
        race.world.ships[slot].active = false;
    }
    race.world.ships[0].active = false;

    let mut best: Option<u64> = None;
    let mut lap = race.world.ships[LONE].standing.lap;
    let mut started = 0u64;
    let mut recovered_this_lap = false;
    let mut respawn_indices = Vec::new();
    let mut airborne = Vec::new();
    let mut was_airborne = false;
    let mut window_start_index = 0u32;
    let mut window_ticks = 0u32;

    for tick in 0..TICKS {
        let before = race.respawns_of(LONE);
        let index_before = race.world.ships[LONE].driver.index;
        race.tick(&oag_gameplay::InputSnapshot::default());
        if race.respawns_of(LONE) != before {
            recovered_this_lap = true;
            respawn_indices.push(index_before);
        }
        let now = race.world.ships[LONE].standing.lap;
        if now != lap {
            if lap > 1 && !recovered_this_lap {
                let taken = tick - started;
                best = Some(best.map_or(taken, |held: u64| held.min(taken)));
            }
            started = tick;
            lap = now;
            recovered_this_lap = false;
        }

        let airborne_now = race.world.ships[LONE].physics.time_airborne > 0.0;
        let index_now = race.world.ships[LONE].driver.index;
        if airborne_now && !was_airborne {
            window_start_index = index_now;
            window_ticks = 0;
        }
        if airborne_now {
            window_ticks += 1;
        }
        if !airborne_now && was_airborne {
            airborne.push(AirborneWindow {
                liftoff_index: window_start_index,
                landing_index: index_now,
                seconds: window_ticks as f32 / 60.0,
            });
        }
        was_airborne = airborne_now;
    }

    Some(Record {
        respawn_indices,
        laps: lap,
        respawns: race.respawns_of(LONE),
        shield: race.world.ships[LONE].physics.shield,
        capacity: race.world.ships[LONE].handling.dimensions.shield,
        best,
        airborne,
        line_len: race.racing_line().len() as u32,
    })
}

/// Buckets a set of line indices into fixed-width ranges over `0..line_len`,
/// so a reader can see whether respawns cluster at one site or scatter.
fn histogram(indices: &[u32], line_len: u32, bucket: u32) -> Vec<(u32, u32, usize)> {
    let mut buckets: Vec<(u32, u32, usize)> = Vec::new();
    let mut start = 0u32;
    while start < line_len.max(bucket) {
        let end = (start + bucket).min(line_len);
        let count = indices
            .iter()
            .filter(|&&index| index >= start && index < end)
            .count();
        if count > 0 {
            buckets.push((start, end, count));
        }
        start += bucket;
    }
    buckets
}

/// Question 1 and 2: where a Novice craft respawns on `13_Track`, and whether
/// it lines up with the authored jump - line indices 19-49, per
/// `the_racing_line_has_track_under_it_where_it_is_known_to` in
/// `race_ground_truth.rs`.
///
/// Diagnostic only: prints the histogram and the airborne-window table rather
/// than asserting, because what to do about the finding is a separate
/// question from confirming it exists.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn where_the_novice_craft_respawns_on_13_track() {
    let Some(entry) = entry_for(TRACK_ID) else {
        return;
    };
    let Some(record) = run(oag_ai::Difficulty::Novice, &entry, None) else {
        return;
    };
    println!(
        "{TRACK_ID} novice: laps {} respawns {} shield {:.2}/{:.2} best {:?} line_len {}",
        record.laps, record.respawns, record.shield, record.capacity, record.best, record.line_len
    );
    println!("respawn histogram (bucket 25, index..index, count):");
    for (start, end, count) in histogram(&record.respawn_indices, record.line_len, 25) {
        let marker = if start < 50 {
            "  <- jump region (19-49)"
        } else {
            ""
        };
        println!("  {start:>5}..{end:<5} {count:>4}{marker}");
    }
    println!(
        "airborne windows: {} total, first 10:",
        record.airborne.len()
    );
    for window in record.airborne.iter().take(10) {
        println!(
            "  liftoff {:>5} landing {:>5} duration {:.2}s",
            window.liftoff_index, window.landing_index, window.seconds
        );
    }
    let jump_landings = record
        .airborne
        .iter()
        .filter(|window| window.landing_index < 50)
        .count();
    println!(
        "{jump_landings} of {} airborne windows land with driver index below 50 (the jump region)",
        record.airborne.len()
    );
    let in_jump_region = record
        .respawn_indices
        .iter()
        .filter(|&&index| index < 50)
        .count();
    println!(
        "{} of {} respawns ({:.0}%) fired with driver index below 50 (the jump region)",
        in_jump_region,
        record.respawn_indices.len(),
        100.0 * in_jump_region as f32 / record.respawn_indices.len().max(1) as f32
    );
}

/// Question 3: was this already true at `look_speed` 0.35, or did lowering it
/// to 0.30 for the Ace S-bend fix break Novice?
///
/// Diagnostic only, same reason as above.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn novice_13_track_before_and_after_look_speed() {
    let Some(entry) = entry_for(TRACK_ID) else {
        return;
    };
    for look_speed in [0.35f32, 0.30] {
        let measured = oag_ai::Tuning {
            look_speed,
            ..oag_ai::Tuning::default()
        };
        let tuning = oag_ai::Difficulty::Novice.tune(&measured);
        let Some(record) = run(oag_ai::Difficulty::Novice, &entry, Some(tuning)) else {
            return;
        };
        println!(
            "{TRACK_ID} novice look_speed={look_speed}: laps {} respawns {} shield {:.2}/{:.2} best {:?}",
            record.laps, record.respawns, record.shield, record.capacity, record.best
        );
    }
}

/// Question 4: a tier x circuit table of respawn counts, which nobody has had
/// before now.
///
/// `OAG_SWEEP`-gated rather than merely `#[ignore]`d - `just test-data` runs
/// ignored tests, and 4 tiers x 12 circuits x 18,000 ticks is minutes of wall
/// clock even in `--release`, which is how a suite stops being run.
///
/// ```sh
/// OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 \
///   cargo nextest run --release -p oag-game --run-ignored all \
///   sweep_novice_respawns --no-capture
/// ```
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1, read the table"]
fn sweep_novice_respawns() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let circuits = circuits();
    if circuits.is_empty() {
        return;
    }
    let mut report = String::from("\ncircuit      novice  skilled  elite  ace\n");
    for (id, entry) in &circuits {
        let mut row = format!("{id:<12}");
        for level in [
            oag_ai::Difficulty::Novice,
            oag_ai::Difficulty::Skilled,
            oag_ai::Difficulty::Elite,
            oag_ai::Difficulty::Ace,
        ] {
            let respawns = run(level, entry, None).map_or(-1, |record| record.respawns as i64);
            row.push_str(&format!(" {respawns:>7}"));
        }
        report.push_str(&row);
        report.push('\n');
    }
    println!("{report}");
}

/// Whether `Difficulty::grip_believed` is the actual bottleneck: sweeps the
/// grip a Novice-level driver believes it has, holding every other Novice
/// axis (`turn_allowed`, `mistakes`, `reaction_ticks`) fixed, and watches
/// whether `13_Track`'s jump stops swallowing the craft.
///
/// Diagnostic only - it does not change `Difficulty::grip_believed`, it tests
/// what would happen if it moved, so a mechanism claim has a number behind it
/// before anything is touched.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn novice_13_track_grip_sweep() {
    let Some(entry) = entry_for(TRACK_ID) else {
        return;
    };
    let mut report = String::from("\ngrip_believed  respawns  laps  shield\n");
    for grip in [0.30f32, 0.40, 0.48, 0.60, 0.70, 0.85, 1.0] {
        // Reproduce `Difficulty::Novice.tune()` by hand, with `grip_believed`
        // replaced - `turn_allowed` (0.60), `mistake_rate` and
        // `reaction_ticks` stay Novice's, so only the one axis under test
        // moves.
        let novice = oag_ai::Difficulty::Novice.tune(&oag_ai::Tuning::default());
        let tuning = oag_ai::Tuning {
            lateral_accel: oag_ai::Tuning::default().lateral_accel * grip,
            ..novice
        };
        let Some(record) = run(oag_ai::Difficulty::Novice, &entry, Some(tuning)) else {
            return;
        };
        report.push_str(&format!(
            "{grip:<14} {:<9} {:<5} {:.2}/{:.2}\n",
            record.respawns, record.laps, record.shield, record.capacity
        ));
    }
    println!("{report}");
}
