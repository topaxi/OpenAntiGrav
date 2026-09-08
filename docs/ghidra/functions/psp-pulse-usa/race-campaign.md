# The Race Campaign, decompiled: `PI_Grid`, `PI_Cell`, medals, points and the unlock gate

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, `pulse-psp-usa.chd`), image
base `0x08804000`. Every address below was reached by cross-reference from a
string in this binary and decompiled in place; nothing here is runtime-verified.

This closes the question
[`docs/formats/race-setup.md`](../../../formats/race-setup.md#the-race-campaign-the-discs-own-campaign-grid-shape-yes-content-no)
left open on 2026-09-08: **where the built-in campaign's per-cell content lives,
what produces a medal, and what `Grid0`..`Grid14` evaluate against.**

## The headline, in three parts

1. **The campaign's content is fully authored on the disc**, in sixteen files
   `Data\Plugins\grids\grid_00.xml` .. `grid_15.xml` under `Data.wad`, listed by
   `Data\Plugins\grids\Definition.xml`. Each is one `PI_Grid` holding 8-16
   `PI_Cell` records; each cell names its track, mode, speed class, lap count,
   weapons/damage switches, AI count, AI skill, and **its own gold, silver and
   bronze targets**. 236 cells across the sixteen grids. The previous pass's
   "every content slot is a runtime placeholder" is correct about the *screen*
   XML and wrong about the campaign: the placeholders are the template the
   screen substitutes these records into.
2. **The campaign does not read `FEData.wad`'s 24 per-track `stats.xml` records
   for any of that** - not the medal targets, not the mode, not the lap count.
   It reads that family for exactly one thing: **AI difficulty**. A cell's
   `skill`/`skillEasy`/`skillHard` is a *position on the track's own
   `SkillScaleValue` curve*, and `FUN_08834df4` interpolates the two.
3. **The medal is a three-way threshold compare, `FUN_088bf620`**, and a medal
   is worth **gold 3, silver 2, bronze 1** points (`FUN_088bf530`). A grid
   unlocks when the points earned across its own cells reach its authored
   `RequiredPoints` (`FUN_0888ebd8`). That ladder is 12, 16, 20, 24, then 28 for
   every grid from `grid4` on.

**The in-race HUD's medal tier is a separate, still-unread value** - see
[what is not determined](#what-is-not-determined). It is not `FUN_088bf620`'s
output and its ordinal runs the other way.

## The authored data: `Data\Plugins\grids\`

`Data\Plugins\grids\Definition.xml` (read with `oag-wad cat`) lists sixteen
files, `grid_00.xml` through `grid_15.xml`. Each carries its own `<code>`
shortening dictionary, so the tag and attribute names below are the expanded
forms. `grid_00.xml`'s first cell, expanded:

```xml
<PI_Grid name="grid0">
  <Values RequiredPoints="12" Locked="false"/>
  <PI_Cell name="grid0_2_1">
    <Values track="16_Track" mode="Race" class="Venom" Weapons="on" damage="on"
            AICount="7" skillEasy="1.1" skill="1.75" skillHard="2.5"
            laps="3" ship="None" ShipChoice="Yes"/>
    <Gold   Target="1"/>
    <Silver Target="2"/>
    <Bronze Target="3"/>
  </PI_Cell>
  ...
</PI_Grid>
```

`grid_01.xml` onwards additionally carry `<Unlock Grid="Grid0"/>` and, for a
`mode="Tournament"` cell, a run of `<TournamentTrack track="..."/>` rows naming
the legs.

| Grid | Cells | Max points | `RequiredPoints` | `Locked` | `<Unlock Grid=>` |
| --- | ---: | ---: | ---: | --- | --- |
| `grid0` | 8 | 24 | 12 | `false` | - |
| `grid1` | 10 | 30 | 16 | `true` | `Grid0` |
| `grid2` | 12 | 36 | 20 | `true` | `Grid1` |
| `grid3` | 14 | 42 | 24 | `true` | `Grid2` |
| `grid4`..`grid14` | 16 | 48 | 28 | `true` | previous grid |
| `grid15` | 16 | 48 | 0 | `true` | `Grid14` |

`grid12`..`grid15` additionally carry `Group="1"`; `grid0`..`grid11` carry no
`Group`. `grid15`'s `RequiredPoints="0"` renders as `FE_NA` rather than a number
(`GridSelection_Update`, below), and no `<Unlock Grid="Grid15">` row appears in
any of the sixteen grid files - `PI_Track`/`PI_TeamModel` rows were not
re-checked for one, so "nothing depends on grid15" is a reading of the grid
files alone.

**The `Locked` column above is reported, not interpreted.** `Locked` is parsed
onto a `PI_Grid` at `+0xa0` and onto a `PI_Cell` at `+0xb9`, and **no consumer
of either byte was traced this pass**. It is not `Definition_IsUnlocked`, which
reads the hard-hidden byte at `+0x99` and the `<Unlock>` list at `+0x9c`; and it
is not `FUN_0888e5e4`, the function both screens call into `screen + 0xd8`,
which compares the definition's source path at `+0x94` against `"ms:"` and
`"PID"` and is therefore a **memory-stick/DLC-source** test, not a lock. So
whether `Locked="true"` drives `Cell Selection`'s `Lock_x_y` overlay, or is
redundant against the `<Unlock>` rows that sit beside it on every locked grid,
is **open**. Confidence 50 on any reading of it; the column is transcribed from
the files so the next pass does not have to re-extract them.

**236 cells, 708 points maximum.** The campaign covers 24 distinct
`NN_Track` names and all seven modes:

| Mode | Cells | Gold / Silver / Bronze target | Laps |
| --- | ---: | --- | --- |
| `Race` | 59 | `1` / `2` / `3` (finishing position) | 3 Venom, 4 Flash, 4 Rapier, 5 Phantom |
| `Time Trial` | 47 | a time in centiseconds, per cell | same per-class table |
| `Speed Lap` | 42 | a lap time in centiseconds, per cell | always `7` |
| `Tournament` | 27 | `1` / `2` / `3` (overall placing) | same per-class table |
| `Head2Head` | 23 | `1` / `0` / `0` (win or nothing) | 4 Flash, 4 Rapier, 5 Phantom |
| `Elimination` | 22 | `10` / `7` / `5` kills, identical on all 22 | no `laps` attribute |
| `Zone` | 16 | 18-24 gold, per track | `0` |

**The lap count is not a guess any more.** Across all 236 cells the
`laps` attribute is *exactly* 3 for Venom, 4 for Flash, 4 for Rapier and 5 for
Phantom, with no exception, and Speed Lap is 7 everywhere. That is the hard
version of `MSC_LOAD_VENOM`/`FLASH`/`RAPIER`/`PHANTOM`'s hedged "most events"
prose, which
[`race-setup.md`](../../../formats/race-setup.md) could only score 78.
Confidence **90** - a flat census over 236 authored records, no exceptions.

`Elimination`'s gold target of `10` is the **same number** `FEData.wad`'s 24
`stats.xml` records carry as `<Targets Elimination="10"/>`, independently
recovered. Zone's grid targets (18-24) are *not* `FEData`'s `Zone="25"`, so that
constant is a default for some other path (custom race, most likely) and not the
campaign's.

Every cell carries `ship="None" ShipChoice="Yes"` - the campaign never forces a
craft. `AICount` is `7` on `Race`/`Tournament`/`Elimination`, `1` on
`Head2Head`, and absent on the solo modes.

## The parsers

### `PI_Cell_ParseElement` (`0x088bf83c`)

One element handler per `PI_Cell` child. `<Values>` attributes land as:

| Attribute | Offset | Type |
| --- | --- | --- |
| `laps` | `+0xac` | int |
| `AICount` | `+0xb0` | int |
| `Weapons` | `+0xb4` | bool |
| `damage` | `+0xb5` | bool |
| `ShipChoice` | `+0xb6` | bool |
| `Status` | `+0xb8` | bool |
| `Locked` | `+0xb9` | bool |
| `track` | `+0xba` | string, 16 |
| `ship` | `+0xca` | string, 16 |
| `mode` | `+0xdc` | enum, table at `0x08ab062c` |
| `class` | `+0xe0` | enum, table at `0x08ab067c` |
| `skill` | `+0xe4` | float |
| `skillEasy` | `+0xe8` | float, defaults to `skill - 1.0` |
| `skillHard` | `+0xec` | float, defaults to `skill + 1.0` |

and the child elements as:

| Element | Offset |
| --- | --- |
| `<Gold Target=>` | `+0xa0` |
| `<Silver Target=>` | `+0xa4` |
| `<Bronze Target=>` | `+0xa8` |
| `<TournamentTrack track=>` | count at `+0xf0`, name hashes at `+0xf4`, **12 slots**, index wraps `% 0xc` |

Anything else falls through to `Definition_ParseUnlock` (`0x0888f094`).

**Targets are decimal, despite being read by `Xml_AttributeAsIntHex`.** Two
corroborations: `Elimination`'s gold `10` matches `FEData`'s
`Targets Elimination="10"` exactly, and `Zone`'s `20` would be `32` read as hex,
which no `Zone` record supports. Times are **centiseconds** - the same unit
`Hud_UpdateTimeCluster_q` feeds `FUN_088196e4` after multiplying a float
seconds value by `100.0`, and the same formatter (`FUN_08819878`) `Cell
Selection` uses on a `Time Trial`/`Speed Lap` target.

Two more things this function does at the end of `<Values>`:

- **It creates the cell's save record if there is none**: `FUN_0880871c(store,
  name, 0xc)` then writes 12 bytes `{ best = 0 for Zone/Elimination else -1,
  bestTime = -1, difficulty = 0xff, medal = 0xff }`. That is the record layout
  everything below reads.
- **It parses the cell's grid coordinates out of its own name.**
  `FUN_089733ec(name, '_')` is a plain `strchr` (read in full at `0x089733ec` -
  it stops at the *first* match), so `grid0_2_1` gives `atoi("2_1") = 2` into
  `+0x124` and `atoi("1") = 1` into `+0x128`. **This only works because a grid
  name contains no underscore of its own and both coordinates are a single
  digit** - a real constraint on any reimplementation, and the reason
  `CellSelection_OnEnter` can go the other way with the format string
  `"%s %d %d"` / `"UserGrid_%d_%d"`.

### `PI_Grid_ParseElement` (`0x088c0240`)

Much smaller. `<Values RequiredPoints=>` to `+0xa4` (again via
`Xml_AttributeAsIntHex`, again decimal in practice: `12`, `16`, `20`, `24`,
`28`), `Locked` to `+0xa0`, `Group` to `+0xb0`. Everything else goes to
`Definition_ParseUnlock`.

### `Definition_ParseUnlock` (`0x0888f094`) - the full `<Unlock>` row schema

Allocates a 0x44-byte node and appends it to the definition's `+0x9c` list -
the list [`race-box-screens.md`](race-box-screens.md)'s `Definition_IsUnlocked`
walks. Attribute to field:

| Attribute | Offset | | Attribute | Offset |
| --- | --- | --- | --- | --- |
| `Exclusive` | `+0x00` (bool) | | `Team` | `+0x24` |
| *(next pointer)* | `+0x04` | | `Track` | `+0x28` |
| `ClassIs` | `+0x08` | | `Tournament` | `+0x2c` |
| `TeamIs` | `+0x0c` | | `Grid` | `+0x30` |
| `TrackIs` | `+0x10` | | `Medal` | `+0x34` |
| `TournamentIs` | `+0x14` | | `MedalCount` | `+0x38` |
| `ModeIs` | `+0x18` | | `TimePlayed` | `+0x3c` |
| `GridIs` | `+0x1c` | | `Loyalty` | `+0x40` |
| `Class` | `+0x20` | | | |

A sibling `<Available Available="..."/>` element sets the definition's `+0x9a`.
`Exclusive` is true when the string reads as one of the two tokens at
`0x08a7d688`/`0x08a7d690`; the rest of the row's semantics are
`Definition_IsUnlocked`'s, already documented.

Each of the four unlock predicates `Definition_IsUnlocked` dispatches to is
guarded by an accessor that returns zero when its own field is absent, so a row
only exercises the conditions it actually names:

| Guard | Field | Predicate | Passes when |
| --- | --- | --- | --- |
| `Unlock_MedalValue` (`0x0888ef80`) | `Medal` | `Unlock_MedalMet` (`0x0888e86c`) | a saved medal at least that good exists for the named track/tournament |

`Unlock_MedalValue` maps the row's `Medal=` string to the **same ordinal
`Cell_EvaluateMedal` produces**: `Gold` to `0`, `Silver` to `1`, `Bronze` and
`any` to `2`, an unrecognised string to `3`, an absent attribute to `0xff`. That
is independent corroboration of the medal ordinal's direction from a second,
unrelated site.

| `Unlock_MedalCountValue` (`0x0888f034`) | `MedalCount` | `Unlock_MedalCountMet` (`0x0888e9e0`) | `required <= FUN_0888a414()`, the profile's total medal count |
| `Unlock_LoyaltyValue` (`0x0888f064`) | `Loyalty` | `Unlock_LoyaltyMet` (`0x0888ea30`) | a team's record word reaches the required value |
| `Unlock_GridName` (`0x0888ef1c`) | `Grid` | `Unlock_GridPointsMet` (`0x0888ebd8`) | **the named grid's earned points reach its `RequiredPoints`** |

## `Grid0`..`Grid14`: what the gate actually evaluates

`Unlock_GridPointsMet` (`0x0888ebd8`), in full:

1. `Unlock_GridName` (`0x0888ef1c`) yields the `<Unlock Grid="...">` string, or
   the *currently selected* grid when the string is `asSelected`.
2. Collect every `PI_Grid` definition and find the one whose name (`+0x74`)
   matches case-insensitively - which is why `Grid0` in an `<Unlock>` row and
   `grid0` in `grid_00.xml` are the same thing.
3. Pass if `Grid_PointsEarned(grid) >= grid->RequiredPoints` (`+0xa4`).

So `<Unlock Grid="Grid0"/>` on a `PI_Track` means **"you have scored at least 12
points in grid0"**, and nothing else. The 21-of-24 `PI_Track`s and every
`PI_TeamModel`/`PI_ModelSkin` carrying a `Grid0`..`Grid10` row that
`race-setup.md` already catalogued are gated on exactly this, on the same grids
the campaign screens walk. Confidence **88** - the predicate reads unambiguously
and the sixteen authored `RequiredPoints` values are consistent with it (a
strictly rising 12/16/20/24/28 ladder against a strictly rising 24/30/36/42/48
maximum, i.e. always between half and two-thirds of a grid's points).

## Medals and points

### `Cell_EvaluateMedal` (`0x088bf620`) - the medal evaluator

This is the law, and it is small:

```
Cell_EvaluateMedal(cell, value):
    if value == 0 or value == 0xffffffff: return 0xff       # no result, no medal
    for tier in 0, 1, 2:                                    # gold, silver, bronze
        target = *(u32 *)(cell + 0xa0 + tier * 4)
        if cell->mode is Zone (6) or Elimination (8):
            if value >= target: return tier                 # more is better
        else:
            if value <= target: return tier                 # less is better
    return 0xff
```

The **tier ordinal is 0 = gold, 1 = silver, 2 = bronze, 0xff = none**, and the
direction of the comparison flips for the two counting modes. `Race`,
`Tournament` and `Head2Head` pass a finishing *position*, so `<Gold Target="1"/>`
means first place; `Time Trial`/`Speed Lap` pass a time in centiseconds; `Zone`
passes a zone count and `Elimination` a kill count. Confidence **88** -
unambiguous decompilation, and the authored target triples corroborate the
direction on every one of the 236 cells (descending `20/17/15` for `Zone`,
ascending `1/2/3` for `Race`).

### `Cell_MedalPoints` (`0x088bf530`) - the points table, measured

```
Cell_MedalPoints(cell, tier):
    if tier == 0xff: tier = Cell_BestMedal(cell); if still 0xff: return 0
    if tier == 0: return 3      # gold
    if tier == 1: return 2      # silver
    if tier == 2: return 1      # bronze
    return 0
```

`race-setup.md` recorded "no points-per-position or points-per-race-type table
was found anywhere on disc". There is none, because the points come from the
medal, not the position: **gold 3, silver 2, bronze 1**. Called with `tier =
0xff` it means "what did the player actually score"; called with `tier = 0` it
means "what is this cell worth at best", which is always 3. Confidence **90** -
the whole function is four constants, and `Grid_PointsPossible` below only makes
sense under this reading.

### The saved record, and its accessors

Records live in a linked list at `profile + 0x43c`, walked by `FUN_08808624`
matching a name hash; `FUN_088085d0` returns that node **plus 0x64**, which is
the 12-byte payload `PI_Cell_ParseElement` initialises:

| Payload offset | Meaning | Written by |
| --- | --- | --- |
| `+0x0` | best result (position, time, zones or kills) | `Race_RecordResult` |
| `+0x4` | best race/lap time | `Race_RecordResult` |
| `+0x8` | difficulty the best medal was set at | `Race_RecordResult` |
| `+0x9` | **best medal ordinal**, `0xff` for none | `Race_RecordResult` |

| Accessor | Returns |
| --- | --- |
| `Cell_BestMedal` (`0x088bf5b4`) | `Cell_EvaluateMedal(cell, record[0])` - the medal the *stored best* is worth right now |
| `Cell_SavedMedal` (`0x088bf6e4`) | `record+9`, the medal as actually banked |
| `Cell_SavedDifficulty` (`0x088bf6a4`) | `record+8` |
| `Cell_SavedRecord` (`0x088bf71c`) | `record[1]` for `Race`/`Tournament`/`Head2Head`, `record[0]` otherwise; `-1` when no record exists |
| `Cell_SkillForDifficulty` (`0x088bf808`) | `skillEasy` / `skill` / `skillHard` for difficulty 0 / 1 / 2 |

**`FUN_088085d0` and `FUN_08808624` are deliberately left unnamed.** Ghidra
recovers three parameters where every call site passes four; the fourth reads as
a create-if-missing flag (`PI_Cell_ParseElement` passes `1` right after
`FUN_0880871c` has created the record, `Unlock_GridPointsMet`'s helper passes `0`
and null-checks the result) but that is inference, not a read, so it stays below
the naming floor. Separately, [`race-box-screens.md`](race-box-screens.md)
describes `FUN_088085d0` as "a handling-stats-shaped table" in
`TrackSelection_ApplySelection`; on this reading it is the **profile record
store**, and the summed lap/stage records that page saw are saved bests, not
handling stats.

### Grid aggregates

| Function | Returns |
| --- | --- |
| `Grid_PointsEarned` (`0x088c048c`) | sum of `Cell_MedalPoints(cell, 0xff)` over the grid's cells |
| `Grid_PointsPossible` (`0x088c0570`) | sum of `Cell_MedalPoints(cell, 0)` - i.e. `3 x cellCount` |
| `Grid_CountMedalsAtLeast` (`0x088c0398`) | count of cells whose best medal ordinal is `<= tier` |
| `Grid_CellCount` (`0x088c0654`) | number of cells |

`Grid_PointsEarned` is what the unlock gate compares against `RequiredPoints`,
and `3 x cellCount` reproducing the authored 24/30/36/42/48 maxima exactly is
the arithmetic invariant this section rests on.

## `Race_RecordResult` (`0x0880ae54`) - where a result becomes a medal

Called at the end of an event. It switches on `mode - 3` and picks the value to
bank per mode, then evaluates and stores:

| `mode` | Value banked | Notes |
| --- | --- | --- |
| 3 `Race`, 9 `Head2Head`, 14, 15 | finishing position (`param_3`) | also writes the top-3 position/name table at `record + 0x5dc`, stride `0x24` per class |
| 4 `Tournament`, 16 | finishing position, **only on the last leg** (`DAT_08b30fa4 == DAT_08b30fa0 - 1`) | and a **second record lookup** first - see below |
| 5 `Time Trial`, 17 | race time (`param_2`) | |
| 6 `Zone` | zone count (`*param_4`, a `u16`) | |
| 7 | nothing | the enum gap |
| 8 `Elimination` | kill count (`param_5`) | |
| 10 `Speed Lap` | best lap (`param_6`) | |

For every mode with a campaign cell in play (`DAT_08b30ffc != 0`) it then:

1. improves `record[0]` (and `record[1]` for `Race`) if the new value beats it -
   `<` for time/position modes, `>` for `Zone`/`Elimination`,
2. computes `medal = Cell_EvaluateMedal(cell, value)`,
3. keeps the **better** of the new and stored medal in `record+9`, and stores the
   difficulty alongside it in `record+8`, preferring the higher difficulty on a
   tie,
4. sets the profile's dirty flag (`+0x45e`) so the campaign autosaves - the
   mechanism behind `TournamentLoad`'s `MSC_MSG_AUTOSAVE3` dialog, and
5. increments the profile's gold/silver/bronze counters at `+0x160`/`+0x164`/
   `+0x168`, **but only when the grid in play is not the player's own custom
   grid** (`DAT_08b30fb8 + 0xb4`). A custom grid earns medals for its own cells
   and contributes nothing to the profile totals `MedalCount` unlocks read.

Confidence **85** - the branch structure and the mode dispatch are unambiguous
and line up exactly with the mode enum below, but the six parameters are only
identified by which mode consumes them, not from a caller.

**The `Tournament` arm is the one that is structurally different, and it is
worth a next pass on its own.** Before touching the cell record at all it hashes
`DAT_08b31158 + 0x74` through `FUN_08945890` and looks up a **second** record
with `FUN_088085d0(profile, hash, 0, 0)`, writing the finishing position into
that record's per-class slot at `+ class*0x24` and a name beside it. Only then
does it fall through to the cell record and `Cell_EvaluateMedal`. That second
record reads as the tournament's own standings - the state behind `ER_TOUR_STAN`
("Tournament standings"), `ER_RACE_POINTS` and the eight `ER_END_TOUR_1..8`
placement strings the earlier pass recovered, and behind `MSC_EVENT_TOURN`'s
"you can save your tournament progress between races". `DAT_08b31158` was not
identified and the per-leg accumulation was not traced. Tournament is the mode
with 27 authored cells and the only one carrying per-leg state, so this is the
concrete starting point for it.

## The mode and class enumerations, read off their own tables

`PI_Cell_ParseElement` resolves `mode=` against a null-terminated
`{ value, name }` table at `0x08ab062c` and `class=` against one at
`0x08ab067c`. Read directly:

| `mode` | Name | | `class` | Name |
| ---: | --- | --- | ---: | --- |
| 3 | `Race` | | 0 | `Venom` |
| 4 | `Tournament` | | 1 | `Flash` |
| 5 | `Time Trial` | | 2 | `Rapier` |
| 6 | `Zone` | | 3 | `Phantom` |
| 8 | `Elimination` | | | |
| 9 | `Head2Head` | | | |
| 10 | `Speed Lap` | | | |
| 11 | `Custom Grid` | | | |
| 12 | `AI Race` | | | |

**7 is absent from the table**, and `Race_RecordResult`'s `case 4:` (that is
`mode == 7`) banks nothing - consistent with a retired or debug mode. `11 Custom
Grid` and `12 AI Race` are two modes no source read so far had named; `Custom
Grid` is what a player-built grid's cells run as, which is why
`CellSelection_PopulateDetail` blanks the speed-class help and the lap row for
it. Confidence **92** - the tables round-trip: every `mode=` string in all
sixteen grid files is one of these nine, and every `class=` one of these four.

## The screens

Both screen classes register the way [`race-box-screens.md`](race-box-screens.md)
documents (`ScreenClass_Register`, a direct vtable store, a list prepend), and
both vtables share the eight root-class slots that page identified. The
per-screen slots:

| | `GridSelection` | `CellSelection` |
| --- | --- | --- |
| Constructor | `0x088df1a4` | `0x088d799c` |
| vtable | `0x08ad0064` | `0x08acfb64` |
| `Update` (word 9) | `0x088dec24` | `0x088d6430` |
| `OnEnter` (word 29) | `0x088de0f0` | `0x088d59c4` |

### `GridSelection_Update` (`0x088dec24`) fills the three stat rows

Up/down moves the selection with a `DECLINE`/`UPDOWN` sound, then on a change:

| Widget | Fed by | Format |
| --- | --- | --- |
| `Title` | the grid's own name (`+0x74`), else `MSC_NONE` | - |
| `Medals` | `Grid_CountMedalsAtLeast(grid, 0)` / `Grid_CellCount(grid)` | `"%d/%d"` |
| `Points` | `Grid_PointsEarned(grid)` / `Grid_PointsPossible(grid)` | `"%03d/%03d"` |
| `Required` | `grid->RequiredPoints`, or `FE_NA` when it is 0 | `"%d"` |
| `honey` | `index*4+1`, `index*4+4`, `max*4+4` | `"%d-%d / %d"` |

So the `Medals "00/16"` placeholder in `CellMode_Definition.xml` is **gold
medals over cell count** - the `16` is the cell count of a full grid, and the
row counts golds specifically (`tier <= 0`), not medals of any colour. The
`Points "000/110"` placeholder is a template width; a real grid maxes at 48.

### `CellSelection_OnEnter` (`0x088d59c4`) and the cell lookup

Formats the cell name it wants - `"UserGrid_%d_%d"` for a player grid
(`DAT_08b30fb8 + 0xb4` set) or `"%s %d %d"` from the current grid's name
otherwise - then scans the collected cell definitions for a name match and a
coordinate match. The `%s %d %d` spacing rather than `%s_%d_%d` is what the
comparison uses; the underscore form at `0x08a78364` is the *serialiser* for a
user-built grid (`<PI_Cell name="%s_%d_%d">`, `0x08a783a4`).

### `CellSelection_PopulateDetail` (`0x088d68d8`) fills the panel

Everything `race-setup.md` read as a placeholder, sourced:

| Widget | Source |
| --- | --- |
| `Title` | the cell's mode name, via the mode enum table |
| `Track Line` | `cell->track` (`+0xba`), or `"%d Races"` on `cell->TournamentTrack` count for `Tournament` |
| `Line1` | the class name, blanked for `Zone` |
| `Line2` | `cell->laps`, or `RC_INF` when `laps < 1` or the mode is `Zone` |
| `Line3` | `FE_ON`/`FE_OFF` from `cell->Weapons`, only for `Race`/`Head2Head`/`Tournament` |
| `Line6` | `Cell_MedalPoints(cell, 0xff)` / `Cell_MedalPoints(cell, 0)`, `"%d/%d"` |
| `Line7` | `IG_HUD_GOLD`/`SILVER`/`BRONZE`/`MSC_NONE` from `Cell_SavedMedal`, suffixed with the difficulty from `Cell_SavedDifficulty` |
| `Line8` / `Line5` | `MSC_CAMREC` and `Cell_SavedRecord`, formatted as a time, `"%d IG_HUD_ZONES"` or `"%d IG_HUD_KILLS"` per mode |
| `Target0..2` | `cell + 0xa0 + i*4`, as a time for `Time Trial`/`Speed Lap` and a plain number otherwise |
| `Speed Class Help` | `MSC_LOAD_VENOM`/`FLASH`/`RAPIER`/`PHANTOM` by class |
| `Event Help` | `MSC_EVENT_SR`/`TOURN`/`TT`/`ZONE`/`ELIM`/`HTH`/`SL` by mode |

**The `Target0..2` widgets are hidden for `Race`, `Tournament` and
`Head2Head`** (the code clears their visibility bit) and shown for `Time
Trial`, `Zone`, `Elimination` and `Speed Lap`. That is why a player never sees
"gold = 1st place" written out: for a race the position targets are implicit.

`race-setup.md` counts the hex grid at **32 cells**, from the authored
`Medal_x_y` widget nodes. Both counts are right and they measure different
things: 32 is the *widget* bounding shape a `CellSelection` screen can display,
and the campaign's grids fill 8, 10, 12, 14 or 16 of those positions, addressed
by the `(x, y)` parsed out of each cell's name. A cell with no definition simply
has no widget content.

## Where `FEData.wad`'s per-track records are actually read

`TrackStats_Load` (`0x088c454c`) opens `"%s\stats.xml"` or
`"%s\stats_reversed.xml"` (`0x08a812b4` / `0x08a8129c`) using the track
definition's own directory name at `+0x94`, and feeds every child element to
`TrackStats_ParseElement` (`0x088c46f8`), which writes onto the **`PI_Track`
definition**:

| Element / attribute | Offset on the track definition |
| --- | --- |
| `<RaceTimes Venom Flash Rapier Phantom>` | `+0xa0`, `+0xa4`, `+0xa8`, `+0xac` (float) |
| `<LapTimes Venom Flash Rapier Phantom>` | `+0xb0`, `+0xb4`, `+0xb8`, `+0xbc` (float) |
| `<Targets Elimination=>` | `+0xc0` (int) |
| `<Targets Zone=>` | `+0xc4` (int) |
| `<Physical Length=>` | `+0xdc` (int) |
| `<SkillLevels><Entry Difficulty Class SkillScaleValue>` | `+0xe0 + class*0xc + difficulty*4` (float) |
| `<SkillLevels><ModeModifiers Class HeadToHead FullGridWithWeapons HalfGridWithWeapons FullGridWithoutWeapons HalfGridWithoutWeapons>` | `+0x110 + class*0x14`, five floats |

The element names are `SkillLevels`, `Entry` and `ModeModifiers` - `race-setup.md`
guessed `<l>`, `<c>` and `<f>` from the `<code>` shortening and can now be
corrected. `SkillScaleValue` defaults to `1.0`/`2.0`/`3.0` for
Easy/Medium/Hard before parsing, so the value is an index-shaped number on a
three-point curve, not a raw multiplier. Confidence **90** - the parser's field
layout reproduces the 24 shipped records exactly, and `AI_ResolveSkillScale`
below consumes the same offsets.

### `AI_ResolveSkillScale` (`0x08834df4`) - the one place the campaign touches this table

```
skill = track->SkillScaleValue[class][g_skill_level]          # default 2.0
if mode == 9 (Head2Head):  skill += track->ModeModifiers[class].HeadToHead
if mode == 3 (Race):       skill += the one of the four Full/Half x With/Without
                                    Weapons fields matching the grid size and
                                    the weapons switch  (full grid == 8 ships)
if a campaign cell is in play and it is not a user grid:
    t     = Cell_SkillForDifficulty(cell, g_skill_level)      # skillEasy/skill/skillHard
    skill = t < 2.0 ? lerp(track.skill[0], track.skill[1], t - 1.0)
                    : lerp(track.skill[1], track.skill[2], t - 2.0)
```

So a cell's `skillEasy="1.1" skill="1.75" skillHard="2.5"` is a **position on
that track's own Easy/Medium/Hard skill curve**, and a campaign cell *replaces*
the mode-modifier arithmetic rather than adding to it. This is the entire
connection between the campaign and `FEData.wad`'s per-track family, and it
closes `ai-stats.md`'s "`SkillScaleValue` appears in the string table and in
none of the nine functions, unchased". Confidence **85** - decompilation only,
but the four `ModeModifiers` fields are selected in exactly the combination
their names describe, which is a strong consistency check.

`FUN_088347b8` (the interpolator) and `FUN_0888a414` (the profile's total medal
count) were not read further and are not renamed.

## `Eliminator_UpdateKillTarget` (`0x0882ce18`)

Out of this page's main line but recovered on the way, and it is the first
*measured* Eliminator ending: the mode's per-tick update takes the kill target
from **`cell->Gold Target` (`+0xa0`) when a campaign cell is in play**, and from
`DAT_08b30fb0` otherwise. It ends the race (`RaceMode_SetState(race, 3)`) as
soon as any ship's kill count (`+0x8d8`) reaches that target, or the player's
ship reaches state 2. Confidence **80** - clean read, but the surrounding mode
state machine was not traced.

## What is not determined

- **The in-race HUD's medal tier is a different value and is still unread.**
  [`hud.md`](../../../ui/hud.md) records `Hud_UpdateTimeCluster_q`
  (`0x0881c9d0`) picking a caption from a five-way table on an ordinal where
  `0 = BRONZE, 1 = SILVER, 2 = GOLD, 3 = RECORD`. `Cell_EvaluateMedal` runs the
  **opposite** way (`0 = gold`) and has no `RECORD` tier at all, so the two are
  not the same number and this page does not close that item. What is pinned
  down for the next pass: the field is `*(hud + 0x3c) + 0x34`, cached against
  `hud + 0x190` so the localised string is re-resolved only on a change; its
  siblings are `+0x30` (the target value, formatted by `FUN_08819878` or
  `FUN_088196e4`) and `+0x38` (a bool that turns the number red). Ruled out as
  its writer: `Hud_BindWidgets`' two `DAT_08b30ffc` reads at `0x088207d8` /
  `0x088207e4` (a results-screen `sprintf` of `cell + 0xa0`),
  `Eliminator_UpdateKillTarget_q` and `AI_ResolveSkillScale`. Confidence **50**,
  deliberately not renamed.
- **`Status` (`+0xb8`) and `Locked` (`+0xb9`) on a `PI_Cell`, and `Locked`
  (`+0xa0`) on a `PI_Grid`**, are parsed and no consumer of any of them was
  traced. No shipped grid file sets `Status`; `Locked="false"` appears on a
  handful of `grid_00`/`grid_03` cells only. `Definition_IsUnlocked` and
  `FUN_0888e5e4` are both ruled out as the reader - see the grid table above.
  Whether these bytes drive `Cell Selection`'s `Lock_x_y` overlay at all is
  open, at 50.
- **`Group` (`+0xb0`) on a `PI_Grid`** is `1` on `grid12`..`grid15` and absent
  elsewhere; its consumer was not traced. A "these four are the expert set"
  reading is plausible and unverified.
- **The `Loyalty` unlock predicate** (`Unlock_LoyaltyMet`) compares the required
  value against a whole 32-bit word at the record's `+8`, which under this
  page's record layout is `difficulty | medal << 8`. That does not read as a
  loyalty counter, so either the team record's payload is laid out differently
  from a cell's or the comparison is doing something this pass did not follow.
  Scored 72 and named `_q` on the strength of the field it reads, not the
  arithmetic.
- **Only the USA pressing was read.** The EU and PS2 pressings of Pulse, and
  Pure and HD/Fury, were not checked for `Data\Plugins\grids` at all.
- **Nothing is runtime-verified.** No PPSSPP breakpoint was taken this pass;
  every score is capped at the rubric's 85-94 "arithmetic invariant across many
  real files" band and most sit at 84-90.

## Reproducing the data

```sh
just wad cat data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
  'Data\Plugins\grids\Definition.xml'
just wad cat data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
  'Data\Plugins\grids\grid_00.xml'
```

Each file's `<code as="..." bs="..." ...>` first line is its own tag/attribute
dictionary; expand `<x y="...">` by looking `x` and `y` up as `xs`/`ys`.

## Names recovered

| Address | Name | Conf |
| --- | --- | ---: |
| `0x088bf83c` | `PI_Cell_ParseElement` | 88 |
| `0x088c0240` | `PI_Grid_ParseElement` | 88 |
| `0x0888f094` | `Definition_ParseUnlock` | 85 |
| `0x088bf620` | `Cell_EvaluateMedal` | 88 |
| `0x088bf530` | `Cell_MedalPoints` | 90 |
| `0x088bf5b4` | `Cell_BestMedal` | 82 |
| `0x088bf6e4` | `Cell_SavedMedal` | 82 |
| `0x088bf6a4` | `Cell_SavedDifficulty` | 80 |
| `0x088bf71c` | `Cell_SavedRecord` | 80 |
| `0x088bf808` | `Cell_SkillForDifficulty` | 85 |
| `0x088c048c` | `Grid_PointsEarned` | 88 |
| `0x088c0570` | `Grid_PointsPossible` | 85 |
| `0x088c0398` | `Grid_CountMedalsAtLeast` | 85 |
| `0x088c0654` | `Grid_CellCount` | 85 |
| `0x0888ebd8` | `Unlock_GridPointsMet` | 85 |
| `0x0888ef1c` | `Unlock_GridName` | 82 |
| `0x0888e9e0` | `Unlock_MedalCountMet` | 80 |
| `0x0888f034` | `Unlock_MedalCountValue` | 80 |
| `0x0888e86c` | `Unlock_MedalMet` | 72 |
| `0x0888ef80` | `Unlock_MedalValue` | 85 |
| `0x0888ea30` | `Unlock_LoyaltyMet` | 72 |
| `0x0888f064` | `Unlock_LoyaltyValue` | 78 |
| `0x0888edfc` | `Unlock_TeamName` | 78 |
| `0x0888eeb8` | `Unlock_TournamentName` | 78 |
| `0x0880ae54` | `Race_RecordResult` | 85 |
| `0x088c454c` | `TrackStats_Load` | 90 |
| `0x088c46f8` | `TrackStats_ParseElement` | 90 |
| `0x08834df4` | `AI_ResolveSkillScale` | 85 |
| `0x0882ce18` | `Eliminator_UpdateKillTarget` | 80 |
| `0x089733ec` | `Libc_StrChr` | 92 |
| `0x088df1a4` | `GridSelection_Construct` | 78 |
| `0x088d799c` | `CellSelection_Construct` | 78 |
| `0x088de0f0` | `GridSelection_OnEnter` | 80 |
| `0x088de380` | `GridSelection_PopulateGrids` | 80 |
| `0x088dec24` | `GridSelection_Update` | 85 |
| `0x088d59c4` | `CellSelection_OnEnter` | 80 |
| `0x088d68d8` | `CellSelection_PopulateDetail` | 85 |
| `0x08ab062c` | `g_mode_name_table` (data) | 92 |
| `0x08ab067c` | `g_class_name_table` (data) | 92 |
