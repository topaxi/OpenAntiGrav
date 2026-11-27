# Detonator layout files

`Data\XML\DetonatorLayouts\PatternFile.xml` and `GamePlayValues.xml`: what Detonator lays on a
circuit. Two **global** files, the same on every track (nothing is per-track, nothing is in the
`.vex`), found by `Detonator_LoadPatternFile` and `Detonator_LoadGamePlayValues`
(`docs/ghidra/functions/ps3-hdfury-eu/detonator-placement.md`, confidence 85). Read by
`oag_tables::detonator_layout`; the entries are `oag_title::Detonator::layouts`.

## `PatternFile.xml`

Top-level `<Pattern stageToAppearFrom>` elements, each holding `<ClusterN>` children. On the disc:
18 patterns, 79 clusters.

| Attribute (per cluster) | Reader field | Notes |
| --- | --- | --- |
| `numOfObjects` | `num_objects` | `1` on 76 clusters |
| `distanceBetweenObjects` | `distance_between_objects` | track entries between objects of a run |
| `type` | `kind` | `0` on every disc cluster; the placement lays only `0` |
| `percentageAcrossTrack` | `percentage_across_track` | float in the file, truncated, as stored |
| `angleUp`, `angleRight` | `angle_up`, `angle_right` | parsed; never passed to the spawn (70) |
| `distanceFromOriginalPosition` | `distance_from_original_position` | entries from the group's base |
| `object1Type`..`object10Type` | `object_types` | `0` mine, `1` bomb; all `0` on the disc |

`stageToAppearFrom` defaults to `1`. A missing attribute is `0`: the original clears a record and
reads with `atoi`/`atof`, so the reader does the same and reports no error.

## `GamePlayValues.xml`

One `<GamePlayValues>` element. The attributes it names replace
`GamePlayValues::default()` (the executable's static block); the rest keep it. The disc's
`MineCountStage1`..`14` are `80, 90, 95, 100, 105, 110, 120, 130, 140, 150, 170, 180, 190, 200`
(the static defaults are `70`..`200` in steps of 10 and are a fallback only). With
`MineCountsRepresentMaxTotalToAppear="1"` a stage's count is a total that includes the mines
still alive.

`MinesScaleToMusic` is this project's attribute, in no disc file: absent or `1`, laid mines
scale with the music; `0` holds them at their authored size
([detonator-pulse.md](../gameplay/detonator-pulse.md#mine-size)).

## Across titles

`GamePlayValues.xml` is byte-identical on HD (`DATA00`), 2048 (`data.psarc`) and Omega
(`data00.psarc`); `PatternFile.xml` decodes to the same 18 groups on all three
(`hd_weapons_ground_truth.rs`). **Checked, applies**, wired on all three through the same axis.

## `DetonatorScoring.xml`

A third global file, read by `oag_tables::detonator_layout::parse_scoring` and named by
`oag_title::DetonatorLayouts::scoring` (`Data\XML\DetonatorScoring.xml` on HD, 2048 and Omega).
Its rows sit inside a `<DetonatorScoring>` element: `Bullet` (`multiplier` 1.0, `new_mine_score` 100,
`old_mine_score` 50, `bomb_hit_score` 0, `bomb_score` 2000, `bomb_increment` 500) and `Emp`
(50, 25, 1000). What each pays and when:
[detonator-cannon.md](../ghidra/functions/ps3-hdfury-eu/detonator-cannon.md), "The score". The same
values on all three titles, checked by the resolve tests (HD `hd_weapons_ground_truth.rs`, 2048
`patch_ground_truth.rs`, Omega `omega_title_ground_truth.rs`).
