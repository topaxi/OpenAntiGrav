//! The "press a button to join" screen before a split-screen race.
//!
//! **Ours: no disc authors this.** The PS2 pressing has `Split Screen` race
//! screens (`docs/formats/race-setup.md`) but takes its players from the pads in
//! port order, with no join step to read. Everything here is **chosen, not
//! measured**: the rows, the wording, and which press does what.
//!
//! The model knows nothing of devices. The composition root keeps the
//! `oag_input::lobby::Lobby` that decides who has a seat, hands this the seats'
//! device names, and feeds a pad's or key's press to the lobby itself - a press
//! has to be told apart by *which* device made it, which a cursor cannot do. What
//! this owns is the half a pointer drives: hover, and a click on a seat, the
//! empty row, START or BACK, answered as an [`Event`] the root acts on. The mouse
//! has no seat of its own, so a click on the empty row joins the keyboard (which
//! is what a click arrives as), and a click on a taken seat leaves it.
//!
//! Drawn as the title's own menu pages are, the way
//! [`crate::pure_chain::draw_list`] is: the title at the skin's title position,
//! the rows at the skin's menu origin and pitch.

use oag_ui::frontend::{Align, Draw};
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::pointer::{Pointer, row_at};

/// The words the screen says, already looked up in the player's language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labels {
    /// The screen's title.
    pub title: String,
    /// The line under the title: what to press.
    pub hint: String,
    /// "PLAYER", before a seat's number.
    pub player: String,
    /// What an empty seat says.
    pub empty: String,
    /// The keyboard's name, which is also the mouse's.
    pub keyboard: String,
    /// "GAMEPAD", for a pad the backend gave no name.
    pub pad: String,
    /// The row that begins the race.
    pub start: String,
    /// The row that leaves the screen.
    pub back: String,
}

/// One line of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// A seat somebody holds.
    Seat(usize),
    /// The next seat, empty.
    Empty,
    /// Begin the race.
    Start,
    /// Leave the screen.
    Back,
}

/// What the pointer asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The hover moved to another row.
    Moved,
    /// A click on the empty row: the keyboard asks to join.
    Join,
    /// A click on seat `n`: it is given up.
    Leave(usize),
    /// A click on START with somebody seated.
    Start,
    /// A click on BACK, or the secondary button.
    Back,
}

/// The screen: who sits where, and what the pointer is over.
#[derive(Debug, Clone)]
pub struct Join {
    labels: Labels,
    /// The device name of each seat, seat 0 first.
    seats: Vec<String>,
    /// The seats the grid has.
    capacity: usize,
    /// The row the pointer last moved onto.
    hover: Option<Row>,
}

/// Where the rows sit on the skin's grid.
struct Layout {
    /// Where a row's label starts.
    text_x: f32,
    /// Where the device column ends.
    right_x: f32,
    /// From one row to the next.
    line: f32,
    /// The text's scale.
    scale: f32,
    /// The top of the first row, which is what a click is measured from.
    top: f32,
    /// Whether the hint goes under the rows rather than between the title and the first
    /// one: where a block row would sit on the screen's rule, or where this build's own
    /// row origin leaves no room under a title (2048's placeholder menu).
    hint_below: bool,
    /// How far a row's text sits below its top: nothing for bare text, the tab's own
    /// pad inside a block.
    text_pad: f32,
    width: f32,
    /// The row boxes, on a title whose menus draw blocks.
    blocks: Option<BlockRows>,
}

/// The boxes behind the rows on HD and Omega, in the grid being drawn in. **Chosen, not
/// measured**: the screen takes HD's own `<List>` geometry (`oag_title::MenuList` and
/// `ListBlocks`: where the rows start, their pitch, their text scale, the block height and
/// the gap) and gives a label block and a value block, the value block wider than any
/// value HD's lists hold because a device's name is longer.
struct BlockRows {
    x: f32,
    label_width: f32,
    value_x: f32,
    value_width: f32,
    height: f32,
    scale: (f32, f32),
}

/// A value block's width in multiples of the title's own list value width.
const VALUE_WIDTHS: f32 = 2.0;
/// How far a device name sits in from its block's right edge: HD's own value inset.
const VALUE_INSET: f32 = 13.0;
/// The tab text's pad over a block's top, as a fraction of the 272-line reference screen
/// (`oag_ui::menu::Skin`'s own, 1.9).
const TEXT_TOP_PAD: f32 = 1.9;

impl Layout {
    fn new(skin: &Skin) -> Self {
        let (width, grid_height) = skin.space().size;
        if let (Some(list), Some(numbers)) = (skin.list(), skin.blocks()) {
            let (sx, sy) = skin.theirs_scale();
            let numbers = numbers.list;
            let label_width = numbers.label_width * sx;
            let value_x = list.x + label_width + numbers.gap * sx;
            let value_width = numbers.value_width * sx * VALUE_WIDTHS;
            return Self {
                text_x: list.x + (numbers.marker_offset.0 + numbers.arrow_size) * sx,
                right_x: value_x + value_width - VALUE_INSET * sx,
                line: list.pitch,
                scale: skin.row_scale() * list.text_scale,
                top: list.y,
                hint_below: true,
                text_pad: TEXT_TOP_PAD * grid_height / oag_display::space::SCREEN.1,
                width,
                blocks: Some(BlockRows {
                    x: list.x,
                    label_width,
                    value_x,
                    value_width,
                    height: numbers.height * sy * list.height_scale,
                    scale: (sx, sy),
                }),
            };
        }
        let text_x = skin.menu_x();
        // The right column ends at the same margin the screen title starts at. The
        // rows' own left edge is that margin on the PSP titles, where mirroring it
        // is the same thing.
        let margin = text_x.min(skin.title_at().0);
        Self {
            text_x,
            right_x: width - margin,
            line: skin.row_pitch(),
            scale: skin.row_scale(),
            top: skin.first_row_y(),
            hint_below: skin.first_row_is_ours(),
            text_pad: 0.0,
            width,
            blocks: None,
        }
    }

    /// The text's `y` on row `index`.
    fn text_y(&self, index: usize) -> f32 {
        self.top + index as f32 * self.line + self.text_pad
    }
}

impl Join {
    /// A screen with nobody seated, for a grid of `capacity` seats.
    #[must_use]
    pub fn new(labels: Labels, capacity: usize) -> Self {
        Self {
            labels,
            seats: Vec::new(),
            capacity,
            hover: None,
        }
    }

    /// Who sits where now: one device name per seat.
    pub fn set_seats(&mut self, seats: Vec<String>) {
        self.seats = seats;
        if let Some(Row::Seat(n)) = self.hover
            && n >= self.seats.len()
        {
            self.hover = None;
        }
    }

    /// The rows, top to bottom: every seat, the empty one if the grid has room,
    /// START, BACK.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        let mut rows: Vec<Row> = (0..self.seats.len()).map(Row::Seat).collect();
        if self.seats.len() < self.capacity {
            rows.push(Row::Empty);
        }
        rows.push(Row::Start);
        rows.push(Row::Back);
        rows
    }

    /// How many are seated.
    #[must_use]
    pub fn count(&self) -> usize {
        self.seats.len()
    }

    /// The row the pointer is over, if it has moved onto one.
    #[must_use]
    pub fn hover(&self) -> Option<Row> {
        self.hover
    }

    /// Consumes a tick of pointer input. The secondary button backs out.
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
        let layout = Layout::new(skin);
        let rows = self.rows();
        let Some(row) = row_at(at, 0.0, layout.width, layout.top, layout.line, rows.len())
            .map(|index| rows[index])
        else {
            return out;
        };
        if (pointer.moved || pointer.clicked) && self.hover != Some(row) {
            self.hover = Some(row);
            out.push(Event::Moved);
        }
        if pointer.clicked {
            match row {
                Row::Seat(n) => out.push(Event::Leave(n)),
                Row::Empty => out.push(Event::Join),
                Row::Start if !self.seats.is_empty() => out.push(Event::Start),
                Row::Start => {}
                Row::Back => out.push(Event::Back),
            }
        }
        out
    }

    /// The boxes behind the rows, on a title whose menus draw blocks: a label block and a
    /// value block per row in the frame's ink, the hovered row's in the selected fill. Empty
    /// where the frame has no block art, which is both PSP titles: their rows are bare text.
    #[must_use]
    pub fn blocks(&self, skin: &Skin, frame: &Frame) -> Vec<Draw> {
        let layout = Layout::new(skin);
        let (Some(rows), Some(art), Some(ink)) = (layout.blocks.as_ref(), frame.blocks, frame.ink)
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (index, row) in self.rows().into_iter().enumerate() {
            let color = if self.hover == Some(row) {
                frame.tab_selected.unwrap_or(ink)
            } else {
                ink
            };
            let mut block = oag_ui::menu::block::Block {
                x: rows.x,
                y: layout.top + index as f32 * layout.line,
                width: rows.label_width,
                height: rows.height,
                color,
                landing: false,
            };
            oag_ui::menu::block::draw(&block, &art, rows.scale, &mut out);
            if matches!(row, Row::Back) {
                continue;
            }
            block.x = rows.value_x;
            block.width = rows.value_width;
            block.landing = true;
            oag_ui::menu::block::draw(&block, &art, rows.scale, &mut out);
        }
        out
    }

    /// The hint and the rows, drawn on the skin's grid. `frame` is the title's own, for the
    /// hint's ink on a title that draws blocks.
    #[must_use]
    pub fn body(&self, skin: &Skin, frame: &Frame) -> Vec<Draw> {
        let layout = Layout::new(skin);
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
        let rows = self.rows();
        let hint_y = if layout.hint_below {
            layout.top + rows.len() as f32 * layout.line
        } else {
            layout.top - layout.line * 0.5
        };
        // The hint is the one line outside a block on a blocked title, so it takes the frame's
        // own ink rather than the rows' white, which is invisible on Omega's light page.
        let hint_ink = frame.ink.filter(|_| layout.blocks.is_some());
        let mut out = vec![text(
            layout.text_x,
            hint_y,
            layout.scale * 0.6,
            hint_ink.unwrap_or_else(|| skin.normal()),
            Align::Left,
            self.labels.hint.clone(),
        )];
        for (index, row) in rows.into_iter().enumerate() {
            let y = layout.text_y(index);
            let mut ink = if self.hover == Some(row) {
                skin.selected()
            } else {
                skin.normal()
            };
            let (left, right) = match row {
                Row::Seat(n) => (
                    format!("{} {}", self.labels.player, n + 1),
                    self.seats[n].clone(),
                ),
                Row::Empty => {
                    ink = dim(ink);
                    (
                        format!("{} {}", self.labels.player, self.seats.len() + 1),
                        self.labels.empty.clone(),
                    )
                }
                Row::Start => {
                    if self.seats.is_empty() {
                        ink = dim(ink);
                    }
                    (self.labels.start.clone(), self.seats.len().to_string())
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

    /// The title this screen is called.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.labels.title
    }

    /// The words, for the caller that names a device.
    #[must_use]
    pub fn labels(&self) -> &Labels {
        &self.labels
    }
}

/// The whole screen in paint order: the title's frame, the title, the row blocks where the
/// title draws them ([`Join::blocks`]), then [`Join::body`]. The shape [`crate::pure_chain::draw_list`] has.
#[must_use]
pub fn draw_list(
    join: &Join,
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
        join.title().to_string(),
    ));
    layers.body = join.blocks(skin, frame);
    layers.body.extend(join.body(skin, frame));
    layers
}

#[cfg(test)]
mod tests;
