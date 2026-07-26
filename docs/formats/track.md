# Track data

**Status: structure recovered from static analysis, not implemented.**

Two things that sound like they should be the same are not:

- **`section` nodes are a visibility partition**, not the track path. They carry
  a potentially-visible-set bitmask and a bounding box, and nothing else.
- **The track path lives in the `WO Track` node**, published at runtime under
  the resource name `"AI track data"`. It is a **graph of cubic B-spline
  paths**, with a racing line and AI corridor.

That correction matters. The `.vex` class-ID table lists `section` and `gate`
alongside `Start Position` and the pads, which invites reading them as the lap
structure. They are not.

## Class IDs

Decoded from the table at `0x08ab2370` (stride 12, `{u32 id, char *name, ptr}`,
names at `0x08a84d40`). Confidence **99**: the five collision IDs already known
from an independent reading fell out of the same walk unchanged.

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
    u8  has_bounds;    // selects the optional bbox block
    u8  pad[6];        // never read
    u32 pvs_mask_lo;   // sections 0..31 visible from here
    u32 pvs_mask_hi;   // sections 32..63; own bit OR'd in at load
    // if has_bounds:
    float bbox_min[4];
    float bbox_max[4];
};
```

The load path builds `1u64 << index` and publishes a 64-bit mask per section;
the lookup at `0x0891e908` returns all-ones when the index is out of range. That
is a textbook PVS.

**The engine hard-caps at 64 sections**, from the mask width and a 64-entry
gather buffer. Worth knowing before designing anything that assumes more.

Confidence **90** for the mask, **85** for the bounding box.

## `WO Track`: the spline graph

```c
struct AiTrackHeader {      // 0x20 bytes
    u32       magic;
    u32       version;      // >= 0x100 required; gates at 0x101, 0x103-0x105
    u32       path_count;
    u32       junction_count;
    Path     *paths;        // file: offset, fixed up at load
    Junction *junctions;
};

struct Path {               // 0x20 bytes
    u32       point_count;
    u32       unk04;
    SplinePt *points;       // 0x70 stride
    Junction *entry;        // file: u32 index, 0x7fffffff = null
    Junction *exit;
};

struct Junction {           // 0x10 bytes, a 2-in / 2-out merge-split
    Path *prev_primary;
    Path *prev_alternate;
    Path *next_primary;
    Path *next_alternate;
};

struct SplinePt {           // 0x70 bytes
    float pos[4];             // +0x00 control point
    float tangent[4];         // +0x10 normalised
    float up[4];              // +0x20
    float right[4];           // +0x30
    float half_width_left;    // +0x44
    float half_width_right;   // +0x48
    float ai_bound_left;      // +0x4c clamped to <= racing_line - 0.1
    float ai_bound_right;     // +0x50 clamped to >= racing_line + 0.1
    float racing_line;        // +0x54 lateral offset, clamped into the width
    u8    section_id;         // +0x60 links to the `section` node index
    u8    flags;              // +0x61 OR-accumulated across four control points
    u8    dist_to_prev;       // +0x67 min(255, distance * 20)
};
```

Each point carries a **full orientation frame**, a track width, an explicit
**racing line**, and an **AI corridor** around it. The load pass at `0x0887eba8`
clamps the racing line into the track width and forces the corridor bounds to
straddle it by at least 0.1, so those relationships are invariants rather than
data to trust blindly.

Confidence **90** for the header and path structures, **80-85** for the
`SplinePt` fields.

### The curve

A **uniform cubic B-spline**, basis
`[(1-t)^3, 3t^3-6t^2+4, -3t^3+3t^2+3t+1, t^3] / 6`, blending all four control
points' vectors *and* scalars. Confidence **92**: the basis constants are
visible as VFPU immediates.

So width, racing line and corridor are interpolated along the curve too, not
just position.

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

Confidence **88** for the traversal, **85** for reading slots 0/1 as
predecessors and 2/3 as successors. That ordering is inferred from which slots
each direction reads, not proven.

## Locating a ship on the track

`AiTrack_LocatePosition` (`0x0887ce78`), 15 callers, over
`AiTrack_UpdateCursor` (`0x0887e464`). It is **both** a spatial query and an
incremental advance:

1. **A per-caller cursor**, `{track, path_index, point_index, point}`. Each ship
   and the camera owns one.
2. **A spatial hash**: position quantised, hashed into a 128-bucket table, each
   bucket caching three `(path, point)` candidates with an LRU stamp.
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

## `Start Position`

Grid slots are **named resources**, not an array. The bind formats
`"start position %d"` and publishes the node's transform into the resource
dictionary, re-orthonormalised with up forced to `(0,1,0)`. Consumers look them
up by name.

How a ship is assigned a slot number is **not** determined.

## Pads

`Speedup Pad` carries an AABB at `+0x10`, expanded vertically at load
(`min.y -= 2.0`, `max.y += 8.0`), followed by **eight zeroed words** — one per
racer, almost certainly per-ship trigger latches. `Weapon Pad` shares the base
vtable, so it is very likely identical. Confidence **80**.

## Open questions

### Where is lap counting?

**`gate` has no runtime class registration.** All 46 callers of the class
registrar were enumerated and none passes `0x3ca`. There is no bind handler, so
nothing reads a `gate` payload.

No lap or split logic was found. Every `lap`-matching string in the binary is a
HUD label, a save key or a music cue.

The plausible reading is that lap counting rides on `SplinePt.flags` (`+0x61`),
which is OR-accumulated and surfaced by the spline evaluator. **This was not
verified**, and the consumer of that byte was not found. Treat `gate` as an
authoring-time construct until something proves otherwise.

### Not determined

- The `section` payload's `pad[6]` and its trailing variable-length data.
- `Path+0x04` and `+0x14..0x1f`; `SplinePt` `+0x40`, `+0x58`, `+0x62..0x66`,
  `+0x68..0x6f`.
- How a ship gets its grid slot.
- The junction slot ordering, inferred rather than proven.
- **Nothing here has been validated against a real track file.** Every claim is
  static analysis. Extracting a track `.vex` and parsing it would be the
  cheapest possible confirmation, exactly as it was for mesh geometry, where it
  corrected two errors immediately.
