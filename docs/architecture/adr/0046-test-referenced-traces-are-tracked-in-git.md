# ADR-0046: A trace a test names is tracked in git

## Status

Accepted, 2026-09-06. Narrows [ADR-0006](0006-no-copyrighted-content.md)'s
`data/` exclusion for one specific class of file. It does not touch that ADR's
rule about shipped content - assets, executables and extracted disc data stay
excluded exactly as before, and `just audit-leakage` still enforces it.

## Context

**Five of the six trace captures this workspace's ground-truth tests name are
absent from this checkout**, measured 2026-09-06:

| Capture | State |
| --- | --- |
| `talons-junction-time-trial-lap.csv` | present |
| `pad0-boost.csv` | missing |
| `talons-junction-pitch-both-ways.csv` | missing |
| `talons-junction-standing-start.csv` | missing |
| `talons-junction-time-trial-lap-omega.csv` | missing |
| `venom-straight.csv` | missing |

They were not deleted by any policy. `data/` is gitignored, so they only ever
existed in whichever checkout captured them, and this one no longer has them
(a second workstation is being checked as of 2026-09-06).
The docs still assert they exist: `docs/physics/cornering-ground-truth.md`
states flatly "That capture was taken" of the `-omega` lap, and
`docs/physics/angular-velocity-column.md` builds an argument on it.

**The loss was silent, and that is the worse half.** A ground-truth test whose
reference is absent *skips*. It does not fail. So the M3 verification harness -
the half of this project that compares our simulation against the original -
degraded to one working capture without a single red build. The failure only
became visible when someone ran with `OAG_REQUIRE_GAME_DATA=1`, which converts
those skips into hard errors, and that variable is not part of any routine
command.

**A trace is not shipped content.** Its columns are `tick, dt, grounded,
throttle, brake, steer, airbrake_l, airbrake_r, speed_cached, stun_timer,
shield`, three orientation vectors, position, velocity and angular velocity -
measured runtime state, produced by observing the original execute. This is the
same category as the confidence-scored readings throughout `docs/`, and a
weaker case for exclusion than the shipped tuning values this project already
commits with a maintainer's explicit approval (see `HANDOVER.md`'s "Pending
maintainer decision" section and its two resolved instances).

**One honest caveat.** `pos_x/y/z` sampled over a full lap is a sparse sampling
of the circuit's own geometry - one racing line, not the mesh, but derived from
shipped geometry all the same. That is a real if narrow exposure, recorded here
rather than left implied. It is judged acceptable against the alternative: a
verification harness that quietly stops verifying.

## Decision

**A trace that a test names is tracked in git. Every other capture is not.**

Re-inclusion is per file, by name, in `.gitignore`:

```
!/data/traces/pad0-boost.csv
!/data/traces/talons-junction-pitch-both-ways.csv
...
```

Never `!/data/traces/` and never a glob over the directory. A new capture is
untracked until someone adds its name deliberately, in the same change that
adds the test naming it. Exploratory captures - the `hd-trail-*` sets, anything
taken to answer a question rather than to pin a test - stay excluded.

The list is the allowlist in `.gitignore` and nothing else. There is no second
register to drift out of sync with it.

## Consequences

**A reference capture cannot go missing again.** Absence stops being possible
for the tracked set, which is what makes the follow-up worth doing: the
skip-on-missing path for these tests should be removed, so a reference that
somehow is not there breaks the build instead of quietly reducing coverage.
Until that lands, `OAG_REQUIRE_GAME_DATA=1` remains the only way to see the
difference, and it is worth running before trusting a green `just test-data`.

**The repository carries the weight.** The tracked set is roughly 2-3 MB today
at one capture per test, and git keeps every revision forever. A recaptured
trace is a new blob, not a delta - CSV floats do not diff usefully - so
re-running a capture and committing it costs its full size again. Compressing
to `.csv.zst` was considered and declined: it would cut the size by about ten
times, and it would make the files opaque to `git diff` and put a decompression
step in the ground-truth path, which is the path that most needs to be boring.

**It narrows an exclusion that was easier to reason about when it was total.**
"Nothing under `data/` is ever committed" needs no judgement. "Nothing except
traces a test names" needs someone to decide, each time, whether a new file
qualifies. The per-file allowlist is the mitigation - the decision is visible
in a diff, and it cannot be made by accident.

**The five missing captures are not recovered by this.** They come back either
from another checkout that still holds them - being checked as of 2026-09-06 -
or by recapturing to the recipe in
[ppsspp-debugger.md](../../reverse-engineering/ppsspp-debugger.md). Until one of
those happens those tests stay unexercisable, and the docs that cite them are
asserting something this checkout cannot demonstrate. This ADR is what makes the
recovery stick once it happens: a trace that arrives is tracked the moment it
lands, rather than living in one workstation again.
