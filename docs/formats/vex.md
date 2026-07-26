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

Confidence: **95**, validated against real models.

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

The GU vertex type at batch `+0x0a` selects the layout. **Position is always
three `s16`**, because the game's own stride calculator hard-codes a `+ 6`, so
only twelve combinations are reachable:

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

## Other extensions found

`.dat`, `.svml` (a markup format under `Data\SVML\`), `.tga` (under
`Data\Tex\caustics\`), and a `.COLLISIONS` token at `0x08a886e0` whose purpose
is unknown.
