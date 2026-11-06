//! Opening a phone's on-screen keyboard when a text prompt is up.
//!
//! winit 0.30's `Window::set_ime_allowed` shows and hides the soft keyboard on Android and iOS
//! (`platform_impl/android/mod.rs` calls `show_soft_input` / `hide_soft_input`), and does
//! something else on a desktop (composition, no dead keys on X11), so it is only asked for on a
//! phone build. **Untested on a device**: no phone was driven for this
//! (`docs/ui/player-name.md`, "Soft keyboards").

use super::Session;

/// Whether the soft keyboard should be showing: a phone build with a typing prompt open.
pub(crate) const fn wanted(mobile: bool, typing_open: bool) -> bool {
    mobile && typing_open
}

impl Session {
    /// Shows or hides the soft keyboard to match whether a typing prompt is open. Called every
    /// frame and acts only on a change.
    pub(crate) fn sync_soft_keyboard(&mut self) {
        let wanted = wanted(oag_mobile::MOBILE, self.typing_is_open());
        if wanted != self.soft_keyboard {
            self.soft_keyboard = wanted;
            self.gpu.output.window.set_ime_allowed(wanted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::wanted;

    #[test]
    fn only_a_phone_with_a_typing_prompt_open_asks_for_the_keyboard() {
        assert!(wanted(true, true));
        assert!(!wanted(true, false));
        assert!(!wanted(false, true));
    }
}
