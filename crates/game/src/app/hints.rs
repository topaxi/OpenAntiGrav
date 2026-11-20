//! Window titles and the key hints printed under them.
//!
//! One module because both windowed and headless paths print them, and a
//! title belongs beside the keys it is shown with.
//!
//! # `OAG_HINTS_*` ids
//!
//! Every string here is this project's own invention rather than the
//! disc's - unlike a menu row or a race mode's short name, none of it has a
//! disc idstring to override - so what each one needed was somewhere
//! translatable to live, not an override. Every constant here became a
//! function taking a [`StringTable`], looked up by an `OAG_HINTS_*` id and
//! falling back to the literal below it exactly the way `resolve()` does for
//! `assets/ui/menu.toml`'s rows. A caller builds the table with
//! `oag_ui::strings::project_table`, the same primitive
//! `loading::Screen::new` and `prepare::definition` both use.

use oag_gameplay::input::Button;
use oag_ui::language::StringTable;

/// Looks `id` up in `strings`, falling back to `literal` when there is no
/// override - the same fallback `resolve()` uses in `oag_ui::menu::definition`
/// for a `menu.toml` row with no match.
fn resolved(strings: &StringTable, id: &str, literal: &str) -> String {
    strings.get_or_fallback(id, literal).to_string()
}

/// The window's title while the front end is on screen.
pub(crate) fn title(strings: &StringTable) -> String {
    resolved(strings, "OAG_HINTS_TITLE", "OpenAntiGrav")
}

/// Printed before the front end opens: the picker's own keys.
///
/// Escape quits here and only here, because the boot sequence is the first
/// thing on screen and has nothing behind it to go back to.
pub(crate) fn menu_keys(strings: &StringTable) -> String {
    resolved(
        strings,
        "OAG_HINTS_MENU_KEYS",
        "arrow keys or the left stick move, return, X or cross \
         selects, space or start skips, escape quits",
    )
}

/// And once the menus have the window.
pub(crate) fn shell_title(strings: &StringTable) -> String {
    resolved(strings, "OAG_HINTS_SHELL_TITLE", "OpenAntiGrav - menu")
}

/// The word for a face button in a hint, in the player's language: `circle`, `triangle`. Any other
/// button is named as it is.
pub(crate) fn button_word(strings: Option<&StringTable>, button: Button) -> String {
    let id = match button {
        Button::Circle => "OAG_HINTS_BUTTON_CIRCLE",
        Button::Triangle => "OAG_HINTS_BUTTON_TRIANGLE",
        other => return other.to_string(),
    };
    strings
        .and_then(|table| table.get(id))
        .map_or_else(|| button.to_string(), str::to_string)
}

/// What a hint calls the buttons and keys that go back: the keys bound to `back` and the pad
/// button's word, as `backspace, z, y, circle`.
pub(crate) fn back_list(strings: Option<&StringTable>, back: Button, keys: &[&str]) -> String {
    let mut parts: Vec<String> = keys.iter().map(|key| key.to_ascii_lowercase()).collect();
    parts.push(button_word(strings, back));
    parts.join(", ")
}

/// Printed when the menus open, which have one key the picker does not.
///
/// `%s` is [`back_list`] for the Back this title and pressing resolved to
/// (`docs/ui/back-and-help.md`).
pub(crate) fn shell_keys(strings: &StringTable, back: Button, keys: &[&str]) -> String {
    resolved(
        strings,
        "OAG_HINTS_SHELL_KEYS",
        "up and down move, left and right change a setting, \
         return, X or cross selects, %s or escape goes back",
    )
    .replace("%s", &back_list(Some(strings), back, keys))
}

/// The line under the on-screen keyboard's grid. `%s` is the word for the Back button.
pub(crate) fn keyboard_hint(strings: Option<&StringTable>, back: Button) -> String {
    strings
        .and_then(|table| table.get("OAG_KEYBOARD_HINT"))
        .unwrap_or("CROSS TYPE   SQUARE DELETE   START ACCEPT   %s CANCEL")
        .replace("%s", &button_word(strings, back).to_uppercase())
}

/// The line under Pulse's own `TagInput` row. `%s` is the word for the Back button.
pub(crate) fn tag_entry_hint(strings: Option<&StringTable>, back: Button) -> String {
    strings
        .and_then(|table| table.get("OAG_TAG_ENTRY_HINT"))
        .unwrap_or("LEFT/RIGHT CELL   UP/DOWN GLYPH   CROSS/START ACCEPT   %s CANCEL")
        .replace("%s", &button_word(strings, back).to_uppercase())
}

/// And once a race has taken it over.
pub(crate) fn race_title(strings: &StringTable) -> String {
    resolved(strings, "OAG_HINTS_RACE_TITLE", "OpenAntiGrav - race")
}

pub(crate) fn race_keys(strings: &StringTable) -> String {
    resolved(
        strings,
        "OAG_HINTS_RACE_KEYS",
        "arrow keys or the left stick steer, X, return or R2 thrusts, \
         Q and E or the shoulders are the airbrakes, L2 is both, C fires a pickup and Z absorbs it, \
         space or start pauses",
    )
}

/// What escape does from a race the menus started, and from one `--race` did.
///
/// Escape is "back one level" everywhere; the difference is only that `--race`
/// has no level behind it. See [`Session::escape`].
pub(crate) fn esc_to_menu(strings: &StringTable) -> String {
    resolved(
        strings,
        "OAG_HINTS_ESC_TO_MENU",
        ", escape returns to the menus",
    )
}

pub(crate) fn esc_quits(strings: &StringTable) -> String {
    resolved(strings, "OAG_HINTS_ESC_QUITS", ", escape quits")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn an_id_with_an_override_resolves_to_it() {
        let mut strings = StringTable::default();
        strings.merge(HashMap::from([(
            "OAG_HINTS_TITLE".to_string(),
            "AntiGravOuvert".to_string(),
        )]));
        assert_eq!(title(&strings), "AntiGravOuvert");
    }

    #[test]
    fn an_id_with_no_override_falls_back_to_the_current_literal() {
        assert_eq!(title(&StringTable::default()), "OpenAntiGrav");
    }

    /// The sentence names the keys and the pad button that go back on every title, pressing and
    /// MENU BACK choice (`crate::prompts::roles`): the triangle and `v` on Pulse's PS2 under
    /// `original` and `triangle`, the circle and its keys everywhere else.
    #[test]
    fn the_shell_hint_names_the_back_the_title_pressing_and_choice_resolve() {
        use oag_disc::Platform;
        use oag_title::prompts::MenuBack;
        let strings = StringTable::default();
        let hint = |title: &oag_title::Title, platform, choice| {
            let back = crate::prompts::roles(title, platform, choice, Button::Triangle).back;
            shell_keys(&strings, back, &oag_input::keys::bound_keys(back))
        };
        let circle = "return, X or cross selects, backspace, z, y, circle or escape goes back";
        let triangle = "return, X or cross selects, v, triangle or escape goes back";
        for (choice, ps2, other) in [
            (MenuBack::Original, triangle, circle),
            (MenuBack::Circle, circle, circle),
            (MenuBack::Triangle, triangle, triangle),
        ] {
            let got = hint(oag_pulse::TITLE, Platform::Ps2, choice);
            assert!(got.ends_with(ps2), "Pulse PS2 {choice:?}: {got}");
            for title in [
                oag_pulse::TITLE,
                oag_pure::TITLE,
                oag_hd::TITLE,
                oag_omega::TITLE,
                oag_2048::TITLE,
            ] {
                for platform in [Platform::Psp, Platform::Ps3, Platform::Ps4, Platform::Vita] {
                    let got = hint(title, platform, choice);
                    assert!(
                        got.ends_with(other),
                        "{} {platform:?} {choice:?}: {got}",
                        title.name
                    );
                }
            }
            let got = hint(oag_pure::TITLE, Platform::Ps2, choice);
            assert!(got.ends_with(other), "Pure PS2 {choice:?}: {got}");
        }
    }

    /// A key rebound off the Back button leaves the hint with the keys that are bound now, and a
    /// language that words the button words it.
    #[test]
    fn the_hint_follows_the_bound_keys_and_the_language() {
        let mut strings = StringTable::default();
        strings.merge(HashMap::from([(
            "OAG_HINTS_BUTTON_TRIANGLE".to_string(),
            "Dreieck".to_string(),
        )]));
        assert!(
            shell_keys(&strings, Button::Triangle, &["F4"])
                .ends_with("f4, Dreieck or escape goes back")
        );
        assert!(shell_keys(&strings, Button::Circle, &[]).ends_with("circle or escape goes back"));
    }

    /// The keyboard and tag-entry hints name the Back button, uppercase, in the language.
    #[test]
    fn the_keyboard_hints_name_the_back_button_and_leave_no_placeholder() {
        let none = None;
        for back in [Button::Circle, Button::Triangle] {
            let word = back.to_string().to_uppercase();
            assert!(keyboard_hint(none, back).ends_with(&format!("{word} CANCEL")));
            assert!(tag_entry_hint(none, back).ends_with(&format!("{word} CANCEL")));
        }
    }

    /// Every language's sentence takes the Back button through `%s`, so no language keeps saying
    /// "circle" where Back is the triangle.
    #[test]
    fn every_language_words_the_back_button_through_the_placeholder() {
        for (language, file) in [
            (
                "english",
                include_str!("../../../../assets/ui/strings/english.toml"),
            ),
            (
                "german",
                include_str!("../../../../assets/ui/strings/german.toml"),
            ),
            (
                "portuguesebr",
                include_str!("../../../../assets/ui/strings/portuguesebr.toml"),
            ),
        ] {
            for id in [
                "OAG_HINTS_SHELL_KEYS",
                "OAG_KEYBOARD_HINT",
                "OAG_TAG_ENTRY_HINT",
            ] {
                let line = file
                    .lines()
                    .find(|line| line.starts_with(&format!("{id} = ")))
                    .unwrap_or_else(|| panic!("{language} has no {id}"));
                assert!(line.contains("%s"), "{language} {id} names no Back: {line}");
            }
        }
    }

    /// Every id used here is distinct - a copy/paste mistake giving two hints
    /// the same id would make one override silently apply to both.
    #[test]
    fn every_hint_has_its_own_id() {
        let strings = StringTable::default();
        let literals = [
            title(&strings),
            menu_keys(&strings),
            shell_title(&strings),
            shell_keys(&strings, Button::Circle, &[]),
            race_title(&strings),
            race_keys(&strings),
            esc_to_menu(&strings),
            esc_quits(&strings),
        ];
        let mut overridden = std::collections::HashSet::new();
        for (n, id) in [
            "OAG_HINTS_TITLE",
            "OAG_HINTS_MENU_KEYS",
            "OAG_HINTS_SHELL_TITLE",
            "OAG_HINTS_SHELL_KEYS",
            "OAG_HINTS_RACE_TITLE",
            "OAG_HINTS_RACE_KEYS",
            "OAG_HINTS_ESC_TO_MENU",
            "OAG_HINTS_ESC_QUITS",
        ]
        .into_iter()
        .enumerate()
        {
            let mut one = StringTable::default();
            one.merge(HashMap::from([(id.to_string(), format!("MARK_{n}"))]));
            let resolved = [
                title(&one),
                menu_keys(&one),
                shell_title(&one),
                shell_keys(&one, Button::Circle, &[]),
                race_title(&one),
                race_keys(&one),
                esc_to_menu(&one),
                esc_quits(&one),
            ];
            let changed: Vec<usize> = (0..literals.len())
                .filter(|&i| resolved[i] != literals[i])
                .collect();
            assert_eq!(
                changed,
                vec![n],
                "overriding {id} changed a hint other than the {n}th"
            );
            assert!(overridden.insert(id), "{id} listed twice");
        }
    }
}
