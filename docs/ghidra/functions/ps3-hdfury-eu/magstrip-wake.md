# `MagstripWake` in Wipeout HD / Fury (PS3 `EBOOT.elf`)

Functions in `EBOOT.elf` (Wipeout HD Fury, `BCES-00664`, EU), PowerPC64, image
base `0`. The full object description, evidence and caveats are on
[`ps4-omega-eu/ships-effects.md`](../ps4-omega-eu/ships-effects.md) (section
"2026-10-05"); this page is the HD half, written 2026-10-05 by the
magfloor-omega-re lane, static reading only (no RPCS3 run). Addresses are
the code addresses Ghidra shows as `.opd` entries (`_opd_FUN_0010a0c0` is the code
at `0x0010a0c0`).

**The question the brief asked: does HD have the class? Yes (confidence 85).**
`MagstripWake.cpp` (`0x00782fa0`), `"MagstripWake"` (`0x00782f90`),
`"arc_anchor_point"` (`0x007834a0`, `0x00782fb8`),
`"**** WARNING **** : Ship has no arc_anchor_point locator"` (`0x00782fd0`),
`MagStripArc_vp`/`MagStripArc_fp` (`0x007a0d18`, `0x007a0d28`),
`Data/Tex/HD_electric_arc_8x8.gtf` (`0x007a0d38`), `Data/Tex/HD_ElectricArc_Contact.gtf`
(`0x007a0d70`) and `"Send due m_overMagStrip change - now %i\n"` (`0x007a9320`) are
all present. This is the class Omega and 2048 carry forward, not a newer one.

## Constructors - `0x0010a0c0` and `0x00109e40`

`MagstripWake_Construct` is `0x0010a0c0`; `MagstripWake_ConstructAlt` is `0x00109e40`.
They are byte-for-byte the same body (a complete and a base-object constructor
of the same class, a compiler pair), and both: call a base constructor, store the vtable
(`PTR_PTR_008a98d0`, the table at `0x00863868`), store `PTR_s_MagstripWake_cpp_008a98d8`
at `param_1[0xc]`, resolve `"arc_anchor_point"` against the ship (`FUN_006762e8` on the
ship's locator set, then a walk up the parent chain), allocate two `0x200`-byte objects
(the two ribbons), a `0x5e0` arc pool, OR `6` into the flags word and increment the
live-instance counter (a global block at `+0xb80`). **85** for `0x0010a0c0`: the same tag
string, the same `arc_anchor_point` lookup, the same `| 6` and counter increment as the
PS4 and Vita constructors. **75** for `0x00109e40` (identical body, but which one the
ship constructor calls was not checked).

## Vtable `0x00863868` (slot, function descriptor, code)

Slots 3, 5, 7, 12 and 13 hold `.opd` descriptors in the `0x0087540x` run; the others
are base class (`0x00885axx`/`0x00885cxx`). Descriptors at `0x00875408` ->
`0x00109028`, `0x00875428` -> `0x00109350`, `0x00875438` -> `0x001095e0`, `0x00875440`
-> `0x00109720`, `0x00875448` -> `0x00109858`.

- **Slot 5, `0x00109028` = `MagstripWake_EnqueueRender`, conf. 78.** Appends
  `{param_1, 0x4d000000 | (inst+0x11c & 0xfffff)}` to the list at `*(PTR_DAT_008a98c0) + 0x630`
  (count at `+0x44b0`): the same body as the PS4's `0x012e2720`. This is the
  cross-binary agreement the PS4 name rests on, and `renderer.md` already
  attributes this site to `MagstripWake.cpp` through the TOC slot `008a98c0`.
- **Slot 3, `0x00109350`** is *not* the PS4 update. It draws the two ribbons
  (`FUN_002aa1c8` on `inst+0x50` and `inst+0x54`) and calls `FUN_002bbd60` with the
  speed read at `ship_body+0x4c4` (the PS4's `+0x4b8`) and `ship+0x7820`/`+0x7830`
  (the ribbons' lateral frame) - which is the tail of the PS4's draw `0x012e2770`.
  So HD's slot order is not the PS4's and **slot index is not evidence across the
  two builds**; only slot 5 is matched. The arc update and arc quad build are
  presumably `0x00109858` (slot 7) and `0x001095e0`/`0x00109720` (slots 13, 12);
  **none of those were read**, and they stay unnamed (below 50).
  **Corrected 2026-10-05 (magstrip-hd-measure): that presumption was wrong** - `0x001095e0`
  and `0x00109720` are destructors and `0x00109858` is the ribbons' update; the arc pool is
  `0x002bbd60`/`0x002bb530`/`0x002bc7b0`, see the last section.
- `0x001092d0`/`0x00109198`/`0x00109568` etc. in the descriptor run are other
  vtable-adjacent code and were not read.

## What HD does with the arc textures and the sound

- **Textures** `Data/Tex/HD_electric_arc_8x8.gtf` and `HD_ElectricArc_Contact.gtf`
  ship in `DATA02.PSARC` (`/data/tex/hd_electric_arc_8x8.gtf`, 349,696 bytes;
  `/data/tex/hd_electricarc_contact.gtf`, 5,632 bytes), plus siblings
  `hd_electric_arc_8x8_nonanchored.gtf` (349,696), `psys/tex/hd_electric_arc_8x8.gtf`
  and `hd_electricity_1x4.gtf` (699,264). **No literal in the EBOOT names the
  `_nonanchored` or `electricity_1x4` files** (strings search), so they are unreferenced
  by name from the executable (they may be reached by a built path; not checked).
- **No `WO_MAGSTRIP_*` string exists in the HD EBOOT**, and no `MagStrip_Player`/`_NPC`.
  HD ships a different particle file instead: `/data/psys/wo_magstrip_lightning.pob`
  (3,360 bytes, `DATA02`) which **no string in the EBOOT references**. It, and its textures
  `psys/tex/pulse_mag_sprite1_orange_rings.gtf` and `weapons/textures/mag_lightning2_add_glow.gtf`,
  are found-but-unwired as far as static strings go (the name could be built by a format
  string; not searched).
- **Sound:** `~magstrip01` is a cue in `shiphd.bnk` (`DATA01`, `/data/sound/shiphd.bnk`, at
  file offset `0x19790`, beside `~jet..` cues; a `### Magstrip` section label follows at
  `0x198a8`). Its two string literals in the EBOOT (`0x00781080`, `0x00781dd0`) have no direct code
  references (they are TOC-addressed); the start site was **not located**.
- **Rumble:** `enter_mag_rumble.xml`, `travel_mag_rumble.xml`, `exit_mag_rumble.xml`
  (`/data/xml/rumble/`, 356, 548, 356 bytes) ship for the magstrip pad; the same three
  ship in the Omega `data00.psarc`. Who plays them was not searched.

## 2026-10-05, magstrip-omega-law lane: the replicated `m_overMagStrip` and the HD anchors

Static reading, `EBOOT.elf`; full evidence for the shared law is on
[`ps4-omega-eu/ships-effects.md`](../ps4-omega-eu/ships-effects.md) ("2026-10-05,
magstrip-omega-law lane").

### `ShipNet_CheckSendState` - `0x00337d78`, conf 65

The only reader of the string `"Send due m_overMagStrip change - now %i\n"` (`0x007a9320`,
TOC slot `0x008b5688`, reached as `-0x7d3c(r2)` on the `r2 = 0x008bd3c4` half of the two-TOC
split; found by searching `lwz ..,-0x7d3c(r2)`, three hits, one in game code). It is a
per-ship **should this state be sent** test over a record at `param + slot * 0x24 + 0x10`,
compared against the last-sent record at `param + slot * 0x28 + 0x160`: it returns 1 (and,
when the debug flag `param+0x2c0` is set, prints why) on a time overrun
(`"Send due to time"`), a position error beyond `param+0x2a4`, a rotation error beyond
`param+0x2ac`, or a **flag-word change**. The flag word is at record `+0x2c`:

- bit `0x400` changed: prints the next string (`-0x7d40(r2)`, the other replicated boolean).
- **bit `0x200` changed: prints `m_overMagStrip` with `(flags >> 9) & 1`.**

So the replicated field `m_overMagStrip` is **bit 9 of the ship's network flag word** on
HD (the PS4 packs the same boolean as bit 1 of a byte at `+0x41` of a `0x21`-byte record,
`FUN_012ed9b0`). Both are filled from the ship's own over-the-strip flag; on the PS4 that is
`controller+0x5d0`, written by `FUN_0131b510` as `(probe hit a triangle of surface type 3)`
- the same literal Pulse's `craft+0x240` uses. The HD writer of the flag word was **not**
located (the HD analogue of `FUN_012ed9b0` is unread), so on the PS3 this page confirms the
field's existence, its width and that it is replicated, not its producer: the producer
claim rests on the PS4 and Pulse (85).

### Hull anchors, HD (conf 90)

All 39 `data/ships/<hull>/locators.vex` carry exactly one `arc_anchor_point` node (class
id `110`, on the centreline), and **match the Omega copies hull for hull**; the table and the
`#[ignore]`d test (`crates/game/tests/magstrip_anchor_ground_truth.rs`) are on the Omega
page.

## 2026-10-05, magstrip-hd-measure lane: HD's own arc pool, read and measured live

Static reading of `EBOOT.elf` plus **two RPCS3 boots** (`rpcs3-drive.py capture --region`,
`data/scratch/magstrip-hd-measure/live1`, `live2`; Talon's Junction, Fury grid cell). The
vtable slots earlier assumed to hold the arc update were not it:

| Address | Name | Conf | What it is |
| --- | --- | --- | --- |
| `0x001095e0` | `MagstripWake_DeletingDestruct` | 80 | resets the vtable, decrements the live-instance counter, frees the two ribbons (`+0x50`, `+0x54`) and the arc pool (`+0x58`), then `FUN_006761f8(this)` (free) |
| `0x00109720` | `MagstripWake_Destruct` | 80 | the same body without the free |
| `0x00109858` | `MagstripWake_UpdateRibbons` | 70 | calls the pool's `0x002bc7b0` (gated on `ship+0x5f43 == 0`) and the ribbons' `0x002a8f58`, then offsets the ribbons by `+/- clamp(speed - [0], [1], ...) * [2]`: the PS4 ribbon law, a second source for it |
| `0x002bbd60` | `MagstripArcs_Update` | 85 | nine slots of `0xa0` bytes: age, `0.85` smoothing, shed, jitter |
| `0x002bb530` | `MagstripArcs_Spawn` | 75 | picks the first free slot, sets life, contact scale, spread (the decompiler stops at bad data; the disassembly was read) |
| `0x002bc7b0` | `MagstripArcs_Draw` | 65 | VMX vertex build: 40-byte float vertices, position, uv at `+0x10/+0x14`, RGBA floats at `+0x18..+0x24` |

### The tuning block is initialised data, and it is read, not a `kIntensity`

All the arc constants live in one table of 22 floats at `0x008c2610` (TOC slot `0x6380`),
**initialised in the image, not run-time**, and the same bytes read back from live memory on both
boots (so the PS4's runtime-zero `DAT_020e52a0` has no HD counterpart for these):

| Offset | Value | Used by |
| --- | --- | --- |
| `+0x00,+0x04` | `0.2`, `1.1` | arc life (`0.2 + 0.9u`) |
| `+0x08` | `0.4` | start reach (`START_REACH`) |
| `+0x0c,+0x10` | `3.0`, `5.0` | slow-speed reach |
| `+0x14,+0x18` | `15.0`, `22.0` | fast reach; also the shed radius `lerp(15, 22, 0.6) = 19.2` (`368.64` squared) |
| `+0x1c,+0x20` | `10.0`, `200.0` | the speed blend's ends: `t = (speed - 10) / 190` |
| `+0x24` | `0.8` | strip half width (`HALF_WIDTH`) |
| `+0x28` | `0.55` | ahead draw offset (`0.55 + u`) |
| `+0x2c,+0x30` | `0.125`, `0.2` | **body brightness sample range** |
| `+0x34` | `0.3` | **body vertex alpha** |
| `+0x38,+0x3c` | `0.05`, `0.1` | **contact brightness sample range** |
| `+0x40` | `0.25` | **contact vertex alpha** |
| `+0x44` | `0.4` | read by the draw/update setup (unresolved) |
| `+0x48,+0x4c` | `3.5`, `7.0` | contact scale |
| `+0x50,+0x54` | `0.6`, `0.8` | spread: `0.6` slow, `0.8` fast |

Agrees with the PS4 reading on life, reach, speed blend, scale, spread, shed radius and the
`0.85`/`0.15` smoothing (`x = 0.85 x + 0.15 sample`, from TOC `0x63a4`/`0x63a8`).

**Disagrees on brightness and alpha** (conf 80, static, table confirmed live): the PS4 reading
has body brightness `0.0862u + 0.01875` (sample up to `0.7`) and a fixed alpha `0xb2` in a
`0..255` byte colour; HD's sample is `0.125..0.2` (smoothed mean `0.1625`), the contact's
`0.05..0.1` (mean `0.075`), and the vertex is **float RGBA with alpha `0.3` (body) and `0.25`
(contact)**. HD's `MagstripArcs_Spawn` never writes the brightness fields, so a slot's brightness
ramps from whatever the slot held (`0.85^n`, about 15 ticks to settle). HD names no `kIntensity`
(string search: zero hits); the PS4's `DAT_020e52a0 + 0x1e0` has no HD analogue found.

### The jitter scales: measured, live (conf 90)

`FUN_00676fe8` is `_FSin`. Both class initialisers (`0x002bd750`, `0x002bde18`) store, at
`0x00ad8984 + 4k`, `sin(arg_k) * 0.6 + 0.4` with `arg = 0, pi/4, pi/2, 3pi/4, pi`. **Read back
from RPCS3 on two cold boots, in race: `0.4, 0.824264, 1.0, 0.824264, 0.4`** (the same bytes at
three captures), term for term what `MagstripArcs_Update` multiplies the five jitter terms by.
The PS4's `DAT_02134210..20` is almost certainly the same table (not read there).

`MagstripArcs_Update` also pairs the draws: each tick it draws a fresh offset
`spread * (2u - 1)` for terms 0, 2 and 4; term 1 reuses term 0's draw and term 3 reuses
term 2's, unless `rand() & 7 == 0` (1 in 8) draws a fresh one (conf 85).

### What a frame shows (not a number)

`data/scratch/magstrip-hd-measure/run1.mp4` (RPCS3's recorder, 30 fps, `rpcs3-drive.py record`),
frames 31-36 of the 28 s window against ours at ticks 1136-1141 (`ours_consec.png`,
`orig_consec.png`): the original's arcs look several times wider and whiter on screen than ours,
and wavier (smooth, not zig-zag). **This is not a matched pose**: the original is ~0:18 on the
lavender grid with the default walk's hull, ours is tick 1136 over the dark floor with `feisar_c1`,
and additive arcs on a bright floor plus HD's bloom look different from the same arcs on a dark one. The world widths agree
(`+0x24 = 0.8`), so the gap is not geometry: the fragment program `MagStripArc_fp` (compiled
into the executable, no file on the disc) and HD's bloom are the open candidates. **No intensity
can be measured from these frames**, since there is no arc-free frame at the same pose.
