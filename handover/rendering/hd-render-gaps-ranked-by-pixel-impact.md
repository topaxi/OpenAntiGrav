---
status: measure
titles: [hd]
updated: 2026-10-26
summary: "HD render error by 120 px tile on 13 pairs: ceiling rim highlights lead the bias; scenery, HUD and wall are not yet lanes"
---

# HD/Fury render gaps ranked by pixel impact

2026-10-26, lane `hd-impact-scout`. Question: where does HD differ most from the
original by pixels a player sees, so the next HD lane aims at the biggest gap.

## Open

**Set.** 13 HD pairs under `compare-page/hd/` (Harimau classic, Time Trial start,
1920x1080, chase cam). `talons-pack` (`hd-race/`) is excluded: its frame is a
different moment with an imposed HUD and a warm-up exhaust path, so a per-pixel
diff ranks nothing there.

**Method (reproducible).** For each pair, per pixel: mean abs RGB difference
(0-255) between `theirs.png` and `ours.png`; signed luma `ours - theirs`. Tile
the frame 120 px (16x9 grid). Impact = sum of the per-pixel error in a tile.
Low frequency = mean abs RGB difference after a Gaussian blur of sigma 7 px on
both frames (what survives is colour, brightness or missing content). High
frequency = raw minus low (AA, texture detail, noise). Sharpness = mean
absolute gradient of ours over theirs, per block, on luma. Blocks below are
hand-drawn tile groups; the tile list is in the table's column "tiles".

**Global tone is not the story.** A per-channel linear fit `ours ~ a*theirs + b`
over each whole frame removes only 5% of raw error (fit/raw 0.95). There is no
frame-wide exposure lane. The luma error is spatial: the top rows run ours
brighter (+3.7 to +6.3 luma), the lower left runs darker. The vertical split is
already recorded in [hds-frame-was-too-bright-and-too-bloomy.md](hds-frame-was-too-bright-and-too-bloomy.md)
(2026-09-17, one circuit); the 13-pair grid confirms it on every circuit.

**Totals.** Raw error per pair: mean 6.2 to 11.0 per pixel (13-pair mean 8.8).
65% of the raw error survives the sigma-7 blur, so most of it is low frequency.
Sharpness is not the gap: ours-over-theirs gradient is 0.98 to 1.06 per block.

**Ranked table.** Ranked by low-frequency share, the part a lane can fix. Raw
share of all 13-pair error in brackets. "Low err/tile" is the low-frequency
error per tile summed over the 13 pairs, in thousands.

| # | Block (120 px tiles) | Low-freq share | Raw share | Low err/tile | Luma bias | Covered by |
|---|---|---|---|---|---|---|
| 1 | Centre scenery and gate (tx 6-12, ty 1-4) | 21.8% | [21.6%] | 108 | +2.7 | position bucket: content changes per circuit; partly [hd-glow-mask.md](../../docs/rendering/hd-glow-mask.md); not a lane yet |
| 2 | Tiles containing HUD (LAP, shield, RECORD, bottom gauges) | 18.8% | [16.7%] | 79 | +2.4 | tiles include track and wall behind the HUD, so the HUD-only share is unmeasured; [hd-hud.md](../../docs/formats/hd-hud.md) |
| 3 | Left wall and stands (tx 0-5, ty 1-4) | 17.1% | [17.5%] | 84 | +0.6 | position bucket; the FX-350 advert is a timed board, see [hd-glow-mask.md](../../docs/rendering/hd-glow-mask.md) ("Still open on the board") |
| 4 | Track left and right (tx 0-5 or 10-15, ty 5-6) | 11.2% | [11.7%] | 55 | +2.3 | [hds-frame-was-too-bright-and-too-bloomy.md](hds-frame-was-too-bright-and-too-bloomy.md) (blown lightmap); crops match closely |
| 5 | **Ceiling and light shafts (tx 9-12, ty 0-1)** | 9.6% | [8.3%] | **142** | **+6.3** | **new lane candidate**: white rim highlights on ramp edges; see Next Steps 1 |
| 6 | Craft hull (tx 6-9, ty 5-6) | 7.8% | [10.3%] | 58 | +1.0 | [hds-engine-light-washes-the-hull-on-boost.md](hds-engine-light-washes-the-hull-on-boost.md); half its error is high frequency (low/raw 0.50) |
| 7 | Other scenery (tx 0-15, ty 2-4, outside 1-3) | 7.3% | [7.3%] | 96 | +1.7 | position bucket |
| 8 | Other sky and track (rest of rows 0-1 and 5-8) | 6.5% | [6.5%] | 57-86 | +1.0 | position bucket |

Row 5 has the highest error per tile of any block (142 against 55 to 108 for
the others) and the largest luma bias. It is the first lane candidate, not the
largest block.

**Luma bias by tile.** Top row, columns 6-15: ours brighter by +6 to +12. Row 1,
columns 6-12: +3 to +13. Rows 2-4: +2 to +5. Left columns of rows 0 to 3 and
the lower rows: near zero, some darker. The top-right bias is the largest
single bias on the frame.

**Capture mismatch, do not lane these.**

- `talons` (heading drift 1.72 deg, camera 0.29 units, race clock -7.2 s): kept
  in the totals, but dropping it moves every block share by under one point.
  Its scenery error is partly pose.
- `talons-assegai-c1` (0.54 deg, clock -2.3 s): same; marginal.
- `talons-pack` (`hd-race/`): excluded, see Set.
- Advert timing on the left wall (table row 3): one pair's FX-350 text is absent in
  ours. Check the board's own timeline before reading it as a missing decal.
- HUD digits: on `vineta-k` the lap clock reads 0.13.7 in both frames; on
  `the-amphiseum` it reads 0.13.6 in both. The HUD row still needs a HUD-only
  mask before it is a lane.
- Race clock drift on the other 11 pairs runs -0.1 to -1.4 s, and ship and camera
  heading within 0.2 deg. Pose is close enough for a per-block read; not for
  per-object claims.

## Next Steps

1. **Lane `hd-ceiling-rim-highlights`** (ready-to-wire, the stand-in specular
   thread [hd-lit-billboard-specular-and-gantry-leftovers.md](hd-lit-billboard-specular-and-gantry-leftovers.md)
   is the first place to look). Claim: HD's no-LG2 ceiling and ramp materials
   draw white rim highlights the original does not, and they cause most of
   the ceiling row's +6.3 luma bias (table row 5). Verify first with one ablation on `vineta-k`
   and `the-amphiseum` (NO_SPECULAR on the ceiling families only, before any
   change to the shared shader). Acceptance region: tiles tx 9-12, ty 0-1 on
   the 13 pairs, low-frequency error down and luma bias down to within +2; the
   craft block and track block unchanged within noise; `vineta-k` crop shows
   no white rim on the ramp edges.
2. **Measure, not a lane: HUD-only share.** Claim: the HUD's own pixels carry
   under 5% of the 13-pair error, so the HUD row is an estimate artifact. Method:
   mask the HUD layout rects from [hd-hud.md](../../docs/formats/hd-hud.md) and
   take the low-frequency error inside the mask over the 13 pairs. Acceptance:
   a single number in this thread. If it is above 10%, open a HUD text-alpha lane.
3. **Measure, not a lane: left wall and centre gate.** Claim: blocks 1 and 3
   (39% of low-frequency error together) are many draws, not one. Method: a
   per-draw ablation on two circuits, the same moment, from the existing
   capture tooling. Acceptance: one or two named draws explain at least half
   the block's low-frequency error on both circuits. Only then name a lane.

## Later

- The left-wall advert timeline (table row 3): capture a same-moment frame
  before calling it a missing decal.
- Craft hull high-frequency error (table row 6, low/raw 0.50): AA or texture
  detail; not a lane until the hull's own draws are checked.
- Refresh this table after any HD lane merges (same method, same 13 pairs).
