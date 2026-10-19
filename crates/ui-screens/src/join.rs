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
    /// The title's own line for the second seat, where its disc authors one (`FE_GUEST_JOIN`
    /// on HD and Omega: "PLAYER 2 PRESS START TO JOIN"); it replaces the whole empty row for
    /// that seat. `None` elsewhere, and for every later seat.
    pub guest: Option<String>,
    /// The keyboard's name, which is also the mouse's.
    pub keyboard: String,
    /// "GAMEPAD", for a pad the backend gave no name.
    pub pad: String,
    /// The room owner's row that opens the room's settings.
    pub settings: String,
    /// The room owner's row that sets how many people the room seats.
    pub max_players: String,
    /// The room owner's row that sets the laps, in a mode that ends on them.
    pub laps: String,
    /// The row that switches barrel rolls on or off for the room.
    pub barrel_rolls: String,
    /// The row that says whether the room allows Pilot Assist.
    pub pilot_assist: String,
    /// The row that sets the zone a Zone race ends at.
    pub zone_target: String,
    /// The weapons row.
    pub weapons: String,
    /// The row that begins the race.
    pub start: String,
    /// The row that leaves the screen.
    pub back: String,
}

impl Labels {
    /// The words in the language `strings` is, English where an id is missing. The screen is
    /// LOCAL MULTIPLAYER's join step.
    #[must_use]
    pub fn read(strings: &oag_ui::language::StringTable) -> Self {
        let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
        let disc_or = |disc: &str, ours: &str, english: &str| {
            strings
                .get(disc)
                .map_or_else(|| say(ours, english), str::to_uppercase)
        };
        Self {
            title: say("OAG_MP_LOCAL", "LOCAL MULTIPLAYER"),
            hint: say("OAG_JOIN_HINT", "EACH PLAYER PRESSES A BUTTON TO JOIN"),
            player: say("OAG_JOIN_PLAYER", "PLAYER"),
            empty: say("OAG_JOIN_EMPTY", "PRESS A BUTTON"),
            guest: strings.get("FE_GUEST_JOIN").map(str::to_uppercase),
            keyboard: say("OAG_JOIN_KEYBOARD", "KEYBOARD"),
            pad: say("OAG_JOIN_PAD", "GAMEPAD"),
            settings: say("OAG_ONLINE_ROOM_SETTINGS", "ROOM SETTINGS"),
            max_players: say("OAG_ONLINE_MAX_PLAYERS", "MAX PLAYERS"),
            laps: say("OAG_ONLINE_LAPS", "LAPS"),
            // Omega's own words where this title's disc has them (`MAN_2_BR`, `FE_PILOT_ASSIST_ALLOWED`,
            // `FE_ZONE_TARGET`), else this project's.
            barrel_rolls: disc_or("MAN_2_BR", "OAG_ONLINE_BARREL_ROLLS", "BARREL ROLLS"),
            pilot_assist: disc_or(
                "FE_PILOT_ASSIST_ALLOWED",
                "OAG_ONLINE_PILOT_ASSIST",
                "PILOT ASSIST ALLOWED",
            ),
            weapons: disc_or("RB_WEAP", "OAG_ONLINE_WEAPONS", "WEAPONS"),
            zone_target: disc_or("FE_ZONE_TARGET", "OAG_ONLINE_ZONE_TARGET", "ZONE TARGET"),
            start: say("OAG_JOIN_START", "START"),
            back: say("OAG_MENU_BACK", "BACK"),
        }
    }
}

impl Labels {
    /// The words of LAN play's host lobby (the screen in its lobby form).
    #[must_use]
    pub fn read_lan_lobby(strings: &oag_ui::language::StringTable) -> Self {
        let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
        Self {
            title: say("OAG_LAN_LOBBY_TITLE", "LAN LOBBY"),
            hint: say(
                "OAG_LAN_LOBBY_HINT",
                "THE OTHERS JOIN FROM JOIN GAME, THEN YOU START",
            ),
            empty: say("OAG_LAN_WAITING", "WAITING FOR PLAYERS"),
            // The disc's "PLAYER 2 PRESS START TO JOIN" is local split screen's line; here the
            // others arrive over the network and nobody presses anything to join.
            guest: None,
            start: say("OAG_LAN_START", "START RACE"),
            ..Self::read(strings)
        }
    }

    /// The words of LAN play's host list (the screen in its list form).
    #[must_use]
    pub fn read_lan_list(strings: &oag_ui::language::StringTable) -> Self {
        let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
        Self {
            title: say("OAG_LAN_JOINLIST_TITLE", "JOIN GAME"),
            hint: say("OAG_LAN_JOINLIST_HINT", "CHOOSE A GAME ON YOUR NETWORK"),
            empty: say("OAG_LAN_ADDRESS", "TYPE AN ADDRESS"),
            ..Self::read(strings)
        }
    }
}

impl Labels {
    /// The words of the online lobby's room list (the screen in its list form). The empty
    /// row makes a room of one's own.
    #[must_use]
    pub fn read_online_list(strings: &oag_ui::language::StringTable) -> Self {
        let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
        Self {
            title: say("OAG_ONLINE_LIST_TITLE", "ONLINE GAMES"),
            hint: say("OAG_ONLINE_LIST_HINT", "CHOOSE A GAME TO JOIN, OR MAKE ONE"),
            empty: say("OAG_ONLINE_CREATE_ROW", "CREATE A GAME"),
            ..Self::read(strings)
        }
    }

    /// The words of a room on the online lobby (the screen in its lobby form).
    #[must_use]
    pub fn read_online_room(strings: &oag_ui::language::StringTable) -> Self {
        let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
        Self {
            title: say("OAG_ONLINE_ROOM_TITLE", "GAME LOBBY"),
            hint: say("OAG_ONLINE_ROOM_HINT", "THE HOST STARTS THE RACE"),
            empty: say("OAG_LAN_WAITING", "WAITING FOR PLAYERS"),
            // The disc's "PLAYER 2 PRESS START TO JOIN" is local split screen's line; here the
            // others arrive over the network and nobody presses anything to join.
            guest: None,
            start: say("OAG_LAN_START", "START RACE"),
            ..Self::read(strings)
        }
    }
}

/// One line of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// A seat somebody holds.
    Seat(usize),
    /// The next seat, empty.
    Empty,
    /// The room owner's row that opens the room's settings.
    Settings,
    /// The room owner's row that shows, and steps, how many people the room seats.
    MaxPlayers,
    /// One of the owner's room settings, shown with its value and stepped in place.
    Setting(Setting),
    /// Begin the race.
    Start,
    /// Leave the screen.
    Back,
}

/// A room setting the owner steps on the room screen, after Omega's `MP Select` rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// Laps (`0` is the mode's own).
    Laps,
    /// Barrel rolls on or off.
    BarrelRolls,
    /// Whether Pilot Assist is allowed.
    PilotAssist,
    /// Whether weapons are on.
    Weapons,
    /// The zone a Zone race ends at.
    ZoneTarget,
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
    /// The owner's click on ROOM SETTINGS.
    Settings,
    /// The owner's click on MAX PLAYERS (`1`, one more, wrapping round to the fewest), or a pad's
    /// left and right on it (`-1`, `1`).
    MaxPlayers(i32),
    /// The owner's click on one of the room's settings (`1`), or left and right on it (`-1`, `1`).
    Setting(Setting, i32),
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
    /// The seats START needs filled: one, or more where the caller needs them (two for a
    /// multiplayer race).
    needs: usize,
    /// The row the pointer last moved onto, or the cursor a pad or key moved.
    hover: Option<Row>,
    /// The screen as a list to choose from rather than a lobby (LAN's host list): no START
    /// row, a seat row says its own label instead of "PLAYER n", and a confirm on a row
    /// answers [`Event::Leave`] with that row's index, which the caller reads as "chosen".
    listing: bool,
    /// What a seat row says on the left in a [`Self::listing`] screen, row for row.
    row_labels: Vec<String>,
    /// In a list, the line a row says as the hint while it is highlighted, row for row.
    notes: Vec<Option<String>>,
    /// In a list, the rows that cannot be chosen: drawn dimmed, answering nothing.
    blocked: Vec<bool>,
    /// The room owner's rows: ROOM SETTINGS, and MAX PLAYERS showing this number. `None` for
    /// everybody else, who sees neither row.
    owner_max: Option<u8>,
    /// The owner's other settings rows, each with the value it says on the right.
    owner_settings: Vec<(Setting, String)>,
}

/// Where the rows sit on the skin's grid.
pub(crate) struct Layout {
    /// Where a row's label starts.
    pub(crate) text_x: f32,
    /// Where the device column ends.
    pub(crate) right_x: f32,
    /// From one row to the next.
    pub(crate) line: f32,
    /// The text's scale.
    pub(crate) scale: f32,
    /// The top of the first row, which is what a click is measured from.
    pub(crate) top: f32,
    /// Whether the hint goes under the rows rather than between the title and the first
    /// one: where a block row would sit on the screen's rule, or where this build's own
    /// row origin leaves no room under a title (2048's placeholder menu).
    pub(crate) hint_below: bool,
    /// How far a row's text sits below its top: nothing for bare text, the tab's own
    /// pad inside a block.
    pub(crate) text_pad: f32,
    pub(crate) width: f32,
    /// The row boxes, on a title whose menus draw blocks.
    pub(crate) blocks: Option<BlockRows>,
}

/// The boxes behind the rows on HD and Omega, in the grid being drawn in. **Chosen, not
/// measured**: the screen takes HD's own `<List>` geometry (`oag_title::MenuList` and
/// `ListBlocks`: where the rows start, their pitch, their text scale, the block height and
/// the gap) and gives a label block and a value block, the value block wider than any
/// value HD's lists hold because a device's name is longer.
pub(crate) struct BlockRows {
    pub(crate) x: f32,
    pub(crate) label_width: f32,
    pub(crate) value_x: f32,
    pub(crate) value_width: f32,
    pub(crate) height: f32,
    pub(crate) scale: (f32, f32),
}

/// A value block's width in multiples of the title's own list value width.
const VALUE_WIDTHS: f32 = 2.0;
/// How far a device name sits in from its block's right edge: HD's own value inset.
const VALUE_INSET: f32 = 13.0;
/// The tab text's pad over a block's top, as a fraction of the 272-line reference screen
/// (`oag_ui::menu::Skin`'s own, 1.9).
const TEXT_TOP_PAD: f32 = 1.9;

/// How much smaller a list row's two columns are drawn than a lobby row. **Chosen**: the
/// widest face (Pulse's) fits `NAME  OTHER VERSION` beside `WIPEOUT PULSE (8/8)` at this size.
const LIST_ROW_SCALE: f32 = 0.7;

/// Where the last row may end, as a fraction of the grid's height: the footer's chrome (the
/// legend, and the ticker on Pulse) starts under it. **Chosen, not measured**, the same limit
/// the layout screen keeps its rows above ([`crate::window_layout`]).
const ROWS_BOTTOM: f32 = 0.84;
/// The smallest a crowded screen's rows are drawn, as a share of their usual size.
const MIN_FIT: f32 = 0.4;

impl Layout {
    /// These rows closed up so `rows` of them end above the footer: the pitch, the text and a
    /// block's height shrink together, and a screen whose rows already fit is left as it is.
    /// The owner of a full room has twelve (eight seats, ROOM SETTINGS, MAX PLAYERS, START,
    /// BACK), which at the usual pitch ran START and BACK off the bottom of the picture.
    pub(crate) fn fitted(mut self, rows: usize, height: f32) -> Self {
        // The hint goes under the rows on a title that draws blocks.
        let below = if self.hint_below { 0.7 } else { 0.0 };
        let need = (rows as f32 + below) * self.line;
        let room = height * ROWS_BOTTOM - self.top;
        if need <= room {
            return self;
        }
        let by = (room / need).max(MIN_FIT);
        self.line *= by;
        self.scale *= by;
        self.text_pad *= by;
        if let Some(blocks) = self.blocks.as_mut() {
            blocks.height *= by;
            blocks.scale.1 *= by;
        }
        self
    }

    pub(crate) fn new(skin: &Skin) -> Self {
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
    pub(crate) fn text_y(&self, index: usize) -> f32 {
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
            needs: 1,
            hover: None,
            listing: false,
            row_labels: Vec::new(),
            notes: Vec::new(),
            blocked: Vec::new(),
            owner_max: None,
            owner_settings: Vec::new(),
        }
    }

    /// Shows the room owner's two rows, MAX PLAYERS saying `max`; `None` hides them.
    pub fn set_owner(&mut self, max: Option<u8>) {
        self.owner_max = max;
        if !self.owner_max.is_some() && matches!(self.hover, Some(Row::Settings | Row::MaxPlayers))
        {
            self.hover = None;
        }
    }

    /// Shows the owner's settings rows, each with its value; an empty list hides them all.
    pub fn set_settings(&mut self, settings: Vec<(Setting, String)>) {
        self.owner_settings = settings;
        if let Some(Row::Setting(on)) = self.hover
            && !self.owner_settings.iter().any(|(s, _)| *s == on)
        {
            self.hover = None;
        }
    }

    /// How many seats the grid has: a room's size, which its owner can change.
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
        if let Some(Row::Seat(n)) = self.hover
            && n >= self.capacity
        {
            self.hover = None;
        }
    }

    /// The pad's left (`-1`) and right (`1`) on the highlighted row: the owner steps MAX PLAYERS.
    #[must_use]
    pub fn adjust(&self, by: i32) -> Option<Event> {
        match self.hover {
            Some(Row::MaxPlayers) => Some(Event::MaxPlayers(by)),
            Some(Row::Setting(setting)) => Some(Event::Setting(setting, by)),
            _ => None,
        }
    }

    /// This screen as a list to choose from ([`Self::listing`]), the rows' left-hand words
    /// set with [`Self::set_row_labels`].
    #[must_use]
    pub fn as_list(mut self) -> Self {
        self.listing = true;
        self
    }

    /// Replaces the line under the title: a refusal's reason, or what is happening now.
    pub fn set_hint(&mut self, hint: String) {
        self.labels.hint = hint;
    }

    /// What each row says on the left in a list, one per seat row (the device column stays
    /// the seat's own text).
    pub fn set_row_labels(&mut self, labels: Vec<String>) {
        self.row_labels = labels;
    }

    /// The line each list row says as the hint while highlighted, and which rows cannot be
    /// chosen (a blocked row is dimmed and answers nothing).
    pub fn set_notes(&mut self, notes: Vec<Option<String>>, blocked: Vec<bool>) {
        self.notes = notes;
        self.blocked = blocked;
    }

    fn note(&self, n: usize) -> Option<&str> {
        self.notes.get(n).and_then(Option::as_deref)
    }

    fn is_blocked(&self, n: usize) -> bool {
        self.blocked.get(n).copied().unwrap_or(false)
    }

    /// Whether this is the list form ([`Self::as_list`]).
    #[must_use]
    pub fn is_list(&self) -> bool {
        self.listing
    }

    /// Moves the cursor one row down (`1`) or up (`-1`), starting on the first row, and
    /// wrapping. The pad's and the keyboard's way to choose a row; the pointer sets the same
    /// highlight by hovering.
    pub fn step(&mut self, by: i32) {
        let rows = self.rows();
        let at = self
            .hover
            .and_then(|row| rows.iter().position(|r| *r == row))
            .unwrap_or(if by > 0 { rows.len() - 1 } else { 0 });
        let len = rows.len() as i32;
        let next = (at as i32 + by).rem_euclid(len) as usize;
        self.hover = Some(rows[next]);
    }

    /// What confirming the highlighted row asks for, the same answer a click on it gives.
    #[must_use]
    pub fn confirm(&self) -> Option<Event> {
        self.hover.and_then(|row| self.answer(row))
    }

    fn answer(&self, row: Row) -> Option<Event> {
        match row {
            Row::Seat(n) if self.is_blocked(n) => None,
            Row::Seat(n) => Some(Event::Leave(n)),
            Row::Empty => Some(Event::Join),
            Row::Settings => Some(Event::Settings),
            Row::MaxPlayers => Some(Event::MaxPlayers(1)),
            Row::Setting(setting) => Some(Event::Setting(setting, 1)),
            Row::Start if self.can_start() => Some(Event::Start),
            Row::Start => None,
            Row::Back => Some(Event::Back),
        }
    }

    /// This screen with START refused until `seats` are taken.
    #[must_use]
    pub fn requiring(mut self, seats: usize) -> Self {
        self.needs = seats.clamp(1, self.capacity);
        self
    }

    /// Whether START can begin the race now.
    #[must_use]
    pub fn can_start(&self) -> bool {
        self.seats.len() >= self.needs
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

    /// The rows, top to bottom: every seat, the empty one if the grid has room, the owner's
    /// ROOM SETTINGS and MAX PLAYERS, START, BACK.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        let mut rows: Vec<Row> = (0..self.seats.len()).map(Row::Seat).collect();
        if self.seats.len() < self.capacity {
            rows.push(Row::Empty);
        }
        if self.owner_max.is_some() && !self.listing {
            rows.push(Row::Settings);
            rows.push(Row::MaxPlayers);
            rows.extend(self.owner_settings.iter().map(|(s, _)| Row::Setting(*s)));
        }
        if !self.listing {
            rows.push(Row::Start);
        }
        rows.push(Row::Back);
        rows
    }

    /// How many are seated.
    #[must_use]
    pub fn count(&self) -> usize {
        self.seats.len()
    }

    /// Where this screen's rows sit: the skin's, closed up when there are too many for the
    /// picture. The drawing and the pointer's hit-test both read it, so a click lands on the
    /// row that was drawn there.
    pub(crate) fn layout(&self, skin: &Skin) -> Layout {
        Layout::new(skin).fitted(self.rows().len(), skin.space().size.1)
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
        let layout = self.layout(skin);
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
            out.extend(self.answer(row));
        }
        out
    }

    /// The boxes behind the rows, on a title whose menus draw blocks: a label block and a
    /// value block per row in the frame's ink, the hovered row's in the selected fill. Empty
    /// where the frame has no block art, which is both PSP titles: their rows are bare text.
    #[must_use]
    pub fn blocks(&self, skin: &Skin, frame: &Frame) -> Vec<Draw> {
        let rows = self.rows();
        let shape: Vec<(bool, bool)> = rows
            .iter()
            .map(|row| {
                (
                    self.hover == Some(*row),
                    // A list's TYPE AN ADDRESS row, like START and BACK, has no value.
                    !matches!(row, Row::Start | Row::Back | Row::Settings)
                        && !(self.listing && *row == Row::Empty),
                )
            })
            .collect();
        row_blocks_in(&self.layout(skin), frame, &shape)
    }

    /// The hint and the rows, drawn on the skin's grid. `frame` is the title's own, for the
    /// hint's ink on a title that draws blocks.
    #[must_use]
    pub fn body(&self, skin: &Skin, frame: &Frame) -> Vec<Draw> {
        let layout = self.layout(skin);
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
            match self.hover {
                Some(Row::Seat(n)) => self.note(n).unwrap_or(&self.labels.hint).to_string(),
                _ => self.labels.hint.clone(),
            },
        )];
        for (index, row) in rows.into_iter().enumerate() {
            let y = layout.text_y(index);
            let mut ink = if self.hover == Some(row) {
                skin.selected()
            } else {
                skin.normal()
            };
            if matches!(row, Row::Seat(n) if self.is_blocked(n)) {
                // The row the cursor is on stays readable, so the help line is plainly its own.
                ink = if self.hover == Some(row) {
                    let mut lit = ink;
                    lit[3] *= 0.75;
                    lit
                } else {
                    dim(ink)
                };
            }
            let (left, right) = match row {
                Row::Seat(n) => (
                    match self.row_labels.get(n).filter(|_| self.listing) {
                        Some(label) => label.clone(),
                        None => format!("{} {}", self.labels.player, n + 1),
                    },
                    self.seats[n].clone(),
                ),
                Row::Empty if self.listing => (self.labels.empty.clone(), String::new()),
                Row::Empty => {
                    ink = dim(ink);
                    match (&self.labels.guest, self.seats.len()) {
                        (Some(guest), 1) => (guest.clone(), String::new()),
                        _ => (
                            format!("{} {}", self.labels.player, self.seats.len() + 1),
                            self.labels.empty.clone(),
                        ),
                    }
                }
                Row::Settings => (self.labels.settings.clone(), String::new()),
                Row::Setting(setting) => (
                    match setting {
                        Setting::Laps => self.labels.laps.clone(),
                        Setting::BarrelRolls => self.labels.barrel_rolls.clone(),
                        Setting::PilotAssist => self.labels.pilot_assist.clone(),
                        Setting::Weapons => self.labels.weapons.clone(),
                        Setting::ZoneTarget => self.labels.zone_target.clone(),
                    },
                    self.owner_settings
                        .iter()
                        .find(|(s, _)| *s == setting)
                        .map(|(_, value)| value.clone())
                        .unwrap_or_default(),
                ),
                Row::MaxPlayers => (
                    self.labels.max_players.clone(),
                    self.owner_max.unwrap_or_default().to_string(),
                ),
                Row::Start => {
                    if !self.can_start() {
                        ink = dim(ink);
                    }
                    (self.labels.start.clone(), String::new())
                }
                Row::Back => (self.labels.back.clone(), String::new()),
            };
            // A list with a state and a title on every row sets them smaller, so the two columns
            // clear each other on the widest face.
            let scale = match row {
                Row::Seat(_) if self.listing && !self.row_labels.is_empty() => {
                    layout.scale * LIST_ROW_SCALE
                }
                _ => layout.scale,
            };
            out.push(text(layout.text_x, y, scale, ink, Align::Left, left));
            if !right.is_empty() {
                out.push(text(layout.right_x, y, scale, ink, Align::Right, right));
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

/// The boxes behind a list of rows on HD and Omega, one `(hovered, has a value)` per row:
/// a label block, and a value block where the row has a value, in the frame's ink and the
/// selected fill under the hovered row. Empty where the frame has no block art (both PSP
/// titles, whose rows are bare text). Shared by every screen drawn in the join screen's
/// rows ([`crate::window_layout`]).
pub(crate) fn row_blocks(skin: &Skin, frame: &Frame, rows: &[(bool, bool)]) -> Vec<Draw> {
    row_blocks_in(&Layout::new(skin), frame, rows)
}

/// [`row_blocks`] on a layout already worked out, for a screen that closes its rows up.
pub(crate) fn row_blocks_in(layout: &Layout, frame: &Frame, rows: &[(bool, bool)]) -> Vec<Draw> {
    let (Some(shape), Some(art), Some(ink)) = (layout.blocks.as_ref(), frame.blocks, frame.ink)
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (index, (hovered, has_value)) in rows.iter().copied().enumerate() {
        let color = if hovered {
            frame.tab_selected.unwrap_or(ink)
        } else {
            ink
        };
        let mut block = oag_ui::menu::block::Block {
            x: shape.x,
            y: layout.top + index as f32 * layout.line,
            width: shape.label_width,
            height: shape.height,
            color,
            landing: false,
        };
        oag_ui::menu::block::draw(&block, &art, shape.scale, &mut out);
        if !has_value {
            continue;
        }
        block.x = shape.value_x;
        block.width = shape.value_width;
        block.landing = true;
        oag_ui::menu::block::draw(&block, &art, shape.scale, &mut out);
    }
    out
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
