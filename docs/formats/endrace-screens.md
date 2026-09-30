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
| `Line2`..`Line8` | blank `Text` rows, `x=100` | **never filled**: `EndRaceResults_OnEnter` is the only code that names them and it writes the empty string; none of the four populate helpers touches them (2026-09-30, all read) | 85 - cross-references, see the ghidra page |
| `table` (a `Text` acting as a group) | wraps the whole per-lap grid | shown/hidden as one unit | 80 |
| `lap0.0`/`lap0.1`/`lap0.2` | blank header cells, `x=140/220/320` | `RC_LAP` ("LAP"), `PRO_TIME` ("TIME"), blank | 82 - decompiled |
| `lap1.0`..`lap8.2` (24 more cells, rows 2-9) | blank, `y` 95..235 | per-lap values, row *n* filled only while `n <=` laps completed (max 5 shown by the single-player path, see below); a 9th row (`tablebg9`) is the totals line | 80 |
| `perfectlap1`..`perfectlap5` | plain `Image` + `idstring="MSC_PL"` overlay, `x=352`, `y` matching rows 1-5 | hidden by `ResetTable`, then **shown** for lap `n` when its stored per-lap flag byte is nonzero - the bit-`0x4`-is-visible correction, 2026-09-30 | 88 - polarity settled |
| `boostimg` | plain `Image`, `x=320 y=77` (header row), 12x12 at `U=480 V=32` of `pulse_assets.mip` | hidden by `ResetTable`, **shown by `PopulateLapTable` and by nothing else** - the header icon of the third column; the Tournament, Zone, Elimination and multiplayer tables never show it | 88 |
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
| 2 | `lap{n}.2` | a stored 16-bit value at `+0x10` - the speedup pads entered on this lap | decimal |

`docs/ui/campaign-screens.md`'s capture read column 2 as `6`/`9`/`10` summing
to a `25` total and called it "a pennant-icon column" on sight, and it was right:
**the icon is `boostimg`**. An earlier revision of this page read the decompile as
hiding `boostimg` unconditionally, from `|= 4`/`&= ~4` pairs read the wrong way
round; bit `0x4` is the *visible* bit (the correction is on the ghidra page and was
found from an unrelated live flag read), so `EndRaceResults_ResetTable` hides the
icon and `EndRaceResults_PopulateLapTable` shows it. The header cell `lap0.2` is
blanked, which makes the icon itself the column's header. **The column counts the
speedup pads the player entered on that lap, and its totals cell is their sum** (settled
2026-09-30, confidence 92: a live write watchpoint caught the per-lap field's only writer
inside `Ship_ApplySpeedupPad` - see [the ghidra page's "The third column counts
speedup pads entered"](../ghidra/functions/psp-pulse-usa/endrace-screens.md#the-third-column-counts-speedup-pads-entered-and-ship_applyspeeduppad-is-its-writer-2026-09-30)).
The captures' `6`/`9`/`10` (total `25`) and `6`/`10`/`8` (`24`) are pad counts on Talon's
Junction, which has about eleven pad positions on a lap; lap 1 reading `6` both times is
not explained. A weapon/pickup count was ruled out by both captures being `Weapons="off"`
cells, and a finishing position by exceeding the field size.

### One table, five fillings: which widgets each mode shows (2026-09-30)

The `table` group is one authored grid - `zonetopline`, `tablebg1`-`tablebg8` (four
widgets each: a dark fill, a `hex_bg.mip` tile and two fading rules, on a 20 px
pitch from `y = 92`), `topbarcenter`, the 27 `lap{r}.{c}` cells, `perfectlap1`-`5`,
`boostimg` and `tablehighlight` - and every mode fills it differently. Bit `0x4` of a
widget's flag word is the visible bit; `ResetTable` hides the rows and icons and
each populate shows what it uses. Read on the ghidra page's "The variant populates":

| Mode | Header row | Rows shown | `boostimg` | Highlight (at `y = 93 + 20 * (row - 1)`) | Cells |
| --- | --- | --- | --- | --- | --- |
| Time Trial, Speed Lap, single race | `RC_LAP` / `PRO_TIME` / *blank, `boostimg` on top* | one per lap (max 5) + a totals row (none for Speed Lap) | **shown** | the totals row (hidden for Speed Lap) | lap number, time, pads entered |
| Tournament | `PRO_POS` / `ER_TEAM` / `ER_POINTS` | one per craft | hidden | the player's row | position, team, points |
| Zone | none (`topbarcenter` hidden) | six | hidden | hidden | label in column 0, value in column 2 |
| Eliminator | `ER_DEATHS` / `ER_TEAM` / `IG_HUD_KILLS` | one per craft | hidden | the player's row | deaths, team, kills |
| Network play | `PRO_POS` / `PRO_NAME` / `PRO_TOT_TIME` | one per craft | hidden | the player's row | place, name, time |

`Line1` reads `ER_ZONE_COM` for Zone and `"%s %s"` of `ER_ELIM_COM` and the place's
ordinal for Eliminator; `Line2`-`Line8` are never filled in any mode.

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

### The loyalty award: `90 Points` is a decompiled law, confirmed on two independent live races

**`EndRaceRewards_Update` (`0x088dd2b8`) is where `RewardLoyaltyActive` and
`loyaltynum` actually get their text, not `OnEnter`.** Confidence 85,
decompiled in full and runtime-confirmed (below). Once the ticker's cycle
index reaches its own reason-string count (i.e. on the ticker's very first
"beat", before any reason string has had a turn):

- `RewardLoyaltyActive` <- `"%d %s"` of `*(g_endrace_result + 8)` and
  `ER_POINTS` - **this is `"90 Points"`, read off a field now traced to its
  writer, `Race_ComputeLoyaltyAward` - see below.**
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
a plain, medal-less, non-suggested-ship Time Trial - and, as of 2026-09-14,
this is no longer an inference from a name. `Race_ComputeLoyaltyAward`
(`0x0880ac50`, [ghidra page](../ghidra/functions/psp-pulse-usa/endrace-screens.md)),
called from the race-end summariser `Race_BuildEndRaceResult` (`0x0882a498`)
right after `Race_RecordResult`, is the writer of `g_endrace_result + 8` -
found by tracing `Race_RecordResult`'s own caller statically, exactly the
route the previous pass's own "find the writer" note pointed at. Its
decompiled law **is** the suffix-numbers-as-rates reading: `laps*15 +
perfectLaps*25` for the Race family, `laps*30 + perfectLaps*50` for
Time Trial/Speed Lap, `laps*10 + perfectLaps*20` otherwise, plus a per-kill
term and a Race-family-only difficulty multiplier (Easy x2/Medium x3/Hard
x4, doubled again if a suggested ship was offered). Runtime-confirmed on
two independently-driven `grid0_3_2` (Venom Time Trial) races at different
paces, both landing on the identical `laps=3, perfectLaps=0` shape and both
reproducing `award = 90` exactly - live-read off `g_endrace_result+8` on
the second race (`90`), and shown on both screens (`Assegai Loyalty: 90
Points`, `Total loyalty: 90`, up from `0` on this fresh profile's first
race). Confidence **95** for the law; see the ghidra page for the full
two-data-point table and the Race-family/Zone/Elimination branches this
pass still could not runtime-verify (both captures are Time Trial).

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

- ~~**The per-lap third column on `EndRace Results`**~~ **Settled 2026-09-30**: the number of
  speedup pads entered on the lap (see above). Still open: why lap 1 reads `6` on both
  captures.
- ~~**The `perfectlap{n}` flag's direction**~~ **Settled 2026-09-30**: nonzero
  shows the icon. What makes a lap "perfect" is not read (the bytes come from
  `craft+0x900 + lap*0x10 + 0x8c`).
- ~~**`boostimg`'s actual firing condition.**~~ **Settled 2026-09-30**: shown on the
  ordinary per-lap table alone - see the widget table. The Tournament, Zone,
  Elimination and multiplayer populates (`EndRaceResults_PopulateTournamentTable`,
  `..ZoneTable`, `..EliminationTable`, `..MultiplayerTable`) are all read and none
  shows it.
- **`EndRace Results`'s two-Redirect toggle** - which of `EndRace Rewards`/
  `Race End Save` fires is decompiled at a structural level only; the exact
  bit arithmetic was read once, not independently re-derived or
  runtime-checked.
- ~~The loyalty-award computation itself~~ **Closed 2026-09-14** - the
  writer is `Race_ComputeLoyaltyAward` (`0x0880ac50`), decompiled in full
  and confirmed on two independent live races - see above.
- **`Endrace Difficulty`'s own binding** was not traced - likely the generic
  `<List global=...>` mechanism `race-setup.md` already documents for
  `Single Player`'s own difficulty row, not re-verified here.
- **The five support screens** (`EndRaceAutoSaveDisabled/Failed`,
  `EndRaceSaveGhost`, `EndRaceDeleteGhost`, `TournySaveWarning`,
  `Kill Game Transition`) are read directly off the XML above (all plain
  `Dialog`/`MemoryStick` blocks, no widgets needing a code binding) and not
  decompiled further - nothing in their own XML shape needed it.
- **Two no-medal Time Trial captures now exist** (both `grid0_3_2`), which
  settled the loyalty law and ruled out weapon/pickup readings for the
  pennant column, but **neither is a medal-earning run** - the `MedalImg`
  no-medal-only question is still open, and the Race-family/Zone/Elimination
  loyalty branches are still decompiled-only.
