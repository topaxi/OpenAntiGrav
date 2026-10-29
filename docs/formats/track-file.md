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
directory. Raced with `oag-game --track-file <manifest>` (host title's ship, rules, physics). Authored in Blender with [the add-on](../tools/blender.md).

### Manifest (TOML)

| Key | Meaning |
| --- | --- |
| `[toml-schema]` | `location` ending `v1/track.tosd`, `version = "1.0.0"`: the kind and major, per [ADR-0064](../architecture/adr/0064-project-toml-files-declare-a-toml-schema.md) and [TOML Schema](toml-schema.md). Missing, another kind or an unknown major: refused by name. This replaced `track.format` and `track.version`; there is no minor. A key the reader does not know is read and reported as unread. |
| `track.id`, `track.title` | Identifier (keys records and ghosts as `foreign:<id>`) and display title. |
| `track.glb` | The glb's file name, relative to the manifest. |
| `track.notes` | The producer's free text (what it chose, ignored, could not map). Printed in the loader report. |
| `spline.point_version` | The `WO Track` point version (`SHORT_POINT_VERSION` = `0x107` = 263 for importer output). |
| `[[spline.path]]` | `points` (count, checked against the glb), `entry`/`exit` (junction index, `-1` none), `max_spacing_bits` (the `f32` bit pattern, an integer so exact). |
| `[[spline.junction]]` | `prev`, `next`: two path indices each, `-1` none (primary, then alternate). |
| `[counts]` | `collision_nodes`, `speedup_pads`, `weapon_pads`, checked against the glb; `art_meshes` only when the track has art. |
| `[art]` | Optional (ADR-0076). Absent, the file is byte for byte what 1.0 wrote. `sky`: a host circuit's `.vex` entry whose `Skycube` is drawn. `ambient`: linear RGB (read, not yet drawn). |
| `[art.sun]` | `direction` (unit, towards the sun), `colour` (linear RGB), `intensity`, `angular_diameter` (degrees, 0 a point sun), `shadows`. All required when the table is present. |
| `[art.lensflare]` | Pure's sun flare: `atlas` (an index into the texture rows), optional `colour` overriding the sun's. Refused without a sun. |
| `[[art.texture]]` | Exactly one of `image = "file.png"` (the author's picture beside the manifest; the only form a committed fixture may use) or `host = { entry = "...", name = "..." }` (a `Texture` node of a circuit on the host title's disc, by its runtime asset path, or by file name alone when `name` has no directory). |

No simulation float is in the manifest: a decimal float is an `f64` round trip. The art's
numbers are render-side only, so they are TOML floats (an integer reads too).

### Glb (glTF 2.0 binary)

One scene, flat nodes, tightly packed little-endian accessors. Right-handed, Y up, world units.

| Node / mesh name | Primitive | Attributes |
| --- | --- | --- |
| `oag/start` | one `POINTS` vertex | `POSITION`, `_OAG_LEFT`, `_OAG_UP`, `_OAG_FORWARD` (VEC3) |
| `oag/spline/<i>` | `POINTS`, one vertex per control point | `POSITION` (pos), `_OAG_TANGENT`, `_OAG_DOWN`, `_OAG_LATERAL` (VEC3), `_OAG_PROGRESS`, `_OAG_RACING_LINE` (SCALAR), `_OAG_HALF_WIDTH`, `_OAG_AI_BOUND` (VEC2 left, right), `_OAG_SECTION_FLAGS` (u8 VEC4: section, flags, 0, 0), `_OAG_LIGHT_SCALE` (u8 VEC4), and optionally `_OAG_AI_BYTES` (u8 VEC4: lane band low, lane band high, speed-floor byte, 1 when the point carries them; absent reads as none), and optionally `_OAG_CAGE_A`, `_OAG_CAGE_B` (u8 VEC4 each: the collision cage's eight signed bytes `+0x68..+0x6f` in order, two's complement; absent reads as all zero) |
| `oag/pads/speedup`, `oag/pads/weapon` | `POINTS`, one vertex per pad | `POSITION` (informational: the pad's world centre), `_OAG_TO_WORLD` (MAT4, the row-major row-vector matrix as stored), `_OAG_MIN`, `_OAG_MAX` (VEC3), `_OAG_DISABLED` (SCALAR) |
| `oag/collision/<n>` | `TRIANGLES`, u16 indices, one primitive per mesh | `POSITION`, `_OAG_SCALAR` (per-vertex scalar, `1.0` when absent); node extras `oag_kind` (`floor`, `wall`, `reset`, `mag_floor`, `cage`, `track_wall`), `oag_name`, `oag_version` |
| `oag/art/<n>` | `TRIANGLES`, one primitive per draw | `POSITION`, `NORMAL`, `TEXCOORD_0` (v down, as the disc's), `COLOR_0` (VEC4 float: the baked light, copied as the renderer holds it); node extras `oag_name`, `oag_texture` (texture row, absent for untextured), `oag_blend` (`opaque`, `cutout`, `alpha`, `additive`, `unblended`), `oag_culled`, `oag_lit`, `oag_shadow` (`both`, `cast`, `receive`, `none`) |

The cage bytes travel, but the `WO Track` header flag that turns the cage on
(`oag_vex::track::cage_flag`) has no field here, so a race on a track file runs no
collision cage ([collision-cage.md](../ghidra/functions/psp-pulse-usa/collision-cage.md#the-cage-is-a-contact-source-live-2026-10-20)).

A reader finds a collision node by its `oag_kind` extra (so a Blender object tagged with a
custom property qualifies whatever it is named), the rest by name. Mesh nodes without either
are counted as unread, never dropped silently. A non-identity node transform on a record is
refused (`Transformed`): bake it before export.

## What format 1.x carries no field for

`oag_trackfile::ABSENT`, named in every loader report and **drawn as nothing**: fog, point
lights, clouds, scenery animation, track particle emitters, sound emitters, visibility sets
(PVS), the Zone variant.

A file with **no art section** races on flat-shaded floor and wall collision, with no sky, sun
or lens flare, and its report says so. With one, the loader (`oag_raceplay::foreign::art`)
draws:

| Part | How |
| --- | --- |
| Art meshes | One draw each, in file order, in the draw list its blend class names, as the Pulse/Pure track model (`oag_mesh::mesh::Model`) is built. A mesh whose texture does not resolve is **not drawn**, and the report counts it. |
| Textures | `image`: the PNG beside the manifest. `host`: the named node of that circuit, decoded as the race loader decodes it (the PS2's external texture set included). A miss is named by entry and name; nothing stands in. Rows no mesh or flare names are not resolved. |
| Sky | The named circuit's `Skycube`, through the disc circuit's own sky path. |
| Sun | Its direction is the `mapped` shadow tier's when `shadows` is set; otherwise that tier keeps the host title's axis. The hull lighting stays the host's: the authored rig (`Light::authored`) is HD's equation, and switching it on would relight Pulse's hulls (chosen, 2026-10-29). |
| Lens flare | Pure's flare (`oag_fx::lens_flare`) along the sun, in the sun's colour unless the flare names one, with the atlas row's picture. |
| Ambient | Read and reported, not drawn (same reason as the hull lighting). |

What a mesh carries and the disc's draws have more of, so is lost by an export: texture
animation, geometry motion (an `Anim Transform`'s vertices are written at their time-zero
pose), level of detail (the finest tier is written), the render layer and per-frame depth
order (meshes are written in the order the lists dispatch), the per-batch alpha-test
reference (the shader's default stands), the PSP glow stamp, the extra (shine) pass, and the
gantry's live panel, which is not part of the track model.

## Census and round trip (2026-10-11)

| Check | Result |
| --- | --- |
| Disc circuits exported, written, read back, compared field by field (`trackfile_ground_truth`) | **80 of 80**: Pulse PSP EU, PSP USA and PS2 EU, every circuit of each `PI_Track` list, the reversed ones included. `ai`, `start`, both pad lists, every collision node's kind, name, version, vertices, triangles and vertex scalars equal; a second write of what was read is byte-identical. |
| Other titles' default circuits | Pure PSP EU, HD/Fury PS3 EU, Omega PS4 EU, a 2048 DLC `.vpk` and the 2048 base game (`altima`, from `data/extracted/vita/PCSF00007`) export and read back the same. The base `.pkg` itself is PFS-encrypted and is not read in place (`oag-disc` refuses it by name); `just play 2048` and this check read the package decrypted and extracted offline with its zRIF (`data/keys/README.md`), which is how its base circuits are checkable (2026-10-29). |
| Art proof (ADR-0076), `every_*_circuit_exports_and_reads_back_the_same`, one test per image (2026-10-29) | **96 of 96** circuits: Pulse PSP EU and USA (24 each), PS2 EU (32), Pure PSP EU and USA (8 each). The track model rebuilt from the exported, written and read-back art equals the circuit's own (finest LOD tier): every draw in each list's order, its vertices' position, normal, colour, texture coordinate and `lit` flag bit for bit, its triangles, texture slot, blend class, culled flag and bounds. 162,366 draws, 17.4 M vertices, 16.1 M triangles; 12,718 used texture slots resolved by name and equal pixel for pixel (38 share an asset path with another node and still resolve to the same pixels); 96 skies; 14 Pure flares equal the circuit's own node and atlas. |
| Workshop `Original` tracks (27) mapped, written, read back | 27 of 27 equal. |
| Lone AI craft, native circuit vs its exported file, world hash every tick, 3,600 ticks | identical on `03_Track` (Moa Therma) and `16_Track`. |
| Lion's Den, `--ballisticng-track` vs its converted file, 3,600 ticks | identical. |
| The committed fixture (`tests/fixtures/oval.*`, a rounded octagon, 64 control points) | a lone AI craft gets round it: lap 4 in 7,200 ticks, no respawns, on Pulse's physics. |

**What does not survive a disc round trip**, all by construction:
`CollisionNode::node_index` (a position in a `.vex` tree; becomes the node's list position),
`CollisionMesh::chunks` (byte ranges into the disc payload; empty), and what the `.vex`
authors that the format has no field for (the absent list, and the art losses above). The
simulation reads none of them, which is why the world hash agrees with the art present too.

**Where the proof is not bit for bit, and why.** Each draw's block of vertices sits elsewhere
in the buffer: the `.vex` build appends batches in node order and then sorts each list by a
render layer the format does not carry, so a file that keeps the dispatch order cannot also
keep the buffer offsets. A draw under an `Anim Transform` (9,193 of them) is bounded by the
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
A manifest or glb that will not read is listed dimmed with the reader's reason. The format carries no
author field, so none is shown.

## Open

- Authoring: [the Blender add-on](../tools/blender.md) writes this form (2026-10-12). Blender 5.2.2's own
  glTF importer cannot open the file (it raises on the pad `_OAG_TO_WORLD` `MAT4`); the add-on has its
  own importer. Banking and per-point width are not authored yet.
- The Blender add-on's art export (ADR-0076's second landing).
- The absent list above, and what an export loses (texture animation, motion, level of detail,
  draw order, alpha-test reference, glow), as optional records in a later revision.
- The hull lighting and the ambient from `[art.sun]`/`[art.ambient]`: needs the host title's own
  hull lighting law, not HD's rig.
