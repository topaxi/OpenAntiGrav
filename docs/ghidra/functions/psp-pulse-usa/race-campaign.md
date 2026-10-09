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

**The `Locked` column above is reported, not interpreted, as of this
pass.** `Locked` is parsed onto a `PI_Grid` at `+0xa0` and onto a `PI_Cell`
at `+0xb9`, and **no consumer of either byte was traced this pass**. **A
later pass, 2026-09-14, did find one for each - see ["Unlock rules, cell and
tier"](#unlock-rules-cell-and-tier) below, which supersedes the "is open"
sentence a few lines down (not the offsets or the `Definition_IsUnlocked`
negative result, both still correct).** It is not `Definition_IsUnlocked`, which
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
`Hud_UpdateTimeCluster` feeds `FUN_088196e4` after multiplying a float
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
   difficulty alongside it in `record+8` under the exact rule "The `record+8`/
   `record+9` write, decompiled in full" below (2026-09-28) settles precisely -
   preferring the higher difficulty on a tie, but **unconditionally overwriting
   the stored difficulty on a strict medal improvement, even downward**,
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

**Closed, 2026-09-14, a separate pass**: [`tournament.md`](tournament.md) has
`DAT_08b31158` fully fielded, the per-leg points table, and what value the
medal above actually compares for a Tournament cell (the final standings
rank by total points, not the last leg's own position).

### The `record+8`/`record+9` write, decompiled in full, 2026-09-28

`Race_RecordResult`'s own tail, common to every mode, once `uVar8` (the
medal `Cell_EvaluateMedal` just computed, `0` gold .. `2` bronze, `0xff` for
none) is in hand:

```
record = FUN_088085d0(profile, hash(cell.name), 0, 1)
stored_medal = record+9        // 0xff on a fresh record
stored_difficulty = record+8   // 0xff on a fresh record
if stored_medal == 0xff or uVar8 <= stored_medal:   // no downgrade
    if stored_difficulty == 0xff or uVar8 < stored_medal:
        record+8 = g_skill_level     // unconditional overwrite
    elif uVar8 == stored_medal and stored_difficulty < g_skill_level:
        record+8 = g_skill_level     // tie: harder rung wins
    record+9 = uVar8
```

Two consequences neither this page nor `oag_game::records::Store::record_campaign`
previously carried, both confidence **88** (a direct read of the byte
comparisons, no inference beyond the ordinal convention `Cell_EvaluateMedal`
already established):

- **A strict medal improvement overwrites `record+8` unconditionally**, even
  to an *easier* rung than what was stored - there is no comparison against
  the old difficulty in that branch at all. A bronze medal stored at `Hard`
  becomes silver-at-`Easy` if that is what the very next, easier run scores,
  because silver beats bronze outright; the difficulty stamped is simply
  whatever `g_skill_level` (see below) was on the run that produced the
  improvement, full stop.
- **Only an exact tie in medal tier compares difficulty at all**, and even
  then only ever *raises* the stored rung, never lowers it. This is the one
  branch the previous read of this function ("preferring the higher
  difficulty on a tie") had right; the "unconditional on improvement" branch
  was not read before this pass.

`oag_game::records::Store::record_campaign`'s own `improves` match arm
previously implemented neither of these - it treated *any* difficulty
mismatch as difficulty-dominant ("the harder rung wins outright, whatever
either medal is"), reasoned from HD's `SaveData_MigrateCellMedalsToHardElite`
grandfather clause with no comparison function found to check it against.
This decompile is that comparison function, on Pulse; `Store::record_campaign`
now matches it exactly.

**This is one function with no title branch, so the corrected rule applies
to HD's own recorded medals too - not a Pulse-only fix left sitting beside
an untouched HD one.** There never was a separate HD rule in the code:
`record_campaign`'s `applies`/`overwrite_difficulty` logic is the single
shared implementation both titles' campaign launches call into, and always
was. HD's own equivalent to `Race_RecordResult` was **not decompiled this
pass**, so whether the corrected algorithm is *literally* right for HD
specifically remains unconfirmed - but it now rests on materially stronger
evidence than the rule it replaced, which was never HD-measured either:
`docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s own medal-law
section records "no comparison function found to check it against" for the
old "harder wins outright" reading. The new rule *is* that comparison
function, just read off Pulse rather than HD - on a title confirmed to
share HD's own profile-record primitives and even the literal
`"DifficultyRC"` string (present in `/hdfury/EBOOT-ps3-hdfury-eu.elf` too,
`0x0077a5a0`, the same PI001-lineage `DifficultyButton` mechanism
`docs/ui/campaign-screens.md`'s HD section documents). A follow-up
decompile of HD's own `Race_RecordResult`-equivalent would either confirm
the shared rule outright, or, if HD genuinely diverges, be the evidence
`record_campaign` would need to branch by title on.

### The `DifficultyRC` persisted rung and Cell Selection's own square button, 2026-09-28

`g_skill_level` above is a global `int`, not this cell's own state - it is
the same **"AI difficulty"** rung `Single Player`'s own `Difficulty` row
persists under the global key `SkillLevel` (`docs/formats/race-setup.md`'s
`RB_AI_DIF` row), and Cell Selection's square button browses and commits the
identical value through a second, independent path:

- **`Profile_DifficultyRC` (`0x08809980`)**/**`Profile_SetDifficultyRC`**
  (`0x088098f0`) are a plain getter/setter pair over one more hashed record
  in the same per-profile record store `Cell_SavedDifficulty`/`Cell_SavedMedal`
  read - keyed on the literal string `"DifficultyRC"` (`Libc_HashString`),
  not a cell name. `CellSelection_OnEnter` loads it into the screen's own
  `+0xf0` byte on every entry; `FUN_08808300` (the profile object's own
  constructor - offsets `+0x43c`/`+0x45d..f`/`+0x160/164/168` match the
  documented record-list head, dirty flag and medal counters above, so this
  reads as `Profile_Construct`, left `FUN_`-prefixed pending a fuller check)
  seeds it to `1` (Medium) for a fresh profile - matching this build's own
  `CellSelection::difficulty` default and the PPSSPP capture below, which
  never observed anything but `"AI difficulty (Medium)"` on a fresh entry.
  Confidence **85** on both - clean decompilation, the hashed-key idiom
  already established for every other per-profile record on this page.
- **`CellSelection_Update` (`0x088d6430`, vtable word 9, cited by address
  since 2026-09-14 and decompiled and named this pass)** rebuilds the
  `DifficultyButton` widget's own text every frame:
  `sprintf("%s (%s)", resolve("RB_AI_DIF"), resolve(rung))`, where `rung` is
  `"Easy"`/`"Medium"`/`"Hard"` (bare idstrings, no `MSC_`/`FE_` prefix - the
  same three strings `Cell_SavedDifficulty`'s own three callers already use)
  keyed on the screen's own `+0xf0` byte, **not** `Cell_SavedDifficulty` -
  this is the *browsed* rung, unrelated to any medal already banked on the
  selected cell. `Square` (`Input_IsPressed(g_input, 7, 0)`) steps that byte
  `(+1) % 3` every press - wrapping, no upper/lower clamp. Confidence **85**.
- **`CellSelection_CommitSelection` (`0x088d6138`, already named, confidence
  93) is where the browsed rung actually takes effect**: on `Confirm` it
  writes the RaceBox's own `SkillLevel` global to `"Easy"`/`"Medium"`/`"Hard"`
  off the identical `+0xf0` byte (the same `%s (%s)`-adjacent string table at
  `0x08a826e4`), **and** calls `Profile_SetDifficultyRC` to persist that byte
  back to the profile - so the rung browsed on Cell Selection is what both
  drives the race's own AI skill (`AI_ResolveSkillScale`, reading the
  `SkillLevel` global `Single Player`'s own Difficulty row also writes) and
  survives to be read back next visit.
- **Line7's own difficulty suffix, `CellSelection_PopulateDetail`
  (`0x088d68d8`)**: when `Cell_SavedMedal` is not `0xff` (a medal is banked),
  it resolves `Cell_SavedDifficulty`'s own rung through the identical three
  bare idstrings and appends it with the identical `"%s (%s)"` format -
  `"Gold (Medium)"`, not a bare medal word - confirmed against
  `CellSelection_PopulateDetail`'s own raw decompile, confidence **88**.
  No medal (`0xff`) draws `MSC_NONE` alone, no suffix.

Measured live, PPSSPP, `pulse-psp-usa.chd`, Xvfb (`docs/ui/campaign-screens.md`'s
"AI difficulty (square) cycles" finding, 2026-09-25): `Square` on Cell
Selection changes only the bottom-bar label, `"AI difficulty (Medium)"` ->
`"(Hard)"` - matching this decompile's "browsed rung only, no other panel
change" reading exactly.

**The mechanism is shared with Wipeout HD/Fury, not Pulse-specific**:
`/hdfury/EBOOT-ps3-hdfury-eu.elf` carries the identical literal string
`"DifficultyRC"` (`0x0077a5a0`), the same PI001-lineage `DifficultyButton`/
`DifficultyButtonIcon` widget pair `docs/ui/campaign-screens.md`'s HD
section already documents. Not decompiled on HD this pass - see that
section's own note on what is and is not shared between the two titles'
exact wording and defaults.

**What recording a browsed rung does *not* mean, worth stating plainly
rather than leaving implicit**: `g_skill_level` never reaches this
project's own AI at all - `AI_ResolveSkillScale`
(`docs/ui/campaign-screens.md`'s "Not launched from a cell" note) is not
implemented, so a race launched from Cell Selection at any on-screen rung
still runs against whatever the ordinary `[ai] difficulty` setting has
opponents flying at. A player can browse `Hard`, race against
`[ai] difficulty`-scaled opponents regardless, and bank `"Gold (Hard)"` on
`Line7` - correct per the measured mechanism above (the rung recorded is
the screen's own browsed selection, not opponent strength), but worth
knowing before reading a banked `(Hard)` medal as "beat hard AI".

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
| `Required` | `grid->RequiredPoints`, or `FE_NA` when it is 0 | `"%03d"` |
| `honey` | `index*4+1`, `index*4+4`, `max*4+4` | `"%d-%d / %d"` |

**Corrected 2026-09-25**: this row previously read `"%d"`, unpadded - a
misreading of the decompile's generic `FUN_08972550(buf, fmt, value)` call
shape, which does not show the format string's own literal contents inline.
`read_memory` on the format string address actually referenced
(`&DAT_08a83568`, the third argument at `0x088dec24`'s own `Required`
branch) reads `"%03d\0"` byte for byte - confirmed against
`grid-selection-page1-grid0-unlocked.png`, whose `Required` row reads
`"012"` for `grid->RequiredPoints == 12`, not `"12"`. Confidence 95 (a raw
memory read of the literal, not a decompile inference).

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

### `CellSelection_OnEnter`'s own default-cursor scan, decompiled in full, 2026-09-28

Closes `docs/ui/campaign-screens.md`'s own "default cell" question
(`pulse-campaign` lane): on a fresh entry with no cell already selected,
`CellSelection_OnEnter` walks the screen's own collected cell definitions in
document order and selects the **first one whose `Locked` byte (`+0xb9`)
parses as the literal value `false`** - not the first cell overall, and not
`Locked`'s own "absent defaults to locked" reading the lock-glyph predicate
uses elsewhere on this page:

```
for cell in cells (document order):
    if cell->flags & 8 == 0 and cell->Locked (+0xb9) == 0:
        select cell
        break
```

An absent `Locked` attribute and an explicit `Locked="true"` are both
skipped alike - only an explicit `Locked="false"` stops the scan. Measured
live against PPSSPP (`pulse-psp-usa.chd`, Xvfb, a genuinely fresh profile):
`grid0`'s own default cursor is `grid0_3_1` (`Locked="false"`, fourth in
`grid_00.xml`'s own document order), never `grid0_2_1` (no `Locked`
attribute at all, first in document order) - matching this scan and
falsifying the "first cell listed" reading `oag_ui_screens::campaign::CellSelection::new`
carried until this pass. Confidence **80**, the same band the rest of this
function's own reads sit at - clean decompilation, corroborated by a live
capture, not itself breakpoint-verified. `cell->flags & 8` was not chased
this pass (no consumer of that bit found elsewhere in this file), and every
authored cell this pass checked reads `0` there, so it has not yet excluded
anything real; `oag_ui_screens::campaign::CellSelection::with_medals_and_records`
reproduces the scan and falls back to document order when no cell answers
`Locked="false"` at all (**chosen, not measured** for that fallback case -
the decompile itself would leave the selection at whatever it already was,
which a reimplementation cannot do for a screen that must show something).

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
    skill = t < 2.0 ? lerp(track.skill[0], track.skill[1], t - 1.0): lerp(track.skill[1], track.skill[2], t - 2.0)
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

## How a campaign event launches, and what a cell writes on the way

This closes the previous pass's blocker: "do not implement the cell-selection
wiring without first tracing the launch path." Traced 2026-09-14, entirely
from `CellSelection`'s own vtable slots and the functions reachable from
them; **nothing here is runtime-verified**, a PPSSPP breakpoint was not taken
(the Ghidra bridge was the only tool held this pass).

### The headline: a raw cell pointer, not a parallel set of globals - except it's actually both

`DAT_08b30ffc` is **the current campaign (or custom-grid) cell** - a raw
`PI_Cell` pointer, zero when no cell is in play. It is what
`Race_RecordResult`, `Eliminator_UpdateKillTarget`, `AI_ResolveSkillScale`
and `Hud_BindWidgets` already read directly (all four already on this page or
`hud.md`). This pass adds the **write** side and finds it does two things at
once, not one or the other:

1. Sets `DAT_08b30ffc` itself, so every one of those numeric consumers can
   dereference the cell's own fields (target triples, `laps`, `skill*`,
   `mode`) without a second lookup.
2. **Also** writes the exact same string-keyed global store
   [`race-setup.md`](../../../formats/race-setup.md)'s `Single Player` screen
   binds its `<List global="Mode">`/`Class`/`Weapons`/`Difficulty` rows to -
   `Mode`, `Class`, `Track`, `Team`, `Weapons`, `Opponents`, `Laps`, `Damage`,
   `SkillLevel`, and (Tournament only) `Tournament`. So a campaign launch and
   a custom-race-box launch converge on the same named store before
   `Launch Game` runs, rather than the campaign carrying a value the race box
   never sets or vice versa.

### `CellSelection_CommitSelection` (`0x088d6138`) - the write site, and a real vtable slot

Confidence **80**. Corrected from an initial 68/`_q`: the first pass through
this function found it via `get_xrefs_to(DAT_08b30ffc)` and could not find
*its own* caller by cross-reference, and wrongly reasoned that through to "no
evidence of when it runs." **It is not a free function waiting on a caller -
it is word 39 of the `CellSelection` vtable itself**, positionally confirmed
the same way word 9/29/31 already were on this page: `read_memory` on
`0x08acfb64` for 256 bytes (not the 128 `race-box-screens.md` read for
`TrackSelection`/`TeamSelection`) decodes word 39 (byte offset `0x9c`) as
`0x088d6138` exactly. `get_xrefs_to`/`get_function_callers` return nothing on
*any* of these four slots (9, 29, 31, 39) for the same reason - a vtable
store is a data write, not a call site, and this binary's dispatch is by
vtable throughout (see `race-box-screens.md`'s own vtable section). That is
the same evidentiary class `CellSelection_OnEnter`/`_OnExit` already carry at
74-82, so this slot is scored the same way rather than capped for a gap that
turns out not to exist.

**Independent corroboration that word 39 is a real, consistently-purposed
slot**: `TrackSelection`'s vtable (`0x08ad0ce4`) and `TeamSelection`'s
(`0x08ad0b04`) both carry a function at the identical word 39, which
`race-box-screens.md` never read (it stopped at 128 bytes / word 31).
`TrackSelection`'s (`0x088ed784`, named `TrackSelection_CommitSelection`
this pass, confidence 80) is one line: `Globals_Set("Track",
current_track_definition->name)` - the same commit-current-selection-to-the-
global-store duty as `CellSelection`'s slot, just for one field instead of
nine. `TeamSelection`'s (`0x088e9fa0`, named `TeamSelection_CommitSelection`
this pass, confidence 74 - gated behind three `strcasecmp`s against state
names `"Pre_Race_Music_Select"`/`"Team_Help"` this pass did not chase)
does music-selection/help-text bookkeeping rather than a `Globals_Set` call,
so the slot's *exact* contract varies by screen, but its role - fired
whenever that screen's current selection should be committed, distinct from
`OnEnter`/`OnExit`/`Update` - is now evidenced on three screens, not
inferred from one.

In full, `CellSelection_CommitSelection`:

```
FUN_088902c8()                                         # base-class hook, unread
if param_2 != 0:                                        # a cell is selected
    cell = *(param_1 + 0xdc)
    Globals_Set("Mode",   localize(cell->mode))          # Globals_HashKey hashes "FE_Mode"
    Globals_Set("Class",  localize(cell->class))
    Globals_Set("Track",  cell->track)                    # +0xba, raw string, no localize
    Globals_Set("Team",   cell->ship)                     # +0xca - "None" on all 236 authored cells
    Globals_Set("Weapons", FE_ON/FE_OFF)                  # cell->Weapons, +0xb4
    Globals_Set("Opponents", "%d" of cell->AICount)       # +0xb0
    Globals_Set("Laps", "%d" of cell->laps)               # +0xac
    Globals_Set("Damage", FE_ON/FE_OFF)                   # cell->damage, +0xb5
    DAT_08b30ffc = cell
    strcpy(DAT_08b31000, cell->name)                      # +0x74, display copy only
    Globals_Set("SkillLevel", Easy/Medium/Hard[param_1->f0])  # the screen's own difficulty cursor
    if cell->mode == 4 (Tournament):
        Globals_Set("Tournament", "yourTourney")
        hash = Libc_HashString(DAT_08b31158 + 0x74)        # tournament record key
        DAT_08b31158->0xa0 = 0; DAT_08b31158->0xdc = 1; DAT_08b30fa4 = 0   # reset standings/leg counter
        for each of cell->TournamentTrackCount (+0xf0) TournamentTrack name hashes (+0xf4):
            FUN_088c3990(DAT_08b31158, hash)               # append a leg
Store("DifficultyRC") = param_1->f0    # unconditional - runs even when param_2 == 0, see below
```

`Store("DifficultyRC")` is **not** gated by `param_2` - the `if` above only
guards the cell-specific writes; the difficulty persist sits at the
function's single shared exit and runs on every call, cell selected or not.
`cell->ship` (`+0xca`) written under the unnamed `"Team"` key is the literal
string `"None"` on all 236 authored cells (`race-campaign.md`'s own
"every cell carries `ship="None"`" finding), so the campaign writes `"None"`
into `Team` rather than ever forcing a craft - consistent with
`ShipChoice="Yes"`, not a new fact but a direct corroboration of it from a
second call site.

`Globals_Set` (`0x08888ee0`, confidence **85**) is the front-end global
setter: it hashes `"FE_" + key` (`Globals_HashKey`, `0x08888db4`, confidence
**78**, itself `Libc_HashString` of the formatted `"FE_%s"` string) and
inserts into a single flat hash table at `&DAT_08b317b8`, replacing any
existing entry for that key first. **This is not a guess at the mechanism -
it is the same function `TrackSelection_OnExit` (already named,
[`race-box-screens.md`](race-box-screens.md)) calls to write the `Track`
global** when a custom race is set up through `Track Creation`, and the same
one `TrackSelection_CommitSelection` above calls too. Confirmed by
`get_function_callers(0x08888ee0)`, which lists `TrackSelection_OnExit`
(`0x088ed6cc`) alongside `CellSelection_CommitSelection` and three others. So
**the campaign writes into the identical store the custom race box's screens
write into**, under the identical key names `race-setup.md` already
catalogued (`Mode`, `Class`, `Weapons`, `SkillLevel` - it calls the fourth
`Difficulty` row's persisted key `SkillLevel` too, exactly matching
`race-setup.md`'s "the widget name and the persisted key differ" finding),
plus keys the race box's own list widgets never expose (`Team`, `Opponents`,
`Laps`, `Damage`, `Tournament`) that a read-only detail panel or `Launch Game`
still needs.

**One caveat this pass did not close**: the values passed to `Globals_Set`
for `Mode`/`Class` are **localised display strings** (`FUN_088055d8`/
`FUN_08805458`, which take a string-table context `&DAT_08b30f90` first), not
raw enum ints. Whether `Launch Game` reads this same key back as text and
re-parses it, or reads `DAT_08b30ffc` directly for anything numeric and only
uses the string globals for display, was not traced - see the breakpoint
below.

### The record store keys a cell by a hash of its own authored `name` - not a grid/cell index pair

Answers "how is a race mapped back to its cell": **`Libc_HashString`**
(`0x08945890`, confidence **85** - `strlen` then `FUN_08945a08`, decompiled
this pass and confirmed as a textbook table-driven CRC32: init `0`, `(table[
(crc ^ byte) & 0xff]) ^ (crc >> 8)` per byte against a 256-entry table at
`DAT_08b04480`, final `XOR 0xffffffff`. That table is a different address
from the already-named `g_wad_crc_table` (`0x08afbffc`, `Wad_HashName`,
`wad-subsystem.md`) - whether the two tables hold the same 256 values was not
checked) on the cell's own literal name string is what `PI_Cell_ParseElement`
already calls (`race-campaign.md`'s `FUN_0880871c`, not renamed - the
existing prototype-ambiguity caveat on it and on `FUN_08808624` stands and
neither was touched this pass) to create a save record, keyed by
`Libc_HashString(cell->name)` where `cell->name` is the literal authored
string (`"grid0_2_1"`, `"UserGrid_2_1"` for a custom cell). `FUN_0880871c`
also `strcpy`s the raw name into the record itself (offset `+4` of the node),
so the record carries both the hash (for lookup) and the name (for display) -
there is no separate grid-index/cell-index pair anywhere in the key. The
same mechanism keys **two more things this pass found, neither of them a
cell**: the tournament standings record (`Libc_HashString(DAT_08b31158 +
0x74)`, already flagged as unidentified by the previous pass - `DAT_08b31158
+ 0x74` is itself a string, not resolved this pass) and a **profile-wide,
not-per-cell** difficulty record keyed by the fixed literal string
`"DifficultyRC"` (`0x088098f0`): the `SkillLevel` cursor `CellSelection`'s
`Update` cycles with button 7 persists here on confirm, read back as the
default the next time any cell is entered. So the "record store" is a
generic name-hashed key/value store the campaign, the tournament and a
plain settings value all share - not a structure specific to `PI_Cell`.

### `Locked` does drive the `Lock_x_y` overlay - closing that open item, at least for the `PI_Cell` byte

`CellSelection_PopulateGrid` (`0x088d5de4`, confidence **85**, raised from 78 -
the six-neighbour loop below is now fully decompiled, though still not
runtime-verified) fills the hex overlay per cell and settles what the
previous pass and `race-box-screens.md` both left open at 50:

```
medal = Cell_BestMedal(cell)                  # already-named accessor
draw_base_overlay(x, y)
if cell->Locked (+0xb9) != 0 and medal == 0xff:
    draw_lock_overlay(x, y)                   # the Lock_x_y glyph, fully opaque
if medal == 0xff:
    ... # a six-neighbour adjacency loop - see "Unlock rules, cell and tier" below,
        # now fully traced: it is cosmetic, not a second unlock mechanism
else:
    draw_medal_colour(x, y, gold/silver/bronze[medal])   # Medal_x_y
```

So **`Locked="true"` does gate the lock glyph, directly, and only while no
medal has been earned on that cell** - the earlier "redundant against
`<Unlock>`" hypothesis is not what the code does; a `Locked` cell that has
never been medalled shows locked regardless of what else is true.
**Correction to the previous pass's own hedge**: the six-neighbour loop this
same function runs when `medal == 0xff` is traced in full below
("Unlock rules, cell and tier") and is **not** a second, adjacency-based
gameplay-unlock path - it only decides whether the lock glyph is shown, tinted
or fading, and never touches `cell->Locked`, `PI_Grid.Locked` or any
`Unlock_*` predicate. **`Status` (`+0xb8`) and `PI_Grid`'s own `Locked`
(`+0xa0`) are addressed below too** - `Status` remains untouched by anything
this pass read, but `PI_Grid.Locked` is now settled the same way `PI_Cell`'s
byte is.

### `CellSelection_OnExit` (`0x088d5cb8`) - the given vtable slot, read in full

Confidence **82**. Word 31 of the `CellSelection` vtable (`0x08acfb64`,
confirmed by `read_memory` on the vtable itself: word 9 is `0x088d6430`
(`CellSelection_Update`, already named) and word 29 is `0x088d59c4`
(`CellSelection_OnEnter`, already named), both matching this page's existing
table, and word 31 is `0x088d5cb8`). It restores the *wrapping* screen's
title to `FE_RACE_CAM` ("RACE CAMPAIGN") - matching
`CellMode_Definition.xml`'s outermost `<Screen>` block, which authors exactly
that `ScreenTitle`/`FE_RACE_CAM` pair as the last thing before its closing
tag - copies one field (`+0x54`) from the current grid (`DAT_08b30fb8`) into
an info-panel widget, and, when its own `param_2` is non-zero, re-runs the
lock-overlay refresh loop over every cell definition. It does **not** write
`DAT_08b30ffc` or any `Globals_Set` key itself - the write happens in
`CellSelection_CommitSelection` (word 39, not word 31 - see below), a
separate vtable slot from this one.

### Two named, buttonless redirects - a mechanism this project has not documented before

`CellMode_Definition.xml` (`just wad cat ... 'Data\Plugins\PI001\GUI\CellMode_Definition.xml'`)
carries, at the same nesting level as the `CellSelection` screen block itself
rather than inside it:

```xml
<Redirect c="Cell Mode Redirect Team">
  <Default goto="Team Selection"></Default>
</Redirect>
<Redirect c="Cell Mode Redirect Game">
  <Default goto="Launch Game"></Default>
</Redirect>
```

Neither carries an `<a forward="..."/>` button condition - every other
`<Redirect>` this project has read so far (`race-setup.md`'s "Redirect out"
sections, this file's own `Definition_ParseUnlock` table) is either a button
mapping or an `<Entry item=... equals=... goto=...>` value test. These two
are named (`c="..."`) with no button or value trigger at all - a third
redirect shape. `search_strings` for both names against the executable
returns nothing, so native code is **not** looking either name up as a
string at fire time; the two blocks are more likely selected by their
position in the parsed redirect list, or through a pointer to the
already-parsed XML node captured once at screen construction, than by name.
Which, and from where, was not located this pass - `get_xrefs_to` on both
redirect blocks' addresses and on `CellSelection_CommitSelection` returns
nothing either (consistent with the vtable-dispatch pattern above: a value
stored in a data structure, not a call site `get_xrefs_to` tracks).
**Whether `Cell Mode Redirect Game` (straight to `Launch Game`, skipping
`Team Selection`) is ever actually taken for a campaign cell, or exists only
for a same-cell retry after a finished race, is open** - every campaign cell
carries `ship="None" ShipChoice="Yes"`, so there is no obvious reason a first
launch would skip craft selection, but nothing rules it out either.

### What the next pass should verify, in PPSSPP

None of the above is runtime-verified. In order of how much each would move:

1. **Breakpoint at `0x088d6138` (`CellSelection_CommitSelection`) entry**,
   press confirm on a grid cell in `Cell Selection`. Confirms the vtable-slot
   reading above by showing it actually fires on a real confirm, and a
   step-out identifies the dispatcher `get_xrefs_to` could not find (almost
   certainly a generic `Screen_CommitSelection`-style virtual call, matching
   how `Update`/`OnEnter`/`OnExit` are already known to be dispatched).
2. **Watch `0x08b30ffc`** (`DAT_08b30ffc`) for writes across a full
   `Cell Selection -> Team Selection -> Launch Game` sequence, and check
   whether it is ever written `0` again before `Launch Game`'s own `OnEnter`
   runs (would mean `Launch Game` cannot rely on it staying set).
3. **Read the `Globals_Set`/`&DAT_08b317b8` table's `"FE_Track"` entry**
   right before `Launch Game` starts loading, for both a campaign launch and
   a custom Race Box launch, to settle the open caveat above: does
   `Launch Game` actually read the string global, or only `DAT_08b30ffc`.
4. **Breakpoint at `0x088d5de4`** (`CellSelection_PopulateGrid`) with a save
   carrying a `Locked="true"` cell with no medal, then again after medalling
   it, to confirm the lock-glyph branch flips as read above. **Answered this
   pass without a breakpoint**: the six-neighbour loop
   (`Grid_FindCellAtCoordinate`/`g_anCellNeighbourOffsets`, decompiled in
   full) is not a second unlock path - see
   ["Unlock rules, cell and tier"](#unlock-rules-cell-and-tier) above.
5. **Confirm a Tournament-mode cell** and watch `DAT_08b31158` and the
   `FUN_088c3990` calls to verify the per-leg standings reset this pass
   read matches what the previous pass's "Tournament" open item described in
   `Race_RecordResult`.

### Runtime-verified 2026-09-14

PPSSPP v1.20.4 (SDL build), `pulse-psp-usa.chd`, Xvfb `:97`, debugger on
`ws://127.0.0.1:47810/debugger`. Breakpoints 1-4 taken in priority order; 5
was blocked (see below). Full transcript in
`/tmp/oag-drive/campaign-ppsspp-measure.md`.

**1. `CellSelection_CommitSelection` (`0x088d6138`) fires on a real confirm,
and the dispatcher is `0x08891360`.** Armed the breakpoint, pressed `cross`
on a selected cell in `Cell Selection`: the CPU stopped at `pc=0x088d6138`
with `a0=0x08d73170` (the `CellSelection` instance - matches the `this` seen
at every other breakpoint this pass), `a1=0x08d731c4` (`this+0x54`, not a
bare `0`/`1` - the "`param_2 != 0`" framing above is a boolean *test* the
decompiler rendered from a pointer compare, not a literal 0/1 argument;
worth a caveat, not a correction, since the branch direction is unaffected),
and `ra=0x08891360` - the exact call site `get_xrefs_to` could not find by
cross-reference (a vtable dispatch, not a direct call). `0x08891360` was not
decompiled or named this pass (no Ghidra bridge held), but it is now a
concrete address for whoever holds the bridge next to confirm the
"generic `Screen_CommitSelection`-style virtual call" guess directly rather
than inferring it. Resuming past the hit let the confirm complete normally
(-> `Team Selection`), so the breakpoint did not observably change behaviour.
**Confidence raised 80 -> 92**: runtime trace, single binary (rubric caps a
runtime trace at 94 until a second binary corroborates).

**Tooling trap found, not previously documented**: sending
`input.buttons.press` for the very button whose resulting code path hits an
already-armed execution breakpoint leaves that `input.buttons.press` call
itself unacknowledged (client-side `TimeoutError`) even though PPSSPP
received and processed the input and the breakpoint did fire correctly
afterward - confirmed by catching the timeout and calling `wait_for_break`
anyway, which returned the hit immediately. Treat a `press()` timeout as
inconclusive, not as "nothing happened," whenever a breakpoint is armed on
the code the press is expected to reach.

**2. `DAT_08b30ffc` is never cleared before `Launch Game` reads it.** Read
directly (`0x08f61490`) right after the `CellSelection_CommitSelection` hit
above, armed a write-watch (`enabled: false, log: true`) on it, then drove
`cross` (confirm craft) -> `Launch Game Transition` ->
`InGameTrackDescriptionScreen` -> `cross` -> `InGame`. **0 hits**, and a
direct re-read at each of those four states returned the identical
`0x08f61490` throughout, including once the race was actually running. Not
just a zero-hit count (which the confidence rubric's own "always arm a
positive control" caveat would want corroborated) - the *value itself* was
read and compared directly at four points, which is stronger than the watch
alone. **Confidence 90** that `Launch Game` (and everything after it) can
rely on the pointer staying set once a campaign cell is confirmed.

**3. `Globals_Set` writes the documented nine keys, in the documented order,
with resolved values matching the confirmed cell's own authored XML digit
for digit.** Breakpointed `Globals_Set` (`0x08888ee0`) itself and caught
every hit during one `CellSelection_CommitSelection` call: `Mode`, `Class`,
`Track`, `Team`, `Weapons`, `Opponents`, `Laps`, `Damage`, `SkillLevel`, in
exactly that order, no `Tournament` key (the confirmed cell was `Time
Trial`, not `Tournament` - consistent). **One correction to the earlier
reading**: `a1` at the breakpoint is not the value string directly - it is
the address of a small per-call scratch slot (`0x08aeeaf0`, `+4` per key)
that itself holds the value pointer, i.e. one more level of indirection
than `a0` (the key, which *is* the string directly). Dereferencing once
gave clean text for every key: `Mode="Time Trial"`, `Class="Venom"`,
`Track="16_Track"`, `Team="None"`, `Weapons="Off"`, `Opponents="0"`,
`Laps="3"`, `Damage="Off"`, `SkillLevel="Hard"` - matching `grid0_3_2`'s own
authored record (`track="16_Track" mode="Time Trial" ... damage="off"`, no
`AICount` attribute hence `0` opponents) to the letter, and `SkillLevel`
matching the difficulty (`Hard`) set on the pad immediately beforehand in
this same session. `Track` reads raw (`"16_Track"`, unlocalized) exactly as
documented; `Mode`/`Class` did not exercise the "`Race` -> `"Single Race"`"
transformation this session (the confirmed cell was `Time Trial`, whose
enum name and display name coincide) - see the UI-side finding in
`docs/ui/campaign-screens.md`'s "Measured against PPSSPP" section, which
independently shows a `Race` cell's on-screen `Title` reading `"SINGLE
RACE"`, a different render path (`CellSelection_PopulateDetail`) from this
`Globals_Set` call, not yet cross-checked against each other. **Confidence
raised 85 -> 92** for `Globals_Set` (runtime trace, values corroborate
authored data exactly) and **80 -> 93** for `CellSelection_CommitSelection`'s
body specifically (the write side, as opposed to the vtable-slot claim
scored separately above).

**The read side of item 3 (does `Launch Game` itself read `&DAT_08b317b8`)
was attempted and is inconclusive, not negative.** A broad read-watch
(`0x08b317b8`, `0x1000` bytes, `log: true`) produced **over five million
hits in under five seconds** - consistent with `log: true`'s own documented
disk-filling failure mode, not with a clean signal - so no PC or address was
distinguishable from it, and it was removed immediately rather than let run.
The hash table's own bucket count, stride and key/value layout remain
undocumented (a separate RE pass, per the assignment's own scope note), so a
narrower, targeted watch was not attempted this pass. Left open.

**4. `CellSelection_PopulateGrid` (`0x088d5de4`) fires once per screen
entry, not once per cell.** Breakpointed it and re-entered `Cell Selection`
from `Grid Selection`: exactly **one** hit, `a0=0x08d73170` (the same
`CellSelection` instance), `a1=1`, `a2=1` - small integers, not a cell
pointer, so the per-cell `medal`/`Locked` branch this page's pseudocode
describes runs in an **internal loop inside this one call**, not once per
invocation as the earlier reading's phrasing could be misread to imply. The
lock-glyph branch itself was not step-verified at the instruction level (no
Ghidra bridge held this pass to find an inner address to break on), but the
live capture in `docs/ui/campaign-screens.md` corroborates it structurally:
`grid0`'s own 8 cells show a lock glyph on exactly the 6 that carry **no**
`Locked` attribute at all in `grid_00.xml`, and no glyph on the 2 that
explicitly author `Locked="false"` - which only makes sense if **an absent
`Locked` attribute defaults to `true`** in `PI_Cell_ParseElement`, a default
this page never stated either way. **Confidence for
`CellSelection_PopulateGrid` raised 78 -> 82** (function entry and its
two-int-argument shape confirmed live; the per-cell branch is corroborated
by the capture, not itself breakpointed). The "absent `Locked` defaults to
`true`" reading is new this pass, confidence **72** - one grid's worth of
cells, consistent but not a census, and not itself breakpointed either.

**5. Blocked, not attempted: no reachable Tournament cell in this profile.**
Every grid but `grid0` reads `Locked="true"` on this session's save (no
medals earned, `grid0_...`'s own points never banked), and pressing
`Confirm` on a locked `Grid Selection` tile does nothing (see
`docs/ui/campaign-screens.md`'s "Grid-tier locking is not cosmetic" finding)
- so `grid1`'s own `Tournament`-mode cells, and every other grid's, were
unreachable without either earning points first or writing profile memory
by hand, both out of scope for this pass. `DAT_08b31158` was not watched.
Left for a pass that either plays a race to earn the 12 points `grid0`
needs, or accepts the risk of a direct memory write to force an unlock.

## Unlock rules, cell and tier

Traced 2026-09-14, entirely from decompilation with the Ghidra bridge - no
PPSSPP breakpoint was taken this pass (the bridge was the only tool held).
This closes the previous pass's "may be a second unlock mechanism" hedge on
the six-neighbour loop, and reads the `Grid Selection` tier's own lock glyph
for the first time.

**Read this section as two separate questions, answered to very different
degrees.** "When is the lock glyph drawn" is settled below at high
confidence for both the cell and the tier, and it is a real, three-term,
player-visible rule on each screen - not a cosmetic detail. "When is a cell
actually playable, and when is a tier actually enterable" - i.e. whether
`Confirm` itself is refused - is **not** settled: no PI001 function this
pass traced ever refuses a transition on either `Locked` byte, yet PPSSPP
measured live that confirming a locked tier does nothing. The glyph rule is
not evidence either way for the confirm question; see the dedicated
paragraphs below and the three breakpoints at the end of this section.

### The cell unlock rule: `Locked` is the static default, and a medal (the cell's own or a hex neighbour's) clears it

`Grid_FindCellAtCoordinate` (`0x088c072c`, confidence **85**) is a plain
linear search: given a grid's cell list and an `(x, y)` pair, it walks the
list (the same iterator, `FUN_088bff48`, every other cell-list walk in this
file uses) comparing each cell's `+0x124`/`+0x128` - the coordinates
`PI_Cell_ParseElement` already parses out of the cell's own name - and
returns the first match or `0`. It has exactly one caller,
`CellSelection_PopulateGrid`, confirmed by `get_function_callers`.

`g_anCellNeighbourOffsets` (data, `0x08ab1e68`, confidence **85**) is not one
six-entry table but **two**, back to back, 48 bytes (six `(dx, dy)` `int32`
pairs) each:

| Direction | Even `x` (bytes `0x00-0x2f`) | Odd `x` (bytes `0x30-0x5f`) |
| --- | --- | --- |
| left | `(-1, 0)` | `(-1, 0)` |
| up | `(0, -1)` | `(0, -1)` |
| right | `(1, 0)` | `(1, 0)` |
| down | `(0, 1)` | `(0, 1)` |
| diagonal 1 | `(-1, -1)` | `(-1, 1)` |
| diagonal 2 | `(1, -1)` | `(1, 1)` |

The four orthogonal offsets are identical for both parities; only the two
diagonals flip between "up-left/up-right" (even columns) and
"down-left/down-right" (odd columns) - exactly the offset-column convention a
staggered hex grid needs, and read directly off the raw bytes (`read_memory`
on `0x08ab1e68`, 96 bytes), not inferred.

`CellSelection_PopulateGrid`'s neighbour loop, decompiled in full
(disassembly at `0x088d5fb0`-`0x088d60c0`, confidence **85**):

```
if medal == 0xff:                                            # this cell has no medal
    table = (x & 1) ? g_anCellNeighbourOffsets[6..11] : g_anCellNeighbourOffsets[0..5]
    found = false
    for (dx, dy) in table:                                    # six tries
        neighbour = Grid_FindCellAtCoordinate(current_grid, x + dx, y + dy)
        if neighbour != 0 and Cell_BestMedal(neighbour) != 0xff:   # a medalled neighbour exists
            GridController_ClearTileFlags(widget, LOCK_LAYER, x, y, VISIBLE)   # hide the lock glyph
            if GridController_IsLockFading_q(widget, x, y):        # was already fading out
                GridController_SetLockTint(widget, 0xff, x, y)     # reset its tint to opaque white
                found = true
            break
    if GridController_IsLockFading_q(widget, x, y) and not found:
        GridController_ClearLockFade_q(widget, x, y)                # stop the fade, no neighbour found
```

`GridController_SetTileFlags`/`ClearTileFlags` (`0x088a3638`/`0x088a374c`,
confidence **82** each), `GridController_SetTileColor`/`SetLockTint`
(`0x088a3850`/`0x088a3ad0`, confidence **85**/**78**) and
`GridController_IsLockFading_q`/`ClearLockFade_q`
(`0x088a39d4`/`0x088a39ac`, confidence **65** each, `_q` - the *reader*
behaviour is measured directly in `CellSelection_Update` below, but the bit's
name is inferred from that one consumer, not itself documented) are a shared
widget-layer accessor family: each takes a `(widget, layer, x, y)` tile
address, where `layer` selects one of three overlapping sub-widgets per
`(x, y)` slot (`1`/`2`/`4` at byte offsets `+0xc0`/`+0x1c0`/`+0x2c0` off the
tile's base) - `1` the base hex, `2` the medal-colour swatch, `4` the lock
glyph. They are used identically by `GridSelection_PopulateTiles` below, so
they belong to the shared `GridController` widget class both screens'
hex/row layouts are built from, not to `CellSelection` alone.

**`GridController_IsLockFading_q`'s bit is a fade-out timer flag, read
(not written) by `CellSelection_Update`'s own per-cell loop**
(`FUN_088d6430`, already on this page; full body re-read this pass): every
frame, for every cell whose bit is set, it computes
`elapsed = currentTime - screen->+0xc8` - **one shared timestamp on the
screen object itself, not a per-cell start time**, so every currently-fading
lock glyph on the whole grid fades on one synchronised schedule, not
independently from when each one's neighbour was medalled - and either
interpolates the lock glyph's alpha down towards zero
(`GridController_SetLockTint`) while `elapsed` is still under a threshold,
or - once the threshold passes - hides the glyph outright
(`GridController_ClearTileFlags`) and clears the bit
(`GridController_ClearLockFade_q`). The writer of `screen+0xc8` was not
located this pass. `GridController_ResetLockFades_q` (`0x088a3a40`,
confidence **68**, `_q`) clears the bit for every `(x, y)` in the grid's own
bounding box, and is what `CellSelection_PopulateGrid` calls on first entry
(or when the current grid changed) to discard stale animation state.

**Checked and ruled out as the fade-bit's setter**:
`GridController_SetTileFlags` (`0x088a3638`) ends with an unconditional call
to `FUN_088a38d0(widget, x, y)`, which looked like a promising candidate for
turning the `+0x3c4` fade bit on - but it decompiles to
`*(*(widget+0xb8) + y*4) |= 1 << x`, a **different** row-bitmask pointer
field (`+0xb8`, not `+0x3c4`) on the same widget, structurally identical but
distinct. `GridController_ClearTileFlags` has no matching call at all, so
whatever `+0xb8` tracks is set on every layer-flag write and never cleared
by the tile-flag accessors themselves - read as "this tile has been
drawn/touched," not lock-fade state. **The actual setter of the `+0x3c4`
fade bit was not found this pass.**

**The conclusion this settles, restated as the actual display-level
predicate rather than "cosmetic vs not"**: the cell's lock glyph is visible
exactly when

```
cell->Locked (+0xb9) != 0
  and Cell_BestMedal(cell) == 0xff             # the cell has no medal of its own
  and no hex-adjacent cell has a medal either  # the six-neighbour check
```

`Locked` is the **static default** an unmedalled cell starts from; a medal -
the cell's own, or (with a brief fade) a hex neighbour's - is what actually
*clears* the glyph. Calling the neighbour loop "cosmetic" undersells it: it
is real, player-visible unlock-*display* logic, decompiled in full and
correct at confidence **85**. What is **not** settled, and is a genuinely
separate question from the display rule above, is whether this predicate (or
any part of it) also gates whether the cell can be *played* - see below.

### The runtime tint layer: literal colours behind `GridController_SetTileColor`'s own callers

Traced 2026-09-25, closing `docs/ui/campaign-screens.md`'s "no new gap"
finding - it had only re-read the screen's *text*, not its colour. All three
findings below read a literal ARGB directly out of a decompiled call site
(`inspect_memory_content`/`decompile_function`), not out of the XML - `Grid
Selection`'s hex/lock/arrow widgets and `Cell Selection`'s `Selector` all
author only a multiply-neutral `i="0xffffffff"` default (or nothing at all),
so every colour below is applied by native code the disc's own XML never
states.

**`GridSelection_PopulateTiles` (`0x088de630`, already named) tints every
tier's own base hex unconditionally**, locked or not, selected or not:

```
GridController_SetTileColor(widget, /*layer=*/1, tile, 0, alpha<<24 | 0x34acc2)
  where alpha = (int)(param_2 * 0.5 * 255.0)   # 127 at the settled param_2 == 1.0
```

The lock glyph (layer 4) gets a *separate* call, white RGB at roughly double
that alpha (`param_2 * 255.0`, ~255 settled) - so a locked tier's own hex
outline and its padlock are never the same brightness, by design: the
outline is deliberately dimmer than what will draw over it. Layer 2 (the
medal swatch) reuses the same three literals `crate::campaign::draw::medal_argb`
already carries (`0xfffaeb38`/`0xffdae3e4`/`0xffdf942f`), picked by a
points-vs-threshold comparison this pass read but did not carry into this
project's own `medal_tint` (still `medal_argb(Gold)` always - see that
function's own "chosen, not measured" doc, now itself only half right: which
colour is still chosen, but *whether* it varies by tier is now known and
this build does not yet implement it). Confidence **90** (decompile literal,
consistent with `grid-selection-page1-grid0-unlocked.png`'s own dim
cyan-teal outline and bright padlock).

**`CellSelection_PopulateGrid` (`0x088d5de4`, already named), read again in
full this pass, never calls `GridController_SetTileColor` on layer 1 at
all** - only `SetTileFlags` (visibility) and, conditionally,
`SetLockTint`/layer-2 medal colour. So `Cell Selection`'s own hex outlines
are **not** tinted by this function, unlike `Grid Selection`'s. This is a
real asymmetry, not a gap in this pass's reading: `Cell Selection`'s
`Outline_x_y` widgets author `i="FEGlobals->CM_HEX_Outline"` in
`CellMode_Definition.xml` instead (confirmed, `just wad cat` on
`Data\Plugins\PI001\GUI\CellMode_Definition.xml`) - a `FEGlobals->` live
binding (`docs/formats/fexml.md`) this project has not resolved (`CM_HEX_Outline`
is not one of the two confirmed names, `FE_TeamModel`/`FE_ModelSkin`).
`cell-selection-grid0-default-cell.png` shows the same dim cyan-teal outline
`Grid Selection` gets from its own literal, which is suggestive - the same
`0x34acc2` is also what `Medals Title`/`Points Title`/`Required Title` author
directly (`i="0xff34ACC2"`, full alpha) on the very same file - but this pass
did not read the `FEGlobals` registry itself, so `Cell Selection`'s own
outline tint is named as an open question rather than implemented on the
strength of that inference. Confidence **80** for "the asymmetry is real",
no confidence assigned to `CM_HEX_Outline`'s own value.

**Superseded, `pulse-campaign` lane, 2026-09-28: `CM_HEX_Outline`'s own
value was already read and wired the same day this pass's own `Outline_x_y`
section above was written (2026-09-14, `crate::campaign::load`'s
`fallback_globals`), and the "not one of the two confirmed names" sentence
above simply never re-checked the code.** `just wad cat` on
`Data\Plugins\PI001\GUI\Skin.xml` reads a `<Variable global="CM_HEX_Outline">`
declaring `0x7F34ACC2` directly (a semi-transparent teal, the same
`0x34acc2` RGB every inference above already suspected, now a literal
`<Values color=>` read rather than a guess); `search_strings` for
`"CM_HEX_Outline"` against this binary returns **zero** hits, so no native
code path ever names this key at runtime - the `FEGlobals->` resolver hashes
whatever name the XML gives it, and `Skin.xml`'s own declared value is
final, with no override to chase further. `crate::campaign::load` already
threads the front-end root's own parsed globals through as
`fallback_globals` and `oag_ui_screens::campaign::draw::cell_draw_list`'s
`Outline_x_y` arm already falls to the generic `image_draw` path, which
applies `image.color` (the widget's own resolved attribute) unmodified - so
this was fixed by the earlier pass's own `fallback_globals` plumbing, not
left open. Confirmed against a fresh live capture, not just static code
reading: a locked hex's outline stroke on `pulse-psp-usa.chd` (PPSSPP,
`Cell Selection`, fresh profile) averages RGB `(31, 58, 63)` over ~2,000
sampled pixels; this build's own `--menu-page cell-select` still of the same
hex averages `(26, 56, 62)` - within a few units, well inside anti-aliasing/
background-gradient noise. Confidence **90**. `docs/ui/campaign-screens.md`'s
own "Open" bullet on this and the handover thread's matching bullet are
stale and are corrected in the same change that adds this section.

**`GridController_UpdateSelectorPulse` (`0x088a5700`, renamed this pass,
confidence 85)** is the actual source of the selected tile's own glow -
reached generically (not a PI001 screen method) whenever a `GridController`
widget's own `+0x2c` flags word has bit `0x200` set, the same
"cursor-highlight" bit `GridSelection_Update`'s own page-flip logic already
toggles. It looks up two child widgets by name on the currently-highlighted
tile - `"Selector"` and `"SelectorGlow"` (the two string xrefs that led here,
`0x08a7f130`/`0x08a7f178`) - and drives each one's own colour off a shared
phase `tile+0x9c`, advanced `elapsed*2.0` per frame and wrapped at `2.0`
(`FUN_088cd2b8` is a plain ARGB `lerp(t, from, to)`):

```
phase = (tile.phase + elapsed*2.0) mod 2.0
Selector:     phase > 1.0 ? lerp(phase-1.0, white, 0xff33a6b9) : lerp(phase, 0xff33a6b9, white)
SelectorGlow: phase > 1.0 ? lerp(phase-1.0, white, transparent) : lerp(phase, transparent, white)
```

So `Selector` is a continuous ~1s triangle wave between white and
`0xff33a6b9` (cyan), and `SelectorGlow` is a same-phase alpha pulse of a
second, separate widget - **not authored anywhere in `CellMode_Definition.xml`**
(only `Selector` itself is; `SelectorGlow` is created procedurally, the same
way the generic `GridController` widget class itself is never authored as
XML content either). `crate::campaign::draw` implements the colour half of
this (`SELECTOR_TINT = 0xff33a6b9`, a static draw of one endpoint rather than
the animation - the draw-list builder has no clock) and does not implement
`SelectorGlow` at all, since this pass did not locate its own geometry/texture
- drawing a halo shape this build never measured would be exactly the
"plausible-looking stand-in" this project's CLAUDE.md warns against.
`docs/ui/campaign-screens.md`'s own comparison names the gap.

**`pulse-campaign` lane, 2026-09-28: an 8-frame burst shows the halo is
real and phase-correlated, but its own xref count still gives no creation
site to point at.** `get_xrefs_to` on the `"SelectorGlow"` string
(`0x08a7f178`) returns exactly **one** hit in the whole binary: the lookup
inside this function. No separate widget-creation or allocation site
references the string anywhere - evidence that nothing in the executable
*names* a widget `"SelectorGlow"` at any traceable site, though not proof
that no widget is ever built with that name at runtime (a generic
`GridController`-driven clone, the way the controller class itself is never
authored as XML content either, would not need to re-reference this
string). (For comparison, `"Selector"` at `0x08a7f130` has two xrefs - this
function's own lookup plus a second site, `FUN_088a4d24` - consistent with
it being a real, authored, twice-referenced widget the way
`docs/ui/campaign-screens.md`'s "authored twice" note already describes.)

**The halo is not a one-frame illusion.** Eight frames of `Grid Selection`'s
own selected tile, `pulse-psp-usa.chd`, ~180ms apart (`Xvfb :93`,
`f1..f8.png`): the halo's own
size and brightness track `Selector`'s documented colour phase exactly -
small and dim when `Selector` reads cyan (`f1`, `f4`, `f5`, `f8`), large and
bright when `Selector` reads white (`f2`, `f3`, `f6`, `f7`), the same
period the decompile above gives both widgets. A colour tint on `Selector`'s
own sprite cannot explain this by itself: `docs/ui/campaign-screens.md`'s
own texture read of the `Selector` crop (42x43, content bbox `(5,9)`-`(37,37)`,
**zero alpha in the padding**) has no ink outside its own 32x28 content to
brighten, so a plain alpha-blended tint of that one sprite cannot bleed
light past the hex's own edge the way every frame in the burst does.
**Something does draw a second, softer element in sync with the pulse** -
this is stronger than "one frame might have caught a bright phase", but
still not a located geometry/texture to draw, so `oag_ui_screens::campaign::draw`
continues to draw nothing extra for it rather than invent a stand-in shape,
per this project's own rule. Left open, now with a clearer description of
what is missing: not necessarily the literal `"SelectorGlow"` widget this
function's own lookup names, but *some* phase-synced soft-edged draw the
burst makes undeniable.

**`pulse-cellsel` lane, 2026-09-28: confirmed absent from both files this
screen is built from, not merely unreferenced in code.** `oag-wad cat
--expand` on `Data.wad`'s own copies of `Data\Plugins\PI001\GUI\CellMode_Definition.xml`
(826 lines, both `Grid Selection` and `Cell Selection`, no `LoadXML`
include) and `Data\Plugins\PI001\GUI\Skin.xml` (468 lines, the shared
front-end root every screen inherits from) - the two files this project's
own `crate::campaign::load` reads for Race Campaign, in full - contain the
literal string `"SelectorGlow"` **zero times**; the file's only near-miss is
an unrelated `RealGlow="128"` attribute on a different, unrelated `<Values>`
node. `GridController_UpdateSelectorPulse`'s own lookup
(`FUN_088906b0(screen, "SelectorGlow", 0)`, the identical null-checked
by-name lookup `"Selector"` itself resolves through, `if (widget != 0)`
guarding the whole block) therefore always returns null on this screen and
this function's own `SelectorGlow` branch is confirmed dead code for Race
Campaign specifically - not "not yet located", but "not present in the data
this code path reads at all". The burst's own visible halo has to come from
somewhere this pass did not chase: `Selector`'s own XML (`<Image name="Selector">`,
all three copies in `CellMode_Definition.xml`) authors no `blend=`/`additive=`
attribute either, so if the bleed is a blend-mode effect on the already-
implemented colour pulse rather than a second widget at all, it is an engine
default this project has not read, not disc-authored data - the next
concrete lead for whoever picks this up, not a decided explanation.

**`GridSelection_UpdatePageTransition` (`0x088de9bc`, renamed this pass,
confidence 85)**, called at the end of every `GridSelection_Update`, is
mostly the `Grid`/`Grid1` crossfade easing (`fVar4 = 1.0 - elapsed/0.3`,
feeding back into `GridSelection_PopulateTiles`'s own `alpha`/`offset`
parameters as a page turn animates) - but it also tints the two page-arrow
widgets every frame, unconditionally, off the current page bounds alone:

```
up arrow:   page == 0                ? 0xff505050 : 0xffffffff
down arrow: page == maxPage (+0xec)  ? 0xff505050 : 0xffffffff
```

`CellMode_Definition.xml` authors both arrows at a fixed white
(`i="0xffffffff"`), so - like the hex tiles and the selector - this build's
previous plain-white draw was the XML's own untinted default with no runtime
gate applied. Fixed (`ARROW_DISABLED_TINT`), `oag_ui_screens::campaign::draw::grid_draw_list`.
`g_anCellNeighbourOffsets`'s two-parity table and
`Grid_FindCellAtCoordinate` never write `cell->Locked`, `PI_Grid.Locked`,
`Status`, or call any `Unlock_*` predicate - only the glyph's visibility and
fade state change.

**One live-capture caveat this reading exposes**: the profile
`docs/ui/campaign-screens.md` captured had zero medals earned anywhere, so
the neighbour clause could never have fired in that session - the capture
(`grid0`'s lock glyph on exactly the cells with no authored `Locked`
attribute) is evidence for the *two-term* `Locked && no own medal` shape and
says nothing about the third, neighbour-medal term. The "absent `Locked`
defaults to `true`" reading (confidence 72) is unaffected by this, since
none of those cells had a medalled neighbour either.

**Where the "absent defaults to true" question actually has to be settled,
and why it wasn't this pass**: `PI_Cell_ParseElement` (`0x088bf83c`, full
body decompiled this pass) never writes a default value to `+0xb9` - the
`Xml_AttributeAsBool(attr, param_1+0xb9, 1)` call only runs inside the
`if (attribute name is "Locked")` branch, so an absent attribute leaves the
byte at whatever the `PI_Cell` object already held before parsing began.
That makes the answer depend entirely on the object's allocator (zero-init
would default to `false`, a deliberate pre-fill to `true` would explain the
capture), which is a different function from this one. `PI_Cell_ParseElement`
has **no direct callers or xrefs** (`get_function_callers` and `get_xrefs_to`
both return nothing) - it is reached the same vtable/generic-XML-dispatch
way every other `_ParseElement` in this file is, so finding the allocator
means tracing that dispatch table, not this function. Left open; the
existing confidence-72 reading stands as a live-capture inference, not a
decompiled fact.

**What is still open, and could not be settled from decompilation alone**:
neither `CellSelection_CommitSelection` (`0x088d6138`) nor
`CellSelection_Update` (`0x088d6430`) reads `cell->Locked` or
`Cell_BestMedal` to *refuse* committing the currently-selected cell - as far
as PI001's own code goes, **confirming a `Locked="true"`, no-medal, no
medalled-neighbour cell would launch it normally**. Whether that is actually
true in the original, or a generic (non-PI001) widget rule refuses `Confirm`
whenever the focused tile's lock layer is visible, is exactly the same open
question the tier rule below raises, and is the single highest-value
breakpoint left for whoever holds PPSSPP next (see "What the next pass
should verify" at the end of this section).

### The tier unlock rule: `PI_Grid.Locked` is the static default, cleared by this grid's own points or the previous tile's

`GridSelection_PopulateTiles` (`0x088de630`, confidence **82** - the
`GridSelection` counterpart to `CellSelection_PopulateGrid`, same
`GridController_*` accessor family, not runtime-verified) fills the four
tiles of the currently-shown page:

```
for column in 0..3:                                          # the four tiles on this page
    grid = GridSelection_FindTileAt(screen, column, current_page)
    if grid == 0: continue
    GridController_SetTileFlags(widget, BASE_LAYER, column, 0, VISIBLE)
    GridController_SetTileColor(widget, BASE_LAYER, column, 0, pulsing_teal)
    if grid->Locked (+0xa0) != 0:                             # PI_Grid's own Locked byte
        GridController_SetTileFlags(widget, LOCK_LAYER, column, 0, VISIBLE)
        GridController_SetTileColor(widget, LOCK_LAYER, column, 0, pulsing_white)
        previous = GridSelection_FindTileAt(screen, column - 1, page (wrapping to the prior page's last slot))
        if previous != 0 and cached_required_points[previous] <= cached_points_earned[previous]:
            GridController_ClearTileFlags(widget, LOCK_LAYER, column, 0, VISIBLE)   # previous tile's points met - hide anyway
    pointsEarned = cached_points_earned[grid]
    if pointsEarned != 0:                                     # this grid has any points of its own
        GridController_ClearTileFlags(widget, LOCK_LAYER, column, 0, VISIBLE)       # hide the lock unconditionally
        GridController_SetTileColor(widget, COLOUR_LAYER, column, 0, medal_tier_colour(pointsEarned, ...))
```

This last `if pointsEarned != 0` check runs **unconditionally after the
`Locked` branch**, not inside it - so a grid the player has already scored
any points in has its lock glyph hidden regardless of `Locked` and
regardless of the previous tile, exactly mirroring the cell rule's "the
cell's own medal clears it" term. The full three-term predicate, symmetric
with the cell one above:

```
tier lock glyph visible  ⟺  grid->Locked (+0xa0) != 0
                        AND  this grid's own Grid_PointsEarned == 0
                        AND  NOT (previous tile's earned >= previous tile's required)
```

`GridSelection_FindTileAt` (`0x088de320`, confidence **80**) is
`GridSelection`'s own coordinate lookup, structurally identical to
`Grid_FindCellAtCoordinate` but over the cached `PI_Grid*` array
`GridSelection_PopulateGrids` builds, matching against two **previously
undocumented** `PI_Grid` fields at `+0xa8` and `+0xac` (not parsed by
`PI_Grid_ParseElement`, so written elsewhere, presumably at
grid-list-collection time as the grid's own flat tile position: column
`0..3` and page `0..3`). Their writer was not located this pass, so they are
recorded here as bare offsets rather than named.

**So `PI_Grid.Locked` (`+0xa0`) does drive the tier's lock glyph directly**,
closing race-campaign.md's own long-standing "`Locked` on `PI_Grid`... no
consumer traced" item (see "The authored data" section above) - the same
shape as the `PI_Cell` finding, and settled by the same kind of read.
Confidence **82**.

**The "previous tile" check is a hard-coded UI shortcut, not the real
`<Unlock Grid="...">` predicate.** It does not call `Unlock_GridPointsMet`
(`0x0888ebd8`) or read the `<Unlock>` node's `Grid=` attribute at all -
it directly compares the *immediately preceding tile's own* cached
points-earned/required-points (the same arrays `GridSelection_PopulateGrids`
builds for the honey-counter math already documented above). This
reproduces the correct shipped behaviour **only because** every authored
`<Unlock Grid=>` from `grid1` onward names exactly the tile immediately
before it - a fact this page's own unlock-ladder table already established
(`grid1` unlocks on `Grid0`, `grid2` on `Grid1`, ... `grid15` on `Grid14`).
Nothing in the 16 shipped grids exercises the divergent case (a grid naming
a non-adjacent `<Unlock Grid=>`), so the UI's shortcut and the data-driven
mechanism cannot be told apart on the shipped disc - but they are two
different code paths, and a modded or future grid that broke the adjacency
assumption would show the wrong lock state without actually being ungated
(or vice versa). Confidence **85** for the mechanism, **90** for "it
reproduces the shipped ladder" (arithmetic invariant against the existing
12/16/20/24/28 table).

**`GridSelection_CommitSelection` (`0x088de180`, word 39 of `GridSelection`'s
vtable at `0x08ad0064`, positionally confirmed the same `read_memory` way
`CellSelection`'s was - word 9 at this address decodes to
`GridSelection_Update` and word 29 to `GridSelection_OnEnter`, both already
named, corroborating the offset) does not check `Locked` at all.** In full:

```
FUN_088902c8()                                    # base-class hook, unread - shared with CellSelection_CommitSelection
if screen->highlightedIndex (+0xdc) != -1:
    DAT_08b30fb8 = screen->gridDefinitions[+0xdc]   # unconditional - no Locked check
    if DAT_08b30fb8 != 0:
        strcpy(DAT_08b30fbc, DAT_08b30fb8->name)    # +0x74, display copy
```

Confidence **82**. So, precisely mirroring the cell-level finding: **no
PI001 function this pass traced ever refuses a grid-tier confirm.** The
`GridSelection` screen's own XML redirect
(`Data\Plugins\PI001\GUI\CellMode_Definition.xml`,
`<Screen type="GridSelection">`'s own `<Redirect><Default
goto="Cell Selection"/></Redirect>`) is **unconditional** too - no `<a
forward=>` button guard, no `<Entry item= equals=>` value test, unlike every
other conditional redirect this file documents. Yet
[`docs/ui/campaign-screens.md`](../../../ui/campaign-screens.md)'s "Grid-tier
locking is not cosmetic" finding measured, live, that pressing `Confirm` on a
`Locked="true"` tile does nothing. **Both facts are true at once only if the
actual refusal lives outside PI001** - most plausibly a generic widget/engine
rule ("a focused `GridController` tile cannot be confirmed while its
lock-layer flag is set"), which this pass's scope (the Ghidra bridge, PI001's
own functions only) cannot reach. If that generic rule exists, it would
apply to `CellSelection` identically, which would mean the cell-level
`Locked` byte is **also** a real gameplay gate after all - just enforced once,
generically, rather than duplicated per screen. This is now the single most
important open question this page has, because it recasts both "is this cell
playable" and "is this tier enterable" as depending on the same unresolved
mechanism rather than two independent ones.

### The dispatcher at `0x08891360`: it is `StateMachine_TransitionTo`, already named, and the redirect chain lives inside it

`0x08891360` (the return address the previous pass's PPSSPP breakpoint
caught) is not a function entry point - it is the instruction immediately
after the `jalr` at `0x08891358` inside `StateMachine_TransitionTo`
(`0x0889123c`, confidence 88, already named and documented on
[`main-loop.md`](main-loop.md)). Confirmed by `get_function_by_address` (body
`0889123c`-`08891447`) and by disassembling that range directly: the `jalr`
at `0x08891358` calls `*(*(int*)(old_state + 0x38) + 0x9c)` - word 39, byte
offset `0x9c`, the exact `CommitSelection` slot this page already
positionally confirmed on `CellSelection`, `TrackSelection` and
`TeamSelection` - on `old_state` (found via `StateMachine_FindState` against
the machine's current-state name), passing through the transition's own
`param_3`. This is the generic "`Screen_CommitSelection`-style virtual call"
the previous pass predicted from the runtime trace alone; confidence **90**
for "`StateMachine_TransitionTo` is what calls a screen's word-39 slot on the
way out," now read directly from the decompile rather than inferred from one
return address.

**New this pass: the `<Redirect>` mechanism itself is implemented inside
this same function, and needs no button or name lookup at all.** Immediately
after the `CommitSelection` call, at `0x08891360`-`0x08891398`:

```
while (new_state->flags (+0xd0) & 1) and (new_state->redirectTarget (+0x94) != 0):
    new_state = StateMachine_FindState(machine, new_state->redirectTarget, create=1)
```

So a `<Redirect>` block is simply a state table entry with bit 0 of a flags
byte at `+0xd0` set and a resolved target pointer at `+0x94` - `<Default
goto="X">` is what populates `+0x94`. The chain is walked *before* the final
state's `OnEnter` (`+0x74`) fires and *before* the outgoing state's `OnExit`
(`+0x7c`) fires (call order: `CommitSelection(old)` -> walk redirects ->
`OnEnter(new)` -> `OnExit(old)`). This is a **name-free, button-free
selection**: whichever other state names a redirect block as its own `goto`
target is the only thing that ever reaches it, exactly matching the earlier
pass's own finding that `search_strings` turns up neither
`"Cell Mode Redirect Team"` nor `"Cell Mode Redirect Game"` anywhere in the
executable - native code never needs to look either name up, because nothing
resolves a redirect *by* name at transition time. **Still open**: which
state names `Cell Mode Redirect Game` as its `goto` (skipping straight to
`Launch Game`) was not located this pass either - that requires reading the
XML's own button/`<a forward=>` wiring for `CellSelection`'s confirm action,
not this generic transition function, and is a different investigation from
the one this pass's tools (the Ghidra bridge alone) are suited to finish
quickly.

### `Grid`/`Grid1`: a double-buffer pair, not two independent widgets

`GridSelection_Update` (`0x088dec24`, already named, re-read in full this
pass) toggles an index at `screen+0x114` between `0` and `1` on every
successful page move (`*(uint*)(screen+0x114) = (index + 1) & 1`), and reads
`screen+0x10c`/`screen+0x110` (`Grid`/`Grid1`) as a two-element array indexed
by it - `screen + index*4 + 0x10c`. On a page move it: clears the cursor-
highlight flag (bit `0x200`) on the **currently active** buffer,
re-populates the buffer that is about to become active via
`GridSelection_PopulateTiles` with the new page's four grids, flips the
index, then sets the cursor-highlight flag on the **newly active** buffer.
Confidence **82** - direct decompile of the toggle and the flag writes, not
runtime-verified.

This settles what the previous pass scored 50: `Grid`/`Grid1` are a
**double-buffered pair of the same `GridController` row widget**, one always
holding the currently-displayed page and the other being (re)populated with
the *next* page in the background, swapped by flipping which one is
"active" rather than by animating either one's contents. It also explains,
structurally, [`docs/ui/campaign-screens.md`](../../../ui/campaign-screens.md)'s
"no visible crossfade... down to the earliest captured frame" finding: an
index flip between two already-fully-populated widgets has no interpolated
state to be caught mid-transition, unlike a blend or scroll would.

### The cell movement rule: not this table - a negative result

`Grid_FindCellAtCoordinate` and `g_anCellNeighbourOffsets` have **exactly one
caller between them**, `CellSelection_PopulateGrid` (confirmed by
`get_function_callers` on both) - neither is reached from any
button/d-pad-handling code this pass located. `CellSelection_Update`
(`0x088d6430`, read in full) drives the AI-difficulty cycle (button 7) and
the per-cell fade timers, but never reads a directional input at all; cursor
movement across the hex grid must be driven by a different function this
pass did not find, most plausibly on the same generic, non-PI001
`GridController` widget class the tier-confirm question above also points
to. **So the six-neighbour table does not double as the movement rule** -
this project's own chosen nearest-neighbour cursor search on the staggered
grid is not yet correctable against a real one from this pass's evidence.
Left open.

### What the next pass should verify, in PPSSPP

In order of how much each would move, and additive to the priority list the
previous pass already left:

1. **Breakpoint on `Confirm`/cross with the cursor on a `Locked="true"`,
   unmedalled `Cell Selection` tile that also has no medalled hex neighbour**
   (`grid0` has none such once a few cells are medalled, or use a fresh
   profile's `grid1`+ once reachable). Does `CellSelection_CommitSelection`
   (`0x088d6138`) fire at all? If it does not, the refusal is a generic
   widget rule and both open items above (cell and tier) collapse to the
   same mechanism; if it does fire, `Locked` is confirmed cosmetic-only at
   the cell level and the tier-level refusal is a *different*, still-unknown
   mechanism.
2. **Single-step (or breakpoint) the widget/input code between an armed
   `Confirm` press and `StateMachine_TransitionTo`'s entry**, on a locked
   `Grid Selection` tile, to find what (if anything) suppresses the
   transition attempt before it starts - this is the direct way to answer
   (1) without needing a second cell-level test.
3. **Read `*(GridController+0x2c)` on a locked tile** (both a `Grid
   Selection` tile and a `Cell Selection` cell) for a bit that flags
   "not interactive," to check the "generic rule" hypothesis structurally
   rather than behaviourally.

### Runtime-verified 2026-09-14 (deliverable 1: does a lock refuse confirm)

PPSSPP v1.20.4 (SDL build), `pulse-psp-usa.chd`, Xvfb `:97`, debugger on
`ws://127.0.0.1:47810/debugger`, same zero-medal profile the earlier
2026-09-14 pass used (confirmed again live: `grid0`'s `Gold medals 0/8`).
All three breakpoints taken, each with a same-session positive control on
an unlocked tile of the same kind, arming order `cpu.stepping` ->
`cpu.breakpoint.add` -> `cpu.resume` throughout. `grid_00.xml` read fresh
this pass (`just wad cat`) to pick a target with certainty: `grid0_2_1` and
`grid0_4_1`/`_2_2`/`_4_2`/`_2_3`/`_4_3` carry no `Locked` attribute at all
(the six glyphed cells), `grid0_3_1` and `grid0_3_2` explicitly author
`Locked="false"` (the two the previous pass's own captures already showed
unlocked and, in `grid0_3_2`'s case, already breakpoint-confirmed to
launch). `grid0_2_1` (`Race`, `16_Track`, no `Locked` attribute, zero
medals anywhere on the profile so no hex-adjacent medal either) is this
pass's locked test cell.

**1. `CellSelection_CommitSelection` (`0x088d6138`) does not fire on a
locked cell.** Positive control first: cursor on `grid0_3_1` (unlocked),
armed the breakpoint, pressed `cross` - fired at `pc=0x088d6138`,
`a0=0x08d73170`, `a1=0x08d731c4`, `ra=0x08891360`, state advanced to `Team
Selection`, matching the existing 2026-09-14 trace on `grid0_3_2` digit for
digit. Backed out (`circle` x2) to `Cell Selection`, moved the cursor onto
`grid0_2_1` (confirmed by the panel: `Talon's Junction White`, `16_Track`'s
own display name, matching `grid_00.xml`'s `track="16_Track"` for that
cell), re-armed the same breakpoint, pressed `cross`: **the breakpoint never
fired** (`wait_for_break` timed out at 6 s; the `input.buttons.press`
timeout trap did not fire this time either, since the press itself has
nothing to hit and returns normally when the confirm has nowhere to go).
The state name stayed `Cell Selection` throughout. Confidence **90** -
runtime trace, single binary, with its own positive control in the same
session on the same screen.

**2. `StateMachine_TransitionTo` (`0x0889123c`) is not entered either, so
the refusal is not "a transition starts and something short-circuits it" -
no transition is even attempted.** Same locked `grid0_2_1` selection,
breakpoint moved to `StateMachine_TransitionTo`'s own entry: **no hit**
within 6 s. Positive control on `grid0_3_1` again: fired at
`pc=0x0889123c`, `a0=0x08d0a820` (the state-machine pointer,
`G_STATE_MACHINE`'s own value), `a1=0x08d8d450` (the target state, almost
certainly `Team Selection`'s own state-table entry), `a2=0x08d731c4`
(the same value `CommitSelection`'s `a1` carried above - passed straight
through as the transition's own `param_3`), `a3=0x08a6bbcc`,
**`ra=0x088c8a10`** - a concrete, previously unseen return address for a
function that calls `StateMachine_TransitionTo` on a cell confirm. Not
decompiled this pass (no Ghidra bridge held), but it is now the direct
target for whoever holds the bridge next: this is almost certainly the
`CellSelection`-side confirm-button handler (or a shared, generic one -
see the tier result below), sitting between the `cross` press and the
state machine, and it is the earliest point in the call chain this pass's
tools can place the refusal at. The same test repeated on the tier side
(below) points at the identical two-function shape, which is why "generic,
non-PI001 rule" is now the leading reading rather than a hedge. Confidence
**85** for "no transition is attempted on a locked cell" (one cell, one
positive control, but a clean binary yes/no on an address independently
confirmed to fire).

**3. The tier side reproduces both results exactly, with its own positive
control.** `GridSelection_CommitSelection` (`0x088de180`) and
`StateMachine_TransitionTo` (`0x0889123c`) were each armed in turn with the
cursor on `GRID 2` (`grid1`, `Locked="true"`, `RequiredPoints="16"`,
`0/10` gold medals, `000/030` points - confirmed on the panel): **neither
fired**, matching the cell result term for term. Positive control on `GRID
1` (`grid0`, unlocked, already the page's default cursor): `cross` fired
`GridSelection_CommitSelection` at `pc=0x088de180`, `a0=0x08d357e0`,
`a1=0x08d35834`, `a2=0`, `a3=0x088de180`, `ra=0x08891360` - the identical
internal call site inside `StateMachine_TransitionTo` that
`CellSelection_CommitSelection`'s own `ra` already carried, confirming
both screens' word-39 slots are reached through the same dispatch path -
and the state advanced to `Cell Selection`. **So the cell-level and
tier-level questions are now the same question, answered the same way**:
on both screens, confirming a `Locked`, no-medal(/no-points) tile never
reaches `StateMachine_TransitionTo` at all, let alone either screen's own
`CommitSelection` slot. Confidence **88** - two screens, two functions,
each with its own positive control, all four results consistent with one
mechanism.

**4. `GridController_SetTileFlags` (`0x088a3638`) fired live with exactly
the `(widget, layer, column, row)` shape `GridSelection_PopulateTiles`'s
decompiled pseudocode already predicted, and its layer writes line up
digit for digit with the known lock state.** Armed on screen re-entry
(back to `Main Menu`, `cross` into `RACE CAMPAIGN`) rather than on a
confirm - this function fires from `PopulateTiles`, not from a button
press. Two widget pointers hit in sequence, `0x08d38320` then `0x08d6fe40`
(the `Grid`/`Grid1` double-buffer pair the previous pass's decompile-only
reading already named), each with an **identical** six-call pattern for
page 1's four tiles: `(widget, 1, 0, 0)`, `(widget, 1, 1, 0)`, `(widget, 4,
1, 0)`, `(widget, 1, 2, 0)`, `(widget, 4, 2, 0)`, `(widget, 1, 3, 0)` (the
capture window closed here, at 20 hits; the seventh call, `(widget, 4, 3,
0)`, is inferred from the pattern rather than captured). Column 0
(`grid0`, unlocked) gets **only** the base layer (`1`); columns 1-3
(`grid1`/`grid2`/`grid3`, all `Locked="true"` on this profile) each get
**both** the base layer and the lock layer (`4`) - exactly
`GridSelection_PopulateTiles`'s own `if grid->Locked != 0: SetTileFlags(...,
LOCK_LAYER, ...)` branch, live, on the real four tiles of the real page,
with `row` fixed at `0` and `column` counting `0..3` exactly as the
pseudocode's `for column in 0..3` names it. This is a **runtime trace of a
non-input code path**, not a confirm-refusal test, but it is the strongest
evidence yet for `GridSelection_PopulateTiles`'s reading of `PI_Grid.Locked`
- raised **82 -> 90**, and `GridController_SetTileFlags` itself **82 -> 90**
- and it independently corroborates the panel readings for `GRID 1`/`GRID
2` above (`0/10` medals, `Locked="true"` on `grid1`) from a completely
different address. **`*(GridController+0x2c)`'s own "interactive" bit was
not reached this pass** - `SetTileFlags`'s arguments are `(widget, layer,
column, row)`, not a per-tile struct pointer this pass could dereference
without decompiling how `widget`/`layer`/`column`/`row` resolve to a tile
base address, which needs the Ghidra bridge this pass did not hold. The two
widget pointers above (`0x08d38320`, `0x08d6fe40`) are recorded so that
work does not have to re-find them.

**The answer to "does a lock refuse confirm," at confidence 85-90 across
four independent breakpoints and their positive controls**: yes, on both
`Cell Selection` and `Grid Selection`, and the refusal happens **before**
`StateMachine_TransitionTo` is ever entered - so it is not any of the
functions this project has decompiled so far (`CellSelection_Update`,
`CellSelection_CommitSelection`, `GridSelection_CommitSelection`,
`StateMachine_TransitionTo` itself), all of which sit downstream of where
the confirm press is actually being swallowed. The likely mechanism is
still the generic, non-PI001 widget rule the previous pass hypothesised
(the confirm dispatcher upstream of the state machine, return address
`0x088c8a10`, un-decompiled), now narrowed from "somewhere before the
transition" to "the specific function at `0x088c8a10` or something it
itself calls" - a concrete next target rather than an open-ended search.
**Cosmetic-only at either level is ruled out**: a `Locked` tile genuinely
cannot be entered or played in the original, on this profile, at either
granularity.

### `0x088c8a10` decompiled: a negative result, not the lock check

`0x088c8a10` is a return address inside `StateMachine_EvaluateRedirect`
(`0x088c8798`, confidence 75, [full read on the EndRace screens
page](endrace-screens.md#deliverable-3-0x088c8a10s-containing-function-is-not-a-lock-check) -
decompiled that pass because it was reached chasing the EndRace screens'
own confirm handling and turned out to be the same function this page's
positive control already pinned). It is the generic `<Redirect><Entry
item= equals= goto=>...<Default goto=>` evaluator every screen's own
`<Redirect>` block goes through on confirm - eleven call sites across the
binary, none of them specific to `CellSelection` or the campaign. **It
never reads `Locked`, a medal, or any `PI_Cell`/`PI_Grid`-shaped offset** -
every field it touches is the redirect node's own `Entry` list, `Default`
target, and the special-cased widget name `"Focus"`. So the function
containing `0x088c8a10` is **not** the predicate that swallows a confirm on
a locked tile; whatever does that runs *before* this function is ever
invoked on the node in question, narrowing "somewhere before
`StateMachine_TransitionTo`" to "somewhere before
`StateMachine_EvaluateRedirect`, or in whichever of its eleven callers
decides whether to invoke it at all" - still open, not closed. One of those
callers, `0x088d7e1c`, is close in address to `CellSelection`'s own code and
carries an unconfirmed "enabled" gate (`*(short*)(redirectNode + 0xbe)`)
ahead of its own confirm-button check - a plausible site for a per-tile
lock gate, not yet shown to be one; see the EndRace screens page's own
write-up of it (confidence 55, not renamed) for the exact shape and the
breakpoint that would settle it.

### `0x088d7e1c` ruled out; `Cell Selection`'s real dispatcher is `ConfirmButton_Update`, and it isn't the gate either

**Runtime-confirmed 2026-09-14** (full transcript on the [EndRace screens
page](endrace-screens.md#0x088d7e1c-ruled-out-the-real-cell-selection-confirm-dispatcher-found-and-the-swallow-narrowed-to-input-consumption)):
`0x088d7e1c` is the held-confirm variant used by `InGame Photo`, not
`Cell Selection` - it never runs on this screen at all, so its `+0xbe` flag
was a dead end. Breakpointing `StateMachine_EvaluateRedirect` directly, with
the cursor on `grid0_3_1` (unlocked, positive control) then `grid0_2_1`
(locked, `left` from the default cursor, negative control - the same two
cells "Runtime-verified 2026-09-14 (deliverable 1)" above used), found the
real caller: `$ra = 0x088c907c`, inside `ConfirmButton_Update` (`0x088c8e88`,
confidence 85) - fired on the unlocked positive control, **never fired** on
the locked negative control, reproducing this page's own cell/tier findings
from a third, independent function.

`ConfirmButton_Update` itself was then cleared as the gate: passively
breakpointed at its own entry (no button pressed), it fires identically -
same instance, same internal state - regardless of which tile the cursor is
on, and its own decompiled accept-branch (`0x088c8fec`-`0x088c9074`) reaches
the `StateMachine_EvaluateRedirect` call unconditionally once "accept is
pressed" is true, with no `Locked`/medal read anywhere in that span. So the
swallow is not inside any PI001 function traced so far, on either screen -
it is specifically why `ConfirmButton_Update`'s own accept-button check
(`Input_IsPressed(g_input, 4, 0)`, a screen-global button code, not a
per-tile one) reads *false* on a locked-tile frame despite `cross` being
held for the whole 6 s of the negative-control press. **The open question is
now input consumption**: what calls `Input_ConsumePress`/otherwise makes
`Input_IsPressed` miss button `4` specifically when the focused
`GridController` tile's lock layer is visible - not located this pass, and
a materially narrower target than either the eleven-callers list or the
unconfirmed `+0xbe` flag were.

## What is not determined

- **`g_class_name_table` (`0x08ab067c`) is read above as four entries, and a
  shipped `Zone` cell's own `class=` attribute contradicts that.** Every
  `Zone`-mode cell in `grid_00.xml` .. `grid_15.xml` carries the literal
  `class="Zone"` (e.g. `grid0_4_2`), not one of `Venom`/`Flash`/`Rapier`/
  `Phantom` - found by
  [`crates/tables/src/race_campaign.rs`](../../../../crates/tables/src/race_campaign.rs)'s
  parser against the real disc, not re-read in Ghidra. Either the table has a
  fifth entry this pass missed, or `PI_Cell_ParseElement` leaves an
  unrecognised `class=` string in the field rather than rejecting it - both
  are consistent with `CellSelection_PopulateDetail` blanking the class label
  for `Zone`. Left open rather than guessed; the parser keeps `class` as a raw
  string and maps only the four known names, per
  [`docs/formats/race-campaign.md`](../../../formats/race-campaign.md).
- ~~The in-race HUD's medal tier is a different value and is still
  unread...~~ **Closed 2026-09-28.** The writer is `PlayerStatus_Update`
  (`0x0883b3b8`), runtime-verified with a PPSSPP write watchpoint. It is
  genuinely a different ordinal from `Cell_EvaluateMedal`'s, as this bullet
  suspected - `0 = BRONZE` up to `3 = RECORD`, related to
  `Cell_EvaluateMedal`'s `0 = gold, 1 = silver, 2 = bronze` by `2 - medal` on
  the three tiers they share - evaluated live, every tick, against **this
  same cell's own** `gold`/`silver`/`bronze` fields (`cell + 0xa0/0xa4/0xa8`,
  exactly `Target0..2` above) for a campaign Time Trial/Speed Lap cell,
  **except that `RECORD` can still win there** when the player's own stored
  personal best beats gold and the live pace beats that too; or a separate
  personal-best/track-record pair otherwise (always `RECORD` on that second
  path). Full law and the live evidence:
  [`race-progress.md`](race-progress.md#the-target-time-readout-0x780x7c0x80-closed-2026-09-28).
  Wired into the HUD for the campaign branch's gold/silver/bronze ladder,
  `oag_hud::TimeTrialPace`; `RECORD` on either path stays
  unimplemented - see that page's own "What stays open".
- **`Locked` on both `PI_Cell` (`+0xb9`) and `PI_Grid` (`+0xa0`) is now
  settled, 2026-09-14** - see
  ["Unlock rules, cell and tier"](#unlock-rules-cell-and-tier) above.
  `CellSelection_PopulateGrid` (`0x088d5de4`) and `GridSelection_PopulateTiles`
  (`0x088de630`) each draw their own lock glyph exactly when `Locked != 0` and
  no medal/points threshold has been met yet. **The six-neighbour adjacency
  loop is traced in full and is *not* a second unlock mechanism** - it is a
  cosmetic lock-glyph fade trigger (`Grid_FindCellAtCoordinate`,
  `g_anCellNeighbourOffsets`, the `GridController_*` widget-flag family), and
  the tier side has its own hard-coded "previous tile" shortcut instead of
  calling `Unlock_GridPointsMet`. **`Status` (`+0xb8`) on a `PI_Cell` remains
  untraced** - no shipped grid file sets it. `Definition_IsUnlocked` and
  `FUN_0888e5e4` remain ruled out as any of these bytes' reader.
- **Runtime-verified 2026-09-14: `Locked` (either byte) does gate `Confirm`,
  and the refusal sits upstream of every function this project has
  decompiled.** Four breakpoints (`CellSelection_CommitSelection`,
  `StateMachine_TransitionTo`, the tier-level repeat of both, and
  `GridController_SetTileFlags` as a structural check), each with its own
  positive control on an unlocked tile of the same kind, all agree: on a
  `Locked`, unmedalled `Cell Selection` cell (`grid0_2_1`, no hex-adjacent
  medal either, on a zero-medal profile) and on a `Locked` `Grid Selection`
  tier (`grid1`/`GRID 2`), `StateMachine_TransitionTo` is never entered at
  all - so neither screen's own `CommitSelection` vtable slot ever gets the
  chance to check the byte, cosmetic or otherwise. The confirm press is
  being swallowed before the state machine is even asked to transition.
  `StateMachine_TransitionTo`'s own `ra` on the *successful* positive-control
  runs is `0x088c8a10` - a concrete, previously unseen return address, not
  yet decompiled (no Ghidra bridge held this pass), and the next target for
  finding the actual generic rule. See `race-campaign.md`'s own new
  "Runtime-verified 2026-09-14 (deliverable 1)" subsection under "Unlock
  rules, cell and tier" for the full four-breakpoint transcript, including
  `GridController_SetTileFlags` reproducing `GridSelection_PopulateTiles`'s
  lock-layer branch live, tile for tile, on the real page.
  **Decompiled, 2026-09-14: a negative result.** `0x088c8a10` sits inside
  `StateMachine_EvaluateRedirect` (`0x088c8798`), the generic, screen-agnostic
  `<Redirect>` evaluator - it never reads `Locked` or a medal, so it is not
  the swallowing predicate itself. See "`0x088c8a10` decompiled" above.
  **Narrowed further, 2026-09-14 (runtime):** the real `Cell Selection`
  dispatcher (`ConfirmButton_Update`, `0x088c8e88`) is found and cleared too
  - see "`0x088d7e1c` ruled out..." above. The gate is upstream of every
  PI001 function traced on either screen, specifically in whatever consumes
  button `4` before `ConfirmButton_Update`'s own `Input_IsPressed` check
  sees it on a locked-tile frame.
- **`Group` (`+0xb0`) on a `PI_Grid`** is `1` on `grid12`..`grid15` and absent
  elsewhere; its consumer was not traced. A "these four are the expert set"
  reading is plausible and unverified.
- **`PI_Grid` carries two more previously-unlisted fields, `+0xa8` and
  `+0xac`**, read by the new `GridSelection_FindTileAt` (`0x088de320`) as a
  `(column, page)` tile position - not written by `PI_Grid_ParseElement`, so
  computed elsewhere at grid-list-collection time. Writer not located.
- **The cell cursor-movement rule (d-pad across the staggered hex grid) is
  not `g_anCellNeighbourOffsets`** - that table and
  `Grid_FindCellAtCoordinate` have exactly one caller between them
  (`CellSelection_PopulateGrid`, confirmed by `get_function_callers`), not
  any input-handling code. The real movement rule was not located this pass;
  a reimplementation is still on its own chosen nearest-neighbour search.
- ~~The `Loyalty` unlock predicate... does not read as a loyalty counter~~
  **Resolved, 2026-09-14, decompiling the EndRace screens
  ([`endrace-screens.md`](endrace-screens.md)).** `Unlock_LoyaltyMet` reads a
  **different** store from a cell's own record: `DAT_08b31774`, a per-team
  record keyed by team name through the same generic accessor
  (`FUN_08808664`) `race-box-screens.md` already names for `TeamSelection`'s
  per-craft rating table. `EndRaceRewards_Update` independently reads that
  same store's own `+8` field as a plain accumulated loyalty total (feeding
  the `EndRace Rewards` screen's `loyaltynum`/`loyaltybar`, "Total loyalty"),
  which is decompiled corroboration - not a runtime trace - that
  `Unlock_LoyaltyMet`'s comparison is exactly what its name says, not a
  misread of a cell-shaped `difficulty | medal << 8` word. Raised
  `Unlock_LoyaltyMet`/`Unlock_LoyaltyValue` 72/78 -> 82/82 on this basis.
  **Fully closed 2026-09-14**: the writer is `Race_ComputeLoyaltyAward`
  (`0x0880ac50`), called from `Race_BuildEndRaceResult` (`0x0882a498`) and
  reproducing `90` exactly on two independent live races - see
  `endrace-screens.md`'s "The loyalty-award computation, decompiled and
  runtime-confirmed" section for the full law and both data points.
- **Only the USA pressing was read.** The EU and PS2 pressings of Pulse, and
  Pure and HD/Fury, were not checked for `Data\Plugins\grids` at all.
- **Nothing is runtime-verified.** No PPSSPP breakpoint was taken this pass;
  every score is capped at the rubric's 85-94 "arithmetic invariant across many
  real files" band and most sit at 84-90.
- **The fifth breakpoint (`DAT_08b31158`, a confirmed Tournament cell) is
  still not taken, deliberately, 2026-09-14.** This pass's own deliverable-1
  work (see "Runtime-verified 2026-09-14 (deliverable 1)" above) settled the
  precondition it was waiting on: `Locked` genuinely refuses `Confirm` at the
  tier level too, so a Tournament cell (every one of which lives on `grid1`
  or later) stays unreachable without first earning `grid0`'s own 12
  `RequiredPoints`. This pass's own deliverable-2 race (`grid0_3_2`, `Time
  Trial`) earned **no medal** (`3.11.76` against a `2.03.00` bronze target -
  see `docs/ui/campaign-screens.md`'s "After a campaign race, measured"), so
  it did not open `grid1` either. Neither a second race attempt (this pass's
  autopilot has no per-craft filter, and the two unlocked cells are both
  hard to medal quickly - a position-based `Race` needs an untested 8-craft
  capture, a faster `Time Trial` run needs autopilot tuning) nor a memory
  poke of the record store (the aggregate `Grid_PointsEarned` sums
  `Cell_MedalPoints` over every cell in the grid, not a single counter, and
  `ppsspp-debugger.md` already documents `memory.write` crashing the
  emulator on more than one occasion) was judged worth the "modest effort"
  this item was scoped to. Left for a pass that either earns the points
  properly or accepts the memory-write risk on its own terms.

## Reproducing the data

```sh
just wad cat data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
  'Data\Plugins\grids\Definition.xml'
just wad cat data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
  'Data\Plugins\grids\grid_00.xml'
just wad cat data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
  'Data\Plugins\PI001\GUI\CellMode_Definition.xml'
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
| `0x0888ea30` | `Unlock_LoyaltyMet` | 82 |
| `0x0888f064` | `Unlock_LoyaltyValue` | 82 |
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
| `0x088d5cb8` | `CellSelection_OnExit` | 82 |
| `0x088d5de4` | `CellSelection_PopulateGrid` | 85 |
| `0x088d6138` | `CellSelection_CommitSelection` | 93 |
| `0x08888ee0` | `Globals_Set` | 92 |
| `0x08888db4` | `Globals_HashKey` | 78 |
| `0x08945890` | `Libc_HashString` | 85 |
| `0x088ed784` | `TrackSelection_CommitSelection` | 80 |
| `0x088e9fa0` | `TeamSelection_CommitSelection` | 74 |
| `0x088c072c` | `Grid_FindCellAtCoordinate` | 85 |
| `0x08ab1e68` | `g_anCellNeighbourOffsets` (data) | 85 |
| `0x088a3638` | `GridController_SetTileFlags` | 82 |
| `0x088a374c` | `GridController_ClearTileFlags` | 82 |
| `0x088a3850` | `GridController_SetTileColor` | 85 |
| `0x088a3ad0` | `GridController_SetLockTint` | 78 |
| `0x088a39d4` | `GridController_IsLockFading` | 65 |
| `0x088a39ac` | `GridController_ClearLockFade` | 65 |
| `0x088a3a40` | `GridController_ResetLockFades` | 68 |
| `0x088de180` | `GridSelection_CommitSelection` | 82 |
| `0x088de630` | `GridSelection_PopulateTiles` | 82 |
| `0x088de320` | `GridSelection_FindTileAt` | 80 |
| `0x08809980` | `Profile_DifficultyRC` | 85 |
| `0x088098f0` | `Profile_SetDifficultyRC` | 85 |
| `0x088d6430` | `CellSelection_Update` | 85 |

## Where `Cell Selection` keeps its cursor across a back-out (2026-10-02, `pulse-loyaltybar`)

Headless decompile of `CellSelection_OnEnter` (`0x088d59c4`) and `CellSelection_Update`
(`0x088d6430`), reading the one question the earlier passes left: what the "cursor" is
and where it lives. Confidence **70** from the decompile alone; **raised to 90 on
2026-10-02 (`pulse-cursor-live`)** by a runtime read on two boots, see the last bullet
and [`docs/ui/campaign-screens.md`](../../../ui/campaign-screens.md)'s "Where the cursor lives".

- **The cursor is the `Selector` grid-controller widget's own slot**, `(+0xa0, +0xa4)`,
  not a cell pointer. `CellSelection_Update` rebuilds the name `"%s %d %d"` of the current
  grid's name (`DAT_08b30fb8 + 0x74`; `"UserGrid_%d_%d"` for a player grid) and the
  Selector's `(x, y)` every frame, finds the collected cell definition with that name, and
  stores it in the screen's `+0xdc` (`CellSelection_PopulateDetail` runs when it changes).
  So `+0xdc` is derived, never the source.
- **`OnEnter` re-resolves it**: if `+0xdc != 0` it builds the same name from the
  Selector's *surviving* `(x, y)`, scans the cells for a name match whose slot also passes
  `FUN_088a37cc(selector, 4, cell.x, cell.y, 4) == 0` (a flag test on the tile at that
  grid position; not chased), re-seats the Selector on that cell and refreshes it
  (`FUN_088a4248`). No match (the name carries the *current* grid's name) clears `+0xdc`.
- **The first-visit default scan only runs when `+0xdc` was cleared or was zero and the
  screen was not entered from or returning to `Cell Help`** (`strcasecmp` on the screen's
  previous/next names, `+0x18c`/`+0x1e8` of its definition). That is why backing out of
  `Cell Selection` keeps `grid0_3_2` (the measured case): the Selector widget and the
  screen object outlive the exit, and nothing resets the slot.
- **Consequence for the build (applied 2026-10-02, see the measured bullet below)**: the persisted thing is **one Selector
  position shared by every grid**, not a per-grid memory. Entering a different grid keeps
  the same `(x, y)` when that grid has a cell there (named for the new grid) and falls to
  the default scan otherwise. `CellCursors` (`crates/game/src/main/campaign_stage.rs`)
  keyed per grid, which the measurement below falsified; it is now one shared slot.
- Whether the Selector survives leaving the campaign altogether (to `Main Menu` and back)
  depends on the screen object's lifetime and is not read here.
- **Measured 2026-10-02 (`pulse-cursor-live`, PPSSPP, two boots, a breakpoint at
  `CellSelection_Update` reading screen `+0xdc` and the cell's name at `cell+0x74`).** One shared
  slot: `grid0_3_2` then entering `grid1` lands on `grid1_3_2` (not `grid1`'s default `grid1_3_1`);
  `grid1_3_1` then entering `grid0` lands on `grid0_3_1` (not the `grid0_3_2` it was left on). The
  screen object (`0x08d73170`) is the same across a leave to `Main Menu` and back, and the cursor
  survives it. The `FUN_088a37cc(selector, 4, x, y, 4) == 0` test (layer 4 is the lock glyph, see "The cell unlock
  rule") behaves as "the new grid's tile at the slot shows no lock glyph", not as the `Locked` byte: a glyph-visible
  `grid1_2_2`/`grid0_2_2`/`grid0_2_1` at the slot gives the default scan's `_3_1`, including re-entering the same grid,
  while `grid0_2_2` (byte `+0xb9` still 1) beside a gold `grid0_3_2` keeps the cursor. Confidence 90 (shared slot,
  persistence), 85 (the glyph filter: one boot, one control). `CellCursor` in `campaign_stage.rs` now implements it
  (the screen's own lock predicate) - not the byte, which a first draft used and the gold-neighbour row refuted.

