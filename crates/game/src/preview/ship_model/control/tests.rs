use super::*;

const LAW: ShipControl = oag_hd::frontend::SHIP_CONTROL;

/// Runs `frames` of the original's update on `input`.
fn run(control: &mut CraftControl, input: &CraftInput, frames: usize) {
    for _ in 0..frames {
        control.advance(Some(&LAW), input, FRAME_SECONDS, |_| {});
    }
}

fn pushed(right: (f32, f32), left: (f32, f32), zoom: f32) -> CraftInput {
    CraftInput {
        right,
        left,
        zoom,
        ..CraftInput::default()
    }
}

/// The update's turn: the stick's distance past the dead band, a thirtieth of a radian a frame,
/// and the drawn turn an eighth of the way to where that has put the accumulator.
#[test]
fn a_stick_pushes_the_turn_and_the_drawn_turn_eases_after_it() {
    let mut control = CraftControl::default();
    run(&mut control, &pushed((1.0, 0.0), (0.0, 0.0), 0.0), 1);
    let pushed_to = 0.8 / 30.0;
    let drawn = control.pose().turn[1];
    assert!(
        (drawn.abs() - pushed_to * LAW.turn_ease).abs() < 1.0e-6,
        "{drawn}"
    );
    run(&mut control, &pushed((1.0, 0.0), (0.0, 0.0), 0.0), 1);
    let second = control.pose().turn[1];
    let accumulated = 2.0 * pushed_to;
    let expected = drawn.abs() + (accumulated - drawn.abs()) * LAW.turn_ease;
    assert!(
        (second.abs() - expected).abs() < 1.0e-6,
        "{second} against {expected}"
    );
    assert_eq!(control.pose().turn[0], 0.0, "the other axis stays at rest");
}

/// Both axes: the stick's vertical turns about X, its horizontal about Y.
#[test]
fn each_stick_axis_turns_its_own_term() {
    let mut control = CraftControl::default();
    run(&mut control, &pushed((0.0, 1.0), (0.0, 0.0), 0.0), 10);
    let pose = control.pose();
    assert!(pose.turn[0] != 0.0 && pose.turn[1] == 0.0, "{pose:?}");
}

/// The dead band and the clamp: a push inside 0.2 moves nothing and does not count as the
/// player's touch; a push past 1 is a push of 1.
#[test]
fn the_dead_band_holds_and_the_stick_is_clamped() {
    let mut control = CraftControl::default();
    run(&mut control, &pushed((0.2, -0.15), (0.0, 0.0), 0.0), 30);
    assert_eq!(control.pose(), CraftPose::default());
    let mut a = CraftControl::default();
    let mut b = CraftControl::default();
    run(&mut a, &pushed((1.0, 0.0), (0.0, 0.0), 0.0), 20);
    run(&mut b, &pushed((7.5, 0.0), (0.0, 0.0), 0.0), 20);
    assert_eq!(a.pose(), b.pose());
}

/// The zoom button's pull is half a zoom term, eased a quarter of the way a frame, and the term
/// narrows the lens by sixty degrees a unit.
#[test]
fn the_zoom_button_eases_the_lens_in() {
    let mut control = CraftControl::default();
    run(&mut control, &pushed((0.0, 0.0), (0.0, 0.0), 1.0), 1);
    assert!((control.pose().zoom - 0.5 * LAW.zoom_ease).abs() < 1.0e-6);
    run(&mut control, &pushed((0.0, 0.0), (0.0, 0.0), 1.0), 60);
    assert!((control.pose().zoom - 0.5).abs() < 1.0e-3);
    let craft = control.pose().of(&LAW);
    assert!((craft.fov_delta + 30.0).abs() < 0.1, "{craft:?}");
    run(&mut control, &CraftInput::default(), 60);
    assert!(
        control.pose().zoom.abs() < 1.0e-3,
        "released, it eases back"
    );
}

/// The pan is the left stick times the zoom button's pull, so it is dead until the craft is
/// zoomed, and twice as far across as down.
#[test]
fn the_left_stick_pans_only_while_the_craft_is_zoomed() {
    let mut control = CraftControl::default();
    run(&mut control, &pushed((0.0, 0.0), (1.0, 1.0), 0.0), 30);
    assert_eq!(control.pose().pan, [0.0, 0.0]);
    run(&mut control, &pushed((0.0, 0.0), (1.0, 1.0), 1.0), 80);
    let pan = control.pose().pan;
    assert!(
        (pan[0] - 2.0).abs() < 0.01 && (pan[1] - 1.0).abs() < 0.01,
        "{pan:?}"
    );
}

/// The idle return: a pushed turn stays put for the 181 frames after the last touch, then its
/// accumulator walks back a twentieth of a radian a frame (and the drawn turn follows) to within
/// a step of rest, where the original's two comparisons leave it.
#[test]
fn the_turn_returns_to_rest_after_181_idle_frames() {
    let mut control = CraftControl::default();
    run(&mut control, &pushed((1.0, 0.0), (0.0, 0.0), 0.0), 60);
    let turned = control.pose().turn[1];
    assert!(turned.abs() > 0.2, "{turned}");
    run(&mut control, &CraftInput::default(), 150);
    let held = control.pose().turn[1];
    let pushed_to = 60.0 * 0.8 / 30.0;
    assert!(
        (held.abs() - pushed_to).abs() < 0.01,
        "held through the wait: {held}"
    );
    run(&mut control, &CraftInput::default(), 90);
    assert!(control.pose().turn[1].abs() < 0.07, "{:?}", control.pose());
    assert!(control.pose().turn[1].abs() < turned.abs() / 4.0);
}

/// The accumulator wraps at the original's 3.1459 and the drawn turn goes with it.
#[test]
fn the_turn_wraps_at_pi() {
    let mut control = CraftControl::default();
    let mut peak: f32 = 0.0;
    for _ in 0..400 {
        control.advance(
            Some(&LAW),
            &pushed((1.0, 0.0), (0.0, 0.0), 0.0),
            FRAME_SECONDS,
            |pose| peak = peak.max(pose.turn[1].abs()),
        );
    }
    assert!(
        peak < LAW.wrap + 0.3,
        "the drawn turn trails the accumulator across the wrap: {peak}"
    );
    assert!(peak > 3.0, "it did reach the wrap: {peak}");
}

/// A title with no law keeps every term at rest whatever the devices say.
#[test]
fn without_a_law_the_craft_stays_at_rest() {
    let mut control = CraftControl::default();
    for _ in 0..30 {
        control.advance(
            None,
            &pushed((1.0, 1.0), (1.0, 1.0), 1.0),
            FRAME_SECONDS,
            |_| {},
        );
    }
    assert_eq!(control.pose(), CraftPose::default());
}

/// A tick of any length plays the same frames.
#[test]
fn a_tick_of_any_length_plays_the_same_frames() {
    let play = |ticks: usize| {
        let mut control = CraftControl::default();
        for _ in 0..ticks {
            control.advance(
                Some(&LAW),
                &pushed((1.0, 0.0), (0.0, 0.0), 0.0),
                1.0 / ticks as f32,
                |_| {},
            );
        }
        control.pose().turn[1]
    };
    assert!((play(60) - play(30)).abs() < 0.01);
}

/// A pointer drag turns the craft, the wheel pulls the zoom in, and a drag while it is zoomed
/// pans it; each walks back on the same idle count.
#[test]
fn a_drag_and_the_wheel_do_what_the_sticks_do() {
    let mut control = CraftControl::default();
    let drag = CraftInput {
        drag: Some((200.0, -100.0)),
        ..CraftInput::default()
    };
    run(&mut control, &drag, 1);
    run(&mut control, &CraftInput::default(), 60);
    let turn = control.pose().turn;
    assert!(turn[0] != 0.0 && turn[1] != 0.0, "{turn:?}");
    assert!((turn[1].abs() - 200.0 * DRAG_TURN).abs() < 0.02, "{turn:?}");

    let wheel = CraftInput {
        wheel: -5,
        ..CraftInput::default()
    };
    run(&mut control, &wheel, 1);
    run(&mut control, &CraftInput::default(), 60);
    assert!(
        (control.pose().zoom - 0.5).abs() < 0.01,
        "{:?}",
        control.pose()
    );

    run(&mut control, &drag, 1);
    run(&mut control, &CraftInput::default(), 80);
    let pan = control.pose().pan;
    assert!(
        pan[0] != 0.0 && pan[1] != 0.0,
        "zoomed, a drag pans: {pan:?}"
    );

    run(&mut control, &CraftInput::default(), 400);
    let pose = control.pose();
    assert!(
        pose.zoom.abs() < 0.01 && pose.pan[0].abs() < 0.01 && pose.turn[1].abs() < 0.05,
        "{pose:?}"
    );
}

/// A drag is one event: a tick that says it spends it on its first frame only.
#[test]
fn a_drag_is_spent_by_the_first_frame_of_its_tick() {
    let mut once = CraftControl::default();
    once.advance(
        Some(&LAW),
        &CraftInput {
            drag: Some((100.0, 0.0)),
            ..CraftInput::default()
        },
        FRAME_SECONDS * 3.0,
        |_| {},
    );
    let mut single = CraftControl::default();
    single.advance(
        Some(&LAW),
        &CraftInput {
            drag: Some((100.0, 0.0)),
            ..CraftInput::default()
        },
        FRAME_SECONDS,
        |_| {},
    );
    assert_eq!(once.accumulated, single.accumulated);
}

fn zoomed_in_by_the_wheel() -> CraftControl {
    let mut control = CraftControl::default();
    run(
        &mut control,
        &CraftInput {
            wheel: -5,
            ..CraftInput::default()
        },
        1,
    );
    run(&mut control, &CraftInput::default(), 40);
    control
}

/// A pointer's pan is the zoomed craft's: zooming back out brings the craft back to the middle.
#[test]
fn a_dragged_pan_lets_go_as_the_zoom_does() {
    let mut control = zoomed_in_by_the_wheel();
    let drag = CraftInput {
        drag: Some((200.0, 0.0)),
        ..CraftInput::default()
    };
    run(&mut control, &drag, 1);
    run(&mut control, &CraftInput::default(), 40);
    assert!(control.pose().pan[0].abs() > 0.5, "{:?}", control.pose());
    run(
        &mut control,
        &CraftInput {
            wheel: 5,
            ..CraftInput::default()
        },
        1,
    );
    run(&mut control, &CraftInput::default(), 80);
    let pose = control.pose();
    assert!(
        pose.zoom.abs() < 0.01 && pose.pan[0].abs() < 0.02,
        "{pose:?}"
    );
}

/// Panning with the left stick while the wheel holds the zoom is the player at work: the idle
/// return does not unzoom the craft mid-pan.
#[test]
fn a_stick_pan_under_a_wheel_zoom_holds_the_idle_count_off() {
    let mut control = zoomed_in_by_the_wheel();
    run(&mut control, &pushed((0.0, 0.0), (1.0, 0.0), 0.0), 400);
    assert!(control.pose().zoom > 0.4, "{:?}", control.pose());
    assert!(control.zoomed(&LAW));
}

/// Drift inside the dead band is not a touch: a turned craft still returns to rest under it.
#[test]
fn stick_drift_inside_the_dead_band_does_not_hold_the_return_off() {
    let mut control = CraftControl::default();
    run(&mut control, &pushed((1.0, 0.0), (0.0, 0.0), 0.0), 40);
    assert!(control.pose().turn[1].abs() > 0.5);
    run(&mut control, &pushed((0.15, -0.15), (0.0, 0.0), 0.0), 400);
    assert!(control.pose().turn[1].abs() < 0.07, "{:?}", control.pose());
}
