use super::*;
use oag_ui::frontend::Align;

fn text(x: f32, y: f32) -> Draw {
    Draw::Text {
        x,
        y,
        scale: 1.5,
        color: [1.0; 4],
        border: None,
        align: Align::Left,
        text: "88".into(),
        wrap_width: Some(40.0),
    }
}

const HALF: Placement = Placement {
    scale: 0.5,
    offset: [100.0, 20.0],
    clip: [100.0, 20.0, 240.0, 136.0],
};

#[test]
fn the_identity_changes_nothing() {
    let draw = Draw::Sprite {
        rect: [10.0, 20.0, 30.0, 40.0],
        uv: [0.0; 4],
        color: [1.0; 4],
    };
    let screen = Placement::identity((480.0, 272.0));
    assert_eq!(place(&draw, &screen), Some(draw));
}

#[test]
fn a_rect_scales_about_the_origin_and_moves_by_the_offset() {
    let draw = Draw::Fill {
        rect: [10.0, 20.0, 30.0, 40.0],
        color: [1.0; 4],
    };
    assert_eq!(
        place(&draw, &HALF),
        Some(Draw::Fill {
            rect: [105.0, 30.0, 15.0, 20.0],
            color: [1.0; 4]
        })
    );
}

#[test]
fn text_scales_its_size_and_its_wrap_with_its_position() {
    let Some(Draw::Text {
        x,
        y,
        scale,
        wrap_width,
        ..
    }) = place(&text(100.0, 50.0), &HALF)
    else {
        panic!("text inside the clip is kept");
    };
    assert_eq!((x, y, scale, wrap_width), (150.0, 45.0, 0.75, Some(20.0)));
}

#[test]
fn what_lands_outside_the_view_is_dropped() {
    let far = Draw::Fill {
        rect: [2000.0, 20.0, 30.0, 40.0],
        color: [1.0; 4],
    };
    assert_eq!(place(&far, &HALF), None);
    assert_eq!(place(&text(-900.0, 50.0), &HALF), None);
}

#[test]
fn a_frame_keeps_its_lists_apart() {
    let frame = Frame {
        sprites: vec![Draw::Fill {
            rect: [0.0, 0.0, 10.0, 10.0],
            color: [1.0; 4],
        }],
        hud_text: vec![text(10.0, 10.0)],
        small_text: vec![text(10.0, 10.0), text(-900.0, 0.0)],
        default_text: Vec::new(),
        ..Frame::default()
    };
    let placed = frame.placed(&HALF);
    assert_eq!(
        (
            placed.sprites.len(),
            placed.hud_text.len(),
            placed.small_text.len(),
            placed.default_text.len()
        ),
        (1, 1, 1, 0)
    );
    let mut both = placed.clone();
    both.extend(placed);
    assert_eq!(both.len(), 6);
}

/// A frame given up to [`Frame::into_placed`] places exactly as [`Frame::placed`] does: the
/// same draws moved, the same dropped, one span, the glow moved with them.
#[test]
fn an_owned_frame_places_as_a_borrowed_one_does() {
    let frame = Frame {
        sprites: vec![
            Draw::Fill {
                rect: [0.0, 0.0, 10.0, 10.0],
                color: [1.0; 4],
            },
            Draw::Fill {
                rect: [2000.0, 20.0, 30.0, 40.0],
                color: [1.0; 4],
            },
        ],
        hud_text: vec![text(10.0, 10.0)],
        small_text: vec![text(10.0, 10.0), text(-900.0, 0.0)],
        skin_text: vec![text(40.0, 30.0)],
        spans: vec![
            Span {
                clip: [0.0; 4],
                start: [0; 5],
            };
            2
        ],
        glow: vec![crate::boost_smear::GlowStamp {
            rect: [4.0, 8.0, 16.0, 2.0],
            value: 200,
        }],
        ..Frame::default()
    };
    assert_eq!(frame.clone().into_placed(&HALF), frame.placed(&HALF));
}

#[test]
fn each_extended_frame_keeps_where_its_draws_start() {
    let frame = Frame {
        sprites: vec![Draw::Fill {
            rect: [0.0, 0.0, 10.0, 10.0],
            color: [1.0; 4],
        }],
        hud_text: vec![text(10.0, 10.0)],
        ..Frame::default()
    };
    let one = frame.placed(&HALF);
    let mut all = Frame::default();
    all.extend(one.clone());
    all.extend(one);
    assert_eq!(all.spans.len(), 2);
    assert_eq!(all.spans[0].start, [0, 0, 0, 0, 0]);
    assert_eq!(all.spans[1].start, [1, 1, 0, 0, 0]);
    assert_eq!(all.spans[1].clip, HALF.clip);
}
