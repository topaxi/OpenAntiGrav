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
fn no_button_sits_in_the_stick_zone_below_the_top_strip() {
    for (control, r) in layout(SIZE) {
        let in_zone = r.x < SIZE.0 * STICK_ZONE_WIDTH && r.y + r.h > SIZE.1 * TOP_STRIP;
        assert!(!in_zone, "{control:?} is where the stick lands");
    }
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
