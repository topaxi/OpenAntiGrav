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
the axis `settings.display.front_end_style` offers. **Unpicked, the styling is the one the menus are drawn in**,
not the first entry: the menus follow the ground of the archive the source serves (Fury's black and red on
`DATA00`, which the EU disc serves), so `oag_title::FeatureStyle::dark_ground` names that match, and an empty
setting resolves to the style whose ground it is (`oag_game::loading::Assets::style` records the choice; the
OPTIONS row reads the same). A saved name wins. The whole screen follows it: pictures, mode icon's clip and the
four globals. A change made in OPTIONS reaches the next load without a restart (`apply_setting` re-reads the
assets from the new setting). The first entry used to be taken, which put HD's white art on Fury's black
page for every player who never opened OPTIONS. Asserted in
`every_feature_illustration_is_on_the_disc_in_both_stylings`, which checks all
ten resolve and that the Fury name is the base name plus a suffix, and
`an_unpicked_style_is_the_one_the_menus_are_drawn_in`, which checks the deck, the mode icons and the globals
against the menus' own ground.

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

## What this build draws

**The screen is a file**, since 2026-11-01: `assets/fallback/Data/XML/LoadingScreen_HD.toml`
([screen-layout.md](screen-layout.md), [ADR-0081](../architecture/adr/0081-screen-layouts-are-files-and-a-track-may-carry-its-loading-screen.md)),
named by `oag_title::Loading::layout`, with no layout number left in code. Its numbers are read off
`LoadingScreen_Draw` (`0x002b61c8`) and its drawing helpers, with their addresses and confidences in the
file's comments and in
[loading-screen.md](../ghidra/functions/ps3-hdfury-eu/loading-screen.md), section "2026-11-01: the race load's
layout". They are **the original's own 1920x1080 grid, drawn in it**: the renderer is given that grid, fonts
are the front end's roles at the scale the code stores (`Title` 1.0 for the caption, `Default` 1.0 for the
paragraph, 0.8 for the feature's name row, 0.45 for the labels), and the player's Safe Area scales the grid
about its centre. The earlier build drew this screen in the PSP's 480x272 with every face divided down to a
13 px line height, which is what made its text larger and its rule sit at y 209 against 133 of 1080.

What is drawn, in the file's order: two rules (`line.gtf`, 1600x8 at y 110 and 975), the caption arrow and
`LOADING... <circuit>` in the `Title` face, five labels behind `square.gtf` bullets, bracket corners round the
four panels, the feature's picture (a 1024x512 texture's rows 0 to 480, 1:1 at (180, 190)), the feature's name
and its heading each behind an arrow, the paragraph (wrap 780, pitch 40), the 166 x 30 dot bar and the mode
icon's clip in a 250 px square. Every one of those is the original's number.

Departures, each deliberate:

- **The picture is cut to its panel when its ground would show outside it.** The window is 896x480 at
  (180, 190), which reaches x 1076 and y 670 against the panel's 160 to 950 by 155 to 655. Both stylings' pictures
  are opaque to the edge: Fury's ground is the screen's own black, so the overhang shows nothing and the picture
  is drawn as the layout places it, to the bit; HD's ground is white, and drawn whole it fills the padding and
  covers the description beside it, at every screen size (the grid is one 1920x1080 drawn under the Safe Area, so
  the shape of the screen only letterboxes it). `Frame::fit_to_panel` cuts the quad and its texels at the panel's
  edge when anything that differs from the ground lies outside. **Chosen, not measured**: how the original treats
  the overhang in the HD styling is unread (no RPCS3 frame of the HD styling's loading screen). Tests:
  `a_picture_with_a_showing_ground_lies_inside_its_panel_on_every_screen_shape` (1280x800, 1920x1080, 2560x1080
  and a phone's 20:9) and, on the disc's art, `an_unpicked_style_is_the_one_the_menus_are_drawn_in`. Still open:
  in the HD styling the white ground also covers the lower half of the feature's name row (y about 190 to 220).
- **The mode icon plays for one mode only.** The panel plays a Bink clip, and an RPCS3 frame of a
  `GetMode()==3` race matches `single_race_icon_fury.bik` (see "The pair"). Only that mode is mapped
  (`oag_hd::loading::MODE_ICONS_FURY`); every other mode id draws the panel's label and brackets and no icon,
  rather than a clip guessed from a file name. The base styling's `single_race_icon.bik` is the same name
  without `_fury` and **chosen, not measured** (a different file, 250 px square against the Fury clip's 400,
  so it is stretched to the same 250 quad).
- **The bar's fill is driven by the race load's stages**, since 2026-10-06: a first target of 0.2, then 0.4,
  0.5, 0.75 and 0.95 as `race::load` reaches `TrackRead`, `TrackBuilt`, `WorldBuilt` and `CraftsBuilt`
  (`oag_hd::loading::PROGRESSION`), eased at 0.1 column a frame with the rate doubled by each milestone that
  lands while the bar is behind, see loading-screen.md. **Chosen, not measured**: the inner two stages'
  places in this build's load order, and the whole bar the moment the work is over (this build's load ends
  long before the ease catches up). A title with no `progression` keeps the time estimate
  (`loading::wording::estimated_fraction`).
- **Four rows under the bar** (`counts`, `percentage`, `current`, `step`) are this build's own waits (a
  transcode, a prefetch, the boot's movie loads), which the original has no counterpart for: **chosen, not
  measured**, and drawn only while something is counted, so a race load draws none.
- **A paragraph breaks a word when the line and the space after it would pass the width.** Not in the
  executable: read off the Barrel Roll frame, where "with" would end 5 px inside the edge and the original
  breaks before it while "cost" stays (inferred from two lines of one frame).
- **Unread, so not drawn:** the second `Title` pass at the caption's point (a border or shadow), the
  tournament screen (type 1) and kind 8 (`+4 == 8`), whose blocks are `0x002b778c` and `0x002b6a64`.

A **mod overrides the file by path**, and a **mod track supplies its own** (layout, caption, picture, feature
texts) from its manifest's `[loading]` table; the rules are in [screen-layout.md](screen-layout.md).

The **five labels** are `WIPEOUT® HD` (a literal of the executable's, now `text` in the file) and four string
ids `LoadingScreen_BuildText` resolves (`string_id` in the file), all placed by `LoadingScreen_DrawLabel`.
This page used to call them the game's debug overlay; they are not.

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

## The pair (2026-11-01, `hd-loading-screen`)

The original: RPCS3 cold boot, the Racebox path to Team Selection (Feisar) then
Cross, grabbing the window at 1920x1080 about 1 s apart (`TTY.log`:
`Team Launch Transition`, `Launch Game`, `InGame`, `GetMode()==3`, Vineta K,
`Feature type` Barrel Roll). HD's log names no loading screen, so the screen
name of this pair is declared. Ours: `oag-game ... --loading-screen 0/1
--loading-step race --ticks 325 --mode single_race --track
"Data\Environments\01_Vineta_K\track.vex" --size 1920x1080`, `front_end_style
= FURY`, `aspect = wide`.

Measured off those frames (1080 px tall): the dot rows run y 703 to 929 and the
closing rule is at y 960; the mode clip (400 px) is drawn at 0.6, 240 px square
with its top left at (1488, 694); the clip loops (two grabs 2 s apart are on
different beats) and builds the ring logo up, then shows a grey ship at its
corner.

Share of pixels differing by more than 40 grey (luma, 1920x1080), original
frame 3 and frame 6 against ours:

| ours | vs frame 3 | vs frame 6 |
| --- | ---: | ---: |
| before (21 rows, no icon) | 11.91 % | 12.41 % |
| 30 rows and the bar and rule placed as measured | 10.33 % | 10.55 % |
| plus the mode icon clip | 10.45 % | 10.56 % |

Noise: original frame 2 against 3 is 0.05 %, 3 against 6 0.15 %, ours against
itself 0.08 %. So the 30 rows and their placement are worth about 1.5 points,
and **the icon is inside the noise**: a 240 px square of mostly black is under
0.3 % of the frame, and our clip phase and bar fill are not locked to the
original's. The remaining 10 % is the heading, feature and prose layout: ours
draws the heading and prose larger and the illustration about half the
original's size, the title rule at y 209 against 133. That was this build's
code layout, and is what 2026-11-01's `hd-loading-layout` replaced:

| ours | share > 40 grey vs the promoted pair `hd-loading` |
| --- | ---: |
| the code layout (480x272, PSP line height) | 10.52 % |
| `LoadingScreen_HD.toml`, the measured layout in its own grid | 0.76 % |

(The pair is a Racebox single race on Vineta K, Feisar, Barrel Roll, photographed 4 s after the load starts,
`scripts/rpcs3-hd-menu-pair.py --walk loading`, 96 % Safe Area; ours is `--ticks 325` with
`--loading-draw` picking the original's feature (the bar: next section). What remains is glyph rasterisation and the clip's and the
bar's phase, which are not locked between the two.)

### The bar's stated state (2026-11-12)

The original's frame shows **32 lit columns of 166** (19.3 %; counted as the runs of `HD_Blue` dots between x 193 and the
mode icon, the last lit dot ending at x 443 of 1920). `LoadingScreen_Draw` lights `(int)obj+0x104` columns, and ours lights
`floor(Fill::shown)` (`loading::fill`), so the counts compare directly. 32 is below the first target (33.2, `0x002b9a28`),
so the bar had only eased at 0.1 column a frame and no load stage had landed: a state ours reaches without a load.

**`--loading-screen DONE/TOTAL` does not state it.** On `--loading-step race` the counts are zeroed (`main/headless.rs`),
so the stated state is the tick: `lit = floor(0.1 t)`, capped at 33. `t = 10 n + 5` lights `n` columns with half a column of
margin either side of the f32 sum's edge at `10 n` (ticks 320 to 330 all light 32; 319 lights 31, 331 lights 33). A
`--loading-live` capture instead runs a real load, whose stage timing and finish vary run to run (two consecutive
re-renders gave different `ours.png`); the pair is at `--ticks 325` and two re-renders are byte-identical.
`scripts/rpcs3-hd-menu-pair.py --walk loading` counts the columns off every frame it photographs and refuses a bar past
the first target, since a stated race screen cannot show one. Open: the mode icon's clip phase is the tick's, and the original's is not
locked to anything we can read, so the ship at the icon's corner differs.

**2048: checked, differs** ([2048-loading.md](2048-loading.md)): the same class, strings, deck and layout operands in a kind
the retail executable never starts (it starts its kind 4 for a race load), so nothing is wired.

**The white ground over the name row, fixed 2026-11-06.** The draw order was not the cause: the picture is drawn before
the name and heading rows in 2048's `LoadingScreen_Draw` (the same order as ours; HD's heading row, `0x002b7738`, follows
its picture, `0x002b76d8`, the name row's call site was not located) and the rows draw over it. What showed was the HD
styling's white pictures on the served archive's black ground: the four globals came from the served front end's `Skin.xml`
(the dark `DATA00`), whichever style was chosen, so the white picture met a black ground mid-row (y about 190 to 220). The
loader now takes the globals from the copy of the front-end file whose `HD_BG` matches the chosen style's ground
(`DATA06` for HD, white; `Assets::load`, `loading screen: HD style takes its colours from ...`). Fury, which matches the served
copy, is unchanged. Test: `a_style_takes_the_ground_its_own_pictures_were_drawn_for`. A style with no matching copy keeps the
served colours and says so in the loader report.

**Omega: ported.** Omega's loading screen is this one with `.gnf` files, a still for the mode icon and a string-id brand
([omega-loading.md](omega-loading.md)); the unpicked-style rule and the panel cut are title data and a layout rule, and
carry over (Omega has one styling).

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
