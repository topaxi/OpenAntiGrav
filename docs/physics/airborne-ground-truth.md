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
identity at `100.00 %` plus this dynamic leg on three captures). Wipeout HD
rotates its tensor (`body+0x50` read live as `R I R^T`,
[craft-inertia.md](../ghidra/functions/ps3-hdfury-eu/craft-inertia.md)), so the frame is a
per-title law.

**On (2026-10-20), a per-title craft law** (`oag_physics::InertiaFrame`,
`oag_title::craft_laws::CraftLaws::inertia_rotates`): Pulse and the PS2 world axes, measured;
HD rotated, measured; Omega and 2048 inherit HD's; Pure rotated, chosen, unchanged until a
`pure-physics` lane measures it (with Pulse's frame Pure's opponent stint walls rose
`69 -> 541`). It was ported and held twice (`9a2c5b017`, held by `c6116853b`) because this
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
(`0.9009`, `0.8976`). **Confidence 90. On (2026-10-20)**, after the tensor and the bias: on its own it cost
the original's autopilot driver two rescues on Pulse's `05_Track`
(`autopilot_original_ground_truth::the_original_autopilot_finishes_every_pulse_circuit_without_a_rescue`,
tick 7282), which passes with all three on; that test also fails under 2 of the 9
perturbations [above](#our-ai-against-the-air-laws) with every law off. The mechanism is
`oag_physics::engine::pitch_aligned`,
reading the located sample `Environment::track_sample` carries (the entity's `+0xaf0`,
whose `down` is `+0xb10`); with no sample the gate is open, chosen. It is a per-title craft
law (`oag_title::craft_laws::CraftLaws::pitch_gate`, `ShipState::pitch_gate`): Pulse runs it
(measured) and the PS2 with it; Wipeout HD's `Craft_UpdatePitch` (`0x000f0980`) has the same
gate, read statically, not seen closing live
([physics.md](../ghidra/functions/ps3-hdfury-eu/physics.md)); Omega and 2048 inherit HD's;
**Pure has none**: its `Ship_UpdatePitch` (`0x0892caf8`) never calls
`Ship_TestPitchAlignment` (measured, [pure-physics.md](pure-physics.md#pitch-is-three-code-literals)). 2048/Omega: **not checkable** here, no executable read of
either's pitch update and no capture path for Omega. The lone craft's regression gate and
every committed reference were unchanged by it: no scripted or lone-craft run pitches
past the cone.

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
| Wipeout HD | on | measured: read live as `-4.0` (Feisar), confidence 85, the airborne pitch residual closing to `-0.026` with it (below) |
| Omega | on | inherited from HD |
| 2048 | off | inherited from HD, **held** (chosen): the original AI field respawns once on park at `eps = 0` (1 of 9 perturbations, 0 of 9 with the bias off); measure on 2048 to release |
| Pure | off | **measured**: Pure's `Ship_UpdatePitch` has no `weight_distribution` term, and Pure authors none ([pure-physics.md](pure-physics.md#pitch-is-three-code-literals)) |

2048/Omega: **not checkable** here (no executable read of their pitch update, no Omega
capture path).

On Pulse, with it on, the autopilot survived far better: `opponent_autopilot_sweep`'s whole-race player autopilot (4 seeds, 48 races per
difficulty) went from 62 to 24 rescues on Novice with the switch off and 68 to 16 with it
on, and from 29 to 24 and 34 to 24 on Ace; the opponents' paired-fork stints went from
5,317 to 4,570 wall ticks.

## Wipeout HD: the class scale is on flight gravity (Ubermall)

The question: our autopilot fell about `20` units/s² short of the original in
Ubermall's jump. Measured on RPCS3 `0.0.42`, HD EU (`BCES00664`), Racebox Single
Race, PHANTOM, weapons off, Ubermall forward, Feisar, the original's autopilot
armed after GO. `scripts/rpcs3-hd-autopilot.py` logs every new body position with
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
**Confidence 85** (static read, and the residual closes on two jumps of one boot).
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
above `0.926`), so HD's gate is still read, not seen.
