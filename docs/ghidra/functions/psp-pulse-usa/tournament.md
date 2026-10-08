# Tournament, decompiled: the points law, the per-leg state, and what a medal compares

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, `pulse-psp-usa.chd`, UCUS-98712),
image base `0x08804000`. This closes the open item
[`race-campaign.md`](race-campaign.md#race_recordresult-0x0880ae54---where-a-result-becomes-a-medal)
left on `Race_RecordResult`'s Tournament arm: "`DAT_08b31158` was not identified and
the per-leg accumulation was not traced." It is now identified, and the
accumulation is decompiled in full. Read [`race-campaign.md`](race-campaign.md)
first for `Cell_EvaluateMedal`, `Cell_MedalPoints` (the campaign-grid medal
points, 3/2/1 - a **different** table from the one on this page) and
`CellSelection_CommitSelection`'s own Tournament branch, which this page
extends rather than repeats.

## The headline, in five parts

1. **A leg's points are a fixed table by finishing position, not the
   campaign's 3/2/1 medal points and not arithmetic on the position.**
   `g_tournament_points_by_position` (`0x08ab0ba4`) reads, for positions
   1..8: **8, 6, 5, 4, 3, 2, 1, 0.** A destroyed craft, or one in a race
   state the table's own guard reads as `7`, scores **0** regardless of
   where it finished. Confidence **88** - a direct table read. The read is
   `0x08ab0ba0 + 4 * position` with `position` 1-based, so the word at
   `0x08ab0ba0` itself is never a points entry: it is a separate global,
   `g_player_status`, written by `Race_CreatePlayer` with a pointer
   ([race-progress.md](race-progress.md)). An earlier reading had the table
   start there as an "unused index 0". No fastest-lap or elimination bonus was found anywhere in the
   per-leg accumulation path.
2. **The runtime tournament object is `DAT_08b31158`, now fully fielded**:
   `+0x74` name (hashed for the standings key, already known),
   `+0xa0` **leg count** (not "standings" as the previous pass's hedge
   framed it - see below), `+0xa4` a "this is a multiplayer tournament" bool,
   `+0xa8`..`+0xd4` twelve 4-byte leg name-hashes, `+0xdc` a byte flag whose
   exact meaning is still open.
3. **What advances a leg** is `Tournament_AdvanceLeg` (`0x0882e0e0`): on the
   state machine transitioning to `Load_Next_Race`, it increments
   `DAT_08b30fa4` and loads the next leg's track. `EndRace Menu`'s own
   `ER_NEXT_RACE` row (already documented in
   [`endrace-screens.md`](endrace-screens.md#endrace-menu)) is what reaches
   that transition, and it is offered only when
   `DAT_08b30fa4 < DAT_08b30fa0 - 1` - i.e. every leg but the last.
4. **The medal-eligible value is the final standings rank by total points,
   not the last leg's own finishing position and not the raw point total.**
   `Race_BuildEndRaceResult`'s Tournament block (mode `4`/`0x10`, previously
   flagged as untraced) accumulates each craft's `Tournament_PointsForCraft`
   result into a persistent per-craft total, sorts all crafts by that total
   (descending), and it is *that rank* which becomes `param_3` into
   `Race_RecordResult` - only on the tournament's last leg, since that
   function's own mode-4 arm is gated on
   `DAT_08b30fa4 == DAT_08b30fa0 - 1`.
5. **Tie-break is insertion order (grid slot), not a secondary criterion.**
   The standings sort is a plain bubble sort with a strict `<` swap
   condition; two crafts with equal totals are never swapped, so whichever
   was already ahead (in original grid-slot order) stays ahead. No fastest
   lap, no head-to-head leg result and no second-leg placing was found
   acting as a tie-break anywhere in this path.

## `Tournament_PointsForCraft` (`0x08826ef4`) - the points law

```
Tournament_PointsForCraft(craft, isSplitScreenSecondary):
    if DAT_08b31048 < 0xe (single-player family, modes 3..12):
        if craft->flags (+0x860) & 0x1000 (destroyed):        return 0
        if craft's race state (Ship_RaceState_q, below) == 7:  return 0
        if isSplitScreenSecondary and that state == 1:         return 0
        return g_tournament_points_by_position[craft->finishPosition - 1]   # craft+0xbd8
    else (split-screen family, modes >= 14):
        if FUN_08826b18(craft) == 0 (lap-completion check, not traced): return 0
        return g_tournament_points_by_position[craft->finishPosition - 1]
```

`craft`'s race state is read by `FUN_0883e64c` (not renamed, confidence 60):
`craft+0x8c` normally, or `Ship_RaceState`-shaped lookup through `craft+0x364`
under the split-screen flag `craft+0x860 & 0x800`. State `7`'s exact meaning
(a guess: DNF/retired) was not chased - the table read is unambiguous either
way, since state `7` always scores zero.

`craft+0xbd8` is the same finishing-position field `Race_RecordResult`'s own
`param_3` and `Cell_EvaluateMedal` already read (`race-campaign.md`), so a
craft's Tournament points for a leg are keyed off exactly the field this
project already treats as "finishing position" everywhere else.

Called from `Race_BuildEndRaceResult`'s Tournament block (below) and from
`FUN_08823ed8` (not traced this pass - a second call site, likely a
mid-race or per-leg-transition standings preview rather than the final
one).

## `DAT_08b31158` - the runtime tournament object, fielded

| Offset | Field | Evidence |
| --- | --- | --- |
| `+0x74` | name string, hashed for the standings/save key | `race-campaign.md`, unchanged |
| `+0xa0` | **leg count**, 0..12 | `Tournament_AppendLegHash`/`Tournament_AppendLegByName` both read-increment-write it; `TournamentSelection`'s own `trackCounter` widget divides by it (see below) |
| `+0xa4` | bool, "this is a multiplayer tournament" | `TournamentSelection_OnEnter`: set to `1` only when the screen's own mode string reads `"Multiplayer Tournament"` |
| `+0xa8` .. `+0xd4` | 12 x 4-byte leg name-hashes (`% 0xc` wraps, matching `PI_Cell`'s own `TournamentTrack` slot count) | `Tournament_AppendLegHash`/`Tournament_AppendLegByName`, both append at `+0xa8 + legCount*4` |
| `+0xdc` | byte flag, meaning open | Set `1` by `CellSelection_CommitSelection`'s campaign-launch path, `0` by `TournamentSelection_CommitSelection`'s and `Tournament_LoadProgress_q`'s custom-race paths - a "this is a live, in-progress tournament" reading is plausible (campaign sets it when actually starting a run; the two Racebox-side writers reset it while still *editing* the leg list) but not itself tested |

**Correction to `race-campaign.md`'s own hedge.** That page read
`CellSelection_CommitSelection`'s `DAT_08b31158->0xa0 = 0` as part of
"reset standings/leg counter," leaving open which of the two `+0xa0` was.
It is the **leg count alone** - confirmed independently by
`TournamentSelection`'s own `trackCounter` widget populate (inlined in the
un-named `InGameTrackDescriptionScreen`-shaped function at `0x088b0e84`),
which formats `"%d / %d"` from a per-screen leg cursor and
`*(DAT_08b31158 + 0xa0)` as the total - i.e. "Race 2 / 4" reads leg count
directly off this field. There is no separate standings blob at `+0xa0`;
the standings live in the profile-keyed record `Race_RecordResult` already
reads via `Libc_HashString(DAT_08b31158 + 0x74)` (`race-campaign.md`,
unchanged by this pass).

**`Tournament_AppendLegHash` (`0x088c3990`) / `Tournament_AppendLegByName`
(`0x088c392c`)**, both confidence **85**, full decompile:

```
Tournament_AppendLegHash(tournament, nameHash):
    if tournament->legCount (+0xa0) == 12: FUN_088c3910(tournament)   # not traced - a wrap/reset guard
    tournament->legs[tournament->legCount] = nameHash    # +0xa8 + legCount*4
    tournament->legCount += 1

Tournament_AppendLegByName(tournament, name):
    # identical, but hashes `name` itself first (Libc_HashString)
```

`CellSelection_CommitSelection` (campaign launch) calls the hash variant with
a `PI_Cell`'s own pre-hashed `TournamentTrack` name; `TournamentSelection_CommitSelection`
and the `Cell_Creation`-shaped populate function at `0x088e69bc` (Racebox's
own paths, below) call the by-name variant directly off a track-picker
widget's string.

**Two globals outside `DAT_08b31158` bound the leg loop, and this pass could
not find where one of them is written**:

- `DAT_08b30fa4` - the current leg index (0-based). Written by
  `Tournament_AdvanceLeg` (below); read everywhere else (`race-campaign.md`,
  `endrace-screens.md`).
- `DAT_08b30fa0` - the **session's own copy** of the total leg count, used in
  every "is this the last leg" comparison (`Race_RecordResult`,
  `Tournament_SaveProgress`, `EndRaceMenu_PopulateOptions`,
  `EndRaceResults_OnEnter`). **Its writer was not found**:
  `get_xrefs_to(0x08b30fa4 0)` returns eleven `[READ]` sites and zero
  `[WRITE]` sites. It is presumably copied from `DAT_08b31158+0xa0` once,
  at the point a tournament actually launches (as distinct from being
  edited), but that copy site is not among the functions this pass
  decompiled. **Open** - the next pass should watch `0x08b30fa0` live
  rather than search further by cross-reference, since a static miss this
  clean across two whole screens' worth of functions suggests an indirect
  write Ghidra's own analysis is not resolving.

## `Tournament_AdvanceLeg` (`0x0882e0e0`) - what moves a tournament from leg to leg

Confidence **82**, full decompile:

```
Tournament_AdvanceLeg(uiState, tournament):
    label <- "Race %d" % (DAT_08b30fa4 + 1)                 # display only
    currentLegTrack <- *(tournament + DAT_08b30fa4*4 + 0x138)   # NOT tournament's own +0xa8 array - see caveat
    copy currentLegTrack->name into a display buffer
    (base-class hook)
    if state_machine's own state name == "Load_Next_Race":
        (a one-shot guard call)
        DAT_08b30fa4 += 1
        (a second, untraced setup call)
        record a timestamp
    if state_machine's own state name == "InGame_Restart":
        (an unrelated per-lap array shift - a same-race restart, not a leg advance)
```

**Caveat, not itself resolved this pass**: `tournament + DAT_08b30fa4*4 + 0x138`
is not `DAT_08b31158`'s own leg-hash array (`+0xa8`..`+0xd4`, ending at
`+0xd8`) - `+0x138` is well past it. So `Tournament_AdvanceLeg`'s own
`param_2` is either a *different* object than `DAT_08b31158` (a resolved
per-leg `PI_Track`-pointer array, built once when a tournament actually
starts by hashing each of `DAT_08b31158`'s twelve name-hashes against the
collected track definitions) or the same object with more fields than this
pass fully mapped. **Open**: the resolution step from "leg name-hash" to
"leg's own `PI_Track` pointer" was not located, so it is unknown whether
`Tournament_AdvanceLeg` reads `DAT_08b31158` directly or a sibling table.

`Tournament_AdvanceLeg` has **no callers found by cross-reference** - the
same vtable-dispatch pattern `race-campaign.md` already established for
`CellSelection_CommitSelection` and friends (a data write into a function
table, not a call site). Given the label format (`"Race %d"`) and the
`InGame_Restart`/`Load_Next_Race` state-name tests, this reads as a
**loading-screen or track-description populate function**, most likely
`InGameTrackDescriptionScreen`'s own equivalent of `OnEnter` for a
Tournament leg - consistent with `docs/gameplay/race-modes.md`'s already
runtime-measured `InGameTrackDescriptionScreen` state name for an ordinary
race. Not confirmed by a vtable read this pass (budget went to the
`DAT_08b31158`/points chain instead).

## `Race_BuildEndRaceResult`'s Tournament block - the accumulation and the standings sort

`Race_BuildEndRaceResult` (`0x0882a498`, already named, confidence
unchanged at 78) carries a block gated on `puVar20[0x2e] == 4` (i.e.
`DAT_08b31048 == 4`, non-split-screen Tournament) that this pass opens for
the first time:

```
for each craft in the grid (0 .. DAT_08b30f90 - 1):
    teamIndex   <- craft->0x98->0x360                       # this craft's index into the persistent team-points table
    g_team_points[teamIndex].previous (+0xb0) <- g_team_points[teamIndex].total (+0x90)   # snapshot, for a "+N this leg" display
    legPoints   <- Tournament_PointsForCraft(craft, false)
    g_team_points[teamIndex].total (+0x90) += legPoints
    record this craft's name and legPoints (+0x90c) into a scratch per-craft row
    if craft == the currently-tracked race object:
        thisCraftRank <- current row index (1-based)         # provisional, before the sort below

# stable-ish bubble sort of the scratch rows by g_team_points[...].total (+0x90 via +0x118c alias), descending:
repeat:
    swapped <- false
    for each adjacent pair of rows:
        if row[i].total < row[i+1].total:      # strict less-than: an exact tie never swaps
            swap rows i and i+1
            swapped <- true
until not swapped

DAT_08b34320 rank <- 1-based index of the currently-tracked craft's row, post-sort
param_1->0x7dc <- that rank
```

`g_team_points` here is `DAT_08b34320`, a large, pre-existing global this
project has not named (it is read by over twenty unrelated functions,
including two texture loaders, so it reads as a general per-craft/per-team
persistent-state array rather than anything Tournament-specific; `+0x90`/
`+0xb0` are two of its many fields). **Its own reset point (when does a
craft's `+0x90` total go back to zero between tournaments) was not
located** - the one write site this pass found (`0x08820db0` in
`FUN_08820d78`) was not decompiled; a generic craft-init/spawn routine is
the working guess, not itself tested. Confidence for this whole block:
**75** - full decompile of the arithmetic and the sort, but the persistent
table's own identity, reset condition and exact indexing (`craft->0x98->0x360`
as "team index") are read rather than independently corroborated.

**This settles the "does the medal compare the last leg's position or the
overall standing" question.** Immediately after this block,
`Race_BuildEndRaceResult` calls `Race_RecordResult` with its `position`
argument switched from the ordinary `param_1+0x7d8` (this leg's own
finishing position) to **`param_1+0x7dc`** - the just-computed standings
rank - specifically for mode 4. `Race_RecordResult`'s own mode-4/16 arm
(`race-campaign.md`, unchanged) then runs `Cell_EvaluateMedal(cell, param_3)`
against that same value, so **a campaign Tournament cell's medal is a
threshold on final standings rank (1st/2nd/3rd = gold/silver/bronze,
same `Cell_EvaluateMedal` direction as `Race`/`Head2Head`), not on total
points and not on the last leg's own placing.** Confidence **85** - direct
decompilation of both the value computed and the value consumed, though
not runtime-verified (see below).

### Correction, 2026-09-28: the "scratch per-craft row" above is `g_endrace_result` itself, and it is one of two rows this block writes

A later pass re-decompiled this exact block (`0x0882a498`) alongside
`EndRaceResults_PopulateTournamentTable` (`0x088dad90`, below) and
`EndRaceResults_Update` (`0x088da530`, new this pass) to close the "copy
into `g_endrace_result`'s own `+0x35`/`+0x134`... not located" item this
page's own "What is not determined" section carried since the pass above.
It is located, and the pseudocode above undersells what the block does:
**it writes two separate per-craft rows, both inside `g_endrace_result`
(`param_1+0x7d8`, `endrace-screens.md`'s own base), not one scratch row
outside it:**

```
iVar13 = param_1 + 0x80d          # g_endrace_result + 0x35 (name, row n at +n*0x110)
iVar9  = param_1 + 0x108d         # g_endrace_result + 0x8b5 (name, row n at +n*0x110)
iVar15 = param_1                  # g_endrace_result + 0    (row n at +n*0x110)
for each craft in the grid (0 .. DAT_08b30f90 - 1), in `param_1+0x98`'s own order:
    slot        <- craft->0x360
    g_team_points[slot].previous (+0xb0) <- g_team_points[slot].total (+0x90)   # snapshot before this leg
    legPoints   <- Tournament_PointsForCraft(craft, false)
    g_team_points[slot].total (+0x90) += legPoints
    *iVar13 (+0x35  this row) <- craft's own name string           # ER_TEAM/PRO_NAME column, table A
    *(iVar15 + 0x90c) (+0x134 this row) <- legPoints                # ER_POINTS column, table A - THIS LEG's points, not the running total
    *iVar9  (+0x8b5 this row) <- craft's own name string           # table B, same name, second copy
    *(iVar15 + 0x118c) (+0x9b4 this row) <- g_team_points[slot].total (+0x90, post-increment)  # table B - the CUMULATIVE total
    if craft == the currently-tracked race object:
        thisCraftRank <- current row index (1-based)                # provisional, before table B's own sort below
    iVar13 += 0x110; iVar9 += 0x110; iVar15 += 0x110
```

Then the bubble sort this section already documented (line 207 above) sorts
**table B only** (the `+0x8b5`/`+0x9b4` rows, via the `+0x118c` alias) by its
own now-cumulative `+0x9b4` field, descending, stable on a tie. **Table A
(`+0x35`/`+0x134`) is never sorted by this function at all** - its own row
order is simply whatever order `param_1+0x98`'s craft-pointer array already
holds.

This is the same struct, and the same two field pairs,
`EndRaceResults_PopulateTournamentTable` (below) reads: table A is its
`param_2 == 0` argument (`ER_RACE_STAN`, "this leg's own placings"), table B
is `param_2 == 1` (`ER_TOUR_STAN`, "the tournament standings"). **The
previous pass's own reading of `+0x9b4` as a split-screen variant of
`+0x134` (this page's line 303 as it stood before this correction, and
`endrace-screens.md`'s mirror of it) is wrong** - split-screen is a
*different* field, `screen+0xe9` (`0xd < g_game_mode`), which only ever
picks the header idstring (`ER_TEAM` vs `PRO_NAME`) and gates the
`ER_DNF`/`ER_RACING` substitution, and is `false` for Tournament's own mode
`4` regardless of which of these two tables is showing. `+0x9b4` is real,
and it is the **standings** table's own points column, not a split-screen
echo of the leg table's.

**`EndRaceResults_Update` (`0x088da530`, confidence 85 - full decompile, not
independently live-verified) is `EndRace Results`' own per-frame handler,
and for Tournament (`g_game_mode` 4 or 16) it is what actually chooses
between table A and table B: a three-second timer
(`screen+0xdc`/`screen+0xe0`) that alternates
`EndRaceResults_PopulateTournamentTable(screen, 0)` (`Line1` ->
`ER_RACE_STAN`) with `EndRaceResults_PopulateTournamentTable(screen, 1)`
(`Line1` -> `ER_TOUR_STAN`), starting on table A the instant the screen is
entered** (`EndRaceResults_OnEnter`'s own `case 4: case 0x10:` populates
table A once immediately, then sets the same timer/flag pair so the first
toggle - three seconds later - is to table B). **This runs on every
Tournament leg's own `EndRace Results`, not only the last** - the gate is
`g_game_mode`, not the leg index; only `BigTopText` (`ER_END_TOUR` on the
last leg, `"%s %d/%d"` of `ER_RES` and the leg counter otherwise, read off
`DAT_08b30f90+0x10`/`+0x14` the same function reads for the last-leg
check on line 216 above) differs between a mid-tournament leg and the
final one. The same handler also drives the unrelated "1ST/2ND/3RD PLACE"
fanfare line for the `Race`/`Head2Head`/`Time Trial`/`Speed Lap` family
(`g_game_mode` 3 or 8) - a second, independent gate in the same function,
not touched by this correction.

**What this closes, from "What is not determined" below:** the copy into
`g_endrace_result`'s `+0x35`/`+0x134` fields (table A, closed - it is this
block, written unsorted). **What stays open:** the exact writer/order of
`param_1+0x98`'s own craft-pointer array, which fixes table A's own row
order (and therefore what `PRO_POS` reads on that page) - not traced this
pass either, though the fact that a leg's points column reads `8, 6, 5,
4, ...` top-to-bottom on every capture this pass took is strong behavioural
evidence it is finish order, not grid-slot order; see "What is not
determined" for the caveat.

## `Tournament_SaveProgress` (`0x0880b488`) / `Tournament_LoadProgress_q` (`0x088ec350`)

The mechanism behind `MSC_EVENT_TOURN`'s "you can also save your tournament
progress between races" and the `MSC_MSG_AUTOSAVE3` dialog
(`race-campaign.md`), both sides now decompiled.

**`Tournament_SaveProgress`**, confidence **76**: called from
`Race_BuildEndRaceResult` immediately after the standings block, but only
when the tournament is *not* on its last leg (`DAT_08b30fa4 == DAT_08b30fa0-1`
takes the *other* branch, which instead calls `FUN_08809a88` - an ordinary
end-of-tournament path, not traced). On every other leg it builds a
0xa0-byte snapshot (every grid slot's team-name hash and craft-state
pointer, the current leg index at `+4`, the tournament's own `Ranked`/team
bookkeeping) and writes it via the same generic name-hashed record store
(`FUN_088085d0`) the campaign's own per-cell saves use, keyed under one of
two literal strings - `"TournamentRC"` for the primary racer, `"TournamentRB"`
for a second, split-screen one (guarded by `craft->0xb4`, the same custom-grid
byte `race-campaign.md` already reads elsewhere) - into `DAT_08b31774`, the
same per-team/profile store `Unlock_LoyaltyMet` and `Race_ComputeLoyaltyAward`
already read (`endrace-screens.md`).

**`Tournament_LoadProgress_q`**, confidence **65** (`_q` per this project's
naming rule): looks up that same record by a hash of `DAT_08b30fb4`, and on
a hit restores `DAT_08b30fa4` from the saved leg index (`local_38[1]`),
re-runs the same `Globals_Set` sequence `CellSelection_CommitSelection`
uses (`Mode`, `Class`, `Damage`, `Opponents`, `Weapons`, `SkillLevel`), resets
and re-appends `DAT_08b31158`'s own leg list from the saved per-leg name
hashes, and sets `Globals_Set("Tournament", "yourTourney")` - the exact
resume-a-tournament counterpart to `CellSelection_CommitSelection`'s
start-a-tournament write. Not positionally confirmed against a screen's own
vtable this pass (no state-name string comparisons appear in its own body
the way `Tournament_AdvanceLeg`'s do), so its exact trigger (a `TournamentLoad`
screen's `OnEnter`, most likely, given the literal string `"TournamentLoad"`
appears as a *sibling* check inside the unrelated `0x088e69bc` populate
function below) is inferred rather than read.

## `EndRaceResults_PopulateTournamentTable` (`0x088dad90`) - the standings table

Confidence **82**, full decompile plus disassembly-level confirmation of the
format strings (`FUN_08972550` calls resolved against `read_memory`, since
the decompiler's own typing hid a `%s`/`%d` distinction on first read).
Dispatched from `EndRaceResults_OnEnter`'s mode-4/16 case, which sets the
results screen's `BigTopText` to `ER_END_TOUR` on the last leg or an
`"%s %d/%d"` leg counter otherwise, and its `Line1` header to a **new**
idstring, `ER_RACE_STAN` (English text not extracted this pass - distinct
from `ER_TOUR_STAN`, "Tournament standings", which this and the previous
pass both cite from `race-setup.md` but whose own consumer is still
unfound; the two may be the same rendered text or two different screens,
not settled here).

Per grid slot (`DAT_08b30f90` rows), three columns, in order:

| Column | Header idstring | Source | Format |
| --- | --- | --- | --- |
| `lap{n}.0` | `PRO_POS` | the loop's own row index (i.e. **the table is pre-sorted on entry** - this function does not sort) | `%d` |
| `lap{n}.1` | `ER_TEAM` (single-player) / `PRO_NAME` (split-screen) | an inline name string at `g_endrace_result + craft*0x110 + 0x35` | `%s` |
| `lap{n}.2` | `ER_POINTS` | a 4-byte int at `g_endrace_result + craft*0x110 + 0x134` (`+0x9b4` split-screen), unless that craft's own `+0x140` reads negative, in which case `ER_DNF`/`ER_RACING` is shown instead | `%d` |

`g_endrace_result` is the same `DAT_08b317b4 + 0x7d8` base `endrace-screens.md`
already names; `EndRaceResults_OnEnter` sets `param_1+0xe4` to exactly that
address on entry, confirmed by decompiling it this pass (`race-campaign.md`
and `endrace-screens.md` had only inferred the alias). **The table itself is
not sorted by this function** - whatever populates it (not located this
pass; the standings-sort in `Race_BuildEndRaceResult` operates on a
*different* scratch array, at `param_1+0x108d`/`+0x118c`, not on
`g_endrace_result` directly) must already write rows in rank order before
`EndRaceResults_OnEnter` runs. **Open**: the copy from
`Race_BuildEndRaceResult`'s own sorted scratch rows into
`g_endrace_result`'s `+0x35`/`+0x134` fields was not located.

> **Correction, 2026-09-28** (see `Race_BuildEndRaceResult`'s own section
> above, "the scratch per-craft row above is `g_endrace_result` itself"):
> every claim in this section's table and prose above is confirmed
> *except* two. First, `+0x9b4` is **not** a split-screen variant of
> `+0x134` - it is the **standings** table's own points column (`param_2 ==
> 1`, cumulative total), where `+0x134` is the **leg** table's (`param_2 ==
> 0`, this leg's own points); `param_2` is the argument that picks the
> table, a totally different axis from the `ER_TEAM`/`PRO_NAME`/`ER_DNF`
> split-screen flag (`screen+0xe9`) this section's own table correctly
> names for the *name* column, just mislabelled onto the *points* column
> too. Second, the "copy... not located" line is now closed: it is
> `Race_BuildEndRaceResult`'s own tournament block, which writes both
> tables (unsorted `+0x35`/`+0x134` in `param_1+0x98`'s own order; sorted
> `+0x8b5`/`+0x9b4`, by the bubble sort this page already had). The
> "`param_1+0x108d`/`+0x118c`... a *different* scratch array, not on
> `g_endrace_result` directly" line above is also corrected by the same
> finding: `param_1+0x108d` **is** `g_endrace_result+0x8b5` (`param_1 ==
> g_endrace_result - 0x7d8`), not a separate array - see the corrected
> pseudocode. `ER_TOUR_STAN`'s own consumer, left "unfound" above, is this
> function's `param_2 == 1` call from `EndRaceResults_Update`
> (`0x088da530`, new this pass) - see that section for the three-second
> toggle between the two.

## Custom Race's own `Tournament C` writes the identical `DAT_08b31158` the campaign does

This was the open question a runtime verification pass most needed settled
before spending a live session on it: **does `Racebox`'s own `Tournament C`
(`TournamentSelection`, `race-setup.md`'s redirect target for
`Mode == Tournament`) exercise the same law as the campaign, or an adjacent
one that only looks similar?** It is the same one. `TournamentSelection`'s
vtable (`0x08ad0c44`, positionally read the same way `race-campaign.md`
reads every other screen's) puts `TournamentSelection_OnEnter` at word 29
and `TournamentSelection_CommitSelection` at word 39:

- **`TournamentSelection_CommitSelection`** (`0x088ecb54`, confidence
  **85**): resets `DAT_08b31158`'s `+0xa0`/`+0xdc` to `0`, then walks
  **twelve** `Track_%d` list widgets (`0x1`..`0xc` - the PS2's own slot
  count, per `race-setup.md`'s "twelve on PS2" note; the PSP screen simply
  leaves the extra widgets on `MSC_NONE` and this loop skips them) and
  appends each non-`MSC_NONE` entry via `Tournament_AppendLegByName` -
  **exactly** the mechanism `CellSelection_CommitSelection`'s Tournament
  branch uses for a campaign cell.
- **`TournamentSelection_OnEnter`** (`0x088eca0c`, confidence **85**): the
  function that sets `DAT_08b31158+0xa4` (the "multiplayer tournament" bool)
  and, for a single-player entry, resets the same two fields again before
  the picker is shown.

So a custom Racebox tournament and a campaign Tournament cell write the
*same* global, through the *same* two append functions. **One real
divergence, confirmed live (below) rather than assumed**:
`Race_BuildEndRaceResult`'s Tournament block (the points accumulation and
standings sort) runs unconditionally, but `Race_RecordResult`'s own
`Cell_EvaluateMedal` step - the part that turns a standings rank into a
gold/silver/bronze - is gated on `DAT_08b30ffc != 0` (`race-campaign.md`,
"For every mode with a campaign cell in play"). A Custom Race Tournament
has no cell (`DAT_08b30ffc == 0` throughout), so **it exercises the points
table and the standings sort faithfully, but never reaches the medal step**
- that half needs an actual campaign cell, which the previous pass found
blocked behind `grid1`'s own lock. Confidence **88** for the points/standings
equivalence specifically (two independently-positioned vtable slots, on two
different screens, calling the two sibling append functions with the same
reset pattern); the medal step's equivalence is untested by construction,
not just unverified.

## Head2Head - not reached this pass

**Closed since: see [head2head.md](head2head.md) (2026-09-29, and its 2026-10-08 section for `Race_RecordResult`'s arm).** What follows is the original note.

Deliverable 4 (Head2Head) was not attempted: the points/standings chain
above was the higher-value target given the assignment's own priority
order, and what remained of this pass's budget went to durability (this
write-up) and the live verification below rather than a second mode. See
[Next steps](#next-steps).

## What is not determined

- **`DAT_08b30fa0`'s write site.** Narrowed live this pass to the window
  between `Team Selection`'s own confirm and `Launch Game` reaching
  `InGame` (see "Live verification" below) - still not pinned to a single
  function.
- **`DAT_08b31158+0xdc`'s exact meaning.** A "live tournament in progress"
  reading fits every write site seen, but nothing reads the flag back in
  any function this pass decompiled, so the reading is unconfirmed.
- **The leg-name-hash to `PI_Track`-pointer resolution** `Tournament_AdvanceLeg`
  appears to consume from `param_2+0x138` - a different array from
  `DAT_08b31158`'s own `+0xa8` hash slots.
- **`DAT_08b34320`'s own identity, and the reset of its `+0x90` field**
  between tournaments - the persistent points-total array's write site
  (`FUN_08820d78`) was found but not decompiled.
- ~~The copy into `g_endrace_result`'s own `+0x35`/`+0x134` per-craft
  fields...~~ **Closed 2026-09-28**: `Race_BuildEndRaceResult`'s own
  tournament block writes it, unsorted, in `param_1+0x98`'s own order -
  see that section's own dated correction.
- **`param_1+0x98`'s own craft-pointer array: what order it holds, and who
  writes it.** New this pass, and the one piece the correction above did
  not close: `Race_BuildEndRaceResult`'s tournament block walks it to fill
  the leg table (`g_endrace_result+0x35`/`+0x134`), and that walk is never
  sorted by anything this pass decompiled, so the leg table's own `PRO_POS`
  is exactly that array's own order. Every `--menu-page
  endrace-results-tournament-leg` capture this pass took shows points
  strictly decreasing top-to-bottom (`8, 6, 5, 4...`), which only happens
  if the array is already in finish-position order - behavioural evidence,
  not a traced writer. If it is finish order (not grid-slot order), a
  drawing pass should order the leg table by this leg's own finishing
  place, not by `oag_race::tournament::Standings`' own rank - flagged for
  whoever draws this, not answered here.
- **`Tournament_LoadProgress_q`'s own trigger** (which screen's `OnEnter`
  calls it) was not positionally confirmed.
- **Head2Head** entirely - see above.
- **Only the USA pressing was read.** Per this project's EU-preference
  ([ADR-0048](../../../architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md)),
  none of the addresses on this page have been cross-checked against
  `psp-pulse-eu`.

## Live verification

PPSSPP v1.20.4 (SDL build), `pulse-psp-usa.chd`, Xvfb `:97`, debugger on
`ws://127.0.0.1:47810/debugger`, fresh profile (first-boot dialogs answered
live). Screenshots under `data/reference/psp-tournament/` (gitignored, not
committed).

**Reached and confirmed live, this pass**: `Racebox -> Single Player`,
`RACE TYPE` cycled to `TOURNAMENT` (screenshotted), confirmed into
`Tournament C` - the exact screen the redirect in `race-setup.md` predicts,
by state name. `Tournament C` shows **twelve** `Track %d` slots, all
`None` (matching the "twelve on PS2, four on PSP-authored-cells" note -
the *screen* always offers twelve regardless of platform; a `PI_Cell`'s own
authored slot count is a different, smaller number). Picked "Talon's
Junction White" for slots 1 and 2 (screenshotted both selections), confirmed
into `Team Selection` (an existing Assegai save with `Loyalty 90` from a
prior pass's own race), confirmed into `InGame`.

**`DAT_08b31158` read live, immediately after `Tournament C`'s own
confirm** (`TournamentSelection_CommitSelection`), via `memory.read_u32`/
`read_u8` on the tournament pointer (`0x08c0e070` this session):

| Field | Value | Matches |
| --- | --- | --- |
| `+0xa0` (leg count) | `2` | the two slots picked |
| `+0xa4` (multiplayer bool) | `0` | single-player Custom Race |
| `+0xa8` (leg 0 hash) | `0x51f3a6e9` | |
| `+0xac` (leg 1 hash) | `0x51f3a6e9` | identical to leg 0 - same track picked twice, as intended |
| `+0xb0` (leg 2 hash) | `0x0` | unused slot, confirming the count really is 2 |
| `+0xdc` | `0` | matches the decompiled `TournamentSelection_CommitSelection`, corroborating the "Racebox editing path writes 0" reading above |

This is the first time any pass has read `DAT_08b31158` live - the
previous campaign pass's own fifth breakpoint was blocked entirely (no
reachable Tournament cell in that profile). Confidence for the struct
layout above: raised to **92** for `+0xa0`/`+0xa8`/`+0xac` specifically
(static decompile plus an exact live match on a value this pass chose and
could predict in advance - the strongest form of corroboration this
project's rubric recognises).

**`DAT_08b30fa0` (the still-unlocated "session leg count copy") was caught
live, closing part of that open item.** Read `0` immediately after
`Tournament C`'s confirm (Team Selection screen), and `2` once `InGame` was
reached (after Team Selection's own confirm) - so **the write happens
somewhere in the Team-Selection-confirm-to-Launch-Game window**, not at
`Tournament C`'s own commit. The exact function is still not identified
(this pass did not have a free Ghidra-bridge budget left to binary-search
that window with more checkpoints), but the window itself is new
information - narrower than "not found by cross-reference" alone.
`DAT_08b30fa4` (leg index) read `0` at both checkpoints, as expected before
any leg has finished.

**`DAT_08b30ffc` (the campaign-cell pointer) read `0x0` throughout** -
confirming a Custom Race Tournament never sets it, and therefore never
reaches `Race_RecordResult`'s `Cell_EvaluateMedal` step (see the caveat
added to the previous section).

**Not completed this pass: driving a leg to a finish.** `scripts/psp-autopilot.py`
against this track's own spline (`oag-trace track`, `16_Track` - the tool
picked the same circuit by default) tracked the racing line poorly on
Talon's Junction's own branch (`2 path(s), 2 junction(s)` per `oag-trace`'s
own log) - 5,700 ticks covered only 0.234 laps, roughly ten times slower
than this project's own "race ends around tick 7,500" baseline for a clean
3-lap Venom run, with repeated wall contact visible in the `off`-from-line
column. Finishing three laps at that rate, times two legs, was not a
"modest effort" any more (the same phrase the previous pass's own deferred
item used) - extrapolated real time was on the order of an hour, for a
result (the points table and the medal-comparison value) already
established at high confidence from clean decompilation. **This is a
time-boxed, honest stop, not a blocked one**: the points write and the
`EndRace Results` Tournament standings table were not captured live, and
whoever picks this up next should either retune the autopilot's steering
for a junction-laden track or pick a simpler circuit (`Venom Straight` has
a committed reference trace already) when re-attempting.

## Live capture of the drawn standings table (PPSSPP v1.20.4, 2026-09-29)

`pulse-psp-usa.chd`, Xvfb, own profile. Racebox `Tournament` -> `Tournament C`
with **one** slot (Talon's Junction White) -> `Team Selection` (Assegai) ->
`InGame` (a full grid of eight, the player 8th). The leg was ended by writing
`g_race_laps` (`0x08b30f9c`) = 1 at run time and holding thrust across the
start line, so the first crossing finished it - a **shortcut to reach the
screen, not a race**: the player finished 8th of 8, which is exactly the
point of interest (the law gives 8th place 0). Flow after the finish:
`EndRace Rewards` (`Tournament complete - 8th place`, `Assegai Loyalty: 0
Points`) -> `EndRace Menu` (`RETURN TO MAIN MENU`, `VIEW RESULTS AGAIN`, no
`RACE AGAIN`/`NEXT RACE` on the last leg) -> `EndRace Results` via `VIEW
RESULTS AGAIN`. Frames: `data/reference/psp-tournament/er1.png`, `er2.png`,
`er4.png`, `er7.png`, `er10.png` (a 16-frame burst at 0.6 s), `fz1.png`
(Rewards), `em1.png` (Menu), and `cmp_tournament_orig_vs_ours_synthetic.png`.

| Claim | Result | Conf |
| --- | --- | --- |
| Points by finish place are 8/6/5/4/3/2/1/0 | **confirmed**: rows read Feisar 8, EG-X 6, Triakis 5, Piranha 4, AG Systems 3, Goteki 45 2, Qirex 1, Assegai 0 - the eight-craft table, with the player (last) on 0 | 92 |
| Columns `PRO_POS`/`ER_TEAM`/`ER_POINTS`, header `Pos`/`Team`/`Points` | **confirmed** | 92 |
| Layout: columns at x = 280/440/640 (960x544), header y 168, rows at y = 206 + 40n | **confirmed**: ours (synthetic field) lands on the same x and y | 90 |
| Two pages cycling, subtitle `ER_RACE_STAN` / `ER_TOUR_STAN` | **confirmed**: the subtitle alternates `Race Standings` / `Tournament Standings` with a fade, and the rows are identical on a one-leg tournament | 88 |
| Cycle period 3 s | **consistent** (subtitle flips about every 5 frames of a 0.6 s burst, so 3 s; not timed to better than 0.3 s) | 70 |
| `BigTopText` = `ER_END_TOUR` on the last leg | **confirmed**: `End of Tournament` | 90 |
| Player's row highlighted (`tablehighlight`) | **confirmed**, blue, over the player's row (last here) | 90 |
| Row order = descending points, both pages | **confirmed for a one-leg field** (both pages are the same order). The leg page's finish-order reading and the standings page's rank order cannot be told apart by one leg | 75 |
| Backdrop is the race scene, darkened | **observed**: the running race behind the table, not this project's plain backdrop | 90 |

**Differences from our draw, found by the comparison** (not changed here):

- ~~Team cells show the team's **display name**~~ **Fixed 2026-09-30**: the roster's folder
  ids (`AG_Systems`) are resolved through the string table at draw time
  (`oag_ui_screens::endrace`'s cell text, shared with the new Eliminator table), which is the
  original's own `localise(craft+0x798)`; a live Eliminator run showed `AG SYSTEMS`,
  `EG-X` and `GOTEKI 45` on this build's own screen. It was not the catalogue the fix
  needed after all - the string table already carries every team under its folder id.
- Every table cell and header draws in the mixed-case `Default` font. Ours
  uses the upper-case front-end font, exactly as it already does on the
  ordinary per-lap `EndRace Results` table (`results-01.png` shows the same
  difference), so this is the shared table's font, not a tournament fault.
- Row background bars and the race backdrop: the original draws translucent
  dark bars over the live scene.

**Still open:** a two-leg tournament (which page order the running totals
give, the `NEXT RACE` menu row, the leg counter `Results n/m`); a
non-8th finish; the `param_1+0x98` row order.

## Names recovered

| Address | Name | Conf |
| --- | --- | ---: |
| `0x08826ef4` | `Tournament_PointsForCraft` | 85 |
| `0x08ab0ba4` | `g_tournament_points_by_position` (data) | 88 |
| `0x0882e0e0` | `Tournament_AdvanceLeg` | 82 |
| `0x088c3990` | `Tournament_AppendLegHash` | 85 |
| `0x088c392c` | `Tournament_AppendLegByName` | 85 |
| `0x088ecb54` | `TournamentSelection_CommitSelection` | 85 |
| `0x088eca0c` | `TournamentSelection_OnEnter` | 85 |
| `0x088ed428` | `TournamentSelection_Construct` | 78 |
| `0x0880b488` | `Tournament_SaveProgress` | 76 |
| `0x088ec350` | `Tournament_LoadProgress_q` | 65 |
| `0x088dad90` | `EndRaceResults_PopulateTournamentTable` | 82 |
| `0x088da530` | `EndRaceResults_Update` | 85 |

`FUN_0883e64c` (craft race-state accessor, confidence 60), `FUN_08826b18`
(split-screen lap-completion check, confidence 55), `FUN_08823dbc` (a
leg-transition function reading `DAT_08b31158`/`DAT_08b30fa4`/`DAT_08b30fa0`
together, purpose not settled) and `FUN_08820d78` (the one write site for
`DAT_08b34320`) are all below this project's 70-confidence naming floor or
not decompiled at all, and are left as `FUN_` addresses with the hypotheses
stated inline above rather than renamed.

## Next steps

1. Watch `0x08b30fa0` live in the narrowed `Team Selection confirm ->
   InGame` window (see "Live verification") to pin its exact writer.
2. Decompile `FUN_08820d78` to confirm `DAT_08b34320+0x90`'s reset
   condition (does a fresh tournament actually zero it, or does it carry
   over from an unrelated prior race).
3. ~~Locate the copy into `g_endrace_result`'s own per-craft `+0x35`/`+0x134`
   fields...~~ **Done 2026-09-28** - `Race_BuildEndRaceResult`'s own
   tournament block. What is left of this item: trace `param_1+0x98`'s own
   writer, to confirm the leg table's own row order is finish order (see
   "What is not determined").
4. Head2Head's own `Race_RecordResult` arm and launch globals - deliverable
   4, not attempted this pass.
5. Cross-check this page's addresses against `psp-pulse-eu`.
6. **Draw the tournament standings table for real** - `oag_ui_screens::endrace`
   now has both pages (`crates/ui-screens/src/endrace.rs`'s `TournamentResults`),
   fed by `oag_race::tournament`/`crate::race::tournament::Progress`
   (`crates/game/src/main/race_stage/endrace.rs`'s `tournament_results`).
   Not yet checked against a live PPSSPP capture of a real tournament leg
   ending - see `docs/ui/endrace-screens.md`'s own Open section for why a
   live capture through to a finished leg was not attempted this pass
   either (same autopilot cost the "Live verification" section below
   names).
