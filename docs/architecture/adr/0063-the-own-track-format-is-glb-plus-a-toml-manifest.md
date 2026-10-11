# ADR-0063: This project's own track format is a glTF binary plus a TOML manifest

## Status

Accepted (maintainer, 2026-10-11). Builds on [ADR-0062](0062-unity-content-is-a-generic-reader-with-a-content-layer-on-top.md),
whose foreign track mapping becomes one importer into this format, and on
[ADR-0061](0061-two-asset-layers-around-the-disc.md) (a track file is not an
override-layer entry). Layout and evidence: [`track-file.md`](../../formats/track-file.md).

## Context

A custom track needs a form to be authored in. The first one raced, a BallisticNG
Workshop track, arrives through a mapping with chosen constants (a world scale, a
handedness flip, tile codes). That is right for a foreign file and wrong for a
track somebody authors for this engine: nothing about it should be a conversion.
The mapping's output was already a plain-data model, the shapes the race loader
reads from a disc circuit (`oag_vex` spline, collision nodes, pad volumes, start
slot), so the question was how to serialise that model, and where it lives.

## Decision

1. **The model is `oag_trackfile::Track`**: id, title, the spline graph
   (`AiTrack`: every path, every junction, the point version), the start slot,
   collision nodes (floor, wall, reset, magstrip, cage, HD track wall), speed
   pads, weapon pads, and the producer's notes. It is the loader's own
   `oag_vex` types, not a parallel set, so the simulation reads a file the way it
   reads a circuit. Collision geometry is world-space, as a disc circuit's is.
2. **Two files, one stem**: `<name>.track.toml` and the `<name>.glb` it names.
   - **Every `f32` the simulation reads is binary** in a glTF accessor:
     little-endian, tightly packed, bit-exact. The TOML holds names, the version,
     counts, and the integer structure of the spline graph. A decimal float in a
     text file is a rounding round trip through `f64`; the format has none.
     (The maintainer's brief put "race data" in the TOML; the floats of it are in
     the glb instead, for this reason. The TOML keeps the integers.)
   - Records are meshes of the glb: `oag/start`, `oag/spline/<i>` (one `POINTS`
     primitive per path, custom `_OAG_*` attributes for the frame, widths, AI
     bounds and flags), `oag/pads/speedup`, `oag/pads/weapon`, and one node per
     collision node carrying an `oag_kind` extra (`floor`, `wall`, `reset`,
     `mag_floor`, `cage`, `track_wall`) over `TRIANGLES` primitives with an
     `_OAG_SCALAR` attribute. A stock glTF viewer or Blender shows the geometry.
   - Coordinates are this engine's world: right-handed, Y up, in world units.
     Anything a Blender export should bake (node transforms) is refused by name,
     not guessed at.
3. **Versioning.** `track.format = "oag-track"` and `track.version = "major.minor"`.
   A reader **refuses an unknown major by name**. A newer minor reads, and every
   manifest key it does not know is reported as unread. A minor only adds
   optional keys and attributes; a change that makes an old reader misread is a
   major.
4. **A reader decodes or refuses by name** (`oag_trackfile::Error`): another
   format, a bad version, a count the manifest and the glb disagree on, an
   unknown collision tag, a transformed record node, a missing or mistyped
   accessor, a non-finite number, an index out of range. Mesh nodes it does not
   know (visual meshes) are counted in `Loaded::unread`, never silently dropped.
5. **Absent means absent.** Format 1.x has no field for sky, fog, lights, clouds,
   art meshes and materials, scenery and its animation, track particle emitters,
   sound emitters, visibility sets or the Zone variant. The loader report names
   them (`oag_trackfile::ABSENT`); nothing is borrowed from the host title.
   Adding any of them is a minor version that adds optional records.
6. **Where it lives**: a new crate `oag-trackfile` (ADR-0050's split by format
   family) depending on `oag-vex` for the types, `gltf` (default features off,
   only `extras`, `names` and `utils`, so no `image` decoder), `serde_json` for
   the extras, `serde`, `toml`. It joins the determinism scan (`check-transcendentals`):
   it parses numbers the simulation reads. It is forbidden to the gameplay crates
   like `oag-unity` (rule 1's list): the race loader reads it, the simulation
   never does. `oag-ballisticng` stays unable to reach it (rule 5); the importer is
   `oag_raceplay::foreign`, which already owns the mapping.
7. **Importers and the exporter.** BallisticNG is `oag_raceplay::foreign` mapping
   into `Track`. A disc circuit exporter (`cargo run -p oag-raceplay --example
   export_track`) is **local only**: its output is disc content and goes under
   `data/`, never committed (`just audit-leakage`). Only a project-authored
   track is a committed fixture (`crates/trackfile/tests/fixtures/oval.*`, written
   by `cargo run -p oag-trackfile --example make_fixture`, with a test that fails
   when the committed bytes drift from the generator).
8. **Raced by `--track-file <manifest>`** (and `--unity-track`'s existing
   `foreign_track` option, which now takes either): the same loader early return,
   host title's ship, rules and physics, a track id of `foreign:<id>` keying
   records and ghosts.

## Consequences

- A custom track has no chosen constants of its own: its units are the engine's.
  The foreign mapping's constants stay in `foreign::choice`, labelled *chosen,
  not measured*, and apply to importing only.
- The file is not hand-authorable as text (the numbers are binary). That is the
  trade for bit-exactness; the authoring path is a Blender export plus the
  manifest, and a Blender add-on is the next piece of work, not part of this one.
- `node_index` of a collision node and `chunks` of a collision mesh (a position
  in a `.vex` tree and byte ranges into a disc payload) have no meaning in a file
  and do not survive; every other field of 80 disc circuits does (see the page).
- A second serialisation of the same model (RON, CBOR) would be a new major.
