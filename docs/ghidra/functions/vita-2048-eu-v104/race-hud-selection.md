# Which HUD skin a real race constructs

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. **The names here are applied**, from [names.tsv](names.tsv).
Found while composing `oag_2048::hud`'s 26 shipped HUD layouts and asking which
one a race actually reads - `crates/2048/src/hud.rs` carried a bare guess
("the root set is the default ... for the same reason it is [HD]") that this
settles with code evidence instead.

## `SpArcadeRaceManager_Construct` - `0x8117af40`

**Confidence: 90**

A race-manager constructor: tags its allocations
`"Backend/General/SPArcade_RaceManager.cpp"` (`param_1[0xb]`, the same
tagged-allocation idiom `RcsModel_Load` and `GameRoot_Construct` both rest
their own confidence on) and, on the single-screen path
(`DAT_8153fd28 < 2`), constructs its HUD from the literal
`Data\XML\2048_hud\Arcade_HUD.xml` - not the bare `Data\XML\Arcade_HUD.xml`
[`oag_hd::hud`]-style root. The two split-screen branches load
`Data\XML\SplitScreen_hud\SplitScreen_HUD_{0,1}.xml` and the vertical variant,
neither of which [`oag_2048::hud`] models - out of scope, [`oag_hd::hud`]
excludes its own split-screen family on the same terms.

"SP" is Single Player: the constructed name distinguishes this from whatever
multiplayer arcade race construction is (unread).

## `DemoRaceManager_Construct` - `0x8114a090`

**Confidence: 85**

Tags its allocations `"Backend/General/Demo_RaceManager.cpp"` and constructs
its HUD from the **bare** `Data\XML\Arcade_HUD.xml` - the root, HD-shaped set.
This is the attract-mode/demo race, not a mode a player reaches by playing the
game normally.

## `FUN_812bf3f0` (Elimination) - `0x812bf3f0`, `FUN_812c1722`
(SpeedLap/TimeTrial) - `0x812c1722`, `FUN_812c4e2e` (Zone) - `0x812c4e2e`

**Not renamed - confidence 70, below the renaming floor for a class name this
project has not confirmed independently for each.** Same shape as
`SpArcadeRaceManager_Construct` without an equally direct `__FILE__`-style
string in the snippet decompiled, but each references only `2048_hud\`
paths, never a bare or `wo3_hud`/`2097_hud` one:

- `FUN_812bf3f0` builds `Data\XML\2048_hud\Elimination_HUD.xml`.
- `FUN_812c1722` branches on a param (`*(int*)(param_1+0x18c) > 0`) between
  `Data\XML\2048_hud\SpeedLap_HUD.xml` and
  `Data\XML\2048_hud\SpeedLap_TimeTrial_HUD.xml` - so **speed lap and time
  trial are two distinct files on this title**, not one file shared the way
  both PSP discs share time trial's for speed lap. `oag_title::HudLayouts`
  models the two as separate fields already; this is the evidence that they
  should carry different values here, where HD's own table gives them the
  same file only because that happens to be what its manifest ships.
- `FUN_812c4e2e` builds `Data\XML\2048_hud\Zone_HUD.xml`.

## What this settles: `2048_hud` is the played skin, not `wo3_hud`/`2097_hud`

Every real race-construction path found reaches into `2048_hud\`. Searching
`eboot.elf`'s defined-string table for `wo3_hud` and `2097_hud` (both
substrings) returns **zero** matches - those two skins, which the archive
ships in full (`Data\XML\wo3_hud\*`, `Data\XML\2097_hud\*`, five files each,
composing cleanly with the same widget dialect), are never referenced by any
string this build's analysis has found. They are present on disc and
unreachable from any code path found so far - not proven dead code, since
`search_strings` only walks Ghidra's *auto-detected* string table and the
`SpeedLap_HUD.xml` / `MPTag_HUD.xml` misses below show that table is
incomplete, but no positive evidence for either skin exists either.

**Caveat found while writing this up**: `FUN_812c1722`'s own
`Data\XML\2048_hud\SpeedLap_HUD.xml` literal, visible directly in its
decompilation, does not appear in `search_strings("2048_hud.SpeedLap_HUD")`
- so a `search_strings` miss is not evidence a path is unreferenced, only that
it was not independently corroborated. The two constructors renamed above
each combine a `search_strings` hit *and* a decompiled read, which is why they
cleared the renaming floor and the three `FUN_*` ones did not.

**Consequence for `oag_2048::hud::LAYOUTS`**: it should read the `2048_hud\`
set (`Arcade_HUD.xml`, `Elimination_HUD.xml`, `SpeedLap_HUD.xml`,
`SpeedLap_TimeTrial_HUD.xml`, `Zone_HUD.xml`), not the bare root set the
previous reading guessed at by analogy with HD's own skin shape. See
`docs/formats/2048-hud.md`.

## The `XP` and `OBJECTIVE` lines are bound to the current `SP.xml` event (2026-10-26, `2048-dlc-hud`)

**Confidence: 90** for the binding, **85** for the `MP_*` census, **75** for "the HD and Fury Campaign
launch never sets the event". Static decompile of `Hud_BindWidgets` (`0x81192cd0`) and `Hud_UpdateWidgets`
(`0x81195cdc`), plus two Vita3K frames.

`Hud_BindWidgets` writes `hud+4 = GameMode_Current()` (`0x812b5890`, which returns `g_CurrentGameMode`,
`0x81965b58` - read live as `"2048 - Event 4"` in [ai-controls.md](ai-controls.md)). The one writer of that
global is `GameMode_SetCurrent` (`0x812b5af2`); its callers are the campaign's event card
(`CampaignEventCard_HandleInput`, `FUN_810924e8` and the five `FUN_8124xxxx` map handlers), the attract mode
(`FUN_810763d8`, an event named `GameModeAttractMode`), and two clears (`FUN_81123ec6`, an online-entry handler, and
`FUN_81124414`, reached from the two front-end shell exits `FUN_810909d2` and `FUN_81092602`). **Neither `HDCampaignNode_Select` (`0x810f7060`) nor
`FuryCampaignNode_Select` (`0x810f3fa4`), the handlers the `FE3DCanvas` hotspots of the two DLC campaigns
forward a tap to, calls it.** They read the tapped node's `Class`, `Track`, `Weapons`, `Opponents`, `Damage`,
`SkillLevel` and `Tournament` fields and write `g_HdFuryCampaignNode` (`0x8153fcd4`) instead, which
`HudScreen_Construct` (`0x8114f498`) reads to pick the HD medal models (`Data\FE\Trophies\hd_gold.vex`). So a
DLC campaign race binds no event.

| Widget | Bound in `Hud_BindWidgets` | Updated in `Hud_UpdateWidgets` |
| --- | --- | --- |
| `RaceXPTxt` (`HUD_award_xp*.xml`, caption `FE_XP`) | visible iff `hud+4 != 0` | - |
| `RaceXP`, `AwardXP` | written by `Hud_UpdateXpLines` (`0x8119451c`) - `"%d"` of the event's `+0x1e8` tally and `"%s +%d"` for an award | `Hud_UpdateXpLines` runs only when `hud+4 != 0` |
| `ObjectiveParent` (`ObjectiveTxt`, `ObjectivePoint0-2`, `ObjectiveTick0-2`, `ObjectiveName0-2`) | visible iff `hud+4 != 0` **and** `(event+0x1a0 & 4) != 0` **and** `event+0x1b0` (`m_PassObjective`) is non-null; `hud+0x6fa = (m_options & 4) == 0` is stored beside it | the `ObjectivePoint` rows are shown or hidden only when `(m_options & 4) != 0` |

**`m_options & 4` is the multiplayer flag.** `CampaignEventCard_BuildPageList` calls an event with that bit clear a
single-player event ([campaign-event-card.md](campaign-event-card.md)), and across `SP.xml`'s 141 events
(`m_options`, `GameModeOptions`) the values are: empty 87, `1` 13, `4` 21, `8` 5, `16` 13, `17` 2. The 21 with `4`
are exactly `MPElimination` and the `MP_<circuit>_Race_flash` / `_Eliminator_flash` events; none of the 120 others
has it. So **`OBJECTIVE` is not shown on any single-player race**, including the campaign's own: `2048 - Event 1`
authors a pass objective and an empty `m_options`, and the original's frame of it
(`data/reference/compare-page/2048-vita/empire-climb/theirs.png`) draws `XP 0` and no `OBJECTIVE`. The DLC pair's
original frame (`dlc-vineta-k/theirs.png`, the HD Campaign's Uplift Race, a cold boot that never opened an event
card) draws neither. Both frames, and the three bind rules, agree.

**Not excluded:** an event the player opened earlier in the same session and then left through a path other than
`FUN_81124414` would still be bound when a DLC node is raced. Nothing read shows such a path, and the DLC frame
is a cold boot, so the claim "the HD and Fury Campaigns draw neither" is 75, not 90.

### What ours does

`oag_raceplay::load_event` sets `Setup::campaign_event`; it reaches `oag_hud::Readout::campaign_event`, and
`oag_hud::draw::text_for` draws `RaceXPTxt` (and `RaceXP`, `AwardXP`) only with it and never draws `ObjectiveTxt`.
Tests: `crates/hud/src/campaign_event_tests.rs` (fixture), `crates/raceplay/src/tests/campaign_event.rs`
(`Setup` to readout), `crates/raceplay/tests/vita_2048_hud_event_ground_truth.rs` (the disc's own
`Arcade_HUD.xml`, an event against the DLC's Vineta K). The XP **value** is not drawn: ours keeps no per-race XP
tally (`2048-and-omega-xp-is-read-not-kept`), so a campaign event shows `XP` alone where the original shows `XP 0`.

### Omega: checked, differs

Omega's `eboot.bin` carries the same widget strings (`RaceXPTxt` at `0x018137ab`, `ObjectiveParent` at
`0x018137dc`) and the `2048_hud\` HUD set (`HUD_award_xp.xml` byte for byte 2048's, `HUD_objectives.xml` the same
with the `.2x` atlas spelling), **but** the bind function `FUN_012b8e90` clears `ObjectiveParent`'s visible bit
outright (`0x012bb8a7`, no event test) and no direct code reference to `RaceXPTxt` was found. The layouts a race reads
(`oag_omega::TITLE.hud` and both retro skins) are HD's root set, which author neither `RaceXPTxt` nor
`ObjectiveTxt`; the `2048_hud\` set is reached through virtual methods only (see `ps4-omega-eu/race-hud-selection.md`).
Nothing to hide on Omega, nothing wired:
`crates/raceplay/tests/omega_hud_ground_truth.rs::omega_plays_no_xp_or_objective_line` pins it and an Omega
headless still shows the HD hexagon HUD with neither line.

## The race managers, and which layout each loads (2026-10-26, `2048-dlc-hud`)

**Confidence: 90** for each constructor's tag and literal (the same `__FILE__`-style tag the arcade manager above
rests on), **80** for the reading below. `RaceManager_ConstructByMode` (`0x8115f3de`) switches on the race-mode word
`g_RaceMode` (`0x8153fd24`). The ids below `0x17` are the HD-lineage named modes; 2048's own campaign and
multiplayer events carry a CRC of their `m_name` instead ([countdown-voice.md](countdown-voice.md),
[particle-paths.md](particle-paths.md)), and a campaign Time Trial read `0x026886dc` live, a value no case
below matches.

| Mode | Constructor (`.cpp` tag) | HUD literal in the constructor |
| --- | --- | --- |
| 2 | `DemoRaceManager_Construct` | `Data\XML\Arcade_HUD.xml` (the bare root) |
| 3, 9, `0xc` | `SpArcadeRaceManager_Construct` | `2048_hud\Arcade_HUD.xml` |
| 4 | `SPTournamentRaceManager_Construct` (`0x811853ec`) | `2048_hud\Arcade_HUD_HD.xml` |
| 5, 10 | `SPTimeTrialRaceManager_Construct` (`0x81183e50`) | `2048_hud\TimeTrial_HUD_HD.xml` |
| 6 | `SPZoneRaceManager_Construct` (`0x81186584`) | `2048_hud\Zone_HUD_HD.xml` |
| 7 | `SPFreePlayRaceManager_Construct` (`0x81180970`) | `Data\XML\TimeTrial_HUD.xml` (the bare root) |
| 8 | `SPEliminationRaceManager_Construct` (`0x8117fb48`) | `2048_hud\Elimination_HUD_HD.xml` |
| `0xd` | `SPNitroRaceManager_Construct` (`0x81182148`) | `2048_hud\ZoneBattle_HUD_HD.xml` |
| `0xe` | `SPDetonatorRaceManager_Construct` (`0x8117c5ce`) | none in the constructor |
| `0x10`, `0x12` | `MPArcadeRaceManager_Construct` (`0x81160574`) | `MPTag_HUD.xml` |
| `0x11` | `MPTournamentRaceManager_Construct` (`0x8116471c`) | none in the constructor |
| `0x13` | `MPTimeTrialRaceManager_Construct` (`0x8116385c`) | `Data\XML\Arcade_HUD.xml` and `MPTag_HUD.xml` |
| `0x14` | `MPEliminationRaceManager_Construct` (`0x81161170`) | `2048_hud\Elimination_HUD_HD.xml` and `MPTag_HUD.xml` |
| `0x15` | `MPNitroRaceManager_Construct` (`0x81161c74`) | `2048_hud\ZoneBattle_HUD_HD.xml` and `MPTag_HUD.xml` |
| 1 | `AIBatchRaceManager_Construct` (`0x81148220`) | `Data\XML\TimeTrial_HUD.xml` |

(`zone-audio.md` reads `0xd` and `0x15` as the Zone Battle pair, the SP and the MP manager. `0x10` and `0x12` share
a case. The table is read off the switch and the constructors, not played.)

**The `_HD` files are the HD Campaign's layouts for every mode but Race.** `TimeTrial_HUD_HD.xml`,
`SpeedLap_HUD_HD.xml`, `Zone_HUD_HD.xml`, `Elimination_HUD_HD.xml`, `ZoneBattle_HUD_HD.xml` and
`Detonator_HUD_HD.xml` ship in the patch's `data1.psarc` and `data2.psarc`; only `Arcade_HUD_HD.xml` is in the
base package. `Arcade_HUD_HD.xml` differs from `Arcade_HUD.xml` by one include (`HUD_positions_HD.xml`, which adds
`MPWeaponIcon0-7` beside the positions list). **`Zone_HUD_HD.xml` matches the HD-campaign Vineta K Zone frames
`67-zone-grid.png` and `68-zone-5.png` that [2048-hud.md](../../../formats/2048-hud.md) called "positioned by a
file this composition does not reach"**: its `ScoreFrame` sits at `(480, 20)` with `IG_HUD_SCORE` over `Score`
(the `SCORE` / `1780` top centre), and its `SpeedClass` group at `(137, 451)` authors `SpeedClass` (`VENOM`),
`Zone Txt` (`ZONE`) and `ZonePlus0` (the `5`) over the `ZoneCounterBG` arc at `(127, 441)` (the bottom-left text).
The base `Zone_HUD.xml` authors `ZoneNumber` top left, which no frame shows. A native campaign Time Trial frame
(`43-tt-start-14.png`) matches `SpeedLap_TimeTrial_HUD.xml` (`TARGET`, `XP`, no shield) rather than
`TimeTrial_HUD_HD.xml` (which loads `HUD_damage_indicator.xml`, `HUD_pickups.xml` and no `XP`), so the native
events keep the base `2048_hud\` files and the HD Campaign's TT, Speed Lap, Zone and Elimination events read the
`_HD` ones. **Checked, applies, not wired** - ours has no HD-campaign launch yet (`--track` on a DLC circuit is the
only way in), so nothing selects them.

### Names

| Address | Name | Conf |
| --- | --- | ---: |
| `0x8114f498` | `HudScreen_Construct` | 85 |
| `0x81192940` | `Hud_Construct` | 75 |
| `0x8119451c` | `Hud_UpdateXpLines` | 78 |
| `0x812b5af2` | `GameMode_SetCurrent` | 85 |
| `0x810f7060` | `HDCampaignNode_Select` | 75 |
| `0x810f3fa4` | `FuryCampaignNode_Select` | 75 |
| `0x8153fcd4` | `g_HdFuryCampaignNode` (data) | 75 |
| `0x8153fd24` | `g_RaceMode` (data) | 80 |
| `0x811853ec` | `SPTournamentRaceManager_Construct` | 90 |
| `0x81183e50` | `SPTimeTrialRaceManager_Construct` | 90 |
| `0x81186584` | `SPZoneRaceManager_Construct` | 90 |
| `0x81180970` | `SPFreePlayRaceManager_Construct` | 90 |
| `0x8117fb48` | `SPEliminationRaceManager_Construct` | 90 |
| `0x81182148` | `SPNitroRaceManager_Construct` | 90 |
| `0x8117c5ce` | `SPDetonatorRaceManager_Construct` | 90 |
| `0x81160574` | `MPArcadeRaceManager_Construct` | 90 |
| `0x8116471c` | `MPTournamentRaceManager_Construct` | 90 |
| `0x81161170` | `MPEliminationRaceManager_Construct` | 90 |
| `0x81161c74` | `MPNitroRaceManager_Construct` | 90 |
| `0x81148220` | `AIBatchRaceManager_Construct` | 90 |

## Next step: `FUN_81158d88` - `0x81158d88`

**Not renamed - unread, not just unnamed.** The single xref to every one of
the six `MissileSightBG`/`Outer`/`Inner`/`Middle`/`LockedOnLines`/
`LockedOnMiddle` strings (`search_strings("MissileSight")`, all six, one
address apiece) all land in this one function. It decompiles to 66 KB - by
far the largest function this thread has opened - and a keyword sweep of the
full decompile for `Always`/`always`/`Visible`/`visible`/`Show`/`Hide` finds
**none**, which reads as a widget name-to-slot registration table rather than
a per-frame visibility state machine.

This is the strongest lead anyone has found for `oag_2048::hud::ALWAYS_ON`,
which is otherwise empty and unread - see `docs/formats/2048-hud.md#what-is-not-done`.
It is not itself the answer: a registration table says a widget *exists*, not
when it draws. What it is worth next: finding what calls into or reads
whatever structure this function builds, on the theory that the actual
per-frame show/hide logic is a caller or a sibling this function's own
callers point at - `get_function_callers`/`get_function_callees` on this
address, not another read of this function itself.

[`oag_hd::hud`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/hd/src/hud.rs
[`oag_2048::hud`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/2048/src/hud.rs
