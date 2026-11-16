//! The music wave on 2048's campaign map: the lattice cells near the current
//! event rise and fall with the bass of the playing track.
//!
//! Read off `Canvas3D_UpdateCellHeights` (`0x81071794`, music branch) and
//! `Canvas3D_Update` (`0x81072220`), `docs/ghidra/functions/vita-2048-eu-v104/canvas-music-wave.md`,
//! "The wave":
//!
//! - Every step the music path hands over band 1 of the track's spectrum
//!   (`oag_music::spectrum` reads the sidecar; this crate sees only the level).
//!   The last 30 levels are kept, newest first.
//! - A cell inside the radius whose priority is below [`STILL_PRIORITY`] gains
//!   `int(600 * f * (h[k] - h[k + 1]))` alpha, with `f = (R^2 - d^2) / R^2` and
//!   `k = min(uint((1 - f) * 30), 29)`: a cell at distance `d` follows the bass
//!   delayed by `30 d^2 / R^2` steps.
//! - Every cell loses 230 alpha a second, and the texture the vertex program
//!   reads eases 15 % of the way to the cell's alpha each step.
//!
//! The steps are the original's 60 Hz frames: the ease is per frame there, not
//! per second (confidence 80 on the law; that the player updates once per frame
//! is inferred, 60).
//!
//! **Not ported**: the analyser's automatic gain (`MusicAnalysis_Update`
//! `0x8125f460`, confidence 60, its peak-hold detail unread), so a quiet passage
//! moves the cells less here than in the original. The ripple branch has no
//! writer found.

use std::collections::BTreeMap;

use super::canvas_lattice::{CELL_DX, CELL_DY};

/// A cell whose priority is at or above this is a tile and does not move.
pub const STILL_PRIORITY: u8 = 5;
/// The radius the campaign map gives the wave, in lattice units
/// (`0x810f0a58`).
pub const RADIUS: f32 = 7.0;
/// The seconds one step covers.
pub const STEP: f32 = 1.0 / 60.0;
/// Alpha units per unit of band-1 difference, `0x44160000`.
const GAIN: f32 = 600.0;
/// Alpha lost a second.
const DECAY_PER_SECOND: f32 = 230.0;
/// The share of the way the texture moves to the cell's alpha each step.
const EASE: f32 = 0.15;
/// History length.
const HISTORY: usize = 30;

/// One cell: the alpha the music wrote, and the alpha the texture shows.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Cell {
    alpha: i32,
    shown: f32,
}

/// The wave's state: the bass history and every cell the music has lifted.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Wave {
    /// Band-1 levels, newest first.
    history: [f32; HISTORY],
    cells: BTreeMap<(i32, i32), Cell>,
    /// Seconds not yet paid out as whole alpha units of decay.
    decay: f32,
}

impl Wave {
    /// Advances one [`STEP`]: pushes `band1`, lifts the cells within [`RADIUS`] of
    /// `centre` (a lattice point, as [`super::canvas_camera::tile_top`] gives),
    /// decays and eases. `priority` is the canvas's priority of a cell, 0 for
    /// plain ground ([`super::canvas_cells::priorities`]). `flat` lists the
    /// cell groups that average ([`flatten`]).
    pub fn step(
        &mut self,
        band1: f32,
        centre: Option<(f32, f32)>,
        priority: &dyn Fn((i32, i32)) -> u8,
        flat: &[&[(i32, i32)]],
    ) {
        self.history.copy_within(0..HISTORY - 1, 1);
        self.history[0] = band1;
        if let Some(centre) = centre {
            self.lift(centre, priority);
        }
        self.decay += STEP;
        let lost = (((self.decay * DECAY_PER_SECOND) as u32) & 0xff) as i32;
        self.decay -= lost as f32 / DECAY_PER_SECOND;
        for cell in self.cells.values_mut() {
            cell.alpha = (cell.alpha - lost).max(0);
            cell.shown += EASE * (cell.alpha as f32 - cell.shown);
        }
        for group in flat {
            self.flatten(group);
        }
        self.cells
            .retain(|_, cell| cell.alpha > 0 || cell.shown >= 0.5);
    }

    /// Writes the focus ripple at `t` seconds: every cell within `radius` of
    /// `centre` whose priority is below [`STILL_PRIORITY`] takes
    /// [`super::map_focus::ripple_alpha`] as its alpha, as `Canvas3D_Update`
    /// writes the tile alpha. The decay and the ease in [`Self::step`] fade it.
    pub fn ripple(
        &mut self,
        centre: (f32, f32),
        radius: f32,
        t: f32,
        priority: &dyn Fn((i32, i32)) -> u8,
    ) {
        let cols = ((centre.0 - radius) / CELL_DX).floor() as i32
            ..=((centre.0 + radius) / CELL_DX).ceil() as i32;
        let rows = ((centre.1 - radius) / CELL_DY).floor() as i32
            ..=((centre.1 + radius) / CELL_DY).ceil() as i32;
        for row in rows {
            for col in cols.clone() {
                if priority((col, row)) >= STILL_PRIORITY {
                    continue;
                }
                let dx = col as f32 * CELL_DX - centre.0;
                let dy = row as f32 * CELL_DY - centre.1;
                let d2 = dx * dx + dy * dy;
                if d2 >= radius * radius {
                    continue;
                }
                let alpha = super::map_focus::ripple_alpha(t, d2, radius) as i32;
                if alpha > 0 {
                    self.cells.entry((col, row)).or_default().alpha = alpha;
                }
            }
        }
    }

    /// Whether no cell is raised or rising.
    #[must_use]
    pub fn is_calm(&self) -> bool {
        self.cells.is_empty()
    }

    fn lift(&mut self, centre: (f32, f32), priority: &dyn Fn((i32, i32)) -> u8) {
        let r2 = RADIUS * RADIUS;
        let cols = ((centre.0 - RADIUS) / CELL_DX).floor() as i32
            ..=((centre.0 + RADIUS) / CELL_DX).ceil() as i32;
        let rows = ((centre.1 - RADIUS) / CELL_DY).floor() as i32
            ..=((centre.1 + RADIUS) / CELL_DY).ceil() as i32;
        for row in rows {
            for col in cols.clone() {
                if priority((col, row)) >= STILL_PRIORITY {
                    continue;
                }
                let dx = col as f32 * CELL_DX - centre.0;
                let dy = row as f32 * CELL_DY - centre.1;
                let falloff = ((r2 - (dx * dx + dy * dy)) / r2).clamp(0.0, 1.0);
                if falloff <= 0.0 {
                    continue;
                }
                let k = (((1.0 - falloff) * HISTORY as f32) as usize).min(HISTORY - 1);
                let next = (k + 1).min(HISTORY - 1);
                let add = (GAIN * falloff * (self.history[k] - self.history[next])) as i32;
                if add == 0 {
                    continue;
                }
                let cell = self.cells.entry((col, row)).or_default();
                cell.alpha = (cell.alpha + add).clamp(0, 255);
            }
        }
    }

    /// Averages the alpha of a hidden spot's cells and writes the mean back to
    /// all of them: it moves as one flat value while the ground round it ripples.
    ///
    /// Measured live, confidence 80 (`canvas-music-wave.md`, section 5): the
    /// cells are raised individually and averaged within the frame, after the
    /// texture upload, so the drawn height keeps one frame's differential (the
    /// shown height here eases toward the mean the same way). A hidden event's
    /// button is a 4-cell shape; `group` is whatever cells the button owns. The
    /// caller passes the cells of each revealable hidden event
    /// ([`super::campaign_reveal`]); with none, every cell moves alone.
    pub fn flatten(&mut self, group: &[(i32, i32)]) {
        if group.is_empty() {
            return;
        }
        let mean = group
            .iter()
            .map(|cell| self.cells.get(cell).map_or(0, |c| c.alpha))
            .sum::<i32>()
            / group.len() as i32;
        for cell in group {
            self.cells.entry(*cell).or_default().alpha = mean;
        }
    }

    /// The height of a cell in canvas units: its shown alpha over 255.
    #[must_use]
    pub fn height(&self, cell: (i32, i32)) -> f32 {
        self.cells.get(&cell).map_or(0.0, |c| c.shown / 255.0)
    }

    /// Every cell with a non-zero height, with the height.
    pub fn raised(&self) -> impl Iterator<Item = ((i32, i32), f32)> + '_ {
        self.cells
            .iter()
            .filter(|(_, c)| c.shown > 0.0)
            .map(|(cell, c)| (*cell, c.shown / 255.0))
    }
}

#[cfg(test)]
mod tests;
