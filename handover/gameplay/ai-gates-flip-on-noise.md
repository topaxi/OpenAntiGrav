# Our AI's disc-backed gates flip on noise

2026-10-20, `driver-vs-new-physics`. Lead's name for the follow-up: `ai-gate-robustness`.
The four measured air laws (Pulse's world-axis inertia diagonal, pitch-input gate and in-air
`weight_distribution` torque; HD's torque, Omega and 2048 inheriting it) are on. Landing them
showed that this engine's AI gates measure chaos as much as the AI. Method, the per-perturbation
counts for every configuration and what changed in our AI:
[airborne-ground-truth.md, "Our AI against the air laws"](../../docs/physics/airborne-ground-truth.md#our-ai-against-the-air-laws).

## Open

- **The gates are a single exact run.** With every law off, an airborne pitch torque of `1e-5`
  to `1e-2` (the bias is `8`) regresses 0 to 7 of the 58 AI-gate tests (median 4):
  `ai_clean_lap_gate`'s 48 rows, `speed_plan_ground_truth`'s 8 slices,
  `quiet_replay_ground_truth`'s moving pack and
  `autopilot_original_ground_truth::the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue`.
  The same holds outside them: `ai_fork_split_ground_truth`, `ai_dekonstruct_black_ground_truth`,
  `original_ai_ground_truth`'s Talon's field (`oag-game`) and `oag-net`'s Eliminator over the
  wire flip on 0 to 3 of 16 tests per perturbation with every law off. A gate should judge over
  seeds or perturbations, as the lead's interim bar does (median at most laws-off + 2, max at
  most laws-off max + 2), not one bit-exact run.
- **Plan selection is the amplifier.** Whether `oag_ai::SpeedPlan::adapt` verifies a hull's plan
  decides whether a craft follows it, the reference at its yaw share (`0.77` for Triakis on
  Assegai's plan) or nothing; it flips on tiny trajectory changes. A graded fallback (verify the
  reference at the share on that hull and keep the better) would damp it. Chosen work, not RE.
- **A hull wedges into the floor after `01_Track`'s jump** (sample ~150, `1.3` under the
  surface, forward velocity 165, not moving). `adapt` now fails it after 600 ticks; the
  reference build and the race's own stall rescue (`oag_race::recovery::STALL_SPEED`, a
  velocity test) still cannot see it. The collision continuity that lets a landing hull sink
  that far is the real fault.
- **05_Track's first lip after a reset** (`ai_dekonstruct_symptoms`, RAPIER field): every dead
  stop follows a `ResetZone` rescue at samples 88-103 by about 70 ticks, in either inertia
  frame. Laws off `2 1 1 2 1 1 1 6 2`, all four `2 2 2 3 4 0 4 4 3` per perturbation.
  [ai.md](../../docs/gameplay/ai.md), "After a reset, the relaunch falls short".
- **06_Track's gap landing** (samples 1229-1237): our driver keeps steering in the air (yaw rate
  to 1.2 rad/s) and lands about 6 units off its line into the wall, in either frame. Steering
  held straight in the air is far worse (median 19 regressions with every law off); a landing-
  aware air steer is open.

- **The cage and the air laws together (2026-10-20, `cage-laws-red`)** turned two single runs
  red, both inside the noise: `oag-net`'s Eliminator over the wire (now 10800 ticks; its doc
  comment has the 18 finish ticks) and Black's PHANTOM seed 1 (1 -> 2, the row comment has both
  lists). Destroyed per Black board over the 9 perturbations went from mean 11.9 (cage off) to
  15.2 (on), about 2.6 standard errors: worth a look by whoever owns the cage's AI effect; the
  cage-off runs used Airborne rescues the cage-on runs did not need. Margins on Black at
  `eps = 0`, cage on (destroyed / bound, forward then reversed): at the bound VENOM 1 (0/0,
  0/0), FLASH 2 forward 0/0, FLASH 3 forward 1/1, RAPIER 1 forward 1/1, RAPIER 2 reversed 2/2,
  PHANTOM 1 forward 2/2; one under FLASH 1 forward 1/2, FLASH 2 reversed 0/1, VENOM 2 reversed
  0/1, RAPIER 1 reversed 2/3, RAPIER 3 2/3 both, PHANTOM 1 reversed 1/2, PHANTOM 2 reversed
  3/4, VENOM 3 forward 0/1. Not re-pinned.
- **2048's inherited bias is held** (chosen): `vita_2048_original_ai_ground_truth::the_field_finishes_park`
  respawns once at `eps = 0` with it (1 of 9 perturbations, 0 of 9 without). Measure the bias on
  2048 itself (Vita3K) to release it; Omega keeps HD's.

## Next Steps

1. Turn the perturbation sweep into a tool (a test-only hook that does not touch
   `oag_physics`'s determinism, e.g. seeds on the race rather than a force term) and gate the
   AI on its distribution. About a day.
2. The graded fallback in `oag_raceplay::speed_plan::hull_plans_for`. Half a day, measured with (1).
