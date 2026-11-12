//! Raised ground cells of 2048's `Canvas3D` mesh as triangles: the lattice cells
//! the music wave lifts ([`super::canvas_wave`]).
//!
//! The canvas lifts each vertex by its cell's alpha and shades it by the cells
//! that share it; [`super::canvas_prism`] runs those two programs on a tile in
//! software and bakes a picture. A wave cell moves every frame, so here the
//! same programs run on the CPU and the result is drawn as [`Draw::Triangles`]
//! over the cell pictures [`super::canvas_cells`] bakes:
//!
//! - The camera is orthographic, so lifting a cell moves its top face by a
//!   screen vector and changes nothing else: the face is the picture the cell
//!   shows on the ground (the lattice's, or a ring cell's own), three corners
//!   of it at the lifted place.
//! - A cell also draws the walls the mesh builds under its edges, where the cell
//!   across the edge is lower, from the top down to that cell's top or one unit,
//!   with the top's atlas coordinates (a wall samples along the cell's edge).
//! - A vertex is shaded `1 - s`, `s = 1/6 * sum max(height_k - h, 0)` over the
//!   six cells that share it (`canvas_prism::NEIGHBOURS`); the baked picture is
//!   composite-encoded, so the colour is the square root of
//!   `min(1 - s + 0.005, 1)`, as the prism's does.
//! - The z-buffer is replaced by drawing far to near (rows ascending). Only the
//!   raised cells are drawn.
//!
//! Not read, so not drawn: a vertex's occlusion from a tile's cells uses the
//! tile's full height only (no other tile).

use std::collections::BTreeSet;

use super::canvas_camera::Camera;
use super::canvas_cells;
use super::canvas_lattice::{CELL_DX, CELL_DY, CORNERS_DOWN, CORNERS_UP};
use super::canvas_prism::NEIGHBOURS;
use super::draw::Corner;

/// How far a wall hangs below its top, in canvas units.
const SKIRT: f32 = 1.0;
/// How far inside the cell's edge a wall samples the picture, in pixels.
const INSET: f32 = 1.0;

/// Where a cell's picture is: its three corners on the ground, in viewport
/// pixels, and the sheet pixel each shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// The corners on the screen at height 0.
    pub at: [[f32; 2]; 3],
    /// The sheet pixels they sample.
    pub uv: [[f32; 2]; 3],
}

/// What the mesh needs of its surroundings.
pub struct Scene<'a> {
    /// The view.
    pub camera: &'a Camera,
    /// Viewport pixels to the draw list's space, per axis.
    pub scale: (f32, f32),
    /// A cell's height in canvas units: the wave's, or a tile's.
    pub height_of: &'a dyn Fn((i32, i32)) -> f32,
    /// Whether a cell is drawn by something else (a tile): never drawn here.
    pub skip: &'a dyn Fn((i32, i32)) -> bool,
    /// Where a cell's picture is, given its three corners on the ground in
    /// viewport pixels; `None` for a cell with no picture to show.
    pub place: &'a dyn Fn((i32, i32), [[f32; 2]; 3]) -> Option<Placed>,
}

impl std::fmt::Debug for Scene<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scene")
            .field("camera", self.camera)
            .finish_non_exhaustive()
    }
}

/// The triangles of every cell in `raised`, far to near.
#[must_use]
pub fn triangles(scene: &Scene, raised: &[(i32, i32)]) -> Vec<[Corner; 3]> {
    // Only the raised cells: a wall stops at the top of the lower cell it faces,
    // so it never covers that cell's own ground, which stays the ground picture
    // and is byte for byte what it was.
    let drawn: BTreeSet<(i32, i32)> = raised
        .iter()
        .filter(|&&cell| !(scene.skip)(cell))
        .map(|&(col, row)| (row, col))
        .collect();
    let mut out = Vec::new();
    for (row, col) in drawn {
        cell_triangles(scene, (col, row), &mut out);
    }
    out
}

fn cell_triangles(scene: &Scene, cell: (i32, i32), out: &mut Vec<[Corner; 3]>) {
    let up = canvas_cells::is_up(cell);
    let corners = if up { &CORNERS_UP } else { &CORNERS_DOWN };
    let camera = scene.camera;
    let centre = [cell.0 as f32 * CELL_DX, cell.1 as f32 * CELL_DY];
    let own = (scene.height_of)(cell);
    let ground: [[f32; 2]; 3] = std::array::from_fn(|at| {
        let on_screen =
            camera.project([centre[0] + corners[at][0], centre[1] + corners[at][1], 0.0]);
        [on_screen.0, on_screen.1]
    });
    let Some(placed) = (scene.place)(cell, ground) else {
        return;
    };
    let middle = [
        placed.uv.iter().map(|uv| uv[0]).sum::<f32>() / 3.0,
        placed.uv.iter().map(|uv| uv[1]).sum::<f32>() / 3.0,
    ];
    // The shade of a corner at the top and at the foot of its skirt, from the
    // cells that share it.
    let lit = |corner: usize, z: f32| -> f32 {
        let occlusion: f32 = NEIGHBOURS[corner + if up { 0 } else { 3 }]
            .iter()
            .map(|&(du, dv)| ((scene.height_of)((cell.0 + du, cell.1 + dv)) - z).max(0.0))
            .sum::<f32>()
            / 6.0;
        ((1.0 - occlusion).clamp(0.0, 1.0) + 0.005).min(1.0).sqrt()
    };
    // One vertex at height `z`, sampling the sheet at `sample` and shaded `lit`.
    let vertex = |corner: usize, z: f32, sample: [f32; 2], lit: f32| -> Corner {
        let world = [
            centre[0] + corners[corner][0],
            centre[1] + corners[corner][1],
        ];
        let lifted = camera.project([world[0], world[1], z]);
        let flat = camera.project([world[0], world[1], 0.0]);
        Corner {
            at: [
                (placed.at[corner][0] + lifted.0 - flat.0) * scene.scale.0,
                (placed.at[corner][1] + lifted.1 - flat.1) * scene.scale.1,
            ],
            uv: sample,
            color: [lit; 3],
        }
    };
    // A wall samples the picture along the cell's edge, a pixel inside it.
    let inside = |corner: usize| -> [f32; 2] {
        let uv = placed.uv[corner];
        let (dx, dy) = (middle[0] - uv[0], middle[1] - uv[1]);
        let length = (dx * dx + dy * dy).sqrt().max(1e-6);
        [uv[0] + dx / length * INSET, uv[1] + dy / length * INSET]
    };
    // The three walls, as the corners of the edge and the cell across it.
    let walls: [((usize, usize), (i32, i32)); 3] = if up {
        [((0, 2), (-1, 0)), ((0, 1), (0, 1)), ((1, 2), (1, 0))]
    } else {
        [((0, 2), (0, -1)), ((0, 1), (-1, 0)), ((1, 2), (1, 0))]
    };
    for ((a, b), (dc, dr)) in walls {
        // A wall shows only where the cell across the edge is lower, from the top
        // down to that cell's top or one unit, whichever is nearer: the rest is
        // behind a neighbour's top, which the canvas's depth buffer hides and
        // this far-to-near order does by drawing it later. The shade runs
        // linearly down the wall, as the original's does to the foot.
        let beside = (scene.height_of)((cell.0 + dc, cell.1 + dr));
        let drop = (own - beside).min(SKIRT);
        if drop <= 1e-4 {
            continue;
        }
        let bottom = own - drop;
        let share = drop / SKIRT;
        let shade = |corner: usize| {
            let top = lit(corner, own);
            top + (lit(corner, own - SKIRT) - top) * share
        };
        let (ta, tb) = (
            vertex(a, own, inside(a), lit(a, own)),
            vertex(b, own, inside(b), lit(b, own)),
        );
        let (ba, bb) = (
            vertex(a, bottom, inside(a), shade(a)),
            vertex(b, bottom, inside(b), shade(b)),
        );
        out.push([ta, tb, bb]);
        out.push([ta, bb, ba]);
    }
    out.push(std::array::from_fn(|corner| {
        vertex(corner, own, placed.uv[corner], lit(corner, own))
    }));
}

#[cfg(test)]
mod tests;
