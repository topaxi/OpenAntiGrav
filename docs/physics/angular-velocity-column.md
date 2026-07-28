# What `body+0x160` holds, measured

**Status: settled by measurement on five captures. The column is the craft's
own angular velocity, in body-local components, negated, and multiplied by
about `21.2` on the yaw axis.** That last factor is the finding: it is the same
number [`YAW_DRIVE_CALIBRATION`](../../crates/physics/src/forces.rs) was fitted
to, arrived at from a completely different direction and with no simulation
involved, which turns a fitted constant into a measured one.

**Read this page's later sections in order - it was written in three passes and
the conclusion moves.** The measurement below is unchanged and still stands. What
changed is what it is a measurement *of*: the tensor's writer has since been read
(a hard-coded solid box giving `21.6`), the constant is now
`oag_physics::forces::YAW_INVERSE_INERTIA` and is recovered rather than fitted,
and this page's own numbers turned out to discriminate which mass the tensor was
built with. See the last two sections.

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
(**Superseded for these five captures only** - the warning is about *them*, and
it stands for them. Two captures taken later with a real pitch input determine
`k_x` to `0.13 %` and `k_z` to `2 %`; see
[the pitch section](#the-pitch-axis-measured-directly---and-the-tensor-confirmed-on-all-three).)

A direct probe for the constant came back **negative**: nothing in
`body+0x000..0x400` or `craft+0x000..0x400` holds a float near `21.2` or
`0.0471` at runtime. The same probe did confirm, at runtime, what
[the force-balance page](force-balance-ground-truth.md) read statically -
`body+0x374 = 1.0` and `body+0x378 = 1.0`, mass and its reciprocal, stored
unscaled.

## This resolves the 22x yaw discrepancy, and vindicates the law

[engine.md](../ghidra/functions/psp-pulse/engine.md) closes its
"torque or angular acceleration" question by observing that steering and damping
land in **one** accumulator (`craft+0x340` -> `body+0x120`), so any common factor
- the inertia tensor included - cancels at equilibrium, leaving

```text
omega_y = steer * Turning.amount / 5
```

unconditionally. Against the shipped Assegai `Turning.amount` (a design value,
read off the user's own disc and not recorded here per
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md) and
[handling-stats.md](../formats/handling-stats.md)'s schema-not-values rule) and
a `steer` column that saturates near `100`, that predicts a value around `33` -
and the craft observably yaws at about `1.5` rad/s, 22x less. That gap is what
`YAW_DRIVE_CALIBRATION` exists to paper over.

**The law is right. It predicts the stored column, not the rotation.** Held full
lock, `data/traces/talons-junction-steer-right.csv`:

| tick | `steer` | stored `avel_y` | observed yaw | ratio |
| ---: | ---: | ---: | ---: | ---: |
| 30 | 91.5 | 25.86 | -1.226 | -21.08 |
| 45 | 97.8 | 31.50 | -1.485 | -21.21 |
| 60 | 104.1 | 32.84 | -1.535 | -21.40 |
| 65 | 98.2 | **32.92** | -1.546 | -21.30 |

`steer * amount / 5` evaluated with the disc's own `amount` at that plateau
lands within about **2 %** of the stored `32.92` still settling onto it. The
mirrored left capture climbs the same curve with the sign flipped. (The stored
and observed columns above are runtime measurements; the design value itself is
recoverable from any of them and deliberately not printed.)

So the cancellation argument is **correct about the accumulator** and the
equilibrium it derives is now confirmed by measurement. What it cannot do is
carry over to the observable, because the accumulator's equilibrium value *is
not the angular velocity* - it is the quantity this page measures at `21.2` times
the rotation performed. One factor survives, in the conversion, and it is exactly
the missing 22x.

Two things follow. The note's "it does not matter" should be read as scoped to
the accumulator rather than to the yaw rate: **the equilibrium is not
inertia-independent in anything observable.** And `YAW_DRIVE_CALIBRATION` is not
covering for a missing or mis-signed *term* - every term in the yaw law is
confirmed, and the constant is the unit conversion between the two quantities.

Confidence **85**: two captures, mirrored, agreeing to 2 % on a formula derived
independently at instruction level, with the ratio holding at `-21.0` to `-21.4`
across the whole ramp rather than only at the plateau.

## Consequences

- `Summary::angular_readings` **cannot decide this question in its current
  form**, and a future reader should not take its ordering as an answer. It
  needs a fitted scale per reading before the residuals mean anything; until it
  has one, its four numbers differ by less than the noise. The verdict above
  came from `scripts/trace-angular-fit.py` instead.
- `YAW_DRIVE_CALIBRATION` should stop being described as a fitted stand-in for
  an unknown term. It is a division by `21.2` on the yaw axis, and the reason a
  single global constant works across teams is that the factor is a property of
  the *body layer*, not of `Turning.amount`. **Granted in full by the last
  section: the constant is now recovered and renamed.**
- Whether the factor is per-ship was open here and decided the same way -
  capture a second team and refit `k_y`. **That is no longer needed**: the last
  section reads the tensor's writer, and the box is a code literal shared by
  every craft.

## Confirmed at instruction level: the factor is the inertia tensor

The `I * w` reading above was an inference from a fit. It is now read out of
`Body_Integrate` (`0x0884e230`), and it holds:

- **`body+0x160` is angular momentum**, body frame. Torque integrates into it
  directly - `+0x160 += torque * h` at `0x0884e354`-`0x0884e364` - with no
  inverse inertia and no mass divide, where the linear half four instructions
  earlier does scale by `invMass`. That asymmetry only makes sense for `dL/dt`.
- **`body+0x150` is the angular velocity**, and it is what turns the basis
  (`0x0884e2f8`-`0x0884e334`).
- **The map between them is `basis^T * (body+0x40) * basis`**
  (`0x0884e380`-`0x0884e39c`), a tensor change of basis, so **`body+0x40` is the
  body-space inverse inertia tensor**.

So the per-axis factor this page fitted is the diagonal of `I`, the negative sign
is the `w_game = -w_physics` convention, and the roadmap's "do the angular
accumulators hold torque or angular acceleration" is answered: **torque**.
Confidence **88**; evidence in
[rigid-body.md](../ghidra/functions/psp-pulse/rigid-body.md#body0x160-is-angular-momentum-body0x40-is-the-inverse-inertia-tensor).

## The tensor's writer is now read, and the constant is retired

An earlier revision of this section ended "one thing this does not do is retire
the constant" - `Body_Init` leaves `body+0x40` as identity and the real writer
was unknown, so the *mechanism* was recovered and the *number* was not. **Both
are recovered now.** `Body_SetBoxInertia` (`0x0884e1ac`) builds a textbook solid
box, and its single call site is the ship-entity constructor with the literal
dimensions `(12, 8, 12)` and a mass of `0.9` set two calls earlier:

```text
I = m * (y^2 + z^2) / 12  ->  (15.6, 21.6, 15.6) on (right, up, forward)
```

against this page's fitted `~(15, 21.2, 15)`. `YAW_DRIVE_CALIBRATION` is gone;
`oag_physics::forces::YAW_INVERSE_INERTIA` is `12 / (0.9 * (12^2 + 12^2))` =
`0.046296`, against the fit's `0.0452`. Confidence **92**; evidence in
[rigid-body.md](../ghidra/functions/psp-pulse/rigid-body.md).

**This page's measurement did more than corroborate the read - it discriminated
the mass.** The constructor sets `0.9` and `Ship_UpdateCraft` overwrites the
body's mass from `Physical.mass` every frame afterwards, but nothing recomputes
the tensor. Building it with `m = 1.0` instead would give `I_yy = 24.0`, `13 %`
from what this page measured, against `1.9 %` for `0.9`. So the fit here is what
confirms the tensor is frozen at construction.

It also answers the third bullet above: the factor is **not** per-ship, and no
second-team capture is needed to decide it. The box is a code literal at a single
call site, so every craft in the game shares the tensor. `Misc`
`width`/`length`/`height` do reach the constructor - scaled by `0.75` - but they
go to the collider, not the inertia.

## Confirmed at racing speed - and the two attitude axes are not

Everything on this page was measured on captures topping out near `25` units/s.
A completed Time Trial lap at 90-164 units/s reproduces the yaw result on 2,845
wall-free intervals: `avel_y / omega_y` fits at `-20.948` against the recovered
`-21.6`, `3.0 %`, with `99.5 %` explained. The accumulator law this page
vindicates is confirmed at the same time and much more tightly - `steer *
Turning.amount` at `0.9998 +/- 0.0013` and the damping at `-5.0387 +/- 0.0097`,
once the ramp columns are paired with the frame that actually used them (they
lag by one tick, which is worth a factor of `1.68` if missed).

**The same fit refutes the relation on pitch and roll**, at `0.231x` and
`0.647x` of the tensor's own entries. `avel = -I * omega` is a kinematic
identity, so failing it means the basis carries rotation that `body+0x160` does
not account for - and the excess is `91 %` confined to the plane that tilts
`up`, i.e. it is attitude alignment, not a wrong tensor. This page's first
bullet ("local beats world") is unaffected; what is now open is whether
`body+0x160` is the *only* input to the basis advance. See
[cornering-ground-truth.md](cornering-ground-truth.md#pitch-and-roll-the-momentum-column-does-not-explain-the-rotation).

## The pitch axis, measured directly - and the tensor confirmed on all three

Everything above rests on captures that **never touched the pitch axis**: the
column's `x` and `z` entries were excited only incidentally, by the track pushing
the ship around, and this page's own warning was "do not read a diagonal inertia
tensor into the per-axis fits". Two scenarios close that gap, and they are the
first captures in this repository taken with a pitch input at all:

| scenario | what it is |
| --- | --- |
| `verification/scenarios/pitch-both-ways.inputs` | 360 ticks, stationary on the start line, **no thrust**: nose down held, released, nose up held, released |
| `verification/scenarios/pitch-hold-thrust.inputs` | 300 ticks, thrust and a held nose-up off the line, clean to tick 128 at 84 units/s |

`pitch-both-ways` is the cleanest capture in `data/traces/`: `speed/|velocity|`
reads **`1.0000` on 360 of 360 ticks**, `grounded` is `1.0` throughout, and the
stun timer never arms. Without thrust the craft never reaches a wall, so nothing
in it is contaminated by the contact response - which matters, because every
earlier capture's pitch and roll entries are measured through exactly that.

Fitted per axis against the rotation the recorded basis performs, and pooled
over both captures (658 ticks):

| axis | fitted `k` | `-I` from `Body_SetBoxInertia` | apart | explained |
| --- | ---: | ---: | ---: | ---: |
| pitch (right) | **`-15.620`** | `-15.6` | **`0.13 %`** | 94.4 % |
| yaw (up) | `-21.770` | `-21.6` | `0.8 %` | 93.1 % |
| roll (forward) | `-15.289` | `-15.6` | `2.0 %` | 94.0 % |

The pitch entry alone moves from "29 % explained, scattering from `-14` to
`-24`" to `94 %` explained on a signal that is now the *dominant* axis
(`2.12` rms against yaw's `0.11`), and it lands on the code literal to a
fraction of a percent. The clean prefix of the thrust capture reads
`-15.620` on pitch (94.7 %) and `-15.582` on roll (98.2 %).

One thing falls out for free: **these captures separate the frame far more
sharply than any earlier one.** Fitted as a world-frame quantity the column
explains `0.4 %` of itself, against `92.5 %` local - a factor of 200 rather than
the 3.6 the pooled straight-line captures gave. A ship that is pitching has body
axes far from the world ones on two axes at once, which is precisely the case
the frame question could not be decided on before.

### The input, measured rather than assumed

`Ship_UpdatePitch` reads the pitch axis at `*(craft+0x78) + 0x10`, and what
fills it was read at runtime by probing that field while cycling the pad:

- **`up` on the d-pad writes `-100`, and the nose goes DOWN.** `down` writes
  `+100` and the nose rises. So the axis is negative-nose-up, which is the
  opposite of `oag_physics::ShipControls::steer_y`'s documented "positive nose
  up".
- **The scale is `+/-100`, the same in-memory range the steering axis uses**
  (`oag_physics::controls::CONTROL_RANGE`), not a normalised `-1..1`. The
  analog stick reads `98.2`-`98.8` at full deflection, which is the PSP's own
  byte quantisation.
- **The analog stick's `y` feeds the same field with the same sign**:
  `stick_y=+1` reads `-100`, exactly like `up`. That settles
  [`oag-trace`'s open question](../tools/oag-trace.md#what-is-still-not-confirmed)
  about PPSSPP's analog polarity - it is positive-up, matching
  `InputSnapshot::stick_y` - as a side effect rather than by a separate test.

Confidence **90**: one binary, one emulator version, but the field is read
directly and all three inputs agree on it.

The response is a damped second-order one rather than a sustained rate - the
hover spring and the surface-alignment torque pull the attitude back within
about a second - so the fit is over a transient, an overshoot and a settle in
each direction, not over a plateau.

## `body+0x150` is recorded now, and it closes the chain

[cornering-ground-truth.md](cornering-ground-truth.md#what-to-do-next) asks for
one column above all others: `body+0x150`, which `Body_Integrate` advances the
basis by and which is therefore the angular velocity *by definition*. It is in
`scripts/psp_trace_fields.py` now, as `omega_x/y/z`, and both pitch captures
carry it. Two identities become directly measurable, with no force law and no
simulation in either:

| identity | pitch | yaw | roll |
| --- | ---: | ---: | ---: |
| `body+0x160 = k * body+0x150` | `15.601` (**100.0 %**) | `21.14` (86.2 %) | `15.689` (96.7 %) |
| `body+0x150 = k * omega(basis)` | `-1.0011` (94.5 %) | `-0.9999` (98.7 %) | `-0.9993` (97.4 %) |

on `pitch-both-ways`; the thrust capture reads `15.6055` / `-1.0011` on pitch
over its whole length. So, stated plainly and measured on all three axes:

```text
body+0x160 = I * body+0x150,   body+0x150 = -omega_physical
```

with `I` the code-literal `(15.6, 21.6, 15.6)`. The first row is the tensor,
recovered to `0.01 %` on the axis with the signal; the second is where the
`w_game = -w_physics` convention lives - **the negation is between `+0x150` and
the world, not between the two stored columns**, which no earlier capture could
have told apart. The residual on the second row is finite-difference noise from
the capture's seven significant digits, not a missing term.

Confidence **90** on both identities: two captures, three axes, one binary.

### What this does *not* do is overturn the racing-speed negative

The section above this one reports `avel / omega` fitting at `0.231x` and
`0.647x` of the tensor on pitch and roll over a completed lap, and attributes
the excess to attitude alignment slaved to a curving surface. **Nothing here
contradicts that, and the two together narrow it.** Broken out by speed band,
these captures hold the identity at every band they reach:

| band | `avel = k*omega`, pitch | `omega = k*basis`, pitch |
| --- | ---: | ---: |
| 0-20 u/s | `15.603` (100.0 %) | `-1.0006` (94.4 %) |
| 20-40 u/s | `15.629` (99.6 %) | `-0.9929` (91.2 %) |
| 40-60 u/s | `15.601` (100.0 %) | `-0.9975` (93.8 %) |
| 60-80 u/s | `15.605` (100.0 %) | `-1.0211` (95.3 %) |

So the excess is **not** a property of the mechanism, of the tensor, or of
speed alone up to `84` units/s - it is absent on a flat straight and present on
a lap of crests and banking, which is what that page's own tilt-plane signature
already said. The consequence for
[Task #18](../../HANDOVER.md) is the useful one: the momentum model **can** be
validated on pitch, on these captures, and the axis to validate it on is the one
where the identity is exact.

## The crate is on this model now, and what that measured

`oag-physics` used to route the angular accumulators as angular *accelerations*
and apply the recovered yaw entry as a single scalar on the summed yaw drive -
algebraically exact for that one axis, and nothing else. It now holds torque,
damps momentum, and carries the tensor on
[`oag_physics::Body::inertia`](../../crates/physics/src/ship.rs), so every angular
path is divided by `I`: the drives, the weathervane, the surface alignment and
the hover probes' lever arms. Three fitted or derived constants came out of the
crate in the process - `ALIGNMENT_INERTIA = 19.8`, the `<Misc>`-derived inertia
tensor, and the special-cased yaw scale.

**Validated on the pitch captures rather than asserted.** Both were replayed
through `oag-trace run --script` and compared axis by axis against the rotation
the original's own basis performs, so neither side's column conventions enter:

| stage | our pitch rate, rms | error rms | against a recorded `0.1358` |
| --- | ---: | ---: | ---: |
| before | `8.6821` | `8.6864` | 6397 % |
| accumulators as torque, tensor applied | `0.0021` | `0.1369` | 100.8 % |
| **and the pitch axis on its measured scale** | `0.2060` | `0.1823` | **134.2 %** |

The middle row is the momentum model with the pitch *input* still 100x weak, and
it is worth keeping: it is the crate reproducing none of the pitch response at
all, which is what a coherent half-change looks like. The bottom row is with
`engine::pitch` on the `+/-100` axis scale measured above.

**The drive magnitude is right to 1 %, and what is wrong is the return.** Over
the twelve ticks of the leading edge of each step - before the attitude starts
coming back - the recorded rate is `0.991` and `0.986` times ours on the two
captures, at an error rms of `14.9 %` and `14.2 %` of a signal of `0.32` rad/s.
Over the whole hold that degrades to `134 %`, and the time series says why: the
original's pitch rate **decays smoothly** to zero, ours **rings**. Two
independent captures, one stationary and one accelerating, agree on both halves.

So the pitch term itself is confirmed and the *restoring* path is not. The
obvious suspect is named rather than guessed at: `Ship_HoverTwoPoint`'s alignment
torque is read as having its right-axis component projected out, which leaves the
crate's pitch with no restoring torque except the probes' own differential spring
through a lever arm whose placement is itself recorded as "a guess awaiting M3".
Either that projection or that placement would explain an under-damped pitch, and
re-reading the projection is the cheaper of the two.

**Two other measurements from the same change, both reported rather than tuned
to:**

- The whole-lap comparison **improves**: seeded from the recording, the position
  error over 3,146 ticks falls from `1.076e4` to `4.045e3` and velocity from
  `3.496e2` to `2.788e2`.
- The whole-lap *scenario*, driven from the canonical spawn, **regresses**: it
  used to hold the track for all 3,146 ticks (worst off-spline `42.9`) and now
  leaves the surface at tick 742 over a crest and never returns. The mechanism is
  measured, not assumed - the craft is level (`4°` of bank) and climbing when it
  goes airborne at about 60 units/s, so this is not an instability. It is the
  weathervane's in-air authority dropping by `I_yy`, which is the recovered model
  doing what it says; what the old behaviour was masking is the
  contact-generation gap that lets our ship leave a crest the original never
  leaves. The lap script is an open-loop recording of what an autopilot pressed
  at the original, so it is sensitive to exactly this.

The yaw axis is unchanged, which is the check that the move was not a
half-application: `yaw_authority_ground_truth` is 2/2 under `just test-data`, and
`full_lock_out_yaws_the_bank_on_a_steeply_cambered_track` needed no edit at all -
it pins the ratio between two *body-local* yaw writes, and dividing both by
`I_yy` leaves it invariant.

## Reproducing it

```sh
python3 scripts/trace-angular-fit.py data/traces/talons-junction-*.csv --joint
```

Captures are derived game data and are not committed; see
[the capture page](../reverse-engineering/ppsspp-debugger.md) for how to take
them, and `verification/scenarios/` for the committed input scripts the four
scripted ones were driven by.
