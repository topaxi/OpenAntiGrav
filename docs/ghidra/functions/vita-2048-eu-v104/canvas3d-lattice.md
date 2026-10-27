# The pale triangle lattice behind every 2048 page is `3dCanvas_Item`'s procedural mesh

Functions in `eboot.elf` (Wipeout 2048, Vita, `PCSF00007` patch v1.04), image base `0x81000000`,
Ghidra program `/2048/eboot-vita-2048-eu-v104.elf`. 2026-10-27, lane `2048-backdrop-re`.

Question: `2048/menu-*` reference captures draw a pale triangle lattice (horizontal-line period about
47.8 px, side about 55 px, 60-degree diagonals, line grey 233-245 on 255) behind every page; ours is
flat white. No disc texture is the lattice (previous lane, base `data.psarc` 722 FE entries) and the
patch archives hold none either: the `data1`/`data2.psarc` `data/FE/` entries are demo upsells, HD
track icons, team-grid HD icons, `canvasTextureHD.gxt` and the `gamemodechoice` pair, none a lattice.
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

### Mesh law (read, confidence 65, one decompile of `0x81073cb8`)

- Grid of 90 columns (`< 0x5a`) by 32 rows (`< 0x20`) of cells; each cell is 6 vertices of 36 bytes
  and 18 indices (six triangles), `bVar6` alternates per column and per row.
- Cell centre: `x = col * 0.5`, `y = row * 0.8660254` (`DAT_81732ab4`, `DAT_81732ab8`). Because the
  row step is sqrt(3)/2 and the half-column step is 0.5, adjacent cells are triangles of side 1.0
  alternating up and down: `DAT_81732c20..` = (-0.5,-0.4330127), (0,+0.4330127), (+0.5,-0.4330127)
  for one parity and `DAT_81732c38..` the mirrored set for the other (`0xbeddb3d7` =
  -0.4330127, `0x3f000000` = 0.5). Those are the three corner offsets added to each cell centre.
- Vertices 3..5 of each cell repeat the corners with a `-1.0` marker in the `offsetIndices` stream,
  and the index list `(0,1,2)(0,2,3)(3,2,5)(0,1,4)(1,2,5)(1,5,4)`-style pairs the inner triangle with
  an outer ring: a filled triangle plus an edge band. Its width is not a constant in this function;
  it comes from the vertex shader through `posOffset`/`sampleOffsets`.

### When it draws (confidence 70)

`FUN_8103da80` (the per-frame render thread loop, one call site of `Canvas3D_Draw` at
`0x8103db70`) runs `Canvas3D_Draw(DAT_8151d3f4, ...)` early in each frame, after the clear and before
the front-end item pass, whenever the singleton exists and `*(uint *)(canvas + 0x30) & 0x80` is set.
It is not tied to a page: `BootFlowCanvas_Item`'s own draw slot is a bare `bx lr` (`0x813c668c`) and
`ScreenClear_Item` is unused by the shipped pages. So the lattice is a persistent layer behind the
front end, which matches "every page" in the captures. What sets bit `0x80`, and which pages clear
it (the race HUD, the title movie), is unread.

## What is not read

1. **The line law.** Whether the 1 px lines are the ring band of the mesh or a fragment-shader edge
   function, their grey and their width are in `Canvas3D_vp` (`0x815270dc`) and `Canvas3D_fp`
   (`0x8152a32c`). `scripts/vita-gxp.py` reads only the GXP container, not the instructions, so
   this stays unread. `White2048` (`0xffe1e4eb`, `NEWGUI/Skin.xml`) is near the measured 233-245 but
   no read path from it to this draw exists: do not adopt it.
2. **The camera.** `Canvas3D_BuildWorldViewProj` takes `+0xb8/+0xbc/+0xc0` of the item (probably pan
   x, pan y, half-width in canvas units). The measured 55.2 px per unit (47.8 / 0.866) implies a
   half-width of about 8.7 units at 960 px, and 5/9 vs 544/960 = 0.5667 shows the aspect is a
   constant, not derived from the display. Default values and whether a page scrolls it (the
   `TouchScroll` view in `newFEshell`) are unread, so lattice phase is unknown.
3. The role of the second 512x128 texture (`canvasTex2`, zero-filled) and the per-cell colour
   texture (`canvasTex`, filled `0xffffff`): cell colours are all white at start; the labels and
   highlight states change them.

## Consequence for the port

No lattice is wired. Drawing the pattern needs the line width and grey (shader) and the camera
default (item fields) and any stand-in would be tuned by eye to the capture, which CLAUDE.md
forbids. `crates/ui` keeps its flat white and this page is the next-address list:

1. Decode the `Canvas3D_vp`/`Canvas3D_fp` instructions (a Vita GXP disassembler is not in
   `scripts/vita-gxp.py`) or capture the lattice with a Vita3K shader dump.
2. Read the item's camera fields (`param_1+0xb8..0xc0`) where the item is created and from the
   `TouchScroll` handler, to get default phase and the scroll law.
3. Read who sets bit `0x80` at `canvas+0x30`.

## HD / Omega cross-check (CLAUDE.md)

- **Omega: checked, applies, not wired.** The PS4 `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`)
  carries `Canvas3D_fp`/`Canvas3D_vp` (`0x0181aa64`, `0x0181aa70`), `3dCanvas_Item.cpp`
  (`0x0181aa22`) and `FE3DCanvas` (`0x0181aa59`): the same item ships, so Omega's 2048 campaign
  branch draws the same mesh. Omega's HD-derived pages sit on the same shell. Not compared with a
  capture (shadPS4 race shots are untrusted per the lane notes; menu pairs were not taken).
- **HD: checked, differs.** `/hdfury/EBOOT-ps3-hdfury-eu.elf` has no `Canvas3D` string; HD's
  front-end backdrop is not this mesh.

## Data

| Address | Name | Conf |
| --- | --- | --- |
| `0x8151d3f4` | `g_pCanvas3DInstance` | 70 |
