# `EndRace Results`, `EndRace Rewards`, `EndRace Menu`, decompiled

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, `pulse-psp-usa.chd`, UCUS-98712),
image base `0x08804000`. This is the code half of
[`docs/formats/endrace-screens.md`](../../../formats/endrace-screens.md), which
reads the XML these functions fill in; that page has the widget tables, this
one has the addresses and the evidence. **Runtime-verified so far**: the single `grid0_3_2` Time Trial capture
`docs/ui/campaign-screens.md` holds (twice), the loyalty law on two live races, and - added
2026-09-30 - an Eliminator race's whole `EndRace Results` table, the `+0x2c` visible-bit
polarity, and the third column's writer (each named at its own section). Everything else on
this page is a decompile, and its score is capped accordingly.

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
| `Update` (word 9) | `0x088da530` (decompiled 2026-09-30, see below) | `0x088dd2b8` | `0x088d8338` (not decompiled this pass) |
| `OnEnter` (word 29) | `0x088d98cc` | `0x088dbbd4` | `0x088d81dc` |
| `OnExit` (word 31) | `0x088d92e0` (not decompiled this pass) | `0x088dbb5c` (not decompiled this pass) | `0x088d8248` (not decompiled this pass) |
| `CommitSelection` (word 39) | `0x088902c8` (shared base, not overridden) | `0x088902c8` (shared base) | `0x088902c8` (shared base) |

## `EndRace Results`

| Address | Name | Conf | Role |
| --- | --- | --- | --- |
| `0x088d98cc` | `EndRaceResults_OnEnter` | 82 | sets `BigTopText`/`Line1`, dispatches to the per-mode populate helper, toggles the two outgoing Redirects |
| `0x088da8c8` | `EndRaceResults_PopulateLapTable` | 80 | the single-player (`Race`/`Time Trial`/`Head2Head`/`Speed Lap`) per-lap table |
| `0x088d939c` | `EndRaceResults_ResetTable` | 88 | blanks all 27 `lap{r}.{c}` cells, **hides** the 8 `tablebg` rows, the 5 `perfectlap` icons and `boostimg`, and shows `topbarcenter` - see "Bit `0x4` is the visible bit" below (an earlier revision read every one of these backwards) |
| `0x088da530` | `EndRaceResults_Update` | 85 | the per-frame update: accumulates the screen timer at `+0xdc`, plays the finishing-place jingle once, flips the Tournament page every 3 s, and re-runs the multiplayer populate every frame in modes `0xe`/`0xf` |
| `0x088dad90` | `EndRaceResults_PopulateTournamentTable` | 82 | Tournament and Multiplayer Tournament - read on [`tournament.md`](tournament.md) |
| `0x088db574` | `EndRaceResults_PopulateZoneTable` | 82 | Zone: six label/value rows, no header - see "The variant populates" below |
| `0x088db1ec` | `EndRaceResults_PopulateEliminationTable` | 92 | Eliminator and Multiplayer Elimination: `Deaths`/`Team`/`Kills` per craft, **read on a live frame** |
| `0x088d9588` | `EndRaceResults_PopulateMultiplayerTable` | 78 | the network-play variant of the ordinary table: `Pos`/`Name`/`Total Time` per craft; gated on `+0xe9`, which is `g_game_mode > 0xd` and not split-screen |

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
| `+0x10` | 2 bytes | `lap{n}.2` | decimal - **the speedup pads entered on the lap** (see "The third column counts speedup pads entered" below) |
| `+0x12` | byte (flag) | **shows** `perfectlap{n}` when nonzero (bit `0x4` is the visible bit) | - |

The totals row (`tablebg{n+1}`, `n` = laps shown) reads
`g_endrace_result + 0x114c` (a time) and `g_endrace_result + 0x1148` (a
separately-tracked aggregate, not summed client-side) under the `PRO_STATS_TOT`
label - confirmed by the aggregate reproducing the capture's own `25` from
`6+9+10` exactly.

### Bit `0x4` is the visible bit, and the earlier reading had it backwards (2026-09-30)

Every widget on these screens carries its state in a flag word at `+0x2c`, and
**bit `0x4` set means drawn**. The evidence is independent of these screens:
on a running PPSSPP Eliminator race, `Hud_UpdateTimeCluster` clears `0x4` on
`TotalTime`/`TotalTimeTxt` and their flag words read `0xb082` (hidden, and absent
from the frame) against `0xf086` on `CurrentTime` (drawn, and present) - see
[`race-progress.md`](race-progress.md#which-modes-hide-the-clock-confirmed-live-2026-09-30).
This page and its sibling formats page had read `|= 4` as hide and `&= ~4` as show
for these three functions. Corrected, on the same decompiles:

- `EndRaceResults_ResetTable` **hides** `tablebg1`-`tablebg8` (`&= ~4`),
  `perfectlap1`-`perfectlap5` and `boostimg`, and **shows** `topbarcenter`
  (`|= 4`). It also blanks every `lap{r}.{c}` cell. Nothing in it un-hides a row.
- `EndRaceResults_PopulateLapTable` then **shows** `boostimg` (`|= 4`, at
  `0x088daa00`), shows `tablebg{i}` for each lap row it fills and for the totals
  row, and shows `perfectlap{i}` only `if (lap.+0x12 != 0)`. So **`boostimg` is
  visible on every ordinary per-lap table**, which is what the one live capture
  shows (`results-01.png`: a pennant glyph at `x=320 y=77`, the header cell of the
  third column) and what `docs/ui/endrace-screens.md` had flagged as disagreeing
  with the decompile. The decompile never disagreed: it was misread. The direction
  of the `perfectlap{i}` flag is settled by the same correction - a **nonzero**
  byte shows the "perfect lap" icon, and `Race_BuildEndRaceResult` counts the same
  bytes into the perfect-lap total the loyalty law multiplies by 25/50.
- **`boostimg` is written by exactly two functions**, `ResetTable` (hide) and
  `PopulateLapTable` (show); `get_xrefs_to` on its string (`0x08a82d84`) returns
  those two and nothing else. The Tournament, Zone, Elimination and multiplayer
  populates call `ResetTable` and never show it again, so it is drawn on the
  ordinary per-lap table **only**. `Line2`..`Line8` are written by
  `EndRaceResults_OnEnter` alone (`get_xrefs_to` on `0x08a82e34` and `0x08a82e64`
  each return `OnEnter` and nothing else), and only ever with the empty string
  (`&DAT_08a82d38`): they are authored and never filled, in any mode.
  Confidence **88** for "never shown / never filled" - a widget looked up by a
  name built at runtime would not appear in the cross-references, and no
  `Line%d`-shaped format string exists for this screen.
- The tail of `PopulateLapTable` moves `tablehighlight` to `y = laps * 0x14 + 0x5d`
  (`93 + 20 * laps`) onto the totals row, or hides it for Speed Lap (mode 10) which
  has no totals row. That is the same `93 + 20 * (row - 1)` the Tournament table
  uses, one pixel below where `tablebg_y` puts the row background.
- `boostimg` is the header **icon** of the third column. That column's own header
  cell (`lap0.2`) is blanked, so the icon *is* the header - which is the strongest
  single hint yet at what the column counts: a boost. The values themselves (`6`,
  `9`, `10` on the first capture) remain unexplained; see "Open".

Confidence **88** for the polarity (a single live measurement of the flag on a
widget that is provably hidden, plus one frame that shows `boostimg` on the
per-lap table) and for every consequence above that is a direct read of a
`|= 4`/`&= ~4` pair.

## The third column counts speedup pads entered, and `Ship_ApplySpeedupPad` is its writer (2026-09-30)

Confidence **92**. The column headed by `boostimg`
(`EndRaceResults_PopulateLapTable`'s `lap{n}.2`, the 16-bit per-lap field at `session +
0x900 + lap * 0x10 + 0x94`) is **the number of speedup pads the player's craft entered on
that lap**, and its totals cell is the sum (`+0x1148`, accumulated by
`Race_BuildEndRaceResult`'s loop over the copied laps).

- **The writer, from a live write watchpoint.** PPSSPP, a Time Trial on Talon's Junction
  (Venom, Assegai): write watchpoints armed on the six per-lap slots `craft+0x994 + 16 * i`
  with `craft = *(*(0x08b317b4) + 0x2c0)`, plus a control on `craft+0x920` (the running lap
  time, which must fire every tick: **3,440 hits**, so a zero elsewhere is a zero). The craft
  was driven along the committed lap script; slot 0 was written **twice**, both at
  `PC = 0x0884910c` - the `sw a1, 0(a0)` at the tail of the counter increment inside
  `Ship_ApplySpeedupPad` (`0x08848f9c`), the function that applies the pad boost - and no
  other slot and no other PC was ever hit. That is the field's only writer in this run, and
  the store follows the load-add-store sequence `0x08849104`-`0x0884910c` exactly.
- **The condition, from the decompile.** Inside the new-pad branch (the pad `Pads_TestCraft`
  returned is not the one latched at `craft+0x1d0` - the edge that also arms
  `ExhaustFlare_OnSpeedupPad` and, in Zone, raises the 100-point flag), when the craft's
  entity has `+0x368 == 0` - the local player, by the five converging uses on
  [`pads.md`](pads.md) - it does `+0x8d0 += 1` (a per-race pad total), bumps the profile-side
  counter at `DAT_08b31774 + 0x11c` when `FUN_08809b38()` says so, and, when the craft's lap
  counter `craft + 0xac8` minus one is below `0x14`, adds one to `craft + 0x900 +
  (lap - 1) * 0x10 + 0x94`. **One count per pad entry, not per tick**, and an AI craft's laps
  never count.
- **The data agrees.** Talon's Junction has 17 speedup-pad volumes in about eleven places
  round the lap (`oag-trace pads`: several sit in pairs a few units apart, and a few are on the
  alternative path), against the captures' `9`, `10`, `10`, `8` on full laps. The first lap reads
  `6` in both captures; why is not determined - the count is only bumped while `craft+0xac8 - 1`
  is a valid slot, and this project has not pinned how `+0xac8` moves at the start line.
- **Earlier readings retired.** The candidates on record were a finishing position, weapon
  and pickup counts (both ruled out) and "a boost/speedup-pad count, unconfirmed"; the last is
  the answer. The `boostimg` icon is the pad's own glyph.

## The variant populates (2026-09-30)

All four run after `EndRaceResults_ResetTable` (so every `tablebg`, `perfectlap` and
`boostimg` starts hidden and every cell blank) and each begins by showing the
`table` group (`|= 4`). They are selected by `EndRaceResults_OnEnter`'s switch:
Zone (6) runs the Zone populate; Elimination and Multiplayer Elimination (8, `0x12`)
run the Elimination populate; Tournament and Multiplayer Tournament (4, `0x10`) the
Tournament one; every mode in `{3, 5, 9, 10, 0xe, 0xf, 0x11}` runs the lap table, or
the multiplayer table when `g_game_mode > 0xd`.

### `EndRaceResults_PopulateEliminationTable` - `0x088db1ec`, read on a live frame

Confidence **92**. Header cells: `lap0.0` = `ER_DEATHS` ("Deaths:"), `lap0.1` =
`ER_TEAM` ("Team"; `PRO_NAME` when `+0xe9` is set, i.e. network play), `lap0.2` =
`IG_HUD_KILLS` ("Kills"). One row per craft in the field (`DAT_08b30f90` of them),
in the order `Race_BuildEndRaceResult` left them. Row `r`, from the per-craft
record at `g_endrace_result + 0x110 * (r - 1)`:

| Cell | Source | Format |
| --- | --- | --- |
| `tablebg{r}` | - | shown |
| `lap{r}.0` | `+0x13c`, the craft's deaths (`craft+0x8d4`) | `%d`, or `ER_DNF` when `+0x140 == -1` |
| `lap{r}.1` | the localised team name at `+0x35` (`localise(craft+0x798)`) | text |
| `lap{r}.2` | `+0x138`, the craft's kills (`craft+0x8d8`) | `%d`, or `ER_DNF` when `+0x140 == -1` |
| `tablehighlight` | moved to `y = 0x5d + 0x14 * (r - 1)` when `+0x34` (the player flag) is set | - |

`Line1` is `"%s %s"` of `ER_ELIM_COM` ("Eliminator complete - ", trailing space
included, hence the double space on screen) and `ER_1STP`..`ER_8THP`, for the place
in `+0x0`; outside 1..8 `Line1` stays blank. **`Race_BuildEndRaceResult` sorts the
records before this runs** (a bubble sort at `0x0882a498`, mode 8 only): kills
descending, then deaths ascending, then a full tie puts the player's record first
(the swap tests `next.player != 0`); any other tie keeps the order
`FUN_08826d80` returned the crafts in. The place is the player's index after the
sort. The `+0x140` word is never written on this path - it read `0` on all eight
records - so `ER_DNF` is unreachable in a single-player Eliminator and is not drawn
by this build.

**Live measurement**, PPSSPP v1.20.4, `pulse-psp-usa.chd`, Eliminator on Talon's
Junction, the player parked so the AI reached the 5-kill target in 85 s. The
struct was dumped at `Race End Photo` (`*(0x08b317b4) + 0x7d8`): `place = 8`,
records `EG-X` 5 kills / 5 deaths, `Piranha` 4/1, `Goteki 45` 3/1, `Qirex` 3/3,
`AG Systems` 2/4, `Feisar` 2/4, `Triakis` 1/2, `Assegai` (`+0x34 = 1`) 0/1, all
`+0x140 = 0`. The `EndRace Results` frame reads, top to bottom, `RESULTS`,
`ELIMINATOR COMPLETE -  8TH PLACE`, header `Deaths: | Team | Kills` at `x = 140 /
220 / 320`, and the same eight rows in the same order with the player's row
highlighted, static over a 6 s watch (no page cycling, unlike Tournament). Every
cell of the table is accounted for by the row above. Sort law, columns, header,
`Line1` wording and highlight are **confirmed**; the frame shows the team names
as display names (`Goteki 45`, `AG Systems`), so the localise call is a string-table
lookup of the team's folder id.

### `EndRaceResults_PopulateZoneTable` - `0x088db574`

Confidence **82** (decompile plus the writers of the source fields; no Zone frame -
Zone is greyed on a fresh profile and a forced `g_game_mode = 6` hangs the loader).
`table` shown, `topbarcenter` **hidden** (there is no header row), `tablebg1`-
`tablebg6` shown, and `Line1` = `ER_ZONE_COM` ("Zone session complete!"). After the
populate `OnEnter` hides `tablehighlight`. Six label/value rows; the label goes in
`lap{r}.0` and the value in `lap{r}.2`, `lap{r}.1` stays blank:

| Row | Label (`idstring`) | Value | Source: the Zone mode object's stats block |
| ---: | --- | --- | --- |
| 1 | `ER_ZONE_CLEAR` "Total zones cleared:" | `%d` | `+0x1a10` u16, the zone number - `Zone_Update` adds 1 per 10 s step |
| 2 | `ER_PERF_ZONE` "Perfect zones:" | `%d` | `+0x1a12` u16 - `Zone_Update` adds 1 at a zone step when the "dirty" byte `+0x1a28` is clear |
| 3 | `ER_LAPSC` "Laps cleared:" | `%d` | `+0x1a14` u16 - `Zone_Update` adds 1 per call while `craft+0x911` bit 0 is set and `craft+0xacc > 2` |
| 4 | `MSC_DATA_PLAP` "Perfect laps:" | `%d` | `+0x1a16` u16 - `Zone_Update` adds 1 (and 2000 score) when `craft+0x860 & 0x200000` |
| 5 | `ER_TOP_SPEED` "Top speed:" | `"%d %s"` of `top * 0xe10 / 100000` and `RC_KMH` | `+0x1a1a` u16, the running maximum of `craft->+0x94->+0x2ec * 100` |
| 6 | `ER_ZONE_SCORE` "Zone score:" | `%d` | `+0x1a1c` s32 - 1 per call, 500 per zone step, 500 more for a clean zone, 2000 per row-4 event, 100 per new speedup pad |

`Race_BuildEndRaceResult` copies the block through the mode object's virtual at
`vtable+0x74` into `g_endrace_result + 0x1134` (zones), `+0x1138` (perfect zones),
`+0x1140` (row 3), `+0x1144` (row 4), `+0x1150` (top speed) and `+0x113c` (score).
The Zone page's older table called `+0x1a16` "laps completed" and `+0x1a14` "a
counter stepped on a lap-like condition"; the end screen's own labels say row 3 is
*laps cleared* and row 4 *perfect laps*, and the `0x200000` flag that feeds row 4 is
also worth 2000 points, so that is a bonus event and not a plain lap count.
What `craft+0x911` bit 0 and `craft+0x860 & 0x200000` are is **not determined**, which
is why this build cannot honestly fill rows 3 and 4.

### `EndRaceResults_PopulateMultiplayerTable` - `0x088d9588`

Confidence **78**. Not split-screen: it is entered when the screen's `+0xe9` byte is
set, and `OnEnter` sets that byte to `g_game_mode > 0xd` - the network-play family,
modes `0xe`, `0xf`, `0x10`, `0x11` and `0x12`. It fills the same `table` with a
`PRO_POS` / `PRO_NAME` / `PRO_TOT_TIME` header and one row per craft: the place
(`%d`), the name at `+0x35`, and the finishing time at `+0x140` - `ER_DNF` for `-1`,
`ER_RACING` for `0`, and otherwise the time through `FUN_08819878`. The player's row
takes the highlight. `EndRaceResults_Update` calls it every frame in modes `0xe` and
`0xf`, so the times fill in as other players finish. This build has no network play,
so nothing draws it.

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
loyaltybar width (+0x9c) and U extent (+0xc4) <- *(record + 8) * 0.00124   # PIXELS, see "The loyalty bar's fill" below
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
- ~~**The per-lap `+0x10` column's own writer, during the race**~~ **Closed 2026-09-30**:
  `Ship_ApplySpeedupPad` at `0x0884910c`, a speedup-pad entry count per lap - see "The third
  column counts speedup pads entered". Open inside it: why lap 1 reads `6` against `9`-`10`
  on the laps after it in both captures (how `craft+0xac8` moves at the start line is not
  pinned), and what `craft+0x368` is (named only as "the local player" by convergence).
- ~~**`0x088d7e1c`'s own `+0xbe` gate**~~ **Ruled out 2026-09-14** -
  `0x088d7e1c` is the held-confirm variant used by `InGame Photo`, not
  `Cell Selection`'s own dispatcher; see "`0x088d7e1c` ruled out..." above.
  The real dispatcher (`ConfirmButton_Update`, `0x088c8e88`) is found and
  cleared too - the remaining gate is upstream of it, in input consumption,
  not a `+0xbe`-shaped flag.
- **`EndRaceResults`/`EndRaceMenu`'s own `Update`/`OnExit` slots**
  (`0x088da530`, `0x088d92e0`, `0x088d8338`, `0x088d8248`) were positionally
  located (vtable word 9/31) but not decompiled this pass.
- ~~**The Tournament/Zone/Elimination/split-screen populate helpers**~~ **Closed
  2026-09-30**: all four are read - see "The variant populates". The Elimination
  table is confirmed on a live frame; the Zone table is decompile-only (no Zone
  frame is reachable) and the multiplayer table is not runnable here. `boostimg` is
  shown on the ordinary lap table only, and `Line2`..`Line8` are never filled.
  Still unknown inside them: what `craft+0x911` bit 0 and `craft+0x860 & 0x200000`
  are (Zone's "Laps cleared" and "Perfect laps"), and who writes the `+0x140`
  word on the Eliminator path (it reads `0`, and `-1` would print `DNF`).
- **`EndRaceResults_OnExit` and `EndRaceMenu`'s `Update`/`OnExit`** (`0x088d92e0`,
  `0x088d8338`, `0x088d8248`) remain undecompiled; `EndRaceResults_Update`
  (`0x088da530`) is read.
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
| `0x088d939c` | `EndRaceResults_ResetTable` | 88 |
| `0x088da530` | `EndRaceResults_Update` | 85 |
| `0x088dad90` | `EndRaceResults_PopulateTournamentTable` | 82 |
| `0x088db574` | `EndRaceResults_PopulateZoneTable` | 82 |
| `0x088db1ec` | `EndRaceResults_PopulateEliminationTable` | 92 |
| `0x088d9588` | `EndRaceResults_PopulateMultiplayerTable` | 78 |
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
| `0x088a6a9c` | `Image_DenormalizeUv` | 80 |
| `0x088a68ec` | `Image_NormalizeUv` | 80 |

## The loyalty bar's fill: pixels, not a fraction (2026-10-02, `pulse-loyaltybar`)

Headless decompile of `EndRaceRewards_Update` (`0x088dd2b8`) and the two helpers it
brackets the bar's write with. The earlier reading ("fill fraction = `total * 0.00124`")
took the product for a fraction of the authored width; it is the width itself.

```
bar.+0x9c   <- 0                                   # start of the tick block, width cleared
...
bar.+0x9c   <- (float) total * 0.00124             # width, pixels
Image_DenormalizeUv(bar)   # 0x088a6a9c  U/V ((+0xbc,+0xc0), (+0xc4,+0xc8)) *= texture size, flag +0xd0 = 0
bar.+0xc4   <- (float) total * 0.00124             # U extent, texels
Image_NormalizeUv(bar)     # 0x088a68ec  UVs /= texture size, flag +0xd0 = 1; +0x9c, +0xa0 kept
```

- `Image_NormalizeUv` defaults `+0x9c`/`+0xa0` to the texture's own width and height when
  they hold `FLT_MIN` (`1.1754944e-38`) and divides `+0xbc..+0xc8` by the texture size, so
  `+0x9c`/`+0xa0` are the width and height in pixels and `+0xc4`/`+0xc8` the U/V extent
  in texels before normalisation. Confidence 80 for the two names (the arithmetic is
  unambiguous; the field names are read off it).
- **A second, independent call site agrees**: `FEScreen_SetStatBar` (`0x088ea6b8`, the
  `TeamSelection` stat bars) computes `(scale * value) / max` and writes the same number
  to `+0x9c` and `+0xc4` around the same pair. There `scale` is the bar's full width in
  pixels, so a bar is `fullWidth * fraction` pixels wide and as many texels of the sheet:
  cropped, not squashed.
- The loyalty bar passes no `scale`: `total * 0.00124` is already pixels, and
  `Loyalty_AccumulateTotal`'s ceiling of `100000` gives `100000 * 0.00124 = 124`, exactly
  the authored `width="124"`/`TxtrWidth="124"`. That is the only reason it reads as a
  fraction, and the bar only fills at the cap.
- At the live capture's total of `90` the bar is `0.1116` pixels wide. The reference frame
  (`results-02.png`) shows exactly that: every one of the twenty segments reads one
  uniform ~130 (the `loyaltybg` image, `Color=0x7fffffff`, half alpha over the dark band),
  with no brighter column anywhere. The earlier note that the reference showed the "whole
  bar lit" mistook that background for fill.

Confidence **88** for the width law from the decompile and the one point near zero;
**raised to 93 on 2026-10-02 (`pulse-cursor-live`)** by two live points on the slope, below.

### The slope, measured at two totals (2026-10-02, `pulse-cursor-live`)

PPSSPP v1.20.4, `pulse-psp-usa.chd`, Xvfb, a campaign Time Trial (`grid0_3_2`, Assegai) finished with the game's own
autopilot pickup (`psp-postrace.py`'s method: fire word `|= 0x1000`, countdown `record+0x148` raised) and
`g_race_laps` written to 1 to shorten it. A breakpoint at `Loyalty_AccumulateTotal` (`0x08807884`) wrote the team
record's `+8` (`a0 + 8`; it read 0 on the first race and 50080 on the second) to a chosen value just before the
award is added, so the screen reads `chosen + award` (the award read 80 at the hit, from `a1`; it was not decomposed).
`EndRace Results` was then stepped through with `Cross`, and `EndRace Rewards` captured at 960x544.

| Poked, then total shown | Predicted fill | Bright fill measured (row y=241, runs brighter than 200) | Our render, same total |
| --- | --- | --- | --- |
| 50080 (`Total loyalty: 50080`) | 62.1 px = 124.2 screen px | 681..803, ten full segments and the 11th cut after 3 px: **123 px** | 680..803, the same cut: **123-124 px** |
| 20080 (`Total loyalty: 20080`) | 24.9 px = 49.8 screen px | 681..729, four full segments and a 1 px sliver: **49 px** | not rendered |

Both land within one screen pixel of `total * 0.00124` PSP pixels (the second point's slope against the first,
`(123 - 49) / 30000 = 0.00247` screen px per point, is `0.00123` PSP px). The bar is cropped, not squashed: the
11th segment is cut mid-bar and the segment pitch is unchanged. Our render comes from `--menu-page
endrace-rewards-gold` with the page's hardcoded award and total temporarily set to 80 and 50080 (not committed), laid
next to the capture: ten full segments and a half-cut 11th in both, the same edge to within a pixel. Preconditions that are pokes, not the game's own path: the
record's `+8` was written, and `g_race_laps` was shortened; the displayed total therefore says nothing about earning
50000 points. Each point was seen once (one race each). Confidence **93**
for the width law and slope (a runtime trace, one binary); the pre-ticker visibility below is still unchased.

Visible bit: `loyaltybar`, `loyaltybg` and `loyaltynum` get `+0x2c |= 4` only inside the
end-of-ticker branch (`cycle == reasonCount`), so the original probably keeps them hidden
until the ticker finishes; this build draws them from the first frame. Not chased.
