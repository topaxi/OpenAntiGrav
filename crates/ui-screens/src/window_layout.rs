//! The window layout screen, after the join screen and before a split-screen race: how many
//! OS windows the people are dealt across, how many seats each window holds, how a two-seat
//! window is cut, which monitor each window sits on, and windowed or fullscreen.
//!
//! **Ours: no disc authors this, and the original has no multi-window mode at all.**
//! Everything here is **chosen, not measured**: the rows, the wording, the preview and which
//! press does what. What it writes is what `--window-seats` writes
//! (`oag_game::split_screen::WindowSeats`, `docs/rendering/multi-window.md`).
//!
//! Drawn in the join screen's rows ([`crate::join`]): the title at the skin's title position
//! and the rows at its menu origin and pitch, with HD's and Omega's row blocks where the
//! title draws them. The preview is a box per window with its seats cut up as a split screen
//! cuts them, numbered, and the window being edited lit. The model knows nothing of winit: the
//! composition root hands it the monitors' names and takes the choice back.
//!
//! The rows are one cursor for a pad, a keyboard and a pointer alike: up and down move it,
//! left and right change the value under it (confirm steps it on), a click on a row steps
//! it, the wheel over a row changes it, and a click on a box in the preview picks that
//! window. START, or a click on START, goes on; circle or the secondary button goes back.

use oag_gameplay::input::{Button, Input};
use oag_ui::frontend::{Align, Draw};
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::pointer::{Pointer, contains, row_at};

use crate::join::{Layout, row_blocks};

/// Most windows one layout holds, and most seats (the grid's own cap).
pub const MAX_WINDOWS: usize = 8;

/// The words the screen says, already looked up in the player's language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labels {
    /// The screen's title.
    pub title: String,
    /// The line under the title.
    pub hint: String,
    /// The row that counts the windows.
    pub windows: String,
    /// The row that picks which window the rows below edit.
    pub window: String,
    /// The row that says how many seats the picked window holds.
    pub seats: String,
    /// "PLAYER", before a seat's number.
    pub player: String,
    /// The row that says how a two-seat window is cut.
    pub cut: String,
    /// One seat above the other.
    pub horizontal: String,
    /// One seat beside the other.
    pub vertical: String,
    /// The row that picks a window's monitor.
    pub monitor: String,
    /// The row that picks windowed or fullscreen.
    pub mode: String,
    /// The windowed value.
    pub windowed: String,
    /// The fullscreen value.
    pub fullscreen: String,
    /// The row that begins the race box.
    pub start: String,
    /// The row that leaves the screen.
    pub back: String,
}

impl Labels {
    /// The words in the language `strings` is, English where an id is missing. The monitor and
    /// window-mode words are the DISPLAY page's own rows.
    #[must_use]
    pub fn read(strings: &oag_ui::language::StringTable) -> Self {
        let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
        Self {
            title: say("OAG_WINDOW_LAYOUT_TITLE", "WINDOW LAYOUT"),
            hint: say(
                "OAG_WINDOW_LAYOUT_HINT",
                "CHOOSE HOW THE PLAYERS ARE SPLIT ACROSS WINDOWS",
            ),
            windows: say("OAG_WINDOW_LAYOUT_WINDOWS", "WINDOWS"),
            window: say("OAG_WINDOW_LAYOUT_WINDOW", "WINDOW"),
            seats: say("OAG_WINDOW_LAYOUT_SEATS", "SEATS"),
            player: say("OAG_JOIN_PLAYER", "PLAYER"),
            cut: say("OAG_WINDOW_LAYOUT_CUT", "CUT"),
            horizontal: say("OAG_WINDOW_LAYOUT_HORIZONTAL", "HORIZONTAL"),
            vertical: say("OAG_WINDOW_LAYOUT_VERTICAL", "VERTICAL"),
            monitor: say("OAG_DISPLAY_MONITOR", "MONITOR"),
            mode: say("OAG_DISPLAY_WINDOW_MODE", "WINDOW MODE"),
            windowed: say("OAG_WINDOW_LAYOUT_WINDOWED", "WINDOWED"),
            fullscreen: say("OAG_WINDOW_LAYOUT_FULLSCREEN", "FULLSCREEN"),
            start: say("OAG_JOIN_START", "START"),
            back: say("OAG_MENU_BACK", "BACK"),
        }
    }
}

/// One window of the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    /// How many seats it holds, from 1.
    pub seats: usize,
    /// Whether two seats are cut side by side rather than one above the other.
    pub vertical: bool,
    /// The monitor it sits on, an index into the monitors the screen was given.
    pub monitor: usize,
}

/// One line of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// How many windows.
    Windows,
    /// Windowed or fullscreen.
    Mode,
    /// Which window the next rows edit (more than one window only).
    Window,
    /// How many seats that window holds (more than one window only).
    Seats,
    /// How that window cuts two seats (a window of exactly two seats only).
    Cut,
    /// Which monitor that window sits on (more than one monitor only).
    Monitor,
    /// Go on to the race box.
    Start,
    /// Leave the screen.
    Back,
}

/// What a tick of input asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The cursor moved to another row, or another window was picked.
    Moved,
    /// A value changed.
    Changed,
    /// START.
    Start,
    /// BACK, or circle, or the secondary button.
    Back,
}

/// The screen: the layout being chosen and where the cursor is.
#[derive(Debug, Clone)]
pub struct Picker {
    labels: Labels,
    /// How many people the windows are dealt across: the join screen's seats.
    people: usize,
    windows: Vec<Window>,
    /// The window the rows below the picker edit.
    edit: usize,
    monitors: Vec<String>,
    fullscreen: bool,
    /// An index into [`Self::rows`].
    cursor: usize,
}

/// Where the rows and the preview sit on the skin's grid.
pub(crate) struct Geometry {
    pub(crate) layout: Layout,
    /// Where the hint line goes.
    pub(crate) hint_y: f32,
    /// How far right a row answers a click.
    pub(crate) column: f32,
    /// The preview's area: `[x, y, width, height]`.
    pub(crate) preview: [f32; 4],
}

/// How much smaller a bare-text title's rows are drawn here than on its own pages, so eight
/// rows and the preview fit one screen. **Chosen, not measured.**
const BARE_SHRINK: f32 = 0.72;
/// Where a bare-text title's rows end and its preview begins, as a fraction of the grid's
/// width.
const BARE_SPLIT: f32 = 0.56;

impl Geometry {
    pub(crate) fn new(skin: &Skin) -> Self {
        let mut layout = Layout::new(skin);
        let (width, height) = skin.space().size;
        let hint_y = layout.top - layout.line * 0.5;
        let margin = layout.text_x;
        if let Some(blocks) = layout.blocks.as_ref() {
            // HD and Omega: the rows are blocks as wide as their lists, and the free space
            // is under them.
            let rows_end = layout.top + (MAX_ROWS as f32 + 0.6) * layout.line;
            let column = blocks.value_x + blocks.value_width;
            return Self {
                hint_y: layout.top + MAX_ROWS as f32 * layout.line,
                column,
                preview: [
                    blocks.x,
                    rows_end + layout.line * 0.4,
                    (width - 2.0 * blocks.x).max(1.0),
                    (height * 0.84 - rows_end - layout.line * 0.4).max(1.0),
                ],
                layout,
            };
        }
        layout.scale *= BARE_SHRINK;
        layout.line *= BARE_SHRINK;
        layout.right_x = width * BARE_SPLIT;
        // Where the title leaves no room under it (2048's placeholder menu) the hint goes
        // under the rows, as the join screen's does.
        let hint_y = if layout.hint_below {
            layout.top + MAX_ROWS as f32 * layout.line
        } else {
            hint_y
        };
        let preview_x = width * (BARE_SPLIT + 0.05);
        Self {
            hint_y,
            column: width * BARE_SPLIT + margin * 0.5,
            preview: [
                preview_x,
                layout.top,
                (width - preview_x - margin).max(1.0),
                (height * 0.78 - layout.top).max(1.0),
            ],
            layout,
        }
    }
}

/// `ink` where it reads on a dark box, else white: a light page (Omega's) inks its text black,
/// which the preview's dark boxes would swallow.
pub(crate) fn legible(ink: [f32; 4]) -> [f32; 4] {
    if ink[0] + ink[1] + ink[2] < 1.2 {
        [1.0, 1.0, 1.0, 0.6]
    } else {
        ink
    }
}

/// The most rows the screen has (the rows and preview are laid out for this many).
pub(crate) const MAX_ROWS: usize = 8;

impl Picker {
    /// A screen for `people` seated people, starting from `windows` (what was chosen last,
    /// or one window), over `monitors` (their names, in the platform's order) and with
    /// `fullscreen` as the mode now.
    ///
    /// `windows` is made to fit: at most `people` of them, each holding a seat, the seats
    /// adding up to `people` (a layout of another head count is dealt anew, keeping each
    /// window's cut and monitor), every monitor one that exists.
    #[must_use]
    pub fn new(
        labels: Labels,
        people: usize,
        mut windows: Vec<Window>,
        monitors: Vec<String>,
        fullscreen: bool,
    ) -> Self {
        let people = people.clamp(1, MAX_WINDOWS);
        let monitor_count = monitors.len().max(1);
        windows.truncate(people);
        if windows.is_empty() {
            windows.push(Window {
                seats: people,
                vertical: false,
                monitor: 0,
            });
        }
        for window in &mut windows {
            window.monitor = window.monitor.min(monitor_count - 1);
        }
        if windows.iter().map(|window| window.seats).sum::<usize>() != people
            || windows.iter().any(|window| window.seats == 0)
        {
            let count = windows.len();
            for (index, window) in windows.iter_mut().enumerate() {
                window.seats = people / count + usize::from(index < people % count);
            }
        }
        Self {
            labels,
            people,
            windows,
            edit: 0,
            monitors,
            fullscreen,
            cursor: 0,
        }
    }

    /// The layout as chosen: one entry per window.
    #[must_use]
    pub fn windows(&self) -> &[Window] {
        &self.windows
    }

    /// Whether the windows are fullscreen.
    #[must_use]
    pub fn fullscreen(&self) -> bool {
        self.fullscreen
    }

    /// How many people the layout seats.
    #[must_use]
    pub fn people(&self) -> usize {
        self.people
    }

    /// The window the rows below the picker edit, from 0.
    #[must_use]
    pub fn edited(&self) -> usize {
        self.edit
    }

    /// The most windows this head count allows: one a person, eight at most.
    #[must_use]
    pub fn most_windows(&self) -> usize {
        self.people.min(MAX_WINDOWS)
    }

    /// The rows, top to bottom, for the layout as it stands.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        let mut rows = vec![Row::Windows, Row::Mode];
        if self.windows.len() > 1 {
            rows.push(Row::Window);
            rows.push(Row::Seats);
        }
        if self.windows[self.edit].seats == 2 {
            rows.push(Row::Cut);
        }
        if self.monitors.len() > 1 {
            rows.push(Row::Monitor);
        }
        rows.push(Row::Start);
        rows.push(Row::Back);
        rows
    }

    /// The row the cursor is on.
    #[must_use]
    pub fn selected(&self) -> Row {
        let rows = self.rows();
        rows[self.cursor.min(rows.len() - 1)]
    }

    /// Picks the next window to edit, wrapping: what a click on a box or the WINDOW row does.
    pub fn pick_next_window(&mut self) {
        self.edit = (self.edit + 1) % self.windows.len();
        self.cursor = self.cursor.min(self.rows().len() - 1);
    }

    /// The first seat (from 0) of window `index`: the seats before it.
    fn first_seat(&self, index: usize) -> usize {
        self.windows[..index]
            .iter()
            .map(|window| window.seats)
            .sum()
    }

    /// Sets the window count to `count` (1 to [`Self::most_windows`]): a new window takes a
    /// seat from the fullest one and the next monitor; a window that goes hands its seats
    /// to the one before it.
    pub fn set_window_count(&mut self, count: usize) {
        let count = count.clamp(1, self.most_windows());
        let monitors = self.monitors.len().max(1);
        while self.windows.len() > count {
            let gone = self.windows.pop().map_or(0, |window| window.seats);
            if let Some(last) = self.windows.last_mut() {
                last.seats += gone;
            }
        }
        while self.windows.len() < count {
            let donor = self
                .windows
                .iter()
                .enumerate()
                .max_by_key(|(index, window)| (window.seats, std::cmp::Reverse(*index)))
                .map(|(index, _)| index);
            let Some(donor) = donor.filter(|&index| self.windows[index].seats > 1) else {
                break;
            };
            self.windows[donor].seats -= 1;
            let monitor = self.windows.len() % monitors;
            self.windows.push(Window {
                seats: 1,
                vertical: false,
                monitor,
            });
        }
        self.edit = self.edit.min(self.windows.len() - 1);
        self.cursor = self.cursor.min(self.rows().len() - 1);
    }

    /// Gives the edited window one seat more or fewer, from or to the window after it (the
    /// first when it is the last). Returns whether anything moved: a window keeps at least
    /// one seat, so a trade that would empty one does not happen.
    fn trade_seat(&mut self, delta: i32) -> bool {
        let count = self.windows.len();
        if count < 2 {
            return false;
        }
        let from_to = |step: usize| (self.edit + step) % count;
        if delta > 0 {
            // Takes from the next window that can spare one.
            let Some(other) = (1..count).map(from_to).find(|&i| self.windows[i].seats > 1) else {
                return false;
            };
            self.windows[other].seats -= 1;
            self.windows[self.edit].seats += 1;
        } else {
            if self.windows[self.edit].seats <= 1 {
                return false;
            }
            self.windows[self.edit].seats -= 1;
            self.windows[from_to(1)].seats += 1;
        }
        self.cursor = self.cursor.min(self.rows().len() - 1);
        true
    }

    /// Steps `row`'s value by `delta` (+1 or -1); values that are a cycle wrap. Returns the
    /// event it makes, if the value changed.
    fn adjust(&mut self, row: Row, delta: i32) -> Option<Event> {
        let wrap = |value: usize, count: usize| {
            (value as i64 + i64::from(delta)).rem_euclid(count as i64) as usize
        };
        match row {
            Row::Windows => {
                let to = wrap(self.windows.len() - 1, self.most_windows()) + 1;
                let changed = to != self.windows.len();
                self.set_window_count(to);
                changed.then_some(Event::Changed)
            }
            Row::Mode => {
                self.fullscreen = !self.fullscreen;
                Some(Event::Changed)
            }
            Row::Window => {
                let to = wrap(self.edit, self.windows.len());
                let changed = to != self.edit;
                self.edit = to;
                self.cursor = self.cursor.min(self.rows().len() - 1);
                changed.then_some(Event::Moved)
            }
            Row::Seats => self.trade_seat(delta).then_some(Event::Changed),
            Row::Cut => {
                let window = &mut self.windows[self.edit];
                window.vertical = !window.vertical;
                Some(Event::Changed)
            }
            Row::Monitor => {
                let count = self.monitors.len().max(1);
                let window = &mut self.windows[self.edit];
                window.monitor = wrap(window.monitor, count);
                Some(Event::Changed)
            }
            Row::Start | Row::Back => None,
        }
    }

    fn move_cursor(&mut self, direction: i32) -> bool {
        let count = self.rows().len();
        let to = (self.cursor as i64 + i64::from(direction)).rem_euclid(count as i64) as usize;
        let moved = to != self.cursor;
        self.cursor = to;
        moved
    }

    /// Consumes a tick of pad and keyboard input; every button is taken as an edge.
    pub fn update(&mut self, input: &mut Input) -> Vec<Event> {
        let mut out = Vec::new();
        if input.take(Button::Down) && self.move_cursor(1) {
            out.push(Event::Moved);
        }
        if input.take(Button::Up) && self.move_cursor(-1) {
            out.push(Event::Moved);
        }
        let row = self.selected();
        if input.take(Button::Left) {
            out.extend(self.adjust(row, -1));
        }
        if input.take(Button::Right) {
            out.extend(self.adjust(row, 1));
        }
        if input.take(Button::Circle) {
            out.push(Event::Back);
        }
        if input.take(Button::Start) {
            out.push(Event::Start);
        }
        if input.take(Button::Cross) {
            match row {
                Row::Start => out.push(Event::Start),
                Row::Back => out.push(Event::Back),
                _ => out.extend(self.adjust(row, 1)),
            }
        }
        out
    }

    /// Consumes a tick of pointer input. Hover moves the cursor, a click steps the row's
    /// value (START and BACK act), the wheel changes it, a click on a box of the preview
    /// picks that window, and the secondary button backs out.
    pub fn pointer(&mut self, pointer: &Pointer, skin: &Skin) -> Vec<Event> {
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
        );
        if let Some(index) = over {
            if (pointer.moved || pointer.clicked || pointer.scroll != 0) && index != self.cursor {
                self.cursor = index;
                out.push(Event::Moved);
            }
            let row = rows[index];
            if pointer.clicked {
                match row {
                    Row::Start => out.push(Event::Start),
                    Row::Back => out.push(Event::Back),
                    _ => out.extend(self.adjust(row, 1)),
                }
            }
            if pointer.scroll != 0 {
                out.extend(self.adjust(row, pointer.scroll.signum()));
            }
        }
        if pointer.clicked
            && let Some(window) = boxes(
                self.windows.len(),
                self.monitors.len() > 1,
                geometry.preview,
            )
            .iter()
            .position(|rect| contains(*rect, at))
            && window != self.edit
        {
            self.edit = window;
            self.cursor = self.cursor.min(self.rows().len() - 1);
            out.push(Event::Moved);
        }
        out
    }

    /// What a row's value says.
    fn value(&self, row: Row) -> String {
        let window = &self.windows[self.edit];
        match row {
            Row::Windows => self.windows.len().to_string(),
            Row::Mode => if self.fullscreen {
                &self.labels.fullscreen
            } else {
                &self.labels.windowed
            }
            .clone(),
            Row::Window => (self.edit + 1).to_string(),
            Row::Seats => {
                let first = self.first_seat(self.edit) + 1;
                if window.seats == 1 {
                    format!("{} {first}", self.labels.player)
                } else {
                    format!(
                        "{} {first} - {}",
                        self.labels.player,
                        first + window.seats - 1
                    )
                }
            }
            Row::Cut => if window.vertical {
                &self.labels.vertical
            } else {
                &self.labels.horizontal
            }
            .clone(),
            Row::Monitor => self.monitor_name(window.monitor),
            Row::Start | Row::Back => String::new(),
        }
    }

    /// A monitor's name as a row can hold it.
    fn monitor_name(&self, monitor: usize) -> String {
        let name = self
            .monitors
            .get(monitor)
            .map_or_else(|| (monitor + 1).to_string(), |name| name.to_uppercase());
        if name.chars().count() > 14 {
            name.chars().take(13).chain(std::iter::once('.')).collect()
        } else {
            name
        }
    }

    fn row_label(&self, row: Row) -> &str {
        match row {
            Row::Windows => &self.labels.windows,
            Row::Mode => &self.labels.mode,
            Row::Window => &self.labels.window,
            Row::Seats => &self.labels.seats,
            Row::Cut => &self.labels.cut,
            Row::Monitor => &self.labels.monitor,
            Row::Start => &self.labels.start,
            Row::Back => &self.labels.back,
        }
    }

    /// The boxes behind the rows on a title that draws them (HD, Omega); empty elsewhere.
    #[must_use]
    pub fn blocks(&self, skin: &Skin, frame: &Frame) -> Vec<Draw> {
        let shape: Vec<(bool, bool)> = self
            .rows()
            .iter()
            .enumerate()
            .map(|(index, row)| (index == self.cursor, !matches!(row, Row::Back)))
            .collect();
        row_blocks(skin, frame, &shape)
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
            let ink = if index == self.cursor {
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
                self.row_label(row).to_string(),
            ));
            let value = self.value(row);
            if !value.is_empty() {
                out.push(text(
                    layout.right_x,
                    y,
                    layout.scale,
                    ink,
                    Align::Right,
                    value,
                ));
            }
        }
        out
    }

    /// The preview: a box per window, its seats cut up and numbered, the edited window lit.
    /// `cells` says how `seats` seats are cut (`vertical` for two side by side) as
    /// rectangles in the unit square, in seat order - the caller's own cut, which is
    /// `oag_render::viewport::layout`, so the picture is the one the race draws.
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
                caption: (self.monitors.len() > 1).then(|| self.monitor_name(window.monitor)),
            })
            .collect();
        let owner: Vec<usize> = self
            .windows
            .iter()
            .enumerate()
            .flat_map(|(index, window)| std::iter::repeat_n(index, window.seats))
            .collect();
        draw_preview(skin, frame, &windows, Some(self.edit), cells, &|seat| {
            CellPaint {
                text: (seat + 1).to_string(),
                lit: owner.get(seat) == Some(&self.edit),
                firm: true,
            }
        })
    }
}

/// Each window's box in the preview, in window order: 16:9, centred, in as many columns
/// as make the boxes largest for the area. `labelled` leaves room under each box for a line
/// of text (the monitor, drawn when there is more than one).
pub(crate) fn boxes(count: usize, labelled: bool, area: [f32; 4]) -> Vec<[f32; 4]> {
    let gap = (area[2] * 0.03).min(area[3] * 0.06).max(1.0);
    let label = if labelled { area[3] * 0.1 } else { 0.0 };
    let width_for = |per_row: usize| {
        let rows = count.div_ceil(per_row);
        let cell_w = (area[2] - gap * (per_row - 1) as f32) / per_row as f32;
        let cell_h = (area[3] - gap * (rows - 1) as f32) / rows as f32 - label;
        cell_w.min(cell_h.max(1.0) * 16.0 / 9.0)
    };
    let per_row = (1..=count)
        .max_by(|&a, &b| width_for(a).total_cmp(&width_for(b)).then(b.cmp(&a)))
        .unwrap_or(1);
    let width = width_for(per_row);
    let height = width * 9.0 / 16.0;
    (0..count)
        .map(|index| {
            let (row, column) = (index / per_row, index % per_row);
            let in_row = per_row.min(count - row * per_row);
            let used = in_row as f32 * width + (in_row - 1) as f32 * gap;
            [
                area[0] + (area[2] - used) / 2.0 + column as f32 * (width + gap),
                area[1] + row as f32 * (height + label + gap),
                width,
                height,
            ]
        })
        .collect()
}

/// The rectangles of a window box's seat cells, in seat order, inside its border.
pub(crate) fn cell_rects(
    rect: [f32; 4],
    seats: usize,
    vertical: bool,
    cells: &dyn Fn(usize, bool) -> Vec<[f32; 4]>,
) -> Vec<[f32; 4]> {
    let edge = (rect[2] * 0.012).max(1.0);
    let inner = [
        rect[0] + 2.0 * edge,
        rect[1] + 2.0 * edge,
        rect[2] - 4.0 * edge,
        rect[3] - 4.0 * edge,
    ];
    cells(seats, vertical)
        .into_iter()
        .map(|cell| {
            [
                inner[0] + cell[0] * inner[2],
                inner[1] + cell[1] * inner[3],
                cell[2] * inner[2],
                cell[3] * inner[3],
            ]
        })
        .collect()
}

/// One window of a preview: how many seats it cuts and which way, and the line under its box.
pub(crate) struct PreviewWindow {
    pub(crate) seats: usize,
    pub(crate) vertical: bool,
    /// The line drawn under the box (the monitor), if the preview has one.
    pub(crate) caption: Option<String>,
}

/// How a preview paints one seat's cell.
pub(crate) struct CellPaint {
    /// What the cell says (the seat's number, or the person in it).
    pub(crate) text: String,
    /// Whether it is drawn lit, in the selected ink.
    pub(crate) lit: bool,
    /// Whether it is drawn at full strength rather than dimmed.
    pub(crate) firm: bool,
}

/// The preview both the layout screen and the seat assignment draw: a box per window, its seats
/// cut up as a split screen cuts them (`cells`), the `edited` window's box lit, and each seat's
/// cell painted by `paint(seat)`, seats counted across the windows from 0.
pub(crate) fn draw_preview(
    skin: &Skin,
    frame: &Frame,
    windows: &[PreviewWindow],
    edited: Option<usize>,
    cells: &dyn Fn(usize, bool) -> Vec<[f32; 4]>,
    paint: &dyn Fn(usize) -> CellPaint,
) -> Vec<Draw> {
    let geometry = Geometry::new(skin);
    let ink = frame.ink.filter(|_| geometry.layout.blocks.is_some());
    let ink = ink.unwrap_or_else(|| skin.normal());
    let lit = skin.selected();
    let dim = |mut colour: [f32; 4], alpha: f32| {
        colour[3] *= alpha;
        colour
    };
    let labelled = windows.iter().any(|window| window.caption.is_some());
    let mut out = Vec::new();
    let mut seat = 0;
    for (index, (window, rect)) in windows
        .iter()
        .zip(boxes(windows.len(), labelled, geometry.preview))
        .enumerate()
    {
        let is_edited = edited == Some(index);
        let border = if is_edited { lit } else { dim(ink, 0.5) };
        let edge = (rect[2] * 0.012).max(1.0);
        out.push(Draw::Fill {
            rect,
            color: border,
        });
        out.push(Draw::Fill {
            rect: [
                rect[0] + edge,
                rect[1] + edge,
                rect[2] - 2.0 * edge,
                rect[3] - 2.0 * edge,
            ],
            color: [0.0, 0.0, 0.0, 0.85],
        });
        for cell in cell_rects(rect, window.seats, window.vertical, cells) {
            let paint = paint(seat);
            seat += 1;
            out.push(Draw::Fill {
                rect: cell,
                color: dim(if paint.lit { lit } else { ink }, 0.28),
            });
            let scale = (cell[3] * 0.045).clamp(0.2, 1.0).min(cell[2] * 0.02);
            out.push(Draw::Text {
                x: cell[0] + cell[2] / 2.0,
                y: cell[1] + cell[3] / 2.0 - scale * 8.0,
                scale,
                color: if paint.lit {
                    dim(lit, if paint.firm { 1.0 } else { 0.6 })
                } else {
                    dim(legible(ink), if paint.firm { 1.0 } else { 0.6 })
                },
                border: None,
                align: Align::Centre,
                text: paint.text,
                wrap_width: None,
            });
        }
        if let Some(caption) = &window.caption {
            out.push(Draw::Text {
                x: rect[0] + rect[2] / 2.0,
                y: rect[1] + rect[3] + rect[3] * 0.04,
                scale: (rect[3] * 0.006).clamp(0.2, 0.7),
                color: dim(ink, if is_edited { 1.0 } else { 0.6 }),
                border: None,
                align: Align::Centre,
                text: caption.clone(),
                wrap_width: None,
            });
        }
    }
    out
}

/// The whole screen in paint order: the title's frame, the title, the row blocks where the
/// title draws them, the rows, then the preview. The shape [`crate::join::draw_list`] has.
#[must_use]
pub fn draw_list(
    picker: &Picker,
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
        picker.labels.title.clone(),
    ));
    layers.body = picker.blocks(skin, frame);
    layers.body.extend(picker.body(skin, frame));
    layers.body.extend(picker.preview(skin, frame, cells));
    layers
}

#[cfg(test)]
mod tests;
