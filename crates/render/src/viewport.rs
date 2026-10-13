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
//! View `i` follows grid slot `i`, left to right then top to bottom.

/// Most views one race can show, the grid's own cap (`MAX_SHIPS`).
pub const MAX_VIEWPORTS: usize = 8;

/// The gap between two neighbouring views, in pixels of the target. Chosen, not
/// measured.
pub const GAP: f32 = 2.0;

/// One view's rectangle in target pixels: `x, y, width, height`, the shape
/// `Scene::render`'s `viewport` already takes.
pub type Rect = (f32, f32, f32, f32);

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

/// Cuts `area` into `count` views.
///
/// `count` is clamped to `1..=MAX_VIEWPORTS`, and `1` returns `area` itself,
/// untouched, so a race without a split draws exactly where it always did. Edges
/// are rounded to whole pixels and the views tile `area` with exactly [`GAP`]
/// pixels between neighbours and none at the outer edge, however `area`'s size
/// divides.
#[must_use]
pub fn layout(count: usize, area: Rect) -> Vec<Rect> {
    let count = count.clamp(1, MAX_VIEWPORTS);
    if count == 1 {
        return vec![area];
    }
    let (top, bottom) = rows(count);
    let (x, y, w, h) = area;
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
        assert_eq!(layout(1, AREA), vec![AREA]);
        assert_eq!(layout(0, AREA), vec![AREA], "zero clamps to one");
        let offset = (96.0, 0.0, 1248.0, 816.0);
        assert_eq!(
            layout(1, offset),
            vec![offset],
            "a letterboxed area is kept"
        );
    }

    #[test]
    fn two_views_split_top_and_bottom_at_full_width() {
        let views = layout(2, AREA);
        assert_eq!(views.len(), 2);
        assert_eq!(views[0], (0.0, 0.0, 1440.0, 407.0));
        assert_eq!(views[1], (0.0, 409.0, 1440.0, 407.0));
    }

    #[test]
    fn four_views_are_quadrants() {
        let views = layout(4, AREA);
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
                let views = layout(count, area);
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
    fn a_count_past_the_cap_is_the_cap() {
        assert_eq!(layout(99, AREA).len(), MAX_VIEWPORTS);
    }
}
