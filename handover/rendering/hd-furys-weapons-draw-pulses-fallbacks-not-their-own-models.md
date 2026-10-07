# HD/Fury's weapons draw Pulse's fallbacks, not their own models and effects

2026-09-17. Found while drawing lanes for the weapon-fidelity pass. Every
weapon model entry in `oag_raceplay` is a Pulse PSP name
(`Data\Weapons\Rocket.vex`, `Pulse_Mine.vex`, `pulse_muzzleflash.vex`,
`pulse_plasma_*.vex` - `crates/raceplay/src/load/weapon_models.rs`), none of
which resolves on the PS3 disc, so on HD every projectile falls back to
`Race::projectile_sprites`' procedural billboard (`exhaust::sprite`) - a
stand-in - and only the `.pob` effects whose names HD shares
(`WO_ROCKET_FLARE`, `WO_MISSILE_HEAD`, `WO_PLASMA_HEAD`, ...) play.

**2026-09-17, same day: the per-title axis landed, and with it the Rocket,
Mine, Bomb and Cannon round all draw their own HD model too, not only
Plasma.** `oag_title::weapons::WeaponModels` is a new field on `Title`
(`crates/title/src/weapons.rs`), filled by all four title crates;
`load/weapon_models.rs::load` takes the PS3 external-geometry branch
`oag_livery::shield::shield_model` already had (`mesh::geometry_is_external` +
`mesh::rcs::build`), so every entry below except the two still-open rows
now resolves off its `.vex`/`.rcsmodel` pair on a real HD race rather than
falling back to a billboard. Verified from the loader's own report line at
`--give <weapon>`: `hd_Rocket.vex`, `HD_Mine.vex`, `HD_Bomb.vex` and
`hd_muzzleflash.vex` (the Cannon round's own body - not the two hand-drawn
quads, which `cannon-quads` owns separately) all report a real triangle
count and material, not "falls back to a billboard".

What HD authors, all in DATA02 (`.vex` + `.rcsmodel` pairs, load
through `oag_mesh::mesh::rcs::build` the way a craft hull does; full entry
dump was taken with `scripts/psarc.py list` over all seven PSARCs):

| weapon | models | HD-only effects |
| --- | --- | --- |
| ~~Plasma~~ **bolt done; blast drawn** | `HD_plasma_ball` (the **bolt head**, loaded, its program routed to `RIM_EDGE` 2026-09-25, still not drawn - `HD_PLASMA_BALL_DRAWN`, held off by the cull thread below), `HD_plasma_ring`/`_sphere`/`_halo` (the explosion, ramps read: targets 100/7.1/7.0, rates 0.01/0.3/0.2, windows 1.7/1.3/1.3 s, 3.5 s life, [ps3-hdfury-eu/plasma.md](../../docs/ghidra/functions/ps3-hdfury-eu/plasma.md) - **drawn since 2026-09-23, culled as authored and on HD's own right-handed basis; the scale was right all along**) | `WO_PLASMA_CHARGING`, `WO_PLASMA_LAUNCH` still unwired; `WO_PLASMA_LIGHTNING_EXPAND`/`_COLLAPSE` wired and confirmed against the disc's own `.pob` internal name field, not just the fourcc |
| ~~Rocket / ~~Missile | ~~`hd_Rocket`~~ **model wired**, `HD_missile_ball_bloomring`, `HD_missile_explosion` still open | `WO_MISSILE_LAUNCH` |
| ~~Mine / ~~Bomb | ~~`HD_Mine`~~, ~~`HD_Bomb`~~ **models wired**; **the Bomb's detonation drawn 2026-10-07** (`HD_bomb_sphere`, `_white`, `hd_bomb_sphere_bloomring`, `hd_bomb_shockwaves` x8 - four models, not five; `HD_bomb_halo` is the armed bomb's, `HD_Mine_halo` the Mine's); `HD_Mine_halo` still open | `WO_BOMB_RAYS` **wired 2026-10-07** (0.5 s in); `WO_BOMB_SHOCKWAVE_FLASH`, `WO_BOMB_EXPLO_DETONATOR` are the Detonator mode's |
| ~~Cannon~~ **done 2026-09-25** | `hd_muzzleflash` **is a muzzle flash, not a round body**: drawn at the craft's `cannon_flash` locator for the round's first 0.1 s; HD's own `.gtf` bolt/flash quads drawn; `detonator_cannonbolt` (Fury) still open | `WO_CANNON_MUZZLEFLASH`, `WO_CANNON_HOTSPOT`: **no trigger exists** (named by nothing but their own files); `WO_CANNON_SPARKS_DETONATOR` Detonator-only |
| LeachBeam | `hd_leachbeam_ball_bloomring` **drawn 2026-09-25** through its own program (`RIM_GLOW`, [hd-unlit-programs.md](../../docs/rendering/hd-unlit-programs.md)) | `WO_LEACHBEAM_ABSORB` **wired 2026-09-25** (fires each drain trip); `_LAUNCH`/`_BREAK`/`_HIT_TARGET` confirmed to have no discoverable trigger in the retail EBOOT (same shape as the Cannon's two); `_EMIT`/`_HITSHELL`/`_BALL_SPARKS`/`_CHARGING_SPARKS`/`_ENERGY_SPRAY` still unread |

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
  `_opd_FUN_002c1b30` - **2026-09-25: the bind mechanism is read (by
  pointer, not by value - `Material_BindInstanceParamPointer`), and
  confirmed as the same helper pair Missile's own `UV_offset`/
  `Shockwave_scalar` bindings use on `MissileManager_Construct`, both
  aliasing the same `node + 0xc0`. What law drives that field's value is
  still unread - it needs the Anim Transform track's own keys parsed out of
  each `.vex`, not another code read; see plasma.md's 2026-09-25 section**;
  (2) ~~the sphere's `noise.gtf` second sampler~~ **read 2026-09-25**: unit 2,
  sampled at `(u + 10 UV_offset, 15 v + 10 UV_offset)`, modulating a
  twenty-fold sun specular added to the colour - see plasma.md's newest
  section. Left unwired: the program also needs the sun, the ambient,
  `0x7480de6d` (unauthored, source unread) and a paraboloid reflection probe,
  so even a played `UV_offset` would not complete it; (3) the
  sphere's track-fitted basis (`FUN_000a97f0`) - **2026-09-25: confirmed at
  the control-flow level that `WeaponExplosions_Start` branches on this
  call's found/not-found return, and only the found branch does the extra
  Gram-Schmidt pass that makes the shared basis track-fitted rather than
  the seeded identity; what feeds that construction (`r1+0x140`) was not
  traced to its source, confidence 60, below this project's rename
  threshold - see plasma.md's 2026-09-25 section**. The camera-facing basis
  still stands in for the engine's own draw. The live
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
- **LeachBeam's ball: period law recovered and wired 2026-09-25; the
  placement and draw path built the same day, second pass, and the model
  draws since the third.** `LeachBall_Advance`
  (`0x00114c78`) remaps the strip's own length (`param_4[4]`, plausibly but
  not confirmed to be the beam's own length), clamped `[20, 100]`, onto a
  `[0.3, 1.0]`-second period per drain trip; `oag_fx::beam::hd_ball`
  carries that law and `Race::advance_leach_beam_ribbon` drives a
  render-side accumulator off it, firing `WO_LEACHBEAM_ABSORB` (a real
  trigger, confirmed: it is inlined into `LeachBall_Advance`'s own wrap, not
  only in the standalone, uncalled `LeachBeam_SpawnAbsorbEffect` at
  `0x00114a00`) as a one-shot burst at the wrap point.
  **`LAUNCH`/`HIT_TARGET`/`BREAK` have no discoverable caller** - confirmed
  three independent ways this pass (the OPD block they sit in is ordinary
  `.opd` layout, not a vtable; the one real vtable `LeachBeam` installs
  resolves to generic render-node functions only; a raw byte/fourcc scan of
  the whole `EBOOT.elf` finds no reference to any of their OPD addresses
  anywhere, validated against a byte pattern known to exist) - the same
  shape `WO_CANNON_MUZZLEFLASH`/`_HOTSPOT` already have on cannon.md.

  **The ball draws since 2026-09-25 (third pass).** The earlier "flat grey
  sphere" had two causes: the inline sphere's `Uv1` was read out of its
  colour bytes (`NaN`), and its material's program had no shading path.
  Both are closed - see
  [hd-unlit-programs.md](../../docs/rendering/hd-unlit-programs.md) - and
  `LEACH_BALL_DRAWN` is deleted. Still chosen, not measured: the ball's
  orientation and scale (`_opd_FUN_001141d8` unresolved past 70), and no
  original capture exists to compare the picture against.
- **Cannon: done 2026-09-25**, see
  [ps3-hdfury-eu/cannon.md](../../docs/ghidra/functions/ps3-hdfury-eu/cannon.md).
  The earlier "round body wired" was wrong: `CannonBullet_Update` shows
  `hd_muzzleflash` only while the round is under 0.1 s old, at the firing
  craft's `cannon_flash_left`/`_right` locator (re-rolled scale 0.25..1.0,
  Z stretch 0.7..1.3), then hides it. It now draws that way
  (`oag_hd::race::CANNON_LOOK`, `race/weapons/visuals/cannon.rs`,
  `livery::cannon_flash`), with HD's own `Cannon_bolt.gtf`/`Cannon_muzzle_flash.gtf`,
  the 0.25 half-width and the full-segment streak. Still open on it:
  (1) the sim spawns the round a quarter-hull off the nose, where HD fires
  it from the locator itself (moving it moves the hashes); (2) which parity
  bit maps to `left` is chosen, not measured; (3) the quads' blend/depth
  state on HD (`0x002c4ad0`) is unread, Pulse's pipeline stands in;
  (4) `FUN_001310e8`'s `0x006778c8` call looks like a per-round point
  light, unread; (5) what sets `ship+0x5f42` (the `Ship Muzzle` fallback
  branch) is unread.
- **Missile (2026-09-23, read, not wired):** `HD_missile_ball_bloomring` is
  the flying missile's head (`Missile.cpp` constructor `0x0011ccc8`), not an
  explosion model; `HD_missile_explosion` loads in `0x00154cf0` (from
  `MissileManager_Construct`), binds `UV_offset` and `Shockwave_scalar`, and
  grows by its own keyed `Anim Transform` (1 -> 18x). What starts it is
  unread - see weapons.md's 2026-09-23 section for every address.
- ~~The Bomb's five-model detonation~~ **Done 2026-10-07**: four models, eleven
  instances, one blast object (`NormalBombBlast_*`, weapons.md), checked by
  running the executable's own instructions; `--force-bomb-trip` frames it.
- `0x00121418` (`Plasma_PostUpdate`'s visual placement) suggests a
  velocity-plus-carried-normal basis for the bolt, not velocity alone, but
  is not resolved past confidence ~55 - stays unrenamed per `CLAUDE.md`'s
  below-70 rule. The engine draws the bolt velocity-only for now, same as
  the Rocket, documented as chosen rather than measured.

- **2026-10-05 (`hd-weapons`): the Plasma explosion's `UV_offset` is played on
  the ring and halo, and was never a keyed track.** The trio's `Root` node
  authors one constant key; the field is the blast's age in seconds
  (`MeshImporter_SetTime`, see plasma.md's 2026-10-05 section, which corrects
  the 09-25 "keyframe evaluator" reading). `slots::CLOCK_SCROLL_RING`/`_HALO`
  plus a per-drawable `model_clock`. Still open on the explosion: the sphere's
  noise tap (needs the sun, ambient, `0x7480de6d` and the reflection probe, all
  unwired), and the ring/halo's own colour and alpha combine (the engine's
  path stands in; only the tap coordinates are the program's).
- **Missile's detonation pair and the Bomb's five detonation models: no
  trigger read, so nothing drawn.** `HD_missile_explosion`'s object (vtable
  `0x00864b38`, shared with the constructors at `0x00154990`/`0x00155c10`/
  `0x00155d00`; slot 5 `0x00155420` only writes the per-viewport matrix at
  `this + 0xf0`) has no found writer of `this + 0xf0` and nothing that starts
  its clock; `Shockwave_scalar` and `UV_offset` are both its age. The Bomb's
  `HD_Mine_halo`/`HD_bomb_*` have neither a load order nor a placement read.
  Next: the manager's per-tick walker on the hit path.

## 2026-10-07 (`hd-weapon-fx`): the Missile's pool read live, not wired

Read `weapons.md` last section and `rpcs3-capture.md`, "Polling guest memory live". The pool is
entered only on a craft hit (never on a wall: state 1's 5th bounce left it untouched), lives 1.0 s
(conf 55) and plays the model's own keys. Not wired because: (1) the three `hd_missile_explosion_*`
materials' fragment programs are unread, (2) no keyed-scale node clock exists for an HD weapon model,
(3) the only captured use came from an unidentified firer. Next: read the three programs
(`scripts/ps3-microcode.py fp-file`), add a keyed node clock beside `bomb_blast::hd`, then film
a Missile hitting a rival (an AI craft in front of the player; state 1 fires one, but it needs a
target in range, or a second RPCS3 boot with `--teleport-back`-style placement behind a rival,
`rpcs3_place.write_pose`). `scripts/rpcs3-mem-poll.py` times the pool without pausing.

## Next Steps

1. ~~Plasma first: per-title entries, `HD_plasma_ball` on the bolt, the
   ring/sphere/halo trio on `blast_models.rs`'s HD branch with the recovered
   ease, `WO_PLASMA_LIGHTNING_EXPAND`/`_COLLAPSE` on the recovered
   trigger.~~ **Landed 2026-09-17** - see plasma.md's own dated section for
   what the implementation pass itself found (the Collapse/Draw split, the
   oversized picture).
2. ~~Cannon~~ **done 2026-09-25** (see the Open bullet). ~~LeachBeam~~
   **its period law, `WO_LEACHBEAM_ABSORB`'s trigger, and the ball's own
   placement/draw path done 2026-09-25** (see the Open bullet);
   `LAUNCH`/`HIT_TARGET`/`BREAK` confirmed to have no trigger. **Drawn
   2026-09-25** through its own program. Left open: the ball's own orientation
   and scale are chosen, not measured (`_opd_FUN_001141d8` unresolved past
   confidence 70); whether `param_4[4]` really is the beam's own length is
   still not confirmed; and `WO_LEACHBEAM_ENERGY` playing on HD when the
   retail executable never names it is still unfixed. A live RPCS3 look at
   one Cannon shot would settle the Cannon's own open points (side sense,
   spawn point, blend).
3. Rocket/Missile/Mine/Bomb **bodies** are wired; the Bomb's detonation is
   drawn (2026-10-07). **The Missile's explosion is the next one; it is read
   this far** (all with Ghidra truncated at AltiVec, so use capstone over the
   raw bytes): a pool of 16 objects at the manager's `+0xc0`, count at `+0x10c`,
   entered through `0x00141288` (also `0x001423a8`, `0x00143580`). **Start**
   `0x00155568` copies the matrix to `+0xf0`, builds **16 randomly rotated
   entries at `+0x190` (stride `0x50`, `0x28c660` = a ranged random)** of
   unknown purpose, writes the per-viewport frame like the Bomb's, calls
   `AnimNode_UpdateTransformTree(model, 0)`, spawns `WO_MISSILE_EXPLO` (tag
   `'MIEX'`) and lights a point light. **Draw** `0x00155420` (vtable slot 5)
   draws the one model (`+0x184`) at `+0xf0 + viewport * 0x40`. **No per-tick
   update was found**: the model animates by its own keys (`sphere` and `bloom`
   scale 1 to 18 over 60 frames) on the node clock, `UV_offset` and
   `Shockwave_scalar` its age. **Open:** the lifetime (what retires it), the
   16 entries, and the four-plus materials' programs (`sphere`, `bloom`,
   `rays`, `shockwave` nodes). The harness is
   `data/scratch/hd-weapon-blasts/{ppcdis,emu,runblast,spec}.py`; the camera
   table's lane masks at `0xc47730` are runtime-initialised and must be seeded.
4. ~~Settle the Plasma explosion's scale~~ **Done 2026-09-23**. ~~Read what
   `node + 0xc0` is on the `PTR_PTR_008b3988` node class~~ **Done
   2026-09-25 at the mechanism level**: it is a generic Anim-Transform
   node's own live output, bound to the shader's `UV_offset` constant *by
   pointer* (`Material_BindInstanceParamPointer`), the same two-helper
   pattern Missile's own `UV_offset`/`Shockwave_scalar` bindings use on a
   different node - see plasma.md's 2026-09-25 section. **Done 2026-10-05**: the field is the blast's age
   (not a track - the keys are constant), played on the ring and halo.
   ~~The sphere's track-fitted basis~~ **2026-09-25: the found/not-found
   branch structure in `WeaponExplosions_Start` is read (confidence 60,
   see plasma.md); what actually seeds the found branch's construction
   (`r1+0x140`, presumably from the `RaceManager+0xbc` track/spatial
   query `FUN_000a97f0` calls) is not traced to its source.** The sphere's
   `noise.gtf` sampler role remains unread. Compare against a real RPCS3
   capture of one detonation when the held-weapon slot is known - no
   capture of the original's blast exists yet.
5. ~~Make `Race::projectile_model_matrices` a rotation and honour state bit
   4 for every HD weapon model~~ **Done 2026-09-24.** Next on it: read HD's
   own Rocket/Mine/Bomb per-tick placement (the `Rocket` vtable off
   `Rocket_Construct` `0x001254c8`) so the HD poses stop borrowing Pulse's.

## From the HANDOVER.md index (moved 2026-09-25)

every weapon model entry is a Pulse PSP name; HD authors its own `.rcsmodel` set per weapon, Plasma's triggers already read; 2026-09-24: placement a rotation, HD bodies culled as authored, HD's own placements unread

## 2026-10-06 (hd-weapons): re-check

Every HD weapon that has a model draws it now and takes the circuit's scene
block (`docs/rendering/hd-unlit-programs.md`, "Weapon scene blocks"). Still
undrawn: the Missile's pair, the Bomb's five (blast pair unwritten), the
beam's strips (`hds-bomb-blast-scene-block-and-leach-strips.md`). No trigger
was found for the effects listed above.

## 2026-10-07 (`hd-weapon-ref`): a held weapon now exists on RPCS3

`scripts/rpcs3-hd-weapon.py --state N` gives the player any weapon and films the
shot (`docs/reverse-engineering/rpcs3-capture.md`, "Giving the player a weapon").
State 9 is the Bomb, 7 the Plasma, 8 the Cannon ("Machine Gun"), 0 the Rocket,
4 the Turbo. Not done from it, in order of ready-ness:

- **Plasma ring reference.** State 7 was fired twice on one boot (`data/scratch/hd-weapon-ref/e11/s3.png`):
  a white flash and violet sparks at the craft within 0.3 game seconds, **no growing violet
  shell** in the 5 fps contact sheet. That is not the "violet shell around the track" this
  build draws; the shot hit something at the craft's nose, so it may be the bolt's own
  impact rather than the ring. Re-film at 30 fps down an open straight with `--teleport-back` and
  compare frame by frame before judging the ring.
- **Missile** (state 1 or 2, readouts 15 and 15 do not tell them apart), **LeachBeam** (state 3
  or 10, readout 1; state 3 never fired) and **Cannon's side sense**: one boot each with the
  recipe; none was filmed this pass.
- **Absorb shell colour**: circle with a held state spends it and plays the shell; a clean
  frame was not taken.
