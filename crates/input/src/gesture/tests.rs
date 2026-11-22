use super::*;

const WIN: (f32, f32) = (2400.0, 1080.0);

fn g() -> Gestures {
    Gestures::new(WIN)
}

fn ticks(g: &mut Gestures, n: u32) -> Vec<Frame> {
    (0..n).map(|_| g.take(WIN)).collect()
}

fn sum_drag(frames: &[Frame]) -> (f32, f32) {
    frames
        .iter()
        .fold((0.0, 0.0), |a, f| (a.0 + f.drag.0, a.1 + f.drag.1))
}

fn sum_pan(frames: &[Frame]) -> (f32, f32) {
    frames
        .iter()
        .fold((0.0, 0.0), |a, f| (a.0 + f.pan.0, a.1 + f.pan.1))
}

fn tap(g: &mut Gestures, id: u64, at: (f32, f32)) {
    g.down(id, at);
    g.take(WIN);
    g.up(id, at);
}

#[test]
fn a_short_still_touch_taps() {
    let mut g = g();
    g.down(1, (500.0, 500.0));
    g.moved(1, (505.0, 503.0));
    g.take(WIN);
    g.up(1, (505.0, 503.0));
    let f = g.take(WIN);
    assert_eq!(f.tap, Some((505.0, 503.0)));
    assert_eq!(f.drag, (0.0, 0.0));
    let rest = ticks(&mut g, DOUBLE_TAP_TICKS + 1);
    assert_eq!(rest.iter().filter_map(|f| f.single_tap).count(), 1);
}

#[test]
fn travel_past_the_slop_is_a_drag_and_never_a_tap() {
    let mut g = g();
    g.down(1, (500.0, 500.0));
    let mut frames = vec![g.take(WIN)];
    g.moved(1, (500.0, 500.0 + 108.0));
    frames.push(g.take(WIN));
    g.up(1, (500.0, 608.0));
    frames.push(g.take(WIN));
    let d = sum_drag(&frames[..2]);
    assert!((d.1 - 0.1).abs() < 1e-5 && d.0.abs() < 1e-6);
    assert!(frames.iter().all(|f| f.tap.is_none()));
}

#[test]
fn a_slow_touch_is_not_a_tap() {
    let mut g = g();
    g.down(1, (500.0, 500.0));
    ticks(&mut g, TAP_TICKS + 2);
    g.up(1, (500.0, 500.0));
    assert!(g.take(WIN).tap.is_none());
}

#[test]
fn two_close_taps_are_a_double_tap_and_no_single() {
    let mut g = g();
    tap(&mut g, 1, (500.0, 500.0));
    let a = g.take(WIN);
    assert!(a.tap.is_some());
    ticks(&mut g, 8);
    tap(&mut g, 2, (510.0, 505.0));
    let mut all = vec![g.take(WIN)];
    all.extend(ticks(&mut g, 40));
    assert_eq!(all.iter().filter_map(|f| f.double_tap).count(), 1);
    assert!(all.iter().all(|f| f.single_tap.is_none()));
}

#[test]
fn taps_too_far_apart_in_time_or_space_are_two_singles() {
    let mut g = g();
    tap(&mut g, 1, (500.0, 500.0));
    tap(&mut g, 2, (1500.0, 500.0));
    let all = ticks(&mut g, 60);
    assert_eq!(all.iter().filter_map(|f| f.single_tap).count(), 2);
    assert!(all.iter().all(|f| f.double_tap.is_none()));

    let mut g = Gestures::new(WIN);
    tap(&mut g, 1, (500.0, 500.0));
    ticks(&mut g, DOUBLE_TAP_TICKS + 3);
    tap(&mut g, 2, (500.0, 500.0));
    let all = ticks(&mut g, 60);
    assert!(all.iter().all(|f| f.double_tap.is_none()));
}

/// Two fingers at `(cx ± r·cos a, cy ± r·sin a)` for `steps` ticks from
/// `(r0, a0)` to `(r1, a1)`, with the centre moving by `shift`.
fn pair(
    g: &mut Gestures,
    (cx, cy): (f32, f32),
    (r0, a0): (f32, f32),
    (r1, a1): (f32, f32),
    shift: (f32, f32),
    steps: u32,
) -> Vec<Frame> {
    let at = |t: f32| {
        let r = r0 + (r1 - r0) * t;
        let a = a0 + (a1 - a0) * t;
        let c = (cx + shift.0 * t, cy + shift.1 * t);
        (
            (c.0 - r * a.cos(), c.1 - r * a.sin()),
            (c.0 + r * a.cos(), c.1 + r * a.sin()),
        )
    };
    let (p, q) = at(0.0);
    g.down(1, p);
    g.down(2, q);
    let mut frames = vec![g.take(WIN)];
    for s in 1..=steps {
        let (p, q) = at(s as f32 / steps as f32);
        g.moved(1, p);
        g.moved(2, q);
        frames.push(g.take(WIN));
    }
    frames
}

#[test]
fn spreading_two_fingers_pinches_without_pan_or_twist() {
    let mut g = g();
    let frames = pair(
        &mut g,
        (1200.0, 540.0),
        (100.0, 0.0),
        (200.0, 0.0),
        (0.0, 0.0),
        20,
    );
    let ratio: f32 = frames.iter().map(|f| f.pinch).product();
    assert!((ratio - 2.0).abs() < 0.01, "ratio {ratio}");
    assert!(sum_pan(&frames).0.abs() < 1e-5);
    assert!(frames.iter().all(|f| f.twist == 0.0));
    assert!(frames.iter().all(|f| f.tap.is_none()));
}

#[test]
fn turning_the_pair_clockwise_twists_positive_and_wraps() {
    let mut g = g();
    let q = std::f32::consts::FRAC_PI_2;
    let frames = pair(
        &mut g,
        (1200.0, 540.0),
        (150.0, 0.0),
        (150.0, q),
        (0.0, 0.0),
        30,
    );
    let twist: f32 = frames.iter().map(|f| f.twist).sum();
    assert!((twist - q).abs() < 1e-3, "twist {twist}");
    let ratio: f32 = frames.iter().map(|f| f.pinch).product();
    assert!((ratio - 1.0).abs() < 1e-3);

    // Across the ±π seam: no 2π spike.
    let mut g = Gestures::new(WIN);
    let frames = pair(
        &mut g,
        (1200.0, 540.0),
        (150.0, PI - 0.3),
        (150.0, PI + 0.3),
        (0.0, 0.0),
        30,
    );
    let twist: f32 = frames.iter().map(|f| f.twist).sum();
    assert!((twist - 0.6).abs() < 1e-3, "twist {twist}");
    assert!(frames.iter().all(|f| f.twist.abs() < 0.2));
}

#[test]
fn moving_the_pair_together_pans_by_the_centroid() {
    let mut g = g();
    let frames = pair(
        &mut g,
        (1200.0, 540.0),
        (100.0, 0.0),
        (100.0, 0.0),
        (216.0, -108.0),
        20,
    );
    let p = sum_pan(&frames);
    assert!(
        (p.0 - 0.2).abs() < 1e-4 && (p.1 + 0.1).abs() < 1e-4,
        "{p:?}"
    );
    let ratio: f32 = frames.iter().map(|f| f.pinch).product();
    assert!((ratio - 1.0).abs() < 1e-4);
}

#[test]
fn a_second_finger_landing_does_not_kick_the_drag() {
    let mut g = g();
    g.down(1, (500.0, 500.0));
    g.moved(1, (500.0, 600.0));
    let a = g.take(WIN);
    assert!((a.drag.1 - 100.0 / 1080.0).abs() < 1e-5);
    g.down(2, (900.0, 600.0));
    let b = g.take(WIN);
    assert_eq!(b.drag, (0.0, 0.0));
    assert_eq!(b.fingers, 2);
    g.up(2, (900.0, 600.0));
    g.moved(1, (500.0, 610.0));
    let c = g.take(WIN);
    assert!(c.drag.1.abs() < 20.0 / 1080.0, "no jump: {:?}", c.drag);
}

#[test]
fn a_two_finger_touch_never_taps_and_a_third_finger_is_ignored() {
    let mut g = g();
    g.down(1, (500.0, 500.0));
    g.down(2, (700.0, 500.0));
    g.down(3, (900.0, 500.0));
    assert_eq!(g.take(WIN).fingers, 2);
    g.moved(3, (900.0, 900.0));
    g.up(3, (900.0, 900.0));
    g.up(2, (700.0, 500.0));
    g.up(1, (500.0, 500.0));
    let all = ticks(&mut g, 40);
    assert!(
        all.iter()
            .all(|f| f.tap.is_none() && f.single_tap.is_none())
    );
}

#[test]
fn cancel_and_release_all_clear_the_fingers() {
    let mut g = g();
    g.down(1, (500.0, 500.0));
    g.down(2, (700.0, 500.0));
    g.cancel(1);
    assert_eq!(g.take(WIN).fingers, 1);
    g.release_all();
    let f = g.take(WIN);
    assert_eq!(f.fingers, 0);
    assert_eq!(f, Frame::default());
}

fn flick(g: &mut Gestures, per_tick: f32, steps: u32) {
    g.down(1, (500.0, 500.0));
    g.take(WIN);
    let mut y = 500.0;
    for _ in 0..steps {
        y += per_tick;
        g.moved(1, (500.0, y));
        g.take(WIN);
    }
    g.up(1, (500.0, y));
}

#[test]
fn a_flick_coasts_down_to_nothing() {
    let mut g = g();
    flick(&mut g, 30.0, 8);
    let frames = ticks(&mut g, 200);
    let coast: Vec<f32> = frames
        .iter()
        .take_while(|f| f.coasting)
        .map(|f| f.drag.1)
        .collect();
    assert!(coast.len() > 10);
    assert!(coast.windows(2).all(|w| w[1] < w[0]));
    assert!(
        frames
            .iter()
            .skip(coast.len())
            .all(|f| !f.coasting && f.drag == (0.0, 0.0))
    );
}

#[test]
fn a_slow_release_does_not_coast() {
    let mut g = g();
    flick(&mut g, 0.5, 8);
    assert!(ticks(&mut g, 5).iter().all(|f| !f.coasting));
}

#[test]
fn a_finger_during_a_coast_catches_it_and_does_not_tap() {
    let mut g = g();
    flick(&mut g, 30.0, 8);
    assert!(g.take(WIN).coasting);
    g.down(2, (800.0, 800.0));
    g.take(WIN);
    g.up(2, (800.0, 800.0));
    let all = ticks(&mut g, 30);
    assert!(
        all.iter()
            .all(|f| !f.coasting && f.tap.is_none() && f.single_tap.is_none())
    );
}

#[test]
fn the_same_script_gives_the_same_frames() {
    let run = || {
        let mut g = g();
        let mut out = pair(
            &mut g,
            (1200.0, 540.0),
            (100.0, 0.0),
            (180.0, 0.4),
            (50.0, 20.0),
            15,
        );
        g.up(1, (1000.0, 500.0));
        g.up(2, (1400.0, 600.0));
        out.extend(ticks(&mut g, 60));
        out
    };
    assert_eq!(run(), run());
}
