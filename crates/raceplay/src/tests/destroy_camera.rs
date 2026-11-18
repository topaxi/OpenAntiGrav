//! The camera the player is cut to when their craft goes out - see
//! `race::destroy_camera`. The law itself is pinned in
//! `oag_render::camera::destroy`'s tests against the running original.

use super::*;
use crate::destroy_camera::DestroyCamera;
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
    race.core.sim.world.ships[0].physics.craft_state = state;
    race.view.advance_destroy_camera(&race.core);
}

/// State 4 cuts the picture to the station's eye; the chase camera comes back
/// when the craft is racing again.
#[test]
fn the_player_going_out_is_seen_from_the_circuits_own_camera() {
    let mut race = with_a_station();
    race.core.sim.world.ships[0].physics.body.position = Vec3::new(8.0, 0.0, 3.0);
    let chase_eye = race.view.camera_position(&race.core);
    assert_ne!(chase_eye, station().eye);

    set_player(&mut race, CraftState::Destroyed);
    assert!((race.view.camera_position(&race.core) - station().eye).length() < 1e-3);
    let wreck = race.core.sim.world.ships[0].physics.body.position;
    let in_view = race.view.view(&race.core).transform_point3(wreck);
    assert!(in_view.z < 0.0, "the wreck is in front of the camera");
    set_player(&mut race, CraftState::Eliminated);
    assert!((race.view.camera_position(&race.core) - station().eye).length() < 1e-3);

    set_player(&mut race, CraftState::Racing);
    assert!((race.view.camera_position(&race.core) - chase_eye).length() < 1e-3);
}

/// The field is the camera's own - tens of degrees at first, easing to the
/// framing of the wreck - not the chase rig's authored one.
#[test]
fn the_destroy_camera_zooms_to_frame_the_wreck() {
    let mut race = with_a_station();
    race.core.sim.world.ships[0].physics.body.position = Vec3::new(8.0, 0.0, 3.0);
    let chase_fov = race
        .view
        .vertical_fov(&race.core, 16.0 / 9.0, Fov::AUTHORED);
    set_player(&mut race, CraftState::Destroyed);
    for _ in 0..300 {
        race.view.advance_destroy_camera(&race.core);
    }
    let zoomed = race
        .view
        .vertical_fov(&race.core, 16.0 / 9.0, Fov::AUTHORED)
        .to_degrees();
    let reach = station()
        .eye
        .distance(race.core.sim.world.ships[0].physics.body.position);
    let framing = oag_render::camera::destroy::framing_fov_degrees(35.0, reach);
    assert!((zoomed - framing).abs() < 0.01, "{zoomed} vs {framing}");
    assert!(zoomed < chase_fov.to_degrees() / 4.0);
}

/// A circuit with no cameras leaves the chase camera alone.
#[test]
fn a_circuit_with_no_cameras_keeps_the_chase_camera() {
    let mut race = race_with_a_grid();
    let chase_eye = race.view.camera_position(&race.core);
    set_player(&mut race, CraftState::Destroyed);
    assert!((race.view.camera_position(&race.core) - chase_eye).length() < 1e-3);
}

/// An opponent's wreck does not take the player's camera.
#[test]
fn an_opponents_wreck_leaves_the_players_camera_alone() {
    let mut race = with_a_station();
    let chase_eye = race.view.camera_position(&race.core);
    race.core.sim.world.ships[3].physics.craft_state = CraftState::Destroyed;
    race.view.advance_destroy_camera(&race.core);
    assert!((race.view.camera_position(&race.core) - chase_eye).length() < 1e-3);
}

/// The race's own tick starts it, not only a direct call: the wiring a test
/// of [`RaceView::advance_destroy_camera`] alone would miss.
#[test]
fn a_tick_with_the_player_destroyed_cuts_to_the_circuits_camera() {
    let mut race = with_a_station();
    race.core.sim.force_destroy(0);
    race.tick(&oag_gameplay::PlayerInputs::none());
    assert!((race.view.camera_position(&race.core) - station().eye).length() < 1e-3);
}

/// The player's two explosion shakes in a race that goes on: `(0.3, 0.4)` on
/// the state 5 edge, `(0.8, 0.6)` with the big explosion, and none for an
/// opponent.
#[test]
fn the_players_explosions_shake_their_camera() {
    let mut race =
        race_with_weapon_table(Mode::Eliminator, enveloping_pad(), 1.0, one_mine_table());
    race.view.advance_craft_flashes(&race.core);
    assert!(!race.view.contact.shake.active());
    race.core.sim.world.ships[0].physics.craft_state = CraftState::Destroyed;
    race.view.advance_craft_flashes(&race.core);
    race.core.sim.world.ships[0].physics.craft_state = CraftState::Eliminated;
    race.view.advance_craft_flashes(&race.core);
    assert!(race.view.contact.shake.active());
    assert!((race.view.contact.shake.magnitude() - 0.3).abs() < 1e-6);
    for _ in 0..92 {
        race.view.contact.shake.advance(1.0 / 60.0);
        race.view.advance_craft_flashes(&race.core);
    }
    assert!(
        (race.view.contact.shake.magnitude() - 0.8).abs() < 1e-6,
        "state 6's own"
    );

    let mut other = with_a_station();
    other.core.sim.world.ships[3].physics.craft_state = CraftState::Destroyed;
    other.view.advance_craft_flashes(&other.core);
    other.core.sim.world.ships[3].physics.craft_state = CraftState::Eliminated;
    other.view.advance_craft_flashes(&other.core);
    assert!(
        !other.view.contact.shake.active(),
        "an opponent's blast shakes nobody"
    );
}

/// Where the race ends on the state 5 edge the first shake is armed too: the
/// finished race steps its cosmetics, so it decays instead of holding at its
/// largest frame (`wreck_finished` pins the playing out).
#[test]
fn a_race_that_ends_on_the_edge_is_shaken_like_any_other() {
    let mut race = with_a_station();
    assert_ne!(race.core.sim.world.mode(), Mode::Eliminator);
    race.core.sim.world.ships[0].physics.craft_state = CraftState::Destroyed;
    race.view.advance_craft_flashes(&race.core);
    race.core.sim.world.ships[0].physics.craft_state = CraftState::Eliminated;
    race.view.advance_craft_flashes(&race.core);
    assert!((race.view.contact.shake.magnitude() - 0.3).abs() < 1e-6);
}

/// State 4 cancels a shake already running, as `Ship_SetState` case 4 does.
#[test]
fn destruction_cancels_a_running_shake() {
    let mut race = with_a_station();
    race.force_shake(1.0);
    assert!(race.view.contact.shake.active());
    race.view.advance_craft_flashes(&race.core);
    race.core.sim.world.ships[0].physics.craft_state = CraftState::Destroyed;
    race.view.advance_craft_flashes(&race.core);
    assert!(!race.view.contact.shake.active());
}

/// The cut counter the temporal upscaler watches moves when the camera takes
/// over and when it lets go, and not while it runs.
#[test]
fn the_camera_taking_over_and_letting_go_are_cuts() {
    let mut race = with_a_station();
    let before = race.view.camera_cuts(&race.core);
    set_player(&mut race, CraftState::Destroyed);
    assert_eq!(race.view.camera_cuts(&race.core), before + 1);
    set_player(&mut race, CraftState::Eliminated);
    assert_eq!(race.view.camera_cuts(&race.core), before + 1);
    set_player(&mut race, CraftState::Racing);
    assert_eq!(race.view.camera_cuts(&race.core), before + 2);
}
