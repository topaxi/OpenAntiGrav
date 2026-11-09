# HD's `wingtip` locators: collected, and read by nothing

Read 2026-11-09 (lane `hd-nose-glow-control`) off the decrypted `EBOOT.elf` with
[`scripts/ps3-toc.py`](../../../../scripts/ps3-toc.py) and a capstone disassembly
of the cited ranges (no Ghidra bridge was up; a rename in the Ghidra database is
still to do, the names are in `names.tsv`). The question it answers: HD's `.vex`
files author a `wingtip` locator pair (class `0x3ec`), the executable names an
importer `ShipWingtipVortices`, and a player reports white contrails at the Zone
ship's nose tips. Does the game draw anything from those locators? The
measured half, with the frames, is
[`rendering/hd-wingtip-contrails.md`](../../../rendering/hd-wingtip-contrails.md).

## The class

`ShipWingtipVortices_RegisterClass` (`0x002db500`) is the static registration
pair, the same shape as [`ShipAbsorbNode_RegisterClass`](absorb-feedback.md):
`(1, 0xffff)` selects the register branch, which loads `li r4, 0x3ec` at
`0x002db558` and stamps the importer name `ShipWingtipVortices_Importer.h`
(`0x007a2608`, TOC slot `0x008b40c4`). The constructor `0x002db5e8` writes the
vtable `0x0086a340` at `+0`, the **type token `0x00874590`** at `+8` and clears
`+0xe4` (the craft back-pointer every ship node carries). The factory that
allocates the `0x100`-byte node is `0x006b4558` (unnamed; its `.cpp` name is
`ShipWingtipVortices_Importer.cpp`, `0x007a2628`).

The token is the neighbour of absorb's `0x00874598`: `ps3-toc.py`'s slot scan
finds `0x00874590` in two TOC slots, `0x008b4084` (the three importer functions)
and `0x008a8b48`, which exactly one function loads.

**Confidence 85.** Every function was read; the id, the token and the name close
against each other.

## The gatherer, called, and its result never read

`Ship_GatherWingtipNodes` (`0x000d2d20`) is `Ship_GatherAbsorbNodes`' and
`Ship_GatherCollisionFxNodes`' third sibling: it clears five words at
`craft+0x79f8`, takes the root at `craft+0x6af0`, walks the subtree with
`0x00683518` and keeps every node whose `+8` equals the token at `0x00874590`
(loaded from TOC `-0x4990(r2)`, slot `0x008a8b48`), at most **five**, and stores
the count at `craft+0x7a28`.

It is called. `0x000db090`, `0x000db1e0` and `0x000db400` (the hull and LOD swap
paths) call the collision-spark gatherer (`0x000d2e30`) and branch straight to
this one (`b 0x000d2d20` at `0x000db14c`, `0x000db17c`, `0x000db2a0`,
`0x000db4bc`, `0x000db4ec`); the two ship constructors (`0x000ddd58`,
`0x000dfd90`) zero the count at `0x000de740` and `0x000e077c`.

**Nothing reads what it collects.** A scan of every D-form load, store and
`addi` in the code segment (`0x00010000`-`0x00850000`) for the displacements
`0x79f8`, `0x79fc`, `0x7a00`, `0x7a04`, `0x7a08` and `0x7a28`, on any base
register, finds only the gatherer's own stores and the two constructors' zeroing
of the count. The sibling arrays are read: the collision-spark slots
`0x79d0`-`0x79f4` by `0x000d14c8`, `0x000e6fd8` and `Trail_SpawnHitEffect`
(`0x002e3858`); the absorb slots `0x7a0c`-`0x7a20` and count `0x7a2c` by
`Ship_PlayAbsorbFeedback` (`0x000d9398`) and `0x00758110`.

What this does **not** exclude: a reader that walks the array through a computed
index (`lwzx`) rather than a displacement, one in `DFENGINE.SPRX` or on an SPU
job, or a consumer that finds the nodes by scene-graph walk (class id or name)
instead of the craft's array. None was found: the string `wingtip`
(`0x0079c970`) sits in the class-name table and is attributed to no function by
`ps3-toc.py attrib`, and nothing on the disc is named for a wingtip, vortex or
contrail: none of the 88 `.pob` entries across the seven archives, and the ribbon
models in `/data/ribboneffects/` are the engine, wake, rocket, rocket-shadow and
leech ribbons only.

**Confidence 80** that HD's own code draws nothing from the `wingtip` locators.
Reproduce: `python3 scripts/ps3-toc.py attrib 0x007a2628` lists the importer's
three functions; the displacement scan is twenty lines over
`ps3-toc.py`'s `Image` (every `op in {14, 32..38, 40..44, 48..54, 58, 62}` with
the immediate in the set and a base other than `r1`/`r2`).

## Where the locators sit

Two per hull, `wingtip_left` and `wingtip_right`, a pure translation matrix, in
each ship's `Locators.vex`. Read with the `oag-vex` node reader:

| Hull (HD) | `wingtip_left` (x, y, z) | Same point as |
| --- | --- | --- |
| Feisar | `(2.687, -0.740, -4.248)` | `ship_collision_fx2` (the rear wing tip; `engine_flare` is at `z = -4.93`, the nose muzzle at `z = +6.22`) |
| Assegai | `(2.776, -1.402, -2.016)` | |
| Qirex | `(2.762, -1.433, -5.879)` | |
| Zone | `(2.840, 0.179, 2.202)` | `ship_collision_fx2` (the forward wing; the cannon flashes are at `z = +6.77`) |

The right one is the mirror. On the Feisar they are rear wing tips, not nose
tips; on the Zone ship they are the forward-swept wing tips, level with the
middle of the hull and 4.6 units behind its cannon flashes. All 39 `Locators.vex`
entries on the disc (38 distinct paths: `mirage` is in both `DATA03` and
`DATA06`) were read: **every one carries exactly two class-`0x3ec` nodes**,
`wingtip_left` and `wingtip_right`, at `x = +-2.1` to `+-3.0`. The `_c1` and
`_n1` variants repeat the base hull's pair with small offsets (Feisar `_c1`:
`z = -3.54` against the base's `-4.25`).

## The other titles

- **2048** (`data.psarc`, `hdships/*/Locators.vex`, little-endian): the same two
  nodes at the same coordinates on Feisar and Zone (checked bit for bit against
  HD's). **Checked, applies, not wired**: its executable is an encrypted
  `eboot.bin`, so the importer and the gatherer were not read here.
- **Omega** (`data03.psarc`): the same two nodes at the same coordinates on
  Feisar and Zone. `eboot.bin` carries `ShipWingtipVortices_Importer.cpp`/`.h`
  and the string `wingtip`. **Checked, applies, not wired**: the consumer scan
  above was not repeated on the x86-64 image (a displacement scan reads
  differently there; the next lane that opens it in Ghidra starts from the
  importer string).
- **Pulse** authors none: its class table stops at `0x3eb`
  ([vex-classes.md](vex-classes.md)).

## Names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x000d2d20` | `Ship_GatherWingtipNodes` | 80 |
| `0x002db500` | `ShipWingtipVortices_RegisterClass` | 80 |
| `0x002db5e8` | `ShipWingtipVortices_Construct` | 75 |

Not named: `0x006b4558`, the node factory (allocates `0x100` bytes, stores the
parent at `+0xc`, links it, runs the constructor's tail); `0x002db680`, a third
function of the importer file whose role was not read.
