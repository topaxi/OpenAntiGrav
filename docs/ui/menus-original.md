# The original's menus

**Status: main-menu layout measured, transition measured. Both PSP titles'
own frames are built - Pulse's top bar and footer strips, Pure's rule lines
and background - and each has one open piece: Pulse's footer ticker/button
prompts, Pure's background texture itself.**
Implemented in [`oag_title::menu`](../../crates/title/src/menu.rs),
[`oag_game::menu`](../../crates/game/src/menu.rs) and
[`oag_game::anim`](../../crates/game/src/anim.rs).

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
2. **The original enters an attract demo after about 120 seconds idle**, which
   silently invalidates a capture left sitting.
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
past the documented ~120 s attract-demo threshold partway through and
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
`collect_widgets` doc, and `crates/game/tests/pulse_frame_ground_truth.rs`,
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

See `crates/game/tests/pure_frame_ground_truth.rs` for the frame's own
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

- **The footer's ticker text and button-prompt line** - see above.
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
