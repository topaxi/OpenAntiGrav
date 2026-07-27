//! Turns a real device into the one thing the simulation is allowed to see: an
//! [`oag_gameplay::InputSnapshot`].
//!
//! The dependency points from here into `oag-gameplay`, never the other way.
//! That is rule 1 of `docs/architecture/workspace-layout.md`, and it is what
//! lets a replay or a test drive the simulation with no device attached at all.
//!
//! Only a keyboard is mapped so far. A pad and the PSP's own analog nub are the
//! obvious next devices, and both produce the same snapshot, which is the point
//! of the abstract button layer sitting between them and the game.

pub mod keys;

use oag_gameplay::InputSnapshot;
use oag_gameplay::input::{Input, button};
use winit::keyboard::Key;

/// Accumulates key state between ticks and hands out one snapshot per tick.
///
/// Kept separate from the event loop so a test can drive it by calling
/// [`Self::set_key`] directly, with no window and no compositor. The last
/// session could not verify the viewer's interactive camera for exactly the
/// opposite reason: there was no way to inject a key event.
#[derive(Debug, Clone, Default)]
pub struct Keyboard {
    held: u32,
    buttons: Input,
}

impl Keyboard {
    /// A keyboard with nothing held.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a key going down or coming up.
    ///
    /// Unbound keys are ignored rather than being an error: a player pressing
    /// F5 is not a fault condition.
    pub fn set_key(&mut self, key: &Key, pressed: bool) {
        let Some(index) = keys::map_key(key) else {
            return;
        };
        let bit = 1u32 << (index & 0x1f);
        if pressed {
            self.held |= bit;
        } else {
            self.held &= !bit;
        }
    }

    /// Releases everything.
    ///
    /// Call this on focus loss. Without it a key held while the window loses
    /// focus is never seen to come up, and the ship keeps turning while the
    /// player is in another application.
    pub fn release_all(&mut self) {
        self.held = 0;
    }

    /// Ends the tick: computes the button edges, then derives the axes.
    ///
    /// Axes come out of the same bits the buttons do, because a keyboard has no
    /// analog anything. So a keyboard can only ever produce -1, 0 or 1 where a
    /// pad produces a continuum, and the simulation cannot tell the difference:
    /// it reads the snapshot, not the device.
    pub fn snapshot(&mut self) -> InputSnapshot {
        self.buttons.begin_frame(self.held);
        InputSnapshot {
            buttons: self.buttons,
            stick_x: axis(
                self.buttons.is_held(button::RIGHT),
                self.buttons.is_held(button::LEFT),
            ),
            stick_y: axis(
                self.buttons.is_held(button::UP),
                self.buttons.is_held(button::DOWN),
            ),
            airbrake_left: f32::from(u8::from(self.buttons.is_held(button::L))),
            airbrake_right: f32::from(u8::from(self.buttons.is_held(button::R))),
        }
        .sanitised()
    }

    /// The button state as of the last [`Self::snapshot`].
    ///
    /// The front end drives itself off this rather than off a snapshot, because
    /// it needs [`Input::consume_press`] and a snapshot is a value.
    #[must_use]
    pub fn buttons(&self) -> &Input {
        &self.buttons
    }

    /// Mutable button state, for [`Input::consume_press`].
    pub fn buttons_mut(&mut self) -> &mut Input {
        &mut self.buttons
    }
}

/// Two opposing keys into one axis. Both down cancel.
fn axis(positive: bool, negative: bool) -> f32 {
    f32::from(i8::from(positive) - i8::from(negative))
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::NamedKey;

    fn key(name: NamedKey) -> Key {
        Key::Named(name)
    }

    #[test]
    fn a_held_key_becomes_a_held_button() {
        let mut keyboard = Keyboard::new();
        keyboard.set_key(&key(NamedKey::Space), true);
        let snapshot = keyboard.snapshot();
        assert!(snapshot.buttons.is_pressed(button::START));
        assert!(snapshot.buttons.is_held(button::START));

        let snapshot = keyboard.snapshot();
        assert!(
            !snapshot.buttons.is_pressed(button::START),
            "held, not pressed"
        );
        assert!(snapshot.buttons.is_held(button::START));
    }

    #[test]
    fn steering_comes_out_of_the_arrow_keys() {
        let mut keyboard = Keyboard::new();
        keyboard.set_key(&key(NamedKey::ArrowRight), true);
        assert_eq!(keyboard.snapshot().stick_x, 1.0);

        keyboard.set_key(&key(NamedKey::ArrowRight), false);
        keyboard.set_key(&key(NamedKey::ArrowLeft), true);
        assert_eq!(keyboard.snapshot().stick_x, -1.0);
    }

    #[test]
    fn opposing_keys_held_together_cancel() {
        let mut keyboard = Keyboard::new();
        keyboard.set_key(&key(NamedKey::ArrowLeft), true);
        keyboard.set_key(&key(NamedKey::ArrowRight), true);
        let snapshot = keyboard.snapshot();
        assert_eq!(snapshot.stick_x, 0.0);
        // The buttons are still both held: only the derived axis cancels, since
        // a menu that binds left and right separately must still see both.
        assert!(snapshot.buttons.is_held(button::LEFT));
        assert!(snapshot.buttons.is_held(button::RIGHT));
    }

    #[test]
    fn the_shoulders_are_the_airbrakes() {
        let mut keyboard = Keyboard::new();
        keyboard.set_key(&Key::Character("q".into()), true);
        let snapshot = keyboard.snapshot();
        assert_eq!(snapshot.airbrake_left, 1.0);
        assert_eq!(snapshot.airbrake_right, 0.0);
    }

    /// A key held across a focus loss is never seen to come up, so without this
    /// the ship keeps turning while the player is in another window.
    #[test]
    fn releasing_everything_clears_held_keys() {
        let mut keyboard = Keyboard::new();
        keyboard.set_key(&key(NamedKey::ArrowLeft), true);
        assert_eq!(keyboard.snapshot().stick_x, -1.0);

        keyboard.release_all();
        let snapshot = keyboard.snapshot();
        assert_eq!(snapshot.stick_x, 0.0);
        assert!(snapshot.buttons.is_released(button::LEFT));
    }

    #[test]
    fn an_unbound_key_changes_nothing() {
        let mut keyboard = Keyboard::new();
        keyboard.set_key(&key(NamedKey::F1), true);
        assert_eq!(keyboard.snapshot().buttons.held_mask(), 0);
    }

    /// Every axis a keyboard produces is already in range, so this pins that the
    /// sanitiser is on the path rather than that it is needed here.
    #[test]
    fn every_axis_a_keyboard_produces_is_in_range() {
        let mut keyboard = Keyboard::new();
        for named in [
            NamedKey::ArrowUp,
            NamedKey::ArrowDown,
            NamedKey::ArrowLeft,
            NamedKey::ArrowRight,
        ] {
            keyboard.set_key(&key(named), true);
        }
        keyboard.set_key(&Key::Character("q".into()), true);
        keyboard.set_key(&Key::Character("e".into()), true);
        let snapshot = keyboard.snapshot();
        for axis in [
            snapshot.stick_x,
            snapshot.stick_y,
            snapshot.airbrake_left,
            snapshot.airbrake_right,
        ] {
            assert!((-1.0..=1.0).contains(&axis), "axis out of range: {axis}");
        }
    }
}
