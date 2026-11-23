//! HD's destroyed craft's sounds start and stop on `Ship_SetState`'s edges
//! (`docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`, rows 3, 12 and 15), for the local
//! player and for an opponent, and not in the modes that play `shipdestroy` instead.

use super::*;
use oag_gameplay::cue::Cue;
use oag_physics::CraftState;
use oag_title::destroyed::DestroyedCraft;

const PLAYER: usize = 0;
const OPPONENT: usize = 3;

fn race(destroyed: DestroyedCraft, mode: Mode) -> Race {
    let mut setup = setup(hulled_handling());
    setup.destroyed = destroyed;
    setup.mode = mode;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    Race::start(
        setup,
        crate::tests::fixture_view(),
        crate::extensions::Prepared::default(),
    )
}

/// Puts `slot` in `state` with `timer` seconds left, runs the cue pass, and returns what it raised
/// for that slot.
fn step(race: &mut Race, slot: usize, state: CraftState, timer: f32) -> Vec<Cue> {
    let physics = &mut race.core.sim.world.ships[slot].physics;
    physics.craft_state = state;
    physics.state_timer = timer;
    race.core.sim.raise_destroyed_cues();
    race.core
        .sim
        .drain_cues()
        .into_iter()
        .filter(|event| usize::from(event.slot) == slot)
        .map(|event| event.cue)
        .collect()
}

#[test]
fn the_players_wreck_blasts_burns_and_stops_its_loop_at_state_6() {
    let mut race = race(DestroyedCraft::HD, Mode::SingleRace);
    assert!(race.core.sim.world.person_flown(PLAYER));
    assert_eq!(step(&mut race, PLAYER, CraftState::Racing, 0.0), []);
    assert_eq!(
        step(&mut race, PLAYER, CraftState::Destroyed, 0.3),
        [
            Cue::ShipExplosionPlayer,
            Cue::ShipDestroyed,
            Cue::ShipOnFire
        ],
        "state 4: the blast and both loops, the player's blast"
    );
    assert_eq!(
        step(&mut race, PLAYER, CraftState::Eliminated, 20.0),
        [],
        "state 5 starts nothing"
    );
    assert_eq!(
        step(&mut race, PLAYER, CraftState::Eliminated, 0.0),
        [Cue::ShipDestroyedStop],
        "state 6 stops `~SHIP_DESTROYED` and leaves the fire"
    );
    assert_eq!(step(&mut race, PLAYER, CraftState::Eliminated, 0.0), []);
    assert_eq!(
        step(&mut race, PLAYER, CraftState::Racing, 0.0),
        [Cue::ShipOnFireStop],
        "the craft back in the race (state 8 and out) ends the fire"
    );
}

#[test]
fn an_opponents_wreck_blasts_and_burns_but_has_no_destroyed_loop() {
    let mut race = race(DestroyedCraft::HD, Mode::SingleRace);
    assert!(!race.core.sim.world.person_flown(OPPONENT));
    assert_eq!(
        step(&mut race, OPPONENT, CraftState::Destroyed, 0.3),
        [Cue::ShipExplosion, Cue::ShipOnFire]
    );
    assert_eq!(step(&mut race, OPPONENT, CraftState::Eliminated, 0.0), []);
}

#[test]
fn the_eliminator_blasts_and_loops_until_state_8_but_never_burns() {
    let mut race = race(DestroyedCraft::HD, Mode::Eliminator);
    assert_eq!(
        step(&mut race, PLAYER, CraftState::Destroyed, 0.3),
        [Cue::ShipExplosionPlayer, Cue::ShipDestroyed]
    );
    assert_eq!(
        step(&mut race, PLAYER, CraftState::Eliminated, 0.0),
        [],
        "its wreck goes to state 8, not 6"
    );
    assert_eq!(
        step(&mut race, PLAYER, CraftState::Racing, 0.0),
        [Cue::ShipDestroyedStop]
    );
}

#[test]
fn zone_and_detonator_play_none_of_it() {
    for mode in [Mode::Zone, Mode::Detonator] {
        let mut race = race(DestroyedCraft::HD, mode);
        assert_eq!(step(&mut race, PLAYER, CraftState::Destroyed, 0.3), []);
        assert_eq!(step(&mut race, OPPONENT, CraftState::Destroyed, 0.3), []);
    }
}

#[test]
fn a_title_with_the_pulse_sound_raises_none_of_it() {
    for destroyed in [DestroyedCraft::PULSE, DestroyedCraft::HD_BLOWUP] {
        let mut race = race(destroyed, Mode::SingleRace);
        assert_eq!(step(&mut race, PLAYER, CraftState::Destroyed, 0.3), []);
        assert!(race.core.craft_is_exploding(), "`~BLOWUP` stays theirs");
    }
}

#[test]
fn hd_never_plays_blowup() {
    let mut race = race(DestroyedCraft::HD, Mode::SingleRace);
    step(&mut race, PLAYER, CraftState::Destroyed, 0.3);
    assert!(!race.core.craft_is_exploding());
}
