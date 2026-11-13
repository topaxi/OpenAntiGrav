//! A Detonator race on HD/Fury runs its fourteen stages, one per lap, and ends on the fifteenth.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^detonator_ground_truth::/)'
//! ```
//!
//! The law is `Detonator_UpdateRace`'s (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mode.md`):
//! the stage opens at `1`, steps on each completed lap and the race is over at `15`. The
//! player flies alone on the autopilot's racing line (`Race::set_autopilot`), and the run
//! stops on the tick the race finishes.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

fn load() -> Option<race::Loaded> {
    let path = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    Some(
        race::load(&race::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::Detonator,
            ..race::Options::default()
        })
        .expect("loading an HD Detonator race"),
    )
}

/// The load takes Detonator's own grade table, opens on stage `1`, and fields nobody else.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_race_loads_its_own_stage_table() {
    let Some(loaded) = load() else {
        return;
    };
    let grade = loaded
        .zone_grade
        .as_ref()
        .expect("HD ships /data/environments/detonatormode.effectsettings");
    assert!(
        grade.describe().contains("detonatormode"),
        "graded from {}",
        grade.describe()
    );
    assert_eq!(grade.last_stage(), 14);
    assert_eq!(grade.blend().current, 1);
    let (decoded, numbered) = grade.stage_art_counts();
    assert_eq!(
        (decoded, numbered),
        (15, 15),
        "detonatormodetrack0..14 decode"
    );
}

/// **The stage counter.** Stage `1` until the first lap is completed, then one step per lap,
/// and the race finishes on the tick the fourteenth lap completes - stage `15` - not before.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_detonator_race_steps_a_stage_per_lap_and_ends_at_fifteen() {
    const LONE: usize = 0;
    let Some(loaded) = load() else {
        return;
    };
    let mut race = race::Race::start(loaded.setup);
    // The player flies their own racing line (`Race::set_autopilot`); the rules are the race's.
    race.set_autopilot(true);
    assert_eq!(race.sim.world.race[LONE].mode, oag_race::Mode::Detonator);
    assert_eq!(race.sim.world.race[LONE].laps_target, Some(14));

    let stage = |race: &race::Race| {
        oag_race::Mode::detonator_stage(race.sim.world.race[LONE].laps_completed())
    };
    assert_eq!(stage(&race), 1);
    let mut seen = vec![stage(&race)];
    let mut shield_per_lap = Vec::new();
    // Fourteen laps of an HD circuit at Venom are well under ten minutes of race.
    for _ in 0..60 * 60 * 10 {
        race.tick(&PlayerInputs::none());
        let now = stage(&race);
        if now != *seen.last().expect("one stage at least") {
            shield_per_lap.push(race.sim.world.ships[LONE].physics.shield);
            seen.push(now);
        }
        if race.sim.world.race[LONE].finished {
            break;
        }
    }
    println!("stages seen: {seen:?}");
    println!("per-lap shield (at each line crossing): {shield_per_lap:?}");
    println!(
        "end-of-run shield: {}",
        race.sim.world.ships[LONE].physics.shield
    );
    assert!(race.sim.world.race[LONE].finished, "the race ends");
    assert_eq!(
        seen,
        (1..=15).collect::<Vec<u32>>(),
        "one stage per lap, 1 to 15"
    );
    assert_eq!(race.sim.world.race[LONE].laps_completed(), 14);
}
