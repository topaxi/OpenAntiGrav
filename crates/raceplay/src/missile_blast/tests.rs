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

fn streak_pool() -> oag_title::weapons::BlastStreak {
    oag_title::weapons::BlastStreak {
        texture: "ThickLine_bighalo",
        line: oag_title::weapons::FatLineSpec {
            width: [0.0, 2.0],
            lifetime: 1.2,
            u_per_unit: 0.2,
            chains: 1,
            capacity: 350,
            phase_step: 0.06,
            radius: 0.6,
            ramp: false,
        },
        pool: 20,
    }
}

fn streaking_race() -> Race {
    let mut race = hd_race();
    race.view.hd_missile_streaks = Some(streaks::Streaks::new(streak_pool()));
    race
}

/// Elements the blast in `slot` took.
fn taken(race: &Race, slot: usize) -> usize {
    race.view
        .hd_missile_streaks
        .as_ref()
        .and_then(|pool| pool.entries(slot))
        .expect("a blast with streaks")
        .iter()
        .filter(|streak| streak.element.is_some())
        .count()
}

/// `fcmpu ... bgt` against 1.0: an entry whose age is exactly a second still lives
/// (`MissileExplosion_Update`, `0x00154a78`), and the next update frees it. Halves are
/// exact in `f32`, so the age lands on 1.0 itself.
#[test]
fn retirement_is_strictly_past_a_second() {
    let mut race = hd_race();
    race.spawn_hd_missile_blast(Vec3::ZERO, Quat::IDENTITY);
    race.advance_hd_missile_blasts(0.5);
    race.advance_hd_missile_blasts(0.5);
    let (_, age) = race.hd_missile_blast_draws()[0].expect("alive at exactly 1.0 s");
    assert_eq!(age, 1.0);
    race.advance_hd_missile_blasts(0.5);
    assert_eq!(live(&race), 0, "freed on the first update past 1.0 s");
}

/// A blast starts its sixteen streaks from the shared pool of twenty: the first takes
/// sixteen, a second inside their life only the four left, a third none.
#[test]
fn blasts_share_a_pool_of_twenty() {
    let mut race = streaking_race();
    race.spawn_hd_missile_blast(Vec3::ZERO, Quat::IDENTITY);
    assert_eq!(taken(&race, 0), 16);
    race.spawn_hd_missile_blast(Vec3::X, Quat::IDENTITY);
    assert_eq!(taken(&race, 1), 4, "only the slots left");
    race.spawn_hd_missile_blast(Vec3::Y, Quat::IDENTITY);
    assert_eq!(taken(&race, 2), 0, "none when the pool is empty");
    assert_eq!(race.missile_streaks_live(), 20);
}

/// The streaks outlive the blast by their nodes' life, and the pool frees an element when
/// its last node dies, not when the entry retires.
#[test]
fn the_streaks_outlive_the_blast_by_their_node_life() {
    let mut race = streaking_race();
    race.spawn_hd_missile_blast(Vec3::ZERO, Quat::IDENTITY);
    let dt = 1.0 / 60.0;
    for _ in 0..61 {
        race.advance_hd_missile_blasts(dt);
    }
    assert_eq!(live(&race), 0, "the blast is spent");
    assert_eq!(race.missile_streaks_live(), 16, "the streaks are not");
    let eye = Vec3::new(0.0, 0.0, 100.0);
    let ramp = oag_fx::rocket_smoke::OpacityRamp::from_tga(&[0; 0x14 + 256]).expect("a ramp");
    assert!(!race.missile_streak_vertices(eye, &ramp).is_empty());
    // The last node was laid at 1.0 s with a life of at most 1.2 s.
    for _ in 0..74 {
        race.advance_hd_missile_blasts(dt);
    }
    assert_eq!(race.missile_streaks_live(), 0, "every node has died");
    assert!(race.missile_streak_vertices(eye, &ramp).is_empty());
}

/// A freshly started entry sits at the blast's centre on an orthonormal basis, and its
/// first update lays a node there before it moves `speed` along its row 1.
#[test]
fn an_entry_lays_a_node_at_the_centre_then_steps_along_its_row() {
    let mut race = streaking_race();
    let centre = Vec3::new(10.0, 20.0, 30.0);
    race.spawn_hd_missile_blast(centre, Quat::from_rotation_y(0.7));
    let entries = |race: &Race| {
        *race
            .view
            .hd_missile_streaks
            .as_ref()
            .unwrap()
            .entries(0)
            .unwrap()
    };
    let started = entries(&race);
    for streak in &started {
        assert_eq!(streak.position, centre);
        assert!((7.0..8.0).contains(&streak.speed));
        assert!((0.5..1.0).contains(&streak.scale));
        let [r0, r1, r2] = streak.rows;
        assert!((r0.length() - 1.0).abs() < 1e-4 && (r1.length() - 1.0).abs() < 1e-4);
        assert!(r0.dot(r1).abs() < 1e-4 && r1.dot(r2).abs() < 1e-4 && r0.dot(r2).abs() < 1e-4);
    }
    race.advance_hd_missile_blasts(1.0 / 60.0);
    let stepped = entries(&race);
    for (before, after) in started.iter().zip(&stepped) {
        let moved = after.position - centre;
        assert!(
            (moved.dot(before.rows[1]) - before.speed).abs() < 1e-3,
            "the step along row 1 is the speed, not times dt"
        );
        assert!(moved.dot(before.rows[0]).abs() < 4.0 && moved.dot(before.rows[2]).abs() < 4.0);
    }
    let directions: Vec<Vec3> = started.iter().map(|streak| streak.rows[1]).collect();
    assert!(
        directions
            .iter()
            .any(|direction| direction.dot(directions[0]) < 0.9),
        "the sixteen streaks fly in different directions"
    );
}

/// A title without the streak pool starts none, and the blast is otherwise unchanged.
#[test]
fn a_title_without_streaks_starts_none() {
    let mut race = hd_race();
    race.spawn_hd_missile_blast(Vec3::ZERO, Quat::IDENTITY);
    assert!(race.view.hd_missile_streaks.is_none());
    race.advance_hd_missile_blasts(1.0 / 60.0);
    assert_eq!(race.missile_streaks_live(), 0);
}
