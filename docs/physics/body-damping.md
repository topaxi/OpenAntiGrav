# The rigid body's velocity damping on Pulse and Pure

`Body_Integrate` ends each step with `velocity -= velocity * h * 0.01` and the angular twin
`omega -= omega * h * 0.01`. The address, the constructor's store and HD's live measurement are on
[the HD page](../ghidra/functions/ps3-hdfury-eu/body-damping.md); Pulse's own read is
[rigid-body.md](../ghidra/functions/psp-pulse-usa/rigid-body.md) ("The linear damping really is
`0.01`"). This engine carries the pair as `oag_physics::laws::BodyDamping`; Pulse's is
`BodyDamping::PULSE` (`0.01`, `0.01`).

| Title | Pair | Source | Flown |
| --- | --- | --- | --- |
| Pulse PSP | `0.01` / `0.01` | read (`0x0884e368`/`0x0884e374`), linear measured per frame below (confidence 90), angular measured on roll steps ([roll-step-response.md](roll-step-response.md), "The damping", confidence 85) | **yes**, `Laws::PULSE` since 2026-11-12; held a day for [de Konstruct Black's first jump](#de-konstruct-blacks-first-jump-under-player-input) |
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
so it holds whatever `Laws::PULSE` flies.

## What it moved

With `Laws::PULSE.body_damping = BodyDamping::PULSE` (first measured 2026-11-12 on main
`97304bed1`, the two de Konstruct rows re-measured the same day on `8b0581a74`):

- **Committed hashes**: `oag-physics` and `oag-ai` determinism (three tests) and `oag-raceplay`'s
  `VANILLA_FOLD`, regenerated in their own commit; `BodyDamping::NONE` reproduces the previous
  constants bit for bit.
- **`oag-trace` `replay::tests`**, three free-body self-checks: they now step with
  `BodyDamping::NONE` (a fraction of the velocity is not a force, so the harness's heavy test mass
  cannot make it negligible). Landed.
- **The lone-craft gate** (`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`):
  twelve clean, 0 respawns both ways; laps `0.0`-`1.3` s slower, `07_Track` `0.2` s faster
  (16 `38.8 -> 39.0`, 03 `38.7 -> 39.0`, 02 `35.0 -> 35.2`, 10 `38.2 -> 39.5`, 05 `33.4 -> 33.5`,
  04 `37.2 -> 38.0`, 09 `44.8 -> 45.0`, 14 `42.0 -> 42.4`, 01 `31.7 -> 31.9`, 13 `36.8 -> 37.0`,
  06 `35.3 -> 35.3`, 07 `49.2 -> 49.0` s). The test prints no shield, per lap or at the end.
- **The eight Pulse trajectory tests the HD lane listed**:

| Test | Compares against | Damping off | Damping on | Verdict |
| --- | --- | --- | --- | --- |
| `ai_clean_lap_gate::track_14_phantom` | our plan | pass | pass | no longer moves |
| `original_ai_ground_truth::at_easy_the_lower_stop_reads_what_the_original_logged` | the original's log | pass | pass | no longer moves |
| `eliminator_finish_ground_truth::the_chosen_kill_target_is_what_the_eliminator_ends_on` | ours | pass | pass | no longer moves |
| `speed_plan_ground_truth::speed_plan_phantom_forward` | our plan | pass | pass | no longer moves |
| `plasma_timing_ground_truth::the_plasma_releases_flies_and_detonates_on_the_originals_ticks` | the original's probe | pass | fail | **closer to the original**: re-base |
| `hover_contact_ground_truth::a_reseeded_replay_lifts_off_the_crest_on_time_with_the_cage` | the original's capture | pass | fail | **one tick later than the original**: open residual |
| `autopilot_original_ground_truth::..._without_a_rescue_05` | ours (the original's driver on our physics) | pass, 4 of 5 clean | 2 of 5 clean, every rescue the first jump's pit | **the original falls there too**: re-based, below |
| `ai_dekonstruct_symptoms_ground_truth::de_konstruct_symptoms_phantom` | ours | pass, field 2 stops, 4 respawns | pass, field 3 stops, 7 respawns (bound 6 stops) | no longer moves past its bound, below |

Three more moved in the affected test-data run on `8b0581a74`:

| Test | Compares against | Damping off | Damping on | Verdict |
| --- | --- | --- | --- | --- |
| `slipstream_ground_truth::a_fire_is_a_second_out_of_the_world_and_four_seconds_on_at_a_slower_speed` | ours (the official mod) | pass, transit `31.1` units after the press | fail, `29.67` | **the test's reference**: the guest places the entrance 30 units on from the progress it reads as the press tick begins, the test measured from the tick after (one tick's travel); from the guest's reading `32.6` and `31.1`. Re-based on that reading |
| `ai_pair_contact_push_ground_truth::two_opponents_forced_side_by_side_push_their_lanes_apart` | ours (`Ai_PlanLane`'s push) | pass, rival placed and touching on the left | fail, rival placed on the right, touching on the left | **the test's reference**: the push reads the rival's side at the contact, and the closing craft is past the centre line by then; the test now judges "away" from the side at the first contact |
| `speed_plan_ground_truth::speed_plan_rapier_forward` | our plan | pass, `06_Track` clean on 15 of 17 offsets | fail, 2 of the 5 gate builds, 11 of 17 offsets | **open**: the landing coin after `06_Track`'s gap at 1196-1200; walls at 1232-1242 stay unresolved in clean builds both ways |

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
against `+0.0170`. Re-based to a one-tick lag (`RESEEDED_LIFTOFF_LAG`), kept as an open residual.

## De Konstruct Black's first jump, under player input

The flip was held a day because two tests went red at de Konstruct Black's first jump
(`05_Track` forward, the lip at line samples 205-208, [ai.md](../gameplay/ai.md), "de Konstruct
Black: the first jump is a magstrip"): the original Autopilot's port fell into the pit under the
upper road's lip on three runs of five, and (on main `97304bed1`) one PHANTOM field craft looped
relaunching short of the lip. The question was physics: what the original's craft keeps of its
climb over the strip's convex top.

**The capture.** PPSSPP v1.20.4, software renderer, muted, `pulse-psp-usa` (`UCUS98712`), Time
Trial, de Konstruct Black, VENOM, the profile's Assegai, from the grid save state. Per run
`psp-trace.py --warmup 2 --hold cross --mag-lock --place=<pose> --place-repeat 2 --ticks 180`: the body is
written to a pose before the ramp, facing along it at a set speed with no rotation, and recorded
with the throttle held and no steer. Never the Autopilot. The capture also read the magnet-lock
blend `craft+0x280` per frame (`--mag-lock`'s column; `oag_trace::Trace` reads past it). Ours is
`oag-trace run --hold cross` over the same rows (seeded once from row 0, the recording's frame
times). The line pose is the spline's racing line at `(-407.91, -5.23, -530.12)`, tangent
`(-0.9952, -0.0826, 0.0517)`; "right 6" is six units along the sample's lateral, onto the ramp's
plain `Floor` third. A run clears if the craft passes x `-690` above y `55` (the upper road); a
short run meets the lip's face at x about `-650`, y `58` to `68`, and falls into the pit.

| Placement | Original | Ours, damping on | Ours, damping off |
| --- | --- | --- | --- |
| line, 120 / 130 / 140 / 142 | short ×4 | short ×4 | short ×4 |
| line, 143 | short | short | **clears** |
| line, 144 (two captures) | short, short | short, short | **clears**, **clears** |
| line, 145 | short | short | **clears** |
| line, 146 (two captures) | clears, clears | clears, clears | clears, clears |
| line, 147 | short | short | **clears** |
| line, 148 (two captures) | short, clears | clears, clears | clears, clears |
| line, 150 | clears | clears | clears |
| right 6, 125 / 130 | clears, clears | clears, clears | clears, clears |
| right 10, 130 / left 6, 130 | short, short | short, short | short, short |
| the port's pose, 30 / 24 / 18 ticks before takeoff | short ×3 | short ×3 | short ×3 |

On the line at 130 units/s (the held-throttle pace the flat run-up settles at) the original's lock
lets go on frame 67 with `78.5` u/s of climb, the craft leaves the ground on frame 75 at `126.1`
u/s with `74.4` of climb and meets the lip at y `65.6`; ours with the damping takes off on the same
frame at `125.9` with `74.6` and the same apex, without it at `126.8` with `75.1`. Six units right
the original's lock is off from frame 42, it takes off with `85.1` of climb and clears.

So **on the authored line, under player input, the original's craft falls short from every
placement up to 145 units/s**, goes either way from 146 to 148 (146 cleared twice, 147 fell, 148
fell once and cleared once: a chaotic band) and clears at 150, and **six units off the strip it clears at 125 and 130**: confidence **85** (twenty-one
placements, one emulator, no hardware). Ours agrees with the damping on 20 of the 21 (the miss is
the 148 that went short once), without it on 15: the damping is what puts
our threshold in the original's band. This also measures [ai.md](../gameplay/ai.md)'s chosen
takeoff-line move off the strip as the original's own physics: off the strip the original clears
where on it does not.

**The Autopilot port.** It arrives on the strip at about 139.9 u/s, five units left of the line
(further onto the strip), and holds that speed up the ramp; the lock lets go on the ramp's top
with `62` u/s of climb and it meets the lip at y `60.8`. Placed at its own pose on that lap (tick
2646, 2652, 2658 of the unperturbed run) with the throttle held, the original's craft falls short
three times of three (apex `62.3`, `60.3`, `58.2`; ours `63.8`, `61.5`, `57.7`). The port's own
thrust keeps it about 5 units/s faster than held throttle to the takeoff; on our faithful physics
it falls short there too. So the port's rescue at that lip is the original's physics, not a gap in
ours: `autopilot_original_ground_truth`'s 05 row no longer counts a rescue from the first jump's
pit (`the_original_falls_here_too`, x `-665..-615`, z `-535..-490`), and any other rescue still
fails it. Before the re-base, damping on: 2 of 5 clean, `[(6578, 1), (6443, 0), (6643, 1),
(6670, 1), (6449, 0)]`; after, 5 of 5 pass the rule. Why the port holds 139.9 units/s where held
throttle settles at 130 is not chased here.

**The PHANTOM field.** On `8b0581a74` the damped field no longer loops: 3 dead stops (samples
205-207) and 7 respawns against 2 and 4 undamped, under its bound of 6 stops. The loop the
`97304bed1` measurement saw (19 stops, 23 respawns) does not reproduce on this base.

**The drift lever, not landed.** Zeroing `oag_ai::Driver::drift` while `Line::is_takeoff` holds,
measured on `05_Track` field, seed 1, dead stops / respawns (lone laps moved at most a tick; all
`21_Track` cells 0 / 0 in every column):

| Class | undamped | undamped, lever | damped | damped, lever |
| --- | --- | --- | --- | --- |
| VENOM | 2 / 3 | 0 / 4 | 2 / 6 | 0 / 4 |
| FLASH | 3 / 5 | 2 / 7 | 3 / 5 | 2 / 3 |
| RAPIER | 1 / 7 | 1 / 5 | 2 / 5 | **4 / 5** |
| PHANTOM | 2 / 4 | **3 / 6** | 3 / 7 | 2 / 6 |

It does not hold everywhere (RAPIER damped and PHANTOM undamped get worse, single seed), and the
field it was for passes without it, so it stays out.

**The test.** `crates/trace/tests/dekonstruct_takeoff_ground_truth.rs` replays seven of these
captures (`data/traces/dekonstruct-takeoff-*.csv`: line 130, 143, 144, 145, 146, right 6 at 130,
the port's pose 2652) and asserts ours clears exactly where the original did. Green with
`Laws::PULSE`'s damping; with `BodyDamping::NONE` it fails on 143, 144 and 145.

Checked for the other titles: Pure, HD, Omega and 2048 already fly the same pair (table above);
this is a Pulse flip, not a 2048 or Omega finding.
