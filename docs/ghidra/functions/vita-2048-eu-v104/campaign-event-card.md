# The native campaign-map event card, and what its launch button checks

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. Found chasing the same lead as
[game-mode-base-fields.md](game-mode-base-fields.md): who calls
`GameModeBase_IsShipTypeAllowed` (`0x812b41da`). Both names here are
applied, from [names.tsv](names.tsv).

**2026-09-29: the layout is recovered - see [the card's layout](#the-cards-layout-recovered-2026-09-29)
below; the paragraph that follows is the 2026-09-28 state, and its "lap-count
arrows" are page arrows.** This is the screen `docs/formats/2048-frontend.md`'s campaign-map module doc
comment says a live capture shows (frame `14`,
`data/reference/2048-frontend/14-campaign-map-event-card-unity-square.png`: a
photo backdrop, the event's name and kind, a `PASS`/objective line, a pair of
lap-count arrows, three pagination dots and three bottom buttons) but no
`NEWGUI` XML screen was ever found to author - "native-code-driven,
unlocated in any XML" was that pass's own conclusion. This pass locates the
*code* (the input handler and the draw call), not the layout: nothing here
places the photo backdrop, the pagination dots or the exact button rects on
screen, only what the three bottom buttons *do* once tapped.

## `CampaignEventCard_HandleInput` - `0x810f2164`

**Confidence: 75.** `undefined4 CampaignEventCard_HandleInput(GameModeBase
*card_owner, GameModeBase *event)` - decompiled looking for
`GameModeBase_IsShipTypeAllowed`'s own callers and found here, alongside two
other literal, button-shaped branches in the same function. Not renamed
higher: the three branches below are structurally clear (each is a hit-test
against a literal float rect in 960x544 space, `y=422`, matching the real
card's own bottom-button row height), but this was read off the decompile,
not watched running against a live tap this pass.

Three buttons, by their own literal `x` origin:

- **`x=812` - Launch.** Fires only when
  `GameModeBase_IsShipTypeAllowed(event, "PlayerLivery")` (the literal string
  `"PlayerLivery"`, resolved through `FUN_8105969e` to the player's own
  currently-selected livery) returns true; otherwise the tap is silently
  eaten - no redirect, no sound, nothing. `CampaignEventCard_Draw` (below)
  dims this same button to alpha `0x40` in the identical condition, so a
  disallowed craft is shown as disabled before it is ever tapped.
- **`x=682` - Back.** Plays the `newGUI_deny` sound cue and returns to the
  map - the same cue [`Frontend::launch_selected_event`]'s own `Locked`
  refusal answers to conceptually, though this project draws no sound for
  either refusal yet.
- **`x=552` - Change craft.** Exists only when `event+0x3c`
  (`m_pPlayerShipModelData`) is `0` - i.e. **never drawn on a forced-craft
  event**, matching `oag_2048::campaign::craft::forced_craft`'s own "14 of
  141, never alongside a restriction" census exactly. Its handler writes
  `event+0x2e8` into the player profile (`DAT_81545468+0x32588`) and redirects
  to the screen named at `0x81458244`, which [`inspect_memory_content`] reads
  as the ASCII string `"team"` - **the same `Team_Definition.xml` screen
  `oag_ui::frontend::team` already implements**, not a bespoke picker. No
  filter was found applied on that redirect (nothing on this path writes to
  or reads `GameModeBase+0x80`..`0x83` beyond the check already covered
  above) - the destination screen offers every craft, same as today, and the
  restriction is enforced only back on this card's own Launch button once
  the player returns to it. `CampaignEventCard_Draw` pulses this button
  orange/blue while the current craft is disallowed, the same visual cue a
  disabled Launch gets.

`[Frontend::launch_selected_event]`: `crates/ui/src/frontend/campaign_map.rs`.

## `CampaignEventCard_Draw` - `0x810f1196`

**Confidence: 75.** The card's own per-frame draw, decompiled as
`CampaignEventCard_HandleInput`'s sibling (same file region, same
`GameModeBase*` argument shape). Composites the Launch/Back/Change-craft
button art with the alpha-`0x40`/pulsing-colour states `HandleInput`'s own
branches describe above, and (in the branch gated on `event+0x3c == 0`, no
forced craft) draws the current livery's own class name through
`WOShipModelData_LiveryLabelId` (`0x8105b6c2`,
[game-mode-base-fields.md](game-mode-base-fields.md)) and
`FUN_8106b86e` (the string-table lookup every other card label already goes
through).

**Side finding, not chased this pass**: this function also computes the
native `M_X`/`M_Y` → screen-pixel projection
`docs/formats/2048-campaign.md`'s "What is not determined" section lists as
unfound - `DAT_81880450 + M_X*DAT_81880448 + canvasTweak_x(+0x314)`, the same
shape for `y` with `+0x2c8`/`+0x318`, projected through the matrix at
`DAT_8151d3f4+0x5ec90`. Left for whoever picks up that thread; the addresses
are recorded here so a second pass does not have to re-find this function to
get them.

## The card's layout, recovered 2026-09-29

The 2026-09-28 pass above located the card's code and stopped at its three
buttons. This pass followed `CampaignEventCard_Draw`'s call into the shared
panel painter and read the layout out of it, then checked every number against
the live Vita3K frame
`data/reference/2048-frontend/14-campaign-map-event-card-unity-square.png`
(pixel-measured, not eyeballed). **The card is not a `NEWGUI` screen and no XML
authors it**: an archive-wide search of every `.xml` in `data.psarc` for
`trackscreens`, `num_laps`, `callout\play` and `callout\cross` finds nothing,
which is why it was "unlocated" - its art was never named by a widget. It is in
the archive all the same (`data/FE/NewImages/trackscreens/*.gxt`,
`tracks/*.gxt`, `callout/*.gxt`, `speedclass/*.gxt`, `medals/*.gxt`), and the
executable names it in string tables.

### `CampaignEventCard_DrawPanel` - `0x81055a16`

**Confidence: 75.** `void (float slide, GameModeBase *event)`. The shared
painter `CampaignEventCard_Draw` (`0x810f1196`) calls as
`FUN_81055a16(slide, event)`; it is also the pre-race callout's
panel (its `param_2 == 0` branch is the tournament header, not read). For an
event it draws, with `x` the slide-in offset (`0` at rest) - every literal
below is in the decompile and matches frame 14 to the pixel:

| Piece | Where | Evidence |
| --- | --- | --- |
| Photo | `x=16`, 404 wide, 334 tall (`y=92`); texture UV `0..404/512` | `FUN_81060744(x, trackName, isZone)` builds the quad `x..x+404`, `u1 = 0x3f4a0000 = 0.789 = 404/512`; the texture is a 512x512 `PVRTII4bpp` whose photo fills the top-left 404x334 (decoded: `trackscreens/Square.gxt`) |
| Header panel | `x=420` `y=92` 524x86, `Transparent2048` (`0xc0ffffff`) | two `0x18`-stride quad writes, `y` `92`..`178` |
| Body panel | `x=420` `y=182` 524x244 | second quad, `y` `182`..`426` |
| Emblem | centre `(463, 134)`, 64 square (`+/-32`) | `FUN_81060584(x, y, trackName, colour)`; one textured quad tinted Blue2048 (the texture is a white square with the glyph as alpha) |
| Title | centre `x=682`, `y=100`, font `default`, scale `0.82` | `DAT_819488ec = 0x3f51eb85` |
| Kind line | centre `x=682`, `y=140`, `NEOSANS` | |
| Mode icon | centre `(902, 134)`, 64 square | `FUN_81061040(x, y, 1.0, event+0x190, colour)` picks one of four textures (`DAT_816c87e0`..`ec`) by the event's mode ordinal |
| Page arrows | `32x32` at `(445, 282)` and `(920, 282)`, right one the same texture mirrored | `FUN_8105c5dc` with swapped `u`; tap regions `x=420` and `x=682`, 262 wide (`FUN_8105b54c`), plus a swipe path; each step plays the `NGP_Change` cue |
| Page dots | centre `x=682`, spaced 18, the current one Orange2048, the rest Blue2048 | `(682 - (n-1)*9) + i*18`, drawn only when the page count exceeds 1 |

**The arrows are page arrows, not lap-count arrows.** The task brief and the
2026-09-21 frame note both read them as lap-count arrows; they step
`DAT_816c782c` (the page index) by `+/-1` and wrap. The lap count is the
`num_laps` glyph in the objective page, with the number drawn on it.

Not renamed higher: 75, because this function is shared with the pre-race
callout and only its event branch was read.

### `CampaignEventCard_BuildPageList` - `0x8105114a`

**Confidence: 80** (raised 2026-09-29, when every field it reads was mapped
against `GameModeBase_RegisterFields`, `0x812b0e2a`). Fills `DAT_816c7804[]`
(page kind per page) and `DAT_816c7830` (the count) for an event, in this
order for a single-player event (`event+0x1a0 & 4 == 0`, `m_options`):

| Kind | When | The field it tests |
| --- | --- | --- |
| `0` objective | `event+0x1b0 != 0` | `m_PassObjective` (`SP.xml` `M_PASSOBJECTIVE`) |
| `1` leaderboard | always | - |
| `2` trophy or cup | `event+0x19c` is `1`, `2`, `9`, `10` or `11` | `m_buttonShape` (`M_BUTTONSHAPE`, `CanvasButtonShape`) - **not** a mode ordinal; the earlier "mode ordinal" reading was wrong |
| `3` pass-to-unlock | `event+0x7c != 0` | not a registered field: a runtime `WOShipModelData*` (`FE_PASS_TO_UNLOCK` names its team and livery), so probably the resolved unlock reward. `M_PUNLOCKDATA` (`+0x310`) is authored on none of `SP.xml`'s 141 events, so **kind 3 is probably unreachable** - confidence under 50 |
| `4` rules | the count of rule icons is non-zero | `+0x190 != 0` (class), `+0x18c` (`m_numOfLaps`), the weapon callout (`FUN_810626ce`), `+0x3c` (forced craft), and the class glyphs `GameModeBase_IsShipTypeAllowed` still allows when any of `+0x80..+0x83` is set |

Frame 14 shows three dots on a card with an objective, a race class and three
laps: kinds `0`, `1` and `4`. **This build builds `0`, `1` and `4`.** Kind `2`
draws art this build has no handle for, so it is left out (the ten
`"... - Event N-2/3"` trophy events and the cup shapes `9`-`11`); kind `3` is
left out for the reason above. The card opens on the rules page when the
player's current craft is refused (`DAT_816c782c`, `PlayerLivery` not
allowed, no forced craft).

Field offsets, from `GameModeBase_RegisterFields`: `+0x18c` `m_numOfLaps`,
`+0x194` `m_speedClass`, `+0x198` `m_weaponSet`, `+0x19c` `m_buttonShape`,
`+0x1a0` `m_options`, `+0x1b0` `m_PassObjective`, `+0x1c8` `m_EliteObjective`,
`+0x1f0` `m_activeWeaponPads`, `+0x310` `m_pUnlockData`. `+0x190` is not
registered: each `GameMode_*` constructor writes it (the mode-icon ordinal,
`0` zone, `1` elimination, `2` race, `3` speed, see the vtable table below).

### `CampaignEventCard_DrawObjectivePage` - `0x81055150`

**Confidence: 75.** Page kind `0`. `FE_PASS` (`NEOSANS_LARGE`) at `y=229`, the
objective's own wording at `y=256`, both centred on `x=702` and wrapped at
345 wide; a medal glyph (`FUN_81061344`) at the left; a rule at `y=329`
(`x=482..882`); then a row of 44.8-unit glyphs at `y=370`: the speed class
(`FUN_81061274`, `param_3[0x65]`) and, for event modes `1`/`2`/`9`/`10`/`11`,
`callout/num_laps` with the count; then the three restriction glyphs
(`FUN_81061db6`, `"combat"`/`"agility"`/`"speed"`) when the event restricts
craft. For event modes `3`/`4` (`param_3[0xb6]`) it adds an `FE_ELITE_PASS`
row. The elite row, the trophy glyph, the weapon callout and the restriction glyphs
are drawn 2026-10-05; see [weapon-callout.md](weapon-callout.md).

### `GameModeObjective_FormatText` - `0x812b671a`

**Confidence: 85.** The ordinal-to-wording table for `M_OBJECTIVETYPE`,
switch on the ordinal: `1` -> `SP_Objective_Finish` (`MP_Objective_Finish` on a
multiplayer event), `2` -> `FE_SCORE_POINTS`, `4` -> `ER_FINISH_1ST`/
`ER_FINISH_2ND`/`ER_FINISH_3RD`/`ER_FINISH_IN_POS` (`FINISH AT LEAST %dTH`),
`7` -> `FE_ELIMINATE_OPP`, `3`/`5`/`6`/`8`/`0xb`/`0xc` the multiplayer
wordings. It first offers the event a per-mode override
(`vtable+0x6c`, arguments `(event, type, target, buf)`, returns non-zero when
it wrote the text). **Read 2026-09-29.** Eight `GameMode_*` vtables carry the
slot; six keep the base class's stub at `0x813ea238` (`movs r0,#0; bx lr`,
disassembled), two override it, and each vtable is pinned to its class by the
mode-icon ordinal its constructor writes to `+0x190` (`FUN_81061040` maps
`0` zone, `1` elimination, `2` race, `3` speed) - the class hash in
`oag_tables::mjolnir::campaign::typedef` agrees:

| Vtable base | Constructor | `+0x190` | Class | `+0x6c` |
| --- | --- | --- | --- | --- |
| `0x81516568` | `FUN_812c3d00` | `0` | `GameMode_ZoneRace` | `GameModeZoneRace_FormatObjective` (`0x812c4a2a`): type `2` -> `"%s : %d"` over `FE_ZONE_TARGET` (`ZONE TARGET : 15`) |
| `0x815163f0` | `FUN_812bfe14` | `3` | `GameMode_SpeedLapRace` | `GameModeSpeedLapRace_FormatObjective` (`0x812c1010`): type `2` -> `MP-Objective_Beat_1` (`BEAT %s`) over the target as `M:SS` from `Time_SplitCentiseconds` (centiseconds, saturating at `9:59`) |
| `0x81516340` | `FUN_812be628` | `1` | `GameMode_EliminatorRace` | default (`return 0`) |
| `0x8151611c`, `0x815161cc`, `0x815164b8` | `FUN_812bb914`, `FUN_812bd128`, `FUN_812c377a` | `2`, `2`, `4` | the other race classes | default |

So `BEAT_VALUE` (`2`) is worded three ways: SpeedLapRace as a time, Zone as a
count, and everything else - Elimination in particular - falls through to
`FE_SCORE_POINTS` (`SCORE %d POINTS`, which **is** in the disc's English table
in both `data.psarc` and the patch's `entries.xml`; the previous pass's "not in
the string table" was wrong). The three classes' `SP.xml` typedefs carry every
`BEAT_VALUE` event: 13 on `-1915183557` (SpeedLapRace), 10 on `1018671239`
(Zone), 15 on `1311982788` (Elimination). The Elimination wording is evidence
for the metric `2048-campaign.md` had left unidentified: the original words its
`25`-`100` targets as **points scored**, confidence 80 (the class was pinned by
its constructor and the vtable stub was disassembled; what the sim counts as a
point is still unread). Confidence for the wording table: 85.

### `CampaignEventCard_DrawLeaderboardPage` - `0x810540c4`

**Confidence: 75.** Page kind `1`, `void (float dx, float dy, event)`. Three
tabs of 138x44 at `x=471`/`613`/`755`, `y=186` (`FE_PERSONAL`, `FE_FRIENDS`,
`FE_GLOBAL`, `NEOSANS_BOLD` at `0.7`, centred, `y+13`); the selected tab is
`Orange2048`, the others `Blue2048`, or `Grey2048` when
`FUN_81258f8e` (the network check `BuildPageList` also uses) is `0`, with the
label dimmed to `0x40ffffff`. With no network `BuildPageList` forces the
Personal tab (`DAT_816c7840 = 1`), whose body is a `Grey2048` bar
(`x=471..893`, `y=234..271`) with `FE_CURRENT_BEST` in white at `(682, 243)`,
and, when the event has no record, `"--"` (`0x814274d4`) centred at
`(682, 332)`. With a record it draws the player's best (`FUN_8106ed02`
fields: name, time or count, XP) - **not drawn by this build**: it keeps no
per-event result beyond the medal. The online tabs draw `FE_NO_RECORDS` or a
row list (`x=511..893`, rows 21 apart, up to eight) - not drawn: no network.

### `CampaignEventCard_DrawRulesPage` - `0x810535fe`

**Confidence: 75.** Page kind `4`. Up to six items placed by a table at
`0x8151c9c8` (copied into a local; item `k` of `n` is at row `n-1`, column
`k`, `(x, y)` pairs; the five-item row leaves the lower left empty as
authored): class glyph (`FUN_81061274`, 64 unit quad, caption
`Speed_Class_{C,C,B,A,A_Plus}_0` for ordinals `0`-`4` by `FUN_812b26cc`),
laps (`FUN_81061102`, `Callout_Lap`/`Callout_Laps`), the weapon callout
(`CampaignEventCard_DrawWeaponCallout`, `0x810626ce`: icons composed from
`m_weaponSet` and `m_activeWeaponPads`, see [weapon-callout.md](weapon-callout.md)), the forced craft
(`FUN_81061808`: team logo and class icon, caption `"%s %s"` of team and
livery labels), then a glyph and `FE_SHIP_COMBAT_ONLY`/`_AGILITY_ONLY`/
`_SPEED_ONLY` for each class still allowed when any prevent flag is set
(`FUN_81061db6`). Captions are `NEOSANS_BOLD` at `0.6`, centred at `y+38`,
140 wide. The glyphs are `Team_Logos/Icon_Ship_{Combat,Agility,Racer}_1col`
(`FUN_81061db6`; `Icon_Ship_Racer_1col` for speed), the class glyphs
`speedclass/{d,c,b,a,ap}_class` (`FUN_81061274`, confirming the ordinal order),
the team logos `Team_Logos/Icon_Team_{Feisar,AG-SYS,Qirex,Auricom,Pirhana}`
and the class icons `Icon_Ship_{Combat,Agility,Racer}` with `_proto` variants
(`FUN_81061808`). 2026-10-05: `FUN_81061808`'s two quads are disassembled (see
[weapon-callout.md](weapon-callout.md)): the logo from the item's `x`, the class
icon 40 to its left and over it, both 64 square from `y - 32`, the caption 16
right of the item.

### `FrontEnd_LoadCardTextures` - `0x8105dcb8`

**Confidence: 80.** Loads every texture the card and the HD-era glyph helpers
draw with, into the `DAT_816c87xx`/`DAT_816c88xx` handles. The class glyphs at
`0x816c87c8..d8` are `d_class`, `c_class`, `b_class`, `a_class`, `ap_class`
in that order; the four mode icons at `0x816c87e0..ec` are `combat_mode`,
`speed_mode`, `race_mode`, `zone_mode`; `0x816c8770..80` the five native team
logos, `0x816c8784/88/8c` the `_1col` class glyphs and `0x816c8790/94/98` the
coloured class icons.

### `CampaignEventCard_DrawTrophyPage` - `0x81052fb4`

**Confidence: 75** (raised 2026-10-05 from 65, when `_q` came off). Page kind `2`:
the nine named events' elite trophy and the three cup events' cup, with a
heading and a callout. The image handles, the strings, the events and the layout
are in [weapon-callout.md](weapon-callout.md); it is drawn.

### `CampaignEventCard_DrawUnlockPage_q` - `0x81052810`

**Confidence: 60.** Page kind `3`: `FE_PASS_TO_UNLOCK` and a panel naming the
craft `event+0x7c` points at. Left out, see `BuildPageList`.

### `GameMode_GetKindLabel` - `0x812b021a`

**Confidence: 85.** The line under the card's title, by `event+0x190`: `0`
the literal key `ZONE`, `1` `FE_GAMEMODE_ELIM` (`COMBAT` in English), `2`
`IG_HUD_RACE`, `3` `SPEED LAP` when `event+0x19c` (`m_buttonShape`) is `5` or
`6` and `TIME TRIAL` otherwise, anything else empty. So `GameMode_SpeedLapRace`'s
53 events are 40 `SPEED LAP` (shapes `5`, `6`) and 13 `TIME TRIAL`, and
Elimination's line reads `COMBAT`, not `ELIMINATION`.

### `Time_SplitCentiseconds` - `0x8114f252`

**Confidence: 85.** `(centiseconds, char *m_ss, char *cc)`: writes `M:SS` and
the hundredths, saturating at `9:59` and reading the `0xffff` "no time"
sentinel as zero.

### `CampaignEventCard_DrawTrackPhoto` - `0x81060744`

**Confidence: 85.** `void (float x, char *trackName, char isZone)`. Matches
`M_TRACKNAME` against `square`, `park`, `tower`, `mall`, `bridge`, `arena`,
`subway`, `cathedral`, `sol`, `altima`, then the DLC circuits and their `_R`
reverses, then `Zone_1`..`Zone_4`, and indexes two 36-entry string-pointer
tables at `0x8151cc68`: the plain `data/fe/newimages/trackscreens/<Name>.gtf`
(`Square`, `Park`, ...) and, when `isZone`, `Zone<Name>.gtf` for the ten base
circuits (`event+0x190 == 0`). The names in those tables (read from `0x814292ac`)
are exactly the files in the archive, so the circuit-to-file map is data, not
a guess. `SP.xml`'s Zone events author no circuit, so this build draws no photo
for them: the zone photo would need the circuit a Zone run uses, which is a
title fact not found.

### Button glyphs and texts

The three buttons are 122x96 at `x=562`/`692`/`822`, `y=432`, Blue2048, with a
64-unit glyph, `FUN_8106202a(x, y, 122, 96, 64, 64, ...)` (`CampaignEventCard_Draw`).
Hit rects are 142x116 at `x=552`/`682`/`812`, `y=422`
(`CampaignEventCard_HandleInput`); Back is tested before Launch before Change
craft, so Back owns the 12 units it shares with each neighbour. Glyphs, by
decoded image: Launch is `callout/play`, Back is `callout/cross`, Change craft is
the ship silhouette that is `Icon_Team_HomeBut` (matched by eye against frame
14; the texture handles are stored by the card's constructor, not found, so this
one is confidence 65).

## `PostRace_ShipCategoryTip` - `0x8111eb74`

**Confidence: 70.** A post-race "try a different ship class" tip picker -
references `"SkipRaceTip"`/`Post_Race_Fail_Tips_Ship` and, for each of
`"combat"`/`"agility"`/`"speed"` in turn, checks (a) that category's own
prevent-flag is clear, (b) the event authors no forced craft
(`GameModeBase+0x3c == 0`), and (c) the player's *current* craft is already
that category - then searches the ship-model registry
(`DAT_81967280`, the same one `GameModeBase_IsShipTypeAllowed`'s own
prototype-substitution walks) for any craft of a category the event does
still allow, and if one exists, shows a tip naming it through
`WOShipModelData_LiveryLabelId`. Not renamed higher than 70: the surrounding
tip-selection machinery (a 9-slot table walked by an LCG,
`DAT_8153ebe4 = DAT_8153ebe4 * 0x343fd + 0x269ec3`) is read but not itself
understood well enough to be confident this is the *only* thing the function
does. Corroborating evidence for the restriction mechanism, not itself
player-facing UI this project builds anything from this pass.

## See also

- [game-mode-base-fields.md](game-mode-base-fields.md) - `GameModeBase`'s own
  field table and `GameModeBase_IsShipTypeAllowed`'s own logic.
- [`docs/formats/2048-campaign.md`](../../../formats/2048-campaign.md)'s
  "Craft choice" section - the player-facing finding this evidence supports.
- [`docs/formats/2048-frontend.md`](../../../formats/2048-frontend.md) - the
  campaign map's own "bottom panel is gone" section, whose "unlocated in any
  XML" conclusion this page narrows to "unlocated layout, located code".
