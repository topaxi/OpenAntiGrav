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
/// "The autopilot flies the original's player driver").
fn flies_the_original_autopilot(image: &str) {
    let Some(path) = oag_testdata::exact(image) else {
        return;
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
}

#[test]
#[ignore = "needs a Pure image"]
fn pure_flies_the_original_autopilot() {
    flies_the_original_autopilot("data/images/pure-psp-usa.chd");
}

#[test]
#[ignore = "needs an HD image"]
fn hd_flies_the_original_autopilot() {
    flies_the_original_autopilot("data/images/hdfury-ps3-eu-dec.iso");
}

#[test]
#[ignore = "needs an Omega image"]
fn omega_flies_the_original_autopilot() {
    flies_the_original_autopilot("data/images/omega-ps4-eu.pkg");
}

#[test]
#[ignore = "needs a 2048 package"]
fn vita_2048_flies_the_original_autopilot() {
    flies_the_original_autopilot("data/test-vpk/2048-PCSF00007-dlc1.vpk");
}

/// Distance covered and rescues taken over one fork's window.
type Leg = (f32, u32);

/// An opponent on this project's driver handed an Autopilot, against the
/// same tick with the absorb: Pulse PSP USA, VENOM, one fork at `trigger` on
/// `slot`, weapons off unless `weapons`. Returns (distance, rescues) for the
/// absorb and for the original's player driver over the 5 s stint and 3 s
/// after it, and whether the original driver flew it at full throttle.
fn opponent_fork(
    track: &str,
    difficulty: oag_ai::Difficulty,
    seed: u64,
    trigger: u64,
    slot: usize,
) -> Option<(Leg, Leg, bool)> {
    let image = oag_testdata::image("pulse-psp-usa.chd")?;
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(format!(r"Data\Environments\{track}_Track\track.vex")),
        seed: Some(seed),
        difficulty,
        weapons_override: Some(false),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the track");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.set_autopilot(true);
    race.set_opponent_autopilot(false);
    for _ in 0..trigger {
        race.tick_sim_only(&PlayerInputs::none());
    }
    let snapshot = race.snapshot_sim();
    let branch = |race: &mut oag_raceplay::Race, on: bool| {
        race.restore_sim(&snapshot);
        race.set_opponent_autopilot(on);
        if on {
            // Weapons off, so no table: the Pulse table's `time`, 5 s.
            race.sim.world.ships[slot].autopilot_timer = 5.0;
        }
        let d0 = race.race_distance_of(slot).unwrap_or(0.0);
        let mut flown = false;
        for _ in 0..480 {
            race.tick_sim_only(&PlayerInputs::none());
            flown |= race
                .original_autopilot(slot)
                .is_some_and(|a| a.driver.held_thrust == oag_ai::original::AUTOPILOT_THRUST)
                && race.sim.world.ships[slot].physics.thrust == 100.0;
        }
        (
            (
                race.race_distance_of(slot).unwrap_or(0.0) - d0,
                race.respawns_of(slot),
            ),
            flown,
        )
    };
    let (absorbed, _) = branch(&mut race, false);
    let (autopilot, flown) = branch(&mut race, true);
    Some((absorbed, autopilot, flown))
}

/// The Autopilot speeds an opponent up under `Race::set_opponent_autopilot`
/// (`docs/gameplay/ai.md`, "An opponent's Autopilot, measured and not
/// wired"): one Pulse 01 fork at Novice, where the paired sweep's median gain
/// is 72 units over the window. Fails if the switch stops flying the
/// original's player driver.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_opponent_flies_its_autopilot_on_the_original_driver_and_gains() {
    let Some((absorbed, autopilot, flown)) =
        opponent_fork("01", oag_ai::Difficulty::Novice, 1, 1_200, 1)
    else {
        return;
    };
    println!("absorbed {absorbed:?} autopilot {autopilot:?}");
    assert!(flown, "the original's player driver never flew slot 1");
    assert!(
        autopilot.0 > absorbed.0 + 20.0,
        "autopilot {autopilot:?} against absorb {absorbed:?}"
    );
    assert_eq!(autopilot.1, 0);
}

/// Two Pulse 05 forks at Ace (weapons off) that the opponent's Autopilot
/// once lost: engaged at `[-860, 41, -445]` a whole-line search put the
/// driver on another deck (rescued 89 ticks in); engaged at
/// `[-125, -10, -477]` the original driver took the jump short of its
/// landing (rescued 407 ticks in). Fails if the stint re-searches the line or
/// flies the run-up.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_opponents_autopilot_keeps_its_place_and_leaves_the_jump_to_its_driver() {
    for (seed, trigger, slot) in [(2, 3_000, 4), (6, 4_800, 7)] {
        let Some((absorbed, autopilot, flown)) =
            opponent_fork("05", oag_ai::Difficulty::Ace, seed, trigger, slot)
        else {
            return;
        };
        assert!(flown, "seed {seed}: not flown");
        assert_eq!(
            autopilot.1, absorbed.1,
            "seed {seed}: autopilot {autopilot:?} against absorb {absorbed:?}"
        );
    }
}

/// The original's own field keeps absorbing it even with the switch on:
/// `WeaponAi_Update` never fires the Autopilot outside an Eliminator.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_original_field_opponent_is_not_handed_an_autopilot() {
    let Some(mut race) = race(
        "pulse-psp-usa.chd",
        r"Data\Environments\01_Track\track.vex",
        None,
    ) else {
        return;
    };
    race.set_opponent_autopilot(true);
    assert!(race.engage_opponent_autopilot(1), "ours flies it");
    race.sim.world.ships[1].autopilot_timer = 0.0;
    assert!(race.use_original_ai(None, 1));
    assert!(
        !race.engage_opponent_autopilot(1),
        "the original's absorbs it"
    );
}

/// What the player's own craft did against the walls over one autopiloted
/// race, weapons off.
#[derive(Debug, Default)]
struct WallRun {
    ticks: u64,
    contact: u32,
    inbound: u32,
    respawns: u32,
    finish: Option<u64>,
    /// `|lateral|` samples on ticks the craft raced, off the rail's own lane
    /// point.
    lateral: Vec<f32>,
    /// Contact episodes: runs of consecutive contact ticks.
    episodes: u32,
    /// Speed and the rail's blend on the same ticks as `lateral`.
    speed: Vec<f32>,
    blend: Vec<f32>,
    /// Slot 0's shield at the end of each lap, and at the end of the run.
    shield_per_lap: Vec<f32>,
    shield_end: f32,
}

fn wall_run(image: &str, class: &str, track: &str, ticks: u64) -> Option<WallRun> {
    let image = oag_testdata::image(image)?;
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(track.to_string()),
        seed: Some(1),
        weapons_override: std::env::var_os("OAG_SWEEP_WEAPONS").is_none().then_some(false),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the track");
    let mut race = oag_raceplay::Race::start(loaded.setup);
    assert!(race.use_original_ai(None, 1), "the AI tables read");
    race.set_autopilot(true);
    let mut out = WallRun::default();
    let mut last = 0;
    let mut touching = false;
    let mut lap = race.sim.world.ships[0].standing.lap;
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
        out.ticks += 1;
        let now = race.wall_contact_ticks_of(0);
        let touch = now != last;
        if touch && !touching {
            out.episodes += 1;
        }
        touching = touch;
        last = now;
        if let Some(autopilot) = race.original_autopilot(0)
            && race.sim.world.tick > 300
        {
            out.lateral.push(autopilot.driver.rail.lateral.abs());
            out.blend.push(autopilot.driver.rail.blend);
            out.speed
                .push(race.sim.world.ships[0].physics.body.linear_velocity.length());
        }
        if race.sim.world.ships[0].standing.lap != lap {
            lap = race.sim.world.ships[0].standing.lap;
            out.shield_per_lap.push(race.sim.world.ships[0].physics.shield);
        }
        if race.sim.world.ships[0].standing.finished() {
            out.finish = Some(race.sim.world.tick);
            break;
        }
    }
    out.contact = race.wall_contact_ticks_of(0);
    out.inbound = race.wall_inbound_ticks_of(0);
    out.respawns = race.respawns_of(0);
    out.shield_end = race.sim.world.ships[0].physics.shield;
    Some(out)
}

/// The mean of `values`.
fn mean(values: &[f32]) -> f32 {
    values.iter().sum::<f32>() / values.len().max(1) as f32
}

/// A scratch sweep: the player's own wall contact and lateral offset under a
/// whole-race autopilot on every Pulse circuit, weapons off, for the classes
/// in `OAG_SWEEP_CLASSES` (default `VENOM,PHANTOM`).
///
/// ```sh
/// OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay \
///     --run-ignored all autopilot_original_ground_truth::wall_sweep --no-capture
/// ```
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn wall_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    let image = std::env::var("OAG_SWEEP_IMAGE").unwrap_or_else(|_| "pulse-psp-usa.chd".into());
    let classes = std::env::var("OAG_SWEEP_CLASSES").unwrap_or_else(|_| "VENOM,PHANTOM".into());
    let tracks: Vec<String> = std::env::var("OAG_SWEEP_TRACKS").map_or_else(
        |_| {
            CIRCUITS
                .iter()
                .map(|id| format!(r"Data\Environments\{id}_Track\track.vex"))
                .collect()
        },
        |list| list.split(',').map(str::to_string).collect(),
    );
    for class in classes.split(',') {
        for track in &tracks {
            let Some(mut r) = wall_run(&image, class, track, 15_000) else {
                return;
            };
            r.lateral.sort_by(f32::total_cmp);
            let pct = |p: f32| {
                r.lateral
                    .get(((r.lateral.len() as f32 - 1.0) * p) as usize)
                    .copied()
                    .unwrap_or(f32::NAN)
            };
            println!(
                "{class} {track} ticks {} finish {:?} respawns {} contact {} inbound {} episodes {} lateral p50 {:.2} p95 {:.2} p99 {:.2} max {:.2} speed mean {:.1} blend mean {:.2} shield per lap {:?} shield end {:.1}",
                r.ticks,
                r.finish,
                r.respawns,
                r.contact,
                r.inbound,
                r.episodes,
                pct(0.5),
                pct(0.95),
                pct(0.99),
                pct(1.0),
                mean(&r.speed),
                mean(&r.blend),
                r.shield_per_lap,
                r.shield_end
            );
        }
    }
}
