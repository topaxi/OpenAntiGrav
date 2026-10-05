//! A full field spreads across both sides of every fork, and no craft gains or
//! loses a lap doing it.
//!
//! **The maintainer's observation, as an assertion**: before 2026-10-05 every
//! craft followed the ring and so took the same side at every fork. The
//! original flips a fair coin per craft per fork (`Ai_ChooseBranch`,
//! `0x08854920`, `docs/ghidra/functions/psp-pulse-usa/ai-branch-choice.md`),
//! so over a few laps of a full grid every way round should be taken. Drop the
//! coin (`oag_raceplay`'s `steer_branching`) and every fork reads "ring only".
//!
//! **`#[ignore]`d and never run in CI**: it needs the Pulse disc and the 2048
//! package. Run with `just test-data`.

use std::collections::BTreeMap;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// What a field did at the forks: per pre-fork path, how often each line was
/// chosen (`0` the ring, `k` route `k - 1`).
#[derive(Debug, Default)]
struct Forks {
    chosen: BTreeMap<u16, BTreeMap<u16, u32>>,
    laps: Vec<u32>,
    /// Craft destroyed during the run, left out of the lap check. Altima's
    /// slot 5 is destroyed on lap 1 with or without routes (checked by
    /// disabling the coin, 2026-10-05), so it is the race, not the forks.
    wrecked: Vec<bool>,
}

/// Runs a full grid, the player's craft on autopilot, for `ticks`, tallying
/// every decision and checking the lap count as it goes.
fn run(source: &str, track: &str, ticks: u32) -> Option<(Forks, usize)> {
    let loaded = race::load(&race::Options {
        source: source.to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("loading {track}: {e:#}"));
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    let count = race.ship_count() as usize;
    assert!(count >= 2, "{track}: no field to split");
    let routes = race.route_count();
    let mut forks = Forks::default();
    let mut decided = vec![0u16; count];
    let mut laps: Vec<u32> = (0..count)
        .map(|s| race.sim.world.ships[s].standing.lap)
        .collect();
    for tick in 0..ticks {
        race.tick(&PlayerInputs::none());
        for slot in 0..count {
            let ship = &race.sim.world.ships[slot];
            let branching = ship.driver.branching;
            if branching.decided_at != 0 && branching.decided_at != decided[slot] {
                *forks
                    .chosen
                    .entry(branching.decided_at - 1)
                    .or_default()
                    .entry(branching.route)
                    .or_default() += 1;
            }
            decided[slot] = branching.decided_at;
            // A lap is counted one at a time and never taken back.
            let lap = ship.standing.lap;
            assert!(
                lap == laps[slot] || lap == laps[slot] + 1,
                "{track}: slot {slot} went from lap {} to {lap} at tick {tick} \
                 (route {}, progress {:?})",
                laps[slot],
                branching.route,
                ship.standing.progress
            );
            laps[slot] = lap;
        }
    }
    forks.laps = laps;
    forks.wrecked = (0..count)
        .map(|s| race.sim.world.ships[s].physics.craft_state != oag_physics::CraftState::Racing)
        .collect();
    for slot in 0..count {
        let ship = &race.sim.world.ships[slot];
        println!(
            "{track}: slot {slot} lap {} route {} respawns {} state {:?} progress {:?} index {}",
            ship.standing.lap,
            ship.driver.branching.route,
            race.respawns_of(slot),
            ship.physics.craft_state,
            ship.standing.progress,
            ship.driver.index
        );
    }
    Some((forks, routes))
}

/// Both sides of every fork were taken, and the field's lap counts agree.
fn assert_split(track: &str, forks: &Forks, routes: usize, forks_expected: usize) {
    println!(
        "{track}: {routes} route(s), decisions {:?}, laps {:?}",
        forks.chosen, forks.laps
    );
    assert_eq!(
        forks.chosen.len(),
        forks_expected,
        "{track}: decisions were made at {} fork(s): {:?}",
        forks.chosen.len(),
        forks.chosen
    );
    for (pre_fork, lines) in &forks.chosen {
        assert!(
            lines.contains_key(&0),
            "{track}: nobody stayed on the ring at the fork after path {pre_fork}: {lines:?}"
        );
        assert!(
            lines.keys().any(|&line| line != 0),
            "{track}: nobody took a route at the fork after path {pre_fork}: {lines:?}"
        );
    }
    // Nobody gained or lost a whole lap on a route: a field driving the same
    // physics finishes within a lap of itself over a few laps.
    let running = || {
        forks
            .laps
            .iter()
            .zip(&forks.wrecked)
            .filter(|(_, wrecked)| !**wrecked)
            .map(|(lap, _)| *lap)
    };
    let min = running().min().unwrap_or(0);
    let max = running().max().unwrap_or(0);
    assert!(
        min >= 2,
        "{track}: a craft only reached lap {min}: {:?}",
        forks.laps
    );
    assert!(
        max - min <= 1,
        "{track}: laps spread {min}..={max}: {:?}",
        forks.laps
    );
}

#[test]
#[ignore = "needs the Pulse disc in data/images/"]
fn a_pulse_field_splits_at_the_fork_on_05_track() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let source = image.display().to_string();
    let mut archives = oag_pulse::open(&source).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    let track = race::catalogue::tracks(&definition)
        .into_iter()
        .find(|t| t.id == "05_Track" && !t.reversed)
        .expect("05_Track is on the disc")
        .entry_name();
    let Some((forks, routes)) = run(&source, &track, 60 * 150) else {
        return;
    };
    assert_eq!(routes, 1, "05_Track forks once");
    assert_split("05_Track", &forks, routes, 1);
}

#[test]
#[ignore = "needs the decrypted 2048 package in data/extracted/vita/"]
fn a_2048_field_splits_at_both_forks_on_altima() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let source = source.display().to_string();
    let Some((forks, routes)) = run(&source, oag_2048::race::DEFAULT_TRACK, 60 * 210) else {
        return;
    };
    assert_eq!(routes, 2, "altima forks twice");
    assert_split("altima", &forks, routes, 2);
}

/// A craft whose driver chose the route but that is physically on the ring's
/// side of the fork - shoved across the divider - follows the ring rather
/// than steering back through the wall: `Ai_ChooseBranch`'s re-commit
/// (`0x08854ae4`). Without it the driver would aim at a line up to 108 units
/// away on `05_Track`.
#[test]
#[ignore = "needs the Pulse disc in data/images/"]
fn a_craft_shoved_onto_the_other_side_of_a_fork_follows_that_side() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let source = image.display().to_string();
    let mut archives = oag_pulse::open(&source).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    let track = race::catalogue::tracks(&definition)
        .into_iter()
        .find(|t| t.id == "05_Track" && !t.reversed)
        .expect("05_Track is on the disc")
        .entry_name();
    let loaded = race::load(&race::Options {
        source,
        mode: oag_race::Mode::SingleRace,
        track: Some(track),
        ..race::Options::default()
    })
    .expect("loading 05_Track");
    let mut race = race::Race::start(loaded.setup);
    for slot in 2..8 {
        race.sim.world.ships[slot].active = false;
    }

    // The ring point in the bypassed stretch furthest from the route.
    let course = race.course().expect("a ring").clone();
    let route = &course.routes()[0];
    let n = course.len();
    let span = (route.merge + n - route.split) % n;
    let (ring_at, stray) = (0..span)
        .map(|i| (route.split + i) % n)
        .map(|i| {
            let p = course.position(i).expect("in range");
            let d = route
                .positions
                .iter()
                .map(|q| (*q - p).length())
                .fold(f32::INFINITY, f32::min);
            (i, d)
        })
        .fold((0, 0.0f32), |a, b| if b.1 > a.1 { b } else { a });
    assert!(
        stray > 50.0,
        "the route never leaves the ring's side: {stray:.1}"
    );

    let on_route = race.line_of(1).len() - route.len() / 2;
    let position = course.position(ring_at).expect("in range");
    let tangent = course.tangent(ring_at).expect("in range");
    {
        let ship = &mut race.sim.world.ships[1];
        ship.driver.branching = oag_ai::branch::Branching {
            route: 1,
            decided_at: 0,
            entered: true,
            visits: 1,
        };
        ship.driver.index = on_route as u32;
        ship.physics.body.position = position;
        ship.physics.body.linear_velocity = tangent * 50.0;
    }
    race.tick(&PlayerInputs::none());
    let branching = race.sim.world.ships[1].driver.branching;
    assert_eq!(
        branching.route, 0,
        "a craft {stray:.0} units from its route, on the ring, still follows the route"
    );
}
