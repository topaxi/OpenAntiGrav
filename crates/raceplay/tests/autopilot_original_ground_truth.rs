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
    let mut setup = setup(image, track, weapons)?;
    crate::perturb::apply(&mut setup, 0.0);
    Some(oag_raceplay::Race::start(setup))
}

fn setup(image: &str, track: &str, weapons: Option<bool>) -> Option<oag_raceplay::Setup> {
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
    Some(loaded.setup)
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
///
/// **One test per circuit, judged on the majority of five races**
/// (2026-10-20, lane `ai-gate-robustness`), one per scale of `perturb::GATE`
/// (`tests/perturb/mod.rs`). As one twelve-circuit run it failed under 4 of 8
/// sweep offsets with every law on, and under 2 of 9 force-term perturbations
/// with every law off (`docs/physics/airborne-ground-truth.md`).
fn finishes_without_a_rescue(id: &str) {
    let track = format!(r"Data\Environments\{id}_Track\track.vex");
    let Some(setup) = setup("pulse-psp-usa.chd", &track, Some(false)) else {
        return;
    };
    let runs: Vec<(Option<u64>, u32)> = crate::perturb::GATE
        .iter()
        .map(|&eps| {
            let mut setup = setup.clone();
            crate::perturb::apply(&mut setup, eps);
            let r = run_from(oag_raceplay::Race::start(setup), 15_000);
            (r.finish, r.respawns)
        })
        .collect();
    let good = runs
        .iter()
        .filter(|(finish, respawns)| finish.is_some() && *respawns == 0)
        .count();
    assert!(
        good * 2 > runs.len(),
        "{id}: {good} of {} runs finished without a rescue: {runs:?}",
        runs.len()
    );
}

macro_rules! finishes_test {
    ($name:ident, $id:expr) => {
        #[test]
        #[ignore = "needs a disc image in data/images/"]
        fn $name() {
            finishes_without_a_rescue($id);
        }
    };
}

finishes_test!(
    the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_01,
    "01"
);
finishes_test!(
    the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_02,
    "02"
);
finishes_test!(
    the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_03,
    "03"
);
finishes_test!(
    the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_04,
    "04"
);
finishes_test!(
    the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_05,
    "05"
);
finishes_test!(
    the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_06,
    "06"
);
finishes_test!(
    the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_07,
    "07"
);
finishes_test!(
    the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_09,
    "09"
);
finishes_test!(
    the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_10,
    "10"
);
finishes_test!(
    the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_13,
    "13"
);
finishes_test!(
    the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_14,
    "14"
);
finishes_test!(
    the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue_16,
    "16"
);

/// Every Pulse circuit, forward.
const CIRCUITS: [&str; 12] = [
    "01", "02", "03", "04", "05", "06", "07", "09", "10", "13", "14", "16",
];

fn run(image: &str, track: &str, ticks: u64, weapons: Option<bool>) -> Option<Run> {
    Some(run_from(race(image, track, weapons)?, ticks))
}

fn run_from(mut race: oag_raceplay::Race, ticks: u64) -> Run {
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
    out
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
        team: std::env::var("OAG_SWEEP_TEAM").ok(),
        weapons_override: std::env::var_os("OAG_SWEEP_WEAPONS")
            .is_none()
            .then_some(false),
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
            out.speed.push(
                race.sim.world.ships[0]
                    .physics
                    .body
                    .linear_velocity
                    .length(),
            );
        }
        if std::env::var_os("OAG_SWEEP_TRACE").is_some() {
            let ship = &race.sim.world.ships[0];
            let rail = race.original_autopilot(0).map(|a| a.driver.rail);
            if std::env::var_os("OAG_SWEEP_TRACE_ALL").is_some()
                || race.sim.world.tick.is_multiple_of(2)
                || race.respawns_of(0) != out.respawns
            {
                println!(
                    "trace {} pos {:?} speed {:.1} grounded {:.2} lateral {:?} blend {:?} respawns {} cause {:?} craft [{:.1}, {:.1}, {:.1}, {:.1}, {:.1}] fwd {:?} up {:?} vel {:?}",
                    race.sim.world.tick,
                    ship.physics.body.position,
                    ship.physics.body.linear_velocity.length(),
                    ship.physics.grounded,
                    rail.map(|r| r.lateral),
                    rail.map(|r| r.blend),
                    race.respawns_of(0),
                    race.last_respawn_cause_of(0),
                    ship.physics.thrust,
                    ship.physics.brake,
                    ship.physics.steer,
                    ship.physics.airbrake_left,
                    ship.physics.airbrake_right,
                    ship.physics.body.forward(),
                    ship.physics.body.up(),
                    ship.physics.body.linear_velocity
                );
            }
            out.respawns = race.respawns_of(0);
        }
        if race.sim.world.ships[0].standing.lap != lap {
            lap = race.sim.world.ships[0].standing.lap;
            out.shield_per_lap
                .push(race.sim.world.ships[0].physics.shield);
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
/// in `OAG_SWEEP_CLASSES` (default `VENOM,PHANTOM`). `OAG_SWEEP_TEAM` flies
/// another team; `OAG_SWEEP_TRACE` prints the craft every other tick (pose,
/// velocity, control states), `OAG_SWEEP_TRACE_ALL` every tick.
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

/// Slot 0's warm driver against a cold locate of the same craft
/// (`Race::set_autopilot` re-locates it, as a pickup does), every `every`
/// ticks over `ticks` of an autopiloted Pulse VENOM race, seed 1, weapons on.
/// Returns (located, of them on a route, wrong: the cold point more than 10
/// units further from the craft than the warm one), and the first wrong tick.
fn cold_locates(track: &str, ticks: u64, every: u64) -> Option<(u32, u32, u32, Option<u64>)> {
    let mut race = race("pulse-psp-usa.chd", track, None)?;
    race.set_autopilot(true);
    let (mut n, mut on_route, mut wrong, mut first) = (0, 0, 0, None);
    for tick in 1..=ticks {
        race.tick_sim_only(&PlayerInputs::none());
        let ship = &race.sim.world.ships[0];
        if !tick.is_multiple_of(every)
            || ship.physics.craft_state != oag_physics::CraftState::Racing
            || ship.standing.finished()
        {
            continue;
        }
        let warm = ship.driver;
        let warm_point = race.line_of(0).point(warm.index as usize);
        let snapshot = race.snapshot_sim();
        race.set_autopilot(true);
        let cold = race.sim.world.ships[0].driver;
        let cold_point = race.line_of(0).point(cold.index as usize);
        race.restore_sim(&snapshot);
        n += 1;
        on_route += u32::from(warm.branching.route != 0);
        let craft = race.sim.world.ships[0].physics.body.position;
        if cold_point.distance(craft) > warm_point.distance(craft) + 10.0 {
            wrong += 1;
            {
                let ship = &race.sim.world.ships[0];
                println!(
                    "  tick {tick} craft {:?} air {:.2} grounded {} warm {} {:?} at {warm_point:?}, cold {} {:?} at {cold_point:?}",
                    ship.physics.body.position,
                    ship.physics.time_airborne,
                    ship.physics.grounded,
                    warm.index,
                    warm.branching,
                    cold.index,
                    cold.branching
                );
            }
            first.get_or_insert(tick);
        }
    }
    Some((n, on_route, wrong, first))
}

/// A cold locate of the player's craft on a route's own samples finds that
/// route, not the ring's nearest point on another deck: Pulse `05_Track`,
/// where the ring passes 55 units above route 1 (`docs/gameplay/ai.md`, "An
/// opponent's Autopilot"). Fails if `Race::locate_player_driver` searches the
/// ring alone.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_cold_locate_on_a_route_stays_on_the_route() {
    let Some((n, on_route, wrong, first)) =
        cold_locates(r"Data\Environments\05_Track\track.vex", 4_000, 30)
    else {
        return;
    };
    println!("located {n}, on a route {on_route}, wrong {wrong} (first {first:?})");
    assert!(
        on_route > 10,
        "slot 0 never took the route: {on_route} of {n}"
    );
    assert_eq!(wrong, 0, "first wrong at tick {first:?}");
}

/// [`cold_locates`] on every Pulse circuit for a whole race.
#[test]
#[ignore = "a scratch sweep: set OAG_SWEEP=1"]
fn cold_locate_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    for id in [
        "01", "02", "03", "04", "05", "06", "07", "09", "10", "13", "14", "16",
    ] {
        let track = format!(r"Data\Environments\{id}_Track\track.vex");
        let Some((n, on_route, wrong, first)) = cold_locates(&track, 12_000, 30) else {
            return;
        };
        println!("LOCATE {id} located {n} on a route {on_route} wrong {wrong} first {first:?}");
    }
}

/// The Autopilot's opponent switch is the title's per-class rule
/// (`oag_title::opponent_autopilot::BELOW_RAPIER`): an opponent at VENOM or
/// FLASH flies the pickup, one at RAPIER or PHANTOM absorbs it. Pulse and HD,
/// weapons on. Fails if a race starts with the switch the other way.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_opponent_flies_its_autopilot_below_rapier_only() {
    for (image, track) in [
        ("pulse-psp-usa.chd", r"Data\Environments\01_Track\track.vex"),
        (
            "hdfury-ps3-eu-dec.iso",
            "/data/environments/talons_junction/track.vex",
        ),
    ] {
        for (class, flies) in [
            ("VENOM", true),
            ("FLASH", true),
            ("RAPIER", false),
            ("PHANTOM", false),
        ] {
            let Some(path) = oag_testdata::image(image) else {
                return;
            };
            let loaded = oag_raceplay::load(&oag_raceplay::Options {
                source: path.display().to_string(),
                class: class.to_string(),
                mode: oag_race::Mode::SingleRace,
                track: Some(track.to_string()),
                seed: Some(1),
                ..oag_raceplay::Options::default()
            })
            .expect("loading the track");
            let mut race = oag_raceplay::Race::start(loaded.setup);
            assert_eq!(race.engage_opponent_autopilot(1), flies, "{image} {class}");
        }
    }
}
