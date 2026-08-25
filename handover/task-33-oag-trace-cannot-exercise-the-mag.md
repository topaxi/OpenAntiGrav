# Task #33: `oag-trace` cannot exercise the mag-lock hold

**Landed and measured.** `replay`, `drive`, `drive_with` and `plan::to_gate`
all take an `Option<&[oag_formats::track::Sample]>` now, located fresh every
tick from the ship's own position - `crate::replay::locate`, the same
"nearest table entry and its successor" pair `oag_game::race::Race::tick`
reads for the player, resampled independently because `oag-trace` cannot
depend on `oag-game` (rule 2, `scripts/check-dependency-rules.py`).
`crates/trace/src/replay/tests.rs` pins the mechanism with a synthetic
magstrip fixture first; the real measurement below is what actually settles
it.

## The real-lap measurement

`data/traces/` had no Talon's Junction capture in this checkout, so one was
taken live: PPSSPP v1.20.4 under Xvfb, booted straight off
`pulse-psp-usa.chd` (no `chdman` extraction needed - see
`docs/reverse-engineering/ppsspp-debugger.md`), driven into a Time Trial, then
flown one lap by `scripts/psp-autopilot.py --spline <oag-trace track output>
--trace-out data/traces/talons-junction-autopilot.csv --script-out
verification/scenarios/talons-junction-inverted-section.inputs`. That
recorded script is now committed - regenerate the capture from it with:

```sh
uv run --with websocket-client scripts/psp-drive.py restart
uv run --with websocket-client scripts/psp-trace.py \
    --script verification/scenarios/talons-junction-inverted-section.inputs \
    --script-lead 2 --out data/traces/talons-junction-autopilot.csv
```

**The first capture taken (`talons-junction-time-trial-lap.csv`, from the
already-committed `verification/scenarios/talons-junction-time-trial-lap.inputs`)
turned out to be the wrong one to test this on** - its `up_y` column never
drops below `-0.5` anywhere in 3,146 ticks, so that scripted line never
enters the inverted section at all. The `1,072-1,329` tick range
`cornering-ground-truth.md` and `maglock_ground_truth.rs` cite is from a
*different* capture (`talons-junction-time-trial-lap-omega.csv`, flown live by
the autopilot, whose script "was deliberately not committed"), not from the
time-trial-lap script's own ticks - a coincidence of similar tick counts (3,146
vs 3,140) that cost real time to notice. The freshly-flown lap above does go
inverted, at ticks 1,131-1,234 in that specific capture.

Compared with `--reseed 10` (so the run stays near the recorded pose rather
than measuring open-loop chaos - see `docs/tools/oag-trace.md`), locator
attached versus not, against the fresh capture:

| window (ticks)          | mean pos error, with locator | mean pos error, without |
| ------------------------ | ----------------------------: | ------------------------: |
| 1,050-1,131 (approach)   | 0.208                          | 0.408                     |
| 1,131-1,234 (inverted)   | 0.190                          | 0.414                     |
| 1,235-1,300 (exit)       | 0.161                          | 0.449                     |

The two runs are bit-identical before tick 1,034 and diverge from each other
from there on - right where the inverted section's reseed window begins. A
diagnostic count of `oag_physics::maglock::probe`'s raycasts over the whole
lap found 298 of 3,020 hit `Surface::MagFloor` (versus `Floor` on the
open track), confirming the mechanism is engaging on real geometry, not a
coincidence of the reseed windows. **With the locator attached, position
error through and around the inverted section is roughly half what it is
without one** - a real, measured improvement from a mechanism that could not
be reached by this tool before this task.

## Open

- The residual left after the hold's 49.5 % (per `maglock_ground_truth.rs`)
  may still come from locator fidelity - our 4-per-segment resampled spline
  may not match the original's evaluated curve. The measurement above shows
  the hold itself helps; it does not separate how much of the *remaining*
  error is locator resolution versus something else. Nobody has taken that
  finer measurement.

## Next Steps

- If the residual above is worth chasing further: densify the resample (more
  than 4 samples per segment) for `talons-junction-inverted-section.inputs`
  specifically and see whether the inverted-section error shrinks further,
  which would point at locator fidelity rather than the force law.
