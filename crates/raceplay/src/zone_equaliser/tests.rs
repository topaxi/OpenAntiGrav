use super::*;

fn at(t: &[u8], x: u32, y: u32) -> [u8; 4] {
    t[((y * SIDE + x) * 4) as usize..][..4].try_into().unwrap()
}

#[test]
fn the_grid_has_the_cell_and_line_the_original_target_has() {
    let t = texels();
    assert_eq!(at(&t, 0, 0), GROUND);
    assert_eq!(at(&t, 13, 13), GROUND);
    assert_eq!(at(&t, 14, 0), LINE_COLOUR);
    assert_eq!(at(&t, 0, 15), LINE_COLOUR);
    assert_eq!(at(&t, 16, 0), GROUND);
    assert_eq!(t.len(), (SIDE * SIDE * 4) as usize);
}

/// Silence (every held level zero) is the bare grid byte for byte, frame after frame:
/// the cap of an empty bar falls on the floor's line row. The race-start pair rests on it.
#[test]
fn silence_keeps_the_bare_grid() {
    let mut eq = Equaliser::default();
    for _ in 0..30 {
        eq.step(&[0.0; BANDS], Some([1, 216, 255, 255]), style(1));
    }
    assert_eq!(eq.texels(), texels());
}

/// A held level draws its band's bar from the floor to `510 - 508 * held`, 30 pixels
/// wide from `32 * band + 1`: the cap (the whole tint) is the top 16 rows, the body (the
/// tint halved) the rest, the grid lines over both. Other bands and the gap stay ground.
#[test]
fn a_held_level_draws_a_bar_of_its_height_in_its_column() {
    let mut eq = Equaliser::default();
    let mut held = [0.0; BANDS];
    held[3] = 0.5; // top at 510 - 254 = 256
    assert!(eq.step(&held, Some([2, 216, 255, 255]), Some(0)));
    let t = eq.texels();
    let x = 32 * 3 + 1;
    assert_eq!(at(&t, x, 253), GROUND, "above the top");
    assert_eq!(at(&t, x, 256), [2, 216, 255, 255], "the cap's first row");
    assert_eq!(
        at(&t, x, 269),
        [2, 216, 255, 255],
        "inside the cap, above the line"
    );
    assert_eq!(at(&t, x, 270), LINE_COLOUR, "the grid over the cap");
    assert_eq!(at(&t, x, 272), [1, 108, 127, 255], "the body below the cap");
    assert_eq!(at(&t, x, 509), [1, 108, 127, 255], "the body's last row");
    assert_eq!(
        at(&t, 32 * 3, 300),
        GROUND,
        "the column's first pixel is never drawn"
    );
    assert_eq!(
        at(&t, 32 * 3 + 29, 300),
        [1, 108, 127, 255],
        "the bar's last pixel"
    );
    assert_eq!(at(&t, 32 * 3 + 30, 300), LINE_COLOUR);
    assert_eq!(at(&t, 32 * 2 + 1, 300), GROUND, "band 2 is silent");
}

/// A full level reaches row 2: `510 - 508`.
#[test]
fn a_full_level_reaches_the_top_cell() {
    let mut eq = Equaliser::default();
    eq.step(&[1.0; BANDS], Some([255, 255, 255, 255]), Some(0));
    let t = eq.texels();
    assert_eq!(at(&t, 1, 1), GROUND);
    assert_eq!(at(&t, 1, 2), [255, 255, 255, 255]);
    assert_eq!(at(&t, 1, 20), [127, 127, 127, 255]);
}

/// A bar that falls leaves a trail that fades by `1 - 0x60/255` a frame toward the
/// ground of 1, as the read target's `0x60xxxxxx` trails do.
#[test]
fn a_fallen_bar_fades_to_the_ground() {
    let mut eq = Equaliser::default();
    let mut held = [0.0; BANDS];
    held[0] = 0.5;
    eq.step(&held, Some([200, 200, 200, 255]), Some(0));
    eq.step(&[0.0; BANDS], Some([200, 200, 200, 255]), Some(0));
    let t = eq.texels();
    assert_eq!(
        at(&t, 1, 300),
        [62, 62, 62, 255],
        "100 * 159 / 255, rounded"
    );
    for _ in 0..40 {
        eq.step(&[0.0; BANDS], Some([200, 200, 200, 255]), Some(0));
    }
    assert_eq!(eq.texels(), texels(), "faded out to the bare grid");
}

/// Stages 0 to 2 draw style 0; the others' styles are unread and draw the bare grid.
#[test]
fn only_style_zero_draws_bars() {
    assert_eq!((0..3).map(style).collect::<Vec<_>>(), [Some(0); 3]);
    assert_eq!(style(3), Some(7));
    assert_eq!(style(8), Some(1));
    assert_eq!(style(15), None);
    let mut eq = Equaliser::default();
    eq.step(&[1.0; BANDS], Some([255, 255, 255, 255]), style(1));
    assert_ne!(eq.texels(), texels());
    eq.step(&[1.0; BANDS], Some([255, 255, 255, 255]), style(3));
    assert_eq!(eq.texels(), texels());
}
