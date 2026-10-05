//! `Race::spectator_press`: the d-pad on `Race End Photo` reaches the director after a line
//! finish, and never after a wreck (the original has no photo state then).

use super::*;
use crate::finish_camera::{FinishCamera, SPECTATOR_SEED, START_TICKS, Subject, ViewMode};
use oag_gameplay::input::Button;

fn a_finished_race_with_a_running_director() -> Race {
    let mut race = race_with_a_grid();
    race.sim.world.ships[0].standing.finish_tick = Some(0);
    let mut director = FinishCamera::new(
        vec![oag_render::camera::destroy::Station {
            eye: Vec3::new(400.0, 30.0, 0.0),
            aim: Vec3::new(10.0, 0.0, 0.0),
        }],
        0.4,
        SPECTATOR_SEED,
    );
    let me = Subject {
        slot: 0,
        position: Vec3::ZERO,
        orientation: oag_core::math::Quat::IDENTITY,
    };
    director.step(START_TICKS, &[me], 0, None);
    race.view.finish_camera = Some(director);
    race
}

#[test]
fn up_and_down_step_the_directors_mode_after_a_line_finish() {
    let mut race = a_finished_race_with_a_running_director();
    assert_eq!(race.spectator_mode(), Some(ViewMode::Track));
    assert!(race.spectator_press(Button::Up));
    assert_eq!(race.spectator_mode(), Some(ViewMode::Nose));
    assert!(race.spectator_press(Button::Down));
    assert_eq!(race.spectator_mode(), Some(ViewMode::Track));
    assert!(
        !race.spectator_press(Button::Cross),
        "only the d-pad drives it"
    );
}

#[test]
fn right_watches_the_next_slot() {
    let mut race = a_finished_race_with_a_running_director();
    assert!(race.spectator_press(Button::Right));
    let watched = race
        .view
        .finish_camera
        .as_ref()
        .and_then(FinishCamera::subject);
    assert_eq!(watched, Some(1));
}

#[test]
fn the_d_pad_does_nothing_before_the_line() {
    let mut race = a_finished_race_with_a_running_director();
    race.sim.world.ships[0].standing.finish_tick = None;
    assert!(!race.spectator_press(Button::Up));
    assert_eq!(race.spectator_mode(), Some(ViewMode::Track));
}
