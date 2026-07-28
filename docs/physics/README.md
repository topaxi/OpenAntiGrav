# Physics

**Status: model recovered from static analysis, and now partly verified against
captures of the original.** The structure below is implemented in
[`oag-physics`](../../crates/physics/src/lib.rs).

The shape of the ship dynamics is known, and every constant it needs lives in
[handling stats](../formats/handling-stats.md). **"None of it is
runtime-verified" stood here until captures existed and is no longer true**: the
engine's thrust law, the two passive resistances, the lateral grip law, the
airbrake terms and the yaw accumulator have all been measured against the
original and are marked in place below. The rest of this page is still a
specification to test against rather than truth, and what exists in code is a
transcription of it, deliberately including the parts that look wrong. See [what is implemented](#what-is-implemented) for the split,
[the cross-product signs](#the-cross-product-signs-a-contradiction-here-resolved)
for the contradiction on this page that arithmetic could settle, and
[still open](#still-open) for the readings the evidence leaves undetermined.

**One part of this model is measured against the original rather than read.**
[The along-track force balance](force-balance-ground-truth.md) reconstructs, from
captured traces, what force the original actually applies along a craft's forward
axis. **Its conclusion has since inverted, and this paragraph used to carry the
retracted version** ("the crate applies about 53 units of net force too much, in
a term linear in speed"): the force law was right all along, and the two
reference captures were recorded scraping a wall, losing `3.5 %` of their
tangential velocity per frame in a post-integrate contact pass no force
accumulator can see. That page is still the authority on the longitudinal
balance and on which candidate terms are eliminated; read its resolution
section, not its investigation, for the current state.

**A wall-free cornering capture now measures the rest of the law.**
[Cornering, measured against the original](cornering-ground-truth.md) projects a
completed Time Trial lap onto the ship's forward *and* right axes: it reconfirms
the forward law at slip angles up to 25 degrees, settles `Ship_ApplyLateralGrip`
at `0.9985` of the disc's `grip_ground`, closes the yaw accumulator term by term
(`steer * Turning.amount` at `0.9998`, damping at `-5.04`), and **refutes**
`avel = -I * omega` on the pitch and roll axes while confirming it on yaw. It is
the authority on the lateral axis and on the airbrake terms' magnitudes.

**A second part is measured rather than read**, and it is the rotational half of
the same story. [What `body+0x160` holds](angular-velocity-column.md) settles the
sign and the frame of the recorded angular-velocity column against five captures'
own basis derivatives, and finds the yaw axis carrying a factor of `21.2`,
arrived at with no simulation and no fit against our own physics. The tensor's
writer has since been read - a hard-coded solid box, see
[rigid-body.md](../ghidra/functions/psp-pulse/rigid-body.md) - and it gives
`21.6`, so `oag_physics::forces::YAW_INVERSE_INERTIA` is now a recovered value
rather than the fitted `YAW_DRIVE_CALIBRATION` that stood there.

**This page no longer covers the whole force law.**
[Engine, brakes, steering and pitch](../ghidra/functions/psp-pulse/engine.md) was read
out of `BOOT.BIN` afterwards, and it is the authority wherever the two disagree: it was
read from the XML loader and the craft update path directly, with addresses and a
per-component enumeration of every accumulator write. It **corrected five things on this
page**, all of them now marked in place: `ride_height` does reach the force law, the
`slidegrip` pseudocode was right once its load-time scaling is known, gravity has no
track-relative component, groundedness is one frame stale for every control term rather
than only for the hover spring's load factor, and the roll negative needed narrowing from
"no Z component is ever written" to "no *control input* writes one". It also closed this
page's "no angular damping" gap, and it introduced a second instance of the
[cross-product sign problem](#the-cross-product-signs-a-contradiction-here-resolved).

## The two answers that were blocking M4

### It is floating point, not fixed point

Pure IEEE single precision throughout, scalar FPU plus VFPU. The only integer
operations in the craft path are pointer arithmetic, loop counters and flag
masks. Confidence **92**: an exhaustive negative over the whole craft path,
every call site consistent, which is exactly the shape of claim decompilation
is good at settling. Not checked against shipped data and never run under an
emulator, so per the [rubric](../reverse-engineering/confidence-rubric.md) it
stays below 95 like the rest of this page; the earlier 97 predated the rubric
saying explicitly that nothing here has earned the top band.

So [ADR-0002](../architecture/adr/0002-determinism-model.md)'s fallback of
mirroring a fixed-point format exactly does not apply. Bit-exactness is not
available for free, and the tolerance-based approach stands.

### The integrator does not sub-step at 1/60

This was the open question from [frame pacing](../psp/frame-pacing.md), and the
answer is the less convenient one:

```
dt = clamp(measured_dt, 0, 0.06666)
evaluate all forces once at full dt
integrate 3 explicit Euler sub-steps of dt/3
```

The sub-step *count* is fixed at three; the sub-step *size* is variable. There
is no 1/60 anywhere in the craft path, despite fourteen other systems using it.

**So the original's ship handling is frame-rate dependent.** A longer frame
produces larger sub-steps and a different trajectory, and explicit Euler diverges
more at larger steps rather than degrading gracefully.

That makes [ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md)'s
stated cost real rather than hypothetical: our fixed timestep will not track the
original through a frame-rate dip, and the divergence is cumulative. The
verification harness must record the original's per-frame delta so drifted
intervals can be told apart from genuine bugs.

Reproducing the three-sub-step structure at our fixed `dt` is still right: at a
steady 60 Hz it is exactly what the original does.

## The air cushion

Two raycast probes, each a spring-damper. Rays are cast along the **ship's own
up axis**, not world gravity, which is what lets magstrips and inversions work.

Per probe:

```
h    = dot(probeWorld - rayHit, up)
v    = bodyLinearVel + M * cross(probeLocal, angularVel)
vn   = dot(v, normal)

spring = mass * 0.3 * (targetHeight - h) * K
       * (0.75 * grounded_prev + 0.25)
       * (normal_gravity + track_gravity)

d    = clamp(-0.1 * vn, -1.0, 2.0)
reb  = timeSinceLanding >= 0.2
         ? rebound
         : 0.5 * (rebound * t + landing_rebound * (1 - 5*t))

F    = up * spring * (1 + reb * d) * (1 - magLockBlend)
AddForceAtPoint(F, probeWorld)
```

Several details are counter-intuitive and worth calling out:

- **Stiffness carries no handling parameter.** It is
  `mass * 0.3 * (normal_gravity + track_gravity)`, calibrated against gravity so
  weight balances at a height error of `1 / 0.3 ≈ 3.33` units. Ship-to-ship
  differences come from the gravity values, not from a spring constant.
- **`rebound` and `landing_rebound` scale only the damping**, never the spring.
  `landing_rebound` replaces `rebound` for the first 0.2 s after touchdown.
- ~~**`ride_height` never appears in the force law.**~~ **Corrected.** It is the
  raycast length *and* the primary term of the hover spring's target height. The offset
  chain from the parser to the spring is traced in
  [engine.md](../ghidra/functions/psp-pulse/engine.md) at confidence **88**:
  `target = (ride_height + craft+0x74 - min(leapTimer, 4)) * (1 + 0.2 * magLockBlend) * K2`.
  What writes the additive `craft+0x74` was not found, so the composition is still open;
  `antigrav_height_adjust` is the obvious suspect and nothing was found that copies it
  there. The height the ship settles at is still emergent, being wherever the spring
  balances the load, but it is emergent *around a target that the data sets*.
- **Nothing explicitly pitches the ship to the track.** Pitch and roll response
  is entirely emergent from applying two forces at two points.
- Contact is accepted only for surface types 1 (Floor) and 3 (Mag Floor).
- Groundedness is quantised to `{0, 0.5, 1.0}`: contact count over two.

`K` is a global that the static image holds as `1.0` but which ship construction
overwrites with `1.3333` before any race runs. Confidence **90**, and worth a
runtime check since it scales hover stiffness directly.

A hard constraint exists but is not the hover mechanism: when `h < 1.0` on a
floor surface, position is teleported by `up * (1 - h)` with no velocity change.
That is penetration escape for the last unit only.

Grounded-only terms from the same function: a surface-alignment torque of
magnitude `400` along `cross(up, avgNormal)` with the right-axis component
projected out, which levels roll and yaw but deliberately **not** pitch; a
bank-to-yaw coupling of `+30 * right.y`; and a downforce along the ground normal
opposing the hover spring, whose magnitude is
`track_gravity * mass * grounded * (1 - magLockBlend)` - read at instruction
level in
[engine.md](../ghidra/functions/psp-pulse/engine.md#the-grounded-downforce-read-instruction-by-instruction),
where the coefficient is `1` and the term turns out to be what the spring's
`normal_gravity + track_gravity` calibration is calibrated *for*.

Confidence **91** for the force law.

### The cross-product signs: a contradiction here, resolved

This page originally recorded that term as `-400 * cross(up, avgNormal)` *and*, in
the same sentence, as one "which levels roll and yaw". **Those two statements
cannot both be true**, and the disagreement is settled by arithmetic rather than by
re-reading the prose, which is the lesson [`HANDOVER.md`](../../HANDOVER.md) draws
from every other disagreement this project has had.

Let `omega = up x n`. A torque along `omega` grows angular velocity along `omega`,
and the up axis then moves as

```text
d(up)/dt = omega x up = (up x n) x up
         = n * (up . up) - up * (n . up)      [ (a x b) x c = b(a.c) - a(b.c) ]
         = n - up * (n . up)                  [ up is a unit vector ]
```

which is the component of `n` perpendicular to `up`: it points **from `up` toward
`n`**. So `+k * cross(up, n)` aligns and `-k * cross(up, n)` diverges, and no
magnitude can change that - not the gain, not the inertia tensor, not the probe
offsets, not the target height. The prose's claim is about *behaviour*, which is
what a trace would show and is the stronger claim; a leading sign or an operand
order is a detail of how a decompiled expression was written down. Two readings of
the original produce the aligning term and both fit the prose: transposed operands,
`cross(avgNormal, up)`, or a transposed sign.

Simulation of the transcription corroborates it: with the literal `-400`, a ship
placed on a flat floor with a 0.02 rad roll grows to a full tumble in under a
second of simulated time at 60 Hz, which is the signature of anti-alignment rather
than of a mistuned gain.

**A second term has since turned out to have the same shape, and that changes the best
explanation.** [engine.md](../ghidra/functions/psp-pulse/engine.md)'s weathervane torque
is `cross(forward, velocity) * (grounded ? -0.1 : -0.3)`, described two lines later as
"what turns the nose toward the direction of travel" - the identical pattern, a negative
coefficient on a cross product that the same identity says must be positive for the
stated behaviour. An anti-weathervaning term would spin every ship out of every corner.

Two independent transcription errors of one shape is a poor explanation where a
**systematic handedness difference** is a good one, and handedness is listed as not
determined on both pages. If that is what this is, both expressions are correct as
written *in the original's own frame*, nothing is a typo, and a reimplementation in a
right-handed frame is simply obliged to flip every cross product it inherits from this
path. That is a prediction: the next cross-product term recovered from the craft path
should need the same flip. Cheap to check, and it would settle the handedness question
as a side effect.

**Confidence 84** on the direction, which is the top of the *Probable* band and the
[rubric](../reverse-engineering/confidence-rubric.md)'s stated ceiling for
decompilation-only evidence. Deliberately not 85, even though the arithmetic admits
no third reading: neither leg of the 85-94 band applies. The decompilation as
recorded here is not unambiguous - it contradicts itself, which is the whole
problem - no call site was re-read, and there is no multi-file data invariant. What
puts it at the top of its band rather than lower is that the derivation is
compulsory and that simulation agrees. What would raise it: re-reading
`Ship_HoverTwoPoint` at `0x0884a658` to see which of the two candidate errors this
was, and then a trace.

**Only the signs are settled: the magnitudes `400`, `0.1` and `0.3` are unverified and
stay M3 work**, like everything else on this page. `oag-physics` implements the aligning
direction for both terms, carries the derivation at each constant, and pins each
direction with a test asserting it on the torque itself rather than through a simulation,
so that neither the integrator nor an invented inertia tensor can mask a regression.

### A numeric prerequisite for M3, found while implementing this - and since resolved

This section used to derive the stability limit wrong, in a way that made the problem
look nearly twice as mild as it is, and then asked M3 to settle three open questions with
a trace. A trace has since answered two of them, and the third turned out to need a
different kind of evidence than a trace at all. What follows is corrected; see
[oag_physics::hover::ALIGNMENT_GAIN](../../crates/physics/src/hover.rs) and
[docs/ghidra/functions/ps2-pulse/craft-update.md](../ghidra/functions/ps2-pulse/craft-update.md)
for the full evidence trail.

First, this **closes an earlier observation**, made while implementing from this page
alone, that there is no angular damping anywhere in the model. That was true of what this
page recorded and false of the game: `pitch_damping` is consumed by
`Ship_ApplyAngularDamping` rather than by the pitch term, and that function damps all
three axes at `(-pitch_damping, -5.0, -2.0)` times the body-local angular velocity. Only
the pitch axis is per ship; yaw and roll damping are the same for every craft in the game.
**Both binaries now confirm this at instruction level**: the PS2 build's
`Ship_ApplyAngularDamping` uses the identical triple, with roll damping at `-5.0` rather
than `-2.0` in what the PS2 code calls "mode 0" - so `c = 2` is specifically the
non-zero-mode value.

`Ship_ApplyAngularDamping` damps roll at a hard `-2.0` outside mode 0, so roll is a damped
oscillator with stiffness `k = 400` and damping `c = 2.0` - both now confirmed as literal
immediates in the PS2 binary too (`0xC3C80000` and the damping triple), not a PSP
transcription error. **The derivation this page previously gave, `h <= c/k` against the
sub-step `h = 1/180`, is wrong**, because it assumes the acceleration is recomputed every
sub-step. The integrator does not do that - `oag_physics::integrate` computes the
acceleration once from the frame's starting state and holds it across all three sub-steps,
and the PS2's own `Body_Integrate` does the identical thing at instruction level. That
changes the governing step from the sub-step to the **frame**, and the correct condition,
accounting for the real position/velocity update across `n` sub-steps of `h = H/n`, is

```text
c >= k * H * (n + 1) / (2n)
```

which at `k = 400`, `H = 1/60` and `n = 3` needs `c >= 4.44` against the actual `c = 2` -
a **2.22x** shortfall, not the 11 % this page previously computed. Raising the sub-step
count cannot fix it: the requirement floors at `k*H/2 = 3.33` as `n -> infinity`, still
above 2. A ship sitting still on flat ground with a small roll grows its oscillation and
tumbles within a few seconds under this reading, which `oag-physics` used to reproduce.

Two properties still make this a useful measurement:

- It is **independent of whether the angular accumulators hold torque or angular
  acceleration**, since `k` and `c` scale together - which mattered, because that question
  was open.
- It is now **confirmed in a second binary at instruction level**, closing the
  "is a magnitude just wrong" question definitively: it isn't. `-400` and `-2.0` are both
  exactly what the PS2 build uses too.

**What this page asked M3 to settle with a trace has been settled, and the answer is a
third thing neither "a transcription error" nor "the sub-step count is wrong" covers.** A
step response on the running PSP game - roll the ship `0.25 rad` via the debugger, trace
the recovery - measured the *closed-loop* stiffness at about **20.2**, not 400, while the
damping came out near the transcribed `2.0`. Both readings are correct at once because
something divides the `400` torque by about **20** before it reaches the body as angular
acceleration - most likely a roll moment of inertia this crate's accumulators bypassed
(`oag_physics::forces::drain` multiplied by the inertia tensor that
`oag_physics::integrate` then divided back out, so the term never actually saw it).

**That guess was right, and the fitted `ALIGNMENT_INERTIA = 19.8` it stood on is now
retired.** The accumulators hold torque, the round trip is gone, and the divisor is the
recovered tensor's own entries - `15.6` on roll and `21.6` on yaw, the two axes the term
acts on once its pitch component is projected out. The fit's `19.8` sits between them,
which is what a term spread across two axes should look like and is a check the fit could
not have arranged. On roll alone the recovered value is `400 / 15.6`, a natural frequency
of `5.06` rad/s against the original's measured `4.49`: **13 % stiff, left standing rather
than tuned away**, and pinned by
`hover::tests::the_roll_oscillator_is_stable_by_the_margin_that_was_measured`. See
`ALIGNMENT_GAIN` in `crates/physics/src/hover.rs` and
[angular-velocity-column.md](angular-velocity-column.md).

The remaining open question is not this stability arithmetic - it's the separate,
still-unresolved torque-sign anomaly (`Body_AddForceAtPoint` computes `F x r`, not the
physically-correct `r x F`, confirmed in both binaries' raw assembly and ruled out as an
operand-convention misreading against the vendor VU manual). Whatever compensates that is
not this section's concern, since the measured behaviour above is convergent (aligning),
not divergent - see `craft-update.md` for where that question currently stands.

## Airbrakes

Each side ramps toward its analog input at `gain` upward and `falloff` downward,
so `falloff` is a per-second decay rate rather than a drag coefficient. The states run
`0..=100`, not `0..=1`; see the [control range](#still-open) note.

```
slide  = |L - R| * drag * |steerX| * 0.01
world += forward * speed * slide * 0.001
world += right   * speed * amount * (R - L)
angAccelLocal.y += speed * turn * (R - L) * 0.001
```

- `amount` is a **lateral force gain**, not a drag.
- `turn` feeds body-local angular acceleration directly.
- **Both `(R - L)` terms are written in the original's frame and change sign on
  the way into this crate.** `right` above is row 0, which points *left*, and a
  positive `angAccelLocal.y` there turns the nose *right*. In a right-handed
  `(right, up, forward)` frame the pair becomes `(L - R)`, so braking one side
  turns the nose toward that side and pushes the body the other way. Transcribed
  literally, both terms run backwards; see
  [engine.md](../ghidra/functions/psp-pulse/engine.md#ship_updateairbrakes-force-block-read-end-to-end)
  for the measurement (`airbrake-left-only`, 33/33 wall-free ticks) and
  `crates/physics/src/airbrake.rs` for where the negation lives.
- **Airbrakes produce no direct roll torque.** Confidence **85** on this negative,
  established by checking every write to the angular accumulators. Visible roll must come
  from the surface-alignment and bank-coupling terms, or from graphics-only state.
- **Narrowed.** This page used to add "no Z component is ever written", which
  [engine.md](../ghidra/functions/psp-pulse/engine.md) showed was too strong. The precise
  claim is that **no control input writes an angular Z**: not the engine, the brakes, the
  steering, the airbrakes or the pitch axis, so roll is never *commanded*. Angular Z is
  written in exactly two places and both are passive - `Ship_ApplyAngularDamping` and the
  surface-alignment torque. Confidence **80** on the narrowed version, which is lower
  because `Ship_UpdateMagLock` was not scanned and a Z write there would have been missed.
- `sideshift` is a **world-space force** applied straight to the body, bypassing
  the craft accumulator, for the `0.2 s` its per-side timer runs and **only while
  the craft is in contact**. Read at instruction level in
  [engine.md](../ghidra/functions/psp-pulse/engine.md#the-sideshift-is-a-force-and-its-direction-is-read-rather-than-guessed),
  which also settles the direction: a craft shifts toward the side it was
  flicked.

Lateral grip:

```
k = max(L, R) * (0.01 - slidegrip) - 1.0
lateral += grip_ground * dot(vel, right) * k * grounded
lateral += grip_air    * dot(vel, right) * k * (1 - grounded)
```

With the load scaling, `slidegrip` reads as **percent of grip retained at full
airbrake**, 0 to 100: at 0 grip vanishes and the ship drifts freely, at 100 it is
unchanged. Confidence **86**, and now **90**, because
[engine.md](../ghidra/functions/psp-pulse/engine.md) found the load-time factor that
makes the pseudocode above and this reading the same statement: the parser stores
`slidegrip * 1e-4`, so an XML 0..100 is 0..0.01 in memory, `(0.01 - slidegrip)` runs from
`0.01` down to exactly `0`, and the control states run 0..100 rather than 0..1.

**Now runtime-confirmed, and raised to 92.** Projecting a wall-free cornering
capture onto the ship's right axis fits the whole lateral law to `99.95 %` with
an rms of `3.28` units against a grip force reaching `375`, and returns
`grip_ground` at `0.9985 +/- 0.0005` of the disc's own value. The `k` factor is
confirmed by the data rather than only by arithmetic: dropping it triples the
residual. The law is **linear in `dot(v, right)` and independent of speed** -
alternatives scaling with `|v|` or with `|v_lat|` are 3.5x and 6x worse - and it
does not saturate out to a lateral speed of `37.5`. `grip_air` remains untested;
no capture has a usable airborne stretch. Evidence in
[cornering-ground-truth.md](cornering-ground-truth.md#the-lateral-axis-ship_applylateralgrip-is-exactly-right).

**Worth recording as a method note.** This crate first implemented the prose and
rejected the pseudocode as garbled, on the grounds that taken at face value it multiplied
grip by about a hundred. The pseudocode was correct and the *units* were missing. A
formula that cannot be right on the units in front of you is evidence about the units.

**"There is no dedicated airbrake drag term" - a fourth copy of a claim that is
false, and this is its correction.** `Ship_UpdateAirbrakes` applies
`forward * fs * |L - R| * Airbrake.drag * |steerX| * 1e-5` (conf 88, read from
both literals), `crates/physics/src/airbrake.rs` has implemented it since the
crate was written, and a wall-free cornering capture measures it at `1.03` to
`1.07` of the read value on 1,997 intervals where it reaches `31.5` units of
force. Three other copies of this sentence were corrected in an earlier pass and
this one was missed.

What the sentence should say is that the `drag` parameter **does not slow the
ship down**: every factor is non-negative and the term points along `+forward`,
so for a ship moving forwards it accelerates. Speed loss under braking is
indirect: lateral grip converting sideways motion into body-frame force, plus the
always-on quadratic drag and a constant `-2.0 * unit(v)` rolling resistance.

## Magstrips

Magstrip surfaces are **ordinary floor geometry with a tag** (surface type 3
rather than 1), not a distinct mechanism. See
[collision](../ghidra/functions/psp-pulse/collision.md).

When a dedicated probe detects one, a 0-to-1 blend fades out the ordinary
suspension: spring, damping and lift are all scaled by `(1 - blend)` while a
separate magnetic-hold block takes over. Confidence **82**; the hold block's own
physics was not decoded.

## What is implemented

[`oag-physics`](../../crates/physics/src/lib.rs) transcribes this page and
[engine.md](../ghidra/functions/psp-pulse/engine.md). It is structure only: no constant
was invented, nothing was tuned, and every test in that crate asserts a structural
invariant rather than a speed, a height or a turn rate. M3's
[verification protocol](../reverse-engineering/verification-protocol.md) comes before any
tuning, per the [roadmap](../overview/roadmap.md).

**Implemented, transcribed from the evidence:**

| Term | Where | From |
| --- | --- | --- |
| Clamped delta, forces evaluated once, three explicit Euler sub-steps of `dt/3` | [`integrate.rs`](../../crates/physics/src/integrate.rs) | this page |
| Two-probe air cushion: spring, damping multiplier, rebound and landing blend, magstrip blend consumption, target height from `ride_height` | [`hover.rs`](../../crates/physics/src/hover.rs) | both |
| Penetration escape at `h < 1.0`, surface alignment, bank-to-yaw, the downforce **and its magnitude**, the code-literal probe offsets and the target-length reach | [`hover.rs`](../../crates/physics/src/hover.rs) | this page, engine.md |
| Engine thrust and its cap, the brake and its ramp, steering and its asymmetric ramp, the pitch axis | [`engine.rs`](../../crates/physics/src/engine.rs) | engine.md |
| The five control states and their ramps, on the original's `0..=100` scale | [`controls.rs`](../../crates/physics/src/controls.rs) | engine.md |
| Quadratic drag with all four coefficients, rolling resistance, weathervane, angular damping, vertical damping, gravity | [`passive.rs`](../../crates/physics/src/passive.rs) | engine.md |
| Airbrake ramps, slide, lateral force, yaw, sideshift impulse, lateral grip | [`airbrake.rs`](../../crates/physics/src/airbrake.rs) | both |
| Four accumulators, the fifteen-step term order, and the stale/fresh groundedness split | [`forces.rs`](../../crates/physics/src/forces.rs) | engine.md |
| Segment-triangle narrowphase as a plane sign change plus three edge half-space tests | [`collide.rs`](../../crates/physics/src/collide.rs) | [collision](../ghidra/functions/psp-pulse/collision.md) |
| The magstrip hold: blend ramp, mag-floor probe, two-sample axis blend, reposition, velocity projection and the **kinematic basis rewrite** | [`maglock.rs`](../../crates/physics/src/maglock.rs) | engine.md |

**Implemented as a shape with the coefficient left at the identity or zero**, because the
evidence records that the term exists and not how large it is. That is deliberate: a
plausible-looking number would be indistinguishable from a recovered one later.

- The engine's `craft+0x294` output multiplier (identity).
- The per-class gravity scale from the table at `0x08ab0dcc` (identity, and an input
  rather than a constant, since it belongs to the speed class - the shipped table
  reads `1.0` for all four, so the identity is also the value).

Two names have left this list. `K2`, the hover target's global scale, was read at
`0.75`. The grounded **downforce** was read at `track_gravity * mass * grounded`,
and it was carrying more than its own weight: because the spring's damper is a
multiplier on the spring magnitude, the missing downforce was also the missing
pitch damping - see
[angular-velocity-column.md](angular-velocity-column.md#resolved-the-missing-49-is-the-hover-downforce).

**Deliberately not implemented, and why:**

- **Everything flag-gated on the undecoded `craft+0x1c0`**: turbo and its boost lift, the
  uncapped-thrust mode, the `craft+0x2a0` and `craft+0x31c` engine multipliers, the engine
  kill switch, and the steering bias at `craft+0x2e4`. Implementing them means inventing
  their triggers.
- **The `craft+0x2a4` mode enum**, which selects a `-0.9` drag coefficient, a `-5.0` roll
  damping and a disabled `rebound` in mode 0, and skips the control block entirely in 4,
  5, 6 and 8. Naming it is a guess at confidence 40, so the racing values are used.
- **The four-corner hover variant and its auto-speed law.** The selector is now *known* -
  `DAT_08ab07e3 == 0 && DAT_08b31048 == 6`, confidence 84 - and the same condition gates
  the brakes off, but what the mode *is* sits at confidence 50.
- **The track-section force** (`FUN_08848f9c`), a guess at confidence 45 as to what it
  even is.
- **The per-team in-air pitch bias** at `stats_base + 0x90`, which is outside every class
  block and whose XML element is unknown, so there is no field to read it from.
- **The gate on pitch input** (`FUN_088492bc`), not decoded, so pitch applies
  unconditionally.
- **The sweep-and-prune broadphase.** A linear scan over triangles behind an AABB reject
  is correct and is not the bottleneck yet. Worth recording before anyone implements it:
  the original's packing quantises coordinates to 1 unit over roughly +/-1024 and caps ids
  at 1024 per list, a hard limit inherited from the data rather than a design choice.
- **The in-air roll-levelling branch**, which is [dead code](#a-dead-branch-not-to-port).
- **The engine throttle ramp**, which is dead in the original: `<Engine gain/>` and
  `<Engine falloff/>` are parsed and overwritten by the raw input before anything reads
  them, verified in disassembly at confidence 85. Both parameters are carried and unused.
  Reintroducing the ramp is the obvious mistake, and the throttle lag it adds is exactly
  the kind of difference that reads as "feels close enough".

## Still open

- **Nothing is runtime-verified.** All of the above is static analysis.
- **The parameter scaling boundary.** Four fields are pre-scaled at load
  (`Engine.amount` x0.001, `Brakes.amount` x-0.01, `Airbrake.amount` x1e-4,
  `Airbrake.slidegrip` x1e-4). `oag_formats::handling` returns the document's raw values,
  `oag_physics::Handling` holds the scaled in-memory form, and `oag-gameplay` applies the
  factors between them. **Applying them twice is the most likely integration bug here**
  and it would be silent: the ship would simply be sluggish.
- **The control range - settled for the steering axis.** Three separate pieces of
  arithmetic only come out right if the control states run `0..=100` rather than `0..=1`:
  the brake ramp's `100.0` clamp, the engine's `throttle > 100.0` test, and the grip
  coefficient reaching exactly zero at full airbrake. That the *analog axes* share that
  scale was an inference; for `stick_x` it is now **measured**. Both captures in
  `data/traces/` show `craft+0x2c0` ramping and falling at rates that reproduce the
  shipped `Turning.gain`/`Turning.falloff` to under 1 %, which only works on the
  `0..=100` reading. `stick_y` stays an inference - the craft control block holds no
  second axis to capture.

  **The old warning attached to this entry - that an unscaled `Turning.amount` would put
  yaw out by whatever the axis scale was wrong by - is falsified.** The axis scale is
  right and `Turning.amount` really is stored verbatim, yet yaw still comes out about
  **22x** too strong. Dividing by the control range "fixes" it to 4.7x too weak. See
  [engine.md](../ghidra/functions/psp-pulse/engine.md#steering) for the instruction-level
  reading and `oag_physics::forces::YAW_INVERSE_INERTIA` for what closes it -
  **the yaw entry of the body's inverse inertia tensor**, recovered from
  `Body_SetBoxInertia`, applied to the whole body-local yaw axis because the
  bank-to-yaw coupling recorded above shares the same accumulator and therefore
  the same factor.
- **Whether the two angular accumulators hold torque or angular acceleration.** Nothing in
  any term visibly divides by an inertia, and the angular damping term's shape is
  dimensionally an acceleration, so `oag-physics` treats both as acceleration. Under that
  reading the inertia tensor has no effect on any angular term at all.

  **For magnitude this no longer matters, and that is now proven rather than assumed.**
  `Ship_UpdateSteering` and `Ship_ApplyAngularDamping` write to the *same* accumulator
  (`craft+0x340`), which `Ship_UpdateCraft` zeroes every frame and `Body_AddTorqueLocal`
  forwards unscaled, so any common factor - the inertia tensor included - cancels out of
  the resulting equilibrium whichever quantity the accumulator turns out to hold.
- **The alignment gain and the roll damping are marginally unstable together**, by 11 % at
  the specified sub-step; see
  [the arithmetic above](#a-numeric-prerequisite-for-m3-found-while-implementing-this---and-since-resolved).
- **Nothing in the *force law* can hold an inverted ship, and nothing was ever going to.**
  Gravity writes world `.y` only, `track_gravity` reaches the force law solely through the
  hover spring's *magnitude*, and the hover spring always pushes the ship *away* from the
  surface it found. This used to end "so the magnetic hold must be what holds a ship to a
  ceiling, and it is exactly the block that was not decoded". That block is read now, and
  the reason no force could be found is that **there is no force**: `Ship_UpdateMagLock`
  writes the body's position, velocity and basis directly and never touches an
  accumulator. See [`maglock.rs`](../../crates/physics/src/maglock.rs) and
  [engine.md](../ghidra/functions/psp-pulse/engine.md#ship_updatemaglock-rewrites-the-basis-directly-and-that-is-the-missing-mechanism).
- **The hover target's additive offset**, `craft+0x74`. Nothing was found that writes it,
  and `antigrav_height_adjust` is only a suspect (a search for readers of that field found
  none, a weak negative at confidence 50). It matters more than it looks: with the offset
  at zero the target equals the raycast length, so a ship sits only its own spring sag
  below the top of its probe range and a small pitch loses contact. A negative offset
  would give the probes headroom.
- **Whether the airbrake `drag` term accelerates or decelerates.** Every factor in
  `world += forward * speed * slide * 0.001` is non-negative, so the literal reading
  accelerates while the parameter's name argues the other way. Implemented literally,
  flagged in code as a guess.
- **The quadratic drag's direction while reversing.** `velocity * forwardSpeed * k`
  delivers power `|v|^2 * forwardSpeed * k`; with `k` always negative that is dissipative
  exactly while `forwardSpeed > 0` and *adds energy* while reversing. Unlike the
  cross-product signs, no prose on either page contradicts the expression - the "20 to 50
  times more drag" note is about the coefficient's magnitude - so it is transcribed
  literally, and no test in `oag-physics` lets a ship reverse.
- **The rebound blend inside the landing window.**
  `0.5 * (rebound * t + landing_rebound * (1 - 5*t))` is discontinuous at `t = 0.2`, where
  it evaluates to `0.1 * rebound` against the other arm's `rebound`, and it multiplies a
  coefficient by a time. `1 - 5*t` reaching zero exactly at the window's end is the one
  part that reads as intended, so a factor was probably lost when this was written down.
  Transcribed as written.
- **The probe point velocity.** This page writes
  `bodyLinearVel + M * cross(probeLocal, angularVel)`, the negation of the conventional
  rigid-body point velocity. The crate uses the conventional `omega x r`, since that is
  what damps a descending nose rather than pumping it.
- ~~**Where the two probes sit.**~~ - **closed**: a code literal,
  `(0, -1.125, +/-4.5)` after the `0.75` global, identical for every craft, and
  landed. `<Misc>` reaches the collider, not the probes.
- ~~**Whether `sideshift` is an impulse or a velocity**~~ - **closed**: it is
  neither, it is a force for `0.2 s`, so mass does divide it. What is still open
  is the *trigger*: the stick-flick path is read, the tap-history path above it
  is not, and `oag-input` produces no sideshift at all.
- **Which speed the airbrake terms use.** This page writes only "speed"; the craft caches
  `|dot(velocity, forward)|` and the engine reads it from there, so that is what the crate
  uses, rather than `|velocity|` as the brake does.
- **Whether the brake force is horizontal-only.** A VFPU target prefix reads, under the
  standard selector, as zeroing the world `y` lane. Confidence 55, the encoding was not
  confirmed, and it is low-stakes because braking only runs while grounded. Not
  implemented.
- **The pitch axis's sign convention**, and coordinate conventions generally: handedness,
  units and angle representation are open on both pages. Triangle winding in the collision
  data is unknown too, and the crate assumes a normal that points out of the surface.
- **The XML `mass` at `+0xf4` versus the body mass at `body+0x374`.** Every force term in
  engine.md reads the latter; the hover spring on this page reads "mass". The crate reads
  the parameter set for the spring and the body for gravity, and keeping the two equal is
  the integration layer's job.
- The magnetic-hold force law.
- What the `craft+0x2a4` mode enum is, and the eleven flag bits at `craft+0x1c0` other than
  bit 0.
- **What arms the `craft+0x2e0` timer**, which lowers the hover target and suppresses
  lateral grip while it runs. A leap, a respawn and a race start are all plausible.

## A dead branch not to port

An in-air roll-levelling term is computed into a VFPU register and **never
stored**; the next function reloads that register immediately. It is dead code,
most likely a compiler artefact.

Recorded here because it is exactly the kind of thing a careful reimplementation
would faithfully copy by mistake.

## Approach

Physics is where "it feels right" is most tempting and least trustworthy. Every
part of this system gets verified against a trace from the original before it is
considered done. See the
[verification protocol](../reverse-engineering/verification-protocol.md), whose
early scenarios exist specifically to isolate the systems on this page.
