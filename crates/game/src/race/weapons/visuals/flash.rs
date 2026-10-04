//! Which screen flash a weapon's detonation starts.

/// The `ScreenFlash_Start` kind a weapon's detonation starts, or `None` for
/// one that starts none - split out of [`Race::ignite_blast`] so every row is
/// asserted without a disc, the way [`Race::blast_for`] is.
///
/// Each row is one caller of `ScreenFlash_Start` (`0x088f00c0`), read
/// 2026-09-30:
///
/// - **Rocket**: kind 0 from `Rocket_SpawnCraftExplosion_q` (`0x0886ed34`), a
///   craft hit only. `Rocket_Update`'s two track branches call no flash, so a
///   track hit washes nothing.
/// - **Missile**: kind 0 from `Missile_SpawnExplosion` (`0x08868d50`), every
///   ending.
/// - **Shuriken**: kind 0 from the teardown `FUN_08870c78`, every ending.
/// - **Plasma**: kind 1 from `PlasmaBlast_Construct` (`0x0885fd90`).
/// - **Bomb**: kind 3 from `BombBlast_Construct` (`0x08872078`).
/// - **Mine**: kind 8 from `Mine_SpawnExplosion` (`0x08867f1c`).
/// - **Cannon, Quake, the rest**: none from here. The Quake flashes from its
///   own per-tick update ([`Race::advance_quake_visual`]).
///
/// The Repulser's kind 2 comes from `Repulser_SpawnWaves` (`0x08876300`) when
/// its waves start, which `Race::advance_repulser_visual` raises.
pub(in crate::race) fn flash_for(
    kind: oag_tables::weapons::Weapon,
    struck: bool,
) -> Option<oag_render::flash::Kind> {
    use oag_render::flash;
    use oag_tables::weapons::Weapon;
    match kind {
        Weapon::Rocket if struck => Some(flash::BLAST),
        Weapon::Missile | Weapon::Shuriken => Some(flash::BLAST),
        Weapon::Plasma => Some(flash::PLASMA),
        Weapon::Bomb => Some(flash::BOMB),
        Weapon::Mine => Some(flash::MINE),
        _ => None,
    }
}
