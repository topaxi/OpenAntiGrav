# The Rocket's craft blast: where it stands and what it rides

2026-11-26. Read in `/hdfury/EBOOT-ps3-hdfury-eu.elf` (TOC `r2 = 0x008ad4d8` for every
function here, `scripts/ps3-toc.py toc`: "exact"), with the 2048 (`vita-2048-eu-v104`) and
Omega (`ps4-omega-eu`, patch 1.07) twins and Pulse's PSP spawn for contrast. Ghidra marks part
of the HD spawn as bad data (a Cell `lvlx` it cannot decode), so the carry block below is
quoted from `llvm-objdump -d --triple=powerpc64`.

**The law (confidence 85).** On a Rocket's craft hit the spawn copies the struck craft's
position, dropped 2.5, into the new `WO_ROCKET_EXPLO` once. It then writes the craft's body
velocity times 1/60 into the instance's inherited velocity. Every particle born afterwards adds
that times its emitter's `+0x4d0`. The blast is not parented: its emitters stay where it was
spawned, nothing refreshes the velocity, and the craft's later fate does not reach it. The cloud
travels with the craft only because `WO_ROCKET_EXPLO` authors `+0x4d0 = 1.0` on every root, and
it falls behind because those roots author a drag of 0.92 a tick.

| Address | Name | Confidence | What it does |
| --- | --- | --- | --- |
| `0x0014aa88` | `Rocket_HitCraft` | 72 | the only caller of the spawn; takes the struck racer, spawns the blast once if any viewport sees the craft (`0x000d02f0`), then pushes nearby craft (`0x00149f70`) |
| `0x0014a7d8` | `Rocket_SpawnCraftBlast` | 85 | spawns `WO_ROCKET_EXPLO` (tag `ROEX`) at a copied matrix, hands it the struck craft's velocity, submits a point light |
| `0x00283b40` | `Psys_SetInheritedVelocity` | 82 | stores a vector at instance `+0x160` and recurses into `+0x24` and `+0x1b4` |
| `0x0027c288` | `Psys_FindInstanceByHandle` | 80 | the handle table on `*(manager) + 0x468c` (key mod table size, stride 12, match on `+4`) |
| `0x00149f70` | `Rocket_PushNearbyCraft_q` | 60 | for each other racer within a radius, adds `normalize(d) * (1 - |d| / radius) * strength` to the racer's `+0x110` |

## The spawn copies a point

`Rocket_HitCraft` (`0x0014aa88`) reads the struck racer `*(this + struck * 4 + 0x58)`, takes its
position at `+0x90` and lowers y by the TOC float at `-0x2500` (`0x40200000`, 2.5), the drop
Pulse's spawn already carries (`oag_title::engine_effects::CRAFT_BLAST_DROP`). That vector is
the only argument `Rocket_SpawnCraftBlast` gets besides `this` and the struck index.

`Rocket_SpawnCraftBlast` builds an identity matrix on its stack (rows from `0x00769d70`) with
that position in row 3, allocates a `0x180`-byte effect node, registers it with
`RaceManager` (`0x003238f8`), and calls the effect constructor:

```
0014a8e0  lis   r5,0x5845 ; ori r5,r5,0x4f52   tag 'ROEX'
0014a8e4  lwz   r4,-0x2510(r2)                 "WO_ROCKET_EXPLO" (0x007854f0)
0014a8f0  addi  r6,r1,0xb0                     the stack matrix
0014a8f4  li    r7,0x0                         attach flags
0014a8f8  li    r8,0x0
0014a8fc  bl    0x002c9108
```

The constructor (`0x002c9108` -> `0x002c8c70`) keeps the caller's pointer only when bit 0 of
the flags is set (`param_1 + 0xd0 = param_4`, an attached effect, the shape
`WO_SHIP_COLL_SPARK_DAMAGE` uses with flags `1`). With flags `0` and a matrix it copies the four
rows into the node's own `+0xf0..+0x12f` and points `+0xd0` there. So the blast keeps a copy:
it does not ride the craft's transform.

## The carry

```
0014a92c  lwz   r3,0x468c(r9)                  the particle manager's handle table
0014a930  bl    0x0027c288                     instance of the handle just spawned (*0x00ad5fd4)
0014a93c  beq   0x0014a994                     none: skip
0014a94c  addi  r10,r2,-0x2504                 0x008aafd4 = 0x3c888889, 1/60
0014a960  lvlx  v13,0,r10                      (Cell; Ghidra: bad data)
0014a96c  vspltw v13,v13,0
0014a970  lwz   r10,8(r11)                     the struck racer
0014a974  lwz   r9,0xf0(r10)                   its craft
0014a978  lwz   r11,0x6944(r9)                 its body
0014a97c  lvx   v0,r11,r0                      r0 = 0x190: the body's velocity
0014a984  vmaddfp v0,v0,v13,v1                 velocity * 1/60 (+ 0)
0014a988  stvx  v0,r1,r0                       r0 = 0xa0
0014a98c  bl    0x00283b40                     Psys_SetInheritedVelocity(instance, &v)
```

Body `+0x190` is the velocity the physics pages already read (`detonator-mine.md`,
`zone-destroy.md`). `Psys_SetInheritedVelocity` writes instance `+0x160`, which
`Psys_InitParticle` (`0x00289890`, `0x002899ac`-`0x00289a18`) adds to each newborn's velocity
times the record's `+0x4d0` ([engine-trail.md](engine-trail.md), "What the float and the
velocity do"). A child instance takes the parent particle's velocity times its own `+0x4d0`
(`0x0028a344`-`0x0028a358`), which `oag_fx::psys` already does.

**Written once.** Every direct branch to `0x00283b40` in the executable (`llvm-objdump`,
`bl 0x283b40`): `0x000d99a8` (in `Ship_PlayDestroyExplosion`), `0x00124738`, `0x00124cec`,
`0x0014a98c` and its own recursion. Only `0x0014a98c` reaches the blast, and it runs at the
spawn.

What `WO_ROCKET_EXPLO`'s twelve emitters author, read through `oag_fx::psys` from the disc:
`+0x4d0 = 1.0` on all eleven but `trail` (`0.0`); roots `WO_ROCKET_EXPLO`, `SMOKE_BLUCLD`,
`FIREBALL_FIJETS` and `FIREBALL_TUMULT` drag `0.92` a tick, `FIRESPIKES` and
`FIRE_TRAILS_ONEQUAD` none.

The spawn ends by submitting a point light at the same position: colour `(7, 5, 1)` from a
static it fills once (`0x00994510`), `D = 100.0` (TOC `-0x2538`), `w = 1.0` (TOC `-0x2570`),
through the `SpuLight_AddCandidate` thunk `0x006778c8` ([renderer.md](renderer.md)).

## Against the capture

The 2026-11-26 film ([particle-fill.md](../../../rendering/particle-fill.md), "Against the
original at a near blast") measured the original's large-particle centroid behind the struck
craft. Ours with the law, a forced hit on a rival at about 100 units a second
(`craft_blast_carry::hds_craft_blast_travels_with_the_struck_craft`, all live particles):

| Age | Original: craft moved | Original: cloud from craft | Ours: craft moved | Ours: cloud from craft |
| --- | --- | --- | --- | --- |
| 0.18 s | 23.9 | 6.1 | 19.3 | 5.8 |
| 0.23 s | 29.0 | 9.6 | 24.3 | 7.4 |

The cloud's lag over the craft's travel: original 0.26 and 0.33, ours 0.30 and 0.30. Without
the carry ours stays at the spawn point: 0.6 units along a craft that moved 34.

## The other titles

- **2048: ported.** `Rocket_SpawnCraftBlast` (`0x81206928`, confidence 80) spawns `WO_ROCKET_EXPLO` (tag `ROEX`) with flags `0, 0` at
  a stack matrix of the passed position, looks the instance up by its handle (`+0xec`), and
  hands it `*(*(*(racer)+0xe8)+0x5f84)+0x90` times `0x3c888889` (1/60) through
  `Psys_SetInheritedVelocity` (`0x812aa644`, 80), which stores it at `+0x140` and recurses into `+0x24` and `+0x184`: the
  same setter. A light of `(7, 5, 1)` follows (`FUN_8103731c`).
- **Omega: ported.** `Rocket_HitCraft` (`0x0139a6b0`, 75; the only `ROEX` immediate in the 1.07 executable)
  lowers the racer's `+0x90` by 2.5, spawns through `FUN_0175ea50` with flags `0, 0`, looks
  the instance up by `+0xc0` and hands it `*(craft + 0x75c8) + 0xa0` times `0.016666668`
  through `Psys_SetInheritedVelocity` (`0x01728780`, 80; stores `+0x1d0`, recurses `+0x88` and `+0x228`). It queues a light
  of `D = 100`, colour `(7, 5, 1)`, and pushes nearby craft by `(1 - d / r) * strength`, as
  HD's `Rocket_PushNearbyCraft_q` does.
- **Pulse PSP: checked, differs.** `Rocket_SpawnCraftExplosion_q` (`0x0886ed34`) spawns at a
  copied matrix with flags `0, 0` and writes no velocity, then starts `ScreenFlash_Start(0)`.
  Pulse PS2 inherits it. Pure is not read.

Wired as `oag_title::Effects::craft_blast_carry` (HD, 2048 and Omega `Measured`);
`RaceView::craft_blast_velocity` hands the struck craft's velocity to `Stage::play_carried`.

## The screen wash: narrowed, not found

The capture's full-screen additive quads (half height exactly twice their depth) were
re-measured against this spawn's geometry (the films' `hit.json` struck pose, the frames'
cameras, the pairs' field files for the craft's later position). In all three hits they are:

- one quad per draw, square, colour bytes opaque, warm white to yellow
  (`(255, 231, 178)` at 0.1 s, about `(255, 233, 91)` to `(255, 224, 79)` later);
- **near the spawn point** (2.3 to 11.9 units from it) while the struck craft is 13 to 30
  units on: they do not travel with the cloud.

So they are not the carried particles of the blast's travelling roots. Candidates left, none
above 45: `FIRESPIKES` (64x64 additive sprite, near cap 2.0, life 11 +- 6 ticks, which matches
"gone by 0.3 s", but it emits 5 a tick and is a from-spawn streak, not a square), or something
drawn for the spawn's point light. **Not wired.** The check that decides it is a capture with
one candidate removed (the `.pob`'s emission count zeroed in memory, or the `bl 0x006778c8` at
`0x0014aa34` replaced by a `nop`), filmed with `scripts/rpcs3-hd-rocket-film.py`.

## Open

- What draws the near-spawn full-screen quads (above).
- `Rocket_PushNearbyCraft_q`'s radius and strength table (`*(TOC -0x2568) + 4 * *(... + 0xc)`,
  fields `+0x20` and `+0x24`), and whether `+0x110` is the pending impulse Pulse's blasts post
  ([weapon-lineage.md](../../../gameplay/weapon-lineage.md), "Blast push").
