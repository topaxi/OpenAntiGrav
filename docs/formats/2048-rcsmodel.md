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

## Section B carries a vertex declaration, structurally identical to HD's

**2026-08-27.** Section B - the object graph this reading otherwise never
walks - carries a per-chunk vertex declaration in the same shape
[rcsmodel.md](rcsmodel.md#the-chunk-declares-its-vertex-layout-at-the-word-0x58-points-at)
documents for Wipeout HD: a 4-byte
header (`count`, `stride`, two reserved bytes) then `count` 8-byte attribute
records (`name_hash`, a repeated `stride`, a type byte, an offset byte). Found
by scanning section B for HD's own `~crc32` attribute-name hashes
(`position` `0xb9d31b0a`, `normal` `0xde7a971b`, `tangent` `0xdbe5f417`,
`lightmapUV` `0x26a7b665`) and decoding outward from each hit - the reproducer
is `crates/game/examples/vita_rcsmodel_rosetta.rs`.

**The container mixes byte order within one 8-byte record.** `name_hash` is
little-endian, matching the rest of this format - but the record's own
repeated `stride` field only cross-checks against the header's stride read
**big-endian**; read little-endian it disagrees on every record. Consistent
with a serializer that writes scalar fields through explicit byte shifts
(host-endian-independent) while `name_hash` is a straight native-endian word
copy that the PS3-to-Vita port never adjusted.

**Confidence 85 on the structure and the offsets below**, from the same kind
of closure this whole format's reading rests on: every decoded declaration's
attribute records cross-check their restated stride against the header's, and
the last attribute's offset plus its byte span reaches the header's own
stride exactly. Measured on the twelve HD-ported circuits' declarations (7 to
30 per environment). The common layout, stride 28:

| Attribute | Offset | Type nibble (components, code) |
| --- | --- | --- |
| `position` | `0x00` | `c3 t9` |
| `normal` | `0x0c` | `c3 t5` |
| `tangent` | `0x10` | `c4 t5` |
| `Uv1` | `0x14` | `c2 t8` |
| `lightmapUV` | `0x18` | `c2 t8` |

Smaller strides drop attributes from the tail (stride 20 keeps
`position`/`normal`/`VertexColour1`; stride 16 keeps only
`position`/`normal`) rather than reordering the ones they keep.
`find_declarations` anchors on the `position` hash and assumes it is always
record 0 (`header_at = position_at - 4`) - true on every declaration checked
by hand, not independently verified against every chunk in the corpus, so a
declaration where `position` is not first would be silently missed rather
than misdecoded.

**What the offsets do not give: the type nibble's meaning.** 2048 runs on the
Vita's SceGxm, not the PS3's RSX, so the low nibble is a different hardware
enum and HD's `RSX_*` codes are not a safe translation - confirmed rather than
assumed: `normal`'s declared type (`t5`) tested as HD's packed 11:11:10 word
at this now-*correct* offset still scores at chance (1.1% within 18°,
mean|dot| 0.311 against a 0.5 random baseline, n=532,140 - the same
position-matched HD-pairing oracle `rcsmodel.md`'s own normal recovery used).
The byte budget is a second, independent constraint: `normal@0x0c` to
`tangent@0x10` is exactly 4 bytes for 3 declared components, which alone rules
out at least one otherwise-plausible per-component width. `position`'s type
(`t9`) is the one nibble this reading can name with confidence, because
`position` is independently confirmed as 3 little-endian `f32` (measured, not
recalled) - so `t9` is *some* 4-byte-per-component float code, whatever SceGxm
calls it.

`lightmapUV`'s plausibility (decodes to a finite `f16` pair within a
texcoord-shaped range) is 80.7% at the declared offset against 76.2% for the
old last-four-bytes guess - **not meaningfully different, and not evidence
either way**: on the dominant 28-byte layout `lightmapUV`'s declared offset
(24) *is* `stride - 4`, so the two tests read the same bytes on most of the
corpus. A lightmap atlas is also baked per platform, so even a correct decode
has no reason to match HD's own coordinates numerically at the same vertex -
unlike position and normal, which are geometry and port unchanged.

## What is not decoded

Named here rather than left to be rediscovered:

- **Section B's object graph**, past the declaration above. 5,294 of
  `altima`'s GPU-pointer entries - about one in a hundred corpus-wide - are not
  half of a submesh pair, and are counted and reported rather than dropped in
  silence (`psp2::Model::unpaired_pointers`).
- **The SceGxm type codes** (`t5`, `t8`, and `t9` beyond "is a 4-byte float").
  The declaration states *where* a vertex's normal, tangent and texture
  coordinates are (see above); it does not state how the bytes there decode,
  and HD's RSX encodings are confirmed not to apply. A model still draws
  untextured and lit off computed face normals until this is read.
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
