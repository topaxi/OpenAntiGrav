# Ship physics specification

How a craft moves, per tick, in the Studio Liverpool anti-gravity engine.
Pulse (PSP) is the reference; where another title differs, the
[per-title table](#per-title-differences) says how and how well it is known.
The page is meant to be read alone: a modder, an emulator author or a
revival-server author should be able to build the craft update from it.

Every law carries a confidence from 0 to 100 ([rubric](../reverse-engineering/confidence-rubric.md)).
A law with no score is **chosen, not measured**: nothing in the original says
so. Tuning values (engine `amount`, gravity, `ride_height` and so on) are
authored per ship and speed class in the disc's `handlingstats.xml` and are not
reproduced here; [handling stats](../formats/handling-stats.md) is the schema.
Numbers on this page are code literals out of the executable.

## Conventions

- **Axes.** `forward`, `right`, `up` are the craft's own unit axes. Torques are
  described by effect ("turns the nose toward travel"), not by a sign. The
  original stores angular velocity negated (`w_game = -w_physics`) and its
  cross-product coefficients are written for that convention, which is why its
  weathervane and alignment coefficients read negative where the effect below
  needs a positive one. Apply exactly one flip, everywhere. The sign convention
  has no score of its own in the evidence; the recorded momentum column is
  reproduced with it applied once.
- **Timing.** The original runs a variable `dt`, clamped to `[0, 0.06666]`
  (Pure's craft update runs at 30 Hz, Pulse's at the display rate). This engine
  runs a fixed 60 Hz, which is what the original does at its target frame rate.
- **Units.** World units and seconds. A speed shown in km/h is `3.6 *` units/s.
  Control states run `0..100`, not `-1..1`.
- **Mass and gravity.** Gravity acts on world `-y` only. `track_gravity` never
  appears in the gravity force; it enters the hover spring and the downforce.
- **Previous versus current contact.** `grounded` is `0`, `0.5` or `1`: the
  fraction of hover probes in contact. Terms marked *prev* read last frame's
  value, because hover (step 8) rebuilds it mid-frame.

## The frame

```text
dt = clamp(measured_dt, 0, 0.06666)
evaluate every force once, at full dt            (steps 1 to 15 below)
3 explicit Euler sub-steps of dt/3, same acceleration each time
resolve wall contacts on the integrated pose
```

Explicit Euler: position advances by the old velocity, then velocity by the
acceleration. Forces are evaluated once, so the stability bound of a stiff term
is set by the frame `dt`, not `dt/3` (a spring-damper `a = -k x - c v` is stable
while `c >= (2/3) k dt`). Derived arithmetic, unscored in the evidence.

| Step | Term | Reads contact |
| --- | --- | --- |
| 0 | control states ramp (throttle, brake, airbrakes, steering) | none |
| 1 | engine (local force) | prev |
| 2 | brakes (world force); skipped airborne and in Zone | prev |
| 3 | airbrakes, sideshift | prev |
| 4, 5 | steering, pitch (local torque) | prev |
| 6, 7 | quadratic drag, gravity | prev |
| 8 | hover: two probes, alignment torque, downforce, bank-to-yaw | produces current |
| 9 | magstrip hold (kinematic basis rewrite, not a force) | current |
| 10 | stun and slowdown timers decay | |
| 11 | lateral grip; skipped while either timer runs | current |
| 12 | weathervane torque | current |
| 13 | angular damping, rolling resistance | current |
| 14 | vertical damping | current |
| 15 | speed-pad boost | |

The order is read from the original's fifteen-step craft update (confidence 84
for the ordering, from one basic block). Four accumulators are drained once at
the end: local force, local torque, world force, world torque. Cached forward
speed `fs = |dot(velocity, forward)|` is the **absolute** value (confidence 90),
so every passive term reads a non-negative speed.

## Control states

| State | Law | Confidence |
| --- | --- | --- |
| Throttle | `thrust = input * 100`, **no ramp**. `Engine.gain` and `Engine.falloff` are parsed and dead | 85 |
| Airbrake L, R | ramp toward `input * 100` at `Airbrake.gain` up, `Airbrake.falloff` down. **Unsettled:** linear is a guess, an exponential would also fit "per second"; a capture of a released airbrake would settle it | unsettled |
| Brake | no axis: both airbrakes held raises it at `Brakes.gain` to a ceiling of 100, otherwise it falls at `Brakes.falloff` | unscored in the evidence |
| Steering | ramp toward `stick * 100`: `Turning.gain` per second toward a larger magnitude, `Turning.falloff` back toward centre; reaches zero exactly | 84 |

The steering ramp on Pulse has no clamp, so a held lock overshoots by float
rounding and cycles `83.3 / 91.7 / 100`. HD clamps at the target (measured, see
the table below). Measured against the original: the steering state ramps at the
authored rates to under 1 % on the `0..100` scale (confidence 90).

## Forces

`T` is thrust, `fs` the cached speed, `v` the linear velocity.

**Engine** (confidence 88, the lowest of its recovered parts; reproduces a
standing-start capture at rms 0.127 force units).

```text
if (stun_timer > 0 or slowdown_timer > 0) and no running Turbo:  T = 0, lift = 0
T = throttle * Engine.amount                  (amount pre-scaled by 0.001)
T = T * grounded + T * 0.2 * (1 - grounded)   (blended, not switched)
cap = 0.5 * fs + Engine.accelcap
if throttle > 100: cap = cap * throttle * 0.01
T = min(T, cap)
if Autopilot running: T = T * 1.2
if Turbo running or barrel-roll payout running: T = T + Engine.turbo
                                                lift = turbo_jump * T   (d-pad Up held)
T = T * launch_multiplier * 2.0               (launch multiplier is 1.0 outside the start window)
if one_shot_scale < 1: T = T * one_shot_scale (LeachBeam drain; reset to 1)
```

The doubling is a code literal (confidence 88 on `launch_multiplier` being
`1.0` racing). Zone replaces the throttle path with a speed law and no cap (see
[race rules](race-rules.md#zone)). The stun timer is armed by a posted weapon
or rival impulse (`+= 0.5` s), not by touching a wall (confidence 85).

**Brakes** (unscored in the evidence). Only while `prev grounded > 0`:
`F = unit(v) * Brakes.amount * brake`, `Brakes.amount` pre-scaled by `-0.01`
(negative in memory), faded by `speed * 0.1` below 10 units/s.

**Steering** (90). `yaw_torque = -(steer * Turning.amount)` turning the nose
toward the stick, **no speed factor**, so steering works at a standstill.
Reverse-controls effects blend it: `yaw * (1 - 2 b)` for `b <= 1`, else `-yaw`.
The yaw axis is divided by its inertia (below); the force law has no other yaw
gain. Measured: a full lock yaws at about 1.4 to 1.5 rad/s.

**Pitch** (90). `pitch = steer_y * 100 * (grounded ? pitch_ground : pitch_air)`,
applied only while `dot(up, -track_down) > 0.9` (about 26 degrees from the
track's up under the craft; measured live on three crossings, 90). While not
grounded add `Misc.weight_distribution`, whatever the gate says (85). No ramp.

**Airbrakes** (88; sign and scale 90). `L`, `R` are the ramped states.

```text
slide   = |L - R| * Airbrake.drag * |steer_input| * 0.01        (steer_input on 0..100)
F      += forward * fs * slide * 0.001              (it accelerates; not a drag)
F      += lateral(toward the unbraked side) * fs * Airbrake.amount * (R - L)
yaw    += fs * Airbrake.turn * (R - L) * 0.001      (nose turns toward the braked side)
```

`Airbrake.amount` is a lateral force gain, `Airbrake.drag` a very small forward
acceleration; speed loss under braking comes from lateral grip and the brake.
No control writes a roll torque (85).

**Lateral grip** (90, arithmetic). Body-local force along `right`:

```text
k = max(L, R) * (0.01 - Airbrake.slidegrip) - 1.0     (slidegrip pre-scaled by 1e-4)
F_right = (grip_ground * grounded + grip_air * (1 - grounded)) * dot(v, right) * k
```

`k = -1` with no airbrake, so grip cancels sideways velocity; at full airbrake
with `slidegrip = 0` it reaches `0` and the craft drifts. Multiply both grips
by `1.5` while a barrel-roll payout runs (88). Skipped while a stun or slowdown
timer runs.

**Sideshift** (80 for the timings; 95 for the lockout). A double tap of an
airbrake (veteran) or a steering flick past `+-0.1` of full lock (novice)
pushes sideways with `Airbrake.sideshift` as a world force for `0.2` s. A
second shift cannot start until `1.0` s after the first ends. The tap window is
`0.25` s (85).

**Quadratic drag** (95; Pure measured live at 92).

```text
k = grounded(prev) ? -0.005 : -0.002          (air is smaller than ground)
F = v * fs * k
```

A third coefficient, `-0.1` for `forward_speed < -0.2`, exists in the code but
is unreachable because `fs` is absolute. Lateral velocity is dragged too.

**Rolling resistance** (92). `F = -unit(v) * 2.0`, constant, only while
`fs > 0`.

**Gravity** (class scale 90; the rest is read from the force site, unscored).

```text
F.y = -( normal_gravity * class_scale * mass * grounded
       + flight_gravity * mass * (1 - grounded) )          prev grounded
```

`class_scale` is `<GlobalClass><GravityMul airborne>` per speed class. Its
attribute name says airborne, but on Pulse it scales the **grounded** term; the
airborne term takes `1.0`. Check: the predicted resting compression is 1.2390
with the class scale and 1.2500 without, against 1.237 read live.

**Vertical damping** (92). `F = up * dot(up, v) * -0.25 * (1 - grounded)`:
airborne only, along the craft's own up.

**Weathervane** (84 on direction, magnitude unverified).
`torque = cross(forward, v) * k` with `k = 0.1` grounded and `0.3` airborne,
oriented so it turns the nose toward the direction of travel. This, not any
control term, is what aligns the craft with its path.

**Angular damping** (74 to 80). On the angular momentum `L = I w` in the body frame:
`torque = (-pitch_damping, -5.0, -2.0) * L` for (pitch, yaw, roll). Only the
pitch coefficient is per ship.

## Hover and track following

Two probes, front and back, sit at `(0, -1.125, -/+4.5)` in body space (a code
literal, `(0, -1.5, +/-6) * 0.75`, 92). Each casts a segment along `-up` of
length `target`, the spring's own target height.

```text
target = (ride_height - min(slowdown_timer, 4.0)) * (1 + 0.2 * mag_lock) * 0.75   (92)
h      = dot(probe_point - hit_point, up)
spring = mass * 0.3 * (target - h) * (4/3) * (0.75 * grounded_prev + 0.25)
                 * (normal_gravity + track_gravity)
damp   = clamp(-0.1 * dot(point_velocity, hit_normal), -1, 2)
F      = up * spring * (1 + rebound * damp) * (1 - mag_lock)      at the probe point
```

- The spring's global stiffness `4/3` is written at construction (95).
- `rebound` scales **damping only**. For `0.2` s after touchdown it is
  `0.5 * (rebound * t + landing_rebound * (1 - 5 t))`, discontinuous at the end
  of the window; kept literal and unscored (open, see below).
- Only floor and magstrip surfaces hover; walls and reset geometry do not.
- A probe with `h < 1.0` on a floor or wall teleports the body up by `1 - h`
  with no velocity change (penetration escape).
- **Downforce** (92): `-track_gravity * mass * grounded * (1 - mag_lock)` along
  the averaged contact normal. It carries the pitch damping; without it the
  damper multiplies a load 17 times too small.
- **Resting compression** `= (normal_gravity * class_scale + track_gravity) /
  (0.8 * (normal_gravity + track_gravity))`, so a craft rests near half its
  travel. Measured 1.237 against 1.2390 predicted (0.16 %).
- **Alignment torque** (84): while grounded,
  `400 * cross(up, avg_normal)` with the component along `right` removed, so it
  levels roll and yaw but not pitch. On roll the stiffness per unit inertia is
  `400 / 15.6 = 25.6`, measured on the original by a roll step response: half
  period `0.6249 s` against `0.6250 s` here (92,
  [roll-step-response.md](../physics/roll-step-response.md)).
- **Bank-to-yaw** (grounded): `yaw += 30 * right.y * (1 - mag_lock)`; 50 in
  Zone's four-corner hover.
- **Magstrip hold** (maglock): over a magstrip the suspension fades out as the
  blend ramps (`0.2` per frame, so five frames to full) and the craft is repositioned kinematically to
  `0.8` of the target height along the strip's surface axis. Inversions work
  because the probes cast along the craft's own up axis and
  `track_gravity` presses along the contact normal.

## Rigid body

- **Inertia** (92): a solid box `(12, 8, 12)` of mass `0.9`, a code literal for
  every craft, giving `I = (15.6, 21.6, 15.6)` on (right, up, forward). The mass
  the craft flies with differs; the tensor is frozen at construction. Pulse
  applies the diagonal in **world** axes (93: explains 100 % of the recorded
  momentum column on three captures).
- **Torque to acceleration**: `dw/dt = I^-1 torque`, with control torques
  divided by `I` and damping applied to momentum.

## Wall contact

Resolved after integration, per contact (scores per item below):

```text
j        = -(1 + 0.4) * dot(v_point, n) / (1/m + dot(n, cross(I^-1 cross(r, n), r)))
impulse  = j n - friction * (v_point - n dot(v_point, n))
v       += impulse / m
position += n * penetration
```

- Restitution `0.4` is a property of the body, not the wall (88).
- The angular half of the impulse is scaled by `0.1` (90): soft in translation,
  very stiff in rotation, so a hull corner does not flip the craft.
- Friction combines `(0.05 wall + 0.02 ship) / 2 = 0.035` per frame as a
  multiplicative loss on tangential velocity, with no `dt` and no Coulomb clamp
  (88). This is the whole sustained speed loss of a scrape. A floor or
  magstrip contact has friction `0`.
- The hull is a box from `Misc.width/height/length * 0.75`, tested with ten
  sample points from the centre. A contact exists only while the point is within
  `2.0` behind the surface (90). Every surface except reset geometry produces
  contacts.
- A swept pre-integration pass backs the body off the box face along its
  velocity by `0.9` of the overshoot (85).
- Hull energy: each contact costs `|impulse| * 0.035` of the shield pool
  (94, 25 of 25 calls), every tick it persists. Weapons-off races halve it
  (the original's `0.5` factor). Energy-critical fires on crossing 20 %.
  Zero energy starts a `0.5` s explosion, then the craft is out (88).

## Slowdown from weapons

A weapon hit adds its `slowdown_time` to `slowdown_timer`, clamped to the global
`slowdown_limit`; the timer decays by `dt` (88). While it is positive: no thrust,
no lateral grip, and the hover target is reduced by `min(timer, 4)`. A running
Shield pickup blocks the add (85).

## Speed classes

A speed class is a name on a ladder: `VECTOR, VENOM, FLASH, RAPIER, PHANTOM,
SUPERPHANTOM`, slowest first. Pulse and HD use the middle four, Pure adds
`VECTOR` below, 2048 adds `SUPERPHANTOM` above. A class selects one `<Class>`
block of every ship's `handlingstats.xml` (engine, brakes, steering, airbrakes,
antigravity, physical, pitch) and one `<GlobalClass>` entry (speed-pad
`amount` and `time`, gravity scale, weapon-pad refresh). Lap counts are 3, 4,
4, 5 for Venom to Phantom. No law on this page changes between classes: only the
tuning does. Zone's escalating speed does not select classes (it is a single
scale with names over it).

## Per-title differences

Everything above is Pulse. Rows are the laws the engine carries per title;
**M** measured on that title, **I:X** inherited from X without a check.

| Law | Pulse | Pure | HD / Fury | 2048 | Omega |
| --- | --- | --- | --- | --- | --- |
| Engine doubling | `x2.0` | none, M 92 | `x2.0` M | I:HD | I:HD |
| Grounded drag | `-0.005` | `-0.0025` M 92 | `-0.005` M | I:HD | I:HD |
| Bank-to-yaw gain | 30 | 50 M 92 | 30 M | I:Pulse | I:Pulse |
| Pitch gain | XML | `0.1` air, `0.4` ground, code, M 92; no gate, M | XML; gate and in-air bias M | gate and bias I:HD | gate and bias I:HD |
| In-air pitch bias | `weight_distribution` | none, M 92 (no term in Pure's pitch code) | M | I:HD | I:HD |
| Pitch damping | XML | `1.5` racing, `0.6` on grid, M 92 | XML | XML | XML |
| Sideshift force | XML | `200.0`, code, M 92 | XML | XML | XML |
| Inertia mass | 0.9 | 0.9 (I:Pulse) | 1.0 M | 1.0 M 84 | 1.0 M 84 |
| Inertia tensor frame | world axes | world axes, M | rotates with the body, M | rotates, I:HD | rotates, I:HD |
| Steering ramp | unclamped | I:Pulse | clamped M | I:Pulse | I:Pulse |
| Gravity class scale on | grounded term | I:Pulse | airborne term M 85 | I:HD | I:HD |
| Wall contact | two-sided | I:Pulse | one-sided; steers away; restitution from data M | one-sided and steers away M 84 | one-sided and steers away M 84 |
| Wall point rate | world | I:Pulse | body-local, M 85 | body-local, M 82 | body-local, M 82 |
| Launch boost | `<StartBoost>` XML | code law, M 88 | XML (see [launch boost](../physics/launch-boost.md)) | XML law (no code law) | XML law (no code law) |

Pure has no magstrips, no sideshift XML attribute and no `<pitch>` block. Not
every row above is separately measured on 2048 and Omega: they inherit HD's
craft laws, and the tables record each inheritance with its source. The
inertia mass, the wall contact and the wall point rate are read in both
executables, statically
([2048](../ghidra/functions/vita-2048-eu-v104/craft-laws.md),
[Omega](../ghidra/functions/ps4-omega-eu/craft-laws.md)): each is HD's.

## Open and unsettled

- The rebound blend (above) is discontinuous as transcribed; a factor may have
  been lost. Kept literal and unscored; a capture across a landing would settle it.
- The weathervane **magnitude** is unverified against play.
- The `craft+0x2a4` mode enum (mode 0 changes damping and drag by roughly 180x
  and reads as a front-end state) is not interpreted.
- The brake force may have its world-y lane zeroed (VFPU prefix, 55); not
  applied, low stakes because braking is grounded.
- Which engine early-return arm (stun or slowdown) fired in the first reference
  capture was not recorded.

## Where this engine differs from the original

Chosen, not measured: a fixed 60 Hz step; contacts built probe-major where the
original is triangle-major (the set is identical, the order is not); wall
contacts dropped past 40 per frame (the original's limit is a 128-entry array).

## Evidence

- Schema and scaling: [handling stats](../formats/handling-stats.md)
- Force law, hover, alignment: [physics](../physics/README.md),
  [force balance](../physics/force-balance-ground-truth.md),
  [cornering](../physics/cornering-ground-truth.md),
  [angular velocity column](../physics/angular-velocity-column.md),
  [airborne](../physics/airborne-ground-truth.md)
- Per title: [Pure](../physics/pure-physics.md),
  [HD handling](../physics/hd-handling-ground-truth.md),
  [HD ride height](../physics/hd-ride-height.md),
  [launch boost](../physics/launch-boost.md)
- Ghidra: [engine](../ghidra/functions/psp-pulse-usa/engine.md),
  [contact response](../ghidra/functions/psp-pulse-usa/contact-response.md),
  [rigid body](../ghidra/functions/psp-pulse-usa/rigid-body.md),
  [shield](../ghidra/functions/psp-pulse-usa/shield.md)
- Code: `crates/physics/src` (`engine`, `airbrake`, `passive`, `hover`,
  `wall`, `integrate`, `forces`, `laws`), `crates/title/src/craft_laws.rs`
