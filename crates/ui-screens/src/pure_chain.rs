//! One screen of Wipeout Pure's SINGLE PLAYER chain: a title, a context line and
//! a list of one-choice rows.
//!
//! **The chain is the disc's, the drawing is ours.** Pure walks `Single Player`
//! (mode), `Class Selection`, `League Selection` or `Tournament Selection`,
//! `Track Selection` or `Zone Track Selection`, then `Team Selection`
//! (`oag_tables::pure_flow` reads the order off the screens' own redirects).
//! Each of the screens before the team is a title and a menu of names the code
//! fills; this is that, with the medal and lock a row holds beside it. The
//! original's own row look on those screens was not captured, so the panel, the
//! cursor and the medal inks are chosen, not measured.
//!
//! The model knows nothing of tables, medals' meaning or races. It is rows in,
//! [`Event`]s out, the same shape [`crate::prompt::Confirm`] is, and the
//! composition root owns what a confirmed row does.

use oag_gameplay::input::{Button, Input};
use oag_tables::race_campaign::Medal;
use oag_ui::frontend::{Align, Draw};
use oag_ui::menu::Skin;
use oag_ui::pointer::{Pointer, row_at};

/// What a row holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Open, no medal yet.
    Open,
    /// Gated; the row says what opens it in [`Row::requires`].
    Locked,
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
    /// language: empty for an open event, a lock word for a locked one.
    pub status_text: String,
    /// What opens a locked row, for the footer while it is selected.
    pub requires: String,
    /// The caller's own index for this row, handed back on confirm.
    pub tag: usize,
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
            requires: String::new(),
            tag,
        }
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
    context: String,
    rows: Vec<Row>,
    selected: usize,
    scroll: usize,
}

const SCRIM: [f32; 4] = [0.0, 0.0, 0.0, 0.78];
const PANEL: [f32; 4] = [0.04, 0.07, 0.12, 0.96];
const CURSOR: [f32; 4] = [0.20, 0.42, 0.62, 0.85];
const DIM: f32 = 0.55;
const GOLD: [f32; 4] = [1.0, 0.80, 0.20, 1.0];
const SILVER: [f32; 4] = [0.82, 0.84, 0.88, 1.0];
const BRONZE: [f32; 4] = [0.80, 0.50, 0.25, 1.0];
const NOTICE: [f32; 4] = [1.0, 0.76, 0.25, 1.0];

/// How much smaller than a menu row the list's rows are drawn: a list of 36
/// events at a menu row's pitch shows six at once.
const ROW: f32 = 0.6;

/// The ink a medal word is drawn in.
#[must_use]
pub fn medal_ink(medal: Medal) -> [f32; 4] {
    match medal {
        Medal::Gold => GOLD,
        Medal::Silver => SILVER,
        Medal::Bronze => BRONZE,
    }
}

/// Where the list is laid out, in the skin's grid.
struct Layout {
    panel: [f32; 4],
    text_x: f32,
    right_x: f32,
    /// The heading's own pitch and scale, a menu row's.
    head: f32,
    head_scale: f32,
    line: f32,
    scale: f32,
    /// The context line: the class, league or mode already chosen.
    context_y: f32,
    /// The first row.
    rows_y: f32,
    /// How many rows fit.
    visible: usize,
    /// The footer line.
    footer_y: f32,
}

impl Layout {
    fn new(skin: &Skin) -> Self {
        let (screen_w, screen_h) = skin.space().size;
        let head = skin.row_pitch();
        let line = head * ROW;
        let panel = [
            screen_w * 0.06,
            screen_h * 0.045,
            screen_w * 0.88,
            screen_h * 0.91,
        ];
        let text_x = panel[0] + screen_w * 0.03;
        let right_x = panel[0] + panel[2] - screen_w * 0.03;
        let context_y = panel[1] + head * 1.25;
        let rows_y = context_y + line * 1.5;
        let footer_y = panel[1] + panel[3] - line * 1.4;
        let room = (footer_y - rows_y).max(line);
        Self {
            panel,
            text_x,
            right_x,
            head,
            head_scale: skin.row_scale(),
            line,
            scale: skin.row_scale() * ROW,
            context_y,
            rows_y,
            visible: ((room / line).floor() as usize).max(1),
            footer_y,
        }
    }

    fn row_rect(&self, slot: usize) -> [f32; 4] {
        [
            self.panel[0] + 2.0,
            self.rows_y + slot as f32 * self.line,
            self.panel[2] - 4.0,
            self.line,
        ]
    }
}

impl Choice {
    /// A screen on `rows`, with the cursor on the first row.
    #[must_use]
    pub fn new(title: String, context: String, rows: Vec<Row>) -> Self {
        Self {
            title,
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
        let layout = Layout::new(skin);
        let first = self.window_start(&layout);
        let over = row_at(
            at,
            layout.panel[0],
            layout.panel[2],
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
        self.scroll = self.window_start(&Layout::new(skin));
    }

    /// What the screen looks like, over whatever is already on it.
    #[must_use]
    pub fn draw(&self, skin: &Skin) -> Vec<Draw> {
        let layout = Layout::new(skin);
        let text = |x: f32, y: f32, color: [f32; 4], align: Align, text: String| Draw::Text {
            x,
            y: y + layout.line * 0.15,
            scale: layout.scale,
            color,
            border: None,
            align,
            text,
            wrap_width: None,
        };
        let mut out = vec![
            Draw::Fill {
                rect: [-4000.0, -4000.0, 8000.0, 8000.0],
                color: SCRIM,
            },
            Draw::Fill {
                rect: layout.panel,
                color: PANEL,
            },
            Draw::Text {
                x: layout.text_x,
                y: layout.panel[1] + layout.head * 0.25,
                scale: layout.head_scale,
                color: [1.0, 1.0, 1.0, 1.0],
                border: None,
                align: Align::Left,
                text: self.title.clone(),
                wrap_width: None,
            },
            text(
                layout.text_x,
                layout.context_y,
                skin.normal(),
                Align::Left,
                self.context.clone(),
            ),
        ];
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
            let cursor = first + slot == self.selected;
            if cursor {
                out.push(Draw::Fill {
                    rect: layout.row_rect(slot),
                    color: CURSOR,
                });
            }
            let ink = if cursor {
                skin.selected()
            } else {
                skin.normal()
            };
            let dim = |c: [f32; 4]| [c[0] * DIM, c[1] * DIM, c[2] * DIM, c[3]];
            let (label_ink, status_ink) = match row.status {
                Status::Locked => (dim(ink), dim(ink)),
                Status::Medal(medal) => (ink, medal_ink(medal)),
                Status::Open => (ink, ink),
            };
            let indent = layout.line * 0.6;
            out.push(text(
                layout.text_x + indent,
                y,
                label_ink,
                Align::Left,
                row.label.clone(),
            ));
            if !row.status_text.is_empty() {
                out.push(text(
                    layout.right_x,
                    y,
                    status_ink,
                    Align::Right,
                    row.status_text.clone(),
                ));
            }
        }
        let footer = self
            .selected_row()
            .filter(|row| row.status == Status::Locked && !row.requires.is_empty())
            .map(|row| row.requires.clone());
        if let Some(footer) = footer {
            out.push(text(
                layout.text_x,
                layout.footer_y,
                NOTICE,
                Align::Left,
                footer,
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests;
