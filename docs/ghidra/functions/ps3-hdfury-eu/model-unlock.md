# `PI_TeamModel` `<Unlock>`: the parser on HD and Omega, and what the rows count

2026-11-14, lane `hd-skin-unlocks`. Static reading (Ghidra bridge, `/hdfury/EBOOT-ps3-hdfury-eu.elf`
and `/omega/eboot-ps4-omega-eu-v107.bin`) plus a census of both discs' definitions and archives.
**The parser is read on both titles; the evaluator is not found on either.** What each condition
counts is therefore read off the data, below, and wired as such
([race-setup.md](../../../formats/race-setup.md#hd-era-model-unlocks)).

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00166210` (HD) | `Definition_ParseUnlock` | 80 |
| `0x008ab8e0` (HD) | `Unlock_ConditionNameTable` | 75 |
| `0x013f8880` (Omega **v1.07**, not renamed) | the same parser | 80 |

## The parser, `0x00166210` (HD)

Found by `scripts/ps3-toc.py attrib 0x00786be8 0x00786c18` (the strings `MedalCountHD` and
`Loyalty`): one function names both. Ghidra's own xrefs (`0x0047b...`) are TOC misresolutions
([memory.md](memory.md)). The decompile:

- compares the element name with `PTR_s_Unlock_008ab8b8` (`"Unlock"`); any other element goes to a
  second branch reading one attribute into the owner at `+0xaa`;
- allocates a `0x4c`-byte record, zeroes it, and walks the attributes, comparing each name against
  eighteen TOC strings in turn and storing the value pointer in its own slot (`+0x08` .. `+0x48`);
- clears the slot at `+0x30` when it reads `"None"` (Omega's same step names that slot `Grid`);
- sets `+0x00` and `+0x01` from two attribute values compared with `"true"`/`"1"`;
- appends the record to the owner's list at `+0xac`.

Pulse's `Definition_ParseUnlock` (`0x0888f094`, [race-campaign.md](../psp-pulse-usa/race-campaign.md))
is the same job; the name is reused. 80, not higher: decompile only, and the TOC slots behind
`iStack_544 - 0x1c18 .. -0x1bd4` were not resolved one by one (Omega's copy names them).

## The same parser on Omega, `0x013f8880` (v1.07)

x86, so the strings are direct operands: `strcasecmp` against `ClassIs`, `TeamIs`, `TrackIs`,
`TournamentIs`, `ModeIs`, `GridIs`, `Class`, `Team`, `Track`, `Grid`, `Tournament`, `Medal`,
`MedalCount`, `MedalCountHD`, `MedalCountFURY`, `TimePlayed`, `Loyalty`, `Purchase`, the record
`0x98` bytes. Slots (byte offsets): `Team` `+0x48`, `Track` `+0x50`, `Tournament` `+0x58`, `Grid`
`+0x60` (cleared on `"None"`), `Medal` `+0x68`, `MedalCount` `+0x70`, `MedalCountHD` `+0x78`,
`MedalCountFURY` `+0x80`, `TimePlayed` `+0x88`, `Loyalty` `+0x90`. **`Locked` is not among the
compared names**, so the `locked="true"` every skin row carries is not a condition this parser
stores under its own name. The address is the v1.07 program's; `names.tsv` holds base (v1.00)
addresses only ([patch-v107-addresses.md](../ps4-omega-eu/patch-v107-addresses.md)), and the base
address was not resolved, so no row is added for Omega.

## The condition names, `0x008ab8e0` (HD)

Twelve pointers: `Track`, `Grid`, `Tournament`, `Medal`, `MedalCount`, `MedalCountHD`,
`MedalCountFURY`, `TimePlayed`, `Loyalty`, `Purchase`, `None`, `true` (`0x00786bb0` ..
`0x00786c38`). The order is the condition order Omega's parser compares in, which reads as an enum
of unlock kinds; its readers were not located (Ghidra's xrefs to it are TOC misresolutions). 75.

## What the rows ask for, read off the data

`Data\Plugins\frontend\definition.xml` on HD's `DATA00.PSARC` (the copy the loader serves, pinned
by `selection_screens_ground_truth`) and `data\plugins\teams\Definition.xml` on Omega's
`data09.psarc` carry, for each of the twelve HD-era teams, seven model rows in this order:

| Model | `modellocation` | `texturelocation` | `<Unlock>` |
| --- | --- | --- | --- |
| `chrome_c1` | `<Team>_c1` | `livery4` | `locked="true" team="<Team>" MedalCountFURY="80"` |
| `nitro` | `<Team>_n1` | `livery1` | `grid="gridN"` (Feisar `grid8`; 8 to 15 by team) |
| `concept1` | `<Team>_c1` | `livery1` | none |
| `normal` | `<Team>` | `livery1` | none |
| `SKIN1` | `<Team>` | `livery2` | `locked="true" team="<Team>" loyalty="4000"` |
| `SKIN2` | `<Team>` | `livery3` | `locked="true" team="<Team>" loyalty="8000"` |
| `chrome` | `<Team>` | `livery4` | `locked="true" team="<Team>" MedalCountHD="87"` |

Other copies differ and are not what either game reads: HD's `DATA06` (Fury) and `DATA03` copies
write a plain `MedalCount="87"` on `chrome` and no row on `chrome_c1`; Omega's
`Definition_Demo.xml` (same archive) does the same and is the demo's file. Omega's `Tigron` and
`VanUber` name every model in one directory, `SKIN1`/`SKIN2`/`chrome` in `livery3`/`livery2`/
`livery4` (`Tigron`) or `livery2`/`livery3`/`livery4` (`VanUber`), with no `<Unlock>`.

**What each condition counts, as wired** (`oag_game::unlock::ModelGate`):

- `MedalCountHD="87"` / `MedalCountFURY="80"`: **gold medals over the grids whose `Campaign` is
  `HD` / `Fury`.** 87 and 80 are exactly those campaigns' cell counts (`grid_00`..`grid_07` 6+8+10+10+10+12+14+17,
  `grid_08`..`grid_15` 80; `docs/ui/campaign-screens.md`), the `GOLD MEDALS` denominators the
  original's `Campaign Selection` prints (`0 / 87`, `0 / 80` on RPCS3). Two independent tables agreeing
  on both numbers. Gold, rather than any medal, is **chosen**: the evaluator is unread, and a
  threshold equal to the cell count makes "every gold" the reading that leaves no slack. 70.
- `loyalty="N"` with `team="<Team>"`: **that team's own running total** reaches `N`, Pulse's
  traced `Unlock_LoyaltyMet` rule; on HD the total is the one `Race_ComputeLoyaltyAward` pays
  ([endrace-loyalty.md](endrace-loyalty.md)), shared across titles by team identity. **Inherited from
  Pulse**, not traced on HD.
- `grid="gridN"`: **the named grid's own medal points reach its `RequiredPoints`**, Pulse's
  `Unlock_GridPointsMet` ([race-campaign.md](../psp-pulse-usa/race-campaign.md)). **Inherited from
  Pulse.** `nitro` is the Fury hull, and `grid8` is the Fury campaign's first grid.
- A row passes when all its conditions pass; a model passes when all its rows pass (each model
  authors one). A row naming nothing read never passes. `locked="true"` is not evaluated (see the
  parser).

**What would settle the open half:** the reader of the record list at owner `+0xac` (HD) /
`+0x120` (Omega). On Omega it is the function that walks `*(owner + 0x120)` and reads the string at
`+0x78`; a search for loads at those two offsets in one function is the next step.

## A skin is a folder swap

Feisar's `ship.rcsmodel` (HD `DATA02`) names four textures, all under `data/ships/feisar/livery1/`:
`colours_flashing_glow`, `feisar_glass`, `feisarhd_skin_1024`, `feisarhd_skin_n_1024`. Each is
shipped under `livery2/`, `livery3/` and `livery4/` too; `feisar_c1`'s hull names its textures under
`textures_dlc/livery1/` and `textures_dlc/livery2..4/` ship the same names. Omega ships
`hdships/feisar/Livery1..4/` with the same names as `.gnf`. So a model is its directory's hull with
every `livery1` path read from its own `texturelocation`. Pinned on every team of both titles by
`hd_livery_ground_truth::every_hd_era_livery_folder_holds_what_its_hull_names` (48 models on HD)
and `every_omega_livery_folder_holds_what_its_hull_names` (54 on Omega, the two Tigrons' six
included).

## Cross-title

- **Omega: ported.** Same rows, same parser, same folders; the same gate and swap run on it,
  pinned by `hd_ship_preview_ground_truth::omega_draws_every_teams_ps4_hull_at_the_updates_rest_pose`.
- **2048: checked, differs.** Its five teams carry no `<Unlock>` on a model (Omega's copies of them
  carry `<Unlock purchase="1">` on the team), and its craft are numbered directories, not liveries.
- **Pulse: differs.** Its variants carry `Team`/`loyalty`/`Exclusive` rows, read by
  `oag_game::unlock::loyalty_unlocked`.
