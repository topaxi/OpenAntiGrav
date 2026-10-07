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
| [`oag-core::probe`](../../crates/core/src/probe.rs) | Float divergence in the shared foundation | `oag-core` |
| [`oag_physics::probe`](../../crates/physics/src/probe.rs) | Drift in the real force law, under a scripted input | `oag-physics` |
| [`crates/gameplay/tests/determinism.rs`](../../crates/gameplay/tests/determinism.rs) | Drift in what a race carries around the dynamics: inventory, projectiles, lap state, the generator's position | `oag-gameplay` |
| [`oag_ai::probe`](../../crates/ai/src/probe.rs) | Drift in what the *drivers decide* - the aim point, the curvature, the speed target, the personality draws | `oag-ai` |
| CI `determinism` job | Divergence between Linux, Windows and macOS | all four of the above, release **and** debug |
| [`StateHasher`](../../crates/core/src/hash.rs) | Per-tick state comparison | every gate above is built on it |

The core probe exercises what the real simulation leans on hardest: accumulated
arithmetic, `sqrt`, `sin`, quaternion composition and normalisation, seeded
random draws, and a data-dependent branch. The three above it step the real
code.

### Each gate covers the layer below it, and one thing more

Audited 2026-07-27, when the honest summary was that the gate proved the *floor*
was portable and nothing about the crates built on it. That is no longer true,
and the order the holes were closed in is worth keeping, because each was closed
by adding the layer that could see what the one below could not:

- **`oag-physics` (2026-07-29)**: the force law itself, over a synthetic
  corridor. The core probe cannot reach it - `oag-core` is the bottom of the
  dependency graph and nothing may point upward from it.
- **`oag-gameplay` (2026-08-11)**: a race's own state, which the pickups opened
  a hole in and `hash_world` closed.
- **`oag-ai` (2026-08-15)**: the controller. The physics gate drives a
  *scripted* input, so until this landed **no cross-platform run had ever asked
  a driver to decide anything**, and every arithmetic path in `oag-ai` was
  outside the gate.

That last gap was not hypothetical. `oag_ai::Line::curvature` called the
platform's own `acos` from the day it was written, in violation of the rule
above, and nothing failed for months because nothing cross-platform ran it.
Measured over the domain a clamped unit-vector dot actually produces, this
machine's `f32::acos` and `libm::acosf` differ on **8.6 % of samples**, by up to
one ULP - `2.4e-7` rad at the extreme and `3e-8` in the band a racing line lands
in. One ULP in an angle that sets a speed target is one bit in the world hash.

**And it is enforced now, not merely stated.**
[`scripts/check-transcendentals.py`](../../scripts/check-transcendentals.py)
fails the `just` gate and the `check` CI job on any of the forbidden calls in
`crates/{core,physics,gameplay,ai,race,formats}/src`, outside `#[cfg(test)]`.
**`oag-formats` is in that set and is not simulation code**: handling stats,
splines and track data are parsed there and handed straight to the simulation,
so a transcendental applied to a parsed value at load time reaches the world
hash exactly as surely as one applied at tick time. It joined on 2026-08-18
(finding I2), when the crate set was narrower than the claim it was enforcing.
Three files are allowed by name and each says why: `oag_core::math` holds the
wrappers, `oag_core::probe` calls `sin` deliberately so that a platform whose
libm differs shows up as a failing gate rather than as a mystery desync, and
`oag_formats::entropy` scores a byte histogram for `oag-unpack sniff` - a
triage label a human reads, which no parser branches on and which reaches
nothing. Adding a fourth entry is a decision about determinism, not a
formality. The script was
checked against the bug that motivated it - restoring `f32::acos` in
`Line::curvature` makes it fail, naming the line.

`sqrt` is deliberately not on its list, and neither are `to_radians` and
`to_degrees`: the first is required to be correctly rounded, the other two are
a multiply by a constant.

**And a textual gate over method names had a hole exactly the shape of its
patterns.** `glam` in this workspace is `["std", "scalar-math"]` with no `libm`
feature, so `Quat::from_axis_angle` is `sin_cos(angle * 0.5)` through the
platform's libm - the same portability hole as `f32::acos`, wearing a type name
instead of a method name. Both patterns wanted a leading `.` or an `f32::`, so
neither matched it, and `oag_weapons::projectile::launch` computed the rocket
fan's spread directions through it into `Projectile::velocity`, which is hashed
state. Three gates missed it at once (finding D1, 2026-08-18): this script's
patterns, the gameplay determinism scenario that spawned rockets with
hand-written velocities and never called `launch`, and the physics probe, which
fires no weapon. The fix is all three - a
[`quat_from_axis_angle`](../../crates/core/src/math.rs) wrapper beside `acos`,
a third pattern here over the bare constructor names, and a committed volley
scenario in `crates/gameplay/tests/determinism.rs` that fires a real fan from a
craft deliberately not axis-aligned. `from_mat3`, `from_rotation_arc` and
`lerp` stay off the list: they are `sqrt` and arithmetic, and a gate that
flags clean code is one people learn to skip.

**The fix is the one this page already named**: bring the implementation in
rather than weaken the rule. [`oag_core::math::acos`](../../crates/core/src/math.rs)
wraps the `libm` crate - a pure-Rust port of MUSL's libm, so every target runs
the same source - and simulation code calls that instead of `f32::acos`. It is
not promised to be correctly rounded, because no libm is; it is promised to be
the *same everywhere*, which is what determinism needs. Any further
transcendental the simulation turns out to want goes in beside it.

### What is still not covered

- **Order-dependent reductions over collections.** The nearest-hit scan across
  `CollisionWorld::colliders()`, the deepest-of-eight hull probe comparison, and
  the nearest-sample scan in the race loop are all reductions whose result
  depends on iteration order. They are over `Vec`s today, which is correct - but
  nothing would fail if one became a `HashMap`.
- **`oag-race`**, which has no gate of its own; lap timing reaches the committed
  hashes only through `oag_gameplay`'s scenario.
- **Anything that needs a disc image.** Every gate above builds its world in its
  own file, deliberately: `data/` is gitignored and absent in CI, so a
  disc-backed scenario would never run on Windows or macOS, which is precisely
  where a portability bug shows up. What that costs is that no *authored* track,
  hull or racing line is under a cross-platform hash - those are covered by the
  disc-backed ground-truth suites, on one machine, via `just test-data`.

### 2026-10-07: the core probe's own `sin` is the first cross-platform failure

The first `ci.yml` runs to reach the determinism tests in about 26 days (runs
37595443950, 37599197810, 37599357599) failed `oag-core`'s reference on macOS
(aarch64) and Windows (x86_64), Linux passing. The cause is the probe itself, not
simulation code: [`probe.rs`](../../crates/core/src/probe.rs) calls the platform's
`f32::sin` and glam's `Quat::from_axis_angle` (platform `sin_cos`), allowlisted in
`scripts/check-transcendentals.py` precisely so a libm difference fails here. It
did. The reference was recorded on x86_64 glibc, so it bakes glibc's `sinf`.

Evidence that this is libm and nothing else, reproduced on Linux by swapping only
those two calls for `oag_core::math`'s (the `libm` crate):

| Probe variant on Linux (glibc 2.44) | Matches |
| --- | --- |
| `math::sin_cos` for `sin`, `math::quat_from_axis_angle` | Windows' `ticks=1000` trajectory `0xbcd056e334f2b1cb` exactly |
| `math::sin_cos` for `sin`, glam's `from_axis_angle` kept | macOS' `ticks=100000` final `0x1877155ea261a7ba` exactly |
| unchanged | the committed reference (glibc's own bits) |

Both swaps matter: each alone moves the Linux trajectory hashes. That all three
platforms agree once the probe takes `oag_core::math` is expected (the `libm`
crate is pure Rust) but unverified: Windows' 10,000- and 100,000-tick
trajectories match none of the Linux variants, and confirming takes one push. Confidence the
divergence is libm alone: 90 (two cross-platform hashes reproduced bit for bit;
the other stages, physics, race-level and driver, never ran on macOS or Windows
because the job stopped at the first assert, so whether *they* hold is unknown).

Not fixed here, because it moves the committed constants: switching the probe to
`oag_core::math` changes the Linux trajectory hashes (finals unchanged on Linux)
to `0xbcd056e334f2b1cb`, `0xa774f231b9e343d0`, `0x31337c04f2032805`. It also ends
the probe's role as a platform-libm canary, which `check-transcendentals.py`
already covers for every scanned crate. That regeneration is the maintainer's
decision. Note that `ubuntu-latest` moves to Ubuntu 26 (a newer glibc) from
2026-10-19, so a glibc-baked reference can go red on Linux too.

**When the determinism test fails, do not update the constants.** That converts
a real bug into a silent one. The failure output names the platform, and each CI
log already contains that gate's `*_determinism_report` example output for its
platform.
