//! A split-screen view's HUD reads its own slot: the numbers, and the bars that
//! animate from them.

use super::*;
use oag_gameplay::PlayerInputs;

#[test]
fn a_slots_readout_is_off_its_own_craft() {
    let mut race = race_with_a_grid();
    for _ in 0..5 {
        race.tick(&PlayerInputs::none());
    }
    race.sim.world.ships[1].physics.shield = 25.0;
    race.sim.world.ships[2].physics.shield = 75.0;
    race.sim.world.ships[1].physics.body.linear_velocity = Vec3::new(30.0, 0.0, 0.0);

    let (one, two) = (race.readout_for(1), race.readout_for(2));
    assert_eq!((one.shield, two.shield), (25.0, 75.0));
    assert!(one.speed_kmh > two.speed_kmh);
    assert_eq!(one.place, u32::from(race.places()[1]));
    assert_eq!(two.place, u32::from(race.places()[2]));
    assert_eq!(race.readout(), race.readout_for(race.player_slot()));
}

#[test]
fn a_message_shows_in_the_view_of_the_slot_it_was_raised_for() {
    let mut race = race_with_a_grid();
    race.sim.world.controllers[1] = oag_gameplay::Controller::Local;
    race.raise_message("IG_HUD_TEST", true);
    for _ in 0..2 {
        race.tick(&PlayerInputs::none());
    }
    assert!(race.readout_for(0).messages.iter().any(Option::is_some));
    assert!(race.readout_for(1).messages.iter().all(Option::is_none));
}

#[test]
fn a_viewed_slots_shield_bar_flashes_on_a_hit_and_an_unviewed_slots_does_not() {
    let mut race = race_with_a_grid();
    assert_eq!(race.set_viewport_count(2), 2);
    // A full pool on slots 1 (viewed) and 2 (not), settled, then a hit on each.
    for slot in [1, 2] {
        let ship = &mut race.sim.world.ships[slot];
        ship.handling.dimensions.shield = 100.0;
        ship.physics.shield = 100.0;
    }
    for _ in 0..5 {
        race.tick(&PlayerInputs::none());
    }
    for slot in [1, 2] {
        race.sim.world.ships[slot].physics.shield = 50.0;
    }
    race.tick(&PlayerInputs::none());
    assert!(
        race.readout_for(1).shield_flashing,
        "the viewed slot's bar flashes on the drop"
    );
    assert!(
        !race.readout_for(2).shield_flashing,
        "a slot nobody is looking at keeps no HUD state"
    );
}

#[test]
fn with_one_view_no_other_slots_bars_move() {
    let mut race = race_with_a_grid();
    for _ in 0..5 {
        race.tick(&PlayerInputs::none());
    }
    let ship = &mut race.sim.world.ships[1];
    ship.handling.dimensions.shield = 100.0;
    ship.physics.shield = 100.0;
    for _ in 0..5 {
        race.tick(&PlayerInputs::none());
    }
    race.sim.world.ships[1].physics.shield = 50.0;
    race.tick(&PlayerInputs::none());
    assert!(!race.readout_for(1).shield_flashing);
}

/// 2048's pickup grant announcement ([`oag_hud::Readout::pickup_announcing`]) is up for one
/// second after the held weapon changes (`Hud_UpdatePickupIcon`'s `1.0` at `hud+0x228`), not
/// before a grant, and re-arms on a new grant of the same weapon after the slot was empty.
#[test]
fn a_grant_announces_for_one_second_and_a_regrant_announces_again() {
    use oag_tables::weapons::Weapon;
    let mut race = race_with_a_grid();
    let ticks_up = |race: &mut Race| {
        let mut up = 0;
        for _ in 0..90 {
            race.tick(&PlayerInputs::none());
            up += u32::from(race.readout().pickup_announcing);
        }
        up
    };
    assert_eq!(ticks_up(&mut race), 0, "nothing held, nothing announced");

    let slot = race.player_slot();
    race.sim.world.ships[slot].pickup.weapon = Some(Weapon::Missile);
    let up = ticks_up(&mut race);
    assert!((59..=60).contains(&up), "a second at 60 Hz, got {up} ticks");
    assert!(!race.readout().pickup_announcing);

    race.sim.world.ships[slot].pickup.weapon = None;
    race.tick(&PlayerInputs::none());
    race.sim.world.ships[slot].pickup.weapon = Some(Weapon::Missile);
    race.tick(&PlayerInputs::none());
    assert!(race.readout().pickup_announcing);
}
