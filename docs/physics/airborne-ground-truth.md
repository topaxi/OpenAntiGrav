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

**Measured, not ported (held, maintainer 2026-10-19).** Ported as a per-title craft law
(Pulse, Pure and PS2 world axes; HD rotated, Omega and 2048 inheriting HD's), the lone
craft's regression gate stayed twelve of twelve clean, but the player's whole-race
autopilot in `opponent_autopilot_sweep` (4 seeds, 48 races per Pulse difficulty) gained
rescues: novice `62 -> 106` with the switch off and `68 -> 91` on, mostly `05_Track` and
`06_Track`, and new ones on Ace's `04_Track`, `07_Track` and `10_Track` (Ace totals
`29 -> 25` and `34 -> 32`). HD, Omega and 2048 were unchanged bit for bit. The law waits
until this engine's own driver and pace plan fly the world-axis craft as well as the
body-axis one; the measured law is not to be tuned to get there.

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
(`0.9009`, `0.8976`). **Confidence 90.** Ported as `oag_physics::engine::pitch_aligned`,
reading the located sample `Environment::track_sample` carries (the entity's `+0xaf0`,
whose `down` is `+0xb10`); with no sample the gate is open, chosen. Every title runs it:
Wipeout HD's `Craft_UpdatePitch` (`0x000f0980`) has the same gate, read statically, not
measured live ([physics.md](../ghidra/functions/ps3-hdfury-eu/physics.md)); Pure and the PS2
inherit Pulse's, Omega and 2048 inherit HD's. 2048/Omega: **not checkable** here, no executable read of
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
frame, and the residual).
