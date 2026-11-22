//! Photo mode's screen for a finger: the same chips as the desktop's, thumb-sized and laid out
//! as a camera app does, with the shutter under the right thumb.
//!
//! **Chosen, not measured.** Sizes follow `oag_input::touch`: a primary target is at least
//! [`BIG`] of the window's height (about 9 mm on a 6 inch phone) and the panel's rows are as
//! tall as the 272-unit grid lets ten of them be ([`PANEL_ROW`]); everything keeps inside
//! `oag_input::touch::SAFE_X` and `SAFE_Y`. Pure functions of the grid, so every rule is a unit
//! test.
//!
//! ```text
//! BACK                                  CAMERA
//! HUD                                  (SHUTTER)
//! DRAG (free lens)                      EFFECTS
//!
//! RESET            < SHIP   SHIP >
//! ```

use oag_input::touch::{SAFE_X, SAFE_Y};
use oag_ui::frontend::{Align, Draw};

use super::{BAND, Chip, Geom, Lens, Photo, SCALE, SELECTED, TEXT, Words, strings};

/// A primary target's least height, as a fraction of the grid's height.
pub const BIG: f32 = 0.13;
/// The shutter's side, as a fraction of the grid's height.
pub const SHUTTER: f32 = 0.20;
/// The effects panel's row height, in grid units: ten rows and the margins fit under 272.
pub const PANEL_ROW: f32 = 25.0;
const GAP: f32 = 4.0;
const SIDE_WIDTH: f32 = 60.0;
const COLUMN_WIDTH: f32 = 70.0;
const SHIP_WIDTH: f32 = 64.0;
const PANEL_WIDTH: f32 = 188.0;
const PANEL_ARROW: f32 = 30.0;
const HINT_LINE: f32 = 9.5;

fn insets(grid: (f32, f32)) -> (f32, f32) {
    (SAFE_X * grid.1, SAFE_Y * grid.1)
}

/// The chips this layout draws, in order.
#[must_use]
pub fn chips(free: bool) -> Vec<Chip> {
    let mut chips = vec![
        Chip::Back,
        Chip::Hud,
        Chip::Reset,
        Chip::Previous,
        Chip::Next,
        Chip::Camera,
        Chip::Take,
        Chip::Effects,
    ];
    if free {
        chips.push(Chip::Drag);
    }
    chips
}

/// Where a chip sits on the overlay grid `grid`: `[x, y, w, h]`.
#[must_use]
pub fn rect(chip: Chip, grid: (f32, f32)) -> [f32; 4] {
    let (sx, sy) = insets(grid);
    let big = BIG * grid.1;
    let shutter = SHUTTER * grid.1;
    let column = grid.0 - sx - COLUMN_WIDTH;
    let shutter_y = grid.1 / 2.0 - shutter / 2.0;
    let bottom = grid.1 - sy - big;
    match chip {
        Chip::Back => [sx, sy, SIDE_WIDTH, big],
        Chip::Hud => [sx, sy + big + GAP, SIDE_WIDTH, big],
        Chip::Drag => [sx, sy + 2.0 * (big + GAP), SIDE_WIDTH, big],
        Chip::Reset => [sx, bottom, SIDE_WIDTH, big],
        Chip::Previous => [
            grid.0 / 2.0 - GAP / 2.0 - SHIP_WIDTH,
            bottom,
            SHIP_WIDTH,
            big,
        ],
        Chip::Next => [grid.0 / 2.0 + GAP / 2.0, bottom, SHIP_WIDTH, big],
        Chip::Camera => [column, shutter_y - GAP - big, COLUMN_WIDTH, big],
        Chip::Take => [
            column + (COLUMN_WIDTH - shutter) / 2.0,
            shutter_y,
            shutter,
            shutter,
        ],
        Chip::Effects => [column, shutter_y + shutter + GAP, COLUMN_WIDTH, big],
    }
}

/// The chip under a point on the grid, if any. `free` is whether the free lens is up.
#[must_use]
pub fn chip_at(grid: (f32, f32), at: (f32, f32), free: bool) -> Option<Chip> {
    chips(free).into_iter().find(|&chip| {
        let [x, y, w, h] = rect(chip, grid);
        at.0 >= x && at.0 < x + w && at.1 >= y && at.1 < y + h
    })
}

/// Where the effects panel sits: left of the right-hand column, from the top of the safe area.
pub(super) fn panel(grid: (f32, f32)) -> Geom {
    let (sx, sy) = insets(grid);
    Geom {
        top: sy,
        right: sx + COLUMN_WIDTH + 2.0 * GAP,
        width: PANEL_WIDTH,
        row: PANEL_ROW,
        arrow: PANEL_ARROW,
    }
}

/// The chips, the panel and the gesture hint for a finger.
pub(super) fn chrome(photo: &Photo, grid: (f32, f32), words: &Words) -> Vec<Draw> {
    let say = words.say;
    let text = |x: f32, y: f32, text: String| Draw::Text {
        x,
        y,
        scale: SCALE,
        color: TEXT,
        border: None,
        align: Align::Centre,
        text,
        wrap_width: None,
    };
    let lens = match photo.lens() {
        Lens::Internal => strings::INTERNAL,
        Lens::Close => strings::CLOSE,
        Lens::Track => strings::TRACK,
        Lens::Free => strings::FREE,
    };
    let on_off = |on: bool| say(if on { strings::ON } else { strings::OFF });
    let label = |chip: Chip| match chip {
        Chip::Camera => format!("{}: {}", say(strings::CAMERA), say(lens)),
        Chip::Hud => format!("{}: {}", say(strings::HUD), on_off(photo.hud())),
        Chip::Effects => say(strings::EFFECTS),
        Chip::Take => say(strings::TAKE),
        Chip::Back => say(strings::BACK),
        Chip::Reset => say(strings::RESET),
        Chip::Previous => format!("< {}", say(strings::SHIP)),
        Chip::Next => format!("{} >", say(strings::SHIP)),
        Chip::Drag => format!(
            "{}: {}",
            say(strings::DRAG),
            say(if photo.drag_moves() {
                strings::MOVE
            } else {
                strings::LOOK
            })
        ),
    };
    let mut list = Vec::new();
    for chip in chips(photo.lens() == Lens::Free) {
        let rect = rect(chip, grid);
        let colour = if chip == Chip::Take { SELECTED } else { BAND };
        list.push(Draw::Fill {
            rect,
            color: colour,
        });
        if chip == Chip::Take {
            list.push(Draw::Fill {
                rect: [rect[0] + 2.0, rect[1] + 2.0, rect[2] - 4.0, rect[3] - 4.0],
                color: BAND,
            });
        }
        list.push(text(
            rect[0] + rect[2] / 2.0,
            rect[1] + rect[3] / 2.0 - 3.0,
            label(chip),
        ));
    }
    if photo.panel() {
        list.extend(super::panel_list(photo, grid, say, true));
    } else {
        let hint = if photo.lens() == Lens::Free {
            strings::TOUCH_HINT_FREE
        } else {
            strings::TOUCH_HINT
        };
        let (_, sy) = insets(grid);
        let y = grid.1 - sy - BIG * grid.1 - GAP - HINT_LINE;
        list.push(text(grid.0 / 2.0, y, say(hint)));
    }
    list
}

#[cfg(test)]
mod tests;
