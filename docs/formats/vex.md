# `.vex` scene format

**Status: partial.** The node-tree structure, class-ID table and mesh batch
layout are read from the loader. Nothing is parsed in code yet, and the
track-specific node payloads are undecoded.

`.vex` is **the** 3D format in Pulse. Ships, tracks, weapons, the skycube and
front-end props are all `.vex`. It is a **Maya scene export**: the class-ID
table carries around 600 Maya node-type names alongside the game's own.

The `06 00 00 00` magic seen in the blob census is a **version word**, not a
tag. `Vex_RelocateNodeTree` at `0x08911670` runs a pointer-fixup pass when the
version is below 6.

## Structure

```text
+0x00  16-byte file header, first word = version
+0x10  node tree, depth-first pre-order, each node 16-byte aligned
       embedded textures, appended after the tree
```

Node chunk header:

```text
+0x00  u32  class_id      patched by Vex_ResolveClassId (0x089372b0)
+0x04  u16  header_size
+0x08  u32  data_size
+0x0c  u16  child_count
```

The class-ID to name table is at `0x08ab2370`, stride 12
(`{u32 id, char *name, ptr}`), with names at `0x08a84d40`.

Confidence: **85**, read from the loader, nothing parsed yet.

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

## Geometry is pre-batched GE display lists

Meshes are **not** stored as portable vertex and index buffers. They are
pre-batched PSP Graphics Engine draw calls, compiled once at load into five
cached display lists by `Mesh_CompileDisplayLists` (`0x0890fad8`) and invoked
with `sceGuCallList`.

Batch record:

```text
+0x00  u16  pass_mask
+0x02  u8   material_index
+0x03  u8   flags            bit 6 selects an 0x80 header instead of 0x40
+0x04  u16  vertex_count
+0x08  u8   primitive_type
+0x0a  u16  GU vertex type
+0x0c  u16  payload_size
+0x0e  u16  alternate vertex offset
+0x2c  ptr  chunk whose +0x10 is a pre-compiled GE display list
+0x40       vertices
```

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

Textures are appended after the node tree rather than living in separate WAD
entries, and are bound by `Texture_BindEmbeddedData` (`0x08927f28`):

```text
+0x00  u16  width
+0x02  u16  height
+0x04  u8   bits_per_pixel
+0x05  u8   mip_count
+0x06  u8   flags
+0x08  u32  clut_size
+0x0c  u32  texel_size
+0x10  ptr  texels
+0x14  ptr  clut
```

That corroborates the [standalone `.mip` layout](psp-texture.md) and answers one
of its open questions: **`unk_0x06` is a mip count**, and a texture with mips
carries more pixel data than `w * h * bpp / 8`. The `.mip` size arithmetic
therefore holds only for `mip_count == 1`, which is every texture examined.

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
- **Collision mesh representation** for the `Floor`/`Wall`/`Mag Floor`/`Cage`
  collision nodes.
- The GU vertex-type values actually used, needed to decode geometry.
- Exact `pass_mask` bit meanings. Partial: `0x800` means the batch has its own
  display list, `0xc0` relates to alpha, `0x2000` to an extra pass.
- Whether the 16-byte file header carries anything beyond the version.
- The `.dat` format paired with ships.

## Other extensions found

`.dat`, `.svml` (a markup format under `Data\SVML\`), `.tga` (under
`Data\Tex\caustics\`), and a `.COLLISIONS` token at `0x08a886e0` whose purpose
is unknown.
