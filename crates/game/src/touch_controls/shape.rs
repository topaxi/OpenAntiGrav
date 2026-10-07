//! Rounded shapes out of horizontal fill rows.
//!
//! The overlay's draw vocabulary has rectangles and chamfered rectangles but
//! no circle, and a new variant would reach into the rasteriser for one
//! widget. A round button is a stack of rows instead: each row is one
//! [`Draw::Fill`], and rows whose span did not change from the one above are
//! merged, so a rounded square is a handful of fills and a disc is a few
//! dozen. **Spans never overlap**, so a translucent colour has the same
//! alpha everywhere it is drawn - stacked fills are what made the first
//! design read as opaque grey.

use oag_ui::frontend::Draw;

/// Row height in grid units: a quarter of a unit is one pixel at 1080p.
const ROW: f32 = 0.25;

/// A rounded rectangle, `[x, y, w, h]` with a corner `radius` (a radius of
/// half the shorter side is a stadium, or a disc on a square).
#[derive(Debug, Clone, Copy)]
pub(super) struct Rounded {
    pub(super) rect: [f32; 4],
    pub(super) radius: f32,
}

impl Rounded {
    /// The same shape pulled in by `by` on every side.
    pub(super) fn inset(self, by: f32) -> Self {
        let [x, y, w, h] = self.rect;
        Self {
            rect: [
                x + by,
                y + by,
                (w - 2.0 * by).max(0.0),
                (h - 2.0 * by).max(0.0),
            ],
            radius: (self.radius - by).max(0.0),
        }
    }

    /// The horizontal extent at height `y`, or `None` outside the shape.
    fn span(self, y: f32) -> Option<(f32, f32)> {
        let [x, top, w, h] = self.rect;
        if y < top || y > top + h || w <= 0.0 {
            return None;
        }
        let r = self.radius.min(w / 2.0).min(h / 2.0);
        let from_edge = (y - top).min(top + h - y);
        let inset = if from_edge >= r {
            0.0
        } else {
            let dy = r - from_edge;
            r - (r * r - dy * dy).max(0.0).sqrt()
        };
        Some((x + inset, x + w - inset))
    }

    fn rows(self) -> impl Iterator<Item = (f32, f32, f32, f32)> {
        let [_, top, _, h] = self.rect;
        let count = (h / ROW).ceil().max(1.0) as usize;
        (0..count).filter_map(move |i| {
            let y0 = top + i as f32 * ROW;
            let y1 = (y0 + ROW).min(top + h);
            self.span((y0 + y1) / 2.0).map(|(a, b)| (y0, y1, a, b))
        })
    }
}

/// Pushes the fills for spans `(y0, y1, x0, x1)`, merging neighbours with the
/// same span.
fn emit(spans: impl Iterator<Item = (f32, f32, f32, f32)>, color: [f32; 4], out: &mut Vec<Draw>) {
    let mut open: Option<(f32, f32, f32, f32)> = None;
    let flush = |open: Option<(f32, f32, f32, f32)>, out: &mut Vec<Draw>| {
        if let Some((y0, y1, x0, x1)) = open
            && x1 > x0
        {
            out.push(Draw::Fill {
                rect: [x0, y0, x1 - x0, y1 - y0],
                color,
            });
        }
    };
    for (y0, y1, x0, x1) in spans {
        open = match open {
            Some((oy0, oy1, ox0, ox1))
                if (ox0 - x0).abs() < 1e-3
                    && (ox1 - x1).abs() < 1e-3
                    && (oy1 - y0).abs() < 1e-3 =>
            {
                Some((oy0, y1, ox0, ox1))
            }
            other => {
                flush(other, out);
                Some((y0, y1, x0, x1))
            }
        };
    }
    flush(open, out);
}

/// The shape filled, optionally clipped to a rectangle `[x, y, w, h]`.
pub(super) fn fill(shape: Rounded, clip: Option<[f32; 4]>, color: [f32; 4], out: &mut Vec<Draw>) {
    let spans = shape.rows().filter_map(|(y0, y1, a, b)| match clip {
        None => Some((y0, y1, a, b)),
        Some([cx, cy, cw, ch]) => {
            let (a, b) = (a.max(cx), b.min(cx + cw));
            let (y0, y1) = (y0.max(cy), y1.min(cy + ch));
            (b > a && y1 > y0).then_some((y0, y1, a, b))
        }
    });
    emit(spans, color, out);
}

/// The band between the shape and the shape pulled in by `width`, in `color`,
/// and the inside in `body`: the two tile the shape exactly, so nothing is
/// drawn twice.
pub(super) fn button(
    shape: Rounded,
    width: f32,
    edge: [f32; 4],
    body: [f32; 4],
    out: &mut Vec<Draw>,
) {
    let inner = shape.inset(width);
    let (mut left, mut right, mut inside) = (Vec::new(), Vec::new(), Vec::new());
    for (y0, y1, a, b) in shape.rows() {
        match inner.span((y0 + y1) / 2.0) {
            Some((ia, ib)) => {
                left.push((y0, y1, a, ia));
                right.push((y0, y1, ib, b));
                inside.push((y0, y1, ia, ib));
            }
            None => left.push((y0, y1, a, b)),
        }
    }
    emit(left.into_iter(), edge, out);
    emit(right.into_iter(), edge, out);
    emit(inside.into_iter(), body, out);
}

/// A triangle, apex up when `up`, as rows inside `rect`.
pub(super) fn triangle(rect: [f32; 4], up: bool, color: [f32; 4], out: &mut Vec<Draw>) {
    let [x, y, w, h] = rect;
    let count = (h / ROW).ceil().max(1.0) as usize;
    let spans = (0..count).map(|i| {
        let y0 = y + i as f32 * ROW;
        let y1 = (y0 + ROW).min(y + h);
        let t = ((y0 + y1) / 2.0 - y) / h;
        let half = w / 2.0 * if up { t } else { 1.0 - t };
        (y0, y1, x + w / 2.0 - half, x + w / 2.0 + half)
    });
    emit(spans, color, out);
}

/// A triangle whose tip points right when `right`, else left, as rows inside
/// `rect`.
pub(super) fn arrow_side(rect: [f32; 4], right: bool, color: [f32; 4], out: &mut Vec<Draw>) {
    let [x, y, w, h] = rect;
    let count = (h / ROW).ceil().max(1.0) as usize;
    let spans = (0..count).map(|i| {
        let y0 = y + i as f32 * ROW;
        let y1 = (y0 + ROW).min(y + h);
        let t = ((y0 + y1) / 2.0 - y) / h;
        let reach = w * (1.0 - (2.0 * t - 1.0).abs());
        if right {
            (y0, y1, x, x + reach)
        } else {
            (y0, y1, x + w - reach, x + w)
        }
    });
    emit(spans, color, out);
}

/// A shield: straight sides to the middle, then narrowing to a point.
pub(super) fn shield(rect: [f32; 4], color: [f32; 4], out: &mut Vec<Draw>) {
    let [x, y, w, h] = rect;
    let count = (h / ROW).ceil().max(1.0) as usize;
    let spans = (0..count).map(|i| {
        let y0 = y + i as f32 * ROW;
        let y1 = (y0 + ROW).min(y + h);
        let t = ((y0 + y1) / 2.0 - y) / h;
        let half = if t < 0.45 {
            w / 2.0
        } else {
            w / 2.0 * (1.0 - (t - 0.45) / 0.55)
        };
        (y0, y1, x + w / 2.0 - half, x + w / 2.0 + half)
    });
    emit(spans, color, out);
}
