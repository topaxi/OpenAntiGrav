//! The original's distance trigger (`0x08841cec`) and the shield charge its reset
//! state (and every other entry to it) costs a person's craft.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_race::recovery::{DISTANCE_RESET_TICKS, DISTANCE_RESET_UNITS};

fn far_race() -> (Race, Vec3) {
    let mut race = Race::start(setup(hulled_handling()));
    let away = Vec3::new(20.0, 6.0, 5_000.0);
    assert!(
        race.spline().distance_to(away).expect("a sample") > DISTANCE_RESET_UNITS,
        "the fixture does not put the craft beyond the distance"
    );
    race.sim.world.ships[0].physics.body.position = away;
    (race, away)
}

/// Half a second beyond 200 units from the located sample resets the craft, and a
/// tick short of it does not.
#[test]
fn a_craft_held_beyond_200_units_for_30_ticks_resets_and_not_at_29() {
    let (mut race, away) = far_race();
    for _ in 0..DISTANCE_RESET_TICKS - 1 {
        race.tick(&PlayerInputs::none());
        race.sim.world.ships[0].physics.body.position = away;
    }
    assert_eq!(race.respawns(), 0, "reset before the 30th tick");
    race.tick(&PlayerInputs::none());
    assert_eq!(race.respawns(), 1, "not reset on the 30th tick");
    assert_eq!(
        race.last_respawn_cause_of(0),
        Some(RespawnCause::Distance),
        "something else put the craft back"
    );
}

/// The count is consecutive: a tick inside the distance starts it again.
#[test]
fn coming_back_inside_zeroes_the_distance_dwell() {
    let (mut race, away) = far_race();
    for _ in 0..DISTANCE_RESET_TICKS - 1 {
        race.tick(&PlayerInputs::none());
        race.sim.world.ships[0].physics.body.position = away;
    }
    assert_eq!(race.sim.distant_ticks[0], DISTANCE_RESET_TICKS - 1);
    let start = race.sim.spline.start().expect("a sample").pos;
    race.sim.world.ships[0].physics.body.position = Vec3::from_array(start);
    race.tick(&PlayerInputs::none());
    assert_eq!(race.sim.distant_ticks[0], 0);
}

fn shield_after_a_reset(shield: f32) -> f32 {
    let mut handling = hulled_handling();
    handling.dimensions.shield = 100.0;
    let mut race = Race::start(setup(handling));
    race.sim.weapons_on = true;
    race.sim.world.ships[0].physics.shield = shield;
    race.reset(0, None);
    race.sim.world.ships[0].physics.shield
}

/// `min(5, shield - 1)`, floored at zero.
#[test]
fn a_reset_costs_a_person_min_5_and_shield_less_1() {
    assert_eq!(shield_after_a_reset(100.0), 95.0);
    assert_eq!(shield_after_a_reset(4.0), 1.0);
    assert_eq!(shield_after_a_reset(3.0), 1.0);
    assert_eq!(shield_after_a_reset(1.0), 1.0);
}

/// An AI craft pays nothing for the same state.
#[test]
fn a_reset_costs_an_opponent_nothing() {
    let mut handling = hulled_handling();
    handling.dimensions.shield = 100.0;
    let mut race = Race::start(setup(handling));
    race.sim.weapons_on = true;
    race.sim.world.ships[3].physics.shield = 50.0;
    let before = race.sim.world.ships[3].physics.shield;
    race.reset(3, None);
    assert_eq!(race.sim.world.ships[3].physics.shield, before);
}
