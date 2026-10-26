---
status: ready-to-wire
titles: [all]
updated: 2026-10-11
summary: "Foreign-track picker row, atlas and sky from BallisticNG tracks; Unity formats no real file exercises are untested"
---

# Unity content support: the reader, the BallisticNG layer and a raceable foreign track have landed

2026-10-11: `oag-unity` reads UnityFS v6 to v8 and SerializedFile v17 to v23 (layout and census:
[`docs/formats/unity-bundles.md`](../../docs/formats/unity-bundles.md), the layering decision:
[ADR-0062](../../docs/architecture/adr/0062-unity-content-is-a-generic-reader-with-a-content-layer-on-top.md)).
`oag-ballisticng` reads a Workshop track, and `oag_raceplay::foreign` maps it into a race.

## What landed

- `oag-ballisticng` (`crates/ballisticng`): wrappers, typed track, texture decode, ground truth over all 32 tracks,
  field census. Page: `docs/formats/ballisticng-tracks.md` (also holds the mapping, every chosen constant and the
  raced results). Rule 5 in `scripts/check-dependency-rules.py`.
- Mapping and loader: `oag_raceplay::foreign` (`choice.rs` has every chosen number), `oag_source::ballisticng` (the
  `$OAG_BALLISTICNG` / `data/ballisticng` / `<data dir>/oag/ballisticng` source), `Options::foreign_track`, `catalogue::Track::foreign`
  (the field only, no front-end row), `oag-game --ballisticng-track`. `load.rs` shrank by 142 lines: the track-derived
  inputs are `load/track_inputs.rs`, with the foreign path one early return.
- Raced on Pulse's ship and physics: all 27 `Original` tracks load, 26 get round, 14 with a clean lap. Lion's Den:
  loads, a craft sits on the grid, drives (a speed pad fires), and under `--autopilot` laps in 50.8 s (best lap) with
  the AI rescued across its two jumps. Nothing was tuned.

## Open

- **Not exercised by any real file, so untested against Unity** (the format page scores each): meshes with more than one
  vertex stream, non-float channels, triangle strips and `Float16`, vertex format ids of 3 or more on engines before 2019,
  compressed meshes, SerializedFile versions 18 to 21, bundles with the info at the end, crunched textures. Audio
  (78 `.resource` streams) is not read.
- Foreign tracks have no menu row (the `catalogue::Track::foreign` field exists, nothing builds a list from
  `oag_source::ballisticng::tracks()`); the front-end row needs `string_id` text and pointer support.
- The track atlas, sky, fog, lights and scenery are not drawn; the floor and wall are flat-shaded collision. Floor UVs
  need the `TrackFloor`/`TrackWall` mesh pointers (kept unread), checked equal to `FloorVertices` first.
- Jumps: Lion's Den crosses two gaps (111 and 141 units) by rescue. The original uses `ModJumpZone` push; unmapped.
- Pit lanes, recharge strips, the authored 8 grid slots, scenery colliders (and collider kind 3 on 15 tracks) are
  reported, not mapped. Valkyrie's Flight stalls at lap 1 (66 respawns); 12 tracks lap only with respawns.
- The world scale (x8.7) makes laps longer than Pulse's; retune in `foreign::choice` once the hull and speeds are
  looked at. Ships (`.csf`) and the handling mapping are not started.
- Staten Towers' 92 unreachable sections and Gehennom's lead-in (`LoopEnd::Rho`) are handled by naming them.

## Next Steps

1. A foreign-track row in the track picker (strings, pointer support) reading `oag_source::ballisticng::tracks()`.
2. Draw the atlas: keep `TrackFloor`/`TrackWall` pointers, check mesh positions equal the arrays, take UVs, decode the
   atlas through `oag_ballisticng::texture`. Then scenery meshes.
3. Map `ModJumpZone` and read what the 12 respawn-dependent tracks need.
4. Ships: Cliffhawk's meshes and liveries, hull from a disc ship's handling at first (labelled chosen).
