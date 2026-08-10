# The original's menus

**Status: main-menu layout measured, transition measured, chrome unbuilt.**
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
| selected row | rgb(157, 255, 255), and rgb(107, 226, 247) on another frame | - |
| help text | white | `0xffffffff` |
| screen title | black, on a light top bar | `FEGlobals->TitleColor` `0xFF000000` |

The unselected measurement is the authored colour times 1.24 on all three
channels equally, which is the lit backdrop showing through rather than a
different colour. **`TextColor` is confirmed.**

**Selection is a brightening toward white.** Not a bar behind the row, and not
the pink `MenuHighLightArrowColor` the palette declares - no pink appears
anywhere on this screen. This build drew an invented translucent bar until this
page existed.

**The highlight moves.** Two frames of the same *still* menu measured different
peaks. Its period and depth were not measured, so nothing implements a pulse;
the brighter of the two values is used flat. This is the same discipline
[HANDOVER](../../HANDOVER.md)'s task 7 states for `BOOT_PRESS_START`'s pulse -
and the same warning applies: **do not measure it off our own build.**

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

## Not built

- **The top bar** (`topbarleft`, `topbarcenter`, `topbarright`, and their
  arrows) and **the footer** (a tag block, a scrolling ticker, a button-prompt
  line). Until the bar exists, the authored black `TitleColor` would be black
  text on a dark backdrop, so this build substitutes its own title colour - the
  same substitution the language picker already makes.
- **The highlight's pulse**, for want of a period and a depth.
- **Pure's menus**, beyond the six globals its `Skin.xml` states. Its
  definitions have not been read for row geometry.
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
