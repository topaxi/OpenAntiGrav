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
- The accumulator carries integer nanoseconds, not a float, so it cannot drift
  over a long session.

### Randomness

- **No OS entropy.** Every draw comes from a seeded
  [`Rng`](../../crates/core/src/rng.rs) whose state is part of the world
  snapshot.
- The current generator is a placeholder. Wipeout Pulse has its own, and AI
  decisions and pickup rolls will only match once it is recovered. Routing every
  draw through `oag-core` from the start makes that a one-file change.

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

| Layer | What it catches |
| --- | --- |
| [`oag-core::probe`](../../crates/core/src/probe.rs) | Float divergence in the shared foundation |
| [`crates/core/tests/determinism.rs`](../../crates/core/tests/determinism.rs) | Drift against committed reference hashes |
| CI `determinism` job | Divergence between Linux, Windows and macOS |
| [`StateHasher`](../../crates/core/src/hash.rs) | Per-tick state comparison, once there is state |

The probe exercises what the real simulation will lean on hardest: accumulated
arithmetic, `sqrt`, `sin`, quaternion composition and normalisation, seeded
random draws, and a data-dependent branch.

**When the determinism test fails, do not update the constants.** That converts
a real bug into a silent one. The failure output names the platform, and each CI
log already contains the `determinism_report` output for its platform.
