# The Blender add-on: author a track, export it, race it

**Status: built (2026-10-12).** Blender 4.2 or later; developed and tested on 5.2.2 LTS, headless.
The add-on writes this project's own track format ([track-file.md](../formats/track-file.md),
[ADR-0063](../architecture/adr/0063-the-own-track-format-is-glb-plus-a-toml-manifest.md)) and
nothing else: every field it writes is a field the format defines. Where it derives a value
the author did not give, the derivation is **chosen, not measured** and listed below.

Source: `scripts/blender/oag_track/` (an extension: `blender_manifest.toml`, plain Python, no
dependency). It lives under `scripts/` beside the other tooling; the format code (`trackfile.py`,
`derive.py`, `f32.py`) imports no `bpy`, so it can be read and tested without Blender.

## Install

1. Zip the folder: `cd scripts/blender && zip -r oag_track.zip oag_track`.
2. Blender: Edit > Preferences > Get Extensions > Install from Disk, pick the zip.
3. The panel is in the 3D view sidebar (N), tab **OAG**.

Check the manifest without installing anything: `blender --command extension validate scripts/blender/oag_track`.

## Author a track

Everything in the scene that is **tagged** is exported, and nothing else. A tag is the custom
property `oag_kind` on the object (the panel's buttons set it; so does adding the property by hand).

| `oag_kind` | Object | What it becomes |
| --- | --- | --- |
| `spline` | a closed (cyclic) Curve, or a Mesh of baked points | the AI line / racing spline: one record per sampled point |
| `floor`, `wall`, `reset`, `mag_floor`, `cage`, `track_wall` | a Mesh | a collision node of that kind, triangulated, transforms baked |
| `start` | an Empty: +Y forward, +Z up | the start slot |
| `speedup`, `weapon` | an Empty: X lateral, +Y forward, +Z up | a pad |

The six collision kinds and the four above are the whole list: the format has no `scenery`
kind (a reader refuses an unknown tag by name), so scenery, sky, fog, lights and art are not
exported ([the absent list](../formats/track-file.md#what-format-1x-carries-no-field-for)).
An untagged object is simply not part of the track.

Steps (the same ones `scripts/blender/roundtrip.py loop` runs headless):

1. Draw a **cyclic Curve** (Bezier or poly, any number of control points) and tag it `spline`.
   In the panel set *Spline points* (default 64), *Half width* (default 20) and *AI line
   fraction* (default 0.6). The add-on samples the curve evenly by arc length.
2. **Build Floor and Walls** makes a tagged `floor` mesh and a `wall` mesh (inner and outer,
   *Wall height* default 15) along the spline. Edit or replace them: any tagged mesh works,
   and nothing requires them to follow the spline.
3. **Add Start** puts the start slot 154 units behind spline point 0 (see below); move or
   rotate the Empty to place it elsewhere.
4. **Add speedup pad** / **Add weapon pad** put a pad on the spline point nearest the 3D
   cursor. The pad's box (`oag_min`, `oag_max`, in the format's pad-local axes lateral, up,
   forward) defaults to the fixture's.
5. Set the track id, title and notes at the top of the panel, then **Export OAG Track**
   and choose `name.track.toml`; `name.glb` is written beside it.

### What the add-on derives (chosen, not measured)

All of it follows `crates/trackfile/src/fixture.rs`, in `f32`, so a flat loop authored here
matches what that generator would write.

| Field | Rule |
| --- | --- |
| Spline `pos` | the curve sampled at equal arc length, around the ring |
| `tangent` | the unit chord from the previous point to the next |
| `lateral`, `down` | `lateral` = unit(tangent x world up); `down` = -up, with up = lateral x tangent (the lateral axis stays level, up follows a slope; **banking is not authored**) |
| `progress` | cumulative gap over the total, 0 at point 0 |
| `half_width`, `ai_bound` | the object's half width; AI bound = -/+ half width x AI fraction |
| `racing_line`, `section`, `flags`, `light_scale` | 0, `0xff`, 0, 255 |
| path `entry`/`exit`, junction | `0`/`0` and one junction `[0, -1]` -> `[0, -1]`: a single closed loop |
| `max_spacing` | the largest gap between neighbours |
| Start slot | 154 units back along the ring from point 0, level, facing along the line. The race puts the **lap line** 154 units ahead of the start slot (`oag_race::Course::START_LINE_ADVANCE`), so spline point 0, the curve's first control point, is the lap line |
| Pad matrix | the Empty's unit basis in rows (lateral, up, forward) and its position; scale is ignored, the size is the box |
| Track id | the scene's id, else the scene name slugged |

"Tiles", "AI line" and "start grid" in other tools map to fields that exist here: the pad lists,
the spline with its `ai_bound`/`racing_line`, and the start slot (the engine lays its own grid from it).

Axes: Blender is Z-up, the format Y-up, `(x, y, z)_blender = (x, -z, y)_format`. A permutation and a
negation, so exact `f32` values and winding survive in both directions. Mesh transforms are baked
into the vertices on export (`matrix_world`; an identity transform is not multiplied, so bits do not move).
A mesh over 65,535 vertices is split into several collision meshes of one node, because the format's
indices are `u16`.

## Import, and baked records

**Import OAG Track** reads a `.track.toml` + `.glb` into tagged objects. A spline arrives as a
Mesh of baked points (the `oag_*` attributes: tangent, down, lateral, progress, half width, AI
bound, racing line, section, flags, light scale), pads and the start as Empties, collision as one
object per collision mesh (`oag_node`, `oag_mesh`, `oag_name`, `oag_version`, and a per-vertex
`oag_scalar`). Exporting again writes **baked records back verbatim**: a spline Mesh is never
resampled, and a pad or start Empty that has not been moved writes its exact stored matrix
(`oag_to_world`, `oag_frame`; Blender decomposes a matrix into location, rotation and scale,
which is not bit-exact in `f32`). Move one and it is derived from the Empty like an authored one.

Blender's own glTF importer **cannot** open these files: Blender 5.2.2 raises on the pad
accessor `_OAG_TO_WORLD` (`MAT4`) while creating `oag/pads/speedup`, so nothing after it is
made. The add-on's importer is the way in. (The glb is also written by this add-on directly,
not through Blender's glTF exporter, which re-splits vertices and cannot emit the `u8`/`MAT4`
accessors the format needs.)

## Race it

Put `name.track.toml` and `name.glb` in `tracks/` under a directory `--ballisticng-track` searches
(`$OAG_BALLISTICNG`, `data/ballisticng`, beside the program, `<data dir>/oag/ballisticng`), then RACE REMIX,
TRACK TITLE `CUSTOM`, TRACK `<title> (TRACK FILE)`, START
([menus.md](../architecture/menus.md#race-remix-custom-tracks)); or
`oag-game --track-file name.track.toml`. The race uses the host title's ship, rules and
physics, and draws flat-shaded collision. A file that will not read is listed dimmed with the reason.

## Proof (2026-10-12)

| Check | Result |
| --- | --- |
| `just blender-fixtures` rebuilds the two fixtures headless and compares them to the committed bytes | up to date, twice (the output is deterministic) |
| `oval.track.toml` imported by the add-on and exported again (`oval.reexport.*`) | `read(oval) == read(oval.reexport)` and every spline, vertex, pad, start and `max_spacing` float is bit-equal (`crates/trackfile/tests/blender.rs`) |
| A loop built from scratch in Blender: 8 Bezier control points, 64 samples, floor, walls, start, one pad of each kind (`blender_loop.*`) | reads, nothing unread, schema-valid (`schema.rs`) |
| Lone AI craft on Pulse's physics, 7,200 ticks (`trackfile_ground_truth`) | `blender_loop`: lap 3, no respawns; the re-exported oval: lap 4, no respawns (the committed oval: lap 4) |
| `oag-game --race --autopilot --track-file blender_loop.track.toml` | the player gets round at 127 speed; frames at ticks 400, 1200, 2400, 3600 read at 1280x720 |

The Blender-driven part cannot run in CI (no Blender), so it is a local recipe whose output is
committed: `just blender-fixtures` (`--update` rewrites the fixtures). The gate reads the committed
files, so dropping the add-on's spline, start or ribbon fails a Rust test, and a stale fixture
shows as a diff the next time the recipe runs. A different Blender version may sample a curve
differently; regenerate with `--update` and review the diff.

## Open

- Banking (a per-point tilt feeding `up`/`lateral`) and a per-point width; both need an authoring
  surface on the curve.
- More than one path and junction authoring beyond the scene property `oag_junctions` (a JSON list
  of `{"prev": [a, b], "next": [a, b]}`, `-1` for none) that the importer writes.
- Visual meshes and materials: not in format 1.x ([track-file.md](../formats/track-file.md#open)).
