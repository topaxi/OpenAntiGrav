//! Modal overlays over the menus: an on-screen [`Keyboard`], and a yes/no
//! [`Confirm`].
//!
//! Like [`crate::menu`], this holds no GPU handles, opens no files and reads
//! no clock. It takes an [`Input`] snapshot in and emits an [`Outcome`] and a
//! list of [`Draw`]s out, both plain data, so every one of it can be driven
//! headlessly in a unit test. It moves to `oag-ui` alongside `menu` when that
//! crate exists - see `docs/architecture/workspace-layout.md`.
//!
//! # Why a keyboard exists here at all
//!
//! Nothing in this project could accept typed text before this module. The
//! whole menu layer is rows and abstract buttons, so a feature that needs a
//! *name* - renaming a pilot, and eventually a profile, a replay or a saved
//! setup - had no way to ask for one. A physical keyboard is not the answer
//! on its own: `oag_input` maps real devices onto an abstract button layer
//! precisely so a pad works everywhere a keyboard does, and a text field only
//! a keyboard can fill would be the first thing in the build that a pad
//! cannot reach.
//!
//! So this is **pad-first**: a grid of keys stepped with the d-pad, typed with
//! Cross, deleted with Square, accepted with Start or the grid's own accept
//! key, cancelled with Circle. A desk keyboard types into it directly as well
//! - that is the binary's own `typing` module, which turns a raw key event
//! into an [`Edit`] and is deliberately a separate, testable decision from
//! this model, the same way `rebind` is separate from the CONTROLS page.
//!
//! # Nothing here is recovered, and that is checked rather than assumed
//!
//! **The layout below is ours: chosen, not measured, and carries no
//! confidence score.** The PSP does not let a game author one - text entry on
//! that machine is the *firmware* `sceUtilityOsk` dialog, drawn by the system
//! rather than by the title - so there is no keyboard on the disc to recover
//! and no key glyph to draw with. That is a recorded negative result rather
//! than an unchecked assumption; see
//! `handover/players-should-be-able-to-edit-pilots-in-game.md`.
//!
//! # Every label is passed in, already resolved
//!
//! [`Labels`] and [`ConfirmLabels`] are plain `String`s the caller has already
//! looked up in its own [`StringTable`](crate::language::StringTable). This
//! module holds no table and no literal English of its own beyond the key
//! glyphs, which are the characters a name may contain and are not language.
//! It is the same seam [`crate::menu::Choice::labelled`] draws - what a row
//! *stores* against what it *shows* - one layer up.

use crate::frontend::{Align, Draw};
use crate::input::{Button, Input};
use crate::menu::Skin;

/// What a prompt did on a tick.
///
/// Deliberately carries no payload: what was typed is [`Keyboard::text`], read
/// off the model the caller already holds. A payload here would be a second
/// copy of the buffer, which is the mistake `session/pilot_editor.rs`'s own
/// module doc explains at length for the pilot rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Still on screen; nothing to do.
    Pending,
    /// The player accepted. Read [`Keyboard::text`] or take [`Confirm`]'s yes.
    Accepted,
    /// The player backed out. Change nothing.
    Cancelled,
}

/// One cell of the [`Keyboard`]'s grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Types this character.
    Char(char),
    /// Removes the last character typed.
    Delete,
    /// Finishes, the same as Start.
    Accept,
}

/// The characters the grid offers, in the order they are laid out.
///
/// **Exactly the set [`crate::pilots::check_name`] allows**, which is the
/// point: a grid that offered a space or a slash would be offering a key that
/// makes the name it is building invalid, and a player would find that out
/// only on pressing accept. It is not a claim that every future caller wants
/// the same set - when one wants uppercase or punctuation this becomes a
/// parameter, and it is cheap to make one then rather than to guess now.
const KEYS: &str = "abcdefghijklmnopqrstuvwxyz0123456789-_";

/// How many cells a row of the grid holds.
///
/// **Chosen, not measured.** Ten by ten-times-four arithmetic: [`KEYS`] is 38
/// characters and the two special keys make 40, which is four full rows of ten
/// with nothing left ragged. Ten cells also still fit across the 480-wide
/// screen at the measured menu scale with room for a 3-character label in the
/// widest of them.
const COLUMNS: usize = 10;

/// Every cell, characters first and the two special keys last.
const CELLS: usize = KEYS.len() + 2;

/// How many rows of [`COLUMNS`] the grid has.
const GRID_ROWS: usize = CELLS.div_ceil(COLUMNS);

/// What the cell at `index` does.
#[must_use]
fn key_at(index: usize) -> Key {
    match KEYS.chars().nth(index) {
        Some(c) => Key::Char(c),
        None if index == KEYS.len() => Key::Delete,
        _ => Key::Accept,
    }
}

/// The text a [`Keyboard`] draws that is not a key: already resolved, never
/// looked up here. See this module's own doc.
#[derive(Debug, Clone, Default)]
pub struct Labels {
    /// What is being named - "RENAME PILOT".
    pub title: String,
    /// The delete key's own label.
    pub delete: String,
    /// The accept key's own label.
    pub accept: String,
    /// One line under the grid saying which button does what. Drawn only when
    /// it is not empty, so a caller with nothing to say says nothing.
    pub hint: String,
}

/// An on-screen keyboard: a grid of keys, a buffer, and a note.
///
/// Reusable by construction - it knows nothing about pilots, or about what the
/// text it is building will be used for. See this module's doc.
#[derive(Debug, Clone)]
pub struct Keyboard {
    labels: Labels,
    text: String,
    /// A live remark about what is currently typed - "this replaces the
    /// built-in". Set from outside every tick, because whether there is
    /// anything to say depends on what the text *means*, which is exactly what
    /// this type does not know.
    note: Option<String>,
    /// How many characters the buffer may hold.
    limit: usize,
    /// Which cell the cursor is on, indexing [`key_at`].
    cell: usize,
}

impl Keyboard {
    /// A keyboard opened on `initial`, holding at most `limit` characters.
    ///
    /// `initial` is **not** filtered against [`KEYS`]: what a caller seeds it
    /// with is that caller's business, and silently dropping characters from a
    /// name a player is looking at would be worse than showing them a name
    /// they must now edit. It *is* truncated to `limit`, because a buffer over
    /// its own limit could never be got back under one except by deleting.
    #[must_use]
    pub fn new(labels: Labels, initial: &str, limit: usize) -> Self {
        Self {
            labels,
            text: initial.chars().take(limit).collect(),
            note: None,
            limit: limit.max(1),
            cell: 0,
        }
    }

    /// What has been typed so far.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Sets, or clears, the live remark drawn under the buffer.
    ///
    /// Called every tick by whoever knows what the text means. Cheap, and
    /// deliberately not a callback: a closure here would have to borrow the
    /// caller's own state for as long as the prompt was open.
    pub fn set_note(&mut self, note: Option<String>) {
        self.note = note;
    }

    /// The cell the cursor is on, for a test and for the draw list.
    #[must_use]
    pub fn selected(&self) -> Key {
        key_at(self.cell)
    }

    /// Applies one edit, from the grid or from a desk keyboard alike.
    ///
    /// Public because the binary's own `typing` module produces exactly this
    /// from a raw key event: one place decides what an edit *is*, and two
    /// devices may say one happened.
    pub fn edit(&mut self, edit: Edit) {
        match edit {
            Edit::Type(c) => {
                if self.text.chars().count() < self.limit {
                    self.text.push(c);
                }
            }
            Edit::Delete => {
                self.text.pop();
            }
        }
    }

    /// Consumes a tick of input and says what happened.
    ///
    /// **Every button is taken as an edge**, exactly as
    /// [`crate::menu::Menu::update`] does and for a sharper version of the
    /// same reason: the Cross that opened this prompt and the Cross that
    /// closes it are one press each, and a prompt that only *peeked* would
    /// accept on the very edge that opened it and reopen forever.
    pub fn update(&mut self, input: &mut Input) -> Outcome {
        let (mut row, mut column) = (self.cell / COLUMNS, self.cell % COLUMNS);
        if input.take(Button::Right) {
            column = (column + 1) % COLUMNS;
        }
        if input.take(Button::Left) {
            column = (column + COLUMNS - 1) % COLUMNS;
        }
        if input.take(Button::Down) {
            row = (row + 1) % GRID_ROWS;
        }
        if input.take(Button::Up) {
            row = (row + GRID_ROWS - 1) % GRID_ROWS;
        }
        // A grid whose last row is short would otherwise let the cursor land
        // on a cell that is not there. `CELLS` is a whole number of rows
        // today, so this is the guard that keeps changing `KEYS` from being a
        // panic rather than a ragged bottom row.
        self.cell = (row * COLUMNS + column).min(CELLS - 1);

        // A pad shortcut for the key the cursor would otherwise have to travel
        // to. Ours, and the one convenience worth having: deleting a typo is
        // the most repeated action on any on-screen keyboard, and making it a
        // round trip across the grid every time is what makes one tedious.
        if input.take(Button::Square) {
            self.edit(Edit::Delete);
        }
        if input.take(Button::Circle) {
            return Outcome::Cancelled;
        }
        // Start accepts wherever the cursor is, the same shorthand
        // `Menu::update` gives it for activating a row.
        if input.take(Button::Start) {
            return Outcome::Accepted;
        }
        if input.take(Button::Cross) {
            match self.selected() {
                Key::Char(c) => self.edit(Edit::Type(c)),
                Key::Delete => self.edit(Edit::Delete),
                Key::Accept => return Outcome::Accepted,
            }
        }
        Outcome::Pending
    }

    /// What this looks like, over whatever is already on screen.
    ///
    /// `skin` is where every colour and the row pitch come from, the same way
    /// [`crate::menu::rows`] reads them, so a title with a taller face gets a
    /// taller keyboard rather than one drawn at PSP numbers on an HD screen.
    /// The *arrangement* is this module's own and is marked as such
    /// throughout.
    #[must_use]
    pub fn draw(&self, skin: &Skin) -> Vec<Draw> {
        let panel = Panel::new(skin);
        let mut out = panel.frame(&self.labels.title);

        let line = panel.line;
        // The buffer, drawn larger than a row because it is the one thing on
        // this screen the player is actually building.
        let mut y = panel.body_y;
        out.push(Draw::Text {
            x: panel.text_x,
            y,
            scale: panel.scale * 1.4,
            color: skin.selected(),
            border: None,
            align: Align::Left,
            // A caret, so an empty buffer is visibly empty rather than
            // indistinguishable from a missing one.
            text: format!("{}_", self.text),
            wrap_width: None,
        });
        y += line * 1.3;
        if let Some(note) = &self.note {
            out.push(Draw::Text {
                x: panel.text_x,
                y,
                scale: panel.scale * 0.8,
                color: NOTE,
                border: None,
                align: Align::Left,
                text: note.clone(),
                wrap_width: Some(panel.text_width),
            });
        }
        y += line;

        let cell_w = panel.text_width / COLUMNS as f32;
        let cell_h = line * 0.95;
        for index in 0..CELLS {
            let (row, column) = (index / COLUMNS, index % COLUMNS);
            let x = panel.text_x + column as f32 * cell_w;
            let top = y + row as f32 * cell_h;
            let selected = index == self.cell;
            if selected {
                // **A box, not a brightened glyph - chosen, not measured.**
                // `menu::rows` deliberately draws no fill behind the selected
                // row, because a capture of the original shows none and a
                // column of seven rows reads fine on brightness alone. Forty
                // cells in a grid do not: finding the one brighter glyph among
                // them is a search, and a box is the cheapest thing that makes
                // it a glance. Nothing on the disc is being contradicted here,
                // because the disc has no grid.
                out.push(Draw::Fill {
                    rect: [x, top, cell_w - 2.0, cell_h - 2.0],
                    color: CURSOR,
                });
            }
            let label = match key_at(index) {
                Key::Char(c) => c.to_string(),
                Key::Delete => self.labels.delete.clone(),
                Key::Accept => self.labels.accept.clone(),
            };
            out.push(Draw::Text {
                x: x + cell_w * 0.5,
                y: top + cell_h * 0.15,
                scale: panel.scale,
                color: if selected {
                    skin.selected()
                } else {
                    skin.normal()
                },
                border: None,
                align: Align::Centre,
                text: label,
                wrap_width: None,
            });
        }

        if !self.labels.hint.is_empty() {
            out.push(Draw::Text {
                x: panel.text_x,
                y: y + GRID_ROWS as f32 * cell_h + line * 0.3,
                scale: panel.scale * 0.8,
                color: skin.normal(),
                border: None,
                align: Align::Left,
                text: self.labels.hint.clone(),
                wrap_width: Some(panel.text_width),
            });
        }
        out
    }
}

/// One change to a [`Keyboard`]'s buffer, whichever device asked for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    /// Append this character.
    Type(char),
    /// Remove the last one.
    Delete,
}

/// The text a [`Confirm`] draws. See [`Labels`].
#[derive(Debug, Clone, Default)]
pub struct ConfirmLabels {
    /// What is being asked - "DELETE PILOT".
    pub title: String,
    /// The whole question, and anything the player needs to know before
    /// answering it. Wrapped to the panel.
    pub message: String,
    /// The affirmative answer's label.
    pub yes: String,
    /// The negative answer's label.
    pub no: String,
}

/// A yes/no question with a message the caller wrote.
///
/// Separate from [`Keyboard`] rather than a mode of it: they share a panel and
/// nothing else, and a type that was sometimes a grid and sometimes two
/// buttons would have a `text()` that meant nothing half the time.
#[derive(Debug, Clone)]
pub struct Confirm {
    labels: ConfirmLabels,
    /// Which answer the cursor is on.
    yes: bool,
}

impl Confirm {
    /// A question, opened on **no**.
    ///
    /// Ours, and deliberate: everything this is used for so far destroys
    /// something, and a dialog that opens on the destructive answer turns a
    /// stray Cross into a deleted file. A caller that wants the other default
    /// can say so, and none does.
    #[must_use]
    pub fn new(labels: ConfirmLabels) -> Self {
        Self { labels, yes: false }
    }

    /// Which answer the cursor is on, for a test and for the draw list.
    #[must_use]
    pub fn on_yes(&self) -> bool {
        self.yes
    }

    /// Consumes a tick of input. See [`Keyboard::update`] on edges.
    pub fn update(&mut self, input: &mut Input) -> Outcome {
        // Both axes move it: two answers side by side answer to left and
        // right, and a player whose thumb goes down on a menu has not asked
        // for nothing to happen - the same argument `Menu::update` makes for
        // up and down on a horizontal strip.
        for button in [Button::Left, Button::Right, Button::Up, Button::Down] {
            if input.take(button) {
                self.yes = !self.yes;
            }
        }
        if input.take(Button::Circle) {
            return Outcome::Cancelled;
        }
        if input.take(Button::Cross) || input.take(Button::Start) {
            return if self.yes {
                Outcome::Accepted
            } else {
                Outcome::Cancelled
            };
        }
        Outcome::Pending
    }

    /// What this looks like, over whatever is already on screen.
    #[must_use]
    pub fn draw(&self, skin: &Skin) -> Vec<Draw> {
        let panel = Panel::new(skin);
        let mut out = panel.frame(&self.labels.title);
        out.push(Draw::Text {
            x: panel.text_x,
            y: panel.body_y,
            scale: panel.scale,
            color: skin.normal(),
            border: None,
            align: Align::Left,
            text: self.labels.message.clone(),
            wrap_width: Some(panel.text_width),
        });
        // Under the message rather than under a fixed number of its lines: the
        // wrap is the renderer's, so this module cannot know how tall the
        // message came out. Parked at a height that clears the longest message
        // anything currently asks - the built-in-restore sentence - which is a
        // limitation worth naming rather than hiding. See this module's doc.
        let y = panel.body_y + panel.line * 4.0;
        for (index, (label, is_yes)) in [(&self.labels.no, false), (&self.labels.yes, true)]
            .into_iter()
            .enumerate()
        {
            let x = panel.text_x + index as f32 * panel.text_width * 0.5;
            let selected = is_yes == self.yes;
            if selected {
                out.push(Draw::Fill {
                    rect: [x - 4.0, y, panel.text_width * 0.4, panel.line],
                    color: CURSOR,
                });
            }
            out.push(Draw::Text {
                x,
                y: y + panel.line * 0.15,
                scale: panel.scale,
                color: if selected {
                    skin.selected()
                } else {
                    skin.normal()
                },
                border: None,
                align: Align::Left,
                text: label.clone(),
                wrap_width: None,
            });
        }
        out
    }
}

/// What everything outside the panel is dimmed with.
///
/// **Ours, chosen not measured**, on the same footing as `menu_stage`'s own
/// `PAUSE_OVERLAY` and for the same reason: neither PSP title's front-end XML
/// defines a modal dialog to read a tint off, so there is nothing to recover.
/// Darker than the pause overlay, because what is behind this is a menu the
/// player must not mistake for the one still taking input.
const SCRIM: [f32; 4] = [0.0, 0.0, 0.0, 0.72];

/// The panel's own fill. Ours; see [`SCRIM`].
const PANEL: [f32; 4] = [0.04, 0.06, 0.09, 0.94];

/// The box behind the cell or answer the cursor is on. Ours; see
/// [`Keyboard::draw`] for why this exists at all when a menu row has none.
const CURSOR: [f32; 4] = [0.20, 0.42, 0.62, 0.85];

/// A live remark about what has been typed - amber, the same channel
/// `menu::rows`' own `WARNING` uses, and for the same reason: the other
/// colours already mean selected, normal and inert.
const NOTE: [f32; 4] = [1.0, 0.76, 0.25, 1.0];

/// The box both prompts are drawn in, and where their text goes inside it.
///
/// Every number is a fraction of the screen the skin is already working in
/// rather than a PSP pixel count, so an HD skin gets a panel of the same
/// shape rather than a PSP-sized one in a corner. **Ours throughout.**
struct Panel {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    text_x: f32,
    text_width: f32,
    /// Where the first line under the title goes.
    body_y: f32,
    /// One line of text, from the skin's own row pitch.
    line: f32,
    scale: f32,
}

impl Panel {
    fn new(skin: &Skin) -> Self {
        let (screen_w, screen_h) = skin.space().size;
        let line = skin.row_pitch();
        let (x, y) = (screen_w * 0.08, screen_h * 0.07);
        let (width, height) = (screen_w * 0.84, screen_h * 0.86);
        let pad = screen_w * 0.04;
        Self {
            x,
            y,
            width,
            height,
            text_x: x + pad,
            text_width: width - pad * 2.0,
            body_y: y + line * 1.4,
            line,
            scale: skin.row_scale(),
        }
    }

    /// The scrim, the panel and the title: what both prompts start with.
    fn frame(&self, title: &str) -> Vec<Draw> {
        vec![
            Draw::Fill {
                // Deliberately far larger than the screen: `Renderer` letterboxes
                // this list into the window, and a scrim sized to exactly the
                // grid would leave live menu showing in the bars whenever the
                // player's ASPECT setting disagrees with the title's own shape -
                // the bug `menu_stage::overlay_rect` was written to fix for the
                // pause tint. Overdrawing costs nothing and cannot be wrong.
                rect: [-4000.0, -4000.0, 8000.0, 8000.0],
                color: SCRIM,
            },
            Draw::Fill {
                rect: [self.x, self.y, self.width, self.height],
                color: PANEL,
            },
            Draw::Text {
                x: self.text_x,
                y: self.y + self.line * 0.25,
                scale: self.scale,
                color: [1.0, 1.0, 1.0, 1.0],
                border: None,
                align: Align::Left,
                text: title.to_string(),
                wrap_width: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests;
