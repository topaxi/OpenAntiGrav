# Engine, brakes, steering and pitch

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`. **The names here are applied**, from [names.tsv](names.tsv).

This page covers the part of the craft force law that
[physics](../../../physics/README.md) does not: how the analog stick, the
throttle and the shoulder buttons become force. It also fixes the handling
parameter block's field layout to the byte, read out of the XML loader rather
than inferred from consumers.

## The parameter block, read from the loader

`Handling_ParseStats` (`0x0883a2f0`) walks `handlingstats.xml` and dispatches one
parser per element. Each parser compares an attribute name and stores the parsed
float at a fixed offset, so **the name-to-offset mapping is a direct read, not an
inference**. The class index is a file-scope global (`g_handling_parse_class`,
`0x08b36bfc`), set when a `<Class name="...">` attribute matches one of the four
speed-class names; the stride is `class * 0x80`.

| Block | Parser | Offsets (`+ class * 0x80`) |
| --- | --- | --- |
| `Antigrav` | `HandlingXml_ParseAntigrav` (`0x08839278`) | `ride_height 0x94`, `rebound 0x98`, `landing_rebound 0x9c`, `rebound_jump_time 0xa0`, `grip_ground 0xa4`, `grip_air 0xa8` |
| `Brakes` | `HandlingXml_ParseBrakes` (`0x0883962c`) | `gain 0xac`, `amount 0xb0`, `falloff 0xb4` |
| `Engine` | `HandlingXml_ParseEngine` (`0x0883945c`) | `gain 0xb8`, `amount 0xbc`, `falloff 0xc0`, `accelcap 0xc4`, `turbo 0xc8` |
| `Turning` | `HandlingXml_ParseTurning` (`0x088398e0`) | `gain 0xcc`, `amount 0xd0`, `falloff 0xd4` |
| `Airbrake` | `HandlingXml_ParseAirbrake` (`0x08839a04`) | `gain 0xd8`, `falloff 0xdc`, `amount 0xe0`, `turn 0xe4`, `drag 0xe8`, `slidegrip 0xec`, `sideshift 0xf0` |
| `Physical` | `HandlingXml_ParsePhysical` (`0x08838f50`) | `mass 0xf4`, `normal_gravity 0xf8`, `flight_gravity 0xfc`, `track_gravity 0x100` |
| `pitch` | `HandlingXml_ParsePitch` (`0x0883977c`) | `pitch_air 0x104`, `pitch_ground 0x108`, `pitch_damping 0x10c`, `antigrav_height_adjust 0x110` |

Confidence **90**.

**The arithmetic closes exactly.** 6 + 3 + 5 + 3 + 7 + 4 + 4 = 32 fields; 32
floats is `0x80` bytes; the observed per-class stride is `0x80`; the first field
sits at `0x94` and the last at `0x110`, so the block runs `0x94 .. 0x114` and
class *n* + 1 starts precisely where class *n* ends. **There is no gap and no
overlap anywhere in the table, and no offset in the range is unaccounted for.**
Independently, `Ship_UpdateCraft` caches `stats + class * 0x80 + 0x94` in a
member and every consumer reads through that pointer, which is only consistent
if `0x94` is the block base.

Not higher than 90 because nothing here has been observed at runtime: the
mapping shows the loader and the consumers agree on a layout, not that a real
file produces the values a race then uses.

Two of the four unlabelled string references were read out of memory rather than
inferred: `0x08a7b164` is `"gain"`, `0x08a7b084` is `"mass"`, `0x08a7b1d0` is
`"drag"`, `0x08a7b1d8` is `"turn"`. So every one of the 32 rows above is a
name comparison the loader actually performs.

Note that memory order is **not** XML document order: `Antigrav` and `Brakes`
precede `Engine`, and `Airbrake` follows `Turning`. Nor is field order within a
block consistent between blocks: `Engine`, `Brakes` and `Turning` are
`gain, amount, falloff`, but `Airbrake` is `gain, falloff, amount`. A decoder
that assumed either regularity would place five fields wrongly.

### Four parameters are pre-scaled at load

The parsers do not store four of the values verbatim. This answers the **Units**
open question in [handling stats](../../../formats/handling-stats.md), which that
page could not settle from the XML alone:

| Field | Stored as | Site |
| --- | --- | --- |
| `Engine.amount` | `xml * 0.001` | `0x0883949c` region, in `HandlingXml_ParseEngine` |
| `Brakes.amount` | `xml * -0.01` | in `HandlingXml_ParseBrakes` |
| `Airbrake.amount` | `xml * 0.0001` | in `HandlingXml_ParseAirbrake` |
| `Airbrake.slidegrip` | `xml * 0.0001` | in `HandlingXml_ParseAirbrake` |

Confidence **88**. Two consequences worth stating outright:

- **`Brakes.amount` is negative in memory.** The brake term applies force along
  `+normalize(velocity)`, so the sign lives in the parameter, not in the force
  law. A reimplementation that negated at the use site as well would accelerate
  under braking.
- **`slidegrip` is confirmed as "percent of grip retained".** With the `1e-4`
  scaling, an XML value on 0..100 becomes 0..0.01, and the grip term's
  `(0.01 - slidegrip)` therefore runs from `0.01` at `slidegrip = 0` down to
  exactly `0` at `slidegrip = 100`. Combined with the `max(L, R)` factor
  (airbrake state is also 0..100), the coefficient reaches exactly 0 at full
  airbrake and `slidegrip = 0`, and stays at `-1` for `slidegrip = 100`. This
  was read as an interpretation in [physics](../../../physics/README.md) at
  confidence 86; the load-time factor makes it arithmetic. Confidence **90**.

`AirbrakeGraphics` lives on the stats base rather than in a class block:
`up_speed 0x70`, `down_speed 0x74` (`HandlingXml_ParseAirbrakeGraphics`,
`0x08839c68`), driving the two graphics-only deflection states at `craft+0x2d8`
and `craft+0x2dc`. Confidence **80**. `Ship_UpdatePitch` also reads
`stats_base + 0x90`, a per-team scalar outside every class block; its element is
not determined.

## The frame: `Ship_UpdateCraft`

`Ship_UpdateCraft` (`0x08849618`) is the whole per-frame craft update, and every
force term below is reached from it. Signature, from register use:

```c
void Ship_UpdateCraft(float dt /* f12 */, Craft *craft /* a1 */, RigidBody *body /* a2 */);
```

It stashes `dt` at `craft+0x1c8` and `body` at `craft+0x1cc`, copies the body's
4x4 transform, linear velocity and position into the craft, zeroes the four
accumulators, then runs the terms in a fixed order and finally hands each
accumulator to the body.

Row conventions, established by cross-checking three independent uses (the
raycast direction, the `|dot(v, row2)|` speed scalar, and the lateral grip term's
`dot(v, row0)`): **row 0 is right, row 1 is up, row 2 is forward.** Handedness
and units are still not determined.

Order of terms, all of which write only accumulators unless noted:

| Order | Function | Writes |
| ---: | --- | --- |
| 1 | `Ship_UpdateEngine` (`0x0884c5c8`) | local `.z`, local `.y` |
| 2 | `Ship_UpdateBrakes` (`0x088489d8`) | world `.x`, `.z` |
| 3 | `Ship_UpdateAirbrakes` (`0x0884c9a4`) | world `.xyz`, angular-local `.y`; `sideshift` straight to the body |
| 4 | `Ship_UpdateSteering` (`0x08848788`) | angular-local `.y` |
| 5 | `Ship_UpdatePitch` (`0x08848d08`) | angular-local `.x` |
| 6 | `Ship_ApplyQuadraticDrag` (`0x08848e28`) | world `.xyz` |
| 7 | inline gravity | world `.y` |
| 8 | `Ship_UpdateHover` (`0x0884870c`) | see [physics](../../../physics/README.md) |
| 9 | `Ship_ApplyLateralGrip` (`0x08848b78`) | body local-force accumulator directly |
| 10 | inline dead branch | nothing |
| 11 | `Ship_ApplyWeathervaneTorque` (`0x08848dc4`) | angular-world `.xyz` |
| 12 | `Ship_ApplyAngularDamping` (`0x08848ed0`) | angular-local `.xyz` |
| 13 | `Ship_ApplyRollingResistance` (`0x08848f4c`) | world `.xyz` |
| 14 | inline vertical damping | world `.xyz` |
| 15 | track-section force (`0x08848f9c`) | world `.xyz` |

Steps 1 to 6 are skipped entirely when `craft+0x2a4` (a mode or lifecycle enum)
is 4, 5, 6 or 8, or when the control-input pointer is null. Step 2 is
additionally gated on grounded and on not being in the four-corner mode. Step 9
only runs when the timer at `craft+0x2e0` has expired.

Confidence **84** for the ordering, which is read straight off one basic-block
sequence. `get_function_callers` finds **no direct caller**, so the call is
almost certainly through a vtable; "once per frame per craft" is therefore an
inference from the structure, not something located. That is the single cheapest
thing that would raise this page.

## The four accumulators

The craft holds four 16-byte accumulators, zeroed at the top of every
`Ship_UpdateCraft` and drained at the bottom into the rigid body:

| Craft | Body | Frame | Meaning |
| --- | --- | --- | --- |
| `+0x320` | `+0x110` | body-local | Force. `x` right, `y` up, `z` forward |
| `+0x330` | `+0x100` | world | Force |
| `+0x340` | `+0x120` | body-local | Angular. `x` pitch, `y` yaw, `z` roll |
| `+0x350` | `+0x130` | world | Angular |

`+0x320` is added to `body+0x110` inline with a single `vadd.q`; the other three
go through one-line helpers, `Body_AddForceWorld` (`0x0884d4c8`),
`Body_AddTorqueLocal` (`0x0884d5bc`) and `Body_AddTorqueWorld` (`0x0884d604`),
each of which is nothing but `body[off] += *v`. Confidence **82** for the force
pair, **74** for the two angular ones: the local/world distinction rests on
which frame the terms feeding each are expressed in (`Ship_UpdatePitch` feeds
`+0x340` a scalar times a body-axis component; `Ship_ApplyWeathervaneTorque`
feeds `+0x350` a cross product of two world vectors), not on reading the
integrator.

**Whether the two angular accumulators hold torque or angular acceleration is
not determined.** Nothing in any term visibly divides by an inertia, and
[physics](../../../physics/README.md) calls this `angAccelLocal`. The `AddTorque`
names are chosen for the `Subsystem_VerbNoun` scheme and should not be read as
settling the question; the integrator was not examined.

### Every write, by component

Enumerated from the `Ship_UpdateCraft` call tree. Component names use the row
convention above.

**Local force `+0x320`:**

- `.z += thrust` and `.y += boostLift` - `Ship_UpdateEngine`, the only two
  writes, both via `vadd.s` on single components.
- `.x += grip` - `Ship_ApplyLateralGrip`, written to `body+0x110` directly
  rather than through the craft accumulator, twice (a grounded and an airborne
  term).

**World force `+0x330`:** brakes, the airbrake lateral and slide terms, gravity,
quadratic drag, rolling resistance, the hover spring epilogue's downforce, the
vertical-damping term and the track-section force. All whole-vector `vadd.t`.

**Angular local `+0x340`:** `.y += airbrakeYaw` (`Ship_UpdateAirbrakes`),
`.y += steerYaw` (`Ship_UpdateSteering`), `.x += pitch` (`Ship_UpdatePitch`),
`.y += 30 * right.y * (1 - magLockBlend)` (hover epilogue), and
`.xyz += (-pitch_damping, -5, k) * bodyAngularVelLocal`
(`Ship_ApplyAngularDamping`, `k` is `-5` in mode 0 and `-2` otherwise).

**Angular world `+0x350`:** the surface-alignment torque (hover epilogue) and
the weathervane torque.

### The roll negative, stated precisely

[physics](../../../physics/README.md) says "airbrakes produce no direct roll
torque. No Z component is ever written." The first sentence holds; **the second
is too strong and should be narrowed when that page is next edited**:

- **No control input writes an angular Z component.** Not the engine, not the
  brakes, not the steering, not the airbrakes, not the pitch axis. Roll is never
  commanded.
- Angular Z *is* written, in exactly two places: `Ship_ApplyAngularDamping`
  (`-2.0` or `-5.0` times the body's local angular velocity `z`) and, via the
  world accumulator, the surface-alignment torque. Both are passive.

Confidence **80** on the narrowed negative. It rests on reading every writer in
the `Ship_UpdateCraft` call tree, and it is weaker than it could be because
`Ship_HoverFourCorner` (`0x0884ae90`) and `Ship_UpdateMagLock` (`0x0884ba0c`)
were not read line by line. A mechanical check partly covers them:
`Ship_HoverFourCorner`'s only accumulator base address is `craft+0x340`, so it
touches no other accumulator. The equivalent scan of `Ship_UpdateMagLock`
truncated, so it is **not** covered, and any Z write there would be missed.

## Engine

`Ship_UpdateEngine` (`0x0884c5c8`). Reads `controls->thrust` at
`*(craft+0x78) + 0x04`; `grounded` is the 0/0.5/1 fraction at `craft+0x2b0`;
`speed` is `|dot(velocity, forward)|`, cached at `craft+0x2ec`.

```
throttle = controls.thrust                  // NOT ramped, see below
T = throttle * Engine.amount                // amount already * 1e-3 at load
T = T * grounded + T * 0.2 * (1 - grounded) // 20 % thrust with no contact

if (normal mode) {
    cap = 0.5 * speed + Engine.accelcap
    if (throttle > 100.0)  cap = throttle * 0.01 * cap
    if (flags & 0x0008)    cap = 1e10                   // uncapped
    T = min(T, cap)
} else {                                                // four-corner mode
    T = g_autospeed_base + g_autospeed_step * (float)craft+0x28c
}

if (flags & 0x0004)  T *= craft+0x2a0                   // a scalar multiplier
lift = 0
if ((flags & 0x0200) || (flags & 0x0400)) and craft+0x2a4 == 1 {
    T += Engine.turbo
    if (controls.buttons & 1)  lift = g_boost_lift * T
}
T = T * craft+0x294 * 2.0
if (craft+0x31c < 1.0) { T *= craft+0x31c; craft+0x31c = 1.0 }  // one-shot scale
if (flags & 0x2000) { throttle = 0; T = 0; lift = 0 }

localForce.z += T
localForce.y += lift
```

Confidence **84**.

### Engine `gain` and `falloff` are dead

The function computes a per-second ramp of the throttle state toward the input,
in exactly the shape the airbrakes use, stores it, and then **overwrites it with
the raw input two instructions later**. Verified in disassembly rather than in
the decompiler, because this is precisely where Ghidra's branch handling could
mislead:

```
0884c700: c.lt.s f12,f13          # ramped < 0 ?
0884c708: bc1f   0x0884c714
0884c70c: swc1   f12,0x2b8(a0)    # delay slot: store the ramped value
0884c710: swc1   f13,0x2b8(a0)    # ... or zero
0884c714: lw     a1,0x78(a0)      # a1 = controls
0884c720: lwc1   f12,0x0(a1)      # f12 = controls.thrust
0884c728: swc1   f12,0x2b8(a0)    # throttleState = raw input, unconditionally
0884c72c: lwc1   f15,0x28(a2)     # Engine.amount
0884c734: mul.s  f12,f12,f15      # and the raw input is what gets multiplied
```

Both branches of the ramp land on the store at `0x0884c728`, and the value the
thrust is computed from is the one loaded at `0x0884c720`, not the ramped state.
So `Engine.gain` (`+0xb8`) and `Engine.falloff` (`+0xc0`) **are read from the XML
and have no effect on the simulation.** Confidence **85**.

Worth flagging loudly, because the obvious reimplementation is to give the engine
the same ramp the airbrakes have. It would be wrong, and the resulting throttle
lag is exactly the kind of difference that reads as "feels close enough".

### The Y-component add reads a register the function never loads

The final accumulate is:

```
0884c978: addiu a0,a0,0x320
0884c97c: lv.s  S222,0x8(a0)      # accumulator .z
0884c980: lv.s  S600,0x0(sp)      # T
0884c988: lv.s  S601,0x0(a1)      # lift
0884c98c: vadd.s S222,S222,S600
0884c990: vadd.s S221,S221,S601   # S221 was never loaded here
0884c994: sv.s  S222,0x8(a0)
0884c998: sv.s  S221,0x4(a0)
```

`S221` (the accumulator's `.y` slot in the same VFPU column) is never loaded from
memory in this function; Ghidra names it as an input register. The plausible
reading is that the caller's `vzero.q` over `craft+0x320` left that element at
0.0 and the compiler exploited it across the call boundary, making the observable
result `accumulator.y = lift`. **Confidence 65 on that explanation**, and it is
recorded because a reimplementation should write `accumulator.y = lift` rather
than try to reproduce a register carry. If the register is ever *not* zero, this
is a live bug in the original.

## Brakes

`Ship_UpdateBrakes` (`0x088489d8`). Called from `Ship_UpdateCraft` only when
`(flags & 1) != 0`, i.e. **braking does nothing in the air**, and only outside
the four-corner mode.

There is no dedicated brake axis. The brake engages when **both airbrake inputs
are positive at once**:

```
if (controls.airbrakeL > 0 && controls.airbrakeR > 0)
    brake = min(brake + Brakes.gain * dt, 100.0)
else
    brake = max(brake - Brakes.falloff * dt, 0.0)

if (brake > 0) {
    speed = |velocity|
    dir   = speed != 0 ? velocity / speed : velocity
    if (speed < 10.0)  dir *= speed * 0.1          // fade out at low speed
    worldForce += dir * Brakes.amount * brake      // amount is negative
}
```

`brake` lives at `craft+0x2bc`, next to the throttle (`+0x2b8`), steering
(`+0x2c0`) and the two airbrake states (`+0x2c4`, `+0x2c8`). Confidence **82**.

Unlike the engine, the brake's `gain`/`falloff` ramp is **live**: nothing
overwrites `craft+0x2bc` afterwards.

A VFPU target prefix (`vpfxt`) sits on the accumulate. Read as the standard
per-component selector it zeroes the `y` and `w` lanes, which would mean the
brake force is horizontal-only in whatever frame the world accumulator uses.
**Confidence 55**: the prefix encoding was not confirmed against a reference, so
this is recorded as a caution rather than a claim. It is also low-stakes -
braking only runs while grounded, where the velocity is close to horizontal, so
zeroing the world `y` lane changes very little either way.

## Steering

`Ship_UpdateSteering` (`0x08848788`). Reads the analog X axis at
`*(craft+0x78) + 0x00`.

```
target = controls.steerX
if (target > 0)
    steer = (steer < target) ? steer + Turning.gain    * dt
                             : steer - Turning.falloff * dt
else if (target < 0)
    steer = (steer > target) ? steer - Turning.gain    * dt
                             : steer + Turning.falloff * dt
else {                                        // decay to exactly zero
    if (steer > 0) { steer -= Turning.falloff * dt; if (steer < 0) steer = 0; }
    if (steer < 0) { steer += Turning.falloff * dt; if (steer > 0) steer = 0; }
}
if (craft+0x2a4 == 5 || craft+0x2a4 == 6)  steer = 0

yaw = steer * Turning.amount
if ((flags & 0x20) && *(craft+0x1c4) + 0x368 == 0)
    yaw = (steer + craft+0x2e4) * Turning.amount     // an added steering bias

if ((flags & 0x40) && *(craft+0x1c4) + 0x368 == 0) { // reversed controls
    yaw = craft+0x2e8 > 1.0 ? -yaw
                            : yaw * (1.0 - 2.0 * craft+0x2e8)
}

angularLocal.y += yaw
```

Confidence **84**. Two details worth keeping:

- **`gain` and `falloff` are asymmetric by intent, not by sign.** `gain` is the
  rate while moving *toward* a larger-magnitude target and `falloff` the rate
  while moving back toward centre, on both sides of zero. So the same pair of
  parameters gives a fast bite and a slow return, or the reverse.
- **`craft+0x2e8` is a 0-to-1 blend into reversed steering, not a boolean.** At
  0 steering is normal, at 0.5 it is dead, past 1 it is fully inverted. A
  reimplementation that treated the reverse-controls pickup as a flag would snap
  where the original ramps.

`Turning.amount` feeds body-local yaw acceleration directly, with no speed
factor. Steering authority at a standstill is therefore not zero, and any
speed-dependence must come from the damping and grip terms.

## Pitch

`Ship_UpdatePitch` (`0x08848d08`). Reads the pitch axis at
`*(craft+0x78) + 0x10`.

```
p = 0
if (FUN_088492bc(craft))                       // gate, not decoded
    p = controls.pitch * (grounded ? pitch_ground : pitch_air)
if (!grounded)
    p += *(craft+0x6c) + 0x90                  // per-team in-air pitch bias
angularLocal.x += p
```

Confidence **82**. `pitch_air` and `pitch_ground` are plain gains on the input
axis, not rates: there is no ramp and no state. `pitch_damping` is consumed
elsewhere, by `Ship_ApplyAngularDamping`.

The gate at `0x088492bc` was not decoded; without it, pitch input would apply
unconditionally.

## The passive terms

Each of these is small, always on, and easy to overlook. All carry confidence
**76-80**, decompilation only.

**Quadratic drag**, `Ship_ApplyQuadraticDrag` (`0x08848e28`):

```
k = craft+0x2a4 == 0     ? -0.9
  : forwardSpeed < -0.2  ? -0.1
  : grounded             ? -0.005
  :                        -0.002
worldForce += velocity * forwardSpeed * k
```

Note the airborne coefficient is **smaller** than the grounded one, and that
reversing (`forwardSpeed < -0.2`) multiplies drag by 20 to 50 times, which is
what makes the ship reluctant to fly backwards.

**Rolling resistance**, `Ship_ApplyRollingResistance` (`0x08848f4c`): when
`forwardSpeed > 0`, adds `2 * unit(velocity)` under a source prefix that negates
all three lanes, so `-2 * unit(velocity)`. A constant-magnitude opposing force,
independent of speed and of every handling parameter.

**Weathervane torque**, `Ship_ApplyWeathervaneTorque` (`0x08848dc4`):

```
angularWorld += cross(forward, velocity) * (grounded ? -0.1 : -0.3)
```

Three times stronger in the air. This, and not any explicit control term, is
what turns the nose toward the direction of travel.

**Angular damping**, `Ship_ApplyAngularDamping` (`0x08848ed0`):

```
angularLocal += (-pitch_damping, -5.0, k) * bodyAngularVelocityLocal
                                       // k = -5.0 in mode 0, else -2.0
```

Only the pitch axis is tunable per ship. Yaw damping is a hard `-5.0` and roll
damping a hard `-2.0` for every craft in the game.

**Vertical damping**, inline in `Ship_UpdateCraft`:

```
worldForce += up * dot(up, velocity) * -0.25 * (1 - magLockBlend)
```

**Gravity**, inline in `Ship_UpdateCraft`:

```
worldForce.y += -( normal_gravity * g_class_gravity_scale[class] * mass * grounded
                 + flight_gravity                               * mass * (1 - grounded) )
```

as a single `vmul.p` pair chain with a source prefix negating both lanes.
`normal_gravity` applies on the ground and `flight_gravity` in the air, blended
by the same 0/0.5/1 grounded fraction, and only `normal_gravity` gets the
per-class scale from `0x08ab0dcc`. `mass` is read from `body+0x374`, not from the
XML `mass` at `+0xf4`; how the two relate is not determined.

The negation matters and is checkable without trusting the prefix encoding: the
hover spring multiplies by `normal_gravity + track_gravity` and must push the
ship **up**, so both are positive in the data. A downward gravity force therefore
requires the sign to be applied here, in the code. The same prefix bit pattern
appears on the rolling-resistance term, which must also oppose motion.

## Contradictions with `docs/physics/README.md`

**`ride_height` does appear in the force law.** That page states it "never
appears in the force law. It is only the raycast length; the height the ship
settles at is emergent." The offset chain says otherwise:

- `HandlingXml_ParseAntigrav` stores `ride_height` at `+0x94`.
- `Ship_UpdateCraft` sets `craft+0x70 = stats + class * 0x80 + 0x94`, so
  `**(float**)(craft+0x70)` *is* `ride_height`.
- `Ship_UpdateCraft` then computes
  `craft+0x2f0 = (ride_height + craft+0x74 - leapAdjust) * (1 + 0.2 * magLockBlend) * K2`
  where `leapAdjust` is `min(craft+0x2e0, 4.0)` while that timer runs and `K2` is
  the global at `0x08ab0e1c`.
- `Ship_HoverTwoPoint` uses `craft+0x2f0` as the spring's target height:
  `(craft+0x2f0 - h) * ...`.

So `ride_height` is the primary term of the hover target height and scales the
spring force directly. Confidence **88**, resting on the parser evidence rather
than on a reading of the consumer. What `craft+0x74` is, and who writes it, is
**not determined** - it is an additive height offset, and `antigrav_height_adjust`
is an obvious suspect but nothing was found that copies it there.

**The four-corner selector is `DAT_08ab07e3 == 0 && DAT_08b31048 == 6`.** That
page lists "a four-corner hover variant exists alongside the two-point one; which
is used when was not established" as open. `Ship_UpdateHover` (`0x0884870c`) is
three lines: it calls `Ship_HoverFourCorner` under exactly that condition and
`Ship_HoverTwoPoint` otherwise, then `Ship_UpdateMagLock` unconditionally. The
same condition gates the brakes off and swaps the engine for an auto-speed law
driven by a counter at `craft+0x28c`, which reads like a game mode rather than a
ship or track property. Confidence **84** for the selector, **50** for it being
a specific mode, so the globals are labelled but not interpreted.

**Groundedness is one frame stale for every control term.** That page records
`0.75 * grounded_prev + 0.25` for the hover spring's load factor, so the lag was
noticed there. It is not local to the spring: it splits the force law in two.

`Ship_UpdateHover` clears flag bit 0 (`flags &= 0xfffffffe`) as its first
statement, and `Ship_HoverTwoPoint` zeroes `craft+0x2b0` on entry - after reading
the old value for the load factor - then re-accumulates one half per contacting
probe and ORs bit 0 back in. Hover is **step 8** of fifteen. So, from the
ordering table above:

| Reads | Sees |
| --- | --- |
| `Ship_UpdateEngine` (`craft+0x2b0`, the 20 %-in-air blend) | **last frame** |
| `Ship_UpdateBrakes` (gated on `flags & 1` by the caller) | **last frame** |
| `Ship_ApplyQuadraticDrag` (`flags & 1` selects the coefficient) | **last frame** |
| inline gravity (`craft+0x2b0` blends normal against flight) | **last frame** |
| `Ship_UpdatePitch` (`flags & 1` selects air or ground gain) | **last frame** |
| `Ship_ApplyLateralGrip` (`craft+0x2b0`) | this frame |
| `Ship_ApplyWeathervaneTorque` (`flags & 1`) | this frame |

Confidence **84**, from the call order and the two clearing sites. The obvious
reimplementation - resolve contacts, then apply forces - gets a different answer
on every takeoff and landing frame, in the engine, the brakes, gravity, drag and
pitch simultaneously. Unlike the dead roll-levelling branch, this one changes
trajectories.

**The dead in-air roll-levelling term is confirmed**, and it is inline in
`Ship_UpdateCraft` rather than in a hover function: under `(flags & 1) == 0` it
computes `5 * right.y`, a comparison of `up.y` against zero, a `vcmovf`, a
`vscl.t` of the forward axis and a final `vadd.t` whose result is **never
stored**. Same conclusion, now with the location.

## Not determined

- **Nothing on this page is runtime-verified.** Per the
  [rubric](../../../reverse-engineering/confidence-rubric.md), decompilation
  alone caps at 84, and no claim about the force law here exceeds that. Only the
  parameter block layout scores higher, because it has a second source (the XML
  schema) and an exact arithmetic invariant.
- **The call site of `Ship_UpdateCraft`.** No direct caller exists; the dispatch
  is presumably a vtable. Until it is found, "once per frame per craft, before
  integration" is structural inference.
- **The `craft+0x2a4` enum.** Values 0 through 8 are compared against. 0 disables
  the `rebound` parameter and multiplies drag by ~180x, 1 is required for turbo,
  3 skips the accumulator drain entirely, and 4, 5, 6 and 8 skip the control
  block. A front-end display mode and a racing mode are the obvious guesses, at
  confidence 40, so it is deliberately unnamed.
- **`FUN_088492bc`**, the gate on pitch input.
- **`FUN_08848f9c`**, which locates the craft's track section, updates lap
  bookkeeping, and adds a world force along the section's forward axis with a
  per-class magnitude from `0x08b36bc0`/`0x08b36bd0`. Its timer is re-armed every
  frame a section is found, so the force looks continuous rather than one-shot.
  Whether that is a rail assist, a catch-up mechanism or something else is a
  guess at confidence 45; it is described here and not named.
- **Any reader of `antigrav_height_adjust` (`+0x110`).** A search for scalar FPU
  loads at the block-relative offset `0x7c` found none in the craft path. That is
  a weak negative - it does not cover VFPU `lv.s` loads or a copy through a
  cached member - so "parsed but never consumed" is a hypothesis at confidence
  50, not a finding.
- **The `mass` at `+0xf4` versus the mass at `body+0x374`.** Every force term
  uses the latter. Whether ship construction copies one to the other, or scales
  it, was not traced.
- **The VFPU prefix encoding.** Three claims lean on it: gravity's negation,
  rolling resistance's negation, and the brake force's zeroed `y` lane. The first
  two are corroborated by an independent sign argument (above); the third is not,
  and sits at confidence 55.
- **`Ship_UpdateMagLock`'s writes.** The mechanical scan for accumulator base
  addresses truncated, so the "no commanded roll" negative does not cover it.
- **The `flags` word at `craft+0x1c0`.** Eleven bits are used across this path
  (`0x1, 0x2, 0x4, 0x8, 0x10, 0x20, 0x40, 0x80, 0x100, 0x200, 0x400, 0x800,
  0x1000, 0x2000`). Only bit 0 is established (grounded, set by the hover probes).
  The rest are described where they appear and not named.
- **Units and handedness.** Still open, and this page does not settle them.

## Applied renames

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0883945c` | `HandlingXml_ParseEngine` | 90 |
| `0x0883962c` | `HandlingXml_ParseBrakes` | 90 |
| `0x088398e0` | `HandlingXml_ParseTurning` | 90 |
| `0x08839a04` | `HandlingXml_ParseAirbrake` | 90 |
| `0x08839278` | `HandlingXml_ParseAntigrav` | 90 |
| `0x08838f50` | `HandlingXml_ParsePhysical` | 90 |
| `0x0883977c` | `HandlingXml_ParsePitch` | 90 |
| `0x0883a2f0` | `Handling_ParseStats` | 88 |
| `0x08849618` | `Ship_UpdateCraft` | 82 |
| `0x0884c5c8` | `Ship_UpdateEngine` | 84 |
| `0x08848788` | `Ship_UpdateSteering` | 84 |
| `0x0884c9a4` | `Ship_UpdateAirbrakes` | 84 |
| `0x088489d8` | `Ship_UpdateBrakes` | 82 |
| `0x08848d08` | `Ship_UpdatePitch` | 82 |
| `0x0884870c` | `Ship_UpdateHover` | 82 |
| `0x0884d4c8` | `Body_AddForceWorld` | 82 |
| `0x08839c68` | `HandlingXml_ParseAirbrakeGraphics` | 80 |
| `0x08848b78` | `Ship_ApplyLateralGrip` | 80 |
| `0x08848e28` | `Ship_ApplyQuadraticDrag` | 80 |
| `0x08848ed0` | `Ship_ApplyAngularDamping` | 80 |
| `0x08848f4c` | `Ship_ApplyRollingResistance` | 76 |
| `0x0884d5bc` | `Body_AddTorqueLocal` | 74 |
| `0x0884d604` | `Body_AddTorqueWorld` | 74 |
| `0x08848dc4` | `Ship_ApplyWeathervaneTorque` | 74 |
| `0x08b36bfc` | `g_handling_parse_class` | 85 |
| `0x08ab0dcc` | `g_class_gravity_scale` | 78 |

## Cross-platform

| Platform | Notes |
| --- | --- |
| PSP (Pulse) | This page |
| PS2 (Pulse) | Not located. Comparing `HandlingXml_ParseEngine`'s equivalent would confirm the `0x80` stride and the four load-time scale factors in a second binary, which is the cheapest available route from 90 to the mid-90s |
| PSP (Pure) | Not located |

## History

- 2026-07-26: first pass. Parameter block layout 90 from the XML loader; force
  law 74-85 from decompilation; nothing runtime-verified.
