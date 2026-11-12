//! The cells of 2048's `Canvas3D` lattice that a campaign button paints without
//! raising them: the ring of striped and dotted cells around a tile, and the
//! flat tint of a tile that is not reached yet.
//!
//! A button's slot (`CanvasButton_PaintIconAndRing` `0x8106fbd8`) writes its
//! ring mode into 18 cells around its footprint (`0x81073aee`): a half-float
//! `(u, v)` shift into `canvasTex2`, which moves the cell's atlas triangle to
//! another outlined triangle of `canvasTexture.gxt`. A cell is written only
//! when its priority (`canvas + 0x2f0c4`) is not above the mode's, and the write
//! raises it to the mode, so the highest fill wins and the paint order does not
//! matter. A tile's own cells hold priority 6 (state 1, flat) or 7 (state 2,
//! raised), above every ring.
//! The shifts are the static initialiser's `0x81072a32` stores, read as the
//! floats `x = (94, 0)/2048` and so on (`docs/ghidra/functions/vita-2048-eu-v104/campaign-map-projection.md`,
//! "The ring").
//!
//! The camera is fixed, so a cell's picture only moves with the pan: [`bake`]
//! renders one cell of each parity per fill from the atlas, exactly as the
//! lattice picture does ([`super::canvas_lattice`]), and the map draws it at
//! each cell's projected place.

use super::canvas_camera::Camera;
use super::canvas_lattice::{
    BAKED_VIEW, CELL_DX, CELL_DY, VIEWPORT, bilinear, cell_at, composite_encode, world_at,
};

/// The atlas shift of each fill, in atlas pixels (`x`, `y`): fill 0 is the cell's
/// own triangle. Fills 1 to 4 are the ring modes' `DAT_817326a4`, `DAT_817326ac`,
/// `DAT_81732694`, `DAT_8173269c`: light dots, dark dots, light stripes, dark
/// stripes.
pub const SHIFTS: [[f32; 2]; 5] = [
    [0.0, 0.0],
    [188.0, 0.0],
    [141.0, 82.0],
    [94.0, 0.0],
    [47.0, 82.0],
];

/// The sheet key of each fill's picture, apex-up cell then apex-down. Not paths:
/// nothing on a disc has them.
const NAMES: [[&str; 2]; 5] = [
    ["oag:2048-canvas-cell-0-up", "oag:2048-canvas-cell-0-down"],
    ["oag:2048-canvas-cell-1-up", "oag:2048-canvas-cell-1-down"],
    ["oag:2048-canvas-cell-2-up", "oag:2048-canvas-cell-2-down"],
    ["oag:2048-canvas-cell-3-up", "oag:2048-canvas-cell-3-down"],
    ["oag:2048-canvas-cell-4-up", "oag:2048-canvas-cell-4-down"],
];

/// The 18 cells of a button's ring, as `(col, row)` from its first cell
/// (`0x8106fbd8`): two columns left and two right on each of the footprint's
/// two rows, and five columns on the row above and the row below.
pub const RING: [(i32, i32); 18] = [
    (-2, 0),
    (-1, 0),
    (3, 0),
    (4, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
    (2, -1),
    (3, -1),
    (-2, 1),
    (-1, 1),
    (3, 1),
    (4, 1),
    (-1, 2),
    (0, 2),
    (1, 2),
    (2, 2),
    (3, 2),
];
/// The cells of a tile: three columns by two rows from its first cell.
pub const FOOTPRINT: (i32, i32) = (3, 2);

/// A tile's cell priority: state 1 (flat) and state 2 (raised)
/// (`CanvasItem_Paint` `0x8106fae2`).
const FLAT_PRIORITY: u8 = 6;
const RAISED_PRIORITY: u8 = 7;

/// What a button paints on its own footprint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tile {
    /// State 1: the cells take the button's colour and stay at the ground.
    Flat([f32; 3]),
    /// State 2: the cells are raised ([`super::canvas_prism`] draws them).
    Raised,
}

/// One button: where its footprint starts, what it paints on it, and its ring
/// mode (a fill from 1 to 4, 0 for none).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasButton {
    pub cell: (i32, i32),
    pub tile: Tile,
    pub ring: u8,
}

/// What a resolved ground cell shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Painted {
    pub cell: (i32, i32),
    pub fill: u8,
    /// The colour a flat tile gives the cell, white for a ring cell.
    pub colour: [f32; 3],
}

#[derive(Clone, Copy)]
struct Slot {
    priority: u8,
    fill: u8,
    colour: [f32; 3],
    raised: bool,
}

/// The priority each button's writes leave on every cell, raised tiles included.
fn slots(buttons: &[CanvasButton]) -> std::collections::BTreeMap<(i32, i32), Slot> {
    use std::collections::BTreeMap;
    let mut cells: BTreeMap<(i32, i32), Slot> = BTreeMap::new();
    let mut write = |cell: (i32, i32), slot: Slot| {
        let held = cells.entry(cell).or_insert(Slot {
            priority: 0,
            fill: 0,
            colour: [1.0; 3],
            raised: false,
        });
        if held.priority <= slot.priority {
            *held = slot;
        }
    };
    for button in buttons {
        let (col, row) = button.cell;
        if button.ring > 0 {
            for (dc, dr) in RING {
                write(
                    (col + dc, row + dr),
                    Slot {
                        priority: button.ring,
                        fill: button.ring,
                        colour: [1.0; 3],
                        raised: false,
                    },
                );
            }
        }
        let (priority, colour, raised) = match button.tile {
            Tile::Flat(colour) => (FLAT_PRIORITY, colour, false),
            Tile::Raised => (RAISED_PRIORITY, [1.0; 3], true),
        };
        for dc in 0..FOOTPRINT.0 {
            for dr in 0..FOOTPRINT.1 {
                write(
                    (col + dc, row + dr),
                    Slot {
                        priority,
                        fill: 0,
                        colour,
                        raised,
                    },
                );
            }
        }
    }
    cells
}

/// The cells the canvas ends up painting at the ground, from every button:
/// the ring cells and the flat tiles, each by the priority rule above. A raised
/// tile's cells are left out (the prism picture draws them) but still hold
/// their priority, so no ring is drawn there.
#[must_use]
pub fn resolve(buttons: &[CanvasButton]) -> Vec<Painted> {
    slots(buttons)
        .into_iter()
        .filter(|(_, slot)| slot.priority > 0 && !slot.raised)
        .map(|(cell, slot)| Painted {
            cell,
            fill: slot.fill,
            colour: slot.colour,
        })
        .collect()
}

/// The canvas priority of every cell a button wrote, raised tiles included
/// (`canvas + 0x2f0c4`); a cell not listed is plain ground, 0.
#[must_use]
pub fn priorities(buttons: &[CanvasButton]) -> std::collections::BTreeMap<(i32, i32), u8> {
    slots(buttons)
        .into_iter()
        .map(|(cell, slot)| (cell, slot.priority))
        .collect()
}

/// Whether a cell is apex up: `col + row` even.
#[must_use]
pub fn is_up(cell: (i32, i32)) -> bool {
    (cell.0 + cell.1).rem_euclid(2) == 0
}

/// The sheet key of a fill's picture for a cell of this parity.
#[must_use]
pub fn name(fill: u8, up: bool) -> &'static str {
    NAMES[usize::from(fill).min(NAMES.len() - 1)][usize::from(!up)]
}

/// Where a cell's picture sits: its size in pixels and where the cell's
/// anchor (its centre on the lattice plane) is in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub size: (u32, u32),
    pub anchor: (f32, f32),
}

/// The pixels of the baked lattice picture a reference cell covers.
struct Reference {
    cell: (i32, i32),
    /// The picture's top left pixel in the lattice picture, and its size.
    origin: (i32, i32),
    layout: Layout,
}

/// The reference cell of one parity: found near the middle of the baked
/// lattice picture.
fn reference(cell: (i32, i32)) -> Reference {
    let camera = Camera::scrolled(BAKED_VIEW);
    let at = camera.project([cell.0 as f32 * CELL_DX, cell.1 as f32 * CELL_DY, 0.0]);
    let (mut low, mut high) = ((i32::MAX, i32::MAX), (i32::MIN, i32::MIN));
    let (x0, y0) = (at.0 as i32 - 48, at.1 as i32 - 40);
    for py in y0..y0 + 80 {
        for px in x0..x0 + 96 {
            let inside = u32::try_from(px).is_ok_and(|px| px < VIEWPORT[0])
                && u32::try_from(py).is_ok_and(|py| py < VIEWPORT[1]);
            if inside
                && cell_at(world_at(px as u32, py as u32)).is_some_and(|(found, _)| found == cell)
            {
                low = (low.0.min(px), low.1.min(py));
                high = (high.0.max(px), high.1.max(py));
            }
        }
    }
    // One clear pixel around the cell, so a filtered draw never reads a
    // neighbour in the sheet.
    let origin = (low.0 - 1, low.1 - 1);
    let size = (
        (high.0 + 2 - origin.0) as u32,
        (high.1 + 2 - origin.1) as u32,
    );
    Reference {
        cell,
        origin,
        layout: Layout {
            size,
            anchor: (at.0 - origin.0 as f32, at.1 - origin.1 as f32),
        },
    }
}

/// The two reference cells, apex up then apex down: the centre cell of the baked
/// view, and its right neighbour.
fn references() -> &'static [Reference; 2] {
    static REFERENCES: std::sync::OnceLock<[Reference; 2]> = std::sync::OnceLock::new();
    REFERENCES.get_or_init(|| {
        let centre = world_at(VIEWPORT[0] / 2, VIEWPORT[1] / 2);
        let col = (centre[0] / CELL_DX).round() as i32;
        let row = (centre[1] / CELL_DY).round() as i32;
        let up = if is_up((col, row)) {
            (col, row)
        } else {
            (col + 1, row)
        };
        [reference(up), reference((up.0 + 1, up.1))]
    })
}

/// Where the picture of a cell of this parity is.
#[must_use]
pub fn layout(up: bool) -> Layout {
    references()[usize::from(!up)].layout
}

/// The picture of a cell of this parity, drawn so that its anchor lands on the
/// cell's projected centre: its top left corner in the camera's pixels.
#[must_use]
pub fn origin(camera: &Camera, cell: (i32, i32)) -> (f32, f32) {
    let at = camera.project([cell.0 as f32 * CELL_DX, cell.1 as f32 * CELL_DY, 0.0]);
    let anchor = layout(is_up(cell)).anchor;
    (at.0 - anchor.0, at.1 - anchor.1)
}

/// One baked picture: its sheet key, size and RGBA pixels, straight alpha.
#[derive(Debug)]
pub struct Picture {
    pub name: &'static str,
    pub size: (u32, u32),
    pub rgba: Vec<u8>,
}

/// The ten cell pictures (five fills, two parities), off the atlas's RGBA at mip
/// 0: the lattice picture's own pixels ([`super::canvas_lattice::bake`]) for one
/// cell, with the fill's shift added to the atlas position, and clear outside the
/// cell. `None` when the atlas is too small to hold every shifted triangle.
#[must_use]
pub fn bake(atlas: &[u8], width: u32, height: u32) -> Option<Vec<Picture>> {
    let reach = SHIFTS.iter().fold([0.0_f32; 2], |far, shift| {
        [far[0].max(shift[0] + 160.0), far[1].max(shift[1] + 128.0)]
    });
    if (width as f32) < reach[0]
        || (height as f32) < reach[1]
        || atlas.len() < (width * height * 4) as usize
    {
        return None;
    }
    let mut out = Vec::new();
    for (fill, shift) in SHIFTS.iter().enumerate() {
        for up in [true, false] {
            let reference = &references()[usize::from(!up)];
            let (w, h) = reference.layout.size;
            let mut rgba = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    let (px, py) = (reference.origin.0 + x, reference.origin.1 + y);
                    let texel = u32::try_from(px)
                        .ok()
                        .zip(u32::try_from(py).ok())
                        .filter(|&(px, py)| px < VIEWPORT[0] && py < VIEWPORT[1])
                        .and_then(|(px, py)| cell_at(world_at(px, py)))
                        .filter(|&(found, _)| found == reference.cell)
                        .map(|(_, at)| {
                            bilinear(atlas, width, height, [at[0] + shift[0], at[1] + shift[1]])
                        });
                    match texel {
                        Some(texel) => {
                            rgba.extend(texel[..3].iter().map(|&c| composite_encode(c)));
                            rgba.push(255);
                        }
                        None => rgba.extend([0, 0, 0, 0]),
                    }
                }
            }
            out.push(Picture {
                name: name(fill as u8, up),
                size: (w, h),
                rgba,
            });
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests;
