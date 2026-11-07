//! [`Event`]: a change to the race that is not an input and not a tick.
//!
//! ADR-0055's rule is that anything which changes the race outside `Race::tick` must become an
//! input or be recorded. Pilot Assist is switched on the pause screen and a capture hands a
//! craft a weapon between two ticks; neither is in an [`oag_gameplay::InputSnapshot`], so each is
//! an entry in the `EVNT` chunk (`file.rs`), applied by whoever re-drives the file between the
//! same two ticks it was made between (ADR-0087, decision 7).
//!
//! This crate stores the entry and knows nothing of what it means: `value` is a number whose
//! meaning is the kind's, decoded by the crate that applies it.

/// One recorded change, applied **before** tick [`Self::tick`] runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// The world tick the change was made before: the first tick that sees it.
    pub tick: u64,
    /// The grid slot it applies to.
    pub seat: u8,
    /// What changed, a [`Kind`]'s code. Kept as the byte so an event of a kind a later build
    /// writes survives a read and a write by this one.
    pub kind: u8,
    /// The new value, in the kind's own terms.
    pub value: u32,
}

/// The kinds of change this build writes and applies.
///
/// Code `3` is reserved for `controller` (ADR-0087, decision 9: a seat handed to the opponent
/// driver at a fork) and is not implemented: a file carrying it reads, and the build that
/// applies events refuses it by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The seat's Pilot Assist level changed. The value is the level's ordinal.
    PilotAssist,
    /// The seat's pickup slot was topped up with a weapon (the capture flag `--give`). The value
    /// is the weapon's type number.
    Give,
}

impl Kind {
    /// The byte this kind is stored as.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::PilotAssist => 1,
            Self::Give => 2,
        }
    }

    /// The kind stored as `code`, or `None` for one this build does not know.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::PilotAssist),
            2 => Some(Self::Give),
            _ => None,
        }
    }
}

impl Event {
    /// An event of a known `kind`.
    #[must_use]
    pub const fn new(tick: u64, seat: u8, kind: Kind, value: u32) -> Self {
        Self {
            tick,
            seat,
            kind: kind.code(),
            value,
        }
    }

    /// The kind, or `None` when this build does not know the code.
    #[must_use]
    pub const fn kind(&self) -> Option<Kind> {
        Kind::from_code(self.kind)
    }
}
