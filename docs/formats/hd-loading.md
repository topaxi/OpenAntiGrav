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

Read off a screenshot of a running Fury race, with the widget names its own
debug overlay prints:

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
of the same string table — which `oag_game::catalogue::label` already resolves,
by the copy-selection this project had to work out separately (see
[hd-frontend.md](hd-frontend.md)). So the heading needs no new recovery: two
things already in hand, joined the way the original joins them. Confidence
**92**; the string is verbatim on screen and every one of the disc's sixteen
languages carries the id.

`MSC_LOADING` is deliberately **not** this: it belongs to
`AutoLoadingProfileScreen` in `frontend/gui/memorystickbootscreens_psp.xml`.

### Five features, in two stylings, all ten on the disc

| feature | image (base / Fury) | title id | description id |
| --- | --- | --- | --- |
| Barrel Roll | `Barrel_Roll[_fury].gtf` | *unrecovered* | `FE_BR_INST` |
| Side Shift | `Side_Shift_Tap[_fury].gtf` | *unrecovered* | `FE_SS_INST` |
| Pilot Assist | `Pilot_Assist[_fury].gtf` | `FE_PILOT_ASSIST` | `FE_PA_INST` |
| Absorb | `Absorb[_fury].gtf` | *unrecovered* | `FE_ABSORB_INST` |
| Flip | `Flip[_fury].gtf` | `FE_FLIP` | `FE_FLIP_INST` |

The order is the executable's, not this project's arrangement. `FE_PA_INST` is
**verbatim** the paragraph in the screenshot, which is what fixes the whole
column: the other four are its immediate namesakes in the same table.

**Three title ids are left unrecovered rather than guessed.**
`FE_PILOT_ASSIST` and `FE_FLIP` are unambiguous — right namespace, exact words.
There is no `FE_BR`, `FE_SS` or `FE_ABSORB`; the strings holding the right words
are `MAN_2_BR`, `MAN_2_SS`/`OPT_CTRL_SS` and `IG_HUD_ABSORB`/`MSC_ABSORB`, and
choosing among them would put a name on screen this project cannot vouch for. A
feature with no title id draws its description alone.

**The `_fury` half is a styling, not different content.** `OPT_FE_STYLE` offers
exactly `HD` and `FURY`, and the disc ships every illustration twice —
white-and-blue and black-and-red — with the same two strings behind both. That
is the axis `settings.display.front_end_style` offers, defaulting to the base
game's. Asserted in
`every_feature_illustration_is_on_the_disc_in_both_stylings`, which checks all
ten resolve and that the Fury name is the base name plus a suffix.

### The feature is a random draw, and the deck is the race mode's

Read out of the screen's own constructor —
[loading-screen.md](../ghidra/functions/ps3-hdfury-eu/loading-screen.md), which
is also where the four colours below come from. `Feature type == %i` reports
`object+0x444`, and that field is a counter reduced modulo a range the race mode
picks: `% 5` on the two Eliminator modes, `% 3` on two more, a fixed `1` on one,
`% 2` on another, and `% 3` or `% 4` otherwise.

So it is neither a rotation nor a fixed choice, and this section's earlier claim
that "what drives that number has not been read" is superseded. The mode is
`g_GameState`'s, and eleven of its twenty-two ids are now named too - see
[mode-manager.md](../ghidra/functions/ps3-hdfury-eu/mode-manager.md#the-mode-enum-22-ids-eleven-of-them-named),
which this screen's question is what prompted.

**This build shows one feature and does not draw.** It loads a single
illustration at boot; drawing per race means loading all five, which is a change
to the asset path rather than to a constant — and with the mode ids unread there
is no honest range to draw from. Both observations of the running game logged
type 2, which is Pilot Assist, and that is the one loaded.

### The four colours are the disc's, and named in the executable

The same constructor resolves `FE_HD_BG`, `FE_HD_Grey`, `FE_HD_Blue` and
`FE_HD_LightGrey` through the palette lookup and keeps the four together. Those
are `FEGlobals` in `skin.xml`, so `oag_hd::loading::PALETTE` names them and
`oag_game::loading::Assets` reads their values off the source itself — the
screen is tinted by the disc rather than by a scheme this project chose.

Their **values differ by archive**: `DATA06` gives `HD_BG` as `0xffffffff` and
the served copy does not, which is why the screen comes out white-and-blue on
one and black-and-red on the other. That is the same open question about
`skin.xml`'s six copies [hd-frontend.md](hd-frontend.md) already carries, and it
lines up suggestively with `OPT_FE_STYLE` — suggestively, and unproven.

Which of the four plays which role is **not** recovered: the drawing code that
reads them has not been traced, so the assignment (ground, rule, accent, text)
is a reading of a screenshot at confidence 60.

## A `.gtf`'s rows run bottom-up, and nothing 2D knew

Found by drawing this screen and comparing it with the original: the
illustration came out **inverted**. `Pilot_Assist_fury.gtf` decoded straight
into a sprite sheet draws the craft upside down, and reversed it matches the
screenshot exactly.

**The decoder is not what is wrong.** Every other consumer of
`oag_formats::gtf` is a 3D one - `mesh::rcs::skin`, `mesh::sky_cube`,
`race::assets`'s trail noise - and those are already right: the hull lettering
on an HD craft reads the correct way up, which is the check
[rcsmodel.md](rcsmodel.md) used to confirm the texture coordinate in the first
place. Their sampling convention and the file's row order agree. So the row
order is a property of the file and which way up a consumer wants it is the
consumer's, and the flip belongs in `oag_game::sprite::Image::decode_gtf`,
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

- **The widget labels are not drawn.** `FEATURE IMAGE`, `FEATURE DESCRIPTION`,
  `PROGRESSION BAR`, `MODE ICON` and the small `WIPEOUT® HD` in the screenshot
  are the game's own debug overlay printing widget *names*. Reproducing them
  would be reproducing a developer tool.
- **The bar is flat, not dotted.** The original tiles `dot.gtf`; `Draw::Sprite`
  has no repeat mode, so an 8x8 dot stretched across the bar comes out as a
  blurred smear - tried, and it read as a rendering fault. The shape and the
  place are the original's and the texture is not, which is the right way round.
  The rules *are* faithful: `line.gtf` is an 8x8 tile the original stretches
  itself (see [hd-frontend.md](hd-frontend.md)).
- **The bar has no fill.** `race::load` reports no progress, so there is no
  honest number to fill it with and the trough is drawn empty. See
  [Still unread](#still-unread).

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
- **Which of the four palette colours plays which role**, and whether
  `FUN_006762f8` is a random source or a frame counter.
- **`LOADING SCREEN TYPE type == %i`**, observed only at 0. It sits with
  `scePresents.gtf`, the twelve `presents_<language>.gtf` files and the cut demo
  still, so more than one type exists and the presents screens may be among
  them.
- **The progression bar's source.** The original fills one; this build's screen
  draws no bar for a race load because it has no honest number to fill it with —
  `race::load` reports no progress. That is a gap this project could close on its
  own terms rather than a recovery.
- **Three of the five feature title ids**, above.
