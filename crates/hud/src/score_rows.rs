//! A score Eliminator's panels behind [`super::kill_tags`]' rows: the `TARGET` header's panel, a
//! bracket per row and `HighlightBar` on the player's row. HD's `HUD_Elim_positions.xml` authors
//! them as children of `PosTag0` to `PosTag7`, with no name to find them by but their place in
//! the layout. `docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`.
//!
//! **The bar's width is the layout's.** `Hud_PlaceHighlightBar` (`0x0008c568`) fixes its right edge
//! and pulls its left edge in to the row's measured text, and this list has no font to measure
//! with, so it ends at the middle of the row's bracket and runs left by its authored width.

use super::{Layout, Readout, Sprite};

const BAR: &str = "HighlightBar";
/// The layout's last widget in the rows.
const LAST: &str = "MPWeaponIcon7";

/// The panels to draw this frame, in layout order.
#[must_use]
pub fn sprites(layout: &Layout, readout: &Readout) -> Vec<Sprite> {
    if !readout.score_rows || readout.kill_tags.is_empty() {
        return Vec::new();
    }
    let position = |name: &str| layout.sprites.iter().position(|s| s.name == name);
    let (Some(bar), Some(last)) = (position(BAR), position(LAST)) else {
        return Vec::new();
    };
    if last < bar {
        return Vec::new();
    }
    let bracket_middle = layout.sprites[bar + 1..last]
        .iter()
        .find(|s| s.name.is_empty())
        .map(|s| s.rect[0] + s.rect[2] / 2.0);
    let mut drawn: Vec<Sprite> = layout.sprites[bar + 1..last]
        .iter()
        .filter(|s| s.name.is_empty() || s.name == "PositionTxt2BG")
        .cloned()
        .collect();
    let row = readout.kill_tags.iter().position(|tag| tag.player);
    if let Some(label) = row.and_then(|row| layout.label(&format!("PosTag{row}"))) {
        let mut highlight = layout.sprites[bar].clone();
        highlight.rect[1] = label.y - highlight.rect[3] / 2.0;
        if let Some(right) = bracket_middle {
            highlight.rect[0] = right - highlight.rect[2];
        }
        drawn.push(highlight);
    }
    drawn
}
