# The rigid body's velocity damping on Pulse and Pure

`Body_Integrate` ends each step with `velocity -= velocity * h * 0.01` and the angular twin
`omega -= omega * h * 0.01`. The address, the constructor's store and HD's live measurement are on
[the HD page](../ghidra/functions/ps3-hdfury-eu/body-damping.md); Pulse's own read is
[rigid-body.md](../ghidra/functions/psp-pulse-usa/rigid-body.md) ("The linear damping really is
`0.01`"). This engine carries the pair as `oag_physics::laws::BodyDamping`; Pulse's is
`BodyDamping::PULSE` (`0.01`, `0.01`).

| Title | Pair | Source | Flown |
| --- | --- | --- | --- |
| Pulse PSP | `0.01` / `0.01` | read (`0x0884e368`/`0x0884e374`), linear measured per frame below (confidence 90), angular measured on roll steps ([roll-step-response.md](roll-step-response.md), "The damping", confidence 85) | **no**: `Laws::PULSE` carries `NONE`, see [below](#why-pulse-does-not-fly-it-yet) |
| Pulse PS2 | `0.01` / `0.01` | read (`0x0015d088`); inherited from PSP, no PS2 measurement | as PSP |
| Pure | `0.01` / `0.01` | read: `Craft_Construct` passes the pair and `Body_Integrate` (`0x089345e8`) is Pulse's loop ([pure-physics.md](pure-physics.md)); not measured live | **yes**, `oag_pure::race::CRAFT_LAWS` |
| HD, Omega, 2048 | `0.01` / `0.01` | [HD page](../ghidra/functions/ps3-hdfury-eu/body-damping.md) | yes |

## The per-frame check on Pulse

`crates/trace/tests/body_damping_pose_walk_ground_truth.rs` steps `oag_physics::step` once from
every recorded pose of `data/traces/talons-junction-standing-start.csv` (PSP, Talon's Junction,
Venom, Assegai, held throttle from the scripted emulator: player input, not Autopilot) and compares
our next velocity with the original's along the recorded direction of travel, as a fraction of
`speed * dt`. Rows below 8 units/s, with a `dt` outside `0.015..0.018`, on a throttle edge or with a
wall contact are dropped: 169 frames remain.

| Capture | Frames | Median residual, damping off | With `BodyDamping::PULSE` | Difference |
| --- | ---: | ---: | ---: | ---: |
| `talons-junction-standing-start.csv` (asserted) | 169 | `+0.00961` | `-0.00043` | `0.0100` |
| `talons-junction-clean-lap.csv` | 2,849 | `+0.02682` | `+0.01674` | `0.0101` |
| `talons-junction-time-trial-lap.csv` | 679 | `+0.02409` | `+0.01405` | `0.0100` |

HD's twin read `0.01002` off and `0.00003` on. On a straight held-throttle launch the term is the
whole forward residual, so Pulse's linear damping is `0.01` live, **confidence 90**. On the two
lap captures it removes exactly `0.0100` as well, and leaves a further `+0.014` to `+0.017` of
`speed * dt` that is not this law: those laps steer and brake hard, so it belongs to the
cornering forces (open, `cornering-ground-truth.md`'s territory). The test asserts the off median
in `0.008..0.012` and the damped one under `0.002`; it steps with `BodyDamping::PULSE` explicitly,
so it holds whether or not `Laws::PULSE` flies the pair.

## What it moved

With `Laws::PULSE.body_damping = BodyDamping::PULSE` (measured 2026-11-12 on main `97304bed1`):

- **Committed hashes**: `oag-physics` and `oag-ai` determinism (three tests) and `oag-raceplay`'s
  `VANILLA_FOLD`. To regenerate in their own commit when the flip lands.
- **`oag-trace` `replay::tests`**, three free-body self-checks: they now step with
  `BodyDamping::NONE` (a fraction of the velocity is not a force, so the harness's heavy test mass
  cannot make it negligible). Landed.
- **The lone-craft gate** (`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`):
  twelve clean, 0 respawns both ways; laps `0.0`-`1.3` s slower, `07_Track` `0.2` s faster.
- **The eight Pulse trajectory tests the HD lane listed**:

| Test | Compares against | Damping off | Damping on | Verdict |
| --- | --- | --- | --- | --- |
| `ai_clean_lap_gate::track_14_phantom` | our plan | pass | pass | no longer moves |
| `original_ai_ground_truth::at_easy_the_lower_stop_reads_what_the_original_logged` | the original's log | pass | pass | no longer moves |
| `eliminator_finish_ground_truth::the_chosen_kill_target_is_what_the_eliminator_ends_on` | ours | pass | pass | no longer moves |
| `speed_plan_ground_truth::speed_plan_phantom_forward` | our plan | pass | pass | no longer moves |
| `plasma_timing_ground_truth::the_plasma_releases_flies_and_detonates_on_the_originals_ticks` | the original's probe | pass | fail | **closer to the original**: re-base |
| `hover_contact_ground_truth::a_reseeded_replay_lifts_off_the_crest_on_time_with_the_cage` | the original's capture | pass | fail | **one tick later than the original**: open residual |
| `autopilot_original_ground_truth::..._without_a_rescue_05` | ours (the original's driver on our physics) | pass, 4 of 5 clean | fail, 2 of 5 | **regression**, below |
| `ai_dekonstruct_symptoms_ground_truth::de_konstruct_symptoms_phantom` | ours | pass, field 2 stops, 4 respawns | fail, 19 stops, 23 respawns | **regression**, below |

**Plasma.** Our craft's lead over the original's at the release frame falls from `3.1` units to
`1.7` (x `240.66` and `239.21` against `237.5`); the first three unpinned updates fly `2.205` a tick
against the original's `2.19` (`2.216` off); the bolt detonates on tick 76, the original's frame,
where it ended on 75. The test pinned 75 and an absolute last position (`288.51` against `288.60`)
that matched only because the lead equalled one update's travel; damped, that position is `290.05`,
the remaining lead. The re-base pins tick 76 and the seventeenth release-relative row at `0.25`
(ours `46.77` against `47.01`). The ride rows run up to `0.22` short damped (`0.04` off): the
ride's own pin, which the test already names as short per frame.

**The reseeded crest.** Ours lifts off at 1596 against the original's 1595; without the damping
tick 1595 already read `0.5` against the original's `0`, so the lap's disagreement count is 2 both
ways. One step from each recorded pose of the approach (ticks 1525-1596, the 1583 cage hit
included) is closer to the original's speed with the damping: median `+0.0070` of `speed * dt`
against `+0.0170`. Re-based to a one-tick lag when the flip lands, kept as an open residual.

## Why Pulse does not fly it yet

Both regressions are de Konstruct Black's first jump (`05_Track` forward, the lip at line samples
205-208, [ai.md](../gameplay/ai.md), "de Konstruct Black: the first jump is a magstrip").

- **The original Autopilot's port** (`oag_ai::original::Autopilot`, VENOM, weapons off, five
  perturbed runs): off `[(6416, 0), (6424, 0), (6399, 0), (6611, 1), (6403, 0)]`, on
  `[(6578, 1), (6443, 0), (6643, 1), (6670, 1), (6449, 0)]` (finish tick, rescues). Every rescue
  is a fall into the pit under the lip on an ordinary lap, not after a reset. The port flies the
  original's authored line, which runs up the magstrip: the lock holds to about x `-556`, the climb
  peaks at `85` u/s and is `62` when the lock has let go, still grounded, and the craft reaches the
  lip at about y `60.4` and 112 u/s, a unit or two under it. Off it arrives at 114 u/s and clears in
  four runs of five. The port cannot leave the strip without ceasing to be the original's driver,
  so this is a physics question: what the original's craft keeps of its climb over the strip's
  convex top. Not measured; the next read is a PSP capture with player input on that ramp.
- **Our field at PHANTOM**: one craft falls, and the faithful reset
  ([reset-relocation.md](../ghidra/functions/psp-pulse-usa/reset-relocation.md)) puts it back at
  line sample 95 at `50 + 0.25 * 140 = 85` u/s, from which it reaches the ramp's top at 141.6 u/s
  and the lip a unit low, falls again and relaunches from the same crumb at the same speed: one
  craft looped thirteen times. The relaunched craft drives 8-9 units off the takeoff line, on the
  magstrip side (lock `1.0` from sample 100 to 140), because its driver's drift (line bias, wander,
  inside line) still applies on the run-up. Zeroing the drift while `Line::is_takeoff` holds took
  the damped field to 3 dead stops and 8 respawns (lone best lap 1,332 to 1,333 ticks); it does not
  help the Autopilot's port, which drives no drift.

Without either fixed, a player would see the Autopilot fall at Black's first jump and an
opponent stuck relaunching short of it, so `Laws::PULSE` keeps `BodyDamping::NONE`.
