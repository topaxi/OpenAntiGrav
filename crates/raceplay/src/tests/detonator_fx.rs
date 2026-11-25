//! Which of the disc's effects each of Detonator's events plays, and the pose a round is drawn in.

use super::*;
use crate::detonator_fx::{effect_for, round_matrix};
use oag_title::Trigger;
use oag_weapons::detonator::EventKind;
use oag_weapons::detonator::cannon::Round;

#[test]
fn each_event_plays_the_effect_the_original_spawns_for_it() {
    let table = [
        (
            EventKind::MineTouched,
            Some(Trigger::DetonatorMineExploDead),
        ),
        (
            EventKind::MineShot { live: true },
            Some(Trigger::DetonatorMineExplo),
        ),
        (
            EventKind::MineShot { live: false },
            Some(Trigger::DetonatorMineExploDead),
        ),
        (
            EventKind::MineChained { live: true },
            Some(Trigger::DetonatorMineExplo),
        ),
        (
            EventKind::MineChained { live: false },
            Some(Trigger::DetonatorMineExploDead),
        ),
        (EventKind::BombTouched, Some(Trigger::DetonatorBombExplo)),
        (EventKind::BombShot, Some(Trigger::DetonatorBombExplo)),
        (EventKind::BombReplaced, Some(Trigger::DetonatorBombExplo)),
        (
            EventKind::MineEmped { live: true },
            Some(Trigger::DetonatorMineExplo),
        ),
        (
            EventKind::MineEmped { live: false },
            Some(Trigger::DetonatorMineExploDead),
        ),
        (EventKind::BombEmped, Some(Trigger::DetonatorBombExplo)),
        (EventKind::EmpLaunch, None),
        (
            EventKind::RoundWall {
                normal: Vec3::Y,
                spawn: Vec3::ZERO,
                rebound: Vec3::X,
            },
            Some(Trigger::DetonatorCannonSparks),
        ),
        (EventKind::Volley, None),
    ];
    for (kind, want) in table {
        assert_eq!(effect_for(kind), want, "{kind:?}");
    }
}

/// The render queue turns the rows half a turn about up (`0x00677688`, pi): the model's `+Z`, the
/// streak's length, points back along the heading and its `+X` to the round's left-hand side.
#[test]
fn a_round_is_drawn_turned_half_about_up_so_the_streak_trails_it() {
    let round = Round {
        live: true,
        pos: Vec3::new(1.0, 2.0, 3.0),
        vel: Vec3::new(0.0, 0.0, 500.0),
        normal: Vec3::Y,
        heading: Vec3::Z,
        age: 1.0,
        ..Round::default()
    };
    let m = round_matrix(&round);
    assert_eq!(m.w_axis.truncate(), round.pos);
    assert!(
        (m.z_axis.truncate() + Vec3::Z * 0.75).length() < 1e-6,
        "the model's +Z back along the heading, scaled"
    );
    assert!(
        (m.x_axis.truncate() + Vec3::X * 0.75).length() < 1e-6,
        "right = up x forward, turned"
    );
    assert!(
        (m.y_axis.truncate() - Vec3::Y * 0.75).length() < 1e-6,
        "up, scaled"
    );
    assert!(
        (m.determinant() - 0.75_f32.powi(3)).abs() < 1e-5,
        "a scaled rotation, not a mirror"
    );
}

#[test]
fn a_young_round_is_half_as_long_and_grows_over_a_tenth_of_a_second() {
    let at = |age: f32| {
        round_matrix(&Round {
            live: true,
            normal: Vec3::Y,
            heading: Vec3::Z,
            age,
            ..Round::default()
        })
    };
    assert!((at(0.0).z_axis.length() - 0.375).abs() < 1e-6);
    assert!((at(0.05).z_axis.length() - 0.5625).abs() < 1e-6);
    assert!((at(0.2).z_axis.length() - 0.75).abs() < 1e-6);
    assert!(
        (at(0.0).x_axis.length() - 0.75).abs() < 1e-6,
        "width is not grown"
    );
}

#[test]
fn a_round_is_drawn_along_its_heading_and_up_its_surface_normal_not_its_velocity() {
    let round = Round {
        live: true,
        vel: Vec3::new(500.0, 0.0, 0.0),
        heading: Vec3::Z,
        normal: Vec3::Y,
        age: 1.0,
        ..Round::default()
    };
    assert!((round_matrix(&round).z_axis.truncate().normalize() + Vec3::Z).length() < 1e-6);
}

#[test]
fn a_round_flying_straight_up_the_launch_up_is_still_a_rotation() {
    let round = Round {
        live: true,
        vel: Vec3::new(0.0, 500.0, 0.0),
        normal: Vec3::Y,
        ..Round::default()
    };
    assert!((round_matrix(&round).determinant() - 0.75_f32.powi(3) * 0.5).abs() < 1e-5);
}

/// **The disc's own effects play for what a tick did.** An HD Detonator race resolves all four
/// triggers to loaded `.pob` effects, and an event of each kind puts one on the stage; with the
/// handles dropped, nothing plays.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_detonator_race_plays_the_discs_effects_for_each_event() {
    use oag_weapons::detonator::Event;
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: Mode::Detonator,
        track: Some(TALONS.to_string()),
        ..crate::Options::default()
    })
    .expect("loading an HD Detonator race");
    let mut race = Race::start(loaded.setup, loaded.view, loaded.modules);
    check_the_wall_spark_is_the_discs_pictures(&race);
    let at = Vec3::new(1.0, 2.0, 3.0);
    let spawn = Vec3::new(4.0, 5.0, 6.0);
    for (kind, trigger) in [
        (
            EventKind::MineShot { live: true },
            Trigger::DetonatorMineExplo,
        ),
        (EventKind::MineTouched, Trigger::DetonatorMineExploDead),
        (EventKind::BombShot, Trigger::DetonatorBombExplo),
        (
            EventKind::RoundWall {
                normal: Vec3::Y,
                spawn,
                rebound: Vec3::X,
            },
            Trigger::DetonatorCannonSparks,
        ),
    ] {
        assert!(
            race.view.handles.get(trigger).is_some(),
            "{trigger:?} resolves to a loaded effect"
        );
        let playing = race.view.stage.playing_count();
        race.core.sim.detonator_events = vec![Event { kind, at }];
        race.view.play_detonator_events(&race.core);
        assert!(
            race.view.stage.playing_count() > playing,
            "{kind:?} puts {trigger:?} on the stage"
        );
        if let EventKind::RoundWall { .. } = kind {
            let effect = race.view.handles.get(trigger).cloned().unwrap();
            assert_eq!(
                race.view.stage.frames_of(&effect),
                [(spawn, Vec3::X)],
                "the spark spawns on the road below the hit and faces its rebound"
            );
        }
    }
    race.core.sim.detonator_events = vec![
        Event {
            kind: EventKind::MineShot { live: false },
            at,
        },
        Event {
            kind: EventKind::BombTouched,
            at,
        },
    ];
    race.core.sim.drain_cues();
    race.core.sim.detonator_cues();
    let heard: Vec<_> = race.core.sim.drain_cues().iter().map(|e| e.cue).collect();
    assert_eq!(
        heard,
        [
            oag_gameplay::cue::Cue::MineExplosion,
            oag_gameplay::cue::Cue::BombExplosion
        ],
        "a mine and a bomb going off are heard where they were"
    );
    let playing = race.view.stage.playing_count();
    race.core.sim.detonator_events = vec![Event {
        kind: EventKind::Volley,
        at,
    }];
    race.view.play_detonator_events(&race.core);
    assert_eq!(
        race.view.stage.playing_count(),
        playing,
        "a volley plays no effect of its own"
    );
    check_the_wall_spark_matches_the_originals_matrices(&mut race);
}

const TALONS: &str = r"Data\Environments\Talons_Junction\track.vex";

/// **The spark is drawn from the disc's pictures, not a procedural stand-in:** the effect the
/// race loaded is `WO_CANNON_SPARKS_DETONATOR`, every emitter has its `.gtf` placed on the sheet,
/// and its three class-6 emitters draw as bars.
fn check_the_wall_spark_is_the_discs_pictures(race: &Race) {
    use oag_fx::psys::{Render, streak::StreakDraw};
    let effect = race
        .view
        .handles
        .get(Trigger::DetonatorCannonSparks)
        .expect("the spark loads");
    assert_eq!(effect.name, "WO_CANNON_SPARKS_DETONATOR");
    assert_eq!(effect.emitters.len(), 5);
    for spec in &effect.emitters {
        assert!(
            spec.sheet_rect.is_some(),
            "{} draws the disc's sprite, not the procedural profile",
            spec.name
        );
        if matches!(spec.render, Render::Streak { .. }) {
            assert!(
                matches!(spec.streak, StreakDraw::FarBar { .. }),
                "{}: {:?}",
                spec.name,
                spec.streak
            );
        }
    }
}

/// `(prev, velocity, surface normal, the matrix's row 1, its translation)` of seven rounds
/// polled as they died on a wall of Talon's Junction, with the particle instance the original
/// spawned for each (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, "The spark's
/// matrix, read live").
type Death = ([f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3]);

const DEATHS: [Death; 7] = [
    (
        [619.3002, 3.2127, 124.2231],
        [3.6515, 83.1358, 493.0265],
        [-0.3899, 0.9085, -0.1503],
        [-0.3870, -0.0178, 0.9219],
        [620.9861, 0.3114, 125.5396],
    ),
    (
        [613.5492, 3.4330, 138.9191],
        [-4.8292, 100.4846, 489.7750],
        [-0.4028, 0.8958, -0.1878],
        [-0.6116, -0.1234, 0.7815],
        [615.2009, 1.8068, 146.0654],
    ),
    (
        [620.7049, 1.9799, 111.5507],
        [45.0844, 76.9835, 491.9766],
        [-0.3819, 0.9178, -0.1086],
        [-0.3273, -0.0269, 0.9446],
        [622.6589, -0.5903, 115.1285],
    ),
    (
        [550.5379, -22.8851, -23.0110],
        [482.5850, 25.8275, -128.2366],
        [-0.0737, 0.9943, -0.0770],
        [-0.2199, 0.0591, 0.9737],
        [551.3989, -26.3431, -23.0346],
    ),
    (
        [545.8762, -19.7601, 19.1973],
        [-47.2410, 36.7154, 496.4073],
        [-0.0684, 0.9944, -0.0801],
        [0.9957, 0.0738, 0.0552],
        [544.8777, -22.4320, 30.9273],
    ),
    (
        [563.0886, -20.9972, -10.6707],
        [482.7393, 23.6959, -128.0675],
        [-0.0702, 0.9943, -0.0805],
        [-0.2875, 0.0570, 0.9561],
        [563.8041, -24.4655, -10.6543],
    ),
    (
        [557.5680, -17.3981, 39.9514],
        [-45.6567, 17.2844, 497.6110],
        [-0.0835, 0.9956, -0.0422],
        [0.9659, 0.1128, 0.2331],
        [557.2659, -20.6850, 46.2157],
    ),
];

/// Ours, seeded with the original's dying round, reflects off the same wall: the frame it hands
/// the spark is within a degree of the original's row 1 and its spawn within `0.6` units of the
/// original's translation. A spark left facing world up is 83 to 97 degrees off.
fn check_the_wall_spark_matches_the_originals_matrices(race: &mut Race) {
    use oag_weapons::detonator::cannon::Round;
    while oag_race::RaceState::thrust_gated(race.core.sim.world.tick) {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    for (prev, vel, normal, row1, translation) in DEATHS {
        let (prev, vel) = (Vec3::from_array(prev), Vec3::from_array(vel));
        race.core.sim.world.detonator.live_mines = 0;
        race.core.sim.world.detonator.cannon.rounds = Default::default();
        race.core.sim.world.detonator.cannon.rounds[0] = Round {
            live: true,
            pos: prev,
            prev,
            vel,
            normal: Vec3::from_array(normal),
            heading: vel.normalize(),
            age: 0.15,
            rebounds: 0,
            ..Round::default()
        };
        let mut seen = None;
        for _ in 0..4 {
            race.tick(&oag_gameplay::PlayerInputs::none());
            seen = race
                .core
                .sim
                .detonator_events
                .iter()
                .find_map(|e| match e.kind {
                    EventKind::RoundWall { spawn, rebound, .. } => Some((spawn, rebound)),
                    _ => None,
                });
            if seen.is_some() {
                break;
            }
        }
        let (spawn, rebound) = seen.unwrap_or_else(|| panic!("no wall death near {translation:?}"));
        let off = rebound
            .dot(Vec3::from_array(row1).normalize())
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        let drift = spawn.distance(Vec3::from_array(translation));
        assert!(
            off < 1.0,
            "row 1 {off} degrees from the original's {row1:?}"
        );
        assert!(
            drift < 0.6,
            "spawn {drift} units from the original's {translation:?}"
        );
    }
}

/// **The EMP's wave draws the disc's `WO_QUAKE_DETONATOR` on its front and nothing once it ends.**
/// An HD Detonator race resolves [`Trigger::DetonatorEmp`]; a wave fired from a stand puts one
/// instance on the stage that follows the front down the track, sits within the track's edges
/// there, and is released on the tick the wave ends.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_emp_wave_carries_the_discs_effect_on_its_front_until_it_ends() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: Mode::Detonator,
        ..crate::Options::default()
    })
    .expect("loading an HD Detonator race");
    let mut race = Race::start(loaded.setup, loaded.view, loaded.modules);
    assert!(
        race.view.handles.get(Trigger::DetonatorEmp).is_some(),
        "WO_QUAKE_DETONATOR resolves to a loaded effect"
    );
    let survey = race
        .core
        .sim
        .detonator
        .as_ref()
        .expect("a Detonator race")
        .survey
        .clone();
    let wave = &mut race.core.sim.world.detonator.emp.wave;
    *wave = oag_weapons::detonator::emp::Wave {
        live: true,
        from: 10,
        front: 0,
        lifetime: 0.25,
        age: 0.0,
        ..oag_weapons::detonator::emp::Wave::default()
    };
    race.view.spu_vertex_lights = true;
    race.view.hd_trail_active = true;
    race.view.detonator_cannon_lights = true;
    race.set_weapon_lights(true);
    let no_rounds = crate::CannonDraw {
        look: None,
        muzzles: Vec::new(),
    };
    let mut anchors = Vec::new();
    let mut lit = 0;
    for _ in 0..30 {
        if race.core.sim.world.detonator.emp.wave.live {
            race.core.sim.world.detonator.emp.wave.front += 1;
            race.core.sim.world.detonator.emp.wave.age += 1.0 / 60.0;
            if race.core.sim.world.detonator.emp.wave.age >= 0.25 {
                race.core.sim.world.detonator.emp.wave.live = false;
            }
        }
        race.view.advance_emp_visual(&race.core);
        let anchor = race
            .view
            .emp_effect
            .and_then(|p| race.view.stage.anchor_of(p));
        let lights = race.detonator_cannon_lights(&no_rounds);
        if let Some(at) = anchor {
            assert!(
                lights
                    .iter()
                    .any(|l| l.colour == [0.0, 5.0, 10.0, 50.0]
                        && l.position == [at.x, at.y, at.z, 1.0]),
                "the wave lights its centre: {lights:?}"
            );
            lit += 1;
        } else {
            assert!(lights.is_empty(), "no light once the wave ends");
        }
        anchors.push(anchor);
    }
    assert!(lit >= 10, "the wave was lit for {lit} ticks");
    let live: Vec<Vec3> = anchors.iter().flatten().copied().collect();
    assert!(live.len() >= 10, "the effect rides the wave: {anchors:?}");
    assert!(
        anchors[20..].iter().all(Option::is_none),
        "released once the wave ends"
    );
    let first = survey.entries[11].centre;
    assert!(
        (live[0] - first).length() < 20.0,
        "the effect starts on the front's entry: {:?} against {first:?}",
        live[0]
    );
    assert!(
        (live[live.len() - 1] - live[0]).length() > 20.0,
        "and moves down the track with it"
    );
}
