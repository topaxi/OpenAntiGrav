//! Gamepad to abstract button mapping, the pad's half of what
//! [`crate::keys`] does for a keyboard.
//!
//! Hardcoded, deliberately: there is no remapping UI and no Steam Input
//! integration yet (both stay planning, see
//! `docs/overview/modern-features.md`). The layout is the Steam Deck's, which
//! is an Xbox-shaped pad, and the mapping targets the same abstract button
//! layer the keyboard produces, so the simulation cannot tell which device a
//! snapshot came from.
//!
//! Two decisions worth naming, because neither is a 1:1 binding:
//!
//! - **R2 is thrust.** The snapshot has no analog throttle - the original's
//!   thrust is the cross *button* - so R2 counts as cross once it is past
//!   [`TRIGGER_THRESHOLD`]. Cross itself (the South button) still works, which
//!   is the original's own convention.
//! - **L2 is "brake", and there is no brake.** The original's action set has
//!   none: the closest thing an anti-gravity ship has is pulling both
//!   airbrakes, so L2 feeds *both* airbrake axes with its analog value. It
//!   deliberately does **not** set the L and R button bits, which would throw
//!   away the analog value the moment it crossed the threshold.
//!
//! The mapping itself is [`map_button`] and [`resolve`], both free functions
//! over plain values, so every case is testable with no pad attached - which
//! is also why `just test` passes on a machine that has none.

use log::warn;
use oag_gameplay::input::button;

/// How far a trigger has to travel before it counts as a button press.
///
/// A resting trigger on a worn pad does not read exactly zero, and thrust that
/// engages itself is worse than thrust that needs a deliberate pull.
pub const TRIGGER_THRESHOLD: f32 = 0.25;

/// Stick movement below this is treated as centred.
///
/// [`oag_gameplay::InputSnapshot::sanitised`] clamps an axis into range; it
/// cannot tell drift from intent, which is what this is for.
pub const STICK_DEADZONE: f32 = 0.15;

/// Maps a pad button to an abstract button index, or `None` if it is not bound.
///
/// The action pad follows the original's own naming rather than the host pad's
/// letters: the South button is cross whatever the pad prints on it.
#[must_use]
pub fn map_button(pad: gilrs::Button) -> Option<u8> {
    use gilrs::Button;

    Some(match pad {
        Button::DPadUp => button::UP,
        Button::DPadDown => button::DOWN,
        Button::DPadLeft => button::LEFT,
        Button::DPadRight => button::RIGHT,
        Button::South => button::CROSS,
        Button::East => button::CIRCLE,
        Button::West => button::SQUARE,
        Button::North => button::TRIANGLE,
        Button::LeftTrigger => button::L,
        Button::RightTrigger => button::R,
        Button::Start => button::START,
        Button::Select => button::SELECT,
        _ => return None,
    })
}

/// Every button [`map_button`] binds, which is what a poll reads.
pub const BOUND_BUTTONS: [gilrs::Button; 12] = [
    gilrs::Button::DPadUp,
    gilrs::Button::DPadDown,
    gilrs::Button::DPadLeft,
    gilrs::Button::DPadRight,
    gilrs::Button::South,
    gilrs::Button::East,
    gilrs::Button::West,
    gilrs::Button::North,
    gilrs::Button::LeftTrigger,
    gilrs::Button::RightTrigger,
    gilrs::Button::Start,
    gilrs::Button::Select,
];

/// One tick's raw readings off a pad, before they mean anything.
///
/// Split out from the pad itself so [`resolve`] - where every decision in this
/// module actually lives - can be driven from a test with no device attached.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Reading {
    /// Abstract bits from the digital buttons, per [`map_button`].
    pub buttons: u32,
    /// Left stick, right positive.
    pub stick_x: f32,
    /// Left stick, up positive, which is the sign the keyboard's UP produces.
    pub stick_y: f32,
    /// R2, 0 to 1.
    pub throttle: f32,
    /// L2, 0 to 1.
    pub brake: f32,
}

/// What a pad contributes to a snapshot.
///
/// Not a snapshot itself: a pad is one of the devices on the window, and the
/// buttons of all of them are merged into one [`crate::Input`] before any edge
/// is computed. See [`crate::Controls`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PadState {
    /// Abstract buttons held, the d-pad and R2-as-thrust included.
    pub held: u32,
    /// Steering, deadzoned.
    pub stick_x: f32,
    /// Pitch, deadzoned.
    pub stick_y: f32,
    /// Left airbrake: L1, or L2 as a brake.
    pub airbrake_left: f32,
    /// Right airbrake: R1, or L2 as a brake.
    pub airbrake_right: f32,
}

/// Turns one tick's readings into what the pad contributes.
///
/// The whole mapping policy is here: the trigger threshold, the deadzone, and
/// L2 meaning both airbrakes.
#[must_use]
pub fn resolve(reading: Reading) -> PadState {
    let mut held = reading.buttons;
    if reading.throttle > TRIGGER_THRESHOLD {
        held |= 1u32 << button::CROSS;
    }

    let shoulder = |index: u8| f32::from(u8::from(held & (1u32 << index) != 0));
    let brake = reading.brake.clamp(0.0, 1.0);

    PadState {
        held,
        stick_x: deadzone(reading.stick_x),
        stick_y: deadzone(reading.stick_y),
        airbrake_left: shoulder(button::L).max(brake),
        airbrake_right: shoulder(button::R).max(brake),
    }
}

/// Zero inside the deadzone, and rescaled outside it so the axis still reaches
/// its full travel rather than jumping from 0 to [`STICK_DEADZONE`].
fn deadzone(value: f32) -> f32 {
    let magnitude = value.abs();
    if magnitude <= STICK_DEADZONE {
        return 0.0;
    }
    let scaled = (magnitude - STICK_DEADZONE) / (1.0 - STICK_DEADZONE);
    scaled.min(1.0) * value.signum()
}

/// Every pad attached to the machine, as one device.
///
/// Pads are merged rather than assigned to players: there is one ship, and a
/// Deck with a pad plugged into its dock should steer from either. A machine
/// with no pad, or a platform `gilrs` cannot open, is not an error - it is a
/// keyboard-only session, which is what every headless capture and CI run is.
pub struct Pad {
    gilrs: Option<gilrs::Gilrs>,
}

impl std::fmt::Debug for Pad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pad")
            .field("available", &self.gilrs.is_some())
            .finish()
    }
}

impl Default for Pad {
    fn default() -> Self {
        Self::new()
    }
}

impl Pad {
    /// Opens the pad subsystem, or reports why it could not and carries on.
    #[must_use]
    pub fn new() -> Self {
        match gilrs::Gilrs::new() {
            Ok(gilrs) => Self { gilrs: Some(gilrs) },
            Err(e) => {
                warn!("no gamepad support ({e}); keyboard only");
                Self { gilrs: None }
            }
        }
    }

    /// A pad that was never opened, and so reads as nothing held.
    ///
    /// What a run that must not touch a device gets: a headless capture, a
    /// test, CI. Opening `gilrs` talks to udev, and a test that reads whatever
    /// pad the developer happens to have plugged in is a test that fails for
    /// one person.
    #[must_use]
    pub fn none() -> Self {
        Self { gilrs: None }
    }

    /// Whether a pad subsystem was opened at all.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.gilrs.is_some()
    }

    /// Names every connected pad, for the line the game prints at startup.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        let Some(gilrs) = &self.gilrs else {
            return Vec::new();
        };
        gilrs
            .gamepads()
            .map(|(_, pad)| pad.name().to_string())
            .collect()
    }

    /// Drains the event queue and reads the merged state of every pad.
    ///
    /// The events have to be drained for `gilrs` to update the state this then
    /// reads, so polling is not optional even though nothing here looks at an
    /// individual event.
    pub fn poll(&mut self) -> PadState {
        let Some(gilrs) = &mut self.gilrs else {
            return PadState::default();
        };
        while gilrs.next_event().is_some() {}

        let mut reading = Reading::default();
        for (_, pad) in gilrs.gamepads() {
            for button in BOUND_BUTTONS {
                if pad.is_pressed(button)
                    && let Some(index) = map_button(button)
                {
                    reading.buttons |= 1u32 << index;
                }
            }
            reading.stick_x = larger(reading.stick_x, pad.value(gilrs::Axis::LeftStickX));
            reading.stick_y = larger(reading.stick_y, pad.value(gilrs::Axis::LeftStickY));
            reading.throttle = reading
                .throttle
                .max(analog(&pad, gilrs::Button::RightTrigger2));
            reading.brake = reading.brake.max(analog(&pad, gilrs::Button::LeftTrigger2));
        }
        resolve(reading)
    }
}

/// A trigger's analog travel, falling back to its digital state on a pad whose
/// mapping does not report one.
fn analog(pad: &gilrs::Gamepad<'_>, button: gilrs::Button) -> f32 {
    pad.button_data(button).map_or_else(
        || f32::from(u8::from(pad.is_pressed(button))),
        |data| data.value(),
    )
}

/// Whichever reading is further from centre, sign kept.
pub(crate) fn larger(a: f32, b: f32) -> f32 {
    if b.abs() > a.abs() { b } else { a }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_south_button_is_cross_whatever_the_pad_prints_on_it() {
        assert_eq!(map_button(gilrs::Button::South), Some(button::CROSS));
        assert_eq!(map_button(gilrs::Button::East), Some(button::CIRCLE));
    }

    #[test]
    fn the_shoulders_are_the_airbrakes() {
        assert_eq!(map_button(gilrs::Button::LeftTrigger), Some(button::L));
        assert_eq!(map_button(gilrs::Button::RightTrigger), Some(button::R));

        let state = resolve(Reading {
            buttons: 1u32 << button::L,
            ..Reading::default()
        });
        assert_eq!(state.airbrake_left, 1.0);
        assert_eq!(state.airbrake_right, 0.0);
    }

    #[test]
    fn every_bound_button_is_in_the_polled_list() {
        for button in BOUND_BUTTONS {
            assert!(map_button(button).is_some(), "{button:?} is polled unbound");
        }
    }

    #[test]
    fn an_unbound_button_changes_nothing() {
        assert_eq!(map_button(gilrs::Button::Mode), None);
        assert_eq!(map_button(gilrs::Button::LeftThumb), None);
        // The analog triggers are deliberately not buttons: they are read as
        // axes and turned into thrust and brake by `resolve`.
        assert_eq!(map_button(gilrs::Button::RightTrigger2), None);
        assert_eq!(map_button(gilrs::Button::LeftTrigger2), None);
    }

    #[test]
    fn r2_past_the_threshold_is_thrust() {
        let idle = resolve(Reading {
            throttle: TRIGGER_THRESHOLD,
            ..Reading::default()
        });
        assert_eq!(idle.held & (1u32 << button::CROSS), 0, "resting trigger");

        let pulled = resolve(Reading {
            throttle: 1.0,
            ..Reading::default()
        });
        assert_ne!(pulled.held & (1u32 << button::CROSS), 0);
    }

    /// L2 is "brake", which the original's action set does not have. Both
    /// airbrakes is the closest thing to one, and it stays analog: setting the
    /// L and R bits instead would quantise it to 0 or 1.
    #[test]
    fn l2_pulls_both_airbrakes_and_stays_analog() {
        let state = resolve(Reading {
            brake: 0.4,
            ..Reading::default()
        });
        assert_eq!(state.airbrake_left, 0.4);
        assert_eq!(state.airbrake_right, 0.4);
        assert_eq!(state.held, 0, "no button bit, or the analog value is lost");
    }

    #[test]
    fn a_held_shoulder_wins_over_a_lighter_brake() {
        let state = resolve(Reading {
            buttons: 1u32 << button::R,
            brake: 0.3,
            ..Reading::default()
        });
        assert_eq!(state.airbrake_left, 0.3);
        assert_eq!(state.airbrake_right, 1.0);
    }

    #[test]
    fn drift_inside_the_deadzone_does_not_steer() {
        let state = resolve(Reading {
            stick_x: STICK_DEADZONE * 0.9,
            stick_y: -STICK_DEADZONE * 0.9,
            ..Reading::default()
        });
        assert_eq!(state.stick_x, 0.0);
        assert_eq!(state.stick_y, 0.0);
    }

    #[test]
    fn a_stick_at_full_travel_still_reaches_one() {
        let state = resolve(Reading {
            stick_x: 1.0,
            stick_y: -1.0,
            ..Reading::default()
        });
        assert_eq!(state.stick_x, 1.0);
        assert_eq!(state.stick_y, -1.0);
    }

    #[test]
    fn the_deadzone_is_rescaled_rather_than_stepped() {
        let just_outside = resolve(Reading {
            stick_x: STICK_DEADZONE + 0.001,
            ..Reading::default()
        });
        assert!(
            just_outside.stick_x.abs() < 0.01,
            "a step at the edge of the deadzone: {}",
            just_outside.stick_x
        );
    }

    #[test]
    fn a_pad_that_could_not_be_opened_reads_as_nothing_held() {
        let mut pad = Pad::none();
        assert!(!pad.is_available());
        assert_eq!(pad.poll(), PadState::default());
        assert!(pad.names().is_empty());
    }
}
