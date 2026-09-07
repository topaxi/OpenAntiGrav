//! Follow-up to `novice_respawn_ground_truth.rs`: **why** the grip sweep's
//! cliff between 0.40 and 0.48 exists, not just that it does.
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
//! does not touch `crates/game/src/race/`.

use std::path::{Path, PathBuf};

use oag_game::{catalogue, race};

const TRACK_ID: &str = "13_Track";
const LONE: usize = 1;

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

fn entry_for(id: &str) -> Option<String> {
    let image = image()?;
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_formats::fexml::expand(&blob).expect("expanding it");
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
            race.world.ships[slot].active = false;
        }
        race.world.ships[0].active = false;

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
            race.tick(&oag_gameplay::InputSnapshot::default());
            let ship = &race.world.ships[LONE];
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
