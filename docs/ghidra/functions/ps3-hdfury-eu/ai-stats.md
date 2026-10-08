# AI tuning and the classic controller: Pulse's tables and Pulse's thrust law, four changes

**Binary:** `ps3-hdfury-eu` `EBOOT.elf` (PowerPC, TOC `r2 = 0x8ad4d8`).
Read 2026-10-08 for the `hd-ai-classic` lane: Ghidra (`/hdfury/EBOOT-ps3-hdfury-eu.elf`),
capstone for the AltiVec stretches Ghidra truncates (`data/scratch/hd-weapon-blasts/ppcdis.py`),
and a TOC-slot scan for the string loads (a string is loaded as `lwz rX, off(r2)` from a TOC
word holding its address, so its users are the instructions with that `off`). The Pulse side is
[`psp-pulse-usa/ai-stats.md`](../psp-pulse-usa/ai-stats.md); the design reading is
[gameplay/ai.md](../../../gameplay/ai.md). The networks HD also carries, and which never run, are
[ai-net.md](ai-net.md).

Per [ADR-0006](../../../architecture/adr/0006-no-copyrighted-content.md) no value from either XML
file is written here: the census states identity, and the code constants below are the
executable's own.

## The finding

1. **HD's two AI files are Pulse's, value for value** (confidence 95, an exact comparison of every
   attribute). `AIControlStats.xml` (20 attributes) and the four `AIRaceStats_<class>.xml` (56
   each) in `DATA02.PSARC` equal Pulse PSP EU's `Data.wad` copies after number parsing. The second
   set in `DATA03.PSARC` is the same plus one row, `<SplitScreenMultiplier Easy Medium Hard>`. The
   PSN release's `data02`/`data03` are byte-identical to the disc's. Test:
   `crates/tables/tests/ai_stats_lineage_ground_truth.rs`.
2. **HD's parser is Pulse's, with one block added** (confidence 88): the same nine functions in the
   same chain, the same tag names, the same dead `VectorStats` branch and the same
   `BaseThrust`/`BaseStartThrust` mismatch, plus `SplitScreenMultiplier` stored at `+0xc4..+0xcc`.
   That pushes `SkillScale` from `+0xc4` to `+0xd0` and the class stride from `0x10c` to `0x118`.
3. **HD's opponent-thrust law is Pulse's `AI_ComputeOpponentThrust`, line for line, with four
   changes** (confidence 85; a static read against Pulse's decompile, not a capture). They are
   in [the table below](#the-thrust-law-pulses-with-four-changes).
4. **The steering consumer is found and its law is not read.** `AI_ComputeControls`
   (`0x00100658`) takes `LookAheadSecs` and `SteerMul` from the craft's AI object, `SteerDamp` and
   `xtrackMul` from the class record, and three constants `AiStats_LoadAll` hard-codes. Past that
   it is AltiVec that Ghidra truncates.

So for HD, "drive like HD's" means Pulse's tables under Pulse's thrust law with the four changes
below. `oag-ai` ports neither the thrust schedule nor the steering constants for any title
([gameplay/ai.md](../../../gameplay/ai.md), "What we build instead"), so HD's opponents already run
on the same project-own controller as Pulse's and nothing in that controller has an HD value to take.

## Census, every title

| Title | `AIControlStats` | `AIRaceStats_<class>` | Other AI files, read? |
| --- | --- | --- | --- |
| Pulse PSP | the reference | the reference | `WeaponAIstats.xml` |
| HD disc, `DATA02` | = Pulse | = Pulse | `airacestats.xml`, `aicontrolstats_kart.xml`, `airacestats_kart.xml`: **unread**, none of their element names (`ShipSpacingVariation`, `ControllerPO`, `ZoningStats`, `Fatigue*`, `NormalRaceSpeed*`) is a string in the EBOOT |
| HD disc, `DATA03` | - | = Pulse + `SplitScreenMultiplier` | - |
| HD disc, `DATA00`/`DATA05` | - | - | `duelstats.xml` **read** (`Data\XML\DuelStats.xml`, `0x00781048`); `aiduelstats.xml` **unread**, its name is nowhere in the EBOOT |
| HD PSN | byte-identical to the disc | byte-identical | no `DuelStats.xml` string in its EBOOT (no Fury) |
| 2048 EU base | = Pulse + `SuperPhantomStats` (a copy of Phantom's) | = Pulse + `SplitScreenMultiplier`, and a `..._<class>2048.xml` set equal to it | `AIControlStats2048.xml` = its `AIControlStats.xml` |
| Omega `data00` | = HD's | = HD `DATA03`'s, and the `2048` set = 2048's | same file family as 2048 |

2048 and Omega: **checked, applies, not wired.** The data is the same table, and their executables
were not read for the law; neither title's opponents take a value from it in this project.

## Functions

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x000bd1a0` | `AiStats_LoadAll` | 90 | sets the object's header (`+0x00` `950.0`, `+0x04` `80.0`, `+0x08` `0.002`, `+0x0c` `23000.0`, `+0x10` `10.0`), defaults per class, then calls `AiStats_ParseFile` with `Data\XML\AIControlStats.xml` and the four `AIRaceStats_<class>.xml` (TOC `-0x510c..-0x50fc`); called from `0x56d18` and `0x5d460` |
| `0x000bcf10` | `AiStats_ParseFile` | 88 | finds `AIStats`, maps `VenomStats`/`FlashStats`/`RapierStats`/`PhantomStats` to `0..3`, discards `VectorStats`'s match exactly as Pulse does, record `this + 0x14 + class * 0x118` |
| `0x000bcdf0` | `AiStats_ParseClass` | 88 | dispatches `RaceBalancing` and `Controller` |
| `0x000bcc40` | `AiStats_ParseRaceBalancing` | 88 | dispatches `StartStats`, `RubberBanding`, `PosBalancing`, `SkillScale` and, new, `SplitScreenMultiplier` |
| `0x000bbf40` | `AiStats_ParseController` | 88 | `SteerMul +0x00`, `SteerDamp +0x04`, `LookAheadSecs +0x08`, `xtrackMul +0x0c`, `xtrackMax +0x10`, `xtrackDamp +0x14`: Pulse's offsets |
| `0x000bc310` | `AiStats_ParseStartStats` | 88 | `GridPlace<N>`: `BaseThrust +0x28`, `StartBoost +0x48` |
| `0x000bc0d8` | `AiStats_ParsePosBalancing` | 88 | `PlayerInPos<N>`: `AIThrust +0x68`, `SpreadDist +0x88` |
| `0x000bc930` | `AiStats_ParseRubberBanding` | 88 | `WhenLeading +0xa8..+0xb0`, `WhenBehind +0xb4..+0xbc` |
| `0x000bc560` | `AiStats_ParseSkillScale` | 88 | `SkillScalePoint<N>` at `+0xd0 + 0x18 n`, `AIPackSwapping<N>` at `+0xdc + 0x18 n` |
| `0x000bbe00` | `AiStats_ParseSplitScreenMultiplier` | 85 | `Easy +0xc4`, `Medium +0xc8`, `Hard +0xcc` (strings `0x780920..0x780930`) |
| `0x000fdf00` | `AI_ComputeOpponentThrust` | 85 | [below](#the-thrust-law-pulses-with-four-changes) |
| `0x000babf8` | `AI_ResolveSkillScale` | 82 | Pulse's function: the class's three-rung curve (`g_GameState+0xd4` class, `+0xdc` skill), default `2.0`, the mode-3 and mode-9 terms (stride `0x14`), and the campaign cell's own value through `0x001e1110` interpolated on the same curve |
| `0x000fdd18` | `AI_UpdateSpreadWander` | 60 | Pulse's `FUN_08852ef4` (unnamed there): lap-fraction lerp of two offsets plus a random wander, refreshed when its timer runs out. HD adds a finished branch, below |
| `0x000bb960` | `AIManager_Update` | 70 | once per race shuffles the AI positions `numAIPositionSwaps` times (`Libc_Rand`, `record + 0xe0 + 0x18 * skill`), then calls `AI_UpdateCraft` for each AI craft in its list (`0xbba30`) |
| `0x00102d70` | `AI_UpdateCraft` | 75 | per AI craft: caches the class record at `ai+0x168`, copies `LookAheadSecs` to `ai+0x170` and `SteerMul` to `ai+0x174`, applies the Duel override below, smooths the craft's speed into `ai+0x1a0`, and calls `AI_ComputeControls` (`0x10363c`) |
| `0x00100658` | `AI_ComputeControls` | 65 | the controls: steering off `LookAheadSecs`, `SteerMul`, the record's `SteerDamp` and `xtrackMul` and the header's three constants; calls `AI_ComputeOpponentThrust` (`0x100e38`); scales its thrust output by `SplitScreenMultiplier[skill]` when `g_GameState+0xe4 > 1` (`0x100db8`) |

`0x000ba410` repeats `AIManager_Update`'s shuffle body with no direct caller; left unnamed.

## The thrust law: Pulse's, with four changes

`AI_ComputeOpponentThrust` (`0x000fdf00`) against Pulse's (`0x08855904`, read side by side
2026-10-08). The same, in order: the spread wander call; the skill scale split at `2.0` into
`SkillScalePoint1..2` or `2..3` (`+0xd0`, `+0xe8`, `+0x100`) with the same `a(1-t) + tb` lerp; the
`PosBalancing` row `AIThrust[pos-1]` and `SpreadDist[pos] * SpreadMultiplier`; the nearest-gap scan
over the field; the target `ahead + spread * (wander - ahead's wander) - self`; the accumulator
dead band `max(1000 - 50 * time, 100)`; the position step `clamp(error * max(2 - 0.08 * time, 0.5),
+-0.3 AIThrust)`; the two `RubberBanding` terms gated off in ship states `4..6`; the final
`ThrustOffset + (x - AIThrust[0]) * ThrustMultiplier + AIThrust[0]`; and the clamp to `[1, 1000]`,
`100` after 20 s within 350 units of the reference craft. Every constant matches Pulse's
(`2.0`, `1.0`, `100000`, `-50`, `1000`, `100`, `-0.08`, `0.5`, `0.3`, `20`, `350`; TOC
`0x8a9564..0x8a95dc`), except:

| # | Change | Where |
| --- | --- | --- |
| 1 | A finished craft's spread is **`75`** (TOC `0x8a95c8`), Pulse's is `50` | `0xfdf00`, the `+0x82` branch |
| 2 | A finished craft's three wander offsets are set to **`8 - place`** (`0x8a95a8` = `8.0` minus `ship+0x994`); Pulse's wander has no finished branch | `AI_UpdateSpreadWander`, first block |
| 3 | In **Eliminator** (`g_GameState+0xe0` `8` or `20`, `SPElimination`/`MPElimination`, [mode-manager.md](mode-manager.md)) the `WhenBehind` term is dropped unless the byte at `0x9384e1` is set | `0xfdf00`, after the `WhenLeading` term |
| 4 | **Several human players**: when `g_GameState+0xe4 > 1` the reference craft is the human furthest along (`+0x7818`), not slot 0; above mode `0xf` it comes from `0x00043310` | `0xfdf00`, first block |

Which direction each rubber-band row pushes, read here (confidence 70, it rests on `+0x898` being
progress that grows forward): the `WhenLeading` term is **added** when the reference craft leads every
opponent by more than its dead band, and the `WhenBehind` term is **subtracted** when every opponent
leads the reference craft by more than its dead band. So the larger `WhenBehind` multiplier is the
opponents easing off for a player who is behind, not a catch-up. The two titles' code agrees, so this
reading applies to Pulse too.

None of this reaches `oag-ai`: it refuses the player-coupled terms on every title. The one place the
law is ported, the finished player's thrust cap (`oag_raceplay::finished_thrust`), uses its
lower-stop regime, which reads neither change 1 nor change 2. On HD that cap is **Pulse's law,
unmeasured on HD**.

## The Duel override (Fury), and a clean negative

`AI_UpdateCraft`, in mode `0xd` with the `0x9384e1` byte clear, walks the skill level's `Input` rows
(up to ten, `{Zone, LookAheadSecs, TurnMul}` at `RaceManager + 0x2f2c`, parsed by `0x000be3f0`) and,
for each `Zone` threshold the craft's zone has reached, overwrites `ai+0x170`/`ai+0x174`. So in a
Duel the `Controller`'s `LookAheadSecs` and `SteerMul` are replaced zone by zone; the file's own
comments name the second field as the turn multiplier, which is `SteerMul`'s slot.

The file authors `turnMul` and the executable's only spelling is `TurnMul` (`0x00780d00`). **That is
not a dead attribute**: the attribute compare (`0x00676328` to `0x00346708`) is `strcasecmp`, so the
two match. This also means HD's whole parser matches attributes case-insensitively, which does not
rescue `BaseStartThrust` (different letters).

## Three extra tags and who reads them (static, 2026-10-08)

Read off `EBOOT.elf` by the `hd-ai-classic` lane after its merge; static, not run.

- **`SplitScreenMultiplier`**: `AiStats_ParseSplitScreenMultiplier` stores it at record `+0xc4/+0xc8/+0xcc`.
  `AI_ComputeControls` reads it at `0x100db8`: when `g_GameState+0xe4 > 1` (more than one local player) the
  thrust output (`out+4`) is multiplied by `record[+0xc4 + 4*skill]`. A single-player race never reads it.
  Confidence 80.
- **`VectorStats`**: `AiStats_ParseFile` compares the name and discards the result, as Pulse does, so no class
  index is assigned and nothing reads it. HD's classed files author none (only the unread `_kart` file does).
  Confidence 85.
- **`xtrackMax`** (record `+0x10`): parsed, no reader found. Every load through the craft's cached record pointer
  (`ai+0x168`) touches only `+0x00..+0x0c` and `+0xa8..+0xbc` (`0xfe25c`, `0x100a40`, `0x100c9c`, `0x100db8`,
  `0x1030f8`), and HD's `AIControlStats.xml` authors no `xtrackMax`. Confidence 60 that nothing reads it: the
  unread AltiVec past `0x100734` is where a reader would be.

## Not read

- **The steering law inside `AI_ComputeControls`** past its inputs: `0x100658..0x100df7`, AltiVec
  from `0x100734`. Its outputs go to the struct at `r4` (`+0` byte, `+4` thrust, `+8`, `+0x10`,
  `+0x14`). The next address is the call `0x000fe688` (`0x100954`, `0x100a04`), which takes two floats in
  `f1`/`f2`. Pulse's own consumer of these fields is not identified either
  ([psp-pulse-usa/ai-stats.md](../psp-pulse-usa/ai-stats.md)), so there is no Pulse law to compare
  with yet.
- **What the `0x9384e1` byte is.** It gates the Duel override and change 3 alike.
- No live capture: every row here is static. A RPCS3 read of `ai+0x170`/`+0x174` in a Racebox race
  would raise the `AI_UpdateCraft` row.
