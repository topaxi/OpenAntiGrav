//! The replay viewer's screen for a finger: thumb-sized controls inside the safe area, a
//! full-width bar to scrub, and what a double tap on each third of the picture means.
//!
//! **Chosen, not measured.** Sizes follow `oag_input::touch` (a primary target is at least
//! [`BIG`] of the grid's height). Pure functions of the grid, so every rule is a unit test.
//!
//! ```text
//! EXIT                       PHOTO  CAMERA  HUD
//!                                  < CRAFT  CRAFT >
//!
//!      0:12 / 1:20
//!  [==========|-------------------------------]
//!        -5s     PLAY     +5s     1X
//! ```

use oag_input::touch::{SAFE_X, SAFE_Y};
use oag_ui::frontend::{Align, Draw};

use super::{
    BAND, BUILT, Control, PLAYED, SCALE, TEXT, TRACK, View, banners, clock, speed_label, strings,
};

/// A primary target's least height, as a fraction of the grid's height.
pub const BIG: f32 = 0.13;
/// Ticks a playing viewer waits without a touch before it hides its controls (3 s).
pub const CHROME_HIDE_TICKS: u32 = 180;
const GAP: f32 = 4.0;
const SIDE_WIDTH: f32 = 60.0;
const TOP_WIDTH: f32 = 62.0;
const TRANSPORT_WIDTH: f32 = 56.0;
const PLAY_WIDTH: f32 = 72.0;
const BAR_HEIGHT: f32 = 5.0;

fn insets(grid: (f32, f32)) -> (f32, f32) {
    (SAFE_X * grid.1, SAFE_Y * grid.1)
}

/// The controls this layout draws, in order. The bar is not among them.
pub const CONTROLS: [Control; 10] = [
    Control::Exit,
    Control::Photo,
    Control::Camera,
    Control::Hud,
    Control::Previous,
    Control::Next,
    Control::Back,
    Control::Play,
    Control::Forward,
    Control::Speed,
];

/// Where a control sits on the overlay grid `grid`: `[x, y, w, h]`. The bar's rectangle is its
/// finger-sized hit area, taller than the strip drawn in it.
#[must_use]
pub fn rect(control: Control, grid: (f32, f32)) -> [f32; 4] {
    let (sx, sy) = insets(grid);
    let big = BIG * grid.1;
    let right = |n: usize| grid.0 - sx - (n as f32 + 1.0) * TOP_WIDTH - n as f32 * GAP;
    let transport_y = grid.1 - sy - big;
    let row_width = 2.0 * TRANSPORT_WIDTH + PLAY_WIDTH + TRANSPORT_WIDTH + 3.0 * GAP;
    let left = (grid.0 - row_width) / 2.0;
    match control {
        Control::Exit => [sx, sy, SIDE_WIDTH, big],
        Control::Photo => [right(2), sy, TOP_WIDTH, big],
        Control::Camera => [right(1), sy, TOP_WIDTH, big],
        Control::Hud => [right(0), sy, TOP_WIDTH, big],
        Control::Previous => [right(1), sy + big + GAP, TOP_WIDTH, big],
        Control::Next => [right(0), sy + big + GAP, TOP_WIDTH, big],
        Control::Back => [left, transport_y, TRANSPORT_WIDTH, big],
        Control::Play => [left + TRANSPORT_WIDTH + GAP, transport_y, PLAY_WIDTH, big],
        Control::Forward => [
            left + TRANSPORT_WIDTH + PLAY_WIDTH + 2.0 * GAP,
            transport_y,
            TRANSPORT_WIDTH,
            big,
        ],
        Control::Speed => [
            left + 2.0 * TRANSPORT_WIDTH + PLAY_WIDTH + 3.0 * GAP,
            transport_y,
            TRANSPORT_WIDTH,
            big,
        ],
        Control::Bar => [sx, transport_y - GAP - big, grid.0 - 2.0 * sx, big],
    }
}

/// The strip of the bar that is drawn: `[x, y, w, h]`, centred in its hit area.
#[must_use]
pub fn bar_strip(grid: (f32, f32)) -> [f32; 4] {
    let [x, y, w, h] = rect(Control::Bar, grid);
    [x, y + (h - BAR_HEIGHT) / 2.0, w, BAR_HEIGHT]
}

/// What a point on the grid is over, if anything.
#[must_use]
pub fn control_at(grid: (f32, f32), at: (f32, f32)) -> Option<Control> {
    let inside = |[x, y, w, h]: [f32; 4]| at.0 >= x && at.0 < x + w && at.1 >= y && at.1 < y + h;
    CONTROLS
        .into_iter()
        .chain([Control::Bar])
        .find(|&control| inside(rect(control, grid)))
}

/// The tick a touch at `x` on the bar means, out of `ticks`.
#[must_use]
pub fn tick_at(grid: (f32, f32), x: f32, ticks: u64) -> u64 {
    let [left, _, width, _] = bar_strip(grid);
    let fraction = ((x - left) / width).clamp(0.0, 1.0);
    (fraction as f64 * ticks as f64).round() as u64
}

/// What a double tap on the picture does, by the third it lands in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// Five seconds back.
    Back,
    /// Play or pause.
    Toggle,
    /// Five seconds on.
    Forward,
}

/// The zone of a double tap at grid `x`.
#[must_use]
pub fn zone_at(grid: (f32, f32), x: f32) -> Zone {
    if x < grid.0 / 3.0 {
        Zone::Back
    } else if x >= grid.0 * 2.0 / 3.0 {
        Zone::Forward
    } else {
        Zone::Toggle
    }
}

/// Whether the controls are showing: a paused viewer always shows them, a playing one hides
/// them after [`CHROME_HIDE_TICKS`] without a touch, and a tap on the picture toggles them.
#[must_use]
pub const fn chrome_shown(forced_hidden: bool, playing: bool, idle_ticks: u32) -> bool {
    !forced_hidden && (!playing || idle_ticks < CHROME_HIDE_TICKS)
}

/// The viewer's screen for a finger. With `chrome` off only the banners are left, and a thin
/// line along the bottom edge that says where the playhead is.
#[must_use]
pub fn draw_list(
    view: &View,
    grid: (f32, f32),
    say: &dyn Fn(&str) -> String,
    chrome: bool,
) -> Vec<Draw> {
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
    let fraction = |tick: u64| {
        if view.ticks == 0 {
            0.0
        } else {
            (tick.min(view.ticks) as f32 / view.ticks as f32).clamp(0.0, 1.0)
        }
    };
    let mut list = Vec::new();
    if chrome {
        let [x, y, w, h] = bar_strip(grid);
        list.push(Draw::Fill {
            rect: [x - 2.0, y - 2.0, w + 4.0, h + 4.0],
            color: BAND,
        });
        list.push(Draw::Fill {
            rect: [x, y, w, h],
            color: TRACK,
        });
        if let Some(built) = view.built {
            list.push(Draw::Fill {
                rect: [x, y, w * fraction(built), h],
                color: BUILT,
            });
        }
        list.push(Draw::Fill {
            rect: [x, y, w * fraction(view.at), h],
            color: PLAYED,
        });
        list.push(Draw::Fill {
            rect: [x + w * fraction(view.at) - 1.0, y - 3.0, 3.0, h + 6.0],
            color: TEXT,
        });
        list.push(text(
            grid.0 / 2.0,
            y - 14.0,
            format!("{} / {}", clock(view.at), clock(view.ticks)),
        ));
        for control in CONTROLS {
            let rect = rect(control, grid);
            let label = match control {
                Control::Play => say(if view.playing && !view.at_end {
                    strings::PAUSE
                } else {
                    strings::PLAY
                }),
                Control::Back => format!("-{}", say(strings::SKIP)),
                Control::Forward => format!("+{}", say(strings::SKIP)),
                Control::Speed => speed_label(view.speed),
                Control::Photo => say(strings::PHOTO),
                Control::Exit => say(strings::EXIT),
                Control::Camera => say(strings::CAMERA),
                Control::Hud => say(strings::HUD),
                Control::Previous => format!("< {}", say(strings::CRAFT)),
                Control::Next => format!("{} >", say(strings::CRAFT)),
                Control::Bar => String::new(),
            };
            list.push(Draw::Fill { rect, color: BAND });
            list.push(text(
                rect[0] + rect[2] / 2.0,
                rect[1] + rect[3] / 2.0 - 3.0,
                label,
            ));
        }
    } else {
        list.push(Draw::Fill {
            rect: [0.0, grid.1 - 2.0, grid.0, 2.0],
            color: TRACK,
        });
        list.push(Draw::Fill {
            rect: [0.0, grid.1 - 2.0, grid.0 * fraction(view.at), 2.0],
            color: PLAYED,
        });
    }
    list.extend(banners(view, grid, say, grid.1 * 0.3));
    list
}

#[cfg(test)]
mod tests;
