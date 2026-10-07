#!/usr/bin/env python3
"""A ratchet on how long `just test-data` takes, per test and in total.

`just test-data` went from **3:17 to 9:47** between 2026-08-17 and 2026-09-09
without one commit that looked wrong. `ai_roll_ground_truth`'s full-grid test
landed on 2026-09-06 measuring **525 s** - 89% of the suite's entire 587 s wall
clock on its own - with its cost budgeted honestly in its own doc comment and
nothing in the gate to object. That is the same failure mode
`check-file-size.py` was written for: every diff is small, and the thing that
grows is never anybody's diff.

**Why a per-test ceiling and not just a total.** `cargo nextest` runs every test
in its own process and parallelises across *tests*, so a matrix crammed into one
`#[test]` runs on one core while the other fifteen idle. The suite is tail-bound
to the point that the slowest single test *is* the wall clock: on 2026-09-09
only 62 s of a 587 s run was not overlapped with `ai_roll`. A total alone tells
you the suite got slower; the per-test ceiling tells you which test to split,
which is the whole of the fix.

Splitting is nearly always available and nearly always free, because these are
matrices - N circuits by M tiers by K seeds - and the assertion usually
partitions with it. `max` over a partition is `max` over the whole;
`assert_eq!` against an ordered list filters with the list. See
`stall_rescue_ground_truth.rs` for the simplest worked example and
`ai_roll_ground_truth.rs` for the one where the split had to be argued.

# What this checks

- The suite's own wall clock may not exceed `SUITE_CEILING`. This is the gate
  that does most of the work.
- A test not in `BASELINE` may not exceed `CEILING`, which catches a single
  pathological test that somehow fits inside a passing suite.
- A test in `BASELINE` may not exceed the seconds recorded there, times
  `TOLERANCE`. It may shrink freely.
- Once a baselined test fits **comfortably** under `CEILING` - under
  `CEILING / TOLERANCE` - its row must be deleted, so the baseline can only get
  shorter. The gap matters: a test sitting either side of the ceiling would
  otherwise fail one run for having no row and the next for having one.

**A ceiling is lowered, never raised.** Raising one turns a ratchet into a
record of what happened, which is what the absence of this script already was.

# Why the numbers have slack in them

A nextest duration is **wall-per-test under load**, not isolated CPU: the
2026-09-09 run's durations sum to 4,639 s against 3,143 s of actual CPU, because
a test descheduled behind fifteen others still reports the time it sat there. On
fewer cores, or on a busy machine, the same test honestly reports longer. So
`CEILING` sits at 180 s against a post-split maximum of 114 s, and `TOLERANCE`
gives baselined rows a fifth on top.

**This gates gross regressions, not drift.** It is here to catch the next 500 s
test at review, not to make anyone chase a 12% wobble.

# Running it

    just test-data              # writes target/test-data.log and checks it
    just check-test-budget PATH # check a log you already have

`just test-data-affected` runs it with `--partial`: a member's run of the
packages its branch changed. Every test that ran is still held to `CEILING`
(or its `BASELINE` row), but `SUITE_CEILING` is not checked, because a partial
run's wall clock says nothing about the whole suite's, and no `BASELINE` row is
called for deletion off a partial run. The ceiling on the whole suite is the
lead's full `just test-data` once per merge batch.

It is deliberately not in the default `just` gate: that runs `test`, which skips
every `#[ignore]`d test, so there would be nothing to measure.
"""

import re
import sys
from pathlib import Path

# **Measured on 2026-09-09, after the ADR-0050/ADR-0051 crate splits, on an
# idle machine with everything pre-built:**
#
#   commit 6120c207 (before the splits)   4,114 tests   488 s
#   commit 7983eda3 (after them)          4,117 tests   415 s
#
# The suite got **15% faster with three more tests in it**, and the mechanism is
# worth stating because it is not the one either ADR predicted. `oag-game` is
# deliberately `opt-level = 0` - see the build-profile table in
# `docs/architecture/workspace-layout.md` - so the ~18,000 lines that moved out
# of it into `oag-ui`, `oag-display` and `oag-race` are now compiled at
# `opt-level = 2` when the tests run them. Both ADRs argued the splits buy no
# build time, which is true of *compiling and linking*; neither noticed that
# moving code out of the composition root optimises it.
#
# The `344 s` figure that CLAUDE.md and the ceilings below were once calibrated
# against is **not reproducible on this machine** - the pre-split commit measures
# 488 s idle. Treat the numbers here as calibrated against the two runs above.

# Seconds a single test may take before it has to be split or baselined.
#
# **This is a duration under load, not an isolated cost, and the two differ by a
# lot.** The six `ai_roll` grid tests measure 95-114 s each run on their own and
# 154-321 s in a full `test-data` run, because 4,100 other tests are competing
# for the same sixteen cores. The factor is not fixed either - it fell with the
# suite's total CPU, and the slowest test went from 321 s to 184 s without that
# test changing at all. So this is set from the measured full-run maximum with
# room over it, not from an isolated timing; the tail before the 2026-09-09
# split was 525 s.
CEILING = 300.0

# The suite's own wall clock, from nextest's `Summary` line, and the gate that
# does most of the work: a test can only be slow at the suite's expense.
# Measured at 344 s on 2026-09-09, against 587 s before that day's work, so this
# leaves about a third of headroom - enough that ordinary growth does not trip
# it and little enough that a second 500 s test does.
SUITE_CEILING = 450.0

# How far over a recorded baseline a run may land before it counts as growth
# rather than as machine noise. See "Why the numbers have slack in them".
TOLERANCE = 1.2

# Tests that cannot currently be brought under `CEILING`, with the seconds they
# measured on the day they were baselined and why they are here. A row is
# deleted the moment its test fits.
BASELINE: dict[str, float] = {
    # `ai_roll_ground_truth`'s six full-grid tests (including
    # `a_full_grid_of_skilled_arms_no_fewer_rolls_than_novice_at_the_held_out_seed`,
    # which carried a 305 s row here from 2026-09-09 to 2026-09-13) do
    # identical work and land within a ~90 s band of each other depending on
    # what they overlap with in a given run - `CEILING` sits inside that band,
    # so whichever one draws the short straw is the one that trips, and
    # whichever measured over it on a given day is the one that needed a row.
    # Measured clear of `CEILING` (158-165 s) in two back-to-back full
    # `test-data` runs on 2026-09-13, on the same machine and load - not
    # independent samples, so this is one data point on one afternoon, not
    # proof the contention this file's own history warns about is gone. The
    # row is deleted per this file's own rule regardless: re-add it,
    # re-profiled, the day a run actually needs it.
    #
    # `ps2_source_ground_truth`'s uncapped transcode - 950 frames of the PS2
    # intro, with `refresh: true` load-bearing for what it asserts, so the work
    # cannot be cached away - measured 160 s on 2026-09-09 and needs no row.
}

# `PASS [   1.234s] (  12/4101) crate::binary test_name`
RESULT = re.compile(
    r"^\s+(?:PASS|FAIL|TRY \d+ FAIL)\s+\[\s*([0-9.]+)s\]\s+\(\s*[\d/ ]+\)\s+(\S+)\s+(\S+)"
)
SUMMARY = re.compile(r"^\s*Summary\s+\[\s*([0-9.]+)s\]")


def measure(log: str) -> tuple[dict[str, float], float | None]:
    """The slowest run of each test in `log`, and the suite's wall clock."""
    durations: dict[str, float] = {}
    wall = None
    for line in log.splitlines():
        if found := RESULT.match(line):
            seconds, binary, name = found.groups()
            test = f"{binary} {name}"
            durations[test] = max(durations.get(test, 0.0), float(seconds))
        elif found := SUMMARY.match(line):
            wall = float(found.group(1))
    return durations, wall


def check(
    durations: dict[str, float], wall: float | None, partial: bool = False
) -> list[str]:
    failures = []
    for test, seconds in sorted(durations.items(), key=lambda kv: -kv[1]):
        allowed = BASELINE.get(test)
        if allowed is None:
            if seconds > CEILING:
                failures.append(
                    f"{test}\n"
                    f"    took {seconds:.0f}s, over the {CEILING:.0f}s ceiling.\n"
                    f"    Split it: nextest parallelises across tests, so a matrix in one\n"
                    f"    #[test] runs on one core. See this script's own doc comment."
                )
        elif seconds > allowed * TOLERANCE:
            failures.append(
                f"{test}\n"
                f"    took {seconds:.0f}s against a baseline of {allowed:.0f}s "
                f"(+{TOLERANCE:.0%} slack).\n"
                f"    A baseline is lowered, never raised - find what grew."
            )
        elif seconds <= CEILING / TOLERANCE and not partial:
            # Not merely `<= CEILING`: a test that lands either side of the
            # ceiling from run to run would fail one for having no row and the
            # next for having one.
            failures.append(
                f"{test}\n"
                f"    took {seconds:.0f}s, now clear of the {CEILING:.0f}s ceiling.\n"
                f"    Delete its BASELINE row in scripts/check-test-budget.py."
            )

    if wall is not None and wall > SUITE_CEILING and not partial:
        slowest = sorted(durations.items(), key=lambda kv: -kv[1])[:5]
        table = "\n".join(f"      {s:6.0f}s  {t}" for t, s in slowest)
        failures.append(
            f"the suite took {wall:.0f}s, over the {SUITE_CEILING:.0f}s ceiling.\n"
            f"    Even with every test under its own ceiling the total can drift up as\n"
            f"    tests are added. Re-profile the tail before adding more:\n{table}"
        )
    return failures


def main() -> int:
    args = sys.argv[1:]
    partial = "--partial" in args
    args = [a for a in args if a != "--partial"]
    path = Path(args[0] if args else "target/test-data.log")
    if not path.exists():
        print(
            f"no run to check at {path} - `just test-data` writes it",
            file=sys.stderr,
        )
        return 1

    durations, wall = measure(path.read_text(errors="replace"))
    if not durations:
        print(f"{path} holds no nextest results", file=sys.stderr)
        return 1

    failures = check(durations, wall, partial)
    if failures:
        print(f"test budget, from {path}:\n", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}\n", file=sys.stderr)
        return 1

    slowest = max(durations.values())
    print(
        f"test budget: {len(durations)} tests, slowest {slowest:.0f}s, "
        f"suite {wall:.0f}s"
        if wall
        else f"test budget: {len(durations)} tests, slowest {slowest:.0f}s"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
