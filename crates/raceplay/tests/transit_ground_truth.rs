//! A craft's jump along a real circuit (`oag_raceplay::transit`), the pieces a portal
//! pickup mod builds on, driven through the engine API on Pulse PSP EU's `16_Track`
//! (Venom, Single Race, empty grid, this project's driver flying):
//!
//! - a transit over the start line is a lap on its exit tick, on the standing and on the
//!   person's own clock alike, and the craft is frozen until then;
//! - the craft drives on from where it came out, with no rescue;
//! - a transit over the line on the final lap finishes the race on its exit tick;
//! - a clearance probe on the track reads its width;
//! - an exit on a fork's route (Pulse `05_Track`) puts the craft on that route, with the
//!   driver on its line, and it drives the detour on to the rejoin instead of being sent
//!   back to the ring.
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
const FORKED: &str = "Data\\Environments\\05_Track\\track.vex";

fn race_on_16() -> Option<race::Race> {
    race_on(TRACK)
}

fn race_on(track: &str) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(track.to_string()),
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
    let ship = |r: &race::Race| r.sim.world.ships[0];
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
        person.lap_splits[0], standing.lap_splits[0],
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

/// The nearest the craft is to a sample of course route `route` and to a ring point, in units.
fn offsets(race: &race::Race, route: usize) -> (f32, f32) {
    let course = race.course().expect("a course");
    let at = race.sim.world.ships[0].physics.body.position;
    let nearest = |points: &mut dyn Iterator<Item = oag_core::math::Vec3>| {
        points.map(|p| p.distance(at)).fold(f32::INFINITY, f32::min)
    };
    (
        nearest(&mut course.routes()[route].positions.iter().copied()),
        nearest(&mut (0..course.len()).filter_map(|i| course.position(i))),
    )
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn an_exit_on_a_forks_route_seats_the_craft_and_its_driver_on_the_route() {
    let Some(mut race) = race_on(FORKED) else {
        return;
    };
    let (route, half) = {
        let course = race.course().expect("a course");
        let route = &course.routes()[0];
        let span = (course.progress_at(route.merge).unwrap()
            - course.progress_at(route.split).unwrap())
        .rem_euclid(course.length());
        (0usize, course.advance(route.split, span * 0.4))
    };
    // Past the first lap's gate, so the move is a plain jump.
    drive_until(&mut race, 1200, |r| {
        r.sim.world.ships[0].standing.lap_start_tick.is_some()
    });
    let exit = race::transit::Exit {
        to: half,
        route: u16::try_from(route + 1).unwrap(),
    };
    assert!(race.move_to(0, exit, 150.0));
    let driver = race.sim.world.ships[0].driver;
    assert_eq!(
        usize::from(driver.branching.route),
        route + 1,
        "the driver is on the route's line"
    );
    assert!(driver.branching.entered, "past the split");
    let (on_route, on_ring) = offsets(&race, route);
    assert!(
        on_route < 25.0 && on_route * 3.0 < on_ring,
        "put on the route's deck: {on_route:.1} from it, {on_ring:.1} from the ring"
    );

    // It drives the detour: still the route's driver a second on, never back at the ring.
    let respawns = race.respawns();
    for _ in 0..60 {
        race.tick(&throttle());
    }
    assert_eq!(race.respawns(), respawns, "no rescue");
    let driver = race.sim.world.ships[0].driver;
    assert_eq!(usize::from(driver.branching.route), route + 1);
    let (on_route, on_ring) = offsets(&race, route);
    assert!(on_route * 3.0 < on_ring, "{on_route:.1} vs {on_ring:.1}");

    // The same ring point as a ring exit is the ring: the route is the exit's own to name.
    let mut ring = race_on(FORKED).unwrap();
    drive_until(&mut ring, 1200, |r| {
        r.sim.world.ships[0].standing.lap_start_tick.is_some()
    });
    assert!(ring.move_to(0, race::transit::Exit::ring(half), 150.0));
    assert_eq!(ring.sim.world.ships[0].driver.branching.route, 0);
}
