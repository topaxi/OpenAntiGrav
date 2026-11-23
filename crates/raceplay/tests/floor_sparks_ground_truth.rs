//! A frictionless contact on HD's own circuits throws the disc's own
//! `WO_SHIP_COLL_SPARK_NODAMAGE` (`docs/ghidra/functions/ps3-hdfury-eu/collision-cues.md`,
//! "which records carry no friction").
//!
//! Chenghou Project's autopilot lap, whose ramp landings meet a floor: the seat's craft lands at
//! tick 3273 (three records) and again at 5466, and a floor contact is rare enough that a lap
//! holds a handful of them (`hd_chenghou_jumps_sweep`). `#[ignore]`d: needs the disc images,
//! never runs in CI.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay floor_sparks_ground_truth:: --run-ignored all --no-capture
//! ```

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// The first landing is at tick 3273; the run stops at the first spark.
const TICKS: u64 = 6_000;

fn started(
    image: &str,
    track: Option<&str>,
    class: &str,
    team: Option<&str>,
) -> Option<(u32, bool)> {
    let image = oag_testdata::image(image)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        track: track.map(str::to_string),
        team: team.map(str::to_string),
        difficulty: oag_ai::Difficulty::Ace,
        seed: Some(1),
        ..race::Options::default()
    })
    .expect("loading");
    let loaded_effect = !loaded
        .report
        .iter()
        .any(|line| line.contains("WO_SHIP_COLL_SPARK_NODAMAGE") && line.contains("missing"));
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    race.core.sim.set_autopilot(true);
    for _ in 0..TICKS {
        race.tick(&PlayerInputs::none());
        if race.floor_sparks_started_for_tests() > 0 {
            break;
        }
    }
    Some((race.floor_sparks_started_for_tests(), loaded_effect))
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_throws_the_no_damage_spark_on_a_floor_contact() {
    let Some((count, loaded)) = started(
        "data/images/hdfury-ps3-eu-dec.iso",
        Some("/data/environments/04_chenghou_project/track.vex"),
        "VENOM",
        Some("Piranha"),
    ) else {
        return;
    };
    assert!(loaded, "WO_SHIP_COLL_SPARK_NODAMAGE did not load");
    assert!(
        count > 0,
        "no floor contact threw the spark in {TICKS} ticks"
    );
}
