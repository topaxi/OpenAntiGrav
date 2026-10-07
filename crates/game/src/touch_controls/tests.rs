use super::*;

const SIZE: (f32, f32) = (2340.0, 1080.0);
const SCALE: f32 = 272.0 / 1080.0;

fn go() -> touch::Rect {
    touch::layout(SIZE)[0].1
}

fn at(fx: f32, fy: f32) -> (f32, f32) {
    let r = go();
    (r.x + r.w * fx, r.y + r.h * fy)
}

fn list(touches: &Touches, paused: bool, zones: bool, opacity: f32) -> Vec<Draw> {
    draw(touches, SIZE, SCALE, paused, zones, opacity)
}

fn fills(list: &[Draw]) -> impl Iterator<Item = ([f32; 4], [f32; 4])> + '_ {
    list.iter().filter_map(|d| match d {
        Draw::Fill { rect, color } => Some((*rect, *color)),
        _ => None,
    })
}

fn lit(list: &[Draw]) -> usize {
    fills(list).filter(|(_, c)| *c == ZONE_LIT).count()
}

fn words(list: Vec<Draw>) -> Vec<String> {
    list.into_iter()
        .filter_map(|d| match d {
            Draw::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

#[test]
fn go_carries_its_brake_corners_only_when_they_are_on() {
    let off = words(list(&Touches::default(), false, false, 1.0));
    let on = words(list(&Touches::default(), false, true, 1.0));
    assert_eq!(on.len(), off.len() + 2);
    assert!(on.contains(&"L".to_string()) && on.contains(&"R".to_string()));
    assert_eq!(off.iter().filter(|w| *w == "GO").count(), 1);
}

#[test]
fn the_zone_under_the_finger_lights_and_the_centre_lights_none() {
    let mut touches = Touches::default();
    touches.set_go_zones(true, SIZE);
    touches.down(1, at(0.9, 0.9), SIZE);
    assert!(lit(&list(&touches, false, true, 1.0)) > 0);
    touches.moved(1, at(0.5, 0.9), SIZE);
    assert_eq!(lit(&list(&touches, false, true, 1.0)), 0);
}

#[test]
fn the_lit_zone_stays_inside_gos_own_shape() {
    let mut touches = Touches::default();
    touches.set_go_zones(true, SIZE);
    touches.down(1, at(0.1, 0.9), SIZE);
    let go = go();
    let (gx, gy, gw, gh) = (go.x * SCALE, go.y * SCALE, go.w * SCALE, go.h * SCALE);
    for (rect, _) in fills(&list(&touches, false, true, 1.0)).filter(|(_, c)| *c == ZONE_LIT) {
        assert!(rect[0] >= gx && rect[0] + rect[2] <= gx + gw * touch::GO_ZONE_SIDE + 0.01);
        assert!(rect[1] >= gy + gh * (1.0 - touch::GO_ZONE_BOTTOM) - 0.01);
        assert!(rect[1] + rect[3] <= gy + gh + 0.01);
    }
}

/// The brief's complaint was an opaque cyan GO: nothing may be near solid,
/// pressed or not, so the track shows through every control.
#[test]
fn nothing_is_opaque_even_pressed() {
    let mut touches = Touches::default();
    touches.set_go_zones(true, SIZE);
    touches.down(1, at(0.1, 0.9), SIZE);
    touches.down(2, (300.0, 800.0), SIZE);
    for (_, colour) in fills(&list(&touches, false, true, 1.0)) {
        assert!(colour[3] <= 0.9, "{colour:?}");
    }
    for (_, colour) in fills(&list(&touches, false, true, 1.0)).filter(|(_, c)| *c == BODY_HELD) {
        assert!(colour[3] <= 0.45);
    }
}

#[test]
fn a_held_control_draws_a_different_body_and_a_stick_adds_shapes() {
    let idle = list(&Touches::default(), false, false, 1.0);
    let mut touches = Touches::default();
    touches.down(1, go().centre(), SIZE);
    let held = list(&touches, false, false, 1.0);
    assert!(fills(&held).any(|(_, c)| c == BODY_HELD));
    assert!(!fills(&idle).any(|(_, c)| c == BODY_HELD));
    touches.down(2, (300.0, 800.0), SIZE);
    let with_stick = list(&touches, false, false, 1.0);
    assert!(fills(&with_stick).any(|(_, c)| c == STICK_KNOB));
    assert!(!fills(&held).any(|(_, c)| c == STICK_KNOB));
}

#[test]
fn no_two_fills_of_one_control_overlap() {
    let mut touches = Touches::default();
    touches.down(1, go().centre(), SIZE);
    let all = list(&touches, false, false, 1.0);
    let bodies: Vec<_> = fills(&all)
        .filter(|(_, c)| *c == BODY_HELD || *c == EDGE_HELD)
        .collect();
    assert!(bodies.len() > 2);
    for (i, (a, _)) in bodies.iter().enumerate() {
        for (b, _) in &bodies[i + 1..] {
            let x = (a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0]);
            let y = (a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1]);
            assert!(x <= 0.01 || y <= 0.01, "{a:?} overlaps {b:?}");
        }
    }
}

#[test]
fn opacity_scales_every_alpha() {
    let full = list(&Touches::default(), false, true, 1.0);
    let half = list(&Touches::default(), false, true, 0.5);
    assert_eq!(full.len(), half.len());
    for ((_, a), (_, b)) in fills(&full).zip(fills(&half)) {
        assert!((b[3] - a[3] * 0.5).abs() < 1e-5);
    }
}

#[test]
fn pause_is_a_glyph_and_resume_is_a_different_one() {
    let pause = list(&Touches::default(), false, false, 1.0);
    let resume = list(&Touches::default(), true, false, 1.0);
    assert_ne!(pause.len(), resume.len());
    assert!(!words(pause).iter().any(|w| w == "PAUSE" || w == "RESUME"));
}

#[test]
fn the_controls_do_not_overlap_each_other() {
    let layout = touch::layout(SIZE);
    for (i, (_, a)) in layout.iter().enumerate() {
        for (_, b) in &layout[i + 1..] {
            let x = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
            let y = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
            assert!(x <= 0.0 || y <= 0.0, "{a:?} overlaps {b:?}");
        }
    }
}

#[test]
fn everything_stays_inside_the_grid() {
    let width = SIZE.0 * SCALE;
    let mut touches = Demo::StickAndGo.touches(SIZE, true);
    touches.set_go_zones(true, SIZE);
    for (rect, _) in fills(&list(&touches, false, true, 1.0)) {
        assert!(
            rect[0] >= -0.01 && rect[0] + rect[2] <= width + 0.01,
            "{rect:?}"
        );
        assert!(rect[1] >= -0.01 && rect[1] + rect[3] <= 272.01, "{rect:?}");
    }
}

#[test]
fn the_capture_poses_build_the_fingers_they_name() {
    assert!(Demo::parse("go-left").is_ok() && Demo::parse("nope").is_err());
    assert!(Demo::GoLeft.touches(SIZE, true).zone_down(GoZone::Left));
    assert!(Demo::GoRight.touches(SIZE, true).zone_down(GoZone::Right));
    let both = Demo::StickAndGo.touches(SIZE, true);
    assert!(both.is_down(Control::Accelerate) && both.stick().is_some());
    assert!(!Demo::Idle.touches(SIZE, true).any());
}
