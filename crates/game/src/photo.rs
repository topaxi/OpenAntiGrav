//! Photo mode's screen and its files: what is drawn over the picture, where a shot goes,
//! and when the mode is on offer.
//!
//! The camera is `oag_raceplay::photo`; this is the half that needs a screen and a disc.
//! The mode is the pause plus a camera: nothing here ticks the race, and the picture it saves
//! is the frame the player sees, at the size they see it, without the screen's own text.
//!
//! **Chosen, not measured:** the screen's layout and the file name.

use std::path::{Path, PathBuf};

use oag_gameplay::input::Button;
use oag_raceplay::photo::{Lens, Photo};
use oag_ui::frontend::{Align, Draw};

/// The directory under the user's data directory shots are written to.
pub const DIRECTORY: &str = "photos";

/// Whether this build can write a shot: a desktop can; a page and a phone have no directory the
/// player can reach (the row is left out there rather than offered and failing).
pub const SAVES: bool = !cfg!(target_arch = "wasm32") && !oag_mobile::MOBILE;

/// Whether photo mode is on offer for a race drawn as `views` views. A split screen has no
/// single picture to frame and no single pad to drive it.
#[must_use]
pub fn offered(views: usize) -> bool {
    SAVES && views <= 1
}

/// Whether the race ticks: not while photo mode is open. The rule the frame loop asks, in one
/// place so the test of the hash and the loop cannot disagree.
#[must_use]
pub fn ticks(open: bool) -> bool {
    !open
}

/// Where the next shot of `title` goes in `dir`: a name no earlier shot has, so a second picture
/// in the same second does not replace the first.
#[must_use]
pub fn shot_path(dir: &Path, title: &str, unix_seconds: u64) -> PathBuf {
    let slug: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    (1_u32..)
        .map(|n| dir.join(format!("oag-{slug}-{unix_seconds}-{n}.png")))
        .find(|path| !path.exists())
        .unwrap_or_else(|| dir.join(format!("oag-{slug}-{unix_seconds}.png")))
}

/// The ids of the screen's words, in `assets/ui/strings/english.toml`.
pub mod strings {
    pub const CAMERA: &str = "OAG_PHOTO_CAMERA";
    pub const INTERNAL: &str = "OAG_PHOTO_INTERNAL";
    pub const CLOSE: &str = "OAG_PHOTO_CLOSE";
    pub const TRACK: &str = "OAG_PHOTO_TRACK";
    pub const HUD: &str = "OAG_PHOTO_HUD";
    pub const ON: &str = "OAG_PHOTO_ON";
    pub const OFF: &str = "OAG_PHOTO_OFF";
    pub const TAKE: &str = "OAG_PHOTO_TAKE";
    pub const BACK: &str = "OAG_PHOTO_BACK";
    pub const STICK: &str = "OAG_PHOTO_STICK";
    pub const ROTATE: &str = "OAG_PHOTO_ROTATE";
    pub const ZOOM_TILT: &str = "OAG_PHOTO_ZOOM_TILT";
    pub const PAN: &str = "OAG_PHOTO_PAN";
    pub const RESET: &str = "OAG_PHOTO_RESET";
    pub const SHIP: &str = "OAG_PHOTO_SHIP";
    pub const HELP: &str = "OAG_PHOTO_HELP";
    pub const SAVED: &str = "OAG_PHOTO_SAVED";
    pub const FAILED: &str = "OAG_PHOTO_FAILED";
    pub const ROW: &str = "OAG_MENU_PHOTO_MODE";
}

/// The four things the screen can be clicked on, in the order they are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chip {
    /// The next lens.
    Camera,
    /// The HUD in the picture, on or off.
    Hud,
    /// Take the shot.
    Take,
    /// Leave the mode.
    Back,
}

/// The chips, in order.
pub const CHIPS: [Chip; 4] = [Chip::Camera, Chip::Hud, Chip::Take, Chip::Back];

const MARGIN: f32 = 8.0;
const CHIP_HEIGHT: f32 = 18.0;
const GAP: f32 = 4.0;
const BAND: [f32; 4] = [0.0, 0.0, 0.0, 0.6];
const TEXT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const LINE: f32 = 12.0;

/// Where a chip sits on the overlay grid `grid` (width, height): `[x, y, w, h]`.
#[must_use]
pub fn chip_rect(chip: Chip, grid: (f32, f32)) -> [f32; 4] {
    let index = CHIPS.iter().position(|&c| c == chip).unwrap_or(0);
    let width = (grid.0 - 2.0 * MARGIN - GAP * 3.0) / 4.0;
    [
        MARGIN + index as f32 * (width + GAP),
        MARGIN,
        width,
        CHIP_HEIGHT,
    ]
}

/// The chip under a point on the grid, if any.
#[must_use]
pub fn chip_at(grid: (f32, f32), at: (f32, f32)) -> Option<Chip> {
    CHIPS.into_iter().find(|&chip| {
        let [x, y, w, h] = chip_rect(chip, grid);
        at.0 >= x && at.0 < x + w && at.1 >= y && at.1 < y + h
    })
}

/// What the screen needs to word itself: the player's words and the keys their devices have.
#[allow(missing_debug_implementations)]
pub struct Words<'a> {
    /// The text for a project id.
    pub say: &'a dyn Fn(&str) -> String,
    /// The name of a button and the first key bound to it, as `CROSS (ENTER)`.
    pub button: &'a dyn Fn(Button) -> String,
}

/// The screen over the picture: the chips along the top, a message from the last shot, and the
/// help along the bottom. With the help hidden only the message is left. Everything is in the
/// overlay, which a shot does not read.
#[must_use]
pub fn draw_list(
    photo: &Photo,
    grid: (f32, f32),
    words: &Words,
    message: Option<&str>,
) -> Vec<Draw> {
    let say = words.say;
    let mut list = Vec::new();
    let text = |x: f32, y: f32, text: String, align: Align| Draw::Text {
        x,
        y,
        scale: 1.0,
        color: TEXT,
        border: None,
        align,
        text,
        wrap_width: None,
    };
    if let Some(message) = message {
        list.push(Draw::Fill {
            rect: [0.0, grid.1 * 0.5 - 10.0, grid.0, 20.0],
            color: BAND,
        });
        list.push(text(
            grid.0 / 2.0,
            grid.1 * 0.5 - 4.0,
            message.to_string(),
            Align::Centre,
        ));
    }
    if !photo.help() {
        return list;
    }
    let lens = match photo.lens() {
        Lens::Internal => strings::INTERNAL,
        Lens::Close => strings::CLOSE,
        Lens::Track => strings::TRACK,
    };
    let chip = |chip: Chip| match chip {
        Chip::Camera => format!("{}: {}", say(strings::CAMERA), say(lens)),
        Chip::Hud => format!(
            "{}: {}",
            say(strings::HUD),
            say(if photo.hud() {
                strings::ON
            } else {
                strings::OFF
            })
        ),
        Chip::Take => say(strings::TAKE),
        Chip::Back => say(strings::BACK),
    };
    for c in CHIPS {
        let rect = chip_rect(c, grid);
        list.push(Draw::Fill { rect, color: BAND });
        list.push(text(
            rect[0] + rect[2] / 2.0,
            rect[1] + 3.0,
            chip(c),
            Align::Centre,
        ));
    }
    let button = words.button;
    let lines = [
        format!(
            "{} {}    {} {}    {} {}",
            button(Button::Cross),
            say(strings::TAKE),
            button(Button::Circle),
            say(strings::BACK),
            button(Button::Start),
            say(strings::HELP),
        ),
        format!(
            "{} {}    {} {}    {} {}",
            button(Button::Select),
            say(strings::CAMERA),
            button(Button::Square),
            say(strings::HUD),
            button(Button::Triangle),
            say(strings::RESET),
        ),
        format!(
            "{} / {} {}    {}",
            button(Button::L),
            button(Button::R),
            say(strings::SHIP),
            say_lens_keys(photo, say, button),
        ),
    ];
    let top = grid.1 - MARGIN - LINE * lines.len() as f32;
    list.push(Draw::Fill {
        rect: [0.0, top - 4.0, grid.0, grid.1 - top + 4.0],
        color: BAND,
    });
    for (i, line) in lines.into_iter().enumerate() {
        list.push(text(MARGIN, top + LINE * i as f32, line, Align::Left));
    }
    list
}

/// The line of what the stick does on the lens that is up.
fn say_lens_keys(
    photo: &Photo,
    say: &dyn Fn(&str) -> String,
    button: &dyn Fn(Button) -> String,
) -> String {
    let stick = say(strings::STICK);
    match photo.lens() {
        Lens::Internal => String::new(),
        Lens::Close => format!(
            "{stick} {}    {} + {stick} {}    {} + {stick} {}",
            say(strings::ROTATE),
            button(Button::Select),
            say(strings::ZOOM_TILT),
            button(Button::Square),
            say(strings::PAN)
        ),
        Lens::Track => format!("{stick} {}", say(strings::ZOOM_TILT)),
    }
}

#[cfg(test)]
mod tests;
