//! A person whose link dropped and came back: the seat held away, the Autopilot countdown that
//! hands the craft back over the pickup held before the drop, and the cancel.
//!
//! The pace of the away driver needs a real circuit and is measured by
//! `oag-net`'s `net_ground_truth` (`an_away_craft_loses_time_against_the_opponent_driver`).

use super::autopilot::two_people;
use super::*;
use oag_gameplay::{Controller, PlayerInputs};
use oag_hud::SubWeapon;
use oag_tables::weapons::Weapon;

/// Two people (slot 1 away and returning), slot 1 holding a Turbo.
fn returning() -> Race {
    let mut race = two_people(Some(0), true);
    race.sim.world.ships[1].pickup.weapon = Some(Weapon::Turbo);
    race
}

#[test]
fn an_away_seat_keeps_the_pickup_it_held() {
    let mut race = returning();
    assert!(race.set_away(1));
    assert!(!race.set_away(1), "once");
    assert_eq!(race.sim.world.controllers[1], Controller::Away);
    for _ in 0..120 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.sim.world.ships[1].pickup.weapon, Some(Weapon::Turbo));
}

#[test]
fn only_a_person_goes_away_and_only_an_away_seat_is_handed_back() {
    let mut race = returning();
    assert!(!race.set_away(5), "an opponent's slot is no person's");
    assert!(!race.hand_back(1), "a present person is not handed back");
    assert!(race.set_away(1));
    assert!(race.hand_back(1));
    assert!(!race.hand_back(1), "once");
}

/// The HUD shows an Autopilot pickup and its countdown over the pickup that is held; the
/// pickup is shown again, usable, when the seconds are up.
#[test]
fn a_hand_back_counts_down_as_an_autopilot_over_the_held_pickup() {
    let mut race = returning();
    race.set_away(1);
    race.hand_back(1);
    assert_eq!(race.sim.world.controllers[1], Controller::Remote);
    let shown = race.readout_for(1);
    assert_eq!(shown.pickup, Some(Weapon::Autopilot));
    assert_eq!(
        shown.sub_weapon,
        Some(SubWeapon::Autopilot(crate::replay::HAND_BACK_SECONDS))
    );
    assert_eq!(race.sim.world.ships[1].pickup.weapon, Some(Weapon::Turbo));
    assert!(race.sim.flown_for_the_player(1), "the driver has the craft");
    assert_eq!(
        race.sim.autopilot_engine_flags(1).speedup,
        None,
        "a hand-back gives no Autopilot boost"
    );

    for _ in 0..90 {
        race.tick(&PlayerInputs::none());
    }
    let mid = race.readout_for(1);
    assert_eq!(mid.pickup, Some(Weapon::Autopilot));
    let Some(SubWeapon::Autopilot(left)) = mid.sub_weapon else {
        panic!("{:?}", mid.sub_weapon);
    };
    assert!((0.9..1.1).contains(&left), "a second left, not {left}");

    for _ in 0..70 {
        race.tick(&PlayerInputs::none());
    }
    let after = race.readout_for(1);
    assert_eq!(after.pickup, Some(Weapon::Turbo), "the pickup is back");
    assert_eq!(after.sub_weapon, None);
    let ship = &race.sim.world.ships[1];
    assert!(!ship.handback);
    assert_eq!(ship.autopilot_timer, 0.0);
    assert!(!race.sim.flown_for_the_player(1), "the person flies again");
}

/// Absorb cancels the countdown on every title, and it does not pay out the held pickup.
#[test]
fn absorb_cancels_a_hand_back_and_keeps_the_pickup() {
    let mut race = returning();
    race.sim.autopilot_absorb_cancels = false;
    race.set_away(1);
    race.hand_back(1);
    let mut buttons = Buttons::new();
    let mut inputs = PlayerInputs::none();
    inputs.set(1, buttons.tick(CIRCLE));
    race.tick(&inputs);
    let ship = &race.sim.world.ships[1];
    assert_eq!(ship.autopilot_timer, 0.0, "the countdown was cancelled");
    assert!(!ship.handback);
    assert_eq!(
        ship.pickup.weapon,
        Some(Weapon::Turbo),
        "the pickup was kept"
    );
}

/// A real Autopilot pickup still cancels as the title's law says, and keeps its boost.
#[test]
fn a_real_autopilot_is_not_a_hand_back() {
    let mut race = returning();
    race.sim.world.ships[1].autopilot_timer = 2.0;
    assert!(!race.sim.world.ships[1].handback);
    assert_eq!(race.sim.autopilot_engine_flags(1).speedup, Some(1.2));
}

/// The state the recording replays: a re-drive that applies the two events reaches the state
/// the live change made, and a value outside the kind is refused.
#[test]
fn presence_events_reproduce_the_change() {
    let live = |steps: u8| {
        let mut race = returning();
        race.set_away(1);
        if steps > 1 {
            race.hand_back(1);
        }
        race.sim.state_hash()
    };
    let applied = |values: &[u32]| {
        let mut race = returning();
        for &value in values {
            race.sim
                .apply_event(&oag_replay::Event::new(
                    0,
                    1,
                    oag_replay::Kind::Presence,
                    value,
                ))
                .expect("applies");
        }
        race.sim.state_hash()
    };
    assert_eq!(applied(&[0]), live(1));
    assert_eq!(applied(&[0, 1]), live(2));
    assert_ne!(live(1), live(2));
    assert_ne!(live(1), returning().sim.state_hash());
    assert!(
        returning()
            .sim
            .apply_event(&oag_replay::Event::new(0, 1, oag_replay::Kind::Presence, 2))
            .is_err()
    );
}

/// A thrust press, a pulled airbrake or a hard stick ends the countdown on that tick, but not the
/// input already held when the link came back.
#[test]
fn a_deliberate_input_takes_the_craft_back_after_a_short_grace() {
    let cancels = |snapshot: InputSnapshot, wait: u32| {
        let mut race = returning();
        race.set_away(1);
        race.hand_back(1);
        for _ in 0..wait {
            race.tick(&PlayerInputs::none());
        }
        let mut inputs = PlayerInputs::none();
        inputs.set(1, snapshot);
        race.tick(&inputs);
        let ship = &race.sim.world.ships[1];
        assert_eq!(
            ship.pickup.weapon,
            Some(Weapon::Turbo),
            "the pickup is kept"
        );
        !ship.handback
    };
    let mut buttons = Buttons::new();
    let thrust = buttons.tick(CROSS);
    assert!(!cancels(thrust, 0), "inside the grace");
    let mut buttons = Buttons::new();
    assert!(cancels(buttons.tick(CROSS), 30));
    let steer = InputSnapshot {
        stick_x: 0.8,
        ..InputSnapshot::default()
    };
    assert!(cancels(steer, 30));
    let brake = InputSnapshot {
        airbrake_left: 1.0,
        ..InputSnapshot::default()
    };
    assert!(cancels(brake, 30));
    let drift = InputSnapshot {
        stick_x: 0.2,
        ..InputSnapshot::default()
    };
    assert!(!cancels(drift, 30), "inside the deadzone");
}

/// A one-person room whose only person goes away: the craft is flown by the driver below
/// racing pace, not stepped as the player, and the view stays on it.
#[test]
fn a_lone_person_who_goes_away_is_not_stepped_as_the_player() {
    let mut race = two_people(Some(0), true);
    race.sim.world.controllers[1] = Controller::Ai;
    assert!(race.sim.world.person_flown(0));
    assert!(race.set_away(0));
    assert_eq!(race.sim.world.controllers[0], Controller::Away);
    assert_eq!(
        race.sim.world.primary_slot(),
        0,
        "the view stays on their craft"
    );
    assert!(!race.sim.world.person_flown(0), "the driver flies it");
    let mut held = PlayerInputs::none();
    held.set(0, oag_gameplay::InputSnapshot::EMPTY);
    for _ in 0..600 {
        race.tick(&held);
    }
    assert!(
        race.sim.world.ships[0].physics.body.position.x.abs() > 1.0,
        "a driver moved the craft that no input was holding"
    );
    assert!(race.hand_back(0));
    assert!(race.sim.world.person_flown(0), "the person flies it again");
}
