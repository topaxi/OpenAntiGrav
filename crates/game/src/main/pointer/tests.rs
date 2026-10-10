use super::*;
use winit::dpi::PhysicalPosition;

fn touch(id: u64, phase: TouchPhase, x: f64, y: f64) -> Touch {
    Touch {
        device_id: winit::event::DeviceId::dummy(),
        phase,
        location: PhysicalPosition::new(x, y),
        force: None,
        id,
    }
}

#[test]
fn a_click_between_ticks_is_delivered_once_on_the_next() {
    let mut window = Window::default();
    window.cursor_moved(10.0, 20.0);
    window.button(MouseButton::Left, ElementState::Pressed);
    window.button(MouseButton::Left, ElementState::Released);
    let tick = window.take();
    assert_eq!(tick.at, Some((10.0, 20.0)));
    assert!(tick.moved && tick.clicked && !tick.back);
    let next = window.take();
    assert_eq!(next.at, Some((10.0, 20.0)), "the position persists");
    assert!(next.is_idle(), "the click does not: {next:?}");
}

#[test]
fn the_right_button_is_back_and_a_release_alone_is_nothing() {
    let mut window = Window::default();
    window.button(MouseButton::Right, ElementState::Released);
    assert!(window.take().is_idle());
    window.button(MouseButton::Right, ElementState::Pressed);
    assert!(window.take().back);
}

#[test]
fn a_wheel_up_moves_the_cursor_up_and_pixels_accumulate_to_detents() {
    let mut window = Window::default();
    window.wheel(MouseScrollDelta::LineDelta(0.0, 1.0));
    assert_eq!(window.take().scroll, -1);
    window.wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
        0.0, -30.0,
    )));
    assert_eq!(window.take().scroll, 0, "not a whole detent yet");
    window.wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
        0.0, -30.0,
    )));
    assert_eq!(window.take().scroll, 1, "the remainder was carried");
}

#[test]
fn a_touch_down_reports_where_the_finger_landed_on_that_tick_only() {
    let mut window = Window::default();
    window.touch(touch(1, TouchPhase::Started, 100.0, 50.0));
    window.touch(touch(1, TouchPhase::Moved, 160.0, 50.0));
    let tick = window.take();
    assert_eq!(tick.press_at, Some((100.0, 50.0)), "not where it moved to");
    assert_eq!(window.take().press_at, None);
}

#[test]
fn a_tap_reports_on_lift_as_a_move_and_a_click_and_then_is_nowhere() {
    let mut window = Window::default();
    window.touch(touch(1, TouchPhase::Started, 100.0, 50.0));
    let tick = window.take();
    assert_eq!(tick.at, None, "nothing is decided at touch-down");
    assert!(tick.pressed && !tick.moved && !tick.clicked, "{tick:?}");
    window.touch(touch(1, TouchPhase::Moved, 104.0, 52.0));
    assert!(window.take().is_idle(), "within the threshold");
    window.touch(touch(1, TouchPhase::Ended, 104.0, 52.0));
    let tick = window.take();
    assert_eq!(tick.at, Some((104.0, 52.0)));
    assert!(tick.moved && tick.clicked && tick.released);
    let after = window.take();
    assert_eq!(after.at, None);
    assert!(after.moved && !after.clicked, "{after:?}");
}

#[test]
fn a_drag_past_the_threshold_scrolls_and_never_clicks() {
    let mut window = Window::default();
    window.touch(touch(1, TouchPhase::Started, 100.0, 100.0));
    window.touch(touch(1, TouchPhase::Moved, 100.0, 80.0));
    let tick = window.take();
    assert!(tick.pressed);
    assert_eq!(tick.drag, (0.0, -20.0), "the whole travel, once past it");
    assert_eq!(tick.at, None);
    assert!(!tick.clicked && !tick.moved);
    // Coming back inside the threshold does not turn it into a tap.
    window.touch(touch(1, TouchPhase::Moved, 100.0, 95.0));
    assert_eq!(window.take().drag, (0.0, 15.0));
    window.touch(touch(1, TouchPhase::Ended, 100.0, 98.0));
    let tick = window.take();
    assert!(
        tick.released && !tick.clicked && !tick.moved && tick.at.is_none(),
        "a lifted drag is not a tap: {tick:?}"
    );
    assert_eq!(tick.drag, (0.0, 3.0), "the travel up to the lift counts");
}

#[test]
fn a_cancelled_touch_is_nothing() {
    let mut window = Window::default();
    window.touch(touch(1, TouchPhase::Started, 100.0, 100.0));
    window.take();
    window.touch(touch(1, TouchPhase::Cancelled, 100.0, 100.0));
    let tick = window.take();
    assert!(tick.released && !tick.clicked && tick.at.is_none(), "{tick:?}");
    assert!(window.take().is_idle());
}

#[test]
fn touch_mode_lasts_until_another_device_speaks() {
    let mut window = Window::default();
    assert!(!window.touch_spoke_last());
    window.touch(touch(1, TouchPhase::Started, 100.0, 50.0));
    window.touch(touch(1, TouchPhase::Ended, 100.0, 50.0));
    window.take();
    assert!(window.touch_spoke_last(), "a lifted finger is still touch");
    window.other_device();
    assert!(!window.touch_spoke_last());
    window.touch(touch(2, TouchPhase::Started, 100.0, 50.0));
    window.cursor_moved(1.0, 2.0);
    assert!(!window.touch_spoke_last(), "the mouse takes it back");
}

#[test]
fn only_the_mouse_draws_a_cursor_and_it_returns_when_the_mouse_moves() {
    let mut window = Window::default();
    assert_eq!(window.cursor_at(), None, "nothing has spoken yet");
    window.cursor_moved(10.0, 20.0);
    assert_eq!(window.cursor_at(), Some((10.0, 20.0)));
    // A finger is its own pointer: the models see it, the arrow does not.
    window.touch(touch(1, TouchPhase::Started, 100.0, 50.0));
    assert_eq!(window.cursor_at(), None);
    window.touch(touch(1, TouchPhase::Ended, 100.0, 50.0));
    assert_eq!(window.take().at, Some((100.0, 50.0)));
    assert_eq!(window.cursor_at(), None);
    window.cursor_moved(12.0, 22.0);
    assert_eq!(window.cursor_at(), Some((12.0, 22.0)));
    // A key or a pad hides it where it is; the position is kept, so the
    // models still know where the mouse is, and a wheel turn or a click
    // brings the arrow back as much as a move does.
    window.other_device();
    assert_eq!(window.cursor_at(), None);
    assert_eq!(window.take().at, Some((12.0, 22.0)));
    window.wheel(MouseScrollDelta::LineDelta(0.0, 1.0));
    assert_eq!(window.cursor_at(), Some((12.0, 22.0)));
    window.other_device();
    window.button(MouseButton::Left, ElementState::Pressed);
    assert_eq!(window.cursor_at(), Some((12.0, 22.0)));
}

#[test]
fn a_second_finger_is_ignored_while_the_first_is_down() {
    let mut window = Window::default();
    window.touch(touch(1, TouchPhase::Started, 100.0, 50.0));
    window.take();
    window.touch(touch(2, TouchPhase::Started, 300.0, 200.0));
    window.touch(touch(2, TouchPhase::Moved, 310.0, 200.0));
    let tick = window.take();
    assert!(tick.is_idle(), "{tick:?}");
    window.touch(touch(2, TouchPhase::Ended, 310.0, 200.0));
    assert!(window.take().is_idle(), "the second finger never counted");
    window.touch(touch(1, TouchPhase::Ended, 100.0, 50.0));
    assert_eq!(window.take().at, Some((100.0, 50.0)), "still the first");
}

#[test]
fn losing_focus_drops_the_pending_presses_but_not_the_position() {
    let mut window = Window::default();
    window.cursor_moved(10.0, 20.0);
    window.button(MouseButton::Left, ElementState::Pressed);
    window.wheel(MouseScrollDelta::LineDelta(0.0, 3.0));
    window.release();
    let tick = window.take();
    assert_eq!(tick.at, Some((10.0, 20.0)));
    assert!(!tick.clicked && tick.scroll == 0);
}

/// The live failure this exists for: a flick's lift and a tap that
/// should catch the coast arrived in one tick behind a stalled frame,
/// and read as one gesture whose tap selected.
#[test]
fn a_touch_after_an_untaken_lift_waits_for_the_next_tick() {
    let mut window = Window::default();
    window.touch(touch(1, TouchPhase::Started, 100.0, 100.0));
    window.touch(touch(1, TouchPhase::Moved, 40.0, 100.0));
    window.take();
    window.touch(touch(1, TouchPhase::Ended, 30.0, 100.0));
    window.touch(touch(2, TouchPhase::Started, 200.0, 50.0));
    window.touch(touch(2, TouchPhase::Ended, 200.0, 50.0));
    let lift = window.take();
    assert!(lift.released && !lift.pressed && !lift.clicked, "{lift:?}");
    let tap = window.take();
    assert!(tap.pressed && tap.released && tap.clicked, "{tap:?}");
    assert_eq!(tap.at, Some((200.0, 50.0)));
    assert!(window.take().at.is_none());
}

#[test]
fn losing_focus_with_a_finger_down_lifts_it() {
    let mut window = Window::default();
    window.touch(touch(1, TouchPhase::Started, 100.0, 100.0));
    window.touch(touch(1, TouchPhase::Moved, 100.0, 60.0));
    window.take();
    window.release();
    let tick = window.take();
    assert!(tick.released && !tick.clicked, "{tick:?}");
    assert_eq!(tick.drag, (0.0, 0.0));
}
