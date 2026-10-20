# Our AI's disc-backed gates flip on noise

2026-10-20, `driver-vs-new-physics`. Lead's name for the follow-up: `ai-gate-robustness`.
The four measured air laws (Pulse's world-axis inertia diagonal, pitch-input gate and in-air
`weight_distribution` torque; HD's torque, Omega and 2048 inheriting it) are on. Landing them
showed that this engine's AI gates measure chaos as much as the AI. Method, the per-perturbation
counts for every configuration and what changed in our AI:
[airborne-ground-truth.md, "Our AI against the air laws"](../../docs/physics/airborne-ground-truth.md#our-ai-against-the-air-laws).

## Open

- **The flippiest gates now judge a distribution (2026-10-20, `ai-gate-robustness`)**:
  `ai_clean_lap_gate`, `speed_plan_ground_truth` and the autopilot's every-circuit run
  race each row over five airborne-gravity scales (`oag-raceplay`'s `tests/perturb`) and
  assert a median or majority against a bound taken as the worst median over nine offsets.
  Held out: median 0, max 2 failures per offset, against the single runs' median 5, max 8;
  `air_trim` off still fails them (method, tables:
  [airborne-ground-truth.md, "The gates judge a distribution"](../../docs/physics/airborne-ground-truth.md#the-gates-judge-a-distribution-2026-10-20-ai-gate-robustness)).
  **Still single runs**: `ai_dekonstruct_black_ground_truth`, `ai_dekonstruct_symptoms_ground_truth`,
  `ai_fork_split_ground_truth`, the HD/Omega/2048 original-field tests (hooked to
  `OAG_PERTURB`, each failed at most 2 of 9 scales), and, not hooked at all,
  `oag-game`'s `original_ai_ground_truth` Talon's field and `oag-net`'s Eliminator over the
  wire (other crates; `tests/perturb` would need a home both reach). Unsettled, not
  asserted: `06_Track` VENOM and `29_Track` RAPIER plans.
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

- **The cage's AI effect is weapon deaths, not walls** (2026-10-20): over 108 Black races
  each way, wall-dominant deaths 39 and 39, weapon-dominant 9 to 20; `Airborne` rescues 755
  to 37, replaced by reset-zone rescues and cage resets
  ([collision-cage.md](../../docs/ghidra/functions/psp-pulse-usa/collision-cage.md#what-the-cage-does-to-our-ai-2026-10-20-ai-gate-robustness)).
  Whether the original's field takes the same extra weapon deaths is not measured. The
  2026-10-20 margins `cage-laws-red` listed for Black's table are not re-pinned.
- **2048's inherited bias is held** (chosen): `vita_2048_original_ai_ground_truth::the_field_finishes_park`
  respawns once at `eps = 0` with it (1 of 9 perturbations, 0 of 9 without). Measure the bias on
  2048 itself (Vita3K) to release it; Omega keeps HD's.

## Next Steps

1. Convert Black's board and the other single-run AI gates the same way if they start to
   flip (each about `+4x` its test time); give `tests/perturb` a home `oag-game` and
   `oag-net` can reach. Half a day.
2. The graded fallback in `oag_raceplay::speed_plan::hull_plans_for`: measure it on
   `06_Track` RAPIER (fast mode in 4 of 9 medians) with `print_baseline_source` and the
   held-out offsets. Half a day. Chosen work.
