//! HD's attached collision spark on the whole field: every craft a local view sees within 100
//! units throws it from a wall or a pair contact, at the original's strength law, and never
//! under a Shield (`ship-collision-fx.md`, "Every craft a view can see within 100 units").

use super::respawn::one_emitter_pob;
use super::*;
use oag_gameplay::PlayerInputs;
use oag_pob::flags;

/// A three-craft race against a wall whose collision spark is `blob`.
fn field_race(blob: &[u8]) -> Race {
    let mut setup = setup_with(
        hulled_handling(),
        vec![upward_plane(-40.0, oag_physics::Surface::Wall)],
    );
    let mut view = crate::tests::fixture_view();
    let effect = oag_fx::psys::Effect::parse(blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    view.handles.insert(Trigger::CollisionSpark, effect);
    setup.mode = Mode::SingleRace;
    let mut race = Race::start(setup, view, crate::extensions::Prepared::default());
    race.core.sim.world.ship_count = 3;
    for slot in 1..3 {
        race.core.sim.world.ships[slot].active = true;
        race.core.sim.world.ships[slot].handling = hulled_handling();
    }
    race
}

/// The seat resting where it is, and the view's eye after a tick has settled the camera.
fn settled_eye(race: &mut Race) -> Vec3 {
    race.core.sim.world.ships[0].physics.body.position = Vec3::new(20.0, 0.0, 0.0);
    race.core.sim.world.ships[0].physics.body.linear_velocity = Vec3::ZERO;
    race.core.sim.world.ships[1].physics.body.position = Vec3::new(20.0, 0.0, 900.0);
    race.core.sim.world.ships[2].physics.body.position = Vec3::new(20.0, 0.0, 950.0);
    race.tick(&PlayerInputs::none());
    race.view.camera_frame(&race.core).position
}

/// Slots 1 and 2 touching head on at `centre`, 20 units a second each.
fn collide(race: &mut Race, centre: Vec3) {
    let a = &mut race.core.sim.world.ships[1].physics;
    a.body.position = centre;
    a.body.linear_velocity = Vec3::new(0.0, 0.0, 20.0);
    let b = &mut race.core.sim.world.ships[2].physics;
    b.body.position = centre + Vec3::new(0.0, 0.0, 1.0);
    b.body.linear_velocity = Vec3::new(0.0, 0.0, -20.0);
}

fn looping() -> Vec<u8> {
    one_emitter_pob("WO_TEST_SPARK", flags::LOOPING)
}

fn held(race: &Race, slot: usize) -> bool {
    race.view
        .field_sparks
        .strength_of(&race.view.stage, slot)
        .is_some()
}

/// **An opponent's pair contact within 100 units of the view starts its own attached spark,
/// at the law's strength, and one past 100 units does not.** Red on a tree that throws the
/// spark for the seat's craft alone.
#[test]
fn an_opponent_pair_contact_within_a_hundred_units_of_the_view_throws_the_spark() {
    let mut race = field_race(&looping());
    let eye = settled_eye(&mut race);
    collide(&mut race, eye + Vec3::new(0.0, 0.0, 60.0));
    race.tick(&PlayerInputs::none());
    for slot in [1, 2] {
        assert!(held(&race, slot), "slot {slot} threw no spark inside 100");
    }
    let strength = race
        .view
        .field_sparks
        .strength_of(&race.view.stage, 1)
        .unwrap();
    assert!(
        strength > 0.1,
        "a closing pair is above the cutoff, got {strength}"
    );
    assert!(strength <= 2.0);

    let mut far = field_race(&looping());
    let eye = settled_eye(&mut far);
    collide(&mut far, eye + Vec3::new(0.0, 0.0, 140.0));
    far.tick(&PlayerInputs::none());
    assert!(
        !held(&far, 1) && !held(&far, 2),
        "a pair 140 units from the eye threw a spark"
    );
}

/// **A shielded craft throws nothing; its unshielded partner still does.**
#[test]
fn a_shielded_opponent_throws_no_spark() {
    let mut race = field_race(&looping());
    let eye = settled_eye(&mut race);
    race.core.sim.world.ships[1].physics.shield_pickup_timer = 5.0;
    collide(&mut race, eye + Vec3::new(0.0, 0.0, 60.0));
    race.tick(&PlayerInputs::none());
    assert!(!held(&race, 1), "a Shield did not stop the spark");
    assert!(held(&race, 2), "the unshielded partner threw nothing");
}

/// **The spark is let go the first tick nothing asks for it, and the slot goes back.**
#[test]
fn an_opponent_spark_ends_with_its_contact() {
    let mut race = field_race(&looping());
    let eye = settled_eye(&mut race);
    collide(&mut race, eye + Vec3::new(0.0, 0.0, 60.0));
    race.tick(&PlayerInputs::none());
    assert!(held(&race, 1));
    let a = &mut race.core.sim.world.ships[1].physics;
    a.body.position = eye + Vec3::new(0.0, 0.0, 60.0);
    a.body.linear_velocity = Vec3::ZERO;
    let b = &mut race.core.sim.world.ships[2].physics;
    b.body.position = eye + Vec3::new(0.0, 0.0, 400.0);
    b.body.linear_velocity = Vec3::ZERO;
    race.tick(&PlayerInputs::none());
    assert!(!held(&race, 1), "the spark outlived the contact");
}

/// **A burst title (Pulse's shape: the effect is not looping) holds no opponent spark and the
/// seat keeps `severity`'s law.** The unchanged-Pulse control.
#[test]
fn a_burst_effect_leaves_the_opponents_and_the_seats_law_alone() {
    let mut race = field_race(&one_emitter_pob("WO_TEST_SPARK", 0));
    let eye = settled_eye(&mut race);
    collide(&mut race, eye + Vec3::new(0.0, 0.0, 60.0));
    race.tick(&PlayerInputs::none());
    assert!(!held(&race, 1) && !held(&race, 2));
    assert_eq!(race.view.stage.playing_count(), 0);

    let setup = setup_with(
        hulled_handling(),
        vec![upward_plane(-40.0, oag_physics::Surface::Wall)],
    );
    let mut view = crate::tests::fixture_view();
    let blob = one_emitter_pob("WO_TEST_SPARK", 0);
    let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full).unwrap();
    view.handles.insert(Trigger::CollisionSpark, effect);
    let mut seat = Race::start(setup, view, crate::extensions::Prepared::default());
    super::respawn::push_toward_wall(&mut seat);
    let evaluated = seat.tick(&PlayerInputs::none());
    assert!(evaluated.wall.impact);
    assert_eq!(
        seat.sparks().scale(),
        oag_fx::sparks::severity(evaluated.wall.impact_speed)
    );
}

/// **The seat's attached spark follows HD's law, not Pulse's**: its speed term and the wall
/// record's impulse, with the craft's velocity carried.
#[test]
fn the_seats_attached_spark_plays_at_the_attached_strength() {
    let setup = setup_with(
        hulled_handling(),
        vec![upward_plane(-40.0, oag_physics::Surface::Wall)],
    );
    let mut view = crate::tests::fixture_view();
    let effect = oag_fx::psys::Effect::parse(&looping(), oag_fx::psys::ColourScale::Full).unwrap();
    view.handles.insert(Trigger::CollisionSpark, effect);
    let mut race = Race::start(setup, view, crate::extensions::Prepared::default());
    super::respawn::push_toward_wall(&mut race);
    let evaluated = race.tick(&PlayerInputs::none());
    assert!(evaluated.wall.impact);
    let speed = race.core.sim.world.ships[0]
        .physics
        .body
        .linear_velocity
        .length();
    let law = oag_fx::sparks::attached_strength(speed, evaluated.wall.impulse_largest);
    assert_eq!(race.sparks().scale(), law);
    assert_ne!(law, oag_fx::sparks::severity(evaluated.wall.impact_speed));
}

/// **An opponent's wall contact within 100 units of the view starts its own spark, one past 100
/// units does not.**
#[test]
fn an_opponent_wall_contact_throws_the_spark_inside_a_hundred_units_only() {
    let drive_into_wall = |race: &mut Race, at: Vec3| {
        let body = &mut race.core.sim.world.ships[1].physics.body;
        body.position = Vec3::new(at.x, -39.7, at.z);
        body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
        body.orientation = oag_core::math::Quat::IDENTITY;
        body.angular_velocity = Vec3::ZERO;
        race.tick(&PlayerInputs::none());
    };
    let mut near = field_race(&looping());
    let eye = settled_eye(&mut near);
    drive_into_wall(&mut near, eye + Vec3::new(0.0, 0.0, 60.0));
    assert!(
        near.view.field_sparks.walls[1].is_some_and(|wall| wall.impact),
        "the fixture never reached the wall"
    );
    assert!(held(&near, 1), "a wall contact in reach threw no spark");

    let mut far = field_race(&looping());
    let eye = settled_eye(&mut far);
    drive_into_wall(&mut far, eye + Vec3::new(0.0, 0.0, 140.0));
    assert!(
        far.view.field_sparks.walls[1].is_some_and(|wall| wall.impact),
        "the fixture never reached the wall"
    );
    assert!(!held(&far, 1), "a wall contact past 100 units threw one");
}
