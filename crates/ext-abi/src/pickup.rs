//! A mod pickup's look: the icon it shows and the colour of the backdrop behind it.
//!
//! Both name their source outright: the title's own, or the mod's. Nothing is implied
//! by the pickup being a mod's.

use crate::{HudStr, Rgba, Weapon};

/// The name of one of the mod's own `[icons]`, at most `MAX_HUD_TEXT` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModIcon(HudStr);

impl ModIcon {
    /// The icon named `name`, if the name fits.
    #[must_use]
    pub fn new(name: &str) -> Option<Self> {
        HudStr::new(name).map(Self)
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) const fn from_string(name: HudStr) -> Self {
        Self(name)
    }
}

/// Which icon a pickup shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// The title's own icon sprite for a weapon. A title with no icon for it draws none.
    Title(Weapon),
    /// A PNG of the sending mod's `[icons]`.
    Mod(ModIcon),
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
