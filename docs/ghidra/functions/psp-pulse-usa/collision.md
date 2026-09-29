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

### The packing subtracts no base, and the range is enforced by a clamp

`SapAxis_Update` (`0x08833cc8`) is where an endpoint is packed, and the
arithmetic is bare:

```text
08833d58  sub.s  f13,f13,f15      ; min -= 1.0   (a one-unit skin)
08833d74  trunc.w.s f13,f13       ; (int)min
08833db4  addiu  t2,t2,0x400      ; + 1024
08833dbc  sll    t2,t2,0x1        ; * 2
08833dc8  or     t2,t2,t0         ; | (objectId << 12)
08833dd4  and    t2,t2,a3         ; & 0x3fffff
```

with the max endpoint taking `add.s f12,f12,f15` and an extra `addiu t1,t1,0x1`
at `0x08833e78`. **There is no base, no per-level offset, and no per-track
origin** - the float that reaches `trunc.w.s` is the world coordinate. Confidence
**92**.

`Sap_Init` (`0x0882f8f4`) does not establish one either: it allocates the three
0xa4-byte axis structures, zeroes `sap+0x10`, and passes its fourth argument
through to each axis. The only coordinate state an axis keeps is `+0x9c` and
`+0xa0`, a running min and max that `SapAxis_Update` *records* and never
subtracts.

**What actually bounds the input is a clamp, applied identically at all three
entry points** - `Sap_Insert` (`0x0882fb20`), `Sap_Update` (`0x088301e0`) and
`Sap_QueryAabb` (`0x088304d4`) each run the same eight instructions before
packing anything:

```text
lui   a0,0x8ab
addiu a0,a0,0xc50
lv.q  C720,0x0(a0)        ; g_sap_clamp_min
lv.q  C730,0x10(a0)       ; g_sap_clamp_max
vmax.t C700,C700,C720     ; both corners raised to the min
vmax.t C710,C710,C720
vmin.t C700,C700,C730     ; both corners lowered to the max
vmin.t C710,C710,C730
```

So a coordinate outside the window **saturates onto the boundary rather than
wrapping**, and because the query clamps the same way as the insert, the result
is still conservative: the broadphase over-reports and the narrowphase filters.
Confidence **90** for the clamp itself.

**The two globals are all zero in shipped `.data`, and nothing in the image
writes them - and the clamp is non-degenerate anyway, confirmed live.**
`0x08ab0c50` and `0x08ab0c60` have three read references each - the three
functions above - and no write, by both `get_xrefs_to` and a scan for the
`0xc50`/`0xc60` operand forms; `Sap_Init`, the single `.ctors` entry's region,
and the surrounding 80 bytes are all clear. Independently re-checked
2026-09-02 against the shipped disc image byte-for-byte (`BOOT.BIN`'s ELF
program header maps `0x08ab0c50` to file offset `0x2accd0`, which reads all
zero) - the static reading of `.data` itself was never in doubt.

**What was wrong was the inference drawn from it.** Read live off a running
`pulse-psp-usa.chd` (PPSSPP v1.20.4, `RemoteDebuggerOnStartup`, a read
watchpoint armed on `g_sap_clamp_min` as its own positive control): the
watchpoint fires during a live race, at `PC=0x08830534` - exactly
`Sap_QueryAabb`'s entry (`0x088304d4`) plus the `0x60`-byte offset to its own
copy of the eight-instruction clamp block above, proving the clamp path is
genuinely executing against these two addresses in a live race, not a quiet
or dead one. The values it reads are **not zero**:

```text
g_sap_clamp_min = (-1021.0, -1021.0, -1021.0, 0.0)
g_sap_clamp_max = ( 1021.0,  1021.0,  1021.0, 0.0)
```

identical across two independent cold boots. That is not the degenerate
single-cell reading this page used to hedge on - it is close to the packing's
own representable limit (`bits[0:11] = ((int)coord + 0x400) * 2`, i.e. an
11-bit field that needs `coord` within roughly `[-1024, 1023]` to stay
non-negative before the `* 2`), which is exactly the shape a deliberately
chosen clamp window for this packing would have. **Confidence 92** for the
clamp being genuinely ±1021-ish and not degenerate - a runtime trace, not
corroborated in a second binary (PS2 Pulse has no equivalent broadphase to
check this against), so short of the 95+ band per the confidence rubric.

The write site itself is still not found: a write watchpoint on both globals,
armed within roughly 100ms of emulated time after a cold boot (`cpu.status`
read `ticks: 22061009` at the first successful connection, CPU already
running rather than paused), never fired through menu navigation and a full
track load into a live race - so whatever writes these two quadwords runs
earlier than that, most plausibly through an address computed with an
immediate split the `0xc50`/`0xc60` operand scan would not have matched (a
base a few words off `0x08ab0c50` itself, e.g. `addiu a0,a0,0xc40` followed by
`sv.q ...,0x10(a0)`), rather than genuinely writer-less. Not chased further:
nothing here turns on which instruction does it.

Nothing a reimplementation needs turns on it either way. Both readings make
the packing a *conservative accelerator*: it never drops a genuinely
overlapping pair, so exact AABB overlap is a legal substitute and the bit
packing does not have to be reproduced at all.

- **World broadphase**: 1024 slots, 8 cursors per axis, returns collider indices.
- **Per-mesh**: one entry per triangle AABB, where the **bit index is the
  triangle index directly**.

`Sap_QueryAabb` (`0x088304d4`) runs a range query per axis into a bitset, ANDs
the three, and expands set bits into an index array.

## Raycasts

`Collision_RaycastWorld` (`0x08816ac0`) splits the segment into
`floor(len * 0.01) + 1` sub-segments, queries the world broadphase, skips self
and skips Reset colliders unless explicitly requested, then for each remaining
candidate runs a **fresh virtual dispatch** - `(*collider+0x78)+0xc`, called
with `collider + *(short*)(collider+0x78+0x8)` - and branches on its return
value: exactly `1` (and only when `param_7` bit 0 is clear) calls
`Collision_RaycastMesh`; exactly `3` (and only when `param_7` bit 1 is clear)
calls `Collision_RaycastBox`; every other return value (`0`, `2`, and `4` and
up) skips the candidate with no raycast at all. **This is a different read from
the `collider+0x6c` surface type** (`Wall`/`Floor`/`Reset`/`MagFloor`, already
recovered above) already used two lines earlier in the same function to decide
the Reset skip - `collider+0x78` is not yet in this page's collider layout
table. **It is a shape classifier, not the surface-type enum, and it is the
same accessor `Collision_DispatchPair` reads for its own dispatch below**
(identical `(*collider+0x78)+0xc` call, confirmed byte-for-byte 2026-08-25):
`1` is a mesh-shaped collider, `3` is a box-shaped one, settled by cross-reading
`Collision_BoxAgainstMesh`'s own field accesses against each side's classified
value - see [the corrected paragraph there](#collision_stepnarrowphase-0x088159c0-builds-the-pair-list)
for the derivation. So `1`/`3` here and `1`/`3` there are the *same* numbering,
not reversed; do not, however, assume it lines up 1:1 with `Surface`
(`Wall`/`Floor`/`Reset`/`MagFloor`) - a shape classifier and a surface-material
tag are different axes in principle, and nothing yet ties this one's raw
values back to specific node types beyond "whatever shape `Floor`/`Wall`/etc.
colliders are actually built as." Every box collider's own classifier is now
confirmed directly rather than by inference - see
[the narrowphase-pairing section below](#collision_stepnarrowphase-0x088159c0-builds-the-pair-list)
for the two-instruction stub that settles it at confidence 92.

`Collision_RaycastMesh` (`0x0881646c`) does the AABB clip, two slab rejects,
per-mesh sweep and prune, then a segment-triangle test using a plane sign
change on both endpoints plus three edge half-space tests.
`Collision_RaycastBox` (`0x08817b1c`, renamed 2026-08-25 from `FUN_08817b1c`)
instead separating-axis-rejects the segment against the collider's own three
box axes (the same `+0x1d0`/`+0x1d4`/`+0x1d8` dimensions
`Collider_BoxSamplePoints` reads), then builds the box's 12 triangles (two per
face) and runs the same segment-triangle test against each - a raycast against
a procedural box rather than an authored mesh. Confidence **85** on
`Collision_RaycastBox` itself: read from the decompiler this pass, with the
shared box-dimension field and the 12-triangle loop count both matching the box
collider layout `Collider_SetBoxDimensions` already established.

The hit result is 0x2c bytes: point (`+0x00`), normal (`+0x10`), collider
index (`+0x20`), the averaged per-vertex scalar (`+0x24`), and, at `+0x28`,
**which of the two narrowphase raycasts produced the hit** - `Collision_RaycastMesh`
always writes the constant `1` there, `Collision_RaycastBox` always writes the
constant `3`, both as raw denormal-float bit patterns for the integer
(`1.4013e-45`/`4.2039e-45` in the decompiler's own float rendering), and never
any other value from this call site. It is not a boolean "did anything hit"
flag - that is a separate byte (`Collision_RaycastWorld`'s own `local_6d0[0]`)
tested by the outer loop before this field is ever read back, and it is not a
raw copy of `collider+0x6c` either, for the same reason the dispatch above is
not: both narrowphase functions write a fixed literal, not anything read off
the collider. Read 2026-08-25 by decompiling all three functions and
cross-checking the offset arithmetic three ways: the caller's own copy into its
output param (`local_38[10]`, byte `0x28`), the callee's write through the same
pointer (`param_4[0x12]`, byte `0x48` from a shared struct base two calls up
the stack), and the constant each callee writes matching its own branch
condition in the caller. This is
[`craft+0x208`](engine.md#the-probes-depenetrate-as-well-as-push)'s field - see
that page for what this settles about the hover probe's escape gate.
Confidence **90**.

## Contact generation

Read end to end this pass. The one-paragraph summary that used to stand here
(a per-triangle separating-axis reject, ten box sample points, 0x40-byte
contacts, 128 slots) was right about every part of it, and the parts it did not
say are the ones a reimplementation needs: **what the ten sample points are,
what a contact's point and depth actually hold, and the order contacts resolve
in**.

### The frame, from `Body_StepWorld` (`0x0884f70c`)

Six passes, the last two being the ones here.
[contact-response.md](contact-response.md#frame-order-from-body_stepworld-0x0884f70c)
lists all six; the tail of the function is:

```text
Collision_StepNarrowphase(dt, world, &count)      ; pass 5, fills world+0x450
for i in 0 .. count:                              ; pass 6, ASCENDING
    Collision_GetContact(world, i, &contact)
    bodyA = bodyForProxy(contact+0x28)            ; the box side
    bodyB = bodyForProxy(contact+0x20)            ; the mesh side
    if bodyA == 0 or bodyB == 0:
        Body_ResolveContact(dt, contact, whichever is non-zero)
    else:
        FUN_0884ef30(dt, contact, bodyB, bodyA)   ; two-body, unread
```

**Iteration is ascending contact index**, and each contact reads the body state
the previous one left behind - there is no accumulate-then-apply. A craft against
track geometry always takes the one-body path, because track meshes have no
body. Confidence **88**.

The contact array is at `world+0x450` with stride `0x40` and the count at
`world+0x2450`, which fixes the capacity at exactly **128** by layout;
`Collision_AddContact` increments the count with no bound test, so contact 128
would overwrite the count itself. `Collision_GetContact` (`0x08815c1c`) is four
instructions and is the only reader.

### `Collision_StepNarrowphase` (`0x088159c0`) builds the pair list

`Sap_Update` for every proxy (descending index), a flush, then `Sap_QueryAabb`
per proxy into a scratch list of `(self, other)` pairs, skipping self-pairs.
Each pair goes to `Collision_DispatchPair` (`0x08816eac`), which switches on
each side's shape kind through the same `collider+0x78` virtual read the
[raycast dispatch](#raycasts) uses: kind 1 against kind 1 is `0x08815ccc`, kind
1 against kind 3 is `Collision_BoxAgainstMesh` with the **kind-1 side passed
first** whichever order the pair arrived in, and kind 3 against kind 3 is
`Collision_BoxAgainstBox` (`0x0881702c`) gated on a world flag at
`world+0x5464`.

**Corrected 2026-08-25: kind 1 is `mesh`, kind 3 is `box` - the reverse of
what this paragraph said before.** The previous labels (`kind 1 (box)`,
`kind 3 (mesh)`, "mesh passed first") were inferred from branch order alone,
with `0x08815ccc` and `Collision_BoxAgainstBox` (`0x0881702c`) left unread;
decompiling
`Collision_BoxAgainstMesh` this pass shows its own two collider arguments used
unevenly - the one classified kind 1 supplies mesh vertices/indices at
`+0x84`/`+0x8c`, the one classified kind 3 goes through `func_0x00014a00`, the
same box-corner helper `Collision_RaycastBox` calls on its own box collider -
so kind 1 is the mesh side and kind 3 is the box side, and it is passed
*first* into `Collision_BoxAgainstMesh` regardless of which side of the pair
it arrived as (both call sites in `Collision_DispatchPair` reorder to put the
kind-1/mesh collider in the earlier argument). That makes this the *same*
numbering the raycast dispatch above uses (`Collision_RaycastMesh` at `1`,
`Collision_RaycastBox` at `3`), not the reverse of it - the two accessors were
never in conflict, only this page's gloss on one of them was backwards.

**The classifier itself is now read directly, not just inferred from field
usage.** Every box collider's `collider+0x78` descriptor is written by
`Collider_InitBox` (`0x088188a0`), the box-collider field/vtable initialiser
called from every place a box collider is allocated (`Collision_AddBoxCollider`,
`0x0881585c`) - it hardcodes the descriptor pointer to a fixed static object
(offset `0x2c3bc0` from the same base the `jal`-target relocation trap applies
to; `docs/reverse-engineering/methodology.md`'s image-base-add trick also
applies to `.data` words carrying code/data pointers here, not only to `jal`
operands, which is new information worth carrying past this page) whose own
`+0xc` slot is `Collider_BoxShapeKind` (`0x0881894c`), a **two-instruction
stub**: `jr ra; li v0, 0x3` - every box collider classifies as exactly `3`,
unconditionally, with no branch of any kind. Confidence **92** on the
classifier identity: this is no longer an inference from which fields a caller
happens to read, it is the constant the classifier function itself returns,
read straight off its two instructions.

**And `Collision_BoxAgainstBox` (`0x0881702c`) is not a stub - it is a full,
working oriented-box-against-oriented-box narrowphase**, decompiled in full
this pass.
Fifteen-axis separating-axis theorem (three face axes each side, nine
edge-edge cross products - the standard OBB-OBB SAT, not GJK or anything
custom), reading each collider's basis rows and half-dimensions from the same
`+0x80`/`+0x90`/`+0xa0`/`+0x1d0..0x1d8` fields the box raycast and
`Collider_BoxSamplePoints` already established. On overlap it writes a real
contact into the shared array at `world+0x2450`-counted, `+0x40`-stride slots -
the same array `Collision_AddContact`/`Collision_GetContact` already
documented - with normal, midpoint, penetration depth, both collider indices,
both owner ids (`+0x60`), and a friction term read from each collider's own
`+0x64`-equivalent field. **Two gates stand between this code existing and it
ever running for a pair of craft, and both are now confirmed live, not just
plausible from static reading.** `Collision_DispatchPair` only reaches it when
`world+0x5464` is nonzero, and the function itself additionally requires both
colliders' own `+0x68` byte to be nonzero (a box-collider field, not the same
struct as the mesh collider's layout table above). `world+0x5464` is written
`1` unconditionally by what reads as the collision world's own constructor
(`FUN_088151bc` - sets up the same `+0x5454` object-count field, `+0x5458`
ready flag and `+0x440` broadphase pointer `Collision_RaycastWorld` itself
reads, and stores its pointer to two global world-instance slots at
`0x08ab0820`/`0x08ab0824`, image-base-relative literals) and written `0` by an
unrelated, much larger setup routine (`FUN_08822dd8`) whose own allocation
pattern reads like a different subsystem (menu or front-end, not race setup)
reaching into the world through one of those same globals to disable it.

**Settled 2026-09-07, live, PPSSPP v1.20.4, a full eight-craft SINGLE RACE
grid on `pulse-psp-usa.chd`:** reading `world+0x5464` off the running game
during the race gave `1`, and every one of the eight craft's own box
collider's `+0x68` byte (reached via `craft+0x1cc` -> body, `body+0x3bc` ->
collider) read `1` too - all eight, not a sample. Two craft were then
teleported onto the same point (`memory.write` on the followed craft's body at
a `Ship_UpdateCraft` breakpoint, the same mechanism `place()` in
`scripts/psp-drive.py` uses for a single craft), and a breakpoint on
`Collision_BoxAgainstBox`'s own entry (`0x0881702c`) fired **ten times running**
with `a1`/`a2` resolving, through the proxy table at `world+0x2454`
(stride `0xc`, collider pointer at each entry's offset `0`), to exactly the two
placed craft's colliders - their `+0x60` owner ids read `7` and `6` (a second
run, with a different pair placed, read `7` and `5`), matching the known
craft addresses directly, not inferred. The world's own
contact counter at `world+0x2450` was read `0` at the first hit's entry and `1`
at the very next hit's entry (same frame, the pair visited in the opposite
order), which is the counter actually incrementing across the call that ran in
between - not just dispatch, a real contact written. Confidence **92** on "runs
for two craft, and produces a contact" - a live measurement on the exact
proxy/collider/owner chain this page already predicted, not an inference from
static reading. See [`pair.rs`](../../../../crates/physics/src/pair.rs) for
what changed as a result.

**This retires the claim `contact-response.md` and `crates/physics/src/pair.rs`
used to carry** - that craft-to-craft detection "never comes out of Pulse's
narrowphase" on the strength of `0x08815ccc` being a stub. `0x08815ccc` is the
kind-1-vs-kind-1 (mesh vs mesh) dispatch, and craft, being box colliders, are
kind 3: their pairs never reach `0x08815ccc` at all. They reach
`Collision_BoxAgainstBox` instead, and now-confirmed-live it runs for them.

### `Collision_BoxAgainstMesh` (`0x08815cd4`) is a ten-ray star from the box centre

```text
for each candidate triangle from Sap_QueryAabb(mesh->sap, boxAabb):   ; ASCENDING
    reject if the triangle misses the box on any of the box's own 3 axes
    n = normalise(cross(v1 - v0, v2 - v1))                            ; raw winding
    for i in 0 .. 9:                                                  ; ASCENDING
        s = samplePoint[i]
        skip unless dot(boxCentre - s, n) > 0
        skip unless Collision_SegmentHitsTriangle(boxCentre, s, v0, v1, v2)
        Collision_AddContact(5.0, world, s, v0, v1, v2, n, mesh, box, ...)
```

Three things follow that the summary did not carry:

- **The penetration test is a segment from the box centre to the sample point**,
  not a plane distance. `Collision_SegmentHitsTriangle` (`0x08818d58`) is the
  boolean twin of `Collision_SegmentTriangle`: opposite plane signs on the two
  ends, then three edge half-space tests on the intersection point. So a sample
  point counts as penetrating exactly when the segment from the centre to it
  crosses the triangle - which is a raycast, and is what makes the whole thing
  reimplementable against an ordinary raycaster.
- **Walls are single-sided.** The `dot(boxCentre - s, n) > 0` gate rejects the
  sample point when the triangle's stored winding faces away from the box centre.
  There is no flip anywhere: the normal that reaches the contact is the raw
  winding normal.
- **The separating-axis test uses the box's three axes only**, not the
  triangle's normal and not the nine edge cross products, so it is a cheap
  conservative reject rather than a real SAT.

Confidence **88**.

### The ten sample points, from `Collider_BoxSamplePoints` (`0x08818a00`)

Twenty branch-free instructions over three axis vectors the collider caches.
`Collider_SetBoxTransform` (`0x08818964`) fills those from the body's basis and
the box dimensions, `axis = row_i * dimension_i * 0.5` (the `0.5` is
`vfim.s S400,0x3800`, a half-float immediate):

| Collider field | Value |
| --- | --- |
| `+0x80`, `+0x90`, `+0xa0`, `+0xb0` | basis rows 0/1/2 and the centre, copied from `body+0x00..0x30` |
| `+0xc0` | `row1 * dimension1 * 0.5` |
| `+0xd0` | `row2 * dimension2 * 0.5` |
| `+0xe0` | `row0 * dimension0 * 0.5` |
| `+0x1d0`, `+0x1d4`, `+0x1d8` | the box dimensions, from `Collider_SetBoxDimensions` (`0x08818954`) |

With `A = +0xc0`, `B = +0xd0`, `C = +0xe0`, the ten points land at `+0x130`
upward, 16 bytes apart, in this order:

```text
0..3   (centre - A) +- C +- B
4..7   (centre + A) +- C +- B
8      centre + 0.25*B + C
9      centre + 0.25*B - C
```

The `0.25` is `vfim.s S400,0x3400`. Points 0-7 are the eight box corners; **8
and 9 are a left/right pair an eighth of the hull's length ahead of centre, at
full half-width** - wall-scrape probes, which is what a craft that never rests
its hull on a floor actually needs. Confidence **90**.

Rows 0/1/2 are the body's right, up and forward and dimensions 0/1/2 are width,
height and length. That used to rest only on `Body_SetBoxInertia`'s agreement
with the trace fit (see
[rigid-body.md](rigid-body.md#answered-body_setboxinertia-0x0884e1ac-writes-body0x40)) -
a leg that cannot separate width from length, because its literal box is square
in `x` and `z`. A second, independent read now confirms the same mapping and
distinguishes the two: **confidence 95**, up from 88.

### The dimensions feeding the collider are scaled, not the authored `<Misc>` values

Read at instruction level in `Ship_InitCraft` (`0x08840c74`, `0x08841360`-
`0x0884139c`), the block that builds the ship's own box collider:

```text
local_1c0 = stats+0x78 * K2      ; <Misc width>,  DAT_08ab0e1c
local_1bc = stats+0x80 * K2      ; <Misc height>
local_1b8 = stats+0x7c * K2      ; <Misc length>
FUN_0884e694(world, class, 1, 1, &craft+0x794, &local_1c0)
```

`stats+0x78`/`+0x7c`/`+0x80` are `<Misc>`'s `width`/`length`/`height`
(`handling-stats.md`'s PS2-confirmed offsets), and `K2` (`0x08ab0e1c`) is the
same global `hover::TARGET_GLOBAL_SCALE` already reads as `0.75` for the hover
target height - two unrelated systems, one shared scale. `FUN_0884e694`
dispatches to `Body_SetBoxDimensions` (`0x0884dccc`) with `(width * K2, height *
K2, length * K2)` in that argument order, which is also what settles the
width/length ordering above: `stats+0x78` (width) reaches the first argument and
`stats+0x7c` (length) the third, unambiguous where the inertia tensor's square
`(12, 8, 12)` box could not distinguish them.

**The scale itself was already on record, in a sibling doc, for two days
before it reached the collision code.**
[rigid-body.md](rigid-body.md#answered-body_setboxinertia-0x0884e1ac-writes-body0x40)
noted in passing, while establishing that the inertia tensor is a code literal
rather than per-team: "`Misc` `width`/`length`/`height` are read a few lines
earlier in the same constructor, scaled by `0.75`, and handed to the collider
setup ... not to the inertia." That sentence was written 2026-07-28 and is
correct as far as it goes; it just never propagated to
`crates/physics/src/wall.rs::hull_sample_points`/`hull_extent`, which kept
building the hull box straight from `<Misc>` with no scale until this pass.
Fourth instance of the pattern HANDOVER already tracks ("a page can go stale
while a sibling page in the same tree already carries the correction") - this
one cost two days rather than reaching the code at all until a second,
independent chase (the six-tick-early wall contact) landed back on the same
instructions. The hull box was a third larger in every dimension than the
original's, which is the right direction and rough magnitude for that gap; see
`crates/trace/tests/wall_contact_ground_truth.rs` for the after-the-fix
measurement. Confidence **90** on the scale.

`Body_SyncBoxCollider` (`0x0884db20`) is what calls the transform setter, copies
the body's velocity to `collider+0x120` and rebuilds the collider AABB as
`body.position +- (|A| + |B| + |C|)` using the VFPU's absolute-value prefixes.

### What a contact holds, from `Collision_AddContact` (`0x08816864`)

The friction half is in
[contact-response.md](contact-response.md#collision_addcontact-fills-contact0x34-by-averaging-the-two-colliders);
the rest of the record is:

| Offset | Holds |
| --- | --- |
| `+0x00` | **the sample point**, world space - *not* where the segment met the triangle |
| `+0x10` | the unit triangle normal |
| `+0x20`, `+0x28` | mesh and box **proxy indices**, which pass 6 maps back to bodies |
| `+0x24`, `+0x2c` | mesh and box collider owner ids |
| `+0x30` | penetration depth |
| `+0x34` | combined friction |

The point being the *sample* point rather than the intersection point did not
matter while contacts resolved at the centre of mass. It matters now: the two
are a whole penetration depth apart along the normal, and the lever arm is
measured from one of them.

Depth is `-dot(n, sample - v0)` and the contact **only exists while that plane
distance is in `(-2.0, 0)`**: `0x08816920` rejects a sample point in front of
the plane and `0x08816938` rejects one more than `2.0` behind it. A deeper
overlap produces no contact at all, which is not a bug - a hull that far through
a wall crossed it within the frame, and that case belongs to pass 1's swept move.
The `5.0` passed in `f12` reaches `Collision_SegmentTriangle` as a maximum
distance and is unreachable given the `-2.0` gate, as is the `d < -5.0`
alternative at `0x08816a00`. Confidence **90**.

### How many contacts a craft-vs-track frame makes

Up to ten per candidate triangle, with no de-duplication between triangles: one
sample point behind two triangles of the same wall produces two contacts, and
both are resolved. The measured bound is the useful one -
`data/traces/talons-junction-standing-start.csv` never loses more than `5.21 %`
of speed in a contact frame, and one contact costs `3.5 %`, so **the recorded
scrape is one contact per frame**, consistent with a craft leaning on a wall
with one flank point. A square-on impact would generate five (four corners and a
flank point) and scrub `1 - 0.965^5 = 16.3 %`.

### Every mesh surface reaches the hull narrowphase, 2026-09-29

**Read end to end for the sunk-craft divergence** (`docs/gameplay/leaving-the-track.md`).
Nothing between the pair list and the resolver reads a collider's surface
type (`collider+0x6c`):

- `Collision_DispatchPair` (`0x08816eac`) switches on the shape classifier
  (`collider+0x78`, kind 1 mesh / kind 3 box) and nothing else.
- `Collision_BoxAgainstMesh` (`0x08815cd4`) walks every candidate triangle of
  the mesh with no read of `+0x6c`.
- `Collision_AddContact` (`0x08816864`) reads the two colliders' friction
  (`+0x64`) and owner (`+0x60`) and nothing else.

So a craft's box makes contacts against `Floor`, `MagFloor` and `Reset` exactly
as against `Wall`. The ring consumer proves non-wall contacts arrive:
`FUN_088418e0`, at `0x088426e8`-`0x08842728`, looks each ring record's mesh
proxy up through `world+0x2454` and calls `FUN_08844100(craft, 3)` when that
collider's `+0x6c` is `2` - a **`Reset` contact** reaching the gameplay loop.
Confidence **88**. `crates/physics/src/wall.rs` had filtered the hull to `Wall`
alone on its own reasoning ("the hover spring owns floors"); it now takes every
surface but `Reset`, which `crates/physics/src/reset.rs` handles.

### `Collision_AddContact` has a second test: the projection must land in the same triangle

`0x088168e4` onward, after the `(-2.0, 0)` plane-distance gate: with
`d = dot(n, sample - v0)`, it builds `p1 = sample - n * (d - 0.01)` - the sample
carried along the normal to `0.01` in front of the plane - and calls
`Collision_SegmentTriangle` (`0x08818bdc`) with `(5.0, sample, p1, v0, v1, v2)`.
The contact is written only if that returns `1`. `Collision_SegmentTriangle` is
a plane sign change plus three edge half-space tests (`>= 0`, inclusive) on the
crossing point, so **the sample point's perpendicular projection has to lie
inside the same triangle the centre-to-sample segment crossed**. The `5.0` is a
maximum start distance and is unreachable behind the `-2.0` gate. Confidence
**90**, decompiled and read against the call's argument order (`0x08815cd4`
passes `v0, v1, v2` to `Collision_AddContact` and `v0, v2, v1` to
`Collision_SegmentHitsTriangle`; both tests are winding-consistent).

Measured: placed 3.6 units into `16_Track`'s floor at spline index 200, the
original's first free tick moves the craft **1.81** along the normal - two
corners' depth - where four ungated contacts give about **3.5**. A prototype of
the gate reproduces the original tick for tick there (y `-41.26` vs `-41.28` on
the first tick, overshoot `-39.17` vs `-39.16` at tick 30, rest `-39.65` vs
`-39.60`). **Not ported**: on `03_Track` the same gate rejects all four corners
(the centre segments cross a long sliver triangle whose neighbour holds the
projections), and the original gets out through pass 1 below instead. Without
pass 1 the gate also strips wall contacts: `05_Track`'s lone Ace went from 60
wall-contact ticks to 2,475. The two land together or not at all.

### `Body_StepWorld`'s pass 1 is a clip along the velocity to the box face

The first per-body loop of `Body_StepWorld` (`0x0884f70c`), before any force
is evaluated:

```text
v = body+0x140
if v != 0:
    probe = normalise(v) * 10.0                  ; 0x41200000
    half  = FUN_0884dcec(body) * 0.5             ; the box (w, h, l) / 2
    du = dot(probe, row1); dr = dot(probe, row0); df = dot(probe, row2)
    if du >  half.h: probe *= half.h /  du       ; each factor from the unscaled
    if du < -half.h: probe *= half.h / -du       ; dot, applied multiplicatively,
    if dr >  half.w: probe *= half.w /  dr       ; in this order
    if dr < -half.w: probe *= half.w / -dr
    if df >  half.l: probe *= half.l /  df
    if df < -half.l: probe *= half.l / -df
    end = body+0x30 + v * dt + probe
    if Collision_RaycastWorld(world, body+0x30, end, &hit, body+0x3a8, 1, 2):
        Body_SetPosition(body+0x30 - normalise(probe) * 0.9 * |end - hit|)
```

`Collision_RaycastWorld`'s `1` is "do not skip `Reset`" and its `2` is "skip box
colliders" (read off its own `param_6`/`param_7` handling), so the clip runs
against every mesh surface. The `0.9` is `0x3f666666`. No velocity is touched.
Confidence **85**.

Measured on `03_Track` spline index 200, 3.6 units into the floor, with the
craft's centre 0.50 above the floor: the first frame whose velocity points down
lifts the original **0.757**, against `0.9 * (1.3125 + 0.024 - 0.50) = 0.752`
from the reading; the next frame its two hover probes are above the floor and
their penetration escapes lift it **1.82**. A prototype of pass 1 plus the gate
recovers ours to the same rest height (`4.663` vs `4.67`). **Not ported**: with
it, `01_Track`'s lone Ace loses its clean lap (the regression gate) - see
`docs/gameplay/leaving-the-track.md`. Replacing this crate's swept guard with it
did not close the `orig16` shove replay's tick-9 wall stop either.

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
| `0x08817b1c` | `Collision_RaycastBox` | 85 |
| `0x088304d4` | `Sap_QueryAabb` | 88 |
| `0x08816864` | `Collision_AddContact` | 86 |
| `0x08815c1c` | `Collision_GetContact` | 90 |
| `0x08818a00` | `Collider_BoxSamplePoints` | 90 |
| `0x08818964` | `Collider_SetBoxTransform` | 90 |
| `0x08815cd4` | `Collision_BoxAgainstMesh` | 88 |
| `0x0881702c` | `Collision_BoxAgainstBox` | 92 |
| `0x0881894c` | `Collider_BoxShapeKind` | 92 |
| `0x088188a0` | `Collider_InitBox` | 80 |
| `0x0881585c` | `Collision_AddBoxCollider` | 80 |
| `0x08818d58` | `Collision_SegmentHitsTriangle` | 88 |
| `0x088159c0` | `Collision_StepNarrowphase` | 85 |
| `0x08816eac` | `Collision_DispatchPair` | 85 |
| `0x08818954` | `Collider_SetBoxDimensions` | 90 |
| `0x0884db20` | `Body_SyncBoxCollider` | 85 |
| `0x0884dccc` | `Body_SetBoxDimensions` | 90 |
| `0x08818540` | `CollisionMesh_BuildSap` | 85 |
| `0x088183e8` | `CollisionMesh_Init` | 85 |
| `0x0881835c` | `CollisionMesh_AvgVertexScalar` | 84 |
| `0x0882f8f4` | `Sap_Init` | 82 |
| `0x0882fb20` | `Sap_Insert` | 80 |
| `0x088301e0` | `Sap_Update` | 82 |
| `0x08833cc8` | `SapAxis_Update` | 82 |
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
[the survey of all 16 tracks](../../../formats/collision.md#resolved-the-broadphase-clamps-world-space-rather-than-rebasing-it),
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
