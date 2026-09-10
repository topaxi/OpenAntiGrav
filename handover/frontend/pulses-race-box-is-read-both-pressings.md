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
`oag_ui::picker` (model, layout read off `Selection_Definition.xml`, draw),
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
walking back down. Read by `oag_ui::picker::slideshow`, loaded through the
front end's sheet extended per selection, drawn on both pressings. On the
way: the hex grid is a 32x16 tile the sheet could not repeat
(`Draw::TiledSprite`), the PS2's `Team Selection` widgets carry a `0`
player suffix, the PS2 previews needed `race::ps2_texture_set`, and the
RACE page's TEAM/VARIANT/TRACK rows are dropped on a title with these
screens (`Definition::drop_rows_picked_on_screen`). Docs:
[`selection-screens.md`](../../docs/ui/selection-screens.md),
[`race-box-screens.md`](../../docs/ghidra/functions/psp-pulse-usa/race-box-screens.md).

## Open

- **The cards arrive without their transition, and the outline is framed
  off the capture rather than off `screen.xml`'s own `Mode3D` pose** -
  both read (`slideshow::Model`, the `transition` attributes), neither
  acted on. The pose needs the `Mode3D` projection understood
  (`OriginX/OriginY` are in an unmeasured space: `-145,13` on the PSP,
  `193,-21` on the PS2).
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
3. Walk the PS2's race box on PCSX2 the way `scripts/psp-frontend-capture.py`
   walks the PSP's, and read the two screens' PS2 captures against
   `--menu-page track-select`/`ship-select` on `pulse-ps2-eu.chd`.
4. HD's equivalent screen-population code is still unfound - see
   `docs/ghidra/functions/ps3-hdfury-eu/track-selection-screen.md` - and
   solving it there would let the two titles' readings corroborate each
   other the way `race-box-screens.md`'s method could be reused for.
