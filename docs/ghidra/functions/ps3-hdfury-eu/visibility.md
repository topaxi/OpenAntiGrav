# Visibility: how Wipeout HD loads and applies `track.pvs`

Binary: `PS3_GAME/USRDIR/EBOOT.BIN` from `hdfury-ps3-eu-dec.iso`, as
`/hdfury/EBOOT-ps3-hdfury-eu.elf` in Ghidra.

The file's own layout, and the measurements that decoded it from data alone,
are on [hd-pvs.md](../../../formats/hd-pvs.md). This page is the executable's
side: what it reads, what it ignores, and how many cells a frame uses.

**Nothing here is runtime-verified.** Per [renderer.md](renderer.md), static
reading of one binary caps every score on this page at **84**.

## The trap that shapes every address below

The Ghidra database gives all 24,155 functions the entry point's TOC. Every
function on this page is above `0x0032d5e0` - module B - so Ghidra's
TOC-relative data reads are wrong by a constant `0xfeec`, and labels like
`PTR_s_OfTxt_008a7948` are wrong-TOC artifacts whose names mean nothing.
Everything below is resolved with `scripts/ps3-toc.py`, which walks the OPD and
uses each function's own TOC.

## The names

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x003f5550` | function | `Pvs_LoadForTrack` | 84 |
| `0x003c6f20` | function | `Pvs_Load` | 84 |
| `0x006d1548` | function | `Pvs_BuildCellTree` | 80 |
| `0x003c7980` | function | `Pvs_NearestCell` | 82 |
| `0x003f2e68` | function | `Pvs_NearestCellCached` | 78 |
| `0x003c5b50` | function | `Pvs_CellBitmap` | 82 |
| `0x003c5b48` | function | `Pvs_BitmapBytes` | 82 |
| `0x003c5af8` | function | `Pvs_IsUsable` | 82 |
| `0x003fa120` | function | `Visibility_ShowAllChunks` | 80 |
| `0x003fa0f8` | function | `Visibility_HideChunk` | 82 |
| `0x003fad70` | function | `Visibility_CopyCellMask` | 80 |
| `0x00d44f80` | data | `g_ChunkVisibilityMask` | 80 |
| `0x003fab50` | function | `Scene_SubmitVisibleChunks` | 82 |
| `0x003fdae8` | function | `Scene_SortAndLightVisibleChunks` | 78 |
| `0x003fb330` | function | `Scene_RefreshNodeMatrices` | 80 |
| `0x003faf00` | function | `Scene_BuildStaticChunkMask` | 75 |

## The strings, and what names them

| Address | String |
| --- | --- |
| `0x007b1cf0` | `PVS: Loading from data file %s\n` |
| `0x007b1d10` | `FATAL: PVS Load Error!` |
| `0x007b3f88` | `%s\%strack%s.pvs` |
| `0x007b3fa0` | `%s\%strack%s.probes` |
| `0x007b3fb8` | `%s\%strack%s.pvspatch` |
| `0x007b0b70` | `Debug.Visibility.Enable Pvs` |

`scripts/ps3-toc.py attrib` gives `0x003c6f20` for the first two and
`0x003f5550` for the three filenames.

## `Pvs_LoadForTrack` (`0x003f5550`)

Three `sprintf`s into three 512-byte stack buffers, then the loader:

```
lwz r4,-0x5234(r2)   -> 0x007b3f88 '%s\%strack%s.pvs'       -> buf A (r1+0x70)
lwz r4,-0x5230(r2)   -> 0x007b3fa0 '%s\%strack%s.probes'    -> buf B (r1+0x270)
lwz r4,-0x522c(r2)   -> 0x007b3fb8 '%s\%strack%s.pvspatch'  -> buf C (r1+0x470)
bl 0x003fa010                       ; r5 = chunk count
bl 0x003c6f20                       ; Pvs_Load(pvs, buf A, chunkCount, param_1)
bl 0x003c4d78                       ; the .probes loader, only on success
```

**`track.pvspatch` is dead in the retail EU executable.** Its filename is
formatted at `0x003f56dc` into `r1+0x470` and that buffer is never read again;
no other function in the image names the string. Confidence 80. So the file is
shipped and never opened, and there is no reason to decode it.

## `Pvs_Load` (`0x003c6f20`)

`bool Pvs_Load(PvsObject *this, const char *path, u32 chunks, u32 param_4)`:

```c
stream = open(path, 0x2001, 0, 1);
if (!stream) { puts("FATAL: PVS Load Error!"); trap; return 0; }
read(stream, &this->0x10, 4);        // file +0x00 -> cell count
read(stream, &this->0x1c, 4);        // file +0x04 -> a chunk count
read(stream, sp-0x140, 4);           // file +0x08 -> discarded
read(stream, sp-0x13c, 4);           // file +0x0c -> discarded
this->0x1c = chunks;                 // OVERWRITTEN by the caller's count
this->0x14 = alloc(this->0x10 * 16, 16);
read(stream, this->0x14, this->0x10 * 16);
this->0x28 = alloc(this->0x10 * 4, 16);
this->0x20 = (chunks >> 3) + 1;      // BYTES PER BITMAP
for (i = 0; i < this->0x10; i++) {
    this->0x28[i] = alloc(this->0x20, 16);
    read(stream, this->0x28[i], this->0x20);
}
this->0x24 = 1;
... build the cell vector and its tree ...
destroy(stream);
```

### Three corrections this makes to a reading taken from the bytes alone

**1. The bitmap stride is `(chunks >> 3) + 1`, not `ceil(chunks / 8)`.**
Confidence 84. The two agree on every chunk count except a multiple of eight,
where the engine allocates and reads **one whole spare byte** per cell. Four of
the disc's twenty-eight files have such a count, and a `ceil` reader
under-counts their table by exactly `cells` bytes - which is what made
`03_track_reversed` (1,744 chunks, 559 cells) and `12_sol_2_reversed` (1,816
chunks, 477 cells) look like they carried a 559- and a 477-byte trailing
section. They carry none.

**2. Three of the four header words are read and thrown away.** Word 1 lands in
the object and is overwritten with the caller's `chunks` before any use; words 2
and 3 land in stack slots nothing reads again, consumed only to advance the
stream. Confidence 82. So **the engine trusts the model for the bitmap width,
not the file** - word 1 agreeing with the sibling `.rcsmodel` on all 28 files is
a fact about the tool that wrote them, not a dependency of the game.

**3. The loader cannot reach the trailing bytes.** Six read sites, no seek, and
the stream is destroyed immediately afterwards; the object retains no pointer
into file data beyond its two allocations. Confidence 80.

## The chunk count comes from a global (`0x003fa010`)

`return g ? *(int*)(g + 0x1c) : 0;`. Its `bl` sets no arguments, so Ghidra's
`_opd_FUN_003fa010(iVar4)` is decompiler noise. What that global is has not been
traced; that it is a chunk count follows from its use as the bitmap stride.

## `Pvs_BuildCellTree` (`0x006d1548`)

After the reads, `Pvs_Load` builds a `std::vector` of 32-byte elements, one per
cell - the cell's 16-byte record verbatim, an `int` for a split axis, and the
cell's original index - computes an AABB with `vmaxfp`/`vminfp` seeded from
`+FLT_MAX` (`0x008b7824`) and `-FLT_MAX` (`0x008b7828`), and calls this.

It is a **median-split kd-tree build**: widest of the AABB's components 0, 1
and 2, `nth_element` about the midpoint with one of three per-axis comparators,
the chosen axis written into the element, the box split at the median's
coordinate, recurse on both halves. Confidence 80.

## `Pvs_NearestCell` (`0x003c7980`)

`Pvs_NearestCell(PvsObject *this, vec4 pos)`; Ghidra drops the vector argument,
which arrives in `vs34`. It descends the tree from the median element:

- the metric is `d.x*d.x + d.y*d.y + d.z*d.z` - **three components**. The
  record's fourth float is loaded (it is a 16-byte aligned vector load) and
  **never enters the metric**;
- `bestDist` starts at `*(float*)0x008c3668`, statically `+FLT_MAX`, so **there
  is no radius**: the search always returns the globally nearest cell. That
  address is mutable, so this is the shipped initialiser rather than a constant
  - confidence 75 on "no radius", 82 on the rest;
- the plane epsilon is `0x008b7804 = 0.01f`;
- it returns the cell's original index.

`Pvs_NearestCellCached` (`0x003f2e68`) wraps it in a 256-entry MRU ring at
`manager+0x10` keyed on `dot3(pos - entry, pos - entry)`, forcing `w` to zero
first and caching the cell id in the entry's `w` lane. The cost is amortised;
the answer is still one cell. Confidence 78.

## The frame path is five instructions, and it unions nothing

`Debug.Visibility.Enable Pvs` is registered at `0x003a9520` against the debug
object at `0x00c489f0`, so the flag is the byte at `0x00c48f87`, and the whole
image has **exactly one** instruction that reads it:

```
003ada90  bl     0x003c5af8            ; Pvs_IsUsable(pvs)
003adaa0  beq    cr7, 0x003adab0       ; not usable -> everything visible
003adaa4  lbz    r0, 0x597(r22)        ; Debug.Visibility.Enable Pvs
003adaac  bne    cr7, 0x003aea20       ; enabled -> the PVS path
003adab0  bl     0x003fa120            ; disabled -> Visibility_ShowAllChunks
```

`Pvs_IsUsable` is `this->0x24 != 0 && this->0x1c != 0` - the loaded flag and the
chunk count, exactly the two fields `Pvs_Load` sets.

The PVS path in full:

```
003aea20  addi r3, r31, 0x100      ; &position, one vec4
003aea24  bl   0x003f2e68          ; cell = Pvs_NearestCellCached(&position)
003aea34  bl   0x003c5b50          ; bitmap = Pvs_CellBitmap(pvs, cell)
003aea44  bl   0x003c5b48          ; n = Pvs_BitmapBytes(pvs)
003aea54  bl   0x003fad70          ; Visibility_CopyCellMask(bitmap, n)
```

with `Visibility_CopyCellMask` a bare `memcpy(g + 0x100, src, n)`.

**One position, one cell, one `memcpy`.** There is no second lookup, no `or`,
no accumulation - **the frame's visible set is the chosen cell's bitmap, byte
for byte.** Confidence 82. `Pvs_CellBitmap` takes the cell index and
`Pvs_BitmapBytes` takes none, so there is nowhere for a union to hide.

This project's `oag_render::pvs::ChunkSet` unions the cells within
`CHUNK_PAD` of the craft *and* of the camera, which makes it a **superset** of
what the original draws. That is the direction the error has to point in, and
it is deliberate: the original never culls to a camera, so it has no chase
spring to pad for.

**Whose position `r31+0x100` is - craft or camera - is not established.**
`r31` is a large object (offsets to `0x7d1c` are used in the same function) and
was not traced. Confidence 55 that it is the camera, on the weak evidence that
the same vector is uploaded as a shader constant.

## The mask (`0x00d44f80`)

`0x1000` bytes, so at most 32,768 chunks. Three writers:

```c
Visibility_ShowAllChunks() { memset(mask, 0xff, 0x1000); }
FUN_003fa158()             { memset(mask, 0x00, 0x1000); *(u32*)(g+0x1100) = 0; }
Visibility_HideChunk(i)    { mask[i >> 3] &= ~(1 << (i & 7)); }
```

**`Visibility_HideChunk` settles the bit order outright**: byte `i >> 3`, bit
`1 << (i & 7)` - **LSB first**, confidence 84, independently of the spatial
measurement on [hd-pvs.md](../../../formats/hd-pvs.md). Ghidra renders it as a
64-bit rotate of `~1`; the low byte of that is `~(1 << (i & 7))`.

Note the asymmetry: the no-PVS path fills all `0x1000` bytes, while the PVS
path copies only `stride` of them, leaving the tail at whatever it held.

## Who reads the mask: three consumers, one list, no gate

All three are workers in the task table at `0x0086c9xx`, reached through their
`.opd` descriptors - which is why Ghidra shows them with no callers.

| Address | What it does with the mask |
| --- | --- |
| `0x003fab50` | the **draw submit**: skips any item whose bit is clear |
| `0x003fdae8` loop A | `dot3` distance per **visible** item, then a sort |
| `0x003fdae8` loop B | up to two lights per **visible** item |

The submit loop:

```c
count = scene->0x1c;
do {
  if ((1 << (i & 7) & mask[(i >> 3) + 0x100]) != 0) {
    item = scene->0x20[i];
    ... submit every draw call of item ...
  }
  i++;
} while (i != count);
```

**`i` is the loop counter and nothing else**, indexing the flat pointer array at
`scene+0x20`. So bit `i` means "slot `i` of the scene's item list" - not a draw
list, not an instance table, not a tree walk. And **`scene->0x1c` is exactly the
number handed to `Pvs_Load`**: `0x003fa010` returns it, `Pvs_LoadForTrack` passes
it, and it becomes `(chunks >> 3) + 1`. The bitmap's width and the list it
indexes are the same field of the same object. Confidence 82.

### And the scene object *is* the `.rcsmodel`

**There is no list builder.** Nothing in the image sorts, filters, buckets or
tree-walks the items, and nothing writes `scene+0x1c` or `scene+0x20` at all.
The object comes straight back from the generic RCS resource loader
(`0x005da6b8` -> `0x005d98b8`), which allocates a buffer, reads the file into it
and relocates its header offsets to pointers - so the scene's fields are the
file's own header, one for one:

| scene field | `.rcsmodel` header |
| --- | --- |
| `+0x1c` item count | mesh count |
| `+0x20` item pointer array | mesh **offset table** |
| `+0x24` per-item vec4 | bounds block |
| `+0x2c` / `+0x30` | material count / material offset table |

Checked against the disc for `talons_junction/track.rcsmodel`: 983 chunks, and
`0x40 + 983 * 16 = 0x3db0` is exactly the material-table offset - one `vec4` per
chunk, no gap, same order. Every consumer indexes by plain `i * 4`.

**So PVS bit `i` is `.rcsmodel` mesh `i`, in file order.** Confidence 90. That
is the executable-side reason for what
[hd-pvs.md](../../../formats/hd-pvs.md) already holds at 92 from spatial
measurement, and it settles that the mapping is not where a rendering
difference lives.

**A trap from the same pass, worth more than the confirmation: a `.vex` node
finds its chunk by HASH, not by index.** `0x003eeb08` linear-scans the chunk
table comparing `chunk[0]` against the node's `*(*(node + 0xdc) + 0x30)` and
yields 0 on no match. So bit `i` is chunk `i`, and node `i` is *not*.

**The test is not gated.** In all three loops the bit is the sole condition -
no flag on the item, no render-layer or pass id, no distance short-circuit, no
always-draw escape. An item whose bit is clear is not submitted, given no
distance, and given no lights.

**And a fourth pass only ever takes bits away.** `FUN_003fada8`, the next task
in the same table, walks a sub-range testing only what is still visible and
clearing on a bounds test - `mask[b] &= ~bit` or `&= 0xff`, never setting.
Confidence 80.

So the pipeline is `memcpy` one cell's bitmap, **narrow further**, submit: the
main view's visible set is a *subset* of the cell's. A reimplementation that
gates every chunk on the cell's bitmap and stops there draws **more** than the
original, not less.

### A second bit array exists, and it is not this one

`FUN_003faf00` ORs bits into an array whose effective base is `r26 + 0xC180`
(`stb r0, -0x3e80(r9)` with `r29 = r26 + 0x10000`), **not** the frame mask at
`g + 0x100`, whose base resolves through `-0x5160(r2)` to `0x00d44e80`. It is
built once at track load - `Pvs_LoadForTrack` calls it at `0x003f55d0`, before
the `.pvs` is even opened - for every item whose `flags & 1` is set. Its only
readers are `0x00401ba8`, `0x00402a88` and `0x004053e0`, and **all three are
also callers of `Pvs_NearestCellCached`**: three further passes that each
resolve their own cell and consult this static array, almost certainly
secondary views (shadow, reflection, mirror), unconfirmed.

This does not weaken the section above - `0x003fab50` and `0x003fdae8` never
read `g + 0xC180`, and nothing writes bits into `g + 0x100` per frame except
the `memcpy`. It is a correction to the broader claim that nothing anywhere
adds a bit back. Confidence 75; the `r26 = g` identification is inferred from
the shared `-0x3e80` bias across all thirteen sites rather than traced.

## `Visibility_HideChunk` belongs to the fallback, not the PVS

Its single caller is `0x003aa888`, inside the **non-PVS branch only** - the arm
that begins with `Visibility_ShowAllChunks` and then clears bits for items whose
own flag word says so:

```c
Visibility_ShowAllChunks();
for (i = 0; i < scene->0x1c; i++) {
  flags = *(u32 *)(scene->0x20[i] + 4);
  if (hide_bit(flags)) Visibility_HideChunk(i);
}
```

It never runs while the PVS is live. So there is no data-driven mechanism that
removes a chunk a cell declared visible - this is the *replacement* for the PVS,
and there is nothing here to port. Confidence 80.

## The chunk kind byte, and where it reaches the submit

A chunk's `+0x07` is a **three-way kind tag**; what it selects is the chunk's
world transform, not its geometry. The format side is on
[rcsmodel.md](../../../formats/rcsmodel.md), "Byte `+0x07` says which space a
chunk's positions are in". What belongs here is the executable's own shape.

**`Scene_RefreshNodeMatrices` (`0x003fb330`)** walks the scene's chunk list and
runs its body for `+0x07 == 2` only:

```c
block = **(uint **)(chunk + 8);            // the chunk's runtime block
node  = *(int *)(block + 0x70);
if (node != 0) {
    if ((*(uint *)(node + 0x34) & 0x1000) != 0) refresh(node);
    // four 16-byte rows from node->0x38 into block +0x00/+0x10/+0x20/+0x30
}
```

64 bytes as four rows is a 4x4 matrix. A kind-1 chunk is skipped entirely, so
`2` means **"this chunk's world transform is re-read from its scene node every
frame"** and `1` means it is not. Confidence 82.

**The load-time switch is three-way, and verified to rejoin.** At `0x003efda0`
inside `0x003eeb08`: test `1`, branch; test `2`, branch; otherwise fall through
to `lhz r11, 0x10(r8)`, the surface count. Arm 1 (`0x003eff2c`) emits opcode
`0x12` with the value at `sp+0x1d0` behind a redundancy check; arm 2
(`0x003eff68`) emits `0x12` with a *different* value, then `0x16`, then `0x03`.
Both arms end in a `b 0x003efdb4` back to the common path - checked, not
assumed. A second, structurally identical switch on the same byte sits at
`0x003ef77c` in the same function, over a different loop. **The opcodes are not
identified and are not guessed at**; what is established is that arm 2 emits
strictly more transform state than arm 1.

**The negative that matters for the format reader**: across both switch sites,
no arm reads the vertex declaration, the index buffer, the stride, the surface
count, or anything under the surface record - every one of them only stores
command words into the emission cursor. The byte cannot be a
list-versus-strip, an index-width or an attribute-set selector. Confidence 82.

**Six sites read it, and every one tests `== 2`**; none distinguishes `1` from
the fall-through. `Scene_SubmitVisibleChunks` folds it into a draw record's
sort key (`key.lo = (chunk[7] == 2)`), and `Scene_SortAndLightVisibleChunks`
guards two of its five draw buckets with `((key & 1) == 0) && (((key >> 1) & 4)
== 0)`, **excluding kind-2 chunks from that pair** - the `g+0x491f4` and
`g+0x4f1f8` lists. Confidence 78. This project's renderer does not model those
buckets.

**Confirmed and completed, 2026-09-05, from the producer's side.** The sort
key is built in one expression in `Scene_SubmitVisibleChunks`:
`rec[2] = (*(short *)(*(int *)(chunk + 8) + 6) << 1) | (chunk[7] == 2)`. So
`key.lo` really is the kind-2 test - the reading above stands, at 85 now
rather than 78 - and every *other* bit of the key, the ones that route a
record between the five buckets, is the halfword at `+0x06` of the chunk's
render block shifted up by one. The five buckets, their five consumers, and
the material-transparency test that splits the `g+0x491f4` and `g+0x4f1f8`
pair are written up in the twenty-ninth pass of
[zone-effectsettings-loader.md](zone-effectsettings-loader.md).

Two names from that pass belong to this subsystem: `Scene_ResetDrawLists`
(`0x003fb240`), the per-frame zeroing of all ten list counts and the four
merged-list pointers, and `g_MaterialDrawClass` (`0x00d45f90`), the one-byte
per-material class `Scene_BuildStaticChunkMask` computes and
`Scene_SubmitVisibleChunks` uses to pick which of the five input lists a
surface is submitted to.

**`+0x05` is read by nothing.** The one `lbz` of it in the geometry path,
`0x003f01e0`, sits inside an unrolled byte-by-byte block copy that carries
`+0x04` through `+0x08` alike - `+0x07` included, so even that site is not
reading a field. Confidence 85, and it closes `rcsmodel`'s old note that the
byte counts chunks.

## 2026-10-07, `hd-glass-opus`: the frustum cull, and the behind-the-glass target that chunk flag `0x10` draws into

Run against a live RPCS3 frame at the Vineta K tunnel pose (`place --pose=-839.8,-146.6,215.0`, kept
`-840.6,-146.7,214.2`), with guest memory read while the target was paused
(`data/scratch/hd-glass-opus/out/boot1`), and against the earlier capture of the same pose
(`data/scratch/vineta-k-fidelity/out/boot6`). Every draw of the track model was tied to its chunk by its vertex
array's IO offset: the scene object **is** the loaded file (`*0x00d42c68 = 0x418d6080`), so a draw's vertex
offset minus `0x018d6080` is a file offset, and exactly one surface descriptor names it (125 of 125 track draws
mapped, none ambiguous; `py/draw2chunk.py`).

### The pipeline, end to end

| Address | Name | Confidence | What it does |
| --- | --- | ---: | --- |
| `0x0040aba0` | `Scene_CullAndSortViewChunks` | 75 | per view: (debug byte `+0x598`) two frustum jobs over the two halves of the chunk list, a fence (`frustumTrackFence`, `After FrustumTestTrack Fence`), then the submit job |
| `0x003fada8` | `Visibility_FrustumCullChunks` | 85 | the frustum job: for each chunk still set in `g_ChunkVisibilityMask`, clear it when the test returns 1 (outside) |
| `0x005bcf48` | `Render_ClassifyBoxAgainstPlanes` | 80 | centre/half-extent box against planes `n..5`: 1 outside, 2 straddling, 0 inside |
| `0x005bc608` | `Render_BuildFrustumPlanes` | 85 | `(planes, fov, aspect, near, far)`; top/bottom at half-angle `fov * 0.5`, sides at `atan(tan(half) * aspect)` |
| `0x00987888` | `g_CameraFovDegrees` | 80 | the fov the cull planes are built from |
| `0x003aa2e8` | `Scene_GetPrimaryFog` | 85 | returns `0x00c49110` |
| `0x003aa2f8` | `Scene_GetAlternateFog` | 85 | returns `0x00c49120` |
| `0x003aa308` | `Scene_GetZoneTrackFog` | 70 | returns `0x00c49130` |

`Scene_PrepareFrame` builds the six planes into `0x00c49000 + 0x7cb0` (`bl 0x005bc608` at `0x003aad8c`: `f1` =
`g_CameraFovDegrees * 0.0174533`, `f3 = 0.1`, `f4 = 10000`) and copies them, with the position at `+0x100`, into the
job object at `+0x240..+0x2a0` (vtable `0x0086c850`, `0x003abbc4`). The job's run (`0x006cde00`) passes `job+0x40`
(the planes) and `job+0xa0` (the position) to `Scene_CullAndSortViewChunks`.

**Who gets which test.** `Scene_BuildStaticChunkMask` fills an 8-byte entry per chunk at `g+0x211c`: the chunk's
render-record pointer, and a word whose sign bit is `chunk[7] == 2`. The frustum job sends a kind-1 chunk to the box
test on the record's `+0x20` centre and `+0x30` half extent, and a kind-2 (node-placed) chunk to
`Render_ClassifyAgainstPlanes` (`0x005bd0d8`, a sphere) with the chunk's entry of the scene's **bounds block**.
**The engine rewrites that block for node-placed chunks at runtime**: 197 of 1,811 entries differ from the file in
the live read, and chunk 1316's live sphere sits at its node (`(-694.8, -127.6, 304.0)` against draw 57's node
translation `(-694.8, -97.2, 304.0)` applied to the file's `(0, -32.6, 0)`). The writer is not located.
Confidence 85 on the routing (both branches read in the disassembly), 80 on the rewrite (live read only).

### Three measured facts

1. **The PVS position `+0x100` is the camera** (it settles the 55 under "Open" below). Live: `0x00c49100` =
   `(-841.71, -143.81, 202.96)`, which is the eye solved from the frame's own projection, not the craft
   (`-840.6, -146.7, 214.2`). Confidence 90.
2. **The cull planes are built from the authored chase fov.** `g_CameraFovDegrees` reads `59.99`, and the live
   planes are 30.0 degrees vertical and 45.7 horizontal half-angles (`tan 45.7 = tan 30 * 16/9`); Feisar's
   `handlingstats.xml` authors `<ExternalCameraFar fov="60">`. Confidence 90 on the planes (read, not derived).
   The builder's aspect argument is a TOC float picked by the byte at `0x00938998` (`8/9` when set, `32/9` when
   clear); the byte read `0` and the planes are 16/9, so how that argument becomes the planes' aspect is **not
   understood**.
3. **Chunk render flag `0x10` sends a chunk to a second target, not to the main view** (confidence 88). The
   frame renders in two places: draws with surface clip `0x280 x 0x168` go to a **640x360** target at VRAM
   `0xC1E50000` (format word `0x123`, pitch `0x500`), the rest to the 1280x720 frame. The 640x360 pass is the sky plus
   every drawn chunk whose `Mesh::render_flags` has `0x10`, drawn with a projection **4/3 wider in tangent** than the
   main view (`c[256]` `sy = 1.29923 = 0.75 * cot 29.996`, 75.2 degrees vertical, against the main view's
   `1.7323`, 60 degrees). Tied draw by draw to chunks: 39 of 39 640-wide draws are `0x10` chunks and 84 of 84
   frame draws are not (boot 6); 39/82 (boot 1); 15/191 at the start slot (boot 7). No exception in 370 draws.
   The flag is block bit 4, which `Scene_SortAndLightVisibleChunks` routes to bucket B3 (`FUN_003fc140`) instead of
   B1 (zone-effectsettings-loader.md, twenty-ninth and thirtieth passes): **B3 is this pass**. The tunnel glass
   (`mt_tunnelrefraction`, chunk 1616, draws 96-97) carries the main view in `c[256]` and the 75.2-degree matrix in
   `c[260]`, so it reads the target at its own position under the target's projection. The flag occurs on
   Vineta K only (420 chunks on the disc). The glass binds the target at unit 2 (image rect `0x280 x 0x168`,
   clamp to edge, linear); how it reads it, and the target's draw state (all 35 chunk draws cull back faces), is in
   [rcsmaterial.md](../../../formats/rcsmaterial.md), "The behind-the-glass target is drawn" (2026-10-07,
   `hd-behind-glass`), which also draws it.

### What it predicts, and the score

The rule "nearest-cell PVS of the camera, then the live planes, kind-1 by record box, kind-2 by live sphere"
against the chunks the captured frame draws (`py/predict.py`, `py/fit.py`):

- **Kind-1, boot 6:** 44 of 44 drawn chunks kept, and every culled chunk culled but one. With the live planes
  the culled set is chunks 16, 17, 18, 55, 1315, 1317-1321 (a strut and girder column at `(-695..-728,
  -112..-152, 282..314)`, all `0x10` chunks, so behind-glass scenery) plus speed pad 1749: just off the main
  view's left edge, inside the wider behind-glass projection, which is culled with the main view's planes. The one residual is pad 1749, inside the planes by 0.3 (sphere) to 3.3 units (box) in boot 1's
  dumps, whose draw stream is a later frame than the planes; not scored.
- **Kind-2:** the original draws 5 or 6 node-placed chunks at this pose (818, 1316, blimps 1738-1743); this
  project draws every node-placed chunk the PVS allows (197) without a frustum test, because each one's
  `DrawCall::moving` turns the test off.
- Chunk 76 (`and_sand_sand`, drawn in boot 6, not in boot 1) is allowed by no cell near the camera: unexplained.

### Correction to the line above

"A reimplementation that gates every chunk on the cell's bitmap and stops there draws **more** than the original"
is now measured: at this pose the original submits 51 track chunks, this project 255.

## Open

- **What opcodes `0x12`, `0x16` and `0x03` are**, and what the third
  (neither-1-nor-2) chunk kind would do. No chunk on the disc takes it.
- ~~Whether `r31+0x100` is the craft or the camera (55).~~ The camera, measured live 2026-10-07 (above).
- Where the behind-glass projection's 4/3 enters, whether it holds at speed, and who rewrites the kind-2 spheres.
- The five other callers of `Pvs_NearestCellCached`: `0x003e63c0`, `0x003e9660`,
  `0x00401ba8`, `0x00402a88`, `0x004053e0`.
- The `.probes` loader at `0x003c4d78`, unexamined.
