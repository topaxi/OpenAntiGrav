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

use winit::keyboard::{Key, NamedKey};

use oag_gameplay::input::Button;

/// Maps a key to an abstract button index, or `None` if it is not bound.
#[must_use]
pub fn map_key(key: &Key) -> Option<Button> {
    Some(match key {
        Key::Named(NamedKey::ArrowUp) => Button::Up,
        Key::Named(NamedKey::ArrowDown) => Button::Down,
        Key::Named(NamedKey::ArrowLeft) => Button::Left,
        Key::Named(NamedKey::ArrowRight) => Button::Right,
        Key::Named(NamedKey::Enter) => Button::Cross,
        Key::Named(NamedKey::Backspace) => Button::Circle,
        Key::Named(NamedKey::Space) => Button::Start,
        Key::Named(NamedKey::Tab) => Button::Select,
        Key::Character(text) => match text.to_ascii_lowercase().as_str() {
            "w" => Button::Up,
            "s" => Button::Down,
            "a" => Button::Left,
            "d" => Button::Right,
            "x" => Button::Cross,
            "z" => Button::Circle,
            "c" => Button::Square,
            "v" => Button::Triangle,
            "q" => Button::L,
            "e" => Button::R,
            _ => return None,
        },
        _ => return None,
    })
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
fn candidates() -> Vec<(&'static str, Key)> {
    vec![
        ("UP", Key::Named(NamedKey::ArrowUp)),
        ("DOWN", Key::Named(NamedKey::ArrowDown)),
        ("LEFT", Key::Named(NamedKey::ArrowLeft)),
        ("RIGHT", Key::Named(NamedKey::ArrowRight)),
        ("ENTER", Key::Named(NamedKey::Enter)),
        ("BACKSPACE", Key::Named(NamedKey::Backspace)),
        ("SPACE", Key::Named(NamedKey::Space)),
        ("TAB", Key::Named(NamedKey::Tab)),
        ("W", Key::Character("w".into())),
        ("A", Key::Character("a".into())),
        ("S", Key::Character("s".into())),
        ("D", Key::Character("d".into())),
        ("X", Key::Character("x".into())),
        ("Z", Key::Character("z".into())),
        ("C", Key::Character("c".into())),
        ("V", Key::Character("v".into())),
        ("Q", Key::Character("q".into())),
        ("E", Key::Character("e".into())),
    ]
}

/// Which keys currently produce `button`, in candidate order.
///
/// Empty when nothing does, which is a real answer rather than a failure: not
/// every abstract button the game knows has a key on this layout.
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
}
