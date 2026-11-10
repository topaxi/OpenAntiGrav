//! Whether a race runs with weapons, and at which rung, asked of the options.

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

impl Options {
    /// The original's integer difficulty this race flies at, `0` Easy to `2` Hard:
    /// [`Self::skill_level`] where the launch named a rung, else the level
    /// [`Self::difficulty`] stands for. The index into a craft's three shield slots.
    #[must_use]
    pub fn skill_level(&self) -> u8 {
        self.skill_level
            .unwrap_or_else(|| u8::try_from(self.difficulty.level()).unwrap_or(u8::MAX))
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
