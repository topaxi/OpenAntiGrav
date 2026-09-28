# The native campaign-map event card, and what its launch button checks

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. Found chasing the same lead as
[game-mode-base-fields.md](game-mode-base-fields.md): who calls
`GameModeBase_IsShipTypeAllowed` (`0x812b41da`). Both names here are
applied, from [names.tsv](names.tsv).

This is the screen `docs/formats/2048-frontend.md`'s campaign-map module doc
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
