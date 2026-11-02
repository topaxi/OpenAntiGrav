# `.vex` scene files

The 3D format. Ships, tracks, weapons, the sky cube and front-end props are all
`.vex`. It is a Maya scene export: a tree of typed nodes, with mesh batches,
collision, the AI spline and trigger volumes as node payloads, and (on PSP)
textures appended after the tree.

| Title / platform | Version word | Notes |
| --- | ---: | --- |
| Pulse PSP, Pulse PS2 | 6 | PSP meshes are GE batches; PS2 meshes are VIF packets; PS2 textures are separate files |
| Pure PSP | 4 or 3 | Same header and node framing; **class ids are renumbered** |
| HD/Fury (PS3) | 6 | Big-endian (`XXEV`); node tree, splines, collision and pads as below; meshes live in a separate `.rcsmodel` |
| 2048 (Vita), Omega (PS4) | 6 | Little-endian; meshes also external; collision in a separate `track_col.col` |

This page covers version 6 as shipped by Pulse. All integers are little-endian
unless stated; HD is the same layout byte-swapped. Confidence scores follow the
[rubric](../reverse-engineering/confidence-rubric.md) (0-100). Most layout
claims rest on shipped data agreeing with an exact invariant and so cap at 94; a
few (the embedded texture swizzle flag, the `LodGroup` switch) were also
measured live. A claim with no score has none on its evidence page.

## File and node framing

```text
file header, 16 bytes
  +0x00  u32   version
  +0x04  u32   tree_len: bytes of node tree
  +0x08  u32   texture_len: bytes of embedded texture block
  +0x0c  char[4] "VEXX"   ("XXEV" on a big-endian build)

16 + tree_len + texture_len == file length

node (depth-first pre-order)
  +0x00  u32   class_id
  +0x04  u16   header_size
  +0x08  u32   data_size
  +0x0c  u16   child_count     immediate children only
  +0x0e  u16   unknown         zero on almost every node
  +0x10  char[] name, NUL terminated, present when header_size >= 0x20

payload = node + header_size .. + data_size
next node = node + header_size + data_size
```

Walk the tree until `tree_len` is reached. `child_count` is 16 bits; read as 32
bits it is wrong on nodes where `+0x0e` is non-zero. The check that settles the
framing: the `child_count` values sum to exactly `node_count - 1`. Names are the
Maya node names (`world`, `ship_collision_fx`); the `world` payload is the
original scene path.

Confidence 94. Both invariants hold on 171 of 171 Pure files and on every Pulse
file tried.

**Class ids are table indices, not a stable enum.** Choose the table from the
file's version word. Pure's `Transform` is `0x6d`, `Mesh` `0x11e`, `Texture`
`0x373`. A version-4 skycube inside a Pulse archive uses version-4 ids.

## Class ids, version 6 (Pulse)

| Group | Classes |
| --- | --- |
| Scene | `World` 0xf4, `Transform` 0x6e, `Anim Transform` 0x3c0, `LodGroup` 0x2ee, `Camera` 0xf7, `gridCamera` 0x3dd |
| Geometry | `Mesh` 0x125, `Texture` 0x3c1, `MeshNode_Ghost` 0x3d4 |
| Lights | `AmbientLight` 0x12c, `DirectionalLight` 0x131, `PointLight` 0x132, `Dynamic Point Light` 0x3c2, `lensflare` 0x3de |
| Track | `WO Track` 0x3bb, `Start Position` 0x3bc, `Speedup Pad` 0x3bd, `Weapon Pad` 0x3be, `section` 0x3c9, `gate` 0x3ca |
| Collision | `Floor` 0x3b9, `Wall` 0x3ba, `Reset` 0x3cd, `Mag Floor` 0x3e6, `Cage` 0x3e7 |
| Ship | `Engine Flare` 0x3bf, `Airbrake` 0x3c5, `Ship Muzzle` 0x3e2, `Ship Collision Fx` 0x3d0, `engine_fire` 0x3e5, `exitglow` 0x3e4, `cannon_flash` 0x3eb |
| Effects, environment | `ParticleSystem` 0x3c4 (names a [`.pob`](pob.md)), `Trail` 0x3c8, `Quake` 0x3c7, `Skycube` 0x3c6, `fogCube` 0x3d3, `cloudCube` 0x3d8, `sea` 0x3d5, `weatherPos` 0x3da |
| Audio | `sound` 0x3e1, `soundcone` 0x3e9, `speaker` 0x3cc |

The class table in the Pulse executable holds 863 records. HD adds
`collision_trackwall` 0x3ed and `absorb` 0x3ee; Pulse's table does not name
them. Confidence 94 (ten ids were checked independently against decoded payloads
and the rest read from the table).

## `Transform` (0x6e) and `Anim Transform` (0x3c0)

`Transform`: a 64-byte row-major 4x4 matrix, or an empty payload meaning
identity. Row-vector convention (`v' = v * M`); translation in row 3; rows 0 to
2 are an orthonormal basis; row 1 is up and `+y` is world up. A mesh's vertices
are in the space of every transform above it (up to 25 deep on a track), so the
world matrix is the product down the tree. Confidence 92.

`Anim Transform`: the same, with keyframed translation, rotation and scale.
Payload (all pointers are byte offsets from the payload start):

```text
+0x02 u16 translation key count    +0x04 u16 rotation key count
+0x06 u16 scale key count
+0x08 u32 rotation times (u16 frames)      +0x0c u32 translation times (u16)
+0x10 f32[3] translation base              +0x20 f32[3] translation quantum
+0x1c u32 rotation values (s16 x3, /32767) +0x2c u32 translation values (s16 x3)
+0x34 u32 flags: bit 0 translation keys are f32 triples, bit 2 rotation keys are f32 quaternions
+0x38 u32 scale times (u16)  +0x3c f32 seconds per key unit (1/60)
+0x40 u32 scale values (s16 x3, /256)
```

The six arrays tile `[0x50, payload_len)` exactly on all 393 Pulse nodes (not
scored on the evidence page). Translation is `base + quantum * s16`. Key times are in
frames; hold the first and last key outside the range.

`LodGroup` (0x2ee): a 64-byte identity, position, `1.0`, a `child_count` u32 (1
or 2), a fixed 12-byte constant, and, when `child_count` is 2, a switch
distance `f32` (`child_count - 1` distances). Confidence 88. Both children are
real geometry, and the PSP selects one child per frame by view depth
(`LodGroup_SelectChild`, 85; see `docs/formats/vex.md`).

## Mesh payload (0x125)

```text
+0x00  u16     mesh flags
+0x02  u16     material_count
+0x04  u32     offset of batch list A, from the payload start
+0x08  u32     offset of batch list B (0 = no list)
+0x10  f32[3]  bounding box min (model units)
+0x20  f32[3]  bounding box max
+0x30  material[material_count], 0x14 bytes each
       then texture-transform block[material_count], 0x40 bytes each
```

Material, 0x14 bytes: `u16 flags` (render state: `0x80` accompanies additive
`_GLOW` textures, `0x2000` marks a second environment-map pass, `0x10` enables
the texture transform), `u32 texture` (index into the model's `Texture` nodes
in node order), `u32 second_texture`, and 8 bytes that are zero in every file.
Not scored on the evidence page.

### Batches

Geometry is a list of pre-batched draw calls with inline vertices, not indexed.
A batch belongs to list A while `pass_mask & 1`, to list B while `pass_mask & 2`;
a record without the list's bit ends that list (and still occupies 0x40 bytes).

```text
+0x00  u16   pass_mask
+0x02  u8    material_index
+0x03  u8    flags: 0x40 = 0x80-byte header; 0x10 = additive blend (PSP)
+0x04  u16   vertex count
+0x06  u16   alternate vertex count; non-zero = use the alternate block
+0x08  u8    primitive type (3 triangle list, 4 triangle strip)
+0x09  u8    alternate primitive type
+0x0a  u16   GE vertex type (below)
+0x0c  u16   vertex payload bytes
+0x0e  u16   alternate block offset from the end of the header
+0x10  f32   position scale
+0x18  s16[3] bounding box min (vertex space)   +0x20 s16[3] max
header_size = (flags & 0x40) ? 0x80 : 0x40
vertices    = batch + header_size (+ alternate offset if alternate is used)
next batch  = batch + header_size + payload bytes
```

Strips alternate winding per triangle. Not scored on the evidence page; every
decoded vertex falls inside its batch's own bounding box, which is the standing
check on layout and scale, and Pure's 20,832 batches use the same header. The meaning of list B and of `pass_mask` bits is render-pass
selection; the details are not specified here.

### PSP vertex format

The vertex type selects the layout. Fields are in GE order: texcoord, colour,
normal, position. Position is always three `s16`:

```text
position = s16 / 32768.0 * batch.scale
normal   = s8 / 128.0
texcoord = u8 / 128.0   (or two f32)
```

| vtype bits | Meaning |
| --- | --- |
| 0-1 | Texcoord: 0 none, 1 two `u8`, 3 two `f32` (2, `u16`, is refused) |
| 2-4 | Colour: 0 none, 4 BGR5650, 5 ABGR5551, 6 ABGR4444 (tracks), 7 ABGR8888 (ships) |
| 5-6 | Normal: 0 none, 1 three `s8` |
| 7-8 | Position: must be 2 (`s16`); 3 means PS2 |
| 9+ | Must be zero (no weights, indices or morph) |

Each component is aligned to its own size and the stride is the end of position
rounded up to the largest component alignment. The twelve reachable types are
`0x100, 0x101, 0x103, 0x11c, 0x11d, 0x11f, 0x120, 0x121, 0x123, 0x13c, 0x13d,
0x13f`. Widen 16-bit colour channels by replication, not by shifting. Not scored
on the evidence page; the scale and layout are checked by every vertex lying
inside its batch's bounding box.

### PS2 batches

When `vtype & 0x180 == 0x180` the payload is a VIF DMA packet and positions are
`f32`. Header: `u32 packet bytes, u32 vtype, u32 pass_mask, u32 0`, then a DMA
tag and a VIF command stream of chunks, each ending in `MSCNT`, each unpacking
to fixed VU1 addresses: 0 GS setup (low 15 bits are the chunk's vertex count),
4 position (`V3_32` or `V4_32`), 5 colour (`V4_8`), 6 texcoords (`V2_32`), 7
normal (`V3_32`). The batch bounding box is `f32` at `+0x20` and `+0x30`, and
the scale is 1.0. A strip split across chunks repeats two vertices per boundary
(harmless zero-area triangles). Colour bytes are 0 to 128, where 128 is full
intensity (confidence 70). Only the vtype's normal bit is trustworthy; every PS2
chunk carries colour and float texcoords. Confidence 94, from three framing
lengths that close exactly and 11.8 million vertices inside their boxes.

### Texture transform blocks

One 0x40-byte block per material, after the material array, driven only when
the material's `flags & 0x10` is set:

```text
+0x00 u16 offset-track key count    +0x02 u16 scale-track key count
+0x04 u32 offset times (u16 each)   +0x08 u32 scale times
+0x0c f32 seconds per key unit (1/60)
+0x10 u32 offset values (s16 u, s16 v; 1/256)   +0x14 u32 scale values (256 = 1.0)
+0x2c f32 loop period in seconds; bit 0 of the word is the step-vs-lerp flag
```

Offsets in `+0x04..+0x14` are relative to the **start of the block array**, not
the block holding them. The loop period is authored and is not the last key
time. Confidence 90.

## Embedded textures

`Texture` nodes (0x3c1) describe textures in the block after the tree: see
[textures](textures.md#psp-embedded-texture-inside-vex) for the layout, row
padding and swizzle. The block packs each texture's palette then texels in node
order, and the sizes sum exactly to `texture_len`. A track's `Texture` nodes have
no name; match by the asset path at payload `+0x38`. On PS2, `texture_len` is 0.

## `WO Track` (0x3bb): the racing spline

One node per track: a graph of uniform cubic B-spline paths joined by junctions.
Payload:

```text
+0x00  u32   magic 0x574f7464 (bytes "dtOW" little-endian, "WOtd" big-endian)
+0x04  u32   version: 0x105 on Pulse (0x103 Pure, 0x106 HD, 0x107 2048)
+0x08  u32   path count
+0x0c  u32   junction count
+0x10  u32   paths pointer, 0 on disc
+0x14  u32   junctions pointer, 0 on disc
+0x18  u32   bit 0: collision cage enabled (1 on Pulse, 0 on Pure)
+0x20  reserved 0x20 bytes, only when version >= 0x101
then   paths:     path count   x 0x20 bytes
then   junctions: junction count x 0x10 bytes
then   control points of each path in path order,
       0x70 bytes each (0x60 from version 0x107)
```

Path (0x20): `+0x00 u32 control-point count`, `+0x04 f32` longest gap between
consecutive points, `+0x0c u32 entry junction`, `+0x10 u32 exit junction`
(`0x7fffffff` = none). Junction (0x10): four `u32` path indices, slots 0 and 1
predecessors, 2 and 3 successors, `0x7fffffff` null. A shortcut is an alternate
successor.

Control point:

| Offset | Type | Field |
| ---: | --- | --- |
| +0x00 | f32[3] | position (on the track surface; the game lifts it by 3 units along `down`) |
| +0x10 | f32[3] | tangent |
| +0x20 | f32[3] | down (`(0,-1,0)` on level ground) |
| +0x30 | f32[3] | lateral |
| +0x44 | f32 | half width, left |
| +0x48 | f32 | half width, right |
| +0x4c, +0x50 | f32 | AI corridor bounds, left and right |
| +0x40 | f32 | normalised arc position round the circuit, 0 to 1 (lap progress); -1024 is a runtime "no sample" sentinel |
| +0x54 | f32 | racing line offset |
| +0x5c, +0x5d | u8 | AI lane band low and high end (`byte / 127.5 - 1`; `0xff, 0x00` disables); read from version 0x105, forced to `0xff, 0x00` before |
| +0x60 | u8 | section id (at +0x5c from version 0x107) |
| +0x61 | u8 | flags (at +0x5d from version 0x107) |
| +0x62..+0x65 | u8[4] | hull light scales: ambient, directional, two point-light classes; `0xff` before version 0x103 and on the 0x107 short point |
| +0x66 | u8 | AI opponent speed floor when below 35 |
| +0x68..+0x6f | i8[8] | collision cage offsets, `s * |s| * 250` with `s = byte / 127`; zeroed at load before version 0x104 |

The spline does not pass through its control points. For segment `i` and
parameter `t` use points `i-1, i, i+1, i+2` (clamped at the ends) with weights
`[(1-t)^3, 3t^3-6t^2+4, -3t^3+3t^2+3t+1, t^3] / 6`, applied to vectors and
scalars alike.

Confidence 94 for the layout (the decoded structure accounts for the payload
length to the byte on every shipped track, and the 86 paths' `+0x04` gap
matches measured spacing exactly). The reserved block is the trap: reading
paths at `+0x20` lands one field early. 92 for the frame semantics and the
lift; 75 for the lane band (70 static, 75 after a census of 40 tracks; only
`07_Track` authors one); 70 for `+0x66`. The light scale and cage bytes are not
scored on the evidence page.

### `Start Position` (0x3bc)

A 64-byte row-major matrix, one per track, a grid slot (3.3 to 20.5 units off
the centreline). Row 2 is forward, row 3 the position; the game forces row 1 to
`(0, 1, 0)` and re-orthonormalises. The other grid slots are not in the file.
Which of the two non-up rows the game preserves was not read; keeping forward is
the reader's choice. Not scored on the evidence page.

### `section` (0x3c9): authored visibility

```text
+0x00 u8 index (also the bit position)   +0x01 u8 has_bounds
+0x08 u64 pvs_mask: bit n = section n visible from here
+0x10 f32[4] bbox min, +0x20 f32[4] bbox max      (only when has_bounds)
```

At most 64 sections per track (`01_Track` has exactly 64). Ids are not dense, an
id may repeat (union the masks), and an out-of-range id means "everything
visible". A section governs its parent's whole subtree. Confidence 90 for the
mask, 85 for the box.

## Collision (0x3b9, 0x3ba, 0x3cd, 0x3e6, 0x3e7)

All five classes share one payload layout: an indexed triangle soup separate from
the render mesh. The class selects only the surface type.

```text
payload
  +0x00  u32   0xffffffff
  +0x04  u32   object count
  object: u32 chunk count (3), then chunks in the order 1, 3, 2
  chunk:  u32 type, u16 element stride, u16 element count, then count * stride bytes
```

| Chunk | Stride | Contents |
| ---: | ---: | --- |
| 1 | 0x0c | Vertex positions, 3 x f32 |
| 3 | 0x04 | One f32 per vertex (always 1.0 on the shipped data) |
| 2 | 0x06 | Triangle indices, 3 x u16 |

The payload is padded to 16 bytes. Surface types: `Wall` 0 (the springy
surface), `Floor` 1, `Reset` 2 (contact triggers a respawn; excluded from
ordinary raycasts), `Mag Floor` 3 (magstrip, otherwise identical to floor),
`Cage` (parsed, then skipped; PS2 ships six, PSP none). Floor, mag floor and
reset have friction `-1.0`, a sentinel that forces contact friction to zero; do
not treat it as a value. Confidence 94: the chunk walk reaches the padding on
319 of 319 nodes, the stride word matches on 56,235 of 56,235 chunks, and every
index names a vertex.

2048 and Omega instead ship `track_col.col`: magic `kdtr`, a k-d tree (node
stride 24 on Vita, 19 on PS4) over one triangle soup with a surface byte per
triangle; bounds are centre then half-extent. See
[`docs/formats/2048-collision.md`](../formats/2048-collision.md).

## Pads (0x3bd, 0x3be)

A pad is a `Mesh` subclass: its payload is a mesh payload and it carries its own
geometry. The trigger volume is the mesh bounding box at payload `+0x10`
(min) and `+0x20` (max), in pad-local space, expanded at bind by `min.y -= 2.0`
and `max.y += 8.0`. Placement comes from the parent `Transform` chain, so nine
pads of one track can share a byte-identical payload. Row 2 of the world matrix
is the direction a speed pad pushes. Pad boxes are 9.55 to 9.71 units wide on
all 544 speed pads. Confidence 85.

## Known unknowns

- Meaning of the node header word at `+0x0e`.
- `pass_mask` bit semantics beyond the A/B list bits; batch flag bits other
  than `0x40` and `0x10`.
- Pure's class table beyond the three ids measured; HD, 2048 and Omega mesh
  data (external `.rcsmodel`, see [`docs/formats/rcsmodel.md`](../formats/rcsmodel.md)).
- Lighting and fog node payloads, `gridCamera`, `lensflare`, audio nodes and
  `Quake`: decoded in `crates/vex` but not specified here.
- The AI-only control point bytes `+0x5a`, `+0x5b`, `+0x58`, `+0x59` on 2048.

## Evidence

- Evidence pages: [`docs/formats/vex.md`](../formats/vex.md),
  [`docs/formats/track.md`](../formats/track.md),
  [`docs/formats/collision.md`](../formats/collision.md),
  [`docs/formats/pads.md`](../formats/pads.md),
  [`docs/formats/pure-status.md`](../formats/pure-status.md).
- Readers: [`crates/vex/src/vex.rs`](../../crates/vex/src/vex.rs) and
  `vex/` submodules, [`track.rs`](../../crates/vex/src/track.rs),
  [`collision.rs`](../../crates/vex/src/collision.rs),
  [`pads.rs`](../../crates/vex/src/pads.rs),
  [`pvs.rs`](../../crates/vex/src/pvs.rs),
  [`kdcol.rs`](../../crates/vex/src/kdcol.rs),
  [`vif.rs`](../../crates/vex/src/vif.rs).
