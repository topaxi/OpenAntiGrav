---
status: ready-to-wire
titles: [omega, 2048]
updated: 2026-11-16
summary: "Engine light law read on Omega 1.07 and 2048 (vertex light, colour by era, Fury skin, mag strip); open: wiring, live check (port needs rebuild)"
size: M (55)
difficulty: medium (45)
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

- **Not wired on Omega or 2048.** `engine_light` is `false` in both titles' `Flare`
  (`crates/omega/src/lib.rs`, `crates/2048/src/lib.rs`).
- **No live value.** The native port does not start (`libfmt.so.9`, `libZydis.so.4.1`,
  `libZycore.so.1.5` are gone from the host; it needs a rebuild of its source tree). The
  read recipe is on the Omega page, "Live check".
- **Omega's shader side is inherited from 2048's program**, by constant name and record layout.
- **Unread:** the 21 other producers of the light table (weapons, effects, one more light per ship in
  the ship's own render), what arms the transition tint's input, 2048's writers of the two ship flags
  and its mag-strip gate (`FUN_81000930(0x8153fc40, -1)`), 2048's jitter ranges, and whether HD
  scales its jitter by `1 - blend`.
- **Jitter on Omega is 0.2 in position and 1.0 in range** (HD: 0.1 each), confidence 75. On an HD-era
  craft that is a range of 0 to 3 units redrawn every flare update: a visible flicker. Check it
  against a short capture before trusting it.
- **Stale comment:** `oag_title::flare::Authored::engine_light`'s doc says the Vita consumer is unread
  and that the light floods white on HD's law; the wire lane rewrites it.

## Next Steps

1. Turn `Authored::engine_light: bool` into data: the three bases with their gains, the mag-strip
   colour and gain scale, the jitter sizes (HD 0.1/0.1, Omega 0.2/1.0, 2048 inherited from Omega) and
   whether the jitter scales by `1 - blend`. HD keeps its values; Omega and 2048 take the table above.
2. Pick the base per craft: Fury skin as HD does today (`RaceView::hd_trail_red`'s flag), the 2048 era
   from the roster (`oag_omega::race::ERA_2048_ROSTER`; on 2048, a native `Ships\` craft against an
   `hdships\` one), the mag-strip colour from the craft's instantaneous mag-floor contact, on 2048-mode
   circuits only (`RaceDefaults::circuit_lineages`).
3. Reuse `oag_raceplay::engine_light` and the `spu_lights` scene block; bind them to track and hull on
   both titles. The original caps a draw at six lights chosen in table order by bounds overlap; ours
   sums every light, which is the same picture while fewer than seven reach a draw.
4. Tests that fail if the law is dropped: one per base (a 2048-era craft's record is
   `(3.0, 0.5, 0.1)` with range 30 +- 1, a `concept1` hull's is `(40, 10, 4)`, a classic hull's is
   `(4, 10, 40)`), one for the mag-strip swap, and `crates/tables/tests/enginelight_ground_truth.rs`
   extended to Omega's and 2048's files (`3.0f` / `30.0f` on all twenty 2048-era craft).
5. Pairs to re-render: `omega/the-amphiseum` and `omega/modesto-heights` (expected warm nozzle and
   housing, the craft there is a Fury-skin hull if the walk picked `concept1`), a 2048-mode Omega pair
   such as `omega/queens-mall-2048` (expected a dim red pool on the floor behind each craft), and
   `2048-vita/moa-therma` (blue means a classic `hdships` hull or a mag-strip start; check which).
   Boxes: the nozzle housing and the floor directly behind the craft.
6. Live check when the port is rebuilt, before or after wiring: the recipe on the Omega page.
