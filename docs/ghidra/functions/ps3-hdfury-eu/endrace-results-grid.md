# `EndRace Results`' standings grid: the frame and the rows are laid out in code

2026-09-28. `Data\Plugins\Frontend\Gui\EndRace_Definition.xml` authors the
grid's frame at fixed numbers and all forty `Grid{col}.{row}` cells at
`x="0" y="0"` ([hd-endrace-screens.md](../../../formats/hd-endrace-screens.md)).
The frame numbers turn out to be one mode's layout - Time Trial / Speed Lap -
and the rows have no authored position at all: the screen's code rewrites
both per mode. This page is that code, found off the widget-name strings
(`0x00794370` `Grid%d.%d`, `0x00794530` `GridHighlight`, `0x00794c08`
`GridBottomBlock`) through `scripts/ps3-toc.py attrib`. Every function below
reads TOC `0x008ad4d8`, `exact`, so Ghidra's own `DAT_`/`PTR_` names are the
right addresses.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0022c068` | `EndRaceResults_LayoutGrid` | 78 |
| `0x00223f50` | `EndRaceResults_ResetGrid` | 78 |
| `0x0022b688` | `EndRaceResults_FillRaceRows` | 72 |
| `0x002278c0` | `EndRaceResults_HighlightRow` | 80 |

All positions below are inside the grid's own `<Item OffsetX="375"
OffsetY="368">`. "Hide" is clearing bit `4` of a widget's `+0x34` flags and
"show" setting it - the bit `Block_Update` blinks an arrow with
([menu-blocks.md](menu-blocks.md)).

## `EndRaceResults_LayoutGrid` - `0x0022c068`

`(screen, mode)`, a branch per mode. Every branch sets `GridHead1`/
`GridHead2`'s captions (`_opd_FUN_00189e40`), positions and widths, calls
`EndRaceResults_ResetGrid`, then a per-mode filler.

**Modes `3`, `4` and `0xf` - the race family.** `RecordNotifyBlock`,
`GridTopBar` and `GridBottomBlock` hidden; `GridSideBarL`/`R` moved to
`y = 44.0` (`0x008b08c0`) and `443.0` tall (`0x008b0874`); `GridBottomBar`
to `y = 487.0` (`0x008b087c`), which is `44 + 443`; `GridHead1` at `x = 0`,
`497` wide (`0x008b0884`), captioned `IG_HUD_POS`; `GridHead2` at `x = 505`
(`0x008b0888`), `286` wide (`0x438f0000`), captioned `IG_HUD_TIME` (mode 4
captions `ER_TEAM`/`ER_POINTS` instead - a points table); `GridHead3`/`4`
hidden. Mode `3` then calls `EndRaceResults_FillRaceRows`.

**Modes `5` and `10` - Time Trial and Speed Lap** (`Line1` set to
`ER_TT_COM` / `ER_SL_COM`). Side bars `347.0` tall (`0x008b08a0`), bottom bar
at `391.0` (`0x008b08a4`), `GridBottomBlock` **shown**, four captioned
columns (`IG_HUD_LAP`, `IG_HUD_TIME`, `MSC_SPEED_PADS`). These are exactly the
numbers the file authors, which is why the file's frame reads as the
Time-Trial layout rather than a default.

Modes `6` (Zone, `ER_ZONE_COM`), `0xe` (Detonator, `ER_ZDETONATOR_COM`) and
`0xd` have their own branches; not read further here.

**Which mode is a single race is inferred, confidence 75**: mode `3`'s filler
writes `ER_%dPLACE` into `Line1` for a placed player, the headline a live
HD single race shows (`4TH PLACE`, `5TH PLACE`), and its columns are the
position/time pair the race screen captions. The call that passes `mode`
arrives through a vtable (`0x0087f1d8`), not traced.

## `EndRaceResults_ResetGrid` - `0x00223f50`

Run first on every mode: all forty `Grid{c}.{r}` and ten `Gridp.{r}` cleared
and coloured `0xff646464`, the ten `Gridi.{r}` badges hidden, the
`Grid{0,1,2}.h` header texts cleared, and **`GridHighlight`,
`GridHighlight2` and `GridStrikeThrough` hidden** (TOC `0x312c`/`0x3130`/
`0x3134`). Nothing a race draws shows the strike-through again.

## `EndRaceResults_FillRaceRows` - `0x0022b688`

Per racer `r` (records `300` bytes apart):

- `Grid0.r` - the position, formatted off the record's `+0x168` - at
  `x = 40` (`0x42200000`), `y = 96 + 45 r` (`0x008b07b4` = `96.0`,
  `0x008b07b0` = `45.0`).
- `Grid1.r` - a string off the record's `+0x50` - at `x = 200`
  (`0x43480000`), same `y`. **Re-read 2026-11-06 (confidence 75): it is a
  team idstring.** On a non-multiplayer race (`g_GameState+0xe0 < 0x10`) the
  string is set through `_opd_FUN_00202f58(widget, str, 0, 1)`, the same call
  that sets `Line1` from the idstring `ER_SHIP_DES`, i.e. the localising
  setter, where the number columns go through `_opd_FUN_00202d28`, the literal
  one. The same `+0x50` string is then hashed (`FUN_006764c8` / `_opd_FUN_0032a740`)
  and compared against the hashes of the 32 badge textures loaded on the screen
  (`param_1 + 0xb3c`, hash at `+0x80`), and the match is assigned to `Gridi.r`:
  the string is the key of the row's ship badge (`Data\Ships\<Team>\fe\miniBW.tga`),
  so it names the craft's team. The multiplayer branch (`+0xe0 == 0x10`) prints
  the same field as a literal name instead (the player's name; the podium reads it
  likewise). What this build draws: the team, through the string table (`Assegai`
  is `ASSEGAI`, `Goteki` is `GOTEKI 45`).
- `Grid2.r` - the finish time at `x = 545` (`0x008b07d4`), same `y`, off the
  record's `+0x160`: above `0` the time (`_opd_FUN_00083720`), `-1` the string
  `ER_DNF` (TOC `0xc6b`, `0x008b0684`), anything else `0` the string `ER_RACING`
  (TOC `0xcd4`, `0x008b0828`; confidence 80, both strings read off the executable).
  A craft still racing when the table fills reads `RACING`, which is what this
  build draws; nothing here produces `DNF`.

**Omega**: `checked, applies, not wired`. Omega's `eboot.bin` carries the same
`Grid%d.%d` format string (`0x01823d9a`), so the same screen code exists there; its
`FillRaceRows` was not read for `+0x50`/`+0x160`, and Omega's front end does not load the
screen yet (`docs/ui/endrace-screens.md`, "HD's `Results` names every craft").
- `Gridi.r` - the badge image - shown at `x = 130` (`0x008b07d8`),
  `y = 82 + 45 r` (`0x008b07bc`).
- For the player's record, `EndRaceResults_HighlightRow(screen, 94 + 45 r,
  r)` - the running `0x5e` start, `+ 0x2d` a row.

**A cross-check the two functions agree on**: a Block's label sits at its
`X + 40` (`Block_CreateLabel`, [menu-blocks.md](menu-blocks.md)), so
`GridHead1` at `x = 0` and `GridHead2` at `x = 505` caption `x = 40` and
`x = 545` - exactly columns 0 and 2. Eight rows end at `96 + 7 * 45 = 411`,
inside the `487` bottom bar.

**Called every frame (2026-11-08, `endrace-live-board`, confidence 75).** `FillRaceRows`'s caller
is the `EndRace Results` screen's Update (`0x0022e680`, vtable `0x0087f1f0`): on a race layout
(`g_GameState+0xe0` of `3` or `0x10`) with the grid page up it calls `0x0005a770` and then
`FillRaceRows` on every update. `0x0005a770` rewrites each record's `+0x160` (base `+0x1980`
in the screen, stride 300) from the live race: `-1`/`-2` for a craft that is out, else the sum
of its recorded lap times, `0` while any lap is missing. So a craft still racing reads
`RACING` and then its time when it crosses, with the screen up. Not read: the `-2` case and
whether the position (`+0x168`) is re-sorted per frame. Our board follows the field the same way
(`docs/ui/endrace-screens.md`, "The field keeps finishing behind `Results`").

## `EndRaceResults_HighlightRow` - `0x002278c0`

`(screen, y, row)`: colours the row's four `Grid{c}.row` texts, its
`Gridi.row` badge and its `Gridp.row` label `0xffffffff`, shows
`GridHighlight` and moves it to `y` (vtable `+0x70`, the `SetY` slot).

## Implemented

`oag_ui_screens::endrace::hd::hd_results_draw_list`'s `RaceGrid` - every number
above, applied to a race-family headline, with `Grid1.r` the team and `Grid2.r`
`RACING` for a craft still racing (2026-11-06; `Gridi.r`, the badge, is not drawn); Time Trial and Speed Lap keep the
file's frame and this build's older, chosen row pitch, since their fillers
(`0x002239a8`, `0x00227b30`) were not read. Pinned against the real disc by
`crates/game/tests/hd_endrace_ground_truth.rs`'s
`an_eight_craft_hd_results_grid_clears_its_own_bottom_bar`.
