//! Wipeout HD's opponents plan and drive on HD's own craft laws, not Pulse's.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test hd_ai_craft_laws_ground_truth --run-ignored all --no-capture
//! ```
//!
//! HD builds the craft's inertia with mass `1.0` (`I_yy` 24 against Pulse's 21.6), clamps its
//! steering ramp at the target and hovers on four probes
//! (`docs/ghidra/functions/ps3-hdfury-eu/craft-inertia.md`, `hover-four-point.md`). The AI
//! obeys the player's physics, so its yaw ceiling and the craft its speed plan simulates must be
//! that craft. The speed plan is built in `Race::start`, before the first tick, so the laws
//! have to be on the craft at seating, not only written by the tick.
//!
//! `lone_opponent_board` and the `field_on_*` tests print laps, wall contacts and shield, the before-and-after
//! record in `docs/gameplay/ai.md`, "Each title's craft laws".

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::Race;

const IMAGE: &str = "data/images/hdfury-ps3-eu-dec.iso";

fn started(track: &str) -> Option<Race> {
    started_seeded(track, None)
}

fn started_seeded(track: &str, seed: Option<u64>) -> Option<Race> {
    let image = oag_testdata::image(IMAGE)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        weapons_override: Some(false),
        track: Some(track.to_string()),
        seed,
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(Race::start(loaded.setup))
}

/// Every opponent carries HD's laws from seating, which is when its speed plan was built, and
/// the yaw ceiling the AI plans with is the one that inertia gives.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_opponents_are_seated_on_hds_craft_laws() {
    let Some(race) = started(oag_hd::race::DEFAULT_TRACK) else {
        return;
    };
    for slot in 0..race.sim.world.ship_count as usize {
        let physics = &race.sim.world.ships[slot].physics;
        assert_eq!(physics.body.inertia.y, 24.0, "slot {slot} I_yy");
        assert!(physics.steer_ramp_clamped, "slot {slot} steering ramp");
        let rig = physics.hover_rig;
        assert_eq!(rig.count, 4, "slot {slot} hover probes");
        assert_eq!(rig.spring_share, 0.15, "slot {slot} spring share");
        assert!(rig.along_normal, "slot {slot} spring direction");
    }
    let ship = &race.sim.world.ships[1];
    let ceiling = oag_ai::hull_yaw_ceiling(&ship.handling, &ship.physics.body);
    let pulse = oag_ai::hull_yaw_ceiling(&ship.handling, &oag_physics::Body::default());
    println!("slot 1 yaw ceiling {ceiling} rad/s (Pulse's inertia would read {pulse})");
    assert!(
        (ceiling / pulse - 0.9).abs() < 1e-5,
        "{ceiling} against {pulse}"
    );
}

/// One lap's record for one craft.
struct Lap {
    ticks: u64,
    contacts: u32,
    respawns: u32,
    shield: f32,
}

/// One craft's run: its completed laps, and every tick its shield fell by more than a
/// point while no wall charged it (tick, line index, loss).
#[derive(Default)]
struct Run {
    laps: Vec<Lap>,
    off_wall_hits: Vec<(u64, u32, f32)>,
}

/// Runs `race` for `ticks` and returns each listed slot's run.
fn runs_of(race: &mut Race, slots: &[usize], ticks: u64) -> Vec<Run> {
    let mut runs: Vec<Run> = slots.iter().map(|_| Run::default()).collect();
    let mut mark: Vec<(u32, u64, u32, u32)> = slots
        .iter()
        .map(|&slot| (race.sim.world.ships[slot].standing.lap, 0, 0, 0))
        .collect();
    for tick in 0..ticks {
        let before: Vec<(f32, f32)> = slots
            .iter()
            .map(|&slot| {
                (
                    race.sim.world.ships[slot].physics.shield,
                    race.wall_shield_charged_of(slot),
                )
            })
            .collect();
        race.tick(&PlayerInputs::none());
        for (i, &slot) in slots.iter().enumerate() {
            let ship = &race.sim.world.ships[slot];
            let lost = before[i].0 - ship.physics.shield;
            let charged = race.wall_shield_charged_of(slot) - before[i].1;
            if lost > 1.0 && charged <= 0.0 {
                runs[i].off_wall_hits.push((tick, ship.driver.index, lost));
            }
            let now = ship.standing.lap;
            let (lap, started, contacts, respawns) = mark[i];
            if now != lap {
                runs[i].laps.push(Lap {
                    ticks: tick - started,
                    contacts: race.wall_contact_ticks_of(slot) - contacts,
                    respawns: race.respawns_of(slot) - respawns,
                    shield: race.sim.world.ships[slot].physics.shield,
                });
                mark[i] = (
                    now,
                    tick,
                    race.wall_contact_ticks_of(slot),
                    race.respawns_of(slot),
                );
            }
        }
    }
    runs
}

fn print_run(race: &Race, slot: usize, run: &Run, ticks: u64) {
    for (n, lap) in run.laps.iter().enumerate() {
        println!(
            "    lap {}: {:.2}s  wall-contact ticks {:<4} respawns {}  shield at lap end {:.1}",
            n + 1,
            lap.ticks as f32 / 60.0,
            lap.contacts,
            lap.respawns,
            lap.shield
        );
    }
    println!(
        "    end of run ({ticks} ticks): wall-contact ticks {}  respawns {}  shield {:.1}  \
         wall-charged {:.1}  shield spent off walls (barrel rolls: tick, line index, cost) {:?}",
        race.wall_contact_ticks_of(slot),
        race.respawns_of(slot),
        race.sim.world.ships[slot].physics.shield,
        race.wall_shield_charged_of(slot),
        run.off_wall_hits
    );
}

/// The twelve racing circuits (the four Zone environments left out), one opponent alone.
const CIRCUITS: [&str; 12] = [
    "talons_junction",
    "01_vineta_k",
    "05_ubermall",
    "amphiseum",
    "modesto_heights",
    "tech_de_ra",
    "02_track",
    "03_track",
    "04_chenghou_project",
    "10_sebenco_climb",
    "12_sol_2",
    "15_anulpha_pass",
];

/// A lone Ace opponent on every HD circuit: laps, wall contacts, respawns and shield per lap,
/// then the end of the run. Prints; asserts only that each circuit saw a lap.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn lone_opponent_board() {
    const TICKS: u64 = 12_000;
    for environment in CIRCUITS {
        let Some(mut race) = started(&oag_hd::names::track(environment)) else {
            return;
        };
        for slot in (0..8).filter(|&slot| slot != 1) {
            race.sim.world.ships[slot].active = false;
        }
        let plan = if race.speed_plan().is_some() {
            "followed"
        } else {
            "corner model"
        };
        let run = runs_of(&mut race, &[1], TICKS).remove(0);
        println!("{environment} (speed plan {plan})");
        print_run(&race, 1, &run, TICKS);
        assert!(
            run.laps.len() >= 2,
            "{environment}: the lone opponent finished no lap"
        );
    }
}

/// The full AI field, weapons off, at five seeds: every opponent's laps and its end of run,
/// then one summary line per seed.
fn field(environment: &str) {
    const TICKS: u64 = 12_000;
    for seed in 1..=5u64 {
        let Some(mut race) = started_seeded(&oag_hd::names::track(environment), Some(seed)) else {
            return;
        };
        race.sim.world.ships[0].active = false;
        let slots: Vec<usize> = (1..race.sim.world.ship_count as usize).collect();
        let runs = runs_of(&mut race, &slots, TICKS);
        let (mut contacts, mut charged, mut lost, mut laps, mut lap_ticks) = (0, 0.0, 0.0, 0, 0);
        for (run, &slot) in runs.iter().zip(&slots) {
            println!("{environment} seed {seed} slot {slot}");
            print_run(&race, slot, run, TICKS);
            contacts += race.wall_contact_ticks_of(slot);
            charged += race.wall_shield_charged_of(slot);
            lost += 95.0 - race.sim.world.ships[slot].physics.shield;
            // Flying laps only: the first is the standing start.
            for lap in run.laps.iter().skip(1) {
                laps += 1;
                lap_ticks += lap.ticks;
            }
        }
        println!(
            "{environment} seed {seed}: field wall-contact ticks {contacts}  wall-charged {charged:.1}  \
             shield lost {lost:.1}  mean flying lap {:.2}s over {laps}",
            lap_ticks as f32 / 60.0 / laps.max(1) as f32
        );
    }
}
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn field_on_talons_junction() {
    field("talons_junction");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn field_on_vineta_k() {
    field("01_vineta_k");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn field_on_ubermall() {
    field("05_ubermall");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn field_on_sebenco_climb() {
    field("10_sebenco_climb");
}
