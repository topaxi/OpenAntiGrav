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
//! `oag_game::strings::project_table`, the same primitive
//! `loading::Screen::new` and `prepare::definition` both use.

use oag_game::language::StringTable;

/// Looks `id` up in `strings`, falling back to `literal` when there is no
/// override - the same fallback `resolve()` uses in `oag_game::menu::definition`
/// for a `menu.toml` row with no match.
fn resolved(strings: &StringTable, id: &str, literal: &str) -> String {
    strings.get(id).unwrap_or(literal).to_string()
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

/// Printed when the menus open, which have one key the picker does not.
pub(crate) fn shell_keys(strings: &StringTable) -> String {
    resolved(
        strings,
        "OAG_HINTS_SHELL_KEYS",
        "up and down move, left and right change a setting, \
         return, X or cross selects, backspace, circle or escape goes back",
    )
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

    /// Every id used here is distinct - a copy/paste mistake giving two hints
    /// the same id would make one override silently apply to both.
    #[test]
    fn every_hint_has_its_own_id() {
        let strings = StringTable::default();
        let literals = [
            title(&strings),
            menu_keys(&strings),
            shell_title(&strings),
            shell_keys(&strings),
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
                shell_keys(&one),
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
