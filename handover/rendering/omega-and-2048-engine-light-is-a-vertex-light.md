---
status: measure
titles: [omega, 2048]
updated: 2026-11-16
summary: "Engine light wired and live-confirmed; open: warm-pixel acceptance, N scale, hull-tone recipe (albedo factor 2.4-3.1x)"
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
- **Ours' lit panels are brighter than the original's brightest, and the hull's own tone is the cause (narrowed, 2026-11-16,
  `omega-hull-tone`, `docs/ghidra/functions/ps4-omega-eu/hull-tone.md`).** Housing box `(890, 690)-(1010, 800)` on Modesto, engine
  light off both sides, linear light: original `(0.045, 0.051, 0.044)`, ours `(0.101, 0.097, 0.105)` (`2.2, 1.9, 2.4` times).
  Measured on the original by writing the live lighting object: the sun constant equals ours' (`sun * 1.6` is in the process; the
  hull's bound ambient was not verified); `Ship diffuse brightness` is a live neutral term worth about 20 % of the box at `0.25`
  (ours reads it nowhere); specular, reflection and a premultiplied `K * ambient` or `K * sun` are ruled out. What is left is how
  ours' hull combines its diffuse terms (`2.4` to `3.1` times after the K term): albedo, the unnormalised normal or the bound
  ambient, not separated (confidence 55). Ours' knockouts are confounded by `zone_grade`.
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
3. **Hull tone (two reads, then wire).** Read 1: on the port write `Ship diffuse brightness` (`+0x164`, base `0x800eb2898`+12) to `0`
   then `1` and diff the per-draw constant region around the `sun * 1.6` copies: what moves with it is where `K` enters and
   the ambient the hull really binds. Read 2: the Feisar_c1 housing's vertex colour times albedo texel, divided by what ours' hull
   multiplies, should be about `1 / 2.8` (`1 / 2.4` to `1 / 3.1` by channel) if the factor is albedo; about `1` if it is the
   normal (read the hull world matrix scale in the port; feed the sun sum the unnormalised normal) or the ambient. Wire only what a
   read names; `K` waits on read 1 (the `+0.03` per unit on this box is a fit, not a law).
   **Acceptance:** `omega/modesto-heights`, tick 100, engine light off, housing box `(890, 690)-(1010, 800)`: linear mean within
   15 % of `(0.045, 0.051, 0.044)` (sRGB about `(37.7, 43.2, 42.2)`, ours today `(65.5, 63.2, 72.7)`), the track panels
   (`scripts/hull-tone-box-linear.py` on a floor box) unmoved, and a second pair (Amphiseum, Moa Therma) falling the same way.
   Wire it for the hull draw only: `zone_grade.rs` builds its rig through `Light::authored` too (`panelmap-sun` owns it).
   Evidence and numbers: `docs/ghidra/functions/ps4-omega-eu/hull-tone.md`.
4. Boxes for the merge: Queens Mall floor `(1140, 650)-(1260, 800)` must rise about `+3.6` red
   (the original, light on against off; the held branch gives `+4.8`, which passes). Modesto,
   Amphiseum and Moa Therma housing boxes are **not** judged against the stored frame; re-grab the
   three originals at a forced range (`scripts/wipeout-pc-engine-light-live.py --calibrate=0`) and
   render ours at the same range if a still comparison is wanted.
