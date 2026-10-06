//! `borderExtendPixels`: the glyph quad grown to take in the baked halo.
//!
//! Its own file for the 1,000-line rule in `scripts/check-file-size.py`.

use super::*;

impl Atlas {
    /// This atlas drawing each glyph up to `px` atlas pixels past its box, the
    /// `borderExtendPixels` its language definition authors. `0` is identity.
    ///
    /// **Each glyph's reach is cut to half its distance to the nearest other
    /// glyph's box**, so two neighbours' halos never overlap and no quad
    /// samples the next letter. Pulse's PSP menu faces pack their boxes 1 px
    /// apart against an authored 3 and 5 (reach 0: drawn as before), 2048's
    /// `NEOSANS` 6 px apart against 15 (reach 3); extended in full, every quad
    /// draws a sliver of its neighbour. The PS2's menu faces and every HUD face
    /// are packed far enough apart to get all of it. **Chosen, not measured**:
    /// what the original does with a tightly packed face is not captured.
    #[must_use]
    pub fn with_border_extend(mut self, px: u32) -> Self {
        self.border_extend = px;
        let boxes: Vec<(char, Cell)> = self.glyphs.iter().map(|(c, cell)| (*c, *cell)).collect();
        for (index, (ch, cell)) in boxes.iter().enumerate() {
            let (x0, y0) = (i64::from(cell.x), i64::from(cell.y));
            let (x1, y1) = (x0 + i64::from(cell.width), y0 + i64::from(cell.height));
            let mut reach = i64::from(px);
            for (other_index, (_, other)) in boxes.iter().enumerate() {
                // A codepoint sharing this glyph's very box (a caps-only face
                // maps both cases to one) is the same ink, not a neighbour.
                if other_index == index
                    || (other.x, other.y, other.width, other.height)
                        == (cell.x, cell.y, cell.width, cell.height)
                {
                    continue;
                }
                let (ox0, oy0) = (i64::from(other.x), i64::from(other.y));
                let (ox1, oy1) = (ox0 + i64::from(other.width), oy0 + i64::from(other.height));
                let gap_x = (ox0 - x1).max(x0 - ox1);
                let gap_y = (oy0 - y1).max(y0 - oy1);
                reach = reach.min(gap_x.max(gap_y).max(0) / 2);
            }
            if let Some(slot) = self.glyphs.get_mut(ch) {
                slot.extend = reach as u32;
            }
        }
        self
    }

    /// The quad `cell` draws as with its pen at `(pen, y)`: `(rect, uv)`, the
    /// box grown by [`Cell::extend`] on all four sides in both spaces.
    ///
    /// `uv` is clamped to the glyph rows of the atlas so a bottom-row glyph
    /// never samples the opaque solid patch appended below them. Where the
    /// clamp bites the rect shrinks by the same amount, so the texel size on
    /// screen stays constant.
    #[must_use]
    pub fn glyph_quad(&self, cell: &Cell, pen: f32, y: f32, scale: f32) -> ([f32; 4], [f32; 4]) {
        let unit = scale * self.texel_scale;
        let grow = cell.extend as f32;
        let (mut left, mut top) = (cell.x as f32 - grow, cell.y as f32 - grow);
        let (mut right, mut bottom) = (
            (cell.x + cell.width) as f32 + grow,
            (cell.y + cell.height) as f32 + grow,
        );
        let (mut dl, mut dt, mut dr, mut db) = (grow, grow, grow, grow);
        let glyph_rows = self.glyph_rows() as f32;
        for (edge, lo, hi, d) in [
            (&mut left, 0.0, self.width as f32, &mut dl),
            (&mut right, 0.0, self.width as f32, &mut dr),
            (&mut top, 0.0, glyph_rows, &mut dt),
            (&mut bottom, 0.0, glyph_rows, &mut db),
        ] {
            let clamped = edge.clamp(lo, hi);
            *d -= (clamped - *edge).abs();
            *edge = clamped;
        }
        let rect = [
            pen - dl * unit,
            y - dt * unit,
            (cell.width as f32 + dl + dr) * unit,
            (cell.height as f32 + dt + db) * unit,
        ];
        (rect, [left, top, right - left, bottom - top])
    }

    /// Rows of the atlas that hold glyphs: everything above the solid patch.
    fn glyph_rows(&self) -> u32 {
        if self.real {
            self.height.saturating_sub(2)
        } else {
            self.height
        }
    }
}
