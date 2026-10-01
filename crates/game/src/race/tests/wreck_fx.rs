//! What a craft throws as it goes out of the race - see `race::wreck_fx`.
//!
//! Hand-laid effects and locators, no disc: what is asserted is the trigger
//! (the `Destroyed` to `Eliminated` edge, not the explosion before it), the
//! pair per node, and that a craft with no wreck locators throws nothing.

use super::*;
use crate::livery::SparkAnchor;
use crate::race::wreck_fx::{DEATH_SPARKS_EFFECT, FXNODE_EXPLO_EFFECT, WreckFx};
use oag_physics::CraftState;

fn locators(count: usize) -> Vec<SparkAnchor> {
    (0..count)
        .map(|i| SparkAnchor {
            position: Vec3::new(i as f32, 0.0, 0.0),
            up: Vec3::Y,
        })
        .collect()
}

/// A grid whose slot 3 has three wreck locators and the two effects loaded.
fn race_with_wreck_locators() -> Race {
    let mut race = race_with_a_grid();
    assert!(race.sim.world.ship_count > 3, "the fixture has opponents");
    let mut anchors = vec![Vec::new(); 4];
    anchors[3] = locators(3);
    race.view.wreck_fx = WreckFx::new(anchors);
    for name in [FXNODE_EXPLO_EFFECT, DEATH_SPARKS_EFFECT] {
        let blob = super::respawn::one_emitter_pob(name, 0);
        let effect = oag_render::psys::Effect::parse(&blob, oag_render::psys::ColourScale::Full)
            .expect("the hand-laid effect parses");
        race.view.effects.insert(name, effect);
    }
    race
}

fn go_out(race: &mut Race, slot: usize) {
    race.sim.world.ships[slot].physics.craft_state = CraftState::Destroyed;
    race.advance_craft_flashes();
    assert_eq!(
        race.wreck_fx_started_for_tests(),
        0,
        "the explosion (state 4) throws nothing"
    );
    race.sim.world.ships[slot].physics.craft_state = CraftState::Eliminated;
    race.advance_craft_flashes();
}

/// State 5 spawns the explosion and the sparks, in a pair, at each node.
#[test]
fn a_craft_going_out_throws_a_pair_at_each_wreck_locator() {
    let mut race = race_with_wreck_locators();
    go_out(&mut race, 3);
    assert_eq!(race.wreck_fx_started_for_tests(), 6);
}

/// A craft with no locators on its wreck, or a source with no wreck at all,
/// throws nothing - `FUN_0883e064` walks the gathered node list and a null
/// one ends the loop.
#[test]
fn a_craft_with_no_wreck_locators_throws_nothing() {
    let mut race = race_with_wreck_locators();
    go_out(&mut race, 2);
    assert_eq!(race.wreck_fx_started_for_tests(), 0);
}

/// The instances ride the craft while they emit and are let go after, so a
/// run of wrecks does not hold every stage slot.
#[test]
fn the_wreck_effects_are_let_go_once_they_stop_emitting() {
    let mut race = race_with_wreck_locators();
    go_out(&mut race, 3);
    for _ in 0..60 {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    assert_eq!(race.wreck_fx_riding_for_tests(), 0);
}
