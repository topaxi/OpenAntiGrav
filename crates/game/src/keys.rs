//! Keyboard to abstract button mapping.
//!
//! The PSP's own layout is preserved rather than flattened: X and Return both
//! mean `activate`, which is cross, and Z and Backspace both mean `cancel`,
//! which is circle. Keeping the names rather than the buttons is what will make
//! remapping and region-specific swaps straightforward later. See
//! `docs/ghidra/functions/psp-pulse/input.md`.

use winit::keyboard::{Key, NamedKey};

use crate::input::button;

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
            "x" => button::CROSS,
            "z" => button::CIRCLE,
            "s" => button::SQUARE,
            "a" => button::TRIANGLE,
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
