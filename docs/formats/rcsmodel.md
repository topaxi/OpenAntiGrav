# `.rcsmodel`: Wipeout HD's render geometry

**All of it.** 643 files and 686.5 MiB on the HD/Fury disc, against 72.3 MiB of
[`.vex`](vex.md) - the single largest thing on the disc after the
[`.gtf`](hd-status.md#what-is-genuinely-new) textures. On the PSP and PS2 a
`Mesh` node's payload *is* its geometry; on the PS3 that payload is a
bounding-box pair and a 32-bit word, and the vertices are here.

Implemented in [`oag_formats::rcsmodel`](../../crates/formats/src/rcsmodel.rs),
drawn by [`oag_render::mesh::rcs`](../../crates/render/src/mesh/rcs.rs), and
checked against the disc by
[`rcsmodel_ground_truth.rs`](../../crates/formats/tests/rcsmodel_ground_truth.rs).

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
| Vertex stride | **In no field of the file.** 14, 18 or 22, recovered three ways | 88 |
| Where the stride is declared | Unknown, and **not** in the `.rcsmaterial` - see below | - |
| Vertex normal | **`+6`, packed 11:11:10 signed, big-endian** | 88 |
| The rest of a vertex | Texture coordinate located, tangent partly; not decoded | - |

## What is decoded, and what is not

**Positions, triangle indices and vertex normals.** The normals landed on
2026-08-17 and the renderer uses them; before that it lit these models off face
normals it computed itself, and it still does for any vertex the file has none
for. That fallback is reported per model rather than silent - a load report line
ending `531904 authored vertex normal(s)` is the file's own data, and one ending
`lit off face normals computed from the triangles` is a derivation, which is the
distinction [`CLAUDE.md`](../../CLAUDE.md) exists to keep.

**The texture coordinate is located and not confirmed.** The last four bytes of
every vertex read as two `f16` in a plausible range, and there is nothing to
check that against until [`.gtf`](hd-status.md#what-is-genuinely-new) is read -
so nothing consumes it, and it is not called a UV anywhere in the code. The
bytes between the normal and it are partly identified; see below.

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

**Confidence 95**, on the strongest invariant available: across `ship`,
`ship_lod1` and `talons_junction` - **1,274 of 1,274 submeshes** - the index
count is a multiple of three and every index is below its submesh's own vertex
count. Neither would survive a wrong offset or a wrong width, and the second is
checked against a number read from a different field.

## The vertex layout

```text
+0x00  i16[3]  position, through the chunk's own bias and scale
+0x06  u32     normal, packed 11:11:10 signed - see below
+0x0a  ...     a unit direction on some meshes, a constant on others: unidentified
+0x0e  ...     stride 22 only; four bytes that look like an RGBA vertex colour
last 4 bytes   two f16, in a texture-coordinate range. Located, not confirmed
```

The three widths are one layout with optional fields rather than three formats,
which is what the constant normal offset says: stride 14 is position, normal and
the last field; 18 adds the `+0x0a` field; 22 adds the colour as well.

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

### `+0x0a` is a tangent on one mesh and something else on the others

Recorded because the measurement is worth more than the guess. On Assegai's hull
the four bytes at `+0x0a` are three biased `u8` that are unit on **99.9 %** of
23,608 vertices and **perpendicular** to the normal above - median `|dot|` 0.02
on its stride-18 meshes and 0.04 on its stride-22 ones. That is a tangent.

On `ship_lod1` and on Talon's Junction the same bytes give a median `|dot|` of
**0.577**, which is `1/sqrt(3)` - exactly what a constant `(-1,-1,-1)` scores
against any axis-aligned normal, so those records hold `00 00 00` there. What
the field is in general is unrecovered, and no code reads it.

**The most promising lead on it is the descriptor byte.** A submesh's
vertex-format word is `83 XX 10 10 10 10 10 00` with `XX` running `07` to `0d`,
and that byte is already known *not* to determine the stride. Whether it
determines the **field set** - which optional attributes a vertex carries, and
therefore why `+0x0a` is a tangent on one mesh and zero on another - is a
different question and has not been tested. It would also settle the layout
table above, which today rests on three hex dumps plus the widths adding up.

## The vertex stride is not in the file

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

There is also no *link* from a submesh to a material. The 0x80-byte descriptor
has no field that partitions the 123 stride-labelled submeshes into few enough
groups to be a material index: every candidate is either the buffer offsets
(123 distinct values over 123 submeshes, so "pure" by being unique) or a field
that mixes strides. Finding the binding would be the first step of that work,
not a detail of it.

### How it is recovered instead

Three rules. The first two are validated against an oracle the format supplies;
the third validates *them*.

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

**Third, and structural rather than statistical: the file's own buffer layout.**
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
so it is `solve_stride_by_layout` first and `solve_stride_by_extent` after, and
the two are complements rather than alternatives.

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

## Wipeout HD's road is not in the `.vex`

The finding that took the longest to see, and the reason a first render of
Talon's Junction showed sky traffic hanging over an empty void.

**All 126 `Mesh` nodes of `talons_junction/track.vex` are props** - blimps,
girders, tankers, skycars, the largest of them 166 units across. There is no
road among them. The circuit itself is in the **904 of 983** chunks that no
`.vex` node references at all, and those chunks carry a **world-space** bias:
their biases span `-7009..2986` in x and `-7069..6759` in z, which is the
environment, where a referenced chunk's bias is a few units and needs its node's
transform.

So a PS3 model is read in two passes: the meshes its `.vex` places, and the
geometry nothing in the `.vex` mentions, drawn at identity. On a craft the
second pass is empty; on a circuit it is nearly everything.

**What drives it at runtime is unread.** HD moved visibility out of the `.vex`
into 28 `.pvs` files, and the obvious hypothesis is that a `.pvs` names the
chunks a section draws. Nothing here has read one.

## What is still open

Named explicitly, with what each would take.

1. **Where the vertex stride is *declared*.** Still nowhere anyone has found -
   and no longer the `.rcsmaterial`, which was read and is compiled RSX shader
   code. Three rules recover the number without it, so this is now a question
   about the format rather than a blocker. The remaining routes are
   disassembling a shader's microcode, or HD's own executable.
2. **The texture coordinate**, which is located and unconfirmed: the last four
   bytes read as two `f16` in a plausible range and there is no way to check
   that until [`.gtf`](hd-status.md#what-is-genuinely-new) is read. Nothing
   consumes it, and the code does not call it a UV.
3. **What `+0x0a` is, and whether `83 XX` names the field set.** The two are
   probably one question; see above.
4. **Why a vertex buffer sometimes ends 16, 32 or 96 bytes short** of
   `vertex_count * stride`. 8 of 46 measured pairs, always negative, always a
   multiple of 16. Harmless to the layout rule, unexplained all the same.
5. **56 of Talon's Junction's 126 `Mesh` nodes address no chunk.** All are
   scenery (`Skycar_1Shape`, `tanker1aShape`, `shipintersteller1Shape`,
   `HyperContintentCraft1Shape`). No word anywhere in their payloads is a chunk
   hash in this file, and the circuit's directory holds no second model file. A
   shared props archive elsewhere on the disc is the obvious guess and has not
   been looked for.
6. **Which chunk a `Skycube` or a pad belongs to.** Their `.vex` classes carry
   the same payload shape as a `Mesh` on the PSP, and on the PS3 the tie to a
   chunk has not been made - so a PS3 race draws neither.
7. **The `.pvs` mapping**, which is what would let a renderer draw a section at
   a time rather than all 904 chunks at once.

## See also

- [hd-status](hd-status.md) - everything else HD's assets do, and the `.vex`
  layer this sits on
- [vex](vex.md) - the scene format, and what a `Mesh` payload holds on the PSP
- [psarc](psarc.md) - the container both files come out of
- [ps3-disc](ps3-disc.md) - getting at the bytes in the first place
