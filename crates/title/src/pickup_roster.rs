//! Which pickups a title's executable lets its draw hand out, mode by mode.
//!
//! A weapon table's `<Pickupodds>` is only half of the roster. HD's Eliminator table weights the
//! Repulser and the Shuriken, and HD never hands out either: its executable gates every weapon
//! behind a per-mode bit mask before the draw can grant it
//! (`docs/ghidra/functions/ps3-hdfury-eu/pickup-draw.md`).

use oag_tables::weapons::Weapon;

/// The weapons a title's draw may grant, and how a person's roll treats the others.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickupRoster {
    /// Enabled in every weapon mode but Eliminator.
    pub race: &'static [Weapon],
    /// Enabled in Eliminator.
    pub eliminator: &'static [Weapon],
    /// The order a **person's** roll walks. A weapon the mode disables keeps its weight in the
    /// roll, and a roll landing on it passes to the next enabled weapon in this order. An AI
    /// draw leaves disabled weapons out of the roll instead.
    pub player_order: &'static [Weapon],
    /// Whether an AI driver's draw rolls again on a grant equal to its previous one. A person's
    /// always does.
    pub ai_refuses_repeat: bool,
    /// Weapons both draws leave out, from the total and from a person's walk alike, while the
    /// field already holds or runs enough of them.
    pub exclusions: &'static [Exclusion],
}

/// A weapon no draw hands out while [`Self::limit`] or more are held or running in the race.
///
/// Running is the weapon's own live state after the shot: a LeachBeam's beam, a Quake's wave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exclusion {
    /// The weapon left out.
    pub weapon: Weapon,
    /// How many held or running take it out of both draws.
    pub limit: u8,
}

impl PickupRoster {
    /// The weapons enabled in a mode: [`Self::eliminator`] or [`Self::race`].
    #[must_use]
    pub const fn enabled(&self, eliminator: bool) -> &'static [Weapon] {
        if eliminator {
            self.eliminator
        } else {
            self.race
        }
    }
}
