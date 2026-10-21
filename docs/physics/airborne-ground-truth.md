# Ramps and the air, measured against the original (2026-10-19)

Lane `airborne-capture`. The question: on Fort Gale Black's first jump (`14_Track`,
PHANTOM) this engine's autopilot reached the lip steering `-20` where the
original steers `-98`, and launched at `vy 75.5` where the original launches at
`67`. The original's driver had already been replayed exactly through ours
([ai-controls.md](../ghidra/functions/psp-pulse-usa/ai-controls.md#the-airborne-driver-replayed-live-2026-10-19)),
so the gap was the craft's state: the force law, the rail, or the angular law.

**Answer: the angular law.** The linear force law reproduces the original's
next velocity from its own frames to about `0.05` units/s per tick on the ramp,
at the lip and in the air. The original's rail (`Ai_SnapToLane`) is undone
exactly. What differs is how torque becomes rotation: Pulse applies its inertia
diagonal in **world axes**, this engine applied it in body axes, and on a banked
ramp under full steering the two part. Three smaller pitch terms were measured
on the way: the pitch-input gate and the in-air `weight_distribution` torque,
both live.

## How it was measured

- **The capture.** `scripts/psp-autopilot-trace.py` (PPSSPP v1.20.4, software
  renderer, muted, `pulse-psp-usa`, Time Trial, Fort Gale Black, PHANTOM; the
  craft's `<Misc>` read live as `5.5, 13, 3.5`, shields `135/95/95`,
  `weight_distribution 8.0`). It arms the player's Autopilot at GO+60 frames and
  stops once a frame at `Ship_UpdateCraft` (`0x08849618`) for the player's craft.
  It writes `scripts/psp-trace.py`'s columns (so `oag_trace::Trace` reads it) and
  adds the rail's blend (`driver+0x48`) and the own sample's lateral
  (`racer+0x260+0x30`), the driver's raw output (`driver+0x08`), the pitch gate's
  inputs (`craft+0x160`, `*(craft+0x1c4)+0xb10`) and `craft+0x1c0`. Three captures,
  each after a fresh boot and menu walk: 1,300, 1,300 and 2,400 rows.
- **The rail is undone, not ignored.** The AI update runs before the world step
  in the same frame (`ai-controls.md`, "One AI update per world step"), so a row's
  velocity is already past that frame's rail. `Ai_SnapToLane` keeps the speed and
  scales the lateral component by `1 - k`, `k = blend^2 * 0.7 / 4`, which inverts
  exactly: `v_pre = v + lat * dot(v, lat) * k / (1 - k)`, rescaled to `|v|`. On
  the ramp the rail takes up to `0.89` units/s of `vy` a frame from the original
  (its lateral axis has a `-0.31` vertical component there); with it undone, the
  per-tick velocity residual is `0.05`, so the ordering and the inversion hold.
- **The sweep.** `crates/trace/tests/autopilot_ramp_pose_walk.rs` (ignored,
  `OAG_SWEEP`) seeds a `ShipState` from each row with
  `oag_trace::replay::initial_state`, steps `oag_physics::step` once with the
  driver's raw output, the autopilot's engine flags (`uncapped`, `x1.2`) and the
  class's `GravityMul`, and compares the stepped velocity and angular velocity
  with the next row's. Every tick reseeds, so nothing compounds. A second pass
  rolls our physics forward from one row on the original's controls and rail.

```sh
uv run --with websocket-client python3 scripts/psp-autopilot-trace.py \
    --port <port> --out data/traces/fortgale-autopilot-ramp.csv --frames 2400
OAG_SWEEP=1 OAG_RAMP_TRACE=data/traces/fortgale-autopilot-ramp.csv OAG_RAMP_TICKS=1150..1250 \
OAG_RAMP_GATE=1 OAG_RAMP_BIAS=8 cargo nextest run -p oag-trace autopilot_ramp_pose_walk \
    --run-ignored all --no-capture
```

`OAG_RAMP_GATE` and `OAG_RAMP_BIAS` emulate the two pitch terms in the sweep's
input, before they were ported.

## Capture 3's redirect over the second crest is the collision cage

Between ticks 1341 and 1342 of capture 3 (`(-296, 127, 691)`, `up.y 0.65`, no
stun, rail blend 0) the airborne craft takes a one-tick `dv` of about 31 units/s
and then two decaying corrections. No collision mesh is involved: it is the
collision cage, the AI track data's invisible tube
([collision-cage.md](../ghidra/functions/psp-pulse-usa/collision-cage.md#the-cage-is-a-contact-source-live-2026-10-20)).
Our step plus the cage, from each recorded row (`14_Track`, PHANTOM):

| tick | `\|v - next\|` without -> with the cage |
| ---: | --- |
| 1341 | `29.72 -> 0.12` |
| 1342 | `5.07 -> 0.13` |
| 1343 | `2.71 -> 0.13` |

That is the air contact our autopilot missed every lap on Fort Gale's second
jump; since 2026-10-20 the race runs the cage for every craft. A whole race of our
autopilot on the original driver's port (`autopilot_original_ground_truth::wall_sweep`,
PHANTOM, `14_Track`), cage off -> on: respawns `5 -> 0`, lateral offset max `202.5 ->
17.0`, end shield `64.6 -> 95.0`, finish `8,423 -> 7,892` ticks. The original is
rescued `0` times. Reproduce with
`crates/trace/tests/collision_cage_ground_truth.rs`'s sweep:
`OAG_SWEEP=1 OAG_SWEEP_TRACK='Data\Environments\14_Track\track.vex' OAG_SWEEP_CLASS=PHANTOM
OAG_CAGE_CSV=<capture 3> OAG_CAGE_TICKS=1341,1342,1343`.

### HD runs no cage (2026-10-20)

Wipeout HD's Chenghou Project is the one HD circuit whose header sets the cage flag.
Our race ran Pulse's cage there and capped both jumps at about 16.5 above the spline,
with a single-tick vy step of `-35` and `-23`. The original, placed at the same pose and
speed on RPCS3, falls smoothly through the same spot: its largest per-frame vy drop is
`-2.3`. The cage is off for HD by the title
([ps3-hdfury-eu/collision-cage.md](../ghidra/functions/ps3-hdfury-eu/collision-cage.md)).

## The tensor is applied in world axes: measured dynamically

[cornering-ground-truth.md](cornering-ground-truth.md#the-tensor-is-a-world-axis-diagonal-and-that-answers-a-not-c)
showed the engine's momentum column is the world-axis diagonal times the rate at
`100.00 %`, a kinematic identity, and left the crate on its body-space diagonal
until "a sustained-bank capture" could measure the difference. Fort Gale's ramp
is one: up axis about `(-0.25, 0.89, -0.38)`, full left steering and airbrake.

One-step angular-acceleration residual (rad/s², rms, ticks clear of any stun),
body-axis against world-axis, the pitch gate and bias emulated in both:

| capture | ground yaw | ground pitch | ground roll | air yaw | air pitch | air roll |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 2 (1,153 ground, 104 air) | 0.389 → **0.215** | 0.332 → **0.234** | 0.200 → **0.124** | 0.322 → **0.140** | 0.417 → **0.411** | 0.419 → **0.161** |
| 3 (2,123 ground, 199 air) | 0.375 → **0.283** | 0.390 → **0.247** | 0.176 → **0.114** | 0.585 → **0.256** | 0.724 → **0.591** | 0.327 → **0.244** |

Open-loop rollout from tick 1150 of capture 2, our physics on the original's
recorded controls and rail, error against the original:

| tick | phase | body-axis `|dv|`, `vy` error, up error | world-axis |
| ---: | --- | --- | --- |
| 1200 | on the ramp | 1.80, +1.15, 1.37° | 1.45, +0.83, 0.24° |
| 1215 | 11 ticks airborne | 3.97, +2.81, 3.23° | 2.30, +1.73, 1.48° |
| 1245 | 40 ticks airborne | 13.13, +12.44, 7.99° | 1.89, +1.47, 5.87° |

Body-axis, the craft's attitude drifts, the engine's thrust tilts with it, and
the fall is about `12` units/s short within forty ticks; world-axis, it falls
with the original.

**Closed loop**, this engine's own autopilot (`autopilot_original_ground_truth::wall_sweep`,
Assegai, PHANTOM, seed 1), matched by position on the same ramp:

| | lip steer (x -523..-531) | launch `vy` | respawns over the race |
| --- | --- | ---: | ---: |
| original (capture 2) | -81 .. -107 | 66-69 | - |
| body-axis | -10 .. -47 | 75.9 | 5 |
| world-axis | -60 .. -103 | 70-73 | 4 |

The tensor accounts for most of the gap, not all: about `5` of the `8.5`
units/s of launch speed.

**Confidence 90** that Pulse applies the diagonal in world axes (kinematic
identity at `100.00 %` plus this dynamic leg on three captures; 93 since the player-input
leg [below](#under-player-input-2026-10-21-pulse-gate-tensor)). Wipeout HD
rotates its tensor (`body+0x50` read live as `R I R^T`,
[craft-inertia.md](../ghidra/functions/ps3-hdfury-eu/craft-inertia.md)), so the frame is a
per-title law.

### Under player input (2026-10-21, `pulse-gate-tensor`)

The dynamic leg above flew the original's Autopilot (the rail undone, the driver's raw
output for controls), which the maintainer's rule of 2026-10-20 does not accept as a physics
reference. This leg is player input only.

- **The capture.** PPSSPP v1.20.4, software renderer, muted, `pulse-psp-usa`, Time Trial,
  Fort Gale Black (`14_Track` forward, the grid at `(242.2, 111.8, 540.8)` heading `+x`),
  PHANTOM (`psp-drive.py menu --unlock --class-right 3 --track-down 15` from a cold boot),
  the profile's Assegai. Per run `psp-drive.py restart`, then `psp-trace.py --warmup 6 --hold
  cross --hold left [--hold ltrigger] --place=<pose> --place-repeat 2 --ticks 140`: the body is
  written to capture 2's pose at ticks 1170, 1180 and 1190 (position, nose, speed, and the
  recorded bank as `--place`'s roll value), on two consecutive hits, then recorded with
  thrust, full left and the left airbrake held. No Autopilot, so no rail and no engine flags.
  `data/traces/fortgale-player-ramp-{1170,1180,1190}.csv`, plus `-1180nb` with the airbrake
  released.
- **Why `--place-repeat`.** One write left the craft with whatever it carried from 6 s down
  the track, and the first stepped tick kicked it to tens of rad/s on every placement (the
  same kick took the first pitch-gate drops below). Two writes, and it rides the ramp.
- **The sweep.** `autopilot_ramp_pose_walk.rs` with `OAG_RAMP_PLAYER=1` (the control block's
  `stick_steer`/`stick_pitch`, full thrust, `OAG_RAMP_AIRBRAKE_LEFT=1` where the shoulder was
  held, default engine flags, no rail) and `OAG_RAMP_FRAME=world|body`. Rows from tick 5 on,
  with no stun and a velocity residual under `1` unit/s (20 rows on the two runs that met a
  wall), aggregated by `scripts/pose-walk-residuals.py frames --from 5`.

One-step angular-acceleration residual (rad/s²), world axes against body axes:

| phase (n) | yaw rms | yaw median \|r\| | pitch rms | pitch median \|r\| | roll rms | roll median \|r\| |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ground (404) | 0.825 → **0.317** | 0.262 → **0.054** | 1.046 → **0.964** | 0.483 → **0.291** | 0.978 → **0.867** | 0.432 → **0.251** |
| air (98) | 0.393 → **0.073** | 0.384 → **0.032** | 0.771 → **0.527** | 0.359 → **0.208** | 0.593 → 0.629 | 0.404 → **0.198** |

World axes win every cell by median, by about a factor of two and by twelve in air yaw; the
one rms that does not move (air roll) is carried by a few rows the median ignores. The same
sweep on the Autopilot's capture 2 (`OAG_RAMP_GATE=1`, the bias now in the physics) gives the
table above to the third decimal (ground yaw `0.389 → 0.215`), so the tool is unchanged and
the two legs agree. **Confidence 93** that Pulse applies the diagonal in world axes: a
runtime trace on two kinds of drive (the Autopilot's frames with the rail undone, and player
input) plus the kinematic identity, and Pure's `Body_Integrate` read as the same instruction
sequence; a live Pure capture would take it to the top band.

What these runs do not settle: the pitch-gate drops below tumble at `2` to `8` rad/s, and
there neither frame fits (yaw rms about `2.5` in both, the residual growing with the rate),
so some term that scales with the rate is missing from ours. Not chased here.

**On (2026-10-20), a per-title craft law** (`oag_physics::InertiaFrame`,
`oag_title::craft_laws::CraftLaws::inertia_rotates`): Pulse and the PS2 world axes, measured;
HD rotated, measured; Omega and 2048 inherit HD's; Pure world axes since 2026-10-21, read off
its own `Body_Integrate` (Pulse's instruction sequence,
[pure-physics.md](pure-physics.md#the-inertia-frame-is-pulses-world-axes-2026-10-21-deviations-to-original));
it flew HD's rotated frame by choice before. It was ported and held twice (`9a2c5b017`, held by `c6116853b`) because this
engine's own AI gates regressed under it; how they were judged, and what was changed in our
AI, is [below](#our-ai-against-the-air-laws).

- On the tree before the pitch gate and the bias, the player's whole-race autopilot in
  `opponent_autopilot_sweep` (4 seeds, 48 races per Pulse difficulty) gained rescues: novice
  `62 -> 106` with the switch off and `68 -> 91` on.
- On the tree with the gate, the bias and HD's gravity law it lowered them: novice
  `24 -> 14` and `16 -> 7`, Ace `24 -> 22` and `24 -> 25`, opponent stint rescues `6 -> 1`
  and `3 -> 2`. The lone craft's `race_ground_truth` stayed twelve of twelve clean.

## Our AI against the air laws

The three Pulse laws and HD's bias were measured on 2026-10-19 and held a day because this
engine's AI gates regressed under them (`ai_clean_lap_gate`, `speed_plan_ground_truth`,
`quiet_replay_ground_truth`'s moving pack, `autopilot_original_ground_truth`'s every-circuit
run). Lane `driver-vs-new-physics` found two things.

**The gates flip on noise.** With every law off, a constant airborne pitch torque `eps`
added to the force law (`acc.local_angular.x += eps` while the contact flag is clear, a
scratch switch, not in the tree; the bias is `8`) regresses this many of the 58 AI-gate tests
(48 clean-lap rows, 8 speed-plan slices, the moving-pack replay, the original's every-circuit
autopilot), against `0` at `eps = 0`:

| `eps` | `-1e-2` | `-1e-3` | `-1e-4` | `-1e-5` | `0` | `1e-2` | `1e-3` | `1e-4` | `1e-5` | median | max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| laws off | 1 | 4 | 6 | 7 | 0 | 2 | 7 | 4 | 1 | 4 | 7 |
| gate | 1 | 4 | 5 | 6 | 1 | 2 | 8 | 4 | 1 | 4 | 8 |
| tensor | 7 | 4 | 5 | 4 | 7 | 5 | 5 | 8 | 4 | 5 | 8 |
| gate + tensor | 7 | 4 | 5 | 4 | 7 | 5 | 5 | 7 | 4 | 5 | 7 |
| bias + `air_trim` | 3 | 5 | 5 | 3 | 1 | 5 | 7 | 8 | 1 | 5 | 8 |
| bias + tensor + `air_trim` | 9 | 3 | 5 | 6 | 4 | 6 | 7 | 6 | 6 | 6 | 9 |
| all four + `air_trim` | 8 | 6 | 6 | 4 | 2 | 5 | 4 | 7 | 8 | 6 | 8 |
| bias, no `air_trim` | 5 | 2 | 5 | 8 | 5 | 8 | 9 | 6 | 7 | 6 | 9 |
| all four, no `air_trim` | 10 | 8 | 11 | 8 | 6 | 9 | 6 | 9 | 8 | 8 | 11 |

(Improvements, a row newly clean, are not counted.) The mechanism is plan selection: whether
a re-learned hull plan verifies (`oag_ai::SpeedPlan::adapt`), and so whether a craft follows
it, the reference at its yaw share, or no plan, turns on trajectory differences far below
anything a law changes. A single run of a gate row is a coin. **The bar the lead set**
(2026-10-20): a law may regenerate `ai_clean_lap_gate::BASELINE` and
`speed_plan_ground_truth::NOT_CLEAN` when its median is at most laws-off + 2 and its max at
most laws-off max + 2. Each law met it with our AI changed as below; the regenerating
commits carry the counts. The same sweep on `ai_dekonstruct_symptoms`' 05_Track RAPIER field:
dead stops laws off `2 1 1 2 1 1 1 6 2`, all four with `air_trim` `2 2 2 3 4 0 4 4 3`; every
stop follows a `ResetZone` rescue at samples 88-103 by about 70 ticks (the short relaunch,
[ai.md](../gameplay/ai.md)), in either inertia frame.

**What changed in our AI** (ours, chosen, not measured):

- `oag_ai::air_trim`: the driver holds `-weight_distribution / (100 * pitch_air)` of pitch
  while the contact flag the force law reads is clear, the stick a player who knows their craft
  holds against it. Our driver sent no pitch input at all before. Zero with the bias off.
  Read the contact flag the force law reads on entry (`ShipState::grounded`), not
  `grounded_prev`: one frame late, the trim missed every takeoff and landing frame and the
  bias row read `7` rather than `5`.
- `oag_ai::SpeedPlan::adapt`'s runs fail as `Stalled` after 600 ticks without a new furthest
  point on the line. Under the tensor a Triakis hull wedged into the floor after
  `01_Track`'s jump landing (sample ~150, `1.3` units under the surface) with a forward
  velocity of 165 and its index frozen for the whole 22,688-tick verification: no failure
  fired, `adapt` never ran a learning pass, and the race fell back to the reference plan at
  `0.77`. Scoped to `adapt`: the reference build tolerates a hull that wedges and frees
  itself (`01_Track` RAPIER, every law off).
- Tried and dropped, each failing the laws-off check (58 of 58 at `eps = 0`): pitching in
  the air toward the line's tangent (the original's `Ai_ComputePitch` law) and a rate-only
  pitch damper; and steering held straight in the air (median `19` with every law off).

**HD's `12_sol_2` rescues**, the reason HD's bias was held: traced with
`opponent_autopilot_sweep::races_others`, every extra rescue is a `ResetZone` hit by an
opponent on this engine's driver that never flew an autopilot stint, not the original's
driver. With `air_trim`, 12 seeds, novice and Ace (79 paired races): rescues `10 -> 11` with
the switch off and `10 -> 10` on.

### The gates judge a distribution (2026-10-20, `ai-gate-robustness`)

The sweep above is now a tool that touches no simulation crate: `oag-raceplay`'s
`tests/perturb` scales every craft's airborne gravity (`Physical::flight_gravity`, read
only by `oag_physics::passive::gravity` while airborne) by `1 + eps` on the loaded
`Setup`, so the race and every speed plan it learns see it, and `eps = 0` is the race bit
for bit. The race seed was rejected as the knob: it re-draws the roster (a different team
in the lone slot) and no plan build reads the generator. `OAG_PERTURB=<eps>` offsets
every run of every hooked test, for sweeps. All of it is chosen, not measured.

Calibration, every law on (the 58 AI-gate tests, regressions per scale):

| `eps` | `0` | `1e-3` | `-1e-3` | `1e-4` | `-1e-4` | `1e-5` | `-1e-5` | `1e-6` | `-1e-6` | median | max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| airborne gravity | 0 | 7 | 6 | 3 | 7 | 3 | 8 | 5 | 4 | 5 | 8 |

against the force-term sweep's "all four + `air_trim`" row (median 6, max 8): the two
knobs reach the same chaos. Nearly all of it is `ai_clean_lap_gate`'s `06_Track` and
`07_Track` rows and `speed_plan_ground_truth`'s `06_Track` VENOM and RAPIER and
`14_Track` PHANTOM, the gap landing at samples 1233-1241. The other AI gates
(`ai_dekonstruct_black_ground_truth`, `ai_dekonstruct_symptoms_ground_truth`,
`ai_fork_split_ground_truth`, the HD, Omega and 2048 original-field tests in
`oag-raceplay`) failed 0, 5, 1, 0, 1, 1, 1, 0, 0 of 53 over the same scales, no test more
than twice; they stay single runs. The quiet-replay moving pack never failed.

Converted: `ai_clean_lap_gate` (each row the median of five races, one per scale of
`perturb::GATE`), `speed_plan_ground_truth` (each layout the majority of five builds) and
the autopilot's every-circuit run (twelve tests, a majority of five races each). A gate's
median is itself a draw, so its bound is the worst median over the nine offsets of
`perturb::BOUND_OFFSETS`; two speed-plan layouts whose majority flips between offsets
(`06_Track` VENOM, 3 of 9 clean; `29_Track` RAPIER, 8 of 9) are printed and not asserted.
`06_Track` RAPIER's lap ceiling is its slow mode (2,693 ticks, the plan not verifying;
the fast mode, 1,750, in 4 of 9 medians): the plan-selection amplifier in one row.

- **Held out** (`perturb::HELD_OUT`, eight offsets no bound was taken from), the 69
  converted tests failed `2 0 0 0 1 1 0 0`: median 0, max 2, against the single runs'
  median 5, max 8.
- **Strength**: with `oag_ai::air_trim` returning `0` (a scratch edit; the force-term sweep
  read "all four, no `air_trim`" median 8, max 11, which the interim bar rejects), the
  converted gates failed 4, 2 and 4 tests at offsets `0`, `-1.5e-5` and `9e-4`:
  `14_Track` PHANTOM and `speed_plan_phantom_forward` every time.

Cost: five races where there was one, about `+30` s of the full suite's wall clock at its
measured parallelism; the slowest converted test is a speed-plan slice at 28.7 s in a full loaded run (9,233
tests, 364 s).

## The pitch-input gate, live

`Ship_UpdatePitch` (`0x08848d08`) applies the pitch axis only while
`Ship_TestPitchAlignment` (`0x088492bc`) passes,
`dot(hull_up, -located_down) > 0.9`
([engine.md](../ghidra/functions/psp-pulse-usa/engine.md#pitch)). With the input
applied ungated in the sweep, the airborne pitch residual divided by the full
input torque (`input * 0.3 / I_xx`, `1.92` rad/s² at full deflection) is:

| capture | gate open (`> 0.9`) | gate closed (`<= 0.9`) |
| --- | --- | --- |
| 2 | median `-0.002` (n 64) | median `1.25` (n 14) |
| 3 | median `0.003` (n 96) | median `1.08` (n 80) |

So the original drops the whole input exactly where the dot product crosses
`0.9`, on three crossings and both input signs: capture 2 ticks 1236/1237
(`0.9022` open, `0.8984` closed) and 1297/1298, capture 3 ticks 1312/1313
(`0.9009`, `0.8976`). **Confidence 90, 93 since the player-input leg [below](#under-player-input-2026-10-21-pulse-gate-tensor-1). On (2026-10-20)**, after the tensor and the bias: on its own it cost
the original's autopilot driver two rescues on Pulse's `05_Track`
(`autopilot_original_ground_truth::the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue`, twelve tests since 2026-10-20,
tick 7282), which passes with all three on; that test also fails under 2 of the 9
perturbations [above](#our-ai-against-the-air-laws) with every law off. The mechanism is
`oag_physics::engine::pitch_aligned`,
reading the located sample `Environment::track_sample` carries (the entity's `+0xaf0`,
whose `down` is `+0xb10`); with no sample the gate is open, chosen. It is a per-title craft
law (`oag_title::craft_laws::CraftLaws::pitch_gate`, `ShipState::pitch_gate`): Pulse runs it
(measured) and the PS2 with it; Wipeout HD's `Craft_UpdatePitch` (`0x000f0980`) has the same
gate, seen closing live under a held stick (confidence 90,
[below](#wipeout-hd-under-player-input-2026-10-20-hd-air-player-input);
[physics.md](../ghidra/functions/ps3-hdfury-eu/physics.md)); Omega and 2048 inherit HD's;
**Pure has none**: its `Ship_UpdatePitch` (`0x0892caf8`) never calls
`Ship_TestPitchAlignment` (measured, [pure-physics.md](pure-physics.md#pitch-is-three-code-literals)). 2048/Omega: **not checkable** here, no executable read of
either's pitch update and no capture path for Omega. The lone craft's regression gate and
every committed reference were unchanged by it: no scripted or lone-craft run pitches
past the cone.

### Under player input (2026-10-21, `pulse-gate-tensor`)

The closings above were all on the original's Autopilot, its input the driver's raw output
rather than the craft's control block. These are player input.

- **The capture.** PPSSPP v1.20.4, software renderer, muted, `pulse-psp-usa`, Time Trial,
  Talon's Junction (`16_Track`), VENOM, the profile's Assegai. Per run `psp-drive.py restart`
  (some runs), then `psp-trace.py --warmup 6 --hold cross --hold up|down --place=<pose> --ticks
  60|90` on the start straight: nose up 24 to 40 degrees (a tangent with `y` `0.41` to `0.64`),
  or level and rolled 18 or 30 degrees (`--place`'s eighth value), 8 to 15 units over the
  track, 40 to 80 units/s along the nose. `psp-trace.py` now records the located sample's down
  (`spline_down_*`, `*(craft+0x1c4) + 0xb10`), the vector the gate dots the hull up against.
  `data/traces/talons-pitch-{up-30,down-24,down-24-s1,down-24-s3,down-24-s4,roll30-a,roll18-a,roll30-b}.csv`.
  These single-write placements kick the body on the first tick (above), so most runs tumble;
  each row is reseeded from the record, so a tumbling airborne row still tests the gate.
- **The sweep.** `crest_pitch_pose_walk.rs` on every airborne row (`grounded` `0` on both
  sides), dropping rows where `|pos(t+1) - pos(t) - v(t+1) dt|` exceeds `0.05` (a cage or
  escape moved the body, 32 of 396), in three modes: `OAG_CREST_OURS_GATE=1` (the stick goes in
  ungated and our own `engine::pitch_aligned` gates it, off a `TrackSample` built from the
  recorded down), `OAG_CREST_UNGATED=1` (the stick on every row), and the first with
  `OAG_CREST_NO_BIAS=1`. Aggregated by `scripts/pose-walk-residuals.py gate`; the residual is
  over the full input torque `1.923` rad/s², signed by the stick.

| runs with the recorded down (6), closed rows (364) | median | mean |
| --- | ---: | ---: |
| our gate (`pitch_aligned`, the located down) | `-0.010` | `-0.033` |
| ungated | `-0.999` | `-1.001` |
| our gate, bias off | `+0.224` | `+0.031` |

On the one run that rotates slowly through the cone (`down-24`, nose up 24 degrees, d-pad
down), the crossing is between ticks 4 (`0.9028`, open, residual `+0.012` either way) and 5
(`0.8993`, closed: `+0.009` gated, `-0.991` ungated), and the whole closed stretch reads
`+0.001` gated against `-0.997` ungated (31 rows). The open side holds on `up-30` (nose up 30,
d-pad up, it pitches down into the cone and stays there): 38 open airborne rows, median
`-0.006` with the input applied. **That crossing needs the located down, not world down**: on
`up.y` alone tick 4 reads `0.8993`, closed, and the gated sweep misses it by the whole input
(`+1.012`). `down-24` was taken before `spline_down_*` was recorded, so its down is
`down-24-s1`'s at the same tick (same placement, positions within `1` unit, `down` within
`0.001` over the run): `talons-pitch-down-24-sd.csv`, grafted, said so. With the bias off its
closed rows read `+0.266` against the bias' own `0.513 / 1.923 = 0.267`: the in-air
`weight_distribution` torque acts while the gate is closed, live, as the static read has it.

**Confidence 93** for the gate (`0x088492bc`, `dot(hull_up, -located_down) > 0.9`): runtime
on two kinds of drive, both stick signs, closed by pitch and by roll, and our own
implementation reproducing it on 364 rows. Below the top band for want of a second binary
(Pure has no gate, the PS2 is not read).

## The in-air `weight_distribution` torque, live

The bias add at `0x08848d98` (`add.s f12, f12, f13`, `f13 = *(craft+0x6c) + 0x90`)
was stopped on live, in the air, 40 times: `f13 = 8.0` every time and
`f12 = 0.3 * axis + 8` (`38 = 30 + 8`, `-22 = -30 + 8`). In the sweep, airborne
pitch residual with the world-axis tensor:

| capture | no bias: mean, rms | `+8`: mean, rms |
| --- | --- | --- |
| 2 | `+0.329`, `0.493` | `-0.175`, `0.411` |
| 3 | `+0.334`, `0.665` | `-0.142`, `0.591` |

`8 / I_xx = 0.513` rad/s²; the bias moves the mean by `0.50` and lowers the rms
on both captures. A regression of the residual on pitch rate, yaw rate and input
puts the remaining constant at `+0.009` (capture 2) and `+0.226` (capture 3) with
the bias: a pitch-rate term the bias does not explain is still in the air.
**Confidence 85** for the bias (static read, the live register on every airborne
frame, and the residual). **On (2026-10-20)**: the mechanism is in
`oag_physics::engine::pitch` (`Dimensions::weight_distribution`, behind
`oag_title::craft_laws::CraftLaws::air_pitch_bias` and `ShipState::air_pitch_bias`). Held
for a day while our AI's gates regressed; this engine's driver now holds it off with pitch
(`oag_ai::air_trim`, [above](#our-ai-against-the-air-laws)).

| title | bias | origin |
| --- | --- | --- |
| Pulse (PSP, PS2) | on | measured |
| Wipeout HD | on | measured under player input: each team's own `+0x9c` (`-4` Feisar and Harimau, `+4` Assegai), the airborne pitch residual closing to `-0.007`..`+0.009` with it, confidence 92 ([below](#wipeout-hd-under-player-input-2026-10-20-hd-air-player-input)) |
| Omega | on | inherited from HD |
| 2048 | on | inherited from HD via Omega (2026-10-20): held off until HD's was re-measured under player input, then switched to the original's (maintainer's decision); `vita_2048_original_ai_ground_truth` passes 18 of 18 with it at `eps = 0`. 2048's own pitch update is unread |
| Pure | off | **measured**: Pure's `Ship_UpdatePitch` has no `weight_distribution` term, and Pure authors none ([pure-physics.md](pure-physics.md#pitch-is-three-code-literals)) |

2048/Omega: **not checkable** here (no executable read of their pitch update, no Omega
capture path).

On Pulse, with it on, the autopilot survived far better: `opponent_autopilot_sweep`'s whole-race player autopilot (4 seeds, 48 races per
difficulty) went from 62 to 24 rescues on Novice with the switch off and 68 to 16 with it
on, and from 29 to 24 and 34 to 24 on Ace; the opponents' paired-fork stints went from
5,317 to 4,570 wall ticks.

## The air pitch laws against a drop test (2026-10-20, `pulse-air-pitch`)

No committed capture held the pitch axis in the air, and the Fort Gale check above read the
driver's raw output (`ai_pitch`), not the control block the craft reads. These captures hold
the control block's own axis.

- **The capture.** PPSSPP v1.20.4, software renderer, muted, `pulse-psp-usa`, Time Trial,
  Talon's Junction (`16_Track`), VENOM, the profile's team (Assegai's handling reproduces the
  rows; shield `135`). `psp-drive.py menu`, then per run `psp-drive.py restart` and
  `psp-trace.py --place 55.2,-41,-205.5,0.94,0.33,-0.18,110 --hold cross [--hold up|down]
  --ticks 90`: at the first breakpoint hit the body is written about `9` units over the start
  straight (`up.y` `0.9998`), nose up `19` degrees, `110` units/s along the nose, no rotation,
  and every tick after it is recorded. `--place` exists because `psp-drive.py place` followed by
  a capture loses about a second of free-running frames; placed `20` units up the craft was put
  back on the track before tick 0 (not traced; the cage's top is about that high).
- **The input.** The new `stick_pitch` column (`*(craft+0x78) + 0x10`,
  `scripts/psp_trace_fields.py`) read `-100` on every row holding up, `+100` holding down and `0`
  centred; `stick_steer` (`+0x00`) read `0` and the ramped `steer` `0` on every row of all four.
- **The sweep.** `crates/trace/tests/crest_pitch_pose_walk.rs` seeds each row with
  `oag_trace::replay::initial_state`, steps `oag_physics::step` and the cage once, and compares
  the angular velocity with the next row's. The gate is taken from world down on this level
  straight.

Pitch-axis angular-acceleration residual on the rows airborne before, during and after the
step (rad/s², ours minus the original's about the craft's lateral axis), against the full input
torque `100 * 0.3 / I_xx = 1.92`:

| run | rows | pitch term ours applied | residual mean | residual rms |
| --- | ---: | ---: | ---: | ---: |
| centred, two runs | 68 | `-0.513` (the bias alone) | `+0.008` | `0.147` |
| d-pad up | 44 | `+1.410` (`-0.513 + 1.923`) | `+0.008` | `0.111` |
| d-pad down | 37 | `-2.436` (`-0.513 - 1.923`) | `+0.001` | `0.137` |

The up run's tick 7 (a one-tick `-12` rad/s² event, `+7.6` in yaw) is left out of its row.
Regressing the residual on the input: coefficient `-0.004` (up) and `0.000` (down). So
`pitch_air` (`0.3`), the axis' sign and `+/-100` scale, the in-air bias and the in-air pitch
damping all hold on the control block's own axis to well under one percent of the input
torque. **Confidence 90** for `pitch_air` as a plain gain on that axis (three inputs, both
signs, the static read). These runs do not test the gate: the sweep gates on world down, and the
lowest airborne `up.y` was `0.912` (the up run; `0.959` on the others), above the `0.9` threshold
on every row, so ours and the original agree on an open gate throughout.

**The weathervane is not settled by these.** It is a yaw term here (the craft is placed with
its velocity along the nose, and the fall builds about `15` degrees of incidence): its yaw
contribution is `0.16` rad/s² rms against a yaw residual of `0.23` rms, median `-0.004`. Fitting
`residual = a + b * vane` puts the airborne coefficient at `-0.16`; without the constant, at
`-0.37`. The `-0.3` is consistent with both and fitted by neither. What would settle it is a
capture with sideslip in the air (a placed velocity off the nose).

## The 1595 crest residual is a hover escape off a wall (2026-10-20, `pulse-air-pitch`)

The measured residual on Talon's Junction's 1595 crest (clean lap, `oag-trace run --reseed 60`):
our nose `3` to `32` mrad off the original's along its up axis from 1596 to 1610. It is not an
air pitch law.

- **One step from each recorded row** (the sweep above, on `talons-junction-clean-lap.csv`):
  on the ten rows airborne on both sides the pitch residual is a steady `-0.34` rad/s², the
  sign that *closes* the free-run gap, and on the six half-grounded rows (`0.5`, rear probe
  only) it averages `-0.23`, also against the gap.
- **Rolled forward from 1594 with one state component put back each tick:** nothing put back,
  `-12.1` mrad at 1608; velocity, `-13.4`; **position alone, `+1.1`**; attitude, `-1.8`. So the
  error grows through where the craft is.
- **Where it is:** after one step from 1594 ours is `0.202` units below the record along up, and
  at no other tick on the crest. The record's own `pos(t+1) - pos(t) - v(t+1) * dt` is `+0.2026`
  along up on that tick and under `0.005` on the ticks around it: the original moved the body
  without moving its velocity.
- **The mechanism.** From the 1594 pose, above `50` units/s, the front probe's nearest hit is a
  `Wall` (`2.652` down, its normal `0.06` off horizontal and `0.309` along the craft's forward).
  The rear hit is derived from it, `6.0 * 0.309` higher: `0.798` above the derived point, inside
  the one-unit penetration escape. The escape is `1 - 0.798 = 0.2017` against the recorded
  `0.2026`. Neither probe is in contact (a wall carries no suspension), and the record's next
  `grounded` is `0`.

Ours missed it twice over: it escaped off `Floor` hits only, and a frame with no probe in
contact dropped every probe's escape. Pulse's gate, `craft+0x208 == 1`, is the hit record's
mesh-shape tag (`Collision_RaycastMesh` writes `1`, `Collision_RaycastBox` `3`;
[collision.md](../ghidra/functions/psp-pulse-usa/collision.md#raycasts)), not the floor class,
and the translate sits inside the probe loop before the type test
([engine.md](../ghidra/functions/psp-pulse-usa/engine.md#the-probes-depenetrate-as-well-as-push)).
**Confidence 85** for both on `Wall` and `Floor` hits (the static reads and one live tick to a
millimetre). A `MagFloor` hit passes the shape tag too, and since 2026-10-21 it escapes
([below](#a-magstrip-passes-the-gate-and-the-escape-fires-on-one-2026-10-21-deviations-to-original-pulse-magstrip-hold)); it was held out by choice
until then. The floor-only gate this replaces (2026-08-12) was a reading of the field as
the floor class, made the same day the shape tag was not yet recovered; nothing measured it
against the original. **On** since
2026-10-20 (`oag_physics::hover::Rig::escape_floor_only` false for Pulse, a field `oag_physics::hover::Rig::mag_floor_escape` replaced on 2026-10-21; the escape summed
before the contact test for every rig).

| | before | after |
| --- | --- | --- |
| one step from 1594, height against the record | `-0.202` | `-0.0004` |
| rolled from 1594, nose error at 1608 | `-14.6` mrad | `-1.5` mrad (peak) |
| rolled from 1594, landing | one tick early (1608) | the original's tick (1609) |
| one-step height misses over `0.05` on five captures (12,838 rows) | 391 | 390 (1594 alone moved) |
| `race_ground_truth` lone craft | 12/12 clean | 12/12 clean (`06_Track` 36.1 -> 35.0 s, `02_Track` 35.1 -> 35.0 s) |

Our AI's gates, with MagFloor held out: `ai_clean_lap_gate` unchanged; `speed_plan_ground_truth`'s
`06_Track` RAPIER goes from clean under 0 of the nine `perturb::BOUND_OFFSETS` to 8 of 9 (moved
from `NOT_CLEAN` to `UNSETTLED`); `ai_dekonstruct_black_ground_truth` VENOM seed 1 forward loses
one craft at offset 0 (bound 0 -> 1). Noise method over the nine offsets, de Konstruct cells over
the old bounds (of 12): before `0 0 0 1 2 1 2 1 1`, after `1 0 0 1 1 1 1 1 1` (median 1 both, max
2 -> 1); destroyed per board, `143` -> `135` summed over the nine.

Pinned by `hover_contact_ground_truth.rs`'s
`the_crest_lifts_off_through_a_wall_escape_with_no_contact` and, in `oag-physics`,
`penetration_escape_fires_off_floor_and_wall_and_off_floor_alone_when_gated` and
`a_wall_under_the_probes_lifts_the_body_with_no_contact`.

**What is left of the `--reseed 60` figure** is unchanged by this: at 1594 that run arrives
carrying `+1.7` units/s of vertical velocity from 1580-1584 (`+0.3` to `+0.7` after the 1580
landing, `+1.33` more on the 1584 cage tick), so its own 1594 step does not reach the wall
escape and its 1595 contact disagrees. That residual belongs to the landing escape at 1579 and
the cage response's vertical component, not to the air.

### A magstrip passes the gate, and the escape fires on one (2026-10-21, `deviations-to-original`, `pulse-magstrip-hold`)

**Static.** `Ship_HoverTwoPoint`'s escape (`0x0884a800`-`0x0884a86c`) is `swc1 f12,0x308(s2)`,
`c.lt.s f12,f20` against `1.0`, `lw a0,0x208(s1)` against `1`, then `Body_Translate`
(`jal 0x0884d9f8`). No magnet-lock read and no class read sit between the height and the
translate; the class (`*(*(craft+0x300) + 12 * craft+0x200 + 0x2454) + 0x6c`, `1` or `3`) is
read only after it, for the contact (`0x0884a874`-`0x0884a8c4`). HD's loop has the magnet-lock
test, Pulse's does not.

**Live**, PPSSPP 1.20.4 software renderer, `time-trial-talons-venom-grid.ppst` (Talon's
Junction, VENOM, Assegai), player input only: the throttle held and this project's
pure-pursuit stick on the authored line, sent through the virtual pad, a breakpoint on
`Ship_UpdateCraft` reading the previous update's hover state (`scripts/psp-magstrip-escape.py`).
One lap, 4,000 updates, the magstrip section spline rows `1150`-`1320`:

| Probe hits | Count | Tag `+0x208` | Lowest height |
| --- | ---: | --- | ---: |
| `Floor` | 7,392 | `1` on all | `1.393` |
| `MagFloor` | 578 | `1` on all | `1.864` (blend `0.2`, entering) |
| `MagFloor`, blend `1.0` | 564 | `1` on all | `2.194` (median `3.011`, max `4.093`) |

So the original's gate admits a magstrip, and on **Talon's Junction's** strip its hold never
lets a probe near one: no magstrip tick of that lap had a height under `1.0`, and the escape
never fired there. The position residuals over the strip (`-0.07` to `+0.19` along up a tick)
are the hold's own `Body_Translate`, not an escape. That is a property of that strip, not of the
hold: on Vertica's crest the escape fires (below).

**Ported** as `oag_physics::hover::MagFloorEscape::Always` on Pulse's two-point rig (the escape
off a magstrip, whatever the lock), which Pure, Omega and 2048 fly too. Confidence **92**: the
static read, the tag `1` on 578 magstrip hits on Talon's Junction, and the escape seen firing
on a magstrip at full lock on Vertica, in the position residual it predicts.

#### On Vertica's crest the original escapes too (2026-10-21, `pulse-magstrip-hold`)

`06_Track` (Vertica White, forward) climbs a vertical magstrip wall (up `+x`, travel `+y`) and
crests over its lip onto a floor (up `+y`): the surface normal turns about `90` degrees over
spline rows `2745`-`2790` (`oag-trace track --track 'Data\Environments\06_Track\track.vex'`),
about `30` units of radius, centre near `(721, 340)`. Ours escaped there off a magstrip on every
pass of `ai_clean_lap_gate::track_06_rapier` (489 escape ticks over about 280 passes across the
perturbed runs, 1-2 a pass, heights `-0.39` to `0.95`, median `0.42`; VENOM 2-3 a pass, median
`0.28`). The escaping probe is almost always the **derived** rear one: above `50` units/s the
original casts probe 0 (local `+4.5`, the nose) and builds probe 1's hit from it,
`hit + (p1 - p0) + up * dot(normal, forward) * 6.0` (`0x0884a168`-`0x0884a2cc`: the record at
`craft+0x1e0` copied to `+0x210`, `craft+0xf0 - craft+0xe0` added, then `craft+0x160` scaled by
`dot(craft+0x1f0, craft+0x180)` and by `0x40c00000`). On a crest whose normal runs ahead of the
craft's up the slope term is positive and the rear height is the nose's minus `6 * slope`:
`1.52 - 6 * 0.295 = -0.25` on ours' first escape tick. `oag_physics::hover::derived_hit` is that
expression term for term.

**Live**, PPSSPP 1.20.4 software renderer, USA, Time Trial, Vertica White, Assegai, player input
only (held throttle, the pure-pursuit stick on the authored line through the virtual pad,
`scripts/psp-magstrip-escape.py`). VENOM from `data/saves/pulse-psp/time-trial-vertica-white-venom-grid.ppst`;
RAPIER walked from that race by `psp-drive.py menu --unlock --any-track --class-right 2` (QUIT
RACE, then the walk), identified by the craft at `(141.3, 2.47, 311.3)` on the Vertica grid and
the hover target `craft+0x2f0 = 4.5` (RAPIER's `5.4` at full lock). Heights are the probe's along
up as `Ship_HoverTwoPoint` stores them (`craft+0x308`), at blend `1.0`, every hit tagged `1` and
of class `3`:

| Class, one pass | Speed at the crest | Escape ticks (height under `1.0`) | Heights |
| --- | ---: | ---: | --- |
| VENOM | `98`-`104` | 4 | `0.86` (nose), `-0.21`, `0.80`, `-0.04` (rear) |
| RAPIER | `88`-`131` | 2 | `-0.05`, `0.46` |

The two deep VENOM escapes show in the next update's position residual along up,
`dot(pos(t+1) - pos(t) - v(t+1) * dt, up)`: `+1.357` and `+1.299` against `1 - h` of `1.213`
and `1.042` plus the hold's own `+0.2` to `+0.25` a tick there, then `-0.91` and `-0.78` the
update after, as the hold pulls back. Ours shows the same bounce (`+0.62` hold translation on
the escape tick, `-0.35` on the next, RAPIER).

Row-aligned against our AI (not the same input: ours flew VENOM at about `124` units/s and
RAPIER at `179` on a line near `z = -1`, the original `100` and `88`-`131` near `z = +10`), the
probe heights agree within about `0.2` over the whole crest at VENOM: the nose falls from `2.73`
to `1.21` (original) and `2.58` to `1.06` (ours) over rows `2740`-`2758`, the rear dips under
zero at row `2758`-`2759` in both (`-0.21`, ours `-0.33`), and both recover above `2.2` by row
`2776`. So **the 06_Track escapes are the original's law**. The `ai_clean_lap_gate` contact
rise on that row (`154` to `419`) did not come from the crest at all: no wall tick lands at rows
`2750`-`2768`. The escape changed which perturbed runs fall back to an unchecked plan, which
grinds samples 800-960; with that plan verified the row is back at `154`
([ai.md](../gameplay/ai.md#the-fallback-plan-is-verified-2026-10-21)).

Two negatives:

- **The hold reads the position from before the escape.** `Ship_UpdateMagLock` (`0x0884ba0c`)
  takes its heights from `craft+0x1a0` (`0x0884bb6c`, again for the from-ray height at
  `0x0884c090`), the copy of `body+0x30` made at the top of `Ship_CastHoverProbes`
  (`0x08849f8c`-`0x08849fa4`), not the body the escape's `Body_Translate` (`0x0884d9f8`) has just
  moved. So within one update the escape and the hold add, as ours does (the hold reads the
  untranslated body, the escape lands after `drain`). One live `body+0x30` load at
  `0x0884bc14` feeds a point (`pos - n1 * d1`) whose use was not traced. Confidence **85** on the
  order, static.
- **A mid-wall teleport does not reproduce the crest.** Writing the body at our pose on the
  wall (`y = 250`, up `+x`, `124` units/s) at a `Ship_UpdateCraft` breakpoint left the hold not
  taking over: up took about 70 updates to converge on the wall and the craft stalled to rest
  within a second. The cause was not read (the located sections `+0xaf0`/`+0xb60` after the
  write are the first thing to look at); a lap from the grid is what measured the crest.

Cross-title, for the crest escape: **Pure, Omega and 2048: checked, applies, not measured
live.** All three fly the two-point default rig (`hover_rig: None`, `MagFloorEscape::Always`),
so the same derived-hit dip escapes on any crest like this one; no capture of theirs was taken.
**HD: checked, differs.** Its gate is `BelowFullLock` (`crates/hd/src/race.rs`, the mesh-tag
read on [hover-four-point.md](../ghidra/functions/ps3-hdfury-eu/hover-four-point.md#hit0x28-is-the-marchs-mesh-shape-tag-2026-10-21-deviations-to-original)),
so at full lock a magstrip probe never escapes there.

**The drift it was held for was the harness.** `pad_crossing_ground_truth` replayed with no track
locator (`replay(..., None)`), so our magstrip hold never engaged: over the 1284 crossing our
craft sank `1.5` units below the recording in 17 ticks (body height `4.18 -> 2.44` above the
strip, against the recording's `4.18 -> 3.93`) and reached the escape. With the locator the
test now passes (`lap_window_ground_truth`'s resampled spline), the craft holds, and the gain
reads the same with the escape held out or on:

| 1284 crossing | before | locator, held out | locator, on |
| --- | ---: | ---: | ---: |
| single-seeded (recording `+46.24`) | `+48.94` (`+53.26` with it on) | `+46.82` | `+46.82` |
| reseeded every 5 ticks | `+46.66` | `+46.67` | `+46.67` |

| title | escape off any hit | escape without contact | origin |
| --- | --- | --- | --- |
| Pulse (PSP, PS2) | yes (`Floor`, `Wall`, `MagFloor`) | yes | measured; `MagFloor` read and seen tagged `1` live, held out until 2026-10-21 |
| Pure | yes | yes | inherited from Pulse (Pure's `Ship_HoverTwoPoint` lineage; its gate not read) |
| Wipeout HD | yes (`Floor`, `Wall`; `MagFloor` below full magnet lock) | yes | without contact: read statically (`Craft_HoverFourPoint` translates at `0x000ee6ac` before its contact test, [hover-four-point.md](../ghidra/functions/ps3-hdfury-eu/hover-four-point.md#the-escapes-sum-2026-10-20-hd-hover-residual)); the gate `hit+0x28 == 1` is the march's mesh-shape tag, read and seen constant live, `Floor` only before 2026-10-21 ([hover-four-point.md](../ghidra/functions/ps3-hdfury-eu/hover-four-point.md#hit0x28-is-the-marchs-mesh-shape-tag-2026-10-21-deviations-to-original)) |
| Omega, 2048 | yes | yes | **changed by this port** (they had Pulse's floor-only gate too), inherited from Pulse: both fly Pulse's two-point rig (`hover_rig: None`) until their own loops are read ([hover-four-point.md](../ghidra/functions/ps3-hdfury-eu/hover-four-point.md#the-escapes-sum-2026-10-20-hd-hover-residual)); not checkable here, no executable read of either's hover loop and no Omega capture path |

## Wipeout HD: the class scale is on flight gravity (Ubermall)

The question: our autopilot fell about `20` units/s² short of the original in
Ubermall's jump. Measured on RPCS3 `0.0.42`, HD EU (`BCES00664`), Racebox Single
Race, PHANTOM, weapons off, Ubermall forward, Feisar, the original's autopilot
armed after GO. **Superseded** as evidence by the player-input captures
[below](#wipeout-hd-under-player-input-2026-10-20-hd-air-player-input): the Autopilot's rail
writes the craft's velocity, so the pose walk now refuses these rows unless
`OAG_HD_AUTOPILOT=1` asks for them. `scripts/rpcs3-hd-autopilot.py` logs every new body position with
the basis rows `body+0x1d0..+0x1f0`, the rate `body+0x1a0` (negated body-local, as on
Pulse: a median `0.006` error against the basis' own motion, against `0.7` or worse
for the other three readings), the craft's up `craft+0x200`, the located sample's
down `ship+0x7850` and the team block's `+0x9c`. One boot, 15,499 rows, two
Ubermall jumps (two laps).
`crates/raceplay/tests/hd_air_pose_walk.rs` (ignored, `OAG_SWEEP`) loads the race
for HD's own craft laws, handling and collision, seeds a craft from each row and
steps it once; `dt` is the position change along the velocity.

**Static.** `Craft_Update` (`0x000f1958`) builds gravity as

```text
force.y = -( normal_gravity * m * grounded
           + flight_gravity * m * scale[class] * (1 - grounded) )
```

with `m` the body's `+0x4a4`, `grounded` `craft+0x304` and `scale` a per-speed-class
table indexed by the class. Pulse puts its class scale on the **grounded** term
(`oag_physics::forces::Environment::class_gravity_scale`); HD puts it on the airborne one.

**Live.** With the scale on the grounded term (Pulse's law), every airborne row
falls short by `0.380` units/s a tick (median, 117 and 115 rows on the two jumps),
`22.9` units/s², almost all of it along the craft's up. Feisar's `flight_gravity`
is `115` and PHANTOM's scale `1.2`, so moving the scale to flight gravity predicts
`115 * 0.2 = 23.0`; with it there, the median residual is `0.000` on both jumps.
**Confidence 85** (static read, and the residual closes on two jumps of one boot);
**92** since the player-input re-measure below (two classes whose scales sit either side of
`1.0`).
Ported as `oag_title::craft_laws::CraftLaws::class_gravity_on_flight` and
`oag_physics::passive::ClassGravityTerm`: HD measured; Omega and 2048 inherit HD's;
Pulse and the PS2 keep the grounded term (measured); Pure inherits Pulse's,
unmeasured. 2048/Omega: **not checkable** here (no executable read of either's
gravity site, no Omega capture path).

Sweep (`opponent_autopilot_sweep`, 4 seeds) against the baseline: HD rescues unchanged
(novice 0/0, Ace 1/0) and its opponent stints' wall ticks `1016 -> 912` and `528 -> 484`;
2048's wall ticks down (`169 -> 122` on Ace, stints `33 -> 16`, `17 -> 0`), no rescues;
Pulse and Pure identical. **Omega: inherited, unmeasured; small respawn rise under our
driver**: Ace rescues `0 -> 1` with the switch off and `1 -> 2` on, and one stint rescue
(`0 -> 1`), over 4 races.

On the ground the two readings differ by `0.02` units/s a tick and cannot be told
apart here. The grounded rows carry a separate residual of about `0.95` units/s a
tick along the craft's up, the same under either law: open, not this law.

**The in-air `weight_distribution` torque on HD, live.** The team block reads
`-4.0` (Feisar). With the bias off, the airborne pitch residual is `+0.205` and
`+0.176` rad/s² (median, the two jumps) against `4 / 17.33 = 0.231` predicted; on, it
is `-0.026`. **Confidence 85**; on, as the table above says. The pitch gate never closed over this jump (the product stayed at or
above `0.926`), so HD's gate was still read, not seen. Both are now measured under player
input, below.

## Wipeout HD under player input (2026-10-20, hd-air-player-input)

The three HD air laws above were first measured on one run the original's Autopilot flew,
and the Autopilot is not a physics reference: its rail (`Ai_SnapToLane`, `0x000ff9b0`) writes
the craft's velocity and position. All three are re-measured here with player input alone,
and all three stand. The pitch gate is seen closing live for the first time on HD.

**Method.** RPCS3 `0.0.42`, HD EU (`BCES00664`), Racebox **Time Trial** (no opponents, no
Autopilot). `scripts/rpcs3-hd-air-hold.py` holds cross (and the stick, when asked) for
`2.5` s, pauses, writes the craft's pose and speed with `rpcs3_place.write_pose`, resumes
with the input still held and logs every new body position for a few seconds. A row reads
the control record through `craft+0x84` (thrust `100`, pitch `+/-99.9` with the stick held,
`0` centred) and the pickup slot's flags: **the Autopilot's running bit `0x800` is clear on
every row** (3,875 rows). The placements:

| Captures (`data/reference/hd-air-player-input/`) | Class, team | Placed | Walk env |
| --- | --- | --- | --- |
| `ubermall-phantom-feisar-centred-{1,2,3}.csv` | PHANTOM (scale `1.2`), Feisar (`-4`) | Ubermall's long jump, 0.5 s before the lip, at the old capture's pose and 219 u/s | `OAG_HD_CLASS=PHANTOM OAG_HD_TEAM=Feisar` |
| `ubermall-phantom-feisar-stick-back-{1,2}.csv` | the same, stick held back (`+99.9`) | the same | the same |
| `ubermall-phantom-feisar-stick-forward-{1,2}.csv` | the same, stick held forward (`-99.9`) | the same | the same |
| `ubermall-venom-harimau-centred-{1,2}.csv` | Venom (scale `0.85`), classic Harimau (`-4`) | the same | `OAG_HD_CLASS=VENOM OAG_HD_TEAM=Harimau` |
| `chenghou-venom-harimau-jump-{a-1,a-2,b-1}.csv` | Venom, classic Harimau | Chenghou Project's two jumps, the poses below | the same plus `OAG_HD_TRACK=/data/environments/04_chenghou_project/track.vex` |
| `talons-venom-assegai-drop-{1,2,3}.csv` | Venom, classic Assegai (`+4`) | Talon's start straight, lifted 30 units, 60 u/s | `OAG_HD_CLASS=VENOM OAG_HD_TEAM=Assegai OAG_HD_TRACK=/data/environments/talons_junction/track.vex` |

The PHANTOM state is `data/saves/hd-fury/hd-timetrial-ubermall-feisar-phantom-start`
(`emu-hd-timetrial-state.sh <lane> 6 0 0 <name> 3`); the others are the existing
`hd-timetrial-*-start` states. `crates/raceplay/tests/hd_air_pose_walk.rs` steps ours once
from every row (one call per capture, with that capture's walk env from the table):

```text
OAG_SWEEP=1 OAG_HD_AIR=data/reference/hd-air-player-input/<capture>.csv OAG_HD_ROWS=0..400 \
OAG_HD_MODE=tt <walk env> [OAG_HD_GRAV=ground] [OAG_HD_BIAS=0] [OAG_HD_DIAG=1 [OAG_HD_SUM=0]] \
    cargo nextest run -p oag-raceplay hd_air_pose_walk --run-ignored all --no-capture
```

Each figure below is the median over airborne rows (the third airborne row of a stretch
on), per capture, of ours minus the original after one step.

**Class gravity is on flight gravity.**

| Capture | `vy` residual, flight term (ours) | `vy` residual, grounded term | predicted, grounded term |
| --- | ---: | ---: | ---: |
| PHANTOM, 7 captures | `-0.02` to `+0.02` | `+0.41` to `+0.51` | `+115 * 0.2 / 60 = +0.38` |
| Venom, 2 captures | `0.000` | `-0.270` | `-110 * 0.15 / 60 = -0.275` |

Two scales either side of `1.0` move the grounded-term residual in opposite directions, and
the flight term closes both. **Confidence 92** (static read plus a runtime trace on two
classes; one binary).

**The in-air bias is the team's own `+0x9c`.** Stick centred, so nothing else torques pitch:

| Captures | `+0x9c` live | pitch residual, bias on | bias off | predicted off |
| --- | ---: | ---: | ---: | ---: |
| PHANTOM Feisar, 7 | `-4.0` | `-0.007` to `-0.002` | `+0.224` to `+0.229` | `+4 / 17.33 = +0.231` |
| Venom Harimau, 2 | `-4.0` | `-0.002` | `+0.228`, `+0.229` | `+0.231` |
| Venom Assegai, 3 drops | `+4.0` | `0.000` to `+0.009` | `-0.231` to `-0.221` | `-0.231` |

Assegai's opposite sign separates a per-team field from a constant `-4`. HD's teams author
three values (our loader's `<Misc weight_distribution>`): `-4` AG Systems, Feisar, Harimau,
Piranha; `+4` Assegai, Auricom, Qirex, Triakis; `0` EG-X, Goteki, Icaras, Mirage.
**Confidence 92.**

**The pitch gate closes live, at 0.9, and keeps the bias.** With the stick held back, the
product `dot(up, -down)` (the craft's up against the located sample's down, `ship+0x7850`)
falls below `0.9` over the long jump. On those rows the original drops the whole input:
ours stepped with the gate open leaves `+1.148` rad/s² of pitch the original does not have,
on every row from the crossing until the product climbs back. Closed (the walk applies it
off the row's own down, since it carries no track sample of its own; the race fills
`Environment::track_sample` every tick, airborne too), the residual is `-0.003`, and turning
the bias off still moves it by `+0.227`: the bias stays on with the gate closed, as on Pulse.
The crossings: open at `0.9016`, `0.9072`, `0.9075`, `0.9152`; closed at `0.8988`, `0.8982`
and below (one row at `0.9015` reads closed, its hull up `craft+0x200` not yet updated for
the frame). Held forward, the product stays above `0.9` and the residual is `-0.004` to
`-0.007`. **Confidence 90.**

**HD sums its hover escapes.** `OAG_HD_DIAG=1` with and without `OAG_HD_SUM=0`, rows where
summing adds more than `0.005`: 131 escape rows (111 PHANTOM, 20 Venom). With the largest
escape alone, the position residual along up fits `-0.955 * (sum - max) + 0.034`
(correlation `-0.874`; the Autopilot run gave `-0.954`, `-0.996`). Summed, the median is
`+0.025` (p5 `+0.006`, p95 `+0.093`, mean abs `0.042` against `0.227`). The `+0.02` bias on
escape rows the Autopilot run left is here too, unattributed. **Confidence 90.**

**The rail, and why the old capture is set aside rather than undone.** The pose walk adds
the residual along the craft's left. On the old Autopilot rows (Ubermall PHANTOM, rows
1000-1600) the airborne median is `0.022` u/s in velocity and `0.0075` in position per
frame; on the player-input rows it is `0.000` and `0.0007`-`0.0010`. That residual is the
evidence: the rail shows on the Autopilot rows and is absent under player input. The static
reading agrees without proving it: the rail's weight is `driver+0x52 ? a * b : 0`
([ai-controls.md](../ghidra/functions/ps3-hdfury-eu/ai-controls.md#the-rail)), and `+0x52` is
the "controls consumed" byte (Pulse's `driver+0x21`), whose writer was not read here. The walk now
refuses Autopilot rows (`slot+0x200 & 0x800`) and takes the engine bits off each row's own
`craft+0x264`: the old walk stepped every row with uncapped thrust and the `x1.2` speedup,
which are the Autopilot's alone.

**Chenghou Project's takeoff was the harness.** The hd-chenghou-jumps lane recorded ours
about 2 units low off Chenghou's first jump (`pz -100`: vy `8.8` against `13.0`). The
sweep placed ours after 300 idle ticks, while HD's hover cap was still on its release ramp
(`3.52`): the placed craft found no floor on the ramp and flew from the first tick, where
the original's probes hold it up the ramp for six frames. Placed after 1500 ticks (cap
`23.5`, released), ours sits inside the original's own spread (two repetitions of jump A,
one of B; cells `py / vy`):

| Jump | pz | Original | Ours, placed at 300 | Ours, placed at 1500 |
| --- | ---: | --- | --- | --- |
| A | -100 | `55.56 / 15.05`, `55.48 / 13.86` | `54.61 / 8.43` | `55.32 / 14.18` |
| A | -125 | `56.89 / -3.21`, `56.57 / -4.56` | `54.59 / -8.88` | `56.38 / -4.05` |
| A | -150 | `54.52 / -20.40`, `53.85 / -21.90` | `51.00 / -25.16` | `53.76 / -21.19` |
| A | -170 | `49.97 / -33.41`, `48.94 / -35.03` | `45.59 / -37.56` | `49.02 / -34.16` |
| B | -260 | `59.81 / 7.13` | `58.79 / 3.01` | `59.64 / 7.77` |
| B | -300 | `56.62 / -24.19` | `54.13 / -27.08` | `56.65 / -23.23` |
| B | -340 | `41.42 / -52.21` | `38.15 / -54.06` | `42.20 / -50.88` |

`hd_chenghou_jumps_sweep.rs` now idles 1500 ticks by default (`OAG_IDLE`). One step at a
time over the original's own rows, ours matches it in the air on Chenghou too (`vy` residual
`0.000`, pitch `-0.002`); at the lip, the two frames with only the rear probes in contact
leave ours `+0.66` and `+0.87` u/s higher, small and the other way.

**Open: a forward residual, not an air law.** Ours ends each step faster along the craft's
forward than the original: in the air by `+0.12` to `+0.16` u/s per frame on PHANTOM (about
`8` u/s²) and `+0.04` on Venom, on the ground by `+0.15` to `+0.44`. Unattributed. The first
suspect is the walk's own seeding: it copies the row's throttle `craft+0x30c` into
`ShipState::thrust`, which may not be the same quantity. After that, HD's engine doubling
and ground drag, which are inherited from Pulse and unread on HD.

**Cross-title.** Omega already flies HD's bias, gate and gravity term, inherited (**checked,
applies, wired**: the same `CraftLaws` values; not re-measured, no Omega capture path). The
escape sum does not reach Omega or 2048: both fly Pulse's two-point rig until their own hover
loops are read ([hover-four-point.md](../ghidra/functions/ps3-hdfury-eu/hover-four-point.md#the-escapes-sum-2026-10-20-hd-hover-residual)).
2048's gate and gravity term were already HD's; its bias was held off until this re-measure
and is now on, inherited from HD via Omega (**checked, applies, wired**; 2048's own pitch update
is not read, so not re-measured on 2048).
