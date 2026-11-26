# Detonator's end-of-race screen: rows, values, formats, and what the record uses

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`). Static reading, 2026-11-22; checked live
2026-11-26 ("Checked against the original"). Every `lwz rN,disp(r2)` was resolved with
`scripts/ps3-toc.py resolve <function> 0x<disp>` (all the functions below declare TOC
`0x008ad4d8`, `ps3-toc.py toc` prints `exact`; displacements are hex, and a decimal `13192` read
as hex resolves a different string), store and load sites by `scripts/ps3-dform-scan.py`, bodies
Ghidra truncates by `llvm-objdump -d --mattr=+altivec,+vsx`. Read first:
[endrace-results-grid.md](endrace-results-grid.md) (the grid frame and the race rows),
[endrace-loyalty.md](endrace-loyalty.md) (the stats block at `RaceManager+0x1980`),
[detonator-cannon.md](detonator-cannon.md) (the score, the percentage, the streak counts),
[cell-medal-award.md](cell-medal-award.md) (the end-of-race builder and the record call).

## The answer in one table

Mode `0xe` (Detonator, `g_GameState+0xe0`) draws `Line1` plus one grid of six label/value rows and a
score line. The filler is `0x002234b0`; it has **no finished/destroyed branch**, it prints the same
rows from a record the end-of-race builder `0x0005ac00` filled. Confidence **80** (every row is a
string reference and a load at instruction level; Omega's executable has the same code; two live frames).

| Row (widgets) | Label id (HD English, `DATA06`) | Value format | Value read | Source in the race | Conf |
| --- | --- | --- | --- | --- | ---: |
| 0 (`Grid0.0`, `Grid1.0`) | `ER_DETONATOR_STAGE` = `STAGE CLEARED` | `%d / %d` | `rec+0x1330` (int), then the literal `14` | **laps completed**, `craft+0x7810 - 1` clamped `0..99` (**not** the HUD stage `rm+0x2dee`) | 80 |
| 1 (`Grid0.1`, `Grid1.1`) | `PERCENT_MINES_CLEARED` = `MINES CLEARED` | `%.1f%%` | `rec+0x131c` (f32) | `rm+0x2ddc`: `rm+0x6de3c / 1860 * 100` | 80 |
| 2 (`Grid0.2`, `Grid1.2`) | `PERCENT_BOMBS_CLEARED` = `BOMBS CLEARED` | `%i / %i` | `rec+0x1320` (u16, `lhz`), then the literal `14` | `rm+0x2de0` = bombs destroyed `rm+0x6de40`, a count, not a percentage | 80 |
| 3 (`Grid0.3`, `Grid1.3`) | `ER_NUM_5_STREAKS` = `NUMBER OF 5 CHAINS` | `%d` (`0x007940f0`) | `rec+0x1328` (u16) | `rm+0x2de8` | 78 |
| 4 (`Grid0.4`, `Grid1.4`) | `ER_NUM_10_STREAKS` = `NUMBER OF 10 CHAINS` | `%d` | `rec+0x132a` (u16) | `rm+0x2dea` | 78 |
| 5 (`Grid0.5`, `Grid1.5`) | `ER_NUM_15_STREAKS` = `NUMBER OF 15 CHAINS` | `%d` | `rec+0x132c` (u16) | `rm+0x2dec` | 78 |
| header (`Grid0.h`, `Grid1.h`) | `ER_DETONATOR_SCORE` = `TOTAL SCORE` | `%d` | `rec+0x1314` (int) | the score `rm+0x2df0` | 82 |
| `Line1` | `ER_ZDETONATOR_COM` = `DETONATOR SESSION COMPLETE!` | none | the id, through the localising setter | none | 80 |
| `RecValue` | `ER_NSR` = `NEW SCORE RECORD`, else `ER_SR` = `SCORE RECORD` | `%s %d` | `rec+0x1358` | **always written**: the new score with the flag `rec+0x1357` set, else the standing best ("The record notice") | 90 |

`rec` is `screen+0xb34` = `RaceManager_GetInstance() + 0x1980`
(`EndRaceResults_OnEnter`, `0x0022e3d0`: the decompile stores `RaceManager_GetInstance() + 0x1980` at `screen+0xb34`; the store is `stw r9, 0xb34(r31)` at `0x0022e414`, found with `ps3-dform-scan.py --disp 0xb34`), so
`rec+X` is `rm+0x1980+X`. The HUD stage `rm+0x2dee` is copied to `rec+0x1318` (`rm+0x2c98`) and **the
filler never reads it**.

## Where the record comes from: `0x0005ac00` copies the mode's record

`0x0005ac00(rm, force)` is the single-player end-of-race builder
([endrace-loyalty.md](endrace-loyalty.md): not renamed here either; only its Detonator reads are
read). With `r24` the result of the race manager's vtable slot `+0x3c` called with `0` (`0x0005b17c`-`0x0005b190`,
the call `detonator-cannon.md` reads for the score: `SPDetonator`'s slot `0x0067dc10` is `addi r3,r3,0x2ddc; blr`, the record at
`rm+0x2ddc`) it copies, at `0x0005b19c`-`0x0005b1d8`:

| Source (`rm+0x2ddc` record) | Destination | `rec+` | What the filler does |
| --- | --- | --- | --- |
| `+0x14` int (`rm+0x2df0`, the score) `lwz` / `stw` | `rm+0x2c94` | `0x1314` | prints it in `Grid1.h` |
| `+0x12` u16 (`rm+0x2dee`, the HUD stage) `lhz` / `sth` | `rm+0x2c98` | `0x1318` | unread by the screen |
| `+0x00` f32 (`rm+0x2ddc`, the percentage) `lfs` / `stfs` | `rm+0x2c9c` | `0x131c` | row 1 |
| `+0x04` int (`rm+0x2de0`, bombs) `lwz` / `sth` (truncated) | `rm+0x2ca0` | `0x1320` | row 2 |
| `+0x08` f32 `lfs` / `stfs` | `rm+0x2ca4` | `0x1324` | unread by the screen |
| `+0x0c`, `+0x0e`, `+0x10` u16 `lhz` / `sth` (the three streak counts) | `rm+0x2ca8`, `+0x2caa`, `+0x2cac` | `0x1328`, `0x132a`, `0x132c` | rows 3, 4, 5 |

The other record the builder copies, from `vtable[+0x38]` (arg `0` and `1`, to `rm+0x2c88`..`+0x2cc0`
and `+0x2ce0`..), is **null for `SPDetonator`**: its slots `+0x38` and `+0x40` are `li r3,0; blr`
(`0x0067d010`, `0x0067d020`; the vtable is at `0x008629c0`, slot `+0x3c` at `0x008629fc`), so the
builder skips that copy (`cmpwi cr3,r3,0` at `0x0005b0d8`) and the stats block carries no lap or time
cells from it.

**The stage row is laps completed.** `rec+0x1330` = `rm+0x2cb0` is written by the builder's own lap
code, not by the Detonator record: `0x0005ae80` `addi r5, [craft+0x7814], -2` (`r20 = craft+0x6ff0`, so
`+0x824` is `craft+0x7814`, the crossing count plus one) with the stores at `0x0005ae84` and, in the
`force == 0` form, `[craft+0x7810] - 1` at `0x0005b584`; both are the crossing count minus one,
clamped `0..99` (`0x0005ae98`, `0x0005b548`). That is the value `endrace-loyalty.md` gives as "laps
completed" for the same field. In Detonator the crossing that starts the race is count `1`, each lap
adds one, and the end path runs at the 15th crossing ([detonator-mode.md](detonator-mode.md): stage
`15` at the end of lap 14), so a finished race reads `14 / 14`; a craft destroyed during stage `k`
(crossing count `k`) reads `k-1 / 14`. The `14` is a literal (`li r6,0xe` at `0x00223604`, the same
`li r6,0xe` at `0x002236f8`), not the table's stage count. Confidence **80**.

## The layout (mode `0xe` branch of `EndRaceResults_LayoutGrid`, `0x0022c068`)

The branch is reached when `g_GameState+0xe0 == 0xe` (the dispatch at `0x0022c11c`-`0x0022c128`
tests `6`, then `0xe`, then `0xd`; the byte at `0x009384e1` (`F`) must be clear) and calls the
filler at `0x0022e240` after `EndRaceResults_ResetGrid` (`0x0022e238`), then the record tail
`0x0022bdb8`. What it does to the frame (the decompile of the branch, TOC indices resolved with
`ps3-toc.py resolve 0x22c068 0x<4*index>`):

| Widget | Change |
| --- | --- |
| `Line1` | the idstring `ER_ZDETONATOR_COM` (TOC index `0xcf9`), through `_opd_FUN_00202f58(widget, id, 0, 1)` (the localising setter) |
| `RecordNotifyBlock`, `GridTopBar`, `GridBottomBlock` | shown (bit `4` of `+0x34` set) |
| `RecLogoD` | shown; `RecLogoSL`, `RecLogoTT`, `RecLogoZ` hidden (the Detonator logo) |
| `GridSideBarL`, `GridSideBarR` | `y = 29.0` (`0x008b08d0`), `+0xb0` (height) `347.0` (`0x008b08a0`) |
| `GridBottomBar` | `y = 391.0` (`0x008b08a4`) |
| `ZBGridSideBarL` | hidden |
| `GridHead1`..`GridHead4` | hidden (no column captions; the labels are the rows' own) |

`347.0` and `391.0` are the numbers the file authors and the Time Trial / Speed Lap branch writes back
([endrace-results-grid.md](endrace-results-grid.md)); Zone's branch writes the same `29.0`.

The filler `0x002234b0` then, in order:

1. Places `Grid0.r` at `x = 40.0` (`0x42200000`) and `Grid1.r` at `x = 590.0` (`0x44138000`,
   `0x008b05e4`, `lfs f30,0x310c(r2)`) for `r = 0..8` (nine pairs), `y = 37 + 40 r` (`li r26,0x25;
   addi r26,r26,0x28`), widget fields `+0xb0` (x) and `+0xb4` (y).
2. Per row, the label widget `Grid0.r` gets the idstring through the localising setter
   `0x00202f58(widget, id, 0, 1)`, the value widget `Grid1.r` a literal string through
   `0x00202d28(widget, buf, 0, 0)`; `buf` comes from `FUN_006761b8` (`sprintf`). The six rows are the
   table above, `r = 0..5`; rows `6..8` are placed and never written.
3. `Grid0.h` (the score label, `ER_DETONATOR_SCORE`) at `x = 40.0`, `y = 360.0` (`0x43b40000`),
   `Grid1.h` (the score) at `x = 590.0`, `y = 360.0`.

`EndRaceResults_ResetGrid` ran first and left every `Grid` cell cleared and inked `0xff646464`; the
filler sets no colour, so the rows draw in that grey like the race rows. Whether a widget aligns its
text right (the values at `x = 590`, the rows' right edge) is its own flag word, not read here.

## The strings, and a correction: HD does author every id

All nine ids are in `DATA06`'s `/data/plugins/languages/english/entries.xml`, and in all sixteen of
its language tables (`scripts/psarc.py cat data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/DATA06.PSARC
/data/plugins/languages/<language>/entries.xml | grep -E 'ZDETONATOR|ER_DETONATOR|PERCENT_(MINES|BOMBS)|ER_NUM_|ER_NSR'`
prints 9 lines for each of the sixteen). `DATA02`, `DATA03`, `DATA04` and `DATA05` carry **none** of
the eight Detonator ids (checked in their English and German tables; `ER_NSR`, `ER_TT_COM` and
`ER_ZONE_COM` are in `DATA02`), the same split
[hd-frontend.md](../../../formats/hd-frontend.md) records for the four Zone circuit names: the Fury
table (`DATA06`) names what the Fury modes add.

| Id | English |
| --- | --- |
| `ER_ZDETONATOR_COM` | `DETONATOR SESSION COMPLETE!` |
| `ER_DETONATOR_SCORE` | `TOTAL SCORE` |
| `ER_DETONATOR_STAGE` | `STAGE CLEARED` |
| `PERCENT_MINES_CLEARED` | `MINES CLEARED` |
| `PERCENT_BOMBS_CLEARED` | `BOMBS CLEARED` |
| `ER_NUM_5_STREAKS` | `NUMBER OF 5 CHAINS` |
| `ER_NUM_10_STREAKS` | `NUMBER OF 10 CHAINS` |
| `ER_NUM_15_STREAKS` | `NUMBER OF 15 CHAINS` |
| `ER_NSR` | `NEW SCORE RECORD` |

**This corrects** `docs/ui/endrace-screens.md`, "Detonator's headline" (HD "draws nothing": the scan
there missed `DATA06`) and the thread's phase 2 row ("nothing on HD"); the headline code itself
(`Headline::Detonator` looks the id up in the served table) is right and draws on HD once that table
has the id. The string provider on HD serves
`DATA02`'s table; the ids must come from `DATA06`'s, the arrangement `oag_game::language::CircuitNames`
already has for the circuit names. Confidence **92** (a table lookup on the disc, read directly).

2048: its Brazilian-Portuguese translation file (`assets/ui/strings/disc/2048/portuguesebr.toml`)
keys all nine ids by 2048's own idstrings, so its disc table carries them (the table itself was not
re-read); Omega's `data00`, `data05`, `data08` and `data09` English tables carry all nine.

## What the record and the medal use as their value: the score

`0x0005ac00` hands the record call `Profile_RecordCampaignResult` (`0x0001f430`) its sixth argument
in `r8`. The mode switch at `0x0005b338`/`0x0005b35c` (`cmpwi r16,0xe`) sends mode `0xe` to
`0x0005c3c0`, `lwz r31, 0x2c94(r26)`: the **score** (`rm+0x2c94` = `rec+0x1314`, the copy of
`rm+0x2df0`). `r31` becomes `r8` at `0x0005b3ac`, the call is at `0x0005b3b0`. Inside the call,
mode `0xe` is the jump-table entry `+0x210` from `0x0001f560` = `0x0001f770`: it finds the profile
node for the launched cell (key `(g_GameState+0x90)->+0x58`), then updates `node+0x64` to the value
when `0`, `-1` or unsigned-less (`0x0001fcc8`), `node+0x68` the same way (`0x0001fc64`), and calls
`PI_Cell_EvaluateMedalForValue(cell, value)` at `0x0001fa64` (the `r29` moved from `r8` in the
prologue, `0x0001f47c`). So **the medal tier is evaluated on the score**, larger is better
(`counting_modes`'s flip set holds `0xe`). 2048 agrees: its builder passes the record's
`(int)rec[5]` (`+0x14`) to `FUN_8100cb72`. Confidence **82** (two sites, and the HUD's
`Detonator_UpdateHud` compares the same thresholds against `rm+0x2df0`, [detonator-cannon.md](detonator-cannon.md)).
This closes the "chosen" point `docs/formats/race-campaign.md` carries ("the value it hands the
compare is the score").

**The "new record" banner** is separate from the tier ("The record notice" below).

## The record notice

`0x0005ac00` at `0x0005c220` (mode `0xe`) walks a five-entry table and compares each entry's `+0x14`
unsigned against the new score (`0x0005c244`). The flag `rm+0x2cd7` (`rec+0x1357`) is set only when
the first entry smaller than the score is index `0`, i.e. the score is **strictly above the best**;
then `rm+0x2cd8` (`rec+0x1358`) is the new score (`0x0005c5c4`-`0x0005c5d0`), else the flag is `0`
and the value is the best entry's score (`0x0005c258`-`0x0005c260`). The tail `0x0022bdb8` shows
`RecBlockBlue` or `RecBlockGrey` by the flag and, for modes `6`, `13` and `14` (jump table
`0x0022be60`, all `0x0022be88`), writes `RecValue` as `sprintf("%s %d", localise(id), rec+0x1358)`
with `id = ER_NSR` when set and `ER_SR` when clear (`0x0022c03c`: TOC `0x336c`). Both frames show it:
the notice is **always drawn** (confidence **90**, read and seen). `RecLogoD` (Detonator) shows; the
`RecLogoSL/TT/Z` logos are hidden.

**The table.** `0x0005b310` calls `0x0001ef28`, which `sprintf`s `"%s_fury"` (TOC `0x008a595c`) with
the string at `+0x78` of an object found by id, hashes it with `0x0032a740` (a CRC-32, polynomial
`0xedb88320`, **initial value 0**, result inverted, so `zlib.crc32` does not reproduce it) and walks
the profile's record list (`*(*(0x009389a0) + 0x450)`, next pointer at node `+0x50`) for that key,
returning `node + 0x64`: five entries of `0x2c` bytes. Read live on the state of the second run
(288 nodes; one non-zero): node key `0xc685710b` = CRC of `"17_Track_fury"` (matched exactly), entry
0 `+0x00` `0.9139784` (mines percent), `+0x04` `1` (bombs), `+0x14` `5600` (the score), `+0x18`
`"PLAYER"`; entries 1 to 4 zero but for a `1` at `+0x28`. So the table is **per circuit and
direction** (`18_Track` is the reverse), not per speed class, and starts at `0`. Confidence **85**
(one live match; the string's source object is not identified).

Ours keeps one best score on the records row (`oag_game::records::Record::best_score`) and takes the
highest of every class of the title, circuit and mode (`oag_game::records::Store::judge_score`,
judged where the race is saved, before it is folded in). Not kept: the other four entries and the
`PLAYER` name. An unofficial run (a mod) sets no record and shows the standing best, **chosen**.

## Checked against the original (2026-11-26, RPCS3)

`scripts/rpcs3-hd-detonator-endscreen.py` on a restored Racebox Detonator state (Talon's Junction,
Feisar), run to `EndRace Results`; each death **forced** by writing the shield (`ship+0x5fa0`) to
`0.5` (at lap clock `25.18` for the first run, `45.37` for the second). Comparison pairs:
`hd/detonator-talons-endrace-record` and `-norecord` in the private compare repository.

| Seen | Run 1 (nothing scored) | Run 2 (cannon fired) |
| --- | --- | --- |
| Rows | `0 / 14`, `0.0%`, `0 / 14`, `0`, `0`, `0`, total `0` | `0 / 14`, `0.9%`, `1 / 14`, `0`, `0`, `0`, total `5600` |
| Record | `rec+0x1357 = 0`, grey `SCORE RECORD 0` | `rec+0x1357 = 1`, `+0x1358 = 5600`, blue `NEW SCORE RECORD 5600` |

- Rows grey and left-aligned at `x = 590`; `TOTAL SCORE` white on the dark bar; `LOYALTY STATUS`
  block `0 POINTS`, total `60`.
- The `MiniText` captions draw: `SCREEN TITLE`, `PLAYER POSITION`, `FINAL POSITION`, `LOYALTY
  STATUS`, `RECORD NOTIFICATION` (the first four also on the Eliminator frame,
  `data/reference/hd-eliminator-score/results.png`, which has no record notice).
- Behind the panel: the destroyed craft's particle burst, not a menu backdrop.
- **Timing:** the out-of-race bit was set at lap clock `45.73`, `EndRace Results` came at `51.13`
  (**5.4 s** of game time, about ±0.7 s at the poll period). Between them the craft glows, then
  bursts into the particle cloud, and the panel wipes in.

**Differences left in ours** (named, not fixed): the HD FE style's colours and the destroyed
craft's glow and burst on black ([endrace-screens.md](../../../ui/endrace-screens.md), "HD's text, glyph and frame against the
original"). The text sizes, the `CONFIRM` glyph and the frame are fixed there.

## The wait before the results

`SPDetonator`'s Update (`0x00068160`) dispatches on `rm+0x1970` (the phase) through a five-entry
table at `0x000681f4`; phase 3's entry is `0x000630c8`, which is `SPZone_UpdateFinish`'s shape
([zone-destroy.md](zone-destroy.md), Z9) plus a call to `0x00297f70` with `0.0` (`0x008a6bf0`) and
`rm+0x6de28` on every tick, a function that is a bare `blr` and does nothing: sub 0, once the phase time `+0x196c` reaches `1.0`
(`0x008a6bec`): clear HUD bits, sub 1; sub 1, at `rm+0x2dc4` (`4.5`, written at the blast in
single player, Z1): push `"EndRace Results"`, `RaceManager_SetPhase(4)`. So **5.5 s after the
blast**, against `5.4 +- 0.7 s` measured. Confidence **85** (read, and seen once). The race rules
stop at the blast (phase 3), and the builder `0x0005c630` has already copied the rows and record.

**The names.** `SPDetonator`'s vtable `0x008629c0` slot `+0x0c` holds the descriptor `0x00872680`,
whose entry is `0x00068160` (Update); the jump table's entry 3 is `0x0006837c`, which is
`bl 0x000630c8`; the phase-4 arm (`0x00068208`) reads `rm+0x2e10`, the Detonator stage counter.

**The camera.** `Ship_SetState` takes mode 2, the close orbit, for the Zone family except modes 13
and 21, so mode 14 with it, and the orbit holds until the results ([zone-destroy.md](zone-destroy.md),
Z2; [destroy-camera.md](destroy-camera.md), Z2f).

Wired (2026-11-26): `DestroyedCraft::detonator_end` (the wait and the orbit); the race ends there, with the rules
and the mines stopped and the HUD gone from the blast (`RaceSim::end_played_out`,
`detonator_ending`; pinned by the stage-5 `hd_endrace_ground_truth` run, 330 ticks). Where the craft
completes all fourteen stages the value of `rm+0x2dc4` is not read: ours ends at once, as before.

- **2048: ported.** Phase 3 of the mode-14 manager in `/2048/eboot-vita-2048-eu-v104.elf`
  (`0x8117f03e`, inside the switch that calls `FUN_8117e9c2`): `1.0`, then the float at `+0x131ac`
  (the `4.5` Z1 writes), then the results push and phase 4; the orbit, Z2 of zone-destroy.md. Pinned by `oag-title`'s data test only;
  no 2048 Detonator run reached here.
- **Omega: ported.** `FUN_012a0fc0` (`/omega/eboot-ps4-omega-eu.bin`) case 3: the same `1.0` then
  the float at `rm+0x1aff0`, then `"EndRace Results"` and phase 4. Same data test only.

## A destroyed craft and a finished race: the same rows, a record on both

- **The screen** has no branch on how the run ended: the filler reads only the copied record. A
  destroyed run shows the same six rows and the score line; only the values differ (laps completed
  short of `14`, a smaller percentage, fewer bombs).
- **Both endings call the builder with `force = 1`.** `Detonator_UpdateRace`'s end path
  (`0x00067f68`-`0x00067fa8`) is `0x0012ae38` (`RemoveAll`), `0x00087d40`, `RaceManager_SetPhase(rm, 3)`
  (`0x00056368`) and `0x0005c630(rm, 1)`; `0x0005c630` is a bare `b 0x0005ac00` (one instruction; the only
  other branch to the builder is a tail call at `0x0005dd40`), so `r4 = 1` reaches the builder's
  `r31` (`mr r31,r4` at `0x0005ac58`). With `r31 != 0` the builder sets the record gate `rm+0x2cd5 = 1` (`0x0005ae4c`) without
  looking at the finished byte `craft+0x7002` (the gate in [cell-medal-award.md](cell-medal-award.md)),
  and the end path runs on stage `>= 15` **or** the out-of-race bit `+0x6a64 & 0x1000`
  ([detonator-mode.md](detonator-mode.md)). So a destroyed run records its score and awards a medal
  exactly as a finished one does (the campaign lane's "a destroyed craft ends ours as finished" is the
  right reading); a quit race never reaches this path and records nothing. Confidence **78** (the call
  chain read, the `force` meaning inferred from the builder's one test of `r31`).

## Omega and 2048 (checked, per CLAUDE.md)

- **Omega: ported, same code.** `FUN_0155bc90` (`/omega/eboot-ps4-omega-eu-v107.bin`) is the same
  grid filler: `Line1` `ER_ZDETONATOR_COM`, `Grid%d.%d` at `x = 0x42200000`/`0x44138000`, `y = 37 +
  40 r`, then the same six labels with the same formats (`%d / %d` of `rec+0x1330` and `14`,
  `%.1f%%` of `rec+0x131c`, `%i / %i` of the u16 `rec+0x1320` and `14`, `%d` of the u16s `+0x1328`,
  `+0x132a`, `+0x132c`, and `Grid0.h`/`Grid1.h` `ER_DETONATOR_SCORE` and `%d` of `rec+0x1314`);
  `rec` is `param_1+0x1e28` there. Its string tables carry all nine ids (above). Omega's front end is
  HD's, so the wiring is the same. Not read: the Omega builder's record copy and the record call
  (the `rec` offsets agree, which is the evidence it copies the same record).
- **2048: checked, differs.** `FUN_81172882` (`/2048/eboot-vita-2048-eu-v104.elf`, its end-of-race
  builder and result list) appends **four** rows through `FUN_8106f722`/`FUN_8106f6ca`, not a grid:
  `ER_DETONATOR_STAGE` (the record's u16 at `+0x12`, **the HUD stage**, printed `%d` with no
  `/ 14`), `PERCENT_MINES_CLEARED` (`%.1f%%` of record `+0x00`), `PERCENT_BOMBS_CLEARED` (`%d` of
  record `+0x04`, no `/ 14`), `IG_HUD_SCORE` (`%d` of record `+0x14`: the label is `IG_HUD_SCORE`,
  not `ER_DETONATOR_SCORE`). The three streak counts (`+0x0c`/`+0x0e`/`+0x10`) are copied into the
  stats block and **not drawn**. The record value is the score: the mode-`0xe` block compares
  `(uint)rec[5]` against a five-entry table (stride `0x3c`, `+0x14`) the same way (`index == 1`
  after the loop means the first entry was beaten) and `piVar36[0x4c1d]` (the score) is the value
  passed to `FUN_8100cb72`. `FUN_810d2dc4`'s mode-`3` case gives a records list its column
  captions `IG_HUD_SCORE`, `PRO_TAG`, `FE_STAGE`, `PERCENT_MINES_CLEARED` (a leaderboard, not the
  end screen). 2048's results are a different screen dialect (a list, `crates/ui-screens`'s 2048
  front end), so the six-row HD grid is **not** its layout.

## Wired (2026-11-22, `det-end`)

HD draws the screen from live race state; the walk was a Fury campaign cell (`grid8_4_3`) through
`--drive`, the cannon held, the craft destroyed in stage 1: `DETONATOR SESSION COMPLETE!`, `STAGE
CLEARED 0 / 14`, `MINES CLEARED 1.5%`, `BOMBS CLEARED 0 / 14`, the three chain counts `0`, `TOTAL SCORE
6900`, the loyalty block beside it at `0 POINTS`.

| Piece | Where | Pinned by |
| --- | --- | --- |
| The six rows and the score line, the frame, the headline | `oag_ui_screens::endrace::hd::detonator` (`DetonatorGrid`, beside `RaceGrid`), `DetonatorResults` (`modes.rs`) | `endrace/hd/detonator/tests.rs`: `4 / 14` for a run destroyed in stage 5, `14 / 14` for a finish, the bombs count truncated to a `u16`, positions at `40`/`590`/`y = 37 + 40 r`/`360`, the frame, no banner, the loyalty block still drawn |
| The values | `oag_game::results_tables::detonator_results` over `RaceSim::{detonator_percentage, detonator_destroyed_bombs, detonator_chains, detonator_score}` and the scoreboard row's `laps_completed` | `hd_endrace_ground_truth::a_detonator_run_destroyed_in_stage_five_draws_four_of_fourteen_and_its_session_rows` (real race, real board, real definition; the HUD stage reads 5 where the row reads 4), `detonator_ground_truth` (a finish: the board says `14`) |
| The ids | the boot fills HD's served table with `DetonatorResults::IDS` from `DATA06`'s language table (`strings.fill`, `crate::campaign::hd_data06_strings`) | `hd_boot_ground_truth::the_boot_resolves_detonators_end_screen_ids_from_data06` (fails with the fill dropped) |
| The sim | `Cannon::{fives, tens, fifteens}` (a clip's second five counts again; they survive a reload, `Awards` do not), `destroyed_bombs` on an EMP's bomb kill | `oag_weapons::detonator::{cannon,emp}` tests |

Two things this fixed that the page's first reading did not have: the observation's laps (padded to the
target on any `finished` run) would have printed `14 / 14` for a destroyed run and banked the full 14
stages of loyalty; both now read the scoreboard row. `Headline::Detonator` and the rows draw only when
the model carries a Detonator result, so every other HD mode is unchanged.

**Drawn:** the rows, the captions and the record notice (`RecLogoD` comes from `DATA06`'s copy of
`EndRace_Definition.xml`, the only one authoring it; the other widgets are `DATA02`'s). `ER_NSR`,
`ER_SR` and `ER_RECORD_NOT` are in the served table. Only the Detonator draws a record notice; the
tail also serves modes `6` and `13`, unchecked here.

**Omega: checked, applies, not wired.** The record key is the same: `FUN_012943b0`
(`/omega/eboot-ps4-omega-eu.bin`) `sprintf`s `"%s_fury"`, hashes it with the same init-0 CRC-32,
walks a list whose next pointer is at `+0x50`, and for mode `0xe` compares five `+0x14` entries
(stride `0x3c`) against the score; its banner flag was not read. The filler, ids and `rec` offsets are HD's; the ids are in
its served table (`omega_menu_blocks_ground_truth::omegas_served_table_carries_detonators_end_screen_ids`);
`DetonatorResults` is title-agnostic. Omega keeps the built-in scoreboard
(`endrace_entry` is `None`: wiring HD's reader there leaves `EndRace Menu` with no options,
`docs/formats/omega-status.md`), so nothing draws until that is solved.
**2048: checked, differs, not wired.** Its four-row list (stage = HUD stage with no `/ 14`, mines,
bombs `%d`, `IG_HUD_SCORE`) is modelled and worded (`oag_ui_screens::endrace::Detonator2048Results`,
tested; no screen draws it) and is the `Results` page's, which `docs/ui/endrace-2048.md` lists as a named gap
(`<RaceResults>`, native code undecompiled); its Touch Summary page has no widget for a list.

## Names

| Address | Name | Conf |
| --- | --- | ---: |
| `0x002234b0` | `EndRaceResults_FillDetonatorRows` | 80 |
| `0x0022bdb8` | `EndRaceResults_ShowRecordNotice` | 90 |
| `0x0001ef28` | `Profile_FindTrackRecords` | 80 |
| `0x0032a740` | `Crc32_HashStringZeroInit` | 85 |
| `0x00068160` | `SPDetonator_Update` | 80 |
| `0x000630c8` | `SPDetonator_UpdateFinish` | 80 |

`0x0005ac00` (the builder) and `0x0005c630` (its thunk with `force = 1`) stay unnamed here: the
builder's other modes were not traced ([endrace-loyalty.md](endrace-loyalty.md)).

## Open, with the next address to try

- **The object `0x0001ef28` takes `+0x78` from** (looked up by id through `0x00013688`) is not
  identified; the live key match fixes the string it holds (`17_Track`).
- **Bombs and streaks counting rules**: the writers of `rm+0x6de40` (`0x00110a48`, `0x00110c70`,
  `0x00112b64`, `0x0012e334` inside `BombManager_DestroyAtLane`) and `rm+0x6de3c` (`0x00110494`,
  `0x00110990`, `0x00112a34`, `0x0011a6a4`) were found by `ps3-dform-scan.py --disp -0x21c0,-0x21c4`;
  the cannon and EMP sites are read ([detonator-bomb.md](detonator-bomb.md), [detonator-emp.md](detonator-emp.md));
  `0x0011a6a4` is not on any page.
