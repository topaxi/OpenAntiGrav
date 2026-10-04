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
    finished_time_trial_at(oag_ai::Difficulty::default())
}

/// [`finished_time_trial`] at `difficulty`, which sets the skill scale the
/// finished craft's thrust is read at.
fn finished_time_trial_at(difficulty: oag_ai::Difficulty) -> Option<race::Race> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        difficulty,
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

/// The ten `Camera` nodes of `16_Track` as the original's own camera object held them while
/// the race ran on behind the panels (PPSSPP, 2026-10-01): `(eye, aim)` read from
/// `*(0x08b32c64 + 0x40 + 4 * n) + 0x90` and `+ 0xa0`, in list order.
const ORIGINAL_NODES_16_TRACK: [([f32; 3], [f32; 3]); 10] = [
    ([-335.4, -0.0, 94.1], [-233.9, -74.6, 53.6]),
    ([-817.8, 47.6, 118.8], [-512.1, -20.5, 132.8]),
    ([-618.5, 6.9, -163.2], [-753.3, 0.0, -100.6]),
    ([-535.1, -32.6, -617.1], [-574.8, -36.9, -470.2]),
    ([-105.9, -13.0, -796.2], [-303.5, -50.3, -659.0]),
    ([-433.8, -34.2, -347.0], [-253.3, -60.3, -536.1]),
    ([32.2, -23.4, -202.0], [-374.9, -46.1, -187.5]),
    ([468.3, -22.8, -39.9], [344.1, -43.2, -137.1]),
    ([531.5, 12.8, 234.4], [592.0, -14.8, 123.2]),
    ([62.8, -34.7, 162.2], [305.2, -39.5, 233.1]),
];

#[test]
#[ignore = "needs a disc image under data/images/"]
fn the_circuits_camera_nodes_are_the_ones_the_original_holds() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(r"Data\Environments\16_Track\track.vex")
        .expect("the circuit's track.vex");
    let nodes = oag_vex::camera::cameras(&blob);
    assert_eq!(nodes.len(), ORIGINAL_NODES_16_TRACK.len());
    for (index, (node, (eye, aim))) in nodes.iter().zip(ORIGINAL_NODES_16_TRACK).enumerate() {
        let dist = |a: [f32; 3], b: [f32; 3]| {
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
        };
        assert!(
            dist(node.position(), eye) < 0.1,
            "node {index}: eye {:?} against {eye:?}",
            node.position()
        );
        assert!(
            dist(node.aim, aim) < 0.1,
            "node {index}: aim {:?} against {aim:?}",
            node.aim
        );
    }
}

#[test]
#[ignore = "needs a disc image under data/images/"]
fn the_spectator_camera_takes_over_sixty_one_frames_after_the_line() {
    let Some(mut race) = finished_time_trial() else {
        return;
    };
    let finish = race.sim.world.ships[0]
        .standing
        .finish_tick
        .expect("finished");
    let nodes: Vec<oag_core::math::Vec3> = oag_vex::camera::cameras(
        &oag_pulse::open(&image().expect("image").display().to_string())
            .expect("mounting the disc")
            .read_name(r"Data\Environments\16_Track\track.vex")
            .expect("track.vex"),
    )
    .iter()
    .map(|camera| oag_core::math::Vec3::from_array(camera.position()))
    .collect();
    let mut cuts = 0;
    let mut last_eye = None;
    for since in 1..=1500_u64 {
        race.tick(&PlayerInputs::none());
        let eye = race.camera_position();
        if since < race::finish_camera::START_TICKS {
            assert!(
                nodes.iter().all(|node| (*node - eye).length() > 1.0),
                "the camera was already on a node at frame {since}"
            );
        } else if let Some(mode) = race.spectator_mode() {
            if mode.is_node_camera() {
                assert!(
                    nodes.iter().any(|node| (*node - eye).length() < 0.01),
                    "frame {since}: a node camera is not at a node: {eye:?}"
                );
            }
            if last_eye.is_some_and(|last| last != eye) {
                cuts += 1;
            }
            last_eye = Some(eye);
        }
        assert_eq!(race.sim.world.tick - finish, since);
    }
    assert!(
        race.spectator_mode().is_some(),
        "the director never started"
    );
    println!(
        "{cuts} eye changes in 1500 frames, {} camera cuts",
        race.camera_cuts()
    );
}

/// The finished craft flies at the original's own post-finish thrust, not at the
/// driver's racing pace.
///
/// Measured 2026-10-04 (`docs/ghidra/functions/psp-pulse-usa/race-finish.md`): Venom,
/// Easy, Talon's Junction, the original's throttle word after a first place reads `56.7`
/// on most frames and never more, and the craft covered `3,500-3,900` units in the 35 s
/// logged. A lone Time Trial craft is first, and Easy is `Novice` here. Falsifier:
/// without `Race::finished_thrust_cap` in the tick the throttle reads the driver's own
/// `100` and the first assertion on it fails. The distance is a sanity band only: at
/// `Novice` the driver's own pace lands inside it too.
#[test]
#[ignore = "needs a disc image under data/images/"]
fn the_finished_craft_flies_at_the_originals_post_finish_thrust() {
    let Some(mut race) = finished_time_trial_at(oag_ai::Difficulty::Novice) else {
        return;
    };
    let cap = race
        .finished_thrust_cap(0)
        .expect("Venom's AIRaceStats and 16_Track's stats.xml are on the disc");
    assert!(
        (cap - 0.567).abs() < 0.0005,
        "first place at Venom Easy: the original read 56.7 %, the law gives {cap}"
    );
    let mut travelled = 0.0_f32;
    let mut previous = race.sim.world.ships[0].physics.body.position;
    let (mut highest, mut at_cap) = (0.0_f32, 0_u32);
    let ticks = 35 * 60;
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
        let here = race.sim.world.ships[0].physics.body.position;
        travelled += (here - previous).length();
        previous = here;
        let throttle = race.sim.world.ships[0].physics.thrust;
        highest = highest.max(throttle);
        at_cap += u32::from((throttle - 100.0 * cap).abs() < 0.01);
    }
    println!(
        "after the line at the law's thrust: {travelled:.0} units in 35 s, throttle at most \
         {highest:.1}, at the cap on {at_cap} of {ticks} ticks"
    );
    assert!(
        highest <= 100.0 * cap + 0.01,
        "the finished craft's throttle reached {highest:.1}; the original's never passed 56.7"
    );
    assert!(
        at_cap * 2 > ticks,
        "the throttle sat at the cap on only {at_cap} of {ticks} ticks; the original's did on most"
    );
    assert!(
        (3_000.0..=4_200.0).contains(&travelled),
        "the finished craft covered {travelled:.0} units in 35 s; the original 3,500-3,900"
    );
}
