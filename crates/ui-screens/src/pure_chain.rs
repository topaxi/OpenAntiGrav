//! One screen of Wipeout Pure's SINGLE PLAYER chain: a title, a context line and
//! a list of one-choice rows.
//!
//! **The chain is the disc's, and so is the look.** Pure walks `Single Player`
//! (mode), `Class Selection`, `League Selection` or `Tournament Selection`,
//! `Track Selection` or `Zone Track Selection`, then `Team Selection`
//! (`oag_tables::pure_flow` reads the order off the screens' own redirects).
//! Each screen before the team is a title and a menu of names the code fills, drawn
//! as Pure's own menu pages are: the title at the skin's title position, the rows
//! at the skin's menu origin and pitch, the selected one in the skin's selected
//! ink, on the skin's background. A PPSSPP capture of a fresh profile
//! (2026-10-12) shows the original **hides** what the profile has not opened
//! instead of drawing it locked, and this does the same. **Chosen, not
//! measured:** the medal word drawn at a row's right edge and its inks, and the
//! context line (what has been chosen so far) between the title and the first row; the original
//! also draws a help line, a bottom bar and per-screen art that are not drawn
//! here.
//!
//! The model knows nothing of tables, medals' meaning or races. It is rows in,
//! [`Event`]s out, the same shape [`crate::prompt::Confirm`] is, and the
//! composition root owns what a confirmed row does.

use oag_gameplay::input::{Button, Input};
use oag_tables::race_campaign::Medal;
use oag_ui::frontend::{Align, Draw};
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::pointer::{Pointer, row_at};

/// What a row holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Open, no medal yet.
    Open,
    /// The best medal this event holds.
    Medal(Medal),
}

/// One line of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// What the line says.
    pub label: String,
    /// Open, locked or medalled.
    pub status: Status,
    /// The medal word drawn at the right edge, already in the player's
    /// language: empty for an event with no medal.
    pub status_text: String,
    /// The caller's own index for this row, handed back on confirm.
    pub tag: usize,
    /// The disc's description of this row (`Text->SCREEN->ITEM->VALUE` in its
    /// string table), typed along the screen's bottom while the row is
    /// selected. Empty where the disc authors none.
    pub description: String,
}

impl Row {
    /// A choice.
    #[must_use]
    pub fn event(
        label: impl Into<String>,
        tag: usize,
        status: Status,
        status_text: String,
    ) -> Self {
        Self {
            label: label.into(),
            status,
            status_text,
            tag,
            description: String::new(),
        }
    }

    /// This row with `description` as its bottom line.
    #[must_use]
    pub fn described(mut self, description: String) -> Self {
        self.description = description;
        self
    }
}

/// What a tick of input did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The cursor moved.
    Moved,
    /// Confirm on the row whose [`Row::tag`] this is. A locked row confirms too:
    /// refusing it is the caller's, so a pad and a pointer cannot disagree.
    Confirmed(usize),
    /// Back out of the screen.
    Back,
}

/// The screen, and where its cursor and window are.
#[derive(Debug, Clone)]
pub struct Choice {
    title: String,
    /// The distance between two rows in the PSP's 272-line grid.
    pitch: f32,
    context: String,
    rows: Vec<Row>,
    selected: usize,
    scroll: usize,
}

/// The grid height a [`Choice::new`] pitch is measured in.
const PSP_LINES: f32 = 272.0;

const GOLD: [f32; 4] = [0.96, 0.66, 0.0, 1.0];
const SILVER: [f32; 4] = [0.55, 0.58, 0.66, 1.0];
const BRONZE: [f32; 4] = [0.78, 0.42, 0.18, 1.0];

/// The ink a medal word is drawn in.
#[must_use]
pub fn medal_ink(medal: Medal) -> [f32; 4] {
    match medal {
        Medal::Gold => GOLD,
        Medal::Silver => SILVER,
        Medal::Bronze => BRONZE,
    }
}

/// Where the screen is laid out, in the skin's grid.
struct Layout {
    /// The rows' left edge and the right edge medal words align to.
    text_x: f32,
    right_x: f32,
    /// A row's pitch and scale.
    line: f32,
    scale: f32,
    /// The first row.
    rows_y: f32,
    /// How many rows fit above the bottom margin.
    visible: usize,
    /// The screen's width, which a row's click target spans.
    width: f32,
}

impl Layout {
    fn new(skin: &Skin, pitch: f32) -> Self {
        let (width, height) = skin.space().size;
        let line = pitch * height / PSP_LINES;
        let rows_y = skin.first_row_y();
        let text_x = skin.menu_x();
        let room = (height - rows_y - line * 2.0).max(line);
        Self {
            text_x,
            right_x: width - text_x,
            line,
            scale: skin.row_scale(),
            rows_y,
            visible: ((room / line).floor() as usize).max(1),
            width,
        }
    }
}

impl Choice {
    /// A screen on `rows`, with the cursor on the first row.
    #[must_use]
    pub fn new(title: String, context: String, pitch: f32, rows: Vec<Row>) -> Self {
        Self {
            title,
            pitch,
            context,
            rows,
            selected: 0,
            scroll: 0,
        }
    }

    /// The same screen with other rows, or after a medal changed a gate. The
    /// cursor stays on the row with the same tag when there still is one.
    pub fn replace(&mut self, rows: Vec<Row>) {
        let tag = self.selected_row().map(|row| row.tag);
        self.rows = rows;
        self.selected = tag
            .and_then(|tag| self.rows.iter().position(|r| r.tag == tag))
            .unwrap_or(0);
    }

    /// Puts the cursor on the event row with this tag, if there is one.
    pub fn select_tag(&mut self, tag: usize) {
        if let Some(index) = self.rows.iter().position(|r| r.tag == tag) {
            self.selected = index;
        }
    }

    /// The row under the cursor.
    #[must_use]
    pub fn selected_row(&self) -> Option<&Row> {
        self.rows.get(self.selected)
    }

    /// Every row.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The context line.
    #[must_use]
    pub fn context(&self) -> &str {
        &self.context
    }

    fn move_by(&mut self, direction: i32) -> bool {
        let count = self.rows.len();
        if count == 0 {
            return false;
        }
        let to = (self.selected as i64 + i64::from(direction)).rem_euclid(count as i64) as usize;
        let moved = to != self.selected;
        self.selected = to;
        moved
    }

    /// Consumes a tick of input. Every button is taken as an edge, as
    /// [`crate::prompt::Confirm::update`] does.
    pub fn update(&mut self, input: &mut Input) -> Vec<Event> {
        let mut out = Vec::new();
        if input.take(Button::Down) && self.move_by(1) {
            out.push(Event::Moved);
        }
        if input.take(Button::Up) && self.move_by(-1) {
            out.push(Event::Moved);
        }
        if input.take(Button::Circle) {
            out.push(Event::Back);
        }
        if (input.take(Button::Cross) || input.take(Button::Start))
            && let Some(row) = self.selected_row()
        {
            out.push(Event::Confirmed(row.tag));
        }
        out
    }

    /// Consumes a tick of pointer input: hover moves the cursor, a click on a
    /// row is a confirm of it, the wheel scrolls the cursor and the secondary
    /// button backs out. Targets are where [`Self::draw`] puts them.
    pub fn pointer(&mut self, pointer: &Pointer, skin: &Skin) -> Vec<Event> {
        let mut out = Vec::new();
        if pointer.is_idle() {
            return out;
        }
        if pointer.back {
            out.push(Event::Back);
        }
        for _ in 0..pointer.scroll.unsigned_abs() {
            if self.move_by(pointer.scroll.signum()) {
                out.push(Event::Moved);
            }
        }
        let Some(at) = pointer.at else {
            return out;
        };
        let layout = Layout::new(skin, self.pitch);
        let first = self.window_start(&layout);
        let over = row_at(
            at,
            0.0,
            layout.width,
            layout.rows_y,
            layout.line,
            layout.visible.min(self.rows.len().saturating_sub(first)),
        )
        .map(|slot| first + slot)
        .filter(|&index| index < self.rows.len());
        if let Some(index) = over {
            if (pointer.moved || pointer.clicked) && index != self.selected {
                self.selected = index;
                out.push(Event::Moved);
            }
            if pointer.clicked {
                out.push(Event::Confirmed(self.rows[index].tag));
            }
        }
        out
    }

    /// The first row of the visible window: the scroll offset moved the least
    /// that keeps the cursor inside it.
    fn window_start(&self, layout: &Layout) -> usize {
        let visible = layout.visible;
        let mut start = self.scroll.min(self.rows.len().saturating_sub(visible));
        if self.selected < start {
            start = self.selected;
        } else if self.selected >= start + visible {
            start = self.selected + 1 - visible;
        }
        start
    }

    /// Advances the window to follow the cursor. Call once a frame with the
    /// skin the list is drawn with; [`Self::draw`] needs it to be current.
    pub fn follow(&mut self, skin: &Skin) {
        self.scroll = self.window_start(&Layout::new(skin, self.pitch));
    }

    /// The rows and the context line, drawn on the skin's grid.
    #[must_use]
    pub fn body(&self, skin: &Skin) -> Vec<Draw> {
        let layout = Layout::new(skin, self.pitch);
        let text =
            |x: f32, y: f32, scale: f32, color: [f32; 4], align: Align, text: String| Draw::Text {
                x,
                y,
                scale,
                color,
                border: None,
                align,
                text,
                wrap_width: None,
            };
        let mut out = Vec::new();
        if !self.context.is_empty() {
            out.push(text(
                layout.text_x,
                layout.rows_y - layout.line * 0.5,
                layout.scale * 0.5,
                skin.normal(),
                Align::Left,
                self.context.clone(),
            ));
        }
        let first = self.window_start(&layout);
        for (slot, row) in self
            .rows
            .iter()
            .enumerate()
            .skip(first)
            .take(layout.visible)
            .map(|(index, row)| (index - first, row))
        {
            let y = layout.rows_y + slot as f32 * layout.line;
            let ink = if first + slot == self.selected {
                skin.selected()
            } else {
                skin.normal()
            };
            out.push(text(
                layout.text_x,
                y,
                layout.scale,
                ink,
                Align::Left,
                row.label.clone(),
            ));
            if !row.status_text.is_empty() {
                let status_ink = match row.status {
                    Status::Medal(medal) => medal_ink(medal),
                    Status::Open => ink,
                };
                out.push(text(
                    layout.right_x,
                    y,
                    layout.scale,
                    status_ink,
                    Align::Right,
                    row.status_text.clone(),
                ));
            }
        }
        out
    }
}

/// The whole screen in paint order: the title's frame (its background and marks),
/// the title, then [`Choice::body`]. The shape `crate::picker::draw_list` gives
/// the race box's screens, so the two read as one front end.
#[must_use]
pub fn draw_list(
    choice: &Choice,
    skin: &Skin,
    frame: &Frame,
    backdrop: Option<Picture>,
    race_behind: bool,
) -> Layers {
    let (title_x, title_y, title_scale) = skin.title_at();
    let mut layers = Layers {
        backdrop: frame.backdrops(
            skin.space(),
            skin.background(),
            backdrop.map(Picture::draw),
            race_behind,
        ),
        ..Layers::default()
    };
    layers.chrome.push(Draw::title(
        skin.title_font(),
        title_x,
        title_y,
        title_scale,
        skin.title_color(frame.ink),
        choice.title.clone(),
    ));
    layers.body = choice.body(skin);
    layers
}

#[cfg(test)]
mod tests;
