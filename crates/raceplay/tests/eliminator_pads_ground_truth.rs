//! In an Eliminator an opponent with an empty weapon slot steers for weapon
//! pads, on a real circuit out of a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! The steering is ours, chosen, not measured (`oag_raceplay::PadSeeking`);
//! what this guards is that it is wired and does what it is for. The 240-seed
//! sweep that decided it ships is in `docs/gameplay/race-modes.md`, "Steering
//! for weapon pads".

use oag_gameplay::PlayerInputs;
use oag_physics::CraftState;
use oag_raceplay as race;
use oag_raceplay::PadSeeking;

/// One game-minute per seed: long enough for a few pickups per craft.
const TICKS: u64 = 60 * 60;

/// Fixed, and nothing was chosen for passing.
const SEEDS: [u64; 6] = [1, 2, 3, 4, 5, 6];

fn eliminator(seed: u64) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Eliminator,
        eliminator_kill_target: Some(5),
        seed: Some(seed),
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup))
}

/// Pickups the opponents collected, and the craft-ticks they spent racing
/// with an empty slot - the only ticks a pad can grant anything in.
fn pickups_and_empty_ticks(race: &mut race::Race) -> (u32, u32) {
    let parked = PlayerInputs::single(oag_gameplay::InputSnapshot::new());
    let count = race.sim.world.ship_count as usize;
    let mut was_empty = vec![true; count];
    let (mut pickups, mut empty_ticks) = (0, 0);
    for _ in 0..TICKS {
        race.tick(&parked);
        for (slot, was) in was_empty.iter_mut().enumerate().skip(1) {
            let ship = &race.sim.world.ships[slot];
            let empty = ship.pickup.is_empty();
            if empty && ship.physics.craft_state == CraftState::Racing {
                empty_ticks += 1;
            }
            if *was && !empty {
                pickups += 1;
            }
            *was = empty;
        }
        if race.sim.world.primary_race().finished {
            break;
        }
    }
    (pickups, empty_ticks)
}

/// An empty-slot opponent collects measurably more often with the steering on
/// than with it off: 5.7 pickups per empty craft-minute off and 8.0 on over
/// seeds 1 to 240 to the finish (2026-10-03). Fails if the channel is dropped
/// anywhere between `Race::pad_for` and `Driver::drift`, because the two arms
/// then drive the same race.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_empty_slot_eliminator_opponent_collects_more_often_steering_for_pads() {
    let (mut on, mut off) = ((0, 0), (0, 0));
    for seed in SEEDS {
        let Some(mut steering) = eliminator(seed) else {
            return;
        };
        assert_eq!(steering.pad_seeking(), PadSeeking::EmptySlot);
        let (p, e) = pickups_and_empty_ticks(&mut steering);
        on = (on.0 + p, on.1 + e);
        let mut plain = eliminator(seed).expect("loaded once already");
        plain.set_pad_seeking(PadSeeking::Off);
        let (p, e) = pickups_and_empty_ticks(&mut plain);
        off = (off.0 + p, off.1 + e);
    }
    let rate = |(p, e): (u32, u32)| p as f32 / (e as f32 / 3600.0);
    println!(
        "pickups per empty craft-minute: steering {:.2} ({on:?}), off {:.2} ({off:?})",
        rate(on),
        rate(off)
    );
    assert!(
        rate(on) > rate(off) * 1.2,
        "steering for pads should lift the empty-slot pickup rate: on {on:?}, off {off:?}"
    );
}

/// Every other mode keeps the authored line: the steering is Eliminator only.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_single_race_does_not_steer_for_pads() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let race = race::Race::start(loaded.setup);
    assert!(
        !race.weapon_pads().is_empty(),
        "a Single Race arms its pads"
    );
    assert_eq!(race.pad_seeking(), PadSeeking::Off);
}
