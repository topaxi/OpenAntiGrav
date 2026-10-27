# HD's chunk draws: no per-chunk drop, but the draw issues only the surviving triangles (2026-10-27, `hd-chunk-drop-re`)

Answers the question `hd-unreferenced-chunks` left open: why the original draws neither surface of Vineta K's chunk 1556
(slot 633) when its PVS bit is set and its box is in the frustum. Evidence is the executable (decompiled in Ghidra on
`/hdfury/EBOOT-ps3-hdfury-eu.elf`) plus the seven original draw lists the earlier lane dumped from RPCS3; no new boot.

## Result in one paragraph

There is no rule R that drops a *chunk*. The scene's draw consumers draw every record they are handed. What the original
does is **cull per triangle**: a surface whose material state word (`material + 0x10`) has **bit 4** set (back-face culling
on) loses its clockwise (window space, front face CCW) triangles, and every surface loses triangles wholly outside the view
frustum; the draw is issued with the surviving index count, and **a surface with no survivor issues no draw**. Chunk 1556's
two surfaces have 20 triangles each and **all 40 are clockwise** from the start-grid eye, so the original draws nothing for
it. Ours culls no HD track triangle (`mesh_render.rs`: "Culling is off on purpose unless asked for"), so it draws the
slab's back faces over the lightmapped wall top. Confidence 85 for "1556 is absent because every triangle is culled"; 75 for
"the survivor count is what the draw carries"; the executable side of the culling (the SPU job) is **not read** (below).

## What the code does (decompiled, all read in full)

| Address | Function | What it does | Confidence |
| --- | --- | --- | ---: |
| `0x003fdae8` | `Scene_SortAndLightVisibleChunks` | copies the five input lists into the merged list, depth-sorts it (heap/intro sort on `g+0x20124c + chunk*4`), routes each record by its flag halfword into B1 to B5 (the table in `zone-effectsettings-loader.md`), then picks up to two lights per visible chunk. **No record is dropped**: every record is routed to at least one of B1 and B3. | 90 |
| `0x003fab50` | `Scene_SubmitVisibleChunks` | appends every surface of every PVS-set chunk, test is the mask bit only (already on `visibility.md`). | 82 |
| `0x00400a00` | B4 consumer (`FUN_00400a00`) | per record: picks the fog set, the shader variant, the textures and lights, then calls `0x005d7320`, `0x005d72b8` (state) and **`0x005d7590(ctx, chunk, surface)`**. No test that skips a record. `0x003ff860` (B5) has the same shape. | 85 |
| `0x005d7590` | `RenderOps_DrawChunkSurface` | switches on the **chunk's layout byte `+0x06`**: `1` (inline) to `0x005d7b40`, `5` (described) to `0x005d84b0` -> `RenderOps_BuildEdgeGeomJob` (`0x005d7b70`); **any other value draws nothing**. | 80 |
| `0x005d7b40` -> `0x005d78a8` -> `0x005c47cc` | plain draw | binds the streams and issues one indexed draw of `surface+0x38` indices (`0x005c47cc` emits nothing when the count is 0, and splits at 256). | 75 |
| `0x005d7b70` | `RenderOps_BuildEdgeGeomJob` | for a described chunk, builds one SPU `EdgeGeom` job per submesh and reserves a hole in the RSX command buffer **that the SPU fills with the processed draw**. | 85 |

So the draw for a described chunk (39,372 of the disc's 41,861 chunks, every Vineta K chunk in the census) is written by
Sony's `libedgegeom` job (`renderer.md`, "EdgeGeom's light path": the binary carries `attempt to use culling without
allocating an extra uniform table`), not by the PPU. The job record carries a copy of the view-projection constants
(`BuildEdgeGeomJob`, the 64-byte copy from `ctx+0xd8 + slot*0x20 + 0x18`), which is what a per-triangle frustum and
back-face cull needs. **What the SPU code itself does with them is not read**; the law below is measured from its output.

## The measurement: the draw's index count is the front-facing, in-frustum triangle count

Method (reproducible from the dumps; the tool was a throwaway example, `hd_facing_dump`, that reads a circuit's
`.rcsmodel`, transforms every world-space triangle by the frame's own view-projection (`c[256..259]` of a world-baked draw:
`clip = x*c256 + y*c257 + z*c258 + c259`, validated by the eye `c[465]` landing on clip `w = 0`), and counts per submesh:
triangles by sign of the homogeneous determinant of `[x y w]` (positive = counter-clockwise in window space), and those not
wholly outside one of the six planes). Draws are joined to surfaces by the surface bias in `c[467]` (unique key only).

1. **Cull enable is material state bit 4, exactly.** For the 1,531 uniquely matched Vineta K, Amphiseum, Talon's, Sebenco,
   Sol 2 and Tech De Ra surfaces, the draw's `cull` register is `1/405` (back faces) for all 1,523 with state bit 4 set and
   `0/405` for all 8 with it clear. **Zero exceptions.** (`material-state.md` already read bit 4 as `SET_CULL_FACE_ENABLE`
   from `Material_ApplyRenderState`; the front face is CCW from `Rsx_SetFrontFace`.) Confidence 90.
2. **The draw's index count is `3 x` the counter-clockwise, in-frustum triangles of the surface**: 1,139 of the 1,523
   culling surfaces match exactly, against **44 of 342** for the file's own index count (`hd-track-tone.md`'s "index-count
   anomaly", now explained). Worked example: chunk 1557 has 56 triangles, all counter-clockwise, 40 inside the frustum;
   the draw has 120 indices. Chunks 1554 and 1555 have 12 clockwise and 12 counter-clockwise triangles each; each draws 36
   indices.
3. **Chunk 1556: 0 counter-clockwise triangles of 20 on both surfaces** (surface 0 has 15 in the frustum, surface 1 has 11,
   all clockwise). Nothing survives, so no draw. Slot 633 is a navy slab under the pool whose upward face is a different
   chunk (slot 392); its own faces point away from the camera.
4. **The 30 census exceptions** (orphan chunks with their cell bit set and their box inside the frustum, no draw, from
   `hd-track-tone.md`): **29 of 30 have no surviving triangle** by this rule on their own list's view-projection: 18 have
   every triangle outside the frustum, 8 have only clockwise triangles, 3 are a mix. The one that does not fit is the
   Amphiseum's `lightbarrierpanel_new_additive` (chunk 573, bit 4 clear, 22 triangles in the frustum): open. Chunks that are
   drawn (1,332 of 1,336 predicted-drawn in the main view) keep their draws; 4 predicted-culled chunks are matched only by
   draws with another view-projection or target (a second pass), so not main-view counterexamples. The weapon pads (chunk 829,
   Talon's) are predicted drawn and absent; the pad path (`pads.md`) draws them, open.

## What is not explained

- **349 of the 1,523 culling surfaces draw a few triangles fewer than predicted** (for example 744 against 747 indices).
  A sub-pixel area cull (thresholds 0.25 and 1 px at 1920x1080) makes the match worse (1,000 and 813 against 1,139), and
  requiring all three `w > 0` does not help (1,130), so it is neither. The SPU's own frustum test (guard band, near plane)
  is the candidate; it is the `EdgeGeom` binary's cull routine, `data/extracted/ps3/spu-jobs/edgegeom-0x007f6d80.dis`.
- Node-space (kind 2) chunks were not predicted (their draw uses the node's matrix, not `c[467]` as a world bias).
- Inline-layout chunks (layout 1) take the plain draw and are not culled by the job; none was matched in these lists.

## 2048 / Omega

Same word, no draw list: **checked, applies, not wired.** The state word is the shared `.rcsmodel` material record (Vita
header `+0x12`, PS4 `+0x22`; the same 17 values as HD's). Census of state bit 4 (2026-10-27, `oag-rcs`
`hd_lineage_blend_ground_truth::state_bit_4_census_on_omega_and_2048`): Omega's five base archives 4,751 of 4,918, 5,804 of
5,871, 8,503 of 8,565, 286 of 289 and 13,568 of 13,722 materials; 2048 22,483 of 22,994 (97 to 99 percent, HD's 1,523 of
1,531 matched surfaces is the same shape). No 2048 or Omega draw list exists (shadPS4 gives no memory readout, and its race
shots are untrusted), so that these titles' tracks cull on the bit is read off the format, not measured: `Looks::unread`
leaves `honour_render_state` off for both and the `psp2` builder does not fill `material_state`. Wiring is a title rule plus
that builder, open in the thread.

## Names applied

| address | kind | name | confidence |
| --- | --- | --- | ---: |
| `0x005d7590` | function | `RenderOps_DrawChunkSurface` | 80 |
