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
//! **Not drawn: the new-record banner** (`RecordNotifyBlock`, `RecBlockBlue`/`RecBlockGrey`,
//! `RecLogo*`, `RecValue`). It needs the best-score table `0x0001ef28` returns, which is unread
//! (72), and this build keeps no Detonator best score; `RecBlockGrey` would assert "not a
//! record", so the whole group stays off.

use oag_ui::frontend::Placed;
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Layers, Picture, Skin};

use super::{GRID_HEAD_Y, find_block};
use crate::endrace::draw::{fill_draw, image_draw, text_draw};
use crate::endrace::{DetonatorResults, Layout};

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

/// Widgets this layout never writes: the record banner, the loyalty, target and online blocks,
/// and every grid cell that is not one of the six rows or the score line.
fn hidden(name: &str) -> bool {
    name.starts_with("Grid")
        || name.starts_with("Rec")
        || name.starts_with("loyalty")
        || name.starts_with("Target")
        || matches!(
            name,
            "DelayPostMsg" | "ControlTextConfirmButton" | "MedalBlock"
        )
}

/// The Detonator `EndRace Results`' draw list.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts hd_results_draw_list takes"
)]
pub(super) fn draw_list(
    model: &DetonatorResults,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let mut layers = Layers::behind(skin, frame, backdrop, race_behind);
    let screen = &layout.screen;
    let Some(head) = find_block(screen, "GridHead1") else {
        return layers;
    };
    let origin = (head.x, head.y - GRID_HEAD_Y);
    let mut out = Vec::new();
    for fill in &screen.fills {
        let moved = |y: f32, height: Option<f32>| {
            let mut moved = fill.clone();
            moved.y = origin.1 + y;
            if height.is_some() {
                moved.height = height;
            }
            fill_draw(&moved)
        };
        out.push(match fill.name.as_deref() {
            Some("GridStrikeThrough" | "GridHighlight") => continue,
            Some("GridSideBarL" | "GridSideBarR") => moved(SIDE_BAR_Y, Some(SIDE_BAR_HEIGHT)),
            Some("GridBottomBar") => moved(BOTTOM_BAR_Y, None),
            _ => fill_draw(fill),
        });
    }
    for image in &screen.images {
        let name = image.name.as_deref().unwrap_or("");
        if hidden(name) {
            continue;
        }
        if let Some(placed) = sprites(&image.src) {
            out.push(image_draw(image, placed));
        }
    }
    let rows = model.rows();
    for text in &screen.texts {
        let name = text.name.as_deref().unwrap_or("");
        let cell = match name {
            "Grid0.h" => Some((
                strings.get_or_id(DetonatorResults::SCORE_LABEL).to_string(),
                LABEL_X,
                SCORE_Y,
            )),
            "Grid1.h" => Some((model.score_text(), VALUE_X, SCORE_Y)),
            _ => name
                .strip_prefix("Grid")
                .and_then(|rest| rest.split_once('.'))
                .and_then(|(col, row)| {
                    Some((col.parse::<usize>().ok()?, row.parse::<usize>().ok()?))
                })
                .and_then(|(col, row)| {
                    let (label, value) = rows.get(row)?;
                    let y = ROW_TOP + ROW_PITCH * row as f32;
                    match col {
                        0 => Some((strings.get_or_id(label).to_string(), LABEL_X, y)),
                        1 => Some((value.clone(), VALUE_X, y)),
                        _ => None,
                    }
                }),
        };
        if let Some((content, x, y)) = cell {
            let mut positioned = text.clone();
            positioned.x = origin.0 + x;
            positioned.y = origin.1 + y;
            out.push(text_draw(&positioned, &content, layout));
            continue;
        }
        let content = match name {
            "Line1" => strings.get(DetonatorResults::HEADLINE).map(str::to_owned),
            _ if hidden(name) => None,
            _ => text.string.clone(),
        };
        if let Some(content) = content {
            out.push(text_draw(text, &content, layout));
        }
    }
    layers.body = out;
    layers
}

#[cfg(test)]
mod tests;
