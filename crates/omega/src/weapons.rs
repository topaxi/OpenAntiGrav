//! Where Wipeout: Omega Collection keeps each weapon's own body model, and what
//! its ribbon effects draw with.
//!
//! **Inherited from Wipeout HD, played from Omega's own archives.** `data00`
//! ships HD's `Data/weapons/HD_*.vex` set - each name with its PS4 `.rcsmodel`
//! beside it - and the executable's own ribbon-model table
//! (`rockettrail_triangle`, `waketrail_triangle`, `leachbeam_triangle`,
//! `RocketTrail_Shadow_triangle`, `0x018332bc`..`0x018332f7`) is HD's, in HD's
//! order. What is *not* Omega's own measurement is every law that draws them:
//! the placement, the cull rule and the cannon's look are HD's, read off HD's
//! executable, and no PS4 capture has confirmed any of them.
//!
//! **Which root a race reads is decided by its mode, not its circuit.**
//! `Rocket_Construct` (`0x01368610`) formats `Data/Weapons/%s` when
//! `DAT_01f999e4 < 0x17` and `Data/Weapons2048/%s` otherwise
//! (`docs/ghidra/functions/ps4-omega-eu/weapons.md`). Every mode this project
//! launches on Omega is an HD-lineage one, below `0x17`
//! (`crate::race::ORIGINAL_DRIVER`), so the `Data\Weapons` root is right for
//! every race there is, a 2048-era circuit included. `Data/Weapons2048` ships
//! the same names with re-cooked bytes (`hd_bomb_residue.rcsmodel` is 55,957
//! bytes in `Data/weapons` and 97,625 in `Data/weapons2048`) and would be the
//! root of a 2048-lineage mode, which nothing here launches; a title that
//! launched one would need a per-mode root, not a second table.
//!
//! **Held back on purpose** (`None` below, named so the next reader does not
//! re-derive them): the plasma bolt's head, the LeachBeam's ball and the bomb's
//! blast set load and build, and then draw the wrong picture. Their HD
//! materials are shaded by fragment programs `oag_mesh::mesh::rcs::rim_glow`
//! recognises by fingerprint on the PS3's RSX code (`RIM_EDGE`, `RIM_GLOW`,
//! `BOMB_FIRE`); the PS4 builder (`oag_mesh::mesh::rcs::psp2`) sets none of
//! those bits. The head draws as a barely visible ghost, the ball as a grey
//! bubble, and the bomb's bloom ring paints its fire-ramp lookup texture over
//! the road as a red wash. See `docs/gameplay/weapon-lineage.md`.

use oag_title::weapons::{CannonBody, CannonLook, HdPlasmaBlast, MagstripWake, WeaponModels};

/// Omega's weapon bodies and ribbon textures.
pub const WEAPON_MODELS: &oag_title::weapons::WeaponModels = &oag_title::weapons::WeaponModels {
    rocket: Some(r"Data\Weapons\hd_Rocket.vex"),
    mine: Some(r"Data\Weapons\HD_Mine.vex"),
    bomb: Some(r"Data\Weapons\HD_Bomb.vex"),
    // A flash at the craft's `cannon_flash` locator, not a body on the round:
    // [`CANNON_LOOK`] says which.
    cannon: Some(r"Data\Weapons\hd_muzzleflash.vex"),
    // Held back: see the module doc.
    plasma_ball: None,
    plasma_blast_hd: Some(HdPlasmaBlast {
        ring: r"Data\Weapons\HD_plasma_ring.vex",
        sphere: r"Data\Weapons\HD_plasma_sphere.vex",
        halo: r"Data\Weapons\HD_plasma_halo.vex",
    }),
    // Held back: see the module doc.
    bomb_blast_hd: None,
    missile_blast_hd: Some(r"Data\Weapons\HD_missile_explosion.vex"),
    // **The executable's own literals** (`0x01837096`, `0x018370b7`): `data/Tex/
    // HD_electric_arc_8x8.gnf` and `data/Tex/HD_ElectricArc_Contact.gnf`, the
    // pair HD's `MagstripWake_Construct` names as `.gtf`. Built only under an
    // HD-lineage mode (`ships-effects.md`, "One effect or two").
    magstrip_wake: Some(MagstripWake {
        atlas: r"Data\Tex\HD_electric_arc_8x8.gnf",
        contact: r"Data\Tex\HD_ElectricArc_Contact.gnf",
    }),
    // Held back: Omega's own `leachbeam_triangle.rcsmodel` names `data/ribboneffects/
    // textures/hd_leechbeam.gnf` (128x32) and `hd_waketrail_clouds.gnf` (128x128),
    // where HD's names `hd_leechbeam_glow` (256x256), a file no Omega archive
    // holds. HD's strip law over Omega's two draws a line too faint to see at
    // player size (`weapon-lineage.md`), so it stays unwired until the PS4
    // `HD_Leachbeam` material's own samplers are read.
    leach_strip: None,
    // Held back: see the module doc.
    leachbeam_ball: None,
    cannon_look: Some(CANNON_LOOK),
    // Ships it as `.gnf`; the eboot does not name it, so Pulse's law.
    ghost_static: Some(oag_title::weapons::GhostStatic {
        entry: r"Data\Tex\staticglow.mip",
        origin: oag_title::Origin::InheritedFrom("Wipeout Pulse"),
    }),
    // `cockpit_hit` stays `None`: HD, whose front end Omega carries forward, draws
    // none, and Omega's own executable cannot be read to say otherwise.
    ..WeaponModels::EMPTY
};

/// How a Cannon round draws on Omega: **HD's, inherited**, with Omega's `.gnf`
/// spellings of the two textures.
///
/// `CannonBullet.cpp` is in Omega's executable (`CannonManager_Construct`,
/// `weapons.md`) and its update and draw are unread, so the numbers are
/// `oag_hd::race::CANNON_LOOK`'s - half-width `0.25`, the whole segment, a
/// flash rolled from `(0.25, 1.0)` and `hd_muzzleflash` re-placed at the craft's
/// `cannon_flash` locator for the round's first `0.1` s, stretched along Z from
/// `(0.7, 1.3)`. The textures exist at `Data/weapons/Textures/Cannon_bolt.gnf`
/// and `cannon_muzzle_flash.gnf`. Duplicated rather than borrowed: a title
/// package depends on no other.
pub const CANNON_LOOK: CannonLook = CannonLook {
    bolt_texture: r"Data\Weapons\Textures\Cannon_bolt.gnf",
    flash_texture: r"Data\Weapons\Textures\Cannon_muzzle_flash.gnf",
    bolt_half_width: 0.25,
    bolt_near_fraction: 0.0,
    flash_size_range: (0.25, 1.0),
    body: CannonBody::Muzzle {
        stretch_range: (0.7, 1.3),
    },
};
