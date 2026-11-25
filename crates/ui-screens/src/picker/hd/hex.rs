//! `Team Selection`'s `NAVIGATE TEAM` honeycomb - the `HexSelection` widget.
//!
//! # What the disc authors, and what the widget class keeps
//!
//! `Team_Selection_Definition.xml` authors the grid's shape and colours:
//! `columns="5" rows="7"`, `HexCol` / `HexColFade` for a cell and its faded
//! form, and `SelectedLockCol` / `SelectedLockColFade` for a padlock. The art is the
//! front end's own: `Hexagon_HD.gtf` for a cell, `Hexagon_HD_THICK_OUT.gtf`
//! for the cursor, `Padlock.gtf` (the `Padlock` image this screen authors,
//! at 512x512) and each model's `<modellocation>\FE\thumb<LiveryNumber>.gtf`.
//! The widget class's own constants (pitch 72 by 83, half row 42, the 92x80 quad), the selected
//! column's `HD_Blue` fill (`SelectedColumnCol` is not a key it reads), the ring's pulse and the
//! slide on a step are read in
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
//! fresh profile): the hexagon draws at 1.22 times its 72x62 art. The padlock's visible art
//! (91x129 texels) measures 40x56 units, 0.44 per texel; the thumbnail's
//! 1.0 per texel. A padlock reads `128` on the selected column (exactly
//! `SelectedLockCol`), `110` one column out and `68` two out, which is
//! `SelectedLockColFade` blended in by `0.2` and `0.65`. One frame, so
//! confidence 70, not higher.
//!
//! **Chosen, not measured**: the ghost of an open
//! row's thumbnail in the neighbouring columns (the frame shows faint ship
//! silhouettes there; their tint is ours), the pointer targets, and the pulse's zero point (the
//! widget's clock starts with the picker; when the original builds the widget relative to the page
//! entry is unread). **Not wired**: the cells' edge-fade corners, the ring's ease on a model-row
//! change, and Omega's `Scale="0.8"` against the 72-unit pitch.
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

/// The cursor ring's quad, `Hexagon_HD_THICK_OUT.gtf`'s 74x64 texel window drawn 92x80 units
/// (the same quad the widget's cells use), centred on the cell's hexagon.
const RING_QUAD: [f32; 2] = [92.0, 80.0];
const RING_TEXELS: [f32; 2] = [74.0, 64.0];

/// The pulse's period in seconds: the phase advances 3 a second and wraps at 2.
pub const PULSE_PERIOD_SECONDS: f32 = 2.0 / 3.0;

/// A cell's art against the authored 72x62 hexagon.
const HEX_SCALE: f32 = 1.22;
/// The hexagon's visible centre inside its 128x64 texture, in texels.
const HEX_CENTRE: [f32; 2] = [37.0, 31.5];
/// The visible hexagon's size in units: 72x62 at [`HEX_SCALE`].
const HEX_SIZE: [f32; 2] = [72.0 * HEX_SCALE, 62.0 * HEX_SCALE];
/// Units per padlock texel, and per thumbnail texel.
const LOCK_SCALE: f32 = 0.44;
/// The width of HD's `Padlock.gtf`, in texels, which [`LOCK_SCALE`] was measured on.
const LOCK_TEXELS: f32 = 256.0;
const THUMB_SCALE: f32 = 1.0;
/// How far a padlock's colour has faded toward `SelectedLockColFade` one
/// and two columns from the selected one.
const LOCK_FADE: [f32; 3] = [0.0, 0.2, 0.65];

pub const HEX_SRC: &str = r"Data\FE\Images\Hexagon_HD.gtf";
pub const THICK_OUT_SRC: &str = r"Data\FE\Images\Hexagon_HD_THICK_OUT.gtf";
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
    /// The widget's own `Scale`, which shrinks the cell pitch and size alike: `1.0` where it
    /// authors none (HD's `Team Selection`), `0.8` on Omega's, whose grid holds nine rows
    /// where HD's holds seven.
    pub scale: f32,
    pub hex: u32,
    pub hex_fade: u32,
    /// How a cell's fill eases from `hex` to `hex_fade` with its column's distance from the
    /// selected one; `None` for a grid whose cells do not fade.
    pub fade: Option<oag_title::HexFade>,
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
            scale: 1.0,
            hex: HEX_COL,
            hex_fade: HEX_COL_FADE,
            fade: Some(oag_title::HexFade {
                columns: 4.0,
                span: None,
            }),
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

/// A cell's hexagon fill: one colour, or - where the fade has a `span` - a colour at each
/// of the sprite's two edges, so it runs from the value toward the selected column to the
/// value away from it, as the original's per-vertex colours do. The sprite is the whole 128
/// texel texture, its visible hexagon centred on [`HEX_CENTRE`].
fn fill(grid: &HexGrid, cell: &Cell, placed: Placed, rect: [f32; 4]) -> Draw {
    let Some(fade) = grid.fade else {
        return sprite(placed, rect, lerp_colour(grid.hex, grid.hex_fade, 0.0));
    };
    let colour_at = |columns: f32| {
        let t = (columns / fade.columns).clamp(0.0, 1.0);
        lerp_colour(grid.hex, grid.hex_fade, t)
    };
    let Some(span) = fade.span else {
        return sprite(placed, rect, colour_at(cell.distance as f32));
    };
    // Columns of fade at a texel `x` right of the centre: it grows away from the selected
    // column, so rightward on a cell right of it and leftward on one left of it.
    let away = cell.offset.signum() as f32;
    let at = |x: f32| colour_at((cell.distance as f32 + away * x / span).max(0.0));
    Draw::GradientSprite {
        rect,
        uv: placed.sheet_rect([0.0, 0.0, placed.width as f32, placed.height as f32]),
        left: at(-HEX_CENTRE[0]),
        right: at(placed.width as f32 - HEX_CENTRE[0]),
    }
}

fn sprite(placed: Placed, rect: [f32; 4], color: [f32; 4]) -> Draw {
    Draw::Sprite {
        rect,
        uv: placed.sheet_rect([0.0, 0.0, placed.width as f32, placed.height as f32]),
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
        let hex_scale = HEX_SCALE * grid.scale;
        if let Some(placed) = sprites(HEX_SRC) {
            let rect = [
                cx - HEX_CENTRE[0] * hex_scale,
                cy - HEX_CENTRE[1] * hex_scale,
                placed.width as f32 * hex_scale,
                placed.height as f32 * hex_scale,
            ];
            if chosen {
                out.push(sprite(placed, rect, argb_to_rgba(grid.selected_column)));
            } else {
                out.push(fill(grid, &cell, placed, rect));
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
                    let scale = scale * grid.scale;
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
                    // `LOCK_SCALE` is per texel of HD's 256-wide padlock; a title that
                    // ships it larger (Omega's is 512) draws it at the same size.
                    let lock = LOCK_SCALE * grid.scale * LOCK_TEXELS / placed.width as f32;
                    let (w, h) = (placed.width as f32 * lock, placed.height as f32 * lock);
                    let fade = LOCK_FADE[cell.distance.min(LOCK_FADE.len() - 1)];
                    out.push(sprite(
                        placed,
                        [cx - w * 0.5, cy - h * 0.5, w, h],
                        lerp_colour(grid.selected_lock, grid.selected_lock_fade, fade),
                    ));
                }
            }
        }
        if cell.weight < 1.0 {
            for draw in &mut out[first_draw..] {
                if let Draw::Sprite { color, .. } = draw {
                    color[3] *= cell.weight;
                }
            }
        }
    }
    if let (Some(cursor), Some(peak), Some(placed)) =
        (cursor, grid.ring_peak, sprites(THICK_OUT_SRC))
    {
        let parity = match (grid.phase_by_entry, slide) {
            (false, _) => 0.0,
            (true, None) => (selected % 2) as f32,
            (true, Some(slide)) => f32::from(u8::from(slide.ring_target_low())),
        };
        let centre = (grid.columns / 2) as f32;
        let (cx, cy) = (
            grid.origin[0] + centre * PITCH[0] * grid.scale,
            grid.origin[1]
                + cursor.row as f32 * PITCH[1] * grid.scale
                + (parity + cursor.low) * HALF_ROW * grid.scale,
        );
        let [w, h] = RING_QUAD.map(|side| side * grid.scale);
        out.push(Draw::Sprite {
            rect: [cx - w * 0.5, cy - h * 0.5, w, h],
            uv: placed.sheet_rect([0.0, 0.0, RING_TEXELS[0], RING_TEXELS[1]]),
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
        Some(model) if model.open => model.thumb.clone().map_or(Face::Empty, |src| Face::Art {
            src,
            size: ArtSize::PerTexel(THUMB_SCALE),
        }),
        _ => Face::Lock,
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
