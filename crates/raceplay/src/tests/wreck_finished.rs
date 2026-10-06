//! What a race a wreck ended goes on doing under the results - see
//! `Race::tick_cosmetics` and `Race::runs_on_after_the_wreck`. In a Single Race
//! the world runs on, as the original's does (the field races, the player's wreck
//! stays down); in a Zone run only what is seen moves: the explosion, the shake and
//! the destroy camera.

use super::*;
use crate::wreck_fx::WreckFx;
use oag_physics::CraftState;

/// A single race whose player's craft has been shot down and the race is over,
/// the wreck effects loaded, and a destroy camera station to cut to.
fn a_race_the_wreck_ended() -> Race {
    a_race_the_wreck_ended_in(Mode::SingleRace)
}

fn a_race_the_wreck_ended_in(mode: Mode) -> Race {
    let mut race = a_race_ready_for_a_wreck(mode);
    race.force_destroy(0);
    for _ in 0..120 {
        race.tick(&oag_gameplay::PlayerInputs::none());
        if race.finished() {
            return race;
        }
    }
    panic!("the destroyed player's single race never ended");
}

/// The fixtures a wreck needs - the effect it throws, a destroy camera station - and no wreck yet.
fn a_race_ready_for_a_wreck(mode: Mode) -> Race {
    let mut race = race_with_a_grid();
    if mode != Mode::SingleRace {
        race = zone_grid(mode);
    }
    race.view.wreck_fx = WreckFx::new(vec![Vec::new(); 1]);
    let blob = super::respawn::one_emitter_pob(
        crate::tests::respawn::trigger_name(Trigger::WreckExplosion),
        0,
    );
    let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    race.view.handles.insert(Trigger::WreckExplosion, effect);
    race.view.destroy_camera = crate::destroy_camera::DestroyCamera::new(
        vec![oag_render::camera::destroy::Station {
            eye: Vec3::new(400.0, 30.0, 0.0),
            aim: Vec3::new(10.0, 0.0, 0.0),
        }],
        0.4,
    );
    race
}

/// Zone's wreck ends the race the same way but was not looked at, so the world stands
/// still while the cosmetics play: shake, destroy camera, and the big explosion 1.5 s on.
#[test]
fn a_zone_wreck_plays_out_under_the_results_without_moving_the_world() {
    let mut race = a_race_the_wreck_ended_in(Mode::Zone);
    assert!(!race.runs_on_after_the_end());
    assert_eq!(
        race.sim.world.ships[0].physics.craft_state,
        CraftState::Eliminated
    );
    assert!(race.view.shake.active(), "the state 5 shake is armed");
    assert_eq!(race.wreck_explosion_at_for_tests(), None);
    let hash = race.sim.state_hash();
    let tick = race.sim.world.tick;
    let fov = race.vertical_fov(16.0 / 9.0, oag_display::display::Fov::AUTHORED);

    let mut shaken = 0;
    for _ in 0..30 {
        race.tick_finished();
        shaken += u32::from(race.view.shake.active());
    }
    assert!(
        shaken > 0 && !race.view.shake.active(),
        "the shake plays out"
    );
    assert_eq!(
        race.wreck_explosion_at_for_tests(),
        None,
        "1.5 s is 90 ticks"
    );
    for _ in 0..70 {
        race.tick_finished();
    }
    assert!(
        race.wreck_explosion_at_for_tests().is_some(),
        "the big explosion goes off under the results"
    );
    assert!(race.view.shake.active(), "and shakes the camera again");
    assert_ne!(
        race.vertical_fov(16.0 / 9.0, oag_display::display::Fov::AUTHORED),
        fov,
        "the destroy camera keeps easing in"
    );
    assert_eq!(race.sim.world.tick, tick, "the world's own clock stands");
    assert_eq!(race.sim.state_hash(), hash, "nothing the hash reads moved");
}

/// A Single Race wreck: the world runs on under the results - the clock advances, the
/// player's craft stays down, the race stays over - and the explosion still plays.
#[test]
fn a_single_race_wreck_keeps_the_world_running_under_the_results() {
    let mut race = a_race_the_wreck_ended();
    assert!(!race.runs_on_after_the_line(), "the player never crossed");
    assert!(race.runs_on_after_the_wreck());
    assert!(race.runs_on_after_the_end());
    let tick = race.sim.world.tick;
    for _ in 0..200 {
        race.tick_finished();
    }
    assert_eq!(race.sim.world.tick, tick + 200, "the world's clock runs on");
    assert!(race.finished(), "the race stays over");
    assert_eq!(
        race.sim.world.ships[0].physics.craft_state,
        CraftState::Eliminated,
        "a single race's wreck never comes back"
    );
    assert!(
        race.wreck_explosion_at_for_tests().is_some(),
        "the explosion goes off"
    );
}

/// The spectator director takes over from the destroy camera `WRECK_HANDOFF_TICKS` after the
/// race ended - not on the line's `START_TICKS` - in the death camera's mode 5, and follows
/// a craft that is still live.
#[test]
fn the_director_takes_over_from_the_destroy_camera_after_a_single_race_wreck() {
    use crate::finish_camera::{FinishCamera, SPECTATOR_SEED, ViewMode, WRECK_HANDOFF_TICKS};
    let mut race = a_race_the_wreck_ended();
    let station = oag_render::camera::destroy::Station {
        eye: Vec3::new(400.0, 30.0, 0.0),
        aim: Vec3::new(10.0, 0.0, 0.0),
    };
    race.view.finish_camera = Some(FinishCamera::new(vec![station], 0.4, SPECTATOR_SEED));
    for _ in 0..WRECK_HANDOFF_TICKS - 1 {
        race.tick_finished();
    }
    assert_eq!(
        race.spectator_mode(),
        None,
        "the destroy camera still has it"
    );
    race.tick_finished();
    race.tick_finished();
    assert_eq!(
        race.spectator_mode(),
        Some(ViewMode::Death),
        "then the director starts, in the death camera's mode"
    );
}

/// `race_with_a_grid` with the mode switched: eight craft in an Eliminator.
fn an_eliminator_grid() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::Eliminator;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    Race::start(setup)
}

/// The Eliminator's state 8: the player comes back `1.0` s after state 5's
/// dwell, an opponent `0.8` s after it (`Ship_SetState` case 8, counted down by
/// `Ship_UpdateRespawn`), so the destroy camera is still on the wreck when the
/// big explosion goes off and lets go a second later.
#[test]
fn the_eliminators_wait_holds_the_destroy_camera_through_the_explosion() {
    let mut race = an_eliminator_grid();
    race.view.wreck_fx = WreckFx::new(vec![Vec::new(); 1]);
    let blob = super::respawn::one_emitter_pob(
        crate::tests::respawn::trigger_name(Trigger::WreckExplosion),
        0,
    );
    let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    race.view.handles.insert(Trigger::WreckExplosion, effect);
    race.view.destroy_camera = crate::destroy_camera::DestroyCamera::new(
        vec![oag_render::camera::destroy::Station {
            eye: Vec3::new(400.0, 30.0, 0.0),
            aim: Vec3::new(10.0, 0.0, 0.0),
        }],
        0.4,
    );
    race.sim.world.ships[1].physics.craft_state = CraftState::Eliminated;
    race.force_destroy(0);

    let (mut edge, mut blast, mut back, mut opponent_back) = (None, None, None, None);
    for tick in 0..260u32 {
        race.tick(&oag_gameplay::PlayerInputs::none());
        let state = race.sim.world.ships[0].physics.craft_state;
        if edge.is_none() && state == CraftState::Eliminated {
            edge = Some(tick);
        }
        if blast.is_none() && race.wreck_explosion_at_for_tests().is_some() {
            blast = Some(tick);
            assert!(
                race.destroy_camera_now().is_some(),
                "the camera is on the wreck as the explosion goes off"
            );
        }
        if back.is_none() && edge.is_some() && state == CraftState::Racing {
            back = Some(tick);
            assert!(
                race.destroy_camera_now().is_none(),
                "and lets go as it returns"
            );
        }
        if opponent_back.is_none()
            && race.sim.world.ships[1].physics.craft_state == CraftState::Racing
        {
            opponent_back = Some(tick);
        }
    }
    let (edge, blast, back) = (edge.unwrap(), blast.unwrap(), back.unwrap());
    assert!(
        (88..=92).contains(&(blast - edge)),
        "1.5 s to the blast: {}",
        blast - edge
    );
    assert!(
        (148..=152).contains(&(back - edge)),
        "2.5 s to the return: {}",
        back - edge
    );
    assert!(
        back > blast + 55,
        "the camera holds a second past the blast"
    );
    let opponent = opponent_back.expect("the opponent returns");
    assert!(
        (136..=140).contains(&opponent),
        "2.3 s from tick 0: {opponent}"
    );
}

/// `race_with_a_grid` in another mode.
fn zone_grid(mode: Mode) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = mode;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    Race::start(setup)
}

/// A camera that stands on an authored station is masked by the section of the craft it
/// draws (`cam+0x1e8`, which `Camera_UpdateSpectatorView` publishes in every mode), not by
/// where it stands: that is how the original leaves out the inside of the structure a
/// station sits in. Any other camera has no such section.
#[test]
fn a_station_camera_is_masked_by_the_section_of_the_craft_it_draws() {
    let race = race_with_a_grid();
    assert_eq!(race.station_camera_section(), None, "the chase camera");
    let mut race = a_race_the_wreck_ended();
    assert!(
        race.destroy_camera_now().is_some(),
        "the destroy camera has the picture"
    );
    assert_eq!(
        race.station_camera_section(),
        Some(race.section_of_slot(0)),
        "the wreck's own section"
    );
    // The director draws whichever craft it was handed: a live opponent after a cut.
    race.view.finish_camera = Some(crate::finish_camera::FinishCamera::new(
        vec![oag_render::camera::destroy::Station {
            eye: Vec3::new(400.0, 30.0, 0.0),
            aim: Vec3::new(10.0, 0.0, 0.0),
        }],
        0.4,
        crate::finish_camera::SPECTATOR_SEED,
    ));
    for _ in 0..crate::finish_camera::WRECK_HANDOFF_TICKS + 2 {
        race.tick_finished();
    }
    let slot = race
        .view
        .finish_camera
        .as_ref()
        .and_then(crate::finish_camera::FinishCamera::drawn_slot)
        .expect("the director has started");
    assert_eq!(
        race.station_camera_section(),
        Some(race.section_of_slot(slot))
    );
}

/// The hand-off is a law of the wreck: `Camera_UpdateSpectator` clears the subject when the
/// wreck's state-6 timer crosses zero, `240` frames after `Ship_SetState(4)` (measured twice
/// on PPSSPP: `0.5 + 1.5 + 2.0` s). Pinned to the call, not to a derived offset.
#[test]
fn the_director_starts_240_ticks_after_the_wreck_call() {
    use crate::finish_camera::{FinishCamera, SPECTATOR_SEED};
    let mut race = a_race_ready_for_a_wreck(Mode::SingleRace);
    race.view.finish_camera = Some(FinishCamera::new(
        vec![oag_render::camera::destroy::Station {
            eye: Vec3::new(400.0, 30.0, 0.0),
            aim: Vec3::new(10.0, 0.0, 0.0),
        }],
        0.4,
        SPECTATOR_SEED,
    ));
    race.force_destroy(0);
    let called = race.sim.world.tick;
    for _ in 0..600 {
        if race.finished() {
            race.tick_finished();
        } else {
            race.tick(&oag_gameplay::PlayerInputs::none());
        }
        if race.spectator_mode().is_some() {
            assert_eq!(race.sim.world.tick - called, 240);
            return;
        }
    }
    panic!("the director never started");
}
