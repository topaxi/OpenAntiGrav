---
status: measure
titles: [hd]
updated: 2026-10-26
summary: "HD gaps by 120 px tile; top-right band cut (+6.3 to +3.0): Amphiseum +5.7 and Vineta K -3.1 left; scenery, HUD, wall open"
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
(2026-09-17, one circuit). The 13-pair mean shows the same split; it does not
hold on every pair (anulpha-pass: whole-frame bias -2.2, row-5 bias +0.1).

**Totals.** Raw error per pair: mean 6.2 to 11.0 per pixel (13-pair mean 8.8).
65% of the raw error survives the sigma-7 blur, so most of it is low frequency.
Sharpness is not the gap: ours-over-theirs gradient is 0.98 to 1.06 per block
(6 pairs: the-amphiseum, vineta-k, sol-2, talons, metropia, modesto-heights).
Hue is not measured; only luma is in this table.

**Ranked table.** Ranked by low-frequency share, the part a lane can fix.
Raw share of all 13-pair error in brackets. Low impact is low share times the
total low error (about 153 M, the sum of per-pixel error over 13 pairs at
0-255). "Low err/px" is the mean per-pixel low-frequency error in its block.
"Top 3 pairs" are the three with the most raw error in the block.

| # | Block (120 px tiles) | Low share (impact) | Raw share | Low err/px | Luma bias | Top 3 pairs | Covered by |
|---|---|---|---|---|---|---|---|
| 1 | Centre scenery and gate (tx 6-12, ty 1-4) | 21.8% (33 M) | [21.6%] | 7.5 | +2.7 | amphiseum, metropia, ubermall | position bucket: content changes per circuit; partly [hd-glow-mask.md](../../docs/rendering/hd-glow-mask.md); not a lane yet |
| 2 | Tiles containing HUD (LAP, shield, RECORD, bottom gauges) | 18.8% (29 M) | [16.7%] | 5.5 | +2.4 | amphiseum, vineta-k, anulpha | tiles include track and wall behind the HUD, so the HUD-only share is unmeasured; [hd-hud.md](../../docs/formats/hd-hud.md) |
| 3 | Left wall and stands (tx 0-5, ty 1-4) | 17.1% (26 M) | [17.5%] | 5.9 | +0.6 | sol-2, modesto-heights, sebenco-climb | position bucket; the FX-350 advert is a timed board, see [hd-glow-mask.md](../../docs/rendering/hd-glow-mask.md) ("Still open on the board") |
| 4 | Track left and right (tx 0-5 or 10-15, ty 5-6) | 11.2% (17 M) | [11.7%] | 3.8 | +2.3 | moa-therma, anulpha, sol-2 | [hds-frame-was-too-bright-and-too-bloomy.md](hds-frame-was-too-bright-and-too-bloomy.md) (blown lightmap); crops match closely |
| 5 | **Top-right band (tx 9-12, ty 0-1)** | 9.6% (15 M) | [8.3%] | **9.9** | **+6.3** | vineta-k, amphiseum, modesto-heights | **new lane candidate, cause split**: see Next Steps 1 |
| 6 | Craft hull and plume (tx 6-9, ty 5-8) | 7.8% (12 M) | [10.3%] | 4.0 | +0.3 | anulpha, ubermall, sol-2 | [hds-engine-light-washes-the-hull-on-boost.md](hds-engine-light-washes-the-hull-on-boost.md); half its error is high frequency (low/raw 0.50) |
| 7 | Right wall and sky (tx 13-15, ty 2-4) | 7.3% (11 M) | [7.3%] | 6.7 | +2.5 | vineta-k, modesto-heights, ubermall | position bucket; no crop of its own |
| 8 | Other sky and track (rest of rows 0-1 and 5-8) | 6.5% (10 M) | [6.5%] | 3.9-6.0 | +1.0 | position bucket | position bucket; no crop of its own |

Row 5 has the highest error per pixel of any block and the largest luma bias.
It is the first lane candidate, not the largest block. Rows 1 to 3 are large
because they cover the most tiles, and their content differs on each circuit.
Row 5 is named for its place: on Vineta K and Talon's it is open sky.

**Luma bias by tile.** Top row, columns 6-15: ours brighter by +6 to +12. Row 1,
columns 6-12: +3 to +13. Rows 2-4: +2 to +5. Left columns of rows 0 to 3 and
the lower rows: near zero, some darker. Per pair, row 5's bias runs from +16.1
(the-amphiseum) and +14.9 (vineta-k) down to +0.1 (anulpha-pass). The top-right
bias is strongest on the Amphiseum and Vineta K pairs.

**Row 5 cause is split (checked on crops, 2026-10-26).** Not one rim-highlight
cause. What the crops show:

- `the-amphiseum`: ours draws brighter additive light beams from the ceiling
  lights (diagonal white shafts through the trusses); the original's beams are
  dimmer. The light-cone material was fixed on this circuit earlier
  ([hd-accuracy-pass-2026-10-05.md](hd-accuracy-pass-2026-10-05.md), `dc_lightcone`);
  what remains is unmeasured.
- `vineta-k`: white highlight lines on the ramp edges against the ceiling and
  building ledges, present in ours and not in the original. This is the one
  crop that fits the stand-in specular in
  [hd-lit-billboard-specular-and-gantry-leftovers.md](hd-lit-billboard-specular-and-gantry-leftovers.md),
  and it is not yet checked.
- `talons`: scattered small particle sprites in the sky, in different places
  in ours. Position, not brightness: a particle timing or capture moment
  difference, not a material.
- `anulpha-pass`: a bright glow column on the right pillar in ours, dim in the
  original. Unexplained.

**Capture mismatch, do not lane these.**

- `talons` (heading drift 1.72 deg, camera 0.29 units, race clock -7.2 s): kept
  in the totals, but dropping it moves every block share by under one point.
  Its scenery error is partly pose, and its sky particles are not in the same
  places.
- `talons-assegai-c1` (0.54 deg, clock -2.3 s): same; marginal.
- `talons-pack` (`hd-race/`): excluded, see Set.
- Race clock drift on the 11 other pairs runs from +0.13 to -1.42 s (three
  positive). The largest offsets, `moa-therma` and `modesto-heights` at about
  -1.4 s, are also the pairs most likely to catch a timed board in a different
  state. That is why the advert on the left wall (row 3) is not read as a
  missing decal: on `vineta-k` the FX-350 text is absent in ours.
- HUD digits: on `vineta-k` the lap clock reads 0.13.7 in both frames; on
  `the-amphiseum` it reads 0.13.6 in both. The HUD row still needs a HUD-only
  mask before it is a lane.
- Ship and camera heading are within 0.2 deg on the 11 other pairs; pose is
  close enough for a per-block read, not for per-object claims.
- No other craft appear in these Time Trial pairs. The plume cannot be split
  from the hull at 120 px, so it sits inside row 6.

## Next Steps

1. ~~**Measure first, then lane: ceiling light beams and glow (row 5).**~~ Measured and wired 2026-10-26, see
   "Top-right band, measured and wired" below. The beams are `dc_lightcone` read raw (its taps are gamma-7 units);
   Vineta K's rim lines were a stand-in specular on a program with no half-vector power. Rows left: the open list
   below.
2. **Measure, not a lane: HUD-only share.** Claim: the HUD's own pixels carry
   under 5% of the 13-pair error, so the HUD row is an estimate artifact.
   Method: mask the HUD layout rects from
   [hd-hud.md](../../docs/formats/hd-hud.md) and take the low-frequency error
   inside the mask over the 13 pairs. Acceptance: one number in this thread.
   If it is above 10%, open a HUD text-alpha lane.
3. **Measure, not a lane: left wall and centre gate.** Claim: rows 1 and 3
   (39% of low-frequency error together) are many draws, not one. Method: a
   per-draw ablation on two circuits at the same moment. Acceptance: one or two
   named draws explain at least half the block's low-frequency error on both
   circuits. Only then name a lane.

## Top-right band, measured and wired (2026-10-26, `hd-top-right-band`)

Claim (row 5): ours runs too bright in tx 9-12, ty 0-1; acceptance, mean luma bias within +2 on
`the-amphiseum` and `vineta-k` with the craft and track blocks unchanged. **Half true, half open.**
Evidence and method are in `docs/formats/rcsmaterial.md`, "A program with no `LG2` reading a dot product
draws no specular; the cone's taps are sRGB-decoded". Per-slot ablation on both pairs, then a live RPCS3 frame
dump for the cone's sampler words.

- **`vineta-k` (ramp-edge lines): a specular the program does not compute.** Slot 257 (`lambertzeroalpha`
  `martin_alum_grill`) has `LG2`s on its lightmap curve only; `Program::has_dot_power` is `false`, so it
  and every program like it draws no specular. Band +14.9 to -3.1 (error 25.1 to 9.1), whole frame +3.1
  to -0.2. Overshoots the acceptance the other way: -3.1.
- **`the-amphiseum` (light shafts): the cone's taps are gamma-7 units** (live draws 724 and 730, `@0x1520`
  matched, address words `0x60710101` and `0x60750102`). Band +16.1 to +5.7 (error 23.5 to 14.4). Not within
  +2.
- **`talons` sky particles and `anulpha-pass` pillar glow: not touched, still open** (not measured; the first
  is capture-mismatched).
- 13-pair band mean +6.3 to +3.0 (error 13.1 to 10.4). Per pair, before to after (bias, error): moa-therma
  +7.1/11.6 to -1.3/5.1, sebenco-climb +8.4/15.1 to +5.5/13.0, chenghou-project +2.7/8.3 to +1.2/7.2,
  tech-de-ra +2.9/6.1 to +2.1/5.5; the other seven moved under 0.5. Worse in absolute error: Talon's and
  Talon's Assegai left wall +0.5 (the cone streaks the original does not show), Vineta K track block +0.1.
- Omega ships both materials as PS4 containers (unread); 2048 neither: not checkable, reasons on the page.

### Open from this lane

1. **Amphiseum band +5.7, 3.4 over the no-cone floor.** Shaft width or the `s` scale (0.49596); alpha
   composition is the same as the authored blend `SRC_ALPHA, ONE`. Needs the cone's geometry against
   the original's at the same pose, or the live `s` read off the draw's fragment constants.
2. **Vineta K band -3.1.** Slot 96's strip is grey where the original's is blue (hue); the tower's
   yellow-lit glass is dimmer than the original's.
3. **Modesto Heights, Metropia and the Talon's pairs band** are unmoved by either rule (+5.6, +0.3 with
   15.6 error, +9.2): another cause, never ablated.
4. **Talon's cone:** same decode applied, not independently verified (no matched frame that frames a
   cone).

## Later

- The left-wall advert timeline (table row 3): capture a same-moment frame
  before calling it a missing decal.
- Craft hull high-frequency error (table row 6, low/raw 0.50): AA or texture
  detail; not a lane until the hull's own draws are checked.
- The `anulpha-pass` pillar glow (row 5) and the `talons` sky particles: one
  circuit each, not yet explained.
- Refresh this table after any HD lane merges (same method, same 13 pairs).
