# The original's menus

**Status: main-menu layout measured, transition measured. Both PSP titles'
own frames are built - Pulse's top bar and footer strips, Pure's rule lines
and background - and each has one open piece: Pulse's footer ticker/button
prompts, Pure's background texture itself.**
Implemented in [`oag_title::menu`](../../crates/title/src/menu.rs),
[`oag_game::menu`](../../crates/ui/src/menu.rs) and
[`oag_game::anim`](../../crates/ui/src/anim.rs).

This page is about how the *original* draws its menus. What this project draws
in them is a separate question with its own page - see
[the menus](../architecture/menus.md), whose first line is "ours, not the
disc's". That still holds: **the tree is ours, the presentation is the disc's.**

## How this was measured

`pulse-psp-usa.iso` under PPSSPP 1.20.4 with `--graphics=gles`, on an Xvfb
display at 960x544 - exactly twice the PSP's 480x272, so a capture divides by
two with no resampling - photographed with `import -display :97 -window root`.
Every coordinate below is PSP-space.

Frames come one per emulated frame rather than one per wall-clock moment, by
breaking on `Gfx_PresentFrame` (`0x0891e1b8`, from
[main-loop](../ghidra/functions/psp-pulse-usa/main-loop.md)) and photographing
at each hit. That matters: `import` needs about 200 ms a frame, which is most of
a 0.5-second transition, and a free-running capture yields two useless samples.

Four things cost time and are worth knowing before repeating this:

1. **`just drive menu` does not stop at the main menu.** Its one mode walks all
   the way into a live Time Trial. Drive the presses directly and poll
   `state_name()` until it reads `Main Menu`.
2. **The original enters an attract demo after 60 seconds idle on the main
   menu** (front-end time, measured to within one frame twice; the demo then runs
   120 s per circuit), which silently invalidates a capture left sitting.
   An earlier reading of this item said "about 120 seconds"; that is the demo's
   length, not the threshold
   ([attract-demo.md](../ghidra/functions/psp-pulse-usa/attract-demo.md)).
3. **Xvfb and PPSSPP must be `setsid`-detached**, or they die with the shell
   that launched them - taking the X server out from under the emulator
   mid-run.
4. `data/cache/fe-transitions/orig/*.png` are 250-byte blanks left by an
   abandoned run. **They are placeholders, not evidence.**

## Layout

**Confidence 95.** Authored values read out of
`Data\Plugins\PI001\GUI\Skin.xml` and `MainMenu_Definition.xml`; measured values
off the capture. The two agree wherever both exist.

| Thing | Measured | Authored |
| --- | --- | --- |
| left edge of the rows | x = 50.0 | `FEGlobals->MenuXOffset` = 50 |
| first row, glyph top | y = 40.0 | menu widget `y="32"` |
| row pitch | 28.0, over all seven rows | see below |
| rows on screen | 7 | 7 `<Entry>` elements |
| help text, glyph top | y = 54.0 | `helptext0 y="50"` |
| row font | - | `font="menu"`, 22px |
| title | `TitleXOffset` 50, `TitleYOffset` 0, `TitleScale` 1.0 | |

**`y` is the top of the line box, not of the glyphs.** The authored 32 draws ink
at 40, and the authored 50 draws the 13-pixel help text at 54. The offset is the
glyph's position inside its line box, which the `.fnt` already bakes into each
cell - so a renderer that draws a row at the authored `y` lands correctly
without applying anything.

### The row pitch is not the authored `gap`

**Confidence 80.** This is the one number that cannot be read straight off the
data, and reading `gap` as row spacing produces a visibly wrong menu.

| Screen | authored `gap` | `font` | face line height | measured pitch |
| --- | --- | --- | --- | --- |
| `Main Menu` | 15 | `menu` | 22 | 28.0 |
| `Racebox` | 15 | `menu` | 22 | 28.0 |
| `Single Player` | - | `small` | 17 | 23.0 |
| `Cell Setup` | 0 | `small` | 17 | 23.0 |

**pitch = the face's line height + 6**, exactly, on all four - while `gap` of 0
and 15 give the *same* pitch for the same face. Whatever `gap` is for, it is not
row spacing. The 6 is measured, not derived, and nothing here claims to know
where it comes from.

Line heights are this project's own readings of the `.fnt` files
([fnt](../formats/fnt.md)), confirmed again here by reading `line_height` back
out of `Atlas::from_font`.

### The help texts are per-row

Rows sit at 32, 60, 88, 116, 144, 172, 200; the seven `helptext` widgets at 50,
78, 106, 134, 162, 190, 218 - the same pitch, each 18 below its own row. Only
the selected row's is drawn, which is why two columns authored at the same `x`
never collide.

## Colours

**Confidence 90.**

| Role | Measured peak | Authored |
| --- | --- | --- |
| unselected row | rgb(64, 205, 230) | `FEGlobals->TextColor` `0xFF33A6B9` |
| selected row | `0xFFFFFFFF`, exact, on three independent cycles | - |
| help text | white | `0xffffffff` |
| screen title | black, on a light top bar | `FEGlobals->TitleColor` `0xFF000000` |

The unselected measurement is the authored colour times 1.24 on all three
channels equally, which is the lit backdrop showing through rather than a
different colour. **`TextColor` is confirmed.**

**Selection is a brightening toward white.** Not a bar behind the row, and not
the pink `MenuHighLightArrowColor` the palette declares - no pink appears
anywhere on this screen. This build drew an invented translucent bar until this
page existed.

The language picker follows the same rule since 2026-10-09 (Pulse's rows pulse by
the same `selected_ink`, no bar; HD and Omega, with no measured ink, keep a
band that is the pointer's own hit rect, chosen, not measured). Other titles'
pickers: Pure unchanged (static ink, no bar), Pulse PS2 as PSP, 2048 not
checked (its picker is not this widget), Omega shares HD's path but its disc
is `.pkg`-only and was not captured. See `frontend-boot.md`, "The Language
Selection screen".

### The highlight's pulse - period and depth measured 2026-09-05

**Confidence 90 for the period, 90 for the peak, 75 for the trough.** The two
single-frame samples this section used to carry - `rgb(157,255,255)` and
`rgb(107,226,247)` - are retired: both sit at or near saturation on the green
and blue channels, so solving `lerp(base, white, t)` per channel gives `t`
values that disagree by a factor of two between channels on the same sample.
That is the clipping showing, not a measurement of depth, and no curve
reconciles the two.

**Method**: PPSSPP under Xvfb, breaking on `Gfx_PresentFrame` (`0x0891e1b8`,
see [main-loop](../ghidra/functions/psp-pulse-usa/main-loop.md)) and
photographing the X11 root window at every hit - frame-accurate rather than
wall-clock sampled, so the screenshot cadence cannot alias against the
animation's own period. A first attempt (breaking on presented frames while
screenshotting between them, which costs real wall-clock time per frame) idled
past the 60 s attract-demo threshold partway through and
captured race footage instead of the still menu without any obvious sign in
the frame indices; it was discarded once a frame midway through showed a
racing circuit rather than `Main Menu`. The kept run combined "walk to Main
Menu" and "start capturing" in one script with no gap, reached the menu and
captured 150 consecutive presented frames (about 46 real seconds, well inside
the idle window) without ever leaving it.

**Period: exactly 33 presented frames, four cycles in a row.** Sampling a
300x30 px box mean over the selected row's own label (`RACE CAMPAIGN`) against
an identical box over an unselected row (`RACEBOX`) as a control: the control
never moved (a flat `rgb(18,46,51)` for the whole run bar one single-frame
outlier at frame 40, discarded), and the selected row's mean rose and fell in
a repeating wave whose troughs sat at presented frames 12, 45, 78, 111 and 144
- each exactly 33 frames from the last. At the front end's own measured 30 Hz
presentation, `33 / 30 = 1.1` seconds.

**Depth: `TextColor` to `0xFFFFFFFF`.** Rather than the diluted box mean, the
brightest pixel of the selected label's own ink was located (a grid search
over the "R" of "RACE") and sampled at every trough and peak: `(53,153,169)`
at all four troughs, bit-identical to the digit; `(255,255,255)`, exact, at
all three peaks sampled. The trough sits within single-pixel measurement error
of authored `TextColor` `(51,166,185)` - the largest channel deviation is 16 of
255 on blue - so the pulse is read as running between the row's own resting
colour and pure white, not between two otherwise-unexplained numbers.

**The shape between the two endpoints was not solved.** The box-mean sequence
crosses eleven discrete levels from trough to peak, each held one to three
frames, and the gaps between consecutive levels are not equal (8.6 to 12.9,
smallest at the ends) - enough to rule out a linear ramp, not enough samples to
name the real curve. `oag_game::menu::Skin::selected` uses a raised cosine
between the two measured endpoints and says so is an invented shape, the same
way `anim::Tween::eased`'s own curve is marked.

Recorded in `oag_title::MenuSkin::selected` (now the pulse's peak rather than
a static colour) and the new `selected_pulse_period_secs`, `Some(1.1)` on
Pulse only - see that type's own field docs. **Do not fill this number in for
another title just because Pulse has one.** HD authors no oscillation
anywhere in its front end (a 2026-09-05 whole-archive census - see
[hd-frontend.md](../formats/hd-frontend.md#three-questions-this-closes)) and
whether Pure's own, differently-directioned selected effect pulses at all is
still unmeasured.

## The transition

**Confidence 85.** A page change is a **zoom and crossfade**, not a slide.
`LeftLayer` - the only layer element in all 17 of Pulse's GUI files - names the
layer, not a direction of travel.

- the outgoing page **scales up and fades out**
- the incoming page **scales up from smaller and fades in**, at the same time
- the chrome - top bar title, footer - **crossfades in place without zooming**

The front end presents at **30 Hz**: the median gap between `Gfx_PresentFrame`
hits is 41.0 ms including a websocket round trip of roughly 8 ms. 60 Hz would
have measured about 24 ms.

Outgoing page, per presented frame, `Main Menu` to `Grid Selection`:

| frame | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 11 | 13 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| scale | 1.000 | 1.026 | 1.053 | 1.079 | 1.105 | 1.140 | 1.193 | | gone |
| alpha | 1.00 | 1.00 | 1.00 | 0.97 | 0.90 | 0.87 | 0.82 | 0.55 | gone |

The layer is gone by frame 13 or 14. **At 30 Hz that is 0.43 to 0.47 s, which is
the authored `transition="0.5"`** - so `transition` is in seconds, confirmed
rather than assumed.

### What is invented

**The easing curve.** Nothing in the data states one. The measurement shows only
that the motion *accelerates* - the scale steps are 0.026, 0.027, 0.026, 0.026,
0.035, 0.053 - so `Tween::eased` is the simplest curve with that property,
picked to match the shape rather than derived. **Confidence 30**, and it is one
function to change if someone reads the real curve out of the executable.

**2026-09-28: the per-widget fade mechanism `LeftLayer`'s own `transition`
attribute drives is now read from code, and it is exactly linear** -
`Widget_UpdateTransitionFraction` (`0x0888d8e4`), see
[race-box-screens.md](../ghidra/functions/psp-pulse-usa/race-box-screens.md#the-per-widget-fade-is-confirmed-linear-2026-09-28).
That does not settle this curve, though: a raw linear disable ramp off that
mechanism predicts alpha `0.267` at frame 11 of this page's own transition,
against the `0.55` measured above. The discrepancy is left open rather than
explained away - see that page's own account of what was ruled out and what
a live capture of the transition's first few frames would still need to
confirm.

**The zoom origin. Confidence 55.** Solving `p' = p0 + (p - p0) * s` across
frames 0-4 puts it near y=110 and x=120, neither of which is the screen centre
(240, 136); both sit nearer the centre of the menu's own content block. The
horizontal solve is the weaker of the two, the incoming page contaminating the
same crop. So the screen centre is ruled out and the exact point is not
measured.

## Pure is a different skin

Read off `pure-psp-eu.chd`, so this is measured rather than assumed. See
[pure-status](../formats/pure-status.md).

| | Pulse | Pure |
| --- | --- | --- |
| `Skin.xml` dialect | `<code>`-shortened | plain `<?xml` |
| `MenuXOffset` / `MenuScale` | 50 / 1.0 | 21 / 1.15 |
| `TitleXOffset` / `TitleYOffset` / `TitleScale` | 50 / 0 / 1.0 | 21 / 20 / 0.97 |
| `TextColor`, `TitleColor` | declared | absent from `Skin.xml` |
| `MainMenu_Definition.xml` | present | absent |
| layer element | `LeftLayer`, 17 files | none at all |
| `transition` values | 0, 0.2, 0.5, 0.7 | 0, 0.25, 0.3, 0.5 |
| menu `font` / `gap` | `menu` / 15 | `Title`, `Stats`, `scroll` / 0, 25 |

Every number they share, they disagree on. So this is a per-title table and not
a set of constants: `oag_title::MenuSkin`, filled in by `oag-pulse` and
`oag-pure` separately, with a test asserting the two never come to match. What
Pure has not been measured for is left empty rather than filled in from Pulse.

## The top bar and the footer's two strips - built 2026-08-25

**Not `topbarleft`/`topbarcenter`/`topbarright`.** That was a guess made
before `Skin.xml` was actually read; on the disc the top bar is **one**
`<Image>`, `x="20" y="0" width="224" height="24"`, and it and the footer's two
strips (`x="12" y="236"` and `x="12" y="249"`, both `width="454" height="14"`)
are three patches of one shared `Data\FE\Images\pulse_assets.mip` - the
footer pair sampling their own sub-rect (`U="5" V="27"` and `U="5" V="52"`)
of it. All three sit on `Top FE Screen->FE Screen`, the screen every menu
nests inside, three levels of anonymous (unnamed) `<Screen>` down from it.

**That nesting is why they were missing rather than wrong.** `Screens`'
widget collection recursed into a `<Viewport>` and an `<Animation>` but not
into a bare `<Screen>` used purely to group - so three anonymous wrappers
were enough to lose all three images with no error and no picture, just
silence. `collect_widgets` now recurses through an anonymous `Screen` the
same way it already did the other two, closing that gap for every title
`oag_game` reads, not only Pulse's. See `crates/game/src/screen.rs`'s own
`collect_widgets` doc, and `crates/hud/tests/pulse_frame_ground_truth.rs`,
which pins all three widgets' coordinates and sub-rects against the disc.

Wired through `oag_pulse::FRONT_END::menu_frame` (`oag_title::FrontEnd`'s
already-generic "the screen names its own frame" mechanism - see
`crates/game/src/menu/frame.rs` - the same one Wipeout HD's frame already
used, so nothing new needed building to draw Pulse's). The authored black
`TitleColor` draws now too: `oag_game::menu::skin::Skin::title_color` used to
have only two answers, this build's own substitute or (for a title with a
frame) the frame's own ink - and Pulse's marks carry no tint of their own, so
that would have put white text on the bar instead. It now checks the disc's
own declared colour first, which for both titles that have ever reached this
code (HD and now Pulse) is also the only branch either reaches in practice.

**The footer's own ticker text and button-prompt line are still unbuilt.**
The two strips are backdrop art; the moving/localised parts on top of them -
`<TextInfo>` (the tag and the scrolling news text) and the
`<NavigationController>`'s button prompts - are elements nothing in
`oag_game::screen` recognises yet, the same gap `crate::menu::frame`'s own
module doc already names for Wipeout HD's trial-build text and button
prompts.

## Pure's own frame, and its white background - built 2026-08-25

**A player-reported bug, not a handover thread**: Pure's menus drew on black.
`FE Screen` authors its own frame too - rule lines top and bottom, a scroll
arrow pair, `ArrowSelect`, a squiggle-text date/version strip, a barcode and a
world logo - wrapped one container deeper than Pulse's: `<BackgroundController>`,
which `Screens::collect_widgets` did not recurse into any more than it did the
anonymous `<Screen>` above, so all of it was silently dropped along with
`Main Menu`'s whole backdrop. Fixed the same way, and wired through
`oag_pure::FRONT_END::menu_frame` naming `FE Screen`.

**The background image itself stays unauthored, and is the one place this
build now states a measurement the disc's `Skin.xml` does not.**
`BackgroundController`'s own `BackgroundImage` - sized to the full screen -
names no `src`, the same "the engine carries a compiled-in default this
project has not found" gap already on file for `TitleColor`/`DesignColor`/
`TextColor`/`FrameLineColor`. A capture of `Main Menu` itself (`pure-psp-usa.chd`,
PPSSPP 1.20.4, 2026-08-25) shows solid white, not the black this build
cleared to for want of anything else - recorded as `oag_title::MenuSkin::background`,
`Some(0xFFFFFFFF)`, confidence 70.

**The same capture found the selected row drawn wrong too, and this is the
one that made the bug visible rather than merely incomplete.** Nothing had
ever measured Pure's own selected colour, so `selected: None` fell through to
this build's own substitute - white, the same colour Pulse's own capture
happened to need. Once the background above started drawing white, that
substitute became a white row on a white screen. The real colour is not
white at all: sampling `SINGLE PLAYER` (selected) against `MULTIPLAYER`/
`PROFILE`/`DOWNLOAD` (not) gives a *darker*, more saturated ink -
`#16AED1` (22,174,209) against `TextColor`'s own `#88D6E8` (136,214,232) -
the opposite direction from Pulse's brightening. `Some(0xFF16AED1)`,
confidence 65: one capture, effect rather than source, same basis as this
title's other pixel measurements.

See `crates/ui/tests/pure_frame_ground_truth.rs` for the frame's own
widgets pinned against the disc, and `crates/game/tests/menu_skin.rs`'s
`pures_main_menu_capture_measured_its_background_and_its_selected_row` for
the two colours.

**2026-09-08: the corner logo is found, the backdrop is ruled negative -
same scan, sized for a full screen instead of `TitleFrame`'s `512x128`.**
`BackgroundController`'s other image, `BackgroundTopRightImage` (the
"ワイプアウト" wordmark beside the swoosh logo, top right), names no `src`
either and is now filled the same way `TitleFrame` is: entry 27 of `Data.wad`
(hash `7ba78aca`), an exact visual match to a real `Main Menu` capture cropped
to the widget's own rect. `BackgroundImage` itself - the full-screen backdrop
- stayed unfilled on purpose: every `480x272`/`512x256`/`512x512`-shaped
candidate across all 361 decoded textures was checked against a fresh
`Main Menu` capture (PPSSPP under Xvfb, driven start to finish through the
websocket debugger's `input.buttons.press` rather than a keyboard binding),
and none of them is what the real screen shows - the background there is
flat white with nothing drawn on it, sampled at exactly `255,255,255`, which
is already what `MenuSkin::background` supplies as a plain fill. See
[`pure-status.md`'s own section on it](../formats/pure-status.md#fe-screens-corner-logo-backgroundtoprightimage)
for the full candidate table.

## Not built

- ~~The footer's ticker text and button-prompt line~~ - built since (`oag_ui_screens::campaign::footer`); what still differs is item 3 and 4 of "What still differs, ranked".
- ~~The highlight's pulse, for want of a period and a depth on Pulse~~ -
  measured 2026-09-05, see "The highlight's pulse" above, and built in
  `oag_game::menu::Skin::selected`. Its exact curve shape between the two
  measured endpoints is still invented, the same way `anim::Tween::eased`'s
  page-change curve is. **On Pure, whether the selected row pulses at all
  remains unmeasured** - its own effect moves the opposite direction
  (darkening, not brightening toward white), so Pulse's numbers must never be
  borrowed for it.
- **`BackgroundImage`'s own texture on Pure.** Its colour is measured (see
  above), and a full image-content scan of all 361 decoded textures against a
  real `Main Menu` capture found no picture at all - the screen is genuinely
  flat white. Whether the disc's engine ever assigns a texture there on some
  other profile or theme is unread; what changed is that the negative is now
  a checked one rather than an unlooked-for gap. `BackgroundTopRightImage`,
  the sibling image in the same container, **is** found this way - see above.
- **Cursor motion between rows.** There is nothing positional to move -
  selection is a colour - and no capture shows the brightness changing over
  anything but an instant.

## `<Text DisplayType="TELETYPE">` is drawn whole, as the original does - measured 2026-10-11

A `<Text>` can author `DisplayType` as `NORMAL`, `SCROLLING` or `TELETYPE`; Pulse authors 58 `TELETYPE` ones (button
prompts, screen titles, `BOOT_PRESS_ANY2`) and no `SCROLLING`. `oag_ui::screen::Text::display_type` carries the authored
value ([`DisplayType`](../../crates/ui/src/screen/display_type.rs)) and **nothing draws differently for it**, because
the original does not: the `Text` class's draw prints the whole string, `TELETYPE` only sets a flag bit nothing reads,
and a live PPSSPP capture shows the prompts and plate titles complete from their first frame (confidence 85 static,
70 live). Evidence, the flag table and the other titles:
[text-display-type.md](../ghidra/functions/psp-pulse-usa/text-display-type.md). HD authors 105 and its `Text` class is
unread; if a capture there shows a type-in, `display_type` is where it would start.

## Wipeout Pure types its description line in, and loops it - measured 2026-10-29

Pulse draws its `TELETYPE` text whole (above); **Pure's selection screens (Class, League, Tournament and Track; the main
menu's help line is seen typing too, `PLAY._`, in the walk's `main` frame, and is not wired) type the amber description along the bottom, one character at a time, hold it, clear it
and type it again**, for as long as the screen is open. Measured on PPSSPP v1.20.4 (software renderer, `InternalResolution 1`,
USA disc `UCUS98612`, muted, own profile), from 30 fps recordings of the 480x272 window and from stopped-CPU grabs. The emulator's
own clock (`cpu.status.ticks / 222e6`) ran at 1.0000 of wall time over two 52 s recordings. The code was not read (Pure's
`BOOT.BIN` is not open in Ghidra here); everything below is observed, confidence 70.

| Quantity | Value | Evidence |
| --- | --- | --- |
| rate | **58 characters a second** (17 ms each) | the loop period of Class Selection's 86-character line is 138.2 frames at 30 fps (4.607 s, 11 clears over 52 s), League Selection's 135-character line 163.2 frames (5.441 s, 10 clears): the 49-character difference is 0.834 s, 58.8 a second; eight stopped grabs of Class between 0.18 s and 1.32 s after the press fit 57.1 a second. One character per 60 Hz frame (59.94) is within a frame of both and is not excluded |
| hold | **3.12 s** from the last character to the clear, then it types again at once | the same two periods less their typing time (3.124 and 3.113 s) |
| start | **0.067 s after the pad press** | the eight grabs (0.082, 0.046, 0.044, 0.042, 0.073, 0.069, 0.099, 0.077 s); includes the press-to-screen latency, which cannot be separated |
| cursor | `_` after the last typed character, **solid while typing**, then **blinking 0.5 s on, 0.5 s off** | every one of the eight partial grabs shows it; after the line completes it toggles every 15 frames of 30. The blink is **free-running, not tied to the cycle**: cycles 2 and 3 of one 52 s League recording toggle in opposite senses (163.2 frames is not a multiple of 30) |
| wrap | the whole string is wrapped first, then revealed across the lines | Class keeps `AG` on line 1 while line 2 is still empty. **Whether a wrap's space costs a typed character is not measured**: ours counts it (chosen); leaving it out gives 57.6 a second from the same periods, also inside the held-out check |
| line pitch, advance | 10 px between lines (glyph rows 222, 232, 242); 7.6 px per character in a 7 px cap height | the `Scroll` face's own metrics. **Wired** (`pure-sr-track`, `pure-leftovers`): `LTe50325.fnt` at 0.8 on the snapped pen, lines `DESCRIPTION_LINE_GAP` apart (below, "Pure's text pens step up to whole pixels", "Pure's glyph quads are whole pixels") |

**The authored `<TextInfo delay="1.6">` matches none of these** (the start is 0.07 s, the hold 3.1 s) and is not used.
**Not measured: whether moving the cursor to another row restarts the line.** Ours restarts it on a row change (chosen, not
measured). **The blink's phase** is chosen: ours anchors it to the screen's clock zero.

**Held-out check.** Track Selection (about 100 characters, not used in the fit) predicts 53 typed characters 5.87 s in; the
frame shows 55 (`4.45K OF TUNGSTEN-ALUM HI-BOUNCE TM SNAKES PAST PRIME` and the `O` of the next line). League predicted 23 against 21 seen,
Class 9 against 10: all within two characters, 35 ms.

**Wired:** `oag_ui_screens::campaign::footer::typein` (the law, with its own unit tests) and
`description_draws(.., since, ..)`, which wraps the whole line and then shows the first `typed(total, since).chars` characters
and the cursor. Live, `PureChainStage` carries the seconds since the screen or row changed; the still takes
`--menu-picker-seconds` (`None` draws it settled, no cursor).

**The pair clock.** Pure has no recovered front-end clock or state name, so `scripts/psp-menu-pair.py` reads the emulator's tick
counter at the start of the pad press that enters the screen and again with the CPU stopped for the grab (`PURE_CLOCKED`):
`seconds_since_entry`. Its error is the press-to-screen latency (a frame or two) and the one frame the presented picture lags
the stop, about 35 ms, two characters of the line. Pulse's clock addresses (`0x08b317b0`) are not read on Pure any more.

**Pairs, description box `(18, 217, 450, 250)`** (the differing share counts pixels more than 40 grey apart; ours rendered
settled against ours typed at the original frame's seconds, both against the same original frame):

| Pair | Seconds since entry | Whole frame before | Whole frame after | Box before | Box after |
| --- | --- | --- | --- | --- | --- |
| Class Selection | 4.82 | 3.63 % | 2.58 % | 13.3 % | 3.7 % |
| League Selection | 5.90 | 4.51 % | 2.82 % | 21.0 % | 5.6 % |
| Track Selection | 5.87 | 6.39 % | 5.58 % | 16.9 % | 9.5 % |
| Tournament Class | 4.82 | 3.60 % | 2.55 % | 13.3 % | 3.7 % |
| Tournament Selection | 5.87 | 4.70 % | 3.02 % | 20.9 % | 5.6 % |

Mean grey whole frame, ours settled / typed / original: League 246.1 / 247.3 / 247.1, Track 244.6 / 245.2 / 246.3, Tournament
245.9 / 247.2 / 246.9. What stays in the box is the face (ours the menu face, the original the thin `Scroll` face, so a
wrapped line breaks one word later in ours: `EACH TRACK` against `EACH`) and Track's second line (ours wraps `PRIME` and
`OCEAN` apart). Cross-title: Pulse authors 58 `TELETYPE` texts and types none (checked, differs); HD, 2048 and Omega author
`TELETYPE` too (HD 105) and their `Text` class is unread (not checkable here).

## Two faces, not one swapped for the other - measured 2026-09-21

**The front end draws every label in one 22px face; the original draws each
widget in the face its own `font=` role names, mixed case where that face has
lowercase.** `Cell Selection`'s footer alone (`Confirm`/`Back`, both
`ControlTextConfirm`/`ControlTextBack` in `Skin.xml`'s
`NavigationController`, authoring no `font=` and falling to `Default`) is
fixed as of this pass; `Grid Selection`/`Cell Selection`'s own
`n="default"` widgets - `Speed class`/`Laps`/`Weapons`/`Points`/`Best` and
their values - are not, because they draw through `oag_ui_screens::campaign::draw`,
a file this lane does not own. See `docs/ui/campaign-screens.md`'s own
dated paragraph.

### The role table

Every `<Font>` slot Pulse's language plugins fill, its resolved `.fnt`, that
face's own line height (read off the `.fnt` header, matching `Atlas::from_font`),
and whether **the disc's own font** carries distinct lowercase glyph art -
measured directly off each `.fnt`'s glyph table (`crates/game/tests/font_roles_ground_truth.rs`'s
`menu_and_small_have_no_lowercase_art_and_default_does`), not inferred from a
picture. Confidence 95 - a disc-backed test asserts it on both PSP pressings.

| Role (as authored) | `.fnt` | Line height | Lowercase has its own art? |
| --- | --- | ---: | --- |
| `Default` | `pulse_text.fnt` | 13px | **Yes** - every lowercase ASCII letter has its own `(u0, v0, width, height)` box, distinct from its uppercase twin |
| `menu` | `Pulse_20.fnt` | 22px | No - every lowercase letter shares the *identical* box its uppercase twin has |
| `Small` / `Title` | `Pulse_14.fnt` | 17px | No - same as `menu` |
| `HUD` | `PulseHud.fnt` | 25px | Not checked (HUD text is numerals and short caps labels; out of this lane's scope) |
| `HUDSmall` | `small.fnt` | 10px | Not checked, same reason |

`pulse_text.fnt` is the **only** Pulse face with real lowercase glyph art.
This is not a code bug and not a folding decision this build made -
`oag_ui::font::Atlas::cell` already prefers a real font's own glyph over
folding it (see `crates/ui/src/font.rs`'s "Folding, and the letter that
disappeared") - it is what the disc's own three menu-side faces ship. A
`menu`- or `Small`-role widget authored in mixed case (`"Single Race"`,
`"Time Trial"`) draws in caps on **real hardware too**, because the face it
draws in has nothing else to draw. Confusing that with a bug is exactly how
the previous reading of this defect over-scoped its own fix.

### The fix: a second atlas, not a swapped one

The menu stage's renderer bound exactly one atlas (`rows_face`, the `menu`
role's `Pulse_20.fnt`) as its whole glyph source before this pass -
`crates/game/src/app/session/menus.rs`'s `Renderer::new` call - with every
other role's text drawn through it too, scaled by a ratio
(`oag_ui_screens::picker::FaceScales`/`crate::picker::FaceScales::face_scale`)
that changes *size* and never *which glyphs*. Swapping which atlas is
primary was tried and reverted in this same pass: `Grid Selection`'s own
`Title`-role mode name (`font="Menu"`, e.g. `"GRID 1"`, `"Time Trial"`) is
authored and drawn through `oag_ui_screens::campaign`, a file this lane cannot
touch, and it reads `text.font` through a match that only recognises
`"default"`/`"small"` - anything else, including `"menu"`, draws through
whatever the primary atlas is at scale `1.0`. Making `Default` primary would
have made that one widget draw *through* `Default`'s own lowercase-capable
glyphs at the old, unadjusted scale - a **new**, un-measured divergence
(mixed case where the reference and real hardware both show caps), on a
widget this crate cannot repair, for the sake of covered ground this crate
also cannot repair. So the primary atlas is unchanged, and a **second**,
`Default`-role atlas is loaded beside it - `crates/game/src/render/face.rs`'s
`Renderer::set_face_atlas`, `crates/game/src/boot/fonts.rs`'s
`face_atlas_slot` - the same slot Wipeout HD's chrome title already used for
its own `Title` role (the two are mutually exclusive per title, which is
what lets one slot serve either). `Draw::FacedText`'s own `role` is now
checked against whichever role that slot loaded (`crates/game/src/render.rs`),
where it used to be ignored outright.

**What this reaches, today:** anything this lane owns that draws a
`"default"`-labelled string through `Draw::in_role` -
`oag_ui_screens::campaign::footer`'s `Confirm`/`Back`/ticker prompts, and Pulse's own
per-row subtitle (`oag_title::HelpText`, `helptext0`..`6`, whose `scale`
dropped from a derived `13.0 / 22.0` to `1.0` now that it draws its own face
rather than a scaled-down `menu` one - see that field's own doc). **What it
does not reach:** every `n="default"` widget inside
`Data\Plugins\PI001\GUI\CellMode_Definition.xml` that `oag_ui_screens::campaign::draw::text_draw`
draws - `Speed class`, `Laps`, `Weapons`, `Points`, `Best`, and their
values - which is one edit (`text_draw`'s single `Draw::Text` literal
becoming a role-aware constructor, mirroring `Draw::in_role`) in a file
outside this lane's boundary.

**The Layout table above still holds, re-verified rather than assumed.**
This section's own "help text, glyph top" row (`y = 54.0`, off `helptext0`'s
authored `y="50"`) was measured while the subtitle drew through `Pulse_20.fnt`
scaled to `13/22` - the switch to `pulse_text.fnt` at its own native scale
changes *which* atlas supplies that inset, so the row was re-checked rather
than trusted: `mainmenu-pulse-after.png` (a scratch directory, not kept,
1440x816 at 3x native) puts the subtitle's own ink between capture rows
161-191, native `y = 53.7`-`63.7` - matching the documented `54.0` top edge
exactly. `pulse_text.fnt`'s own baked-in vertical inset happens to equal
`Pulse_20.fnt`'s scaled one here; nothing in this table needed regenerating.

**A pre-existing, separate defect, found while checking the above and not
this lane's mechanism:** Cell Selection's `Points` value renders `0/3` as
`OS3`, and `DifficultyButton`'s literal `string="Change Difficulty"` as
`CHANGE JIFFICULTY` - present in this build before this pass's own changes
(confirmed: `cellselect-usa-before.png`, captured before any commit here).
`crates/game/src/render/text.rs`'s own `push_text` was probed directly and
the *source* string is exactly right both times (`"0/3"`, codepoints `[48,
47, 51]`; `"Change Difficulty"`, `D` at its correct position) - so the
string is not the problem. The obvious next hypothesis, that `Pulse_20.fnt`
gives `'0'`/`'O'`, `'D'`/`'J'` or `'/'`/`'S'` the same glyph box the way it
does every lowercase/uppercase pair, is **checked and false**: all three
pairs have distinct, unrelated `(u0, v0)` in the raw `.fnt` table. The
actual mechanism is not identified - left as "cause unknown" rather than a
plausible-sounding guess, for whoever picks up `oag_ui_screens::campaign::draw`'s
own fix above.

## What still differs, ranked - measured 2026-10-08

**Method.** Pulse PSP USA under PPSSPP 1.20.4 with `SoftwareRenderer = True`
in its own ini (the key is set and read back; the log does not name the
backend, so that it took effect is not independently confirmed), 1280x720 Xvfb,
the game's 960x544 area cropped at `+160+88`. `scripts/psp-frontend-capture.py`
walked 65 states; the transition burst is
`scripts/psp-menu-transition-burst.py` (30 `Gfx_PresentFrame` hits after one
`cross`, end state asserted as `Grid Selection`). Ours is `oag-game
data/images/pulse-psp-usa.chd --menu-page <id> --size 960x544`. Sheets (original
above, ours below) are in a scratch directory, not kept: `main`,
`custom`, `track`, `team`, `grid`, `cell`, `boot`, `options-orig` against
`options-ours`, `burst-orig` against `burst-ours` and `burst-ours-options`.

**What is not compared, and why.** A `--menu-page` still runs no clock and no
state machine, so the selected row's white pulse, the ticker's scroll, the
backdrop's ship fly-by and the 3D previews' rotation are not judged from it;
the original's backdrop movie plays a ship across the screen (burst frames),
ours plays the same movie. No `--press` walk stops on `Main Menu` (the first
walk launches a race), so a live frame of ours was not taken. The original's
loading screen and EndRace were not captured this pass (the loading wave is
in [loading-screen.md](../ghidra/functions/psp-pulse-usa/loading-screen.md); EndRace needs a finished race).

Ranked by how much a player notices, ours first listed worst:

1. ~~**Our Race page is not the original's Racebox settings page.**~~ **Closed 2026-10-08** - see "The Racebox settings page" below.
   *(was:)* The disc has
   `Racebox` (Custom Race / Load Grid / Edit Grid) then a settings page whose
   rows are label left, a `<` `>` selector then the value at x of about 240,
   with a thin rule between rows and the focused row in white. Ours is one
   `RACE` page with the value right-aligned at the far edge, greyed rows and no
   rules. Structural (our page tree is our own, `menus.md`); the rule lines and
   the selector arrows are disc art and not drawn.
2. ~~**The top-bar title is set in the wrong face on every page.**~~ **Closed 2026-10-08 on both PSP pressings**: `oag_pulse::frontend::MENU_SKIN::title_font` is `Some("Title")` and the renderer's third glyph slot carries `Pulse_14.fnt` (`oag_game::boot::fonts::third_atlas_slot`; `body_font` keeps `Default` in the second). The PS2 port took it on 2026-11-27 ("Pulse PS2's headings draw in the Title face" below). *(was:)* The original's
   `MAIN MENU`/`TRACK SELECT`/`RACE CAMPAIGN` are the `Title` role,
   `Pulse_14.fnt` (17px): 9 glyphs span 152 px of the 960-wide crop, 17 px a
   glyph. Ours draws `Pulse_20.fnt` (22px): 12 glyphs span 305 px, 25 px a glyph,
   a 0.68 ratio against 14/20 = 0.70. `oag_pulse::frontend::MENU_SKIN::title_font`
   is `None` with a note that nothing had re-checked the capture against the
   smaller face; this is that check, and the answer is the original is the
   smaller one. Not fixed this pass: Pulse needs the `Title` face and the
   `Default` face loaded at once and the renderer has one secondary slot
   (`boot::fonts::face_atlas_slot`), so it is a third slot, not a flag.
3. **The footer ticker is set in the wrong face, and the profile tag is empty.**
   The original's ticker is lowercase `pulse_text.fnt` (`Default` role, the
   same face as `Help`/`Confirm`/`AI difficulty` beside it), dark ink, scrolling
   in the strip; the tag at its left is the profile tag in white caps (`AAA`
   here). Ours drew the tip in `menu`-role capitals at 17/22 scale, which on
   `Main Menu` ran past the strip's ends. The tag draws nothing (no honest
   string; `campaign-screens.md`). **The face is fixed 2026-10-08**: the ticker
   now draws in the `Default` role (`TickerLayout::read`'s `font`, which was a
   chosen `"small"` and is now a chosen `"default"` corroborated by this
   capture; `before`/`after` in `sheets/footer-compare.png`: lowercase, inside
   the strip, ink and size matching). **The tag now draws the stored profile tag** at the authored
   `(30, 235)` in the `Default` face (`NavigationLegend::set_tag`, `docs/ui/player-name.md`); the
   colour is the file's `BackgroundColor`, and neither face nor colour is checked against the original's `AAA`. On the campaign
   pages a `--menu-page` still draws no tip at all (`static_footer_overlay`
   passes an empty rotation), so only the live stage and `--menu-page main`
   show the new face. *(Our `Confirm` label was
   smaller than the original's and sat higher against its button glyph, a
   shrink-to-fit workaround; closed 2026-10-29, it draws as authored.)*
4. ~~**Track and Ship Select labels are capitals where the original is mixed
   case.**~~ **Closed 2026-10-08**: `font="default"` text draws in the `Default` atlas at native size (`FaceScales::native_default`), the pickers carry the `Confirm`/`Back` legend and the ticker (`MenuStage::picker_footer`, and the same for `--menu-page`), and the slash in the counters is the real one. *(was:)* `Distance(m)`, `Lap record`, `Classic`, `Speed`, `Thrust`, the
   `Help`/`Music playlist` prompts are `Default` role on the disc. Ours draws
   them in the capitals-only menu face. The `Confirm`/`Back` prompts and the
   ticker are missing on our pickers' footer. Same cause as item 3 on another
   widget family.
5. **Page transitions.** The original zooms the outgoing page up while fading
   it and the incoming page up from smaller, 14 to 15 presented frames at
   30 Hz (`burst-orig`: frames 0-3 barely move, then the zoom accelerates, gone
   by frame 13). Ours (`--menu-anim-phase`) draws only the arriving half on
   menu pages, and **draws no transition at all on the campaign and picker
   pages**: `grid-select` at phase 0.0 through 1.0 is the same picture. The
   easing curve is still the invented one (confidence 30); the burst frames at
   a scratch directory, not kept are the data a fit needs, a
   per-frame scale and alpha read off them, and are not fitted yet.
6. **Selected-hex and row glow.** The original's focused grid hex is a white
   hexagon with a bloom; ours is a flat cyan outline at pulse phase 0. The
   pulse is measured (`menus-original.md`, "The highlight's pulse") so this is
   likely the still's phase, not a missing effect; a live frame is needed to
   say.
7. **Boot title.** `--screen "Show Logo"` draws the logo on black with no teal
   backdrop, no copyright lines and `Press START button` in the small face;
   the original has the backdrop, two copyright lines and `PRESS START
   BUTTON` in the menu face, larger and with a glow. This is the debugging
   view of one screen and not the boot flow, so it is unconfirmed as a player
   gap.
8. **Notes, not ranked.** ~~The track circuit's distance reads 5178 on the
   original and 5094 on ours for Talon's Junction White~~ (closed 2026-10-29: the original prints
   the circuit's authored `stats.xml` `<Physical Length>`, now read; the `Distance(m)` label and
   the `-.--.--` empty record are wired too, `selection-screens.md`). A circuit count of 1 / 3 on a fresh
   original profile against 1 / 11 on ours is our unlock default. `/` in
   counters and the HUD lap draws as a long-s glyph on the pickers but as a
   plain slash on the grid page.

**What is not missing:** the top bar and both footer strips, the row pitch and
colours, the help subtitle, the hex cluster and its panels, the `Confirm`/`Back`
prompts on the campaign pages and the backdrop movie all line up with the
original on the sheets.

## The Racebox settings page - 2026-10-08

The original's `Single Player` screen (`RaceBox_Definition.xml`) is the page
`RACEBOX` > `CUSTOM RACE` opens. What it authors, **read off the disc at boot**
(`oag_ui::menu::SettingsLayout`, `crates/game/tests/pulse_settings_ground_truth.rs`,
both PSP pressings), confidence 95:

- an `<item OffsetX="50" OffsetY="33">` of twelve `<Image>`s: the two halves of
  one fading rule per row boundary (`Color1`..`Color4`, `0x00ffffff` to
  `0xffffffff`, 190 wide, 1 high, a one-texel quad of `pulse_assets.mip` at
  `U=505 V=1`), at y = 0, 23, 46, 69, 92, 115 - six rules around five rows;
- labels at `FEGlobals->MenuXOffset` (50), values in `<List>`s at x=250, rows at
  y = 35, 58, 81, 104, 127 - a pitch of 23 - all `font="small"`, which is
  `Pulse_14.fnt`, the `Title` face (17 px), not `menu`'s 22;
- `arrowcolor` = `TextColor`: the step arrows are the executable's.

What it does **not** author and was measured instead (a PPSSPP 1.20.4 software
capture, 2x, `custom.png`; confidence 80 - the
`List` widget's own draw code is unread):

- the arrows: two 9x10 texels at `U=289 V=76` and `U=314 V=76` of
  `pulse_assets.mip` - the same two `InGame_Definition.xml` draws as plain
  `<Image>`s for the pause bar - left edge 23 and 13 units before the value, 5
  below the row's text top; a focused row draws a lit halo under each, 20x22 at
  `U=285 V=98` and `U=308 V=98`, centred on the arrow (the halo's rect is read off
  the sheet's alpha, the one number here with no disc authoring behind it);
- colours (sampled): labels teal on every row, **the focused row's value alone
  whitens** (a white peak with a glow, 234/249/249 against the teal 48/157/174),
  a disabled row's label and value both a dark grey (34/34/34) with no arrows;
  the left arrow is half-bright on a list's first entry, which is every row in the
  capture - ours wraps, so both stay lit;
- the capture's rule peaks at 178-190 where ours is 255: the capture is filtered
  (a 1 px rule spread over 4 rows of 2x pixels, energy conserved), not dimmer.

**What is ours, chosen, not measured:** the `race` page of this build's tree
stands in this screen's place (`oag_title::MenuSettings::pages`); its `START` and
`BACK` rows continue the pitch below the last rule with no rules of their own, and
whiten their own label when focused since they have no value; the arrows' hit
areas are the arrow's width by the row band between two rules. Pointer: a click
on a row activates it as before, on an arrow steps the way it points.

**Checked against the other screens that draw this idiom** (census of every
`*_Definition.xml` on the PSP disc for `arrowcolor` lists): `Settings` (Game
Options) in `Additional_Definition.xml` - value column x=310, first row y=27,
pitch 23, **no rules**; `Cell Setup` in `RaceBox_Definition.xml`; `InGame
Settings`/`MP`/`TT`/`Photo`; `Tournament C`, `Team Selection` and `Pre Race Music
Select` in `Selection_Definition.xml`. Ours has no page that stands in for any of
them, so: **checked, applies, not wired**. The Options hub (`GAME OPTIONS`,
`CONTROLS`, `MUSIC PLAYLIST`) is a plain list on the disc and stays one.
**PS2 Pulse**: `oag_pulse::frontend::PS2_MENU_SKIN::settings` is `None` - the PS2
screen is in its own 640x448 grid and the layout reader keeps numbers in the
screen's grid; **checked, applies, not wired**. **HD, Omega, 2048, Pure**: their
settings rows are the executable's `List` blocks (`draw_list_rows`) or Pure's
own list; **checked, differs**, untouched - HD's `--menu-page options` and `race`
stills are byte-identical before and after.

## Game Options: the original's list against ours

**Confidence 92 for the rows and entries** (read off the disc, 2026-11-06): `Data\Plugins\PI001\GUI\Additional_Definition.xml`,
`Settings` screen (title `OPT_GAME_OPT`, "Game options"), `oag-wad cat <image>:PSP_GAME/USRDIR/Data.wad <name> --expand` on `pulse-psp-usa.chd` and
`pulse-psp-eu.chd` (byte-identical after expansion) and `oag-wad cat <image>:54748/WADS2.WAD <name> --expand` on
`pulse-ps2-eu.chd`. Display text from `Data\Plugins\PI012\entries.xml`. Every list is `save="true"`: the original
persists each into the profile, as ours persist into `settings.toml`.

| Row (`idstring`, text) | Entries, default first where the XML says so | PSP | PS2 | Ours |
| --- | --- | --- | --- | --- |
| `OPT_MUSIC_VOL`, Music volume | 0, 25, 50, 75, 100 % (default 100) | yes | yes | present: AUDIO page, `audio.music_volume` (0, 25, 50, 75, 90, 100) |
| `OPT_SFX_VOL`, Sfx volume | 0, 25, 50, 75, 100 % (default 100) | yes | yes | present: AUDIO page, `audio.sfx_volume` (same steps) |
| `OPT_MUSIC_PL`, Music playlist (list `Music Order`) | `OPT_SEQ` Sequential, `FE_RANDOM` Random (default sequential) | yes | yes | **added**: MUSIC ORDER on the AUDIO page, beside the volumes the disc's screen also holds (`audio.music_order`: sequential default, random). Random plays a shuffled deck, every song once before any repeats (`oag_sound::shuffle`, `docs/architecture/music-threads.md`, "Music order"). Shown on every title: the disc authors it on Pulse, the shuffle is wanted for every soundtrack. The label is ours (`OAG_AUDIO_MUSIC_ORDER`), not the disc's `OPT_MUSIC_PL` text: a menu row has no disc-string mechanism |
| `OPT_CAM`, Camera | `OPT_INT` Internal, `OPT_CLOSE` Close, `OPT_FAR` Far (default close) | yes | yes | present: `graphics.camera_view` (internal, close, far) |
| `OPT_HUD`, Dynamic HUD | `FE_DEFAULT`, `FE_CONCAVE`, `FE_FLAT`, `FE_OFF` (default `FE_DEFAULT`) | all four | three: no `FE_FLAT` | present, **all four**: `FE_CONCAVE` bends every HUD vertex towards the middle row (`Hud_WarpVertexY`, `y += 35 sin(pi x / 480) (1 - y / 136)`), `FE_FLAT` never, `FE_DEFAULT` in the cockpit view only; seen live and matched to the pixel ([hud-game-options.md](../ghidra/functions/psp-pulse-usa/hud-game-options.md#the-warp-2026-11-19)). Every other title offers `default` and `concave` (ours) |
| `OPT_MP_TAGS`, Multiplayer tags | `FE_ON`, `FE_OFF` (default on) | yes | **no row** | **added**: MULTIPLAYER TAGS, a player's name over each other person's craft in a multiplayer race (`oag_hud::player_tags`) |
| `OPT_GHOST_VIS`, Ghost ships visible | `FE_ON`, `FE_OFF` (default on) | yes | yes | **added**: GHOST SHIPS VISIBLE, hides the ghost's draw (`Race::set_ghost_visible`) |
| `OPT_AL_GHOST`, Auto load ghost | `FE_ON`, `FE_OFF` (default on) | yes | **no row** | **added**: AUTO LOAD GHOST, gates loading the stored lap when a Time Trial or Speed Lap starts |
| `OPT_SCREEN_SIZE`, Screen size (HD's slider) | not authored on Pulse: no row in any front-end XML or in the executable | no | no | **added**: SCREEN SIZE, the shared slider shared with every title (`hud_safe_area`, 80 to 100, default 100), [hd-hud.md](../formats/hd-hud.md#screen-size-on-every-title) |
| `OPT_LANG`, Language | the installed languages | yes | yes | present: `language` |

Where the two pressings differ the PSP list wins (the project rule for anything invented for Pulse), so the PS2
pressing offers both ghost rows and all four Dynamic HUD entries (its executable runs the same warp and still
compares `FE_FLAT`).

**Behaviour the rows have in the original** (`docs/ghidra/functions/psp-pulse-usa/ghost.md`, "Best lap, restarting at the
line" and "Saving and loading"): `Ghost_Update` sets the ghost's visibility from the options byte `options+0x45c`
(`FUN_08809bc8` reads it from the profile, anything but `FE_OFF` is on); the ghost is still raced, recorded and replaced
by a quicker lap while hidden. `Ghost_Construct` starts the stored-ghost load only when `Auto Load Ghost` reads `FE_ON`.
Ours: GHOST SHIPS VISIBLE gates `Race::ghost_pose`, the one place every draw path asks for the ghost, so no hull and no fade
is built while it is off; AUTO LOAD GHOST gates `oag_game::ghosts::auto_load`, and a run with it off still records and still
saves a lap that beats the stored one. A `--ghost <file>` on the command line is an explicit request and ignores AUTO LOAD
GHOST. Both rows are `Title` data (`oag_title::race::GhostOptions`, Pulse only), seeded and shown only where the title
lists them, and default to on, so no title's behaviour moved.

**MULTIPLAYER TAGS is wired** (`docs/ghidra/functions/psp-pulse-usa/hud-game-options.md`, confidence 80): a white name 2 units
above each other person's craft, fading in at 1.2 per second, fully opaque to 20 units and gone by 60, forced to 0 by the
option. The row is `Title` data (`GameOptions::multiplayer_tags`, Pulse only; the type was `GhostOptions` and covers the three
Pulse rows), default on, seeded and shown only where the title lists it, and not hidden under `--original-menu` (that flag
only drops our extra tiles). The text comes off the disc's `MPTag_HUD.xml` (`PlrTag0`: `HUD` face, scale 0.5, centred, white,
border alpha 0.25; the eight labels share it, a ground-truth test pins that) and is placed where the lock-on reticle is,
projected through each view's own camera at the craft's drawn pose.
**Read from the executable** ([hud-game-options.md](../ghidra/functions/psp-pulse-usa/hud-game-options.md), "Multiplayer Tags"):
the 2-unit raise runs along the world's Y axis, the local player's own craft never has a tag, and the gate
(`FUN_0883e37c`, confidence 80) is the track-section row: a craft is tagged only where the viewer's section row holds its
section. **Chosen, not measured:** only other person-flown craft are tagged (the original also tags AI craft; whether a
multiplayer race has any is unread); a name is the lobby's where the race has one, else seat 1's stored player name (`SimRace::set_person_names`), and otherwise
`Player N`, the maintainer's stop gap (the original falls back to one shared string, `ONL_CRJ_PLAYER`, for every
unnamed slot). There is no occlusion test beyond that section row (a tag shows through a wall inside a visible section, as the
original's does). A craft behind a view's camera has no tag in that view.

**Checked against the other titles** (the Pulse rule: an invented Pulse row is checked for the others, not widened to them):

- **Pure** ships no `Additional_Definition.xml`: its `Data.wad` (841 entries) has no front-end XML at all, so its settings
  list lives in the executable and is **not checkable** from the data. The rows stay off (`GhostOptions::NONE`).
- **HD** (`docs/formats/hd-frontend.md`, `DATA06`'s `Additional_Definition.xml`): `Multiplayer Tags` and a ghost row
  (`GhostOption`, "Ghost Ships Visible") exist, with no `Auto Load Ghost` and no `Dynamic HUD`. **Checked, applies, not wired**:
  HD races a ghost (Pulse's recipe, chosen not measured) with no way to hide it. `Multiplayer Tags` is also listed: **checked,
  applies, not wired** (`HudLayouts::player_tags` is `None` for HD, whose `mptag_hud.xml` and the 1920x1080 label face are unread
  here; the engine path is title-blind once a title names the layout and lists the row). **Read 2026-11-20
(`EBOOT-ps3-hdfury-eu.elf`)**: HD's `Hud_Update` (`0x0009e3d0`) reads the `Multiplayer Tags` list value (the string at
`0x0077fd58`, referenced at `0x0009f8e8`) and compares it with `FE_OFF`, so the option acts there (confidence 70; the
byte it writes and the per-frame tag update were not followed). Pulse's section-row gate does **not** transplant: HD's
visibility is one bit per `.rcsmodel` chunk per cell, not 64 sections ([hd-pvs.md](../formats/hd-pvs.md)), so HD's gate
(if it has one) is a different test: **checked, differs** for the gate, **applies, not wired** for the row.
- **Omega**: Omega's front end is HD's (`PI001`), but nobody has read Omega's own `Settings` list: **not checkable** from this
  lane. Its pause screen authors `FE_GHOST` (the ghost ship's visibility, seen live on shadPS4 in Time Trial and Speed Lap,
  `docs/ui/pause-menu.md`, "Omega"), so this build offers GHOST SHIPS VISIBLE there on that evidence alone, **inherited from HD**
  for the options row; `Auto Load Ghost` and `Multiplayer Tags` stay off.
- **2048**: its `Settings` list is unread too: **not checkable** from this lane, named in the handover thread.

## Reproducing this

```sh
setsid nohup Xvfb :97 -screen 0 960x544x24 &
setsid nohup env DISPLAY=:97 SDL_VIDEODRIVER=x11 PPSSPPSDL --graphics=gles \
  --appendconfig=/tmp/debugger.ini --fullscreen data/cache/pulse-psp-usa.iso &
```

with `/tmp/debugger.ini` holding `RemoteDebuggerOnStartup`,
`RemoteDebuggerLocal` and `RemoteISOPort = 47810`. See
[the PPSSPP debugger](../reverse-engineering/ppsspp-debugger.md).

Ours, for comparison:

```sh
just play usa --screenshot /tmp/ours.png --menu-page main --size 480x272
just play usa --screenshot /tmp/mid.png --menu-page options --size 480x272 \
  --menu-anim-phase 0.35
```

`--menu-anim-phase` exists because `--ticks` does nothing alongside
`--menu-page`: that path draws one frame and runs no clock, so without it a
still cannot show a transition at all.

## Pure's `Stats` and `Scroll` face is one small letter-spaced file (2026-11-01, `pure-menu-pairs`)

Pure's `Stats` and `Scroll` roles resolve to `LTe50325.fnt` (the table in `oag_ui::language::roles`; `font_roles_ground_truth::pures_stats_slot_is_its_own_face_and_the_scroll_face` pins it on both pressings). Every `font="Stats"` widget of a team's `screen.xml` (the stat labels and `n/5` values, scale 0.8) and of a circuit's (`RACE RECORD`, `LAP RECORD`, `LENGTH`, `HEIGHT` and their values), the footer's `Confirm`/`Back` (`font="scroll"`) and the typed description all print in it: in a PPSSPP frame at 8x they are 5-pixel capitals with a wide letter space and an upper-case `M` for the unit, where this build had drawn them in the menu face (`FX300ANG.fnt`). It rides the renderer's third glyph slot (`oag_title::MenuSkin::stats_font`, Pure only: its `Buttons` role does not exist, so the slot is free), and a `Draw::FacedText { role: "Stats" }` falls back to the menu face on a title that loads no such face. The `<Menu>` rows of `Team Selection` name no `font`; they draw in the menu face at `MenuScale` 1.15, 18 grid lines apart (measured 18.2 on the frame, the chain's own `FlowFiles::row_pitch`). Confidence 90 (a frame measurement, one capture each).

**Open, same capture:** the record's separators read `0 . 00 . 00` with small mid-height dots and wide gaps at 8x, which is neither this build's `0.00.00` (baseline dots) nor the `0'00"00` the 2x reading suggested; no Latin-1 character drawn in `LTe50325.fnt` as the separator matches the frame (a sweep, 2026-11-01), so the original's format string is not recovered.

## A viewport clips its images (2026-11-23, `pulse-psp-track`)

Pulse's `Skin.xml` draws its two footer strips as 14-row images (`pulse_assets.mip` at `v=27` and `v=52`, `height="14"`) in
`<Viewport>`s authored `y="236" height="13"` and `y="249" height="13"`. Measured on PPSSPP's frame of Track Select (confidence 85, four
grabs, and the Ship Select, Grid and Cell pairs read the same): the original's row `y=249` holds the lower strip's first row (grey 72 at
`x=200`) and **nothing of the upper strip's last row**; ours drew that row (grey 123), a one-pixel line across the whole ticker bar
(316 of the pair's 2022 differing pixels). So the viewport is a scissor on what it holds, here its bottom edge.
`oag_ui::screen::Screens::clip_images_to_viewport` cuts an image's height (and its sampled rows) to the viewport's `y + height` when the
viewport authors `x`, `y`, `width` and `height`. Census (a temporary print in the clip, then `oag-game IMAGE --menu-page main|track-select|ship-select --screenshot` headless; on Pulse PSP it prints exactly the two strips,
so the route reads the whole tree): **Pulse PSP** (USA) the two strips; **Pulse PS2** (EU) the same two strips, viewports `y=389` and `y=410` 21 rows tall over 23-row images,
cut to 21 (the three PS2 menu pairs, ours re-rendered with and without the clip against the committed `theirs.png`: pixels over 40 grey fall 38374 to 36284 (Grid), 59487 to 57546
(Cell), 79328 to 77230 (Team), the bottom 15 % 10820 to 8730, 16001 to 14060, 8937 to 6839; so the law holds on PS2, confidence 80); **Pure PSP** (USA and EU), **HD/Fury**
(`hdfury-ps3-eu.iso`), **Omega** (`omega-ps4-eu.pkg`) and **2048** (`data/extracted/vita/PCSF00007`, `main`; it authors no Track selection screen): checked, clips nothing.
Test: `a_viewport_cuts_the_bottom_off_an_image_that_runs_past_it`,
`menu_layout_ground_truth::pulses_footer_strips_are_cut_to_their_viewports`. Not wired: the top, left and right edges (no image
overruns them in Pulse), and the strips' slide-in `Key` animation, which moves the image inside the viewport and is cut by it live.

## Pure's text pens step up to whole pixels (2026-11-23, `pure-sr-track`)

**Law (confidence 85).** On Pure's front end a glyph's ink edge is the pen plus the glyph's left bearing at the
widget's scale, **rounded up to a whole pixel**; the next pen is that edge plus the advance less the bearing
(`oag_ui::font::pen_step`, the arithmetic of `Font_MeasureString`'s snapped pass on HD, which on HD only a layout
uses and the draw does not). The first pen is the widget's authored `x`. Nothing is read from the text's length:
a label is as wide as its glyphs' snapped steps make it, so `RACE RECORD` at 0.8 spans 79 px where the fractional
pen (advance 9 texels = 7.2 px) spans 73.

Evidence: PPSSPP software renderer, USA, `Track Selection` (`menu-sr-track`, a fresh grab), glyph ink-left columns
read off `theirs.png` (rows of the label, first column of each run of inked columns) against the law with **no free
offset** where the widget's `x` is authored, otherwise one integer offset:

| string | face, scale | glyphs | residual, snapped pen | residual, fractional pen |
| --- | --- | ---: | --- | --- |
| `MODESTO HEIGHTS`, `CHENGHOU PROJECT`, `BLUE RIDGE` (x 21) | `FX300ANG`, 1.15 (`MenuScale`) | 14, 15, 9 | 0.00 px, 0.00 px, 0.00 px (absolute) | 1.84, 1.42, 0.81 px mean |
| `TRACK SELECTION` (x 21) | `FX300ANG`, 0.97 (`TitleScale`) | 14 | 0.00 px (absolute) | 2.42 px mean |
| `RACE RECORD`, `LAP RECORD`, `LENGTH`, `HEIGHT` | `LTe50325`, 0.8 | 10, 9, 6, 6 | 0.00 px each (offset 249 or 357) | 2.9 to 0.4 px mean at 0.7 to 0.85, no scale fits |
| description line, 46 glyphs and the cursor | `LTe50325`, 0.8 (not authored, see below) | 47 | 0.04 px mean, 2 px max at one glyph (offset 21) | 0.29 px mean at 0.865, a scale nothing authors |
| `SPEED` on `Team Selection` (`menu-team`) | `LTe50325`, 0.8 | 5 | 0.00 px (offset 249) | not separable on five glyphs |

A pen of 7.2 or 13.8 px stepping a steady 8 or 14 px is what the picture shows: `CHENGHOU PROJECT` stands at 21, 35,
49, 63 ... (steady 14, advance 12 texels at 1.15 is 13.8), `TRACK SELECTION` at 21, 33, 45 ... (steady 12, 11.64), the
label at 250, 258, 266, 274 and, across its space, 289. **The description line's scale is 0.8, not 0.7 (confidence 75).** The footer's `TextInfo` names `FEGlobals->TextInfoScale`
(and `TextInfoGap`, `TextInfoNumLines`), none of which `FEGlobals` declares, so the value is in the executable's code.
The snapped pen fits the line's 47 glyphs from 0.78 to 0.80 and from no other scale (0.7 leaves 13 px of residual, 0.85
2.6 px); 0.8 is also `MSWarningScale`'s value and the stat labels'. At 0.8 the line's ink is 8 rows tall in both
frames. The earlier 0.7 was chosen off the capital height of one pair. With the right scale and the right face for
the wrap (the `Stats` face's snapped widths, `boot::fonts::description_ink`; the wrap had measured the menu face) the line stays on one
line as the original's does.

The same glyphs' ink widths are unchanged, so only where
each glyph sits moved: ours drew the list 2 % and the title 4 % narrower than the original, the stat labels 9 %, and the
description line 20 %, all of which is this and (for the description line) its scale.

Wired for Pure alone as `oag_title::MenuSkin::snapped_pens` (`Renderer::set_snapped_pens`, set where the front end
installs its faces); the race HUD keeps its measured `round` advances (`docs/ui/hud.md`, "Pulse's HUD text steps the
pen a whole pixel per glyph"). Pure's seven Single Race and Tournament pairs, re-rendered on copies against the same
`theirs.png` (share of pixels differing by more than 40 grey, whole frame, before to after):
`menu-team` 1.86 % to 1.00 %, `menu-sr-track` 2.73 % to 1.57 %, `menu-mode` 1.51 % to 0.70 %, `menu-sr-class` 0.88 % to 0.50 %,
`menu-sr-league` 0.99 % to 0.62 %, `menu-t-class` 0.86 % to 0.49 %, `menu-t-sel` 1.21 % to 0.65 %.

Checked against the other titles, as the rule asks: **HD**: checked, differs (HD's measure snaps and its draw does not,
`oag_ui::font::ink_width`; no HD pair has been measured for a snapped draw). **Pulse**: not measured; its front end
keeps the fractional pen, and a Pulse pair's label spacing is the check. **2048 and Omega**: not checkable here
(no Pure-shaped front end).


## The footer's button glyphs are Pulse_14's (2026-11-23, `pulse-fe-leftovers`)

**Law (confidence 90).** `ControlTextConfirmButton`/`BackButton` (and Cell Selection's `HELP` and `CHANGE DIFFICULTY` glyphs) author `font="small"`,
which Pulse's language plugins resolve to `Pulse_14.fnt`, the 17 px face that is also the `Title` role. Its `ε γ δ β` cells are 17 x 17 (advance 16,
bearing -1). This build drew them from the `menu` face (`Pulse_20.fnt`, 22 px cells) at `faces.small` = 17/22, a minified resample: the ring was a
half texel soft and half a pixel off the original's crisp, pixel-aligned one (theirs: square edges 238 and 254 on one pixel each; ours 168 and 98 split
over two). With the glyph drawn from `Pulse_14` at scale 1 the four rings agree with `theirs.png` to a grey level (`Help` ring 0 px over 40 grey in the
region, 97 before; difficulty ring 0 of 129; maximum difference 1 to 3 grey; Confirm and Back keep a separate difference, below).

**Wiring.** `oag_title::MenuSkin::small_font` (`Some("Title")` on Pulse PSP only: the renderer's third glyph slot already holds `Pulse_14` under `Title`)
reaches `oag_ui_screens::picker::FaceScales::small_role`; with it a `font="small"` text is a `Draw::FacedText` in that role at scale 1 (the footer legend,
the campaign still, the picker's texts) and `faces.small` is not applied. Test: `pulse_menu_ticker_ground_truth::pulses_footer_button_glyphs_draw_in_the_small_face_at_its_own_size`
(fails with the skin field removed), unit test `a_small_face_in_a_glyph_slot_draws_the_button_glyphs_at_its_own_size`.

Share of pixels over 40 grey on the Cell pair re-rendered with main's binary and with this change alone against the same `theirs.png`: 0.40 % to 0.10 % (522 to 127 px).
The picker stills (Track, Team) draw their footer through `menu_page::picker_footer`, a second route that built its own `FaceScales`; it takes the skin's `small_font`
too (and so does the reset prompt, `prompts::footer_faces`), pinned for both stills by `the_stills_footer_glyphs_are_the_small_faces_own_cells` (the brightest pixel of the ring is 246 and 239 from the minified cell, 254 from the 17 px one).
A prompt-style substitute (`--prompt-style xbox`) still draws on both stills: the `Title` face carries the substitute art like the others.

**Cross-title.** HD, Omega and 2048 author `font="buttons"` and have their own `PS_BUTTONS.fnt` slot (not changed: their grid, cell and Track Select pairs re-render
byte-identical). Pure's prompts draw in its `Stats` face (unchanged, byte-identical). **Pulse PS2: checked, differs in wiring** - its skin names no
`small_font` (`PS2_MENU_SKIN.small_font` is `None`; the third slot carries the heading since 2026-11-27), so the glyphs keep the menu face; the PS2's `Pulse_14.fnt` carries the six glyphs at its own size
(`prompt_glyphs_ground_truth`), so this is checked, applies, not wired (a PS2 glyph slot is the open step).

**Open: Confirm and Back dim at the top.** On every Pulse pair the four `NavigationController` texts (the two glyphs and the two words) read dimmer in the
original over their first ten rows: (original minus background) over ours is 0.58 at `y=252` and 1.0 from about `y=262`, the same ramp on the glyphs and
the words (row-mean differences -21, -22, -13, -9 ... on both rings), and not on `Help`, which belongs to a later screen. The ramp ends where the lower footer strip
(`Viewport y=249 height=13`) ends, and `Skin.xml` authors the `NavigationController` before the two strips, so the likely cause is draw order: the original draws the
strip's translucent lower rows **over** the prompts; ours draws the strips in the frame, under every text. Confidence 50 (not tested: ours has no way to put one
frame image above a text). 56 px over 40 grey on each of the Cell and Track Select stills.

## A text's pen starts at its first glyph's ink (2026-11-23, `pulse-fe-leftovers`)

**Law (confidence 80).** On Pulse's front end the first glyph's ink starts at the widget's authored `x`, each later glyph's ink sits its font bearing right
of the pen, and the next pen is the previous ink edge plus the advance less the bearing (`oag_ui::font::pen_step`, started
at `x` minus the first glyph's bearing; each ink edge steps up to a whole unit, "Pulse's text pens step up to whole units" below; the same measure `font::ink_width` states). `Pulse_14.fnt` gives every letter but `T` a left bearing of -1, so
the advance-only pen this build used inked every such letter one pixel right of the original's: Track Select's heading `TRACK SELECT` (x 50, `Title` role) had
ink starts 50, 60, 68, 77, 86, 100, 109, 118, 126, 135, 143 against the original's 50, 59, 67, 76, 85, 99, 108, 117, 125, 134, 143. With the law they agree
(the two `T`s have bearing 0 and never moved). Applying the bearing **without** shifting the first pen moves the first glyph of every string one
pixel left wherever its bearing is -1 (prompt glyphs, words; whole-frame 2.71 % on Cell), so the first-glyph rule is part of the law; it differs from Pure's (`pen_step` from `x`, ink at `x` plus the first bearing, rounded
up).

Wired as `oag_title::MenuSkin::bearing_pens` (`Renderer::set_bearing_pens`, with the faces in `install_faces`): Pulse PSP measured; **Pulse PS2 took it from the PSP, and its headings have since measured it** (below)
(its skin sets the same field; the three PS2 pairs improve 0.02 to 0.05 points, 24876 to 24708, 23351 to 22874 and 60066 to 59656 px over 40 grey, so it is not
a regression there; not measured on the PS2). Share over 40 grey on the four stored Pulse PSP pairs, main's binary against the branch (the glyph face, this pen law and the footer routes together), same
`theirs.png`: Cell 0.40 % to 0.04 %, Track Select 0.53 % to 0.13 %, Grid 1.41 % to 1.13 %, Team 1.38 % to 0.96 %. Test: `render::tests::a_bearing_pen_starts_the_first_ink_at_x_and_steps_by_bearing`
(fails with the pen law off), `title_behaviour_data::only_pulse_lays_its_front_end_pens_out_by_bearing`.

Every measure of a drawn text takes the same pen (`oag_ui::font::Pens`: advance, bearing, snapped): the renderer's alignment and wrap, the hint scan and the wrapped legend's tap rectangles (`hints::placed`, `segments`, `piece_ink`), the footer prompts' widths (the `small` ones in `Pulse_14`'s atlas) and the reset prompt's tap rect, the description wrap, the ticker, marquee and wrapped-name measures and the pointer regions. Help paragraphs drawn in the glyph-slot face are measured in it (`cell_help_draw`'s `HelpFace`), or they stack on one another. Tests: `hints::tests::a_bearing_legend_rect_covers_its_drawn_ink`, `boot::tests::a_small_prompt_is_measured_in_the_small_face_on_the_drawn_pen`, `paragraphs_in_a_glyph_slot_face_flow_too`.

**Cross-title: HD, Omega, 2048 checked, not measured.** `Font_MeasureString` is the same bearing arithmetic on HD (`font::ink_width`); whether HD's *draw* uses it has no
capture here, so those three keep the advance-only pen (all their menu pairs re-render byte-identical with the field off). Pure: its own measured law above.

## Pulse's text pens step up to whole units (2026-11-27, `pulse-menu-headings`)

**Law (confidence 90).** Each glyph's ink edge, its bearing past the pen, steps up to a whole unit of the front end's grid, and the
next pen is that edge plus the advance less the bearing: `oag_ui::font::pen_step(cell, scale, pen, true)`, the step Pure's front end
takes. Unlike Pure's, Pulse's quads stay the atlas cell's (Pure's whole-pixel sprite quads make every Pulse PSP pair worse, 0.7 to 4.6
mean grey in the header box). At a whole scale and a whole `x` the step changes nothing, which is why the 1.0 measurements above did
not show it.

**Evidence**, the left edge of every glyph's ink against the law's, read as column runs off stored frames:

| Frame | Text (face, scale) | Law | Frame |
| --- | --- | --- | --- |
| PCSX2 `pulse-ps2/menu-main` | `MAIN MENU` (`Pulse_14`, 0.8, x 66) | 66 76 87 95 110 120 129 139 | the same, plus a constant half unit |
| PCSX2 `menu-team` | `SHIP SELECT` | 66 76 87 95 110 120 129 137 146 156 | the same (two glyphs merge in one run) |
| PCSX2 `menu-grid` | `RACE CAMPAIGN` | 66 75 85 94 108 117 127 137 147 158 166 176 | the same (one pair merges) |
| PPSSPP `pulse-psp/menu-title` | the legal line's first line (`Pulse_14`, 0.7, x 40) | 40 47 54 61 68 78 85 ... 357 363 373 | all of them |

The other readings fail on the legal line's first line, 338 wide in the frame: fractional advances end it at 354 (314 wide), each
advance rounded at 347, each advance rounded up at 389.

The step is counted from the text's own `x` (**chosen, not measured**: every measured widget authors a whole `x`, where this and an
absolute step are the same), so a centred line or a scrolling ticker at a fractional position moves as one piece. `Pens::Bearing` is the one
law for the draw and every measure of it (`run_width`, alignment, wrap, the hint rectangles). Tests:
`font_roles_ground_truth::pulses_pens_put_a_fractional_text_where_the_original_does` (the disc faces against the edges above),
`render::tests::a_bearing_pen_starts_the_first_ink_at_x_and_steps_by_bearing`,
`hints::tests::a_bearing_legend_rect_covers_its_drawn_ink` and `a_bearing_hint_ends_on_the_stepped_pen`.

**Pulse PS2 is measured, no longer inherited.** **Cross-title: HD, Omega and 2048 are not affected** (they keep the advance-only pen,
`bearing_pens` off).

## Pure's glyph quads are whole pixels (2026-11-23, `pure-leftovers`)

**Law (confidence 85 on the Stats face at 0.8, 75 on the menu face at 1.15 and 0.97).** Each glyph is a PSP sprite: its quad
starts at the **pen rounded up** (`ceil(pen)`, where the pen is the one `pen_step` carries, not the ink edge), is the glyph box's
width and height at the widget's scale **rounded down** (`floor(width x scale)` by `floor(height x scale)`), and its texture
coordinates start **half a texel early** on both axes and span `(texels + 1) * whole / fractional` of them (`oag_ui::font::snapped_quad`, `SNAPPED_UV_SHIFT`; the span and the y shift came in with `pure-title-press-start`, below, and this paragraph's first measurement had x only). The line's ink
edge, which every layout reads (`snapped_run`: wraps, alignment, the pointer rects), is the last quad's right edge.

Evidence: a fresh PPSSPP software grab of Track Selection (`menu-sr-track`, USA). Free fits of a quad (x edges, y edges, bilinear
sampling at pixel centres, the Stats atlas dumped from `LTe50325.fnt`) to the ten glyphs of `RACE RECORD`: every glyph fits
`left = ink-edge law - 0.65`, width 4.95 (box 7 texels, so 5.6 nominal), step 8, height 10.5 to 10.85 (14 texels, 11.2
nominal). The same fit on ours' frame returns the nominal 5.6 and 11. On the menu face (`FX300ANG.fnt`, `MODESTO` at 1.15) the
free fit returns widths 1 px under nominal (12.85 to 13.05 against 13.8) and a left edge half a pixel right of the ink edge.
Read as one convention: quad `[ceil(pen), + floor(w x s)]`, `u` from -0.5. Worked by hand for the `R`'s top bar (texel
columns 61, 96, 85, 85, 91, 91 % ink, bilinear at pixel centres, 1.4 texels a pixel), it gives 43, 95, 85, 90 and 64 % across
columns 249 to 253 against the frame's 44, 100, 90, 97 and 58 %, and nothing in 254, where ours drew a fractional quad from the
ink edge (250, 5.6 wide) with nothing in 249 and a faint 255.

Whole-frame share of pixels over 40 grey on `menu-sr-track` (one `theirs.png`, rendered by main's binary and by this branch) by
region, ours as drawn before to after:

| region (x, y, w, h) | before | after |
| --- | ---: | ---: |
| title (0, 25, 300, 10) | 174 px | 9 |
| list rows (0, 51, 250, 47) | 76 px | 0 |
| stat labels and values (240, 172, 240, 38) | 871 px | 108 |
| description (0, 220, 480, 31) | 360 px | 81 |
| footer Confirm and Back (0, 254, 480, 9) | 209 px | 101 |
| **whole frame** | **1.32 %** | **0.26 %** |

The seven Single Player and Tournament pairs, before (main) to after, on one fresh `theirs.png` each: `menu-team` 0.98 % to
0.20 %, `menu-sr-track` 1.32 to 0.26, `menu-mode` 0.70 to 0.19, `menu-sr-class` 0.44 to 0.12, `menu-sr-league` 0.51 to 0.14,
`menu-t-class` 0.44 to 0.14, `menu-t-sel` 0.51 to 0.14.

Half-way variants tried on the same frame, whole `menu-sr-track`: an integer quad from the ink edge less one (`ink_left - 1`)
fixed the Stats face (stats region 871 to 0) and broke the menu face (title 174 to 436, list 76 to 382); a texture shift in y as
well made every region worse (0.91 %; the y shift was right and the span was not widened with it, see the correction below); a width of `round((w - 1) x s)` is as good on the Stats face and worse in the list (31
px); a u shift of 0.3 to 0.7 texel is within 0.05 points of 0.5, so the half texel is the principled value, not a fit.
Guards (a review's findings, measured on both Pure faces): no glyph of `LTe50325.fnt` or `FX300ANG.fnt` is under two texels wide, so
the floor never reaches zero on a measured glyph (a box with ink is still held to one pixel); no box starts left of texel 2 and
none has ink in the texel before it, so the half-texel shift reads transparent texels on both faces (the shift is clamped at the
atlas edge for any other face). Every snapped line starts on a whole pixel (`ceil` of its aligned pen), so `snapped_run`, which
counts from 0, measures it wherever it is placed; the pointer rects of a wrapped or scanned line follow one pen across the
line and the glyph quads' own extent.
What is still open: the pen law that ceils the ink edge and the quad that ceils the pen are two readings of one rule that this
data cannot separate (every Stats pen is fractional), and the remaining 108 px on the stat labels and 101 on the footer are the
record separators' dots (`0 . 00 . 00`) and the `Confirm` glyph rings.

The second row of the description is **10 px** below the first (ink rows 222, 232, 242), not the viewport's 32 px shared among its
three lines (222, 233, 243): `oag_ui_screens::campaign::footer::DESCRIPTION_LINE_GAP`, confidence 75, on a late grab of
`menu-team` and `menu-t-sel` that shows all three lines (`TextInfoGap` is named by `Skin.xml` and undeclared in `FEGlobals`).
`menu-team` 1.75 % to 1.54 % on that frame for the gap alone.

Checked against the other titles: **Pulse and HD**: not applicable, the snapped quad is `MenuSkin::snapped_pens` (Pure only); a
Pulse and an HD menu page rendered by main's binary and this one are byte for byte equal. **2048 and Omega**: not checkable
(no Pure-shaped front end).

**Correction (`pure-title-press-start`): the texture span.** The executable's sprite samples `[u0 - 0.5, u1 + 0.5]` by `[v0 - 0.5, v1 + 0.5]`, `w + 1` by `h + 1` texels, over the fractional `w*s` by `h*s` it is built with and keeps whole pixels of
(`docs/ghidra/functions/psp-pure-usa/menu-text-centring.md`, "The quad", confidence 80). `snapped_quad` now carries that span (`u_span`, `v_span`) and the start is half a texel early in y too. The title pair's `PRESS START BUTTON` (the scale-1.5 menu face, the share the
first measurement above left at 0.32 %) goes 0.32 % to 0.01 % whole frame (its text box 5.91 % to 0.00 %); all ten Pure menu pairs improve, `menu-sr-track` 0.25 % to 0.18 % and mean grey 0.73 to 0.33, `menu-pause` 2.23 % to 1.80 %. Pure circuit pairs (`modesto`, `karbonis`), a Pulse PSP menu pair and an HD menu pair re-render byte for byte equal.
Checked against the other titles: Pulse and HD do not use the snapped quad (equal); 2048 and Omega: not applicable.

## Pure's right-aligned text ends on its ink (2026-11-23, `pure-letter-spacing`)

**Law (confidence 70, one widget in one frame).** A right-aligned string in a snapped face puts its **last glyph's quad edge on
the anchor** (`oag_ui::font::snapped_run`'s `ink_right`), not its pen end: the pen starts `ink_right` to the left of `x`, rounded up
(`Renderer::push_text`). Evidence: the title screen's `HOLD ON!` (`Skin.xml`: `font="Stats"`, `scale="0.7"`, `align="right"`,
`x="416"`, `DisplayType="TELETYPE"`; PPSSPP software, USA, the pair `menu-title`). The original's seven glyphs start at columns 366, 373,
380, 388, 400, 407, 415; aligned by pen end ours started at 362, 369, 376, 383, 397, 403, 410 (the same pitch, 4 px left: the last glyph's
trailing advance, 4.2 px for the `!`); on the ink edge ours starts 366, 373, 380, 387, 401, 407, 414, each within 1 px. Only this
widget measures it: no other captured Pure frame has a right-aligned snapped string (`menu-main`, `-team`, `-mode`, `-sr-*`, `-t-*`
and `-pause` re-rendered before and after are pixel for pixel equal). A centred string starts at `ceil(x - W / 2)`, `W` the original's measure (`oag_ui::font::snapped_centre_pen`, below, "Pure's centred text starts at half its measure"). The pointer rectangles over a drawn line (`hints::scan`) follow
the same rule, so a tap lands on the ink; our own `menu.toml` pages' right-hand value columns on Pure now end their ink on
`value_right` too (no Pure pair draws one).

**`HOLD ON!` is the `Stats` face.** Ours had drawn it in the menu face (`FX300ANG.fnt`, wide, bold): `Frontend::draw_screen_over`
emitted every screen text as a plain `Draw::Text`. A `font="Stats"` or `"Scroll"` text now draws as `Draw::FacedText { role: "Stats" }`
at its authored scale when the title's menu skin names a Stats face (`MenuSkin::stats_font`, Pure only), which is how the team panel,
the circuit panel and the description already reached `LTe50325.fnt`. No ratio against the `Default` atlas applies (that ratio
corrects a text drawn in the wrong atlas). Pulse, HD, 2048 and Omega name no such face and draw as before.

**Why the title pair rose from 0.51 to 0.58 mean grey when the glyphs became whole-pixel sprites (`pure-leftovers`).** Measured with
the binary of that merge's first parent against main's, both re-rendered against the same `theirs.png`: 0.23 % to 0.42 % of pixels over
40 grey. By band (`x 240-480, y 172-209` / `y 220-250` / the rest): 46, 126, 114 px to 152, 133, 257. The rise is the `PRESS START
BUTTON` row (`font="Title"`, scale 1.5, the menu face, y 202-216, across the first and third band), **not** `HOLD ON!` (the typed band,
wrong in both binaries: 126 and 133 px). On `PRESS START BUTTON` the merge moved the glyph starts onto the original's (ink-left 101,
119, 137, 155, 173, 200 ... in the original and on main, one column left in the parent) and the edge sampling off it: the pixels over 40
grey in the row's box went 168 to 416, spread evenly over the letters' rows. **Open, not fixed**: the half-texel `u` shift
(`SNAPPED_UV_SHIFT`) is 0.75 px at scale 1.5 against 0.4 at 0.8 and 0.58 at 1.15; a shift of 0.3 to 0.4 px at every scale takes the
title pair from 0.32 % to 0.10 to 0.13 % over 40 grey and leaves `menu-sr-track` and `menu-team` at 0.24 to 0.25 and 0.18, but
`menu-pause` goes 2.51 % to 2.60 % and every pair's mean grey rises 0.01 to 0.03 (0.73 to 0.75 on `menu-main`), so one fixed pixel
shift is not the law either. Shortening the quad's height by 1 px or 0.5 px, or using `floor((h - 1) x s)`, at scales over 1.3 made
the pair worse (0.97 %, 0.45 %, 0.99 %). The law behind the original's 1.5 quads needs a second measurement at a scale over 1.3.

**`HOLD ON!` is fixed on its own account** (above): the typed band of the title pair 133 px to 8, the whole pair 0.42 % to 0.32 % over
40 grey, mean grey 0.58 to 0.53 (the first parent's 0.51).

**The typed description's spacing needs no further law.** Re-measured on main, glyph ink-left columns of the typed lines against the
originals' (`theirs.png`), line by line: `menu-main` 0.19 px and 0.24 px mean absolute residual (8 of 43 and 10 of 45 glyphs one pixel
off), `menu-sr-track` line 1 0.26 px (11 of 47). The one-pixel misses on `menu-main` line 1 are the first glyph of every word after the
first (ours one pixel right, the rest of the word equal); neither exact decimal against `f32` arithmetic, an unsnapped space, nor a
space of another advance (3 to 10 texels, 28 mismatches in every variant over three lines) removes them, so they are sub-pixel quad
detail of the original, unexplained. The typed band's remaining pixels over 40 grey: `menu-main` 150, `menu-mode` 142, `menu-sr-track` 75,
`menu-t-sel` 46 (of 480x32). `menu-mode`'s are the reveal count, not the spacing: the original has typed three characters more (`CHOI_`
against ours `C_`), 50 ms at 58 a second, inside the pair clock's stated two-character error ("The pair clock").

## Pure's centred text starts at half its measure (2026-11-24, `pure-centre-rounding`)

**Law (confidence 80, read off `Text_BuildGlyphQuads` `0x08947ae0` and `Text_MeasureString` `0x08947868`, fits all seven measured `<Menu>` rows).** A centred string's pen starts at `x - W / 2 - b0 * s` and is not rounded; the first glyph's own step (`ceil(pen + b0 * s)`) rounds it,
so the first quad is `ceil(x - W / 2)`. `W` is the executable's measure: the first glyph's ink to the last glyph's ink, each pen stepped up to a whole unit on the way (`oag_ui::font::snapped_centre_pen`), not the advance sum nor `snapped_run().end`. An odd `W` at scale 1 is a `.5` tie and rounds up. Evidence and
the seven-row table: `docs/ghidra/functions/psp-pure-usa/menu-text-centring.md`; the before and after of the pause pair: `docs/tools/compare-page.md`, "The rows' rounding, recovered from the executable". Wired for Pure only (`MenuSkin::snapped_pens`): `menu-pause`'s band-word box 610 to 240 px over 40 grey, every other Pure pair, Pulse and HD pairs pixel for pixel
equal. Test: `font::centre_tests` (a tie at scale 1, a 1.2 string the old rule starts a unit left, a leading bearing) and `render::tests::a_snapped_pen_draws_each_glyph_at_a_whole_unit` (they fail with the law off), `pure_centre_ground_truth` (the seven rows, on the disc).

## Pulse PS2's headings draw in the Title face (2026-11-27, `menu-pairs`)

**Authored.** The PS2 pressing's own XML names `font="Title"` on the heading widgets: `FE_MM`
(`MainMenu_Definition.xml`), `FE_RACEBOX` (`RaceBox_Definition.xml`), `RC_SHIPSEL`
(`Selection_Definition.xml`) and `FE_OPT_PLUS` (`Additional_Definition.xml`), all in `54748/WADS2.WAD`.
`PS2_MENU_SKIN` now names `title_font: Some("Title")` and `body_font: Some("Default")`, as the PSP skin
does. Pinned by `font_roles_ground_truth::the_ps2_headings_author_the_title_role_and_its_skin_draws_them_in_it`.

**Checked against the frames** (the stored PCSX2 GS grabs at 1280x720, not a fresh 1:1 capture; confidence 85):

- `SHIP SELECT` was 231 px wide in ours against 195 in the original; in the Title face at the skin's 0.8 it
  is 186 to 187.
- With `title_scale` at 1.0, ours' `OPTIONS` glyph left edges are 134, 158, 182, 250, 274 against the
  original's 133, 157, 182, 248, 272 (a threshold-90 column-run read, luma, the 128..388 x 22..44 box): the
  same face to 2 px.

**`Additional`'s heading is larger, and each heading's scale is read.** `FE_OPT_PLUS` and `FE_EXTRAS` author no
`scale` (1.0), where `FE_MM`, `FE_RACEBOX`, `RC_SHIPSEL` and the rest author `scale="FEGlobals->TitleScale"` (0.8; the
PSP's `TitleScale` is 1.0, so the PSP does not move). The boot reads the first title-font `Text` of the screen a menu
page stands where (`oag_title::PageHeading`: `options` is `Additional`, `extras` is `extras`; a compare page is its
`ListScreen`) and `menu::draw_list` draws that page's heading at it (`Frame::heading_scales`); every other heading keeps
`MenuSkin::title_scale`. PS2 `menu-options`' header box falls from 9.50 to 7.57 mean grey. Tests:
`selection_screens_ground_truth::the_ps2_pressing_reads_the_same_screens_on_its_own_grid`,
`menu::tests::drawing::a_pages_heading_draws_at_the_scale_the_disc_authors_for_it`.

Other headings on the disc that author no scale, none of which a page of this build stands where: `Profile`, `Profile Details`,
`Manage Profile`, `Records` (the four-category hub), `TournamentLoad` and the memory-stick screens. This build's `records` page
is a table of class bests, which on the disc is `Race Records` and its three siblings, headed at `TitleScale`.

**Not closed:** the original's glyphs are heavier (an outline about 1 px wide at 2x that ours does not draw).
