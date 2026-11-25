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
            EventKind::RoundWall { normal: Vec3::Y },
            Some(Trigger::DetonatorCannonSparks),
        ),
        (EventKind::Volley, None),
    ];
    for (kind, want) in table {
        assert_eq!(effect_for(kind), want, "{kind:?}");
    }
}

#[test]
fn a_round_is_drawn_nose_along_its_velocity_as_a_rotation_at_its_position() {
    let round = Round {
        live: true,
        pos: Vec3::new(1.0, 2.0, 3.0),
        vel: Vec3::new(0.0, 0.0, 500.0),
        up: Vec3::Y,
        ..Round::default()
    };
    let m = round_matrix(&round);
    assert_eq!(m.w_axis.truncate(), round.pos);
    assert!((m.z_axis.truncate() - Vec3::Z).length() < 1e-6, "nose");
    assert!((m.y_axis.truncate() - Vec3::Y).length() < 1e-6, "up");
    assert!(
        (m.determinant() - 1.0).abs() < 1e-5,
        "a rotation, not a mirror"
    );
}

#[test]
fn a_round_flying_straight_up_the_launch_up_is_still_a_rotation() {
    let round = Round {
        live: true,
        vel: Vec3::new(0.0, 500.0, 0.0),
        up: Vec3::Y,
        ..Round::default()
    };
    assert!((round_matrix(&round).determinant() - 1.0).abs() < 1e-5);
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
        ..crate::Options::default()
    })
    .expect("loading an HD Detonator race");
    let mut race = Race::start(loaded.setup, loaded.view, loaded.modules);
    let at = Vec3::new(1.0, 2.0, 3.0);
    for (kind, trigger) in [
        (
            EventKind::MineShot { live: true },
            Trigger::DetonatorMineExplo,
        ),
        (EventKind::MineTouched, Trigger::DetonatorMineExploDead),
        (EventKind::BombShot, Trigger::DetonatorBombExplo),
        (
            EventKind::RoundWall { normal: Vec3::Y },
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
