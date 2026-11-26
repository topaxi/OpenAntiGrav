//! HD's and Fury's pickup roster, read from the executable
//! (`docs/ghidra/functions/ps3-hdfury-eu/pickup-draw.md`).

use oag_tables::weapons::Weapon;
use oag_title::pickup_roster::PickupRoster;

/// Mask `0x7FF`, the default of all three menu masks, and `0x7A7`, what choosing Eliminator
/// resets one to. Neither holds bit 11 (Repulser) or 12 (Shuriken), and the weapons screen has
/// no row for either.
pub const ROSTER: PickupRoster = PickupRoster {
    race: &[
        Weapon::Rocket,
        Weapon::Missile,
        Weapon::Quake,
        Weapon::Turbo,
        Weapon::Shield,
        Weapon::Cannon,
        Weapon::Autopilot,
        Weapon::Plasma,
        Weapon::Bomb,
        Weapon::Mine,
        Weapon::LeachBeam,
    ],
    eliminator: &[
        Weapon::Rocket,
        Weapon::Missile,
        Weapon::Quake,
        Weapon::Cannon,
        Weapon::Plasma,
        Weapon::Bomb,
        Weapon::Mine,
        Weapon::LeachBeam,
    ],
    // The order `WeaponManager_RollPickup` tests them in.
    player_order: &[
        Weapon::Autopilot,
        Weapon::Bomb,
        Weapon::Cannon,
        Weapon::Mine,
        Weapon::LeachBeam,
        Weapon::Missile,
        Weapon::Plasma,
        Weapon::Quake,
        Weapon::Repulser,
        Weapon::Shuriken,
        Weapon::Rocket,
        Weapon::Shield,
        Weapon::Turbo,
    ],
};
