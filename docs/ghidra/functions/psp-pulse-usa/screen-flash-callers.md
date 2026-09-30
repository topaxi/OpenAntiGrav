# Who starts a screen flash: the fifteen callers of `ScreenFlash_Start`

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`. Read 2026-09-30 with
the Ghidra bridge: `get_xrefs_to(0x088f00c0)`, each call site's delay slot for the
kind (`li a1, N`), each caller's gate read above it.

**Status:** **all fifteen callers read, eleven of the twelve kinds placed, ten
built.** The consumer and the kind table are in
[particle-system.md](particle-system.md), "The screen flash's consumer, read and
measured"; this page is the other end. The table there was re-derived from the
function's own stores (all twelve kinds, every key and every key time) and
matches, with one addition it did not have: kind 7 has three keys, the second at
a tenth of its duration.

Confidence **85** for every caller's kind and gate: each is a branch-clear read of
the call site, and the kind is a literal in the delay slot.

## The callers

| Caller | Address of the call | Kind | Gate | Built |
| --- | --- | ---: | --- | --- |
| `Rocket_SpawnCraftExplosion_q` (`0x0886ed34`) | `0x0886ee5c` | 0 | a Rocket that struck a craft | yes |
| `Missile_SpawnExplosion` (`0x08868d50`) | `0x08868e78` | 0 | every ending | yes |
| `Shuriken_SpawnExpiry` (`0x08870c78`) | `0x08870da8` | 0 | every ending (see below) | yes |
| `Ship_SpawnExplosionSmall` (`0x0883e064`) | `0x0883e114` | 0 | `Ship_SetState` state 5, every craft | yes |
| `Ship_SpawnExplosionBig` (`0x088407b0`) | `0x088408a0`, `0x0884090c` | 7 when `craft+0x368 == 0`, else 0 | `Ship_UpdateDestroyed`, and state 7 in modes 0, 0xe-0x12 | yes |
| `PlasmaBlast_Construct` (`0x0885fd90`) | `0x0885fe68` | 1 | every Plasma ending | yes |
| `Repulser_SpawnWaves_q` (`0x08876300`) | `0x088765ac` | 2 | the Repulser's field beginning | no: the Repulser is not built |
| `BombBlast_Construct` (`0x08872078`) | `0x0887219c` | 3 | every Bomb detonation | yes |
| `Quake_Update` (`0x0891d268`) | `0x0891dc7c` | 4 | every frame the wave runs | yes (earlier) |
| `Ship_SetState` (`0x08844100`) case 3 | `0x088441fc` | 6 | entering state 3 from another state, `craft+0x368 == 0` | yes |
| `Ship_UpdateRespawn` (`0x08847914`) | `0x08847a2c` | 6 | the respawn timer running out, `craft+0x368 == 0` | yes |
| `Mine_SpawnExplosion` (`0x08867f1c`) | `0x08868050` | 8 | every Mine ending | yes |
| `Race_CreateModeObject` (`0x0882112c`) | `0x0882116c` | 9 | unconditionally, on building the race scene | no |
| `RaceMode_UpdateIntro` (`0x08829e6c`) | `0x0882a184` | 10 | the intro camera's pass ending (or skipped) | no |

**Kinds 5 and 11 have no caller.** No direct call, and a byte search for
`0x088f00c0` as a pointer (`c0 00 8f 08`) finds nothing, so it is not in a table
either. Indirect `jalr` sites were not traced exhaustively; a plain function is
not reached through a vtable, so this is unlikely to be a miss.

**A Rocket's track hit starts no flash.** `Rocket_Update`'s two track branches
call `Psys_Spawn_q` for `WO_ROCKET_EXPLO_TRACK` and nothing else, and the xref
list above holds only one Rocket function. Nothing washes for a close track hit.

## Kind 7 has three keys

`ScreenFlash_Start` case 7 stores key 0 `(1, 1, 1, 1)` at time 0, key 1
`(1, 1, 0, 0.8)` at time `0x3dcccccd` (0.1) and key 2 `(1, 0, 0, 0)` at time 1.0.
`ScreenFlash_Update` walks the keys as a piecewise ramp: the first `j >= 1` whose
time is at or past `t` closes the segment `j-1 .. j`, and the segment's own
reciprocal was precomputed by `Start`'s loop. Every other kind stores two keys,
so it is one lerp. The node's key count is 4, written once by
`ScreenFlash_Construct`; `Start` never writes it, and the two unread key slots
are never reached because `t < 1` ends the flash first.

## The craft's own washes

`Ship_SetState` (`0x08844100`), whose jump table the decompiler cannot render
(see [zone-mode.md](zone-mode.md)), reads as:

- **case 3** - if the previous state was not 3 and `craft+0x368 == 0`: play
  `RESET` dry, `ScreenFlash_Start(6, DAT_08a90a00)`, and set the state timer to
  `min(shield - 1, 5)` (10 in game mode 6);
- **case 4** - the explosion opens `~BLOWUP` (already built), timer 0.5;
- **case 5** - `Ship_SpawnExplosionSmall`: the flash (kind 0 at the craft's node,
  `+0x794`, `+0x30`), `Camera_ArmShake` for the player only, `EXPLSMALL`, then
  for each of ten `Fx` nodes at `craft+0xc80` a `WO_SHIP_FXNODE_EXPLO` and a
  `WO_SHIP_DEATH_SPARKS`;
- **`Ship_UpdateDestroyed`** (`0x08847650`), when state 5's 1.5 s timer runs out:
  state 6 (8 in modes 8 and 0x12), then `Ship_SpawnExplosionBig`: `EXPLBIG` and,
  for the player, `SHIP_DEST` and kind 7 with `Camera_ArmShake`; for everyone
  else `EXPLBIG` positional and kind 0; then `WO_SHIP_EXPLOSION`.

**Not built:** the particles (`WO_SHIP_FXNODE_EXPLO`, `WO_SHIP_DEATH_SPARKS`,
`WO_SHIP_EXPLOSION`), `Camera_ArmShake` on either, and the two sounds. Their
triggers are the ones above and are recovered; `psys_inventory_ground_truth.rs`
still lists all three as "no craft destruction in this engine", which is stale
about the state machine (it exists: [`oag_physics::CraftState`]) and right that
nothing plays them.

## The Shuriken's ending

`ShurikenPool_Update` (`0x0886ff38`) runs `Shuriken_Update` per live blade, then a
teardown pass. A blade is destroyed when `*(craft table + 0x174) < blade+0x48`
(the authored `fuse` against its clock) with its live bit set, or when
`Shuriken_Update`'s craft-hit branch sets flag `0x14`. Either sets bit 4, and the
teardown then calls `Shuriken_SpawnExpiry` (`0x08870c78`), which:

1. spawns `WO_SHURIKEN_EXPIRE` (fourcc `SHEX`) at the blade's position with an
   identity basis;
2. starts `ScreenFlash_Start(0)` there;
3. clears the blade's flags; and the pool then plays `SHURIKENEXPL`.

It calls nothing that spends damage. **Built** as `oag_game::race::SHURIKEN_EXPIRE_EFFECT`,
with the fuse ending reported as an `Impact` with `blast: false`
(`oag_gameplay::projectile`); the sound is not.

## Applied names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0886ff38` | `ShurikenPool_Update` | 85 |
| `0x08870c78` | `Shuriken_SpawnExpiry` | 85 |
| `0x0883e064` | `Ship_SpawnExplosionSmall` | 80 |
| `0x088407b0` | `Ship_SpawnExplosionBig` | 80 |
| `0x08876300` | `Repulser_SpawnWaves_q` | 62 |

`Repulser_SpawnWaves_q` stays below 70: it spawns two `WO_REPULSER` instances at
the entity's own two positions and resets its node pair, but the weapon around it
(`FUN_08875400`) is not read. That function is left unnamed.

## Not read

- The `g_display+0x5dec` gate on the draw, as in the consumer's page.
- Which of `Race_CreateModeObject`'s and `RaceMode_UpdateIntro`'s washes a
  player actually sees: the intro is a fly-through this port does not have.
- Whether the flash draws over the results screen when a craft's big explosion
  lands after its race has ended.
