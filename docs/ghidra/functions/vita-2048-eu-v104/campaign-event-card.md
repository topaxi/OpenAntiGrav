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

**Confidence: 70.** Fills `DAT_816c7804[]` (page kind per page) and
`DAT_816c7830` (the count) for an event. Single-player (`event+0x1a0 & 4 == 0`):
kind `0` (the objective page) first **when `event+0x1b0` is set** - the resolved
`M_PASSOBJECTIVE` - then kind `1` always; kinds `2` (event mode ordinals
`1`/`2`/`9`/`10`/`11`), `3` (`event+0x7c`) and `4` (any of: mode `!= 0`,
`event+0x18c`, a restriction) are appended by predicates over fields this build
does not read. Frame 14 shows three dots. **This build shows the pages it can
count: kind `0` when the event authors a pass objective, and kind `1`.** Kind
`1` (`FUN_810540c4`, three 142-wide tap zones at `x=471`/`613`/`755`, a
leaderboard-shaped row) is unread and is drawn blank.

### `CampaignEventCard_DrawObjectivePage` - `0x81055150`

**Confidence: 75.** Page kind `0`. `FE_PASS` (`NEOSANS_LARGE`) at `y=229`, the
objective's own wording at `y=256`, both centred on `x=702` and wrapped at
345 wide; a medal glyph (`FUN_81061344`) at the left; a rule at `y=329`
(`x=482..882`); then a row of 44.8-unit glyphs at `y=370`: the speed class
(`FUN_81061274`, `param_3[0x65]`) and, for event modes `1`/`2`/`9`/`10`/`11`,
`callout/num_laps` with the count; then the three restriction glyphs
(`FUN_81061db6`, `"combat"`/`"agility"`/`"speed"`) when the event restricts
craft. For event modes `3`/`4` (`param_3[0xb6]`) it adds an `FE_ELITE_PASS`
row. Not drawn by this build, by name: the elite row and the restriction
glyphs.

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
| `0x81516568` | `FUN_812c3d00` | `0` | `GameMode_ZoneRace` | `FUN_812c4a2a` (`0x812c4a2a`): type `2` -> `"%s : %d"` over `FE_ZONE_TARGET` (`ZONE TARGET : 15`) |
| `0x815163f0` | `FUN_812bfe14` | `3` | `GameMode_SpeedLapRace` | `FUN_812c1011` (`0x812c1010`): type `2` -> `MP-Objective_Beat_1` (`BEAT %s`) over the target as `M:SS` from `FUN_8114f252` (centiseconds, saturating at `9:59`) |
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
