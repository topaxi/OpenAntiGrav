//! The race feeds the tally the in-race awards beyond the zone step, the kill and the held
//! place: laps, the launch, a weapon pad, the player's place and the weapon hits, each from the
//! edge the simulation already reports.

use super::*;
use crate::xp_tally::XpTally;
use oag_gameplay::PlayerInputs;
use oag_tables::weapons::Weapon;
use oag_tables::xp::{Awards, Column};

/// Every award these edges pay, at its own index as the value so a swapped pair reads as a
/// wrong total.
fn table() -> Awards {
    let rows: String = [5, 8, 11, 12, 13, 17, 21, 22, 23, 32, 33, 44]
        .iter()
        .map(|i| {
            format!(r#"<Award index="{i}" name="a{i}" zone="{i}" elim="{i}" zombie="{i}" race="{i}" speed="{i}"/>"#)
        })
        .collect();
    Awards::parse(&format!("<XPAwards>{rows}</XPAwards>")).expect("fixture")
}

fn armed(mut race: Race) -> Race {
    race.arm_xp(XpTally::new(table(), Column::Race));
    race
}

fn paid(race: &Race, index: u32) -> u32 {
    race.xp_tally().expect("armed").times_paid(index)
}

fn past_the_countdown(race: &mut Race, ticks: u64) {
    for _ in 0..oag_race::COUNTDOWN_TICKS + ticks {
        race.tick(&PlayerInputs::none());
    }
}

/// Fails if the launch edge stops feeding the tally.
#[test]
fn a_perfect_launch_pays_boost_start() {
    let mut race = armed(Race::start(perfect_start::launch_setup()));
    let mut buttons = Buttons::new();
    for tick in 0..oag_race::COUNTDOWN_TICKS + 40 {
        let press = tick >= oag_race::COUNTDOWN_TICKS + 12;
        race.tick(&PlayerInputs::single(buttons.tick(if press {
            CROSS
        } else {
            0
        })));
    }
    assert_eq!(paid(&race, 11), 1);
}

/// Fails if the grant stops feeding the tally, or the countdown stops gating it.
#[test]
fn a_pad_pickup_pays_once_the_race_is_on_and_not_on_the_grid() {
    let mut race = armed(race_with_weapon_pads(
        Mode::SingleRace,
        enveloping_pad(),
        1.0,
    ));
    race.tick(&PlayerInputs::none());
    assert!(
        race.ship_pickup().is_some(),
        "sanity: the pad granted on the grid"
    );
    assert_eq!(paid(&race, 8), 0, "the grid is not racing");

    past_the_countdown(&mut race, 2);
    race.sim.world.ships[0].pickup.weapon = None;
    race.sim.weapon_pad_current[0] = None;
    race.sim.weapon_pad_distance[0].fill(0.0);
    race.sim.weapon_pad_refresh_left.fill(0.0);
    race.tick(&PlayerInputs::none());
    assert!(
        race.ship_pickup().is_some(),
        "sanity: the pad granted again"
    );
    assert_eq!(paid(&race, 8), 1);
}

/// The player's best place is paid once per first reach of 1st to 3rd, and only after the
/// first crossing of the line.
#[test]
fn the_places_the_player_takes_pay_once_each() {
    let mut race = armed(race_with_a_grid());
    let me = 0;
    race.sim.world.ship_count = 2;
    past_the_countdown(&mut race, 2);
    let body = race.sim.world.ships[me].physics.body;
    race.sim.world.ships[1].physics.body.position = body.position - body.forward() * 30.0;
    race.sim.world.ships[me].standing.lap_start_tick = Some(0);
    race.sim.world.ships[1].standing.lap_start_tick = Some(0);
    race.tick(&PlayerInputs::none());
    assert_eq!(
        race.sim.places()[me],
        1,
        "sanity: the player leads the pair"
    );
    assert_eq!(paid(&race, 21), 1, "Taken 1st!");
    race.tick(&PlayerInputs::none());
    assert_eq!(paid(&race, 21), 1, "held, not paid again");
}

fn two_craft_race() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_rocket_table());
    let mut race = armed(Race::start(setup));
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    race.sim.world.ships[1].handling.dimensions.shield = 100.0;
    race.sim.world.ships[1].physics.shield = 100.0;
    past_the_countdown(&mut race, 2);
    race
}

/// A rocket the player fires into a rival pays `Hit with Rocket`; the same rocket fired by the
/// rival into the player does not.
#[test]
fn a_rocket_the_player_lands_pays_hit_with_rocket_and_one_that_lands_on_the_player_does_not() {
    let mut race = two_craft_race();
    let body = race.sim.world.ships[0].physics.body;
    race.sim.world.ships[1].physics.body.position = body.position + body.forward() * 5.0;
    race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
    let before = race.sim.world.ships[1].physics.shield;
    race.sim.world.projectiles.spawn(
        Weapon::Rocket,
        body.position + body.forward() * 2.0,
        body.forward() * 600.0,
        0,
    );
    for _ in 0..4 {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        race.sim.world.ships[1].physics.shield < before,
        "sanity: the rocket struck the rival"
    );
    assert_eq!(paid(&race, 12), 1);

    let body = race.sim.world.ships[0].physics.body;
    race.sim.world.projectiles.spawn(
        Weapon::Rocket,
        body.position - body.forward() * 3.0,
        body.forward() * 600.0,
        1,
    );
    for _ in 0..4 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(
        paid(&race, 12),
        1,
        "a hit on the player is not the player's hit"
    );
}

/// A missile the player's shield swallows pays `Shielded against Missile`.
#[test]
fn a_hit_the_players_shield_swallows_pays_shielded_against() {
    let mut race = two_craft_race();
    race.sim.world.ships[0].physics.shield_pickup_timer = 5.0;
    let body = race.sim.world.ships[0].physics.body;
    race.sim.world.projectiles.spawn(
        Weapon::Rocket,
        body.position - body.forward() * 3.0,
        body.forward() * 600.0,
        1,
    );
    for _ in 0..4 {
        race.tick(&PlayerInputs::none());
    }
    assert_eq!(paid(&race, 32), 1, "Shielded against Rocket");
}

/// The Quake wave the player fires pays `Hit with Quake` on a rival it lifts.
#[test]
fn a_quake_the_player_fires_pays_hit_with_quake() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_quake_table());
    let mut race = armed(Race::start(setup));
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Quake);
    let mut buttons = Buttons::new();
    for tick in 0..oag_race::COUNTDOWN_TICKS + 30 {
        let press = tick == oag_race::COUNTDOWN_TICKS + 1;
        race.tick(&PlayerInputs::single(buttons.tick(if press {
            SQUARE
        } else {
            0
        })));
    }
    assert_eq!(paid(&race, 17), 1);
}
