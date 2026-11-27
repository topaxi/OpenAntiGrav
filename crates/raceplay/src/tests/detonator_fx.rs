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
/// race loaded is `WO_CANNON_SPARKS_DETONATOR`, every emitter has its picture (`.gtf`, `.gxt` or
/// `.gnf`) placed on the sheet, and its three class-6 emitters draw as bars.
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
        [619.3, 3.213, 124.223],
        [3.6515, 83.136, 493.026],
        [-0.39, 0.908, -0.15],
        [-0.387, -0.018, 0.922],
        [620.986, 0.311, 125.54],
    ),
    (
        [613.549, 3.433, 138.919],
        [-4.829, 100.485, 489.775],
        [-0.403, 0.896, -0.188],
        [-0.612, -0.123, 0.781],
        [615.201, 1.807, 146.065],
    ),
    (
        [620.705, 1.98, 111.551],
        [45.084, 76.984, 491.977],
        [-0.382, 0.918, -0.109],
        [-0.327, -0.027, 0.945],
        [622.659, -0.59, 115.1285],
    ),
    (
        [550.538, -22.885, -23.011],
        [482.585, 25.828, -128.237],
        [-0.074, 0.994, -0.077],
        [-0.22, 0.059, 0.974],
        [551.399, -26.343, -23.035],
    ),
    (
        [545.876, -19.76, 19.197],
        [-47.241, 36.715, 496.407],
        [-0.068, 0.994, -0.08],
        [0.996, 0.074, 0.055],
        [544.878, -22.432, 30.927],
    ),
    (
        [563.089, -20.997, -10.671],
        [482.739, 23.696, -128.067],
        [-0.07, 0.994, -0.0805],
        [-0.2875, 0.057, 0.956],
        [563.804, -24.465, -10.654],
    ),
    (
        [557.568, -17.398, 39.951],
        [-45.657, 17.284, 497.611],
        [-0.0835, 0.996, -0.042],
        [0.966, 0.113, 0.233],
        [557.266, -20.685, 46.216],
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
    race.view.spu_vertex_lights = true;
    race.view.hd_trail_active = true;
    race.view.detonator_cannon_lights = true;
    race.set_weapon_lights(true);
    check_the_wave_is_the_discs_effect(race);
}

/// A wave fired from a stand on `race`: the effect and the light ride its front and go with it.
fn check_the_wave_is_the_discs_effect(race: Race) {
    check_the_wave_rides(race, true);
}

/// [`check_the_wave_is_the_discs_effect`] for a title whose wave submits a light, or not.
fn check_the_wave_rides(mut race: Race, lights_the_wave: bool) {
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
        if let Some(at) = anchor.filter(|_| lights_the_wave) {
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
    assert!(
        !lights_the_wave || lit >= 10,
        "the wave was lit for {lit} ticks"
    );
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

/// 2048's and Omega's tables answer the EMP's trigger too: each EMP launch spawns the disc's
/// `WO_QUAKE_DETONATOR` and each draws the same light (`detonator-emp.md`, "2048 and Omega draw
/// it"). Nothing is switched on by hand: the light, the trail gate and the effect all come from
/// the loaded race.
fn check_the_title_draws_the_emp_wave(source: String, track: Option<&str>) {
    let loaded = crate::load(&crate::Options {
        source,
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Detonator,
        track: track.map(str::to_string),
        ..crate::Options::default()
    })
    .expect("loading a Detonator race");
    let race = Race::start(loaded.setup, loaded.view, loaded.modules);
    let effect = race
        .view
        .handles
        .get(Trigger::DetonatorEmp)
        .expect("WO_QUAKE_DETONATOR resolves to a loaded effect");
    assert_eq!(effect.name, oag_title::engine_effects::DETONATOR_EMP_EFFECT);
    assert!(effect.has_turbulence(), "its sparks feel their noise force");
    check_the_wave_is_the_discs_effect(race);
}

#[test]
#[ignore = "needs the unpacked 2048 packages in data/extracted/"]
fn a_2048_emp_wave_carries_the_discs_effect_and_light_on_its_front() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/extracted/vita/PCSF00007");
    if !path.join("patch-v104").exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return;
    }
    check_the_title_draws_the_emp_wave(
        path.display().to_string(),
        Some(r"data\art\published\DLC1\environments\zone_1\track.vex"),
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_omega_emp_wave_carries_the_discs_effect_and_light_on_its_front() {
    let Some(path) = oag_testdata::image("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    check_the_title_draws_the_emp_wave(path.display().to_string(), None);
}

/// 2048's and Omega's tables answer the wall spark's trigger: their own `PulseBullet` updates
/// spawn it on the last wall only, in HD's frame (`detonator-cannon.md`, "The spark on 2048 and
/// Omega"). A wall death puts the disc's effect on the stage at the spawn point, facing the
/// rebound.
fn check_the_title_plays_the_wall_spark(source: String, track: Option<&str>) {
    use oag_weapons::detonator::Event;
    let loaded = crate::load(&crate::Options {
        source,
        class: "VENOM".to_string(),
        mode: Mode::Detonator,
        track: track.map(str::to_string),
        ..crate::Options::default()
    })
    .expect("loading a Detonator race");
    let mut race = Race::start(loaded.setup, loaded.view, loaded.modules);
    check_the_wall_spark_is_the_discs_pictures(&race);
    let (spawn, rebound) = (Vec3::new(4.0, 5.0, 6.0), Vec3::X);
    race.core.sim.detonator_events = vec![Event {
        kind: EventKind::RoundWall {
            normal: Vec3::Y,
            spawn,
            rebound,
        },
        at: spawn,
    }];
    race.view.play_detonator_events(&race.core);
    let effect = race
        .view
        .handles
        .get(Trigger::DetonatorCannonSparks)
        .cloned()
        .expect("the spark loads");
    assert_eq!(race.view.stage.frames_of(&effect), [(spawn, rebound)]);
}

#[test]
#[ignore = "needs the unpacked 2048 packages in data/extracted/"]
fn a_2048_wall_death_plays_the_discs_spark() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/extracted/vita/PCSF00007");
    if !path.join("patch-v104").exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return;
    }
    check_the_title_plays_the_wall_spark(
        path.display().to_string(),
        Some(r"data\art\published\DLC1\environments\zone_1\track.vex"),
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_omega_wall_death_plays_the_discs_spark() {
    let Some(path) = oag_testdata::image("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    check_the_title_plays_the_wall_spark(path.display().to_string(), None);
}

/// A Pulse Detonator race: its title ships no Detonator, so no cannon model, EMP effect or
/// light of its own.
fn pulse_race() -> Option<Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: Mode::Detonator,
        ..crate::Options::default()
    })
    .expect("loading a Pulse Detonator race");
    Some(Race::start(loaded.setup, loaded.view, loaded.modules))
}

/// **A Pulse Detonator round is drawn as Pulse's own Cannon's**: the Cannon model posed on the
/// round, its bolt streak and, while young, its flash quad.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_pulse_detonator_round_is_drawn_as_the_pulse_cannons_own() {
    let Some(mut race) = pulse_race() else {
        return;
    };
    let draw = crate::CannonDraw::default();
    assert!(race.cannon_model_matrices(&draw).is_empty());
    let at = race.core.sim.world.ships[0].physics.body.position;
    race.core.sim.world.detonator.cannon.rounds[0] = Round {
        live: true,
        pos: at + Vec3::Z * 10.0,
        prev: at + Vec3::Z * 2.0,
        vel: Vec3::Z * 500.0,
        normal: Vec3::Y,
        age: 0.02,
        ..Round::default()
    };
    let models = race.cannon_model_matrices(&draw);
    assert_eq!(models.len(), 1, "the round carries the Cannon's model");
    assert!((models[0].w_axis.truncate() - (at + Vec3::Z * 10.0)).length() < 1e-3);
    let (mut bolt, mut flash) = (Vec::new(), Vec::new());
    race.cannon_quad_vertices(&draw, Vec3::X, Vec3::Y, &mut bolt, &mut flash);
    assert_eq!(bolt.len(), 12, "its bolt streak");
    assert_eq!(flash.len(), 6, "and its flash while young");
    race.core.sim.world.detonator.cannon.rounds[0].age = 1.0;
    let (mut bolt, mut flash) = (Vec::new(), Vec::new());
    race.cannon_quad_vertices(&draw, Vec3::X, Vec3::Y, &mut bolt, &mut flash);
    assert_eq!(
        (bolt.len(), flash.len()),
        (12, 0),
        "no flash once it is old"
    );
}

/// **A Pulse EMP draws the Repulsor**: `WO_REPULSER` rides the wave front with no light, and
/// the launch opens the field ring on the firer, with the screen flash, for the Repulsor's life.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_pulse_emp_wave_is_the_repulsors_wave_and_opening() {
    let Some(mut race) = pulse_race() else {
        return;
    };
    let effect = race
        .view
        .handles
        .get(Trigger::DetonatorEmp)
        .expect("the EMP resolves to a loaded effect");
    assert_eq!(effect.name, oag_title::engine_effects::REPULSER_EFFECT);
    assert!(!race.view.detonator_emp_light, "no light is submitted");
    assert!(race.repulser_field_draws()[0].is_none());
    race.core
        .sim
        .detonator_events
        .push(oag_weapons::detonator::Event {
            kind: EventKind::EmpLaunch,
            at: race.core.sim.world.ships[0].physics.body.position,
        });
    race.view.advance_emp_repulsor(&race.core);
    race.core.sim.detonator_events.clear();
    let (_, alpha, _) = race.repulser_field_draws()[0].expect("the ring is up at the launch");
    assert!(alpha > 0.0);
    let eye = race.view.camera_position(&race.core);
    let flash = race.view.screen_flash.as_mut().expect("Pulse flashes");
    flash.advance(race.core.sim.dt, eye);
    assert!(
        flash.colour().is_some_and(|c| c[2] > c[0]),
        "the Repulsor's blue screen flash starts"
    );
    for _ in 0..120 {
        race.view.advance_emp_repulsor(&race.core);
    }
    assert!(
        race.repulser_field_draws()[0].is_none(),
        "and it is gone after the Repulsor's life"
    );
    check_the_wave_rides(race, false);
}

/// **A Pulse Detonator mine holds its authored size** however loud the music's envelope is,
/// and a layout that names `MinesScaleToMusic="1"` grows it again.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_pulse_detonator_mine_scales_to_the_music_only_if_its_layout_says_so() {
    let Some(mut race) = pulse_race() else {
        return;
    };
    race.tick(&oag_gameplay::PlayerInputs::none());
    let (ordinary, laid) = race.detonator_mine_looks();
    assert!(laid.iter().any(|mine| mine.live), "stage 1 is laid");
    let loud = vec![
        crate::detonator_look::MineEnvelope {
            football: 1.0,
            spikes: 1.0,
        };
        laid.len()
    ];
    let size = |race: &Race| {
        race.mine_model_matrices()[ordinary]
            .x_axis
            .truncate()
            .length()
    };
    race.record_mine_sizes(&laid, &loud);
    assert!((size(&race) - 1.0).abs() < 1e-4, "held: {}", size(&race));
    let detonator = race.core.sim.detonator.as_mut().expect("a Detonator race");
    std::sync::Arc::make_mut(&mut detonator.layout)
        .values
        .mines_scale_to_music = true;
    race.record_mine_sizes(&laid, &loud);
    assert!(size(&race) > 2.0, "scaled: {}", size(&race));
}
