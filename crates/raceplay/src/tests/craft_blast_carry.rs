//! A Rocket's craft blast carries the struck craft's velocity on the titles whose spawn hands
//! it over (`docs/ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md`).

use super::*;
use oag_title::{Origin, Trigger};

/// HD's, 2048's and Omega's spawns write the struck craft's velocity into the blast, each read
/// in its own executable; Pulse's PSP spawn writes none, and Pure's is unread.
#[test]
fn only_hd_2048_and_omega_carry_the_craft_blast() {
    for title in [oag_hd::TITLE, oag_2048::TITLE, oag_omega::TITLE] {
        assert_eq!(
            title.effects.craft_blast_carry,
            Some(Origin::Measured),
            "{}",
            title.name
        );
    }
    for title in [oag_pulse::TITLE, oag_pure::TITLE] {
        assert_eq!(title.effects.craft_blast_carry, None, "{}", title.name);
    }
}

/// The struck craft's velocity, for the craft blast alone, and only while that craft is live.
#[test]
fn the_craft_blast_takes_the_struck_crafts_velocity() {
    let mut race = race_with_a_grid();
    let velocity = Vec3::new(120.0, 0.0, -15.0);
    race.core.sim.world.ships[1].physics.body.linear_velocity = velocity;
    let carried =
        |race: &Race, trigger, struck| race.view.craft_blast_velocity(&race.core, trigger, struck);

    race.view.craft_blast_carry = false;
    assert_eq!(
        carried(&race, Trigger::CraftBlast, Some(1)),
        None,
        "carry off"
    );

    race.view.craft_blast_carry = true;
    assert_eq!(carried(&race, Trigger::CraftBlast, Some(1)), Some(velocity));
    assert_eq!(
        carried(&race, Trigger::TrackBlast, None),
        None,
        "a wall hit"
    );
    assert_eq!(
        carried(&race, Trigger::CraftBlast, None),
        None,
        "nothing struck"
    );
    race.core.sim.world.ships[1].active = false;
    assert_eq!(
        carried(&race, Trigger::CraftBlast, Some(1)),
        None,
        "an inactive craft"
    );
}

/// `--force-rocket-hit-velocity`: the struck craft takes the velocity, and its craft blast carries
/// exactly it even after the craft's own has changed again, as it does in the tick that lands the
/// hit. Other craft keep their own, and a craft that is not racing is left alone.
#[test]
fn a_forced_velocity_is_what_the_craft_blast_takes() {
    let mut race = race_with_a_grid();
    race.view.craft_blast_carry = true;
    let velocity = Vec3::new(-3.0, 40.0, 7.0);
    let carried = |race: &Race, slot| {
        race.view
            .craft_blast_velocity(&race.core, Trigger::CraftBlast, Some(slot))
    };
    race.force_struck_velocity(1, velocity);
    assert_eq!(
        race.core.sim.world.ships[1].physics.body.linear_velocity,
        velocity
    );
    race.core.sim.world.ships[1].physics.body.linear_velocity = Vec3::new(1.0, 2.0, 3.0);
    assert_eq!(carried(&race, 1), Some(velocity));
    let other = Vec3::new(9.0, 0.0, 0.0);
    race.core.sim.world.ships[2].physics.body.linear_velocity = other;
    assert_eq!(carried(&race, 2), Some(other));

    race.core.sim.world.ships[2].active = false;
    race.force_struck_velocity(2, velocity);
    assert_eq!(
        race.core.sim.world.ships[2].physics.body.linear_velocity,
        other
    );
    assert_eq!(carried(&race, 1), Some(velocity), "slot 1's is kept");
}

/// A field past its countdown with a Rocket table, a hand-laid craft blast whose particles take
/// all of the velocity they are handed, and the player's Rockets just fired.
fn race_with_rockets_in_flight() -> Race {
    let mut race = race_with_a_grid();
    race.core.sim.weapons = Some(std::sync::Arc::new(one_rocket_table()));
    let mut blob =
        super::respawn::one_emitter_pob(super::respawn::trigger_name(Trigger::CraftBlast), 0);
    let inherit = blob.len() - oag_pob::EMITTER_LEN + 0x4d0;
    blob[inherit..inherit + 4].copy_from_slice(&1.0f32.to_le_bytes());
    let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    race.view.handles.insert(Trigger::CraftBlast, effect);
    for _ in 0..oag_race::COUNTDOWN_TICKS {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    race.core.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Rocket);
    let mut buttons = Buttons::new();
    race.tick(&oag_gameplay::PlayerInputs::single(buttons.tick(SQUARE)));
    assert_eq!(rockets_in_the_air(&race, 0), 3, "the volley was fired");
    race
}

fn rockets_in_the_air(race: &Race, owner: u8) -> usize {
    race.core
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(oag_tables::weapons::Weapon::Rocket) && p.owner == owner)
        .count()
}

fn craft_blast(race: &Race) -> std::sync::Arc<oag_fx::psys::Effect> {
    race.effect_for_tests(Trigger::CraftBlast).expect("loaded")
}

fn blasts(race: &Race) -> (Vec<(Vec3, Vec3)>, usize) {
    let effect = craft_blast(race);
    (
        race.view.stage.frames_of(&effect),
        race.view.stage.alive_of_effect(&effect),
    )
}

/// `--force-rocket-hit-at` played at the end of the tick a `--force-rocket-hit` puts the craft on
/// its Rocket draws what that real hit draws on the next one: as many blasts at the same points,
/// at the same age, and as many Rockets gone. No craft is damaged.
#[test]
fn a_forced_blast_at_a_point_matches_the_real_hit_it_stands_for() {
    let mut real = race_with_rockets_in_flight();
    assert!(real.core.sim.force_rocket_hit(1), "a Rocket is in flight");
    let mut forced = race_with_rockets_in_flight();
    let shield = forced.core.sim.world.ships[1].physics.shield;
    real.tick(&oag_gameplay::PlayerInputs::none());
    let (frames, _) = blasts(&real);
    assert!(!frames.is_empty(), "the real hit drew no craft blast");
    let point = frames[0].0;
    assert!(frames.iter().all(|(at, _)| *at == point), "{frames:?}");

    assert_eq!(
        forced.force_rocket_hit_at(point, frames.len(), None),
        Some(frames.len())
    );
    forced.tick(&oag_gameplay::PlayerInputs::none());
    assert_eq!(blasts(&forced), blasts(&real), "one tick after the hit");
    assert_eq!(rockets_in_the_air(&forced, 0), rockets_in_the_air(&real, 0));
    forced.tick(&oag_gameplay::PlayerInputs::none());
    real.tick(&oag_gameplay::PlayerInputs::none());
    assert_eq!(blasts(&forced), blasts(&real), "two ticks after the hit");
    assert_eq!(forced.core.sim.world.ships[1].physics.shield, shield);
}

/// Each wave stands at its own point with its own count, retires as many of the player's
/// Rockets and nobody else's, and plays its blasts even when fewer Rockets are left.
#[test]
fn each_forced_wave_plays_its_count_at_its_point_and_retires_the_players_rockets() {
    let mut race = race_with_rockets_in_flight();
    race.core.sim.world.projectiles.spawn(
        oag_tables::weapons::Weapon::Rocket,
        Vec3::new(900.0, 0.0, 0.0),
        Vec3::ZERO,
        1,
    );
    let (first, second) = (Vec3::new(40.0, 5.0, -7.0), Vec3::new(52.0, 6.0, -3.0));
    assert_eq!(race.force_rocket_hit_at(first, 1, None), Some(1));
    assert_eq!(rockets_in_the_air(&race, 0), 2);
    assert_eq!(race.force_rocket_hit_at(second, 3, None), Some(2));
    assert_eq!(rockets_in_the_air(&race, 0), 0);
    assert_eq!(
        rockets_in_the_air(&race, 1),
        1,
        "an opponent's Rocket stays"
    );
    let points: Vec<Vec3> = blasts(&race).0.iter().map(|(at, _)| *at).collect();
    assert_eq!(points, [first, second, second, second]);
}

/// The velocity handed to `--force-rocket-hit-at` moves its blast's cloud on a title that
/// carries it, and only there.
#[test]
fn a_forced_blast_carries_the_velocity_only_where_the_title_does() {
    let cloud = |carry: bool, velocity: Option<Vec3>| {
        let mut race = race_with_rockets_in_flight();
        race.view.craft_blast_carry = carry;
        race.force_rocket_hit_at(Vec3::new(40.0, 5.0, -7.0), 1, velocity);
        for _ in 0..3 {
            race.tick(&oag_gameplay::PlayerInputs::none());
        }
        race.view
            .stage
            .centroid_of_effect(&craft_blast(&race))
            .expect("alive")
    };
    let velocity = Some(Vec3::new(120.0, 0.0, 0.0));
    let still = cloud(true, None);
    assert!(cloud(true, velocity).x > still.x + 1.0, "carried");
    assert_eq!(cloud(false, velocity), still, "a title that carries none");
}

/// On HD the blast's cloud travels with the craft it struck while its emitters stay where it
/// was spawned: 20 ticks after a forced hit on a racing rival, the cloud's centre has moved
/// along that craft's heading by a good share of the craft's own travel.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hds_craft_blast_travels_with_the_struck_craft() {
    use oag_gameplay::input::{Button, Input};
    use oag_gameplay::{InputSnapshot, PlayerInputs};

    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..crate::Options::default()
    })
    .expect("loading an HD race");
    let mut race = crate::Race::start(loaded.setup, loaded.view, loaded.modules);
    assert!(race.view.craft_blast_carry, "HD carries the craft blast");
    let held = |before: u32, now: u32| {
        let mut buttons = Input::new();
        buttons.begin_frame(before);
        buttons.begin_frame(now);
        PlayerInputs::single(InputSnapshot {
            buttons,
            ..InputSnapshot::new()
        })
    };
    for _ in 0..oag_race::COUNTDOWN_TICKS + 120 {
        race.tick(&held(0, 0));
    }
    let blast = race
        .effect_for_tests(Trigger::CraftBlast)
        .expect("HD loads the craft blast");
    race.core.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Rocket);
    race.tick(&held(0, Button::Square.bit()));
    assert!(race.core.sim.force_rocket_hit(1), "a Rocket is in flight");
    let mut spawned = None;
    for _ in 0..5 {
        race.tick(&held(0, 0));
        if race.view.stage.alive_of_effect(&blast) > 0 {
            spawned = Some((
                race.view.stage.frames_of(&blast)[0].0,
                race.view.stage.centroid_of_effect(&blast).expect("alive"),
                race.core.sim.world.ships[1].physics.body.position,
            ));
            break;
        }
    }
    let (anchor, cloud_before, craft_before) = spawned.expect("the craft blast never spawned");
    for _ in 0..20 {
        race.tick(&held(0, 0));
    }
    let craft_moved = race.core.sim.world.ships[1].physics.body.position - craft_before;
    assert!(
        craft_moved.length() > 10.0,
        "the struck craft moved {craft_moved:?}: too slow to tell"
    );
    let heading = craft_moved.normalize();
    let cloud_moved = race.view.stage.centroid_of_effect(&blast).expect("alive") - cloud_before;
    let along = cloud_moved.dot(heading);
    assert!(
        along > 0.4 * craft_moved.length(),
        "the cloud moved {along} along a craft that moved {}",
        craft_moved.length()
    );
    assert_eq!(
        race.view.stage.frames_of(&blast)[0].0,
        anchor,
        "the blast is not parented to the craft"
    );
}
