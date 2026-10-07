//! Rounded shapes out of a few textured quads.
//!
//! The overlay's draw vocabulary has no circle, so a rounded rectangle is
//! nine cells: four corners that are quarters of the disc or ring texture
//! (see [`super::art`]) and five that are plain fills. A button is a ring and
//! a disc inset by one ring, so **the two tile the shape and nothing is drawn
//! twice** - stacked translucent layers are what made the first design read
//! as flat grey. A lit region is the same cells cut to a rectangle, with each
//! corner's texture coordinates cut with it.

use oag_ui::frontend::Draw;

use super::art::{Art, RING_RATIO};

/// A rounded rectangle, `[x, y, w, h]` with a corner `radius` (half the
/// shorter side is a stadium, or a disc on a square).
#[derive(Debug, Clone, Copy)]
pub(super) struct Rounded {
    pub(super) rect: [f32; 4],
    pub(super) radius: f32,
}

impl Rounded {
    /// The ring's thickness for this shape.
    pub(super) fn border(self) -> f32 {
        self.radius.min(self.rect[2] / 2.0).min(self.rect[3] / 2.0) * RING_RATIO
    }

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
}

fn overlap(a: [f32; 4], b: [f32; 4]) -> Option<[f32; 4]> {
    let x0 = a[0].max(b[0]);
    let y0 = a[1].max(b[1]);
    let x1 = (a[0] + a[2]).min(b[0] + b[2]);
    let y1 = (a[1] + a[3]).min(b[1] + b[3]);
    (x1 - x0 > 1e-4 && y1 - y0 > 1e-4).then_some([x0, y0, x1 - x0, y1 - y0])
}

/// One corner cell: the matching quarter of `texture`, cut to `clip`.
fn corner(
    cell: [f32; 4],
    quarter: [f32; 4],
    clip: Option<[f32; 4]>,
    color: [f32; 4],
    out: &mut Vec<Draw>,
) {
    let shown = match clip {
        Some(clip) => overlap(cell, clip),
        None => Some(cell),
    };
    let Some(shown) = shown else { return };
    let u = (shown[0] - cell[0]) / cell[2];
    let v = (shown[1] - cell[1]) / cell[3];
    out.push(Draw::Sprite {
        rect: shown,
        uv: [
            quarter[0] + u * quarter[2],
            quarter[1] + v * quarter[3],
            shown[2] / cell[2] * quarter[2],
            shown[3] / cell[3] * quarter[3],
        ],
        color,
    });
}

/// The nine cells of `shape`, as `(column, row, rect)` with columns and rows
/// 0 to 2.
fn cells(shape: Rounded) -> [(usize, usize, [f32; 4]); 9] {
    let [x, y, w, h] = shape.rect;
    let r = shape.radius.min(w / 2.0).min(h / 2.0).max(0.0);
    let xs = [x, x + r, x + w - r, x + w];
    let ys = [y, y + r, y + h - r, y + h];
    let mut out = [(0, 0, [0.0; 4]); 9];
    for row in 0..3 {
        for col in 0..3 {
            out[row * 3 + col] = (
                col,
                row,
                [
                    xs[col],
                    ys[row],
                    xs[col + 1] - xs[col],
                    ys[row + 1] - ys[row],
                ],
            );
        }
    }
    out
}

fn quarter(texture: [f32; 4], col: usize, row: usize) -> [f32; 4] {
    let (hw, hh) = (texture[2] / 2.0, texture[3] / 2.0);
    [
        texture[0] + if col == 2 { hw } else { 0.0 },
        texture[1] + if row == 2 { hh } else { 0.0 },
        hw,
        hh,
    ]
}

/// The shape filled, optionally cut to a rectangle `[x, y, w, h]`.
pub(super) fn fill(
    art: &Art,
    shape: Rounded,
    clip: Option<[f32; 4]>,
    color: [f32; 4],
    out: &mut Vec<Draw>,
) {
    for (col, row, cell) in cells(shape) {
        if (col == 1 && cell[2] <= 0.0) || (row == 1 && cell[3] <= 0.0) {
            continue;
        }
        if col != 1 && row != 1 {
            corner(cell, quarter(art.disc, col, row), clip, color, out);
        } else if let Some(shown) = clip.map_or(Some(cell), |c| overlap(cell, c)) {
            out.push(Draw::Fill { rect: shown, color });
        }
    }
}

/// The ring around `shape`, one [`Rounded::border`] thick.
pub(super) fn ring(art: &Art, shape: Rounded, color: [f32; 4], out: &mut Vec<Draw>) {
    let t = shape.border();
    for (col, row, cell) in cells(shape) {
        if col != 1 && row != 1 {
            corner(cell, quarter(art.ring, col, row), None, color, out);
            continue;
        }
        if cell[2] <= 0.0 || cell[3] <= 0.0 || (col == 1 && row == 1) {
            continue;
        }
        let [x, y, w, h] = cell;
        let strip = match (col, row) {
            (1, 0) => [x, y, w, t],
            (1, 2) => [x, y + h - t, w, t],
            (0, 1) => [x, y, t, h],
            _ => [x + w - t, y, t, h],
        };
        out.push(Draw::Fill { rect: strip, color });
    }
}

/// A button: the ring in `edge` and, inside it, the body in `body`.
pub(super) fn button(
    art: &Art,
    shape: Rounded,
    edge: [f32; 4],
    body: [f32; 4],
    out: &mut Vec<Draw>,
) {
    ring(art, shape, edge, out);
    fill(art, shape.inset(shape.border()), None, body, out);
}

/// One texture stretched over `rect`, turned clockwise by `rotation`
/// radians about its centre.
pub(super) fn sprite(
    rect: [f32; 4],
    uv: [f32; 4],
    rotation: f32,
    color: [f32; 4],
    out: &mut Vec<Draw>,
) {
    out.push(if rotation == 0.0 {
        Draw::Sprite { rect, uv, color }
    } else {
        Draw::RotatedSprite {
            rect,
            uv,
            color,
            rotation,
        }
    });
}
