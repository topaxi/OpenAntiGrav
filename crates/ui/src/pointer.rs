//! A mouse or a finger on a screen, as one tick's worth of plain data.
//!
//! **Ours, with no counterpart on any disc.** Every title this project
//! reimplements was authored for a pad - `docs/architecture/menus.md` opens
//! on exactly that point - so nothing here is a reading of the original.
//! What a PC or a handheld with a touchscreen needs is the same screens
//! answering a pointer as well as a pad, and this is the vocabulary those
//! screens read: where the pointer is, in the screen's own grid, and what it
//! did this tick.
//!
//! # Why this is not an [`oag_core::buttons::Input`] button
//!
//! A pointer has a position, and a position is a pair of window-relative
//! floats. `InputSnapshot` feeds the `World` and the committed state hash,
//! so a coordinate in it would be a determinism failure waiting for a
//! second monitor. The pointer therefore stops at the front end: the models
//! in this crate consume it directly, the way they consume button edges,
//! and it never reaches a simulation crate. Where a screen's whole answer to
//! a click is "the same as pressing cross" the composition root synthesises
//! that press instead, which is the one place the two vocabularies meet.
//!
//! # Grid space, not window pixels
//!
//! [`Pointer::at`] is in the same space every [`crate::frontend::Draw`] is -
//! the title's 480x272 or the PS2's 640x448 - so a model hit-tests against
//! the rects it just drew and never learns what a window is. The conversion
//! from physical pixels through the aspect viewport and the letterbox is the
//! renderer's own mapping run backwards, and it lives beside that mapping in
//! `oag_game::render`, not here.
//!
//! # Hover is a move, not a position
//!
//! [`Pointer::moved`] is what a hover reacts to, deliberately. A mouse left
//! resting over the third row while the player walks the list with the
//! keyboard must not drag the cursor back to the third row every tick - so
//! the models highlight what is under the pointer only on the tick it
//! actually moved there. A click always targets what is under the pointer,
//! moved or not, because a click is the player pointing at a thing.

/// One tick of pointer input, in the screen's own grid.
///
/// A default `Pointer` is one that is not there: no position, nothing
/// pressed, nothing scrolled. Every model treats it as no input at all, so a
/// headless capture that never builds one draws exactly what it always did.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pointer {
    /// Where the pointer is, or `None` when it is outside the window, has
    /// never entered it, or is a finger that has lifted.
    pub at: Option<(f32, f32)>,
    /// Whether [`Self::at`] changed this tick. See the module doc on hover.
    pub moved: bool,
    /// The primary button went down, or a finger tapped, this tick.
    ///
    /// An edge, not a level: the models act once per press, the way
    /// [`oag_core::buttons::Input::take`] hands out one press per edge.
    pub clicked: bool,
    /// The secondary button went down this tick. Read as "back" wherever a
    /// screen has a back, which is what circle means on the same screens.
    pub back: bool,
    /// Wheel detents this tick, positive toward the player - the direction a
    /// list moves *down* by, matching every desktop's own convention.
    pub scroll: i32,
    /// How far a finger has dragged this tick, in the screen's own grid:
    /// positive `y` is the finger moving *down* the screen, which a list
    /// follows by revealing the rows above.
    ///
    /// **Chosen, not measured**: a touch only drags once it has travelled
    /// past a threshold, and from then on it is a drag for good - it
    /// reports no `at`, no hover and no click, so a scroll never selects.
    /// See `oag_game::pointer::Window`.
    pub drag: (f32, f32),
    /// A finger touched down this tick. Carries no position, no move and no
    /// click - a touch is decided on lift - and is only what a moving
    /// scroll needs to be caught by (see [`crate::kinetic`]).
    pub pressed: bool,
    /// The finger lifted this tick, whether it tapped or dragged, or its
    /// touch was cancelled. What a scroll needs to coast and settle.
    pub released: bool,
}

impl Pointer {
    /// Whether anything at all happened this tick.
    ///
    /// The cheap early-out a model takes before it lays out hit regions: a
    /// still pointer with nothing pressed is the ordinary tick, and laying
    /// out a page to find out it was not pointed at is work for nothing.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        !self.moved
            && !self.clicked
            && !self.back
            && self.scroll == 0
            && self.drag == (0.0, 0.0)
            && !self.pressed
            && !self.released
    }

    /// The position, if the pointer is over the screen at all.
    #[must_use]
    pub fn position(&self) -> Option<(f32, f32)> {
        self.at
    }
}

/// Where a font's capitals actually have ink within the cell a
/// `Draw::Text` is drawn at, in font pixels down from the cell's top edge
/// at scale 1.
///
/// A disc font's glyph box is the whole atlas row - see
/// [`crate::font::Cell::advance`]'s note on `v0` - and a capital sits at
/// the *bottom* of it, under room kept for accents. Pure's `Default` face
/// keeps seven of its sixteen rows empty above the cap line, so a hit band
/// laid from the pen down covers the gap above one row and the ink of the
/// next: the player points at DEUTSCH and ESPAÑOL lights. A band centred on
/// where the ink is answers what the player sees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowInk {
    /// First row of the cell with any coverage, over `A`..=`Z`.
    pub top: f32,
    /// One past the last such row.
    pub bottom: f32,
}

impl RowInk {
    /// Reads the capitals' ink off `atlas`, or `None` when it has no
    /// capitals with any ink - which no real face and not the built-in set
    /// is, so `None` means a caller passed an empty atlas.
    #[must_use]
    pub fn measure(atlas: &crate::font::Atlas) -> Option<Self> {
        let mut top = u32::MAX;
        let mut bottom = 0;
        for cell in ('A'..='Z').filter_map(|ch| atlas.cell(ch)) {
            for row in 0..cell.height {
                let start = ((cell.y + row) * atlas.width + cell.x) as usize;
                // `get`, not an index: a glyph box past the atlas's own
                // pixels is a face's problem to report, not a boot panic.
                let Some(span) = atlas.coverage.get(start..start + cell.width as usize) else {
                    continue;
                };
                let inked = span.iter().any(|&alpha| alpha > 0);
                if inked {
                    top = top.min(row);
                    bottom = bottom.max(row + 1);
                }
            }
        }
        (bottom > 0).then_some(Self {
            top: top as f32,
            bottom: bottom as f32,
        })
    }

    /// The top edge of a band `pitch` tall centred on the ink of a line
    /// whose pen is at `y`, drawn at `scale`.
    #[must_use]
    pub fn band_top(self, y: f32, scale: f32, pitch: f32) -> f32 {
        y + (self.top + self.bottom) * 0.5 * scale - pitch * 0.5
    }
}

/// Whether `point` lies inside `rect`, which is `[x, y, width, height]` -
/// the shape every `Draw` rect has.
///
/// Half-open on the far edges, so two rows that share an edge cannot both
/// claim it; a zero or negative extent contains nothing.
#[must_use]
pub fn contains(rect: [f32; 4], point: (f32, f32)) -> bool {
    let [x, y, width, height] = rect;
    point.0 >= x && point.0 < x + width && point.1 >= y && point.1 < y + height
}

/// Whether `point` lies inside the hexagon a campaign hex sprite draws,
/// where `rect` is that sprite's own `[x, y, width, height]` - the same
/// rect [`contains`] would bounding-box-test.
///
/// **Measured off the shipped art, idealised to a regular hexagon.**
/// `Data\FE\Images\hex_filled.mip` (`pulse-psp-eu.chd`'s `Data.wad`) decodes
/// to a 32x32 4bpp indexed image whose alpha spans the sprite's own width
/// vertex-to-vertex at mid-height (a full 32px row) and narrows to roughly
/// half that at the top and bottom edges (an 16-18px row, two rows of
/// padding in from the sprite's own top/bottom) - a flat-top hexagon whose
/// circumradius is half the sprite's width, which is what this tests
/// exactly rather than the anti-aliased raster. **The idealisation is
/// chosen, not measured**: the on-disc pixels are anti-aliased raster art,
/// not vector data, so no rect of pixels is *the* hexagon, only evidence
/// for one. See `docs/ui/campaign-screens.md`.
///
/// This is why a click is tested against the hexagon and not `rect`
/// itself: two neighbouring hexes in the campaign's staggered grid overlap
/// at their bounding boxes' corners, so a box test would sometimes pick the
/// wrong hex near an edge.
#[must_use]
pub fn hex_contains(rect: [f32; 4], point: (f32, f32)) -> bool {
    let [x, y, width, height] = rect;
    if width <= 0.0 || height <= 0.0 {
        return false;
    }
    let radius = width * 0.5;
    let top = radius * 3.0_f32.sqrt() * 0.5;
    let dx = (point.0 - (x + width * 0.5)).abs();
    let dy = (point.1 - (y + height * 0.5)).abs();
    dy <= top && dx <= radius - dy / 3.0_f32.sqrt()
}

/// The index of the row a point is over, given rows of one pitch from one
/// origin.
///
/// The shape almost every list on these screens has - the language picker,
/// the disc chooser, a PSP title's menu page, Pure's selection screens - so
/// it is written once. `count` bounds the answer; a point above the first
/// row or past the last returns `None`. `left` and `width` bound it
/// horizontally, so a click in the margin beside a list is not a click on
/// it.
#[must_use]
pub fn row_at(
    point: (f32, f32),
    left: f32,
    width: f32,
    top: f32,
    pitch: f32,
    count: usize,
) -> Option<usize> {
    if count == 0 || pitch <= 0.0 || width <= 0.0 {
        return None;
    }
    if point.0 < left || point.0 >= left + width || point.1 < top {
        return None;
    }
    let row = ((point.1 - top) / pitch).floor();
    // A row past the end is not a row, and `as usize` on a large float
    // would saturate to one that is.
    if row >= count as f32 {
        return None;
    }
    Some(row as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_default_pointer_is_idle_and_nowhere() {
        let pointer = Pointer::default();
        assert!(pointer.is_idle());
        assert_eq!(pointer.position(), None);
    }

    #[test]
    fn any_action_makes_the_pointer_not_idle() {
        for pointer in [
            Pointer {
                moved: true,
                ..Pointer::default()
            },
            Pointer {
                clicked: true,
                ..Pointer::default()
            },
            Pointer {
                back: true,
                ..Pointer::default()
            },
            Pointer {
                scroll: -1,
                ..Pointer::default()
            },
            Pointer {
                drag: (0.0, 3.0),
                ..Pointer::default()
            },
            Pointer {
                pressed: true,
                ..Pointer::default()
            },
            Pointer {
                released: true,
                ..Pointer::default()
            },
        ] {
            assert!(!pointer.is_idle(), "{pointer:?}");
        }
    }

    /// The built-in set's capitals fill their 5x7 box from row 0, so its
    /// band is the box; a face with room above the cap line gets its band
    /// moved down to the cap.
    #[test]
    fn row_ink_reads_the_capitals_and_centres_the_band_on_them() {
        let ink = RowInk::measure(&crate::font::Atlas::build()).expect("the built-in set has ink");
        assert_eq!((ink.top, ink.bottom), (0.0, 7.0));
        assert_eq!(ink.band_top(100.0, 1.0, 7.0), 100.0);
        let pure_like = RowInk {
            top: 7.0,
            bottom: 16.0,
        };
        // Cap centre at 11.5 font pixels, 13.225 at 1.15, less half a pitch.
        let top = pure_like.band_top(46.0, 1.15, 14.95);
        assert!((top - (46.0 + 13.225 - 7.475)).abs() < 1e-4, "{top}");
    }

    #[test]
    fn contains_is_half_open_on_the_far_edges() {
        let rect = [10.0, 20.0, 30.0, 40.0];
        assert!(contains(rect, (10.0, 20.0)));
        assert!(contains(rect, (39.9, 59.9)));
        assert!(!contains(rect, (40.0, 30.0)));
        assert!(!contains(rect, (20.0, 60.0)));
        assert!(!contains(rect, (9.9, 30.0)));
        assert!(!contains([10.0, 10.0, 0.0, 10.0], (10.0, 10.0)));
    }

    #[test]
    fn hex_contains_the_middle_and_not_the_boxs_corners() {
        let rect = [0.0, 0.0, 32.0, 32.0];
        assert!(hex_contains(rect, (16.0, 16.0)), "dead centre");
        assert!(hex_contains(rect, (1.0, 16.0)), "left vertex, mid-height");
        assert!(hex_contains(rect, (31.0, 16.0)), "right vertex, mid-height");
        assert!(
            !hex_contains(rect, (0.0, 0.0)),
            "top-left corner of the box is outside the hexagon"
        );
        assert!(!hex_contains(rect, (31.0, 0.0)), "top-right corner");
        assert!(!hex_contains(rect, (0.0, 31.0)), "bottom-left corner");
        assert!(!hex_contains(rect, (31.0, 31.0)), "bottom-right corner");
        assert!(
            !hex_contains([0.0, 0.0, 0.0, 32.0], (0.0, 0.0)),
            "no extent"
        );
    }

    /// Two hexes side by side the way the campaign grid draws them - 30px
    /// apart horizontally, staggered 19px vertically - whose bounding boxes
    /// overlap at a corner. A box test would sometimes pick whichever hex
    /// happens to be listed first; the hexagon test picks the one whose own
    /// shape the point is actually inside.
    #[test]
    fn a_corner_where_two_hex_boxes_overlap_resolves_to_the_hexagon_that_contains_it() {
        let left = [0.0, 19.0, 32.0, 32.0];
        let right = [30.0, 0.0, 32.0, 32.0];
        let point = (31.0, 16.0);
        assert!(!hex_contains(left, point), "outside left's own hexagon");
        assert!(hex_contains(right, point), "inside right's own hexagon");
    }

    #[test]
    fn a_point_in_the_gap_between_hexes_is_in_neither() {
        let a = [0.0, 0.0, 32.0, 32.0];
        let b = [30.0, 19.0, 32.0, 32.0];
        let point = (0.0, 31.0);
        assert!(!hex_contains(a, point));
        assert!(!hex_contains(b, point));
    }

    #[test]
    fn row_at_steps_by_pitch_and_stops_at_the_count() {
        assert_eq!(row_at((50.0, 100.0), 0.0, 480.0, 100.0, 28.0, 3), Some(0));
        assert_eq!(row_at((50.0, 127.9), 0.0, 480.0, 100.0, 28.0, 3), Some(0));
        assert_eq!(row_at((50.0, 128.0), 0.0, 480.0, 100.0, 28.0, 3), Some(1));
        assert_eq!(row_at((50.0, 183.9), 0.0, 480.0, 100.0, 28.0, 3), Some(2));
        assert_eq!(row_at((50.0, 184.0), 0.0, 480.0, 100.0, 28.0, 3), None);
        assert_eq!(row_at((50.0, 99.0), 0.0, 480.0, 100.0, 28.0, 3), None);
    }

    #[test]
    fn row_at_respects_the_horizontal_band_and_an_empty_list() {
        assert_eq!(row_at((5.0, 100.0), 10.0, 100.0, 100.0, 28.0, 3), None);
        assert_eq!(row_at((110.0, 100.0), 10.0, 100.0, 100.0, 28.0, 3), None);
        assert_eq!(row_at((50.0, 100.0), 0.0, 480.0, 100.0, 28.0, 0), None);
        assert_eq!(row_at((50.0, 100.0), 0.0, 480.0, 100.0, 0.0, 3), None);
    }
}
