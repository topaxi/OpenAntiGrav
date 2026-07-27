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

## What the first real comparison found

Run against a real capture - the [reference
scenario](../reverse-engineering/ppsspp-debugger.md#the-reference-scenario), 200
ticks of Talon's Junction White in Venom on an Assegai - against our own
simulation seeded from the same first row and given the same held thrust:

| Field | First tick outside tolerance | Max error | Trend |
| --- | ---: | ---: | --- |
| `speed` | 0 | 188 units/s | growing |
| `grounded` | 1 | 1.0 (exact field) | bounded |
| `velocity` | 1 | 211 units/s | growing |
| `orientation.forward` | 1 | 1.52 rad | growing |
| `speed_cached` | 2 | 188 units/s | growing |
| `position` | 3 | 424 units | growing |
| `throttle`, `brake`, `steer`, airbrakes | - | 0 | exact |

`speed` leads the report and should be read past: it is out at tick 0, which is
the *seeded* row, so it cannot be a physics result. It is the `+0x398` mismatch
described below - the recording disagreeing with our model of what that column
holds. The physics headline is the tick 1 group.

**The ship falls through the floor at the first step.** `grounded` is 1.0 on all
200 recorded ticks and 0.0 on all but tick 0 of ours - and tick 0 is the seeded
initial condition, so our hover probes find nothing *on the original's own
starting position*. Everything below it in the table follows from that: our ship
free-falls 420 units while the original stays on the track, and its speed grows
to 211 units/s against a recorded 22.7. The control columns match exactly, so the
input path and the scaling are right and the divergence is entirely in the force
law. This is the measurement `docs/physics/README.md`'s hover section was waiting
for: the probe reach, the ride height, or the target height is wrong, and the
trace says which tick to look at.

**The handedness question is settled, and not by simulation.** The capture alone
answers it: the ship travels along **+row 2** (`dot(velocity_hat, fwd) = +0.997`
on every tick), so row 2 is the nose, and `cross(row0, up) = forward` holds
200/200. `oag_physics::Body` has `right x up = -forward`, so row 0 cannot be the
right axis under that convention - it is the **left**. That is an independent
confirmation of the 84-confidence finding in the roadmap, which came from the
sign of the yaw response to a held steer rather than from the direction of
travel. Running the comparison the other way (`--basis right-up-back`) is worse
on every field - 2.80 rad of orientation error against 1.52, 512 units of
position against 423 - which is the switch doing its job.

**Neither speed column is the velocity's length.** `speed_cached` on the craft is
**the previous tick's `dot(velocity, forward)`** - the forward-projected speed -
which is what
[`engine.md`](../ghidra/functions/psp-pulse/engine.md#the-cached-speed-and-its-staleness-measured)
already had at confidence 95 and what a shorter pass here wrongly recorded as a
stale magnitude. This capture corroborates the page and tightens it: over 200
ticks the residual is a mean of 3.3e-6 and a max of 9.7e-6, which at a speed of
24 is the capture's own `%.7g` print rounding, so it is indistinguishable from
exact rather than merely inside a 0.01 tolerance. The stale magnitude is out by a
mean of 0.077.

It also pins *which* vectors are stale, which the tolerance test could not:

| Candidate | Mean residual |
| --- | ---: |
| `dot(velocity(t-1), forward(t-1))` | 3.3e-6 |
| `dot(velocity(t-1), forward(t))` | 1.3e-3 |
| `dot(velocity(t), forward(t-1))` | 1.6e-2 |

Both vectors are the previous tick's, by a factor of 400 over the mixed reading -
so the cache is written from a coherent pre-integration state, not part-way
through the update.

Confidence **90**: one binary, one capture, and a residual four orders of
magnitude below the competing reading - but a *straight* run, where the
projection and the magnitude differ by only the 0.3 % that a `+0.997` heading
cosine allows. The two readings separate properly only under slip, and nothing
real has been captured there yet; the crate's own across-the-nose test is
synthetic. That is what holds this below `engine.md`'s 95 rather than above it.

`speed` at body `+0x398` is neither: it runs 3.67 % above the length (ratio
1.0367, sd 0.0026) and 4.02 % above the forward projection (1.0402, sd 0.0028),
and those spreads overlap, so this capture cannot say which it is a scaling of,
nor separate a constant factor from a constant offset across a 3 % speed range.

A run writes the projection for `speed_cached`, which is what was measured, and
the length for `speed`, which is the closer of two readings that this capture
cannot separate. So a comparison carries a permanent ~3.5 % divergence on `speed`
from tick 0 onwards. That is the recording disagreeing with our model of it
rather than a physics error, and it is left visible rather than scaled away -
but it does mean `speed` heads the first-divergence line while saying nothing
about the force law.

## Not yet

- No save state and no committed input script, so a scenario is "whatever was
  captured" rather than one of the protocol's seven named ones.
- **No cornering capture.** The reference scenario is a straight, where the
  forward projection and the velocity's magnitude agree to 0.3 %. Everything the
  speed columns above claim is therefore confirmed only in the regime where the
  candidate readings nearly coincide; a capture through a corner, at real slip,
  is what would settle them.
- Nothing compares a PSP capture against a PS2 one.
- Shield, weapons, lap and race-position state are in the protocol's list of what
  gets traced and in neither the capture nor this tool: nothing downstream of the
  craft has been decoded yet.
