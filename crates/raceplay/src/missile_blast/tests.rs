//! Unit tests for [`super`]: only a craft hit enters the pool, the age runs a
//! second, and the pool is the original's sixteen. None needs a disc.

use super::*;
use crate::tests::race_with_a_grid;
use oag_tables::weapons::Weapon;

fn hd_race() -> Race {
    let mut race = race_with_a_grid();
    race.view.hd_missile_blast = true;
    race
}

fn live(race: &Race) -> usize {
    race.hd_missile_blast_draws().iter().flatten().count()
}

/// `0x001423a8` takes a pool entry in its craft-hit branch only; a missile
/// that dies on a wall leaves the count at zero (live, `weapons.md`).
#[test]
fn only_a_missile_that_reaches_a_craft_enters_the_pool() {
    let mut race = hd_race();
    race.ignite_blast(Weapon::Missile, Vec3::ZERO, None, Quat::IDENTITY);
    assert_eq!(live(&race), 0, "a wall hit takes no entry");
    race.ignite_blast(Weapon::Missile, Vec3::ZERO, Some(0), Quat::IDENTITY);
    assert_eq!(live(&race), 1, "a craft hit takes one");
    race.ignite_blast(Weapon::Rocket, Vec3::ZERO, Some(0), Quat::IDENTITY);
    assert_eq!(live(&race), 1, "a rocket is not a missile");
}

/// Off HD nothing is played: the model is the title's, not a default.
#[test]
fn a_title_without_the_model_plays_nothing() {
    let mut race = race_with_a_grid();
    race.ignite_blast(Weapon::Missile, Vec3::ZERO, Some(0), Quat::IDENTITY);
    assert_eq!(live(&race), 0);
}

/// The age field runs 0 to 1 at one per second and the entry is freed when it
/// gets there (two boots, `weapons.md`).
#[test]
fn an_entry_lives_one_second() {
    let mut race = hd_race();
    race.spawn_hd_missile_blast(Vec3::ZERO, Quat::IDENTITY);
    let dt = 1.0 / 60.0;
    for _ in 0..58 {
        race.advance_hd_missile_blasts(dt);
    }
    let (_, age) = race.hd_missile_blast_draws()[0].expect("still live at 0.97 s");
    assert!(
        (age - 58.0 * dt).abs() < 1e-4,
        "the age is the elapsed seconds"
    );
    for _ in 0..3 {
        race.advance_hd_missile_blasts(dt);
    }
    assert_eq!(live(&race), 0, "freed once the age reached a second");
}

/// `pool_take` enters an object while `count < 16`; a seventeenth is dropped.
#[test]
fn the_pool_is_sixteen() {
    let mut race = hd_race();
    for _ in 0..SLOTS + 1 {
        race.spawn_hd_missile_blast(Vec3::ZERO, Quat::IDENTITY);
    }
    assert_eq!(live(&race), SLOTS);
}
