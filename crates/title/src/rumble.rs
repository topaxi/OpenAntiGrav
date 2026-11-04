//! Whether a title's original plays pad patterns, and how its small motor
//! reads them. Decides whether the RUMBLE setting offers `original`, and what
//! that value plays.
//!
//! Per title (ADR-0058). HD/Fury and Omega ship and play the 64 patterns
//! (`docs/gameplay/rumble.md`, confidence 84, read from the executables, no
//! runtime trace). The PSP titles have no motors, the PS2 pressing plays a
//! level and a timer instead of patterns, and 2048 ships the files and reads
//! none.

/// How a pattern's small-motor track becomes the pad's small motor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmallMotor {
    /// Any non-zero among the mixed voices is full on (HD/Fury, PS3).
    OnOff,
    /// The byte sum of the mixed voices, so 1..5 of 255 (Omega, PS4).
    Sum,
}

/// What a title's original does with the pad's motors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rumble {
    /// The original plays no pattern rumble, so the setting has no `original`.
    Absent,
    /// The original plays the patterns with this small-motor law.
    Patterns(SmallMotor),
}

impl Rumble {
    /// Whether the setting offers `original` on this title.
    #[must_use]
    pub const fn is_played(self) -> bool {
        matches!(self, Self::Patterns(_))
    }

    /// The small-motor law `original` plays, if there is one.
    #[must_use]
    pub const fn small_motor(self) -> Option<SmallMotor> {
        match self {
            Self::Patterns(law) => Some(law),
            Self::Absent => None,
        }
    }
}
