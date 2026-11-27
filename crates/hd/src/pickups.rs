//! HD's and Fury's pickup roster, read from the executable
//! (`docs/ghidra/functions/ps3-hdfury-eu/pickup-draw.md`).

use oag_tables::weapons::Weapon;
use oag_title::pickup_roster::{Exclusion, PickupRoster};

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
    // `WeaponManager_RollPickupAi` writes the slot without comparing the previous grant.
    ai_refuses_repeat: false,
    // "One-live exclusions": the words at `0x009943a0` and `0x009943a4`, and the count at
    // `0x00993d80`, each counting from the grant.
    exclusions: &[
        Exclusion {
            weapon: Weapon::LeachBeam,
            limit: 1,
        },
        Exclusion {
            weapon: Weapon::Quake,
            limit: 1,
        },
        Exclusion {
            weapon: Weapon::Cannon,
            limit: 2,
        },
    ],
};
