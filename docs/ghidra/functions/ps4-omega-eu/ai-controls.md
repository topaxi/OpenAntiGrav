# The opponent driver on Omega: 2048's code, HD's laws in an HD-lineage mode, and a coin of its own

**Binary:** `ps4-omega-eu` `eboot.bin` (x86-64, image base `0x01000000`, Ghidra program
`/omega/eboot-ps4-omega-eu.bin`, the **base package's** executable; the 1.07 patch's
`eboot.bin` was not compared). Read 2026-10-12 by the `orig-ai-omega` lane, which flies
Omega's original opponent in Omega's campaign. The templates are HD's map,
[ps3-hdfury-eu/ai-controls.md](../ps3-hdfury-eu/ai-controls.md), and 2048's,
[vita-2048-eu-v104/ai-controls.md](../vita-2048-eu-v104/ai-controls.md), which this page
compares against piece by piece.

**Static only.** There is no PS4 emulator in this project's toolchain, so nothing here was
seen running. Every score is capped at 84 ("decompilation only, consistent call sites",
[confidence rubric](../../../reverse-engineering/confidence-rubric.md)); the HD and 2048
laws measured on RPCS3 and Vita3K corroborate the structure, not Omega's behaviour.

**The answer in one paragraph.** Omega's opponent is 2048's code: one constructor and one
update function carrying both lineages, switched by the byte `driver+0xa0 = (mode id <
0x17)`, exactly as 2048's `driver+0x98`. In an **HD-lineage** mode (every named mode,
`Race` `3` to `Multiplayer Ranked` `0x16`) it runs HD's laws: the round robin, the base
`AIRaceStats_<class>.xml` set, HD's thrust law with its four changes, the ring rate
`0.001`, HD's halved lookahead on `05_Track` and `20_Track`. In a **2048-lineage** mode (a
CRC id, `>= 0x17`) it runs 2048's: every tick, the `*2048.xml` set, the event's AI skills,
the preferred lane following the choice, pad tracking and a contact sidestep. **One law is
neither HD's nor gated:** the lane search's coin. Omega draws it in the constructor and
reads it in the search whatever the lineage, and HD has no trace of it. Every grid cell of
this project's Omega campaign (`CellMode_Definition.xml`, HD's schema) maps its `Mode`
to a named id, so the campaign flies HD's laws plus the coin.

## The cast

| Address | Name | Confidence | HD / 2048 counterpart | Verdict |
| --- | --- | --- | --- | --- |
| `0x012c80c0` | `AiManager_AddDriver` | 80 | HD `0x000ba9e0` with `Ai_Construct` inlined; 2048 `0x81198708` + `0x8119853c` | 2048's, both lineages ([below](#the-constructor)) |
| `0x012c6570` | `AIManager_Update` | 80 | HD `0x000bb960`, 2048 `0x8119fa02` | identical to both ([below](#the-round-robin-and-the-shuffle)) |
| `0x012c0480` | `AI_UpdateCraft` | 78 | HD `0x00102d70`, 2048 `0x8119dda4` | 2048's, with the planner and controls inlined ([below](#ai_updatecraft-0x012c0480)) |
| `0x012c5650` | `AI_ComputeOpponentThrust` | 82 | HD `0x000fdf00`, 2048 `0x81199dc8` | 2048's, the wander and 2048's Eliminator-family law inlined ([below](#the-thrust-law)) |
| `0x012c9650` | `AI_ResolveSkillScale` | 80 | HD `0x000babf8`, 2048 `0x8119fb5e` | 2048's plus one term ([below](#the-skill)) |
| `0x012c99b0` | `AiStats_Load` | 84 | 2048 `0x811a0b18` | identical to 2048's ([below](#the-tables)) |
| `0x0147e8e0` | `CampaignCell_ParseValues` | 72 | - | reads a grid cell's `<Values>`; `Mode` through the table at `0x019267c0` ([below](#which-lineage-a-campaign-cell-runs)) |
| `0x015a0f90` | `GameMode_SetCurrent` | 70 | 2048 `GameMode_Current`'s setter | stores the event object at `0x01e31910` and its id from its name; `0` clears it |
| `0x01926700` | `g_GameModeNames` (data) | 84 | 2048 `0x815191cc` | the same 23 names in the same order |

The game state: `0x01f999d8` class (`0` Venom to `4` Super Phantom, the names at
`0x01926620`), `0x01f999e0` the level, `0x01f999e4` the mode id, `0x01f999e8` the human
count, `0x01e31910` the current game-mode object (2048's `g_CurrentGameMode`),
`0x01f99990` the campaign cell, `0x01f99ad0` the circuit's stats record. The racer table is
`0x01a07390`, stride `0x9b0`: `+0x00` craft, `+0x08` driver, `+0x9a0` the turn flag.

## Which lineage a campaign cell runs

`CampaignCell_ParseValues` (`0x0147e8e0`) reads a cell's `<Values>` attributes (`Weapons`,
`Damage`, `AICount`, `Status`, `Track`, `skillEasy`/`skillMedium`/`skillHard`, `Class`,
...). For `Mode` it walks a table of `{id, name}` pairs at `0x019267c0` with
`strcasecmp` and stores the id at `cell+0x180`:

| Cell text | Id |
| --- | --- |
| `Race` | `3` |
| `Tournament` | `4` |
| `Time Trial` | `5` |
| `Zone` | `6` |
| `Elimination` | `8` |
| `Head2Head` | `9` |
| `Speed Lap` | `0xa` |
| `Custom Grid`, `AI Race`, `NitroBattle`, ... | `0xb`, `0xc`, `0xd`, ... |

All below `0x17`. `FUN_01526370`, which publishes a selected cell to the front end's
variables (`Mode`, `Class`, `Track`, `Weapons`, `SkillLevel`) and stores the cell at
`0x01f99990`, prints the mode through `g_GameModeNames[cell+0x180]` when it is below
`0x17` and through a game-mode object's name otherwise. The campaign flyer
(`FUN_01520a10`, strings `HD_Campaign`, `HDCampaignFlyerModel`, `FuryCampaignFlyerModel`,
`2048CampaignFlyerModel`) clears `0x01e31910`. So in an HD/Fury cell the lineage byte is
set and no game-mode object is current: the `+1.9` skill term, the game-mode flags `1`/`4`
and 2048's Eliminator-family thrust law below are not reached. **Confidence 75**: the
route from the published `Mode` variable to `0x01f999e4` at race start was not walked;
the other writer of `0x01f999e4` read, the racebox commit `FUN_0156f6e0`, stores the id it
is handed.

The racebox is different and is not this project's original-AI path (the racebox flies this
project's driver): `FUN_0156ef00` calls `FUN_0156f6e0` with a flag that is set for an
`MSC_WOHD` (HD-era) session; with it clear, `FUN_0156f6e0` builds a game-mode object for
the named id and makes it current with `m_skillScale` `0.5`/`1.5`/`3.5` at Easy, Medium and
Hard (`+0x234`).

## The constructor

`AiManager_AddDriver` (`0x012c80c0`) allocates `0x5f0` bytes, appends the driver to the
manager (`mgr+0x78` count, `+0x80` array, `+0x150 = 1`) and the racer table, and inlines
2048's `Ai_Construct` whole:

| Field | Value | Lineage |
| --- | --- | --- |
| `driver+0xa0` | `0x01f999e4 < 0x17` | both: the lineage byte (2048 `+0x98`) |
| `driver+0xd0` | `rand() % 2` | **both**: the search coin (2048 `+0xc8`) |
| `driver+0x194..` | `8 - grid` (`grid = 8` for the player) | both: HD's spread |
| `driver+0x198` | phase `1e6` (`0x49742400`), segments `1.0` | both |
| `driver+0x2c0..+0x2cc` | four `rand()` values (`* 9.313226e-08`, `* 9.313226e-11 + 0.2`) | both, readers not followed (2048 the same) |
| `driver+0x5e8`, `+0x5e9` | `name == "05_Track" && class == 3`; `name == "20_Track"` | both: HD's two circuit flags, the name off `*0x01f99ad0 + 0xb8` |
| `driver+0xa4`, `+0xa8` | the event's `+0x2ac`, `+0x2b0` (2048 lineage); `0`, `0` (HD) | |
| `driver+0xcc` | `0.001 + 0.00065 * (1 - (1 - c)^2)` (2048); `0.001` (`0x3a83126f`, HD) | the ring rate |
| `driver+0xac..+0xc8` | `150 + 250 a`, `150 - 120 a`, `500 - 200 a`, `0.2 + 0.3 a`, `0.3 + 0.55 a`, `300 + 525 b`, `0.5 + 0.5 b` | both; with `a = b = 0` in HD lineage these are HD's constants |

The HD-lineage branch writes the two skills as `0` rather than skipping the products, so
`+0xac..+0xc8` read `150, 150, 500, 0.2, 0.3, 300, 0.5`: 2048's "HD constants" by
arithmetic. Class `3` is Phantom by the class-name table at `0x01926620` (`Venom`,
`Flash`, `Rapier`, `Phantom`, `SuperPhantom`). Confidence 80.

## The round robin and the shuffle

`AIManager_Update` (`0x012c6570`) increments `mgr+0x148`, wraps it after `7` to `0`, sets
that racer's `+0x9a0` (`0x01a07d30 + i * 0x9b0`), then runs `AI_UpdateCraft` on every
driver and the weapon AIs. HD's and 2048's pre-increment. The once-a-race shuffle (on
`mgr+0x150` set, `+0x151` clear) takes the opponents (driver `+0x08` not the player's craft
`0x01a07380`), `numAIPositionSwaps` of `AIPackSwapping<level+1>` adjacent swaps by
`rand() % (n - 1)`, the record `RM + 0x18bf8` (HD lineage) or `RM + 0x19280` (2048) plus
`class * 0x118` and `0xf0 + level * 0x18`, segments `max((int) numIntermediatePositions,
1)`, amplitude `intermediateOffsetSize`. Confidence 80.

## `AI_UpdateCraft` (`0x012c0480`)

About 3,400 lines of decompile: 2048's `AI_UpdateCraft` with `Ai_PlanLane`,
`AI_ComputeControls`, the rail and the pitch inlined (HD's `FUN_012c0480` reading, made on
the HD page, is this function). The lineage byte is read at nine sites:

| Site | Lineage byte clear (2048) | Set (HD) |
| --- | --- | --- |
| `0x012c0978` | `racer+0x9a0 = 1`: the turn flag every tick | the round robin's flag only |
| first call | the class record `RM + 0x19280 + class * 0x118` | `RM + 0x18bf8 + ...` |
| `0x012c1426` | a contact sidestep: per contact, `driver+0x178 = 1` (decaying by `dt / 3`), `+0x170` a lateral fraction, `+0x174` a speed | none |
| the pad loop after the contacts | pad tracking: walks the pads ahead (`0x01a0f320`, count `0x01a0c114`) | none |
| `0x012c2c36` | the blocked-own-lane search (own score `<= -1`), 2048's form | HD's form; no score reaches `-1` in an HD-lineage race |
| `0x012c2ff6` | the preferred lane becomes `1 - chosen^2` when the chosen score is above `0.5` | none |
| `0x012c31a0`, `0x012c3e80`, `0x012c4543`, `0x012c49b6`, `0x012c405e` | the player's own driver (autopilot, `driver+0x1c0` set): pilot assist | - |

**Not under the lineage byte:** the choice is gated on the turn flag alone (`cmp byte
[rax+0x9a0], 0` at `0x012c170d`); inside it the search runs from the own-lane candidate
with `-1` to beat, in two passes whose order is `driver+0xd0` (clear: down first, then up
through candidate `8`). After it, outside both gates, `rand() % 400 == 125` moves the
preferred lane (the divide by `0x51eb851f` at `0x012c3066`) and `rand() % 200 == 1`
redraws the coin `rand() % 2` (`0x012c30db`), every call. That is 2048's search
(`0x8119d534..0x8119d632`) and 2048's redraw, in both lineages. HD's planner
(`0x00101b50`) has the `% 400` draw and no `% 200`, and HD's `Ai_Construct` (`0x000fee30`)
has no `rand() % 2`: **Omega differs from HD here.** Confidence 78.

The rest is HD's and 2048's: the halved lookahead (`driver+0x5e4 = 1` outside lap distance
`[250, 4600]` under `+0x5e8`, `2` outside `[400, 5400]` under `+0x5e9`, `racer+0x90`), the
airborne term `(h - 15) * 0.025`, the `0.0676` lead, the ring fill from `driver+0xcc`, the
speed floor (`SplinePt+0x5e < 35`, `-0.002715 x^2 - 0.869027 x + 95.11588`, not for
`RM+0x18bf0`'s craft), the split-screen multiplier when `0x01f999e8 > 1`, the Duel block on
mode `0xd` extended to `0x15`, and the Eliminator spin on modes `8`/`0x14` or a game-mode
object flagged `1`/`4` with the craft kind `2`. Confidence 78.

## The thrust law

`AI_ComputeOpponentThrust` (`0x012c5650`) is 2048's: HD's law at the same record offsets
(`AIThrust` `+0x68`, `SpreadDist` `+0x88`, skill points `+0xd0`/`+0xe8`/`+0x100`, the
rubber band `+0xa8..+0xbc`), `75` as the finished spread, `1000 - 50 t` floored at `100`,
`2 - 0.08 t` floored at `0.5`, `+/-0.3 AIThrust`, the `100` cap after `20` s within `350`
units, the floor `1`; `WhenBehind` skipped in modes `8`/`0x14` or under a game-mode flag
`1`/`4`; the reference craft from `0x01a05440 + 0x178c8` in modes `0x10..0x16` and the human
furthest along when several play. The wander is inlined: a finished racer's slot, start
and end become `8 - place` (`racer+0x988`); otherwise HD's phase step and `rand() *
1.8626451e-09 - 1` draw. A 2048-lineage Eliminator-family mode (`> 0x16`, flag `1` or `4`)
takes a law of its own at the head of the function, 2048's `0x81199800`, unread as on 2048.
Confidence 82.

## The skill

`AI_ResolveSkillScale` (`0x012c9650`): the current game mode's `m_skillScale` (`+0x234`),
**plus `1.9` when its `+0x21c` is `2`** (not in 2048 v1.04's `0x8119fb5e`); with no game mode,
the stats record's `+0x180 + class * 0xc + level * 4`, or `2.0` with none. Then HD's mode-3
and mode-9 terms, and the campaign-cell branch: with a cell at `0x01f99990`, its
`skillEasy`/`skillMedium`/`skillHard` (`+0x18c`/`+0x188`/`+0x190` by level) placed on the
record's three-point curve, replacing what came before. HD's law (`0x000babf8`, through
`0x001e1110`), inlined. Confidence 80.

## The tables

`AiStats_Load` (`0x012c99b0`), called by `RaceManager_Construct` (`0x0128e550`) for each set,
is 2048's `AiStats_Load`: the header `950`, `80`, `0.002`, `23000`, `10`, then
`AIControlStats.xml` and `AIRaceStats_{venom,flash,rapier,phantom,superphantom}.xml`, or the
`*2048.xml` names when its flag is set. Census on the package pair (base plus patch), read by
`crates/raceplay/tests/omega_original_ai_ground_truth.rs`:

| File | Ships | Reads |
| --- | --- | --- |
| `AIRaceStats_<class>.xml`, five classes | yes | four classes whole through `oag_tables::ai_race_stats` |
| `AIRaceStats_<class>2048.xml`, five classes | yes | the same text as the base set, whitespace aside |
| `AIControlStats.xml` | yes (659 bytes) | four `<Controller>` rows |
| `AIControlStats2048.xml` | yes (813 bytes) | the same four rows plus a `SuperPhantomStats` row |
| `<circuit>\stats.xml`, all 38 `PI_Track` entries | yes | every one through `oag_tables::track_stats` |

Confidence 84 (the loader read, every file parsed).

## Flown in this project

`oag_omega::race::ORIGINAL_DRIVER` carries the HD-lineage laws: HD's four branches, HD's two
halved-lookahead rows (Omega's `Data\Plugins\tracks\Definition.xml` makes `05_Track` Ubermall
forward and `20_Track` Amphiseum reversed, as HD's does), `every_tick`, `event_ring_rate` and
`preferred_follows_choice` off, and `coin_ordered_search` on. `original_opponent` is on, so a
campaign cell flies the original's field. The circuit id is looked up in the title's
`track_plugin_definition` (`crate::load::original_laws`), which Omega's halved lookahead
needed: its `plugin_definition` is the team file.

`crates/raceplay/tests/omega_original_ai_ground_truth.rs` holds the table census, both
halved-lookahead rows resolved on the disc (and not for a Venom on Ubermall), the first
cell's skill (`grid0_3_1`, Vineta K, Venom: `0.0`, `1.225`, `3.68` at Easy, Medium, Hard off
its circuit's curve), and the field finishing on four circuits with the player parked and
no respawn: Vineta K and Talon's Junction (Venom), Ubermall (Phantom) and Amphiseum reversed
(Flash), at the race's ambient skill (`2.0`, level 1). Flown 2026-10-12 in this engine, not
on the original: all 28 opponents finished in 121.3 s, 140.3 s, 152.9 s and 184.4 s, shield
`100` at every lap crossing and at the end of the run on all four. Vineta K at the first
cell's own Easy (`0.0`, level 0) and Hard (`3.68`, level 2) finished in 133.1 s and 114.8 s,
no respawn, shield `100` throughout (a one-off run, not held by a test). The seven tests take
54 s together in isolation, the longest 22 s (Amphiseum). None of the 167 parsed campaign
cells is Super Phantom, the class with no `SpeedClass`.

**Chosen, not measured:**

- **`rand()`'s width.** The wander and the preferred lane scale `rand()` by `2^-29`
  (`1.8626451e-09`), HD's convention, which RPCS3 measured as a 30-bit `rand()` on the PS3.
  The PS4's C library was not measured; the port draws as HD does.
- **The coin's draw order** across slots: one draw a slot at construction, in slot order, as
  2048's port chose.
- **The frame rate** does not enter: an HD-lineage driver chooses on its round-robin turn and
  the per-call terms run at the original's cadence.

Not ported, as on HD and 2048: the Eliminator spin, the Duel block, the split-screen
multiplier, and every 2048-lineage law (no 2048-lineage event launches in this project's
Omega campaign: its 2048 campaign, `campaign2048_definition.xml`, is not read).

## Cross-check against 2048 and HD

- **2048: checked, applies, not wired.** Omega carries 2048's four ported laws under the same
  lineage byte (`every_tick` at `0x012c0978`, the ring rate in `AiManager_AddDriver`, the
  coin search, the preferred lane at `0x012c2ff6`) plus 2048's unported pad tracking and a
  contact sidestep; its `*2048.xml` tables equal its base set as 2048's do. One difference:
  Omega's `AI_ResolveSkillScale` adds `1.9` under a game-mode `+0x21c` of `2`, which 2048
  v1.04's does not. Wiring needs Omega's 2048 campaign.
- **HD: checked, differs.** Everything HD's map lists is here, with HD's constants; the coin
  search is not HD's.
