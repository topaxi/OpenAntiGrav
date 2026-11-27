# Where HD draws its particles, and the near-camera law

2026-11-25, lane `particle-fill-fable`. The question came from
[particle-fill.md](../../../rendering/particle-fill.md): a Rocket hit near the camera stacks
dozens of screens of blended particle fill in ours. How does a PS3 draw the same hit?

Read on disc 02.00 (`BCES00664`): the executable in Ghidra, the `PartRender` SPU job with
`scripts/ps3-spu-disasm.py`, a stored 720p frame dump, and eight frames captured live on
RPCS3 0.0.43-20574 at 1920x1080. Omega's CPU twin of the vertex builder was read on patch 1.07.

## Summary

| Claim | Answer | Confidence |
| --- | --- | --- |
| A reduced-resolution particle target | **No.** Every particle quad is drawn into the scene's own colour target | 90 |
| The target | 1280x720 with 2x MSAA, or `1920 - 20 * level` by 1080 with no MSAA ([scene-width-throttle.md](scene-width-throttle.md)); `A8R8G8B8` + `Z24S8` | 92 |
| When | One queue entry, layer `0x4d4`, depth 0: after the layer `0x4d0` and `0x4d3` blends, in submission order, no sort by distance | 85 |
| Draw state | Depth test `LEQUAL`, no depth write, no cull, blend by the emitter's class | 92 |
| A per-emitter resolution mode by distance | **Authored and computed, then ignored.** Byte `+0xc8c`, thresholds 17 and 35 units; the flush's handler never reads it | 85 |
| Near cull | A billboard whose centre is nearer than 1 unit in view depth is not drawn | 92 |
| Near fade | Alpha is scaled by `(depth - 1) / 3` between depth 1 and 4 | 88 |
| Size cap | A billboard's size is capped at `depth * cap`, `cap` the emitter's float `+0xc80` | 94 |
| A particle budget, spawn limit or screen-area cap | None found in the draw path. The pool is 256 instances; each emitter has its authored live cap | 70 |

So the PS3 keeps a near blast cheap in three ways, none of them a second target: it draws a
small frame (1.4 to 2.07 megapixels at 1080p, against 8.3 at 3840x2160), it narrows that frame
while the RSX is slow, and it shrinks, fades and drops the particles nearest the eye.

## The draw: one queue entry on layer `0x4d4`

`ParticleManager`'s constructors (`FUN_0027ff58`, `FUN_00280290`, [engine-trail.md](engine-trail.md))
allocate a `0x40`-byte child node and store the vtable at `0x00869a80` into it (`0x00280198`,
`0x002804d0`; TOC slot `0x008b293c`). Two of its slots are the particle renderer's:

- `+0x14`, `ParticleRenderNode_Enqueue` (`0x006af030`): the render queue's enqueue idiom
  ([renderer.md](renderer.md), "The enqueue idiom") with key `0x4d400000`, or
  `depth_override & 0xfffff | 0x4d400000` when `RenderManager + 0x11c` is set. The node computes no
  depth of its own.
- `+0x1c`, `ParticleRenderNode_Draw` (`0x006af000`): calls `PartRender_FlushDraws`
  (`0x002c3ca0`) on the singleton at TOC slot `0x008b290c`. `RenderManager_FlushDrawQueue`
  dispatches `vtable[0x1c]` on every queued item, so this is the one place particles are drawn.

The queue is sorted ascending on the key, so every particle of every system is drawn in one
run, after the layer `0x4d0` blends (the magstrip wake) and the layer `0x4d3` bomb blasts, and
before layers `0x570` and `0x5b0`. Inside the run the order is the order the instances were
submitted. Nothing sorts particles against each other or against other blended draws.

`PartRender_FlushDraws` sets, once: cull face off (`0x183c`), depth test on (`0xa74`), depth
function `0x203` (`LEQUAL`), depth mask 0, primitive restart on with index `0xffff`, alpha test on
with `GREATER 0`. Then per draw record, by the render state's blend word (`state + 0xc`):

| Blend | Alpha test | Blend | Equation, factors |
| --- | --- | --- | --- |
| 1 | on | off | |
| 2 | off | on | `FUNC_ADD`, `(SRC_ALPHA, ONE)`, alpha `(ZERO, ONE)` |
| 3 | off | on | `FUNC_ADD`, `(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)`, alpha `(ZERO, ONE_MINUS_SRC_ALPHA)` |
| 4 | off | on | `FUNC_REVERSE_SUBTRACT` (`0x800b`), `(SRC_ALPHA, ONE)` |
| other | off | off | |

## No second target: the resolution mode is dead

A draw record is `0x18` bytes (`PartRender_QueueDraw`, `0x002c39a8`): vertex pointer, vertex
count, first vertex (filled by the flush), render state index, and at `+0x10` a mode.

`Psys_DrawInstance` (`0x002c9248`) computes the mode from the emitter's byte `+0xc8c`:

```text
mode = emitter[0xc8c]
if mode == 4:                                   ; by distance
    depth = -(view * instance.position).z       ; view matrix at TOC slot 0x008b3b38
    mode = 2 if depth < 17.0 else 1 if depth < 35.0 else 0
elif mode == 3:
    mode = 0
```

The two floats are `Psys_ResolutionModeDistances` (`0x008c2670`: `17.0`, `35.0`). The mode goes
through `PartRender_SubmitJob` (its `r6`) into the record (`mullw r6, r21, r19; clrldi r7, r18`
before the call at `0x002ccd1c`).

The flush reads it at `0x002c40fc`:

```text
002c40fc  lwz    r4, 0x10(r24)        ; the record's mode
002c4100  cmpwi  cr7, r4, -1
002c4104  beq    cr7, 0x2c4128
002c410c  mr     r3, r30              ; the context
002c4110  bl     0x679508             ; a TOC stub in front of 0x003af240
002c4120  bl     0x678148             ; depth mask 0
```

`Render_BindSceneTargets` (`0x003af240`) takes the mode in `r4` and never reads it. It saves
`r3`, calls `Render_GetSceneDepthTarget` (`0x003de2f8`) and `Render_GetSceneColourTarget`
(`0x003de2b8`), overwrites `r4` with the first result at `0x003af28c`, and calls
`Rsx_SetRenderTargets` (`0x005a40f8`) with three zeroed slots for the other colour targets. Then
it sets the split-screen viewport. The two getters are seven instructions each: the address of
entry `g_SceneWidthLevel` of the array at `g_RenderGlobals + 0x08` (`g_SceneColourTargets`) and
`+0x10c` (`g_SceneDepthTargets`, `0x00c81b8c`). Neither reads an argument.

So whatever the mode, the flush binds the scene's own colour and depth targets at the current
width level. A mode that picked a smaller target would have to be read here, and it is not. The
executable has no string for a low-resolution particle option either (`strings` for `low`,
`half`, `quarter`, `offscreen`, `upsampl` near `particle` or `psys`: nothing).

Of the 189 emitters in the 71 effect files of `DATA02.PSARC`, 161 author mode 4, 22 mode 3 and 6
mode 0. The smoke of `WO_ROCKET_EXPLO` is mode 4. The data asks for a per-distance treatment
that this build does not give.

**Live, 720p.** A stored frame dump of a Detonator bomb blast (RPCS3, 1280x720): seven `QUADS`
draws of 4 to 1,440 vertices, all on target `0x00f50000`, surface word `0x03148` (colour
`A8R8G8B8`, depth `Z24S8`, antialias 3, `DIAGONAL_CENTERED_2`), pitch `0x2800`, clip 1280x720,
fragment program `0x00741000`. That target is the frame's scene target (513 of its 669 draws).

**Live, 1080p.** Eight frames paused through a race start with a Rocket fired at `GO` (width level
31 on a loaded host). In the five complete ones every particle draw is on target `0x01920000`,
surface `0x00148` (no antialias), pitch `0x2000`, clip **1300x1080**, the frame's scene target
(681 to 820 of its draws), fragment program `0x01111000`:

| Frame | Race clock | Additive quads | Alpha-over quads | Target |
| --- | --- | --- | --- | --- |
| `s00` | 1.37 s | 63 | 24 | `0x01920000`, 1300x1080 |
| `s01` | 1.61 s | 171 | 162 | same |
| `s03` | 2.11 s | 292 | 285 | same |
| `s05` | 2.61 s | 276 | 258 | same |
| `s07` | 3.11 s | 317 | 365 | same |

Every one has blend on, `(SA, 1)` or `(SA, 1-SA)`, depth test on, depth mask 0, depth function
`0x203`, alpha test off: the flush's state. They include quads 1.1 units from the eye (below),
which a distance mode would have sent elsewhere.

Why 90 and not higher: the handler's dead argument is read statically, and both live captures
agree with it, but no frame was taken on a real console.

## The near-camera law (billboards, render modes 1 to 4)

The vertices are built before the GPU sees them, in **view space**, by the `PartRender` SPU job
(image `0x0080bb80`; [engine-trail.md](engine-trail.md), "The class-6 draw law"). Its routine for
modes 1 to 4 is at image offset `0x3bb8` (called from `0x5884`). Per live particle, with `z` the
view-space z of its centre (negative in front of the eye) and `size` its float `+0x70`:

```text
3c40..3c50                     view-space position (the matrix at local 0x5b00..0x5b30)
3c58  fcgt  $2,$27,$3          z > -1.0
3c5c  brz   $2,0x3ec0          not nearer than 1: go on. Else fall through to 0x3c60,
                               which writes four zero vertices (a degenerate quad)
3ec0  ilhu  $24,0xc080         -4.0
3ec4  shlqbyi $20,$8,0         fade = 1.0
3ec8  fcgt  $23,$27,$24        z > -4.0
3ecc  brnz  $23,0x4c68
4c70  fa    $30,$27,$8         z + 1.0
4c78  ilhu  $29,0xbeaa ; iohl $29,0xaaaa      -1/3
4c84  fm    $20,$30,$29        fade = (z + 1) * -1/3
4c88  fceq  $8,$20,$25         fade == 0: the degenerate quad (0x3c60), else 0x3ed0
3ed4  lqd   $55,112($10)       size, particle +0x70
3ee0..3f24                     size / z (frest, one Newton step)
3efc  lqr   $8,0x5b40          -(emitter +0xc80), the job's parameter +0x48
3f34  fcgt  $31,$8,$46         -cap > size / z
3f54  brnz  $31,0x4c94         the capped path
3fa0  cuflt $57,$58,0          the colour word's first byte (alpha)
3fd0  fm    $8,$57,$20         alpha * fade
```

As a law, with `depth = -z`:

1. **Cull.** `depth < 1`: no quad (four zero vertices, so the draw's vertex count does not change).
2. **Fade.** `depth < 4`: `alpha *= (depth - 1) / 3`. At depth 4 and beyond the alpha is the
   particle's own.
3. **Cap.** `size = min(size, depth * cap)`, `cap` the emitter's `+0xc80`. The quad's half
   height is `size` and its half width `aspect * size` (`+0x4c8`), so the cap bounds the angle a
   particle may fill: at `cap` 0.5 its height is never more than its distance.

The streak modes 5 and 6 have their own rule (both ends at or beyond `-1`, engine-trail.md) and
read neither the fade nor `+0xc80`.

**Omega agrees, on another ISA.** `Particle_PushQuadBatch` (patch 1.07 `0x0175f540`, base
`0x01714000`, [heat-haze.md](../ps4-omega-eu/heat-haze.md)) is the same routine on the CPU:
`if (z <= -1.0)` around the whole quad; `fade = (-4.0 < z) ? (z + 1.0) * -0.33333334 : 1.0`
multiplied into the alpha byte (`particle + 0x77`); `size = (size / z < -cap) ? z * -cap : size`
with `cap = emitter + 0xc80`; half width `emitter + 0x4c8` times `size`. Omega has one extra
term behind a flag at `instance + 0x71`, a sphere fade around a fixed point (radius 290), not
in HD's routine as read.

**Live: the vertex buffers carry all three.** The 1080p frames' particle vertices are 16 bytes:
position as three half floats, four colour bytes (alpha last), two half-float texture
coordinates. Positions are view space. Over the five frames:

- No quad's centre is nearer than `z = -1.14`. The emitters that reach the eye carry zero
  quads: 5 of 8 in one draw of `s01`, 9 of 20 in one draw of `s07`.
- The quads nearest the eye sit **on** their cap, and the same emitter's farther quads keep
  their own size:

  | Frame | `z` | Half size | `half / depth` | Alpha byte |
  | --- | --- | --- | --- | --- |
  | `s01` | -1.144 | 1.144 | 1.0000 | 3 |
  | `s01` | -1.340 | 1.340 | 1.0000 | 6 |
  | `s01` | -2.438 | 1.643 | 0.674 | 45 |
  | `s07` | -1.778 | 0.887 | 0.499 | 48 |
  | `s07` | -2.963 | 1.480 | 0.499 | 137 |
  | `s07` | -5.832 | 2.912 | 0.499 | 193 |
  | `s07` | -7.883 | 2.891 | 0.367 | 222 |
  | `s07` | -10.266 | 2.885 | 0.281 | 193 |
  | `s07` | -1.526 | 0.297 | 0.195 (long side 0.200) | 24 |
  | `s07` | -8.500 | 0.282 | 0.033 | 145 |

  Three caps are seen pinned: 1.0, 0.5 and 0.2, the three common values the disc authors.
- The alpha of the near quads is what the fade predicts from their neighbours: the `s07`
  emitter's unfaded quads read 158 to 225, and its quads at depth 1.78 and 2.96 read 48 and 137,
  which is 185 and 209 divided by the fade. This is consistent with the law, not a separate
  proof of it: a particle's own alpha moves with its age.

The frames' explosions were 70 to 350 units away (the Rockets flew on down the straight), so
the near quads above are the player's own effects, not a blast's. The law is per particle and
does not know which effect it is in.

Why these scores: the cap is read in two executables on two ISAs and seen exact to four digits
on live vertices (94). The cull is read in both and seen as zero quads at the right depths (92).
The fade is read in both, and the live alphas fit it without proving it (88).

## What the disc authors for the cap

`+0xc80` over the 88 billboard emitters (modes 1 to 4) of `DATA02.PSARC`'s effect files: 0.2 on
47, 0.3 on 1, 0.5 on 19, 1.0 on 21. `WO_ROCKET_EXPLO`, the effect of a Rocket striking a craft:

| Emitter | Mode, blend | Cap `+0xc80` | Size channel range |
| --- | --- | --- | --- |
| `WO_ROCKET_EXPLO` (root, smoke) | 2, over | 0.5 | 4.0 to 7.5 |
| `SMOKE_BLUCLD` | 2, over | 1.0 | 4.0 to 7.5 |
| `trail` (child of `FIRE_TRAILS_ONEQUAD`) | 2, over | 0.5 | 3.5 to 7.5 |
| `FIREBALL_FIJETS` | 2, additive | 0.5 | 3.5 to 7.5 |
| `FIREBALL_FIJETS_SMOKE_TRAIL` | 2, over | 0.5 | 7.0 to 11.0 |
| `FIREBALL_TUMULT` | 2, additive | 0.5 | 4.5 to 9.5 |
| `FIREBALL_TUMULT_SMOKE_TRAIL` | 2, over | 0.5 | 7.0 to 11.0 |
| `SHIP_DEBRIS` | 2, over | 1.0 | 0.2 to 0.6 |
| `MOLTEN_DEBRIS` | 2, over | 0.2 | 0 to 0.3 |
| `FIRESPIKES`, `FIRE_TRAILS_ONEQUAD`, `SPARKS` | 5, additive | streaks: the cap is not read | |

A smoke particle of size 7 to 11 with cap 0.5 is capped whenever it is nearer than 14 to 22
units. That is every frame of a blast the camera flies into, which is the case ours draws as a
white screen ([particle-fill.md](../../../rendering/particle-fill.md) has the pictures and the
fill it costs).

## The atlas frame

How HD's particles walk a flipbook. Read in `EBOOT.elf` and checked against the original's own
quads; confidence **90**.

**Per instance** (`FUN_00281060`, one of three draw-state prologues, `0x00281060`, `0x00286698`,
`0x00288d98`; unnamed):

- the advance is on (`state+0xc40`) when the frame-rate channel's `hi` (`+0x788`) is above zero
  (`*0x008b2980 = 0.0`) and the instance's frame count (`instance+0x174`) is not 1;
- `state+0xc44 = count - 0.01` (`*0x008b2978`).

`Psys_BuildUpdateContext` (`0x00281ec0`) copies them to `ctx+0x6b2` and `ctx+0x6b8`, the
emitter's flags to `ctx+0x6b4`, and the record's bytes `+0xc8f` and `+0xc94` to `ctx+0x6c0`
and `+0x6c1`.

**Per particle** (`Psys_UpdateParticles`, `0x0028d0f0`-`0x0028d1bc`), `k = *0x008b2a78 = 0.01`
and `top = ctx+0x6b8`:

```text
flags & 0x40 ? frame = age * (top - k)            age clamped to *0x008b2a84 = 1.0
             : frame += rate * ctx+0x6bc
if (lo != 0 || hi != 0) && frame > hi: frame = lo     lo = +0xc8f, hi = +0xc94
if frame >= top: frame -= top
drawn = floor(frame)
```

**At spawn** (`Psys_InitParticle`, `0x00289890`): under flag `0x4000000` the frame is a random
integer in `[+0xc8d, +0xc8e]`, or in `[0, count - 1]` when both are zero; without the flag it is
`+0xc8d`.

The step is Pulse's (`particle-system.md`, "The atlas frame advances", where `top` is also
`frames - 0.01`). The hold and the spawn range are HD's addition: on `DATA00` and `DATA02` a
hold names one frame twice (`hd = lo`) and every range ends inside its grid
(`crates/pob/tests/pob_ground_truth.rs`).

**Runtime** (RPCS3, the command ring's particle quads, read per cell by
`scripts/hd-particle-uv.py`; films `emp1`, `cannon1`, `plasma1`, `plasma2`, `missile2`,
`quake1`, `quake2`, `rocket1` under `data/reference/hd-capture/particle-uv/`):

- **Rate**, per time: cells per 1/30 s are twice the authored rate (`debris006` +1 on 567 of
  567, `Debris_DetonatorMines` +2 on 2442 of 2442, `PlasmaRings` +2 on 85 of 85). A random
  channel does not advance (`molten_debris` +0 on 658 of 658).
- **Hold:** on `explosion_fijets_8x8_grey_gaseous` (`Explosion_Core`, `explosion_fijets`) a
  particle on the last cell stayed there 107 times and never wrapped; looping sheets wrapped
  (`Debris_LightBarrier` 374 of 374).
- **Spawn range:** no quad of that texture read a cell below 23 over 267 quads, where the one
  emitter that binds it (`Explosion_Core`) authors `[20, 63]`.
- **Over life:** the ages in [particle-fill.md](../../../rendering/particle-fill.md#hd-advances-its-flipbooks-from-the-authored-top-left).

**Not exercised:** Pulse stops the advance of an emitter with a periodic keyframed channel; no
HD flipbook with a frame rate authors one, so what HD does there was not read.

**Wired** for HD and Fury (`oag_title::Effects::flipbooks`, measured):
`oag_fx::psys::frames` and `EmitterSpec::random_frame`.

**2048: the update differs, the spawn frame applies.** `Psys_UpdateParticle` (`0x812ab114`,
v1.04): over life `age * (count - 0.01)` under bit `4` of `record+0x884`; at a rate
`frame += rate * dt` and `frame -= count` once it reaches `count`, not `count - 0.01`; no hold
step: **checked, differs**. The particle init (`0x812aa6ac`) draws the spawn frame as HD does
from `record+0xc8d`/`+0xc8e`: **checked, applies, not wired**. 2048's `trail` authors HD's
values at HD's offsets (spawn `0, 3`, hold `31, 31`). Confidence 70 (static; the runtime
record's channel offsets are assumed from the code's shape).

**Omega** carries two readers. `FUN_01724c00` (v1.07) is HD's step, hold and wrap, with the
hold bytes at `record+0xc97`/`+0xc9c`: **checked, applies, not wired** (its prologue is
unread). `FUN_01728ae0` is 2048's (wraps at `count`, no hold): **checked, differs**. Which
reader serves which effect directory was not read. The particle init (`FUN_01722c10`) draws
the spawn frame from `record+0xc95`/`+0xc96`: **checked, applies, not wired**.

**Omega's record is HD's with 8 bytes added at `+0xc84`.** In the patch's `data05.psarc`, in
both particle directories, `trail` of `WO_ROCKET_EXPLO` reads `1.0` at `+0xc84`, `0` at
`+0xc88`, its size bounds `0.2, 1` at `+0xc8c`/`+0xc90`, its spawn range `0, 3` at `+0xc95` and
its hold `31, 31` at `+0xc97`/`+0xc9c`. `oag_pob::Emitter::{size_span, frame_range}` read HD's
offsets, so on an Omega file they return other words: wiring either for Omega starts there.
Nothing plays them today (`oag_title::Effects` leaves both off for Omega). Confidence 85.

## The per-particle size multiplier

`Psys_InitParticle` (`0x00289890`) stores a multiplier at particle `+0x10`, and the drawn size
is the size channel times it (the init's store to `+0x70`, and `0x0028d058`-`0x0028d07c` in the
update, which also multiplies by `ctx+0x6ac`). With `lo = record+0xc84`, `hi = record+0xc88`:

```text
flags & 0x20000000 && flags & 0x4000 ? m = (1.0 - a) * (hi - lo) + lo       1.0 = *0x008b296c
                                       a = flags & 0x10000 ? instance+0x14c : parent particle +0x90
                                     : m = rand(lo, hi)
```

**`instance+0x14c` is the emitter's run fraction.** `FUN_00287168` loads `instance+0x148` with
the duration (`record+0x24`). The instance update (`FUN_00288d98`), for an emitter without the
looping flag, stores `1 - instance+0x148 / record+0x24` at `+0x14c`
(`0x00288fa0`-`0x00288fb8`) before it emits, and takes the instance's `dt` off `+0x148` after
(`0x00289804`-`0x00289808`). A looping emitter counts `+0x148` up instead and stores
`+0x148 * *(r2+0x54e4)` (`0x0028987c`-`0x00289888`); that form was not followed. The parent
particle's `+0x90` is its age fraction (`+0x94 = 1 / life` added per update). Confidence 90.

So on the run arm a particle born at the start of the run takes `hi` and one born at the end
takes `lo`.

**Runtime** (one film, `rocket-craft-hit/far3`, partly repeated on `hit2`). `trail` (flags
`0x2401420c`, duration 25, lifetime `201 +/- 49`) authors `lo = 0.2`, `hi = 1`, and its frame is
its spawn frame in `0..3` plus one per tick, so the smallest quad of a sheet cell dates from a
known birth. `(3.5 + 4 * age / 201) * (1 - 0.8 * born / 25)` matches the smallest half size per
cell with rms 0.08, 0.06 and 0.12 at 0.1, 0.233 and 0.4 s; the size channel alone is 0.45, 1.01
and 1.67 out. On `hit2` the same law is 0.14 out at 0.1 s against 0.52, and looser later (1.17
against 2.10 at 0.233 s); `far1` and `hit1` gave no rows. Quads above the minimum are the other
blasts of the same frame.

**Wired** for HD and Fury, the run arm of a non-looping emitter only
(`oag_title::Effects::size_ramps`, measured; `oag_fx::psys`, `Burst::size`). On the disc that
arm is `WO_ROCKET_EXPLO/trail` (`0.2, 1`), `WO_SHIP_EXPLOSION/FIRE_ANIMS_TRAILS` (`0.3, 1`) and
`WO_NITRO_SHIP_DEATH/Trail_Ribbon` (`0, 1`); only the first has a film.

**Open arms** (58 of 241 emitters by file and name, `DATA00`, `02`, `03`, `06`, author bounds
other than `(1, 1)`; ours plays three):

- **Parent age** (both ramp flags, without `0x10000`): `TRAIL_SMOKE` in the wall blast and the two
  ship explosion files, `plasma_goo_trail`, `SPARK_TRAILS` of the Quake detonator.
- **Random** (without both ramp flags, `0.7, 1.3` the commonest): the rest. The original draws a
  number per particle; wiring it takes an extra draw from the effect's generator, only where
  `lo != hi`, or every later particle changes.

**2048: checked, applies, not wired.** `0x812aa6ac` carries the same two flags, both arms and
the same offsets (`record+0xc84`/`+0xc88`; the instance fraction at `instance+0x134`, the
parent's at `+0x88`), and its `trail` authors `0.2, 1`. What its instance fraction holds was
not read. **Omega: checked, applies, not wired.** `FUN_01722c10` carries the same, with the
bounds at `+0xc8c`/`+0xc90` (see "The atlas frame") and the parent fraction at `+0x98`.
Confidence 70 (static decompile, instance writers unread).

**Open overlap:** `oag_pob` reads `+0xc84` as `colour_scale` on render mode 5 and as
`distort_strength` on blend class 8. HD's init reads the same word as the multiplier's lower
bound (`SPARKS` of the craft blast: `0.7, 1.3`), and Omega's layout has a word for each. Whether
HD's renderer also reads `+0xc84` as a colour scale is open.

## Names

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x006af030` | function | `ParticleRenderNode_Enqueue` | 85 |
| `0x006af000` | function | `ParticleRenderNode_Draw` | 82 |
| `0x00869a80` | data | `ParticleRenderNode_Vtable` | 80 |
| `0x003af240` | function | `Render_BindSceneTargets` | 78 |
| `0x003de2b8` | function | `Render_GetSceneColourTarget` | 85 |
| `0x003de2f8` | function | `Render_GetSceneDepthTarget` | 72 |
| `0x00c81b8c` | data | `g_SceneDepthTargets` | 72 |
| `0x008c2670` | data | `Psys_ResolutionModeDistances` | 65 |

- `ParticleRenderNode_Enqueue`: 85. Five instructions of key and three of queue, the idiom
  renderer.md reads at five other sites. The layer is this page's.
- `ParticleRenderNode_Draw`: 82. One call. Its place in the vtable is slot `+0x1c`, the one the
  queue dispatches.
- `ParticleRenderNode_Vtable`: 80. Stored by both manager constructors and both destructors
  (`0x006af0e0`, `0x006af098`). The class's own name was not found.
- `Render_BindSceneTargets`: 78. Read whole to the viewport call; the three `0x005a7728` calls on
  the returned slots and the split-screen branches were read for shape only. Its one known
  caller is the particle flush, so the name is from what it binds.
- `Render_GetSceneColourTarget`: 85. Returns `&g_SceneColourTargets[g_SceneWidthLevel]`, both
  named on [scene-width-throttle.md](scene-width-throttle.md).
- `Render_GetSceneDepthTarget`, `g_SceneDepthTargets`: 72. The same getter over the array at
  `+0x10c`, passed as `Rsx_SetRenderTargets`' depth argument. One call site. This names one of
  the five arrays that page left unseparated.
- `Psys_ResolutionModeDistances`: 65. Two floats compared against an instance's view depth to
  pick a value nothing downstream uses, so what the modes meant is a reading of the code's shape.

`PartRender_FlushDraws` and `PartRender_QueueDraw` (engine-trail.md, 70 each) are confirmed by
the live frames' state on every point this page lists. Their rows are left as that page set them.

## Wired

2026-11-25. `oag_fx::psys`'s `near` module runs the three rules on every billboard of an effect
the title's data puts under the law (`oag_title::Effects::near_law`: HD, 2048 and Omega,
**measured**; Pulse and Pure none). The cap is the emitter's own `+0xc80`, read by `oag-pob`
([pob.md](../../../formats/pob.md)). Which parts of ours it reaches and does not:

- Billboards (render modes 1 to 4), the heat-haze quads and the quads a mod's shader draws.
  Streaks (modes 5 and 6) are left as they are.
- The collision sparks, the race's effect stage and the scenery stage. **Not** the weather field
  and the lens glass: no title with the law authors weather.
- The effects loaded from the title's own files. A mod's parametric effect, and a burst lent to a
  title without the law (the trail hit), carry none.
- Sprite templates cull and fade; a template record is not an emitter and has no `+0xc80`, so it
  has no cap. That the original's template draw goes through the same routine is not read.
- **Chosen, not measured:** the depth is taken where the particle is drawn between two simulation
  ticks, since ours draws at display rate and the originals at one position a frame.

## Cross-title

- **Omega: ported.** Its `Particle_PushQuadBatch` carries the same cull, fade and cap (above),
  and its billboard emitters author `+0xc80`
  (`crates/fx/tests/psys_omega_ground_truth.rs`). Omega's one extra term, the sphere fade behind
  `instance + 0x71` (radius 290), is not wired: what sets the flag is unread. Its draw passes
  differ from HD's (`Particle_DrawBatchesForPass`: passes 8, 9, 10); whether one of them is a
  reduced target was not read.
- **2048: ported.** Both its quad builders carry the law
  ([particle-near-law.md](../vita-2048-eu-v104/particle-near-law.md)).
- **Pulse, Pure (PSP, PS2): not checked.** Their records end at `0x9b8`, before `+0xc80`, so the
  cap as a field does not exist there. Whether their builder has a near rule is open.

## Reproducing

```sh
# the SPU routine
scripts/ps3-spu-disasm.py 0x0080bb80 0x5ad0 | less          # offsets 0x3bb8..0x4030, 0x4c68..0x4c94
# the flush's target handler and its two getters
llvm-objdump -d --start-address=0x3af240 --stop-address=0x3af3a0 <EBOOT.elf>
llvm-objdump -d --start-address=0x3de2b8 --stop-address=0x3de318 <EBOOT.elf>
# a frame's particle draws: QUADS (primitive 8) on the psys program, with target, clip and blend
python3 scripts/rsx-pass-events.py <dump-dir> <stem> | grep 'prim 8 '
```

The live frames: restore `hd-racebox-talons-venom-weapons-grid-rpcs3-20574` with the GDB stub off
(`scripts/emu-restore-state.sh <lane> <state> --no-gdb`; with the stub on, this state's load
stalled past the 240 s limit on a host at load 60). Memory goes through the `serve` socket
(`RemoteMem`), the pause is `SIGSTOP` on the emulator's own pid. Start the race, write Rocket
(`0`) to the pickup slot (`*(ship + 0x5edc) + 0x204`, twice), hold thrust and fire. Per paused
frame, dump the command ring (`0x40000000`, `0x20000` bytes, then the pages its jumps reach) and,
for each `QUADS` draw, attribute 0's span: offset register `0x1680`, format `0x1740` (stride 16),
first and count from `0x1814`. `scripts/rpcs3_draw_hook.py` dumps indexed draws only, so the
particle spans (array draws) need this extra step.

## Open

- The picture at a near CraftBlast does not match yet, for reasons outside the law
  ([particle-fill.md](../../../rendering/particle-fill.md), "Against the original at a near
  blast"). The law's cull and cap were read there on the blast's own quads (2026-11-26, three
  hits: no quad nearer than 1.037, size pinned at 0.5, 1.0, 0.3 and 0.2 times depth).
- ~~Single-quad additive draws at half height = 2 x depth, full screen, in the frames of a Rocket
  craft hit up to 0.23 s: which emitter or node draws them.~~ The blast's sprite templates
  (2026-11-26, [rocket-craft-blast.md](rocket-craft-blast.md#the-screen-wash-the-blasts-sprite-templates));
  where the template cap of 2.0 is read is open. They sit 2 to 12 units from the
  blast's spawn point and do not travel with it ([rocket-craft-blast.md](rocket-craft-blast.md)). 2.0 is the cap authored by the mode 5
  emitters of `WO_ROCKET_EXPLO` (`FIRESPIKES`, `FIRE_TRAILS_ONEQUAD`), whose sizes (2.1 to 9.2)
  cannot reach the sizes seen (up to 47).
- Where a `.pob` effect's streaks are drawn: the dumped `QUADS` lists held only screen-facing
  quads.
- A real PS3's frame at a near blast: how far the width level moves, and whether it holds 60.
- What modes 1 and 2 of `+0xc8c` did where they were live, if any build has them.
- Omega's passes 8 to 10 by target size; the PSP builds' quad builders.
- `0x1fec`, one enable the flush sets to 1, was not identified.
