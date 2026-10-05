//! The weapon cues whose trigger was recovered on 2026-09-30: a Missile's two
//! endings, the LeachBeam that finds nothing to lock, and the Shuriken's launch.
//!
//! Each fixture is one of `cues.rs`'s own two-craft shapes, kept in a file of
//! their own because `cues.rs` is near the 1,000-line ceiling. What a cue
//! *sounds like* is `crates/game/tests/sfx_weapon_ground_truth.rs`.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_sound::sfx::Cue;

/// One craft parked at the origin with `weapon`, and, when `target` is set, a
/// second 15 units down its nose (60 for the LeachBeam's lock window). The
/// button is pressed on tick 1 and the run reads every cue raised.
fn press_once(
    weapon: oag_tables::weapons::Weapon,
    table: oag_tables::weapons::WeaponStats,
    target: Option<f32>,
    ticks: usize,
) -> std::collections::BTreeSet<Cue> {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(table);
    let mut race = Race::start(setup);
    if target.is_some() {
        race.sim.world.ship_count = 2;
        race.sim.world.ships[1].active = true;
        race.sim.world.ships[1].handling = hulled_handling();
    }
    race.sim.world.ships[0].pickup.weapon = Some(weapon);

    let mut buttons = Buttons::new();
    let mut raised = std::collections::BTreeSet::new();
    for tick in 0..ticks {
        race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
        let forward = race.sim.world.ships[0].physics.body.forward();
        race.sim.world.ships[0].physics.body.linear_velocity = forward * 60.0;
        if let Some(distance) = target {
            race.sim.world.ships[1].physics.body.position = forward * distance;
            race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
        }
        let snapshot = buttons.tick(if tick == 1 { SQUARE } else { 0 });
        race.tick(&PlayerInputs::single(snapshot));
        raised.extend(race.drain_cues().into_iter().map(|e| e.cue));
    }
    raised
}

pub(super) fn missile_hits_a_craft() -> std::collections::BTreeSet<Cue> {
    press_once(
        oag_tables::weapons::Weapon::Missile,
        one_missile_table(),
        Some(15.0),
        240,
    )
}

pub(super) fn missile_outlives_its_fuse() -> std::collections::BTreeSet<Cue> {
    press_once(
        oag_tables::weapons::Weapon::Missile,
        one_missile_table(),
        None,
        260,
    )
}

pub(super) fn leach_fires_unlocked() -> std::collections::BTreeSet<Cue> {
    press_once(
        oag_tables::weapons::Weapon::LeachBeam,
        one_leach_beam_table(),
        None,
        10,
    )
}

pub(super) fn shuriken_is_thrown() -> std::collections::BTreeSet<Cue> {
    press_once(
        oag_tables::weapons::Weapon::Shuriken,
        one_shuriken_table(),
        None,
        10,
    )
}

/// `MissilePool_Update`'s teardown plays `MISSILEEXPSHIP` on the craft-hit bit
/// and never the wall cue or the fuse cue on the same ending.
#[test]
fn a_missile_that_hits_a_craft_raises_missileexpship_and_only_that_ending() {
    let raised = missile_hits_a_craft();
    assert!(
        raised.contains(&Cue::MissileHitShip),
        "MISSILEEXPSHIP never fired on a craft hit: {raised:?}"
    );
    assert!(
        !raised.contains(&Cue::MissileExpire),
        "the fuse cue fired on a craft hit: {raised:?}"
    );
}

/// The pool's `3.0 < age` pass plays the cue at `0x08a7c950`, which reads
/// `"SHURIKENEXPL"`, and a fuse ending strikes nothing.
#[test]
fn a_missile_that_outlives_its_fuse_raises_the_cue_the_original_names_shurikenexpl() {
    let raised = missile_outlives_its_fuse();
    assert!(
        raised.contains(&Cue::MissileExpire),
        "SHURIKENEXPL never fired at the fuse: {raised:?}"
    );
    assert!(
        !raised.contains(&Cue::MissileHitShip),
        "the craft-hit cue fired with no craft: {raised:?}"
    );
}

/// The three ways a Missile impact reaches the cue table: a craft hit, the fuse
/// (`blast: false`), and a wall ending after the bounce budget, which is neither.
#[test]
fn a_missile_that_spends_its_bounces_is_neither_the_fuse_nor_a_craft_hit() {
    use oag_weapons::projectile::Impact;
    let base = Impact {
        point: Vec3::ZERO,
        kind: oag_tables::weapons::Weapon::Missile,
        owner: 0,
        struck: None,
        blast: true,
        effect: None,
    };
    let ending = |impact: Impact| super::super::weapons::missile_ending_cue(&impact);
    assert_eq!(
        ending(Impact {
            struck: Some(1),
            ..base
        }),
        Some(Cue::MissileHitShip)
    );
    assert_eq!(
        ending(Impact {
            blast: false,
            ..base
        }),
        Some(Cue::MissileExpire)
    );
    assert_eq!(ending(base), None, "a wall ending plays neither");
    assert_eq!(
        ending(Impact {
            kind: oag_tables::weapons::Weapon::Plasma,
            blast: false,
            ..base
        }),
        None,
        "another weapon's no-blast ending is not the Missile's fuse"
    );
}

/// `LeachBeam_InitUnlocked` plays `LEACHFAIL`; only the locked constructor
/// plays `LEACH`.
#[test]
fn a_leachbeam_fired_with_no_lock_raises_leachfail_and_not_leach() {
    let raised = leach_fires_unlocked();
    assert!(raised.contains(&Cue::LeachFail), "{raised:?}");
    assert!(!raised.contains(&Cue::Leach), "{raised:?}");
}

/// `Shuriken_Init` plays `SHURIKEN` on the firing craft's own emitter.
#[test]
fn a_thrown_shuriken_raises_its_launch_cue() {
    let raised = shuriken_is_thrown();
    assert!(raised.contains(&Cue::ShurikenLaunch), "{raised:?}");
}
