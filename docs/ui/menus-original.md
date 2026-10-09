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
`crates/game/src/main/session/menus.rs`'s `Renderer::new` call - with every
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
2. ~~**The top-bar title is set in the wrong face on every page.**~~ **Closed 2026-10-08 on both PSP pressings**: `oag_pulse::frontend::MENU_SKIN::title_font` is `Some("Title")` and the renderer's third glyph slot carries `Pulse_14.fnt` (`oag_game::boot::fonts::third_atlas_slot`; `body_font` keeps `Default` in the second). The PS2 port keeps `None`: nothing re-measured its title. *(was:)* The original's
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
   the strip, ink and size matching). The tag is still empty. On the campaign
   pages a `--menu-page` still draws no tip at all (`static_footer_overlay`
   passes an empty rotation), so only the live stage and `--menu-page main`
   show the new face. Also visible there: our `Confirm` label is smaller than
   the original's and sits higher against its button glyph (shrink-to-fit
   scale, not measured against the disc).
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
8. **Notes, not ranked.** The track circuit's distance reads 5178 on the
   original and 5094 on ours for Talon's Junction White (ours is not the
   authored value, worth a trace). A circuit count of 1 / 3 on a fresh
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
