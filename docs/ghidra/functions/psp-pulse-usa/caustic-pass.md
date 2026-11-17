# The caustic pass: a second draw of flagged batches with an animated ripple texture

**Binary:** `psp-pulse-usa` `BOOT.BIN`. Read 2026-11-17 (lane `water-re`), decompile
only, no PPSSPP run. Companion to [mesh-draw.md](mesh-draw.md) (the four pass groups)
and [loading-screen.md](loading-screen.md) (`Texture_BindCausticFrame`,
`g_caustic_frames`). The cross-title picture is on
[water.md](../../../rendering/water.md).

## Answer

Pulse's one water *effect* is a caustic overlay. A batch whose header byte 3 has bit
`0x04` is drawn a second time, after the ordinary passes, with one of 16 caustic
frames bound, the texture coordinate generated from the vertex position by a texture
matrix (a top-down planar projection, one tile every 71.4 world units), tinted
`(0.2, 1.0, 0.8)` times a per-mesh byte, at depth-equal with depth writes off.

**Which data carries the bit (measured, confidence 90):** no batch on the Pulse PSP
EU disc (0 of every `.vex` in `Data.wad`), and on the PS2 EU disc only Vostok Reef:
`Data\Environments\08_Track\track.vex` 257 of 1,418 batches (40,300 vertices, 44 mesh
nodes: `Floor_nosubdivide`, `track25Shape`, `track26Shape`, ...), entry 1386 (hash
`0xed5754de`, unresolved name, 257 of 1,447) and `track_reversed.vex` 205 of 1,204.
The same circuit's `TrackStartup.xml` sets `<UnderwaterSound type="1"/>`. Vostok Reef
is download content on the PSP, so the PSP disc image never reaches this pass.
Reproduce: `OAG_SWEEP=1 cargo nextest run -p oag-vex --run-ignored all --no-capture
-E 'test(/water_caustic_sweep::/)'`.

## Functions and data

| Address | Name | Confidence | What |
| --- | --- | ---: | --- |
| `0x0890dc64` | `Mesh_BeginCausticPass` | 75 | GE state for the pass, below |
| `0x0890ddd8` | `Mesh_SetCausticNodeState` | 72 | per mesh node: tint, batch state list `0x202`, fog, the texture matrix |
| `0x0890e0dc` | `Mesh_EndCausticPass` | 75 | `Gu_TexMapMode(0,0,0)`, `Gu_DepthFunc(6)` |
| `0x08abf494` | `g_caustic_tint` | 70 | three floats `0.2, 0.8, 1.0` at `+0`, `+4`, `+8` |
| `0x08abf4a0` | `g_caustic_uv_scale` | 72 | float `0.014` (`0x3c656042`) |

Already named: `Texture_BindCausticFrame` `0x0890dcfc` (85), `g_caustic_frames`
`0x08b62d90` (85), `Mesh_DrawBatchSet` `0x0893021c`.

## Evidence

**The caller.** `FUN_089307b4` (the pass-group dispatcher, [mesh-draw.md](mesh-draw.md)),
group `8`, array `set+0x80`, count `set+0x7c`:

```c
if ((groups & 8) && set->caustic_count != 0) {
    if (!baked) Texture_BindCausticFrame();
    Mesh_BeginCausticPass(g_display);          // 0x0890dc64
    Gu_DepthMask(0); Gu_DepthFunc(2);          // equal, no depth write
    for each caustic batch:
        on a new mesh node: Gu_CallList(node + 0xd8);      // its world matrix
                            Mesh_SetCausticNodeState(node, g_display);   // 0x0890ddd8
        if (batch->pass_mask & 2) { Gu_DepthMask(1); Gu_DepthFunc(6); }
        else                      { Gu_DepthMask(0); Gu_DepthFunc(2); }
        Gu_CallList(batch list);
    Mesh_EndCausticPass(g_display);            // 0x0890e0dc
}
```

`Mesh_DrawBatchSet` replays the baked form of the same group (`set+0xb0`) right after
`Texture_BindCausticFrame()`.

**`Mesh_BeginCausticPass`** in full: `Gu_TexProjMapMode(0)` (project from position),
`Gu_TexMapMode(1,0,0)` (texture-matrix mapping), `Gu_TexWrap(0,0)` (repeat),
`Gu_Enable(10)`, `Gu_Disable(0xb..0xe)` (the four lights), `Gu_Color(0xffffffff)`,
`FUN_0881129c(1, white)`, `FUN_0881129c(2, white)`, `Gu_AlphaFunc(1,0,0xff)`,
`Gu_Disable(0)`.

**`Mesh_SetCausticNodeState`**:

```c
f = (float) mesh_header->byte_0x0e;                      // node+0x58 -> +0x0e
Gu_Ambient(0xff000000 | (int)(g_caustic_tint[0] * f)     // red   0.2
                      | (int)(g_caustic_tint[2] * f) << 8    // green 1.0
                      | (int)(g_caustic_tint[1] * f) << 16); // blue  0.8
Gfx_ResetTexTransform(display);
Gfx_BuildBatchStateList(display, 0x202, 0, node->0xcc, node->0xd0, 0);
Gu_StencilOp(0,0,0); Gu_PixelMask(0xff000000);
(mesh_header->byte_0x0c & 8) ? FUN_0891eac0(display) : Fog_Disable(display);
M = scale(g_caustic_uv_scale, g_caustic_uv_scale, 1) * rotation(1.5707964) * node_world;
M.column3 = (0, 0, 0, 1);
Gu_SetMatrix(3, M);                                      // 3 = the texture matrix
```

The rotation's angle is the literal `0x3fc90fdb` (a quarter turn); its sine and
cosine are placed through four `vpfxs` constant patterns. The two matrix products are
`FUN_08986020`. Nothing writes `g_caustic_tint` or `g_caustic_uv_scale`: each has
only read references, both inside this function.

**The frame.** `Texture_BindCausticFrame`: `frame = (int)(track_time * 30.0) & 0xf`,
so 16 frames at 30 per second, a 0.533 s loop. `Texture_LoadEffectSurfaces` loads
`Data\Tex\caustics\save.NN.mip` (128x64, 4bpp, alpha-only white palette).

## Confidence

- **75** on the pass state: every call is read in the decompile and the three
  functions have one caller. Not run: no PSP disc batch reaches it.
- **70** on the projection being top-down: a quarter turn and a uniform scale applied
  to the world position is the shape of a ground-plane projection; which axis the
  turn is about is read from four VFPU prefix patterns, not single-stepped.
- **70** on the tint's channel order: the three globals are packed into `Gu_Ambient`'s
  `0xAABBGGRR` word as read above.
- **Not read:** what batch state `0x202` blends as (`Gfx_BuildBatchStateList`'s table,
  [mesh-draw.md](mesh-draw.md)); what `mesh_header+0x0e` holds on Vostok Reef's nodes.

## PS2

**The PS2 executable was not read** (no PS2 program was open in Ghidra when this was wired);
the law is **inherited from PSP**, no PS2 confidence. What the PS2 disc itself shows:

- Byte 3 of a batch header is the same field (`Batch::draws_in_caustic_pass`), and only
  Vostok Reef carries the bit (`crates/vex/tests/caustic_pass_ground_truth.rs`).
- The frames ship as `Data\Tex\caustics\save.NN.pct` in `WADS2.WAD` (odd `NN`, 01 to 31, found
  by `Texture_FindOrLoad`'s `.mip` to `.pct` rewrite), so the PS2 build has the same 16-frame table.
- The mesh payload's byte at `+0x0e` (the tint scale) takes the values 25, 51, 76, 102, 127, 153,
  204 and 229 on the flagged meshes: tenths of 255, so the PSP's `/255` reading holds (a PS2
  `0x80 = 1.0` reading would be impossible past 128).
- Batch state `0x202` is `pass_mask` `0x202`: `BlendClass::Additive` (`GU_ADD`, `GU_SRC_ALPHA`,
  `GU_FIX` white) and alpha test `GREATER 0` (`Batch::blend_class`, `alpha_test_reference`), which
  settles the blend this page listed as not read.

Still open: the quarter turn's axis (read as about the horizontal axis, so the projection is
top-down: confidence 70, the frames show the pattern lying on the floor), the mesh fog function
(`FUN_0891eac0`, drawn here with black fog), and a PCSX2 frame.

## What this engine draws

The pass, on the PS2 disc's Vostok Reef (`oag_mesh::mesh::caustic_pass`, `oag_raceplay`'s
`scene::caustic`): 255 of 257 flagged batches of `track.vex` (the other two sit under an animated
node and are left undrawn), unlit, tinted `tint * byte / 255`, additive by the frame's alpha, depth
test only, frame `(int)(clock * 30) & 0xf`. `graphics.water = off` skips it. The coordinate is the
node-local position rotated by the node's rotation (the node translation is dropped, as the matrix
does), so a vertex moved in height keeps its coordinate.
