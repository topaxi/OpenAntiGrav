# `TouchTeamGrid`: the Select Team grid, its draw and its input

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04,
`/2048/eboot-vita-2048-eu-v104.elf`), image base `0x81000000`. Read for the
`team` screen of `Team_Definition.xml`, which authors only the widget's anchor
(`<TouchTeamGrid name="teamgrid_touch"><Values x="16" y="94">`). Source file
named by the executable: `Frontend/Items/TouchTeamGrid_Item.cpp`
(string `0x8145bf04`). **No function is renamed** (every row is below the
renaming bar or unnecessary for the wiring), so nothing is added to
`names.tsv`; the addresses below are Ghidra's own `FUN_` names.

Wired in `crates/ui/src/frontend/team.rs`; the pair `2048/menu-team` went from
mean grey 94.9 to 38.3, and with the statistics panel and the hull
(`team_info.rs`, `oag_game::team_hull`, below) to 27.3. Everything here was read from a decompile, not traced
live: confidence is per row.

## The class

The vtable is `0x8150edc8` (thumb bit set on each entry). Slots read:

| Slot | Address | What |
| --- | --- | --- |
| 3 | `0x8110772e` | per-frame update: hit-test, cursor, input. Confidence 85 |
| 7 | `0x8110771a` | draw dispatcher: `[this+0x1b8] == 0` runs `FUN_8110700e`, else `FUN_81106a46`. 85 |
| 11 | `0x811067be` | on-enter init: lock flags, start cell. 80 |
| 14 | `0x813d0254` | constructor: field defaults. 85 |
| 15 | `0x81105710` | loads the `Values` and every texture. 90 |

`[this+0x1b8]` is the layout mode: **0 = the 2048 grid** (global `Team2048`,
`PlayerLivery2048`), **1 = the HD/Fury campaign grid** (global `TeamHD`,
`PlayerLiveryHD`, textures under `teamgrid/hd/`). Which mode a screen runs is
set outside this class (not found). Confidence 75.

## Constructor floats (`FUN_813d0254`)

Stored at `this[0x28..0x2b]` as ints and `this[0x73..0x7b]` as floats; each
indexed by mode (`+mode`):

| Field | Mode 0 | Mode 1 | Read as |
| --- | --- | --- | --- |
| `[0x28+mode]` | 4 | 3 | rows |
| `[0x2a+mode]` | 5 | 4 | columns |
| `[0x73+mode]` | 80.0 | 100.0 | cell width |
| `[0x75+mode]` | 65.0 | 82.0 | cell height |
| `[0x7a]`, `[0x7b]` | 202.0, 80.0 | | a DLC2 toggle rect size (below) |

The cursor starts at `[0x4d] = [0x4e] = 0`: column 0, row 0. Confidence 85.

## Geometry, from the update (`FUN_8110772e`) and draw (`FUN_8110700e`)

- Pitch is cell size `+2` on both axes (`fVar18 = w + 2`, `fVar20 = h + 2`):
  mode 0 is 82 x 67.
- Origin `x = this[0x2c] + this[0x12]` (the anchor, 16). Row 0 begins at
  `y = anchor.y + (h - 1)` = 94 + 64 = 158; the strip above it, 64 high,
  carries the team badges.
- The input hit-test is `FUN_8105b25a(x + col*pitchX, y + row*pitchY,
  pitchX + 1, 68.0)`: **83 x 68**, so neighbouring cells overlap by one unit and
  the later (column, row) in loop order wins. Only cells are tested; the
  badge strip is not a button in this function.
- Draw, per cell (2048 mode): a fill rect `(x, y)..(x + 80, y + 65)`; the craft
  icon at its own texture size, centred in the cell; for a locked cell a
  `lock` texture drawn 32 x 32 at `(cellX + 74 - iconW/2, cellY + 38 - iconH/2)`.
  The team badge is its texture centred in `(col * 82, 94, 80, 65)`.
- Colour law (`FUN_81059736` of `Blue2048`, `White2048`, `Orange2048`): fill
  `Blue2048`; `Orange2048` on the cursor cell (`[0x134],[0x138]`) or a pressed
  cell (`[0x12c],[0x130]`); a locked cell's fill and icon tint get alpha
  `0x80`. Icons tint `White2048`. Confidence 80.
- The intro animation factor `[0x64]` scales how many columns (`x 5.5`) and
  rows (`x 4.5`) are drawn; at rest all are.

## Order and names

String tables (confidence 90, read as bytes):

- Textures, loader order (`FUN_81105710`, fields `0x3b`..`0x45`): `Feisar`,
  `AG_SYS`, `Qirex`, `Auricom`, `Pir-Hana` (badges, the column order);
  then `Speed`, `Fighter`, `Agility`, `ProtoSpeed`, `ProtoCombat`,
  `ProtoAgility` (craft icons); `lock` (field `0x47`), `new_circle` (`0x46`).
  HD mode: `hd/Feisar`, `Triakis`, `Mirage`, `Qirex`, `Goteki`, `Harimau`,
  `Piranha`, `EGX`, `Auricom`, `AG_Systems`, `Assegai`, `Icaras`, plus
  `hd_logo_button`, `fury_logo_button`, `wipeout_HD_logo`.
- Team global values, `0x8151ebc4` (pointers to `0x8145bde4..`): `Feisar2048`,
  `AG_Systems2048`, `Qirex2048`, `Auricom2048`, `Piranha2048` (column order).
- Craft values, `0x8151ec08`: `speed`, `combat`, `agility`, `prototype` (row
  order; `PlayerLivery2048`).
- Prototype row's icon (draw, `iVar8` remap): column 1 (AG) `ProtoAgility`
  (field 5), columns 2 and 3 (Qirex, Auricom) `ProtoCombat` (4), the others
  `ProtoSpeed` (3). Confidence 85.

`FUN_81106580(this, col, row)` writes the picked values into the `Team2048` /
`PlayerLivery2048` globals (mode 0) or `TeamHD[col*3 + row]` (mode 1).

## Lock flags (hypothesis)

`FUN_811067be` fills `[this + 0x144 + col*4 + row]`: for each ship record in
the list at `0x81967280` whose string at `+5` equals the column's team name
and whose string at `+0x15` equals the row's craft name, it stores
`FUN_812d16f6(record)`. That function caches at `record+0x44` and is true when
`*(record+0x3c)+0x2d8 > 2`, or when `record+0x40 - 1 < N` with `N` read by
`FUN_810133da(DAT_81545468, *(DAT_81545468 + 0x3257c))`. On a fresh save the
frame shows 19 of 20 cells locked (Feisar Speed open). **Hypothesis
(confidence 40):** `record+0x40` is the ship's unlock order and `N` the number
of unlocked ships; neither the record's source (an XML `Unlock` element or
`SP.xml`) nor `N`'s save field was traced, and the unlock count is not
derived from the frame. Open: read the record builder and
`FUN_810133da`'s save offset.

Also open: `record+0x158` per cell drives the `new_circle` badge and the text
`ONL_CON_NEW` for an unlocked, unseen cell.

## The DLC2 toggle

When `g_bDlc2Mounted`, a rect `(x, y, 2*202 + 2, 80)` at the anchor flips
`[0x77]` (toggling the grid between the base and the HD set) on a hit and
resets the selection cache. Confidence 60: the flag's effect on the draw
was not followed.

## The statistics panel (`TeamInfo`, read 2026-10-27)

`Frontend/Items/TeamInfo_Item.cpp` (string `0x81474f78` names the widget
`teamscreeninfo`; `Team_Definition.xml` authors only `x=16 y=307`). Vtable
`0x8150e4ac`: slot 3 update `0x810e1bc0` (counts the four displayed values up
0.5 a tick toward their targets), 7 draw `0x810e0bf6`, 11 enter `0x810e06f4`
(zeroes the values), 14 constructor `0x813ceab0`, 15 loader `0x810e05e4`.
Everything below is at rest and drawn by `crates/ui/src/frontend/team_info.rs`
(module doc lists the same numbers). The screen's own update is `0x8113a3b0`
(not a function in the database until 2026-10-27; created, not renamed).

| What | Value | Where read | Confidence |
| --- | --- | --- | --- |
| Colours | rows `Transparent2048` (`0xc0ffffff`, 75 % white), labels and numbers `Blue2048`, filled cells `Orange2048`; the register each call passes (`r10`, `r9`, `[sp+0x20]` of `0x810e19d6`) | disassembly of `0x810e0bf6`..`0x810e19f0` | 85 |
| Empty cell | `FUN_8105bc14(1.0, 0x808080, 0xff808080)`: grey 128 | `0x810e071a` | 90 |
| Row | `x = 16 + 416`, `y = 307 + 30 n`, 512 x 27 | `0x810e0bf6` (`+ 416.0`, `+ 0x1e`), `0x810e071a` (`+ 27`) | 90 (frame: first row at 307, cells at 632) |
| Cells | ten, at `x + 200 + 25 i`, 22 wide; filled when `rating - i - 0.5 > 0`, alpha `clamp(2 * (rating - i - 0.5))` | `0x810e071a` | 90 |
| Label | `(x + 9, y + 5)`, `NEOSANS_BOLD` at `0.7` (`0x3f333333`) | `0x810e071a` | 85 |
| Number | `%d` (`0x81453e50`), `--` (`0x81453e54`) for 0 and 11, centred at `x + 478` | `0x810e071a` | 85 |
| Rows | `RC_SPEED`, `RC_HANDLING`, `RC_HEALTH`, `RC_FIREPOWER` (`RC_SHIELD`, `RC_THRUST` when `DAT_8153fee8 == 1`, the HD mode) | `0x810e19f0`.. | 90 |
| Header | band `(526, 263)` 418 x 44, team icon `(462, 253)` 64 x 64, class icon 40 left of it, text `(546, 268)` at 0.82 | `0x810e0bf6`, `0x81061808`, frame (band left edge 526, text 546) | 75 |
| Ability icons | 64 x 64 centred at `(886, 224)` and 80 to the left | `0x810614aa`, call at `0x810e1976` (`+ 870`, `- 83`) | 85 |

**Where the numbers come from.** Not `HandlingStats/<team>/<n>/handlingstats.xml`
(an earlier reading of this page): the screen's update (`0x8113a3b0`) passes the
ship record's `+0x1c4`, `+0x1cc`, `+0x1d0` and `+0x1d8` to the panel, and that
record is the plugin's `PI_TeamModel` (`+0x1b0` is its `modellocation`,
`"%s/ship_lod.vex"`; `+0x1ac` its `texturelocation`). `Data\Plugins\teams\Definition.xml`
authors `<FE speed thrust handling shield length firepower description>` on
each, which puts them at `+0x1c4`, `+0x1c8`, `+0x1cc`, `+0x1d0`, `+0x1d4`,
`+0x1d8`: speed, handling, shield (the HEALTH row) and firepower are rows 1 to
4 (`+0x1c8`, `thrust`, is the HD mode's fourth). Feisar Speed is 8, 7, 4, 6 in
both files, which is why the first reading looked right; **Feisar Fighter is
speed 5 here and 3 in `HandlingStats`**, so the files disagree. Confidence 85
(the offsets fit the attribute order and the `+0x1b0` path; the frame checks
one craft). `description` is the string id of the marquee a prototype shows in
place of the rows (`strcmp(craft, "prototype") == 0` selects that branch of
`0x810e0bf6`); it is on disc and not drawn.

**Not drawn:** the prototype's description marquee and scrollbar, the
`FE_SHIP_STATS_TUT` bubble, the count-up and the slide-in, the lighter face of
the header's second word (one face here), the ability icons' touch tooltips
(`Ship_Select_No_*`), and `RB_LOCKED` (the locked-craft panel: needs the lock
rule above). Pair `2048/menu-team`, the panel box `(430, 262, 515, 163)`: mean
grey 58.4 to 10.6.

## The hull (`<Model name="ShipModel">`, drawn 2026-10-27)

`oag_game::team_hull` draws it through `Preview::draw_matrices`, the pass every
menu mesh uses (HD and Omega Ship Select go through the same `Preview`).
**Authored** (`Team_Definition.xml`): `OriginX=1400 OriginY=264`, `x=y=0`,
`z=-25`, `nearZ=1`. The origin is a point of a grid twice the screen's:
(700, 132) of 960 x 544, which is where the original frame puts the hull's
centre. **Read off `0x8113a3b0`**: the file is `"%s/ship_lod.vex"` of the
craft's `modellocation` (`Data\art\published\Ships\<team>\<n>\Ship_LOD.vex`, 5,000
to 10,000 triangles, 80 ms to build on this machine); the rotation fields
(`+0xb4`, `+0xb8`) start at 1.57 and -1.57 and ease at 1/6 a tick toward
`0.546` and `-0.932` (plus a touch-gyro term), so those are the rest pose
(confidence 75); `+0xd8` is set to `40.0`, **read as the field of view in
degrees: confidence about 50, fitted by eye, not verified** (the frame's hull
length and position agree). **Chosen:** the far plane, drawing over the 2-D
list (nothing the hull covers is drawn over it on the frame), the model's own
default `Textures_1` livery (yellow: the original is teal; the same open
livery question as the 2048 race pair), the loader running on the frame
thread. Not drawn: the touch turntable, the screen's slide-in. Frame check:
the silhouette sits on the original's (nose at about (490, 195), tail at
(865, 110)).

## Omega and HD

- **Omega: checked, applies, not wired.** Omega's `eboot.bin` carries the same
  class (`C:\WOPS4\Wipeout\Code\Frontend\Items\TouchTeamGrid_Item.cpp`, the same
  `teamgrid\*.gtf` names) and its archives ship `teamgrid/*.gnf` (the 13
  files here plus `_vr` variants in the patch's `data08`). No
  `Team_Definition.xml` exists in any Omega archive, so whether Omega's HD-derived
  front end instantiates this item was not traced.
- **The statistics panel on Omega: checked, applies, not wired.** `eboot.bin`
  names `C:\WOPS4\Wipeout\Code\Frontend\Items\TeamInfo_Item.cpp` (`0x0181fd8e`)
  and `RC_FIREPOWER` (`0x01827600`), and `data00.psarc`'s
  `Data/plugins/teams/Definition.xml` authors the same `<FE speed thrust handling
  shield length firepower description>` on `Feisar2048`'s four `PI_TeamModel`s
  (speed 8, 8, 7, 4, 3.8, 6) and 20 `firepower=` rows in all. Its screens
  author no `team` screen to place the item, so nothing instantiates it; the
  draw's numbers were not compared against Omega's own code.
- **HD/Fury campaign mode** (`[0x1b8] == 1`, draw `FUN_81106a46`): 4 columns by
  3 rows of 100 x 82 cells from `teamgrid/hd/`, 12 teams. Ours still draws the
  2048 grid there. Open.
