//! A frictionless record throws the one-shot no-damage spark and nothing else
//! (`docs/ghidra/functions/ps3-hdfury-eu/collision-cues.md`, "which records carry no friction").

use super::respawn::one_emitter_pob;
use super::*;
use crate::floor_sparks::{ContactKind, contact_kind};
use oag_gameplay::PlayerInputs;
use oag_livery::SparkAnchor;
use oag_physics::wall::{FrictionlessRecord, WallResponse};
use oag_pob::flags;

fn locators() -> Vec<SparkAnchor> {
    (0..6)
        .map(|i| SparkAnchor {
            position: Vec3::new(i as f32 - 2.5, 0.0, 0.0),
            up: Vec3::Y,
        })
        .collect()
}

/// A race whose craft carry `anchors` (empty: Pulse's shape) and whose no-damage spark is a
/// one-emitter stand-in.
fn race(anchors: Vec<Vec<SparkAnchor>>, colliders: Vec<oag_physics::TriangleSoup>) -> Race {
    let mut setup = setup_with(hulled_handling(), colliders);
    setup.mode = Mode::SingleRace;
    let mut view = crate::tests::fixture_view();
    let blob = one_emitter_pob(
        super::respawn::trigger_name(Trigger::CollisionSparkNoDamage),
        0,
    );
    let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    view.handles.insert(Trigger::CollisionSparkNoDamage, effect);
    view.weapon_spark_anchors = anchors;
    Race::start(setup, view, crate::extensions::Prepared::default())
}

/// An HD-shaped race with the seat off the grid.
fn hd_race() -> Race {
    let mut race = race(vec![locators(); MAX_SHIPS], Vec::new());
    race.core.sim.world.ships[0].physics.on_grid = false;
    race
}

/// A wall response holding `points` as frictionless records.
fn floor_records(points: &[Vec3]) -> WallResponse {
    let mut wall = WallResponse::default();
    for (record, point) in wall.frictionless.iter_mut().zip(points) {
        *record = FrictionlessRecord {
            point: *point,
            impulse: 3.0,
        };
    }
    wall.floor_contacts = points.len() as u32;
    wall
}

fn note(race: &mut Race, wall: &WallResponse, reset: Option<Vec3>) {
    let anchors = race.view.hit_sparks.dispatch_anchors(0);
    race.view
        .floor_sparks
        .note(0, &race.core.sim.world.ships[0], wall, reset, anchors);
}

/// Notes `wall` on the seat's craft and runs the throw, once.
fn throw(race: &mut Race, wall: &WallResponse) {
    note(race, wall, None);
    race.view.advance_floor_sparks(&race.core);
}

/// **The kind rule**: no friction is kind 2, a craft that is not racing is kind 3 whatever the
/// friction, and a NaN friction keeps kind 0.
#[test]
fn the_kind_follows_the_records_friction_and_the_crafts_state() {
    assert_eq!(contact_kind(0.0, true), ContactKind::NoDamage);
    assert_eq!(contact_kind(-1.0, true), ContactKind::NoDamage);
    assert_eq!(contact_kind(0.035, true), ContactKind::Attached);
    assert_eq!(contact_kind(f32::NAN, true), ContactKind::Attached);
    assert_eq!(contact_kind(0.0, false), ContactKind::NotRacing);
    assert_eq!(contact_kind(0.035, false), ContactKind::NotRacing);
}

/// **A floor record throws the one-shot, one spark per record per tick, with no cooldown.**
#[test]
fn a_floor_record_throws_one_spark_per_record_every_tick() {
    let mut race = hd_race();
    let at = race.core.sim.world.ships[0].physics.body.position;
    let wall = floor_records(&[at, at + Vec3::new(1.0, 0.0, 0.0)]);
    throw(&mut race, &wall);
    assert_eq!(race.floor_sparks_started_for_tests(), 2, "one per record");
    race.view
        .stage
        .advance(1.0 / 60.0, &mut race.view.stage_rng);
    assert!(race.view.stage.alive_count() > 0, "nothing is alive");
    throw(&mut race, &wall);
    assert_eq!(
        race.floor_sparks_started_for_tests(),
        4,
        "the next tick fires again: there is no gate"
    );
}

/// **A wall and a pair keep the attached spark: a response with no frictionless record throws
/// no one-shot.**
#[test]
fn a_wall_response_throws_no_one_shot() {
    let mut race = hd_race();
    throw(&mut race, &WallResponse::default());
    assert_eq!(race.floor_sparks_started_for_tests(), 0);
}

/// **The reset plane's point is a record too.**
#[test]
fn a_reset_plane_is_a_record() {
    let mut race = hd_race();
    let at = race.core.sim.world.ships[0].physics.body.position;
    note(&mut race, &WallResponse::default(), Some(at));
    race.view.advance_floor_sparks(&race.core);
    assert_eq!(race.floor_sparks_started_for_tests(), 1);
}

/// **A reset plane's record teleports the craft the same tick; the spark is still thrown, at the
/// impact.** Red when the placement and the reach test read the pose after the respawn.
#[test]
fn a_reset_record_sparks_at_the_impact_not_the_respawn() {
    let mut race = hd_race();
    let at = race.core.sim.world.ships[0].physics.body.position;
    note(&mut race, &WallResponse::default(), Some(at));
    race.core.sim.world.ships[0].physics.body.position = Vec3::new(0.0, 0.0, 9000.0);
    race.view.advance_floor_sparks(&race.core);
    assert_eq!(
        race.floor_sparks_started_for_tests(),
        1,
        "the reach test read the respawn pose"
    );
}

/// **The countdown is state 0: a floor record on the grid is kind 3, no one-shot.**
#[test]
fn a_floor_record_on_the_grid_is_not_the_one_shot() {
    let mut race = hd_race();
    race.core.sim.world.ships[0].physics.on_grid = true;
    let wall = floor_records(&[race.core.sim.world.ships[0].physics.body.position]);
    throw(&mut race, &wall);
    assert_eq!(race.floor_sparks_started_for_tests(), 0);
}

/// **Pulse's shape (no dispatch locators) throws nothing**, and neither does a craft under a
/// Shield or one that is not racing.
#[test]
fn only_a_racing_unshielded_craft_on_an_hd_lineage_title_throws() {
    let wall_at =
        |race: &Race| floor_records(&[race.core.sim.world.ships[0].physics.body.position]);
    let mut pulse = race(Vec::new(), Vec::new());
    let wall = wall_at(&pulse);
    throw(&mut pulse, &wall);
    assert_eq!(pulse.floor_sparks_started_for_tests(), 0, "Pulse sparked");

    let mut shielded = hd_race();
    shielded.core.sim.world.ships[0].physics.shield_pickup_timer = 5.0;
    let wall = wall_at(&shielded);
    throw(&mut shielded, &wall);
    assert_eq!(
        shielded.floor_sparks_started_for_tests(),
        0,
        "a Shield sparked"
    );

    let mut wreck = hd_race();
    wreck.core.sim.world.ships[0].physics.craft_state = oag_physics::CraftState::Eliminated;
    let wall = wall_at(&wreck);
    throw(&mut wreck, &wall);
    assert_eq!(wreck.floor_sparks_started_for_tests(), 0, "a wreck sparked");
}

/// **A craft out of every view's reach throws nothing.**
#[test]
fn a_craft_beyond_a_hundred_units_of_every_eye_throws_nothing() {
    let mut race = hd_race();
    race.core.sim.world.ships[0].physics.body.position = Vec3::new(0.0, 0.0, 5000.0);
    let wall = floor_records(&[Vec3::new(0.0, 0.0, 5000.0)]);
    throw(&mut race, &wall);
    assert_eq!(race.floor_sparks_started_for_tests(), 0);
}

/// **A craft pressed into a real `Floor` collider (the mag floor's records are the physics
/// crate's, `a_floor_inside_the_hull_pushes_it_out_and_drives_no_reaction`), ticked through `Race::tick`, throws the
/// spark; the same craft against a `Wall` throws none.** Red when the physics stops recording
/// the floor or the tick stops handing it over.
#[test]
fn a_craft_in_a_floor_sparks_and_in_a_wall_does_not() {
    let started = |surface| {
        let mut race = race(
            vec![locators(); MAX_SHIPS],
            vec![upward_plane(-0.3, surface)],
        );
        let mut on_the_grid = 0;
        let mut first_alive = None;
        for tick in 0..330 {
            race.core.sim.world.ships[0].physics.body.position = Vec3::ZERO;
            race.tick(&PlayerInputs::none());
            if first_alive.is_none() && race.floor_sparks_started_for_tests() > 0 {
                first_alive = Some(race.view.stage.alive_count());
            }
            if tick == 200 {
                on_the_grid = race.floor_sparks_started_for_tests();
            }
        }
        assert_eq!(
            on_the_grid, 0,
            "{surface:?}: a record on the grid threw the one-shot"
        );
        if surface == oag_physics::Surface::Floor {
            assert!(
                first_alive.is_some_and(|alive| alive > 0),
                "the spark drew nothing on the tick it started: {first_alive:?}"
            );
        }
        race.floor_sparks_started_for_tests()
    };
    assert!(
        started(oag_physics::Surface::Floor) > 0,
        "a floor did not spark"
    );
    assert_eq!(
        started(oag_physics::Surface::Wall),
        0,
        "a wall threw the one-shot"
    );
}

/// **A frame of only frictionless records keeps a running attached spark alive**, where the
/// effect is authored looping: the byte that holds it is set after every kind.
#[test]
fn a_floor_only_frame_keeps_the_attached_spark_alive() {
    use crate::contact_fx::{Attach, attach_rule};
    // The call sites pass `resolved.is_some() || floor_contacts > 0`; a frame with only floor
    // records is `touching` and holds, where it used to release.
    let wall = floor_records(&[Vec3::ZERO]);
    let touching = wall.resolved.is_some() || wall.floor_contacts > 0;
    assert_eq!(
        attach_rule(true, false, false, touching, false),
        Attach::Hold
    );
    assert_eq!(
        attach_rule(true, false, false, false, false),
        Attach::Release
    );

    let mut setup = setup_with(hulled_handling(), Vec::new());
    setup.mode = Mode::SingleRace;
    let mut view = crate::tests::fixture_view();
    let effect = oag_fx::psys::Effect::parse(
        &one_emitter_pob("WO_TEST_SPARK", flags::LOOPING),
        oag_fx::psys::ColourScale::Full,
    )
    .expect("the hand-laid effect parses");
    view.handles.insert(Trigger::CollisionSpark, effect);
    let mut race = Race::start(setup, view, crate::extensions::Prepared::default());
    race.view.contact.attached = true;
    let evaluated = oag_physics::Evaluated {
        wall: floor_records(&[Vec3::ZERO]),
        ..oag_physics::Evaluated::idle(&race.core.sim.world.ships[0].physics)
    };
    race.view.advance_contact_fx(&race.core, 0, &evaluated);
    assert!(
        race.view.contact.attached,
        "a floor-only frame released the spark"
    );
    let quiet = oag_physics::Evaluated::idle(&race.core.sim.world.ships[0].physics);
    race.view.advance_contact_fx(&race.core, 0, &quiet);
    assert!(
        !race.view.contact.attached,
        "a frame with no record kept it"
    );
}

/// **Kind 3: a craft that is not racing restarts the attached spark from a floor record**, where
/// a racing one throws the one-shot and leaves the attached spark alone.
#[test]
fn a_floor_record_of_a_craft_that_is_not_racing_restarts_the_attached_spark() {
    let mut setup = setup_with(hulled_handling(), Vec::new());
    setup.mode = Mode::SingleRace;
    let mut view = crate::tests::fixture_view();
    let effect = oag_fx::psys::Effect::parse(
        &one_emitter_pob("WO_TEST_SPARK", flags::LOOPING),
        oag_fx::psys::ColourScale::Full,
    )
    .expect("the hand-laid effect parses");
    view.handles.insert(Trigger::CollisionSpark, effect);
    let mut race = Race::start(setup, view, crate::extensions::Prepared::default());
    race.core.sim.world.ships[0].physics.on_grid = false;
    let evaluated = oag_physics::Evaluated {
        wall: floor_records(&[Vec3::ZERO]),
        ..oag_physics::Evaluated::idle(&race.core.sim.world.ships[0].physics)
    };
    race.view.advance_contact_fx(&race.core, 0, &evaluated);
    assert!(
        !race.view.contact.attached,
        "a racing craft's floor record restarted it"
    );

    race.core.sim.world.ships[0].physics.craft_state = oag_physics::CraftState::Eliminated;
    race.core.sim.world.ships[0].physics.state_timer = 0.0;
    race.view.advance_contact_fx(&race.core, 0, &evaluated);
    assert!(!race.view.contact.attached, "a state 6 wreck sparked");

    race.core.sim.world.ships[0].physics.state_timer = 1.0;
    race.view.advance_contact_fx(&race.core, 0, &evaluated);
    assert!(
        race.view.contact.attached,
        "kind 3 did not restart the attached spark"
    );
}
