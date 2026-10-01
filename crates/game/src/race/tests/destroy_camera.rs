//! The camera the player is cut to when their craft goes out - see
//! `race::destroy_camera`. The law itself is pinned in
//! `oag_render::camera::destroy`'s tests against the running original.

use super::*;
use crate::race::destroy_camera::DestroyCamera;
use oag_display::display::Fov;
use oag_physics::CraftState;
use oag_render::camera::destroy::Station;

fn station() -> Station {
    Station {
        eye: Vec3::new(400.0, 30.0, 0.0),
        aim: Vec3::new(10.0, 0.0, 0.0),
    }
}

fn with_a_station() -> Race {
    let mut race = race_with_a_grid();
    race.view.destroy_camera = DestroyCamera::new(vec![station()], 0.4);
    race
}

fn set_player(race: &mut Race, state: CraftState) {
    race.sim.world.ships[0].physics.craft_state = state;
    race.advance_destroy_camera();
}

/// State 4 cuts the picture to the station's eye; the chase camera comes back
/// when the craft is racing again.
#[test]
fn the_player_going_out_is_seen_from_the_circuits_own_camera() {
    let mut race = with_a_station();
    race.sim.world.ships[0].physics.body.position = Vec3::new(8.0, 0.0, 3.0);
    let chase_eye = race.camera_position();
    assert_ne!(chase_eye, station().eye);

    set_player(&mut race, CraftState::Destroyed);
    assert!((race.camera_position() - station().eye).length() < 1e-3);
    let wreck = race.sim.world.ships[0].physics.body.position;
    let in_view = race.view().transform_point3(wreck);
    assert!(in_view.z < 0.0, "the wreck is in front of the camera");
    set_player(&mut race, CraftState::Eliminated);
    assert!((race.camera_position() - station().eye).length() < 1e-3);

    set_player(&mut race, CraftState::Racing);
    assert!((race.camera_position() - chase_eye).length() < 1e-3);
}

/// The field is the camera's own - tens of degrees at first, easing to the
/// framing of the wreck - not the chase rig's authored one.
#[test]
fn the_destroy_camera_zooms_to_frame_the_wreck() {
    let mut race = with_a_station();
    race.sim.world.ships[0].physics.body.position = Vec3::new(8.0, 0.0, 3.0);
    let chase_fov = race.vertical_fov(16.0 / 9.0, Fov::AUTHORED);
    set_player(&mut race, CraftState::Destroyed);
    for _ in 0..300 {
        race.advance_destroy_camera();
    }
    let zoomed = race.vertical_fov(16.0 / 9.0, Fov::AUTHORED).to_degrees();
    let reach = station().eye.distance(race.sim.world.ships[0].physics.body.position);
    let framing = oag_render::camera::destroy::framing_fov_degrees(35.0, reach);
    assert!((zoomed - framing).abs() < 0.01, "{zoomed} vs {framing}");
    assert!(zoomed < chase_fov.to_degrees() / 4.0);
}

/// A circuit with no cameras leaves the chase camera alone.
#[test]
fn a_circuit_with_no_cameras_keeps_the_chase_camera() {
    let mut race = race_with_a_grid();
    let chase_eye = race.camera_position();
    set_player(&mut race, CraftState::Destroyed);
    assert!((race.camera_position() - chase_eye).length() < 1e-3);
}

/// An opponent's wreck does not take the player's camera.
#[test]
fn an_opponents_wreck_leaves_the_players_camera_alone() {
    let mut race = with_a_station();
    let chase_eye = race.camera_position();
    race.sim.world.ships[3].physics.craft_state = CraftState::Destroyed;
    race.advance_destroy_camera();
    assert!((race.camera_position() - chase_eye).length() < 1e-3);
}

/// The race's own tick starts it, not only a direct call: the wiring a test
/// of [`Race::advance_destroy_camera`] alone would miss.
#[test]
fn a_tick_with_the_player_destroyed_cuts_to_the_circuits_camera() {
    let mut race = with_a_station();
    race.force_destroy(0);
    race.tick(&oag_gameplay::PlayerInputs::none());
    assert!((race.camera_position() - station().eye).length() < 1e-3);
}

/// The player's two explosion shakes: `(0.3, 0.4)` on the state 5 edge,
/// `(0.8, 0.6)` with the big explosion, and none for an opponent.
#[test]
fn the_players_explosions_shake_their_camera() {
    let mut race = with_a_station();
    race.advance_craft_flashes();
    assert!(!race.view.shake.active());
    race.sim.world.ships[0].physics.craft_state = CraftState::Destroyed;
    race.advance_craft_flashes();
    race.sim.world.ships[0].physics.craft_state = CraftState::Eliminated;
    race.advance_craft_flashes();
    assert!(race.view.shake.active());
    assert!((race.view.shake.magnitude() - 0.3).abs() < 1e-6);
    for _ in 0..92 {
        race.view.shake.advance(1.0 / 60.0);
        race.advance_craft_flashes();
    }
    assert!((race.view.shake.magnitude() - 0.8).abs() < 1e-6, "state 6's own");

    let mut other = with_a_station();
    other.sim.world.ships[3].physics.craft_state = CraftState::Destroyed;
    other.advance_craft_flashes();
    other.sim.world.ships[3].physics.craft_state = CraftState::Eliminated;
    other.advance_craft_flashes();
    assert!(!other.view.shake.active(), "an opponent's blast shakes nobody");
}

/// State 4 cancels a shake already running, as `Ship_SetState` case 4 does.
#[test]
fn destruction_cancels_a_running_shake() {
    let mut race = with_a_station();
    race.force_shake(1.0);
    assert!(race.view.shake.active());
    race.advance_craft_flashes();
    race.sim.world.ships[0].physics.craft_state = CraftState::Destroyed;
    race.advance_craft_flashes();
    assert!(!race.view.shake.active());
}
