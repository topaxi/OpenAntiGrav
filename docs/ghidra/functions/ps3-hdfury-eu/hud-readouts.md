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
3. **A ship-side condition, `c`.** `ship` is the viewing player's craft,
   found through `RaceManager_GetInstance`'s eight-slot lookup (the same
   boilerplate [hud-sight.md](hud-sight.md) describes). `c = Ship_AbsorbWindowActive(ship) ||
   ship+0x6958 != 0`, where `0x000cf490` returns `0.0 <= ship+0x6a30 - ship+0x6a80 <= 1.0`:
   within one second of the **absorb** stamp, and `ship+0x6958` is a LeachBeam
   flag. Both are read in [the section below](#what-c-is-the-absorb-window-and-a-leach-flag).
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

## What `c` is: the absorb window, and a leach flag

2026-10-02. Names, with the evidence below them.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x000cf490` | `Ship_AbsorbWindowActive` | 75 |
| `0x000e9160` | `Ship_UpdatePickup_q` | 55 |
| `0x0013d7a8` | `LeachBeamManager_Update_q` | 65 |

**`ship+0x6a80` is the time of the last pickup absorb** (`ship+0x6a30` being the
craft's own clock, advanced at `0x000eaeb8`). Its writers are the constructors
(`-10.0`, so the window is closed at the start) and `0x000e9160`, which the
per-tick pickup step `0x000eabd0` calls after `0x000ea0c8` and `0x000e9cd0`.
`0x000e9160` returns at once without a held pickup (`*(ship+0x5edc) == 0`) and
otherwise dispatches on the slot object's state word (`*(ship+0x5edc)+0x204`, `-1`
empty) through two jump tables; four sites store `ship+0x6a30` into
`ship+0x6a80` (`0x000e93c0`, `0x000e9c0c`, `0x000e9c7c`, `0x000e9cc8`), two of
them reached through the `'B'`/`'Q'`/`'D'` network messages
`absorb-feedback.md` already met on the absorb path, and the fall-through that
follows the first calls `Ship_PlayAbsorbFeedback` (`0x000d9398`).

**Measured on the running original (live, AI craft).** Every craft runs
`0x000e9160`, so the eight craft were swept through the GDB stub every ~1.2 s
(`ship+0x6a30`, `+0x6a80`, `+0x6958`, `+0x7a5c`, the slot state) for 150 s and
200 s of a Talon's Junction race, two boots. A stamp appeared **three times**
and each time with `ship+0x7a5c` - the absorb feedback's timer, `1.0` counting
down, idle at `-ship+0x6a30` - at `1.0 - (ship+0x6a30 - stamp)` to within a
sweep (0.783 at 0.2 s after stamp `69.94`; 0.515 at 0.45 s; 0.466 at 0.51 s).
**Twelve other pickup slots cleared (a weapon fired, the state going to `-1`)
without a stamp and without `+0x7a5c` leaving idle.** The craft that stamped
held states 6, 6 and 0, so it is not a weapon type. That is the absorb
feedback starting, 3 of 3 and 0 of 12, not "a pickup was used"; the player's
own craft could not be made to absorb (no pad on the line in 35 s of
thrust - `circle`/`triangle`/`r1`/`l1`/`square` taps with an empty slot
changed nothing). Confidence 75: the correlation is clean and small.

**The visible effect agrees with the table in step 6, row `c`.** Writing
`ship+0x6a30` into `ship+0x6a80` on the player's craft (at 59.9 %) blinked the
fill and the number together for about a second, four frames on and four off,
with the background white throughout - no red in the bracket on any of 36 frames.

**`ship+0x6958` is set by the LeachBeam manager's per-tick update.**
`0x0013d7a8` sits beside `LeachBeamManager_Construct` (`0x0013d520`, 85), walks
the manager's beam list (`param+0x88` of them, each a `0xc9c0`-byte
`LeachBeam`), clears `*(beam+0xc94c)+0x6958` and sets it again to `1` for a beam
whose state word (`+0x4c`) is `4` with bit `0x1` of `+0x40`, while moving shield
between two craft (`+0x120` and `+0x128` of their pickup-slot objects). Which of
the two craft `beam+0xc94c` is - the one firing or the one beamed - was not
read, and so `ship+0x6958` is **not wired**. Confidence 65 for the name, 0 for
the owner.

## What the running original does at low shield

2026-10-02, a private RPCS3 (Recompiler), the Fury campaign's Talon's Junction
single race, Feisar, 1280x720 at 30 fps through the emulator's own recorder, the shield poked through the GDB stub.
The cells, found by scanning for the 140.0 the Feisar's pool starts at and
confirmed by writing 70.0 and reading the HUD's record follow within 0.5 s:

- **The local craft** is the slot of `rm+0xe8..0x104` (plus `rm+0x13e8`, the
  fallback) whose `ship+0x7a60` equals `hud+0x118` (0). `rm` is
  `[[0x008a6814]]`. The shield is **`ship+0x5fa0`** (f32). The HUD's record
  (`hud+0x40`, lap `+0xc`, laps `+0x10`, place `+0x14`, shield `+0x48`, maximum
  `+0x50`) is a mirror of it refreshed every tick, so a write to the record is
  undone within a frame - the first run poked it and saw nothing but a one-frame
  blank of the number.
- **The HUD object** (`hud`) sat at `0x32921a30` in four boots out of four, the
  interpreter's and three Recompiler's; the craft, the record and the shield
  cell moved by a few kilobytes boot to boot. Its `+0x10c`, `+0x110`, `+0x1e8`
  are the previous percentage, the post-hit timer and the flash accumulator.

| Question | Measured | Matches |
| --- | --- | --- |
| Flash rate | cycles per second at 30 fps over 22, 17 and 14 cycles (first boot): 3.95, 3.98, 4.00; over 14, 9 and 9 (second boot, `scripts/rpcs3-hud-probe.py schedule`): 3.96, 3.91, 4.03 | 8 phases a second |
| 20.0 % | flashes without end: red bracket and the number on, pale bracket and an empty plate off; the fill solid blue throughout | `<= 20` |
| 20.5 % | after the drop from 50 %: about 0.97 s of flashing (28 frames; 29-30 in the second boot), then a steady `20` for the rest of the 5 s | the post-hit window, no threshold flash |
| 15 %, 5 % | flashes, the digits `15` and `5` | |
| Rise to 60 % | the flash ends within a frame of the poke (30 fps, a poke freezes the clip, so a tick cannot be resolved) | |
| Post-hit arming | 60.9 to 60.2 %: `hud+0x110` stays `0.0`; 60.2 to 59.9 %: 0.20-0.234 a quarter second on; a rise arms nothing - in each of **three boots**, three reps a boot (first boot `0.2286`, `0.2298`, `0.2330`) | **`fctiwz`-truncated whole percent** |
| Absorb | fill and number blink, background white | row `c` |

`scripts/rpcs3-hud-probe.py arming` reproduces the arming rows (`schedule` films
the rest, `sweep` reads the absorb stamps).

The flash accumulator `hud+0x1e8` was read at 0.9833, 0.9574, 0.9395 and 0.9269
when a flash ended and picked up from there on the next (the values are read; that
a flash starting at 0.98 therefore starts on an "off" phase, `floor(7.9)` being odd,
is arithmetic on them - no recorded frame isolates a flash's first phase), so
**the phase at which a flash starts is the accumulator's, not the race clock's**.

Only the `arming` mode of the probe script has been run end to end three times;
`schedule` once (the second boot above); `sweep` is the `arm2.py` sweep that the
absorb reading comes from, folded into the script and not re-run from it.

## Not read

- **Which craft `beam+0xc94c` is** (above), so `ship+0x6958` is not wired.
- **Why the absorb stamp has four writers** - which pickup states reach which,
  and the two jump tables' cases. The player's own absorb was not seen.
- **Which bits of `hud+0x44`** each mode's HUD sets.
- **`0x0067e6d0`'s traversal order** - a hash-cached lookup
  (`0x0016d9a8`) falling back to a tree walk (`0x0016a920`). `arcade_hud.xml`
  composes two widgets named `DamageBar`, and which one this binds was not
  read from here; see [hd-hud.md](../../../formats/hd-hud.md) for why the one
  sharing `DamageBarBg`'s rectangle is taken as the bound one.
- **`DamageBar+0x10c`**, written `0xFF`/`0` in step 6.
