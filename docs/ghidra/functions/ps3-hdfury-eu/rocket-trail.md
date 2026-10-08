# The Rocket's smoke ribbon: `RibbonEffects` pool 4, read live

Read 2026-10-08 (`hd-rocket-trail`), three ways: the executable in Ghidra
(`/hdfury/EBOOT-ps3-hdfury-eu.elf`), the AltiVec bodies run in a scratch PPC
interpreter where the decompiler truncates, and the running game's own memory
over RPCS3's GDB stub after a Rocket shot (five pauses, 0.1 to 2.2 s after the
fire press). Every law on this page was first read statically and then matched
against the live node chain and the live vertex buffer.

**Correction to the 2026-10-07 reading** on
[trail-ribbon.md](../../../rendering/trail-ribbon.md): the Rocket's ribbon is
**not** built by the `WakeTrail` SPU job at `0x00ad81f0`. The ribbon-model
name table `0x00920fc0` is used by `RibbonEffects_Construct` (`0x002a7560`), a
PPU-side pool manager; `WakeTrail`'s registration (`0x002a8678`) only shares
the TOC block. The vertices are written on the PPU by
`RibbonBuilder_WriteVertexPair` (`0x002a4660`). Confidence 90.

`oag_fx::rocket_smoke` implements what this page measures and
`oag_raceplay::rocket_smoke` owns one ribbon per rocket; what is chosen
rather than measured there (vertex RGB white, jitter distribution, push at
the rocket's origin) is labelled in that module.

## The manager

`RibbonEffects_Construct` (`0x002a7560`, called from 24 circuit-scene
constructors) builds six pools and loads the four ribbon models of
`0x00920fc0` - `rockettrail_triangle`, `waketrail_triangle`,
`leachbeam_triangle`, `RocketTrail_Shadow_triangle` - into `this + 0x40..0x4c`.
The instance lives at `RaceManager + 0x2dcc` (`RaceManager` = `*0x0095ae78`).
Each pool is `{count, elements, alive flags}`; each element is initialised
from a static config block:

| pool | elements | config | texture named by the config | user |
| --- | --- | --- | --- | --- |
| `+0x50` | 6 x 0x60 | `0x00920fd0` | `ThickLine_Red.gtf` | not read |
| `+0x5c` | 20 x 0x60 | `0x00921050` | `ThickLine_bighalo.gtf` | not read |
| `+0x68` | 6 x 0x60 | `0x00921010` | `ThickLine_Red.gtf` | not read |
| `+0x74` | 18 x 0x70 | `0x00921090` | `smoke_trails_frame1_ALPHA.gtf` | Missile (acquire `0x002ab9d0` is called from `Missile.cpp`'s functions `0x0011d288`, and from `0x001535f8`/`0x00154a78`) |
| `+0x80` | **9 x 0x190** | **`0x00766c4c`** | (model) | **Rocket** |
| `+0x8c` | 1 x 0x60 | `0x009210d0` | `ThickLine.gtf` | not read |

`RibbonEffects_UpdatePools` (`0x002a5dc0`, vtable slot 3) ticks every alive
element through its own vtable slot 1 and frees it when that returns 0.
`RibbonEffects_Render` (`0x002a6478`, slot 7) draws pools 0, 2, 1, 5 under
`blend(0x302, 1)` - `SRC_ALPHA, ONE` - and then switches to `blend(0x302,
0x303)` - `SRC_ALPHA, ONE_MINUS_SRC_ALPHA` - for pools 3 and 4. The last
argument pair of that call is `(0, 0x303)`, read as the alpha channel's own
`ZERO, ONE_MINUS_SRC_ALPHA` (the four-argument order is read off the call, not
off the RSX wrapper; confidence 60).

**Pool 4 is the Rocket's, confidence 92.** Three independent discriminators:
`Rocket_Update` (`0x00123fb0`) calls `RocketTrail_PushSample` (`0x002aa820`)
on the ribbon pointer at `rocket + 0x124`; the element's two builders draw
model indices 0 and 3 of `0x00920fc0`, `rockettrail_triangle` and
`RocketTrail_Shadow_triangle`; and the live dump after a Rocket shot found
three fresh pool-4 elements, one per rocket, whose node chains follow the
three rockets' paths.

## The element and its law

`RocketTrail_Construct` (`0x002aa770`, vtable `0x00869dc0`) derives from a
generic ribbon (`Ribbon_Construct` `0x002ed6d8`). Its init `RocketTrail_Init`
(`0x002aaae0`) first runs `Ribbon_Init` (`0x002edaa8`), which copies the
config's first 0x20 bytes to `+0x144` and allocates the main builder at
`+0x10`; then it allocates a second builder (`+0x18c`, model index 3) for the
shadow and copies the same 0x20 bytes to `+0x16c` as the shadow's config.

The config at `0x00766c4c`:

| offset | value | what it is | how it is known |
| --- | ---: | --- | --- |
| `+0x00` | 1.85 | node lifetime, seconds | `Ribbon_PushSample` writes it to a new node's `+0x64`; live nodes are born at 1.85 - dt |
| `+0x04` | 0.03 | `u` per world unit of arc length | `Ribbon_AgeSamples` multiplies the node-to-node distance by it; live `u` steps are `0.03 x` the dumped spacing |
| `+0x08` | 0.4 | half-width at birth | live: newest node 0.4000, its vertex pair 0.8 apart |
| `+0x0c` | 2.0 | half-width at death | the lerp's far end |
| `+0x10` | 140 | node capacity of the builder | `RibbonBuilder_Alloc(140, 3)` |
| `+0x14` | 3 | strips (fins) per node | the same call; live buffers carry 6 vertices per node |

`Ribbon_Init` also stores `+0x15c = width_birth - width_death` (-1.6) and
`+0x160 = 1 / lifetime`.

### Pushing a node: `Ribbon_PushSample` (`0x002edc38`)

One node per call, from a global pool of 4,000 0x80-byte nodes
(`RibbonNode_Alloc` `0x002ebde8`), appended at the newest end of a doubly
linked chain (`RibbonNode_Link` `0x002ebce8`):

- `+0x00..0x3f`: the rocket's basis rows and position, copied from the matrix
  the caller passes;
- the position then gets `rand() * 0.18` added to each of x, y, z, with
  `rand() = (Libc_Rand() - 2^29) * 2^-29` (`0x002a3ba8`). `Libc_Rand`'s own
  range is not read, so whether the jitter is +-0.18 or skewed is open;
- `+0x40` = 0 (the strip angle offset), `+0x48` = the caller's colour word,
  `+0x4c` = 0 (visible);
- `+0x64` = lifetime, `+0x68` = 1.0, `+0x6c` = the sentinel `0x00800000`
  (`u` not yet assigned).

`Rocket_Update` pushes **once per update**, every update the rocket lives -
live, consecutive nodes are 1/60 s apart in life and about 3.5-3.8 world units
apart in space. The colour word comes from `RibbonEffects_SampleLighting`
(`0x002a4280`): it looks the rocket's position up in a spatial tree
(`0x003c2598`/`0x003c2488` over `0x006d00b0`, data at `*0x00d43cc4`) and packs
an RGB with alpha 0xff. **That lighting volume is unread**; live values on the
dumped circuit run from `0xbaffff` near the grid to `0x2b5d7b` in the dark.

### Ageing: `Ribbon_AgeSamples` (`0x002ed7d8`), confidence 95

Walks the chain from the oldest node to the newest, for each node:

```text
if life <= 0:         unlink and free the node        (life read before the decrement)
t     = life * (1 / lifetime)                        (life before the decrement)
life -= dt
half_width = t * (width_birth - width_death) + width_death   ->  0.4 at birth, 2.0 at death
alpha = ramp[ trunc((1 - t) * 255) ]                 (unless +0x4c's top byte is set: alpha 0)
colour = (+0x48 & 0xffffff00) | alpha
arc  += |pos - previous pos| * 0.03                   (from the second node on)
if u == sentinel: u = arc                            (u is frozen at the first update)
```

After the walk the oldest and the newest node have their alpha byte cleared,
so the ribbon's two ends are always transparent. The element is alive while
any node survives (`RocketTrail_Update` `0x002aaa80` returns the walk's node
count test), so **a ribbon outlives its rocket by up to 1.85 s**.

Live check, one rocket at the second pause (`life` after the decrement):

| life | dumped half-width | law | dumped alpha | law |
| ---: | ---: | ---: | ---: | ---: |
| 1.8333 (newest) | 0.4000 | 0.4000 | `0x00` | cleared |
| 1.8167 | 0.4143 | 0.4144 | `0xf1` | `ramp[2]` = 241 |
| 1.7494 | 0.4726 | 0.4726 | `0xdf` | `ramp[11]` = 223 |
| 1.6832 | 0.5298 | 0.5298 | `0xc0` | `ramp[20]` = 192 |
| 1.3611 (oldest) | 0.8090 | 0.8084 | `0x00` | cleared |

The last row's 0.0006 is the original's variable timestep: the half-width
implies the frame's own `dt`, which across all dumped nodes runs 0.0158 to
0.0182 s (median 0.01671). **The alpha matches on all 695 interior live nodes
of the five pauses**, ten elements in all, with none off by even one step.

### The opacity ramp is a file on the disc, read two bytes late

`RibbonEffects_LoadOpacityRamp` (`0x002c38a8`, called last in
`RibbonEffects_Construct`) opens
`Data/RibbonEffects/textures/smoke_trails_opacity_ramp.tga` (300 bytes in
`DATA02`, a 256x1 8-bit greyscale TGA), skips **0x14** bytes and reads 256
bytes into the table at `0x00ad7cf0`, plus a float copy `byte / 255`.
`OpacityRamp_Lookup` (`0x002c3788`) returns `table[(int)(x * 255.0)]`.

The TGA header is 18 bytes, so the engine's table starts at the image's
**third** pixel and its last two entries are the file's first two footer bytes
(both 0). Entry 0 is 250, entry 255 is 0. The live alphas above match the
table read at byte offset 20 and do not match it at offset 18. Confidence 95.

### The shadow ribbon draws nothing

`RocketTrail_Init` copies the raw config for the shadow and adjusts only its
two widths (`+0x174 = 0.4 + 0.5`, `+0x178 = 2.0 + 0.5`); it never computes the
shadow's `width delta` and `1 / lifetime`, which stay the config's own zero
bytes at `+0x18`/`+0x1c`. So for every shadow node `t = 0`, the half-width is
2.5, and the alpha is `ramp[255]` = 0. Live: **every shadow node of every
element in all five pauses has alpha 0**, and the shadow material's fragment
program multiplies by that vertex alpha
(`hd_rockettrail_shadow.rcsmaterial`, `MUL H0.w, tex.a, f[TC3].z`, with the
vertex program's `MOV o[TC3].z, v[3].w`). A shipped bug; nothing to draw.
Confidence 90.

## The geometry

`RibbonBuilder_AddSample` (`0x002a4c20`) runs once per node and strip `k`,
calling `RibbonBuilder_StripSide` (`0x002a47e8`) with angle parameter
`k / strips + node+0x40`, which multiplies it by pi and evaluates sine and
cosine by polynomial. Run in the scratch interpreter on an identity and a
yawed basis (rows `right`, `up`, `forward`):

```text
side(k)   = cos(k * 60deg) * up - sin(k * 60deg) * right
normal(k) = cos(k * 60deg) * right + sin(k * 60deg) * up
```

`RibbonBuilder_WriteVertexPair` (`0x002a4660`) then writes two vertices per
strip: `centre + side * half_width` with `uv = (u, 1)` and
`centre - side * half_width` with `uv = (u, 0)`, both carrying the normal and
the node's colour word. Stride 0x24 (position, normal, `Uv1`, `VertexColour1`).
`RibbonBuilder_Flush` (`0x002a51e8`) builds a triangle list,
`(a, a+1, b), (a+1, b, b+1)` per strip per segment with `b = a + 2 * strips`,
and draws it through model `RaceManager+0x2dcc -> +0x40 + index * 4`.

`Ribbon_Render` (`0x002ed740`) feeds nodes newest first and skips any node
whose life is not above 0. Live vertex buffers: newest node first, 6 vertices
per node, each pair 2 x half-width apart, fin 0's normal along the rocket's
`right` row. Confidence 92.

## The material, and what the draw patches

`hd_rockettrail.rcsmaterial` names `Texture1 = smoke_trails_frame1_alpha.gtf`
(unit 1) and `Texture2 = smoke_trails_frame2_alpha.gtf` (unit 0). Vertex
program: `TC0 = colour.rgb`, `TC3 = (uv, colour.a)`, `TC1 = eye - position`,
`TC2 = normal`. Fragment program, 29 instructions
(`ps3-microcode.py fp-file`):

```text
facing = saturate(max(min(dot(norm(TC2), norm(TC1)), 0.32), 0) * 3.12433)
tex    = lerp(Texture1, Texture2, saturate(time * 0.3))
rgb    = tex.rgb * TC0.rgb * constantAmbientColour
alpha  = saturate(14 * tex.a * facing * TC3.z) * saturate(window_z * 0.75)
```

So each fin is single-sided: it shows only from its normal's side, fully from
`dot >= 0.32`.

`RibbonBuilder_Flush` writes the engine parameter table directly before the
draw: entry 0 (`time`) from the frame singleton `*0x00936fd4 + 0xc4`, the
monotonic clock in seconds; entry 10 (`constantAmbientColour`, `+0x158`)
**`(0.8, 0.8, 0.8, 1)`**, its own constant; entry 7 (`fogColour`, `+0xf8`)
`(1, 1, 1, 0)`. So the circuit's ambient does not reach the smoke, and in a
race `time * 0.3` is long past 1, so the picture is `Texture2` alone.
Confidence 80 (the slot table is
[renderer.md](renderer.md)'s "The engine's own parameter table").

## Named here

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x002a7560` | `RibbonEffects_Construct` | 88 |
| `0x002a5dc0` | `RibbonEffects_UpdatePools` | 80 |
| `0x002a6478` | `RibbonEffects_Render` | 80 |
| `0x002a4280` | `RibbonEffects_SampleLighting` | 70 |
| `0x002c38a8` | `RibbonEffects_LoadOpacityRamp` | 92 |
| `0x002c3788` | `OpacityRamp_Lookup` | 90 |
| `0x002aa770` | `RocketTrail_Construct` | 80 |
| `0x002aaae0` | `RocketTrail_Init` | 85 |
| `0x002aa820` | `RocketTrail_PushSample` | 90 |
| `0x002aaa80` | `RocketTrail_Update` | 85 |
| `0x002aa9d8` | `RocketTrail_Render` | 80 |
| `0x002ed6d8` | `Ribbon_Construct` | 75 |
| `0x002edaa8` | `Ribbon_Init` | 88 |
| `0x002edc38` | `Ribbon_PushSample` | 90 |
| `0x002ed7d8` | `Ribbon_AgeSamples` | 95 |
| `0x002ed740` | `Ribbon_Render` | 82 |
| `0x002a4d58` | `RibbonBuilder_Alloc` | 85 |
| `0x002a45f8` | `RibbonBuilder_Reset` | 75 |
| `0x002a4c20` | `RibbonBuilder_AddSample` | 88 |
| `0x002a47e8` | `RibbonBuilder_StripSide` | 90 |
| `0x002a4660` | `RibbonBuilder_WriteVertexPair` | 92 |
| `0x002a51e8` | `RibbonBuilder_Flush` | 80 |
| `0x002ebde8` | `RibbonNode_Alloc` | 85 |
| `0x002ebce8` | `RibbonNode_Link` | 75 |
| `0x002ebd48` | `RibbonNode_Unlink` | 75 |
| `0x00123fb0` | `Rocket_Update` | 70 |

`Rocket_Update` sits between `PulseBullet.cpp`'s and `Rocket.cpp`'s
constructors, reads the ribbon pointer at `rocket + 0x124`, and also calls the
point light `0x006778c8` (at `0x001246cc`) - the launch light, unread here.

## Open

- The lighting volume behind `RibbonEffects_SampleLighting`: what file fills
  `*0x00d43cc4`, and its tree and colour tables. Until it is read, the vertex
  colour's RGB has no measured source.
- `Libc_Rand`'s range, which decides whether the 0.18 jitter is symmetric.
- What offset from the rocket's own origin the pushed matrix carries
  (`0x00124880..0x00124888` adds a scaled vector to the position).
- Pools 0, 1, 2 and 5 (the `ThickLine` family) and their users.

## Omega

**Checked, differs.** Omega's `data00.psarc` ships
`Data/ribboneffects/rockettrail_triangle.rcsmodel`,
`RocketTrail_Shadow_triangle.vex`, `HD_RocketTrail*.rcsmaterial` (three
variants each) and both `smoke_trails_frame{1,2}_ALPHA.gnf`, and its `eboot.bin`
names `rockettrail_triangle` and `RocketTrail_Shadow_triangle` beside
`Data\RibbonEffects\`. But **no `smoke_trails_opacity_ramp` exists on Omega's
disc or in its executable's strings** (case-insensitive search of all five
archives and `eboot.bin`), so its alpha-over-life law cannot be this table and
is unread. Not wired on Omega; no PS4 capture path exists.

## Reproducing this

The live dump, the decoder and the interpreter harness live in the lane's
scratch directory (`data/scratch/hd-rocket-trail/`: `rocketdump.py`,
`ana.py`, `side.py`) and are not committed; the dump drives the same front-end
walk `scripts/rpcs3-hd-weapon.py` does, writes held weapon state 0 and taps
Triangle, then pauses five times and reads `RaceManager + 0x2dcc`'s pool 4.
