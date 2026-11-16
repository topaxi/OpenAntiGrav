---
status: measure
titles: [omega, 2048]
updated: 2026-11-16
summary: "Engine light wired on Omega/2048 and confirmed live (hull lit, flickers); open: warm-pixel acceptance test, forced-range curve, N scale"
size: S (30)
difficulty: easy (60)
---

# Omega's and 2048's engine light is HD's light with two more colours and a wider reach

2026-11-16. Three lanes: `engine-glow-re` read the producer, `engine-light-wire` wired it (held,
then merged once the live read showed its housing boxes had judged one flicker frame), `engine-light-re`
measured the original live and read the receivers. Evidence:
[Omega](../../docs/ghidra/functions/ps4-omega-eu/engine-light.md),
[2048](../../docs/ghidra/functions/vita-2048-eu-v104/engine-light.md).

**The held branch's picture is the original's picture.** The original lights the hull with this light
(confidence 90): on the native port the housing's grilles, nozzle rim, fin inner faces and bay walls
take the warm cast on some frames and none on others, because the range is redrawn as `Radius +- 1.0`
(an HD-era craft authors `Radius` 1.0 to 2.0). With the producer switched off the hull stays dark and
the flame in the nozzle stays: the nozzle glow is the flare model, not the light. The stored
`omega/modesto-heights` frame is a draw near zero range, so a single-frame box metric against it gets
worse for a correct light. The premise "the original's light does not wash the hull" was false.

The law, at rest (`blend = 0`), now read live on both eras (eight records each on the Modesto Heights
and Queens Mall grids):

| Craft | Colour | Range |
| --- | --- | --- |
| HD-era hull, classic skin | `(4, 10, 40)` | `Radius` (1.0 to 2.0) `+- 1.0` |
| HD-era hull, Fury skin (`concept1`, `nitro`, `chrome_c1`, `vr_fury`, `detonator`) | `(40, 10, 4)` | same |
| 2048-era craft (team `GameVersion="2048"`) | `(3.0, 0.5, 0.1)` | `Radius` (30) `+- 1.0` |
| any craft over a mag strip, 2048 modes | `(1.5, 0.1, 3.0)` | its own |

Receivers, read on Omega's GCN and 2048's GXP hull programs: per vertex
`colour * max(0, N.L) * max(0, 1 - d / range)`, then
`(ambient + vertexLight + sun * N.L) * texture * vertexColour + specular`. No weight, divide or
exposure term on the light.

## Open

Wired 2026-11-16 (lane `engine-light-wire`): `oag_title::flare::EngineLightLaw` is `Title` data on HD, Omega
and 2048; `oag_raceplay::engine_light::record` places it; the base is the 2048 era by roster directory,
the Fury skin by `hd_trail_red`, the strip by `mag_floor_fx::Shown::on` on a gated circuit (Omega: a
`circuit_lineages` circuit, 2048: all). 2048's strip gate is closed as read: `FUN_81000930(0x8153fc40, -1)`
is `GameMode_IsHdLineage`. Results and per-box numbers: `docs/ghidra/functions/ps4-omega-eu/engine-light.md`,
"Wired". HD is pixel-identical on `hd/the-amphiseum`.

- **`engine-light-wire` merged 2026-11-16 unchanged.** Its `EngineLightLaw` needed no change: colours, ranges, the
  `0.2` / `1.0` wander and the `0.01` floor are all confirmed live on Omega.
- **Its acceptance was the wrong test.** The housing box against one stored frame cannot pass:
  the original's own frames span the whole range (warm-pixel count 142 to 16,232 over 60 frames, 447
  with the light off; the stored frame counts 115).
- **Ours' lit panels are brighter than the original's brightest.** Housing box
  `(890, 690)-(1010, 800)` on Modesto: the original at a forced range of 2.0 rises
  `(+10.5, +2.8, +1.4)` over unlit `(40.6, 44.2, 41.3)`; ours at tick 100 rises `(+20.3, +10.1, +3.9)`
  over `(65.7, 63.4, 72.9)`. The footprint is the same panels. Ours' unlit hull is already 1.6 times
  the original's there, and the light multiplies the same texture, so most or all of the excess is
  the hull's own brightness (confidence 55, ours' range at that tick was not recorded). That belongs
  to the hull-tone threads, not to the light.
- **One term ours has and the original lacks: ours normalises `N`.** The original's vertex program
  uses the world matrix times the vertex normal as is. If the Omega or 2048 hull's world matrix
  carries a scale (HD's is 0.75), the original's light is that much weaker. The scale on Omega was
  not measured.
- **2048's own jitter sizes are unread** (the writer of `flare+0x248`; `FUN_812707dc` and
  `FUN_81271e94` are not it). 2048 runs Omega's `0.2` / `1.0` by inheritance. No live run of 2048's
  executable was made.
- **Unread, not needed for the merge:** the redraw rate of the jitter, what arms the transition tint,
  the producers of the other lights seen in a race (an explosion pair `(500, 200, 50)` range 40 and
  `(20, 5, 0.5)` range 100; `(0.21, 0.3, 1.0)` range 15), the ship's own `(100, 50, 50)` light
  (`FUN_01307d40`, `ship+0x770c > 0`, off at rest), `vr_fury` on Omega.

## Next Steps

1. Keep `EngineLightLaw` and the receivers as merged. Add the acceptance: render `omega/modesto-heights` at 60 consecutive ticks and run
   `scripts/engine-light-warm-census.py` over them. Pass: the count spreads from the unlit count to
   its maximum (the original: 10th to 90th percentile 368 to 12,458, median 2,700, mean 4,712,
   unlit 447); a constant count means the range draw is not reaching the shader.
2. Add a forced-range path for the test (the jitter's `d` fixed) and compare with the original's
   curve at `Radius + w`: 1.0 gives 3,275 warm pixels, 1.5 gives 9,245, 2.0 gives 13,182 (summed
   excess 106,450, 355,657 and 550,931). Ours' count should follow the curve; its excess is expected
   to run high by the hull's own over-brightness until the hull tone is fixed.
3. Decide `N`: read the Omega hull's world-matrix scale (the flare node's `row2` length is the same
   scale; the live driver can read it once the flare object's address is found) and, if it is not 1,
   feed the light sum the unnormalised world normal on Omega and 2048.
4. Boxes for the merge: Queens Mall floor `(1140, 650)-(1260, 800)` must rise about `+3.6` red
   (the original, light on against off; the held branch gives `+4.8`, which passes). Modesto,
   Amphiseum and Moa Therma housing boxes are **not** judged against the stored frame; re-grab the
   three originals at a forced range (`scripts/wipeout-pc-engine-light-live.py --calibrate=0`) and
   render ours at the same range if a still comparison is wanted.
