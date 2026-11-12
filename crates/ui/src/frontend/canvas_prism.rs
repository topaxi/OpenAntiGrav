//! An event tile of 2048's campaign map: six painted cells of the `Canvas3D`
//! mesh, raised into a prism, drawn by the canvas's own two programs.
//!
//! The original paints a tile into the canvas's cell textures (colour, and a
//! height in alpha) and lets `Canvas3D_vp`/`Canvas3D_fp` draw the mesh: a
//! cell's top triangle and three walls, each vertex lifted by its cell's
//! alpha, shaded by the cells that share the vertex, textured from the atlas
//! triangle the cell type maps to (`docs/ghidra/functions/vita-2048-eu-v104/canvas3d-lattice.md`,
//! "Mesh law" and "What the shaders compute"; the seven triangles per cell are
//! `Canvas3D_BuildLatticeMesh`'s index writes, `campaign-map-projection.md`,
//! "The law").
//!
//! The camera is orthographic and fixed, so a prism's picture only moves when
//! the view pans, by a translation: [`bake`] runs the programs once, in
//! software, over one tile and the ground around it, and the map draws that
//! picture at each tile's projected place ([`Layout`]).
//!
//! # What the bake keeps of the programs
//!
//! - Vertex height `h = z + alpha` (`z` is 0 on the top, -1 on the skirt),
//!   occlusion `s = 1/6 * sum max(alpha_k - h, 0)` over the six cells that
//!   share the vertex (`sampleOffsets`), vertex colour `(1 - s) * cell colour`.
//! - Fragment `t.rgb * ((1 - t.a) * min(c + 0.005, 1) + t.a)`. A painted cell's
//!   `t` is the atlas's clear texel, `(255, 255, 255, 0)` ([`CLEAR`]): the
//!   button's icon writer (`CanvasButton_PaintIconAndRing` `0x8106fbd8` calling
//!   `0x81072702`) shifts the uvs of its six cells onto the icon's region of
//!   `canvasTexture.gxt` by `(u + 0.0459 k, v + 0.0400 row)`, one atlas triangle
//!   pitch a step, and that region is clear white around the glyph (measured on
//!   the decoded sheet), so the face and the walls, which copy the top's uvs,
//!   carry none of the lattice's outline. The glyph itself is not drawn here:
//!   the map lays the disc's own mode icon over the face (the icon's `u`, `v` per
//!   event shape are unread).
//! - A z-buffer with `LESS_EQUAL`, and the composite pass's square-root encode
//!   ([`super::canvas_lattice`], "Part 2"), which commutes with the cell colour:
//!   `sqrt(t * c * colour) = sqrt(t * c) * sqrt(colour)`. The picture is baked
//!   for a white cell and [`tint`] carries a state's colour.
//! - The ground is a darkening of the lattice picture beneath, not a second
//!   copy of it: a ground pixel is `lattice * sqrt(c)`, written as black with
//!   alpha `1 - sqrt(c)`.
//!
//! **Not read, so not drawn**: the ground relief (the unpainted cells' alpha,
//! which the music spectrum and ripples move), so the ground is flat; the
//! occlusion of a vertex shared with a neighbouring tile, which the bake takes
//! from an isolated tile.

use super::canvas_camera::{Camera, TILE_HEIGHT, tile_top};
use super::canvas_lattice::{CELL_DX, CELL_DY, CORNERS_DOWN, CORNERS_UP, composite_encode};

/// The sheet key of the baked picture. Not a path: nothing on a disc has it.
pub const TILE: &str = "oag:2048-canvas-prism";

/// The cell the bake paints: any tile's first cell would do, every one has
/// `col + row` even.
const CELL: (i32, i32) = (55, 25);
/// The tile's footprint, in cells.
const FOOTPRINT: (i32, i32) = (3, 2);
/// The cells the bake draws, around the footprint: far enough for every
/// vertex that shares a cell with the tile.
const MARGIN: (i32, i32, i32, i32) = (4, 2, 4, 3);
/// Samples per pixel along each axis.
const SUPER: usize = 3;
/// The occlusion's six neighbours of a vertex, `(du, dv)` in cells, by the
/// vertex's `offsetIndices` (`DAT_81732994..`).
const NEIGHBOURS: [[(i32, i32); 6]; 6] = [
    [(0, 0), (-1, 0), (-2, 0), (0, 1), (-1, 1), (-2, 1)],
    [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
    [(-1, 0), (0, 0), (1, 0), (-1, -1), (0, -1), (1, -1)],
    [(0, 0), (-1, 0), (-2, 0), (0, -1), (-1, -1), (-2, -1)],
    [(-1, 0), (0, 0), (1, 0), (-1, 1), (0, 1), (1, 1)],
    [(0, 0), (1, 0), (2, 0), (0, -1), (1, -1), (2, -1)],
];
/// The seven triangles of a cell, as vertex numbers: the top, then the walls
/// (`Canvas3D_BuildLatticeMesh`).
const TRIANGLES: [[usize; 3]; 7] = [
    [0, 1, 2],
    [0, 2, 3],
    [3, 2, 5],
    [0, 1, 4],
    [4, 3, 0],
    [1, 2, 4],
    [2, 5, 4],
];
/// The atlas texel a painted cell shows, `[r, g, b, a]` in 0 to 255.
const CLEAR: [f32; 4] = [255.0, 255.0, 255.0, 0.0];
/// The camera's tilt about `x`: a point is nearer the viewer by `y sin + z cos`.
const TILT_X: f32 = 0.45;
/// The colours `Canvas3D` paints a tile with, by what the event is
/// (`g_CanvasStateColours`, read live). A passed event is green, a passed elite
/// one yellow, an open one the grey `909090`.
pub mod colours {
    /// `0x817327cc`, an open event.
    pub const OPEN: [f32; 3] = [
        0x90 as f32 / 255.0,
        0x90 as f32 / 255.0,
        0x90 as f32 / 255.0,
    ];
    /// `0x817327d0`, an event passed.
    pub const PASSED: [f32; 3] = [0x20 as f32 / 255.0, 0xd0 as f32 / 255.0, 0.0];
    /// `0x817327d4`, an event passed at elite.
    pub const ELITE: [f32; 3] = [0xd0 as f32 / 255.0, 0xd0 as f32 / 255.0, 0.0];
    /// `0x817327c4`, the colour of the first state. **Chosen for a locked
    /// event**: the original paints none.
    pub const LOCKED: [f32; 3] = [
        0xe0 as f32 / 255.0,
        0xe0 as f32 / 255.0,
        0xe9 as f32 / 255.0,
    ];
}

/// Where the baked picture sits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// The picture's size in pixels.
    pub size: (u32, u32),
    /// Where the tile's top-face centre ([`tile_top`]) is in the picture, in
    /// whole pixels.
    pub anchor: (f32, f32),
}

/// A vertex of the mesh after the vertex program.
#[derive(Clone, Copy)]
struct Vertex {
    /// Picture pixels.
    at: [f32; 2],
    /// How near the viewer: larger wins the z-buffer.
    near: f32,
    /// `1 - s`.
    shade: f32,
}

fn height_of(col: i32, row: i32) -> f32 {
    let inside = (CELL.0..CELL.0 + FOOTPRINT.0).contains(&col)
        && (CELL.1..CELL.1 + FOOTPRINT.1).contains(&row);
    if inside { TILE_HEIGHT } else { 0.0 }
}

fn cells() -> impl Iterator<Item = (i32, i32)> {
    let (left, up, right, down) = MARGIN;
    (CELL.1 - up..CELL.1 + FOOTPRINT.1 + down).flat_map(move |row| {
        (CELL.0 - left..CELL.0 + FOOTPRINT.0 + right).map(move |col| (col, row))
    })
}

/// The projection of a lattice point at the reset camera, before the picture's
/// own origin.
fn project(world: [f32; 3]) -> (f32, f32) {
    Camera::scrolled((0.0, 0.0)).project(world)
}

/// The whole-pixel shift that puts the tile's anchor on a pixel, and the
/// picture's origin: its top left corner in the shifted screen.
fn frame() -> ((f32, f32), (f32, f32), (u32, u32)) {
    let anchor = project(tile_top((CELL.0 as f32, CELL.1 as f32)));
    let shift = (anchor.0.round() - anchor.0, anchor.1.round() - anchor.1);
    let (mut low, mut high) = (
        (f32::INFINITY, f32::INFINITY),
        (f32::NEG_INFINITY, f32::NEG_INFINITY),
    );
    for (col, row) in cells() {
        let up = (col + row).rem_euclid(2) == 0;
        let corners = if up { &CORNERS_UP } else { &CORNERS_DOWN };
        for corner in corners {
            for z in [-1.0, TILE_HEIGHT] {
                let at = project([
                    col as f32 * CELL_DX + corner[0],
                    row as f32 * CELL_DY + corner[1],
                    z,
                ]);
                low = (low.0.min(at.0), low.1.min(at.1));
                high = (high.0.max(at.0), high.1.max(at.1));
            }
        }
    }
    let origin = (
        (low.0 + shift.0).floor() - 1.0,
        (low.1 + shift.1).floor() - 1.0,
    );
    let size = (
        ((high.0 + shift.0).ceil() + 1.0 - origin.0) as u32,
        ((high.1 + shift.1).ceil() + 1.0 - origin.1) as u32,
    );
    (shift, origin, size)
}

/// Where the picture is, relative to the tile's anchor.
#[must_use]
pub fn layout() -> Layout {
    static LAYOUT: std::sync::OnceLock<Layout> = std::sync::OnceLock::new();
    *LAYOUT.get_or_init(|| {
        let (shift, origin, size) = frame();
        let anchor = project(tile_top((CELL.0 as f32, CELL.1 as f32)));
        Layout {
            size,
            anchor: (anchor.0 + shift.0 - origin.0, anchor.1 + shift.1 - origin.1),
        }
    })
}

/// The sprite tint of a tile of cell colour `colour`: its square root, which is
/// what the composite's encode makes of a product with the baked white.
#[must_use]
pub fn tint(colour: [f32; 3]) -> [f32; 4] {
    [colour[0].sqrt(), colour[1].sqrt(), colour[2].sqrt(), 1.0]
}

/// The vertex program on one cell: its six vertices.
fn vertices(col: i32, row: i32, shift: (f32, f32), origin: (f32, f32)) -> [Vertex; 6] {
    let up = (col + row).rem_euclid(2) == 0;
    let corners = if up { &CORNERS_UP } else { &CORNERS_DOWN };
    let own = height_of(col, row);
    let (sin_x, cos_x) = oag_core::math::sin_cos(TILT_X);
    std::array::from_fn(|at| {
        let corner = at % 3;
        let z = if at < 3 { 0.0 } else { -1.0 };
        let h = z + own;
        let world = [
            col as f32 * CELL_DX + corners[corner][0],
            row as f32 * CELL_DY + corners[corner][1],
        ];
        let index = corner + if up { 0 } else { 3 };
        let occlusion: f32 = NEIGHBOURS[index]
            .iter()
            .map(|&(du, dv)| (height_of(col + du, row + dv) - h).max(0.0))
            .sum::<f32>()
            / 6.0;
        let on_screen = project([world[0], world[1], h]);
        Vertex {
            at: [
                on_screen.0 + shift.0 - origin.0,
                on_screen.1 + shift.1 - origin.1,
            ],
            near: world[1] * sin_x + h * cos_x,
            shade: (1.0 - occlusion).clamp(0.0, 1.0),
        }
    })
}

/// One sample of the z-buffer: the triangle in front and where in it.
#[derive(Clone, Copy)]
struct Sample {
    near: f32,
    cell: (i32, i32),
    shade: f32,
}

/// Runs the canvas's programs over one tile and the ground around it and
/// returns the picture: RGBA, straight alpha, at [`layout`]'s size.
#[must_use]
pub fn bake() -> Vec<u8> {
    let (shift, origin, size) = frame();
    let (w, h) = (size.0 as usize, size.1 as usize);
    let mut buffer = vec![
        Sample {
            near: f32::NEG_INFINITY,
            cell: (i32::MIN, 0),
            shade: 1.0,
        };
        w * h * SUPER * SUPER
    ];
    let stride = w * SUPER;
    let scale = SUPER as f32;
    for (col, row) in cells() {
        let verts = vertices(col, row, shift, origin);
        // A ground cell's walls hang below the ground its neighbours make: the
        // bake's window ends, the canvas's mesh does not, so they are left out.
        let painted = height_of(col, row) > 0.0;
        let drawn = if painted { TRIANGLES.len() } else { 1 };
        for tri in &TRIANGLES[..drawn] {
            let [a, b, c] = tri.map(|at| verts[at]);
            let det = (b.at[0] - a.at[0]) * (c.at[1] - a.at[1])
                - (c.at[0] - a.at[0]) * (b.at[1] - a.at[1]);
            if det.abs() < 1e-6 {
                continue;
            }
            let min = |f: fn(&Vertex) -> f32| f(&a).min(f(&b)).min(f(&c));
            let max = |f: fn(&Vertex) -> f32| f(&a).max(f(&b)).max(f(&c));
            let x0 = ((min(|v| v.at[0]) * scale).floor().max(0.0)) as usize;
            let x1 = ((max(|v| v.at[0]) * scale).ceil() as usize).min(stride);
            let y0 = ((min(|v| v.at[1]) * scale).floor().max(0.0)) as usize;
            let y1 = ((max(|v| v.at[1]) * scale).ceil() as usize).min(h * SUPER);
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let p = [(sx as f32 + 0.5) / scale, (sy as f32 + 0.5) / scale];
                    let wb = ((p[0] - a.at[0]) * (c.at[1] - a.at[1])
                        - (c.at[0] - a.at[0]) * (p[1] - a.at[1]))
                        / det;
                    let wc = ((b.at[0] - a.at[0]) * (p[1] - a.at[1])
                        - (p[0] - a.at[0]) * (b.at[1] - a.at[1]))
                        / det;
                    let wa = 1.0 - wb - wc;
                    if wa < -1e-5 || wb < -1e-5 || wc < -1e-5 {
                        continue;
                    }
                    let near = wa * a.near + wb * b.near + wc * c.near;
                    let slot = &mut buffer[sy * stride + sx];
                    if near < slot.near - 1e-5 {
                        continue;
                    }
                    *slot = Sample {
                        near,
                        cell: (col, row),
                        shade: wa * a.shade + wb * b.shade + wc * c.shade,
                    };
                }
            }
        }
    }
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let (mut colour, mut alpha) = ([0.0_f32; 3], 0.0_f32);
            for sy in 0..SUPER {
                for sx in 0..SUPER {
                    let s = buffer[(y * SUPER + sy) * stride + x * SUPER + sx];
                    if s.cell.0 == i32::MIN {
                        continue;
                    }
                    if height_of(s.cell.0, s.cell.1) > 0.0 {
                        let t = CLEAR;
                        let a = t[3] / 255.0;
                        let lit = (1.0 - a) * (s.shade + 0.005).min(1.0) + a;
                        for (sum, texel) in colour.iter_mut().zip(&t) {
                            *sum += f32::from(composite_encode(texel * lit));
                        }
                        alpha += 1.0;
                    } else {
                        alpha += 1.0 - s.shade.max(0.0).sqrt();
                    }
                }
            }
            let samples = (SUPER * SUPER) as f32;
            let covered = alpha.max(1e-6);
            let painted = colour.iter().any(|&c| c > 0.0);
            for channel in colour {
                let value = if painted { channel / covered } else { 0.0 };
                out.push(value.round().clamp(0.0, 255.0) as u8);
            }
            out.push((alpha / samples * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests;
