# Collision geometry

**Status: decoded and validated.** The chunked triangle-soup payload parses on
**every collision node on both discs** - 130 on the PSP disc and 189 on the PS2
one - with every byte accounted for, and is implemented in
[`oag_formats::collision`](../../crates/formats/src/collision.rs). The checks are
kept as tests
([`collision_ground_truth.rs`](../../crates/formats/tests/collision_ground_truth.rs)),
not as prose.

Collision geometry lives in the track's [`.vex`](vex.md) file as ordinary scene
nodes, under five class IDs. It is an **indexed triangle soup**: no BSP, no
quadtree, no heightfield, and **not the render mesh**. The function behaviour
behind it - the broadphase, the raycasts, the contact rules - is documented
separately in
[the Ghidra page](../ghidra/functions/psp-pulse-usa/collision.md); this page is the
file format and what shipped data says about it.

Three things that would be reasonable guesses and are wrong:

- **The visual mesh is not the collision mesh.** They are separate node types
  with separate geometry, and reusing one for the other would be wrong.
- **Magstrips are not special geometry.** A `Mag Floor Collision` node is
  ordinary floor with a different surface type.
- **The vertices are already in world space.** All 319 nodes sit at depth 1 with
  an identity world transform, so a consumer that composes the `.vex` transform
  chain over them moves them twice.

## What a real track contains

`Data\Environments\01_Track\track.vex` carries four collision nodes, out of
2,071. Three of their payload sizes, measured here: `Floor Collision` 62,080
bytes, `Wall Collision` 39,296, `Reset Collision` 160. Its `Mag Floor Collision`
is listed as 9 KB by [track data](track.md#what-a-real-track-contains)'s node
census, which is that page's figure rather than one taken here. The census across
both discs:

| Node | Class ID | Surface type | PSP nodes | PS2 nodes |
| --- | ---: | ---: | ---: | ---: |
| `Floor Collision` | `0x3b9` | 1 | 40 | 59 |
| `Wall Collision` | `0x3ba` | 0 | 40 | 59 |
| `Reset Collision` | `0x3cd` | 2 | 26 | 32 |
| `Mag Floor Collision` | `0x3e6` | 3 | 24 | 33 |
| `Cage Collision` | `0x3e7` | - | **0** | **6** |

Totals: 319 nodes, 18,745 objects, 602,086 vertices, 594,615 triangles.

Objects are small. A track's floor is 85 to 127 separate soups of roughly 32
vertices each rather than one large one, which is what the per-mesh sweep and
prune is sized for.

## The layout

```c
struct CollisionPayload {   // 8 bytes, then the objects
    u32 header_word;        // 0xffffffff on every node; see open questions
    u32 object_count;
};

struct Object {             // 4 bytes, then the chunks
    u32 chunk_count;        // 3 on every object seen
};

struct Chunk {              // 8 bytes, then count * stride payload bytes
    u32 type;               // 1, 2 or 3
    u16 stride;             // element stride, matching the type
    u16 count;              // elements, not bytes
};
```

| Chunk | Type | Stride | Contents |
| --- | ---: | ---: | --- |
| Vertices | 1 | `0x0c` | 3 x `f32` position |
| Triangles | 2 | `0x06` | 3 x `u16` index |
| Vertex scalars | 3 | `0x04` | one `f32` per vertex |

Every object in both builds stores exactly three chunks, in the order **1, 3,
2** - the scalars *before* the indices. That ordering is not arbitrary: an odd
triangle count leaves the index array ending on a half-word, so a chunk of
`f32`s placed after it would start 2-byte aligned, which a MIPS `lwc1` cannot
load. Putting the scalars first keeps every `f32` array 4-byte aligned. The
decoder still locates chunks **by type rather than position**, because nothing in
the format requires the order and reading by position would silently transpose an
array.

### The checks that settle it

Four invariants, all exact, with no exception anywhere in either build.

**1. The payload closes.** A node's payload is padded to a 16-byte boundary with
zeros, and the chunk walk consumes everything else:

```
8 + sum over objects (4 + sum over chunks (8 + count * stride))
    rounded up to 16  ==  node data_size
```

**319 of 319 nodes, exact.** 53 need no padding at all; the rest need 2 to 14
bytes, always even, always zero-filled, never 16 or more. A parse that is one
field out cannot make that come out even on one node, let alone 319, which is the
same argument that settled [`WO Track`](track.md#the-check-that-settles-it). It
is also what pins the per-object chunk count at 32 bits: as a `u16` the walk
would be two bytes short per object, which for an 85-object node is 170 bytes,
not 10.

**2. The chunk header's second field is the element stride.** It equals the
stride its type implies on **56,235 of 56,235 chunks**. The loader hard-codes
strides and never reads the field, which is why it was recorded as an unknown
`u16`; the decoder checks it, so a walk that drifts out of alignment fails on the
very next chunk instead of decoding plausible garbage.

**3. The scalar array is per-vertex.** Its count equals the vertex count on
**18,745 of 18,745 objects**. This was a falsifiable prediction, from
`0x0881835c` indexing the array by a triangle's three vertex indices: a
per-triangle array would have been the wrong length on almost every object, since
vertex and triangle counts differ per object (2,718 against 2,699 on
`01_Track`'s floor alone). The decoder refuses a mismatch and its error carries
both counts, so if such an object ever appears the rival reading is visible
rather than silently accepted.

**4. Every triangle index names a vertex.** All 1.78 million indices, no
exceptions.

Confidence **94** for the whole layout, including each of the four above
separately. This is data-agreement evidence - an exact arithmetic invariant across
many real files - which the [rubric](../reverse-engineering/confidence-rubric.md)
caps at 94, and it is corroborated on a second platform's build. It does not
reach 95 because nothing has been traced under an emulator: the layout is
self-consistent and the engine's use of it is still read rather than observed.

### The class IDs are the format's own signature

The decoder doubles as a detector. Pointed at **every** node of **every** `.vex`
file on both discs - about 132,000 nodes - a payload that parses, ends exactly on
its 16-byte boundary and opens with the all-ones header word occurs under
**exactly the five collision class IDs and no others**. So the IDs recovered from
the class table at `0x08ab2370` and the node types carrying this format are the
same set, from two directions. Confidence **94**.

## `Cage Collision` is a platform difference

The Ghidra reading was that cage collision is parsed and then skipped by the
loader: "dead content, or another SKU". The data answers the disjunction. **The
PS2 disc ships 6 cage nodes and the PSP disc ships none at all**, and the
per-track plugin definition carries a `collisionCageEnabled` attribute (see
[track data](track.md#what-a-real-track-contains) for the XML). Confidence **92**
for the node counts, which are a census rather than an inference.

That the PS2 loader also drops them, or that the attribute is what gates them, is
**not** established - the loader was only read for the PSP build, and nothing was
traced. Confidence **70** for the explanation, as distinct from the counts.

## Surface types and the friction sentinel

All five classes are registered with one vtable and share one parser. The class
ID selects a surface type and one `f32` at `collider+0x64`, nothing else.

That float is `0.05` for `Wall` and **`-1.0` for `Floor`, `Mag Floor` and
`Reset`**, and the negative value is a **sentinel, not a coefficient**:
`Collision_AddContact` (`0x08816864`) averages the two colliders but forces zero
if either side is negative. So a floor-against-wall contact is `0.0` rather than
the average `-0.475`, and a negative never reaches the response - which matters,
because a negative coefficient in that response would *add* velocity instead of
removing it.

`oag_formats::collision` therefore models it as `Option<f32>`, where `None` means
"no effect", and `combine_friction` is the rule rather than a comment.
Confidence **88**, unchanged from the Ghidra reading: this is decompilation
evidence, and no shipped byte bears on it.

**It is friction, and this page called it restitution until 2026-07-28.**
`Body_ResolveContact` (`0x0884e968`) takes its restitution from `body+0x388`
(`0.4` for a craft) and uses the combined `collider+0x64` only to scale the
contact's *tangential* relative velocity, once per frame, with no Coulomb clamp.
Averaged against the ship collider's own `0.02` that gives `0.035`, and a craft
scraping a wall loses exactly that fraction of its speed every frame - the
"missing resistance force" of
[force-balance-ground-truth.md](../physics/force-balance-ground-truth.md). The
values and the sentinel rule on this page were right; only the field's name was
wrong. See
[contact-response.md](../ghidra/functions/psp-pulse-usa/contact-response.md).

## The same format is in Wipeout Pure

Pure PSP renumbers the entire class-ID space - `0x6d` and `0x11e` dominate its
23,677 nodes where Pulse has `Transform` at `0x6e` and `Mesh` at `0x125` - and
**none of Pulse's five collision IDs appears in it at all**. Running the detector
above over Pure instead finds three class IDs whose payloads decode exactly:

| Class ID | Nodes | Objects | Vertices | Triangles |
| ---: | ---: | ---: | ---: | ---: |
| `0x36b` | 16 | 1,006 | 32,316 | 31,954 |
| `0x36c` | 16 | 977 | 34,485 | 31,075 |
| `0x37f` | 16 | 101 | 3,320 | 2,968 |

16 nodes each, one per track, with the same all-ones header word and the same
closure. Confidence **90** that this is the same format, from 48 of 48 nodes
closing exactly with zero false positives elsewhere in the file set - the first
direct evidence of asset-pipeline reuse for collision specifically, which is the
premise [future-2048](../future-2048/shared-concepts.md) is built on.

The renumbering is not specific to collision: Pure shifts the **whole** class-ID
table, and the [Pure probe](pure-status.md#the-class-id-space-is-renumbered)
pins `Transform`, `Mesh`, `Texture` and `WO Track` there by exact invariants.
The five collision classes remain the ones it could not place.

**Which of the three is floor, wall or reset is not determined.** The object
counts look like Pulse's shape - two large classes and one small one - and
analogy is not evidence. Confidence **45**, so nothing is named and no constants
were added for them.

## The per-vertex scalar is authored and unused

The `f32` per vertex reaches the raycast hit result, averaged over a triangle's
three corners by `CollisionMesh_AvgVertexScalar` (`0x0881835c`), which returns
`1.0` when the array is absent. What it means was recorded as unknown, with "grip,
friction or roughness" a guess at confidence 40.

The data narrows the question in an unexpected direction: **every one of the
602,086 scalars in both builds is exactly `1.0`**. The array is always present -
no object omits chunk 3 - and always neutral. So no shipped track uses the field,
and reading its meaning off the data is impossible in principle. Confidence
**94** for the observation.

That is worth knowing before spending time on it: whatever it is for, a faithful
reimplementation gets identical behaviour by treating it as `1.0` everywhere, and
the only way to learn what it does is to read the consumer, not the assets. The
ground-truth test asserts zero non-neutral scalars, and its failure message says
outright that a failure there is a **finding**, not a bug.

## Open questions

### Resolved: the broadphase clamps world space rather than rebasing it

Collision vertices are world space: every node sits at depth 1 with an identity
world transform, on both discs.
[The sweep-and-prune reading](../ghidra/functions/psp-pulse-usa/collision.md#two-level-sweep-and-prune)
packs endpoints as `bits[0:11] = ((int)coord + 0x400) * 2`. Twelve bits hold
`0..4095`, so `coord + 1024` runs `0..2047`: the packing covers a **2048-unit
window, centred on the origin**, quantised to one unit.

This page previously recorded only whole-disc maxima - 1,335.8 on PSP and 1,554.1
on PS2 - and left it at "both cannot be right as stated". **Every track has now
been measured**, with `oag-view --collision` over both discs, and the answer is
sharper than a contradiction.

Both numbers below are measured over the **union of all collidable classes in one
file**, which is the set the *world-level* broadphase would hold. The second,
per-mesh level packs a single mesh's endpoints, and a mesh is far smaller - 32
vertices or so - so nothing here says anything about that level.

| Environment | Reach from origin | Widest span | Over ±1024 | PSP | PS2 |
| --- | ---: | ---: | :-: | :-: | :-: |
| `01_Track` | 884.6 | 1321.7 | | yes | yes |
| `02_Track` | 1015.6 | 1517.9 | | yes | yes |
| `03_Track` | 819.7 | 1476.0 | | yes | yes |
| `04_Track` | 837.1 | 1601.8 | | yes | yes |
| `05_Track` | 1005.9 | 1564.2 | | yes | yes |
| `06_Track` | 1038.8 | 1425.3 | **yes** | yes | yes |
| `07_Track` | **1335.8** | 1250.6 | **yes** | yes | yes |
| `08_Track` | **1554.1** | 1739.1 | **yes** | - | yes |
| `09_Track` | 886.5 | 1242.5 | | yes | yes |
| `10_Track` | 1072.2 | **2026.6** | **yes** | yes | yes |
| `11_Track` | 1129.2 | 1757.8 | **yes** | - | yes |
| `12_Track` | 979.7 | 1693.4 | | - | yes |
| `13_Track` | 1155.4 | 1711.2 | **yes** | yes | yes |
| `14_Track` | 969.8 | 1748.2 | | yes | yes |
| `15_Track` | 1132.4 | 1755.2 | **yes** | - | yes |
| `16_Track` | 856.2 | 1481.3 | | yes | yes |

Measured on `pulse-psp-usa.chd` and `pulse-ps2-eu.chd`. "Reach" is the furthest
any collidable vertex sits from the origin on any axis; "widest span" is the
longest edge of the collidable bounding box.

**Two facts settle it.**

1. **Seven of sixteen tracks exceed ±1024**, so this is not one outlier. The
   whole-disc maxima this page used to quote are simply `07_Track` (PSP) and
   `08_Track` (PS2); they are ordinary track collision geometry and nothing more
   exotic.
2. **Not one track exceeds 2048 in span.** The widest is `10_Track` at
   **2026.6057**, which fills 98.96% of the packing's 2048-unit range and clears
   it by 21.4 units.

   That second fact is **exhaustive, not a sample of named files**. The
   collision ground-truth test now walks every `.vex` entry of both discs by
   index - 1,038 files and 189 collision nodes on PS2, 340 and 130 on PSP,
   which is every collision node either disc ships - and reports the widest
   per-file collidable span it finds. It is 2026.6057 on both, because
   `10_Track` is on both. Nothing anywhere on either disc is wider.

A designer working to a 2048-unit budget is the only reading that puts a track at
99% of exactly that number by accident. So the conflict resolves in favour of the
packing being right and the *input* to it not being world space:

> **If the packing reading is right, its input cannot be raw world space.** The
> broadphase must subtract an origin of its own - the world's bounding-box
> minimum, or a per-level offset - before packing. Every shipped track fits the
> 2048-unit window; not one of them fits it *centred on the world origin*.

That was a **falsifiable prediction**, and reading the code has now falsified it.

> **There is no base.** `SapAxis_Update` (`0x08833cc8`) packs a bare world
> coordinate - `trunc.w.s`, `+ 0x400`, `* 2`, or in the object id - with nothing
> subtracted, and `Sap_Init` (`0x0882f8f4`) establishes no origin. The prediction
> above is wrong, and so is its alternative: **the packing reading is right too.**

What the survey did not anticipate is the third possibility. `Sap_Insert`,
`Sap_Update` and `Sap_QueryAabb` all **clamp** both AABB corners into a fixed
window before packing, with the same eight `vmax.t`/`vmin.t` instructions. A
coordinate beyond the window saturates onto the boundary instead of wrapping,
and because queries clamp exactly as inserts do, the broadphase stays
conservative - it over-reports and the narrowphase filters. So the original never
needed a track to fit the window: **it clips, and accepts the lost selectivity.**

That resolves the conflict without disturbing anything measured here. The
geometry is world space, seven tracks really do exceed ±1024, the packing really
does cover 2048 units, and the original simply does not care.

**Consequence for this project: the packing does not have to be reimplemented.**
It is an accelerator that never drops a genuinely overlapping pair, so exact AABB
overlap is a legal - and strictly better - substitute. The 1-unit quantisation
and the 1024-id cap are properties of the original's index, not of the format.

One loose end is recorded rather than resolved: the two clamp bounds are globals
at `0x08ab0c50`/`0x08ab0c60` that are **all zero in shipped `.data`, with no
writer anywhere in the image**. See
[collision.md](../ghidra/functions/psp-pulse-usa/collision.md#the-packing-subtracts-no-base-and-the-range-is-enforced-by-a-clamp)
for what was checked; confidence **45** on the constant, **90** on the clamp
being there. Nothing above depends on which value it holds.

Two smaller findings from the same survey:

- **The collision geometry is identical across platforms.** All twelve
  environments present on both discs report the same reach and the same span to
  the decimal. The PS2 exclusives are `08`, `11`, `12` and `15`.
- **A track's variants share one collision set.** `track.vex`,
  `track_reversed.vex`, `zone_track.vex` and `zone_track_reversed.vex` report
  identical geometry, so the 40 PSP and 55 PS2 named track files carry only 12
  and 16 distinct collision sets. The single exception is PS2 `11_Track`, whose
  zone variants reach 1132.0 against the plain pair's 1129.2 - worth knowing
  before assuming the variants are interchangeable.
- **There is a fifth variant name.** [track data](track.md) lists four;
  `Data\Environments\11_Track\track_zone.vex` exists on the PS2 disc as well,
  found by hashing candidate names against the archive's directory rather than by
  mining. That brings the named PS2 track files to 55 against 59 `Floor
  Collision` nodes, so **four collision-bearing PS2 files still have no recovered
  name**. They are covered by the exhaustive by-index sweep above, so they cannot
  change the conclusion, but the per-track table is a table of *named* files and
  does not claim to be all 59.

Reproduce with `oag-view --collision <track.vex>`, which prints both numbers and
flags each bound separately; see [oag-view](../tools/oag-view.md#collision).

### The header word is probably not a version

The first `u32` is `0xffff_ffff` on all 319 nodes, and the loader reads it without
acting on it. A version field with exactly one value across every node of two
platforms' builds is indistinguishable from an unused slot - and this format's
siblings store **pointer slots that are patched at load** in exactly this shape:
`WO Track`'s array pointers are zero on disc, and `.vex`'s embedded-texture
pointers likewise. All-ones is a plausible "invalid pointer" fill.

Confidence that it is a version: **40**. It is decoded and exposed as
`CollisionGeometry::version` because that is what the Ghidra page calls it, with
the value documented next to it.

### Still not determined

- **What the per-vertex scalar means**, now known to be unanswerable from the
  assets: see above.
- **Whether a fourth chunk type exists.** Only types 1, 2 and 3 appear in either
  build. The decoder refuses an unknown type rather than skipping it, even though
  the chunk's own stride field would let it step over one, because the contents
  would still be unknown and quietly dropping data is worse than stopping.
- **Where Pure keeps its surface types**, and which of its three class IDs is
  which.
- **Why the loader skips cage nodes**, and whether the PS2 loader does too.
- **Whether the 1024-id sweep-and-prune limit is enforced** for objects with more
  than 1024 triangles. The largest shipped object was not measured against it.
- **Nothing on this page has been verified under an emulator.** Everything is
  static reading plus exact agreement with shipped data, which is what caps the
  scores at 94.
