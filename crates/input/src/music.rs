//! Pad controls for the race music: which pad control means which [`MusicAction`], and what a
//! pad did with them since the last look.
//!
//! These controls are read from the pad's own events, apart from the abstract buttons in
//! [`crate::pad::map_button`]: they never reach an `InputSnapshot`, `World` or a replay. Only a
//! control `map_button` leaves unbound may appear in [`MUSIC_BINDINGS`], so a press does not
//! also steer or confirm.

/// What a music control does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicAction {
    /// Show the current song's name again.
    ShowSong,
    /// Start the current song from its beginning.
    RestartSong,
    /// Skip to the next song.
    NextSong,
    /// Go back to the previous song.
    PreviousSong,
    /// Held: raise the music, then, at its maximum, lower everything else.
    MusicUp,
    /// Held: undo what [`Self::MusicUp`] did, then lower the music.
    MusicDown,
}

impl MusicAction {
    /// Every action, in bit order.
    pub const ALL: [Self; 6] = [
        Self::ShowSong,
        Self::RestartSong,
        Self::NextSong,
        Self::PreviousSong,
        Self::MusicUp,
        Self::MusicDown,
    ];

    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// A pad control: one the backend names, or one it only has a raw code for (a back paddle the
/// kernel reports as an extra button).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadControl {
    /// A button the backend names.
    Named(gilrs::Button),
    /// A button the backend does not name, by its raw event code.
    Raw(u32),
}

/// Which control does which action. **The one place the binding lives**: a pad rebinding
/// screen edits this table's rows, and a pad with a different back-button code changes one row.
/// Empty until a control is chosen for each action.
pub const MUSIC_BINDINGS: &[(PadControl, MusicAction)] = &[];

/// What the pads did with music controls since the last [`Pad::take_music`](crate::Pad).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MusicInput {
    pressed: u8,
    held: u8,
}

impl MusicInput {
    /// What a pad that pressed `pressed` and holds `held` reports; for a test that has no pad.
    #[must_use]
    pub fn new(pressed: &[MusicAction], held: &[MusicAction]) -> Self {
        let mask = |actions: &[MusicAction]| actions.iter().fold(0, |bits, a| bits | a.bit());
        Self {
            pressed: mask(pressed),
            held: mask(held),
        }
    }

    /// Whether `action`'s control went down since the last look, even if it came up again
    /// within the same frame.
    #[must_use]
    pub fn pressed(self, action: MusicAction) -> bool {
        self.pressed & action.bit() != 0
    }

    /// Whether `action`'s control is down now.
    #[must_use]
    pub fn held(self, action: MusicAction) -> bool {
        self.held & action.bit() != 0
    }

    /// Whether nothing happened.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.pressed == 0 && self.held == 0
    }
}

/// Tracks the music controls from pad events.
#[derive(Debug, Default)]
pub struct Tracker {
    pressed: u8,
    held: u8,
}

impl Tracker {
    /// Notes one event: `down` for a press, `false` for a release. `code` is the raw event
    /// code, which only a [`PadControl::Raw`] row reads.
    pub fn note(
        &mut self,
        bindings: &[(PadControl, MusicAction)],
        button: gilrs::Button,
        code: u32,
        down: bool,
    ) {
        for &(control, action) in bindings {
            let hit = match control {
                PadControl::Named(named) => button == named,
                PadControl::Raw(raw) => button == gilrs::Button::Unknown && code == raw,
            };
            if !hit {
                continue;
            }
            if down {
                self.pressed |= action.bit();
                self.held |= action.bit();
            } else {
                self.held &= !action.bit();
            }
        }
    }

    /// A pad went away: nothing it held is held any more.
    pub fn release_all(&mut self) {
        self.held = 0;
    }

    /// What happened since the last call. Presses are spent; what is held stays.
    pub fn take(&mut self) -> MusicInput {
        MusicInput {
            pressed: std::mem::take(&mut self.pressed),
            held: self.held,
        }
    }
}

#[cfg(test)]
mod tests;
