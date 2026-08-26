# Wipeout 2048's `.rcsmodel`

Wipeout HD's `.rcsmodel` is an offset-table archive read big-endian -
[rcsmodel.md](rcsmodel.md) is that format. **Wipeout 2048's is not that file.**
It shares the extension and nothing else: a linker-style image, read
little-endian, with a header section carrying relocation tables and two
back-to-back payload blocks that are rebased at load.

Read by `oag_formats::rcsmodel::psp2`, drawn by `oag_render::mesh::rcs::psp2`,
validated by `crates/formats/tests/psp2_rcsmodel_ground_truth.rs` over **all 993
files** the three EU packages ship. The loader is `RcsModel_Load`
(`0x812f15b2`), tagged `PSP2/Psp2.RcsModelLoader.cpp` - see
[track-and-collision-loaders.md](../ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md).

## The container

```c
struct Image {                 // section A, the header
    u32 magic;                 // 0xca5caded
    u32 unread;
    u32 section_count;         // 2 with geometry, 1 without
    u32 header_size;           // section A's own length
    ...
    Descriptor sections[section_count];   // from +0x20
    Relocation tables[section_count];     // in descriptor order
};                             // then section B, then section C

struct Descriptor {            // 0x20 bytes
    u32 tag;                   // unread
    u32 size;
    u32 table;                 // its relocation table, from the end of the descriptors
    u32 entries;
    u32 link_base;             // 0 on every shipped file
    ...
};

struct Relocation {            // 8 bytes
    u32 offset;                // of a pointer, within the section named below
    u32 section;               // which section that pointer lives in
};
```

**Confidence 94.** All 993 files have `A + B + C` equal to their own length, and
the header, the descriptors and both tables tile section A with 40 bytes of tail
padding on `altima`. `RcsModel_Load` walks table 0 rebasing every pointer *to* B
and table 1 rebasing every pointer *to* C.

**The link base is zero on every shipped file**, which is the fact that makes
this readable: a pointer on disc is already the offset of its target within its
section, so nothing has to simulate the rebase.

Section **B** is a serialized C++ object graph and section **C** is raw GPU
buffer data. On `altima` all 23,711 pointer locations live in B; C holds no
pointers at all.

## The geometry

```c
struct SubMesh {               // located, not walked to - see below
    u32 index_count;           // +0x00, divisible by three
    u32 vertex_count;          // +0x04
    ...
    u32 index_buffer;          // +0x10, an offset into section C
    ...
    u32 vertex_buffer;         // +0x2c
};
```

Indices are little-endian `u16` triangle lists - section C opens
`0,1,2, 2,1,3, 4,5,6, 6,5,7`, quads split into triangle pairs. A vertex begins
with its position as three little-endian `f32`.

### The record is *located* rather than walked to, deliberately

Section B's object layout is unread. Rather than guess it, this reading takes
the one thing the header states exactly - **where every pointer into the GPU
section lives** - and pairs them: a submesh holds its index-buffer pointer and,
28 bytes later, its vertex-buffer pointer.

What makes that a reading rather than a pattern match is that it is **checked**,
and the check is arithmetic that cannot come out even by accident:

| | corpus |
| --- | --- |
| files whose sections add up to the file's own length | **993 of 993** |
| submeshes whose index buffer is `count * 2` rounded up to 4 | **244,889 of 244,889** |
| …whose index count divides by three | **244,889 of 244,889** |
| …whose vertex buffer length divides by its vertex count | **244,889 of 244,889** |
| indices naming a vertex outside their own submesh | **0 of 33,335,682** |

### The vertex stride is in no field, and comes from the buffer packing

Section C is exactly the buffers, back to back in pointer order. Sorting the
distinct pointer targets and differencing them gives every buffer's length, and
a vertex buffer's length divided by its own vertex count is the stride. That is
the same oracle [rcsmodel.md](rcsmodel.md) records as one of four for Wipeout
HD; here it is the only one needed, and it is exact rather than a search.

The strides that come out are **16 to 64 in steps of four**, 24 and 20 the
commonest.

### Positions are the model's own space, and no transform is composed

A circuit's positions come out in world coordinates and a craft's about its own
origin. Measured rather than assumed: `altima`'s track model spans
x -1886..2575, y -459..839, z -1486..2435, bracketing the box its own
`track_col.col` states, while `Assegai`'s hull sits inside 2.8 x 1.7 x 7.0 units.
So both are already in the space their caller draws them in, and
`mesh::rcs::psp2::build` composes nothing.

**456 of 33,335,682 positions** across the corpus fall outside a generous world
box. That is 0.0014 %, it is recorded rather than explained, and a reading wrong
about the offset would not miss by that little.

## What is not decoded

Named here rather than left to be rediscovered:

- **Section B's object graph.** Its layout is unread; see above for what is done
  instead. 5,294 of `altima`'s GPU-pointer entries - about one in a hundred
  corpus-wide - are not half of a submesh pair, and are counted and reported
  rather than dropped in silence (`psp2::Model::unpaired_pointers`).
- **Vertex normals and tangents**, the four bytes at each vertex's `+0x0c` and
  `+0x10`. They look like packed 11:11:10 triples, which is not a measurement.
  The renderer derives face normals from the triangles instead and says so.
- **Texture coordinates.** The last four or eight bytes of a vertex are one or
  two `f16` pairs - on a track the second is a lightmap coordinate, which the
  per-circuit `lmaps/*-lmap.gxt` entries corroborate. Not placed, so a 2048
  model draws untextured.
- **The material and texture binding**, and the 64-bit hashes each submesh
  record carries beside its buffer pointers.
- **The section tags** at each descriptor's `+0x00` (`0xe35e00df` and
  `0xe9f17935` on `altima`).

A model with **no GPU section at all** is an ordinary state, not a failure: 43
of the 993 declare one section, and `RcsModel_Load` skips the whole second
allocation when its size is zero.

## What it looks like

`just play 2048 --race` draws Altima's road, barriers and scenery - 413,358
triangles over 2,800 submeshes - and the craft's hull, untextured and lit off
computed face normals. That is the same honest half-picture an HD model gives
before its material binding is read, and it is what the two undecoded vertex
fields above cost.
