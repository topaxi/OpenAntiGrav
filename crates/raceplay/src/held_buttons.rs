//! The button path a headless run drives: a held-button mask turned into per-tick
//! [`InputSnapshot`]s through the real keyboard mapping.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/held_buttons.rs`.

use oag_gameplay::InputSnapshot;
use oag_gameplay::input::Button;
use oag_input::Keyboard;

/// A held-button mask, as one snapshot per tick, through the real keyboard path.
///
/// The headless capture and the ground-truth test both need input with no window.
/// They get it by pressing the **keys** a window would have seen, so there is
/// exactly one mapping from a device to a snapshot and the axes are derived in
/// exactly one place - [`oag_input::Keyboard`]. Deriving them a second time here is
/// how "left" ends up meaning two different things in two files.
#[derive(Debug, Default)]
pub struct HeldButtons {
    keyboard: Keyboard,
}

impl HeldButtons {
    /// Holds every button in `mask` down, and nothing else.
    ///
    /// Buttons with no key bound to them are skipped: a mask naming START in a race
    /// is not a fault condition.
    #[must_use]
    pub fn new(mask: u32) -> Self {
        let mut keyboard = Keyboard::new();
        for index in 0..32u8 {
            if mask & (1u32 << index) == 0 {
                continue;
            }
            if let Some(key) = key_for_button(Button::from_index(index)) {
                keyboard.set_key(&key, true);
            }
        }
        Self { keyboard }
    }

    /// One tick's snapshot.
    pub fn snapshot(&mut self) -> InputSnapshot {
        self.keyboard.snapshot()
    }

    /// Makes exactly `mask`'s buttons held, releasing everything else.
    ///
    /// For script-driven capture: each tick's authored state is a level, and
    /// the keyboard path derives the edges (and the axes) from the level
    /// changes exactly as it would for a real player. Unbound buttons are
    /// skipped, same as [`Self::new`].
    pub fn set_held(&mut self, mask: u32) {
        for index in 0..32u8 {
            if let Some(key) = key_for_button(Button::from_index(index)) {
                self.keyboard.set_key(&key, mask & (1u32 << index) != 0);
            }
        }
    }

    /// Sets `mask`'s buttons down and clears them, on top of what is held.
    ///
    /// For a gesture that needs *edges* rather than a level. A held button
    /// produces one rising edge and never another, so anything reading
    /// `Input::is_pressed` - the veteran sideshift's double tap, for one - is
    /// invisible to [`Self::new`]'s mask alone. Buttons in both masks stay down:
    /// holding and pulsing the same button is a contradiction, and resolving it
    /// toward held is the reading that does not silently drop a hold.
    pub fn pulse(&mut self, mask: u32, held: u32, down: bool) {
        for index in 0..32u8 {
            if mask & (1u32 << index) == 0 || held & (1u32 << index) != 0 {
                continue;
            }
            if let Some(key) = key_for_button(Button::from_index(index)) {
                self.keyboard.set_key(&key, down);
            }
        }
    }

    /// One tick's input: the authored script if there is one, else `--press`
    /// pulsed against `--held`.
    ///
    /// The branch every headless leg that can be script-driven needs, kept in
    /// one place so two legs cannot read it two different ways again - see
    /// `race::capture`'s `advance_one_tick`, which drove this out, and
    /// `--trace-out`'s `write_trace`, which never had it at all until this
    /// existed - a scripted `--trace-out` run used to write a plausible CSV
    /// of a craft that never moved, with no error.
    ///
    /// **Only the script's `buttons` reach the keyboard path** - `stick_x`,
    /// `stick_y` and the two `airbrake_*` fields a script can author are not
    /// read here, the same gap `advance_one_tick` already had. A script
    /// naming `stick_x=0.5` drives exactly as `left` does through this path,
    /// where `oag-trace run`/`replay` (`crates/trace/src/replay.rs`) applies
    /// the analog value directly. Not fixed here - the axes come out of
    /// [`Self::snapshot`]'s keyboard mapping, which has no room for an
    /// authored analog value at all.
    pub fn advance(
        &mut self,
        script: Option<&oag_trace::script::Script>,
        pressed: u32,
        held: u32,
        tick: u32,
    ) {
        if let Some(script) = script {
            self.set_held(script.at(tick as usize).buttons);
        } else {
            self.pulse(pressed, held, tick.is_multiple_of(2));
        }
    }
}

/// A key that produces the given abstract button.
///
/// The inverse of [`oag_input::keys::map_key`], for the buttons a race uses. An
/// inverse and not a second mapping: a unit test asserts every pair round-trips,
/// so this table cannot drift away from the one the window uses.
pub(crate) fn key_for_button(button: Button) -> Option<winit::keyboard::Key> {
    use winit::keyboard::{Key, NamedKey};

    let key = match button {
        Button::Up => Key::Named(NamedKey::ArrowUp),
        Button::Down => Key::Named(NamedKey::ArrowDown),
        Button::Left => Key::Named(NamedKey::ArrowLeft),
        Button::Right => Key::Named(NamedKey::ArrowRight),
        Button::Cross => Key::Character("x".into()),
        // Fire and absorb, the two buttons a pickup reads. The same keys
        // `oag_input::keys::map_key` binds them to, so a capture presses what a
        // player presses. **They were missing until weapons existed**, which
        // made `--press square` a silent no-op and a weapon capture impossible -
        // the mask named a button, `key_for_button` returned `None`, and the
        // skip this function documents swallowed it.
        Button::Square => Key::Character("c".into()),
        Button::Circle => Key::Character("z".into()),
        Button::L => Key::Character("q".into()),
        Button::R => Key::Character("e".into()),
        _ => return None,
    };
    Some(key)
}
