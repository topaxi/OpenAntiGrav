# The Blender add-on: author a track, export it, race it

**Status: built (2026-10-12); the art section since 2026-10-29.** Blender 4.2 or later; developed and tested on 5.2.2 LTS, headless.
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
| `racing_line` | a closed Curve beside a spline (`oag_path` = the spline's index, default 0) | that spline's `racing_line` offsets, [hand-editable](#the-racing-line) |
| `floor`, `wall`, `reset`, `mag_floor`, `cage`, `track_wall` | a Mesh | a collision node of that kind, triangulated, transforms baked |
| `start` | an Empty: +Y forward, +Z up | the start slot |
| `speedup`, `weapon` | an Empty: X lateral, +Y forward, +Z up | a pad |

The six collision kinds and the four above are the whole list: the format has no `scenery`
kind (a reader refuses an unknown tag by name). An **untagged** mesh that renders is **art**
(see [Art](#art-meshes-textures-the-sun-and-baked-light) below); fog, point lights and scenery
animation are not in the format
([what it carries](../formats/track-file.md#what-format-1x-carries-no-field-for)).
An untagged object hidden from render (`hide_render`) is not part of the track.

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
5. Set the track id, title, **Runs on**, **Offered on** and notes at the top of the panel, then **Export OAG Track**
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
| `racing_line` | computed by default (minimum curvature inside the AI bound), or read off a racing-line curve you edit; see [The racing line](#the-racing-line) |
| `section`, `flags`, `light_scale` | `0xff`, 0, 255 (`light_scale` lowered under art when the spline sets *Cover light*, see [Art](#art-meshes-textures-the-sun-and-baked-light)) |
| path `entry`/`exit`, junction | `0`/`0` and one junction `[0, -1]` -> `[0, -1]`: a single closed loop |
| `max_spacing` | the largest gap between neighbours |
| Start slot | 154 units back along the ring from point 0, level, facing along the line. The race puts the **lap line** 154 units ahead of the start slot (`oag_race::Course::START_LINE_ADVANCE`), so spline point 0, the curve's first control point, is the lap line |
| Pad matrix | the Empty's unit basis in rows (lateral, up, forward) and its position; scale is ignored, the size is the box |
| Track id | the scene's id, else the scene name slugged |
| `track.host` | the scene's **Runs on** (`oag_track_host`: `none`, default, or one of the five title slugs): the title whose disc the art's host textures and sky resolve from; export refuses `none` when the art borrows ([ADR-0076](../architecture/adr/0076-a-track-file-carries-its-art-as-an-optional-section.md) decision 13). Import refuses an unknown value |
| `track.titles` | the scene's **Offered on** (`oag_track_titles`), comma-separated slugs; empty writes no key (every title) |

"Tiles", "AI line" and "start grid" in other tools map to fields that exist here: the pad lists,
the spline with its `ai_bound`/`racing_line`, and the start slot (the engine lays its own grid from it).

Axes: Blender is Z-up, the format Y-up, `(x, y, z)_blender = (x, -z, y)_format`. A permutation and a
negation, so exact `f32` values and winding survive in both directions. Mesh transforms are baked
into the vertices on export (`matrix_world`; an identity transform is not multiplied, so bits do not move).
A mesh over 65,535 vertices is split into several collision meshes of one node, because the format's
indices are `u16`.

## The racing line

`racing_line` is one number per spline point: a signed distance **along that point's own
`lateral` axis, positive to the driver's right**, from the spline point (the centre) to
the line the AI aims at (`oag_vex::track::Sample`, `oag_raceplay::spline::racing_line`:
the driven point is `pos - lift * down + racing_line * lateral`). The AI corridor
(`ai_bound_left`, `ai_bound_right`, also offsets along `lateral`) is cut around that line,
and Pulse's loader forces it to straddle the line by at least 0.1
([track.md](../formats/track.md#the-curve), "the load pass"). Before 2026-10-29 the add-on
wrote 0 here, so opponents took every bend down the middle. (Pulse's own circuits mostly
author 0 too, [track.md](../formats/track.md#wipeout-2048-authors-version-0x107-and-the-control-point-shrank-to-96-bytes);
a line off the centre is this add-on's own choice, not something the discs show.)

**Default: computed at export (chosen, not measured).** A spline drawn as a Curve gets the
minimum-curvature line inside its AI corridor, unless the spline's *Compute racing line*
(`oag_auto_racing_line`) is off, which writes the centre as before. The method is in the
docstring of `scripts/blender/oag_track/racing_line.py`: the sideways second difference of the
driven line over the square of its gap, integrated along the line (gaps re-measured on the
line itself, six fixed rounds), minimised inside the **middle 40 percent of the corridor**
(the box: 30 percent of the corridor's width in from each bound, never under 0.1) by an active
set over a sparse banded solve. Pure Python, no numpy, fixed order over doubles,
offsets rounded to `f32` at the end, so the same scene gives the same bytes on every machine.
A ring of more than 64 points is solved on every `m`-th point and joined by a cyclic cubic
(the sampled polyline has a kink at every vertex, which a minimiser would otherwise fit).
A line is only as good as the centre line is smooth.

*Why the middle 40 percent, not the whole corridor.* The original driver (`oag_ai::original`) measures
its lane across the room between the line and each bound, so a line near a bound leaves one
side with almost none. Measured on `blender_loop` (corridor +-12), headless, Pulse physics: a line
allowed to 0.1 from the bounds, and one allowed to 20 percent of the width in, both leave
the opponents' speed plan unable to learn a lap (`1 unresolved`, no lap in 256,000 steps);
25, 30 and 40 percent get round. 30 keeps a margin from that cliff. **Chosen, not measured**,
and it caps the gain: the line may use only +-7.2 of a +-12 corridor.

**By hand.** *Compute Racing Line* (panel, spline selected) makes, or replaces, one closed
poly **Curve** tagged `racing_line`, `oag_path` = the spline's index, one point per spline
point at its computed offset. It never renders. Edit it like any curve: move points
sideways (G, then X/Y along the track's width), add or delete points, or replace it with a
Bezier. Export reads it:

| What you did to the curve | What is written |
| --- | --- |
| nothing (every point where the operator put it) | the stored offsets, bit for bit |
| anything else | for each spline point, the curve cut by the plane through the point whose normal is its tangent (the cut nearest the point, going forward), the distance of the cut along the point's `lateral`, boxed to the same middle-40-percent box |

A curve that never crosses a spline point's cross-section is refused by name. Two racing-line
curves for one spline are refused. Delete the curve and the default is back. An imported
track whose file carries a nonzero `racing_line` arrives with the curve already made (a
file with the centre everywhere gets none), and a baked spline with no curve keeps the
imported records verbatim: **the computed default applies to Curve splines only**, so
importing and exporting a track never moves its line. To give an imported track a computed
line, select its spline and run *Compute Racing Line*.

Headless, the same two calls the panel makes: set `curve_obj.oag_auto_racing_line = False`
to opt out, or `bpy.ops.oag.compute_racing_line()` with the spline active to make the
editable curve (a builder that only wants the default needs neither).

## Art: meshes, textures, the sun and baked light

**Since 2026-10-29** the add-on writes and reads the optional art section
([ADR-0076](../architecture/adr/0076-a-track-file-carries-its-art-as-an-optional-section.md),
[track-file.md](../formats/track-file.md)); the reader of record is `oag_trackfile`, and
`crates/trackfile/tests/blender.rs` reads what the add-on wrote. Code:
`scripts/blender/oag_track/scene_art.py` (Blender side) and `artfile.py` (the format, no `bpy`).
A scene with no art mesh, no Sun lamp and no sky writes no art section: the file is what 1.0 wrote.

| In Blender | In the file |
| --- | --- |
| An untagged Mesh object that renders | one `oag/art/<n>` draw per material slot it uses, vertices split per face corner, transforms baked, in natural name order |
| The active colour attribute, read as `.color` (Blender keeps it **linear**; never `.color_srgb`), either domain | `COLOR_0`, the baked light; white without one |
| The active UV map | `TEXCOORD_0`, `v` flipped to the disc's v-down |
| The material's first Image Texture node: an image with custom properties `oag_host_entry` and `oag_host_name` (or **Make Host Texture** in the panel) | `[[art.texture]] host = { entry, name }`: a `Texture` node of a circuit on the host title's disc, by its full asset path. No pixels are exported |
| Any other image (a PNG file on disk) | `[[art.texture]] image = "<file>.png"`, the file copied beside the manifest |
| Material *Blend* (`oag_blend`), or from Blender when *auto*: alpha clip is `cutout`, *Blend* is `alpha`, else `opaque` | `oag_blend` |
| Alpha clip: a Math node set to **Greater Than** feeding the Principled BSDF's Alpha (Blender 4.2+), or the *Alpha Clip* blend mode on older versions | `oag_alpha_ref` = threshold x 255. Blender 5.2 silently turns `blend_method = 'CLIP'` into Hashed, so the node is the way |
| Material backface culling | `oag_culled` |
| Object *Shadow* (`oag_shadow`), *Layer* (`oag_layer`), *Lit* (`oag_lit`) | the same extras |
| The scene's *Sun* (`oag_sun`), else the only Sun lamp | `[art.sun]`: world +Z is the direction towards it, the lamp colour, *Intensity* (`oag_intensity`, default 1), the angle as `angular_diameter` in degrees, the shadow switch as `shadows` |
| Every other Sun lamp | an `[[art.light]]` directional row |
| The world's Background colour times its strength | `[art.ambient]` |
| *Sky* (a host circuit's `.vex` entry), *Lens flare* and its atlas image | `[art] sky`, `[art.lensflare] atlas` |
| A Curve spline's *Cover light* below 255 | `_OAG_LIGHT_SCALE` per spline point, see below |

**Intensity is not Blender's strength.** The file's `intensity` multiplies the colour the PSP hull
rig is built from, clipped to a byte, and a disc export writes 1.0; a Sun lamp's strength is
Cycles irradiance (the API default is 10). So the two are separate: strength lights the bake,
*Intensity* lights the ships.

**Cover light (chosen, not measured).** A spline point is covered when a ray straight up from 5
units above it hits art that casts (`oag_shadow` `both` or `cast`); its ambient byte becomes
the *Cover light* value, and so does its directional byte when a ray towards the sun is blocked
too. An uncovered point stays 255 whatever shades it. This is how a ship darkens under a roof or
in a tunnel (`oag_race::Course::light_scale_at`). Off (255) by default, so a track without it
keeps its bytes.

**Baking light into `COLOR_0`.** The format stores the result, never the method (ADR-0076
decision 9); the add-on has no Bake operator yet, so a builder script does it with Cycles,
headless: give each art mesh a `FLOAT_COLOR` colour attribute and make it active, swap each
material for a plain diffuse one of roughly the texture's albedo (the host textures have no pixels
in Blender), hide the tagged collision from render (the floor lies in the road's plane and would
shadow it), then `bpy.ops.object.bake(type='DIFFUSE', pass_filter={'DIRECT', 'INDIRECT'},
target='VERTEX_COLORS')` with `scene.render.bake.target = 'VERTEX_COLORS'`. Diffuse without the
colour pass is the light alone; a Combined bake would bake the albedo in and the texture would
multiply it again. A vertex bake is only as sharp as the mesh is dense.

**What an import keeps.** Imported art carries the file's normals and texcoords as corner
attributes (`oag_normal`, and `oag_texcoord` as a 3-vector, because Blender takes any 2-float
corner attribute for a UV map) and each lamp's direction and colour, and writes them back
verbatim while what Blender holds still matches, as pads do. A lens flare's colour and direction
overrides travel as the scene properties `oag_lensflare_colour`/`oag_lensflare_direction`.

### Hillside, the worked example (2026-10-29)

Hillside, the 12-point circuit in the example campaign, was re-skinned with all of the above from
a headless builder script that is not committed (its spline, collision, start and pads came out
bit-equal to the plain track's, so records and ghosts keyed `foreign:hillside` stay valid):

- Pulse 03_Track's textures by name: `GenericTrackTextures\_surface6_1`, `_wall6_1`, `_walltop6_1`
  for road and walls, `03_Track\Textures\Grass`, `SeaRock`, `Tree1low`, `_pod2_ceiling`, `Metalbase1`;
  03_Track's sky. Full asset paths, read off the disc with
  `cargo run -p oag-raceplay --example export_track -- <image> 'Data\Environments\03_Track\track.vex' <out>`
  (the generic road and wall set is not under the circuit's own `Textures` folder).
- Terrain either side (receive), 18 rocks and 46 crossed-quad trees (both; the trees `cutout` at
  a 0.25 clip, `oag_alpha_ref` 64, because Pulse's texel alpha tops out near half), a covered bend
  over spline points 84 to 94 on pillars outside the walls (both), amber chevrons as an own PNG
  (`alpha`, layer 1, lifted 0.3 above the road, receive).
- The Sun lamp where 03_Track's skycube paints its sun (format `(0.24, 0.31, -0.92)`, 18 degrees
  up, read off a frame aimed at it) so the baked shadows, the ships' light and the flare agree with
  the sky; a cool fill lamp at strength 0 (it lights the ships only); the world colour as ambient;
  an own 64 x 64 flare atlas in the 2x2 cell layout ([lens-flare.md](../rendering/lens-flare.md)).
- Cycles bake, 128 samples: the road at 2.8 vertices per 1,000 square units, terrain 0.6, an
  exposure of 1.35 so the road's median (0.23 linear) sits in 03_Track's own range for the same
  texture (about 0.23 to 0.55). Cover light 110 under the roof.
- In a race on Pulse PSP EU and USA the loader reports 77 meshes, 10 textures, the sky, the sun,
  the hull rig and the flare, and no texture that did not resolve.

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

`oag-game --track-file name.track.toml` works in every build, and so does RACE REMIX's CUSTOM title (the files in
a track directory: `$OAG_TRACKS`, `data/tracks`, `tracks/` beside the program, `<data dir>/oag/tracks`; TRACK
`<title> (TRACK FILE)`; the `ballisticng` feature only adds Workshop rows to it;
[menus.md](../architecture/menus.md#race-remix-custom-tracks)). The race uses the host title's ship, rules and
physics, and draws flat-shaded collision unless the file has an art section. A file that will not read is listed dimmed with the reason.

## Proof (2026-10-12, art 2026-10-29)

| Check | Result |
| --- | --- |
| `just blender-fixtures` rebuilds the fixtures headless and compares them to the committed bytes | up to date, twice (the output is deterministic) |
| `oval.track.toml` imported by the add-on and exported again (`oval.reexport.*`) | `read(oval) == read(oval.reexport)` and every spline, vertex, pad, start and `max_spacing` float is bit-equal (`crates/trackfile/tests/blender.rs`) |
| A loop built from scratch in Blender: 8 Bezier control points, 64 samples, floor, walls, start, one pad of each kind (`blender_loop.*`) | reads, nothing unread, schema-valid (`schema.rs`) |
| Lone AI craft on Pulse's physics, 7,200 ticks (`trackfile_ground_truth`) | `blender_loop`: lap 3, no respawns; the re-exported oval: lap 4, no respawns (the committed oval: lap 4) |
| `oag-game --race --autopilot --track-file blender_loop.track.toml` | the player gets round at 127 speed; frames at ticks 400, 1200, 2400, 3600 read at 1280x720 |
| Racing line (2026-10-29), AI speed plan lap on Pulse EU, `--race --opponents`, centre then computed | `blender_loop` 2,383 -> 2,373 ticks (-0.4 %); the same loop at 0.35 scale 845 -> 835 (-1.2 %), at 0.2 scale 501 -> 506 (+1.0 %): small, and not always faster (the plan is verified clean in every case, no wall touched) |
| Art (2026-10-29): the loop's canopy, checker, sun, fill light, ambient, sky and flare (`blender_loop.*`, `blender_checker.png`) | every mapping read back as authored (`blender.rs`); the loop imported and exported again is byte-identical (`blender_loop.reexport.*`); on Pulse PSP the art resolves with no miss in the loader report (`trackfile_ground_truth`) |

The Blender-driven part cannot run in CI (no Blender), so it is a local recipe whose output is
committed: `just blender-fixtures` (`--update` rewrites the fixtures). The gate reads the committed
files, so dropping the add-on's spline, start or ribbon fails a Rust test, and a stale fixture
shows as a diff the next time the recipe runs. A different Blender version may sample a curve
differently; regenerate with `--update` and review the diff.

## Open

- The racing line's gain is small (see Proof) and the box is a chosen cliff-avoider: a driver-aware
  line (one that weighs the original driver's lane model, or the speed plan's own lap time) could use
  more of the corridor. A tight track (0.2 scale of `blender_loop`) came out 1 % slower than the centre.
- Banking (a per-point tilt feeding `up`/`lateral`) and a per-point width; both need an authoring
  surface on the curve.
- More than one path and junction authoring beyond the scene property `oag_junctions` (a JSON list
  of `{"prev": [a, b], "next": [a, b]}`, `-1` for none) that the importer writes.
- A *Bake lighting* operator (ADR-0076 decision 9): the bake above lives in a script today.
- Keyframed motion (decision 10): not built; nothing animated is exported.
