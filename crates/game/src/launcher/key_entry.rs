//! The disc-key prompt: what the chooser shows for an encrypted PS3 image no
//! key opens.
//!
//! A PS3 disc is read encrypted in place (`oag_disc::ps3_crypt`), and the one
//! thing it needs that this project cannot ship is the disc's own 16-byte key.
//! Players normally put it beside the image as a `.dkey`; this is the other
//! way in, for a build where that is awkward (a phone, a handheld).
//!
//! # Input
//!
//! - **A hex keypad**, driven by the d-pad and Cross or by a pointer or finger.
//!   It needs no text input method, so it is the one path that works everywhere,
//!   including Android's `NativeActivity`, where winit delivers no IME text.
//! - **A desk keyboard** types hex digits and backspace straight in
//!   (`crate::typing` already folds case and filters to the grid's alphabet).
//! - **Paste** (Ctrl+V, or the PASTE cell) on desktop builds, which read the
//!   system clipboard. A pasted key may carry spaces, dashes or a `0x` prefix.
//!   There is no clipboard path on Android in this build, so the PASTE cell is
//!   not drawn there; use the keypad, or a key file beside the image.
//!
//! The layout is ours: **chosen, not measured**.
//!
//! # What is stored
//!
//! A key is checked against the image first (`oag_disc::ps3_crypt::key_opens`:
//! the same oracle a load uses) and written only if it opens it, to the app's
//! own keys directory, `<config>/oag/keys/<image name>.dkey`. A wrong key is
//! reported and never written, and nothing here ever prints a key.

use std::path::{Path, PathBuf};

use oag_disc::ps3_crypt::{self, DiscKey};
use oag_gameplay::input::{Button, Input};
use oag_ui::frontend::Draw;

use super::{DIM, HEADING, HIGHLIGHT, MARGIN, SELECTED, TEXT, UNAVAILABLE_COLOUR, text};
use oag_display::space::SCREEN;

/// Hex digits in a sector key.
pub const DIGITS: usize = 32;

/// One cell of the keypad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// Types this digit.
    Hex(char),
    /// Removes the last digit.
    Delete,
    /// Empties the buffer.
    Clear,
    /// Reads the clipboard (desktop builds only).
    Paste,
    /// Checks and stores the key.
    Accept,
    /// Leaves without storing.
    Cancel,
}

/// What a tick of the prompt did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Still open.
    Pending,
    /// The player backed out.
    Cancelled,
    /// The key opened the image and was stored.
    Stored,
    /// The player asked to paste: the caller reads the clipboard and calls
    /// [`KeyEntry::paste`].
    WantsPaste,
}

/// The prompt's state: a hex buffer, a cursor and a one-line message.
#[derive(Debug, Clone)]
pub struct KeyEntry {
    image: PathBuf,
    digits: String,
    cell: usize,
    message: Option<String>,
    can_paste: bool,
}

const HEX_COLUMNS: usize = 8;
const HEX_CELLS: usize = 16;
const ACTIONS: [Cell; 5] = [
    Cell::Delete,
    Cell::Clear,
    Cell::Paste,
    Cell::Accept,
    Cell::Cancel,
];

/// Where the pad starts, and the size of one hex cell.
const PAD_TOP: f32 = 112.0;
const CELL_W: f32 = 48.0;
const CELL_H: f32 = 22.0;
const ACTION_W: f32 = 76.0;

impl KeyEntry {
    /// A prompt for `image`. `can_paste` is whether this build can read a
    /// clipboard; without it the PASTE cell is neither drawn nor reachable.
    #[must_use]
    pub fn new(image: &Path, can_paste: bool) -> Self {
        Self {
            image: image.to_path_buf(),
            digits: String::new(),
            cell: 0,
            message: None,
            can_paste,
        }
    }

    /// The image this key is for.
    #[must_use]
    pub fn image(&self) -> &Path {
        &self.image
    }

    /// How many digits are in the buffer.
    #[must_use]
    pub fn len(&self) -> usize {
        self.digits.len()
    }

    /// Whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.digits.is_empty()
    }

    /// The line under the buffer, if there is one.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Whether the PASTE cell is on offer.
    #[must_use]
    pub fn can_paste(&self) -> bool {
        self.can_paste
    }

    fn actions(&self) -> Vec<Cell> {
        ACTIONS
            .into_iter()
            .filter(|c| self.can_paste || *c != Cell::Paste)
            .collect()
    }

    /// Every cell, in reading order.
    #[must_use]
    pub fn cells(&self) -> Vec<Cell> {
        let mut cells: Vec<Cell> = "0123456789abcdef".chars().map(Cell::Hex).collect();
        cells.extend(self.actions());
        cells
    }

    /// The cell the cursor is on.
    #[must_use]
    pub fn selected(&self) -> Cell {
        self.cells()[self.cell]
    }

    /// Adds one digit. Anything that is not hex, and anything past 32, is
    /// ignored.
    pub fn type_digit(&mut self, c: char) {
        if c.is_ascii_hexdigit() && self.digits.len() < DIGITS {
            self.digits.push(c.to_ascii_lowercase());
            self.message = None;
        }
    }

    /// Removes the last digit.
    pub fn delete(&mut self) {
        self.digits.pop();
        self.message = None;
    }

    /// Takes the clipboard's text: a whole key in any of the forms
    /// [`DiscKey::parse`] reads replaces the buffer; text made only of hex
    /// digits (and separators) is appended, up to 32; anything else, prose
    /// included, is refused whole rather than mined for digits.
    pub fn paste(&mut self, clipboard: &str) {
        self.message = None;
        if let Some(key) = DiscKey::parse(clipboard.as_bytes()) {
            self.digits = key.to_hex();
            return;
        }
        let digits: Vec<char> = clipboard
            .chars()
            .filter(|c| !c.is_whitespace() && !matches!(c, '-' | ':'))
            .collect();
        if digits.is_empty() || !digits.iter().all(char::is_ascii_hexdigit) {
            self.message = Some("THE CLIPBOARD HOLDS NO KEY".to_string());
            return;
        }
        for c in digits {
            self.type_digit(c);
        }
    }

    /// Checks the buffer with `opens` and, if it passes, hands the key to
    /// `store`. The seam exists so a test needs no disc.
    pub fn submit_with(
        &mut self,
        opens: impl FnOnce(&DiscKey) -> bool,
        store: impl FnOnce(&DiscKey) -> std::io::Result<()>,
    ) -> Outcome {
        let Some(key) = DiscKey::parse(self.digits.as_bytes()) else {
            self.message = Some(format!(
                "A KEY IS {DIGITS} DIGITS - YOU HAVE {}",
                self.len()
            ));
            return Outcome::Pending;
        };
        if !opens(&key) {
            self.message = Some("THAT KEY DOES NOT OPEN THIS DISC".to_string());
            return Outcome::Pending;
        }
        match store(&key) {
            Ok(()) => Outcome::Stored,
            Err(_) => {
                self.message = Some("COULD NOT SAVE THE KEY".to_string());
                Outcome::Pending
            }
        }
    }

    fn submit(&mut self) -> Outcome {
        let image = self.image.clone();
        let name = image
            .file_stem()
            .map_or_else(|| "disc".to_string(), |s| s.to_string_lossy().into_owned());
        self.submit_with(
            |key| ps3_crypt::key_opens(&image, key).unwrap_or(false),
            |key| ps3_crypt::store_key(key, &name).map(drop),
        )
    }

    fn press(&mut self) -> Outcome {
        match self.selected() {
            Cell::Hex(c) => self.type_digit(c),
            Cell::Delete => self.delete(),
            Cell::Clear => {
                self.digits.clear();
                self.message = None;
            }
            Cell::Paste => return Outcome::WantsPaste,
            Cell::Accept => return self.submit(),
            Cell::Cancel => return Outcome::Cancelled,
        }
        Outcome::Pending
    }

    /// A desk keyboard's Enter.
    pub fn accept(&mut self) -> Outcome {
        self.submit()
    }

    /// One tick of pad input.
    pub fn update(&mut self, input: &mut Input) -> Outcome {
        let count = self.cells().len();
        let actions = count - HEX_CELLS;
        let (row, column) = self.position();
        let (mut row, mut column) = (row, column);
        if input.take(Button::Right) {
            column = (column + 1) % self.row_len(row, actions);
        }
        if input.take(Button::Left) {
            let n = self.row_len(row, actions);
            column = (column + n - 1) % n;
        }
        if input.take(Button::Down) {
            row = (row + 1) % 3;
        }
        if input.take(Button::Up) {
            row = (row + 2) % 3;
        }
        self.cell = Self::index(row, column.min(self.row_len(row, actions) - 1));

        if input.take(Button::Square) {
            self.delete();
        }
        if input.take(Button::Circle) {
            return Outcome::Cancelled;
        }
        if input.take(Button::Start) {
            return self.submit();
        }
        if input.take(Button::Cross) {
            return self.press();
        }
        Outcome::Pending
    }

    fn row_len(&self, row: usize, actions: usize) -> usize {
        if row < 2 { HEX_COLUMNS } else { actions }
    }

    fn position(&self) -> (usize, usize) {
        if self.cell < HEX_CELLS {
            (self.cell / HEX_COLUMNS, self.cell % HEX_COLUMNS)
        } else {
            (2, self.cell - HEX_CELLS)
        }
    }

    fn index(row: usize, column: usize) -> usize {
        if row < 2 {
            row * HEX_COLUMNS + column
        } else {
            HEX_CELLS + column
        }
    }

    /// The `[x, y, w, h]` of cell `index`, shared by the draw and the hit test.
    fn rect(&self, index: usize) -> [f32; 4] {
        if index < HEX_CELLS {
            let (row, column) = (index / HEX_COLUMNS, index % HEX_COLUMNS);
            [
                MARGIN + column as f32 * CELL_W,
                PAD_TOP + row as f32 * CELL_H,
                CELL_W,
                CELL_H,
            ]
        } else {
            [
                MARGIN + (index - HEX_CELLS) as f32 * ACTION_W,
                PAD_TOP + 2.0 * CELL_H + 6.0,
                ACTION_W,
                CELL_H,
            ]
        }
    }

    fn cell_at(&self, at: (f32, f32)) -> Option<usize> {
        (0..self.cells().len()).find(|&i| {
            let [x, y, w, h] = self.rect(i);
            at.0 >= x && at.0 < x + w && at.1 >= y && at.1 < y + h
        })
    }

    /// One tick of pointer input: hover moves the cursor, a click presses a
    /// cell, the secondary button cancels. A click off the pad does nothing.
    pub fn pointer(&mut self, pointer: &oag_ui::pointer::Pointer) -> Outcome {
        if pointer.is_idle() {
            return Outcome::Pending;
        }
        if pointer.back {
            return Outcome::Cancelled;
        }
        let Some(cell) = pointer.at.and_then(|at| self.cell_at(at)) else {
            return Outcome::Pending;
        };
        if pointer.moved {
            self.cell = cell;
        }
        if pointer.clicked {
            self.cell = cell;
            return self.press();
        }
        Outcome::Pending
    }

    /// What the prompt looks like, in the chooser's 480x272 grid.
    #[must_use]
    pub fn draw(&self) -> Vec<Draw> {
        let mut out = vec![Draw::Fill {
            rect: [0.0, 0.0, SCREEN.0, SCREEN.1],
            color: super::BACKDROP,
        }];
        out.push(text(MARGIN, 24.0, 1.0, HEADING, "DISC KEY"));
        let name = self
            .image
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        out.push(text(MARGIN, 40.0, 1.0, DIM, &format!("FOR {name}")));
        out.push(text(
            MARGIN,
            54.0,
            1.0,
            TEXT,
            "ENTER THE 32 HEX DIGITS OF THIS DISC'S OWN KEY",
        ));

        let grouped: Vec<String> = (0..4)
            .map(|g| {
                let mut group: String = self.digits.chars().skip(g * 8).take(8).collect();
                while group.len() < 8 {
                    group.push('.');
                }
                group
            })
            .collect();
        out.push(text(MARGIN, 76.0, 1.5, SELECTED, &grouped.join(" ")));
        let count = format!("{}/{DIGITS}", self.len());
        out.push(text(MARGIN, 94.0, 1.0, DIM, &count));
        if let Some(message) = &self.message {
            out.push(text(MARGIN + 60.0, 94.0, 1.0, UNAVAILABLE_COLOUR, message));
        }

        for (index, cell) in self.cells().into_iter().enumerate() {
            let [x, y, w, h] = self.rect(index);
            let selected = index == self.cell;
            if selected {
                out.push(Draw::Fill {
                    rect: [x, y, w - 2.0, h - 2.0],
                    color: HIGHLIGHT,
                });
            }
            let label = match cell {
                Cell::Hex(c) => c.to_ascii_uppercase().to_string(),
                Cell::Delete => "DEL".to_string(),
                Cell::Clear => "CLEAR".to_string(),
                Cell::Paste => "PASTE".to_string(),
                Cell::Accept => "OK".to_string(),
                Cell::Cancel => "CANCEL".to_string(),
            };
            out.push(text(
                x + 6.0,
                y + 6.0,
                1.0,
                if selected { SELECTED } else { TEXT },
                &label,
            ));
        }

        let hint = if self.can_paste {
            "PAD OR TOUCH THE KEYS   TYPE OR CTRL+V   ENTER OK   ESC CANCEL"
        } else {
            "PAD OR TOUCH THE KEYS   OR A .DKEY BESIDE THE IMAGE"
        };
        out.push(text(MARGIN, SCREEN.1 - 20.0, 1.0, DIM, hint));
        out
    }
}

#[cfg(test)]
mod tests;
