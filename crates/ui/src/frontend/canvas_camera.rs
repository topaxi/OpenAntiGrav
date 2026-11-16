//! The camera of 2048's `Canvas3D` item, and how the shell's scroll moves it.
//!
//! The campaign map is the triangle lattice of [`super::canvas_lattice`] with
//! cells painted and raised: an event is a hexagon of six cells, and where it
//! lands on the screen is this camera applied to its cells. Read off
//! `docs/ghidra/functions/vita-2048-eu-v104/campaign-map-projection.md`
//! ("The law") and `canvas3d-lattice.md` ("The camera is tilted").

use super::canvas_lattice::{CELL_DX, CELL_DY};

/// The viewport the projection works in, and its centre: the point a centred
/// event is brought to.
pub const VIEWPORT: (f32, f32) = (960.0, 544.0);
/// See [`VIEWPORT`].
pub const CENTRE: (f32, f32) = (480.0, 272.0);
/// The camera's Euler angles about `x` and `y`, radians.
const TILT: (f32, f32) = (-0.45, -0.1);
/// Half-width of the orthographic window, and its height over its width.
const HALF_WIDTH: f32 = 8.0;
const HEIGHT_RATIO: f32 = 0.555_555_6;
/// The pan the item's reset stores.
const RESET_PAN: (f32, f32) = (27.162_66, 0.395_522);
/// A painted tile's height: its cells' alpha, `0xfa` of 255.
pub const TILE_HEIGHT: f32 = 250.0 / 255.0;
/// The eased re-centre ends once the scroll is this close to its target.
const NEAR_TARGET: f32 = 25.0;
/// The share of the remaining distance one 60 Hz frame of the ease covers
/// (rate 5.0 a second).
const EASE_PER_FRAME: f32 = 5.0 / 60.0;

/// The five coefficients of the canvas matrix that do not move with the pan:
/// `x' = ax X + bx Y + zx Z`, `y' = by Y + zy Z`.
struct Rows {
    ax: f32,
    bx: f32,
    zx: f32,
    by: f32,
    zy: f32,
}

fn rows() -> Rows {
    let (sin_x, cos_x) = oag_core::math::sin_cos(TILT.0);
    let (sin_y, cos_y) = oag_core::math::sin_cos(TILT.1);
    let vertical = HALF_WIDTH * HEIGHT_RATIO;
    Rows {
        ax: cos_y / HALF_WIDTH,
        bx: -(sin_y * sin_x) / HALF_WIDTH,
        zx: sin_y * cos_x / HALF_WIDTH,
        by: -cos_x / vertical,
        zy: -sin_x / vertical,
    }
}

/// The plane distance the item moves its pan by for a scroll of one whole
/// viewport: twice the distance from the screen centre to the bottom right
/// corner, so the `x` span carries the camera's shear and a scroll of `d`
/// moves the picture a little further than `d`.
#[must_use]
pub fn span() -> (f32, f32) {
    let m = rows();
    let y = -1.0 / m.by;
    let x = (1.0 - m.bx * y) / m.ax;
    (2.0 * x, 2.0 * y)
}

/// The canvas camera at one pan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pan: (f32, f32),
}

impl Camera {
    /// The camera for a view scrolled `offset` pixels right and down of the
    /// reset camera. The original's scroll position is the negative of this.
    #[must_use]
    pub fn scrolled(offset: (f32, f32)) -> Self {
        let span = span();
        Self {
            pan: (
                RESET_PAN.0 + offset.0 / VIEWPORT.0 * span.0,
                RESET_PAN.1 - offset.1 / VIEWPORT.1 * span.1,
            ),
        }
    }

    /// Where a point of the canvas lands, in viewport pixels.
    #[must_use]
    pub fn project(&self, world: [f32; 3]) -> (f32, f32) {
        let m = rows();
        let (x, y, z) = (
            world[0] - self.pan.0,
            world[1] + self.pan.1,
            world[2] - HALF_WIDTH,
        );
        let clip = (m.ax * x + m.bx * y + m.zx * z, m.by * y + m.zy * z);
        (
            (clip.0 + 1.0) * 0.5 * VIEWPORT.0,
            (1.0 - clip.1) * 0.5 * VIEWPORT.1,
        )
    }

    /// The screen displacement between two lattice points one unit apart
    /// along `x`, and between two a row apart (half a unit across, one row
    /// down): the two vectors the lattice picture repeats by.
    #[must_use]
    pub fn lattice_period() -> ((f32, f32), (f32, f32)) {
        let camera = Self::scrolled((0.0, 0.0));
        let origin = camera.project([0.0, 0.0, 0.0]);
        let across = camera.project([1.0, 0.0, 0.0]);
        let down = camera.project([CELL_DX, CELL_DY, 0.0]);
        (
            (across.0 - origin.0, across.1 - origin.1),
            (down.0 - origin.0, down.1 - origin.1),
        )
    }
}

/// The ground point the original centres an event by: the middle of the first
/// of its six cells, on the lattice plane.
#[must_use]
pub fn cell_ground(cell: (f32, f32)) -> [f32; 3] {
    [cell.0 * CELL_DX, cell.1 * CELL_DY, 0.0]
}

/// The middle of a tile's top face: the lattice vertex its six cells
/// `(col .. col + 2) x (row, row + 1)` share, at the tile's height.
#[must_use]
pub fn tile_top(cell: (f32, f32)) -> [f32; 3] {
    [
        (cell.0 + 1.0) * CELL_DX,
        (cell.1 + 0.5) * CELL_DY,
        TILE_HEIGHT,
    ]
}

/// The size of a tile's top face on screen: a hexagon two units across and
/// two rows tall.
#[must_use]
pub fn tile_face() -> (f32, f32) {
    let (across, down) = Camera::lattice_period();
    (2.0 * across.0, 2.0 * down.1)
}

/// The view offset that brings `cell`'s ground point to the screen centre,
/// as the original computes it from a view already at `offset`: the point's
/// place on screen now, less the centre, added to where the view is. The shear
/// in [`span`] makes one application land a few pixels off in `x`.
#[must_use]
pub fn centred_on(cell: (f32, f32), offset: (f32, f32)) -> (f32, f32) {
    let at = Camera::scrolled(offset).project(cell_ground(cell));
    (at.0 + offset.0 - CENTRE.0, at.1 + offset.1 - CENTRE.1)
}

/// The view the map opens on for the event at `cell`: the reset camera
/// centred directly, then an eased second centring that stops as soon as it
/// is within [`NEAR_TARGET`] pixels of its target. The original waits a second
/// between the two and plays the ease; this is where it comes to rest.
#[must_use]
pub fn opening_offset(cell: (f32, f32)) -> (f32, f32) {
    let mut offset = centred_on(cell, (0.0, 0.0));
    let target = centred_on(cell, offset);
    let near = |at: (f32, f32)| {
        let (dx, dy) = (target.0 - at.0, target.1 - at.1);
        dx * dx + dy * dy <= NEAR_TARGET * NEAR_TARGET
    };
    while !near(offset) {
        offset.0 += (target.0 - offset.0) * EASE_PER_FRAME;
        offset.1 += (target.1 - offset.1) * EASE_PER_FRAME;
    }
    offset
}

#[cfg(test)]
mod tests;
