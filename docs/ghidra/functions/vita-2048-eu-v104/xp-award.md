# 2048's XP: a per-profile rank system, not loyalty

2026-10-16, lane `xp-loyalty`. Ghidra `/2048/eboot-vita-2048-eu-v104.elf`, tables read
from `data.psarc` (`data/xml/XPAwards.xml`, `data/xml/XPRanks.xml`). Static reading only;
**no live Vita3K run backs any row**, so nothing scores above the rubric's 84 ceiling.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x8101311c` | `Profile_Construct` | 70 |
| `0x81012ecc` | `XPAwards_Load` | 84 |
| `0x81012c80` | `XPAwards_ParseAward` | 80 |
| `0x81012ad0` | `XPRanks_Load` | 84 |
| `0x810134ce` | `Profile_AwardXP` | 80 |
| `0x8101340a` | `Profile_BankRaceXP` | 78 |
| `0x810133da` | `Profile_GetXPRank` | 84 |

## What it is

`Profile_Construct` (`0x8101311c`, singleton `DAT_81545468`) loads the two tables
(`XPAwards_Load(this, "Data\\XML\\XPAwards.xml")`, `XPRanks_Load(this, ...)`). The XP total
is a `u32` at profile `+0x3257c`. It is **one total per profile**, not per team.

**Awards** are a 78-row table (indices 0-77, the destructor loops `0x4e` times) at `0x81577760`, stride `0x1c`: `+0` name string, `+4` zone,
`+8` elim, `+0xc` zombie, `+0x10` race, `+0x14` speed (the XML's five per-mode values, in
that order), `+0x18` group string (`AWARD_GROUP_AGILITY`/`COMBAT`/`SPEED`).
`Profile_AwardXP(profile, awardIndex, useClassMult, useSkillMult)`:
`xp = row[mode] * ClassXPMultipliers[class] (if flag) * SkillXPMultipliers[skill] (if flag)`,
the pilot's race tally gets it through `FUN_812b3a3c(pilot, xp, groupName)`, and a second
award inside a 2.0 s window counts toward a chain: five in a row pays `chain * 2` as
`AWARD_GROUP_CHAIN`. Both multiplier tables are all `1.0` on this disc. About 30 call
sites (gameplay events: barrel roll, perfect lap, kill, pad hit, boost start, zones, air
time, evading a weapon, finishing position, "held 1st for N s").

`XPAwards.xml` (78 rows, `index` 0-77): the terms are **events**, not a lap/kill/zone
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
Read by `RaceSummary`'s populate and update (`FUN_810cf5fe`, `EndRaceSummary_UpdateXpBlock`: `RaceXP`,
`PassBonusXP`, `TotalXP`, `RankText`, `TotalXPFill`, `RESULTS_NEW_RANK` on a rank-up;
[endrace-summary.md](endrace-summary.md), "The XP block"), by two readouts that are not the end-race
page (`FUN_81131100`: `xprank`, `xpbartext`, `xpbar`; `FUN_810c8286`: `CO_RANK`, `CO_NEXT_RANK`,
`CO_TOTAL_XP`; host screens not found) and by a Vita activity-feed string (`FUN_81013c8e`, the rank
is the payload).

**The end-race page reads the completion award as a first-earn bonus.** `PassBonusXP` shows only
when the event's result state improved on the best before (`+0x2d8 != +0x2dc`), and is `Event
complete- ELITE` (plus `PASS` when the old best was below a pass) or `PASS`. That points at the
completion awards paying on improvement, not on every finish, and at an elite on a first pass paying
both. `Store::bank_event_xp` still pays one completion award on every finish and Elite instead of
Pass (chosen, above); the page shows the bonus that was banked, so the numbers add up. Aligning the
banking with the page is open.

## Verdict on "2048's XP is its loyalty"

**Falsified, confidence 84** (the rubric's decompile-only ceiling; the authored XML is a second source for the table, not for the live behaviour). Against the falsifier the lane was given: 2048's XP is a
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

**Correction (2026-11-12, [canvas-music-wave.md](canvas-music-wave.md), section 3):** a rank does gate
an unlock. `GameModeBase_IsUnlockable` (`0x812b3a76`) compares the XP total `+0x3257c` with
`Profile_GetRankThresholdXP(m_RankRequired)` (`0x8101361a`, this page's rank table), and
`Campaign_UnlockEvents` (`0x812b5b6c`) holds the fifteen side challenges until it passes. The gate
lives in the event objects, not in `Unlocks_Definition.xml`, which is why the XML census missed it.

## In-race awards (2026-11-12, lane `w2048-race-xp`)

Ghidra `/2048/eboot-vita-2048-eu-v104.elf`, 70 xrefs to `Profile_AwardXP` from 31 functions and
two bare call sites. Confidence 75 for the chain rule (decompile of `0x810134ce` plus the
per-frame decrement at `0x81013708`), 70 for each wired site's condition. No live run.

**The chain rule, exact.** `Profile_AwardXP` drops an award whose column value is 0 (nothing
paid, the chain untouched). Otherwise it adds the value to the pilot's race tally
(`FUN_812b3a3c`, `pilot+0x1e8`), and then: if the chain timer `+0x32668` is above 0, the chain
counter `+0x32674` rises by one; the timer is set to 2.0; if the counter is 5, the tally gets
`counter << 1` (10) as `AWARD_GROUP_CHAIN` and the counter resets to 0. So the first award
of a chain does not count: **six awards** inside the window reach 5. A bonus pays once per
five, not on every award after. Per frame (`0x81013708`) the timer runs down by the frame time
and, when it reaches 0 or less, the counter is zeroed. The race-end awards go through the same
function (flags 0,0) and chain like any other. The class and skill multipliers are 1.0 on this
disc and are not applied.

**The shared gate** `FUN_8100d2a6` is false when `DAT_8151f0e4 == 0`, when mode id
`DAT_8153fd24 == 2`, when `GameMode_Current()` is the mode object `DAT_8151d438`, or unless the
race object's phase `+0x11d50 == 2` (racing). Which modes `2` and `DAT_8151d438` are is unread.
Almost every site also needs the pilot to be human (`pilot+0x58dc == 0`).

**Ours.** `oag_raceplay::xp_tally::XpTally` is armed from the event's `XPAwards.xml` and column
and fed from the simulation's edges, outside the state hash: the zone step in
`advance_player_rules`, and kills and place each tick in `Race::tick` while the player is
racing. The 2.0 s window is 120 fixed ticks, standing for the original's variable timestep.
At the finish `Store::bank_event_xp` runs the completion and `Finished Nth` awards through the
tally and banks its total.

| Index | Award | Status |
| --- | --- | --- |
| 0 | Event complete- PASS | **wired** at the finish, `0x812b3f90` (flags 0,0, joins the chain); Pass or Elite by medal, **chosen** |
| 1 | Event complete- ELITE | **wired** at the finish, `0x812b3f90` (flags 0,0, joins the chain); Pass or Elite by medal, **chosen** |
| 2 | Finished multiplayer race | n/a: multiplayer (`0x81166252`, `0x812bbac6`, `0x812beb48`); no online 2048 |
| 3 | Won multiplayer race | n/a: multiplayer (`0x81166252`, `0x812bbac6`, `0x812beb48`); no online 2048 |
| 4 | Barrel roll | unwired (condition head unread, `0x811bf7aa`) |
| 5 | Perfect lap | unwired (pilot flag `+0x60a4 & 0x20` unread, `0x811c2ad6`) |
| 6 | Opponent destroyed | **wired** (`0x812ccfe0`, `0x8123b5ea`, `0x811c4a06`: human killer, `FUN_8100d2a6`); fed from `Standing::kills` |
| 7 | Speed pad hit | unwired (extra suppressor `pilot+0x6128 == 0` unread, 26 readers, `0x811ccdf4`) |
| 8 | Weapon pad hit | unwired (condition head unread, `0x811c9c7a`) |
| 9 | Zone | **wired** (`0x812c4052`, `0x81186862`): a zone step with a wall contact |
| 10 | Perfect Zone | **wired** (same sites): a clean zone step; our `zone_dirty` stands for flag `+0x60a4 & 0x1000000`, **chosen** |
| 11 | Boost start | unwired (flag `*(+0x5640)+0x595` unread, `0x81175fc8`) |
| 12 | Hit with Rocket | unwired (weapon-type case to award map, cases 0-7 and 10, not tied to our weapon ids, `0x811c42d2`) |
| 13 | Hit with Missile | unwired (weapon-type case to award map, cases 0-7 and 10, not tied to our weapon ids, `0x811c42d2`) |
| 14 | Hit with Plasma | unwired (weapon-type case to award map, cases 0-7 and 10, not tied to our weapon ids, `0x811c42d2`) |
| 15 | Hit with Cannon | unwired (weapon-type case to award map, cases 0-7 and 10, not tied to our weapon ids, `0x811c42d2`) |
| 16 | Hit with Mine | unwired (weapon-type case to award map, cases 0-7 and 10, not tied to our weapon ids, `0x811c42d2`) |
| 17 | Hit with Quake | unwired (weapon-type case to award map, cases 0-7 and 10, not tied to our weapon ids, `0x811c42d2`) |
| 18 | Hit with Bomb | unwired (weapon-type case to award map, cases 0-7 and 10, not tied to our weapon ids, `0x811c42d2`) |
| 19 | Hit with Leach Beam | unwired (weapon-type case to award map, cases 0-7 and 10, not tied to our weapon ids, `0x811c42d2`) |
| 20 | Hit with Light Barrier | unwired (weapon-type case to award map, cases 0-7 and 10, not tied to our weapon ids, `0x811c42d2`) |
| 21 | Taken 1st! | unwired (initial value of the best-place field `+0x6f3c` unread, `0x812bbeb2`) |
| 22 | Taken 2nd! | unwired (initial value of the best-place field `+0x6f3c` unread, `0x812bbeb2`) |
| 23 | Taken 3rd! | unwired (initial value of the best-place field `+0x6f3c` unread, `0x812bbeb2`) |
| 24 | Finished 1st! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 25 | Finished 2nd! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 26 | Finished 3rd! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 27 | Finished 4th! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 28 | Finished 5th! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 29 | Finished 6th! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 30 | Finished 7th! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 31 | Finished 8th! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 32 | Shielded against Rocket | unwired (weapon-type case to award map, `0x811bd214`) |
| 33 | Shielded against Missile | unwired (weapon-type case to award map, `0x811bd214`) |
| 34 | Shielded against Plasma | unwired (weapon-type case to award map, `0x811bd214`) |
| 35 | Shielded against Cannon | unwired (weapon-type case to award map, `0x811bd214`) |
| 36 | Shielded against Mine | unwired (weapon-type case to award map, `0x811bd214`) |
| 37 | Shielded against Quake | unwired (weapon-type case to award map, `0x811bd214`) |
| 38 | Shielded against Bomb | unwired (weapon-type case to award map, `0x811bd214`) |
| 39 | Shielded against Leach Beam | unwired (weapon-type case to award map, `0x811bd214`) |
| 40 | Shielded against Light Barrier | unwired (weapon-type case to award map, `0x811bd214`) |
| 41 | Held 1st for 30sec | **wired** (`0x811712aa`): first place held, human, rivals present, not elimination; our place is `oag_race::places` |
| 42 | Held 1st for 20sec | **wired** (`0x811712aa`): first place held, human, rivals present, not elimination; our place is `oag_race::places` |
| 43 | Held 1st for 10sec | **wired** (`0x811712aa`): first place held, human, rivals present, not elimination; our place is `oag_race::places` |
| 44 | Cross Start/Finish Line with full health | unwired (health test unread, `0x811c2ad6`) |
| 45 | 5 seconds ahead | unwired (lap-time lead test only partly read, `0x811c2800`) |
| 46 | 10 seconds ahead | unwired (lap-time lead test only partly read, `0x811c2800`) |
| 47 | 15 seconds ahead | unwired (lap-time lead test only partly read, `0x811c2800`) |
| 48 | Hit Zone 10 | **wired** (`0x812c4052`): zone number a multiple of 10 up to 100, index `0x2f + zone/10` |
| 49 | Hit Zone 20 | **wired** (`0x812c4052`): zone number a multiple of 10 up to 100, index `0x2f + zone/10` |
| 50 | Hit Zone 30 | **wired** (`0x812c4052`): zone number a multiple of 10 up to 100, index `0x2f + zone/10` |
| 51 | Hit Zone 40 | **wired** (`0x812c4052`): zone number a multiple of 10 up to 100, index `0x2f + zone/10` |
| 52 | Hit Zone 50 | **wired** (`0x812c4052`): zone number a multiple of 10 up to 100, index `0x2f + zone/10` |
| 53 | Hit Zone 60 | **wired** (`0x812c4052`): zone number a multiple of 10 up to 100, index `0x2f + zone/10` |
| 54 | Hit Zone 70 | **wired** (`0x812c4052`): zone number a multiple of 10 up to 100, index `0x2f + zone/10` |
| 55 | Hit Zone 80 | **wired** (`0x812c4052`): zone number a multiple of 10 up to 100, index `0x2f + zone/10` |
| 56 | Hit Zone 90 | **wired** (`0x812c4052`): zone number a multiple of 10 up to 100, index `0x2f + zone/10` |
| 57 | Hit Zone 100 | **wired** (`0x812c4052`): zone number a multiple of 10 up to 100, index `0x2f + zone/10` |
| 58 | Air Time | unwired (condition unread, `0x811d0370`) |
| 59 | No wreck for 30 seconds | unwired (no call site found among the 70 xrefs; the index is not a literal) |
| 60 | No wreck for 60 seconds | unwired (no call site found among the 70 xrefs; the index is not a literal) |
| 61 | No wreck for 90 seconds | unwired (no call site found among the 70 xrefs; the index is not a literal) |
| 62 | No wreck for entire event | unwired (condition unread, `0x8100bfba`) |
| 63 | Shoot/Destroy Mine | unwired (`FUN_8120fae0` test unread, `0x811dfa0e`, `0x811fe134`, `0x812060a4`, `0x811f89fa`) |
| 64 | Shoot/Destroy Bomb | unwired (`FUN_8120fae0` test unread, `0x811dfa0e`, `0x811fe134`, `0x812060a4`, `0x811f89fa`) |
| 65 | No hits for 10 seconds | unwired (no call site found among the 70 xrefs; the index is not a literal) |
| 66 | No hits for 20 seconds | unwired (no call site found among the 70 xrefs; the index is not a literal) |
| 67 | No hits for 30 seconds | unwired (no call site found among the 70 xrefs; the index is not a literal) |
| 68 | No hits for 40 seconds | unwired (no call site found among the 70 xrefs; the index is not a literal) |
| 69 | No hits for 50 seconds | unwired (no call site found among the 70 xrefs; the index is not a literal) |
| 70 | No hits for 60 seconds | unwired (no call site found among the 70 xrefs; the index is not a literal) |
| 71 | Successful Turbo | unwired (condition unread, `0x811bf07e`) |
| 72 | Hit with cannon full clip | unwired (condition unread, `0x811e0b56`) |
| 73 | Hit with all 3 rockets | unwired (condition unread, `0x81206f6e`) |
| 74 | Evade Mine | unwired (`piVar12[0x13]` flag unread, `0x811f453e`) |
| 75 | Evade Bomb | unwired (flag unread, `0x811dbb5a`) |
| 76 | Evade Missile | unwired (condition unread, `0x811f89fa`) |
| 77 | Evade Zombie | unwired (no call site found among the 70 xrefs; the index is not a literal) |

Not wired by anything here, in the original: the rank bar and the `RaceXP` popups.

**Omega check.** Omega ships a byte-identical `XPAwards.xml` and the same loader: **checked,
applies, not wired** (Omega's event launch banks nothing yet; the tally takes any
`Awards`, so the wiring is arming it from Omega's event progress).

## Cross-check

- **Omega**: XP - checked, applies, not wired for events (`oag_tables::xp` parses Omega's `XPAwards.xml` and `XPRanks.xml`, ground truth in `crates/omega/tests/omega_title_ground_truth.rs`; the profile total is shared, Omega's event launch does not bank yet); ranks - checked, differs. Omega's `eboot.bin` carries the same
  loader (`FrontendRoot_Construct` references `Data\XML\XPAwards.xml` at `0x0182eb7a`) and
  its `data00.psarc` ships a **byte-identical** `Data/xml/XPAwards.xml` (9177 bytes) and a
  different `XPRanks.xml` (same 50 ranks, thresholds exactly **half** of 2048's: 15000,
  30000, 50000, ...). See `ps4-omega-eu/xp-and-loyalty.md`.
- **HD**: checked, differs (no XP; loyalty only). Wiring left open in a handover thread.

## Wired (2026-11-12)

`oag_tables::xp` reads both files; `Store::bank_event_xp` banks a finished 2048 event's
`Event complete` (Pass or Elite) plus `Finished Nth`. **Chosen, not measured**: pays on every
finish; Elite instead of Pass; column by event kind. The profile total is one number shared with
Omega.

**Shared XP across 2048 and Omega (maintainer rule, 2026-11-12).** One XP total per player
profile: a race in either title adds to it and either title's map reads it for the hidden-event
gate. It is stored as the top-level `xp` key of the profile-wide `records.toml` (`Store::xp`), not
in a title-keyed row, so no cross-title store needed building. Awards come from 2048's
`XPAwards.xml`, which Omega ships byte-identical. The rank tables are **not** identical: Omega's
`XPRanks.xml` has half the thresholds, so each title ranks the shared total by its own table (the
same total is a higher Omega rank). Decided (maintainer, 2026-11-12): each title ranks by its own table.

## When a hidden event is promoted (2026-11-13, lane `w2048-xp-unlock-re`)

Ghidra `/2048/eboot-vita-2048-eu-v104.elf`, decompile and disassembly; `SP.xml` re-read from the
v1.04 `data.psarc` (`data/xml/SP.xml`, 1,316,363 bytes; the patch's `data1.psarc` copy differs by one
byte at offset 1,291,041 and gives the same census below). Builds on
[canvas-music-wave.md](canvas-music-wave.md), section 3, which read `Campaign_UnlockEvents` and
`GameModeBase_IsUnlockable`; this section reads the flag and the triggers that section left open.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x812b40b4` | `GameModeBase_IsChainMember` | 80 |
| `0x812b40ba` | `GameModeBase_MarkChain` | 82 |
| `0x812b4872` | `GameModeBase_OnRaceEnd` | 78 |
| `0x810f041e` | `TouchCampaign_BuildEventButtons` | 72 |

**The rule (confidence 80, decompile only; two call sites and Omega's copy agree).** Promotion is
`Campaign_UnlockEvents` (`0x812b5b6c`) and nothing else. It runs at exactly two moments:

1. **Race end**, after the race's XP is banked: `GameModeBase_OnRaceEnd` (`0x812b4872`) calls
   `Profile_BankRaceXP` first, shows the unlock cards, then `Campaign_UnlockEvents` at `0x812b49d8`.
2. **Building the map's event buttons**: `TouchCampaign_BuildEventButtons` (`0x810f041e`, slot `+0x2c`
   of the TouchCampaign vtable at `0x8150e958`, the slot the base widget fills with `0x81064900`) calls
   it at `0x810f0800`, before it creates a button for every event whose button `+0x1ec` is still null.

In that pass an event is promoted when all of these hold:

- progress `+0x2d8 == 0`;
- **not a chain member**: `GameModeBase_IsChainMember` (`+0x2e5`) is 0. `GameModeBase_PostLoad`
  (`0x812b40de`) calls `GameModeBase_MarkChain` (`0x812b40ba`) on every event with a next (`+0x2cc`)
  or branch (`+0x2d0`) event, and `MarkChain` sets `+0x2e5 = 1` on the event and recursively on
  everything reachable along next and branch. So the numbered campaign is never promoted by the
  loop; the chain opens through `SetProgress` (a pass opens next and branch at 2). The three season
  heads `2048/2049/2050 - Event 1` are chain members too (each has a next), which is why the
  function opens them by name before the loop (to 2, same unlockable test). Two reads agree on this
  meaning of `+0x2e5`: the setter, and the head special case that is only needed if heads fail the
  loop's test;
- `GameModeBase_IsUnlockable` (`0x812b3a76`): force flag `+0x2e4`, or XP total `profile+0x3257c >=
  Profile_GetRankThresholdXP(m_RankRequired)` and the required event `+0x2d4` null or at progress 3+.

The new progress is **1** when the hidden test (vtable `+0x74`, `m_options & 0x10`) or `m_options & 8`
holds, else **2**. A one-shot news headline builder (`FUN_812b274a`, guarded by `+0x326`) runs for
every event in the list first; it gates nothing.

**The fifteen side challenges pass every gate but the rank** (census of `SP.xml` v1.04): none has a
next, branch or required event, none is the target of another event's next, branch or required
edge, so `+0x2e5` stays 0 and `+0x2d4` is null; `M_Options` is 16 (`0x10`, hidden) on thirteen and 17
on Qirex and Auricom P Ship Challenge (also hidden). `M_RankRequired`: Feisar 10, Auricom 20,
AG-Systems 26, Qirex 30, Pirhana 50 (P Ship Challenges); Square 25, Mall 26, Park 27, Bridge 28,
Tower 29, Arena 30, Cathedral 31, Subway 32, Sol 33, Altima 34 (S Phantom Challenges). So **a side
challenge goes to progress 1 at the first race end or map build at which the XP total has reached
its rank's `XPRanks.xml` threshold**, and a tap on its spot takes it to 2 (canvas-music-wave.md).

**Why the rank-50 test promoted nothing (explanation, confidence 60).** That test wrote the XP total
into a running game whose map was already built and did not finish a race, so neither trigger ran
after the write. The law above needs no extra gate to explain it. Not yet confirmed live: the check
is to write `profile+0x3257c` before the first map entry of a boot, then read a side challenge's
`+0x2d8` on the map (expect 1); a 0 there would point at the event list `Campaign_UnlockEvents`
walks (`0x81965b28`, vector at `+0x24`) not holding the side challenges, since the button builder
walks a different list (`0x81965b60`, vector at `+4`).

**Ours.** `Store::rank_reveals`, asked at boot, is the map-build trigger. The race-end trigger is not
wired (a reveal earned by a race shows at the next launch); wiring it means asking the same query
after `Store::bank_event_xp`.

**Omega: checked, applies, not wired.** Omega's `eboot.bin` has the same function at `0x015a1200`
(found through `"2049 - Event 1"`): the three heads by name to 2, then the loop with
`IsUnlockable` inlined (force `+0x3cc`; XP `DAT_02039628+0x4dc` against the rank table
`DAT_0203a570[rank - 1]`, rank `+0x3c8`; required event `+0x3b8` at progress `+0x3c0 > 2`), the chain
flag at `+0x3cd`, and 1 versus 2 by vtable `+0xe8` or `+0x22c & 8`. Its two callers are the same
pair: `FUN_0159dea0` and the campaign map's button builder `0x014e32c0`. Only the offsets differ.
Omega's map does not launch events yet, so no Omega reveal is wired.
