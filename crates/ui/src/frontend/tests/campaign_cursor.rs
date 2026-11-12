//! The right stick's cursor on the 2048 campaign map.

use super::wipeout2048::{
    boot, pass_event_one, press, reach_the_grid, reach_the_shell, three_events, tick,
};
use super::*;

fn on_the_map() -> (Frontend, Input) {
    let mut frontend = boot(0);
    frontend.set_campaign(three_events());
    pass_event_one(&mut frontend);
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    (frontend, input)
}

fn push(frontend: &mut Frontend, input: &mut Input, stick: (f32, f32)) {
    frontend.set_view_stick(stick);
    tick(frontend, input, 0);
}

fn centre_of(frontend: &Frontend, index: usize) -> (f32, f32) {
    let face = frontend.campaign.face_of(index).unwrap();
    (face[0] + face[2] * 0.5, face[1] + face[3] * 0.5)
}

fn selected_name(frontend: &Frontend) -> Option<&str> {
    frontend.selected_event().map(|e| e.name.as_str())
}

#[test]
fn no_cursor_until_the_stick_moves() {
    let (mut frontend, mut input) = on_the_map();
    assert_eq!(frontend.map_cursor(), None);
    push(&mut frontend, &mut input, (0.0, 0.0));
    assert_eq!(frontend.map_cursor(), None);
}

#[test]
fn the_stick_moves_the_cursor_off_the_selected_event_and_clamps_it_to_the_view() {
    let (mut frontend, mut input) = on_the_map();
    let start = centre_of(&frontend, frontend.campaign.selected);
    push(&mut frontend, &mut input, (1.0, 0.0));
    let first = frontend.map_cursor().expect("the cursor is up");
    assert!(first.0 > start.0, "right pushes right: {first:?} {start:?}");
    // Up is positive on the stick, and up the screen is a smaller y.
    push(&mut frontend, &mut input, (0.0, 1.0));
    let up = frontend.map_cursor().unwrap();
    assert!(up.1 < first.1);
    for _ in 0..600 {
        push(&mut frontend, &mut input, (1.0, -1.0));
    }
    let (width, height) = frontend.space().size;
    let end = frontend.map_cursor().unwrap();
    // Held short of the edges by the arrow's own size, never past them.
    assert!(end.0 > width * 0.9 && end.0 < width, "{end:?}");
    assert!(end.1 > height * 0.9 && end.1 < height, "{end:?}");
}

#[test]
fn a_small_push_is_slower_than_a_full_one() {
    let (mut frontend, mut input) = on_the_map();
    push(&mut frontend, &mut input, (0.25, 0.0));
    let small = frontend.map_cursor().unwrap().0;
    let (mut other, mut other_input) = on_the_map();
    push(&mut other, &mut other_input, (1.0, 0.0));
    let full = other.map_cursor().unwrap().0;
    let start = centre_of(&frontend, frontend.campaign.selected).0;
    assert!(small - start > 0.0 && (full - start) > 4.0 * (small - start));
}

#[test]
fn pushing_into_an_edge_scrolls_the_map_within_its_limits() {
    let (mut frontend, mut input) = on_the_map();
    let before = frontend.campaign.scroll();
    for _ in 0..240 {
        push(&mut frontend, &mut input, (1.0, 0.0));
    }
    let after = frontend.campaign.scroll();
    assert!(after.0 > before.0, "the view moved right: {before:?} {after:?}");
    assert_eq!(after.1, before.1, "and only right");
    let (_, max) = frontend.campaign.limits(0);
    for _ in 0..2400 {
        push(&mut frontend, &mut input, (1.0, 0.0));
    }
    assert!(frontend.campaign.scroll().0 <= max);
    for _ in 0..4800 {
        push(&mut frontend, &mut input, (-1.0, 0.0));
    }
    let (min, _) = frontend.campaign.limits(0);
    assert!(frontend.campaign.scroll().0 >= min);
}

#[test]
fn confirm_opens_the_event_under_the_cursor_even_one_the_pad_steps_skip() {
    let (mut frontend, mut input) = on_the_map();
    assert_eq!(selected_name(&frontend), Some("2048 - Event 1"));
    // Steer the cursor onto the third event, panning as it goes.
    for _ in 0..3000 {
        let target = centre_of(&frontend, 2);
        let at = frontend.campaign.cursor.at.unwrap_or_else(|| centre_of(&frontend, 0));
        let (dx, dy) = (target.0 - at.0, target.1 - at.1);
        let length = dx.hypot(dy).max(1.0);
        if frontend.campaign.hit_at(at) == Some(2) {
            break;
        }
        push(&mut frontend, &mut input, (dx / length, -dy / length));
    }
    frontend.set_view_stick((0.0, 0.0));
    assert_eq!(selected_name(&frontend), Some("2049 - Event 1"));
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.event_card_open());
    assert_eq!(selected_name(&frontend), Some("2049 - Event 1"));
}

#[test]
fn confirm_on_empty_map_with_the_cursor_up_opens_nothing() {
    let (mut frontend, mut input) = on_the_map();
    frontend.campaign.cursor.at = Some((1.0, 1.0));
    press(&mut frontend, &mut input, Button::Cross);
    assert!(!frontend.event_card_open());
}

#[test]
fn a_rank_gated_event_is_never_under_the_cursor() {
    let mut events = three_events();
    events[2].rank_gated = true;
    let mut frontend = boot(0);
    frontend.set_campaign(events);
    pass_event_one(&mut frontend);
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    assert!(frontend.campaign.hidden(2));
    let centre = centre_of(&frontend, 2);
    assert_eq!(frontend.campaign.hit_at(centre), None);
    frontend.campaign.cursor.at = Some(centre);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(!frontend.event_card_open());
    assert_eq!(selected_name(&frontend), Some("2048 - Event 1"));
}

#[test]
fn without_the_cursor_confirm_still_opens_the_selected_event() {
    let (mut frontend, mut input) = on_the_map();
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.event_card_open());
}

#[test]
fn the_cursor_goes_after_a_pause_a_dpad_press_or_a_mouse() {
    let (mut frontend, mut input) = on_the_map();
    push(&mut frontend, &mut input, (1.0, 0.0));
    assert!(frontend.map_cursor().is_some());
    for _ in 0..200 {
        push(&mut frontend, &mut input, (0.0, 0.0));
    }
    assert_eq!(frontend.map_cursor(), None, "three seconds without a push");

    push(&mut frontend, &mut input, (1.0, 0.0));
    push(&mut frontend, &mut input, (0.0, 0.0));
    press(&mut frontend, &mut input, Button::Right);
    assert_eq!(frontend.map_cursor(), None);

    push(&mut frontend, &mut input, (1.0, 0.0));
    frontend.set_pad_driving(false);
    assert_eq!(frontend.map_cursor(), None);
}

#[test]
fn the_stick_does_nothing_off_the_campaign_map() {
    let mut frontend = boot(0);
    frontend.set_campaign(three_events());
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    let selected = frontend.touch.selected;
    for _ in 0..30 {
        push(&mut frontend, &mut input, (1.0, 1.0));
    }
    assert_eq!(frontend.map_cursor(), None);
    assert_eq!(frontend.campaign.cursor.at, None);
    assert_eq!(frontend.touch.selected, selected);
    assert!(!frontend.wants_mouse_pan());
}
