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

### Completion pays on improvement

**The end-race page reads the completion award as a first-earn bonus.** `PassBonusXP` shows only
when the event's result state improved on the best before (`+0x2d8 != +0x2dc`), and is `Event
complete- ELITE` (plus `PASS` when the old best was below a pass) or `PASS`. That points at the
completion awards paying on improvement, not on every finish, and at an elite on a first pass paying
both (confidence 70: the page's branch, not the award call sites, which `0x812b3f90` and the pass
path were not re-read for the improvement test). **Ours** (`XpTally::finish`, `Completion::on_improvement`):
the best before the race is read from the campaign row before the race's medal is recorded
(`Store::campaign_medal`'s `best_medal`, Gold is Elite, any other medal Pass); Pass pays when no pass
stood, Elite when the result is Elite over a lesser best, both for an elite first pass, none for a
repeat or a worse result. `Finished Nth` still pays on every finish (**chosen, not measured**). The
page's `PassBonusXP` is the completion payout the tally recorded (`XpTally::completion_xp`), so it is
absent on a repeat, as in the original.

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
| 0 | Event complete- PASS | **wired** at the finish (`0x812b3f90`, flags 0,0, joins the chain), **on improvement only** (see "Completion pays on improvement") |
| 1 | Event complete- ELITE | **wired** at the finish (`0x812b3f90`, flags 0,0, joins the chain), **on improvement only**; an elite on a first pass pays 0 and 1 |
| 2 | Finished multiplayer race | n/a: multiplayer (`0x81166252`, `0x812bbac6`, `0x812beb48`); no online 2048 |
| 3 | Won multiplayer race | n/a: multiplayer (`0x81166252`, `0x812bbac6`, `0x812beb48`); no online 2048 |
| 4 | Barrel roll | **read**, unwired: a completed roll (after the roll boost `+0x6104` is stored), human, gate, local slot `+0x7060 == 0`; `0x811c03d6` in `FUN_811bf7aa` (65) |
| 5 | Perfect lap | **wired**: lap completed with no health lost in the lap (ours: no drop in the ship's shield pool since the last line), human, gate; `Pilot_UpdateLapProgress` `0x811c38a0` (78); `XpTally::lap_completed` |
| 6 | Opponent destroyed | **wired** (`0x812ccfe0`, `0x8123b5ea`, `0x811c4a06`: human killer, `FUN_8100d2a6`); fed from `Standing::kills` |
| 7 | Speed pad hit | **read**, unwired: pad hit, human, gate, and `pilot+0x6128` byte 0 (that flag still unread); `0x811cd052` (60) |
| 8 | Weapon pad hit | **wired**: the pad hands the human a weapon, gate; `0x811c9ee8` (75); `Race::test_weapon_pads`'s grant |
| 9 | Zone | **wired** (`0x812c4052`, `0x81186862`): a zone step with a wall contact |
| 10 | Perfect Zone | **wired** (same sites): a clean zone step; our `zone_dirty` stands for flag `+0x60a4 & 0x1000000`, **chosen** |
| 11 | Boost start | **wired**: the perfect launch, human; no gate; `Race_UpdateLaunchGrade` `0x811760de` (80); ours is the player's launch grade turning Perfect (`XpTally::observe_launch`, `crates/physics/src/launch.rs`), read in the simulation so a build without the view pays it too |
| 12 | Hit with Rocket | **wired**: the human's projectile (`Impact::owner`) or Quake wave (`Wave::owner`) damages another craft (`WeaponHit::landed`), gate; weapon type 0; `XpAward_WeaponHit` `0x811c42d2` (75) |
| 13 | Hit with Missile | **wired**: the human's projectile (`Impact::owner`) or Quake wave (`Wave::owner`) damages another craft (`WeaponHit::landed`), gate; weapon type 1; `XpAward_WeaponHit` `0x811c42d2` (75) |
| 14 | Hit with Plasma | **wired**: the human's projectile (`Impact::owner`) or Quake wave (`Wave::owner`) damages another craft (`WeaponHit::landed`), gate; weapon type 2; `XpAward_WeaponHit` `0x811c42d2` (75) |
| 15 | Hit with Cannon | **wired**: the human's projectile (`Impact::owner`) or Quake wave (`Wave::owner`) damages another craft (`WeaponHit::landed`), gate; weapon type 3; `XpAward_WeaponHit` `0x811c42d2` (75) |
| 16 | Hit with Mine | **wired**: the human's projectile (`Impact::owner`) or Quake wave (`Wave::owner`) damages another craft (`WeaponHit::landed`), gate; weapon type 4; `XpAward_WeaponHit` `0x811c42d2` (75) |
| 17 | Hit with Quake | **wired**: the human's projectile (`Impact::owner`) or Quake wave (`Wave::owner`) damages another craft (`WeaponHit::landed`), gate; weapon type 5; `XpAward_WeaponHit` `0x811c42d2` (75) |
| 18 | Hit with Bomb | **wired**: the human's projectile (`Impact::owner`) or Quake wave (`Wave::owner`) damages another craft (`WeaponHit::landed`), gate; weapon type 6; `XpAward_WeaponHit` `0x811c42d2` (75) |
| 19 | Hit with Leach Beam | **read, unwired**: the Leach Beam drains every tick of the link (`Beam::advance`); whether the original pays per tick or per link is unread. Type 7; `XpAward_WeaponHit` `0x811c42d2` (75) |
| 20 | Hit with Light Barrier | **read, not wired**: Light Barrier (type 10) is not a weapon this engine has (`Weapon::from_type` drops it, `docs/gameplay/weapon-lineage.md`); `XpAward_WeaponHit` `0x811c42d2` (75) |
| 21 | Taken 1st! | **wired**: first time the human's place reaches 1 (best place starts at 99, kept for every place), after crossing the line once (ours: `lap_start_tick`), pilot count > 1, gate; `Race_SortPlaces` (75); `XpTally::observe_taken` |
| 22 | Taken 2nd! | **wired**: first time the human's place reaches 2 (best place starts at 99, kept for every place), after crossing the line once (ours: `lap_start_tick`), pilot count > 2, gate; `Race_SortPlaces` (75); `XpTally::observe_taken` |
| 23 | Taken 3rd! | **wired**: first time the human's place reaches 3 (best place starts at 99, kept for every place), after crossing the line once (ours: `lap_start_tick`), pilot count > 3, gate; `Race_SortPlaces` (75); `XpTally::observe_taken` |
| 24 | Finished 1st! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 25 | Finished 2nd! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 26 | Finished 3rd! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 27 | Finished 4th! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 28 | Finished 5th! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 29 | Finished 6th! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 30 | Finished 7th! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 31 | Finished 8th! | **wired** at the finish (`0x812bbc5c`, `0x8100bfba`); paid once, through the tally |
| 32 | Shielded against Rocket | **wired**: the human's shield absorbs a hit (`WeaponHit::absorbed` with the weapon named), gate; weapon type 0; `Pilot_ApplyPendingHit` `0x811bd214` (72) |
| 33 | Shielded against Missile | **wired**: the human's shield absorbs a hit (`WeaponHit::absorbed` with the weapon named), gate; weapon type 1; `Pilot_ApplyPendingHit` `0x811bd214` (72) |
| 34 | Shielded against Plasma | **wired**: the human's shield absorbs a hit (`WeaponHit::absorbed` with the weapon named), gate; weapon type 2; `Pilot_ApplyPendingHit` `0x811bd214` (72) |
| 35 | Shielded against Cannon | **wired**: the human's shield absorbs a hit (`WeaponHit::absorbed` with the weapon named), gate; weapon type 3; `Pilot_ApplyPendingHit` `0x811bd214` (72) |
| 36 | Shielded against Mine | **wired**: the human's shield absorbs a hit (`WeaponHit::absorbed` with the weapon named), gate; weapon type 4; `Pilot_ApplyPendingHit` `0x811bd214` (72) |
| 37 | Shielded against Quake | **wired** for the projectile paths; **unreachable for Quake**: a wave meeting a running shield asks nothing and reports no absorb (`Wave::apply_hits`), so the weapon is never named; weapon type 5; `Pilot_ApplyPendingHit` `0x811bd214` (72) |
| 38 | Shielded against Bomb | **wired**: the human's shield absorbs a hit (`WeaponHit::absorbed` with the weapon named), gate; weapon type 6; `Pilot_ApplyPendingHit` `0x811bd214` (72) |
| 39 | Shielded against Leach Beam | **read, unwired**: the Leach Beam link reports a drain, not an absorb (`advance_one_leach_beam`); weapon type 7; `Pilot_ApplyPendingHit` `0x811bd214` (72) |
| 40 | Shielded against Light Barrier | **read, not wired**: Light Barrier (type 10), see 20; `Pilot_ApplyPendingHit` `0x811bd214` (72) |
| 41 | Held 1st for 30sec | **wired** (`0x811712aa`): first place held, human, rivals present, not elimination; our place is `oag_race::places` |
| 42 | Held 1st for 20sec | **wired** (`0x811712aa`): first place held, human, rivals present, not elimination; our place is `oag_race::places` |
| 43 | Held 1st for 10sec | **wired** (`0x811712aa`): first place held, human, rivals present, not elimination; our place is `oag_race::places` |
| 44 | Cross Start/Finish Line with full health | **wired**: lap completed with the shield pool at the ship's maximum (`dimensions.shield`); human, gate; `Pilot_UpdateLapProgress` `0x811c39a4` (75); paid whether or not the lap was clean |
| 45 | 5 seconds ahead | **read, unwired**: needs split checkpoints (three a lap, positions unread) and each rival's first crossing; this engine has no split signal; `DAT_8154135c == 0`, no gate; `Race_RecordSplit` (75) |
| 46 | 10 seconds ahead | **read, unwired**: needs split checkpoints (three a lap, positions unread) and each rival's first crossing; this engine has no split signal; `DAT_8154135c == 0`, no gate; `Race_RecordSplit` (75) |
| 47 | 15 seconds ahead | **read, unwired**: needs split checkpoints (three a lap, positions unread) and each rival's first crossing; this engine has no split signal; `DAT_8154135c == 0`, no gate; `Race_RecordSplit` (75) |
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
| 58 | Air Time | **read**, unwired: landing after more than 1.5 s airborne (`ship+0x534`), human; no gate; bare site `0x811d0370` (65) |
| 59 | No wreck for 30 seconds | **never paid**: no call site in v1.04 passes 59 (census of all 82 calls, below) (75) |
| 60 | No wreck for 60 seconds | **never paid**: as 59 (75) |
| 61 | No wreck for 90 seconds | **never paid**: as 59 (75) |
| 62 | No wreck for entire event | **read in part**: race-end results `FUN_8100bfba` `0x8100c4a0`, paid when `(r8+4) == 0`; the field is unread (50) |
| 63 | Shoot/Destroy Mine | **read in part**: a projectile or plasma destroys a mine; 13 sites in five weapon functions (below), per-site tests unread (60) |
| 64 | Shoot/Destroy Bomb | **read in part**: a projectile destroys a bomb; sites below, per-site tests unread (60) |
| 65 | No hits for 10 seconds | **never paid**: as 59 (75) |
| 66 | No hits for 20 seconds | **never paid**: as 59 (75) |
| 67 | No hits for 30 seconds | **never paid**: as 59 (75) |
| 68 | No hits for 40 seconds | **never paid**: as 59 (75) |
| 69 | No hits for 50 seconds | **never paid**: as 59 (75) |
| 70 | No hits for 60 seconds | **never paid**: as 59 (75) |
| 71 | Successful Turbo | **read**, unwired: turbo ended with time since the last hit (`+0x60f8`) above the turbo's duration (weapon table `+0xfc`), human; `Ship_UpdateStartBoost` `0x811bf29e` (60) |
| 72 | Hit with cannon full clip | **read**, unwired: hits from one cannon clip reach the clip size (weapon table `+0xa8`); `0x811e0e0a` (60) |
| 73 | Hit with all 3 rockets | **read**, unwired: rocket hits reach 3 and the ship's rocket count (`+0x4a0`); `0x81207164` (60) |
| 74 | Evade Mine | **read in part**: a mine's near-pass latch `+0x4c` set inside a distance window, paid on leaving it; `0x811f480a` (55) |
| 75 | Evade Bomb | unwired: head unread, `0x811dbde8` in `FUN_811dbb5a` (45) |
| 76 | Evade Missile | **read in part**: missile state `+0x64 == 1` and a distance test; `0x811f9352` in `FUN_811f89fa` (50) |
| 77 | Evade Zombie | **never paid**: as 59 (75) |

Awarded in ours through `XpTally`, fed from `Race::tick`'s own edges and never part of the state hash:
`oag_raceplay::tests::xp_edges` pins the lap, launch, pad, place, rocket, shield and Quake feeds, and
`an_autopiloted_event_pays_the_lap_and_place_awards_the_table_authors` runs a disc event. A hit is
attributed to the human through `Impact::owner` (the Eliminator's `Race::impact_reaching`, shared)
or `Wave::owner`; Missile, Plasma, Cannon, Mine and Bomb go through the same lookup and are not
tested one by one. Never paid by the original and left unpaid here: 59-61, 65-70, 77.

Not wired by anything here, in the original: the rank bar and the `RaceXP` popups.

**Omega check.** Omega ships a byte-identical `XPAwards.xml` and the same loader: **ported** (an
Omega `--event` arms the tally from Omega's table, `omega_event_xp_ground_truth`).

## Cross-check

- **Omega**: XP - checked, applies, not wired for events (`oag_tables::xp` parses Omega's `XPAwards.xml` and `XPRanks.xml`, ground truth in `crates/omega/tests/omega_title_ground_truth.rs`; the profile total is shared, Omega's event launch does not bank yet); ranks - checked, differs. Omega's `eboot.bin` carries the same
  loader (`FrontendRoot_Construct` references `Data\XML\XPAwards.xml` at `0x0182eb7a`) and
  its `data00.psarc` ships a **byte-identical** `Data/xml/XPAwards.xml` (9177 bytes) and a
  different `XPRanks.xml` (same 50 ranks, thresholds exactly **half** of 2048's: 15000,
  30000, 50000, ...). See `ps4-omega-eu/xp-and-loyalty.md`.
- **HD**: checked, differs (no XP; loyalty only). Wiring left open in a handover thread.

## Wired (2026-11-12)

`oag_tables::xp` reads both files; `Store::bank_event_xp` banks a finished 2048 event's
`Event complete` awards on improvement ("Completion pays on improvement") plus `Finished Nth`.
**Chosen, not measured**: `Finished Nth` on every finish; column by event kind. The profile total is one number shared with
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
v1.04 patch's `data1.psarc` (`data/xml/SP.xml`, 1,316,362 bytes, the copy v1.04 loads; the base
`data.psarc` copy is 1,316,363 bytes, differs at offset 1,291,041 and gives the same census below). Builds on
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
   of the TouchCampaign vtable at `0x8150e958`; it calls the base widget's `0x81064900` first) calls
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

**Why the rank-50 test promoted nothing: unknown (hypothesis, 60).** If that test wrote the XP total
into a running game whose map was already built, never left and re-entered the map page and did not
finish a race, neither trigger ran after the write and the law needs no extra gate. Against it: the
builder only creates a button where `event+0x1ec` is null, so it expects to run more than once, and
the earlier note does not say whether the page was re-entered. Not yet confirmed live: the check
is to write `profile+0x3257c` before the first map entry of a boot, then read a side challenge's
`+0x2d8` on the map (expect 1); a 0 there would point at the event list `Campaign_UnlockEvents`
walks (`0x81965b28`, vector at `+0x24`) not holding the side challenges, since the button builder
walks a different list (`0x81965b60`, vector at `+4`).

**Ours.** `Store::rank_reveals` is asked by `boot::reveals2048::refresh` every time a map is built
from the live profile: at boot, and on every opening of the hosted map (`MapSeed::open`, Omega's
chooser). So a rank reached in a race reveals its challenges the next time a map is built, with no
restart (`a_rank_reached_by_a_finish_reveals_its_events_on_the_next_map_build`, which banks through
`Store::bank_event_xp` and refreshes the same map). **Vita 2048 itself cannot reach a map after a
race**: `Session::leave_finished_race` lands on this build's own menus and `Stage::Frontend` has no
rebuild path (`MapSeed::of` requires `front_end.menu`, 2048 has none), so there a revealed event shows
at the next launch until the way back to the map exists.

**Omega: checked, applies, not wired.** Omega's `eboot.bin` has the same function at `0x015a1200`
(found through `"2049 - Event 1"`): the three heads by name to 2, then the loop with
`IsUnlockable` inlined (force `+0x3cc`; XP `DAT_02039628+0x4dc` against the rank table
`DAT_0203a570[rank - 1]`, rank `+0x3c8`; required event `+0x3b8` at progress `+0x3c0 > 2`), the chain
flag at `+0x3cd`, and 1 versus 2 by vtable `+0xe8` or `+0x22c & 8`. Its two callers are the same
pair: `FUN_0159dea0` and the campaign map's button builder `0x014e32c0`. Only the offsets differ.
Omega's map does not launch events yet, so no Omega reveal is wired.

## The other 52 award triggers (2026-11-13, lane `w2048-xp-unlock-re`)

Ghidra `/2048/eboot-vita-2048-eu-v104.elf`, decompile and disassembly only, no live run: every
score is at or under the rubric's 84 ceiling. The table in "In-race awards" carries each award's
result; this section is the evidence. "Gate" is `FUN_8100d2a6` (that section), "human" is
`pilot+0x58dc == 0`. "Ready" means the trigger is read well enough to wire.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x811c4a06` | `Pilot_TakeDamage` | 75 |
| `0x811c42d2` | `XpAward_WeaponHit` | 75 |
| `0x811bd214` | `Pilot_ApplyPendingHit` | 70 |
| `0x811c2ad6` | `Pilot_UpdateLapProgress` | 72 |
| `0x811c2800` | `Race_RecordSplit` | 72 |
| `0x812bbeb2` | `Race_SortPlaces` | 72 |
| `0x81175fc8` | `Race_UpdateLaunchGrade` | 80 |
| `0x811bf07e` | `Ship_UpdateStartBoost` | 78 |

**The census (negative for 59-61, 65-70 and 77, confidence 75).** `Profile_AwardXP` has 82 direct
calls and no other reference. Each call's index register was read off the instruction before it
(a scan of 34 sites, with six of them re-read by hand, plus the sites the earlier lane read):
every one is a literal except three bounded expressions: `Finished Nth` is `place + 0x17` (24 to
31; `0x8100c33c`, `0x812bbdc2`) and `Hit Zone N` is `0x2f + zone/10` (48 to 57; `0x812c4310`, capped
by `cmp r0, #0x64; bgt` at `0x812c42c0` and `zone % 10 == 0` at `0x812c42de`).
No call can pass 59, 60, 61, 65 to 70 or 77. **"No wreck for 30/60/90 seconds", "No hits for 10 to
60 seconds" and "Evade Zombie" are authored in `XPAwards.xml` and never paid by this executable**;
an indirect call through a pointer is the only way left open, and none was found. Omega agrees for
the two indices checked: no caller of its `AwardXP` (`0x01667e30`, 51 calls) loads `ESI` with 0x3b
or 0x41.

**Damage and the perfect lap (78).** `Pilot_TakeDamage` (`0x811c4a06`, pilot, kind, weapon type,
amount in `s0`; the decompiler gives up on it, read as disassembly) sets `+0x60a4 |= 0x20 | 0x400000`
on any damage above 0 (`0x811c4ed2`), plus `0x1000000` when `GameMode_IsZoneFamily` (`0x810018d4`, named at 65 elsewhere) holds, and
zeroes the time since the last hit `+0x60f8` (`0x811c4cca`). `Pilot_UpdateLapProgress`
(`0x811c2ad6`, per pilot per frame) on a completed lap (the lap counter `+0x6e50` reaching the
expected `+0x6e54`; the first crossing at the start only arms it) pays, for a human through the gate:
`Perfect lap` when `0x20` is clear (`0x811c38a0`; it also counts a profile stat and fires trophy 0x36
at ten), then `Cross Start/Finish Line with full health` when the ship's maximum health (ship table
`+0x80`, class slot `+0x98`) is at or below the health `+0x5634` (`0x811c39a4`), and then clears
`0x20`. So the earlier lane's open `+0x60a4 & 0x20` is "damaged this lap", and `0x1000000` is the same
damage edge in the zone family (as sure as that function's name, 65), the flag our `zone_dirty`
stands for.

**Weapon hits (75).** `Pilot_TakeDamage` calls `XpAward_WeaponHit(victim, attacker index, kind,
weapon type)` at `0x811c4cc0`, reached by a short path for non-human pilots (`0x811c4ca6`) and at the
end of the main damage path (`0x811c540c`, `0x811c54aa` jump back to `0x811c4cac`), so any victim;
the attacker index is the hit record's `+0x12c`. It pays when `kind == 2`, the attacker is human, the
attacker is not the victim (`+0x58d4`) and the gate passes. Weapon type to award: 0 Rocket (12), 1
Missile, 2 Plasma, 3 Cannon, 4 Mine, 5 Quake, 6 Bomb, 7 Leach Beam (19), 10 Light Barrier (20); 8
and 9 pay nothing. This is a third enum, neither `WeaponType`'s bits
([weapon-type-bits.md](weapon-type-bits.md)) nor the held-weapon id: read off the case order against
the award names, and Leach Beam = 7 is checked a second way (only type 7 copies the record's `+0x124`
to the leech target, in `Pilot_ApplyPendingHit`).

**Shield (72).** `Pilot_ApplyPendingHit` (`0x811bd214`, per frame) resolves the pending hit record
`+0x557c` (amount `+0xf4`, type `+0x128`): unshielded it goes through the pilot's damage call (vtable
`+0x30`); with record `+0x1f4 & 0x10` and the shield up (`pilot[0x196c]+0xf4`) it is absorbed, and a
human through the gate gets `0x20 + type` (same type order as the hits).

**Taken Nth (75).** `Race_SortPlaces` (`0x812bbeb2`, race phase above 1) bubble-sorts the pilots;
then for each place `i` (1-based) a human pilot that has crossed the line once (`+0x6640`) and whose
best place `+0x6f3c` is worse than `i` takes `i` as its best and, through the gate, is paid `Taken
1st` (`i == 1`, more than one pilot), `2nd` (more than two) or `3rd` (more than three). The best place
is set to 99 at pilot reset (`0x811b8c28`, `0x811baa0e`), so each pays at most once per race and the
first evaluation after the start pays the place held then.

**N seconds ahead (75).** `Race_RecordSplit(pilot, split)` keeps one record per lap and split (three
splits a lap; the line is split 0) in the race object (`+0xfb7c`). Only while
`DAT_8154135c == 0` (unread; likely single player). The human crossing first stores the race clock
as an integer (`*piVar5 = (int)clock`, truncated); when a rival is the first
non-human to cross that split afterwards, the gap is the clock minus the human's time: 15 s or more
pays 47, 10 to 15 s pays 46, 5 to 10 s pays 45. There is no gate here.

**Boost start (80).** `Race_UpdateLaunchGrade` (`0x81175fc8`) is Pulse's launch grader
([launch-boost.md](../../../physics/launch-boost.md)) with the same four windows, read from
`windowStart`, `windowEnd`, `stallEnd` and `overallDuration` by `Handling_ReadGlobalSettings`
(`0x811ad82c`, keys at `0x814901e4`): a human whose first-thrust edge (`ship+0x595`; it has no literal-offset writer in 2048, so
"first-thrust edge" is inherited from Pulse's `+0x2d5`) lands in `[windowStart, windowEnd)` gets grade 2 and award 11 (`0x811760de`). No
gate. `Ship_UpdateStartBoost` (`0x811bf07e`) reads the grade `+0x58e0` into the multiplier
`+0x548` exactly as Pulse's does, and also holds the turbo award (71).

**Others.** Barrel roll, speed pad, weapon pad, air time, cannon clip, three rockets: the address
and condition are in the table. `+0x6128` (speed pad) is reset at pilot init (`0x811ba22a`) and
copied to the HUD (`0x811b21b4`); its writers `0x812cbbe8`, `0x812cbd20` (`FUN_812cb4da`) and
`0x811920b6` are the next read. Mine and bomb destroyed: 63 at `0x811dfd1a`, `0x811dfdd0`,
`0x811e02b4`, `0x811e0516`, `0x811fe416`, `0x811fe4d4`, `0x81206388`, `0x812063d8`, `0x811f9180`,
`0x811f9196`; 64 at `0x811e01b0`, `0x811e038a`, `0x811e0412`, `0x811e05ec`, `0x811fe7c6`,
`0x811fe894`, `0x812066e6`, `0x812067c6`, `0x811f8d20`, `0x811f8d36`, `0x811f8e36`, `0x811f8e4c`.
Recipe for each "read in part" row: decompile the named function and follow the branch into the
call site back to its first compare.

**Ready to wire, in order of reach**: perfect lap (5), full health at the line (44), Taken 1st to
3rd (21-23), N seconds ahead (45-47), boost start (11), weapon pad (8), weapon hits (12-20), shield
(32-40). Ours already grades the launch (`crates/physics/src/launch.rs`); which of our signals stands for
each other trigger is the wiring lane's check.

**Omega: checked, applies, ported through the shared tally.** Omega's `AwardXP` (`0x01667e30`, found
through `AWARD_GROUP_CHAIN`) has 51 direct calls against 2048's 82. Spot checks use the same indices:
`WeaponPads_TestCraft` pays 8 (`0x01305932`), `PlasmaManager_Update` pays 63 (`0x0135fae5`). An Omega
`--event` race arms the same `XpTally` from Omega's byte-identical `XPAwards.xml` and the feeds above
run in it unchanged (they hang off the shared simulation, not a title). Only 8 was checked against
Omega's own call sites; the other wired awards take 2048's reading by lineage (2048 <-> Omega), and
Omega's dispatch between its loyalty block and its XP block is still untraced
(`ps4-omega-eu/xp-and-loyalty.md`), so paying XP there is **chosen, not measured**.
