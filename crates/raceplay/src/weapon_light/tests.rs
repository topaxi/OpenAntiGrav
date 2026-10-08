//! Unit tests for [`super`]: the Missile's law at its ends, the Rocket's
//! literals, and that a live blast or rocket reaches the built list.

use super::*;
use crate::tests::race_with_a_grid;
use oag_core::math::Quat;

fn hd_race() -> Race {
    let mut race = race_with_a_grid();
    race.view.hd_missile_blast = true;
    race.view.hd_trail_active = true;
    race
}

/// At birth `f = 1`: `D = 150`, `w = 8.5`, the seeded colour.
#[test]
fn a_new_missile_light_is_a_hundred_and_fifty_wide_and_steep() {
    let light = missile_record(Vec3::new(1.0, 2.0, 3.0), 0.0).unwrap();
    assert_eq!(light.position, [1.0, 2.0, 3.0, 8.5]);
    assert_eq!(light.colour, [500.0, 200.0, 50.0, 150.0]);
}

/// Half way `f = 0.25`: the range and the exponent both follow `(1 - p)^2`.
#[test]
fn the_missile_light_dies_quadratically() {
    let light = missile_record(Vec3::ZERO, 0.5).unwrap();
    assert_eq!(light.colour[3], 37.5);
    assert_eq!(light.position[3], 3.25);
}

/// The entry is freed at `p = 1`, and the light with it.
#[test]
fn a_spent_missile_blast_lights_nothing() {
    assert!(missile_record(Vec3::ZERO, LIFETIME_SECONDS).is_none());
    assert!(missile_record(Vec3::ZERO, -0.1).is_none());
}

/// `D = 100`, `w = 1`, `(14, 10, 2)`.
#[test]
fn the_rocket_light_is_the_literals_the_update_loads() {
    let light = rocket_record(Vec3::new(4.0, 5.0, 6.0));
    assert_eq!(light.position, [4.0, 5.0, 6.0, 1.0]);
    assert_eq!(light.colour, [14.0, 10.0, 2.0, 100.0]);
}

/// Dropping the wiring from the built list fails this.
#[test]
fn a_live_missile_blast_reaches_the_frame_list() {
    let mut race = hd_race();
    let before = race.hd_spu_lights().len();
    race.spawn_hd_missile_blast(Vec3::new(7.0, 8.0, 9.0), Quat::IDENTITY);
    let lights = race.hd_spu_lights();
    assert_eq!(lights.len(), before + 1);
    let light = lights.last().unwrap();
    assert_eq!(light.position, [7.0, 8.0, 9.0, 8.5]);
}

/// Off HD, or with the circuit's switch off, the weapons light nothing.
#[test]
fn the_circuits_switch_silences_the_weapons_too() {
    let mut race = hd_race();
    race.spawn_hd_missile_blast(Vec3::ZERO, Quat::IDENTITY);
    race.view.spu_vertex_lights = false;
    assert!(race.hd_spu_lights().is_empty());
    race.view.spu_vertex_lights = true;
    race.view.hd_trail_active = false;
    assert!(race.hd_spu_lights().is_empty());
}
