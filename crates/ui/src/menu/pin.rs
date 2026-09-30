//! A row pinned to one of its own values by another row: greyed, and showing
//! that value instead of the one it stores.
//!
//! The original's RACE page does this to WEAPONS - Time Trial, Speed Lap and
//! Zone grey it to `Off`, Eliminator greys it to `On`, and the player's own
//! pick comes back when a mode that allows one does (`docs/formats/race-setup.md`).
//! `disabled_by` alone would grey the row but keep drawing the stored value,
//! which is a false statement about what the race will do.

use super::{Condition, Entry, Menu, Value};

/// While `when` holds, the row is inert and shows the option stored as `shows`.
///
/// The row's own stored value is untouched: pinning is a matter of what is
/// shown and whether it can be moved, so leaving the pinned mode restores the
/// player's choice.
#[derive(Debug, Clone, PartialEq)]
pub struct Pin {
    /// When the pin applies.
    pub when: Condition,
    /// The **stored** value of the option the row shows meanwhile - what the
    /// row's own list calls it, not its label.
    pub shows: String,
}

impl Entry {
    /// This row's declared pins, in the order they are tried.
    #[must_use]
    pub fn pins(&self) -> &[Pin] {
        match self {
            Self::Choice { pins, .. } => pins,
            _ => &[],
        }
    }
}

impl Menu {
    /// The first of `entry`'s pins that applies right now, if any.
    #[must_use]
    pub fn pin<'a>(&self, entry: &'a Entry) -> Option<&'a Pin> {
        entry
            .pins()
            .iter()
            .find(|pin| pin.when.matches(self.held(&pin.when.setting).as_ref()))
    }

    /// What `entry` shows on its right-hand side: the pinned option's label
    /// while a pin applies, otherwise [`Entry::value`].
    ///
    /// A pin naming an option the list does not (yet) hold shows nothing
    /// rather than the stored value, which would be the false statement the
    /// pin exists to avoid.
    #[must_use]
    pub fn shown(&self, entry: &Entry) -> Option<Value> {
        let Some(pin) = self.pin(entry) else {
            return entry.value();
        };
        let Entry::Choice { values, .. } = entry else {
            return None;
        };
        values
            .iter()
            .find(|option| option.value == pin.shows)
            .map(|option| Value::Text(option.label.clone()))
    }
}
