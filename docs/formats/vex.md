# `.vex` scene format

**Status: understood.** Implemented in
[`oag-formats::vex`](../../crates/formats/src/vex.rs), with `WO Track` payloads in
[`oag-formats::track`](../../crates/formats/src/track.rs). A whole track now
assembles: `oag-view --mesh` places all 482 drawable meshes of `01_Track` from
composed `Transform` matrices, and the outline it draws matches the one the
[spline](track.md) draws independently.

`.vex` is **the** 3D format in Pulse. Ships, tracks, weapons, the skycube and
front-end props are all `.vex`. It is a **Maya scene export**: the class-ID
table carries around 600 Maya node-type names alongside the game's own.

The `06 00 00 00` magic seen in the blob census is a **version word**, not a
tag. `Vex_RelocateNodeTree` at `0x08911670` runs a pointer-fixup pass when the
version is below 6.

## Structure

```text
file header, 16 bytes:
  +0x00  u32   version, 6 in Pulse
  +0x04  u32   size of the node tree
  +0x08  u32   size of the embedded texture block
  +0x0c  char  "VEXX"

node:
  +0x00  u32   class_id
  +0x04  u16   header_size
  +0x08  u32   data_size
  +0x0c  u16   child_count      immediate children, not descendants
  +0x0e  u16   unknown          zero on all but a few dozen nodes
  +0x10  char  name, NUL-terminated, when header_size >= 0x20

next node = offset + header_size + data_size
```

Corrections to the reading taken from the loader alone, all found by running the
decoder against real files:

- **Nodes are not on a fixed 16-byte stride.** Each is followed immediately by
  the next, at `header_size + data_size`. A fixed stride walks two nodes and
  then lands inside a name string.
- **`child_count` is a `u16`**, and it counts *immediate* children. An earlier
  pass recorded it as a `u32`, which is wrong in a way that hides: the `u16` at
  `+0x0e` is zero on 2,009 of `01_Track`'s 2,071 nodes, so a 32-bit read is
  usually right and occasionally returns 1,572,865.

  The check that settles it: in a pre-order tree with immediate child counts, the
  counts sum to **one less than the node count**. As a `u16` that holds exactly on
  every file tried, ships and tracks alike; as a `u32` the sum comes out at 111
  million for 2,071 nodes. It is asserted in the ground-truth tests.
- **`+0x0e` is not decoded.** Values are small and round (24, 36, 48, 56, 68,
  368) and look like byte counts. Nothing has been traced to them.

Nodes also carry their **Maya names** in the header, which the loader ignores
but which make a dump immediately readable: `world`, `ship_collision_fx`,
`ship_engine_glow`. The `world` node's payload is the original scene path, for
example `Z:/WipeoutPSP/X2/Data/Ships/Feisar/Ship.mb`.

The `VEXX` magic is at `+0x0c`, not at the start, so a naive signature check
misses it. The header's two sizes are exact:
`16 + tree_len + texture_len == file size`.

The class-ID to name table is at `0x08ab2370`, stride 12
(`{u32 id, char *name, ptr}`), with names at `0x08a84d40`.

Confidence: **94**, validated against real models: `child_count` as a `u16`
sums to exactly one less than the node count, ships and tracks alike, which is
an exact arithmetic invariant rather than a plausible reading. Per the
[rubric](../reverse-engineering/confidence-rubric.md), data agreement caps at
94, even across many real files; the earlier 95 predated the rubric saying so.

## Node types

Game-specific types, from the class-ID table. This list is effectively a
specification of what a track contains, which makes it the most valuable single
find so far for M1 and M4.

| Group | Types |
| --- | --- |
| Scene | `World`, `Transform`, `Anim Transform`, `LodGroup`, `Camera`, `gridCamera` |
| Geometry | `Mesh`, `MeshNode_Ghost`, `NurbsSurface`, `Texture` |
| Lighting | `AmbientLight`, `DirectionalLight`, `PointLight`, `Dynamic Point Light`, `Dynamic Shadow Occluder`, `lensflare` |
| **Track** | `WO Track`, `section`, `gate`, `Start Position`, `Speedup Pad`, `Weapon Pad` |
| **Collision** | `Floor Collision`, `Wall Collision`, `Mag Floor Collision`, `Cage Collision`, `Reset Collision`, `Ship Collision Fx` |
| Ship | `Airbrake`, `Engine Flare`, `Ship Muzzle`, `engine_fire`, `exitglow`, `cannon_flash` |
| Effects | `ParticleSystem`, `Trail`, `Quake`, `blob`, `textureBlob`, `shadow` |
| Environment | `Skycube`, `fogCube`, `cloudCube`, `cloudGroup`, `sea`, `seareflect`, `seaweed`, `weatherPos` |
| Audio | `sound`, `soundcone`, `speaker` |
| Misc | `wospot`, `wopoint`, `animationTrigger` |

`section` is a **visibility partition**, not the lap structure, and the spline
lives in `WO Track`; `gate` is never registered as a runtime class at all. Those
readings are the obvious ones and they are wrong, so see
[track data](track.md). `Mag Floor Collision` is the magstrip surface that holds
ships through inversions.

## `Transform`: the scene hierarchy

Class `0x6e`, 715 of `01_Track`'s nodes, payload **64 bytes**: a row-major 4x4
matrix with the translation in row 3 and rows 0 to 2 an orthonormal basis. A
`Transform` with an **empty** payload is the identity, which is how 57 of those
715 are stored.

Row-major with row-vector multiplication (`v' = v * M`) is corroborated outside
this file: the `Start Position` bind forces **row 1** to `(0, 1, 0)` when it
re-orthonormalises a grid slot, so row 1 is the up axis and `+y` is world up.

A mesh's vertices are in the local space of the transforms above it, and on a
track that can be **25 deep**. `vex::world_transforms` composes the chain so a
mesh can be placed with one lookup. Ships have a single transform, which is why
none of this was needed to render one.

Confidence **92**: the matrices are unambiguous in the data, every basis is
orthonormal, and composing them puts 482 meshes into a recognisable track whose
outline matches the independently decoded spline. Not higher because nothing has
been run under an emulator.

## Vertex format

The GU vertex type at batch `+0x0a` selects the layout. On **PSP**, position is
always three `s16`, because the game's own stride calculator hard-codes a `+ 6`,
so only twelve combinations are reachable. On **PS2** the same word's position
field says `f32` instead and the payload is a VIF packet rather than a vertex
array; that is a separate layout and
[its own section](#ps2-the-vertex-type-still-names-the-attributes-but-the-data-is-a-vif-packet)
below.

| vtype | texcoord | colour | normal | stride | position at |
| --- | --- | --- | --- | ---: | ---: |
| `0x100` | - | - | - | 6 | 0 |
| `0x101` | u8 | - | - | 8 | 2 |
| `0x103` | f32 | - | - | 16 | 8 |
| `0x11c` | - | ABGR8888 | - | 12 | 4 |
| `0x11d` | u8 | ABGR8888 | - | 16 | 8 |
| `0x11f` | f32 | ABGR8888 | - | 20 | 12 |
| `0x120` | - | - | s8 | 10 | 4 |
| `0x121` | u8 | - | s8 | 12 | 6 |
| `0x123` | f32 | - | s8 | 20 | 12 |
| `0x13c` | - | ABGR8888 | s8 | 16 | 8 |
| `0x13d` | u8 | ABGR8888 | s8 | 20 | 12 |
| `0x13f` | f32 | ABGR8888 | s8 | 24 | 16 |

Fields are in GE order: texture, colour, normal, position.

**Tracks carry both normals and vertex colour.** An earlier note here claimed the
two were mutually exclusive, one meaning lit and the other prelit. Real track
batches use `0x139` and `0x13b`, which have both, so whatever selects lighting is
GE state we have not recovered. `oag-view` treats vertex colour as prelit and
skips its own light rig for those batches, which is the viewer's choice and not a
claim about the game.

### PS2: the vertex type still names the attributes, but the data is a VIF packet

**Status: decoded.** Implemented in
[`oag-formats::vif`](../../crates/formats/src/vif.rs) and the PS2 path of
`vex::mesh_batches`; validated by
`crates/formats/tests/vex_ps2_ground_truth.rs`.

The PS2 build declares vertex types **outside** the twelve GU combinations
above - `0x1b9` is the one that first refused to decode - and the reason is one
bit field: bits 7-8, which say how a position is stored, are `3` rather than `2`.
That is **32-bit float** positions, which the PSP builds can never use because
their stride calculator hard-codes a `+ 6` for three `s16`. So the whole
"position at, stride" table above is inapplicable, and the value of the position
bits is what selects the decoder:

```text
vertex_type & 0x0180 == 0x0180   ->  PS2 VIF packet
```

Eight types appear, and they are the same GU bit fields as the PSP's with the
position field changed:

| vtype | texcoord bits | colour bits | normal bit | batches |
| --- | --- | --- | --- | ---: |
| `0x181` | u8 | - | - | 254 |
| `0x183` | f32 | - | - | 149 |
| `0x199` | u8 | ABGR4444 | - | 34,350 |
| `0x19b` | f32 | ABGR4444 | - | 53,202 |
| `0x1a1` | u8 | - | s8 | 495 |
| `0x1a3` | f32 | - | s8 | 32 |
| `0x1b9` | u8 | ABGR4444 | s8 | 9,022 |
| `0x1bb` | f32 | ABGR4444 | s8 | 739 |

**Only the normal bit survives the port.** A normal array is present exactly when
bits 5-6 say `s8 normal`, on all 98,243 batches; but *every* PS2 chunk carries
colour and texture coordinates, including the 930 batches whose type declares no
colour at all, and the coordinates are always two floats whatever the
texture-coordinate bits say. The decoder therefore enforces the normal
correspondence and takes colour and texture coordinates as it finds them.

#### The batch payload

Where a PSP batch's payload is an interleaved vertex array, a PS2 batch's is a
**DMA packet of VIF commands** - the PS2's equivalent of the GE display lists the
PSP batches are compiled from, and just as unportable:

```text
+0x00  u32   bytes of DMA packet that follow this header
+0x04  u32   vertex type again, matching the batch header's +0x0a
+0x08  u32   pass mask again, matching +0x00
+0x0c  u32   zero
+0x10  DMA tag quadword: qwc in the low 16 bits, then two VIF command words
+0x20  VIF command stream, to the end of the payload
```

The command stream is a run of **chunks**, each ending in `MSCNT`, each holding
one `UNPACK` per attribute at a fixed VU1 address:

| VU address | Attribute | Unpack |
| ---: | --- | --- |
| 0 | One quadword of GS setup. Its low 15 bits are the chunk's vertex count. | `V4_32`, one element |
| 4 | Position, three floats | `V3_32` masked, or `V4_32` |
| 5 | Colour, four bytes | `V4_8` unsigned |
| 6 | Texture coordinates, two floats | `V2_32` masked |
| 7 | Normal, three floats | `V3_32` masked |

`STCYCL cl=4 wl=1` before the attribute unpacks is what interleaves them into one
quadword-per-attribute vertex in VU memory, which is why the four addresses are
consecutive.

Three consequences worth knowing:

- **A chunk is one draw.** A strip longer than VU1 memory allows is split across
  several, and the split **repeats two vertices**. 44,967 of the 89,302 strip
  batches are split, so their decoded vertex count exceeds the batch header's by
  two per boundary. That is not an error, and concatenating the chunks is
  correct: the repeats become zero-area triangles at the seams.

  Correct, though, for a reason worth stating rather than assuming: **every chunk
  but the last has an even vertex count**, on all 89,302 strip batches. A strip's
  winding alternates per triangle, so an even chunk length means the next chunk's
  first triangle starts at an even index and the concatenation winds the way the
  hardware would. One odd non-final chunk would invert every triangle after it -
  inside-out geometry wherever the batch is culled - so the decoder refuses that
  case instead of drawing it, and the ground-truth run over the whole disc is
  what says the case does not arise.
- **Triangle lists use `V4_32` positions**, and the fourth float is a per-vertex
  flag in a repeating `1,1,0` pattern - the ADC/kick convention for drawing a
  list through strip hardware, suppressing the first two vertices of each
  triangle. Nothing here depends on that reading: the batch header's primitive
  type already says list, and the two agree. Confidence **75**, on the pattern
  alone.
- **The batch's bounding box is elsewhere and is `f32`.** On PS2 it is at `+0x20`
  and `+0x30`, in the space the positions are already in; the PSP's `s16` box at
  `+0x18`/`+0x20` and the per-batch `scale` at `+0x10`, which is always exactly
  1.0 here, do not apply.

#### Colour is 0 to 128, not 0 to 255

The GS treats 128 as full intensity through the texture-modulate path, so the
bytes are widened by `v * 255 / 128`. Every colour byte in both discs' models is
0 to 127, so nothing saturates - and reading them as 0-255 makes every PS2 model
exactly half as bright, which looks like a lighting problem rather than a
decoding one. Confidence **70**: the 0-128 convention is the platform's, and the
data is consistent with it, but nothing in the executable has been read and
0-127 as a plain range would fit the same bytes.

#### Confidence: 94

The reading rests on invariants that cannot come out even by accident, checked
over **every `.vex` file on the PS2 disc**: 757 models, 98,243 batches, 11,767,670
vertices.

- **Three framing lengths close exactly.** The region header's declared size plus
  its own 16 bytes is the batch's payload size; the DMA tag's quadword count
  spans the packet; and the command walk lands exactly on the end. A wrong
  command length desynchronises the walk and fails one of the three.
- **Every decoded position falls inside its batch's own bounding box**, with *no
  slack at all* - 11.8 million of them. A wrong attribute address or element
  width leaves the box immediately.
- **The vertex counts reconcile**: a list unpacks exactly the declared count and
  a multiple of three; a strip unpacks two extra per split.
- **Normals are unit length** to 1.2e-7, which an accidentally-correct address
  would not produce.
- **The same model comes out the same size through both paths.** `oag-view
  --mesh` reports a radius of 831.02 for the PS2 `01_Track` against 830.92 for
  the PSP one, and 6.45 for the Feisar ship on both.

Capped at 94 by the [rubric](../reverse-engineering/confidence-rubric.md): an
exact arithmetic invariant across many real files, with nothing verified against
a runtime trace.

### 16-bit colour, which only the tracks use

Bits 2 to 4 are the GU colour format, and all four hardware formats appear or are
supported: `4` BGR5650, `5` ABGR5551, `6` ABGR4444, `7` ABGR8888. The 16-bit ones
are two bytes and two-byte aligned, so they change the stride and every offset
after them.

Ship models use only ABGR8888, so a decoder written against ships refuses a
track outright — which is exactly what happened. Widening 4-bit and 5-bit
channels must **replicate** rather than shift, or white comes out as `0xf8`.

`u16` texture coordinates fall through the game's own branch **without
advancing the offset**, which is either a pruned case or a latent bug. The
implementation refuses them rather than guessing, because a wrong offset there
shifts every following field.

### The scale, which decides whether models come out the right size

```text
position = s16 / 32768.0 * scale        scale = f32 at batch +0x10
```

The mesh's world matrix is `parent x uniform_scale(scale)`, applied with
`vmscl.q`, with the translation restored unscaled. There is no additional
offset.

Missing this is the classic failure: every model comes out a uniform wrong
size, which reads as a units problem rather than a decoding bug.

**Self-check, and it is a test rather than an anecdote now.** Each *batch*
carries its own bounding box as `s16` at `+0x18` and `+0x20`, in the same space
as its positions. Decoded vertices must fall inside it, so a wrong stride or a
wrong scale shows up immediately: with a 16-bit colour format misread, position
bytes come out of the middle of a colour.

`decoded_vertices_stay_inside_their_declared_bounds` checks **3,181 batches and
342,115 vertices** across two tracks and two ships, covering five vertex types.
It was previously a measurement someone took by hand and wrote up as though it
were a standing guarantee.

## Geometry is pre-batched GE display lists

Meshes are **not** stored as portable vertex and index buffers. They are
pre-batched PSP Graphics Engine draw calls, compiled once at load into five
cached display lists by `Mesh_CompileDisplayLists` (`0x0890fad8`) and invoked
with `sceGuCallList`.

Batch record:

```text
+0x00  u16    pass_mask
+0x02  u8     material_index
+0x03  u8     flags          bit6: 0x80 header instead of 0x40; bit2: env pass
+0x04  u16    vertex_count            primary
+0x06  u16    vertex_count            alternate; 0 means absent
+0x08  u8     primitive_type          primary
+0x09  u8     primitive_type          alternate
+0x0a  u16    GU vertex type          shared by both
+0x0c  u16    payload_size            bytes of vertex data
+0x0e  u16    alternate vertex offset
+0x10  f32    position scale
+0x18  s16[3] bounding box min, in s16 vertex space
+0x20  s16[3] bounding box max, in s16 vertex space
+0x40 or +0x80  vertices, inline
```

Vertices are **inline**, never pointed to:

```text
header_size = (flags & 0x40) ? 0x80 : 0x40
vertices    = batch + header_size [+ alternate_offset]
next batch  = batch + header_size + payload_size
```

`+0x28`, `+0x29` and `+0x2c` are written at load and are **not file data**. An
earlier note describing `+0x2c` as a pre-compiled display list was describing a
runtime allocation.

Mesh header:

```text
+0x00  u16      mesh flags
+0x02  u16      material_count
+0x04  u32      offset to batch list A, relative to the mesh payload
+0x08  u32      offset to batch list B, relative to the mesh payload
+0x10  f32[3]   bounding box min, model units
+0x20  f32[3]   bounding box max, model units
+0x30  material[material_count], stride 0x14
```

A batch belongs to list A while `pass_mask & 1` is set, and to list B while
`pass_mask & 2` is set.

**Geometry is never indexed** — the index argument to `sceGuDrawArray` is always
zero. The material array is reached through `mesh+0x5c` at runtime, stride 0x14,
with a texture index at `+0x04` into the model's texture array (`model+0x1a4`,
count at `+0x1a0`). In the **file** that array sits at payload `+0x30`; `+0x5c`
is the runtime pointer to it, not a file offset. See
[materials](#materials) for the on-disc form, which is what the
decoder reads.

This matters for the renderer. We cannot replay PSP display lists on a modern
GPU, so the loader has to *decode* them into portable vertex buffers, which
means understanding the GU vertex-type encoding. That is a known, bounded piece
of work: the format is public.

Confidence: **90** for `Mesh_EmitDrawArray` (`0x0890c1e4`) emitting
`sceGuDrawArray`, confirmed by the GE command words written at `0x08810e98`
(0x12 VTYPE, 0x10 BASE, 0x01 VADDR, 0x04 PRIM).

## Embedded textures

Textures live in a block appended after the node tree, described by
`Texture` nodes (class **`0x3c1`**) whose payload is:

```text
+0x00  u16   width
+0x02  u16   height
+0x04  u8    bits_per_pixel      4 or 8
+0x05  u8    mip_count
+0x06  u8    flags
+0x07  u8    texture index
+0x08  u32   clut_size
+0x0c  u32   texel_size
+0x10  ptr   texels              zero at rest, patched at load
+0x14  ptr   clut                zero at rest, patched at load
+0x38  char  runtime asset path, e.g. Data\Ships\Feisar\Textures\texture1.tga
```

The node *name* carries the original artist path, for example
`Z:/WipeoutPSP/X2/Data/Ships/Feisar/Textures/engine_general.tga`.

**Nothing points at the pixel data.** Both pointer fields are zero in the file.
Instead each texture's palette and texels are packed back to back, in node
order, starting immediately after the tree:

```text
for each Texture node, in order:
    clut   (clut_size bytes, RGBA8888)
    texels (texel_size bytes, base level then mips)
```

The sizes add up **exactly** to the header's declared texture length, which is
what confirms the packing: for the Feisar ship, eight textures totalling 47,296
bytes against a declared 47,296.

Only the base level is needed for display; the mips follow it.

This also answers an open question from the
[standalone `.mip` layout](psp-texture.md): the byte after `bits_per_pixel` is a
**mip count**, so the `.mip` size arithmetic holds only when it is 1, which is
true of every standalone texture examined but not of these.

### PS2: the texture block is empty, because the textures are elsewhere

**Confidence: 94.** On the PS2 disc, `Data\Ships\Feisar\Ship.vex` (from
`WADS2.WAD`) declares a texture block length of **0** at header `+0x08`, and
`16 + tree_len + 0` equals the file's actual decompressed size (250,416 bytes)
exactly - the same invariant that confirms the PSP packing above, just with a
zero on the other side of it. `01_Track\track.vex` shows the same pattern.

The textures are **separate WAD entries**, and a model's set is gathered into a
nested WAD with one entry per `Texture` node, in node order. Each entry is a
Graphics Synthesizer upload packet rather than a texture file. That is a format
of its own: see [PS2 texture](ps2-texture.md), decoded and rendering.

The `Texture` nodes that remain in a PS2 `.vex` are **stale**. They still carry
PSP-shaped `clut_size`/`texel_size` fields including mip chains, and they
disagree with the texture the game actually loads - the Feisar ship's node 0
says 512x512 8bpp with 6 mips against a 256x256 single-level entry. Reading them
as if PSP's packing applied walks off the end of the file, so
`oag_formats::vex::textures` checks the header's declared length before touching
a node's fields, with a synthetic-fixture test
(`a_zero_length_texture_block_returns_none_for_every_slot`) rather than a second
real disc image, per [the legal policy](../overview/legal.md) on test fixtures.

## Materials

At mesh payload `+0x30`, stride `0x14`, count at `+0x02`:

```text
+0x00  u16  flags
+0x04  u32  texture index into the model's texture array
+0x08  u32  second texture index, for the 0x2000 extra pass
```

A batch's `material_index` selects an entry here, which then selects a texture.
The mapping is confirmed semantically as well as structurally: the mesh named
`underbrake_flashrightShape` resolves to `colours_flashing_GLOW.tga`.

## Path templates

Names are built at runtime, which is why so few resolve from binary strings. The
templates are the naming scheme:

| Address | Template |
| --- | --- |
| `0x08a7cfd4` | `%s\%strack%s.vex` |
| `0x08a7cfc0` | `%s\start_grid.vex` |
| `0x08a7cfa8` | `%s\%sstart_grid%s.vex` |
| `0x08a7ba64` | `%s\Ship.vex` |
| `0x08a7ba4c` | `%s\Zone.vex` |
| `0x08a7babc` | `%s\%swreck.vex` |
| `0x08a7c178` | `%s\%sshield.vex` |
| `0x08a84ccc` | `%s\%sboost.vex` |
| `0x08a7d008` | `%s\TrackStartup.xml` |
| `0x08a7d6a8` | `%s\Definition.xml` |
| `0x08a8104c` | `%s\handlingstats.xml` |
| `0x08a7d598` | `%s\stringtable.xml` |
| `0x08a80eb8` | `%s\screen.xml` |
| `0x08a7ff7c` | `%s\ship_eliminator.dat` |
| `0x08a884dc` | `Data\Psys\%s.POB` |
| `0x08a88e94` | `Data\Music\FEMusic\frontend%d.at3` |

The leading `%s` is a directory read from a scene-graph node field at `+0x94`,
sourced from XML rather than from the binary. Track directory names are
therefore **not** in the executable; team names are, at `0x08a78200`:
`AG_Systems`, `Assegai`, `Goteki`, `Feisar`, `Piranha`, `Qirex`, `Triakis`.

`%s\handlingstats.xml` is worth noting: ship handling is **data**, not code.
That is good news for M4.

## Loaders

| Address | Name | Conf |
| --- | --- | ---: |
| `0x08912b80` | `Vex_LoadModel` | 90 |
| `0x08911670` | `Vex_RelocateNodeTree` | 85 |
| `0x08908b68` | `Vex_FindClassDescriptor` | 82 |
| `0x089372b0` | `Vex_ResolveClassId` | 75 |
| `0x08943330` | `Resource_LoadFile` | 88 |
| `0x08927f28` | `Texture_BindEmbeddedData` | 86 |
| `0x0890fad8` | `Mesh_CompileDisplayLists` | 84 |
| `0x0890c1e4` | `Mesh_EmitDrawArray` | 90 |
| `0x08810e98` | `Gu_DrawArray` | 92 |
| `0x08810598` | `Gu_CallList` | 90 |
| `0x08883794` | `World_LoadTrack` | 85 |
| `0x08843258` | `Ship_LoadModel` | 87 |

Applied, from
[names.tsv](../ghidra/functions/psp-pulse/names.tsv). See
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md).

## Not determined

- ~~The `section` and `WO Track` node payloads~~ — recovered, see
  [track data](track.md). Note the correction: `section` is a **visibility
  partition**, and the spline lives in `WO Track`.
- **Primitive type values.** No code inspects them; the byte goes straight to
  `sceGuDrawArray`. Observed 3 (triangles) and 4 (strip) in real models. A strip
  needs degenerate triangles to join, since the GE has no primitive restart.
- Whether the alternate vertex block overlaps or follows the primary one.
- Batch `+0x14`, and the `.x`/`.z` components of the s16 bounding box.
- **Collision mesh representation**: see
  [collision](../ghidra/functions/psp-pulse/collision.md), now decoded.
- Exact `pass_mask` bit meanings. Partial: `0x800` means the batch has its own
  display list, `0xc0` relates to alpha, `0x2000` to an extra pass.
- Whether the 16-byte file header carries anything beyond the version.
- The `.dat` format paired with ships.
- ~~**PS2 mesh batches use a vertex type this decoder does not recognise.**~~ -
  decoded, see [PS2: the vertex type still names the attributes, but the data is
  a VIF packet](#ps2-the-vertex-type-still-names-the-attributes-but-the-data-is-a-vif-packet).
  The guess recorded here was that `0x1b9` was "presumably a Graphics
  Synthesizer-native encoding, not a corrupt read of the same scheme". Half
  right: the *encoding* is the same GU bit fields, and it is the **data** that is
  GS-native. What still is not determined about the PS2 packets:
  - **What the microprogram does with the setup quadword at VU address 0.** Its
    low 15 bits are the chunk's vertex count and bit 15 is always set; the other
    three words are unread.
  - **The `STMASK` values and the fill registers behind them.** Masked unpacks
    take `w` from `STROW`/`STCOL`, and no `STROW` or `STCOL` appears in a model
    packet, so those registers are set elsewhere. Nothing geometric depends on
    it: `w` is not a position component.
  - ~~**Where PS2 textures live.**~~ - found and decoded, see
    [PS2 texture](ps2-texture.md). They are standalone archive entries holding a
    GS upload packet, gathered per model into a nested WAD. What is still open
    is **which** entry belongs to which model: the set is found by name hash at
    runtime and the name is not recovered, so `oag-view` is told the entry.

## Other extensions found

`.dat`, `.svml` (a markup format under `Data\SVML\`), `.tga` (under
`Data\Tex\caustics\`), and a `.COLLISIONS` token at `0x08a886e0` whose purpose
is unknown.
