# The race box's selection screens

**Status: both of Pulse's screens are read off the disc and drawn - the
panel, its rows, the arrows, the counter, the rating bars, the livery row
on the disc's own skin axis, the lap distance measured off each circuit,
and both preview meshes. Two things are known and unbuilt: the circuit
flythrough and the `Loyalty` bar.** Implemented in
[`oag_ui::picker`](../../crates/ui/src/picker.rs) (model, layout, draw),
[`oag_game::preview`](../../crates/game/src/preview.rs) (the 3D pass) and
`crates/game/src/main/session/picker.rs` (the flow). Captured headlessly
with `--menu-page track-select` / `--menu-page ship-select`.

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
| hex window | left, roughly (45..270, 55..200) | **not a widget** - the flythrough's frame |

**The circuit name wraps into whichever `Info Track N.x` block has the
right line count.** `Talon's Junction White` is three lines, `Moa Therma
White` two; a greedy word wrap against the panel's inner width (170 minus
the group's 14 on each side) reproduces both, which is what
`oag_ui::picker::wrap_name` does. Confidence 80 that this is the rule and not
only the result: two names is a small sample, and the executable's own
`Info Track %d.%d` formatter has not been read.

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

The flythrough behind the hexagonal window - detailed architecture, the
camera moving down the corridor - is therefore the **real circuit scene**,
or a purpose-built one this project has not located. It is not built here:
a track load is seconds of work per selection and belongs on a worker, which
is the shape `race::LoadWorker` already has and the picker does not yet use.
**The hex window draws nothing and the panel says nothing about it**, per
the rule for an asset that will not play. The frame image the window sits in
is not on this screen's XML either, so it is not drawn.

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
  ([menus.md](../architecture/menus.md#unlocks-our-race-box-offers-everything-deliberately)).
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
recorded as unlocated**. Read into `oag_game::catalogue::Team::rating`;
the bar is drawn at `rating / 10` of its width with the texture sub-rect
scaled the same way, so the segments stay aligned. Confidence 95.

**The craft is `Data\Ships\<team>\ship_FE.vex`** (6 meshes, 1,303 vertices
for Assegai), turning once every twelve seconds seen slightly from above -
the period and the pitch are this build's own readings of two frames a
second apart, not recovered numbers.

**The livery row is the skin axis, as on the original.** `Classic` is the
disc's own string id for the baseline paint (the Japanese plugin
translates it; English falls back to the id), and each further entry is a
`PI_ModelSkin` the team declares (`Alternative`), applied to the preview
hull through the same texture-slot swap a race makes
([ship-skin.md](../ghidra/functions/psp-pulse-usa/ship-skin.md)) and
stored as `race.skin`, which the launched race reads the way `--skin`
does. A team that declares no skin - every HD and 2048 team - gets the
title's *variant* axis on the same row instead, the RACE page's VARIANT.
No unlock is checked on either: this build's race box offers everything,
by design.

**Loyalty is not drawn - title, bar and backing together.** It is a per-team
counter the profile keeps and this build does not, and an empty bar would
read as a loyalty of zero rather than as an absence.

## The flow

`RACE` page `START` -> Track Select -> Ship Select -> the race. The same
three pages Pulse walks (`Single Player` -> `Track Creation` -> `Team
Selection`), with the RACE page keeping its rows: each picker writes the
setting its row reads, so a circuit picked on the screen is the one the row
shows afterwards and the one the race loads. Zone skips Ship Select, since
the mode forces the Zone hull. Back from Track Select is the RACE page;
back from Ship Select is Track Select. A title that authors neither screen
(`oag_title::FrontEnd::race_box` is `None` - Pure, HD, 2048 today) launches
from `START` as before.

## Open

- The flythrough, above. Needs the circuit scene on a worker and a camera on
  its spline; `race::LoadWorker` and `oag_trace track`'s spline reader are the
  two halves.
- Which curve the original's `Distance(m)` sums - see above.
- The `Loyalty` counter, once anything keeps one.
- The `Track Help` / `Team Help` overlays and `Pre Race Music Select`: read
  (they are on the same XML) and not drawn.
- The outline's glow.
- The `small` face: the footer's `Help` / `Music playlist` prompts are
  authored in it and drawn in the menu face scaled by 17/22, since that face
  is not loaded for the menus.
