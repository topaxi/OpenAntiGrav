//! [`super::wrap`]: the words a label keeps and the width it keeps them within.

use super::*;

/// Every line fits, and nothing is lost or duplicated in the middle.
#[test]
fn wrapping_fits_every_line_and_keeps_every_word() {
    let atlas = Atlas::build();
    let text = "Actuating the left or right airbrake through a corner turns tighter \
                without shedding speed";
    let lines = wrap(&atlas, text, 1.0, 200.0);
    assert!(lines.len() > 1, "{lines:?}");
    for line in &lines {
        assert!(
            measure(&atlas, line) <= 200.0,
            "{line:?} is {} wide",
            measure(&atlas, line)
        );
    }
    assert_eq!(
        lines.join(" "),
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    );
}

/// A word that cannot fit is left over-long rather than cut in half or
/// dropped.
#[test]
fn a_word_wider_than_the_column_is_kept_whole() {
    let atlas = Atlas::build();
    let lines = wrap(&atlas, "antigravitational", 1.0, 10.0);
    assert_eq!(lines, ["antigravitational"]);
}

/// A snapped pen steps up to a whole unit before each glyph, so a line is wider than its fractional
/// advance sum says: at 0.7 the five glyphs of `aa bb` measure 21 and draw 23.5 wide. A limit
/// between the two keeps the line whole by the first measure and breaks it by the second.
#[test]
fn a_snapped_pen_breaks_a_line_the_fractional_one_keeps() {
    let atlas = Atlas::build();
    assert_eq!(wrap(&atlas, "aa bb", 0.7, 22.0), ["aa bb"]);
    assert_eq!(wrap_pens(&atlas, "aa bb", 0.7, 22.0, false), ["aa bb"]);
    assert_eq!(wrap_pens(&atlas, "aa bb", 0.7, 22.0, true), ["aa", "bb"]);
}

/// Pure's glyph is a PSP sprite: it starts at the pen rounded up and is the box's size at the scale
/// rounded down, so a `Stats` glyph (box 7 by 14 texels) at 0.8 is 5 by 11 pixels from x 249 and
/// the next pen (256.4) starts the next quad at 257. The run's ink edge is the last quad's right.
#[test]
fn a_snapped_glyph_is_a_whole_pixel_quad_from_the_pen_and_the_run_ends_at_its_right() {
    let atlas = Atlas::build();
    let cell = Cell {
        width: 7,
        height: 14,
        advance: 9.0,
        bearing: 1.0,
        ..atlas.cell('A').expect("a glyph")
    };
    assert_eq!(
        snapped_quad(&cell, 0.8, 249.0),
        SnappedQuad {
            left: 249.0,
            width: 5.0,
            height: 11.0
        }
    );
    assert_eq!(snapped_quad(&cell, 0.8, 256.4).left, 257.0);
    let one = atlas.cell('A').expect("a glyph");
    let quad = snapped_quad(&one, 0.8, 0.0);
    assert_eq!(
        snapped_run(&atlas, "A", 0.8).ink_right,
        quad.left + quad.width
    );
}

/// A box with ink is never narrower than a pixel: a one-texel glyph at a scale under 1 would floor to
/// nothing, and a zero-width quad inside a clip window divides 0 by 0.
#[test]
fn a_one_texel_glyph_keeps_a_pixel_and_an_empty_box_none() {
    let atlas = Atlas::build();
    let one = Cell {
        width: 1,
        height: 1,
        ..atlas.cell('A').expect("a glyph")
    };
    let quad = snapped_quad(&one, 0.8, 10.0);
    assert_eq!((quad.width, quad.height), (1.0, 1.0));
    let none = Cell { width: 0, ..one };
    assert_eq!(snapped_quad(&none, 0.8, 10.0).width, 0.0);
}
