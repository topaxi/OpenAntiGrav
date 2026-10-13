//! A second person's fire button spends *their* pickup and credits *their*
//! craft.
//!
//! `Race::spend_pickup` ran for slot 0 alone until `Race::tick` began to step
//! every person-flown slot, and its arms for the Rocket, the Plasma, the
//! Shuriken, the Quake and the Repulser carried a literal `0` as the shot's
//! owner. With slot 1 pressing fire that is a rocket owned by slot 0: it would
//! pass through slot 0's craft and strike slot 1's. Each test below presses
//! fire in slot 1 only, and asserts the shot is slot 1's.

use super::*;
use oag_gameplay::{Controller, PlayerInputs};
use oag_tables::weapons::Weapon;

/// A grid race with slot 1 person-flown, holding `weapon` under `table`, and
/// a few ticks run so the standings have located every craft.
fn second_person_holding(weapon: Weapon, table: oag_tables::weapons::WeaponStats) -> Race {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(table);
    race.sim.world.controllers[1] = Controller::Local;
    assert!(race.sim.world.person_flown(1), "slot 1 must be person-flown");
    for _ in 0..10 {
        race.tick(&PlayerInputs::none());
    }
    race.sim.world.ships[1].pickup.weapon = Some(weapon);
    race
}

/// Presses fire in slot 1 for one tick and returns the race after it.
fn press_in_slot_1(race: &mut Race) {
    let mut buttons = Buttons::new();
    buttons.tick(0);
    let mut inputs = PlayerInputs::none();
    inputs.set(1, buttons.tick(SQUARE));
    race.tick(&inputs);
}

#[test]
fn a_second_persons_rocket_volley_is_theirs() {
    let mut race = second_person_holding(Weapon::Rocket, one_rocket_table());
    press_in_slot_1(&mut race);
    let shots: Vec<u8> = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(Weapon::Rocket))
        .map(|p| p.owner)
        .collect();
    assert!(!shots.is_empty(), "slot 1's press fired no rocket");
    assert!(
        shots.iter().all(|&owner| owner == 1),
        "slot 1's rockets are owned by {shots:?}"
    );
}

#[test]
fn a_second_persons_plasma_charge_and_cue_are_theirs() {
    let mut race = second_person_holding(Weapon::Plasma, one_plasma_table());
    press_in_slot_1(&mut race);
    let owner = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .find(|p| p.kind == Some(Weapon::Plasma))
        .map(|p| p.owner);
    assert_eq!(owner, Some(1), "slot 1's plasma charge is owned by {owner:?}");
    let cue = race
        .drain_cues()
        .into_iter()
        .find(|e| e.cue == oag_sound::sfx::Cue::Plasma)
        .expect("the press raised no Plasma cue");
    assert_eq!(cue.slot, 1, "the Plasma cue sounds from slot {}", cue.slot);
}

#[test]
fn a_second_persons_shuriken_is_theirs() {
    let mut race = second_person_holding(Weapon::Shuriken, one_shuriken_table());
    press_in_slot_1(&mut race);
    let owners: Vec<u8> = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(Weapon::Shuriken))
        .map(|p| p.owner)
        .collect();
    assert!(!owners.is_empty(), "slot 1's press threw nothing");
    assert!(owners.iter().all(|&owner| owner == 1), "owned by {owners:?}");
}

#[test]
fn a_second_persons_quake_wave_is_theirs() {
    let mut race = second_person_holding(Weapon::Quake, one_quake_table());
    press_in_slot_1(&mut race);
    let wave = race.sim.world.quake.expect("slot 1's press fired no wave");
    assert_eq!(wave.owner, 1, "the wave would hit its own firer");
}

#[test]
fn a_second_persons_repulser_is_theirs() {
    let mut race = second_person_holding(Weapon::Repulser, one_repulser_table());
    press_in_slot_1(&mut race);
    let owners: Vec<u8> = race
        .sim
        .world
        .repulsers
        .iter()
        .flatten()
        .map(|r| r.owner)
        .collect();
    assert_eq!(owners, vec![1], "slot 1's repulser is owned by {owners:?}");
}

/// The Missile's reticle hold belongs to the primary slot's reticle, not to
/// slot 0 as such: with slot 0 an opponent and a person in slot 1, slot 1 is
/// the primary and its candidate needs the hold, while slot 0's does not.
#[test]
fn the_missiles_reticle_hold_follows_the_primary_slot() {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(one_missile_table());
    race.sim.world.controllers[0] = Controller::Ai;
    race.sim.world.controllers[1] = Controller::Local;
    assert_eq!(race.sim.world.primary_slot(), 1);
    let stats = race
        .sim
        .weapons
        .as_ref()
        .and_then(oag_tables::weapons::WeaponStats::missile)
        .expect("the fixture authors a Missile");
    let forward = race.sim.world.ships[1].physics.body.forward();
    // Slot 0 is dead ahead of slot 1, and slot 1 dead ahead of nothing.
    race.sim.world.ships[0].physics.body.position =
        race.sim.world.ships[1].physics.body.position + forward * 60.0;
    let target_of = |race: &Race, owner: u8| {
        race.sim
            .world
            .projectiles
            .slots
            .iter()
            .find(|p| p.kind == Some(Weapon::Missile) && p.owner == owner)
            .map(|p| p.target)
    };
    assert!(race.fire_missile(1, &stats));
    assert_eq!(
        target_of(&race, 1),
        Some(None),
        "the primary's candidate was passed through before its reticle locked"
    );
}
