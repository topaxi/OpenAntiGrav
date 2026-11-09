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
`+0x74` is the `smoke_trails_frame1_ALPHA` ribbon (widths `0.05` to `1.5`, life `0.7` s,
three chains, 100 nodes, helix radius `0.8`, element class `0x002ac070`, a `FatLine`
subclass with its own update). `node+0xe8` read `1` on the first node and `0` on the
second in the live run, in `Locators.vex` file order (`wingtip_left` first): a side
flag, read nowhere else.

**Confidence 80** for the law above (read in full, constants resolved through the TOC,
and the node's live fields match it: `+0xe4` read `1.283` with two handles `0x70` apart
(`0x342c95a0`, `0x342c9610`) while the pad time was forced above zero, and `0` with
handles `0` before; `-0.017`, one frame time below zero, in a run that never forced it).
**Not read:** what `0x002ac208` and the subclass update `0x002ac070` do with the strength
(it reads like the ribbon's life scale, as the Missile's `f1 = 1.0` is in
`FatLine_PushChains`), the helix handedness, and what flag `0x40` is.

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

Not named: `0x006b4558`, the node factory (allocates `0x100` bytes, stores the parent at
`+0xc`, links it, runs the constructor's tail); `0x002db680`, `0x002db378`, `0x002db718`,
`0x002db740`, `0x002db7a0` and `0x002db8e0`, the importer's other vtable slots, not read;
`0x002ac208` and `0x002ac070`, the pool `+0x74` push and element class.
