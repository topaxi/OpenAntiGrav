# Track data

**Status: decoded and validated.** The `WO Track` spline graph parses on all 40
track files on the PSP disc with **zero bytes unaccounted for**, and is
implemented in [`oag_vex::track`](../../crates/vex/src/track.rs). The
visibility `section` payload is now decoded too, in
[`oag_vex::pvs`](../../crates/vex/src/pvs.rs), and validated against
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

### White and Black

**There is no fixed rule.** Which of a circuit's two Track Select entries is
titled White and which Black depends on the circuit: see the table. Maintainer ruling, 2026-10-03: *align with the original*; the authors
may have relabelled tracks, so this build reads the disc and invents no
convention. Earlier lanes inferred a colour from `Definition.xml`'s entry order, and a
later one from a "White = forward" rule; neither is evidence, and the second is
wrong for five of the twelve circuits.

**Mechanism.** An entry's display title is its `PI_Track` id looked up in the
language's `entries.xml` (`PI008`-`PI012` carry the same English text), ending in
"White" or "Black". The entry's `location` and `Reversed` flag pick the file:
`NN_Track\track.vex`, or `track_reversed.vex` when `Reversed="True"`. This build
does the same - `oag_raceplay::catalogue::label` and `Track::entry_name` - so our
Track Select shows the disc's titles and starts the layout the original starts.
`crates/game/tests/pulse_variant_titles_ground_truth.rs` pins the table below on
both PSP discs (colour words only; the titles stay on the disc).

| Circuit | White entry | White loads | Black entry | Black loads | Confidence |
| --- | --- | --- | --- | --- | ---: |
| Talon's Junction | `16_Track` | `16` forward | `32_Track` | `16` reversed | 85 |
| Moa Therma | `03_Track` | `03` forward | `19_Track` | `03` reversed | 85 |
| Metropia | `18_Track` | **`02` reversed** | `02_Track` | **`02` forward** | 95 (White live) |
| Arc Prime | `10_Track` | `10` forward | `26_Track` | `10` reversed | 85 |
| de Konstruct | `21_Track` | **`05` reversed** | `05_Track` | **`05` forward** | 95 (both live) |
| Tech de Ra | `04_Track` | `04` forward | `20_Track` | `04` reversed | 85 |
| The Amphiseum | `25_Track` | **`09` reversed** | `09_Track` | **`09` forward** | 85 |
| Fort Gale | `30_Track` | **`14` reversed** | `14_Track` | **`14` forward** | 85 |
| Basilico | `17_Track` | **`01` reversed** | `01_Track` | **`01` forward** | 95 (both live) |
| Platinum Rush | `13_Track` | `13` forward | `29_Track` | `13` reversed | 85 |
| Vertica | `06_Track` | `06` forward | `22_Track` | `06` reversed | 85 |
| Outpost 7 | `07_Track` | `07` forward | `23_Track` | `07` reversed | 85 |

Five circuits (Metropia, de Konstruct, The Amphiseum, Fort Gale, Basilico) have
White on the reversed layout and Black on the forward one; the other seven have
the opposite. Nothing in the circuit's number or in the entry's position predicts the group,
so the table is the only source.

**Evidence.** The disc rows come from reading all 24 entries off
`pulse-psp-usa.chd` and `pulse-psp-eu.chd` (identical on both; raw dump was not kept). Live rows, PPSSPP 1.20.4,
2026-10-03, dev-unlock byte, Time Trial, VENOM, the Track Select entry started
by its on-screen title (screenshots beside the dump), craft position once the
countdown ended, against our own start on each layout
(`--race --trace-out`, tick 0):

| Entry started | Original start | Ours, forward | Ours, reversed | Matches |
| --- | --- | --- | --- | --- |
| Basilico Black | (-721.16, 4.01, 282.58) | (-721.16, 4.00, 282.63) | (-701.21, 4.00, 144.44) | forward |
| Basilico White | (-701.21, 4.01, 141.55) | | | reversed |
| de Konstruct Black | (-39.86, -16.40, -193.97) | (-39.57, -16.40, -191.02) | (20.05, -17.40, -67.61) | forward |
| de Konstruct White | (21.98, -17.48, -63.71) | | | reversed |
| Metropia White | (533.28, -12.91, 169.66) | (394.09, -13.38, 192.40) | (529.36, -12.93, 169.73) | reversed |

Every live row lands on the layout the disc table names; Metropia White is the
discriminating one among the three circuits a fresh profile offers, because there
White is *not* the forward layout. The live starts sit within 4 units of ours
(the countdown settle) against 20 to 140 for the wrong layout. Each de Konstruct row was started on two
separate boots and agreed. Confidence 95 for a row with a live start, 85 for a
row read off the disc alone: the mechanism (title, then `Reversed`, then file)
is measured on five entries covering both orders.

### Not every circuit has the zone pair, and the entry says which do

`availableInZone="true"` above is **load-correctness, not menu decoration**: a
circuit that does not declare it carries no `zone_track.vex` at all, so a Zone
race on one asks the archive for a name that hashes to nothing.

Measured by probing all 24 entries by name against `pulse-psp-usa.chd`: the
attribute predicts the file **24 times out of 24**, sixteen present and eight
absent, with no exception in either direction. `pulse-ps2-eu.chd` declares 32
entries and the same biconditional holds there, 22 present and 10 absent.
Confidence **94** - direct name resolution over the whole set.
`crates/game/tests/zone_ground_truth.rs` is that sweep, and
`oag_raceplay::catalogue::Track::available_in_zone` is where it is read.

**A zone circuit is a different environment, not a filtered view of the race
one.** `16_Track` decodes to 602 meshes / 163,178 triangles and its
`zone_track.vex` to 564 / 154,538, with its own lights, its own `fogCube` and its
own `Skycube` - and the sky is the tell, because the twelve one-material skies
[`skycube.md`](skycube.md) counts on the disc are exactly the zone variants
against five or six materials elsewhere.

**The racing line is usually but not always shared.** Comparing the two splines
across all sixteen zone-available circuits: fourteen have identical control-point
counts and two do not - `10_Track` (844 race / 848 zone) and its reversed twin
`26_Track` (847 / 852), which are the same environment. So a zone circuit's
`WO Track` is authored separately rather than copied, and one environment's
differs slightly. See [race modes](../gameplay/race-modes.md) for what that costs.

**One circuit authors no `Weapon Pad` node at all** where its race twin does -
which is the data agreeing with Zone having weapons off, and is the case that
found a real bug in `oag_mesh::mesh::build_optional_class`: it returned an
empty model for an *unrecovered class id* but errored for a class the file simply
does not author, so a Zone race failed to load with `decoded to no triangles`
while the file parsed perfectly.

**The selector inside the executable is unread.** The `%s\%strack%s.vex` template
is recovered and its first `%s` is where `zone_` goes, but the branch that puts
it there - the analogue of `Ship_LoadModel`'s `case 6` for the Zone hull, see
[zone-mode.md](../ghidra/functions/psp-pulse-usa/zone-mode.md) - has not been
found. Everything above is shipped data, which is what caps it at 94.

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

Decoded by [`oag_vex::pvs`](../../crates/vex/src/pvs.rs) and validated
against every track file on both Pulse discs by
[`pvs_ground_truth.rs`](../../crates/vex/tests/pvs_ground_truth.rs): 2,268
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
[`oag_vex::pvs::governing_sections`](../../crates/vex/src/pvs.rs).

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
([`track_ground_truth.rs`](../../crates/vex/tests/track_ground_truth.rs))
rather than a claim, and [`AiTrack::encoded_len`] exists so anything else can
run the same check.

[`AiTrack::encoded_len`]: ../../crates/vex/src/track.rs

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
Confidence **92**: an exact numerical match on 86 independent samples - and the
reason it is exact is that the running game **recomputes** it: `AiTrack_ComputeLength`
(`0x0887dba0`) overwrites the field with the measured maximum gap at load (a
reset of out-of-range values to `0.5` sits before that write and is dead). The
exporter wrote the same number the loader would.

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
    float progress;           // +0x40 normalised arc position round the circuit, 0..1; -1024.0 = "no sample" at runtime
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
    u8    light_scale[4];     // +0x62 hull light scales, 2026-09-23 - see below
    u8    unk_0x66;
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

**The prototype Ghidra shows is wrong, and the call sites say what it is** (2026-10-02,
`pulse-grid-walk`): `AiTrack_LocatePosition(float radius /* f12 */, track /* a0 */, record
/* a1, 0x70 bytes out */, position /* a2 */, cursor /* a3, 0 for a fresh one */, excluded_path
/* t0, -1 */, force_scan /* t1 */)`. The grid layout passes the same buffer as position and as
record, so the call overwrites the point it was given.

**What it computes is a projection** (confidence 88): the nearest control point, the segment
between it and its nearer neighbour, then three Gauss-Newton steps on the cubic B-spline of the
**lifted** control points from `t = 0.5` (`FUN_0887c340`), clamped to `0..1` last, and the record
blended at that `t` (`FUN_0887c7e8`). **Every field of the record comes out `8190/8192` of the true
blend**: the evaluator multiplies its weights by the half-float immediate `0x3155`
(`0.16662598`, not `1/6`), so the `w` lane of a located position reads `0.999755859375`, the
tangent is `0.99976` long and the position is `0.000244` of its coordinate toward the world
origin. See [grid.md](../ghidra/functions/psp-pulse-usa/grid.md#the-grid-walk-read-to-the-end-2026-10-02);
`oag_gameplay::grid_walk::locate` is the port.

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
- **`+0x40` is the lap-progress parameter, and doubles as a sentinel.** In the
  file it is the authored **normalised arc position** round the circuit, rising
  from `0.0` to just under `1.0` across the ring (on `16_Track`: `0.0` at path
  1's first point, `0.9988` at path 0's last, and each step times the track
  length matches the point spacing to a mean 0.16 units). The Catmull-Rom
  evaluator interpolates it like every other field, taking the maximum of the
  four control points when they straddle the wrap; `AiTrack_ComputeLength`
  (`0x0887dba0`) derives the track's units-per-`t` from it at load, and the
  lap counter multiplies the two to get an arc length
  ([race-progress.md](../ghidra/functions/psp-pulse-usa/race-progress.md)).
  `AiTrack_UpdateCursor` (`0x0887e464`) writes `-1024.0` into it at
  `0x0887e9f8`-`0x0887ea00` when it has no sample to report, and
  `Ship_UpdateMagLock` skips the second record when it reads that. (The entity
  constructor merely zero-fills both records.)
- The consumer that pins `+0x20`'s direction is the magstrip attitude hold, which
  slaves the ship's up axis to `unit(-(sample+0x20))` and recovers the surface
  point with `sample+0x00 - 3.0 * that` - the exact inverse of this page's
  `pos -= 3.0 * down` lift, which only closes if `+0x20` points into the surface.
  See
  [engine.md](../ghidra/functions/psp-pulse-usa/engine.md#0xb10-is-splineptdown-and-the-whole-record-is-a-located-spline-sample).

## `Start Position`

**Status: decoded, and the spawn path uses it.**
[`oag_vex::track::start_position`](../../crates/vex/src/track.rs) reads
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
[engine.md](../ghidra/functions/psp-pulse-usa/engine.md)). Row 1 is up, so rows 0 and
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
`crates/vex/tests/track_ground_truth.rs` and
`crates/game/tests/race_ground_truth.rs`, against the disc rather than in prose.

### The heading is authored, and on Wipeout HD it is sometimes stale

The slot's heading is the value to prefer — where a craft points on the grid is
authored deliberately, where a spline tangent is this project's own resampling
of a curve. That holds on the PSP and does not always hold on the PS3.

Every shipped circuit file on all four discs in hand was measured: the authored
forward against the tangent of the nearest resampled point to the slot.

| source | files | agree | stale |
| --- | ---: | ---: | ---: |
| `pulse-psp-eu.chd` | 24 | 24 (`+0.925` to `+1.000`) | 0 |
| `pure-psp-eu.chd` | 8 | 8 (`+0.988` to `+1.000`) | 0 |
| `pulse-ps2-eu.chd` | 32 | 31 (`+0.920` to `+1.000`) | **1** (`-0.441`) |
| `hdfury-ps3-eu-dec.iso` | 28 | 19 (`+0.999` to `+1.000`) | **9** (`-1.000`, one `0.000`) |

HD's nine are `01_vineta_k`, `04_chenghou_project`, `05_ubermall`,
`10_sebenco_climb`, `12_sol_2` and `15_anulpha_pass` reversed,
`modesto_heights` reversed, `tech_de_ra` reversed, and **`zone_3` forward** — so
it is stale authoring rather than a rule about reversed circuits. On eight of
them the node's rotation rows are byte-identical to the forward file's and only
the position row moved: the slot was dragged to the other end of the track and
never turned round. `tech_de_ra` reversed carries a bare identity matrix, which
is a slot nobody authored at all.

A craft on one of those faces backwards down its own circuit, and so does the
whole field: the grid is walked along the spline in the base slot's forward
direction, so a reversed base lays the field out ahead of pole instead of behind
it.

### The PS2's one stale slot is what corroborates the correction

`09_Track` is the same circuit on the PSP and PS2 pressings, and its slot has a
**byte-identical position** on both — `(-506.0834, 2.1623526, 242.45824)`. The
rotation is not identical: the PSP's forward row runs straight down the track
and the PS2's is 116 degrees off it. One slot exported twice, one rotation
maintained and one not, on a disc four years older than HD's — so this is a
property of the pipeline rather than of the HD era.

It is also the only case where the right answer exists in another pressing, and
that makes it checkable: the heading the correction substitutes on the PS2 file
agrees with the PSP file's **authored** one to within **0.08 degrees**. Nothing
about that comparison went into deriving the rule.

### The rule

`oag_raceplay::spawn` takes the heading from the spline **only where the two
disagree** — `dot <= 0.5`, a threshold across an empty band: all 92 slots
measured either agree at `+0.920` or better, or disagree at `0.000` or worse.
The tangent rather than the negated slot, because the slot is not "backwards"
but *unmaintained*, and nothing says the next unmaintained one is exactly 180
degrees out. The substituted tangent is **levelled** first, because the bind
forces the up row to world `(0, 1, 0)` and so every authored heading is exactly
horizontal; taking a raw tangent would put those craft in a frame the bind
cannot produce. Position, height and the grid's own layout are untouched, and
no track on either PSP disc reaches the branch — so no Pulse trace, hash or
capture comparison moves by it.

Asserted against all four discs in
`crates/game/tests/spawn_heading_ground_truth.rs`, which pins the outcome, the
census of which files needed the correction, the levelness, and the PSP/PS2
cross-check above.

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
[`pads.md`](../ghidra/functions/psp-pulse-usa/pads.md) for `Pad_SweptTest`.

## Wipeout HD authors version `0x106`, and the control point is unchanged

**Confidence 90**, corpus-wide across all 28 of HD's circuits; the survey is
[hd-status](hd-status.md). Read big-endian, the whole payload above decodes with
nothing moved: magic `WOtd`, the `0x20` header, the `0x20` reserved block, paths
at `0x20` bytes, junctions at `0x10` with `0x7fffffff` for null, control points
at `0x70`. On every file the bytes after the arrays divide by `0x70` exactly and
equal the summed path lengths, and **0 of 71,622 frame vectors** are off unit
length - three per point, which is what says the field *offsets* are right rather
than only the stride.

Two things follow that are worth having on this page rather than only on that one.

**The unread fields are unread in both games.** `+0x5c` and the bytes from
`+0x62` are in the "Not determined" list below; HD authors them too, and
**per track rather than per title** - `+0x5c` is filled on all 776 points of
HD's `02_track` and on none of `talons_junction`, exactly as Pulse fills it on
all 862 of `16_Track` and on 30 of 713 of `06_Track`. So a reader comparing the
two discs will find a difference there and it is authoring, not format.

**`racing_line` is zero on both.** A first pass on the HD disc concluded that HD
authors no racing line; it is zero on all 862 points of Pulse's own `16_Track`
too, and on `01`, `05`, `06`, `13` and `14`. The field is simply unauthored on
the circuits anyone has looked at, in either game.

### The 64-bit `section` mask has to be read as one quantity

[`oag_vex::pvs`](#section-visibility-not-geometry) reads the visibility mask
as `pvs_mask_lo` at `+0x08` and `pvs_mask_hi` at `+0x0c`. On a little-endian
file that is the same thing as reading the eight bytes as one `u64`. **On a
big-endian file it is not**, and a byte-order pass that swaps each word where it
stands leaves the halves the wrong way round.

Measured over 3,536 set bits in 526 `section` nodes on 24 HD circuits, counting
bits that name a section the file does not declare: **22.2 %** reading one
big-endian `u64`, **55.6 %** reading the two words swapped in place, and 100 %
dangling on 15 of the 24 files under the second. The same script's control run
over Pulse's own 24 circuits - where the two readings are by construction
identical - gives **2.10 %**, reproducing the "one bit in fifty" figure the
ground-truth test already records. The remaining 22 % is three circuits whose
section IDs run to the 64-bit cap, not a spread.

This is the only place found so far where a mechanically correct per-field byte
swap produces plausible garbage instead of an error.

## Wipeout 2048 authors version `0x107`, and the control point shrank to 96 bytes

**Confidence 95** for the stride and the five floats, **90** for `section_id`,
**92** for `flags`, **80** for `racing_line`. Measured on
`data/extracted/vita/PCSF00007`, the EU base and DLC PSARCs, with
`crates/game/examples/vita_rosetta.rs` as the reproducer.

Everything above the control point is unchanged: magic `dtOW` little-endian, the
`0x20` header, the `0x20` reserved block, paths at `0x20` bytes, junctions at
`0x10`. Only the control point moved.

```c
struct SplinePt_0x107 {       // 0x60 bytes
    float pos[4];             // +0x00  unchanged
    float tangent[4];         // +0x10  unchanged
    float down[4];            // +0x20  unchanged
    float lateral[4];         // +0x30  unchanged
    float unk_0x40;           // +0x40
    float half_width_left;    // +0x44  unchanged
    float half_width_right;   // +0x48  unchanged
    float ai_bound_left;      // +0x4c  unchanged
    float ai_bound_right;     // +0x50  unchanged
    float racing_line;        // +0x54  unchanged
    u8    unk_0x58[2];        // +0x58  varies per point, unplaced
    u8    unk_0x5a;           // +0x5a  0xff on every point measured
    u8    unk_0x5b;           // +0x5b  0x00 on every point measured
    u8    section_id;         // +0x5c  was +0x60; 0xff when the track has none
    u8    flags;              // +0x5d  was +0x61
    u8    unk_0x5e[2];        // +0x5e  unplaced
};
```

### Why this is measured rather than reasoned

**Wipeout 2048's DLC re-ships twelve Wipeout HD circuits, and the splines are
the same splines.** `Vineta_K`, `Ubermall`, `Sebenco_Climb`, `Sol_2`,
`amphiseum`, `modesto_heights`, `talons_junction`, `tech_de_ra` and `zone_1`
through `zone_4` appear in `dlc1.psarc`/`dlc2.psarc` beside HD's own copies in
`DATA00.PSARC`/`DATA02.PSARC`. On all twelve the path count, the junction count
and **every per-path control-point count** are identical, so control point `k`
of path `i` is the same point in both files and HD's fully decoded `0x106`
record is ground truth for 2048's `0x107` one. That is 10,165 paired control
points.

**Word by word, the two records agree up to `+0x58`.** Comparing 2048's
little-endian word at each offset against HD's big-endian word at the same
offset, agreement runs 10,127/10,165 at `+0x00` down to 6,895/10,165 at `+0x48`
- the shortfall being points 2048 re-exported with slightly different values -
and then falls off a cliff to **0/10,165 at `+0x58` and 17/10,165 at `+0x5c`**.
So the record is HD's, verbatim, through `racing_line`, with the eight bytes
HD leaves zero at `+0x58` replaced and HD's `+0x60..+0x70` dropped: 112 - 16 =
96.

**The four floats past `lateral` are exact, and where they are not, they are the
same number.** Bit-identical on 6,951 (`half_width_left`), 6,895
(`half_width_right`), 7,485 (`ai_bound_left`) and 7,506 (`ai_bound_right`) of
10,165 points; across every remaining point the **worst relative difference is
0.0002** - the two exporters wrote the same value and rounded it differently.
A wrong offset does not produce that.

**`flags` at `+0x5d` is exact on every point.** HD authors a non-zero `flags` on
173 of the 10,165 paired points; the byte at 2048's `+0x5d` equals it on all
173, and equals zero on all 9,992 of the rest. The `(0x5d, HD flags)` histogram
has exactly two entries, `(0,0)` and `(1,1)`.

**`section_id` at `+0x5c` is confirmed against each file's own scene tree, not
against HD's numbering.** 2048 re-sectioned most of the ports, so the *values*
differ; what does not differ is the count. On every one of the twelve ported
circuits and all fourteen the base package ships, **the number of distinct values at
`+0x5c` equals the number of `section` nodes that file's own `.vex` authors** -
`Vineta_K` 4 and 4, `talons_junction` 18 of 19, `tech_de_ra` 22 and 22, `park`
10 and 10, `altima` 2 and 2 - and a circuit that authors no `section` node at
all (`mall`, `sol`, `Ubermall`, `Sol_2`, `zone_1`..`zone_4`) carries the single
value `0xff` on every point. On the two circuits 2048 did **not** re-section,
`talons_junction` and `tech_de_ra`, the indices at which `+0x5c` changes are
*exactly* the indices at which HD's `section_id` changes - 18 of 18 and 22 of
22, a Jaccard agreement of 1.000 against the ~0.03 every other candidate offset
scores.

One circuit does not close: `cathedral` has 6 distinct `+0x5c` values with a
maximum of 9 against 6 `section` nodes, so at least one id indexes past the
table. Recorded rather than explained.

**`racing_line` at `+0x54` rests on structural continuity, not on a matched
value.** HD authors zero on all 10,165 paired points and 2048 authors zero on
all 28,290 points of the fourteen circuits its base package ships, so the pairing cannot
distinguish this field from any other zero. What it does say is that 2048's
word at `+0x54` equals HD's on 10,161 of 10,165 points, and that the fields
either side of it are confirmed - the field is where the layout says it is,
and it is unauthored, exactly as it is unauthored on the Wipeout HD and Pulse
circuits above.

### The layout reads sanely on the circuits nothing can be paired against

All fourteen circuits in 2048's base package parse - the ten it authors itself
plus the four Wipeout HD circuits it ships there rather than as DLC
(`Anulpha_Pass`, `Chenghou_Project`, `Moa_Therma`, `Vineta_K`) - and
`AiTrack::encoded_len()` lands on the payload length **to the byte** on every
one (`altima` 195,104; `mall` 316,480). Half-widths run 1.77 to 71.84 and are
positive on every point of every track; AI bounds run -59.85 to 59.88; `flags`
is `{0}` or `{0,1}`; `+0x5a`/`+0x5b` are `(0xff, 0x00)` on all 28,290 points.

### What is not placed

`+0x58`/`+0x59` vary per point and per circuit (up to 71 distinct values on
`Sebenco_Climb`) and are not placed. `+0x5a` and `+0x5b` are constant across
every point of every track measured, so nothing distinguishes a field from
padding. `+0x5e`/`+0x5f` carry up to 13 distinct values corpus-wide and are not
placed either. The `WO Track` point reader in `vita-2048-eu-v104/eboot.elf` is
still unnamed and unlocated - none of the above needed it.

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
  Measured rather than merely named: `oag_vex::track_coverage::section_coverage`
  claims everything else, and a corpus sweep over all 2,272 PSP section nodes
  (`crates/vex/tests/payload_coverage_ground_truth.rs`) finds exactly 6
  unclaimed bytes per node - `pad[6]` and nothing more, the trailing name
  region being claimed whole (content, not just reach) since it is already
  the payload's own tail.
- `WO Track` itself has no gap at all under the same instrument -
  `oag_vex::track_coverage::wo_track_coverage` claims all 3,843,824 bytes
  across the 40 PSP files, matching [`AiTrack::encoded_len`](../../crates/vex/src/track.rs)'s
  own "equal to the payload length on every shipped track."
- What reads the reserved block, and what `Path.max_spacing` is used for.
- ~~`SplinePt` `+0x40`~~ - **answered**: the normalised arc position the lap
  counter runs on, see above. Still open: `+0x58` (always `0x10`),
  `+0x5c`-`+0x5f`, `+0x66`, `+0x68`-`+0x6f`.
- ~~`+0x62`-`+0x65`~~ - **answered 2026-09-23**: four light scales,
  ambient, directional and two point-light classes, `0..=255`.
  `Spline_SampleSegment` (`0x0887c7e8`) blends them at a craft with the
  B-spline weights (`(byte * trunc(w * 255)) >> 8`, summed into a `u8`), and
  `Craft_ApplyHullLightScale` copies the result into the hull model, whose
  GE lights they scale. Forced to `0xff` below version `0x103`. On
  `16_Track` 455 of 862 points read `255` and 231 read `127` (the tunnels).
  Read live (the player on the grid carries `250`); see
  [scene-light.md](../ghidra/functions/psp-pulse-usa/scene-light.md).
  Decoded as `oag_vex::track::SplinePoint::light_scale`. The version
  `0x107` short point moves these bytes and is not read.
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
