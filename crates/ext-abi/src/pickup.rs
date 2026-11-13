//! A mod pickup's look: the icon it shows and the colour of the backdrop behind it.
//!
//! Both name their source outright: the title's own, or the mod's. Nothing is implied
//! by the pickup being a mod's.

use crate::{ModName, PICKUP_OTHER_FIRST, PICKUP_OTHER_LAST, PICKUP_OURS, Rgba, Weapon};

/// What a craft holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Pickup {
    /// Nothing.
    #[default]
    None,
    /// One of the title's weapons.
    Weapon(Weapon),
    /// The reading mod's own pickup (`give-pickup`).
    Ours,
    /// Another mod's pickup, by that mod's kind in the race (below [`crate::MAX_PICKUP_KINDS`], in
    /// module id order among the mods that declare a `[pickup]`). A module sees its own
    /// kind as [`Self::Ours`], never as this.
    Other(u8),
}

impl Pickup {
    /// Its code in a craft record.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Weapon(weapon) => weapon.code(),
            Self::Ours => PICKUP_OURS,
            Self::Other(kind) => PICKUP_OTHER_FIRST + kind,
        }
    }

    /// The pickup a code names, if any.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::None),
            PICKUP_OURS => Some(Self::Ours),
            PICKUP_OTHER_FIRST..=PICKUP_OTHER_LAST => Some(Self::Other(code - PICKUP_OTHER_FIRST)),
            _ => Weapon::from_code(code).map(Self::Weapon),
        }
    }
}

/// Which icon a pickup shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// The title's own icon sprite for a weapon. A title with no icon for it draws none.
    Title(Weapon),
    /// A PNG of the sending mod's `[icons]`.
    Mod(ModName),
}

/// Which colour a pickup's backdrop has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colour {
    /// The title's own backdrop row for a weapon. A title with no row for it, or with no
    /// backdrop widget (Pure), draws none.
    Title(Weapon),
    /// A colour of the mod's own.
    Rgba(Rgba),
}
