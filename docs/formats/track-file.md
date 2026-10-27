# The track file (this project's own custom-track format)

**Status: designed and built (2026-10-11).** Decision and rationale:
[ADR-0063](../architecture/adr/0063-the-own-track-format-is-glb-plus-a-toml-manifest.md).
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
| `[counts]` | `collision_nodes`, `speedup_pads`, `weapon_pads`, checked against the glb. |

No simulation float is in the manifest: a decimal float is an `f64` round trip.

### Glb (glTF 2.0 binary)

One scene, flat nodes, tightly packed little-endian accessors. Right-handed, Y up, world units.

| Node / mesh name | Primitive | Attributes |
| --- | --- | --- |
| `oag/start` | one `POINTS` vertex | `POSITION`, `_OAG_LEFT`, `_OAG_UP`, `_OAG_FORWARD` (VEC3) |
| `oag/spline/<i>` | `POINTS`, one vertex per control point | `POSITION` (pos), `_OAG_TANGENT`, `_OAG_DOWN`, `_OAG_LATERAL` (VEC3), `_OAG_PROGRESS`, `_OAG_RACING_LINE` (SCALAR), `_OAG_HALF_WIDTH`, `_OAG_AI_BOUND` (VEC2 left, right), `_OAG_SECTION_FLAGS` (u8 VEC4: section, flags, 0, 0), `_OAG_LIGHT_SCALE` (u8 VEC4), and optionally `_OAG_AI_BYTES` (u8 VEC4: lane band low, lane band high, speed-floor byte, 1 when the point carries them; absent reads as none), and optionally `_OAG_CAGE_A`, `_OAG_CAGE_B` (u8 VEC4 each: the collision cage's eight signed bytes `+0x68..+0x6f` in order, two's complement; absent reads as all zero) |
| `oag/pads/speedup`, `oag/pads/weapon` | `POINTS`, one vertex per pad | `POSITION` (informational: the pad's world centre), `_OAG_TO_WORLD` (MAT4, the row-major row-vector matrix as stored), `_OAG_MIN`, `_OAG_MAX` (VEC3), `_OAG_DISABLED` (SCALAR) |
| `oag/collision/<n>` | `TRIANGLES`, u16 indices, one primitive per mesh | `POSITION`, `_OAG_SCALAR` (per-vertex scalar, `1.0` when absent); node extras `oag_kind` (`floor`, `wall`, `reset`, `mag_floor`, `cage`, `track_wall`), `oag_name`, `oag_version` |

The cage bytes travel, but the `WO Track` header flag that turns the cage on
(`oag_vex::track::cage_flag`) has no field here, so a race on a track file runs no
collision cage ([collision-cage.md](../ghidra/functions/psp-pulse-usa/collision-cage.md#the-cage-is-a-contact-source-live-2026-10-20)).

A reader finds a collision node by its `oag_kind` extra (so a Blender object tagged with a
custom property qualifies whatever it is named), the rest by name. Mesh nodes without either
are counted as unread, never dropped silently. A non-identity node transform on a record is
refused (`Transformed`): bake it before export.

## What format 1.x carries no field for

`oag_trackfile::ABSENT`, named in every loader report and **drawn as nothing**: sky, fog,
lights, clouds, art meshes and materials, scenery and its animation, track particle
emitters, sound emitters, visibility sets (PVS), the Zone variant. A track file races on
flat-shaded floor and wall collision.

## Census and round trip (2026-10-11)

| Check | Result |
| --- | --- |
| Disc circuits exported, written, read back, compared field by field (`trackfile_ground_truth`) | **80 of 80**: Pulse PSP EU, PSP USA and PS2 EU, every circuit of each `PI_Track` list, the reversed ones included. `ai`, `start`, both pad lists, every collision node's kind, name, version, vertices, triangles and vertex scalars equal; a second write of what was read is byte-identical. |
| Other titles' default circuits | Pure PSP EU, HD/Fury PS3 EU, Omega PS4 EU and a 2048 DLC `.vpk` (`altima`) export and read back the same. The 2048 base `.pkg` is PFS-encrypted and not readable in place (checked, not checkable for its base circuits). |
| Workshop `Original` tracks (27) mapped, written, read back | 27 of 27 equal. |
| Lone AI craft, native circuit vs its exported file, world hash every tick, 3,600 ticks | identical on `03_Track` (Moa Therma) and `16_Track`. |
| Lion's Den, `--ballisticng-track` vs its converted file, 3,600 ticks | identical. |
| The committed fixture (`tests/fixtures/oval.*`, a rounded octagon, 64 control points) | a lone AI craft gets round it: lap 4 in 7,200 ticks, no respawns, on Pulse's physics. |

**What does not survive a disc round trip**, all by construction:
`CollisionNode::node_index` (a position in a `.vex` tree; becomes the node's list position),
`CollisionMesh::chunks` (byte ranges into the disc payload; empty), and everything the `.vex`
authors outside the model: the circuit's art, sky, fog, lights, emitters, visibility sets,
animation, sound emitters and Zone variant. The simulation reads none of them, which is why
the world hash agrees; a loaded track file is drawn as flat-shaded collision, not as the
circuit.

## Cross-title (2048 and Omega)

The model is the loader's, which every title reaches through the same `track_inputs::resolve`:
**ported** for Pure, HD, Omega and 2048 (the default-circuit round trip above, each with its own
collision source, 2048's through `track_col.col`). Reading a file back races on a host title's
physics; HD, 2048 and Omega as hosts are not exercised (a Pulse image hosted every race).

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
- Visual meshes, materials and the absent list above, as optional records in a minor version.
