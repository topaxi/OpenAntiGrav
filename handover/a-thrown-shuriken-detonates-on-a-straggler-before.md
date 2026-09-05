# A thrown Shuriken detonates on a straggler before the test can see it fly

`crates/game/tests/shuriken_ground_truth.rs::a_thrown_blade_bounces_off_a_real_circuit_and_dies_on_its_fuse`
is deterministically red - reproduced identically across three full `just test-data`
runs on 2026-09-05 (contended at load 22-29, quiet, and plain), and it is one of only
**two** tests red under the plain documented command. Not a flake, not an environment
gap: the test needs only a disc image, which is present.

## The first attribution was wrong, and the refutation is the finding

This thread was opened as *"...after the vector class refactor"*, blaming commit
`0c78c477` ("VECTOR speed class selectable"), on the reasoning that it made
`shuriken::launch` return an `Option` through a name-keyed lookup while its own
commit message claimed the `None` path was unreachable on measured Pulse data.

**That reading is refuted, and `0c78c477`'s claim is correct.** Measured directly by
instrumenting `shuriken::launch` against real Pulse `VENOM` data:

- `speed_for_named("VENOM")` returns `Some(700.0)`.
- `launch()` returns `Some(..)`.
- `throw()` succeeds, `live = 1`.
- **None of the four candidate early-returns fires.**

`Global::class_named` and `Stats::class_named` also resolve correctly on measured
data. (`pickup::table_for` is not exercised by this test, which sets the pickup
directly, so it is untested here rather than cleared.)

**And the failure predates the commits it was blamed on.** Checking out `d11c5fb7` -
the commit immediately before `91f192dd`/`0c78c477` - rebuilding, and rerunning the
same test against the same disc image reproduces **the identical failure**. So this is
not a same-day regression at all, and the baseline's "both deterministic failures are
fresh regressions" line is wrong on this one.

## What actually happens

The freshly-thrown blade's **first same-tick advance sweeps into opponent ship 7's
hull** - 11.4 units away, a normal race straggler, while the other six are 34-131
units clear ahead - and detonates per the documented "a hull hit always detonates"
rule, before the test can observe it alive. The test is not wrong about what it
asserts; the blade genuinely dies immediately.

## Open

- **What moved ship 7 into the blade's path is unknown.** The last known-good touch of
  this test is `b4c45477` (2026-09-02), so something between there and `d11c5fb7`
  changed the grid's spacing, the straggler's pace, or the throw geometry.
- **Whether `hull_radius`'s lateral width is over-wide** for a 20-degrees-off-nose
  throw. The module docs mark that width **"ours, not recovered"** - so this may not be
  a regression in our code at all, but an invented number finally colliding with a
  case that exposes it.

## Next Steps

1. **Bisect `b4c45477`..`d11c5fb7`** against this test to find what moved ship 7. That
   range is the whole remaining search space and the test runs in seconds by name -
   this is the cheapest path to an answer and should be done first.
2. **Then decide which side is wrong.** If the bisect lands on a deliberate change to
   grid spacing or opponent pacing, the blade hitting a straggler may be *correct* and
   the test's premise is what needs revisiting. If it lands on nothing plausible,
   `hull_radius`'s unrecovered lateral width is the next suspect.
3. **A from-play or PPSSPP read would settle the second question directly**: does a
   Shuriken thrown just off the nose in a real race clip the craft beside you? The
   maintainer's play observations have been a reliable oracle; this is a good question
   to ask before more static work.

## Notes

- Do not "fix" this by widening the test's tolerance or by making the blade ignore a
  hull. The detonation follows a documented rule; the question is why a hull is there.
- `oag-gameplay` is determinism-bound. Anything that changes where a blade goes will
  move the committed state hash - explain the movement, and **never** edit the
  reference constants in `crates/core/src/hash.rs`.
