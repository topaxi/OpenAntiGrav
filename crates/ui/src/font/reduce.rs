//! An atlas at a fraction of its size: how a face fits a texture that is smaller than the picture
//! a texture pack gave it, or smaller than the glyphs added beside it.

use super::*;

impl Atlas {
    /// This atlas with every `d` by `d` block of texels averaged to one, so a glyph keeps its size
    /// on screen ([`Self::texel_scale`] grows by `d`, the pen advance shrinks by it) and the atlas
    /// holds `1/d` of the texels on each axis. `1` is the atlas as it was.
    ///
    /// The opaque solid patch ([`Self::solid`]) splits the atlas into the glyph rows above it and
    /// the rows below (the substitute button glyphs, [`Self::with_prompts`]); each band is reduced
    /// on its own and the patch is re-stamped between them, so a fill never samples a glyph.
    /// A cell lands exactly when its edges are multiples of `d`, which a pack picture's are for
    /// any `d` that divides its factor; otherwise its box grows outward to the texels it touches.
    /// A built-in 5x7 atlas is returned as it is.
    #[must_use]
    pub fn reduced(&self, d: u32) -> Self {
        if d <= 1 || !self.real {
            return self.clone();
        }
        let (top, tail) = (self.solid.y, self.solid.y + 2);
        let glyph_rows = top.div_ceil(d);
        let width = self.width.div_ceil(d);
        let height = glyph_rows + 2 + self.height.saturating_sub(tail).div_ceil(d);
        let mut coverage = vec![0u8; (width * height) as usize];
        let mut luma = vec![0u8; (width * height) as usize];
        let band = |from: u32, to: u32, row: u32, coverage: &mut [u8], luma: &mut [u8]| {
            let rows = (to - from).div_ceil(d);
            for out_y in 0..rows {
                let (y0, y1) = (from + out_y * d, (from + (out_y + 1) * d).min(to));
                for out_x in 0..width {
                    let (x0, x1) = (out_x * d, ((out_x + 1) * d).min(self.width));
                    let (mut alpha, mut body, mut count) = (0u32, 0u32, 0u32);
                    for y in y0..y1 {
                        for x in x0..x1 {
                            let at = (y * self.width + x) as usize;
                            let a = u32::from(self.coverage[at]);
                            alpha += a;
                            body += a * u32::from(self.luma[at]);
                            count += 1;
                        }
                    }
                    let at = ((row + out_y) * width + out_x) as usize;
                    coverage[at] = (alpha + count / 2).checked_div(count).unwrap_or(0) as u8;
                    luma[at] = (body + alpha / 2).checked_div(alpha).unwrap_or(0) as u8;
                }
            }
        };
        band(0, top, 0, &mut coverage, &mut luma);
        band(
            tail.min(self.height),
            self.height,
            glyph_rows + 2,
            &mut coverage,
            &mut luma,
        );
        let solid = solid_patch(&mut coverage, width, self.solid.x / d, glyph_rows);
        solid_patch(&mut luma, width, self.solid.x / d, glyph_rows);

        let row_of = |y: u32| {
            if y < tail {
                y / d
            } else {
                glyph_rows + 2 + (y - tail) / d
            }
        };
        let row_end = |y: u32| {
            if y <= top {
                y.div_ceil(d)
            } else {
                glyph_rows + 2 + (y - tail).div_ceil(d)
            }
        };
        let glyphs = self
            .glyphs
            .iter()
            .map(|(ch, cell)| {
                let (x, y) = (cell.x / d, row_of(cell.y));
                let cell = Cell {
                    x,
                    y,
                    width: (cell.x + cell.width).div_ceil(d).saturating_sub(x).max(1),
                    height: row_end(cell.y + cell.height).saturating_sub(y).max(1),
                    advance: cell.advance / d as f32,
                    extend: cell.extend.div_ceil(d),
                };
                (*ch, cell)
            })
            .collect();
        Self {
            width,
            height,
            coverage,
            luma,
            solid,
            glyphs,
            texel_scale: self.texel_scale * d as f32,
            border_extend: self.border_extend.div_ceil(d),
            line_height: self.line_height,
            real: self.real,
            outlined: self.outlined,
        }
    }

    /// This atlas reduced by the smallest whole factor that makes both its sides at most `side`:
    /// the guard for a device whose textures are smaller than the atlas it was handed.
    #[must_use]
    pub fn fitting(self, side: u32) -> Self {
        let side = side.max(1);
        if self.width.max(self.height) <= side {
            return self;
        }
        let mut d = self.width.max(self.height).div_ceil(side);
        loop {
            let reduced = self.reduced(d);
            if reduced.width.max(reduced.height) <= side || d >= self.width.max(self.height) {
                return reduced;
            }
            d += 1;
        }
    }
}
