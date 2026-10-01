//! What the player's craft does after it has crossed the line for the last time.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image under `data/images/`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     post_finish_ground_truth
//! ```
//!
//! The original keeps the race running behind the end-race panels and flies the
//! player's craft itself: measured on PPSSPP 2026-10-01 (Time Trial and Single Race
//! on `16_Track`, the pad released 43 frames before the line), the craft laps the
//! circuit for the whole 35 s logged, its control record rewritten by the AI from
//! the frame after the flag. See `docs/gameplay/after-the-finish.md`.
//!
//! The finish is reached with the operator's `--autopilot`, which is then switched
//! off, so what flies the craft afterwards is the finish-line rule and nothing the
//! test left on. Falsifier: with the `standing.finished()` term removed from
//! `Race::flown_for_the_player` the craft is left to a released pad and every test
//! below fails.

use std::path::PathBuf;

use oag_game::race;
use oag_gameplay::PlayerInputs;

const CAP: u64 = 30_000;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// A lone Time Trial craft flown to the line, then handed back to nobody.
fn finished_time_trial() -> Option<race::Race> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    while !race.finished() && race.sim.world.tick < CAP {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        race.finished(),
        "the Time Trial never finished in {CAP} ticks"
    );
    assert!(
        race.sim.world.ships[0].standing.finished(),
        "the lone craft ended the race by something other than the line"
    );
    race.set_autopilot(false);
    Some(race)
}

fn speed(race: &race::Race) -> f32 {
    race.sim.world.ships[0]
        .physics
        .body
        .linear_velocity
        .length()
}

#[test]
#[ignore = "needs a disc image under data/images/"]
fn the_craft_goes_on_lapping_with_no_input_after_the_line() {
    let Some(mut race) = finished_time_trial() else {
        return;
    };
    assert!(
        race.flown_for_the_player(0),
        "the finish line did not hand the craft to the driver"
    );
    let at_line = race.sim.world.ships[0].physics.body.position;
    let mut travelled = 0.0_f32;
    let mut previous = at_line;
    let mut slowest = f32::MAX;
    // The original logged 35 s; double it, so a craft that merely coasted on
    // the line's momentum for a few seconds cannot pass.
    for _ in 0..70 * 60 {
        race.tick(&PlayerInputs::none());
        let here = race.sim.world.ships[0].physics.body.position;
        travelled += (here - previous).length();
        previous = here;
        slowest = slowest.min(speed(&race));
    }
    println!(
        "after the line: {travelled:.0} units in 70 s, slowest {slowest:.1}, state {:?}",
        race.sim.world.ships[0].physics.craft_state
    );
    assert!(
        travelled > 3_000.0,
        "the craft covered only {travelled:.0} units in 70 s after the line; the original's \
         covered about 3,300 in 35 s"
    );
    assert!(
        slowest > 20.0,
        "the craft stopped or crawled (slowest {slowest:.1}) after the line; the original \
         held 90 or more for 35 s"
    );
    assert_eq!(
        race.sim.world.ships[0].physics.craft_state,
        oag_physics::CraftState::Racing,
        "the craft left the racing state after the line"
    );
}

#[test]
#[ignore = "needs a disc image under data/images/"]
fn the_result_is_frozen_at_the_line_while_the_race_runs_on() {
    let Some(mut race) = finished_time_trial() else {
        return;
    };
    let standing = race.sim.world.ships[0].standing;
    let board = race.results().cloned();
    let lap = race.sim.world.primary_race().lap;
    let ghost_lap = race.sim.world.ships[0].standing.best_lap_ticks;
    // Long enough for the craft to come all the way round to the line again, which
    // is the moment a lap counter that kept counting would move.
    for _ in 0..90 * 60 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(
        race.sim.world.ships[0].standing, standing,
        "the player's standing moved after the finish: its laps, splits and best lap are \
         frozen at the line (the original only records a lap in craft state 1)"
    );
    assert_eq!(race.sim.world.primary_race().lap, lap);
    assert_eq!(race.sim.world.ships[0].standing.best_lap_ticks, ghost_lap);
    assert_eq!(
        race.results().cloned(),
        board,
        "the board taken at the line was revised"
    );
    assert!(race.finished(), "the race un-finished itself");
}
