# ADR-0002: Determinism model

## Status

Accepted.

## Context

Replays, golden tests and behavioural verification all compare simulation state.
All three are worthless if the simulation is not reproducible.

There is a second, separate question: how closely should our arithmetic match
the *original's* arithmetic? These get conflated, and conflating them leads to
either an unachievable goal or an untestable one.

The PSP's VFPU is fast but not IEEE-conformant. Its reciprocal and reciprocal
square root instructions are approximations with documented error bounds rather
than correctly rounded results. The PS2's VU0 and VU1 are worse: no denormals,
no infinities, no NaN, and a different rounding mode.

Reproducing either exactly means writing a software VFPU or VU. That is a large
amount of reverse engineering, a large runtime cost, and a large ongoing
maintenance burden.

## Decision

Two goals, stated separately, with different strengths.

**Goal 1, hard: cross-platform reproducibility.** The same build, given the same
inputs, produces bit-identical state on every platform we ship. Enforced by CI
on Linux, Windows and macOS from M0 onward.

**Goal 2, soft: behavioural equivalence with the original.** Verified by
comparing traces within documented tolerances, not by bit comparison.

Implementation:

- `f32` throughout the simulation, strict IEEE-754.
- No `mul_add`, no fast-math, no FMA contraction, no reassociation.
- `glam` with `scalar-math`, so no SIMD path is taken.
- Fixed timestep with an integer-nanosecond accumulator.
- Seeded PRNG whose state is part of the world snapshot; no OS entropy.
- No `HashMap` iteration feeding simulation state.
- Single-threaded simulation.

Where the original turns out to use fixed-point, we mirror the fixed-point
format exactly. Fixed-point is integer arithmetic and is therefore bit-exact for
free. `oag_core::math::fixed` exists for this; whether Pulse's physics is
fixed-point is an open question for M2.

## Alternatives considered

**Bit-exact via a software VFPU.** Maximum fidelity, and the strongest possible
verification signal: a trace comparison becomes a bit comparison, and any
divergence is unambiguously a bug. Rejected on cost. It requires reverse
engineering the exact VFPU approximation tables, costs perhaps an order of
magnitude in simulation throughput, and would have to be redone for the PS2's
VUs. The fidelity gain is imperceptible in play.

**Fixed-point everywhere.** Bit-exact for free, and a reasonable guess for an
engine of this era. Rejected as premature: we do not yet know whether Pulse's
physics is fixed-point. Forcing fixed-point onto float code would introduce
quantisation error the original does not have, which is *less* faithful, not
more.

**`f64` for extra headroom.** Rejected. Wider intermediates would diverge from
the original more, not less, and reproducibility does not depend on precision.

**Accept non-determinism and compare with tolerances everywhere.** Rejected.
Replays stored as inputs stop working, and every test needs a hand-tuned epsilon
that hides real regressions.

## Consequences

**Good.** Replays are inputs, so they are tiny and survive engine changes.
Golden tests compare one hash. A desync is always a bug with a first divergent
tick, not a mystery. The CI gate makes drift impossible to introduce quietly.

**Bad.** Some optimisations are off the table: no SIMD in the simulation, no
FMA, no reassociation. This costs measurable performance, though the simulation
is not expected to be the bottleneck for eight ships.

Transcendentals remain a live risk. IEEE-754 does not require `sin`, `cos` or
`exp` to be correctly rounded, so they resolve to the platform's libm and may
differ. The determinism probe uses `sin` deliberately, so this surfaces as a
test failure. If it fires, the fix is to bring our own implementations into
`oag-core`, which is a known cost we have chosen to defer rather than pay
upfront.

**We cannot claim bit-exactness with the original, and should not.** Any
documentation that implies otherwise is wrong. The honest claim is behavioural
equivalence within stated tolerances, and the
[verification protocol](../../reverse-engineering/verification-protocol.md) is
where those tolerances live.
