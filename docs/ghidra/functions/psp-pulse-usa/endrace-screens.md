# `EndRace Results`, `EndRace Rewards`, `EndRace Menu`, decompiled

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, `pulse-psp-usa.chd`, UCUS-98712),
image base `0x08804000`. This is the code half of
[`docs/formats/endrace-screens.md`](../../../formats/endrace-screens.md), which
reads the XML these functions fill in; that page has the widget tables, this
one has the addresses and the evidence. **Nothing here is runtime-verified**
beyond the single capture `docs/ui/campaign-screens.md` already has (a
`grid0_3_2` Time Trial, no medal) - every score below is capped accordingly,
and the breakpoint that would raise it is named at each open item.

## The registration pattern, again

All three screens register the identical way
[`race-box-screens.md`](race-box-screens.md) already documents for
`TrackSelection`/`TeamSelection`: a global-constructor-style function with no
`get_xrefs_to` callers, that inserts the class into the name-sorted registry
(`ScreenClass_Register`, `0x0888fc68`), stores the vtable pointer directly, and
prepends to the generic linked list (`FUN_08971afc`).

| Address | Name | Conf | Vtable |
| --- | --- | --- | --- |
| `0x088db968` | `EndRaceResults_Construct` | 78 | `0x08acfde4` |
| `0x088dd89c` | `EndRaceRewards_Construct` | 78 | `0x08acfe84` |
| `0x088d90dc` | `EndRaceMenu_Construct` | 78 | `0x08acfd44` |

`read_memory` on all three vtables (256 bytes each) confirms the same layout
`race-box-screens.md`/`race-campaign.md` already established: word 9 is
`Update`, word 29 is `OnEnter`, word 31 is `OnExit`, word 39 is
`CommitSelection` - and on all three of these screens, word 39 is
`0x088902c8`, the same **shared base-class no-op** `CellSelection`/
`GridSelection`'s own `CommitSelection` call at the top of their bodies
(`race-campaign.md`). None of the three EndRace screens override
`CommitSelection` - consistent with none of them committing a "selection" the
way a hex-grid screen does; their own confirm path is the generic redirect
mechanism below instead.

| | `EndRace Results` | `EndRace Rewards` | `EndRace Menu` |
| --- | --- | --- | --- |
| `Update` (word 9) | `0x088da530` (not decompiled this pass) | `0x088dd2b8` | `0x088d8338` (not decompiled this pass) |
| `OnEnter` (word 29) | `0x088d98cc` | `0x088dbbd4` | `0x088d81dc` |
| `OnExit` (word 31) | `0x088d92e0` (not decompiled this pass) | `0x088dbb5c` (not decompiled this pass) | `0x088d8248` (not decompiled this pass) |
| `CommitSelection` (word 39) | `0x088902c8` (shared base, not overridden) | `0x088902c8` (shared base) | `0x088902c8` (shared base) |

## `EndRace Results`

| Address | Name | Conf | Role |
| --- | --- | --- | --- |
| `0x088d98cc` | `EndRaceResults_OnEnter` | 82 | sets `BigTopText`/`Line1`, dispatches to the per-mode populate helper, toggles the two outgoing Redirects |
| `0x088da8c8` | `EndRaceResults_PopulateLapTable` | 80 | the single-player (`Race`/`Time Trial`/`Head2Head`/`Speed Lap`) per-lap table |
| `0x088d939c` | `EndRaceResults_ResetTable` | 76 | blanks all 27 `lap{r}.{c}` cells, unhides the 8 `tablebg` rows and the 5 `perfectlap` icons, re-hides `boostimg` and `topbarcenter`'s prior state |

`EndRaceResults_OnEnter` switches on a mode-shaped value read two ways in the
same function (`local_48[0x2e]`, i.e. `*(int*)(DAT_08b30f90 + 0xb8)`, and
`DAT_08b31048` directly) into the same seven groups `race-campaign.md`'s own
`Race_RecordResult` table already uses: `{3,5,9,10,0xe,0xf,0x11}` (the
single-race family, including split-screen variants `14`/`15`/`17`),
`{4,0x10}` (Tournament), `6` (Zone), `{8,0x12}` (Elimination). This is
independent corroboration, from a second decompiled function, of that
table's own enum grouping.

For the single-race family, `Line1` narrows in two steps:

1. `*(char*)(g_endrace_result + 0x1165) == 0` -> `ER_SHIP_DES`; otherwise a
   finishing-position idstring (`ER_1STP`..`ER_8THP`) off
   `*(int*)g_endrace_result`.
2. Then, only for Time Trial (`5`/`0x11`) -> overwritten again to `ER_TT_COM`;
   only for Speed Lap (`10`) -> `ER_SL_COM`.

`g_endrace_result` here (and throughout this page) names
`*(int*)(DAT_08b317b4 + 0x7d8)` - a fixed-offset sub-structure off a much
larger, already-in-use global (`DAT_08b317b4`, over fifty cross-references
across unrelated subsystems - not renamed here, out of scope). Every field
this page cites off it is a byte offset into that one sub-structure, not a
new global of its own.

`EndRaceResults_PopulateLapTable`'s own per-lap record is 8 bytes, indexed by
lap number (1-based, up to 5 shown):

| Offset (within one lap's 8 bytes) | Width | Bound widget | Format |
| --- | --- | --- | --- |
| `+0x13` | byte | `lap{n}.0` | decimal |
| `+0xc` | 4 bytes | `lap{n}.1` | the centisecond time formatter `race-campaign.md` already names, `FUN_088196e4` |
| `+0x10` | 2 bytes | `lap{n}.2` | decimal - meaning not determined, see the formats page |
| `+0x12` | byte (flag) | hides `perfectlap{n}` when nonzero | - |

The totals row (`tablebg{n+1}`, `n` = laps shown) reads
`g_endrace_result + 0x114c` (a time) and `g_endrace_result + 0x1148` (a
separately-tracked aggregate, not summed client-side) under the `PRO_STATS_TOT`
label - confirmed by the aggregate reproducing the capture's own `25` from
`6+9+10` exactly.

`boostimg` is hidden unconditionally by both `EndRaceResults_ResetTable`
(which un-hides it first, matching the XML's own baseline-visible authoring)
and then immediately re-hidden by `EndRaceResults_PopulateLapTable` before
its per-lap loop runs, for every mode this function handles. Confidence 78 -
full decompile of both functions, the double un-hide-then-hide read directly
off the instructions in that order.

## `EndRace Rewards`

| Address | Name | Conf | Role |
| --- | --- | --- | --- |
| `0x088dbbd4` | `EndRaceRewards_OnEnter` | 85 | picks the medal idstring and trophy, seeds the loyalty-reason ticker's string array |
| `0x088dd2b8` | `EndRaceRewards_Update` | 85 | drives the loyalty ticker, fills `RewardLoyaltyActive`/`loyaltynum`/`loyaltybar`, fires the outgoing Redirect once the ticker finishes |

### The medal ordinal and the trophy select

`EndRaceRewards_OnEnter` reads `*(int*)(g_endrace_result + 0x1160)` -
`0 = gold, 1 = silver, 2 = bronze, else (including the `<0` a signed-byte
read of the `0xff` sentinel produces) = none` - the **same ordinal**
`Cell_EvaluateMedal` (`race-campaign.md`, `0x088bf620`) already established.
That value:

```
RewardLine1 <- ER_GMA / ER_SMA / ER_BMA / ER_NMA   (unconditional)
if DAT_08b30ffc != 0 (a campaign cell in play):     # race-campaign.md's own campaign-cell pointer
    unpause exactly one of g_trophy/s_trophy/b_trophy (Data\FE\trophies\*.vex)
    play the matching jingle (gold_med / silver_med / bronze_med)
else:
    hide MedalImg unconditionally; no trophy is resolved at all
```

All three trophies start with their own inner-node flag cleared at setup
(`&= ~4`), then exactly one gets it set (`|= 4`) inside the ordinal switch -
the trophies are `StartPaused="yes"` in the XML, so this reads as an
unpause-the-correct-one pattern rather than a hide/show one. `MedalImg`
itself (`Data\FE\Images\reward_icons.mip`, a single fixed 32x32 icon, not
re-sourced to a different sub-rect anywhere in this function) is left at its
baseline-visible state in every gold/silver/bronze branch, and is only
explicitly re-hidden in the no-medal branch. Confidence 80 for the
mechanism; 55 for "MedalImg is the no-medal glyph specifically, and stays
hidden under an earned trophy" - the one capture this pass has is itself a
no-medal run, so it cannot show whether a medal-earning run leaves `MedalImg`
visible underneath the trophy by some code this pass did not reach.

### The loyalty ticker: where `90 Points` and `Total loyalty` actually come from

`EndRaceRewards_Update` runs every tick; once its own cycle index equals its
reason-string count (the ticker's first "beat", before any reason string has
shown):

```
RewardLoyaltyActive <- "%d %s" of *(g_endrace_result + 8), ER_POINTS       # "90 Points"
record <- FUN_08808664(DAT_08b31774, Libc_HashString(team_name), 0, 0)     # same accessor race-box-screens.md
                                                                            # already names for the per-craft rating table
loyaltynum  <- "%s %d" of ER_TOT_LOY, *(record + 8)                       # "Total loyalty: <team's running total>"
loyaltybar fill <- *(record + 8) * 0.00124
```

Confidence 85 - full decompile, and the field `*(g_endrace_result + 8)`
reproduces the capture's `90 Points` exactly under the hypothesis below.
**Every later "beat"** (roughly every 10 accumulated update-timer units,
`elapsed += frameDelta * 20.0` against a fixed threshold of `10.0`) instead
advances to the next reason string from the array `OnEnter` built - see the
formats page for the full idstring table (`MSC_LOY_LAP{15,30,10}` etc.) - and
the screen's own outgoing Redirect (`EndRaceRewardsMenuRedirect`) is enabled
once total elapsed ticker time passes `reasonCount * 0.5 + 1.5` seconds.

**This closes an open item on `Unlock_LoyaltyMet`** (`race-campaign.md`,
`0x0888ea30`, previously scored 72 with the note "compares against a whole
32-bit word at the record's `+8`, which under this page's [cell] record
layout is `difficulty | medal << 8`... either the team record's payload is
laid out differently... or the comparison is doing something this pass did
not follow"). It reads differently because it **is** a different store:
`DAT_08b31774` (a per-team record, keyed by team name through the same
generic accessor `FUN_08808664`), not `DAT_08b317b4 + 0x43c` (the per-cell
record store `race-campaign.md` documents). `EndRaceRewards_Update`'s own
read of that same store's `+8` field as a plain accumulated total is
independent, decompiled corroboration that `Unlock_LoyaltyMet`'s comparison
is exactly what its name says. Raise `Unlock_LoyaltyMet` and
`Unlock_LoyaltyValue` from 72/78 to **82/82** on this basis - a second,
unrelated call site reading the identical field as a plain counter, not a
runtime trace, so short of the rubric's top bands.

**What was not found: the writer of `g_endrace_result + 8` itself.**
`get_xrefs_to(DAT_08b317b4)` returns over fifty call sites (two `[WRITE]`,
`0x08826d40` in `FUN_08826cac` and `0x088291ac` in `FUN_08829124` - both
likely the pointer's own allocation site, not a race-end summariser writing
into an offset of what it points at, which would show as a `[READ]` of the
base followed by an offset store `get_xrefs_to` does not resolve). Whether
`0x08826cac`/`0x08829124` or one of the ~48 `[READ]` sites is the actual
loyalty-award computation was not narrowed further this pass - see
[Open](#open).

**The `30 points per Time-Trial-family lap` reading is a hypothesis, not a
decompile.** `MSC_LOY_LAP15`/`MSC_LOY_LAP30`/`MSC_LOY_LAP10`'s own resolved
English text was not read this pass (no `MSC_LOY_*` string exists in the
executable itself - every one of these idstrings resolves through
`FUN_088938ec`'s runtime localisation call, the same one every other
idstring on this page goes through, to text this pass did not extract).
Reading the `30` suffix as a literal per-lap point value and multiplying by
this run's own `3` laps reproduces `90` exactly, with every other bonus term
(perfect lap, elimination, zone, the difficulty multiplier) correctly absent
for a plain, medal-less, non-suggested-ship Time Trial - a real, checkable
fit, but one observation. Confidence **70**.

## `EndRace Menu`

| Address | Name | Conf | Role |
| --- | --- | --- | --- |
| `0x088d81dc` | `EndRaceMenu_OnEnter` | 76 | seeds the difficulty-list handle, calls the base hook, dispatches to the option-list populate |
| `0x088d84ec` | `EndRaceMenu_DispatchPopulate` | 72 | single-player vs. split-screen (`DAT_08b31048 < 0xe`) dispatch |
| `0x088d8c00` | `EndRaceMenu_PopulateOptions` | 80 | builds the `Endrace Options` rows, and (for Time Trial/Speed Lap) the ghost comparison |
| `0x088d8648` | `EndRaceMenu_PopulateExistingGhost` | 78 | loads/reports the existing on-disk ghost for this track/class |

`EndRaceMenu_PopulateOptions` adds rows conditionally, matching
`docs/formats/endrace-screens.md`'s own numbered list exactly - `ER_NEXT_RACE`
only mid-Tournament (`DAT_08ab07e3 == 0 && DAT_08b31048 == 4 &&
DAT_08b30fa4 < DAT_08b30fa0 - 1`), then exactly one of `ER_SAVE_QUIT`/
`ER_RETURN_MENU`/`ER_RETURN_GRID`, then `ER_RACE_AGAIN` unless mid-Tournament,
then `ER_VIEW_AGAIN` unconditionally, then (Time Trial/Speed Lap only)
`ER_SAVE_GHOST`+`MSC_DEL_DATA` alongside the ghost block.

The just-driven run's own best lap (`GhostLine2`/`GhostTime2`, `ER_NEW_GHOST`)
is read from a scalar at `DAT_08b317b4 + 0x1930` - a second offset off the
same large global `g_endrace_result` sits under, distinct from it. A
disabled-reason flag (`0x8`, on the menu entry's own state) is set and
`GhostTime2` blanked to a fixed placeholder when
`*(int*)(DAT_08b317b4 + 0x1918) == 0` (no existing ghost was found to compare
against) - decompiled, not observed in the one capture this pass has (which
shows a real existing-ghost-absent case going the *other* way, i.e. the
capture's own `NO GHOST TIME FOUND` + a real `NEW GHOST RECORD: 0.49.34`, so
this specific disabled-SAVE-GHOST branch was not exercised by that capture
either - see [Open](#open)).

## Deliverable 3: `0x088c8a10`'s containing function is not a lock check

**Assignment: decompile the caller `StateMachine_TransitionTo`'s own
positive-control breakpoint pinned at `ra = 0x088c8a10`
(`race-campaign.md`'s "Runtime-verified 2026-09-14 (deliverable 1)"), and
write the predicate it tests into that page's "Unlock rules" section.**

`0x088c8a10` is a return address inside `FUN_088c8798` (body
`0x088c8798`-`0x088c8a3b`), not a function entry. Decompiled in full:

| Address | Name | Conf | Role |
| --- | --- | --- | --- |
| `0x088c8798` | `StateMachine_EvaluateRedirect` | 75 | the generic `<Redirect><Entry item= equals= goto=>...<Default goto=>` evaluator |

```
StateMachine_EvaluateRedirect(redirectNode):
    focusValue <- the currently focused widget's own resolved value
    if redirectNode has no "always redirect" pair set:
        for each <Entry item= equals= goto=> row on this node:
            if item == "Focus" and that widget's value == focusValue:      # the Focus special case
                resolve goto by name, StateMachine_TransitionTo(...); return 1
            if the named widget's own cached string == this row's "equals": # an ordinary value test
                resolve goto by name, StateMachine_TransitionTo(...); return 1
        if a <Default goto=> exists on this node:
            StateMachine_TransitionTo(..., default_goto, ...); return 1
        # no Entry matched and no Default: falls through, no transition, return 0
    else:
        StateMachine_TransitionTo(..., the "always redirect" target, ...); return 0
```

This is the code behind the `<Redirect>` schema `race-setup.md` and this
page's own `EndRace Menu` section both already read from XML - `EndRace
Menu`'s own big `<Redirect><Entry item="Endrace Options" equals="ER_RACE_AGAIN"
goto="InGame Restart">...` is exactly this shape. `get_function_callers`
lists eleven call sites spanning `0x08814014` to `0x088e81f8` - this is a
shared, screen-agnostic evaluator, not code specific to `CellSelection` or
the campaign.

**It never reads a `Locked` byte, a medal, or anything shaped like
`PI_Cell`/`PI_Grid`'s own offsets** - confirmed by inspecting every field
this function touches: the redirect node's own `Entry` list, its `Default`
target, and the special-cased string `"Focus"`. **So `StateMachine_EvaluateRedirect`
itself is not the predicate that swallows a confirm on a locked tile** -
whatever refuses the transition does so *before* this function is ever
invoked on the redirect node in question, not inside it. This narrows, but
does not close, `race-campaign.md`'s own open item: the refusal is
upstream of `StateMachine_TransitionTo` (already established) and now also
upstream of the generic redirect evaluator specifically, not inside it.

One of this function's eleven callers, `0x088d7e1c`, is close in address to
`CellSelection`'s own code region and worth naming as the most promising
next thread, at confidence below the naming floor for now:

```
FUN_088d7e1c(widget, redirectNode):          # hypothesis: a generic Redirect widget's own per-frame Update
    if *(short*)(redirectNode + 0xbe) == 0:  # an "enabled" gate - not confirmed against the XML's own
        base_update(widget, redirectNode)    # `enabled`/`StartEnabled` attribute this pass saw on other
    else:                                    # <Redirect> nodes (e.g. EndRace_Definition.xml's own
        if Input_IsPressed(g_input, CONFIRM): #  AutoRedirect/BackwardsAutoRedirect, StartEnabled="false")
            StateMachine_EvaluateRedirect(redirectNode->target)
        ... (a second, timed confirm path via a held-button duration, not decompiled in full here)
        base_update(widget, redirectNode)
```

**Confidence 55 - below the naming floor, not renamed.** If `+0xbe`'s enable
bit is toggled per-tile (cleared while the focused `GridController` tile's
own lock layer is visible, set otherwise), this would be exactly the
"generic, non-PI001 widget rule" `race-campaign.md` already hypothesises -
but no write site for `+0xbe` was located this pass, so this is a plausible
shape, not a finding. **The single highest-value breakpoint left on this
question**: arm on `0x088d7e1c`'s entry with the cursor on a locked,
unmedalled `Cell Selection` cell versus an unlocked one (the same two cells
`race-campaign.md`'s own "Runtime-verified 2026-09-14 (deliverable 1)"
pass already used), and read `*(short*)(param_2 + 0xbe)` at each - if it
differs, this is the gate; if it is identical on both, the refusal sits
somewhere else in this function's own un-narrowed input-handling path, or a
level further upstream still.

## Open

- **The loyalty-award computation's own writer** (`g_endrace_result + 8`,
  and its siblings `+0x1134`/`+0x1138`/`+0x1140`/`+0x1144`/`+0x1148`/
  `+0x114c`/`+0x1154`/`+0x1160`/`+0x1164`/`+0x1165`) was not located. Arm a
  breakpoint on entry to the `EndRace Results` state (or watch
  `DAT_08b317b4 + 0x7d8 + 8` for a write across the finish line) to find it
  directly - the single highest-value next step on this page.
- **`0x088d7e1c`'s own `+0xbe` gate** - see above, the lead deliverable 3
  leaves open.
- **`EndRaceResults`/`EndRaceMenu`'s own `Update`/`OnExit` slots**
  (`0x088da530`, `0x088d92e0`, `0x088d8338`, `0x088d8248`) were positionally
  located (vtable word 9/31) but not decompiled this pass.
- **The Tournament/Zone/Elimination/split-screen populate helpers**
  (`FUN_088dad90`, `FUN_088db574`, `FUN_088db1ec`, `FUN_088d9588`) were
  identified by call site only, not opened - see `docs/formats/endrace-screens.md`'s
  own `Line2`..`Line8` and `boostimg` open items, both of which likely close
  once one of these is read.
- **`MedalImg`'s own state in a medal-earning run** was not observed (the
  one capture is a no-medal run) - see the formats page.
- **Nothing here is cross-checked against `psp-pulse-eu`.** Per this
  project's EU-preference
  ([ADR-0048](../../../architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md)),
  a `get_bulk_function_hashes` sweep of this page's fourteen new addresses
  against the EU binary is the next cross-platform step - not attempted
  this pass (budget went to the USA-side reading itself, which had not been
  opened at all before this pass).

## Names recovered

| Address | Name | Conf |
| --- | --- | ---: |
| `0x088db968` | `EndRaceResults_Construct` | 78 |
| `0x088dd89c` | `EndRaceRewards_Construct` | 78 |
| `0x088d90dc` | `EndRaceMenu_Construct` | 78 |
| `0x088d98cc` | `EndRaceResults_OnEnter` | 82 |
| `0x088da8c8` | `EndRaceResults_PopulateLapTable` | 80 |
| `0x088d939c` | `EndRaceResults_ResetTable` | 76 |
| `0x088dbbd4` | `EndRaceRewards_OnEnter` | 85 |
| `0x088dd2b8` | `EndRaceRewards_Update` | 85 |
| `0x088d81dc` | `EndRaceMenu_OnEnter` | 76 |
| `0x088d84ec` | `EndRaceMenu_DispatchPopulate` | 72 |
| `0x088d8c00` | `EndRaceMenu_PopulateOptions` | 80 |
| `0x088d8648` | `EndRaceMenu_PopulateExistingGhost` | 78 |
| `0x088c8798` | `StateMachine_EvaluateRedirect` | 75 |
