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
