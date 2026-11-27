//! HD/Fury's `EndRace Results` for a Detonator run: `Line1`, six label/value rows and the score
//! line (`EndRaceResults_FillDetonatorRows`, `0x002234b0`, and the mode-`0xe` branch of
//! `EndRaceResults_LayoutGrid`, `0x0022c068`;
//! `docs/ghidra/functions/ps3-hdfury-eu/detonator-endrace.md`, "The layout").
//!
//! The frame is Time Trial's with the side bars rewritten: `GridSideBarL`/`R` at `y = 29`, `347`
//! tall, `GridBottomBar` at `y = 391`, `GridTopBar` and `GridBottomBlock` as authored, no column
//! captions, and the grid's rows and score line placed from the grid's own origin. A destroyed run
//! and a finished one draw the same rows.
//!
//! The record notice (`RecordNotifyBlock`) is always drawn: `RecBlockBlue` or `RecBlockGrey`,
//! `RecLogoD` and `RecValue` (`docs/ghidra/functions/ps3-hdfury-eu/detonator-endrace.md`,
//! "The record notice").

use oag_ui::frontend::Draw;
use oag_ui::language::StringTable;
use oag_ui::screen::{Fill, Screen};

use super::{GRID_HEAD_Y, find_block};
use crate::endrace::DetonatorResults;
use crate::endrace::draw::fill_draw;

/// `Grid0.r` (label) and `Grid1.r` (value): `x = 40` (`0x42200000`) and `590` (`0x44138000`).
const LABEL_X: f32 = 40.0;
const VALUE_X: f32 = 590.0;
/// Row `r` at `y = 37 + 40 r` (`li r26,0x25; addi r26,r26,0x28`).
const ROW_TOP: f32 = 37.0;
const ROW_PITCH: f32 = 40.0;
/// The score line (`Grid0.h`, `Grid1.h`): `y = 360` (`0x43b40000`).
const SCORE_Y: f32 = 360.0;
/// The side bars' `y` (`0x008b08d0`), their height (`0x008b08a0`) and the bottom bar's `y`
/// (`0x008b08a4`).
const SIDE_BAR_Y: f32 = 29.0;
const SIDE_BAR_HEIGHT: f32 = 347.0;
const BOTTOM_BAR_Y: f32 = 391.0;

/// The Detonator layout of the results grid, beside [`super::RaceGrid`]: what the mode-`0xe`
/// branch of `0x0022c068` and the filler do to the screen the file authors.
pub(super) struct DetonatorGrid<'a> {
    results: &'a DetonatorResults,
    /// The grid's `Item`: `GridHead1`'s position less its `y = 24`.
    origin: (f32, f32),
}

impl<'a> DetonatorGrid<'a> {
    /// `None` on a screen with no `GridHead1` to anchor the grid on.
    pub(super) fn of(results: &'a DetonatorResults, screen: &Screen) -> Option<Self> {
        let head = find_block(screen, "GridHead1")?;
        Some(Self {
            results,
            origin: (head.x, head.y - GRID_HEAD_Y),
        })
    }

    /// A frame fill as the Detonator layout draws it: the side bars and the bottom bar moved, the
    /// row highlight hidden, every other fill as authored.
    pub(super) fn fill(&self, fill: &Fill) -> Vec<Draw> {
        let moved = |y: f32, height: Option<f32>| {
            let mut moved = fill.clone();
            moved.y = self.origin.1 + y;
            if height.is_some() {
                moved.height = height;
            }
            vec![fill_draw(&moved)]
        };
        match fill.name.as_deref() {
            Some("GridHighlight") => Vec::new(),
            Some("GridSideBarL" | "GridSideBarR") => moved(SIDE_BAR_Y, Some(SIDE_BAR_HEIGHT)),
            Some("GridBottomBar") => moved(BOTTOM_BAR_Y, None),
            _ => vec![fill_draw(fill)],
        }
    }

    /// Whether the layout owns `name`: every grid cell and caption, and the record notice's own
    /// widgets.
    pub(super) fn owns(name: &str) -> bool {
        name.starts_with("Grid") || name.starts_with("Rec")
    }

    /// Whether `name` is the record box this result shows.
    pub(super) fn shows_box(&self, name: &str) -> bool {
        match name {
            "RecBlockBlue" => self.results.record.new,
            "RecBlockGrey" => !self.results.record.new,
            _ => false,
        }
    }

    /// `RecValue`'s `"%s %d"`.
    pub(super) fn record_text(&self, strings: &StringTable) -> String {
        let record = self.results.record;
        format!("{} {}", strings.get_or_id(record.label()), record.value)
    }

    /// A widget's content and position (absolute): the six rows' labels and values and the score
    /// line. `None` for a widget [`Self::owns`] that this layout never writes (rows six to eight,
    /// the other columns, the captions).
    pub(super) fn cell(&self, name: &str, strings: &StringTable) -> Option<(String, f32, f32)> {
        let at = |content: String, x: f32, y: f32| (content, self.origin.0 + x, self.origin.1 + y);
        match name {
            "Grid0.h" => {
                let label = strings.get_or_id(DetonatorResults::SCORE_LABEL).to_string();
                return Some(at(label, LABEL_X, SCORE_Y));
            }
            "Grid1.h" => return Some(at(self.results.score_text(), VALUE_X, SCORE_Y)),
            _ => {}
        }
        let (column, row) = super::grid_slot(name)?;
        let (label, value) = self.results.rows().into_iter().nth(row)?;
        let y = ROW_TOP + ROW_PITCH * row as f32;
        match column {
            0 => Some(at(strings.get_or_id(label).to_string(), LABEL_X, y)),
            1 => Some(at(value, VALUE_X, y)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
