//! Whether a race runs with weapons, asked of either half of the options.

use super::{Options, Setup};

impl Options {
    /// Whether this race runs with weapons: the override when one is set,
    /// otherwise [`oag_race::Mode::weapons_enabled`]. Every reader that used to ask the
    /// mode asks this, so pads, damage and the load report agree.
    #[must_use]
    pub fn weapons_on(&self) -> bool {
        self.weapons_override
            .unwrap_or_else(|| self.mode.weapons_enabled())
    }
}

impl Setup {
    /// [`Options::weapons_on`], on the half of the options a running race keeps.
    #[must_use]
    pub fn weapons_on(&self) -> bool {
        self.weapons_override
            .unwrap_or_else(|| self.mode.weapons_enabled())
    }
}
