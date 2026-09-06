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

- **What moved ship 7 into the blade's path: found.** `cc395862` ("hold AI opponents at
  the line through the start countdown") is the exact commit - see "Bisect result"
  below. The lateral/heading readout there also answers *how*: it is not a track-side
  bug, it is the field staying grid-tight because the countdown hold now genuinely
  works, and this test throws only 120 ticks (2 seconds) after release.
- **Whether `hull_radius`'s lateral width is over-wide** for a 20-degrees-off-nose
  throw, **still open**. The module docs mark that width **"ours, not recovered"**, and
  clearing the AI-timing question above does not clear this one - a grid-tight field at
  throw time is now confirmed *plausible*, not confirmed *correctly sized*. This is a
  `crates/physics` question and out of this session's lane.
- **Whether the test's own premise is still right** given a grid-tight field is now the
  true post-countdown picture: throwing 120 ticks after release may simply be too early
  in a real race for "the blade clears the whole field" to be a fair assertion at all.
  Unresolved - needs the play/PPSSPP read in Next Step 2 below.

## Bisect result (Next Step 1 - done)

Two-sided adjacent-commit check, not a full `git bisect` sweep (the pair already gives
stronger attribution than a sweep would): built and ran
`a_thrown_blade_bounces_off_a_real_circuit_and_dies_on_its_fuse` against
`data/images/pulse-psp-usa.chd` at each commit, twice each for determinism.

- **`9785a177`** (parent of `cc395862`): **PASS**, twice.
- **`cc395862`** itself: **FAIL**, twice, panicking at the identical
  `shuriken_ground_truth.rs:201` `one press threw 0 blades` assertion that `d11c5fb7`
  and current `main` also panic on - i.e. this is the same failure the thread opened
  with, not a different one along the way.

`cc395862` is `fix(game): hold AI opponents at the line through the start countdown` -
`Race::step_opponents` gained the same `RaceState::thrust_gated` zero-thrust gate the
player already had (`crates/game/src/race/field.rs`). Before it, every opponent drove
at full throttle from tick 0 while the player sat out the measured 272-tick countdown;
by the time this test's `WARM_UP_TICKS` (`COUNTDOWN_TICKS + 120`) throws its blade, the
opponents already had a 272-tick unearned head start.

### Discriminating check: which mechanism moved ship 7

Two mechanisms fit the same bisect and point opposite ways for Next Step 2, so this was
checked directly rather than assumed:

- **(a) Field still in grid order** - everyone releases together now, so 120 ticks
  later the straggler is legitimately still beside you. Fix is right; the test throws
  too early relative to a synchronized start.
- **(b) The gate is incomplete** - the diff zeroes only `controls.thrust`; steering
  stays live through the hold (deliberately, for the player too - see the comment at
  `crates/game/src/race/tick.rs:29-34`, "only thrust was held and recorded"). If yaw
  displaces a parked opponent laterally, that is a `field.rs` bug in this thread's own
  lane, not a test or `hull_radius` question.

Checked with a temporary diagnostic (`along`/`lateral`/heading-dot-forward per opponent,
printed right before the throw, reverted before committing - never landed):

| commit | slot 7 along | slot 7 lateral | slot 7 dist | heading·fwd range (all 7) |
| --- | --- | --- | --- | --- |
| `9785a177` (pre-fix) | 536.3 | 143.0 | 555.3 | 0.10 - 0.73 |
| `cc395862` (post-fix) | 13.3 | -7.8 | 15.4 | 0.9971 - 1.0000 |

Post-fix headings are all ~1.0 (parallel to the player, no rotation), and the whole
field sits 15-136 units out in grid order rather than 536-681 units out on a since-
curved line. That is **mechanism (a)**: no lateral-displacement artifact from the
steering-stays-live design, just a field that has not had time to spread out yet. The
fix in `cc395862` is not implicated as a bug in its own right.

## Next Steps

1. ~~Bisect `b4c45477`..`d11c5fb7`~~ - done above; `cc395862`, mechanism (a).
2. **A from-play or PPSSPP read settles what's left**: does a Shuriken thrown just off
   the nose, seconds after a real race's start-line release, clip the craft beside you?
   If yes, the test's premise needs revisiting (it is asserting something that does not
   happen this early in a real race). If no, `hull_radius`'s unrecovered lateral width
   is the next suspect, in `crates/physics` - out of this session's lane.
3. Separately: `cc395862` updated the three synthetic `oag-race`/`oag-game` tests its
   own gate broke, but this `#[ignore]`d disc-backed test - invisible to CI - broke too
   and landed green anyway. Worth a `HANDOVER.md` "traps that are live" line: a
   start-gate or grid-timing change can only be caught by `just test-data`, not `just
   test`.

## Notes

- Do not "fix" this by widening the test's tolerance or by making the blade ignore a
  hull. The detonation follows a documented rule; the question is why a hull is there.
- `oag-gameplay` is determinism-bound. Anything that changes where a blade goes will
  move the committed state hash - explain the movement, and **never** edit the
  reference constants in `crates/core/src/hash.rs`.
