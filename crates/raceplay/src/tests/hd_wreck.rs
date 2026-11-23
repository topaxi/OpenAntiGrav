//! HD/Fury's destroyed craft against Pulse's: the same engine, a title's own sequence
//! (`docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`).
//!
//! Hand-laid effects and a fixture grid, no disc: what is asserted is which edge throws what,
//! the explosion's length, and the wreck leaving the world.

use super::*;
use crate::wreck_fx::{EXPLOSION_DROP, WreckFx};
use oag_physics::CraftState;
use oag_title::Trigger;
use oag_title::destroyed::DestroyedCraft;

fn with(destroyed: DestroyedCraft) -> Race {
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

fn effect(trigger: Trigger) -> oag_fx::psys::Effect {
    let name = super::respawn::trigger_name(trigger);
    let blob = super::respawn::one_emitter_pob(name, 0);
    oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses")
}

/// A grid whose slot 3 has a wreck and `triggers` loaded, and neither of Pulse's two
/// per-locator effects.
fn grid(destroyed: DestroyedCraft, triggers: &[Trigger]) -> Race {
    let mut race = with(destroyed);
    assert!(race.core.sim.world.ship_count > 3);
    race.view.wreck_fx = WreckFx::new(vec![Vec::new(); 4]);
    for &trigger in triggers {
        race.view.handles.insert(trigger, effect(trigger));
    }
    race.core.sim.world.ships[3].physics.shield = 0.0;
    race
}

fn at(race: &mut Race, state: CraftState) {
    race.core.sim.world.ships[3].physics.craft_state = state;
    race.view.advance_craft_flashes(&race.core);
}

/// HD throws `WO_SHIP_EXPLOSION` on entering state 4, 4.0 of the model's rows behind it, and the
/// death plume on entering state 5; nothing is left for 1.5 s later.
#[test]
fn hd_throws_the_blast_at_state_4_and_the_plume_at_state_5() {
    let mut race = grid(
        DestroyedCraft::HD,
        &[Trigger::WreckExplosion, Trigger::WreckPlume],
    );
    at(&mut race, CraftState::Racing);
    assert_eq!(race.wreck_fx_started_for_tests(), 0);
    at(&mut race, CraftState::Destroyed);
    assert_eq!(
        race.wreck_fx_started_for_tests(),
        1,
        "the blast, on state 4"
    );
    assert_eq!(race.damage_smokes_started_for_tests(), 0, "no plume yet");
    let wrecked_at = race.core.sim.world.ships[3].physics.body.position;
    let up = race.core.sim.world.ships[3].physics.body.orientation * Vec3::Y;
    let blast = race
        .wreck_explosion_at_for_tests()
        .expect("the blast is placed");
    let expect = EXPLOSION_DROP * oag_fx::exhaust::CRAFT_ROW_SCALE;
    assert!(((wrecked_at - blast).dot(up) - expect).abs() < 1e-3);
    at(&mut race, CraftState::Eliminated);
    assert_eq!(
        race.damage_smokes_started_for_tests(),
        1,
        "the plume, on state 5"
    );
    assert_eq!(
        race.wreck_fx_started_for_tests(),
        1,
        "no second blast on state 5"
    );
    for _ in 0..120 {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    assert_eq!(race.wreck_fx_started_for_tests(), 1, "nothing 1.5 s on");
}

/// Pulse's trigger rule is untouched: nothing on state 4, no plume, the blast 1.5 s after state 5.
#[test]
fn pulse_throws_neither_at_those_edges() {
    // Pulse's table has the blast and neither the plume nor the node effects here.
    let mut race = grid(DestroyedCraft::PULSE, &[Trigger::WreckExplosion]);
    at(&mut race, CraftState::Destroyed);
    assert_eq!(race.wreck_fx_started_for_tests(), 0);
    at(&mut race, CraftState::Eliminated);
    assert_eq!(race.wreck_fx_started_for_tests(), 0);
    assert_eq!(race.damage_smokes_started_for_tests(), 0);
}

/// HD's state 4 is `0.3` s, Pulse's `0.5`: the same destroyed craft is out of the race sooner.
#[test]
fn the_explosion_is_0_3_s_on_hd_and_0_5_s_on_pulse() {
    let ticks_out = |destroyed| {
        let mut race = with(destroyed);
        race.core.sim.force_destroy(1);
        let dt = race.core.sim.dt();
        let mut ticks = 0;
        while race.core.sim.world.ships[1].physics.craft_state == CraftState::Destroyed {
            race.tick(&oag_gameplay::PlayerInputs::none());
            ticks += 1;
            assert!(ticks < 100, "the explosion never ended");
        }
        (ticks, dt)
    };
    let (hd, dt) = ticks_out(DestroyedCraft::HD);
    let (pulse, _) = ticks_out(DestroyedCraft::PULSE);
    assert!(
        (f32::from(hd as u16) * dt - 0.3).abs() <= dt * 1.5,
        "{hd} ticks"
    );
    assert!(
        (f32::from(pulse as u16) * dt - 0.5).abs() <= dt * 1.5,
        "{pulse} ticks"
    );
    assert!(hd < pulse);
}

/// State 6 on HD takes the body out of the physics world, where Pulse's wreck keeps landing.
#[test]
fn an_hd_wreck_in_state_6_stops_being_integrated() {
    let moved = |destroyed| {
        let mut race = with(destroyed);
        let ship = &mut race.core.sim.world.ships[1].physics;
        ship.craft_state = CraftState::Eliminated;
        ship.state_timer = 0.0;
        ship.body.linear_velocity = Vec3::new(0.0, 0.0, 30.0);
        let before = ship.body.position;
        for _ in 0..10 {
            race.tick(&oag_gameplay::PlayerInputs::none());
        }
        race.core.sim.world.ships[1]
            .physics
            .body
            .position
            .distance(before)
    };
    assert_eq!(moved(DestroyedCraft::HD), 0.0);
    assert!(moved(DestroyedCraft::PULSE) > 0.0);
}
