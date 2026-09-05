//! The rebinding table: which key produces which abstract button, once a
//! player has moved one off [`crate::keys`]'s built-in layout.
//!
//! Kept a **permutation over the closed candidate set** rather than an open
//! table of arbitrary keys: every key this build can ever offer a player is
//! already named by [`crate::keys::candidates`], and rebinding only ever
//! changes which button a name currently points at. That is what keeps
//! `Vec<&'static str>` in [`crate::keys::bound_keys`]'s shape unchanged -
//! [`Bindings::names_for`] answers with the same static strings, just read
//! off a table a player has moved.
//!
//! # Conflict is a steal, not a refusal
//!
//! [`Bindings::resolve`] returns `Option<Button>` - **one** button, or none -
//! so a key cannot mean two things at once. [`Bindings::rebind`] therefore
//! takes a key away from whatever button held it before handing it to the one
//! being edited, rather than refusing the rebind: refusing would dead-end a
//! player against eighteen keys and twelve buttons with no way to free one up
//! except backing out and finding the other row first. A button a rebind
//! leaves with nothing is not a failure state - [`crate::keys::bound_keys`]'s
//! own doc comment already calls that "a real answer rather than a failure",
//! and [`Bindings::names_for`] answers the same way.
//!
//! # A rebind replaces, it does not add
//!
//! [`Bindings::rebind`] gives `button` **exactly** the one key just pressed,
//! clearing every other key it held. There is no menu affordance to remove a
//! key one at a time, so appending would only ever grow a row's list - a
//! one-way ratchet where every rebind after the first makes the display
//! longer instead of showing what the player just set. Replacing keeps
//! [`Bindings::names_for`] equal to the single key most recently pressed for
//! that row, which is what a binding row is showing a value *of*.
//!
//! # Persistence
//!
//! [`Bindings::to_pairs`] and [`Bindings::from_pairs`] round-trip through
//! `oag_game::settings::Controls::bindings`, a `BTreeMap<String, String>` -
//! this crate depends on nothing serde-shaped, the same reason
//! `oag_game::settings`'s `AnisotropyDef`/`LodDef` mirror types they do not
//! own rather than deriving here. Every one of [`crate::keys::candidates`]'s
//! eighteen names is always written, complete, the same convention
//! `settings::load`'s own doc comment states for the rest of the file - so a
//! button a rebind emptied stays empty across a restart instead of quietly
//! reverting to its default the moment the key that used to own it goes
//! missing from the file.
//!
//! An entry [`Bindings::from_pairs`] cannot parse - a name this build does not
//! offer, or a value that is not a button name or the `"none"` sentinel - is
//! **dropped and the default kept for that candidate**, never a load failure:
//! a settings file travels between builds, and `oag_game::main::args`'s
//! `resolve_scheme`/`resolve_triggers` already treat every other control token
//! this way for the same reason. [`Bindings::from_pairs`] returns the names it
//! had to fall back on, so a caller that wants to say so can.

use winit::keyboard::Key;

use oag_gameplay::input::{self, Button};

use crate::keys;

/// A live key-to-button table: [`crate::keys::candidates`]'s default,
/// permuted by whatever a player has rebound.
///
/// `table[i]` is what candidate `i` of [`crate::keys::candidates`] currently
/// produces - `None` once every button that used to claim it has had it stolen
/// away by a later rebind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bindings {
    table: Vec<Option<Button>>,
}

/// The sentinel [`Bindings::to_pairs`] writes for a candidate no button
/// currently owns, and [`Bindings::from_pairs`] reads back the same way.
///
/// Never a button's own name: [`oag_gameplay::input::Button::Display`] never
/// prints it, `Any` included, so a stored `"none"` cannot collide with a real
/// button.
const UNBOUND: &str = "none";

impl Default for Bindings {
    /// [`crate::keys::map_key`]'s own table, one row per candidate - what a
    /// fresh install, and every candidate a rebind has not yet touched, uses.
    fn default() -> Self {
        Self {
            table: keys::candidates()
                .into_iter()
                .map(|(_, key)| keys::map_key(&key))
                .collect(),
        }
    }
}

impl Bindings {
    /// Which button `key` produces, if any.
    ///
    /// What a live [`crate::Keyboard`] resolves a key event through, in place
    /// of [`crate::keys::map_key`]'s fixed default.
    #[must_use]
    pub fn resolve(&self, key: &Key) -> Option<Button> {
        let index = keys::candidates()
            .iter()
            .position(|(_, candidate)| keys::key_matches(candidate, key))?;
        self.table[index]
    }

    /// Which keys currently produce `button`, in candidate order.
    ///
    /// The live counterpart of [`crate::keys::bound_keys`] - what a Controls
    /// page should draw a binding row's value from, once a keyboard may have
    /// been rebound. Empty is a real answer: see the module doc's note on
    /// what a rebind can leave a button with.
    #[must_use]
    pub fn names_for(&self, button: Button) -> Vec<&'static str> {
        keys::candidates()
            .into_iter()
            .zip(&self.table)
            .filter(|(_, owner)| **owner == Some(button))
            .map(|((name, _), _)| name)
            .collect()
    }

    /// Gives `button` exactly the key named `name`, stealing it from whatever
    /// button held it and clearing every other key `button` held before.
    ///
    /// See the module doc for why a steal and a replace, rather than a refusal
    /// or an addition. Returns the button `name` was taken from, when the
    /// steal actually moved it from somewhere else - `None` when `name` was
    /// already unbound or already `button`'s, or when `name` does not name one
    /// of [`crate::keys::candidates`]'s eighteen keys, in which case nothing
    /// changes at all.
    pub fn rebind(&mut self, button: Button, name: &str) -> Option<Button> {
        let index = keys::candidates()
            .iter()
            .position(|(candidate, _)| *candidate == name)?;
        let stolen_from = self.table[index].filter(|owner| *owner != button);
        for owner in &mut self.table {
            if *owner == Some(button) {
                *owner = None;
            }
        }
        self.table[index] = Some(button);
        stolen_from
    }

    /// Every candidate name, complete, mapped to the button that owns it or
    /// [`UNBOUND`] - what [`crate::keys::candidates`] wrote out means "always
    /// present" in this table too. See the module doc's persistence section.
    #[must_use]
    pub fn to_pairs(&self) -> std::collections::BTreeMap<String, String> {
        keys::candidates()
            .into_iter()
            .zip(&self.table)
            .map(|((name, _), owner)| {
                let value = owner.map_or_else(|| UNBOUND.to_string(), |button| button.to_string());
                (name.to_string(), value)
            })
            .collect()
    }

    /// Rebuilds a table from `pairs`, falling back to the default for any
    /// candidate whose entry is missing or does not parse.
    ///
    /// Never fails: an entry this build cannot make sense of is dropped
    /// rather than refused, the same tolerance `oag_game::main::args`'s other
    /// control-token resolvers give a stale or newer-build settings file. The
    /// second element is every candidate name that had to fall back, in
    /// [`crate::keys::candidates`] order, for a caller that wants to report
    /// it - `oag_game::main::args::resolve_bindings` does; the settings side
    /// that only draws a picture from the file, `oag_game::settings::Controls`,
    /// does not.
    #[must_use]
    pub fn from_pairs(pairs: &std::collections::BTreeMap<String, String>) -> (Self, Vec<String>) {
        let mut bindings = Self::default();
        let mut ignored = Vec::new();
        for (index, (name, _)) in keys::candidates().into_iter().enumerate() {
            match pairs.get(name).map(String::as_str) {
                None => {}
                Some(UNBOUND) => bindings.table[index] = None,
                Some(text) => match input::button_from_name(text) {
                    Some(button) => bindings.table[index] = Some(button),
                    None => ignored.push(name.to_string()),
                },
            }
        }
        (bindings, ignored)
    }
}

#[cfg(test)]
mod tests;
