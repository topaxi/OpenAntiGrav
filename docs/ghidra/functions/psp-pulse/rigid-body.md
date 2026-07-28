# The rigid body: construction, force application and the integrator

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`. **The names here are applied**, from [names.tsv](names.tsv).

[engine.md](engine.md) documents what `Ship_UpdateCraft` puts *into* the four
accumulators. This page documents what happens to them afterwards, which had
never been read on the PSP side: the constructor that establishes the body's
defaults, the helper that turns a force at a point into a force plus a torque,
and the integrator itself.

All three were read with the [Allegrex processor
module](../../../psp/allegrex-vfpu.md) installed, so the VFPU instructions
(`vtfm3.t`, `vscl.t`, `vcrsp.t`, `vfad.t`, `vrsq.s`) decode as instructions
rather than as invalid bytes. That is the difference from the earlier
capstone-only passes over this area.

## `Body_AddForceAtPoint` (`0x0884d510`)

Twelve instructions plus two calls:

```text
0884d530  jal   Body_AddForceWorld        ; body+0x100 += force
0884d538  a0 = body+0x30                  ; position
0884d554  vsub.q C320,C300,C310           ; r = point - position
0884d570  C300 = *force
0884d580  vcrsp.t C320,C300,C310          ; torque = cross(force, r)
0884d59c  jal   Body_AddTorqueWorld       ; body+0x130 += torque
```

Signature `void Body_AddForceAtPoint(RigidBody *body, const Vec4 *force, const
Vec4 *point)`. Confidence **88**.

**The cross product is `cross(force, r)`, not the textbook `cross(r, force)`.**
That is the same negated convention the PS2 integrator shows in
[craft-update.md](../ps2-pulse/craft-update.md#the-angular-sign-convention-w_game---w_physics),
and it is a second, independent witness for it on the PSP side: the engine's
angular quantities are `-w_physics` throughout, so a hover spring pushing up at
the nose produces a torque with the sign this engine expects and nothing needs a
compensating flip. It is the only caller-visible consequence of the convention on
this page.

`Ship_HoverTwoPoint` is the caller that matters - it applies each hover point's
spring force at that point's world position through this helper, which is where
the suspension's pitch and roll response comes from.

## `Body_Init` (`0x0884de5c`)

Zeroes all four accumulators (`+0x100`, `+0x110`, `+0x120`, `+0x130`), sets the
transform, its transpose and the two matrices at `+0x40` and `+0xc0` to identity,
zeroes both velocities (`+0x140`, `+0x160`) and the `+0x150` twin, and then:

| Offset | Value | Instruction |
| --- | --- | --- |
| `+0x374` mass | `1.0` | `0x0884ded8` |
| `+0x378` invMass | `1.0` | `0x0884dedc` |
| `+0x380` angular damping | `0.0` | `0x0884e178` |
| `+0x384` linear damping | `0.0` | `0x0884e17c` |
| `+0x388` | `0.5` | `0x0884e188` |
| `+0x38c` | `1.0` | `0x0884e18c` |
| `+0x398` | `0.0` | `0x0884e1a0` |

Confidence **85**. The two damping defaults being *zero* here matters: a body
only damps if something else sets those fields, and for a craft that something is
the ship-entity constructor `FUN_08840c74`, which writes `0.01` to both at
`0x088414a0`/`0x088414ac`. So the damping is a per-entity choice, not a physics
default.

## `Body_Integrate` (`0x0884e230`)

```c
int Body_Integrate(RigidBody *body /* a0 */, int subSteps /* a1 */, float dt /* f12 */);
```

`h = dt / (float)subSteps` (`0x0884e270`), and the whole body of the function is
one loop running `subSteps` times. Everything it needs is hoisted above the loop
into VFPU registers; only the two *world* accumulators are re-read inside it.

Per sub-step, in program order:

```text
position        += velocity * h                          ; 0884e2ec-0884e2f4
basis            = basis + basis * skew(angularWorld * h) ; 0884e2f8-0884e334
velocity        += worldForce * h * invMass              ; 0884e338-0884e340
velocity        += (basis * localForce) * invMass * h    ; 0884e344-0884e350
angularVel      += localTorque * h                       ; 0884e354-0884e358
angularVel      += (basis^T * worldTorque) * h           ; 0884e35c-0884e364
velocity        -= velocity   * h * body+0x384           ; 0884e368-0884e370
angularVel      -= angularVel * h * body+0x380           ; 0884e374-0884e37c
basis            = orthonormalise(basis)                 ; 0884e3a0-0884e3d8
```

and after the loop it writes the basis back, rebuilds the transpose at `+0xc0`,
and stores `sqrt(dot(velocity, velocity))` to `body+0x398`.

Confidence **88**: every line above is a single decoded VFPU instruction or a
short run of them, and the shape matches the PS2 `Body_Integrate` (`0x0015d088`)
that [engine.md](engine.md) already records, including the two damping terms.

Three things follow, and each closes something that was open.

### The linear damping really is `0.01`, and applying it per sub-step does not change that

`velocity -= velocity * h * c` with `h = dt / N`, run `N` times, is
`velocity * (1 - h*c)^N`, which to first order is `velocity * (1 - dt*c)` - the
same as one step of `dt`. So the sub-stepping does **not** multiply the damping
up, and the equivalent opposing force at `24` units/s is `0.01 * 24 = 0.24`
against the `~53` that
[the force balance](../../../physics/force-balance-ground-truth.md) is missing.
This is now confirmed on the PSP side at instruction level and not only inferred
from the PS2 build.

Worth recording how the field was found, because the obvious search fails: a
`search_instructions` scan for the literal displacement `0x384(` finds **stores
only**. Every VFPU access in this binary goes `addiu aN, base, disp` and then
`lv.s 0x0(aN)`, so a displacement scan cannot see the reads. Scanning for
`addiu` with `0x384` in the operands is what finds `0x0884e2b0`. **A "no reads
found" result from a displacement scan over a VFPU-heavy binary is not evidence
of a dead field.**

### `body+0x398` is `|linear velocity|`

```text
0884e450  vmul.t C010,C300,C300
0884e454  vfad.t S010,C010
0884e458  vsqrt.s S000,S010
0884e45c  mfv    a1,S000
0884e460  sw     a1,0x398(a0)
0884e468  sv.q   C300,0x0(t3)      ; t3 == body+0x140, the same vector
```

[engine.md](engine.md) records this field at confidence **60** as "`|velocity|`
to seven digits in one sample but about 4 % above it across a 200-tick trace, so
**it is not the speed**". Half of that is now settled and half is not:

- **It is the speed by construction.** The store at `0x0884e460` and the store of
  the velocity itself at `0x0884e468` take the same register. Confidence **90**.
- **The 4 % is therefore not evidence against the identity** - it is evidence
  that the velocity is changed between the end of `Body_Integrate` and the point
  the trace samples, or that the trace samples the two at different times. This
  is the same `1.0367` factor
  [force-balance-ground-truth.md](../../../physics/force-balance-ground-truth.md)
  records on the capture's `speed` column, so that column *is* this field, and
  the open question narrows from "what is this field" to "what touches the
  velocity after the integrator". `Ship_UpdateMagLock` and the contact resolver
  `FUN_0884e968` are the two candidates; neither has been ruled in.

### The angular quantities

The loop advances the basis rows by `e' = e + e * skew(w*h)`, built by scattering
`w*h` and `-w*h` into `C400`/`C410`/`C420` at `0x0884e308`-`0x0884e31c` and then
three `vtfm3.t`. Which of `e x w` and `w x e` that is depends on the scatter
pattern, which was not worked through element by element here; the PS2 reading
already settles it as `e x w`, and this page does not disturb that.

`body+0x150` is written after the loop as `E600 * body+0x160` (`0x0884e39c`),
i.e. the angular velocity carried through a matrix built from `+0x40` and the
basis - which is the `+0x160`-to-`+0x150` mapping
[craft-update.md](../ps2-pulse/craft-update.md) records as a lead. Confirmed to
exist on PSP; the frame it maps *to* is not established here.

## Not determined

- **What calls `Body_Integrate`, and with what `subStep` count.** The count is an
  argument, so it is a caller's choice and could differ between the race and the
  front end. Nothing on this page depends on it - see the damping argument above,
  which is sub-step invariant - but a reimplementation that wants bit-comparable
  intermediate state does.
- **`body+0x388` (`0.5`) and `body+0x38c` (`1.0`).** `+0x388` is read by the
  contact resolver `FUN_0884e968` at `0x0884eb28` and added to `1.0` before
  scaling a normal-direction relative velocity, which reads exactly like a
  restitution coefficient - but that function is not otherwise documented, so
  the field stays offset-named at confidence **55**.
- **`FUN_0884e968` itself**, the contact resolver, and `FUN_0884d64c`, which
  applies an impulse at a point and is what it calls. Both were read far enough
  to rule them out as sources of a linear-in-velocity drag; neither is renamed.
