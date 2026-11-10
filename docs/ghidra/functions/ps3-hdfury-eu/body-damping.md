# HD's rigid body damps its own velocity (2026-11-10, hd-late-speed)

Why this engine's craft ran about 0.7% faster than Wipeout HD's from 1 s of held throttle on:
HD's integrator takes `0.01` per second off the body's linear and angular velocity after the
forces, and ours did not. It is Pulse's law at HD's offsets; Pulse's own page had recorded it as
read and not implemented ([rigid-body.md](../psp-pulse-usa/rigid-body.md#body_integrate-0x0884e230)).

Read in Ghidra (`/hdfury/EBOOT-ps3-hdfury-eu.elf`, TOC `r2 = 0x008ad4d8`) from the bridge's
disassembly, past the decompiler's AltiVec truncation. Measured on RPCS3 with
[`scripts/rpcs3-trace.py`](../../../../scripts/rpcs3-trace.py): one boot, four repeats of
`verification/scenarios/hd-thrust.inputs` from grid slot 0 of Talon's Junction, Racebox Time
Trial, Venom, Feisar (`<Engine amount="412" accelcap="18.5">`, read back from the craft's class
block), Pilot Assist off, throttle `100` on every thrust row. The captures are not kept in the
repository (guest memory is game content). The measured comparison is on
[hd-handling-ground-truth.md](../../../physics/hd-handling-ground-truth.md).

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x000f5878` | `Body_Init` | 82 | zeroes the four accumulators `+0x150`..`+0x180`, writes `1.0` to `+0x4a8` and `+0x4a4` (inverse mass, mass), the identity blocks, and `0.01` to `+0x4bc` and `+0x4b8`; two callers (`0x000f5c98`, `0x000f5e18`) and an OPD slot; Pulse's `Body_Init` in shape |
| `0x008a92f4` (data) | `Body_DefaultDamping` | 90 | `3c23d70a`, `0.01f`; `lfs f0,-0x41e4(r2)` at `0x000f5a5c`, stored twice at `0x000f5a94`/`0x000f5a98` |

`Body_Integrate` (`0x000f6870`) is named on [wall-response.md](wall-response.md).

## The law, in `Body_Integrate`

`f1` is the step, `r19` the body, `r6` its velocity (`+0x190`), `r4` its angular velocity
(`+0x1a0`), `v8` the step splatted.

    position += velocity * dt                                  # 0x000f6914..0x000f6950, under body+0x4d1
    velocity += worldForce(+0x150) * invMass(+0x4a8) * dt      # 0x000f6ee0..0x000f6f28
    velocity += (basis * localForce(+0x160)) * invMass * dt    # 0x000f6f2c..0x000f6fdc
    velocity -= velocity * dt * body+0x4bc                     # 0x000f6fd8..0x000f6ffc
    body+0x1b0 = velocity - old velocity                       # 0x000f7018..0x000f7020
    body+0x4c4 = |velocity|                                    # 0x000f7024..0x000f706c
    angularVel += torque terms * dt                            # 0x000f7090..0x000f70dc
    angularVel -= angularVel * dt * body+0x4b8                 # 0x000f70f8..0x000f711c

The two damping lines are each one `vmaddfp` of the just-updated velocity by the splatted step,
a second by the splatted coefficient (`lfs f0,0x4bc(r19)` at `0x000f6fe0`, `lfs f0,0x4b8(r19)`
at `0x000f7100`), and a `vsubfp`. That is Pulse's
`velocity -= velocity * h * body+0x384` and `angularVel -= angularVel * h * body+0x380`
(`0x0884e368`, `0x0884e374`) term for term, including the order: after both force accumulators,
on the updated velocity.

**Who writes the coefficients.** A census of `stfs` with displacement `0x4bc` and `0x4b8` over
the image finds one store of each on a body, both in `Body_Init`, both of the same register
loaded from `0x008a92f4` (`0.01`). The other four hits are stack slots (`r1`) in two unrelated
functions. Pulse's `Body_Init` zeroes the pair and its ship constructor writes `0.01`; on HD
the initialiser itself writes it, so every body carries it.

**`body+0x4c4` is the body's speed**, `|velocity|` after the damping, stored every step
(Pulse's `body+0x398`). That settles the question [hull-shake.md](hull-shake.md) left open
(speed, not thrust); the "engine thrust" reading on [physics.md](physics.md) was already
corrected there for the craft's `+0x340`.

## Measured live (confidence 92 linear, 85 angular)

One step of this engine's physics from every frame of a held-throttle run, with the frame's own
step, against the next frame (`crates/raceplay/tests/hd_air_pose_walk.rs`, `OAG_HD_FWD=1`,
`OAG_HD_MODE=tt OAG_HD_CLASS=VENOM OAG_HD_TRACK=/data/environments/talons_junction/track.vex`).
The forward residual is ours minus HD's, in units/s per frame, 203 grounded rows of one run
(a second run reads the same to the last printed digit of the medians):

| Body damping stepped with | median residual | residual / (speed x dt), median (min..max) |
| --- | ---: | --- |
| `0` (`OAG_HD_DAMP=0`, what this engine flew) | `+0.0158` | `+0.01002` (`0.00965`..`0.01054`) |
| `0.01` | `+0.00005` (max `0.00018`) | `+0.00003` |
| `0.02` (`OAG_HD_DAMP=0.02`) | `-0.0157` | `-0.00995` |

By speed, without the term: `+0.0028` at 5-30 units/s, `+0.0075` at 30-60, `+0.0129` at
60-90, `+0.0170` at 90-110, `+0.0197` at 110-125, and the ratio to `speed x dt` is
`0.0100` in every band. So the missing term is linear in the speed with coefficient `0.01`
per second, which is the instruction above and nothing else in the craft update (drag is
quadratic, thrust constant or speed-capped).

**Per second, not per frame.** RPCS3 delivered four frames of odd length in the first run
(`0.0236`, `0.0098`, `0.0219`, `0.0115` s against the usual `0.0167`). Their residual over
`speed x dt` reads `0.0101`, `0.0102`, `0.0100`, `0.0100`; over the speed alone it runs from
`1.0e-4` to `2.4e-4`. The term scales with the step.

**The angular term is read, not isolated.** At a full-lock yaw rate of `1.3` rad/s it takes
`0.013` rad/s^2, `0.2 %` of the craft's own `-5 L` yaw damping and under the walk's yaw
residual on these rows (`0.02`..`0.3` rad/s^2). It rests on the instruction pair and on the
shared store of the same constant.

## What it does to the speed curve

Open loop from rest, the same script through `oag-game --race --mode time_trial --team feisar
--class venom --trace-out` (thrust held from well after the start-boost window), forward speed
at game time since the first thrust frame, ours against HD (four runs agree to `0.01`):

| t (s) | HD | ours before | ours after |
| ---: | ---: | ---: | ---: |
| 1.0 | `53.82` | `53.96` | `53.74` |
| 2.0 | `100.48` | `101.09` | `100.44` |
| 2.5 | `111.68` | `112.45` | `111.66` |
| 3.0 | `118.04` | `118.89` | `118.02` |
| 3.5 | `121.52` | (wall) | `121.52` |

Over every HD frame from 2.0 to 3.0 s, ours minus HD ran `+0.61`..`+0.86` before and
`-0.04`..`-0.01` after (61 frames); from 3.0 to 3.5 s, `-0.013`..`-0.005`. The remaining offset
is largest in the first second (`-0.09`) and decays, the shape of a fraction of a frame
between the two runs' first thrust step.

## The other titles

- **Pulse: checked, applies, not wired.** The same two lines are in Pulse's `Body_Integrate` on
  PSP and PS2 with the same `0.01` ([rigid-body.md](../psp-pulse-usa/rigid-body.md)). Replaying
  `talons-junction-standing-start.csv` (PSP, Venom, Assegai) through this engine with the term
  switched on, the forward speed error over the clean launch decays from `-1.15` at tick 60 to
  `-0.19` at tick 185, where without it it drifts from `-0.91` to `+0.73`; the replay's largest
  speed error over the capture falls from `3.88` to `1.25` units/s. So Pulse has it live as well
  as on paper (confidence 88). It is not flown there yet: with it on, the committed state hashes
  move and eight disc-backed tests that ride a Pulse trajectory change verdict (the AI's clean-lap
  gate on one circuit at Phantom, its speed plan, the original Autopilot's rescue count on one
  circuit, the plasma's tick timing among them), each of which has to be re-measured rather
  than re-pinned. `oag_physics::laws::Laws::PULSE` carries `BodyDamping::NONE` until then.
- **Pure: checked, applies, not wired.** Its constructor passes `0.01`, `0.01` and its
  `Body_Integrate` (`0x089345e8`) is Pulse's loop instruction for instruction
  ([pure-physics.md](../../../physics/pure-physics.md)); read, not measured live. It follows
  Pulse: off until Pulse's is flown.
- **Omega: checked, applies, wired.** Read off `eboot.bin` (v1.07, `/omega/eboot-ps4-omega-eu-v107.bin`):
  the integrator inside `FUN_013463f0` (not renamed here) multiplies the updated velocity by the
  splatted step and by `body+0x4b0` and subtracts it (`0x013467d9`..`0x013467eb`), stores the
  velocity change at `+0x200` and the speed at `+0x4b8`, then reads `body+0x4ac` for the angular
  twin (`0x013468cc`). HD's `+0x4bc`/`+0x4b8`/`+0x4c4` sit `0xc` lower on the PS4. Three body
  constructors store a packed `0.01, 0.01` (`mov rcx, 0x3c23d70a3c23d70a`) at `+0x4ac`
  (`0x012e529b`, `0x01331a26`, `0x0133680e`) on a `0x4f0`-byte allocation. Confidence 80; no live
  value (shadPS4 gives no memory readout). Omega's law data carries `0.01` as its own.
- **2048: not read; inherits HD's.** Its `eboot.elf` was not opened in this lane. It flies HD's
  craft laws by lineage, so it takes the `0.01`; the read is one function (the integrator its
  physics step calls per body) and is open.
