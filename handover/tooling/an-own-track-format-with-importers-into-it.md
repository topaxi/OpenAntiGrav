# An own track format, with BallisticNG and the originals as importers into it

2026-10-11, the maintainer's idea after the Unity content support phases: for custom
tracks, this project's **own** format should be the canonical one, and a foreign track
format (BallisticNG's `.trk`, read by `oag-ballisticng` on top of `oag-unity`) becomes one
importer into it rather than the format custom tracks are authored in. **Queued to start
right after Unity content support phase 2 lands** (`unity-content-support-phase-2.md`).
Nothing below is built; it is a design to check against the code before building.

## Why

- **No lossy mapping.** The format holds what the race consumes: the spline frame, floor
  and wall collision, pads, the start grid, the lap line, the AI line, visual meshes and
  their materials. The chosen constants a BallisticNG track needs (scale about 8.7,
  handedness, tile codes 1-4) do not exist for a track authored in it.
- **Ours to version, keep deterministic and extend** per title: Zone, Pulse's pad kinds,
  HD's emissive pad masks, sound emitters, scenery animation.
- **Open tooling.** Geometry as glTF 2.0 (Blender and Unity both export it; `extras` carries
  per-mesh tags such as floor/wall/scenery), race data as a small TOML beside it. No
  proprietary engine is needed to author one.

BallisticNG keeps its value as a library: an existing community and a large Workshop. Our
format starts with no tracks and no authoring tool; a Blender add-on is the real cost, not
the reader.

## Open

- **Is the phase-2 mapping output already the model?** `oag-ballisticng`'s mapping is meant
  to produce plain data the existing sim consumes. If it is a named type, the format is
  that type serialized; if it is wired straight into the race loader, extract it first.
- **What the model must carry** to race a disc circuit unchanged: compare against what
  `oag-raceplay::catalogue::Track` and the race loader read from a title today (spline,
  `CollisionWorld` soups, pads, grid, sky, fog, lights, emitters, PVS). Decide what a
  custom track may omit, and what the loader report says when it does.
- **Versioning and validation**: a version field, refusal of unknown majors, every refusal
  named in the loader report (the never-invent rule: absent means absent).
- **Determinism**: anything the sim reads from the file is `f32` data, parsed the same on
  every platform; no float formatting round trip in between.
- **Legal**: an exporter from the originals into this format writes only under `data/`,
  never committed (`just audit-leakage`). Only a project-authored track may become a
  committed fixture, as `crates/unity/tests/fixtures/` already is.

## Next Steps

1. After phase 2 lands: read its mapping output and the race loader, and write the model
   as one documented type (an ADR, since it fixes a file format).
2. Reader and writer for glTF plus TOML, a ground truth that Lion's Den converted from
   BallisticNG into it races the same as loaded directly.
3. An exporter from a disc circuit (Pulse's Moa Therma first) into the format, local only:
   loading it back must reproduce the native load exactly. This doubles as the round-trip
   check of the whole mapping.
4. Later: a project-authored fixture track in the format, committed, and a Blender add-on.
