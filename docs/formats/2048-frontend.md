# Wipeout 2048: the front end, as its own package states it

**This page is a reading plus one real capture, not a full measurement.**
`docs/formats/2048-status.md` has said since the title's first pass that
`oag_2048::TITLE` carries `front_end: None`, `loading: None` and `music: None`,
and that "neither has been read." That was true. This page is the same sweep
[hd-frontend.md](hd-frontend.md) ran for Wipeout HD, applied to 2048's Vita
package: every front-end-shaped entry in `PSP2/data.psarc` (plus the v1.04
patch and both DLC packages) read through this project's own PSARC reader,
the boot chain read out of the front-end XML and then watched on Vita3K, and
a read-only sweep of `eboot.elf` for corroboration. Where a claim rests on the
disc's own XML it is capped at 94 per the
[confidence rubric](../reverse-engineering/confidence-rubric.md); where it
rests on the Vita3K capture it is a runtime trace, scored the way
`hd-frontend.md`'s own RPCS3 captures are.

The package is `data/extracted/vita/PCSF00007` (EU, decrypted, base plus
`patch-v104`/`dlc1`/`dlc2`), the same one `docs/formats/2048-status.md` and
`docs/formats/2048-hud.md` read. Everything below is read through
`oag_assets::psarc::Archive` - `cargo run -p oag-assets --example psarc_list`/
`psarc_cat`, the same reader `oag-unpack`/`oag-wad` use for a disc-hosted
archive, pointed at a loose `.psarc` file instead.

## The headline

| Question | Answer | Confidence |
| --- | --- | --- |
| Where is the front end? | `data/plugins/frontend/NEWGUI/`, base package only - **one copy**, no lineage question at all | 92 |
| Is it Wipeout HD's `skin.xml`/`FEGlobals` idiom? | **No.** No `<FEGlobals>` block anywhere; the whole vertical/horizontal `MenuSkin` vocabulary is absent | 90 |
| What replaced it? | A touch-icon grid (`GameModeChoice`) over a persistent 3D scene (`FE3DCanvas`) - Vita-native, not a ported PSP/PS3 menu | 90 |
| Coordinate space | **960x544** - `setResolution="544"` on the two root screens, the same space `2048-hud.md` measured for the HUD | 92 |
| Declared boot chain | Five screens, quoted below | 85 |
| Runtime boot order | **The same order**, one cold Vita3K launch, 2026-09-17 | 80 |
| Do language plugins carry a HUD font role? | **Yes.** `english/Definition.xml` names `2048HUD` -> `Data\XML\2048_hud\font\2048_hud.fnt`, contradicting a claim on `2048-hud.md`/`2048-status.md` this page corrects | 90 |
| Does the in-race HUD draw in that font? | **Yes, since 2026-09-21** - `oag_title::HudArt` gained a per-title `hud_font_role`/`hud_small_font_role` axis so `hud_font` asks each title for the role its own plugins name instead of the literal `"HUD"`/`"HUDSmall"` every other title happens to share; see [below](#resolved-2026-09-21-the-hud-font-draws) | 90 |
| Do HD's own language-plugin bugs reproduce here? | **Yes, byte for byte** - the `Svenska` mixups, the mangled Russian name, and the still-labelled `Wipeout Pulse` internal id | 90 |
| What plays the boot movie? | `data/Videos/intro.mp4` - a real MP4/ISOBMFF container, not `.bik`/`.pmf`/`.ipf` | 92 |
| Does `just play 2048` walk it? | **Yes, since 2026-09-21** - the five declared screens, then `GameModeChoice`, `Home` and the campaign map, drawn off the disc's own widgets; see [Wired: the boot walks and the grids draw](#wired-the-boot-walks-and-the-grids-draw-2026-09-21) | - |
| Is `front_end` wired? | **Yes, since 2026-09-20** - [ADR-0054](../architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md) widened `oag_title::FrontEnd::menu` to `Option` and added a `touch` axis for exactly this vocabulary. `menu` itself stays `None`, correctly - see [below](#why-front_end-stays-none), now describing why one field of `FrontEnd` is `None` rather than why the whole struct was. | - |

## Reading it yourself

```sh
cargo run -q -p oag-assets --example psarc_list -- \
    data/extracted/vita/PCSF00007/base/PSP2/data.psarc frontend
cargo run -q -p oag-assets --example psarc_cat -- \
    data/extracted/vita/PCSF00007/base/PSP2/data.psarc \
    "data/plugins/frontend/NEWGUI/Skin.xml"
```

`fd`/`rg` respect `.gitignore` and return nothing under `data/`; use
`/bin/ls`, `find` or `grep`, or pass `--no-ignore`.

## Inventory: one front end, in one archive

Every front-end-looking path (`frontend`, `fe`, `gui`, `menu`, `movies`,
`loading`, `music`, `fonts`) across all five archives (`data.psarc`, the
patch's `data1.psarc`/`data2.psarc`, `dlc1.psarc`, `dlc2.psarc`), all of which
parse cleanly (18,430 / 854 / 1,526 / 1,903 / 1,721 entries):

| Archive | Front-end entries | What is there |
| --- | ---: | --- |
| `data.psarc` (base) | 25 XML documents under `NEWGUI/`, plus fonts/images/movies | The whole front end. Ships once. |
| `patch-v104/data1.psarc` | 23 | Newer copies of a subset - `Definition.xml`, `Team_Definition.xml`, four `Intro_Definition_<region>[_Demo].xml` variants not in the base at all, four `Legal_Line_Definition_<region>_Demo.xml`. **No `Skin.xml`** - the patch never touches the root screen or the boot chain. |
| `patch-v104/data2.psarc` | 9 | A second, smaller set of the same files (`Extras_Definition_*`, `Profile_Definition.xml`, `Unlocks_Definition.xml`, the two `LegacyEndRace`/`LegacyInGame` files). Also no `Skin.xml`. |
| `dlc1.psarc` | 0 | Track/environment content only. |
| `dlc2.psarc` | 0 | Same. |

**This is the opposite of Wipeout HD's lineage problem.** HD ships `skin.xml`
six times across seven archives and the choice of copy was itself a
measurement; 2048 ships it **exactly once**, in the base package, and neither
patch nor DLC carries a second copy to disagree with it. There is nothing to
resolve here - reading the base copy *is* reading the live one.

`NEWGUI`'s 25 files, by screen count (`<Screen` occurrences, the same census
`hd-frontend.md` runs):

| File | Screens | File | Screens |
| --- | ---: | --- | ---: |
| `InGame_Definition.xml` | 36 | `Community_Definition.xml` | 7 |
| `Demo_Definition.xml` | 20 | `Unlocks_Definition.xml` | 6 |
| `Definition.xml` (root) | 13 | `TutorialMovies_Definition.xml` | 6 |
| `EndRace_Definition.xml` | 11 | `Extras_Definition_<region>.xml` (x4) | 6 each |
| `Options_Definition.xml` | 8 | `LegacyEndRace_Definition.xml` | 5 |
| `InGame_PauseOptions.xml` | 8 | `Profile_Definition.xml` | 4 |
|  |  | `Intro_Definition.xml` | 4 |
|  |  | `Team_Definition.xml` | 2 |
|  |  | `Skin.xml` | 2 |
|  |  | `LegacyInGame_Definition.xml` | 2 |
|  |  | `Bootup_Definition_<region>.xml` (x4) | 1 each |
|  |  | `Legal_Line_Definition_<region>.xml` (x4) | 0 (one bare `<Text>`, no `<Screen>` wrapper) |

25 files against HD's ~30-file, ~200-screen front end. 2048's is a much
smaller tree - not because less survives, but because a touch UI needs far
fewer discrete screens than a list-menu one built from nested `<Menu>`
widgets and per-page redirects.

**Resolved 2026-10-06: `EndRace_Definition.xml` is read and two of its pages
draw.** Its `EndRace` tree (`RaceSummary`, `ObjectiveSummary`, `Results`, `Podium`,
`Badges`) is what a finished race walks, in the touch idiom this document
describes, not HD's `EndRace Results`/`EndRace Menu` (those are
`LegacyEndRace_Definition.xml`, not reached by the chain). The patch's
`data1.psarc` copy differs from the base by one attribute. See
[`docs/ui/endrace-2048.md`](../ui/endrace-2048.md) and the decompiled chain in
[`vita-2048-eu-v104/endrace-summary.md`](../ghidra/functions/vita-2048-eu-v104/endrace-summary.md).
**Omega:** ships the same file byte-for-byte as `vita/vita_EndRace_Definition.xml`
(`checked, applies, not wired`).

## The front end is a touch-icon grid, not `FEGlobals`/`MenuSkin`

**Confidence 90.** `data/plugins/frontend/NEWGUI/Skin.xml`, quoted in full
except the colour table (below):

```xml
<Screen>
	<Variable global="Blue2048"><Values String="0xff19295d"></Values></Variable>
	<Variable global="Orange2048"><Values String="0xffdd580b"></Values></Variable>
	<Variable global="White2048"><Values String="0xffe1e4eb"></Values></Variable>
	<Variable global="Transparent2048"><Values String="0xc0ffffff"></Values></Variable>
	<Variable global="Grey2048"><Values String="0xff717b96"></Values></Variable>
	<Variable global="Pass2048"><Values String="0xff018400"></Values></Variable>
	<Variable global="ElitePass2048"><Values String="0xfffef502"></Values></Variable>
	<Variable global="HardcorePass2048"><Values String="0xff602c90"></Values></Variable>

	<Screen name="HD_Colours">
		<!-- HD_Blue/HD_Grey/HD_LightGrey/HD_White/HD_BG/HD_transBG, inherited verbatim -->
	</Screen>

	<LoadXML><Values SrcRel="Definition.xml"></Values></LoadXML>
	<LoadXML><Values SrcRel="InGame_Definition.xml"></Values></LoadXML>
	<LoadXML><Values SrcRel="LegacyInGame_Definition.xml"></Values></LoadXML>
	<LoadXML><Values SrcRel="EndRace_Definition.xml"></Values></LoadXML>
	<LoadXML><Values SrcRel="LegacyEndRace_Definition.xml"></Values></LoadXML>
	<LoadXML><Values SrcRel="Demo_Definition.xml"></Values></LoadXML>
</Screen>
```

**There is no `<FEGlobals>` block.** Not a smaller one, not a renamed one -
grepped across all 25 `NEWGUI` files plus the patch's 32: zero hits for
`FEGlobals`, `MenuXOffset`, `MenuScale`, `TitleXOffset`, `TitleYOffset`,
`TitleScale`, `TextColor` or `TitleColor`. HD's `HD_Colours` block is
inherited verbatim (six identical globals, same names, same nesting), which
is more lineage evidence for "2048 is HD's codebase retargeted"
([game-boot.md](../ghidra/functions/vita-2048-eu-v104/game-boot.md#gameroot_construct---0x81000032)) -
but the *menu* globals that block exists beside on HD are gone, not
renamed and not zero-filled.

What replaced them, read off `NEWGUI/Definition.xml`'s root screen
(`overshell` -> `newFEshell`):

- **`GameModeChoice`** (`type="GameModeChoice_Screen"`) is four
  `<TouchButton>`s in a row (`offline`/`multiplayer`/`adhoc`/`crossplay`),
  each a `width="140" height="140"` icon at a literal `x`/`y` - not a `<Menu>`
  row list and not a `<HorizMenu>` strip. Confirmed by the Vita3K capture to
  be **six** tiles at runtime (below) - two more than this file declares.
- **`Home`** (`type="Home"`) is the same shape: five icons in a row
  (Team/Community/Profile/Options/Extras), each its own `<TouchButton>`.
- **The persistent background is `<FE3DCanvas>`** inside `newFEshell`'s own
  `<TouchScroll>`, with ~50 `<CanvasLabel>` hotspots (`Trophy-2048-2-3`,
  `ShipUnlock-2049-3-4`, `SpeedRating_Top_C`, ...), each carrying a
  `linkedevent` naming a specific race event. **Corrected 2026-09-18: "a real
  3D scene" was read from the class name and a screen capture, not from
  authored data, and the disc's own XML does not support it.** No
  `<CanvasLabel>` and no `<FE3DCanvas>` tag ever carries a `<Model>`, a
  camera, or any other 3D-scene attribute - each label is a flat pair of
  rects, `x`/`y` (a pixel position in the 960x544 screen) **and** `u`/`v`
  (a 0.0-1.0 coordinate into a texture), e.g. `<Values x="57" y="40" u="0.0"
  v="0.4384765625" width="2" height="1">`. That is the shape of a 2D sprite
  atlas lookup, not a 3D projection. The atlas itself is real and decodes
  today: `data/FE/NewImages/canvasTexture.gxt` (2048x2048, PVRTC-II, base
  package) is a sheet of exactly the kind of thing a `CanvasLabel` would
  pick out - `A·G·R·C 2048`/`2049`/`2050` season badges, event numbers
  `01`-`20` in two column groups, a trophy glyph, and three tiers of
  rank-circle badges - decoded via `cargo run -p oag-texture --example
  gxt_to_png -- canvasTexture.gxt`, the same PVRTC-II path
  [`frontend-campaign-map.md`](../ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md)
  found the executable loading a DLC-specific second copy of
  (`canvasTextureHD.gxt`, in `patch-v104/data2.psarc`, not the base
  package - a matching 2048x2048 PVRTC-II sheet of HD/Fury circuit wordmarks
  and bronze/silver/gold medal hexagons, feeding exactly the
  `FE3DCanvas_Add{HD,Fury}CampaignEventButtons` widgets that page
  documents). Whether `FE3DCanvas` *also* renders a real 3D backdrop model
  underneath - hardcoded in the executable rather than XML-declared, since
  no background-map asset with an obvious name turned up in either package
  - is unmeasured; what is measured is that its icon/label layer is a
  conventional 2D atlas, already decodable, not a blocker for drawing it.
  `TouchCampaign`'s own redirect target is `Launch 2048`, one of the
  boot-mode strings the executable also knows by name (below).

  **2026-09-18: drawn, and the draw corrects the "~50 hotspots... across the
  map" framing above.** `cargo run -p oag-tools --example
  campaign_map_preview -- <base data.psarc> out.png` (`crates/tools/examples/
  campaign_map_preview.rs`) reads every `<CanvasLabel>` under `Definition.xml`'s
  `<FE3DCanvas>`, crops `canvasTexture.gxt` at `(round(u*2048), round(v*2048))`
  and trims to the surrounding non-background pixels (no invented crop size -
  see the example's own doc comment for why `width`/`height` could not be
  trusted: several labels share identical `u`/`v`/`width`/`height` while
  linking different events, so those two fields pick *which* icon, not how
  big to crop it), then pastes the result at the label's `x`/`y`. The anchor
  reading is now empirically confirmed, not inferred: cropping at the
  `Trophy-2048-2-3`/`Trophy-2048-3-3`/`Trophy-2048-5-3` labels' shared anchor
  lands exactly on the trophy-cup glyph, and the composite (`data/reference/
  2048-frontend/11-fe3dcanvas-composite-preview.png`, gitignored - generated,
  not captured, see that directory's `README.md`) shows real digits, medal
  rims and logo fragments in place, not noise. **What it also shows: all 69
  of this file's `<CanvasLabel>`s cluster in `x` 0-196, `y` 0-87 of the
  960x544 canvas** - one small corner (a trophy-shelf/season-badge widget),
  not hotspots spread across a city map. The "explorable city" reading one
  level up was extrapolated from the class name and a screen capture that
  never isolated this block; the wider hotspot set a real playthrough shows
  spread across the map is most plausibly the DLC-gated buttons
  `frontend-campaign-map.md` already found being added by native code from a
  node array, not from this XML - unconfirmed, but consistent with both
  findings landing the same day.
- **Team selection is 3D too.** `Team_Definition.xml`'s `team` screen positions
  a `<Model name="ShipModel">` at `OriginX="1400" OriginY="264"` in the same
  960x544 space and lays touch buttons (`teamgrid_touch`, skin picker,
  AR-view button) around it - no `<Menu>` widget here either.

**What this means for the type this project already has**:
`oag_title::MenuSkin` was built to describe a `FEGlobals`-driven vertical
list or a `<HorizMenu>` strip - the vocabulary all three prior titles share,
even though they disagree on its numbers (that disagreement is exactly what
made `MenuSkin` an axis in the first place, per its own module doc). 2048
does not disagree on `MenuSkin`'s numbers; it does not author the *vocabulary*
`MenuSkin` reads at all. See [why `front_end` stays `None`](#why-front_end-stays-none).

## The declared boot chain

**Confidence 85** - read screen-by-screen out of `NEWGUI/Bootup_Definition_EU.xml`
and `NEWGUI/Intro_Definition.xml`, the same way `hd-frontend.md` reads HD's.
2048 ships the bootup screen **once per SIE territory** (`_AS`/`_EU`/`_JP`/`_US`
suffix on the file name, loaded through `Definition.xml`'s own
`<LoadXML SrcRel="Bootup_Definition.xml" localised="true">` - a *region*
switch, not a *language* one; the 17 language plugins are a separate axis
entirely, see below), and all four are structurally identical except the
region-specific legal text they load.

| # | Screen | Leaves by | To |
| --- | --- | --- | --- |
| 1 | `Boot Connect` | `<UnityConBasic>` `MoveTo` (`success` **and** `failure` both go the same place) | `Boot Studio Logo` |
| 2 | `Boot Studio Logo` | `<Redirect delay="4.0">` auto, or `cross` | `Boot Intro Movie` |
| 3 | `Boot Intro Movie` | `<Redirect StartEnabled="false">` auto on end, or any of six buttons | `Load Save Bootup` |
| 4 | `Load Save Bootup` (`type="HDDBoot"`) | `<Redirect StartEnabled="false">`, auto once the save check completes | `TitleScreen` |
| 5 | `TitleScreen` | any of six buttons | `GameModeChoice` |

Quoted, the two steps that carry the only movie and the only unconditional
network check:

```xml
<Screen name="Boot Connect">
  <UnityConBasic>
    <Connection checkNP="true" allowFailure="true" lan="false"></Connection>
    <MoveTo success="Boot Studio Logo" failure="Boot Studio Logo"></MoveTo>
  </UnityConBasic>
</Screen>

<Screen name="Boot Intro Movie">
  <Movie name="LogoMovie" transition="0">
    <Values X="0" Y="0" Width="960" height="544" src="data/Videos/intro.mp4"
            ignoreSafeZoneFullscreen="true" repeat="false" preload="true"></Values>
  </Movie>
  <Redirect name="AutoRedirect" StartEnabled="false">
    <Default goto="Load Save Bootup"></Default>
  </Redirect>
  <!-- six more redirects, one per button, all to the same target -->
</Screen>
```

`Boot Connect`'s `MoveTo` going to the same screen on success and failure is
the same shape HD's `PreFMVConnect` takes - a network check the boot chain
passes through unconditionally within whatever `allowFailure="true"` grants,
not a fork. `TitleScreen` carries its own `<AttractModeController
timeout="60">` and the legal footer via a fourth, region-suffixed
`LoadXML` (`Legal_Line_Definition_EU.xml`, one bare `<Text>` each).

### Corroborated on Vita3K, 2026-09-17

**Confidence 80** - one cold process launch of `vita3k -r PCSF00007`,
screenshotted every 0.75s from just after the window appeared. Captures under
`data/reference/2048-frontend/` (gitignored; see that directory's own
`README.md` for the full frame-by-frame account). Four of the five declared
screens were caught, in the declared order, with content matching the XML
exactly:

- **`Boot Studio Logo`**: five identical 0.75s samples (~3.75s) of the Studio
  Liverpool card - matches the XML's own `delay="4.0"` almost to the second.
- **`Boot Intro Movie`**: `data/Videos/intro.mp4` playing, confirmed by
  frame-to-frame content changing across ~90s of samples (a leaf on wet
  tarmac, an aerial flyover, a Feisar chassis schematic reel) - this is a
  ship-history short film, not a `.bik`/`.pmf` logo reel.
- **`TitleScreen`**: `WIPEOUT 2048` mark, `PRESS ANY BUTTON TO START` (the
  `BOOT_PRESS_ANY` idstring), and the footer `©2012 Sony Computer
  Entertainment Europe. Published by Sony Computer Entertainment Europe.` -
  an exact match for `Legal_Line_Definition_EU.xml`'s
  `BOOT_LEGAL_TRADEMARK_FULL_EU` string.
- **`GameModeChoice`**: reached, eventually - see the next section for what
  sits between them.

`Boot Connect` and `Load Save Bootup` were not caught as distinct frames
(both plausibly sub-0.75s: the connection check has `allowFailure`, and this
profile already had a save from the 2026-09-16 `2048-hud` capture, so
`Load Save Bootup`'s "new profile" text path never ran). Neither gap
contradicts the declared order - every frame that *was* caught landed exactly
where the XML says it should, which is what the confidence above is scored
against.

**One thing the capture found that the XML does not declare.** Pressing a
button on `TitleScreen` did not go straight to `GameModeChoice`. It went
through a real percentage-driven `LOADING...` screen (present in no `NEWGUI`
file - engine chrome, not FE data) into an **attract-mode demo race**: a
chase-cam shot through a lit anti-gravity circuit, no HUD, cycling through at
least three different circuits over the samples taken. A tap during it
returned to `TitleScreen`, not to the mode grid. This is a live frame of
`docs/formats/2048-hud.md`'s own `DemoRaceManager_Construct` finding (a
race-manager constructor found in the executable that reads the *bare-root*
HUD skin set, "the attract-mode demo, not a race a player starts") -
previously read from decompiled code alone, now seen running. A second
attempt (tap `TitleScreen` again) reached `GameModeChoice` directly. Not
chased further: whether the demo fires unconditionally once per boot or on
some other trigger is unmeasured.

## The Game Mode grid has six tiles; the file declares four - and the two extra are not `GameModeChoice` tiles at all

> **Superseded in part, 2026-10-07 (`title-patches` lane), confidence 90.** The
> four-button reading was of the **base** package's `Definition.xml`. The v1.04
> patch's copy (`data1`/`data2`) authors **six** `GameModeChoice` `<TouchButton>`s
> itself - `FE_SP_CAMPAIGN`, `FE_RC_HD`, `FE_RC_FURY` at `x` 228/410/592, `y` 110,
> then `FE_MP_CAMPAIGN`, `FE_ADHOC`, `FE_CROSSPLAY` at `y` 320, 135x135 - which is
> the Vita3K capture's grid exactly. With the patch mounted this build draws it. The hotspot mechanism below is a
> separate, DLC-gated map object and is not retracted; whether the original
> hides these two tiles without the DLC is not read. Choosing HD or Fury leaves
> a note ("another title's campaign") and stays. This build's RACEBOX and REMIX
> tiles moved to the left column, where the second row would have covered them.

**Confidence 85 for the runtime observation; confidence 78 for the mechanism,
decompiled 2026-09-18.** `NEWGUI/Definition.xml`'s `GameModeChoice` screen
authors exactly four `<TouchButton>`s - `offline`/`multiplayer`/`adhoc`/
`crossplay` (`FE_SP_CAMPAIGN`/`FE_MP_CAMPAIGN`/`FE_ADHOC`/`FE_CROSSPLAY`). The
Vita3K capture (`08-game-mode-grid-clean.png`) shows **six**: top row `SINGLE
PLAYER CAMPAIGN`, `HD CAMPAIGN`, `FURY CAMPAIGN` (all enabled); bottom row
`ONLINE CAMPAIGN`, `ADHOC`, `CROSS-PLAY` (all network-gated, pale).

**`HD CAMPAIGN`/`FURY CAMPAIGN` are not `GameModeChoice` tiles.** A decompile
of `eboot.elf` (below) found
[`FE3DCanvas_AddHDCampaignEventButtons`/`FE3DCanvas_AddFuryCampaignEventButtons`](../ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md),
a symmetric pair that adds hotspot buttons to the persistent 3D campaign map
(`FE3DCanvas`), gated on `g_bDlc1Mounted`/`g_bDlc2Mounted` - two globals with
exactly one writer, `Boot_CheckDlcPackageFlags`, which checks for the literal
add-on content IDs `DLC1W2048PACKAGE`/`DLC2W2048PACKAGE` via
`SceAppUtil_2DB7BE3B`. "Once it detects `dlc1.psarc`/`dlc2.psarc` are
mounted" is no longer a plausible guess - it is what the two gating globals'
only writer does. What the earlier pass could not tell apart from a screen
capture alone - a `GameModeChoice` grid item versus a differently-styled
hotspot on the map behind it - the decompile resolves: they share the
touch-icon visual language but are a different object on a different screen,
built by a caller this pass did not resolve (see the evidence page's Open
section).

One sub-page deep, `ADHOCTutorial`'s own text (`Tut_AH_FE_Overlay`) matched
the capture word for word: *"Welcome to Ad-Hoc Multiplayer. Here you can play
Multiplayer games against players in the vicinity, without going online."*

## The language plugins carry a HUD font role too

**Confidence 90 - this corrects a claim on two other pages.**
`docs/formats/2048-hud.md` and `docs/formats/2048-status.md` both said the
HUD draws raw `IG_HUD_*` ids in the 5x7 fallback "because this title names no
language plugin or HUD font." That was a misreading of the *symptom* as the
*cause*. `oag_2048::TITLE.front_end` is `None`, so
`craft_title.front_end.map_or::<&[&str], _>(&[], |fe| fe.language_plugins)`
(`crates/raceplay/src/hud.rs`, `crates/raceplay/src/load.rs`) always gets
an empty slice and never loads a font - **regardless of what the disc
authors**. The disc authors plenty:

```xml
<!-- data/plugins/languages/english/Definition.xml -->
<Font><Values name="Default" Language="English" Src="Data\FE\Fonts\NEOSANS_BOLD_LARGE.fnt"></Values></Font>
<Font><Values name="NEOSANS_LARGE" Language="English" Src="Data\FE\Fonts\NEOSANS_LARGE.fnt"></Values></Font>
<Font><Values name="2048HUD" Language="English" Src="Data\XML\2048_hud\font\2048_hud.fnt"></Values></Font>
<Font><Values name="NEOSANS_BOLD" Language="English" Src="Data\FE\Fonts\NEOSANS_BOLD.fnt" borderExtendPixels="15"></Values></Font>
<Font><Values name="NEOSANS" Language="English" Src="Data\FE\Fonts\NEOSANS.fnt" borderExtendPixels="15"></Values></Font>
```

A `2048HUD` role naming `Data\XML\2048_hud\font\2048_hud.fnt` directly - the
same `.fnt`/`.gxt` pair `2048-hud.md` already lists under "four skin sets,
and only one is played" without connecting it to a font role. So the font
this title's HUD wants is located and named; the gap is entirely that nothing
supplies `front_end.language_plugins` for the loader to walk. This is real
motivation to close the `MenuSkin` gap below, since it fixes more than menus -
but it is not fixed by this pass, for the same reason `front_end` stays
`None` throughout.

Front-end fonts, all under `Data\FE\Fonts\`, each an `.fnt`/`.gxt` pair:
`NEOSANS`, `NEOSANS_BOLD`, `NEOSANS_LARGE`, `NEOSANS_BOLD_LARGE` (the default
role), plus dedicated `Jap*`/`*RUSSIAN*` faces for the two scripts Latin glyphs
cannot cover - unread past their names this pass.

### 2048 reproduces every one of HD's known language-plugin bugs

**Confidence 90 - not read by analogy, read off 2048's own 17 plugins.**
Every finding `hd-frontend.md` records about HD's sixteen language plugins
reproduces here, several byte for byte:

- **`Wipeout Pulse` is still the internal game-name string**, in every
  plugin's `<StringTable>` - `<Entry Language="English" ID="Wipeout Pulse"
  String="Wipeout Pulse"></Entry>` on English, the same shape on all 17.
  HD's own copy of this string is what `hd-frontend.md` calls "HD's own
  language plugin still identifies the game as Wipeout Pulse"; 2048 carries
  it forward a second time.
- **Japanese, Korean and TraditionalChinese all report their native name as
  `Svenska`** - the identical three languages HD gets wrong, the identical
  wrong word. (Swedish also reads `Svenska` here, which is simply correct for
  Swedish - not a fourth instance of the bug, just adjacent to it in the same
  copy-paste family.)
- **Russian's native name is mangled to `P??????`** - character-for-character
  the same garbled string HD ships (literal `0x3f` bytes in the file, not an
  encoding fault). **Ported 2026-10-09**: the loader reads `OPT_RUSSIAN`
  (`Русский`) from the plugin's own table, and the picker's face
  (`NEOSANS_BOLD_LARGE.fnt`, no Cyrillic) then leaves Russian out. Also seen
  here, **open and not changed**: the `Polish` plugin, which 2048 EU offers,
  declares itself as `Español`, so the picker shows that word twice; its own
  table has `OPT_POLISH` = `Polish`, no better.
- **Portuguese's own name fails to round-trip**, though not identically to
  HD's failure mode: HD's `portuguese/definition.xml` keeps the raw Latin-1
  byte (`Portugu\xeas`) inside a file declared `encoding="utf-8"`; 2048's copy
  has **no byte at all** where "ê" belongs (`Portugus`, 8 characters, not 9) -
  the same root cause, a Latin-1 "ê" in a UTF-8-declared file, but this
  build's own packaging step dropped the invalid byte outright rather than
  carrying it through. Confirmed at the byte level (`xxd`), not inferred from
  the rendered string.

Every one of `english/`...`traditionalchinese/`'s `Dynamic Entry File Source`
entries resolves to a real `entries.xml` in the same directory (17 checked,
17 present) - the per-language string tables this project's Pulse/HD reader
already expects.

## Movies are MP4 - read since 2026-09-21

**Confidence 92.** `data/Videos/intro.mp4` opens `66 74 79 70 6d 70 34 32`
(`ftyp` `mp42`) - a standard ISO Base Media File Format (MPEG-4 Part 14)
container, not `.bik` (HD), `.pmf` (PSP) or the PS2's IPU wrapper. 26 `.mp4`
files ship in the base package, all under `data/Videos/`: `intro.mp4`,
`2048Movie.mp4`/`2049Movie.mp4`/`2050Movie.mp4` (season recaps),
`bb2048.mp4`/`bb2048Zone8.mp4`, and twenty `shipunlocks/<Team>2048_<variant>.mp4`
clips (one per team per craft class). `oag-video` (`crates/video`) reads
`pmf`/`ipf`/`bik` and writes `ivf`/`av1`; **`mp4` joined them 2026-09-21**, a
fourth read-only container beside the three `oag-video` already carried, per
this codebase's own
[format-crates-split](../architecture/adr/0050-format-crates-split-by-format-family.md)
boundary. All 26 files parse; `intro.mp4` is 960x544 H.264 at 30000/1001 fps
(2,984 frames, 99.57 s) with a stereo 48 kHz AAC track (4,666 frames) - the
numbers this section's own confidence score was measured against, now
independently reproduced off the container alone with no H.264 or AAC decode.
`movie::open` plays it through the same AV1-cache pipeline `.bik` uses, with
no change to its own signature. See [mp4.md](mp4.md) for the container's
layout, the three-invariant survey and the box-order finding (`moov` is
always last, but `bb2048Zone8.mp4` alone swaps `free` after `mdat`).

## What the executable sweep did not reach

**2026-09-17 pass: could not decompile the Vita binary.** The live Ghidra
session this project shares had `/hdfury/EBOOT-ps3-hdfury-eu.elf` open under a
different lane's active work, and this lane's instructions were explicit:
bridge reads must name `program=/2048/eboot-vita-2048-eu-v104.elf` but never
call `switch_program`. In practice the bridge refuses a program that is not
*open* (`get_function_by_address` with that `program=` returns `Program not
found... Available programs: EBOOT.elf`), and opening it risked stealing the
other lane's GUI focus - the exact failure mode the instructions warn against
- so that pass did not attempt it, and instead ran the `strings` sweep below.

**2026-09-18: resolved.** `open_program` on `/2048/eboot-vita-2048-eu-v104.elf`
succeeds without disturbing the other lane's program - the bridge supports
more than one open program at once, and `list_open_programs` shows both. The
decompile this unblocked found the six-vs-four mechanism above; see
[frontend-campaign-map.md](../ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md)
for the full evidence. The 2026-09-17 `strings`-only sweep below is kept for
what it corroborates independent of the decompile.

What the 2026-09-17 pass had instead was a `strings` sweep of the raw, unencrypted
`eboot.elf` (`patch-v104/eboot.elf`, ARM/Thumb-2, no section headers - a
stripped SCE binary, so no symbol table to walk without Ghidra's own loader).
Textual corroboration only, no addresses, no names recovered or proposed:

- `"Boot Connect"`, `"Launch 2048"`, `"RaceBox"`, `"Main Menu"`, `"MPStress"`
  all appear as literal strings, clustered beside `"CheckCommandLine[%d] =
  %s"` and a `-mpscreen` command-line flag - consistent with
  [`game-boot.md`](../ghidra/functions/vita-2048-eu-v104/game-boot.md)'s
  existing, already-scored finding that `Game_Main` "picks a boot-mode string
  ... from flags read off `DAT_818bbf88`": a debug/launch-option selector that
  can jump straight to one of five named entry points, `Boot Connect` (the
  real boot) being the default. This pass adds no address and changes no
  confidence on that page - it is the same finding, seen from the string
  table rather than the decompiler.
- `"GameModeChoice"` and `"newFEshell"`/`"newFEShell"` (both cases) also
  appear, confirming the touch-shell screen names are referenced by the
  executable and not only by the XML that names them.
- `"frontend.bnk"` exists as a real archive entry
  (`data/audio/sound/frontend.bnk`), matching `SoundManager_Construct`'s own
  decompiled bank-path list in `game-boot.md`. **Resolved 2026-09-25: its
  cues are unread because there are none to read** -
  `oag_formats::sblk::Bank::sound_names()` on the extracted bank returns
  empty, `HAS_NAME_TABLE` clear, so nothing in it can be addressed by name
  the way a cue-based track is. `oag_title::Music::front_end` is filled from
  a standalone file instead: `data/audio/music/FEMusic/frontend_stereo.at9`,
  RIFF-wrapped ATRAC9 (the Vita's own codec - the new `oag_music::at9`
  decoder this needed), 48000 Hz stereo, a 302-second `fact` chunk. Beside
  it sits a second, distinctly-named
  `data/audio/music/FEDemoMusic/frontend_stereo.at9` for the attract-mode
  demo. The `FEMusic` folder and `frontend` stem are exactly the shape
  Pulse's `Data\Music\FEMusic\frontend1.at3` and HD's
  `Data\Music\FEMusic\frontend1_stereo.mp3` already use - **confidence 70**:
  unambiguous by placement and naming across three titles, not confirmed
  against a decompiled construction site the way Pulse's own template
  expansion is. `oag_2048::TITLE.music` is `Some` now; see
  `oag_2048::names::FRONT_END_MUSIC`'s own doc and
  `crates/game/tests/vita_2048_music_ground_truth.rs`, which loads it
  through `Audio::start_music`'s own path and renders it through a real
  `Mixer` to a WAV to confirm it is not silence. Wiring this also needed
  `Platform::Vita` added to `oag_sound::MusicDiscs::survey`/`pick` -
  every match there previously folded Vita in with `Unknown`, so
  `oag_2048::TITLE.music` going from `None` to `Some` was not by itself
  enough to reach the menus.

The 2026-09-17 pass proposed no new `names.tsv` rows - the addresses
`game-boot.md` already carried were all it cited. The 2026-09-18 decompile
above added six: three functions and three data globals, all confidence
75-78, recorded on
[frontend-campaign-map.md](../ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md).

## Why `front_end` stays `None`

The boot chain above is read to roughly the standard HD's `Declared` state
was in before its RPCS3 captures - screen-by-screen out of the XML, now with
one cold Vita3K launch confirming the order rather than none. That alone
would be enough to fill `FrontEnd::boot`. It is not enough to fill
`FrontEnd` as a whole, because `FrontEnd::menu: &'static oag_title::MenuSkin`
is not optional, and `MenuSkin`'s mandatory fields - `space`, `menu_x`,
`menu_scale`, `title_x`, `title_y`, `title_scale` - all read out of a
`FEGlobals` block 2048 does not ship.

Filling them with placeholder numbers would be exactly the
"hand-transcribed-table" and "plausible-looking stand-in" failures
`CLAUDE.md`'s "never invent what the assets already author" section names:
values attributed to a disc that does not state them, for a menu shape (a
scrolling text list or a horizontal strip) this title does not draw at all.
2048's menus are icon grids and a 3D touch canvas - a genuinely different
presentation idiom, the same class of disagreement that made `MenuSkin` an
axis over `oag-pulse`'s constants in the first place, one level up: not
*different numbers* in the same vocabulary, but no shared vocabulary to hold
numbers in.

**So `front_end` stays `None`, deliberately, and for a reason distinct from
"unread."** The boot chain, the colour globals, the language-plugin census
and the touch-screen tree are all read and are all sitting in this document
for whoever designs the axis that would hold them - most plausibly a new,
optional field on `FrontEnd` for a touch-driven front end, or a `MenuSkin`
variant, either of which is a type decision bigger than a first sweep should
make unasked. `loading` and `music` stay `None` on narrower, ordinary
evidence gaps (a real loading screen with no located plugin XML; a located
bank with no read cues) rather than this structural one.

### Resolved 2026-09-20: `front_end` is wired, and `menu` is the one field left `None`

[ADR-0054](../architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md)
made the call this section left open: `oag_title::FrontEnd::menu` is now
`Option<&'static MenuSkin>`, and `FrontEnd` gained a `touch:
Option<&'static TouchFrontEnd>` axis. `oag_2048::frontend::FRONT_END` fills
`root`, `language_plugins` (the seventeen plugins above, spelled exactly as
the archive's own lowercase directories), `boot` (the five-screen chain,
`Provenance::Declared`) and `touch` (every number in this section: both
grids' `TouchButton`s with their real `x`/`y`, the team screen's ship-model
origin, the `FE3DCanvas` screen and atlas names - re-read from the archive
for the type rather than transcribed from this page's prose, which never
quoted individual button coordinates). `menu` stays `None`, on the same
evidence this whole page already gave: no `<FEGlobals>` block, no
`<Menu>`/`<HorizMenu>` widget. `oag-game`'s menu-driven boot
(`boot::load_shell`) refuses on that narrower ground now, by name, pointing
at `--race`; `oag_game::main::session::placeholder` still stands in this
build's own menu for the title, on the same terms it always did.

**The HUD font this section names is now locatable and still does not
draw**, and the reason moved rather than closed. Wiring `front_end` makes
`language_plugins` reachable for the first time - a `--race` capture logs
all seventeen plugins where it used to log none - but
`crates/raceplay/src/hud.rs`'s `hud_font` asks for the literal role name
`"HUD"`/`"HUDSmall"` (`oag_ui::language::roles`, shared by every title), and
none of 2048's plugins name that role; they all name `2048HUD` instead. Two
of the seventeen - `korean` and `traditionalchinese` - do carry a leftover
`HUD`/`HUDSmall` role pointing at `Data\FE\Fonts\PulseHud.fnt`/
`koreanHudSmall.fnt`, neither of which 2048 ships, and because `hud_font`
searches every loaded language rather than only the chosen one, `korean`'s
dangling entry is what a `--race` report now names regardless of the
player's own language. See `docs/formats/2048-status.md`'s HUD bullet for
the full log evidence. Closing this is a new `oag_title` axis (a per-title
HUD font role name) and a real gap of its own for the `HUDSmall` half, since
no 2048 plugin names a distinct small-face role for it to read - out of
scope for ADR-0054, which wires the front end far enough to expose the
mismatch rather than to fix it.

### Resolved 2026-09-21: the HUD font draws

`oag_title::HudArt` gained `hud_font_role: &'static str` and
`hud_small_font_role: Option<&'static str>`, on the same terms
`MenuSkin::menu_font` already models a title's own role spelling rather than
a literal every caller shares. `crates/raceplay/src/hud.rs::load_hud` now
resolves `title.hud_art.hud_font_role` instead of the literal
`oag_ui::language::roles::HUD`. Filled per title:

| Title | `hud_font_role` | `hud_small_font_role` |
| --- | --- | --- |
| Pulse, Pure, HD | `"HUD"` (measured - each names it, confirmed against `docs/formats/hd-frontend.md`'s own quoted plugin XML for HD) | `Some("HUDSmall")` (measured) |
| Omega | `"HUD"` (placeholder - `oag_omega::hud::ART` is provably inert, see that module's doc) | `Some("HUDSmall")` (placeholder) |
| 2048 | `"2048HUD"` (measured, confidence 90 - the row above) | `None` (**a real gap**, not a default omitted - see below) |

**`HUDSmall` on 2048 is `None` because it is a real gap, measured at two
levels rather than assumed at either.** All seventeen of 2048's language
plugins were read for a second `<Font>` slot naming a caption face and not
one carries one - `2048HUD` is the entirety of this title's own HUD font
vocabulary. And composing every one of the seven roots the played skin
(`oag_2048::hud::skins::PLAYED`) actually carries finds **zero**
`font="HUDSmall"` widgets in any of them - every label in that skin is
`font="HUD"`
(`vita_2048_hud_ground_truth.rs`'s
`the_played_skins_layouts_author_no_hudsmall_widget_at_all`) - so no widget
the played HUD draws will ever ask this build to resolve a caption role at
all. `font="HUDSmall"` does exist in this archive, 261 widgets' worth, only
in the three skins (`wo3_hud`, `2097_hud`, the bare root) this title's
race-manager constructors never read.

So the visible size split is measured too, and it is not a second file. A
Vita3K race frame (`21-race2-start.png`, cited in [2048-hud.md](2048-hud.md))
shows `LAP`'s caption visibly smaller than the `1/3` value beside it, and
the composed layout explains it exactly: `LapTxt` (`"LAP"`) is authored at
`scale=0.6` beside `Laps` (`"1/3"`) at `scale=1.0`, both `font="HUD"`;
`RaceXPTxt`/`RaceXP` (`"XP"`/its value) are both `scale=0.6`, which is why
that pair reads as one size in the frame while `LAP`/`1/3` reads as two.
Each widget's own `oag_hud::widget::Label::scale` carries the size, on
one atlas - not a guess standing in for an unlocated second `.fnt`. So
`hud_font` falling back to `hud_font_role`'s own face for captions is not
merely the reading that draws something recognisable rather than nothing -
it is the reading the played skin's own layouts already assume, since none
of them ever names a second face to fall back *from*.

Verified with `just play 2048 --race --screenshot ...`: the boot report now
reads `HUD font Data\XML\2048_hud\font\2048_hud.fnt (role "2048HUD")` where
it used to read `no language plugin names a "HUD" font on this source;
drawing with 5x7`, and the screenshot shows the clean sans-serif `2048HUD`
face - matching the caption style in the real Vita3K captures this page and
[2048-hud.md](2048-hud.md) cite - rather than blocky 5x7 glyphs. Pulse and
HD were re-captured the same way and are unchanged: both still resolve
`"HUD"`/`"HUDSmall"` to `PulseHud.fnt`/`small.fnt` as before.

Pinned by two tests in
`crates/game/tests/vita_2048_hud_ground_truth.rs`:
`the_resolved_hud_font_is_2048_huds_own_face_not_a_leftover_or_the_fallback`,
which also asserts the regression this axis closes - resolving the shared
literal `"HUD"` against 2048's own plugins still finds `korean`'s leftover
`Data\FE\Fonts\PulseHud.fnt` entry, and that entry still fails to decode
(2048's own archive does not carry it), so the old first-match-across-every-
plugin behaviour is provably wrong, not merely superseded - and
`the_played_skins_layouts_author_no_hudsmall_widget_at_all`, which composes
all seven played roots and asserts none of them authors `font="HUDSmall"`.

## Wired: the boot walks and the grids draw (2026-09-21)

`just play 2048` (no `--race`) boots this front end now. What it does, in
the order a player sees it, with what each part rests on:

| Screen | Driven by | Drawn from | Standing |
| --- | --- | --- | --- |
| `Boot Connect` | left the tick it is entered: its `<UnityConBasic>` `MoveTo` goes to the same screen on success and failure, and this build has no network to check | its own white `<Image>` | authored, 92 |
| `Boot Studio Logo` | its own `<Redirect delay="4.0">`, read off the widget, or its `forward="cross"` redirect | `StudioLogo.gtf` at its authored centred `960x128` | authored, 92; the white ground under the `<BootFlowCanvas>` is **chosen** - the canvas's triangle-grid art has no located asset |
| `Boot Intro Movie` | its six button redirects; `AutoRedirect` when the movie ends | `intro.mp4`, picture and AAC track, through the AV1 cache ([mp4.md](mp4.md)) | plays its 99.57 s out and fires `AutoRedirect`, or leaves on any of its six buttons; the first run transcodes and caches |
| `Load Save Bootup` | left the tick it is entered: `StartEnabled="false"` on its one redirect, armed by a save check this build has nothing to check | `icon_save.gtf`, `VITA_AUTO_MSG` | authored, 92 |
| `TitleScreen` | its own six button redirects, all to `GameModeChoice` | `Title_Screen.gtf` centred, `BOOT_PRESS_ANY` pulsing after its `delay="1.0"`, the EU legal footer through the `DirectEmbed` include, in `NEOSANS_BOLD` at its own 22-unit line height against `NEOSANS_BOLD_LARGE`'s 37 | authored, 92; the font-role ratio is read off the two `.fnt` headers |
| `GameModeChoice` | left/right and cross on the pad, hover and click with a pointer; two taps on a mode (the second is the tick), only `FE_SP_CAMPAIGN` confirms | the four `<TouchButton>`s: `Blue2048` squares, the icon at its texture's own size, the label under it; `Grey2048` for the three network modes; the `Blue2048` block under the white header icon | authored, 92; tile/label/icon geometry measured off `08-game-mode-grid-clean.png`, 80; the `Orange2048` cursor ring is **chosen** |
| `Home` | the same | its five `<TouchButton>`s; each destination now draws - see [Wired: `Home`'s five destinations draw](#wired-homes-five-destinations-draw-2026-09-21) | authored, 92 |
| `newFEshell` (the campaign map) | d-pad to the nearest event, cross or a second click to launch; circle back to `GameModeChoice`, triangle to `Home` (the `<TouchHomeButton>`'s two targets, on buttons **chosen** - the widget names none) | `SP.xml`'s 115 events with a cell, on an even grid over the shell's own scrollable canvas, **each drawn with the disc's own hex tile art and mode icon since 2026-09-27** - see below; **no panel or card draws on a tap since 2026-09-25** - see below | the events, cells and canvas are authored ([2048-campaign.md](2048-campaign.md)); **the cell-to-pixel mapping is chosen, not measured** - `M_X`/`M_Y` are real `GameModeBase` fields the executable does read, but only into a raw struct offset (`0x2c4`/`0x2c8`); the DLC tiers' own hotspots read a *different*, still-unfound cached position (`+0x15c`/`+0x160`) that something else must derive from them ([frontend-campaign-map.md](../ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md)'s 2026-09-21 section), and a live capture confirms the real map is a hexagonal tessellation, not this square grid (`data/reference/2048-frontend/README.md`'s frames `12`-`14`, gitignored); **2026-09-25: the grid's own scale is now measured, not the DLC-tier projection**, and **2026-09-27: each cell's own hex tile, outline, selection ring and mode icon are the disc's own decoded art**, pixel-confirmed against frames `12`/`13`'s green/grey chequered-flag and stopwatch tiles - see below |

**2026-09-25: the campaign map's canvas/pitch/marker scale is measured off
frames `12`/`13`, correcting a wrong reading of the shell's own XML.** The
`<TouchScroll>`'s declared `MaxScrollX="960" maxscrolly="544"` was read as
the whole canvas (view + scroll = 1920x1088); the two live frames' own
scrollbar thumbs measure a canvas roughly 5.3x-5.4x the view instead (the
vertical thumb spans 103 of 544 px, the horizontal thumb 177 of 960 px and
moves between the two frames the way a real scrollbar would), putting the
real canvas near 5207x2873 - about 2.7x bigger on each axis than this build
assumed. `oag_ui::frontend::campaign_map`'s `CANVAS`/`PITCH`/`ORIGIN` are
rescaled by that factor and `MARKER` is set directly off a second, independent
measurement - a flood-filled bounding box around one clean hex tile in frame
`13` (122x117 px) - rather than derived from the old, wrong canvas. The grid
is still a square standing in for the real hexagon, so this stays **chosen,
not measured, no confidence score** as a whole, but the scale itself is now a
pixel measurement of the disc's own screen rather than a wrong reading of an
XML attribute; node size and spacing now visibly match the reference frames'
own density (previously about a third of the correct size and spacing).
**Still unmeasured**: the per-event `M_X`/`M_Y` -> pixel formula itself (the
square grid still places events by cell index alone, not by whatever
compiled-in projection the DLC tiers' own `+0x15c`/`+0x160` reads), the light
triangle-pattern background and its striped/dotted "reachable path" overlay,
the persistent header (home button, top-right badge cluster) and the season
title card / per-node trophy badges
`FE3DCanvas`'s `<CanvasLabel>`s carry. **A negative finding worth recording
so it is not re-attempted**: a linear least-squares fit of the 47
`linkedevent`-carrying `<CanvasLabel>`s' own `x`/`y` against their linked
event's `M_X`/`M_Y` looks promising at first (errors under 15 px on a fit
with `y` almost exact) - but the same file's twenty `MPSeason01`-`MPSeason20`
labels, which carry **no** `linkedevent` at all and have no relationship to
`SP.xml`'s grid whatsoever, occupy the *identical* small coordinate range
(`x` 55-196, `y` 25-87). That is conclusive: the apparent fit is a spurious
correlation between two independently monotonic small-number ranges, not a
real projection, and the existing "one small corner, not hotspots spread
across the map" reading (below) stands. A full survey of every plausible
asset name for the hex tile, the season card and the per-node badges
(`hex_filled.gxt`, `hex_outline.gxt`, `hex_select.gxt`, `Hexagon_HD*.gxt`,
`Cup2048.gxt`/`Cup2049.gxt`/`Cup2050.gxt`, every `trophy/*.gxt`, every
`medals/Icon_*.gxt`, every `TinyCallout_*.gxt`) against every `.xml` entry in
the base package, with `cargo run -p oag-tools --example psarc_grep`,
matches zero of them: **matching no widget's own text is not the same as not
existing in the archive**, and every one of the names in that list is a real
file that decodes (checked directly, 2026-09-27, below) - the per-event
*placement* is what stays native-code-driven and out of reach without a
Ghidra pass, the same conclusion `frontend-campaign-map.md` already reached
for the DLC tiers' own hotspots, but the *art itself* was never actually
missing.

**2026-09-25: the invented bottom panel is removed, and no event-card
screen replaces it - searched for as XML and not found.** Until this pass a
tap drew a full-width bar naming the selected event's name/circuit/mode/laps
/weapons, covering roughly 15% of the screen - this build's own chrome, not
anything the disc draws there. The real screen shows nothing on the base map
itself; tapping a node (`data/reference/2048-frontend/
14-campaign-map-event-card-unity-square.png`, gitignored - **and this fires
for a locked node too**, not only an open one) opens a separate, richer
card: a per-event photo backdrop, the event's name and kind, a `PASS`/
objective line, lap-count arrows, three pagination dots and three bottom
buttons. That card was searched for as an authored `NEWGUI` screen the way
`CANVAS`/`PITCH`/`MARKER` above were measured as pixels, and it is not
there: `<TouchCampaign>` carries exactly one child, `redirect="Launch
2048"`, in both the base package and the `v1.04` patch; `Launch 2048`
(`InGame_Definition.xml`, byte-identical between the two) is nothing but a
`<BackendController task="Launch">` straight through to `InGame2048`, no
confirm screen in between. The two screens that share the tick/cross
vocabulary a confirm dialog would use - `StartEventConfirm`/
`FriendStartEventConfirm` (`Community_Definition.xml`) - are a small
660x192 online "join this friend's event?" box, two buttons, one line of
text: nothing like the three-button, photo-backed card. None of the 25
`NEWGUI/` documents across the base package, the patch's two archives or
either DLC package name anything shaped like it (`cargo run -p oag-tools
--example psarc_grep -- <data.psarc> <needle>` against `TouchCampaign`,
`Launch 2048` and every screen name the archive itself lists; a new scratch
tool, `dump_screen_scratch.rs`, dumps any one named `<Screen>` block out of
any `NEWGUI/*.xml` entry to check each candidate in full). The card's own
text is real and already shared with `EndRace_Definition.xml`'s post-race
objective screen (`ER_FINISH_5TH` and its siblings in
`data/plugins/languages/*/entries.xml`), but its layout, its per-event photo
and its button chrome are not - native-code-driven and unlocated, the same
conclusion the hex tile art, background, header and season card above
already reached. Per this project's "never invent" rule, nothing stands in
for it: a player who selects an event today sees only its marker recolour
under the cursor ring, with no name, circuit or objective text anywhere on
screen - `Frontend::selected_event`/`MapEvent::detail` still carry that real,
disc-authored text (built from `SP.xml` in
`crates/game/src/boot/campaign2048.rs`) for whenever the card itself is
recovered, but the draw call that used to show it is gone, not replaced.

**2026-09-28: the card's own *code* is now located, its layout still is
not.** Chasing the user's own play observation that 2048 restricts craft
choice on some events led to `GameModeBase_IsShipTypeAllowed` (`0x812b41da`)
and its two callers, `CampaignEventCard_HandleInput`/`CampaignEventCard_Draw`
(`0x810f2164`/`0x810f1196`,
`docs/ghidra/functions/vita-2048-eu-v104/campaign-event-card.md`) - the input
handler and draw call for exactly this card: three buttons at the same
bottom-row height the live capture shows, one of which (Launch) is disabled
by the same craft-restriction check this page's own "never invent" paragraph
above says nothing draws. **Still nothing stands in for the layout** - the
photo backdrop, the per-event art and the pagination dots are drawn by this
same function but their placement was not decompiled past the button rects,
so the "never invent" conclusion above is unchanged: this project's own
craft-restriction gate runs at
`oag_ui::frontend::campaign_map::Frontend::launch_selected_event` instead, on
the existing map screen, rather than waiting on this card - see
`docs/formats/2048-campaign.md`'s "Craft choice" section.
| `Launch 2048` | the map's and `<TouchCampaign>`'s own `redirect` | - | the disc's own name for leaving the front end; carries the event name to `oag_raceplay::load_event` |

**2026-09-27: each map marker draws the disc's own hex tile art and mode
icon, not a flat colour square.** The 2026-09-25 pass's own texture survey
(`hex_filled.gxt`, `hex_outline.gxt`, the per-kind icon names) only grepped
candidate filenames against every `.xml` entry's own text and, finding no
widget naming them, filed the hex art as unlocated alongside the season card
and the header. That check never opened the files as pixels. This pass did:
`data/FE/Images/hex_{filled,outline,select}.gxt` and `data/FE/NewImages/
callout/{race,speed,zone,combat}_mode.gxt` (all base package) decode to a
gloss-filled hexagon, a thin hex outline, a thicker glow ring, and a
chequered flag / stopwatch / radar-target / crosshair icon respectively -
confirmed against the same `data/reference/2048-frontend/` frames `12`/`13`
the grid's own scale was measured off: those frames show green hexagon tiles
carrying a chequered-flag glyph and a stopwatch glyph, grey ones carrying the
same two glyphs unlit, exactly matching `race_mode`/`speed_mode`'s decoded
art on the same `Pass2048`/`Grey2048` split this build already drew a flat
square in. `oag_2048::campaign::EventKind` (already measured at confidence
78-82, `docs/formats/2048-campaign.md`) picks the icon per event, plus the
already-measured `laps == 0` Speed Lap split. See
`oag_ui::frontend::campaign_map`'s own "2026-09-27" doc section for the
implementation and what the real tile still has that this build does not (a
3D bevel/drop-shadow, the striped/dotted path texture connecting a season's
own tiles, the season card, the header). `crates/game/tests/
vita_2048_boot_ground_truth.rs::the_campaign_map_draws_the_discs_own_hex_tiles_not_flat_squares`
pins it against the real package.

**One plumbing bug this pass found and fixed, worth naming since it hid in
plain sight.** `oag_game::boot::sprites::load`'s own report already said "N
of N front-end image(s) decoded" including the seven new names - the decode
worked from the first attempt. What did not work silently was
`boot::assemble`'s own `Frontend::placements`, rebuilt by re-walking every
screen's `Image`/`TouchButton` `Src=` rather than reading `Sheet::entries`
directly - a walk that, by construction, never named an `extra`-only texture
(the menu blocks' own nine-patch was in the same position before this pass,
just never read back out through `placements` by anything). Fixed by reading
`Sheet::entries` directly, a strict superset. Caught only by taking an actual
screenshot and comparing it against the boot log's own success line, not by
either alone - the log was truthful and the screen was still wrong.

**How the screens are found.** `NEWGUI/Skin.xml` declares no screen; the
boot follows four of the root's `<LoadXML>` includes by name
(`oag_2048::frontend::includes::FOLLOWED`: `Definition.xml`,
`Bootup_Definition.xml`, `Intro_Definition.xml`,
`Legal_Line_Definition.xml`), resolving `localised="true"` to `_EU` - the
package's own territory, `PCSF00007` - and `DirectEmbed="true"` into the
including screen. Every `.gtf` the XML names is respelled `.gxt` once the
name as written fails, which is every one of them on this package. The
`--dry-run` report names each include and each image.

**Two tiles the disc does not author.** At the user's request, `RACEBOX`
and `REMIX` sit on `GameModeChoice` after the four authored tiles and the
tick - **this build's own, not on the disc**. 2048 ships no race box and
no `CellMode_Definition.xml` (base, patch and both DLC packages checked);
every other title reaches this build's race box and RACE REMIX pages from
its own front end, and these two tiles are how 2048 does. They are
labelled with the menu tree's own `OAG_MENU_RACEBOX`/`OAG_MENU_REMIX`
strings, drawn as the 122x96 text box the disc's own game-list screen
authors (`10-adhoc-game-list.png`), and placed at `(16, 432)` and
`(158, 432)` - the tick's row, bottom-left - **chosen, not measured**. A
tap opens this build's menus and walks into the `race` or `remix` page, so
BACK returns to the menu root. See [menus.md](../architecture/menus.md)'s
own section on them.

**Reproduce.** With `data/extracted/vita/PCSF00007` in place:

```sh
just play 2048 --until TitleScreen --press cross --ticks 1 --screenshot title.png
just play 2048 --until GameModeChoice --press cross --ticks 1 --screenshot grid.png
just play 2048 --until newFEshell --press cross --ticks 1 --screenshot map.png
just play 2048 --press cross --ticks 120 --screenshot race.png   # ...taps through to 2048 - Event 1
just play 2048                                                    # the window: X skips, mouse works
```

`crates/game/tests/vita_2048_boot_ground_truth.rs` pins the chain, the
includes, the grid against `oag_2048::frontend::TOUCH`, the pad walk to the
map and the event launch; `crates/ui/src/frontend/tests/wipeout2048.rs`
does the same on a fixture with no disc.

**Still open after this pass**: the intro's picture and length
(`lane/2048-intro-mp4`); the `BootFlowCanvas` art; the campaign map's real
per-event pixel anchor (2026-09-21: `M_X`/`M_Y` are real `GameModeBase`
fields, but what copies them into the position the DLC tiers' own hotspots
actually read - `+0x15c`/`+0x160`, a different offset - is still unfound;
2026-09-25: the grid's overall *scale* is now measured off the reference
captures, but not this per-event formula - see the campaign-map row above)
and the season title card's own asset (`A·G·R·C 2048`/`2049`/`2050`, seen
live on Vita3K, not located in either package - 2026-09-25: confirmed absent
from every `.xml` in the base package by name too, so it stays native-code
driven) - there is no 3D city backdrop to find, confirmed from the live
capture too; the network modes. `Home`'s five destinations are resolved
below.

**Resolved 2026-09-21: the unlock graph.** Every event was offered
regardless of the disc's own `M_PNEXTEVENT`/`M_PBRANCHEVENT`/
`M_PEVENTREQUIRED` chain, since nothing read a save. `oag_2048::campaign::
unlock_gates` now walks it, `oag_game::records::Store` persists a finished
event's result under its own name (`Store::record_campaign`, the same
sibling table Pulse/HD's own campaign cells use), and
`oag_ui::frontend::Frontend::refresh_campaign_progress` folds that save into
the map once at boot - a locked event draws `Grey2048` and refuses a
launch, a passed one draws `Pass2048`/`ElitePass2048`. See
[2048-campaign.md](2048-campaign.md)'s "The unlock graph" and "The
objective law" sections for the measured law, and
`oag_ui::frontend::campaign_map`'s own "Progression" section for the draw
side. Still open: which medal tier is required to unlock the next event is
chosen, not measured (see 2048-campaign.md's own note on this), and the map
still lays cells out on the same chosen even grid rather than the real
hexagonal tessellation - see this page's own campaign-map row above.

## Wired: `Home`'s five destinations draw (2026-09-21)

`includes::FOLLOWED` now carries `Team_`, `Community_`, `Profile_`,
`Options_` and `Extras_Definition.xml` (`Extras_Definition_EU.xml` on this
package - `localised="true"`, the same territory switch `Bootup_Definition.xml`
already goes through), and their bare screen names are in
`oag_ui::frontend::wipeout2048::STATES` - closing the "screen this build does
not load" gap every one of the five tiles hit before this pass. Reproduce:

```sh
just play 2048 --until team --press cross,cross,triangle,cross --ticks 40 --screenshot team.png
```

(`--press` ORs every named button into one held mask rather than replaying a
sequence - see the note at the end of this section - so a multi-screen walk
through screens with conflicting Left/Right priorities has to be driven from
a test rather than the CLI; `crates/game/tests/vita_2048_boot_ground_truth.rs`
does this for all five.)

| Screen | Driven by | Drawn from | Standing |
| --- | --- | --- | --- |
| `team` | Left/Right cycles the team, Up/Down the craft slot, Cross cycles the skin list, Circle fires `select_button`'s own `redirect="PreviousScreen"` | header Image/Text (authored, 92); a team x craft-slot grid built from `oag_2048::race::NATIVE_TEAMS`/`SHIP_TYPES`, labelled with the disc's own idstrings (`AG_Systems2048`/`Auricom2048`/... and `FE_SHIP_COMBAT`/`FE_SHIP_AGILITY`/`FE_SHIP_SPEED`/`FE_SHIP_PROTO`, all confirmed in `english/entries.xml`); `teamskin_touch`'s own three-entry skin list | grid tile size/gap **chosen**, bounded inside `x` 16-414 (`teamskin_touch`'s own `x="430"`) and above `y="307"` (`TeamInfo`'s own `y`) - see `crates/ui/src/frontend/team.rs`'s module doc for why `TouchTeamGrid`'s own `teamgrid/` icon set is not drawn (native-code-populated, unwired into the sprite sheet); the `<Model name="ShipModel">` preview is not drawn (no 2D-front-end-to-3D-mesh renderer seam, the same gap `oag_ui_screens::picker::slideshow`'s own `Model` sits on) |
| `communityAdhocCheck` | `FE_COMMUNITY`'s **real** redirect target - `Home` never reaches `community` itself, which needs a live session | authored, 92 - the `FE_COMMUNITY_UNAVAILABLE_ADHOC` text and the tick back to `Home` | refused by note, the same standing `GameModeChoice`'s three network modes carry |
| `profile` | generic touch grid | header Image/Text, arrow buttons to `profile_stats` | authored, 92; `ProfileTouchMain`'s own stat panel and the screen's `<Model>` preview are not drawn, both on the same native-populated/no-renderer-seam grounds `team` gives |
| `OptionsCamera`/`OptionsAudio`/`OptionsControls`/`OptionsPilot` | Left/Right steps the panel's own control, Circle returns to `options` - **chosen, no widget on any of the four names a back button** | `options`'s own header and four buttons, drawn first (nested-not-sibling screens, see below), then the panel's own new widget kind: `Touchlist` (`CameraP1`/`Pilot Assist`/`Motion Sensor`) or `TouchSlider` (`Music Volume`/`SFX Volume`), `crates/ui/src/screen/settings.rs` | Camera and the two volumes feed `Frontend::camera_choice`/`music_choice`/`sfx_choice` (`None` until touched, applied once at `Launch Game` into `settings.graphics.camera_view`/`audio.music_volume`/`sfx_volume` - see `crate::main::session::frame`); Controls and Pilot draw and cycle and reach no setting this build has. Per-entry spacing across the widget's own one rect is **chosen** the same way the team grid's is. The picker's starting index is this build's own default, not `CameraP1`'s authored `default="OPT_CLOSE"` - see `options2048`'s module doc for why threading the live setting in was left open |
| `2048extras`, `manual3D`, `extrasCredits` | generic touch grid; `manual3D`'s own tick is `redirect="PreviousScreen"` | header Image/Text, four real tiles (`<aTouchButton>`'s own tag-name typo drops the fifth, AR Museum, exactly the way a real player's build would - it is not a `<TouchButton>`) | authored, 92; the ship-unlock and season-recap movies the brief asked about are not named by any widget here - native-code driven, the same open gap `frontend-campaign-map.md` already records for the campaign map's own hotspot buttons, so nothing is played rather than a guessed path |

**Home's own hub-versus-panel shape.** `optionsshell->options`'s four
buttons sit at `x` 95-345; `OptionsCamera`/`OptionsAudio`/`OptionsControls`/
`OptionsPilot` are **nested inside** `options`, not siblings of it, and their
own `Touchlist`/`TouchSlider` sit at `x` 430-940 - non-overlapping, and
neither carries a header or a back button of its own. Contrast
`OptionsControlsConfig`, a true sibling page one level further in that does
carry both. `Home`'s own `FE_OPT_PLUS` tile confirms this by jumping
straight to `OptionsCamera`, never to `options` itself - read directly off
`Definition.xml`.

**A new back gesture: `redirect="PreviousScreen"`.** `Team_Definition.xml`'s
`select_button` and `manual3D`'s own tick both author this literal target
rather than a screen name, and nothing in this build read it before this
pass - the two screens were simply stuck once entered generically. Resolved
with a small stack (`TouchState::history`, pushed by
`Frontend::redirect_touch` on every named forward hop, popped on
`"PreviousScreen"`), which is also what makes `Team`'s and `Extras`' own
sub-screens return to whichever tile sent them there rather than to `Home`
by name.

**One touch-tile bug found and fixed in the same pass.** The `GameModeChoice`
toggle's "tapped again confirms through the tick" rule (`Frontend::activate_touch`)
used to run on every screen, matched by *shape* - a chosen toggle plus a
label-less redirect tile - rather than by screen name. `Team_Definition.xml`'s
own `replay_unlock`/`replay_unlock_2` (`redirect="team_unlock_video"`, no
label) fit that shape too, so a second tap on any of `team`'s own toggle
buttons would have silently auto-fired one of them. Guarded to
`GameModeChoice` alone now that a second screen exercises the generic path.

**What `--press` actually does, for the next reader who tries a multi-screen
`--until`.** `crates/game/src/main/args.rs::button_mask` ORs every named
button into **one mask**, held on even ticks and released on odd ones - it is
not a sequence. `--press cross,cross,triangle` does not mean "cross, then
cross, then triangle"; it means cross+triangle held together, pulsed forever.
That is harmless exactly where the two buttons' handlers never compete for
the same tick (Triangle is checked only inside `newFEshell`'s own top-of-function
special case, Cross everywhere else), which is why `cross,triangle` walks
`Boot Connect` all the way to `Home`'s first tile - but `Home`'s own
Left/Right-vs-Cross priority (`Frontend::update_touch`: Right, then Left,
then Cross) means adding `right` to reach a later tile makes Right win on
*every* generic touch screen the mask is held through, including
`GameModeChoice`, and the walk never leaves it. A multi-step walk through
different tiles needs a test driving per-tick input
(`crates/game/tests/vita_2048_boot_ground_truth.rs::press_release`), not a
single `--press` mask.

## See also

- [hd-frontend](hd-frontend.md) - the reading this page's shape and rigor are
  modelled on, and the title 2048's own codebase is retargeted from
- [2048-status](2048-status.md) - where this fits in the wider probe
- [2048-hud](2048-hud.md) - the in-race HUD, corrected above on the font-role
  finding, and the source of the `DemoRaceManager` finding this page's attract
  capture corroborates
- [vita3k-capture](../reverse-engineering/vita3k-capture.md) - the capture
  recipe, extended here with this pass's own display numbers
- [game-boot](../ghidra/functions/vita-2048-eu-v104/game-boot.md) - the
  decompiled boot-mode selector this page's `strings` sweep corroborates
- [frontend-campaign-map](../ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md) -
  the decompile that resolved the six-vs-four `GameModeChoice` question above
- [gxt](gxt.md) - the Vita's texture container, already reading every font and
  image this page names
- [ADR-0025](../architecture/adr/0025-a-boot-chain-carries-its-provenance.md) -
  why a boot chain's order and its layout are recorded separately, and why
  neither alone is enough to wire `front_end`
