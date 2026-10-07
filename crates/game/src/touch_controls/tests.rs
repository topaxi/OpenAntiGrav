use super::art::Art;
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
    draw(&Art::flat(), touches, SIZE, SCALE, paused, zones, opacity)
}

fn fills(list: &[Draw]) -> impl Iterator<Item = ([f32; 4], [f32; 4])> + '_ {
    list.iter().filter_map(|d| match d {
        Draw::Fill { rect, color }
        | Draw::Sprite { rect, color, .. }
        | Draw::RotatedSprite { rect, color, .. } => Some((*rect, *color)),
        _ => None,
    })
}

fn is_lit(c: [f32; 4]) -> bool {
    c[..3] == ZONE_LIT[..3]
}

fn lit(list: &[Draw]) -> usize {
    fills(list).filter(|(_, c)| is_lit(*c)).count()
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
fn go_carries_the_l_and_r_corners_when_zones_are_on() {
    let mut on = words(list(&Touches::default(), false, true, 1.0));
    on.sort();
    assert_eq!(on, ["GO", "L", "R"]);
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
    for (rect, _) in fills(&list(&touches, false, true, 1.0)).filter(|(_, c)| is_lit(*c)) {
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
fn no_two_pieces_of_one_layer_overlap() {
    let mut touches = Touches::default();
    touches.down(1, go().centre(), SIZE);
    let all = list(&touches, false, false, 1.0);
    for layer in [BODY_HELD, EDGE_HELD] {
        let pieces: Vec<_> = fills(&all).filter(|(_, c)| *c == layer).collect();
        assert!(pieces.len() > 2, "{layer:?}");
        for (i, (a, _)) in pieces.iter().enumerate() {
            for (b, _) in &pieces[i + 1..] {
                let x = (a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0]);
                let y = (a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1]);
                assert!(x <= 0.01 || y <= 0.01, "{a:?} overlaps {b:?}");
            }
        }
    }
}

/// A button is a handful of quads, not a stack of rows.
#[test]
fn a_whole_overlay_is_a_few_dozen_draws() {
    let touches = Demo::StickAndGo.touches(SIZE, true);
    let n = list(&touches, false, true, 1.0).len();
    assert!(n < 150, "{n} draws");
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
    let layout = touch::layout_for(SIZE, false);
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

fn lit_area(list: &[Draw]) -> f32 {
    fills(list)
        .filter(|(_, c)| is_lit(*c))
        .map(|(r, _)| r[2] * r[3])
        .sum()
}

#[test]
fn the_separate_brakes_are_drawn_only_with_zones_off() {
    let on = list(&Touches::default(), false, true, 1.0);
    let off = list(&Touches::default(), false, false, 1.0);
    assert_eq!(touch::layout_for(SIZE, true).len(), 5);
    assert_eq!(touch::layout_for(SIZE, false).len(), 7);
    // Zones off draws two more controls and none of GO's corner letters; the
    // first drawn control is GO in both.
    assert_eq!(words(off.clone()).iter().filter(|w| *w == "L").count(), 1);
    assert_eq!(words(on).iter().filter(|w| *w == "L").count(), 1);
    let zones_off_shapes = fills(&off).count();
    let zones_on_shapes = fills(&list(&Touches::default(), false, true, 1.0)).count();
    assert_ne!(zones_off_shapes, zones_on_shapes);
}

#[test]
fn the_lit_segment_grows_and_brightens_with_the_pull() {
    let pulled = |fx: f32, fy: f32| {
        let mut touches = Touches::default();
        touches.set_go_zones(true, SIZE);
        touches.down(1, at(fx, fy), SIZE);
        list(&touches, false, true, 1.0)
    };
    let light = pulled(0.30, 0.70);
    let full = pulled(0.0, 1.0);
    assert!(lit_area(&full) > lit_area(&light));
    let alpha = |l: &[Draw]| {
        fills(l)
            .find(|(_, c)| is_lit(*c))
            .map(|(_, c)| c[3])
            .unwrap()
    };
    assert!(alpha(&full) > alpha(&light));
}

#[test]
fn the_generated_textures_are_antialiased_and_reach_the_overlay_sheet() {
    let images = art::images();
    assert_eq!(images.len(), 4);
    let alpha = |image: &oag_hud::sprite::DecodedImage, x: u32, y: u32| {
        image.rgba[((y * image.width + x) * 4 + 3) as usize]
    };
    let disc = &images[0];
    assert_eq!(alpha(disc, 128, 128), 255, "centre");
    assert_eq!(alpha(disc, 0, 0), 0, "corner");
    let partial = (0..disc.width).any(|x| (1..255).contains(&alpha(disc, x, x)));
    assert!(partial, "an antialiased edge has partial coverage");
    assert_eq!(alpha(&images[1], 128, 128), 0, "the ring is hollow");
    let sheet = crate::cursor::sheet(crate::cursor::LAUNCHER);
    assert!(Art::from_sheet(&sheet).is_some());
    assert!(Art::from_sheet(&oag_hud::sprite::Sheet::default()).is_none());
}
