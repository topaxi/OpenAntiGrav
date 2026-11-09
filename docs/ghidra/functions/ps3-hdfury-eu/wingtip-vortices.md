# HD's wingtip vortices: the `wingtip` locators drive two ribbons that run while a speed pad or turbo acts

Read 2026-11-09 (lane `hd-nose-glow-control`) off the decrypted `EBOOT.elf` with
[`scripts/ps3-toc.py`](../../../../scripts/ps3-toc.py) and a capstone disassembly
of the cited ranges (no Ghidra bridge was up, so the names below are in `names.tsv`
but not yet in the Ghidra database), then checked live on RPCS3. The question: HD's
`.vex` files author a `wingtip` locator pair (class `0x3ec`), the executable names an
importer `ShipWingtipVortices`, and a player reports white contrails at the Zone
ship's wing tips. **The class is a real effect.** Each of the two nodes updates
itself every frame and lays a ribbon behind its wing tip while the craft's
speed-pad force time or a turbo flag is up, scaled by forward speed. The measured
half, with the frames, is
[`rendering/hd-wingtip-contrails.md`](../../../rendering/hd-wingtip-contrails.md).

This page corrects its own first reading of the same day, which found no reader of
the craft's `wingtip` array and concluded the locators draw nothing. That reader
search was right and the conclusion was wrong: the node does not need the array.

## The class

`ShipWingtipVortices_RegisterClass` (`0x002db500`) is the static registration
pair, the same shape as [`ShipAbsorbNode_RegisterClass`](absorb-feedback.md):
`(1, 0xffff)` selects the register branch, which loads `li r4, 0x3ec` at
`0x002db558` and stamps the importer name `ShipWingtipVortices_Importer.h`
(`0x007a2608`, TOC slot `0x008b40c4`). The constructor `0x002db5e8` writes the
vtable `0x0086a340` at `+0`, the **type token `0x00874590`** at `+8` and clears
`+0xe4`. The node is `0x100` bytes; the factory is `0x006b4558` (unnamed; its
`.cpp` name is `ShipWingtipVortices_Importer.cpp`, `0x007a2628`).

The token is the neighbour of absorb's `0x00874598`: `ps3-toc.py`'s slot scan
finds `0x00874590` in two TOC slots, `0x008b4084` (the three importer functions)
and `0x008a8b48`, which exactly one function loads (the gatherer below).

**Confidence 85.**

## The node's own update: `ShipWingtipVortices_Update` (`0x002db130`, vtable slot 3)

The vtable's slot 3 is class-specific code, as it is for absorb's node (`0x002d8920`).
Read in full, with `f1 = dt` and `r3 = node` (`node+0xec` is the craft, `*(craft+0x5fac)`
the craft entry):

```text
turbo   = (entry+0x264 & 0x20 or entry+0x264 & 0x40) and FUN_000d19b0(craft) == 1   # 0x002db164..0x002db254
padtime = entry+0x2e8 > 0                                                           # 0x002db180..0x002db18c
if (turbo or padtime) and craft+0x5fa4 not in {4, 5, 6}:                           # 0x002db258..0x002db26c, unsigned
    node+0xe4 = 1.3                                                                 # 0x3fa66666, TOC 0x008b4094
speed = entry+0x340                          # |dot(velocity, forward)|, craft-flags.md
ramp  = speed < 50  ? 0                      # 0x42480000, TOC 0x008b40a8
      : speed >= 100 ? 0.3                   # 0x42c80000, 0x3e99999a
      : (speed - 50) * 0.006                 # 0x3bc49ba6
if node+0xe4 > 0:                            # 0x002db1bc
    if node+0xe0 == 0: take a free element of pool +0x74 of RaceManager+0x2dcc, flag byte = 3, node+0xe0 = it
    0x002ac208(node+0xe0, node+0x38, ramp * node+0xe4 * 0.7692, node+0xe8 byte)     # 0x3f44ec4f = 1/1.3
    node+0xe4 -= dt
else:
    node+0xe0 = 0                            # 0x002db290
```

So **the amount is a 1.3 s timer** refilled to its top while the trigger holds, and
the strength handed to the ribbon is the timer's fraction times the speed ramp: nothing
below 50 units/s, at most `0.3` from 100. `entry+0x2e8` is
`Craft_ApplySpeedPad_q`'s remaining pad force time (`0x000eeb70` loads it at
`0x000eebec`, subtracts the frame time and stores it back at `0x000eec08`, and scales the
pad force by it); the two flag bits are the Turbo pickup's (`0x20`,
[craft-flags.md](craft-flags.md), `Ship_UpdateTurboFlag`) and `0x40`.
`0x002ac208` is pool `+0x74`'s element push
([missile-ribbon.md](missile-ribbon.md), which left the pool's user unnamed): pool
`+0x74` is the `smoke_trails_frame1_ALPHA` ribbon, whose element is a `FatLine`
subclass (`VortexLine`, constructor `0x002ac070`) read in full below. `node+0xe8` read
`1` on the first node and `0` on the second in the live run, in `Locators.vex` file
order (`wingtip_left` first): a side flag, read only by the push, where it mirrors the
spin.

**Confidence 80** for the law above (read in full, constants resolved through the TOC,
and the node's live fields match it: `+0xe4` read `1.283` with two handles `0x70` apart
(`0x342c95a0`, `0x342c9610`) while the pad time was forced above zero, and `0` with
handles `0` before; `-0.017`, one frame time below zero, in a run that never forced it).
What the push does with the strength and the side byte is the next section (it is an
opacity scale, not a life scale). **Not read:** what flag `0x40` is.

## The ribbon: pool `+0x74`'s element, `VortexLine` (2026-11-09, lane `hd-wingtip-vortices`)

Read 2026-11-09 in Ghidra (`/hdfury/EBOOT-ps3-hdfury-eu.elf`) and with capstone over
`EBOOT.elf` where Ghidra's function body is truncated (`FatLine_Update`), constants
resolved with `ps3-toc.py`. The base class, `FatLine`, its update, render and
shaders are [missile-ribbon.md](missile-ribbon.md)'s; this section is what the
subclass changes. `VortexLine` is our name for it: the executable names neither class.

### The class

`0x002ac070` (`VortexLine_Construct`) calls the `FatLine` constructor `0x002ab028`,
writes the vtable `0x00869e00` (TOC `0x5d94(r2)` of `0x002ac208`'s TOC `0x008ad4d8`,
slot `0x008b326c`) and stores `1.0` (`lis r9, 0x3f80`) at `+0x60`, the strength.
The vtable's six slots are OPD entries; resolved:

| Slot | OPD | Code | Shared with `FatLine` (`0x00869de0`)? |
| --- | --- | --- | --- |
| 0 init | `0x00881e90` | `0x002ab098` `FatLine_Init` | yes |
| 1 update | `0x00881ef0` | `0x002ac0f0` `VortexLine_Update` | no (base `0x002ab608`) |
| 2 render | `0x00881ea0` | `0x002ab458` `FatLine_Render` | yes |
| 3 | `0x00881ed0` | `0x002ac010` `VortexLine_Destruct` | no (vtable, then base `0x002aaf18`) |
| 4 | `0x00881ec8` | `0x002abfc8` `VortexLine_Delete` | no (the same, then `0x00676778`, the free) |
| 5 opacity | `0x00881ec0` | `0x002abfb8` `VortexLine_AlphaOverLife` | no (base `0x002ab410`) |

### The config, `0x00921090`

`FatLine_Init` copies 0x40 bytes to `element+0x20`. The ELF's bytes, plus the colour a
static initializer writes at boot:

| Config | Element | Value | Read as (by) |
| --- | --- | --- | --- |
| `+0x00..+0x0f` | `+0x20` | **`(1.0, 1.0, 1.0, 0.3)`** | colour, `vBaseColour` (below) |
| `+0x10` | `+0x30` | `0.05` | width at birth (`t = 1`) |
| `+0x14` | `+0x34` | `1.5` | width at death (`t = 0`) |
| `+0x18` | `+0x38` | `0.7` | lifetime, s |
| `+0x1c` | `+0x3c` | `0.025` | `fTextureScale`: `u` (world units along the chain) times this |
| `+0x20` | `+0x40` | `0x0079c278` = `Data/Tex/smoke_trails_frame1_ALPHA.gtf` | texture |
| `+0x24` | `+0x44` | `3` | chains |
| `+0x28` | `+0x48` | `100` | nodes per chain (builder capacity) |
| `+0x2c` | `+0x4c` | `0.06` | phase step per chain push, radians |
| `+0x30` | `+0x50` | `0.8` | "radius": only a direction here, normalised away (below) |
| `+0x34` | `+0x54` | `0` | opacity-ramp byte; never read, slot 5 is overridden |

**The colour.** All five ribbon configs read zero colour in the file. The static
initializer `0x002a5ce0` (`RibbonEffects_InitConfigColours`, a `(1, 0xffff)`
constructor pair; `0x002a5d40` loads TOC slot `0x008b312c` = `0x00921090`) writes
`1.0` to `+0x00`, `+0x04`, `+0x08` (`lis r0, 0x3f80`, `stw` at `0x002a5d74..0x002a5d7c`)
and `*(0x00766ce0)` = `0.3` to `+0x0c`. The other four take vectors from
`0x0079c110..`: `+0x50` (Missile stub) `(1, 0.2, 0, 1)`, `+0x68` (Missile helix)
`(1, 0.2, 0.1, 1)`, `+0x5c` (Missile explosion) `(1, 0.25, 0.1, 1)`, `+0x8c` (`ThickLine`)
`(0.729, 1, 1, 1)`. `FatLine_Init` hands `element+0x20` to the line builder's init
`0x002ec408` in `v2`, which stores it at `builder+0x40` (`stvx v2, r3, r0`, `r0 = 0x40`,
at `0x002ec448`); `LineBuilder_BindParams_q` (`0x002ec748`) binds `builder+0x40` by
name: its four parameter names resolve (TOC `0x737c..0x7388` of `0x002ec748`) to
`kWorldViewProj`, `vCameraPos`, `fTextureScale` and `vBaseColour`, and `vBaseColour` is
`builder+0x40` (`addi r30, r24, 0x40` at `0x002ecc60`, passed at `0x002ecccc`). In
`FatLine_vp` it is `c[209]` (the parameter table's `float4` at `c465`), used only as
`o[COL0].xyz = c[209].xyz`: **the ribbon is white times the texture; the `0.3` in `w`
is never read.** (So the negated global `missile-ribbon.md` took for the tint is
`vCameraPos`, `c[210]`, and the Missile's tint is the red `(1, 0.2, 0)` above.)
Confidence 85.

**The texture** is `/data/tex/smoke_trails_frame1_alpha.gtf` in `DATA02.PSARC`
(175,104 bytes, SHA-1 `d1ed2863...`). `DATA02` also ships a different file of the same
name under `/data/ribboneffects/textures/` (131,200 bytes, SHA-1 `4f9edbc8...`); the
config's `Data/Tex/` prefix names the first. Confidence 85 (PSARC paths are
lower-cased; the loader's case folding is assumed, not read).

### The push: `VortexLine_PushChains` (`0x002ac208`)

Called once per frame per wingtip node, `r3 = element`, `r4 = node+0x38` (the node's
4 x 4 world matrix: rows 0, 1, 2 the basis, row 3 the position; the locator is a pure
translation, so the basis is the craft's), `f1 = strength`, `r6 = side byte`:

```text
element+0x60 = strength                                     # stfs f1, 0x60(r3)     @0x002ac284
step  = pi / chains                                         # 0x40490fdb, TOC 0x5d9c @0x002ac2d8
phase = side ? element+0x10 : -(element+0x10)               # beq r6==0 -> fneg     @0x002ac86c
for i in 0 .. chains-1:
    node = RibbonNode alloc (0x002ebde8); if none, skip
    node+0x00..0x3f = the anchor matrix                     # rows 0..3             @0x002ac510..0x002ac558
    node+0x60 = node+0x64 = lifetime (0.7)                  # life, life0; NOT scaled by strength
    node+0x68 = width at birth (0.05)
    node+0x6c = 0x00800000 sentinel (u; not drawn before the first update)
    theta = step * i + phase                                # fmadds f11, f30, f11, f31 @0x002ac480
    o     = rotate(row1 * radius, about row2, by theta)
    node+0x40 = -normalize(o) * 2.5                         # 0x40200000 at 0x00766ce4 @0x002ac6b4..0x002ac798
    link node at the chain's newest end (RibbonNode_Link 0x002ebce8)
    element+0x10 += phase step (0.06)                       # @0x002ac7b4; phase above is not reloaded
```

So the node is born **at the wing tip itself** (row 3 copied, the rotated offset never
stored as a position) and carries a **velocity** of 2.5 units/s away from the craft's
forward axis, in the direction opposite the rotated `row1`.
`RibbonNode_Advance` (`0x002ebec8`, `node+0x30 += node+0x40 * dt`) moves it every
update: over its 0.7 s life a node drifts 1.75 units out from where the tip was. The
config's `0.8` only picks the direction: `|o| = 0.8` cancels in the normalisation.

The rotation is the Rodrigues matrix the vector code builds from `s = sin(theta)`, `c =
cos(theta)` (quadrant reduction `theta * 2/pi`, Cody-Waite `pi/2` split `0x3fc90fda` +
`0x33a22169`, the sine and cosine polynomials at `0x0079c4e0..0x0079c530`) and the axis
`a = row2`. The permute tables at `0x008c24c0` (three 16-byte patterns) place
`(c, s a_z, -s a_y)`, `(-s a_z, c, s a_x)`, `(s a_y, -s a_x, c)` as the skew part of
the three rows, and the product sums `o_x R0 + o_y R1 + o_z R2`, so:

```text
o = c v + s (a x v) + (1 - c)(a . v) a          with v = row1 * 0.8, a = row2
```

the right-handed rotation by `+theta` about `row2` in the matrix's own coordinates.
`f31` (the phase) is loaded once before the loop (`0x002ac2e0`) and not reloaded, so
the three chains of one push sit at `theta = phase, phase + pi/3, phase + 2pi/3`:
**a 120-degree fan, not a full circle**. The stored phase still advances `0.06` per
chain, so the fan turns by `0.18` rad per push (one push per frame), about 1.7 turns a
second at 60 Hz. The left node (side `1`) turns
by `+phase`, the right (side `0`) by `-phase`: **mirrored spin**. At `theta = 0` the
velocity is `-row1`; HD's `row1` points down on a level track
([rocket-trail.md](rocket-trail.md)), so chain 0 starts drifting up. The phase starts at
`0` (`FatLine_Init`, `stw r0, 0x10(r27)` at `0x002ab3a8`) when the element is taken and
keeps running while the element is held.

Confidence 82: read instruction by instruction, every constant resolved, and the
node offsets agree with `RibbonNode_Advance` and `FatLine_Render` (position at
`+0x30`, width `+0x68`, `u` `+0x6c`). The velocity's sign and the turn per frame are
the parts a live node dump would lift above 84.

### The update and the opacity

`VortexLine_Update` (`0x002ac0f0`, slot 1) calls `FatLine_Update` (`0x002ab608`), then,
for each chain (`element+0x18`, 8 bytes per chain: oldest, newest), zeroes the alpha
lane of the **second** node from each end (`oldest->+0x74`, `newest->+0x70`), with the
same lane mask (`0x00c47730 + 0x30`) the base update writes the alpha through
(`0x002ab804`). The base already clears the end nodes, so each chain fades over two
nodes at each end. It returns the base's result (alive while any node was walked).

`FatLine_Update` calls slot 5 at `0x002ab7d0` with `f1 = t = life * (1 / lifetime)`
(`0x002ab79c..0x002ab7a0`), 1 at birth and 0 at death. `VortexLine_AlphaOverLife`
(`0x002abfb8`) is three instructions:

```text
alpha = t * element+0x60          # lfs f0, 0x60(r3); fmuls f1, f1, f0; blr
```

against the base's `1 - t` (or the ramp texture). So **a node is most opaque when
born and fades linearly to nothing over 0.7 s**, and the whole element's opacity is
scaled by the **current** strength, rewritten by every push: `ramp(speed) * timer / 1.3`,
at most `0.3`. When the trigger stops the timer runs down for 1.3 s and the ribbon
fades with it; the last push leaves the strength at about `ramp * dt / 1.3`, so nodes
still alive after the last push are all but invisible. Width grows from `0.05` at
birth to `1.5` at death.

Confidence 84 (static; the live check is the strength at `element+0x60` against the
node's timer).

### The draw state

`RibbonEffects_Render` (`0x002a6478`) draws pools `+0x50`, `+0x68`, `+0x5c`, `+0x8c` after
`0x00678248(0x302, 1, 0, 0x303)` and then calls `0x00678248(0x302, 0x303, 0, 0x303)`
at `0x002a6980` before the `+0x74` loop (`lwzu r10, 0x74(r29)` at `0x002a6988`) and
the `+0x80` (Rocket) loop: **pool `+0x74` blends `SRC_ALPHA, ONE_MINUS_SRC_ALPHA`**, not
additive. Fragment: `FatLine_fp`, `tex * COL0`, so `rgb = tex.rgb * (1, 1, 1)`, `a =
tex.a * t * strength`. Two-sided, camera-facing strip, as
[missile-ribbon.md](missile-ribbon.md) reads `FatLine_vp`. Confidence 85.

### Limits

Pool `+0x74` holds 18 elements; each craft takes two while its timer runs (one per
wing tip), so a full grid of eight under pads at once needs 16. An element released
(`node+0xe0 = 0`) stays alive, drawing nothing new, until its last node dies (0.7 s).

## The gatherer, called, and its result never read

`Ship_GatherWingtipNodes` (`0x000d2d20`) is `Ship_GatherAbsorbNodes`' and
`Ship_GatherCollisionFxNodes`' third sibling: it clears five words at `craft+0x79f8`,
walks the subtree under `craft+0x6af0` and keeps every node whose `+8` equals the token
(loaded from TOC `-0x4990(r2)`, slot `0x008a8b48`), at most five, and stores the count at
`craft+0x7a28`. It is called after the collision-spark gatherer on the hull and LOD swap
paths (`b 0x000d2d20` at `0x000db14c`, `0x000db17c`, `0x000db2a0`, `0x000db4bc`,
`0x000db4ec`); the ship constructors zero the count (`0x000de740`, `0x000e077c`).

No D-form load, store or `addi` in the code segment (`0x00010000`-`0x00850000`) reads
`craft+0x79f8..0x7a08` or `+0x7a28`. The collision-spark and absorb arrays are read
(`0x000d14c8`, `0x000e6fd8`, `Trail_SpawnHitEffect`; `Ship_PlayAbsorbFeedback`,
`0x00758110`). The vortices need no reader because each node drives itself.
Reproduce: `python3 scripts/ps3-toc.py attrib 0x007a2628` lists the importer's three
functions; the scan is twenty lines over `ps3-toc.py`'s `Image` (every opcode in
`{14, 32..38, 40..44, 48..54, 58, 62}` with the immediate in the set and a base other
than `r1`/`r2`).

## Where the locators sit

Two per hull, `wingtip_left` and `wingtip_right`, a pure translation matrix, in each
ship's `Locators.vex`. Read with the `oag-vex` node reader:

| Hull (HD) | `wingtip_left` (x, y, z) | Same point as |
| --- | --- | --- |
| Feisar | `(2.687, -0.740, -4.248)` | `ship_collision_fx2` (the rear wing tip; `engine_flare` is at `z = -4.93`, the nose muzzle at `z = +6.22`) |
| Assegai | `(2.776, -1.402, -2.016)` | |
| Qirex | `(2.762, -1.433, -5.879)` | |
| Zone | `(2.840, 0.179, 2.202)` | `ship_collision_fx2` (the forward wing; the cannon flashes are at `z = +6.77`) |

The right one is the mirror. On the Feisar they are rear wing tips; on the Zone ship
the forward-swept wing tips, level with the middle of the hull and 4.6 units behind
its cannon flashes. All 39 `Locators.vex` entries on the disc (38 distinct paths:
`mirage` is in both `DATA03` and `DATA06`) were read: **every one carries exactly two
class-`0x3ec` nodes**, `wingtip_left` and `wingtip_right`, at `x = +-2.1` to `+-3.0`.
The `_c1` and `_n1` variants repeat the base hull's pair with small offsets (Feisar
`_c1`: `z = -3.54` against the base's `-4.25`).

## The other titles

- **2048** (`data.psarc`, `hdships/*/Locators.vex`, little-endian): the same two
  nodes at the same coordinates on Feisar and Zone (checked bit for bit against
  HD's). **Checked, applies, not wired**: its executable is an encrypted
  `eboot.bin`, so the importer and the update were not read here.
- **Omega** (`data03.psarc`): the same two nodes at the same coordinates on Feisar and
  Zone. `eboot.bin` carries `ShipWingtipVortices_Importer.cpp`/`.h` and the string
  `wingtip`. **Checked, applies, not wired**: the update was not read on the x86-64
  image; the next lane that opens it in Ghidra starts from the importer string, and
  the update's constants (`50`, `100`, `0.3`, `0.006`, `1.3`) are the search key.
- **Pulse** authors none: its class table stops at `0x3eb` ([vex-classes.md](vex-classes.md)).

## Names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x002db130` | `ShipWingtipVortices_Update` | 80 |
| `0x000d2d20` | `Ship_GatherWingtipNodes` | 80 |
| `0x002db500` | `ShipWingtipVortices_RegisterClass` | 80 |
| `0x002db5e8` | `ShipWingtipVortices_Construct` | 75 |
| `0x002ac208` | `VortexLine_PushChains` | 82 |
| `0x002ac0f0` | `VortexLine_Update` | 82 |
| `0x002abfb8` | `VortexLine_AlphaOverLife` | 84 |
| `0x002ac070` | `VortexLine_Construct` | 80 |
| `0x002ac010` | `VortexLine_Destruct` | 72 |
| `0x002abfc8` | `VortexLine_Delete` | 72 |
| `0x002a5ce0` | `RibbonEffects_InitConfigColours` | 80 |
| `0x002ec408` | `LineBuilder_Init` | 60 |

Not named: `0x006b4558`, the node factory (allocates `0x100` bytes, stores the parent at
`+0xc`, links it, runs the constructor's tail); `0x002db680`, `0x002db378`, `0x002db718`,
`0x002db740`, `0x002db7a0` and `0x002db8e0`, the importer's other vtable slots, not read.
`LineBuilder_Init` (`0x002ec408`) is named off one store (`v2` to `+0x40`) and its
call site in `FatLine_Init` (texture, capacity and texture scale passed in); the rest
of it was not read, hence 60.
