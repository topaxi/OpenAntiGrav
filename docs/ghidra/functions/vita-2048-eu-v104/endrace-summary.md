# The race-ending pages: `RaceSummary`, `ObjectiveSummary`, `Results`

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`, read 2026-10-06 for the end-race lane. **None is renamed**
(no row in [names.tsv](names.tsv)): the addresses below are Ghidra's own
`FUN_` names, and the confidence scores are for the *reading*, not for a name.
The screen file they drive is `NEWGUI/EndRace_Definition.xml`, laid out in
[`docs/ui/endrace-2048.md`](../../../ui/endrace-2048.md).

## Which screens a single-player race walks

**Confidence 85** (read off the decompile; the flow was then watched in this
build, not in the original). `FUN_810cd0b8` is the per-frame update of the
`EndRace` screen. Its page chain, with the state bytes `FUN_810cc556` (the
constructor) sets on `this`:

| Byte | Set from | Meaning here |
| --- | --- | --- |
| `+0x228` | event flags bit `4` | the race is online/multiplayer |
| `+0x229` | `event+0x1b0 != 0` | the event authors an objective |
| `+0x22a` | `event+0x290 == 0` | the event has a `Results` page |
| `+0x22b` | `+0x228` | a `Podium` page (online only) |
| `+0x22c` | `0` | a `Badges` page (never set here) |
| `+0x22f` | `1` | auto-advance is on |

With `+0x22f` set, a page that has held for `5.0` seconds (`*(this+0xac) > 5.0`)
hands over: `RaceSummary` -> `ObjectiveSummary` when `+0x229`, else straight to
`Results`; `ObjectiveSummary` -> `Results`; `Results` -> `Podium` only when
`FUN_812b34fa` says so (online). So a single-player campaign event walks
`RaceSummary` -> `ObjectiveSummary` -> `Results`, and an event with no objective
`RaceSummary` -> `Results`. **Open:** `*(this+0xa8)`, a second timer that starts
at `0`, reads `10.0` and fires `KillGameVita` when more than one page exists,
and is reset to `-480.0` first. It reads like an idle timeout that leaves the
race; nothing in this lane watched it fire, and this build does not implement
it.

## The tiles

`FUN_810cc556` sets each `TouchButton`'s enable bit (`+0x30`, bit `4`): the
restart tile and the exit tile on in a single-player race, the parade-laps tile
on unless the event is a Zone run (`+0x22d`), the `Near` tile only online. It
swaps `QuitTouchButton` for `QuitTouchButtonPass` once `event+0x2d8 > 2` -
the event's state byte reads as passed. **Confidence 75**: that `+0x2d8` is the
pass state is inferred from `FUN_810cf5fe`'s use of `3`/`4` as pass/elite.

## `RaceSummary`'s populate - `FUN_810cf5fe`

**Confidence 85.** Run when the screen is built. What it sets, single player:

- **Title** (`RaceSummaryTitle`): `FE_ENDRACE_SUMMARY`; `FE_END_ZONE_SUMMARY`
  for a Zone event (mode query `0`), `FE_END_COMBAT_SUMMARY` (`1`),
  `FE_END_TIMETRIAL_SUMMARY` or, for a speed lap (type `3` with `+0x18c == 0`),
  `FE_END_SPEED_LAP_SUMMARY`.
- **The objective list**: `event+0x1b0` (the pass chain), or `event+0x1c8` (the
  elite chain) once the elite state `+0x1dc == 2` and `+0x1c8` exists, which
  also sets `this+0x1e0 = 1`. Its first objective is worded by
  `GameModeObjective_FormatText` into `Objective`.
- **The verdict**, from the chain's three state words: all met -> `ER_CONGRAT`
  on `Message`, the medal shown, `PostRaceMedal+0xc8` (the sampled U, in
  texels) set to `128` (pass), `256` (elite) or `384` (a third tier this build's
  law never produces), the label `FE_PASS`/`FE_ELITE_PASS`/`FE_HARDCORE_PASS`,
  and `MessageBox`/`TotalXPBox` painted `Pass2048`, `ElitePass2048` or
  `HardcorePass2048`. Not all met -> `MessageBox`/`TotalXPBox` `0xffcd0102`,
  the medal shown at U `0`, `FE_FAIL`, `Message` `ER_END_TOUR_7`. No objective
  at all -> both `0xff525e84`, the medal hidden, `Message` empty.
- **Text colour** on the two bars: `White2048` for a pass or fail, `Blue2048`
  for an elite.
- **Speed lap**: `SpeedResultText` takes the result and `ResultBox` is hidden;
  every other mode uses `RaceResultText` in `ResultBox`. A speed lap that did
  not pass paints the bars `0xff525e84` and says `IG_HUD_BESTLAP`.
- **The result line** (`RaceResultText`/`SpeedResultText`) is `event+0x250`, a
  string some game-mode code wrote. **Not traced**; this build words a place, a
  time, a zone count or a kill count instead (chosen, not measured).
- **XP**: `RaceXP`, `PassBonusXP`, `SpeedRaceXP`, `TotalXP`, `RankText` and the
  rank bar are counted up from `event+0x1e8` and a profile total this build
  does not keep. Not drawn.

## `ObjectiveSummary`'s rows - `FUN_810d0e6e`, `FUN_810d0c46`

**Confidence 85.** Three rows, `Pass{i}`/`ElitePass{i}`/`Fail{i}` icons and
`Objective{i+1}` texts. Row `i` exists when `i < *(this+0x1cc)`; with its met
flag `m = *(this+0x1d0+i)` and the elite-chain flag `e = *(this+0x1e0)`:
`Pass{i}` shows when `m && e == 0`, `ElitePass{i}` when `m && e == 1`, `Fail{i}`
when `!m`. A single-player event has one objective per chain, so one row.

## Not read

`Results`' own table (`<RaceResults>`, filled by native code), `Podium` and
`Badges` (online), `ParadeLaps` (a free camera), the `Near` upload flow, and the
`EndRaceTips` screen (`PostRace_ShipCategoryTip`, [campaign-event-card.md](campaign-event-card.md)).

## Evidence

Decompiles of `FUN_810cd0b8`, `FUN_810cc556`, `FUN_810cf5fe`, `FUN_810d0e6e`
and `FUN_810d0c46` in `/2048/eboot-vita-2048-eu-v104.elf`; `RaceSummary` string
at `0x8144cb44`. The disc-backed check is
[`vita_2048_endrace_ground_truth.rs`](../../../../crates/game/tests/vita_2048_endrace_ground_truth.rs).
**Omega:** its `data09.psarc` ships `vita/vita_EndRace_Definition.xml`
byte-identical (22,098 bytes) to this package's v1.04 copy - see
[`docs/ui/endrace-2048.md`](../../../ui/endrace-2048.md#omega).
