# Wipeout HD's loading screen

**What it is: a caption naming the circuit, one of five illustrated features,
and a progress bar.** Confidence **90**, and it is the first page in this tree
whose central claim was *checked against the game running* rather than only
against the disc — see [What the emulator settled](#what-the-emulator-settled),
which is also where the reading this page used to carry was disproved.

The reimplementation is `oag_hd::loading`, wired through the
[`oag_title::Loading`](../architecture/workspace-layout.md) axis and drawn by
`oag_game::loading::Screen`.

## The screen, as it draws

Read off a screenshot of a running Fury race, with the labels the game itself
prints over each region:

```text
> LOADING... VINETA K
-------------------------------------------------
 FEATURE IMAGE            | FEATURE DESCRIPTION
 > PILOT ASSIST           | > DESCRIPTION
 [ illustration ]         | Pilot Assist can aid your navigation by
                          | steering you away from the track sides...
 PROGRESSION BAR                              | MODE ICON
 [###...........................]             |  arc
```

Every element of it is named in `EBOOT.elf`, in one run of strings that ends
with the thread that draws them:

```text
Data/FE/Images/corner.gtf              <- the bracket corners round each panel
Data/FE/Images/dot.gtf                 <- the dotted fill of the progression bar
Data/Fe/Images/square.gtf
Data/Fe/Images/line.gtf                <- the horizontal rules
Data/Fe/Images/Title_Arrow_HD.gtf      <- the marker before LOADING...
Data/Fe/Images/Subtitle_Arrow_HD.gtf   <- the marker before PILOT ASSIST
Data/Fe/Images/Barrel_Roll_fury.gtf
Data/Fe/Images/Barrel_Roll.gtf
Data/Fe/Images/Side_Shift_Tap_fury.gtf
Data/Fe/Images/Side_Shift_Tap.gtf
Data/Fe/Images/Pilot_Assist_fury.gtf
Data/Fe/Images/Pilot_Assist.gtf
Data/Fe/Images/Absorb_fury.gtf
Data/Fe/Images/Absorb.gtf
Data/Fe/Images/Flip_fury.gtf
Data/Fe/Images/Flip.gtf
LoadingScreenThread
Loading Screen Finished
/Data/FE/Ads/LSAD_%s_16x9_01.gtf ...
```

That run **is** the screen's asset list. Reading it as anything narrower is how
this page got it wrong the first time.

### The caption is the disc's own, and carries the circuit

`LOADING... VINETA K` is `FE_LOADINGDOT` followed by the circuit's own name out
of the same string table — which `oag_raceplay::catalogue::label` already resolves,
by the copy-selection this project had to work out separately (see
[hd-frontend.md](hd-frontend.md)). So the heading needs no new recovery: two
things already in hand, joined the way the original joins them. Confidence
**92**; the string is verbatim on screen and every one of the disc's sixteen
languages carries the id.

`MSC_LOADING` is deliberately **not** this: it belongs to
`AutoLoadingProfileScreen` in `frontend/gui/memorystickbootscreens_psp.xml`.

### Five features, in two stylings, all ten on the disc

| index | feature | image (base / Fury) | title id | heading id | description id |
| ---: | --- | --- | --- | --- | --- |
| 0 | Barrel Roll | `Barrel_Roll[_fury].gtf` | `MAN_2_BR` | `FE_INSTRUCTIONS` | `FE_BR_INST` |
| 1 | Side Shift | `Side_Shift_Tap[_fury].gtf` | `MAN_2_SS` | `FE_INSTRUCTIONS` | `FE_SS_INST` |
| 2 | Pilot Assist | `Pilot_Assist[_fury].gtf` | `FE_PILOT_ASSIST` | `ONL_CON_DESC` | `FE_PA_INST` |
| 3 | Absorb | `Absorb[_fury].gtf` | `IG_HUD_ABSORB` | `FE_INSTRUCTIONS` | `FE_ABSORB_INST` |
| 4 | Flip | `Flip[_fury].gtf` | `FE_FLIP` | `FE_INSTRUCTIONS` | `FE_FLIP_INST` |

**Every cell is read off the executable** (confidence 92), no longer inferred
from namesakes: the illustration loader and the text builder each index their
own TOC slots by the one `Feature type` field, and
[loading-screen.md](../ghidra/functions/ps3-hdfury-eu/loading-screen.md) lists
the slots. The three title ids this page used to leave unrecovered were never
`FE_`-namespaced - the screen reuses `MAN_2_BR`, `MAN_2_SS` and `IG_HUD_ABSORB`.
`ONL_CON_DESC` is the heading over Pilot Assist's paragraph, which is why that
frame reads `DESCRIPTION` and the others `INSTRUCTIONS`.

Two live RPCS3 frames pin the index from outside: `Feature type == 1` over a
Side Shift screen and `== 2` over Pilot Assist, each against its own boot's
screenshot.

**The `_fury` half is a styling, not different content.** `OPT_FE_STYLE` offers
exactly `HD` and `FURY`, and the disc ships every illustration twice -
white-and-blue and black-and-red - with the same strings behind both. That is
the axis `settings.display.front_end_style` offers, defaulting to the base
game's. Asserted in
`every_feature_illustration_is_on_the_disc_in_both_stylings`, which checks all
ten resolve and that the Fury name is the base name plus a suffix.

### The feature is a random draw, and the deck is the race mode's

Read out of the screen's own constructor -
[loading-screen.md](../ghidra/functions/ps3-hdfury-eu/loading-screen.md).
`Feature type == %i` reports `object+0x444`, set from `rand()` reduced modulo a
range the mode picks, and the range is a **set of indices**, not always a
prefix:

| mode id | what it is | indices |
| ---: | --- | --- |
| `8`, `0x14` | `SPElimination`, `MPElimination` | `0 1 2 3 4` |
| `0xd`, `0x15` | unnamed, `MPArcade` | `0 1 3` |
| `0xe` | unnamed | `1` |
| `6` | unnamed | `0 1` |
| otherwise (`3` `SPArcade`, `5` `SPTimeTrial`, `4` `SPTournament`) | | `0 1 2 3`, or `0 1 2` without the Fury content |

The last row's width is a byte at `0x00b979fd`, the **Fury-content flag**
(`00 01 00 00` at `0x00b979fc` read live on the EU Fury disc, so four).
`FUN_006762f8` is `rand()`.

**This build draws from the same deck.** `oag_hd::loading::DECK` carries the
law, and `Screen::for_mode` draws uniformly from the mode's set with a seed
the run's race count varies - a counter rather than a clock, so a run is
reproducible. Tests in `loading/deck_tests.rs` fail if the draw ignores the
mode: a single race never shows Flip, Eliminator shows all five, `0xd` skips
Pilot Assist.

**Chosen, not measured:** Speed Lap and Zone have no named executable id (they
are among the seven unnamed ones), so they draw from the default deck. The
source's Fury flag is `true` for every HD source this build opens, which is
the value measured on the one disc in hand.

### The four colours are the disc's, and their roles are usage counts

The same constructor resolves `FE_HD_BG`, `FE_HD_Grey`, `FE_HD_Blue` and
`FE_HD_LightGrey` through the palette lookup and keeps the four together. Those
are `FEGlobals` in `skin.xml`, so `oag_hd::loading::PALETTE` names them and
`oag_game::loading::Assets` reads their values off the source itself.

**The names mislead and the values say so**, which is what an early wiring of
this got wrong. Each palette names its colours relative to its own ground:

| global | `DATA00` (served) | `DATA06` |
| --- | --- | --- |
| `HD_BG` | `ff000000` black | `ffffffff` white |
| `HD_Grey` | `ff969696` | `ff646464` |
| `HD_Blue` | `ffac0717` red | `ff8ac0ca` blue |
| `HD_LightGrey` | `7f646464` half-alpha | `ffdedede` |

So "light grey" is the *dark, translucent* one on the archive this disc serves,
and reading it as the text colour drew a heading dimmer than its own body text.

What settles the roles is how often the original's draw function reads each, per
screen variant: `HD_BG` **once**, `HD_Grey` about **ten** times, `HD_Blue`
**once**, `HD_LightGrey` **once**. One ground, one workhorse and two
single-element colours — and `HD_Grey` is the only one of the four that is a
legible ink against both grounds. So everything written and every mark drawn is
`HD_Grey`, the accent is `HD_Blue`, and `HD_LightGrey` is the **unfilled dots of
the progression bar** (confidence 88, raised from 55 on 2026-10-04): a live
frame caught during the fade-in shows the whole bar as dark translucent dots
and the filled part as `HD_Blue` dots on the same grid.

Confidence **80** on the rest of the roles, up from 60 when they were a reading
of a screenshot alone. See
[loading-screen.md](../ghidra/functions/ps3-hdfury-eu/loading-screen.md).

### Two of the five descriptions are in one copy of the string table only

`FE_ABSORB_INST` and `FE_FLIP_INST` — Absorb and Flip, the two Fury mechanics —
are absent from the served `entries.xml` and present in `DATA06`'s. Every copy
carries the other three.

**That is the circuit-name finding arriving a second time from a different
direction.** `DATA06` is the copy that carries all 28 circuit names too (see
[hd-frontend.md](hd-frontend.md)), and it is now needed twice for two unrelated
reasons — which is the strongest evidence yet that it is the complete, Fury-era
table. `oag_game::loading::Assets` picks the copy that resolves every
description, the same way `CircuitNames` picks the one that names every circuit,
and reports which it took. Without it this screen offers three features on a
disc that ships five.

## A `.gtf`'s rows run bottom-up, and nothing 2D knew

Found by drawing this screen and comparing it with the original: the
illustration came out **inverted**. `Pilot_Assist_fury.gtf` decoded straight
into a sprite sheet draws the craft upside down, and reversed it matches the
screenshot exactly.

**The decoder is not what is wrong.** Every other consumer of
`oag_texture::gtf` is a 3D one - `mesh::rcs::skin`, `mesh::sky_cube`,
`race::assets`'s trail noise - and those are already right: the hull lettering
on an HD craft reads the correct way up, which is the check
[rcsmodel.md](rcsmodel.md) used to confirm the texture coordinate in the first
place. Their sampling convention and the file's row order agree. So the row
order is a property of the file and which way up a consumer wants it is the
consumer's, and the flip belongs in `oag_hud::sprite::Image::decode_gtf`,
where a sheet is built top-down.

**It was not only this screen.** `Sheet::build` is also what the front-end
sprite sheet and the in-race HUD atlas go through, so on Wipeout HD both were
inverted too and nobody had noticed - the front end's three `.gtf`s are a
stretched 8x8 rule, a vertically symmetric arrow and an icon nothing draws yet.
The PSP and PS2 paths are untouched: `.mip` and `.pct` go down different
branches of the same function.

## The code kept Pulse's loading plugin and the data was cut

HD's executable still carries the whole of Pulse's loading-plugin machinery.
`FrontendRoot.cpp`'s plugin list is in `EBOOT.elf` verbatim:

```text
Data\Plugins\frontend      Data\Plugins\billboards    Data\Plugins\grids
Data\Plugins\music         Data\Plugins\loading       Data\Plugins\news
```

`PI_LoadingScreen` and `PI_LoadingScreen_Item.cpp` are in there too — the node
kind and the source file behind Pulse's 26 rotating tips.

**Three of those six directories are not on the retail disc**: `music`,
`loading` and `news`. And this is the one part of the page that needs no
inference at all, because the running game says it out loud on every boot:

```text
FileSystem::Open FAILED for 'Data\Plugins\loading\Definition.xml'
```

So HD's tips were cut with their plugin, and the features replaced them. Nor is
there a glow strip: `LoadingPulseOverlay.mip` is a Pulse entry and is on no HD
archive. **This title draws no wave and no tip.** Confidence **95** — a manifest
count, a pair of misses, and the game's own complaint.

`/Data/FE/Images/WP_HD_DEMO_Loading_screen1.gtf` is the same story from the
other end: a loading still named by the executable, absent from retail, named
for the demo build.

## What this build draws, and where it departs

The arrangement is the original's, read off the screenshot as *fractions* of its
frame and multiplied into this build's 480x272 grid: a marked title over a rule,
the illustration in a bracketed panel on the left, the feature's name and
paragraph in another on the right, a wide bar low down, and a closing rule. The
marks are the disc's own - `Title_Arrow_HD`, `Subtitle_Arrow_HD`, `line`,
`corner`, `dot`, all five in `DATA02` and all five named in the same run of the
executable as the illustrations.

Three departures, each deliberate:

- **The mode icon is not drawn.** The original puts an animated Bink clip
  chosen by the mode in that panel; this build draws the panel's label and
  brackets and no icon, rather than a stand-in.
- **The bar has 21 rows, not 30.** It is 166 columns of `dot.gtf` tiled by
  `Draw::TiledSprite`, as the original is (measured, see below), and the lit
  columns are whole columns in `HD_Blue` over `HD_LightGrey` ones. The row count
  is chosen to fit this layout's flatter bar, not measured.
- **The bar's fill is driven by the race load's stages**, since 2026-10-06:
  a first target of 0.2, then 0.4, 0.5, 0.75 and 0.95 as `race::load` reaches
  `TrackRead`, `TrackBuilt`, `WorldBuilt` and `CraftsBuilt`
  (`oag_hd::loading::PROGRESSION`), eased at 0.1 column a frame with the rate
  doubled by each milestone that lands while the bar is behind - the original's
  own law, measured, see
  [loading-screen.md](../ghidra/functions/ps3-hdfury-eu/loading-screen.md).
  **Chosen, not measured**: the inner two stages' places in this build's load
  order, and the whole bar the moment the work is over (this build's load ends
  long before the ease catches up). A title with no `progression` keeps the
  time estimate (`loading::wording::estimated_fraction`). `oag-game
  --loading-screen 0/1 --loading-step race --loading-live --track <entry>
  --ticks N --screenshot out.png` runs a real load, paced at 60 Hz, and draws
  the frame at tick N.

**The five labels are drawn** since 2026-10-05: `WIPEOUT® HD`, `FEATURE IMAGE`,
`FEATURE DESCRIPTION`, `PROGRESSION BAR` and `MODE ICON`, each behind a
`square.gtf` bullet. This page called them the game's debug overlay; they are
not - four are string ids `LoadingScreen_BuildText` resolves and the fifth is a
literal in the executable, all placed by `LoadingScreen_DrawLabel`.

Since 2026-10-04 the feature's **name sits over the illustration and its
heading (`INSTRUCTIONS` or `DESCRIPTION`) over the prose, in one row**, as in
the original; before that the name was drawn alone over the prose and no
heading was drawn, because its id was unread.

And the panels are two columns of this build's proportions rather than the
original's exact halves: our grid is 480 wide where HD's is 1920, and prose in
the right-hand half of a 480-wide screen wraps to a column three words across.
The *relationship* is kept - picture left, prose right of it, bar under both.

## What the emulator settled

This page previously said the loading screen **was** the
`Data\FE\Ads\LSAD_*.gtf` stills — twelve 1024x512 DXT1 hoardings with a blank
grey panel, sitting beside `LoadingScreenThread` in the executable, all twelve
on the disc and decoding cleanly. Every fact in that reading was true and the
conclusion was wrong: the run of strings around them is the *whole screen's*
asset list, so the ad is one element of the screen rather than the screen.

Driving the real game under RPCS3 settled it in four lines of `TTY.log`:

```text
LOADING SCREEN TYPE type == 0
Feature type == 2
DIGA State "STATE_PRE_ADVERT"
AdServer Not Found
```

`DIGA` is Double Fusion's in-game advertising, whose SPRX the same executable
loads. With no ad server the frame is **never drawn** — the screen is the
caption, the feature and the bar, and then a flyby of the loaded circuit with a
`START RACE` prompt. `/USRDIR/GameAd.bik`, the served video that filled the
blank panel, is **not on the disc**: it was fetched at runtime by a service that
has not answered since Sony withdrew the ads in August 2009.

So `oag_hd::loading::BACKDROP` and `BACKDROP_VARIANTS` stay as a record of the
ad slot, and nothing draws them. The lesson is worth keeping with them: a run of
strings in an executable is a *neighbourhood*, and taking the part of it you
recognise for the whole is how a well-evidenced reading lands on the wrong
object.

## Looking at it

The recovered screen, with no window:

```sh
cargo run --release -p oag-game -- data/images/hdfury-ps3-eu-dec.iso \
    --loading-screen 0/0 --loading-step race --ticks 0 \
    --track "Data\Environments\Talons_Junction\track.vex" \
    --screenshot /tmp/hd_loading.png
```

`--ticks 0` matters: a `DONE/TOTAL` whose halves are equal reads as finished,
and a finished screen is fading, so any later tick captures it part-way out.
`display.front_end_style` in `settings.toml` — or the FRONT END STYLE row on the
DISPLAY page — chooses which styling is drawn.

The original, for comparison:

```sh
uv run --with evdev python3 scripts/rpcs3-drive.py \
    --image data/images/hdfury-ps3-eu-dec.iso race --shots --load-shots 14
```

See [rpcs3-debugger.md](../reverse-engineering/rpcs3-debugger.md) for the traps —
including the stale `RPCS3.buf` that makes every run after a scripted one hang
on a modal dialog.

## Still unread

- **Seven of the twenty-two mode ids** - `0`, `6`, `7`, `11`, `13`, `14`, `15`,
  the ones with no `ModeManager` of their own. Three of them pick a feature
  range, so they are real modes rather than gaps.
- **`LOADING SCREEN TYPE type == %i`**, observed only at 0. It sits with
  `scePresents.gtf`, the twelve `presents_<language>.gtf` files and the cut demo
  still, so more than one type exists and the presents screens may be among
  them.
- **Whether the original holds the screen for the bar.** The done threshold
  (`149.4` columns) is read, what waits on it is not.
- **The Fury-content flag's writer**, whose value is read (`1`) and whose
  readers are four; see loading-screen.md for the next addresses to try.
