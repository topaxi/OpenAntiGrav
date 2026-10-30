//! The look of a simulation mod's own pickup: the icon it shows and the colour of the
//! backdrop behind it, each naming its source. Declared by a mod's `[pickup]` and
//! overridden per tick by its `pickup-look` command; read by the HUD.
//!
//! Data only: the title's fade, overshoot and size pulse are the HUD's, and play on any
//! look.

use crate::weapons::Weapon;

/// The icon of a mod's own pickup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Icon {
    /// The title's own icon for a weapon; none where the title has no icon for it.
    Title(Weapon),
    /// A mod's own PNG: its `[icons]` key in a manifest, and the HUD sheet's key once the
    /// race has resolved it.
    Mod(String),
}

/// The backdrop colour of a mod's own pickup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colour {
    /// The title's own backdrop row for a weapon. A title with no row draws the backdrop as
    /// it draws a weapon with none, and one with no backdrop colours (Pure, 2048) draws it
    /// as authored.
    Title(Weapon),
    /// A colour of the mod's own, `[r, g, b, a]`. Only a title that colours its backdrop
    /// (Pulse, HD, Omega) shows it.
    Rgba([u8; 4]),
}

/// Icon and colour together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickupLook {
    /// The icon.
    pub icon: Icon,
    /// The backdrop colour.
    pub colour: Colour,
}
