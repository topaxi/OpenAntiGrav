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
| Do HD's own language-plugin bugs reproduce here? | **Yes, byte for byte** - the `Svenska` mixups, the mangled Russian name, and the still-labelled `Wipeout Pulse` internal id | 90 |
| What plays the boot movie? | `data/Videos/intro.mp4` - a real MP4/ISOBMFF container, not `.bik`/`.pmf`/`.ipf` | 92 |
| Is `front_end` wired? | **No, and not for lack of evidence** - `oag_title::MenuSkin` cannot honestly hold this front end's numbers. See [below](#why-front_end-stays-none). | - |

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
- **The persistent background is a real 3D scene**, `<FE3DCanvas>` inside
  `newFEshell`'s own `<TouchScroll>`, with ~50 `<CanvasLabel>` hotspots
  (`Trophy-2048-2-3`, `ShipUnlock-2049-3-4`, `SpeedRating_Top_C`, ...) mapped
  onto UV coordinates of a rendered campaign-map model, each carrying a
  `linkedevent` naming a specific race event. This is the literal "explorable
  city" campaign map, not a page of text - `TouchCampaign`'s own redirect
  target is `Launch 2048`, one of the boot-mode strings the executable also
  knows by name (below).
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

## The Game Mode grid has six tiles; the file declares four

**Confidence 85, runtime observation.** `NEWGUI/Definition.xml`'s
`GameModeChoice` screen authors exactly four `<TouchButton>`s -
`offline`/`multiplayer`/`adhoc`/`crossplay` (`FE_SP_CAMPAIGN`/`FE_MP_CAMPAIGN`/
`FE_ADHOC`/`FE_CROSSPLAY`). The Vita3K capture (`08-game-mode-grid-clean.png`)
shows **six**: top row `SINGLE PLAYER CAMPAIGN`, `HD CAMPAIGN`, `FURY
CAMPAIGN` (all enabled); bottom row `ONLINE CAMPAIGN`, `ADHOC`, `CROSS-PLAY`
(all network-gated, pale). `HD CAMPAIGN`/`FURY CAMPAIGN` correspond to no
`TouchButton` in the base file at all - they read as tiles the executable
inserts once it detects `dlc1.psarc`/`dlc2.psarc` are mounted (both ship HD's
own re-shipped circuits, per `docs/formats/2048-status.md`), rather than
something authored anywhere in `NEWGUI`. Not chased into the executable this
pass - see [what the executable sweep did not reach](#what-the-executable-sweep-did-not-reach).

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
(`crates/game/src/race/hud.rs`, `crates/game/src/race/load.rs`) always gets
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
  the same garbled string HD ships.
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

## Movies are MP4 - a container this project does not read

**Confidence 92.** `data/Videos/intro.mp4` opens `66 74 79 70 6d 70 34 32`
(`ftyp` `mp42`) - a standard ISO Base Media File Format (MPEG-4 Part 14)
container, not `.bik` (HD), `.pmf` (PSP) or the PS2's IPU wrapper. 26 `.mp4`
files ship in the base package, all under `data/Videos/`: `intro.mp4`,
`2048Movie.mp4`/`2049Movie.mp4`/`2050Movie.mp4` (season recaps),
`bb2048.mp4`/`bb2048Zone8.mp4`, and twenty `shipunlocks/<Team>2048_<variant>.mp4`
clips (one per team per craft class). `oag-video` (`crates/video`) reads
`pmf`/`ipf`/`bik` and writes `ivf`/`av1`; it has no MP4 demuxer.
**This is a new format for this project**, per this codebase's own
[format-crates-split](../architecture/adr/0050-format-crates-split-by-format-family.md)
boundary - MP4/ISOBMFF demuxing would be a fourth read-only container beside
the three `oag-video` already carries, not a fix to any of them. Not
implemented this pass; the magic and the count above are what a reader would
need to target.

## What the executable sweep did not reach

**This pass could not decompile the Vita binary.** The live Ghidra session
this project shares has `/hdfury/EBOOT-ps3-hdfury-eu.elf` open under a different
lane's active work, and this lane's instructions are explicit: bridge reads
must name `program=/2048/eboot-vita-2048-eu-v104.elf` but never call
`switch_program`. In practice the bridge refuses a program that is not
*open* (`get_function_by_address` with that `program=` returns `Program not
found... Available programs: EBOOT.elf`), and opening it risked stealing the
other lane's GUI focus - the exact failure mode the instructions warn against
- so this pass did not attempt it. `analyzeHeadless` against a second,
independent project would sidestep that, but was judged out of scope for a
first sweep given the time already spent capturing and reading; it is the
concrete next step, named in the handover thread.

What this pass has instead is a `strings` sweep of the raw, unencrypted
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
  decompiled bank-path list in `game-boot.md`. Its cues are unread this
  pass - `oag-wad sounds` reads a WAD-hosted bank, and 2048's is PSARC-hosted;
  extracting it first was judged out of scope here. This is why
  `oag_title::Music::front_end` (which needs a *cue* name, not a bank
  filename) stays unfilled.

No new function or address is proposed for `names.tsv` this pass - the
addresses `game-boot.md` already carries are all this page cites, and nothing
below confidence 50 gets a name per `CLAUDE.md`'s own rule.

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
- [gxt](gxt.md) - the Vita's texture container, already reading every font and
  image this page names
- [ADR-0025](../architecture/adr/0025-a-boot-chain-carries-its-provenance.md) -
  why a boot chain's order and its layout are recorded separately, and why
  neither alone is enough to wire `front_end`
