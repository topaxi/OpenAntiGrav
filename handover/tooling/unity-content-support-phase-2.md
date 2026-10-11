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
