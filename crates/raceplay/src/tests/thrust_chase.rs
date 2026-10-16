//! [`oag_hud::Readout::thrust_chase_percent`] through a real [`Race::tick`]:
//! 2048's `ThrustBar` rises at 140 percent a second while thrust is held and
//! falls at 100 a second once it is released.

use super::*;
use oag_gameplay::PlayerInputs;

fn held(race: &mut Race, ticks: u32) {
    let mut input = HeldButtons::new(oag_gameplay::input::Button::Cross.bit());
    for _ in 0..ticks {
        race.tick(&PlayerInputs::single(input.snapshot()));
    }
}

#[test]
fn the_bar_rises_at_140_a_second_and_falls_at_100() {
    let mut race = Race::start(setup(Handling::default()));
    for _ in 0..oag_race::COUNTDOWN_TICKS {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.readout().thrust_chase_percent, 0.0);

    held(&mut race, 30);
    let risen = race.readout().thrust_chase_percent;
    assert!(
        (risen - 70.0).abs() < 0.01,
        "half a second at 140 a second is 70, got {risen}"
    );

    held(&mut race, 60);
    assert_eq!(race.readout().thrust_chase_percent, 100.0);

    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    let fallen = race.readout().thrust_chase_percent;
    assert!(
        (fallen - 50.0).abs() < 0.01,
        "half a second at 100 a second is 50, got {fallen}"
    );
}

/// HD's `ThrustBar` has no chase of its own: `Readout::thrust_percent` is the
/// craft's throttle after the tick, so it is zero with the button up and at
/// the full figure the moment the physics reaches it.
#[test]
fn hds_thrust_percent_is_the_craft_throttle_with_no_chase() {
    let mut race = Race::start(setup(Handling::default()));
    for _ in 0..oag_race::COUNTDOWN_TICKS {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.readout().thrust_percent, 0.0);

    held(&mut race, 90);
    let held_value = race.readout().thrust_percent;
    assert_eq!(held_value, race.ship().physics.thrust);
    assert!(
        held_value > 90.0,
        "full throttle reads near 100, got {held_value}"
    );

    for _ in 0..90 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(race.readout().thrust_percent, 0.0);
}
