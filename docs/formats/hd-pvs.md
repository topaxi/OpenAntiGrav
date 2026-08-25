# `track.pvs`: Wipeout HD's authored potentially-visible set

**Status: understood.** The visibility table is decoded, validated against all
28 shipped files, corroborated against the engine's own loader, and consumed by
the renderer. A trailing region on twelve files is measured, shown to be
unreachable by the retail loader, and deliberately left alone.

Parser: [`oag_formats::hd_pvs`](../../crates/formats/src/hd_pvs.rs).
Ground truth: `crates/formats/tests/hd_pvs_ground_truth.rs`.
Consumer: `oag_render::pvs::ChunkSet`, wired in
`crates/game/src/race/visibility.rs`. Toggle a run with `--pvs true|false`.

**The engine's own loader and lookup were read afterwards** and are on
[visibility.md](../ghidra/functions/ps3-hdfury-eu/visibility.md). Where that
page and this one describe the same field, it wins - it is the executable
rather than an inference from bytes. Two things it corrected outright are
marked below.

## Why it matters

**A renderer that ignores this file draws about three times the geometry the
original does.** Averaged over the sampled cells of Talon's Junction and
Anulpha Pass, a cell sets **33%** of the circuit's chunks. Every one of the
other two thirds is something the artists decided you cannot see from there -
scenery behind a wall, the far side of a loop, the underside of a deck you are
standing on. Drawn anyway, inside a dense multi-level circuit, they read as
structures standing in the middle of the track.

This is the PS3 counterpart of the PSP's `section` partition
([ADR-0011](../architecture/adr/0011-authored-pvs-before-frustum-culling.md)),
and it is a different mechanism rather than the same one in another file:

| | Pulse (PSP) | HD (PS3) |
| --- | --- | --- |
| Where | `section` nodes inside `track.vex` | `track.pvs`, a file of its own |
| Partition | up to 64 authored boxes | 354 to 763 cells along the racing line |
| A cell's set | a 64-bit mask over sections | one bit per `.rcsmodel` chunk |
| Draw-call key | the governing `section` node | the chunk index |

The two never coexist: a `.vex` with `section` nodes has no `.rcsmodel` chunks
and a PS3 circuit has no `section` nodes.

## Layout

Big-endian throughout, like every other PS3 asset here.

```text
+0x00  u32  cells
+0x04  u32  chunks          - the sibling .rcsmodel's chunk count, exactly
+0x08  u32  16              - constant across all 28 files on the disc
+0x0c  u32  ?               - differs per file; not read
+0x10       cells x 4 f32   - one record per cell: x, y, z, and a repeat of z
...         cells x (chunks / 8 + 1) bytes - one bitmap per cell, LSB first
...         optional trailing bytes - see "What is not read"
```

## How each claim was settled

### `chunks` is the sibling model's chunk count - confidence 97

It equals `rcsmodel::Model::meshes.len()` for **all 28** `.pvs` files on the
disc, over a range from 84 (`zone_2`) to 1,902 (`modesto_heights`), and for
every `_reversed` variant separately - Talon's Junction's forward model has 983
chunks and its reversed one 984, and the two `.pvs` files declare 983 and 984.
Asserted by `every_declared_chunk_count_is_the_sibling_models`.

### The bitmap is `chunks / 8 + 1` bytes - confidence 84

**Corrected from the executable.** The loader computes exactly
`(chunks >> 3) + 1`, so a chunk count that is a multiple of eight gets a whole
spare byte rather than a tight fit. `ceil(chunks / 8)` agrees on every other
count and is wrong on those - and four of the disc's twenty-eight files have a
chunk count that divides by eight, which is why this page's first draft
reported a trailing section on two files that have none: a `ceil` reader
under-counts the table by exactly `cells` bytes.

**The disc corroborates the width by what it leaves out.** The last byte of
every cell's map carries only the bits the declared chunk count leaves valid:
Talon's Junction's 983 bits end seven into byte 123, and none of its 621 blocks
has a last byte reaching `0x80`; Anulpha Pass's 1,125 end five into byte 141,
and none of its 763 reaches `0x20`. 621 bytes all below `0x80` by chance is
`2^-621`. A reader with the width wrong would be seeing the next cell's leading
bits there. Asserted over all 28 files by
`no_bitmap_sets_a_bit_past_its_own_chunk_count`.

### Bit `k` is chunk `k` in file order, LSB first - confidence 92

**The bit order is settled by the executable**, not inferred from the file's
big-endian words: the routine that clears one chunk from the frame mask is
`mask[i >> 3] &= ~(1 << (i & 7))`, LSB first, confidence 84.

**That the index is the chunk's own file position is measured spatially.** For each sampled cell take the 50 chunks whose bounding
spheres are nearest the cell's position and the 50 that are furthest, and count
how many are set:

| index offset | Talon's near / far | Anulpha near / far |
| --- | --- | --- |
| -8 | 48% / 34% | 43% / 31% |
| -1 | 66% / 19% | 71% / 14% |
| **0** | **84% / 10%** | **90% / 11%** |
| +1 | 66% / 18% | 75% / 13% |
| +8 | 49% / 25% | 44% / 24% |

**The peak at zero is sharp, and that is the point.** File order in these
models is itself roughly spatially clustered, so a merely-coherent result would
be a plateau; a one-place shift costing 18 to 24 points of near-visibility is
what says the index is the chunk's own and not a neighbour's. MSB-first is
coherent too but far weaker - 2:1 near-to-far where LSB-first is 8:1.

**And the mapping does not drift.** Split Anulpha Pass's 1,125 chunks into
quartiles by index and ask each separately which offset maximises
near-visibility: **all four answer `+0`**, at 82%, 93%, 80% and 83%, with `+1`
next at 63%, 79%, 74% and 74%. A reader that skipped or merged a chunk would
peak at `+0` early and at some other offset later. It does not, and our parsed
`meshes.len()` equals the file's declared count on all 28 files.

### A cell's first three floats are a position - confidence 90

They march along the racing line in even steps: the median gap between
consecutive cells is **12.0 units on both circuits measured**, and Talon's
Junction's first twelve records step 12 units apart in `x` while `y` and `z`
drift smoothly. Large jumps occur where the authored order crosses between
branches (max 364 units on Talon's, 1,956 on Anulpha), so the records are a
list of viewpoints rather than a single polyline. The lookup built on this
position is what the near/far measurement above runs through, so the two
claims are confirmed by the same evidence.

**The fourth float repeats the third** on every record read, and is not
interpreted.

## What is not read

**Header words 1, 2 and 3 - and the engine does not read them either.** The
loader reads all four header words, then throws three away: word 1 lands in the
object and is **overwritten** before any use with a chunk count the *caller*
passes from the model, and words 2 and 3 land in stack slots nothing reads
again. So the engine trusts the model for the bitmap width, not the file. Word
1 matching the model's chunk count on all 28 files is therefore a fact about
the tool that wrote them rather than a field the game depends on - it is still
the check this parser makes, since it has no caller to be told by. Word 2 is
`16` on every file and is checked rather than skipped, so a file that differs
there surfaces as a refusal instead of a silently misparsed table.

**The trailing section, which the retail loader provably never reaches.**
`0x003c6f20` has six read sites - four header words, the cell records, and the
per-cell bitmaps - no seek, and it destroys the stream immediately afterwards.
16 of the 28 files are exactly `0x10 + cells * 16 + cells * width` bytes and
the other 12 carry between 3,525 and 78,900 further bytes. What is known about
it:

- It is **not more bitmaps**. On Talon's Junction the first 123-byte block past
  the declared 621 passes the padding test by chance and the next fails, so the
  table genuinely ends where the header says.
- It is **dense and high-entropy** - 256 distinct byte values, 34% zeros.
- It is **variable-length**. Its size divides the cell count evenly in some
  files (`+5` bytes per cell on `04_chenghou_project_reversed`, `+29`, `+30`,
  `+41`, `+60`, `+61` elsewhere) and not at all in others, including both
  Talon's Junction files. The two files that looked like `+1` byte per cell
  were the `ceil`-versus-`+1` width bug and have no trailing section at all.
- **Six of the twelve share one byte-identical file size that their own tables
  do not explain.** Seven `.pvs` entries on the disc are *exactly* 114,712
  bytes - `talons_junction` and its reversed, `zone_3`, `01_vineta_k` and its
  reversed, `02_track_reversed`, `15_anulpha_pass_reversed` - across three
  different PSARC archives, and only `01_vineta_k`'s table actually needs that
  many (472 cells x 227 bytes). Four `.pvspatch` entries are likewise exactly
  2,284 bytes each. These are the archives' own declared entry sizes, so the
  padding is on the disc rather than in this project's reader. Together with
  the loader provably never seeking past its table, the likeliest reading is
  that the authoring tool wrote a fixed-capacity buffer and the surplus is
  **stale buffer content, not a section** - which also explains why it looks
  like dense bitmap data. Not proven, and not worth proving: nothing reads it.

`Pvs::trailing()` reports how much there is; the load report prints it per
race. The visibility lookup does not need it.

**`track.pvspatch`, which is dead.** A second, much smaller file beside each
`.pvs` - 2,284 bytes for Talon's Junction, header `0xbe` = 190 followed by what
read as plausible floats. Its filename is formatted into a stack buffer beside
the other two and **that buffer is never read again**; no other code in the
image names the string. So in the retail EU executable it is written to the
disc and never opened. Confidence 80. Not decoded, and there is no reason to.

## How the renderer uses it

`oag_render::mesh::rcs` records each draw call's chunk index in
`DrawCall::chunk`, which is the join key. Per frame,
`oag_render::pvs::ChunkSet::around` unions the cells within `CHUNK_PAD` (24
units, two median cell spacings) of the craft and of the camera, and the
per-draw test is a byte load and a shift ahead of the frustum test - the same
two-tier ordering ADR-0011 argues for on the PSP.

Two conservative rules, both pointing the error towards drawing too much:

- **A draw call with no chunk index always draws.** The ship, the pads, the
  collision overlay and the sky are not chunks of the circuit's model.
- **A viewpoint more than `CHUNK_TRUST_RADIUS` (64 units) from any cell gets no
  first tier at all.** A craft that has fallen off the circuit is somewhere the
  partition was not authored around, and culling to a stale cell exactly then
  is the most visible way this could go wrong.

**The camera union is ours, and the original unions nothing.** Its whole PVS
path is five instructions: one cached nearest-cell query on one position, a
pointer to that cell's bitmap, and a `memcpy` of it into the frame mask. No
second lookup, no `or`, no accumulation - the frame's visible set *is* one
cell's bitmap, byte for byte. Ours is therefore a **superset** of the
original's, which is the direction the error has to point in. Whether the
position it queries is the craft's or the camera's is not established; unioning
both is what makes that not matter.

Gated on `[graphics] pvs_culling`, the knob that already gates the PSP tier.
