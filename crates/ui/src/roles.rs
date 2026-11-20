//! Which face buttons mean Back and Help on the screen being shown.
//!
//! The original binds the abstract controls `backward` and `cancel` to a pad bit per pressing: the
//! circle on the PSP, the triangle on the PS2 (`docs/ghidra/functions/ps2-pulse-eu/input.md`,
//! "The divergence"). A screen therefore never names the circle or the triangle as its Back: it
//! asks [`Roles::take_back`]. `oag_title::Title::menu_buttons` is the one place that decides what
//! the roles are on a title and pressing, under the player's MENU BACK choice
//! (`docs/ui/back-and-help.md`); `oag_game` hands the result to each screen model.

use oag_core::buttons::{Button, Input};

/// The pad buttons that leave a screen and that open its Help.
///
/// A model that takes Back holds one, defaulting to the PSP's, and the session sets it through the
/// model's `set_roles` before it reads the pad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Roles {
    /// The button that leaves the screen.
    pub back: Button,
    /// The button that opens the screen's Help, or the face button Back left over.
    pub help: Button,
}

impl Roles {
    /// The PSP's, and every title's but Pulse on the PS2: the circle leaves, the triangle helps.
    pub const DEFAULT: Self = Self {
        back: Button::Circle,
        help: Button::Triangle,
    };

    /// Whether Back went down this frame, consuming it.
    pub fn take_back(self, input: &mut Input) -> bool {
        input.take(self.back)
    }

    /// Whether Help went down this frame, consuming it.
    pub fn take_help(self, input: &mut Input) -> bool {
        input.take(self.help)
    }

    /// Whether Back went down this frame, leaving it for the next reader.
    #[must_use]
    pub fn back_pressed(self, input: &Input) -> bool {
        input.is_pressed(self.back)
    }
}

impl Default for Roles {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pressed(button: Button) -> Input {
        let mut input = Input::new();
        input.begin_frame(button.bit());
        input
    }

    #[test]
    fn back_is_the_roles_button_and_not_the_circle() {
        let ps2 = Roles {
            back: Button::Triangle,
            help: Button::Circle,
        };
        let mut circle = pressed(Button::Circle);
        assert!(!ps2.take_back(&mut circle));
        assert!(ps2.take_help(&mut circle));
        let mut triangle = pressed(Button::Triangle);
        assert!(ps2.back_pressed(&triangle));
        assert!(ps2.take_back(&mut triangle));
        assert!(!ps2.back_pressed(&triangle), "taking consumes the press");
    }

    #[test]
    fn the_default_is_the_psps() {
        assert_eq!(Roles::default().back, Button::Circle);
        assert_eq!(Roles::default().help, Button::Triangle);
    }
}
