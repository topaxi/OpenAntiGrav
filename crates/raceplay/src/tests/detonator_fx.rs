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
