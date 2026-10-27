# The pale triangle lattice behind every 2048 page is `3dCanvas_Item`'s procedural mesh, textured from the canvas atlas

Functions in `eboot.elf` (Wipeout 2048, Vita, `PCSF00007` patch v1.04), image base `0x81000000`,
Ghidra program `/2048/eboot-vita-2048-eu-v104.elf`. 2026-10-27, lane `2048-backdrop-re`.

Question: `2048/menu-*` reference captures draw a pale triangle lattice (horizontal-line period about
47.8 px, side about 55 px, 60-degree diagonals, line grey 233-245 on 255) behind every page; ours is
flat white. **No whole disc texture is the lattice** (previous lane, base `data.psarc` 722 FE entries; the
`data1`/`data2.psarc` `data/FE/` entries are demo upsells, HD track icons, team-grid HD icons,
`canvasTextureHD.gxt` and the `gamemodechoice` pair) **but the lattice is a region of one sheet**:
`canvasTexture.gxt` carries an outlined triangle pair in its top-left corner, and the mesh's uvs point
at it (below, 2026-10-27).
`NEWGUI` XML names no backdrop (`ScreenClear` appears only in `Demo_Definition.xml` and
`LegacyInGame_Definition.xml`).

## Where the lattice comes from (confidence 70)

It is geometry built at run time by the `FE3DCanvas` item (`Frontend/Items/3dCanvas_Item.cpp`,
string `0x8142eefc`), the same persistent campaign-map canvas
[frontend-campaign-map.md](frontend-campaign-map.md) already describes. That page and
`docs/formats/2048-frontend.md` left open whether `FE3DCanvas` "also renders a real 3D backdrop
model"; this answers it: it renders a procedural triangle-cell mesh, and the campaign labels
(`canvasTexture.gxt` atlas) sit on its cells.

| Address | Name | Conf | What it does |
| --- | --- | --- | --- |
| `0x81070da0` | `Canvas3D_Ctor` | 75 | Sets vtable `0x8150ad38`, file string, singleton `DAT_8151d3f4`; allocates two cell buffers (`0x40000`, `0x80000`), fills 401x120 cells with `0xffffff`, creates two 512x128 sceGxm textures, allocates 18000-vertex (`0x9e340` = 18000 x 36 bytes) and 63000-index (u16) buffers. |
| `0x810733dc` | `Canvas3D_LoadShaders` | 75 | Looks up `Canvas3D_vp` / `Canvas3D_fp` (strings), uniforms `worldViewProj`, `canvasTex`, `canvasTex2`, `sampleOffsets`, `texCoord2offset`, `posOffset`, attributes `position`, `texCoord`, `texCoord2`, `offsetIndices`. |
| `0x810744e4` | `Canvas3D_InitStatics` | 70 | One-time statics: cell spacing `DAT_81732ab4` = `0x3f000000` (0.5) and `DAT_81732ab8` = `0x3f5db3d7` (0.8660254 = sqrt(3)/2), texcoord steps `DAT_8173298c` = `0x3b000000`, `DAT_81732990` = `0x3c000000`, the sample-offset table `DAT_81732994..`, Canvas3D shader registration. |
| `0x81073cb8` | `Canvas3D_BuildLatticeMesh` | 65 | Writes the mesh (below). Single call site (`FUN_81071300` at `0x81071406`). |
| `0x8107086c` | `Canvas3D_BuildWorldViewProj` | 80 | Builds the canvas camera matrix (`param_1+0x5ec90`): view (flip `y`, translate by the pan and `-half-width`), a rotation from the Euler angles `+0xac..+0xb4`, then an orthographic window of half-width `+0xc0` and `5/9` of it vertically. Executed 2026-10-27, see "The camera is tilted" below. |
| `0x81074280` | `Canvas3D_Draw` | 70 | Binds `Canvas3D_vp/fp`, both cell textures, uploads `worldViewProj` (`+0x5ec90`), `sampleOffsets`, `texCoord2offset` (`+0xd0`), `posOffset` (`+0xc4`), then `sceGxmDraw` (`SceGxm_BC059AFC`) with `DAT_81732828` indices. |

`DAT_8151d3f4` is `g_pCanvas3DInstance` (confidence 70, data row below).

### Mesh law (read, confidence 75; rewritten 2026-10-27 after the shaders were decoded)

- Grid of 90 columns (`< 0x5a`) by 32 rows (`< 0x20`) of cells; each cell is 6 vertices of 36
  bytes and 18 indices, `bVar6` alternating per column and per row. Vertex layout, 9 floats: `position`
  xyz at `+0`, `texCoord` (atlas uv) at `+0x0c`, `texCoord2` (the cell's texel in the cell textures)
  at `+0x14`, `offsetIndices` at `+0x1c`.
- Cell centre: `x = col * 0.5`, `y = row * 0.8660254` (`DAT_81732ab4`, `DAT_81732ab8`). Adjacent cells
  are equilateral triangles of side 1.0, alternating apex-up and apex-down, with `y` pointing down the
  screen. Corner offsets for one parity: (-0.5, +0.4330127), (+0.5, +0.4330127), (0, -0.4330127)
  (`DAT_81732c38..`, apex up); the other: (-0.5, -0.4330127), (0, +0.4330127), (+0.5, -0.4330127)
  (`DAT_81732c20..`).
- Vertices 0 to 2 are the triangle's corners at `z = 0`. Vertices 3 to 5 repeat them at **`z = -1.0`**
  (the `0xbf800000` stores; the earlier reading took it for an `offsetIndices` marker): a skirt, not
  an edge band. `offsetIndices` is the corner number: 0, 1, 2 on one parity and 3, 4, 5 on the other,
  in both components, copied onto the skirt vertices.
- **The atlas triangle each parity maps to** (`texCoord`, floats at `DAT_81732ac0..aec`, times 2048):
  apex-up cells take (14, 122), (107, 122), (60, 40); apex-down cells take (60, 40), (107, 122),
  (155, 40). Both triangles lie in the top-left corner of `canvasTexture.gxt`'s 2048 x 2048 sheet.
  Corner and atlas vertex correspond (bottom-left to bottom-left, apex to apex).
- `texCoord2` is the cell's texel in the 512 x 128 cell textures: `(col + 0.5) / 512`,
  `(row + 0.5) / 128` (`DAT_8173298c`, `DAT_81732990`), so each cell reads one texel of `canvasTex`
  and `canvasTex2`.

## What the shaders compute (decoded 2026-10-27, lane `gxp-usse-decoder`)

`Canvas3D_vp` is program #16 of the patch eboot's 67 embedded programs (84 primary, 4 secondary
instructions) and `Canvas3D_fp` #46 (7 primary). Both list in full with `python3
scripts/vita-usse.py disasm <patch eboot.elf> 16` and `... 46` (decoder and checks:
[gxp.md](../../../formats/gxp.md#the-instruction-stream)). The base build's vertex program is #25 (`0x51899c`, 88 + 2 instructions) and was not read line by
line; the v1.04 patch's is read here.

**Register map (confidence 85, measured from the parameter table and the container table).**
Vertex inputs `position` `pa0..3`, `texCoord` `pa4..7`, `texCoord2` `pa8..11`, `offsetIndices` `pa12`.
Uniforms: `worldViewProj` `sa0..15` (column-major, four rows), `texCoord2offset` `sa16..17`,
`posOffset` `sa18..20`, `sampleOffsets` `sa22..93` (36 vec2). The sampler descriptors follow at `sa94`
(`canvasTex`) and `sa98` (`canvasTex2`); `sa102..109` are the program's literal words; the secondary
program zeroes the three lod registers `sa110`, `sa112`, `sa114`.

**Vertex program (confidence 70 on the whole, 85 per instruction):**

1. `o4.xy = texCoord2 + texCoord2offset`: the cell's texel plus a CPU offset.
2. Two **vertex-texture fetches** at lod 0: `canvasTex(o4)` into `r4..r7` (the cell's colour and a
   height in alpha) and `canvasTex2(o4)` into `r0..r3`.
3. `o2.xy = position.xy + posOffset.xy` and **height `h = position.z + posOffset.z + canvasTex(o4).a`**.
4. **Atlas uv `o8.xy = texCoord.xy + canvasTex2(o4).xy`**: `canvasTex2` is a per-cell uv shift, zero
   until a page writes one, so by default each cell shows the atlas triangle the mesh gave it.
5. Six more fetches around the vertex, `canvasTex(o4 + sampleOffsets[6 * offsetIndices.x + k])` for
   `k = 0..5` (the index is built as `offsetIndices.x * 6 + {3, 1, 4, 2, 5, 0}` from the literals).
   Their alphas give an occlusion term **`s = (1/6) * sum_k max(alpha_k - h, 0)`** (confidence 60: the
   register allocation of the six fetches was followed by hand, one chain, not executed).
6. **Vertex colour `= ((1 - s) * canvasTex(o4).rgb, 1)`**, out as the varying `o4..o7`.
7. `gl_Position = worldViewProj * (o2.x, o2.y, h, 1)`.

**The six offsets per corner** (`DAT_81732994..`, set in `Canvas3D_InitStatics`, 72 floats; shown in
cell-texel units, `u` in 1/512 and `v` in 1/128). They are the cells that share the vertex:

| `offsetIndices.x` | the six `(du, dv)` |
| --- | --- |
| 0 | (0,0) (-1,0) (-2,0) (0,1) (-1,1) (-2,1) |
| 1 | (0,0) (1,0) (2,0) (0,1) (1,1) (2,1) |
| 2 | (-1,0) (0,0) (1,0) (-1,-1) (0,-1) (1,-1) |
| 3 | (0,0) (-1,0) (-2,0) (0,-1) (-1,-1) (-2,-1) |
| 4 | (-1,0) (0,0) (1,0) (-1,1) (0,1) (1,1) |
| 5 | (0,0) (1,0) (2,0) (0,-1) (1,-1) (2,-1) |

**Fragment program (confidence 70; the register roles `pa0` = interpolated vertex colour and `pa2` =
the `tex` texel are inferred from the order of the varyings, not measured).** With `c` the vertex
colour and `t` the sample of `tex` at `o8`:

```text
out.rgb = t.rgb * ((1 - t.a) * min(c.rgb + 0.005, 1) + t.a)        out.a = 1
```

where `0.005` is the literal pair `(0x1d1f, 0x1d1f)` and `1` the constant ROM's `c15.y`. The program
declares one sampler, `tex`, bound by `Canvas3D_Draw` as fragment texture 0 to the canvas atlas
resource: **`DAT_81732674` is loaded from `"Data/FE/NewImages/canvasTexture.gxt"` in `FUN_81071300`**
(the single caller of the mesh builder, `0x81071300`), and `DAT_81732678` is written only by the
HD / Fury campaign button builders; the draw uses `...678` when it is loaded and `...674` otherwise,
so the default backdrop is `canvasTexture.gxt`, and the two cell textures as **vertex** textures 0 and 1.

### The lattice law (what is drawn)

**The lines are not computed. They are pixels of the canvas atlas.** The default cell is white with
alpha 0 (`canvasTex` is filled `0xffffff`), so `h = posOffset.z`, `s = 0`, the vertex colour is white
and `c' = 1`: the pixel is `t.rgb`, the atlas texel, whatever the line width or grey. The two UV
triangles above cover the top-left of `canvasTexture.gxt` (2048 x 2048, PVRTC-II, base
`data/FE/NewImages/`), where the sheet holds a pair of outlined equilateral triangles (apex up and apex
down, side 93 to 95 px) drawn in light grey on white:

- measured in the decoded sheet: the line **core is grey 222 (RGB 222, 222, 222, alpha 0), about 2 px
  wide, with 4 to 5 px of falloff** (234, 242, 249 at 1 px steps), on white (255, 255, 255, alpha 0)
  inside; outside the lines alpha is 0 to 17 on a 255 base, so `t.a` is below 7 % and the blend term
  above is within 7 % of 1 times white-ish rgb;
- the captures' **line grey 233 to 245 on 255** is lighter than the sheet's 222 core, which a
  minified draw explains (the atlas triangle is 93 to 95 px on a side and the screen cell about
  55 px, so a 2 px core is about a pixel and mixes with the white around it). That is the only
  consistency check between the sheet and the captures, and it is weak: a pixel comparison on a ported
  draw is the proof. (The period and side in the captures come from the mesh and the camera, not
  from the sheet, so they are not evidence for it.)

So **the port draws the mesh above with `tex = canvasTexture.gxt`** (`canvasTextureHD.gxt` where the HD resource is loaded; **its top-left carries the same pair,
bit for bit in the measured 170 x 105 px region**, so the lattice is the same on either sheet) at the uv triangles above, vertex colour white, alpha 1, behind everything,
and takes the line grey and width from the sheet. **No constant is invented and no stand-in is
needed.** The earlier note that "no disc texture is the lattice" looked at whole entries; the
lattice is a 94 x 82 px region of one sheet.

**Camera and offsets, read 2026-10-27** (item reset `FUN_81070c0c`, confidence 70): `posOffset` (item
`+0xc4..+0xcc`) = (0, 0, 0), `texCoord2offset` (`+0xd0`, `+0xd4`) = (0, 0), camera fields
`+0xb8` = 27.1627 (`0x41d94d21`; this page first wrote 27.1976, an arithmetic slip), `+0xbc` = 0.39549 (`0x3eca81dc`), `+0xc0` = 8.0 (`0x41000000`), and
two floats at `+0x5ed18` = 5.0, `+0x5ed1c` = 17.0 (probably zoom limits, unread). The view is an
orthographic window of half-width `+0xc0` (`0.5555556` of it vertically) around the pan, so the
default shows 16 units across: **60 px per unit at 960 px, against the captures' 55.2** (side 55.2
and period 47.8 px, an effective half-width of about 8.7). That is an 8 % difference the defaults do
not explain; either a page changes `+0xc0` (it sits inside 5 to 17) or the viewport is not the full
width. Not resolved here and not to be fitted: the wire lane finds the writer of `+0xc0` first.

What stays open:

1. The writers of the camera fields after the reset (the `TouchScroll` view, a page zoom): they
   decide the phase and the 8 % scale difference above.
2. The skirt triangles (vertices 3 to 5 at `z = -1`) shade to `(1 - s)` with `s` up to 1 (black) when
   their `h` is below their neighbours' alphas; the reference frames show no such triangle, which
   fits an orthographic view straight down the `z` axis (a vertical wall has no area). Draw the top
   triangles only unless a camera tilt is found.
3. Painted cells (`canvasTex` alpha above 0, `canvasTex2` uv shifts): written by the campaign labels,
   not part of the default backdrop. The cell textures are 512 x 128, linear, `0xffffff` per
   texel in `A8B8G8R8` order, so default alpha is 0 and the default `canvasTex2` shift is 0.
4. Who sets bit `0x80` at `canvas+0x30` (which pages draw the lattice).

Confidences: the register map 85; the vertex program's per-instruction listing 85, its reading as an
occlusion shade 60; the fragment formula 70; the lattice-from-atlas law 85 (the texture load is read,
the sheet measured on both atlases, the uv-to-triangle correspondence a decompile); the camera
defaults 70; the screen grey consistency is arithmetic, not a capture comparison.

**The 2048 lattice thread is ready to wire** (a ready-to-wire thread in CLAUDE.md's sense) on the law,
the mesh, the uvs and the grey, with the camera's runtime writer as its first step.

### When it draws (confidence 70)

`FUN_8103da80` (the per-frame render thread loop, one call site of `Canvas3D_Draw` at
`0x8103db70`) runs `Canvas3D_Draw(DAT_8151d3f4, ...)` early in each frame, after the clear and before
the front-end item pass, whenever the singleton exists and `*(uint *)(canvas + 0x30) & 0x80` is set.
It is not tied to a page: `BootFlowCanvas_Item`'s own draw slot is a bare `bx lr` (`0x813c668c`) and
`ScreenClear_Item` is unused by the shipped pages. So the lattice is a persistent layer behind the
front end, which matches "every page" in the captures. What sets bit `0x80`, and which pages clear
it (the race HUD, the title movie), is unread.

## Camera writers and the mesh window (read 2026-10-27, lane `2048-lattice`)

Confidence 75 on the readings, 60 on what they say about the page's actual camera.

| Address | What | Conf |
| --- | --- | --- |
| `0x81070c0c` | The item reset: pan `+0xb8/+0xbc` = 27.1627 / 0.39552, half-width `+0xc0` = 8.0, limits `+0x5ed18/1c` = 5.0 / 17.0 (read earlier), then `Canvas3D_BuildWorldViewProj` and `FUN_8107369a`. | 75 |
| `0x81072220` | Per-frame item update. The **pan writer**: a touch drag (`+0x5ed10/14` is the last touch) subtracts `delta * (+0x5e0a8/ac)` from `+0xb8` and adds it to `+0xbc`. No drag, no write. | 70 |
| `0x810f285c`, `0x81101c34` | The **half-width writers**: a two-finger pinch adds `-(finger distance now - before) * 0.03` to `+0xc0` and clamps it to `[+0x5ed18, +0x5ed1c]` = 5.0 to 17.0. Both are the campaign event-card pages' touch code. | 75 |
| `0x8107369a` | After every camera change: unprojects the screen centre and corner through the inverse of `+0x5ec90`, then sets **where the 90 x 32 mesh window sits**: `+0xc4/+0xc8` (`posOffset`) = `(col0 * 0.5, row0 * 0.866)`, `+0xd0/+0xd4` (`texCoord2offset`) = `(col0, row0)` cell texels, with `col0, row0` the window's first cell, centred on the camera (`- 22.5` and `- 13.86`, half the mesh's 45 x 16 units) and nudged so that `col0 + row0` is even. | 70 |

So the mesh is a window over an unbounded lattice that follows the camera: the picture is the plane
lattice, whatever the pan, and **a page nobody touched sits on the reset camera**. No code outside the
touch paths writes the camera. The first reading above ("the default shows only a third of the mesh")
was wrong for that reason: the mesh moves with the camera.

The cell type is read: the window's first cell has `col0 + row0` even and takes the apex-up atlas
triangle, and the type alternates per column and per row, so **a cell is apex-up when `col + row` is
even**.

### Built, not merged (2026-10-27)

The draw below was built and measured, then held back: it scored further from the captures than
plain white in every empty area measured (the scale and the line grey below). It is described here as
built so the next attempt starts from it.


`oag_ui::frontend::canvas_lattice`: `bake` renders one tile (one unit wide, two rows tall, 112 x 194
texels, **chosen** resolution) off the atlas's decoded RGBA by the mesh's own corner-to-atlas-triangle
carry, `Frontend::canvas_lattice` tiles it over the screen with the reset camera (`x` scale `960 / 16`
= 60 px per unit, `y` scale `544 / (16 * 5/9)` = 61.2), and `oag_game::boot::sprites` bakes the tile
into the sprite sheet at load (`TILE`), the atlas itself never entering the sheet. Drawn on the title
screen and the game mode page only: those are the 2048 pages the reference captures show with the sharp
lattice. **The home, team, extras, options and profile pages show a blurred ship and a blurred pattern
instead**, which is another effect (unread; possibly a blur of this canvas).

Measured against the reference captures (960 x 544, title screen box x 20..110, y 20..430):

| | reference | ours |
| --- | --- | --- |
| horizontal line period | 47.75 px | 53.0 px |
| line nearest the centre | row 272 | row 275 |
| darkest line grey (of 255) | 233 | 202 |
| line ink (sum of 255 - v across one line) | about 30 | about 83 |

**The claim does not come out true at the reset camera.** The period is the defaults' 53 px (the
`y` window is `5/9` of the width, so 61.2 px per unit, times 0.866) against the capture's 47.75 px, an
unexplained 10 %: the capture's lattice is about 55.2 px per unit both ways, which would need a
half-width of about 8.7 and a window height of 0.567 of it, not the `0.5556` the code carries. Nothing
that writes `+0xc0` or the aspect constant runs without a pinch, so the cause is either a reading of
the projection matrix (`+0x5ebd0..+0x5ec10` come from `FUN_812e4a04`, unread) or the capture's own
state (Vita3K touch emulation). Not fitted. The phase at the centre is 3 px off (2.3 px predicted from
the pan), inside the 2 px bar only if the scale were right. The lines are lighter in the capture
(core 233 to 240, ink a third of ours): the sampling filter of the original's draw (Vita3K's) is not
known, and the bake's bilinear tap is a choice. Edge NCC against the capture is about 0 at the 10 % scale
gap, so no shift was measurable.

What stays open: the projection's own matrix (the 10 %), the blur behind the home and team pages, the
second cell texture's writers (painted cells, the campaign map's lattice), and **who sets bit `0x80`
at `canvas+0x30`** (`FUN_8103da80` draws the canvas only when it is set; nothing in the camera sweep
writes it, so the page set above is read off the captures, not off that bit).

## The camera is tilted: the 10 % is read (2026-10-27, lane `lattice-scale`)

**`FUN_812e4a04` (`0x812e4a04`) builds a rotation from three Euler angles; it is not a viewport
(confidence 85).** It runs the three trig helpers (`0x813efb84`, `0x813efa04`, `0x813ef878`) on its
`s0..s2` and multiplies the three single-axis matrices. `Canvas3D_BuildWorldViewProj` (`0x8107086c`)
calls it with `s0 = +0xac + +0xd8`, `s1 = +0xb0 + +0xdc`, `s2 = +0xb4 + +0xe0`, and the item reset
(`0x81070c0c`) stores `+0xac` = `0xbee66666` = **-0.45 rad** (25.8 degrees), `+0xb0` = `0xbdcccccd` =
**-0.1 rad** (5.7 degrees), `+0xb4` = 0, `+0xd8..+0xe0` = 0. The page's earlier "straight down the `z`
axis" reading was wrong: **the canvas camera is orthographic and tilted.**

**The matrix, executed rather than traced (confidence 80).** Unicorn ran `0x8107086c` alone (patch
`eboot.elf` PT_LOAD segments mapped, VFP on, item buffer filled with the reset values, `0x814e7e50`'s
identity as the starting matrices; one CPU function, not a game emulator) and stopped at `0x81070c00`.
Results, row-vector convention (`p' = p * M`):

| Matrix | Rows |
| --- | --- |
| `+0x5ebd0` (view) | `diag(1, -1, 1)`, translation row `(-27.1627, -0.39552, -8.0)`: the pan, and `-half-width` as a `z` offset |
| `+0x5ec10` (rotation) | `(0.995, 0, 0.0998)`, `(0.0434, 0.9004, -0.4328)`, `(-0.0899, 0.4350, 0.8960)` = `Ry(-0.1) * Rx(-0.45)` |
| `+0x5ec50` (ortho) | `x` 0.125 (= 1/8), `y` 0.225 (= 1/(8 * 5/9)), `z` -0.002, near/far 0.1 / 1000 |
| `+0x5ec90` (final) | `x' = 0.12438 x - 0.00543 y - 0.01124 z - 3.29062`; `y' = -0.2026 y + 0.09787 z - 0.86307` (`z = 0` plane: `x' * 480 = 59.7 x - 2.61 y + ...`, `y' * 272 = -55.1 y + ...`) |

So the lattice's screen scale is **59.7 px per unit along `x` and 55.1 along `y`** (60 x `cos 0.1`, and
61.2 x `cos 0.45`), a **row period of 0.866 x 55.1 = 47.72 px against the capture's 47.75**, and a
**shear of 2.6 px per unit of `y`**. The capture agrees on every count: row spacing 47.75 px (rows
128, 176, 224, 272, 319, ...), diagonal crossings 29.85 px apart on one row (period 59.7). With the
executed matrix the lattice lines correlate with the capture at edge NCC 0.87 (game mode left box), 0.88
(title left box) after a world-plane offset: the scale, the shear and the line angles are no longer
open.

**Still open, not fitted into any draw:**

1. **A residual phase.** The lattice with the executed matrix lands in the capture only after a
   world-plane offset of about `(0.40, 0.49)` (screen: 22.6 px right, 27 px up; the same on the title
   and the game mode pair, so it is not a touch drag in one walk). The 3 px "centre phase" gap noted
   above came from the wrong scale; this one is the real one. Candidates not yet separated: the
   mesh's own placement (`posOffset` and the cell origin the builder applies, with the `col0 + row0`
   parity nudge), the unread writers of the pan that run on the page load, and a lattice point that
   the vertex program's `h = position.z + canvasTex.a` shifts. Confidence the lattice itself is right
   (period, angles, shear): 85; the offset is a measurement, 70.
2. **The ink.** Our atlas-sampled lines carry about 2.1 times the capture's ink (measured across the
   horizontal line at row 272, columns 20..200: capture 253 -> 240/241 over two rows, simulated 251 ->
   227/228 over three). A numpy render of the lattice (executed matrix, offset above, atlas levels
   decoded from the file) scored **mean absolute grey difference against the capture** in the empty
   boxes (plain white first):

   | Box | white | lattice, ink as authored | lattice, ink x 0.5 |
   | --- | --- | --- | --- |
   | title (0, 20, 960 x 110) | 2.43 | 2.51 | 0.54 |
   | title (0, 515, 960 x 29) | 1.68 | 1.76 | 0.52 |
   | title (0, 140, 110 x 200) | 2.20 | 2.31 | 0.50 |
   | game mode (0, 100, 215 x 440) | 2.25 | 2.37 | 0.53 |
   | game mode (740, 100, 220 x 440) | 2.22 | 2.35 | 0.53 |

   Fitting a single ink factor on the game mode box gives 0.48 at mip level 0 (mean difference 0.33),
   0.52 at level 0.78 (0.51), 0.56 at level 1.5 (1.15): **the factor is about one half whichever
   filter, and level 0 fits best**. It is `lattice = 0.5 * white + 0.5 * atlas`, which looks like a
   translucent white layer over the canvas (a page background item or the draw's own blend), **not a
   sampling filter**; no part of the shader read above halves the ink (the fragment program is
   `t.rgb * (...)` with `t.a` under 7 %). The blend source is unread (confidence 40). The atlas
   itself has ten mip levels (`canvasTexture.gxt`, 2048 x 2048, header byte `+0x38` = 10, the file size
   matches the full chain), and the authored levels 1 to 3 carry per-line ink 76, 41, 30 against
   level 0's 99 (atlas texels, across the base edge of the apex-up triangle; PVRTC noise included), so
   filtering alone cannot make the 0.4 to 0.5 ratio either.

**What this means for the draw.** The branch's tile draw is the wrong projection: it assumed an
axis-aligned scale (60 x 61.2) and no shear. A draw from the executed matrix needs the lattice sheared
(per row band, or a per-pixel inverse projection baked at load) and the two open items above. With
both measured values fed in, the simulated draw beats white in all five boxes (0.5 against 1.7 to 2.4);
without the ink factor it loses to white (2.3 to 2.6), which is why it stays held back: the offset and
the 0.5 are measurements of the capture, not recovered laws.

Confidences: `FUN_812e4a04` is a three-angle Euler rotation builder, 85; the tilt angles, 90 (stored
constants, decoded); the matrix values, 80 (an executed function in isolation, the item fields set from
the reset); the `Ry * Rx` order, 80 (the executed matrix); the screen scale and period, 85 (matches the
capture on three counts); the residual offset, 70; the 0.5 ink factor, 60 as a number and 40 as
"a translucent white layer".

## Phase and grey: every static path reads zero (2026-10-27, lane `lattice-wire`)

The capture's lattice sits at a world offset of **(0.40, 0.50)** from the executed matrix (re-fitted on
a 0.01-unit grid: edge NCC 0.938 title top box, 0.934 title left, 0.884 and 0.938 game mode left and
right, all at the same offset; the title's bottom strip gives (0.90, 1.36), the same offset plus the
lattice vector (0.5, 0.866)). Screen: 22.6 px right, 27.6 px down. **Nothing read below produces
it.** Each candidate, with where it was ruled out:

| Candidate | Read at | Result | Conf |
| --- | --- | --- | --- |
| Mesh window | `Canvas3D_PlaceMeshWindow` `0x8107369a` | `posOffset = (col0 * 0.5, row0 * 0.866)`, `posOffset.z = 0`, `texCoord2offset = (col0/512, row0/128)`; the nudge makes `col0 + row0` even, so the offset is always a lattice vector: no phase | 80 |
| Mesh parity | `Canvas3D_BuildLatticeMesh` `0x81073cb8` | `bVar6` starts true, toggles per column and once per row (`0x81073cb8` lines at the loop ends): apex-up iff `col + row` even, as modelled | 85 |
| Camera reset | `Canvas3D_ResetCamera` `0x81070c0c` (disassembled) | the values above, `+0xa0..+0xa8 = 0`, then zeroes the parent TouchScroll's position and the canvas's last-touch fields `+0x5ed10/+0x5ed14` | 85 |
| Stick tilt | `Canvas3D_Update` `0x81072220` | every frame `+0xd8 += 1.8 dt (0.2 * pad[+0x5c60] - +0xd8)`, `+0xdc += 1.8 dt (-0.2 * pad[+0x5c5c] - +0xdc)`, `+0xe0` decays; `pad = 0x8122d664(DAT_818b9ea8, 0)`, the fields are the deadzone-clamped analog stick (`0x8122c20c`, `0x8122c6ca..`). A stick parallax, zero at rest. Fitting both extra angles against the capture with no offset reaches NCC 0.54 (against 0.938 for the plain offset), and an `x` tilt would move the 47.75 px row period: ruled out | 75 |
| Pan by scroll | `Canvas3D_Update` and `TouchScroll_Update` `0x81103b46` | the pan follows the parent TouchScroll's position (`+0xa0/+0xa4`, getters at vtable `+0x5c/+0x60` = `0x813d0078`, `0x813d0080`): `pan.x -= (dx / 960) * span.x`, `pan.y += (dy / 544) * span.y`, with `span` = (16.5112, 9.8716) (`+0x5e0a8/ac`, Unicorn run of `0x8107086c` then `0x8107369a`). The canvas's parent is `FeShellTouchScroll` (`viewWidth 960`, `MaxScrollX 960`, `maxscrolly 544`); `TouchScroll_ParseValues` `0x811035a8` compares names with `SceLibc_184C4B07` = **`strcasecmp`** (vitasdk NID db), so the lower-case spellings parse, the range equals the view, and `TouchScroll_Update` pins the position at (0, 0). A 272 px centring (the case-sensitive reading) would land 6 px off anyway. **Struck 2026-10-27 (`lattice-live-read`): the live scroll position at the title is (-3000, -1500), not (0, 0); see the next section** | 80, wrong |
| Taps | `scripts/vita3k-2048-menu-pair.py` `PLAN` | the title and game mode shots are taken before the walk's first tap (keys only), so no drag moved the pan | 90 |
| Cell height | `Canvas3D_Ctor` `0x81070da0` | `canvasTex` is `U8U8U8U8_ABGR` (`0x0c000000`) filled `0x00ffffff`: alpha 0, `h = 0`; `canvasTex2` (`0x1b000000`, 8 bytes a texel) is zeroed: no uv shift. `Canvas3D_UpdateCellHeights` `0x81071794` raises cells only in a ripple (`+0x5ed24`) or around a music-spectrum centre (`+0x5ed40`, `DAT_818c3d58`), radial, not uniform | 70 |
| Viewport | `sceGxmSetViewport` (`0x813f2340`), callers `0x81036462`, `0x8103659e`, `0x81036670`, `0x8103f11e`, `0x8103f5a4` | all `(x0 + w/2, w/2, y0 + h/2, -h/2)` with `x0 = y0 = 0` on one player; the scene target `DAT_816ae82c` is the display size. Flipping either axis in the fit drops NCC to 0.34 or keeps an offset | 80 |

**Grey.** `Canvas3D_Draw` selects its fragment program from `DAT_8151d408 + iVar9` with
`iVar9` in {0, 8, 16, 24, 32, 40} (format and MSAA variants); the blended variants are at
`+0x40c`, `+0x414`, ... (`Canvas3D_LoadShaders` creates them with a blend info `0x0f, 0x11, 0x54, 0x54`)
and the draw never picks them: **the canvas draws opaque** (confidence 85). `Canvas3D_Draw` clears colour
to 0 and depth to 1 first (`0x812f60c2`), sets depth func `LESS_EQUAL` (`0xc00000`). The
`GameModeChoice` page XML has no full-screen item over it, and the parent TouchScroll's draw
(`0x811038da`) draws only its scroll bars. So nothing read halves the ink either.

**What a draw from the read state scores** (numpy render, executed matrix, atlas level 0, mean absolute
grey difference in the five boxes, white first: 2.43, 1.68, 2.20, 2.25, 2.22):

| Draw | title top | title bottom | title left | game left | game right |
| --- | --- | --- | --- | --- | --- |
| read state (offset 0, ink as authored) | 6.08 | 6.43 | 6.12 | 6.24 | 6.22 |
| offset 0, ink x 0.5 | 4.08 | 3.87 | 3.99 | 4.07 | 4.04 |
| offset (0.40, 0.50), ink as authored | 2.76 | 1.82 | 2.54 | 2.58 | 2.56 |
| offset (0.40, 0.50), ink x 0.5 | 0.92 | 0.57 | 0.83 | 0.87 | 0.86 |

So the draw stays held back: from the code it is 2.5 to 4 times further from the capture than white.
**The two numbers are a property of the captured run, not of any path read statically.** The next
step needs a live readout from a run of the original, which this lane was not allowed: at the title,
read the canvas singleton `*(0x8151d3f4)` fields `+0xac..+0xe0` (angles, stick offsets, pan, half-width,
`posOffset`), `+0x5ec90` (the uploaded matrix), the parent TouchScroll's `+0xa0/+0xa4`, and one
`canvasTex` texel (`*(0x8151d3ec)`). If those equal the reset values, the offset and the half ink come
from the emulator's draw (a Vita3K vertex-texture or sampling difference) and the reset camera with the
authored ink is the faithful draw; if they differ, the writer is found by a watchpoint on the field
that moved.

Confidences: each row as listed; "no static path gives the phase" 75 (the music and ripple height paths
were read, not executed); the TouchScroll chain 80.

| Address | Name | Conf |
| --- | --- | --- |
| `0x81070c0c` | `Canvas3D_ResetCamera` | 85 |
| `0x8107369a` | `Canvas3D_PlaceMeshWindow` | 80 |
| `0x81072220` | `Canvas3D_Update` | 70 |
| `0x81071794` | `Canvas3D_UpdateCellHeights` | 60 |
| `0x813cff48` | `TouchScroll_Create` | 80 |
| `0x811035a8` | `TouchScroll_ParseValues` | 85 |
| `0x81103b46` | `TouchScroll_Update` | 70 |
| `0x811042ae` | `TouchScroll_SetScroll` | 65 |

Omega: **checked, applies, not wired.** `eboot.bin` carries `TouchScroll_Item.cpp`, `FeShellTouchScroll`,
`MaxScrollY` and `autoScrollSpeed`: the same scroll item and the same shell parent ship; the GCN draw and
the update were not read.

## The live read at the title: the phase is the game's state, the ink is not (2026-10-27, lane `lattice-live-read`)

Vita3K `0.2.1 4095` (headless, muted, a private pref copy of the pair profile), EU v1.04, read through
`/proc/<pid>/mem` of a Vita3K started as a child (`scripts/vita3k-mem.py`, guest = host - `0x400000000`),
the guest stopped with `SIGSTOP` for each read. The state is the settled title ("PRESS ANY BUTTON TO
START", before any key). The claim was falsifiable: had every field equalled the reset values, the offset
and the ink would have been Vita3K's drawing.

**Canvas singleton `*(0x8151d3f4)` = `0x85d69510`** (confidence 85, two boots, identical):

| Field | Live | Reset (`Canvas3D_ResetCamera`) |
| --- | --- | --- |
| `+0xac`, `+0xb0`, `+0xb4` (angles) | -0.45, -0.1, 0 | same |
| `+0xd8..+0xe0` (stick tilt) | 0, 0, 0 | same |
| `+0xb8`, `+0xbc` (pan) | **78.76, -26.824** | 27.1627, 0.3955 |
| `+0xc0` (half-width) | 8.0 | 8.0 |
| `+0xc4`, `+0xc8` (`posOffset`) | 55.0, 8.66025 (a lattice vector: 110 x 0.5, 10 x 0.866) | window placement |
| `+0xd0`, `+0xd4` (`texCoord2offset`) | 110/512, 10/128 | same law |
| `+0x5ec90` (matrix, translation row) | (-9.56032, 4.65163, -0.94692) | (-3.29062, -0.86307, -0.98078) |
| `+0x5ed10`, `+0x5ed14` (last scroll) | -3000, -1500 | 0, 0 |
| `+0x30` | `0x0000f086` (bit `0x80` set: the canvas draws) | |

The rotation and ortho rows equal the executed ones, so the matrix differs only by the pan.

**Parent `FeShellTouchScroll`** (`canvas+8` = `0x820c9e20`, vtable `0x8150ecf0`, getters at `+0x5c/+0x60`
= `0x813d0078`/`0x813d0080`): **`+0xa0/+0xa4` = (-3000, -1500)**, velocity `+0xa8/+0xac` = 0, view `+0xd4/+0xd8` =
960 x 544, content bounds (`+0xb4..+0xc0`) (-103.4, 5123.3, 1495.3, 4386.1). Sampled every 0.1 s from the launch
on two boots: the canvas exists at about 2.2 s with pan (27.1627, 0.3955) and scroll (0, 0), and **in one
step at about 5.2 s** (about 3 s after the canvas, before any key press) the scroll is (-3000, -1500) and the
pan (78.76, -26.824) in the same sample. A third boot on an empty `PCSF00007` save gave the same values, so
the value does not depend on campaign progress.

**The phase is explained, by arithmetic and not by a fit (confidence 85).** `Canvas3D_Update`
(`0x81072220`) moves the pan by the scroll delta from the last scroll it saw (the reset zeroed it):
`pan.x -= (dx / 960) * 16.5112` and `pan.y += (dy / 544) * 9.8716` with `d` = (-3000, -1500) gives
27.1627 + 51.5975 = 78.7602 and 0.3955 - 27.2195 = -26.824, to every printed digit. The world offset
between the live and the reset matrix solves `A * delta = t_live - t_reset` on the `z = 0` plane
(`A` = the 2 x 2 of `+0x5ec90`): `delta` = (-51.597, -27.2195). Reduced by the lattice vectors (1, 0) and
(0.5, 0.866) that is **(0.4027, 0.4933)**; the lane `lattice-wire` fitted **(0.40, 0.50)** to the capture.
Feeding the live matrix to the same numpy render with no offset, the mean grey difference to the
`2048/menu-title` and `menu-game-mode` captures in the five boxes, as authored and at ink x 0.5:

| Box | white | live matrix, ink 1.0 | live matrix, ink 0.5 |
| --- | --- | --- | --- |
| title top (0, 20, 960 x 110) | 2.43 | 2.61 | 0.65 |
| title bottom (0, 515, 960 x 29) | 1.68 | 1.80 | 0.57 |
| title left (0, 140, 110 x 200) | 2.20 | 2.35 | 0.58 |
| game mode left (0, 100, 215 x 440) | 2.25 | 2.40 | 0.61 |
| game mode right (740, 100, 220 x 440) | 2.22 | 2.39 | 0.62 |

The offset-0 rows of the table in "Phase and grey" were the reset camera; the live camera needs no offset.

**Who writes (-3000, -1500): not pinned (confidence 55 on the candidates).** It is not a literal (no
`0xc53b8000` or `0xc4bb8000` word in the patch `eboot.elf`) and not `Canvas3D_ResetCamera` (which writes
(0, 0) through `TouchScroll_SetScroll` `0x811042ae`, `pos = (p3 - p1, p4 - p2)`). Every caller of
`TouchScroll_SetScroll` other than the reset passes zeros except two paths of the campaign map page,
which centre the current event's node on the screen: `FUN_810efc4a` (call `0x810effcc`) and
`FUN_810efff6` (call `0x810f032a`) both compute the node's screen position through the canvas matrix
(`+0x5ec90`..`+0x5ecc0`, node fields `+0x2c4/+0x2c8/+0x314/+0x318`, default node `"2048 - Event 1"`) and
call `SetScroll(sx, sy, 480.0, 272.0)`, i.e. a scroll of `(480 - sx, 272 - sy)`. -3000 and -1500 are round
for a projected node and the bounds above do not make them a clamp; which call ran, and why the numbers are
round, is open. So the lattice's phase at the title is **the campaign map's centring of the first event**,
state the page code computes, not a Vita3K artefact and not the reset camera.

**The ink is not explained (confidence 70 that no game state read causes it).** Read at the same title:

- The atlas sampled by the draw: `DAT_81732674` is a resource record (`0x829b8c30`); 16-byte
  `SceGxmTexture` descriptors with data pointer `0x71492020` exist in dozens of command-buffer copies, and
  **the 2,796,208 bytes at `0x71492020` are identical to `canvasTexture.gxt`'s data (offset `0x40`)**. The texture
  the game hands to the GPU is the disc's, unchanged.
- `canvasTex` (`*(0x8151d3ec)` = `0x70bc4410`): over the whole 512 x 128 texture two texels only,
  `ff ff ff 00` (48,120) and `00 00 00 00` (17,416); the 90 x 32 window is all `ff ff ff 00`. Alpha 0, so
  `h = 0`, `s = 0`, vertex colour white. `canvasTex2` (`*(0x8151d3f0)`) is 65,536 zero texels.
- The draw is opaque (the static read above), and nothing else moved: the live matrix with the authored ink
  loses to white (table above).

So the half ink is not in the game's state and not a translucent layer the canvas item holds. What remains
is outside the memory read: Vita3K's host-side texture decode or filtering of the PVRTC-II atlas (the file
header says `0x83000000`, `PVRTII4BPP`; the descriptor word 1 reads `0x82721600`, whose packing was not decoded
and is a lead), or a layer drawn by another item after the canvas. By elimination only: the GPU copy of the
texture was not reachable. A pair judged on this basis is a question for the maintainer (the lead asks).

Omega: **checked, applies** for the scroll law (the same `TouchScroll_Item.cpp` and shell parent ship); the live
value is **not checkable** (shadPS4 has no memory readout), and its campaign-map centring was not read.
HD: **checked, differs** (no `Canvas3D`).


## HD / Omega cross-check (CLAUDE.md)

- **Omega: checked, applies, not wired. The GCN program is the open half.** `canvasTexture.gnf` ships in `data00.psarc` (4,194,560 bytes, `Data/fe/NewImages/`) and again in the patch's `data08.psarc` (5,597,440 bytes); its triangle region was not decoded. The PS4 `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`)
  carries `Canvas3D_fp`/`Canvas3D_vp` (`0x0181aa64`, `0x0181aa70`), `3dCanvas_Item.cpp`
  (`0x0181aa22`) and `FE3DCanvas` (`0x0181aa59`): the same item ships, so Omega's 2048 campaign
  branch draws the same mesh. Omega's HD-derived pages sit on the same shell. The USSE decoder does not apply to Omega's shaders (GCN `.sb`, a different instruction set:
  **checked, differs**), so Omega's `Canvas3D` programs are unread; the mesh and the atlas law above
  are the same item's CPU side and sprite, and the atlas the same sheet's PS4 `.gnf` sibling (unchecked).
  Not compared with a
  capture (shadPS4 race shots are untrusted per the lane notes; menu pairs were not taken).
- **Omega camera: checked, applies (2026-10-27).** The reset pair `-0.45`, `-0.1` (`0xbee66666`,
  `0xbdcccccd`) sits together at `0x018086ec` in `eboot.bin` (one hit for each pattern), next to the
  `Canvas3D` strings' code, so the tilted camera is the same item's. The GCN draw and the matrix code
  were not read.
- **HD: checked, differs.** `/hdfury/EBOOT-ps3-hdfury-eu.elf` has no `Canvas3D` string; HD's
  front-end backdrop is not this mesh.

## Data

| Address | Name | Conf |
| --- | --- | --- |
| `0x812e4a04` | `Math_BuildEulerRotation` | 85 |
| `0x8151d3f4` | `g_pCanvas3DInstance` | 70 |

## The atlas descriptor decoded: the format is the file's, the ink is not in it (2026-10-27)

Read live (Vita3K 0.2.1, headless, title screen), every 16-byte `SceGxmTexture` whose data word is
`0x71492020`: 606 copies `82721600 030b000b 71492020 80000000` and 139 with control word 0 `82721601`
(bit 0 differs, purpose unread). One more, `40001000 828de66c 71492020 402aaab0`, is not a texture
descriptor of this atlas. Layout read against Vita3K's `SceGxmTexture` (read for behaviour, decode handwritten).
Confidence 85 for the decode, since it matches the file on size and mip count.

- **`0x82721600` is control word 0 (sampler state), not a format word.** The format is
  `base_format` of word 1 (`0x03`, bits 24-28) with `format0` (word 0 bit 31, set): `0x83` = `PVRTII4BPP`, type 0
  (swizzled), swizzle `ABGR` (word 3 bits 12-14 = 0). That is exactly the file header's `0x83000000`, `0x0` type.
  Word 1 low half gives 2048 x 2048 (`2^0xb` both), word 0 `mip_count` 9 (ten levels, the file's `0x0a`).
- Sampler: addressing 0 (repeat) both axes, `mip_filter` 1, `min_filter` 1, `mag_filter` 1 (trilinear),
  `gamma_mode` 0, `lod_bias` raw 19. Vita3K maps raw to `(raw - 31) / 8` = **-1.5** (sharper).
- **So no format or gamma difference exists**, and a texel under the game's format is the file's texel.
  The lattice's own LOD is `log2(~1.72 texels per pixel) = 0.78` (the earlier fit's lod, which is thus derived);
  with the bias, `lod = max(0, 0.78 - 1.5) = 0`: mip 0, ink 1.0.
- Live-matrix sim at lod 0 (offset 0, phase already in the matrix): title boxes 2.61/1.80/2.35
  against white 2.43/1.68/2.20, game mode 2.40/2.39 against 2.25/2.22. **Loses to white.** At ink 0.5 it scores 0.5-0.65.

Verdict: the descriptor does not tell the GPU to read half ink; it says the opposite (mip 0, no blur). The half ink
is still unexplained (confidence 75 it is not in the texture state): what is left is a layer drawn over the canvas by another item,
or the pass's output blend, neither read. Not wired. Omega: canvasTexture.gnf, **not checkable** (no readout), the same
item ships.

## Wired (2026-10-27, lane `lattice-final`): full ink as read; the capture shows about half, see part 2

The maintainer's decision: draw the lattice now at the texture's own ink. `oag_ui::frontend::canvas_lattice`
bakes **one 960 x 544 picture** at load (the camera is fixed on these pages): for each screen pixel centre it
inverts the tilted matrix back to the lattice plane (`x' = cos(0.1)/8 * x - sin(0.1) sin(0.45)/8 * y + t0`,
`y' = -cos(0.45)/(8 * 5/9) * y + t1`, translation row `(-9.56032, 4.65163)` read live at the title), picks the
cell and its atlas triangle (the branch's `atlas_at`) and reads `canvasTexture.gxt` at mip 0 bilinear. It is
inserted over the white clear on the title screen and the game mode page only (`wipeout2048.rs`). The branch
`2048-lattice`'s axis-aligned tile draw is gone. Scale (59.7 x 55.1 px/unit, row period 47.72), shear and the
phase come from the matrix, not a fit; a unit test pins the scales.

Measured against the Vita3K captures (`2048/menu-title` via `--until TitleScreen --press start
--press-until-reached --ticks 424`, `2048/menu-game-mode` via `--press start,cross,cross,cross --original-menu`),
960 x 544:

| Measure | Title | Game mode |
| --- | --- | --- |
| Best shift of the edge map (ours against capture, empty box, +-6 px) | (0, 0) | (0, 0) |
| Edge NCC in the empty box at shift 0 | 0.939 | 0.937 |
| Line profile across row 271/272, columns 20..200: capture / ours | 253 -> 240 / 253 -> 220 | same |
| Mean grey difference, white plate before / now, empty boxes | 2.43/2.52, 1.68/1.70, 2.20/2.25 | 2.25/2.31, 2.22/2.32 |
| Whole frame mean abs grey difference (ours with lattice) | 5.39 | 7.68 |

The period, phase and line angles land (shift 0, row 271/272 on the same rows in both). The ink is the open
part: our lines dip to 220 where the capture's dip to 240, so the empty boxes score 0.04 to 0.10 worse than
plain white, as the thread predicted. Part 2 looks for what halves the ink.

Omega: **checked, applies, not wired.** `canvasTexture.gnf` ships and the reset angles are the same
(`0x018086ec`); Omega's front end is HD's, so whether its pages draw a `Canvas3D` was not read.
HD: **checked, differs** (no `Canvas3D`).
