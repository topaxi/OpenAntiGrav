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
