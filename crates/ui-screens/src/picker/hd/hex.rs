//! `Team Selection`'s `NAVIGATE TEAM` honeycomb - the `HexSelection` widget.
//!
//! # What the disc authors, and what the widget class keeps
//!
//! `Team_Selection_Definition.xml` authors the grid's shape and colours:
//! `columns="5" rows="7"`, `HexCol` / `HexColFade` for a cell and its faded
//! form, and `SelectedLockCol` / `SelectedLockColFade` for a padlock. The art is the
//! widget class's own: `Hexagon_HD_Faded.gtf` for a cell (alpha 221 of 255),
//! `hexlock_HD.gtf` for a padlock and `Hexagon_HD_THICK_OUT.gtf` for the cursor; each
//! model's `<modellocation>\FE\thumb<LiveryNumber>.gtf` is the picture, under the
//! padlock when the model is locked (`docs/ghidra/functions/ps3-hdfury-eu/ship-select-ring.md`,
//! "The cells").
//! The widget class's own constants (pitch 72 by 83, half row 42, the 92x80 quad), the fill's
//! fade from column to column ([`edge_share`]), the selected column's `HD_Blue` fill
//! (`SelectedColumnCol` is not a key it reads), the ring's pulse and the slide on a step are read in
//! `docs/ghidra/functions/ps3-hdfury-eu/ship-select-ring.md`; [`slide`](super::slide) holds the
//! step's state.
//!
//! # The columns are teams, the rows are models
//!
//! Measured on RPCS3: `Right` from `concept1` lands on the
//! next team's `concept1`, so the selected team is the centre column and
//! its neighbours are the teams either side, each column holding the seven
//! `PI_TeamModel` rows in file order. Neighbours wrap round the team list;
//! that the original wraps is **chosen, not measured**. The columns of odd
//! teams sit half a row lower (the rest grabs of all twelve teams on two boots,
//! `docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`, "The banner and the hex column
//! on a step").
//!
//! # What is measured, and from what
//!
//! One settled RPCS3 frame (`BCES-00664`, 2000x1200, Feisar selected,
//! fresh profile): the hexagon draws at 1.22 times its 72x62 art and the thumbnail
//! at 1.0 per texel.
//!
//! **Chosen, not measured**: the pointer targets, and the pulse's zero point (the
//! widget's clock starts with the picker; when the original builds the widget relative to the page
//! entry is unread). **Not wired**: the ring's ease on a model-row change.
//!
//! # Panning under a finger
//!
//! **Chosen, not measured** (the original is pad-only): `cells`' `pan`
//! slides every column by a fraction of [`PITCH`]`[0]` while a touch drag
//! moves the grid, with the column entering at one edge and the one leaving
//! at the other faded by their `Cell::weight`. The pan is `oag_ui::kinetic`'s
//! (follow, flick, settle on a column); see `Picker::fold_pan`.

use oag_ui::frontend::{Draw, Placed};
use oag_ui::screen::argb_to_rgba;

use super::super::Picker;
use super::slide::Slide;

/// Distance between column centres, and between row centres, in units.
pub(in crate::picker) const PITCH: [f32; 2] = [72.0, 83.0];

/// How far a column of an odd team sits below an even one: half a row, as the widget's own
/// literal (`docs/ghidra/functions/ps3-hdfury-eu/ship-select-ring.md`, "Fields").
pub const HALF_ROW: f32 = 42.0;

/// The quad of a cell and of the cursor ring, in units, and the texel window of the 128x64 art
/// it shows, both centred on the cell.
const QUAD: [f32; 2] = [92.0, 80.0];
const QUAD_TEXELS: [f32; 2] = [74.0, 64.0];

/// The pulse's period in seconds: the phase advances 3 a second and wraps at 2.
pub const PULSE_PERIOD_SECONDS: f32 = 2.0 / 3.0;

/// The visible hexagon's size in units: the art's 72x62 texels on [`QUAD`].
const HEX_SIZE: [f32; 2] = [
    72.0 * QUAD[0] / QUAD_TEXELS[0],
    62.0 * QUAD[1] / QUAD_TEXELS[1],
];
/// Units per thumbnail texel.
const THUMB_SCALE: f32 = 1.0;
/// The texel window of `hexlock_HD.gtf`'s 128x64 art that a padlock shows on [`QUAD`].
const LOCK_QUAD_TEXELS: [f32; 2] = [75.0, 64.0];

pub const HEX_SRC: &str = r"Data\FE\Images\Hexagon_HD_Faded.gtf";
pub const THICK_OUT_SRC: &str = r"Data\FE\Images\Hexagon_HD_THICK_OUT.gtf";
pub const LOCK_SRC: &str = r"Data\FE\Images\hexlock_HD.gtf";

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
///
/// A cell outside the selected column is `hex_fade` shading to `hex` across its quad, by
/// [`edge_share`] of each side; its picture and padlock are shaded by the same two shares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HexGrid {
    pub origin: [f32; 2],
    pub columns: usize,
    pub rows: usize,
    /// The widget's own `Scale`, which shrinks the cell pitch and size alike: `1.0` where it
    /// authors none (HD's `Team Selection`), `0.8` on Omega's, whose grid holds nine rows
    /// where HD's holds seven.
    pub scale: f32,
    pub hex: u32,
    pub hex_fade: u32,
    pub selected_column: u32,
    pub selected_lock: u32,
    pub selected_lock_fade: u32,
    /// The colour the cursor ring's pulse passes through between white and black: `HD_Blue`, which
    /// the front end's style decides, or a title's own fixed one. `None` draws no ring.
    pub ring_peak: Option<u32>,
    /// Whether the half-row shift of a column follows its team's index (`Team Selection`: a
    /// column of an odd team sits half a row lower whichever window slot it is in) rather than
    /// its slot in the window (`TrackHexSelection`, unmeasured).
    pub phase_by_entry: bool,
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
    /// The column's signed offset from the selected one: negative to its left.
    pub offset: i64,
    /// How much of the cell shows: `1.0` for every column but the two that
    /// slide in and out of the grid's edge while it is panned.
    pub weight: f32,
}

/// The widget class's own defaults, for a grid whose XML authors no colour
/// (`TrackHexSelection`).
const HEX_COL: u32 = 0x6480_8080;
const HEX_COL_FADE: u32 = 0x3280_8080;
const SELECTED_COL: u32 = 0x64ff_0000;
const LOCK_COL: u32 = 0xff80_8080;
const LOCK_COL_FADE: u32 = 0xff16_1616;

/// A cell's picture where it is not the selected column's open one: black, at this alpha at the
/// selected column's edges (and on its locked cells) and at [`PICTURE_COL_FADE`]'s at the grid's
/// outer edges (`HexSelection_SetCell`).
const PICTURE_COL: u32 = 0x6400_0000;
const PICTURE_COL_FADE: u32 = 0x1200_0000;

impl HexGrid {
    /// A grid of `columns` by `rows` at `origin` in the colours above.
    #[must_use]
    pub fn unauthored(origin: [f32; 2], columns: usize, rows: usize) -> Self {
        Self {
            origin,
            columns,
            rows,
            scale: 1.0,
            hex: HEX_COL,
            hex_fade: HEX_COL_FADE,
            selected_column: SELECTED_COL,
            selected_lock: LOCK_COL,
            selected_lock_fade: LOCK_COL_FADE,
            ring_peak: None,
            phase_by_entry: false,
        }
    }

    /// Every cell, left column first, for a picker of `count` teams with
    /// `selected` the centre column's.
    ///
    /// `pan` is the grid's fractional column offset while a finger drags it
    /// (see [`super::super::Picker::pan`]): positive slides the columns
    /// right, which brings one more column in at the left edge and lets the
    /// right-most one out, each fading by how far it has travelled.
    #[must_use]
    pub fn cells(&self, selected: usize, count: usize, pan: f32) -> Vec<Cell> {
        self.cells_at(selected, count, pan, None)
    }

    /// [`Self::cells`] while `slide` runs: every column of the previous window moves a pitch toward
    /// the step's side, linearly, keeping its own half-row shift, and the spare column brings the
    /// incoming team in from the other.
    #[must_use]
    pub fn cells_at(
        &self,
        selected: usize,
        count: usize,
        pan: f32,
        slide: Option<&Slide>,
    ) -> Vec<Cell> {
        let mut out = Vec::new();
        if count == 0 || self.columns == 0 {
            return out;
        }
        let slide = slide.filter(|_| self.phase_by_entry);
        let moving = slide.filter(|slide| slide.moving() && pan == 0.0);
        let dir = moving.map_or(0, |slide| slide.direction());
        let along = moving.map_or(0.0, |slide| f32::from(slide.direction()) * slide.progress());
        let selected = slide.map_or(selected, |slide| {
            if pan == 0.0 {
                slide.centre()
            } else {
                slide.shown()
            }
        });
        let centre = self.columns / 2;
        let blend = pan.abs().min(1.0);
        let first = if pan > 0.0 || dir > 0 { -1 } else { 0 };
        let last = self.columns as i64 - 1 + i64::from(pan < 0.0 || dir < 0);
        let phase = match (self.phase_by_entry, slide) {
            (false, _) => 0,
            (true, None) => selected as i64,
            (true, Some(slide)) => i64::from(if pan == 0.0 {
                slide.parity()
            } else {
                slide.ring_target_low()
            }),
        };
        let heave = |column: i64| {
            if (column + phase).rem_euclid(2) == 1 {
                1.0
            } else {
                0.0
            }
        };
        for column in first..=last {
            let offset = column - centre as i64;
            let entry = (selected as i64 + offset).rem_euclid(count as i64) as usize;
            let toward = column + if pan < 0.0 { -1 } else { 1 };
            let shift =
                HALF_ROW * self.scale * (heave(column) * (1.0 - blend) + heave(toward) * blend);
            let weight = if moving.is_some() {
                1.0
            } else if column < 0 || column >= self.columns as i64 {
                blend
            } else if (pan > 0.0 && column == self.columns as i64 - 1) || (pan < 0.0 && column == 0)
            {
                1.0 - blend
            } else {
                1.0
            };
            for row in 0..self.rows {
                let (cx, cy) = (
                    self.origin[0] + (column as f32 + pan + along) * PITCH[0] * self.scale,
                    self.origin[1] + row as f32 * PITCH[1] * self.scale + shift,
                );
                out.push(Cell {
                    entry,
                    row,
                    rect: [
                        cx - HEX_SIZE[0] * self.scale * 0.5,
                        cy - HEX_SIZE[1] * self.scale * 0.5,
                        HEX_SIZE[0] * self.scale,
                        HEX_SIZE[1] * self.scale,
                    ],
                    distance: offset.unsigned_abs() as usize,
                    offset,
                    weight,
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

/// How much of `HexCol` (against `HexColFade`) a cell's vertex has, `from_middle` columns from the
/// middle of the selected column, in a grid whose selected column is `centre` columns in: whole at
/// that column's edges, none at the grid's outer edges and on the spare column of a slide
/// (`HexSelection_SetCell`, `docs/ghidra/functions/ps3-hdfury-eu/ship-select-ring.md`, "The cells").
#[must_use]
pub fn edge_share(centre: usize, from_middle: f32) -> f32 {
    (1.0 - (from_middle.abs() - 0.5) / centre.max(1) as f32).clamp(0.0, 1.0)
}

/// The colours a cell's images have at their left and right sides, from `faded` to `full` by
/// [`edge_share`] of the cell's own two sides: `middle` is where the cell's centre stands, in
/// columns right of the selected column's. All three images of a cell take the same two shares,
/// each across its own quad.
fn ramp(grid: &HexGrid, middle: f32, faded: u32, full: u32) -> ([f32; 4], [f32; 4]) {
    let colour_at = |from_middle: f32| {
        let share = edge_share(grid.columns / 2, from_middle);
        lerp_colour(faded, full, share)
    };
    (colour_at(middle - 0.5), colour_at(middle + 0.5))
}

/// An image on a cell outside the selected column, shaded across by `colours`.
fn shaded(rect: [f32; 4], uv: [f32; 4], (left, right): ([f32; 4], [f32; 4])) -> Draw {
    Draw::GradientSprite {
        rect,
        uv,
        left,
        right,
    }
}

/// What a cell shows on its hexagon.
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct Face {
    /// A picture, by sheet name, centred at its size.
    pub picture: Option<(String, ArtSize)>,
    /// Whether the cell cannot be picked: it draws a padlock over its picture.
    pub locked: bool,
}

/// How big a cell's picture is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ArtSize {
    /// This many units per texel.
    PerTexel(f32),
    /// Scaled so the longer side is this many units.
    Fit(f32),
}

/// Where the cursor ring sits and how it is tinted: on `row` of the centre column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Cursor {
    pub row: usize,
    /// The ring's offset below its target, in half rows (see [`Slide::ring_offset`]).
    pub low: f32,
}

/// The ring's colour at pulse `phase` (`0..=2`): white to `peak` over the first unit, `peak` to
/// black over the second, each byte `a + t (b - a)` truncated, alpha included
/// (`Colour_LerpArgb`, `docs/ghidra/functions/ps3-hdfury-eu/ship-select-ring.md`, "The pulse").
#[must_use]
pub fn pulse_argb(peak: u32, phase: f32) -> u32 {
    let (from, to, t) = if phase < 1.0 {
        (0xffff_ffff_u32, peak, phase)
    } else {
        (peak, 0xff00_0000, phase - 1.0)
    };
    let byte = |shift: u32| {
        let (a, b) = (
            ((from >> shift) & 0xff) as f32,
            ((to >> shift) & 0xff) as f32,
        );
        ((a + t * (b - a)) as u32).min(255) << shift
    };
    byte(24) | byte(16) | byte(8) | byte(0)
}

/// The pulse's phase `seconds` after the widget was built: `+= dt * 3`, wrapping past 2.
#[must_use]
pub fn pulse_phase(seconds: f32) -> f32 {
    let phase = seconds * 3.0;
    if phase > 2.0 {
        phase.rem_euclid(2.0)
    } else {
        phase
    }
}

/// Where the cursor ring stands: the centre column, on `cursor`'s row and the half row that
/// column sits at.
pub(super) fn ring_centre(
    grid: &HexGrid,
    selected: usize,
    slide: Option<&Slide>,
    cursor: Cursor,
) -> (f32, f32) {
    let parity = match (grid.phase_by_entry, slide) {
        (false, _) => 0.0,
        (true, None) => (selected % 2) as f32,
        (true, Some(slide)) => f32::from(u8::from(slide.ring_target_low())),
    };
    let centre = (grid.columns / 2) as f32;
    (
        grid.origin[0] + centre * PITCH[0] * grid.scale,
        grid.origin[1]
            + cursor.row as f32 * PITCH[1] * grid.scale
            + (parity + cursor.low) * HALF_ROW * grid.scale,
    )
}

/// The grid's draws, behind-to-front per cell: the hexagon, then its face; then the cursor ring
/// once on `cursor`'s row of the centre column, over every cell, as one quad of
/// `Hexagon_HD_THICK_OUT.gtf` pulsing with `seconds` (`phase` of the widget's own clock).
pub(super) fn draw_cells(
    grid: &HexGrid,
    (selected, count, pan, slide): (usize, usize, f32, Option<&Slide>),
    (cursor, seconds): (Option<Cursor>, f32),
    face: &dyn Fn(&Cell) -> Face,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    out: &mut Vec<Draw>,
) {
    for cell in grid.cells_at(selected, count, pan, slide) {
        let first_draw = out.len();
        let chosen = cell.distance == 0;
        let (cx, cy) = centre(cell.rect);
        if let Some(placed) = sprites(HEX_SRC) {
            let [w, h] = QUAD.map(|side| side * grid.scale);
            let rect = [cx - w * 0.5, cy - h * 0.5, w, h];
            if chosen {
                out.push(Draw::Sprite {
                    rect,
                    uv: placed.sheet_rect([0.0, 0.0, QUAD_TEXELS[0], QUAD_TEXELS[1]]),
                    color: argb_to_rgba(grid.selected_column),
                });
            } else {
                let colours = ramp(grid, cell.offset as f32 + pan, grid.hex_fade, grid.hex);
                let uv = placed.sheet_rect([0.0, 0.0, QUAD_TEXELS[0], QUAD_TEXELS[1]]);
                out.push(shaded(rect, uv, colours));
            }
        }
        let Face { picture, locked } = face(&cell);
        let middle = cell.offset as f32 + pan;
        if let Some((src, size)) = picture
            && let Some(placed) = sprites(&src)
        {
            let scale = match size {
                ArtSize::PerTexel(scale) => scale,
                ArtSize::Fit(units) => units / placed.width.max(placed.height) as f32,
            };
            let scale = scale * grid.scale;
            let (w, h) = (placed.width as f32 * scale, placed.height as f32 * scale);
            let rect = [cx - w * 0.5, cy - h * 0.5, w, h];
            let uv = placed.sheet_rect([0.0, 0.0, placed.width as f32, placed.height as f32]);
            if chosen {
                let color = if locked {
                    argb_to_rgba(PICTURE_COL)
                } else {
                    [1.0; 4]
                };
                out.push(Draw::Sprite { rect, uv, color });
            } else {
                let colours = ramp(grid, middle, PICTURE_COL_FADE, PICTURE_COL);
                out.push(shaded(rect, uv, colours));
            }
        }
        if locked && let Some(placed) = sprites(LOCK_SRC) {
            let [w, h] = QUAD.map(|side| side * grid.scale);
            let rect = [cx - w * 0.5, cy - h * 0.5, w, h];
            let uv = placed.sheet_rect([0.0, 0.0, LOCK_QUAD_TEXELS[0], LOCK_QUAD_TEXELS[1]]);
            if chosen {
                let color = argb_to_rgba(grid.selected_lock);
                out.push(Draw::Sprite { rect, uv, color });
            } else {
                let colours = ramp(grid, middle, grid.selected_lock_fade, grid.selected_lock);
                out.push(shaded(rect, uv, colours));
            }
        }
        if cell.weight < 1.0 {
            for draw in &mut out[first_draw..] {
                match draw {
                    Draw::Sprite { color, .. } => color[3] *= cell.weight,
                    Draw::GradientSprite { left, right, .. } => {
                        left[3] *= cell.weight;
                        right[3] *= cell.weight;
                    }
                    _ => {}
                }
            }
        }
    }
    if let (Some(cursor), Some(peak), Some(placed)) =
        (cursor, grid.ring_peak, sprites(THICK_OUT_SRC))
    {
        let (cx, cy) = ring_centre(grid, selected, slide, cursor);
        let [w, h] = QUAD.map(|side| side * grid.scale);
        out.push(Draw::Sprite {
            rect: [cx - w * 0.5, cy - h * 0.5, w, h],
            uv: placed.sheet_rect([0.0, 0.0, QUAD_TEXELS[0], QUAD_TEXELS[1]]),
            color: argb_to_rgba(pulse_argb(peak, pulse_phase(seconds))),
        });
    }
}

/// `Team Selection`'s honeycomb: a team per column, a model per row.
pub(super) fn draw(
    grid: &HexGrid,
    picker: &Picker,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    out: &mut Vec<Draw>,
) {
    let slide = picker.slide();
    let cursor = models_of(picker, slide.shown())
        .iter()
        .position(|cell| cell.variant == Some(picker.variant_index()));
    let face = |cell: &Cell| match models_of(picker, cell.entry).get(cell.row) {
        Some(model) => Face {
            picture: model
                .thumb
                .clone()
                .map(|src| (src, ArtSize::PerTexel(THUMB_SCALE))),
            locked: !model.open,
        },
        None => Face {
            picture: None,
            locked: true,
        },
    };
    draw_cells(
        grid,
        (
            picker.index(),
            picker.entries().len(),
            picker.pan(),
            Some(slide),
        ),
        (
            cursor.map(|row| Cursor {
                row,
                low: slide.ring_offset(),
            }),
            picker.ring_seconds(),
        ),
        &face,
        sprites,
        out,
    );
}
