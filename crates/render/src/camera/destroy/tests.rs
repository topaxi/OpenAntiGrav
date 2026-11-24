//! The destroy camera against the numbers read off a running PPSSPP: Talon's
//! Junction, a Venom player craft on the start line put into state 4, the
//! camera controller's fields read each frame (`capA`, 2026-10-01).

use super::*;

const FRAME: f32 = 1.0 / 60.0;

/// The eight-th of Talon's Junction's ten authored cameras, `track.vex` node
/// `1065`: the eye is the node's translation, the aim its payload's `+0x10`.
fn station() -> Station {
    Station {
        eye: Vec3::new(468.343_6, -22.805_176, -39.869_6),
        aim: Vec3::new(344.118_74, -43.194_11, -137.083_9),
    }
}

/// Where the craft sat at the start line, as the camera's focus read it.
fn craft() -> Vec3 {
    Vec3::new(6.077_749, -50.066_387, -196.007_03)
}

/// The nine others, from the same file: no start-line craft is nearer to any
/// of their aim points than the eighth's, which is why it was picked.
fn all_ten() -> Vec<Station> {
    let eyes_and_aims: [([f32; 3], [f32; 3]); 10] = [
        ([-335.4, -0.03, 94.07], [-233.88, -74.62, 53.65]),
        ([-817.77, 47.56, 118.83], [-512.1, -20.55, 132.76]),
        ([-618.5, 6.85, -163.21], [-753.3, 0.0, -100.56]),
        ([-535.14, -32.64, -617.07], [-574.84, -36.86, -470.21]),
        ([-105.9, -13.04, -796.15], [-303.53, -50.27, -659.02]),
        ([-433.83, -34.19, -347.0], [-253.32, -60.33, -536.06]),
        ([32.17, -23.39, -202.03], [-374.88, -46.15, -187.53]),
        ([468.34, -22.81, -39.87], [344.12, -43.19, -137.08]),
        ([531.52, 12.83, 234.43], [591.99, -14.81, 123.21]),
        ([62.81, -34.74, 162.25], [305.15, -39.46, 233.1]),
    ];
    eyes_and_aims
        .iter()
        .map(|(eye, aim)| Station {
            eye: Vec3::from_array(*eye),
            aim: Vec3::from_array(*aim),
        })
        .collect()
}

#[test]
fn the_pick_is_the_nearest_aim_point_and_not_the_nearest_eye() {
    let stations = all_ten();
    assert_eq!(nearest_station(&stations, craft()), Some(7));
    // Placed at (-450, -40, -100) the original picked the sixth, whose aim is
    // 115.5 away, over the third, whose eye is 186 away.
    let elsewhere = Vec3::new(-450.0, -40.0, -100.0);
    assert_eq!(nearest_station(&stations, elsewhere), Some(6));
    let by_eye = stations
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.eye
                .distance(elsewhere)
                .total_cmp(&b.eye.distance(elsewhere))
        })
        .map(|(i, _)| i);
    assert_eq!(by_eye, Some(2), "the two rules differ at this point");
    assert_eq!(nearest_station(&[], craft()), None);
}

/// `fov_target` read at the first frame, with `camera+0x268` still at its
/// constructor's `60.0`, and one frame on with it at mode 5's `35.0`.
#[test]
fn the_zoom_frames_the_craft_at_the_extent_it_was_given() {
    let reach = station().eye.distance(craft());
    assert!((reach - 488.6).abs() < 0.1, "{reach}");
    assert!((framing_fov_degrees(60.0, reach) - 7.025_89).abs() < 1e-3);
    assert!((framing_fov_degrees(35.0, reach) - 4.101_826).abs() < 1e-3);
}

/// The field started at 19.914203, eased to 18.965461 the next frame and read
/// 6.730304 at the twenty-ninth.
#[test]
fn the_field_closes_on_its_target_at_six_hundredths_a_frame() {
    let mut camera = Destroy::start(
        &[station()],
        craft(),
        Ease::per_frame(0.4),
        19.914_203 - framing_fov_degrees(START_FRAME_SIZE, station().eye.distance(craft())),
    )
    .expect("one station");
    assert!((camera.fov - 19.914_203).abs() < 1e-3);
    camera.advance(craft(), false, FRAME);
    assert!((camera.fov - 18.965_46).abs() < 1e-3, "{}", camera.fov);
    for _ in 0..28 {
        camera.advance(craft(), false, FRAME);
    }
    assert!((camera.fov - 6.730_30).abs() < 2e-3, "{}", camera.fov);
}

/// The view at the hundred and seventy-ninth frame: the live view node's
/// columns were the camera's right, up and back, and its translation `-eye`.
#[test]
fn the_view_matches_the_live_view_node() {
    let camera = Destroy {
        eye: station().eye,
        focus: Vec3::new(6.077_87, -50.065_22, -196.015_55),
        focus_target: Vec3::new(6.077_87, -50.065_22, -196.015_55),
        fov: 4.102_042_7,
        focus_rate: 0.4,
        fov_rate: FOV_RATE,
        substeps_per_second: None,
        basis: Basis::Flattened,
        up: Vec3::Y,
    };
    let world = camera.to_world();
    let near = |a: Vec3, b: [f32; 3]| (a - Vec3::from_array(b)).length() < 2e-4;
    assert!(near(
        world.x_axis.truncate(),
        [0.320_020_17, 0.0, -0.947_410_76]
    ));
    assert!(near(
        world.y_axis.truncate(),
        [-0.051_119_585, 0.998_543_26, -0.017_267_374]
    ));
    assert!(near(
        world.z_axis.truncate(),
        [0.946_030_6, 0.053_957_15, 0.319_553_97]
    ));
    assert_eq!(world.w_axis.truncate(), station().eye);
    let view = camera.view();
    let at_the_craft = view.transform_point3(Vec3::new(6.077_87, -50.065_22, -196.015_55));
    assert!(at_the_craft.z < 0.0, "the craft is in front of the camera");
}

/// A held camera leaves the focus where it was while the craft moves on, as the
/// original's state 6 does; a free one chases it at the rate it was given.
#[test]
fn a_held_focus_does_not_follow() {
    let mut camera =
        Destroy::start(&[station()], craft(), Ease::per_frame(0.4), 0.0).expect("one station");
    let moved = craft() + Vec3::new(10.0, 0.0, 0.0);
    camera.advance(moved, true, FRAME);
    assert_eq!(camera.focus, craft());
    camera.advance(moved, false, FRAME);
    assert!((camera.focus.x - (craft().x + 10.0 * 0.4)).abs() < 1e-4);
}

#[test]
fn the_starting_field_is_never_below_three_degrees() {
    let camera =
        Destroy::start(&[station()], craft(), Ease::per_frame(0.4), -10.0).expect("one station");
    assert_eq!(camera.fov, START_FOV_FLOOR);
}

fn hd_ease() -> Ease {
    Ease {
        focus_rate: 0.4,
        fov_rate: FOV_RATE,
        substeps_per_second: Some(6000.0),
    }
}

/// `trunc(dt * k)` at 60 Hz: 100 for HD's constant and for 2048's `5999.9995` too: in `f32` the product rounds up to
/// exactly 100.
#[test]
fn a_frame_is_trunc_dt_times_k_substeps() {
    assert_eq!(substeps(FRAME, 6000.0), 100);
    assert_eq!(substeps(FRAME, 5999.9995), 100);
    assert_eq!(substeps(1.0 / 30.0, 6000.0), 200);
}

/// A frame moves `1 - (1 - rate / n)^n` of the way: `0.330` for a `0.4` rate and `0.0582` for
/// `0.06` (row D6), not the `rate` Pulse moves.
#[test]
fn substeps_move_a_little_less_than_the_rate_each_frame() {
    let mut camera = Destroy::start(&[station()], craft(), hd_ease(), 0.0).expect("one station");
    let moved = craft() + Vec3::new(100.0, 0.0, 0.0);
    let (start, fov_start) = (camera.focus.x, camera.fov);
    camera.advance(moved, false, FRAME);
    let share = (camera.focus.x - start) / 100.0;
    assert!((share - 0.330).abs() < 2e-3, "{share}");
    let target = framing_fov_degrees(FRAME_SIZE, station().eye.distance(moved));
    let share = (camera.fov - fov_start) / (target - fov_start);
    assert!((share - 0.0582).abs() < 2e-4, "{share}");
}

#[test]
fn the_ease_rates_are_the_ones_it_was_given() {
    let ease = Ease {
        focus_rate: 0.25,
        fov_rate: 0.12,
        substeps_per_second: None,
    };
    let mut camera = Destroy::start(&[station()], craft(), ease, 0.0).expect("one station");
    let moved = craft() + Vec3::new(8.0, 0.0, 0.0);
    let fov_start = camera.fov;
    camera.advance(moved, false, FRAME);
    assert!((camera.focus.x - (craft().x + 2.0)).abs() < 1e-4);
    let target = framing_fov_degrees(FRAME_SIZE, station().eye.distance(moved));
    assert!((camera.fov - (fov_start + (target - fov_start) * 0.12)).abs() < 1e-4);
}

/// Row W8: the eye stays, the look-at leads the craft by `7.5 x` its velocity, the field is 65.
#[test]
fn the_hold_view_looks_ahead_of_the_craft_from_a_fixed_eye() {
    let eye = Vec3::new(5.0, 3.0, 40.0);
    let hold = Destroy::hold(eye, Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0));
    assert_eq!(hold.eye, eye);
    assert_eq!(hold.focus, Vec3::new(15.0, 0.0, 0.0));
    assert_eq!(hold.fov, HOLD_FOV);
    assert_eq!(hold.basis, Basis::LookAt);
    let ahead = hold.view().transform_point3(hold.focus);
    assert!(ahead.x.abs() < 1e-3 && ahead.y.abs() < 1e-3 && ahead.z < 0.0);
}

/// The station view flattens a wide field's look direction; the hold view does not.
#[test]
fn only_the_station_view_flattens_the_look_direction() {
    let eye = Vec3::new(0.0, 100.0, 100.0);
    let mut station_view = Destroy::hold(eye, Vec3::ZERO, Vec3::ZERO);
    let plain = station_view.to_world().z_axis.y;
    station_view.basis = Basis::Flattened;
    assert!(station_view.to_world().z_axis.y < plain);
}
