# The race box's selection screens

**Status: both of Pulse's screens are read off the disc and drawn, on both
pressings - the panel, its hex grid, its rows, the arrows, the counter, the
rating bars, the livery row on the disc's own skin axis, the lap distance
measured off each circuit, both preview meshes, and (since 2026-09-10) the
slideshow of the circuit's own stills behind the hexagonal window. One
thing is known and unbuilt: the `Loyalty` bar.** Implemented in
[`oag_ui_screens::picker`](../../crates/ui-screens/src/picker.rs) (model, layout, draw),
[`oag_ui_screens::picker::slideshow`](../../crates/ui-screens/src/picker/slideshow.rs)
(the window's state machine),
[`oag_game::preview`](../../crates/game/src/preview.rs) (the 3D pass and
the slideshow's loader) and `crates/game/src/main/session/picker.rs` (the
flow). Captured headlessly with `--menu-page track-select` /
`--menu-page ship-select`, on `pulse-psp-usa`, `pulse-psp-eu` and
`pulse-ps2-eu` alike.

This page is the *picture* half of [race-setup.md](../formats/race-setup.md):
that page reads the XML and the executable, this one says what the screens
look like running and what this build draws in them. The frame around them -
the tab, the footer strips, the row colours - is
[menus-original.md](menus-original.md)'s and is not repeated.

## How this was measured

`pulse-psp-usa.chd` under PPSSPP v1.20.4 (SDL build) on an Xvfb display at
960x544, twice the PSP's 480x272 so a capture divides by two, photographed
with `import -window root`. **The walk is now a script**:
`scripts/psp-frontend-capture.py` boots from wherever the emulator is,
answers the first-boot dialogs, and screenshots every screen from `Show Logo`
to `Team Selection` - each row of `Main Menu`, `Racebox` and `Single Player`,
every `RACE TYPE` value, both selection screens per entry (twice, a second
apart, for the animation), and the two `*_Help` overlays and the music
select. 54 frames on 2026-09-09 in one run, none committed: they are game
content, and every number below is a reading off them.

## `Track Creation` (the title bar reads `TRACK SELECT`)

| Thing | Where | Authored as |
| --- | --- | --- |
| up / down arrow | (117, 30) and (117, 215), 17x14 | `pulse_assets.mip` at `U=242`, `V=71` / `V=94` |
| counter | (135, 213), `1 / 3` after a glyph | `honey`, `String="1/1"` |
| info panel | (290, 26), 170x200 | `Infogradient`, a `0x2f000000` gradient with all four corners equal |
| hex grid | the panel's top 60 | `hex_bg.mip`, `TxtrWidth=340 TxtrHeight=120` drawn at half size, `0x7fffffff` |
| circuit name | x=304, up to three lines at y=28/43/57 | `Info Track 3.1..3.3`; `2.1/2.2` at 35/50; `1.1` at 43 - `Menu` face, `InfoHexBGColor` |
| circuit outline | between the name block and the first rule | **not a widget** - a mesh, see below |
| three stat rows | y = 166/181/196, title at x=304, value at x=399 | `Info1..3 Title` (`IG_HUD_DISTANCE`, `IG_HUD_LAP_REC`, `ER_RR`), `Info1..3`, `default` face |
| rules | 85x1 white fades at y=25 and y=201; 85x14 `0x3f` fades under each stat row | two mirrored gradient halves each - see below |
| hex window | left, cards at (50,70), (43,65), (36,60), (29,55), 256x128 each | **not on this XML** - the circuit's own `screen.xml`, see below |

**The circuit name wraps into whichever `Info Track N.x` block has the
right line count.** `Talon's Junction White` is three lines, `Moa Therma
White` two; a greedy word wrap against the panel's inner width (170 minus
the group's 14 on each side) reproduces both, which is what
`oag_ui_screens::picker::wrap_name` does. Confidence 80 that this is the rule and not
only the result: two names is a small sample, and the executable's own
`Info Track %d.%d` formatter has not been read.

**The hex grid is a 32x16 tile, repeated.** `hex_bg.mip` is 32x16 on
both discs (the PS2's `hex_bg.pct` too - `HANDOVER.md`'s "a texture
resolution not yet listed" was this), and `Infohexgrid` samples
`TxtrWidth="340" TxtrHeight="120"` of it into a 170x60 rect: ten and a
half tiles across, seven and a half down, at half size. Off a sprite sheet
that read the sheet's neighbours instead of the tile again, which is why
the grid was missing from this build's panel until 2026-09-10 -
[`Draw::TiledSprite`](../../crates/ui/src/frontend/draw.rs) wraps the
tile in the fragment stage now. Confidence 95.

**The rules are gradients, and the XML says so.** Every horizontal line on
both screens is two 85-wide `<Image>` halves with `Color1`/`Color2` at one
edge and `Color3`/`Color4` at the other, the left half fading in from
`0x00ffffff` and the right half fading back out - a line brighter in the
middle than at the ends, which is exactly what the capture shows. Drawn by
[`oag_ui::frontend::Draw::GradientFill`](../../crates/ui/src/frontend/draw.rs),
a variant added for it: nothing else on either PSP title authors one.
Confidence 90.

### The outline is the disc's own mesh, and it is not the flythrough

`Data\Environments\16_Track\FE\forward.vex` decodes to **one mesh, 364
vertices, 356 triangles, bounding radius 805** - a ribbon at the circuit's
own scale, and rendered it is the plan shape of Talon's Junction: the same
silhouette the capture shows glowing on the info panel, with the two bright
marks at the same two places. **So the `FE\<run>.vex` file the
`TrackSelection` class loads is the panel's outline, not the flythrough.**
[race-setup.md](../formats/race-setup.md) had read it as the flythrough's
scene; the size (18 KB against the circuit's 4.25 MB) was the tell, and
rendering it settled it. Confidence 95.

### The hexagonal window is a slideshow, authored per circuit

**There is no flythrough.** What the window shows is a chain of the
circuit's own stills, and the file that authors it is on the disc beside
the circuit: `Data\Environments\16_Track\screen.xml`, which
`TrackDefinition_EnterScreenState` loads into the selected definition's
own state machine and enters at `Info` - the code is in
[race-box-screens.md](../ghidra/functions/psp-pulse-usa/race-box-screens.md#the-hexagonal-window-is-a-per-circuit-screenxml-and-it-shows-stills).
The file is six states two seconds apart, each nested one card deeper:

```text
Info -> Info2 -> Info3 -> Info4 -> Info3 Close -> Info2 Close -> Info
  1        2        3        4          3             2         cards
```

The cards are `<location>\FE\image_01.mip` .. `image_04.mip`, 256x128,
**each already a hexagon** - the crop and the white border are in the
pixels, not in a frame drawn over them - at (50,70), (43,65), (36,60),
(29,55), and each card after the first sits on a `track_sel_shadow.mip`
(128x64 drawn 226x128 from a 113x64 sub-rect, `0xCF000000`) four units
down and right of it. So the picture stacks up over eight seconds and
back down over four, and the "stack of frames" the second capture on
Talon's Junction shows is the chain three deep. The two captures a second
apart that [race-setup.md](../formats/race-setup.md) once read as "the
camera visibly further along the corridor" were two different stills.
Confidence 95: the file's own content, drawn, against the capture.

**The slideshow restarts on every selection**, as the original's
`TransitionTo("Info")` does, and runs off the picker's own fixed-tick
clock, so `--menu-page track-select` is always the first card. Read by
[`oag_ui_screens::picker::slideshow`](../../crates/ui-screens/src/picker/slideshow.rs),
loaded by `oag_game::preview::slideshow`, and drawn off the front end's own
sprite sheet **extended** with the circuit's cards
(`oag_hud::sprite::Sheet::extended`) - every widget the menus already placed
keeps its rectangle, so nothing is swapped back when the screen closes.

Zone mode enters `Zone` rather than `Info`, off `screen_zone.xml` on the
PSP (`zone_01.mip` ..) and off the same `screen.xml` on the PS2, which
authors both chains in one file and five cards to the PSP's four. A
circuit with no `screen.xml` draws an empty window and says so in the log.

The same file's `<Screen name="Top">` carries the `<Mode3D><Model
name="Ship">` the outline ribbon is loaded into, with an authored pose
(`x=0 y=-300 z=-3700 RotX=1.5 RotY=0.1`, `OriginX=-145 OriginY=13
nearZ=1000 farZ=5000`). **Now read and drawn from, not just carried** -
the camera has no position or rotation of its own (only `Origin`/`nearZ`/
`farZ`; the `Model` is what moves), a fixed vertical FOV and the PSP's own
`480/272` aspect regardless of the panel's shape, and `Origin` is a plain
screen-pixel shift of the viewport's own centre -
[race-box-screens.md](../ghidra/functions/psp-pulse-usa/race-box-screens.md#mode3ds-own-fixed-camera-and-the-child-models-own-pose-2026-09-28)
has the addresses and confidence. `oag_game::preview::mode3d_view_projection`
builds it; `oag_game::preview::orbit_for`'s own `Track` branch is now dead
code kept only as the fallback for a circuit whose `screen.xml` authors no
usable `Mode3D`. Open: the `Model`'s own rotation composition order in this
renderer's column-vector convention is a plausible transpose of the PSP's
own row-vector one, not confirmed by stepping through a frame, and the PS2's
own `Origin` sign is inferred from the two pressings' authored numbers
rather than read off `psp-pulse-eu`.

**The cards fade in on the original, and now here too.** Every `LeftLayer`
on the screen carries a `transition` - a fade-in/fade-out duration in
seconds, on the generic widget class every XML element goes through
(`Widget_CreateFromElement`,
[race-box-screens.md](../ghidra/functions/psp-pulse-usa/race-box-screens.md#leftlayers-own-transition-attribute-is-a-fade-duration)),
not a slide or a scale as the earlier reading of "the second capture shows a
card mid-arrival" left open. Confirmed against a live capture: the panel
(`transition="0.5"`) and the hexagonal window's own first card are both
faint at 130ms into `Track Creation` and settled by 320-480ms, while the
title bar's own `transition="0"` group is solid from the first frame.

Read into `Text`/`Image`/`Fill::transition` by `oag_ui::screen`'s
`LeftLayer` container (inherited by everything nested under it, the same
way `OffsetX`/`OffsetY` already thread down) and applied in
`oag_ui_screens::picker::body` as a linear alpha ramp against
`Picker::seconds()` - **the ramp's own shape is measured, not chosen**:
`Widget_UpdateTransitionFraction` (`0x0888d8e4`, confidence 80,
[race-box-screens.md](../ghidra/functions/psp-pulse-usa/race-box-screens.md#the-per-widget-fade-is-confirmed-linear-2026-09-28)),
the generic per-widget fade every widget kind's own `Update` calls, is
`elapsed / duration` on both its enable and disable branches with no curve
anywhere, confirmed against several thousand live breakpoint hits, not just
the decompile. The hexagonal window's own stills author no `transition` of their own (the
per-circuit `screen.xml` has no `LeftLayer`), so their fade
(`oag_game::preview::CARD_FADE_SECONDS`) reuses the panel's own measured
`0.5`s against `Picker::since_selection()` rather than inventing an
unrelated number - it restarts with the slideshow itself, on every
selection, matching "the slideshow restarts on every selection" above.

**Verify with `--menu-page track-select`/`ship-select --menu-picker-seconds
<seconds>`** - `--ticks` and `--menu-anim-phase` both reach no clock this
screen runs (see the CLI flag's own docs), so this is what stands in for
"play it and watch": `0.0` is the instant the screen opens, and it is
settled by `0.5`. The flag omitted draws it settled, so no still capture
this build already had changes appearance.

### What this build draws differently, and why

- **Distance is measured, and reads about two per cent under the
  original.** The original prints `5178` for Talon's Junction White, `5350`
  Moa Therma, `4419` Metropia. This build measures the lap
  `oag_race::course::Course` walks - both authored paths in ring order,
  closed - off each circuit's own file on a worker thread as the screen
  opens (selected circuit first), and prints `5094`, `5228`, `4330`. **The
  gap is not sampling**: the figure is identical to a unit at one, four,
  sixteen and sixty-four steps per segment and from the raw control
  points, so the original sums a slightly different curve - the centreline
  rather than the racing line, most likely - or counts the two junction
  links differently. Unread; confidence 70 on what the screen's number is.
  Pinned at under three per cent in
  `crates/game/tests/circuit_length_ground_truth.rs`, and drawn as this
  build's own measurement rather than a stand-in. Until the worker has
  read a circuit its row shows `-`.
- **Lap record / Race record are this build's own store**, keyed the way
  the RECORDS page keys them - circuit, mode and the RACE page's class.
- **The counter is `n / 32` on a fresh profile**, not `1 / 3`: this build
  offers every circuit the definition authors, by design
  ([menus.md](../architecture/menus.md#unlocks-pulses-circuits-and-craft-variants-gate)).
- **The outline is darker than the original's.** It is drawn through the
  ordinary mesh pipeline with the stand-in light; the original's has a
  teal glow whose source (a blend class on the ribbon, a second pass, a
  bloom) is unread.

## `Team Selection` (the title bar reads `SHIP SELECT`)

| Thing | Where | Authored as |
| --- | --- | --- |
| arrows, counter | as above; the counter reads `1 / 8` | same widgets |
| info panel | (290, 46), 170x162 | `Infogradient` |
| hex grid | the panel's top 44 | `hex_bg.mip`, `TxtrHeight=88` |
| team name | x=304, y=49 | the `List` widget's first row - no `Text` authors it |
| livery | `skin` text at (319, 71), arrows at x=302 and x=439, y=74 | `skin`, `skin left arrow`, `skin right arrow` (children of the text) |
| four bars | y = 92/107/122/137; title at x=304, bar at x=369 (58x10), value right-aligned to x=449 | `<Item OffsetY="44/59/74/89">`, `pulse_assets.mip` at `U=437 V=1`, backing at `0x7fffffff` |
| loyalty | title at y=163, bar at y=181 (148x10, `U=332 V=72`) | `ER_LOY`, `Loyalty`, `Loyalty Bar` |
| craft | the left half, on the backdrop | **not a widget** - `ship_FE.vex` on a turntable |

**A bar is ten segments and the number beside it is how many are lit.**
Assegai's `8 / 8 / 9 / 7`, Qirex's `8 / 7 / 8 / 9`, AG Systems' `7 / 9 / 9 /
8` and Piranha's `10 / 6 / 6 / 9` on the capture are, digit for digit, the
`<FE speed thrust handling shield>` element under each `PI_Team` in
`Data\Plugins\PI001\Definition.xml` - **the rating table race-setup.md
recorded as unlocated**. Read into `oag_raceplay::catalogue::Team::rating`;
the bar is drawn at `rating / 10` of its width with the texture sub-rect
scaled the same way, so the segments stay aligned. Confidence 95.

**The craft is `Data\Ships\<team>\ship_FE.vex`** (6 meshes, 1,303 vertices
for Assegai), turning once every twelve seconds seen slightly from above -
the period and the pitch are this build's own readings of two frames a
second apart, not recovered numbers.

### The craft's swap on HD/Fury, measured 2026-10-10 (hd-ship-select-anim)

Pulse's `ship_FE.vex` turns on a turntable (above). **HD/Fury's does not**:
the race hull (`Data\Ships\<team>\ship.vex`) holds one pose. Method: RPCS3
(`hdfury-ps3-eu.iso`, own `serve`), the Xvfb display grabbed with ffmpeg
`x11grab` into a lossless clip while `scripts/rpcs3-hd-shipselect-film.py`
tapped the pad on a timetable; `scripts/hd-shipselect-series.py` reads the
silhouette (pixels with every channel above 60, in the `SHIP MODEL` frame) per
frame. Clips: a 30 fps run (entry, five steps, 25 s hold) and a 60 fps run
(three steps; 1,080 frames in 18 s, no duplicate frame, so the emulator
presents at least 60 fps there). Reference frames and the per-frame series:
`data/reference/hd-capture/ship-select-anim/`. One boot, two films.

| Thing | Reading | Confidence |
| --- | --- | --- |
| Turntable | **none.** Silhouette identical to the pixel across 18 s of hold (width and height of the 1-99 percentile box constant to 1 px after the settle; frame-to-frame crop RMSE 0.3% where no particle crosses it) | 85 |
| Opening the screen | craft **absent** while the screen's particle transition plays, then **cut in** whole at rest pose, no settle: first frame 119-120 of a press at frame 90, 30 fps, so 0.97 s less about 0.1 s of input latency | 60 |
| Stepping, either direction | old craft cut in **one frame**; **empty frame 3 captured frames at 60 fps twice, 4 once (50-67 ms)**, and 3 at 30 fps (100 ms) in the heavier capture - counted in emulator frames, not a fixed time; new craft cut in solid (no alpha ramp, no slide) | 70 |
| Settle after a step | the new craft appears slightly off its rest size and **eases to it in 0.45 s** (5% left at 26-28 frames of 60). Excess per 50 ms: `1.00 1.03 0.97 0.83 0.56 0.39 0.24 0.17 0.10 0.06 0.03 0.01 0`. Pure scale about a pivot right of and above the hull's centre (Qirex: fit of both silhouette edges to 1 px) | 65 (curve), 40 (what is scaled) |
| Start size of the settle | **per team**: 0.95 of rest (Feisar), 1.08 and 1.12 (two others), independent of the step direction (the same team entered from either side starts alike). Not recovered | 50 |
| Particle burst | a red then orange/yellow streak cloud starts about 2 frames after each cut at the left of the craft and decays over about a second; red wisps also drift past with no press. Which is the screen's backdrop and which a step effect is **not separated**; no `.pob` named | open |
| Camera | **One authored pose for every team**, so the per-team look is each hull's own placement: `Team_Selection_Definition.xml`'s `<Model name="ShipModel">` is `OriginX=1220 OriginY=412 x=0 y=0 z=-24 RotX=0.4 RotY=-0.5 nearZ=1 delay=1.0` (`delay` matches the 0.9 s absence). The per-team `Data\Ships\<team>\screen.xml` files are Pulse-lineage leftovers naming a `ship_FE.vex` HD does not ship, and are not this screen's. The 12 teams at rest on RPCS3 all share one view (nose lower left, top seen, tail upper right) | 85 (authored values), 80 (one pose for all) |
| Lens and pitch | the widget authors no lens. Drawn on raw coordinates with the authored origin as an absolute grid point, `RotX*RotY` order, the silhouette fit (Feisar, Qirex, Assegai at rest, coordinate descent on overlap) wants a vertical FOV of **0.41 rad**, a pitch of **0.22** (authored 0.4) and a 0.36 slide; overlap 0.45 against 0.31 at the authored pose. Compared side by side for Feisar, Qirex, Assegai, Icaras (Icaras not fitted): size, angle and position agree. The authored pitch reads steeper than the original in all four, so the difference is a convention not yet found | **fitted, not authored**; no score |
| Step burst | **not a `.pob`.** The red and gold field is the Fury backdrop widget, `BackgroundAnimFury_Item` (a point cloud of a team hull through the `RadioHead` effect modes, `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`). A 26 s film with steps at 7, 13 and 19 s shows the cloud thicken around each cut and a gold starburst at about 12 s with no press; HD's `data/psys/*.pob` are weapon, damage and number effects only. Measured 2026-10-10 on RPCS3 (menu-backdrop.md, "The tint is applied in linear light"): the screen is the **`default`** row, `0.125`, like every screen without a row (not white), and a tint of 0.125 reads about 0.4 as bright because it is applied in linear light; this build now draws the backdrop under Ship Select and Track Select that way. The widget's `OnEnable` pulse fires on **screen changes**, not on a team step (five presses, the pulse block did not move), so the step reaction is **not** wired and its mechanism is unlocated | 60 (not a pob), 90 (row), 75 (law), open (step) |

Framing: [`oag_game::preview::ship_model`] (replaces the fixed `hull_orbit` on
HD; its swap scale multiplies the model). Omega: checked, differs - the
`Team_Selection_Definition.xml` in `data09` is the `PI_Team` plugin list and
authors no `ShipModel`, and its hull is not drawn (`ship_preview_hull` is
`None`), so nothing is wired. Reference frames: `data/reference/hd-capture/ship-framing/`.

What this build does ([`oag_game::preview::hull_swap`](../../crates/game/src/preview/hull_swap.rs)):
absent for 0.9 s after the screen opens, a 60 ms gap on every step, then the
craft eases from 1.06x to 1x over 0.45 s along the measured curve. **The
1.06 start scale is chosen, not measured** (the original's is per team).
Headless stills: `--menu-page ship-select --menu-picker-seconds T` (entry) and
`--menu-picker-step-seconds T` (a step). Omega: checked, applies, not wired
(`ship_preview_hull` is `None` there and its `Team_Selection_Definition.xml` is
HD's own; the swap is title-agnostic and reaches Omega with the hull itself).

**The livery row is the skin axis, as on the original.** `Classic` is the
disc's own string id for the baseline paint (the Japanese plugin
translates it; English falls back to the id), and each further entry is a
`PI_ModelSkin` the team declares (`Alternative`), applied to the preview
hull through the same texture-slot swap a race makes
([ship-skin.md](../ghidra/functions/psp-pulse-usa/ship-skin.md)) and
stored as `race.skin`, which the launched race reads the way `--skin`
does. A team that declares no skin - every HD and 2048 team - gets the
title's *variant* axis on the same row instead, the RACE page's VARIANT.
A skin is offered only when its `<Unlock>` rows pass (`oag_game::unlock::loyalty_unlocked`, own-team OR any-team; absent when locked, as the original's list builder does), so a fresh profile's row holds `Classic` alone and its arrows are dimmed.
Track Select, by contrast, hides circuits whose `<Unlock Grid>` is not met
(`oag_game::unlock`).

**The Loyalty block is drawn on Pulse** (2026-10-02): the title, the dim
backing, the selected team's own running total beside it and the bar,
`150 * min(total, 100000) / 100000` whole pixels wide (`FEScreen_SetStatBar`
as `TeamSelection_Update` calls it; `crates/ui-screens/src/picker/body.rs`). It is a
read-out with no pointer behaviour. A title with no loyalty counter
(`Details::Ship::loyalty` is `None`) still leaves the block out whole.

### HD's hex grids pan under a finger (2026-10-09)

**Chosen, not measured**: the original is a pad-only PS3 game, so every line
here is this build's own, with no confidence score. On both HD hex grids
(`Team Selection`'s `NAVIGATE TEAM` and `TrackHexSelection`) a horizontal
touch drag pans the columns, through the front end's one touch-scroll model,
`oag_ui::kinetic` (the same model long menu pages and 2048's campaign map
scroll through):

- The columns follow part of the finger: `Picker::pan` is a fractional column
  offset (drag travel times `DRAG_GAIN` 0.6, over the 72.2-unit column pitch), and a column slides in at the
  edge it is revealed from while the opposite one slides out, each faded by
  how far it has travelled. The half-row stagger of odd columns eases with the
  pan instead of jumping.
- The selection is the column nearest the centre: once the pan passes 0.6 of
  a column (`picker::pointer::STEP_AT`; half a column, plus a tenth so a
  finger resting on the halfway line does not flip the selection and reload
  the preview as it trembles) the selection steps by one, wrapping, with the
  pad's own `Event::Moved` (selection sound and preview included). Dragging
  right brings the column on the left to the centre.
- On lift a flick coasts on across columns, stepping each one it passes, and
  comes to rest on a column; a slow lift settles onto the nearest column. A
  tap on a coasting grid stops it and selects nothing. A pad or wheel step
  during a coast settles it on the column it steps to. A drag never selects or
  confirms, and a vertical drag does nothing. Pulse and Pure pickers are
  unchanged.
- A drag pans only when the press began on a hex tile (`Pointer::press_at`,
  tested with `hex_contains` against the open cells). A press on the backdrop,
  the info panel, the craft or track preview or an arrow pans nothing and
  steps nothing; a drag that began on a tile keeps panning after the finger
  leaves the grid. Each new press decides again, and a touch-down still
  catches a coast wherever it lands.
- Only a touch produces `Pointer::drag` (`oag_game::main::pointer`); a mouse
  drag does not, so a mouse still clicks cells and uses the wheel.

Calmer on a phone (2026-10-10, maintainer: dragging was too fast to land on
a specific ship or track). Both were at fault: the grid is authored on a
1280-unit canvas, so on a phone a 72.2-unit column is about 8 mm of finger at
a 1:1 drag, and a light flick (6 columns/s) coasted 3 more columns at the
shared friction, up to 12 at the 24 columns/s cap. The hex grids now take
their own values, `picker::pointer`'s, passed per use through
`oag_ui::kinetic::Extent::{max_fling, friction}` so long menu pages and 2048's
campaign map keep the shared defaults below untouched. The window's DPI is not
consulted: the pointer layer reports grid units only, so the gain is a fixed
fraction, not a physical distance.

| Hex-grid constant | Value | Why |
| --- | --- | --- |
| `DRAG_GAIN` | 0.6 | The columns move 0.6 of the finger, so a column takes about 13 mm instead of 8; the fling speed is read off the scaled travel, so a flick slows by the same factor |
| `HEX_MAX_FLING` | 12 columns/s (shared 24) | A hard swipe coasts at most 4 columns instead of 12 |
| `HEX_FRICTION` | 3.0 /s (shared 2.0) | A flick travels `speed / 3`, a third less; an aimed coast still lands on a column (`FRICTION_RANGE`) |

The model's constants, all **chosen, not measured**:
(The table is the shared defaults; the hex grids override two of them as above.)

| Constant | Value | Why |
| --- | --- | --- |
| `FRICTION` | 2.0 /s | A coast's speed decays by `exp(-2t)`, travelling `speed / 2`: iOS's normal deceleration (0.998 per ms), the feel players already have |
| `FRICTION_RANGE` | 1.0..8.0 /s | The decay an aimed coast may be given to land on the item the unaimed one would stop nearest |
| `SPRING` | 18 rad/s | Critically damped settle: half an item to within 1% in about a third of a second, solved exactly so a long tick cannot make it ring |
| `HANDOFF` | 2 items/s | A coast slower than this hands over to the spring |
| `MIN_FLING` | 1 item/s | A slower lift is not a flick; it settles straight onto the nearest item |
| `MAX_FLING` | 24 items/s | A hard swipe; under one column a tick at 30 fps, since each column passed on `Team Selection` reloads the preview |
| `VELOCITY_WINDOW` | 0.1 s | The finger's velocity is read off this much of the recent past (Android's tracker window); a finger held still that long flings nothing |
| `RUBBER_BAND` | 0.55 | Past an end of clamped content the pull gives `c x b / (c x + b)`, iOS's coefficient; HD's grids wrap and have no band |
| `CATCH_SPEED` | 0.5 items/s | A touch-down on content moving faster than this catches it and swallows its tap |
| `REST_SPEED` | 0.1 items/s | An unsnapped coast (2048's map) slower than this has stopped |

The finger's velocity is estimated by the model from the screen's own tick
`dt` and the per-tick drag, not from event timestamps, so it is deterministic
and unit-tested (`crates/ui/src/kinetic/tests.rs`: fling distance, snap
target, rubber band, bounce, tap-to-stop, wrap, determinism).

## The PS2 pressing

The same two screens, in the same `Selection_Definition.xml`, on the PS2's
own 640x448 grid - every position scaled by (4/3, 448/272), the way the
whole PS2 front end is ([aspect-ratio.md](../ps2/aspect-ratio.md)). Three
things differ, and all three are handled rather than special-cased:

- **`Team Selection`'s widgets carry a player index**: `honey0`,
  `Speed Bar0`, `skin0` and so on, one set per split-screen player, where
  the PSP's are bare (`Track Creation`'s are bare on both). Read under the
  PSP's names by stripping the `0` when a screen has a `honey0` and no
  `honey` (`picker::strip_player_suffix`); the `1` set only exists on
  `Team SelectionSplit`, a screen this build does not open.
- **Both previews need the circuit's or the craft's sibling texture set**,
  the same rule a race applies (`oag_livery::entry::ps2_texture_set`); without it the
  outline and the hull both drew flat white.
- **The cards are `.pct`**, found through `oag_pulse::read_image`'s rewrite
  like every other PS2 image, and the PS2 `screen.xml` is plain XML rather
  than dictionary-compressed.

The handful of numbers this module measured off the PSP capture rather
than read out of a widget - the panel's fallback rect, the craft's
viewport, the name block's inset - are multiplied by the grid ratio
(`Layout::scale`) rather than copied.

## The flow

`RACE` page `START` -> Track Select -> Ship Select -> the race. The same
three pages Pulse walks (`Single Player` -> `Track Creation` -> `Team
Selection`), **and the RACE page has the rows `Single Player` has**: on a
title that authors these screens its TEAM, VARIANT and TRACK rows are
dropped (`oag_ui::menu::Definition::drop_rows_picked_on_screen`), since
every one of the three is picked on the screens with its preview, its
ratings and its livery. Each picker writes the setting the row used to, so
the circuit picked is the one the race loads and the one RECORDS looks up.
A stored circuit the mode's list does not hold - a race circuit after MODE
moved to Zone - lands the screen on its first entry and writes that, so a
confirm without moving launches what is on screen. Zone skips Ship Select,
since the mode forces the Zone hull. Back from Track Select is the RACE
page; back from Ship Select is Track Select. A title that authors neither
screen (`oag_title::FrontEnd::race_box` is `None` - Pure, HD, 2048 today)
keeps all three rows and launches from `START` as before.

## Open

- ~~The cards' arrival transition, and the `Mode3D` pose of the outline -
  both authored in the circuit's `screen.xml` and read, neither acted
  on.~~ **2026-09-28: both halves are closed.** The transition - see "The
  cards fade in on the original, and now here too" above and
  [race-box-screens.md](../ghidra/functions/psp-pulse-usa/race-box-screens.md#leftlayers-own-transition-attribute-is-a-fade-duration).
  The `Mode3D` pose - see the new paragraph above and
  [race-box-screens.md](../ghidra/functions/psp-pulse-usa/race-box-screens.md#mode3ds-own-fixed-camera-and-the-child-models-own-pose-2026-09-28):
  the outline is now framed by the disc's own fixed camera
  (`oag_game::preview::mode3d_view_projection`), with the rotation
  composition order and the PS2's `Origin` sign left as the two open
  sub-items that page's own history entry names.
- ~~The PS2 screens are captured headlessly against the PSP's live capture
  only: no PCSX2 walk of the PS2's own race box exists yet~~ **2026-09-27:
  walked.** `pulse-ps2-eu.chd` on PCSX2 (`SCES-54748`), `RACEBOX` -> `TRACK
  SELECT` -> `SHIP SELECT`, English (the profile the memory card already
  carried at this walk's start; see the PS2 layout thread for why French
  was not used here). Both screens read Talon's Junction White (1/3) and
  Assegai/Classic (1/12) with `Speed 8 / Thrust 8 / Handling 9 / Shield 7` -
  digit for digit what `--menu-page track-select`/`ship-select` draws on the
  same source. No card-size or `0`-set placement bug found: the two screens
  match. Captures: `pcsx2-track-select.png`,
  `pcsx2-ship-select.png` against `ours-track-select.png`/`ours-ship-select.png`
  in the same directory.
- Which curve the original's `Distance(m)` sums - see above.
- The `Loyalty` counter, once anything keeps one.
- The `Track Help` / `Team Help` overlays and `Pre Race Music Select`: read
  (they are on the same XML) and not drawn.
- The outline's glow.
- The `small` face: the footer's `Help` / `Music playlist` prompts are
  authored in it and drawn in the menu face scaled by 17/22, since that face
  is not loaded for the menus.
