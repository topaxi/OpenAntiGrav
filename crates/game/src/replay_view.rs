//! The replay viewer's screen: a bar with the built prefix and the playhead, five controls and
//! the banner for a build that stops matching the recording. The rules of playback and seeking
//! are `oag_raceplay::viewer`; this is what is drawn over the picture and what a click on it
//! means.
//!
//! Everything is in the overlay grid, over the surface, as photo mode's screen is
//! (`crate::photo`). **Chosen, not measured:** the layout and the words.

use oag_ui::frontend::{Align, Draw};

/// The ids of the screen's words, in `assets/ui/strings/english.toml`.
pub mod strings {
    pub const OPENING: &str = "OAG_REPLAY_OPENING";
    pub const MISMATCH: &str = "OAG_REPLAY_MISMATCH";
    pub const STOPPED: &str = "OAG_REPLAY_STOPPED";
    pub const PLAY: &str = "OAG_REPLAY_PLAY";
    pub const PAUSE: &str = "OAG_REPLAY_PAUSE";
    pub const EXIT: &str = "OAG_REPLAY_EXIT";
    pub const SPEED: &str = "OAG_REPLAY_SPEED";
    pub const SKIP: &str = "OAG_REPLAY_SKIP";
    pub const ENDED: &str = "OAG_REPLAY_ENDED";
    pub const POSE_VIEW: &str = "OAG_REPLAY_POSE_VIEW";
    pub const POSE_END: &str = "OAG_REPLAY_POSE_END";
}

/// What can be clicked, in the order the chips are drawn, and the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// Play, or pause.
    Play,
    /// Five seconds back.
    Back,
    /// Five seconds on.
    Forward,
    /// The next speed.
    Speed,
    /// Leave the viewer.
    Exit,
    /// The bar: a click seeks to where it lands.
    Bar,
}

/// The chips, in order.
pub const CHIPS: [Control; 5] = [
    Control::Play,
    Control::Back,
    Control::Forward,
    Control::Speed,
    Control::Exit,
];

const MARGIN: f32 = 8.0;
const GAP: f32 = 4.0;
const CHIP_WIDTH: f32 = 56.0;
const CHIP_HEIGHT: f32 = 14.0;
const BAR_HEIGHT: f32 = 5.0;
const BAND: [f32; 4] = [0.0, 0.0, 0.0, 0.6];
const TRACK: [f32; 4] = [1.0, 1.0, 1.0, 0.18];
const BUILT: [f32; 4] = [1.0, 1.0, 1.0, 0.38];
const PLAYED: [f32; 4] = [0.35, 0.8, 1.0, 1.0];
const WARNING: [f32; 4] = [0.85, 0.35, 0.05, 0.9];
const TEXT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const SCALE: f32 = 0.75;
const LINE: f32 = 9.5;

/// Where a chip sits on the overlay grid `grid` (width, height): `[x, y, w, h]`. The chips are a
/// row across the top, centred, between the lap box and the position of every title's HUD; the
/// bottom of the picture is the HUD's own (times, speed, shield), so only the bar goes there.
#[must_use]
pub fn chip_rect(chip: Control, grid: (f32, f32)) -> [f32; 4] {
    let index = CHIPS.iter().position(|&c| c == chip).unwrap_or(0);
    let row = CHIP_WIDTH * CHIPS.len() as f32 + GAP * (CHIPS.len() - 1) as f32;
    [
        (grid.0 - row) / 2.0 + index as f32 * (CHIP_WIDTH + GAP),
        MARGIN,
        CHIP_WIDTH,
        CHIP_HEIGHT,
    ]
}

/// Where the bar sits: `[x, y, w, h]`, a thin strip along the bottom edge.
#[must_use]
pub fn bar_rect(grid: (f32, f32)) -> [f32; 4] {
    [
        MARGIN,
        grid.1 - MARGIN / 2.0 - BAR_HEIGHT,
        grid.0 - 2.0 * MARGIN,
        BAR_HEIGHT,
    ]
}

/// What a point on the grid is over, if anything. The bar is taller to the pointer than it is
/// drawn, so a finger lands on it.
#[must_use]
pub fn control_at(grid: (f32, f32), at: (f32, f32)) -> Option<Control> {
    let inside = |[x, y, w, h]: [f32; 4]| at.0 >= x && at.0 < x + w && at.1 >= y && at.1 < y + h;
    let [x, y, w, h] = bar_rect(grid);
    if inside([x, y - 8.0, w, h + 8.0 + MARGIN / 2.0]) {
        return Some(Control::Bar);
    }
    CHIPS
        .into_iter()
        .find(|&chip| inside(chip_rect(chip, grid)))
}

/// The tick a click at `x` on the bar means, out of `ticks`.
#[must_use]
pub fn tick_at(grid: (f32, f32), x: f32, ticks: u64) -> u64 {
    let [left, _, width, _] = bar_rect(grid);
    let fraction = ((x - left) / width).clamp(0.0, 1.0);
    (fraction as f64 * ticks as f64).round() as u64
}

/// `m:ss` of a tick count at 60 ticks a second.
#[must_use]
pub fn clock(ticks: u64) -> String {
    let seconds = ticks / 60;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// What a frame of the viewer shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    /// The playhead.
    pub at: u64,
    /// The recording's length.
    pub ticks: u64,
    /// The last tick built, `None` while the opening pass has not cut the first keyframe.
    pub built: Option<u64>,
    /// Playing, not paused.
    pub playing: bool,
    /// The speed in sixteenths of real time.
    pub speed: u32,
    /// The tick the build stops matching the recording at, when it does.
    pub mismatch: Option<u64>,
    /// The playhead has reached the end of what can be played.
    pub at_end: bool,
    /// The playhead is past what the build reproduces, in the recorded poses.
    pub pose_view: bool,
    /// The pose track ends before the recording does: its end is named, not shown as the end
    /// of the replay.
    pub track_short: bool,
}

/// The speed as the chip says it: `1X`, `0.5X`.
#[must_use]
pub fn speed_label(speed: u32) -> String {
    match speed {
        16 => "1X".to_string(),
        8 => "0.5X".to_string(),
        other => format!("{}X", other / 16),
    }
}

/// The screen over the picture, as draws in `grid`; `say` is the text of a project id and
/// `button` the name of a button and its first key.
#[must_use]
pub fn draw_list(
    view: &View,
    grid: (f32, f32),
    say: &dyn Fn(&str) -> String,
    keys: &str,
) -> Vec<Draw> {
    let text = |x: f32, y: f32, text: String, align: Align| Draw::Text {
        x,
        y,
        scale: SCALE,
        color: TEXT,
        border: None,
        align,
        text,
        wrap_width: None,
    };
    let mut list = Vec::new();
    let [x, y, w, h] = bar_rect(grid);
    let fraction = |tick: u64| {
        if view.ticks == 0 {
            0.0
        } else {
            (tick.min(view.ticks) as f32 / view.ticks as f32).clamp(0.0, 1.0)
        }
    };
    list.push(Draw::Fill {
        rect: [0.0, y - 2.0, grid.0, grid.1 - y + 2.0],
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
        rect: [x + w * fraction(view.at) - 1.0, y - 2.0, 2.0, h + 4.0],
        color: TEXT,
    });
    let first = chip_rect(Control::Play, grid);
    let last = chip_rect(Control::Exit, grid);
    let time = format!("{} / {}", clock(view.at), clock(view.ticks));
    let line = format!("{time}    {keys}");
    let width = line.chars().count() as f32 * 4.7 + 12.0;
    list.push(Draw::Fill {
        rect: [
            (first[0] + last[0] + last[2]) / 2.0 - width / 2.0,
            first[1] + CHIP_HEIGHT + 2.0,
            width,
            LINE + 2.0,
        ],
        color: BAND,
    });
    list.push(text(
        (first[0] + last[0] + last[2]) / 2.0,
        first[1] + CHIP_HEIGHT + 4.0,
        line,
        Align::Centre,
    ));
    for chip in CHIPS {
        let rect = chip_rect(chip, grid);
        let label = match chip {
            Control::Play => say(if view.playing && !view.at_end {
                strings::PAUSE
            } else {
                strings::PLAY
            }),
            Control::Back => format!("-{}", say(strings::SKIP)),
            Control::Forward => format!("+{}", say(strings::SKIP)),
            Control::Speed => speed_label(view.speed),
            Control::Exit => say(strings::EXIT),
            Control::Bar => String::new(),
        };
        list.push(Draw::Fill { rect, color: BAND });
        list.push(text(
            rect[0] + rect[2] / 2.0,
            rect[1] + 3.0,
            label,
            Align::Centre,
        ));
    }
    if view.built.is_none() {
        list.push(Draw::Fill {
            rect: [0.0, grid.1 * 0.5 - 10.0, grid.0, 20.0],
            color: BAND,
        });
        list.push(text(
            grid.0 / 2.0,
            grid.1 * 0.5 - 4.0,
            say(strings::OPENING),
            Align::Centre,
        ));
    }
    if let Some(stops) = view.mismatch {
        let top = MARGIN + CHIP_HEIGHT + LINE + 8.0;
        list.push(Draw::Fill {
            rect: [0.0, top, grid.0, 2.0 * LINE + 6.0],
            color: WARNING,
        });
        list.push(text(
            grid.0 / 2.0,
            top + 3.0,
            say(strings::MISMATCH).replace("%s", &clock(stops)),
            Align::Centre,
        ));
        let line = if view.pose_view && view.at_end && view.track_short {
            Some(strings::POSE_END)
        } else if view.pose_view {
            Some(strings::POSE_VIEW)
        } else {
            view.at_end.then_some(strings::STOPPED)
        };
        if let Some(id) = line {
            list.push(text(grid.0 / 2.0, top + 3.0 + LINE, say(id), Align::Centre));
        }
    } else if view.at_end {
        list.push(text(
            grid.0 / 2.0,
            grid.1 * 0.5 - 4.0,
            say(strings::ENDED),
            Align::Centre,
        ));
    }
    list
}

#[cfg(test)]
mod tests;
