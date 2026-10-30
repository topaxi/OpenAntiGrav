# The track file (this project's own custom-track format)

**Status: designed and built (2026-10-11); the optional art section built 2026-10-29.**
Decision and rationale:
[ADR-0063](../architecture/adr/0063-the-own-track-format-is-glb-plus-a-toml-manifest.md), and
for the art section [ADR-0076](../architecture/adr/0076-a-track-file-carries-its-art-as-an-optional-section.md).
Code: [`oag-trackfile`](../../crates/trackfile). Nothing here is measured off an original
game: the layout is **chosen, not measured**. What *is* measured is that it carries a disc
circuit's race inputs unchanged (census below).

## Files

`<name>.track.toml` (the manifest) and the `<name>.glb` it names in `track.glb`, in the same
directory. Raced with `oag-game --track-file <manifest>` (host title's ship, rules, physics). Authored in Blender with [the add-on](../tools/blender.md), which writes and reads the art section too (its mapping from Blender: [Art](../tools/blender.md#art-meshes-textures-the-sun-and-baked-light)).

### Manifest (TOML)

| Key | Meaning |
| --- | --- |
| `[toml-schema]` | `location` ending `v1/track.tosd`, `version = "1.0.0"`: the kind and major, per [ADR-0064](../architecture/adr/0064-project-toml-files-declare-a-toml-schema.md) and [TOML Schema](toml-schema.md). Missing, another kind or an unknown major: refused by name. This replaced `track.format` and `track.version`; there is no minor. A key the reader does not know is read and reported as unread. |
| `track.id`, `track.title` | Identifier (keys records and ghosts as `foreign:<id>`) and display title. |
| `track.host` | The title whose disc the art borrows from, spelled as `oag_source::layers::slug` spells its name (`wipeout-pulse`, `wipeout-pure`, `wipeout-hd`, `wipeout-omega-collection`, `wipeout-2048`; a mod's `titles` vocabulary). **Required exactly when the art borrows** (a `host` texture or a `sky`), optional otherwise. A track that names one loads only with that title's disc as the track source; on another title it races as a Race Remix with the host's disc as the track source ([ADR-0076](../architecture/adr/0076-a-track-file-carries-its-art-as-an-optional-section.md) decision 13). The disc exporter writes its source's title. |
| `track.titles` | Optional. The titles the author offers the track on (same slugs); absent is every title. Checked against the craft's title (chosen, not measured). |
| `track.glb` | The glb's file name, relative to the manifest. |
| `track.notes` | The producer's free text (what it chose, ignored, could not map). Printed in the loader report. |
| `spline.point_version` | The `WO Track` point version (`SHORT_POINT_VERSION` = `0x107` = 263 for importer output). |
| `[[spline.path]]` | `points` (count, checked against the glb), `entry`/`exit` (junction index, `-1` none), `max_spacing_bits` (the `f32` bit pattern, an integer so exact). |
| `[[spline.junction]]` | `prev`, `next`: two path indices each, `-1` none (primary, then alternate). |
| `[counts]` | `collision_nodes`, `speedup_pads`, `weapon_pads`, checked against the glb; `art_meshes` only when the track has art; `triggers` only when it has triggers. |
| `[props]` | Optional: the track's own properties for a simulation mod, name to number or bool ([below](#properties-and-triggers-for-simulation-mods)). Absent when there are none. |
| `[channels]` | Optional: `publishes = ["<mod id>/<name>", ...]`, the event channels the track sends ([below](#channels-events-between-a-track-and-mods)). Absent when it sends none. |
| `[art]` | Optional (ADR-0076). Absent, the file is byte for byte what 1.0 wrote. `sky`: a host circuit's `.vex` entry whose `Skycube` is drawn (it pins the disc, so `track.host` is required). `ambient`: linear RGB, the hull rig's ambient. `draw_collision`: `true` draws the flat-shaded floor and walls under the art (decision 16); absent is `false`, the art alone. |
| `[art.skybox]` | Optional, never with `[art] sky`: the track's own sky (ADR-0076 decision 14). `faces`: six `[[art.texture]]` rows in cubemap order `+X, -X, +Y` (zenith), `-Y` (nadir), `+Z, -Z`, `-1` for a face drawn as the gradient. `zenith`, `horizon`: linear RGB, both or neither, the gradient by elevation. At least one form; a face without a texture needs the gradient. Drawn on every title: `blender_loop` (its gradient, the checker overhead) raced with its own sky on Pulse PSP EU, Pure PSP EU, HD/Fury PS3 EU, Omega PS4 EU and 2048 (the decrypted base folder), 2026-10-29. |
| `[art.sun]` | `direction` (unit, towards the sun), `colour` (linear RGB), `intensity`, `angular_diameter` (degrees, 0 a point sun), all required when the table is present; `shadows`, absent meaning `true` (an authored sun is the shadow light; a disc export writes `false`). |
| `[[art.light]]` | Lights beyond the sun and the ambient. `kind = "directional"` is the only kind in 1.x: `direction` (unit, towards the light), `colour` (linear RGB), `intensity`. A disc export writes a circuit's second and later `DirectionalLight`s here (ADR-0076 decision 12). |
| `[art.lensflare]` | Pure's sun flare: `atlas` (an index into the texture rows), optional `colour` (linear) and `direction` (unit) overriding the sun's. Refused without a sun unless both overrides are given. |
| `[[art.texture]]` | Exactly one of `image = "file.png"` (the author's picture beside the manifest; the only form a committed fixture may use) or `host = { entry = "...", name = "..." }` (a `Texture` node of a circuit on the host title's disc, by its runtime asset path, or by file name alone when `name` has no directory). |

No simulation float is in the manifest: a decimal float is an `f64` round trip. The art's
numbers are render-side only, so they are TOML floats (an integer reads too). `[props]` is the
one exception a mod reads: each value is written as its `f32` widened to `f64` (the shortest
decimal of that double), which reads back to the same `f32`.

### Glb (glTF 2.0 binary)

One scene, flat nodes, tightly packed little-endian accessors. Right-handed, Y up, world units.

| Node / mesh name | Primitive | Attributes |
| --- | --- | --- |
| `oag/start` | one `POINTS` vertex | `POSITION`, `_OAG_LEFT`, `_OAG_UP`, `_OAG_FORWARD` (VEC3) |
| `oag/spline/<i>` | `POINTS`, one vertex per control point | `POSITION` (pos), `_OAG_TANGENT`, `_OAG_DOWN`, `_OAG_LATERAL` (VEC3), `_OAG_PROGRESS`, `_OAG_RACING_LINE` (SCALAR), `_OAG_HALF_WIDTH`, `_OAG_AI_BOUND` (VEC2 left, right), `_OAG_SECTION_FLAGS` (u8 VEC4: section, flags, 0, 0), `_OAG_LIGHT_SCALE` (u8 VEC4), and optionally `_OAG_AI_BYTES` (u8 VEC4: lane band low, lane band high, speed-floor byte, 1 when the point carries them; absent reads as none), and optionally `_OAG_CAGE_A`, `_OAG_CAGE_B` (u8 VEC4 each: the collision cage's eight signed bytes `+0x68..+0x6f` in order, two's complement; absent reads as all zero) |
| `oag/pads/speedup`, `oag/pads/weapon` | `POINTS`, one vertex per pad | `POSITION` (informational: the pad's world centre), `_OAG_TO_WORLD` (MAT4, the row-major row-vector matrix as stored), `_OAG_MIN`, `_OAG_MAX` (VEC3), `_OAG_DISABLED` (SCALAR); node extras `oag_props`, one property object per pad, only when the track carries pad properties |
| `oag/triggers` | `POINTS`, one vertex per trigger | as a pad's, without `_OAG_DISABLED`; node extras `oag_names` (one string per trigger) and, when any trigger has properties, `oag_props` (one object per trigger) and, when any sends a channel, `oag_emits` (one string per trigger, `""` for none) |
| `oag/collision/<n>` | `TRIANGLES`, u16 indices, one primitive per mesh | `POSITION`, `_OAG_SCALAR` (per-vertex scalar, `1.0` when absent); node extras `oag_kind` (`floor`, `wall`, `reset`, `mag_floor`, `cage`, `track_wall`), `oag_name`, `oag_version` |
| `oag/sun` | no mesh; only a rest `rotation` | present exactly when an animation turns the sun (below) |
| `oag/art/<n>` | `TRIANGLES`, one primitive per draw | `POSITION`, `NORMAL`, `TEXCOORD_0` (v down, as the disc's), `COLOR_0` (VEC4 float: the baked light, RGB **linear** as glTF reads it, alpha coverage); node extras `oag_name`, `oag_texture` (texture row, absent for untextured), `oag_blend` (`opaque`, `cutout`, `alpha`, `additive`, `unblended`), `oag_culled`, `oag_lit`, `oag_shadow` (`both`, `cast`, `receive`, `none`), `oag_alpha_ref` (`0..=255`, the alpha-test reference; absent is the renderer's default), `oag_layer` (unsigned integer render layer, absent is `0`), `oag_pad` (`speedup` or `weapon`: a pad plate, below; absent is track art) |

The cage bytes travel, but the `WO Track` header flag that turns the cage on
(`oag_vex::track::cage_flag`) has no field here, so a race on a track file runs no
collision cage ([collision-cage.md](../ghidra/functions/psp-pulse-usa/collision-cage.md#the-cage-is-a-contact-source-live-2026-10-20)).

A reader finds a collision node by its `oag_kind` extra (so a Blender object tagged with a
custom property qualifies whatever it is named), the rest by name. Mesh nodes without either
are counted as unread, never dropped silently. A non-identity node transform on a record is
refused (`Transformed`): bake it before export. The one exception is an art node an animation
moves (below).

### Properties and triggers (for simulation mods)

A track file may carry **author-defined properties** for a [simulation
mod](../architecture/wasm-extensions/README.md#and-track-properties-2026-10-30) to read:
on a speed or weapon pad, on a trigger, and on the track itself. The engine gives none of
them a meaning. A property is a name and an `f32`; a bool is `1.0` or `0.0`. Refused by
name, on read and on write (`oag_trackfile::props`): a string, an empty name, two names on
one entity whose FNV-1a 32 hashes (the key a mod reads by) are equal, a pad property list
whose length is not the pad count, more than 64 triggers or more than 1,024 properties in
all (the ABI's caps, `oag_ext_abi::track`).

A **spline path** carries properties too: the glb spline node's `oag_props` extra is one
property object for that path (`Track::path_props`, one list per path or none). A mod reads them
off the path, and off every route that drives the path (the track region merges a route's paths'
properties), so a pit lane's `pit` finds its route (Checkpoint Circuit, the mod examples).

A **trigger** is a box placed like a pad (`_OAG_TO_WORLD`, `_OAG_MIN`, `_OAG_MAX`, local
space) with a name, but without a pad's vertical expansion: the box is the author's. A mod
hears a craft enter and leave it (`trigger-enter`, `trigger-exit`); the race does nothing
else with it.

In Blender ([the add-on](../tools/blender.md)): a custom property `mod_<name>` on a pad
empty (`oag/pads/<kind>/<i>`), on a trigger empty (`oag/triggers/<name>`, tagged
`oag_kind = "trigger"`, its box in **Box min** and **Box max**), on a spline object (tagged
`oag_kind = "spline"`) or on the scene is exported as `<name>`; anything else, a string or an array, is refused naming the object and the key.
A track without any writes none of these keys, so every earlier file is byte for byte what
it was (the disc exports in `trackfile_ground_truth` included). Round trip, refusals and the
schema: `crates/trackfile/tests/props.rs`.

### Channels: events between a track and mods

A **channel** is a name, `<mod id>/<name>` (the id of lowercase letters, digits, `.`, `-`, `_`; the
name the same without the `.`; at most 64 bytes), that travels as its FNV-1a 32 hash
(`oag_ext_abi::channel_key`, the hash a property's name has). A track sends and starts on them
with no module at all:

| Field | Meaning |
| --- | --- |
| `[channels] publishes` (manifest) | the channels the track sends. Repeats and names hashing alike are refused |
| a trigger's `oag_emits[i]` (glb, on `oag/triggers`) | the channel a craft sends by **entering** trigger *i*; it must be in `publishes`. Raised for every craft, a person's or the AI's, on the tick it enters; the race raises it itself (`oag_raceplay::extensions`, `Beacons`) |
| an animation's `oag_on` (glb, see Motion) | the channel that starts it; any valid name, from a mod or from this track |

Every module that runs in the race hears a trigger's channel on the next tick as an `Event::Mod`
whose `from` is `TRACK_SENDER`; the view starts each `oag_on` animation at the tick its channel
last fired (`Race::channel_started`), render only: collision never moves, and nothing here is in a
hash of a race with no module. Refused by name on read and write (`oag_trackfile::props`): a
malformed name, a trigger naming a channel the track does not publish. In Blender: a trigger's
**Emit**, an object's **Start on** and the scene's **Channels published**
([the add-on](../tools/blender.md)); `roundtrip.py channels` proves the round trip headless.
The manifest key is in `schemas/v1/track.tosd`.

### Motion (glTF `animations`, ADR-0076 decision 10, since 2026-10-29)

| Field | Meaning |
| --- | --- |
| A moving `oag/art/<n>` node's `translation`, `rotation`, `scale` | its rest pose (glTF's decomposed form; a `matrix` is refused). Its vertices are then in the node's own space. A rest pose on an art node no animation moves is refused (`Transformed`) |
| `animations[]` | glTF animations. A channel targets an art node or `oag/sun` (anything else is refused by name: collision, pads and the spline never move), path `translation`, `rotation` (`x, y, z, w`) or `scale` (the sun: rotation only). Samplers are float accessors, `LINEAR` (rotation by spherical interpolation, shorter arc) or `STEP`; `CUBICSPLINE`, normalised-integer keys and morph weights are refused. Times start at or after 0 and strictly increase; one channel per node property in the whole file |
| An animation's `extras.oag_on` | Optional: a channel name (`<mod id>/<name>`). The animation holds its first key until that channel fires (a trigger's `oag_emit`, or a mod's `emit`), then plays from the moment it did, on the same clock ([below](#channels-events-between-a-track-and-mods)). Absent: it plays from the clock's zero, as before |
| An animation's `extras.oag_loop` | `loop` (absent too): wraps on the animation's length, its latest key; `clamp`: holds its last keys. Before a channel's first key it holds the first |
| `oag/sun`'s rotation | the sun's direction at time *t* is `[art.sun] direction` turned by the rotation from this rest rotation to the channel's rotation at *t*, so a lamp's own axis convention never enters |

The clock is the race's animation clock: the tick over 60, or `--anim-seconds`, never the wall
clock. `oag-trackfile` stores the keys bit for bit and samples nothing (it is in
`check-determinism`'s scan); the renderer samples (`oag_mesh::mesh::Keyed`, a third
`Motion` beside the `.vex` and `.rcsanimclip` forms, because their keys are evenly spaced and always
wrap). A file with no motion writes no `animations` and no `oag/sun`, byte for byte as before.

## What format 1.x carries no field for

`oag_trackfile::ABSENT`, named in every loader report and **drawn as nothing**: fog, point
lights, clouds, track particle emitters, sound emitters, visibility sets (PVS), the Zone variant
(see [Zone mode on a track file](#zone-mode-on-a-track-file)). Keyframed node motion is carried
since 2026-10-29 (above); texture animation is not.

A file with **no art section** races on flat-shaded floor and wall collision, with no sky, sun
or lens flare, and its report says so. With one, the loader (`oag_raceplay::foreign::art`)
draws:

| Part | How |
| --- | --- |
| Art meshes | One draw each, vertices in file order, in the draw list its blend class names, each list ordered by `oag_layer` and then file order (a stable sort, `Model::sort_by_layer`), as the Pulse/Pure track model (`oag_mesh::mesh::Model`) is built. `oag_alpha_ref` is the draw's own alpha-test reference (`DrawCall::alpha_test_ref`, per draw as on the disc). A mesh whose texture does not resolve is **not drawn**, and the report counts it; if no mesh draws (a Pulse export raced on a Pure host), the collision is drawn flat-shaded. The model sphere (far plane, framing) is never smaller than the collision's. With `[art] draw_collision = true` the flat-shaded collision is drawn too, merged under the art into one model (`foreign::art::under_collision`, `oag_mesh::mesh::merge`), and the report says so: for a track whose art does not cover the road. |
| Pad plates | A mesh tagged `oag_pad` is drawn in the host's pad model of that kind (`Loaded::pad_model`, `weapon_pad_model`) instead of the track model, built the same way, so the disc's rule applies: a weapon plate is decoded in every mode and drawn only where the mode has weapons (`Scene::new`; the report says `drawing the track file's weapon pad plates` or `decoded but not drawing (weapons off in this mode)`). Visual only: what a pad does is `oag/pads/*`. A plate an animation moves is refused. A disc export writes each circuit's own plates (`mesh::build_pads`, `build_weapon_pads`) tagged, over the same texture rows. **The host's own pad art (2026-10-29, `foreign::art::host_pads`)**: a kind whose plates carry no texture (the add-on's `Place Pad Plates` with no material, which are coloured quads) or that has volumes and no plate at all takes the donor circuit's first pad of that kind (`mesh::build_pads_placed`, `build_weapon_pads_placed`) drawn once on each volume's frame, lifted 0.3 along its up (chosen, not measured: the add-on's default plate lift): the disc's texture, texture coordinates, blend, layer and texture animation, one `node_vertex_ranges` entry per volume so the weapon pads' colour cycle runs. The donor is the first circuit the art names (a host texture, then its sky), else the title's default circuit. A textured plate (a disc export's) stands untouched. A donor with more than one distinct pad payload of the kind (Pure's `01_Vineta_K` has 3 speed pads) draws its first node's and the report says so. A donor that authors none, will not read, or keeps its pads in a `.rcsmodel` draws nothing and the report names why. Pulse PSP and PS2 and Pure: **ported** (equal to the circuit's own first pad on every circuit of the image, `trackfile_ground_truth`); HD, 2048, Omega: **checked, applies, not wired** (their pads are baked in world space in the `.rcsmodel`, `mesh::rcs::pads`, so there is no pad-local template to place). |
| Textures | `image`: the PNG beside the manifest. `host`: the named node of that circuit, decoded as the race loader decodes it (the PS2's external texture set included). A miss is named by entry and name; nothing stands in. Rows no mesh or flare names are not resolved. |
| Sky | The named circuit's `Skycube`, through the disc circuit's own sky path. |
| Colours | `COLOR_0` is converted once, by sRGB's piecewise curve, to the gamma bytes the renderer draws (`byte / 255`), **rounded** to the nearest byte (chosen, not measured: the curve's last-bit error would drop an exact byte a step). Alpha is on no curve and is quantised the same way. A mesh with `oag_lit` set takes no byte grid: its colour is no PSP vertex colour (the `.vex` path gives such a vertex an invented grey no byte spells), so it is the curve's gamma float. All of it is `oag_raceplay::foreign::art::colour`, render-side: `oag-trackfile` stores the linear numbers untouched and does no `powf` (`check-determinism`). |
| Sun | Its direction is the `mapped` shadow tier's when `shadows` is set; otherwise that tier keeps the host title's axis. |
| Hull lights | Pulse and Pure on the PSP (the host title's `hull_lights` rule): the GE rig (`oag_mesh::mesh_render::HullLights`) from `[art.ambient]`, the sun as directional 0 and the directional `[[art.light]]` rows after it, four at most, each colour times its intensity converted to its gamma byte and clipped to `255`, built at the bare scale as `HullLights::from_track` builds a circuit's; a direction is taken as written. The craft's own darkening is the spline's `_OAG_LIGHT_SCALE` bytes, read through `oag_race::Course::light_scale_at` as on a disc circuit. An HD-family host's authored-light equation (`Light::authored`) is not fed the sun yet. |
| Lens flare | Pure's flare (`oag_fx::lens_flare`) along the sun, or its own `direction`, in the sun's colour unless the flare names one (converted from linear to the gamma float the ring law steps), with the atlas row's picture. |
| Motion | Each moving mesh becomes a node of the model's `anim_nodes` table, its vertices tagged with the slot (`GpuVertex::xform`), so the per-frame node matrices, the unculled moving-draw path and the `mapped` tier's moving casters take it as they take a disc circuit's `Anim Transform`. Meshes with identical rest pose and channels (one Blender object's material slots) share a node; past the table's 384 the rest stand at their rest pose, reported. The model sphere reaches every moving vertex at each key time. A turning sun turns the `mapped` shadows' direction and a flare without its own direction (`SceneryFx::sun`, `lens_flare_direction`); the hull rig keeps the pose at time zero. The loader report says `art motion: N animation(s), M mesh(es) on K node(s)`. |

What a mesh carries and the disc's draws have more of, so is lost by an export: texture
animation, geometry motion (an `Anim Transform`'s vertices are written at their time-zero
pose), level of detail (the finest tier is written), the per-frame depth order of list B
(`DrawCall::depth_point`), the PSP glow stamp, the extra (shine) pass, point lights (the hull rig's
`wopoint`/`wospot` slots), and the gantry's live panel, which is not part of the track model. The
render layer, the per-batch alpha-test reference and every `DirectionalLight` and `AmbientLight`
travel (since 2026-10-29); the sun casts nothing on a disc export, whose shadows follow the host
title's own axis. The pad plates travel too (since 2026-10-29, `oag_pad`).

## Census and round trip (2026-10-11)

| Check | Result |
| --- | --- |
| Disc circuits exported, written, read back, compared field by field (`trackfile_ground_truth`) | **80 of 80**: Pulse PSP EU, PSP USA and PS2 EU, every circuit of each `PI_Track` list, the reversed ones included. `ai`, `start`, both pad lists, every collision node's kind, name, version, vertices, triangles and vertex scalars equal; a second write of what was read is byte-identical. |
| Other titles' default circuits | Pure PSP EU, HD/Fury PS3 EU, Omega PS4 EU, a 2048 DLC `.vpk` and the 2048 base game (`altima`, from `data/extracted/vita/PCSF00007`) export and read back the same. The base `.pkg` itself is PFS-encrypted and is not read in place (`oag-disc` refuses it by name); `just play 2048` and this check read the package decrypted and extracted offline with its zRIF (`data/keys/README.md`), which is how its base circuits are checkable (2026-10-29). |
| Art proof (ADR-0076), `every_*_circuit_exports_and_reads_back_the_same`, one test per image (2026-10-29) | **96 of 96** circuits: Pulse PSP EU and USA (24 each), PS2 EU (32), Pure PSP EU and USA (8 each). The track model rebuilt from the exported, written and read-back art equals the circuit's own (finest LOD tier): every draw in each list's order, its vertices' position, normal, colour, texture coordinate and `lit` flag bit for bit, its triangles, texture slot, blend class, culled flag and bounds. 162,366 draws, 17.4 M vertices, 16.1 M triangles; 12,718 used texture slots resolved by name and equal pixel for pixel (38 share an asset path with another node and still resolve to the same pixels); 96 skies; 14 Pure flares equal the circuit's own node and atlas. Since 2026-10-29 (landing 2a), through linear `COLOR_0` and back: each draw's layer and alpha-test reference too; the hull rig built from the file equals `HullLights::from_track` on the circuit on **96 of 96**; the whole vertex and index buffers equal the `.vex` build's on 76, and on the other 20 (the circuits with `LodGroup`s: Pulse PS2 13 and 16, PSP 03, 10, 13 and 16, forward and reversed) once the coarser tiers' vertices and indices, which `keep_nearest` leaves behind with no draw reading them, are dropped. All 14 Pure flares lie off the circuit's first directional light, so each carries its own `direction`. Since 2026-10-29 (`motion-pads`) the pad plates: the speed and weapon pad models built from the meshes tagged `oag_pad` equal `mesh::build_pads` and `mesh::build_weapon_pads` on the circuit draw for draw, by the same per-draw comparison and texel for texel (Pulse PSP EU 24 and 24, 1,701 draws; Pure EU 8 and 8, 194 draws). Raced from the file, 03_Track and 16_Track's pad models are the circuit's in vertices and triangles, a weapon plate lies within 10 units of every weapon pad volume, and Time Trial decodes the weapon plates without drawing them. |
| Workshop `Original` tracks (27) mapped, written, read back | 27 of 27 equal. |
| Lone AI craft, native circuit vs its exported file, world hash every tick, 3,600 ticks | identical on `03_Track` (Moa Therma) and `16_Track`; the hull rig the file's light builds equals the native load's (`loaded.light.hull`). |
| Lion's Den, `--ballisticng-track` vs its converted file, 3,600 ticks | identical. |
| The committed fixture (`tests/fixtures/oval.*`, a rounded octagon, 64 control points) | a lone AI craft gets round it: lap 4 in 7,200 ticks, no respawns, on Pulse's physics. |

**What does not survive a disc round trip**, all by construction:
`CollisionNode::node_index` (a position in a `.vex` tree; becomes the node's list position),
`CollisionMesh::chunks` (byte ranges into the disc payload; empty), and what the `.vex`
authors that the format has no field for (the absent list, and the art losses above). The
simulation reads none of them, which is why the world hash agrees with the art present too.

**Where the proof is not bit for bit, and why.** The buffers of a circuit with level of detail:
the art carries the finest tier only, the `.vex` build keeps the coarser tiers' vertices and
indices after dropping their draws, so the proof holds those circuits to buffer equality with the
leftovers removed. A draw under an `Anim Transform` (9,193 of them) is bounded by the
box of its placed vertices; the `.vex` build boxes it in anchor space and lifts only the
centre, a sphere up to about 8% wider or narrower, so these are compared against the box of
the reference's placed vertices instead.

## Cross-title (2048 and Omega)

The model is the loader's, which every title reaches through the same `track_inputs::resolve`:
**ported** for Pure, HD, Omega and 2048 (the default-circuit round trip above, each with its own
collision source, 2048's through `track_col.col`). Reading a file back races on a host title's
physics; HD, 2048 and Omega as hosts are not exercised (a Pulse image hosted every race).

The art section: **checked, applies, not wired** for HD, 2048 and Omega. Their circuits draw
from the `.rcsmodel` beside the `.vex` (HD's container, and 2048's re-cooked for Omega), whose
materials carry a shader table, lightmaps and blend factors the art material does not have a
field for; the exporter writes no art for them and says so in the notes
(`art not exported: this circuit's geometry is in its .rcsmodel`), and a host reference names
only a `.vex` `Texture` node. 2048 and Omega share that material model, so one lane serves both.

## How to race one

`oag-game --track-file <manifest>` and the RACE REMIX route below both work in every build, with or without the
`ballisticng` feature (CUSTOM lists own-format tracks either way; see
[menus.md](../architecture/menus.md#race-remix-custom-tracks)).

Put the `*.track.toml` and its `.glb` in a track directory (`$OAG_TRACKS`, `data/tracks`, `tracks/` beside the program,
`<data dir>/oag/tracks`; one level of subdirectory is read too), then RACE REMIX, TRACK TITLE
`CUSTOM`, TRACK `<title> (TRACK FILE)` (the manifest's own title), START
([menus.md](../architecture/menus.md#race-remix-custom-tracks)); or `oag-game --track-file <manifest>`.
A folder mod's `tracks/*.track.toml` is listed on CUSTOM the same way, on the titles the
mod is for, and a campaign can race it ([mods.md](../tools/mods.md#campaigns-pilots-and-tracks-campaigns-pilots-tracks)).
A manifest or glb that will not read is listed dimmed with the reader's reason. The format carries no
author field, so none is shown.

A track that borrows nothing from a disc (only its own images, or no art) races on any title. One
whose art borrows from a disc (`track.host`) races with that disc as the track source: on its host's
own title natively, on any other as a Race Remix, which the game arranges itself when this machine has
a disc of the host (`--track-file` on an HD disc with a Pulse disc present races Pulse's track side
under HD's craft and says so in the log). With no disc of the host the race is refused by name. RACE
REMIX's CUSTOM always takes the track side from the host's disc and lists a file whose host this
machine has no disc of dimmed. `track.titles`, when present, limits the titles whose craft may race
it.

## Zone mode on a track file

`--mode zone` on a track file, RACE REMIX's MODE Zone on CUSTOM and a campaign `zone` event with
`file` all race the file's own loop and spline as the Zone circuit (2026-10-29; before it the loader
refused it by name). A disc keeps a Zone circuit's look in a file of its own (Pulse's `zone_` variant,
Pure's and HD's separate circuits), and a track file has no such file, so **nothing of that is borrowed
from a disc circuit and nothing is invented**:

| Part | On a track file |
| --- | --- |
| Geometry, spline, collision, start | the file's own |
| Craft, handling, speed ramp (the title's `<Zone>` start, increment and recharge), Zone HUD, ship and speech sound banks, stage names | the host title's own Zone ones, as for any Zone race |
| Zone circuit environment (a circuit's Zone meshes, lights, fog, `Skycube`) | drawn as nothing; the file's own art and sky, if it has them, stand |
| HD's `ZoneSky.gtf` (a title-wide Zone default) | drawn when the file carries no sky of its own; a file with its own sky keeps it and the report says the Zone sky is not drawn over it |
| HD's stage grade (`zonemode.effectsettings`, per-stage textures; title-wide) | applied over the file's model, as on an HD Zone circuit |
| The title's Zone start-gantry board, track sound emitters | not placed (a file has no gantry node or emitters; a gantry in its own art stands) |

The report carries `zone: a track file has no Zone variant, ...` and the schema's absent list.
Ground truth: `trackfile_ground_truth::a_zone_race_on_a_track_file_*` (Pulse: the autopilot flies the
ramp to zone 24 round the fixture and the example Octagon with no respawn; HD: the Zone sky and grade).
The autopilot is not a physics reference: it only shows the loop can be flown. On the Octagon it
clears zone 24 in a little over four minutes and wrecks at zone 626 (about 104 minutes).

**Cross-check, 2048 and Omega: ported.** `a_zone_race_on_a_track_file_loads_and_climbs_on_every_title`
loads the oval in Zone on Pure, HD, Omega, the 2048 base package and Pulse's PS2 disc, asserts the
report line and the title's `<Zone>` ramp, and the autopilot's zone counter climbs. Each title flies its
own Zone craft, ramp and HUD; only HD has a Zone sky or a title-wide grade to draw (a test of its own).
Checked, differs: Omega's and 2048's per-circuit Zone environments are not something a file can name,
so a file gets nothing of them (drawn as nothing, as above).

## Open

- With the autopilot on a track file, HD, Pure and Pulse's PS2 disc respawn the craft once at tick 44,
  Pulse's PSP discs do not (a time trial shows it too, so not Zone-specific; Omega and 2048 also show one
  respawn within 7,000 ticks, their tick not measured). Not investigated.
- Authoring: [the Blender add-on](../tools/blender.md) writes this form (2026-10-12). Blender 5.2.2's own
  glTF importer cannot open the file (it raises on the pad `_OAG_TO_WORLD` `MAT4`); the add-on has its
  own importer. Banking and per-point width are not authored yet.
- Motion on a disc export: an `Anim Transform`'s geometry is still written at its time-zero pose;
  writing it as an animation is open.
- `oag_shadow` is not honoured by the `mapped` tier's moving casters (every opaque moving mesh casts).
- The absent list above, and what an export loses (texture animation, motion, level of detail,
  list B's depth order, glow, point lights), as optional records in a later revision.
- An HD-family host feeding `[art.sun]` and `[art.ambient]` to its own authored-light equation
  (`Light::authored`); Pulse and Pure on the PSP light hulls from them since 2026-10-29.
