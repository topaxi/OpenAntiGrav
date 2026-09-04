# The `Dynamic Shadow Occluder` runtime reader

Step 4 of [`docs/rendering/shadows.md`](../../../rendering/shadows.md)'s plan
decoded the `0x3c3` payload's two record arrays
(`crates/formats/tests/shadow_occluder_ground_truth.rs`) but left one thing
the data alone cannot answer: how the hull is *projected* into a shadow. This
page reads the function that does it.

**Static reading of `psp-pulse-usa`'s `BOOT.BIN`. Not runtime-verified.**

## The cast

| Address | Name | Confidence |
| --- | --- | --- |
| `0x089038c8` | `Shadow_RenderOccluderVolume` | 84 |
| `0x0890446c` | `DynamicShadowOccluder_RegisterClass` | 84 |
| `0x08923518` | `Shadow_RegisterClass` | 84 |
| `0x08b62540` | `g_shadow_direction` | 88 |
| `0x08b62530` | `g_shadow_direction_override` | 80 |
| `0x08abffd0` | `g_shadow_node_count` | 82 |

Both sit at the ceiling of the rubric's **70-84 Probable** band, under
"decompilation only, consistent call sites" - not the 85-94 band. The class
id (`0x3c3`) is an unambiguous plain immediate and the byte-exact pointer
match at the method table is real, but this is one binary's own internal
consistency, not corroboration across multiple shipped files or a runtime
trace, so it does not clear the higher band on its own.

## Read this before trusting an address

Same trap as [`autopilot.md`](autopilot.md) and
[`positional-audio.md`](positional-audio.md): this import carries
**unrelocated address constants**. A pointer stored in `.data` prints
`base_address - 0x08804000` short, so `get_xrefs_to` and
`get_function_callers` are silent for this function - confirmed directly,
both return nothing for `0x089038c8`. The one static reference that exists
was found by computing the unrelocated form (`0x089038c8 - 0x08804000 =
0x000ff8c8`) and searching for its little-endian bytes as data, not as a
`jal` operand - see below.

## What it operates on

`Shadow_RenderOccluderVolume(void *self)` reads a structure pointer at
`self+0x50`. Three of the fields it reads from what that pointer points to
match [`shadow_occluder_ground_truth.rs`](../../../../crates/formats/tests/shadow_occluder_ground_truth.rs)'s
already-pinned payload layout exactly, at the same byte offsets, independent
of this reading:

- **`+0x00`**: a `u16`, sanity-checked `< 0x100` before anything else runs.
  This is `n`, the face-record count - the ground-truth test's own sanity
  bound on the same field (`PSP_OCCLUDERS` tops out at 129 nodes, `n` values
  measured over lengths 272..4432, always small).
- **`+0x02`**: a `u16` read as the vertex-record count `m` (`*(short
  *)(*(int *)(self + 0x50) + 2)`) - the second header field the ground-truth
  test pins.
- **`+0x30`/`+0x40`**: read as a padded `vec4` AABB min and max
  (`iVar30+0x30/0x34/0x38` and `+0x40/0x44/0x48`), used to pick whichever box
  corner is farthest along the projection direction. This is the *same* field
  the design doc calls "the padded `vec4` bbox copy at `+0x30`/`+0x40`",
  found independently by the payload-closure sweep and confirmed to match the
  packed box at `+0x0c`/`+0x18` on 97 of 129 nodes.

Three independently-pinned struct offsets landing exactly where this function
reads them was already strong evidence on its own; the class-ID dispatch
chain traced below (registration call site to method-table slot) confirms it
statically rather than by structural inference alone.

### What the padded box carries where it differs from the packed one

The 32 nodes where `+0x30`/`+0x40` does *not* match `+0x0c`/`+0x18` within
this project's `1e-6` tolerance were unread until 2026-09-03, and a first
pass at reading them **overclaimed a clean deterministic rule from a sample
that was structurally selected to fit it** - the 32 mismatchers only, never
checked against the 97 matchers for a counterexample. Corrected the same
day, against the full corpus rather than the mismatcher subset:

- **14 nodes have a positive header `min.y`. 12 get `+0x30`'s `min.y`
  floored to exactly `0.0`** - extending the box down to the ground plane
  even though the hull's own geometry sits entirely above it. **The other 2
  keep their real, positive `min.y` unfloored**: both `shadow_lodShape`,
  both in `Data.wad#809`/`#812`, both the smallest positive `min.y` in the
  set (`0.7280522`) - a sibling `shadowShape` node at the identical value in
  the same two files *does* get floored. No discriminator found for the
  exception; not the same node-type split either, since the pattern
  reverses on other files (below).
- **Where the packed header's `max.y` is the denormal authoring sentinel**
  `0x00800000` (the same one `BEData.wad#20`'s flat hull declares, already
  known from the payload-closure work): 70 nodes carry it, 16 of which have
  a true vertex-derived `max.y` that is itself exactly `0.0` (where the two
  possible behaviours are indistinguishable and excluded). **Of the other
  54: 16 get `+0x40`'s `max.y` set to the true vertex-derived value, and 38
  get a hard `0.0` instead.** Neither node name (`shadowShape` vs
  `shadow_lodShape`) nor the specific numeric value discriminates - the
  same `max.y` value gets fixed in one file's node and hard-zeroed in
  another's sibling. This is also why `PSP_OCCLUDERS_WITH_PADDED_BBOX = 97`
  is not "97 identical copies" - the test's `1e-6` tolerance can't tell a
  true copy from a hard-zero substitution against the `~1e-38` denormal, so
  an unknown slice of the 97 is the latter; see the constant's own doc
  comment.

**The honest summary: the padded box leans toward the ground plane far more
often than away from it, which is still consistent with it being a
shadow-relevant box rather than a plain geometry cache - but it is not the
single deterministic rule this page claimed on first read.** Read `HOW`
this function actually *uses* whichever value lands there (it picks the
farthest AABB corner along the projection direction, so a floored `min.y`
changes the answer only when the light-facing direction has a negative Y
component) before trusting either version further. See
`PSP_OCCLUDERS_WITH_PADDED_BBOX`'s doc comment in
[`shadow_occluder_ground_truth.rs`](../../../../crates/formats/tests/shadow_occluder_ground_truth.rs)
for the full numbers; not pinned as an assertion there, since the split

### File/build-version, checked and ruled out; the world-space population does differ on X/Z

2026-09-04, [`shadow_padding_probe.rs`](../../../../crates/formats/examples/shadow_padding_probe.rs)
(`cargo run -q -p oag-formats --example shadow_padding_probe`) walked all 32
mismatching nodes by archive path and directory index to check the two
remaining Open items directly, rather than guess further.

**File identity is not the discriminator.** There is no per-entry
build/version field to check in the first place - `docs/formats/wad.md`'s
directory has no timestamp, so "file/build-version" as literally stated
cannot be tested. What can be checked is whether *which file* a node sits in
predicts its behaviour, and it does not: the two `min.y` exceptions
(`Data.wad#809`/`#812`) sit in the same two files as the sibling
`shadowShape` nodes that *do* get floored, and three of the seventeen
distinct repeated `max.y` values split behaviour between sibling nodes in
adjacent file entries (`#589` hard-zero / `#592` kept at `-0.40124527`;
`#597` hard-zero / `#600` kept at `-0.02563141`; `#718`/`#721` kept /
`#724` hard-zero at `-0.39955962`). A same-file, same-value pair disagreeing
rules out both file identity and the specific numeric value at once.

**The repeated true-vertex value is a strong but incomplete predictor.**
Grouping the 32 mismatchers' `max.y` cases by their exact vertex-derived
value gives 17 distinct values; 14 of the 17 agree on kept-vs-hard-zero
across every node that shares them (up to seven repeats, e.g. `-0.44388673`
hard-zero on all 14 of its occurrences), and only the three listed above
disagree. That is a much stronger correlation than "no discriminator" reads,
but it is not a rule, and the three exceptions are exactly where the
mechanism actually needs explaining: whatever decides kept-vs-hard-zero
depends on something beyond the node's own geometry, since two nodes that
authored the identical value diverge.

**The 10 unnamed/world-space occluders do behave differently, confirmed on
real data.** Two of them - `Data.wad#142` and `Data.wad#144`, each carrying
two occluder nodes - diverge on `x` (one node of each pair also on `z`),
where every local/named node in the 32 diverges on `y` alone. This settles
the open question below: the world-space population is not just "the same
Y-axis behaviour on different nodes", it diverges on a different axis
entirely, consistent with a track-side occluder's hull genuinely extending
in `x`/`z` where a craft's local hull does not.
depends on `vertex_extent`'s own correctness and would be circular as a
test.

## What it does

1. **Builds the node's own world matrix** from `self+0x30` (a source 4x3/4x4
   transform) into scratch storage, the ordinary "compose this node's
   placement" step every scene-graph node does.
2. **Derives the shadow direction from the node's own orientation, not any
   light.** A fixed local axis constant (one of two candidates, selected by a
   global flag) is transformed by the node's own rotation and normalized.
   Nothing in this function reads a light direction, a `DirectionalLight`
   register, or any other external source. **This answers
   [`shadows.md`](../../../rendering/shadows.md)'s open question "where the
   light direction for a Pulse shadow comes from"**: it does not come from a
   light at all. Each occluder (and the craft's own drop shadow, below)
   projects along its own authored local axis, transformed into world space -
   consistent with the rest of the disc's evidence that Pulse has no light
   rig to take a direction from.
3. **Finds how far to project**, branching on whether the node is
   free-standing (`self+0x84 == 0`, the track-side occluder case) or attached
   to a parent (the craft-shadow case, below):
   - Free-standing: tests up to five nearby plane-shaped records for the
     first one the projected ray crosses, capped at a maximum distance of
     `1000.0`.
   - Attached: reads a height field off the parent object directly
     (`parent+0x94` then `+0x2f8`, offset by `10.0`) and divides by the
     projection direction's vertical component - a cheap "drop straight to
     the known ground height" shortcut rather than a ray cast.
4. **Extrudes**: writes a small `0.2`-scaled bias offset and the full
   projection-distance offset into two scratch vectors, then walks the `m`
   vertex records building near-cap and far-cap points for each - textbook
   shadow-volume extrusion (a near bias to avoid self-shadowing, a far cap at
   the projected distance).
5. **Silhouette test**: walks the `n` face records, dot-producting each
   face's stored normal against the projection direction from up to four
   reference points and recording a front/back-facing byte per face, then
   builds an edge list from the boundary between them - the standard
   "silhouette edges separate front-facing from back-facing polygons" step of
   stencil shadow-volume construction.

   **The data side of this step is now read, 2026-09-04**, which is what makes
   the walk cheap rather than a search: a face record's tail is `u16[4]` of
   *the face across each edge* at `+0x10` and `u16[4]` of vertex indices at
   `+0x18`. Adjacency is reciprocal on 14,328 of 14,328 edges disc-wide and a
   triangle's declared normal agrees with the geometry of the three vertices it
   indexes to 0.028 degrees, so the boundary this step walks is a lookup per
   edge. See [`shadows.md`](../../../rendering/shadows.md#the-16-bytes-at-0x10-are-the-edge-graph-and-it-closes-on-itself)
   and `oag_formats::shadow_occluder`.
6. **Draws**, through a short run of calls whose shape matches a stencil
   pass: a state/matrix select, a draw call using the edge-list vertex/index
   buffers just built, and calls bracketing it that read as enabling and
   restoring stencil test state.

## The craft's own drop shadow is the same function

[`exhaust.md`](exhaust.md#the-crafts-075-render-scale-confirmed-three-ways---and-a-residual-that-is-not-it)
already named `FUN_089038c8` from an unrelated angle - tracing
`g_craft_scale` (`_DAT_002ace1c`) - and described it as "a stencil
shadow-volume pass over the craft; it computes its ground projection, scales
it by `1.0 / g_craft_scale`". That is the `self+0x84 != 0` branch above: this
single function renders **both** the craft's own drop shadow (attached mode)
and every free-standing `Dynamic Shadow Occluder` hull on a circuit
(unattached mode, reading the `0x3c3` payload). Two independent readings -
one from a scale-residual investigation, one from the occluder payload's own
field layout - landed on the same function and agree on what it does where
they overlap (the `1.0 / g_craft_scale` division matches `_DAT_002ace1c`
exactly).

## The static class link, confirmed byte for byte

Vex node classes dispatch through a per-class method table, not an inline
compare - `exhaust.md`'s reading of `Vex_RegisterClass` already established
this ("Class dispatch is by descriptor lookup, never by immediate compare...
the registration call site is the only route"). Consistent with that: the
only static reference to `Shadow_RenderOccluderVolume` in the whole binary is
as **data**, not as a `jal` target - `get_xrefs_to` and `get_function_callers`
return nothing for it, same as every function reached this way.

`exhaust.md` names 46 callers of `Vex_RegisterClass` (`0x08908eb8`), one per
node class, but not which. Its own unrelocated-constant trap
(`autopilot.md`, `positional-audio.md`) is what hides them from `get_xrefs_to`
too; finding them means `search_instructions` on the unrelocated `jal`
target (`0x08908eb8 - 0x08804000 = 0x00104eb8`), which turns up 50 call
sites (some callers, like `Collision_RegisterNodeClasses`, register several
classes each). One of them is exactly the class this page is about:

```text
0890446c: addiu sp,sp,-0x10
08904470: lui   a0,0x9
08904478: addiu s0,a0,-0x7408        ; s0 = 0x88bf8, the class descriptor
0890447c: move  a0,s0
08904484: jal   0x00104eb8           ; Vex_RegisterClass(s0, 0x3c3)
08904488: li    a1,0x3c3             ; delay slot - the class id itself
0890448c: lui   a0,0x2d
08904494: sw    a0,0x38(s0)          ; s0+0x38 = 0x2cd214 (-> 0x08ad1214)
```

`0x3c3` is `Dynamic Shadow Occluder`'s class id, and it is a plain immediate
in a `li`, not a pointer needing any relocation correction - no ambiguity
here at all. Named `DynamicShadowOccluder_RegisterClass`.

The store at `s0+0x38` sets the descriptor's method-table pointer to
`0x2cd214`, unrelocated for `0x08ad1214`. Reading bytes from there shows a
run of zero and non-zero 4-byte words - a pointer table, non-zero words
pointing into code:

| Table offset | Raw (unrelocated) | Relocated |
| --- | --- | --- |
| `+0x0c` | `0x001404d0` | `0x089444d0` |
| `+0x14` | `0x0014079c` | `0x0894479c` |
| `+0x1c` | `0x001407a4` | `0x089447a4` |
| `+0x24` | `0x001407ac` | `0x089447ac` |
| `+0x2c` | `0x001407f4` | `0x089447f4` |
| `+0x34` | `0x000feea8` | `0x08902ea8` |
| `+0x3c` | `0x0014084c` | `0x0894484c` |
| **`+0x44`** | **`0x000ff8c8`** | **`0x089038c8`** |
| `+0x4c` | `0x00140864` | `0x08944864` |
| `+0x54` | `0x000fec04` | `0x08902c04` |
| … | … | … |

(Relocated with `python3 -c "print(hex(0x08804000 + v))"` for each value,
not by hand - an earlier pass at this table added the base incorrectly for
every row except the one independently cross-checked by the byte-pattern
search, and the wrong sums went uncaught until redone this way.
`+0x34`/`0x08902ea8` is a defined function in Ghidra; `+0x0c` and `+0x14`
are not - no function boundary, consistent with reaching them only through
this same indirect table the way `billboards.md`'s vtable calls do.)

Byte offset `+0x44` from the table base is `0x000ff8c8` - the unrelocated
form of `0x089038c8` - matching `Shadow_RenderOccluderVolume`'s address to
the byte, not to the neighbourhood. That is the load-bearing fact: class id
`0x3c3` -> `Vex_RegisterClass` call site -> method table `0x08ad1214` ->
`+0x44` -> `Shadow_RenderOccluderVolume`, with only that one offset and its
exact byte content asserted.

**What is not claimed**: that `+0x44` is "slot 7" of a fixed-stride table,
or that method tables sit a fixed distance apart. Checked directly against
two more registration functions (`FogCube_RegisterClass`, `0x08906b78`, and
`Mesh_Register`, `0x089100b8`) - their own `s0+0x38` stores give method
tables at `0x08ad1394` and `0x08ad1694`. Neither delta from
`0x08ad1214` (`0x180`, `0x480`) is a multiple of `0x88`, so table size
varies per class and there is no fixed grid to index into. What *does*
corroborate the table being real rather than coincidental bytes: `FogCube`'s
table repeats the exact same value at the exact same offset, `+0x0c`
(`0x001404d0` again), while `Mesh`'s table has a different value there
(`0x0010884c`) - consistent with `exhaust.md`'s "assign the base method
table … then overwrite it with the derived table": classes that don't
override a given virtual slot share the base class's default, and `Mesh`
overrides this one where `DynamicShadowOccluder` and `FogCube` don't. That
is evidence the table shape is real; it says nothing about which offset
means what.

**One sub-lead did not pan out and is recorded rather than silently
dropped**: the same registration function also passes a second unrelocated
constant (`0x280c10`, stored at `s0+0x28`) to another call
(`func_0x0026a878`), which looked like a promising candidate for the class's
*name* pointer - it would have named the class directly from code rather
than relying on the class-ID table's separately-established `0x3c3 ->
"Dynamic Shadow Occluder"` mapping. Relocating it the same way
(`+0x08804000 = 0x08a84c10`) lands mid-way through an unrelated packed
string blob (`"file"`, `"_world"`, `"AI track data"`, `"PCHSPEEDUPPAD"`, …) -
not a class name. Either `s0+0x28` isn't a name pointer, or this particular
constant needs a different correction than the one that works for `s0+0x38`
and for the class-ID table's own name column. Left unresolved rather than
asserted either way.

## The projection direction is `normalize(0.5, -5, 1)`

**Read 2026-09-04**, and it closes the open item that said the axis constant
was "unbacked by concrete bytes". It is not in `.data` or `.rodata`; it is in
`.bss`, written at class registration, and the obvious reading of the
instruction pair lands in `.text` instead - the exact
[`shield.md`](shield-pickup.md) trap, one segment over.

### The selector, at instruction level

```asm
08903a1c: lui   a0, 0x002c
08903a20: lw    a0, -16432(a0)     ; g_shadow_node_count, 0x08abffd0
08903a24: beq   a0, zero, +4       ; none alive -> take the constant
08903a28: lui   a0, 0x0009         ; (delay slot)
08903a2c: lui   a0, 0x0009
08903a30: b     +2
08903a34: addiu a0, a0, -29288     ; (delay slot) g_shadow_direction_override
08903a38: addiu a0, a0, -29272     ; g_shadow_direction
08903a3c: lv.q  v16, 0(a0)         ; the 4-float axis
```

Then `lv.q v4..v7` off `self+0xb0` - the node's own world matrix, built
earlier in the function - a `vtfm4` at `0x08903a5c` and a normalize. So the
direction is `normalize(M_node * axis)`: the constant is a **local** axis and
the node's own rotation carries it into world space, which is the reading step
2 of [What it does](#what-it-does) already gave from the decompiler, arrived at
here independently from the instruction stream.

### Why the addresses looked unbacked, and where they really are

Both `lui`/`addiu` pairs carry PRX relocations - `.rel.text` entries at offsets
`0xffa28`/`0xffa2c` (type 5/6) and `0xffa34`/`0xffa38` - with **`addr_base =
1`**, so the loader adds segment 1's base `0x002d5798` rather than segment 0's
zero. The encoded `0x00088d98`/`0x00088da8` are therefore
`0x0035e530`/`0x0035e540`, both inside `.bss` (`0x002d7300`, `0xba7c0` long),
which is `0x08b62530`/`0x08b62540` at this project's `0x08804000` base. Read as
segment-0 offsets they land in `.text` and read as instructions, which is
exactly what "no `.data`/`.rodata` content at either" was seeing.

### The constant, and the arithmetic tell that settles it

`Shadow_RegisterClass` (`0x08923518`) writes `g_shadow_direction` from four
immediates before calling `Vex_RegisterClass` (`0x08908eb8`) with class
**`0x3cb`** - `shadow` - and storing that class's method table
(`0x08ad1c24`) at `+0x38`, the same shape `DynamicShadowOccluder_RegisterClass`
has for `0x3c3`:

| Slot | Bits | Value |
| --- | --- | --- |
| `+0x00` | `0x3dc7dd06` | `+0.09758954` |
| `+0x04` | `0xbf79d448` | `-0.97589540` |
| `+0x08` | `0x3e47dd06` | `+0.19517908` |
| `+0x0c` | `0x00000000` | `0.0` |

Length `0.999995`, and **`x : z` is exactly `0.5`**: this is
`normalize(0.5, -5, 1)`, an authored triple rather than a fitted one, 12.60
degrees off straight down. That exactness is the tell - three independently
stored floats agreeing on one clean rational direction is not what a
misidentified address produces.

### `shadow` `0x3cb` is a direction override, not dead weight

`g_shadow_node_count` is a **reference count**, not a mode flag: `0x08923234`
increments it and `0x089232ac` decrements it, each beside a store of the same
method table (`0x08ad1c24`) into its object's `+0x38` - a constructor and a
destructor for the `shadow` class. While one is alive,
`Shadow_RenderOccluderVolume` reads `g_shadow_direction_override` instead, and
`0x0892342c` is what fills it: three floats at `+0x10` of the object at
`self+0x30`, **each negated**.

So the class the census recorded as inert is inert *in the shipped data* and
not in the code: it is a scene node whose job is to point every shadow
somewhere else. `shadow` `0x3cb` is authored **zero times across all 415
`.vex` files on the Pulse disc**
([`shadow_occluder_ground_truth.rs`](../../../../crates/formats/tests/shadow_occluder_ground_truth.rs)),
so the count is zero in any scene the disc can build and every shadow projects
along the constant.

Confidence **88** on the constant's value (instruction-level immediates, an
exact rational tell, and the relocation table read directly); **82** on the
override reading, which rests on the increment/decrement pair and the shared
method table rather than on a full decompilation of the class. Neither is
runtime-verified, which is what keeps both under the 95-100 band.

## Open

- ~~The fixed local axis constant's value~~ **Read 2026-09-04**: it is
  `normalize(0.5, -5, 1)` in `g_shadow_direction` (`0x08b62540`), and the
  second candidate is an override a live `shadow` `0x3cb` node installs. See
  the section above.
- **What `0x0892342c` belongs to.** Its tail fills
  `g_shadow_direction_override` from an object's own vector; its entry is at
  `0x0892342c` and the rest of it is unread, so it carries no name here.
- ~~The `m` face-to-vertex index mapping's exact byte layout~~ **Read
  2026-09-04**: `u16[4]` of per-edge adjacent faces at `+0x10` and `u16[4]` of
  vertex indices at `+0x18`, closing on 14,328 of 14,328 reciprocal edges. See
  step 5 above.
- **The `s0+0x28` field's real meaning**, from the sub-lead above - not a
  class name pointer as far as this pass could tell, and what it actually is
  was not chased further.
- **The fixed local axis constant's value.** Read at one of two addresses
  selected by a global flag; both are unbacked by concrete bytes in this
  Ghidra import (no `.data`/`.rodata` content at either), so the axis itself
  (down? forward? something authored per-title?) is unread.
- **The four-reference-point silhouette test.** Faces are tested against the
  projection direction from up to four slightly different points rather than
  one; which four and why is not worked out.
- **The near-cap bias constant (`0.2`) and far-cap distance cap (`1000.0`)**
  are read directly off the decompilation, not cross-checked against any
  disc-authored value.
- **No runtime trace exists for any of this.** A PPSSPP watchpoint on
  `self+0x50` during a lap that passes a known occluder-bearing circuit
  section would be the fastest way into the 95-100 band.
- ~~What discriminates the padded box's two exceptions from its two rules~~
  **Checked 2026-09-04: file/build-version ruled out** (no such field exists
  in the WAD directory to begin with, and sibling nodes in the same file
  disagree regardless); **the repeated exact vertex-derived value predicts
  behaviour on 14 of 17 groups but not the other 3** - see the new section
  above. What actually decides the 3 exceptions, and the 2-of-14 `min.y`
  ones, is still open: it depends on something other than the node's own
  geometry, file, or name.
- ~~Whether `f32::min`/hard-zero and true-vertex-value are the only two
  behaviours, or whether a third population (the unnamed/track-side nodes)
  behaves differently again on `X`/`Z`~~ **Confirmed 2026-09-04**: two
  world-space nodes (`Data.wad#142`, `Data.wad#144`) diverge on `x` (one also
  on `z`), where every local/named node only ever diverges on `y` - see the
  new section above.
