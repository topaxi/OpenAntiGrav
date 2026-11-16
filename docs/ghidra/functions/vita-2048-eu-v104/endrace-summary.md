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
- **XP**: the widgets are filled by the populate below and counted up by
  `EndRaceSummary_UpdateXpBlock`, in "The XP block".

## The XP block - `FUN_810cf5fe` (populate) and `EndRaceSummary_UpdateXpBlock` (`0x810d056c`)

**Confidence 80** (decompile and disassembly of both; no Vita run backs a number). The update
function is the fourth word of `RaceSummary`'s vtable at `0x8150dd98` (`FUN_810d0ea0` builds the
screen object). Ghidra had no function at its entry; one was created there and named
`EndRaceSummary_UpdateXpBlock` (row in [names.tsv](names.tsv)).

**What the populate sets** (single player, `event+0x1a0 & 4 == 0`). `E` is the event's tally
`event+0x1e8`, `T` the profile total `+0x3257c` read *after* banking, `B` the pass bonus:

| Field | Value | Meaning |
| --- | --- | --- |
| `+0xc8` | `B` | the completion bonus |
| `+0xd4` | `E` | the whole tally |
| `+0xcc` | `E - B` | `RaceXP`'s end value |
| `+0xd8` | `T` | the rank counter's end value |
| `+0xdc` | `T - E` | the rank counter's start (the total before the race) |
| `+0xd0` | `B` | the `TotalXP` count's start |
| `+0xe0` | `0` | `RaceXP`'s start |

With `+0xe8 == 0` (a tap has already snapped it) every count starts at its end. A speed lap hides
`RaceXPBox` and shows `SpeedRaceXPBox`, `RaceXP` then being the `SpeedRaceXP` text, and shows no
bonus box. **`PassBonusXP` and its box show only when the event's state improved**: `+0x2d8` (this
result, 3 pass or 4 elite) is not `+0x2dc` (the best before). The bonus is `row1[mode]` for an
elite (plus `row0[mode]` when the old best was below a pass) and `row0[mode]` for a pass, the
`Event complete` rows of `XPAwards.xml` (`DAT_81577764 + row * 0x1c + mode * 4`), printed `"%d"`
(`0x8144daa4`). When the box shows, `ResultBox` ends 4 px above it (`height = box.y - result.y -
4`) and `RaceResultText` sits at half that height; otherwise `ResultBox` ends 4 px above
`RaceXPBox`, which is its authored height.

**The per-frame count** (`dt` the frame's seconds, every value a float):

- `RaceXP` text `"%d"` of `+0xe0`, which climbs `200 * dt` a frame to `+0xcc`.
- `TotalXP` text (also written to `ObjectiveSummary`'s `TotalXP`, `+0x1c8`): while `+0xd0` (from
  `B` toward `E`, `200 * dt` a frame) has not reached `E`, `RESULTS_EVENT_TOTAL` with `+0xd0`. Once
  it has, a timer `DAT_8151d838` (a global, wrapping at `6.0`) picks `RESULTS_EVENT_TOTAL` with
  `E` while it is `3.0` or less and `TOTAL_NUMBER_XP` with `T` above `3.0`. The strings are the
  disc's (`EVENT TOTAL %d XP`, `TOTAL %d XP`).
- `RankText` `"%d"` of `Profile_GetXPRank(+0xdc)`; `+0xdc` climbs `8000 * dt` a frame from
  `T - E` to `T` (`0x45fa0000`).
- **The bar**: `TotalXPFill`'s width (`+0xa8`) is `trunc(f * 48.0)` with `f = (c - thr(r)) /
  (thr(r + 1) - thr(r))`, `r = Profile_GetXPRank(c)`, `c = +0xdc`, and `f = 1.0` when the span is
  not above zero; a result of `48` with `f < 1` is `47`. The authored `45` is overwritten. This is
  the law `crates/game/tests/vita_2048_endrace_ground_truth.rs` checks against the parsed
  `XPRanks.xml`.
- **A rank-up** is detected when the rank of the counter's value this frame differs from last
  frame's (`DAT_81777f34`, `DAT_81777f38` start at `0`; `DAT_81777f34` rests at `1.25`). For
  `1.25` s `RankItem`'s width is `64 * s`, `x = 518 + (1 - s) * 32`, `RankText`'s scale `0.65 * s`,
  `s = (cos(3.2 * pi * t) + 1) / 2`; the text is the old rank for `t < 0.3125`, empty to
  `0.9375`, then the new rank. For `3.0` s `TotalXP` says `RESULTS_NEW_RANK`. (The decompile
  hides whether `SceLibm_4A496BC0` is `sin` or `cos`; `cos` makes the text swap land on `s = 0`,
  which is the reading used, confidence 65.)
- **A tap** on the panel rect (`180, 77`, `600 x 347`; `FUN_8110d0d0`, which also plays
  `newGUI_confirm`) sets `+0xe8 = 0` and every count to its end.

**Unread.** What sets `+0xe8` to `1` (the constructor is not read): this build starts the count
armed and snaps it on any page turn (chosen). `DAT_8151d838` is a global, so the first cycle's phase
depends on the last screen that ran it; this build starts the cycle when the count ends.

**Not what the page reads.** `FUN_81131100` (`rank:%d` on `xprank`, `%d / %d XP` on `xpbartext`,
`xpbar`'s width `(c - thr(r)) / (thr(r + 1) - thr(r)) * 300.0`) and `FUN_810c8286` (an
immediate-mode card: `%s %d` with `CO_RANK`, `%s = +%d %s` with `CO_NEXT_RANK` and `FE_XP`,
`CO_TOTAL_XP`, a `222` px bar, the player's craft by `CampaignEventCard_DrawForcedCraft`) are not
`RaceSummary`'s. Neither has a direct caller; they are reached through vtables. No `xprank` or
`xpbar` is authored in any of `NEWGUI/Definition.xml`, `Options_Definition.xml`,
`Team_Definition.xml`, `Skin.xml`, `Intro_Definition.xml`, `Bootup_Definition_EU.xml` or
`Profile_Definition.xml`, so the host screen of `FUN_81131100` is not found and nothing is wired
from either.

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
