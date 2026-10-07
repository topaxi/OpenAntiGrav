//! A Bluetooth or USB gamepad as Android hands it over: key events with
//! `KEYCODE_BUTTON_*` codes, and a left stick that winit delivers as a motion
//! event rather than as an axis.
//!
//! **Why this exists.** `gilrs` has no Android backend (`gilrs-core`'s
//! `platform` has Linux, macOS, Windows and wasm only), so
//! [`crate::pad::Pad`] never sees a pad there. winit's NativeActivity backend
//! does deliver the pad's buttons as keyboard events with an unidentified
//! physical key carrying the Android keycode, which is what [`AndroidPad::key`]
//! reads. This module is pure - no `winit`, no Android - so the table is tested
//! on any machine.
//!
//! The mapping goes through the one table desktop uses, [`crate::pad::map_button`]:
//! Android's A is the south button, B east, X west, Y north, and
//! [`pad::map_button`] already says what each of those is.
//!
//! # What is not reachable
//!
//! winit 0.30's Android backend turns every `MotionEvent` whose action is
//! `Move` into a `Touch`, a joystick's included, at `pointer.x()`/`y()`
//! (`AXIS_X`/`AXIS_Y`, in -1 to 1). So the **left stick** arrives, through
//! [`AndroidPad::stick`]; **the analog triggers, the right stick and the hat
//! never arrive**. A pad that also reports its triggers as
//! `KEYCODE_BUTTON_L2`/`R2` (most do) still works, as a full pull. The d-pad
//! arrives as arrow keys and is the keyboard's.

use gilrs::Button as Pad;
use oag_gameplay::input::Button;

use crate::pad::{self, Reading};

/// `KEYCODE_BUTTON_A`.
pub const BUTTON_A: u32 = 96;
/// `KEYCODE_BUTTON_B`.
pub const BUTTON_B: u32 = 97;
/// `KEYCODE_BUTTON_X`.
pub const BUTTON_X: u32 = 99;
/// `KEYCODE_BUTTON_Y`.
pub const BUTTON_Y: u32 = 100;
/// `KEYCODE_BUTTON_L1`.
pub const BUTTON_L1: u32 = 102;
/// `KEYCODE_BUTTON_R1`.
pub const BUTTON_R1: u32 = 103;
/// `KEYCODE_BUTTON_L2`.
pub const BUTTON_L2: u32 = 104;
/// `KEYCODE_BUTTON_R2`.
pub const BUTTON_R2: u32 = 105;
/// `KEYCODE_BUTTON_START`.
pub const BUTTON_START: u32 = 108;
/// `KEYCODE_BUTTON_SELECT`.
pub const BUTTON_SELECT: u32 = 109;

/// What an Android pad key is, before it means anything to the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A digital button [`pad::map_button`] binds.
    Button(Button),
    /// L2, pulled fully: the left airbrake or the brake, per the trigger mode.
    LeftTrigger,
    /// R2, pulled fully.
    RightTrigger,
}

/// Which pad key an Android keycode is, or `None` for a key that is not one
/// of the gamepad's (the volume keys, a keyboard letter, the d-pad's arrows).
#[must_use]
pub fn key_of(code: u32) -> Option<Key> {
    let pad = match code {
        BUTTON_A => Pad::South,
        BUTTON_B => Pad::East,
        BUTTON_X => Pad::West,
        BUTTON_Y => Pad::North,
        BUTTON_L1 => Pad::LeftTrigger,
        BUTTON_R1 => Pad::RightTrigger,
        BUTTON_START => Pad::Start,
        BUTTON_SELECT => Pad::Select,
        BUTTON_L2 => return Some(Key::LeftTrigger),
        BUTTON_R2 => return Some(Key::RightTrigger),
        _ => return None,
    };
    pad::map_button(pad).map(Key::Button)
}

/// The pad's state between ticks, built from key events and the stick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AndroidPad {
    buttons: u32,
    left_trigger: bool,
    right_trigger: bool,
    stick_x: f32,
    stick_y: f32,
    /// Whether any pad event has ever arrived, for the caller to know a pad
    /// exists at all.
    seen: bool,
}

impl AndroidPad {
    /// Records a key event for Android keycode `code`. Returns whether it was
    /// a pad key, so the caller can keep it from also reaching the keyboard.
    pub fn key(&mut self, code: u32, pressed: bool) -> bool {
        let Some(key) = key_of(code) else {
            return false;
        };
        self.seen = true;
        match key {
            Key::Button(button) => {
                if pressed {
                    self.buttons |= button.bit();
                } else {
                    self.buttons &= !button.bit();
                }
            }
            Key::LeftTrigger => self.left_trigger = pressed,
            Key::RightTrigger => self.right_trigger = pressed,
        }
        true
    }

    /// The left stick as Android reports it: `AXIS_X` right positive,
    /// `AXIS_Y` **down** positive. Stored with up positive, the sign the
    /// keyboard's UP produces.
    pub fn stick(&mut self, axis_x: f32, axis_y: f32) {
        self.seen = true;
        self.stick_x = axis_x.clamp(-1.0, 1.0);
        self.stick_y = (-axis_y).clamp(-1.0, 1.0);
    }

    /// Lets go of everything, on focus loss or a suspend.
    pub fn release_all(&mut self) {
        *self = Self {
            seen: self.seen,
            ..Self::default()
        };
    }

    /// Whether any pad event has ever arrived.
    #[must_use]
    pub fn seen(&self) -> bool {
        self.seen
    }

    /// This tick's reading, in the shape [`pad::resolve`] takes.
    #[must_use]
    pub fn reading(&self) -> Reading {
        Reading {
            buttons: self.buttons,
            stick_x: self.stick_x,
            stick_y: self.stick_y,
            throttle: f32::from(u8::from(self.right_trigger)),
            brake: f32::from(u8::from(self.left_trigger)),
            ..Reading::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_face_buttons_follow_the_original_names_by_position() {
        assert_eq!(key_of(BUTTON_A), Some(Key::Button(Button::Cross)));
        assert_eq!(key_of(BUTTON_B), Some(Key::Button(Button::Circle)));
        assert_eq!(key_of(BUTTON_X), Some(Key::Button(Button::Square)));
        assert_eq!(key_of(BUTTON_Y), Some(Key::Button(Button::Triangle)));
    }

    #[test]
    fn shoulders_start_select_and_triggers_map() {
        assert_eq!(key_of(BUTTON_L1), Some(Key::Button(Button::L)));
        assert_eq!(key_of(BUTTON_R1), Some(Key::Button(Button::R)));
        assert_eq!(key_of(BUTTON_START), Some(Key::Button(Button::Start)));
        assert_eq!(key_of(BUTTON_SELECT), Some(Key::Button(Button::Select)));
        assert_eq!(key_of(BUTTON_L2), Some(Key::LeftTrigger));
        assert_eq!(key_of(BUTTON_R2), Some(Key::RightTrigger));
    }

    #[test]
    fn keys_that_are_not_the_pad_are_left_for_the_keyboard() {
        // DPAD_UP (19), KEYCODE_BACK (4), MODE (110), THUMBL (106).
        for code in [19, 4, 110, 106, 0] {
            assert_eq!(key_of(code), None);
            assert!(!AndroidPad::default().key(code, true));
        }
    }

    #[test]
    fn a_press_holds_until_the_release() {
        let mut pad = AndroidPad::default();
        assert!(pad.key(BUTTON_A, true));
        assert_eq!(pad.reading().buttons, Button::Cross.bit());
        assert!(pad.key(BUTTON_A, false));
        assert_eq!(pad.reading().buttons, 0);
        assert!(pad.seen());
    }

    #[test]
    fn a_trigger_key_is_a_full_pull() {
        let mut pad = AndroidPad::default();
        pad.key(BUTTON_R2, true);
        pad.key(BUTTON_L2, true);
        let reading = pad.reading();
        assert_eq!((reading.throttle, reading.brake), (1.0, 1.0));
    }

    #[test]
    fn the_stick_flips_android_down_positive_y() {
        let mut pad = AndroidPad::default();
        pad.stick(0.5, 1.0);
        let reading = pad.reading();
        assert_eq!((reading.stick_x, reading.stick_y), (0.5, -1.0));
        pad.stick(2.0, -3.0);
        let reading = pad.reading();
        assert_eq!((reading.stick_x, reading.stick_y), (1.0, 1.0));
    }

    #[test]
    fn release_all_clears_the_state_but_remembers_a_pad_exists() {
        let mut pad = AndroidPad::default();
        pad.key(BUTTON_B, true);
        pad.stick(1.0, 1.0);
        pad.release_all();
        assert_eq!(pad.reading(), Reading::default());
        assert!(pad.seen());
    }

    #[test]
    fn a_resolved_reading_steers_and_brakes_like_a_desktop_pad() {
        let mut pad = AndroidPad::default();
        pad.stick(1.0, 0.0);
        pad.key(BUTTON_L1, true);
        let state = pad::resolve(pad.reading(), pad::TriggerConfig::default());
        assert_eq!(state.stick_x, 1.0);
        assert_eq!(state.airbrake_left, 1.0);
        assert_eq!(state.airbrake_right, 0.0);
    }
}
