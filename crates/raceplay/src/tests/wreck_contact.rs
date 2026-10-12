//! A destroyed craft and craft-against-craft contact: solid while it explodes and for state 5's
//! `1.5` s, then not in a single race, and solid throughout in the Eliminator.
//!
//! Measured on PPSSPP v1.20.4 (Pulse USA), 2026-10-12: a teleport of an opponent onto the
//! stationary player fired `Body_ResolveContactPair` for exactly that pair in states 4, 5 and
//! (Eliminator) 8 and never in state 6, the box collider's `+0x68` byte reading `1, 1, 1, 0`.
//! `docs/ghidra/functions/psp-pulse-usa/wreck-collision.md`.

use super::*;
use oag_physics::CraftState;

fn grid(mode: Mode) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = mode;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    Race::start(setup)
}

/// Slot 1 on top of slot 0 and closing, `down` the state slot 1 is in. Returns how far the pass
/// moved slot 1.
fn pushed(mode: Mode, down: CraftState, timer: f32) -> f32 {
    let mut race = grid(mode);
    let at = race.sim.world.ships[0].physics.body.position;
    race.sim.world.ships[1].physics.body.position = at + Vec3::new(0.0, 0.0, 1.0);
    race.sim.world.ships[0].physics.body.linear_velocity = Vec3::new(0.0, 0.0, 8.0);
    race.sim.world.ships[1].physics.body.linear_velocity = Vec3::new(0.0, 0.0, -8.0);
    race.sim.world.ships[1].physics.craft_state = down;
    race.sim.world.ships[1].physics.state_timer = timer;
    let before = race.sim.world.ships[1].physics.body.position;
    race.resolve_craft_pairs();
    race.sim.world.ships[1]
        .physics
        .body
        .position
        .distance(before)
}

#[test]
fn a_craft_still_exploding_is_pushed_like_any_other() {
    assert!(pushed(Mode::SingleRace, CraftState::Destroyed, 0.3) > 0.0);
}

#[test]
fn a_wreck_in_state_five_is_still_solid() {
    assert!(pushed(Mode::SingleRace, CraftState::Eliminated, 1.0) > 0.0);
}

#[test]
fn a_wreck_in_state_six_is_driven_through() {
    assert_eq!(
        pushed(Mode::SingleRace, CraftState::Eliminated, 0.0),
        0.0,
        "a state 6 wreck was pushed; the original's collider byte is zero there"
    );
}

#[test]
fn an_eliminator_wreck_is_solid_until_it_respawns() {
    assert!(pushed(Mode::Eliminator, CraftState::Eliminated, 0.0) > 0.0);
}

#[test]
fn the_craft_driving_through_a_wreck_is_left_alone_too() {
    let mut race = grid(Mode::SingleRace);
    let at = race.sim.world.ships[0].physics.body.position;
    race.sim.world.ships[1].physics.body.position = at + Vec3::new(0.0, 0.0, 1.0);
    race.sim.world.ships[0].physics.body.linear_velocity = Vec3::new(0.0, 0.0, 8.0);
    race.sim.world.ships[1].physics.craft_state = CraftState::Eliminated;
    race.sim.world.ships[1].physics.state_timer = 0.0;
    race.resolve_craft_pairs();
    assert_eq!(
        race.sim.world.ships[0].physics.body.linear_velocity,
        Vec3::new(0.0, 0.0, 8.0),
        "the live craft was slowed by a wreck that is not solid"
    );
}
