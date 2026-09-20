# 2048's front end is read and not wired

2026-09-17, branch `lane/2048-frontend`. Full write-up in
[2048-frontend.md](../../docs/formats/2048-frontend.md); the reference frames
are under `data/reference/2048-frontend/` (gitignored, README there names
each one) and the capture recipe extends
[vita3k-capture.md](../../docs/reverse-engineering/vita3k-capture.md) with
this pass's own display numbers (weston `wayland-oag95`, Xwayland `:95`).

**The one to know**: 2048 ships one front end, in one archive
(`data/plugins/frontend/NEWGUI/`, base `data.psarc` only - the patch touches
a subset, the DLC touches none), and it is not Wipeout HD's `FEGlobals`/
`MenuSkin` idiom retargeted. It is a genuinely different presentation layer:
touch-icon grids (`GameModeChoice`, `Home`) over a persistent 3D campaign map
(`FE3DCanvas`), no `<FEGlobals>` block anywhere, no `<Menu>`/`<HorizMenu>`
widget anywhere. `oag_title::MenuSkin` was built for the vocabulary the other
three titles share and disagree on numbers within; 2048 does not author that
vocabulary at all. That is why `oag_2048::TITLE.front_end` is still `None`
after this pass, on purpose, and not from a lack of evidence.

The boot chain (`Boot Connect` -> `Boot Studio Logo` -> `Boot Intro Movie` ->
`Load Save Bootup` -> `TitleScreen` -> `GameModeChoice`) is declared in the
XML and was watched once on Vita3K, in that order, matching content exactly
(the Studio Liverpool card's own 4-second delay, the `PRESS ANY BUTTON TO
START` text, the exact EU legal footer). A `LOADING...` screen and an
attract-mode demo race - a real frame of `2048-hud.md`'s own
`DemoRaceManager_Construct` finding - sit between `TitleScreen` and
`GameModeChoice`, undeclared in any `NEWGUI` file.

This pass also found and fixed a wrong claim on two other pages:
`2048-hud.md` and `2048-status.md` both said the HUD's raw `IG_HUD_*`
fallback text was because 2048 names no language plugin or HUD font. It
names both - `english/Definition.xml` declares a `2048HUD` font role at
`Data\XML\2048_hud\font\2048_hud.fnt` - the actual cause is `front_end` being
`None`, which starves `crates/game/src/race/hud.rs`/`race/load.rs`'s
`language_plugins` lookup. Both pages now say so.

## Open

- **`oag_title::MenuSkin` has no shape for a touch-icon front end.** This is
  the real blocker, not an unread disc. Filling `menu_x`/`space`/etc. with
  placeholder numbers to satisfy the type would be inventing a menu shape
  2048 does not draw - exactly what `CLAUDE.md`'s "never invent" section
  forbids. Two shapes for a fix, neither taken here because a type decision
  this size is bigger than a first sweep should make unasked:
  - Make `FrontEnd::menu` `Option<&'static MenuSkin>` and add a second,
    optional axis (`touch_menu` or similar) for the icon-grid/3D-canvas
    idiom, filled from `GameModeChoice`/`Home`/`FE3DCanvas`'s own numbers
    (all read and quoted in `2048-frontend.md`).
  - Or decide the touch idiom is out of `MenuSkin`'s scope entirely and give
    `FrontEnd` a `menu: Option<...>` with 2048 the first `None`, since
    nothing downstream currently draws a touch grid anyway.
  Whichever is chosen, `2048-frontend.md`'s "front end is a touch-icon grid"
  section has every number (`GameModeChoice`'s four `TouchButton`s at
  `x`/`y`/`140x140`, `Home`'s five, `Team_Definition.xml`'s 3D ship-model
  origin) already quoted and ready to fill whichever type lands.
  `FE3DCanvas`'s own `CanvasLabel`s are not part of that list any more - see
  the drawn-preview finding below, which found them clustered in one small
  corner rather than spread across a map.
- **Resolved 2026-09-18: the six-vs-four tiles were never a `GameModeChoice`
  fact.** `open_program` on `/2048/eboot-vita-2048-eu-v104.elf` works
  alongside `/hdfury/EBOOT-ps3-hdfury-eu.elf` staying open for the other lane
  - the bridge holds more than one open program at once, so the
  `analyzeHeadless`-on-a-second-project plan below turned out unnecessary;
  opening the already-imported program was enough. The decompile found
  `FE3DCanvas_AddHDCampaignEventButtons`/`FE3DCanvas_AddFuryCampaignEventButtons`
  (`0x810f817c`/`0x810f5000`, confidence 78): a symmetric pair that adds
  hotspot buttons to the persistent 3D campaign map, not to `GameModeChoice`'s
  own four-button grid, gated on `g_bDlc1Mounted`/`g_bDlc2Mounted`
  (confidence 75) - two globals with exactly one writer,
  `Boot_CheckDlcPackageFlags` (`0x810039fc`, confidence 75), which checks the
  literal add-on IDs `DLC1W2048PACKAGE`/`DLC2W2048PACKAGE` via
  `SceAppUtil_2DB7BE3B`. "Unlocked by DLC being mounted, plausibly" is now
  measured. See
  [frontend-campaign-map.md](../../docs/ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md).
  Still open from that page: no caller found for either tier function (a
  vtable slot is located at `0x8150eae0`/`0x8150ea74`, 27 slots apart, but
  its owning class is not), the per-tier position scale/offset tables are
  undumped, and `g_bDlc3Mounted`/`W2048DLC3PACKAGE` has no known reader
  anywhere in the binary.
- **The rest of the executable sweep is still string-only.** The boot-mode
  selector (`Boot Connect`/`Launch 2048`/`RaceBox`/`Main Menu`/`MPStress`,
  clustered beside a `-mpscreen` command-line flag) is confirmed present as
  literal strings but not yet traced to real addresses of its own -
  `docs/reverse-engineering/toolchain.md#vita` has the import recipe, and the
  program is already open in this project as of the pass above, so this no
  longer needs a fresh `analyzeHeadless` run either.
- **`frontend.bnk` is located, its cues are not.** `data/audio/sound/frontend.bnk`
  exists exactly where `SoundManager_Construct`'s decompiled bank list
  (`game-boot.md`) says it should. `oag_title::Music::front_end` needs a
  *cue* name inside the bank, not the bank's filename, and `oag-wad sounds`
  reads a WAD-hosted bank where this one is PSARC-hosted - extracting it
  first (or teaching the CLI a PSARC-backed path) is the gap.
- **The loading screen is real and unauthored.** A percentage-driven
  `LOADING...` bar over the `WIPEOUT 2048` mark runs on every transition seen
  this pass. No `NEWGUI` file declares it - it reads as engine chrome rather
  than FE data, but that was not confirmed against the executable. If it
  *is* engine-native, `oag_title::Loading` (built for a disc-authored screen)
  is the wrong axis for it entirely, on the same "no shared vocabulary"
  grounds as `MenuSkin` above.
- **Resolved 2026-09-18: `FE3DCanvas` was drawn, and drawing it corrects the
  "~50 hotspots... explorable city" reading.** `cargo run -p oag-tools
  --example campaign_map_preview` (`crates/tools/examples/
  campaign_map_preview.rs`, new) crops `canvasTexture.gxt` at each
  `<CanvasLabel>`'s `(round(u*2048), round(v*2048))` and pastes the result at
  its `x`/`y` on a blank 960x544 canvas - no invented crop size, since
  several labels share identical `u`/`v`/`width`/`height` while linking
  different events (those two fields pick an icon, not a per-label crop
  rect), so the tool trims to real non-background pixels around the anchor
  instead. The anchor reading is now confirmed empirically, not inferred -
  cropping at the `Trophy-2048-*` labels' shared anchor lands exactly on the
  trophy-cup glyph. **What the composite also shows: all 69 of
  `Definition.xml`'s own `FE3DCanvas` labels cluster in `x` 0-196, `y` 0-87 of
  the canvas** - a corner trophy/season-badge widget, not hotspots spread
  across a city map. See `data/reference/2048-frontend/
  11-fe3dcanvas-composite-preview.png` (gitignored, generated not captured)
  and `2048-frontend.md`'s own section for the full write-up. Open: what
  `width`/`height` mean if not crop size, and whether the wider hotspot set a
  real playthrough shows across the map comes from the DLC-gated buttons
  above (plausible, not confirmed) or from data this pass has not found.
- **`CheckPoint_HUD.xml`-style dangling references were not swept for in the
  front end.** `2048-hud.md` already flags one HUD XML the executable wants
  and the base package does not ship; this pass did not repeat that search
  for `NEWGUI` screen names against the patch/DLC manifests beyond the
  front-end-shaped paths already listed.

## Next Steps

1. Resolve the `MenuSkin` type question above (ask, don't guess) and, once
   resolved, fill `FrontEnd::menu` (or its replacement) from
   `2048-frontend.md`'s already-quoted numbers.
2. Name the vtable at `0x8150ea5c`-ish that owns
   `FE3DCanvas_AddHDCampaignEventButtons`/`FE3DCanvas_AddFuryCampaignEventButtons`
   (both are Thumb-pointer hits in one table, 27 slots apart) - that names the
   class and, with it, whatever screen/refresh path calls them, which is the
   remaining unknown in `frontend-campaign-map.md`.

   **2026-09-20: done in part.** The class is named -
   `TouchCampaignFury_Item`/`TouchCampaignHD_Item`, recovered from a literal
   source-file string (`"Frontend/Items/TouchCampaign{Fury,HD}_Item.cpp"`)
   each class's own constructor writes into its new object, confidence 82.
   The screen/refresh path is still not found: neither constructor has a
   resolved static caller, and the working hypothesis is a `.init_array`
   global/static-object constructor rather than a `BL` site - unconfirmed.
   Same pass also resolved two smaller open items on that page: the
   "hash-shaped" tag constants at each button's `piVar13[2]` are actually
   Thumb function pointers to each item's own on-tap handler (not a hash),
   and `SceAppUtil_2DB7BE3B` is `sceAppUtilDrmOpen` per the vitasdk NID
   database. Full detail in `frontend-campaign-map.md`'s own 2026-09-20
   sections.
3. Extract `frontend.bnk` and read its cue table (`oag_formats::sblk::Bank::parse`)
   to name `Music::front_end`.
4. Once `front_end` is fillable, re-run
   `crates/game/tests/vita_2048_hud_ground_truth.rs` and a fresh
   `just play 2048 --race` boot report - the HUD font fix
   (`2048-hud.md`'s correction above) is a real, free improvement that falls
   out of wiring `front_end` at all, independent of the `MenuSkin` question.
