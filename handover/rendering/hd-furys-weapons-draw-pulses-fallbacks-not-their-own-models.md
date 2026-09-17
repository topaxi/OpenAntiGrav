# HD/Fury's weapons draw Pulse's fallbacks, not their own models and effects

2026-09-17. Found while drawing lanes for the weapon-fidelity pass. Every
weapon model entry in `oag_game::race` is a Pulse PSP name
(`Data\Weapons\Rocket.vex`, `Pulse_Mine.vex`, `pulse_muzzleflash.vex`,
`pulse_plasma_*.vex` - `crates/game/src/race/load/weapon_models.rs`), none of
which resolves on the PS3 disc, so on HD every projectile falls back to
`Race::projectile_sprites`' procedural billboard (`exhaust::sprite`) - a
stand-in - and only the `.pob` effects whose names HD shares
(`WO_ROCKET_FLARE`, `WO_MISSILE_HEAD`, `WO_PLASMA_HEAD`, ...) play.

What HD authors instead, all in DATA02 (`.vex` + `.rcsmodel` pairs, load
through `oag_render::mesh::rcs::build` the way a craft hull does; full entry
dump was taken with `scripts/psarc.py list` over all seven PSARCs):

| weapon | models | HD-only effects |
| --- | --- | --- |
| Plasma | `HD_plasma_ball` (the **bolt head**), `HD_plasma_ring`/`_sphere`/`_halo` (the explosion, ramps read: targets 100/7.1/7.0, rates 0.01/0.3/0.2, windows 1.7/1.3/1.3 s, 3.5 s life, [ps3-hdfury-eu/plasma.md](../../docs/ghidra/functions/ps3-hdfury-eu/plasma.md)) | `WO_PLASMA_CHARGING`, `WO_PLASMA_LAUNCH`, `WO_PLASMA_LIGHTNING_EXPAND` then `_COLLAPSE` at 1.3 s (trigger read on that page) |
| Cannon | `hd_muzzleflash`, `detonator_cannonbolt` (Fury) | `WO_CANNON_MUZZLEFLASH`, `WO_CANNON_HOTSPOT`, `WO_CANNON_SPARKS_DETONATOR` |
| LeachBeam | `hd_leachbeam_ball_bloomring` | `WO_LEACHBEAM_LAUNCH`/`_EMIT`/`_ABSORB`/`_BREAK`/`_HIT_TARGET`/`_HITSHELL`/`_BALL_SPARKS`/`_CHARGING_SPARKS`/`_ENERGY_SPRAY` |
| Rocket / Missile | `hd_Rocket`, `HD_missile_ball_bloomring`, `HD_missile_explosion` | `WO_MISSILE_LAUNCH` |
| Mine / Bomb | `HD_Mine`, `HD_Mine_halo`, `HD_Bomb`, `HD_bomb_*` (halo, sphere, sphere_white, sphere_bloomring, shockwaves), `bomb_shockwave` | `WO_BOMB_RAYS`, `WO_BOMB_SHOCKWAVE_FLASH`, `WO_BOMB_EXPLO_DETONATOR` |

The executable's own load-path strings name every model above
(`strings -a data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf | grep
'Data.Weapons'`), which is the evidence a model *is* loaded by the game; the
trigger for each effect is the RE, and the Plasma's is done.

## Open

- Weapon model entries need a per-title axis (`oag_title`/`oag_hd`, the
  shape `weapons` XML names already take), not a Pulse constant.
- Which HD blast draws which model set: the Plasma is read; the Bomb's five
  models and the Missile's explosion are not.

## Next Steps

1. Plasma first: per-title entries, `HD_plasma_ball` on the bolt,
   the ring/sphere/halo trio on `blast_models.rs`'s HD branch with the
   recovered ease, `WO_PLASMA_LIGHTNING_EXPAND`/`_COLLAPSE` on the recovered
   trigger. Screenshot at player size, several frames, `--give plasma` on the
   HD image.
2. Cannon (`hd_muzzleflash` + the two muzzle effects) and LeachBeam next -
   read the triggers on `/ps3-hdfury-eu/EBOOT.elf` before wiring any.
3. Rocket/Missile/Mine/Bomb models by the same per-title path; effects only
   as their triggers are read.
