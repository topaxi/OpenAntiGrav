//! HD/Fury's campaign opponent (`oag_ai::original` with HD's
//! `oag_hd::race::ORIGINAL_DRIVER`) flown on the disc.
//!
//! The scenario is the RPCS3 capture on
//! `docs/ghidra/functions/ps3-hdfury-eu/ai-controls.md`, "Measured": Talon's
//! Junction forward, VENOM, a full grid, the player parked on the grid in
//! eighth, skill `-0.07` at level 0 and `3.48` at level 2 (the Fury cell
//! `grid8_3_1` at Easy and Hard). Each figure asserted is one the capture
//! logged and one that depends on the law, the tables and the standings, not
//! on this engine's physics matching HD's lap for lap: the racer one place
//! ahead of the parked player is its own formation reference, and a racer far
//! from its slot sits on a `+/-30 %` stop.

use oag_gameplay::PlayerInputs;

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("hdfury-ps3-eu-dec.iso")
}

fn race(level: usize, skill: f32) -> Option<oag_raceplay::Race> {
    let image = image()?;
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(oag_hd::race::DEFAULT_TRACK.to_string()),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading Talon's Junction");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    assert!(
        race.use_original_ai(Some(skill), level),
        "HD flies the original driver"
    );
    Some(race)
}

/// Every opponent's held throttle with its place, `from..=to` seconds after
/// the green light.
fn held(level: usize, skill: f32, from: f32, to: f32) -> Option<Vec<(u8, f32)>> {
    let mut race = race(level, skill)?;
    let time = |race: &oag_raceplay::Race| {
        race.sim
            .world
            .tick
            .saturating_sub(oag_race::COUNTDOWN_TICKS) as f32
            / 60.0
    };
    let mut out = Vec::new();
    while time(&race) <= to {
        race.tick(&PlayerInputs::none());
        if time(&race) >= from {
            let places = race.places();
            let field = race.original_ai_field().expect("the field is on");
            out.extend((1..8).map(|s| (places[s], field.drivers[s].held_thrust)));
        }
    }
    Some(out)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn easy() -> Option<Vec<(u8, f32)>> {
    held(0, -0.07, 8.0, 40.0)
}

/// The three readings `fury-park-1` logged, off **one** Easy run: they were
/// three tests flying the same deterministic race until 2026-10-12.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn at_easy_the_reference_racer_and_both_stops_read_what_hd_logged() {
    let Some(samples) = easy() else {
        return;
    };
    the_reference_racer_reads_what_hd_logged(&samples);
    the_upper_stop_reads_what_hd_logged(&samples);
    the_lower_stop_reads_what_hd_logged(&samples);
}

/// `fury-park-1` logged the racer one place ahead of the parked player at
/// `54.51` on every one of its turns: `AIThrust` at eighth place,
/// `WhenBehind` at its clamp, the skill curve extrapolated below Point1.
fn the_reference_racer_reads_what_hd_logged(samples: &[(u8, f32)]) {
    let reference: Vec<f32> = samples
        .iter()
        .filter(|(place, _)| *place == 7)
        .map(|&(_, t)| t)
        .collect();
    assert!(!reference.is_empty());
    assert!(
        reference.iter().all(|&t| close(t, 54.51)),
        "place 7 read {reference:?}"
    );
}

/// The same run's `+30 %` stop: `81.21`.
fn the_upper_stop_reads_what_hd_logged(samples: &[(u8, f32)]) {
    assert!(
        samples.iter().any(|&(_, t)| close(t, 81.21)),
        "nobody on the upper stop"
    );
}

/// The same run's `-30 %` stop: `27.81`.
fn the_lower_stop_reads_what_hd_logged(samples: &[(u8, f32)]) {
    assert!(
        samples.iter().any(|&(_, t)| close(t, 27.81)),
        "nobody on the lower stop"
    );
}

/// `fury-park-hard` (skill `3.48`, level 2): the upper stop at `104.58`, a
/// throttle above `100` the engine reads unclamped.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn at_hard_the_upper_stop_reads_what_hd_logged() {
    let Some(samples) = held(2, 3.48, 5.0, 20.0) else {
        return;
    };
    assert!(
        samples.iter().any(|&(_, t)| close(t, 104.58)),
        "nobody on the upper stop"
    );
}

/// HD's campaign cell reads its skill off the circuit's own `stats.xml`, as
/// the original's `AI_ResolveSkillScale` does: the Fury cell `grid8_3_1`
/// (Talon's Junction, VENOM, `skillEasy 0.3`, `skillHard 2.6`) on the track's
/// VENOM curve gives what the capture logged, `-0.07` and `3.48`.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_fury_cell_resolves_the_skill_hd_logged() {
    use oag_tables::race_campaign::Difficulty;
    let Some(image) = image() else {
        return;
    };
    let mut archives =
        oag_assets::Archives::open(&image.display().to_string(), oag_hd::TITLE).expect("HD opens");
    let grid = oag_tables::race_campaign::from_blob(
        &archives
            .read_name("/data/plugins/grids/grid_08.xml")
            .expect("grid 8 reads"),
    )
    .expect("grid 8 parses");
    let cell = grid
        .cells
        .iter()
        .find(|cell| cell.name == "grid8_3_1")
        .expect("the cell the capture raced");
    let stats = oag_tables::track_stats::from_blob(
        &archives
            .read_name(r"Data\Environments\Talons_Junction\stats.xml")
            .expect("its stats.xml reads"),
    )
    .expect("parses");
    let resolve = |rung| {
        oag_tables::track_stats::resolve_skill_scale(cell, rung, Some(&stats)).expect("a skill")
    };
    assert!(
        close(resolve(Difficulty::Easy), -0.07),
        "{}",
        resolve(Difficulty::Easy)
    );
    assert!(
        close(resolve(Difficulty::Hard), 3.48),
        "{}",
        resolve(Difficulty::Hard)
    );
}

/// The field gets round on the player's physics with HD's driver and the
/// rail: three laps in 150 s, nobody put back on the track. Shield is read at
/// each opponent's lap crossings and at the end.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_original_field_gets_round_talons_junction_on_hd() {
    let Some(mut race) = race(0, -0.07) else {
        return;
    };
    let mut lap = [1u32; 8];
    let mut per_lap: [Vec<f32>; 8] = Default::default();
    for _ in 0..60 * 150 {
        race.tick(&PlayerInputs::none());
        for slot in 1..8 {
            let ship = &race.sim.world.ships[slot];
            if ship.standing.lap != lap[slot] {
                lap[slot] = ship.standing.lap;
                per_lap[slot].push(ship.physics.shield);
            }
        }
    }
    for (slot, shields) in per_lap.iter().enumerate().skip(1) {
        let ship = &race.sim.world.ships[slot];
        println!(
            "slot {slot}: lap {} respawns {} wall-contact ticks {} per-lap shield {:?}, end-of-run shield {:.1}",
            ship.standing.lap,
            race.respawns_of(slot),
            race.wall_contact_ticks_of(slot),
            shields,
            ship.physics.shield
        );
        assert!(
            ship.standing.lap >= 3,
            "slot {slot} on lap {}",
            ship.standing.lap
        );
        assert_eq!(race.respawns_of(slot), 0, "slot {slot}");
    }
}

/// The halved lookahead's circuit rows resolve off the disc's own plugin
/// definition: `05_Track` is Ubermall forward, and only a Phantom race on it
/// halves (`Ai_Construct`'s two flags, `0x000fee30`).
fn ubermall(class: &str) -> Option<Option<(f32, f32)>> {
    let image = image()?;
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some("/data/environments/05_ubermall/track.vex".to_string()),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading Ubermall");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    assert!(race.use_original_ai(Some(2.0), 1));
    Some(race.original_ai_field()?.laws().halved_lookahead)
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_phantom_race_on_ubermall_halves_its_lookahead_near_the_line() {
    let Some(window) = ubermall("PHANTOM") else {
        return;
    };
    assert_eq!(window, Some((250.0, 4600.0)));
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_venom_race_on_ubermall_keeps_its_full_lookahead() {
    let Some(window) = ubermall("VENOM") else {
        return;
    };
    assert_eq!(window, None);
}
