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
| Save state | A fixed, reproducible starting point. Committed as a reference by name, not content. |
| Input script | Per-tick controller state. Plain text, committed. |
| Trace capture | Emulator-side recording of the observed values, per tick. |
| `oag-trace` | Runs our engine on the same input and compares. |

## What gets traced

Per tick, per ship:

- Position, orientation, linear and angular velocity
- Speed, thrust, airbrake state
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
