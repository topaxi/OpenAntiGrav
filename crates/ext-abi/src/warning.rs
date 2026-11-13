//! What `raise-warning` names: one of the title's ten warning icons, or an icon of the
//! sending mod's own `[icons]` on a slot the mod picks.

use crate::{ModName, WarningIcon};

coded! {
    /// The incoming-warning group a mod's own icon lights. A title's icon fixes its group
    /// ([`WarningIcon::side`]); a mod's names it.
    WarningSide: u16, first 0 {
        /// The rear slot: a threat behind.
        Rear,
        /// The forward slot: a threat laid ahead.
        Forward,
    }
}

impl WarningIcon {
    /// The group this icon warns on: the eight shots on the rear slot, a mine or bomb on the
    /// forward one.
    #[must_use]
    pub const fn side(self) -> WarningSide {
        match self {
            Self::Mine | Self::Bomb => WarningSide::Forward,
            _ => WarningSide::Rear,
        }
    }
}

/// The icon a `raise-warning` shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warning {
    /// The title's own icon for a weapon; it fixes the group.
    Title(WarningIcon),
    /// A PNG or SVG of the sending mod's `[icons]`, in the group `side` names.
    Mod {
        /// The group it lights.
        side: WarningSide,
        /// The icon's name in the mod's `[icons]`.
        icon: ModName,
    },
}

impl Warning {
    /// The group the warning lights.
    #[must_use]
    pub const fn side(self) -> WarningSide {
        match self {
            Self::Title(icon) => icon.side(),
            Self::Mod { side, .. } => side,
        }
    }
}
