//! Photo mode's screen and its files: what is drawn over the picture, where a shot goes,
//! and when the mode is on offer.
//!
//! The camera is `oag_raceplay::photo`; this is the half that needs a screen and a disc.
//! The mode is the pause plus a camera: nothing here ticks the race (a rewind does, in the
//! session, and puts it back), and the picture it saves is the frame the player sees, at the
//! size they see it, without the screen's own text.
//!
//! **Chosen, not measured:** the screen's layout and the file name.

use std::path::{Path, PathBuf};

use oag_gameplay::input::Button;
use oag_raceplay::photo::effects::Row as Effect;
use oag_raceplay::photo::{Lens, Photo};
use oag_ui::frontend::{Align, Draw};
use oag_ui::hints::Entry;

pub(crate) mod average;
pub mod touch_layout;

/// The directory under the user's data directory shots are written to.
pub const DIRECTORY: &str = "photos";

/// Whether this build can write a shot: a desktop and a phone can (a phone writes into the
/// app's own files directory, the tree a player already copies disc images into); a page has no
/// file system to write to.
pub const WRITES: bool = writes_on(cfg!(target_arch = "wasm32"));

/// Whether the player can browse to where a shot lands: a desktop can; a phone's app directory
/// is hidden from its file manager on recent Android, until an export puts shots in the gallery.
pub const REACHABLE: bool = WRITES && !oag_mobile::MOBILE;

/// [`WRITES`] for a platform: only a page cannot write.
#[must_use]
pub const fn writes_on(wasm: bool) -> bool {
    !wasm
}

/// Whether photo mode is on offer for a race drawn as `views` views. A split screen has no
/// single picture to frame and no single pad to drive it.
#[must_use]
pub fn offered(views: usize) -> bool {
    WRITES && views <= 1
}

/// Whether the race ticks in the frame loop: not while photo mode is open. The rule the frame
/// loop asks, in one place so the test of the hash and the loop cannot disagree. A rewind moves
/// the race outside that loop and restores it on leaving.
#[must_use]
pub fn ticks(open: bool) -> bool {
    !open
}

/// The word to say once the shot at `path` is written. On a phone the file is handed to the
/// gallery (`oag_mobile::export`) and, once the gallery has it, **the copy in the files
/// directory is removed**: that directory is hidden from the phone's file manager, so a second
/// copy there is one the player could never delete. Where the export is not possible (an
/// Android older than 10, or a failure) the shot stays where it was written and the plain word
/// is used.
pub fn saved_word(path: &Path) -> &'static str {
    if !oag_mobile::export::available() {
        return strings::SAVED;
    }
    settle(
        path,
        oag_mobile::export::export(path, oag_mobile::export::Kind::Picture),
    )
}

/// What becomes of the file at `path` after its export came to `outcome`, and the word for it.
fn settle(path: &Path, outcome: Result<String, oag_mobile::export::Error>) -> &'static str {
    match outcome {
        Ok(uri) => {
            log::debug!("photo mode: {} is in the gallery as {uri}", path.display());
            if let Err(e) = std::fs::remove_file(path) {
                log::warn!("photo mode: cannot remove {}: {e}", path.display());
            }
            strings::GALLERY
        }
        Err(oag_mobile::export::Error::Unsupported(why)) => {
            log::debug!("photo mode: not exported: {why}");
            strings::SAVED
        }
        Err(e) => {
            log::warn!("photo mode: cannot export {}: {e}", path.display());
            strings::SAVED
        }
    }
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
    pub const FREE: &str = "OAG_PHOTO_FREE";
    pub const WALK: &str = "OAG_PHOTO_WALK";
    pub const SLIDE_LIFT: &str = "OAG_PHOTO_SLIDE_LIFT";
    pub const ZOOM_ROLL: &str = "OAG_PHOTO_ZOOM_ROLL";
    pub const FREE_KEYS: &str = "OAG_PHOTO_FREE_KEYS";
    pub const DRAG: &str = "OAG_PHOTO_DRAG";
    pub const LOOK: &str = "OAG_PHOTO_LOOK";
    pub const MOVE: &str = "OAG_PHOTO_MOVE";
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
    pub const PAD_KEYS: &str = "OAG_PHOTO_PAD_KEYS";
    pub const SCRUB_KEYS: &str = "OAG_PHOTO_SCRUB_KEYS";
    pub const REWIND: &str = "OAG_PHOTO_REWIND";
    pub const PLAY: &str = "OAG_PHOTO_PLAY";
    pub const MOTION_BLUR: &str = "OAG_PHOTO_MOTION_BLUR";
    pub const SHUTTER: &str = "OAG_PHOTO_SHUTTER";
    pub const PROGRESS: &str = "OAG_PHOTO_PROGRESS";
    pub const CANCELLED: &str = "OAG_PHOTO_CANCELLED";
    pub const ANY_INPUT: &str = "OAG_PHOTO_ANY_INPUT";
    pub const HELP: &str = "OAG_PHOTO_HELP";
    pub const SAVED: &str = "OAG_PHOTO_SAVED";
    pub const GALLERY: &str = "OAG_PHOTO_GALLERY";
    pub const FAILED: &str = "OAG_PHOTO_FAILED";
    pub const TOUCH_HINT: &str = "OAG_PHOTO_TOUCH_HINT";
    pub const TOUCH_HINT_FREE: &str = "OAG_PHOTO_TOUCH_HINT_FREE";
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
    /// What a drag does on the free lens: look, or move. Drawn only on that lens.
    Drag,
    /// Put the camera back (touch layout only).
    Reset,
    /// The previous craft (touch layout only).
    Previous,
    /// The next craft (touch layout only).
    Next,
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

/// Where the drag chip sits: under the camera chip, the width of a chip.
#[must_use]
pub fn drag_rect(grid: (f32, f32)) -> [f32; 4] {
    let [x, y, w, h] = chip_rect(Chip::Camera, grid);
    [x, y + h + GAP, w, h]
}

/// The chip under a point on the grid, if any. `free` is whether the free lens is up, which is
/// the only lens with a drag chip.
#[must_use]
pub fn chip_at(grid: (f32, f32), at: (f32, f32), free: bool) -> Option<Chip> {
    chip_at_with(grid, at, free, false)
}

/// [`chip_at`] under the touch layout when `touch`.
#[must_use]
pub fn chip_at_with(grid: (f32, f32), at: (f32, f32), free: bool, touch: bool) -> Option<Chip> {
    if touch {
        return touch_layout::chip_at(grid, at, free);
    }
    let inside = |[x, y, w, h]: [f32; 4]| at.0 >= x && at.0 < x + w && at.1 >= y && at.1 < y + h;
    CHIPS
        .into_iter()
        .find(|&chip| inside(chip_rect(chip, grid)))
        .or_else(|| (free && inside(drag_rect(grid))).then_some(Chip::Drag))
}

const PANEL_WIDTH: f32 = 176.0;
const ROW_HEIGHT: f32 = 14.0;
const ARROW: f32 = 14.0;
const VALUE: f32 = 48.0;
const SELECTED: [f32; 4] = [1.0, 1.0, 1.0, 0.22];

/// Where the effects panel sits and how big its parts are: the desktop's, or the touch layout's.
#[derive(Debug, Clone, Copy)]
struct Geom {
    top: f32,
    right: f32,
    width: f32,
    row: f32,
    arrow: f32,
}

impl Geom {
    fn of(touch: bool, grid: (f32, f32)) -> Self {
        if touch {
            touch_layout::panel(grid)
        } else {
            Self {
                top: MARGIN + CHIP_HEIGHT + GAP,
                right: MARGIN,
                width: PANEL_WIDTH,
                row: ROW_HEIGHT,
                arrow: ARROW,
            }
        }
    }

    fn rect(self, index: usize, grid: (f32, f32)) -> [f32; 4] {
        [
            grid.0 - self.right - self.width,
            self.top + self.row * index as f32,
            self.width,
            self.row,
        ]
    }
}

/// Where the effects panel's row `index` sits on the overlay grid: `[x, y, w, h]`. The panel
/// hangs under the chips at the right edge.
#[must_use]
pub fn row_rect(index: usize, grid: (f32, f32)) -> [f32; 4] {
    Geom::of(false, grid).rect(index, grid)
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
    row_at_with(photo, grid, at, false)
}

/// [`row_at`] under the touch layout when `touch`.
#[must_use]
pub fn row_at_with(photo: &Photo, grid: (f32, f32), at: (f32, f32), touch: bool) -> Option<RowHit> {
    let geom = Geom::of(touch, grid);
    if !photo.panel() || !photo.help() {
        return None;
    }
    photo
        .effects()
        .rows()
        .iter()
        .enumerate()
        .find_map(|(index, &row)| {
            let [x, y, w, h] = geom.rect(index, grid);
            if at.0 < x || at.0 >= x + w || at.1 < y || at.1 >= y + h {
                return None;
            }
            let from_right = x + w - at.0;
            let step = if from_right <= geom.arrow {
                1
            } else if (geom.arrow + VALUE..2.0 * geom.arrow + VALUE).contains(&from_right) {
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
        Effect::Rewind => strings::REWIND,
        Effect::Play => strings::PLAY,
        Effect::MotionBlur => strings::MOTION_BLUR,
        Effect::Shutter => strings::SHUTTER,
    }
}

/// The race's clock at `tick`, `M:SS.s`: what the rewind row shows.
#[must_use]
pub fn clock(tick: u64) -> String {
    let tenths = tick * 10 / 60;
    format!("{}:{:02}.{}", tenths / 600, tenths / 10 % 60, tenths % 10)
}

/// The effects panel: one line a row, the cursor's row lit, an arrow either side of the value.
fn panel_list(
    photo: &Photo,
    grid: (f32, f32),
    say: &dyn Fn(&str) -> String,
    touch: bool,
) -> Vec<Draw> {
    let geom = Geom::of(touch, grid);
    let (arrow, panel_width, row_height) = (geom.arrow, geom.width, geom.row);
    let effects = photo.effects();
    let rows = effects.rows();
    let first = geom.rect(0, grid);
    let mut list = vec![Draw::Fill {
        rect: [
            first[0] - 2.0,
            first[1] - 2.0,
            panel_width + 4.0,
            row_height * rows.len() as f32 + 4.0,
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
        let [x, y, w, h] = geom.rect(index, grid);
        let dy = (h - 14.0) / 2.0;
        if row == effects.selected() {
            list.push(Draw::Fill {
                rect: [x, y, w, h],
                color: SELECTED,
            });
        }
        let value = match row {
            Effect::DepthOfField | Effect::Play => say(if effects.value(row) == 1 {
                strings::ON
            } else {
                strings::OFF
            }),
            Effect::MotionBlur => say(match effects.value(row) {
                1 => strings::TRACK,
                2 => strings::SHIP,
                _ => strings::OFF,
            }),
            Effect::Rewind => clock(effects.target_tick()),
            _ => format!("{}%", effects.value(row)),
        };
        list.push(text(x + 4.0, y + 2.0 + dy, say(label_of(row)), Align::Left));
        list.push(text(
            x + w - arrow - VALUE / 2.0 - arrow / 2.0 + 2.0,
            y + 2.0 + dy,
            value,
            Align::Centre,
        ));
        list.push(text(
            x + w - 2.0 * arrow - VALUE + arrow / 2.0 - 4.0,
            y + 2.0 + dy,
            "<".into(),
            Align::Left,
        ));
        list.push(text(
            x + w - arrow / 2.0 - 2.0,
            y + 2.0 + dy,
            ">".into(),
            Align::Left,
        ));
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
    /// The buttons that leave the mode and, on the face button Back displaced, reset the camera
    /// (`docs/ui/back-and-help.md`).
    pub roles: oag_ui::roles::Roles,
    /// Whether a finger spoke last: the thumb-sized layout is drawn and tapped.
    pub touch: bool,
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
    progress: Option<(u32, u32)>,
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
    if let Some((done, frames)) = progress {
        // A shot under way: the bar and the way out, and nothing else.
        let top = grid.1 * 0.5 - 14.0;
        let width = grid.0 * 0.5;
        let left = (grid.0 - width) / 2.0;
        list.push(Draw::Fill {
            rect: [0.0, top, grid.0, 36.0],
            color: BAND,
        });
        list.push(text(
            grid.0 / 2.0,
            top + 4.0,
            format!("{}  {done} / {frames}", say(strings::PROGRESS)),
            Align::Centre,
        ));
        list.push(Draw::Fill {
            rect: [left, top + 15.0, width, 4.0],
            color: SELECTED,
        });
        list.push(Draw::Fill {
            rect: [
                left,
                top + 15.0,
                width * done as f32 / frames.max(1) as f32,
                4.0,
            ],
            color: TEXT,
        });
        list.push(text(
            grid.0 / 2.0,
            top + 23.0,
            say(strings::ANY_INPUT),
            Align::Centre,
        ));
        return list;
    }
    if !photo.help() {
        return list;
    }
    if words.touch {
        list.extend(touch_layout::chrome(photo, grid, words));
        return list;
    }
    let lens = match photo.lens() {
        Lens::Internal => strings::INTERNAL,
        Lens::Close => strings::CLOSE,
        Lens::Track => strings::TRACK,
        Lens::Free => strings::FREE,
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
        Chip::Reset => say(strings::RESET),
        Chip::Previous | Chip::Next => say(strings::SHIP),
        Chip::Drag => fitted(
            say(strings::DRAG),
            say(if photo.drag_moves() {
                strings::MOVE
            } else {
                strings::LOOK
            }),
            width,
        ),
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
    if photo.lens() == Lens::Free {
        let rect = drag_rect(grid);
        list.push(Draw::Fill { rect, color: BAND });
        list.push(text(
            rect[0] + rect[2] / 2.0,
            rect[1] + 3.0,
            chip(Chip::Drag),
            Align::Centre,
        ));
    }
    if photo.panel() {
        list.extend(panel_list(photo, grid, say, false));
    }
    let lines = legend_lines(photo, words);
    let top = grid.1 - MARGIN - LINE * lines.len() as f32;
    list.push(Draw::Fill {
        rect: [0.0, top - 4.0, grid.0, grid.1 - top + 4.0],
        color: BAND,
    });
    for (i, line) in lines.iter().enumerate() {
        list.push(text(
            MARGIN,
            top + LINE * i as f32,
            oag_ui::hints::line_text(line),
            Align::Left,
        ));
    }
    list
}

/// The key legend along the bottom: each line is entries, and an entry that names one control is
/// a click target ([`legend_hints`]).
fn legend_lines(photo: &Photo, words: &Words) -> Vec<Vec<Entry>> {
    let say = words.say;
    let button = words.button;
    let words_of = |key: Button, id: &str| format!("{} {}", button(key), say(id));
    let mut lines = vec![
        vec![
            Entry::button(Button::Cross, words_of(Button::Cross, strings::TAKE)),
            Entry::button(words.roles.back, words_of(words.roles.back, strings::BACK))
                .after("    "),
        ],
        vec![
            Entry::button(Button::Select, words_of(Button::Select, strings::CAMERA)),
            Entry::button(Button::Square, words_of(Button::Square, strings::HUD)).after("    "),
            Entry::button(words.roles.help, words_of(words.roles.help, strings::RESET))
                .after("    "),
        ],
        vec![
            Entry::button(Button::Start, words_of(Button::Start, strings::HELP)),
            Entry::button(Button::L, button(Button::L)).after("    "),
            Entry::new(None, "/").after(" "),
            Entry::button(
                Button::R,
                format!("{} {}", button(Button::R), say(strings::SHIP)),
            )
            .after(" "),
        ],
    ];
    let plain = |line: String| vec![Entry::new(None, line)];
    if photo.panel() {
        lines.push(plain(say(strings::PANEL_KEYS)));
    } else {
        lines.extend(lens_lines(photo, say, button).into_iter().map(plain));
        match photo.lens() {
            Lens::Internal => {}
            Lens::Free => lines.push(plain(say(strings::FREE_KEYS))),
            Lens::Close | Lens::Track => lines.push(plain(say(strings::PAD_KEYS))),
        }
    }
    if photo.effects().rewinds() {
        lines.push(plain(say(strings::SCRUB_KEYS)));
    }
    lines
}

/// The hints of the key legend on the overlay grid `grid`; none while the help is hidden or a
/// shot is under way, when the legend is not drawn.
#[must_use]
pub fn legend_hints(
    photo: &Photo,
    grid: (f32, f32),
    words: &Words,
    shot_under_way: bool,
) -> Vec<oag_ui::hints::Hint> {
    if shot_under_way || !photo.help() || words.touch {
        return Vec::new();
    }
    let lines = legend_lines(photo, words);
    let top = grid.1 - MARGIN - LINE * lines.len() as f32;
    let placed: Vec<oag_ui::hints::Line> = lines
        .iter()
        .enumerate()
        .map(|(i, entries)| oag_ui::hints::Line {
            entries,
            x: MARGIN,
            y: top + LINE * i as f32,
            align: Align::Left,
            height: LINE,
        })
        .collect();
    oag_ui::hints::legend(oag_ui::font::builtin(), SCALE, &placed)
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
        Lens::Free => vec![
            format!(
                "{stick} {}    {} + {stick} {}",
                say(strings::WALK),
                button(Button::Select),
                say(strings::ZOOM_ROLL),
            ),
            format!(
                "{} + {stick} {}",
                button(Button::Square),
                say(strings::SLIDE_LIFT)
            ),
        ],
    }
}

#[cfg(test)]
mod tests;
