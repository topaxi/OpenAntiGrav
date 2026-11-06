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
use oag_raceplay::photo::effects::Row as Effect;
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
    pub const EFFECTS: &str = "OAG_PHOTO_EFFECTS";
    pub const EXPOSURE: &str = "OAG_PHOTO_EXPOSURE";
    pub const SATURATION: &str = "OAG_PHOTO_SATURATION";
    pub const SPEED_BLUR: &str = "OAG_PHOTO_SPEED_BLUR";
    pub const DEPTH: &str = "OAG_PHOTO_DEPTH";
    pub const APERTURE: &str = "OAG_PHOTO_APERTURE";
    pub const FOCUS: &str = "OAG_PHOTO_FOCUS";
    pub const PANEL_KEYS: &str = "OAG_PHOTO_PANEL_KEYS";
    pub const HELP: &str = "OAG_PHOTO_HELP";
    pub const SAVED: &str = "OAG_PHOTO_SAVED";
    pub const FAILED: &str = "OAG_PHOTO_FAILED";
    pub const ROW: &str = "OAG_MENU_PHOTO_MODE";
}

/// The row of the screen a project string id words, for the title's own disc words
/// ([`oag_title::photo::Strings`]); the lens names and the two buttons' chips have no disc row.
#[must_use]
pub fn row_of(id: &str) -> Option<oag_title::photo::Row> {
    use oag_title::photo::Row;
    Some(match id {
        strings::CAMERA => Row::Camera,
        strings::HUD => Row::Hud,
        strings::HELP => Row::Help,
        strings::ROTATE => Row::Rotate,
        strings::PAN => Row::Pan,
        strings::RESET => Row::Reset,
        strings::SHIP => Row::Ship,
        strings::SPEED_BLUR => Row::SpeedBlur,
        strings::DEPTH => Row::Depth,
        _ => return None,
    })
}

/// The disc's own text for the project string `id` on `title`, upper case as the screen is
/// drawn, when the title's disc has a word for that row and its string table carries it.
#[must_use]
pub fn disc_word(
    id: &str,
    words: &oag_title::photo::Strings,
    table: &oag_ui::language::StringTable,
) -> Option<String> {
    let disc = words.disc_id(row_of(id)?)?;
    let text = table.get(disc)?.trim();
    (!text.is_empty()).then(|| text.to_uppercase())
}

/// The five things the screen can be clicked on, in the order they are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chip {
    /// The next lens.
    Camera,
    /// The HUD in the picture, on or off.
    Hud,
    /// The effects panel, up or down.
    Effects,
    /// Take the shot.
    Take,
    /// Leave the mode.
    Back,
}

/// The chips, in order.
pub const CHIPS: [Chip; 5] = [
    Chip::Camera,
    Chip::Hud,
    Chip::Effects,
    Chip::Take,
    Chip::Back,
];

const MARGIN: f32 = 8.0;
const CHIP_HEIGHT: f32 = 18.0;
const GAP: f32 = 4.0;
const BAND: [f32; 4] = [0.0, 0.0, 0.0, 0.6];
const TEXT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const LINE: f32 = 9.5;
/// A glyph's advance on the overlay grid at [`SCALE`], and the room a chip keeps either side.
const CHAR_WIDTH: f32 = 4.7;
const CHIP_PADDING: f32 = 3.0;
const SCALE: f32 = 0.75;

/// Where a chip sits on the overlay grid `grid` (width, height): `[x, y, w, h]`.
#[must_use]
pub fn chip_rect(chip: Chip, grid: (f32, f32)) -> [f32; 4] {
    let index = CHIPS.iter().position(|&c| c == chip).unwrap_or(0);
    let width = (grid.0 - 2.0 * MARGIN - GAP * (CHIPS.len() - 1) as f32) / CHIPS.len() as f32;
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

const PANEL_WIDTH: f32 = 176.0;
const ROW_HEIGHT: f32 = 14.0;
const ARROW: f32 = 14.0;
const VALUE: f32 = 48.0;
const SELECTED: [f32; 4] = [1.0, 1.0, 1.0, 0.22];

/// Where the effects panel's row `index` sits on the overlay grid: `[x, y, w, h]`. The panel
/// hangs under the chips at the right edge.
#[must_use]
pub fn row_rect(index: usize, grid: (f32, f32)) -> [f32; 4] {
    [
        grid.0 - MARGIN - PANEL_WIDTH,
        MARGIN + CHIP_HEIGHT + GAP + ROW_HEIGHT * index as f32,
        PANEL_WIDTH,
        ROW_HEIGHT,
    ]
}

/// What a click on the panel asks for: the row, and `0` to put the cursor on it, `-1` or `1`
/// to change it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowHit {
    pub row: Effect,
    pub step: i32,
}

/// The panel's row (and arrow) under a point on the grid, if the panel is up.
#[must_use]
pub fn row_at(photo: &Photo, grid: (f32, f32), at: (f32, f32)) -> Option<RowHit> {
    if !photo.panel() || !photo.help() {
        return None;
    }
    photo
        .effects()
        .rows()
        .iter()
        .enumerate()
        .find_map(|(index, &row)| {
            let [x, y, w, h] = row_rect(index, grid);
            if at.0 < x || at.0 >= x + w || at.1 < y || at.1 >= y + h {
                return None;
            }
            let from_right = x + w - at.0;
            let step = if from_right <= ARROW {
                1
            } else if from_right >= ARROW + VALUE && from_right < 2.0 * ARROW + VALUE {
                -1
            } else {
                0
            };
            Some(RowHit { row, step })
        })
}

fn label_of(row: Effect) -> &'static str {
    match row {
        Effect::Exposure => strings::EXPOSURE,
        Effect::Saturation => strings::SATURATION,
        Effect::SpeedBlur => strings::SPEED_BLUR,
        Effect::DepthOfField => strings::DEPTH,
        Effect::Aperture => strings::APERTURE,
        Effect::Focus => strings::FOCUS,
    }
}

/// The effects panel: one line a row, the cursor's row lit, an arrow either side of the value.
fn panel_list(photo: &Photo, grid: (f32, f32), say: &dyn Fn(&str) -> String) -> Vec<Draw> {
    let effects = photo.effects();
    let rows = effects.rows();
    let first = row_rect(0, grid);
    let mut list = vec![Draw::Fill {
        rect: [
            first[0] - 2.0,
            first[1] - 2.0,
            PANEL_WIDTH + 4.0,
            ROW_HEIGHT * rows.len() as f32 + 4.0,
        ],
        color: BAND,
    }];
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
    for (index, &row) in rows.iter().enumerate() {
        let [x, y, w, h] = row_rect(index, grid);
        if row == effects.selected() {
            list.push(Draw::Fill {
                rect: [x, y, w, h],
                color: SELECTED,
            });
        }
        let value = match row {
            Effect::DepthOfField => say(if effects.value(row) == 1 {
                strings::ON
            } else {
                strings::OFF
            }),
            _ => format!("{}%", effects.value(row)),
        };
        list.push(text(x + 4.0, y + 2.0, say(label_of(row)), Align::Left));
        list.push(text(
            x + w - ARROW - VALUE / 2.0 - ARROW / 2.0 + 2.0,
            y + 2.0,
            value,
            Align::Centre,
        ));
        list.push(text(
            x + w - 2.0 * ARROW - VALUE + 3.0,
            y + 2.0,
            "<".into(),
            Align::Left,
        ));
        list.push(text(x + w - ARROW + 3.0, y + 2.0, ">".into(), Align::Left));
    }
    list
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
        scale: SCALE,
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
    // A chip says its row and its value; a row whose word is too long for the chip (Pulse's
    // own "PHOTO CAMERA MODE") gives the value alone.
    let fitted = |label: String, value: String, width: f32| {
        let both = format!("{label}: {value}");
        if both.chars().count() as f32 * CHAR_WIDTH + 2.0 * CHIP_PADDING <= width {
            both
        } else {
            value
        }
    };
    let width = chip_rect(Chip::Camera, grid)[2];
    let chip = |chip: Chip| match chip {
        Chip::Camera => fitted(say(strings::CAMERA), say(lens), width),
        Chip::Hud => fitted(
            say(strings::HUD),
            say(if photo.hud() {
                strings::ON
            } else {
                strings::OFF
            }),
            width,
        ),
        Chip::Effects => say(strings::EFFECTS),
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
    if photo.panel() {
        list.extend(panel_list(photo, grid, say));
    }
    let button = words.button;
    let mut lines = vec![
        format!(
            "{} {}    {} {}",
            button(Button::Cross),
            say(strings::TAKE),
            button(Button::Circle),
            say(strings::BACK),
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
            "{} {}    {} / {} {}",
            button(Button::Start),
            say(strings::HELP),
            button(Button::L),
            button(Button::R),
            say(strings::SHIP),
        ),
    ];
    if photo.panel() {
        lines.push(say(strings::PANEL_KEYS));
    } else {
        lines.extend(lens_lines(photo, say, button));
    }
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

/// The lines of what the stick does on the lens that is up.
fn lens_lines(
    photo: &Photo,
    say: &dyn Fn(&str) -> String,
    button: &dyn Fn(Button) -> String,
) -> Vec<String> {
    let stick = say(strings::STICK);
    match photo.lens() {
        Lens::Internal => Vec::new(),
        Lens::Close => vec![
            format!(
                "{stick} {}    {} + {stick} {}",
                say(strings::ROTATE),
                button(Button::Select),
                say(strings::ZOOM_TILT),
            ),
            format!("{} + {stick} {}", button(Button::Square), say(strings::PAN)),
        ],
        Lens::Track => vec![format!("{stick} {}", say(strings::ZOOM_TILT))],
    }
}

#[cfg(test)]
mod tests;
