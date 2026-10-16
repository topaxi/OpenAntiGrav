# 2048's XP: a per-profile rank system, not loyalty

2026-10-16, lane `xp-loyalty`. Ghidra `/2048/eboot-vita-2048-eu-v104.elf`, tables read
from `data.psarc` (`data/xml/XPAwards.xml`, `data/xml/XPRanks.xml`). Static reading only;
**no live Vita3K run backs any row**, so nothing scores above the rubric's 84 ceiling.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x8101311c` | `Profile_Construct` | 70 |
| `0x81012ecc` | `XPAwards_Load` | 85 |
| `0x81012c80` | `XPAwards_ParseAward` | 80 |
| `0x81012ad0` | `XPRanks_Load` | 85 |
| `0x810134ce` | `Profile_AwardXP` | 80 |
| `0x8101340a` | `Profile_BankRaceXP` | 78 |
| `0x810133da` | `Profile_GetXPRank` | 85 |

## What it is

`Profile_Construct` (`0x8101311c`, singleton `DAT_81545468`) loads the two tables
(`XPAwards_Load(this, "Data\\XML\\XPAwards.xml")`, `XPRanks_Load(this, ...)`). The XP total
is a `u32` at profile `+0x3257c`. It is **one total per profile**, not per team.

**Awards** are a 78-row table at `0x81577760`, stride `0x1c`: `+0` name string, `+4` zone,
`+8` elim, `+0xc` zombie, `+0x10` race, `+0x14` speed (the XML's five per-mode values, in
that order), `+0x18` group string (`AWARD_GROUP_AGILITY`/`COMBAT`/`SPEED`).
`Profile_AwardXP(profile, awardIndex, useClassMult, useSkillMult)`:
`xp = row[mode] * ClassXPMultipliers[class] (if flag) * SkillXPMultipliers[skill] (if flag)`,
the pilot's race tally gets it through `FUN_812b3a3c(pilot, xp, groupName)`, and a second
award inside a 2.0 s window counts toward a chain: five in a row pays `chain * 2` as
`AWARD_GROUP_CHAIN`. Both multiplier tables are all `1.0` on this disc. About 30 call
sites (gameplay events: barrel roll, perfect lap, kill, pad hit, boost start, zones, air
time, evading a weapon, finishing position, "held 1st for N s").

`XPAwards.xml` (77 rows, `index` 0-77): the terms are **events**, not a lap/kill/zone
sum. Campaign `Event complete - PASS` / `ELITE` pay 10000 each in every mode; multiplayer
finish/win 10; `Perfect lap` 30; `Opponent destroyed` 50; barrel roll, speed pad, weapon pad
10; boost start 30; `Finished 1st`..`8th` 50..5 (0 in zombie after 1st); weapon hit/shield
1..15; `Hit Zone 10`..`100` 10..50 (zone mode only); `Air Time` 5-10; `No wreck for 30/60/90 s
/ entire event` 5/10/15/25 (elimination only).

**Ranks**: `XPRanks.xml` is `MaximumRank 50` and 50 cumulative thresholds
(30000, 60000, 100000, 140000 ... 3580000). `Profile_GetXPRank(total)` = the count of
thresholds `<=` total (the first index whose threshold exceeds it), capped at 50.

**Banking**: `FUN_812b4872` (the race-end hook of the event object) calls
`Profile_BankRaceXP`: `total += pilot[+0x1e8]` (the race's XP tally), then fires platform
trophies `0x18` when the rank first exceeds 24 and `0x19` when it exceeds 49. That hook
also shows the per-event unlock cards (`Post_Race_Speed_Laps_Unlocked`,
`Prototype_Challenge_Unlocked`, `A_Plus_Challenge_Unlocked`, `Unlock_Chrome_Skins`), and
those key on the **event's own unlock flags (`FUN_812b5c98(.., 4|5|6|8)`), not on XP**.
Read by the EndRace screens (`FUN_81131100`, `FUN_810c8286`: `CO_RANK`, `CO_NEXT_RANK`,
`CO_TOTAL_XP`, the `xprank`/`xpbar` widgets, `RESULTS_NEW_RANK`/`Tut_Rank_FE_Overlay` on a
rank-up) and by a Vita activity-feed string (`FUN_81013c8e`, the rank is the payload).

## Verdict on "2048's XP is its loyalty"

**Falsified, confidence 85.** Against the falsifier the lane was given: 2048's XP is a
per-profile level (50 ranks off cumulative thresholds) with its own terms; none of
loyalty's terms appear in it. Loyalty (HD law, `ps3-hdfury-eu/endrace-loyalty.md`) is
`mult * (laps*rate + perfect*rate + zones*5 + perfect_zones*15 + kills*rate)` per **team**,
capped at 100000, with a difficulty multiplier by mode class. 2048's XP is paid per event as
it happens, has no lap/zone/kill per-race sum, no mode-class lap rate, no difficulty
multiplier (the skill table is all 1.0), no per-team bank and no cap (the table ends at
rank 50 but the total keeps counting). The one shared shape is "a number a race adds to a
running total and a bar shows", which is why the end-race page reads alike. The reverse
check: the page that draws XP in 2048's `EndRace_Definition.xml` has no loyalty widget.

What XP **gates**: only the rank itself (trophies at 25 and 50, the rank bar and the
`mp_level_N` icons, which read as the MP "level" = rank in the Unlocks page). **No unlock
row was found that needs a rank.** The `<Unlock>` parser's `Loyalty` attribute
(`FUN_81065ea2`, record `+0x48`) is unused by 2048's own `Unlocks_Definition.xml`, which has
no `rank`/`xp` attribute either (confidence 70: the rank-gate negative rests on one XML and
three of the eight `Profile_GetXPRank` callers decompiled).

## Cross-check

- **Omega**: ported-as-checked, applies, not wired. Omega's `eboot.bin` carries the same
  loader (`FrontendRoot_Construct` references `Data\XML\XPAwards.xml` at `0x0182eb7a`) and
  its `data00.psarc` ships a **byte-identical** `Data/xml/XPAwards.xml` (9177 bytes) and a
  different `XPRanks.xml` (same 50 ranks, thresholds exactly **half** of 2048's: 15000,
  30000, 50000, ...). See `ps4-omega-eu/xp-and-loyalty.md`.
- **HD**: checked, differs (no XP; loyalty only).
