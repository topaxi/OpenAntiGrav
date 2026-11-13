# How HD/Fury awards a campaign cell's medal, Eliminator and NitroBattle included (2026-11-13, `elim-medal`)

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`). Every function below is on TOC
`r2 = 0x008ad4d8` (`scripts/ps3-toc.py toc` reports each `exact`); every TOC slot quoted was
resolved with `scripts/ps3-toc.py resolve`, not read off Ghidra's xref index
([race-campaign.md](race-campaign.md), "The TOC-xref trap"). Read with headless Ghidra
(`analyzeHeadless ... -readOnly`), decompile and disassembly; the jump-table cases of
`Profile_RecordCampaignResult` are not disassembled in the project and were disassembled in a
read-only session.

**The answer to [race-campaign.md](race-campaign.md)'s "what is not determined":**

1. **An Eliminator cell's medal is the finishing place on the score board.** Gold for 1st,
   silver for 2nd, bronze for 3rd. The cell's per-rung `Gold`/`Silver`/`Bronze`, authored `1`/`2`/`3`
   on every rung and read until now as a dummy, are the real targets: place thresholds,
   compared `place <= target`, unflipped.
2. **`NitroElimNovice`/`Skilled`/`Elite` is not a medal threshold.** It is the race-end score
   target per rung: the number `SPElimination_UpdateTarget` (`0x00069130`,
   [eliminator-score.md](eliminator-score.md)) ends the race on. Its unit is score points.
3. **A NitroBattle cell (mode 13) goes through the same record case and the same compare**, but
   its place is the race position, not a score rank (lower confidence, below).

## Names

| Address | Kind | Name | Confidence | Evidence |
| --- | --- | --- | ---: | --- |
| `0x0001f430` | function | `Profile_RecordCampaignResult` | 82 | The only caller is the single-player end-of-race builder (`0x0005b3b0`). Per mode it updates the cell's saved record and returns the tier `0x001e1028` gives; mode 8 and 13 share one case, below. |
| `0x001e1028` | function | `PI_Cell_EvaluateMedalForValue` | 88 | `value - 1 < 0xfffffffe`, then a three-way compare against `cell + 0xb0 + rung * 0xc`, `rung = g_GameState+0xdc`; `>=` for modes 6 and 14, `<=` otherwise; returns `0`/`1`/`2` or `0xff`. |
| `0x001e25c0` | function | `PI_Cell_EvaluateSavedMedal` | 85 | The same compare, `value` read from the saved record's `Achieved` (`+0`). Called by `0x0001c6d0` and from race code at `0x000e58d8`. |
| `0x001e1138` | function | `PI_Cell_GetSavedMedal` | 80 | Copies the `0x48`-byte record and returns byte `+9`, the save format's `Medal`. |
| `0x001e1188` | function | `PI_Cell_GetSavedSkillLevel` | 80 | The same, byte `+8`, `SkillLevel`. |
| `0x001e11e0` | function | `PI_Cell_GetSavedBestValue` | 62 | Returns record `+4` (`Time`) for modes 3, 8, 9, 13 and `+0` (`Achieved`) for every other mode; called by the cell screen (`0x00215258`, `0x002181a8`). What the screen prints with it is not read. |
| `0x00015eb0` | function | `PI_Cell_WriteSave` | 85 | Writes `<PI_Cell name="%s_%d_%d">`, the cell's `Values`, `Gold`/`Silver`/`Bronze`, then `<Save Achieved="%d" Time="%d" SkillLevel="%d" Medal="%d">` from the record's `+0`, `+4`, byte `+8`, byte `+9`. |
| `0x00013098` | function | `PI_Cell_ReadSave` | 75 | Reads the four attributes `Achieved`/`Time`/`SkillLevel`/`Medal` and copies a `0x48`-byte record into the cell's entry. |
| `0x001e1590` | function | `PI_Cell_ParseAttribute` | 90 | The per-element attribute ladder: names and offsets in the table below. |
| `0x009201d8` | data | `PI_Cell_ModeNameTable` | 95 | `{ordinal, name}` pairs, `PI_Cell_ParseAttribute`'s `mode=` lookup (TOC `+0x13cc` = `0x008ae8a4` holds `0x009201d8`). |

`PI_Cell_` and `Profile_` keep these apart from Pulse's `Cell_EvaluateMedal` (`0x088bf620`,
another binary), the clash that left `0x001e25c0` unnamed in
[race-campaign.md](race-campaign.md).

## The cell's fields: `PI_Cell_ParseAttribute` (`0x001e1590`)

Each branch matches one attribute name, read through its TOC slot, and stores it at one cell
offset. A flat `Gold`/`Silver`/`Bronze` fills all three rungs at once.

| TOC | Attribute | Cell offset |
| --- | --- | --- |
| `+0x1410` | `Gold` | `+0xc8`, copied to `+0xb0` and `+0xbc` |
| `+0x1418` | `Silver` | `+0xcc`, copied to `+0xb4` and `+0xc0` |
| `+0x141c` | `Bronze` | `+0xd0`, copied to `+0xb8` and `+0xc4` |
| `+0x1438`..`+0x1440` | `EasyGold`, `EasySilver`, `EasyBronze` | `+0xb0`, `+0xb4`, `+0xb8` |
| `+0x142c`..`+0x1434` | `MediumGold`, `MediumSilver`, `MediumBronze` | `+0xbc`, `+0xc0`, `+0xc4` |
| `+0x1420`..`+0x1428` | `HardGold`, `HardSilver`, `HardBronze` | `+0xc8`, `+0xcc`, `+0xd0` |
| `+0x144c` | `NitroElimNovice` | `+0xd4` |
| `+0x1448` | `NitroElimSkilled` | `+0xd8` |
| `+0x1444` | `NitroElimElite` | `+0xdc` |
| `+0x13cc` | `mode` (through `PI_Cell_ModeNameTable`) | `+0x110` |

So the medal triple for rung `r` (`0` easy, `1` medium, `2` hard) is `+0xb0 + 12 r`, and the
nitro number for the same rung is `+0xd4 + 4 r`. `SPElimination_UpdateTarget` ends a campaign
Eliminator on `cell+0xd4 + difficulty * 4` ([eliminator-score.md](eliminator-score.md), "Ending",
84 for the cell branch). The offsets now line up: the nitro triple is that target, Novice to
Elite.

`PI_Cell_ModeNameTable` (`0x009201d8`): `3 Race`, `4 Tournament`, `5 Time Trial`, `6 Zone`,
`8 Elimination`, `9 Head2Head`, `10 Speed Lap`, `11 Custom Grid`, `12 AI Race`,
**`13 NitroBattle`**, **`14 Detonator`**. This settles [race-campaign.md](race-campaign.md)'s
"`0xd` ... plausibly" and agrees with [mode-manager.md](mode-manager.md)'s "id 14 is Detonator".

## The law

### End of race: `0x0005ac00` hands a place and a value to `Profile_RecordCampaignResult`

`0x0005ac00` is the single-player end-of-race builder ([endrace-loyalty.md](endrace-loyalty.md)).
In mode 8 it fills a 300-byte row per craft at `param_1 + 0x672 * 4`: the player flag at
`+0`, the truncated score `(int)craft+0x6f80` at `+0x42 * 4`, kills `+0x6f7c` at `+0x44 * 4`,
deaths `+0x6f78` at `+0x45 * 4`. It bubble-sorts the rows:

- a higher score ranks first (unsigned compare);
- on equal score, **fewer kills ranks first** (`next.kills < this.kills` swaps);
- on equal score and kills, the player's row ranks above an opponent's.

The player's place is the 1-based index of the flagged row, stored at `param_1[0x660]`.
Every other mode first takes `param_1[0x660] = craft+0x7918` (the race position); mode 3
overwrites it from its own table.

Then, at `0x0005b3b0`:

```text
Profile_RecordCampaignResult(profile, r4, place & 0xff, r6, craft+0x7028, value)
    value = param_1[place * 0x4b + 0x669]   // mode 8: the player's row's score
    value = param_1[0xb22]                  // mode 13: a u16 off the race record, not read
```

`param_1[place * 0x4b + 0x669]` is row `place - 1`'s `+0x42` word, the score the rows were
sorted on. The prototype Ghidra gives (`(int, uint)`) is wrong: the call passes six arguments,
and the body moves `r5` to `r24` and `r8` to `r29` in its prologue (`0x0001f474`, `0x0001f47c`).

### The record and the tier: the mode 8 and 13 case (`0x0001f804`)

`Profile_RecordCampaignResult` switches on `g_GameState+0xe0 - 3` through a 17-entry table
(TOC `-0x7998` = `0x0001f560`). Modes 8 and 13 both land at `0x0001f804`; Race (3) and Head2Head
(9) share a different case (`0x0001f998`), so this is not the race-position case renamed.

```text
rec = profile record whose key == (g_GameState+0x90)->+0x58      // the launched cell
if place < rec.Achieved or rec.Achieved == 0:   rec.Achieved = place      // 0x0001f85c
if value > rec.Time or rec.Time in {0, -1}:     rec.Time = value          // 0x0001f870
profile+0x48a = 1                                                         // dirty
tier = PI_Cell_EvaluateMedalForValue(cell, place)                         // 0x0001fa64
tier 0 -> profile+0x17c += 1, 1 -> +0x178 += 1, 2 -> +0x174 += 1
```

then the shared tail at `0x0001f60c` updates the medal and the rung:

```text
saved = rec.Medal                           // byte +9, 0xff = none
if saved != 0xff and saved < tier: keep both bytes          // a better medal is already saved
elif rec.SkillLevel == 0xff or tier < saved: rec.SkillLevel = rung    // g_GameState+0xdc
elif tier == saved: rec.SkillLevel = max(rec.SkillLevel, rung)
rec.Medal = tier
return tier
```

`PI_Cell_EvaluateMedalForValue(cell, place)` for mode 8 or 13 is the `<=` branch (it flips only
for `+0x110 == 6` or `== 0xe`):

```text
place <= cell[+0xb0 + 12 rung] -> 0 gold
place <= cell[+0xb4 + 12 rung] -> 1 silver
place <= cell[+0xb8 + 12 rung] -> 2 bronze
else 0xff
```

With every Eliminator and NitroBattle rung authored `1`/`2`/`3`
(`crates/hd/tests/campaign_grids_ground_truth.rs`,
`eliminationfamily_cells_carry_a_real_nitro_triple_and_a_dummy_flat_one`), that is gold for 1st,
silver for 2nd, bronze for 3rd, nothing from 4th. The score is kept as the cell's best `Time`
(higher is better) and decides nothing.

`PI_Cell_EvaluateSavedMedal` (`0x001e25c0`) is the second, independent reader of the same
storage: it runs the same compare on the saved `Achieved` (`+0`), which this case writes as the
best place. Two consumers, one writing a place and one comparing the saved value against the
same thresholds, agree.

### Confidence

| Claim | Confidence | Why not higher |
| --- | ---: | --- |
| Eliminator medal = score-board place against the rung's `1`/`2`/`3`, `<=` | 82 | Static only: decompile, disassembly and the TOC agree, two consumers agree; no live finish read. |
| `NitroElim*` is the race-end score target, not a medal threshold | 88 | Parse offsets `+0xd4..+0xdc` against `SPElimination_UpdateTarget`'s `+0xd4 + 4 * difficulty`; nothing reads them in `PI_Cell_EvaluateMedalForValue`. |
| The tie order on equal score (fewer kills first, then the player) | 72 | Read off the bubble sort's two compares; never seen live. |
| NitroBattle (13) medal = race position `craft+0x7918` against `1`/`2`/`3` | 70 | Same case and compare; that `+0x7918` is the race position and that mode 13 takes the default branch are read, not watched. |
| A better medal overwrites `SkillLevel` with the current rung, even an easier one | 80 | Four branches, read; never seen live. |

## Live check: not run, the recipe

Not run this pass. The save file does not help: `DATA.BIN` in the maintainer's
`BCES00664-AUTO-` save is not plaintext (`strings` finds no `PI_Cell`). The recipe for whoever
has an RPCS3 session on a Fury Eliminator cell (`grid8_3_2`):

1. Finish the cell; note the board's place and points and the medal the screen awards.
2. Read `g_GameState+0x90` (the cell) and its `+0x58` (the record key).
3. The profile is `*(*(TOC - 0x7b80))`; walk `profile+0x450`: key at `+0`, next at `+0x50`,
   record at `+0x64`.
4. Expect `Achieved` = best place, `Time` = best points, `SkillLevel` = the rung, `Medal` =
   place - 1 when place is 3 or better, else `0xff`.

## What this changes in ours

`oag_tables::race_campaign::Cell::medal_law_is_unmeasured` withholds every HD Eliminator and
NitroBattle medal; `oag-game`'s `race_stage.rs` passes `kills` as an Elimination cell's value and
nothing for `Other("NitroBattle")`; `Cell::evaluate_medal_for_difficulty` flips for
`Zone | Elimination`, which is Pulse's set. HD's flip set is modes 6 and 14 (Zone, Detonator).

The rung tail above also answers [race-campaign.md](race-campaign.md)'s reconciling question:
the saved `SkillLevel` is written here and is the rung of the best medal, not a never-downgraded
maximum. Ours (`oag_game::records::CampaignRecord::best_difficulty`) keeps the hardest rung
instead; that stays a chosen difference until someone decides to follow the original.

## Other titles

- **Omega:** see the dated note at the end of this page.
- **Pulse:** not this law. Pulse's `Eliminator` cells keep a kill target in the flat `Gold`
  ([`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`](../psp-pulse-usa/race-campaign.md)),
  and its flip set includes Elimination.
