//! Where each sprite goes on the sheet: the free space is kept as the maximal
//! empty rectangles, and a sprite takes the highest, then leftmost, position it
//! fits at.
//!
//! Every sprite is placed as it arrives (its rectangle is recorded in the
//! emitter at once), so nothing can be sorted first. The corpora mix sizes from
//! 32x32 to 1024x1024 in whatever order their effects load, and rows of one
//! height each waste most of the sheet on that mix.

use super::{PAD, SHEET_SIZE};

/// An empty rectangle: left, top, width, height.
type Rect = [u32; 4];

/// The sheet's empty space, as every maximal rectangle of it.
///
/// Each sprite is taken [`PAD`] texels wider and taller than it is, out of a
/// sheet that much larger, so two sprites are always [`PAD`] apart and one may
/// still end exactly on the sheet's right or bottom edge.
#[derive(Debug, Clone)]
pub(super) struct Free(Vec<Rect>);

impl Default for Free {
    fn default() -> Self {
        Self(vec![[0, 0, SHEET_SIZE + PAD, SHEET_SIZE + PAD]])
    }
}

impl Free {
    /// Takes room for a `width` by `height` sprite and returns its top-left
    /// corner, or `None` when no empty rectangle holds it.
    pub(super) fn take(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        self.take_aligned(width, height, 1)
    }

    /// [`Self::take`] at a corner whose coordinates are both multiples of `align`.
    pub(super) fn take_aligned(
        &mut self,
        width: u32,
        height: u32,
        align: u32,
    ) -> Option<(u32, u32)> {
        let (w, h) = (width + PAD, height + PAD);
        let (x, y) = self
            .0
            .iter()
            .filter_map(|&[fx, fy, fw, fh]| {
                let (x, y) = (fx.next_multiple_of(align), fy.next_multiple_of(align));
                (x + w <= fx + fw && y + h <= fy + fh).then_some((x, y))
            })
            .min_by_key(|&(x, y)| (y + h, x))?;
        let mut split = Vec::with_capacity(self.0.len() + 4);
        for &[fx, fy, fw, fh] in &self.0 {
            if x >= fx + fw || x + w <= fx || y >= fy + fh || y + h <= fy {
                split.push([fx, fy, fw, fh]);
                continue;
            }
            if x > fx {
                split.push([fx, fy, x - fx, fh]);
            }
            if x + w < fx + fw {
                split.push([x + w, fy, fx + fw - x - w, fh]);
            }
            if y > fy {
                split.push([fx, fy, fw, y - fy]);
            }
            if y + h < fy + fh {
                split.push([fx, y + h, fw, fy + fh - y - h]);
            }
        }
        split.sort_unstable();
        split.dedup();
        self.0 = split
            .iter()
            .filter(|&a| !split.iter().any(|b| a != b && holds(b, a)))
            .copied()
            .collect();
        Some((x, y))
    }
}

/// Whether `outer` covers all of `inner`.
fn holds(outer: &Rect, inner: &Rect) -> bool {
    outer[0] <= inner[0]
        && outer[1] <= inner[1]
        && outer[0] + outer[2] >= inner[0] + inner[2]
        && outer[1] + outer[3] >= inner[1] + inner[3]
}
