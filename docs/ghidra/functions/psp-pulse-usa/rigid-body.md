# The rigid body: construction, force application and the integrator

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`. **The names here are applied**, from [names.tsv](names.tsv).

[engine.md](engine.md) documents what `Ship_UpdateCraft` puts *into* the four
accumulators. This page documents what happens to them afterwards, which had
never been read on the PSP side: the constructor that establishes the body's
defaults, the helper that turns a force at a point into a force plus a torque,
the integrator itself, and **the contact response that runs after it** - which is
where the M4 force-balance blocker's missing `~52` units turned out to live.

All of it was read with the [Allegrex processor
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
[craft-update.md](../ps2-pulse-eu/craft-update.md#the-angular-sign-convention-w_game---w_physics),
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
  the open question narrowed from "what is this field" to "what touches the
  velocity after the integrator". **That question is now answered**: it is
  `Body_ResolveContact` (`0x0884e968`), read below. `Ship_UpdateMagLock` is
  ruled out - it rewrites `body+0x140` but renormalises to the original
  magnitude, so it cannot change the speed.

### The angular quantities

The loop advances the basis rows by `e' = e + e * skew(w*h)`, built by scattering
`w*h` and `-w*h` into `C400`/`C410`/`C420` at `0x0884e308`-`0x0884e31c` and then
three `vtfm3.t`. Which of `e x w` and `w x e` that is depends on the scatter
pattern, which was not worked through element by element here; the PS2 reading
already settles it as `e x w`, and this page does not disturb that.

`body+0x150` is written after the loop as `E600 * body+0x160` (`0x0884e39c`),
i.e. the angular velocity carried through a matrix built from `+0x40` and the
basis - which is the `+0x160`-to-`+0x150` mapping
[craft-update.md](../ps2-pulse-eu/craft-update.md) records as a lead. **What that
matrix is, and therefore what each field means, is settled below.**

### `body+0x160` is angular momentum, `body+0x40` is the inverse inertia tensor

This closes the `I*w` reading that
[angular-velocity-column.md](../../../physics/angular-velocity-column.md) could
only reach by fitting, and it closes the roadmap's "torque or angular
acceleration" question with it. Four instruction groups carry the whole argument.

**The basis is advanced by `+0x150`, not `+0x160`.** `C310` is loaded from
`t2 = a0+0x150` at `0x0884e290`, and it is `C310` that gets scaled by `h` and
scattered into the skew matrix:

```text
0884e2f8  vscl.t C000,C310,S733     ; (body+0x150) * h
0884e300  vneg.t C010,C000
0884e308-0884e31c                   ; scatter +/- into C400/C410/C420
0884e320  vtfm3.t C000,E200,C400    ; basis * skew
0884e32c  vadd.t C200,C200,C000     ; basis += that
```

So **`+0x150` is the angular velocity**: it is the quantity that rotates the
frame, by definition.

**Torque accumulates straight into `+0x160`, with no inertia anywhere:**

```text
0884e354  vscl.t C500,C500,S733     ; body+0x120 (local torque) * h
0884e358  vadd.t C510,C510,C500     ; body+0x160 += that
0884e35c  vtfm3.t C000,M200,C520    ; basis^T * body+0x130 (world torque)
0884e360  vscl.t C000,C000,S733
0884e364  vadd.t C510,C510,C000     ; body+0x160 += that
```

No multiply by an inverse inertia, and no divide by mass - unlike the linear half
four instructions earlier, which does scale by `invMass` at `0x0884e33c`. **The
asymmetry is the tell.** `dL/dt = torque` needs no inertia; `dw/dt = torque`
would need one. So the accumulators at `+0x120` and `+0x130` hold **torque**, and
`+0x160` holds **angular momentum in the body frame**.

**The map between them is a similarity transform of the matrix at `+0x40`:**

```text
0884e380  vtfm3.t C000,E700,C200    ; E700 == matrix at body+0x40
0884e384  vtfm3.t C010,E700,C210    ;   times each basis row
0884e388  vtfm3.t C020,E700,C220
0884e38c  vtfm3.t C600,M200,C000    ; basis^T times the result
0884e390  vtfm3.t C610,M200,C010
0884e394  vtfm3.t C620,M200,C020
0884e398  vzero.s S513              ; w of body+0x160 := 0
0884e39c  vtfm3.t C310,E600,C510    ; body+0x150 := E600 * body+0x160
```

`E600 = basis^T * (body+0x40) * basis` is the textbook change of basis for a
tensor, rebuilt from scratch every sub-step because the basis moves. Therefore
**`body+0x40..0x70` is the body-space inverse inertia tensor `I^-1`**, and
`w = I^-1_world * L` is exactly what `0x0884e39c` computes. Confidence **88**.

**And the result is cached where the impulse path expects it.** After the loop
`E600` is stored to `body+0x80..0xb0` (`0x0884e3f4`-`0x0884e404`) and the basis
*transpose* to `body+0xc0..0xf0` (`0x0884e430`-`0x0884e440`, via row-wise `R2xx`
access of the same registers). Those are precisely the two matrices
`Body_ApplyImpulseAtPoint` uses - `+0xc0` to rotate a world-space torque into the
body frame, `+0x80` to redo the `+0x160`-to-`+0x150` map without rebuilding
anything. The four fields are one consistent scheme:

| Offset | Meaning |
| --- | --- |
| `+0x40..0x70` | inverse inertia tensor, **body space** - **but see the open contradiction below** |
| `+0x80..0xb0` | inverse inertia tensor, **world space** (derived, cached per sub-step) |
| `+0xc0..0xf0` | basis transpose (derived, cached per sub-step) |
| `+0x150` | angular velocity, the quantity that turns the basis |
| `+0x160` | **angular momentum**, body frame, what torque integrates into |

**Open contradiction, 2026-08-19, not resolved here - and a follow-up check
sharpened it rather than closing it.** A live check in
[contact-response.md](contact-response.md#weapon_postblastimpulse_q-0x0886794c-confidence-68),
under "What `T` actually is", read `body+0x50` off a real craft mid-tick, off a
body pointer confirmed by direct pointer equality, and it held a track-scale
value that changed with the craft's actual position - not tensor-shaped data.
A follow-up static read of `Body_SetBoxInertia`'s diagonal patch-back
(`+0x40`/`+0x54`/`+0x68`, stride `0x14`) then independently *confirmed* this
section's own row layout - the constructor's own diagonal arithmetic requires
row 1 to start at `+0x50` - which makes the runtime observation harder to
explain away, not easier: row 1's off-diagonal terms should be `0.0` forever
for an axis-aligned box and were visibly not. Neither reading is weak and
neither has been withdrawn; see that page for the full account of what is and
is not settled. Do not treat `body+0x50` specifically as confirmed tensor
storage until this is reconciled.

**So the trace's fitted per-axis factor is the inertia tensor, and the sign is the
`w_game = -w_physics` convention.** The recorded `+0x160` column is `I * w`, so
dividing it by a body-local angular velocity recovers the **diagonal of `I`** -
the fit's `~(-15, -21..-22, -14..-16)` - and `1 / |k_y| = 0.0452`, the value
`YAW_DRIVE_CALIBRATION` was fitted to, is the **yaw entry of `I^-1`**, i.e. one
element of the matrix at `body+0x40`. The "missing 22x yaw factor" is not a
missing force term or a calibration; it is the tensor this engine has had all
along. The writer is read in the next section, and the crate's constant is now
`oag_physics::forces::YAW_INVERSE_INERTIA`.

### Answered: `Body_SetBoxInertia` (`0x0884e1ac`) writes `body+0x40`

`Body_Init` (`0x0884de5c`) sets `+0x40` to **identity**, and an identity `I^-1`
would make `+0x150 == +0x160` and leave no per-axis factor at all - which the
captures rule out. The writer has now been found. It is 26 instructions, and it
is the textbook solid-box inertia:

```text
0884e1ac  mul.s  f13,f13,f13        ; y*y
0884e1b0  lui    a2,0x8a9
0884e1b4  mul.s  f14,f14,f14        ; z*z
0884e1b8  addiu  a1,a0,0x40
0884e1bc  lv.q   C400,0x7e0(a2)     ; 0x08a907e0, a 64-byte block of zeroes
0884e1c0  mul.s  f12,f12,f12        ; x*x
0884e1c4  sv.q   C400,0x0(a1)       ; body+0x40 := 0
0884e1cc  add.s  f15,f13,f14        ; y*y + z*z
0884e1d8  sv.q   C400,0x0(a2)       ; body+0x50 := 0
0884e1e4  add.s  f14,f12,f14        ; x*x + z*z
0884e1e8  sv.q   C400,0x0(a2)       ; body+0x60 := 0
0884e1f4  sv.q   C400,0x0(a2)       ; body+0x70 := 0
0884e1f8  add.s  f12,f12,f13        ; x*x + y*y
0884e1fc  lui    a1,0x4140          ; 12.0f
0884e200  lwc1   f16,0x374(a0)      ; the body's *current* mass
0884e204  mtc1   a1,f17
0884e208  mul.s  f15,f16,f15
0884e20c  div.s  f15,f17,f15        ; 12 / (m * (y*y + z*z))
0884e210  mul.s  f14,f16,f14
0884e214  div.s  f14,f17,f14        ; 12 / (m * (x*x + z*z))
0884e218  swc1   f15,0x40(a0)       ; I^-1 [0][0]
0884e21c  mul.s  f12,f16,f12
0884e220  div.s  f12,f17,f12        ; 12 / (m * (x*x + y*y))
0884e224  swc1   f14,0x54(a0)       ; I^-1 [1][1]
0884e228  jr     ra
0884e22c  _swc1  f12,0x68(a0)       ; I^-1 [2][2]
```

`I_xx = m (y^2 + z^2) / 12` is the solid rectangular cuboid. Note the tensor is
zeroed rather than set to identity - the constant at `0x08a907e0` was read and is
64 bytes of `0x00` - so the result is purely diagonal with `[3][3] = 0`.

Confidence **92**. Named `Body_SetBoxInertia`; the thin world-level wrapper
`World_SetBodyBoxInertia` (`0x0884e900`, 88) looks the body up in the world's
twelve-entry table and forwards, and `World_SetBodyMass` (`0x0884e8b0`, 88) is
the same shape over `Body_SetMass`.

#### The arguments are literals, and the mass is not the flying mass

Both wrappers have exactly one caller each, and `World_SetBodyBoxInertia`'s is
the ship-entity constructor `FUN_08840c74`. Read at instruction level:

| Where | What |
| --- | --- |
| `0x08841404`-`0x08841414` | `World_SetBodyMass(world, body, 0.9)` - `0x3f666666` |
| `0x08841470`-`0x0884148c` | `World_SetBodyBoxInertia(world, body, 12, 8, 12)` - `0x41400000`, `0x41000000`, `0x41400000` |

In that order, so the tensor is built with `m = 0.9`. Two consequences that are
not obvious and both matter:

- **The box is a code literal, identical for every craft in the game.** `Misc`
  `width`/`length`/`height` are read a few lines earlier in the same
  constructor, scaled by `0.75`, and handed to the *collider* setup
  (`FUN_0884e694`) - not to the inertia. So there is no per-team rotational
  inertia, and a single global constant in a reimplementation is the mechanism
  rather than a compromise. **The scale on the collider side is `<Misc>`'s and
  is per team** - see
  [collision.md](collision.md#the-dimensions-feeding-the-collider-are-scaled-not-the-authored-misc-values)
  for the full argument trace into `Body_SetBoxDimensions`; that page is
  authoritative for the collider box, this one only for the inertia tensor.
- **The tensor is frozen at construction.** `Ship_UpdateCraft` calls
  `Body_SetMass` again on **every frame** (`0x0884985c`, `lwc1 f12,0x60(a1)`
  through the `craft+0x70` class pointer, i.e. `Physical.mass`), and
  `Body_SetMass` is six instructions that touch only `+0x374`/`+0x378`. Nothing
  recomputes `+0x40`. So the craft's rotational inertia is decoupled from its
  translational mass for the whole race.
- **`Body_SetOrientation` (`0x0884d868`, confidence 88)** was read while
  excluding candidate tensor writers: it stores the basis and refreshes the
  cached transpose, and touches nothing else - in particular not `+0x40`.
  Named during that exclusion sweep; recorded here so the rename carries its
  evidence.

#### It agrees with the captures, and the agreement picks out the mass

With `(12, 8, 12)` and `m = 0.9`:

| Axis | `I^-1` | `I` |
| --- | ---: | ---: |
| `x` right | `0.064103` | `15.6` |
| `y` up | `0.046296` | `21.6` |
| `z` forward | `0.064103` | `15.6` |

`scripts/trace-angular-fit.py` fitted the recorded `body+0x160` column against a
body-local angular velocity on two captures and got `~(15, 21..22, 14..16)` -
the same `x == z` symmetry, the same odd axis out, and a ratio of `1.413` against
the recovered `1.385`. That measurement knew nothing about this function.

The agreement is sharp enough to **discriminate which mass the tensor was built
with**, which is the strongest part of this reading:

| tensor built with | `I_yy` | against the fit's `21.2` |
| --- | ---: | ---: |
| `m = 0.9`, the constructor's | `21.6` | `1.9 %` |
| `m = 1.0`, `Body_Init`'s default | `24.0` | `13 %` |

So the captures independently confirm both the `0.9` and the "frozen at
construction" claim: if the tensor tracked the runtime mass the fit would have
landed on `24`. (`docs/physics/force-balance-ground-truth.md`'s `mass = 1` is
`Physical.mass`, the runtime value the *integrator* divides by. Two different
masses, both correct, and now distinguishable.)

`oag_physics::forces::YAW_INVERSE_INERTIA` is this value, replacing the fitted
`YAW_DRIVE_CALIBRATION = 0.0452`. Replayed against the two steer captures under
`just test-data`, the recovered `0.046296` scores RMS `0.1430`/`0.1808` rad/s
against the fitted value's `0.1208`/`0.1995` on a signal near `1.5` - better on
one capture, worse on the other, marginally better in mean square. Within each
other's noise, which is what agreement at this level should look like.

#### What this does not settle: pitch and roll

`Ship_ApplyAngularDamping` (`0x08848ed0`) loads **`body+0x160`** at `0x08848f08`
- the angular *momentum*, not the angular velocity - and multiplies it by
`(-pitch_damping, -5, -2)`. That is why the inertia does not cancel out of the
equilibrium, and it retires the argument (recorded in `forces.rs` before this
reading) that "drive and damping share one accumulator so the tensor cancels".
It cancels only if the damping reads `omega`, and it does not.

On the yaw axis, `dL/dt = drive - 5L` with `omega = c L` differentiates to
`domega/dt = c * drive - 5 * omega`, so scaling the yaw drive by `c` on an
acceleration accumulator is exactly equivalent. Two things are left over:

- **Pitch and roll.** The same reading predicts `omega = drive / (damping * I)`
  where `oag-physics` computes `omega = drive / damping`, i.e. **the crate
  applies about `15.6x` too much pitch and roll authority**.
- **The world-angular terms.** `oag-physics` scales only its body-local yaw
  accumulator, but the weathervane and the surface-alignment torque go into the
  world one, which `craft+0x350` -> `body+0x130` -> `basis^T * worldTorque`
  (`0x0884e35c`) integrates into `L` exactly like the local drives. So they carry
  `I^-1` in the original and do not in the crate - **about `21.6x` too much
  weathervane yaw relative to steering.** This was invisible while the constant
  was fitted, because the fit absorbed it.

Neither is applied yet: both need the whole crate on the momentum model, doing
one alone is a half-application, and there is no captured pitch or roll input to
validate against. A held-pitch capture on the reference scenario is what would
settle it.

#### What a wall-free cornering lap adds, and the negative it turns up

`I_yy = 21.6` now has a second, independent runtime leg at racing speed rather
than at the steer captures' `25 u/s`. Recovering the rotation the recorded basis
actually performs (`omega` from `M[i]^T M[i+1]`) over 2,845 wall-free intervals
of `data/traces/talons-junction-time-trial-lap.csv` and fitting `avel` against
it gives `-20.948` on the up axis against this section's `-21.6` - **`3.0 %`, at
90 to 164 units/s**, with `99.5 %` of the column's variance explained.
Confidence **90** on the yaw entry.

**The same fit refutes `avel = -I * omega` on pitch and roll**, which is a
kinematic identity and so should have held on all three axes: pitch reads
`-3.611` against `-15.6` (`0.231x`, 16 % explained) and roll `-10.100` against
`-15.6` (`0.647x`, 62 %). Both are *below* `I`, so the basis rotates **more**
than the momentum column accounts for. This does not touch `Body_SetBoxInertia`,
which is read at 92 and confirmed on yaw; what it says is that something drives
the attitude axes outside the momentum path. The evidence that it is attitude
alignment rather than an analysis artefact - `91 %` of the excess rotation lies
in the plane that tilts `up`, its lag-1 autocorrelation is `0.978`, and its
magnitude scales with speed - is in
[cornering-ground-truth.md](../../../physics/cornering-ground-truth.md#pitch-and-roll-the-momentum-column-does-not-explain-the-rotation),
along with the four candidate `L`-to-`omega` maps that were tried and rejected.

**So the pitch/roll bullet above cannot be validated from any existing capture**,
and a held-pitch capture alone will not settle it either: the column that would
is `body+0x150`, the angular velocity itself, which no capture records. Adding
it is one line in `scripts/psp_trace_fields.py` and it answers directly whether
the basis advance uses `I^-1 * body+0x160`.

## The contact response, and the friction law the force balance was missing

[force-balance-ground-truth.md](../../../physics/force-balance-ground-truth.md)
resolves the M4 blocker by showing the missing `~52` units of force is a
`3.67 %`-per-frame reduction of the *velocity*, applied outside every force
accumulator, and names `FUN_0884e968` as the live lead. **That function has now
been read, and it is the mechanism.**

### `Body_ResolveContact` (`0x0884e968`)

Signature `void Body_ResolveContact(RigidBody *body /* a0 */, Contact *c /* a1 */)`.
Reconstructed from the instruction stream:

```c
r       = c->point(+0x00) - body->position(+0x30);
wWorld  = basis(+0x00..+0x30) * body->angular(+0x150);   /* vtfm4.q, 0884ea58 */
vPoint  = body->velocity(+0x140) + cross(r, wWorld);     /* vcrsp.t + vadd.q  */
n       = c->normal(+0x10);
vn      = dot(vPoint, n);                                /* vdot.t, 0884eb1c  */

e       = body->restitution(+0x388);                     /* 0884eb28 */
num     = -(1.0f + e) * vn;                              /* 0884eb68-0884eb70 */
denom   = body->invMass(+0x378) + angularTerm;           /* 0884ecb4 */
if (denom == 0.0f)  return;                              /* 0884ecb8/0884ecc0 */
j       = num / denom;                                   /* 0884ecc8 */

impulse = n * j;                                         /* 0884ecec, sp+0xc0 */
/* the -1.0f scale at 0884ed0c writes sp+0x110 and sp+0x150, which nothing
   reads; the impulse handed to Body_ApplyImpulseAtPoint is sp+0xc0 (s3). */

if (c->friction(+0x34) > 0.0f) {                         /* 0884ed4c-0884ed58 */
    vt = vPoint - n * dot(vPoint, n);                    /* 0884ede4 vsub.q   */
    if (vt != (0,0,0))                                   /* 0884ee00-0884ee34 */
        impulse = n * j - vt * c->friction;              /* 0884ee50/0884ee84 */
}

Body_ApplyImpulseAtPoint(body, c, impulse);              /* 0884eea8 */
Body_Translate(body, n * c->depth(+0x30));               /* 0884eecc */
Body_RecordContact(body, c, impulse, c->flags(+0x20));   /* 0884eee4 */
```

Confidence **88**. Every line is a decoded VFPU instruction or a short run of
them, and the shape matches the PS2 `Body_ResolveContactPair` (`0x0015e600`)
that [ps2-pulse-eu/craft-update.md](../ps2-pulse-eu/craft-update.md) records - same
relative-velocity-at-the-point construction, same `-(1 + e)` numerator, same
invMass denominator, same deferred friction using the contact's `+0x34`.

### Why this is the missing `2.28 * fs`

`Body_ApplyImpulseAtPoint` (`0x0884d64c`) opens with

```text
0884d660  lwc1  f12,0x378(s0)     ; invMass
0884d680  vscl.q C300,C500,S400   ; impulse * invMass
0884d6b8  vadd.q C220,C200,C210   ; velocity += impulse * invMass
0884d6d0  sv.q   ...,0x0(a0)      ; a0 == body+0x140
```

and `<Physical mass>` is `1` for every shipped class, so `invMass == 1`. Putting
the two together, the tangential half of the contact response is exactly

```text
velocity -= c->friction * vt
```

where `vt` is the component of the contact point's velocity **perpendicular to
the contact normal**. For a craft scraping a wall the normal is roughly lateral,
so `vt` is very nearly the whole forward velocity, and the per-frame speed loss
is a **multiplicative** `friction`. That is:

- **multiplicative, not a fixed decrement** - which is what the trace shows, the
  ratio holding at `1.040` while `|v|` falls 36 %;
- **applied in velocity space through the body directly**, never through
  `craft+0x320`/`+0x330`, which is why enumerating every force term in
  `Ship_UpdateCraft` could not find it;
- **once per frame per contact**, matching the observed per-frame cadence.

It also explains the decay the capture records - though **the direction of that
argument was wrong the first time this page was written, and it is corrected
here**. The loss runs `5.2 %` on the impact frame and falls monotonically toward
an asymptote. It falls rather than rises: the normal impulse *adds* loss on top
of the tangential one for any plausible denominator, so the observed per-frame
loss has the friction coefficient as a **floor**, approached from above as the
motion becomes purely tangential. The asymptote, and therefore the coefficient,
is `3.5 %` - not `3.67 %`, which is the transient still an eighth of the way
from the impact. `3.5 %` is `(0.05 + 0.02) / 2`, both literals in the binary;
[contact-response.md](contact-response.md) reads them and works the bound
through. The confidence-80 note that used to be here is superseded.

### `Body_ApplyImpulseAtPoint` (`0x0884d64c`) and `Body_Translate` (`0x0884d9f8`)

```c
void Body_ApplyImpulseAtPoint(RigidBody *body, const Vec4 *point, const Vec4 *impulse)
{
    body->velocity(+0x140) += *impulse * body->invMass(+0x378);
    r = *point - body->position(+0x30);
    body->angular(+0x160) -= (invInertia(+0xc0) * cross(r, *impulse)) * body->+0x394;
    body->+0x150 = basis(+0x80..+0xb0) * body->angular(+0x160);
}
```

Confidence **88**. Note the angular update **subtracts**, which is the same
`w_game = -w_physics` convention `Body_AddForceAtPoint` shows above, and the last
line is the `+0x160`-to-`+0x150` mapping again.

`Body_Translate` (`0x0884d9f8`) is eleven instructions: `body->position(+0x30) +=
(f12, f13, f14)`. Confidence **92**. It is also what `Ship_HoverTwoPoint` calls at
`0x0884a86c` to push a craft out of a penetrating hover probe, which is a second
use site agreeing with the name.

### Where the coefficient comes from

**Answered, in [contact-response.md](contact-response.md)**, and implemented in
`crates/physics/src/wall.rs`. `contact+0x34` is filled by `Collision_AddContact`
(`0x08816864`) as the average of the two colliders' `+0x64` fields, forced to
zero if either is negative; a wall's is `0.05` and the craft's box collider's is
`0.02`, so a wall scrape is `0.035` per frame. The paragraphs below are the
reasoning that was correct to stop at before that was read, and the units check
in the second one still stands: the per-vertex scalar is **not** the friction.

The *shape* was recovered at 88, but the coefficient is **per-contact data, not a
constant in the binary** - `c->friction` is read from `contact+0x34`, and nothing
on this page shows what fills it. Implementing the term with a coefficient chosen
to reproduce `3.67 %` would be exactly the "tune a constant to fit the traces"
move [force-balance-ground-truth.md](../../../physics/force-balance-ground-truth.md)
forbids, so it is deliberately left out until the data path is read.

**And the obvious candidate for that data path is refuted by a units check.**
[collision.md](collision.md) records a per-vertex `f32` scalar that "reaches the
hit result and accumulates into ship state", with "grip, friction or roughness" a
guess at confidence 40, and `oag_physics::collide::Hit::vertex_scalar` already
carries it. It is tempting to call that the friction and wire it up. **Do not,
without reading the fill site**: that scalar defaults to `1.0` when the mesh has
no chunk 3, and a friction of `1.0` in the law above removes the craft's entire
tangential velocity in one frame. The measured value is `0.0367`. Either there is
a scale between them or the field is not the friction; a naive identification
would stop the ship dead. Confidence that the two are related at all: **50** -
deliberately not renamed and not implemented. **Settled since**: the friction is
`collider+0x64` averaged over the two colliders, not the per-vertex scalar, so
the two are unrelated and the scalar's confidence-40 guess is refuted rather
than merely unconfirmed.

What a reimplementation needed, in order: the writer of `contact+0x34` (start from
the contact-generation side, `Collision_AddContact` `0x08816864`), then the
tangential term, then a re-run of the standing-start capture with
`speed / |velocity|` as the check - it should read `1.0000` while free and settle
at the surface's friction while scraping. All three were done; the capture does
exactly that, and the settling value is the predicted `0.035`.

## Not determined

- **What calls `Body_Integrate`, and with what `subStep` count.** The count is an
  argument, so it is a caller's choice and could differ between the race and the
  front end. Nothing on this page depends on it - see the damping argument above,
  which is sub-step invariant - but a reimplementation that wants bit-comparable
  intermediate state does.
- **`body+0x38c` (`1.0`).** Still offset-named. `+0x388` is now settled: it is the
  **restitution**, read by `Body_ResolveContact` at `0x0884eb28` and added to
  `1.0` before scaling the normal-direction relative velocity. `Body_Init`
  defaults it to `0.5` and the ship-entity constructor (`0x08840c74`) overwrites
  it with `0.4` (`0x3ecccccd`, loaded at `0x08841424`, stored at `0x088414b8`,
  with no jump target in between so the path is unconditional). Confidence
  **88**, up from 55.
- **`body+0x394`**, the scale `Body_ApplyImpulseAtPoint` applies to the angular
  half of an impulse. Still unread.

## The body is writable from outside, and the craft flies on

`scripts/psp-drive.py place` rewrites a live body wholesale - basis rows,
transpose at `+0xc0`, position, velocity, zeros into `+0x150`/`+0x160` - inside
one `Ship_UpdateCraft` entry, and the game integrates the written state as if
it had always been there: no respawn, hover re-settles within tens of ticks,
and a speedup pad crossed by the written velocity fires its boost normally
(measured 2026-08-04, three placements; details on
[the debugger page](../../../reverse-engineering/ppsspp-debugger.md#teleporting-the-craft-works-and-what-a-settle-looks-like)).
That is consistent with everything above: nothing in the update reads a hidden
copy of the pose, so state written between frames *is* the state. The rows and
the `+0xc0` transpose were always written together, so whether the integrator's
own post-loop rebuild would have covered the transpose alone was not isolated -
the safe pair is what the tool writes.
