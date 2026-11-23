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
/// `EXPLBIG` follows `EXPLSMALL` by the blast's delay.
#[test]
fn every_craft_voices_explsmall_at_state_5_and_explbig_at_the_big_blast() {
    for slot in [PLAYER, OPPONENT] {
        let mut race = race(DestroyedCraft::PURE);
        assert_eq!(step(&mut race, slot, CraftState::Racing, 0.0), []);
        assert_eq!(step(&mut race, slot, CraftState::Destroyed, 0.5), []);
        assert_eq!(
            step(&mut race, slot, CraftState::Eliminated, 1.5),
            [Cue::ShipExplosionSmall],
            "state 5"
        );
        let mut heard = Vec::new();
        for _ in 0..100 {
            heard.extend(step(&mut race, slot, CraftState::Eliminated, 1.0));
        }
        assert_eq!(heard, [Cue::ShipExplosion], "the big blast, once");
    }
}

/// A world that stands still under the results (a Zone wreck) still sounds the big blast: the
/// cue rides the blast's own clock, not the sim's state timer.
#[test]
fn explbig_sounds_in_a_world_that_stands_still() {
    let mut race = race(DestroyedCraft::PURE);
    step(&mut race, PLAYER, CraftState::Destroyed, 0.5);
    step(&mut race, PLAYER, CraftState::Eliminated, 1.5);
    let mut heard = Vec::new();
    for _ in 0..100 {
        race.tick_cosmetics();
        heard.extend(
            race.core
                .sim
                .drain_cues()
                .into_iter()
                .map(|event| event.cue),
        );
    }
    assert!(heard.contains(&Cue::ShipExplosion), "{heard:?}");
}

/// Pulse voices neither: its table is not Pure's.
#[test]
fn pulse_voices_neither_blast() {
    let mut race = race(DestroyedCraft::PULSE);
    step(&mut race, OPPONENT, CraftState::Destroyed, 0.5);
    assert_eq!(step(&mut race, OPPONENT, CraftState::Eliminated, 1.5), []);
    assert_eq!(step(&mut race, OPPONENT, CraftState::Eliminated, 0.0), []);
}

/// The wreck is drawn through state 5 and gone once the big blast fires, on Pulse and Pure; HD's
/// lies in view. Run with the world ticking and with it standing still under the results (a Zone
/// wreck), where only the cosmetics step.
#[test]
fn the_wreck_is_gone_from_the_big_blast_on() {
    for (title, hidden) in [
        (DestroyedCraft::PURE, true),
        (DestroyedCraft::PULSE, true),
        (DestroyedCraft::HD, false),
    ] {
        for world_runs in [true, false] {
            let mut race = race(title);
            let step = |race: &mut Race| {
                if world_runs {
                    race.tick(&oag_gameplay::PlayerInputs::none());
                } else {
                    race.tick_cosmetics();
                }
            };
            race.core.sim.world.ships[OPPONENT].physics.craft_state = CraftState::Destroyed;
            race.view.advance_craft_flashes(&race.core);
            race.core.sim.world.ships[OPPONENT].physics.craft_state = CraftState::Eliminated;
            race.core.sim.world.ships[OPPONENT].physics.state_timer = 1.5;
            race.view.advance_craft_flashes(&race.core);
            for _ in 0..60 {
                step(&mut race);
            }
            assert!(!race.view.wreck_gone(OPPONENT), "state 5");
            for _ in 0..40 {
                step(&mut race);
            }
            assert_eq!(race.view.wreck_gone(OPPONENT), hidden, "after the blast");
            race.core.sim.world.ships[OPPONENT].physics.craft_state = CraftState::Racing;
            race.view.advance_craft_flashes(&race.core);
            assert!(!race.view.wreck_gone(OPPONENT), "put back on the track");
        }
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
        if hidden_at.is_none() && race.view.wreck_gone(OPPONENT) {
            hidden_at = Some(tick);
        }
        if tick > 40 && before == 0 && race.view.live_gaseous_flashes_for_tests() == 1 {
            blast_at = Some(tick);
        }
    }
    assert!(hidden_at.is_some(), "the wreck is hidden at some point");
    assert_eq!(hidden_at, blast_at);
}
