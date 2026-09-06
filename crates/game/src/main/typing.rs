//! What a raw key event means while an on-screen keyboard is open.
//!
//! [`crate::rebind`]'s sibling, split out for exactly the reason that one was:
//! `app.rs`'s winit handler is the one function in this build nothing headless
//! can drive, so the *decision* lives here under a unit test and the handler
//! only acts on it.
//!
//! # Why a desk keyboard is wired at all when the grid already works
//!
//! `oag_game::prompt` is pad-first on purpose - a text field only a keyboard
//! could fill would be the first thing in this build a pad cannot reach. But
//! the reverse is not a virtue: somebody sitting at a keyboard should not have
//! to walk a cursor to `w`, `i`, `n` and so on when the key is right there.
//! Both paths end in the same [`Edit`], applied to the same buffer, and
//! [`oag_game::prompt::accepts`] is what keeps the character sets from
//! drifting apart.
//!
//! # It differs from `rebind` on repeats, deliberately
//!
//! A held key is [`Capture::Ignore`](crate::rebind::Capture::Ignore) over
//! there, because binding one key thirty times a second is nonsense. Here it
//! is not: holding backspace to clear a name is what every text field in
//! existence does, and a player who has to tap it twenty-four times would
//! rightly call this broken.

use winit::keyboard::{Key, NamedKey};

use oag_game::prompt::{Edit, accepts};

/// What `app.rs` should do about one keyboard event while a keyboard prompt
/// is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Typed {
    /// Apply this to the buffer.
    Edit(Edit),
    /// Finish, as the grid's own accept key does.
    Accept,
    /// Not something the prompt understands. **The event still goes on to
    /// `Controls::set_key`**, which is what leaves the arrow keys navigating
    /// the grid while the letters type into it.
    Ignore,
}

/// Decides [`Typed`] for one keyboard event, with a keyboard prompt already
/// known to be open.
///
/// `pressed` and `repeat` are `event.state == ElementState::Pressed` and
/// `event.repeat`, passed as bare bools rather than the winit types so this
/// stays free of a window to construct one with - the same shape
/// [`crate::rebind::decide`] takes.
///
/// **Escape is not here.** It is handled one branch earlier in `app.rs` and
/// reaches `Session::escape`, which cancels an open prompt before it does
/// anything else - so cancelling is one rule in one place rather than two
/// that can disagree.
pub(crate) fn decide(key: &Key, pressed: bool, repeat: bool) -> Typed {
    if !pressed {
        return Typed::Ignore;
    }
    match key {
        // An OS repeat of Enter would accept a prompt that is already gone,
        // and the second accept would land on whatever replaced it. Only
        // typing and deleting repeat; see this module's own doc.
        Key::Named(NamedKey::Enter) if !repeat => Typed::Accept,
        Key::Named(NamedKey::Backspace | NamedKey::Delete) => Typed::Edit(Edit::Delete),
        // Lowercased before the check, so a player with caps lock on or a
        // finger on shift types the same name rather than nothing at all -
        // the grid has no uppercase to offer and `pilots::check_name` has no
        // uppercase to allow.
        Key::Character(text) => match text.chars().next().map(|c| c.to_ascii_lowercase()) {
            // One character only: a dead key or an IME commit can deliver
            // several, and appending them all would type a string the grid
            // could not have produced.
            Some(c) if text.chars().count() == 1 && accepts(c) => Typed::Edit(Edit::Type(c)),
            _ => Typed::Ignore,
        },
        _ => Typed::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_letter_types_itself() {
        assert_eq!(
            decide(&Key::Character("w".into()), true, false),
            Typed::Edit(Edit::Type('w'))
        );
    }

    /// The grid has no uppercase and a pilot name may hold none, so shift is
    /// folded rather than refused - a player holding it should get a name,
    /// not silence.
    #[test]
    fn an_uppercase_letter_folds_to_the_lowercase_the_grid_offers() {
        assert_eq!(
            decide(&Key::Character("W".into()), true, false),
            Typed::Edit(Edit::Type('w'))
        );
    }

    /// Anything the grid cannot produce is left alone, so the two input paths
    /// can never build different names.
    #[test]
    fn a_character_the_grid_does_not_offer_is_ignored() {
        for text in ["/", " ", ".", "é", "\\"] {
            assert_eq!(
                decide(&Key::Character(text.into()), true, false),
                Typed::Ignore,
                "{text:?} should not be typeable"
            );
        }
    }

    /// An IME commit or a dead key can deliver several characters at once,
    /// and appending them would type a string the grid could not have.
    #[test]
    fn a_multi_character_commit_is_ignored() {
        assert_eq!(
            decide(&Key::Character("ab".into()), true, false),
            Typed::Ignore
        );
    }

    #[test]
    fn backspace_and_delete_both_delete() {
        for named in [NamedKey::Backspace, NamedKey::Delete] {
            assert_eq!(
                decide(&Key::Named(named), true, false),
                Typed::Edit(Edit::Delete)
            );
        }
    }

    /// Holding backspace clears a name, which is what every text field does -
    /// the one place this deliberately differs from `rebind`.
    #[test]
    fn a_held_delete_repeats_unlike_a_rebind() {
        assert_eq!(
            decide(&Key::Named(NamedKey::Backspace), true, true),
            Typed::Edit(Edit::Delete)
        );
    }

    /// Enter is the exception: a repeat of it would accept a prompt that is
    /// already gone, and the second accept would land on whatever replaced it.
    #[test]
    fn enter_accepts_once_and_does_not_repeat() {
        assert_eq!(
            decide(&Key::Named(NamedKey::Enter), true, false),
            Typed::Accept
        );
        assert_eq!(
            decide(&Key::Named(NamedKey::Enter), true, true),
            Typed::Ignore
        );
    }

    /// A release types nothing, or every key would type twice.
    #[test]
    fn a_release_types_nothing() {
        assert_eq!(
            decide(&Key::Character("w".into()), false, false),
            Typed::Ignore
        );
    }

    /// The arrow keys have to fall through, or the grid stops navigating the
    /// moment a desk keyboard is used.
    #[test]
    fn the_arrow_keys_are_left_for_the_abstract_buttons() {
        for named in [
            NamedKey::ArrowUp,
            NamedKey::ArrowDown,
            NamedKey::ArrowLeft,
            NamedKey::ArrowRight,
        ] {
            assert_eq!(decide(&Key::Named(named), true, false), Typed::Ignore);
        }
    }
}
