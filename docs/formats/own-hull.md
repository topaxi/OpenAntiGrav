# The own hull file (`hull.glb`)

A craft hull of this project's own. Its geometry is drawn on a **host** model from a
disc: the host's textures, materials, render state, flap animation, plume and wreck all
stay, and only the geometry of its mesh nodes is replaced. The file carries
geometry, normals, texture coordinates and vertex colours, and names the host's
textures by file name. The committed files hold no pixel of the disc or of ours.

It is the hull-geometry slice of the planned own craft format. It has no manifest, no
handling and no picker. The first and only user is Pulse's Detonator craft
([detonator-pulse.md](../gameplay/detonator-pulse.md#the-craft)).

| | |
| --- | --- |
| Reader | `oag_mesh::mesh::own_hull::parse`, and `substitute` draws it on a host `Model` |
| Wiring | `oag_livery::own_hull` (race), `oag_game::preview::own_hull_on` (Ship Select), `oag_raceplay`'s shadow hulls |
| Title data | `oag_title::DetonatorCraft::own_hull`, a list of asset entries (on Pulse a mod's fallback entries, ADR-0100) |
| Marker | the asset's extras: `"oag_kind": "hull"`, `"oag_version": "1.0.0"` (another kind or major is refused by name) |
| Writer | `scripts/blender/oag_hull.py` ([Blender](../tools/blender.md#a-craft-hull)) |

## Why a glb, and why it lives in the mod

The planned own craft format is glTF binary, and nobody edits a hull by hand: Blender writes it
and reads it back. A glb holds every number as a little-endian binary accessor, so an `f32`
reaches the reader bit for bit, with nothing to round and no axis or texture-coordinate change
(glTF's axes and UV origin are the engine's). `scripts/check-leakage.py` refuses a tracked `.glb`
in the engine repository (model sources of our own live in the mod repositories), and the files
are a mod's fallback entries anyway ([ADR-0100](../architecture/adr/0100-a-mod-carries-fallback-entries.md)),
so they live in the official-mods repository, in `oag.detonator-pulse`. The engine's unit tests
build a small hull glb in memory instead of keeping a fixture.

The simulation reads nothing in it.

## Layout

One glb, no manifest beside it. Every node is a root of the one scene:

| Node | Mesh | Holds |
| --- | --- | --- |
| a part, named as the host mesh node (`shipShape`) | one primitive per surface | `matrix`: model space from the part's own, column-major (a `.vex` matrix's 16 floats: rows are basis vectors, translation at 12..14) |
| `shadowShape` | one primitive, `POSITION` and indices only | the shadow occluder in model space; no transform |
| a locator (`engine_flare`, `ship_collision_fx*`) | none | `matrix`, laid out as a `.vex` locator matrix |

A surface's primitive is `TRIANGLES`, counter-clockwise, with:

- `POSITION` (float VEC3, part space, with `min`/`max`), `NORMAL` (float VEC3),
  `TEXCOORD_0` (float VEC2, `v` down), one each per vertex;
- `COLOR_0` (normalised u8 VEC4), optional: absent means the host's tint;
- indices, u16 (u32 where a surface needs more than 65,534 vertices);
- a material named as the host texture's file name (`texture1.tga`). The surface is matched to
  the host by that name.

The file has one buffer, its BIN chunk; a buffer outside it is refused. Materials are ordinary
glTF materials, and the format allows images, textures and samplers: they are reserved for a hull
that brings textures of its own and are **not read today**. A file that carries any still reads,
the host's textures paint it, and the load report's "own hull drawn" line counts the images. The
two committed Detonator hulls carry none, because they paint with the host's textures and must
hold no pixel of the disc (`own_hull::tests`).

Names are matched with the directory and case ignored, and with a Blender `.NNN`
suffix stripped (`fold`). Axes are the engine's: Y up, `+Z` the nose, `+X` the craft's
left, engine units.

## How it is drawn on the host

- **Fit.** A file fits a host when every part names one of the host's mesh nodes and has
  at least one surface naming one of its textures (`fits`, `misfits`). The title lists its
  files in order, and the first that fits is drawn. When none fits, the disc's hull draws,
  and a `warn` names each file's misses. A surface that names a texture the host lacks is
  not drawn, and that is a `warn` too. Pulse PS2's `Zone.vex` is a different model (other
  node names, one `Zone_ColourMap.tga` atlas), so each pressing has a file and neither
  fits the other's host.
- **Surfaces.** A surface lands on the host draws of the same node with the same
  texture. A pair can have several draws (Pulse's flashers draw opaque and cutout), and
  then the surface is drawn once for each. Each copy takes that batch's list, blend,
  cull, alpha reference, layer, LOD node, and per-vertex fields (lighting flag, glow
  stamp, texture animation, depth bias). Each also takes its `0x2000` shine twin.
- **Colour.** Without `COLOR_0`, a surface takes the builder's tint on a batch
  the disc leaves uncoloured, and white on one it colours. Lighting always follows
  the host batch.
- **Pivots.** A part's vertices are baked through its `matrix`. On an airbrake
  node the transform is also the flap's hinge, and the flap swings about its local X
  as Pulse's do (`oag_mesh::mesh::Flap`).
- **LOD.** A part named for a node under a coarser tier of a `LodGroup`
  (`lodShape`) keeps the switch. Without one, the group table is dropped and the hull
  draws at every distance.
- **Anchors.** `engine_flare` (the exhaust and flare anchor, its Z the exhaust axis) and
  `ship_collision_fx*` (spark anchors) replace the host's. Absent ones keep the host's.
  `ship_muzzle` and `cannon_flash_*` are **not** taken from the file: the simulation
  reads them off the disc's `.vex` (`oag_raceplay`'s `load::muzzle`), and the drawn
  weapon must leave where the simulation says.
- **The shadow.** Pulse's default `original` shadows cast each craft's `Dynamic Shadow
  Occluder`. A fitting file casts its `shadowShape` instead, built by
  `oag_vex::shadow_occluder::Occluder::from_triangles` (welded positions, plane per
  triangle, faces across edges). A file with none casts no `original` shadow, and the
  load report says so.
- **Ship Select** draws the same file on the front-end twin (`Zone_FE.vex` on Pulse).
  It has the same mesh and texture names and no `lodShape`; its `ship` node is an
  `Anim Transform`, whose space is the hull's. The flaps are then on the file's hinges
  and are not seated again.

## The Pulse host

Pulse PSP's `Data\Ships\<Team>\Zone.vex`, the same geometry on all eight teams (equal
mesh payload sizes, `oag-view --nodes`), painted per team:

| Mesh node | Textures its draws use |
| --- | --- |
| `shipShape` | `engine_general.tga`, `texture1.tga`, `texture2.tga`, `texture3.tga` (the last three with the shine pass) |
| `cockpitShape`, `pilotShape` | `engine_general.tga` (vertex-coloured) |
| `AirBrake_LeftShape` | `engine_general.tga`, `texture2.tga` |
| `AirBrake_RightShape` | `engine_general.tga`, `texture3.tga` |
| `canopyShape` | `Glass_ADD.tga` (blended) |
| `flashersShape` | `colours_flashing_GLOW.tga` (opaque and cutout) |
| `lodShape` | `texture4.tga` (the far tier) |

Pulse PS2's `Zone.vex`, the same node names on all twelve teams (the disc carries the four
pack teams):

| Mesh node | Textures its draws use |
| --- | --- |
| `shipShape` | `Zone_ColourMap.tga` (the team's one atlas), `zone_engine.tga` |
| `self_illuminated` | `Lights_GLOW.tga` (opaque and cutout) |
| `canapoyShape` (the disc's spelling) | `Zone_ColourMap.tga`, `Glass2_ADD.tga` (blended) |
| `glowingShape` | `Lights_GLOW.tga` (opaque and cutout, part of it with animated coordinates) |
| `AirBrake_leftShape`, `AirBrake_rightShape` | `zone_engine.tga`, `Zone_ColourMap.tga` |
| `lodShape`, `glowing_lodShape` | the far tier |

Feisar's anchors on the PSP, engine space:
- airbrake hinges `locator3` (left) and `locator1` (right) at `(±1.8089, -0.1207, -4.4095)`;
- `pilot` at `(0, 0.1351, -3.9498)`;
- `engine_flare` at `(0, 0, -7.4236)`;
- `ship_muzzle` at `(0, -0.0643, 7.0181)`;
- the `shadow` Transform at `(0, -2.0499, 0)`.

## Proof

- **Round trip.** Feisar's `Zone.vex` was exported to OBJ, imported into Blender,
  placed per this convention (origins and locators from the node matrices) and written
  with `oag_hull.export_hull`. A Pulse Detonator race drew it pixel-identical to the disc's
  hull at tick 30, 960x544, PSP EU. The one difference was the shadow, which the OBJ
  did not carry.
- **`own_hull_ground_truth`** (`crates/livery/tests/`): on all eight teams of PSP EU
  and USA, `hull.glb` fits and every part and surface of it resolves; on all twelve
  teams of PS2 EU, `hull_ps2.glb` does.
- **`just blender-fixtures`** imports both files into Blender and exports them again, and
  compares the bytes.
- **TOML to glb (2026-11-27).** The hull was a TOML file until then. Each glb was written by
  importing its TOML into Blender and exporting it with the glb writer. A check outside Blender
  then compared the two: the same parts, surfaces, materials, transforms, locators and shadow,
  and every position, normal, UV, colour and index equal bit for bit, with the TOML narrowed to
  `f32` as its reader did. The totals stayed 762 vertices and 462 triangles (PSP) and 2,057 and
  1,470 (PS2). Race frames at tick 420 and Ship Select stills, 1280x720, PSP EU and PS2 EU, were
  pixel-identical before and after (0 differing pixels each), and Blender's stock glTF importer
  opens both files.
- **`livery_ground_truth::a_pulse_detonator_race_flies_the_own_hull_on_every_craft`**
  (`crates/raceplay/tests/`): every craft of a Pulse Detonator race flies the file's
  parts. Emptying `own_hull` turns both red.
