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
mean grey 94.9 to 38.3. Everything here was read from a decompile, not traced
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

## The statistics table (not read)

`TeamInfo` (`Frontend/Items/TeamInfo_Item.cpp`, `0x813ceab0`) draws the panel
and the `FE_SHIP_STATS_TUT` bubble; its code was not read. The numbers it
shows are on disc: `HandlingStats/<team>/<n>/handlingstats.xml` carries
`<FE speed thrust handling shield firepower>` and `feisar2048/3` reads
`8 8 7 4 6`, the frame's SPEED 8, HANDLING 7, HEALTH 4, FIREPOWER 6 (n 1..4 =
fighter, agility, speed, prototype). Confidence 80 that the table reads these;
the bar law (10 cells) is not read.

## Omega and HD

- **Omega: checked, applies, not wired.** Omega's `eboot.bin` carries the same
  class (`C:\WOPS4\Wipeout\Code\Frontend\Items\TouchTeamGrid_Item.cpp`, the same
  `teamgrid\*.gtf` names) and its archives ship `teamgrid/*.gnf` (the 13
  files here plus `_vr` variants in the patch's `data08`). No
  `Team_Definition.xml` exists in any Omega archive, so whether Omega's HD-derived
  front end instantiates this item was not traced.
- **HD/Fury campaign mode** (`[0x1b8] == 1`, draw `FUN_81106a46`): 4 columns by
  3 rows of 100 x 82 cells from `teamgrid/hd/`, 12 teams. Ours still draws the
  2048 grid there. Open.
