# `.vex` scene format

**Status: understood.** Implemented in
[`oag-vex::vex`](../../crates/vex/src/vex.rs), with `WO Track` payloads in
[`oag-vex::track`](../../crates/vex/src/track.rs). A whole track now
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
`oag-vex::vex` are Pulse-only by construction. Full mapping, plus the
version-3 batch-header difference and Pure's pre-swizzled embedded textures, in
the [Pure probe](pure-status.md#the-class-id-space-is-renumbered).

## Node types

Game-specific types, from the class-ID table. This list is effectively a
specification of what a track contains, which makes it the most valuable single
find so far for M1 and M4.

The IDs below were read out of the table in 2026-07, and the decode is
**self-validating at confidence 95**: ten of them were already in
`crates/vex/src/vex.rs` from unrelated evidence, and every one agrees -
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
  `1 Base`, `2 Name`, …) up to `0x3b8 Last`. **The full extent is now read**
  (2026-08-26): the terminator sits at `0x08ab4be4`, one entry past `0x3b8`,
  giving **863 real records** in Pulse's own PSP table -
  `(0x08ab4be4 - 0x08ab2370) / 12`. The id column alone settles it, checked
  against dozens of already-documented names (`Mesh 0x125`, `WO Track 0x3bb`,
  `Wall Collision 0x3ba`, … through `cannon_flash 0x3eb`) with no mismatch.
  **`0x3ec` (`wingtip`), `0x3ed` (`Track Wall Collision`) and `0x3ee`
  (`absorb`) are absent from Pulse's own table** - its game-class ids run
  `0x3b9`..`0x3eb` and stop there, three short of HD's 866-record table
  (`crates/vex/src/vex/class_names.rs`), which is exactly the three ids
  above. This settles the question [hd-status.md](hd-status.md#0x3ed-is-the-barrier-along-the-road)
  left open: Pulse's PSP binary does not name `0x3ed` at all, so
  `classes::V6::CLASS_TRACK_WALL_COLLISION` is a version-6 format id carried
  over from HD, not a Pulse-attested one - consistent with Pulse authoring no
  node of that class ([`collision_ground_truth.rs`](../../crates/vex/tests/collision_ground_truth.rs)).
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
[`exhaust.md`](../ghidra/functions/psp-pulse-usa/exhaust.md).

`oag-view --nodes <entry>` prints every node with its class, name and payload
size, plus a per-class census; `--class 0x3bf` filters to one, and `--payload`
hex-dumps each shown node's bytes (`--payload-bytes 0` for all of them). That is
the tool these censuses come from, and it exists because nothing could previously
print a node the parser does not decode. The `--payload` half is what settled
`Skycube`: a size tells a locator from a parameter block, and only the bytes tell
a parameter block from a mesh.

The three **Audio** classes share one 80-byte payload and are decoded in
[track-sound-emitters.md](../ghidra/functions/psp-pulse-usa/track-sound-emitters.md):
a bank label, a cue name, a radius stored twice (as an `f32` and as a one-key
animation curve), and - on a `soundcone` - two cone angles. `speaker` `0x3cc` has
a registered class and no instance on the Pulse disc at all.

`Skycube` `0x3c6` and `fogCube` `0x3d3` are decoded in
[`skycube.md`](skycube.md). The finding that matters for reading this table: a
`Skycube` payload **is a `Mesh` payload**, so a class having its own id does not
mean it has its own layout.

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

## `LodGroup`: authored tiers, switched per frame by view depth

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

**Confidence 88.** `docs/ghidra/functions/psp-pulse-usa/exhaust.md` already
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
  class-name pointers, call the shared allocator) on top of `Node_ConstructBase`,
  the universal base-node constructor (32 unrelated call sites, including
  `Vex_LoadModel` itself). The one call inside it that looked like it might
  parse the payload, `Node_AttachChild`, decompiles to pure sibling-list linkage
  (splicing the new node into its parent's child chain) - it never reads
  `child_count`, the position, or the switch-distance field either.

The generic tree walker that collects drawable nodes (`Vex_CollectNodesByClass`, called
from `Vex_LoadModel` to gather every `Mesh` in the tree) recurses into **every**
child unconditionally:

```c
for (child = *(int*)(node+0x10); child != 0; child = *(int*)(child+0xc))
    Vex_CollectNodesByClass(child, out, cap, count, target_class);
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

**Implemented first as `oag_pulse::textures::Lod`, superseded by the per-frame
switch below** (`both`/`single`, a `[graphics] lod` settings key
defaulting to `both` until 2026-09-23; the type and the both-tiers view
are gone, and the setting is the per-title `model_detail` - see the
sections below): a load-time choice, not a live
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

### 2026-09-23: the running original does not draw tier 1 up close

**Confidence 80 that the original draws only tier 0 within the switch
distance; this contradicts the "both tiers, always" conclusion above for
what reaches the screen.** Two pixel comparisons against PPSSPP frames of
the US disc, each set against this engine under the then-current
`lod = "both"` and `lod = "single"` settings on `main` that day:

- **The player's hull** (`Ship.vex`'s own `LodGroup`: `shipShape` and its
  siblings under tier 0, `lodShape` under tier 1). The original's frame at
  the `16_Track` grid, close camera, matches `single` detail for detail -
  the `8D` decal, the panel lines, a clean rear. `both` draws the coarse
  `lodShape` over it, darkening the rear in a ring and hiding the decal. The
  pose is matched to the grid slot, not to the pixel.
- **A track `LodGroup`** on `16_Track`'s grandstand, well inside every
  authored switch distance (`118.9` to `424.0` units, from
  `crates/vex/examples/lod_group_probe.rs`). The original shows tier 0's
  mottled grass texture; `both` lays tier 1's flat panels and seams over it;
  `single` matches the original. The pose is approximate: the original frame
  is from the absorb probe's grid run, its camera a few units off ours.

Frames: `~/.cache/oag/drive/reports/lod-measure/hull-orig-both-single.png`
(original, `both`, `single` left to right) and `grass3.png` (top to bottom),
from `data/lod-measure/` and the absorb probe's PPSSPP screenshots
`UCUS98712_00001`/`00014`.

So the "no LOD-tier-selection code anywhere" reading above is wrong for
what the player sees, or wrong about which path draws: the section itself
flags `Vex_LoadModel`'s root-node dispatch as only partly read, and the
craft's draw replays a list `Mesh_DrawBatchSet` recorded at load
(`docs/ghidra/functions/psp-pulse-usa/scene-light.md`). Which of those
drops tier 1, and whether the original ever draws tier 1 beyond the
authored switch distance, is **not read**. Beyond it, tier 0 and tier 1
coincide in bounds and differ only in triangle count, so a distance switch
and "tier 0 always" look alike at that range.

### 2026-09-23, later: the switch is found - `LodGroup_SelectChild`

**Confidence 85. The original does select a tier every frame, by view
depth.** The "no LOD-tier-selection code anywhere" reading above rested on
`LodGroup`'s method table read one slot off. The table at `&DAT_08ad160c`
holds function pointers at `+0x0c`, `+0x14`, ... in 8-byte steps (the batch
set's own table, `&DAT_08ad292c`, confirms the layout: its `+0x44` is
`Mesh_DrawBatchSet`). `LodGroup`'s `+0x34` is **`0x0890be68`**, a
`LodGroup`-only function, named here `LodGroup_SelectChild`:

1. It transforms the node's authored position (its payload, reached through
   `node+0x54`) by the node's world matrix and then the view matrix
   (`DAT_08b32d00`), and takes the view-space depth `z`.
2. It scales that depth by the field of view: `d = -z * g_camera_fov_degrees
   / 65.0`.
3. It walks the payload's `child_count - 1` switch distances (the pointer at
   payload `+0x54`, relocated at load, is what the "12-byte magic" above
   is) and picks the child index equal to how many of them `d` reaches.
4. It sets bit `0x4` in the chosen child's flags word (`node+0x2c`) and
   clears it on every other child.

The geometry side agrees: `Mesh_BuildModelDrawData` (`0x08912018`) finds
every `LodGroup` (its class tag `0x08a6ba58`, registered by `FUN_0890c028`)
and compiles each child's meshes into a batch set of its own, and
`FUN_0892e7ec` keeps any mesh with a `LodGroup` ancestor out of the model's
main batch set. So each tier is drawn only through its own child's subtree,
and only the child `LodGroup_SelectChild` enabled that frame is shown.
Not 90: bit `0x4` is read as "draw this subtree" from its use here, not from
the walker that tests it.

**The distances in practice.** `16_Track`'s groups switch at `118.9` to
`424.0` units; every racing craft's own `lodGroup1` switches at **`30.0`**
(`Data\Ships\Assegai\Ship.vex` and `Qirex`, via
`crates/vex/examples/lod_group_probe.rs`). So tier 1 is in constant use:
almost every opponent more than 30 units ahead draws as its coarse
`lodShape` (315 triangles on Assegai against tier 0's 1,130), and distant
track scenery draws its coarse tier. Up close, only tier 0 shows, which is
what the two frames above measured.

**What this meant for the old `oag_pulse::textures::Lod`:** `single` was the
faithful picture up close and `both` was not, the opposite of what its doc
said. `single` became the default the same day and the `[graphics] lod`
settings key was removed; once the per-frame switch landed, the `Lod` type
and its both-tiers view were removed too - every build carries all tiers
and the table, and a fixed view calls `Model::keep_nearest`.

### 2026-09-23: implemented - the switch runs every frame

`single` was superseded the same evening by the switch itself. Every model
built from one `.vex` now keeps all of its tiers, plus an
`oag_pulse::textures::LodGroups` table: each group's payload position (`+0x40`)
carried into model space through the group node's world matrix, its
`child_count - 1` switch distances (from `+0x60`), and which group and child
every scene-tree node sits under. `LodGroups::child_at` is
`LodGroup_SelectChild`'s arithmetic and the only copy of it; each drawn
instance holds its own `LodSwitch`, so eight craft sharing one model switch
at eight distances.

Each frame the race scene switches the circuit and every craft in play,
**the player's own included** - the original's function does not know whose
hull it is on - from the unjittered view matrix and the vertical field of
view the projection is built from, in degrees (`Race::vertical_fov`, which
includes the recovered speed widen). The ghost switches at its own pose.
Anything nobody switches keeps its finest tier: the shadow casters, the
weapon models, `oag-view` (it has no race camera), a `mesh::merge`.

**What this project chose rather than measured:**

- The player's `[render_profiles.<title> (<platform>)] model_detail` multiplies every
  authored distance: `original` x1 (the measured rule, the default), `high`
  x2, `maximum` never switches. `oag-game --lod` takes the same three, for
  one run.
- The field fed to the rule is the one the picture is drawn at, so a player
  who widens `graphics.fov`, or a window narrower than the authored aspect,
  switches sooner - the rule's own `fov / 65` term extended past the PSP's
  fixed screen.
- The shadow caster keeps the finest tier rather than switching per shadow
  pass.

**Per title.** The payload layout reads the same everywhere it was checked,
and `crates/render/tests/lod_switch_ground_truth.rs` pins what it reads - the
distances and group counts below at **confidence 90** each, read off every
file through the same offsets the PSP function reads and asserted exactly,
not 95 only because the offsets themselves are the PSP's:

| Title | Craft switch distance | Circuits | Switch code |
| --- | --- | --- | --- |
| Pulse PSP | 30.0 on all eight | 55 groups on 12 circuits, 20.0 to 812.8 | read, confidence 85 |
| Pulse PS2 | 35.0, Goteki 32.05 | 40 groups on 16 circuits, 20.0 to 760.0 | not read - chosen |
| Pure | 50.0 on all six reachable | not surveyed | not read - chosen |
| HD / Fury | no craft group | 64 groups, none holds a second tier | not read - chosen |
| 2048 | no craft group | 16 groups, all childless | not read, nothing to switch |

The PS2 values sit at the same offset and are different numbers, which reads
as the port re-authoring them rather than moving the field (confidence 60 -
the alternative, that the PS2 build's own switch reads elsewhere, is
unexcluded while its code is unread). The same `65` is used for every title -
chosen, since only the PSP function was read. No group anywhere is nested
under another (confidence 90, asserted on every file above).

**HD and 2048 have nothing to switch** (confidence 90 for both counts,
`crates/render/tests/hd_lod_ground_truth.rs` for HD's). HD's 64 groups are in
six files. The four `talons_junction/start_grid*.vex` carry eight each: four
that declare two children but hold one, at the positions and distances
Pulse's `16_Track` groups use, one that declares and holds one, and three
that hold none. The two `03_track` files carry 16 each, declaring one and
holding none. Why a group that declares two holds one is unread - a coarse
tier dropped in the port is the obvious reading, at confidence 50. Since no
HD group holds a second child in its tree, every node under one is under its
finest child and the switch can hide nothing. `mesh::rcs` builds the table
regardless of `--lod`, so the proof is that census and not a picture
comparison. 2048 authors 16 groups in one file of its 1,059
(`DLC1/environments/Moa_Therma/track_reversed.vex`), all childless. HD's craft level of detail lives in a separate file beside the
hull instead (`ship_lod.vex`, see [`hd-status.md`](hd-status.md)), which
nothing here switches to yet.

**Checked on Pulse PSP (US), headless, `--no-audio`**, frames under
`~/.cache/oag/drive/reports/lod-switch/`:

- Opponents on the grid, far chase camera: slot 7 at `d = 31.4` draws its
  `lodShape`; the same tick from the close camera puts it at `d = 28.6` and
  it draws full detail (`a-snap-slot7-far-d31-over-close-d29.png`). Every
  other opponent is past 46 and coarse (`a-grid-opponents-original-over-maximum.png`).
- The player's own hull: `d = 13.35` under the far chase camera at rest,
  `10.6` under the close one, `15.8` at 147 units a second when the speed
  widen has the field at 71 degrees. The depth itself stays at 14.5 units;
  reaching 30 would take a field of about 135 degrees, a forward speed of
  1,000 units a second, so the player's hull does not switch in either chase
  view at any speed a craft reaches.
- The grid frame differs from `maximum` only inside the opponents' box
  (`562x101+162+343`): the grandstand and everything else on the circuit is
  unchanged, every circuit group being inside its distance there.
- The `16_Track` grandstand group (switch `118.9`) seen from the grid looking
  back: at `d = 139.7` it draws tier 1 (`d-g7-grandstand-d140-original-over-maximum.png`),
  at `d = 84` the frame matches `maximum` pixel for pixel (`c-g7-91u-*.png`).

No frame of the original has yet been taken of an opponent between 30 and 60
units, which is the check that would move the switch's confidence past 85.

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

### The vertex type is not misdeclared, and `u8` texcoords really are one byte each

**Confidence: 95.** Checked by
[`crates/vex/tests/vex_batch_layout_ground_truth.rs`](../../crates/vex/tests/vex_batch_layout_ground_truth.rs).

`shipboost.vex` raised the question. Its two short batches decode to a *single*
texture coordinate - all 9 (or 10) of their vertices carry `u` `0.0078`, `v`
`0.0312` - which reads exactly like the stride misread that
["Texture rows are padded to 16 bytes"](#texture-rows-are-padded-to-16-bytes)
caught one section down. The competing reading is that bytes 0-3 of a `0x13d`
vertex are two `u16` texcoords rather than a `u8` pair plus two bytes of nothing,
which would put a real `u` sweep where the table above sees a constant. Four
measurements over the whole PSP disc close it, and they are worth recording
because the same question will come up again for any batch whose UVs look degenerate.

- **The stride closes on every batch.** A batch declares its vertex count at
  `+0x04` and its vertex-data size at `+0x0c`. Taking the stride from the vertex
  type alone, `count * stride` rounded up to 16 reproduces the declared size on
  **65,279 of 65,279 batches** across 307 models, with no exceptions and at
  eleven different strides. Both candidate readings happen to give stride 20 for
  `0x13d`, so this does not by itself pick between them - but it does rule out
  "our stride is wrong" as the explanation for anything here.
- **No batch anywhere declares 16-bit texcoords.** Bits 0-1 of the vertex type
  are the GU texcoord format, and across those 65,279 batches the value is only
  ever `0` (none), `1` (`u8`) or `3` (`f32`). `2` (`GU_TEXTURE_16BIT`) does not
  occur, which is why `VertexLayout::from_vertex_type` can refuse it outright.
  The competing reading needs a format the exporter never emits.
- **The spare bytes are the exporter's fill.** In a `0x13d` vertex, bytes `+2`
  and `+3` carry the *same value* as the padding at `+11` (after the three
  normal bytes) and `+18`/`+19` (after the three position `s16`), in **744,686
  of 744,686** such vertices on the disc, and that value is only ever `0x00` or
  `0xff`. No reading of the format puts a field at `+11`. Two bytes that track
  the padding byte for byte across three quarters of a million vertices are
  padding - and a `v` coordinate that is binary on every ship hull and every
  circuit is not a texture coordinate.
- **The `u` byte is not systematically pinned.** Read the way the table above
  says, byte 0 takes all **256** values across the disc, and **25,735 of 25,900**
  `u8`-texcoord batches contain a vertex with `u` past texel 2. A decode that
  clamped `u` to nothing would show up here as a disc-wide flat distribution. It
  does not.

The `TEXCOORD_U16_GAIN` question in
[`oag-fx::exhaust`](../../crates/fx/src/exhaust.rs) does **not** transfer
to baked `.vex` data. That constant is about `Trail_DrawRibbon`, geometry the
original generates at runtime and where the game's own code writes 16-bit
texcoords for the GE to divide by 32768. Nothing on the disc is stored that way.

**A single-point UV is ordinary authoring here, not an anomaly.** 1,463 of the
25,900 `u8`-texcoord batches on the disc - 5.6% - carry exactly one distinct
texture coordinate across every vertex, at every vertex count from 3 upward.

### PS2: the vertex type still names the attributes, but the data is a VIF packet

**Status: decoded.** Implemented in
[`oag-vex::vif`](../../crates/vex/src/vif.rs) and the PS2 path of
`vex::mesh_batches`; validated by
`crates/vex/tests/vex_ps2_ground_truth.rs`.

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

#### The chain that consumes it, walked instruction by instruction

Confidence **90**, from `psp-pulse-usa/BOOT.BIN`. The scale is read **once per
mesh at load**, never per batch at draw time:

1. `Mesh_InitFromPayload` (`0x0890e998`) walks both batch lists to their end -
   the loop advancing by `payload_size + header_size`, exactly the walk above -
   keeping only a pointer to the **last** batch it saw. `lwc1 f12, 0x10(s2)` at
   `0x0890ef5c` takes that one batch's `+0x10` and stores it to the runtime
   mesh at `+0x54`.
2. The next four instructions take `1.0 / scale` (`div.s f12, f13, f12` at
   `0x0890ef70`, `f13` set to `1.0` from `lui 0x3f80`) and divide the **mesh
   header's** own `f32` bounding box through it - `+0x10`/`+0x14`/`+0x18` and
   `+0x20`/`+0x24`/`+0x28` in the mesh header, out to the runtime mesh's
   `+0x80..0x98`, with the box centre at `+0xa0..0xa8` and `1.0` at `+0xac`.
   That is the same normalisation in the other direction: the runtime keeps the
   box in the `s16` vertices' own `[-1, 1)` space, which is what makes the GE's
   automatic `s16 / 32768` division and the `32768.0` above the same convention.
3. `Vex_UpdateNodeWorldMatrix` (`0x08944544`) applies it. In the
   `flags & 0x0f000000 == 0x00400000` branch: `addiu a0, s0, 0x54` /
   `lv.s S200, 0x0(a0)` at `0x08944640`, then `vmov.q C230, C030` to hold the
   parent's translation row aside and `vmscl.q E100, E000, S200` at
   `0x0894464c` to scale the 3x3 - the "translation restored unscaled" above,
   now read rather than inferred.

**The consequence for a parser: reading `scale` per batch is safe, and it is
safe only because the data is uniform.** The original looks at exactly one
batch's copy and applies it to the whole mesh, so a file whose batches
disagreed would be drawn at one of the two sizes with no complaint.
`every_batch_in_a_mesh_carries_the_same_position_scale`
(`crates/vex/tests/batch_position_gap_ground_truth.rs`) holds that up:
**21,055 meshes and 65,279 batches on `Data.wad`, with not one mesh whose
batches disagree** about `+0x10` bit-for-bit.

**Also settled by that walk: `+0x06` (`use_alternate`) is a live mechanism no
shipped batch uses.** `Mesh_BuildBatchDrawCommands` visibly branches on it
(`0x0892e900`), and every batch on PSP Pulse USA's `Data.wad` - all 65,279 -
declares an alternate vertex count of zero. Scoped to that archive, which is
the only one this walk covers.

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

A list ends on a record whose `pass_mask` does not carry that list's own bit
(`& 1` for list A, `& 2` for list B) - the walk's own terminator check - and
**that record occupies real space rather than being absent**: claiming a
fixed `0x40` bytes there (never the extended `0x80` form) leaves no residual
gap of that shape anywhere in the corpus swept, which is what took `Mesh`
coverage from ~98.9% to 99.84% once `oag_vex::mesh_coverage` started claiming
it as `"a batch-list terminator record"` instead of leaving it as an unclaimed
run "between a batch and end of file" (or, when list A's terminator sits
immediately before list B's first real batch, "between a batch and a batch").
Its own fields beyond `pass_mask` are not read by anything and are not decoded
here. **A list whose own offset is `0`** - `Skycube`'s list B on every sky,
see [skycube.md](skycube.md) - **is not walked at all**: zero means the list
does not exist, not that it starts at the mesh header, and reading it as the
latter would misclaim header and material bytes as a batch on any mesh whose
header word happens to satisfy the terminator test.

`+0x28`, `+0x29` and `+0x2c` are written at load and are **not file data**. An
earlier note describing `+0x2c` as a pre-compiled display list was describing a
runtime allocation.

**Two of those three are pinned more precisely as of 2026-08-09**, from
`psp-pulse-usa/BOOT.BIN` - see
[mesh-draw.md](../ghidra/functions/psp-pulse-usa/mesh-draw.md):

- **`+0x28` is not written at load. It is rewritten on every draw**, by
  `Mesh_BuildBatchDrawCommands` (`0x0892e8f0`), as
  `batch->0x28 = (batch->0x06 != 0)` - i.e. it caches "this batch uses its
  alternate vertex block", the same predicate `mesh_batches` computes as
  `use_alternate`. Reading it as load-time state is safe for a file parser but
  wrong for anyone modelling the runtime.
- **`+0x2c` is a pointer to an interned GE state-list cache entry**, not to a
  display list itself. `Mesh_InitBatch` (`0x0890e8b4`) sets it from
  `Gfx_AcquireBatchStateList` (`0x0891df48`), which interns on
  `(pass_mask, header_byte3, depth_lo, depth_hi, depth_bias)`; the display list
  is one further dereference away, at `entry+0x10`.

Mesh header:

```text
+0x00  u16      mesh flags
+0x02  u16      material_count
+0x04  u32      offset to batch list A, relative to the mesh payload
+0x08  u32      offset to batch list B, relative to the mesh payload
+0x10  f32[3]   bounding box min, model units
+0x20  f32[3]   bounding box max, model units
+0x30  material[material_count], stride 0x14
after the materials: a 0x40-byte texture-transform keyframe block (see below)
```

A batch belongs to list A while `pass_mask & 1` is set, and to list B while
`pass_mask & 2` is set.

### The texture-transform keyframe block, at `+0x30 + material_count * 0x14`

**2026-08-10, confidence 90.** The mesh payload carries authored
texture-transform animation - the data whose source
[`texture-animation.md`](../ghidra/functions/psp-pulse-usa/texture-animation.md)
had at "confidence 0" until this read. Immediately after the material array:

```text
+0x00  u16    offset-track key count
+0x02  u16    scale-track key count
+0x04  u32    offset-track times  (u16 each), relative to this block
+0x08  u32    scale-track times   (u16 each), relative to this block
+0x0c  f32    seconds per key-time unit - 1/60 on every block read
+0x10  u32    offset-track values (s16 u, s16 v per key, 1/256 units)
+0x14  u32    scale-track values  (same encoding; 256 = 1.0)
+0x18  f32[2] runtime out: scale u, v  (1.0, 1.0 in the file)
+0x20  f32[2] runtime out: offset u, v
+0x28  f32    runtime: last update time
+0x2c  f32    authored loop period in seconds; bit 0 of the word doubles as
              the step-vs-lerp flag
```

There is **one block per material**, `0x40` bytes each, and the array starts
where the materials end - the same array the runtime reaches as
`mesh+0x60 + material_index * 0x40`, relocated in place. A material that
authors no animation leaves its block's two counts at zero, which is the
engine's identity default; on `16_Track` most animated meshes have a single
material, but its hologram panels carry two blocks and animate at different
rates (one tile of `v` per 60 frames and per 120).

**A later block's `times`/`values` are relative to the start of the array, not
to the block that holds them.** Indistinguishable on a single-material mesh,
and the difference between real key data and noise on a second one.

**`+0x2c` is the loop period and it is not the last key time.** Measured across
`16_Track`: the flicker panels' tracks end at frames 12, 18 and 24 and all
three author a **50-frame** loop, which is what lets sibling meshes carry the
same steps at different key times and flicker against one another. Driving
them off their last key runs them at up to four times speed and collapses that
interleave into unison. (An earlier revision of this page said "600.0
everywhere read", which was true of the two boost-plume blocks it had read and
wrong as a generalisation.)

The engine relocates the two `times` and two `values` offsets to absolute
pointers and then uses the block in place; `TexAnim_UpdateTransform`
(`0x08927204`) evaluates both tracks per frame (wrap by `+0x2c`, divide by
`+0x0c` to reach key-time units, clamp outside the key range, lerp inside
unless the step flag is set) and the result reaches the GE as
`TEXSCALE`/`TEXOFFSET` words in the material's five-word list, gated on the
material's `& 0x10` flag.

`oag_vex::vex::mesh_tex_transforms` parses the array and
`TexTransform::sample` reproduces the evaluator;
`oag_mesh::mesh_render::TexAnims` is what draws through it.
`crates/render/tests/authored_uv_ground_truth.rs` pins both against the disc.

Read off `Data\Ships\Assegai\shipboost.vex` and Feisar's (byte-identical), on
both meshes of each: scale constant `(1.0, 1.0)`, offset `u` ramping
`2/256 -> 253/256` over key times 1..90 (frames), `v = 0` - the boost plume's
u-scroll, confirmed live against PSP RAM. Track circuits carry non-identity
blocks too (12-tile-per-4s `u` scrolls, 1-tile-per-second `v` scrolls, a
seven-key plateau), observed live on `16_Track` scenery.

**This narrows the older negative below rather than contradicting it**: the
material's `+0x0c..0x14` really is zero everywhere - the animation is just
not stored on the material. "Rules out the file as a source of animation
data" was the wrong conclusion from a correct scan, and is withdrawn.

**Moa Therma's magstrip is a checked negative instance, not an unexamined
one.** Neither its base-strip nor its far-LOD-overlay material carries the
`0x10` flag, and `mesh_tex_transforms` returns `None` for both -
`crates/render/tests/magstrip_ground_truth.rs`'s
`the_magstrip_material_carries_no_texture_transform`. The magstrip does not
animate; the mechanism exists and is played elsewhere on the same disc, this
surface simply does not use it. Separately: the magstrip's own texture name,
`_magsurface3_1verb.tga` (and a `Dmagsurface3_1verb.tga` variant on the "Dark"
track set), is the **only** place `"verb"` appears in any of 5,017 texture
names across all 307 version-6 `.vex` files - not a naming convention, so
there is nothing more general to decode from it. Left unexplained as an
authoring idiosyncrasy.

### List B is real, distinct geometry, not a second pass over list A

**Confidence: 90.** `oag_mesh::mesh` used to read list A only, on the theory
that B was a duplicate pass and reading both would draw the same surface
twice. Checked directly against `16_Track` (Talon's Junction): most meshes
that have batches in both lists really do share them (mesh `507`'s four batches
sit at offsets reachable from both `list_a_off` and `list_b_off`), but a large
number of other meshes have **zero** list-A batches and are made up entirely
of list-B ones - skipping list B does not avoid a duplicate for these, it
drops their geometry completely. A world-space reconstruction of every batch
(via `oag_vex::vex::world_transforms`, `mesh_batches` and
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

`oag_pulse::textures::Model` splits batches three ways: `draws` (opaque, neither
flag set), `alpha_tested_draws` (`is_alpha_tested()`), `transparent_draws`
(`is_transparent()`). Two more pipelines join the opaque one in
`oag_mesh::mesh_render::Built`, all drawn in the same pass, in that order:

- `alpha_test_pipeline` keeps depth write on and `discard`s pixels below a
  threshold in the shader (`fs_main_alpha_test` in `mesh.wgsl`) rather than
  blending - a cutout is meant to occlude and be occluded exactly like opaque
  geometry wherever its texel clears the threshold, which is what makes both
  the 94 texture-opaque batches and the 209 real-alpha-mask ones (trees among
  them) render correctly through the same pipeline.
- `blend_pipeline` is unchanged from the section above: depth write off,
  `TRANSPARENT_BLEND`, drawn last so the opaque and cutout geometry's depth is
  already resolved.

**Both halves are recovered, and the reference is per batch.** Confidence 86,
2026-09-10.
[`mesh-draw.md`](../ghidra/functions/psp-pulse-usa/mesh-draw.md#the-alpha-test-references-selector-and-the-pad-that-discriminates)
reads `is_alpha_tested()`'s branch of `Gfx_BuildBatchStateList` as
`Gu_AlphaFunc(GU_GREATER, ref, 0xff)` - so the function is `GU_GREATER`,
always - and the selector between `0x7f`, `0` and `0x10` is two bits of the
batch's own header, `pass_mask & 0x80` and `header_flags & 0x20`.
[`vex::Batch::alpha_test_reference`](../../crates/vex/src/vex/batch_flags.rs)
is that branch; `mesh::DrawCall::alpha_test_ref` carries it and
`mesh_render::cutout` builds one pipeline per distinct value, because a
circuit is one `Model` and mixes them.

Censused over three discs (`crates/vex/tests/alpha_test_reference_ground_truth.rs`),
three of the four bit combinations occur, and the third settles the branch
order: Wipeout Pure's `Speedup Pad`, 349 batches, is the only place
`pass_mask & 0x80` is set with `header_flags & 0x20` clear, and its glow
texture (`speedup_GLOW_KEY.tga`) tops out at alpha 58/255 - so the recovered
`0` keeps it and the rival reading's `0x7f` would discard it whole. It renders,
and `crates/render/tests/pad_alpha_test_ground_truth.rs` is what checks that it
does.

`ALPHA_TEST_THRESHOLD` (in `mesh.wgsl`) is now only the default for a draw
whose file authors no reference - a PS3 chunk, a Vita submesh, or synthetic
geometry this crate makes up.

**And `GU_GREATER` has to be reproduced as `GU_GREATER`.** `mesh.wgsl`
discarded at `shaded.a < alpha_test_ref`, which is `GEQUAL`, for as long as
Wipeout HD's `0.5` was the only reference in play - and there the two are
indistinguishable, because the alpha is 8-bit and `0.5` sits between `127/255`
and `128/255` where no texel can land. Every reference recovered off a `.vex`
batch lands on a texel value exactly, and at the `0` Pure's pad asks for the
`<` form discards *nothing*: `a < 0.0` is true of no fragment. Pure's
`Speedup Pad` painted its glow texture's fully transparent background as a
solid plate - a square where the pad shape should be - between b1ce4086 and
the fix. The discard is `<=` now, and `pad_alpha_test_ground_truth` captures
the pad twice, once as authored and once with the test forced off, because a
lit-pixel count on its own passes *harder* when a cutout stops cutting.
`ALPHA_TEST_THRESHOLD` moved to `0` with it: under `<=` that discards exactly
the fully transparent texel, which is what `1/255` under `<` did, so the
default paths keep the picture they had.

**This was not only a Pure fix.** The earlier `0.5` was checked against
*vertex* alpha alone (the census two paragraphs up), never the *texture*
alpha `shaded.a` actually gates on - and that same census records
alpha-tested textures whose range is a partial band like `58-78`, up to
0.306, still under `0.5`. A direct 1024x1024 capture of each full track model
(lit pixels = channel sum > 100) puts a number on it: `01_Track` 16,159 ->
16,156 (edge-antialiasing noise), `16_Track` 369,534 -> 370,987, **+1,453
pixels newly drawn (+0.4%)**. Small, and in the direction the fix predicts -
the old threshold was already discarding a sliver of real Pulse geometry too,
not only Pure's pad.

**And moving from that flat `1/255` to the per-batch reference costs less than
it sounds like it should.** The same instrument, rebuilt as
`crates/render/examples/threshold_probe.rs` and run at four yaws rather than
one, and re-measured under the `<=` discard: `01_Track` 85,704 -> 85,619 lit
over four frames with 1,681 pixels differing, `16_Track` 520,731 -> 520,690
with 6,553. Nothing structural leaves
the picture, and the lit count moves *up* at two of the eight framings -
because a cutout draw returns alpha `1.0` and writes depth, so a texel that
cleared `1/255` was painted solid and occluded what was behind it.

**All of that delta is the `0x7f` bucket, which discards no texels at all**;
isolating it reproduces both numbers exactly. The census's texel counts are
about the decoded image, and the shader samples it filtered and mipped - a
binary cutout's edge reaches the alpha test as a ramp, so `0x7f` cuts it at
half coverage where `1/255` cut at any, and a leaf tightens by about a pixel.
Do not quote a texel count as a claim about the picture.

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
Implemented as `oag_vex::vex::EmbeddedTexture::asset_path` and
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

### Texture rows are padded to 16 bytes

**Confidence: 90.** A texture's rows are **not** stored back to back. Each row
occupies `align_up(width * bits_per_pixel / 8, 16)` bytes, of which the leading
`width * bits_per_pixel / 8` are picture and the rest is padding - the GE's
texture buffer width is in units of 16 bytes, so a narrower row is padded out
and the next row starts on the next boundary.

This is a no-op for most of the disc, which is why it went unnoticed for so
long: a 4-bit texture is 32 pixels wide or more, and an 8-bit one 16 or more,
and the picture already fills the stride. It bites exactly the narrow ones.

The evidence is arithmetic, over a field nothing else reads. Every `Texture`
node declares its own total texel-block size at `+0x0c`; summing the padded
stride over the node's declared mip levels reproduces that number for **5,017 of
5,017 textures** on the PSP disc, while the unpadded sum reproduces **775** -
the ones where the padding happens to be a no-op. **509 of the 5,017 have a
padded base level**, i.e. are textures a back-to-back reader builds out of the
wrong bytes. Measured by
`crates/texture/tests/texture_stride_ground_truth.rs`.

Confirmed directly as padding rather than as some other packing: dumping
`col_arrows1_GLOW_ADD.tga` (16x64, 4-bit, so 8 picture bytes in a 16-byte row)
shows every row as eight content bytes followed by eight zeroes.

It is also **not swizzling**. A swizzled PSP texture is reordered into 16-byte
by 8-row blocks, and reading one row-wise corrupts every texture wider than a
block rather than only the narrow ones; every **unflagged** 64-pixel-wide 4-bit
texture on the disc decodes correctly read row-wise. A node whose flags byte
(`+0x06`) has bit 0 set *is* swizzled in the file - `pulse_bomb.tga` is 64 wide,
4-bit and flagged - and `vex::textures` unswizzles it, each level at its own
stride (measured live, 2026-10-01: [the shield
page](../ghidra/functions/psp-pulse-usa/shield-pickup.md#2026-10-01-third-pass-the-shells-own-ge-state-and-why-ours-looked-dim)).
(The standalone [`.mip` container](psp-texture.md) carries the same flag at `+0x07`.)

**Corroborated from the runtime, 2026-08-08, which raises this from a data
measurement to a mechanism.** The PSP texture-upload path counts every mip
level in **16-byte units and cannot express anything else**:
`FUN_08928704(texture, level)`, the per-level size used to lay levels out in
VRAM, is a one-line `return *(u16 *)(texture + 0x5c + level * 2) << 4` - a
u16 block count shifted up by four. Its caller `FUN_089287c0` stores each
level's address the same way (`>> 4` into `texture+0x84`) and traps when the
value it is handed has any of its low four bits set. Both are reached from
`Gfx_BindTexture` (`0x08928460`) through `FUN_08928550`, the VRAM upload. So a
level whose rows were packed to an unpadded stride would have a size the
runtime has no way to represent, which is the same conclusion the 5,017-of-5,017
closure reaches from the file side and independent of it. Neither function is
renamed here: the upload path is otherwise unread and a name would claim more
than one line each.

**What the old reading looked like on screen.** With the rows read back to back,
a 16-pixel-wide 4-bit texture came out as its own top half stretched over the
whole quad with every other row blank, and an 8x8 one as its first two buffer
rows repeated. On `16_Track` the visible casualty was the start-line gantry's
advertising board: node 74's opaque batch paints
`GenericTrackTextures\billboard8.tga`, an 8x8 4-bit icon spanning exactly one
tile of UV, and the wrong bytes made it a blown-out white block hanging over the
track. This was for a while attributed to the `col_banners2_ADD.tga` sign beside
it, from the texture name; hiding every `col_banners2` draw left the white block
exactly where it was, and hiding the opaque list removed it. Both checks are
`oag-view --draws` / `--only` (see [`oag-view`](../tools/oag-view.md)).

The full affected set on `16_Track` is `billboard1/2/6/7/8.tga`,
`grey_swatch.tga`, `whitenonalpha.tga`, `hologram_effect_ADD.tga`,
`strut__light_rp_GLOW.tga`, `lightglow_anim_ADD.tga` and
`col_arrows1_GLOW_ADD.tga`; disc-wide it also covers every ship's
`Glass_ADD.tga` (16x16) and most of `Data\Weapons\Textures`.

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
`oag_vex::vex::textures` checks the header's declared length before touching
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

All three fields are read, as `oag_vex::vex::Material`.

### `+0x0c..0x14` is empty, and `flags` is render state

Two negative results, both from surveying every material of all 12 PSP circuits
(1,524 on `07_Track` alone). They matter because between them they rule out the
**material record** as a source of animation data. (An earlier revision said
"the file"; that was too broad - the animation is authored per *mesh*, in the
keyframe block documented above, 2026-08-10.)

**The unused tail is zero everywhere.** `+0x0c..0x14` is eight bytes wide - the
right size for a `u`/`v` scroll rate pair, and the exhaust's `Trail` does carry
exactly such a pair in its own descriptor. It is `0000000000000000` on every
material of every circuit. **There is no authored per-surface UV scroll rate in
this format.** Confidence 90: an exhaustive scan of real data, which is what the
[rubric](../reverse-engineering/confidence-rubric.md) caps below 95.

**`flags` as a whole does not mark animation, but one of its bits does.**
Corrected 2026-08-18: this paragraph read "`flags` does not mark animation" and
offered "the static `hub_banner_GLOW` and the banded `flicker1nonalpha_GLOW`
both carry `0x91`" as the proof. Both of those surfaces are in fact animated -
see the box at [Trackside advertising](#trackside-advertising-is-static-and-that-is-measured-on-both-axes) -
so the example proved nothing, and `0x91` carries `0x10`, which is precisely the
bit `Mesh_UpdateTextureTransforms` (`0x0890e160`) tests before evaluating a
material's keyframe block. On all twelve circuits every one of the 922 materials
carrying it authors a non-empty block; the converse was not measured. What the
word still does not do is separate animated from static by its *value*: it is
richly varied - 19 distinct values on `07_Track`, from `0x0001` on plain art to
`0x2001`, `0x0192` and `0x0292` - and correlates with the artists' naming.
Three bits are legible:

| Bit | Where it lands | Reading |
| --- | --- | --- |
| `0x0010` | 922 materials over the twelve circuits, 27 to 125 each | the engine's own texture-transform gate, read at instruction level in [`texture-animation.md`](../ghidra/functions/psp-pulse-usa/texture-animation.md) |
| `0x0080` | `07_archtrim_Glow_01`, `SL_BlueStrip_GLOW`, `col_display7_GLOW`, `07_Pulse_light_BLEND_GLOW` | accompanies the `_GLOW`/additive naming convention |
| `0x2000` | **exactly** the `*_shinemap` textures - `SL_stripwindows_shinemap`, `rf_dome2_strip_shinemap`, `StripWindows_shinemap` | the extra pass, corroborating the same bit's meaning in a batch's `pass_mask` and the second texture index at `+0x08` |

The `0x2000` correlation is the useful one: a bit whose name was guessed from
`pass_mask` turns out to select precisely the textures whose *artist-given name*
says they are environment maps. **Confidence 75** - two independent namings
agreeing, with no code read behind it - since read: `FUN_0890db54`
(`Mesh_DrawExtraPassBatches`) draws every batch with `pass_mask & 0x2000`
under `TEXMAPMODE` 2 with the texture at `+0x08`, and a recorded GE list on a
live Assegai hull shows it doing so (`mesh-draw.md`, "The hull's extra pass").
`second_texture` is 0 on every `07_Track` material, so the pass is not
exercised there; **a Pulse hull draws it** (`oag_render::shine`), a track does
not yet. On a hull the texture is `envtest4bit.tga` (and `envmap_stripe2.tga`
for three teams' canopies), not a `*_shinemap`.

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
`oag_vex::vex::mesh_materials`/`textures`) turns up a single, consistent
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

`oag_pulse::textures::is_blink_light_texture` matches on the decoded texture name
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
> See [`texture-animation.md`](../ghidra/functions/psp-pulse-usa/texture-animation.md).
> Read the section below as the survey of the texture's contents - which is what
> it is good for - not as a recovered mechanism.
>
> **Un-superseded, 2026-08-10: the V scroll was the mechanism after all,
> and it is authored per mesh.** The 2026-07-31 negative measured the wrong
> choke point - the animated transform reaches the GE through per-material
> compiled lists that never call `Gu_TexOffset` (see texture-animation.md,
> "Two earlier readings this corrects"). Every team's
> `colours_flashing_GLOW` mesh carries the texture-transform keyframe block
> documented above with the identical track: `v` scrolling one full tile
> over key times 1..60, looping at an authored 1.0 s. The colour candidate
> is dead. The section below is therefore a recovered mechanism's *content*
> half again, not just a texture survey.


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
`crates/mesh/src/mesh.wgsl` subtracts the scroll offset rather than adding
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

> **Withdrawn 2026-08-18. The billboards scroll, and the disc authors it.** The
> measurement below stands for what it actually tested - a *filmstrip*, stepping
> through a banded texture's rows or blocks - and its conclusion was then
> over-generalised to "do not move, on either axis". The per-material keyframe
> blocks say otherwise for the second axis: `hub_banner_GLOW`, named below as the
> static counter-example, sweeps `u` by **four whole tiles** with a half-tile `v`
> step over a 10 s loop on both circuits that draw it, and seven more banner and
> logo textures carry their own offset tracks. A quad spanning the full width of
> its texture is evidence against a filmstrip and no evidence at all against a
> `TEXOFFSET` scroll, which slides the coordinates whatever they span. The counts
> and the tracks are in
> [`scenery-animation.md`](../rendering/scenery-animation.md); the renderer
> already plays them.

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

So the billboards' contents render and do not step through frames. The sentence
that stood here said "render and do not move, on either axis"; the stepping half
is what these two measurements support, and the moving half is what the box at
the top of this section withdraws.

**That last measurement does not generalise to non-billboard scenery, and a
reading built on it was wrong.** The renderer used to carry a scalar V rate per
vertex on the strength of it. The authored blocks say otherwise: of the 17
distinct tracks on `16_Track`, several are pure **U** scrolls - including
`col_display7_GLOW`, which is on all twelve circuits and which the V-only table
therefore animated on the wrong axis - and others are diagonal. The per-vertex
attribute is now an index into the model's authored tracks
([`GpuVertex::anim`]), which carries `u`, `v`, scale and stepping alike.

A narrow band is evidence for *which* textures, not a runtime rule. The engine
scrolls a shared texture globally and any geometry sampling it inherits the
animation whatever its V span - Assegai's and Piranha's own blink quads span 8.1
and 9.6 of 16 rows and animate regardless - so gating at runtime on a narrow
band would switch those two ships off.

**Confidence 65** for the track surfaces, deliberately below 70: the geometry
and the banded texture content are real measurements, but nothing had confirmed
that the original animates these particular surfaces, and the rates are reused
from the blink light rather than recovered.

**The mechanism is settled, and it is not this one.** The surfaces above are
animated by the per-material keyframe block described in
[The texture-transform keyframe block](#the-texture-transform-keyframe-block-at-0x30--material_count--0x14),
whose compiled five-word display list writes `TEXOFFSET` directly and never
calls `Gu_TexOffset` - which is why a breakpoint on that function saw only the
exhaust ribbon and why the negative it produced did not mean what it looked
like. The renderer replays the authored tracks, so `[graphics]
animated_textures` now defaults **on** *(and was removed outright on 2026-08-18 - see [`scenery-animation.md`](../rendering/scenery-animation.md))*: it is a reproduction rather than a
departure, and what the switch does off is freeze every animated surface at
the first key of its track, for still-frame comparison. See
[`texture-animation.md`](../ghidra/functions/psp-pulse-usa/texture-animation.md).

The geometric survey above is kept, and it is worth being clear about what it
is now: a record of which surfaces the *geometry* singles out, useful as a
cross-check on the authored blocks, and not what drives the picture. Where the
two disagree the blocks win - they are the original's own data.

One reading is narrowed rather than ruled out: the material `flags` **word** is
not an animation switch by its value, but its `0x10` **bit** is exactly the
engine's gate - the counter-example this paragraph used to give, "the static
`hub_banner_GLOW`", turned out to be animated, and `0x91` carries `0x10`. See
the corrected paragraph under
[`+0x0c..0x14` is empty, and `flags` is render state](#0x0c0x14-is-empty-and-flags-is-render-state).
A second is now explained rather
than ruled out - material `+0x0c..0x14` is indeed zero on every material of
every circuit, because the animation is stored in the block after the material
array, not on the material.

## `Anim Transform` `0x3c0`: the keyframed node matrix

The other animation mechanism, and the one that moves geometry rather than
sliding a coordinate under it. Every Pulse circuit authors it - **393 nodes over
the twelve, with 474 meshes below them** - and the class is read whole:
registration, binder and all three channel evaluators are in
[`anim-transform.md`](../ghidra/functions/psp-pulse-usa/anim-transform.md), which
is also where the field map's evidence lives. `oag_vex::vex::anim_transform`
parses it and `oag_pulse::textures::AnimNode` plays it.

The payload is a `0x50`-byte header followed by six key arrays - translation,
rotation and scale, each a `u16` time array in 60 Hz frames paired with
`(s16, s16, s16)` values:

- **translation** is `value * quantum + base`, with a per-axis quantum at
  `+0x20` and the base at `+0x10`;
- **rotation** is a **compressed unit quaternion** - three components in 1/32767
  units with `w = sqrt(1 - x^2 - y^2 - z^2)` - slerped between keys;
- **scale** is **1/256 fixed point**, the same convention the texture-transform
  block uses, multiplying the basis rows.

Composition is scale, then rotate, then translate. Between keys the evaluators
clamp and lerp exactly as `TexAnim_EvalKeyframes` does for the texture block, and
the node's `FixedFrames` attribute snaps to the preceding key instead - the same
role that block's step flag plays.

**A count of zero still stores one key**, which is what makes the six arrays
**tile `[0x50, payload_len)` exactly on every one of the 393**. That tiling is
the field map's evidence: it is falsifiable on every node of every circuit at
once, and a wrong offset or stride breaks it on the first file.

### The node header carries named attributes

Three of them are read by this class's binder, through a list the header
declares - and finding it settles a field that had been recorded as unknown.
The `u16` at header `+0x0e` is the **list's length in bytes**, which is why its
values were "small and round (24, 36, 48, 56, 68, 368)"; header `+0x06` is the
offset to the first entry. `oag_vex::vex::node_attributes` walks it.

| Attribute | Effect |
| --- | --- |
| `LoopEnd` | the wrap period; `AnimTransform_Update` does `fmodf(t, LoopEnd)` |
| `AnimEnd` | stored and, so far as anything read shows, never used |
| `FixedFrames` | snap to the key instead of blending |

Both times default to 6,000 seconds, so a node with no `LoopEnd` does not loop.
The lookup is a `strcmp` and therefore case sensitive: **three nodes author
`Loopend`**, which the original does not match, so those three do not loop -
reproduced rather than corrected.

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
| `0x08a7ff6c` | `%s\%s_FE.vex` |
| `0x08a7ff7c` | `%s\ship_eliminator.dat` |
| `0x08a7ffa4` | `%s\%s.dat` |
| `0x08a884dc` | `Data\Psys\%s.POB` |
| `0x08a88e94` | `Data\Music\FEMusic\frontend%d.at3` |

The leading `%s` is a directory read from a scene-graph node field at `+0x94`,
sourced from XML rather than from the binary.

The last three of those are one site, and it is traced:
[`ship-skin.md`](../ghidra/functions/psp-pulse-usa/ship-skin.md) reads the
function that formats all three, and decodes the `.dat` they name as four
paletted textures that replace the hull model's own. Note that **none of these
strings has a Ghidra xref**, under either the relocated or the naive address -
that page records how the reference was found instead. Track directory names are
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

`Ship_LoadModel` picks between `Ship.vex` and `Zone.vex` on the race-mode
selector - see [zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md#the-ship-model-is-not-the-players-own-hull).

Applied, from
[names.tsv](../ghidra/functions/psp-pulse-usa/names.tsv). See
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md).

## Not determined

- ~~The `section` and `WO Track` node payloads~~ — recovered, see
  [track data](track.md). Note the correction: `section` is a **visibility
  partition**, and the spline lives in `WO Track`.
- **Primitive type values.** No code inspects them; the byte goes straight to
  `sceGuDrawArray`. Observed 3 (triangles) and 4 (strip) in real models. A strip
  needs degenerate triangles to join, since the GE has no primitive restart.
- Whether the alternate vertex block overlaps or follows the primary one.
- Batch `+0x14`'s exact meaning, and the `.x`/`.z` components of the s16
  bounding box. **Ruled out, 2026-08-25: it is not padding.** Censused across
  all 65,279 non-VIF PSP batches on `Data.wad`
  (`crates/vex/tests/batch_position_gap_ground_truth.rs`): read as an
  `f32`, it is nonzero, positive and finite on every one of them - the
  signature of a real value, not an unused gap. Its magnitude does not track
  the batch's own `scale` (the ratio to `scale` spans 0.0004 to 2.0 with no
  central tendency), but sits closer to a fixed band against half the batch's
  own bounding-box diagonal (ratio 1.10-2.65 at the 10th-90th percentile,
  median 1.46) - consistent with, but not proof of, some kind of per-batch
  bounding radius. Below the confidence this project renames anything at -
  recorded as a lead, not a decode.

  **Narrowed twice more, 2026-09-07, both negatives.** First: *nothing in
  `psp-pulse-usa/BOOT.BIN` reads it.* Every function that walks a batch list
  was enumerated by the `payload_size` load each one needs (`lhu ..., 0xc(...)`,
  55 sites program-wide) and each checked for any access at `+0x14`; all that
  come back are stack slots. Independently, every float load at `+0x14` in the
  whole executable was enumerated across all register classes (`lwc1` against
  `s`/`a`/`v`/`t` bases, plus `lv.s`) and the only one anywhere in mesh code is
  `Mesh_InitFromPayload`'s `0x0890efac`, which is the **mesh header's** bounding
  box, not a batch's. Confidence **80**, and the limits are worth stating: an
  integer read, or a read through a base register already offset by `0x10`,
  would evade both sweeps. Contrast `+0x10` beside it, whose single consumer the
  same sweep found immediately (above) - so the sweep does find what is there.
  Second: *the two-packed-sub-fields reading is disfavoured.* Across the 65,279
  batches the low half takes 20,979 distinct values and is zero on 4, and the
  high half 1,252 - full mantissa noise under a genuinely spread exponent, not
  the constrained or small-integer half a packed pair would show. It is also
  **not** mesh-wide data: it is uniform within only 11,167 of the 21,055 meshes,
  where `+0x10` is uniform within all of them.

  What this leaves: a per-batch quantity the PSP runtime authored and never
  reads. The next place to look is another title's executable - `ps2-pulse-eu`
  or `psp-pure-usa` - not more geometry fitted against six `s16`s.
- **Collision mesh representation**: see
  [collision](../ghidra/functions/psp-pulse-usa/collision.md), now decoded.
- Exact `pass_mask` bit meanings. Partial: `0x800` means the batch has its own
  display list, `0xc0` relates to alpha, `0x2000` to an extra pass. `0x1000` is
  now decoded - it is the mesh-layer discriminator between draw keys
  `0x45000000` and `0x4a000000` (only consulted when the mesh's own key is
  already `0x45000000`), confirmed live in `Mesh_CompileBatchSet` (`0x0892f35c`)
  - see [draw-order.md](../rendering/draw-order.md) and
  [mesh-draw.md](../ghidra/functions/psp-pulse-usa/mesh-draw.md#the-layer-derivation-and-what-it-is-worth).
- Whether the 16-byte file header carries anything beyond the version.
- ~~The `.dat` format paired with ships.~~ - decoded, see
  [handling-stats.md](handling-stats.md#related-files) and
  [ship-skin.md](../ghidra/functions/psp-pulse-usa/ship-skin.md): a `0x20`
  team-name header plus four 4bpp palette+pixel blocks, a texture swap on the
  hull model rather than a second mesh. Censused across both PSP pressings and
  the PS2 disc for this pass (2026-09-16): 16 base-team files on
  `pulse-psp-usa`/`pulse-psp-eu` and 24 (base plus all four DLC teams) on
  `pulse-ps2-eu`'s `WADSP.WAD`, every one exactly 26912 bytes, PSP hashes
  identical between the two pressings; Pure ships zero.
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

## Wipeout HD ships version 6 too, byte-swapped

**Confidence 92**, on three independent arithmetic invariants across 40 files.
The full survey is [hd-status](hd-status.md); what belongs here is the shape.

A PS3 `.vex` is this format, big-endian. The header word at `+0x0c` reads
`XXEV` - `VEXX` with its bytes reversed - and every field above, the `u16` child
count included, reads at the same offset with the opposite byte order. On all 28
of HD's circuits and 12 of its front-end models the header arithmetic closes, the
node walk lands exactly on the tree end, and immediate child counts sum to node
count minus one.

**The class IDs are Pulse's**, not a renumbering like [Pure's](pure-status.md#the-class-id-space-is-renumbered):
`Transform 0x06e`, `Mesh 0x125`, `WO Track 0x3bb`, `section 0x3c9`, both pad
classes, all five of Pulse's collision classes and the whole environment
group resolve
against `classes::V6` unchanged. One class was not in that table until
2026-08-17: **`0x3ed`**, authored once per circuit under the node name
`collision_trackwall`, and it is **a sixth collision class** - the barrier along
the road, where `Wall Collision` on HD is the wider scenery. What established
that is not the node name but its geometry, over all 16 circuits: see
[hd-status](hd-status.md#0x3ed-is-the-barrier-along-the-road).

Note what listing it in `classes::V6` does and does not say. **The table above
has now been read to its terminator (2026-08-26) and `0x3ed` is not in it** -
Pulse's game-class ids run `0x3b9`..`0x3eb` and stop, three short of HD's
866-record table. See [the resolution above](#node-types) for the address and the count. Version 6 is a format generation and HD's files
are version 6, so the ID still belongs in the version-6 table; the constant's
name is HD's spelling of the node, not a Pulse-attested table entry.

**What is not there is the geometry.** Everything under
["Geometry is pre-batched GE display lists"](#geometry-is-pre-batched-ge-display-lists)
is PSP-specific in a way this page did not have to say before: on the PS3 a
`Mesh` node's payload is 224 to 22,176 bytes of *description* - a bounding-box
pair at `+0x10`/`+0x20`, `min <= max` componentwise on 1,638 of 1,638 nodes, and
a per-node 32-bit word that is the obvious candidate for a reference - and the
vertices live in a `.rcsmodel` file beside the `.vex`. Talon's Junction carries
90 KB of `Mesh` payload where Pulse's `16_Track` carries 3.49 MB, with a 24.7 MB
`track.rcsmodel` next to it.

So the node tree, the scene hierarchy, the track payloads and the class space are
all one format across three consoles and three titles; the vertex encoding is the
part that has been rewritten every time.

## The Omega Collection's PS4 build reads the same version-6 `.vex`, unmodified

**Confidence 90, measured 2026-09-16 (`lane/omega-rcs`).** `omega-ps4-eu`'s
`data03.psarc` carries `Data/art/published/hdships/auricom/Ship_LOD.vex`
(7,792 bytes): `VEXX` at `+0x0c`, little-endian - this format's own byte-order
sniff already handles that with no code change, since it reads the magic's
byte order rather than assuming one. `oag_vex::vex::nodes` decodes it to four
real nodes with real names (`world`, `persp1`, `Ship_root`, `EngineShape`),
the last a `Mesh` (class `293`) child of `Ship_root` - a real node tree, not
noise. This is the positive half of the byte-order question the psarc lane's
own session opened alongside `.rcsmodel`'s: `.gnf` and `.vex` are both stored
little-endian on this platform where PS3 stores them big-endian, and `.vex`'s
own magic-sniffed reader already took that in stride; `.rcsmodel`/
`.rcsmaterial` are the two formats that turned out **not** to be a simple
byte-order flip - see [rcsmodel.md](rcsmodel.md#the-ps4-omega-collection-a-different-container-not-a-byte-swap-of-this-one).

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
