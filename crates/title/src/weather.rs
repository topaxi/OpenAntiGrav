//! Whether a title's weather is played, and so whether the weather setting is
//! offered.
//!
//! Per title (ADR-0058) rather than a `title.name` comparison. This crate
//! cannot name the setting type, so the game maps [`Weather::is_played`] onto
//! the menu row.

/// Where a title's rain and snow stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weather {
    /// A circuit's weather is played in a race (Pulse: read on the PSP, and
    /// carried to the PS2 pressing's circuits that author it).
    Played,
    /// The title's weather, if it has any, is not played, so there is nothing
    /// for the setting to change.
    Unplayed,
}

impl Weather {
    /// Whether the weather setting has anything to change on this title.
    #[must_use]
    pub const fn is_played(self) -> bool {
        matches!(self, Self::Played)
    }
}
