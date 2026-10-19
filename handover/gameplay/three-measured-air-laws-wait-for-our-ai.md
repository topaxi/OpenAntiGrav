# Three measured air laws wait for our AI to fly them

2026-10-20, `airborne-capture`. Three laws are measured against the original and held
off, because this engine's own AI (our driver, its speed plan and the autopilot built on
the original's driver) regresses under them. Measurements, the 8-test table and how the
causes were separated:
[airborne-ground-truth.md](../../docs/physics/airborne-ground-truth.md). Landed and on:
the pitch-input gate (all titles) and HD's class gravity scale on flight gravity (HD,
Omega and 2048).

## Open

- **Pulse's world-axis inertia diagonal** (confidence 90). Commits `9a2c5b017` (law) and
  `3ee2b2bae` (oag-physics and oag-ai references), reverted by `c6116853b` and `9d4c1fb45`:
  re-apply with `git revert 9d4c1fb45 c6116853b`. Breaks `ai_clean_lap_gate::track_07_flash`,
  `track_07_venom`, `track_14_phantom` and
  `quiet_replay_ground_truth::a_quiet_replay_fires_the_same_locked_missile_in_a_moving_pack`;
  makes `29_Track RAPIER` and `06_Track VENOM` clean (`speed_plan_rapier_reversed`,
  `speed_plan_venom_forward` then ask for `NOT_CLEAN` to shrink, which is allowed). Pure
  stays rotated (chosen) until a `pure-physics` lane measures it: with Pulse's frame Pure's
  opponent stint walls rose `69 -> 541`.
- **Pulse's in-air `weight_distribution` torque** (confidence 85). Law `34639304f`, on until
  `3dde79c9d` switched the default off (re-apply: `git revert 3dde79c9d 7e1743a9e`, the
  second restoring its oag-physics references). Breaks `ai_clean_lap_gate::track_01_phantom`
  and `speed_plan_flash_forward` (`06_Track FLASH`, walls at samples 1236/1237). It also
  cut the whole-race autopilot's rescues by more than half (novice `62 -> 24` and `68 -> 16`).
- **HD's in-air `weight_distribution` torque** (`-4.0` on Feisar, confidence 85). The flip is
  `CraftLaws::air_pitch_bias` measured `true` in `crates/hd/src/race.rs` (Omega and 2048
  inherit `true`). On, HD's whole-race autopilot gains a net 4 rescues over 32 races, every
  one on `12_sol_2` (novice seeds 1 and 4, Ace seeds 1 and 3, switch on).
- HD's grounded hover residual is a separate thread:
  [hd-handling-against-rpcs3.md](hd-handling-against-rpcs3.md).

## Next Steps

1. Re-apply the three laws on a branch, then run the bar: the eight tests above
   (`OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all -E
   'test(track_01_phantom) | test(track_07_flash) | test(track_07_venom) |
   test(track_14_phantom) | test(speed_plan_flash_forward) | test(speed_plan_rapier_reversed)
   | test(speed_plan_venom_forward) | test(a_quiet_replay_fires_the_same_locked_missile_in_a_moving_pack)'`)
   and the sweep (`OAG_SWEEP=1 OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay
   --run-ignored all opponent_autopilot_sweep --no-capture`, about 20 minutes). Fix our
   driver and plan; never re-baseline a gate to hide a regression. 2-4 hours a law.
2. To separate causes, force the frame in `oag_raceplay::launch_hover::inertia_frame`
   (return `Body`) or the bias in `start.rs`'s `air_pitch_bias`, and rerun the eight.
3. HD: trace the whole-race autopilot on `12_sol_2` with the bias on and off
   (`autopilot_original_ground_truth::wall_sweep`, `OAG_SWEEP_IMAGE=hdfury`,
   `OAG_SWEEP_TRACKS=/data/environments/12_sol_2/track.vex`, `OAG_SWEEP_TRACE=1`) and find
   where and why it is rescued. 1-2 hours.
4. Not taken on here: the Talon's Junction crest lifting off 8 ticks late.
