# Collision

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`. **The names here are applied**, from [names.tsv](names.tsv).

## All five collision node types are one class

`Collision_RegisterNodeClasses` (`0x08934d44`) registers all five with the same
vtable (`0x08ad2aac`). The class ID only selects a surface-type enum and a
friction constant.

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
`+0x60`, friction `+0x64`, surface type `+0x6c`, broadphase `+0x80`,
vertices `+0x84`, per-vertex scalars `+0x88`, indices `+0x8c`, counts `+0x90`
and `+0x94`.

`collider+0x64` is **-1.0** for Floor, Mag Floor and Reset, and **0.05** for
Wall. `Collision_AddContact` averages the two colliders' values but forces zero
if either is negative, so the -1.0 is a **sentinel** rather than a value.
Confidence **88**.

**This field is friction, not restitution.** It was recorded as restitution here
until `Body_ResolveContact` (`0x0884e968`) was read: the resolver takes its
restitution from `body+0x388` and uses the combined `collider+0x64` - which
becomes `contact+0x34` - only to scale the contact's *tangential* relative
velocity. So the sentinel means "frictionless", the `0.05` is what makes a wall
scrape cost a craft `3.5 %` of its speed per frame once averaged against the
ship's own `0.02`, and nothing about the combination rule changes. See
[contact-response.md](contact-response.md).

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

## Applied renames

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

## Three of these were settled by decoding the data

Implementing the format (see [collision geometry](../../../formats/collision.md))
resolved three items that used to be in the list below, and turned one claim into
a contradiction that needs resolving here.

- **The chunk-header `u16` at `+0x04` is the element stride.** It matches the
  type's stride on 56,235 of 56,235 chunks across both discs. So it is not a
  discarded field; it is a redundant one, and checking it makes a drifting walk
  fail at the next chunk instead of decoding garbage.
- **The payload's leading `u32` is `0xffff_ffff` on all 319 nodes.** That is
  indistinguishable from a pointer slot patched at load, and confidence that it
  is a version number is only **40**, so it stays unnamed.
- **Cage Collision is not dead content: it is the other SKU.** There are 6 cage
  nodes on the PS2 disc and 0 on the PSP one, which is why the PSP loader
  branches past them. That retires the "dead content, or another SKU"
  disjunction at confidence **90**.

**And what used to be a live contradiction is now a narrowed question.** The
packing documented above covers a 2,048-unit window, while collision vertices
reach 1,335.8 on PSP and 1,554.1 on PS2 in world space. That was recorded here as
"both readings cannot be right". It has since been measured exhaustively - see
[the survey of all 16 tracks](../../../formats/collision.md#the-broadphase-does-not-pack-world-space-a-survey-of-all-16-tracks),
which holds the per-track table and the evidence - and the measurement says the
*geometry* is not at fault:

- **Seven of the sixteen environments reach past ±1024**, so this was never one
  outlier file. The two numbers above are just `07_Track` (PSP) and `08_Track`
  (PS2).
- **Nothing on either disc spans more than 2,048.** The widest collidable span of
  any single file is **2026.6057** (`10_Track`, present on both discs), 98.96% of
  the packing's window. Established by walking every `.vex` entry of both
  archives by index, so it covers all 130 PSP and 189 PS2 collision nodes, not
  only the named track files.

So every shipped track fits the window, and not one fits it *centred on the world
origin*. The remaining question is where the offset comes from, and it is a
question about **this code**, not the data: re-read `Sap_Init` (`0x0882f8f4`) and
`Sap_QueryAabb` (`0x088304d4`) for a base subtracted before the `+ 0x400`, or a
coordinate that arrives already relative to one. If neither shows a base, the
packing reading above is what is wrong. The prediction that a base exists is
confidence **75** (see the survey page); the measurements behind it are direct.

Still worth knowing before anyone implements the SAP: a transcription that packs
raw world coordinates would silently drop geometry at the far end of seven of the
sixteen tracks.

## Not determined

- **What the per-vertex f32 scalar means.** It reaches the hit result and
  accumulates into ship state. "Grip, friction or roughness" is a guess at
  confidence **40**, so it is deliberately unnamed. **This is now known to be
  unanswerable from the assets**: all 602,086 scalars on both discs are exactly
  `1.0`, so the field is authored and unused, and only its consumer can say what
  it was for.
- **Where the sweep-and-prune packing's origin comes from.** Narrowed, not
  answered: the survey above rules out the geometry being at fault (every shipped
  track fits a 2,048-unit window, none fits it centred on the world origin), which
  leaves reading `Sap_Init` for the base it must subtract. See above.
- The magnetic-hold block's physics.
- Whether the 1024-id limit is enforced for meshes with more than 1024
  triangles; the count field is a `u16`, so the format permits more.
- What sets the collider's owner token.
