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
pub mod prompt;

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

    /// Presses `button` for one tick, as if a bound key had gone down and
    /// come up between two reads.
    ///
    /// **A pointer's way of pressing a button.** A screen that waits for
    /// start or cross and nothing else - the boot movies, `PRESS START`, the
    /// results table - has nothing to point *at*, so a click on it is the
    /// press; the composition root turns the one into the other here, and
    /// the screen reads it through the same [`Self::take_taps`] latch a
    /// real key does. Nothing about the tap says it was not a key, which is
    /// the point: the front end keeps one answer to "did the player press
    /// this". Never called from a race - a click is not thrust.
    pub fn tap(&mut self, button: Button) {
        self.tapped |= button.bit();
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
    /// One button state per grid slot, because an edge is per pilot.
    ///
    /// This was a single [`Input`] until 2026-09-16, which was right while
    /// every device fed one craft. `pressed = held & !held_last` has to be
    /// computed against *that slot's* previous frame, so two people cannot
    /// share one: player two letting go of cross would clear a press player one
    /// had not consumed yet.
    ///
    /// Under [`pad::Assignment`]'s default every device is on slot 0, so slot 0
    /// is the old single `Input` and the other seven never see a held bit.
    buttons: [Input; oag_gameplay::MAX_PLAYERS],
    /// Whether the pad contributed anything to the last [`Self::snapshot`]:
    /// a held button or a stick or trigger off centre. See [`Self::pad_spoke`].
    pad_spoke: bool,
    /// Which device was used last, for the button prompts' glyph family.
    prompt: prompt::Detector,
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
            buttons: [Input::new(); oag_gameplay::MAX_PLAYERS],
            pad_spoke: false,
            prompt: prompt::Detector::new(),
        }
    }

    /// Which slot each device drives. See [`pad::Assignment`].
    #[must_use]
    pub fn assignment(&self) -> &pad::Assignment {
        self.pad.assignment()
    }

    /// The assignment, to change - the one call that makes a device drive a
    /// craft other than slot 0's.
    pub fn assignment_mut(&mut self) -> &mut pad::Assignment {
        self.pad.assignment_mut()
    }

    /// Which slot the keyboard drives, and so which snapshot the front end
    /// reads. [`pad::Assignment::keyboard_slot`].
    #[must_use]
    pub fn keyboard_slot(&self) -> usize {
        self.pad.assignment().keyboard_slot()
    }

    /// Records a key going down or coming up. See [`Keyboard::set_key`].
    pub fn set_key(&mut self, key: &Key, pressed: bool) {
        if pressed {
            self.prompt.note_key();
        }
        self.keyboard.set_key(key, pressed);
    }

    /// The glyph family a prompt draws in under `style`, or `None` for the
    /// disc's own glyphs. See [`prompt::Detector::family`].
    #[must_use]
    pub fn prompt_family(&self, style: prompt::PromptStyle) -> Option<prompt::PromptFamily> {
        self.prompt.family(style)
    }

    /// Presses `button` for one tick on the keyboard's own latch. See
    /// [`Keyboard::tap`].
    pub fn tap(&mut self, button: Button) {
        self.keyboard.tap(button);
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
        let slot = self.keyboard_slot();
        *self.player_snapshots().get(slot)
    }

    /// Ends the tick for every grid slot at once: one snapshot per slot.
    ///
    /// **The call a race makes**, where [`Self::snapshot`] is the call the front
    /// end makes - one menu, one cursor, whichever slot the keyboard drives.
    /// The devices are polled exactly once here, which is why the two are not
    /// both callable in a frame: [`Keyboard::take_taps`] is destructive, and a
    /// second poll would find a tap already spent.
    ///
    /// Under [`pad::Assignment`]'s default every device drives slot 0, so slot
    /// 0's entry is what the single-snapshot path produced and the other seven
    /// are `InputSnapshot::default`.
    pub fn player_snapshots(&mut self) -> oag_gameplay::PlayerInputs {
        let pads = self.pad.poll_players();
        if let Some(family) = self.pad.take_activity() {
            self.prompt.note_pad(family);
        } else if let Some(family) = self.pad.first_family() {
            self.prompt.seed_pad(family);
        }
        self.merge_players(pads)
    }

    /// [`Self::player_snapshots`] with the pads' contributions supplied rather
    /// than polled.
    ///
    /// The whole merge lives here so a test can state a pad reading directly.
    /// Everything this function decides - which device wins an axis, and which
    /// bits count as a shoulder - is invisible to `player_snapshots`'s caller
    /// and impossible to reach on a machine with no pad, which is every CI run.
    fn merge_players(
        &mut self,
        pads: [pad::PadState; oag_gameplay::MAX_PLAYERS],
    ) -> oag_gameplay::PlayerInputs {
        // Any slot's pad speaking counts: what this answers is "is a pad the
        // device in use", which a cursor asks about the machine and not about a
        // craft.
        self.pad_spoke = pads.iter().any(|pad| {
            pad.held != 0
                || pad.stick_x != 0.0
                || pad.stick_y != 0.0
                || pad.airbrake_left != 0.0
                || pad.airbrake_right != 0.0
        });
        // The keyboard's *taps* as well as what it still holds - see
        // [`Keyboard::take_taps`]. The pad is polled rather than
        // event-driven, so it has no equivalent to latch. Taken once, for the
        // one slot the keyboard drives: taking them per slot would spend them
        // on the first and leave nothing for the rest.
        let keyboard_slot = self.keyboard_slot();
        let keyboard_keys = self.keyboard.held_mask() | self.keyboard.take_taps();

        let mut inputs = oag_gameplay::PlayerInputs::none();
        for (slot, pad) in pads.into_iter().enumerate() {
            let keys = if slot == keyboard_slot {
                keyboard_keys
            } else {
                0
            };
            inputs.set(slot, self.merge(slot, keys, pad));
        }
        inputs
    }

    /// [`Self::merge_players`] for one pad on the slot the keyboard drives.
    ///
    /// What the single-device merge always was, kept so a test can state a pad
    /// reading and read one snapshot back without spelling out eight.
    #[cfg(test)]
    fn merge_one(&mut self, pad: pad::PadState) -> InputSnapshot {
        let slot = self.keyboard_slot();
        let mut pads = [pad::PadState::default(); oag_gameplay::MAX_PLAYERS];
        pads[slot] = pad;
        *self.merge_players(pads).get(slot)
    }

    /// One slot's devices, as one snapshot.
    fn merge(&mut self, slot: usize, keys: u32, pad: pad::PadState) -> InputSnapshot {
        let buttons = &mut self.buttons[slot];
        buttons.begin_frame(keys | pad.held);

        let digital_x = axis(
            buttons.is_held(Button::Right),
            buttons.is_held(Button::Left),
        );
        let digital_y = axis(buttons.is_held(Button::Up), buttons.is_held(Button::Down));
        // Off the *keyboard's* bits, not the merged mask. A pad trigger under
        // `pad::TriggerMode::Airbrakes` contributes its shoulder's bit as well
        // as an analog value, so reading the merge here would answer `1.0` for
        // a trigger at `0.4` and `.max` would quantise the pull away - the same
        // trap `pad::resolve` documents, one layer up. The taps are folded in
        // because a Q pressed and released between two reads is still a full
        // airbrake for its frame; see [`Keyboard::take_taps`].
        let shoulder = |button: Button| f32::from(u8::from(keys & button.bit() != 0));

        InputSnapshot {
            buttons: *buttons,
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
        &self.buttons[self.keyboard_slot()]
    }

    /// Mutable button state, for [`Input::consume_press`].
    pub fn buttons_mut(&mut self) -> &mut Input {
        let slot = self.keyboard_slot();
        &mut self.buttons[slot]
    }

    /// Whether the pad contributed anything to the last [`Self::snapshot`].
    ///
    /// What the composition root reads to decide that the pad, rather than
    /// the mouse, is the device in use - and so to stop drawing a cursor.
    /// Off the pad's own reading and not the merged `Input`, because a
    /// click presses a button through the keyboard's latch ([`Self::tap`])
    /// and that press is the mouse speaking, not the pad.
    #[must_use]
    pub fn pad_spoke(&self) -> bool {
        self.pad_spoke
    }
}

/// Two opposing keys into one axis. Both down cancel.
fn axis(positive: bool, negative: bool) -> f32 {
    f32::from(i8::from(positive) - i8::from(negative))
}

#[cfg(test)]
mod tests;
