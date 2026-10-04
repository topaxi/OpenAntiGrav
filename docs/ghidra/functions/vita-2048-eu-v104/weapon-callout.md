# The event card's weapon callout, glyph row, trophy art and forced-craft quads

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. Read 2026-10-05 for the card pages
[campaign-event-card.md](campaign-event-card.md) left open. Every name here is
applied from [names.tsv](names.tsv). Implemented by
`oag_2048::campaign::callout`, `oag_2048::campaign::trophy` and
`oag_ui::frontend::event_card`.

## `CampaignEventCard_DrawWeaponCallout` - `0x810626ce`

**Confidence: 75.** `float (x, y, w, h, gap, colour, pads, weapon_bits,
is_elimination, weapons_disabled, draw)`: with `draw == 0` it only measures
(returns the row's width, `0` when nothing would be drawn), with `draw != 0` it
draws the icons and the caption. Callers: `CampaignEventCard_BuildPageList`
(decides whether the rules page has a callout item), `FUN_810535fe` (rules
page, measure at 44, measure at 60, draw at 63), `CampaignEventCard_GlyphRowWidth`
and `CampaignEventCard_DrawObjectivePage` (measure and draw at 44, no caption)
and the pre-race callout `FUN_810ec5d0`.

Arguments at the card's call sites: `pads` is `event+0x1f0`
(`m_activeWeaponPads`), `weapon_bits` is `event+0x198` (`m_weaponSet`),
`is_elimination` is `event+0x190 == 1`, `weapons_disabled` is vtable slot
`+0x64`. The callout is only asked for when `event+0x190` is neither `0` (Zone)
nor `3` (Speed Lap).

**Slot `+0x64`** is `ldrb r0,[r0,#0x328]; bx lr` (`0x813eb960`) in the vtable at
`0x8151611c` and `movs r0,#0; bx lr` in every other (`0x813ea234`,
`0x813ebaa4`, both disassembled). `0x328` is `m_bDisableWeapons`, registered at
that offset by two `RegisterFields` (`FUN_812bc25e`, `FUN_812bdc5a`). The
vtable at `0x8151611c` is `FUN_812bb914`'s class, whose constructor writes
`+0x190 = 2` and `+0x328 = 0`; `SP.xml` authors `m_bDisableWeapons` on all 52
`typedef -1353052320` events (2 of them `true`), so that class is
`GameMode_ArcadeRace` (confidence 75: the typedef is matched by the field, not
by a hash). Elimination, which also authors the field, never reads it here.

**Defaults for an unauthored field**, from the constructors: `m_weaponSet` is
`0x7ff` (`FUN_812afd9e`'s loop sets bits `0..=10`, skipping `11` and `12`) and
`m_activeWeaponPads` is `3` in both `FUN_812bb914` and `FUN_812be628`
(`GameMode_EliminatorRace`). `SP.xml` authors the set on 11 of 52
`GameMode_ArcadeRace` events and 25 of 26 eliminations, and the pads on 18 and 25.

### The decision

With `A` = pads `& 5 != 0`, `B` = pads `& 10 != 0`, `off` = the set has Rocket,
Missile, Quake, Cannon and Plasma (bits `0 1 2 5 7`) and `def` = the set has
Turbo, Shield, Autopilot, Bomb, Mine and Leach Beam (bits `3 4 6 8 9 10`; for an
elimination only bits `8 9 10`):

| Condition | Draws |
| --- | --- |
| `weapons_disabled`, or neither `A` nor `B` | `weapons/weapons_off`, `Event_Variable_Weapons_Off_0` |
| `!A` and `def` | `weapons/offensive_off`, `Event_Variable_Offensive_1` |
| `B` and `off` and `def` | nothing (every weapon is on offer) |
| `!B` and `off` | `weapons/defensive_off`, `Event_Variable_Defensive_1` |
| anything else | one icon per set bit `0..=10`, names joined with `" + "` (`0x8142a998`); `weapons_off` if the set names none |

Icons run in `WeaponType` bit order - `rocket`, `missile`, `quake`, `speedup`,
`shield`, `cannon`, `autopilot`, `plasma`, `bomb`, `mine`, `leach` under
`NewImages/weapons/` (`FrontEnd_LoadCardTextures`, handles `0x816c8858..`) - and
the names `FE_ROCKETS`, `FE_MISSILE`, `FE_QUAKE`, `FE_TURBO`, `FE_SHIELD`,
`FE_CANNON`, `FE_AUTOPILOT`, `FE_PLASMA`, `FE_BOMB`, `FE_MINES`, `FE_LEECHBEAM`
(table `0x8151caf4`). That order is the one `weapon-type-bits.md` derived from
the `WeaponSetDefinition` names alone, so two unrelated sources agree on all
eleven bits (confidence 90 for the order).

**Geometry.** The row is `n * size + (n - 1) * 12` wide. Rules page: measured at
60 to centre, drawn at 63 with 12 between, top edge at item `y - 32`, tinted
`Blue2048`; the caption is centred at item `y + 38` and wraps at `max(140,
n * 63)`. When the rules page has exactly two items and the callout measures over
140 at 44 (three icons or more), `FUN_810535fe` overwrites the two-item row with
`(682, 234)` and `(682, 340)`. The vector-register arithmetic of the draw loop
was read, not run, so the placement is confidence 65.

Measured over the real `SP.xml` (`crates/2048/tests/campaign_ground_truth.rs`,
`the_weapon_callout_census_over_every_event_is_pinned`): of 141 events 63 (Zone
and Speed Lap) have no callout item, 54 offer everything and draw none, 2 draw
weapons off, 3 the defensive-off icon, and 19 list one to ten weapons.

## `CampaignEventCard_GlyphRowWidth` - `0x81055006`

**Confidence: 80.** The width of the objective page's glyph row, in
`CampaignEventCard_DrawObjectivePage`'s own terms: 56 for each of the class
glyph (mode not Zone), the trophy glyph (button shape `1`, `2`, `9`, `10` or
`11`), the lap glyph (`m_numOfLaps != 0`) and each allowed craft class, `n * 56`
for a callout of `n` icons (`width + 12`), and 108 for a forced craft. The row's
first cell is centred at `682 + 28 - width / 2`.

**The trophy glyph** is `callout/trophy`, drawn between the class glyph and the
lap glyph on every event whose button shape gives it a trophy page. Every
elite-trophy and cup event has it; nothing else does.

## `CampaignEventCard_DrawObjectivePage` - `0x81055150`: the elite row

`event+0x2d8` is the progress state; at `3` (passed) and `4` (elite) the page
shifts up 40 units (`190` instead of `230`) and adds `FE_ELITE_PASS` and the
elite objective's line under the pass line (`y` `259` and `287`), each with its
own medal glyph (`FUN_81061344`: `1` pass medal on the pass line; `0` no medal,
or `2` the elite medal at state `4`, on the elite line). Before a pass the page is
the single block frame 14 shows. Confidence 75.

## `CampaignEventCard_DrawForcedCraft` - `0x81061808`

**Confidence: 75.** `(x, y, fade, scale, team, ship, colour)`, disassembled
(vector-register arithmetic, not decompiled). At rest (`fade` 1) it draws the
team logo as a `64 * scale` quad from `(x, y)` and, over it, the craft class
icon as a second `64 * scale` quad from `(x - 40 * scale, y)`, both white. The
rules page calls it with `y = item y - 32` and centres the `"%s %s"` caption (the
team label, then `WOShipModelData_LiveryLabelId`'s `FE_SHIP_*`) at `item x + 16`.
The objective page's row passes a `scale` this pass did not read (so its
placement stays **chosen, not measured**: `0.7`).

## `FrontEnd_LoadTrophyTextures` - `0x8104d1d0`

**Confidence: 80.** Loads, with `FrontEnd_LoadCardTextures`, the art of the trophy
page: `arrow` (`DAT_816c76f0`), `callout/trophy` (`DAT_816c76f4`),
`Button_Indicator_Dot` (`DAT_816c76f8`), nine `trophy/%d_elite_%02d`
(`DAT_816c76fc..`: year `2048 + i / 3`, ordinal `i % 3 + 1`), twenty
`cups/mp_%d` (multiplayer), `trophy/Cup2048`, `Cup2049`, `Cup2050`
(`DAT_816c7720..28`) and the HD callout set. All are in the base package's
`data.psarc` (`data/FE/NewImages/trophy/*.gxt`).

### The trophy page - `CampaignEventCard_DrawTrophyPage` - `0x81052fb4`

**Confidence: 75** (was 65 as `_q`: the handles and the strings are now read, and
12 of 12 events resolve in `SP.xml`). Nine events by name, in this order:
`2048 - Event 2-2`, `3-2`, `5-2`, `2049 - Event 2-3`, `5-3`, `6-3`, `2050 - Event
2-4`, `5-4`, `6-4` - the `i`-th takes `trophy/{2048 + i/3}_elite_{i%3+1:02}` with
`TROPHY_{year}_{n}_1` (heading) and `TROPHY_{year}_{n}_2` (callout); otherwise
`m_buttonShape` `9`, `10`, `11` take `trophy/Cup{2048,2049,2050}` with
`Cup_Name_{year}` and `Cup_Callout_{year}`. The three cup events are `2048 -
Event 7`, `2049 - Event 8` and `2050 - Event 9`. The image is drawn at its own
size, centred on `(520, 283)`; the heading (`NEOSANS_BOLD`, scale `1.0`) starts at
`x = 570` and the callout (scale `0.7`, wrapped at 300, `y + 30` below it) under
it, the block centred on `y = 284` by the callout's wrapped height, in
`Blue2048`. A shape-`1`/`2` event that is not one of the nine has the page (it is
in `BuildPageList`) and draws nothing. **Chosen, not measured**: the heading's
`320.0` argument is read as a width to fit and is approximated by shrinking the
scale; the callout's wrapped height is estimated.
