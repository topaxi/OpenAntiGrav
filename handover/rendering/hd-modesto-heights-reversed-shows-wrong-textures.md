---
status: ready-to-wire
titles: [hd]
updated: 2026-11-12
summary: "Modesto Heights banner band (uvdistortion_diff_vertexcolplusoverlay) draws white; the original draws it dark with teal. Wire the family."
size: S (30)
difficulty: medium (40)
---

# HD Modesto Heights: the banner band is drawn white

Measured 2026-11-12 on a pair (ours against RPCS3 at the maintainer's spot, two boots), written up in
`docs/formats/rcsmaterial.md`, "Two surfaces the maintainer saw wrong". Not reversed-only: forward `track.vex` carries the same
family as slot 551, and the 8 chunk-less nodes and 21 unresolved materials are the same on both directions.

## Open

- Surface: reversed slot 550, `materials_reversed/uvdistortion_diff_vertexcolplusoverlay.rcsmaterial`, pictures
  `and_ossego` (unit 2), `and_tunnelscreens_small` (unit 1), `and_line_rgb_clouds_alpha` (unit 0). Top band over grey 200:
  original 0.1 to 0.3 %, ours 8.5 % (0.3 % with the slot skipped). Cause, confidence 70: ours binds the white Ossego logo as the
  diffuse; the program (`fp` block `0x2ad0`) uses it only inside an overlay of a dark teal cloud term.
- The unread values: unit 0's UV distortion (parameters `d0989794`, `c0594d0e`, `fe619466`, `96e30da5`, `2bdce348`) and
  the overlay colour `961662ae`.

## Next Steps

1. Give the `uvdistortion_*` family (7 slots on Modesto) its own reading in `mesh::rcs::skin`: output =
   `vertex colour x tunnelscreens.rgb x light + overlay(teal x clouds x screens.alpha, ossego)`.
2. Re-render the pair at `--pose=729.72,58.95,-119.54` and compare the top-band boxes (target: original 0.1 to 0.3 % over
   grey 200). Check the forward circuit's slot 551 and `uvdistortion_diff_vertexcol` slots (464, 467, 472, 486, 527, 539) do not move.
3. Omega and 2048 ship no file of this name (their material names differ); record "not checkable by name" or run their own pair.
