# Unity content support: phase 1 (the generic reader) landed, phase 2 is the layer on top

2026-10-11: `oag-unity` reads UnityFS v6 and v7, SerializedFile v17 to v22, any object through its
typetree, and `GameObject`, `Transform`, `Mesh`, `Texture2D` (incl. `.resS` streams), `Material` and
`MonoBehaviour` (pointer plus generic tree). Layout, census and evidence:
[`docs/formats/unity-bundles.md`](../../docs/formats/unity-bundles.md); the layering decision:
[ADR-0062](../../docs/architecture/adr/0062-unity-content-is-a-generic-reader-with-a-content-layer-on-top.md).
The crate holds Unity specifics only. Everything below is the layer *on top of it*, which does not exist.

## Open

- **The content layer crate** (new, depends on `oag-unity`, rule 4 forbids the reverse): reads one
  game's `MonoBehaviour` schema through `Value`. A first probe's findings, not yet re-derived against
  the reader: a track is a `TrackDataStorage` of `Section`s (position, rotation, normal, width,
  height, `Next` chain, main loop plus pit and branch chains), floor and wall `Tile`s (`Type` 1 speed
  pad, 2 weapon pad, 3 recharge, 4 grid slot), a `RacingLineContainer` of lateral offsets, and
  `BallisticMeshCollider` types on the floor and wall objects. Position-like fields are in the
  track's scaled space (x1.15 against the mesh arrays). **Read these again off the generic tree and
  check them before building on them.**
- **The `.wtrk` and `.csf` wrappers**: a `.wtrk` is a magic line, tags and a title, then a UnityFS
  bundle at an offset (found by searching for `UnityFS\0`, as the test scaffolding does); the legacy
  `.shp` (13 items) is a different non-Unity format. Neither wrapper is in `oag-unity`.
- **Units and handedness** (chosen, not measured): Unity is left-handed Y-up, this engine
  right-handed Y-up, so negate one horizontal axis and reverse winding. The scale into this
  engine's units is a compromise: widths match at about x8.7, lap lengths do not (a Short track at
  that scale is 24% longer than Pulse's longest). A ship's handling block is a different model
  from `oag-physics`; a mapping would be a guess until that game's behaviour is read.
- **The foreign-track source** beside `oag_source` (ADR-0062 decision 4): a player directory
  (`data/unity/` like `data/dlc`), a synthetic `catalogue::Track` carrying "foreign" as data (not a
  `title.name` branch, the title-branching ratchet forbids it), one branch in the race loader, the
  loader report naming every refused or substituted piece. Not an asset-override-layer entry.
- **Block texture decode**: the crate returns raw `DXT1`/`DXT3`/`DXT5` pixels. A consumer decodes
  through `oag_texture::bcn` (`dxt1`, `dxt23`, `dxt45`, now `pub`); a whole-texture helper belongs in
  the layer, not in `oag-unity`.
- **Not exercised by any real file, so untested against Unity** (the format page scores each): meshes
  with more than one vertex stream, non-float channels, triangle strips and `Float16` decoding (only the
  synthetic tests build these), vertex format ids of 3 or more on engines before 2019 (refused by name:
  UnityPy's 2017/2018 table disagrees with the older one), compressed meshes, SerializedFile versions 18
  to 21, bundles with the info at the end, crunched textures. A new content family may hit them first.
- **Audio** (78 `.resource` streams, 129 MB) is not read, nor are `AudioClip`s.
- **Scenery colliders**: whether the originating game treats scenery `MeshCollider`s as solid is
  unknown; a minimal race would ignore them and say so in the loader report.

## Next Steps

1. Decide the content layer's crate name and its first deliverable: reading one track's
   `TrackDataStorage` through `Value` into a typed struct, with a ground truth over the 32 tracks.
   One day.
2. Write the foreign-track source and its one loader branch, with Lion's Den as the first circuit
   to load, and a loader report. Two to three days, after step 1.
3. Ships: the frontend and in-race meshes and the six liveries are solid (`Cliffhawk` reads end to
   end); handling needs a chosen mapping, labelled chosen. Half a day for the meshes and skins.

## Phase 2 progress (2026-10-11, unity-phase2 lane): step 1 done and committed, gate not run

- Done: `oag-ballisticng` (`crates/ballisticng`, commits 399d58b81 and 8e3951c99): `.wtrk`/`.csf` wrappers,
  typed `Track` (sections, chains, tiles, AI line, colliders, per-track `Placement`), whole-texture decode, field
  census, ground truth over all 32 tracks. Format page: `docs/formats/ballisticng-tracks.md`. Rule 5 added to
  `scripts/check-dependency-rules.py` and CLAUDE.md. `oag-ballisticng` has no `[workspace.dependencies]` entry
  until `oag-raceplay` depends on it (cargo shear).
- Not gated: `just gate-affected` died with `No space left on device` at link (disk 100% full, 3.5 MB free;
  worktree targets: lead-gate 121G, ai-pace 68G, ai-pace-exp 44G, this lane 32G). Re-run the gate once space exists.
- Findings that change the design: the floor scale is per track (1.0 x26, 1.1 x3, 1.15 x2, 0.92 x1), read from
  `Track::floor_placement`, apply to `Floor/WallVertices` only (section and floor-tile positions are already
  placed, wall tile positions are not). Gehennom's lap is a cycle not containing section 0
  (`Chains::lap_loop`). Staten Towers has 92 sections no link reaches.
- Regression gate before any raceplay edit: `a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round` PASS (15.2 s, all twelve clean).

## Next Steps (replaces 1 above)

1. Re-gate step 1 when disk is free.
2. Mapping module (chosen constants): `AiTrack` paths from `Chains::lap_loop`, `PadVolume`s, `TriangleSoup`s,
   `StartSlot`. Measure first on a Pulse circuit: does `SplinePoint.pos` sit on the floor, the sign of
   `cross(tangent, down)` vs `lateral`, floor normal vs `down`, how a closed ring sits in `paths`/`junctions`.
3. Foreign-track branch: `load.rs` is at the 1,000-line ceiling, so the branch lives in a new module and must cost
   `load.rs` no net lines; destructure `Setup`/`Loaded` without `..` so each track-derived field gets a decision.
