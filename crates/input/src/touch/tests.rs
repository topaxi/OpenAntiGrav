use super::*;

const SIZE: (f32, f32) = (2400.0, 1080.0);
const STANDARD: Setup = Setup::new(Scheme::Standard, true);
const EASY: Setup = Setup::new(Scheme::Easy, true);
const EASY_OFF: Setup = Setup::new(Scheme::Easy, false);

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
    for (control, r) in rects.iter().copied() {
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
        // The thinnest is ABSORB's bar at 0.11 of the height; the slop adds
        // 0.025 on every side, so a thumb has at least 0.16 to land on.
        assert!(
            r.w >= 0.11 * SIZE.1 - 0.01 && r.h >= 0.11 * SIZE.1 - 0.01,
            "{control:?}"
        );
    }
    let tall = go_rect((1000.0, 500.0));
    let wide = go_rect((2000.0, 500.0));
    assert_eq!(tall.w, wide.w, "the size follows the height, not the width");
}

fn go_at(fx: f32, fy: f32) -> (f32, f32) {
    let r = go_rect(SIZE);
    (r.x + r.w * fx, r.y + r.h * fy)
}

fn live() -> Touches {
    let mut touches = Touches::default();
    touches.set_setup(STANDARD, SIZE);
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
    let r = go_rect(SIZE);
    touches.down(1, (r.x + 5.0, r.y + r.h + SIZE.1 * SLOP * 0.5), SIZE);
    assert!(touches.zone_down(GoZone::Left));
}

#[test]
fn the_zones_are_off_when_the_setting_is() {
    let mut touches = Touches::default();
    touches.set_setup(Setup::new(Scheme::Standard, false), SIZE);
    touches.down(1, go_at(0.1, 0.9), SIZE);
    assert_eq!(touches.reading(SIZE).buttons, Button::Cross.bit());
    touches.set_setup(STANDARD, SIZE);
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
    off.set_setup(Setup::new(Scheme::Standard, false), SIZE);
    off.down(2, centre(Control::AirbrakeRight), SIZE);
    assert!(off.reading(SIZE).buttons & Button::R.bit() != 0);
}

#[test]
fn the_brake_buttons_exist_only_with_zones_off() {
    let has = |zones: bool, control: Control| {
        layout_for(SIZE, Setup::new(Scheme::Standard, zones))
            .iter()
            .any(|(c, _)| *c == control)
    };
    for control in [Control::AirbrakeLeft, Control::AirbrakeRight] {
        assert!(has(false, control) && !has(true, control));
    }
    let mut on = live();
    let spot = centre(Control::AirbrakeRight);
    assert_eq!(
        control_at(spot, SIZE, STANDARD),
        None,
        "no hit area with zones on"
    );
    on.down(1, spot, SIZE);
    assert_eq!(on.reading(SIZE).airbrake_right, 0.0);
    assert_eq!(
        control_at(spot, SIZE, Setup::new(Scheme::Standard, false)),
        Some(Control::AirbrakeRight)
    );
}

#[test]
fn pause_and_view_take_the_top_left_corner_when_the_brakes_are_in_go() {
    let rects = layout_for(SIZE, STANDARD);
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

fn easy(zones: bool) -> Touches {
    let mut touches = Touches::default();
    touches.set_setup(Setup::new(Scheme::Easy, zones), SIZE);
    touches
}

fn spot(setup: Setup, control: Control) -> (f32, f32) {
    layout_for(SIZE, setup)
        .into_iter()
        .find(|(c, _)| *c == control)
        .expect("on screen")
        .1
        .centre()
}

#[test]
fn thrust_latches_to_the_finger_that_began_on_go() {
    let mut touches = live();
    touches.down(1, go_at(0.5, 0.5), SIZE);
    touches.moved(1, spot(STANDARD, Control::Fire), SIZE);
    assert_eq!(
        touches.reading(SIZE).buttons,
        Button::Cross.bit() | Button::Square.bit(),
        "sliding onto FIRE keeps thrust and fires"
    );
    touches.moved(1, spot(STANDARD, Control::Absorb), SIZE);
    assert_eq!(
        touches.reading(SIZE).buttons,
        Button::Cross.bit() | Button::Circle.bit(),
        "ABSORB absorbs and FIRE lets go"
    );
    touches.moved(1, (go_at(0.5, 0.5).0, 5.0), SIZE);
    assert_eq!(
        touches.reading(SIZE).buttons,
        Button::Cross.bit(),
        "off all buttons"
    );
    touches.up(1, SIZE);
    assert_eq!(touches.reading(SIZE).buttons, 0);
}

#[test]
fn a_finger_that_began_on_fire_does_not_latch_until_it_reaches_go() {
    let mut touches = live();
    touches.down(1, spot(STANDARD, Control::Fire), SIZE);
    assert_eq!(touches.reading(SIZE).buttons, Button::Square.bit());
    touches.moved(1, go_at(0.5, 0.5), SIZE);
    touches.moved(1, (go_at(0.5, 0.5).0, 5.0), SIZE);
    assert_eq!(touches.reading(SIZE).buttons, Button::Cross.bit());
}

#[test]
fn the_column_is_fire_go_absorb_top_to_bottom_and_absorb_is_the_thinnest() {
    let at = |c| {
        layout_for(SIZE, STANDARD)
            .into_iter()
            .find(|(k, _)| *k == c)
            .unwrap()
            .1
    };
    let (fire, go, absorb) = (
        at(Control::Fire),
        at(Control::Accelerate),
        at(Control::Absorb),
    );
    assert!(
        fire.y + fire.h <= go.y && go.y + go.h < absorb.y,
        "a gap each side of GO"
    );
    assert!(absorb.h < fire.h && absorb.w == go.w);
}

#[test]
fn a_finger_in_go_is_not_stolen_by_the_absorb_slop_below_it() {
    let r = go_rect(SIZE);
    let near_bottom = (r.x + r.w / 2.0, r.y + r.h - 1.0);
    assert_eq!(
        control_at(near_bottom, SIZE, STANDARD),
        Some(Control::Accelerate)
    );
}

#[test]
fn the_brake_reads_full_at_85_percent_depth() {
    let mut touches = live();
    touches.down(
        1,
        go_at(0.0, 1.0 - GO_ZONE_BOTTOM * (1.0 - GO_FULL_DEPTH)),
        SIZE,
    );
    assert!((touches.reading(SIZE).airbrake_left - 1.0).abs() < 1e-4);
    let mut touches = easy(true);
    touches.down(1, go_at(0.5, 1.0 - EASY_BAND * (1.0 - GO_FULL_DEPTH)), SIZE);
    assert!((touches.brake_pull() - 1.0).abs() < 1e-4);
}

#[test]
fn the_easy_curve_steers_to_full_lock_then_brakes_toward_the_rim() {
    let table = [
        (0.0, 0.0, 0.0),
        (0.3, 0.5, 0.0),
        (0.6, 1.0, 0.0),
        (0.8, 1.0, 0.5),
        (1.0, 1.0, 1.0),
        (-0.8, -1.0, 0.5),
        (1.5, 1.0, 1.0),
    ];
    for (u, steer, brake) in table {
        let (s, b) = easy_curve(u);
        assert!(
            (s - steer).abs() < 1e-5 && (b - brake).abs() < 1e-5,
            "{u}: {s} {b}"
        );
    }
}

fn stick_drag(touches: &mut Touches, dx_of_radius: f32) {
    touches.down(9, (300.0, 800.0), SIZE);
    let reach = SIZE.1 * EASY_STICK_RADIUS * dx_of_radius;
    touches.moved(9, (300.0 + reach, 800.0), SIZE);
}

#[test]
fn the_easy_stick_blends_its_rim_into_the_airbrake_on_that_side() {
    let mut touches = easy(true);
    stick_drag(&mut touches, -0.8);
    let reading = touches.reading(SIZE);
    assert!((reading.stick_x + 1.0).abs() < 1e-4);
    assert!((reading.airbrake_left - 0.5).abs() < 1e-4);
    assert_eq!(reading.airbrake_right, 0.0);
    let mut touches = easy(true);
    stick_drag(&mut touches, 0.4);
    let reading = touches.reading(SIZE);
    assert!(reading.stick_x > 0.5 && reading.stick_x < 1.0);
    assert_eq!((reading.airbrake_left, reading.airbrake_right), (0.0, 0.0));
}

#[test]
fn easys_go_band_brakes_the_steered_side_through_novice_airbrakes() {
    let deep = go_at(0.5, 0.97);
    let mut centred = easy(true);
    centred.down(1, deep, SIZE);
    let both = centred.reading(SIZE);
    assert!(both.airbrake_left > 0.5 && both.airbrake_left == both.airbrake_right);
    let mut right = easy(true);
    right.down(1, deep, SIZE);
    stick_drag(&mut right, 0.5);
    let reading = right.reading(SIZE);
    assert_eq!(
        reading.airbrake_left, 0.0,
        "steering right brakes only the right"
    );
    assert!(reading.airbrake_right > 0.5);
    let mut left = easy(true);
    left.down(1, deep, SIZE);
    stick_drag(&mut left, -0.5);
    let reading = left.reading(SIZE);
    assert_eq!(reading.airbrake_right, 0.0);
    assert!(reading.airbrake_left > 0.5);
    assert_eq!(reading.buttons, Button::Cross.bit());
}

#[test]
fn the_rim_and_the_band_combine_by_the_larger_on_each_side() {
    let mut touches = easy(true);
    touches.down(1, go_at(0.5, 0.97), SIZE);
    stick_drag(&mut touches, -1.0);
    let reading = touches.reading(SIZE);
    assert!(
        (reading.airbrake_left - 1.0).abs() < 1e-4,
        "{}",
        reading.airbrake_left
    );
    assert_eq!(reading.airbrake_right, 0.0);
}

#[test]
fn easy_with_the_zone_off_splits_the_bar_into_brake_and_absorb() {
    let rects = layout_for(SIZE, EASY_OFF);
    assert!(
        !rects
            .iter()
            .any(|(c, _)| matches!(c, Control::AirbrakeLeft | Control::AirbrakeRight)),
        "no separate BRAKE L/R buttons"
    );
    let at = |c| rects.iter().find(|(k, _)| *k == c).unwrap().1;
    let (brake, absorb) = (at(Control::Brake), at(Control::Absorb));
    assert!(brake.x + brake.w <= absorb.x && brake.y == absorb.y);
    // GO is thrust alone in the zone-off mode.
    let mut touches = easy(false);
    touches.down(1, go_at(0.5, 0.97), SIZE);
    assert_eq!(touches.reading(SIZE).airbrake_left, 0.0);
    assert_eq!(touches.reading(SIZE).buttons, Button::Cross.bit());
}

#[test]
fn the_bar_brake_is_a_latched_finger_away_and_picks_the_steered_side() {
    let mut touches = easy(false);
    touches.down(1, go_at(0.5, 0.5), SIZE);
    touches.moved(1, spot(EASY_OFF, Control::Brake), SIZE);
    let reading = touches.reading(SIZE);
    assert_eq!(
        reading.buttons,
        Button::Cross.bit(),
        "thrust held, no L or R bit"
    );
    assert_eq!(
        (reading.airbrake_left, reading.airbrake_right),
        (1.0, 1.0),
        "centred: both"
    );
    stick_drag(&mut touches, 0.5);
    let reading = touches.reading(SIZE);
    assert_eq!((reading.airbrake_left, reading.airbrake_right), (0.0, 1.0));
    touches.moved(1, spot(EASY_OFF, Control::Absorb), SIZE);
    assert_eq!(
        touches.reading(SIZE).buttons,
        Button::Cross.bit() | Button::Circle.bit()
    );
    assert_eq!(touches.reading(SIZE).airbrake_right, 0.0);
}

#[test]
fn a_novice_sim_gets_easys_pull_on_the_right_axis_alone() {
    let mut reading = Reading::default();
    reading.airbrake_left = 0.7;
    fold_for_novice_sim(&mut reading);
    assert_eq!((reading.airbrake_left, reading.airbrake_right), (0.0, 0.7));
    let sim = oag_gameplay::controls::novice_airbrakes(reading.airbrake_right, -1.0);
    assert_eq!(sim, (0.7, 0.0), "the sim sides it off the steering again");
}

#[test]
fn scheme_tokens_round_trip() {
    for scheme in [Scheme::Standard, Scheme::Easy] {
        assert_eq!(Scheme::parse(scheme.name()), Some(scheme));
    }
    assert_eq!(Scheme::parse("nope"), None);
    assert_eq!(Scheme::default(), Scheme::Standard);
}
