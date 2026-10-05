# The menu `Block`: HD's tab and row boxes, their border, their fill, and how they grow

2026-09-14. What draws the box behind every entry of HD's `<HorizMenu>` strip, `<List>`
settings row and `<aVertMenu>` pause menu is one widget class, `Block_Item.cpp`, and
everything a capture shows about those boxes - the lighter border, the darker inside,
the chamfered corners, the selected one being wider and easing there - is code in this
binary plus one 64x64 texture. None of it is authored in the GUI XML, which is why
[hd-frontend.md](../../../formats/hd-frontend.md)'s strip section could only measure
it. This page is what that section was missing.

Read [memory.md](memory.md) first for the per-function TOC defect. Every function below
is under `0x0026_0000`, and every one was still checked with `scripts/ps3-toc.py toc`
before a TOC-relative constant was trusted: all read `exact`, TOC `0x008ad4d8`, so
Ghidra's own `DAT_`/`PTR_` names are right for these functions.

## How it was found

`search_strings` for the `<aVertMenu>` attributes `FocusWidth`/`FocusColour` lands in a
string pool beside `VertMenu_Item.cpp`; the parser that reads them (`VertMenu_ParseXml`,
below) is the only function naming them, and the class it belongs to creates one child
of type `"Block"` per `<Entry>`. `Block_Item.cpp` is one of the 114 `*_Item.cpp` tags in
this binary, its constructor is found by the `__FILE__`-at-`+0x30` trick
(`scripts/ps3-toc.py attrib 0x00789038`), and its render method follows from the vtable.
The `<HorizMenu>` and `<List>` classes were then read the same way and turned out to
build the same `Block` children.

## Class attribution

| `.cpp` tag | Address | Constructor(s) via `attrib` |
| --- | --- | --- |
| `Block_Item.cpp` | `0x00789038` | `0x0018b818` (`Block_Construct`), `0x0018ba58` |
| `HorizMenu_Item.cpp` | `0x0078b578` | `0x001b47c8` (`HorizMenu_Construct`), `0x001b4a38` |
| `VertMenu_Item.cpp` | `0x00792430` | `0x0020fc78` (`VertMenu_Construct`), `0x0020fd68` |
| `List_Item.cpp` | `0x0078be08` | `0x001bf7f8` (`List_Construct`), `0x001bfad0` |

The second address of each pair is the other half of a base/complete constructor pair,
left unnamed for the reason [mode-manager.md](mode-manager.md) gives.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0018b818` | `Block_Construct` | 85 |
| `0x0018b050` | `Block_Render` | 82 |
| `0x0018acb8` | `Block_DrawFill` | 80 |
| `0x0018a5d0` | `Block_DrawTopBand` | 78 |
| `0x00189f00` | `Block_DrawCorner` | 82 |
| `0x0018a3b8` | `Block_DrawVerticalEdge` | 80 |
| `0x0018a1a0` | `Block_DrawHorizontalEdge` | 80 |
| `0x00189c08` | `Block_SetColour` | 85 |
| `0x0018dda8` | `Block_SetHeight` | 80 |
| `0x0018ce78` | `Block_SetLandingStyle` | 75 |
| `0x001898c8` | `Block_LayoutSlices` | 60 (below 70, carries `_q`) |
| `0x001b47c8` | `HorizMenu_Construct` | 85 |
| `0x001b3430` | `HorizMenu_ParseXml` | 82 |
| `0x001b2b98` | `HorizMenu_AddEntryBlock` | 82 |
| `0x001b4158` | `HorizMenu_LayoutBlocks` | 82 |
| `0x001b52d0` | `HorizMenu_Update` | 75 |
| `0x0020fc78` | `VertMenu_Construct` | 85 |
| `0x002105f0` | `VertMenu_ParseXml` | 85 |
| `0x0020f810` | `VertMenu_AddEntryBlock` | 82 |
| `0x00211930` | `VertMenu_LayoutBlocks` | 82 |
| `0x00211df0` | `VertMenu_Update` | 75 |
| `0x001bf7f8` | `List_Construct` | 85 |
| `0x001bdb48` | `List_CreateWidgets` | 80 |
| `0x001c03e0` | `List_Update` | 78 |
| `0x001be208` | `List_SetValueBlockWidth` | 80 |
| `0x0016aaa8` | `Widget_CreateChildByType` | 72 |
| `0x001b5708` | `Image_SetVertexColours` | 70 |
| `0x0015b620` | `FrontEnd_IsFuryStyle` | 72 |

## `Block_Construct` - `0x0018b818`

Sets the `Block_Item.cpp` tag at `+0x30`, the vtable at `0x008654e8`, and the defaults
that matter below: height `+0xb0 = 40.0`, border enabled `+0x10c = 1`, landing style
`+0x11c = 0`, no animated-fill override `+0x104 = 0`, no extra highlight `+0x14c = 0`,
and a fallback colour trio `+0xbc = 0xffffffff`, `+0xc0 = 0xff8ac0ca`, `+0xc4 =
0xffffffff` - the second literal is `HD_Blue`'s own `DATA06` value, compiled in. Its
texture comes off a two-entry pointer table at `0x00920a00`:

```
0x00920a00 -> 0x00789240 "Data\FE\Images\file2.gtf"
0x00920a04 -> 0x007892f0 "Data\FE\Images\corner.gtf"
```

`file2.gtf` is what every block samples. It ships in `DATA06` only, 64x64 `A8R8G8B8`,
RGB white everywhere; **its alpha channel is the whole picture**. Dumped with
`crates/texture/examples/gtf_to_png.rs`, `#` is alpha 255, `.` is 0, digits are the
handful of partial texels:

```
row  1-3   top line, columns 10..62, plus a 45-degree diagonal from (10,1) to (1,10)
cols 1-3   left line, rows 10..62
cols 60-62 right line, rows 1..49
row 50-52  a lower horizontal line, columns 44..62   (the "landing")
diag       from (44,50) down-left to (36,58)         (the step's own cut)
row 60-62  bottom line, columns 1..36
cols 4-5   rows 54..59: alpha 223, 79, 79, 79, 110, 110   (fill swatches, see below)
```

So it is a **nine-patch frame**: a three-texel outline of a box whose top-left corner is
chamfered and whose bottom-right corner steps in, plus a column of swatch texels the
fill samples. Confidence 90 for the file: the render paths below address it by exact
sixty-fourths and every one lands on one of those features.

## `Block_Render` - `0x0018b050`

Vtable slot 7. The colour is `+0xbc` (with a small enum at `+0x175` that can force it to
`0`, `0xff646464` or `0xffd0d0d0` - unused by the three menu classes), red and blue
swapped once for the RSX. Position is `+0xa4/+0xa8` plus the parent's `+0x4c/+0x50`.
`w = +0xac`, `h = +0xb0`. Then, in order:

```
Block_DrawFill(x, y, w, h, colour, swatch=0)                 // pass 1, always
if (+0x14c > 0)  Block_DrawFill(x, y, +0x14c, h, FEGlobal("HD_Blue"), ..)   // unused here
if (+0x104 == 0) Block_DrawFill(x, y, +0x148, h, colour, swatch = !FrontEnd_IsFuryStyle())  // pass 2
else             Block_DrawFill(x, y, +0x148, h, +0x108, 1)
if (w > 0 && +0x10c)                                          // the border
    Block_DrawCorner(x,        y + h - 18, kind 0)            // bottom-left
    Block_DrawCorner(x + w - 40, y + h - 18, kind 1)          // bottom-right
    Block_DrawCorner(x,        y,          kind Fury ? 2 : 5) // top-left
    Block_DrawCorner(x + w - 40, y,        kind +0x11c ? 4 : 3) // top-right
    Block_DrawVerticalEdge(x + w - 40, y + 18, h - 36, kind 1)
    Block_DrawVerticalEdge(x,          y + 18, h - 36, kind 0)
    Block_DrawHorizontalEdge(x + 40, y,          w - 80, kind 0)
    Block_DrawHorizontalEdge(x + 40, y + h - 18, w - 80, kind 1)
```

The literals are TOC constants: `18.0` at `0x008ac578`, `40.0` at `0x008ac55c`, `36.0`
at `0x008ac5a0`, `80.0` at `0x008ac5a4`. `+0x148` is the block's *animated* width, which
all three menu classes keep equal to `+0xac`, so passes 1 and 2 cover the same rectangle.

### The fill, and where the transparency comes from

`Block_DrawFill` (`0x0018acb8`) insets by 2 (`0x008ac58c`) on the left and top and by
4 (`0x008ac588`) in width, draws the body from `y+12` (`10.0` at `0x008ac590` below the
inset) to `y+h-2` as one quad, then calls `Block_DrawTopBand` for the ten units above it.
Every vertex of both carries the block colour at **full** alpha and a UV of
`(0.0703, 0.914)` for `swatch=0` or `(0.03125, 0.914)` for `swatch=1` - `lis r6,0x3d90` /
`lis r6,0x3d00` and `lis r10,0x3f6a` in the listing. In texels that is `(4.5, 58.5)` and
`(2.0, 58.5)`: the first sits between the two alpha-`110` swatches at rows 58 and 59, the
second on the outline at alpha `255`. **So pass 1 is always the colour at 110/255 =
0.431, and pass 2 is the colour at 0.431 again on the Fury style or opaque on the HD
style.** Two layers of 0.431 composite to `1 - 0.569^2 = 0.676`; one of 0.431 under an
opaque layer is 1.0.

That is exactly what the captures measure, and it is the reason the two styles looked
like different rules. On the Fury style (`hd-menu-style-toggle/03-main-menu-after.png`,
black page) an unselected tab's inside is `(102,102,102)` under a `(150,150,150)`
border: `102/150 = 0.68`. The selected tab is `(116,5,16)` inside `(172,7,23)`:
`0.674 / 0.71 / 0.70`. On the HD style (`hd-menu-tab-corner/mainmenu-3840x2160.png`,
white page) the inside is `(100,100,100)` - `HD_Grey` `0x646464` to the byte - with no
border visible, because the border is the same colour at full alpha and so is the fill.
Nothing in `skin.xml` states an alpha for either; it is the texture's swatch and the
style flag. Confidence 88 (two independent captures, three channels each, both styles).

### The shape: chamfers are geometry, the border is the texture

`Block_DrawTopBand` (`0x0018a5d0`, called with `x+1`, `y+2`, band height 10) draws the
top ten units as a middle quad and two end pieces. The **right** end is always a
triangle: a 45-degree cut of the band's own height, 10 across and 10 down. The **left**
end is a triangle only when `FrontEnd_IsFuryStyle()` is set, and a square quad when it
is not (`0x0018aad0`). The band's width is the fill width plus 1 for a plain block, or
**minus 17** (`0x008ac594`) for a block with `+0x11c` set - which is what leaves a flat
landing between the diagonal's foot and the right edge. Read together with the corner
pieces:

| Block corner | Fury style | HD style |
| --- | --- | --- |
| top-left | 45-degree chamfer, 10 units | square |
| top-right, `+0x11c = 0` | 45-degree chamfer, 10 units | same |
| top-right, `+0x11c = 1` | diagonal 10 across, then a flat 17-unit landing to the right edge | same |
| bottom, both | square | square |

`Block_DrawCorner` (`0x00189f00`) draws a 40x18 quad whose UV rectangle is picked by a
six-way jump table at `0x00189f48`; `Block_DrawVerticalEdge` (`0x0018a3b8`) and
`Block_DrawHorizontalEdge` (`0x0018a1a0`) draw a 40-unit-wide (18-unit-tall) strip
sampling one texel column (row) of the outline. All in sixty-fourths of `file2.gtf`,
`u` left to right, `v` top to bottom; a reversed pair mirrors the piece:

| Kind | Piece | `u` | `v` | What it shows |
| --- | --- | --- | --- | --- |
| 0 | bottom-left | `1.0 -> 0.375` | `0.281 -> 0.0` | top-right square corner, flipped both ways |
| 1 | bottom-right | `0.375 -> 1.0` | `0.281 -> 0.0` | the same, flipped vertically |
| 2 | top-left (Fury) | `0.0 -> 0.625` | `0.0 -> 0.281` | the chamfered top-left corner |
| 3 | top-right (plain) | `0.625 -> 0.0` | `0.0 -> 0.281` | the same, mirrored |
| 4 | top-right (landing) | `0.375 -> 1.0` | `1.0 -> 0.719` | the stepped bottom-right corner, flipped vertically |
| 5 | top-left (HD) | `1.0 -> 0.375` | `0.0 -> 0.281` | the square corner, mirrored - case 0 without the vertical flip |
| vertical edge | left / right | `0.0 -> 0.625` / reversed | `0.469 -> 0.484` | one row of the left line |
| horizontal edge | top / bottom | `0.219 -> 0.234` | `0.0 -> 0.281` / reversed | one column of the top line |

`0.375 = 24/64`, `0.625 = 40/64`, `0.281 = 18/64`, `0.719 = 46/64`: every piece is a
40x18 texel region drawn at 40x18 units, so the outline is three units thick, one unit in
from the block's rectangle. The visible tab is therefore 62 tall inside a 64-unit block
and its left edge sits at `x+1`, which is what hd-frontend.md measured (`126.0` for an
authored `y=125`, `160.5` for `x=160`, height `62.0`) before any of this was read.

The 45-degree diagonal of the *fill* is rasterised geometry (a triangle); the diagonal
of the *border* is texels. hd-frontend.md's "rasterised geometry, not a texture mask"
was right about the fill and was looking at the wrong texture (`corner2.gtf`, a
`<Bracket>` asset) for the border.

### `Block_LayoutSlices` - `0x001898c8`

Called by every `AddEntryBlock` after the width is set. Splits `+0xac` into three
horizontal slices (`+0x12c`, `+0x130`, `+0x134`, with `u` offsets in `+0x118`,
`+0x120`, `+0x128` and `x` positions in `+0x138..+0x140`) - an 8-unit left cap, a
stretched middle and a 35- or 8-unit right cap. **Nothing in `Block_Render` reads
those fields**; the render above builds its own geometry from `+0xac` directly. Named at
60 because what does consume the slices was not found - possibly a child widget at
`+0x164`/`+0x168`, which the same function repositions.

## `HorizMenu` - the main menu's tabs

### `HorizMenu_Construct` - `0x001b47c8`

Defaults: scale `+0xd8 = 1.0`, scroll speed `+0xc0 = 0.1`, `+0xbc` (gap) `0`, and
**`+0xe0 = 0x43950000 = 298.0`**. Three colour names are hashed for later lookup, all
through the same `FEGlobals` resolver `List_Construct` uses: `FE_HD_Grey` -> `+0x1c8`,
`FE_HD_Blue` -> `+0x1cc`, `FE_HD_White` -> `+0x1d0` (TOC slots `0xec`/`0xf0`/`0xf4`).
Two textures are loaded, `Data\FE\Images\file.mip` and `Data\FE\Images\cursor.mip`
(slots `0xe4`/`0xe8`; the `.mip` suffix is the PSP-era spelling the loader maps to
`.gtf`).

### `HorizMenu_ParseXml` - `0x001b3430`

The attribute chain, resolved through the function's own TOC: `X` -> `+0xac`, `Y` ->
`+0xb0`, `Gap` -> `+0xbc`, `ScrollSpeed` -> `+0xc0`, `DisableEntriesBitField` ->
`+0xf8`, `GSDisableEntriesBitField` -> `+0xfc`, `DefaultItem` -> `+0xdc`, **`ItemWidth`
-> `+0xe0`**, `Color` -> `+0xc8`, `Scale` -> `+0xd8`, `Font` -> `+0xcc`, then `Wrap`,
`Align`, `VertAlign`, `Allocate`, `MenuDisplayType` into locals. Each `<Entry>` ends in
`HorizMenu_AddEntryBlock`.

So the 298 the constructor writes is `ItemWidth`'s default, and the main menu - which
authors no `ItemWidth` - gets it. hd-frontend.md measured `296.88` per unselected tab and
called it "an engine default the disc declines to state"; it is this constant, less the
one-unit border inset on each side.

### `HorizMenu_AddEntryBlock` - `0x001b2b98`

Per entry `i`: the target width array at `+0x15c + 4i` gets `ItemWidth + 70.0` for the
first entry and `ItemWidth` for every other (the first is selected at construction);
a `Block` child is created at `x = X + 100 i - 2`, `y = Y` (placeholder positions,
relaid every frame), width 90, height `64.0` (`0x008ad4f0`), colour `+0x1c8`
(`HD_Grey`), landing style `1`.

### `HorizMenu_LayoutBlocks` - `0x001b4158`

Runs every frame from `HorizMenu_Update`. Walking the entries left to right from
`x = X`:

- **Width eases toward its target at one sixth of the remaining distance per frame**:
  `w += (target - w) * 0.16667` (`0x008ad594`). The selected entry's target is
  `ItemWidth + 70.0` (`0x008ad5a0`), every other entry's is `ItemWidth`. With `reset`
  set (the call from `OnEnable`) the widths snap instead.
- **Pitch is `w + 10`** (`0x008ad59c`): the next block starts at `x + w + 10`, and the
  entry's text at `x + 10`.
- **Colour**: the selected block gets `+0x1cc` (`HD_Blue`) while the menu has focus
  (`+0x34 & 0x200`) and `+0x1c8` (`HD_Grey`) otherwise; an unselected block gets
  `HD_Grey` with focus and the literal `0xffd0d0d0` without it. An entry masked in
  `+0xf8` draws its text `0xffa0a0a0`; every other entry's text is `+0x1d0`
  (`HD_White`).
- **The underline** is an `Image` child (`+0x108`, created on the first layout) of
  `cursor.gtf`, 32x16 at `(x + slide + 10, Y + 35)`, where `slide` starts at
  `ItemWidth` on a reset and is multiplied by `0.8333` (`0x008ad598`) each frame - it
  slides in from the right and settles. Its colour is `+0x1d0` (white) for a 17-frame
  counter's values `0..7` and `0` (invisible) for `8..16`: **an 8-on/9-off blink at
  60 Hz**, in both the focused and unfocused branches.

`cursor.gtf` (`DATA02`, 32x16 DXT) is a white 21x8 bar at texel `(1,1)`, matching the
`20.8 x 7.82` mark hd-frontend.md measured. Its measured top is `Y + 42.6`, against
`Y + 35 + 1` from this reading; the 6.6-unit residual is unexplained - either an
`Image`-side offset this page did not read or a vertically flipped decode of this one
DXT file - and the build keeps the measured number.

**The blink is the one claim here a capture argues with.** Five captures of the main
menu, two boots apart, all show the underline, which at an 8/17 duty cycle is a 2 %
event. The code is unambiguous and the counter is not gated on anything a still would
freeze, so the reading stands at confidence 70 with that tension recorded; a short
recording (`just rpcs3-record`) would settle it in a minute. The *tab* does not pulse -
nothing above touches its colour or width on a timer - which is the half hd-frontend.md
had already measured.

**The 1600 check.** `368 + 4 * 298 + 4 * 10 = 1600`: the five tabs and four gaps of the
main menu span exactly the frame's own `line.gtf` rules (`x=160`, `w=1600`). hd-frontend.md
measured `366.67 + 4 * 296.88 + 4 * 11.3 = 1599.8` off the capture - the same span,
with the border inset moving ~1.1 units from each width into each gap.

## `VertMenu` - the pause menu

`VertMenu_ParseXml` (`0x002105f0`) reads the same base attributes as `HorizMenu` plus
`Colour` -> `+0x1a8`, `FocusColour` -> `+0x1a4`, `Height` -> `+0x1a0`, `ShowSelection`
-> `+0x1bc`, `FocusWidth` -> `+0x1b4`, `Width` -> `+0x1b8` and `handleInput` -> `+0x1c0`,
clamping both widths to at least `80.0` (`0x008afc78`). `VertMenu_Construct`
(`0x0020fc78`) defaults `Height` 64, `Width` 280, `FocusWidth` 400.
`VertMenu_AddEntryBlock` (`0x0020f810`) creates one `Block` per entry at
`y = Y + i * (Height + 10)` with landing style `1`. `VertMenu_LayoutBlocks`
(`0x00211930`) is `HorizMenu_LayoutBlocks` turned on its side: width eases at the same
`0.16667` toward `FocusWidth` or `Width`, the block colour is `Colour` (or `HD_Grey` when
the widget authored none) and `FocusColour` (or `HD_Blue`) for the selected entry while
focused, a disabled entry is `HD_LightGrey`, and the text sits at `X + 8` (`X + 48` when a
cursor child exists) and `y + 3` on a menu shorter than 64.

This is the one widget whose growth **is** authored: `ingame_definition.xml`'s pause
menus write `Width="520" FocusWidth="580"` on every `<aVertMenu>`. The strip and the
settings rows author neither, and get the defaults above.

## `List` - the settings rows

`List_Construct` (`0x001bf7f8`) defaults: label block width `+0x12c = 520.0`, value
block width `+0x13c = 280.0` and its range `+0x13c .. +0x140` = `280.0 .. 340.0`, arrow
images 32x32 (`+0xe8/+0xec/+0xf0/+0xf4`) at label-relative `x` offsets `-18` (`+0xf8`)
and `-30` (`+0xfc`), colour names `FE_HD_Grey` -> `+0x18c`, `FE_HD_Blue` -> `+0x190`,
`FE_HD_White` -> `+0x194`, `FE_HD_LightGrey` -> `+0x198`, and the arrow texture
`Data\FE\Images\HD_options_arrow.gtf` (TOC slot `0x4cc`; `DATA02`, 32x32, a white
right-pointing triangle from column 8 to 23, rows 6 to 25).

`List_CreateWidgets` (`0x001bdb48`) builds three `Image` children - a 32x32 marker at
`(X + 8, Y + 4)` coloured `0` (invisible) at creation, a **left** arrow at
`x = X + 520 - 18` drawn with negative width (mirrored), a **right** arrow at
`x = X + 520 - 30`, both in `arrowcolor` (`+0xb8`) - and two `Block`s: the label box at
`(X, Y)`, width 520, landing style `0`; the value box at `(X + 520 + 10, Y)`, width
280, landing style `1`. Height is the constructor's 40.

`List_Update` (`0x001c03e0`): a focus fraction `+0x144` eases toward `1.0` when the
widget is focused and `0.0` when not, at `0.16667` (`0x008ad9c0`) of the remaining
distance per frame; the value block's width is `280 + (340 - 280) * fraction`, set
through `List_SetValueBlockWidth` (`0x001be208`). Both blocks are `HD_Blue` while
focused and `HD_Grey` otherwise, or `HD_LightGrey` with text at `0x3fffffff` when the
row is disabled (`+0x34 & 0x100` clear). The marker image blinks on the same 17-frame
counter the strip's underline does, white for `0..7` and invisible for `8..16`.

The Fury settings capture (`hd-settings-screenshot-2/01.png`, `0.639` scale) agrees:
row boxes `39` units tall (`40` less the inset) with a 2-px `(150,150,150)` border
over a `(102,102,102)` inside, and the selected `MENU STYLE` value box wider than the
rows above it by the eased 60.

## A standalone `<Block>`: parse, selectable, update (2026-09-28)

The three menu classes above build their `Block`s in code. A screen can also
author one directly, and `EndRace Menu`'s five options and `EndRace Results`'
two column headers do. What such a block draws is its own parser and its own
update, found off the attribute strings beside `Block_Item.cpp`
(`0x007891c0` `selectable`, `0x007891d0` `ArrowColor`, `0x00789218` `shaped`;
`scripts/ps3-toc.py attrib` names one function for all three). Every
function below reads TOC `0x008ad4d8`, `exact`.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0018df70` | `Block_ParseXml` | 85 |
| `0x0018c0e8` | `Block_MakeSelectable` | 75 |
| `0x00189c38` | `Block_CreateLabel` | 75 |
| `0x0018d588` | `Block_Update` | 80 |

### `Block_ParseXml` - `0x0018df70`

Vtable slot 18 of `0x008654e8`. The attribute chain, each string resolved
through the function's own TOC (`-0xeb8` .. `-0xe58`):

| Attribute | Lands in |
| --- | --- |
| `IDString` / `string` | label text (locals) |
| `X` / `Y` / `Width` / `Height` | `+0xa4` / `+0xa8` / `+0xac` / `+0xb0` |
| `Color` | `+0xbc` (the fill; copied to `+0xc4` once parsed) |
| `ActiveColor` | `+0xc0` |
| `DeltaWidth` / `DeltaHeight` | `+0xb4` / `+0xb8` |
| `scroll` | `+0xcd` |
| `unselectable` / `selectable` | locals, compared against `"true"` / `"1"` |
| `ArrowColor` | `+0x154` |
| `TextColor` | `+0x150` |
| `AlwaysSolidColor` | `+0x108` |
| `RenderEdges` | `+0x10c` |
| `shaped` | local, then stored to `+0x11c` - the landing-style byte |
| `TextScale` | `+0xec` |

Unauthored, the defaults are `Block_Construct`'s: height `40.0`, `Color`
`0xffffffff`, **`ActiveColor` the literal `0xff8ac0ca`** (`+0xc0`; neither
`Block_Construct` nor the widget base constructor it calls first,
`0x00165ba8`, sets an "is a global" bit - the base one zeroes the whole
`+0x80` flag word), `TextScale` `0x3f4ccccd` =
`0.8`. `EndRace Menu` authors `Color="0xff646464"` and no `ActiveColor`, so
its focused option is `0xff8ac0ca`. A `selectable="true"` block calls
`Block_MakeSelectable` with its `ArrowColor`.

### `Block_MakeSelectable` - `0x0018c0e8`

Sets `+0x159` (the byte `Block_Update` keys focus on) and `+0x15a`, stores
the resting width in `+0x160`, and creates an `Image` child (`+0x168`,
`Widget_CreateChildByType(.., "Image", ..)`) of
`Data\FE\Images\HD_options_arrow.gtf` - TOC `-0xef4` - at `X + 8.0`
(`0x008ac554`), `Y + 4.0` (`0x008ac588`), `32.0` square (`0x42000000`),
tinted `ArrowColor` through `Image_SetVertexColours`. The same arrow, and
the same `(8, 4)` offset, `List_CreateWidgets` gives a settings row's marker.

### `Block_CreateLabel` - `0x00189c38`

Creates the label as a `Text` child (`+0x164`) in the `default` font,
coloured `TextColor`, scaled `TextScale`, at `X + 40.0` (`0x008ac55c`) and -
on an output of 720 lines or more - `Y + 3.0` (below 720 lines: `Y`, with
the scale times `1.25`). The line count is the console's video mode, not a
window this build opens.

### `Block_Update` - `0x0018d588`

Vtable slot 3. For a block with `+0x159` set:

- **Focused** (`+0x34 & 0x200`): colour `+0xbc = +0xc0` (`ActiveColor`); a
  byte counter at `+0x158` counts up and wraps past `0x11`; the arrow image
  is shown (`|= 4`) for counter values `0..7` and hidden for `8..16` - eight
  ticks on, nine off, the rhythm `HorizMenu_LayoutBlocks` gives the strip's
  underline. The focus target is `1.0` (`0x008ac540`).
- **Unfocused**: arrow hidden; colour `+0xbc = +0xc4` (the parsed `Color`)
  when enabled (`+0x34 & 0x100`), else `0x5fdedede` with the label at
  `0x3fffffff`. Focus target `0.0`, and the counter is left where it was.
- **Either way**: the label takes `TextColor`, and the focus fraction
  `+0x15c` moves toward its target by `|target - f| * 0x3e2aaaab` (one sixth,
  `0x008ac608`). The block's width is `+0x160 + f * 60.0` (`0x008ac60c`).

So a focused standalone block is its `ActiveColor`, sixty units wider once
eased, with a blinking arrow at its left edge; nothing brightens its label.
Implemented in `oag_ui::menu::block::Focus` and
`oag_ui_screens::endrace::hd::hd_menu_draw_list`; the parsed attributes are
`oag_ui::screen::BlockWidget`.

## `FrontEnd_IsFuryStyle` - `0x0015b620`

Returns the byte at `0x009947c8`. It is read in `Block_Render` to pick the top-left
corner piece (kind 2, chamfered, when set; kind 5, square, when clear) and in
`Block_DrawTopBand`/`Block_DrawFill` to pick the fill's left end and the second pass's
swatch. Both effects are visible in the captures on exactly the style axis: the Fury
capture's tabs have a 10-unit top-left chamfer (`03-main-menu-after.png`, rows 49-55)
and a `0.68` inside; the HD capture's have a square top-left and an opaque inside.
Named at 72 on that behavioural match. **What writes the byte was not found** - the
`FE Style` option's own apply path (`0x0024b408`, the settings-save handler, writes a
run of bytes on a runtime object whose base this page did not resolve) is the place
to look, and hd-frontend.md's open question of whether the option swaps `skin.xml` or
rebinds the globals is still open.

## Two helpers used throughout

- `Widget_CreateChildByType` (`0x0016aaa8`) - `(parent, type_name, owner, order)`,
  creates a child widget of a registered type by its XML element name: `"Block"`,
  `"Image"` and `"Text"` are the three names the classes above pass it. Confidence 72:
  the three call shapes agree and the names are literal, but the registry it looks the
  type up in was not read.
- `Image_SetVertexColours` (`0x001b5708`) - writes one colour into an `Image`'s four
  vertex slots `+0xbc..+0xc8` and sets or clears the low four bits of `+0x80`, the
  "colour is a global reference" flags. Colour `0` is how the strip hides its underline.

## Not recorded

- `0x0015b778` / `0x0015b968`, the `FEGlobals` lookup and colour-resolve pair every
  class above calls. Their shape is clear from use, but neither was read.
- `0x0018b7e8`, which copies a `+0x74` opacity onto the block and two children.
- The `+0x14c` extra-highlight pass in `Block_Render`, which none of the three menu
  classes sets; whichever widget does was not looked for.
- `List_Update`'s arrow dimming: the captures show an arrow bright when a step in
  that direction is possible and dim otherwise, but the function's own
  `0x3fffffff`/`arrowcolor` split only covers the disabled-row case, so the per-arrow
  rule is still unread.
