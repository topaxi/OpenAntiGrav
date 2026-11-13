# The 2048 campaign map: an event is six painted cells of the `Canvas3D` lattice

Functions in `eboot.elf` (Wipeout 2048, Vita, `PCSF00007` patch v1.04), image base `0x81000000`,
Ghidra program `/2048/eboot-vita-2048-eu-v104.elf`. 2026-11-10, lane `2048-campaign-map`.

Question: where does the original put each event tile of the base campaign map, and what does the
map draw behind them. [frontend-campaign-map.md](frontend-campaign-map.md) left the writer of the
"cached position" unfound and [canvas3d-lattice.md](canvas3d-lattice.md) left the scroll writer at
confidence 55. Both are answered here: **there is no pixel law of its own**. A tile is a block of
cells of the `Canvas3D` triangle lattice, raised and coloured, and the screen position falls out of
the canvas camera.

## The law

1. **Cell** (confidence 92). For an event with `m_x`, `m_y` (`GameModeBase+0x2c4/+0x2c8`) and
   `m_canvasTweak_x/_y` (`+0x314/+0x318`):

   ```text
   col = uint(55.0 + 4.0 * m_x + tweak_x)        row = uint(25.0 + 2.0 * m_y + tweak_y)
   ```

   The quad `(4.0, 2.0, 55.0, 25.0)` is `.bss` at `0x81880448..0x81880454`, stored by the static
   initialiser at `0x810f3f3c..0x810f3f68` (`movt #0x4080`, `#0x40000000`, `movt #0x425c`,
   `movt #0x41c8`) and read back live as `[4.0, 2.0, 55.0, 25.0]`. The DLC tiers use `(3.0, 2.0,
   55.0, 25.0)` plus a per-grid offset ([frontend-campaign-map.md](frontend-campaign-map.md)): the
   same canvas, the same bias, a narrower column step.
2. **Tile** (confidence 90). The event's canvas button (`event+0x1ec`, vtable `0x8150abb8`) owns
   the six cells `(col .. col+2) x (row, row+1)` (`+0x1c/+0x20` = col, row; `+0x24/+0x28` = 3, 2;
   cell list at `+0x44`, read live on four buttons). `col + row` is always even (`55 + 25` even,
   steps 4 and 2, and every authored tweak pair keeps the parity), so the six triangles are the
   hexagon around the lattice vertex

   ```text
   centre = ((col + 1) * 0.5, (row + 0.5) * 0.8660254)
   ```

   with corners one unit away. Its cells carry the state colour and alpha `0xfa` (height 0.98;
   `0xf7` while it settles), so the tile is a hexagonal prism 0.98 high.
3. **Screen** (confidence 90). The canvas camera of [canvas3d-lattice.md](canvas3d-lattice.md),
   with `ax = cos(0.1) / 8`, `bx = -sin(0.1) sin(0.45) / 8`, `zx = -sin(0.1) cos(0.45) / 8`,
   `by = -cos(0.45) / (8 * 5/9)`, `zy = sin(0.45) / (8 * 5/9)` and the pan `(px, py)`
   (`canvas+0xb8/+0xbc`):

   ```text
   x' = ax (X - px) + bx (Y + py) + zx (Z - 8)        sx = (x' + 1) * 480
   y' =               by (Y + py) + zy (Z - 8)        sy = (1 - y') * 272
   ```

   This is the matrix at `canvas+0x5ec90` written out: its translation row is
   `(-ax px + bx py - 8 zx, by py - 8 zy)`, checked on three live pans (reset, title, map: rows
   `(-3.29062, -0.86307)`, `(-9.56032, 4.65163)`, `(-4.69069, 6.49192)`).
4. **Pan from scroll** (confidence 90, [canvas3d-lattice.md](canvas3d-lattice.md) and re-read
   live). With the parent `FeShellTouchScroll` at `(Sx, Sy)` and the reset pan `(27.16266,
   0.395522)`: `px = 27.16266 - Sx / 960 * 16.5112`, `py = 0.395522 + Sy / 544 * 9.8716`. The span
   `(16.5112, 9.8716)` is twice the plane distance from the screen centre to the bottom right
   corner, so it carries the camera's shear: a scroll of `d` moves the picture by
   `(1.02684 dx - 0.047296 dy, dy)` pixels, not by `d`.

One cell step of `m_x` is 4 columns = 2 units = 119.4 px; one of `m_y` is 2 rows = 1.732 units =
95.4 px down and 4.5 px left. A tile's top face is 119.4 px wide and 95.4 px tall, its front wall
26.1 px (`0.98 * 272 * zy`).

## Evidence

**Static.** Four functions compute the same expression (the `uint` conversions, then `X = col *
DAT_81732ab4` (0.5), `Y = row * DAT_81732ab8` (0.866), the four rows of `canvas+0x5ec90`, the
perspective divide, `(x/w + 1) * 0.5 * 960`, `(2 - (y/w + 1)) * 0.5 * 544`): `0x810ef98c`,
`0x810efc4a`, `0x810efff6`, `0x810eecb4`.

**Live** (Vita3K 0.2.1, EU v1.04, private profile, `scripts/vita3k-mem.py`; the guest stopped for
each read and the window grabbed in the same stop). Save: Events 1 to 3 passed.

| Read | Value |
| --- | --- |
| `0x81880448..54` | 4.0, 2.0, 55.0, 25.0 |
| event list `0x81965b60` (`+4` begin, `+8` end) | 120 `GameModeBase` pointers |
| `2048 - Event 3`'s button (`m` = (5, 5)) | `+0x1c/+0x20` = (75, 35), cells (75..77, 35..36), colour `0xff00d020` |
| `canvasTex` at those six cells | `20 d0 00 fa` (the neighbours `ff ff ff 01..22`) |

Tile boxes, predicted from the law against the frame of the same stop and against the committed
reference frames (`data/reference/2048-frontend/`, 960 x 544). "Top" is the top face's bounding
box; the measured box is the connected saturated-green region, which adds the lit wall below it.

| View | Event (`m`) | Predicted top face `x`, `y` | Measured |
| --- | --- | --- | --- |
| frame 13 = the opening view, scroll (-746.61, -2000.56) | (5, 5) | 352.0..471.4 (+2.3 shear), 126.7..222.1 | 352..472, 127.. |
| frame 12 = Event 1 selected, scroll (-425.09, -1714.21) | (3, 3) | 438.7..560.4, 222.0..317.4 | 439..561, 222.. |
| same | (4, 4) | 553.6..675.3, 317.5..412.9 | 554..675, 318.. |
| same | (5, 5) | 668.5..790.2, 412.9..508.3 | 669..790, 413.. |

Both reference frames were re-taken on 2026-11-10 from a fresh boot of the same save: the boxes
agree with the committed frames to the pixel (frame 13: 352..472 and 126..245 on both), so the
committed frames are these two scroll states.

## The scroll: one centring law, used three ways

`TouchScroll_SetScroll(x, y, 480, 272)` (`0x811042ae`) stores `pos = (480 - x, 272 - y)` at
`scroll+0xa0/+0xa4`; `TouchScroll_SetTarget` (`0x81104294`) stores the same difference at
`+0xe0/+0xe4` with a rate at `+0xe8` and a flag at `+0xdc`, and the scroll's update eases towards
it by `rate * dt` of the remaining distance each frame (measured: 13 steps of 8.19 to 8.71 % at
rate 5.0, confidence 80). Every caller passes

```text
(x, y) = screen(cell ground point) - item position
```

where the ground point is `(col * 0.5, row * 0.866, 0)` (the centre of the tile's first cell, not
the hexagon's centre) and the item position (`TouchCampaign_Item+0x48/+0x4c`) is the item's place
under the scroll, which equals the scroll position one frame later.

| Use | Function | Live |
| --- | --- | --- |
| Map opens | `0x810efff6`, `0x810efc4a`: reset the camera (`Canvas3D_ResetCamera`), zero the scroll, then set it directly | scroll (0, 0) -> (-695.175, -2000.557), stored `x, y` = (1175.175, 2272.557): the reset camera's projection of cell (79, 37) |
| One second later | `TouchCampaign_Update` `0x810f2f04`, state 1 -> 2 (timer `+0x154`, 1.0 s): the same expression under the moved camera, as a target at rate 5.0 | target (-771.135, -2000.561); the ease stops at (-746.615, -2000.560) |
| A tap on a tile | `0x810eecb4` (sounds `newGUI_select`, `newGUI_trans03`, then the target, then the card's page list) | target (-425.094, -1714.214) from (-746.614, -2000.560) for cell (67, 31); reached in full |

**Why the second step exists and where it stops (confidence 85).** Because of the shear in the
span, centring is not exact in `x`: after the direct set the cell sits at `(555.96, 272)`, not
`(480, 272)`. The re-centre aims `75.96 px` further, and state 2 ends as soon as
`TouchScroll_IsNearTarget(25.0)` (`0x811042ca`: `dx^2 + dy^2 <= 25^2`) holds: at 60 Hz the
remaining distance is `75.96 * (11/12)^n`, 26.76 at `n = 12` and 24.53 at `n = 13`, and the live
stop is 24.52 short. Three boots (2026-09-21, two on 2026-11-10) stop on the same value to 0.001.
The current event's ground point therefore rests 23 px right of centre in the opening view. Which
event that is: the first of the chain from `"2048 - Event 1"` (`FUN_812b3a6a` = next) whose
progress word `+0x2d8` is below 3, or the event named at `*(0x81545468)+0x32588` when one is set.

**The title's (-3000, -1500)** ([canvas3d-lattice.md](canvas3d-lattice.md)) is state 0 of the same
item: `0x810f2d18` sweeps a scroll target from (300, 1600) to (3000, 3350) and the half-width from
6 to 15 over about 7.3 s (`t = clamp(0.137 * elapsed, 0, 1)`; statics `0x81880560..6c`). Read, not
executed (confidence 55); the settled title value was not re-derived from it.

**Scroll bounds.** `0x810ef98c` runs every frame of the map: it takes the minimum and maximum of
the events' projected positions and widens them by 650 px each way (`TouchScroll_ResetBounds`,
`TouchScroll_ExtendBounds`). Live: `x` -28.8..5197.8, `y` 1495.3..4386.1 at the opening view. The
decompile's vector code was not followed lane by lane, so which sign and which origin those are in
is not read (confidence 45).

## What the canvas draws behind and around the tiles

The map is the same `Canvas3D` mesh as the title's lattice, with cells painted by up to 700 canvas
items (`canvas+0x5e0d0`, 283 live). `Canvas3D_PaintItems` (`0x810712d2`) calls each item's paint
slot when the dirty byte `canvas+0x5e0a4` is set (`Canvas3D_PrepareDraw` `0x810739a4`).

| Vtable | Live count | What | Confidence |
| --- | --- | --- | --- |
| `0x8150abb8` | 212 | The hex button: 3 x 2 cells, colour `+0x38`, state `+0x3c`, ring mode `+0x34`, icon `u` at `+0x48` (0.415 = the flag, 0.507 = the stopwatch in `canvasTexture.gxt`) | 85 |
| `0x8150ad08` | 1 | The **canvas label** (`"Dummy label button"`): a rectangle of cells painted with a crop of the atlas. The live one is `2048SeasonText`, 14 x 3 cells at (51, 29), atlas origin `v` = 0.4785: the `A-G-R-C 2048` season card ("The season cards", below; the earlier reading as one item holding three cards was wrong) | 90 |
| `0x8150ac48` | 3 | 7 x 4 footprints of 16 cells, colour `0xfaf3f3` | 50 |
| `0x8150acd8`, `0x8150abe8`, `0x8150aca8`, `0x8150ac78`, `0x8150ac18` | 40, 11, 10, 5, 1 | not read | - |

`CanvasItem_Paint` (`0x8106fae2`): for a state 1 or 2 item, every cell of its list gets the item's
colour where the cell's priority (`canvas+0x2f0c4`, 6 for state 1, 7 for state 2) is not higher,
keeping the cell's alpha, or alpha `0xff` on the first paint. The button's own slot
(`0x8106fbd8`) then writes its icon (`0x81072702`) and, when `+0x34` is not 0, the 18 cells that
ring the hexagon (`0x81073aee`, fill type `+0x34`): columns `col-2, col-1, col+3, col+4` on `row`
and `row+1`, and `col-1 .. col+3` on `row-1` and `row+2`. Those are the striped and dotted cells
around and between tiles in the captures; the fill types are atlas shifts in `canvasTex2`
(half-float pairs, live: `(188, 0)`, `(94, 0)`, `(47, 82)`, `(141, 82)` atlas pixels, 619, 335,
60 and 28 cells), i.e. the other outlined triangles in the top left of `canvasTexture.gxt`.

**State to colour** (`0x810ef78c`, per event, off the progress word `+0x2d8`; colours at
`0x817327c4..d4`, read live as RGBA `e0e0e9`, `f3f3fa`, `909090`, `20d000`, `d0d000`):

| `+0x2d8` | Button | Colour |
| --- | --- | --- |
| 0, and vtable `+0x74` says locked | state 0: not painted | - |
| 0, open | state 1 | `0x817327c4` |
| 1 | state 3 (the ring blinks between fill 3 and 4) | - |
| 2 | state 2 | `0x817327cc` (drawn `b5b5b5` / `c3c3c3` live: grey) |
| 3 | state 2 | `0x817327d0`, green `20d000` |
| 4 | state 2 | `0x817327d4`, yellow `d0d000` |

On the measured save the three passed events are 3 and the three events open next are 2:
**an open, unpassed event is the grey tile**. An event not reached yet (progress 0) is not
absent: state 1 paints its six cells flat in `0x817327c4` (a main-chain event) or `0x817327c8`
(a branch event, `0x810ef536`), at the ground, with its own ring ("The ring", below). Only an
event whose `m_options & 0x10` is set (the authored "locked" bit, `GameModeBase` vtable slot
`+0x74`) and whose progress is 0 gets no tile and no ring. Confidence 75 for the table (one save);
the flat tint and the absent events were checked against frame `13` ("The ring").
Correction to the sentence this paragraph carried before 2026-11-12, "an event that is not yet
reachable has no tile": it has the flat tint, not nothing.

**Ground relief.** Unpainted cells carry alpha `0x01..0x22` that changes from frame to frame
(`Canvas3D_UpdateCellHeights`, the music spectrum and the ripple): the faint shaded triangles of the
captures. Not read further.

**The Home page's blurred backdrop is this canvas.** A capture of Home taken from the map shows the
map's own tiles, blurred, behind the five tiles (the parked question of
[canvas3d-lattice.md](canvas3d-lattice.md)'s neighbours). Seen once, confidence 70.

## Names

| Address | Name | Conf | Evidence |
| --- | --- | --- | --- |
| `0x810efff6` | `TouchCampaign_SetMapActive` | 70 | Shows or hides 48 named widgets, stores the flag at `+0x168`; on show: clear cells, apply states, reset camera, centre, state 2 |
| `0x810efc4a` | `TouchCampaign_CentreOnCurrentEvent` | 80 | The arithmetic above; its stored `x, y` read live |
| `0x810eecb4` | `TouchCampaign_SelectEvent` | 85 | Live target reproduced to 0.05 px |
| `0x810f2f04` | `TouchCampaign_Update` | 75 | Timer and states at `+0x150/+0x154`; live 1 -> 2 -> 4 with the timings above. Ghidra's decompile fails on it; read as disassembly |
| `0x810f2d18` | `TouchCampaign_UpdateIntroSweep` | 55 | The 300..3000 sweep, read only |
| `0x810ef98c` | `TouchCampaign_UpdateScrollBounds` | 70 | Min and max of the projections, 650 px margin |
| `0x81074da4` | `CanvasLabel_SetShown` | 80 | Creates the label item on first show and sets its state to 2, or to 0 when hidden; a label whose linked event's progress is not above 1 is hidden ("The season cards") |
| `0x81070670` | `CanvasLabel_Paint` | 85 | The label item's paint slot: the icon writer over its cell list, then priority 5 on every cell when the state is 2 |
| `0x813c5ca0` | `CanvasLabel_BuildCellList` | 80 | Vtable slot `+0x10`: the cell list of a `w x h` rectangle, `(col + i) + (row + j) * 401` |
| `0x810ef3f8` | `TouchCampaign_ClearCanvasCells` | 75 | Fills the 401 x 120 colours with `0xffffff`, zeroes priorities and `canvasTex2` |
| `0x810ef78c` | `TouchCampaign_ApplyEventStates` | 70 | The state table |
| `0x810ef5fe` | `TouchCampaign_PaintChainRings` | 60 | Walks the chain from Event 1 setting ring mode 3 or 4 and colour |
| `0x8106fa8c` | `CanvasItem_SetState` | 80 | Stores `+0x3c`, raises the dirty byte |
| `0x8106fae2` | `CanvasItem_Paint` | 80 | Above |
| `0x8106fbd8` | `CanvasButton_PaintIconAndRing` | 70 | Above |
| `0x81072702` | `CanvasButton_PaintIconCells` | 70 | The icon writer: a half-float `(u, v)` shift per cell of the button's list, "The prism" below |
| `0x81071560` | `Canvas3D_AddItem` | 80 | First free of 700 slots at `+0x5e0d0`, count at `+0x5ebc0` |
| `0x810712d2` | `Canvas3D_PaintItems` | 80 | Above |
| `0x810739a4` | `Canvas3D_PrepareDraw` | 75 | Matrix, mesh window, repaint when dirty |
| `0x81104294` | `TouchScroll_SetTarget` | 85 | Live target and ease |
| `0x811042ca` | `TouchScroll_IsNearTarget` | 85 | Decompile is four lines; the live stop distance |
| `0x81104228` | `TouchScroll_ExtendBounds` | 80 | Four compares and stores |
| `0x81104272` | `TouchScroll_ResetBounds` | 85 | Stores +-10000 |
| `0x81073aee` | `Canvas3D_WriteRingCell` | 85 | `(canvas, col, row, mode)`: the mode's shift into `canvasTex2` if the cell's priority is not above it; decompiled, "The ring" |
| `0x810ef536` | `TouchCampaign_PaintBranchRings` | 70 | The chain walk's twin over a branch: modes 1 and 2, colour `0x817327c8` for an unreached event |
| `0x81072a32` | `Canvas3D_InitStaticConstants` | 70 | Stores the offset indices, the ring shifts `0x81732694..ac` and `g_CanvasStateColours` |
| `0x81732694` | `g_CanvasRingShift3` | 90 | `(94, 0)` atlas px: light stripes |
| `0x8173269c` | `g_CanvasRingShift4` | 90 | `(47, 82)`: dark stripes |
| `0x817326a4` | `g_CanvasRingShift1` | 90 | `(188, 0)`: light dots |
| `0x817326ac` | `g_CanvasRingShift2` | 90 | `(141, 82)`: dark dots |
| `0x8151ea78` | `g_CampaignSpeedLapNames` | 85 | 40 name pointers, `"<circuit> Speed Lap - <class>"` |
| `0x81880448` | `g_CampaignCellScaleBias` | 90 | Initialiser, live read, the frames |
| `0x81965b60` | `g_CampaignEventList` | 75 | 120 events live; begin and end at `+4/+8` |
| `0x817327c4` | `g_CanvasStateColours` | 70 | Five RGBA words, live |

`TouchScroll_SetScroll` (`0x811042ae`, [canvas3d-lattice.md](canvas3d-lattice.md)) rises from 65
to 85 on the same live evidence; its row stays on that page.

## Withdrawn

- "`+0x15c/+0x160` is a cached position some unfound step derives from `m_x`/`m_y`"
  ([frontend-campaign-map.md](frontend-campaign-map.md), 2026-09-21): on the base tier the event's
  position is recomputed from `m_x`/`m_y` at every use. The DLC tier functions' `+0x15c/+0x160`
  are fields of their own node records, not of `GameModeBase`.
- "`TouchCampaign_Item`'s constructor has no caller, so `.init_array`": the item lives at
  `0x820cf880` at run time with vtable `0x8150e958` and its `+0xf0` pointing at the shell's scroll;
  how it is created was not chased (the `Create` functions are reached through the item factory's
  table, the usual shape for `NEWGUI` tags).

## Wired

`oag_ui::frontend::canvas_camera` is the camera and the scroll law, `oag_2048::campaign::tier::MAP`
the quad (the title's `TouchFrontEnd::map_nodes`, applied by `oag_title::NodeMap::nudged`), and `frontend::campaign_map` places every tile through them: the baked prism
("The prism", below), the lattice beneath from the picture the title page
bakes (shifted to the view's phase by whole lattice periods), the map opening where the two-step
centring comes to rest, and a selection scrolling by the tap's law. The downloadable tiers go
through the same camera with their own `(3, 2)` step and grid offsets.

Tile centres, ours (the prism picture's anchor, the middle of its top face) against the reference frames' top faces:

| View | Event | Theirs | Ours | Delta |
| --- | --- | --- | --- | --- |
| frame 13, the opening view | 3 | (412.0, 174.7) | (411.7, 174.3) | (-0.3, -0.4) |
| same | 4 | (527.0, 268.0) | (526.6, 269.8) | (-0.4, +1.8) |
| frame 12, Event 1 tapped | 1 | (500.0, 269.7) | (498.5, 269.8) | (-1.5, +0.1) |
| same | 2 | (614.5, 365.7) | (613.4, 365.2) | (-1.1, -0.5) |
| same | 3 | (729.5, 460.7) | (728.3, 460.7) | (-1.2, 0.0) |

The other tiles of those frames are cut by the screen edge or covered by a notification banner;
their visible edges agree to 2 px in `x`. The lattice under the map: row lines at 344, 392, 440,
487, 535 against the frame's 343, 391, 439, 487, 535, and the diagonals' crossings on row 452 at
25, 68, 85, 128, 144, 188 against 25, 68, 85, 128, 145, 187.

Pinned by `crates/ui/src/frontend/canvas_camera/tests.rs` (the law against the live reads),
`the_campaign_map_puts_its_tiles_where_the_canvas_camera_does` in
`crates/game/tests/vita_2048_boot_ground_truth.rs` (the disc's events at the two reference views;
red on the even grid) and, for the prism, `the_opening_view_renders_the_prism_the_rings_and_no_prism_for_an_unreached_event`
in the same file (the rendered map: green face, a wall that darkens towards its foot, no tinted square
around the tile).

Not wired, with what each needs: the other item classes, the 45 other canvas labels ("The season cards"), the ground relief,
the scroll bounds, the wait and the eases (ours arrives at once), the icon
glyph through the cells' atlas shift (below). The prism itself is wired
("The prism", below).

## The prism: the mesh's seven triangles, drawn by the two programs (2026-11-12, lane `w2048-frontend-canvas`)

**What a tile is on screen** (confidence 85 for the picture, 70 for the foot
shade's level). `Canvas3D_BuildLatticeMesh` writes **seven** triangles per cell
(21 `u16` indices at `0x81073cb8`, read from the decompile): the top `(0, 1, 2)`
and three walls, as vertex numbers `(0, 2, 3) (3, 2, 5)`, `(0, 1, 4) (4, 3, 0)`,
`(1, 2, 4) (2, 5, 4)`, vertices 3 to 5 being 0 to 2 at `z = -1` with the same
`texCoord`. With a cell's alpha `0.98`, `h` is `0.98` on the top and `-0.02`
at the foot of a wall, and the vertex program's occlusion
`s = 1/6 * sum max(alpha_k - h, 0)` (`canvas3d-lattice.md`) is 0 on the top face
and up to 1 at the foot of a wall: **the wall's dark foot is the vertex colour,
not a separate shading pass**. `col + row` even makes every tile the same
picture, translated.

**What the icon writer does to the cells** (`0x81072702`, reached from
`CanvasButton_PaintIconAndRing` only for state 2). It writes a `(u, v)` shift
(two half-floats, `canvasTex2`) into each of the button's cells: `u = icon_u +
0.0459 k` and `v = icon_v + 0.0400 row`, where `0.0459` is `94 / 2048` and
`0.0400` is `82 / 2048` (the cell's atlas triangle's width and height), `k`
stepping once per pair of cells along a row and starting half a step back on a
row of the other parity. The six cells therefore read the **icon's own region of
`canvasTexture.gxt`** in the same triangular tiling. That region is clear white
around the glyph: the decoded sheet reads `(255, 255, 255, 0)` at ten texels
beside the flag, the stopwatch and the crosshair, and `(255, 255, 255, 255)`
inside the glyph. So the original's tile face and walls carry **no outline
line** (frame `13` shows none) and the flag or stopwatch is the canvas's own
pixels on the face. The `icon_u`, `icon_v` per event shape are unread here (two
`u` were read live: 0.415 the flag, 0.507 the stopwatch; `v` not read).

**Wired.** `oag_ui::frontend::canvas_prism` runs `Canvas3D_vp` and `Canvas3D_fp`
in software over one tile and the ground around it (z-buffer with `LESS_EQUAL`,
the occlusion shade, a painted cell's texel the clear white above, the composite
pass's square-root encode of `canvas3d-lattice.md`, "Part 2"), at three samples
a pixel along each axis, and the map draws the picture at each tile's projected
top-face centre as an ordinary sprite. **Why it is baked and not a GPU draw**:
the camera is orthographic and fixed, so a prism's picture only translates with
the pan, and `sqrt(t * c * colour) = sqrt(t * c) * sqrt(colour)` lets one white
picture serve every state through a sprite tint; the picture is derived from the
mesh and the programs, with no hand-placed art. The ground is drawn as a black
overlay with alpha `1 - sqrt(c)`, so it darkens the lattice picture beneath it
and does not repeat it. A tile is placed by the camera's projection of
`tile_top` and not rounded to a pixel: rounded, Event 4 of frame `13` read 2.0 px
off in `y` against 1.76 unrounded (the table above is the unrounded place).

**Colours: the canvas's own, `g_CanvasStateColours`** (read live above):
`20d000` a passed event, `d0d000` an elite one, `909090` an open one. The
`Skin.xml` globals the flat art used (`Pass2048` `0x018400`, `Blue2048`) are a
different set and the frames show the canvas's. An event not reached
yet is not a prism: it is the flat state-1 tint, `e0e0e9` (`0x817327c4`) on the main chain and `f3f3fa`
(`0x817327c8`) on a branch ("The ring").

**Against frame `13`** (`data/reference/2048-frontend/13-campaign-map-mid-cluster.png`,
the opening view with Events 1 to 3 passed): green `(90, 230, 0)` on the face
both sides; the wall's shape, the face outline and the lit left wall agree on
the stopwatch tile. The tile centres are unchanged by the bake (table above).
Measured difference: down the wall under Event 3 (column 395, rows 205 to 262)
the green channel goes `231 -> 161` in the frame and `230 -> 192` in ours, so
**our foot is lighter**: the frame's linear shade at the foot is 0.49, ours
0.70. The ground's relief alpha (`0x01..0x22`, unread; the bake's ground is flat
at 0) raises the six-cell sum, and one tile's shade at a vertex shared with a
neighbouring tile is taken from an isolated tile; neither was separated.

**The downloadable tiers share the picture and are not measured with it.** Their
step is `(3, 2)` plus a grid offset, so `col + row` is even only when
`dx + dy + x` is (grid 0's offset `(1, 1)` alternates with `x`). A 3 x 2 block of
odd parity is not the hexagon around a lattice vertex, and the HD/Fury button
class (`TouchCampaignHD_Item`) was not read, so its footprint is unknown. Ours
draws the same hexagonal prism at the block's centre, as the flat art did: a
stand-in for those tiers, not a measurement. A step of 3 columns also makes
neighbouring tiles overlap (a hexagon is 4 columns wide); they are drawn far to
near by row.

Not drawn, and said so: the glyph through the cells' shift (the disc's own mode
icon sprite lies over the face instead, as before), the season cards. The ring cells and
the flat tint of an unreached event are "The ring", below.

## The ring: the chain walk and the cells it paints (2026-11-12, lane `w2048-map-rings`)

**Fill types** (confidence 90, the static initialiser `Canvas3D_InitStaticConstants` `0x81072a32`
stores them, `0x8107303e..0x81073148`): the ring mode `+0x34` of a button picks a `(u, v)` shift of
the cell's atlas triangle in `canvasTex2`, in units of the 2048 px atlas.

| Mode | Data | `(u, v)` | Atlas pixels | Picture in `canvasTexture.gxt` |
| --- | --- | --- | --- | --- |
| 1 | `0x817326a4` | `0x3dbc0000, 0` | (188, 0) | light dots |
| 2 | `0x817326ac` | `0x3d8d0000, 0x3d240000` | (141, 82) | dark dots |
| 3 | `0x81732694` | `0x3d3c0000, 0` | (94, 0) | light stripes |
| 4 | `0x8173269c` | `0x3cbc0000, 0x3d240000` | (47, 82) | dark stripes |

The pictures are the other four outlined triangle pairs beside the lattice's own in the top left of
the atlas (decoded: stripes and dots in a pale blue and a dark navy). The live cell counts of the
earlier read (619, 335, 60, 28 for `(188,0)`, `(94,0)`, `(47,82)`, `(141,82)`) are the number of
cells of modes 1, 3, 4, 2: most of the map is light dots (modes 1, unreached branches) and light
stripes (unreached chain), few are dark.

**The writer** `Canvas3D_WriteRingCell` (`0x81073aee`, confidence 85, decompiled): for `(canvas, col, row,
mode)` the cell is written (shift stored into `canvasTex2` at `(col + row * 512) * 8`) only if its priority
`canvas + (row * 401 + col) * 4 + 0x2f0c4` is not above the mode, and the write sets the priority to
the mode. So the highest mode wins and the order of buttons does not matter. A tile's own cells hold 6
(state 1) or 7 (state 2) (`CanvasItem_Paint`), above every ring. `CanvasButton_PaintIconAndRing`
(`0x8106fbd8`) calls it for 18 cells (decompiled): `(c-2, r) (c-1, r) (c+3, r) (c+4, r)`, the same four
on row `r+1`, and `c-1 .. c+3` on rows `r-1` and `r+2`.

**The walk** (confidence 85 for the modes, decompiled; 60 for the name). `TouchCampaign_PaintChainRings`
(`0x810ef5fe`) starts at the event `"2048 - Event 1"` and follows `+0x2cc` (`M_PNEXTEVENT`); for each
event `E` it sets the button's ring mode `+0x34` and colour `+0x38` by the progress word `+0x2d8`
and the lock test (`vtable +0x74`):

| `E` | progress | lock test | mode | colour |
| --- | --- | --- | --- | --- |
| main chain | 0 | open | 3 | `0x817327c4` |
| main chain | 0 | locked | 0 | none |
| main chain | not 0 | open | 4 | `0x817327cc` (2), `0x817327d0` (3), `0x817327d4` (4), none (1) |
| main chain | not 0 | locked | 3 | `0x817327c4` |

and, for `E`'s branch (`+0x2d0`, `M_PBRANCHEVENT`) and that event's `+0x2cc` successors,
`TouchCampaign_PaintBranchRings` (`0x810ef536`) does the same with modes 1 (unreached) and 2
(reached) and `0x817327c8` for an unreached one. Then a loop over the 40 names of the table at
`0x8151ea78` (`"<circuit> Speed Lap - <class>"`, ten circuits by Flash, Rapier, Phantom, Super P)
sets mode 3 on each with progress above 1. That loop also calls vtable `+0x70(3, 1)` on an event whose
vtable `+0x10` result is not `0xffff` and colours its button `0x817327d0`, else `0x817327cc`; the
meaning of those two slots was not read and ours takes the button's colour from the state alone
(not wired).

**`+0x50` is not a cursor rule.** `0x810ef78c` sets the button byte `+0x50` to 0 for the current event
and for any event below progress 2, else 1; `CanvasItem_Paint` (`0x8106fae2`, decompiled) reads it
as "snap": 1 writes alpha `0xff` into the cells (the raise), 0 keeps the cell's alpha. Which of the
two events the pad or a tap may select is still unread (no caller of the select function
`0x810eecb4` reading a lock was found in this pass); ours keeps every drawn event selectable.

**The lock test** is `GameModeBase` vtable slot `+0x74`: every derived class's slot is the stub at
`0x813ea23c` (`ldr r0, [r0, #0x1a0]; ands r0, r0, #0x10`, so `m_options & 0x10`); the base
vtable `0x81515f88` has the same entry. The campaign's 15 side challenges carry `m_options = 16`
(`"* P Ship Challenge"`, `"* - S Phantom Challenge"`; the census: 13 with 16, 2 with 17) and the other
events of the map carry none, so **a main-chain or branch event is never "locked"** and an unreached
one is state 1 (flat tint, ring 3 or 1), while a side challenge is not drawn at all until its progress
is raised. The same 15 events carry `m_RankRequired` (10 to 50), so the raising is **hypothesised** to be
the player rank (confidence 45) ([2048-campaign.md](../../../formats/2048-campaign.md), "The unlock graph"; not read).

**Read (2026-11-12, [canvas-music-wave.md](canvas-music-wave.md)):** the raising is the XP rank
(`GameModeBase_IsUnlockable` `0x812b3a76`, confidence 80, replacing the 45 above), and it raises
progress to 1, not 2: the button goes to state 3, which draws nothing (`CanvasItem_Paint` paints states
1 and 2 only), until a tap on one of its six cells sets progress 2 (`Canvas3D_TapCell` `0x81071de4`).
The state table's "1, state 3, the ring blinks" therefore never shows a ring for these events.

**Against frame `13`** (Events 1 to 3 passed, Event 4 open): the prediction (rings by the table, a tile's
cells above rings, the highest fill winning) agrees with the reference's texture class on the
visible ring cells: 38 of 51 predicted dark-stripe cells carry dark stripes in the frame, and the 13
that do not are under the notification banner or a prism's wall; no predicted light cell shows
dark stripes. Around the open Event 4 are dark stripes on its chain side and dark dots on the
side of its branch Event 4-1 (state 1, flat `f3f3fa`, light dots about it); Event 5 (main chain, not
reached) is the flat `e0e0e9` hexagon with light stripes. The mean texture energy (mean absolute
difference of horizontally adjacent pixels over a window inside one cell) over six probe cells:
frame `13` 20.6 for dark stripes, 8.4 dark dots, 3.5 light stripes, 0.3 plain lattice; ours 19.9, 10.7, 3.2,
0.0. Pinned by `the_opening_view_renders_the_prism_the_rings_and_no_prism_for_an_unreached_event`
(`crates/game/tests/vita_2048_boot_ground_truth.rs`: red on `e7eede403`, dark stripes 1.7 against 10).

**Wired.** `oag_2048::campaign::map_roles` (24 chain, 36 branch, 40 speed laps on the EU v1.04 package,
`crates/2048/tests/campaign_ground_truth.rs`), `oag_title::MapRole` (the role type, shared by the title package and the front end) on every `MapEvent`,
`oag_ui::frontend::canvas_cells` (the 18-cell ring, the priority rule, the ten cell pictures baked off
the atlas by the lattice's own sampler with the fill's shift, placed by the camera), drawn between the
lattice and the prisms. A tile not reached yet is the fill-0 cell picture tinted by the state colour
(`sqrt` commutes with the composite's encode, as for the prism), without a glyph (the icon writer runs
for state 2 only). The cells are placed to whole pixels from the camera's projection: up to a pixel
off the lattice beneath, which the outlines hide.

**Not drawn, and said so.** The ground relief (the unpainted cells' alpha, so the soft shading of frame
`13` around a flat tile is missing: our flat tile is flat), the blink between modes 3 and 4 for progress 1
(`CanvasButton_PaintIconAndRing` swaps `+0x34` each paint of a state-3 button; ours has no event in
that state), progress 0 on an open event (state 1 with the ring 3, ours has none: an event is reached or
locked). The 15 rank-gated side challenges (`m_options & 0x10`, `MapEvent::rank_gated`) are **hidden**
as the original hides them at rank 1 (frame `12`: no tile at top left): no tile, no ring, skipped by the pad,
the pointer and the travel limits, until a pass is earned on one. The original raises their progress by
player rank (hypothesis, confidence 45); this build has no rank, so until a rank model exists they cannot be
launched from the map (open: "player rank model needed to unlock the 15 rank-gated 2048 challenges
(`m_RankRequired` 10-50)").
The season cards are drawn: "The season cards", below.

## The season cards: canvas labels (2026-11-13, lane `w2048-season-cards`)

**What `0x8150ad08` is** (confidence 90, decompile and disassembly, static): the vtable of the
**canvas label** item, created by `CanvasLabel_SetShown` (`0x81074da4`) for each `<CanvasLabel>` the
front end names. It is not one item holding three cards: `NEWGUI/Definition.xml`'s `<FE3DCanvas>`
authors three labels, one per season, each its own 14 x 3 block with its own origin:

| Label | Cells `(x, y)` | `u`, `v` (atlas fraction) | `linkedevent` |
| --- | --- | --- | --- |
| `2048SeasonText` | (51, 29) | 0, 0.4785 | none |
| `2049SeasonText` | (96, 39) | 0, 0.5981 | `2049 - Event 1` |
| `2050SeasonText` | (131, 53) | 0.3228 (661/2048), 0.6377 | `2050 - Event 1` |

The measured save had one live because a label's item exists only once it is shown: `TouchCampaign_SetMapActive`
(`0x810efff6`) walks 48 names (`0x8151e9b8`; the first three are these) and calls `CanvasLabel_SetShown` on
each. `CanvasLabel_SetShown(label, show)`: a label whose `+200` event pointer is null (no `linkedevent`) or whose
progress word `+0x2d8` is above 1 (open or passed; the state table above) gets its item and state 2; any
other is state 0. So 2048's card is always there and 2049's and 2050's wait for their season's first event.
The other 45 names (the trophies, the HD and Fury group wordmarks) go through the same code; their own
show conditions are unread and **they are not drawn** (the Fury list at `0x8151eb1c` is the nine names
`FUN_810f665e` shows on its own page).

**The paint** (`CanvasLabel_Paint` `0x81070670`, confidence 85; this slot replaces `CanvasItem_Paint`, so the label
writes no colour and no height): `CanvasButton_PaintIconCells` (`0x81072702`) over the label's cell list
with the label's `u`, `v`, then, in state 2, priority **5** into `canvas + 0x2f0c4` for every cell it holds
(only where the cell's priority is not above 5).

**The shift law** (`0x81072702`, confidence 90, read as the full decompile): per row `j` of the block,
`v' = v + j * 82/2048`; per cell, `u' = u + k * 94/2048` where `k` counts pairs. A row whose first cell is apex up
(`row % 2 != (col % 2 == 0)`, flipping every row) opens on a pair, so cells 0 and 1 share `u`, cells 2 and 3
`u + 94/2048`, and so on; the other rows open on one lone cell at `u - 47/2048` and then pair up. Each stored
value is rounded to a half float (round half up on the dropped bits). The vertex shader adds it to the
cell's own atlas triangle, so a pair (an apex-up cell and the apex-down cell beside it) shows one
94-unit slice of the crop. 2048's first row ends at 658 atlas units and 2050's `u` is 661/2048: the crops are
laid side by side in the atlas.

**Wave: the cards do not ride it.** Priority 5 is not below 5 (`canvas-music-wave.md`, section 1), so the wave
neither lifts nor bumps a card cell, and a card outranks every ring mode (1 to 4) and yields to a tile (6, 7).

**Wired** (`oag_ui::frontend::canvas_cells`: `card_shifts`, `bake_card`, `CARD_PRIORITY`; `Screen::canvas_labels`
reads the labels, `season_cards` filters by name): one baked picture per card, drawn over the ring cells and
under the tiles, shown by the rule above. Evidence: `the_season_cards_resolve_to_atlas_rectangles_decoded_from_the_disc`
and `the_season_2048_card_is_drawn_on_the_map` (`crates/game/tests/vita_2048_season_cards_ground_truth.rs`).

**Against frame `12`** (`data/reference/2048-frontend/12-campaign-map-season-2048-start.png`, 2048's card with
Event 1 selected): the card's pixels against ours, best integer shift over the card's box: mean grey error 3.7 of 255 after
a shift of (+26, 0) px, and Event 1's hexagon in the same capture needs (+27, -1) px. That common shift is
the capture's scroll (`--until map:...` stops near, not on, the reference scroll), not a card error; the card
against its own tile is **(-1, +1) px**. Not measured at the exact reference scroll.

**Open**: the 45 other labels and their show conditions; whether `CanvasLabel_BuildCellList`'s slant override
(`+0xc0`, written when the owner is set up) applies to a season card (read as the plain rectangle: the
three entries author no such attribute, confidence 70); the half-pixel rounding of the block's per-cell
offsets (cells are put at whole pixels).

## Omega

**Ring: checked, applies, not wired.** The PS4 `eboot.bin` carries the `u` shifts of modes 4, 2 and 3 and
the `v` shift of modes 2 and 4 as one literal pool of floats (`47/2048`, `141/2048`, `82/2048`, `94/2048` as `0x3cbc0000`,
`0x3d8d0000`, `0x3d240000`, `0x3d3c0000` at `0x018030e4..0x01803112`); mode 1's `188/2048` (`0x3dbc0000`)
was not found as a word beside them (it may be an immediate) and the speed-lap names (ten `"... Speed Lap -
Flash"` strings from `0x01820ce4`), so the same canvas code ships; the walk's decompile and any live
value were not read (no memory readout). Same atlas shifts apply to `canvasTexture.gnf`; not wired.

**Prism: checked, applies, not wired.** Same item and programs ship
(`Canvas3D_vp`/`Canvas3D_fp`, `3dCanvas_Item.cpp`, `canvasTexture.gnf`, the reset
angles: [canvas3d-lattice.md](canvas3d-lattice.md), "HD / Omega cross-check"),
so the mesh and the seven-triangle index layout are the same CPU code; the GCN
programs, the icon writer and the cell textures were not read, and Omega's
front end is HD's, so whether its pages show this map is open
([2048-frontend.md](../../../formats/2048-frontend.md)). Census only; no live
value is readable on shadPS4.

**Checked, applies, not wired.** Census of the PS4 `eboot.bin` (base and patch 1.07): it carries
`"2048 - Event 1"` (`0x82d8ac` / `0x8910e1` in the file), `TouchCampaign_Item.cpp`,
`TouchCampaignHD_Item.cpp` and `canvasTexture`, beside the canvas item and its reset angles
([canvas3d-lattice.md](canvas3d-lattice.md)), so the same map code ships. The quad was looked for
as the four floats within 160 bytes of each other and not found (nine `55.0` words, none beside
`25.0` and `4.0`): its stores are not laid out as on ARM, and the value is unconfirmed. A live
Omega value is **not checkable** (shadPS4 has no memory readout), and whether Omega's front end
(HD's) ever shows this map was not established.

**Season cards: checked, differs.** Omega's `campaign2048_definition.xml` (`data00.psarc`, `Data/plugins/frontend/gui/`)
carries the same three entries with identical `x`, `y`, `u`, `v`, `width`, `height` and `linkedevent`, but spells their tag
`<aCanvasLabel>` (the trophies beside them stay `<CanvasLabel>`), and the PS4 `eboot.bin` has the three name strings
(`2048SeasonText` at `0x888d66`). Whether Omega's engine reads `aCanvasLabel` is unread (no memory readout), and
`oag_omega` declares `touch: None`, so nothing draws a canvas map there; our reader does not read `aCanvasLabel`.
