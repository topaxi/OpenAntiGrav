//! Keyboard to abstract button mapping.
//!
//! The PSP's own layout is preserved rather than flattened: X and Return both
//! mean `activate`, which is cross, and Z and Backspace both mean `cancel`,
//! which is circle. Keeping the names rather than the buttons is what will make
//! remapping and region-specific swaps straightforward later. See
//! `docs/ghidra/functions/psp-pulse/input.md`.
//!
//! Movement is bound to **WASD**, not the arrow keys - both still work, since
//! the arrows cost nothing to keep and some players will reach for them out of
//! habit, but WASD is the documented, primary scheme. That displaces Square and
//! Triangle off `S`/`A`; they move to `C`/`V`, chosen for being the next-nearest
//! keys to the WASD cluster that are not already spoken for.

use winit::keyboard::{Key, NamedKey};

use oag_gameplay::input::button;

/// Maps a key to an abstract button index, or `None` if it is not bound.
#[must_use]
pub fn map_key(key: &Key) -> Option<u8> {
    Some(match key {
        Key::Named(NamedKey::ArrowUp) => button::UP,
        Key::Named(NamedKey::ArrowDown) => button::DOWN,
        Key::Named(NamedKey::ArrowLeft) => button::LEFT,
        Key::Named(NamedKey::ArrowRight) => button::RIGHT,
        Key::Named(NamedKey::Enter) => button::CROSS,
        Key::Named(NamedKey::Backspace) => button::CIRCLE,
        Key::Named(NamedKey::Space) => button::START,
        Key::Named(NamedKey::Tab) => button::SELECT,
        Key::Character(text) => match text.to_ascii_lowercase().as_str() {
            "w" => button::UP,
            "s" => button::DOWN,
            "a" => button::LEFT,
            "d" => button::RIGHT,
            "x" => button::CROSS,
            "z" => button::CIRCLE,
            "c" => button::SQUARE,
            "v" => button::TRIANGLE,
            "q" => button::L,
            "e" => button::R,
            _ => return None,
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_and_x_are_both_activate() {
        assert_eq!(map_key(&Key::Named(NamedKey::Enter)), Some(button::CROSS));
        assert_eq!(map_key(&Key::Character("x".into())), Some(button::CROSS));
        assert_eq!(map_key(&Key::Character("X".into())), Some(button::CROSS));
    }

    #[test]
    fn space_is_start_so_the_intro_can_be_skipped() {
        assert_eq!(map_key(&Key::Named(NamedKey::Space)), Some(button::START));
    }

    #[test]
    fn wasd_and_the_arrow_keys_both_steer() {
        assert_eq!(map_key(&Key::Character("w".into())), Some(button::UP));
        assert_eq!(map_key(&Key::Character("a".into())), Some(button::LEFT));
        assert_eq!(map_key(&Key::Character("s".into())), Some(button::DOWN));
        assert_eq!(map_key(&Key::Character("d".into())), Some(button::RIGHT));
        assert_eq!(map_key(&Key::Named(NamedKey::ArrowUp)), Some(button::UP));
        assert_eq!(
            map_key(&Key::Named(NamedKey::ArrowLeft)),
            Some(button::LEFT)
        );
        assert_eq!(
            map_key(&Key::Named(NamedKey::ArrowDown)),
            Some(button::DOWN)
        );
        assert_eq!(
            map_key(&Key::Named(NamedKey::ArrowRight)),
            Some(button::RIGHT)
        );
    }

    #[test]
    fn square_and_triangle_moved_off_wasd_onto_c_and_v() {
        assert_eq!(map_key(&Key::Character("c".into())), Some(button::SQUARE));
        assert_eq!(map_key(&Key::Character("v".into())), Some(button::TRIANGLE));
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
