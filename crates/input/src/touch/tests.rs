use super::*;

const SIZE: (f32, f32) = (2400.0, 1080.0);

fn centre(control: Control) -> (f32, f32) {
    layout(SIZE)
        .into_iter()
        .find(|(c, _)| *c == control)
        .map(|(_, r)| r.centre())
        .expect("every control has a rectangle")
}

#[test]
fn every_control_sits_inside_the_window_and_none_overlap() {
    let rects = layout(SIZE);
    for (control, r) in rects {
        assert!(r.x >= 0.0 && r.y >= 0.0, "{control:?} starts off screen");
        assert!(
            r.x + r.w <= SIZE.0 && r.y + r.h <= SIZE.1,
            "{control:?} runs off"
        );
    }
    for (i, (a, ra)) in rects.iter().enumerate() {
        for (b, rb) in &rects[i + 1..] {
            let apart = ra.x + ra.w <= rb.x
                || rb.x + rb.w <= ra.x
                || ra.y + ra.h <= rb.y
                || rb.y + rb.h <= ra.y;
            assert!(apart, "{a:?} overlaps {b:?}");
        }
    }
}

#[test]
fn a_control_inside_the_stick_zone_is_a_button_not_the_stick() {
    let mut touches = Touches::default();
    touches.down(1, centre(Control::AirbrakeLeft), SIZE);
    assert!(touches.stick().is_none());
    assert_eq!(touches.reading(SIZE).buttons, Button::L.bit());
    touches.down(2, (300.0, 800.0), SIZE);
    assert!(touches.stick().is_some(), "the stick is still free below");
}

#[test]
fn every_control_keeps_to_the_safe_margins() {
    for (control, r) in layout(SIZE) {
        assert!(r.x >= SAFE_X * SIZE.1 - 0.01, "{control:?} left");
        assert!(
            r.x + r.w <= SIZE.0 - SAFE_X * SIZE.1 + 0.01,
            "{control:?} right"
        );
        assert!(r.y >= SAFE_Y * SIZE.1 - 0.01, "{control:?} top");
        assert!(r.y + r.h <= SIZE.1 * (1.0 - SAFE_Y), "{control:?} bottom");
    }
}

#[test]
fn every_control_is_thumb_sized_by_the_window_height() {
    for (control, r) in layout(SIZE) {
        // The small round buttons are 0.13 of the height drawn; the slop adds
        // 0.025 on every side, so a thumb has at least 0.18 to land on.
        assert!(r.w >= 0.13 * SIZE.1 && r.h >= 0.13 * SIZE.1, "{control:?}");
    }
    let tall = layout((1000.0, 500.0))[0].1;
    let wide = layout((2000.0, 500.0))[0].1;
    assert_eq!(tall.w, wide.w, "the size follows the height, not the width");
}

fn go_at(fx: f32, fy: f32) -> (f32, f32) {
    let r = layout(SIZE)[Control::Accelerate.index()].1;
    (r.x + r.w * fx, r.y + r.h * fy)
}

fn live() -> Touches {
    let mut touches = Touches::default();
    touches.set_go_zones(true, SIZE);
    touches
}

#[test]
fn go_is_thrust_alone_in_the_centre_and_above() {
    for (fx, fy) in [(0.5, 0.95), (0.1, 0.3), (0.9, 0.3), (0.5, 0.1), (0.2, 0.6)] {
        let mut touches = live();
        touches.down(1, go_at(fx, fy), SIZE);
        assert_eq!(
            touches.reading(SIZE).buttons,
            Button::Cross.bit(),
            "{fx},{fy}"
        );
    }
}

#[test]
fn go_adds_the_left_or_right_brake_in_its_bottom_corners() {
    let mut touches = live();
    touches.down(1, go_at(0.1, 0.9), SIZE);
    assert_eq!(
        touches.reading(SIZE).buttons,
        Button::Cross.bit() | Button::L.bit()
    );
    let mut touches = live();
    touches.down(1, go_at(0.9, 0.9), SIZE);
    assert_eq!(
        touches.reading(SIZE).buttons,
        Button::Cross.bit() | Button::R.bit()
    );
}

#[test]
fn sliding_inside_go_changes_the_zone_without_lifting() {
    let mut touches = live();
    touches.down(1, go_at(0.5, 0.5), SIZE);
    touches.moved(1, go_at(0.1, 0.9), SIZE);
    assert!(touches.zone_down(GoZone::Left));
    touches.moved(1, go_at(0.9, 0.9), SIZE);
    assert!(!touches.zone_down(GoZone::Left) && touches.zone_down(GoZone::Right));
    touches.moved(1, go_at(0.5, 0.9), SIZE);
    assert_eq!(touches.reading(SIZE).buttons, Button::Cross.bit());
}

#[test]
fn a_thumb_in_the_slop_below_go_still_reads_the_corner() {
    let mut touches = live();
    let r = layout(SIZE)[0].1;
    touches.down(1, (r.x + 5.0, r.y + r.h + SIZE.1 * SLOP * 0.5), SIZE);
    assert!(touches.zone_down(GoZone::Left));
}

#[test]
fn the_zones_are_off_when_the_setting_is() {
    let mut touches = Touches::default();
    touches.set_go_zones(false, SIZE);
    touches.down(1, go_at(0.1, 0.9), SIZE);
    assert_eq!(touches.reading(SIZE).buttons, Button::Cross.bit());
    touches.set_go_zones(true, SIZE);
    assert!(touches.zone_down(GoZone::Left), "turned on mid-hold");
}

#[test]
fn steering_and_go_with_a_brake_are_two_fingers_at_once() {
    let mut touches = live();
    touches.down(1, (300.0, 800.0), SIZE);
    touches.down(2, go_at(0.9, 0.9), SIZE);
    touches.moved(1, (300.0 - SIZE.1 * STICK_RADIUS, 800.0), SIZE);
    let reading = touches.reading(SIZE);
    assert!((reading.stick_x + 1.0).abs() < 1e-3);
    assert_eq!(reading.buttons, Button::Cross.bit() | Button::R.bit());
    touches.up(2, SIZE);
    assert_eq!(touches.reading(SIZE).buttons, 0);
    assert!(!touches.zone_down(GoZone::Right));
}

#[test]
fn a_zone_does_not_latch_a_tap_and_the_brake_buttons_still_work() {
    let mut touches = live();
    touches.down(1, go_at(0.1, 0.9), SIZE);
    assert_eq!(touches.take_taps(), vec![Button::Cross]);
    touches.down(2, centre(Control::AirbrakeRight), SIZE);
    assert!(touches.reading(SIZE).buttons & Button::R.bit() != 0);
}

#[test]
fn steering_and_accelerating_are_two_fingers_held_together() {
    let mut touches = Touches::default();
    touches.down(1, (300.0, 800.0), SIZE);
    touches.down(2, centre(Control::Accelerate), SIZE);
    touches.moved(1, (300.0 + SIZE.1 * STICK_RADIUS, 800.0), SIZE);
    let reading = touches.reading(SIZE);
    assert!((reading.stick_x - 1.0).abs() < 1e-3);
    assert_eq!(reading.buttons, Button::Cross.bit());
}

#[test]
fn the_stick_is_centred_where_the_finger_landed() {
    let mut touches = Touches::default();
    touches.down(1, (500.0, 700.0), SIZE);
    assert_eq!(touches.reading(SIZE).stick_x, 0.0);
    touches.moved(1, (500.0 - SIZE.1 * STICK_RADIUS / 2.0, 700.0), SIZE);
    assert!((touches.reading(SIZE).stick_x + 0.5).abs() < 1e-3);
    touches.moved(1, (500.0, 700.0 - 10_000.0), SIZE);
    assert_eq!(
        touches.reading(SIZE).stick_y,
        1.0,
        "up is positive and clamps"
    );
}

#[test]
fn a_second_finger_in_the_stick_zone_does_not_steal_the_stick() {
    let mut touches = Touches::default();
    touches.down(1, (300.0, 800.0), SIZE);
    touches.down(2, (700.0, 900.0), SIZE);
    touches.moved(2, (900.0, 900.0), SIZE);
    assert_eq!(touches.reading(SIZE).stick_x, 0.0);
    assert_eq!(touches.stick().map(|(o, _)| o), Some((300.0, 800.0)));
}

#[test]
fn lifting_a_finger_lets_go_of_what_it_held() {
    let mut touches = Touches::default();
    touches.down(2, centre(Control::AirbrakeLeft), SIZE);
    assert_eq!(touches.reading(SIZE).buttons, Button::L.bit());
    touches.up(2, SIZE);
    assert_eq!(touches.reading(SIZE), Reading::default());
    assert!(!touches.any());
}

#[test]
fn a_thumb_rolls_from_accelerate_onto_fire_without_lifting() {
    let mut touches = Touches::default();
    touches.down(1, centre(Control::Accelerate), SIZE);
    touches.moved(1, centre(Control::Fire), SIZE);
    assert_eq!(touches.reading(SIZE).buttons, Button::Square.bit());
}

#[test]
fn a_tap_shorter_than_a_tick_is_still_a_press() {
    let mut touches = Touches::default();
    touches.down(1, centre(Control::Pause), SIZE);
    touches.up(1, SIZE);
    assert_eq!(touches.reading(SIZE).buttons, 0);
    assert_eq!(touches.take_taps(), vec![Button::Start]);
    assert!(touches.take_taps().is_empty(), "the latch clears");
}

#[test]
fn a_finger_that_lands_on_nothing_stays_nothing() {
    let mut touches = Touches::default();
    touches.down(1, (SIZE.0 / 2.0, SIZE.1 / 2.0), SIZE);
    touches.moved(1, centre(Control::Accelerate), SIZE);
    assert_eq!(touches.reading(SIZE), Reading::default());
}

#[test]
fn every_control_presses_its_own_button() {
    for control in Control::ALL {
        let mut touches = Touches::default();
        touches.down(1, centre(control), SIZE);
        let held = touches.reading(SIZE).buttons;
        let taps = touches.take_taps();
        assert_eq!(taps, vec![control.button()], "{control:?} latches a tap");
        if control.is_momentary() {
            assert_eq!(held, 0, "{control:?} is an edge, never a held bit");
        } else {
            assert_eq!(held, control.button().bit(), "{control:?}");
        }
    }
}

#[test]
fn release_all_drops_fingers_and_latches() {
    let mut touches = Touches::default();
    touches.down(1, centre(Control::Fire), SIZE);
    touches.down(2, (300.0, 800.0), SIZE);
    touches.release_all();
    assert_eq!(touches.reading(SIZE), Reading::default());
    assert!(touches.take_taps().is_empty());
    assert!(touches.stick().is_none());
}
