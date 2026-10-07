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
use oag_ui::language::StringTable;

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
    if matches!(key, Key::Named(NamedKey::Escape | NamedKey::BrowserBack)) {
        return Capture::Cancel;
    }
    match keys::name_for(key) {
        Some(name) => Capture::Bind(awaiting, name),
        None => Capture::Ignore,
    }
}

/// The CONTROLS page's own key-capture prompt, for `button` - what
/// `session::draw` shows over the page for as long as `Session::
/// awaiting_binding` names it.
///
/// **Pulled out for the same reason [`decide`] is a free function at all**:
/// `Session::draw` needs a live `Gpu` to reach, so the one piece of it that
/// is pure - turning a button and a string table into the line a player
/// reads - lives here instead, under a unit test, rather than only inside
/// the one closure nothing can drive without a window and a device.
/// `strings` is `None` on a `--race` run with no shell, the same case
/// `session::pilot_editor::say`'s own doc names.
///
/// `OAG_BINDING_CAPTURE_PROMPT` and its English fallback are documented
/// together in `assets/ui/strings/english.toml`; `%s` is `button`'s own
/// `Display` name, upper-cased to match every other value this page draws.
#[must_use]
pub(crate) fn prompt(button: Button, strings: Option<&StringTable>) -> String {
    const ID: &str = "OAG_BINDING_CAPTURE_PROMPT";
    const ENGLISH: &str = "PRESS A KEY FOR %s - ESCAPE CANCELS";
    strings
        .and_then(|table| table.get(ID))
        .unwrap_or(ENGLISH)
        .replace("%s", &button.to_string().to_ascii_uppercase())
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
    fn the_android_back_key_cancels_a_capture_like_escape() {
        assert_eq!(
            decide(Button::Up, &Key::Named(NamedKey::BrowserBack), true, false),
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

    #[test]
    fn with_no_table_the_prompt_names_the_button_in_english() {
        let text = prompt(Button::Circle, None);
        assert!(text.contains("CIRCLE"), "{text:?} does not name the button");
        assert!(!text.contains("%s"), "{text:?} left a %s unsubstituted");
    }

    #[test]
    fn a_table_entry_overrides_english_and_still_substitutes() {
        let mut table = StringTable::default();
        table.merge(std::collections::HashMap::from([(
            "OAG_BINDING_CAPTURE_PROMPT".to_string(),
            "APPUYEZ SUR %s".to_string(),
        )]));
        let text = prompt(Button::Square, Some(&table));
        assert_eq!(text, "APPUYEZ SUR SQUARE");
    }
}
