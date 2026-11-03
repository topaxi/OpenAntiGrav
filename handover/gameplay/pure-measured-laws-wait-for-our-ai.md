---
status: measure
titles: [pure]
updated: 2026-11-03
summary: "Laws all on and our driver adapted (2026-11-03); open: residual Novice gap, unseparated laws, other classes"
size: S (40)
difficulty: medium (35)
---

# Pure's measured craft laws: what still waits for our AI

2026-10-20. Lane `pure-physics` diffed Pure's craft update against Pulse's and measured each
difference live in PPSSPP ([pure-physics.md](../../docs/physics/pure-physics.md); the addresses
are in [psp-pure-usa/craft-physics.md](../../docs/ghidra/functions/psp-pure-usa/craft-physics.md)).
Lane `pure-laws-on` then judged each law against a noise distribution of our AI's gates.

**On:** the speed pair (no engine doubling, grounded drag `-0.0025`), the launch boost, the
sideshift `200`, and the pitch (`0.1`/`0.4`, damping `1.5`, `0.6` on the grid). The page's "What
is on and what is held" section has the method, the verdict table and the tip's per-lap and
end-of-run shields.

## Open

- ~~The pitch-alignment gate as a title axis~~: `CraftLaws::pitch_gate` since `ba5e58047`, Pure
  `measured(false)`; `pure_race_ground_truth::pure_crafts_fly_without_the_pitch_gate_and_pulse_crafts_with_it`
  checks every seated craft (2026-11-03).
- ~~Bank-to-yaw `50` costs our Ace~~: inside the noise bar on today's tree, walls median
  `1853 -> 1814` (p `0.88`), unperturbed `1745` against the `1899` base
  ([pure-physics.md](../../docs/physics/pure-physics.md#our-driver-on-pures-laws-re-measured-2026-11-03-pure-ai-measured-laws)).
- ~~Novice pays for the speed pair~~: the cost was our driver's mistakes into tight corners,
  which Pure's weaker brake cannot recover. `Driver::blunder` no longer rolls there. Novice is
  back inside the bar against the pair-undone base: walls `598` median, end shield `89.2`;
  unperturbed `508` and `90.8`
  ([ai.md](../../docs/gameplay/ai.md#mistakes-are-never-into-a-tight-corner-2026-11-03-pure-ai-measured-laws)).
  Still open: the residual walls gap with the rule on both sides, `520 -> 598` (p `0.067`), and
  the rule's `+1.1` s Novice lap (p `0.019`).
- ~~The bare `oag_ai::Driver` pins on Vineta K's first wall~~: the replay test's pad was
  never seated on the line (its windowed search started at sample `0`, the grid is near `817`).
  Seated now; the bare pad laps Pure
  ([pure-physics.md](../../docs/physics/pure-physics.md#the-pure-pad-replay-test)).
- ~~The inertia frame~~: world-axis since 2026-10-21, read off Pure's own `Body_Integrate`
  ([pure-physics.md](../../docs/physics/pure-physics.md#the-inertia-frame-is-pulses-world-axes-2026-10-21-deviations-to-original)).
  A Pure capture with the up axis well off world Y (a banked corner or a ramp) would take it
  from 85 to a measurement.
- **Not separated live:** the brakes, lateral grip, weathervane and speed-pad functions (Pulse's
  laws, by reading). The EU pressing was not compared.
- **Classes beyond Vector and Venom were not run.** Both available profiles are rookie saves.

## Next Steps

1. Judge any later driver change on Pure with the bar in
   [pure-physics.md](../../docs/physics/pure-physics.md#the-noise-bar) (the scratch `OAG_X_EPS`
   knob, one `it_m_o` binary copied per state and run directly, about 20 s a sample). Keep it
   only when no metric regresses at `P < 0.01` against the law-on base.
2. To measure another class: unlock it in a profile and rerun `scripts/psp-pure-force-capture.py`.
   30 minutes once unlocked.
