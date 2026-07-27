# Determinism

The simulation must produce **bit-identical state on every platform we ship**,
given identical inputs.

This is not a nice-to-have. Three things depend on it, and all three become
worthless without it:

- **Replays** are stored as inputs, not as state. A non-deterministic sim
  desynchronises and the replay shows something that never happened.
- **Golden tests** compare state hashes. Without determinism they fail randomly
  on other people's machines and get disabled.
- **Behavioural verification** compares our per-tick output against a trace from
  the original. If our own output varies between runs, the comparison measures
  our noise rather than our accuracy.

Determinism is also cheap if designed in and expensive if retrofitted, which is
why the [CI gate](../../.github/workflows/ci.yml) shipped in M0, before there
was a simulation to guard.

## What is guaranteed, and what is not

**Guaranteed:** the same build, given the same inputs, produces the same state
on Linux, Windows and macOS, on x86-64 and AArch64, in debug and release.

**Not guaranteed:** bit-identical agreement with the PSP or PS2. The PSP's VFPU
is not IEEE-conformant for reciprocal and reciprocal square root, so matching it
exactly would require a software VFPU. We target behavioural equivalence within
documented tolerances instead. See
[ADR-0002](adr/0002-determinism-model.md) and the
[verification protocol](../reverse-engineering/verification-protocol.md).

## Rules

### Floating point

- **`f32` only** in the simulation, with strict IEEE-754 semantics.
- **No `mul_add`.** Fused multiply-add keeps a wider intermediate, so
  `a.mul_add(b, c)` and `a * b + c` do not always produce the same result.
  Whichever the original used, we pick deliberately rather than letting the
  optimiser or the target ISA pick.
- **No fast-math.** Rust does not enable it. Do not add it through build flags.
- **Fixed operation order.** Do not reassociate arithmetic for readability. If a
  sum must be grouped a particular way, group it and say why.
- **No SIMD in the simulation.** `glam` is built with `scalar-math` for exactly
  this reason; vector paths are free to reassociate and to differ by CPU feature
  level.

`sqrt` is safe: IEEE-754 requires it to be correctly rounded. Transcendentals
(`sin`, `cos`, `exp`, `ln`) are **not** required to be correctly rounded and are
a genuine portability risk, because they resolve to the platform's libm. The
determinism probe uses `sin` deliberately, so this shows up as a test failure
rather than as a mystery desync. If platform libm differences turn out to be
unavoidable, the fix is to bring our own implementations into `oag-core`, not to
weaken the test.

### Iteration order

- **No `HashMap` or `HashSet` iteration feeding simulation state.** Rust's
  default hasher is randomly seeded per process; iteration order is not stable
  even between two runs of the same binary.
- Use arrays, `Vec`, `BTreeMap`, or an explicit sort.

### Time

- **Never read the wall clock in the simulation.** Use
  [`TickClock`](../../crates/core/src/tick.rs).
- **Fixed timestep, always.** The simulation never sees a variable delta. A
  variable timestep makes physics frame-rate dependent, which breaks every form
  of verification.
- Rendering interpolates between the last two states using
  `TickClock::interpolation_alpha`.
- The accumulator carries integer nanoseconds, not a float. That bounds the
  drift rather than removing it: `1_000_000_000 / 60` truncates, so the clock
  gains one tick every 4.8 days of continuous simulation. The figure is pinned
  by a test, because the previous test claimed no drift while feeding the clock
  its own rounded tick length and so could not have seen any.

### Randomness

- **No OS entropy.** Every draw comes from a seeded
  [`Rng`](../../crates/core/src/rng.rs) whose state is part of the world
  snapshot.
- The current generator is a placeholder: `xoshiro128**`, pinned to the
  published reference vector. Wipeout Pulse has its own, and AI decisions and
  pickup rolls will only match once it is recovered. Routing every draw through
  `oag-core` from the start makes that a one-file change.
- **A placeholder still needs an external anchor.** This one shipped with its
  state update written as one parallel assignment instead of four sequential
  ones, which cost it two terms and its bijectivity, and every test passed
  because they all compared the generator against itself. The determinism gate
  could not see it either: it catches a generator that *drifts*, not one that
  was wrong from the first commit.

### Threads

- **The simulation is single-threaded** unless and until there is a measured
  need. If that changes, parallelism must be deterministic: fixed partitioning,
  no work stealing, no completion-order dependence.
- Asset loading, audio and rendering may thread freely. They are not simulation.

## The tick rate, and how the original differs

`TickRate::DEFAULT` is **60 Hz**, and this is now evidence-backed: fourteen
sites in the original sub-step at exactly `1/60`, and every exponential
smoothing coefficient in the engine is tuned for a 1/60 s step. See
[frame pacing](../psp/frame-pacing.md).

**But the original does not use a fixed timestep at all.** It measures real
elapsed time each frame and passes it straight into the update, presenting at
one vblank while racing (~59.94 Hz) and two in menus (~29.97 Hz), with no
catch-up when a frame overruns.

So 60 Hz is the rate the original was *authored against*, not the rate it
*steps at*. We fix the step anyway, deliberately, because determinism is worth
more to this project than frame-for-frame agreement under load. That decision
and its costs are [ADR-0007](adr/0007-fixed-timestep-vs-original.md).

Two rules follow, and they are easy to get wrong:

- **Where the original sub-steps at 1/60 and discards the remainder, do the
  same.** Carrying the remainder would be a real behavioural difference in the
  smoothing, even though carrying it is the textbook approach and is what
  `TickClock` correctly does at the outer level.
- **Trace comparison is only valid while the original held its frame rate.**
  Captured traces must record the original's per-frame delta so that drifted
  intervals can be told apart from genuine bugs.

`TickRate` stays a parameter rather than a constant, since the craft physics
integrator has not been read yet and may turn out to want different treatment.

## How this is enforced

| Layer | What it catches | Scope |
| --- | --- | --- |
| [`oag-core::probe`](../../crates/core/src/probe.rs) | Float divergence in the shared foundation | `oag-core` only |
| [`crates/core/tests/determinism.rs`](../../crates/core/tests/determinism.rs) | Drift against committed reference hashes | `oag-core` only |
| CI `determinism` job | Divergence between Linux, Windows and macOS | runs `-p oag-core` only |
| [`StateHasher`](../../crates/core/src/hash.rs) | Per-tick state comparison | **no callers outside `oag-core`** |

The probe exercises what the real simulation will lean on hardest: accumulated
arithmetic, `sqrt`, `sin`, quaternion composition and normalisation, seeded
random draws, and a data-dependent branch.

### The gate does not cover `oag-physics` or `oag-gameplay`

Audited 2026-07-27, and worth stating plainly because the table above used to
end with "once there is state". **There is state now, and nothing hashes it.**

- [`probe::run`](../../crates/core/src/probe.rs) is a hand-built miniature
  simulation living inside `oag-core`. It hashes exactly **three quantities** -
  a position, a velocity and an orientation, ten `f32` - plus one `u32` drawn
  from the RNG. It cannot reach the simulation crates, because `oag-core` is the
  bottom of the dependency graph and nothing may point upward from it.
- `oag_gameplay::World` is `tick`, an `Rng`, and `[Ship; 8]`; each `Ship` holds a
  `ShipState` of eleven fields, whose `Body` alone is four `Vec3`, a `Quat`, a
  scalar mass and an inertia `Vec3`. **None of it is hashed by anything.**
- `StateHasher` has no caller anywhere outside `oag-core`.
- The CI job that runs on three operating systems runs `-p oag-core`. The job
  that runs the whole workspace runs on Linux only, so the physics and gameplay
  tests are not even *executed* on Windows or macOS, let alone compared.

**What this does and does not mean.** The foundation-level risk is genuinely
covered, and by a superset of what the simulation currently does: the probe uses
`sin`, and the simulation crates today call **no transcendental at all** - three
`sqrt` sites and glam's own `length`/`normalize`, which are `sqrt`. So a
platform whose float pipeline diverges would still be caught.

What is not covered is anything a simulation crate could introduce *for itself*.
Specifically, and none of these has any equivalent in the probe:

- **Order-dependent reductions over collections.** The nearest-hit scan across
  `CollisionWorld::colliders()`, the deepest-of-eight hull probe comparison, and
  the nearest-sample scan in the race loop are all reductions whose result
  depends on iteration order. They are over `Vec`s today, which is correct - but
  nothing would fail if one became a `HashMap`.
- **Accumulation order.** `forces::evaluate` sums roughly fourteen terms into
  four accumulators in a fixed order that is itself evidence. The probe sums
  three.
- **`Quat::inverse` and quaternion-vector rotation**, which the integrator uses
  every tick to move torque in and out of the body frame. The probe composes and
  normalises quaternions but does neither of these.

So the honest summary is that the gate proves the *floor* is portable and proves
nothing about the two crates built on it. Closing that needs a hash-and-compare
over a real `World` after N ticks, with its own committed reference and its own
place in the CI matrix. It is deliberately **not** done as part of this audit:
the reference constant has to be generated from the simulation as it stands, and
generating one while the force law is actively being changed would commit a
number that is stale on arrival - which, under this page's own "never update the
constants" rule, is worse than the gap it closes.

**When the determinism test fails, do not update the constants.** That converts
a real bug into a silent one. The failure output names the platform, and each CI
log already contains the `determinism_report` output for its platform.
