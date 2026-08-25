# Visibility: how Wipeout HD loads and applies `track.pvs`

Binary: `PS3_GAME/USRDIR/EBOOT.BIN` from `hdfury-ps3-eu-dec.iso`, as
`/ps3-hdfury-eu/EBOOT.elf` in Ghidra.

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

**The test is not gated.** In all three loops the bit is the sole condition -
no flag on the item, no render-layer or pass id, no distance short-circuit, no
always-draw escape. An item whose bit is clear is not submitted, given no
distance, and given no lights.

**And a fourth pass only ever takes bits away.** `FUN_003fada8`, the next task
in the same table, walks a sub-range testing only what is still visible and
clearing on a bounds test - `mask[b] &= ~bit` or `&= 0xff`, never setting.
Confidence 80.

So the pipeline is `memcpy` one cell's bitmap, **narrow further**, submit: the
original's visible set is a *subset* of the cell's, and nothing anywhere adds a
bit back. A reimplementation that gates every chunk on the cell's bitmap and
stops there draws **more** than the original, not less.

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

## Open

- Whether `r31+0x100` is the craft or the camera (55). One new data point for
  camera: `0x003fdae8` uses a vec4 at `taskCtx+0x40` as the reference for a
  front-to-back sort, which is a camera-space operation - but that is a
  different parameter, so it is suggestive rather than evidence.
- The five other callers of `Pvs_NearestCellCached`: `0x003e63c0`, `0x003e9660`,
  `0x00401ba8`, `0x00402a88`, `0x004053e0`.
- The `.probes` loader at `0x003c4d78`, unexamined.
