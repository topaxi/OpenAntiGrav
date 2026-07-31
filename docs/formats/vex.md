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

**Validated against three discs**: `pulse-psp-usa`, `pulse-ps2-eu` and
`pure-psp-usa`. Pure's `Data.wad` holds 171 `.vex` files and 23,677 nodes, at
format version 4 or 3 rather than 6, and **both** header invariants hold on
171/171 of them - the tree ends exactly at the declared tree length, and the
`u16` `child_count` sums to `node_count - 1`. Pure's mesh payloads use the same
batch header, vertex-type encoding and per-batch scale, over 20,832 batches and
2.1 M vertices.

**What does not carry across is the class-ID table below.** Pure renumbers it
wholesale - `Transform` is `0x6d` there, `Mesh` is `0x11e`, `Texture` is
`0x373` - so the IDs on this page are Pulse's, and the constants in
`oag-formats::vex` are Pulse-only by construction. Full mapping, plus the
version-3 batch-header difference and Pure's pre-swizzled embedded textures, in
the [Pure probe](pure-status.md#the-class-id-space-is-renumbered).

## Node types

Game-specific types, from the class-ID table. This list is effectively a
specification of what a track contains, which makes it the most valuable single
find so far for M1 and M4.

The IDs below were read out of the table in 2026-07, and the decode is
**self-validating at confidence 95**: ten of them were already in
`crates/formats/src/vex.rs` from unrelated evidence, and every one agrees -
including the two most easily confused, `Mag Floor Collision 0x3e6` and
`Cage Collision 0x3e7`. Mirrored in that file's `CLASS_NAMES`.

| Group | Types |
| --- | --- |
| Scene | `World` `0x0f4`, `Transform` `0x06e`, `Anim Transform` `0x3c0`, `LodGroup` `0x2ee`, `Camera` `0x0f7`, `gridCamera` `0x3dd` |
| Geometry | `Mesh` `0x125`, `MeshNode_Ghost` `0x3d4`, `NurbsSurface` `0x123`, `Texture` `0x3c1` |
| Lighting | `AmbientLight` `0x12c`, `DirectionalLight` `0x131`, `PointLight` `0x132`, `Dynamic Point Light` `0x3c2`, `Dynamic Shadow Occluder` `0x3c3`, `lensflare` `0x3de` |
| **Track** | `WO Track` `0x3bb`, `section` `0x3c9`, `gate` `0x3ca`, `Start Position` `0x3bc`, `Speedup Pad` `0x3bd`, `Weapon Pad` `0x3be` |
| **Collision** | `Floor Collision` `0x3b9`, `Wall Collision` `0x3ba`, `Mag Floor Collision` `0x3e6`, `Cage Collision` `0x3e7`, `Reset Collision` `0x3cd`, `Ship Collision Fx` `0x3d0` |
| Ship | `Airbrake` `0x3c5`, `Engine Flare` `0x3bf`, `Ship Muzzle` `0x3e2`, `engine_fire` `0x3e5`, `exitglow` `0x3e4`, `cannon_flash` `0x3eb` |
| Effects | `ParticleSystem` `0x3c4`, `Trail` `0x3c8`, `Quake` `0x3c7`, `blob` `0x3e0`, `textureBlob` `0x3df`, `shadow` `0x3cb` |
| Environment | `Skycube` `0x3c6`, `fogCube` `0x3d3`, `cloudCube` `0x3d8`, `cloudGroup` `0x3d9`, `sea` `0x3d5`, `seareflect` `0x3d7`, `seaweed` `0x3d6`, `weatherPos` `0x3da` |
| Audio | `sound` `0x3e1`, `soundcone` `0x3e9`, `speaker` `0x3cc` |
| Misc | `wospot` `0x3ce`, `wopoint` `0x3cf`, `animationTrigger` `0x3dc`, `Unused 1` `0x3db` |

Three properties of the table itself, all needed to read it correctly:

- **The third field is a runtime slot, not a shared vtable.** It reads
  `0x08b62c08` in every shipped entry, which looks like one handler for
  everything. `Vex_RegisterClass` (`0x08908eb8`) writes each class's descriptor
  into it at boot, and `Vex_FindClassDescriptor` returns `&DAT_08b62c08` - a
  *fallback* - on a miss. All-identical in `.data` means nothing is registered
  yet.
- **Terminated by `id == -1`**, per both walkers. Past the game classes it
  continues into generic Maya classes with small sequential ids (`0 Invalid`,
  `1 Base`, `2 Name`, …). Read as far as `0x08ab26a0`; **the terminator was not
  reached, so the extent is not stated here.**
- **`0x3e3` has no entry**, and the ids are not strictly ordered (`0x3d0`,
  `0x3e9`, `0x3eb` all sit out of sequence), so a gap is not evidence of a missing
  class.

There are **46 registration call sites**, one per class, in alphabetical order by
class name with monotonically increasing method-table addresses - one translation
unit per class, in static-initialiser link order. Method tables are `0x88` bytes
apart, and slot `+0x24` is update, `+0x34` submit, `+0x44` draw, `+0x7c` init.
**Class dispatch is by descriptor lookup, never by immediate compare**: there is
no `li 0x3bf` anywhere in 635,898 instructions, so searching for a class ID as a
constant will not find its handler.

Neither `engine_fire` nor `exitglow` has a registration site - neither appears
between `Engine Flare` and `fogCube` where alphabetical order would put it - yet
`exitglow` is authored **13 times** on `16_Track`. So an unregistered class can
still have instances, the same way `gate` does. Details and per-file censuses in
[`exhaust.md`](../ghidra/functions/psp-pulse/exhaust.md).

`oag-view --nodes <entry>` prints every node with its class, name and payload
size, plus a per-class census; `--class 0x3bf` filters to one. That is the tool
these censuses come from, and it exists because nothing could previously print a
node the parser does not decode.

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

## `LodGroup`: authored, but never switched at runtime

Class `0x2ee`, one of the two field-having classes in the scene table alongside
`Transform` (`docs/formats/vex.md:103` lists it under Scene). 11 instances on
`16_Track` (PSP), all structurally identical.

### The payload

**Confidence 88.** Every sample decodes as: a 64-byte identity transform, a
12-byte position (x, y, z), a 4-byte constant `1.0`, a 4-byte `child_count`
(`u32`), then a 12-byte constant "magic" (`0x1f8963d8 0x14cbd676 0xf2561de7`,
byte-identical across all 11 samples), then - only when `child_count == 2` - a
4-byte switch-distance `f32`, then zero padding out to 112 bytes total (96
bytes when `child_count == 1`, no switch-distance field and no padding).
`distances_present == child_count - 1` is exact across every sample, not a
loose correlation. 10 of the 11 instances on `16_Track` have `child_count ==
2`; only one has `child_count == 1`. Each `LodGroup`'s node-tree children (the
VEX tree structure, not the payload) are, in every observed case, one or more
`Transform` + `Mesh` pairs per tier - real, distinct geometry in both tiers
when `child_count == 2`, not a placeholder second child.

Not confidence 90+ only because the switch-distance field's *unit* is
unconfirmed (world units are the natural read, but nothing ties it to camera
distance specifically) and the corpus is one track, not several.

**The same track's PS2 build carries the same shape.** 10 `LodGroup` nodes (9
with `child_count == 2`, 1 with `child_count == 1`, versus PSP's 11 and 10),
same offsets, same "magic" field byte-identical across every PS2 sample - just
a *different* platform-wide constant than PSP's
(`0x08028bb8 0x0adbd758 0x46357f86` versus PSP's, above). One constant per
platform rather than per node either way, which is what the corpus now spans:
two platforms agreeing on structure, not one.

### The runtime never reads any of it

**Confidence 88.** `docs/ghidra/functions/psp-pulse/exhaust.md` already
established the class table's structure and the trap in reading it: the
table's third field is a *runtime slot* `Vex_RegisterClass` (`0x08908eb8`)
fills at boot from one of 46 per-class static initialisers, not a shared
vtable, and dispatch is by descriptor lookup, never by an immediate compare -
so the only way to find a class's real behaviour is its registration call
site.

`LodGroup`'s is `FUN_0890c028`, which registers method table `&DAT_08ad160c`.
Three independent checks on that table all come back empty or generic:

- **`update`/`submit`/`draw`** (`+0x24`/`+0x34`/`+0x44`, the slot layout
  `exhaust.md` established) are `0x089447a4`/`0x089447f4`/`0x0894484c`. These
  are not merely "shared defaults" by inference from address range - each was
  disassembled directly. `update` is `jr ra; nop`: return, do nothing, no
  exceptions. `submit` and `draw` are each four instructions that OR one flag
  bit into the node's flags word and return; neither touches `child_count`,
  position, or the switch-distance field, nor calls anything else. There is no
  room in either for a hidden distance check.
- **`init`** (`+0x7c`) is `0x0890bc7c`, which *is* `LodGroup`-specific - but it
  turns out to be pure vtable-stamping boilerplate (stamp the method-table and
  class-name pointers, call the shared allocator) on top of `FUN_08944fc0`,
  the universal base-node constructor (32 unrelated call sites, including
  `Vex_LoadModel` itself). The one call inside it that looked like it might
  parse the payload, `FUN_08944bd4`, decompiles to pure sibling-list linkage
  (splicing the new node into its parent's child chain) - it never reads
  `child_count`, the position, or the switch-distance field either.

The generic tree walker that collects drawable nodes (`FUN_08a71364`, called
from `Vex_LoadModel` to gather every `Mesh` in the tree) recurses into **every**
child unconditionally:

```c
for (child = *(int*)(node+0x10); child != 0; child = *(int*)(child+0xc))
    FUN_08a71364(child, out, cap, count, target_class);
```

No distance check, no class-based branch, nothing keyed on `LodGroup`
specifically. Three independent angles - the method table's per-slot
disassembly, the constructor's actual field reads, and the generic tree walk -
all agree: **this retail PSP binary has no LOD-tier-selection code anywhere.**
When `child_count == 2`, both tiers are collected and drawn, always.

Not confidence 90+ because "no code branches on this" is a negative result -
thorough by three unrelated routes now, but a fourth undiscovered path (e.g.
inside `Vex_LoadModel`'s own root-node dispatch, only partially read) can't be
ruled out with the certainty a positive decode gets.

### The two tiers are genuine, well-formed LOD pairs, not mismatched content

**Confidence 90.** That the runtime never selects a tier raises an obvious
question: are the "detail levels" even real, or could this class have been
repurposed or left half-finished, with its second child being unrelated
content rather than an actual simplification? Checked directly by walking each
`child_count == 2` group's two node-tree children separately (not just the
payload's four-byte counter) and measuring real triangle count, material
count and world-space bounding box for each subtree, on all ten real instances
on `16_Track`:

| Node | Tier 0 triangles | Tier 1 triangles | Ratio | Centres match |
| --- | --- | --- | --- | --- |
| 22  | 1556 | 78   | 20x | yes (within 1.5 units) |
| 27  | 669  | 201  | 3x  | yes |
| 66  | 211  | 91   | 2x  | yes |
| 88  | 414  | 146  | 3x  | yes |
| 417 | 798  | 52   | 15x | yes |
| 431 | 1005 | 197  | 5x  | yes, bbox identical |
| 441 | 2048 | 1024 | 2x  | yes |
| 485 | 2183 | 1251 | 2x  | yes, bbox identical |
| 824 | 1369 | 389  | 4x  | yes (within 3 units) |
| 885 | 957  | 214  | 4x  | yes (within 1 unit) |

Ten for ten: tier 0 always has more triangles than tier 1 (2x to 20x), and
every pair's bounding box and centre coincide to within a few world units -
the same object in the same place, not two unrelated pieces of scenery that
happen to share a parent. This is deliberate, correctly authored LOD content.
**"Never wired up" and "not accurate" are independent questions, and this
project's evidence answers them oppositely**: the selection mechanism was cut
before shipping, but the assets it was meant to select between are real and
correct.

**What this means for a reimplementation:** the payload's `child_count`/
switch-distance fields are real authoring-time data, and the geometry on both
sides of them is real, correctly-scaled content - not a case where the
"missing feature" excuses low confidence in the assets themselves. But there
is still no *observed selection behaviour* to reproduce: the original always
renders every child. Adding distance-based tier switching would not be
recovering a mechanism, it would be *inventing* one, and it would visibly
diverge from the original (culling geometry the original always drew).
Rendering only tier 0 of a `child_count == 2` group is a legitimate,
faithful-to-measurement optimisation (it removes genuine duplicate geometry
the original also draws twice, just without picking a "better" tier by
distance), but it is a deliberate, documented performance divergence, not a
recovered feature - it should be labelled as such wherever it lands, the same
way `TRANSPARENT_BLEND` and `ALPHA_TEST_THRESHOLD` above are labelled as
invented rather than recovered.

**Implemented as `oag_render::mesh::Lod`** (`both`/`single`, `[graphics] lod`
in the settings file, defaulting to `both`): a load-time choice, not a live
switch, and deliberately not named after quality or distance - neither exists
here. `single` measured 3,643 fewer triangles on `16_Track` (exactly the sum
of every tier-1 subtree's own triangle count) and was visually indistinguishable
from `both` at normal viewing distance in every case but one, where `both`
showed a small patch of the tier-1 mesh poking through the tier-0 surface it
mostly sits inside - `single` is if anything the cleaner picture, not just the
cheaper one. A genuine distance-based switch, using the authored
switch-distance value the original itself never reads, would need `Model` to
carry per-node bounds and the render loop to re-check them against the camera
every frame - the same live mechanism the frustum-culling entry below needs,
and not yet built.

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

### List B is real, distinct geometry, not a second pass over list A

**Confidence: 90.** `oag_render::mesh` used to read list A only, on the theory
that B was a duplicate pass and reading both would draw the same surface
twice. Checked directly against `16_Track` (Talon's Junction): most meshes
that have batches in both lists really do share them (mesh `507`'s four batches
sit at offsets reachable from both `list_a_off` and `list_b_off`), but a large
number of other meshes have **zero** list-A batches and are made up entirely
of list-B ones - skipping list B does not avoid a duplicate for these, it
drops their geometry completely. A world-space reconstruction of every batch
(via `oag_formats::vex::world_transforms`, `mesh_batches` and
`transform_point`, list A and list-B-only plotted separately) shows the
list-B-only geometry sitting on the track's own route, not off to the side -
among it, a dense cluster of panels (materials resolving to a flat tint, a
black-to-cyan gradient, and a striped grate texture) is the track's reported
"glass floor". Every batch checked this way reads `Batch::is_transparent() ==
true`, `is_alpha_tested() == false`: meant to be smoothly alpha-blended, not
an alpha-tested cutout. The same list also carries unrelated things - billboard
trees and an animated glow-strip texture (`exitglow`, authored 13 times on
this track per the census above) - since it is simply "every alpha-blended
batch", not a floor-specific list.

### A batch's own attributes decide its pipeline, not which list it came from

**Confidence: 88.** The PS2 build of the same track exposed a case list
membership alone cannot explain: its tree billboards sit in **list A**, not
list B, tagged `Batch::is_alpha_tested() == true`. Routing purely by list
(A -> opaque, B -> blended, as the section above first shipped) left them in
the opaque pipeline, which hardcodes alpha to 1.0 - the billboard's fully
transparent corners rendered solid, showing as a rectangle instead of a tree.
The general rule confirmed by this: a batch's destination is decided from
`is_transparent()`/`is_alpha_tested()` on the batch itself, checked
regardless of which list (A or B) produced it. The two flags were never
observed set on the same batch across either platform's copy of this track.

A further check mattered before trusting `is_alpha_tested()` as a rule:
enumerating every alpha-tested batch on the PS2 build and decoding its
texture's real alpha channel found 303 such batches across 15 textures, but
only 209 of them (12 textures) have a texture with genuine alpha variation
(0-255 range, or a partial range like 58-78); the other 94 (`tex[0]`,
`tex[1]`, `tex[76]`, `tex[82]`, `tex[96]`, `tex[100]`, `tex[129]`) are on
textures that are fully opaque (255 everywhere) despite carrying the flag.
Routing every `is_alpha_tested()` batch into the same blended,
depth-write-off pipeline as `is_transparent()` batches (tried first, and
reverted) rendered those 94 correctly in isolation but broke mutual occlusion
between them and the rest of the opaque scene, since depth write was off for
geometry that has no actual transparency to justify it. `is_alpha_tested()`
batches get their own pipeline instead - see below.

`oag_render::mesh::Model` splits batches three ways: `draws` (opaque, neither
flag set), `alpha_tested_draws` (`is_alpha_tested()`), `transparent_draws`
(`is_transparent()`). Two more pipelines join the opaque one in
`oag_render::mesh_render::Built`, all drawn in the same pass, in that order:

- `alpha_test_pipeline` keeps depth write on and `discard`s pixels below a
  threshold in the shader (`fs_main_alpha_test` in `mesh.wgsl`) rather than
  blending - a cutout is meant to occlude and be occluded exactly like opaque
  geometry wherever its texel clears the threshold, which is what makes both
  the 94 texture-opaque batches and the 209 real-alpha-mask ones (trees among
  them) render correctly through the same pipeline.
- `blend_pipeline` is unchanged from the section above: depth write off,
  `TRANSPARENT_BLEND`, drawn last so the opaque and cutout geometry's depth is
  already resolved.

**What is not recovered:** the GE's real alpha-test reference value and
comparison function. `ALPHA_TEST_THRESHOLD` (0.5, in `mesh.wgsl`) is an
invented placeholder, the same status this file already gives the blink-light
period and `oag_render::mesh_render::TRANSPARENT_BLEND` gives its blend
state - revise the moment the real threshold is known.

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

**The two paths are not interchangeable, and only one of them is always there.**
A ship's `Texture` node uses the long header form (`header_size` 64 to 96) and
carries the `Z:/` authoring path in it. A **track's** `Texture` nodes use the
short 32-byte header with the name field zeroed, so `Node::name` is `None` for
every one of them: the runtime path at `+0x38` is the only name a track texture
has. Both forms carry `+0x38`, so it is the field to match a texture by.
Implemented as `oag_formats::vex::EmbeddedTexture::asset_path` and
`vex::texture_asset_path`; `oag-view --nodes --class 0x3c1` prints it, which is
why a track's texture list is readable at all.

Verified across all 12 PSP circuits: reading `+0x38` per node while stepping the
texture block by each node's own `clut_size + texel_size` lands **exactly** on
the file's end (`01_Track` 5,019,184 of 5,019,184; `07_Track` 4,860,288 of
4,860,288; `16_Track` 5,107,104 of 5,107,104), so the names and the packing
confirm each other. **Confidence 90.**

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

All three fields are read, as `oag_formats::vex::Material`.

### `+0x0c..0x14` is empty, and `flags` is render state

Two negative results, both from surveying every material of all 12 PSP circuits
(1,524 on `07_Track` alone). They matter because between them they rule out the
file as a source of animation data.

**The unused tail is zero everywhere.** `+0x0c..0x14` is eight bytes wide - the
right size for a `u`/`v` scroll rate pair, and the exhaust's `Trail` does carry
exactly such a pair in its own descriptor. It is `0000000000000000` on every
material of every circuit. **There is no authored per-surface UV scroll rate in
this format.** Confidence 90: an exhaustive scan of real data, which is what the
[rubric](../reverse-engineering/confidence-rubric.md) caps below 95.

**`flags` does not mark animation.** It is richly varied - 19 distinct values on
`07_Track`, from `0x0001` on plain art to `0x2001`, `0x0192` and `0x0292` - and
it does correlate with the artists' naming, but it does not separate animated
surfaces from static ones: the static `hub_banner_GLOW` and the banded
`flicker1nonalpha_GLOW` both carry `0x91`. Two bits are legible:

| Bit | Where it lands | Reading |
| --- | --- | --- |
| `0x0080` | `07_archtrim_Glow_01`, `SL_BlueStrip_GLOW`, `col_display7_GLOW`, `07_Pulse_light_BLEND_GLOW` | accompanies the `_GLOW`/additive naming convention |
| `0x2000` | **exactly** the `*_shinemap` textures - `SL_stripwindows_shinemap`, `rf_dome2_strip_shinemap`, `StripWindows_shinemap` | the extra pass, corroborating the same bit's meaning in a batch's `pass_mask` and the second texture index at `+0x08` |

The `0x2000` correlation is the useful one: a bit whose name was guessed from
`pass_mask` turns out to select precisely the textures whose *artist-given name*
says they are environment maps. **Confidence 75** - two independent namings
agreeing, with no code read behind it. `second_texture` is nonetheless 0 on
every `07_Track` material, so the pass is not exercised there and the renderer
does not implement it.

### Ship lights: one shared texture, not a mesh-naming convention

**Confidence: 85** for the texture-identity reading below, decoded from real
materials on all 8 real ships (not a name grep). **The blink *period* is a
separate, unscored, invented placeholder** - see below; folding it into the
same score as the texture-identity claim would hide that the two rest on
completely different evidence.

The first pass at this (superseded, kept here for the record of how the
reading firmed up) grepped the PSP `Data.wad` for `*flash*`/`*GLOW*` **mesh
names** and found three families - `cannon_flash*`, `underbrake_flashrightShape`,
and a bare `flasherShape`/`flashersShape*` family only present on Triakis. That
grep was the wrong unit of evidence: `cannon_flash*` and `Ship Muzzle` are a
different node class (`0x3eb`/`0x3e2`) entirely, not `Mesh` (`0x125`), and are
not part of a ship's own draw list at all. Dumping every ship's actual `Mesh`
node names and the **texture each one's material resolves to**
(`--nodes 'Data\Ships\<team>\Ship.vex' --class 0x125`, cross-checked against
`oag_formats::vex::mesh_materials`/`textures`) turns up a single, consistent
picture instead:

- Every one of the 8 playable teams carries a mesh named `glowingShape` whose
  material resolves to the exact same texture, `Data\Tex\colours_flashing_GLOW.tga`
  - not a per-ship asset, a common one shared across the roster.
- Feisar's `underbrake_flashrightShape`/`underbrake_flashleftShape` and
  Triakis's `flasherShape`/`flasher1Shape` are each ship's own *extra* copies
  of the same light (mounted somewhere the artist named for its location or
  gave a distinct label), and resolve to the identical texture. Neither ship
  is naming a different feature; both are just ships that happen to have more
  than one instance of it.
- Feisar's `self_illuminatedShape` mesh has **two materials**: one batch on
  `colours_flashing_GLOW.tga`, one on the ship's own steady-lit
  `engine_general.tga`. A mesh-name heuristic cannot get this one right in
  either direction - it is genuinely mixed at the batch level.

`oag_render::mesh::is_blink_light_texture` matches on the decoded texture name
(case-insensitively containing `flashing_glow`) rather than the mesh name, so
it judges each batch by what it actually paints. That is what makes it
generalise to all 8 ships at once - see
`crates/render/tests/blink_lights_ground_truth.rs`, a ground-truth test that
walks every team's real `Ship.vex` and fails if any one of them stops
producing a blink-tagged draw call.

### The animation is authored in the texture, on its V axis - not per-ship, not in engine code

> **Superseded in part, 2026-07-31.** The *pulse* below is measured and stands.
> The *mechanism* - the engine scrolling this texture's V coordinate - does not:
> a breakpoint on `Gu_TexOffset` through a live race found that the only
> non-zero texture offsets submitted in a frame are the exhaust ribbon's, and
> five candidate palettes were byte-static over the same window. Whatever makes
> these lights pulse, it is neither a texture-coordinate offset nor a CLUT
> scroll. An animated per-draw **colour** is the untested candidate that fits.
> See [`texture-animation.md`](../ghidra/functions/psp-pulse/texture-animation.md).
> Read the section below as the survey of the texture's contents - which is what
> it is good for - not as a recovered mechanism.


**Confidence: 85.** `colours_flashing_GLOW.tga` is 32x16. Its width (U) is the
colour-band axis already described above; its height (V) turned out to be a
*time* axis - a small filmstrip, not a spatial gradient.

The evidence, all read directly off the decoded model and texture, no
decompiled code involved:

- A light's mesh quad is authored at a **narrow, fixed V**, not spread across
  the texture. Assegai's shoulder-light pair sits at `v` `0.945-0.953`, which
  lands on row 15 of 16 - one single row, picked once at author time.
- That row's colour is one frame of a cycle. Column 18 (the saturated-red
  column, bracketed by near-black at columns 17 and 19) reads, top to bottom:
  a bright peak, a fall to a trough around row 4, and a climb back to a second
  peak at row 8 - **the same 8-value sequence twice** in the 16 rows, not 16
  independent values. Column 30 (the pale cyan/green column, at the nose
  cluster's own U) has the same shape - peak, fall, trough, climb, repeated
  twice - just a different colour. Column 27 (the grey/white column) is
  **not** a smooth peak-and-trough; it is irregular, jumping between values
  with no single hump. Three columns, three different authored curves, one
  texture.
- A live capture (120 frame-accurate screenshots from a running PPSSPP,
  breakpointed on `Game_RenderFrame`, pixels sampled at the shoulder-light
  pair - both mirrored instances tracked byte-identical) measured a real,
  smooth pulse: sharp rise, several frames held at peak, a roughly even fall
  to a trough, a roughly even climb back - repeating every 29-31 frames.
  Downsampled to 8 bins and rotated to start at its own trough, that measured
  shape (roughly `94, 130, 200, 248, 255, 253, 226, 127`) tracks column 18's
  own trough-rotated shape (`79, 120, 203, 255, 255, 248, 203, 111`) closely
  enough - same single-hump shape, same rough proportions of rise, plateau and
  fall - to call it the same curve read two different ways, not a coincidence.
  16 rows at roughly 4 frames each is ~30-32 frames, matching the measured
  period directly.

Put together: the engine scrolls this one shared texture's V coordinate
globally, on a clock around 4 frames per row. Any geometry sampling it inherits
whichever row is currently "up" at its own authored U (colour) and V (phase
within the cycle) - which is why every ship's `glowingShape` uses the exact
same texture, why the material struct carries no per-ship or per-light tag (see
above - there is nothing to tag; the animation is not a material property, it
is what the texture *is*), and why red pulses smoothly while the grey/white
band flickers irregularly: those are different authored pixel sequences in the
same asset, not different code paths. This also resolves the wide measured U
range from the previous revision of this section: it was never one batch
smeared across the palette at one instant, it was several small light elements
at different fixed `(u, v)` points, each one row deep, which only reads as a
wide range when every vertex in a batch is pooled together.

**What is not yet nailed down:** the exact scroll rate (frames-per-row is
inferred from one measured period against one column, not read out of engine
code) and the exact row-to-time correspondence at t=0 (phase). Confidence 85
covers "this is a V-axis palette-scroll animation, authored per-column in the
texture" - the precise timing constant is a narrower, separately-checkable
claim once implemented and compared against a fresh capture.

**Scroll direction: decreasing V, not increasing.** The measured red curve
above is close to symmetric (rise and fall take similar shapes), so it does
not distinguish a forward from a backward scroll. The grey/white column does:
its authored row sequence is asymmetric rather than a single smooth hump, so
playing it the wrong way round is a real, visible difference - a fast
attack/slow decay reads as the reverse, a slow build to a sudden cutoff.
`crates/render/src/mesh.wgsl` subtracts the scroll offset rather than adding
it, on direct report against the original rather than an independent
frame-by-frame capture of the white light specifically (unlike the red curve
above, which is a real measurement). **Confidence 55** for the direction
specifically - lower than the mechanism itself, and worth a proper capture of
the white light if this is revisited.

Track `Mesh` nodes carry no name at all (confirmed on `16_Track\track.vex`
with `--nodes --class 0x125`: every entry's name column is blank), and no track
references `colours_flashing_GLOW.tga` - re-verified by extracting every
`Texture` node's runtime path on all 12 circuits, not just the default one. It
stays a ships-only asset. The `07_Pulse_light_BLEND_GLOW` texture seen on
`07_Track` is a distinct asset.

### Tracks animate too, through the same mechanism and different textures

**The reasoning that first produced the paragraph above was wrong even though
its conclusion was right**, and the error is worth recording because it hid a
whole class of content. The check had been made against `Node::name`, which is
`None` for *every* track texture (see [Embedded textures](#embedded-textures)):
it could not have found a match whatever the track referenced. "No track uses
this texture" and "the parser cannot read any track texture's name" produce the
same empty result, and only the second was true of the tooling.

Reading `+0x38` instead makes track textures matchable, and several are built
the same way as the blink palette. The discriminator is geometric, measured per
draw call by `crates/render/tests/animated_uv_ground_truth.rs`:

| Texture | Circuits | Narrowest draw |
| --- | --- | ---: |
| `col_display7_GLOW.tga` | **all 12** | **0.00** of 8 rows |
| `col_display7_BLEND_GLOW.tga` | 16 | 0.19 of 8 |
| `07_Pulse_light_BLEND_GLOW.TGA` | 07 | 0.00 of 32 |
| `rf_cyclegrad_GLOW.tga` | 13 | 0.25 of 64 |
| `rf_cyclegrad2_GLOW.tga` | 13 | 1.50 of 64 |
| `rf_cyclegrad3_GLOW.tga` | 13 | 0.50 of 16 |
| `SL_BlueStrip_GLOW.tga` | 10 | 0.45 of 4 |
| `SL_Purplestrip_GLOW.tga` | 10 | 0.00 of 64 |

against the static sponsor art a naive `_GLOW`/`_ADD` rule would have swept up,
which is painted by quads spanning the **whole** texture: `hub_banner_GLOW`
65.00 of 128, `col_banners2_ADD` 31.94 of 32, `banner2` 128.00 of 128,
`tunnelanim_sb` 32.00 of 32 despite its name, `flicker1nonalpha_GLOW` 122.62 of
32, `Plasma_scroll_ADD_GLOW` 64.00 of 64. **The two groups do not overlap: every
entry is at or under 1.5 rows and every exclusion is at or above 31.9.**

### Trackside advertising is static, and that is measured on both axes

The rows above only rule out a filmstrip stepping through *rows*. One asset
looks like the horizontal case - **`FEISAR2anim.tga`**, 256x32 in 8 distinct
32x32 blocks, on sponsor art, with `anim` in the artists' own name - and its
shape cannot settle it, because `piranha_banner_ADD_GLOW.tga` is the same 256x32
with the same 8 blocks and is unambiguously a static banner.

Two measurements settle it. Every hoarding, banner and logo is painted by quads
spanning essentially its full **width**: `piranha_banner_ADD_GLOW` 254.00 of 256,
`hub_banner_GLOW` 128.00 of 128, `Moa_logo_GLOW` 128.00 of 128,
`WES_FEISAR_BANNER_A` 63.50 of 64, `Lazerfence_ADD_GLOW` 64.00 of 64,
`Plasma_scroll_ADD_GLOW` 63.75 of 64. And `FEISAR2anim.tga` is **referenced by
zero materials** on all four circuits that embed it (`03`, `04`, `13`, `14`), so
it is never drawn - the same shape as `flicker1/2nonalpha_GLOW`.

So the billboards' contents render and do not move, on either axis. This is also
why the renderer carries a scalar V rate per vertex rather than a `u`/`v` pair:
nothing on the disc is authored to scroll in U.

A narrow band is evidence for *which* textures, not a runtime rule. The engine
scrolls a shared texture globally and any geometry sampling it inherits the
animation whatever its V span - Assegai's and Piranha's own blink quads span 8.1
and 9.6 of 16 rows and animate regardless - so gating at runtime on a narrow
band would switch those two ships off.

**Confidence 65** for the track surfaces, deliberately below 70: the geometry
and the banded texture content are real measurements, but nothing had confirmed
that the original animates these particular surfaces, and the rates are reused
from the blink light rather than recovered.

**The check has since been run, and it came back negative.** A breakpoint on
`Gu_TexOffset` through a live race found the only non-zero texture offsets in the
frame are the exhaust ribbon's; every other call passes `(0, 0)`, on a circuit
carrying two of the surfaces listed above. **The original does not animate track
surfaces by moving texture coordinates**, so `[graphics] animated_textures`
defaults **off** and turning it on is a departure. The selection above is kept
because the geometry behind it is a real measurement that whatever the true
mechanism turns out to be will want. See
[`texture-animation.md`](../ghidra/functions/psp-pulse/texture-animation.md).

Two readings are ruled out rather than untested. There is **no authored scroll
rate**: material `+0x0c..0x14` is zero on every material of every circuit. And
the material `flags` word is **not** an animation switch: the static
`hub_banner_GLOW` and the banded `flicker1nonalpha_GLOW` both carry `0x91`.

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
    GS upload packet, gathered per model into a nested WAD. ~~Which entry
    belongs to which model~~ - directory position, not a name or hash: the
    entry directly before the model's own. Solved for ships and, separately,
    for each circuit's own `track.vex`; a model made of several small
    shared-atlas pieces is the part still open, see
    [PS2 texture](ps2-texture.md#how-a-model-finds-its-texture-set-directory-position-not-a-name).

## Other extensions found

`.dat`, `.svml` (a markup format under `Data\SVML\`), `.tga` (under
`Data\Tex\caustics\`), and a `.COLLISIONS` token at `0x08a886e0` whose purpose
is unknown.

## History

- **2026-07-30** - "none of their `Texture` nodes resolve to
  `colours_flashing_GLOW.tga`" was reached from a parser that read texture names
  only out of the node header, where a track's are absent. The conclusion held
  on re-checking against the runtime path at `+0x38`, but the evidence for it did
  not, and the same gap had hidden every other animated track texture. No score
  changed; the reasoning was replaced and the `+0x38` field documented as
  implemented.
- **2026-07-31** - the V-scroll *mechanism* recorded above at 85 was measured
  against the running game and contradicted: `Gu_TexOffset` is never called with
  a non-zero offset except from `Trail_DrawRibbon`. The reading was an inference
  from a real pulse plus a filmstrip-shaped texture, and both of those still
  hold; only the step from them to "the engine scrolls V" was wrong. The
  observation keeps its score, the mechanism does not, and the renderer's track
  animation now defaults off.
