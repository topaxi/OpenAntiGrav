//! Whether a title's own shadow mechanism is built, and so what its default is.
//!
//! The `original` shadow tier draws what the title itself authors
//! (`oag_display::display::Shadows::Original`). Which titles have anything to
//! draw is a property of the title and of how far this project has got with
//! it, so it is `Title` data rather than a `title.name` comparison
//! (ADR-0058). This crate cannot name the setting type (`oag-display` is not a
//! dependency), so the game maps [`OriginalShadows::draws`] onto it.

/// Where a title's `original` shadow tier stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalShadows {
    /// Built, and it draws in a race: Pulse's authored occluder hulls, HD's
    /// coverage map. The shadow setting defaults to `original`.
    Built,
    /// The title authors no shadow of any kind, so `original` would draw
    /// nothing (Wipeout Pure). The setting defaults to `off`, which says the
    /// same thing without running a pass.
    Absent,
    /// The title has a mechanism this project has not built (2048's
    /// proximity and environment shadows, Omega's untried carry-over from
    /// HD). `original` draws nothing there yet, so the default is `off`.
    Unbuilt,
}

impl OriginalShadows {
    /// Whether `original` draws anything on this title, which is the only
    /// question the shadow default asks.
    #[must_use]
    pub const fn draws(self) -> bool {
        matches!(self, Self::Built)
    }
}
