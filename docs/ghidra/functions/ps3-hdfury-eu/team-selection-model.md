# Team Selection's craft: the screen poses the `ShipModel` widget itself, and fits its depth

2026-11-11. `Team_Selection_Definition.xml` authors one pose for the craft
(`<Model name="ShipModel">`: `OriginX=1220 OriginY=412 z=-24 RotX=0.4 RotY=-0.5 nearZ=1.0
ObeySafeZone=true`), and **the original draws none of it but `nearZ` and `ObeySafeZone`**.
`TeamSelection_Screen`'s update writes the widget's rotation, depth, field of view and origin
every frame, and a helper steps the depth until a box the size of the selected model just fits a
window on screen. This page is that law, read statically (GUI bridge, program
`/hdfury/EBOOT-ps3-hdfury-eu.elf`) and checked against one RPCS3 frame.

All addresses below read TOC `0x008ad4d8` (the main module; [memory.md](memory.md) for the
per-function TOC defect). **Trap**: Ghidra's own string references for `RotX`, `OriginX`,
`Length` and the rest point into `0x005853a0` and `Render_ClassifyBoxAgainstPlanes`, which use
another module's TOC; the readers are found by scanning for the `off(r2)` operand instead.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00254348` | `TeamSelectionScreen_Update` | 85 |
| `0x00250e18` | `TeamSelectionScreen_FitModelDepth` | 90 |
| `0x00250a30` | `TeamSelectionScreen_ProjectFitCorner` | 90 |
| `0x00253458` | `TeamSelectionScreen_Init` | 55 (below 70, carries `_q`) |
| `0x001e7bc8` | `TeamModelItem_ParseElement` | 88 |
| `0x001d5fa8` | `ModelItem_ParseValues` | 85 |
| `0x001d3150` | `ModelItem_Construct` | 78 |
| `0x001d3f08` | `ModelItem_BuildPose` | 75 |
| `0x003be688` | `FeModel_DrawAtOrigin` | 65 (below 70, carries `_q`) |

`0x001d35e8` is `ModelItem_RenderScene` ([menu-backdrop-scene.md](menu-backdrop-scene.md)), the
same widget class: this page adds its lens.

## The widget: what `<Values>` sets and what the lens is

`ModelItem_ParseValues` (`0x001d5fa8`, `Model_Item.cpp`) reads the `<Values>` attributes into
the item, compared against the names at TOC slots `0x008ae35c..0x008ae3c0` in this order:

| Attribute | Field | Constructor default (`0x001d3150`) |
| --- | --- | --- |
| `X`, `Y`, `Z` | `+0xac`, `+0xb0`, `+0xb4` | 0 |
| `RotX`, `RotY`, `RotZ` | `+0xb8`, `+0xbc`, `+0xc0` | 0 |
| `RotationCentreOffsetX`, `Y`, `Z` | `+0xe0`, `+0xe4`, `+0xe8` | 0 |
| `orthoScale`, copied over the three below unless it is the default | `+0xc4` | |
| `orthoScaleX`, `Y`, `Z` | `+0xc8`, `+0xcc`, `+0xd0` | |
| `colour` | `+0xd4` | |
| `startPaused`, `Ztest`, `UseModelCamera` | `+0xd8`, `+0xd9`, `+0xda` (bytes) | 0, 1, 0 |
| `OriginX`, `OriginY` | `+0x10c`, `+0x110` | 0 |
| `nearZ`, `farZ` | `+0x114`, `+0x118` | `20.0`, `100.0` |
| `mirror`, `ObeySafeZone` | `+0x109`, `+0x11c` (bytes) | 0 |
| `mode="orthographic"` | `+0x104 = 1` | 0 (perspective) |
| `FirstPass="yes"` | `+0x108 = 1` | 0 |

**No attribute sets the field of view.** `+0xdc` is the constructor's `57.3` (degrees; the
literal `0x42653333` at `0x008ae2e8`) and only code writes it afterwards. Confidence 85.

`ModelItem_BuildPose` (`0x001d3f08`) builds the model matrix in column form as
`T(x, y, z) * Rx(RotX) * Ry(RotY) * Rz(RotZ)`. Confidence 75: read off the decompile's matrix
products, not re-derived per element; the frame check below would not survive another order.

`ModelItem_RenderScene` (`0x001d35e8`, at `0x001d37a0..0x001d3848`) builds a standard
perspective projection: `f = 1 / tanf(fov * 0.008726646)` (the literal at `0x008ae304` is
`pi / 360`, so the tangent is of half the angle), `x = f / aspect` with the aspect the global at
`0x008c1518` (`1.7777778` in the image), `y = f`, and the depth terms `(n + f) / (n - f)`,
`2nf / (n - f)`, `-1` from `+0x114` and `+0x118`. It then calls `FeModel_DrawAtOrigin`
(`0x003be688`) with `(OriginX + parent x, OriginY + parent y)` and the `ObeySafeZone` byte.
Confidence 85.

`FeModel_DrawAtOrigin` slides the finished picture so the model's origin lands on the grid
point: a device-space offset of `s * ((ox - 960) / 960, (540 - oy) / 540)`, with the picture
itself scaled by `s`, the Safe Area as a fraction, when the byte is set. Confidence 65 on the
static read (one pass over a 2,984-byte function); **the frame check below confirms both
halves**, which is why the law is wired while the name stays below 70.

## What the update writes over the widget

`TeamSelectionScreen_Update` (`0x00254348`), at `0x002546f8..0x002547e8`, with `st` the screen
and `item = *(st + 0x188)`:

| Item field | Written | Constants (TOC slot) |
| --- | --- | --- |
| `RotX` `+0xb8` | `st[0xfe8] + 0.25` | `0x008b1ba0` |
| `RotY` `+0xbc` | `st[0xfec] - 0.6` | `0x008b1b7c` |
| `z` `+0xb4` | `4.0 * st[0xff0] - st[0x1020]` | `0x008b1d34` |
| `x` `+0xac`, `y` `+0xb0` | `-st[0xff4]`, `-st[0xff8]` | |
| `fov` `+0xdc` | `57.3 - 60.0 * st[0x1024]` | `0x008b1ce0`, `0x008b1d3c` |
| `OriginX` `+0x10c` | `1220.0 + st[0x1018]` | `0x008b1ce4` |
| `OriginY` `+0x110` | `500.0 + st[0x101c]` | `0x008b1d40` |

`st[0xfe8]`, `[0xfec]`, `[0xff0]`, `[0xff4]`, `[0xff8]` and `[0x1024]` are the player's own
turn, push, pan and zoom of the craft (pad axes read through `FUN_006778b8`, eased at `0.125`
and `0.25` a frame, the turn wrapped at `+-3.1459`, and stepped back to zero at `0.05` a frame
once `st[0xffc]` has counted 181 idle frames). **All six are zero at rest**, so the rest pose is
`RotX 0.25`, `RotY -0.6`, `fov 57.3`, `z = -st[0x1020]`. Confidence 85 on the rest pose, 65 on
the input half (read, not exercised).

The three eased values follow the fit's targets every frame:

```
st[0x1018] += (st[0x100c] - st[0x1018]) * 0.0625      // origin slide, across
st[0x101c] += (st[0x1010] - st[0x101c]) * 0.0625      // origin slide, down
st[0x1020] += (st[0x1014] - st[0x1020]) * 0.125       // depth
```

(`0x008b1d2c`, `0x008b1d30`.) This is the "new craft cuts in slightly off its rest size and
eases back" that `docs/ui/selection-screens.md` measured off film: the depth target is not reset
when the selection moves, so the next model starts at the last one's depth and eases to its
own. `TeamSelectionScreen_Init` (`0x00253458`, the only other writer, at `0x002538a0`) sets the
target to `13.0`.

## The fit

`TeamSelectionScreen_FitModelDepth(length, st)` (`0x00250e18`) is called once a frame with
`length = *(*(st + 0x194) + 0x1cc)`: the selected `PI_TeamModel`'s `<FE Length>`.
`TeamModelItem_ParseElement` (`0x001e7bc8`, `PI_TeamModel_Item.cpp`) reads `<FE>`'s `Speed`,
`Thrust`, `Handling`, `Shield` and `Length` into `+0x1bc`, `+0x1c0`, `+0x1c4`, `+0x1c8` and
`+0x1cc` (the attribute names are TOC slots `0x008aebdc..0x008aebec`). The disc authors eight
lengths over its 84 rated models: `3.2`, `3.8`, `3.9`, `4.0`, `4.1`, `4.2`, `4.4`, `4.5`.

1. Eight corners `(+-1.125, +-0.75, z)` (the words `0xbf900000`, `0x3f900000`, `0xbf400000`,
   `0x3f400000` at `0x00250e9c..0x00250f00`), with `z` one of two values chosen by the game mode
   at `st + 0xf0`: `-4.2` and `4.2` in Zone (mode 6; `0x008b1bd0`, `0x008b1bcc`), `-2.0` and `0`
   in Detonator (mode 14), else `-length` and `length`.
2. Each corner goes through `TeamSelectionScreen_ProjectFitCorner` (`0x00250a30`) and a running
   minimum and maximum, **both starting at `0.0`**, are kept across and down
   (`0x00251308..0x00251324`).
3. The slide targets are minus the box's centre: `st[0x100c] = -0.5 * (min_x + max_x)`,
   `st[0x1010] = -0.5 * (min_y + max_y)` (`0x008b1bec` is `-0.5`).
4. If the centred box is inside `+-535` across and `+-389` down (`0x008b1bd8..0x008b1be4`) the
   depth target `st[0x1014]` **drops** by `0.1`, else it **rises** by `0.1` (`0x008b1be8`); it
   is then floored at `11.0` (`0x008b1bf0`).

So the target never rests: it alternates between the nearest depth the box fits at and one step
nearer, and the eased depth sits between the two. Confidence 90.

### The projector, and the unit of the window

`TeamSelectionScreen_ProjectFitCorner` (`0x00250a30`), instruction by instruction:

- `Matrix_RotationX(st[0xfe8] + 0.25)` and `Matrix_RotationY(st[0xfec] - 0.6)`
  (`0x00250a80..0x00250af4`), multiplied as `Rx * Ry` and applied to the corner
  (`0x00250b50..0x00250c04`);
- the depth target is subtracted from the result's `z` (`0x00250c08..0x00250c28`,
  `st[0x1014]`, the target and not the eased value);
- a projection is built with `1 / tanf(0.5)` down and that over the aspect global across
  (`0x00250c2c..0x00250ca8`), its last two columns the constants at `0x00799190` and
  `0x007991a0`: `(0, 0, -1.0202, -1)` and `(0, 0, -2.0202, 0)`, a near of 1 and a far of 100;
- the corner is projected, and the result is `x / w * 1920.0` and `y / w * -1080.0`
  (`0x00250d54..0x00250de8`; the literals at `0x008b1bc4` and `0x008b1bc8`).

`x / w` is a normalised device coordinate, `-1..1` across the whole screen, so **the projector's
unit is half a pixel of a 1920 by 1080 frame**: the window of `+-535` by `+-389` is `+-267.5` by
`+-194.5` pixels. Two consequences, both of which the original draws:

- **The hull overhangs the box.** A hull is 12 to 15 units long and its `<FE Length>`, half the
  box's length, is 3.2 to 4.5; Feisar's box rests about 530 pixels wide where its hull is 887.
- **The slide over-corrects.** Step 3 is in the projector's unit and is added to `OriginX` and
  `OriginY`, which are pixels of the grid, so the box's centre ends as far on the other side of
  the origin as it started.

The other reading (the window is pixels, the `1920` a slip for `960`) was tested and fails: it
rests every model on the disc at the floor of `11.0`, which draws Feisar about 9% larger than
the frame, and the fit would then never act.

### Rest depths

From the law, at 16:9 (`oag_game::preview::ship_model::rest`):

| `<FE Length>` | Rest pair | Drawn depth | Slide (across, down) |
| --- | --- | --- | --- |
| 3.2 | 11.0 (floor) | 11.0 | +93, -81 |
| 3.8 | 12.0, 12.1 | 12.05 | +109, -85 |
| 4.5 | 13.6, 13.7 | 13.65 | +118, -84 |

## The frame check

`data/reference/compare-page/hd/menu-ship-select/theirs.png`: RPCS3, 1920 by 1080, Feisar's
classic hull at rest, Safe Area 96 (a RAM read, `docs/formats/hd-hud.md`). The hull's silhouette
(any channel above 24 inside the `SHIP MODEL` frame) against Feisar's `ship.vex` vertices
projected through the law (`hd_ship_preview_ground_truth::the_screens_own_fit_lands_feisar_where_the_original_draws_it`,
which holds each edge to 5 pixels):

| Edge | Original | This law |
| --- | --- | --- |
| Left | 750 | 753.0 |
| Right | 1637 | 1636.0 |
| Top | 256 | 255.9 |
| Bottom | 639 | 638.9 |

All
four edges within 3 pixels fixes the rest rotation, the lens, the near plane's irrelevance, the
depth, the origin, the slide's sign and size, and the Safe Area's scale of picture and origin
together: no one of them can be off and the four still agree.

**Four more teams, four more lengths.** `data/reference/hd-capture/ship-framing/rpcs3-rest-t*.png`
are rest frames of the concept hulls of Feisar, Qirex, Piranha, Assegai and Icaras (2000 by 1200
window grabs; the screen's two rules put the grid at `px = 1058.5 + 0.9406 * (x - 960)`,
`535.7 + 0.9406 * (y - 540)`, Safe Area folded in). Edges in the unscaled grid, original against
the law, where the frame shows the edge against black (the rest are behind backdrop embers or
past the frame's brackets):

| Hull | `<FE Length>` | Rest depth | Left | Right | Top | Bottom |
| --- | --- | --- | --- | --- | --- | --- |
| `Qirex_c1` | 4.5 | 13.65 | 786 / 787 | | 286 / 287 | 706 / 705 |
| `Assegai_c1` | 4.4 | 13.45 | 755 / 751 | 1627 / 1624 | 283 / 285 | |
| `piranha_c1` | 3.9 | 12.25 | | 1665 / 1661 | 268 / 269 | |
| `Feisar_c1` | 3.8 | 12.05 | | 1663 / 1660 | 253 / 257 | |
| `Icaras_c1` | 3.2 | 11.0 (floor) | | 1594 / 1591 | 263 / 263 | |

Every held edge is within 4 pixels, across a depth range of 11.0 to 13.65, so the length is the
fit's argument and the floor is real. The same test holds these (Feisar's concept hull aside).

On the compare page's measure (sigma 7 blur, mean absolute RGB), the hull box
`716,150 1012x650` went from 20.62 to 14.33 and the frame from 9.35 to 7.35. What is left in the
box is not placement: the hull is darker than the original (mean bias R -7.6, G -11.7, B -11.1)
and the Fury backdrop's embers sit behind it where the original's frame has them over the
honeycomb.

## What is wired, and what is not

- **Wired** (HD): the rest pose, the lens, the fit, the slide and `ObeySafeZone`, as
  `oag_title::ShipFit` (`oag_hd::frontend::SHIP_FIT`), drawn by
  `oag_game::preview::ship_model::fitted`. The Zone box is used where the screen draws the Zone
  hull.
- **Not wired**: the easing between two models' depths (the swap still plays
  `oag_game::preview::hull_swap`'s measured settle and its chosen start size, which this law
  replaces: the start is the previous model's rest depth); the player's own turn, pan and zoom;
  the Detonator box (the preview does not draw the Detonator hull).

## Omega and 2048

- **Omega: checked, differs.** `eboot-ps4-omega-eu-v107.bin` carries the fit inside its own
  screen update, `FUN_015a3b80` (`0x015a3b80`): the same constant run at `0x01862f20`
  (`0.25`, `-0.6`, a lens constant `0.472`, `1920.0`, `-1080.0`, `-0.5`, the window `-389`,
  `535`, `-535`, `389`, the floor `11.0`), the same `pos%d` debug name, targets at
  `st[0x7b4..0x7b6]` eased into `st[0x7bb..0x7bd]`. What it does with them differs: the item is
  written with rest angles `+0.4307` and `-0.4438` (`0x01862f00`, `0x01862f04`, at item `+0x134`
  and `+0x138`), a position blended between two sets (`+0x128..+0x130`) and a scale
  (`+0x144..+0x14c`), and its `ShipModel` authors `x=0.5 y=0.1 z=-2.05 nearZ=0.1`. A shadPS4
  frame of the screen shows a smaller, steeper craft than HD's. Omega's teams definition authors
  the same `<FE length>` (Feisar `3.8`), which the reader now carries. Not followed further:
  Omega keeps the pose it borrows (`oag_omega`'s `ship_preview_pose`).
- **2048: checked, differs.** The window's `535.0` occurs nowhere in the v1.04 `eboot.elf` as a
  literal (a byte search for `00 c0 05 44`; an ARM `movw`/`movt` pair would not show, so this is
  absence of the HD constant run, not proof of no fit), its `<FE>` authors `length` beside
  `firepower` and `description`, and its Team Selection is drawn by this build through 2048's
  own path, which this change does not touch.
