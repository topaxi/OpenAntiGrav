# Verification protocol

> **Status: design. Not yet implemented.** The harness is milestone M3. This
> document specifies what it must do, so that subsystems built before it are
> built to be verifiable.

## Why this comes before the physics

Building ship handling and then trying to verify it means building it twice:
once by feel, then again properly once the measurements disagree. The harness is
the instrument; the instrument comes first.

## The loop

```
original executable
        |
   save state (fixed starting point)
        |
   scripted input (deterministic, per tick)
        |
        v
   recorded trace  <----- compare -----> our trace
                            |
                            v
                 first divergent tick, and by how much
```

The output that matters is not pass or fail. It is **the first tick at which the
two diverge, and the magnitude**. A subsystem that tracks the original for 400
ticks and then drifts has a different bug from one that is wrong at tick 1.

## Components

| Piece | Purpose |
| --- | --- |
| Fixed start | A fixed, reproducible starting point. **Built, not as a save state**: a PPSSPP `.ppst` pins position no tighter than the plain menu walk (`0.031` units / `1.41` degrees between two loads, measured 2026-07-30) and is not used. `--start-heading` pins to `0.0022` units / `0.0001` degrees instead - see [the debugger page](ppsspp-debugger.md#the-start-pose-is-pinnable-and-the-craft-was-never-settling) - and is what `just scripted-emu` uses. |
| Input script | Per-tick controller state. Plain text, committed. **Built**: `verification/scenarios/*.inputs`, read by `scripts/input_script.py` on the capture side and `oag_trace::script` on ours. |
| Trace capture | Emulator-side recording of the observed values, per tick. **Built**: `scripts/psp-trace.py`, on top of [PPSSPP's websocket debugger](ppsspp-debugger.md). |
| `oag-trace` | Runs our engine on the same input and compares. **Built**: [`docs/tools/oag-trace.md`](../tools/oag-trace.md). |

Traces are derived game data and are never committed. They go under
`data/traces/`, which `.gitignore` covers, and are regenerated from a disc image
by the recipe on the debugger page.

**Input scripts are the opposite and are committed.** Nothing in one comes off a
disc: it is a list of button names somebody chose, in a format
[`oag-trace`'s page describes](../tools/oag-trace.md#input-scripts). That is what
makes the loop above closed rather than circular - without a script the only
inputs available are "one constant button" and "whatever the recording happened
to hold", and the second compares a run against itself.

## What gets traced

Per tick, per ship:

- Position, orientation, linear and angular velocity
  (angular velocity from `body+0x160`, in the original's own sign and frame -
  both open, see [`oag-trace`'s page](../tools/oag-trace.md))
- Speed, thrust, airbrake state
- The two engine gates, `craft+0x290` (collision stun) and `craft+0x2e0`, which
  between them decide whether the original produced any thrust at all that frame
- Shield energy, held weapon, weapon timers
- Current track section, lap and split times
- Race position

Plus global state: tick counter, PRNG state, race phase.

## Tolerances

We do **not** compare bit patterns against the original. The PSP's VFPU is not
IEEE-conformant, so exact agreement is not achievable in `f32`. See
[ADR-0002](../architecture/adr/0002-determinism-model.md).

Provisional tolerances, to be revised once real measurements exist:

| Quantity | Tolerance | Rationale |
| --- | --- | --- |
| Position | 0.01 units absolute, or 1e-4 relative | Below perceptibility at track scale |
| Orientation | 1e-4 radians | Below one pixel of visible rotation |
| Velocity | 1e-3 relative | Integrates into position, so tighter than position |
| Angular velocity | 1e-4 rad/s absolute **and** 1e-3 relative | Integrates into orientation. The absolute floor is not optional here the way it is for linear velocity: a straight-line capture records *exactly* zero, and a purely relative test would call every straight capture divergent on tick 0. At 60 Hz the floor is a sixtieth of the orientation tolerance per tick, so it cannot mask one. |
| Float timers | 1e-3 s absolute | About a sixteenth of a frame. The row below is about *integers*; `craft+0x290` and `craft+0x2e0` are floats decremented by a variable `dt`, so exact agreement is not achievable. |
| Timers, counters | **exact** | Integers. Any difference is a bug. |
| PRNG state | **exact** | Integers. Any difference means desync. |
| Discrete state | **exact** | Race phase, weapon held, lap number. |

**Integer state is compared exactly, always.** There is no reason for an integer
to be approximately right, and allowing slack there would hide real bugs.

Divergence must also be **bounded over time**, not merely small at each tick. A
position error that grows every tick is a systematic error in the physics, even
while it is still within tolerance. The harness reports the trend, not only the
maximum.

## Cross-platform comparison

Running the same scenario on PSP and PS2 is a third source of evidence. Where
the two originals disagree with each other, our tolerance against either is
necessarily at least as wide as their disagreement, and that fact should be
recorded rather than papered over.

## Determinism first

Before any comparison against the original is meaningful, our own simulation
must be reproducible. That is guaranteed by the
[determinism gate](../architecture/determinism.md), which shipped in M0 for
exactly this reason: a harness measuring an unstable simulation measures noise.

## Scenarios

The suite should cover, at minimum:

1. **Straight line.** Full thrust, no steering. Isolates acceleration and drag.
2. **Constant turn.** Isolates steering response.
3. **Airbrake turn.** Isolates the airbrake model.
4. **Wall scrape.** Isolates collision response.
5. **Jump and landing.** Isolates pitch and the air cushion.
6. **Full lap, no weapons.** Integration test for handling.
7. **Full race, eight ships.** Integration test for AI, weapons and race rules.

Each is a save state plus an input script. Early scenarios isolate one system so
a failure is attributable; later ones catch interactions.

## Reading a capture: rules that each cost a session

These lived on `HANDOVER.md` until 2026-08-09. They are about the *instrument*,
not about any one comparison, and every one of them was learned by publishing a
wrong number first.

**Read the divergence table from the top: check the control columns agree before
reading anything else.** `throttle`, `brake`, `steer` and the airbrake columns
are a pure function of the input script and `dt` - no trajectory reaches them -
so a mismatch there is a harness bug by construction, and every physics number
below it is meaningless. The two-tick `--script-lead` offset sat in plain sight
in `oag-trace compare`'s own output (`throttle max error 1.000e2 at tick 1`) for
a session while the rows underneath it were read as a physics wedge. See
[`oag-trace.md`](../tools/oag-trace.md#the-first-two-ticks-of-a-script-never-reach-the-emulator).

**A control that ramps without clamping has no attractor, and an input error in
one never decays.** The steering ramp travels toward its target without clamping
to it, so a saturated axis oscillates around it forever; the airbrake ramp
clamps and resynchronises on every transition. The same two-tick input error
washed out in six ticks on one and survived all 3,146 on the other.

**Check a capture is clean before fitting anything to it.** `speed` and
`|velocity|` agree to 1e-6 in free flight, so `speed / |velocity| == 1.0000` is
a free contact detector. Two reference captures were recorded scraping a wall
for their whole length, and the resulting "missing linear resistance" stood as
the M4 blocker for a session and a half. A related bound: a friction measurement
only means anything while the craft is *moving* - below a few units per second
the normal impulse dominates and restitution can push `|velocity|` above
`speed`, giving a negative loss.

**Ask the tangent, not the index.** "Which way round the circuit is this
capture going" is not a question the nearest-sample index can answer.
`Spline::from_track` concatenates paths in file order, so on a track with
junctions index order is not travel order; and on a slow capture most windows
step by zero and the rest are jitter. `dot(craft_forward, sample.tangent)` has
neither failure mode and reads `+0.9999` against `-0.9999`.

**Stability against a threshold is not stability across frames, and only the
second one usually matters.** A craft-scale measurement was published as
"measured soundly at last" on the strength of a 0.3 px spread across a 2.25x
threshold sweep - and withdrawn when the same measurement across four ticks
showed a **10.4 %** spread. Sweep the *frame*, not just the parameter.

*The sequel is better than the rule and inverts its own example.* That 10.4 %
was written up as "a spread on a quantity that must be constant", and it was
not: the quantity **was** varying, monotonically with speed, because the
original widens its field of view with speed and we do not
([projection-vs-the-original.md](../rendering/projection-vs-the-original.md)).
The cross-frame sweep was right to withdraw the number and wrong about why - it
had found a real signal and labelled it instrument noise. So the rule has a
second half: **when a cross-frame sweep disagrees with itself, check whether the
"constant" is tracking something before blaming the detector.** Sort the
disagreement by every column the capture carries - speed first - and see whether
it orders.

**Prefer a measurement carrying a control able to refute it**, and this is the
one that would have caught all of it: an exposure scale on an unchanged frame, a
ship-free render, a vertex dump, or - for anything about an object's size on
screen - **the background of the same frame**, which no mesh scale can move.
Six days went into a craft-specific error that did not exist because the one
free control nobody ran was the one that decides between "our model is wrong"
and "our camera is wrong". **The same rule, on a live debugger this time
(2026-09-01)**: a paused-GDB read of a PS3 shader engine-parameter slot came
back `(0,0,0,0)` on all 15 tries across three boots, which reads exactly like
"the value is zero" and would have shipped as a refutation of an open question
- until the same pause instants were made to also read a neighbouring slot
that *must* hold a real, per-frame-changing camera position if the read
mechanism works at all. It did, every time, which is what turned the null
from "nothing here" into "this specific slot is dead, and the mechanism
around it is not" - a materially different, narrower claim. A zero from one
address is not evidence until something that cannot legitimately be zero is
read in the same breath. See
[renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md#a-controlled-live-read-the-engine-param-tables-fogcolour-slot-is-dead-in-gameplay-but-that-doesnt-answer-unscaled-2026-09-01).

**The ramped columns (`craft+0x2c0`-`0x2c8`) lag the frame that used them by one
tick** - the original ramps and consumes in one call, and a capture samples at
entry. Naive pairing multiplies a yaw fit's rms by 5.6.

**The recorded basis columns must be read as Left-Up-Forward.** Taken at their
column names the basis is a reflection, and a 1.25 rad/s residual reads as 32.

**Never read timing off the HUD under breakpoints.** The race clock does not
count emulated frames; laps of identical tick counts timed `0.50.25` and
`1.11.08`. The lap *counter* is unaffected.
