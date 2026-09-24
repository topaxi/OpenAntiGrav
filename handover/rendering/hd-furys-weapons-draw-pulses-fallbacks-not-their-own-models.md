# HD/Fury's weapons draw Pulse's fallbacks, not their own models and effects

2026-09-17. Found while drawing lanes for the weapon-fidelity pass. Every
weapon model entry in `oag_game::race` is a Pulse PSP name
(`Data\Weapons\Rocket.vex`, `Pulse_Mine.vex`, `pulse_muzzleflash.vex`,
`pulse_plasma_*.vex` - `crates/game/src/race/load/weapon_models.rs`), none of
which resolves on the PS3 disc, so on HD every projectile falls back to
`Race::projectile_sprites`' procedural billboard (`exhaust::sprite`) - a
stand-in - and only the `.pob` effects whose names HD shares
(`WO_ROCKET_FLARE`, `WO_MISSILE_HEAD`, `WO_PLASMA_HEAD`, ...) play.

**2026-09-17, same day: the per-title axis landed, and with it the Rocket,
Mine, Bomb and Cannon round all draw their own HD model too, not only
Plasma.** `oag_title::weapons::WeaponModels` is a new field on `Title`
(`crates/title/src/weapons.rs`), filled by all four title crates;
`load/weapon_models.rs::load` takes the PS3 external-geometry branch
`crate::livery::shield_model` already had (`mesh::geometry_is_external` +
`mesh::rcs::build`), so every entry below except the two still-open rows
now resolves off its `.vex`/`.rcsmodel` pair on a real HD race rather than
falling back to a billboard. Verified from the loader's own report line at
`--give <weapon>`: `hd_Rocket.vex`, `HD_Mine.vex`, `HD_Bomb.vex` and
`hd_muzzleflash.vex` (the Cannon round's own body - not the two hand-drawn
quads, which `cannon-quads` owns separately) all report a real triangle
count and material, not "falls back to a billboard".

What HD authors, all in DATA02 (`.vex` + `.rcsmodel` pairs, load
through `oag_render::mesh::rcs::build` the way a craft hull does; full entry
dump was taken with `scripts/psarc.py list` over all seven PSARCs):

| weapon | models | HD-only effects |
| --- | --- | --- |
| ~~Plasma~~ **bolt done; blast drawn** | `HD_plasma_ball` (the **bolt head**, loaded, not drawn - `HD_PLASMA_BALL_DRAWN`), `HD_plasma_ring`/`_sphere`/`_halo` (the explosion, ramps read: targets 100/7.1/7.0, rates 0.01/0.3/0.2, windows 1.7/1.3/1.3 s, 3.5 s life, [ps3-hdfury-eu/plasma.md](../../docs/ghidra/functions/ps3-hdfury-eu/plasma.md) - **drawn since 2026-09-23, culled as authored and on HD's own right-handed basis; the scale was right all along**) | `WO_PLASMA_CHARGING`, `WO_PLASMA_LAUNCH` still unwired; `WO_PLASMA_LIGHTNING_EXPAND`/`_COLLAPSE` wired and confirmed against the disc's own `.pob` internal name field, not just the fourcc |
| ~~Rocket / ~~Missile | ~~`hd_Rocket`~~ **model wired**, `HD_missile_ball_bloomring`, `HD_missile_explosion` still open | `WO_MISSILE_LAUNCH` |
| ~~Mine / ~~Bomb | ~~`HD_Mine`~~, ~~`HD_Bomb`~~ **models wired**; `HD_Mine_halo`, `HD_bomb_*` (halo, sphere, sphere_white, sphere_bloomring, shockwaves), `bomb_shockwave` still open | `WO_BOMB_RAYS`, `WO_BOMB_SHOCKWAVE_FLASH`, `WO_BOMB_EXPLO_DETONATOR` |
| Cannon | `hd_muzzleflash` **round body wired**, own to `cannon-quads`; `detonator_cannonbolt` (Fury) still open | `WO_CANNON_MUZZLEFLASH`, `WO_CANNON_HOTSPOT`, `WO_CANNON_SPARKS_DETONATOR` - `cannon-quads`'s lane |
| LeachBeam | `hd_leachbeam_ball_bloomring` **entry named** (`oag_title::weapons::WeaponModels::leachbeam_ball`), not drawn - see Next Steps | `WO_LEACHBEAM_LAUNCH`/`_EMIT`/`_ABSORB`/`_BREAK`/`_HIT_TARGET`/`_HITSHELL`/`_BALL_SPARKS`/`_CHARGING_SPARKS`/`_ENERGY_SPRAY` |

The executable's own load-path strings name every model above
(`strings -a data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf | grep
'Data.Weapons'`), which is the evidence a model *is* loaded by the game.

## Open

- **The Plasma explosion draws (2026-09-23); three things of it still
  do not.** The scale is not the error: the ring/sphere/halo are discs and
  a ball of radius 8.8/4.1/9.0 m and `cur` is a plain uniform scale (select
  mask `0x00769cd0`). Each shell is authored twice (one each way) under
  material state bit 4 = `NV4097_SET_CULL_FACE_ENABLE`, which this engine now
  honours for the trio, on HD's own right-handed basis (halo checked by
  pixel diff). The 09-17 solid-grey frame was **not reproduced** on this tree
  either way; under continuous fire several overlapping blasts still fill the
  frame purple, culled or not. See plasma.md's 2026-09-23 section. Not played yet: (1) the `UV_offset`
  binding `WeaponExplosions_Construct` makes to `node + 0xc0` of each model's
  first `PTR_PTR_008b3988`-class node, driven by `age` through
  `_opd_FUN_002c1b30` - what that field holds is unread; (2) the sphere's
  `noise.gtf` second sampler; (3) the sphere's track-fitted basis
  (`FUN_000a97f0`), for which the camera-facing one stands in. The live
  RPCS3 check that would falsify the scale reading (Z0 on `Draw`'s three
  `bl 0x327500` sites) was **not run**: getting a Plasma detonation in front
  of the player on the emulator needs the held-weapon slot, which is unread.
- ~~**Every other weapon model is placed with a reflection.**~~ **Fixed
  2026-09-24.** `Race::projectile_model_matrices` and
  `blast_models::billboard_matrix` now build `side = reference x forward`,
  a rotation - the original's own `Rocket_Update` basis, measured live on
  PPSSPP (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`,
  2026-09-24 section). Correction to this bullet as first written: the laid
  Mine and Bomb were never mirrored (they take the quaternion branch); only
  the Rocket, the Cannon round, the Plasma head and Pulse's blast shells
  were. Every HD body now asks `cull_as_authored`: the Rocket and the Bomb
  cull (the log says "back faces culled"), the Mine and the Cannon round
  stay two-sided (mixed materials). Visible on HD: the red wash that filled
  the frame's right half when the chase camera sat inside the Bomb's
  see-through shell is gone. **HD's own placements are still unread** - the
  Rocket's is Pulse's measured basis on HD's model, the Mine's and Bomb's
  the frozen craft pose; all three chosen, not measured, until
  `/hdfury/EBOOT-ps3-hdfury-eu.elf`'s own per-tick updates are read.
- **LeachBeam's ball: placement read 2026-09-23, not wired.**
  `LeachBall_Advance` (`0x00114c78`) carries `hd_leachbeam_ball_bloomring`
  along the beam, one trip per drain, spawning `WO_LEACHBEAM_ABSORB` at each
  arrival. `LAUNCH`/`HIT_TARGET`/`BREAK` each have a named spawner. None of
  the three spawners has a caller found, so their triggers are open. See
  `docs/ghidra/functions/ps3-hdfury-eu/weapons.md`, "2026-09-23: the
  LeachBeam's own HD pieces". Next: find the spawners' callers through the
  vtable at `0x00864af8` (entries are OPD addresses), then read
  `_opd_FUN_00116308` (a point along the beam at a fraction) and the two strip
  objects. After that, wire the ball on `oag_render::mesh::rcs` the way the
  Plasma ball is.
- **Cannon's own round body model is wired** (`hd_muzzleflash.vex`, shared
  spelling with Pulse's own naming quirk); the two hand-drawn quad textures
  and the three Cannon effects are `cannon-quads`'s own lane, not touched
  here.
- **Missile (2026-09-23, read, not wired):** `HD_missile_ball_bloomring` is
  the flying missile's head (`Missile.cpp` constructor `0x0011ccc8`), not an
  explosion model; `HD_missile_explosion` loads in `0x00154cf0` (from
  `MissileManager_Construct`), binds `UV_offset` and `Shockwave_scalar`, and
  grows by its own keyed `Anim Transform` (1 -> 18x). What starts it is
  unread - see weapons.md's 2026-09-23 section for every address.
- The Bomb's five-model detonation (`HD_Mine_halo`, `HD_bomb_*`) is named
  on the executable's own strings but neither its load order nor its
  per-tick placement was read.
- `0x00121418` (`Plasma_PostUpdate`'s visual placement) suggests a
  velocity-plus-carried-normal basis for the bolt, not velocity alone, but
  is not resolved past confidence ~55 - stays unrenamed per `CLAUDE.md`'s
  below-70 rule. The engine draws the bolt velocity-only for now, same as
  the Rocket, documented as chosen rather than measured.

## Next Steps

1. ~~Plasma first: per-title entries, `HD_plasma_ball` on the bolt, the
   ring/sphere/halo trio on `blast_models.rs`'s HD branch with the recovered
   ease, `WO_PLASMA_LIGHTNING_EXPAND`/`_COLLAPSE` on the recovered
   trigger.~~ **Landed 2026-09-17** - see plasma.md's own dated section for
   what the implementation pass itself found (the Collapse/Draw split, the
   oversized picture).
2. Cannon (`hd_muzzleflash` + the two muzzle effects) and LeachBeam next -
   read the triggers on `/ps3-hdfury-eu/EBOOT.elf` before wiring any. The
   round's own body model is wired; the quads and the three particle
   effects are `cannon-quads`'s lane. LeachBeam's ball placement is
   unread - start there.
3. Rocket/Missile/Mine/Bomb **bodies** are wired; their own further
   detonation models (Missile's pair, the Bomb's five) and effects remain,
   by the same per-title path, only as their triggers are read.
4. ~~Settle the Plasma explosion's scale~~ **Done 2026-09-23**, the gate
   is gone. Next on it: read what `node + 0xc0` is on the
   `PTR_PTR_008b3988` node class (the `UV_offset` source), then the sphere's
   `noise.gtf` role. Compare against a real RPCS3 capture of one detonation
   when the held-weapon slot is known - no capture of the original's blast
   exists yet.
5. ~~Make `Race::projectile_model_matrices` a rotation and honour state bit
   4 for every HD weapon model~~ **Done 2026-09-24.** Next on it: read HD's
   own Rocket/Mine/Bomb per-tick placement (the `Rocket` vtable off
   `Rocket_Construct` `0x001254c8`) so the HD poses stop borrowing Pulse's.
