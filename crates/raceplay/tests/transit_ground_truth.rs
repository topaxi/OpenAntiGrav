//! A craft's jump along a real circuit (`oag_raceplay::transit`), the pieces a portal
//! pickup mod builds on, driven through the engine API on Pulse PSP EU's `16_Track`
//! (Venom, Single Race, empty grid, this project's driver flying):
//!
//! - a transit over the start line is a lap on its exit tick, on the standing and on the
//!   person's own clock alike, and the craft is frozen until then;
//! - the craft drives on from where it came out, with no rescue;
//! - a transit over the line on the final lap finishes the race on its exit tick;
//! - a clearance probe on the track reads its width.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored only \
//!     -E 'test(/transit_ground_truth::/)' --no-capture
//! ```

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::Button;
use oag_raceplay as race;
use oag_raceplay::transit::Distance;

const TRACK: &str = "Data\\Environments\\16_Track\\track.vex";

fn race_on_16() -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(TRACK.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        no_driver_for_class: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    Some(race)
}

fn throttle() -> PlayerInputs {
    let mut input = oag_gameplay::input::Input::new();
    input.begin_frame(Button::Cross.bit());
    PlayerInputs::single(oag_gameplay::InputSnapshot {
        buttons: input,
        ..oag_gameplay::InputSnapshot::new()
    })
}

/// Ticks with the throttle held until `done`, at most `limit` ticks.
fn drive_until(race: &mut race::Race, limit: u32, done: impl Fn(&race::Race) -> bool) {
    for _ in 0..limit {
        if done(race) {
            return;
        }
        race.tick(&throttle());
    }
    panic!("not reached in {limit} ticks");
}

/// A transit of most of a lap on, entered `ticks` before it exits: returns the stamp the
/// lap counters give its exit tick (the world's count after that tick's step).
fn jump(race: &mut race::Race, ticks: u16) -> u64 {
    let entry = race.sim.world.ships[0].physics.body.position;
    race.begin_transit(0, ticks, Distance::Lap(0.9), 0.95)
        .expect("a racing craft on the course");
    for _ in 1..ticks {
        race.tick(&throttle());
        assert_eq!(
            race.sim.world.ships[0].physics.body.position, entry,
            "frozen"
        );
        assert!(!race.ship_active(0), "not drawn");
    }
    race.tick(&throttle());
    let exit = race.sim.world.tick;
    assert_eq!(race.sim.world.ships[0].transit, None, "out after its ticks");
    let out = race.sim.world.ships[0].physics.body.position;
    assert!(
        out.distance(entry) > 100.0,
        "came out elsewhere: {out:?} from {entry:?}"
    );
    exit
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_transit_over_the_line_is_a_lap_and_a_finish_on_its_exit_tick() {
    let Some(mut race) = race_on_16() else {
        return;
    };
    // Off the grid, over the line once (the clock starts), and a few seconds into lap 1.
    drive_until(&mut race, 1200, |r| {
        r.sim.world.ships[0].standing.lap_start_tick.is_some()
    });
    for _ in 0..300 {
        race.tick(&throttle());
    }
    let ship = |r: &race::Race| r.sim.world.ships[0].clone();
    assert_eq!(ship(&race).standing.lap, 1);
    let respawns = race.respawns();

    let exit = jump(&mut race, 60);
    let standing = ship(&race).standing;
    assert_eq!(
        standing.lap, 2,
        "nine tenths of the lap on from three hundred ticks in"
    );
    assert_eq!(
        standing.lap_start_tick,
        Some(exit),
        "the lap is timed at the exit tick"
    );
    let person = &race.sim.world.race[0];
    assert_eq!(person.lap, 2);
    assert_eq!(person.lap_start_tick, exit, "the person's clock agrees");
    assert_eq!(
        person.lap_splits[0],
        standing.lap_splits[0].map(|t| t),
        "same lap 1 time"
    );

    // It drives on from where it came out.
    let progress = standing.progress.unwrap();
    for _ in 0..240 {
        race.tick(&throttle());
    }
    let after = ship(&race);
    let kmh = after.physics.body.linear_velocity.length() * oag_core::math::SPEED_TO_KMH;
    assert!(kmh > 100.0, "driving at {kmh} km/h");
    assert!(after.standing.progress.unwrap() > progress, "further round");
    assert_eq!(race.respawns(), respawns, "no rescue after the exit");

    // On to the last lap, then over the line by a transit: the finish is the exit tick.
    let target = race.sim.world.laps_target().expect("a lap target");
    while ship(&race).standing.lap < target {
        jump(&mut race, 2);
        for _ in 0..60 {
            race.tick(&throttle());
        }
    }
    for _ in 0..120 {
        race.tick(&throttle());
    }
    let exit = jump(&mut race, 30);
    let standing = ship(&race).standing;
    assert_eq!(
        standing.finish_tick,
        Some(exit),
        "finished on the exit tick"
    );
    assert!(race.sim.world.race[0].finished, "the person's race is over");
    println!(
        "laps {:?}, finish at {exit}, respawns {}",
        standing.lap_splits,
        race.respawns()
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_clearance_probe_reads_the_real_track() {
    let Some(mut race) = race_on_16() else {
        return;
    };
    drive_until(&mut race, 1200, |r| {
        r.sim.world.ships[0].standing.lap_start_tick.is_some()
    });
    let (fits, ahead) = race.clearance(0, Distance::Units(200.0), 4.0);
    let (wide, _) = race.clearance(0, Distance::Units(200.0), 60.0);
    println!("radius 4 fits {fits} at {ahead}; radius 60 fits {wide}");
    assert!(fits, "a small portal fits the road 200 units on");
    assert!(!wide, "a 120-unit-wide one does not");
    assert_eq!(ahead, 200.0);
}
