//! An Eliminator with the player parked, on a real circuit out of a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! The question it answers is the one a player asks: *does the mode end?* With
//! the player parked, the eight-craft field has to fight to a finish by
//! itself, which needs three things wired at once - weapons that reach the
//! field, kills credited when a weapon finishes a craft, and a kill target the
//! event ends on.

use oag_game::race;
use oag_gameplay::PlayerInputs;

/// Six game-minutes at 60 Hz: past that a run is a wait, not a measurement.
const TICKS: u64 = 60 * 60 * 6;

fn loaded_eliminator_seeded(kill_target: Option<u32>, seed: Option<u64>) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Eliminator,
        eliminator_kill_target: kill_target,
        seed,
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup))
}

/// The field, left alone, credits kills to its craft and reaches the original's
/// own target of five.
///
/// **One test per seed, and every seed must finish.** This used to be a target
/// of 2 over "the first of five seeds that ends", which proved the wiring and
/// nothing about the mode. The seeds are fixed (the four the handover thread
/// measured with, and nothing was chosen for passing); each ends or fails on
/// its own, so a field that does not reach five names its seed, and the
/// matrix is the test axis rather than a loop in one process.
///
/// Prints the finish in seconds beside the original's 85 s on `16_Track` with
/// the player parked. This build is slower - about 100 to 345 s over 24 seeds - and
/// the gap is the AI not cheating: see `docs/gameplay/race-modes.md`.
fn a_parked_player_eliminator_finishes_at_five(seed: u64) {
    let parked = PlayerInputs::single(oag_gameplay::InputSnapshot::new());
    let Some(mut race) = loaded_eliminator_seeded(Some(5), Some(seed)) else {
        return;
    };
    let mut ended = None;
    for tick in 0..TICKS {
        race.tick(&parked);
        if race.sim.world.primary_race().finished {
            ended = Some(tick);
            break;
        }
    }
    let kills: Vec<u32> = (0..race.sim.world.ship_count as usize)
        .map(|slot| race.sim.world.ships[slot].standing.kills)
        .collect();
    println!(
        "seed {seed}: ended at {:?} s (original: 85 s), kills {kills:?}",
        ended.map(|tick| tick as f32 / 60.0)
    );
    assert!(
        ended.is_some(),
        "seed {seed} did not reach 5 kills in six game-minutes: {kills:?}"
    );
    assert!(
        kills.iter().any(|&k| k >= 5),
        "the race ended without anyone on the target: {kills:?}"
    );
    assert_eq!(kills[0], 0, "the parked player cannot have scored");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_parked_player_eliminator_finishes_at_five_seed_5() {
    a_parked_player_eliminator_finishes_at_five(5);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_parked_player_eliminator_finishes_at_five_seed_9() {
    a_parked_player_eliminator_finishes_at_five(9);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_parked_player_eliminator_finishes_at_five_seed_13() {
    a_parked_player_eliminator_finishes_at_five(13);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_parked_player_eliminator_finishes_at_five_seed_16() {
    a_parked_player_eliminator_finishes_at_five(16);
}

/// Seeds tried, in this order, by the target-3 test below: the default first, then a
/// fixed run of small integers. Fixed before any result was looked at, so no
/// seed is chosen because it passes.
const SEARCH_SEEDS: [Option<u64>; 5] = [None, Some(1), Some(2), Some(3), Some(4)];

/// The target a launch carries is the one the race ends on, not a constant:
/// 3 here, where the test above ends on 2, so a KILLS pick dropped on the way
/// into `Options` (leaving the default 5) ends on neither and fails one of the
/// two bounds below.
///
/// **Whether a given field reaches three kills is a weapons lottery** over
/// seven craft (`docs/gameplay/race-modes.md`, and the Eliminator handover
/// thread's open five-kill finish), so this walks [`SEARCH_SEEDS`] until one
/// field ends and checks that race: every race that ends must end on the chosen
/// target. At least one of the five must end, or the test fails.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_chosen_kill_target_is_what_the_eliminator_ends_on() {
    let parked = PlayerInputs::single(oag_gameplay::InputSnapshot::new());
    let mut unfinished = Vec::new();
    for seed in SEARCH_SEEDS {
        let Some(mut race) = loaded_eliminator_seeded(Some(3), seed) else {
            return;
        };
        let mut best_when_ended = None;
        for _ in 0..TICKS * 3 {
            race.tick(&parked);
            if race.sim.world.primary_race().finished {
                best_when_ended = (0..race.sim.world.ship_count as usize)
                    .map(|slot| race.sim.world.ships[slot].standing.kills)
                    .max();
                break;
            }
        }
        match best_when_ended {
            Some(best) => {
                assert!(
                    (3..5).contains(&best),
                    "seed {seed:?} ended with a best of {best} kills"
                );
                return;
            }
            None => unfinished.push(seed),
        }
    }
    panic!("no field of {unfinished:?} reached a finish within eighteen game-minutes");
}

/// A craft that is destroyed and put back is still where it was in the race.
///
/// The respawn used to read the driver's stale index, which stops at the
/// death while the wreck coasts on. A wreck that crossed the start line and
/// was then put back behind it read as a backward wrap, so the craft lost a lap
/// for the rest of the race (seed 5, slot 2, tick 3025: 5,263 units down) and
/// vanished from every other craft's field. Each respawn's distance is compared
/// with the distance at the moment of death, and fails if the craft moved by
/// anything like a lap.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_respawned_craft_keeps_its_place_in_the_race() {
    let Some(mut race) = loaded_eliminator_seeded(Some(5), Some(5)) else {
        return;
    };
    let parked = PlayerInputs::single(oag_gameplay::InputSnapshot::new());
    let slots = race.sim.world.ship_count as usize;
    let mut down = [None::<f32>; 8];
    let mut respawns = 0;
    for _ in 0..4000 {
        race.tick(&parked);
        let course = race.course().expect("a closed circuit").clone();
        for (slot, down) in down.iter_mut().enumerate().take(slots).skip(1) {
            let ship = &race.sim.world.ships[slot];
            let distance = ship.standing.distance(&course);
            match (ship.physics.craft_state, *down) {
                (oag_physics::CraftState::Eliminated, None) => *down = Some(distance),
                (oag_physics::CraftState::Racing, Some(was)) => {
                    respawns += 1;
                    assert!(
                        (distance - was).abs() < 500.0,
                        "slot {slot} was at {was} when destroyed and {distance} when put back"
                    );
                    *down = None;
                }
                _ => {}
            }
        }
    }
    assert!(
        respawns >= 3,
        "only {respawns} respawns: nothing was checked"
    );
}
