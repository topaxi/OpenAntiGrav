# The three EndRace screens: `Results`, `Rewards`, `Menu`

**Status: read.** `Data\Plugins\PI001\GUI\EndRace_Definition.xml`, in `Data.wad`,
dictionary-shortened like every other Pulse front-end file, listed by
`Skin.xml`'s `LoadXML` per
[fe-menu-definitions.md](fe-menu-definitions.md#where-they-are). Ten `<Screen>`
blocks; the three that carry the content this page is about are
`EndRace Results`, `EndRace Rewards` and `EndRace Menu` - what a campaign or
custom race shows between the finish line and being handed back to a menu.
This is the picture half; the decompiled law behind every value is
[`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`](../ghidra/functions/psp-pulse-usa/endrace-screens.md).
`docs/ui/campaign-screens.md`'s "After a campaign race, measured" section is
the one live capture (PPSSPP, a `grid0_3_2` Time Trial, no medal) this page's
readings are checked against; nothing here is runtime-verified beyond that
one capture.

Reproduce with:

```sh
just wad cat data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
  'Data\Plugins\PI001\GUI\EndRace_Definition.xml' --expand
```

## `EndRace Results`: `RESULTS` / a per-lap table

| Widget | Authored as | Runtime source | Confidence |
| --- | --- | --- | --- |
| `BigTopText` | `idstring="ER_RES"` | unconditional ("RESULTS") | 90 - direct XML read |
| `Line1` | `string="race complete!"` template | overwritten per mode - see below | 80 - decompiled |
| `Line2`..`Line8` | blank `Text` rows, `x=100` | filled only by the Tournament/Zone/Elimination-specific populate helpers this pass did not decompile | 50 - not traced |
| `table` (a `Text` acting as a group) | wraps the whole per-lap grid | shown/hidden as one unit | 80 |
| `lap0.0`/`lap0.1`/`lap0.2` | blank header cells, `x=140/220/320` | `RC_LAP` ("LAP"), `PRO_TIME` ("TIME"), blank | 82 - decompiled |
| `lap1.0`..`lap8.2` (24 more cells, rows 2-9) | blank, `y` 95..235 | per-lap values, row *n* filled only while `n <=` laps completed (max 5 shown by the single-player path, see below); a 9th row (`tablebg9`) is the totals line | 80 |
| `perfectlap1`..`perfectlap5` | plain `Image` + `idstring="MSC_PL"` overlay, `x=352`, `y` matching rows 1-5 | shown by default (XML baseline), explicitly hidden per-lap when a stored per-lap flag byte is nonzero | 65 - direction of the flag not resolved |
| `boostimg` | plain `Image`, `x=320 y=77` (header row) | explicitly hidden, unconditionally, on the single-player path (see below) | 78 |
| `ContinueButton`/`ControlTextConfirm` | `FE_CONFIRM_BUTTON`/`FE_CONFIRM` | direct XML idstrings | 90 |
| `EndRaceRewardsRedirect` | `<Default goto="EndRace Rewards">` | one of the two - see below | 70 |
| `EndRaceMenuRedirect` | `<Default goto="Race End Save">` | the other of the two | 70 |

### `Line1`'s headline text is mode-driven, and it is the same field `docs/ui/campaign-screens.md` captured as `TIME TRIAL COMPLETE!`

`EndRaceResults_OnEnter` (`0x088d98cc`, [ghidra page](../ghidra/functions/psp-pulse-usa/endrace-screens.md)) switches on a mode-shaped value and, for the
single-race family (`Race`/`Time Trial`/`Head2Head`/`Speed Lap` and their
split-screen variants), further narrows `Line1` from a bare finishing-position
string down to `ER_TT_COM` for Time Trial or `ER_SL_COM` for Speed Lap -
exactly the two-step "generic, then mode-specific" pattern
`race-campaign.md`'s own `CellSelection_PopulateDetail` table already
documents for other widgets. Confidence 82 - clean decompile, and the one
live capture (`grid0_3_2`, Time Trial) reads `TIME TRIAL COMPLETE!` exactly as
this reading predicts.

### The per-lap table: `Lap`/`Time` read cleanly, the third column does not

`EndRaceResults_PopulateLapTable` (`0x088da8c8`) fills row *n* (1-indexed, up
to 5) of the table from a per-lap record, stride 8 bytes, inside the same
per-race result structure `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`
names `g_endrace_result` (offset conventions below are relative to one lap's
own 8-byte slot):

| Column | Widget | Source | Format |
| --- | --- | --- | --- |
| 0 | `lap{n}.0` | a stored byte at `+0x13` | decimal |
| 1 | `lap{n}.1` | a stored value at `+0xc` | the same centisecond time formatter `race-campaign.md` already names (`FUN_088196e4`) |
| 2 | `lap{n}.2` | a stored 16-bit value at `+0x10` | decimal |

`docs/ui/campaign-screens.md`'s capture read column 2 as `6`/`9`/`10` summing
to a `25` total, and called it "a pennant-icon column" on sight. **This pass's
decompile does not confirm an icon binds to that column at all**: its header
cell (`lap0.2`) is left blank by `EndRaceResults_PopulateLapTable`, and the
one icon on this screen shaped like a per-lap indicator - `boostimg` - is
unconditionally hidden by the same function before the per-lap loop runs, for
every mode this function handles (`Race`/`Time Trial`/`Head2Head`/`Speed
Lap`). So the "pennant" in the capture's own description is most likely the
`perfectlap{n}` icon (`x=352`, sitting immediately to the right of column 2's
`x=320`, not inside it) read as part of the same visual row, not column 2's
own digits. **What column 2's stored 16-bit value actually counts is not
determined this pass** - candidates considered and not confirmed: a
finishing-position-per-lap (ruled out, the observed 6/9/10 exceed the 8-craft
field and this cell has no opposing craft in Time Trial), a per-lap score, or
a boost/pickup count carried in a field distinct from the (unconditionally
hidden) `boostimg` icon. Confidence 40 on any specific reading; 78 on the
structural facts above (the field exists, is a per-lap 16-bit int, and is not
what draws the header icon this pass could find). The totals row's own
aggregate (`+0x1148` of the shared struct, not summed client-side) reproduces
the capture's `25` from `6+9+10` - an internally consistent whole, whatever
the unit is. A PPSSPP breakpoint on `EndRaceResults_PopulateLapTable`
(`0x088da8c8`), read back on a race with a nonzero boost count, would settle
this directly.

### Two Redirects, one screen - which one fires is mode-driven too

`EndRaceRewardsRedirect` and `EndRaceMenuRedirect` are both always-present
state-table entries (unlike `CellMode_Definition.xml`'s buttonless pair,
these two sit *inside* the `EndRace Results` screen block, not beside it).
`EndRaceResults_OnEnter`'s own tail toggles an enable bit on each based on
the same "has a finishing position" flag (`+0x1165` of the shared result
struct) that also picks `Line1`'s text - decompiled, not runtime-verified;
confidence 65 on the exact bit logic (the toggle pattern was read once, not
independently re-derived), 85 that exactly one of the two is reachable per
race. The single capture this pass has (Time Trial, no medal) took the
`EndRace Rewards` branch, matching `docs/ui/campaign-screens.md`'s own
walk (`results-01.png` -> `results-02.png` -> `results-03.png`).

## `EndRace Rewards`: `REWARDS` / the medal glyph / the loyalty ticker

| Widget | Authored as | Runtime source | Confidence |
| --- | --- | --- | --- |
| (unnamed header `Text`) | `idstring="ER_REWARD"` | direct, unconditional ("REWARDS") | 90 |
| `RewardLine1` | `idstring="ER_MEDAL_AWARD"` template | overwritten to `ER_GMA`/`ER_SMA`/`ER_BMA`/`ER_NMA` (gold/silver/bronze/none), then possibly again to a finishing-position or ship-description idstring for the position-carrying modes - see below | 82 - decompiled |
| `MedalImg` (`Data\FE\Images\reward_icons.mip`, `U=0 V=32`, 32x32) | a single fixed icon | visible by default; explicitly re-hidden only in the no-medal case (see below) | 68 - see the medal-glyph section |
| `TrophyPanel` (`Mode3D`), three `Model`s `g_trophy`/`s_trophy`/`b_trophy` (`Data\FE\trophies\gold.vex`/`silver.vex`/`bronze.vex`, all `StartPaused="yes"`) | three paused 3D models at the same origin | exactly one is unpaused, matching the medal ordinal - see below | 80 - decompiled |
| `BigPos` | `string="1"` template | a finishing-position figure, for the modes that have one | 75 |
| `RewardLine2` | `idstring="ER_LOY"` template | overwritten to `"<team> Loyalty:"` (`"%s %s"` of the team's own localised name and `ER_LOY`) | 78 |
| `RewardLoyaltyActive` | `string="line 2"` (generic placeholder) | a two-role scrolling ticker - this race's own loyalty award, then a cycle of "why" reason strings - see below | 78 |
| `loyaltynum` | `string="0"` | the **team's persistent total** loyalty, not this race's award - see below | 78 |
| `loyaltybar`/`loyaltybg` | a fixed-size fill bar, `124x10` | fill fraction = `total * 0.00124` | 75 |
| `ContinueButton` etc. | as `EndRace Results` | direct | 90 |
| `EndRaceRewardsMenuRedirect` | `<Default goto="Race End Save">` | fires once the ticker has scrolled past `reasonCount * 0.5 + 1.5` seconds | 75 - decompiled |

### The medal glyph is not a re-sourced icon - it is which 3D trophy gets unpaused

**Confidence 80, decompiled, not runtime-verified beyond the one no-medal
capture.** `EndRaceRewards_OnEnter` (`0x088dbbd4`) reads an ordinal at offset
`+0x1160` of the same per-race result structure the Results screen reads -
`0 = gold, 1 = silver, 2 = bronze, anything else (including the `<0` sentinel
a signed byte read of `0xff` produces) = none` - **the identical ordinal
`Cell_EvaluateMedal` (`race-campaign.md`) already established**, which is
strong independent corroboration this field is a direct copy of that race's
own medal result, not a separately-invented scale. That ordinal:

1. Sets `RewardLine1` to `ER_GMA`/`ER_SMA`/`ER_BMA`/`ER_NMA` - unconditionally,
   whether or not a campaign cell is in play.
2. Only when a campaign cell **is** in play (`DAT_08b30ffc != 0`,
   `race-campaign.md`'s own campaign-cell pointer): unpauses exactly one of
   the three `TrophyPanel` models (clearing, then setting, a flag on the
   model's own inner node - all three start cleared at setup) and plays the
   matching jingle (`gold_med`/`silver_med`/`bronze_med`). A non-campaign
   race skips the trophy/jingle step entirely and takes a separate,
   simpler branch that unconditionally hides `MedalImg` and never resolves a
   trophy at all.

So the three medal colours are **not** three sub-rects of `MedalImg`'s own
`reward_icons.mip` - unlike `Cell Selection`'s `Target0..2 Image` swatches
(`docs/ui/campaign-screens.md`), which really are a re-sourced image. They
are three separate, pre-modelled trophies sharing one `Mode3D` origin, gated
by a pause flag. `MedalImg` itself is a single fixed icon (most likely the
"no medal" hex-dash glyph `docs/ui/campaign-screens.md`'s capture describes),
explicitly re-hidden only in the no-medal branch; **whether it is also hidden
in a medal-earned run by some code this pass did not reach, or is meant to
sit visible underneath the spinning trophy, is not determined** - the one
capture this pass has is itself a no-medal run, so it cannot distinguish the
two. Confidence 55 on "no-medal-only" specifically; 80 on the trophy-select
mechanism, which does not depend on resolving this.

### The loyalty award: `90 Points` is a read of a precomputed field, and the arithmetic behind it is a one-observation hypothesis, not a decompile

**`EndRaceRewards_Update` (`0x088dd2b8`) is where `RewardLoyaltyActive` and
`loyaltynum` actually get their text, not `OnEnter`.** Confidence 80,
decompiled in full, not runtime-verified. Once the ticker's cycle index
reaches its own reason-string count (i.e. on the ticker's very first
"beat", before any reason string has had a turn):

- `RewardLoyaltyActive` <- `"%d %s"` of `*(g_endrace_result + 8)` and
  `ER_POINTS` - **this is `"90 Points"`, read directly off a field this pass
  did not find the writer of.**
- A **persistent, per-team** record is looked up by `FUN_08808664(DAT_08b31774,
  <team-name-hash>, 0, 0)` - the identical generic keyed-record accessor
  [`race-box-screens.md`](../ghidra/functions/psp-pulse-usa/race-box-screens.md)
  already names for `TeamSelection`'s own per-craft rating table, now shown
  reused for a second, unrelated store. That record's own `+8` field is
  formatted into `loyaltynum` as `"%s %d"` of `ER_TOT_LOY` and this value -
  **`"Total loyalty: 90"`, the team's running total**, not this race's award.
  `loyaltybar`'s fill fraction is the same total scaled by `0.00124`.
- Every following ticker "beat" (roughly every 10 update-timer units,
  `param_1 * 20.0` accumulated against a fixed `10.0` threshold) instead
  shows the next reason string built in `OnEnter` - see below - and the
  Rewards screen's own transition to `Race End Save` fires once the ticker
  has run past `reasonCount * 0.5 + 1.5` seconds total.

This closes one of `race-campaign.md`'s own open items: `Unlock_LoyaltyMet`
(`0x0888ea30`) was scored 72 there because comparing "a whole 32-bit word at
the record's `+8`" did not read as a loyalty counter *under a cell record's
own layout* (`difficulty | medal << 8`). It does not need to - `Unlock_LoyaltyMet`
reads a **different** store (`DAT_08b31774`, this same per-team accessor),
not the cell-hash store `DAT_08b317b4 + 0x43c` `race-campaign.md` documents,
and this pass's own reading of `loyaltynum` shows that store's `+8` genuinely
is a plain accumulated total. See the ghidra page for the confidence bump
this warrants on `Unlock_LoyaltyMet` itself.

**The reason-string ticker, built once in `OnEnter`, is a real, structured
vocabulary, not filler text:**

| Idstring | Shown when |
| --- | --- |
| `MSC_LOY_LAP15` / `MSC_LOY_LAP30` / `MSC_LOY_LAP10` | once per lap completed (count from `g_endrace_result + 0x1140`), picking Race-family / Time-Trial-and-Speed-Lap-family / everything-else by the same three-way mode split `EndRaceResults_OnEnter` uses for `Line1` |
| `MSC_LOY_PLAP25` / `MSC_LOY_PLAP50` / `MSC_LOY_PLAP20` | once per perfect lap (`+0x1144`), same three-way split |
| `MSC_LOY_ELIM30` / `MSC_LOY_ELIM15` | once per elimination kill (`+0x1154`), the `30` variant only for `Elimination` mode specifically (not its split-screen sibling) |
| `MSC_LOY_ZONE10` | once per zone cleared (`+0x1134`) |
| `MSC_LOY_PZONE20` | once per perfect zone (`+0x1138`) |
| `MSC_LOY_MULT` + one of `MSC_LOY_EASY2`/`MSC_LOY_MED3`/`MSC_LOY_HARD4` + (optionally) `MSC_LOY_SUG2` | only for the Race-family split (`uVar5 == 0`) or when a per-race "suggested ship" flag (`+0x1164`) is set - skipped entirely for a plain Time Trial with no suggested ship, exactly this pass's own captured case |

**Reading the suffix numbers as literal per-unit point values (15, 30, 10,
25, 50, 20, 30, 15, 10, 20) is not a decompile** - these are idstring names,
and the actual English text they resolve to was not read this pass (no
in-executable string table entry exists for any `MSC_LOY_*` key; they
resolve through the same runtime localisation call, `FUN_088938ec`, used for
every other idstring on this page, and the resolved text lives outside the
executable). It is nonetheless a strong, checkable hypothesis: **`30` (the
Time-Trial-family per-lap rate) times `3` (this run's own lap count) is
`90`, matching the one captured value exactly**, with every other bonus term
(perfect lap, elimination, zone, the multiplier block) correctly absent for
a plain, medal-less, non-suggested-ship Time Trial. Confidence **70** - one
observation, and the "30" itself is inferred from a name rather than read
off arithmetic, so this sits below the rubric's "one real file parses
plausibly" ceiling of 84 rather than at it. **The function that actually
computes `g_endrace_result + 8` and stores it there was not located this
pass** - `get_xrefs_to` on the struct's own base global (`DAT_08b317b4`)
returns over fifty call sites across the binary, both read and write, and
narrowing that list to the one race-end summariser needs either a
breakpoint (arm it on entry to `EndRace Results`, watch `+8` of
`DAT_08b317b4 + 0x7d8`) or substantially more decompilation than this pass's
scope covered. Left as the single highest-value next step on this page.

## `EndRace Menu`: `RETURN TO GRID` / the ghost comparison

| Widget | Authored as | Runtime source | Confidence |
| --- | --- | --- | --- |
| `Endrace Options` (`Menu`, 7 `test` placeholder entries) | a template list | built entry-by-entry, 2-6 real rows depending on mode - see below | 82 |
| `Endrace Difficulty` (`List`, Easy/Medium/Hard) | direct `<Entry idstring=>` rows | bound by the generic `<List>` mechanism; no code override found this pass | 55 - not traced |
| `GhostLine1`/`GhostTime1` | blank | the **existing** on-disk ghost for this track/class, or `MSC_GHOST_NOGHOST` ("no ghost time found") | 78 |
| `GhostLine2`/`GhostTime2` | blank | `ER_NEW_GHOST` + this run's own best lap, formatted from a scalar this pass did name a field for (`g_new_ghost_time`, `DAT_08b317b4 + 0x1930`) - see the ghidra page | 78 |

### The option list is built per mode, matching the observed `RETURN TO GRID`/`RACE AGAIN`/`VIEW RESULTS AGAIN`/`SAVE GHOST`/`DELETE DATA`

`EndRaceMenu_PopulateOptions` (`0x088d8c00`) adds rows in this order, each
one either present or absent:

1. `ER_NEXT_RACE` ("next race") - only mid-Tournament (a leg remains).
2. **Exactly one of** `ER_SAVE_QUIT` (mid-Tournament), `ER_RETURN_MENU` (no
   campaign cell in play, `DAT_08b30ffc == 0`) or `ER_RETURN_GRID` (a
   campaign cell in play) - this is the row `docs/ui/campaign-screens.md`'s
   capture reads as `RETURN TO GRID` for its own campaign run.
3. `ER_RACE_AGAIN` ("race again") - present unless mid-Tournament.
4. `ER_VIEW_AGAIN` ("view results again") - always present.
5. `ER_SAVE_GHOST` + `MSC_DEL_DATA` ("save ghost" / "delete data") - only for
   `Time Trial`/`Speed Lap`, alongside the ghost-comparison block below.

`<Redirect><Entry item="Endrace Options" equals="...">` in the XML maps every
one of those idstrings to its destination screen almost verbatim
(`ER_RETURN_GRID`/`ER_RETURN_MENU` both go to `Kill Game Transition`,
`ER_RACE_AGAIN` to `InGame Restart`, `ER_VIEW_AGAIN` back to
`EndRace Results`, `ER_SAVE_GHOST`/`MSC_DEL_DATA` to the two `MemoryStick`
screens) - confidence 90, direct XML read, corroborated by the option list
above always naming one of the entries the Redirect table actually handles.

### `NEW GHOST RECORD` reads this run's own best lap; the existing ghost is a separate, on-disk comparison

Two different scalars, two different widgets, confirmed by decompiling both
populate functions in full:

- `GhostLine1`/`GhostTime1` (`EndRaceMenu_PopulateExistingGhost`,
  `0x088d8648`): checks whether a ghost file already exists for this
  track/class (a generic exists-check keyed on the track and class, matching
  the same localisation call used throughout this page), and shows
  `MSC_GHOST_NOGHOST` if not, or attempts to load it and shows
  `MSC_GHOST_LOADFAIL` on a failed load. This is the row `docs/ui/campaign-screens.md`'s
  capture reads as `NO GHOST TIME FOUND` for its own first-ever run on
  `grid0_3_2`.
- `GhostLine2`/`GhostTime2` (inside `EndRaceMenu_PopulateOptions`): always
  shows `ER_NEW_GHOST` plus **this just-finished run's own best lap**, read
  from a scalar at `DAT_08b317b4 + 0x1930` and formatted with the same time
  formatter used throughout this page. This is the `0.49.34` the capture
  shows - the best of the three laps just driven, not the total time,
  matching `docs/ui/campaign-screens.md`'s own reading exactly. If no
  existing ghost was found to compare against
  (`*(DAT_08b317b4 + 0x1918) == 0`), `SAVE GHOST` is marked disabled (a
  disabled-reason flag, `0x8`) and `GhostTime2` is blanked to a fixed
  placeholder string instead of showing the time - not observed in the one
  capture this pass has (which shows a real time), so this branch is
  decompiled-only. Confidence 78.

## What is not determined

- **The per-lap third column on `EndRace Results`** (see above) - a real,
  stored 16-bit field, meaning unresolved.
- **The `perfectlap{n}` flag's direction** - hidden when a stored per-lap
  byte is nonzero; whether "nonzero" means "this lap was perfect" or the
  reverse was not settled.
- **`boostimg`'s actual firing condition.** Authored on this screen but
  unconditionally hidden by every single-player-mode code path this pass
  decompiled; presumably wired in the Tournament/Zone/Elimination/split-screen
  variants (`FUN_088dad90`/`FUN_088db574`/`FUN_088db1ec`/`FUN_088d9588`),
  none of which this pass opened.
- **`EndRace Results`'s two-Redirect toggle** - which of `EndRace Rewards`/
  `Race End Save` fires is decompiled at a structural level only; the exact
  bit arithmetic was read once, not independently re-derived or
  runtime-checked.
- **The loyalty-award computation itself** - the field `EndRaceRewardsUpdate`
  reads (`g_endrace_result + 8`) is real and decompiled; the function that
  *writes* it was not located. The "30 points per Time-Trial-family lap"
  reading is a one-observation hypothesis, not a decompile - see above.
- **`Endrace Difficulty`'s own binding** was not traced - likely the generic
  `<List global=...>` mechanism `race-setup.md` already documents for
  `Single Player`'s own difficulty row, not re-verified here.
- **The five support screens** (`EndRaceAutoSaveDisabled/Failed`,
  `EndRaceSaveGhost`, `EndRaceDeleteGhost`, `TournySaveWarning`,
  `Kill Game Transition`) are read directly off the XML above (all plain
  `Dialog`/`MemoryStick` blocks, no widgets needing a code binding) and not
  decompiled further - nothing in their own XML shape needed it.
- **Nothing here is runtime-verified beyond the one no-medal Time Trial
  capture** `docs/ui/campaign-screens.md` already has. A medal-earning run
  (any of gold/silver/bronze) would directly settle the `MedalImg`
  no-medal-only question and put a second data point on the loyalty
  hypothesis.
