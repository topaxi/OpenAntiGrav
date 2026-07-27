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
below 70.

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
| `0x0015b978` | `Ship_HoverTwoPoint` | 85 |
| `0x0015ded8` | `World_StepBodies` | 80 |
| `0x0015b070` | `Ship_UpdateMagLock` | 78 |
| `0x0019f020` | `Handling_LoadForTeam` | 78 |

## The dispatch, and why the names above are trustworthy

`World_StepBodies` (`0x0015ded8`) walks the body array and, per entity, does:

```c
*(u32 *)(entity + 0x370) = 0;
vtable = *(int *)(body + 0x38);
(**(code **)(vtable + 0x74))(dt, body + *(short *)(vtable + 0x70), entity);
```

[engine.md](../psp-pulse/engine.md) records the PSP call site as a vtable at
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

[engine.md](../psp-pulse/engine.md) gives the PSP selector as
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
cyclically the same statement as [engine.md](../psp-pulse/engine.md)'s
runtime-measured `cross(row0, row1) = row2` on 200 of 200 ticks. PS2 code and PSP
measurement agree the basis is positively oriented under the ordinary
component-wise cross product. Handedness is not the variable. The sign of `w`
is, and it is a separate question that was being conflated with it.

**Two independent legs, and the second is blind to the first's one assumption.**

- *The instruction read above* assumes only that the matrix at `body+0x80`,
  which maps `+0x160` onto `+0x150`, does not flip a sign. It is built from the
  orthonormalised basis, so it is a rotation and its determinant is `+1`. Which
  frame `+0x150` and `+0x160` are each expressed in is **not** settled here, and
  [engine.md](../psp-pulse/engine.md)'s deliberate confidence-74 cap on the
  local/world split of the angular accumulators stands untouched.
- *[engine.md](../psp-pulse/engine.md)'s weathervane term* needs no assumption at
  all: `angularWorld += cross(forward, velocity) * (grounded ? -0.1 : -0.3)`,
  with the coefficients as hardcoded literals and the intended effect - turning
  the nose toward the direction of travel - already stated there. That is
  backwards under the textbook convention and correct under this one, from a
  third term arrived at independently of the other two.
- *[engine.md](../psp-pulse/engine.md)'s steering measurement* is end-to-end,
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
here matches [engine.md](../psp-pulse/engine.md) term for term: yaw damping is a
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
| body accumulators | `+0x100` / `+0x120` / `+0x130` | `+0x100` / `+0x120` / `+0x130` |

## Not determined

- **The sub-step count.** See above.
- ~~**Where the torque sign is compensated.**~~ **Answered**, and the answer is
  that it is not compensated anywhere because it does not need to be. See
  [the angular sign convention](#the-angular-sign-convention-w_game---w_physics).
  The matrix at `body+0xc0`..`+0xf0` is not the site: it is built as the
  adjugate of the basis scaled by `1/det` (`0x0015d478`..`0x0015d4f0`), i.e. its
  inverse, which for the orthonormal basis is the transpose - exactly what
  [engine.md](../psp-pulse/engine.md)'s runtime dump measured element for
  element. A rotation inverse has determinant `+1` and cannot flip a sign.
- **The remaining `Ship_Update*` control terms** - engine, brakes, steering,
  pitch, airbrakes, drag, weathervane. `Ship_UpdateCraft` was read only far
  enough to find the hover and damping calls; the ordering table on
  [engine.md](../psp-pulse/engine.md) is **not** corroborated here.
- **Whether the angular accumulators hold torque or angular acceleration.**
  `Body_Integrate` divides the force accumulator by a mass term at
  `body+0x370+8` but applies the angular accumulator through the `+0xc0` matrix
  with no visible inertia division, which hints at angular acceleration - not
  enough to claim it. The sign result above is unaffected either way: an inertia
  tensor is positive definite and cannot invert a sign.
- **Which frame `body+0x150` and `body+0x160` are each expressed in.** The
  matrix at `body+0x80` maps the second onto the first; it is a rotation, so it
  cannot change a sign, but it was not identified.
- **Nothing here was verified at runtime.** The convention result is the one
  claim on this page with a runtime leg, and that leg is a measurement on the
  *other* build.

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
- 2026-07-27, third pass: **the sign question is closed.** The two remaining
  explanations were checked and both are dead - the hover force is a positive
  multiple of `up` at the call site, and the `+0xc0` matrix is a plain rotation
  inverse. The resolution is that `Body_Integrate` integrates the basis as
  `e' = e x w`, so the engine's angular velocity is the negative of the textbook
  one and `F x r` and `-400 * (up x n)` are both **correct** rather than
  inverted. Confidence 88 in the PS2 build, 80 cross-platform. Also fixed
  `Body_AddForceAtPoint`'s argument order: it is `(body, force, point)`, which
  the prologue's `a1 -> s2` / `a2 -> s1` swap disguises.
