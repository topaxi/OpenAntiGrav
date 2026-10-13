//! Split screen: how one rectangle is cut into one view per local player.
//!
//! Pure rectangle maths, with no GPU and no simulation type, so every count from
//! one to [`MAX_VIEWPORTS`] is testable without a window. What draws into each
//! rectangle is the caller's business: the scene is rendered once per rectangle
//! with its own camera, and the aspect and field of view follow the rectangle
//! through [`crate::camera::projection`] and [`crate::camera::fit_vertical_fov`].
//!
//! # What is a reading of the original and what is chosen
//!
//! **Nothing here is measured.** The PS2 front end names its split-screen race
//! camera rows (`FE_RACE_CAM_SS`, `FE_RACE_CAM_SSH`), which says the original had
//! at least two split shapes, but no capture of either has been read, so which
//! way the screen is cut is **chosen, not measured**:
//!
//! - **Two players split top and bottom**, each view full width. A wide strip
//!   keeps the authored vertical field and sees more to the sides, which suits a
//!   forward-looking racing camera better than two tall slivers.
//! - **Three or more split into two rows**, the top row holding `count / 2`
//!   views and the bottom row the rest, each row divided evenly. Four is a 2x2
//!   quadrant grid, three is one wide view over two, eight is two rows of four.
//!   One rule for the whole range, so a count between the ones that were
//!   checked on screen is not a special case.
//! - **A gap** of [`GAP`] pixels separates neighbours, left as
//!   whatever the target was cleared to (black). Chosen, so the views read as
//!   views and not as one smeared picture.
//!
//! - **Two views may be cut left and right instead** ([`Cut::Vertical`]): the race box of
//!   Wipeout HD/Fury authors a `SplitScreen` list of `RB_HORIZ` and `RB_VERT`, so the choice
//!   between the two cuts is the disc's; the geometry (two full-height columns, the same
//!   gap) is chosen, not measured. The cut means anything for two views only; any other
//!   count is cut in rows whatever was asked.
//!
//! View `i` follows grid slot `i`, left to right then top to bottom.

/// Most views one race can show, the grid's own cap (`MAX_SHIPS`).
pub const MAX_VIEWPORTS: usize = 8;

/// The gap between two neighbouring views, in pixels of the target. Chosen, not
/// measured.
pub const GAP: f32 = 2.0;

/// One view's rectangle in target pixels: `x, y, width, height`, the shape
/// `Scene::render`'s `viewport` already takes.
pub type Rect = (f32, f32, f32, f32);

/// Which way two views are cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cut {
    /// One view above the other, each at full width. What every split was before the cut
    /// was a choice, and what any count but two is.
    #[default]
    Horizontal,
    /// One view beside the other, each at full height. Two views only.
    Vertical,
}

impl Cut {
    /// The spelling the command line uses.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }

    /// Parses [`Self::name`].
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "horizontal" => Some(Self::Horizontal),
            "vertical" => Some(Self::Vertical),
            _ => None,
        }
    }

    /// The cut `count` views are actually drawn in: this one for two, rows for any other.
    #[must_use]
    pub fn for_views(self, count: usize) -> Self {
        if count == 2 { self } else { Self::Horizontal }
    }
}

/// How many views sit in the top row and in the bottom row for `count` views.
///
/// `count` is clamped to `1..=MAX_VIEWPORTS`. One view is a single row of one
/// and no bottom row, which is how a race with no split is the identity.
#[must_use]
pub fn rows(count: usize) -> (usize, usize) {
    let count = count.clamp(1, MAX_VIEWPORTS);
    if count == 1 {
        (1, 0)
    } else {
        (count / 2, count - count / 2)
    }
}

/// Cuts `area` into `count` views, two of them the way `cut` says ([`Cut::for_views`]).
///
/// `count` is clamped to `1..=MAX_VIEWPORTS`, and `1` returns `area` itself,
/// untouched, so a race without a split draws exactly where it always did. Edges
/// are rounded to whole pixels and the views tile `area` with exactly [`GAP`]
/// pixels between neighbours and none at the outer edge, however `area`'s size
/// divides.
#[must_use]
pub fn layout(count: usize, area: Rect, cut: Cut) -> Vec<Rect> {
    let count = count.clamp(1, MAX_VIEWPORTS);
    if count == 1 {
        return vec![area];
    }
    let (x, y, w, h) = area;
    if cut.for_views(count) == Cut::Vertical {
        return split_span(x, w, 2)
            .into_iter()
            .map(|(column_x, column_w)| (column_x, y, column_w, h))
            .collect();
    }
    let (top, bottom) = rows(count);
    let row_spans = split_span(y, h, 2);
    let mut views = Vec::with_capacity(count);
    for (row, per_row) in [top, bottom].into_iter().enumerate() {
        let (row_y, row_h) = row_spans[row];
        for (column_x, column_w) in split_span(x, w, per_row) {
            views.push((column_x, row_y, column_w, row_h));
        }
    }
    views
}

/// `parts` consecutive spans over `[start, start + length)`, [`GAP`] apart,
/// edges on whole pixels. The last span absorbs the rounding so the outer edge is
/// exact.
fn split_span(start: f32, length: f32, parts: usize) -> Vec<(f32, f32)> {
    let gaps = GAP * (parts - 1) as f32;
    let each = ((length - gaps).max(0.0) / parts as f32).floor();
    let end = start + length;
    (0..parts)
        .map(|i| {
            let from = start + (each + GAP) * i as f32;
            let to = if i + 1 == parts { end } else { from + each };
            (from, (to - from).max(0.0))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = (0.0, 0.0, 1440.0, 816.0);

    fn overlap(a: Rect, b: Rect) -> bool {
        a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
    }

    #[test]
    fn one_view_is_the_area_itself() {
        assert_eq!(layout(1, AREA, Cut::Horizontal), vec![AREA]);
        assert_eq!(layout(0, AREA, Cut::Horizontal), vec![AREA], "zero clamps to one");
        let offset = (96.0, 0.0, 1248.0, 816.0);
        assert_eq!(
            layout(1, offset, Cut::Horizontal),
            vec![offset],
            "a letterboxed area is kept"
        );
    }

    #[test]
    fn two_views_split_top_and_bottom_at_full_width() {
        let views = layout(2, AREA, Cut::Horizontal);
        assert_eq!(views.len(), 2);
        assert_eq!(views[0], (0.0, 0.0, 1440.0, 407.0));
        assert_eq!(views[1], (0.0, 409.0, 1440.0, 407.0));
    }

    #[test]
    fn four_views_are_quadrants() {
        let views = layout(4, AREA, Cut::Horizontal);
        assert_eq!(views[0], (0.0, 0.0, 719.0, 407.0));
        assert_eq!(views[1], (721.0, 0.0, 719.0, 407.0));
        assert_eq!(views[2], (0.0, 409.0, 719.0, 407.0));
        assert_eq!(views[3], (721.0, 409.0, 719.0, 407.0));
    }

    #[test]
    fn rows_for_every_count() {
        let rows: Vec<_> = (1..=MAX_VIEWPORTS).map(rows).collect();
        assert_eq!(
            rows,
            [
                (1, 0),
                (1, 1),
                (1, 2),
                (2, 2),
                (2, 3),
                (3, 3),
                (3, 4),
                (4, 4)
            ]
        );
    }

    #[test]
    fn every_count_tiles_the_area_without_overlap() {
        for area in [AREA, (96.0, 0.0, 1249.0, 817.0), (0.0, 0.0, 480.0, 272.0)] {
            for count in 1..=MAX_VIEWPORTS {
                let views = layout(count, area, Cut::Horizontal);
                assert_eq!(views.len(), count);
                for (i, a) in views.iter().enumerate() {
                    assert!(a.2 > 0.0 && a.3 > 0.0, "{count} views: {a:?} is empty");
                    assert!(a.0 >= area.0 && a.1 >= area.1);
                    assert!(a.0 + a.2 <= area.0 + area.2 + 1e-3);
                    assert!(a.1 + a.3 <= area.1 + area.3 + 1e-3);
                    assert_eq!(a.0.fract(), 0.0, "{count} views: x is on a pixel");
                    assert_eq!(a.2.fract(), 0.0, "{count} views: width is whole");
                    for b in &views[i + 1..] {
                        assert!(!overlap(*a, *b), "{count} views overlap: {a:?} {b:?}");
                    }
                }
                let right = views.iter().map(|v| v.0 + v.2).fold(0.0, f32::max);
                let bottom = views.iter().map(|v| v.1 + v.3).fold(0.0, f32::max);
                assert_eq!(right, area.0 + area.2, "{count} views reach the right edge");
                assert_eq!(bottom, area.1 + area.3, "{count} views reach the bottom");
            }
        }
    }

    #[test]
    fn two_views_cut_vertically_are_two_full_height_columns() {
        let views = layout(2, AREA, Cut::Vertical);
        assert_eq!(views, [(0.0, 0.0, 719.0, 816.0), (721.0, 0.0, 719.0, 816.0)]);
        let letterboxed = layout(2, (96.0, 0.0, 1249.0, 817.0), Cut::Vertical);
        assert_eq!(letterboxed[0].0, 96.0, "the first column starts at the area");
        let right = letterboxed[1].0 + letterboxed[1].2;
        assert_eq!(right, 96.0 + 1249.0, "the second reaches the right edge");
        assert!(!overlap(letterboxed[0], letterboxed[1]));
        assert_eq!(letterboxed[1].0 - (letterboxed[0].0 + letterboxed[0].2), GAP);
    }

    #[test]
    fn a_vertical_cut_means_nothing_but_for_two_views() {
        for count in (1..=MAX_VIEWPORTS).filter(|&count| count != 2) {
            assert_eq!(
                layout(count, AREA, Cut::Vertical),
                layout(count, AREA, Cut::Horizontal),
                "{count} views are cut in rows whatever the cut"
            );
        }
        assert_eq!(Cut::Vertical.for_views(2), Cut::Vertical);
        assert_eq!(Cut::Vertical.for_views(3), Cut::Horizontal);
        assert_eq!(Cut::parse("vertical"), Some(Cut::Vertical));
        assert_eq!(Cut::parse(Cut::Horizontal.name()), Some(Cut::Horizontal));
        assert_eq!(Cut::parse("diagonal"), None);
    }

    #[test]
    fn a_count_past_the_cap_is_the_cap() {
        assert_eq!(layout(99, AREA, Cut::Horizontal).len(), MAX_VIEWPORTS);
    }
}
