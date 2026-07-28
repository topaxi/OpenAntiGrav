# What `body+0x160` holds, measured

**Status: settled by measurement on five captures. The column is the craft's
own angular velocity, in body-local components, negated, and multiplied by
about `21.2` on the yaw axis.** That last factor is the finding: it is the same
number [`YAW_DRIVE_CALIBRATION`](../../crates/physics/src/forces.rs) was fitted
to, arrived at from a completely different direction and with no simulation
involved, which turns a fitted constant into a measured one.

The two questions this answers were both open, and both were open in a way that
static reading could not close:

- **Sign.** [The PS2 craft update](../ghidra/functions/ps2-pulse/craft-update.md)
  derives `w_game = -w_physics` from the integrator on three independent legs.
  That is a result about the *accumulators*; whether the stored velocity carries
  the same convention was the obvious reading rather than a measured one.
- **Frame.** The same page names `body+0x160` `angularVelocityLocal` and then
  lists which frame it is expressed in as unresolved, and
  [engine.md](../ghidra/functions/psp-pulse/engine.md) caps the local/world split
  of the angular accumulators at confidence **74**.

## The measurement

A capture records the craft's basis rows and the `body+0x160` column on the same
tick. The rows are orthonormal, so they differentiate into the angular velocity
the craft *demonstrably* had:

```text
w = 0.5 * sum_i cross(e_i, de_i/dt)
```

No simulation, no disc, no handling data enters that. `oag_trace`'s
`Summary::angular_readings` scores the four readings of the column - sign x
local/world - against exactly this, and on a real capture it comes back
**undecided**: every residual lands within a few percent of the column's own
rms. The reason is that the two quantities are not in the same units, and a
one-to-one comparison cannot see a scale factor - it scores every reading as
equally wrong.

`scripts/trace-angular-fit.py` fits the scale first. Five captures, all Time
Trial / Venom / Talon's Junction White / Assegai, with deliberately different
dynamics - a standing start that barely rotates, a scripted turn each way, a
scripted left-then-right, and an asymmetric-airbrake run:

| capture | world, one scale | local, one scale | `k_y` | yaw explained |
| --- | ---: | ---: | ---: | ---: |
| standing start | 25.8 % | **83.9 %** | -22.19 | 98.2 % |
| steer both ways | 65.9 % | **91.9 %** | -21.34 | 96.5 % |
| steer left | 77.8 % | **93.3 %** | -21.20 | 98.4 % |
| steer right | 74.6 % | **93.4 %** | -21.28 | 98.2 % |
| asymmetric airbrake | 72.7 % | **92.4 %** | -21.16 | 97.7 % |
| **1,095 ticks pooled** | 75.0 % | **93.1 %** | **-21.23** | **98.1 %** |

"Explained" is `1 - residual/signal`; the two frame columns are the same model
with the same one free parameter, so they are directly comparable and the better
one is the frame.

Three readings come out of that table:

1. **The frame is local.** Local beats world on every capture, and by a factor of
   3.6 in residual when pooled (`1.37` against `4.95`). This is *not* a
   marginal call that better data would flip, and it does not depend on the
   craft being tilted: the discrimination comes from the roll and pitch
   components, whose body axes are far from the world axes because of the
   craft's **heading**, not its attitude. The standing start, which never tilts
   more than `3.5°`, separates them most sharply of all.
2. **The sign is negated.** The fitted scale is negative on all five captures, so
   the stored column is the negation of the physical angular velocity. The
   negated and un-negated readings can only ever *tie* on residual - a free
   scale absorbs the sign - so the sign is read off `k`, not off a fit. This
   confirms `w_game = -w_physics` for the stored velocity, and confirms the PS2
   page's `angularVelocityLocal` name on both halves.
3. **The yaw axis carries a factor of `21.2`.** `1/|k_y| = 0.0471` against the
   fitted `YAW_DRIVE_CALIBRATION = 0.0452` - **4 % apart**, from measurement that
   shares no input with the fit.

Confidence **90** for the frame and the sign (five captures, consistent, and the
frame margin is 3.6x rather than a few percent); **85** for `k_y = 21.2 +/- 0.4`.

## What the factor is, and what it is not

`forces.rs` records a lead at confidence **45**: that `1/0.0452 = 22.1` is a yaw
moment of inertia, and that the drive is divided by one while the damping is
not. **The number is now confirmed and the derivation offered for it is
refuted.** A textbook box tensor for the shipped Assegai hull
(`Misc width="5.5" g="13" h="3.5"`, mass `1`) gives `I_yy = (5.5² + 13²)/12 =
16.6`, which is 28 % below the measured `21.2` - outside anything these captures
leave room for. So the factor is real, it is a division by something with the
dimensions of an inertia, and the hull's bounding box is not where it comes from.

**Do not read a diagonal inertia tensor into the per-axis fits.** Pooled, the
roll and pitch axes are only 36 % and 29 % explained by their own constants,
against 98 % for yaw, because those components are small and the captures barely
excite them. `k_x` and `k_z` scatter from `-14` to `-24` across the five files;
`k_y` scatters from `-21.16` to `-22.19`. Only the yaw number is determined.

A direct probe for the constant came back **negative**: nothing in
`body+0x000..0x400` or `craft+0x000..0x400` holds a float near `21.2` or
`0.0471` at runtime. The same probe did confirm, at runtime, what
[the force-balance page](force-balance-ground-truth.md) read statically -
`body+0x374 = 1.0` and `body+0x378 = 1.0`, mass and its reciprocal, stored
unscaled.

## Consequences

- `Summary::angular_readings` **cannot decide this question in its current
  form**, and a future reader should not take its ordering as an answer. It
  needs a fitted scale per reading before the residuals mean anything; until it
  has one, its four numbers differ by less than the noise. The verdict above
  came from `scripts/trace-angular-fit.py` instead.
- `YAW_DRIVE_CALIBRATION` should stop being described as a fitted stand-in for
  an unknown term. It is a division by `21.2` on the yaw axis, and the reason a
  single global constant works across teams is that the factor is a property of
  the *body layer*, not of `Turning.amount`.
- Whether the factor is per-ship is still open and still decided the same way:
  capture a second team, ideally at the far end of the `amount` range, and refit
  `k_y`. If it moves, the constant has to become per-hull.

## Reproducing it

```sh
python3 scripts/trace-angular-fit.py data/traces/talons-junction-*.csv --joint
```

Captures are derived game data and are not committed; see
[the capture page](../reverse-engineering/ppsspp-debugger.md) for how to take
them, and `verification/scenarios/` for the committed input scripts the four
scripted ones were driven by.
