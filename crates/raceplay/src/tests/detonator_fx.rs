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
    check_a_shot_mine_throws_the_discs_shards(&race);
    check_a_live_mine_blast_takes_the_stage_colour(&mut race);
    check_a_shot_mine_floats_its_score(&mut race, &loaded.hud);
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
        race.core.sim.detonator_events = vec![Event {
            kind,
            at,
            points: None,
        }];
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
            points: None,
        },
        Event {
            kind: EventKind::BombTouched,
            at,
            points: None,
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
        points: None,
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

/// **A shot mine throws the disc's shards, not procedural puffs:** both mine effects draw every
/// emitter's own picture, and the live one's `Debris` is the 4 x 4 atlas of glass shards the
/// original's frame shows, 50 alpha-over particles picked at random from it
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md`, "The shards, against the original").
fn check_a_shot_mine_throws_the_discs_shards(race: &Race) {
    use oag_fx::psys::Blend;
    for (trigger, name, emitters) in [
        (Trigger::DetonatorMineExplo, "WO_MINE_EXPLO_DETONATOR", 6),
        (
            Trigger::DetonatorMineExploDead,
            "WO_MINE_EXPLO_DETONATOR_DEAD",
            3,
        ),
    ] {
        let effect = race
            .view
            .handles
            .get(trigger)
            .expect("the mine's effect loads");
        assert_eq!(effect.name, name);
        assert_eq!(effect.emitters.len(), emitters, "{name}");
        for spec in &effect.emitters {
            assert!(
                spec.sprite.is_some(),
                "{name}/{} reads its sprite",
                spec.name
            );
            if matches!(spec.render, oag_fx::psys::Render::Billboard) {
                assert!(
                    spec.sheet_rect.is_some(),
                    "{name}/{} draws the disc's sprite, not the procedural profile",
                    spec.name
                );
            }
        }
    }
    let live = race
        .view
        .handles
        .get(Trigger::DetonatorMineExplo)
        .expect("loaded");
    let debris = live
        .emitters
        .iter()
        .find(|e| e.name == "Debris")
        .expect("the live mine's shards");
    let sprite = debris.sprite.as_ref().expect("the shard atlas");
    assert_eq!((sprite.width, sprite.height), (512, 512));
    assert_eq!((debris.atlas.columns, debris.atlas.rows), (4, 4));
    assert!(debris.atlas.random_frame);
    assert_eq!(debris.per_emission, (50, 50));
    assert_eq!(debris.blend, Blend::AlphaOver);
}

/// **A live mine's blast is drawn in one colour from the stage's mine colour:** on the opening
/// stage (`1`, `Sub Venom`, mine colour `122 25 255` in `detonatormode.effectsettings`) every
/// emitter's table is the one entry `DetonatorMine_Explode` writes, `200 163 255`, and a shot live
/// mine plays that recoloured blast (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md`,
/// "The blast's colour").
fn check_a_live_mine_blast_takes_the_stage_colour(race: &mut Race) {
    let blast = race
        .view
        .mine_blasts
        .get(1)
        .cloned()
        .flatten()
        .expect("stage 1 authors a mine colour, so the blast is recoloured");
    assert_eq!(blast.emitters.len(), 6);
    let want = [200.0 / 255.0, 163.0 / 255.0, 1.0];
    for spec in &blast.emitters {
        assert!(
            spec.palette.iter().all(|entry| entry[..3] == want),
            "{} is drawn in the stage's blast colour",
            spec.name
        );
    }
    let at = Vec3::new(7.0, 8.0, 9.0);
    race.core.sim.detonator_events = vec![oag_weapons::detonator::Event {
        kind: EventKind::MineShot { live: true },
        at,
        points: None,
    }];
    race.view.play_detonator_events(&race.core);
    assert_eq!(
        race.view.stage.frames_of(&blast).len(),
        1,
        "a shot live mine on the opening stage plays the recoloured blast"
    );
}

/// **A score add floats its `+N` where the scored object was**, drawn through the layout's own
/// `ScoreFeedBack` widgets, fading and climbing each tick; 2000 points are drawn at scale 2
/// and dim the popup already up (`docs/ghidra/functions/ps3-hdfury-eu/detonator-hud.md`,
/// "The floating score").
fn check_a_shot_mine_floats_its_score(race: &mut Race, hud: &oag_hud::Assets) {
    race.view.score_feedback = Some(Default::default());
    let ahead = race
        .view
        .view(&race.core)
        .inverse()
        .transform_point3(Vec3::new(0.0, 0.0, -30.0));
    let context = hud.context().expect("the Detonator layout loaded");
    let popups = |race: &Race| {
        let frame = oag_hud::draw_list(&context, &race.readout());
        let mut shown = Vec::new();
        for draw in frame.small_text {
            if let oag_ui::frontend::Draw::Text {
                x,
                y,
                scale,
                color,
                border,
                text,
                ..
            } = draw
                && text.starts_with('+')
            {
                shown.push((text, x, y, scale, color[3], border.map(|b| b[3])));
            }
        }
        // The title's text shadow draws each string first; the face is the last of the pair.
        shown
            .chunks(2)
            .map(|pair| pair[1].clone())
            .collect::<Vec<_>>()
    };
    let shot = |points: Option<i32>| oag_weapons::detonator::Event {
        kind: EventKind::MineShot { live: true },
        at: ahead,
        points,
    };
    assert!(popups(race).is_empty(), "nothing scored yet");

    race.core.sim.detonator_events = vec![shot(Some(100))];
    race.view.play_detonator_events(&race.core);
    let [(text, x, first_y, scale, alpha, border)] = &popups(race)[..] else {
        panic!("one popup: {:?}", popups(race));
    };
    assert_eq!((text.as_str(), *scale), ("+100", 1.2));
    // HD's lit text is the written alpha times 0.8, in bytes: 249 reads 199.
    assert_eq!(
        *alpha,
        199.0 / 255.0,
        "faded once by the update after the add"
    );
    assert_eq!(*border, Some(249.0 / 255.0), "the border fades with it");
    let (x, first_y) = (*x, *first_y);
    assert!(
        (x - 960.0).abs() < 1.0,
        "at the point ahead of the camera: {x}"
    );

    race.core.sim.detonator_events.clear();
    for _ in 0..30 {
        race.view.play_detonator_events(&race.core);
    }
    let later = popups(race);
    assert!(
        later[0].2 < first_y - 20.0,
        "it climbs: {first_y} {later:?}"
    );
    assert!(later[0].4 < 0.6, "and fades: {later:?}");

    race.core.sim.detonator_events = vec![shot(Some(50)), shot(Some(2000)), shot(None)];
    race.view.play_detonator_events(&race.core);
    let shown = popups(race);
    let drawn: Vec<(&str, f32)> = shown.iter().map(|p| (p.0.as_str(), p.3)).collect();
    assert_eq!(
        drawn,
        [("+100", 1.2), ("+ 50", 1.2), ("+2000", 2.0)],
        "an event that pays nothing raises none"
    );
}

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
fn check_the_wave_is_the_discs_effect(mut race: Race) {
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
    assert!(
        race.view.score_feedback.is_none(),
        "the floating score is wired on HD only"
    );
    let (spawn, rebound) = (Vec3::new(4.0, 5.0, 6.0), Vec3::X);
    race.core.sim.detonator_events = vec![Event {
        kind: EventKind::RoundWall {
            normal: Vec3::Y,
            spawn,
            rebound,
        },
        at: spawn,
        points: None,
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
