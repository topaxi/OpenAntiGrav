# The engine light: `EngineLightData.xml` feeds a vertex light per craft

Read 2026-11-16, lane `engine-glow-re`, on the **1.07 patch** eboot (Ghidra program
`/omega/eboot-ps4-omega-eu-v107.bin`; Omega addresses on this page are patch addresses unless marked
**base**, see [patch-v107-addresses.md](patch-v107-addresses.md)), on 2048's v1.04 executable
([its page](../vita-2048-eu-v104/engine-light.md)) and against HD's
`EngineFlare_SubmitSpuLight` ([renderer.md](../ps3-hdfury-eu/renderer.md), "the orange family's true
call site"). Static reading only: the live check on the native port did not run (see "Live check").

## Summary

| Question | Answer | Confidence |
| --- | --- | --- |
| What reads `Distance` / `Radius` (`ship+0xbc` / `+0xc0`) | `EngineFlare_SubmitVertexLight`, `0x0173e350` (**base** `0x016f5a00`), once per craft per frame | 85 |
| What it makes | one record in a 32-slot per-frame light table (`0x02047e60`, count `0x020482e0`): a point light with a position, a range and a colour | 85 |
| What the light is | not a sprite and not a material term of the flare: a **vertex light**. Each draw picks up to six table records that overlap its bounds and binds them as `DiffuseVertexLight<N>position` / `DiffuseVertexLight<N>diffuse` | 85 for the pick and the names, 60 for the Omega shader's falloff (read on 2048's program, not on Omega's GCN) |
| Colour source | constants in the executable, picked by three ship facts: the team's `GameVersion`, the Fury skin name, and (2048 modes only) the mag-strip contact. No per-team or per-file colour | 85 |
| Strength at standstill | the base colour times `1.0` (the boost blend is `0`), range `Radius` plus a per-tick jitter | 80 static; HD's live capture shows the shared branch at exactly `(40, 10, 4)` at rest |

The seven float readers of `[reg + 0xbc]` the previous lane listed are not it: `FUN_01083a10` and
`FUN_01083d80` are an audio filter coefficient (`x * -2 pi / sample rate + 1`), `FUN_0177e140` is the
music fade and spectrum, `FUN_0103c000`, `FUN_012cb090`, `FUN_012d9190` and `FUN_012de2d0` read their
own objects' `+0xbc`. The consumer loads `Distance` with `VBROADCASTSS` (`0x0173e4bc`), which a
`VMOVSS` search does not find.

## The producer: `EngineFlare_SubmitVertexLight` (`0x0173e350`)

`this` is the engine flare object (`E:\WOPS4\Wipeout\Code\System\Render\EngineFlare_Importer.cpp`);
`this+0x118` is its ship, `this+0x68` the flare node's world matrix. Constants are one block at
`0x01863abc` (**base** `0x018053fc`), read as bytes:

| Address | Value | Use |
| --- | --- | --- |
| `0x01863abc` | 3.0 | 2048-era red; mag-strip blue |
| `0x01863ac0` | 0.5 | 2048-era green |
| `0x01863ac4` | 0.1 | 2048-era blue; mag-strip green |
| `0x01863ac8` | 5.0 | 2048-era tint add |
| `0x01863acc` | 2.2 | 2048-era boost gain |
| `0x01863ad0` | 4.0 | HD-era low channel |
| `0x01863ad4` | 40.0 | HD-era high channel and tint add |
| `0x01863ad8` | 10.0 | HD-era green and boost gain |
| `0x01863adc` | 1.5 | mag-strip red |
| `0x01863ae0` | 0.2 | mag-strip gain scale |
| `0x01863ae4` | 1.0 | |
| `0x01863ae8` | 0.01 | least range submitted |

```text
ship  = this+0x118
if ship+0x8ba1:                         ; the team's GameVersion is "2048"
    base = (3.0, 0.5, 0.1); gain = 2.2; add = 5.0
elif ship+0x8ba0:                       ; Fury skin
    base = (40, 10, 4);     gain = 10;  add = 40
else:
    base = (4, 10, 40);     gain = 10;  add = 40

t      = this+0x260                     ; transition countdown
tint   = t > 0 || this+0x34c
strip  = mode id (0x0201280c) >= 0x17 && ship->vtable[0xd0](ship) && !tint
if strip:
    base = (1.5, 0.1, 3.0); gain *= 0.2

if !tint:
    blend  = this+0x13c + this+0x130    ; the boost blends, 0 at rest
    colour = base * (1 + gain * blend)
    calm   = 1 - blend
else:                                   ; the transition tint, per the HD page
    t >= 1:  colour = (base.r + add, base.g, 0)    ; (44, 10, 0) classic, (80, 10, 0) Fury, (8, 0.5, 0) 2048-era
    else:    colour = (low + add * t, base.g, high * (1 - t)), t clamped to 0..1
    calm   = 1

anchor   = node.row3 - node.row2 * Distance        ; ship+0xbc, behind the nozzle along the node's Z
off      = this+0x270                              ; vec4 (x, y, 0, w), redrawn every flare update
position = anchor + off.xyz * calm
range    = Radius + off.w * calm                   ; ship+0xc0
if range > 0.01 && count < 32:
    table[count++] = { position, 1 / range, colour, range, id = -1 }
```

`high`/`low` are the 40 and 4 of the HD-era bases (3.0 and 0.1 for the 2048 era), whichever skin.

Evidence, per term:

- **The read is unambiguous (90).** `*(float *)(ship + 0xc0)` and `*(float *)(ship + 0xbc)` are loaded
  off `*(this + 0x118)` at `0x0173e47e`-`0x0173e4bc`; the loader `FUN_01335030` stores `atof` of
  `Distance` and `Radius` there.
- **`ship+0x8ba1` is "the team's `GameVersion` is 2048" (90).** The ship loader `FUN_013354e0` writes
  `*(int *)(*(ship + 0x6e08) + 0x12c) == 1` (`0x013375c6`-`0x013375d4`); `ship+0x6e08` is the team
  record matched by name. `FUN_014ae370` parses the `GameVersion` attribute and stores
  `strcasecmp(value, "2048") == 0` at record `+0x12c`. `Data\plugins\teams\Definition.xml` authors
  `gameversion="2048"` on exactly the five `*2048` teams. The same byte picks the flare sprite:
  `FUN_01739980` loads `data/Tex/EngineFlare/Engine_Flare_Rich_red.gnf` when it is set and
  `Engine_Flare_Rich.gnf` otherwise.
- **`ship+0x8ba0` is the Fury skin (90).** `FUN_01334a30` sets it when the model name
  (`*(ship + 0x6e10) + 0xb8`) is `chrome_c1`, `nitro`, `concept1`, `vr_fury` or contains `detonator`.
  It is HD's `Ship_SetFuryTrailFlag` with `vr_fury` added.
- **The virtual at `+0xd0` is the mag-strip predicate (75).** All five ship vtables
  (`0x0197db90`, `0x01996970`, `0x01996bb0`, `0x01996d88`, `0x01997178`) hold `FUN_01310580` or its
  wrapper `FUN_017b7b10` there. Its body is the shape [ships-effects.md](ships-effects.md) records for
  the base build's `Ship_IsOverMagStrip` (slot kind 0 or 2 reads a byte on the controller object, here
  `controller+0x5e0`; a remote ship reads bit 1 of the replicated byte at `+0x41 + slot * 0x21`). The
  slot moved from `+0xc8` to `+0xd0` and the byte from `+0x5d0` to `+0x5e0` between the builds; the
  match is by shape, the 1.07 writer of `+0x5e0` was not read.
- **Mode ids from `0x17` up are the 2048 game modes (75)**, per
  [race-hud-selection.md](race-hud-selection.md).
- **The jitter (75).** `FUN_017366c0` (the flare's update) writes `this+0x270` at `0x017384f5` from
  three draws of the engine's lagged generator (`0x02102e50`), scaled by `0x02102e9c`, which
  `FUN_0167a020` sets from `1.0` (`0x018636e0`) and an exponent of `-32`: `x, y = r * 0.4 - 0.2`
  (`0x01863a18`, `0x01863a1c`) and `w = r * 2 - 1` (`0x01863a20`), so the position wanders 0.2 units
  either side and the range **one unit** either side. HD's are 0.1 each. The same instruction block
  decrements `this+0x260` by the frame time.
- **The tint is armed by the flare's own state change (60).** `FUN_0173c810` writes
  `this+0x260 = 3.3` (`0x40533333`) and clears `this+0x34c` at `0x0173e1a7`, on the path where
  `this+0xb4` differs from a three-way class of its float argument (thresholds `0x01863ab4`,
  `0x01863ab8`), and sets `this+0x34c` at `0x0173cec9`. Which input that argument is was not read.
  This is the arming the HD page left open.
- **Who calls it (70).** Only `FUN_01307d40` (`0x0130843a`), slot 5 of the ship vtables, when the ship
  has a flare (`ship+0x69c8`) and its state value is below 5.

### At standstill

`blend` is `0` (`this+0x130` is zeroed by `FUN_01739980`; HD's live capture reads the shared branch
as exactly `(40, 10, 4)` on the grid), so the colour is the base and the jitter runs at full size:

| Craft | Colour | `Distance` / `Radius` authored | Range per tick |
| --- | --- | --- | --- |
| HD-era, classic skin | `(4, 10, 40)` | -0.4 to 1.2 / 1.0 to 2.0 | `Radius` +- 1.0 |
| HD-era, Fury skin (`concept1`, `nitro`, `chrome_c1`, `vr_fury`, `detonator`) | `(40, 10, 4)` | same | `Radius` +- 1.0 |
| 2048-era (`GameVersion="2048"`) | `(3.0, 0.5, 0.1)` | 3.0 / 30.0 (all twenty `ships\<team>2048\<n>` files) | 29 to 31 |
| any craft over a mag strip in a 2048 mode | `(1.5, 0.1, 3.0)` | its own | its own |

This is why HD's law washed 2048's circuits white: a 30-unit light drawn at `(4, 10, 40)` instead of
`(3.0, 0.5, 0.1)`. The 2048-era light is a wide, dim red one three units behind the nozzle that
reaches the floor (ride height is about 4 units); the HD-era one is a tight, strong one at the nozzle
that reaches only the housing.

## The table and who receives it

- **Record, `0x24` bytes (85):** `+0x00` position xyz, `+0x0c` `1 / range`, `+0x10` colour rgb,
  `+0x1c` range, `+0x20` an owner id (`-1` = every view). 32 records at `0x02047e60`, count at
  `0x020482e0`, a zero record at `0x020482e4`. `FUN_0162a390` clears the count once a frame
  (`0x0162d3ef`).
- **Other producers.** 21 more functions append to the table (`get_xrefs_to(0x020482e0)`: the
  `0x0134e670`-`0x013a03e0` weapon and effect range, `FUN_012ecc90`, `FUN_01322370`, `FUN_01767a00`,
  and `FUN_01307d40` itself, which appends one more light per ship before the flare's). None was read.
- **The pick (85).** `FUN_0165b890(sphere, &count, view_id)` walks the table in order and keeps the
  records whose owner is the view or `-1` and whose axis-aligned box (`position +- range`) overlaps the
  draw's bounding sphere, stopping at six; `FUN_0165b760` is the same over a box and takes only owner
  `-1`. A count of 3 or 5 is padded to 4 or 6 with the zero record. Callers: `FUN_0127a130`,
  `Scene_PrepareFrame`'s 1.07 twin `FUN_015f2d30`, `FUN_01606410`, `FUN_01610fa0`, `FUN_01618bb0`.
- **The bind (80).** The caller copies the first `0x20` bytes of each picked record into the draw's
  constant block as two vectors, `(position, 1 / range)` and `(colour, range)`
  (`FUN_0127a130`, after `0x0127b6b6`). The executable names twelve constants,
  `DiffuseVertexLight0position` to `5position` and `0diffuse` to `5diffuse` (`0x0188e249`-`0x0188e475`).
- **Which draws (55).** The caller's loop takes a submesh only when `(submesh+0x24 & 7) != 0`, which
  reads as the number of lights its program declares. On 2048 the uniform is declared by 24,291
  programs: every environment's materials and the ship materials
  ([2048's page](../vita-2048-eu-v104/engine-light.md)). So track and hull both receive it.
- **The falloff (2048: 80, Omega: 60).** 2048's vertex program computes, per vertex,
  `diffuse * max(0, N . L) * max(0, 1 - d * position.w)` with `position.w = 1 / range`: a linear
  falloff to zero at the range, with a Lambert term. Omega's record carries the same `1 / range` in
  the same place under the same constant names, so the same law is the reading; Omega's GCN programs
  were not disassembled.

## Cross-title

- **2048: checked, applies, not wired.** The same function is in 2048's own executable with the same
  constants, the same three colour branches and the same record
  (`EngineFlare_SubmitVertexLight`, `0x812726d6`, [page](../vita-2048-eu-v104/engine-light.md)).
  One difference: the mag-strip branch is gated by a call (`FUN_81000930(0x8153fc40, -1) == 0`) where
  Omega tests the mode id; that call was not read. 2048 names no `Engine_Flare_Rich_red` sprite.
- **HD: checked, differs.** HD has the HD-era half only (`EngineFlare_SubmitSpuLight`,
  `0x0029ff28`): the two bases, gain 10, the tint, jitter 0.1 on every axis, and a different receiver
  (the SPU vertex-light list). No `GameVersion` branch and no mag-strip branch. Whether HD scales the
  jitter by `1 - blend` was not re-read. HD's is wired (`oag_raceplay::engine_light`).
- The live HD capture (40 records, all `(40, 10, 4) * (1 + 10 * blend)`) is the check on the branch
  the three executables share.

## Live check: not run

The native port (`wipeout_pc`) no longer starts on this workstation: its binary links
`libfmt.so.9`, `libZydis.so.4.1` and `libZycore.so.1.5`, and the host now ships `fmt` 12. Rebuilding
the port was outside this lane. The read that would confirm the page, once it runs: on the Vineta K
grid, sample `0x484` bytes at Ghidra `0x02047e60` (runtime = image base + address - `0x01000000`)
a few hundred times and keep the samples with the highest count at `+0x480`; the grid's eight craft
should each show one record with `1 / range` at `+0x0c`, a colour from the standstill table at
`+0x10` and `-1` at `+0x20`. The count is cleared and refilled every frame, so most samples are
partial.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x016f5a00` (**base**; `0x0173e350` on 1.07) | `EngineFlare_SubmitVertexLight` | 82 | Decompiled on 1.07 with every constant read as bytes; 2048's executable carries the same function; the shared branch is HD's live-captured law. The base address is the function whose first 24 bytes equal 1.07's (`55 48 89 e5 41 57 41 56 41 54 53 48 83 ec 20 49 89 fc 49 8b bc 24 18 01`) and whose RIP-relative loads hit the base constant block `0x018053fc` at the same offsets on both builds (`+0x37`, `+0x74`, `+0x7c`, `+0x9f`, `+0x10c`); its body was not decompiled on the base build, hence 82 and not 85. |

Unnamed, with their readings above: `FUN_0165b890` and `FUN_0165b760` (the pick), `FUN_01334a30`
(the Fury skin flag), `FUN_01335030` (the loader; the base build's is `FUN_0130e090`), `FUN_017366c0`
(the flare update), `FUN_0173c810` (arms the tint). They are 1.07 addresses with no base twin located,
which `names.tsv` does not key on.
