# Pulse's race box is read on both pressings, and the code side is unblocked

2026-09-05, updated 2026-09-10. The Pulse half of the race-box investigation,
PSP and PS2 together because the PS2 is a delta rather than an independent
measurement. The permanent write-up is
[`docs/formats/race-setup.md`](../../docs/formats/race-setup.md), with the
decompiled screen classes at
[`docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`](../../docs/ghidra/functions/psp-pulse-usa/race-box-screens.md).

## What is settled

The three-page flow, read straight out of the disc's own front-end XML:

```
Main Menu (FE_RACEBOX) -> Single Player -> Track Creation -> Team Selection -> Launch Game
```

`Single Player` authors five lists - Mode, Class, Weapons, Difficulty
(persisted as `SkillLevel`), Eliminations. `Track Creation` and `Team Selection`
author their chrome but bind their rows from code. Both previews are meshes.
See the docs page for the widget inventory, the two unlock axes and the
confidence scores.

**The PS2 pressing's race-setup page is identical** - same lists, same globals,
same idstrings, same defaults, same `GSDisableEntriesBitField="0xFA"`. Its
divergences are all around the page, and they are recorded on the docs page
too: the `Racebox` menu bypassed, split screen added, `Team Selection` widget
names `0`-suffixed, `Tournament C` widened from four track slots to twelve.

**Both previews are now captured live in PPSSPP, not just read out of
`.rodata`** - 2026-09-05, see
[the docs page's capture section](../../docs/formats/race-setup.md#captured-live-in-ppsspp-2026-09-05).
`Track Creation`'s preview is an animated first-person flythrough of the
circuit corridor in a hexagonal frame; `Team Selection`'s is a turntable-
rotating craft on black. The three-entry wrapping track list was walked all
the way round and confirmed to be exactly `16_Track`/`03_Track`/`18_Track`
with no locked entry reachable from this screen. `Black`/`White` in the
`TrackSelection` string block are confirmed to be the two circuit runs (the
screen's own Help text says so), not a preview toggle.

**The relocation blocker is resolved, and both screen classes are now
decompiled in full.** The PSP Allegrex relocation patch landed 2026-09-07
across all four PSP Ghidra databases (`HANDOVER.md`, "Traps that are live"),
and `TrackSelection`/`TeamSelection` decompile cleanly with it applied - see
[`race-box-screens.md`](../../docs/ghidra/functions/psp-pulse-usa/race-box-screens.md)
for the full function table, fifteen names applied at 72-82 confidence, and
what those functions do:

- **What populates the track list is now found.**
  `TrackSelection_PopulateList` filters on `<Unlock>` plus a second, mode-
  gated per-track byte flag - the mechanism behind "why Custom Race offers
  three circuits" going from an XML-only inference to code.
- **`Top->Ship` has a confirmed referent.** It is a node inside each track's
  own dynamically created preview scene, not a static screen widget - which
  is why no button probe on the live capture ever found it.
- **`Team Selection`'s stat bars, Eliminator/Normal variant pick and
  suggested/forced-ship help text are all traced to code**, not just the XML
  widget inventory.
- **One function (`Definition_IsUnlocked`) was cross-checked against
  `psp-pulse-eu`** by exact opcode hash and confirmed (123/123 instructions
  equal); the other fourteen have no exact-hash match on the EU binary, and
  a single-address fuzzy check found nothing above 0.55 either - the
  front-end code region diverges more between pressings than the physics
  code the earlier EU transfer pass covered. See `race-box-screens.md`'s own
  cross-platform section.

**2026-09-09: both screens are built, and the capture that measured them
is a script.** `scripts/psp-frontend-capture.py` walked the original from
boot to `Team Selection` and screenshotted 54 frames (every row of `Main
Menu`/`Racebox`/`Single Player`, every `RACE TYPE`, both selection screens
per entry, the help and music overlays). Read against them, this build now
opens Track Select and Ship Select from the RACE page's START row -
`oag_ui_screens::picker` (model, layout read off `Selection_Definition.xml`, draw),
`oag_game::preview` (the 3D pass), `session/picker.rs` (the flow), with
`--menu-page track-select`/`ship-select` for a headless still. Two
corrections fell out: the `%s\FE\%s.vex` file is the **panel's outline
ribbon** (364 vertices, radius 805), not the flythrough; and the per-craft
rating record is `Definition.xml`'s own `<FE speed thrust handling shield>`
under each `PI_Team`, matching the capture's bars digit for digit. Three
parser gaps closed on the way (`LeftLayer`/`Item` not walked, offsets not
summed, `Color1..4` gradients dropped) and `LeftLayer`'s `OffsetX` is an
origin, settled. Full reading and the remaining gaps:
[`docs/ui/selection-screens.md`](../../docs/ui/selection-screens.md).

**2026-09-10: the hexagonal window is built, and it was never a
flythrough.** `TrackSelection_ApplySelection`'s call into `FUN_088c4410` -
now `TrackDefinition_EnterScreenState` - loads the circuit's own
`<location>\screen.xml` and enters `Info`: a two-second chain of six
states stacking four hex-cropped stills (`FE\image_01..04.mip`) and
walking back down. Read by `oag_ui_screens::picker::slideshow`, loaded through the
front end's sheet extended per selection, drawn on both pressings. On the
way: the hex grid is a 32x16 tile the sheet could not repeat
(`Draw::TiledSprite`), the PS2's `Team Selection` widgets carry a `0`
player suffix, the PS2 previews needed `oag_livery::entry::ps2_texture_set`, and the
RACE page's TEAM/VARIANT/TRACK rows are dropped on a title with these
screens (`Definition::drop_rows_picked_on_screen`). Docs:
[`selection-screens.md`](../../docs/ui/selection-screens.md),
[`race-box-screens.md`](../../docs/ghidra/functions/psp-pulse-usa/race-box-screens.md).

## Open

- ~~The outline is framed off the capture rather than off `screen.xml`'s
  own `Mode3D` pose~~ **2026-09-28: closed.** `Mode3D` has no camera
  position or rotation of its own, only `Origin`/`nearZ`/`farZ`; the child
  `Model` is what moves. `Origin` is a plain screen-pixel shift of the
  viewport's own centre, `-145,13` (PSP) reading as `-(193,-21)` scaled to
  the PS2's own `640x448` grid rather than measured there directly. See
  `docs/ghidra/functions/psp-pulse-usa/race-box-screens.md#mode3ds-own-fixed-camera-and-the-child-models-own-pose-2026-09-28`
  and `oag_game::preview::mode3d_view_projection`. Left open there: the
  `Model`'s own rotation order in this renderer's convention is a plausible
  transpose of the PSP's row-vector one, not breakpoint-confirmed, and the
  PS2's `Origin` sign is inferred rather than read off `psp-pulse-eu`.
  ~~The cards arrive without their transition~~ **2026-09-28: closed.**
  `LeftLayer`'s own `transition` attribute is a per-widget fade-in/fade-out
  duration in seconds - not a slide, not a scale - read off the generic
  widget constructor (`Widget_CreateFromElement`, confidence 72,
  `docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`) and confirmed
  against a live PPSSPP capture (panel and first card both faint at 130ms
  into `Track Creation`, settled by 320-480ms, title bar's own
  `transition="0"` group solid throughout). The panel now fades in
  (`oag_ui_screens::picker::body`, reading `Text`/`Image`/`Fill::transition`); the
  hexagonal window's own stills author no `transition` of their own, so
  their fade (`oag_game::preview::CARD_FADE_SECONDS`) reuses the panel's
  measured `0.5`s rather than inventing one, labelled chosen-not-measured
  in both places it is used. New CLI flag `--menu-picker-seconds` (`--menu-page
  track-select`/`ship-select`'s equivalent of `--menu-anim-phase`, since
  neither that flag nor `--ticks` reaches this screen) makes the arrival
  reviewable as a still. See `docs/ui/selection-screens.md`.
- **Split screen is not built, and the PS2 authors it here.** The `0`/`1`
  widget suffixes are player indices - `Team Selection` carries the `0`
  set alone, `Team SelectionSplit` both, and `Track CreationSplit` sits
  beside it with `MP_Screen.xml` as its per-circuit slideshow. This build
  strips the `0` and opens only the single-player screens
  (`picker::strip_player_suffix`); when split screen arrives, that strip
  becomes "read set N" and the `*Split` screens are the ones to open.
- **The PS2 screens have no capture of their own to read against** - both
  are drawn off the PSP reading scaled to 640x448, with the PS2's own card
  sizes (256x128 on the larger grid) and `0`-set placement unverified on
  PCSX2.
- **Distance is measured on a worker and reads ~2% under the original**
  (`5094` vs `5178`, `5228` vs `5350`, `4330` vs `4419`) - not sampling,
  the figure is stable from one to sixty-four steps per segment. The
  original sums a different curve or counts the junction links differently;
  `circuit_length`'s doc has the numbers. Reading `TrackSelection`'s own
  formatter would settle it.
- **`Loyalty` is not drawn** - no counter to show. ~~The livery row cycles
  the variant axis~~ - it is the skin axis now (`Classic` + each
  `PI_ModelSkin`, `race.skin`), repainting the preview and reaching the
  race; the variant axis stays for teams that declare no skin.
- **The live flow is compile-checked and headless-captured, not played**:
  no display on this machine for the windowed route. First thing for
  whoever has one: START -> Track Select -> Ship Select -> race, then
  Circle back through both.

- **The nine unlock-predicate functions behind `Definition_IsUnlocked` are
  unnamed and untraced.** This is what "circuits gate on a named grid" and
  "craft variants gate on loyalty" need to become a confirmed mechanism
  rather than an XML reading corroborated by one live capture.

- **`GSDisableEntriesBitField` is undecoded.** Read as "bit N disables entry
  N", confidence 72 - self-consistent on Pulse's `0xFA` over seven entries and
  on HD's `0xF0` over five, and in both cases every bit at or above the entry
  count is set. Nothing traced to the consuming code.

- **Six fields in the serialised race record have no authored row**:
  `Locked`, `damage`, `AICount`, `laps`, `ship`, `ShipChoice`. `ShipChoice="Yes"`
  might be the flag deciding whether `Team Selection` is shown at all, which
  would make the third page conditional - confidence 45, do not build on it.

- **Five of Pulse's 23 definition files do not resolve in `Data.wad`**
  (`Controls_`, `Credits_`, `Debug_Screens`, `MemoryStickBootScreens`,
  `MemoryStickScreens`). On Pure, `Controls_Definition` *does* resolve and
  carries no `localised` attribute, so Pure's own two-file failure has a
  different cause and does not explain Pulse's five. Both are still open.

- **`RB_YG` is a dead idstring** - referenced by a redirect in
  `Selection_Definition.xml`, on no menu anywhere.

- **The locked-circuit info panel is confirmed unreachable from `Track
  Creation` itself** (the wrapping list only ever holds the three ungated
  tracks), but still genuinely unanswered: reaching one needs `Tournament C`
  (a different screen, own binding) or campaign-grid progression, neither
  tried.

- **The `Info Track %d.%d` = (count, index) layout-selection hypothesis is
  still untested rather than confirmed or refuted.** The code shows a
  *different* mode-driven axis (2-widget vs 7-widget stat layout) that is
  orthogonal to this question - see `race-box-screens.md`'s own note on it.

- **The per-craft rating record (`Speed`/`Thrust`/`Handling`/`Shield` shown
  on `Team Selection`) resolves through `FUN_08808664` at offsets
  `+0xb4..+0xc0`, a table distinct from `HandlingStats.xml`.** Its own source
  file and struct are unlocated.

- **Nothing in `race-box-screens.md` is runtime-verified.** Everything is
  static decompilation, capped at the rubric's 70-84 band. A PPSSPP capture
  with a breakpoint in `TrackSelection_PopulateList` or `TeamSelection_Update`
  would move several of those into the 90s.

## Next Steps

1. Trace the nine unlock-predicate functions
   (`race-box-screens.md#definition_isunlocked-and-the-two-axis-unlock-read`)
   to turn the two-unlock-axis reading from an XML-plus-one-capture
   corroboration into a confirmed mechanism.
2. Run a proper `bulk_fuzzy_match` sweep (descending thresholds,
   collision-filtered, each candidate individually `diff_functions`-checked)
   against `psp-pulse-eu` for the fourteen functions that had no exact-hash
   match, per this project's EU-preference. A single-address fuzzy check
   already came back too weak to act on; a broader sweep may still turn up
   real matches the same way the physics-code pass found 115.
3. ~~Walk the PS2's race box on PCSX2 the way `scripts/psp-frontend-capture.py`
   walks the PSP's, and read the two screens' PS2 captures against
   `--menu-page track-select`/`ship-select` on `pulse-ps2-eu.chd`.~~ **Done
   2026-09-27**: `RACEBOX` -> `TRACK SELECT` -> `SHIP SELECT` on
   `pulse-ps2-eu.chd` (`SCES-54748`, PCSX2, English). Both screens match
   this build's `--menu-page track-select`/`ship-select` digit for digit -
   Talon's Junction White (1/3, `Distance(m)` reading the disc's `5178`
   against this build's already-documented `5094`, ~2% under, unchanged),
   Assegai/Classic (1/12, `Speed 8 / Thrust 8 / Handling 9 / Shield 7`). No
   card-size or `0`-suffixed-widget placement bug found, closing
   `docs/ui/selection-screens.md`'s "no PCSX2 walk exists yet" gap. Full
   captures and the `RACEBOX` row list itself (which turned up a separate,
   real gap - see below) are under
   a scratch directory, not kept.

   **New finding from the same walk: `RACEBOX` authors two rows this
   build's `race` page does not.** The disc's screen (`racebox1.png`) has
   five - `RACE TYPE`, `SPEED CLASS`, `WEAPONS`, `AI DIFFICULTY`, `KILLS` -
   against this build's three (`MODE`, `SPEED CLASS`, `AI DIFFICULTY`;
   `TEAM`/`VARIANT`/`TRACK` are correctly dropped, per "The flow" above).
   **2026-09-30: `KILLS` now has its row** (`race.kill_target`, values off
   the disc's `Eliminations` list, greyed outside an Eliminator). **2026-09-30
   (`pulse-racebox-rows`): `WEAPONS` has its row too** - `race.weapons`, the
   disc's own `Weapons` list (`On`/`Off`, labels `FE_ON`/`FE_OFF`), editable in a
   single race and greyed showing the mode's own answer elsewhere (Off in Time
   Trial/Speed Lap/Zone, On in Eliminator - `FUN_088e715c`, confidence 85,
   `docs/formats/race-setup.md`), carried into the race as
   `Options::weapons_override` so Off drops the pads and the damage rules'
   weapons flag together. The blank SPEED CLASS value in `--menu-page race`
   was the capture never supplying that list (`capture::menu_page::race_sources`
   now does). Still open on the page: AI DIFFICULTY shows its stored value
   greyed where the original swaps in `N/A` (`DifficultyNaText`), Zone's
   `Zone` text over the SPEED CLASS row is not drawn, and the original's list
   default for Difficulty is `Easy` of three where this build's is `elite` of
   four.
4. HD's equivalent screen-population code is still unfound - see
   `docs/ghidra/functions/ps3-hdfury-eu/track-selection-screen.md` - and
   solving it there would let the two titles' readings corroborate each
   other the way `race-box-screens.md`'s method could be reused for.

## From the HANDOVER.md index (moved 2026-09-25)

**2026-09-10: the hexagonal window is a slideshow of the circuit's own stills, authored in a per-circuit `screen.xml` that `TrackDefinition_EnterScreenState` loads - built on both pressings, with the hex grid tiled, the PS2's `0`-suffixed widgets and untextured previews fixed, and the RACE page's TEAM/VARIANT/TRACK rows dropped on a title with these screens.** Open: the cards' transition and the outline's authored `Mode3D` pose, a PS2 capture to read against. Earlier, 2026-09-09: both selection screens are built and drawn off the disc's own XML** (`oag_ui_screens::picker`, `--menu-page track-select`/`ship-select`), measured against a 54-frame scripted PPSSPP walk (`scripts/psp-frontend-capture.py`, [selection-screens.md](../../docs/ui/selection-screens.md)); the `FE\*.vex` file turned out to be the panel's outline ribbon, not the flythrough, and the rating table is `Definition.xml`'s own `<FE>` element. Open: the flythrough, the distance, loyalty, the skin-vs-variant row, and a played run of the live flow. Earlier: the three-page flow is read straight off the disc: `Single Player` (five lists - Mode, Class, Weapons, Difficulty, Eliminations) -> `Track Creation` -> `Team Selection` -> `Launch Game`, with the PS2 pressing identical on the page itself and diverging only around it (`Racebox` bypassed with the file saying so in its own comments, split screen added, `Team Selection` widgets `0`-suffixed as a player index, `Tournament C` widened from four track slots to twelve). Two unlock axes, not one: circuits gate on `<Unlock Grid="Grid0">` - a **name**, not an index, so an unlock stored as an integer will not round-trip - and craft variants gate on per-team loyalty. Exactly three `PI_Track` entries carry no `<Unlock>` at all, which is why Custom Race offers three circuits. **Both previews are now captured live in PPSSPP (2026-09-05)** rather than read only out of `.rodata`: `Track Creation` is an animated first-person flythrough of the circuit corridor in a hexagonal frame, `Team Selection` a turntable-rotating craft on black, the three-entry track list walked all the way round with no locked circuit ever reachable from that screen, and `Black`/`White` confirmed as the two circuit runs off the screen's own Help text. **2026-09-08: both screen classes are now decompiled, fifteen functions named** ([`race-box-screens.md`](../../docs/ghidra/functions/psp-pulse-usa/race-box-screens.md)), unblocked by the 2026-09-07 PSP relocation patch - `TrackSelection_PopulateList` closes "what populates the track list" (the `<Unlock>` filter plus a second, mode-gated per-track flag), `TrackSelection_ApplySelection` closes `Top->Ship` (a node inside each track's own dynamically-created preview scene, not a static widget - which is why the live capture's button probe never found it), and `TeamSelection_Update` traces the stat bars and the Eliminator/Normal variant pick to code. One function (`Definition_IsUnlocked`) cross-checked exactly against `psp-pulse-eu`; the other fourteen had no exact-hash match there, front-end code diverging more between pressings than the physics code an earlier EU-transfer pass covered. Open: the nine unlock-predicate functions behind `Definition_IsUnlocked` are still untraced; `GSDisableEntriesBitField` is undecoded; the locked-circuit info panel needs `Tournament C` or campaign progression, neither tried; six fields in the serialised race record (`Locked`, `damage`, `AICount`, `laps`, `ship`, `ShipChoice`) have no authored row; and none of the newly-decompiled functions are runtime-verified
