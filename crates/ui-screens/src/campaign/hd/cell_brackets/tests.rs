use super::*;

fn mask(src: &str) -> Option<Placed> {
    (src == SRC).then_some(Placed {
        x: 100,
        y: 40,
        width: 16,
        height: 16,
        quad_extent: None,
        quad_centre: None,
        blend: None,
        scale: [1.0, 1.0],
    })
}

fn bracket(corner: bool) -> Bracket {
    Bracket {
        x: 750.0,
        y: 315.0,
        width: 100.0,
        height: 96.0,
        color: 0xFF96_9696,
        corner,
        delay: 0.0,
        seconds: 0.1,
        group: None,
    }
}

#[test]
fn a_bracket_is_the_mask_at_each_of_its_four_corners_turned_to_it() {
    let draws = draws(&[bracket(true)], &mask, f32::INFINITY);
    let rects: Vec<[f32; 4]> = draws
        .iter()
        .map(|draw| match draw {
            Draw::Sprite { rect, .. } => *rect,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        rects,
        [
            [750.0, 315.0, 16.0, 16.0],
            [834.0, 315.0, 16.0, 16.0],
            [750.0, 395.0, 16.0, 16.0],
            [834.0, 395.0, 16.0, 16.0],
        ]
    );
    let Draw::Sprite { uv, .. } = &draws[3] else {
        panic!("a sprite");
    };
    assert_eq!(
        *uv,
        [116.0, 40.0, -16.0, 16.0],
        "the far corner is the file's own bottom-left mark turned about x only"
    );
}

#[test]
fn nothing_draws_without_the_mask_or_for_a_bracket_that_is_not_a_corner_form() {
    assert!(draws(&[bracket(true)], &|_| None, f32::INFINITY).is_empty());
    assert!(draws(&[bracket(false)], &mask, f32::INFINITY).is_empty());
}

/// A bracket draws nothing for its `delay`, then each mark sits `(1 - alpha) / 3` of the box in
/// from its corner on each axis, alpha running linear over the in-time, and the settled marks are
/// flush with the box. This fails if the delay or the inset is dropped.
#[test]
fn a_bracket_waits_its_delay_and_opens_out_from_a_third_in() {
    let waiting = Bracket {
        delay: 0.4,
        ..bracket(true)
    };
    assert!(draws(std::slice::from_ref(&waiting), &mask, 0.3).is_empty());
    let top_left = |since: f32| match draws(std::slice::from_ref(&waiting), &mask, since).first() {
        Some(Draw::Sprite { rect, .. }) => Some([rect[0], rect[1]]),
        _ => None,
    };
    // The frame the countdown crosses 0 holds one frame of the 6-frame ramp.
    let first = top_left(0.4).expect("the first ramp frame");
    assert!(
        (first[0] - (750.0 + 100.0 * (5.0 / 6.0) / 3.0)).abs() < 1e-3,
        "{first:?}"
    );
    assert!(
        (first[1] - (315.0 + 96.0 * (5.0 / 6.0) / 3.0)).abs() < 1e-3,
        "{first:?}"
    );
    assert_eq!(
        top_left(0.6),
        Some([750.0, 315.0]),
        "flush once the ramp is out"
    );
}
