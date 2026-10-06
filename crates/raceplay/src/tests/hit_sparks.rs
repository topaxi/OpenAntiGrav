//! The struck hull's damage sparks - see `race::hit_sparks`.
//!
//! Hand-laid effect and locators, no disc: what is asserted is the trigger's
//! own rules (one or two locators per landed hit, a 0.8 s gate per locator,
//! nothing on an absorbed hit or a hull with no locators), and that a weapon
//! hit landing in `Race::tick` reaches it.

use super::*;
use oag_gameplay::PlayerInputs;
use oag_livery::SparkAnchor;
use oag_title::Trigger::{HitSpark, LeachHitSpark};
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
    for trigger in [HitSpark, LeachHitSpark] {
        let blob = super::respawn::one_emitter_pob(super::respawn::trigger_name(trigger), 0);
        let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
            .expect("the hand-laid effect parses");
        setup.handles.insert(trigger, effect);
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
    race.view.hit_sparks = crate::hit_sparks::HitSparks::new(vec![six_locators()]);
    let blob =
        super::respawn::one_emitter_pob(crate::tests::respawn::trigger_name(Trigger::HitSpark), 0);
    let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    race.view.handles.insert(Trigger::HitSpark, effect);
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

/// A Cannon round striking a second craft, ticked through `Race::tick`, with
/// `weapon_anchors` as the title built them: HD's craft-hit spark count.
fn cannon_strikes_a_craft(weapon_anchors: Vec<Vec<SparkAnchor>>) -> u32 {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.weapons = Some(one_cannon_table());
    let blob = super::respawn::one_emitter_pob(
        crate::tests::respawn::trigger_name(Trigger::WeaponSpark),
        0,
    );
    let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    setup.handles.insert(Trigger::WeaponSpark, effect);
    setup.weapon_spark_anchors = weapon_anchors;
    let mut race = Race::start(setup);
    race.sim.world.ship_count = 2;
    race.sim.world.ships[1].active = true;
    race.sim.world.ships[1].handling = hulled_handling();
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Cannon);
    let mut buttons = Buttons::new();
    for _ in 0..60 {
        race.sim.world.ships[0].physics.body.position = Vec3::ZERO;
        let forward = race.sim.world.ships[0].physics.body.forward();
        race.sim.world.ships[1].physics.body.position = forward * 15.0;
        race.sim.world.ships[1].physics.body.linear_velocity = Vec3::ZERO;
        let snapshot = buttons.tick(SQUARE);
        race.tick(&PlayerInputs::single(snapshot));
    }
    race.weapon_sparks_started_for_tests()
}

/// HD's Cannon throws `WO_SHIP_SPARK_DAMAGE_WEAPON` on the craft it strikes
/// (`Cannon_ApplyCraftHit`, `Ship_DispatchCollisionFx` kind 1); a title that
/// builds no weapon anchors - Pulse, whose Cannon spawns nothing on a craft -
/// throws none.
#[test]
fn hds_cannon_sparks_the_struck_craft_and_pulses_does_not() {
    // Two craft slots: the struck one is slot 1.
    let locators = || vec![six_locators(), six_locators()];
    assert!(
        cannon_strikes_a_craft(locators()) > 0,
        "an HD Cannon round struck a craft and threw no weapon spark"
    );
    assert_eq!(
        cannon_strikes_a_craft(Vec::new()),
        0,
        "a title with no weapon anchors threw a weapon spark"
    );
}

/// The burst sits on the locator nearest the contact, not on a random one.
#[test]
fn the_weapon_spark_picks_the_locator_nearest_the_contact() {
    let model = Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0));
    let at = crate::hit_sparks::nearest_locator(&six_locators(), model, Vec3::new(11.4, 0.2, 0.0))
        .expect("six locators");
    // The locator at x = 1.5 (model space) is the nearest of the six to 1.4.
    assert!(at.distance(Vec3::new(11.5, 0.0, 0.0)) < 1e-6, "{at:?}");
    assert!(crate::hit_sparks::nearest_locator(&[], model, Vec3::ZERO).is_none());
}

/// The title's `WeaponSpark` entry is the only thing keeping the other titles
/// from firing HD's Cannon spark: present for HD and for no other title this
/// workspace ships.
#[test]
fn only_hd_throws_the_cannon_craft_hit_spark() {
    use crate::hit_sparks::throws_weapon_spark;
    assert!(throws_weapon_spark(oag_hd::TITLE));
    for other in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_2048::TITLE,
        oag_omega::TITLE,
    ] {
        assert!(!throws_weapon_spark(other), "{} fires it", other.name);
    }
    let liveries: Vec<oag_livery::Livery> = Vec::new();
    assert!(crate::hit_sparks::weapon_anchors(oag_omega::TITLE, &liveries).is_empty());
}
