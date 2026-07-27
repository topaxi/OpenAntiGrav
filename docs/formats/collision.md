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
[the Ghidra page](../ghidra/functions/psp-pulse/collision.md); this page is the
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

## Surface types and the restitution sentinel

All five classes are registered with one vtable and share one parser. The class
ID selects a surface type and a restitution constant, nothing else.

Restitution is `0.05` for `Wall` and **`-1.0` for `Floor`, `Mag Floor` and
`Reset`**, and the negative value is a **sentinel, not a coefficient**: contact
combination averages the two surfaces but forces zero if either side is negative.
So a floor contact never bounces, and a floor-against-wall contact is `0.0`
rather than the average `-0.475`. Implementing the `-1.0` literally would add
energy on every contact instead of removing it - a negative restitution is not a
soft surface, it is an accelerating one.

`oag_formats::collision` therefore models it as `Option<f32>`, where `None` means
"never bounce", and `combine_restitution` is the rule rather than a comment.
Confidence **88**, unchanged from the Ghidra reading: this is decompilation
evidence, and no shipped byte bears on it.

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

### The world-space extent contradicts the documented broadphase packing

Collision vertices are world space: every node sits at depth 1 with an identity
world transform, on both discs. The largest coordinate is **1,335.8 on PSP and
1,554.1 on PS2**.

[The sweep-and-prune reading](../ghidra/functions/psp-pulse/collision.md#two-level-sweep-and-prune)
packs endpoints as `bits[0:11] = ((int)coord + 0x400) * 2`, which covers roughly
**-1024 to +1023**. Both cannot be right as stated. Either the packing reading is
wrong, or the broadphase clamps out-of-range coordinates, or it operates in a
space that is not the one these vertices are in - per-object local coordinates,
for instance, which would make the world broadphase's 1024-slot list a different
thing from the per-mesh one. Nothing here resolves it; the conflict is precise
enough to be settled by re-reading `Sap_Init` (`0x0882f8f4`) against a real
track's extent.

`oag-view --collision <track.vex>` prints the reach from the origin per track and
flags anything past ±1024, so the per-track distribution is one command away -
see [oag-view](../tools/oag-view.md#collision). The numbers above are whole-disc
maxima; whether one track is the outlier or every track exceeds it is not
recorded here.

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
