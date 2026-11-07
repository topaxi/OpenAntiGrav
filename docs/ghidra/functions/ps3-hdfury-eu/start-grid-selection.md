# Which `start_grid` file a race flies: the mode picks it (HD/Fury, PS3)

`EBOOT.elf` (Wipeout HD/Fury, PS3), image base `0x00000000`. Static reading, so every score is capped
at 84 by the [confidence rubric](../../../reverse-engineering/confidence-rubric.md). The statement
that a mode takes a file is not a thing anybody watched: the original's fly-by was only ever watched
on the plain file (`docs/gameplay/race-intro.md`, "Other titles").

**The question.** Each circuit ships `start_grid.vex`, `start_grid_reversed.vex` and (nearly
everywhere) `start_grid_ta.vex` and `start_grid_ta_reversed.vex`; the eight numbered circuits also
ship `start_grid_de.vex`. The executable carries four format strings for them. Which one does a race
fly?

| String | Address |
| --- | --- |
| `%s\%sstart_grid_de%s.vex` | `0x00780448` |
| `%s\%sstart_grid%s.vex` | `0x00780468` |
| `%s\start_grid.vex` | `0x00780480` |
| `%s\%sstart_grid_ta%s.vex` | `0x00780660` |

## The caller: `Track_Construct_q` (`0x000b2af0`) and its twin (`0x000b43b0`)

Ghidra's string xrefs name `Shader_InitEngineParams`; that is the wrong-TOC trap
(`memory.md`: every function has its own TOC). `scripts/ps3-toc.py attrib` run on each of the four
strings returns the same two functions, `0x000b2af0` and `0x000b43b0`, and `ps3-toc.py toc` gives
`0x000b2af0` the TOC `0x008ad4d8`. The four format strings sit in TOC slots `-0x53e0` (`_de`),
`-0x53dc` (plain), `-0x53d8` (the fallback) and `-0x5378` (`_ta`). The twin is the same code at a
different address (a constructor pair); only `0x000b2af0` is read line by line here.

The two `%s` after the directory are a prefix and a suffix. Read off the instructions at
`0x000b2c78`-`0x000b2d38` (`lbz r0,0x1b5(r10)`, `r10 = *(g_GameState + 0x1bc)`):

| Byte `+0x1b5` | Prefix (`r15`, slot `-0x53e4`) | Suffix (`r5`, slot `-0x53e8` or `-0x53e4`) |
| --- | --- | --- |
| zero | `""` (`0x00780440`) | `""` |
| set | `""` | `"_reversed"` (`0x00780430`) |

So the byte is "this circuit runs backwards", the prefix is empty for every race, and the same
`track%s.vex` path at `0x000b2e6c` takes the same suffix: a reversed race loads `track_reversed.vex`
and the matching `_reversed` grid file. Confidence **85**.

## The law

After the suffix, at `0x000b2ca0` the code loads `*(g_GameState + 0xe0)`, the game mode
([mode-manager.md](mode-manager.md)), and compares:

```text
0x000b2ca4  cmpwi r11, 2        -> 0x000b3c00   %s\%sstart_grid_de%s.vex   (slot -0x53e0)
0x000b2cac  cmpwi r11, 5        -> 0x000b2d74   %s\%sstart_grid_ta%s.vex   (slot -0x5378)
0x000b2cb4  cmpwi r11, 0xa      -> 0x000b2d74
0x000b2cbc  cmplwi r11, 0x15 ; ble 0x000b2d58 : (1 << mode) & 0x00206040 != 0 -> 0x000b2d74
0x000b2cc4  cmpwi r11, 7        -> 0x000b2d74
otherwise                       -> 0x000b2cd0   %s\%sstart_grid%s.vex      (slot -0x53dc)
```

`0x00206040` is bits 6, 13, 14, 21, the Zone and Detonator family ([zone-sky.md](zone-sky.md)). So:

| Game mode id | Class ([mode-manager.md](mode-manager.md)) | File |
| --- | --- | --- |
| 2 | `Demo` | `start_grid_de` |
| 5, 10 | `SPTimeTrial` (Time Trial, Speed Lap: [pause-menu.md](pause-menu.md), read live) | `start_grid_ta` |
| 6, 13, 14, 21 | Zone, Zone Battle, Detonator (`0x206040`) | `start_grid_ta` |
| 7 | no `ModeManager` (unread) | `start_grid_ta` |
| 3, 9, 12 `SPArcade`; 4 `SPTournament`; 8 `SPElimination`; 1, 11, 15, 16 to 20 | | plain |

Our modes map to HD's ids as `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md` has them (Single
Race 3, Tournament 4, Time Trial 5, Zone 6, Eliminator 8, Head to Head 9, Speed Lap 10). Time
Trial, Speed Lap and Zone therefore fly `_ta`; Single Race, Eliminator, Tournament and Head to Head
fly the plain pair. Head to Head is mode 9 in the campaign code, but no menu offers it and no cell
authors it ([head2head.md](head2head.md)): what the original would fly is read off the rule, never
watched. Confidence **84** on the branch table (a direct read of the compare chain), **75** on
mode 7 belonging to a mode we have (it is `_ta` whichever it is).

**What `_de` is.** Mode 2 is `Demo` (the attract-mode class, `0xa8` bytes), so `start_grid_de` is the
fly-by of the demo. Only the eight numbered circuits ship it (`DATA02`), no `_de_reversed` exists,
and Fury's four circuits ship none. Confidence **80** (the mode is attributed at 88; the file
census is exact). This project has no demo mode, so no row of ours selects it.

## The fallback

After the `sprintf` the code calls `0x0031f438` (`FwFile_Exists` here) on the result. It walks
the list at `*(0x008b5170 + 0x20)`, optionally narrowed by a `dev:` prefix up to a `:`, and calls
virtual slot `+0x2c` of each source with the path, returning 1 at the first yes. On 0 the code
formats `%s\start_grid.vex` (`0x000b2d0c`-`0x000b2d20`, slot `-0x53d8`; **directory only**: no
prefix, no suffix) over the same buffer. So a missing file falls back to the **forward plain**
file, even for a reversed race. Confidence **78** that the callee is a file-exists test: the path
goes in, a bool comes out, and the caller's reaction is to try another name.

The consequence on the disc, all measured with `scripts/psarc.py list`:

- `zone_1` to `zone_4` ship `start_grid.vex` only. Zone mode (6) asks for `zone_1/start_grid_ta.vex`,
  finds nothing and falls back to the plain file, **which has no `grid_camera1`** (`race-intro.md`).
- A reversed Time Trial on a numbered circuit reads `start_grid_ta_reversed.vex` (DATA03 ships them,
  the same size as the `_reversed` file on seven of the eight circuits).
- A reversed Demo would ask for `start_grid_de_reversed.vex`, which no circuit ships, and fall back
  to the forward plain file.

One more gate: the byte at `0x009384e1` (`bne` at `0x000b2c9c` and `0x000b2d44`) sends the code past
the whole choice to the `track%s.vex` load. [ai-stats.md](ai-stats.md) reads the same byte in
Eliminator terms; it is not wired here and what it means in the loader is unread.

## What the files frame

`_ta` is the lone craft's fly-by. Measured on Talon's Junction, Amphiseum, Vineta K and Ubermall by
sampling each file's camera at 30 Hz, projecting the eight grid slots (our spawn slots, read from a
Single Race load) through the pose with the flyby's 54.309 degree vertical field at 16:9 and
counting the samples where a slot sits within 40 world units of the eye and inside the frustum
(no occlusion test). The lone craft of a Time Trial sits in **slot 1** on all four circuits.

| Circuit, file | Samples | Slot 1 only | Slot 1 and others | Other slots only |
| --- | ---: | ---: | ---: | ---: |
| Talon's Junction `start_grid.vex` | 5,650 | 121 | 8 | **1,087** |
| Talon's Junction `start_grid_ta.vex` | 5,450 | 792 | 90 | 63 |
| Amphiseum `start_grid.vex` | 4,500 | 172 | 59 | **699** |
| Amphiseum `start_grid_ta.vex` | 4,500 | 770 | 109 | 50 |
| Vineta K `start_grid.vex` | 5,200 | 0 | 148 | **775** |
| Vineta K `start_grid_ta.vex` | 4,924 | 211 | 376 | 0 |
| Ubermall `start_grid.vex` | 4,324 | 0 | 0 | **581** |
| Ubermall `start_grid_ta.vex` | 4,324 | 385 | 275 | 0 |

The plain file spends most of its close-ups on empty slots; `_ta` spends them on slot 1. The
maintainer's report (a Time Trial fly-by panning over empty grid) is the plain file played for a
mode whose original plays `_ta`. For Head to Head's two craft (our slots 1 and 2) `_ta` also frames
only those two on Talon's Junction (26 empty-only samples of 5,450 at 40 units against 907 of 5,650
for the plain file); the executable, though, gives mode 9 the plain file. Vineta K's `_de` frames no
slot of ours at all (it is a different shot list, for a different grid).

## Omega and 2048

- **Omega** (`/omega/eboot-ps4-omega-eu.bin`): **checked, differs.** The strings are
  `%s\start_grid%s.vex` (`0x018172de`), `%s\start_grid_ta%s.vex` (`0x018172f2`) and
  `%s\start_grid.vex` (`0x01817309`): no `_de`, no prefix. The chooser `FUN_0138b360` compares a
  mode id (`r13d`; its load was not located in this function, and every other reader takes the
  game state's mode word `0x01f999e4`, `ps4-omega-eu/ai-controls.md`) at `0x0138bc3d`-`0x0138bcc6`:
  10, 5, 7 and the set 6, 13, 14, 21 (`bt 0x8181` on `mode - 6`) take `_ta` as in HD; **2 takes
  the plain file** (no demo file); and two further tests send a race to `_ta`: a byte at
  `0x0203a728` with `FUN_01669270` (a case-insensitive match of a track name at `0x0203a6f8`
  against `WV_Zone_1` to `4` and four more names), and `*(0x01e31910 + 0x21c) == 0`, where
  `0x01e31910` is the current game-mode object (`ai-controls.md`) and the field is unread.
  Confidence 65. Omega's `PRE_RACE` is not on, so nothing of ours changes.
- **2048** (`/2048/eboot-vita-2048-eu-v104.elf`): **checked, applies (strings only), not wired.**
  The same four strings with the prefix (`0x814a91bc`, `0x814a91d8`, `0x814a91f4`, `0x814a920c`),
  one caller, `FUN_8121bce2` (the Thumb-2 blocks at `0x8121ca4c`-`0x8121cb3e` load them); the mode
  gate that chooses between them was not read. 2048's `grid_camera1` shape needs an `AnimEnd`
  that mostly is not authored ([race-intro.md](../../../gameplay/race-intro.md)), so a fly-by is
  not turned on there. Confidence 60.

## Function table

| Address | Name | Confidence | What |
| --- | --- | ---: | --- |
| `0x0031f438` | `FwFile_Exists` | 78 | takes a path, asks each mounted file source (`+0x2c`), returns whether any has it |

`Track_Construct_q` (`0x000b2af0`) keeps its name and page ([loading-screen.md](loading-screen.md)).
