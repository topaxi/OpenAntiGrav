# Physics

**Status: model recovered from static analysis, nothing implemented.**

The shape of the ship dynamics is known, and every constant it needs lives in
[handling stats](../formats/handling-stats.md). Nothing here has been verified at
runtime; treat it as a specification to test against, not as truth.

## The two answers that were blocking M4

### It is floating point, not fixed point

Pure IEEE single precision throughout, scalar FPU plus VFPU. The only integer
operations in the craft path are pointer arithmetic, loop counters and flag
masks. Confidence **97**.

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
- **`ride_height` never appears in the force law.** It is only the raycast
  length; the height the ship settles at is emergent.
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
`-400 * cross(up, avgNormal)` with the right-axis component projected out, which
levels roll and yaw but deliberately **not** pitch; a bank-to-yaw coupling of
`+30 * right.y`; and a downforce along the ground normal opposing the hover
spring.

Confidence **91** for the force law.

## Airbrakes

Each side ramps toward its analog input at `gain` upward and `falloff` downward,
so `falloff` is a per-second decay rate rather than a drag coefficient.

```
slide  = |L - R| * drag * |steerX| * 0.01
world += forward * speed * slide * 0.001
world += right   * speed * amount * (R - L)
angAccelLocal.y += speed * turn * (R - L) * 0.001
```

- `amount` is a **lateral force gain**, not a drag.
- `turn` feeds body-local angular acceleration directly.
- **Airbrakes produce no direct roll torque.** No Z component is ever written.
  Visible roll must come from the surface-alignment and bank-coupling terms, or
  from graphics-only state. Confidence **85** on this negative, established by
  checking every write to the angular accumulators.
- `sideshift` is a one-shot lateral impulse applied straight to the body,
  bypassing the craft accumulator.

Lateral grip:

```
k = max(L, R) * (0.01 - slidegrip) - 1.0
lateral += grip_ground * dot(vel, right) * k * grounded
lateral += grip_air    * dot(vel, right) * k * (1 - grounded)
```

With the load scaling, `slidegrip` reads as **percent of grip retained at full
airbrake**, 0 to 100: at 0 grip vanishes and the ship drifts freely, at 100 it is
unchanged. Confidence **86**.

**There is no dedicated airbrake drag term.** Speed loss under braking is
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

## Still open

- **Nothing is runtime-verified.** All of the above is static analysis.
- The magnetic-hold force law.
- Sign conventions on terms that depend on loaded values, notably whether the
  airbrake `drag` term accelerates or decelerates.
- A four-corner hover variant exists alongside the two-point one; which is used
  when was not established.
- Coordinate conventions: handedness, units, angle representation.

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
