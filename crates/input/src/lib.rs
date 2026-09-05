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

pub mod bindings;
pub mod keys;
pub mod pad;

use oag_gameplay::InputSnapshot;
use oag_gameplay::input::{Button, Input};
use winit::keyboard::Key;

pub use bindings::Bindings;
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
    /// The live key-to-button table. [`bindings::Bindings::default`] until a
    /// player rebinds something - see [`Self::set_bindings`].
    bindings: Bindings,
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
        let Some(button) = self.bindings.resolve(key) else {
            return;
        };
        let bit = button.bit();
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

    /// Replaces the live key-to-button table, for a settings file that has
    /// rebound something.
    ///
    /// Read at boot from `oag_game::settings::Controls::bindings` -
    /// `main::args::resolve_bindings` - and again after every interactive
    /// rebind, so [`Self::set_key`] never resolves through a stale table.
    pub fn set_bindings(&mut self, bindings: Bindings) {
        self.bindings = bindings;
    }

    /// The live table, for a rebind to mutate through
    /// [`bindings::Bindings::rebind`] or a Controls page to read through
    /// [`bindings::Bindings::names_for`].
    #[must_use]
    pub fn bindings(&self) -> &Bindings {
        &self.bindings
    }

    /// Mutable access to the live table, for [`Self::set_bindings`]'s
    /// interactive counterpart: a rebind captured off a raw key event.
    pub fn bindings_mut(&mut self) -> &mut Bindings {
        &mut self.bindings
    }

    /// Which keys currently produce `button`, off the **live** table.
    ///
    /// [`keys::bound_keys`]'s counterpart for a keyboard that may have been
    /// rebound - what a Controls page's binding row should draw from, so it
    /// cannot show the default while the game obeys something else.
    #[must_use]
    pub fn bound_keys(&self, button: Button) -> Vec<&'static str> {
        self.bindings.names_for(button)
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

    /// Binds the pad's analog triggers. See [`Pad::set_trigger_mode`].
    pub fn set_trigger_mode(&mut self, mode: pad::TriggerMode) {
        self.pad.set_trigger_mode(mode);
    }

    /// Sets the pad's trigger response curve. See [`Pad::set_trigger_curve`].
    pub fn set_trigger_curve(&mut self, curve: f32) {
        self.pad.set_trigger_curve(curve);
    }

    /// Replaces the keyboard's live key-to-button table. See
    /// [`Keyboard::set_bindings`].
    pub fn set_bindings(&mut self, bindings: Bindings) {
        self.keyboard.set_bindings(bindings);
    }

    /// The keyboard's live table. See [`Keyboard::bindings`].
    #[must_use]
    pub fn bindings(&self) -> &Bindings {
        self.keyboard.bindings()
    }

    /// Mutable access to the keyboard's live table. See
    /// [`Keyboard::bindings_mut`].
    pub fn bindings_mut(&mut self) -> &mut Bindings {
        self.keyboard.bindings_mut()
    }

    /// Which keys currently produce `button`, off the live table. See
    /// [`Keyboard::bound_keys`].
    #[must_use]
    pub fn bound_keys(&self, button: Button) -> Vec<&'static str> {
        self.keyboard.bound_keys(button)
    }

    /// Ends the tick: merges the devices, computes the edges, derives the axes.
    ///
    /// An axis a device produces digitally (a key, a d-pad) is -1, 0 or 1; a
    /// stick produces a continuum. Whichever is further from centre wins, so
    /// resting on the stick does not veto the d-pad and vice versa.
    pub fn snapshot(&mut self) -> InputSnapshot {
        let pad = self.pad.poll();
        self.merge(pad)
    }

    /// [`Self::snapshot`] with the pad's contribution supplied rather than
    /// polled.
    ///
    /// The whole merge lives here so a test can state a pad reading directly.
    /// Everything this function decides - which device wins an axis, and which
    /// bits count as a shoulder - is invisible to `snapshot`'s caller and
    /// impossible to reach on a machine with no pad, which is every CI run.
    fn merge(&mut self, pad: pad::PadState) -> InputSnapshot {
        // The keyboard's *taps* as well as what it still holds - see
        // [`Keyboard::take_taps`]. The pad is polled rather than
        // event-driven, so it has no equivalent to latch.
        let keys = self.keyboard.held_mask() | self.keyboard.take_taps();
        self.buttons.begin_frame(keys | pad.held);

        let digital_x = axis(
            self.buttons.is_held(Button::Right),
            self.buttons.is_held(Button::Left),
        );
        let digital_y = axis(
            self.buttons.is_held(Button::Up),
            self.buttons.is_held(Button::Down),
        );
        // Off the *keyboard's* bits, not the merged mask. A pad trigger under
        // `pad::TriggerMode::Airbrakes` contributes its shoulder's bit as well
        // as an analog value, so reading the merge here would answer `1.0` for
        // a trigger at `0.4` and `.max` would quantise the pull away - the same
        // trap `pad::resolve` documents, one layer up. The taps are folded in
        // because a Q pressed and released between two reads is still a full
        // airbrake for its frame; see [`Keyboard::take_taps`].
        let shoulder = |button: Button| f32::from(u8::from(keys & button.bit() != 0));

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
mod tests;
