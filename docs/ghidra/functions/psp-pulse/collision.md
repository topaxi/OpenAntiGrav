# Collision

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`. **Names are proposals, not applied.**

## All five collision node types are one class

`Collision_RegisterNodeClasses` (`0x08934d44`) registers all five with the same
vtable (`0x08ad2aac`). The class ID only selects a surface-type enum and a
restitution constant.

| Node type | Class ID | Surface type |
| --- | --- | ---: |
| Wall Collision | `0x3ba` | 0 |
| Floor Collision | `0x3b9` | 1 |
| Reset Collision | `0x3cd` | 2 |
| Mag Floor Collision | `0x3e6` | 3 |
| Cage Collision | `0x3e7` | - |

So **magstrips are not special geometry**. A magstrip is ordinary floor with
surface type 3. Confidence **92**.

**Cage Collision is parsed but never instantiated**: the loader compares against
`0x3e7` and branches to the loop tail. Dead content, or another SKU. Confidence
**87**.

## An indexed triangle soup

No BSP, no quadtree, no heightfield, and **not the render mesh**. Collision
geometry is separate, which matters: reusing the visual mesh would be wrong.

`CollisionNode_ParseChunks` (`0x08934a48`) reads
`u32 version, u32 objectCount`, then per object a chunk count and chunks of
`{u32 type, u16 ?, u16 count, payload[count * stride]}`:

| Chunk | Stride | Contents |
| ---: | ---: | --- |
| 1 | 0x0c | Vertex positions, 3 × f32 |
| 2 | 0x06 | Triangle indices, 3 × u16 |
| 3 | 0x04 | **Per-vertex** f32 scalar |

Chunk 3 is per-vertex rather than per-triangle, proven by `0x0881835c` indexing
it by a triangle's three vertex indices and averaging, returning 1.0 when
absent. Confidence **88**.

The collider object is 0xa0 bytes: bounding box at `+0x00`/`+0x10`, owner id
`+0x60`, restitution `+0x64`, surface type `+0x6c`, broadphase `+0x80`,
vertices `+0x84`, per-vertex scalars `+0x88`, indices `+0x8c`, counts `+0x90`
and `+0x94`.

Restitution is **-1.0** for Floor, Mag Floor and Reset, and **0.05** for Wall.
Contact combination averages the two but forces zero if either is negative, so
the -1.0 is a **sentinel meaning "never bounce"** rather than a value.
Confidence **88**. Implementing it as a literal -1.0 restitution would produce
energy-adding collisions.

## Two-level sweep and prune

Both levels use the same structure, with 22-bit packed endpoints:
`bits[0:11] = ((int)coord + 0x400) * 2` (plus one for the max endpoint), and
`bits[12:21] = objectId`.

That quantises coordinates to **1 unit over roughly ±1024**, and caps ids at
1024 per list. Worth noting as a hard limit inherited from the data.

- **World broadphase**: 1024 slots, 8 cursors per axis, returns collider indices.
- **Per-mesh**: one entry per triangle AABB, where the **bit index is the
  triangle index directly**.

`Sap_QueryAabb` (`0x088304d4`) runs a range query per axis into a bitset, ANDs
the three, and expands set bits into an index array.

## Raycasts

`Collision_RaycastWorld` (`0x08816ac0`) splits the segment into
`floor(len * 0.01) + 1` sub-segments, queries the world broadphase, skips self
and skips Reset colliders unless explicitly requested, then runs the mesh
narrowphase: AABB clip, two slab rejects, per-mesh sweep and prune, then a
segment-triangle test using a plane sign change on both endpoints plus three
edge half-space tests.

The hit result is 0x2c bytes: point, normal, collider index, the averaged
per-vertex scalar, and a hit flag. Confidence **90**.

Contact generation against the ship (`0x08815cd4`) does a per-triangle
separating-axis reject against the box axes, then tests ten box sample points
against the triangle. Contacts are 0x40 bytes with 128 slots. Confidence **85**.

## Surface types in practice

**Mag Floor** is byte-identical to Floor at load apart from the type field. Both
hover paths accept types 1 and 3 interchangeably. The only type-3-specific code
is a dedicated probe from the ship origin along `-5.0 * (shipUp - trackGravityUp)`;
a hit sets a flag that drives a 0-to-1 blend which cancels the ordinary
suspension while a magnetic hold takes over. See [physics](../../../physics/README.md).

**Reset** is excluded from ordinary raycasts, and a contact with it triggers a
respawn. Confidence **86**.

## Proposed renames

| Address | Name | Conf |
| --- | --- | ---: |
| `0x08934d44` | `Collision_RegisterNodeClasses` | 90 |
| `0x08934a48` | `CollisionNode_ParseChunks` | 88 |
| `0x08818bdc` | `Collision_SegmentTriangle` | 90 |
| `0x08816ac0` | `Collision_RaycastWorld` | 88 |
| `0x0881646c` | `Collision_RaycastMesh` | 88 |
| `0x088304d4` | `Sap_QueryAabb` | 88 |
| `0x08816864` | `Collision_AddContact` | 86 |
| `0x08818540` | `CollisionMesh_BuildSap` | 85 |
| `0x088183e8` | `CollisionMesh_Init` | 85 |
| `0x0881835c` | `CollisionMesh_AvgVertexScalar` | 84 |
| `0x0882f8f4` | `Sap_Init` | 82 |
| `0x0884ae90` | `Ship_HoverFourCorner` | 80 |
| `0x0884a658` | `Ship_HoverTwoPoint` | 80 |
| `0x0884ba0c` | `Ship_UpdateMagLock` | 78 |

## Not determined

- **What the per-vertex f32 scalar means.** It reaches the hit result and
  accumulates into ship state. "Grip, friction or roughness" is a guess at
  confidence **40**, so it is deliberately unnamed.
- The chunk-header `u16` at `+0x04` and the payload's leading `u32`, both
  discarded by the loader.
- Why Cage Collision is skipped at load.
- The magnetic-hold block's physics.
- Whether the 1024-id limit is enforced for meshes with more than 1024
  triangles; the count field is a `u16`, so the format permits more.
- What sets the collider's owner token.
