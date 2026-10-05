//! The struck hull's damage sparks - see `race::hit_sparks`.
//!
//! Hand-laid effect and locators, no disc: what is asserted is the trigger's
//! own rules (one or two locators per landed hit, a 0.8 s gate per locator,
//! nothing on an absorbed hit or a hull with no locators), and that a weapon
//! hit landing in `Race::tick` reaches it.

use super::*;
use crate::race::hit_sparks::{HIT_SPARK_EFFECT, LEACHBEAM_HIT_SPARK_EFFECT};
use oag_gameplay::PlayerInputs;
use oag_livery::SparkAnchor;
use oag_weapons::projectile::WeaponHit;

/// Six locators, Assegai's count, a unit apart along the hull.
fn six_locators() -> Vec<SparkAnchor> {
    (0..6)
        .map(|i| SparkAnchor {
            position: Vec3::new(i as f32 - 2.5, 0.0, 0.0),
            up: Vec3::Y,
        })
        .collect()
}

/// A race whose slot 0 carries `locators` and whose two hit-spark effects
/// are one-emitter stand-ins.
fn race_with_locators(locators: Vec<SparkAnchor>) -> Race {
    let mut handling = hulled_handling();
    handling.dimensions.shield = 100.0;
    let mut setup = setup(handling);
    for name in [HIT_SPARK_EFFECT, LEACHBEAM_HIT_SPARK_EFFECT] {
        let blob = super::respawn::one_emitter_pob(name, 0);
        let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
            .expect("the hand-laid effect parses");
        setup.effects.insert(name, effect);
    }
    setup.hit_spark_anchors = vec![locators];
    Race::start(setup)
}

fn landed_on_player() -> [WeaponHit; MAX_SHIPS] {
    let mut hits = [WeaponHit::default(); MAX_SHIPS];
    hits[0].landed = true;
    hits
}

/// One landed hit: one or two locators fire (two draws, the second skipped
/// when it repeats the first), and neither fires again inside 0.8 s.
#[test]
fn a_landed_hit_sparks_one_or_two_locators_and_they_rest_for_0_8_s() {
    let mut race = race_with_locators(six_locators());
    race.throw_hit_sparks(&landed_on_player(), false);
    let first = race.hit_sparks_started_for_tests();
    assert!(
        (1..=2).contains(&first),
        "{first} sparks from one hit, not one or two"
    );

    // Every locator hit on every tick for a little under the gate: no
    // locator may fire twice, so at most six can have started.
    let ticks = (oag_fx::sparks::COLLISION_COOLDOWN / race.dt()) as usize - 2;
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
        race.throw_hit_sparks(&landed_on_player(), false);
    }
    assert!(
        race.hit_sparks_started_for_tests() <= 6,
        "a locator fired twice inside its 0.8 s gate: {} started",
        race.hit_sparks_started_for_tests()
    );

    // Past the gate, the locators are free again.
    let before = race.hit_sparks_started_for_tests();
    for _ in 0..4 {
        race.tick(&PlayerInputs::none());
    }
    race.throw_hit_sparks(&landed_on_player(), false);
    assert!(race.hit_sparks_started_for_tests() > before);
}

/// Nothing without a hit that landed, and nothing on a hull with no
/// locators - `Ship_Damage` tests the node count first and never falls back
/// to the craft's centre.
#[test]
fn an_absorbed_hit_or_a_bare_hull_throws_nothing() {
    let mut race = race_with_locators(six_locators());
    let mut hits = [WeaponHit::default(); MAX_SHIPS];
    hits[0].absorbed = true;
    race.throw_hit_sparks(&hits, false);
    assert_eq!(race.hit_sparks_started_for_tests(), 0);

    let mut bare = race_with_locators(Vec::new());
    bare.throw_hit_sparks(&landed_on_player(), true);
    assert_eq!(bare.hit_sparks_started_for_tests(), 0);
}

/// A spark rides the hull while it emits and is handed back to the stage
/// once it stops, so a long fight does not hold every stage slot.
#[test]
fn a_hit_spark_is_let_go_once_it_stops_emitting() {
    let mut race = race_with_locators(six_locators());
    race.throw_hit_sparks(&landed_on_player(), true);
    assert!(race.hit_sparks_started_for_tests() > 0);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        race.view.hit_sparks.riding_len() == 0,
        "a finished spark still holds its stage slot"
    );
}

/// The wiring, end to end: a Rocket that strikes the player in `Race::tick`
/// lands through `Ship_Damage`'s gate and throws the hull's sparks. The
/// weapon's own `WO_ROCKET_EXPLO` is a separate call and is not what is
/// counted here.
#[test]
fn a_rocket_striking_the_player_in_the_tick_throws_its_hull_sparks() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    race.view.hit_sparks = crate::race::hit_sparks::HitSparks::new(vec![six_locators()]);
    let blob = super::respawn::one_emitter_pob(HIT_SPARK_EFFECT, 0);
    let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    race.view.effects.insert(HIT_SPARK_EFFECT, effect);
    // Past the countdown, so the craft is racing and the gate admits the hit.
    for _ in 0..oag_race::COUNTDOWN_TICKS + 2 {
        race.tick(&PlayerInputs::none());
    }
    let body = race.sim.world.ships[0].physics.body;
    let before = race.sim.world.ships[0].physics.shield;
    race.sim.world.projectiles.spawn(
        oag_tables::weapons::Weapon::Rocket,
        body.position - body.forward() * 3.0,
        body.forward() * 600.0,
        1,
    );
    for _ in 0..3 {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        race.sim.world.ships[0].physics.shield < before,
        "the rocket never struck the player - not what this test means to check"
    );
    assert!(race.hit_sparks_started_for_tests() > 0);
}

/// A landed weapon hit shakes the local player's camera, and only theirs:
/// `Ship_Damage`'s `Camera_ArmShake` sits inside its `+0x368 == 0` block.
#[test]
fn a_weapon_hit_shakes_the_players_camera_and_nobody_elses() {
    let mut race = race_with_locators(six_locators());
    let mut on_rival = [WeaponHit::default(); MAX_SHIPS];
    on_rival[3].landed = true;
    race.throw_hit_sparks(&on_rival, false);
    assert!(!race.view.shake.active(), "a rival's hit shook our camera");

    race.throw_hit_sparks(&landed_on_player(), false);
    assert!(race.view.shake.active());
    // Magnitude 0.6, as authored: the wall path's `severity * 0.3` at 2.0.
    assert!((race.view.shake.magnitude() - 0.6).abs() < 1e-5);
}
