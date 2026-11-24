//! An ordinary craft's debris (`docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`, "The debris
//! throw"): 19 pieces on the 4 -> 5 edge, two of them burning, gone after 10 s.
//!
//! Hand-laid pieces and effect, no disc; the disc's own pieces are in
//! `tests/ship_debris_ground_truth.rs`.

use super::*;
use oag_livery::ShipDebrisModels;
use oag_physics::CraftState;
use oag_title::Trigger;
use oag_title::destroyed::DestroyedCraft;
use std::sync::Arc;

fn models(pieces: usize) -> Arc<ShipDebrisModels> {
    let copies = oag_livery::ship_debris::COPIES;
    let mut model = oag_mesh::mesh::Model::none("pieces");
    model.vertices = vec![bytemuck::Zeroable::zeroed(); pieces * 3 * copies];
    Arc::new(ShipDebrisModels {
        debris: model,
        pieces: (0..pieces as u32)
            .map(|piece| (piece * 3..piece * 3 + 3).collect())
            .collect(),
        stride: pieces as u32 * 3,
    })
}

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
    let mut view = crate::tests::fixture_view();
    view.ship_debris = destroyed.ship_debris.map(|_| models(19));
    view.ship_debris_spec = destroyed.ship_debris;
    let mut race = Race::start(setup, view, crate::extensions::Prepared::default());
    let blob = super::respawn::one_emitter_pob("WO_DEBRIS_FIRE", 0);
    let fire = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    race.view.handles.insert(Trigger::DebrisFire, fire);
    assert!(race.core.sim.world.ship_count > 3);
    race
}

fn at(race: &mut Race, state: CraftState) {
    race.core.sim.world.ships[3].physics.craft_state = state;
    race.view.advance_craft_flashes(&race.core);
}

/// Nothing is thrown on state 4; on the 4 -> 5 edge, 19 pieces once, the last two burning.
#[test]
fn an_hd_craft_throws_19_pieces_on_the_edge_to_state_5() {
    let mut race = race(DestroyedCraft::HD);
    at(&mut race, CraftState::Racing);
    at(&mut race, CraftState::Destroyed);
    assert_eq!(race.view.ship_debris_counts_for_tests(), (0, 0), "state 4");
    at(&mut race, CraftState::Eliminated);
    assert_eq!(race.view.ship_debris_counts_for_tests(), (1, 19));
    assert_eq!(race.view.ship_debris_flying_for_tests(), 19);
    assert_eq!(race.view.ship_debris_burning_for_tests(), 2);
    assert_eq!(race.view.ship_debris_burning_pieces_for_tests(), [17, 18]);
    at(&mut race, CraftState::Eliminated);
    assert_eq!(
        race.view.ship_debris_counts_for_tests(),
        (1, 19),
        "once, not every tick"
    );
}

/// A piece goes 10 s after the throw, and its fire with it.
#[test]
fn the_pieces_and_their_fires_go_after_10_s() {
    let mut race = race(DestroyedCraft::HD);
    at(&mut race, CraftState::Destroyed);
    at(&mut race, CraftState::Eliminated);
    let dt = race.core.sim.dt;
    let ticks = |secs: f32| (secs / dt) as usize;
    for _ in 0..ticks(4.0) {
        at(&mut race, CraftState::Eliminated);
    }
    assert_eq!(race.view.ship_debris_burning_for_tests(), 2, "at 4 s");
    for _ in 0..ticks(5.5) {
        at(&mut race, CraftState::Eliminated);
    }
    assert_eq!(race.view.ship_debris_flying_for_tests(), 19);
    assert_eq!(race.view.ship_debris_burning_for_tests(), 2, "at 9.5 s");
    for _ in 0..ticks(1.0) {
        at(&mut race, CraftState::Eliminated);
    }
    assert_eq!(race.view.ship_debris_flying_for_tests(), 0);
    assert_eq!(race.view.ship_debris_burning_for_tests(), 0);
}

/// Pulse breaks into no pieces.
#[test]
fn pulse_throws_no_debris() {
    let mut race = race(DestroyedCraft::PULSE);
    at(&mut race, CraftState::Destroyed);
    at(&mut race, CraftState::Eliminated);
    assert_eq!(race.view.ship_debris_counts_for_tests(), (0, 0));
}

/// The scatter is forward of the body: no piece is thrown backwards of the craft's own velocity.
#[test]
fn every_piece_leaves_forward_of_the_craft() {
    let mut race = race(DestroyedCraft::HD);
    let forward = race.core.sim.world.ships[3].physics.body.forward();
    race.core.sim.world.ships[3].physics.body.linear_velocity = forward * 100.0;
    at(&mut race, CraftState::Destroyed);
    at(&mut race, CraftState::Eliminated);
    let speeds = race.view.ship_debris_launch_speeds_for_tests(forward);
    assert_eq!(speeds.len(), 19);
    for speed in speeds {
        assert!(speed >= 100.0, "{speed} against the craft's 100");
    }
}
