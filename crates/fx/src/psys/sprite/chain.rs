//! The lower levels of a sheet whose sprites carry the file's own mip chains.
//!
//! A title whose loader biases its textures samples a particle sprite down its authored
//! chain, so the atlas needs a level for each. A chained sprite sits on a [`GUTTER`] grid
//! with a gutter of that width on every side, filled by repeating its edge texels at each
//! level, so bilinear filtering at any level reads what a clamped sampler over the
//! sprite's own texture would and never a neighbour.

use std::sync::Arc;

use super::{SHEET_SIZE, Sprite};

/// Levels a chained sheet holds, the base included. Chosen, not measured: files author up to
/// nine, and past this one a [`SHEET_SIZE`] atlas would need a gutter wider than most
/// sprites. At a bias of -1.5 the last level kept is reached only past 45 texels a pixel.
pub const ATLAS_LEVELS: u32 = 5;

/// The grid a chained sprite is placed on and the gutter around it, in base texels: one
/// texel of the last level.
pub const GUTTER: u32 = 1 << (ATLAS_LEVELS - 1);

/// Levels 1 and below of a chained sheet, each [`SHEET_SIZE`] >> level square, grown by
/// rows like the base.
#[derive(Debug, Clone, Default)]
pub(super) struct Lower(Vec<Arc<Vec<u8>>>);

impl Lower {
    /// The rows placed so far on `level` (1 and up), or `None` on a sheet with no chain.
    pub(super) fn level(&self, level: u32) -> Option<&[u8]> {
        let index = usize::try_from(level.checked_sub(1)?).ok()?;
        self.0.get(index).map(|pixels| pixels.as_slice())
    }

    /// Writes every lower level of `sprite`, whose base sits at `(x, y)`.
    pub(super) fn write(&mut self, sprite: &Sprite, chain: &[Arc<[u8]>], (x, y): (u32, u32)) {
        self.0.resize_with(ATLAS_LEVELS as usize - 1, Arc::default);
        let (mut width, mut height) = (u32::from(sprite.width), u32::from(sprite.height));
        let mut above: Arc<[u8]> = sprite.rgba.clone();
        for level in 1..ATLAS_LEVELS {
            let (down_w, down_h) = ((width / 2).max(1), (height / 2).max(1));
            let authored = chain
                .get(level as usize - 1)
                .filter(|texels| texels.len() == (down_w * down_h * 4) as usize);
            // A file that authors fewer levels than the atlas holds: the original stops at
            // its last, which a box filter of it stands in for. Chosen.
            let texels = authored
                .cloned()
                .unwrap_or_else(|| halve(&above, (width, height)).into());
            let pixels = Arc::make_mut(&mut self.0[level as usize - 1]);
            blit(
                pixels,
                SHEET_SIZE >> level,
                &texels,
                (down_w, down_h),
                (x >> level, y >> level),
                GUTTER >> level,
            );
            (above, width, height) = (texels, down_w, down_h);
        }
    }
}

/// Copies a `width` by `height` image to `(x, y)` of a `side`-square sheet and repeats its
/// edge texels `gutter` texels out on every side, clipped to the sheet. `pixels` grows to
/// the last row written.
pub(super) fn blit(
    pixels: &mut Vec<u8>,
    side: u32,
    image: &[u8],
    (width, height): (u32, u32),
    (x, y): (u32, u32),
    gutter: u32,
) {
    let side = i64::from(side);
    let (x, y, gutter) = (i64::from(x), i64::from(y), i64::from(gutter));
    let (width, height) = (i64::from(width), i64::from(height));
    let stride = side as usize * 4;
    let rows = (y + height + gutter).min(side).max(0) as usize;
    if pixels.len() < rows * stride {
        pixels.reserve_exact(rows * stride - pixels.len());
        pixels.resize(rows * stride, 0);
    }
    for line in (y - gutter).max(0)..(y + height + gutter).min(side) {
        let from = (line - y).clamp(0, height - 1);
        for column in (x - gutter).max(0)..(x + width + gutter).min(side) {
            let texel = ((from * width + (column - x).clamp(0, width - 1)) * 4) as usize;
            let to = line as usize * stride + column as usize * 4;
            pixels[to..to + 4].copy_from_slice(&image[texel..texel + 4]);
        }
    }
}

/// A 2x2 box filter of a `width` by `height` image, odd edges repeating their last texel.
fn halve(image: &[u8], (width, height): (u32, u32)) -> Vec<u8> {
    let (down_w, down_h) = ((width / 2).max(1), (height / 2).max(1));
    let mut out = Vec::with_capacity((down_w * down_h * 4) as usize);
    for y in 0..down_h {
        for x in 0..down_w {
            let mut sum = [0u32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let at = ((y * 2 + dy).min(height - 1) * width + (x * 2 + dx).min(width - 1))
                    as usize
                    * 4;
                for (c, total) in sum.iter_mut().enumerate() {
                    *total += u32::from(image[at + c]);
                }
            }
            out.extend(sum.map(|total| ((total + 2) / 4) as u8));
        }
    }
    out
}
