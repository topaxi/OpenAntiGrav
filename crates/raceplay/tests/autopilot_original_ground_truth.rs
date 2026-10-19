//! The player's autopilot flies the original's own player driver
//! (`oag_ai::original::Autopilot`): Pulse PSP USA, VENOM, seed 1, slot 0 on
//! `Race::set_autopilot` for the whole race.

use oag_gameplay::PlayerInputs;

/// What one autopiloted race came to.
#[derive(Debug, Default)]
struct Run {
    /// Laps slot 0 completed.
    laps: u32,
    /// Its place when it finished, or at the tick cap.
    place: u8,
    /// Ticks to the finish, if it finished.
    finish: Option<u64>,
    /// Rescues of slot 0.
    respawns: u32,
    /// Slot 0's shield at the end of each lap.
    shield_per_lap: Vec<f32>,
    /// Slot 0's shield when the run stopped.
    shield_end: f32,
}

fn race(image: &str, track: &str, weapons: Option<bool>) -> Option<oag_raceplay::Race> {
    let image = oag_testdata::image(image)?;
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(track.to_string()),
        seed: Some(1),
        weapons_override: weapons,
        ..oag_raceplay::Options::default()
    })
    .expect("loading the track");
    Some(oag_raceplay::Race::start(loaded.setup))
}

/// The opponents on this project's driver (no `use_original_ai`), and the
/// autopilot still flies the original's: full throttle and the racing line.
/// Fails if the autopilot goes back to `oag_ai::Driver` or drops either of the
/// player's branches.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_autopilot_flies_the_original_player_driver_whatever_the_opponents_fly() {
    let Some(mut race) = race(
        "pulse-psp-usa.chd",
        r"Data\Environments\16_Track\track.vex",
        None,
    ) else {
        return;
    };
    assert!(!race.original_ai_active());
    race.set_autopilot(true);
    for _ in 0..900 {
        race.tick(&PlayerInputs::none());
    }
    let autopilot = race
        .original_autopilot(0)
        .expect("the original driver flies the autopilot");
    assert_eq!(
        autopilot.driver.held_thrust,
        oag_ai::original::AUTOPILOT_THRUST
    );
    assert_eq!(autopilot.driver.planner.target, 0.0);
    assert_eq!(race.sim.world.ships[0].physics.thrust, 100.0);
}

/// The autopilot gets round every Pulse circuit against the original field
/// with no rescue (weapons off, so a hit does not decide it): the
/// round-the-lap check the original field's thread asked for.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue() {
    let mut failed = Vec::new();
    for id in CIRCUITS {
        let track = format!(r"Data\Environments\{id}_Track\track.vex");
        let Some(r) = run("pulse-psp-usa.chd", &track, 15_000, Some(false)) else {
            return;
        };
        if r.finish.is_none() || r.respawns != 0 {
            failed.push((id, r.finish, r.respawns));
        }
    }
    assert!(failed.is_empty(), "{failed:?}");
}

/// Every Pulse circuit, forward.
const CIRCUITS: [&str; 12] = [
    "01", "02", "03", "04", "05", "06", "07", "09", "10", "13", "14", "16",
];

fn run(image: &str, track: &str, ticks: u64, weapons: Option<bool>) -> Option<Run> {
    let mut race = race(image, track, weapons)?;
    assert!(race.use_original_ai(None, 1), "the AI tables read");
    race.set_autopilot(true);
    let mut out = Run::default();
    let mut lap = race.sim.world.ships[0].standing.lap;
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
        let ship = &race.sim.world.ships[0];
        if ship.standing.lap != lap {
            lap = ship.standing.lap;
            out.shield_per_lap.push(ship.physics.shield);
        }
        if ship.standing.finished() {
            out.finish = Some(race.sim.world.tick);
            break;
        }
    }
    let ship = &race.sim.world.ships[0];
    out.laps = ship.standing.lap.saturating_sub(1);
    out.place = race.places()[0];
    out.respawns = race.respawns_of(0);
    out.shield_end = ship.physics.shield;
    Some(out)
}

/// A scratch sweep: prints each Pulse circuit's autopiloted race, for the
/// before and after of a change to the player's driver; `OAG_SWEEP_NO_WEAPONS`
/// turns weapons off.
///
/// ```sh
/// OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay \
///     --run-ignored all autopilot_original_ground_truth::sweep --no-capture
/// ```
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let weapons = std::env::var_os("OAG_SWEEP_NO_WEAPONS").map(|_| false);
    for id in CIRCUITS {
        let track = format!(r"Data\Environments\{id}_Track\track.vex");
        let Some(r) = run("pulse-psp-usa.chd", &track, 15_000, weapons) else {
            return;
        };
        println!(
            "{id}_Track place {} laps {} finish {:?} respawns {} shield per lap {:?} shield end {:.1}",
            r.place, r.laps, r.finish, r.respawns, r.shield_per_lap, r.shield_end
        );
    }
}

/// Pure, HD, Omega and 2048 fly the same player driver on their own tables:
/// each title's default circuit, 1,800 ticks of autopilot. Omega's and
/// 2048's autopilot steering is the opponents' controller, not the pilot
/// assist those two executables give the player's driver (`docs/gameplay/ai.md`,
/// "The autopilot").
#[test]
#[ignore = "needs the other titles' images"]
fn every_title_with_original_tables_flies_the_original_autopilot() {
    let mut flown = Vec::new();
    for image in [
        "data/images/pure-psp-usa.chd",
        "data/images/hdfury-ps3-eu-dec.iso",
        "data/images/omega-ps4-eu.pkg",
        "data/test-vpk/2048-PCSF00007-dlc1.vpk",
    ] {
        let Some(path) = oag_testdata::exact(image) else {
            continue;
        };
        let loaded = oag_raceplay::load(&oag_raceplay::Options {
            source: path.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            seed: Some(1),
            ..oag_raceplay::Options::default()
        })
        .unwrap_or_else(|e| panic!("{image}: {e}"));
        let mut race = oag_raceplay::Race::start(loaded.setup);
        race.set_autopilot(true);
        for _ in 0..1_800 {
            race.tick(&PlayerInputs::none());
        }
        let autopilot = race
            .original_autopilot(0)
            .unwrap_or_else(|| panic!("{image}: the original driver flies the autopilot"));
        assert_eq!(
            autopilot.driver.held_thrust,
            oag_ai::original::AUTOPILOT_THRUST,
            "{image}"
        );
        assert_eq!(race.respawns_of(0), 0, "{image}");
        flown.push(image);
    }
    println!("flown: {flown:?}");
}
