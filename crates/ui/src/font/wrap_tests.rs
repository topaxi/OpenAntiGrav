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
