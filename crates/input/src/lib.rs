//! Turns a real device into the one thing the simulation is allowed to see: an
//! [`oag_gameplay::InputSnapshot`].
//!
//! The dependency points from here into `oag-gameplay`, never the other way.
//! That is rule 1 of `docs/architecture/workspace-layout.md`, and it is what
//! lets a replay or a test drive the simulation with no device attached at all.
//!
//! A keyboard ([`Keyboard`]) and a gamepad ([`pad::Pad`]) are mapped, and
//! [`Controls`] is both of them as one device, which is what a window has. The
//! PSP's own analog nub is the obvious next one, and it would produce the same
//! snapshot, which is the point of the abstract button layer sitting between
//! any device and the game.

pub mod keys;
pub mod pad;

use oag_gameplay::InputSnapshot;
use oag_gameplay::input::{Button, Input};
use winit::keyboard::Key;

pub use pad::Pad;

/// Accumulates key state between ticks and hands out one snapshot per tick.
///
/// Kept separate from the event loop so a test can drive it by calling
/// [`Self::set_key`] directly, with no window and no compositor. The last
/// session could not verify the viewer's interactive camera for exactly the
/// opposite reason: there was no way to inject a key event.
#[derive(Debug, Clone, Default)]
pub struct Keyboard {
    held: u32,
    /// Bits that went down since the last read, whether or not they are still
    /// down. See [`Self::take_taps`].
    tapped: u32,
    /// **Only read when this `Keyboard` is used on its own**, through
    /// [`Self::snapshot`]. Inside [`Controls`] it is dead state: that merges
    /// [`Self::held_mask`] with the pad's bits and computes the edges on its
    /// own `Input`, and never calls `Keyboard::snapshot` at all. Two
    /// edge-computing `Input`s in one path is an invitation to read the stale
    /// one, which is finding U9 of the 2026-08-18 review - recorded here rather
    /// than removed, because the standalone path is real and tested.
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
        let Some(button) = keys::map_key(key) else {
            return;
        };
        let bit = 1u32 << (button.index() & 0x1f);
        if pressed {
            self.held |= bit;
            self.tapped |= bit;
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
        // The latch goes too: a tap that happened before focus was lost is
        // input the player meant for the other application by the time we
        // notice, and delivering it a frame later is worse than dropping it.
        self.tapped = 0;
    }

    /// Bits that went down since the last call, and clears the latch.
    ///
    /// **Why a latch exists at all**: a key pressed *and released* entirely
    /// between two 60 Hz reads leaves [`Self::held_mask`] at zero both times,
    /// so the press was silently lost - finding U5 of the 2026-08-18 review. A
    /// hardware keyboard's repeat rate cannot produce one, but a scripted
    /// press, a macro key and a frame that ran long all can, and a menu
    /// confirm that sometimes does nothing is the worst kind of bug to chase.
    ///
    /// Or-ing this into one frame's held mask makes such a tap a full
    /// press-then-release edge across two frames, which is exactly what a
    /// slower tap would have produced.
    pub fn take_taps(&mut self) -> u32 {
        std::mem::take(&mut self.tapped)
    }

    /// Ends the tick: computes the button edges, then derives the axes.
    ///
    /// Axes come out of the same bits the buttons do, because a keyboard has no
    /// analog anything. So a keyboard can only ever produce -1, 0 or 1 where a
    /// pad produces a continuum, and the simulation cannot tell the difference:
    /// it reads the snapshot, not the device.
    pub fn snapshot(&mut self) -> InputSnapshot {
        let held = self.held | self.take_taps();
        self.buttons.begin_frame(held);
        InputSnapshot {
            buttons: self.buttons,
            stick_x: axis(
                self.buttons.is_held(Button::Right),
                self.buttons.is_held(Button::Left),
            ),
            stick_y: axis(
                self.buttons.is_held(Button::Up),
                self.buttons.is_held(Button::Down),
            ),
            airbrake_left: f32::from(u8::from(self.buttons.is_held(Button::L))),
            airbrake_right: f32::from(u8::from(self.buttons.is_held(Button::R))),
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

    /// The keys held right now, as abstract bits.
    ///
    /// What [`Controls`] merges with a pad's bits before any edge is computed.
    #[must_use]
    pub fn held_mask(&self) -> u32 {
        self.held
    }
}

/// Every device on the window, as the one thing a tick reads.
///
/// A window has devices, not a device: a player on a Deck may steer with the
/// stick and skip the intro with the keyboard in the same second. The merge has
/// to happen **before** the edges are computed, which is why this owns the
/// single [`Input`] and neither the keyboard nor the pad computes its own: a
/// press seen on two devices is one press, and
/// [`Input::consume_press`] has one place to clear it.
#[derive(Debug, Default)]
pub struct Controls {
    keyboard: Keyboard,
    pad: Pad,
    buttons: Input,
}

impl Controls {
    /// Opens every device. A machine with no pad is a keyboard-only session
    /// rather than an error; see [`Pad::new`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every device but the pad, for a run that must not open one. See
    /// [`Pad::none`].
    #[must_use]
    pub fn without_pad() -> Self {
        // Spelled out rather than `..Self::default()`, which would open the pad
        // subsystem only to drop it again.
        Self {
            keyboard: Keyboard::new(),
            pad: Pad::none(),
            buttons: Input::new(),
        }
    }

    /// Records a key going down or coming up. See [`Keyboard::set_key`].
    pub fn set_key(&mut self, key: &Key, pressed: bool) {
        self.keyboard.set_key(key, pressed);
    }

    /// Releases every key. Call it on focus loss, as [`Keyboard::release_all`]
    /// explains.
    ///
    /// The pad is deliberately not released: it is read fresh every tick, and a
    /// pad keeps reporting its own state whether or not the window has focus.
    pub fn release_all(&mut self) {
        self.keyboard.release_all();
    }

    /// Whether a pad subsystem was opened, and what is connected to it.
    #[must_use]
    pub fn pad(&self) -> &Pad {
        &self.pad
    }

    /// Ends the tick: merges the devices, computes the edges, derives the axes.
    ///
    /// An axis a device produces digitally (a key, a d-pad) is -1, 0 or 1; a
    /// stick produces a continuum. Whichever is further from centre wins, so
    /// resting on the stick does not veto the d-pad and vice versa.
    pub fn snapshot(&mut self) -> InputSnapshot {
        let pad = self.pad.poll();
        // The keyboard's *taps* as well as what it still holds - see
        // [`Keyboard::take_taps`]. The pad is polled rather than
        // event-driven, so it has no equivalent to latch.
        self.buttons
            .begin_frame(self.keyboard.held_mask() | self.keyboard.take_taps() | pad.held);

        let digital_x = axis(
            self.buttons.is_held(Button::Right),
            self.buttons.is_held(Button::Left),
        );
        let digital_y = axis(
            self.buttons.is_held(Button::Up),
            self.buttons.is_held(Button::Down),
        );
        let shoulder = |button: Button| f32::from(u8::from(self.buttons.is_held(button)));

        InputSnapshot {
            buttons: self.buttons,
            stick_x: pad::larger(digital_x, pad.stick_x),
            stick_y: pad::larger(digital_y, pad.stick_y),
            airbrake_left: shoulder(Button::L).max(pad.airbrake_left),
            airbrake_right: shoulder(Button::R).max(pad.airbrake_right),
        }
        .sanitised()
    }

    /// The button state as of the last [`Self::snapshot`], which is what the
    /// front end drives itself off.
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
        assert!(snapshot.buttons.is_pressed(Button::Start));
        assert!(snapshot.buttons.is_held(Button::Start));

        let snapshot = keyboard.snapshot();
        assert!(
            !snapshot.buttons.is_pressed(Button::Start),
            "held, not pressed"
        );
        assert!(snapshot.buttons.is_held(Button::Start));
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
        assert!(snapshot.buttons.is_held(Button::Left));
        assert!(snapshot.buttons.is_held(Button::Right));
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
        assert!(snapshot.buttons.is_released(Button::Left));
    }

    /// The merge has to be one `Input`, or a press on one device would be an
    /// edge the other device's `consume_press` cannot clear.
    #[test]
    fn controls_merge_the_keyboard_into_one_button_state() {
        let mut controls = Controls::without_pad();
        controls.set_key(&Key::Named(NamedKey::Space), true);
        let snapshot = controls.snapshot();
        assert!(snapshot.buttons.is_pressed(Button::Start));

        controls.buttons_mut().consume_press(Button::Start);
        assert!(!controls.buttons().is_pressed(Button::Start));
        assert!(controls.buttons().is_held(Button::Start));
    }

    /// A pad is read fresh every tick, so `Controls` still works as a keyboard
    /// on a machine with none - which is every headless capture and CI run.
    #[test]
    fn controls_steer_from_the_keyboard_with_no_pad_attached() {
        let mut controls = Controls::without_pad();
        controls.set_key(&Key::Character("a".into()), true);
        assert_eq!(controls.snapshot().stick_x, -1.0);

        controls.set_key(&Key::Character("q".into()), true);
        assert_eq!(controls.snapshot().airbrake_left, 1.0);

        controls.release_all();
        let snapshot = controls.snapshot();
        assert_eq!(snapshot.stick_x, 0.0);
        assert_eq!(snapshot.airbrake_left, 0.0);
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

    /// Finding U5's guard: a press and release entirely between two reads is
    /// still a press.
    ///
    /// Both halves matter - the tap has to *arrive*, and it has to arrive as an
    /// edge that goes away again, or a menu confirm would repeat for ever.
    #[test]
    fn a_tap_between_two_snapshots_is_not_lost() {
        let mut keyboard = Keyboard::new();
        let key = Key::Named(NamedKey::Enter);

        keyboard.set_key(&key, true);
        keyboard.set_key(&key, false);
        assert_eq!(keyboard.held_mask(), 0, "nothing is held any more");

        let first = keyboard.snapshot();
        assert!(
            first.buttons.is_held(Button::Cross),
            "the tap was dropped between the two reads"
        );

        let second = keyboard.snapshot();
        assert!(
            !second.buttons.is_held(Button::Cross),
            "a latched tap must release, or it repeats for ever"
        );
    }

    /// Focus loss clears the latch too: a tap the player made on the way out
    /// belongs to whatever they switched to.
    #[test]
    fn release_all_drops_a_latched_tap() {
        let mut keyboard = Keyboard::new();
        keyboard.set_key(&Key::Named(NamedKey::Enter), true);
        keyboard.set_key(&Key::Named(NamedKey::Enter), false);
        keyboard.release_all();
        assert!(!keyboard.snapshot().buttons.is_held(Button::Cross));
    }
}
