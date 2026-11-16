//! A Quake press made while a wave is live stays armed and fires once the live
//! wave has ended (`docs/ghidra/functions/ps3-hdfury-eu/weapon-instance-limits.md`;
//! measured live on Pulse PSP, `cannon-quake-leachbeam.md`, "2026-11-02").

use super::*;
use oag_gameplay::PlayerInputs;
use oag_gameplay::cue::Cue;
use oag_tables::weapons::Weapon;

fn quake_race() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_quake_table());
    Race::start(setup)
}

/// What a run saw: the ticks a wave was first seen on, the ticks no wave was up,
/// the `QUAKELAUNCH` cues, and the held weapon after the last tick.
struct Run {
    launched_on: Vec<usize>,
    empty_on: Vec<usize>,
    launch_cues: usize,
    held: Option<Weapon>,
}

/// Presses fire on `presses`, handing the craft a Quake first on the ticks in
/// `grants`, then runs `ticks` ticks in all.
fn run(race: &mut Race, grants: &[usize], presses: &[usize], ticks: usize) -> Run {
    let mut buttons = Buttons::new();
    let mut seen = Run {
        launched_on: Vec::new(),
        empty_on: Vec::new(),
        launch_cues: 0,
        held: None,
    };
    let mut was_up = false;
    for tick in 0..ticks {
        if grants.contains(&tick) {
            race.sim.world.ships[0].pickup.weapon = Some(Weapon::Quake);
        }
        let snapshot = buttons.tick(if presses.contains(&tick) { SQUARE } else { 0 });
        race.tick(&PlayerInputs::single(snapshot));
        seen.launch_cues += race
            .drain_cues()
            .iter()
            .filter(|e| e.cue == Cue::QuakeLaunch)
            .count();
        let up = race.sim.world.quake.is_some();
        if up && !was_up {
            seen.launched_on.push(tick);
        }
        if !up {
            seen.empty_on.push(tick);
        }
        was_up = up;
    }
    seen.held = race.ship_pickup();
    seen
}

/// A second press during the live wave fires exactly one second wave, on the
/// tick after the first wave ended, and the retry plays no cue of its own.
#[test]
fn a_press_during_a_live_wave_fires_one_second_wave_when_the_first_ends() {
    let mut race = quake_race();
    // Fire on 1; a second Quake handed over and pressed on 3, mashed again on 5.
    let seen = run(&mut race, &[1, 3], &[1, 3, 5], 800);
    assert_eq!(
        seen.launched_on.len(),
        2,
        "expected the press and exactly one retried wave, got launches on {:?}",
        seen.launched_on
    );
    let first_end = *seen
        .empty_on
        .iter()
        .find(|&&tick| tick > seen.launched_on[0])
        .expect("the first wave never ended");
    assert_eq!(
        seen.launched_on[1],
        first_end + 1,
        "the second wave must start the tick after the first ends"
    );
    assert_eq!(seen.held, None, "the retried wave left the pickup behind");
    assert_eq!(
        seen.launch_cues, 3,
        "one QUAKELAUNCH per press (3), none for the retry"
    );
}

/// The pickup stays in the slot until the retry fires it.
#[test]
fn the_armed_quake_keeps_the_pickup_while_the_wave_is_up() {
    let mut race = quake_race();
    let seen = run(&mut race, &[1, 3], &[1, 3], 100);
    assert_eq!(seen.launched_on.len(), 1);
    assert_eq!(seen.held, Some(Weapon::Quake));
}

/// A press with no live wave fires on that tick.
#[test]
fn a_press_with_no_live_wave_fires_at_once() {
    let mut race = quake_race();
    let seen = run(&mut race, &[1], &[1], 5);
    assert_eq!(seen.launched_on, vec![1]);
    assert_eq!(seen.held, None);
}
