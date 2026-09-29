# `track_col.col`: Wipeout 2048's collision

Every title before this one keeps its collision geometry inside the track's
[`.vex`](vex.md) file, as nodes of five named classes -
[collision.md](collision.md) is that format. **Wipeout 2048 authors no collision
node instances at all** and ships a separate `track_col.col` beside each
`track.vex`: a k-d tree over one triangle soup, with a surface byte per
triangle.

Read by `oag_vex::kdcol`, validated by
`crates/vex/tests/kdcol_ground_truth.rs` over **all 26 files** the three EU
packages ship. Three scratch probes reproduce the measurements below:
`crates/game/examples/vita_surface.rs` (the Wipeout HD pairing and the winding
check), `vita_colcorpus.rs` (the surface-byte census and the facing statistic)
and `vita_colspline.rs` (the racing-line tests) (14 in the base package, 4 in `dlc1`, 8 in `dlc2`) - 363,646
triangles. The two loaders it is read from are `KdTree_Load` (`0x8118d134`) and
`SimpleMesh_Load` (`0x8118fac8`); see
[track-and-collision-loaders.md](../ghidra/functions/vita-2048-eu-v104/track-and-collision-loaders.md).

## The layout

```c
struct KdTreeCollision {
    char  magic[4];        // "kdtr"
    char  version[4];      // four ASCII hex digits, sscanf-ed and required to be 1
    char  tag[4];          // "----", and again before every section below
    u32   node_stride;     // 24, and the loader cannot handle another value
    u32   node_count;      // N
    KdNode nodes[N];
    char  tag[4];
    u32   leaf_count;      // M
    u16   leaves[M];       // triangle indices; a leaf names a run of these
    char  tag[4];
    f32   tree_bounds[6];  // centre, then half-extent
    char  tag[4];
    SimpleMesh mesh;
    char  tag[4];          // and the file ends, to the byte
};

struct KdNode {            // 24 bytes
    i32   low;             // child index, -1 on a leaf
    i32   high;            // child index, -1 on a leaf
    u32   axis;            // 0/1/2, all-ones on a leaf
    f32   split;           // where on that axis; 0 on a leaf
    u16   triangle_count;  // leaf only, 0 on an internal node
    u16   unknown;         // one value per file, the same on all its nodes: 0x000b on altima
    u32   first_leaf;      // where this leaf's run starts in `leaves`
};

struct SimpleMesh {
    u16   vertex_count;    // V
    f32   positions[3*V];  // world space
    u32   triangle_stride; // 6, and the loader cannot handle another value
    u16   triangle_count;  // T
    u16   indices[3*T];
    u16   surface_count;   // T on every file
    u8    surfaces[T];     // one per triangle - see below
    f32   bounds[6];       // centre, then half-extent
};
```

**Confidence 94** for the layout. Three arithmetic closures carry it, and each
one is a test:

- **The file accounts for every byte.** The reader refuses trailing bytes, and
  all 26 files end exactly on their final `"----"`. A wrong length anywhere
  fails on the first file.
- **The leaf runs tile the leaf index array**, no gap and no overlap, on all 26
  - which is what says a node's `+0x10` low half is a triangle count and its
  `+0x14` is where that run starts, rather than two unrelated words. On
  `altima` the last leaf reads `first = 93,272`, `count = 7`, and the array is
  93,279 long.
- **The stated bounds reproduce the geometry's own** under `centre ± extent`.

## The bounds are a centre and a half-extent

Worth its own heading because the other reading is plausible, and was in this
project's notes for a day. On `altima` the soup states `(-113.495, 333.607,
350.514)` and `(1007.426, 383.553, 1246.650)`; its own vertices span
`(-1120.921, -49.946, -896.136)` to `(893.931, 717.160, 1597.165)`. That is
`centre ± extent` **exactly, on all six numbers**, and nothing like the stated
pair read as a min and a max - the "max" corner would sit inside the geometry
on every axis. The ground-truth test asserts both halves: that centre-plus-
extent reproduces the span, *and* that min-plus-max does not.

## The surface byte

**Six of the nine values are read straight out of the executable.**
`TrackCollision_MeshFromNode` (`0x8126f800`) builds a collision mesh from a
`.vex` collision node and picks one byte from that node's class ID:

| `.vex` class | ID | byte | triangles |
| --- | --- | ---: | ---: |
| `Floor Collision` | `0x3b9` | 2 | 154,045 |
| `Mag Floor Collision` | `0x3e6` | 3 | 6,132 |
| `Wall Collision` | `0x3ba` | 4 | 84,711 |
| `Force Field Collision` | `0x3f2` | 5 | 29,441 |
| `Track Wall Collision` | `0x3ed` | 6 | 55,215 |
| `Reset Collision` | `0x3cd` | 7 | 17,049 |
| *(the default arm)* | | 15 | none shipped |

`Cage Collision` (`0x3e7`) is branched past before a byte is chosen, which
reproduces the behaviour [collision.md](collision.md) already records from
Pulse's loader. `Force Field Collision` is a class only 2048 declares - 2048's
own class-name table pairs `0x3f2` with that string, and the binary holds
exactly seven `"… Collision"` names, one per row above plus the cage.

### The same six fall out of an independent measurement

2048's DLC re-ships twelve Wipeout HD circuits, and HD keeps its collision in
`.vex` nodes whose classes are named. Matching the two titles' collision
triangles by centroid, within one unit, gives **170,744 pairs**:

| byte | HD class it matched | pairs | disagreements |
| ---: | --- | ---: | ---: |
| 2 | `Floor Collision` | 87,115 | 0 |
| 3 | `Mag Floor Collision` | 648 | 0 |
| 4 | `Wall Collision` | 34,731 | 0 |
| 4 | `collision_trackwall` | 7,666 | - |
| 6 | `collision_trackwall` | 35,963 | 0 |
| 7 | `Reset Collision` | 4,621 | 0 |

The one impure row is not an ambiguity: **all 7,666 of it are on `Sebenco_Climb`
and `modesto_heights`**, the two ported circuits that carry no byte 6 at all, so
2048 re-tagged their barrier as an ordinary wall. Both drive `Surface::Wall`
either way, so nothing downstream can tell.

**Winding is HD's**, checked on the same pairs: the signed face normals agree on
**170,572 of 170,744**, and all 172 exceptions are on `modesto_heights`, whose
pairing is the noisiest of the twelve (15,714 triangles against HD's 16,211).
That check exists because a flipped winding is the failure that looks like a
fully-populated collision world a craft still falls through.

## Three bytes no `.vex` class produces

`10`, `11` and `12` appear only on circuits 2048 authored itself, never on a
ported one, and `TrackCollision_MeshFromNode` cannot emit them - so they come
from the offline exporter rather than from a collision node. They are placed as
**drivable floor**, and the evidence is three measurements rather than one:

| | horizontal | under the racing line | triangles |
| --- | ---: | ---: | ---: |
| byte 2, the confirmed floor | 87.6 % | 59.1 % | 154,045 |
| byte 10 | 99.4 % | 91.1 % | 3,089 |
| byte 11 | 99.9 % | 74.8 % | 13,862 |
| byte 12 | 100 % | 47.1 % | 102 |
| byte 4, a confirmed wall | 14.2 % | 5.0 % | 84,711 |

"Under the racing line" is measured against the spline's own authored
half-width, and within two units of the surface the spline was authored on -
which is where a drivable floor is, because
[the spline sits on the track surface](track.md).

**The check that settles it** is what happens to the racing line's ground
coverage. Counting only byte 2, then counting bytes 10, 11 and 12 as well:

| circuit | byte 2 alone | with 10, 11, 12 |
| --- | ---: | ---: |
| `cathedral` | 1.1 % | 47.7 % |
| `park` | 10.5 % | 32.7 % |
| `bridge` | 12.7 % | 27.7 % |
| `tower` | 9.6 % | 27.5 % |
| `square` | 19.0 % | 32.4 % |
| `mall` | 5.4 % | 19.6 % |
| every circuit that uses none of them | unchanged | unchanged |

That last row is the control, and it is what makes this a measurement rather
than a correlation: adding three values to the floor set changes nothing at all
on the twenty circuits that do not carry them. **Confidence 85** for `10` and
`11`.

**`12` is weaker and is not covered by that sentence.** 102 triangles on
`altima` alone, all near-horizontal, adding 0.1 % to that one circuit's
coverage; the racing-line test cannot separate it from a flat piece of scenery.
**Confidence 50.** It is grouped with the other two because the two failures are
a hundred triangles of invisible floor against a hundred triangles of hole, and
the first is the one that fails visibly.

## What is read for the record and not used

The **k-d tree itself**. This engine casts rays against an
`oag_physics::TriangleSoup` of its own and has no use for the original's
acceleration structure, so `oag_vex::kdcol` parses the node array, checks
it, and hands the caller the soup. It is read anyway because a format page that
cannot be checked against the file is a format page that rots - the leaf-tiling
invariant above is what that buys.

The **`unknown` half-word** at each node's `+0x12` is **one value per file**,
the same on every node of it, internal and leaf alike, so nothing here
distinguishes a field from a constant and it is carried rather than named. An
earlier revision of this page said `0x000b` on every node of every file; that
is altima's value and only altima's. Across the ten base-package circuits it is
`0x000b` (altima), `0x0033` (arena, park, square), `0x0045` (bridge), `0x006c`
(cathedral), `0x0084` (mall), `0x005f` (sol), `0x001c` (subway) and `0x003b`
(tower), measured 2026-09-29 - a per-file quantity, plausibly a property of the
tree the exporter built.

## Omega Collection: a 19-byte node, and nothing else moved

The PS4 Omega Collection's 38 `track_col.col` files (its 22 circuits and the
ten `environments2048` ones, forwards and reversed) state a node stride of
**19**, and in every one the next `"----"` sits at exactly `0x14 + 19 * N`.
Everything after the node array is this container unchanged: on
`environments2048\altima` it is **byte-identical** to 2048's own Vita file
(415,286 bytes: leaf indices, bounds, triangle soup, final tag) and the file is
smaller by exactly `5 * 27,199`. The nine other shared circuits were
re-exported (different node counts, or the same count with a different soup),
so they are not comparable byte for byte. **Confidence 85** for the layout:

```c
struct KdNodePacked {      // 19 bytes, no alignment
    i32   low;             // child index, -1 on a leaf
    i32   high;
    u8    axis;            // 0/1/2, 0xff on a leaf
    u16   triangle_count;
    f32   split;
    u32   first_leaf;
};                         // the `unknown` half-word is not stored
```

against 2048's nodes on altima the children, axis, triangle count and leaf
start agree on all 27,199 nodes, and the split on 22,582 exactly and on the
other 4,617 to one ulp (the tree was rebuilt); every one of the 38 files' leaf
runs tiles its leaf array exactly. `oag_vex::kdcol` reads both as
`NodeLayout::Wide` and `NodeLayout::Packed`.
Reproducers: `crates/vex/examples/omega_col_probe.rs` and
`crates/game/tests/omega_race_ground_truth.rs`. See
[omega-status.md](omega-status.md#racing-a-race-starts-on-this-titles-own-data).

## What a race does with it

`oag_vex::kdcol::collision_nodes` groups the soup into the same
`CollisionNode` values the `.vex` path produces, one per surface kind, in
`SurfaceKind::ALL` order - never in the order a map happens to iterate, because
a collider's index *is* its identity to
`oag_physics::forces::Environment::self_collider` and that ordering feeds
simulation state. `oag_game::race::load` asks for the sibling file **only when
the `.vex` answered with no collision at all**, so no other title's load changes
shape.

**The vertices are world space already**, so nothing composes a transform onto
them. That is measured here rather than inherited from the `.vex` path: the
Rosetta above matches 2048's triangles to HD's world-space ones within one unit.
