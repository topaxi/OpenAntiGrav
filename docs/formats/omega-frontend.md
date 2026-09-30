# Wipeout: Omega Collection's front end is HD's `PI001` plugin, carried forward - not a rewrite

**2026-09-21.** A player-level sweep, prompted by "the Omega menu looks very
similar to HD/Fury's". It is not similar - it is the same plugin. This page
answers the specific question this project can check without an emulator: is
Omega's front end [HD/Fury's](hd-frontend.md) `FEGlobals`/`MenuSkin` XML
re-shipped, and how much of it would `oag_ui`/`oag_title::MenuSkin` read
unchanged if `oag-omega` existed. **Omega is not an `oag-title` today** - no
`crates/omega`, no `oag-omega` in any `Cargo.toml` - so every claim below is
static reading against the disc's own bytes, the same posture
[hd-frontend.md](hd-frontend.md) started from before its 2026-09-05 RPCS3
capture. Nothing here was measured on a PS4 or a PS4 emulator; **no PS4
emulator boot capture exists in this project**, so every confidence below is
capped the way an authored-value claim always is, never at a runtime tier.

Prior art this builds on rather than repeats: `data/extracted/ps4/omega-eu/`
and `-patch/` are already documented in
[source-images.md](../reverse-engineering/source-images.md#omega-ps4-eupkg--omega-ps4-eu-patchpkg---wipeout-omega-collection-ps4),
the container's own block-data-location trap is
[psarc.md](psarc.md#the-ps4-omega-collection-family), and the patch's four
extra archives (`data05`/`data07`/`data08`/`data09`, none of the base
package's `data00`-`data04`) were already census'd once at the file-listing
level in an earlier handover thread. This page is the first pass that opens
the front-end files that census only named, and diffs their content against
HD's.

## The headline

| Question | Answer | Confidence |
| --- | --- | --- |
| Is Omega's front end HD's `FEGlobals`/`MenuSkin` idiom? | **Yes.** Every layout global that has a value in HD's own headline table (`docs/formats/hd-frontend.md`) reads byte-for-byte identical in Omega's `skin.xml` | 88 |
| Same load path (`Data\Plugins\PI001\GUI\...`)? | **Yes.** `skin.xml`'s own `<LoadXML>` list still names the `PI001` plugin id, unchanged since Pulse | 85 |
| Same boot chain (8 screens, no dead `LogoFMV`)? | **Declared the same**, screen names and redirect targets identical to HD's `DATA00`/`DATA05`/`DATA06` family. **Not runtime-measured** - no PS4 emulator capture exists | 75 (declared only) |
| Is 2048's campaign re-authored into HD's `Grid Selection`/`CellMode` vocabulary, or kept as 2048's own `NEWGUI`/`FE3DCanvas`? | **Both, for different layers.** The screen *shape* (`Campaign Selection` -> `Grid Selection`/`Grid Selection Fury`/`Grid Selection 2048` -> `Cell Selection`/`Cell Selection 2048`) is HD's own `FlyerSelection`/`CellSelection` screen types, extended with one more branch. The campaign-map *content* inside the 2048 branch is a new `Campaign2048_Definition.xml`, authored in 2048's own `FE3DCanvas`/`CanvasLabel`/`linkedevent` vocabulary, not HD's | 80 |
| Which of nine archives carries the live copy? | **Unresolved**, the same open question HD's own six copies posed before its RPCS3 capture - see "Load order is not settled" below | - |
| Anything genuinely new to Omega, not inherited from HD? | **Yes**: a `2048_Colours` skin block, PS4 pad prompts (`Controls_Definition_PS4.xml`), and PSVR-specific screens/options (`VR Warning...`, `OPT_COMFORTVR`, a `frontendscene2_VR.vex`) | 85 |

## Reading it yourself

```sh
just link-data   # in a worktree, once
cargo run -p oag-assets --example psarc_list -- data/extracted/ps4/omega-eu-patch/uroot/data09.psarc
cargo run -p oag-assets --example psarc_cat -- data/extracted/ps4/omega-eu-patch/uroot/data09.psarc \
    data/plugins/frontend/gui/skin.xml
cargo run -p oag-game --example omega_frontend_probe -- data/extracted/ps4/omega-eu-patch/uroot/data09.psarc
```

`fd`/`rg` return nothing under `data/` without `--no-ignore` (`psarc.md`'s own
warning applies here too); use `/bin/ls`, `find` or the tools above.

## The census: front-end content is in two of nine archives, not spread evenly

**Confidence 92** - this is a direct enumeration, not an inference. Of the
nine `dataNN.psarc` archives (`omega-eu/uroot/data00`-`data04`,
`omega-eu-patch/uroot/data05`/`data07`/`data08`/`data09`),
**`data01`-`data04` carry zero paths under `Data/plugins/frontend/`,
`Data/plugins/languages/` or `Data/fe/`** - checked directly, not inferred
from the base game's five-chunk PlayGo mount
([`psarc-mount.md`](../ghidra/functions/ps4-omega-eu/psarc-mount.md)). All
front-end content lives in `data00` (base) and the patch's `data05`/`data07`/
`data08`/`data09`:

| Archive | Package | `frontend/gui/*.xml` | `plugins/languages/*` | `Data/fe/*` (fonts/images/models) | Total entries |
| --- | --- | ---: | ---: | ---: | ---: |
| `data00.psarc` | base | 10 | 5 | 182 | 1,492 |
| `data05.psarc` | patch | 1 | 10 | 7 | 1,205 |
| `data07.psarc` | patch | 1 | 0 | 4 | 6 |
| `data08.psarc` | patch | 9 | 5 | 182 | 1,786 |
| `data09.psarc` | patch | 45 | 46 | 0 | 120 |

**`data00` alone is not a bootable front end.** Its 10 XML files are
`controls_definition_ps3.xml`, `credits_text_jp.xml`, `ingame_definition.xml`,
`manual_definition.xml`/`_jp.xml`, `minigamelist.xml`, `online_definition.xml`,
`stats_definition.xml`, `team_selection_definition.xml`, and
`vita/vita_ingame_definition.xml`. **No `skin.xml`, no `mainmenu_definition.xml`,
no `racebox_definition.xml` anywhere in the base package** - checked across
all five base archives, not just `data00`. Wipeout: Omega Collection ships as
a base `.pkg` plus a mandatory day-one patch the way many PS4 titles do; the
base disc's own front-end plugin is an older, partial build the patch
supersedes wholesale, not a redundant copy the way HD's PS3 six-skin lineage
was. `data09.psarc` is almost entirely front-end/language/plugin XML (91 of
its 120 entries; the rest are `plugins/billboards`, `plugins/downloads`,
`plugins/grids/grid_00`-`18.xml`, `plugins/music`, `plugins/teams`,
`plugins/tracks` - no assets at all), and it is the only archive that carries
the complete file: `skin.xml`, `mainmenu_definition.xml`,
`racebox_definition.xml`, `cellmode_definition.xml`, and a brand-new
`campaign2048_definition.xml`, none of which the base package or `data05`/
`data07`/`data08` carry in full. `data08.psarc` carries the bulk of front-end
*assets* (fonts, `.gnf` images, `.rcsmodel` flyers - the same 182-file count
as `data00`, but a different, newer set of 9 XML files) and `data07.psarc` is
a six-entry oddity: one stale `cellmode_definition.xml`, two Korean font
pairs, and one `Zone` ship model fix.

### Which files read clean, per the documented block-data-location trap

[`psarc.md`'s "Block data location"](psarc.md#block-data-location-and-the-short-read-extraction)
already established a three-way split (valid / all-zero / garbage-with-real-
bytes-elsewhere) for `.gnf`/`.vex`/`.rcsmodel`/`.rcsmaterial` on the five base
archives. The same trap applies to these XML entries, checked directly by
reading every `plugins/frontend/gui/*.xml` and `plugins/languages/*.xml` entry
in all five front-end-bearing archives and classifying by where its first
non-zero byte falls: `0` is "clean" (parses top to bottom), `all-zero` means
every byte in the declared range is `0`, and any other offset is "garbage" -
real, legible XML text, just not starting where the archive says the entry
starts (the same failure `campaign2048_definition.xml` shows below).

| Archive | frontend/gui XML | clean | garbage | languages XML | clean | garbage | all-zero |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `data00.psarc` | 10 | 8 | 2 | 5 | 1 | 2 | 2 |
| `data05.psarc` | 1 | 1 | 0 | 10 | 2 | 3 | 5 |
| `data07.psarc` | 1 | 0 | 1 | 0 | - | - | - |
| `data08.psarc` | 9 | 9 | 0 | 5 | 1 | 2 | 2 |
| `data09.psarc` | 45 | 39 | 6 | 46 | 19 | 14 | 13 |

**`data09` reads best of all five for exactly the files this page needs**:
`skin.xml`, `mainmenu_definition.xml`, `cellmode_definition.xml` and 36 more
of its 45 `frontend/gui` files are clean from byte zero. Its 6 garbage
`frontend/gui` files are `campaign2048_definition.xml`, `stats_definition.xml`,
`selection_definition.xml`, `showunlocks_definition.xml`,
`credits_text_eu.xml` and `endrace_definition_demo.xml` - real text recovered
from an offset 3.5-18 KB into the declared range, never from the start. Every
`english/definition.xml` on every archive that carries one (`data00`, `data08`,
`data09`) is **all-zero**, not garbage - the language's own name-and-code table
is unrecoverable from this dump on all three copies, a clean negative result
rather than a gap in the check.

## `FEGlobals` is bit-identical to HD's

**Confidence 88.** Read directly out of `data09.psarc`'s `skin.xml`
(`<Variable global="...">` blocks), against
[hd-frontend.md's own headline table](hd-frontend.md#six-skins-one-layout):

| Global | Omega (`data09`) | HD (all six copies) | Match |
| --- | --- | --- | --- |
| `MenuXOffset` | `800` | `800` | yes |
| `MenuYOffset` | `300` | `300` | yes |
| `MenuScale` | `1.0` | `1.0` | yes |
| `TitleXOffset` | `194` | `194` | yes |
| `TitleYOffset` | `62` | `62` | yes |
| `TitleScale` | `1.0` | `1.0` | yes |
| `TextColor` | `0xFFFFFFFF` | `0xFFFFFFFF` | yes |
| `TitleColor` | `0xFF646464` | `0xFF646464` | yes |
| `MSWarningScale` | `1.0` | `1.0` | yes |
| `FMVFrameCount` | `2847` | `2847` | yes |

Ten of ten fields HD's own page names as authored globals match to the digit.
`MSWarningColour1`/`MSWarningColour2` - Memory Stick globals, dead since the
PSP, HD's own "lineage visible in the leftovers" finding - are still present
in Omega's `skin.xml` too, unchanged. This is not a coincidence of two titles
independently choosing the same numbers: it is one file lineage. Scored 88
rather than HD's own 92 for "six copies agree on this" because Omega gives one
copy to agree *with* HD's six, not six copies agreeing with each other - the
same shape of evidence, one comparison short of it.

Confirmed independently through this project's own parser (see
["Does this project's own parser read it"](#does-this-projects-own-parser-read-it-yes-for-the-globals-and-the-boot-chain)
below), which resolves the same ten values off the same file with no
special-casing.

## The colour-skin switch is restructured, not just copied

HD's own page found that `DATA00`'s and `DATA06`'s `skin.xml` each carry
**one** `<Screen name="HD_Colours">` block, and that the other four PS3
archives carry none at all - the Fury/base-HD split on PS3 is achieved by
**shipping a separate baked copy of the whole file per archive** (`DATA00`'s
`HD_Colours` block already holds Fury's palette; `DATA02`'s file has no such
block). Read directly:

```sh
python3 scripts/psarc.py cat 'data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC' \
    /data/plugins/frontend/gui/skin.xml | grep -A8 'Screen name="HD_Colours"'
```

gives `HD_Blue = 0xffac0717`, `HD_Grey = 0xff969696` - Fury's colours, baked
into the one block `DATA00` carries. `DATA02.PSARC`'s copy of the same file
has no `HD_Colours` block at all.

**Omega's `skin.xml` folds this into one file with three named colour-template
`<Screen>` blocks**, `HD_Colours`, `WOHD_Colours` and `Fury_Colours` -
`HD_Colours`'s own comment says why: *"THESE COLOURS ARE IGNORED AND
OVERWRITTEN by the WOHD_Colours and Fury_Colours templates below"*.
`WOHD_Colours`'s values (`HD_Blue = 0xffee0707`, `HD_Grey = 0xFF010001`) are
base-HD's; `Fury_Colours`'s (`HD_Blue = 0xffac0717`, `HD_Grey = 0xff969696`)
match `DATA00`'s baked Fury palette on the nose. **A fourth block,
`2048_Colours`, is new** - `Blue2048`, `Orange2048`, `White2048`,
`Transparent2048`, `Grey2048` - and is not a per-mode override of the same
`HD_*` names the way `WOHD_Colours`/`Fury_Colours` are; it is its own,
independently-named palette, consistent with 2048 drawing through its own
screens rather than through HD's `<Strip>`/`<Menu>` widgets recoloured.
Confidence 85: the *values* are read directly; which runtime condition
selects between the three `HD_*` blocks is not - the same gap HD's own page
leaves for `selected_fill` (no capture on Omega exists to watch the switch
happen).

## The boot chain is declared identically, unmeasured

**Confidence 75 (declared only - compare to HD's post-capture 85).** Read
redirect-for-redirect out of `data09.psarc`'s `skin.xml`: the same eight
screens in the same order as HD's `DATA00`/`DATA05`/`DATA06` family
(`hd-frontend.md`'s "two chain families" table) -
`Language Selection -> PreFMVConnect -> Studio Logo -> EpilepsyWarning ->
FirstPlay -> Save Warning -> EULA -> Update Announcement -> Main Menu` - and
**no `LogoFMV`** (the dead PSP-era screen HD's older `DATA02`-`DATA04` family
still carries). The dialog `TitleID`s are unchanged
(`BOOT_HEALTH_WARN_1`/`_2`, `FE_MSG_NEW_TO_WIPEOUT1`/`_2`, `FE_BOOT_AUTOSAVE`,
`ONL_CON_ANNOU`, `ONL_CON_EULA`), and the `Save Warning` screen's own footer
still reads the identical legal line, verbatim: *"WipEout® HD ©2008 Sony
Computer Entertainment Europe. WipEout is a registered trademark of Sony
Computer Entertainment Europe."* `Studio Logo`'s movie is
`Data/FE/Images/StudioLiverpool.bik` - the base name, not `_fury` - and
**`StudioLiverpool_fury.bik` does not appear anywhere in any of the nine
Omega archives' path listings**, unlike HD where it is the one string that
settled which PS3 archive was live. Whether Omega unified the two cuts into
one `.bik`, dropped the Fury-specific reel, or names it something else
entirely is not checked here - open below.

**Two screens are new and have no HD counterpart**: `VR Warning Locked To
Pilot Ingame` and `VR Warning Pilot Assist Ingame`, both
`<Dialog IsVR="true">` confirmation prompts reached from in-race rather than
from the boot chain proper. `skin.xml` also declares an `OPT_COMFORTVR`
options-menu entry (`FEGlobals->HD_Grey` styled, same as every other option
row), and `data08.psarc` carries `data/fe/frontendscene/frontendscene2_VR.vex`
- a second, VR-specific front-end 3D backdrop scene. None of this exists in
[hd-frontend.md](hd-frontend.md); it is PSVR support genuinely new to Omega,
not inherited. The non-VR scene, `FrontEndScene_HD_ATG.vex`, *is* HD's file and is
what Omega's menus draw - see [omega-status.md](omega-status.md)'s menu backdrop section.

## 2048's campaign: HD's screen shape, 2048's own map content

This is the one place a naive "re-authored into HD's vocabulary" answer would
be wrong, and it took reading both files to see why.

**The screen shape is HD's own `CellMode_Definition.xml` vocabulary,
extended by one branch.** `cellmode_definition.xml`'s `Campaign Selection`
screen redirects on `campaignList`:

```xml
<Entry item="campaignList" playGoChunk="1" equals="FE_RC_HD" goto="Grid Selection"></Entry>
<Entry item="campaignList" playGoChunk="3" equals="FE_RC_FURY" goto="Grid Selection Fury"></Entry>
<Entry item="campaignList" equals="FE_RC" goto="Grid Selection 2048"></Entry>
```

`Grid Selection` and `Grid Selection Fury` are `type="FlyerSelection"` screens
- the same type HD's own two campaign grids use, per
[hd-frontend.md](hd-frontend.md#the-declared-menu-tree)'s screen census (HD's
`cellmode_definition.xml` has 4 screens; Omega's has 5 - the extra one is
exactly this branch). **`Grid Selection 2048` is the same `FlyerSelection`
type**, not a new one, and it redirects onward to `Cell Selection 2048` -
a `CellSelection`-typed screen distinct from HD/Fury's shared `Cell
Selection`, so 2048 gets its own screen instance rather than reusing HD's.
`playGoChunk="1"`/`"3"` on the HD/Fury entries (absent on the 2048 one) are
new PS4 SDK attributes, consistent with
[`psarc-mount.md`](../ghidra/functions/ps4-omega-eu/psarc-mount.md)'s finding
that the executable drives a PlayGo streaming-install monitor per chunk -
plausibly gating HD's and Fury's own content behind separate install chunks
while 2048's ships unconditionally, though this page does not trace that
attribute's consumer.

**The campaign-map content one level in is 2048's own vocabulary, not HD's.**
[`2048-frontend.md`](2048-frontend.md#the-front-end-is-a-touch-icon-grid-not-feglobalsmenuskin)'s
`FE3DCanvas`/`CanvasLabel` widgets - the touch-icon map hotspots that replaced Vita 2048's entire menu -
reappear verbatim in a **new file**, `Campaign2048_Definition.xml`, which
`skin.xml`'s own `<LoadXML>` list loads as a sibling of `CellMode_Definition.xml`,
both under the unchanged `Data\Plugins\PI001\GUI\` path:

```xml
<LoadXML><Values Src="Data\Plugins\PI001\GUI\CellMode_Definition.xml" SrcRel="CellMode_Definition.xml"></Values></LoadXML>
<LoadXML><Values Src="Data\Plugins\PI001\GUI\Campaign2048_Definition.xml" SrcRel="Campaign2048_Definition.xml"></Values></LoadXML>
```

The recovered fragment of `campaign2048_definition.xml` (real text starts at
byte 18,269 of 29,919 - see the census above) is wall-to-wall `<CanvasLabel>`
widgets naming actual 2048 events by their original titles - `"Arena Speed
Lap - Super P"`, `"Mall Speed Lap - Flash"`, `"Cathedral..."` - the same
`linkedevent` attribute 2048-frontend.md documents. **This page did not trace
which screen inside `campaign2048_definition.xml` `Cell Selection 2048`
actually redirects to** - the file's own opening `<Screen>`/`<FE3DCanvas>`
tags sit inside the corrupted, unreadable prefix (bytes 0-18,268 are zero),
so the `<CanvasLabel>` fragment's parent element is not visible in this
extraction. Confidence 80 for "the file exists, is loaded, and carries 2048's
own map vocabulary"; the exact wiring from `Cell Selection 2048` into it is
inference from adjacency and naming, not a traced `goto`.

**2048's *other* own vocabulary - `NEWGUI`, `GameModeChoice`, standalone
`FE3DCanvas` screens - survives too, but as a live, separately-scoped
include, not the campaign path.** `vita_legacy.xml` (also `<LoadXML>`'d by
`skin.xml`, unconditionally, near the file's end) is a real `<FE3DCanvas>`-
bearing screen named `overshell`, full of `GameModeChoice`-targeted
`<UnityCon>` redirects and a `TouchScroll` widget - Vita 2048's own front-end
shell, kept as an included screen rather than deleted the way HD's `LogoFMV`
was. It is not reached from `Main Menu`'s own redirect (which only names
`Campaign Selection`/`Single Player`/`NetLoginPage`/`ScoreSync`/`Additional`),
so this looks like a companion-overlay or debug surface rather than the
primary campaign flow - not chased further this pass.

## `mainmenu_definition.xml`: same redirect, new logo, new PlayGo tags

`mainmenu_definition.xml`'s top-level `<Redirect>` is close to line-for-line
HD's own (`hd-frontend.md`'s ["declared menu tree"](hd-frontend.md#the-declared-menu-tree)),
including the same stray comment:

```xml
<Entry item="Mode" equals="FE_RC" goto="Campaign Selection"></Entry>
<Entry item="Mode" equals="FE_RACEBOX" goto="Single Player" playGoChunk="4"></Entry>
<?SA - Formerly went to Racebox ?>
<Entry item="Mode" equals="FE_ONLINE" goto="NetLoginPage" playGoChunk="4"></Entry>
<Entry item="Mode" equals="FE_RECORDS" goto="ScoreSync"></Entry>
<Entry item="Mode" equals="FE_OPT_PLUS" goto="Additional"></Entry>
<Default goto="To Be Done"></Default>
```

Three changes from HD's copy, all small: `FE_RC` now targets `Campaign
Selection` (HD's own went straight to `Grid Selection`, since HD has no
`Campaign Selection` layer - Omega's is the new top screen that then branches
by HD/Fury/2048 above); `Racebox`/`Online` carry `playGoChunk="4"`, the same
PlayGo attribute seen in `cellmode_definition.xml`; and an `OmegaLogo` image
(`Data\FE\Images\Wipeout_Omega_Logo.gtf`) replaces whatever HD's own logo
reference was. The screen carries 6 screens against HD's own 6
(`Main Menu`, `PurchaseGame`, `ProductInfoConnectingScreen`,
`ProductInfoScreen`, `ProductInfoErrorScreen`, `Demo Launch`) - same count,
one different member (`Demo Launch` where HD likely has none, not checked
against HD's own file directly this pass).

## Does this project's own parser read it? Yes, for the globals and the boot chain

**Deliverable 3, done rather than scoped.** A ten-line probe was cheap here:
[`crates/game/examples/omega_frontend_probe.rs`](../../crates/game/examples/omega_frontend_probe.rs)
reads a `.psarc` directly (`oag_assets::psarc::Archive`), runs
`oag_tables::fexml::is_fexml`/`expand` the same way
`crates/game/src/boot/screens.rs::load_screens` does, and parses the result
through `oag_ui::screen::Screens::from_xml` - no Omega-specific code, no new
dependency edge (`oag-game` already depends on both `oag-assets` and
`oag-ui`).

```sh
cargo run -p oag-game --example omega_frontend_probe -- \
    data/extracted/ps4/omega-eu-patch/uroot/data09.psarc
```

```
data/plugins/frontend/gui/skin.xml: 41150 bytes -> 27 screens, 74 globals, 24 LoadXML includes
  MenuXOffset = 800
  MenuYOffset = 300
  TitleXOffset = 194
  TitleYOffset = 62
  TextColor = 0xFFFFFFFF
data/plugins/frontend/gui/mainmenu_definition.xml: 9797 bytes -> 6 screens, 0 globals, 0 LoadXML includes
data/plugins/frontend/gui/cellmode_definition.xml: 69457 bytes -> 5 screens, 0 globals, 0 LoadXML includes
data/plugins/frontend/gui/campaign2048_definition.xml: 0 screens, 0 globals, 0 LoadXML includes
```

`skin.xml`, `mainmenu_definition.xml` and `cellmode_definition.xml` parse
clean with this project's existing, unmodified `Screens` reader - no fork,
no HD-only assumption breaks. `campaign2048_definition.xml` parses to zero
screens, exactly as the byte-level census predicts: its real content starts
past the file's own corrupted prefix, so there is no top-level `<Screen>`
element in what this reader is handed. **This is not a parser gap** - handing
the same reader the recoverable *fragment* would still find no `<Screen>`
open tag, because the corruption eats it.

What this does **not** show: a full `oag_title::MenuSkin` for Omega. That
needs an `oag-omega` title crate with its own `FrontEnd`/archive-candidate
wiring (`oag-title`'s three axes, per its own module docs) the way `oag-hd`
and `oag-pulse` have - out of scope for a docs-only sweep, and not attempted
here.

**That crate exists now** (`crates/omega`, landed 2026-09-21): `MENU_SKIN`
re-derived independently off `data09.psarc` rather than copied from this
page or from `oag_hd::frontend`, `BOOT_CHAIN` walked redirect by redirect the
same way, `data09` made the archive candidate that wins the load-order
question below by construction (chosen, not measured). `just play omega`
boots and stops on `Language Selection`; see
[`omega-status.md`](omega-status.md) for what draws, what does not, and why
racing stays out of scope. This page's own open questions - which archive
really wins a duplicate path, `campaign2048_definition.xml`'s corrupted
prefix, `StudioLiverpool_fury.bik`'s absence - are all still open; the crate
answers "does this project's parser and archive layer work at all", not any
of those.

## Load order is not settled

**No confidence assigned - this is the one open question HD's own equivalent
took an emulator capture to answer**, and no PS4 emulator capture exists in
this project. Two independent copies of `cellmode_definition.xml` exist
across the patch's own four archives - `data07.psarc`'s (garbage: real text
recovered from byte 35,404 of 69,013) and `data09.psarc`'s (clean from byte
zero, 69,457 bytes, five screens including `Grid Selection 2048`) - and they
are **not** byte-identical even past the corruption (`data07`'s recovered
tail differs from `data09`'s at every line checked). `data08.psarc` also
carries no `cellmode_definition.xml` at all, and `data05.psarc` carries
neither.

[`psarc-mount.md`](../ghidra/functions/ps4-omega-eu/psarc-mount.md)'s own
reading of `eboot.bin` explains the **base package's** five-chunk PlayGo
mount (`PsarcArchive_WaitAndMountAll` polling `scePlayGoGetLocus` for chunks
0-4) and says nothing about how the patch's `data05`/`data07`/`data08`/`data09`
layer on top, or in what order FIOS resolves a path that exists in more than
one mounted archive - that reading was done against the base `eboot.bin`
before the patch's own, newer `eboot.bin`
(`data/extracted/ps4/omega-eu-patch/uroot/eboot.bin`) was in scope. **Not
chased here**: this task's brief was explicitly read-only/no-Ghidra, and
settling this the way HD's was settled needs either a disassembly of the
patch's own `eboot.bin` for its FIOS mount-order logic, or a boot capture on
real hardware or a PS4 emulator (none integrated into this project's
tooling - `oag-trace`'s emulator list is PCSX2/PPSSPP/RPCS3, no PS4 backend).

The circumstantial case for `data09` being the live copy, same shape as the
evidence that once pointed at HD's `DATA00` before its own capture upgraded
it: `data09` is the only archive with a complete plugin/language set (91 of
120 entries are `plugins/*`, nothing else); it reads clean far more often
than the other four (see the census table above); and it is the
highest-numbered patch archive, consistent with (but not proof of) "newest
layer wins" the way HD's own `DATA00`-loads-last turned out to be. This is
**inference from geometry alone, explicitly the caution `hd-frontend.md`'s
own history is a warning against** ("`DATA00`... looking like it is not
evidence. Nothing here establishes load order." - written before that page's
own capture). Scored as **unconfirmed**, not assigned a number, rather than
guessed at 50-60 to look decisive.

## Open

- **Which archive wins when a path is duplicated across the patch's own four
  archives** - `cellmode_definition.xml` in `data07` vs `data09` is the
  concrete, checkable instance; needs either the patch `eboot.bin`'s own FIOS
  logic or a boot capture, neither done here.
- **`campaign2048_definition.xml`'s own opening screen/`FE3DCanvas` block** -
  unreadable in this dump (block-location corruption eats bytes 0-18,268 of
  29,919); what screen name it declares and what widget wraps the recovered
  `<CanvasLabel>` fragment is unknown.
- **Whether `StudioLiverpool_fury.bik` exists anywhere in Omega under a
  different name**, or the Fury/base distinction lost its separate movie
  entirely - not checked past "the old name isn't in any path listing".
- **`Controls_Definition_PS4.xml` vs `_ps3.xml`'s actual diff** - both are
  loaded (`skin.xml`'s own `<LoadXML>` list carries `Controls_Definition_PS4.xml`
  only; `_ps3.xml` is in the file listing but its own load site was not
  found this pass) and both read clean; not diffed field-by-field.
- **`vita_legacy.xml`'s actual role** - confirmed live (`<LoadXML>`'d
  unconditionally) but not confirmed reachable from any menu path a player
  would take; "companion overlay or debug surface" above is a hypothesis at
  confidence 50, not a finding.

## Next steps

- ~~An `oag-omega` title crate is the real unlock~~ - landed 2026-09-21, see
  [`omega-status.md`](omega-status.md). Its own next steps: a `.gnf` reader
  (small if GNF really is a header over a tiled format `oag_texture` already
  decodes - not investigated this lane), and naming `campaign.rs`'s own hex
  textures through the same `.gnf`-sibling report the front-end sheet gets.
- A PS4 emulator (if one is ever added to this project's toolchain) doing for
  Omega what RPCS3 did for HD - settle load order and the boot chain's
  `Provenance` in one pass, per [ADR-0025](../architecture/adr/0025-a-boot-chain-carries-its-provenance.md).
- Extracting past `campaign2048_definition.xml`'s corrupted prefix - either a
  different extraction/repack of the same `.pkg` pair, or a targeted look at
  whether the corruption pattern the census found (real bytes some KB into
  the declared range) has a fixed relationship to the entry's own block table
  that `oag_formats::psarc` could compensate for generically, the way the
  short-stored-block fix in `psarc.md` did for a different symptom of the same
  family.

## See also

- [HD front end](hd-frontend.md) - the page this one is a direct comparison
  against, field by field.
- [2048 front end](2048-frontend.md) - `FE3DCanvas`/`CanvasLabel`/`linkedevent`,
  the vocabulary `campaign2048_definition.xml` carries forward.
- [`psarc.md`](psarc.md#the-ps4-omega-collection-family) - the container-level
  block-data-location trap this page's census applies to XML rather than
  binary formats.
- [`docs/ghidra/functions/ps4-omega-eu/psarc-mount.md`](../ghidra/functions/ps4-omega-eu/psarc-mount.md) -
  the base package's own PlayGo mount reading, and why it does not settle the
  patch's load order.
