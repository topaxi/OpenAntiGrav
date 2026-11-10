//! `play-mod-sound`: one of the sending mod's own sounds (its `[sounds]`) from a craft.
//! An output: heard, never hashed, never read by the simulation.

use crate::{Command, CommandKind, Malformed, Slot};

/// What a `play-mod-sound` does with the sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundMode {
    /// Play it once from the craft.
    Once,
    /// Start it looping from the craft and keep it there, following the craft, until a
    /// [`Self::Stop`] for the same sound and slot, the module faulting or the race ending.
    /// Starting one already looping for that slot leaves it as it is.
    Loop,
    /// Stop the loop of the same sound and slot; none running is not a fault.
    Stop,
}

impl SoundMode {
    /// Every mode, in code order.
    pub const ALL: [Self; 3] = [Self::Once, Self::Loop, Self::Stop];

    /// The code a record carries in its word.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }

    /// The mode a code names.
    #[must_use]
    pub fn from_code(code: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.code() == code)
    }
}

/// `(arg, value, word)` of `play-mod-sound`.
pub(crate) fn encode(command: &Command) -> Option<(u16, u32, u32)> {
    match *command {
        Command::PlayModSound { index, mode, .. } => Some((index, 0, mode.code())),
        _ => None,
    }
}

/// The command of a `play-mod-sound` record: a mode the ABI names.
pub(crate) fn decode(slot: Slot, arg: u16, word: u32) -> Result<Command, Malformed> {
    Ok(Command::PlayModSound {
        slot,
        index: arg,
        mode: SoundMode::from_code(word).ok_or(Malformed::ModSound {
            kind: CommandKind::PlayModSound.code(),
            word,
        })?,
    })
}
