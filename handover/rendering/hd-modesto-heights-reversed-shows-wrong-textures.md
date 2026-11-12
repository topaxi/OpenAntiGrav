---
status: measure
titles: [hd]
updated: 2026-11-12
summary: "Maintainer saw probably wrong textures on HD/Fury Modesto Heights reversed at tick 1020, [741.7, 61.9, -120.9]; not yet paired against RPCS3"
size: S (30)
difficulty: medium (30)
---

# HD Modesto Heights reversed shows probably wrong textures at one spot

2026-11-12, maintainer, from play (a reliable oracle; turn it into a measurement, do not tune
to a memory). Wipeout HD Fury EU (`hdfury-ps3-eu-dec.iso`, two update archives mounted),
`Data\Environments\Modesto_Heights\track_reversed.vex`. The debug readout at the spot:

```
tick 1020  speed 138.35  grounded 1.0  spline 14.71  height 3.69  at [741.7, 61.9, -120.9]
```

The race's loader lines (`~/.local/state/oag/logs/oag-game.log`, 08:23:35):
- `track_reversed.vex`: 326 of 334 mesh nodes drawn from the `.rcsmodel` (900,963 triangles),
  **8 addressed no chunk**;
- 875 material slots; 277 albedo + 122 lightmap textures, 394 block-compressed.

## Open

- Which surface is wrong at that position, and how (wrong texture, wrong UV set, wrong
  lightmap, missing node), is not known. No RPCS3 pair of the spot exists.

## Next Steps

1. Pair the spot: our frame at tick 1020 on Modesto reversed (same craft and team as the
   capture, headless `--screenshot`) against RPCS3 at the same place (`Write Color Buffers: true`
   in both configs). Name the surface and its material/texture entry.
2. Check whether the eight nodes that address no chunk are at that spot, and whether the same
   surface is right on Modesto forward (`track.vex`): a reversed-only fault points at the
   reversed scene's own node or material table.
3. Cross-check Omega and 2048, which re-ship Modesto (CLAUDE.md lineage rule).
