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
  shell's own `<TouchScroll>` canvas, plain `Blue2048`
  markers. **Narrowed 2026-09-21**: `M_X`/
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
  **2026-09-25: the grid's own scale, not the per-event formula, is now
  measured.** The 1920x1088 canvas above was a wrong reading of the shell's
  declared `MaxScrollX="960" maxscrolly="544"` as the whole canvas; frames
  `12`/`13`'s own scrollbar thumbs measure a canvas roughly 5.3x-5.4x the
  view instead (~5207x2873), and `CANVAS`/`PITCH`/`ORIGIN` are rescaled by
  that factor, with `MARKER` set directly off a real hex tile's own
  measured bounding box (122x117 px, frame `13`). See
  `docs/formats/2048-frontend.md`'s campaign-map row for the full pixel
  evidence. A full archive-wide `.xml` grep for every plausible asset name
  (`hex_filled.gxt`, `Hexagon_HD*.gxt`, `Cup2048.gxt`, `TinyCallout_*.gxt`,
  every `medals/`/`trophy/` icon) found none of them referenced by any
  widget - **but referenced by no widget is not the same as absent from the
  archive**, and every name in that list is real. **Resolved 2026-09-27: the
  hex tile art and the four per-kind icons are decoded and drawn.**
  `hex_filled.gxt`/`hex_outline.gxt`/`hex_select.gxt` and `callout/
  {race,speed,zone,combat}_mode.gxt` (all base package) open as real pixels -
  a gloss-filled hexagon, a thin outline, a glow ring, and a chequered flag /
  stopwatch / radar-target / crosshair - confirmed against frames `12`/`13`'s
  own green/grey tiles carrying exactly the flag and stopwatch glyphs.
  `oag_ui::frontend::campaign_map` now draws them, tinted by the same four
  progress colours a flat square always was, gated on `EventKind`;
  `crates/game/tests/vita_2048_boot_ground_truth.rs::
  the_campaign_map_draws_the_discs_own_hex_tiles_not_flat_squares` pins it
  against the real package. Still unmeasured: the per-event `M_X`/`M_Y`
  formula itself (the grid is still an even square, just drawn with hex art
  now), the 3D bevel/drop-shadow the real tile carries, the striped/dotted
  path texture connecting a season's own tiles, the background, the header
  and the season card - all still native-code-driven and needing a Ghidra
  pass, not another data sweep (the sweep this time was pixels, not text,
  and still did not place them). **A dead end worth recording**: correlating
  the 47
  `linkedevent`-carrying `<CanvasLabel>`s' own `x`/`y` against their event's
  `M_X`/`M_Y` looks like a projection at first (a least-squares fit lands
  `y` almost exactly), but the file's own `MPSeason01`-`20` labels - no
  `linkedevent`, no relation to `SP.xml` at all - occupy the identical small
  coordinate range, so the fit is spurious: the whole `<CanvasLabel>` set is
  a small corner widget, not hotspots correlated to the map grid, confirming
  `2048-frontend.md`'s existing reading rather than overturning it.
  **2026-09-25: the invented bottom panel is removed** (it covered ~15% of
  the screen and the real base map shows nothing like it -
  `oag_ui::frontend::campaign_map`'s own "The bottom panel is gone" doc
  section has the full account). A tap on a node in the real game opens a
  separate event card instead (`data/reference/2048-frontend/
  14-campaign-map-event-card-unity-square.png`: photo backdrop, name, kind,
  `PASS`/objective line, lap arrows, pagination dots, three buttons - and it
  opens for a **locked** node too). That card was searched for as an
  authored `NEWGUI` screen this pass and is not one: `<TouchCampaign>`'s
  only child redirects straight to `Launch 2048`, which is nothing but
  `<BackendController task="Launch">` into `InGame2048` in both the base
  package and the `v1.04` patch, no confirm step; the only tick/cross
  confirm dialogs in the archive (`StartEventConfirm`/
  `FriendStartEventConfirm`, `Community_Definition.xml`) are a small
  two-button online-only box, nothing like the card. Native-code-driven and
  unlocated, same as the hex tile art/background/header/season card above -
  not stood in for, per this project's "never invent" rule. A player who
  selects an event today sees only the marker's own cursor ring, no name or
  objective text anywhere on screen; `MapEvent::detail`/`selected_event()`
  still carry the real text for whenever the card is recovered.
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
- **Resolved 2026-09-25: `frontend.bnk` is located, and its cues turned out
  not to exist rather than merely to be unread.** `oag_formats::sblk::Bank::sound_names()`
  on the extracted bank is empty (`HAS_NAME_TABLE` clear), so nothing in it
  can be addressed by name. `oag_title::Music::front_end` is filled from a
  standalone file instead: `data/audio/music/FEMusic/frontend_stereo.at9`,
  RIFF-wrapped ATRAC9 - the same `FEMusic`/`frontend` shape Pulse's and
  HD's own front-end tracks use, confidence 70 (unambiguous by naming and
  placement, not confirmed against a decompile). New `oag_game::at9`
  decoder; `MusicDiscs::survey`/`pick` also needed `Platform::Vita` added,
  which it previously folded into the same arm as `Unknown`. Full account
  in `docs/formats/2048-frontend.md`'s "frontend.bnk" bullet; pinned by
  `crates/game/tests/vita_2048_music_ground_truth.rs`.
- **Resolved 2026-10-05: the race soundtrack plays.** Eleven `PI_Music`
  tracks (`music_stereo.at9`) go through the declared route. Still open: the
  original's order, shuffle and selection rule (ours is sequential from 01,
  chosen, not measured), and whether the `.fft` sidecars drive anything.
  See `docs/formats/2048-status.md`.
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
3. ~~Extract `frontend.bnk` and read its cue table
   (`oag_formats::sblk::Bank::parse`) to name `Music::front_end`.~~ **Done
   2026-09-25**: the bank has no name table at all, so `front_end` is
   instead a standalone `FEMusic/frontend_stereo.at9`, the same naming
   shape Pulse's and HD's tracks use. See the top of this file and
   `docs/formats/2048-frontend.md`'s "frontend.bnk" bullet.
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
   rather than assumed, at two levels: all seventeen 2048 plugins name no
   distinct caption role at all, and composing every root the played skin
   carries finds zero `font="HUDSmall"` widgets in any of them - every label
   is `font="HUD"`, and the visible caption/value size split is `scale`
   alone (`LapTxt` at 0.6 beside `Laps` at 1.0). So the caption face falling
   back to the value face is **measured, not merely chosen** - labelled as
   such in `oag_title::HudArt::hud_small_font_role`'s own doc. Verified with
   `just play 2048 --race --screenshot ...`: the boot report resolves
   `Data\XML\2048_hud\font\2048_hud.fnt (role "2048HUD")` and the screenshot
   shows the real face, not 5x7. Pulse and HD re-captured unchanged. Pinned
   by `vita_2048_hud_ground_truth.rs`'s
   `the_resolved_hud_font_is_2048_huds_own_face_not_a_leftover_or_the_fallback`
   and `the_played_skins_layouts_author_no_hudsmall_widget_at_all`.
   Full account: `docs/formats/2048-frontend.md`'s "Resolved 2026-09-21: the
   HUD font draws" section and `docs/formats/2048-status.md`'s HUD bullet.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-17, updated 2026-09-21. **The boot walks it now**: `just play 2048` runs the five declared screens off their own redirects, draws `GameModeChoice`/`Home`/the campaign map off the disc's own widgets and `SP.xml` (pad and pointer), and a tap on an event races it through `race::load_event`; two tiles the disc does not author (`RACEBOX`, `REMIX`) sit after the authored four as this build's own. Earlier: [ADR-0054](../../docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md) made `FrontEnd::menu` `Option` and added the `touch` axis (`menu: None`, `touch: Some`); the HUD's `IG_HUD_*` fallback is a role-name mismatch (`2048HUD` vs `"HUD"`), not a missing plugin; the six-vs-four tiles are DLC-gated `FE3DCanvas` hotspots; the 69 `CanvasLabel`s cluster in one corner. Open: the MP4 intro's length and picture (`lane/2048-intro-mp4`), the map's chosen cell-to-pixel grid (2026-09-21: `M_X`/`M_Y` are real fields after all, but the copy/scale into the DLC tiers' own cached position is unfound, and live capture shows a hex tessellation regardless), the `BootFlowCanvas` art, `team`'s own icon grid and ship preview, the campaign-event functions' caller, `frontend.bnk`'s cues, the per-title HUD-font-role axis
