use super::*;

fn mask(src: &str) -> Option<Placed> {
    (src == SRC).then_some(Placed {
        x: 100,
        y: 40,
        width: 16,
        height: 16,
        quad_extent: None,
        blend: None,
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
    }
}

#[test]
fn a_bracket_is_the_mask_at_each_of_its_four_corners_turned_to_it() {
    let draws = draws(&[bracket(true)], &mask);
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
        [116.0, 56.0, -16.0, -16.0],
        "the far corner is turned both ways"
    );
}

#[test]
fn nothing_draws_without_the_mask_or_for_a_bracket_that_is_not_a_corner_form() {
    assert!(draws(&[bracket(true)], &|_| None).is_empty());
    assert!(draws(&[bracket(false)], &mask).is_empty());
}
