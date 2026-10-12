//! Wipeout Pure's campaign screen: one scrolling list of events for the class
//! being played.
//!
//! **The shape is the disc's, the screen is ours.** Pure has no grid. Its front
//! end walks a chain of one-choice screens (class, then league or tournament,
//! then circuit, then team) and what it authors of the campaign is a list of
//! circuits per league, a list of tournaments, and the gates on them
//! (`oag_tables::pure_campaign`). A list that shows those at once, with the medal each
//! holds, is this project's own: no capture of it exists to match.
//!
//! The model knows nothing of tables, medals' meaning or races. It is rows in,
//! [`Event`]s out, the same shape [`crate::prompt::Confirm`] is, and the
//! composition root owns what a confirmed row launches.
//!
//! # Chosen, not measured
//!
//! Everything on screen that is not a string off the disc: the panel and cursor
//! colours, the medal inks (gold, silver and bronze), the row pitch and the
//! class selector's arrows.

use oag_gameplay::input::{Button, Input};
use oag_tables::race_campaign::Medal;
use oag_ui::frontend::{Align, Draw};
use oag_ui::menu::Skin;
use oag_ui::pointer::{Pointer, contains, row_at};

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
    /// A section heading: drawn dim, never selected.
    pub header: bool,
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
    /// A section heading.
    #[must_use]
    pub fn header(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            header: true,
            status: Status::Open,
            status_text: String::new(),
            requires: String::new(),
            tag: usize::MAX,
        }
    }

    /// An event.
    #[must_use]
    pub fn event(
        label: impl Into<String>,
        tag: usize,
        status: Status,
        status_text: String,
    ) -> Self {
        Self {
            label: label.into(),
            header: false,
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
    /// The class selector stepped, by `+1` or `-1`.
    ClassStep(i32),
    /// Back out of the screen.
    Back,
}

/// The screen's text, already resolved.
#[derive(Debug, Clone, Default)]
pub struct Labels {
    /// The heading.
    pub title: String,
    /// The word before the class name.
    pub class: String,
}

/// The list, and where its cursor and window are.
#[derive(Debug, Clone)]
pub struct List {
    labels: Labels,
    class_name: String,
    rows: Vec<Row>,
    selected: usize,
    scroll: usize,
    notice: Option<String>,
}

const SCRIM: [f32; 4] = [0.0, 0.0, 0.0, 0.78];
const PANEL: [f32; 4] = [0.04, 0.07, 0.12, 0.96];
const CURSOR: [f32; 4] = [0.20, 0.42, 0.62, 0.85];
const DIM: f32 = 0.55;
const GOLD: [f32; 4] = [1.0, 0.80, 0.20, 1.0];
const SILVER: [f32; 4] = [0.82, 0.84, 0.88, 1.0];
const BRONZE: [f32; 4] = [0.80, 0.50, 0.25, 1.0];
const NOTICE: [f32; 4] = [1.0, 0.76, 0.25, 1.0];

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
    line: f32,
    scale: f32,
    /// The class selector's line.
    class_y: f32,
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
        let line = skin.row_pitch();
        let panel = [
            screen_w * 0.06,
            screen_h * 0.045,
            screen_w * 0.88,
            screen_h * 0.91,
        ];
        let text_x = panel[0] + screen_w * 0.03;
        let right_x = panel[0] + panel[2] - screen_w * 0.03;
        let class_y = panel[1] + line * 1.35;
        let rows_y = class_y + line * 1.25;
        let footer_y = panel[1] + panel[3] - line * 1.35;
        let room = (footer_y - rows_y).max(line);
        Self {
            panel,
            text_x,
            right_x,
            line,
            scale: skin.row_scale(),
            class_y,
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

    fn class_rect(&self) -> [f32; 4] {
        [
            self.panel[0] + 2.0,
            self.class_y,
            self.panel[2] - 4.0,
            self.line,
        ]
    }
}

impl List {
    /// A list on `rows`, with the cursor on the first selectable one.
    #[must_use]
    pub fn new(labels: Labels, class_name: String, rows: Vec<Row>) -> Self {
        let mut list = Self {
            labels,
            class_name,
            rows,
            selected: 0,
            scroll: 0,
            notice: None,
        };
        list.selected = list.step(0, 1).unwrap_or(0);
        list
    }

    /// The same list for another class, or after a medal changed a gate. The
    /// cursor stays on the row with the same tag when there still is one.
    pub fn replace(&mut self, class_name: String, rows: Vec<Row>) {
        let tag = self.selected_row().map(|row| row.tag);
        self.class_name = class_name;
        self.rows = rows;
        self.selected = tag
            .and_then(|tag| self.rows.iter().position(|r| !r.header && r.tag == tag))
            .or_else(|| self.step(0, 1))
            .unwrap_or(0);
    }

    /// The row under the cursor.
    #[must_use]
    pub fn selected_row(&self) -> Option<&Row> {
        self.rows.get(self.selected).filter(|row| !row.header)
    }

    /// Every row.
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The class name being shown.
    #[must_use]
    pub fn class_name(&self) -> &str {
        &self.class_name
    }

    /// A line drawn in the footer until the next input, or `None`.
    pub fn set_notice(&mut self, notice: Option<String>) {
        self.notice = notice;
    }

    /// The first selectable row at or after `from`, going `direction`
    /// (wrapping) and skipping headings.
    fn step(&self, from: usize, direction: i32) -> Option<usize> {
        let count = self.rows.len() as i64;
        let sign = if direction < 0 { -1 } else { 1 };
        (0..count)
            .map(|offset| (from as i64 + offset * sign).rem_euclid(count) as usize)
            .find(|&index| !self.rows[index].header)
    }

    fn move_by(&mut self, direction: i32) -> bool {
        let count = self.rows.len();
        if count == 0 {
            return false;
        }
        let from = (self.selected as i64 + i64::from(direction)).rem_euclid(count as i64) as usize;
        match self.step(from, direction) {
            Some(index) if index != self.selected => {
                self.selected = index;
                true
            }
            _ => false,
        }
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
        if input.take(Button::Right) {
            out.push(Event::ClassStep(1));
        }
        if input.take(Button::Left) {
            out.push(Event::ClassStep(-1));
        }
        if input.take(Button::Circle) {
            out.push(Event::Back);
        }
        if (input.take(Button::Cross) || input.take(Button::Start))
            && let Some(row) = self.selected_row()
        {
            out.push(Event::Confirmed(row.tag));
        }
        if !out.is_empty() {
            self.notice = None;
        }
        out
    }

    /// Consumes a tick of pointer input: hover moves the cursor, a click on a
    /// row is a confirm of it, a click on the class line steps the class (the
    /// left third back, the rest forward), the wheel scrolls the cursor and the
    /// secondary button backs out. Targets are where [`Self::draw`] puts them.
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
        if pointer.clicked && contains(layout.class_rect(), at) {
            let rect = layout.class_rect();
            out.push(Event::ClassStep(if at.0 < rect[0] + rect[2] / 3.0 {
                -1
            } else {
                1
            }));
            return out;
        }
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
        .filter(|&index| self.rows.get(index).is_some_and(|row| !row.header));
        if let Some(index) = over {
            if (pointer.moved || pointer.clicked) && index != self.selected {
                self.selected = index;
                out.push(Event::Moved);
            }
            if pointer.clicked {
                out.push(Event::Confirmed(self.rows[index].tag));
            }
        }
        if !out.is_empty() {
            self.notice = None;
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
            text(
                layout.text_x,
                layout.panel[1] + layout.line * 0.25,
                [1.0, 1.0, 1.0, 1.0],
                Align::Left,
                self.labels.title.clone(),
            ),
            text(
                layout.text_x,
                layout.class_y,
                skin.normal(),
                Align::Left,
                format!("{}  <  {}  >", self.labels.class, self.class_name),
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
            let cursor = !row.header && first + slot == self.selected;
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
                _ if row.header => (dim(skin.normal()), ink),
                Status::Locked => (dim(ink), dim(ink)),
                Status::Medal(medal) => (ink, medal_ink(medal)),
                Status::Open => (ink, ink),
            };
            let indent = if row.header { 0.0 } else { layout.line * 0.6 };
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
        let footer = self.notice.clone().or_else(|| {
            self.selected_row()
                .filter(|row| row.status == Status::Locked && !row.requires.is_empty())
                .map(|row| row.requires.clone())
        });
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
