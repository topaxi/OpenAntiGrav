# Cornering, measured against the original

**Status: the lateral axis is resolved and the yaw accumulator is closed
term by term. The pitch/roll refutation is
[scoped to one stretch of track](#the-refutation-is-scoped-the-inverted-section-was-carrying-it)
by a second lap that records `body+0x150`, and what is left of it is localised
rather than general. The inverse inertia's own frame was re-read on 2026-09-10
and is a **world**-axis diagonal; that answers the momentum-to-rate identity
exactly and the pitch/roll gap not at all, and the crate declines to follow it -
[the section below](#the-tensor-is-a-world-axis-diagonal-and-that-answers-a-not-c).**

[force-balance-ground-truth.md](force-balance-ground-truth.md) settled the
forward axis on straight-line captures and closed on a wall-friction
coefficient. It ends with "recapture a clean straight", because at the time no
trace in `data/traces/` held more than 60 consecutive wall-free ticks. The
autopilot's completed Time Trial lap holds **313**, at 90-164 units/s, with real
steering and real airbrake input throughout - so this page is the measurement
that capture was taken for, and it goes further than a straight could: on a
straight the ship's forward axis, its velocity direction and its speed columns
all coincide, and every lateral term is identically zero.

The one-line version: **`Ship_ApplyLateralGrip` is exactly right.** Its gain
comes out at `0.9985 +/- 0.0005` of the disc's own `Antigrav.grip_ground`, its
airbrake modulation `k = max(L, R) * (0.01 - slidegrip) - 1` is confirmed rather
than assumed, and the whole lateral projection is explained to `99.95 %` with an
rms of `3.28` units of force against a term that reaches `375`. The yaw
accumulator closes the same way, with all five of its writers present and the
three large ones inside `0.5 %` of the values read out of the binary.

Reproduce everything below with:

```sh
python3 scripts/trace-cornering.py \
    data/traces/talons-junction-time-trial-lap.csv \
    --normal-gravity <Physical normal_gravity> \
    --accelcap <Engine accelcap> --engine-amount <Engine amount> \
    --grip-ground <Antigrav grip_ground> --grip-air <Antigrav grip_air> \
    --slidegrip <Airbrake slidegrip> --airbrake-amount <Airbrake amount> \
    --airbrake-drag <Airbrake drag> --airbrake-turn <Airbrake turn> \
    --turning-amount <Turning amount> --turning-gain <Turning gain> \
    --turning-falloff <Turning falloff>
```

Every handling value is a command-line argument read off your own disc at
analysis time and is deliberately not written down here or in the script, per
[handling-stats.md](../formats/handling-stats.md) and
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md): a field name is
documentation, a tuning table is content. The measured runtime columns are the
result; the shipped numbers are only the ruler they are held against, and every
comparison below is reported as a **ratio to the disc's own value**.

## The capture, and the discipline applied to it

`data/traces/talons-junction-time-trial-lap.csv` - 3,146 ticks, Talon's Junction
White, Time Trial, Assegai Venom, captured by `scripts/psp-autopilot.py` at a
`Ship_UpdateCraft` breakpoint. Provenance and its known limits are in
[oag-trace.md](../tools/oag-trace.md); the one that matters here is that the
`dt` column is authoritative and the wall clock is not, because the capture runs
at about `0.37x` real time.

| | |
| --- | --- |
| ticks | 3,146 |
| `speed / \|velocity\| == 1.0000` | 2,996 (95.2 %) |
| worst deviation on a clean tick | `9.37e-07` |
| longest clean run of usable intervals | 313 |
| slip angle | 0 to `33.8` degrees, **`25.4` on clean ticks** |
| `throttle` | 100 on all but 3 ticks |
| `brake` | 0 on every tick |
| `stun_timer`, `timer_2e0` | 0 on every tick |
| `grounded` | 1.0 on all but 7 ticks |

**Every fit below runs only on ticks where `speed / |velocity|` reads exactly
`1.0000` at both ends of the interval.** That test is exact rather than
statistical - `Body_Integrate` writes `body+0x398` as `sqrt(dot(v, v))` from the
same register it stores as the velocity, so any gap means the contact response
touched `body+0x140` afterwards. Both ends are required because a contact
applied during frame `i` only shows up in tick `i + 1`'s ratio. The discipline
is not softened anywhere on this page; it is what made
[the M4 resolution](force-balance-ground-truth.md#resolved-the-force-law-was-right-and-both-reference-captures-were-in-sustained-contact)
possible and it is cheap.

`stun_timer` and `timer_2e0` reading zero on all 3,146 ticks also closes, by
columns rather than argument, the escape hatch that
[engine.md](../ghidra/functions/psp-pulse-usa/engine.md) keeps open for both
`Ship_UpdateEngine`'s and `Ship_ApplyLateralGrip`'s early returns: neither fires
anywhere in this lap, so the grip measurement below is of the ungated term.

## First, a methodological correction: the ramp columns lag by one tick

This is not a side note. Getting it wrong multiplies the fit's residual by 5.6
and buries every small term under the error.

`Ship_UpdateSteering` and `Ship_UpdateAirbrakes` **ramp** `craft+0x2c0`,
`craft+0x2c4` and `craft+0x2c8` and then use the ramped value inside the same
call. A capture samples the craft at the top of `Ship_UpdateCraft`, so tick
`i`'s columns are the *previous* frame's ramp output. The values frame `i`'s
force terms actually saw are the ones recorded at tick `i + 1`.

Measured, not assumed. The same five-term yaw fit, changing nothing but which
tick the ramp states are read from:

| Ramp state paired with the frame's acceleration | steering drive / `Turning.amount` | damping | rms |
| --- | ---: | ---: | ---: |
| tick `i` (the naive pairing) | `1.0323` | `-5.6144` | 16.99 |
| **tick `i + 1` (post-ramp)** | **`0.9998`** | **`-5.0387`** | **3.05** |

A 5.6x reduction in rms, and two coefficients that snap onto their read values.
Confidence **90**: the effect is far outside any noise in the fit, and the
mechanism is a plain reading of where the ramp sits relative to its use.

Anyone reusing a capture for a *rate* law should apply the same offset. The
un-ramped columns - velocity, position, the basis, the momentum - do not need
it, because they are read before anything in the frame modifies them.

## The named question: `speed`, `|velocity|` and `speed_cached` at a real slip angle

Three columns that a straight-line capture cannot tell apart. This lap reaches a
`33.8`-degree slip angle overall and **`25.4` degrees on the clean ticks the
fits use**, where `dot(v, forward)` falls `9.67 %` below `|velocity|`. That is
enough to separate all three; the largest slip angles in the capture are on
wall-contact ticks, which is what a ship scraping a barrier looks like and is
another reason the cleanliness test is not optional.

**`speed` is `|velocity|`, and the slip angle does not touch it.** On clean
ticks the ratio is `1.0000` to better than `1e-6`, and binning by slip angle
shows no drift at all:

| slip angle | n | mean `speed / \|v\|` | max deviation |
| --- | ---: | ---: | ---: |
| 0-4 deg | 683 | `1.00000000` | `9.4e-07` |
| 4-8 deg | 892 | `0.99999999` | `8.7e-07` |
| 8-12 deg | 858 | `1.00000000` | `8.4e-07` |
| 12-16 deg | 481 | `1.00000001` | `8.4e-07` |
| 16-20 deg | 78 | `0.99999995` | `7.1e-07` |

The deviation is the `%.7g` the capture writes, not a physical effect.
Confidence **92** - runtime, single binary, and it is a per-tick identity across
3,000 samples rather than a fit.

**`speed_cached` (`craft+0x2ec`) is `|dot(velocity, forward)|` of the previous
tick, and it is now distinguished from `|velocity|` by four orders of
magnitude.** Against all 3,145 intervals:

| Candidate reading | mean error | max error |
| --- | ---: | ---: |
| `\|dot(v, fwd)\|`, same tick | `0.715` | `34.65` |
| **`\|dot(v, fwd)\|`, tick before** | **`2.29e-05`** | **`1.05e-04`** |
| `\|velocity\|`, tick before | `1.304` | `14.87` |

`scripts/psp_trace_fields.py` records this reading as established on a
straight-line capture where "the two candidates are within each other's spread",
and asks for exactly this measurement. Confidence **92**, raised from 90: the
`vabs.s` at `0x08849930` and the one-tick lag are both now confirmed where they
are actually distinguishable.

The operational consequence is that **`speed_cached` is the right input to every
term that reads `craft+0x2ec`** - the quadratic drag and all three airbrake
terms - and that using `|velocity|` there would be a 16 % error mid-corner while
being invisible on a straight.

## Speed pads are a force, not a velocity impulse

The lap crosses five boost sections, and they are excluded from every fit. What
they are is worth recording, because they are the only place the **track-section
force** (step 15 of `Ship_UpdateCraft`, `0x08848f9c`) is visible in any capture:

| ticks | length | peak excess over the force law | `\|v\|` in -> out | `speed / \|v\|` throughout |
| --- | ---: | ---: | --- | --- |
| 161-182 | 22 | `+262.7` | `112.3 -> 157.6` | `1.000000` |
| 681-701 | 21 | `+230.4` | `112.1 -> 160.5` | `1.000000` |
| 1407-1427 | 21 | `+273.7` | `106.3 -> 149.0` | `1.000000` |
| 2517-2535 | 19 | `+249.5` | `111.4 -> 170.1` | `1.000000` |
| 2902-2923 | 22 | `+256.5` | `97.5 -> 153.8` | `1.000000` |

Five windows, all 19-22 ticks long, all ramping in and out rather than jumping,
and the contact-cleanliness ratio never moves off `1.0000` through any of them.
So the boost is applied in the **force accumulators**, not in velocity space -
which is exactly what distinguishes it from the contact response and, more
usefully, from the tick-8 kick in `talons-junction-venom-assegai.csv` that
force-balance-ground-truth.md attributes to `Ship_ApplySpeedupPad`. That one is a
single frame. **These are not the same mechanism**, whatever they are both
called in play. Confidence **80** on "sustained force, not impulse" (it is
directly measured); **50** on which function applies it, because nothing here
identifies the writer.

The magnitude is worth having as a target: peak `~250` units of force, three
times the engine's own `2 * min(...)` output at full throttle.

**The selection is by residual, which is circular, and the circularity is bounded
rather than hidden.** Ticks are flagged as pads when the forward residual exceeds
`+40`, which is a threshold chosen after looking at the data. Three things keep
it honest: the flagged ticks fall into five contiguous windows at five distinct
places on one lap rather than scattering, which is what a track feature looks
like and what a fitting artefact does not; `40` is above the largest term under
test (the airbrake drag term peaks at `31.5`), so no tick is excluded for having
a large *modelled* term; and it removes 105 of 2,947 intervals, so the fits below
would not move much either way. A capture that recorded the section index would
remove the judgement call entirely.

## The forward axis holds while cornering, and it finally measures the airbrake drag term

The forward projection is the same probe as force-balance-ground-truth.md's,
with two additions the straight captures did not need: `Body_Integrate`'s
`0.01` linear damping (about 1.2 units at these speeds), and the rolling
resistance written as `-2 * fs / |v|` rather than `-2`, which is what it is at a
slip angle.

```text
residual = dot(a, forward) + g * forward.y
         - 2 * min(100 * Engine.amount * 1e-3, 0.5 * fs + accelcap)
         + 0.005 * |fs| * fs  +  2 * fs / |v|  +  0.01 * fs
```

Over 2,842 usable intervals at `fs` from 0 to `157.4`, regressed on a constant,
`fs`, and the airbrake drag term:

| Regressor | Fitted | The reading predicts |
| --- | ---: | ---: |
| **`fs * \|L-R\| * Airbrake.drag * \|steerX\| * 1e-5`** | **`1.0743 +/- 0.0065`** | `1` |
| constant | `+2.92` | - |
| per unit of `fs` | `-0.0528` | - |
| | rms `3.15`, 95.6 % explained | |

Dropping the airbrake drag term alone takes the rms from `3.15` to `10.32`.

**This is the clean magnitude check `talons-junction-airbrake-asymmetric.csv`
could not give.** That capture was wall-contaminated for its whole useful length
and could only say "consistent within a factor of two, confidence ~55". Here the
term reaches `31.5` units of force, 1,997 of the 2,842 intervals have it active,
and it comes out **within 7.4 % of the value read from the two literals**, with
the residual tracking it across the whole range:

| predicted term | n | median residual | median fitted |
| ---: | ---: | ---: | ---: |
| 0-4 | 997 | `-1.81` | `-2.43` |
| 8-12 | 179 | `+7.74` | `+8.00` |
| 16-20 | 327 | `+17.04` | `+17.50` |
| 20-24 | 842 | `+21.05` | `+21.00` |
| 28+ | 33 | `+28.38` | `+27.29` |

Confidence **88** on the term's shape and magnitude, up from the ~55 the
airbrake-asymmetric capture supported. It is short of 92 for two reasons, both
stated so nobody reads the `7.4 %` as a recovered correction:

- **`steerX` is reconstructed, not recorded.** The term multiplies the *raw*
  input, and the trace records only the ramp output, so the script inverts the
  ramp (every step is `gain * dt` away from centre or `falloff * dt` back; the
  two differ by 1.85x here, and all 3,146 ticks invert unambiguously). The
  inversion is sound but it is an inference.
- **There is an unexplained baseline of a few units.** `+2.92 - 0.0528 * fs` is
  `-3.4` at `fs = 120`, about 4 % of the engine's full output. That is the same
  size as the 7.4 % discrepancy, so the two cannot be separated.

**And the forward residual does carry a slip-angle term**, which was tested for
rather than assumed away, because the lateral axis carries a force an order of
magnitude larger than anything unexplained here. Adding `|dot(v, right)|` as a
regressor:

| | fitted | rms |
| --- | ---: | ---: |
| without it | drag term `1.0743 +/- 0.0065` | 3.15 |
| **with it** | drag term `1.0282`, slip term `-0.2077 +/- 0.0064` | **2.69** |

`-0.21` per unit of lateral velocity reaches `-7.8` at the largest slip, which
is **1.8 % of the grip force acting on the right axis at the same instant** -
the size of a one-degree leak between the two projections, not of a physical
forward term. Two candidate sources, neither measured here: the hover downforce
acts along the **averaged surface normal** rather than the ship's own up axis,
so it does not cancel exactly the way the spring does; and the `%.7g` basis
columns bound the projection's own accuracy. **The honest statement about the
airbrake drag term is therefore `1.03` to `1.07` of the read value depending on
whether the leak is modelled**, and both are consistent with `1`.

**The engine law itself is unchanged and reconfirmed at a slip angle.** No new
resistance term appears, and nothing on the forward axis needs one.

## The lateral axis: `Ship_ApplyLateralGrip` is exactly right

This is the new territory, and it comes out cleaner than the forward axis.

The right-axis projection sees far fewer terms than the forward one. The hover
spring and the inline vertical damping act along the ship's own **up**, so they
cancel exactly by orthonormality; the engine, the rolling resistance's forward
share and the airbrake drag all act along **forward**. What is left is grip, the
airbrake's lateral term, and the right-axis shares of the quadratic drag,
rolling resistance and integrator damping - all three of which are known
exactly:

```text
residual = dot(a, right) + g * right.y
         + 0.005 * |fs| * v_lat  +  2 * v_lat / |v|  +  0.01 * v_lat
```

Regressed on the two terms the original writes to that axis, over 2,842
intervals with `|v_lat|` reaching `37.5`:

| Regressor | Fitted | Ratio to the disc's value |
| --- | ---: | ---: |
| **`dot(v, right) * k`**, `k = max(L,R) * (0.01 - slidegrip) - 1` | `9.9852 +/- 0.0046` | **`0.9985`** against `Antigrav.grip_ground` |
| **`fs * (R - L)`** | `0.000956 +/- 0.000009` | **`0.9565`** against `Airbrake.amount` as stored |
| constant | `+0.072` | - |
| | rms `3.28`, **99.95 % explained** | |

For scale: the grip term itself reaches `375` units of force. An rms of `3.28`
is `0.9 %` of that, and the same size as the forward axis's unexplained
baseline - i.e. the lateral fit is at the floor this method has, not at a limit
of the law.

**The `k` factor is confirmed by measurement, not merely by arithmetic.** Fitted
alone, `dot(v, right) * k` reaches rms `7.48`; dropping `k` and fitting plain
`dot(v, right)` reaches `18.49`. So the airbrake's modulation of grip is
*visible in the data* - the slip response really does soften when a brake is
applied, by the amount the `1e-4` load scaling predicts. engine.md derives
`slidegrip`'s meaning from the load-time factor at confidence 90; this is the
runtime leg.

Every alternative shape is far worse, fitted alone:

| Shape | Coefficient | rms | explained |
| --- | ---: | ---: | ---: |
| **`grip * v_lat * k`** | `9.7665` | **`7.48`** | 99.74 % |
| `grip * v_lat` (`k` ignored) | `-8.7070` | `18.49` | 98.42 % |
| `grip * v_lat * \|v\|` | `-0.0764` | `26.07` | 96.86 % |
| `grip * v_lat * \|v_lat\|` | `-0.3588` | `46.41` | 90.05 % |

**So the lateral force is linear in the lateral velocity and independent of
speed**, which is not what a tyre model or an aerodynamic sideforce would give,
and is what `Ship_ApplyLateralGrip`'s single `vdot` against `craft+0x170` says.
The residual holds flat across the whole slip range rather than bending at high
slip: median `|fitted - residual|` runs `1.48`, `1.53`, `1.59`, `1.82`, `2.21`
across `|v_lat|` bins of 0-4, 4-8, 8-12, 12-16 and 16+. **There is no saturation
and no grip limit.**

Confidence **92** on the whole lateral law - shape, gain and the `k` factor.
Runtime-verified, so capped at 94; short of that only because it is one team,
one class and one track, and because `grip_air` is untestable here (the capture
holds `grounded == 1.0` on all but 7 ticks, which is not enough to separate the
airborne coefficient).

Two consequences for `crates/physics`:

- `airbrake.rs`'s `lateral_grip` is correct as written, including the
  `(0.01 - slidegrip)` reading it flagged as an interpretation. Nothing to
  change.
- The `Airbrake.amount` lateral term at `0.9565` is 4.4 % low, the same order as
  the forward term's 7.4 % high, and with the same two candidate explanations
  (the unmodelled downforce leak, and `Airbrake.amount`'s own `1e-4` load
  scale). **Neither is grounds for adjusting a constant.** Confidence **85** on
  that term specifically, lower than the grip term because it is an order of
  magnitude smaller and shares the residual with everything unmodelled.

## The yaw accumulator, closed term by term

`Ship_ApplyAngularDamping` damps `body+0x160` - angular **momentum** - so every
writer of the `.y` lane lands in one accumulator with no inertia between them,
and the accumulator's own rate law is directly measurable from the recorded
column:

```text
d(avel_y)/dt = steer * Turning.amount              Ship_UpdateSteering
             - 5 * avel_y                          Ship_ApplyAngularDamping
             + fs * Airbrake.turn * (R - L) * 1e-3 Ship_UpdateAirbrakes
             - 0.1 * dot(v, right)                 Ship_ApplyWeathervaneTorque
             + 30 * right.y                        the hover epilogue
```

The weathervane's yaw component needs no approximation: `dot(cross(fwd, v), up)`
is `dot(v, right)` exactly, by orthonormality.

Over 2,845 intervals with `|steer|` reaching `108.4`:

| Model | drive / `Turning.amount` | damping | x airbrake yaw | x weathervane | x bank-to-yaw | rms | explained |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| steering + damping | `1.5626` | `-5.2808` | - | - | - | 36.90 | 91.89 % |
| + airbrake yaw | `0.9946` | `-5.0533` | `1.0179` | - | - | 4.34 | 99.89 % |
| **all five** | **`0.9998 +/- 0.0013`** | **`-5.0387 +/- 0.0097`** | **`1.0042 +/- 0.0016`** | `0.4012 +/- 0.1404` | `0.6014 +/- 0.0113` | **3.05** | **99.94 %** |

Three results, in descending order of how firmly the data holds them:

1. **`yaw = steer * Turning.amount` is exact.** `0.9998 +/- 0.0013` - two parts
   in ten thousand, on a drive that spans the full deflection range in both
   directions. `HandlingXml_ParseTurning` storing `amount` verbatim was read at
   confidence 95 from disassembly; this is the runtime leg, and it also
   **retires the last of the "22x" story at the accumulator level**: the
   accumulator obeys the documented law exactly, and everything that made the
   observed yaw rate differ from it lives in the inertia tensor, as
   [rigid-body.md](../ghidra/functions/psp-pulse-usa/rigid-body.md) concluded.
   Confidence **92**.
2. **The yaw damping is `-5`** - `-5.0387 +/- 0.0097`, against a `viim_s(5)`
   read from the instruction stream, and measured on the *momentum* rather than
   on an angular velocity, which is the distinction that retired the
   "drive and damping share an accumulator so the inertia cancels" argument.
   Confidence **92**.
3. **The airbrake yaw term is right, and it is not optional.** `1.0042 +/-
   0.0016` against `fs * Airbrake.turn * (R - L) * 1e-3`, and leaving it out
   inflates the fitted steering drive by 56 % and the rms eightfold. Confidence
   **90**; this is the first runtime confirmation of any part of
   `Ship_UpdateAirbrakes`' angular write.

The two small terms are a genuinely weaker result and are reported as such:

- **The weathervane comes out at `0.40 +/- 0.14` of its read `-0.1`.** That is
  four standard errors below 1, but the term's own rms contribution is `1.68`
  against a signal of `129.6`, so the capture barely constrains it; pinning both
  small terms at their read values only moves the fit's rms from `3.05` to
  `3.66`. Recorded as **open**, confidence **50** on the coefficient being
  genuinely smaller than `0.1`. One unexcluded possibility: the weathervane
  writes the *world* angular accumulator, and the sign/scale of the world path
  into `body+0x160` has never been read, only inferred.
- **The bank-to-yaw term comes out at `0.60 +/- 0.011`.** It has an escape the
  weathervane does not: engine.md reads it as `30 * right.y * (1 - magLockBlend)`
  and `craft+0x280` is not in the capture, so a non-zero magstrip blend would
  produce exactly this kind of uniform under-read. Confidence **45** either way
  until a capture carries `craft+0x280`.

**The relative sizes are the actionable part**, because
[the deferred angular work](../../HANDOVER.md) names the crate's weathervane yaw
authority as ~21.6x too strong relative to steering. On this capture the
steering drive and the airbrake yaw term are pinned to under `0.5 %`, so any
crate discrepancy on those two is a crate bug, not a missing measurement.

## Pitch and roll: the momentum column does not explain the rotation

The cheap check asked for was the yaw law at real cornering speeds. Running it
on all three axes turns up a negative that matters more.

`body+0x160` is body-frame angular momentum and the inertia tensor is the
code-literal box `(12, 8, 12)` at mass `0.9`, giving `I = (15.6, 21.6, 15.6)` on
(right, up, forward). That is a kinematic identity, independent of every force:
the rotation the recorded basis performs must satisfy `avel = -I * omega`.
Recovering `omega` from `M[i]^T M[i+1]`, over 2,845 clean intervals:

| axis | fitted `avel / omega` | `-I` | ratio | % explained | rms |
| --- | ---: | ---: | ---: | ---: | ---: |
| pitch (right) | `-3.611` | `-15.6` | `0.231` | 16.1 | 2.97 |
| **yaw (up)** | **`-20.948`** | `-21.6` | **`0.970`** | **99.5** | 2.34 |
| roll (forward) | `-10.100` | `-15.6` | `0.647` | 62.5 | 2.19 |

**The yaw entry is confirmed to 3 %** at 90-164 units/s - the first check of
`I_yy` at racing speed rather than from the 25 u/s steer captures, and it agrees
with `Body_SetBoxInertia`'s `21.6` well inside the fit's own noise. Confidence
**90**.

**Pitch and roll are refuted, and the direction of the failure is the
informative part.** Both fitted magnitudes are *below* `I`, which means the basis
rotates **more** than the momentum accounts for. Four things pin down what the
excess is:

1. It is not a frame or timing artefact. Pairing `omega` with the momentum at
   tick `i`, at `i + 1`, or with their mean changes the ratios by under 2 %.
2. It is not a wrong change of basis - but the reason given here first time
   round was wrong, and the correction is
   [its own section below](#the-tensor-is-a-world-axis-diagonal-and-that-answers-a-not-c).
   Four candidate maps were tried - body-diagonal, world-diagonal,
   `M^T I^-1 M` and `M I^-1 M^T`, each against both the body- and world-frame
   rotation - and this page recorded the body-frame diagonal as the best of
   them. **It is not**: the engine's own map is a diagonal fixed in *world*
   axes, `L = R (I (x) R^T w)`, and it explains the momentum column to
   `100.00 %` on three captures. What survives, and is remeasured below, is the
   narrower statement - **no reading of the tensor's frame rescues pitch or
   roll**, because the failure is not in the tensor at all.
3. It is not noise. The residual's lag-1 autocorrelation is `0.978` - it is a
   smooth, sustained rotation, not per-frame jitter, and the recorded basis is
   orthonormal to `1.1e-06`.
4. **It is a pure tilt.** `91.0 %` of the residual rotation's power lies in the
   plane that tilts `up`; the residual's yaw component is `0.118` rad/s rms
   against a measured yaw of `1.550`. A rotation that aligns `up` onto a surface
   normal has no component about `up`; a rotation driven by the momentum column
   has no reason to avoid one. Its magnitude also scales with speed - median
   `0.006` rad/s below 40 units/s, `0.07` at 140 - which is what a craft whose
   attitude is slaved to a curving surface does.

**So something drives the ship's pitch and roll onto the track outside the
angular-momentum path.** Confidence **85** on the negative (that
`avel = -I * omega` fails on pitch and roll), **70** on the attribution to
attitude alignment, which rests on the tilt-plane and speed-scaling signatures
rather than on any instruction.

This does **not** retract the tensor: `Body_SetBoxInertia` is read at confidence
92 and the yaw entry is confirmed here. What it says is that `body+0x160` is not
the whole story for the attitude axes, and that
[Task #18](../../HANDOVER.md)'s "apply the full angular momentum model
coherently" cannot be validated on pitch and roll from any existing capture. The
capture that would settle it in one run is described under
[what to do next](#what-to-do-next).

## The refutation is scoped: the inverted section was carrying it

That capture was taken. `data/traces/talons-junction-time-trial-lap-omega.csv`
is the same scenario re-flown by `scripts/psp-autopilot.py` with `body+0x150`
in the column set - 3,140 ticks, 2,969 clean intervals (94.6 %), 0 to 157
units/s, the game's own HUD reading `Lap 2 of 3` afterwards. The first thing it
does is **reproduce the table above**, which is what makes the rest of it
comparable: `0.2313` / `0.9692` / `0.6543` against the old lap's `0.231` /
`0.970` / `0.647`, on a different run of the same circuit.

With `body+0x150` recorded, that one composite splits into two identities that
can be measured separately, and they answer the question this page left open:

| | identity | what a failure means |
| --- | --- | --- |
| (A) | `body+0x160 = I * body+0x150` | a second writer of the momentum column |
| (B) | `body+0x150 = -omega(basis)` | the basis is rotated outside the integrator |
| (C) | `body+0x160 = -I * omega(basis)` | (A) times (B) - all the old lap could see |

Pooled over the whole lap, **(A) roughly holds and (B) is what breaks**: pitch
`1.192` / yaw `0.970` / roll `0.931` on (A), against pitch `0.200` / yaw
`0.9994` / roll `0.642` on (B). So the excess rotation is **not** in the angular
velocity column - it is between that column and the basis.

**But the pooled number is the wrong instrument, and this is the correction.**
Partitioning by `up.y` - a recorded column, independent of everything being
fitted, so this is not "sort by residual and find the residual" - the failure is
not spread across the lap at all:

| `up.y` | n | (A) pitch, yaw, roll | (B) pitch, yaw, roll |
| ---: | ---: | --- | --- |
| 0.99-1.01 | 1,879 | `1.036` `0.998` `0.991` | `0.892` `1.000` `1.005` |
| 0.97-0.99 | 436 | `1.077` `0.989` `0.914` | `0.812` `0.999` `0.980` |
| 0.93-0.97 | 202 | `1.237` `0.978` `0.951` | `0.706` `1.001` `0.869` |
| 0.85-0.93 | 176 | `1.551` `0.965` `1.065` | `0.687` `0.998` `0.767` |
| below 0.85 | 119 | `0.291` `0.804` `0.555` | `0.032` `0.999` `0.034` |

**Talon's Junction has an inverted section**, and the lap goes through it: ticks
1,072-1,329 hold `up.y` down to `-0.99994` while `pos_y` climbs. Drop the
intervals below `up.y = 0.85` - 276 of 2,969, under a tenth of the lap - and the
composite identity this page reported as *refuted* reads

| axis | (C) `avel / -I*omega(basis)` | % explained |
| --- | ---: | ---: |
| pitch (right) | `0.926` | 66.4 |
| yaw (up) | `0.992` | 99.9 |
| roll (forward) | `0.918` | 91.2 |

- within `8 %` on both attitude axes, against `0.231` and `0.647` pooled. A
least-squares fit weights by amplitude, and the inverted stretch rotates at
`1.21` rad/s of tilt against `0.29` on the rest of the lap, so a tenth of the
samples carried the whole result.

**Inside that stretch the basis rotation is essentially unrecorded**: (B) reads
`0.023` on pitch and `0.110` on roll while yaw still reads `0.998`, and the
residual `omega(basis) + body+0x150` is `1.186` rad/s rms against a measured
tilt of `1.209` - **99.9 % of the tilt rotation there passes the angular
velocity column by**, with the yaw component untouched.

Three alternative explanations were tested and refuted rather than argued away:

1. **A frame confusion.** Fitting `body+0x150` against the *world*-frame
   rotation instead of the body-frame one explains `0.1 %` on pitch and `0.9 %`
   on roll, against `21 %` and `62 %` for body-frame - the columns are
   body-local, including through the inverted section.
2. **A one-tick pairing error.** `omega` at tick `i`, at `i + 1`, or their mean
   move the (B) pitch ratio by `0.198` / `0.197` / `0.198` - under 1 %.
3. **A restoring spring.** If the extra rotation were proportional to the
   misalignment `cross(up, normal)` it would explain the residual; it explains
   `1.9 %`. The ship's `up` sits a median `1.6` degrees off the track's own
   surface normal (max `9.6`), so there is barely any misalignment to restore.

What does fit is **slaving**: over the whole lap the ship's `up` tilts at
`0.906x` the rate the *track's own surface normal* turns as the craft drives
along it (72.1 % explained), and the unaccounted residual is `0.713x` that same
rate (70.2 %). Outside the inverted section the residual falls to `0.206x`
(20.9 %) - i.e. on ordinary track most of the surface-following rotation *does*
go through the momentum column, as a hover torque should, and only the inverted
stretch is driven around it.

**So the answer to this page's open question is (B), localised**: something
rotates the basis directly, outside `Body_Integrate`, and on this circuit it is
concentrated where the craft is held inverted.

**And the instructions that do it have since been read.**
`Ship_UpdateMagLock` (`0x0884ba0c`) ends in `sv.q` writes straight into the
three basis rows at `body+0x00`, `+0x10` and `+0x20` - each row projected onto a
track-derived axis, scaled by the mag-lock blend at `craft+0x280`, followed by an
explicit Gram-Schmidt re-orthonormalisation. It is not a torque, it touches no
accumulator, and it therefore **cannot** appear in the momentum column: exactly
the shape of the (B) break measured above, in exactly the place a mag-strip
holds a craft inverted. Evidence and listing in
[engine.md](../ghidra/functions/psp-pulse-usa/engine.md#ship_updatemaglock-rewrites-the-basis-directly-and-that-is-the-missing-mechanism);
the attribution goes from 45 to **90**, measurement and instruction reading
arrived at independently and agreeing.

Confidence **88** on the split ((A) holds, (B) breaks), **85** on the scoping to
the inverted section, **90** on the mechanism being `Ship_UpdateMagLock`.

The implementation consequence, since it is the thing most likely to be got
wrong: **it must not be modelled as a torque.** A torque would go through
`I^-1` and appear in `body+0x150`, and this lap measures that it does not - the
rotation is a kinematic rewrite of the basis. That is queued as a separate
task rather than done here.

### The hold, now measured as a trajectory (oag-trace locator)

Moved from the `oag_physics::maglock` module docs. `oag-trace`'s `replay`/`drive`/`drive_with`
and `plan::to_gate` take an `Option<&[oag_vex::track::Sample]>` and locate fresh every tick
the way `oag_raceplay::Race::tick` locates the player (`crate::replay::locate` duplicates the
resampling rather than sharing it). `crates/trace/src/replay/tests.rs` pins the mechanism on a
synthetic magstrip fixture, and a freshly flown and captured Talon's Junction lap
(`verification/scenarios/talons-junction-inverted-section.inputs`) measured it for real:
reseeded near the inverted section, position error against the capture is roughly **half**
with the locator attached versus without, and **298 of 3,020** probe raycasts over the lap hit
`Surface::MagFloor` rather than `Floor`. **Resample density is not the residual's source**:
1 to 4 samples per segment measurably improves tracking and 4 to 32 changes nothing outside
noise. `crates/game/tests/maglock_ground_truth.rs` still carries the 49.5 % residual figure;
the two are complementary (that one measures the hold's contribution to a rotation identity,
this one its effect on a trajectory).

## The tensor is a world-axis diagonal, and that answers (A), not (C)

2026-09-10, and it is the correction to bullet 2 above. `Body_Integrate` maps
`body+0x160` to `body+0x150` through `R (body+0x40) R^T`, and **both fields hold
their quantity negated and in body coordinates**, so the two rotations are only
the trip in and out of the frame the fields are written in. What the map itself
does is multiply a *world* vector by a diagonal fixed in *world* axes, written
once by `Body_SetBoxInertia`. The derivation, pinned to the instruction each
frame is read off, is
[rigid-body.md](../ghidra/functions/psp-pulse-usa/rigid-body.md);
`scripts/trace-inertia-frame-fit.py` scores it against the body-local and mixed
readings and it explains `100.00 %` of the recorded `+0x160` column on all three
Talon's Junction captures, its free three-parameter fit returning
`(15.600, 21.600, 15.600)` - `Body_SetBoxInertia`'s own code literal, out of a
column it had never been compared against.

`diag(a, b, a)` is invariant under yaw, so on a level craft the world-axis and
body-axis readings are **identical**. They can only differ where the craft is
banked or pitched - which is why the labels survived being the wrong way round
for three weeks, and why the question this section exists to answer had to be
measured rather than argued.

**The question: does the world-axis map close the `0.231x`/`0.647x` pitch and
roll gap?** `scripts/trace-omega-identity.py --world-axis` runs both identities
under both readings on the same intervals. On `talons-junction-clean-lap.csv`,
2,837 clean intervals of 2,977 ticks (the capture that reproduces this page's
tables - see the note under the reproduce command below):

| identity | reading | pitch | yaw | roll |
| --- | --- | ---: | ---: | ---: |
| (A) `avel = I * omega` | body-axis | `1.1667` (86.8 %) | `0.9693` (99.5 %) | `0.9304` (86.7 %) |
| **(A_F)** `avel = R (I (x) R^T omega)` | **world-axis** | **`1.0000` (100.00 %)** | **`1.0000` (100.00 %)** | **`1.0000` (100.00 %)** |
| (B) `omega = -omega(basis)` | no tensor in it | `0.2134` (21.9 %) | `0.9992` (99.9 %) | `0.6600` (62.4 %) |
| (C) `avel = -I * omega(basis)` | body-axis | `0.2533` (19.7 %) | `0.9685` (99.5 %) | `0.6643` (63.2 %) |
| (C_F) the same composite | world-axis | `0.2963` (30.6 %) | `0.9976` (99.9 %) | `0.6314` (63.7 %) |

**No.** (A_F) is exact - ratio `1.0000` on every axis, rms `1e-4` against a
column of order 3 - so the world-axis reading is *the* answer to (A), and (A)
was never the broken half. (C) is (A) times (B), and (B) is a fit of one
recorded column against another **with no tensor anywhere in it**: no reading of
the tensor's frame can move it. The composite duly does not close - pitch goes
`0.2533` to `0.2963`, roll `0.6643` to `0.6314`, residual rms `3.28` to `3.05`
and `2.21` to `2.20`. Restricted to `up.y >= 0.85`, where this page already
scoped the failure away, it is if anything slightly worse on both attitude axes:
`0.7706`/`0.9285` body-axis against `0.7564`/`0.8918` world-axis, with yaw
tightening from `0.9912` to `0.9992`.

Reproduce with `--world-axis`, which is additive and leaves the default output
untouched:

```sh
uv run scripts/trace-omega-identity.py \
    data/traces/talons-junction-clean-lap.csv --world-axis
uv run scripts/trace-omega-identity.py \
    data/traces/talons-junction-clean-lap.csv --world-axis --min-up-y 0.85
```

### The crate keeps its body-space diagonal: chosen, not measured

`crates/physics/src/integrate.rs` applies the inertia as a body-space diagonal,
`R^T (I^-1 (x) (R tau))`, and **that stays**, knowingly diverging from the
original. No confidence score attaches to this: it is a design decision, not a
measurement, and the measurement it is decided against is the exact one above.

Three things decide it, in order:

1. **The payoff is absent.** The change was proposed to close the pitch and roll
   gap. The gap is (B)'s, and (B) has no tensor in it.
2. **The divergence that is left is real, and it is carried knowingly rather
   than argued away.** Where the original holds a craft off level on a magstrip
   it drives the attitude outside the momentum path entirely - the section
   above measures `99.9 %` of the inverted stretch's tilt rotation bypassing
   the angular-velocity column - and `oag_physics::maglock` reproduces that
   rewrite kinematically, so both engines agree there for the same reason. But
   the `up.y` table above also shows (B) roll at `1.005`, `0.980`, `0.869` down
   to `up.y = 0.93` and the residual falling to `0.206x` outside the inverted
   section: **at mild bank off a magstrip the original really is turning a
   world-axis tensor through this path, and the crate is not.** What would
   settle whether it is worth having is a capture with sustained bank and
   `craft+0x280 == 0`, compared tick for tick against the crate; nothing in
   `data/traces/` provides one, and `craft+0x280` is not in any capture's
   column set either.
3. **It is not free, and it is not one site.** The tensor is applied in four
   places - `wall::inverse_inertia` (the contact denominator, already
   world-axis and literal), `wall::body_frame_inverse_inertia` (the angular
   impulse response, rotated), `integrate` (rotated) and
   `forces::YAW_INVERSE_INERTIA` (the yaw drive's folded factor). Adopting the
   world-axis reading means moving all four together or leaving one crate
   applying one tensor in two frames, which is the trap `integrate.rs`'s own
   sign-convention note already exists for. Measured on a throwaway branch with
   `integrate`'s two rotations deleted (a constant world-axis diagonal makes
   `R (I^-1 (x) R^T tau)` equal `I^-1 (x) tau` outright): `crates/physics`'s
   `the_simulation_matches_the_committed_reference` moves on **all three
   scripts, including `Corridor` at 600 ticks** - so it is not confined to
   aerobatics, it reaches every tick the craft is not perfectly level.
   `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
   survives it - twelve of twelve clean either way, `01_Track` at 1 respawn at
   `[794]` either way - with lap times moving by at most `0.3 s` (`16_Track`
   `40.9 -> 41.2`, `01_Track` `38.8 -> 39.1`, `07_Track` `48.4 -> 48.6`). A
   reference hash that moves across four sites for a change with no measured
   payoff is the churn the golden-hash rule exists to make expensive.

Two things this decision is **not**. It is not a claim that the crate's
treatment is correct and the original's a bug: ours is a body-space diagonal
evaluated once per frame from the starting orientation and carrying no
gyroscopic `omega x (I omega)` term, which is an approximation of its own. And
it is not permanent. **Revisit it the next time all four application sites are
open anyway**, or as soon as a sustained-bank capture off a magstrip exists to
measure it against - the case for adopting it is fidelity in that one regime,
and today nothing can measure whether the crate is worse there.

**One thing the correction does settle, at no cost.**
`crates/physics/src/wall.rs` recorded its contact denominator - `body+0x40`
applied straight to a world-space lever arm - as an *asymmetry*, a place where
"the original skips the rotation here and only here", justified by `+0x40`
being the body-space tensor. It is not an asymmetry: a world-axis diagonal
applied to a world vector is the engine's one convention, and that site was
already right for a reason nobody had. Its doc comment now says so. The
asymmetry that remains is between that site and `integrate`, and it is
this page's decision rather than the disc's behaviour.

Reproduce all of it with, and note that no handling values are needed - every
fit here is kinematic:

```sh
cargo run -q -p oag-trace -- track --source data/images/pulse-psp-usa.chd > /tmp/spline.csv
uv run scripts/trace-omega-identity.py \
    data/traces/talons-junction-clean-lap.csv --spline /tmp/spline.csv
uv run scripts/trace-omega-identity.py \
    data/traces/talons-junction-clean-lap.csv --spline /tmp/spline.csv \
    --min-up-y 0.85          # the same lap without the inverted section
```

**The capture names above drifted, and were corrected on 2026-09-10.** The lap
these sections' tables come from is `talons-junction-clean-lap.csv` today -
2,977 ticks, 2,837 clean intervals (95.3 %), (C) reading `0.2533` / `0.9685` /
`0.6643` against the `0.231` / `0.970` / `0.647` recorded above.
`talons-junction-time-trial-lap-omega.csv` no longer exists under that name, and
the `talons-junction-time-trial-lap.csv` in `data/traces/` today is a different
capture from the one the
[lateral and yaw fits](#the-capture-and-the-discipline-applied-to-it) were taken
on: it carries the `omega_*` columns those fits predate, and reads 696 clean
intervals of 3,146 ticks (22.1 %) with (C) at `1.35` / `0.99` / `1.01`. The
provenance was not chased; read the earlier sections' reproduce command as
naming a capture that is no longer in the tree under that name.

## What this page confirms, refutes and leaves open

| Claim | Verdict | Confidence |
| --- | --- | ---: |
| `speed` (`body+0x398`) is `\|velocity\|`, unaffected by slip angle | confirmed to `1e-6` over 3,000 ticks and 25 degrees of slip | 92 |
| `speed_cached` is `\|dot(v, fwd)\|` of the previous tick | confirmed, now separable from `\|velocity\|` by 4 orders of magnitude | 92 |
| `Ship_ApplyLateralGrip` is `grip_ground * dot(v, right) * k` | confirmed at `0.9985x` the disc's gain, 99.95 % explained | 92 |
| `k = max(L, R) * (0.01 - slidegrip) - 1` | confirmed by measurement, not only by the load-time scale | 92 |
| the lateral force saturates, or scales with speed | **refuted**: linear in `v_lat`, flat to `\|v_lat\| = 37.5` | 90 |
| `yaw = steer * Turning.amount`, verbatim | confirmed at `0.9998 +/- 0.0013` | 92 |
| yaw angular damping is `-5`, on the momentum | confirmed at `-5.0387 +/- 0.0097` | 92 |
| airbrake yaw `fs * turn * (R-L) * 1e-3` | confirmed at `1.0042 +/- 0.0016` | 90 |
| airbrake forward drag `fs * \|L-R\| * drag * \|steerX\| * 1e-5` | confirmed at `1.03`-`1.07`, up from a ~55 magnitude check | 88 |
| airbrake lateral `fs * Airbrake.amount * (R-L)` | confirmed at `0.9565 +/- 0.009` | 85 |
| `I_yy = 21.6` from `Body_SetBoxInertia` | confirmed at racing speed, `0.970x` | 90 |
| `avel = -I * omega` on pitch and roll, pooled over a whole lap | **refuted**, `0.231x` and `0.647x` - reproduced on a second lap at `0.231x` / `0.654x` | 88 |
| ...but the refutation is general rather than localised | **refuted** by the `body+0x150` lap: outside the inverted section it reads `0.926x` and `0.918x` | 85 |
| the break is between `body+0x150` and the basis, not in the momentum column | confirmed: (A) `1.19`/`0.97`/`0.93`, (B) `0.20`/`0.9994`/`0.64`, and (A) is *exact* under the engine's own world-axis map | 92 |
| the inverse inertia is a body-space diagonal | **refuted**: it is a diagonal fixed in **world** axes, `100.00 %` of the momentum column on three captures | 92 |
| a world-axis tensor closes the pitch and roll gap | **refuted**: (C) `0.2533`/`0.6643` body-axis against `0.2963`/`0.6314` world-axis - the gap is (B)'s, and (B) has no tensor in it | 92 |
| the crate should adopt the world-axis tensor | **declined - chosen, not measured**, see [above](#the-crate-keeps-its-body-space-diagonal-chosen-not-measured) | - |
| the excess pitch/roll rotation is attitude alignment | open, and now measured against the track's own normal: `0.713x` its turn rate, 70 % explained; a misalignment spring explains 1.9 % | 75 |
| the excess is a wrong frame, a pairing error, or noise | **refuted**: world-frame explains 0.1-0.9 %, pairing moves it under 1 % | 90 |
| the weathervane coefficient is `-0.1` | open, fits at `0.40 +/- 0.14`, term too small to pin | 50 |
| the bank-to-yaw coefficient is `30` | open, fits at `0.60`, `magLockBlend` not captured | 45 |
| speed pads are a sustained force, not an impulse | confirmed, 5 windows of 19-22 ticks at ratio `1.0000` | 80 |
| the ramp columns lag the frame that used them by one tick | confirmed, 5.6x rms improvement | 90 |
| the forward force law needs a new term while cornering | **refuted**: residual rms `3.15`, under 4 % of full thrust | 88 |

## What to do next

1. ~~**Capture `body+0x150`.**~~ **Done** - see
   [the scoping above](#the-refutation-is-scoped-the-inverted-section-was-carrying-it).
   `craft+0x280` is still not captured, and it remains the only escape the
   bank-to-yaw term has; it is a single entry in
   `scripts/psp_trace_fields.py`'s `CRAFT_FIELDS`.
   The follow-on the `omega_*` lap opened - find what writes the basis outside
   `Body_Integrate` - is **also done**: it is `Ship_UpdateMagLock`, read at
   instruction level, see the section above. `craft+0x280` is that term's own
   blend, so capturing it would now serve two questions rather than one.
2. **Capture an airborne stretch.** `grip_air` and the airborne branches of the
   quadratic drag and vertical damping are all untested by every capture in
   `data/traces/` - this lap has 7 half-grounded ticks and no fully airborne
   ones. A jump on a track with one would give the whole airborne column at
   once.
3. **Do not adjust `Airbrake.amount` or `Airbrake.drag` to close the 4-7 %.**
   Both are read from the instruction stream, both sit inside the residual this
   method has, and the residual has an unattributed source (the downforce's
   surface-normal direction) that has never been measured. The measurement is
   the result; the constant that would hide it is not. Same rule as
   [force-balance-ground-truth.md](force-balance-ground-truth.md#what-not-to-do).
4. **Second team and second speed class.** Everything here is Assegai Venom on
   one track. `grip_ground` differs across teams and `grip_air` across classes,
   so a Rapier or Phantom capture would turn the ratio checks above into a
   two-point test of whether the *parameter* is what varies, rather than the law.
