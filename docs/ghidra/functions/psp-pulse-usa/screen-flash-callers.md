# Who starts a screen flash: the fifteen callers of `ScreenFlash_Start`

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`. Read 2026-09-30 with
the Ghidra bridge: `get_xrefs_to(0x088f00c0)`, each call site's delay slot for the
kind (`li a1, N`), each caller's gate read above it.

**Status:** **all fifteen callers read, ten of the twelve kinds placed and built
from a live-measured caller, two with no caller, and the placed-but-unbuilt ones
named below.** The consumer and the kind table are in
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
| `Ship_SpawnExplosionSmall` (`0x0883e064`) | `0x0883e114` | 0 | `Ship_SetState` state 5, every craft | yes, the player excluded (see "Measured live") |
| `Ship_SpawnExplosionBig` (`0x088407b0`) | `0x088408a0`, `0x0884090c` | 7 when `craft+0x368 == 0`, else 0 | `Ship_UpdateDestroyed`, and state 7 in modes 0, 0xe-0x12 | yes |
| `PlasmaBlast_Construct` (`0x0885fd90`) | `0x0885fe68` | 1 | every Plasma ending | yes |
| `Repulser_SpawnWaves` (`0x08876300`) | `0x088765ac` | 2 | the Repulser's waves starting, `blast_time` after the fire ([repulser.md](repulser.md)) | yes, since 2026-10-04 |
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

**Built 2026-10-01:** the two state 5 particles, `WO_SHIP_FXNODE_EXPLO` then
`WO_SHIP_DEATH_SPARKS` at each node of the **wreck** (`oag_raceplay::wreck_fx`;
read live, see [ship-wreck-model.md](ship-wreck-model.md)); then `WO_SHIP_EXPLOSION`
at the placement below, and `Camera_ArmShake` for the player on both (`(0.3, 0.4,
camera, 3)` at state 5 and `(0.8, 0.6, camera, 1)` at state 6, both played where the
race ends on the state 5 edge too, since 2026-10-02 the finished race steps its cosmetics, `Race::tick_cosmetics`), and **the `Bomb_Shockwave.vex` ring below, built 2026-10-01**
([ship-shockwave.md](ship-shockwave.md)). **Not built:** the two sounds.

### `Ship_SpawnExplosionBig`'s placement, read and measured 2026-10-01

`FUN_088407b0` copies the live hull model's world matrix (`FUN_089451dc(entity+0x8b0)`:
`model+0x3c`, `+0x40`, sixteen floats) into a local (`local_370`), and for the local player
plays `EXPLBIG_PC` and `SHIP_DEST`, starts kind 7 at the craft's node (`+0x794`, `+0x30`)
and arms the shake `(0.8, 0.6, camera, 1)`; for anyone else plays `EXPLBIG` and starts
kind 0. Then, in order:

1. it **squashes the hull's own node matrix** with a `1e-4` scale on its three rows
   (`0x38d1b717`) through `FUN_08945284`, which makes the live model vanish;
2. when `DAT_08b34320` is non-zero (it was `0x8c0f410` live) it builds a
   **`Data\Weapons\Bomb_Shockwave.vex`** object (`FUN_0885ecf0`, a `0xb0`-byte node)
   from the same matrix: scale `0.1`, a ring that grows to `20` times its authored `5.4` radius - read
   whole and logged live 2026-10-01, and **drawn** on the Bomb's blast pool
   ([ship-shockwave.md](ship-shockwave.md): `FUN_0885ecf0`, `FUN_0885efc4`);
3. it moves the matrix's translation `DAT_08ab0de8` times its **own second row** back
   (`4.0`: read in `.data` and live), and
4. calls `Psys_Spawn_q(DAT_08ab2248, "WO_SHIP_EXPLOSION", 'EXPL', &matrix, 0, 0)`:
   flags `0` copy the matrix into the instance (`FUN_08916200`) and the parent is the
   particle world root, so the blast sits **in the world at the wreck and does not ride
   the craft**.

Live (Talon's Junction, the player's craft put into state 4, 90 frames after the state
5 edge): the matrix's rows were **0.75 long** (the craft model's own scale) and its
translation was the craft node's position moved `2.9995` units along `-up`, which is
`4.0 * 0.7499`, to four digits. Ported as `oag_raceplay::wreck_fx::EXPLOSION_DROP`:
`model * (0, -4, 0)`, oriented by the model's up, thrown with the big flash.
Confidence **90**.

**No scale is passed to any of the three wreck effects.** `Psys_Spawn_q` takes a matrix
and two flag words and no severity; the only writer of the one-shot severity
`DAT_08abf564` (`FUN_08916200` stores it into the node at `+0xb8` and clears it) is
`Ship_PlayAbsorbFeedback`. So the `1.0` the port passes is the original's neutral
value, not a guess. **The matrix's `0.75` rows do scale the emitters' spawn offsets and
velocities** (and not their sizes), read 2026-10-01 off the four explosion pools and the eight node
instances: [particle-system.md](particle-system.md#the-instance-matrix-scales-a-root-emitters-spawn-and-a-run-emits-one-tick-short-2026-10-01).

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

It calls nothing that spends damage. **Built** as `oag_raceplay::SHURIKEN_EXPIRE_EFFECT`,
with the fuse ending reported as an `Impact` with `blast: false`
(`oag_weapons::projectile`); the sound is not.

## Measured live (2026-09-30, PPSSPP, Talon's Junction, the player stationary)

One breakpoint on `ScreenFlash_Start` (`0x088f00c0`), a weapon bit written into
`craft+0x1b8` (or a 500-point hit posted into the pending-damage channel), and
the first hit logged: kind, `ra`, frames since the write, and the distance from
the eye.

| Trigger | Kind | `ra` | Frame | Where | What it confirms |
| --- | ---: | --- | ---: | --- | --- |
| Missile (`0x40`) | 0 | `0x08868e80` | 179 | 563 units out | `Missile_SpawnExplosion`; its self-detonation at 3 s. Past `far`, so nothing washes |
| Shuriken (`0x20000`) | 0 | `0x08870db0` | 119 | 367 units out | `Shuriken_SpawnExpiry`, at the authored 2 s fuse. Past `far` |
| Plasma (`0x4`) | 1 | `0x0885fe70` | 85 | 56 units out | `PlasmaBlast_Construct` |
| Mine (`0x2`) | 8 | `0x08868058` | 29 | on the craft | `Mine_SpawnExplosion` |
| Bomb (`0x100`) | 3 | `0x088721a4` | 30 | on the craft | `BombBlast_Construct` |
| A posted 500-point hit | 0 | `0x0883e11c` | 31 | on the craft | `Ship_SpawnExplosionSmall`, 30 frames after the hit: state 4's 0.5 s |
| the same | 7 | `0x088408a8` | 121 | on the craft | `Ship_SpawnExplosionBig`, 90 frames later: state 5's 1.5 s |

**The Mine and the Bomb tripped on the stationary craft that laid them, 29 and
30 frames later.** That is `Bomb_InArmingDelay`'s 0.5 s, read for the Bomb in
[mine.md](mine.md) and now seen for the Mine as well: the original has no
permanent owner exclusion. This port had one and retired it on 2026-10-07:
`oag_weapons::projectile::mine::triggered_by` now exempts the owner for
`OWNER_EXEMPT_SECONDS` (0.5) only; see the bomb-owner section of
[mine.md](mine.md).

**The player's own state-5 wash is measured faint, and this port leaves it out.**
The flash's falloff reads `DAT_08ab10b0`, the *active* camera (`ScreenFlash_DistanceFalloff`,
`0x088effb8`), and by state 5 that is the destroy camera (`Camera_SetMode(5)`):
`(32.2, -23.4, -202.0)`, **168.7 units** from the wreck, a falloff of 0.25. This
port keeps the chase camera at 11.6 units, where kind 0 would be four times as
strong. Kind 7 has no falloff and is started.

**The wash matches the original's frame by frame for Plasma.** Struck minus the
frame before, over two dark track patches away from the fireball, red, green, blue added:

| frames after the flash starts | original | ours |
| ---: | --- | --- |
| 0 | 120, 18, 169 | 115, 17, 164 |
| 10 | 100, 15, 155 | 98, 15, 154 |
| 18 | 72, 11, 136 | 70, 10, 134 |
| 28 | 49, 6, 114 | 47, 5, 111 |
| 38 | 29, 4, 88 | 28, 3, 87 |

Ours runs one frame behind (it starts a tick later), and every channel is within
5 counts. Kind 8's first frame in the original adds `(96, 97, 0)`: yellow, no
blue, as the table says. Kinds 0, 3 and 7 were seen placed and coloured but not
measured per frame; the Missile and Shuriken are placed, not seen (their
detonations were out of range).

## Applied names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0886ff38` | `ShurikenPool_Update` | 85 |
| `0x08870c78` | `Shuriken_SpawnExpiry` | 85 |
| `0x0883e064` | `Ship_SpawnExplosionSmall` | 80 |
| `0x088407b0` | `Ship_SpawnExplosionBig` | 80 |
| `0x08876300` | `Repulser_SpawnWaves` | 80, moved to [repulser.md](repulser.md) |

`Repulser_SpawnWaves` was held at 62 here because its caller was unread. The
caller is `Repulser_Update` (`0x08875400`), read 2026-10-04 on
[repulser.md](repulser.md), which owns the row now.

## Not read

- The `g_display+0x5dec` gate on the draw, as in the consumer's page.
- Which of `Race_CreateModeObject`'s and `RaceMode_UpdateIntro`'s washes a
  player actually sees: the intro is a fly-through this port does not have.
- Whether the flash draws over the results screen when a craft's big explosion
  lands after its race has ended.
- ~~What the `Bomb_Shockwave.vex` object `Ship_SpawnExplosionBig` builds looks like.~~ Read and drawn
  2026-10-01, [ship-shockwave.md](ship-shockwave.md); its first-five-frames brightness is not matched.
