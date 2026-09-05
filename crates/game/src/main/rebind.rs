//! What a raw key event means while the CONTROLS page is waiting for one.
//!
//! Split out as a pure function for the reason `session::menus::backdrop_seed`
//! is: `app.rs`'s winit handler is the one place headless verification cannot
//! reach, and this task's own instructions call for headless proof. [`decide`]
//! takes exactly what that handler has in hand and returns a decision `app.rs`
//! only has to act on - escape-cancels, the repeat guard and an unrecognised
//! key being left waiting all live here, under a unit test, rather than in the
//! one function nothing can drive without a window.

use winit::keyboard::{Key, NamedKey};

use oag_gameplay::input::Button;
use oag_input::keys;

/// What `app.rs` should do about one keyboard event while
/// `Session::awaiting_binding` names a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Capture {
    /// Bind `awaiting` to the named candidate key and stop listening.
    Bind(Button, &'static str),
    /// Stop listening without changing anything. Escape's own job here, not
    /// the menus' back key - the same reason `Session::escape` is called out
    /// of band rather than through a button, see `docs/architecture/menus.md`.
    Cancel,
    /// Neither: a release, an OS auto-repeat, or a key off the closed
    /// candidate set. Keep listening.
    Ignore,
}

/// Decides `Capture` for one keyboard event, with the capture already known
/// to be live.
///
/// `pressed` and `repeat` are `event.state == ElementState::Pressed` and
/// `event.repeat`, passed as bare bools rather than the winit type so this
/// stays free of a window to construct one with. Only a **fresh** press acts;
/// a release or an OS-repeated `Pressed` event is [`Capture::Ignore`], the
/// same guard `docs/architecture/menus.md`'s own escape section needed for
/// exactly the same reason - a held key must not fire thirty times a second.
pub(crate) fn decide(awaiting: Button, key: &Key, pressed: bool, repeat: bool) -> Capture {
    if !pressed || repeat {
        return Capture::Ignore;
    }
    if *key == Key::Named(NamedKey::Escape) {
        return Capture::Cancel;
    }
    match keys::name_for(key) {
        Some(name) => Capture::Bind(awaiting, name),
        None => Capture::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_press_of_a_candidate_key_binds_it() {
        assert_eq!(
            decide(Button::Up, &Key::Character("q".into()), true, false),
            Capture::Bind(Button::Up, "Q")
        );
    }

    #[test]
    fn a_key_off_the_closed_set_is_ignored_and_capture_stays_open() {
        assert_eq!(
            decide(Button::Up, &Key::Named(NamedKey::F1), true, false),
            Capture::Ignore
        );
    }

    #[test]
    fn a_release_is_ignored() {
        assert_eq!(
            decide(Button::Up, &Key::Character("q".into()), false, false),
            Capture::Ignore
        );
    }

    #[test]
    fn an_os_repeat_is_ignored() {
        assert_eq!(
            decide(Button::Up, &Key::Character("q".into()), true, true),
            Capture::Ignore
        );
    }

    #[test]
    fn escape_cancels_rather_than_binding_or_backing_out_of_the_menu() {
        assert_eq!(
            decide(Button::Up, &Key::Named(NamedKey::Escape), true, false),
            Capture::Cancel
        );
    }

    #[test]
    fn a_repeated_escape_is_ignored_not_cancelled_twice() {
        assert_eq!(
            decide(Button::Up, &Key::Named(NamedKey::Escape), true, true),
            Capture::Ignore
        );
    }
}
