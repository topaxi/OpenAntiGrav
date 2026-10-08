//! `Team Selection`'s `NAVIGATE TEAM` honeycomb - the `HexSelection` widget.
//!
//! # What the disc authors, and what the widget class keeps
//!
//! `Team_Selection_Definition.xml` authors the grid's shape and colours:
//! `columns="5" rows="7"`, `HexCol` / `HexColFade` for a cell and its faded
//! form, `SelectedColumnCol` for the selected team's column, and
//! `SelectedLockCol` / `SelectedLockColFade` for a padlock. The art is the
//! front end's own: `Hexagon_HD.gtf` for a cell, `Hexagon_HD_OUTLINE.gtf`
//! for the cursor, `Padlock.gtf` (the `Padlock` image this screen authors,
//! at 512x512) and each model's `<modellocation>\FE\thumb<LiveryNumber>.gtf`.
//! The widget class itself (cell pitch, cell size, how the colours combine)
//! is not read.
//!
//! # The columns are teams, the rows are models
//!
//! Measured on RPCS3 (2026-09-29): `Right` from `concept1` lands on the
//! next team's `concept1`, so the selected team is the centre column and
//! its neighbours are the teams either side, each column holding the seven
//! `PI_TeamModel` rows in file order. Neighbours wrap round the team list;
//! that the original wraps is **chosen, not measured**.
//!
//! # What is measured, and from what
//!
//! One settled RPCS3 frame (`BCES-00664`, 2000x1200, Feisar selected,
//! fresh profile), fitted to the authored brackets (title rule `160..1757`
//! authored against `306..1811` px gives 0.9424 px per unit): the first
//! column's centre lands on the authored origin `x = 272`; the column pitch
//! is 72.2 units, the row pitch 82.8, odd columns sit half a row lower; the
//! hexagon draws at 1.22 times its 72x62 art. The padlock's visible art
//! (91x129 texels) measures 40x56 units, 0.44 per texel; the thumbnail's
//! 1.0 per texel. A padlock reads `128` on the selected column (exactly
//! `SelectedLockCol`), `110` one column out and `68` two out, which is
//! `SelectedLockColFade` blended in by `0.2` and `0.65`. One frame, so
//! confidence 70, not higher. The selected column's red is its authored
//! colour drawn twice on the bare cell (no `HexCol` under it), the
//! two-pass fill `Block_Render` uses: `0x64` twice over black is `160`, the
//! frame reads `151`.
//!
//! **Chosen, not measured**: the cursor ring's thickness (the frame's ring
//! is about twice as thick as the outline art draws; its tint is read
//! off the frame, `185/35/55`), the ghost of an open
//! row's thumbnail in the neighbouring columns (the frame shows faint ship
//! silhouettes there; their tint is ours), and the pointer targets.

use oag_ui::frontend::{Draw, Placed};
use oag_ui::screen::argb_to_rgba;

use super::super::Picker;

/// Distance between column centres, and between row centres, in units.
const PITCH: [f32; 2] = [72.2, 82.8];
/// A cell's art against the authored 72x62 hexagon.
const HEX_SCALE: f32 = 1.22;
/// The hexagon's visible centre inside its 128x64 texture, in texels.
const HEX_CENTRE: [f32; 2] = [37.0, 31.5];
/// The visible hexagon's size in units: 72x62 at [`HEX_SCALE`].
const HEX_SIZE: [f32; 2] = [72.0 * HEX_SCALE, 62.0 * HEX_SCALE];
/// Units per padlock texel, and per thumbnail texel.
const LOCK_SCALE: f32 = 0.44;
const THUMB_SCALE: f32 = 1.0;
/// How far a padlock's colour has faded toward `SelectedLockColFade` one
/// and two columns from the selected one.
const LOCK_FADE: [f32; 3] = [0.0, 0.2, 0.65];

pub const HEX_SRC: &str = r"Data\FE\Images\Hexagon_HD.gtf";
pub const OUTLINE_SRC: &str = r"Data\FE\Images\Hexagon_HD_OUTLINE.gtf";
pub const LOCK_SRC: &str = r"Data\FE\Images\Padlock.gtf";

/// The sheet name of a model's own thumbnail.
#[must_use]
pub fn thumb_src(model_location: &str, livery_number: u32) -> String {
    format!(r"{model_location}\FE\thumb{livery_number}.gtf")
}

/// Whether `name` is one of [`thumb_src`]'s. `DATA02` and `DATA06` both ship
/// the classic hulls' `thumb0`..`thumb3`: `DATA02`'s is a 256x64 top-down
/// plan, `DATA06`'s the 126x64 three-quarter view the original's frame
/// draws, so a loader reads the **last** archive's copy of these (this
/// build's mount order reaches `DATA02` first, for every other name).
#[must_use]
pub fn is_thumb(name: &str) -> bool {
    name.to_ascii_lowercase().contains(r"\fe\thumb")
}

/// One model row of one team, as the grid draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCell {
    /// The sheet name of the model's thumbnail, when it has one.
    pub thumb: Option<String>,
    /// Whether the model can be picked now - an open cell shows its
    /// thumbnail, a closed one a padlock.
    pub open: bool,
    /// The index among the team's liveries the model is, when `open`.
    pub variant: Option<usize>,
}

/// The `<HexSelection>` widget: its place, shape and authored colours.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HexGrid {
    pub origin: [f32; 2],
    pub columns: usize,
    pub rows: usize,
    pub hex: u32,
    pub hex_fade: u32,
    /// Columns out at which a cell has faded all the way to `hex_fade`; `0`
    /// for a grid whose cells do not fade with distance (`Team Selection`'s,
    /// as measured).
    pub fade_columns: usize,
    pub selected_column: u32,
    pub selected_lock: u32,
    pub selected_lock_fade: u32,
}

/// One cell: which team column and model row it is, where it sits, and what
/// a click on it does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// Team index into the picker's entries.
    pub entry: usize,
    pub row: usize,
    /// Visible hexagon, `[x, y, width, height]`.
    pub rect: [f32; 4],
    pub distance: usize,
}

/// The fill and lock colours `Team Selection` authors, for a grid whose own
/// XML authors none (`TrackHexSelection`): **chosen, not measured** as the
/// same values, since the frame's red and dim grey read alike on both. The
/// fade with distance is measured: a settled frame's hexagon fill reads 41,
/// 35, 30 and 24 one to four columns out, which is `HEX_COL` over black
/// (50) easing to `HEX_COL_FADE` (25) in four steps; one frame, confidence 60.
const HEX_COL: u32 = 0x6480_8080;
const HEX_COL_FADE: u32 = 0x3280_8080;
const SELECTED_COL: u32 = 0x64ff_0000;
const LOCK_COL: u32 = 0xff80_8080;
const LOCK_COL_FADE: u32 = 0xff24_2424;

impl HexGrid {
    /// A grid of `columns` by `rows` at `origin` in the colours above.
    #[must_use]
    pub fn unauthored(origin: [f32; 2], columns: usize, rows: usize) -> Self {
        Self {
            origin,
            columns,
            rows,
            hex: HEX_COL,
            hex_fade: HEX_COL_FADE,
            fade_columns: 4,
            selected_column: SELECTED_COL,
            selected_lock: LOCK_COL,
            selected_lock_fade: LOCK_COL_FADE,
        }
    }

    /// Every cell, left column first, for a picker of `count` teams with
    /// `selected` the centre column's.
    #[must_use]
    pub fn cells(&self, selected: usize, count: usize) -> Vec<Cell> {
        let mut out = Vec::new();
        if count == 0 || self.columns == 0 {
            return out;
        }
        let centre = self.columns / 2;
        for column in 0..self.columns {
            let offset = column as i64 - centre as i64;
            let entry = (selected as i64 + offset).rem_euclid(count as i64) as usize;
            let shift = if column % 2 == 1 { PITCH[1] * 0.5 } else { 0.0 };
            for row in 0..self.rows {
                let (cx, cy) = (
                    self.origin[0] + column as f32 * PITCH[0],
                    self.origin[1] + row as f32 * PITCH[1] + shift,
                );
                out.push(Cell {
                    entry,
                    row,
                    rect: [
                        cx - HEX_SIZE[0] * 0.5,
                        cy - HEX_SIZE[1] * 0.5,
                        HEX_SIZE[0],
                        HEX_SIZE[1],
                    ],
                    distance: offset.unsigned_abs() as usize,
                });
            }
        }
        out
    }
}

fn centre(rect: [f32; 4]) -> (f32, f32) {
    (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5)
}

fn models_of(picker: &Picker, entry: usize) -> &[ModelCell] {
    match picker.entries().get(entry).map(|entry| &entry.details) {
        Some(super::super::Details::Ship { models, .. }) => models,
        _ => &[],
    }
}

/// The livery a click on `cell` picks, when its model is open.
#[must_use]
pub fn variant_at(picker: &Picker, cell: &Cell) -> Option<usize> {
    models_of(picker, cell.entry)
        .get(cell.row)
        .filter(|model| model.open)
        .and_then(|model| model.variant)
}

fn lerp_colour(from: u32, to: u32, t: f32) -> [f32; 4] {
    let (a, b) = (argb_to_rgba(from), argb_to_rgba(to));
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

fn sprite(placed: Placed, rect: [f32; 4], color: [f32; 4]) -> Draw {
    Draw::Sprite {
        rect,
        uv: [
            placed.x as f32,
            placed.y as f32,
            placed.width as f32,
            placed.height as f32,
        ],
        color,
    }
}

/// What a cell shows on its hexagon.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Face {
    /// Nothing but the hexagon.
    Empty,
    /// The padlock of a cell that cannot be picked.
    Lock,
    /// A picture, by sheet name, centred, `size` units across.
    Art { src: String, size: ArtSize },
}

/// How big a cell's picture is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ArtSize {
    /// This many units per texel.
    PerTexel(f32),
    /// Scaled so the longer side is this many units.
    Fit(f32),
}

/// The cursor ring's tint on `Team Selection`, read off its frame.
pub(super) const TEAM_RING: [f32; 4] = [0.73, 0.14, 0.22, 1.0];

/// The grid's draws, behind-to-front per cell: the hexagon, then its face,
/// then the cursor ring on the cell at `cursor` of the centre column.
pub(super) fn draw_cells(
    grid: &HexGrid,
    (selected, count): (usize, usize),
    cursor: Option<usize>,
    ring: [f32; 4],
    face: &dyn Fn(&Cell) -> Face,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    out: &mut Vec<Draw>,
) {
    for cell in grid.cells(selected, count) {
        let chosen = cell.distance == 0;
        let (cx, cy) = centre(cell.rect);
        if let Some(placed) = sprites(HEX_SRC) {
            let rect = [
                cx - HEX_CENTRE[0] * HEX_SCALE,
                cy - HEX_CENTRE[1] * HEX_SCALE,
                placed.width as f32 * HEX_SCALE,
                placed.height as f32 * HEX_SCALE,
            ];
            if chosen {
                let red = argb_to_rgba(grid.selected_column);
                out.push(sprite(placed, rect, red));
                out.push(sprite(placed, rect, red));
            } else {
                let fade = if grid.fade_columns == 0 {
                    0.0
                } else {
                    (cell.distance as f32 / grid.fade_columns as f32).min(1.0)
                };
                out.push(sprite(
                    placed,
                    rect,
                    lerp_colour(grid.hex, grid.hex_fade, fade),
                ));
            }
        }
        match face(&cell) {
            Face::Empty => {}
            Face::Art { src, size } => {
                if let Some(placed) = sprites(&src) {
                    let scale = match size {
                        ArtSize::PerTexel(scale) => scale,
                        ArtSize::Fit(units) => units / placed.width.max(placed.height) as f32,
                    };
                    let (w, h) = (placed.width as f32 * scale, placed.height as f32 * scale);
                    let color = if chosen {
                        [1.0; 4]
                    } else {
                        [0.0, 0.0, 0.0, 0.35]
                    };
                    out.push(sprite(placed, [cx - w * 0.5, cy - h * 0.5, w, h], color));
                }
            }
            Face::Lock => {
                if let Some(placed) = sprites(LOCK_SRC) {
                    let (w, h) = (
                        placed.width as f32 * LOCK_SCALE,
                        placed.height as f32 * LOCK_SCALE,
                    );
                    let fade = LOCK_FADE[cell.distance.min(LOCK_FADE.len() - 1)];
                    out.push(sprite(
                        placed,
                        [cx - w * 0.5, cy - h * 0.5, w, h],
                        lerp_colour(grid.selected_lock, grid.selected_lock_fade, fade),
                    ));
                }
            }
        }
        if chosen
            && Some(cell.row) == cursor
            && let Some(placed) = sprites(OUTLINE_SRC)
        {
            let rect = [
                cx - HEX_CENTRE[0] * HEX_SCALE,
                cy - HEX_CENTRE[1] * HEX_SCALE,
                placed.width as f32 * HEX_SCALE,
                placed.height as f32 * HEX_SCALE,
            ];
            out.push(sprite(placed, rect, ring));
        }
    }
}

/// `Team Selection`'s honeycomb: a team per column, a model per row.
pub(super) fn draw(
    grid: &HexGrid,
    picker: &Picker,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    out: &mut Vec<Draw>,
) {
    let cursor = models_of(picker, picker.index())
        .iter()
        .position(|cell| cell.variant == Some(picker.variant_index()));
    let face = |cell: &Cell| match models_of(picker, cell.entry).get(cell.row) {
        Some(model) if model.open => model.thumb.clone().map_or(Face::Empty, |src| Face::Art {
            src,
            size: ArtSize::PerTexel(THUMB_SCALE),
        }),
        _ => Face::Lock,
    };
    draw_cells(
        grid,
        (picker.index(), picker.entries().len()),
        cursor,
        TEAM_RING,
        &face,
        sprites,
        out,
    );
}
