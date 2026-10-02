//! What a race a wreck ended goes on doing under the results - see
//! `Race::tick_cosmetics`. The explosion, the shake and the destroy camera are
//! seen; the world, the standings and the board are not touched.

use super::*;
use crate::race::wreck_fx::{EXPLOSION_EFFECT, WreckFx};
use oag_physics::CraftState;

/// A single race whose player's craft has been shot down and the race is over,
/// the wreck effects loaded, and a destroy camera station to cut to.
fn a_race_the_wreck_ended() -> Race {
    let mut race = race_with_a_grid();
    race.view.wreck_fx = WreckFx::new(vec![Vec::new(); 1]);
    let blob = super::respawn::one_emitter_pob(EXPLOSION_EFFECT, 0);
    let effect = oag_render::psys::Effect::parse(&blob, oag_render::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    race.view.effects.insert(EXPLOSION_EFFECT, effect);
    race.view.destroy_camera = crate::race::destroy_camera::DestroyCamera::new(
        vec![oag_render::camera::destroy::Station {
            eye: Vec3::new(400.0, 30.0, 0.0),
            aim: Vec3::new(10.0, 0.0, 0.0),
        }],
        0.4,
    );
    race.force_destroy(0);
    for _ in 0..120 {
        race.tick(&oag_gameplay::PlayerInputs::none());
        if race.finished() {
            return race;
        }
    }
    panic!("the destroyed player's single race never ended");
}

/// The wreck ends the race and is not the line, so the world stands still while
/// the cosmetics play: shake, destroy camera, and the big explosion 1.5 s on.
#[test]
fn a_wreck_plays_out_under_the_results_without_moving_the_world() {
    let mut race = a_race_the_wreck_ended();
    assert!(!race.runs_on_after_the_line());
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
    let blob = super::respawn::one_emitter_pob(EXPLOSION_EFFECT, 0);
    let effect = oag_render::psys::Effect::parse(&blob, oag_render::psys::ColourScale::Full)
        .expect("the hand-laid effect parses");
    race.view.effects.insert(EXPLOSION_EFFECT, effect);
    race.view.destroy_camera = crate::race::destroy_camera::DestroyCamera::new(
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
