# `.rcsmodel`: Wipeout HD's render geometry

**All of it.** 643 files and 686.5 MiB on the HD/Fury disc, against 72.3 MiB of
[`.vex`](vex.md) - the single largest thing on the disc after the
[`.gtf`](hd-status.md#what-is-genuinely-new) textures. On the PSP and PS2 a
`Mesh` node's payload *is* its geometry; on the PS3 that payload is a
bounding-box pair and a 32-bit word, and the vertices are here.

Implemented in [`oag_rcs::rcsmodel`](../../crates/rcs/src/rcsmodel.rs),
drawn by [`oag_render::mesh::rcs`](../../crates/render/src/mesh/rcs.rs), and
checked against the disc by
[`rcsmodel_ground_truth.rs`](../../crates/rcs/tests/rcsmodel_ground_truth.rs)
for the container and
[`rcsmodel_vertex_ground_truth.rs`](../../crates/rcs/tests/rcsmodel_vertex_ground_truth.rs)
for what a vertex holds.

```sh
just view data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC \
    --mesh /data/ships/assegai/ship.vex
just view data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC \
    --mesh /data/environments/talons_junction/track.vex --pitch 1.4
```

## The headline

| Question | Answer | Confidence |
| --- | --- | --- |
| Byte order | **Big-endian**, throughout, with no magic to sniff | 95 |
| Version word | `0x000a0000` on all 643 files | 92 |
| Position format | `i16` triple through a per-mesh `bias + q * scale` | 92 |
| Scale | `1/128` on all but 24 of `talons_junction`'s 983 chunks | 90 |
| Index format | Big-endian `u16` triangle list | 95 |
| Vertex stride | **In no field of the file.** 14, 18 or 22, recovered four ways | 88 |
| Where the stride is declared | Unknown, and **not** in the `.rcsmaterial` - see below | - |
| Vertex normal | **`+6`, packed 11:11:10 signed, big-endian** | 88 |
| Texture coordinate | **The last four bytes, two big-endian halves** | 90 |
| Texture | **The `.gtf` a material names at `+0x58`** | 92 |
| The rest of a vertex | A tangent at stride 22, located; not decoded | - |
| Chunk layouts | **Two**, selected by byte `+0x06`. All 643 files read | 88 |
| Material index | **A chunk's `+0x20`**, resolved by name on a craft | 90 |
| Which surfaces are see-through | **The low two bits of a material's `+0x10`** | 88 |

## What is decoded, and what is not

**Positions, triangle indices, vertex normals and the material table.** The
normals landed on
2026-08-17 and the renderer uses them; before that it lit these models off face
normals it computed itself, and it still does for any vertex the file has none
for. That fallback is reported per model rather than silent - a load report line
ending `531904 authored vertex normal(s)` is the file's own data, and one ending
`lit off face normals computed from the triangles` is a derivation, which is the
distinction [`CLAUDE.md`](../../CLAUDE.md) exists to keep.

**The texture coordinate is confirmed, and what confirmed it is a picture.**
The last four bytes of every vertex are two big-endian halves. That reading had
been *located* since the normals landed and deliberately not called a UV, because
there was no oracle for one. [`.gtf`](hd-status.md#what-is-genuinely-new) being
read supplied it: sampling Assegai's own textures through these coordinates
renders the words "ASSEGAI DEVELOPMENTS" legibly along the hull, and no wrong
offset or packing produces readable lettering. Confidence 90. The statistic
behind it, over every vertex of both models, reading where
[the declaration](#the-chunk-declares-its-vertex-layout-at-the-word-0x58-points-at)
says: **99.8 %** of Assegai's 25,144 land in the unit square, with **6**
non-finite; a circuit is looser as tiling makes it, at 76.0 % of Talon's
Junction's 600,280 in the unit square, with **9** non-finite.

**The circuit's used to be 1.5 % - some 9,000 vertices - and it was not a
residue.** This page called it "a real residue and not rounding" and the
renderer pinned those to zero. Almost every one was a `tangent` or a colour set
read as a coordinate, because the reader took the last four bytes of a vertex;
reading where the file says leaves 9. What survives on both models is three
orders of magnitude smaller and still unexplained, and both counts are now
pinned exactly rather than covered by a percentage bar. The zeroing pin stays,
for those and for [inline](#a-chunk-comes-in-two-layouts-and-byte-0x06-says-which)
chunks, which declare nothing.

The four bytes between the normal and it are partly identified; see below.

**A material record carries the shipped values of its shader's parameters**,
added 2026-08-23 and read by `oag_rcs::rcsmodel::material::parameters`.
Past `+0x38` - the tail this page used to describe as unread, records being 96
to 768 bytes where the shortest holds everything else - two words name a table:
`+0x30` an entry count and `+0x34` its offset, each entry `0x20` bytes of
`(name hash, kind, ..., value offset, quad count)`, with `0x8001` for a sampler
and `0` for a parameter, and a parameter's value four big-endian `f32`s.

This is where a `.rcsmaterial`'s declared constants actually come from: the
shader file says its program takes a `power1`, and *what it is* is authored per
model, here. **Confidence 92, from a disc-wide check rather than the file it
was read on**: 20,445 parameters across 9,757 materials of 643 `.rcsmodel`s,
every one a finite float of ordinary magnitude. The worked example is the
engine flare, on
[engine-flare.md](../ghidra/functions/ps3-hdfury-eu/engine-flare.md); the
finding with the widest reach is that `SpecScale` is per instance (235 on a
hull's paint, 500 on its glass) where
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md) records the
specular exponent as a shared stand-in. Nothing reads `SpecScale` yet.

## A chunk comes in two layouts, and byte `+0x06` says which

**Found by counting.** Every number on this page used to come from three files;
counting how much of the disc actually read turned up **224 of 643 failing**,
all identically, on a second chunk layout those three files never use. All 643
parse now.

| Byte `+0x06` of a chunk | Chunks | What follows |
| --- | ---: | --- |
| `0x05` | 39,372 | A submesh count at `+0x50` and a table of `0x80`-byte descriptors at `+0x60` |
| `0x01` | 2,489 | **No descriptor table at all**: one submesh, its buffers named in the chunk header |

The inline form:

```text
+0x54  u32   vertex buffer offset
+0x58  u32   index count
+0x5c  u32   index buffer offset
+0x6c  u16   vertex count
```

The word at `+0x50` is a file-wide pointer in these chunks, not a submesh count,
and reading it as one is what produced counts in the hundreds and a descriptor
table past the end of a 1 KiB file. `aurora.rcsmodel` is the smallest worked
case: 1,236 bytes, one chunk, three vertices at stride 18 and one triangle.

**Confidence 88, and what settles it is the index-range invariant rather than
the field names.** With both layouts read, **50,873 of 50,873 submeshes** across
all 643 files hold a whole number of triangles and name no vertex they do not
have - and the vertex count and the index buffer come from different fields, so
a wrong offset for either would surface as an out-of-range index rather than as
a plausible-looking mesh. That check used to cover 1,274 submeshes of three
files.

Two things still unexplained. Byte `+0x07` takes the values `01` and `02` and
nothing here distinguishes them. And the byte above the layout selector counts
chunks but reads `0xff` on many, which no reading accounts for.

## The `.vex` is not optional

This format cannot be read on its own, and that is a property of the format:

- **A chunk is addressed by hash.** The 32-bit word at a `Mesh` node's `+0x30`
  is the chunk's own first word. Without the `.vex` you have geometry and no
  idea which node - and therefore which world transform - it belongs to.
- **The vertex stride is not stored**, and the `.vex` node's authored box is the
  tightest oracle for it. Not the only one any more - the buffer layout settles
  it without leaving the file, and that is what the `.vex`-free rules below rest
  on - but the box is what every other rule was checked against.

## Layout

Big-endian throughout, and unlike the `.vex` there is **no magic**: the version
word is the only signature, so a reader checks it rather than assuming.

```text
+0x00  u32   version, 0x000a0000 on every file on the disc
+0x04  u32   end of the directory / first byte of chunk data
+0x08  u32   0xffffffff
+0x1c  u32   mesh count
+0x20  u32   offset of the mesh offset table: `count` big-endian u32s
+0x24  u32   offset of the file-wide bounds block
+0x28  u32   offset of the string pool (material and texture paths)
+0x2c  u32   material count
+0x30  u32   offset of the material offset table
```

**Confidence 92**, and the reason is arithmetic rather than plausibility: on all
four files read, `+0x04` equals `+0x20 + count * 4` exactly - the directory ends
where the last table does. `padreplacement.rcsmodel` is the degenerate case that
pins it, 128 bytes with a mesh count of 0.

| File | `+0x04` | count | table at | check |
| --- | ---: | ---: | ---: | --- |
| `padreplacement.rcsmodel` | `0x40` | 0 | `0x40` | `0x40 + 0` |
| `assegai/ship_lod1.rcsmodel` | `0x260` | 4 | `0x250` | `0x250 + 0x10` |
| `assegai/ship.rcsmodel` | `0x50c` | 15 | `0x4d0` | `0x4d0 + 0x3c` |
| `talons_junction/track.rcsmodel` | `0x22acc` | 983 | `0x21b70` | `0x21b70 + 0xf5c` |

One mesh chunk, at an offset the table gives:

```text
+0x00  u32     hash, matching the `.vex` Mesh node's own +0x30 word
+0x30  f32[3]  position bias, in the node's own space (world space when no node
               references the chunk - see below)
+0x40  f32[3]  position scale
+0x50  u32     submesh count
+0x60  ...     submesh descriptors, 0x80 bytes each
```

One submesh descriptor:

```text
+0x00  u8[8]   vertex-format word, `83 XX 10 10 10 10 10 00`. NOT the stride.
+0x08  u16     vertex count
+0x0a  u16     index count
+0x10  u32     index buffer offset
+0x14  u16     index buffer size, padded to 16
+0x18  u32     vertex buffer offset
```

The string pool holds `.rcsmaterial` and `.gtf` paths in the clear, one pair per
material, immediately before the chunk that uses them - for example
`data/materials/ships/glass_texture.rcsmaterial` and
`data/ships/assegai/livery1/assegai_glass.gtf` in front of Assegai's windscreen.

## The index buffers, checked exhaustively

**Confidence 95**, on the strongest invariant available, and now across the
**whole disc**: all 643 files, **50,873 of 50,873 submeshes**, index count a
multiple of three and every index below its submesh's own vertex count. Neither
would survive a wrong offset or a wrong width, and the second is checked against
a number read from a different field. It stood at 1,274 of 1,274 over three
files until the [second chunk layout](#a-chunk-comes-in-two-layouts-and-byte-0x06-says-which)
was found.

## The vertex layout

**Every vertex layout on the disc is declared** - see
[the chunk declares its vertex layout](#the-chunk-declares-its-vertex-layout-at-the-word-0x58-points-at),
which is the authority. What follows is how the fields were read before that
block was found, kept because it is the evidence each decode rests on and
because an [inline](#a-chunk-comes-in-two-layouts-and-byte-0x06-says-which)
chunk declares nothing:

```text
+0x00  i16[3]  position, through the chunk's own bias and scale
+0x06  u32     normal, packed 11:11:10 signed - see below
+0x0a  ...     tangent at stride 22; something else at stride 18
+0x0e  ...     stride 22 only; four bytes that look like an RGBA vertex colour
last 4 bytes   two f16, in a texture-coordinate range
```

The declaration names three of those: `+0x0a` at stride 22 is `tangent`, the
four bytes at `+0x0e` are a colour set, and **the last four bytes are usually
not the diffuse coordinate at all**. What sits at `+0x0a` on stride 18 - the one
field this reading called unrecovered - is `Uv1`, the diffuse coordinate, on the
layouts whose last four bytes are `lightmapUV`.

### The normal, at `+6`

**Confidence 88.** Three signed fields packed low-to-high in a big-endian `u32`:
**11 bits of x, 11 of y, 10 of z**, each divided by its own half-range.

The odd split is what the recovery turned on, and it was read off rather than
searched for. A **planar** submesh - one whose faces all point the same way -
must carry the same normal on every vertex, so whatever encodes it is simply
whatever is constant across those vertex records. Two of Talon's Junction's
roads supply the two axes that pin the split:

| The submesh's own normal | The constant word at `+6` | What fits |
| --- | --- | --- |
| `(+1, 0, 0)` | `0x000003ff` | `x = 1023` on an 11-bit low field, and nothing else |
| `(0, -1, 0)` | `0x00200800` | `y = -1023` on an 11-bit field at bit 11, and nothing else |

Neither works at 10 bits: `0x3ff` is `-1` there, not `+1023`. Two checks then
hold across whole models:

- **It is a unit vector on 91,376 of 91,480** vertices across the three models
  read, 99.9 %. Per stride the minority widths are thin and noisier - stride 14
  is 97 % on 206 vertices of Assegai and 100 % on 186 of the circuit, stride 22
  is 99.9 % on 23,593 of Assegai and 91.5 % on 437 of the circuit - so the
  ground truth asserts 90 % per width and 99 % pooled rather than one number
  that hides which widths carry real evidence. No other reading
  of any other offset in the vertex exceeds 51 %, and that comparison is what
  located the field before anything asked what it meant.
- **A zero word is an unused vertex, not a misdecode.** 27 of Assegai's records
  and 166 of Talon's Junction's are zero from the position onward - padding at
  the end of a buffer. They are excluded from the statistic above, `emit`
  derives a normal for them from the triangles, and `Report::authored_normals`
  does **not** count them: 530,359 of Talon's Junction's 531,904 vertices are
  lit off the file, and the report says so rather than claiming all of them.
- **It agrees with the geometry**, against an oracle the `.rcsmodel` does not
  state: the area-weighted average of the faces touching each vertex, over
  meshes more than a unit across where `1/128` quantisation cannot make the
  triangles degenerate. **82 %** on Assegai, 92 % on its LOD1, 93 % on Talon's
  Junction, within 18 degrees. The best reading of any other offset reaches
  14.6 %.

The shortfall is the point rather than a defect: **a hard edge is exactly where
the exporter splits a vertex and authors a normal no smooth average has**, which
is why a model stores normals instead of computing them. It is also why the
renderer prefers these over its own.

### `+0x0a` is a tangent at stride 22 and is not one at stride 18

**And the `83 XX` descriptor byte does not decide which**, which was the obvious
hypothesis and is measured false. Grouping every submesh of the three models by
`(stride, XX)` and asking what the four bytes at `+0x0a` are:

| stride | `XX` groups | vertices | unit vectors | median `\|dot\|` with the normal |
| ---: | --- | ---: | --- | --- |
| 22 | `08`-`0e`, all 7 | 78,440 | 90.4 - 99.9 % | **0.007 - 0.008** |
| 18 | `07`-`11`, all 11 | 484,328 | 7.0 - 59.2 % | **0.518 - 0.574** |

At stride 22 the field is a unit vector perpendicular to the vertex's own
normal, in every group without exception: a **tangent**, stored as three biased
`u8` with a fourth byte that is constant per submesh and reads as a handedness
sign. At stride 18 it is a unit vector on a minority of vertices and its median
`|dot|` sits at `1/sqrt(3)` = 0.577, which is what a constant `(-1,-1,-1)`
scores against any axis-aligned normal - so a large share of those records hold
`00 00 00` there and the field is something else. **What it is on stride 18 is
unrecovered**, and no code reads it.

Two things this measurement is worth beyond the conclusion. The split is by
*width* and not by descriptor, so `83 XX` names neither the stride nor the field
set and what it does select is still open. And it corrects this page: an earlier
reading put Assegai's stride-18 meshes at a median `|dot|` of 0.02, on **62**
vertices of one mesh; across 484,328 they are 0.55.

## The chunk declares its vertex layout, at the word `+0x58` points at

**Confidence 92**, measured 2026-08-18 over every `.rcsmodel` on the disc.
Read by [`rcsmodel::vertex_decl`](../../crates/rcs/src/rcsmodel/vertex_decl.rs)
and checked by
[`rcsmodel_decl_ground_truth.rs`](../../crates/rcs/tests/rcsmodel_decl_ground_truth.rs).

**Everything two sections of this page describe as unrecoverable is written
down in the file.** The stride, the byte offset of each attribute, its type and
its *name* are all in a block a [described](#a-chunk-comes-in-two-layouts-and-byte-0x06-says-which)
chunk's `+0x58` points at. Two of the inferences that stood in for it were
wrong, and both are visible in a frame:

- **The last four bytes of a vertex are usually not its diffuse texture
  coordinate.** On the commonest stride-18 layout they are `lightmapUV`, and
  the diffuse coordinate `Uv1` sits at `+0x0a`; on others the last four bytes
  are `tangent` or a colour set. Painting a diffuse texture through a lightmap's
  atlas-packed coordinates smears it into streaks, which is what a quarter of
  Talon's Junction looked like.
- **There are more strides than three.** The disc declares 10, 14, 18, 22, 26,
  34 and 38. [The search](#the-vertex-stride-is-not-in-the-submesh-descriptor)
  knew three, and it gave up on 3,382 of the disc's 39,372 described chunks,
  which drew nothing at all. **Fixed 2026-08-25**: `rcsmodel::STRIDES` now
  carries all seven, widening the disc's read coverage from 96.06% to 97.69%
  and cutting undecodable surfaces from 371 to 115 - see
  [`rcsmodel_stride_ground_truth.rs`](../../crates/rcs/tests/rcsmodel_stride_ground_truth.rs),
  which pins the invariant that broke. The remaining 115 have no declaration
  either and are concentrated in the `fe/` track previews, unchased.

### Layout

```text
+0x00  u8   attribute count - 2 to 7 on the disc
+0x01  u8   vertex stride
+0x02  u16  zero
```

then `count` records of eight bytes:

```text
+0x00  u32  attribute name hash, ~crc32(name)
+0x04  u16  the vertex stride again
+0x06  u8   type: component count in the high nibble, RSX vertex type in the low
+0x07  u8   byte offset of the attribute within the vertex
```

**Only on a described chunk.** On an [inline](#a-chunk-comes-in-two-layouts-and-byte-0x06-says-which)
one `+0x58` is the index count, and reading it as a pointer there is what made
the first disc-wide sweep come back as noise - 606 declarations with a stride of
zero and type codes spread uniformly over all sixteen values. Gating on the
`+0x06` byte makes the same sweep exact.

### What says this is the layout rather than a field that correlates with it

Four independent checks, all disc-wide:

| Check | Result |
| --- | --- |
| The pointer lands inside the file | **39,372 of 39,372** described chunks |
| The stride repeated at each record's `+0x04` equals the header's | **every one** of 118,000-odd records |
| The declared stride equals the one the box search fits, where it fits one | **35,983 of 35,990** |
| The search settles on nothing at all | **3,382 chunks**, every one of which the declaration decodes |

The repeated stride is the strongest of the four, because the bytes did not have
to pass it: a wrong pointer, a wrong header size or a wrong record size each
breaks it on the first record. The agreement with the box search is the second,
because the search is an independent *fitting procedure* against the chunk's own
authored bounding box and shares no reasoning with this block.

### The type codes, and there are exactly seven

Component count in the high nibble, `CELL_GCM_VERTEX_*` in the low - the RSX's
own vertex enumeration, the same numbering
[the blend factors](#the-factor-values-and-which-are-mapped) come from.

| Code | Shape | Uses | What carries it |
| --- | --- | ---: | --- |
| `0x35` | 3 x `S32K` (`i16`, not normalised) | 39,372 | `position`, on every chunk |
| `0x16` | 1 x `CMP` (one packed word) | 39,372 | `normal`, on every chunk |
| `0x23` | 2 x `SF` (half) | 54,120 | every texture coordinate |
| `0x43` | 4 x `SF` | 2,537 | an alias covering two `0x23`s at one offset |
| `0x44` | 4 x `UB` (byte over `0..=1`) | 22,117 | `tangent`, `colorSet1`, `VertexColour1` |
| `0x22` | 2 x `F` (`f32`) | 230 | a texture coordinate, on the few that want the range |
| `0x42` | 4 x `F` | 230 | the alias of two of those |

An eighth code would be a fact this reading has not seen; the ground truth
asserts the set rather than tolerating a new member, for the reason
[`Factor::from_rsx`](#the-factor-values-and-which-are-mapped) returns `None` on
a fifth blend factor.

The aliases are why the attribute list is **not** in offset order and **not** a
partition: a `0x43` at `+0x0a` covers exactly the bytes two `0x23`s at `+0x0a`
and `+0x0e` do, so a shader can bind either view.

### The names, and they are Maya's

The hash is `~crc32(name)` - the same one
[`renderer.md`](../ghidra/functions/ps3-hdfury-eu/renderer.md#the-name-hash-is-crc-32)
recovered from `Crc32_HashString`, complement and all. Naming an attribute is
therefore a wordlist problem, and twelve of the disc's 49 hashes fell to one:

| Hash | Name | Type | Uses |
| --- | --- | --- | ---: |
| `0xb9d31b0a` | `position` | `0x35` at `+0x00` | 39,372 |
| `0xde7a971b` | `normal` | `0x16` at `+0x06` | 39,372 |
| `0x427214fc` | `Uv1` | `0x23`, `0x22` | 28,298 |
| `0x1aaf7631` | *unnamed* | `0x44` | 13,485 |
| `0x26a7b665` | `lightmapUV` | `0x23` | 13,293 |
| `0xdbe5f417` | `tangent` | `0x44`, `0x43` | 5,765 |
| `0x7a3f521c` | `uv1` | `0x23` | 5,258 |
| `0xdb7b4546` | `Uv2` | `0x23` | 2,653 |
| `0x7493d450` | `VertexColour1` | `0x44` | 2,595 |
| `0xce5cd9d9` | `colorSet1` | `0x44` | 484 |
| `0x49f76806` | `Uvset1` | `0x23`, `0x22` | 424 |
| `0x2003d7e6` | `map1` | `0x23` | 360 |
| `0xb90a865c` | `map2` | `0x23` | 91 |

**Three of them are controls, and that is what makes the rest credible.**
`position`, `normal` and `tangent` were decoded from content long before this
block was read - three quantised shorts at `+0x00`, one 11:11:10 word at `+0x06`,
and [four biased bytes at `+0x0a` on stride 22](#0x0a-is-a-tangent-at-stride-22-and-is-not-one-at-stride-18).
Candidate strings picked with no knowledge of where they would land hash onto
exactly those three attributes, on every chunk of the disc. A guessed name
landing on a 32-bit hash is not a coincidence; three of them landing on the
three attributes whose content was already known is not one either.

`map1`, `map2`, `colorSet1` and `VertexColour1` are **Maya's** vocabulary, which
agrees with what the `.vex` files already say about the pipeline: one of Talon's
Junction's node names is
`Z:/WipeoutHD/Data/Source/Wip/DLC3/Environments/Talons_Junction/resource.ma`.

37 hashes are still unnamed. The one worth attacking next is `0x1aaf7631` -
13,485 uses, always four normalised bytes, and it is what sits in the last four
bytes of the stride-18 layout that has no lightmap.

**The shaders share this namespace, and looking there does not name it.** HD's
executable carries 124 `SHO` shader blocks whose attribute tables are keyed by
the *same* `~crc32` hashes - `position` is `0xb9d31b0a` and `normal` is
`0xde7a971b` on both sides, which is how a stride authored per chunk binds to a
program compiled once. Reading all 124 named eleven attribute names, and
**`0x1aaf7631` appears in none of them**: the commonest four-byte attribute on
the disc is one no engine-owned program declares an input for. See
[`renderer.md`](../ghidra/functions/ps3-hdfury-eu/renderer.md) and
`scripts/ps3-sho.py`.

### The four-byte attributes are three different things, and one is vertex colour

**Confidence 88**, measured 2026-08-18 over every model on the disc; the test is
`a_packed_tangent_is_a_unit_vector_and_a_vertex_colour_is_not`.

Type `0x44` is four normalised bytes, and 22,117 attributes carry it. They are
not one thing. The discriminator is the one that located the
[normal](#the-normal-at-6) - read the first three lanes as `byte / 127.5 - 1`
and ask whether the result is a unit vector perpendicular to the vertex's own
normal - and it separates them cleanly:

| Attribute | Vertices | Unit vectors | Mean \|dot\| with the normal | Fourth byte |
| --- | ---: | ---: | ---: | --- |
| `tangent` | 3,518,023 | **96.1 %** | **0.029** | 0/255, a handedness sign |
| `colorSet1` | 682,796 | 9.2 % | 0.524 | **255 on every one** |
| `0x1aaf7631` | 4,257,172 | 13.4 % | 0.517 | 23 % at 255, 66 % at 0 |

0.577 is `1/sqrt(3)`, what a random direction scores against any normal, so the
lower two rows are the null result and `tangent` is the only direction among
them. `tangent` is the control: it was measured as a packed direction before the
declaration named it.

**`colorSet1` is painted vertex colour.** Its alpha lane is 255 on all 682,796
vertices, and on Talon's Junction all 91,312 of them belong to one material the
disc itself calls **`defuse_occulsion_vert_col_tint`** - 101 submeshes. The
three colour lanes run the full `0..=255` with a mean of 128 and peaks at both
ends, which is painted occlusion rather than a light tint.

**Nothing draws it, and that is a hold rather than a gap.** `oag_render`'s
`GpuVertex` has had a colour slot and a shader that consumes it since the PSP
work, and the `lit` field beside it records what happens if baked lighting is
multiplied by a light rig as well: the surface comes out nearly black. HD's
chunks carry authored normals *and* this colour, so a draw wants one or the
other, and which is per-draw state nothing here has recovered on either console.
Wiring it on a guess is the failure `CLAUDE.md` names - so it is read, measured
and left unwired, the same hold [the lightmap](#stride-22-carries-two-texture-coordinate-sets-and-so-do-others)
takes.

**`0x1aaf7631` is neither**, which is why the wordlist has not named it. Not a
direction by the test above, and not an opaque colour either - its fourth lane
is 66 % zero and 23 % 255, a mask's shape rather than an alpha's. 13,485
attributes, the commonest four-byte one on the disc.

### Which coordinate a diffuse texture is sampled through is a rule, not a reading

A vertex may declare several `0x23` attributes and **nothing here reads the
shader that chooses between them** - the `.rcsmaterial` is compiled RSX
microcode, per [the material section](#the-material-table-and-which-surfaces-are-see-through).
So `VertexDecl::diffuse_texcoord` applies a rule and says so: *the first declared
two-component coordinate that is not `lightmapUV`*.

That is right on the evidence there is - a lightmap's coordinates are
atlas-packed, painting a diffuse texture through them streaks visibly, and `Uv1`
is what every layout carrying a lightmap also carries - and it stays a rule
until a shader is read. **984 of the disc's chunks declare no texture
coordinate at all**; those are counted rather than given a substitute.

## The vertex stride is not in the submesh descriptor

**And the section title used to end "is not in the file", which was wrong: it is
in the file, in a block the submesh descriptor points to rather than holds.** See
[the chunk declares its vertex layout](#the-chunk-declares-its-vertex-layout-at-the-word-0x58-points-at). Everything below stands as measured and is kept because it is what the search
`oag_rcs::rcsmodel::stride` still rests on - the searches are how an
[inline](#a-chunk-comes-in-two-layouts-and-byte-0x06-says-which) chunk's stride
is found, since those declare nothing - and because the negative result is
exact: the descriptor really does not carry the stride. Looking there was not
the mistake; looking *only* there was.

This is the one genuinely open part of the format, and it is worth stating
plainly rather than burying: **no field anywhere in the 0x80-byte submesh
descriptor holds the stride.** Searched exhaustively over 138 submeshes whose
stride an authored bounding box settles independently:

- No byte and no `u16` at any of the 128 positions equals the stride on more
  than **1** of the 138.
- No `u32` or `u16` equals `vertex_count * stride`, or that rounded up to 16, on
  more than **11**.
- The vertex-format word does not determine it either. Grouped by it:

| Descriptor | stride 14 | 18 | 22 |
| --- | ---: | ---: | ---: |
| `8307101010101000` | 10 | - | - |
| `8308101010101000` | 7 | 31 | 10 |
| `8309101010101000` | - | 3 | 2 |
| `830a101010101000` | - | 12 | 3 |
| `830b101010101000` | - | 13 | 8 |
| `830c101010101000` | - | 22 | 7 |
| `830d101010101000` | - | 10 | - |

`8308...` appears with all three widths, so the word is not a format id in the
sense that would fix a stride.

### The `.rcsmaterial` was the leading hypothesis, and it is not the answer

This page used to say, at confidence 40, that the `.rcsmaterial` was where the
stride would turn out to live: it binds a shader, a shader declares its input
layout, and a model file that stores paths to materials would not need to repeat
it. **One was read on 2026-08-17 and that is not what it is.**

`data/materials/billboards/cf_fx350.rcsmaterial`, 1,536 bytes, is a **compiled
RSX shader container**. Its shape, so nobody reads it twice for this:

```text
+0x000  u32     2, then a 0x100-byte header block
+0x010  u32[9]  parameter name hashes
+0x100  ...     two shader-object records: source offsets, sizes, a 'SHO' block each
+0x1a0  'SHO'   shader object, then Cg/RSX microcode to the end of the file
+0x1d0  ...     bindings: (name hash, u16 type, u16 count, u16 register, 0xffff)
```

The bindings are shader *parameters* - constants and samplers, addressed by
hashed name and by RSX register - not a vertex input layout, and the microcode
after them would have to be disassembled to recover one. That is a real piece of
work and it is no longer on the critical path, because the layout rule below
answers the question the `.rcsmaterial` was being read for. **Nothing in this
project reads a `.rcsmaterial` today**, and the hypothesis is withdrawn rather
than left standing at 40.

**HD's executable was read for the other half of this on 2026-08-18** and it
agrees: the disc has no shader file type at all, so every piece of compiled RSX
microcode is either in a `.rcsmaterial` or in `EBOOT.elf`, and the executable
carries 121 engine-owned `_vp`/`_fp` program names of its own - the bloom and
depth-of-field chains among them. **Those are `SHO` blocks too**: 126 of them
are linked into the executable, 62 paired name-to-blob by reading the arguments
at every shader-registration call site, and one of them decodes with this page's
own binding record - `(name hash, u16 type, u16 count, u16 register, 0xffff)` at
a 12-byte stride, found where the header's `+0x0c` count and `+0x12` offset say
it will be. So `SHO` is not a `.rcsmaterial` container; it is **HD's shader
container**, and the `.rcsmaterial` is one of two places it ships. It also names exactly eleven `.rcsmaterial`
files, all `fe/materials/cf_fetracks.rcsmaterial`, as field 6 of a 7-pointer
per-circuit front-end record that pairs a `.rcsmodel` with its material **by
path**. That is the only place the model-to-material binding is visible from
outside a `.rcsmodel`. See
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md#shaders-are-in-exactly-two-places-and-neither-is-a-file-type-on-the-disc).

There is also no *link* from a submesh to a material. The 0x80-byte descriptor
has no field that partitions the 123 stride-labelled submeshes into few enough
groups to be a material index: every candidate is either the buffer offsets
(123 distinct values over 123 submeshes, so "pure" by being unique) or a field
that mixes strides. Finding the binding would be the first step of that work,
not a detail of it.

### How it is recovered instead

Four rules. The first is the tightest oracle, the second is what filled in the
road, and the last two corroborate.

**Where a `.vex` node references the chunk** - every craft mesh, and the props
on a circuit - the node's authored bounding box settles it. `min <= max` holds
on 1,638 of 1,638 nodes (see [hd-status](hd-status.md)), and it is the mesh's
*tight* box: at the true stride the dequantised points touch all six faces to
within a quantisation step.

**The test is tightness, not containment**, and the difference is the whole
rule: requiring only "inside the box" admits a stride that is a divisor of the
true one, which walks a subset of the vertices and stays inside by construction.
Requiring the points to *fill* the box left **0 ambiguous** across all 89 meshes
of the three models measured.

Two refinements, both forced by real files:

- **A submesh that fits at no stride is skipped, not fatal.** Assegai's hull is
  one node of 19 submeshes; 18 fit at stride 22 and exactly one fits at none.
  Demanding all 19 rejected 22 for the whole node and the hull vanished. Why
  that one submesh reads differently is unrecovered. A majority of submeshes
  must still fit, so a stride cannot qualify by skipping almost everything.
- **A skipped submesh must not then be drawn.** It was, once: its attribute
  bytes came out as positions and stretched Assegai's bounding sphere from 7
  units to 130, framing the craft as a speck in an empty view.

**Where nothing references the chunk** there is no box, and a second rule
applies: take the stride whose decoded positions are most *compact*. A position
is a quantised `i16` and the attributes after it are normalised across the whole
`i16` range, so a wrong stride spreads points over the full +/-32768 - two orders
of magnitude wider than a real mesh, which occupies a tile.

The winner must beat the runner-up by a factor of two, which is a relative test
with no threshold to tune. Validated against the box oracle on the chunks where
both apply: it agrees on **77 of 78**, and all **70 of 70** of Talon's
Junction's box-labelled chunks clear the decisiveness bar, worst at 0.35 and
median 0.02. The one disagreement is a mesh where the box admitted 36 and this
picks 18 - half of it, so the box was matching every second vertex and the
compactness rule is the better answer rather than a worse one.

**Third, and the one that filled in the road: do the normals decode?** A
vertex's `+6` word is a packed unit vector, and that is a property nothing else
in the record has - read the same four bytes at the wrong stride and they are a
position, a texture coordinate or the tail of a previous vertex, and they come
out unit about a third of the time by chance. So the stride is the one whose
normals are unit.

**This is the only rule that is per-vertex evidence at every chunk size**, and
that is why it mattered: the box needs a `.vex` node, the buffer layout needs
two submeshes, and the compactness rule needs the true reading to be decisively
smaller than the wrong one - a margin that narrows on exactly the large chunks a
circuit's road is made of. Talon's Junction had **252 of its 983 chunks** drawn
as nothing, which is what the holes in the floor were.

| | before | after |
| --- | ---: | ---: |
| Chunks that decide a stride | 731 | **968** of 983 |
| Chunks drawn on the box-less path | 655 | **808** |
| Triangles | 411,617 | 445,630 |
| Collision-floor triangles with art within 4 units | 58.8 % | **71.6 %** |

It **disagrees with the other three on none** of the 729 chunks where more than
one answers. Two bars: the winner must be unit on at least 0.8 of the chunk's
vertices and beat the runner-up by 0.3, and both sit below the cluster the real
answers form and well above the ~0.3 a wrong stride scores. Dropping them to 0.7
and 0.2 decides three more chunks, so this is a knee rather than a tuned
threshold.

**And a fraction of eight is not evidence**: a chunk must carry at least 32
vertices to be judged this way. Without that floor the rule accepted eight-vertex
cards where 7 of 8 unit reads clear the bar by luck, and they decoded to
2,000-unit planes that stretched the circuit's bounding sphere from 1,909 to
8,761.

**Fourth, and structural rather than statistical: the file's own buffer layout.**
A mesh's vertex buffers are packed back to back, so the step from one submesh's
buffer to the next one's, over the first one's vertex count, *is* the stride -
arithmetic on two numbers the file states outright, with no oracle and nothing
decoded. Every consecutive pair votes and the majority wins.

This is the strongest evidence on this page, and most of its value is what it
says about the other two rules:

| Against | Agrees | Disagrees |
| --- | ---: | ---: |
| The authored `.vex` box | 19 | **0** |
| The compactness rule | 96 | **0** |

Two rules that share no input - one about where bytes sit, one about what they
decode to - agreeing exactly on 96 chunks is a better argument for the
compactness rule than the compactness rule can make for itself. It also decides
16 chunks compactness cannot, taking `talons_junction` from 718 to **734 of 983**,
and lifts the three referenced meshes whose box settled nothing.

It says nothing about a chunk with one submesh, which is most of a circuit's -
so `solve_stride_without_a_box` asks the layout first, the normals next and
compactness last, and the three are complements rather than alternatives. Every
chunk any of them decides has unit normals at the stride it chose: worst 1.00
over the layout's 91, 0.81 over the normals' 593, and **0.80 over compactness's
133**, which is the cross-check that matters because those two share no input.

**One thing about it is unexplained.** The step is exactly `count * stride` on 38
of 46 measured pairs and otherwise 16, 32 or 96 bytes *short* - always negative,
always a multiple of 16. Why a buffer starts before the previous one's declared
length ends is unrecovered, and the rule rounds rather than modelling it: the
error is at most 96 bytes over at least 33 vertices, well inside the 2-byte gaps
between the three widths, and a correction nobody can justify is worse than a
rounding everybody can see.

**Only 14, 18 and 22 are considered.** Searching every even width from 6 to 64
is strictly worse on a circuit: 814 of `talons_junction`'s 983 chunks still
choose one of the three, and the other 169 choose a width no measurement
supports and decode to spikes radiating out of the level. A width outside the
set is not evidence of a fourth format; it is the search finding nothing.

## A declared vertex count can overrun its own buffer, and the file is right

**Only the vertices a triangle names can testify about a stride.** Read
2026-08-18, and it recovered 1,053 submeshes the reader had been dropping.

`talons_junction`'s `tanker4c1Shape` declares 1,288 vertices in a buffer 23,152
bytes long. At the stride the chunk itself declares, 18, that is **32 bytes
short**: the last two vertices are read out of the *next* submesh's buffer and
dequantise to about 100 units from anything else in the mesh. The file is not
wrong about the count and the stride is not wrong either. **The index buffer
never references those two**, and RSX fetches a vertex when an index asks for it
rather than sweeping the array, so the original draws the same picture whatever
sits past the end.

That matters because the box test is a *tightness* test over a union of points,
and one point 100 units out ruins it. Two things were doing exactly that:
[`Mesh::solve_stride`], which rejected the true stride for a whole submesh, and
`Mesh::submesh_fits`, which the renderer asks per submesh before drawing and
which was answering "no" for submeshes the file draws perfectly well.

| | Judged on every declared vertex | Judged on the ones a triangle names |
| --- | ---: | ---: |
| Submeshes outside their node's box, disc-wide | **1,377** | **324** |
| Of `talons_junction`'s 117 | 17 | **0** |
| Of `amphiseum`'s 236 | 34 | **0** |
| Of `tech_de_ra`'s 154 | 17 | **0** |

12,624 node-addressed submeshes on 643 models, in
`a_submesh_is_judged_on_the_vertices_a_triangle_names`. On the picture it is
Talon's Junction going from 445,994 triangles to **466,497** with no other
change - a blimp's envelope, a tanker's hull and a lifter ship's flank that had
been silently absent. At the starting grid **32 of 1,175,040 pixels differ**,
which is the honest figure: what came back is above and behind that camera.

**A retraction.** The commit that landed this said the starting-grid frame was
*pixel-identical*. It was measured on two screenshots that were **fully
transparent** - `just play --race --screenshot` wrote alpha 0 over the whole
image, because a frame's alpha channel is the bloom mask and not coverage. That
was fixed separately and the comparison re-run; the conclusion is unchanged and
the evidence for it was worthless.

**324 are still outside and this page does not explain them.** They are not
spread evenly: the three circuits above have none at all, and the residue
clusters in `zone_2`, `zone_3`, `01_vineta_k` and `15_anulpha_pass`
(`wohdtrack_*`, `polySurface*`, `J_ALL_Ads_Frames_pCube*`). **They are not a
fifth vertex format**: every one of them declares a layout, and the widths they
declare are 22 on 161, 14 on 94, 18 on 68 and 38 on one - so it is an ordinary
declaration disagreeing with an authored box rather than a width nothing can
read. A drop from 1,377 to 324 is a reading that got better, not one that is
finished.

[`Mesh::solve_stride`]: ../../crates/rcs/src/rcsmodel/stride.rs

## Wipeout HD's road is not in the `.vex`

The finding that took the longest to see, and the reason a first render of
Talon's Junction showed sky traffic hanging over an empty void.

**All 126 `Mesh` nodes of `talons_junction/track.vex` are props** - blimps,
girders, tankers, skycars, the largest of them 166 units across. There is no
road among them. The circuit itself is in the **913 of 983** chunks that no
`Mesh` node addresses, and those chunks carry a **world-space** bias: their
biases span `-7009..2986` in x and `-7069..6759` in z, which is the
environment, where a referenced chunk's bias is a few units and needs its
node's transform.

So a PS3 model is read in two passes: the meshes its `.vex` places, and the
geometry no `Mesh` node addresses, drawn at identity. On a craft the second
pass is empty; on a circuit it is nearly everything.

**The pads are second-pass geometry with a first-pass-shaped reference**
(confidence 90, 2026-08-19). A `Weapon Pad` (`0x3be`) or `Speedup Pad`
(`0x3bd`) node carries a chunk hash at the mesh payload's own `+0x30`, but
the chunk it names is baked in world space anyway: on all 9 of Talon's
Junction's weapon pads and all 27 of Anulpha Pass's speedup pads, the chunk's
position extent sits beside the node's world translation (within the pad's own
footprint), never at the origin and never at double the translation - so
drawing it through the node's transform would land it off the world, and
drawing it at identity lands it on the track. 423 chunks across the disc's
circuits are addressed this way and by nothing else (239 `weapon_pads`
materials, 182 speedup materials and stragglers), and a reader that excludes
every *mentioned* hash from its second pass - as this project's did - draws
none of them. The node presumably exists for the gameplay trigger and the
armed/used visual state, both unread.

**56 of Talon's Junction's prop nodes address chunks the model does not have**,
and they stay honest absence - open item 6 below has the full sweep (18 of the
56 hashes live in *other circuits'* models, 38 nowhere, the mechanism that
would resolve one at runtime unrecovered). What that absence does **not** cost
the picture: the sky traffic visible on the circuit is the 33 world-space
`animating_traffic` chunks, which the second pass draws; the 56 nodes were
never the visible traffic.

The same pattern holds where a name in the unresolved set could plausibly *be*
the road: `03_track`'s `wohdtrack_*` node family looks like numbered track
segments by name alone, and is not one - checked 2026-08-20
(`the_unresolved_node_phenomenon_is_disc_wide`). 141 of 142 `wohdtrack_*`
nodes resolve normally in `track.vex`, 144 of 164 in `track_reversed.vex`; the
handful that do not are a small minority of a family that mostly resolves like
any other prop. And the road itself is not on the node path at all in either
direction: `track_reversed.rcsmodel` carries 1,272 second-pass (unaddressed,
world-space) chunks against 501 node-addressed ones, `track.rcsmodel` 1,213
against 477 - the same lopsided ratio Talon's Junction shows - and the two
second-pass sets share the *same* world-space bias extent (x `-1799..1292`, z
`-1195..1500`) in both directions, which is what one physical track baked once
per direction looks like. A name that reads as road geometry is not evidence
the road is missing.

**What drives it at runtime is unread.** HD moved visibility out of the `.vex`
into 28 `.pvs` files, and the obvious hypothesis is that a `.pvs` names the
chunks a section draws. Nothing here has read one.

## The material table, and which surfaces are see-through

**Confidence 85**, and it matters more than a material table sounds like it
should: without it a circuit's glass, cloud plates, fences and crowd billboards
are drawn as solid white sheets, and on Talon's Junction the largest surface in
the whole file is a 63-triangle `clouds` plate spanning 2,011 x 2,195 world
units. Seen from above, the track was one white blob.

### Where it is

The header names the table and a chunk indexes it:

```text
+0x2c  u32   material count
+0x30  u32   offset of the material offset table: `count` big-endian u32s
```

```text
material +0x04  u32   file offset of the material's own path, NUL-terminated
material +0x10  u32   state word; the low two bits are the transparency mode
material +0x14  u16   source blend factor
material +0x16  u16   destination blend factor
material +0x18  u32   alpha test comparison function
material +0x1c  f32   alpha test reference, in [0, 1]
```

The last two are read by the same decompiled RSX write that resolved
`Transparency::Mode2` - see
["Mode 2 is a plain alpha test after all"](#mode-2-is-a-plain-alpha-test-after-all-2026-08-31)
below.

Records are not fixed-length: consecutive offsets differ by 96 to 768 bytes,
most often 128. Everything above sits inside the smallest of them.

**A chunk's `+0x20` is the material index** - confidence 90. Range alone would
not settle that, since any small per-chunk count would pass; what settles it is
that it resolves *by name* on a craft whose mesh nodes say what each surface is:

| Assegai `Mesh` node | resolves to |
| --- | --- |
| `WindscreenShape` | `glass_texture_n.rcsmaterial` |
| `cockpit_screenShape` | `screen_test.rcsmaterial` |
| `FlashybitsShape`, `Port_lightShape`, `rowoflights_*Shape` | `emissive_bloom.rcsmaterial` |
| `ShipShape`, `PipesShape`, `PIlotShape`, `Airbrake_*Shape` | `diffuse_with_specular_from_alpha_n_vcol.rcsmaterial` |

It is **per chunk and not per submesh**: no `u32` in the 0x80-byte submesh
descriptor has all its values below the file's material count, and the chunk's
does on every chunk of every model measured.

### The low two bits of the state word gate the blend factors

**Confidence 88, and the evidence is structural rather than a reading of
material names.** Over all 15,762 materials on the disc the field takes three of
its four values, and what separates them is whether the blend factor pair beside
it varies at all:

| Low two bits | Materials | Distinct factor pairs | Reading |
| ---: | ---: | ---: | --- |
| 0 | 13,188 | 3, and 13,183 hold one of them | Opaque; the pair is the default nothing consumes |
| 1 | 2,362 | 7 | See-through, blended with the pair's own equation |
| 2 | 212 | 2, and 211 hold `0302`/`0303` | See-through as well; what else it selects is unrecovered |

Nothing writes seven distinct equations into a field a renderer ignores, and
nothing leaves 13,183 of 13,188 records holding one default pair in a field it
reads. The name evidence agrees and is *how the classification was found*, so it
pins a regression rather than confirming anything: `glass_texture`, `basicalpha`,
`fence_alpha`, `nr_crowd_bustle`, `clouds` and `scanlinebillboard` are all
see-through everywhere they appear, and `track_surface`, `track_wall` and
`weapon_pads` are opaque everywhere - as are `glasstestnoalpha`,
`tunnel_fx_noalpha` and `diffuse_with_specular_from_alpha`, whose names carry a
see-through word and whose state word does not.

**The fourth encoding never appears**, which is why `Transparency` has three
members and the accessor returns an `Option` rather than defaulting.

**What separates value 2 from value 1 is left unnamed - deliberately.** The
obvious hypothesis is an alpha test: `jd_alphalambert_alphatest`,
`jd_alphalambert_test`, `fence_alpha` and `nr_crowd_bustle` are all here and are
exactly what a cutout is for. It does not hold up. `hd_bombfire_glow`,
`zone_death_electricity` and `cf_alpha4glow` are here too and are not cutouts,
and the equation cannot tell the two apart either since 211 of the 212 carry the
same alpha-over pair as the blended mode. Below 70, so the name is not written -
see the [confidence rubric](../reverse-engineering/confidence-rubric.md).

### Bit 7 tracks the texture, not the material name - confidence 84

**A census over the same 12 circuits (2026-08-25) found bit 7 of the state word
carried by exactly eight material names disc-wide**: `animating_traffic`,
`cf_tree`, `jd_alphalambert`, `lambert`, `lambert_alpha_02`, `scanlinetext`,
`uv_anim_diffuse_alpha` and `uv_anim_diffuse_alpha_emissive`. That reads like a
per-material-name flag, and it is not one: `cf_tree` on `amphiseum` resolves to
ten separate material records, and the one of the ten sampling a plain bark
texture (`ol_treemonzabrownshowbark_c.gtf`) is also the one record whose state
word does not carry bit 7 - the other nine, all sampling a texture named with
the disc's own `_atoc` suffix (`ol_tree_birch_1_atoc.gtf` and its siblings),
all do. **The bit follows the texture assignment within a single material
name, not the name itself.**

Disc-wide, across all 16 circuits, the same split holds in both directions:

| | Carries bit 7 | Does not |
| --- | ---: | ---: |
| Samples an `_atoc.`-suffixed texture | 113 | 2 |
| Does not | 32 | (the remaining opaque bulk) |

The two exceptions in the top row are both `wes_billboardholographicscanlines`
(`amphiseum`, `modesto_heights`), which sample an `_atoc` texture without the
bit set. All 32 exceptions in the bottom row are `lambert` - the disc's most
overloaded material name, reused on five circuits for gridwork, rail, grate,
window and distant-crowd geometry (`wes_and_crowd.gtf`) that carries the bit
without an `_atoc` texture to go with it. Measured with a purpose-built sweep,
`hd_atoc_check` (`crates/render/examples/hd_atoc_check.rs`), alongside
`hd_state_census`'s existing per-name tabulation - both walk the same 16
circuits' `track.vex` siblings that `hd_state_census` already reads.

**This is the same evidence class the low-two-bits reading above was scored
at**: an exact split holding across many real files, with the handful of
exceptions concentrated in one over-reused name rather than scattered noise.
It is capped at 84 rather than reaching that reading's 88 because the split is
not perfect (113/145 and 113/115, not all of either) and the `lambert`
exceptions are unexplained rather than merely unmeasured.

**What the bit actually selects is a separate, weaker claim - confidence 55.**
Every `_atoc`-suffixed texture on the disc is foliage, a distant-crowd
silhouette, a traffic sprite or on-screen text/signage
(`leveltext_atoc.gtf`, `air_traffic_test_a_atoc.gtf`,
see [rcsmaterial.md](rcsmaterial.md)) - exactly the content classes real-time
renderers use alpha-to-coverage for, to cut out detail without depth-sorting
transparency. That is a reasonable inference from the content, not a reading
of the executable: no RSX register write has been tied to this bit, and the
name `_atoc` itself is an artist convention that could mean anything from
"alpha test" to "alpha-to-coverage" to a project-local shorthand nobody wrote
down. **Neither this bit nor `Transparency::Mode2` above is wired into
`oag_render`.** Wiring alpha-to-coverage specifically depends on HD's render
path already running multisampled: `mesh_render::build`'s `sample_count` is 1
outside a race and the `[graphics] anti_aliasing` MSAA count inside one, so
the feature is a conditional no-op rather than definitionally unwireable -
`wgpu::MultisampleState::alpha_to_coverage_enabled` takes effect only when
`count` is greater than 1.

### Mode 2's six materials carry no fragment-program kill (2026-08-31)

**The microcode route the previous section left open comes back empty.**
Every fragment block of all six mode-2 material names
(`nr_crowd_bustle`, `jd_alphalambert_test`, `fence_alpha`,
`emissive_alpha_heathaze_test`, `cf_alpha4glow`,
`uv_anim_diffuse_alpha_emissive` - 86 blocks total, decoded end to end with
[`scripts/ps3-microcode.py`](../../scripts/ps3-microcode.py)) was checked for
a `KIL` instruction or a comparison op (`SLT`/`SGE`/`SEQ`/`SNE`/`SGT`/`SLE`)
against anything but the shared boilerplate below. **`KIL` never appears.**
The only comparisons present are `SLT` against parameter `0xa410aa44`
(`zoneColourTint`, per
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md)) and a family of
`ADD_SAT ..., -0.5` range-compresses - and both are present, identically, in
a mode-0 material (`track_surface`) and a mode-1 material (`glass_texture`)
checked as controls. Neither is mode-2-specific; both are ordinary shared
fragment boilerplate (a zone-tint fog term, a signed-normal decode), not an
alpha-cutout threshold. Confidence 80 that no cutout comparison exists in
these six materials' fragment code specifically - lower than the container
facts on this page because it is an absence claim over 86 blocks, not a
positive pattern match, and the three opcodes still unnamed in this corpus
(`0x3b`/`0x3d`/`0x3e`, see [rcsmaterial.md](rcsmaterial.md)) are read as
normalise/rsq helpers from their position rather than confirmed, so a
disguised comparison hiding in one of them cannot be fully ruled out.

**This refutes the premise the previous section's open item was written
against, not just the specific guess.** A programmable discard is something
the fragment microcode would show directly; its absence across every mode-2
material means mode 2 is not implemented as a shader-side cutout at all.
What is left is fixed-function GPU state - an `ALPHA_REF`/`ALPHA_FUNC`-style
comparison the RSX applies after the fragment program runs, the same class of
mechanism bit 7 above is suspected of selecting (alpha-to-coverage). Neither
is visible in a fragment program by construction: both are register writes
made around the draw call, not instructions inside it. Resolving what mode 2
selects now needs the same evidence bit 7 needs - a decompiled RSX method
write in `ps3-hdfury-eu`'s Ghidra database - not more microcode reading.

### Mode 2 is a plain alpha test after all (2026-08-31)

**The RSX-method-write search the previous section pointed at found it.**
`Material_ApplyRenderState` (`0x005d8f68`,
[material-state.md](../ghidra/functions/ps3-hdfury-eu/material-state.md)) is
the caller that turns a material record into GPU state, and it reads
`Material::state` bit 0 into `NV4097_SET_BLEND_ENABLE` and bit 1 into
`NV4097_SET_ALPHA_TEST_ENABLE` - two different fixed-function features, not
two variants of one. `Transparency::Blended` (bit 0) blends with the factor
pair; `Transparency::Mode2` (bit 1) alpha-tests, with the comparison and
reference at the two new fields above, fed straight into
`NV4097_SET_ALPHA_FUNC`/`SET_ALPHA_REF` by a sibling function this pass also
named and read (`Rsx_SetAlphaFunc`). That is why the blend equation never
separated the two modes - 211 of 212 mode-2 materials share mode 1's
`0302`/`0303` pair, because the pair is irrelevant to mode 2's actual
mechanism.

**And the values turn out to be constants, not per-material authoring.**
Extending `oag_rcs::rcsmodel::Material` with the two fields and sweeping
every material name across all 16 circuits and all three `PSARC` archives:
`alpha_ref` is `0.5` everywhere, and `alpha_func` takes exactly two values
disc-wide, `0x0201`/`GL_LESS` and `0x0204`/`GL_GREATER` - every one of the
six `Transparency::Mode2` material names reads `GL_GREATER`. **Mode 2 is
the "obvious hypothesis" this page and the handover thread both tried and
believed refuted**: a plain `GL_GREATER`/`0.5` alpha-test cutout.

**The refutation itself was the error, not the hypothesis.** It rested on
`crowd_avatars_22x4.gtf`'s alpha running `0..255` at a mean of `120` and
concluded a `0.5` threshold "discards most of it". Histogramming the same
texture end to end (`oag_texture::gtf`) shows why the mean was the wrong
statistic: **52.9% of texels sit at alpha `0`, 47.1% at `255`, nothing
between.** `GL_GREATER` against `0.5` keeps exactly the 47.1% - the crowd
figures - and drops the transparent background, which is correct cutout
behaviour, not erasure. A mean near the threshold only means "discards most
of it" if the distribution is roughly uniform around that mean; a
hand-authored cutout mask is built to be the opposite of uniform, and this
one is a clean bimodal split. `nr_crowd_bustle`'s own fragment microcode
confirms this is genuinely the texture the alpha test sees, with nothing
combined in between: block `#3` samples sampler hash `0x11cb4f74` at unit 1
into the register that becomes output alpha, and that hash is exactly
`Material::texture_sampler` for `crowd_avatars_22x4.gtf` on this material.

### Mode 2 is wired, and it is not subtle on every material (2026-08-31)

**`Material::blend()` answers `Blend::AlphaTest` now**, a variant of its own
rather than the `Blend::Factors` it shared with `Transparency::Blended`, and
`oag_render::mesh::rcs` puts those chunks in `Model::alpha_tested_draws` -
depth write on, blending off, drawn between the opaque pass and the blended
one. `wgpu` has no fixed-function alpha test, so the comparison is a
shader-side `discard` in `mesh.wgsl`; the **reference travels as a pipeline
override** (`alpha_test_ref`, off `Model::alpha_test_ref`) rather than as a
constant in the shader, so the number drawn with is the disc's own and a
disc that authored a different one would draw with that instead. Only
`GL_GREATER` is reproduced - `mesh::rcs::cutout` counts a material asking for
anything else in `Report::cutout_unread` and leaves it opaque, because
drawing a `GL_LESS` cutout through a `GL_GREATER` shader inverts it. Zero on
every circuit measured.

**Whether the cutoff changes the picture depends on the material, and for two
of them it changes it a lot.** `crates/render/examples/hd_mode2_alpha.rs`
histograms the channel each material's own microcode says its coverage comes
from - not entry 0's `a` blindly - across every circuit of all three
archives. **103 mode-2 records under 8 distinct names, every one
`GL_GREATER`/`0.5`, every one taking coverage from the first texture's `a`:**

| material | records | chunks | texels strictly between 0 and 255 | kept by `> 0.5` | mean alpha |
| --- | --- | --- | --- | --- | --- |
| `cf_alpha4glow` | 15 | 38 | 67.07 % | 36.2 % | 0.3865 |
| `jd_alphalambert_test` | 5 | 10 | 19.92 % | 49.0 % | 0.4860 |
| `emissive_alpha_heathaze_test` | 4 | 54 | 0.23 % | 72.9 % | 0.7288 |
| `fence_alpha` | 11 | 11 | 0.13 % | 24.3 % | 0.2429 |
| `nr_crowd_bustle` | 33 | 1,123 | 0.11 % | 48.6 % | 0.4858 |
| `jd_alphalambert_alphatest` | 2 | 45 | 0.00 % | 44.7 % | 0.4469 |
| `lambert` | 32 | 100 | 0.00 % | 44.8 % | 0.4476 |
| `uv_anim_diffuse_alpha_emissive` | 1 | 65 | 0.00 % | 0.8 % | 0.0077 |

**Two of the eight names are new against the six the mode-1/mode-2 census
above lists**, and the difference is scope rather than a correction: that
sweep covered 12 circuits in both directions, this one covers all 16
environment directories of all three `PSARC` archives. `lambert` and
`jd_alphalambert_alphatest` are drawn either way - 100 and 45 chunk surfaces
name them.

So the "colour is close either way" reasoning the crowd's own histogram
supported **does not generalise**: it holds for six of the eight, and
`cf_alpha4glow` - two thirds of its texels on a gradient - is a different
picture blended than tested.

**Measured against a frame.** `just view <image>:...PSARC --mesh
/data/environments/amphiseum/track.vex` reports 274 see-through chunks before
and 182 see-through plus 92 cutout after. Rendering one crowd slot alone
(`OAG_ONLY_SLOT=574 OAG_ALBEDO_ONLY=1`, one variable between the two frames)
is where it shows: blended, the amphitheatre's stands are a flat grey sheet
of 44,313 non-background pixels; tested, 24,290 - 55 % of them, against the
48.6 % of the texture's own texels that clear the reference - and they carry
the rows of individual avatars the texture actually holds instead of a smear.
On `01_vineta_k`'s isolated `cf_alpha4glow` the surviving strips draw at full
intensity rather than washed into the background: 19 of the frame's 25
changed pixels are newly lit.

**And a cutout disappears under minification, which is worth stating rather
than rediscovering.** At a framing that puts all 25 of amphiseum's crowd
chunks in one 960x720 capture (radius 902 against radius 213 for the single
slot above), the same comparison goes 7,640 non-background pixels blended to
**8** tested - the crowd is simply gone. The mean-alpha column is why: as a
surface minifies its footprint covers more and more of the texture, so the
filtered sample converges on the texture's mean, and **seven of the eight
means sit below `0.5`** - `nr_crowd_bustle`'s at `0.4858`, a hair under. Once
every sample is that mean, `GL_GREATER` against `0.5` fails everywhere at
once rather than thinning gradually.

**Confidence 75 on that explanation**, from the arithmetic and two framings
of the same geometry rather than from reading the sampler: nothing here has
checked which mip level `mesh.wgsl`'s sampler actually selects, and the
prediction it makes for `emissive_alpha_heathaze_test` - mean `0.7288`,
so it should go *solid* under minification rather than vanish - has not been
captured. The behaviour is the RSX's too, and whether the original mitigates
it is unread; the disc's `..._atoc.gtf` texture names point at
alpha-to-coverage, on exactly the crowd, foliage and traffic surfaces this
affects. That is `Material::state` bit 7's open question, not this one.

### The factor values, and which are mapped

**Confidence 70 on the enum itself.** The four values the disc uses are
`0x0001`, `0x0300`, `0x0302` and `0x0303`, which are exactly `GL_ONE`,
`GL_SRC_COLOR`, `GL_SRC_ALPHA` and `GL_ONE_MINUS_SRC_ALPHA` - the numbering the
RSX inherits from OpenGL. Four distinct values landing on four meaningful
members of a published enum is strong, but nothing here has read the code that
consumes them, so it is short of what an executed branch would carry.

`oag_rcs::rcsmodel::Material::blend` returns **both** factors, each as a
`Factor` member naming one of those four values. A value outside the four would
come back `Blend::Unmapped`; **none is**, on all 15,762 materials
(`every_factor_the_disc_uses_is_one_of_the_four_named_ones`).

**This used to key on the destination alone and drop the source** - `dst ==
0x0001` read as additive, `dst == 0x0303` as alpha-over, everything else
unmapped - so that every see-through surface arrived at a renderer as one of
Pulse's three `vex::BlendClass` members. Saying `BlendClass` has no member for
the source distinction was correct; *dropping* the source because of it was not.
`oag_render::mesh_render`'s two transparent pipelines both hardcode a source
factor of `SrcAlpha`, so every material that authors something else was drawn
with an equation the file does not ask for.

**How much that is, measured over every see-through material on the disc**
(`a_see_through_material_does_not_always_author_a_source_of_src_alpha`):

| src / dst | Materials | Under the old reading |
| --- | ---: | --- |
| `0302`/`0303` | 1,859 | alpha-over - correct |
| `0302`/`0001` | 348 | additive - correct |
| `0001`/`0001` | 144 | additive, so every texel scaled by its own alpha before being added. **Unweighted additive in the file** |
| `0001`/`0303` | 142 | alpha-over, so the colour multiplied by alpha a second time. **Premultiplied alpha in the file** |
| `0300`/`0001` | 57 | unmapped, so **drawn opaque** |
| `0300`/`0302` | 22 | unmapped, so **drawn opaque** |
| `0001`/`0302` | 1 | unmapped, so **drawn opaque** |
| `0302`/`0300` | 1 | unmapped, so **drawn opaque** |

**367 of 2,574**, 14 %. On Talon's Junction it is 96 of that circuit's 353
see-through chunks authoring `0001`/`0001` alone - its trackside advert boards
(`scanlinebillboard`, `loopmaterial`), which were being dimmed by their own
alpha. The equation is the file's, so they are brighter now and that is the
correction rather than a tuning choice.

**What changed in a picture is narrower than that table, and measured rather
than assumed.** The 81 materials in the last four rows used to come back
`Unmapped` and be drawn *opaque*, so making them see-through could newly lose
depth write on geometry that had it. It does not, on either circuit that carries
them: `Report::see_through` is **273 before and after on Talon's Junction and 190
before and after on `12_sol_2`** - the circuit with the most of them, 5 materials
over 73 chunks - and `12_sol_2`'s capture moves **one pixel**. Their chunks are
not in the drawn set at all, so what actually moves is the *equation* on chunks
that were already blended: Talon's Junction's isolated advert boards move 5,819
pixels and get brighter.

`oag_render::mesh::rcs::blend_state` translates a pair into a
`wgpu::BlendState` - the identity mapping on the four names, `Add` for the
operation because the RSX's blend-equation register is a separate field nothing
here has read and `GL_FUNC_ADD` is what it holds at reset. `mesh_render::build`
then creates one pipeline pair per **distinct state the model's own draws name**,
and `TransparentPipelines::select` prefers `DrawCall::blend_state` over
`DrawCall::blend`. A Pulse model authors no factor pair, leaves `blend_state`
`None` and takes the recovered-class path exactly as before.

### The texture a material paints with

**Confidence 92**, and it is in the clear:

```text
material +0x58  u32   file offset of a `.gtf` path - the texture
material +0x60  u32   that texture's sampler name hash
material +0x78  u32   a second `.gtf` path, on the materials that carry one
material +0x80  u32   that texture's sampler name hash
```

**The two hash words were read 2026-08-24** and they change what a texture slot
*means*: the hash, not the slot's ordinal, is what a resolved shader variant
maps to a texture unit, and on 90 % of the disc's readable slots the two
disagree. Nothing is rebound on it yet. The measurement, the counter-example and
the confidence score are in
[rcsmaterial.md](rcsmaterial.md), "A texture slot names its sampler, and the
slot's ordinal is not its unit".

Every material of both models names a first texture - 4 of 4 on Assegai, 442 of
442 on Talon's Junction - and they are the right ones by inspection:
`WindscreenShape`'s material names `assegai_glass.gtf`, the circuit's name
`talons_support_struts.gtf` and `tunnel_fx_diffuse.gtf`. Drawn through
[`oag_texture::gtf`](hd-status.md#what-is-genuinely-new), all 442 of the
circuit's decode.

**The second slot is not one thing, and the first sample said it was.** Assegai
carries exactly two and both end `_n`, which reads as a normal map. Four
materials off the circuit refute it outright:

| material | first texture | second |
| --- | --- | --- |
| `diffuse_with_specular_from_alpha_n_vcol` | `assegai_tp_1024.gtf` | `assegai_n.gtf` |
| `tunnel_fx_noalpha` | `tunnel_fx_diffuse.gtf` | `tunnel_fx_emissive.gtf` |
| `clouds` | `clouds_new.gtf` | `cloud mask.gtf` |
| `diffusewithalphachannel` | `holebaralpha.gtf` | an `lmaps/...-lmap.gtf` |

A normal map, an emissive map, a coverage mask and a lightmap - one field, four
uses, selected by the `.rcsmaterial`'s shader.

**One of the four is identified and is drawn** (2026-08-18): a path under the
circuit's own `lmaps/` ending `-lmap.gtf` is its baked lighting atlas, and
`Material::lightmap` answers it. Four signals agree, the sharpest being that
**every chunk naming such a material declares a `lightmapUV` attribute and none
declares one without it** - 3,584 chunks over 12 circuits, checked by
`a_lightmap_and_a_lightmap_coordinate_come_together`. `oag_render` samples it
through that attribute and multiplies; the *multiply* is this project's
assumption and the load report says so.

**The other three are not**, and [rcsmaterial.md](rcsmaterial.md) is why: a
`.rcsmaterial` is a container of shader *variants*, twenty fragment programs of
one material disagree about which unit a sampler sits at, and what selects a
variant is unread. So a surface whose coverage lives in that slot still paints
solid: Talon's Junction's cloud plate is the one that shows.

**The obvious shortcut was tried and does not work** (2026-08-18). The
`.rcsmaterial`'s `SHO` blocks carry a parameter table -
`(name hash, u16 type, u16 count, u16 register, 0xffff)` at a 12-byte stride,
identified and cross-checked against the executable's own copies in
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md#what-was-deliberately-not-read) -
so the hope was that a material's parameters would name its samplers and settle
which slot is what without disassembling anything. They do not:

- **The table does not discriminate.** `diffusewithalphachannel`, whose second
  texture is an `lmaps/*-lmap.gtf`, has a **byte-identical hash set** to
  `diffuse_with_specular_from_alpha`, which has no second texture at all; the
  same holds for `clouds` (a mask) against `billboarddiffuse` (nothing).
- **Nothing in it is sampler-shaped.** Across three materials' 582 well-formed
  records there are three type codes (`0x201`, `0x203`, `0x204`) and the
  registers are `0x0100`, `0x0104`, `0x01cc`-`0x01d3` and `0xffff` - constant
  registers, not the small texture-unit indices a sampler binds to.

So this is a **constant** table, and the second slot's role is still in the
microcode. **The cheap reopening was run and is closed**: `0x003b31c8`, which
`renderer.md` listed as a second binder, is not one - it is the Detonator scoring
subsystem's static initialiser, and that page is corrected. There is one binder,
it walks this table, and this table holds constants.

What the check did turn up is worth more than the answer it was looking for:
`Crc32_HashString` (`0x005a2090`) is the name hash these records key on, plain
CRC-32 with the complement stored, confirmed by `~crc32("viewProj")` landing on
the one parameter every material declares. Naming a parameter is a wordlist
problem now. It did not name any sampler, because there is no sampler record
here to name.

### What the renderer does with it

**A see-through chunk is drawn blended, with the equation its material names.**
`oag_render::mesh::rcs` puts it in `Model::transparent_draws` and sets
`DrawCall::blend`, and the alpha comes from the `.gtf`'s own RGBA.

That was not possible when this table was first read. With no texture there was
no alpha, so `alpha = 1.0` made alpha-over paint exactly the opaque pixels while
dropping depth write and additive blow every glass panel to white - and the
stopgap was to *leave the chunk out*, an honest absence in place of a wrong
picture. `.gtf` closed it; the stopgap is gone.

**Two things still paint solid, both named rather than worked around:**

1. **A material whose coverage is in the second texture**, the cloud plate above.
   It is classified see-through and blended correctly, and its first texture has
   no alpha to blend with.
2. **A material whose `.gtf` does not decode.** `Texture::to_rgba` refuses the
   RSX's Morton-swizzled layouts and cubemaps - 53 of the disc's 7,333 files -
   and such a draw binds the renderer's white 1x1. It is deliberately **not**
   blended, since blending a white 1x1 with depth write off is worse than
   leaving it opaque, and `Report::untextured` counts it so a partly-textured
   circuit cannot read as a working one.

## A chunk header is 0x20 bytes and then a SURFACE record

**This closed the largest known gap between what this parser read and what the
game drew, on 2026-08-25.** Before it, `Mesh::material` was one index per chunk
and the whole chunk was painted with it; a quarter of the disc's chunks name
several materials, and the geometry belonging to the others was drawn by
nothing at all.

The engine walks a **per-chunk surface table** and indexes the material table
per *surface* - `0x003faf00` and `0x003fb330` both do
`materialTable[*(int *)surface]`, and the record it points at has the same
shape as the tail of a chunk header:

```text
chunk   +0x06  u8    layout, shared by every surface
chunk   +0x10  u16   surface count
chunk   +0x18  u32   surface offset table
chunk   +0x20        SURFACE RECORD 0, 0x40 bytes:
surface  +0x00  u32   material index
surface  +0x10  f32x3 bias
surface  +0x20  f32x3 scale
surface  +0x30  u32   submesh count      (Described)
surface  +0x34  u32   -> its own descriptors, always +0x40
surface  +0x38  u32   -> vertex declaration
surface  +0x40        its 0x80-byte submesh descriptors
```

So every field this module used to call a chunk field - the material at
`+0x20`, the bias at `+0x30`, the scale at `+0x40`, the count at `+0x50`, the
declaration at `+0x58`, the descriptors at `+0x60` - is `+0x00`, `+0x10`,
`+0x20`, `+0x30`, `+0x38` and `+0x40` of a record that recurs elsewhere in the
file for every surface past the first.

**A surface is therefore a `Mesh`.** `rcsmodel::Mesh::extra_surfaces` holds the
rest and `Mesh::surfaces` walks all of them, so every reader written against a
chunk works on a surface unchanged - which is what let 213 call sites stay as
they were.

### What the disc says

Pinned by `crates/rcs/tests/rcsmodel_surface_ground_truth.rs`, over all 643
models and 41,861 chunks:

| | |
| --- | --- |
| chunks whose surface table opens at `chunk + 0x20` | **41,861 of 41,861** |
| chunks declaring more than one surface | **9,891 (24%)** |
| ...of those, naming a material other than their first's | **9,891, all of them** |
| submeshes on first surfaces / on the rest | 50,873 / **25,972** |
| triangles on first surfaces / on the rest | 17,505,484 / **7,111,578 (+40.6%)** |
| extra surfaces naming a material outside the table | **0** |

Amphiseum's chunk 9 declares eleven surfaces naming materials
`[6, 7, 626, 633, 10, 635, 12, 13, 14, 15, 16]`.

**Drawing them is what the renderer now does**, in both of
`oag_render::mesh::rcs`' passes. On Talon's Junction that took the scene from
471,024 triangles to **809,247** and 871 draw calls to 1,792; on Anulpha Pass
44% of the start-line frame changed, and the barrier hex panelling and the
silver structure on the right of the reference frame appeared for the first
time.

**A surface is not a submesh**, which is why the first attempt at this - assume
one submesh per surface and reassign materials - would have been wrong: only
503 of the 9,891 multi-surface chunks have equal counts. Each surface carries
its own descriptors, and that is what makes them geometry rather than a
relabelling.

**Two guards worth keeping.** A surface that will not read is skipped rather
than failing its chunk: the first one is the chunk itself and its failure is a
real error, but a later one failing costs that surface alone. And the node
pass's `submesh_fits` box test is applied **only to the chunk's own surface** -
a later surface has its own bias and is not inside the node's authored box in
the first place, so testing it there would drop it as a stray for being exactly
where it belongs.

## Byte `+0x07` says which space a chunk's positions are in

**The `kk` of the `00 nn LL kk` word at `+0x04`**, which this page carried for
months as "takes the values `01` and `02` for a reason nothing here has
distinguished". The engine branches on it (`*(char *)(chunk + 7) == '\x02'`,
found in the same read that turned up the surface table), and the disc says
what the two values mean:

- **`1` - world-baked.** 33,088 chunks. Positions are already in world space
  and no node transform applies.
- **`2` - node-local.** 8,773 chunks. Positions are in the chunk's own space
  and a `.vex` node places it.

Nothing on the disc carries a third value. Read as `rcsmodel::Space`.

### Three independent lines, because no one of them settles it

**The categories.** Every chunk of `data/ships` (2,163) and `data/fe` (1,136)
is node-local; every chunk of `data/pvsblocker` (75) is world-baked. Within
`data/environments` the split is 32,955 world-baked against 4,703 node-local -
the road and the scenery against the props.

**Node addressing.** 8,769 of the 8,773 node-local chunks are addressed by a
`.vex` `Mesh` node, four are not. World-baked chunks mostly are not (30,407 of
33,088), and the 2,681 that *are* named by a node are the population the
paragraph below is about.

**The bias, which is independent of the byte.** A world-baked chunk's bias is
its world position; a node-local one's is near the origin. Over
`data/environments`: world-baked `|bias|` has a median of **552.9** with
**one** of 32,955 under a unit; node-local a median of **2.7** with 35 % under
a unit.

Pinned by `the_space_byte_separates_world_baked_geometry_from_node_local` in
`crates/rcs/tests/rcsmodel_surface_ground_truth.rs`.

### It replaces a heuristic this project invented, and the heuristic was wrong

`oag_render::mesh::rcs::is_world_baked` decided the same question by carrying a
node's authored box through its transform and asking whether it landed within
**one world unit** of the chunk's bias - a tolerance chosen here, documented at
confidence 88, with a long note attached about the cluster gap that justified
it.

Over all 11,450 `Mesh` nodes on the disc whose chunk is in their own model, and
judged by the bias:

| | agrees with the bias |
| --- | --- |
| the `+0x07` byte | **94.4 %** |
| the geometric heuristic | 43.2 % |

And on **both** directions of disagreement the bias sides with the byte: of the
5,724 chunks the byte calls node-local and the heuristic called world-baked,
5,345 have a bias under 50 units and the median is **1.2**; of the 775 the
other way, only 27 are under 50 units and the median is **544.3**.

So `is_world_baked` now reads the byte and keeps the geometric test only for a
value that is neither 1 nor 2, which nothing on this disc has. The effect is
small and targeted, which is what a correction to a mostly-right heuristic
should look like: Talon's Junction goes from 60 to 70 node-drawn chunks with
**zero** now misfiled as world-baked, and its start-line frame is
byte-identical; Anulpha Pass goes from 101 to 113 and 0.52 % of its frame
changes.

### The executable confirms the mechanism

`Scene_RefreshNodeMatrices` (`0x003fb330`) walks every chunk of the scene and,
**only where this byte is `2`**, follows the chunk's runtime block to a linked
scene node, refreshes that node if its dirty bit is set, and copies four
16-byte rows - a 4x4 matrix - into the head of the block. A `1` chunk is
skipped entirely; the loop body never runs for it.

So the byte says **whether a chunk's world transform is re-read from a node
every frame**, which is the runtime face of the by-hash link a `.vex` node
makes. Confidence 82. See
[visibility.md](../ghidra/functions/ps3-hdfury-eu/visibility.md), "The chunk
kind byte".

**It is genuinely three-way**, which is why `Space::Unknown` exists rather than
a boolean: the load-time switch tests `1`, then `2`, then falls through to a
third path that no chunk on this disc takes.

**And a clean negative about what it is not.** Across both switch sites, no arm
reads the vertex declaration, the index buffer, the stride, the surface count
or anything under the surface record - all they do is store command words into
an emission cursor. So the byte cannot be a triangle-list-versus-strip, an
index-width or an attribute-set selector, and none of this format's geometry
decoding turns on it. Confidence 82.

**Byte `+0x05` - the `nn` of the same word - is read by nothing.** The single
instruction in the geometry path that loads it is an unrolled byte-by-byte
block copy carrying `+0x04` through `+0x08` alike, which is not a semantic
read. That closes the old note that it "counts chunks (and is `0xff` on many)"
as a clean negative, confidence 85.

## What is still open

Named explicitly, with what each would take.

1. **Where the vertex stride is *declared*.** Still nowhere anyone has found -
   and no longer the `.rcsmaterial`, which was read and is compiled RSX shader
   code. Three rules recover the number without it, so this is now a question
   about the format rather than a blocker. The remaining routes are
   disassembling a shader's microcode, or HD's own executable.
2. **What the second texture at a material's `+0x78` is for**, per material,
   for the three uses that are not the lightmap. The lightmap is identified and
   drawn; a normal map, an emissive map and a coverage mask are not, because
   the selector is a shader *variant* and what picks one is unread - see
   [rcsmaterial.md](rcsmaterial.md). A surface whose coverage lives there still
   paints solid.
3. **What `+0x0a` holds on stride 18**, and what `83 XX` selects at all. The
   second used to be the lead on the first and is now ruled out: the field
   follows the width, not the byte. `XX` runs `07` to `11` and is the only part
   of the descriptor that varies and is unaccounted for.
4. **Why a vertex buffer sometimes ends 16, 32 or 96 bytes short** of
   `vertex_count * stride`. 8 of 46 measured pairs, always negative, always a
   multiple of 16. Harmless to the layout rule, unexplained all the same.
5. **What byte `+0x07` of a chunk selects**, and why the chunk counter above
   the layout selector reads `0xff` on many chunks. Both left over from the
   two-layout finding.
6. **56 of Talon's Junction's 126 `Mesh` nodes address no chunk**, and the
   shared-props-archive guess this page used to carry is **refuted**. Every
   `.rcsmodel` under `/data/environments/` sits in one of exactly 16
   directories - **12 circuits and Zone mode's 4 `zone_N` maps**, corrected
   2026-08-20 from "16 circuits"; there is no seventeenth for anything they
   share. What the sweep found instead, over all 643 files and 39,414 chunks:
   **18 of the 56 hashes are carried by other circuits' models** - the sky
   traffic by `amphiseum` and `tech_de_ra`, and eight `pCube*` nodes by three
   circuits sharing nothing else with Talon's Junction. **13 of those 18 have a
   donor whose geometry is demonstrably the node's** - judged at the stride the
   donor chunk declares, over the vertices a triangle names, and required to
   *fill* the node's box rather than merely sit inside it - and 5 have none,
   the three `pCube*` among them. The remaining 38 are nowhere on the disc
   under this addressing. The hash is **not** a hash of the node's name under
   `wad::hash_name`, FNV-1a, DJB2, SDBM or CRC32, over 7 name variants (raw,
   both cases, Maya-path-qualified, `Shape`-stripped and `Shape`-appended) -
   checked against all 70 of Talon's Junction's *resolved* pairs, not only the
   56 unresolved ones, so the negative is not an artifact of only ever testing
   names that were never going to hash to anything.

   **Disc-wide, not Talon's-specific** (`the_unresolved_node_phenomenon_is_disc_wide`,
   2026-08-20). Every one of the 12 circuits was censused, both directions:
   `04_chenghou_project` and `10_sebenco_climb` resolve every node they carry,
   most circuits sit under 15 unresolved out of several hundred, and Talon's
   Junction's 56 of 126 is the outlier, not the typical case. That retires the
   DLC-packaging framing this finding could otherwise have suggested - it is
   not specific to the Fury pack's cook, or to Talon's Junction's own.

   **The donor identity is now confirmed a second, independent way**
   (`nine_of_the_thirteen_confirmed_donors_are_index_identical`, 2026-08-20),
   stronger than the box-fill test that first found it: **9 of the 13 donor
   hashes are index-byte-identical, at equal vertex count, in every file that
   carries them.** The other 4 are not a counter-example - each splits into
   exactly two vertex-count tiers across its files (`Skycar_1Shape` 40
   vertices in one, 848 in another; `shipintersteller1Shape` 8 vs 1280;
   `WesSkycar_C1Shape` 48 vs 824; `HyperContintentCraft1Shape` 72 vs 3568),
   the same key naming two levels of detail of the same asset rather than two
   unrelated meshes. That is what a key derived from the *source* asset would
   produce, and it is the explanation this page never had for why
   `Skycar_1Shape` fills Talon's Junction's box in `tech_de_ra` (the 848-vertex
   copy) and not in `amphiseum` (the 40-vertex stub): both circuits carry
   *something* under that key, at whatever detail their own cook baked.

   So identity is settled two ways now, and it settles as **a shared
   per-title asset key, independently cooked per circuit** - not "the game
   resolves a node against another circuit's file at runtime", which was
   this page's original framing and which the evidence never actually
   distinguished from "these are dead references to an asset this circuit's
   cook chose not to bake". **The mechanism question is answered now, and it
   is the second reading: the original never resolves these either**
   (2026-08-26, `rpcs3-drive.py race`, two independent boots into Talon's
   Junction). `TTY.log` names its own lookup: `GetMeshIdFromName: Couldn't
   find name %x` at `0x007b3d98` (the function itself is not located - no
   Ghidra reference resolves to either debug string, the same computed/split
   addressing that has defeated string xrefs elsewhere on this binary - so
   this is the shipped binary's own black-box behaviour, not a code reading).
   Both boots print the **identical** set of 74 failing hashes, and **all 56
   of this page's documented unresolved `Mesh` nodes are in that set exactly**
   (`hd_unaddressed.rs`'s hash list against `TTY.log`'s, allowing for the
   `%x` format dropping a leading zero three of the fifty-six hashes happen to
   have - `0x09a6aab8` prints as `9a6aab8`, not a fourth mismatch). The
   original engine hits the identical wall this project's parser does: it
   looks the node up by hash in *the track's own* `.rcsmodel` and does not
   find it, and says so on the console, rather than reaching into another
   circuit's file. **`Loading track model %s` fires exactly once per boot in
   both runs, naming only `Talons_Junction\track.rcsmodel`** - no second
   circuit's model is ever opened either. Between the single load line and
   the matching lookup failures, a prop drawn because this project found its
   geometry somewhere is now a documented negative rather than a plausibility
   argument: **the shipped executable does not hold a second circuit's
   `.rcsmodel` open, and does not resolve these 56 nodes at all - it fails
   exactly where this project's reading says it should.** One thing this
   leaves unexplained rather than answers: 18 of the 74 failing hashes are
   *not* among the 56 documented `Mesh` nodes, so `GetMeshIdFromName` is
   evidently consulted for something beyond the node table `hd_unaddressed.rs`
   walks - which other class calls it is unidentified.

   **And none of the 56 is a building** (2026-08-20), which is worth stating
   because a frame comparison read them as one. Listed with their names and
   authored boxes by `crates/render/examples/hd_unaddressed.rs`, all 56 are
   sky traffic (`Skycar_*`, `tug*`, `tanker*`, `shipintersteller*`,
   `Eggship*`, `BlimpPart*`, `HyperContintentCraft*`), four camera-bot parts,
   two 5-unit `amb_glas*` and the sixteen `pCube*` at 1.1 x 0.0 x 0.7. So
   Talon's Junction's **static scenery is drawn in full**: nothing it carries
   is missing from a frame, and the circuit reports no untextured material and
   no texture-coordinate-less chunk either. Forcing `race::visibility::visible`
   to `true` and re-rendering tick 80 of an autopilot lap changes **zero
   pixels**, so the per-section PVS is not hiding anything at that viewpoint
   either. **What is left to explain a side-by-side against a real capture is
   therefore not a missing mesh - it is a covering one.** With the cameras
   matched (`--pose="-96.4,-45.6,-209.8"` puts our chase camera where the
   reference frame's is), the trackside structures a reference frame shows and
   ours does not are **behind the circuit's own `clouds` chunks**, which paint
   an opaque sheet across the foreground because their coverage is not in the
   texture this renderer takes it from. That is identified by an ID render and
   read out of the material's microcode in
   [rcsmaterial.md](rcsmaterial.md#the-cloud-plate-paints-a-solid-sheet-and-the-microcode-says-why-2026-08-20).
   The remaining differences are brightness (the authored-magnitude thread on
   [envsettings.md](envsettings.md#it-is-authored-for-a-linear-hdr-pipeline-and-this-projects-now-is-one))
   and the glass family's unread second slot
   ([rcsmaterial.md](rcsmaterial.md#the-glass-familys-second-slot-traced-not-solved-2026-08-20)).
   **Match the camera before quoting any figure from a pair of frames**: an
   earlier pass compared a mid-lap reference against one of ours on the grid
   and drew a clipped-pixel ratio from it, which measured the framing.
7. ~~**Which chunk a `Skycube` or a pad belongs to.**~~ Made for the pads on
   2026-08-19: a `Weapon Pad` or `Speedup Pad` node names its chunk at the mesh
   payload's own `+0x30`, and the chunk is world-space baked - see "The pads
   are second-pass geometry" above. The `Skycube` tie is still unmade (HD's sky
   is drawn from `sky.gtf` instead).
8. **The `.pvs` mapping**, which is what would let a renderer draw a section at
   a time rather than all 913 chunks at once.
9. **What separates transparency mode 2 from mode 1**, and what the other 15
   bits of the state word select - 13 distinct state words disc-wide (measured
   2026-08-25, over 7,241 material records; the "17 distinct combinations"
   this bullet used to say was never re-measured after the sweep it
   describes). **One of the 15 bits is now measured**: bit 7 tracks the
   texture a material samples, not its name - see
   [above](#bit-7-tracks-the-texture-not-the-material-name---confidence-84).
   **The mode 1/2 split is now measured too** (2026-08-20):
   censused over all 12 circuits in both directions - 8,052 opaque, 2,151
   mode-1 and 537 mode-2 chunks - **mode 2 is carried by exactly 6 distinct
   material names and mode 1 by 52, and no name is ever both.** The partition
   is by material, not per chunk. Mode 2's six are `nr_crowd_bustle` (96 % of
   its triangles - the grandstand crowds), `jd_alphalambert_test`,
   `fence_alpha`, `emissive_alpha_heathaze_test`, `cf_alpha4glow` and
   `uv_anim_diffuse_alpha_emissive`; mode 1's include the glass, billboard,
   glow, light-cone, tree, cloud and traffic families. The names suggest a
   cutout, and **the obvious reading of that is refuted by the picture**:
   routing mode 2 through `oag_render`'s existing alpha-test pipeline erases
   the crowd entirely, because `crowd_avatars_22x4.gtf`'s alpha runs 0..255 at
   a mean of 120 and the threshold takes most of it - so mode 2 was believed
   not to be a plain 0.5 cutout. ~~**Resolved 2026-08-31, and the belief above
   was itself the error.**~~ `Material_ApplyRenderState`
   (`0x005d8f68`, [material-state.md](../ghidra/functions/ps3-hdfury-eu/material-state.md))
   reads state bit 1 straight into `NV4097_SET_ALPHA_TEST_ENABLE` and the two
   new fields at `+0x18`/`+0x1c` into `SET_ALPHA_FUNC`/`SET_ALPHA_REF` - mode
   2 **is** a plain alpha test, `GL_GREATER`/`0.5` disc-wide. The crowd
   texture's alpha is a clean bimodal `0`/`255` split, not the roughly-uniform
   spread "a mean of 120 discards most of it" assumed; the threshold keeps
   exactly the 47.1% that is the crowd figures. See
   [above](#mode-2-is-a-plain-alpha-test-after-all-2026-08-31). **And it is
   drawn as one now** (same day): `Material::blend()` answers its own
   `Blend::AlphaTest`, the chunks go in `Model::alpha_tested_draws`, and the
   reference reaches `mesh.wgsl` as a pipeline override off the disc's own
   value rather than as a constant - see
   [above](#mode-2-is-wired-and-it-is-not-subtle-on-every-material-2026-08-31),
   where a histogram of all eight mode-2 names - eight rather than this
   bullet's six, because that sweep covered 12 circuits and this one covers
   all 16 of all three archives - shows the "subtle either way" reading
   holding for six of them and failing outright for `cf_alpha4glow`. **What is left**: bit 7's
   question is untouched by this pass.
10. **Which of a vertex's several texture coordinates a shader actually
    samples**, per material. The declaration names them - `Uv1`, `Uv2`,
    `lightmapUV`, `map1`, `map2`, `Uvset1` - and
    `VertexDecl::diffuse_texcoord` still applies a rule and says so. What has
    moved: the **lightmap is sampled and drawn on the equation the microcode
    itself states** - `prelit_scale * lightmap^prelit_power`, its alpha a
    shadow mask gating the direct sun; see
    [renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md#the-race-fog-curve-is-read-out-of-the-circuit-materials-own-microcode)
    and `oag_render`'s `mesh.wgsl` - and the lit variants read so far bind it
    through the second half of the interpolant the diffuse shares, agreeing
    with the rule. A per-material read of every variant is what would retire
    the rule entirely.
11. **37 of the 49 attribute name hashes**, and `0x1aaf7631` above all - 13,485
    uses, and measured to be neither a direction nor an opaque colour.
12. **Whether a chunk wants its `colorSet1` or the light rig.** The colour is
    read and measured and deliberately not drawn; see above.
13. **The four bytes at `+0x0a` on a stride-18 chunk that declares no lightmap.**
    Named by the declaration on most layouts and unnamed on those.

## A texture coordinate is one of two types, and the declaration says which

**Retired and replaced on 2026-08-18.** This section used to describe a content
sniff: two coordinate types, `Half` and `Unorm16`, told apart per submesh by
whether the halves decoded plausibly. It is gone, and what it was actually
separating is worth recording, because the shape of the mistake is a general
one.

The sniff was reasoning around a missing field, and the field was not missing -
[the vertex declaration](#the-chunk-declares-its-vertex-layout-at-the-word-0x58-points-at)
carries the type in a byte. The two types it names are `0x23`, a pair of halves
on 54,120 attributes, and `0x22`, a pair of `f32` on 230. **Neither is a
`Unorm16`**, and the disc has none.

**What the sniff was separating was not two types of one attribute. It was two
different attributes.** The reader took the last four bytes of every vertex as
the coordinate; on a layout whose coordinate is elsewhere those four bytes are a
`tangent` or a `colorSet1` - four normalised bytes - and reading four bytes of
colour as a pair of halves gives infinities and values up to `65504`, which is
exactly what the sniff was measuring. Reading them as a pair of `u16` over
`0..=1` gave a *smoother* result, because colour varies smoothly, and the
texel-density metric the old section cited scored that better. It was a real
measurement of a real difference, attached to the wrong cause.

**The lesson, which is the general one:** a discriminator that separates two
groups tells you the groups differ. It does not tell you what they are, and a
metric that improves under a second hypothesis is evidence for that hypothesis
only against the hypotheses you thought of. The sweep that "found no field of
the file separating the two groups" was run against labels the sniff itself
produced, so it could not have found the field even though the field is one word
away from the bytes it swept.

## Stride 22 carries two texture coordinate sets, and so do others

**Recorded on 2026-08-17 from content, and named on 2026-08-18 by the
declaration.** The four bytes at `+0x0e` of a stride-22 vertex read as two
halves that are 100 % finite and 100 % inside the unit square on Talon's
Junction's stride-22 chunks - against `+0x0a`'s 34 % non-finite, which is what a
packed tangent looks like read as halves.

The [declaration](#the-chunk-declares-its-vertex-layout-at-the-word-0x58-points-at)
says the same thing and says whose is whose. One stride-22 layout, in the file's
own words:

```text
position    3 x i16    at +0x00
normal      1 x packed at +0x06
tangent     4 x ubyte  at +0x0a
Uv1         2 x half   at +0x0e
lightmapUV  2 x half   at +0x12
```

and the widths close on every stride the disc uses, not only this one. **It is
not a stride-22 property**: the commonest stride-18 layout carries `Uv1` at
`+0x0a` and `lightmapUV` at `+0x0e`, which is exactly the pair that used to be
read as one coordinate in the wrong place.

**The lightmap still unlocks nothing today**, which is why it is recorded rather
than sampled: which of the second texture slot's four uses applies to a given
material is in the `.rcsmaterial`'s compiled shader, and nothing here reads one.
Only 85 of the circuit's 442 materials name an `lmaps/*-lmap.gtf` second texture
at all. What has changed is that the coordinate for it is now named rather than
mistaken for the diffuse one.

## A texture coordinate's orientation is per material, in the vertex microcode

**Recovered 2026-08-24, confidence 92.** Talon's Junction rendered flat black
bands where the reference capture has light grey barrier walls - the
fourth-largest surface in a race frame, 25,469 sampled pixels at mean luma
14.8/255. The cause is not in this file at all: **the material's own vertex
program flips `v`, and this renderer did not.**

`track_wall`'s lit race-pass variant, vertex block `0x6e20`:

```text
 4  ADD o[TC6].y, -v[3].yyyy, c[206].yyyy
 5  MOV o[TC6].x, v[3].xxxx
```

`v[3]` is the block's own declared `Uv1` and `c[206].y` is the 1.0 that
instruction 9 uses as the `-1` of a `t * 2 - 1` tangent unpack, so what the
fragment program samples with is `1 - Uv1.y`. The chunks store two distinct v
values and no others, **0.724 and 1.000**; flipped they become 0.276 and 0.000,
which is the pale concrete at the top of `ds_wall_cs.gtf` rather than the
near-black, alpha-0 band at its bottom.

**It is per material and not a convention.** 18 of `track_wall`'s 94 vertex
blocks flip and **none** of `track_surface`'s 64 do; across the whole circuit
exactly **one of 283 resolved variants** flips. Flipping unconditionally makes
the walls right and every surface that was already right wrong, so
`oag_rcs::rcsmaterial::vertex::Program::flips` reads it out of the resolved
block and `mesh::rcs::skin::flips` asks per slot.

What the fix moves, measured on the repro frame: pixels crushed to near-black
fall from **9.5 % to 0.3 %** against the reference capture's 1.1 %, and mean
luminance rises from 164 to 178 against its 173. Clipping to white is unchanged
at 21 % against 15 %, which is the separate and still-open exposure question.

Six other explanations were measured and died first, and they are worth keeping
because each is a check someone will otherwise repeat:

| Explanation | How it died |
| --- | --- |
| The atlas is decoded upside down | `leveltext_atoc.gtf` reads "WELCOME TO TALON'S JUNCTION" upright |
| The sampler should mirror rather than repeat | `AddressMode::MirrorRepeat` changes nothing: -0.276 and 0.724 reach the same texels |
| A per-chunk texture transform | The chunk header carries a position bias at `+0x30` and scale at `+0x40` and nothing else |
| A per-submesh texture transform | The `0x80`-byte descriptor is zero past `+0x3e` on every wall chunk |
| The decode truncates and zero-fills the tail | Only the final row of the 1024 is all-zero |
| The mip chain darkens it | A sampler clamped to LOD 0 moves those pixels by at most one level |

Note that the first of those is *true* and still not the answer: the texture is
stored the right way up, and the flip is in the shader. Checking the texture
was what made the shader the only place left to look.

## See also

- [hd-status](hd-status.md) - everything else HD's assets do, and the `.vex`
  layer this sits on
- [vex](vex.md) - the scene format, and what a `Mesh` payload holds on the PSP
- [psarc](psarc.md) - the container both files come out of
- [ps3-disc](ps3-disc.md) - getting at the bytes in the first place
