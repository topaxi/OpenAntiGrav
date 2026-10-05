//! Follow-up to `novice_respawn_ground_truth.rs`: **why** the grip sweep's
//! cliff between 0.40 and 0.48 exists, not just that it does - and, once that
//! is measured, whether anything in `oag-ai` closes it without moving
//! `Difficulty::grip_believed` itself. See
//! `docs/gameplay/ai.md`, "The jump-clearing failure" for the full writeup;
//! the short version is a real corner (not the gap) throttles the approach,
//! the threshold is `0.42..0.43` rather than the coarse sweep's `0.40..0.48`,
//! and the corner is geometrically capable of the needed speed at Novice's
//! own turn-rate degradation - so the shortfall is entirely
//! `grip_believed(Novice) = 0.30` being lower than this one jump needs, not a
//! controller bug.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-game \
//!   --run-ignored all novice_jump_ground_truth --no-capture
//! ```
//!
//! Diagnostic only: every test here prints and does not assert, because the
//! finding is a shape, not a pass/fail. This file only reads
//! `crates/ai`'s public surface and `crates/game`'s own `race` module - it
//! does not touch `crates/raceplay/src/`.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::catalogue;

const TRACK_ID: &str = "13_Track";
const LONE: usize = 1;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn entry_for(id: &str) -> Option<String> {
    let image = image()?;
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    catalogue::tracks(&definition)
        .into_iter()
        .filter(|track| !track.reversed)
        .find(|track| track.id == id)
        .map(|track| track.entry_name())
}

/// Question 2: is the jump's approach gated by a corner, and does
/// `grip_believed` change the speed the craft carries into it?
///
/// Dumps `Line::curvature` for every index `0..30` (span 4.0, well inside a
/// hull length, just to see the shape), then runs a lone Novice craft at two
/// `grip_believed` values either side of the measured cliff and logs its
/// actual forward speed at every driver-line index `0..26` on the **first**
/// pass - before any respawn has had a chance to re-teleport it - so the
/// speeds compared are the ones flown by an undisturbed approach.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn what_the_jumps_approach_looks_like() {
    let Some(entry) = entry_for(TRACK_ID) else {
        return;
    };
    let Some(image) = image() else { return };

    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Novice,
        track: Some(entry.clone()),
        ..race::Options::default()
    })
    .expect("loading 13_Track");
    let probe = race::Race::start(loaded.setup.clone());
    let line = probe.racing_line();
    println!("\ncurvature (span 4.0) and corridor by index, 0..55 on {TRACK_ID}:");
    for index in 0..55 {
        let aim = line.aim(index, 0.0);
        let corridor = aim
            .corridor
            .map(|frame| format!("left {:.2} right {:.2}", frame.left, frame.right))
            .unwrap_or_else(|| "none".to_string());
        let point = line.point(index);
        println!(
            "  {index:>3}: curvature {:.6}  point {point:?}  corridor {corridor}",
            line.curvature(index, 4.0)
        );
    }

    for grip in [0.30f32, 0.48] {
        let novice = oag_ai::Difficulty::Novice.tune(&oag_ai::Tuning::default());
        let tuning = oag_ai::Tuning {
            lateral_accel: oag_ai::Tuning::default().lateral_accel * grip,
            ..novice
        };
        let mut race = race::Race::start(loaded.setup.clone());
        race.set_ai_tuning(tuning);
        for slot in 2..8 {
            race.sim.world.ships[slot].active = false;
        }
        race.sim.world.ships[0].active = false;

        let mut seen = [false; 30];
        let mut logged = 0usize;
        // The grid start is not index 0 - it can be anywhere on a 3,084-point
        // line - so "started" only latches once the driver has actually been
        // seen inside the window this pass is about, and everything before
        // that (including the initial large index while still approaching
        // from the grid) is not logged as though it were part of it.
        let mut started = false;
        println!("\ngrip_believed {grip}: speed by index, first pass through 0..26:");
        for _tick in 0..18_000 {
            race.tick(&PlayerInputs::none());
            let ship = &race.sim.world.ships[LONE];
            let index = ship.driver.index as usize;
            if index < 5 {
                started = true;
            }
            if started && index < 30 && !seen[index] {
                seen[index] = true;
                let forward = ship.physics.body.forward();
                let speed = ship.physics.body.linear_velocity.dot(forward);
                println!(
                    "  index {index:>3}: speed {speed:.2} airborne {}",
                    ship.physics.time_airborne > 0.0
                );
                logged += 1;
            }
            if (started && index >= 26) || race.respawns_of(LONE) > 0 {
                break;
            }
        }
        println!("  ({logged} indices logged before landing or a respawn)");
    }
}

/// One lone Novice-tuned craft on `13_Track`, with `lateral_accel` scaled by
/// `grip` (everything else - `max_turn_rate`, `mistake_rate`,
/// `reaction_ticks` - held at Novice's own value), for 18,000 ticks. Returns
/// `(respawns, laps, liftoff_speed, first_landing_index)`, where
/// `liftoff_speed` is the forward speed at the first tick the driver is found
/// airborne and `first_landing_index` is the driver-line index where the
/// first airborne window ends.
fn run_grip(setup: &race::Setup, grip: f32) -> (u32, u32, f32, u32) {
    let novice = oag_ai::Difficulty::Novice.tune(&oag_ai::Tuning::default());
    let tuning = oag_ai::Tuning {
        lateral_accel: oag_ai::Tuning::default().lateral_accel * grip,
        ..novice
    };
    let mut race = race::Race::start(setup.clone());
    race.set_ai_tuning(tuning);
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }
    race.sim.world.ships[0].active = false;

    let mut liftoff_speed = -1.0f32;
    let mut landing_index = 0u32;
    let mut was_airborne = false;
    for _tick in 0..18_000u32 {
        race.tick(&PlayerInputs::none());
        let ship = &race.sim.world.ships[LONE];
        let airborne_now = ship.physics.time_airborne > 0.0;
        if airborne_now && !was_airborne && liftoff_speed < 0.0 {
            let forward = ship.physics.body.forward();
            liftoff_speed = ship.physics.body.linear_velocity.dot(forward);
        }
        if !airborne_now && was_airborne && landing_index == 0 {
            landing_index = ship.driver.index;
        }
        was_airborne = airborne_now;
        if race.respawns_of(LONE) > 0 {
            break;
        }
    }
    (
        race.respawns_of(LONE),
        race.sim.world.ships[LONE].standing.lap,
        liftoff_speed,
        landing_index,
    )
}

/// Question 2, the discriminating case the grip sweep alone cannot answer:
/// **is the jump reachable by raising grip at all**, or does
/// `Tuning::max_turn_rate` (scaled by `turn_allowed`, untouched by this
/// sweep) put a lower ceiling under the corner regardless of grip? If the
/// craft still lands short of index 50 once `lateral_accel` is scaled so far
/// past the sweep's clean rows that the grip term cannot possibly bind, the
/// yaw term is what is actually limiting the approach, and no amount of
/// throttle-side change in `oag-ai` reaches this jump.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn novice_13_track_grip_ceiling() {
    let Some(entry) = entry_for(TRACK_ID) else {
        return;
    };
    let Some(image) = image() else { return };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Novice,
        track: Some(entry.clone()),
        ..race::Options::default()
    })
    .expect("loading 13_Track");
    println!("\ngrip  respawns  laps  liftoff_speed  first_landing_index");
    for grip in [0.48f32, 1.0, 5.0, 20.0, 100.0] {
        let (respawns, laps, liftoff_speed, landing_index) = run_grip(&loaded.setup, grip);
        println!("{grip:<6}{respawns:<10}{laps:<6}{liftoff_speed:<15.2}{landing_index}");
    }
}

/// Question 2, the fine sweep the prior pass left unswept: `0.41..=0.47`, at
/// `0.01` steps, between the two points the coarse sweep already measured
/// (`0.40` at 350 respawns, `0.48` at 0). Turns "somewhere in that interval"
/// into an actual number.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn novice_13_track_grip_fine_sweep() {
    let Some(entry) = entry_for(TRACK_ID) else {
        return;
    };
    let Some(image) = image() else { return };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Novice,
        track: Some(entry.clone()),
        ..race::Options::default()
    })
    .expect("loading 13_Track");
    println!("\ngrip  respawns  laps  liftoff_speed  first_landing_index");
    let mut grip = 0.41f32;
    while grip <= 0.470_001 {
        let (respawns, laps, liftoff_speed, landing_index) = run_grip(&loaded.setup, grip);
        println!("{grip:<6.2}{respawns:<10}{laps:<6}{liftoff_speed:<15.2}{landing_index}");
        grip += 0.01;
    }
}
