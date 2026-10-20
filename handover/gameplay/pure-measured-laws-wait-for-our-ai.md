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

- **The pitch-alignment gate must become a title axis before anyone turns it on.** Pure's
  `Ship_UpdatePitch` (`0x0892caf8`) has no `Ship_TestPitchAlignment` call, measured live. Ours is
  a bare const, `oag_physics::engine::PITCH_GATE_ON` (off), read for every title.
  `driver-vs-new-physics` is re-applying Pulse's held air laws, and the gate is one of them. Turned
  on for every title, it would give Pure a gate under the measured pitch it now flies. Make it a
  `CraftLaws` axis, as `air_pitch_bias` is, with Pure off (measured).
- **Bank-to-yaw `50`** (confidence 92) is held at Pulse's `30`. The test was on top of the speed
  pair, the launch and the sideshift, with 12 airborne-pitch perturbations of 8 seeds each.
  - Ace race walls: 7 of 12 samples land past the base's worst (`3052`-`4338` against `2339`,
    `P 0.002`), with Ace laps of `45`-`46` s and respawns.
  - The median moves `1911 -> 2594` (Mann-Whitney p `0.13`). Half the perturbations go badly;
    the other half come out slightly better.
  - To re-apply, in `crates/pure/src/race.rs`:
    1. set `CRAFT_LAWS.bank_to_yaw` to `Sourced::measured(50.0)`;
    2. flip `the_bank_to_yaw_gain_is_measured_and_held`.
- **Novice pays for the speed pair.** The pair landed on the lead's decision while outside noise
  in both directions.
  - Ace is better: race walls `-38 %`, sweep A `-75 %`.
  - Novice is worse: race walls `593 -> 786` (every sample past the base's worst), end shield
    `88.2 -> 85.7`, sweep B `16 -> 51`.
  - The law is measured, so this is our driver at half the acceleration. It is a driver item
    for `driver-vs-new-physics` or a later AI lane.
- **The bare `oag_ai::Driver` pins on Vineta K's first wall.**
  - The pad: default tuning, on the racing line alone, with no speed plan, no field and no hull
    yaw ceiling.
  - It hits the wall at the end of the start straight and stays pinned, on Pulse's laws and on
    Pure's alike. It crawls at about `10` u/s on Pulse's and about `5` on Pure's.
  - The Pure replay test no longer uses it: its pad is now `Race::autopilot_wants`, which laps
    in `2206`-`2250` ticks. Pulse, HD and 2048 keep the bare pad.
  - This is a driver bug: with no plan, the bare driver does not back off a wall it is pressed
    against.
- **The inertia frame.** Pure's `Body_Integrate` (`0x089345e8`) is Pulse's, so Pure's world-axis
  tensor is held with Pulse's (`three-measured-air-laws-wait-for-our-ai.md`).
- **Not separated live:** the brakes, lateral grip, weathervane and speed-pad functions (Pulse's
  laws, by reading). The EU pressing was not compared.
- **Classes beyond Vector and Venom were not run.** Both available profiles are rookie saves.

## Next Steps

1. Once our driver handles a stronger yaw coupling, re-judge bank-to-yaw, about 15 minutes of
   machine time. Use the bar in
   [pure-physics.md](../../docs/physics/pure-physics.md#the-noise-bar):
   1. Apply the airborne `OAG_X_EPS` pitch knob as an uncommitted patch.
   2. Build the `it_m_o` test binary once per law state.
   3. Run `opponent_autopilot_sweep::pure` and `::races_others` with `OAG_SWEEP=1
      OAG_SWEEP_IMAGE=pure OAG_SWEEP_SEEDS=8`, per difficulty, over the twelve `eps` values. Each
      job takes 12-25 s, so a dozen run in parallel.
   4. Hold if any metric regresses at `P < 0.01` (Mann-Whitney, or `k` past the base's worst).
2. Fix the bare driver's wall pin (step 1 of a driver lane): trace it with a per-120-tick print in
   `replay_ground_truth.rs`'s `record()` with the Pure pad set back to the bare driver. 1-2 hours.
3. To measure another class: unlock it in a profile and rerun `scripts/psp-pure-force-capture.py`.
   30 minutes once unlocked.
