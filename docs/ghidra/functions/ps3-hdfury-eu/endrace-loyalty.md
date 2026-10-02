# `EndRace Results`' loyalty block: the award law and the ticker that shows it

2026-10-02, lane `hd-endrace-loyalty`. Static reading only (Ghidra bridge,
`/hdfury/EBOOT-ps3-hdfury-eu.elf`); **no live RPCS3 run backs any row**, so
nothing here scores above the rubric's 84 ceiling for "decompilation only,
consistent call sites". Every function below reads TOC `0x008ad4d8`, `exact`
(`scripts/ps3-toc.py toc`), so Ghidra's own `PTR_`/`DAT_` names are the right
addresses; the `TOC_BASE + n` loads in the decompile were resolved with
`scripts/ps3-toc.py resolve <fn> <disp>`.

The strings the thread pointed at are at **`0x007945b0`** (`loyalty1.1`),
`0x007945c0` (`loyalty1.2`) and `0x007945d0` (`loyalty2`) - the handover's
`0x7845b0` is a digit short. `scripts/ps3-toc.py attrib` names two functions
for them, `0x00224488` and `0x00225030`, both inside `EndRaceResults_Screen.cpp`'s
cluster (`0x00222e18`, `0x00222f38`, `0x00224488`, `0x006a85f0`).

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00023f98` | `Race_ComputeLoyaltyAward` | 82 |
| `0x00224488` | `EndRaceResults_BuildLoyaltyTicker` | 78 |
| `0x00225030` | `EndRaceResults_UpdateLoyaltyTicker` | 76 |
| `0x0001b678` | `Loyalty_FindTeamRecord` | 72 |

`Race_ComputeLoyaltyAward` reuses the Pulse name (`0x0880ac50`,
`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`) because it is the same
job in the same shape - a lap/kill/zone sum, a difficulty multiplier, and
`record+? += award` capped at `100000` - but **its constants are not Pulse's**
(table below). Two independent sites agree on every rate: this function and the
ticker that prints the reasons (`0x00224488`). 82, not higher, because no
breakpoint has fired on either.

## `Race_ComputeLoyaltyAward` - `0x00023f98`

`(store, stats) -> award`, where `store` is the profile store (its `+0x450` is
a linked list of per-team records, next pointer at node `+0x50`, key at node
`+0`) and `stats` is the race's own tally block. Callers: `0x00046860` (the
`mode >= 0x10` end-of-race builder) and `0x0005ac00` (the single-player one,
which stores the result at `stats+8`). `0x0005ac00` and `0x00046860` are **not
named**: most of their bodies were not traced.

`g_GameState` is the pointer at `0x00936fe8` (`mode-manager.md`); `mode` is
`g_GameState+0xe0`. `F` is the byte at `0x009384e1` (`camera.md` records it as
"meaning not read"; it is set only by a path this build never takes, and
`0x0005ac00` substitutes `mode = 0` when it is set).

### The stats block (field meanings: 80 for laps and perfect laps, 70 for the rest)

| `stats+` | Meaning | Evidence |
| --- | --- | --- |
| `0x1330` | laps completed, clamped `0..99` | `0x00046860` stores `*(race+0x7810) - 1`, clamped to `0..99`, at `race+0x2cb0` = `0x1980 + 0x1330`; `0x0005ac00` does the same into `param_1[0xb2c]` |
| `0x1334` | perfect laps | the same two functions count the laps whose flag byte at `race+0x71e4 + 0x10 i` is non-zero into `+0x2cb4` = `0x1980 + 0x1334` |
| `0x1308` | zones cleared | ticker caption `IG_HUD_ZONE` (`0x00794608`) |
| `0x130c` | perfect zones | ticker caption `ER_PZONE` (`0x00794618`) |
| `0x1344` | kills | ticker caption `ER_ELIM` (`0x00794600`) |
| `0x134c` | difficulty tier `0` easy / `1` medium / `2` hard | `0x0005ac00` stores `g_GameState+0xdc` at `param_1[0xb33]` (= `0x1980 + 0x134c`); `race-campaign.md` reads `+0xdc` as the selected campaign rung (75) |
| `0x1354` | a byte, set when the cell's team string matches the player's | gates the multiplier *line* on non-race modes in the ticker only; the award never reads it |
| `+8` | the award | `0x0005ac00` stores `0x00023f98`'s return value at `param_1[0x662]` |

### The law

```
mode = g_GameState + 0xe0
if F == 0 and mode < 0x16:
    class = bit(mode) in 0x332318 -> A      # modes 3 4 8 9 13 16 17 20 21
            bit(mode) in 0x420    -> B      # modes 5 10
            bit(mode) in 0x4000   -> C      # mode 14
            otherwise             -> D
else:
    class = D
base  = laps      * lap_rate[class]      A 15   B 30   C 150  D 10
      + perfect   * perfect_rate[class]  A 25   B 50   C 20   D 20
extra = zones * 5 + perfect_zones * 15
if F == 0 and mode == 14:
    sum = base + extra                          # no kill term at all
else:
    kill_rate = 30 if (F == 0 and mode in {8, 0x14}) else 15
    sum = base + extra + kills * kill_rate
mult = 1
if F == 0 and mode in {3, 4, 8, 9, 13} and byte(g_GameState + 0xe4) != u32(g_GameState + 0):
    mult = {0: 2, 1: 3, 2: 4}[stats.tier]       # tier 1 -> 3, tier 2 -> 4, anything else 2
if mode > 0xf:
    mult = 3
award = mult * sum
team  = hash(*(*(g_GameState + 0xf0) + 0x78))   # 0 when no team object
rec   = list node in store+0x450 keyed by team   # same list Loyalty_FindTeamRecord walks
rec[+0x6c] += award; if rec[+0x6c] > 100000: rec[+0x6c] = 100000
return award                                     # the uncapped award
```

Where HD differs from Pulse's `Race_ComputeLoyaltyAward` (`0x0880ac50`), every
row checked against the bytes and not carried over:

| Term | Pulse | HD |
| --- | --- | --- |
| Eliminator laps / perfect laps | 10 / 20 | **15 / 25** (mode 8 is in `0x332318`) |
| Zones / perfect zones | 10 / 20 | **5 / 15** |
| Detonator (mode 14) | no such mode | laps (stages) **x150**, perfect **x20**, **no kill term** |
| Suggested-ship x2 | yes | **none** (`0x1354` only gates the displayed line) |
| Difficulty multiplier applies to | Race, Tournament, Head2Head | modes 3, 4, 8, 9, 13 **with at least one AI craft** |
| Tier 0 (easy) | x2 | **x2 as well, and there is no x1 path on a race with AI**: a project with no rung to offer has to choose one |
| `mode > 0xf` (online) | n/a | forced x3 |

"With at least one AI craft" is `byte(g_GameState+0xe4) != u32(*g_GameState)`.
`0x0005ac00` walks `g_GameState+0xe4` entries against the craft table when it
builds the standings, which reads as the human/local player count, and `*g_GameState`
as the craft count; 70, not measured live.

### What is *not* in this function

`laps`, `perfect_laps`, ... are inputs; how a race tallies perfect laps, zones
and kills is the in-race code's (`0x0005ac00` reads them out of the race state)
and was not traced. The mode ids (`3` = `SPArcade`, `4` = `SPTournament`,
`5`/`10` = `SPTimeTrial`, `8` = `SPElimination`, `6` = Zone, `14` = Detonator)
are `mode-manager.md`'s (88 for the named ones, 75 for 6 and 14), so "which
`oag_race::Mode` is which id" is a separate, lower-confidence claim from the law.

## The corroboration: the reasons ticker prints the same rates

`EndRaceResults_BuildLoyaltyTicker` (`0x00224488`), run when the Results screen
is entered, builds one **(text, running total)** line per non-zero term, in this
order, into the screen's arrays (`screen+0xc90 + 4 i` strings, `screen+0x1090 +
4 i` running totals, `screen+0xc84` the count, hard stop at `255`). Each line's
points are the rates above; the strings are TOC idstrings resolved with
`scripts/ps3-toc.py resolve`:

| Line | Count field | Idstring | Rate it adds |
| --- | --- | --- | --- |
| 1 | `0x1330` | `RC_LAP` (`0x007945e0`); `FE_STAGE` (`0x007945e8`) in class C | A 15 / B 30 / C 150 / D 10 |
| 2 | `0x1334` | `ER_PLAP` (`0x007945f8`) | A 25 / B 50 / C, D 20 |
| 3 | `0x1344`, skipped when `mode == 14` and `F == 0` | `ER_ELIM` (`0x00794600`) | 30 if mode 8 or 0x14, else 15 |
| 4 | `0x1308` | `IG_HUD_ZONE` (`0x00794608`) | 5 |
| 5 | `0x130c` | `ER_PZONE` (`0x00794618`) | 15 |
| last | difficulty | `Easy` + `x2 %s` / `Medium` + `x3 %s` / `Hard` + `x4 %s` (`0x00794628`..`0x00794648`) | the running total x2 / x3 / x4 |

Each count produces one line **per unit** (the `do { ... } while (i < count)`
loops), not one line per term. The multiplier line is added only when
`byte(g_GameState+0xe4) != u32(*g_GameState)`, and on a class other than A only
when `stats+0x1354` is set; for `mode >= 0x10` it is always `Medium x3`. Its
running total replaces the previous total with `total << 1`, `total * 3` or
`total << 2`, which is the same `mult * sum` the award function computes.

The two functions were read separately and agree on every constant, which is
the "two independent sites" leg.

### When nothing ticks

If `stats+8` (the award) is `0`, the constructor forces `stats+0xc` to `1`, and
a set `stats+0xc` skips the ticker entirely. It writes the *final* state at once:

| Widget | Text | Source |
| --- | --- | --- |
| `loyalty1.1` | `"%d %s"` of the award and the localised `ER_POINTS` (`0x007945a0`) | format `0x00794068`; `stats+8` |
| `loyalty1.2` | `""` | TOC `+0x3128`, the empty string at `0x00794528` |
| `loyalty2` | `"%d"` of the team's running total | format `0x007940f0`; `Loyalty_FindTeamRecord(store, team)` then `record + 8` |

so **an award of 0 still draws `0 POINTS` and the total**.

## `EndRaceResults_UpdateLoyaltyTicker` - `0x00225030`

`(screen, dt)`, per frame. Reads: the delay `1.0` (`0x008b0494`), the label
gap `10.0` (TOC `+0x318c` = `0x41200000`), the fade
slope `3.0` (`+0x3190`), the speed cap `600.0` (`+0x3194`), the speed base
`250.0` (`+0x3198`) and the step threshold `85.0` (`+0x319c`).

1. For the first second (`screen+0x1488 < 1.0`): all three widgets blank.
2. Then, line by line: `loyalty1.1` = `"%d"` of the line's running total; `loyalty1.2`
   = the line's text, placed `10.0` to the right of `loyalty1.1`'s right edge
   (`+0xb0` is a widget's `x`; the text width comes through vtable slot `+0x5c`);
   its alpha is `0xff - int(t * 3.0)` over a base colour `0x00646464`; `loyalty2`
   = `""`.
3. `t += (min(n^3, 600.0) + 250.0) * dt` where `n = count - index`; at `t > 85.0`
   the index advances and `t` restarts - so the first lines go by quickly (many
   remaining) and the last one slowly.
4. When the index reaches the count: the "nothing ticks" state above.

The animation is what the table above does *not* give a static capture; the final
state is. The sprite blink and the fade are documented, not reproduced by this
project's wiring (see `docs/ui/endrace-screens.md`).

## `Loyalty_FindTeamRecord` - `0x0001b678`

`(store, team_id, out*) -> record*`: hashes `team_id` (`0x0032a740`), walks the
list at `store+0x450` for the node whose key (`*node`) matches, optionally stores
`node+0x4c` through `out`, and returns **`node + 0x64`**. The ticker reads
`record + 8`, i.e. `node + 0x6c` - exactly the slot `Race_ComputeLoyaltyAward`
adds the award to (`piVar2[0x1b]`). So the total the ticker prints is the total
the award function banks.

## A clean negative: no loyalty bar

`search_strings "loyalty"` over the whole program finds exactly five strings:
`Loyalty` (`0x00786c18`), `loyalty %d: %d\n` (`0x00786cb0`), `loyalty1.1`,
`loyalty1.2`, `loyalty2`. There is **no `loyaltybar`** and no `ER_TOT_LOY` use in
`EndRaceResults_Screen.cpp`'s code: neither Results function looks up a slider,
and the Results loyalty block authors none (`docs/formats/hd-endrace-screens.md`).
The `loyaltybar` `<Slider>` belongs to the `EndRace Rewards` screen this build
never enters. **Pulse's `total * 0.00124` pixels law is therefore not ported to
HD**: HD's Results shows the total as a number only. The other two readers of the
loyalty strings (`0x00166210`, an `Unlock` XML reader that stores per-unlock
thresholds, and `0x00167a08`, which compares the profile's total against them)
are the unlock side, not drawn on Results; unread beyond that.

## Still open

- The live check: a finished race on RPCS3 reading `stats+8` and the record's
  `+0x6c`. That is what lifts every row above 84.
- What `byte 0x009384e1` is, and what `byte(g_GameState+0xe4)` / `*g_GameState`
  count (70 for "players / craft").
- How the race tallies `0x1308`, `0x130c`, `0x1344`: read as inputs only.
- Whether `0x00046860` and `0x0005ac00` can both bank the same race.
