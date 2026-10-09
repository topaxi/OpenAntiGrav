//! The boot sequence answering a mouse or a finger.
//!
//! **Ours, with no counterpart on any disc** - see [`crate::pointer`]. Split
//! out of `frontend.rs` under the file-size rule, the way `updates.rs` is.
//!
//! Only one screen in the chain has anything to point *at*: the language
//! picker, whose rows are the `Menu` widget's. Every other screen the
//! sequence stops on - the movies, `Show Logo`, Pure's storage warning and
//! title screen - waits for a button and nothing else, so a click there is
//! the button, and that is the composition root's to synthesise: this
//! module reports the click as unhandled and `oag_game` presses start and
//! cross for it, which is what every one of those screens reads.
//!
//! # The picker's own rule: two taps
//!
//! A click on a language that is not selected selects it; a click on the
//! one already selected confirms it. Deliberately not the menus' one-click
//! activate: the choice is persisted the moment it is confirmed and the
//! screen never comes back on its own, so a finger that lands one row off
//! would otherwise race the whole game in the wrong language. The same rule
//! the selection screens use, for the same reason.

use crate::pointer::{Pointer, row_at};

use super::{Frontend, states};

/// How wide a language row's target is, in the screen's own units, at
/// scale 1. The rows have no authored width - the widget lists names of
/// any length - and this is the band the old highlight `Fill` covered, so
/// it is as wide as the picker has ever claimed to be.
pub(super) const ROW_WIDTH: f32 = 220.0;

impl Frontend {
    /// Consumes a tick of pointer input, and says whether the screen on
    /// took it.
    ///
    /// `true` means the pointer was the language picker's and has been
    /// acted on. `false` means the screen has no targets: the caller may
    /// then treat a click as the press every such screen waits for.
    pub fn pointer(&mut self, pointer: &Pointer) -> bool {
        // 2048's icon grids are the other screens with something to point
        // at - see `frontend::touch`.
        if self.touch_pointer(pointer) {
            return true;
        }
        if !self.machine.is(states::LANGUAGE_SELECTION) {
            return false;
        }
        if pointer.is_idle() || self.languages.is_empty() {
            return true;
        }
        let count = self.languages.len();
        if pointer.scroll != 0 {
            let last = count - 1;
            self.selected = usize::try_from(
                i64::try_from(self.selected).unwrap_or(0) + i64::from(pointer.scroll),
            )
            .unwrap_or(0)
            .min(last);
        }
        let Some(at) = pointer.at else {
            return true;
        };
        let row = self.language_row_at(at);
        let was = self.selected;
        if pointer.moved
            && let Some(row) = row
        {
            self.selected = row;
        }
        if pointer.clicked
            && let Some(row) = row
        {
            if row == was {
                self.confirm_language();
            } else {
                self.selected = row;
            }
        }
        true
    }

    /// Which language row `at` is over, off the same layout the rows are
    /// drawn with.
    pub(super) fn language_row_at(&self, at: (f32, f32)) -> Option<usize> {
        let screen = self.screens.language_selection()?;
        let rows = self.language_rows(screen.menu.as_ref());
        let band = self.language_band(&rows);
        row_at(
            at,
            band.left,
            band.width,
            band.top,
            rows.pitch,
            self.languages.len(),
        )
    }
}
