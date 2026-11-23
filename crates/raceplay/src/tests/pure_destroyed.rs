//! Pure's destroyed craft (`docs/ghidra/functions/psp-pure-usa/ship-destroy.md`): `EXPLSMALL`
//! at state 5 and `EXPLBIG` at state 6 for every craft, the gaseous flash on both edges, and
//! the wreck gone from the big blast on (Pulse's too).

use super::*;
use oag_gameplay::cue::Cue;
use oag_physics::CraftState;
use oag_title::destroyed::DestroyedCraft;

const PLAYER: usize = 0;
const OPPONENT: usize = 3;

fn race(destroyed: DestroyedCraft) -> Race {
    let mut setup = setup(hulled_handling());
    setup.destroyed = destroyed;
    setup.mode = Mode::SingleRace;
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

/// Every craft, the player included, voices the same two cues: no `_PC`, no `SHIP_DEST`.
#[test]
fn every_craft_voices_explsmall_at_state_5_and_explbig_at_state_6() {
    for slot in [PLAYER, OPPONENT] {
        let mut race = race(DestroyedCraft::PURE);
        assert_eq!(step(&mut race, slot, CraftState::Racing, 0.0), []);
        assert_eq!(step(&mut race, slot, CraftState::Destroyed, 0.5), []);
        assert_eq!(
            step(&mut race, slot, CraftState::Eliminated, 1.5),
            [Cue::ShipExplosionSmall],
            "state 5"
        );
        assert_eq!(step(&mut race, slot, CraftState::Eliminated, 0.5), []);
        assert_eq!(
            step(&mut race, slot, CraftState::Eliminated, 0.0),
            [Cue::ShipExplosion],
            "state 6"
        );
        assert_eq!(step(&mut race, slot, CraftState::Eliminated, 0.0), []);
    }
}

/// Pulse voices neither: its table is not Pure's.
#[test]
fn pulse_voices_neither_blast() {
    let mut race = race(DestroyedCraft::PULSE);
    step(&mut race, OPPONENT, CraftState::Destroyed, 0.5);
    assert_eq!(step(&mut race, OPPONENT, CraftState::Eliminated, 1.5), []);
    assert_eq!(step(&mut race, OPPONENT, CraftState::Eliminated, 0.0), []);
}

/// The wreck is drawn through state 5 and gone on the tick the timer runs out, on Pulse and Pure;
/// HD's lies in view.
#[test]
fn the_wreck_is_gone_from_the_big_blast_on() {
    for (title, hidden) in [
        (DestroyedCraft::PURE, true),
        (DestroyedCraft::PULSE, true),
        (DestroyedCraft::HD, false),
    ] {
        let mut race = race(title);
        fn ship(race: &mut Race) -> &mut oag_physics::ShipState {
            &mut race.core.sim.world.ships[OPPONENT].physics
        }
        ship(&mut race).craft_state = CraftState::Eliminated;
        ship(&mut race).state_timer = 0.01;
        assert!(!race.core.sim.wreck_hidden(OPPONENT), "state 5");
        ship(&mut race).state_timer = 0.0;
        assert_eq!(race.core.sim.wreck_hidden(OPPONENT), hidden, "state 6");
        ship(&mut race).craft_state = CraftState::Racing;
        assert!(!race.core.sim.wreck_hidden(OPPONENT), "put back on track");
    }
}

/// The flash is thrown on the 4 -> 5 edge and again when the big blast fires, and only where the
/// title names the model.
#[test]
fn the_gaseous_flash_fires_on_both_edges_where_the_title_has_one() {
    for (title, flashes) in [(DestroyedCraft::PURE, true), (DestroyedCraft::PULSE, false)] {
        let mut race = race(title);
        let live = |race: &Race| race.view.live_gaseous_flashes_for_tests();
        race.core.sim.world.ships[OPPONENT].physics.craft_state = CraftState::Destroyed;
        race.view.advance_craft_flashes(&race.core);
        assert_eq!(live(&race), 0, "state 4");
        race.core.sim.world.ships[OPPONENT].physics.craft_state = CraftState::Eliminated;
        race.view.advance_craft_flashes(&race.core);
        assert_eq!(live(&race), usize::from(flashes), "the 4 -> 5 edge");
        for _ in 0..40 {
            race.view.advance_bomb_blast_models(race.core.sim.dt);
        }
        assert_eq!(live(&race), 0, "retired at 0.5 s");
        for _ in 0..95 {
            race.tick(&oag_gameplay::PlayerInputs::none());
        }
        assert_eq!(live(&race), usize::from(flashes), "the big blast");
    }
}

/// The wreck leaves view on the tick the big blast is thrown, not one tick before or after.
#[test]
fn the_wreck_is_hidden_on_the_tick_the_big_blast_fires() {
    let mut race = race(DestroyedCraft::PURE);
    race.core.sim.world.ships[OPPONENT].physics.craft_state = CraftState::Destroyed;
    race.view.advance_craft_flashes(&race.core);
    race.core.sim.world.ships[OPPONENT].physics.craft_state = CraftState::Eliminated;
    race.core.sim.world.ships[OPPONENT].physics.state_timer = 1.5;
    race.view.advance_craft_flashes(&race.core);
    let (mut hidden_at, mut blast_at) = (None, None);
    for tick in 1..130 {
        let before = race.view.live_gaseous_flashes_for_tests();
        race.tick(&oag_gameplay::PlayerInputs::none());
        if hidden_at.is_none() && race.core.sim.wreck_hidden(OPPONENT) {
            hidden_at = Some(tick);
        }
        if tick > 40 && before == 0 && race.view.live_gaseous_flashes_for_tests() == 1 {
            blast_at = Some(tick);
        }
    }
    assert!(hidden_at.is_some(), "the wreck is hidden at some point");
    assert_eq!(hidden_at, blast_at);
}
