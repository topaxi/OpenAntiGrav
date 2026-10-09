# The craft update and the rigid body (PS2)

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

This page exists to answer a specific question that
[docs/physics/README.md](../../../physics/README.md) could not settle from the
PSP binary alone: the surface-alignment torque's gain and sign, the roll damping
beside it, and whether the sub-step count rescues a pair that reads as
marginally unstable. It answers two of those and contradicts the third, and it
now also answers the sign question the first edition could only sharpen: the
apparent inversion is
[a sign convention on `w`](#the-angular-sign-convention-w_game---w_physics), not
an error, and nothing compensates it because nothing needs to.

**The names below are applied**, from [names.tsv](names.tsv). Nothing scores
below 70; the one name under 80 carries the `_q` suffix per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md).

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0015c1b0` | `Ship_ApplyAngularDamping` | 92 |
| `0x0015aff0` | `Ship_UpdateHover` | 90 |
| `0x0015a940` | `Ship_HoverFourCorner` | 90 |
| `0x0015dac8` | `Body_AddForceWorld` | 90 |
| `0x0015da60` | `Body_AddForceAtPoint` | 90 |
| `0x0015ddb8` | `Body_AddTorqueWorld` | 88 |
| `0x0015dda0` | `Body_AddTorqueLocal` | 88 |
| `0x0015d088` | `Body_Integrate` | 88 |
| `0x001596f8` | `Ship_UpdateCraft` | 88 |
| `0x0015c448` | `Ship_UpdateEngine` | 90 |
| `0x0015c2d8` | `Ship_UpdatePitch` | 90 |
| `0x0015a058` | `Ship_UpdateBrakes` | 90 |
| `0x0015bf30` | `Ship_UpdateSteering` | 90 |
| `0x0015c7c0` | `Ship_ApplyLateralGrip` | 90 |
| `0x0015ca48` | `Body_ResetAccumulators` | 75 |
| `0x0015d058` | `Body_ClearAccumulators` | 90 |
| `0x0015a290` | `Ship_ApplyWeathervaneTorque` | 88 |
| `0x0015c3a0` | `Ship_ApplyQuadraticDrag` | 88 |
| `0x00159f10` | `Ship_ApplyRollingResistance` | 88 |
| `0x0015b978` | `Ship_HoverTwoPoint` | 85 |
| `0x0015a1d0` | `Ship_ApplyGravity` | 85 |
| `0x00159e78` | `Ship_ApplyVerticalDamping` | 92 |
| `0x0015ded8` | `World_StepBodies` | 80 |
| `0x0015e600` | `Body_ResolveContactPair` | 85 |
| `0x0015ce28` | `Body_QueueDeferredImpulse` | 82 |
| `0x0015b070` | `Ship_UpdateMagLock` | 78 |
| `0x0019f020` | `Handling_LoadForTeam` | 78 |
| `0x0015a550` | `Ship_UpdateAirbrakes_q` | 78 |

## The dispatch, and why the names above are trustworthy

`World_StepBodies` (`0x0015ded8`) walks the body array and, per entity, does:

```c
*(u32 *)(entity + 0x370) = 0;
vtable = *(int *)(body + 0x38);
(**(code **)(vtable + 0x74))(dt, body + *(short *)(vtable + 0x70), entity);
```

[engine.md](../psp-pulse-usa/engine.md) records the PSP call site as a vtable at
`object+0x38` whose entries are 8 bytes of `{i16 this-adjust, void *fn}`, with
`sw zero,0x370(a0)` clearing a field on the entity immediately before the call.
**Same vtable base, same entry layout, same `+0x370` clear, same slot region.**
That is a strong positional identification of the function reached through it,
which is `Ship_UpdateCraft` (`0x001596f8`) - and `Ship_UpdateCraft` is the only
caller of both `Ship_UpdateHover` and `Ship_ApplyAngularDamping`.

`Ship_UpdateHover` (`0x0015aff0`) is three lines, and they are the PSP's three
lines:

```c
craft->flags &= ~1;                                    /* clear grounded */
if (DAT_0027a7a8 == 0 && DAT_002dabd0 == 6)  Ship_HoverFourCorner(craft);
else                                          Ship_HoverTwoPoint(craft);
Ship_UpdateMagLock(craft);
```

[engine.md](../psp-pulse-usa/engine.md) gives the PSP selector as
`DAT_08ab07e3 == 0 && DAT_08b31048 == 6`. **Both globals, both comparisons and
the magic value 6 survive the port**, which corroborates that page's confidence-84
reading of the four-corner selector and fixes the two-point/four-corner identities
here positionally.

## The surface-alignment torque: the gain really is `-400`

At the end of `Ship_HoverFourCorner`, once the contacting probes have been
averaged:

```c
avgNormal = normalSum * (1.0f / contactCount);
/* vopmula ACC, up, avgNormal ; vopmsub d, avgNormal, up   =>  d = up x avgNormal */
torque    = (up cross avgNormal) * -400.0f;
Body_AddTorqueWorld(body, torque);
```

`-400.0f` is `0xC3C80000`, materialised as an immediate by `lui at,0xc3c8` at
`0x0015adf8`. The same constant appears at `0x0015bdc0` inside
`Ship_HoverTwoPoint`. There is no `400.0f` anywhere in `.rodata` and no other
`lui` of `0xc3c8` in the executable.

**So the magnitude and the sign are both confirmed in a second binary.**
[docs/physics/README.md](../../../physics/README.md) treated the PSP's
`-400 * cross(up, avgNormal)` as most likely a transcription error, on the
grounds that the arithmetic makes that term anti-aligning while the prose says it
levels the ship. A different compiler on a different ISA producing the same
expression retires that explanation. Confidence **92**, from an unambiguous
instruction-level read of two functions.

The operand order is not in doubt either, and it has since been settled against
the vendor ISA reference rather than from memory - see
[the cross-product convention](#the-cross-product-convention-settled-against-the-isa-manual)
below. `A` is the up axis (`craft+0x180`) and `B` is the averaged contact
normal, so the term is `-400 * (up x avgNormal)`.

**And that term is aligning, not anti-aligning.** It reads backwards only
against the textbook sign convention for angular velocity, which this engine
does not use; see
[the angular sign convention](#the-angular-sign-convention-w_game---w_physics).

## The cross-product convention, settled against the ISA manual

Every sign question on this page rests on one thing: what
`vopmula`/`vopmsub` actually compute. Ghidra models both as opaque
`CALLOTHER` userops (`0xc5` and `0xc6`), so the decompiler supplies **no**
semantics for them and cannot be appealed to here.

The *VU User's Manual* version 6.0 (SCEI) defines the micro-mode pair on
p.116-117, and `VOPMULA`/`VOPMSUB` on p.305-306 as "same as the micro
instruction":

```text
OPMULA.xyz ACCxyz, VF[fs]xyz, VF[ft]xyz
    ACCx = VF[fs]y * VF[ft]z
    ACCy = VF[fs]z * VF[ft]x
    ACCz = VF[fs]x * VF[ft]y

OPMSUB.xyz VF[fd]xyz, VF[fs]xyz, VF[ft]xyz
    VF[fd]x = ACCx - VF[fs]y * VF[ft]z
    VF[fd]y = ACCy - VF[fs]z * VF[ft]x
    VF[fd]z = ACCz - VF[fs]x * VF[ft]y
```

The manual's own worked example - which carries the parenthetical "*Be careful
to the description order*" - is exactly the idiom this binary uses:

```text
OPMULA.xyz ACCxyz, VF20xyz, VF30xyz
OPMSUB.xyz VF10xyz, VF30xyz, VF20xyz     =>  VF10 = VF20 x VF30
```

So `vopmula(ACC, A, B)` followed by `vopmsub(D, B, A)` gives **`D = A x B`**,
where `A` is the *first* source operand of the `vopmula`.

**Ghidra's operand printing was verified, not assumed.** The encoding places
`ft` at bits 20-16, `fs` at 15-11 and `fd` at 10-6 - `ft` *before* `fs` - so a
disassembler printing in encoding order would render these instructions
reversed and silently invert every conclusion below. Decoding the four raw
words settles it:

| Address | Word | dest | ft | fs | fd | Ghidra prints |
| --- | --- | --- | --- | --- | --- | --- |
| `0x0015ae10` | `0x4bc112fe` | `1110` | `vf1` | `vf2` | - | `vopmula.xyz vf2,vf1` |
| `0x0015ae14` | `0x4bc208ae` | `1110` | `vf2` | `vf1` | `vf2` | `vopmsub.xyz vf2,vf1,vf2` |
| `0x0015da9c` | `0x4bc20afe` | `1110` | `vf2` | `vf1` | - | `vopmula.xyz ACC,vf1,vf2` |
| `0x0015daa0` | `0x4bc1106e` | `1110` | `vf1` | `vf2` | `vf1` | `vopmsub.xyz vf1,vf2,vf1` |

Ghidra prints `(fs, ft)` and `(fd, fs, ft)` - mnemonic order, matching the
manual. Applying the definitions:

- **`Ship_HoverFourCorner`**: `fs = vf2 = up`, `ft = vf1 = avgNormal`, then
  `fd.x = up.y*n.z - n.y*up.z`, which is `(up x avgNormal).x`. The `-400`
  broadcast in `vf3` multiplies that. **The term reads as anti-aligning under
  the textbook convention** - which is not the one the engine uses; see
  [the angular sign convention](#the-angular-sign-convention-w_game---w_physics).
- **`Body_AddForceAtPoint`**: `fs = vf1 = force`, `ft = vf2 = r`, then
  `fd.x = F.y*r.z - r.y*F.z`, which is `(F x r).x`, the negative of the
  physical `r x F`.

Confidence **95** on the operand semantics specifically. That is higher than
this page's decompilation-only ceiling because it is not a behavioural claim
about the game: it is a vendor-documented instruction definition, cross-checked
against the raw encoding of the four instructions in question. It says what the
arithmetic is, not what the game does with it.

## `Body_AddForceAtPoint` computes `F x r`

Signature, from the prologue at `0x0015da60`, which moves `a1` into `s2` and
`a2` into `s1` - the reverse of the obvious order:

```c
void Body_AddForceAtPoint(Body *body /* a0 */, const vec4 *force /* a1 */,
                          const vec4 *point /* a2 */);
```

`a1` is the force, not the point: the function's first act, in the delay slot of
its own prologue, is `Body_AddForceWorld(body, a1)`.

```
0015da84  lqc2  vf1,0x30(s0)      ; body position
0015da8c  lqc2  vf2,0x0(s1)       ; the world point (a2)
0015da94  vsub  vf2,vf2,vf1       ; r = point - position
0015da98  lqc2  vf1,0x0(s2)       ; the force (a1)
0015da9c  vopmula.xyz ACC,vf1,vf2 ; A = force, B = r
0015daa0  vopmsub.xyz vf1,vf2,vf1 ; d = A x B = force x r
0015daa4  jal   Body_AddTorqueWorld
```

Textbook torque is `r x F`. This is `F x r`, its negative - the same apparent
inversion the `-400` alignment gain carries, on the term where the intended
answer is least arguable. The next section settles both.

## The angular sign convention: `w_game = -w_physics`

This page originally offered three candidate explanations for that inversion.
All three have now been checked. Two are dead; the third is right, but not in
the place it was expected, and the answer is that **there is nothing to
compensate.**

### The `vopmsub` operand convention is exactly as read - dead

> ~~the EE's `vopmsub` operand convention is the reverse of the one used here,
> in which case all three terms flip together and everything is consistent with
> the prose.~~

Refuted against the *VU User's Manual* v6.0 and the raw instruction encodings;
see [the section above](#the-cross-product-convention-settled-against-the-isa-manual).
The readings `force x r` and `up x avgNormal` are literal.

### The force is not pre-negated - dead

> ~~`Ship_HoverFourCorner` passes an already-negated force to
> `Body_AddForceAtPoint`.~~

The call site (`0x0015ac98`..`0x0015acec`) builds the force from the craft's up
axis at `craft+0x180`, component by component:

```
0015ac98  lq     v0,0x180(s4)     ; up
0015acc0  mul.S  f6,f6,f0         ; springMag * (1 + 0.8*clamp(...))
0015acc4  mul.S  f3,f6,f3         ; * up.z
0015acc8  mul.S  f1,f6,f1         ; * up.x
0015accc  mul.S  f2,f6,f2         ; * up.y
0015acd8  pextlw ...              ; repack as (x, y, z, 0)
0015acec  jal    Body_AddForceAtPoint
0015acf0  _sq    v0,0x60(sp)      ; delay slot; a1 == sp+0x60, so this is `force`
```

Every component is a **positive** multiple of `up`. There is no negation on any
of the three and the packed `w` lane is zero. Confidence **92**.

`Ship_HoverTwoPoint`'s call site (`0x0015bd00`..`0x0015bd38`) is the same shape
instruction for instruction: one scalar magnitude, three plain `mul.S` against
the source vector's components, the same `pextlw` repack, no negation on any
lane. Its source vector was **not** confirmed to be the up axis, so this is
recorded as "same idiom, no per-component sign" rather than as a second full
check. The resolution below does not depend on it either way.

### The integrator rotates the basis by `-w` - the answer

`Body_Integrate` builds a skew matrix from the vector at `body+0x150` and the
sub-step `h` (`0x0015d134`..`0x0015d1f4`), then applies it to the three basis
rows at `body+0x00`/`+0x10`/`+0x20` (`0x0015d218`..`0x0015d268`). The three base
constants it starts from, at `0x002c69f0`/`+0x10`/`+0x20`, were **read out of
memory and are all zero**, so with `w = (x, y, z)` the built vectors are exactly

```text
v0 = ( 0,   -z*h,  y*h)
v1 = ( z*h,  0,   -x*h)
v2 = (-y*h,  x*h,  0  )
```

and each row is updated by

```text
row_i += v_i.x * row0 + v_i.y * row1 + v_i.z * row2
```

Writing the first one out, `d(row0) = h * (-z*row1 + y*row2)`. The textbook
kinematics `e' = w x e` gives `w x e0 = (0, z, -y)`, that is
`d(row0) = h * (z*row1 - y*row2)`. Rows 1 and 2 carry the same negation.
**The integrator computes `e' = e x w`, which is `-(w x e)`.**

Note this derivation uses no outer-product instruction at all - it is `vmulax` /
`vmadday` / `vmaddz` throughout - so it is independent of the convention settled
above.

That closes it. The engine's angular velocity is the negative of the textbook
one, a left-hand-rule pseudovector, and both halves of the rigid-body
formulation are flipped together:

| Textbook | This engine |
| --- | --- |
| `tau = r x F` | `tau = F x r` |
| `e' = w x e` | `e' = e x w` |
| align `up` to `n`: `+k * (up x n)` | `-k * (up x n)`, with `k = 400` |

Substituting `w_game = -w_physics` turns each right-hand entry into the left
one. **So nothing is inverted and nothing needs compensating: `F x r` and
`-400 * cross(up, avgNormal)` are both correct, and the alignment torque really
does level the ship.** [docs/physics/README.md](../../../physics/README.md)'s
prose and the arithmetic agree after all; what was missing was the convention,
not a sign.

This is **not** a handedness result, and the two pages that reached for
handedness were right to reject it. `Body_Integrate`'s orthonormaliser
(`0x0015d5f4`..`0x0015d630`) rebuilds the basis as `row0 = row1 x row2`, which is
cyclically the same statement as [engine.md](../psp-pulse-usa/engine.md)'s
runtime-measured `cross(row0, row1) = row2` on 200 of 200 ticks. PS2 code and PSP
measurement agree the basis is positively oriented under the ordinary
component-wise cross product. Handedness is not the variable. The sign of `w`
is, and it is a separate question that was being conflated with it.

**Two independent legs, and the second is blind to the first's one assumption.**

- *The instruction read above* assumes only that the matrix at `body+0x80`,
  which maps `+0x160` onto `+0x150`, does not flip a sign. It is built from the
  orthonormalised basis, so it is a rotation and its determinant is `+1`. Which
  frame `+0x150` and `+0x160` are each expressed in is **not** settled here, and
  [engine.md](../psp-pulse-usa/engine.md)'s deliberate confidence-74 cap on the
  local/world split of the angular accumulators stands untouched.
- *[engine.md](../psp-pulse-usa/engine.md)'s weathervane term* needs no assumption at
  all: `angularWorld += cross(forward, velocity) * (grounded ? -0.1 : -0.3)`,
  with the coefficients as hardcoded literals and the intended effect - turning
  the nose toward the direction of travel - already stated there. That is
  backwards under the textbook convention and correct under this one, from a
  third term arrived at independently of the other two.
- *[engine.md](../psp-pulse-usa/engine.md)'s steering measurement* is end-to-end,
  accumulator sign in and observed rotation sign out, so it bypasses every
  intermediate transform including the one the first leg assumes about. Holding
  left puts `craft+0x2c0` at about `-96`, `angularLocal.y = steer *
  Turning.amount`, and the measured `dot(cross(fwd_t, fwd_t+1), up) / dt` is
  `+1.51 rad/s` on 199 of 199 ticks: the accumulator's sign is the opposite of
  the rotation it produces. That is precisely the constraint engine.md said any
  answer would have to satisfy. It is listed third only because it depends on
  `Turning.amount > 0`, which is plausible - it is not one of the four fields
  that page shows being scaled at load - but was not verified.

Confidence **88** for the convention in the PS2 build: raw disassembly rather
than decompiler output, a vendor ISA manual for the one instruction pair that
could have flipped the reading, all three base constants read out of memory, and
a runtime measurement on the other build that it explains. Not higher because
`Body_Integrate` was not itself observed at runtime.

Confidence **80** that the PSP build shares the convention. Its integrator has
never been located, so the cross-platform half rests on the steering and
weathervane terms agreeing, not on reading PSP code.

### What this means for a reimplementation

Recorded here rather than acted on; no Rust was touched.

**Apply exactly one flip.** Either negate every torque source and keep the
textbook `e' = w x e`, or keep the original's torque expressions verbatim and
integrate `e' = e x w`. Doing both restores the bug; doing neither is what makes
a port diverge.

- The hover spring's `F x r` flips too, not only the `-400` term. Any stability
  argument that covers one has to cover both.
- **Angular damping is invariant and must not be touched.** `tau = -c * w` is a
  negative multiple of `w` in either convention, so it is already correct, and
  "fixing" it as well flips it wrong.
- A port that integrates orientation the textbook way while copying
  `-400 * cross(up, n)` verbatim has an **anti**-aligning surface torque. That is
  not the marginal-stability question the sub-step arithmetic below describes; it
  is unconditional divergence, and the two failure modes look similar from the
  outside.
- The same applies to the weathervane torque and to the steering sign.

## Angular damping, confirmed exactly

`Ship_ApplyAngularDamping` (`0x0015c1b0`):

```c
k = (craft->mode == 0) ? -5.0f : -2.0f;          /* craft+0x2d4 */
w = body->angularVelocityLocal;                  /* body+0x160 */
angularLocal += ( -pitch_damping * w.x,
                  -5.0f          * w.y,
                   k             * w.z );        /* Body_AddTorqueLocal */
```

`pitch_damping` is read from `*(craft+0x90) + 0x78`, and `craft+0x90` is the
cached pointer to the handling block. Block-relative `0x78` is absolute `0x10c`,
which [handling-xml.md](handling-xml.md) fixes as `pitch_damping`. Everything
here matches [engine.md](../psp-pulse-usa/engine.md) term for term: yaw damping is a
hard `-5.0` for every craft in the game, roll damping is `-5.0` in mode 0 and
`-2.0` otherwise, and only the pitch axis is per ship.

Confidence **92**. Note for anyone using `c = 2` in a stability calculation: that
is the **non-zero-mode** value. Mode 0 damps roll at `-5.0`.

## Sub-stepping does not do what the physics page assumes

`Body_Integrate` (`0x0015d088`) is `(float dt, Body *body, uint substeps)`:

```c
h = dt / substeps;
for (i = 0; i < substeps; i++) {
    position += velocity * h;
    velocity += worldForce   * invMass * h;     /* body+0x100, read in the loop */
    angularVelocity += (M(body+0xc0) * worldAngular) * h;   /* body+0x130 */
    ... orthonormalise the basis ...
}
```

**The accumulators are read inside the loop and nothing recomputes them.**
`Ship_UpdateCraft` fills them once per frame; every sub-step then applies the
same frozen values.

### Two damping terms, and what initialises them

The sketch also omits a pair of damping terms applied at the end of each sub-step,
after both force accumulators:

```c
velocity        -= velocity        * h * body+0x384;
angularVelocity -= angularVelocity * h * body+0x380;
```

`Body_Init` (`0x0015cb98`) zeroes both, along with every accumulator, and defaults
the `invMass` at `body+0x378` to `1.0`. The ship-entity constructor
(`FUN_00150d20`, the function that also calls `Ship_InitCraft` and
`Handling_LoadForTeam`) then sets **both damping coefficients to `0.01`**
(`0x3c23d70a`), plus `body+0x388 = 0.4` and `body+0x394 = 0.1`, whose meanings are
not established. Confidence **80** - read off VU-macro decompiler output.

`Body_SetPosition` (`0x0015d018`) is `(float x, float y, float z, Body *body)` and
writes the three lanes of `body+0x30`; it is named here because the ship
constructor calls it with `0.75 * 150.0` in `y`, which reads like a mass at a
glance and is a **spawn height**. Confidence **80**.

Neither damping term appears in [physics](../../../physics/README.md) or in
`crates/physics`, and the linear one is a genuine gap in this reimplementation.
It is not, however, large enough to explain the open thrust/resistance discrepancy
recorded in `crates/physics/src/engine.rs`: at `0.01` against an `invMass` of
`1.0` it opposes motion with about `0.24` of equivalent force at 24 units/s.

### The local force accumulator is applied on exactly the same terms

The sketch above omits `body+0x110`, the **body-local** force accumulator, which
is where all engine thrust and the lateral-grip term end up. It is read in the
same sub-step loop, immediately after the world force, and applied like this:

```c
worldFromLocal = row0 * lf.x + row1 * lf.y + row2 * lf.z;   /* body+0x00/+0x10/+0x20 */
velocity += worldFromLocal * invMass * h;
```

using the rows *as already advanced by this sub-step*, and the **same**
`invMass` word at `body+0x378` and the same `h` as the world force one line
earlier. There is no second scale factor, no separate mass, and no `dt`
asymmetry between the two accumulators - the only difference between them is the
basis rotation, which is what "local" means.

This is worth stating because the asymmetry it rules out is an attractive
explanation for a real, open discrepancy: this crate's engine produces roughly
17x more thrust than a genuine capture of the reference scenario shows (see
`ENGINE_OUTPUT_SCALE` in `crates/physics/src/engine.rs`), and thrust is the one
force term that goes through the *local* accumulator while quadratic drag and
rolling resistance go through the *world* one. **If the two were scaled
differently, that would be the bug. They are not.** `crates/physics`'s `drain`,
which rotates the local accumulator by the orientation and adds both to one
force, matches the original. Confidence **85**: read off the decompiled VU-macro
sequence at `0x0015d088` rather than off clean scalar code, which is what keeps
it below the 88 the rest of `Body_Integrate` carries.

That changes the stability arithmetic. Write the roll oscillator as
`a = −k·x₀ − c·ω₀`, frozen for the frame, and note the loop updates position
*before* velocity. Over `n` sub-steps of `h = H/n`:

```text
ω_n = ω₀ + a·H                                  (independent of n)
x_n = x₀ + ω₀·H + a·H²·(n−1)/(2n)
```

The determinant of that map - which is `|λ|²` while the eigenvalues are complex -
is

```text
det = 1 − c·H + k·H²·(n+1)/(2n)
```

so stability needs `c >= k·H·(n+1)/(2n)`. At `k = 400` and `H = 1/60`:

| Sub-steps | `c` required | Against the actual `c = 2` |
| ---: | ---: | ---: |
| 1 | 6.67 | 3.33x short |
| 3 | **4.44** | **2.22x short** |
| infinite | 3.33 | 1.67x short |

Two things follow, and the second is the useful one.

**Raising the sub-step count cannot fix it.** The requirement does not fall
below `k·H/2 = 3.33` however finely the frame is divided, because the velocity
update is a full-`H` Euler step whatever `n` is. That retires one of the three
explanations [docs/physics/README.md](../../../physics/README.md) offers - "the
sub-step count is not three" - and leaves a wrong magnitude or a genuinely
marginal original.

**This confirms the model `crates/physics/src/hover.rs` already uses, and dates
the physics page.** That file's stability note computes `c >= 4.444` at three
sub-steps, which is exactly the `n = 3` row above; the physics page's
`h <= c/k` with `h = 1/180` gives 1.11x and assumes the forces are re-evaluated
per sub-step, which the original does not do. **The page is the thing that needs
correcting, not the crate.** No Rust was touched from here.

Confidence **88**: the loop structure is unambiguous and the arithmetic follows
from it. Not higher because it is decompilation without a trace, and because the
inertia handling between `body+0x130` and `body+0x160` was not read - if the
rotation at `+0xc0` is not orthonormal the effective `k` and `c` both scale, and
the *ratio* that drives this still does not move, but the argument deserves the
caveat stated.

**The sub-step count itself was not located.** `World_StepBodies` threads it
through from its own caller as a parameter, and that function has neither a
direct caller nor a vtable reference Ghidra can resolve. So "three" is not
established for the PS2 - only that the count is a runtime value rather than a
compiled-in constant.

## The rest of the four-corner path

Read while looking for the above; recorded because it corroborates
[docs/physics/README.md](../../../physics/README.md) cheaply.

- **The four probes** are at `(±2.5, ±5.0)` in the craft's own frame, scaled by a
  global - four packed constants `0x40200000` / `0xc0200000` (`±2.5`) and
  `0x40a00000` / `0xc0a00000` (`±5.0`) built at the top of the function.
- **Grounded** accumulates `+0.25` per contacting probe into `craft+0x2e0`, so
  four corners give `1.0`. The PSP's two-point path adds `0.5` per probe for the
  same total. Flag bit 0 of `craft+0x1e0` is ORed in per contact, matching the
  PSP's `craft+0x1c0` bit 0.
- **The hover spring** is
  `(target − h) · K · (normal_gravity + track_gravity) · mass · 0.15 · (1 + 0.8·clamp(−0.1·dot(...), −1, 1))`,
  where `normal_gravity` and `track_gravity` are read from the cached block
  pointer at `craft+0x90` as `+0x64` and `+0x6c` - block-relative for absolute
  `0xf8` and `0x100`, exactly the offsets [handling-xml.md](handling-xml.md)
  assigns them. That confirms both the block base and the physics page's
  "multiplies by `normal_gravity + track_gravity`".
- **The downforce** is `−track_gravity · mass` along the averaged normal, read
  through a *different* base (`craft+0x8c` plus the runtime class index times
  `0x80`), which is a second, independent route to the same stride.
- **A trailing 10-unit down-ray**: if it hits between 3 and 10 units, a
  correction of `(3 − d) · 0.25 · up` is applied. The PSP's penetration escape is
  described as acting below 1 unit; this is a different, softer mechanism and is
  **not** claimed to be the same thing.
- **Bank-to-yaw is `50.0` here**, not the PSP's `30`:
  `craft+0x194 · (1 − craft+0x2b0) · 50.0` into the local angular accumulator,
  gated on `craft+0x2d4 != 0`. Recorded as a difference; neither value is
  runtime-verified and the field at `+0x194` was not independently identified,
  so this is **not** yet a claim that the two builds tune it differently.

## The passive force terms, checked against the PSP page

`Ship_UpdateCraft` has sixteen callees. Four more of them have now been
identified **by content rather than by position** - size and layout order do not
transfer between the two builds, and guessing from them produced two wrong
matches before the decompiler corrected them. The constants are what identify
these functions, and they are distinctive enough to be near-conclusive.

### Weathervane torque - identical

`Ship_ApplyWeathervaneTorque` (`0x0015a290`), found by searching for the `-0.3`
immediate (`lui at,0xbe99`), which occurs in only three functions in the
executable:

```c
torque = cross(craft+0x1a0, craft+0x1b0) * ((craft->flags & 1) ? -0.1f : -0.3f);
Body_AddTorqueWorld(craft->body, torque);
```

`-0.3` is `0xbe99999a` and `-0.1` is `0xbdcccccd`, both as `lui` immediates.
[engine.md](../psp-pulse-usa/engine.md) gives
`angularWorld += cross(forward, velocity) * (grounded ? -0.1 : -0.3)`.
**Both coefficients, the grounded selector and the operand order all match**, so
`craft+0x1a0` is forward and `craft+0x1b0` is velocity in the PS2 layout.
Confidence **88**.

This is also a **third instance of the sign convention, now in the PS2 binary
rather than only on the PSP page.** `cross(forward, velocity)` scaled by a
*negative* coefficient turns the nose toward the direction of travel only if
`w_game = -w_physics`; under the textbook convention it would turn it away. The
corroborating leg cited
[above](#the-angular-sign-convention-w_game---w_physics) is no longer
PSP-measurement-only.

### Quadratic drag - matches, minus one branch

`Ship_ApplyQuadraticDrag` (`0x0015c3a0`), found by the `-0.005` immediate
(`lui at,0xbba3`), which is unique in the executable:

```c
k = (craft+0x320 < -0.2f)      ? -0.1f     /* 0xbdcccccd */: !(craft->flags & 1)        ? -0.002f   /* 0xbb03126f, airborne */: -0.005f;  /* 0xbba3d70a, grounded */
worldForce += velocity * craft+0x320 * k;
```

Three of the four coefficients, the `-0.2` reverse threshold, the grounded
selector and the whole shape are **identical** to
[engine.md](../psp-pulse-usa/engine.md). `craft+0x320` is the cached forward speed
in the PS2 layout (the PSP's `craft+0x2ec`); note that `+0x320` is the *local
force accumulator* on the PSP, so the craft struct is laid out differently
between builds and offsets must not be carried across.

**The PS2 function has no `-0.9` branch.** [engine.md](../psp-pulse-usa/engine.md)
records `k = -0.9` when `craft+0x2a4 == 0`, described there as multiplying drag
by roughly 180x in that mode. Nothing in `Ship_ApplyQuadraticDrag` tests a mode
enum and no `-0.9` immediate reaches it. Recorded as a **difference, not a
correction**: whether the PS2 hoisted that case to the call site in
`Ship_UpdateCraft` was **not** checked, and until it is, "the PS2 drops the
mode-0 drag multiplier" is a hypothesis at confidence **55**, not a finding.
The three shared coefficients are confidence **88**.

### Gravity - outlined here, and it differs in two ways

The PSP applies gravity inline in `Ship_UpdateCraft`; the PS2 has it as a
function, `Ship_ApplyGravity` (`0x0015a1d0`):

```c
mass = body->mass;                                  /* body+0x374, same offset */
g = -( normal_gravity            * mass * grounded
     + flight_gravity * scale[i] * mass * (1 - grounded) )
    * (1 - magLockBlend);
Body_AddForceWorld(body, g_along_one_axis);
```

`normal_gravity` and `flight_gravity` are read through the cached block pointer
at `craft+0x90` at block-relative `0x64` and `0x68`, which are absolute `0xf8`
and `0xfc` - exactly where [engine.md](../psp-pulse-usa/engine.md)'s parser table
puts them. `grounded` is `craft+0x2e0` and `magLockBlend` is `craft+0x2b0`, both
already fixed by other terms on this page. So the *structure* corroborates.
Two details do not:

- **The per-class scale sits on `flight_gravity` here, not on
  `normal_gravity`.** engine.md states the opposite. One of the two pages is
  wrong about which term the table at `DAT_0027e828` / `0x08ab0dcc` multiplies,
  and this page cannot say which build is which without re-reading the PSP.
- **The whole expression is scaled by `(1 - magLockBlend)`.** engine.md records
  that factor on the vertical-damping term and the bank-to-yaw term but not on
  gravity. So on the PS2 a fully mag-locked ship has no gravity at all, which is
  a sensible thing for a mag-lock to do and is worth checking on the PSP.

Confidence **85** for the PS2 reading, **50** on which build the two differences
actually belong to - that needs the PSP function re-read, not this one.

### The track-section force - structurally identical, still unnamed

`FUN_0015a300` is [engine.md](../psp-pulse-usa/engine.md)'s `FUN_08848f9c`, and it
is **left unnamed here for the same reason it is left unnamed there**: what the
force is *for* is a guess at confidence 45, and ADR-0005 says a guess dressed as
a name stops other people from looking.

The match is close enough to be certain of the identification, though. It looks
up the craft's track section from its position, stores the section's direction
into `craft+0x1d0`, bumps lap-style counters on the race object
(`+0x960`, and a per-slot counter at `+0xa5c + n*0x10`), re-arms a timer every
frame a section is found, and adds a world force along the stored direction with
**a magnitude and a duration read from two separate per-class tables** -
`DAT_0033c350` and `DAT_0033c340`, indexed by the same class global. engine.md
describes exactly that, including the two-table detail
(`0x08b36bc0` / `0x08b36bd0`). Confidence **85** on the identification.

One mechanism engine.md does not record: **the force fades**. The timer counts
down by `dt`, and while it is above `0.1` the magnitude is scaled by
`timer * 10`, dropping to the raw magnitude below that. So it is a decaying
impulse re-armed every frame, not a flat continuous push.

## The control terms

### The controls pointer is `craft+0x98`

Every control term reaches its input through `*(craft+0x98)`, with the same
member offsets [engine.md](../psp-pulse-usa/engine.md) records for the PSP's
`craft+0x78`: `+0x00` steering, `+0x04` thrust, `+0x10` pitch, `+0x44` buttons.
**The struct pointer moved between builds; the struct did not.**

### Pitch - identical, term for term

`Ship_UpdatePitch` (`0x0015c2d8`):

```c
p = 0;
if (FUN_0015c270(craft))                            /* the gate, not decoded */
    p = controls->pitch * (grounded ? pitch_ground : pitch_air);
if (!grounded)
    p += *(craft+0x8c) + 0x90;                      /* weight_distribution */
Body_AddTorqueLocal(body, (p, 0, 0));               /* angularLocal.x += p */
```

`pitch_air` and `pitch_ground` are read at block-relative `0x70` and `0x74`,
absolute `0x104` and `0x108` - exactly
[engine.md](../psp-pulse-usa/engine.md)'s parser table, and the grounded branch
picks `pitch_ground` as that page says. The in-air bias is read from
`stats_base + 0x90`, which the same page identifies as `weight_distribution`
from the PS2 `<Misc>` element.

The gate has a PS2 counterpart too, `FUN_0015c270`, and it is **not decoded
here either** - the same open question in both builds. Confidence **90**: every
element matches and two independent offset chains land on the documented
fields.

### Engine - identical, including the dead ramp

`Ship_UpdateEngine` (`0x0015c448`), found by the `1e10` immediate
(`lui at,0x5015`), which is unique in the executable. Every element of
[engine.md](../psp-pulse-usa/engine.md)'s pseudocode reproduces, with the parameter
block read at block-relative `0x24`/`0x28`/`0x2c`/`0x30`/`0x34` - absolute
`0xb8`/`0xbc`/`0xc0`/`0xc4`/`0xc8`, i.e. `gain`/`amount`/`falloff`/`accelcap`/
`turbo`, in the order that table gives them.

Confirmed against that page item by item: the `0.2` in-air thrust fraction and
the `grounded` blend; the four-corner auto-speed law
`base + step * (uint)craft+0x2bc` under the same
`DAT_0027a7a8 == 0 && DAT_002dabd0 == 6` selector; the cap
`0.5 * forwardSpeed + accelcap`; the `throttle > 100` rescale by `0.01`; the
`1e10` uncapping under flag bit 3; the `min`; the flag-bit-2 multiplier; the
turbo add under bits 9/10 with the mode `== 1` guard and the button-gated boost
lift; the `* craft+0x2c4 * 2.0` tail; the one-shot sub-1.0 scale that resets
itself to `1.0`; and the flag-bit-13 kill.

**`craft+0x2c4` is `1.0`, in this build too.** The PSP page records both engine
multipliers as recovered - `craft+0x294` is the start-line boost multiplier and
`craft+0x2a0` the speed-up pickup - and the PS2 constructor agrees on the first
of them. `Ship_InitCraft` (`0x001592e8`) writes
`*(craft + 0x2c4) = 0x3f800000`, i.e. `1.0`, at `0x00159510`. Only three float
stores anywhere in the image target a `0x2c4` displacement, and this is the only
one on a craft base.

The constructor identification is structural rather than by string: it is the
PSP `Ship_InitCraft`'s twin statement for statement, including the two
`(0, -1.5, 6, 0)` / `(0, -1.5, -6, 0)` vectors scaled by a shared global, the
`10.0` at `+0x2e4` where PSP has it at `+0x2b4`, the `0.5` at `+0x330`, the
`0.5`/`1.0` pair at `+0x34c`/`+0x350` where PSP has `+0x318`/`+0x31c`, and the
same trailing reciprocal-table fixup loop over a file-scope array. Confidence
**85**. It does *not* initialise the flag-bit-2 multiplier at all, which is also
true of the PSP constructor - on PSP that field is written only by the pickup
code, together with the flag that gates it.

**The dead `gain`/`falloff` ramp reproduces exactly.** The function computes the
ramp into `craft+0x2e8`, clamps it at zero, and then overwrites it with the raw
input two statements later, so the thrust is computed from the input and not
from the ramped state:

```c
state = craft+0x2e8;
state += (state < controls->thrust) ?  Engine.gain    * dt: -Engine.falloff * dt;
craft+0x2e8 = state;
if (craft+0x2e8 < 0) craft+0x2e8 = 0;
craft+0x2e8 = controls->thrust;                 /* <- the ramp is discarded */
T = controls->thrust * Engine.amount;           /* <- and this uses the input */
```

engine.md scores that finding at 85 from one binary and flags it as the kind of
thing an obvious reimplementation gets wrong. **A different compiler on a
different ISA emitting the same dead store is about as strong as a second leg
gets**, and it should be read as corroborated rather than single-source.

Confidence **90** for the term as a whole.

Two plumbing differences, neither of which changes the physics:

- **The PS2 writes `body+0x110` directly** rather than accumulating into a craft
  member and draining it later. The value is the same.
- **It builds a genuine two-lane vector** for thrust and lift before the add.
  That is worth recording because engine.md carries a confidence-65 note about
  the PSP's `S221` being used without ever being loaded, and recommends a
  reimplementation write `accumulator.y = lift` rather than reproduce the
  register carry. The PS2 doing an ordinary two-lane add **supports that
  recommendation**: the intended semantics really are "thrust in one lane, lift
  in the other", not an accident of register allocation.

### Steering - identical, including the reverse-controls blend

`Ship_UpdateSteering` (`0x0015bf30`), reading `Turning` at block-relative
`0x38`/`0x3c`/`0x40` - absolute `0xcc`/`0xd0`/`0xd4`, i.e.
`gain`/`amount`/`falloff`:

```c
target = controls->steerX;
if (target > 0)  steer += (steer < target) ?  Turning.gain * dt : -Turning.falloff * dt;
else if (target < 0) steer += (steer > target) ? -Turning.gain * dt :  Turning.falloff * dt;
else { /* decay toward centre by Turning.falloff * dt, clamped to exactly 0 */ }
if (craft+0x2d4 == 5 || craft+0x2d4 == 6)  steer = 0;   /* `mode - 5 < 2` */

yaw = steer * Turning.amount;
if ((flags & 0x20) && *(craft+0x1e4) + 0x3f8 == 0)
    yaw = (steer + craft+0x318) * Turning.amount;       /* steering bias */
if ((flags & 0x40) && *(craft+0x1e4) + 0x3f8 == 0)
    yaw = (craft+0x31c > 1.0) ? -yaw : yaw * (1.0 - 2.0 * craft+0x31c);

Body_AddTorqueLocal(body, (0, yaw, 0));
```

**Every element of [engine.md](../psp-pulse-usa/engine.md)'s Steering section
reproduces**, including the asymmetric gain/falloff on both sides of zero, the
exact-zero clamp, the mode 5/6 lockout, the additive steering bias, and the
0-to-1 reverse-controls blend with its `> 1.0` full-inversion case. The
decompiler's lane extraction confirms the write lands on `.y`. Confidence
**90**.

Worth noting for [the sign convention](#the-angular-sign-convention-w_game---w_physics):
**there is no negation at the use site in either build.** `yaw = steer *
Turning.amount` is written straight into the local angular accumulator. So the
runtime measurement's `w_game = -w_physics` reading still rests on
`Turning.amount > 0`, and this second binary neither strengthens nor weakens
that - it rules out a hidden negation in the code, which is the part that could
have been checked statically.

### Lateral grip - identical, and it pins down `slidegrip`

`Ship_ApplyLateralGrip` (`0x0015c7c0`):

```c
if (craft+0x2c0 > 0) { craft+0x2c0 -= dt; return; }     /* timer gate */

gg = grip_ground; ga = grip_air;                        /* block-rel 0x10, 0x14 */
if (flags & 0x400) { gg *= 1.5f; ga *= 1.5f; }
airbrake = max(craft+0x2d8, craft+0x2dc);               /* max(L, R) */
lateral  = dot(velocity, craft+0x190);                  /* row 0 */
k        = airbrake * (0.01f - slidegrip) - 1.0f;       /* block-rel 0x58 */

body->localForce += lateral * gg * k * grounded;
body->localForce += lateral * ga * k * (1 - grounded);
```

`grip_ground` and `grip_air` sit at block-relative `0x10`/`0x14`, absolute
`0xa4`/`0xa8`; `slidegrip` at `0x58`, absolute `0xec`. All three are where
[engine.md](../psp-pulse-usa/engine.md)'s parser table puts them.

**This is the strongest single corroboration on the page.** engine.md derives
`(0.01 - slidegrip)` and the `-1` floor from the load-time `1e-4` scaling and
scores the interpretation at 90, arguing the coefficient "reaches exactly 0 at
full airbrake and `slidegrip = 0`, and stays at `-1` for `slidegrip = 100`."
The PS2 expression is literally `airbrake * (0.01 - slidegrip) - 1.0`, and both
endpoints fall out arithmetically: `100 * 0.01 - 1 = 0` and `100 * 0 - 1 = -1`.
**A derived interpretation reproduced as an explicit expression in a second
binary.** The `max(L, R)` factor, the two grounded/airborne terms, the write
straight to `body+0x110` rather than through a craft accumulator, and the timer
gate are all there too. Confidence **90**.

`craft+0x190` is therefore row 0, the axis engine.md's runtime measurement
identifies as pointing **left** rather than right.

One term engine.md does not record: **flag bit 10 multiplies both grip
coefficients by `1.5`.** The same bit gates the turbo add in
`Ship_UpdateEngine`, so it plausibly marks a boost or pickup state, but that is
a guess at confidence 40 and the bit is left unnamed.

### The accumulators are cleared on the body, not the craft

`Body_ClearAccumulators` (`0x0015d058`) is the routine per-frame clear: called
unconditionally from `World_StepBodies`, once per body, immediately after
`Body_Integrate` has consumed this frame's `body+0x100`/`+0x110`/`+0x120`/
`+0x130`. It zeroes all sixteen words, `w` lanes included, so the next frame's
`Ship_UpdateCraft` starts every accumulator at a plain `(0,0,0,0)`. Together
with `Ship_UpdateEngine` and `Ship_ApplyLateralGrip` writing `body+0x110`
directly, this says the PS2 **drops the four craft-side mirror accumulators
entirely** and has every term accumulate onto the body.
[engine.md](../psp-pulse-usa/engine.md) describes the PSP zeroing
`craft+0x320`/`+0x330`/`+0x340`/`+0x350` at the top of `Ship_UpdateCraft` and
draining them at the bottom.

**Same four accumulators, same four body offsets, one less copy.** That is a
plumbing difference, not a physics one - but it does mean the PSP's four craft
offsets have no PS2 counterpart, which is worth knowing before trying to carry a
craft-struct layout across. Confidence **90**.

A second function at `0x0015ca48` touches the same four fields and was
previously misidentified as this one - see
[below](#not-the-clear-a-second-function-on-the-same-four-fields) for why it
carries a different name.

## The call order matches the PSP's, all fifteen terms

`Ship_UpdateCraft` makes eighteen calls. Sixteen of them are the force pass, and
listing them in program order against
[engine.md](../psp-pulse-usa/engine.md)'s ordering table gives an **exact,
in-sequence match on every one of the fifteen terms**:

| # | PSP term | PS2 call site | PS2 target |
| ---: | --- | --- | --- |
| 1 | `Ship_UpdateEngine` | `0x00159bac` | `Ship_UpdateEngine` |
| 2 | `Ship_UpdateBrakes` | `0x00159be8` | `Ship_UpdateBrakes` |
| 3 | `Ship_UpdateAirbrakes` | `0x00159bf4` | `Ship_UpdateAirbrakes_q` |
| 4 | `Ship_UpdateSteering` | `0x00159c00` | `Ship_UpdateSteering` |
| 5 | `Ship_UpdatePitch` | `0x00159c0c` | `Ship_UpdatePitch` |
| 6 | `Ship_ApplyQuadraticDrag` | `0x00159c2c` | `Ship_ApplyQuadraticDrag` |
| 7 | inline gravity | `0x00159c38` | `Ship_ApplyGravity` |
| 8 | `Ship_UpdateHover` | `0x00159c44` | `Ship_UpdateHover` |
| 9 | `Ship_ApplyLateralGrip` | `0x00159c60` | `Ship_ApplyLateralGrip` |
| 10 | inline dead branch | `0x00159c78` | `FUN_00159fe0` |
| 11 | `Ship_ApplyWeathervaneTorque` | `0x00159c80` | `Ship_ApplyWeathervaneTorque` |
| 12 | `Ship_ApplyAngularDamping` | `0x00159c8c` | `Ship_ApplyAngularDamping` |
| 13 | `Ship_ApplyRollingResistance` | `0x00159c94` | `Ship_ApplyRollingResistance` |
| 14 | inline vertical damping | `0x00159c9c` | `Ship_ApplyVerticalDamping` |
| 15 | track-section force | `0x00159ca8` | `FUN_0015a300` |

**This is the strongest result on the page.** Ten of those fifteen were
identified by *content* before the order was read - independently, from
constants and parameter-block offsets - so the sequence is not a circular fit.
The five that were not are pinned by position between content-identified
neighbours in a sequence where every other slot lands exactly.

Two consequences:

- **[engine.md](../psp-pulse-usa/engine.md)'s ordering table is corroborated**, and
  with it the consequence that page draws from it: groundedness is one frame
  stale for the engine, brakes, drag, gravity and pitch, because hover is step 8
  and those five run before it. That was the single most behaviourally
  consequential claim on that page and it was single-source. It no longer is.
- **The PS2 outlines four terms the PSP has inline** - gravity, the dead branch,
  vertical damping, and the accumulator clear. Same fifteen steps, different
  inlining. The four PSP call sites named "inline" above are real calls here.

`Ship_ApplyVerticalDamping` (`0x00159e78`) is confirmed twice over: it is slot 14
positionally, **and** it holds the only other `-0.25` immediate
(`lui at,0xbe80`) in the craft path, which is the coefficient
[engine.md](../psp-pulse-usa/engine.md) gives for that term.

**Its body has now been read, and it raises the confidence to 92 while settling a
question the PSP side got wrong.** The whole function is 37 instructions, and
here they all are, in address order and with nothing elided - the FPU-to-VU
transfers matter, because the scalars are computed in `f1`/`f2` and have to reach
`vf1`/`vf3` through the integer file before any `vmulx` can use them:

```text
00159e78  addiu sp,sp,-0x20
00159e7c  clear f0                ; f0 = 0.0
00159e80  sd    ra,0x10(sp)
00159e84  lwc1  f3,0x2e0(a0)      ; f3 = craft+0x2e0, the grounded fraction
00159e88  c.eq.S f3,f0            ; grounded == 0 ?
00159e8c  nop
00159e90  bc1f  0x00159f04        ; NOT zero -> return, having done nothing
00159e94  _ld   ra,0x10(sp)       ; delay slot, on the return path
00159e98  lqc2  vf2,0x180(a0)     ; vf2 = craft+0x180, the up axis
00159e9c  vaddw.xyz vf3,vf0,vf0   ; vf3.xyz = (1,1,1), vf0 being (0,0,0,1)
00159ea0  lqc2  vf1,0x1b0(a0)     ; vf1 = craft+0x1b0, the linear velocity
00159ea4  move  a1,sp
00159ea8  vmul.xyz vf1,vf2,vf1    ; vf1 = up * velocity, component-wise
00159eac  lui   at,0x3f80
00159eb0  mtc1  at,f1             ; f1 = 1.0f
00159eb4  vadday.x ACC,vf1,vf1    ; ACC.x = vf1.x + vf1.y
00159eb8  vmaddz.x vf1,vf3,vf1    ; vf1.x = ACC.x + 1*vf1.z == dot(up, velocity)
00159ebc  lui   at,0xbe80
00159ec0  mtc1  at,f2             ; f2 = -0.25f
00159ec4  qmfc2 v0,vf1            ; v0 = bits of dot            (VU -> integer)
00159ec8  sub.S f1,f1,f3          ; f1 = 1.0 - grounded
00159ecc  mtc1  v0,f0             ; f0 = dot                    (integer -> FPU)
00159ed0  lw    a0,0x1ec(a0)      ; a0 = craft+0x1ec, the body
00159ed4  mfc1  v0,f2             ; v0 = bits of -0.25
00159ed8  mfc1  v1,f0             ; v1 = bits of dot
00159edc  qmtc2 v0,vf3            ; vf3.x = -0.25   (vf3 reused, no longer 1,1,1)
00159ee0  qmtc2 v1,vf1            ; vf1.x = dot
00159ee4  vmulx.xyzw vf2,vf2,vf1  ; vf2 = up * dot
00159ee8  mfc1  v0,f1             ; v0 = bits of (1 - grounded)
00159eec  qmtc2 v0,vf1            ; vf1.x = 1 - grounded        (vf1 reused)
00159ef0  vmulx.xyzw vf2,vf2,vf3  ; vf2 *= -0.25
00159ef4  vmulx.xyzw vf2,vf2,vf1  ; vf2 *= (1 - grounded)
00159ef8  jal   Body_AddForceWorld
00159efc  _sqc2 vf2,0x0(sp)       ; delay slot: the force, at a1 == sp
00159f00  ld    ra,0x10(sp)
00159f04  jr    ra
00159f08  _addiu sp,sp,0x20
```

**Both `vf1` and `vf3` are reused mid-function**, which is the only thing that
makes the listing hard to follow: `vf3` carries the `(1,1,1)` broadcast that
completes the dot product at `0x00159eb8` and is then overwritten with `-0.25` at
`0x00159edc`, and `vf1` carries the dot product and is then overwritten with
`1 - grounded` at `0x00159eec`. Read without that, the last three `vmulx` look
like they multiply by the wrong things.

so the term is `up * dot(up, velocity) * -0.25 * (1 - grounded)` - **term for term
the PSP inline at `0x08849c60`, on a different ISA and a different compiler.**
Confidence **92**.

**The scale is the grounded fraction, not the magstrip blend.** `crates/physics`
had it as `1 - magLockBlend`; both binaries say `1 - grounded`, and this page's
own hover section already establishes `craft+0x2e0` as the fraction that
accumulates `+0.25` per contacting probe. A grounded craft gets **no vertical
damping at all** in either build. The crate is corrected.

**One real behavioural difference between the builds, worth recording.** The PS2
returns early when `grounded != 0`, which makes its own `1 - grounded` factor
dead - it is always exactly `1.0` on the path that reaches the multiply. The PSP
has no early return and scales continuously, so a *half* contact
(`grounded == 0.5`) gets half the damping on PSP and **none** on PS2. The two
builds therefore differ on a one-probe contact. Confidence **85** on the
difference: both readings are direct, but nothing has been run to see it.

### The frame ordering, and where the velocity changes after the integrator

`World_StepBodies` is longer than the three lines quoted above; read end to end it
is six passes over the body array, and the order is what matters:

| Pass | Address | What it does |
| ---: | --- | --- |
| 1 | `0x0015df28`-`0x0015e2a8` | swept collision + position correction per body: builds a `10.0`-unit sweep along `normalize(velocity)`, clamps it into a box, sweeps `position + velocity*dt + sweep`, and on a hit backs the body off by `0.9 * length` through `Body_SetPosition` |
| 2 | `0x0015e2c0`-`0x0015e2ec` | virtual call through vtable slot `0x78`/`0x7c` |
| 3 | `0x0015e300`-`0x0015e348` | `*(u32 *)(entity + 0x370) = 0`, then slot `0x70`/`0x74` - **this is `Ship_UpdateCraft`** |
| 4 | `0x0015e360`-`0x0015e390` | `Body_Integrate` (`0x0015d088`), then `0x0015d058`, which zeroes `body+0x100`/`+0x110`/`+0x120`/`+0x130` |
| 5 | `0x0015e3a8`-`0x0015e3cc` | `0x0015efe8` for bodies with bit 0 of `body+0x3a4` set |
| 6 | `0x0015e3d8`-`0x0015e5d4` | `0x0012fc60` collects contacts, then per contact either `0x0015ea90` or **`Body_ResolveContactPair`** (`0x0015e600`) |

**Pass 6 runs after the integrator and before the next frame's
`Ship_UpdateCraft`, and it writes velocities directly.** That is the window
[force-balance-ground-truth.md](../../../physics/force-balance-ground-truth.md)
identifies from the PSP traces as the only place a `3.67 %`-per-frame speed loss
can be coming from - the deficit that looked like a missing `2.28 * fs` force and
is not a force at all. **This is the PS2-side corroboration of that finding: the
engine really does have a post-integrate pass that changes velocity outside every
accumulator on this page.** Confidence **85** for the ordering, read straight off
the basic-block sequence.

`Body_ResolveContactPair` (`0x0015e600`) is an ordinary two-body impulse
resolver. Its shape, with `a0` the contact, `a1` and `a2` the two bodies:

```text
relative velocity at the contact, from body+0x140 and body+0x150 via vopmula/vopmsub
vn = dot(relVel, contact+0x10)
if (vn > 0)  return;                       ; 0015e818, separating contacts are skipped
j  = (-1.1 * vn) / (a1->invMass + a2->invMass + angularTerm)
apply +/- j along the normal through 0x0015d980 and 0x0015cfb0 (+/-0.25 * contact+0x30)
Body_QueueDeferredImpulse(body, contact, contact+0x34)   ; twice, once per body
```

The `-1.1` is `0xbf8ccccd`, i.e. `-(1 + e)` for a restitution `e = 0.1`.
Confidence **85**; the `+0.25`/`-0.25` pair at `0x0015e9d0`/`0x0015ea24` and the
exact angular term were not worked through.

**`body+0x370` is a deferred-impulse queue count, and that closes the `sw zero,
0x370` both binaries perform.** `Body_QueueDeferredImpulse` (`0x0015ce28`)
appends a 0x40-byte record - a vector at `+0x190`, a second at `+0x180`, the
friction scalar `contact+0x34` at `+0x1a0` and a flag at `+0x170` - into an array
indexed by `body+0x370`, **capped at 8 entries**, then increments the count. Pass
3 zeroes that count immediately before `Ship_UpdateCraft` runs, which is exactly
what PSP's `0x0884ff54` does. So the field cleared before every craft update is
not a generic scratch word: it is the per-frame contact queue. Confidence **82**
for the queue reading, **75** for `contact+0x34` being friction specifically -
that comes from the PSP twin `FUN_0884e968` computing a tangential relative
velocity and scaling it by the same offset, not from a string.

### Not the clear: a second function on the same four fields

`0x0015ca48` writes the same four `body+0x100`/`+0x110`/`+0x120`/`+0x130`
fields as `Body_ClearAccumulators`, with `1.0` in each `w` lane rather than a
plain zero - which is what made the two look like duplicate identifications of
one function. They are not. `0x0015ca48` is called from exactly two places,
neither of them the per-frame pass: from `Ship_UpdateCraft`'s own tail, gated
on `*(int *)(craft+0x2d4) == 3`, and from `FUN_00159dd8`, the craft's state
setter (`case 3`), on the transition into that state. Both call sites also
call an unnamed twin, `FUN_0015ca78`, which zeroes `body+0x140` and `+0x160`
(the linear-velocity fields `Body_ResolveContactPair` reads) the same way -
`(0,0,0,1.0)`, not a plain zero.

So the pair together hold a body motionless - forces and velocity both reset
to identity every frame - for as long as `craft+0x2d4 == 3`. What state 3 is
was not fully pinned down, but its own setter (`FUN_001526d8`) is reached from
an off-track timer that fires after `0.5` s past a `200`-unit range gate, and
from a wall contact whose material reads `2`, both patterns fitting a
respawn/reset condition rather than anything routine. Renamed
`Body_ResetAccumulators`, confidence **75**: the mechanical behaviour (what it
writes, when it is called) is read directly, but the game-state label behind
"state 3" is inference, not confirmed by a string or a cross-checked second
build, so the name stops at the verb the content supports rather than naming
the trigger. `FUN_0015ca78` stays unnamed for the same reason ADR-0005 gives for leaving a
function alone below full confidence: read, but not yet worked through enough
to commit to a name - same as `0x0015ea90`, pass 6's other branch, and
`0x0015d980`/`0x0015cfb0`, the two impulse appliers.

`Ship_UpdateAirbrakes_q` (`0x0015a550`) carries the `_q` suffix per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md): it is
**positional only, at confidence 78, and its body was not read.** It is the one
name on this page not backed by content.

### Brakes - identical, and it settles a confidence-55 claim

`Ship_UpdateBrakes` (`0x0015a058`), reading `Brakes` at block-relative
`0x18`/`0x1c`/`0x20` - absolute `0xac`/`0xb0`/`0xb4`, i.e.
`gain`/`amount`/`falloff`:

```c
if (controls[0x08] > 0 && controls[0x0c] > 0)
     brake = min(brake + Brakes.gain    * dt, 100.0);
else brake = max(brake - Brakes.falloff * dt,   0.0);

if (brake > 0) {
    speed = |velocity|;
    dir   = (speed != 0) ? velocity / speed : velocity;
    if (speed < 10.0)  dir *= speed * 0.1;
    force = dir * brake * Brakes.amount;
    force.<one lane> = 0;                    /* explicitly zeroed */
    Body_AddForceWorld(body, force);
}
```

Every element of [engine.md](../psp-pulse-usa/engine.md)'s Brakes section
reproduces, including the both-airbrakes-at-once trigger, the `100.0` clamp and
the `speed < 10` fade. It also fixes `controls+0x08` and `+0x0c` as the two
airbrake axes. Confidence **90**.

**It corroborates that page's weakest claim.** engine.md reads a VFPU target
prefix as zeroing lanes of the brake force and scores it **55**, explicitly
flagged as "a caution rather than a claim" because the prefix encoding was never
confirmed. The PS2 zeroes a lane of the brake force with an ordinary explicit
instruction. That does not prove *which* lane - the decompiler does not say -
but it confirms the prefix was doing what that page guessed rather than nothing.
**Raise it to 78**: the behaviour is confirmed, the specific lane is not.

### Rolling resistance - matches, with one refinement

`Ship_ApplyRollingResistance` (`0x00159f10`):

```c
if (forwardSpeed > 0) {
    speed = |velocity|;
    d = (speed > 2.0) ? normalize(velocity) * 2.0 : velocity;
    Body_AddForceWorld(body, -d);
}
```

The `2.0` magnitude, the `forwardSpeed > 0` gate and the negation all match
[engine.md](../psp-pulse-usa/engine.md). **The negation is explicit here** - a
`0 - v` subtraction rather than a source prefix - which is a second-binary leg
for the other half of that page's confidence-55 prefix concern.

One refinement to that page's "a constant-magnitude opposing force, independent
of speed": **that holds only above 2 units/s.** Below it the raw velocity vector
is used instead of a normalised one, so the force tapers to zero at a standstill
rather than staying at magnitude 2. A reimplementation using a flat `-2 *
unit(v)` would apply a 2-unit shove to a nearly stationary ship. Confidence
**88**.

## Cross-platform

| Function | PS2 (`SCES_547.48`) | PSP (`BOOT.BIN`) |
| --- | --- | --- |
| `Ship_UpdateCraft` | `0x001596f8` | `0x08849618` |
| `Ship_UpdateHover` | `0x0015aff0` | `0x0884870c` |
| `Ship_HoverFourCorner` | `0x0015a940` | `0x0884ae90` |
| `Ship_HoverTwoPoint` | `0x0015b978` | `0x0884a658` |
| `Ship_UpdateMagLock` | `0x0015b070` | `0x0884ba0c` |
| `Ship_ApplyAngularDamping` | `0x0015c1b0` | `0x08848ed0` |
| `Body_AddForceWorld` | `0x0015dac8` | `0x0884d4c8` |
| `Body_AddTorqueLocal` | `0x0015dda0` | `0x0884d5bc` |
| `Body_AddTorqueWorld` | `0x0015ddb8` | `0x0884d604` |
| `Body_AddForceAtPoint` | `0x0015da60` | not located |
| `Body_Integrate` | `0x0015d088` | not located |
| `World_StepBodies` | `0x0015ded8` | the unnamed function at `0x0884f70c` |
| body accumulators | `+0x100` / `+0x110` / `+0x120` / `+0x130` | `+0x100` / `+0x110` / `+0x120` / `+0x130` |

## Not determined

- **The sub-step count.** See above.
- ~~**Where the torque sign is compensated.**~~ **Answered**, and the answer is
  that it is not compensated anywhere because it does not need to be. See
  [the angular sign convention](#the-angular-sign-convention-w_game---w_physics).
  The matrix at `body+0xc0`..`+0xf0` is not the site: it is built as the
  adjugate of the basis scaled by `1/det` (`0x0015d478`..`0x0015d4f0`), i.e. its
  inverse, which for the orthonormal basis is the transpose - exactly what
  [engine.md](../psp-pulse-usa/engine.md)'s runtime dump measured element for
  element. A rotation inverse has determinant `+1` and cannot flip a sign.
- **`Ship_UpdateAirbrakes_q`'s body was never read.** It is slot 3 of fifteen
  and nothing else fits there, but it is the one name on this page resting on
  position alone. [engine.md](../psp-pulse-usa/engine.md) makes substantive claims
  about that term - the `sideshift` write straight to the body, the airbrake
  `turn` and `drag` contributions - and **none of them are corroborated here.**
  This is the cheapest remaining item on the page.
- **`FUN_00159fe0`**, slot 10, which is [engine.md](../psp-pulse-usa/engine.md)'s
  "inline dead branch". It is a real call on the PS2, so **it may not be dead
  here**; that page's finding that the in-air roll-levelling term computes a
  value and never stores it needs re-checking against this build rather than
  assuming it carries over.
- **`FUN_0015cea0`, `FUN_0015ca78`** - the two calls outside the fifteen-term
  sequence, one before it and one after the accumulator clear. Not examined.
- **Whether the PS2 has the PSP's dead in-air roll-levelling branch**, and
  whether `Ship_UpdateMagLock` writes an angular Z component. Both are open on
  [engine.md](../psp-pulse-usa/engine.md) too.
- **Whether the angular accumulators hold torque or angular acceleration.**
  `Body_Integrate` divides the force accumulator by a mass term at
  `body+0x370+8` but applies the angular accumulator through the `+0xc0` matrix
  with no visible inertia division, which hints at angular acceleration - not
  enough to claim it. The sign result above is unaffected either way: an inertia
  tensor is positive definite and cannot invert a sign.
- **Which frame `body+0x150` and `body+0x160` are each expressed in.** The
  matrix at `body+0x80` maps the second onto the first; it is a rotation, so it
  cannot change a sign, but it was not identified.
- **Nothing here was verified at runtime**, except the two fields below,
  added 2026-09-16 - everything else on this page is still decompilation
  without a trace.

## `steer` and `brake`, found live rather than read

Neither field is assigned a craft offset anywhere above - [the steering
section](#steering---identical-including-the-reverse-controls-blend) and
[the brakes section](#the-control-terms) both show `steer`/`brake` only as
local pseudocode variables. A live PCSX2 session (`scripts/pcsx2-trace.py`,
`docs/reverse-engineering/pcsx2-debugger.md`) found both by diffing the
craft block across a held input rather than by reading more code, on a
savestate taken on Moa Therma White's grid.

**`steer` is `craft+0x2f0`.** From rest (`0.0`) it ramps to `-64.425` after
20 verified frames holding `left` and to **exactly** `+64.425` after 20
frames holding `right` from the same anchor - the same magnitude both
signs - and decays to exactly `0.0` within 60 frames of release. All three
match [`Ship_UpdateSteering`](#steering---identical-including-the-reverse-controls-blend)'s
pseudocode verbatim: ramps from zero, symmetric about centre, clamped to
exactly `0` at rest. Confidence **90**: it was the only word in the whole
craft block (`0x000`-`0x330`) that moved this way under `left`/`right`, and
four independent live measurements agree.

**`brake` is `craft+0x2ec`.** From rest it ramps smoothly to `35.4` after 20
frames holding **both** `l1` and `r1` (not a snap to a fixed value the way
`airbrake_l`/`airbrake_r` at `craft+0x2d8`/`craft+0x2dc` do - the six other
candidate words that also moved under the same input all jumped straight to
`100.0`, which is what told this one apart), reaches exactly `100.0` by 60
frames (the documented clamp), decays to exactly `0.0` within 60 frames of
release, and - the decisive check - **stays at `0.0` for 20 frames holding
`l1` alone**. That last one matches
[`Ship_UpdateBrakes`](#the-control-terms)'s `controls[0x08] > 0 &&
controls[0x0c] > 0` gate exactly: one airbrake does nothing, both together
ramp it. Confidence **90**, same basis as `steer`.

Both offsets, and the method that found them, are recorded with full
citations in `scripts/pcsx2_trace_fields.py`. The craft address itself
(`0x00720f20` on this session's savestate) is not repeated here - it is
instance data specific to that savestate, not a structural fact about this
binary.

## History

- 2026-07-27: first pass, driven by the physics investigation's stability
  question. `-400` and the damping triple confirmed at 92 in a second binary;
  the frozen-accumulator sub-step finding at 88; the `F x r` observation
  recorded as a systematic-sign clue at 85 without a claimed resolution.
- 2026-07-27, second pass: the `vopmula`/`vopmsub` operand convention settled at
  95 against the *VU User's Manual* v6.0 plus a decode of the four raw
  encodings, after confirming Ghidra models both as semantics-free
  `CALLOTHER`s and prints them in mnemonic rather than encoding order. This
  **refutes** the reversed-convention explanation for the sign anomaly and
  leaves two candidates. Also corrected the sub-step stability derivation: the
  earlier edition wrongly froze the position update at the full frame step as
  well, giving `c >= 6.67`; the position update does advance by the per-sub-step
  `h`, so the requirement is `c >= k*H*(n+1)/(2n)` - 4.44 at three sub-steps.
  The conclusion that raising the sub-step count cannot fix the instability is
  unaffected.
- 2026-07-27, fourth pass: nine of `Ship_UpdateCraft`'s sixteen callees
  identified by constant fingerprint and checked term by term against
  [engine.md](../psp-pulse-usa/engine.md). Engine, pitch, steering, lateral grip,
  weathervane and the track-section force corroborate that page **completely**,
  including its dead-`Engine.gain`/`falloff` finding and its derived
  `(0.01 - slidegrip)` grip coefficient. Drag matches on three of four
  coefficients and gravity differs in two details, both recorded as differences
  rather than corrections.
- 2026-07-27, fifth pass: the call order read, and **all fifteen terms match
  [engine.md](../psp-pulse-usa/engine.md)'s ordering table exactly and in
  sequence**, which corroborates that page's stale-groundedness consequence.
  Brakes and rolling resistance confirmed by content, settling both halves of
  that page's confidence-55 VFPU-prefix concern; vertical damping confirmed by
  content and position together. Fourteen of the fifteen terms are now named,
  the fifteenth (`Ship_UpdateAirbrakes_q`) positionally only.
- 2026-07-27, third pass: **the sign question is closed.** The two remaining
  explanations were checked and both are dead - the hover force is a positive
  multiple of `up` at the call site, and the `+0xc0` matrix is a plain rotation
  inverse. The resolution is that `Body_Integrate` integrates the basis as
  `e' = e x w`, so the engine's angular velocity is the negative of the textbook
  one and `F x r` and `-400 * (up x n)` are both **correct** rather than
  inverted. Confidence 88 in the PS2 build, 80 cross-platform. Also fixed
  `Body_AddForceAtPoint`'s argument order: it is `(body, force, point)`, which
  the prologue's `a1 -> s2` / `a2 -> s1` swap disguises.
- 2026-09-16: the first runtime leg on this page's own build, driven by
  `scripts/pcsx2-trace.py` needing a working capture. Found `steer`
  (`craft+0x2f0`) and `brake` (`craft+0x2ec`) live rather than by reading
  more decompiled code - see [the section above](#steer-and-brake-found-live-rather-than-read).
  Both confirm their respective pseudocode sections rather than correcting
  them. A separate, PCSX2-transport-specific finding from the same session -
  the game's own physics tick runs at half PCSX2's verified-frame rate,
  because PAL is interlaced - is recorded in
  [pcsx2-debugger.md](../../../reverse-engineering/pcsx2-debugger.md) rather
  than here, since it is about the capture transport, not this binary's
  code.
