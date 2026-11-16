---
status: measure
titles: [omega, 2048]
updated: 2026-11-16
summary: "Wired on Omega/2048 as Title data; open: strength vs the original (six-light cap, fragment combine), live check"
size: S (35)
difficulty: medium (50)
---

# Omega's and 2048's engine light is HD's light with two more colours and a wider reach

2026-11-16, lane `engine-glow-re` (RE only). The idle engine glow on the grid craft is one point light
per craft, submitted by `EngineFlare_SubmitVertexLight` and received by every material that declares
`DiffuseVertexLight<N>`. Evidence:
[Omega](../../docs/ghidra/functions/ps4-omega-eu/engine-light.md),
[2048](../../docs/ghidra/functions/vita-2048-eu-v104/engine-light.md). Confidence 85 for the
producer (two executables agree, HD's live capture covers the shared branch), 80 for the falloff on
2048, 60 on Omega (its GCN programs were not read).

The law, at rest (`blend = 0`):

| Craft | Colour | Range |
| --- | --- | --- |
| HD-era hull, classic skin | `(4, 10, 40)` | `Radius` (1.0 to 2.0) |
| HD-era hull, Fury skin (`concept1`, `nitro`, `chrome_c1`, `vr_fury`, `detonator`) | `(40, 10, 4)` | same |
| 2048-era craft (team `GameVersion="2048"`) | `(3.0, 0.5, 0.1)` | `Radius` (30) |
| any craft over a mag strip, 2048 modes | `(1.5, 0.1, 3.0)` | its own |

Position `flare_node.pos - flare_node.z * Distance + jitter.xyz`, range `Radius + jitter.w`, both
jitters scaled by `1 - blend`; boosted colour `base * (1 + gain * blend)` with gain 10 (HD-era), 2.2
(2048-era), times 0.2 over a mag strip. Per vertex: `colour * max(0, N.L) * max(0, 1 - d / range)`,
which is what `crates/mesh/shaders/spu_light.wesl` already computes at exponent 1.

## Open

Wired 2026-11-16 (lane `engine-light-wire`): `oag_title::flare::EngineLightLaw` is `Title` data on HD, Omega
and 2048; `oag_raceplay::engine_light::record` places it; the base is the 2048 era by roster directory,
the Fury skin by `hd_trail_red`, the strip by `mag_floor_fx::Shown::on` on a gated circuit (Omega: a
`circuit_lineages` circuit, 2048: all). 2048's strip gate is closed as read: `FUN_81000930(0x8153fc40, -1)`
is `GameMode_IsHdLineage`. Results and per-box numbers: `docs/ghidra/functions/ps4-omega-eu/engine-light.md`,
"Wired". HD is pixel-identical on `hd/the-amphiseum`.

- **Strength against the original is not closed.** The hue is right (Amphiseum's orange rim on housing, vents
  and fins; Moa Therma's blue), but all four housing boxes read worse in MAE because ours is brighter:
  Modesto Heights' original housing is near-black where ours rims orange, Moa Therma's blue is stronger and
  wider. Candidates, none tested: the Omega/2048 fragment combine of the vertex light is unread (ours is HD's
  `SVC1` add before the albedo); one original frame's wander may sit anywhere in `Radius` +- 1; our hull's ambient
  is paler than the original's before any light; the original binds at most six lights per draw by bounds overlap
  (ours sums every light), which is not expected to matter for HD-era lights (range at most 3) but can for the
  2048 era's range 30.
- **Queens Mall's floor pool is barely visible** (floor red 162 -> 167 against the original's 179): the
  original's own pool is not a clear disc either on the stored frame; a boosted craft would show it.
- **Wander flicker not checkable.** Ours flickers by a mean 3.5 to 6.1 (0-255) in the housing box between
  consecutive ticks; the stored pairs hold one original frame, so the original's flicker is unknown. Omega's
  0.2 / 1.0 stays as read (confidence 75).
- **No live value.** The native port was not run for this lane. The recipe is on the Omega page, "Live check".
- **Unread:** the 21 other producers of the light table, what arms the transition tint's input (the tint is
  not wired), 2048's jitter ranges (inherited from Omega), whether HD scales its jitter by `1 - blend`, whether
  2048's `hdships` roster has `ship+0x73fd` clear, and `vr_fury` on Omega (the Fury test is the `_c1`/`_n1`/`detonator`
  suffix).

## Next Steps

1. Read Omega's or 2048's fragment program for the vertex light's use (`DiffuseVertexLight0diffuse`,
   `scripts/vita-gxp.py`, `scripts/vita-usse.py`): is it added before the albedo as on HD, or scaled?
2. Live check on the rebuilt native port: the recipe on the Omega page, one run; confirms the wander and the
   base per craft.
3. Render the Amphiseum and Modesto pairs with the table capped at six in table order by bounds overlap
   (`FUN_0165b890`) against the sum, mainly for the 2048 era's range 30.
4. Wire the transition tint once its arming input is read.
