//! Keyboard to abstract button mapping.
//!
//! The PSP's own layout is preserved rather than flattened: X and Return both
//! mean `activate`, which is cross, and Z and Backspace both mean `cancel`,
//! which is circle. Keeping the names rather than the buttons is what will make
//! remapping and region-specific swaps straightforward later. See
//! `docs/ghidra/functions/psp-pulse-usa/input.md`.
//!
//! Movement is bound to **WASD**, not the arrow keys - both still work, since
//! the arrows cost nothing to keep and some players will reach for them out of
//! habit, but WASD is the documented, primary scheme. That displaces Square and
//! Triangle off `S`/`A`; they move to `C`/`V`, chosen for being the next-nearest
//! keys to the WASD cluster that are not already spoken for.
//!
//! # The candidate set is closed
//!
//! [`candidates`] is not a hint, it is the whole universe: every key this
//! build can ever offer a player, rebound or not, is one of these eighteen.
//! [`crate::bindings::Bindings`] only ever changes which button a candidate
//! currently produces, never adds a new one - the same shape as a menu row's
//! `values_from` list, and for the same reason: a closed set is what keeps
//! `Vec<&'static str>` in [`bound_keys`]'s signature true, rather than a
//! `String` a rebind would have to allocate.

use winit::keyboard::{Key, NamedKey};

use oag_gameplay::input::Button;

/// The name, key and default button for every candidate this build offers.
///
/// The one place all three are written down together - [`candidates`] and
/// [`map_key`] are both projections of this, so neither can drift from the
/// other the way two independently maintained tables could.
fn default_table() -> Vec<(&'static str, Key, Button)> {
    vec![
        ("UP", Key::Named(NamedKey::ArrowUp), Button::Up),
        ("DOWN", Key::Named(NamedKey::ArrowDown), Button::Down),
        ("LEFT", Key::Named(NamedKey::ArrowLeft), Button::Left),
        ("RIGHT", Key::Named(NamedKey::ArrowRight), Button::Right),
        ("ENTER", Key::Named(NamedKey::Enter), Button::Cross),
        ("BACKSPACE", Key::Named(NamedKey::Backspace), Button::Circle),
        ("SPACE", Key::Named(NamedKey::Space), Button::Start),
        ("TAB", Key::Named(NamedKey::Tab), Button::Select),
        ("W", Key::Character("w".into()), Button::Up),
        ("A", Key::Character("a".into()), Button::Left),
        ("S", Key::Character("s".into()), Button::Down),
        ("D", Key::Character("d".into()), Button::Right),
        ("X", Key::Character("x".into()), Button::Cross),
        ("Z", Key::Character("z".into()), Button::Circle),
        // Same button as `Z`, so the physical key next to `X` works on
        // both layouts: QWERTZ swaps `Y` and `Z`, so a player reaching
        // for the absorb key hits whichever letter their board prints
        // there. Neither is bound to anything else, so this costs no
        // other binding.
        ("Y", Key::Character("y".into()), Button::Circle),
        ("C", Key::Character("c".into()), Button::Square),
        ("V", Key::Character("v".into()), Button::Triangle),
        ("Q", Key::Character("q".into()), Button::L),
        ("E", Key::Character("e".into()), Button::R),
    ]
}

/// Whether `key` is the key a candidate names, case-insensitively for a
/// character and exactly for a named key.
///
/// The one piece of key-equality logic in the crate - [`map_key`],
/// [`name_for`] and [`crate::bindings::Bindings::resolve`] all call this
/// rather than comparing `Key`s directly, so "X" and "x" agreeing is one fact
/// rather than three copies of it.
pub(crate) fn key_matches(candidate: &Key, key: &Key) -> bool {
    match (candidate, key) {
        (Key::Character(a), Key::Character(b)) => a.eq_ignore_ascii_case(b.as_str()),
        _ => candidate == key,
    }
}

/// Maps a key to an abstract button index, or `None` if it is not bound.
///
/// The **default** mapping - a fresh install, or a candidate a rebind has not
/// touched. A live keyboard reads [`crate::bindings::Bindings::resolve`]
/// instead, which starts here and moves under a rebind.
#[must_use]
pub fn map_key(key: &Key) -> Option<Button> {
    default_table()
        .into_iter()
        .find(|(_, candidate, _)| key_matches(candidate, key))
        .map(|(_, _, button)| button)
}

/// The candidate name for `key`, if it is one of [`candidates`]'s eighteen.
///
/// What a rebind capture turns a raw key press into: the closed set means
/// this is a lookup rather than a spelling decision, so a key the capture
/// does not recognise is left waiting rather than guessed at.
#[must_use]
pub fn name_for(key: &Key) -> Option<&'static str> {
    default_table()
        .into_iter()
        .find(|(_, candidate, _)| key_matches(candidate, key))
        .map(|(name, _, _)| name)
}

/// Every key this build offers to [`map_key`], for showing a player what is
/// bound to what.
///
/// A list of *candidates*, not a binding table: [`bound_keys`] asks `map_key`
/// itself about each one, so what the Controls page shows is literally what the
/// mapping does rather than a second copy of it that can drift. Adding a key to
/// `map_key` and forgetting it here under-reports; the reverse is impossible,
/// and [`tests::every_mapped_button_has_a_key_to_show_for_it`] catches the
/// under-reporting case for any button that has no candidate at all.
///
/// `pub(crate)` rather than private: [`crate::bindings`] walks this same list
/// to resolve a live, rebindable table, and doing so through this function
/// rather than a second copy of [`default_table`] is what keeps the two unable
/// to disagree about which eighteen keys exist.
pub(crate) fn candidates() -> Vec<(&'static str, Key)> {
    default_table()
        .into_iter()
        .map(|(name, key, _)| (name, key))
        .collect()
}

/// Which keys currently produce `button`, in candidate order.
///
/// Empty when nothing does, which is a real answer rather than a failure: not
/// every abstract button the game knows has a key on this layout.
///
/// The **default** table's answer. [`crate::bindings::Bindings::names_for`] is
/// the live one a rebound keyboard should show instead - this stays for the
/// callers that only ever want the built-in layout, and so
/// [`tests::a_button_with_two_keys_reports_both`] keeps proving the default
/// table itself rather than whatever a test happened to rebind it to.
#[must_use]
pub fn bound_keys(button: Button) -> Vec<&'static str> {
    candidates()
        .into_iter()
        .filter(|(_, key)| map_key(key) == Some(button))
        .map(|(name, _)| name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The under-reporting guard. A key added to `map_key` and not to
    /// `candidates` would leave the Controls page quietly wrong; this catches
    /// the case where a whole button ends up with nothing to show, which is the
    /// version of that mistake a player would notice.
    #[test]
    fn every_mapped_button_has_a_key_to_show_for_it() {
        let mut mapped: Vec<Button> = candidates()
            .iter()
            .filter_map(|(_, k)| map_key(k))
            .collect();
        mapped.sort_unstable_by_key(|button| button.index());
        mapped.dedup();
        for button in mapped {
            assert!(
                !bound_keys(button).is_empty(),
                "button {button} maps from a key but reports none"
            );
        }
    }

    #[test]
    fn a_button_with_two_keys_reports_both() {
        assert_eq!(bound_keys(Button::Up), ["UP", "W"]);
        assert_eq!(bound_keys(Button::Cross), ["ENTER", "X"]);
    }

    #[test]
    fn a_button_no_key_produces_reports_nothing() {
        // Nothing on this layout is bound to the PSP's own `any` pseudo-button.
        assert!(bound_keys(oag_gameplay::input::Button::Any).is_empty());
    }

    #[test]
    fn return_and_x_are_both_activate() {
        assert_eq!(map_key(&Key::Named(NamedKey::Enter)), Some(Button::Cross));
        assert_eq!(map_key(&Key::Character("x".into())), Some(Button::Cross));
        assert_eq!(map_key(&Key::Character("X".into())), Some(Button::Cross));
    }

    #[test]
    fn space_is_start_so_the_intro_can_be_skipped() {
        assert_eq!(map_key(&Key::Named(NamedKey::Space)), Some(Button::Start));
    }

    #[test]
    fn wasd_and_the_arrow_keys_both_steer() {
        assert_eq!(map_key(&Key::Character("w".into())), Some(Button::Up));
        assert_eq!(map_key(&Key::Character("a".into())), Some(Button::Left));
        assert_eq!(map_key(&Key::Character("s".into())), Some(Button::Down));
        assert_eq!(map_key(&Key::Character("d".into())), Some(Button::Right));
        assert_eq!(map_key(&Key::Named(NamedKey::ArrowUp)), Some(Button::Up));
        assert_eq!(
            map_key(&Key::Named(NamedKey::ArrowLeft)),
            Some(Button::Left)
        );
        assert_eq!(
            map_key(&Key::Named(NamedKey::ArrowDown)),
            Some(Button::Down)
        );
        assert_eq!(
            map_key(&Key::Named(NamedKey::ArrowRight)),
            Some(Button::Right)
        );
    }

    /// `Y` and `Z` are the same button so that absorbing a pickup works on a
    /// QWERTZ board as well as a QWERTY one - the two layouts swap exactly
    /// these two letters, so the key beside `X` prints differently depending
    /// on which board the player has.
    #[test]
    fn y_and_z_both_absorb_so_qwertz_and_qwerty_agree() {
        assert_eq!(map_key(&Key::Character("z".into())), Some(Button::Circle));
        assert_eq!(map_key(&Key::Character("y".into())), Some(Button::Circle));
        assert_eq!(map_key(&Key::Character("Y".into())), Some(Button::Circle));
        assert_eq!(bound_keys(Button::Circle), ["BACKSPACE", "Z", "Y"]);
    }

    #[test]
    fn square_and_triangle_moved_off_wasd_onto_c_and_v() {
        assert_eq!(map_key(&Key::Character("c".into())), Some(Button::Square));
        assert_eq!(map_key(&Key::Character("v".into())), Some(Button::Triangle));
    }

    #[test]
    fn activate_and_cancel_are_not_the_same_key() {
        assert_ne!(
            map_key(&Key::Character("x".into())),
            map_key(&Key::Character("z".into()))
        );
    }

    #[test]
    fn unmapped_keys_are_ignored() {
        assert_eq!(map_key(&Key::Character("k".into())), None);
        assert_eq!(map_key(&Key::Named(NamedKey::F1)), None);
    }

    #[test]
    fn name_for_is_the_inverse_of_a_candidate_key() {
        for (name, key) in candidates() {
            assert_eq!(name_for(&key), Some(name));
        }
    }

    #[test]
    fn name_for_rejects_a_key_off_the_closed_set() {
        assert_eq!(name_for(&Key::Named(NamedKey::F1)), None);
    }
}
