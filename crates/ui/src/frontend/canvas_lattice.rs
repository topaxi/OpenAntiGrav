//! 2048's pale triangle lattice: the default look of the `Canvas3D` item
//! (`Frontend/Items/3dCanvas_Item.cpp`) behind the title screen and the game
//! mode page.
//!
//! The original builds a 90 x 32 mesh of equilateral triangle cells at
//! `(col * 0.5, row * 0.866)`, apex-up and apex-down alternating, each textured
//! from one of two outlined triangles in the top left of `canvasTexture.gxt`
//! (mip 0 at this scale), vertex colour white, alpha 1, and draws it opaque
//! through an orthographic camera that is **tilted** (Euler angles -0.45 and
//! -0.1 rad). At default state the shader does nothing else: the line grey and
//! width are the atlas's own pixels.
//!
//! The camera is fixed on these pages, so the whole picture is one image:
//! [`bake`] renders it at the 960 x 544 viewport, pixel by pixel, by inverting
//! the executed `Canvas3D_BuildWorldViewProj` matrix back to the lattice plane
//! and reading the atlas there; [`Frontend::canvas_lattice`] stretches it over
//! the screen. Read off
//! `docs/ghidra/functions/vita-2048-eu-v104/canvas3d-lattice.md`.
//!
//! # What is read and what is chosen
//!
//! - **Read**: the cell spacing, the two atlas triangles, the corner offsets,
//!   the cell type from `(col + row)` parity, the tilt angles, the ortho
//!   scales (`1/8`, `1/(8 * 5/9)`), the 960 x 544 viewport.
//! - **Read live**: the matrix translation row at the title in Vita3K,
//!   `(-9.56032, 4.65163)`. It is the reset pan plus the title's
//!   `FeShellTouchScroll` parent at `(-3000, -1500)` (the canvas pan follows
//!   it to `78.76, -26.824`); the writer of that scroll is not pinned.
//! - **Measured, the game's composite pass**: the canvas draws into a
//!   16-bit float scene target and the game's front-end composite reads it
//!   back through a square-root encode (`out = sqrt(scene)`, no exposure,
//!   no bloom: the bloom target is black on these pages), then samples it at
//!   a quarter pixel offset. Applied to an atlas line of 0.86 that is 0.93:
//!   the "about half ink" of the captures. The law is fitted to a RenderDoc
//!   capture of Vita3K at the title (rms 0.17 of 255); the uniform it comes
//!   from is not read.

use super::*;

/// The sheet key of the baked picture. Not a path: nothing on a disc has it.
pub const TILE: &str = "oag:2048-canvas-lattice";

/// Cell spacing along `x` and `y` (`DAT_81732ab4`, `DAT_81732ab8`).
pub const CELL_DX: f32 = 0.5;
/// See [`CELL_DX`]: `sqrt(3) / 2`.
pub const CELL_DY: f32 = 0.866_025_4;

/// The viewport the projection works in.
pub const VIEWPORT: [u32; 2] = [960, 544];
/// The camera's Euler angles about `x` and `y`, radians (`Canvas3D` reset).
const TILT: [f32; 2] = [-0.45, -0.1];
/// Half-width of the orthographic window and the height ratio (`5/9`).
const HALF_WIDTH: f32 = 8.0;
const HEIGHT_RATIO: f32 = 0.555_555_6;
/// The composite pass samples the scene at this offset from the pixel centre,
/// in pixels, on both axes (a 2 x 2 kernel of 0.75 / 0.25 on the capture).
const COMPOSITE_SAMPLE_OFFSET: f32 = 0.25;
/// Translation row of the final matrix (`+0x5ec90`), live at the title.
const TRANSLATION: [f32; 2] = [-9.560_32, 4.651_63];

/// The two outlined triangles in `canvasTexture.gxt`, in atlas pixels: apex
/// up, then apex down (`DAT_81732ac0..aec` times 2048).
const ATLAS_UP: [[f32; 2]; 3] = [[14.0, 122.0], [107.0, 122.0], [60.0, 40.0]];
const ATLAS_DOWN: [[f32; 2]; 3] = [[60.0, 40.0], [107.0, 122.0], [155.0, 40.0]];
/// The cell's corners for each type, in the same order as the atlas triangles
/// (`DAT_81732c38`, `DAT_81732c20`).
const HALF_DY: f32 = CELL_DY / 2.0;
const CORNERS_UP: [[f32; 2]; 3] = [[-0.5, HALF_DY], [0.5, HALF_DY], [0.0, -HALF_DY]];
const CORNERS_DOWN: [[f32; 2]; 3] = [[-0.5, -HALF_DY], [0.0, HALF_DY], [0.5, -HALF_DY]];

fn barycentric(p: [f32; 2], tri: &[[f32; 2]; 3]) -> [f32; 3] {
    let d = (tri[1][1] - tri[2][1]) * (tri[0][0] - tri[2][0])
        + (tri[2][0] - tri[1][0]) * (tri[0][1] - tri[2][1]);
    let a = ((tri[1][1] - tri[2][1]) * (p[0] - tri[2][0])
        + (tri[2][0] - tri[1][0]) * (p[1] - tri[2][1]))
        / d;
    let b = ((tri[2][1] - tri[0][1]) * (p[0] - tri[2][0])
        + (tri[0][0] - tri[2][0]) * (p[1] - tri[2][1]))
        / d;
    [a, b, 1.0 - a - b]
}

/// Where the lattice's own pixel at world `p` reads the atlas, in atlas
/// pixels: the cell containing `p`, the triangle it is textured with, and the
/// barycentric carry from corners to atlas triangle.
fn atlas_at(p: [f32; 2]) -> Option<[f32; 2]> {
    let row = (p[1] / CELL_DY).round();
    let local_y = p[1] - row * CELL_DY;
    let col0 = (p[0] / CELL_DX).floor();
    for col in [col0 - 1.0, col0, col0 + 1.0] {
        let up = (col + row).rem_euclid(2.0) < 0.5;
        let (corners, atlas) = if up {
            (&CORNERS_UP, &ATLAS_UP)
        } else {
            (&CORNERS_DOWN, &ATLAS_DOWN)
        };
        let local = [p[0] - col * CELL_DX, local_y];
        let w = barycentric(local, corners);
        if w.iter().all(|&weight| weight >= -1e-4) {
            return Some([
                w[0] * atlas[0][0] + w[1] * atlas[1][0] + w[2] * atlas[2][0],
                w[0] * atlas[0][1] + w[1] * atlas[1][1] + w[2] * atlas[2][1],
            ]);
        }
    }
    None
}

fn bilinear(rgba: &[u8], width: u32, height: u32, at: [f32; 2]) -> [f32; 3] {
    let x = (at[0] - 0.5).clamp(0.0, (width - 1) as f32);
    let y = (at[1] - 0.5).clamp(0.0, (height - 1) as f32);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let texel = |tx: f32, ty: f32, channel: usize| -> f32 {
        let tx = (tx as u32).min(width - 1);
        let ty = (ty as u32).min(height - 1);
        f32::from(rgba[((ty * width + tx) * 4) as usize + channel])
    };
    let mut out = [0.0; 3];
    for (channel, value) in out.iter_mut().enumerate() {
        let top = texel(x0, y0, channel) * (1.0 - fx) + texel(x0 + 1.0, y0, channel) * fx;
        let bottom =
            texel(x0, y0 + 1.0, channel) * (1.0 - fx) + texel(x0 + 1.0, y0 + 1.0, channel) * fx;
        *value = top * (1.0 - fy) + bottom * fy;
    }
    out
}

/// The 2 x 2 of the final matrix on the lattice plane, row-vector
/// convention: `x' = ax * x + bx * y + t0`, `y' = by * y + t1`.
fn plane_matrix() -> (f32, f32, f32) {
    let (rx, ry) = (TILT[0], TILT[1]);
    let (sin_x, cos_x) = oag_core::math::sin_cos(rx);
    let (sin_y, cos_y) = oag_core::math::sin_cos(ry);
    let ax = cos_y / HALF_WIDTH;
    let bx = -(sin_y * sin_x) / HALF_WIDTH;
    let by = -cos_x / (HALF_WIDTH * HEIGHT_RATIO);
    (ax, bx, by)
}

/// The lattice plane point under the centre of screen pixel `(px, py)`.
fn world_at(px: u32, py: u32) -> [f32; 2] {
    let offset = COMPOSITE_SAMPLE_OFFSET;
    let (ax, bx, by) = plane_matrix();
    let half = [VIEWPORT[0] as f32 / 2.0, VIEWPORT[1] as f32 / 2.0];
    let cx = (px as f32 + 0.5 + offset - half[0]) / half[0];
    let cy = (half[1] - (py as f32 + 0.5 + offset)) / half[1];
    let y = (cy - TRANSLATION[1]) / by;
    let x = (cx - TRANSLATION[0] - bx * y) / ax;
    [x, y]
}

/// The composite's square-root encode of one channel (0 to 255 in, out).
fn composite_encode(channel: f32) -> u8 {
    ((channel / 255.0).clamp(0.0, 1.0).sqrt() * 255.0).round() as u8
}

/// The lattice picture at the viewport size, RGBA, alpha opaque, rendered off
/// the atlas's RGBA at mip 0 and run through the game's composite encode.
///
/// `None` when the atlas is too small to hold the two triangles.
#[must_use]
pub fn bake(atlas: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    if width < 160 || height < 128 || atlas.len() < (width * height * 4) as usize {
        return None;
    }
    let mut out = Vec::with_capacity((VIEWPORT[0] * VIEWPORT[1] * 4) as usize);
    for py in 0..VIEWPORT[1] {
        for px in 0..VIEWPORT[0] {
            let rgb = atlas_at(world_at(px, py))
                .map_or([255.0; 3], |at| bilinear(atlas, width, height, at));
            out.extend(rgb.map(composite_encode));
            out.push(255);
        }
    }
    Some(out)
}

impl Frontend {
    /// Puts the lattice just over the page's white clear (index 1).
    ///
    /// Which pages call it is `wipeout2048.rs`'s list: measured off the
    /// reference captures, the title screen and the game mode page show the
    /// sharp lattice, while the home, team, extras, options and profile pages
    /// show a blurred ship backdrop instead, which is another effect and
    /// unread.
    pub(super) fn insert_lattice(&self, out: &mut Vec<Draw>) {
        if let Some(lattice) = self.canvas_lattice() {
            out.insert(2, lattice);
        }
    }

    /// The baked picture over the whole screen, or nothing when the sheet has
    /// none (no atlas on this disc, or it did not decode; the loader says
    /// which).
    pub(super) fn canvas_lattice(&self) -> Option<Draw> {
        let picture = self.placed(TILE)?;
        let (width, height) = self.space.size;
        Some(Self::sprite_at(
            [0.0, 0.0, width, height],
            picture,
            [1.0; 4],
        ))
    }
}

#[cfg(test)]
mod tests;
