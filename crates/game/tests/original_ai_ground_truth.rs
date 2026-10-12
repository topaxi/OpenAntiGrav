//! The campaign's original opponent (`oag_ai::original`) flown on the disc.
//!
//! The scenario is the live capture's on `ai-controls.md`, "Measured": Pulse
//! PSP USA, Talon's Junction (`16_Track`), VENOM, a full grid, the player
//! parked on the grid in eighth, skill `0.9` at level 0 and `3.0` at level 2.
//! Each test reads a figure the capture logged, chosen so it depends on the
//! law, the tables and the field's standings and not on this engine's
//! physics matching the original's lap for lap: the racer one place ahead of
//! the parked player is its own formation reference (gap `0`), and a racer
//! far behind its slot sits on the `+30 %` stop. What does depend on the
//! physics (how fast the field closes into formation) is recorded on
//! `docs/gameplay/ai.md` as open, not asserted here.

use oag_gameplay::PlayerInputs;

fn race(level: usize, skill: f32) -> Option<oag_raceplay::Race> {
    let image = oag_testdata::image("pulse-psp-usa.chd")?;
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(r"Data\Environments\16_Track\track.vex".to_string()),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading 16_Track");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    assert!(
        race.use_original_ai(Some(skill), level),
        "VENOM's AI tables read"
    );
    Some(race)
}

/// Seconds since the green light, the race time the law reads.
fn race_time(race: &oag_raceplay::Race) -> f32 {
    race.sim
        .world
        .tick
        .saturating_sub(oag_race::COUNTDOWN_TICKS) as f32
        / 60.0
}

/// Every opponent's held throttle, with its place, on every tick of
/// `from..=to` seconds.
fn held(level: usize, skill: f32, from: f32, to: f32) -> Option<(Vec<(u8, f32)>, Vec<f32>)> {
    let mut race = race(level, skill)?;
    let mut out = Vec::new();
    let mut engine = Vec::new();
    while race_time(&race) <= to {
        race.tick(&PlayerInputs::none());
        if race_time(&race) >= from {
            let places = race.places();
            let field = race.original_ai_field().expect("the field is on");
            out.extend((1..8).map(|s| (places[s], field.drivers[s].held_thrust)));
            // The throttle state the engine read this tick: above `100` it
            // lifts the acceleration cap, so nothing on the way may clamp it.
            engine.extend((1..8).map(|s| race.sim.world.ships[s].physics.thrust));
        }
    }
    Some((out, engine))
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// `hard-park` logged the racer one place ahead of the parked player at
/// `75.0` on every one of its turns from `8.48` s, and the racers far behind
/// their slots at `101.7`: `AIThrust` at the player's place, `WhenBehind` at
/// its clamp, Point3 of the skill curve, the `+30 %` stop.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn at_hard_the_reference_racer_and_the_upper_stop_read_what_the_original_logged() {
    let Some((samples, engine)) = held(2, 3.0, 8.5, 9.3) else {
        return;
    };
    let reference: Vec<f32> = samples
        .iter()
        .filter(|(place, _)| *place == 7)
        .map(|&(_, t)| t)
        .collect();
    assert!(!reference.is_empty());
    assert!(
        reference.iter().all(|&t| close(t, 75.0)),
        "place 7 read {reference:?}"
    );
    assert!(
        samples.iter().any(|&(_, t)| close(t, 101.7)),
        "nobody on the upper stop"
    );
}

/// `easy-park` and `easy-auto` both logged `34.6` as the lowest throttle at
/// skill `0.9`: the lower stop with `WhenBehind` at its clamp.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn at_easy_the_lower_stop_reads_what_the_original_logged() {
    let Some((samples, _)) = held(0, 0.9, 64.6, 70.6) else {
        return;
    };
    let lowest = samples
        .iter()
        .map(|&(_, t)| t)
        .fold(f32::INFINITY, f32::min);
    assert!(close(lowest, 34.6), "lowest throttle {lowest}");
}

/// The field gets round on the player's physics with the rail on: four laps
/// in 200 s, nobody put back on the track.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_original_field_gets_round_talons_junction_without_a_respawn() {
    let Some(mut race) = race(0, 0.9) else {
        return;
    };
    // Shield read at each opponent's lap crossings: a per-lap figure, not
    // just the end of the run.
    let mut lap = [1u32; 8];
    let mut per_lap: [Vec<f32>; 8] = Default::default();
    for _ in 0..60 * 200 {
        race.tick(&PlayerInputs::none());
        for slot in 1..8 {
            let ship = &race.sim.world.ships[slot];
            if ship.standing.lap != lap[slot] {
                lap[slot] = ship.standing.lap;
                per_lap[slot].push(ship.physics.shield);
            }
        }
    }
    for slot in 1..8 {
        let ship = &race.sim.world.ships[slot];
        println!(
            "slot {slot}: lap {} respawns {} wall-contact ticks {} shield at each crossing {:?}, end {:.1}",
            ship.standing.lap,
            race.respawns_of(slot),
            race.wall_contact_ticks_of(slot),
            per_lap[slot],
            ship.physics.shield
        );
        assert!(
            ship.standing.lap >= 4,
            "slot {slot} on lap {}",
            ship.standing.lap
        );
        assert_eq!(race.respawns_of(slot), 0, "slot {slot}");
    }
}

/// Only a campaign launch switches it on: a race that never asks keeps this
/// project's driver.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_race_that_does_not_ask_keeps_this_projects_driver() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(r"Data\Environments\16_Track\track.vex".to_string()),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading 16_Track");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    for _ in 0..600 {
        race.tick(&PlayerInputs::none());
    }
    assert!(!race.original_ai_active());
    assert!(race.original_ai_field().is_none());
}

/// HD/Fury ships the same tables, but its driver differs (four changes to the
/// thrust law, the rest unread): its campaign keeps this project's driver
/// until a lane of its own wires it (`oag_title::RaceDefaults::original_opponent`).
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_keeps_this_projects_driver() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(oag_hd::race::DEFAULT_TRACK.to_string()),
        ..oag_raceplay::Options::default()
    })
    .expect("loading HD's default circuit");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    assert!(!race.use_original_ai(Some(1.0), 1));
    assert!(!race.original_ai_active());
}
