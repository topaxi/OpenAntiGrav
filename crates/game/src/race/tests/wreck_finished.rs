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
