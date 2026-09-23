# HD's shield hexagon, lap arc and position arc: what the runtime writes

2026-09-23. Opened to answer one question the layout could not: the in-race
HUD draws `DamageBar`, `DamageBarBg`, `ShieldBarText`, `LapBar0`-`6` and
`PosBar0`-`7` differently from what `HUD_damage_indicator.xml`,
`HUD_lap_counters.xml` and `HUD_positions.xml` author (see
[hd-hud.md](../../../formats/hd-hud.md#what-is-not-done)), and every copy of
those files on the disc was already checked. The answer is three per-tick
update functions, found from the widget names outward.

Read [race-hud.md](race-hud.md) for `Hud_LoadDefinition` and
[hud-sight.md](hud-sight.md) for `Hud_BindWidgets` first. Everything here is
below `0x32d5e0`, where Ghidra's own TOC (`0x008ad4d8`) is the right one -
`scripts/ps3-toc.py toc` prints `exact` for `0x000866c8`, `0x0009bb30` and
`0x000cf490`. Disassembly was cross-checked with `llvm-objdump` against the
decrypted `EBOOT.elf` wherever the decompiler's output is quoted below.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0009e3d0` | `Hud_Update` | 72 |
| `0x000866c8` | `Hud_UpdateShieldReadout` | 82 |
| `0x00096ef0` | `Hud_UpdateLapCounter` | 80 |
| `0x00096088` | `Hud_UpdatePositionCounter` | 78 |
| `0x00200468` | `Text_SetColour` | 72 |

`Text_SetColour(text, argb, flag)` is four instructions: it stores `argb` at
`text+0xc4` and sets or clears bit 0 of `text+0x80` on `flag`. Every call site
below passes a literal ARGB word a `<Text>` widget is then drawn in - the text
counterpart of `Image_SetVertexColours` ([menu-blocks.md](menu-blocks.md)).

## Finding them: the bind offsets, then who reads them

`scripts/ps3-toc.py attrib` attributes all seven strings
(`DamageBarParent`, `DamageBar`, `DamageBarBg`, `DamageBarShieldBg`,
`ShieldBarText`, `LapBar%d`, `PosBar%d`) to `Hud_BindWidgets` (`0x0009bb30`)
and nothing else. Their TOC slots (`0x008a7a64`..`0x008a7a70`, `0x008a7b90`,
`0x008a7b94`) sit at `-0x5a74`..`-0x5a68`, `-0x5948`, `-0x5944` off the TOC, and
each `lwz r4,<disp>(r2)` in the bind is followed by a call to the
resolve-by-name helper `0x0067e6d0` and a `stw r3,<offset>(r30)`:

| Widget | HUD object offset | Bind site |
| --- | --- | --- |
| `DamageBar` | `+0x1b0` | `0x0009be7c` |
| `DamageBarBg` | `+0x1b4` | `0x0009be90` |
| `DamageBarShieldBg` | `+0x1bc`, **hidden at bind** (bit `0x4` of `+0x34` cleared) | `0x0009bf30` |
| `ShieldBarText` | `+0x1cc` | `0x0009bf64` |
| `nrgBarWo3` | `+0x190` | `0x0009bdf4` |
| `LapBar0`-`LapBar6` | `+0x2b4`..`+0x2cc`, each hidden at bind | `0x0009dc2c` loop |
| `PosBar0`-`PosBar7` | `+0x2d0`..`+0x2ec`, each hidden at bind | `0x0009d9e8` loop |

The same bind reads four floats off `DamageBar` for later: its vtable `+0x60`
getter into `+0x1dc` (the height), the `+0x74` getter into `+0x1d8` (its `y`),
and the widget's own `+0xd0`/`+0xd8` into `+0x1e0`/`+0x1e4` (source `V` and
source height). And it sets `+0x6a0`, a byte the shield colour reads below,
from `+0x6bc` - which it loads from `g_GameState+0xe0`, the mode id
([mode-manager.md](mode-manager.md)):

```
0009d1dc: lwz 0,1724(30)      ; +0x6bc, the mode
0009d1e0: cmpwi 7,0,13 / 21 / 8 / 20 / 14   -> +0x6a0 = 1
0009d208: li 0,0 ; stb 0,1696(30)            otherwise +0x6a0 = 0
```

A byte-offset search over the whole disassembly (`lwz rN,432(`, `436(`,
`460(`, `692(`, `720(`) then leaves exactly one reader of each set outside
the bind: `0x000866c8` for the shield trio, `0x00096ef0` for `LapBar*`,
`0x00096088` for `PosBar*`.

## They run every tick, from `Hud_Update`

All four are called from one function, `0x0009e3d0`, which calls
`Hud_BindWidgets` at `0x0009e454` and then dispatches on bits of `hud+0x44`:

```
0009e81c: rlwinm 0,11,0,30,30   ; bit 0x02 -> bl 0x866c8  Hud_UpdateShieldReadout
0009e828: rlwinm 0,11,0,29,29   ; bit 0x04 -> bl 0x96ef0  Hud_UpdateLapCounter
0009e8b4: rlwinm 0,11,0,25,25   ; bit 0x40 -> bl 0x96088  Hud_UpdatePositionCounter
```

each called as `f(dt, hud)` (`fmr 1,27 ; mr 3,27`). Which modes set which bit
of `hud+0x44` is unread; the three HUD layouts that author these widgets are
the obvious reading, and nothing contradicts it. 72 on `Hud_Update` rather than
higher for that reason: the dispatch is read, the per-mode flag word is not.

## `Hud_UpdateShieldReadout` (`0x000866c8`)

### Constants

All read directly (`scripts/ps3-toc.py u32`, big-endian IEEE-754):

| Slot | Value | Role |
| --- | --- | --- |
| `0x008a75c4` | `100.0` | shield to percent |
| `0x008a75b8` | `0.0` | floor of the percent; "timer idle" |
| `0x008a75bc` | `1.0` | one second; the fill's `1 - f` |
| `0x008a76c0` | `0.01` | percent to fraction |
| `0x008a764c` | `0.5` | half the height, for a centred widget |
| `0x008a7678` | `20.0` | the critical threshold, in percent |
| `0x008a7704` | `8.0` | flash rate: phase is `floor(t * 8)` |
| `0x008a76f8` | `-> "%2.1f"` | the `DamageBarShieldBg` variant's format |

### What it does, in order

`pct = max(p.shield * 100 / p.max, 0)`, `p` being the per-player record at
`hud+0x40` (`+0x48` and `+0x50` the two pool fields; the same record carries
the lap and place the two functions below read), and `f = pct * 0.01`.

1. **`DamageBar` is cropped from the top, bottom edge fixed.** Height
   `+0xb0 = f * H`; `y = y0 + (1 - f) * H * 0.5` through the vtable `+0x70`
   setter (the widget is `Centred`, so moving its centre down half the lost
   height pins the bottom); source height `+0xd8 = f * TH`; source
   `V = V0 + (1 - f) * TH`. `0x00086750`-`0x000867ac`. Not clamped above 1.
2. **A post-hit window.** `+0x10c` holds last frame's percent, `+0x110` a
   timer. If `(int)pct < (int)+0x10c`, or the timer is already above zero, the
   timer gains `dt` and wraps to zero once it reaches `1.0`
   (`0x000869a8`-`0x000869d8`). The comparison is on **`fctiwz`-truncated
   integers**, so a drop inside one whole percent does not arm it.
3. **A ship-side condition, unresolved.** `ship` is the viewing player's craft,
   found through `RaceManager_GetInstance`'s eight-slot lookup (the same
   boilerplate [hud-sight.md](hud-sight.md) describes). `c = 0x000cf490(ship) ||
   ship+0x6958 != 0`, where `0x000cf490` returns `0.0 <= ship+0x6a30 - ship+0x6a80 <= 1.0`,
   that is, within one second of some ship event. Which event is not read; left
   unnamed on purpose.
4. **The colour.** `rgb = +0x6a0 ? 0xFFFFFF : 0x1664FF`, computed branch-free:

   ```
   00086a50: lbz 9,1696(31)          ; +0x6a0
   00086a54: lis 0,-234 ; ori 0,0,25856     ; 0xff166500
   00086a60: addi 9,9,-1 ; srawi 29,9,31    ; all-ones iff the byte is 0
   00086a70: and 29,29,0
   00086a78: addis 9,29,256 ; addi 29,9,-1  ; + 0x00ffffff, mod 2^32
   ```

   `0xff166500 + 0x00ffffff = 0x1001664ff`, which truncates to `0x001664FF` -
   the same "HD blue" `zone_hud.xml` authors on its own `DamageBar`. White in
   modes 8, 13, 14, 20 and 21 (SPElimination, 13, Detonator, MPElimination,
   21 - see [mode-manager.md](mode-manager.md) for which of those ids are
   assigned); blue in every other.
5. **Steady or flashing.** Flashing when `pct <= 20`, or the step-2 timer is
   running, or `c`. Flashing advances a second timer, `+0x1e8`, by `dt`; the
   phase is **on** when `(long)(t * 8)` is even, and the timer wraps to zero
   once it exceeds `1.0`. So the blink is 0.125 s on, 0.125 s off. The timer
   keeps its value between flashing episodes; nothing resets it on entry.
6. **The writes** (`0x00086a88`-`0x00086dd8`), `on` being the phase and `A`
   being `0xFF000000` when on and `0` when off:

   | State | `DamageBarBg` | `DamageBar` | `ShieldBarText` |
   | --- | --- | --- | --- |
   | steady | `0xFFFFFFFF` | `0xFF` + rgb | `0xFF` + rgb |
   | flashing, not `c` | `0xFFFF0000` when on, else `0xFFFFFFFF` | `0xFF` + rgb | `A` + rgb |
   | `c` | `0xFFFFFFFF` | `A` + rgb | `A` + rgb |

   `DamageBarBg` is always written `0xFFFFFFFF` first (`0x00086a88`), then
   overwritten red on an "on" frame (`0x00086dc0`:
   `(0xFF000000 | 0x00FF0000) & 0xFFFF0000`). Separately, `DamageBar+0x10c` is
   written `0xFF` in the `c` state and `0` otherwise; what that field is was not
   read.
7. **`nrgBarWo3` overrides all of it** when bound (`+0x190 != 0`, the `wo3`
   skin): `DamageBar` and `DamageBarBg` go to `0`, the bar is tinted
   `0x4CFFFFFF`/`0xBFFFFFFF` and the text white, with red (`0x4CFF0000`,
   `0xFFFF0000`) and green (`0x4C00FF00`, `0xFF00FF00`) variants on the same
   two conditions. Not a default-skin path.
8. **The digits.** When `DamageBarShieldBg` is null or hidden - which it
   always is on the default skin, hidden at bind and in no path's shipped copy
   (see [hd-hud.md](../../../formats/hd-hud.md#what-is-not-done)) - the text is
   `(int)pct` as up to three ASCII digits, **no `%`**, written through the
   string setter `0x00202d28` (`0x00086ba4`-`0x00086c54`, decimal by
   `mulhw` against `0x51eb851f`/`0x66666667`). The other branch formats
   `+0x1c4` with `"%2.1f"` and colours the text white in modes 13 and 21,
   `0xFF30FF30` otherwise. Then `+0x10c = pct`.

**Confidence 82** on the table in step 6, the colour in step 4 and the crop in
step 1: every constant is a direct read, the branch structure was walked in the
disassembly as well as the decompile, and the three `talons-matched` frames of
the running original agree with the steady row - a full blue hexagon and a
blue `100`/`98`. What stays open: the field meanings behind `c` (step 3), and
the `+0x44` bit that gates the call.

## `Hud_UpdateLapCounter` (`0x00096ef0`)

`lap = hud+0x40 -> +0xc`, `laps = hud+0x40 -> +0x10`; the two `Text` widgets
at `+0x278`/`+0x27c` show `lap` and `laps` (the big digit and the small
"of"). With `laps < 1` every `LapBar` stays hidden and the `+0x280` widget is
hidden too. Otherwise, with `n = laps - lap`:

```
LapBar_k is shown  iff  n < k + 1   (k = 0..6; bit 0x4 of +0x34 set)
```

So `7 - n` segments are up, the **last** ones: `LapBar6` (bottom right) first,
`LapBar0` (top) last, one more lighting as each lap starts. At lap 1 of 3,
`n = 2`: `LapBar2`-`LapBar6`, five segments. Nothing in the function writes a
colour to any of them - **the yellow is the atlas's own**: `HUD_Components.gtf`
holds the segments baked yellow at `U=339`, which the layout addresses with `V`
counted from the top of the image while the decoded rows run bottom-up, so a
sampler that reads `V` as a raw row index sees an empty region instead (see
[hd-hud.md](../../../formats/hd-hud.md#posbar0-7s-lit-count-tracks-place-lapbar0-6s-is-confounded)).

`lap` is 1-based in the frame this was checked against: `talons-matched/00.png`
reads `1` over `3`, and shows exactly `LapBar2`-`LapBar6` yellow, `LapBar0`
and `LapBar1` absent. Confidence 80.

## `Hud_UpdatePositionCounter` (`0x00096088`)

Outside the two head-to-head modes (`+0x6bc` of 9 or 18, which take a separate
two-name path) and the `+0x44 & 0x800` results list:

```
PosBar_k is shown  iff  k <= 8 - place   (k = 0..7)
```

with `place = hud+0x40 -> +0x14`, and **`8` a literal, not the field size**.
So 8th shows `PosBar0` alone, 7th `PosBar0`-`PosBar1`, 1st all eight. Again no
colour write: yellow from the atlas. Checked against `talons-matched/00.png`
(8th: one short yellow segment at the bottom) and `03.png` (7th: two, side by
side along the bottom edge, where `PosBar0` and `PosBar1` both sit at
`y=155`). Confidence 78: two places checked of eight, and `place = 0`, which
the formula would light as nine, is a state the function never meets in the
original and this page does not claim to know.

## Not read

- **What `c` is** (`0x000cf490`, `ship+0x6958`, `ship+0x6a80`). Written by
  `0x000e93c0` and three siblings nearby; nothing was read about why.
- **Which bits of `hud+0x44`** each mode's HUD sets.
- **`0x0067e6d0`'s traversal order** - a hash-cached lookup
  (`0x0016d9a8`) falling back to a tree walk (`0x0016a920`). `arcade_hud.xml`
  composes two widgets named `DamageBar`, and which one this binds was not
  read from here; see [hd-hud.md](../../../formats/hd-hud.md) for why the one
  sharing `DamageBarBg`'s rectangle is taken as the bound one.
- **`DamageBar+0x10c`**, written `0xFF`/`0` in step 6.
