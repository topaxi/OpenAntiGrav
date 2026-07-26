# `.vex` scene format

**Status: understood for geometry.** Implemented in
[`oag-formats::vex`](../../crates/formats/src/vex.rs) and validated against real
ship models. Track-specific node payloads (`section`, `gate`) are still
undecoded.

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
  +0x0c  u32   child_count
  +0x10  char  name, NUL-terminated, when header_size >= 0x20

next node = offset + header_size + data_size
```

Two corrections to the reading taken from the loader alone, both found by
running the decoder against a real file:

- **`child_count` is a `u32`**, not a `u16`.
- **Nodes are not on a fixed 16-byte stride.** Each is followed immediately by
  the next, at `header_size + data_size`. A fixed stride walks two nodes and
  then lands inside a name string.

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

`section` and `gate` are almost certainly the lap/AI spline and checkpoints.
`Mag Floor Collision` is the magstrip surface that holds ships through
inversions.

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

Fields are in GE order: texture, colour, normal, position. Normals and vertex
colour are mutually exclusive in practice, since a mesh with normals is lit and
one with colour is prelit.

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

**Self-check:** the mesh header carries an `f32` bounding box in model units at
`+0x10` and `+0x20`. Decoded vertices must fall inside it. Across the nine
meshes of a real ship model the agreement is within **0.15% of extent**, which
is `s16` quantisation and nothing more.

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
zero. Materials live at `mesh+0x5c`, stride 0x14, with a texture index at `+0x04`
into the model's texture array (`model+0x1a4`, count at `+0x1a0`).

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

| Address | Proposed name | Conf |
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

Names proposed, not applied. See
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md).

## Not determined

- **The `section` and `WO Track` node payloads**, which hold the spline control
  points, lap and checkpoint ordering, and the AI racing line. This is the
  highest-value remaining item for both M1 and M5.
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
