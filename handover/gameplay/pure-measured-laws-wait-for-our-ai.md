# Pure's measured craft laws: what waits for our AI

2026-10-20, `pure-physics`. Pure's craft update was diffed against Pulse's and every difference
measured live in PPSSPP: [pure-physics.md](../../docs/physics/pure-physics.md), with the gate
table per law. Evidence for the addresses:
[psp-pure-usa/craft-physics.md](../../docs/ghidra/functions/psp-pure-usa/craft-physics.md).

## Open

- **The speed laws** (confidence 92), held at Pulse's: no engine doubling and grounded drag
  `-0.0025`, which together halve Pure's acceleration and keep its top speed. Re-apply: in
  `crates/pure/src/race.rs` set `engine_output_double` to `Sourced::measured(false)` and
  `drag_ground` to `Sourced::measured(-0.0025)` and flip `the_speed_laws_are_measured_and_held`.
  On, the Pulse gate holds 12/12, Pure's sweep walls go A `69 -> 37`, B `18 -> 50`, laps
  `49.8 -> 54.5` s novice and `41 -> 44` s Ace, and the full suite's one red is the pad test
  below. This is the feel the maintainer reported: Pure on Pulse's laws accelerates twice as hard
  as the original.
- **Sideshift `200`** (confidence 92), held: Pure's files author no `sideshift`, so a Pure craft
  cannot sideshift at all today. Re-apply: set `CRAFT_LAWS.code_sideshift` to
  `Some(Sourced::measured(SIDESHIFT))` and flip `the_sideshift_is_measured_and_held`; the loader
  fills it for the player and every opponent. On Pulse's laws alone, Ace races walled
  `1104 -> 1650` (seed 2 `405 -> 999`, a wreck) and the sweep's Ace A `38 -> 51`; on top of the
  speed laws it was neutral, so release it with them.
- **Bank-to-yaw `50`** (confidence 92), held at Pulse's `30`. Re-apply: in
  `crates/pure/src/race.rs` set `CRAFT_LAWS.bank_to_yaw` to `Sourced::measured(50.0)` and flip
  `the_bank_to_yaw_gain_is_measured_and_held`. On, Pure's opponent sweep Ace walls went
  A `14 -> 42`, B `25 -> 265`, and Ace races wrecked 4 times where they wrecked none.
- **Pitch `0.1` air, `0.4` grounded, damping `1.5`** (confidence 92), held: Pure flies Pulse's
  `<pitch>` stand-in (`0.3`/`1.0`/`3`). Re-apply: set `CRAFT_LAWS.code_pitch` to
  `Some(Sourced::measured(CODE_PITCH))` and flip `the_pitch_is_measured_and_held`. On, sweep Ace
  walls A `14 -> 53`, B `25 -> 72`; 2 Ace wrecks; Ace end shield `64.4 -> 52.8`. The grid's
  `-0.6` pitch damping is not ported either.
- **The launch boost** (confidence 88), held: `oag_pure::race::START_BOOST` (windows `0.1`/`0.4`/
  `0.5` s, `x0.8`/`x1.2`/`x4.2` for `1.0` s, the player's engine alone, `player_only`). Re-apply:
  set `CRAFT_LAWS.code_start_boost` to `Some(Sourced::measured(START_BOOST))` and flip
  `the_launch_boost_is_measured_and_held`; the loader and the player-only field gate are wired.
  On, sweep novice walls A `23 -> 73` and Ace races wrecked 4 times. It needs
  `engine_output_double` false (Pure's multipliers apply to an undoubled engine).
- **The Pure pad replay test.**
  `replay_ground_truth::pure_replays_reproduce_catch_a_tampered_input_and_ghosts_change_nothing`
  needs a lap from its pad (our `oag_ai::Driver`, default tuning, on the player's craft) inside
  7,200 ticks. The pad pins itself against the wall at the end of Vineta K's start straight on
  Pulse's laws and on Pure's alike; at Pure's acceleration it crawls too slowly to lap. Red with
  the speed laws on, which is why they are held. The options put to the lead: hold (taken), land
  them and leave the test red for the driver lane, or give the Pure pad a longer budget.
- **The inertia frame.** Pure's `Body_Integrate` (`0x089345e8`) is Pulse's instruction for
  instruction and `Craft_Construct` passes Pulse's `0.9` and `(12, 8, 12)`, so Pure applies its
  tensor in Pulse's world axes (confidence 88, read; not captured separately). Pulse's frame is
  held for our AI (`three-measured-air-laws-wait-for-our-ai.md`); Pure's goes with it.
- Not separated live: the brakes, lateral grip, weathervane and speed-pad functions (Pulse's laws
  by reading). The EU pressing was not compared.
- **Classes beyond Vector and Venom were not run**: both available profiles (a fresh one and the
  maintainer's) are rookie saves with only those two unlocked. The laws are class-independent
  code reading the class block; Flash, Rapier and Phantom were not watched.

## Next Steps

1. Fix the pad's pin at Vineta K's first corner (our driver, not physics) first: it is what
   blocks the speed laws, the biggest felt difference. Trace it with a per-120-tick print in
   `replay_ground_truth.rs`'s `record()` (position, speed, wall ticks). 1-2 hours.
2. When our driver flies Pulse's held air laws (`driver-vs-new-physics`), re-run Pure's sweep with
   each hold released, one at a time:
   `OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all -E
   'test(/opponent_autopilot_sweep::pure/) | test(/opponent_autopilot_sweep::races_others/)'
   --no-capture` with `OAG_SWEEP_IMAGE=pure` (about 10 minutes). 1 hour a law. Use more seeds
   than four: a player-only launch moved the AI sweep's novice walls `23 -> 73`, so the columns
   also measure the start's reshuffle, not only the law.
3. To measure another class: unlock it in a profile (or write the unlock flag) and rerun
   `scripts/psp-pure-force-capture.py`. 30 minutes once unlocked.
