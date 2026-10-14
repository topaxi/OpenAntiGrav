//! The seat assignment screen, after the window layout and before the RACE page: which pane of
//! the chosen layout each seated person flies in.
//!
//! **Ours: no disc authors this, and the original has no multi-window mode at all.** Everything
//! here is **chosen, not measured**: the rows, the wording, the swap rule and which press does
//! what. What it decides is what `--window-seats` plus the grid order decide on the command
//! line: the window layout says how many panes each window holds, this says whose they are
//! (`oag_game::split_screen::deal_windows`). The grid order is not touched: a person's craft is
//! still the slot their join order gave them; only the pane that shows it moves.
//!
//! Each person moves their **own** marker (the numbered cell of the preview) with their own
//! device: left or up steps to the pane before, right or down to the pane after (in window
//! order, panes in a window in the order the race cuts them), and confirm says "this is where
//! I sit". A pane holds one person, so moving onto a held pane **swaps** the two, which keeps
//! everybody placed; a person who was moved, by themselves or by a swap, has to confirm again.
//! Confirm from the last person to do so, or from anybody when everybody has, goes on; circle
//! goes back. A fresh screen opens everybody confirmed, on the last assignment where the
//! devices are still seated and the join order elsewhere, so OK is enough.
//!
//! The pointer has no seat of its own: a click on a pane picks up the person in it, and a click
//! on another pane puts them there (swapping), and that click is the confirmation of both. A
//! click on a person's row picks them up the same way.
//!
//! Drawn in the join screen's rows ([`crate::join`]) with the layout screen's preview
//! ([`crate::window_layout`]), the cell of a pane showing the person in it.

use oag_gameplay::input::Button;
use oag_ui::frontend::{Align, Draw};
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::pointer::{Pointer, contains, row_at};

use crate::join::row_blocks;
use crate::window_layout::{CellPaint, Geometry, PreviewWindow, boxes, cell_rects, draw_preview};

/// The words the screen says, already looked up in the player's language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labels {
    /// The screen's title.
    pub title: String,
    /// The line under the title.
    pub hint: String,
    /// "PLAYER", before a person's number.
    pub player: String,
    /// "WINDOW", before a window's number.
    pub window: String,
    /// "VIEW", before a pane's number inside a window that holds several.
    pub view: String,
    /// The row that puts everybody back in join order.
    pub auto: String,
    /// The row that goes on to the race settings.
    pub start: String,
    /// The row that leaves the screen.
    pub back: String,
}

impl Labels {
    /// The words in the language `strings` is, English where an id is missing.
    #[must_use]
    pub fn read(strings: &oag_ui::language::StringTable) -> Self {
        let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
        Self {
            title: say("OAG_SEAT_ASSIGN_TITLE", "WHO SITS WHERE"),
            hint: say(
                "OAG_SEAT_ASSIGN_HINT",
                "MOVE YOUR OWN MARKER TO THE VIEW YOU WANT, THEN CONFIRM",
            ),
            player: say("OAG_JOIN_PLAYER", "PLAYER"),
            window: say("OAG_WINDOW_LAYOUT_WINDOW", "WINDOW"),
            view: say("OAG_SEAT_ASSIGN_VIEW", "VIEW"),
            auto: say("OAG_SEAT_ASSIGN_AUTO", "AUTO"),
            start: say("OAG_JOIN_START", "START"),
            back: say("OAG_MENU_BACK", "BACK"),
        }
    }
}

/// One window of the layout being assigned: only what the panes depend on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pane {
    /// How many panes the window holds.
    pub seats: usize,
    /// Whether two panes are cut side by side rather than one above the other.
    pub vertical: bool,
}

/// One line of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// A person, with the pane they sit in.
    Person(usize),
    /// Everybody back in join order, confirmed.
    Auto,
    /// Go on to the race settings.
    Start,
    /// Leave the screen.
    Back,
}

/// What a tick of input asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The pointer moved to another row, or picked somebody up.
    Moved,
    /// A marker moved, or somebody confirmed.
    Changed,
    /// START was asked for while somebody has not confirmed.
    Refused,
    /// Everybody is placed and confirmed: go on.
    Start,
    /// BACK, or circle, or the secondary button.
    Back,
}

/// The screen: who sits in which pane, who has confirmed, and what the pointer holds.
#[derive(Debug, Clone)]
pub struct SeatAssign {
    labels: Labels,
    windows: Vec<Pane>,
    /// The flat pane (panes counted across the windows from 0) each person sits in.
    pane_of: Vec<usize>,
    /// Whether each person has confirmed where they sit.
    ready: Vec<bool>,
    /// The row the pointer is over.
    hover: Option<Row>,
    /// The person the pointer picked up.
    held: Option<usize>,
}

impl SeatAssign {
    /// A screen for the people of `windows`' panes, one pane each, everybody confirmed.
    /// `pane_people` is who sits in each pane (a person per flat pane) as last time: used
    /// when it is a permutation of the people, else the join order (person `i` in pane `i`).
    #[must_use]
    pub fn new(labels: Labels, windows: Vec<Pane>, pane_people: Option<&[usize]>) -> Self {
        let people: usize = windows.iter().map(|window| window.seats).sum();
        let mut pane_of: Vec<usize> = (0..people).collect();
        if let Some(order) = pane_people.filter(|order| is_permutation(order, people)) {
            for (pane, person) in order.iter().enumerate() {
                pane_of[*person] = pane;
            }
        }
        Self {
            labels,
            windows,
            pane_of,
            ready: vec![true; people],
            hover: None,
            held: None,
        }
    }

    /// How many people are placed.
    #[must_use]
    pub fn people(&self) -> usize {
        self.pane_of.len()
    }

    /// The windows being assigned.
    #[must_use]
    pub fn windows(&self) -> &[Pane] {
        &self.windows
    }

    /// The pane (from 0, across the windows) person `person` sits in.
    #[must_use]
    pub fn pane_of(&self, person: usize) -> usize {
        self.pane_of[person]
    }

    /// Who sits in each pane, in pane order: the assignment the race deals by.
    #[must_use]
    pub fn pane_people(&self) -> Vec<usize> {
        let mut out = vec![0; self.people()];
        for (person, pane) in self.pane_of.iter().enumerate() {
            out[*pane] = person;
        }
        out
    }

    /// Whether person `person` has confirmed.
    #[must_use]
    pub fn is_ready(&self, person: usize) -> bool {
        self.ready[person]
    }

    /// Whether everybody has.
    #[must_use]
    pub fn can_start(&self) -> bool {
        self.ready.iter().all(|ready| *ready)
    }

    /// The person the pointer has picked up.
    #[must_use]
    pub fn held(&self) -> Option<usize> {
        self.held
    }

    /// The row the pointer is over.
    #[must_use]
    pub fn hover(&self) -> Option<Row> {
        self.hover
    }

    /// The rows, top to bottom: every person, AUTO, START, BACK.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        let mut rows: Vec<Row> = (0..self.people()).map(Row::Person).collect();
        rows.extend([Row::Auto, Row::Start, Row::Back]);
        rows
    }

    /// The window flat pane `pane` is in, and the pane's place inside it (from 0).
    fn window_of(&self, pane: usize) -> (usize, usize) {
        let mut first = 0;
        for (index, window) in self.windows.iter().enumerate() {
            if pane < first + window.seats {
                return (index, pane - first);
            }
            first += window.seats;
        }
        (self.windows.len().saturating_sub(1), 0)
    }

    /// Puts `person` in `pane`, the person who was there taking `person`'s old one. Both have
    /// to confirm again unless `confirmed`. Returns whether anything moved.
    fn put(&mut self, person: usize, pane: usize, confirmed: bool) -> bool {
        let from = self.pane_of[person];
        if pane >= self.people() || pane == from {
            return false;
        }
        if let Some(other) = self.pane_of.iter().position(|held| *held == pane) {
            self.pane_of[other] = from;
            self.ready[other] = confirmed;
        }
        self.pane_of[person] = pane;
        self.ready[person] = confirmed;
        true
    }

    /// Everybody back in join order, confirmed.
    fn reset(&mut self) -> bool {
        let changed = self
            .pane_of
            .iter()
            .enumerate()
            .any(|(person, pane)| person != *pane)
            || !self.can_start();
        for (person, pane) in self.pane_of.iter_mut().enumerate() {
            *pane = person;
        }
        self.ready.fill(true);
        changed
    }

    /// Consumes one press of `buttons` (a [`Button::bit`] mask, newly down) from the device of
    /// person `person`.
    pub fn press(&mut self, person: usize, buttons: u32) -> Vec<Event> {
        let mut out = Vec::new();
        if person >= self.people() {
            return out;
        }
        let has = |button: Button| buttons & button.bit() != 0;
        let count = self.people();
        let step = |delta: usize| (self.pane_of[person] + delta) % count;
        if has(Button::Left) || has(Button::Up) {
            let to = step(count - 1);
            if self.put(person, to, false) {
                out.push(Event::Changed);
            }
        } else if has(Button::Right) || has(Button::Down) {
            let to = step(1);
            if self.put(person, to, false) {
                out.push(Event::Changed);
            }
        }
        if has(Button::Circle) {
            out.push(Event::Back);
        }
        if has(Button::Cross) || has(Button::Start) {
            if !self.ready[person] {
                self.ready[person] = true;
                out.push(Event::Changed);
            }
            out.push(if self.can_start() {
                Event::Start
            } else {
                Event::Refused
            });
        }
        out
    }

    /// Consumes a tick of pointer input. `cells` cuts a window's panes as the race does. The
    /// secondary button backs out.
    pub fn pointer(
        &mut self,
        pointer: &Pointer,
        skin: &Skin,
        cells: &dyn Fn(usize, bool) -> Vec<[f32; 4]>,
    ) -> Vec<Event> {
        let mut out = Vec::new();
        if pointer.is_idle() {
            return out;
        }
        if pointer.back {
            out.push(Event::Back);
        }
        let Some(at) = pointer.at else {
            return out;
        };
        let geometry = Geometry::new(skin);
        let rows = self.rows();
        let over = row_at(
            at,
            0.0,
            geometry.column,
            geometry.layout.top,
            geometry.layout.line,
            rows.len(),
        )
        .map(|index| rows[index]);
        if (pointer.moved || pointer.clicked) && self.hover != over {
            self.hover = over;
            out.push(Event::Moved);
        }
        if !pointer.clicked {
            return out;
        }
        match over {
            Some(Row::Person(person)) => out.extend(self.pick(person)),
            Some(Row::Auto) => {
                self.held = None;
                if self.reset() {
                    out.push(Event::Changed);
                }
            }
            Some(Row::Start) => out.push(if self.can_start() {
                Event::Start
            } else {
                Event::Refused
            }),
            Some(Row::Back) => out.push(Event::Back),
            None => {
                if let Some(pane) = self.pane_at(at, skin, cells) {
                    out.extend(self.click_pane(pane));
                }
            }
        }
        out
    }

    /// The flat pane under `at`, if the point is over a cell of the preview.
    fn pane_at(
        &self,
        at: (f32, f32),
        skin: &Skin,
        cells: &dyn Fn(usize, bool) -> Vec<[f32; 4]>,
    ) -> Option<usize> {
        let geometry = Geometry::new(skin);
        let rects = boxes(self.windows.len(), false, geometry.preview);
        let mut first = 0;
        for (window, rect) in self.windows.iter().zip(rects) {
            for (offset, cell) in cell_rects(rect, window.seats, window.vertical, cells)
                .into_iter()
                .enumerate()
            {
                if contains(cell, at) {
                    return Some(first + offset);
                }
            }
            first += window.seats;
        }
        None
    }

    /// The pointer picked up `person`: a second pick of the same person puts them down.
    fn pick(&mut self, person: usize) -> Option<Event> {
        self.held = if self.held == Some(person) {
            None
        } else {
            Some(person)
        };
        Some(Event::Moved)
    }

    /// A click on `pane`: picks up its person, or puts the person held there.
    fn click_pane(&mut self, pane: usize) -> Option<Event> {
        let Some(held) = self.held else {
            let person = self.pane_of.iter().position(|held| *held == pane)?;
            return self.pick(person);
        };
        self.held = None;
        if self.pane_of[held] == pane {
            return Some(Event::Moved);
        }
        self.put(held, pane, true).then_some(Event::Changed)
    }

    /// What a person's row says about where they sit.
    fn place(&self, person: usize) -> String {
        let (window, inside) = self.window_of(self.pane_of[person]);
        let seats = self.windows[window].seats;
        if seats > 1 {
            format!(
                "{} {} {} {}",
                self.labels.window,
                window + 1,
                self.labels.view,
                inside + 1
            )
        } else {
            format!("{} {}", self.labels.window, window + 1)
        }
    }

    /// The boxes behind the rows on a title that draws them (HD, Omega); empty elsewhere.
    #[must_use]
    pub fn blocks(&self, skin: &Skin, frame: &Frame) -> Vec<Draw> {
        let shape: Vec<(bool, bool)> = self
            .rows()
            .iter()
            .map(|row| (self.is_lit(*row), matches!(row, Row::Person(_))))
            .collect();
        row_blocks(skin, frame, &shape)
    }

    /// Whether `row` is drawn in the selected ink: the one hovered, or the person held.
    fn is_lit(&self, row: Row) -> bool {
        self.hover == Some(row) || matches!(row, Row::Person(person) if self.held == Some(person))
    }

    /// The hint and the rows, drawn on the skin's grid.
    #[must_use]
    pub fn body(&self, skin: &Skin, frame: &Frame) -> Vec<Draw> {
        let geometry = Geometry::new(skin);
        let layout = &geometry.layout;
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
        let dim = |mut ink: [f32; 4]| {
            ink[3] *= 0.5;
            ink
        };
        let hint_ink = frame.ink.filter(|_| layout.blocks.is_some());
        let mut out = vec![text(
            layout.text_x,
            geometry.hint_y,
            layout.scale * 0.6,
            hint_ink.unwrap_or_else(|| skin.normal()),
            Align::Left,
            self.labels.hint.clone(),
        )];
        for (index, row) in self.rows().into_iter().enumerate() {
            let y = layout.text_y(index);
            let mut ink = if self.is_lit(row) {
                skin.selected()
            } else {
                skin.normal()
            };
            let (left, right) = match row {
                Row::Person(person) => {
                    if !self.ready[person] {
                        ink = dim(ink);
                    }
                    (
                        format!("{} {}", self.labels.player, person + 1),
                        self.place(person),
                    )
                }
                Row::Auto => (self.labels.auto.clone(), String::new()),
                Row::Start => {
                    if !self.can_start() {
                        ink = dim(ink);
                    }
                    (self.labels.start.clone(), String::new())
                }
                Row::Back => (self.labels.back.clone(), String::new()),
            };
            out.push(text(layout.text_x, y, layout.scale, ink, Align::Left, left));
            if !right.is_empty() {
                out.push(text(
                    layout.right_x,
                    y,
                    layout.scale,
                    ink,
                    Align::Right,
                    right,
                ));
            }
        }
        out
    }

    /// The preview: a box per window, its panes cut as the race cuts them, each cell naming
    /// the person in it. A person who has not confirmed is dimmed; the one the pointer holds
    /// is lit.
    #[must_use]
    pub fn preview(
        &self,
        skin: &Skin,
        frame: &Frame,
        cells: &dyn Fn(usize, bool) -> Vec<[f32; 4]>,
    ) -> Vec<Draw> {
        let windows: Vec<PreviewWindow> = self
            .windows
            .iter()
            .map(|window| PreviewWindow {
                seats: window.seats,
                vertical: window.vertical,
                caption: None,
            })
            .collect();
        let people = self.pane_people();
        let hovered = match self.hover {
            Some(Row::Person(person)) => Some(person),
            _ => None,
        };
        draw_preview(skin, frame, &windows, None, cells, &|pane| {
            let person = people[pane];
            CellPaint {
                text: (person + 1).to_string(),
                lit: self.held == Some(person) || hovered == Some(person),
                firm: self.ready[person],
            }
        })
    }

    /// The title this screen is called.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.labels.title
    }
}

/// Whether `order` holds every number below `count` once.
fn is_permutation(order: &[usize], count: usize) -> bool {
    let mut seen = vec![false; count];
    order.len() == count
        && order
            .iter()
            .all(|&n| n < count && !std::mem::replace(&mut seen[n], true))
}

/// The whole screen in paint order: the title's frame, the title, the row blocks where the
/// title draws them, the rows, then the preview. The shape [`crate::window_layout::draw_list`]
/// has.
#[must_use]
pub fn draw_list(
    screen: &SeatAssign,
    skin: &Skin,
    frame: &Frame,
    backdrop: Option<Picture>,
    race_behind: bool,
    cells: &dyn Fn(usize, bool) -> Vec<[f32; 4]>,
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
        screen.title().to_string(),
    ));
    layers.body = screen.blocks(skin, frame);
    layers.body.extend(screen.body(skin, frame));
    layers.body.extend(screen.preview(skin, frame, cells));
    layers
}

#[cfg(test)]
mod tests;
