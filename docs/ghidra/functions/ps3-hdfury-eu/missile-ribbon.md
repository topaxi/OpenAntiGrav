# The Missile's ribbons: two `FatLine` pools, not `RibbonEffects` pool `+0x74`

Read 2026-11-07 (`hd-missile-ribbon`) in Ghidra (`/hdfury/EBOOT-ps3-hdfury-eu.elf`)
and with capstone over the raw bytes of the decrypted `EBOOT.elf` where the
decompiler truncates at AltiVec, plus the original's own frames
(`weapons.md`, "The Missile's red rings", the pinned pair method).

**Correction to [rocket-trail.md](rocket-trail.md)'s pool table.** The row that
named pool `+0x74` (`smoke_trails_frame1_ALPHA.gtf`, 18 x 0x70) as "Missile" is
wrong, and `0x002ab9d0` is not that pool's acquire. Evidence, confidence 90:

- The Missile's init `0x0011c5d0` (`Missile_Init_q`, between `Missile.cpp`'s
  constructor `0x0011ccc8` and its update) acquires **two elements of pool
  `+0x50` and two of pool `+0x68`**, into the object's `+0x184`, `+0x188`,
  `+0x18c`, `+0x190`. It reads `RaceManager + 0x2dcc`, then each pool's
  `{count +0x50/+0x68, flags +0x58/+0x70, elements +0x54/+0x6c}`, takes the first
  free flag, writes `3` and strides 0x60. It never touches `+0x74`.
- A `bl` decode of `0x0011c5d0..0x0011f5d0` finds four calls to `0x002ab9d0`,
  at `0x0011dad0`, `0x0011db00`, `0x0011db24`, `0x0011db44`, one per ribbon
  pointer, and none to `0x002ac208` (pool `+0x74`'s element push).
  `0x002ab9d0` is a **node-laying loop** over `count` chains
  (`FatLine_PushChains`), not an acquire.
- Pool `+0x74` is acquired by `0x002db130`, a per-craft update (it reads
  `craft + 0x5fac`, `+0x340`, a speed ramp), and pushed with `0x002ac208`. Its
  user is a craft effect, **unnamed** (confidence below 50); the smoke texture
  is its own, not the Missile's.
- Pool `+0x5c` (`ThickLine_bighalo`, 20 elements) is the Missile **explosion's**:
  `0x00155568` (the explosion init) acquires 16 of them, and `0x00154a78`
  (its update) calls `0x002ab9d0` with them.
- The film agrees: the original's Missile (`theirs_02.png` of the
  `hd-weapons-2` pair, lap clock 2.7 s) trails thin red sinuous ribbons, no grey
  smoke.

## The element class

Pools `+0x50`, `+0x5c`, `+0x68`, `+0x8c` hold one class, `FatLine` (base
`0x002ab028`, vtable `0x00869de0`): slot 0 `FatLine_Init` `0x002ab098`, slot 1
`FatLine_Update` `0x002ab608`, slot 2 `FatLine_Render` `0x002ab458`, slot 5
`FatLine_AlphaOverLife` `0x002ab410`. The shaders are `FatLine_vp`
(`0x0092ef00`) and `FatLine_fp` (`0x0092ee80`), compiled into the executable:
no `.rcsmaterial` names them (`DATA02` has `thickline*.gtf` and nothing else of
the family).

`FatLine_Init` copies the 0x40-byte config to `this + 0x20`. The blocks, read from
the ELF (`+0x10` onward):

| field (config offset) | `+0x50` (`0x00920fd0`) | `+0x68` (`0x00921010`) | meaning, how it is read |
| --- | ---: | ---: | --- |
| `+0x10` width at birth | 0.5 | 0.08 | `FatLine_Update` writes `width = t * (w10 - w14) + w14` |
| `+0x14` width at death | 0.0 | 0.3 | same |
| `+0x18` lifetime, s | 0.3 | 2.8 | `t = life * (1 / lifetime)`, init stores the reciprocal at `this + 0x2c` |
| `+0x1c` | 0.2 | 0.2 | passed to the line builder, becomes the vertex program's `c[211].x`, which multiplies `u` (a per-world-unit texture scale) |
| `+0x20` | `ThickLine_Red.gtf` | `ThickLine_Red.gtf` | texture name |
| `+0x24` chains | 1 | 3 | `FatLine_PushChains` lays one node per chain per call |
| `+0x28` node capacity | 50 | 350 | builder buffers |
| `+0x2c` phase step | 0 | 0.06 | added to the running phase after each chain |
| `+0x30` helix radius | 0 | 0.6 | multiplies the reference axis in the push |
| `+0x34` byte: use opacity ramp | 0 | 1 | `FatLine_AlphaOverLife` |

(Pool `+0x5c`: widths 0 and 2.0, life 1.2, 1 chain, 350 nodes, step 0.06,
radius 0.6, ramp off. Pool `+0x74`, `smoke_trails_frame1_ALPHA`: widths 0.05 and
1.5, life 0.7, 3 chains, 100 nodes, ramp off, radius 0.8, element class
`0x002ac070`, not `FatLine`.)

So each Missile head gets **a stub** (`+0x50`: 0.3 s, one straight chain,
tapering 0.5 to 0) and **a helix** (`+0x68`: 2.8 s, three chains 60 degrees apart
winding at radius 0.6, widening 0.08 to 0.3), which is the red wavy ribbon the film
shows. Confidence 85.

## The law

`FatLine_PushChains(f1 = 1.0, element, anchor)` (`0x002ab9d0`), confidence 85. The
Missile passes `f1 = 1.0` (`lfs f29, -0x3684(r2)`), the anchor is the head's
4 x 4 matrix (`missile + 0xf0` for head A, `+0x130` for head B; rows 0, 1, 2 the
basis, row 3 the position). For chain `i` of `count`:

```text
node.life = node.life0 = f1 * lifetime
node.width = config +0x10
node.u = sentinel 0x00800000                       (not drawn until the first update)
theta = (pi / count) * i + element.phase           (radians: 2/pi quadrant split, polynomial)
node.pos = anchor.pos + rotate(row1 * radius, about row2, by theta)
element.phase += phase step                        (after each chain)
```

`rotate` is the Rodrigues rotation the vector code builds (`aa^T` rows, `cos`,
`sin` from the polynomial at `0x00879c3d0..`), with `row2` the axis. `row2` is
the head's forward axis and `row1` its second basis row (HD's objects: row 1 points
down on a level track, see rocket-trail.md). The sign of the `sin` term (the
helix's handedness) is **not decided**: the scratch decode shows both terms and
no live node dump was taken.

The node is appended at the newest end of the chain (`RibbonNode_Link`
`0x002ebce8`, the same 4,000-node pool as the Rocket's) and stays where it is
(`RibbonNode_Advance` `0x002ebec8` adds `velocity * dt` and the velocity row is
zero).

`FatLine_Update(dt)` (`0x002ab608`, confidence 88) walks each chain **oldest to
newest**:

```text
if life <= 0: unlink and free                          (life read before the decrement)
t     = life * (1 / lifetime)
width = t * (w_birth - w_death) + w_death
alpha = ramp ? ramp_float(1 - t) : (1 - t)             (`FatLine_AlphaOverLife`)
u     = running sum of |pos - previous pos|            (rewritten every update, not frozen)
life -= dt
```

then the oldest and the newest node of every chain have their alpha cleared, and
the element is alive while any node was walked. The ramp is
`smoke_trails_opacity_ramp.tga`'s table, read as rocket-trail.md reads it.

## The draw

`FatLine_Render` (`0x002ab458`) feeds each chain's nodes, skipping any whose
`u` is still the sentinel, to `LineBuilder_AddSample` (`0x002ebef8`: a 16-byte
`(position, width)` and an 8-byte `(alpha, u)` per node), then
`LineBuilder_Flush` (`0x002ebf80`) draws a **triangle strip of `2n - 2`
vertices**, every attribute on a frequency divider of 2, attribute 4 on modulo
(the side).

`FatLine_vp`, 23 instructions (`ps3-microcode.py vp 0x0092ef00`):

```text
prev = v[0], cur = v[1] (w = width), next = v[2]    (the same array at +0, +16, +32)
side = normalize( (eye - cur) x (cur - next) + (eye - prev) x (prev - cur) )
pos  = cur + side * (width * v[4].x)                (v[4].x the strip's two sides)
TC0  = ( v[5].x * c[211].x,  max(v[4].x, 0) )       (u * texture scale, v)
COL0 = ( c[209].xyz, v[3].x )                       (tint, alpha)
```

The ribbon faces the camera and is two-sided; the cross products are the
screen-space side of the segment, averaged over the two segments at a node.
`v[4]` is read as `-1` and `+1`: **read off the vertex program's `MAX` against
0, not off the array**, which is set by a constant this lane did not locate;
a `0 / 1` array would give a one-sided ribbon. Confidence 70.

`FatLine_fp`: `H0 = tex(TC0) * COL0` (two instructions). The state is the
`RibbonEffects_Render` one for pools 0, 2, 1, 5 (rocket-trail.md, "The manager"):
`blend(SRC_ALPHA, ONE)`. The tint `c[209]` comes from a global vector the
builder's bind function `0x002ec748` negates (`-[g + 0x70]`); its value was **not
read**. Against the film it is white or near it (the texture is red already).

## Named here

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0011c5d0` | `Missile_Init_q` | 65 |
| `0x002ab9d0` | `FatLine_PushChains` | 85 |
| `0x002ab608` | `FatLine_Update` | 88 |
| `0x002ab098` | `FatLine_Init` | 85 |
| `0x002ab458` | `FatLine_Render` | 82 |
| `0x002ab410` | `FatLine_AlphaOverLife` | 85 |
| `0x002ebef8` | `LineBuilder_AddSample` | 80 |
| `0x002ebf80` | `LineBuilder_Flush` | 78 |
| `0x002ebec8` | `RibbonNode_Advance` | 85 |
| `0x002ec748` | `LineBuilder_BindParams_q` | 55 |

`0x002db130` (pool `+0x74`'s craft-side user) and `0x0011e640` (it stores into the
Missile's matrices at `+0xf0..+0x150`, the heads' placement; not read here, the
orbit and scale are `hd-weapons-2`'s open items) stay unnamed. No Ghidra rename was
applied; this page is authoritative (ADR-0005).

## 2048 and Omega

**Checked, applies, not wired.** Both ship the same family: `eboot.bin` (Omega)
and 2048's `eboot.elf` name `FatLine_vp`, `FatLine_fp`, `ThickLine`,
`ThickLine_bighalo`, `ThickLine_Red`, `ThickLine_Yellow`, and Omega's
`data00.psarc` and patch `data08.psarc` ship `Data/tex/ThickLine_Red.gnf`.
Whether their Missile takes the same four elements was not read (their
`Missile.cpp` functions are not in this lane); the `FatLine` constants above are
HD's.
