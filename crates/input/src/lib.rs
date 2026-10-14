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

pub mod android;
pub mod bindings;
pub mod keys;
pub mod lobby;
pub mod pad;
pub mod prompt;
pub mod touch;

use oag_gameplay::InputSnapshot;
use oag_gameplay::input::{Button, Input};
use winit::keyboard::Key;

pub use bindings::Bindings;
pub use pad::{Pad, Reading};

/// Accumulates key state between ticks and hands out one snapshot per tick.
///
/// Kept separate from the event loop so a test can drive it by calling
/// [`Self::set_key`] directly, with no window and no compositor. The last
/// session could not verify the viewer's interactive camera for exactly the
/// opposite reason: there was no way to inject a key event.
#[derive(Debug, Clone, Default)]
pub struct Keyboard {
    held: u32,
    /// Buttons whose keys were held when [`Self::let_go`] released them. A press of such a
    /// key is ignored until it has been seen to come up (or [`Self::forget_let_go`]), because
    /// the platform may go on delivering the held key's auto-repeat to the window that gains the
    /// focus, as a press that is not marked a repeat.
    let_go: u32,
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
        self.press(button, pressed);
    }

    /// [`Self::set_key`] for a key already resolved to its button: what a seat other than
    /// the first receives, resolved through the first keyboard's live table.
    pub fn press(&mut self, button: Button, pressed: bool) {
        let bit = button.bit();
        if pressed {
            if self.let_go & bit != 0 {
                return;
            }
            self.held |= bit;
            self.tapped |= bit;
        } else {
            self.held &= !bit;
            self.let_go &= !bit;
        }
    }

    /// [`Self::release_all`] for a keyboard whose seat is being moved while keys are still down:
    /// what was held stays released until its key is pressed again, however the platform
    /// reports the key it is still holding down.
    pub fn let_go(&mut self) {
        self.let_go |= self.held;
        self.release_all();
    }

    /// Forgets which keys [`Self::let_go`] is waiting to see come up: the next press of any
    /// key counts. For when the keyboard's real state is unknown (nothing has had focus).
    pub fn forget_let_go(&mut self) {
        self.let_go = 0;
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
    /// An Android gamepad, which `gilrs` cannot see. See [`android`].
    android: android::AndroidPad,
    /// An Android pad event arrived since the last tick, held or not, so a
    /// press shorter than a tick still counts as the pad speaking.
    android_spoke: bool,
    /// What the on-screen racing controls hold this tick. See [`touch`]. Not
    /// a pad: it never counts toward [`Self::pad_spoke`].
    touch: Reading,
    /// Who pressed what since the last [`Self::player_snapshots`], for the join
    /// screen. See [`Self::device_presses`].
    presses: Vec<lobby::Press>,
    /// The keyboard's bits as of the last tick, to find the ones that went down.
    last_keys: u32,
    /// The keyboard seats after the first ([`pad::Assignment::keyboard_seats`]): the keys each
    /// one holds, by slot. Resolved through [`Self::keyboard`]'s table, which a rebind changes.
    more_keys: Vec<SeatKeys>,
    /// Physical keyboards the platform has told apart, in the order they were first heard.
    /// See [`Self::keyboard_id`].
    devices: Vec<winit::event::DeviceId>,
    /// Keyboards that have pressed a bound key since the last seating, in a race dealt across
    /// windows. See [`Self::device_key`].
    speakers: Vec<(pad::KeyboardId, Vec<usize>)>,
    /// Whether race keys now come per physical keyboard rather than per window. See
    /// [`Self::per_device_keys`].
    per_device: bool,
    /// Buttons each physical keyboard pressed since the last tick, from the raw key events
    /// the join screen listens to ([`Self::lobby_key`]).
    raw_edges: Vec<(pad::KeyboardId, u32)>,
    /// Whether a raw key event has come in since the join screen opened: from then on the
    /// window's own key presses are the same presses again and are not the keyboard's
    /// ([`Self::lobby_key`]).
    raw_seen: bool,
    /// Buttons the window's key events pressed since the last tick, as distinct from a click
    /// or a touch tap, which arrive through the same latch.
    window_pressed: u32,
}

/// What a keyboard seat after the first holds.
#[derive(Debug, Clone, Default)]
struct SeatKeys {
    slot: usize,
    keys: Keyboard,
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
            android: android::AndroidPad::default(),
            android_spoke: false,
            touch: Reading::default(),
            presses: Vec::new(),
            last_keys: 0,
            more_keys: Vec::new(),
            devices: Vec::new(),
            speakers: Vec::new(),
            per_device: false,
            raw_edges: Vec::new(),
            raw_seen: false,
            window_pressed: 0,
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

    /// Who pressed which buttons in the last [`Self::player_snapshots`]: each pad
    /// that did, off its own press events, and the keyboard (which is also the
    /// mouse and touch, whose clicks arrive as [`Self::tap`]s) when a key went
    /// down. What the join screen reads, because before anyone has joined every
    /// device is merged into slot 0 and the snapshots cannot say which pad pressed.
    #[must_use]
    pub fn device_presses(&self) -> &[lobby::Press] {
        &self.presses
    }

    /// The real pads connected now, in the order the backend lists them.
    #[must_use]
    pub fn connected_pads(&self) -> Vec<gilrs::GamepadId> {
        self.pad.connected()
    }

    /// The backend's name for a pad.
    #[must_use]
    pub fn pad_name(&self, id: gilrs::GamepadId) -> Option<String> {
        self.pad.name_of(id)
    }

    /// Seats `humans` people the default way - keyboard and first pad on slot 0,
    /// each further pad on the next slot - for a race that was not set up on the
    /// join screen (`--players N`). See [`pad::Assignment::seated`].
    pub fn seat_default(&mut self, humans: usize) {
        *self.pad.assignment_mut() = pad::Assignment::seated(humans);
        self.forget_keyboards();
    }

    /// Back to every device on slot 0, which is what the menus need: one cursor,
    /// driven by any device. Called when a split-screen race hands back to the
    /// front end.
    pub fn merge_devices(&mut self) {
        *self.pad.assignment_mut() = pad::Assignment::default();
        self.forget_keyboards();
    }

    /// Forgets which physical keyboards have spoken and every seat after the first: a new
    /// seating starts the claims over.
    fn forget_keyboards(&mut self) {
        self.more_keys.clear();
        self.speakers.clear();
        self.per_device = false;
    }

    /// Every keyboard, the first included.
    fn each_keyboard(&mut self) -> impl Iterator<Item = &mut Keyboard> {
        std::iter::once(&mut self.keyboard)
            .chain(self.more_keys.iter_mut().map(|seat| &mut seat.keys))
    }

    /// Lets go of every key the way a move to another seat does ([`Keyboard::let_go`]): what
    /// is held stays released until it is pressed again. What the whole run losing the focus
    /// does in a race dealt across windows.
    pub fn let_go(&mut self) {
        self.each_keyboard().for_each(Keyboard::let_go);
        self.release_all();
    }

    /// [`Self::let_go`] for the keyboard seats in `window` (the seats of the window that lost
    /// the focus) that take the window's keys. A keyboard bound to its physical device keeps
    /// its keys: the raw events it listens to do not follow a window's focus.
    pub fn let_go_window(&mut self, window: &[usize]) {
        let shared: Vec<usize> = self
            .pad
            .assignment()
            .keyboard_seats()
            .into_iter()
            .filter(|(binding, slot)| *binding == pad::Binding::Shared && window.contains(slot))
            .map(|(_, slot)| slot)
            .collect();
        for slot in shared {
            self.keyboard_at(slot).let_go();
        }
    }

    /// [`Self::let_go`] for the keyboard seats after the first only: what the race parked behind
    /// window 1's menu does, where the first keyboard is the menu's and the others' keys
    /// (which the menu does not read) would otherwise come back held when the race resumes.
    pub fn let_go_extra_seats(&mut self) {
        self.more_keys
            .iter_mut()
            .for_each(|seat| seat.keys.let_go());
    }

    /// See [`Keyboard::forget_let_go`].
    pub fn forget_let_go(&mut self) {
        self.each_keyboard().for_each(Keyboard::forget_let_go);
    }

    /// The keyboard that flies `slot`: the first, or a later seat's, made on first use.
    fn keyboard_at(&mut self, slot: usize) -> &mut Keyboard {
        if slot == self.keyboard_slot() {
            return &mut self.keyboard;
        }
        let at = match self.more_keys.iter().position(|seat| seat.slot == slot) {
            Some(at) => at,
            None => {
                self.more_keys.push(SeatKeys {
                    slot,
                    ..SeatKeys::default()
                });
                self.more_keys.len() - 1
            }
        };
        &mut self.more_keys[at].keys
    }

    /// The id for a physical keyboard the platform reported, numbered in the order they are first
    /// heard from.
    pub fn keyboard_id(&mut self, device: winit::event::DeviceId) -> pad::KeyboardId {
        let at = self
            .devices
            .iter()
            .position(|seen| *seen == device)
            .unwrap_or_else(|| {
                self.devices.push(device);
                self.devices.len() - 1
            });
        pad::KeyboardId(u32::try_from(at).unwrap_or(u32::MAX))
    }

    /// A key going down or coming up on a window whose seats are `window`, in a race dealt
    /// across windows: it goes to that window's keyboard seat, a press that finds none claims
    /// the window's free seat, and in a window that holds no keyboard seat and has none to
    /// claim it is dropped. See [`pad::Assignment::route_key`].
    pub fn window_key(&mut self, window: &[usize], key: &Key, pressed: bool) {
        self.route(None, window, &[], key, pressed);
    }

    /// A key going down or coming up on physical keyboard `device`, in a race dealt across
    /// windows, with `window` the seats of the window in focus (a raw key event names no
    /// window) and `race` every seat of the race. **Inert until a second keyboard has pressed a bound key**: with one keyboard
    /// the window's own events carry the keys ([`Self::window_key`]) and a single keyboard
    /// may move between windows. From then on ([`Self::per_device_keys`]) each keyboard is
    /// bound to the seat it first pressed in and keeps it whichever window has the focus.
    pub fn device_key(
        &mut self,
        device: pad::KeyboardId,
        window: &[usize],
        race: &[usize],
        key: &Key,
        pressed: bool,
    ) {
        if pressed && !self.per_device && self.keyboard.bindings().resolve(key).is_some() {
            match self.speakers.iter_mut().find(|(id, _)| *id == device) {
                Some((_, last)) => *last = window.to_vec(),
                None => self.speakers.push((device, window.to_vec())),
            }
            if self.speakers.len() >= 2 {
                // The keyboards that spoke before keep the seats the window events gave them, and
                // the keys they hold: each is bound where it last pressed, ahead of the one that
                // just spoke, and its release now arrives as a raw event instead of a window's.
                // (A raw event comes ahead of the window event for the same press - measured on
                // X11, assumed on Windows - so the window event of this very press is already
                // dropped. If a platform reverses the order the press lands on the earlier
                // keyboard's seat and is never released there.)
                self.per_device = true;
                log::info!(
                    "keyboards: a second keyboard spoke, race keys now come per physical keyboard"
                );
                let earlier: Vec<_> = self.speakers.clone();
                for (id, last) in earlier.into_iter().filter(|(id, _)| *id != device) {
                    self.pad
                        .assignment_mut()
                        .route_key(Some(id), &last, race, true);
                }
            }
        }
        if self.per_device {
            self.route(Some(device), window, race, key, pressed);
        }
    }

    /// Whether race keys come from [`Self::device_key`] now: the window's own key events are
    /// then the same presses twice, and a windowed race drops them.
    #[must_use]
    pub fn per_device_keys(&self) -> bool {
        self.per_device
    }

    fn route(
        &mut self,
        device: Option<pad::KeyboardId>,
        window: &[usize],
        race: &[usize],
        key: &Key,
        pressed: bool,
    ) {
        let Some(button) = self.keyboard.bindings().resolve(key) else {
            return;
        };
        if pressed {
            self.prompt.note_key();
        }
        if let Some(slot) = self
            .pad
            .assignment_mut()
            .route_key(device, window, race, pressed)
        {
            self.keyboard_at(slot).press(button, pressed);
        }
    }

    /// One tap of `button` from physical keyboard `id`, as the join and seat screens read it
    /// ([`lobby::Device::Keys`]): what a timed script presses for a placeholder seat.
    pub fn press_keyboard_device(&mut self, id: pad::KeyboardId, button: Button) {
        self.raw_edges.push((id, button.bit()));
    }

    /// A button going down or up with no key behind it: what a timed script
    /// (`oag_game::devrun`) taps. The keyboard's own slot, as a key press is.
    pub fn press_button(&mut self, button: Button, pressed: bool) {
        if pressed {
            self.window_pressed |= button.bit();
        }
        self.keyboard.press(button, pressed);
    }

    /// Records a key going down or coming up. See [`Keyboard::set_key`].
    pub fn set_key(&mut self, key: &Key, pressed: bool) {
        if pressed {
            self.prompt.note_key();
            if let Some(button) = self.keyboard.bindings().resolve(key) {
                self.window_pressed |= button.bit();
            }
        }
        self.keyboard.set_key(key, pressed);
    }

    /// A key going down on physical keyboard `device`, for the join screen: it is that
    /// keyboard's press ([`lobby::Device::Keys`]) in [`Self::device_presses`]. **From the first
    /// one on, the window's own key presses are not the keyboard's any more** (a raw event
    /// arrives ahead of the window event for the same press: measured on X11, assumed on
    /// Windows), so one keypress is one device and not two; a click or a touch is still the
    /// anonymous [`lobby::Device::Keyboard`]. A key no button is bound to presses nothing.
    pub fn lobby_key(&mut self, device: pad::KeyboardId, key: &Key, pressed: bool) {
        if !pressed {
            return;
        }
        let Some(button) = self.keyboard.bindings().resolve(key) else {
            return;
        };
        self.prompt.note_key();
        if !self.raw_seen {
            log::info!(
                "keyboards: a raw key event names the keyboard, so the join screen tells keyboards apart"
            );
        }
        self.raw_seen = true;
        match self.raw_edges.iter_mut().find(|(id, _)| *id == device) {
            Some((_, bits)) => *bits |= button.bit(),
            None => self.raw_edges.push((device, button.bit())),
        }
    }

    /// Whether raw key events have told keyboards apart since the join screen opened. A click
    /// on the empty seat does not seat the mouse then: it is not a keyboard.
    #[must_use]
    pub fn keyboards_told_apart(&self) -> bool {
        self.raw_seen
    }

    /// Starts the join screen's keyboard bookkeeping over: no raw event heard, no press
    /// pending. Called as the screen opens and as it closes.
    pub fn forget_lobby_keys(&mut self) {
        self.raw_edges.clear();
        self.raw_seen = false;
        self.window_pressed = 0;
    }

    /// After a lobby seated the keyboards ([`lobby::Lobby::seat`]): a race of two or more
    /// physical keyboards reads its keys from their raw events from the first tick, each
    /// keyboard on the seat it joined, as if every one had already spoken
    /// ([`Self::per_device_keys`]). One keyboard keeps the window's own events.
    pub fn sync_device_keys(&mut self) {
        if self.pad.assignment().device_keyboards() >= 2 {
            self.per_device = true;
        }
    }

    /// The device used last, for the log line that names the prompt family.
    #[must_use]
    pub fn prompt_used(&self) -> prompt::Used {
        self.prompt.used()
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
        self.each_keyboard().for_each(Keyboard::release_all);
        self.android.release_all();
        self.touch = Reading::default();
    }

    /// An Android gamepad key event. Returns whether it was a pad key, so the
    /// caller does not also hand it to [`Self::set_key`]. A press is latched
    /// as a tap too, so one shorter than a tick still reaches the menus.
    pub fn android_key(&mut self, code: u32, pressed: bool) -> bool {
        let spent = self.android.key(code, pressed);
        self.android_spoke |= spent;
        if spent
            && pressed
            && let Some(android::Key::Button(button)) = android::key_of(code)
        {
            self.keyboard.tap(button);
        }
        spent
    }

    /// The Android gamepad's left stick, `AXIS_X` and `AXIS_Y` as reported.
    pub fn android_stick(&mut self, axis_x: f32, axis_y: f32) {
        self.android.stick(axis_x, axis_y);
        self.android_spoke |= axis_x != 0.0 || axis_y != 0.0;
    }

    /// Whether an Android gamepad has ever spoken.
    #[must_use]
    pub fn android_pad_seen(&self) -> bool {
        self.android.seen()
    }

    /// What the on-screen racing controls hold, for the next tick. Cleared by
    /// calling it with `Reading::default()`.
    pub fn set_touch(&mut self, reading: Reading) {
        self.touch = reading;
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
        let mut pads = self.pad.poll_players();
        self.presses = self
            .pad
            .take_presses()
            .into_iter()
            .map(|(id, buttons)| lobby::Press {
                device: lobby::Device::Pad(id),
                buttons,
            })
            .collect();
        let android = pad::resolve(self.android.reading(), self.pad.triggers());
        if android != pad::PadState::default() {
            let slot = self.keyboard_slot();
            pads[slot] = pad::merge_states(pads[slot], android);
            self.prompt.note_pad(None);
        }
        if let Some(family) = self.pad.take_activity() {
            self.prompt.note_pad(family);
        } else if let Some(family) = self.pad.first_family() {
            self.prompt.seed_pad(family);
        }
        let inputs = self.merge_players(pads);
        self.pad_spoke |= std::mem::take(&mut self.android_spoke);
        inputs
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
        let mut pressed = keyboard_keys & !std::mem::replace(&mut self.last_keys, keyboard_keys);
        let window_pressed = std::mem::take(&mut self.window_pressed);
        if self.raw_seen {
            pressed &= !window_pressed;
        }
        if pressed != 0 {
            self.presses.push(lobby::Press {
                device: lobby::Device::Keyboard,
                buttons: pressed,
            });
        }
        for (id, buttons) in std::mem::take(&mut self.raw_edges) {
            self.presses.push(lobby::Press {
                device: lobby::Device::Keys(id),
                buttons,
            });
        }
        let mut pads = pads;
        if self.touch != Reading::default() {
            let touch = pad::resolve(self.touch, self.pad.triggers());
            pads[keyboard_slot] = pad::merge_states(pads[keyboard_slot], touch);
        }

        // A seat after the first that the assignment no longer has (a window closed, a new
        // seating) is gone with its keys.
        let seats = self.pad.assignment().keyboard_seats();
        self.more_keys
            .retain(|seat| seats.iter().any(|(_, slot)| *slot == seat.slot));
        let mut inputs = oag_gameplay::PlayerInputs::none();
        for (slot, pad) in pads.into_iter().enumerate() {
            let keys = if slot == keyboard_slot {
                keyboard_keys
            } else {
                self.more_keys
                    .iter_mut()
                    .find(|seat| seat.slot == slot)
                    .map_or(0, |seat| seat.keys.held_mask() | seat.keys.take_taps())
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

    /// Takes every slot's press of `button` and says which slots made one, bit
    /// `slot` set for each: the keyboard's slot, or any seated pad's. What a split
    /// race's pause reads, since with devices seated a pad's Start lands on its
    /// own slot and [`Self::buttons`] (the keyboard's slot) never sees it - and the
    /// pause needs to know *who* pressed, because only they may resume. Every
    /// press is consumed. With devices merged every press is on slot 0.
    pub fn take_presses(&mut self, button: Button) -> u8 {
        let mut slots = 0;
        for (slot, input) in self.buttons.iter_mut().enumerate() {
            if input.is_pressed(button) {
                input.consume_press(button);
                slots |= 1 << slot;
            }
        }
        slots
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
