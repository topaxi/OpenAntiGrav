# Wipeout 2048's scenery animation: `.rcsskeleton`, `.rcsanimclip`, and the model's node table

**2026-09-16.** Wipeout 2048's `track.vex` authors **no `Anim Transform` at
all** - `altima`'s carries 1,834 `Transform`s, 7 `Mesh`es and not one node of
class `0x3c0` - so the mechanism Pulse and HD animate scenery with
([scenery-animation.md](../rendering/scenery-animation.md)) has nothing to
find here. What moves a 2048 circuit is three files that had all been sitting
beside `track.rcsmodel`:

| File | What it holds | Reader |
| --- | --- | --- |
| `track.rcsmodel`, section B header | a **node table** - one id, one name hash and one bind matrix per node - and, per mesh object, **which node its vertices are authored in** | `oag_rcs::rcsmodel::psp2::nodes` |
| `track.rcsskeleton` | the same nodes as a **hierarchy**: parent, scale, rotation, translation, visibility, pivot, pivot translate | `oag_rcs::rcsskeleton` |
| `track.rcsanimclip` | **keys** for a subset of those nodes, 5 a second, absolute local values | `oag_rcs::rcsanimclip` |

All three share the `0xca5caded` container [2048-rcsmodel.md](2048-rcsmodel.md)
describes (`oag_rcs::rcsmodel::psp2::container`, factored out the day the
other two were read). `oag_rcs::rig` evaluates a node under its track, and
`oag_mesh::mesh::rcs::psp2::placement` decides which nodes bake and which
take a slot of the shader's node table - the same table, upload and shader
path Pulse's and HD's `Anim Transform`s already ride
(`oag_mesh::mesh::Motion` is the two-variant enum that lets it).

**The reading rests on a cross-title oracle**, the same method
[track.md](track.md#wipeout-2048-authors-version-0x107-and-the-control-point-shrank-to-96-bytes)
used for the spline: 2048's DLC re-ships twelve Wipeout HD circuits and four
Zone ones, HD's `Anim Transform` decode is validated, and on those circuits
HD's evaluator is ground truth for 2048's rig - node by node, at four times,
`crates/render/tests/psp2_scenery_animation_ground_truth.rs`. What it
established, in order of what a renderer needs:

1. **A node-bound mesh's vertices are in its node's space**, not the world's
   (confidence 92). Before this page `oag_mesh::mesh::rcs::psp2` drew every
   submesh as-is; the 130 node-bound meshes of `altima`'s 1,153 were a median
   1,086 units (max 5,765) from where the original puts them, and every one
   of them was. On Anulpha Pass, 73 of 78 node-bound meshes have a vertex box
   equal to HD's authored box for the same mesh *in its `Anim Transform`'s
   space* to the float; none equals it in world space.
2. **The clip's keys are HD's animation resampled at 5 Hz**, absolute local
   values (confidence 90). `uterring`'s rotation key 1 is `(0, -0.0251, 0,
   0.9997)`, HD's evaluator at frame 12 is `(0, -0.0251, 0, 0.9997)`; every
   key of every node 2048 kept HD's animation for agrees the same way.
3. **The local matrix is `S * T(-pivot) * R * T(pivot) * T(pivot_translate) *
   T(translation)`** - Maya's own with the rotate pivot set and the scale
   pivot not - row-vector order (confidence 88). On Anulpha Pass the pivot
   form lands 76 of 86 shared nodes on HD's time-zero world matrix and the
   bare `S * R * T` lands 63; the pivot translate settles the 7 trains (from
   520 units off to 4). On `altima`, which has no HD twin, it is what puts
   each wind-turbine rotor on its hub (1,469 vs 1,467 world units) where the
   bare form orbits it 2,500 units out
   (`altimas_rotors_sit_on_their_hubs_because_of_the_pivot`). **Scaling
   about the pivot too** - Maya's form when both pivots are set to the same
   point - was the first reading and is wrong: Metropia's `pCylinder208_1`
   (scale 17.3, pivot 17.3 up, no rotation) lands on HD's matrix exactly
   with the scale about the origin and 282 units off with it about the
   pivot. Every node with a unit scale composes the same either way, which
   is why the first reading survived four circuits - and the choice is not
   small elsewhere: 199 meshes over the fourteen race circuits hang under a
   node with both a scale and a pivot, and the two forms put them up to
   11,766 units apart. Three things pick the origin: Metropia's node
   against HD; `altima`'s two boat wakes, which the origin form puts 60 and
   181 units from the river boat they belong to and the pivot form 612 and
   550; and the slot inventory itself, which has one pivot pair (slots 4
   and 5, vec3) and no scale-pivot pair for a scale-about-pivot form to
   read from - Maya's `scalePivot` and `scalePivotTranslate` would need two
   more vec3 slots, and slots 6 to 8 are scalars on 21 nodes in two files.
4. **World matrices compose `local * parent_world`, a root's parent being the
   skeleton's own static matrix for it** (confidence 90). The per-node parent
   array marks a root by its own index, and 31 of `altima`'s 53 roots have a
   non-identity matrix above them - a Maya group the table does not list.
5. **Interpolation is linear between keys, a quaternion renormalised along
   the shorter arc, the segment past the last key blending to key 0, and
   time wrapping on the track's own duration** (confidence 88). Median error
   against HD's world matrices at 0.1, 0.3, 0.6, 1.0, 1.4, 2.5, 3.0, 4.1 and
   7.3 s is 0.01 units with 5 to 7 nodes over 2.0 at each time; stepping to
   the nearest key instead gives 0.2 to 0.9; holding the last key rather
   than wrapping to key 0 gives 0.15 to 0.23 in the last 0.1 s of a 4 s loop
   where wrapping gives 0.001. Slerp and nlerp were indistinguishable to
   three decimals at 0.2 s key spacing, so the cheaper one is used and no
   claim is made between them.

**The cross-title suite's floors were set by mutation, not by taste.** Two
deliberate breaks were run through it - the parent chain dropped, and the
pivot dropped - and each circuit's floor sits between its intact figure and
its broken one, so a floor is one that has been seen to fail: the parent
break fails eight of the twelve circuits, the pivot break six (Amphiseum,
Anulpha Pass, Modesto Heights, Talons Junction, Tech De Ra and Vineta K,
whose shared nodes carry pivots). Metropia, Moa Therma and Sol 2 move under
neither - their shared nodes are pivotless roots - so their floors pin the
pairing and the key decode only. The table is in the test's own `Pair` doc.
It was the pivot break that surfaced the scale form above: with the pivot
gone Metropia got *better*, by exactly one node.

Everything below is the layout those five claims were read off, with the
corpus figures from `crates/rcs/tests/psp2_animation_ground_truth.rs`: 77
skeleton-bearing models across the three EU packages, 19,509 nodes, 6,285
tracks, 10,160 channels, 2,776,975 keys.

## The model's node table

```c
file header (section B, from its own start):
  +0x10  u16   node count
  +0x12  u16   bind matrices written; the array ends there and other data
               follows (not zeros - see below)
  +0x18  u32   offset of u32[node count]  name hash: ~crc32 of the node's
                                          full Maya path, HD's own hash
  +0x1c  u32   offset of u32[node count]  node id, shared with the skeleton
                                          and the clip; hash unidentified
  +0x20  u32   offset of f32[16][count]   world matrix at bind, row-major,
                                          translation in row 3
  +0x28  u16   mesh object count
  +0x2a  u16   submesh count
  +0x2c  u32   offset of u32[mesh count]  each mesh object's offset

one mesh object:
  +0x00  u32   ~crc32 of the shape's full path (`...:RiverBoat4Shape`)
  +0x04  u32   unread; unique per mesh, no hash tried matches it
  +0x08  u16   node index, or 0xffff for a static mesh in world space
  +0x0a  u16   unread flags: 0x0101 or 0x0201 on a node-bound mesh,
               0x0101 (or 0x0306, 26 times) on a static one
  +0x10  u32   offset of the shape's name, NUL-terminated
  +0x14  u32   submesh count
  +0x1c  u32   offset of u32[submesh count]: submesh object offsets; the
               record `psp2::SubMesh::record` names starts 0x18 bytes in
```

**The PS4 layout is the same table with 8-byte pointers, confidence 90.**
Every offset a pointer names is a `u64`; the two counts stay `u16` at `+0x10`
and `+0x12`, and the rest moves:

```c
Vita header offset -> PS4:   name hashes +0x18 -> +0x18   ids +0x1c -> +0x20
                             binds +0x20 -> +0x28         mesh count +0x28 -> +0x38
                             mesh table +0x2c -> +0x40
mesh object:                 +0x00 hash, +0x08 node, +0x0a flags unchanged;
                             name +0x10 -> +0x18, submesh count +0x14 -> +0x20,
                             submesh list +0x1c -> +0x28, record 0x18 -> 0x28
                             bytes into the submesh object
```

Read by `oag_rcs::rcsmodel::psp2::nodes::Layout::PS4`, chosen by header word
`+0x04`. Evidence, over all 124,709 mesh objects of Omega's five base archives
(`crates/rcs/tests/omega_nodes_ground_truth.rs`) - one invariant per
pointer, because the census "every submesh reaches a mesh object" only
validates the mesh table, the submesh list and the record offset:

- **name pointer and hash**: each mesh object's `+0x00` is `~crc32` of the
  string its name pointer reaches, on **all 124,709**;
- **node-hash array and node index**: `~crc32("X:Thing")` is the name hash of
  the node a shape `X:ThingShape` is bound to, on 4,972 of 7,557 bound meshes
  on `data00` (66 %; the Vita's own rate is 12,556 of 23,326, 54 % - the rest
  are shapes not named after their transform);
- **bind array**: every written matrix is affine (`0 0 0 1` in the last
  column), 7,678 of 7,678 on `data00`, and the written ones are exactly the
  first `+0x12` nodes;
- **id array**: it appears verbatim, in order, in the sibling `.rcsskeleton`
  in 61 files (45 of `data00`'s 50 skeletons; the Vita's 35 of 49);
- **record walk**: the records the mesh objects list are exactly the submeshes
  the reader found plus one per two unpaired pointers, in every file - the 90
  "unpaired pointers" of the census are 45 real submesh records the
  relocation search could not pair, not noise.

**Confidence 92.** Found by walking the header rather than by pattern: the
skeleton's 165 ids turned up verbatim and contiguous in `altima`'s CPU
section, in the skeleton's own order, and the header word that reaches them
was traced back from there. The invariants that make it a reading:

- The **submesh count at `+0x2a` equals the number of records the
  relocation-pair search finds**, and every one of those records is listed
  by exactly one mesh object, on all 77 files and the craft checked -
  which closes the object graph `2048-rcsmodel.md` said was unwalked, from
  the top down to the record it was already finding from the bottom up.
- The **mesh object's `+0x00` is `~crc32` of its own name** on all 1,153 of
  `altima`'s, and the node table's name hash is `~crc32` of the transform
  above the shape (`trackpart_animations:RiverBoat4` for
  `...:RiverBoat4Shape`) on the 100 of 130 node-bound meshes whose transform
  name a `Shape` suffix rule recovers.
- The **bind matrices are the skeleton's scale, rotation and translation
  composed through the hierarchy without the pivots**, exactly, on all
  11,775 nodes that have one written - which is what pins the parent array,
  the root marker and the composition order in one check
  (`every_skeleton_clip_and_model_close_on_each_other`). A model with no
  skeleton beside it (every craft) is placed by these; a circuit is placed
  by the skeleton, pivots included, and the two disagree on every node with
  a pivot - see claim 3.

**`+0x12` is how many bind matrices the exporter wrote.** It equals the node
count on 35 of 49 base-package files and on every craft; a `trackZone` writes
103 of `altima`'s 966. **The remaining 863 slots are not zero, and not
matrices**: 860 of them hold small integers read as denormal floats
(`4.085e-41`, `4.128e-41`, rising - the next table's `u32`s), so past the
count the array has simply ended (re-measured 2026-09-29, on `altima`'s
`trackZone` and on Omega's `tower`, where 776 of 1,009 slots follow the same
pattern). `psp2::nodes::Node::bind` is `None` past the count, and a node with
no matrix and no skeleton entry has nothing authored to place it by: the
render draws its submeshes nowhere and counts them
(`psp2::Report::unplaced`) rather than leaving them at the node's own origin.
On the Vita this changes nothing - every such node is in a skeleton (18,255
submeshes, all with a sibling skeleton); it matters on PS4, where the
skeleton is not read yet.

**The craft's airbrakes were the visible casualty.** `feisar2048/1/ship.rcsmodel`
binds all 18 of its meshes to nodes; sixteen have the identity, and
`Airbrake_Left`/`Airbrake_Right` sit at `(±0.61, 1.19, -4.38)` with their
vertices about the flap's own hinge - so both flaps drew under the cockpit,
overlapping, until this table placed them at the tail.

## `.rcsskeleton`

```c
section, from its own start:
  +0x00  u32   2 - unread, constant on all 77 files
  +0x04  u32   node count
  +0x0c  u32   offset of u32[count]   node ids
  +0x10  u32   offset of u32[count]   parent index; a root names itself
  +0x14  u32   offset of the property block:
                 u32 slot count (9), then per node
                 { u32 offset of u32[9] property offsets; u32 9 }
  +0x18  u32   offset of f32[16][count]  for a root, the world matrix of the
                                         static transform above it; for a
                                         child, unread (see below)

one property:
  +0x00  u32   (type << 8) | slot in the low 16 bits; the high 16 vary per
               file and look uninitialised
  +0x04  u32   offset of the value
```

| Slot | Type byte | Kind | Meaning | Confidence |
| --- | --- | --- | --- | --- |
| 0 | `0x01` | vec3 | scale | 90 - composes into the bind matrices |
| 1 | `0x03` | quaternion `x, y, z, w` | rotation | 90 - same |
| 2 | `0x01` | vec3 | translation | 90 - same |
| 3 | `0x04` | bool, as a u32 | visibility, `1` visible | 75 - see below |
| 4 | `0x01` | vec3 | rotate/scale pivot | 88 - claim 3 |
| 5 | `0x01` | vec3 | pivot translate | 85 - the 7 trains |
| 6, 7, 8 | `0x00` | scalar | unread | - |

All 15,135 nodes in the base package fill slots 0 to 5; 21 nodes across two
`startgridanims` files also fill 7 and 8. The type byte is the same one a
clip channel carries in its own type word, so `oag_rcs::rcsskeleton::Kind`
serves both.

**Visibility at 75.** Every one of the 86 Anulpha Pass nodes HD draws has
`1`; the 2 to 18 race-track nodes per circuit with `0` are the ones a clip
later shows (Arena's `Ship_Futuristic0` is `1` at bind, its channel `0` for
the first 66 s); and a `trackZone` skeleton hides 944 of `altima`'s 960
mesh-bearing nodes, which is what Zone mode looks like. Not read out of the
executable. A hidden node collapses to the zero matrix in the table, so its
geometry draws nothing; a node authored hidden with nothing to show it is not
emitted at all, and the hiding inherits down the hierarchy as Maya's does.

**The parent matrix at `+0x18` is read for roots only.** For a child it
equals the parent's bind world on 89 of `altima`'s 112 and something else on
the other 23, and nothing here needs it either way; the hierarchy composes
through the parent index. A parent's index is lower than its child's on 48 of
the base package's 49 files (`bridge`'s `startgridanims_sp` is the
exception), so `Skeleton::order` sorts rather than assumes.

**The PS4 layout is the same file with 8-byte offsets, confidence 95.** Chosen
by header word `+0x04` like the model's (`psp2::is_ps4`); the count stays at
`+0x04` and everything that is an offset widens:

```c
Vita -> PS4:  ids +0x0c -> +0x10   parents +0x10 -> +0x18
              props +0x14 -> +0x20   parent matrices +0x18 -> +0x28
property block: u64 slot count, then per node { u64 table; u64 9 }  (16 bytes)
per-node table: u64[9] property offsets
one property:   { u32 tag; u32 pad; u64 offset of the value }
```

The id and parent arrays, the matrices and every value stay 4-byte. On
`tech_de_ra` (168 nodes) the arrays and the matrix table close on the section's
length to the byte. Over all 110 skeletons of Omega's five base archives
(`crates/rcs/tests/omega_animation_ground_truth.rs`) **every one of the 14,823
written model bind matrices composes exactly from the skeleton's scale,
rotation and translation through the hierarchy** - the closure that pins the
parent array, the root marker, the parent matrices and the composition order at
once. Not every model node is in the skeleton (`tech_de_ra`'s model lists
1,702 against its 168), so "every node is named" is the Vita's closure and not
this one; the nodes with **no** matrix of their own (12,310 mesh objects hang from
them, on the 2048 Zone models and a few props) **are** all named, which is
what places them.

## `.rcsanimclip`

```c
section, from its own start:
  +0x00  u32   2 - unread, constant
  +0x04  u32   bound node count N
  +0x08  u32   track count T, at most N
  +0x0c  u32   unread: 0 on 44 base files, 0x30/0x22/0xc/0x37 on the rest
  +0x10  u32   offset of u32[N]  node ids, every one a skeleton node
  +0x14  u32   offset of track[T], 20 bytes each; track i animates id[i],
               ids T..N are bound with no keys
  +0x18  f32   duration, the longest track's on all 77 files

one track:
  +0x00  u32   0
  +0x04  u32   slot count, 9
  +0x08  u32   offset of u32[9]: channel offset per slot, 0 for a slot the
               track leaves at its bind value
  +0x0c  u32   0 (1 on one track of `data/StartAnim/model/start`)
  +0x10  f32   the track's own loop length in seconds

one channel:
  +0x00  u32   0x10000 | slot
  +0x04  f32   duration, always the track's
  +0x08  u32   key count
  +0x0c  u32   (type << 16) | 1, type as the skeleton's property tags
  +0x10  u32   offset of the keys, packed by type: 16 bytes a quaternion,
               12 a vec3, 4 a scalar, 1 a bool
  +0x14  f32   rate, keys per second
  +0x18  f32   seconds per key
```

**Confidence 90 for the layout**, off three invariants that hold on all
10,160 channels: `key count == round(duration / seconds_per_key)` to within
one, `rate * seconds_per_key == 1`, and the channel's duration is its
track's. The rate is 5 on 9,728 channels, 30 on 428 (the `trackZone` clips)
and 10 on 4. In the base package's 49 files, slot 1 (4,056 channels) and
slot 2 (2,676) carry nearly all of it; slot 0 scale 92, slot 3 visibility
145, slots 6 and 7 seventeen.

**Which track is which node is positional** - track `i` animates `id[i]` -
and the extra `N - T` ids carry no keys. Pinned on `altima`'s `trackZone`,
966 ids over 7 tracks: the positional reading puts 12 of 13 channels' first
key on the node's own bind value exactly; any shift puts none.

**The keys are absolute, not deltas**, and they are HD's animation: claim 2.
Where 2048 changed an animation the loop length changed with it (Anulpha
Pass's trains 15 s against HD's 7.5, Chenghou Project's cars 38.3 against
33.3), or the phase did (Talons Junction's `Mining_Ship1_Ctrl2` starts half a
loop along HD's path with the same 37.5 s loop), and on Amphiseum 2048 baked
two of HD's loops into one 138.33 s track. `psp2_scenery_animation_ground_truth.rs`
compares only the nodes whose loop is a whole multiple of HD's, and carries a
per-circuit floor under what each measured, because a re-authored vehicle
misses by thousands of units and says nothing about the composition.

**The PS4's clip** widens the same offsets: header `+0x10` node ids and `+0x18`
tracks (`u64`), duration at `+0x20`; a track is 24 bytes, `{ u32 0; u32 9;
u64 channel table; u32 0; f32 loop length }`; a channel keeps its tag, duration,
key count and type word and holds its keys' `u64` offset at `+0x10`, the rate
at `+0x18` and the spacing at `+0x1c`. Confidence 95 on the same arithmetic:
69 clips, every id a skeleton id, every channel's key count its duration over
its spacing to within one, every clip's duration its longest track's, and every
slot the kind it is on the Vita. `tech_de_ra`: 64 tracks, 116 channels, a
250 s loop, and a 376-key 5 Hz rotation channel whose keys are unit
quaternions.

## What a race does with it

`just play 2048 --race` loads the skeleton and clip beside the circuit's
model (`race::load::geometry::psp2_animation`), and the load report says
what it found: `165 skeleton node(s), 113 animated over 113 track(s), 166.7 s
loop` and `224 node-bound submesh(es), 224 moving on 165 animated node(s)`
on `altima`. A node the clip moves, or that hangs under one it moves, keeps
its vertices in node space and takes a slot of the shader's node table; a
static node bakes through its world matrix once, normals through the inverse
transpose. **A moving node's normals go through the node matrix's inverse
transpose too** (`mesh.wgsl`, the cofactor of its upper 3 by 3, since
2026-10-05): 166 skeleton nodes across the corpus scale non-uniformly, and
**51 of them, over the fourteen race circuits, carry a mesh and move** (none on
Anulpha Pass, Chenghou Project, Moa Therma, Park or Sol; ten each on Bridge and
Subway), and those 51 meshes used to be lit off a skewed normal.
`crates/render/tests/moving_normal_inverse_transpose.rs` shades one oblique
quad both ways - moved by the node table under `scale (0.5, 1, 1)`, and baked
with the CPU inverse transpose - and the two differed by up to 8 in a channel
(7,968 summed over the frame) before and by 0 after. Uniform scales and
rotations are unchanged by it. Past
the table's
383 slots a node freezes at time zero rather than misplacing - the same
direction `mesh::anim_node::placement` takes, and none of the fourteen race
circuits reaches it (`park` is the largest at 416 nodes, 357 with meshes,
and only moving nodes take slots).

Two captures of Tower at the grid, `--anim-seconds 0` and `20`, differ by
28,327 pixels: the sky traffic the clip flies in past the statue
(a 0 s and 20 s frame pair, top and bottom; the craft's airbrakes before and after the node table). Altima's
grid view shows none of its 130 moving meshes - the nearest, a cat balloon,
is 190 units off and 37 degrees above the camera's frame - so the fix there
is only measurable, not visible, from the grid.

## A Zone race (2026-10-05)

**A 2048 Zone race loads `trackZone.rcsmodel` and its own `.rcsskeleton`,
`.rcsanimclip` and `.pvs` beside it** (`race::load::geometry::sibling_model`),
where it used to race the ordinary model and say so. They are not the race
model's skeleton under another name: `altima`'s is 966 nodes and 6,349
submeshes against the race model's 165 and 2,817, its clip 7 tracks at 30 Hz
over 6.7 s, and 2,302 of its 2,406 node-bound submeshes are hidden at bind and
never shown by the clip. The load report says `zone: ...trackZone.rcsmodel is
this circuit's Zone model`. The Omega Collection's Zone models are passed over
(their racing is out of scope), and so is a title with no such file.

**What the picture is** is in [2048-material-params.md](2048-material-params.md):
the colours its materials author are drawn unlit and the road, a placeholder
shader whose output is unread, is not drawn. No capture of the original
exists here to compare it with, and the picture has not been judged against
one.

## The start-grid clip (2026-10-05)

`trackpart_startgridanims.rcsanimclip` and its `_sp` twin are **not a camera**.
Beside each is a model, `.rcsskeleton` and `.vex` of the same stem:

| | `trackpart_startgridanims` | `_sp` |
| --- | --- | --- |
| model | 116 submeshes, 113 mesh objects, 120 nodes | 45 submeshes, 44 mesh objects, 45 nodes |
| material | `startanim_fc07_lambert_emmisive` on every submesh, texture `pitbot_col.gxt` | the same |
| clip | 70 tracks (69 on a mesh-bearing node), 38.3 s, 191 keys each | 36 tracks (35), 16.6 s, 83 keys |
| channels | translation 70, rotation 55, **slot 7 on one track** | translation 36, rotation 31, slot 7 on one |

It is a **hierarchy of articulated pieces under one root scaled 100 times**,
the root `cf4e3c8b` at `(366, 21, 143)` on `altima` and **110 units ahead of the
Start Position** `(346, 6, 253)` along the grid's `-z`: the texture name says
pit bots, and nothing about it is a view. **Slot 7 is a scalar on that root**
(`0.66` falling to `0.03` over the loop on the full clip, `0.2` to `0.35` on the
`_sp` one) whose meaning is not read.

**Not wired.** The model builds through `oag_mesh::mesh::rcs::psp2::build`
with its skeleton and clip (116 moving submeshes on 120 animated nodes), but
**nothing says when it plays or where it is drawn**: the circuit's own model
already contains a start grid, so drawing it unconditionally would double the
furniture, and an animation fired on a guess is exactly what this project
refuses. A capture from the original's start, or the race-intro code, settles it.
An attempt to look at it in this renderer from the camera of a posed capture
showed sky only, so its picture has not been seen either.

## What is not read

- **The node id hash.** `~crc32` of the full path, the short name, the
  transform's name, upper- and lower-cased, and FNV-1/1a, djb2, sdbm,
  Java's and Murmur3 of each were tried against the 100 nodes whose names
  are known; none matches. The three files agree on the value, so nothing
  here needs the function, but a tool that wants to build one of these
  files from names does.
- **The mesh object's `+0x04` word and `+0x0a` flags.** `0x0201` looked like
  a node-bound witness on `altima` (130 of 130) and is not: corpus-wide
  12,310 node-bound meshes carry `0x0101` and 6,393 `0x0201`.
- **Property slots 6 to 8 and the high half of the tag word**, the clip
  header's `+0x0c`, and the skeleton's `+0x18` matrix for a child.
- **`trackpart_startgridanims(_sp)`** is **not a camera and is not wired**:
  see "The start-grid clip" below. What triggers it, and when, is not read.
- **The loader.** No function in `vita-2048-eu-v104` was named this pass;
  every claim above is measured against files and against Wipeout HD, which
  is why the layout confidences stop at 92 and the visibility reading at 75.
  `RcsModel_Load` (`0x812f15b2`) is the one already-named entry; the
  skeleton's and clip's have not been looked for.
- **HD's own `02_track` and `03_track`** turned out to be Metropia and Moa
  Therma respectively - told apart by which 2048 circuit shares their node
  names (35 with Metropia and none with Moa Therma for `02_track`; 67 and
  none the other way round). Recorded here because `oag_hd` lists the two
  by directory only.

## Reproducing

- `crates/rcs/tests/psp2_animation_ground_truth.rs` - the corpus figures
  and the bind-matrix composition, `just test-data`.
- `crates/render/tests/psp2_scenery_animation_ground_truth.rs` - the
  twelve circuits against Wipeout HD, one test each.
- `crates/game/examples/vita_2048_anim_hd_oracle.rs` - dumps an HD
  `track.vex`'s `Anim Transform` nodes, keys and world matrices at fourteen
  times, the oracle the Python pass that read these files was checked
  against.
- `crates/game/examples/vita_2048_dump.rs` - writes one package entry to a
  file, for a hex editor.
