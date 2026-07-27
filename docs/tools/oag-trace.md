# `oag-trace`

Runs our simulation over a trace captured from the original and reports **the
first tick at which the two diverge, and by how much**. The reading half of M3's
verification harness; the writing half is
[`scripts/psp-trace.py`](../reverse-engineering/ppsspp-debugger.md), and what the
two are for is [the verification protocol](../reverse-engineering/verification-protocol.md).

```
oag-trace show    <trace.csv>                      what is in a capture
oag-trace run     <trace.csv> [--source <image>]   replay it and compare
oag-trace compare <recorded.csv> <simulated.csv>   diff two traces
```

Traces are **derived game data**. They live under `data/traces/`, which
`.gitignore` covers, and are never committed - so nothing here runs in CI, and
every test in the crate works from a hand-authored fixture instead.

## The whole loop

```sh
# 1. capture, out of a race already running in PPSSPP
just trace --ticks 300 --hold cross --warmup 4 --out data/traces/venom-straight.csv

# 2. replay and compare
cargo run -p oag-trace -- run data/traces/venom-straight.csv \
    --source data/images/pulse-psp-usa.chd --hold cross --out data/traces/ours.csv
```

The capture side's traps - a breakpoint added while the CPU runs never fires, a
memory read costs 520 ms running and 0.3 ms stepping, and holding thrust through
the countdown gets you a false-start penalty that looks exactly like a broken
input path - are all on
[the debugger page](../reverse-engineering/ppsspp-debugger.md). Read it before
capturing.

## What the output says

```
first divergence at tick 1 (row 1), field velocity
  recorded (0.000000, 0.000000, 22.000000), simulated (0.000000, 0.000000, 21.926260)
  error 0.073740 units/s (3.352e-3 relative), tolerance 1e-3 relative
  and on the same tick: speed by 0.073740 units/s, grounded by 1.000000

field                   max error   at tick      max rel  exceeded  trend
position                  1.507e1       199     3.406e-1         5  growing (slope 7.630e-2/tick, 3.344e-1 -> 1.177e1)
velocity                  8.733e0       199     3.970e-1         1  growing (slope 4.365e-2/tick, 1.209e0 -> 7.757e0)
...
```

Not a pass or a fail. A subsystem that tracks the original for 400 ticks and then
drifts has a different bug from one that is wrong at tick 1, so the tick is the
headline; and because an error that grows every tick is systematic even while it
is inside tolerance, every field also carries a **trend** - `exact`, `bounded`,
`growing` or `shrinking`, from a least-squares slope and a comparison of the
first quarter's mean error against the last quarter's.

Fields that go on the same tick as the leader are listed with it. One term wrong
usually shows up in one field; a wrong initial condition shows up in all of them.

## Tolerances

Straight from the protocol's table, and provisional there and here. Each is
overridable (`--position-absolute`, `--orientation`, `--velocity-relative`, ...)
so a measurement can be argued about without editing code - not so a comparison
can be made to pass.

| Quantity | Default | Fields |
| --- | --- | --- |
| Position | 0.01 units absolute **or** 1e-4 relative | `position` |
| Orientation | 1e-4 rad, as the angle between the axes | `orientation.row0`, `.up`, `.forward` |
| Velocity | 1e-3 relative | `velocity`, `speed`, `speed_cached` |
| Control states | 1e-3 relative | `throttle`, `brake`, `steer`, `airbrake_l`, `airbrake_r` |
| Discrete | **exact** | `grounded` |

Control states are not in the protocol's table, which predates the capture
format; comparing them at the velocity tolerance is a provisional choice.

Two absolute floors exist and are **off by default**: `--velocity-absolute` and
`--control-absolute`. A purely relative test has no meaning when both sides are
near zero - on a standing start a simulated `1e-9` against a recorded `0` is a
relative error of 1 - so raising them is a deliberate widening of the protocol
and should be said out loud wherever the result is quoted.

## The two conventions that have to be reconciled

**The recorded basis.** The capture's rows are named `right`, `up` and `fwd`, and
row 0 was measured to be the ship's **left** (confidence 84; see the
[roadmap](../overview/roadmap.md)). The recorded basis is positively oriented -
`cross(row0, row1) = row2`, 200/200 - while `oag_physics::Body`'s is not, since
its forward is `-Z`; so mapping one onto the other takes an *odd* number of sign
flips and a reading that flipped only row 0 would be a reflection rather than a
rotation. Hence two coherent readings, and `--basis` picks between them:

- `left-up-forward` (default): the measured reading.
- `right-up-back`: the column names taken at face value, which then forces row 2
  to be the tail.

Running a comparison both ways and seeing which keeps the orientation inside
`1e-4` rad settles the question, which is why it is a switch and not a constant.

**Sampling phase.** The capture breaks on `Ship_UpdateCraft`'s *entry*, so a
recorded row is the craft as the update found it. A simulated row is therefore
emitted **before** its step. Getting this backwards shifts every field by one
tick and reads as a small systematic lag in the physics.

## What a run is seeded with, and what it cannot be

A run starts from the recording's first row: position, basis, velocity and the
five control states. Four things are not in the capture and are therefore not
replayed, rather than being invented:

- **Angular velocity.** A run starts with the rotation at rest, so capture on a
  straight, not in a corner.
- **Pitch.** The craft's control block holds `steer` and no second axis, so
  `steer_y` is always zero.
- **`time_since_landing` and the leap timer.** A run starts outside the landing
  window with no leap in progress.
- **Raw stick positions.** `steer`, `brake` and both airbrakes are recorded
  *after* the original's own ramps, so replaying them as inputs ramps them twice.
  Use `--hold cross` (and `--steer`, `--airbrake-left`, `--airbrake-right`) to
  drive the run with the constant input the capture was actually taken with;
  without any of those the run falls back to deriving input from the recorded
  states, which is general but approximate.

Without `--source` there are no handling parameters and no track: the run is a
coast, useful for checking the harness and useless for checking the force law.
With one, the handling stats and the track's collision geometry are read off the
disc exactly as `oag-game` reads them.

## Options

| Option | Meaning |
| --- | --- |
| `--source <image>` | Disc image or an `oag-unpack` extraction, for handling and collision |
| `--track`, `--team`, `--class` | Which track, team and speed class the capture was taken in |
| `--hold <button>` | Hold a button for the whole run, as `psp-trace.py --hold` did. Repeatable |
| `--steer`, `--airbrake-left`, `--airbrake-right` | Hold an axis for the whole run |
| `--fixed-dt` | Step at our own 60 Hz instead of the recording's own frame times |
| `--basis` | `left-up-forward` (default) or `right-up-back` |
| `--no-collision` | Ignore the track's geometry: a ship with nothing to hover on |
| `--out <path>` | Write the simulated trace, in the same columns |

## Not yet

- No save state and no committed input script, so a scenario is "whatever was
  captured" rather than one of the protocol's seven named ones.
- Nothing compares a PSP capture against a PS2 one.
- Shield, weapons, lap and race-position state are in the protocol's list of what
  gets traced and in neither the capture nor this tool: nothing downstream of the
  craft has been decoded yet.
