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
| `0x8107086c` | `Canvas3D_BuildWorldViewProj` | 55 | Builds the canvas camera matrix (`param_1+0x5ec90`) from the item's camera fields `+0xb8/+0xbc/+0xc0`; the `0.5555556` (5/9) factor is the height/width ratio of the orthographic window. |
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
`+0xb8` = 27.1976 (`0x41d94d21`), `+0xbc` = 0.39549 (`0x3eca81dc`), `+0xc0` = 8.0 (`0x41000000`), and
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

## What is still open

The camera writers (item 1 above), the second cell texture's writers, and the lattice's visibility
bit are the unread parts; the line law is **read**.

## HD / Omega cross-check (CLAUDE.md)

- **Omega: checked, applies, not wired. The GCN program is the open half.** The PS4 `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`)
  carries `Canvas3D_fp`/`Canvas3D_vp` (`0x0181aa64`, `0x0181aa70`), `3dCanvas_Item.cpp`
  (`0x0181aa22`) and `FE3DCanvas` (`0x0181aa59`): the same item ships, so Omega's 2048 campaign
  branch draws the same mesh. Omega's HD-derived pages sit on the same shell. The USSE decoder does not apply to Omega's shaders (GCN `.sb`, a different instruction set:
  **checked, differs**), so Omega's `Canvas3D` programs are unread; the mesh and the atlas law above
  are the same item's CPU side and sprite, and the atlas the same sheet's PS4 `.gnf` sibling (unchecked).
  Not compared with a
  capture (shadPS4 race shots are untrusted per the lane notes; menu pairs were not taken).
- **HD: checked, differs.** `/hdfury/EBOOT-ps3-hdfury-eu.elf` has no `Canvas3D` string; HD's
  front-end backdrop is not this mesh.

## Data

| Address | Name | Conf |
| --- | --- | --- |
| `0x8151d3f4` | `g_pCanvas3DInstance` | 70 |
