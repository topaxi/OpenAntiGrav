//! What a craft throws as it goes out of the race - see `race::wreck_fx`.
//!
//! Hand-laid effects and locators, no disc: what is asserted is the trigger
//! (the `Destroyed` to `Eliminated` edge, not the explosion before it), the
//! pair per node, and that a craft with no wreck locators throws nothing.

use super::*;
use crate::wreck_fx::{EXPLOSION_DROP, WreckFx};
use oag_livery::SparkAnchor;
use oag_physics::CraftState;
use oag_title::Trigger::{WreckNode, WreckSparks};

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
    for trigger in [WreckNode, WreckSparks] {
        let blob = super::respawn::one_emitter_pob(super::respawn::trigger_name(trigger), 0);
        let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
            .expect("the hand-laid effect parses");
        race.view.handles.insert(trigger, effect);
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

/// The big blast is thrown 1.5 s after the state 5 edge, not on it, at the
/// live model's matrix moved `EXPLOSION_DROP` of its own rows along `-up`:
/// `FUN_088407b0`'s `Psys_Spawn_q(.., "WO_SHIP_EXPLOSION", .., &matrix)`.
#[test]
fn the_big_explosion_follows_after_the_delay_below_the_craft() {
    let mut race = race_with_wreck_locators();
    let blob = super::respawn::one_emitter_pob(
        crate::tests::respawn::trigger_name(Trigger::WreckExplosion),
        0,
    );
    let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    race.view.handles.insert(Trigger::WreckExplosion, effect);
    go_out(&mut race, 3);
    let wrecked_at = race.sim.world.ships[3].physics.body.position;
    let up = race.sim.world.ships[3].physics.body.orientation * Vec3::Y;
    let after_the_nodes = race.wreck_fx_started_for_tests();
    assert_eq!(race.wreck_explosion_at_for_tests(), None, "not on the edge");
    for _ in 0..80 {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    assert_eq!(race.wreck_fx_started_for_tests(), after_the_nodes);
    // Put back on the track meanwhile, as an Eliminator craft is: the blast
    // still goes where the wreck lies.
    race.sim.world.ships[3].physics.body.position += Vec3::new(500.0, 0.0, 0.0);
    for _ in 0..20 {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    let at = race
        .wreck_explosion_at_for_tests()
        .expect("thrown by 1.5 s after the edge");
    assert_eq!(race.wreck_fx_started_for_tests(), after_the_nodes + 1);
    let below = (wrecked_at - at).dot(up);
    let expect = EXPLOSION_DROP * oag_fx::exhaust::CRAFT_ROW_SCALE;
    assert!(
        (below - expect).abs() < 1e-3,
        "{below} below, expected {expect}"
    );
    // The ring `FUN_0885ecf0` builds goes up with it, at the craft itself and not at the drop.
    let ring = race
        .view
        .bomb_blasts
        .iter()
        .flatten()
        .find(|blast| blast.kind == crate::bomb_blast::BlastKind::ShipExplosion)
        .expect("the ship explosion's shockwave starts with the big blast");
    assert!(
        (ring.position - wrecked_at).length() < 1e-3,
        "{:?}",
        ring.position
    );
}

/// A source with no wreck read throws no big blast either.
#[test]
fn a_source_with_no_wreck_throws_no_big_explosion() {
    let mut race = race_with_wreck_locators();
    race.view.wreck_fx = WreckFx::new(Vec::new());
    race.throw_wreck_explosion(3, Mat4::IDENTITY);
    assert_eq!(race.wreck_explosion_at_for_tests(), None);
}
