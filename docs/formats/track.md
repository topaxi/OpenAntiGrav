# Track data

**Status: decoded and validated.** The `WO Track` spline graph parses on all 40
track files on the PSP disc with **zero bytes unaccounted for**, and is
implemented in [`oag_formats::track`](../../crates/formats/src/track.rs). The
visibility `section` payload is now decoded too, in
[`oag_formats::pvs`](../../crates/formats/src/pvs.rs), and validated against
both discs - see [below](#implemented-and-what-running-it-against-the-discs-corrected).

Two things that sound like they should be the same are not:

- **`section` nodes are a visibility partition**, not the track path. They carry
  a potentially-visible-set bitmask and a bounding box, and nothing else.
- **The track path lives in the `WO Track` node**, published at runtime under
  the resource name `"AI track data"`. It is a **graph of cubic B-spline
  paths**, with a racing line and AI corridor.

That correction matters. The `.vex` class-ID table lists `section` and `gate`
alongside `Start Position` and the pads, which invites reading them as the lap
structure. They are not.

## What a real track contains

`Data\Environments\01_Track\track.vex`, 5.0 MB, 2,071 nodes:

| Node | Count | Payload |
| --- | ---: | ---: |
| `Mesh` | 594 | 3.4 MB |
| `Transform` | 715 | 42 KB |
| `Texture` | 130 | 16 KB |
| **`section`** | **64** | 5 KB |
| `WO Track` | 1 | 79.6 KB |
| `Speedup Pad` | 9 | 10 KB |
| `Weapon Pad` | 7 | 12 KB |
| `Floor Collision` | 1 | 62 KB |
| `Wall Collision` | 1 | 39 KB |
| `Mag Floor Collision` | 1 | 9 KB |
| `Reset Collision` | 1 | 160 B |
| `Start Position` | 1 | 64 B |

**Exactly 64 sections** — the cap predicted from the 64-bit PVS mask, hit on the
nose. That is a strong independent confirmation of the mask reading.

Tracks are found via the front end, not by guessing names. The plugin
`Data\Plugins\PI001\Definition.xml` lists every track as

```xml
<PI_Track name="01_Track">
  <Values type="Race" soundregister="18" location="Data\Environments\01_Track"
          availableInZone="true" collisionCageEnabled="true"/>
</PI_Track>
```

so the `location` attribute plus the binary's `%s\%strack%s.vex` template gives
`track.vex`, `track_reversed.vex`, `zone_track.vex` and
`zone_track_reversed.vex`. Mining those raised name resolution in `Data.wad`
from 177 to 269 of 1,142 entries.

## Class IDs

Decoded from the table at `0x08ab2370` (stride 12, `{u32 id, char *name, ptr}`,
names at `0x08a84d40`). Confidence **94**: the five collision IDs already known
from an independent reading fell out of the same walk unchanged, and every ID
below matches a node type actually present in a shipped track.

| Node | ID | Bind handler |
| --- | ---: | --- |
| `WO Track` | `0x3bb` | `0x089294f8` |
| `Start Position` | `0x3bc` | `0x08926ae8` |
| `Speedup Pad` | `0x3bd` | `0x089264f4` |
| `Weapon Pad` | `0x3be` | shares a base |
| `Texture` | `0x3c1` | - |
| `section` | `0x3c9` | `0x08922974` |
| `gate` | `0x3ca` | **none** |

## `section`: visibility, not geometry

```c
struct SectionPayload {
    u8  index;         // array index, also the PVS bit position
    u8  has_bounds;    // selects the optional bbox block; set on every
                       // section on both discs
    u8  pad[6];        // never read, and never initialised either
    u32 pvs_mask_lo;   // sections 0..31 visible from here
    u32 pvs_mask_hi;   // sections 32..63; own bit OR'd in at load
    // if has_bounds:
    float bbox_min[4]; // the fourth component is w, not a coordinate
    float bbox_max[4];
    char  name[];      // NUL-terminated, padded to the 16-byte node alignment
};
```

The load path builds `1u64 << index` and publishes a 64-bit mask per section;
the lookup at `0x0891e908` returns all-ones when the index is out of range. That
is a textbook PVS.

**The engine hard-caps at 64 sections**, from the mask width and a 64-entry
gather buffer. Worth knowing before designing anything that assumes more.

Confidence **90** for the mask, **85** for the bounding box. The cap is
corroborated by the data: no control point on any of the 40 track files carries a
`section_id` above 63.

### Implemented, and what running it against the discs corrected

Decoded by [`oag_formats::pvs`](../../crates/formats/src/pvs.rs) and validated
against every track file on both Pulse discs by
[`pvs_ground_truth.rs`](../../crates/formats/tests/pvs_ground_truth.rs): 2,268
sections over 40 PSP track files and 3,045 over 59 PS2 ones, cross-checked
against 84,479 spline control points. See
[ADR-0011](../architecture/adr/0011-authored-pvs-before-frustum-culling.md) for
what the renderer does with it.

Five corrections to the struct above, all from shipped data rather than from a
re-read of the binary:

1. **The payload does not end at the bounding box.** A NUL-terminated name
   follows, padded to the node alignment, so a real `section` is 0x40, 0x50 or
   0x60 bytes and never 0x30. Nothing reads it at runtime, but a parser that
   assumes the struct is the whole payload is wrong about every section on
   every disc.
2. **The fourth float of each corner is `w`, not zero padding**, and the
   platforms disagree: PSP writes `0.0` on all 4,544 corners, PS2 writes `1.0`
   on 5,872 of 6,098. Either way it is not a coordinate, which is what a
   three-wide read would turn it into.
3. **A mask may name a section its own file does not declare** - 2.17% of set
   bits on PSP, 4.01% on PS2. Masks that outlived a deleted section; inert,
   because the original only ever tests bits for sections it is iterating.
   This is also the evidence for the field offsets: the stray rate at the
   documented `+0x08` is far below the rate four bytes either side (7.74% and
   13.33% on PSP, 8.69% and 22.94% on PS2), which is what a wrong offset
   reading uninitialised pad or a bbox float looks like.
4. **One id is authored three times over**, on 2 of 40 PSP and 2 of 59 PS2
   tracks, with byte-identical masks each time. A parser that treats a repeated
   index as corruption refuses four shipped tracks.
5. **116 of 50,218 PS2 control points name a section their file does not
   declare** - none of the 34,261 PSP ones do. The out-of-range all-ones
   fallback is therefore load-bearing on real data, not just a defensive branch.

Mask density, which is the figure any culling design rests on: **a mean of 7.6
of 64 sections visible from a section on PSP and 7.5 on PS2.**

**Pulse's class ids do not carry to Pure.** Pure track files use classes around
`0x36f..0x393` where Pulse uses `0x3b9..0x3e9`, so `CLASS_SECTION` matches
nothing on that disc. Recovering Pure's own numbering is separate work; until
then an empty result there means the constant does not apply, not that Pure has
no visibility partition.

Note that section ids are **not** guaranteed to be dense. `09_Track`'s reversed
variant has 44 `section` nodes and a maximum `section_id` of 45, so the ids come
from each node's explicit `index` field rather than from its position, and a
consumer must not assume `id < count`. The out-of-range branch in the lookup
exists for a reason.

### Which geometry a section governs: its parent's subtree

The association between sections and render geometry is structural, and it is
on the disc. A track is authored as **sibling groups**: one transform per
group, whose children are the `section` node and the group's geometry.

```text
Transform
+-- section  (id, PVS mask, box)
+-- Transform -- Mesh
+-- Transform -- Mesh
...
```

Every one of Moa Therma's 64 sections has exactly this shape - every
`section` node's parent is a transform whose other children are the meshes it
governs - and the sibling-group rule assigns a governing section to 97-100%
of the draw calls of all 40 PSP track files
([`pvs_placement_ground_truth.rs`](../../crates/render/tests/pvs_placement_ground_truth.rs)).
Implemented as
[`oag_formats::pvs::governing_sections`](../../crates/formats/src/pvs.rs).

Confidence **75**: the tree shape and coverage are measured on every PSP
track, and behaviour matches - hiding by group is the only rule that
reproduces the original's handling of the case below - but the original's
loader has not been read doing the walk.

**The case that proves membership is structural, not spatial**: Moa Therma
ships a coarse far-LOD copy of its track surface, coincident with the real
geometry but mapped differently (`0..1` against the strip's
`2/128..125/128`). Its group is governed by section 62, which only four
distant vantage sections list in their masks - and **no shipped mask names
both 62 and a detailed section the copy coincides with**, so from any single
viewpoint the artists show either the copy or the detail, never both. A
spatial rule places the copy with the racing sections whose boxes it sits in,
draws it coincident with the detailed strip, and the z-fight chops the
magstrip's painted lines into sideways-stepping segments
([`magstrip_ground_truth.rs`](../../crates/render/tests/magstrip_ground_truth.rs)).
See [ADR-0014](../architecture/adr/0014-authored-section-placement.md) for
what the renderer does with this.

## `WO Track`: the spline graph

### The file layout

```c
struct AiTrackHeader {      // 0x20 bytes
    u32       magic;        // 0x574f7464, "WOtd" read big-endian
    u32       version;      // >= 0x100 required; gates at 0x101, 0x103-0x105
    u32       path_count;
    u32       junction_count;
    Path     *paths;        // zero on disc, patched at load
    Junction *junctions;    // zero on disc, patched at load
    u32       always_1;     // +0x18: 1 in Pulse, 0 on all 16 Pure tracks
    u32       always_0;
};
```

Both array pointers are **zero in the file**, the same pattern as
[embedded textures](vex.md#embedded-textures): runtime pointers, patched in as
`AiTrack_ParsePayload` (`0x0887c57c`) walks a cursor through the payload. So the
arrays are positioned by a rule, and the rule is:

| Order | Contents | Size |
| --- | --- | --- |
| 1 | header | `0x20` |
| 2 | **reserved block, only when `version >= 0x101`** | `0x20` |
| 3 | `paths` | `0x20 * path_count` |
| 4 | `junctions` | `0x10 * junction_count` |
| 5 | each path's control points, in path order | `0x70 * point_count` |

**Step 2 is the whole difficulty.** The obvious rule — header, then paths, then
junctions, then points — is right about the *order* and wrong by 32 bytes,
because for version `0x101` and up the loader claims a second `0x20`-byte block
and hands its address to the track object before it reads the paths.

**A third corpus closes under the same layout: Wipeout Pure.** The
[Pure smoke probe](pure-status.md#reported-elsewhere) found `WO Track` nodes in
**16** Pure `.vex` files - under class ID `0x36d`, because Pure renumbers the
whole class table, at version `0x103` against Pulse's `0x105` - and
`encoded_len() == payload.len()` holds **exactly 16 of 16**, the same closure
that settled the layout on Pulse's 40. Both versions take the `>= 0x101` reserved
block, so the one structural trap on this page is exercised by two titles rather
than inferred from one. **One divergence, in the header**: the `+0x18` word
recorded above as always `1` reads `0` on all 16 Pure tracks (re-checked against
a Pulse track, which reads `1`), so it is "1 in Pulse, 0 in Pure" - a title- or
version-dependent flag rather than a constant of the format.

Reading the paths at `+0x20` on a `0x105` file lands one field early. It does not
look like an offset error: it looks like a subtly wrong struct, putting plausible
world coordinates where the orientation frame should be, yielding a `section_id`
of 185 on a 64-section track, and leaving 30 KB unexplained. That is what cost
an earlier pass a day.

### The check that settles it

The payload has to close. With the reserved block included:

```
0x20 + 0x20 + 0x20*paths + 0x10*junctions + 0x70*points == payload length
```

**40 of 40 track files, exact, zero remainder.** 86 paths, 34,261 control
points. A parser that is one structure out cannot make that come out even on one
file, let alone forty, so this is a determination rather than a plausible
reading. It is kept as a test
([`track_ground_truth.rs`](../../crates/formats/tests/track_ground_truth.rs))
rather than a claim, and [`AiTrack::encoded_len`] exists so anything else can
run the same check.

[`AiTrack::encoded_len`]: ../../crates/formats/src/track.rs

Confidence **94**: the layout is confirmed by both the parser's cursor
arithmetic and the data. Not 95+, because nothing has been run under an emulator;
see the [rubric](../reverse-engineering/confidence-rubric.md).

### The reserved block carries no data

32 bytes, whose first 28 are zero in every one of the 40 files. The last four
vary, and vary *implausibly*: `162.83` on one track, `315.75` on another, `0.0`
on ten, and denormals like `3.6e-22` and `7.0e-28` on four more. Values that
small are uninitialised memory, not a field.

The loader stores the block's address in the track object at `+0x08` and nothing
found so far reads it back. Treat it as scratch space the exporter reserved.
Confidence **85** that it carries nothing meaningful, from the value survey; the
consumer was **not** found, so this is a negative claim about a limited search.

### Paths

```c
struct Path {               // 0x20 bytes
    u32       point_count;
    float     max_spacing;  // longest gap between consecutive control points
    SplinePt *points;       // zero on disc, patched at load
    Junction *entry;        // u32 index on disc, 0x7fffffff = null
    Junction *exit;
    u8        unknown[12];  // zero in every shipped track
};
```

`max_spacing` was `unk04`. It is **exactly** the longest distance between
consecutive control points in that path, on all 86 paths, to `f32` precision.
Confidence **92**: an exact numerical match on 86 independent samples, though
what the engine uses it for has not been traced.

### Junctions

```c
struct Junction {           // 0x10 bytes, a 2-in / 2-out merge-split
    Path *prev_primary;     // u32 path index on disc, 0x7fffffff = null
    Path *prev_alternate;
    Path *next_primary;
    Path *next_alternate;
};
```

Slots 0 and 1 are predecessors, 2 and 3 successors. That ordering used to be
inferred from which slots each traversal direction reads; it is now **proven**
two independent ways across all 40 files:

1. **Cross-reference.** Every path is listed as a predecessor of the junction it
   exits, and as a successor of the junction it enters. 86 paths, no exceptions.
2. **Geometry.** Where a junction links two paths, the first path's last control
   point is **within one control-point step** of the second's first: about 6
   units, against 817 units for any unlinked pair. Read the slots the other way
   round and that collapses.

Confidence **92**.

`05_Track` shows what the structure is for: three paths and two junctions, with
junction 1 splitting into paths 0 and 1 and junction 0 merging them into path 2.
Paths 0 and 1 share both endpoints, so they are two lines over the same stretch
rather than a geographic detour. `01_Track` is the degenerate case: two paths
forming a ring, every alternate slot null.

### Control points

```c
struct SplinePt {             // 0x70 bytes
    float pos[4];             // +0x00 control point; w is 0, set to 1 at load
    float tangent[4];         // +0x10 unit, along the path
    float down[4];            // +0x20 unit, into the surface
    float lateral[4];         // +0x30 unit, across the path
    float unk_0x40;          // +0x40 also a "no sample here" sentinel at runtime
    float half_width_left;    // +0x44
    float half_width_right;   // +0x48
    float ai_bound_left;      // +0x4c clamped to <= racing_line - 0.1
    float ai_bound_right;     // +0x50 clamped to >= racing_line + 0.1
    float racing_line;        // +0x54 lateral offset, clamped into the width
    u32   unk_0x58;           // 0x10 in every point sampled
    u8    unk_0x5c;           // 0xff; defaulted to 0xff before version 0x105
    u8    unk_0x5d;           // 0x00; defaulted to 0 before version 0x105
    u8    unk_0x5e[2];
    u8    section_id;         // +0x60 links to the `section` node index
    u8    flags;              // +0x61 OR-accumulated across four control points
    u8    unk_0x62[5];
    u8    dist_to_prev;       // +0x67 overwritten at load
    u8    unk_0x68[8];        // zeroed at load before version 0x104
};
```

Every point carries a **full orientation frame**, a track width, an explicit
**racing line**, and an **AI corridor** around it. The frame is orthonormal:
checked on all 34,261 control points, every axis unit length to 1e-3 and tangent
perpendicular to down.

**The `+0x20` axis points down, not up.** On level ground it is exactly
`(0, -1, 0)`, and `+y` is world up — the `Start Position` bind
(`0x08926ae8`) forces the second row of the grid transform to `(0, 1, 0)` and
re-orthonormalises the other two around it. The same conclusion falls out of the
degenerate-frame branch in the load pass, which derives `+0x30` as
`normalize(cross(tangent, (0,1,0)))` and `+0x20` as
`normalize(cross(tangent, +0x30))`; for a tangent along `+x` that is exactly
`(0,-1,0)`, which is what the shipped data holds. The constant `(0,1,0)` is at
`0x08a909d0`.

Getting that sign wrong matters, because of the next part.

**The load pass lifts every control point.** `AiTrack_LoadPathPoints`
(`0x0887eba8`) does `pos -= 3.0 * down`, so the running game's spline sits 3
units above the surface line the exporter wrote. Checked against `01_Track`: the
control point nearest the start line is at `y = 0.0` on disc and `y = 3.0` after
the lift, and the `Start Position` transform sits at `y = 2.42` between them.
Calling `+0x20` "up" and then subtracting along it would push the racing line
*into* the track.

Confidence **92** for the frame semantics and the lift, from the decompilation
plus the geometric agreement above.

The same pass clamps the racing line into the track width and forces the
corridor bounds to straddle it by at least 0.1. **The shipped data already
satisfies both**, on all 34,261 points, so the clamps are defensive rather than
corrective and the fields can be used as stored.

### The curve

A **uniform cubic B-spline**, basis
`[(1-t)^3, 3t^3-6t^2+4, -3t^3+3t^2+3t+1, t^3] / 6`, blending all four control
points' vectors *and* scalars. Confidence **90**: the basis constants are
visible as VFPU immediates, and the implementation's segments join up, but the
evaluator has not been read instruction by instruction.

So width, racing line and corridor are interpolated along the curve too, not
just position. A B-spline does not pass through its control points, which is
worth remembering when comparing a sampled position against the file.

### Topology: a graph, not a ring

Stepping within a path is `±0x70`. At a path end, the cursor hops through a
junction:

```c
if (point < path->point_count - 1) {
    advance within the path;
} else {
    Path *next = path->exit->next_primary;
    if (path->exit->next_alternate && index(next) == excluded_path)
        next = path->exit->next_alternate;   // branch selection
    move to next->points[0];
}
```

**Shortcuts are alternate paths out of a junction**, chosen by an
`excluded_path` argument threaded down from the caller. A ring track is just the
degenerate case where every junction has one predecessor and one successor.

Confidence **88** for the traversal.

## Locating a ship on the track

`AiTrack_LocatePosition` (`0x0887ce78`), 15 callers, over
`AiTrack_UpdateCursor` (`0x0887e464`). It is **both** a spatial query and an
incremental advance:

1. **A per-caller cursor**, `{track, path_index, point_index, point}`. Each ship
   and the camera owns one.
2. **A spatial hash**: position quantised, hashed into a 128-bucket table, each
   bucket caching three `(path, point)` candidates with an LRU stamp. The table
   is inside the track object and cleared 128 entries of `0x18` bytes at a time
   by `0x0887c520`.
3. **On a miss**, a brute-force scan of every path, keeping the three nearest,
   written back into the bucket.
4. **Refinement** walks each candidate to its best point — and if a candidate
   path matches last frame's, it seeds the walk from last frame's point rather
   than the hashed one. That is the incremental fast path.
5. **Filters** exclude a path or restrict to one, which is how branches are
   disambiguated.
6. **Self-correction**: if the result lands more than 100 units away, it
   recurses with the hash bypassed.

The section index then comes from the spline point, not from a geometric query:
evaluating the spline writes `section_id` and OR-accumulates `flags` across the
four control points.

Confidence **90**.

**Sections are attached to the spline.** There is no point-in-volume test.

### A located sample is a `SplinePt`, and the physics reads one

`AiTrack_LocatePosition` does not return an index. It fills in a **`0x70`-byte
record in the caller's memory, in this same layout**, and a ship keeps two of
them: `entity+0xaf0` for where it is and `entity+0xb60` for the neighbour. That
is worth stating here because it is the only place the struct above is confirmed
from the *consumer* side rather than from the loader, and because one physics
mechanism depends on the `+0x20` axis being what this page says it is.

- The writers are `FUN_08842a18` (`0x08842ae8`-`0x08842b00`, `a1 = entity+0xaf0`
  and `t1 = entity+0xb60`) and `FUN_0883ff6c` (`0x0883ffe4`), which then reads
  `+0x00` through `+0x60` back out of its own buffer in `lv.q` steps.
- The ship-entity constructor `FUN_08840c74` initialises both records
  (`0x08840dc0`-`0x08840e3c`) field by field in exactly this layout: four `vec4`,
  six floats at `+0x40`-`+0x54`, two bytes at `+0x60`/`+0x61`.
- **`+0x40` doubles as a sentinel.** `AiTrack_UpdateCursor` (`0x0887e464`) writes
  `-1024.0` into it at `0x0887e9f8`-`0x0887ea00` when it has no sample to report,
  and `Ship_UpdateMagLock` skips the second record when it reads that. So the
  field is a slot the runtime reuses, whatever the exporter meant by it; the open
  question below stands for the *file*, not for the running game. (The entity
  constructor merely zero-fills both records.)
- The consumer that pins `+0x20`'s direction is the magstrip attitude hold, which
  slaves the ship's up axis to `unit(-(sample+0x20))` and recovers the surface
  point with `sample+0x00 - 3.0 * that` - the exact inverse of this page's
  `pos -= 3.0 * down` lift, which only closes if `+0x20` points into the surface.
  See
  [engine.md](../ghidra/functions/psp-pulse/engine.md#0xb10-is-splineptdown-and-the-whole-record-is-a-located-spline-sample).

## `Start Position`

**Status: decoded, and the spawn path uses it.**
[`oag_formats::track::start_position`](../../crates/formats/src/track.rs) reads
it and `oag_gameplay::spawn::Pose::from_start_position` places a ship on it.

Grid slots are **named resources**, not an array. The bind formats
`"start position %d"` and publishes the node's transform into the resource
dictionary, re-orthonormalised with up forced to `(0,1,0)`.

The payload is 64 bytes: a row-major 4x4 matrix. On `01_Track` it is identity
rotation with translation `(-700.57, 2.42, 144.52)`. That the second row is the
up axis, forced to `(0,1,0)`, is what fixes the world's handedness for everything
else on this page. Confidence **92**.

### The rows are the craft's own left-up-forward basis

`cross(row0, row1) = row2` holds **exactly on all 40 PSP track files** — the
same identity the recorded craft basis satisfies on 200 of 200 ticks (see
[engine.md](../ghidra/functions/psp-pulse/engine.md)). Row 1 is up, so rows 0 and
2 are the remaining two axes in the same order the craft carries them, and the
data agrees axis for axis with the track's own frame:

| Against the nearest spline sample | 40 of 40 files |
| --- | --- |
| `dot(row2, tangent)` | `+1.000` — row 2 is forward |
| `dot(row1, -down)` | `+1.000` — row 1 is up |
| `dot(row0, lateral)` | `-1.000` — row 0 is **left** |

That last row settles an open question this page used to carry: **`lateral`
points to the driver's right.** The widths are stored as
`[-half_width_left, +half_width_right]` along it, so the field names are correct
as written rather than merely internally consistent. Confidence **90**, from the
40-file agreement plus the runtime leg below.

**And the bind's fix-up is real, if small.** The authored up row is already
exactly `(0,1,0)` on 31 of the 40 files and off it by at most **1.600 degrees**
on the other nine, so it is not the no-op an earlier edition of this page called
it. Which of the remaining two rows the original preserves while
re-orthonormalising **was not read**; that 1.600-degree bound is what makes the
question a bounded uncertainty rather than an open one, and the crate keeps
forward.

### It is one grid slot, and it is not pole

Every one of the 40 files carries **exactly one** `Start Position` node, so a
grid of eight is not authored: seven slots come from code that has not been read.
The one that is authored is **not** on the centreline — it sits between **3.24
and 20.48 units** off it on every track, always inside the track's own
half-widths. A start-line marker would be on the line; a grid slot is not.

The runtime leg comes from the reference capture, and it is the **heading** that
carries it. On `16_Track` the authored slot's forward is within **1.12 degrees**
of the craft's own forward at the start of a captured time trial, and its left
within **1.54** of the craft's left. Read the rows in any other order and those
are tens of degrees out.

The *positions* differ, and by a lot: the captured start pose is 139.7 units
away, decomposing in the slot's own frame into **137.9 units ahead, 22.4 to its
left and 0.8 up**. Both are ~10 units off the centreline, on opposite sides.
Those three numbers are what any account of the grid has to explain. **This page
does not offer one** — which slot the authored node is, and what lays out the
other seven, are not determined, and nothing in the crate derives them.

#### One unexplained coincidence, recorded rather than resolved

`track_reversed.vex`'s authored slot sits **2.2 units** from where the original's
craft starts on the *forward* layout, with `y` agreeing to **0.0096**, and it
stands **4.011 units** above its own collision surface against a resting craft
height measured at 4.002–4.009. Three quantities agreeing, where `track.vex`'s
own slot is 139.7 units away and 2.698 above its surface.

It is not the layout the capture is on, and that is measured rather than argued:
the recorded craft faces **along** `track.vex`'s spline tangent at `+0.9999` and
against the reversed variant's at `−0.9999`, on 170 of 170 clean ticks of both
captures (`the_captured_start_pose_faces_along_the_forward_layout`). Worth
running, because the collision geometry is shared between the two variants — the
[M3 track identification](../overview/roadmap.md#m3---verification-harness) cast
positions against *geometry* and so could not have separated them.

The reading that fits is that the two layouts' grids bracket one shared start
line, each set back in its own direction, so the reversed grid's authored slot
lands near the forward grid's front row. That is a reading, not a finding; the
three agreeing numbers are the finding, and they are here so the next person
meets them with the refutation attached.

### The authored `y` is not a ride height

It looks like one and is not. Cast straight down onto each track's own collision
mesh, the slot sits anywhere from **1.03 to 7.36 units** above the surface
beneath it (`02_Track` and `06_Track` are the extremes; `01_Track`'s 2.42 is
mid-range). A ship started a fixed height above the authored value is therefore
out of its own hover probes' reach on some tracks and inside the floor on others.

So the slot supplies **where a ship stands and which way it points**, and the
height comes off the collision geometry — `spawn_height` above the surface, the
same quantity everywhere else. One corroboration falls out of doing it that way:
at `16_Track`'s slot the collision surface and the spline's own surface line
agree to within **0.01 units**, which nothing arranged.

How a ship is assigned a slot number is still **not** determined.

Every claim in this section is asserted in
`crates/formats/tests/track_ground_truth.rs` and
`crates/game/tests/race_ground_truth.rs`, against the disc rather than in prose.

## Pads

**Moved to [`pads.md`](pads.md)**, which now carries the layout, the census
across all 40 track files and the reimplementation. The short version, and the
two corrections to what this section used to say:

`Speedup Pad` is a **`Mesh` subclass** - its bind handler calls the `Mesh` bind
first - so its payload is a mesh payload and the box at `+0x10`/`+0x20` is the
mesh's own bounding-box pair, two `vec4`s rather than a bare AABB. It is expanded
vertically at load (`min.y -= 2.0`, `max.y += 8.0`); the expansion is asymmetric
in `+y`, which is consistent with `+y` being up and a pad being triggered from
above. `Weapon Pad` shares the base vtable and decodes identically. Confidence
**85**.

The **eight zeroed words are not on the payload and are not trigger latches**.
They sit at object `+0x1d0`, are zeroed at bind, and hold each racer's cached
distance to this pad, decremented by how far that racer moved - an optimisation
so the containment test only runs once a craft could have reached the pad. See
[`pads.md`](../ghidra/functions/psp-pulse/pads.md) for `Pad_SweptTest_q`.

## Open questions

### Where is lap counting?

**`gate` has no runtime class registration.** All 46 callers of the class
registrar were enumerated and none passes `0x3ca`. There is no bind handler, so
nothing reads a `gate` payload.

No lap or split logic was found. Every `lap`-matching string in the binary is a
HUD label, a save key or a music cue.

The plausible reading is that lap counting rides on `SplinePt.flags` (`+0x61`),
which is OR-accumulated and surfaced by the spline evaluator. **This was not
verified**, and the consumer of that byte was not found. It is worth noting that
`flags` is `0` on every control point of `01_Track`, so whatever sets it is
either rare or authored per-track. Treat `gate` as an authoring-time construct
until something proves otherwise.

### Not determined

- The `section` payload's `pad[6]` and its trailing variable-length data.
- What reads the reserved block, and what `Path.max_spacing` is used for.
- `SplinePt` `+0x40` (what the *file* means by it - the runtime reuses the slot
  as a sentinel, see above), `+0x58` (always `0x10`), `+0x5c`-`+0x5f`,
  `+0x62`-`+0x66`, `+0x68`-`+0x6f`.
- ~~Which lateral direction is "left".~~ **Answered**: `lateral` points to the
  driver's **right**. `Start Position`'s row 0 is the craft's left axis and
  `dot(row0, lateral)` is `-1.000` on all 40 track files, so the widths stored as
  `[-half_width_left, +half_width_right]` mean what their names say rather than
  only being internally consistent. See [`Start Position`](#start-position).
- How a ship gets its grid slot. **Narrowed**: exactly one slot is authored per
  track and it is neither the centreline nor pole, so the other seven are laid
  out by unread code rather than by data.
- **Almost nothing here has been verified under an emulator.** Everything above
  is static reading plus agreement with shipped data, which is what caps the
  scores at 94. The exception is `Start Position`'s frame, whose heading now
  agrees with the original's own craft at the start line to 1.12 degrees.
