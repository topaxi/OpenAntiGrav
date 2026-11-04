//! Pad controls for the race music: which pad control means which [`MusicAction`], and what a
//! pad did with them since the last look.
//!
//! These controls are read from the pad's own events, apart from the abstract buttons in
//! [`crate::pad::map_button`]: they never reach an `InputSnapshot`, `World` or a replay. Only a
//! control `map_button` leaves unbound may appear in a [`MusicBindings`], so a press does not
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

    /// The `[controls.music]` key that binds this action.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::ShowSong => "show_song",
            Self::RestartSong => "restart_song",
            Self::NextSong => "next_song",
            Self::PreviousSong => "previous_song",
            Self::MusicUp => "music_up",
            Self::MusicDown => "music_down",
        }
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

/// Every button the backend names, by the name a settings file writes (its variant name).
const NAMED: &[(&str, gilrs::Button)] = &[
    ("South", gilrs::Button::South),
    ("East", gilrs::Button::East),
    ("North", gilrs::Button::North),
    ("West", gilrs::Button::West),
    ("C", gilrs::Button::C),
    ("Z", gilrs::Button::Z),
    ("LeftTrigger", gilrs::Button::LeftTrigger),
    ("LeftTrigger2", gilrs::Button::LeftTrigger2),
    ("RightTrigger", gilrs::Button::RightTrigger),
    ("RightTrigger2", gilrs::Button::RightTrigger2),
    ("Select", gilrs::Button::Select),
    ("Start", gilrs::Button::Start),
    ("Mode", gilrs::Button::Mode),
    ("LeftThumb", gilrs::Button::LeftThumb),
    ("RightThumb", gilrs::Button::RightThumb),
    ("DPadUp", gilrs::Button::DPadUp),
    ("DPadDown", gilrs::Button::DPadDown),
    ("DPadLeft", gilrs::Button::DPadLeft),
    ("DPadRight", gilrs::Button::DPadRight),
];

impl PadControl {
    /// Reads a settings value: a named button (`RightThumb`, any case) or `raw:CODE`, the code
    /// in decimal or `0x` hex, as `--probe-pad` prints it.
    ///
    /// # Errors
    /// Says what is wrong with `text`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if let Some(code) = text.strip_prefix("raw:") {
            let code = code.trim();
            let parsed = match code.strip_prefix("0x") {
                Some(hex) => u32::from_str_radix(hex, 16),
                None => code.parse(),
            };
            return parsed
                .map(Self::Raw)
                .map_err(|_| format!("{text:?}: raw: wants a whole number such as raw:300"));
        }
        NAMED
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(text))
            .map(|&(_, button)| Self::Named(button))
            .ok_or_else(|| {
                let names: Vec<&str> = NAMED.iter().map(|&(name, _)| name).collect();
                format!(
                    "{text:?} is not a button name ({}) or raw:CODE",
                    names.join(", ")
                )
            })
    }

    /// The value that parses back to this control; what `--probe-pad` prints.
    #[must_use]
    pub fn describe(button: gilrs::Button, code: u32) -> String {
        match NAMED.iter().find(|&&(_, named)| named == button) {
            Some(&(name, _)) => name.to_string(),
            None => format!("raw:{code}"),
        }
    }
}

/// Which control does which action: the table a session holds, filled from `[controls.music]`.
/// Every action is unbound until a settings file binds it. **The one place the binding
/// lives**; a pad rebinding screen would edit its rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MusicBindings {
    rows: Vec<(PadControl, MusicAction)>,
}

impl MusicBindings {
    /// Reads `[controls.music]`'s pairs. Returns what was kept and one message per entry it
    /// dropped (unknown key, unreadable value, a control that is also a game button): that
    /// action stays unbound.
    pub fn from_pairs<'a>(
        pairs: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> (Self, Vec<String>) {
        let mut rows = Vec::new();
        let mut problems = Vec::new();
        for (key, value) in pairs {
            let Some(action) = MusicAction::ALL.into_iter().find(|a| a.key() == key) else {
                problems.push(format!("{key:?} is not a music action"));
                continue;
            };
            match PadControl::parse(value) {
                Ok(PadControl::Named(button)) if crate::pad::map_button(button).is_some() => {
                    problems.push(format!("{key} = {value:?} is a game button"));
                }
                Ok(control) => rows.push((control, action)),
                Err(why) => problems.push(format!("{key} = {why}")),
            }
        }
        (Self { rows }, problems)
    }

    /// The rows, for [`Tracker::note`].
    #[must_use]
    pub fn rows(&self) -> &[(PadControl, MusicAction)] {
        &self.rows
    }
}

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

    /// [`Self::note`] for an event described as the control it came from; for a caller with no
    /// `gilrs` types to name.
    pub fn note_control(&mut self, bindings: &MusicBindings, control: PadControl, down: bool) {
        let (button, code) = match control {
            PadControl::Named(button) => (button, 0),
            PadControl::Raw(code) => (gilrs::Button::Unknown, code),
        };
        self.note(bindings.rows(), button, code, down);
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
