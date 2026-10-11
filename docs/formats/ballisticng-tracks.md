# BallisticNG Workshop tracks (`oag-ballisticng`)

The first content layer over the generic Unity reader
([`unity-bundles.md`](unity-bundles.md), ADR-0062): what one game's
`MonoBehaviour` classes mean. [`oag-ballisticng`](../../crates/ballisticng) reads
a `.wtrk` track item into typed structs and decodes whole textures. It **maps
nothing into this engine**; that is the consumer's choice and is on the page's
second half, labelled **chosen, not measured**.

**Labels:** *measured* = read out of the files and checked against a second
source or an exact count; *inferred* = consistent with names and counts, no
second source; *chosen* = a decision, never a measurement.

Tested on the Steam Workshop items (32 tracks, 47 ships, 13 legacy `.shp`).
Census-level tests read all 32 tracks for structure only; anything that maps,
renders or drives uses items tagged `Original` and not `Port/Remake` (27 of the
32; 5 are `Port/Remake`, `Mega Mall` carries both tags and counts as a port).

## The `.wtrk` wrapper (measured, 32 of 32)

| What | Bytes |
| --- | --- |
| magic | `u8` length 26, `"Workshop Track (Version 1)"` |
| table | `u32` entry count (7), each `u8` name length, name, `u64` absolute offset |
| value | at its offset: `u32` byte length, payload; each ends where the next starts |

Entries: `WorkshopID` (`u64`), `WorkshopTitle`, `WorkshopDescription`,
`WorkshopPreview` (.NET `BinaryWriter` strings: 7-bit length, UTF-8),
`WorkshopVisibility` (`u64`, 0 on the sample), `WorkshopTags` (`u32` count, then
strings; renderer names such as `DirectX`, `OpenGL` are tags too and may repeat),
`TrackFile` (`u32` bundle length, then the `UnityFS` bundle, length + 4 = the
entry's). The wrapper refuses any unknown or repeated entry, a length that
disagrees, or a bundle without the signature. The offset found by the table
equals the offset found by searching for `UnityFS\0` on all 32. A `.csf` is a
bare bundle (47 of 47 start with the signature).

## Track classes (measured)

Classes are found by each `MonoBehaviour` type's script (`m_ClassName` of its
`MonoScript`), never by class id. Singletons per track: `TrackDataStorage` (1),
`SceneReferences` (1), `RacingLineContainer` (1 on 31 of 32 tracks; Sovereign,
2075028166, has none and reads as no AI line).

`TrackDataStorage`: `FloorVertices`/`FloorNormals`/`WallVertices`/`WallNormals`
(non-indexed triangle lists, per-vertex normals), `FloorTiles`/`WallTiles`,
`Sections`, `AtlasRects`, `SpeedTileIndex`/`WeaponTileIndex`/`RechargeTileIndex`/
`SpawnTileIndex` (atlas indices of the pad art), `StartSectionIndex`,
`StartLineIndex`, `MidLineIndex`, the diffuse, illum and reflection atlases.

- **Section** (344 on Lion's Den): `Index` equals its array position on all 32.
  `Next` chains; `Previous` is unused. `Position` is in the scaled space,
  `Rotation` forward is `rot * (0,0,1)`, `Normal` is up, `Width` ~ half-width to
  the wall top and `Height` the wall height (inferred). `Junction` names a branch
  chain's first section, `PitlaneEntance` (sic) marks a pit branch.
  Unactioned flags (`AllowOutOfBounds`, `NoTiltLock`, `ResetFlyBackwards`,
  `ForTeleporter`, `LockOntoUntilClose`, `AiIgnoreJunction`,
  `NoHoverPositionCorrection`, `NoMaglockSounds`, `RotationMultiplier` not 1) are
  carried by name in `Section::special` so a consumer can report them.
- **Tile**: `Type` 0 plain, 1 speed pad, 2 weapon pad, 3 recharge, 4 grid slot.
  On Lion's Den the type pairs exactly with the atlas index (54 + 11 + 74 + 8 =
  147, all matching) - measured; the *names* are inferred. Wall tiles are all
  type 0 on all 32.
- **Chains** (measured, all 32): the main chain from `StartSectionIndex` closes
  on **31**; **Gehennom** (2864185130) has an 8-section lead-in into a cycle
  that does not include the start section (`LoopEnd::Rho`, `Chains::lap_loop`
  gives the cycle). 33 branch chains in all, **32 pit lanes** (`PitlaneEntance`)
  and one alternate route (Stanza Inter section 13, a port). Branches may nest
  (found by following every junction). **Staten Towers** (1782764483) has 92
  sections (488..579) that no `Next` or `Junction` reaches from the start; the
  chain ends joining the main loop at section 180. They are reported
  (`Chains::unreached`), not guessed at.
- **Per-track scale** (measured): the floor and wall objects sit under a
  transform chain composed exactly into `Placement`. Scales found: **1.0 on 26
  tracks, 1.1 on 3, 1.15 on 2, 0.92 on 1**. A floor tile's `Position` equals the
  placed centroid of its six vertices to 1e-4 on all 31,748 floor tiles of all
  32 tracks, so the earlier "x1.15" is Lion's Den's value, **not a constant**.
  Wall tiles are in unscaled mesh space on all 32. Floor and wall share one pose
  on all 32 (The Tir lists two floor/wall pairs with identical poses).
- **BallisticMeshCollider** `Type`: 0 floor, 2 wall, 1 scenery (as on Lion's
  Den) and a **3** on 15 tracks that is not understood and is only reported
  (`ColliderKind::Unknown`).
- **Textures** (measured): all 1,753 `Texture2D` of the 32 tracks decode (1,721)
  or are the 0 by 0 font atlas (32). A DXT1 atlas against UnityPy: alpha
  identical, every colour byte within 1 (BC1 interpolation rounds here and
  truncates in UnityPy), RGBA32 exact.

`Batur Crater` (2175514157) has 3 `MonoBehaviour`s with a null `m_Script`; they
are counted (`Track::unresolved_scripts`) and skipped, never guessed at. The AI
line has a point per section on all 31 (`SectionNextArrayIndex` equals the
section's `Next` on Lion's Den).

## Field coverage (measured, `tests/field_census.rs`)

Across the 32 tracks, the fields of `Section`, `Tile` and `TrackDataStorage` this
layer does **not** read, as `objects carrying it / objects holding a non-default
value`. The test freezes this list: a field the tools add fails it.

| Class | Unread fields |
| --- | --- |
| `Section` (17,608 objects) | `LockOnToDistance` all non-default (2.0 on Lion's Den; the magnet-lock reach, unused), `Tiles` (all; the same link is `Tile.Section`), `Previous` 0 non-default (so unused census-wide), `NextAiOvverride` 0 of 10,052 (15 tracks only) |
| `Tile` (66,549) | `MapOrder` all, `Swooshable` 316, `UvFlip` 190, `UvOrder` 0, `Color` 0, `ForcedShadow` 0 (visual) |
| `TrackDataStorage` (32) | `TrackFloor`/`TrackWall` (the mesh pointers, kept for UVs later), `TrackTextures` (per-atlas triples), `UseNewUvSytstem` 32, `SpawnTileReplacementIndex` and `WeaponPadReplacementIndex` 13 each (replacement pad art, meaning inferred from names), `MappedFloorTIles`/`MappedWallTiles` (the inverse of `Tile.Indices`), three editor handles, `DataParent` 0 |

Read but not acted on, so reported by name through `Section::special`:
`AllowOutOfBounds` (1,871 sections), `NoTiltLock` (1,352), `NoHoverPositionCorrection`
(1,196), `NoMaglockSounds` (394), `LockOntoUntilClose` (321), `AiAlwaysWallAvoid` (881,
15 tracks), `ForTeleporter` (7), `ResetFlyBackwards` (3), `AiIgnoreJunction` (0),
`RotationMultiplier` not 1 (Oblivion carries it on all 693 sections).

**Staten Towers' 92 unreached sections**: no `Next`, `Junction` or
`NextAiOvverride` leads in (the last is zero everywhere), and no other object of
the bundle holds a pointer to section 488 but its two floor tiles. They are a
chain with no entry that rejoins the main loop at 180; what the game does with
it is unknown.

## Ground truth

`crates/ballisticng/tests/track_ground_truth.rs` (`just test-data`): all 32
parse; chain classification and the counts above; tile positions against placed
centroids; Lion's Den against the probe (307 main + 37 pit sections, 54/11/74/8
tiles, start line at section 52, mid line 219); atlas and texture census.
`tests/wtrk_synthetic.rs` runs in plain `just test`: the layout on built bytes,
every truncation and bit flip a named refusal.

## Not covered

Ship classes (`CustomShipStorage` and friends), `TrackPad` and the `Mod*Zone`
components, `LuaContainer` scripts, the 109 legacy-pack tracks, the legacy `.shp`
format, audio. Collider kind 3 and the meaning of `Section.Type` 2/3 outside
Lion's Den and Stanza are open.
