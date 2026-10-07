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
fn go_adds_an_analogue_brake_in_its_bottom_corners() {
    let mut touches = live();
    touches.down(1, go_at(0.0, 1.0), SIZE);
    let reading = touches.reading(SIZE);
    assert_eq!(reading.buttons, Button::Cross.bit(), "an axis, not a bit");
    assert!((reading.airbrake_left - 1.0).abs() < 1e-4);
    assert_eq!(reading.airbrake_right, 0.0);
    let mut touches = live();
    touches.down(1, go_at(1.0, 1.0), SIZE);
    let reading = touches.reading(SIZE);
    assert!((reading.airbrake_right - 1.0).abs() < 1e-4);
    assert_eq!(reading.airbrake_left, 0.0);
}

#[test]
fn the_brake_is_zero_on_the_zones_inner_edge_and_top() {
    for zone in [GoZone::Left, GoZone::Right] {
        let (inner, outer) = match zone {
            GoZone::Left => (GO_ZONE_SIDE, 0.0),
            GoZone::Right => (1.0 - GO_ZONE_SIDE, 1.0),
        };
        assert_eq!(zone.strength((inner, 1.0)), 0.0, "inner edge");
        assert_eq!(zone.strength((outer, 1.0 - GO_ZONE_BOTTOM)), 0.0, "top");
        assert!((zone.strength((outer, 1.0)) - 1.0).abs() < 1e-5, "corner");
    }
}

#[test]
fn the_brake_only_grows_sliding_down_or_out() {
    let steps = [0.0f32, 0.1, 0.25, 0.5, 0.75, 1.0];
    for zone in [GoZone::Left, GoZone::Right] {
        let x_of = |depth: f32| match zone {
            GoZone::Left => GO_ZONE_SIDE * (1.0 - depth),
            GoZone::Right => 1.0 - GO_ZONE_SIDE * (1.0 - depth),
        };
        let y_of = |depth: f32| 1.0 - GO_ZONE_BOTTOM * (1.0 - depth);
        for &other in &steps {
            let mut last_out = -1.0f32;
            let mut last_down = -1.0f32;
            for &depth in &steps {
                let out = zone.strength((x_of(depth), y_of(other)));
                let down = zone.strength((x_of(other), y_of(depth)));
                assert!(
                    out >= last_out && down >= last_down,
                    "{zone:?} {depth} {other}"
                );
                (last_out, last_down) = (out, down);
            }
        }
        assert!(zone.strength((x_of(0.5), y_of(0.5))) > 0.0);
        assert!(zone.strength((x_of(0.5), y_of(0.5))) < zone.strength((x_of(1.0), y_of(1.0))));
    }
}

#[test]
fn a_partial_pull_reaches_the_reading_and_thrust_stays_full() {
    let mut touches = live();
    touches.down(1, go_at(0.2, 0.8), SIZE);
    let reading = touches.reading(SIZE);
    assert!(reading.airbrake_left > 0.0 && reading.airbrake_left < 1.0);
    assert_eq!(reading.buttons & Button::Cross.bit(), Button::Cross.bit());
    assert_eq!(reading.airbrake_left, touches.zone_strength(GoZone::Left));
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
    touches.down(2, go_at(0.95, 0.95), SIZE);
    touches.moved(1, (300.0 - SIZE.1 * STICK_RADIUS, 800.0), SIZE);
    let reading = touches.reading(SIZE);
    assert!((reading.stick_x + 1.0).abs() < 1e-3);
    assert_eq!(reading.buttons, Button::Cross.bit());
    assert!(reading.airbrake_right > 0.5, "{}", reading.airbrake_right);
    touches.up(2, SIZE);
    assert_eq!(touches.reading(SIZE).buttons, 0);
    assert!(!touches.zone_down(GoZone::Right));
}

#[test]
fn a_zone_does_not_latch_a_tap_and_the_brake_buttons_still_work_with_zones_off() {
    let mut touches = live();
    touches.down(1, go_at(0.1, 0.9), SIZE);
    assert_eq!(touches.take_taps(), vec![Button::Cross]);
    let mut off = Touches::default();
    off.set_go_zones(false, SIZE);
    off.down(2, centre(Control::AirbrakeRight), SIZE);
    assert!(off.reading(SIZE).buttons & Button::R.bit() != 0);
}

#[test]
fn the_brake_buttons_exist_only_with_zones_off() {
    let has =
        |zones: bool, control: Control| layout_for(SIZE, zones).iter().any(|(c, _)| *c == control);
    for control in [Control::AirbrakeLeft, Control::AirbrakeRight] {
        assert!(has(false, control) && !has(true, control));
    }
    let mut on = live();
    let spot = centre(Control::AirbrakeRight);
    assert_eq!(
        control_at(spot, SIZE, true),
        None,
        "no hit area with zones on"
    );
    on.down(1, spot, SIZE);
    assert_eq!(on.reading(SIZE).airbrake_right, 0.0);
    assert_eq!(control_at(spot, SIZE, false), Some(Control::AirbrakeRight));
}

#[test]
fn pause_and_view_take_the_top_left_corner_when_the_brakes_are_in_go() {
    let rects = layout_for(SIZE, true);
    let pause = rects.iter().find(|(c, _)| *c == Control::Pause).unwrap().1;
    assert!((pause.x - SAFE_X * SIZE.1).abs() < 0.01);
    for (i, (_, a)) in rects.iter().enumerate() {
        assert!(a.x >= SAFE_X * SIZE.1 - 0.01 && a.y >= SAFE_Y * SIZE.1);
        for (_, b) in &rects[i + 1..] {
            let x = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
            let y = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
            assert!(x <= 0.0 || y <= 0.0);
        }
    }
}
