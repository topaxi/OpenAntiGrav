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

/// 2048's grant flies the icon at `dt * 1800.0` pixels a second once the announcement is over
/// (`Hud_UpdatePickupIcon`, `0x44e10000`): nothing while the timer is up, a step on the frame it
/// reaches zero and on each after, back to zero when nothing is held. Fails if the travel stops
/// advancing, starts early, or does not reset on a regrant.
#[test]
fn the_grant_flies_at_1800_pixels_a_second_once_the_announcement_ends() {
    use crate::slot_hud::SlotHud;
    use oag_tables::weapons::Weapon;
    let dt = 1.0 / 60.0;
    let step = 1800.0 * dt;
    let mut hud = SlotHud::default();
    let held = Some(Weapon::Missile);

    let mut travel = Vec::new();
    for _ in 0..80 {
        hud.advance(100.0, 1.0, 0.0, false, held, dt);
        travel.push((hud.pickup_announce_left > 0.0, hud.pickup_fly_travel));
    }
    let announcing = travel.iter().take_while(|(up, _)| *up).count();
    assert!(
        (59..=60).contains(&announcing),
        "a second of announcement, got {announcing} ticks"
    );
    assert!(
        travel[..announcing].iter().all(|(_, flown)| *flown == 0.0),
        "nothing flies while the caption is up"
    );
    for (n, (_, flown)) in travel[announcing..].iter().enumerate() {
        let expected = step * (n + 1) as f32;
        assert!(
            (flown - expected).abs() < 0.01,
            "{n} ticks after the announcement: {flown} px, expected {expected}"
        );
    }

    hud.advance(100.0, 1.0, 0.0, false, None, dt);
    assert_eq!(hud.pickup_fly_travel, 0.0, "nothing held, no flight");
    hud.advance(100.0, 1.0, 0.0, false, held, dt);
    assert_eq!(hud.pickup_fly_travel, 0.0, "a regrant announces first");
}

/// The fly reaches the readout: a grant on a race reads `Some(0.0)` through the announcement,
/// then 30 px a tick along y. Fails if `Race::readout` drops `SlotHud::pickup_fly_travel`.
#[test]
fn a_grant_on_a_race_reads_its_flight_off_the_readout() {
    use oag_tables::weapons::Weapon;
    let mut race = race_with_a_grid();
    let slot = race.player_slot();
    race.sim.world.ships[slot].pickup.weapon = Some(Weapon::Missile);
    let mut flights = Vec::new();
    for _ in 0..70 {
        race.tick(&PlayerInputs::none());
        let readout = race.readout();
        flights.push((readout.pickup_announcing, readout.pickup_fly));
    }
    let announcing = flights.iter().take_while(|(up, _)| *up).count();
    assert!(
        flights[..announcing]
            .iter()
            .all(|(_, fly)| *fly == Some(0.0))
    );
    let after: Vec<f32> = flights[announcing..]
        .iter()
        .map(|(_, fly)| fly.expect("a race always reads its flight"))
        .collect();
    assert!(
        after.len() >= 8,
        "ticks after the announcement: {}",
        after.len()
    );
    for pair in after.windows(2) {
        assert!(
            (pair[1] - pair[0] - 30.0).abs() < 0.01,
            "30 px a tick: {pair:?}"
        );
    }
}

/// The pickup fade ([`oag_hud::Readout::pickup_fade`]) advances per tick off the held weapon
/// once a title's rate is seeded, runs on after use with `pickup` itself `None`, and stays idle
/// for a title that seeds none. Fails if the slot's state is dropped from the readout.
#[test]
fn a_held_weapon_plays_the_fade_and_the_last_one_fades_out_after_use() {
    use oag_tables::weapons::Weapon;
    let mut race = race_with_a_grid();
    let slot = race.player_slot();
    race.sim.world.ships[slot].pickup.weapon = Some(Weapon::Missile);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        !race.readout().pickup_fade.is_active(),
        "no rate seeded, no fade"
    );

    race.sim.world.ships[slot].pickup.weapon = None;
    for _ in 0..200 {
        race.tick(&PlayerInputs::none());
    }
    race.view.huds[slot].pickup_fade_rate = 1.2;
    race.sim.world.ships[slot].pickup.weapon = Some(Weapon::Missile);
    for _ in 0..12 {
        race.tick(&PlayerInputs::none());
    }
    let readout = race.readout();
    let look = readout.pickup_fade.look(readout.pickup).unwrap();
    assert_eq!(look.weapon, Weapon::Missile);
    assert!(look.backdrop > 1.5, "mid-overshoot, got {}", look.backdrop);

    race.sim.world.ships[slot].pickup.weapon = None;
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    let readout = race.readout();
    assert_eq!(readout.pickup, None);
    let after = readout.pickup_fade.look(None).unwrap();
    assert_eq!(after.weapon, Weapon::Missile, "the last weapon fades out");
}
