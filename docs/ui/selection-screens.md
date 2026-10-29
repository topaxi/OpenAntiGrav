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
own row-vector one, not confirmed by stepping through a frame. The PS2's
`Origin` is read off its own executable ([mode3d.md](../ghidra/functions/ps2-pulse-eu/mode3d.md)):
it is a switched GS-offset shift (on for the circuit, with the sign measured live; off for the
craft), and the camera's aspect is the grid's own `640/448`.

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

- **Distance is the circuit's authored length** (closed 2026-10-29). The original prints
  `<Physical Length>` of the circuit's `stats.xml` (`stats_reversed.xml` for a reversed entry)
  through `%d`: `5178` for Talon's Junction White, `5350` Moa Therma, `4419` Metropia. This build
  reads the same file on the worker thread as the screen opens (`oag_raceplay::authored_length`,
  `crates/raceplay/tests/circuit_length_ground_truth.rs`) and prints it the same way. A circuit
  whose `stats.xml` is absent or authors no length (a DLC circuit may) falls back to the lap this
  build measures off the circuit's spline (`oag_raceplay::circuit_length`, 1.7 to 2.3 per cent
  short; chosen, not measured). Until the worker has read a circuit its row shows `-`. Pulse PS2
  reads its own `stats.xml` from the race archives (PS2 data, not inherited); Pure keeps the
  measured lap (its panel is unmeasured), and HD, 2048 and Omega print `length_rows` of the
  measured lap (HD's `stats.xml` authors `Length="5.2"` in km: checked, applies, not wired).
- **The first row's label is `Distance(m)`**: the class overwrites the XML's `IG_HUD_DISTANCE`
  (`Distance`) with `MSC_DISTANCE` (`oag_title::FrontEnd::track_label_overrides`).
- **An unset record reads `-.--.--`** (`0x08a84914`, drawn by `0x088edd60` for `0` or `0xffff`),
  `oag_title::FrontEnd::track_record_empty`. Pure's panel draws `0'00"00` through its own binary: a
  different literal, not switched on (open).
- **The panel draws the same three rows in every mode.** The original moves the distance to
  `Info2` and blanks `Info1`/`Info3` outside modes 3, 5, 6, 9 and 10, and Zone shows a zone score
  (`race-box-screens.md`); not wired (open).
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
for Assegai).

**The craft sways, it does not turn (2026-10-29, `pulse-picker-clock`).** The
"one revolution every twelve seconds" this page and `orbit_for` carried was
read off two frames a second apart and is wrong: a PPSSPP frame sequence shows
the hull rocking about one pose. Both halves are on the disc. (1) The pose is
`Data\Ships\<team>\screen.xml`'s `Info` state: a `Mode3D` (`OriginX 120`,
`OriginY` 20 to 28, `nearZ 1`, `farZ` the widget default 100) over a `Model`
at `x=0 y=0 z=-13..-16`, `RotX` 0.3 to 0.4, `RotY` 0.4, per team (all eight
pinned in `selection_screens_ground_truth`), drawn through
`mode3d_view_projection` like the circuit outline. The lobby state
(`MPLobby Info`) authors the same model at `OriginX -100`; the start state's
own is taken. (2) The motion is `ship_FE.vex`'s own `Anim Transform`: on
Assegai a 359-frame (5.983 s) loop with a roll of about +-19 degrees about one
axis and a two-cycle bob on another. Confidence 90 (file facts, and our
render at the node's clock matches the capture to 6.9 mean abs grey against
27 with the clock off). **The clock is the node's own time, not the time since
entry**: the model loads before the screen opens (offsets 0.48 to 1.92 s were
read), `node+0x40`, the front end's animation clock at `*(0x08b317b0)+0x40`
(`anim-transform.md`). `--menu-model-seconds` carries it into a still. The
PS2 pressing authors `Origin 120, 20` on its 640 grid with another `x y z RotX RotY`
(Assegai `-6 2 -15`, `0.4`, `0.7`) and **plays it without the `Origin`** (2026-10-29,
`pulse-ps2-ship-pose`): the PS2's `Mode3D` draw adds `Origin` to the GS offset only while
`g_mode3d_origin_enabled` is set, and a PCSX2 frame of Ship Select puts the hull where `x -6 y 2
z -15` alone places it, with the grid's own aspect `640/448`
([mode3d.md](../ghidra/functions/ps2-pulse-eu/mode3d.md), `placed_mode3d`; confidence 70). Against
that frame, box (60,150)-(640,560) at 1280x720, the best of twelve sway phases of ours: mean
abs grey diff 24.36 before (the 12 s orbit) to 8.16 after (original box mean grey 19.81, ours
18.87); the sway phase is matched by scan (`--model-seconds 3.0`), the PS2's node clock has no
recovered address. `orbit_for`'s craft orbit is only the fallback for a craft whose own
`screen.xml` authors no `Mode3D`.

**Track Select's stills step every 2 s and cross-fade over 0.70 s
(2026-10-29).** Fitted to 24 phase-read frames of a PPSSPP capture (the
first card and three steps): a linear ramp starting at the step, 0.70 s long,
residual 0.08 summed squares; confidence 70 (frames 0.17 s apart). The cards
a step adds ramp in and the ones it drops ramp out on top
(`Slideshow::draws_fading`, `STEP_FADE_SECONDS`); the earlier 0.5 s arrival fade
and the hard steps were chosen. With the clock on the game's own timer the
photo window is pixel-close (mean abs diff 0.28 grey).

**The airbrake flaps are seated from the race hull (2026-10-27, picker-flaps).**
A Pulse `ship_FE.vex` (PSP and PS2, all twelve teams, `Zone_FE.vex` too) hangs
`AirBrake_Left`/`AirBrake_Right` straight under its turntable `Anim Transform`
with **no locator `Transform` above them**, unlike `Ship.vex`/`Zone.vex`
(`locator2` / `locator3` over each flap). The builder therefore had no hinge to
bake, and a PSP flap, authored in hinge space, drew at the model origin, in the
middle of the hull: Ship Select showed two stray flaps mid-craft while the race
was right (confidence 95 for the file facts: no locator in any FE hull, and the
PSP/PS2 split below, both read off every team's tree and vertex bounds). Measured on every team of both pressings (the four DLC teams
included): carried through the race hull's hinge, each FE flap vertex lands
within 0.0014 model units of a vertex of the race hull's flaps (`Feisar` and `Triakis` carry
an `underbrake_flash` the FE hull keeps as a separate, already placed node, so
the FE flap is a subset). Some PS2 files (Goteki, Feisar, EGX, Triakis,
Harimau, Icaras, and every `Zone_FE.vex`) already hold the flap in model space;
moving those again threw them 2 to 6 units off, so `Model::seat_flaps_on`
(`oag-mesh`) picks, per flap, the placement whose centroid is nearer the race
flap's, and an already placed flap only adopts the race hinge. How the original
seats its front-end flaps is **not recovered**: reading the hinge off the race
hull is chosen, with the vertex agreement as its evidence, not a measured law.

The one path: `oag_game::preview::variant::model_or_default` (live
`PickerStage::load_preview` and the headless `--menu-page` capture both call it)
takes the race hull's entry (`oag_game::preview::race_hull_entry`, the race's own
`oag_livery::entry::ship_entry_name`) and calls `preview::seat_flaps`, which
reads that model's `Flap` hinges. Nothing in the preview places a flap on its
own, and the flap swing (`Flap::swung`, `Flap::seat`) shares one vertex
transform. Pinned by `picker_flaps_ground_truth` (`oag-game`; every team, time
trial and Zone, both pressings). The same read found PS2's Triakis race hull
names its flaps `andy_ship:AirBrake_right`/`_left`, which `Flap::collect` did not
match, so that craft's race flaps never deflected; the namespace is stripped now.
HD's Icaras hull names them `Ship:AirBrake_Left`/`_Right` the same way, so its race
flaps were missing too and now deflect (checked on a held-airbrake HD race frame);
PSP Pulse and Pure have no namespaced flap node, so those two are the only models
the strip changes. Whether a node or flag marks which PS2 files hold the flap
already placed was not checked; the centroid comparison is the recorded fallback.
Other titles: Pure's picker draws a pre-rendered sprite (no mesh, no flaps);
HD, Omega and 2048 draw the race hull (`draws_hull`) through the same race
loader, so they carry no front-end twin to seat. HD and Omega checked by
capture, 2048 not captured here (the dumped `.pkg` is PFS-encrypted).

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
  `Team SelectionSplit`, which a split-screen race box opens for two seats -
  see [the split screen](#the-split-screens-team-selection-2026-10-13).
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

## The split screen's Team Selection (2026-10-13)

What the originals author for two people picking a craft on one console, read
from the disc and followed where it exists. Evidence: `Selection_Definition.xml`
(PS2 Pulse, `54748/WADS2.WAD`, entry `Data\Plugins\PI001\GUI\Selection_Definition.xml`)
and HD's `team_selection_definition.xml` (`DATA06.PSARC`,
`/data/plugins/frontend/gui/`), both read with `oag-wad cat` / `just psarc cat`.

**PS2 Pulse: both players pick at once, on one page.** `Team SelectionSplit`
(`type="TeamSelection"`, `NavigationButtons="2"`) holds two full widget sets,
`Player 1` first (`Item OffsetY -15`, stats at 79, `Infogradient` at y 9) and
`Player 2` below it (`OffsetY 181`, stats at 274, `Infogradient` at y 204), the
same shape as the single-player screen. Each set has its own `List` with
`focus="true"` and a global of its own (`Team0`, `Team1`), names carry the player
index (`honey0`/`honey1`, `Speed Bar0`/`Speed Bar1`, `skin0`/`skin1`,
`up arrow0`...), and the page has its own help (`Team HelpSplit`,
`MSC_HELP_RC_SS`) and music pick (`Pre Race Music SelectSplit`). `Track
CreationSplit` is the one shared circuit pick that precedes it. Confidence
**95** for the page's structure, **70** for "one player per list on one pad
each" (read off `focus` and `NavigationButtons`, no capture of the original's
split screen exists). Two players only: there is no third set.

**HD/Fury: the players pick in turn.** `team_selection_definition.xml` holds
`Team Selection`, then `Team Selection Player 1` (title `FE_SHIPSEL_P1`, its
redirect goes to `Team Selection Player 2`) and `Team Selection Player 2` (title
`FE_SHIPSEL_P2`, its redirect launches), all the one `TeamSelection` page
re-titled, with `TeamRedirectBack` bound by `InputPad="1"` and a
`TeamRedirectBack0` by `InputPad="0"` on the second: the redirect names a pad.
Confidence **90** for the sequence and the titles, **60** for what `InputPad`
does (the file names it; no behaviour was observed). 2048's string table carries
the same two ids (`FE_SHIPSEL_P1`/`_P2`); its front end is unread.

**What this build does** (`crates/game/src/main/session/picker.rs` and its `seat`
module, `oag_ui_screens::picker::split`):

- A source that authors `Team SelectionSplit` (the PS2 pressing) **and** a race
  box of exactly two seats shows it: `Layout::read_split` reads the two halves
  back into one `Layout` each. The widget tree is flattened when parsed, so a
  widget's half is read from the player index its name ends in, and from its
  height (below or above the second panel's top) for the unnamed rules and bars
  and the shared `Infogradient`/`Infohexgrid`; the lone `Suggest` line is the
  second half's and the online lobby's status line is neither's. Each half
  draws, points and picks like the single-player screen, on its own seat's
  device. A seat confirms to **hold** its pick (`READY`, new string
  `OAG_PICK_READY`) and the race starts when both hold; back takes a hold back,
  and back on a seat that holds nothing returns both to Track Select.
- **Pulse's PSP pressings, exactly two seats: the PS2's page, borrowed (2026-10-13).** The
  PSP authors no `Team SelectionSplit`; the maintainer's rule is that anything for Pulse is
  the same on both pressings, so `oag_ui_screens::picker::borrow` builds the page from the
  PS2's numbers (`PS2_SETS`: each player's arrows, counter and stat block, the shape of one
  set written once) rescaled into the PSP's 480x272 grid, with the PSP's own sprites, and
  reads it back through `Layout::read_split`, the function that reads the PS2's. The loader's
  report says `split ship borrowed from the PS2 pressing (placement not measured on it)`.
  **Borrowed, not measured on the PSP: no confidence score.** `FrontEnd::borrows_ps2_split_screens`
  is true for Pulse alone, so Pure's PSP pressings keep the sequence below. **Chosen for the
  smaller grid:** the PSP's faces are as tall in 272 rows as the PS2's in 448, so the stat rows'
  text and the team's name are drawn at the grid ratio (`Layout::name_scale`, the text `scale`)
  to keep the PS2's room for them (stat rows are 16 apart in the PS2's grid, 9.7 in the PSP's).
  Pinned by `crates/ui-screens/tests/split_selection_ground_truth.rs::the_borrowed_page_is_the_ps2_authored_one`
  (the numbers in the PS2's own grid read back as the file's two halves, widget for widget; a
  changed number fails it) and `crates/game/tests/selection_screens_ground_truth.rs`. Stills:
  `--menu-page ship-select-split` (both halves; the 3D craft is the one pass a still carries,
  so only the first half's is drawn, at the single page's rect) on either pressing.
- **Every other source and every other count** gets one Ship Select per seat in
  seat order (HD's authored order; chosen elsewhere). Each opens for its seat,
  titled by the source's own `FE_SHIPSEL_P<n>` when its table has one and by
  `<SHIP SELECT> - PLAYER <n>` otherwise, driven by that seat's device alone.
  Back returns to the seat before.

**Chosen, not measured** (the original authors none of this): the second half's
craft rect (the column between its arrows; the original places its craft in
code); a seat opening on the team after the previous seat's, so bare confirms
race different teams; two people may take the same team (the draw does not
offer a person's pick to an opponent, and when the two picks differ the
opponent that held it takes the team a person left); the pointer drives
whichever screen is open in the sequential flow, and on the shared page a click
lands on the half it is over and the wheel steps the half under it; a held seat
ignores everything but back.

**Cross-title.** HD: **checked, applies, wired, untested** - the authored sequence is what
the one-per-seat flow follows. The split entry is the MULTIPLAYER row now (2026-10-14; it was a SPLIT SCREEN row revealed under RACEBOX on HD),
and a windowed walk on 2026-10-13 went
join, Track Select, `Ship Select - Player 1`, `Ship Select - Player 2`, race. Omega: **ported** (corrected 2026-10-13: the earlier "not located" came through `scripts/psarc.py`) - `data09`'s `team_selection_definition.xml` authors `Team Selection`, `Team Selection Player 1` and `Team Selection Player 2` like HD's, `oag-omega` names it as `team_select`, and a race walks Track Select then Ship Select for one person and each seat for a split (see [split-screen.md](../rendering/split-screen.md), which lists what Omega's screen still lacks: model cells, preview, statistics). 2048: checked, string ids present, screens unread. Pure: authors no split screen
(`docs/formats/race-setup.md`), so it takes the one-per-seat order, chosen. PSP Pulse:
authors none either and **borrows the PS2's page for two seats** (above); three seats and more
take the one-per-seat order.

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
confirm without moving launches what is on screen. **Zone goes through Ship
Select too** (2026-10-26): the team is the one the race and its loyalty credit
use, and on Pulse the paint on the shared Zone hull. The original's
`Team Selection` runs in Zone with the livery cycler replaced by a fixed `Zone`
label and the four rating bars forced to 10
(`docs/ghidra/functions/psp-pulse-usa/zone-mode.md`); the screen draws both, the
preview is the team's own `Zone_FE.vex`, and the label has no arrows to press.
`oag_title::ZoneTeamStep` is the per-title answer: Pure's own chain has no team
step in Zone and keeps skipping it. Back from Track Select is the RACE
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
  composition order left open; the PS2's `Origin` is settled by
  [mode3d.md](../ghidra/functions/ps2-pulse-eu/mode3d.md) (2026-10-29).
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
- ~~Which curve the original's `Distance(m)` sums~~ - none: it prints the authored `<Physical Length>` (closed 2026-10-29, above).
- The `Loyalty` counter, once anything keeps one.
- The `Track Help` / `Team Help` overlays and `Pre Race Music Select`: read
  (they are on the same XML) and not drawn.
- The outline's glow.
- The `small` face: the footer's `Help` / `Music playlist` prompts are
  authored in it and drawn in the menu face scaled by 17/22, since that face
  is not loaded for the menus.
