# 2048's scenery moves; the Zone and start-grid clips, the UV-animated materials and the node loader are next

2026-09-16. Started from "nothing on 2048 is animated at all" and found the
reason first: `track.vex` authors **no `Anim Transform`** (1,834 `Transform`s,
7 `Mesh`es, zero of class `0x3c0` on `altima`), so the two mechanisms the
other titles animate scenery with had nothing to reach. What moves a 2048
circuit is three files that were all sitting beside `track.rcsmodel` and
one table inside it - a node table in the model's own header, a
`.rcsskeleton` with the hierarchy and bind pose, and a `.rcsanimclip` with
5 Hz absolute keys. All three are read (`oag_rcs::rcsmodel::psp2::nodes`,
`oag_rcs::rcsskeleton`, `oag_rcs::rcsanimclip`, evaluated by
`oag_rcs::rig`), planned onto the shader's node table by
`oag_render::mesh::rcs::psp2::placement`, and played through the same table
Pulse's and HD's `Anim Transform`s already ride - `oag_render::mesh::Motion`
is the two-variant enum that lets one table carry both. Full format, every
confidence and every number in
[2048-animation.md](../../docs/formats/2048-animation.md).

**The oracle is Wipeout HD, and it is what made this a reading rather than a
plausible decode.** 2048 re-ships twelve HD circuits with the same node
names; HD's `Anim Transform` evaluator is validated; so every claim about
the rig - vertices in node space, keys as HD's animation resampled, Maya's
pivot composition, `local * parent_world` with a root's own static parent
matrix, linear/nlerp interpolation wrapping to key 0 - was checked node by
node against HD's world matrices at up to fourteen times
(`crates/render/tests/psp2_scenery_animation_ground_truth.rs`, one test per
circuit, floors set by running two deliberate breaks through the suite and
recorded in the test). The pivot was the finding that would not have come
from `altima` alone: the bare `S * R * T` puts a wind-turbine rotor 2,500
units from its tower and 63 of Anulpha Pass's 86 shared nodes on HD's
matrix, the pivot form puts the rotor on its hub and 76 of 86 on HD's, and
the pivot *translate* brings the seven trains from 520 units off to 4. And
one node on Metropia said the scale is *not* about that pivot, only the
rotation - a correction the first reading would have shipped without the
mutation pass that surfaced it.

**Reading the node table also fixed a placement defect that shipped**, the
same shape as Pulse's `Anim Transform` one: 130 of `altima`'s 1,153 meshes
are bound to a node and authored in its space, and drew as world coordinates
a median 1,086 units (max 5,765) from their place. Every craft binds all of
its meshes too; the two airbrakes are the ones with a non-identity node, and
both drew under the cockpit until now.

**Two traps worth carrying.** (1) The model's bind matrices are composed
*without* the pivots (11,775 of 11,775 written ones, exact), so they are not
where a node is drawn when a skeleton is present - they match HD's time-zero
world on only 31 of 86 nodes - and only place a model that has no skeleton
(a craft). (2) The header's `+0x12` is how many of those matrices the
exporter wrote; a `trackZone` writes 103 of 966 and the rest are zeros,
which read as "collapse to the origin" until the count was found.

**What the corpus test measures** (`crates/rcs/tests/psp2_animation_ground_truth.rs`,
`just test-data`): 77 skeleton-bearing models across the three EU packages,
19,509 nodes, 6,285 tracks, 10,160 channels, 2,776,975 keys, every channel's
key count its duration over its spacing, every clip id a skeleton node,
every submesh record reachable from exactly one mesh object.

## Open

- **`trackZone.rcsanimclip`/`.rcsskeleton`/`.rcsmodel` are parsed and used
  by nothing.** A 2048 Zone race here races the ordinary `track.vex` and
  its model, so the Zone trio is never loaded; the Zone skeleton hides 944
  of `altima`'s 960 mesh-bearing nodes at bind (visibility, slot 3), which
  is presumably what a Zone circuit looks like on the Vita and not what
  this engine draws for one. Its keys run at 30 Hz.
- **`trackpart_startgridanims(_sp).rcsanimclip`** - the start-grid animation,
  70-odd tracks per circuit, the only files with property slots 7 and 8
  (scalars, unread). No caller; what it animates (a camera? the grid
  furniture?) is unread.
- **The `uv_anim_*`/`cf_uvanim_*` materials 2048 ships** (188 entries name
  them) are HD's `time`-driven shader scroll, wired on HD through
  `mesh::slots::ADD_SECOND` and a per-material table. 2048's material table
  reads names and texture paths but not the shader input floats the scroll
  needs (`rcsmodel/psp2/material.rs`'s module doc says why), so nothing
  scrolls on 2048 yet. Separate from the node rig, deliberately left out of
  this pass.
- **Visibility is at 75**, read off which nodes a clip later shows and which
  a Zone skeleton hides, not off the executable.
- **51 moving meshes across the fourteen race circuits sit under a
  non-uniform scale** and light off a normal the shader's node matrix skews
  (no inverse transpose in `mesh.wgsl`, its own standing caveat); the baked
  path does the inverse transpose, the moving path does not.
- **The node id hash function** is unidentified (ten functions tried); the
  mesh object's `+0x04` word and `+0x0a` flags, property slots 6 to 8 and
  the tag word's high half, the clip header's `+0x0c`, and the skeleton's
  `+0x18` matrix for a non-root are unread.
- **No function in `vita-2048-eu-v104` was named this pass**; the skeleton's
  and clip's loaders have not been looked for, which is why the layouts stop
  at 92 rather than crossing into the runtime-verified band.
- **HD's `02_track` is Metropia and `03_track` is Moa Therma** (35 shared
  node names with Metropia and none with Moa Therma for `02_track`, 67 and
  none the other way); `oag_hd::ENVIRONMENTS` lists them by directory only.

## Next Steps

- Load `trackZone.rcsskeleton`/`.rcsanimclip` in `race::load::environment`
  the way `race::load::geometry::psp2_animation` does for the race model,
  then look at what Zone hides - that is a half-day, the readers are done.
- Find the skeleton and clip loaders in Ghidra off the `"PSP2/Psp2.Rcs*Loader.cpp"`
  allocator-tag strings `RcsModel_Load` was named from; a runtime read of
  slot 3 and of the pivot composition would move 75 and 88 to the 90s.
- The UV scroll needs the material's shader-input floats, which is the
  `material.rs` struct-shape problem, not an animation one.

## From the HANDOVER.md index (moved 2026-09-25)

2048's `track.vex` authors no `Anim Transform`; what moves its scenery is a node table in `track.rcsmodel` plus the `.rcsskeleton`/`.rcsanimclip` beside it, all three read 2026-09-16 and played through the same node-matrix table the other titles use (`mesh::Motion`). Checked node by node against Wipeout HD's own `Anim Transform`s on the twelve circuits both titles ship - the pivot composition is the finding `altima` alone would not have given - and reading the table put 130 of `altima`'s meshes (median 1,086 units off) and every craft's airbrakes where the original draws them. Open: the Zone and start-grid clips are parsed and unwired, 2048's `uv_anim` materials still do not scroll (a material-struct problem), visibility is at 75, no loader named. See [2048-animation.md](../../docs/formats/2048-animation.md)
