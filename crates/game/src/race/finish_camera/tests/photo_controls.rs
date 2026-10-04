//! The d-pad on `Race End Photo`: `Camera_CycleModeForward` / `Camera_CycleModeBack`'s tables,
//! the width they leave alone, and the craft switch.

use super::*;

fn running(field: &[Subject]) -> FinishCamera {
    let mut camera = director(3);
    camera.step(START_TICKS, field, 0, None);
    camera
}

#[test]
fn nothing_cycles_before_the_director_runs() {
    let mut camera = director(3);
    assert!(!camera.cycle(true));
    assert!(!camera.watch_step(true, 3, &[craft(0, 0.0)]));
}

#[test]
fn up_steps_one_four_three_two_seven_and_down_steps_back() {
    let mut camera = running(&[craft(0, 5.0)]);
    assert_eq!(camera.mode(), Some(ViewMode::Track));
    let mut seen = Vec::new();
    for _ in 0..5 {
        assert!(camera.cycle(true));
        seen.push(camera.mode().expect("running"));
    }
    assert_eq!(
        seen,
        [
            ViewMode::Nose,
            ViewMode::Chase,
            ViewMode::Front,
            ViewMode::Rear,
            ViewMode::Track
        ]
    );
    let mut back = Vec::new();
    for _ in 0..5 {
        camera.cycle(false);
        back.push(camera.mode().expect("running"));
    }
    assert_eq!(
        back,
        [
            ViewMode::Rear,
            ViewMode::Front,
            ViewMode::Chase,
            ViewMode::Nose,
            ViewMode::Track
        ]
    );
}

#[test]
fn mode_six_leaves_like_seven_and_the_death_camera_has_no_case() {
    let mut camera = running(&[craft(0, 5.0)]);
    camera.mode = ViewMode::Close;
    camera.cycle(true);
    assert_eq!(camera.mode, ViewMode::Nose);
    camera.mode = ViewMode::Close;
    camera.cycle(false);
    assert_eq!(camera.mode, ViewMode::Rear);
    camera.mode = ViewMode::Death;
    camera.cycle(true);
    camera.cycle(false);
    assert_eq!(camera.mode, ViewMode::Death);
}

#[test]
fn the_cycle_keeps_the_width_the_last_node_mode_set() {
    let mut camera = running(&[craft(0, 5.0)]);
    camera.mode = ViewMode::Close;
    camera.width = 17.0;
    // 6 -> 2 -> 7: the mode word is stored directly, `Camera_SetMode` is not called.
    camera.cycle(false);
    camera.cycle(true);
    assert_eq!(camera.mode, ViewMode::Track);
    assert_eq!(
        camera.width, 17.0,
        "Track entered by the d-pad keeps Close's width"
    );
}

#[test]
fn right_and_left_watch_the_next_and_previous_slot_on_its_nearest_node() {
    let field = [craft(0, 5.0), craft(1, 195.0), craft(2, 105.0)];
    let mut camera = running(&field);
    camera.mode = ViewMode::Close;
    camera.width = 50.0;
    assert!(camera.watch_step(true, 3, &field));
    assert_eq!(camera.subject(), Some(1));
    assert_eq!(
        camera.drawn_slot(),
        Some(1),
        "the craft shown is the new one at once"
    );
    assert_eq!(
        camera.node,
        Some(2),
        "the node whose aim is nearest x = 195"
    );
    assert_eq!(camera.mode, ViewMode::Close, "the mode is kept");
    assert_eq!(
        camera.width, 17.0,
        "and re-applied, so a node mode's width is set"
    );
    camera.watch_step(true, 3, &field);
    assert_eq!(camera.subject(), Some(2));
    camera.watch_step(true, 3, &field);
    assert_eq!(camera.subject(), Some(0), "wraps past the last slot");
    camera.watch_step(false, 3, &field);
    assert_eq!(camera.subject(), Some(2), "and back past the first");
}

#[test]
fn the_nose_and_chase_views_ride_on_the_craft_at_65_degrees() {
    let field = [craft(0, 5.0)];
    let mut camera = running(&field);
    camera.cycle(true);
    let nose = camera.step(START_TICKS + 1, &field, 0, None).expect("pose");
    assert_eq!(nose.fov_deg, Some(craft_view::FOV_DEGREES));
    assert!(
        (nose.eye - Vec3::new(5.0, 0.0, -5.0)).length() < 1e-5,
        "{:?}",
        nose.eye
    );
    camera.cycle(true);
    let chase = camera.step(START_TICKS + 2, &field, 0, None).expect("pose");
    assert!(
        (chase.eye - Vec3::new(5.0, 3.0, 12.0)).length() < 1e-5,
        "{:?}",
        chase.eye
    );
}
