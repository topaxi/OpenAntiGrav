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
reproduces the capture's `90 Points` exactly under the law now decompiled
below.
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

### The loyalty-award computation, decompiled and runtime-confirmed on two independent races

**Found 2026-09-14, entirely from the Ghidra bridge, then verified live in
PPSSPP.** The static route the previous pass left as its own next step -
"the finish-line path, `Race_RecordResult`" - worked directly:
`Race_RecordResult`'s own single caller, `Race_BuildEndRaceResult`
(`0x0882a498`, previously `FUN_0882a498`), calls a second function
immediately after it that reads every field the reason-string table above
enumerates and writes straight into `g_endrace_result + 8`:

```
Race_BuildEndRaceResult(base, shipDestroyedFlag):          # base = *(int*)DAT_08b317b4,
    ...                                                     # g_endrace_result = base + 0x7d8
    base->0x1938 (== g_endrace_result+0x1160) <- Race_RecordResult(...)   # the medal ordinal
    award <- Race_ComputeLoyaltyAward(DAT_08b31774, base + 0x7d8)
    base->0x7e0 (== g_endrace_result+8) <- award                          # "90 Points", this race's own award
```

| Address | Name | Conf | Role |
| --- | --- | --- | --- |
| `0x0882a498` | `Race_BuildEndRaceResult` | 78 | the race-end summariser: fills the whole `g_endrace_result`-shaped sub-structure (ghost time, per-lap array, ranking sorts, tournament standings, the medal ordinal via `Race_RecordResult`, and the loyalty award via `Race_ComputeLoyaltyAward`) from a much larger in-race state object at `base+0x2c0`. Single caller, `0x08827b4c` (a trivial wrapper), itself called from six race/zone-end update paths including `Eliminator_UpdateKillTarget`. Confidence capped below the other rows here because most of its own body (craft ranking sorts, the Tournament-standings block, several offsets read but not named) was not traced past "this writes/reads here" - the loyalty and medal writes specifically are confidence 90+, the function as a whole is not. |
| `0x0880ac50` | `Race_ComputeLoyaltyAward` | **95** | the law itself - full decompile, reproduces `90` exactly on two independent live races (below) |
| `0x08807884` | `Loyalty_AccumulateTotal` | 90 | `*(record+8) += award`, capped at `100000` - the per-team persistent-total writer `Unlock_LoyaltyMet`/`loyaltynum` read |

`Race_ComputeLoyaltyAward(teamStore, g_endrace_result)`, decompiled in full:

```
laps          <- g_endrace_result + 0x1140
perfectLaps   <- g_endrace_result + 0x1144
kills         <- g_endrace_result + 0x1154
zones         <- g_endrace_result + 0x1134
perfectZones  <- g_endrace_result + 0x1138
difficulty    <- g_endrace_result + 0x115c      # 0 easy, 1 medium, 2 hard
suggestedShip <- g_endrace_result + 0x1164      # bool
mode          <- DAT_08b31048 (or 0 under split-screen, DAT_08ab07e3)

lapTerm  <- mode in {Race,Tournament,Head2Head, 3/4/9}: laps*15 + perfectLaps*25
         <- mode in {TimeTrial,SpeedLap, 5/10}:         laps*30 + perfectLaps*50
         <- otherwise (Zone, Elimination, ...):          laps*10 + perfectLaps*20

killTerm <- mode == Elimination(8): kills*30, else kills*15

difficultyMult <- 1                              # default: no multiplier at all
if mode in {Race,Tournament,Head2Head}:
    difficultyMult <- {0: 2, 1: 3, 2: 4}[difficulty]   # Easy x2, Medium x3, Hard x4
if suggestedShip: difficultyMult <- difficultyMult * 2

award <- difficultyMult * (lapTerm + killTerm + zones*10 + perfectZones*20)

record <- FUN_08808664(teamStore, Libc_HashString(team_name), 0, create=1)
Loyalty_AccumulateTotal(record, award)            # record+8 += award, capped 100000
return award
```

This is exactly the reason-string vocabulary above turned into arithmetic:
`MSC_LOY_LAP{15,30,10}`'s own suffix numbers **are** the per-lap rates
(confirming the previous pass's "reading the suffix as a literal value" was
right), `MSC_LOY_PLAP{25,50,20}` the per-perfect-lap rates,
`MSC_LOY_ELIM{30,15}` the per-kill rates, `MSC_LOY_ZONE10`/`MSC_LOY_PZONE20`
the zone rates, and `MSC_LOY_EASY2`/`MSC_LOY_MED3`/`MSC_LOY_HARD4` the
Race-family difficulty multiplier - all five reason-string families this
page's reason-ticker table already named, now each with a decompiled
arithmetic weight rather than an inferred one.

**Runtime-confirmed 2026-09-14, PPSSPP v1.20.4, `pulse-psp-usa.chd`, Xvfb
`:97`, fresh zero-profile boot.** Two independent `grid0_3_2` (Time Trial,
Venom, `16_Track`/Talon's Junction, `Weapons="off"`) races, flown by
`psp-autopilot.py` on different lines and at different paces, both reaching
3 laps with zero perfect laps, zero kills/zones (Time Trial has none) and no
medal:

| Run | Per-lap times | Total | `laps`/`perfectLaps` | Predicted award | `g_endrace_result+8` (live read) | On-screen `X Points` / `Total loyalty` |
| --- | --- | --- | --- | --- | --- | --- |
| A (prior pass, OCR only) | `1.32.49`/`0.49.34`/`0.49.93` | `3.11.76` | 3 / 0 | `3*30 + 0*50 = 90` | not read | `90 Points` |
| B (this pass, live memory) | `1.15.89`/`0.49.21`/`0.49.51` | `2.54.61` | 3 / 0 | `3*30 + 0*50 = 90` | **`90`** | `Assegai Loyalty: 90 Points`, `Total loyalty: 90` |

Run B's `Total loyalty` reads `90` because this is the profile's first-ever
race (Team Selection's own loyalty bar read `0` before launching); the
in-game screenshot and the raw `record+8` read agree, and `Loyalty_AccumulateTotal`'s
own decompile (`*(record+8) += award`) is the reason why. Two different
finishing times, same lap/perfect-lap shape, same award, exactly as the
formula predicts and independent of pace - this closes the "one observation"
caveat the previous pass's reading carried. Confidence **95** for
`Race_ComputeLoyaltyAward` (full decompile plus an exact arithmetic match
replayed on two separately-driven races); **90** for `Loyalty_AccumulateTotal`
and the `Race_BuildEndRaceResult` call sites that wire the two together.

**Still open**: the Race-family branch (difficulty multiplier, the `15`/`25`
lap rate, the `30` kill rate) and the Zone/Elimination `10`/`20` branch are
decompiled but not runtime-verified - both captures this pass has are Time
Trial. A `Single Race`/`Race` cell would need `psp-autopilot.py --craft
ADDRESS` (untested, see `ppsspp-debugger.md`) to drive safely with seven AI
opponents present.

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

### `0x088d7e1c` ruled out; the real `Cell Selection` confirm dispatcher found, and the swallow narrowed to input consumption

**Runtime-confirmed 2026-09-14, PPSSPP v1.20.4, same zero-medal profile.**
`0x088d7e1c` itself is **not** the gate - full decompile (above) plus its
own call graph shows it is the *held-confirm* variant of a generic Redirect
widget (three-button-code accumulator against a 2-second hold timer,
`DAT_08b317b0 + 0x40`), used by the `InGame Photo`-family screens
(`FUN_08814014`'s own `Race_End_Photo` redirect target sits in the same
call chain); it never runs on `Cell Selection` at all.

The **true** dispatcher was found by breakpointing `StateMachine_EvaluateRedirect`
itself (`0x088c8798`) with the cursor on `grid0_3_1` (unlocked, positive
control) and reading `$ra` on the hit: `ra = 0x088c907c`, inside
`FUN_088c8e88` - the twelfth caller this page's own `get_function_callers`
sweep had already decompiled and set aside as "a generic Confirm/Decline
dialog handler" (the same class `TRC_LOCKED`/gallery/account-check dialogs
use). Renamed `ConfirmButton_Update` (confidence **85**): its own
`StateMachine_EvaluateRedirect(param_1)` call, passing itself as the
redirect node, is at `0x088c9074` - the exact call the positive control's
`$ra` sits one instruction past. The same breakpoint **never fired** with
the cursor moved one `left` to `grid0_2_1` (locked, no `Locked` attribute)
- reproducing `race-campaign.md`'s own cell/tier findings from the
`StateMachine_EvaluateRedirect` side, and closing "which of the eleven
callers is `Cell Selection`'s own" with a concrete address.

**`ConfirmButton_Update` itself is not the gate either - traced instruction
by instruction.** Passively breakpointed at its own entry (no button
pressed), it fires identically on both the locked and the unlocked cursor
position - same instance address (`a0 = 0x08d8d030`, a single persistent
per-screen object, not one per tile), same internal state bytes
(`00000000 01000000 04000000 ffffffff ...`, i.e. accept-button code `4`,
decline unbound) on both. Disassembling its own accept branch
(`0x088c8fec`-`0x088c9074`) shows every path through it - the `*(param_1+0x264)`
early-exit gate included - converges on the same unconditional
`jal 0x088c8798` at `0x088c9074` once "accept is pressed" is true; there is
no `Locked`/medal read anywhere in this span. So **the swallow is not a
Cell/Grid-specific check in any PI001 function traced so far** - it is
whatever makes `ConfirmButton_Update`'s own "is accept pressed" test
(`Input_IsPressed(g_input, 4, 0)` off `*(param_1+0x198)`, a screen-global
button code, not a per-tile one) read false on a locked-tile frame despite
`cross` being held for the full duration of the negative-control press (up
to 6 s, no hit). Confidence **85** for "the gate is upstream of
`ConfirmButton_Update`'s own accept-check, not inside it" - a runtime trace
with its own positive and negative control, both reproduced from the
`StateMachine_EvaluateRedirect` side and the `ConfirmButton_Update` side
independently.

**What this leaves for the next pass**: `Input_IsPressed`'s own callers (or
whatever calls `Input_ConsumePress`) on the same frame, for button code `4`,
specifically when the focused `GridController` tile's lock layer is
visible - not located this pass. This is now a much narrower target than
"eleven candidate functions" or "an unconfirmed `+0xbe` flag": a single
input-consumption question, upstream of a function whose own body is fully
understood and cleared.

## Open

- ~~**The loyalty-award computation's own writer**~~ **Closed 2026-09-14** -
  `Race_ComputeLoyaltyAward` (`0x0880ac50`), called from
  `Race_BuildEndRaceResult` (`0x0882a498`); see "The loyalty-award
  computation, decompiled and runtime-confirmed" above.
- **The per-lap `+0x10` pennant column's own writer, during the race** -
  narrowed but not closed 2026-09-14. `Race_BuildEndRaceResult`'s own copy
  loop (already decompiled: `iVar19 = base; ...; *(short*)(iVar19+0x7e8) =
  *(short*)(iVar15+0x94); iVar15 += 0x10; iVar19 += 8`) shows the raw source
  is `*(session + 0x900 + lap*0x10 + 0x94)`, a 2-byte field in an 8-lap,
  16-byte-stride per-lap record (siblings `+0x88` time, `+0x8c` perfect-lap
  flag, `+0x90` lap number - all three already named as the copy targets
  `+0xc`/`+0x12`/`+0x13`) inside the in-race state object at `base+0x2c0`
  (itself not named this pass). This is the *copy site*, not the writer -
  whatever fills `session+0x900+lap*0x10+0x94` during the race itself was
  not located; `search_instructions` on the raw offset `0x94` alone returns
  241 matches (mostly unrelated `sw ra,0x94(sp)` prologue spills) and is not
  selective enough. **Two live data points now exist** (both `grid0_3_2`,
  Venom, `Weapons="off"`): `6,10,8` (sum 24, this pass) and `6,9,10` (sum
  25, the prior pass) - **weapons/pickups are ruled out for both**, since
  neither race could carry a weapon at all. Lap 1 reads `6` in both
  independently-driven races starting from the same grid position;
  laps 2-3 differ by one or two. Speedup/boost pads (present regardless of
  the `Weapons=` setting, and plausible to cross a slightly different count
  of depending on the exact line) are the strongest remaining candidate,
  not confirmed. The next step is a live breakpoint on writes to
  `session+0x900+lap*0x10+0x94` itself (needs `session`'s own address,
  readable at any `EndRace Results` breakpoint as `*(int*)(base+0x2c0)`),
  not another correlation pass.
- ~~**`0x088d7e1c`'s own `+0xbe` gate**~~ **Ruled out 2026-09-14** -
  `0x088d7e1c` is the held-confirm variant used by `InGame Photo`, not
  `Cell Selection`'s own dispatcher; see "`0x088d7e1c` ruled out..." above.
  The real dispatcher (`ConfirmButton_Update`, `0x088c8e88`) is found and
  cleared too - the remaining gate is upstream of it, in input consumption,
  not a `+0xbe`-shaped flag.
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
| `0x0880ac50` | `Race_ComputeLoyaltyAward` | 95 |
| `0x08807884` | `Loyalty_AccumulateTotal` | 90 |
| `0x0882a498` | `Race_BuildEndRaceResult` | 78 |
| `0x088c8e88` | `ConfirmButton_Update` | 85 |
