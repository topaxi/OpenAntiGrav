# Wipeout 2048's `.rcsmodel`

Wipeout HD's `.rcsmodel` is an offset-table archive read big-endian -
[rcsmodel.md](rcsmodel.md) is that format. **Wipeout 2048's is not that file.**
It shares the extension and nothing else: a linker-style image, read
little-endian, with a header section carrying relocation tables and two
back-to-back payload blocks that are rebased at load.

Read by `oag_rcs::rcsmodel::psp2`, drawn by `oag_mesh::mesh::rcs::psp2`,
validated by `crates/rcs/tests/psp2_rcsmodel_ground_truth.rs` over **all 993
files** the three EU packages ship. The loader is `RcsModel_Load`
(`0x812f15b2`), tagged `PSP2/Psp2.RcsModelLoader.cpp` - see
[track-and-collision-loaders.md](../ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md).

## The container

```c
struct Image {                 // section A, the header
    u32 magic;                 // 0xca5caded
    u32 unread;
    u32 section_count;         // 2 with geometry, 1 without
    u32 header_size;           // section A's own length
    ...
    Descriptor sections[section_count];   // from +0x20
    Relocation tables[section_count];     // in descriptor order
};                             // then section B, then section C

struct Descriptor {            // 0x20 bytes
    u32 tag;                   // unread
    u32 size;
    u32 table;                 // its relocation table, from the end of the descriptors
    u32 entries;
    u32 link_base;             // 0 on every shipped file
    ...
};

struct Relocation {            // 8 bytes
    u32 offset;                // of a pointer, within the section named below
    u32 section;               // which section that pointer lives in
};
```

**Confidence 94.** All 993 files have `A + B + C` equal to their own length, and
the header, the descriptors and both tables tile section A with 40 bytes of tail
padding on `altima`. `RcsModel_Load` walks table 0 rebasing every pointer *to* B
and table 1 rebasing every pointer *to* C.

**The link base is zero on every shipped file**, which is the fact that makes
this readable: a pointer on disc is already the offset of its target within its
section, so nothing has to simulate the rebase.

Section **B** is a serialized C++ object graph and section **C** is raw GPU
buffer data. On `altima` all 23,711 pointer locations live in B; C holds no
pointers at all.

## The geometry

```c
struct SubMesh {               // located, not walked to - see below
    u32 index_count;           // +0x00, divisible by three
    u32 vertex_count;          // +0x04
    ...
    u32 index_buffer;          // +0x10, an offset into section C
    ...
    u32 vertex_buffer;         // +0x2c
};
```

Indices are little-endian `u16` triangle lists - section C opens
`0,1,2, 2,1,3, 4,5,6, 6,5,7`, quads split into triangle pairs. A vertex begins
with its position as three little-endian `f32`.

### The record is *located* rather than walked to, deliberately

Section B's object layout is unread. Rather than guess it, this reading takes
the one thing the header states exactly - **where every pointer into the GPU
section lives** - and pairs them: a submesh holds its index-buffer pointer and,
28 bytes later, its vertex-buffer pointer.

What makes that a reading rather than a pattern match is that it is **checked**,
and the check is arithmetic that cannot come out even by accident:

| | corpus |
| --- | --- |
| files whose sections add up to the file's own length | **993 of 993** |
| submeshes whose index buffer is `count * 2` rounded up to 4 | **244,889 of 244,889** |
| …whose index count divides by three | **244,889 of 244,889** |
| …whose vertex buffer length divides by its vertex count | **244,889 of 244,889** |
| indices naming a vertex outside their own submesh | **0 of 33,335,682** |

### The vertex stride is in no field, and comes from the buffer packing

Section C is exactly the buffers, back to back in pointer order. Sorting the
distinct pointer targets and differencing them gives every buffer's length, and
a vertex buffer's length divided by its own vertex count is the stride. That is
the same oracle [rcsmodel.md](rcsmodel.md) records as one of four for Wipeout
HD; here it is the only one needed, and it is exact rather than a search.

The strides that come out are **16 to 64 in steps of four**, 24 and 20 the
commonest.

### A second record shape closes every previously-unpaired pointer

**2026-09-02, confidence 92.** The 28-byte gap above is not the *only* shape a
submesh record takes - `oag_rcs::rcsmodel::psp2::SKY_BUFFER_POINTER_GAP`
is a second, **184 bytes**, found on `SkyCube/skycube.rcsmodel` (see
[2048-sky.md](2048-sky.md)) and tried alongside the first rather than instead
of it. Both have to pass the same arithmetic `one` already checks - a
divisible-by-three index count, a buffer whose byte length matches, a stride
the vertex count divides evenly - so a coincidental 184-byte gap elsewhere in
the corpus is rejected on the same terms a coincidental 28-byte one always
was; nothing about the check itself changed, only how many gaps this reading
tries per candidate pair did.

**The effect on the corpus is total, not incremental.** Before this reading,
`altima` alone left 5,294 GPU-pointer entries unpaired - "about one in a
hundred corpus-wide", the ["What is not decoded"](#what-is-not-decoded)
section below used to say. Adding the second gap and re-running the same
993-file sweep: submesh count rises from 244,889 to **247,536**, and
`psp2::Model::unpaired_pointers` sums to **zero across every shipped file**.
Every one of the corpus's previously-unaccounted GPU pointers turns out to be
this same second record shape - not a sky-only phenomenon (skies account for
only a few dozen of the 2,647 newly-found submeshes; the rest are ordinary
props and craft parts that happened to use the larger record), which is why
the constant is not named after skies despite where it was first found. The
existing per-submesh checks (arithmetic closure, unit-length normals, in-range
positions) all still pass at their usual bar over the enlarged corpus - see
`crates/rcs/tests/psp2_rcsmodel_ground_truth.rs`.

**What the extra bytes hold is still unread.** This finding says where the
vertex-buffer pointer sits in the second shape, not what fills the 156 bytes
between it and the first shape's own tail - a bounding box, like HD's `Mesh`
node carries, is the obvious guess and is exactly that, a guess; nothing here
confirms it.

### Positions are the model's own space - or a node's, for a mesh bound to one

A circuit's positions come out in world coordinates and a craft's about its own
origin. Measured rather than assumed: `altima`'s track model spans
x -1886..2575, y -459..839, z -1486..2435, bracketing the box its own
`track_col.col` states, while `Assegai`'s hull sits inside 2.8 x 1.7 x 7.0 units.

**Corrected 2026-09-16: that is true of a mesh with no node, and wrong for one
with.** The file's own header names a node for 130 of `altima`'s 1,153 mesh
objects (and for all 18 of a craft's), and those meshes' positions are in
*that node's* space: a median 1,086 units, and up to 5,765, from where the
original draws them when taken as world coordinates. The node table, the
`.rcsskeleton` that hierarchises it and the `.rcsanimclip` that moves it are
[2048-animation.md](2048-animation.md); `mesh::rcs::psp2::build` now places
a node-bound mesh through its node and composes nothing onto the rest. The
craft's airbrakes were the visible case - node-bound, with the node at the
tail and the flap about its own hinge, so both drew under the cockpit.

**456 of 33,335,682 positions** across the corpus fall outside a generous world
box. That is 0.0014 %, it is recorded rather than explained, and a reading wrong
about the offset would not miss by that little.

## Section B carries a vertex declaration, structurally identical to HD's

**2026-08-27.** Section B - the object graph this reading otherwise never
walks - carries a per-chunk vertex declaration in the same shape
[rcsmodel.md](rcsmodel.md#the-chunk-declares-its-vertex-layout-at-the-word-0x58-points-at)
documents for Wipeout HD: a 4-byte
header (`count`, `stride`, two reserved bytes) then `count` 8-byte attribute
records (`name_hash`, a repeated `stride`, a type byte, an offset byte). Found
by scanning section B for HD's own `~crc32` attribute-name hashes
(`position` `0xb9d31b0a`, `normal` `0xde7a971b`, `tangent` `0xdbe5f417`,
`lightmapUV` `0x26a7b665`) and decoding outward from each hit - the reproducer
is `crates/game/examples/vita_rcsmodel_rosetta.rs`.

**The container mixes byte order within one 8-byte record.** `name_hash` is
little-endian, matching the rest of this format - but the record's own
repeated `stride` field only cross-checks against the header's stride read
**big-endian**; read little-endian it disagrees on every record. Consistent
with a serializer that writes scalar fields through explicit byte shifts
(host-endian-independent) while `name_hash` is a straight native-endian word
copy that the PS3-to-Vita port never adjusted.

**Confidence 85 on the structure and the offsets below**, from the same kind
of closure this whole format's reading rests on: every decoded declaration's
attribute records cross-check their restated stride against the header's, and
the last attribute's offset plus its byte span reaches the header's own
stride exactly. Measured on the twelve HD-ported circuits' declarations (7 to
30 per environment). The common layout, stride 28:

| Attribute | Offset | Type nibble (components, code) |
| --- | --- | --- |
| `position` | `0x00` | `c3 t9` |
| `normal` | `0x0c` | `c3 t5` |
| `tangent` | `0x10` | `c4 t5` |
| `Uv1` | `0x14` | `c2 t8` |
| `lightmapUV` | `0x18` | `c2 t8` |

Smaller strides drop attributes from the tail (stride 20 keeps
`position`/`normal`/`VertexColour1`; stride 16 keeps only
`position`/`normal`) rather than reordering the ones they keep.
`find_declarations` anchors on the `position` hash and assumes it is always
record 0 (`header_at = position_at - 4`) - true on every declaration checked
by hand, not independently verified against every chunk in the corpus, so a
declaration where `position` is not first would be silently missed rather
than misdecoded.

**What the offsets do not give: the type nibble's meaning.** 2048 runs on the
Vita's SceGxm, not the PS3's RSX, so the low nibble is a different hardware
enum and HD's `RSX_*` codes are not a safe translation - confirmed rather than
assumed: `normal`'s declared type (`t5`) tested as HD's packed 11:11:10 word
at this now-*correct* offset still scores at chance (1.1% within 18°,
mean|dot| 0.311 against a 0.5 random baseline, n=532,140 - the same
position-matched HD-pairing oracle `rcsmodel.md`'s own normal recovery used).
The byte budget is a second, independent constraint: `normal@0x0c` to
`tangent@0x10` is exactly 4 bytes for 3 declared components, which alone rules
out at least one otherwise-plausible per-component width. `position`'s type
(`t9`) is the one nibble this reading can name with confidence, because
`position` is independently confirmed as 3 little-endian `f32` (measured, not
recalled) - so `t9` is *some* 4-byte-per-component float code, whatever SceGxm
calls it. (`t5`'s meaning did not stay open long - the byte-budget argument
above turns out to be the whole answer; see "The normal is cracked" below.)

`lightmapUV`'s plausibility (decodes to a finite `f16` pair within a
texcoord-shaped range) is 80.7% at the declared offset against 76.2% for the
old last-four-bytes guess - **not meaningfully different, and not evidence
either way**: on the dominant 28-byte layout `lightmapUV`'s declared offset
(24) *is* `stride - 4`, so the two tests read the same bytes on most of the
corpus. A lightmap atlas is also baked per platform, so even a correct decode
has no reason to match HD's own coordinates numerically at the same vertex -
unlike position and normal, which are geometry and port unchanged.

### A cleaner oracle exists, and it still doesn't decode the type nibble

**2026-08-27, same session.** The spatial nearest-neighbour match above drops
1,894,162 of ~2.4M candidate vertices as ambiguous - several vertices sit at
the same position with different normals at a hard edge, and a search cannot
tell which is which. An **index-exact** correspondence sidesteps that
entirely, the same way HD's `WO Track` tail was settled: if an HD submesh and
a 2048 submesh share the same vertex count *and* comparing position `v` to
position `v` directly by index agrees closely with no search at all, vertex
`v` is the same authored vertex on both platforms, unambiguously.

That correspondence is real, and rare: only 9 of 7,749 same-vertex-count
candidates (25,712 2048 submeshes total, across the twelve ported circuits)
pass a 5 cm index-agreement bar - most of a track was genuinely re-tessellated
for the Vita, and only a few submeshes were re-exported byte-for-byte in the
same order. Those nine give 1,504 vertices with zero correspondence ambiguity
- the reproducer is `crates/game/examples/vita_rcsmodel_exact.rs`.

Against that clean sample, every encoding hypothesis tried still fails to
generalise:

| Hypothesis | n | within 18° | mean dot |
| --- | --- | --- | --- |
| HD's packed word, read little-endian | 1,504 | 0.8% | -0.042 |
| HD's packed word, read big-endian | 1,504 | 5.6% | -0.019 |
| fields reordered, z derived from unit length (oracle sign) | 1,504 | 20.5% | 0.382 |

The big-endian candidate was worth trying precisely because the declaration's
own repeated-stride field (above) is big-endian in an otherwise little-endian
container - not a stretch, and still wrong. **The third row is the trap worth
naming**: two near-planar submeshes appeared, by eye, to confirm it almost
exactly - the low 11-bit field matched no HD component, but the other two
fields (reordered) landed within a few hundredths of HD's x/y, and the missing
magnitude matched a unit-sphere completion for z to three decimal places on
one of the two. Tested at scale it reaches 20.5%, nowhere near a real decode.
**Both hand-picked examples happen to be simple, close-to-axis-aligned
directions**, which a wrong candidate scheme can fit by chance far more easily
than a general one - the same shape of trap `HANDOVER.md`'s "Traps that are
live" section already records for this format, and the reason nothing here is
implemented on fewer than a full-corpus check.

What survives this pass: the index-exact correspondence itself, as a reusable,
ambiguity-free oracle for the next hypothesis - cheaper to test against than
rebuilding spatial matching, and immune to the hard-edge noise that likely
explains why the spatial-matched sweep never got a clean read either.

### The normal is cracked: three signed bytes, not a packed word

**2026-08-27, same session, next hypothesis.** Every candidate tried above
shares one assumption - that `normal`'s 4-byte slot is one packed word, HD's
shape. The declaration itself argues against that: `tangent` declares the
*same* type code (`5`) with 4 components rather than 3, and `tangent`'s own
byte budget (`+0x10` to `Uv1@0x14`) is exactly 4 bytes for those 4 components
- one byte each, no padding needed. If type `5` is a fixed **one byte per
component**, `normal`'s 3 components take 3 bytes and the 4th byte to
`tangent@0x10` is alignment padding, not a fourth used field - a completely
different shape of encoding than the packed word tried three times over
above.

An exhaustive search over that shape - every assignment of 3 of the 4 raw
bytes to x/y/z, both a signed and an HD-style unsigned-biased reading, plus
every field-to-axis permutation of the packed-word family for good measure
(60 candidates total) - against the same 1,504-vertex index-exact oracle,
settles it outright:

| Hypothesis | n | within 18° | mean dot |
| --- | --- | --- | --- |
| **`byte[0]=x byte[1]=y byte[2]=z, each `i8/127`, byte[3] padding** | 1,504 | **100.0%** | **0.994** |
| next best (bytes reordered) | 1,504 | 54.0% | 0.802 |
| every packed-word permutation tried | 1,504 | ≤27.6% | ≤0.489 |

**Confidence 96.** The winner is not merely best of a field - it is a clean
break, 0.994 against a 0.802 runner-up, and it generalises: the same decode
scores **33,335,682 of 33,335,682 normals unit-length within 2%** across
*all 993 shipped files*, not just the twelve HD-ported ones the oracle could
check content against. The unused 4th byte is `0x00` on every one of the
1,504 oracle vertices - genuine alignment padding, not a fourth field this
reading is leaving on the table. The reproducer is
`crates/game/examples/vita_rcsmodel_bytesearch.rs`; the decode lives in
`oag_rcs::rcsmodel::psp2::unpack_normal`.

**What this does not confirm**: the SceGxm symbolic name for type code `5`.
The decode behaviour (signed, one byte per component, normalised by 127)
matches what a format called `S8N` would do on Vita's GPU, but that is
recognition of a shape, not a Ghidra-confirmed symbol - `RcsModel_Load`
itself never touches section B past relocating it, and the actual consumer
(wherever the eboot binds a `SceGxmVertexAttribute` from this declaration)
was not located this session. The empirical decode is what the renderer
uses; the SDK-level name is left for whoever finds that code.

## `Uv1`'s type is cracked too: two little-endian `f16`s

**2026-08-27, later the same day the normal was cracked.** The byte budget
already argued for this - `Uv1@stride-8` to `lightmapUV@stride-4` is 4 bytes
on the common 28-byte layout, and this reading already assumed `lightmapUV`
was `f16` pairs. What settles it as fact rather than a plausible guess is
content, the same index-exact oracle that scored the normal: 1,432 vertices
across 9 same-export submeshes (twelve HD-ported circuits), decoded as `f16`
against Wipeout HD's own diffuse UV at the same authored vertices. Mean
squared distance is indistinguishable from zero; decoded as `unorm16` (the
only other 2-byte-per-component encoding the byte budget allows) it is 0.41,
chance level. The two decodings even agree on which 16 of 1,432 vertices
leave `[0, 1]` - `unorm16` cannot represent that at all, since a road texture
tiles.

**Found through a stride map, not a per-submesh pointer.** A submesh record
does carry a word at `+0x28` that is a valid declaration offset on some
files - the craft this reading was built against among them - but it
resolves for only 14.2% of the corpus's submeshes, where anchoring on
`position` per file and keying by stride (`psp2::vertex_decl::find_by_stride`)
resolves 92.5%. The reproducer is `crates/game/examples/vita_rcsmodel_uv_oracle.rs`;
the decode is `oag_rcs::rcsmodel::psp2::unpack_texcoord`.

**Confidence 96** on the decode, on the same basis as the normal. What is
not decoded: `lightmapUV`'s content (offset placed via the same declaration,
but its value is a per-platform baked atlas coordinate with no HD
counterpart to check against). `tangent`'s type (`t5`, 4 components - the
byte budget argument for `normal` does not apply since 4 components exactly
fill 4 bytes) is cracked too, at a lower confidence than this - see the next
section.

## `tangent` is cracked too, at a lower confidence

**2026-09-16, confidence 76.** `tangent` (`t5`, 4 components, `+0x10` on the
common 28-byte stride) decodes the same way `normal` does - one signed byte
per component, `byte/127.0` - extended to all four bytes rather than
three-plus-padding, since 4 components exactly fill 4 declared bytes and
leave nothing spare the way `normal`'s 4th byte was. This is weaker evidence
than `normal`'s 96, for a reason specific to this field: **there is no
Wipeout HD twin to check content against.** HD's own renderer has no decoded
tangent frame either - `oag_rcs::rcsmodel`/`oag_mesh::mesh::rcs` (the HD
module) name a tangent frame among the inputs HD's renderer still lacks, not
among what it has decoded - so the index-exact oracle that settled `normal`
and `Uv1` has nothing to compare against here. What follows instead is
internal consistency, scored against deliberately-wrong controls the same
way every other reading on this page is.

**The method**: every byte assignment of the four raw bytes to `(x, y, z, w)`
- which byte is `w` (4 choices) x every ordering of the other three as
`(x, y, z)` (6) x two signedness conventions (`i8/127`, HD's unsigned-biased
`u8/127.5-1`) = 48 candidates - scored over 6,653,653 vertices across the
base package and both DLC packs whose declared stride names a `tangent`
attribute, by unit length of the `(x, y, z)` part and by `|dot|` against the
already-cracked `normal` at the same vertex (a tangent is authored
orthogonal to its normal, so `|dot|` near 0 is the signal and near 0.5 is
chance). Reproducer:
`crates/game/examples/vita_rcsmodel_tangent_bytesearch.rs`.

| Candidate | unit-length pass (`|len-1|<10%`) | mean `|dot(normal)|` | orthogonal-ish (`|dot|<0.342`) |
| --- | ---: | ---: | ---: |
| **`x=byte[0] y=byte[1] z=byte[2]` (`i8/127`), `w=byte[3]`** | **58.1%** | **0.219** | **71.1%** |
| next best of the same byte set, reordered | 58.1% | 0.391-0.492 | 36.9-49.2% |
| best of a different byte set (`u8/127.5-1`, `w=byte[0]`) | 29.2% | 0.480 | 32.8% |
| CONTROL (`byte[0..3]/255`, no sign, not a plausible encoding) | 23.2% | 0.407 | 47.4% |

**The winning row is not merely best of 48 - it is the unique best of the six
permutations that share its own byte set**, which is what makes this a
reading rather than a coincidence: those six all have identical unit-length
statistics (permuting which byte is `x`/`y`/`z` cannot change the vector's
length), so the only thing separating 0.219 from 0.391-0.492 is that this
one ordering is the *authored* one. A coincidental byte-set match would not
show that spread.

**Restricted to the 58.1% that pass the unit-length band, the signal
sharpens substantially**: mean `|dot|` falls to **0.102** and 86.5% of
tangents land within 20 degrees of perpendicular to the normal - against
23.2%/0.407/47.4% for the control on the same terms. A length histogram
shows this is a real population, not an average of unrelated values: a
sharp spike at `len` in `[0.9, 1.0)` (3,524,011 of 6,653,653 vertices)
alongside a long tail including a genuine cluster near `len` = 0
(669,437 vertices, `[0, 0.1)`).

**The remaining ~42% is read as degenerate tangent data, not a second
encoding** - checked rather than assumed: restricting to vertices whose
`Uv1` at the same declaration also decodes finite (ruling out the
declaration/stride-mismatch trap `SubMesh::non_finite_texcoords` documents,
which affects ~21% of texcoords corpus-wide) barely moves either number
(58.9%/0.206/72.9% clean against 43.8%/0.472/37.0% dirty - the *dirty*
subset is worse, as expected, but 95% of all vertices are clean and the
clean subset alone still shows the same weaker-than-`normal` shape). A
tangent basis is commonly undefined or authored as a near-zero vector at UV
poles and seams on real assets, which is consistent with the near-zero
cluster in the length histogram.

**`w`, the fourth byte, reads as a handedness sign on the well-formed
subset**: across the full corpus it is exactly `+-127`/`-128` on only 48.0%
of vertices (top values `-127`, `127`, then a long tail), but restricted to
the same unit-length-passing 58.1% it is exactly `+-127`/`-128` on **82.6%**
- consistent with a genuine `+-1` handedness sign that only means something
once the `(x, y, z)` part is itself well-formed.

**Not runtime-verified.** `RcsModel_Load` (`0x812f15b2`) never touches
section B past relocating it, same as for `normal` and `Uv1`, and the
function that actually binds a declaration's fields into a
`SceGxmVertexAttribute` at runtime was searched for and not located this
session: `search_strings` for a vertex-declaration/attribute source tag
found `System/Render/Model.cpp`, the plausible runtime mesh class, but its
one located caller (`FUN_81287fd4`, not named - confidence too low) is a
texture mip-generation routine, not vertex setup; walking back from
`sceGxmDraw` (NID `0xBC059AFC`) is the same documented dead end the material
binding's own confidence-90 section above already hit. No function was
named, so `names.tsv` gains no row.

Implemented as `oag_rcs::rcsmodel::psp2::unpack_tangent`, wired into
`psp2::SubMesh::tangents` and counted (not yet drawn with) in
`oag_mesh::mesh::rcs::psp2::Report::decoded_tangents` -
`just play 2048 --race` now reports e.g. "260533 tangent(s) decoded, unused
(no normal-map consumer yet)" for Altima's own circuit mesh. **Confirmed to
change nothing visually**: the pre-change render (not kept) and the post-change render are byte-identical (`cmp` exit 0) - nothing in
this title's mesh path samples a tangent-space normal map yet, so adding the
data changes only the load report, not a pixel. Neither the shared
`GpuVertex` vertex layout nor `mesh.wgsl` (used by every title's mesh path)
was touched, deliberately: there is no consumer to justify the risk of
changing a struct every title's rendering depends on for data nothing reads.

## The material table is read, and finding a submesh's own binding was tried and ruled out

**2026-08-27, same day.** `.gxt`/`.rcsmaterial` ASCII text sits in the clear
inside section B - 52,637 and 34,388 occurrences across the 993-file corpus -
found the same way the buffer pointers were: pair a pointer to the thing it
points at. A file-level header (`+0x44` count, `+0x48` an offset table)
names every material's 64-byte header, which carries the `.rcsmaterial` path
at `+0x04` and a technique/shading-group name at `+0x18`. Every texture a
material references is found by scanning that material's own byte extent for
any pointer resolving to `.gxt` text, rather than by decoding the shader
input table's own struct - that struct's shape changes with entry count in a
way this reading did not solve (clean at two entries, garbage read past it
at four, six or eight). See `oag_rcs::rcsmodel::psp2::material`'s module
doc for the full account, including a trap worth naming: the first version of
this reading capped the file-level count at 64, calibrated from small props
and a 16-submesh craft, which silently rejected altima's own 527 real
materials as implausible. Fixed once the miscount was traced back to the
cap rather than the read.

## Which submesh uses which material: a plain index, `0x18` bytes before the record

**2026-08-27, later the same day. Confidence 90.** An earlier pass this same
day recorded this as genuinely unresolved, and that pass's reasoning was sound
while its conclusion was wrong - which is the part worth carrying forward.

It had checked every submesh record field against every *material's own
identity*: the two unidentified header words at `+0x00` and `+0x08`, the name
pointer, and the still-uninterpreted 64-bit word 8 bytes past a submesh's
vertex pointer. Nothing matched anywhere in the corpus, and nothing ever could
have: **an index does not resemble the thing it indexes.** A submesh carrying
`3` matches nothing about material 3, so a search for a material's identity was
structurally blind to the answer.

Searching for an *index* instead - a small integer in `[0, count)` - finds
exactly one field, `0x18` bytes **before** the record's own start
(`psp2::MATERIAL_INDEX_BEFORE_RECORD`), as a little-endian `u32` whose high
half is `0` on every submesh measured. The offset is negative because the
record is *located* rather than walked to (see the section above): what this
reading calls the record's start is where its two buffer pointers put it, not
the start of whatever enclosing struct section B serializes.

### The control group is the evidence, because small integers are everywhere

Swept over every byte-, half- and word-aligned offset from `-0x60` to `+0x80`
across all three EU packages:

| Condition | Result |
| --- | --- |
| Value inside `[0, material count)` | **244,889 of 244,889 submeshes**, no violation |
| Value `0` on every submesh of a one-material model | **147 of 147 models**, no violation |
| Distinct values used per multi-material model | **80.3 on average** |

The first two conditions are passed by **nineteen** offsets in that window -
lots of fields are accidentally small. The third is what separates them:
eighteen of the nineteen average **3.2 or fewer** distinct values per
multi-material model, where this one averages 80.3. **86 of the 414
multi-material models name every single material they declare** through it,
and 138 name at least 90%. Altima's own circuit uses **525 of its 527**
materials across 2,800 submeshes; a field that merely happened to be small
cannot do that.

### And it reads correctly where a human can check it

`feisar2048\3`'s sixteen submeshes resolve to five of its six materials:
`2048_ship_tech` on seven, `2048_ship_paint_shiny_final` (the team livery) on
four, `2048_ship_plastic` on three, `2048_ship_lights` on one, and
`2048_ship_glass_dg` on **exactly one**. **Re-measured 2026-09-02**, after
`SKY_BUFFER_POINTER_GAP` found six more submeshes on this same craft (22 in
total): `2048_ship_tech` on eight, `2048_ship_paint_shiny_final` on four still,
`2048_ship_plastic` on five, `2048_ship_lights` on three,
`2048_ship_glass_dg` still on **exactly one**, and `2048_engine_additive` -
absent from the count before, not merely under-counted - on one. All six of
the craft's materials are used now, not five.

That last one is the canary. The submesh a name match would have guessed at -
the one authored `GlassShape` - is the one this index independently lands on,
**without any name being compared**. Name-matching a submesh to a material is
the invented-not-measured failure `CLAUDE.md` forbids, and is exactly what the
earlier pass rightly refused to do; that it agrees here is corroboration, not
the method.

### What holds this at 90 rather than higher

**It is measured, not read out of the executable.** `RcsModel_Load`
(`0x812f15b2`) never touches section B past relocating it, and the function
that binds a submesh to a GPU texture unit was **not located**: all eleven
callers of `sceGxmDraw` (NID `0xBC059AFC`) in `vita-2048-eu-v104` turn out to
be debug primitives, sprite quads and the engine-flare effect, so this engine's
mesh path reaches the GPU some other way - a deferred command buffer, by
hypothesis, not read. No function was named, so
[names.tsv](../ghidra/functions/vita-2048-eu-v104/names.tsv) gains no row.
Confirming the field at the instruction level is the one thing that would take
it past 90, and it stays open.

The three words beside it are still unread: `-0x20` and `-0x1c` are `0`,
`-0x10` is `0xffffffff` and `-0x08` is `0x00010001` on every file sampled.

Reproducers: `crates/game/examples/vita_rcsmodel_material_index_probe.rs` (the
sweep and its control group) and `vita_rcsmodel_material_index_check.rs` (the
craft and the circuit read out in full). Pinned by
`crates/rcs/tests/psp2_rcsmodel_material_ground_truth.rs`.

## A texture bound also would not have painted, until the same day

**2026-08-27.** Even granting a resolved binding, a sweep of every distinct
diffuse `.gxt` the corpus's materials name (6,135 of them) found **15 decode**.
The other 6,120 failed with `gxt::Error::Unsupported` - 6,037 of them format
`0x83` (`PVRTII4BPP`). So "load textures" had two independent gaps stacked, and
neither was a follow-on to the other: the binding above, and the pixel format
almost everything is compressed in.

**Both are closed.** `oag_texture::pvrtc` decodes `PVRTII4BPP` at confidence
92, validated against Wipeout HD's own `.gtf` copies of 2,284 shared textures -
see [gxt.md](gxt.md).

## The PS4 Omega Collection reads through this module

Added 2026-09-29 (`omega-race`). Omega's `.rcsmodel` is this container with
**64-bit pointers** and `0x100` in header word `+0x04` (0 on all 953 of 2048's
base package, `0x100` on all 1,272 PS4 entries). Everything above holds with
three changes, each measured on Omega's corrected extraction: the submesh
record's vertex pointer sits 32 bytes past its index pointer
(`PS4_BUFFER_POINTER_GAP`, confidence 90; on `ag_systems\ship.rcsmodel` 9 of the
12 records), its material index sits 0x28 bytes before the record (not 0x18),
and the material table is found by the 64-bit pointers to its `.rcsmaterial`
paths (confidence 75) with each texture ranked by the sampler-name hash 0x18
bytes before its `.gnf` pointer (confidence 85). Positions, triangles and the
three-signed-byte normal decode at the same offsets: unit length, mean 0.994.
Omega's Feisar and Qirex hulls agree with 2048's ports on submeshes,
triangles and materials. The node table is read on PS4 with 8-byte pointers
([`2048-animation.md`](2048-animation.md#the-models-node-table)). Evidence and
counts: [omega-status.md](omega-status.md#racing-a-race-starts-on-this-titles-own-data).

## What is not decoded

Named here rather than left to be rediscovered:

- **Section B's object graph**, past the declaration above, the second
  record shape that [closes every GPU pointer the corpus
  names](#a-second-record-shape-closes-every-previously-unpaired-pointer),
  and - since 2026-09-16 - the **node table and mesh objects** that walk
  from the file header down to every one of those records
  ([2048-animation.md](2048-animation.md#the-models-node-table)):
  `psp2::Model::unpaired_pointers` is a real field for a file this reading has
  not yet met, not evidence of a gap left in the 993 it has.
- **`RcsModel_Load`'s actual consumer** - whichever function binds a
  declaration's fields into a GPU vertex-attribute setup at runtime. Not
  located this session or the one that cracked `tangent` (see above); all
  three of `normal`, `Uv1` and `tangent` were settled empirically instead,
  which is why none of `t5`, `t8`'s symbolic SceGxm names are confirmed, only
  recognised by behaviour - and why `tangent`'s own reading stops at
  confidence 76 rather than reaching `normal`'s 96: there is no HD twin to
  check its content against, only internal consistency.
- **The high word of the 64-bit hash** each mesh object carries - the low
  word is `~crc32` of the shape's own full Maya path, on all 1,153 of
  `altima`'s ([2048-animation.md](2048-animation.md#the-models-node-table)) -
  the two unidentified 32-bit words each material header carries, and the
  three words beside the material index at `-0x20`, `-0x10` and `-0x08`. The
  binding itself is read - see the material index section above - but at
  confidence 90, off measurement rather than off the executable.
- **The section tags** at each descriptor's `+0x00` (`0xe35e00df` and
  `0xe9f17935` on `altima`).

A model with **no GPU section at all** is an ordinary state, not a failure: 43
of the 993 declare one section, and `RcsModel_Load` skips the whole second
allocation when its size is zero.

## What it looks like

`just play 2048 --race` draws a **textured** Altima and a textured craft.

- The circuit: 413,358 triangles over 2,800 submeshes, **2,782 of them
  textured**, from 521 of its 527 materials. 18 draws name a `.gxt` that does
  not resolve in the archive or will not decode, and get no texture rather than
  a neighbour's.
- The craft: 7,416 triangles over 16 submeshes, **16 of 16 textured**, from 5
  of its 6 materials - the Feisar livery, its tech panels, its plastics, its
  light strips and its canopy glass, each on the submeshes the file's own
  index names.
- Both lit off the file's own authored normals where a submesh's stride carries
  one - 33,335,682 of 33,335,682 across the corpus - and off computed face
  normals only where it does not, which is not observed on any shipped file.

The screenshot is not kept in the repository: the start straight
with legible grandstand advertising, the overhead gantry banner, panelled road
surface and the craft's own wordmark on its tail.

Still absent from that picture, and unrelated to anything above: the
`.envsettings` sun/fog/bloom blocks do not parse for this title, so the race
draws with a stand-in lighting rig, unfogged and without bloom. (`track.pvs`
was listed here as "not this project's HD PVS layout, so every chunk draws":
it is that layout's little-endian dialect and it is read and culls since
2026-09-30 - [hd-pvs.md](hd-pvs.md), "The 2048 lineage's dialect". Four of the
Vita's DLC1 `track_reversed.pvs` files do not belong to their model and are
refused; the rest agree.)
