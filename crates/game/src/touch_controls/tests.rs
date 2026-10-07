use super::*;

const SIZE: (f32, f32) = (2400.0, 1080.0);

fn go() -> touch::Rect {
    touch::layout(SIZE)[0].1
}

fn at(fx: f32, fy: f32) -> (f32, f32) {
    let r = go();
    (r.x + r.w * fx, r.y + r.h * fy)
}

fn lit(list: &[Draw]) -> usize {
    list.iter()
        .filter(|d| matches!(d, Draw::Fill { color, .. } if *color == ZONE_LIT))
        .count()
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
fn an_idle_overlay_draws_every_control_and_no_stick() {
    let list = draw(&Touches::default(), SIZE, 272.0 / SIZE.1, false, false);
    assert_eq!(words(list.clone()).len(), Control::ALL.len());
    assert_eq!(list.len(), Control::ALL.len() * 3);
}

#[test]
fn go_carries_its_brake_corners_only_when_they_are_on() {
    let scale = 272.0 / SIZE.1;
    let off = words(draw(&Touches::default(), SIZE, scale, false, false));
    let on = words(draw(&Touches::default(), SIZE, scale, false, true));
    assert!(!off.contains(&"L".to_string()));
    assert_eq!(on.len(), off.len() + 2);
    assert!(on.contains(&"L".to_string()) && on.contains(&"R".to_string()));
}

#[test]
fn the_zone_under_the_finger_lights_and_the_centre_lights_none() {
    let scale = 272.0 / SIZE.1;
    let mut touches = Touches::default();
    touches.set_go_zones(true, SIZE);
    touches.down(1, at(0.9, 0.9), SIZE);
    assert_eq!(lit(&draw(&touches, SIZE, scale, false, true)), 1);
    touches.moved(1, at(0.5, 0.9), SIZE);
    assert_eq!(lit(&draw(&touches, SIZE, scale, false, true)), 0);
}

#[test]
fn a_held_control_draws_brighter_and_a_stick_adds_two_shapes() {
    let scale = 272.0 / SIZE.1;
    let idle = draw(&Touches::default(), SIZE, scale, false, false);
    let mut touches = Touches::default();
    touches.down(1, go().centre(), SIZE);
    touches.down(2, (300.0, 800.0), SIZE);
    let list = draw(&touches, SIZE, scale, false, false);
    assert_eq!(list.len(), idle.len() + 2);
    let alpha = |list: &[Draw], at: usize| match &list[at] {
        Draw::Fill { color, .. } => color[3],
        other => panic!("{other:?}"),
    };
    assert!(alpha(&list, 1) > alpha(&idle, 1), "fill");
    assert!(alpha(&list, 0) > alpha(&idle, 0), "outline");
}

#[test]
fn the_pause_button_says_resume_while_paused() {
    let words = |paused: bool| words(draw(&Touches::default(), SIZE, 0.25, paused, true));
    assert!(words(false).contains(&"PAUSE".to_string()));
    assert!(words(true).contains(&"RESUME".to_string()));
    assert!(!words(true).contains(&"PAUSE".to_string()));
}

#[test]
fn everything_stays_inside_the_grid() {
    let scale = 272.0 / SIZE.1;
    let width = SIZE.0 * scale;
    for draw in draw(&Touches::default(), SIZE, scale, false, true) {
        if let Draw::Fill { rect, .. } = draw {
            assert!(rect[0] >= 0.0 && rect[0] + rect[2] <= width + 0.01);
            assert!(rect[1] >= 0.0 && rect[1] + rect[3] <= 272.01);
        }
    }
}
