# Front-end pulse clocks: the `Selector` phase and the `Text` glow ladder

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base `0x00100000`.
Read 2026-11-24 (`ps2-pulse-clock`): a headless decompile into a scratch project, then PCSX2
2.x over PINE on the EU disc's `grid-selection` and `cell-selection` savestates. The PSP map is
[`../psp-pulse-usa/race-campaign.md`](../psp-pulse-usa/race-campaign.md) and
`docs/ui/campaign-screens.md` ("The `Selector` pulse", "The card title's ladder is its own
counter").

## What to take away

Grid and Cell Selection pulse two things on **two counters**, as on the PSP:

| Counter | Where | Law | Ours |
| --- | --- | --- | --- |
| `Selector` phase | `float` at `GridController + 0xa0` (one controller per screen) | `+= 2 dt`, wrapped at `2.0` | `--menu-picker-seconds = phase / 2` |
| title glow ladder | byte `s` at `Text + 0xed`, direction in bit 31 of the word at `Text + 0xec`, the low byte of that word the tick last stepped to | one step of 15 per tick of `*(0x002e3640) + 0x40` times 20, 0 up to 150 and back | not passed: no step of it reaches the frame (below) |

`*(0x002e3640) + 0x40` is the front end's clock in seconds (read live: it advanced 0.049 s for
each 0.05 s of wall time over 30 reads on Cell Selection). It is the address
[anim-transform.md](anim-transform.md) records as `g_ingame_fallback`; `Text_Draw` reads it
directly, so on the front end it plays the role of the PSP's `g_fe_anim_clock`
(`*(0x08b317b0) + 0x40`). The existing name is not changed here.

`scripts/ps2_pulse.py` finds both objects in EE RAM by these shapes and
`scripts/pcsx2-menu-pair.py` reads them on each side of the GS screenshot. Only the `Selector`'s
phase goes to ours: the title's ladder runs in RAM, but the original's title box is the same
picture at every step it read.

## Names recovered

| Address | Name | Conf | PSP twin |
| --- | --- | ---: | --- |
| `0x0018ee30` | `GridController_UpdateSelectorStamp` | 90 | `0x088a3e8c` |
| `0x001a7fe8` | `Text_Draw` | 85 | `0x088ce668` |
| `0x001a7010` | `Text_ParseValues` | 80 | `Text_ParseValues` |
| `0x00298118` | `g_text_vtable` (data) | 75 | - |

### `GridController_UpdateSelectorStamp` (`0x0018ee30`)

Reached through the vtables: `0x001905d8` (`GridController_UpdateSelectorPulse`,
[grid-selection.md](grid-selection.md)) is the only word equal to its address in initialised
memory, at `0x002962d4`; the same slot of the table `0xd8` bytes earlier, `0x002961fc`, holds
`0x0018ee30`. On the PSP the two functions sit in sibling tables the same way (`0x08accca0` and
`0x08accd70`). The decompile is the PSP's `0x088a3e8c` body:

- runs only while the widget's flag word `+0x2c` has `0x200`;
- the cached `Selector` is `*(this + 0xc0)` (PSP `+0xbc`), eased to `(hex.x + 0.5 - 5.0, hex.y + 0.5 - 7.0)` with `hex` at `+0x98`/`+0x9c`;
- `phase = *(this + 0xa0) + 2 dt`, minus `2.0` past `2.0`; the tint `lerp(phase, 0xff33a6b9, white)` on the first half and back on the second (`FUN_001a8cb0`), on all four vertex colours `+0xb0..+0xbc` of the `Selector`;
- **the stamp** `*(Selector + 0xfc) = (int)(t * 150 + 80)`, `t` the triangle of the phase (PSP `+0xf4`);
- the four d-pad polls (`FUN_0018f930`) and a cross (`+5`) poll that opens the tile under the cursor.

`0x001905d8` has the same phase law but looks the `Selector` up by name and writes no stamp.

**Live** (PCSX2, PINE, VM running): a scan of EE RAM for an object whose `+0xa0` is in `[0, 2)`,
whose `+0x2c` has `0x200` and whose `*(+0xc0) + 0xfc` is a stamp finds one moving controller per
screen: `0x003e5f40` on Grid Selection (`Selector` `0x003ebbc0`, phase 1.6140, stamp 137, and
`int(0.386 * 150 + 80) = 137`) and `0x003ee990` on Cell Selection (`Selector` `0x00400100`). 30
reads 0.05 s apart on Cell Selection stepped the phase by 0.098 for each 0.049 s of the clock
above (`2 dt`) and wrapped `1.9519 -> 0.0439`. Read back to back, phase and stamp agree within
one frame's step, three finds out of three. Only `0x0018ee30` writes `+0xfc`, so it is the
update that runs on both screens. The addresses are the heap's in these two states, not
constants: the pair script finds them each time.

**Confidence 90**: the decompile matches the PSP body literal for literal, and the live phase and
stamp follow it on two screens.

### `Text_Draw` (`0x001a7fe8`) and the title's ladder

`Text_ParseValues` (`0x001a7010`) compares the attribute names `PulseMultiplier`
(`0x002b1b88`, stored at `+0xc0`), `RealGlow` (`0x002b1b98`, `+0xbc`) and `pulse` (`0x002b1bf0`),
and sets bit `8` of `+0xcc` when `pulse` reads `true` (PSP: `+0xb8`, `+0xbc`, flag `+0xc8 & 8`). Its address sits in three tables (`0x00295e34`,
`0x002981ac`, `0x0029835c`); the one live `Text` objects carry at `+0x38` is `0x00298118`, and
`0x0029815c` in it holds `0x001a7fe8`.

`Text_Draw`, while `+0xcc & 8` is set:

```c
tick = (u8)(int)(*(float *)(*(int *)0x002e3640 + 0x40) * 20.0);
for (n = tick - last_tick; n > 0; n--)        /* last_tick: byte at +0xec */
    if (up)  { if (s < 150) s += 15; else up = 0; }   /* up: bit 31 of word +0xec */
    else     { if (s == 0) up = 1; else s -= 15; }    /* s: byte at +0xed */
last_tick = tick;
colour = lerp(s / 150, white, colour);
glow   = (150 - s + 70) * PulseMultiplier;            /* the glow pass's alpha */
```

A tick count that wrapped (`tick < last_tick`, a byte) steps nothing that frame. A branch under
`+0xec & 0x4000000` restarts the ladder at `s = 0` when the widget's `+0x2c & 0x200` comes on;
neither title has that bit (`+0xec` read `0x8300697e` and `0x03002d31`).

**Live**: the `Text` objects with vtable `0x00298118` and the pulse flag whose `+0xec` byte
moves: one per screen, `0x003e4ab0` on Grid Selection (`GRID 1`) and `0x004004a0` on Cell
Selection (`SINGLE RACE`; `+0xa0`/`+0xa4` read 360, 63), `PulseMultiplier` 1.0 on both. 30 reads
0.05 s apart read `s` 45, 60, 75 .. 150 rising, 150 again with the direction cleared, 135 .. 0,
0 again with it set, 15: **one step per 1/20 s of the clock and 22 ticks a cycle (1.1 s)**, two
held ticks at the ends. The other pulse `Text`s in RAM (four) did not move: the other screens'.

**Confidence 85**: decompile plus a live ladder on two screens following it step for step; the
draw half after the stamp (the glow pass through `FUN_0020d3e0`) is not dissected, and its
output does not show in the frame (next section).

**Checked, differs** (not wired): ours' `Text::glow_stamp` steps a 20-step, 1.0 s ladder
(`crates/ui/src/screen.rs`); the PS2 original's is 22 ticks, 1.1 s in RAM. The PSP's
`Text_Draw` was not re-read for the held ticks.

## The pair script's placement

The VM runs while PCSX2 takes its screenshot, so `pcsx2-menu-pair.py` reads both counters and the
clock just before the F8 key and again when the file appears (with the ticker record), and puts
the frame the ticker head fit picked the same fraction of the way between the two reads: the
phase advanced `2 dt`, the ladder stepped to that frame's tick. A check on one bracket: placed at
the second read, the phase came out 1.9116 against 1.9117 read and the ladder at 45 rising,
the byte read.

### The ladder does not reach the picture

Over three grabs per screen the ladder read 105, 90 and 120 on Grid Selection and 135, 105 and
75 on Cell Selection, and the original's title box (Grid (690, 235, 180, 70), Cell (690, 95, 330,
60) of the 1280 x 720 pair) is the same picture in all of them: the largest grey difference
between two grabs is 1 on Grid and 0 on Cell. Ours rendered at each of the 11 steps against one
Cell grab scores 35.1 % of the title box over 40 grey at `s = 0` falling steadily to 10.9 % at
`s = 135` and 150: the original reads as ours' dimmest step whatever its ladder holds. So the PS2
draws `SINGLE RACE` and `GRID 1` with no visible pulse, and ours' bright halo at the ladder's top
is ours' alone. Which part of `Text_Draw`'s output never lands (the white-to-colour lerp of a
white text, the glow pass's alpha on a target the PS2 does not composite) is not read.
**Confidence 80** on "no visible pulse" (three grabs per screen, pixel-identical; one sweep).
`--menu-title-seconds` is therefore not passed on PS2 (a value that fits by accident is a
stand-in). Ours now draws the PS2's `pulse="true"` card title with no glow stamp
(`oag_title::MenuSkin::pulse_text_glow`, `docs/ui/campaign-screens.md`, "The card title on Pulse
PS2 has no halo", confidence 75): no stamp fits every grab (six), the dimmest step within a point of it
but with a faint halo the original lacks.

## The pair script's placement

The VM runs while PCSX2 takes its screenshot, so `pcsx2-menu-pair.py` reads both counters and the
clock just before the F8 key and again when the file appears (with the ticker record), and puts
the frame the ticker head fit picked the same fraction of the way between the two reads: the
phase advanced `2 dt`, the ladder stepped to that frame's tick. A check on one bracket: placed at
the second read, the phase came out 1.9116 against 1.9117 read and the ladder at 45 rising,
the byte read.

**Measured effect** (pixels whose grey differs by more than 40, ours rendered with and without
`--menu-picker-seconds` against the same `theirs.png`, 1280 x 720; `Selector` boxes `(x, y, w, h)`:
Grid (140, 370, 140, 115), Cell (280, 200, 130, 120)):

| Pair, grab | `Selector` box | whole frame |
| --- | ---: | ---: |
| Grid, grab 1 | 15.49 % -> 6.53 % | 2.40 % -> 2.19 % |
| Grid, grab 2 | 19.99 % -> 4.82 % | 2.26 % -> 1.87 % |
| Grid, grab 3 | 12.09 % -> 7.21 % | 2.32 % -> 2.20 % |
| Cell, grab 1 | 19.18 % -> 4.97 % | 3.09 % -> 2.62 % |
| Cell, grab 3 | 23.35 % -> 16.72 % | 3.08 % -> 2.45 % |

(Cell grab 2 found no single moving controller with the first version of the finder, which
checked the stamp inside one 32 MiB read that spans several frames; the finder now re-reads phase
and stamp back to back.) The `Selector` box's "before" varies with the original's phase from grab
to grab (12.1 to 23.4 %); every grab improves by 4.9 to 15.2 points, and the "after" spread on
Grid is 2.4 points. What is left in the Cell box at the white end (grab 3) is the hex's shape and
halo, not its phase: the crops show the same tint on both sides. The whole frame moved with the
title too, because ours' title then followed the `Selector`'s seconds (its halo is brightest where
the `Selector` is cyan); the title now stamps nothing on PS2, see above.
