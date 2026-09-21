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
vocabulary at all. That is why `oag_2048::TITLE.front_end` was still `None`
after this pass, on purpose, and not from a lack of evidence.

**Resolved 2026-09-20, on `lane/2048-frontend`:**
[ADR-0054](../../docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md)
made `oag_title::FrontEnd::menu` `Option` and added a `touch` axis for
exactly this vocabulary, so `oag_2048::TITLE.front_end` is `Some` now -
`menu: None` (still correct, still no `MenuSkin` vocabulary to fill it
from), `touch: Some(...)` carrying every number this document names. See
`docs/formats/2048-frontend.md`'s own "Resolved 2026-09-20" section for the
full account, including a second, previously-invisible defect this
unblocked: the in-race HUD font still does not draw, now for a role-name
mismatch (`"HUD"` vs. `2048HUD`) rather than for missing plugins.

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

**Wired and walked 2026-09-21, on `lane/2048-boot`.** `just play 2048` (no
`--race`) boots this front end: the five declared screens, each driven off
its own `<Redirect>` widgets (the card's authored `delay="4.0"`, the six
button exits on the movie and the title screen), then `GameModeChoice`,
`Home` and the campaign map drawn off the disc's own `<TouchButton>`s and
`SP.xml`'s events, pad and pointer alike, and a tap on an event starts its
race through `oag_game::race::load_event`. Two tiles the disc does not
author - `RACEBOX`, `REMIX` - sit after the authored four at the user's
request, marked as this build's own. The full account, with what is
authored, measured and chosen on each screen, is
[2048-frontend.md](../../docs/formats/2048-frontend.md)'s "Wired: the boot
walks and the grids draw" section; the mechanism notes are in
[frontend-boot.md](../../docs/architecture/frontend-boot.md) and
[menus.md](../../docs/architecture/menus.md). Tests:
`crates/game/tests/vita_2048_boot_ground_truth.rs` (eight, on the package)
and `crates/ui/src/frontend/tests/wipeout2048.rs` (seventeen, on a fixture).

**`Home`'s five destinations draw and work, 2026-09-21, on
`lane/2048-home-team`.** `Team_`, `Community_`, `Profile_`, `Options_` and
`Extras_Definition.xml` are all in `oag_2048::frontend::includes::FOLLOWED`
now, and their screens' bare names are registered, closing the "screen this
build does not load" gap every one of `Home`'s tiles hit. Full account in
[2048-frontend.md](../../docs/formats/2048-frontend.md)'s "Wired: `Home`'s
five destinations draw" section: `team` draws a team x craft-slot grid built
from `oag_2048::race`'s own roster (labelled tiles, not the disc's
native-code-populated `teamgrid/` icons) and feeds `settings.race.team`/
`variant`, which the existing `RaceDefaults`/`combine_variant` machinery
already resolves for the campaign launch and the RACE BOX/REMIX pages;
`OptionsCamera`/`OptionsAudio` wire into `settings.graphics.camera_view`/
`audio.music_volume`/`sfx_volume`; `communityAdhocCheck`/`profile`/
`2048extras` (+`manual3D`/`extrasCredits`) draw generically. New mechanism:
a `redirect="PreviousScreen"` back stack, and a fix to a touch-tile bug the
new screens exposed (`GameModeChoice`'s toggle-confirm was matching by
shape, not by screen, and would have mis-fired on `Team`'s own
`replay_unlock` tiles). 14 disc-backed tests in
`crates/game/tests/vita_2048_boot_ground_truth.rs`.

## Open

- **Resolved 2026-09-21: the intro plays, picture and sound.**
  `oag_video::mp4` reads the container ([mp4.md](../../docs/formats/mp4.md)),
  `movie::open` returns a `Movie` for `ftyp`, and the AAC track plays through
  `MovieAudioKind::Container`. The screen paces the 99.57 s picture and fires
  its `AutoRedirect` at the end, or leaves on any of its six buttons.
- **The campaign map's cell-to-pixel mapping is chosen, not measured.** An
  even grid (`oag_ui::frontend::campaign_map::PITCH`/`ORIGIN`) over the
  shell's authored 1920x1088 `<TouchScroll>` canvas, plain `Blue2048`
  markers, a panel naming the selected event. **Narrowed 2026-09-21**: `M_X`/
  `M_Y` *are* real `GameModeBase` fields the executable deserialises
  (`0x2c4`/`0x2c8`, an earlier reading of this pass's own decompile missed
  them - two `FUN_812dab08` registrations Ghidra never auto-stringified),
  but the DLC tiers' own hotspots read a different, still-unfound cached
  position (`+0x15c`/`+0x160`) that something uncaught this pass must derive
  from them - `frontend-campaign-map.md`'s dated section. A live Vita3K
  capture confirms the real map is a hexagonal tile tessellation with a
  season title card, not this square grid or a 3D city
  (`data/reference/2048-frontend/README.md` frames `12`-`14`). Neither
  the real per-event anchor nor the season card's own asset was found - the
  base tier's own `TouchCampaign_Item` class is now named
  (`frontend-campaign-map.md`) but nothing calls its constructor statically,
  the same open item the DLC tier pair already had. The unlock graph is not
  walked: every event is offered, there being no save.
- **Resolved 2026-09-21: `Home`'s five destinations draw.** See the top of
  this file and `2048-frontend.md`'s own "Wired" section for the full
  account. `team`'s own team/craft-slot picker feeds `settings.race.team`/
  `variant`, so `boot::roster::load_teams`'s craft-slot fallback and
  `session::placeholder::raceable_teams`'s stand-in are now only what a
  player gets before ever opening `Home` - not for the rest of the session.
  Still open from this pass: `TouchTeamGrid`'s own `teamgrid/` icon set is
  not wired into the sprite sheet (drawn as labelled tiles instead); the
  `<Model>` ship previews on `team` and `profile` are not drawn (no
  renderer seam from a 2D front-end screen to a 3D mesh); the skin list
  draws and cycles and is not wired to `race::Options::skin` (no measured
  `1`/`2`/`3` -> `PI_ModelSkin` mapping); Options' Controls/Pilot lists draw
  and cycle and reach no setting this build has; the Options picker's
  starting value is this build's own default, not `CameraP1`'s authored one
  (threading the live `Settings` into `Frontend::booting` touches every
  title's boot path for a value only 2048 reads); Extras' ship-unlock and
  season-recap movies are not named by any widget reachable here, so
  nothing plays rather than a guess.
- **The `BootFlowCanvas`'s triangle-grid ground has no located asset**;
  the card and the title screen draw on plain white, labelled chosen.
- **The `TouchHomeButton` and `TouchNews` widgets are not drawn** (no
  authored size on either); the home button's two targets are on circle
  and triangle instead, chosen.
- **Font roles other than `Default` draw in the `Default` face**, scaled by
  their own `.fnt` line heights - right for 2048, whose `NEOSANS_BOLD` and
  `NEOSANS_BOLD_LARGE` are one typeface at two sizes. Only set on a title
  with a touch front end; the other three would need their screens
  re-checked against their captures before it is turned on for them.
- **The `--press` pulse cannot take one screenshot of `TitleScreen` with
  its prompt faded in**: the same cross that reaches it leaves it on the
  next even tick. `--screen TitleScreen` draws it settled instead.

- **Resolved 2026-09-20: `oag_title::MenuSkin` has no shape for a
  touch-icon front end, and now it does not need one.**
  [ADR-0054](../../docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md)
  took the first of the two shapes this bullet named: `FrontEnd::menu` is
  `Option<&'static MenuSkin>`, and a new `oag_title::touch::TouchFrontEnd`
  fills `FrontEnd::touch` instead, off `GameModeChoice`/`Home`/
  `Team_Definition.xml`'s real `x`/`y`/`OriginX`/`OriginY` (re-read from the
  archive for the type, not transcribed from this page's own prose, which
  never quoted individual button coordinates). `FE3DCanvas`'s own
  `CanvasLabel`s stayed out of the type, per the drawn-preview finding below:
  their cluster bounding box is a tool's own measurement over the label set,
  not a number the disc states, so the type carries the screen name and the
  atlas entry instead and lets a caller walk the real label list.
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

0. **Done 2026-09-21**: the boot walks, the grids and the map draw, an
   event races - see the top of this file. What follows is the older list,
   still standing where it says so.

1. ~~Resolve the `MenuSkin` type question above~~ **Done 2026-09-20**, see
   [ADR-0054](../../docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md).
2. Name the vtable at `0x8150ea5c`-ish that owns
   `FE3DCanvas_AddHDCampaignEventButtons`/`FE3DCanvas_AddFuryCampaignEventButtons`
   (both are Thumb-pointer hits in one table, 27 slots apart) - that names the
   class and, with it, whatever screen/refresh path calls them, which is the
   remaining unknown in `frontend-campaign-map.md`. **Still open** - not
   attempted this pass.

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
   to name `Music::front_end`. **Still open** - not attempted this pass.
4. ~~Once `front_end` is fillable, re-run
   `crates/game/tests/vita_2048_hud_ground_truth.rs` and a fresh
   `just play 2048 --race` boot report~~ **Done 2026-09-20**, and the "real,
   free improvement" this step predicted did not fully land: wiring
   `front_end` does make the seventeen language plugins reachable (a
   `--race` capture now logs all seventeen, and
   `vita_2048_hud_ground_truth.rs`'s six tests still pass against real data),
   but the HUD font itself still draws in 5x7 fallback. The remaining cause
   is a role-name mismatch this pass found rather than fixed - see
   `docs/formats/2048-frontend.md`'s "Resolved 2026-09-20" section and
   `docs/formats/2048-status.md`'s HUD bullet for the full mechanism
   (`hud_font` asks for role `"HUD"`/`"HUDSmall"`, 2048 names `2048HUD`, and
   two plugins' leftover `HUD`/`HUDSmall` roles point at files 2048 does not
   ship).
5. ~~New, opened 2026-09-20: give `oag_title` a per-title HUD font role
   name~~ **Done 2026-09-21, on `lane/2048-hud-font`.** `oag_title::HudArt`
   gained `hud_font_role: &'static str` and
   `hud_small_font_role: Option<&'static str>`, mirroring
   `MenuSkin::menu_font`'s "which role names the disc actually uses"
   pattern; `crates/game/src/race/hud.rs::hud_font` now asks each title for
   its own role rather than the literal `"HUD"`/`"HUDSmall"`. 2048 measures
   `"2048HUD"`/`None`. The `HUDSmall` question this item raised was checked
   rather than assumed: all seventeen 2048 plugins name no distinct caption
   role at all - a real gap - so the caption face falls back to the value
   face, **chosen, not measured**, labelled as such in
   `oag_title::HudArt::hud_small_font_role`'s own doc. Verified with
   `just play 2048 --race --screenshot ...`: the boot report resolves
   `Data\XML\2048_hud\font\2048_hud.fnt (role "2048HUD")` and the screenshot
   shows the real face, not 5x7. Pulse and HD re-captured unchanged. Pinned
   by `vita_2048_hud_ground_truth.rs`'s
   `the_resolved_hud_font_is_2048_huds_own_face_not_a_leftover_or_the_fallback`.
   Full account: `docs/formats/2048-frontend.md`'s "Resolved 2026-09-21: the
   HUD font draws" section and `docs/formats/2048-status.md`'s HUD bullet.
