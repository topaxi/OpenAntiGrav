//! The progression bar and the labels over the screen's regions.
//!
//! Split out of `loading.rs` under the 1,000-line rule. Both are Wipeout HD's:
//! a title with no illustration draws neither.
//!
//! # The bar is a grid of dots
//!
//! `LoadingScreen_Draw` (`0x002b61c8`) fills a rectangle of `dot.gtf` tiles, one
//! tile per column and row, in two quads: the lit columns in `HD_Blue` from the
//! left edge, the rest in `HD_LightGrey`. The race-load variant's grid is **166
//! columns by 30 rows** of 8-unit tiles: the right edge `0x44bb0000` is 1496.0
//! and the left 168.0, `(1496 - 168) / 8 = 166`, which is also the float the
//! fill is multiplied by (`DAT_008b35a8`, 166.0) and what a live frame counts
//! (a 7.5 px pitch over a 1251 px bar). The lit count is `(int)obj+0x104`, so
//! the fill moves a whole column at a time. Confidence 85.
//!
//! The lit colour is `HD_Blue` exactly: on the served Fury archive it is
//! `0xffac0717`, and the lit dots of a live frame peak at (172, 7, 23), the same
//! three bytes. So the red is the palette's value, not a team or style colour.
//!
//! **Where this build departs**: 166 columns are kept and the rows are 21, not
//! 30, because this layout's bar is a flatter box (chosen, not measured).

use super::{
    Align, BAR_BOX, CORNER_SIZE, Draw, IMAGE_BOX, MARKER_SIZE, PANEL_RIGHT, PANEL_X, PROSE_BOX,
    RULE_TOP_Y, Screen, corners,
};

/// Dot columns across the bar. Measured: see the module documentation.
pub(super) const COLUMNS: f32 = 166.0;

/// Dot rows down the bar. Chosen, not measured: the original has 30.
const ROWS: f32 = 21.0;

/// One dot tile's width, in this layout's units.
const TILE: f32 = BAR_BOX.2 / COLUMNS;

/// The label text's scale against the heading's `1.4`.
///
/// Measured off the original frames: label capitals are about a third of the
/// heading's height.
const LABEL_SCALE: f32 = 0.5;

/// A label's bullet, and how far its text sits right of it. The original's are
/// 16 and 22 units in a 1920 grid, a quarter of that in this one.
const BULLET: f32 = 4.0;
const BULLET_GAP: f32 = 5.5;

/// Where the mode icon's panel is: the gap right of the bar, to the rule's end.
const MODE_BOX: (f32, f32, f32, f32) = (
    BAR_BOX.0 + BAR_BOX.2 + 5.0,
    BAR_BOX.1,
    PANEL_RIGHT - (BAR_BOX.0 + BAR_BOX.2 + 5.0),
    BAR_BOX.3,
);

/// How many whole columns a fraction of the load lights.
#[must_use]
pub(super) fn lit_columns(fraction: f32) -> f32 {
    (fraction.clamp(0.0, 1.0) * COLUMNS).floor()
}

impl Screen {
    /// The bar: dots where the title ships `dot.gtf`, flat rectangles where it
    /// does not, with its bracket corners.
    pub(super) fn draw_bar(
        &self,
        out: &mut Vec<Draw>,
        fraction: f32,
        trough: [f32; 4],
        fill: [f32; 4],
        marks: [f32; 4],
    ) {
        let lit = lit_columns(fraction);
        match self.dot {
            Some(uv) => {
                let height = ROWS * TILE;
                let mut run = |first: f32, columns: f32, color: [f32; 4]| {
                    if columns > 0.0 {
                        out.push(Draw::TiledSprite {
                            rect: [BAR_BOX.0 + first * TILE, BAR_BOX.1, columns * TILE, height],
                            uv,
                            repeat: [columns, ROWS],
                            color,
                        });
                    }
                };
                run(0.0, lit, fill);
                run(lit, COLUMNS - lit, trough);
            }
            None => {
                out.push(Draw::Fill {
                    rect: [BAR_BOX.0, BAR_BOX.1, BAR_BOX.2, BAR_BOX.3],
                    color: trough,
                });
                if lit > 0.0 {
                    out.push(Draw::Fill {
                        rect: [BAR_BOX.0, BAR_BOX.1, lit * TILE, BAR_BOX.3],
                        color: fill,
                    });
                }
            }
        }
        if let Some(uv) = self.corner {
            for bx in [BAR_BOX, MODE_BOX] {
                for (x, y) in corners(bx) {
                    out.push(Draw::Sprite {
                        rect: [x, y, CORNER_SIZE, CORNER_SIZE],
                        uv,
                        color: marks,
                    });
                }
            }
        }
    }

    /// The five labels, each behind its square bullet.
    ///
    /// **The mode icon itself is not drawn.** The original puts an animated
    /// Bink clip chosen by the mode in that panel (the executable draws a
    /// video widget's texture there); this build has no clip for it, so the
    /// panel is its label and brackets and nothing else rather than a stand-in.
    pub(super) fn draw_labels(&self, out: &mut Vec<Draw>, ink: [f32; 4], border: Option<[f32; 4]>) {
        let Some(labels) = &self.labels else { return };
        let above_corners = RULE_TOP_Y + 1.5;
        let rows: [(f32, f32, Option<&str>); 5] = [
            (PANEL_X, MARKER_SIZE * 2.0, Some(labels.brand.as_str())),
            (IMAGE_BOX.0, above_corners, labels.feature_image.as_deref()),
            (
                PROSE_BOX.0,
                above_corners,
                labels.feature_description.as_deref(),
            ),
            (
                BAR_BOX.0,
                BAR_BOX.1 - 8.0,
                labels.progression_bar.as_deref(),
            ),
            (MODE_BOX.0, MODE_BOX.1 - 8.0, labels.mode_icon.as_deref()),
        ];
        for (x, y, text) in rows {
            let Some(text) = text else { continue };
            if let Some(uv) = self.square {
                out.push(Draw::Sprite {
                    rect: [x, y + 1.0, BULLET, BULLET],
                    uv,
                    color: ink,
                });
            }
            out.push(Draw::Text {
                x: x + BULLET_GAP,
                y,
                scale: self.text(LABEL_SCALE),
                color: ink,
                border,
                align: Align::Left,
                text: text.to_string(),
                wrap_width: None,
            });
        }
    }
}
