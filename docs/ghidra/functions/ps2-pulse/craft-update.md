# The craft update and the rigid body (PS2)

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

This page exists to answer a specific question that
[docs/physics/README.md](../../../physics/README.md) could not settle from the
PSP binary alone: the surface-alignment torque's gain and sign, the roll damping
beside it, and whether the sub-step count rescues a pair that reads as
marginally unstable. It answers two of those, contradicts the third, and leaves
the sign question open in a more useful shape than it was.

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

The operand order is not in doubt either. The EE computes a cross product as
`vopmula.xyz ACC, A, B` followed by `vopmsub.xyz D, B, A`, giving `D = A x B`;
here `A` is the up axis (`craft+0x180`) and `B` is the averaged contact normal.

## `Body_AddForceAtPoint` computes `F x r`, and that is the useful clue

```
0015da84  lqc2  vf1,0x30(s0)      ; body position
0015da8c  lqc2  vf2,0x0(s1)       ; the world point
0015da94  vsub  vf2,vf2,vf1       ; r = point - position
0015da98  lqc2  vf1,0x0(s2)       ; the force
0015da9c  vopmula.xyz ACC,vf1,vf2 ; A = force, B = r
0015daa0  vopmsub.xyz vf1,vf2,vf1 ; d = A x B = force x r
0015daa4  jal   Body_AddTorqueWorld
```

Physical torque is `r x F`. This is `F x r`, its negative - and unlike the
alignment gain, **this term's correct sign is fixed by mechanics rather than by
anyone's reading of the prose.** It is the hover spring's own torque: the force
that pushes each corner probe up, applied at that probe.

That matters because it makes the anomaly **systematic**. Two terms in the same
accumulator, arrived at independently, both carry a sign that the ordinary
convention says is backwards. [engine.md](../psp-pulse/engine.md) predicted
exactly this - "if both of those terms really do need flipping, the sign lives
downstream, in how the angular accumulators are applied to the body" - and this
is a third instance of the same shape, on the term where the intended answer is
least arguable.

**Where the compensation happens was not found, and this page does not claim
it.** `Body_AddTorqueWorld` (`0x0015ddb8`) is a plain `+=` into `body+0x130`.
`Body_Integrate` reads `body+0x130`, rotates it through the matrix at
`+0xc0`..`+0xf0` and adds it to the angular velocity at `+0x160`, also with a
plain `+=` and no negation. So on the path actually read, nothing flips it.

Since the retail PS2 game demonstrably does not spin its ships inside-out, one of
these is true and this page cannot yet say which:

- the compensation is in the `+0xc0` matrix or the inertia handling, neither of
  which was read;
- `Ship_HoverFourCorner` passes an already-negated force to
  `Body_AddForceAtPoint`, which was not checked component by component;
- the EE's `vopmsub` operand convention is the reverse of the one used here,
  in which case **all three terms flip together** and everything is consistent
  with the prose.

The third is worth stating plainly because it is the cheapest to settle and it
would resolve the whole question at once. Confidence **85** that the anomaly is
systematic rather than three independent errors; confidence **0** on which of
the three explanations is right.

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

That has a consequence the physics page's arithmetic does not currently account
for. With the torque frozen at its frame-start value, the velocity change over a
whole frame is

```text
Δv = (−k·x₀ − c·v₀) · n·h = (−k·x₀ − c·v₀) · H
```

which is **one explicit-Euler step at the full frame step `H`, whatever `n`
is**. So the stability condition is `H <= c/k`, not `H/n <= c/k`. At `k = 400`
and `c = 2` that needs `H <= 0.005` against an actual `H = 1/60 = 0.0167`: a
factor of about **3.3**, not the 1.11 the page computes, and **raising the
sub-step count cannot fix it**, because it does not enter.

This does not resolve the instability - it makes it larger. It does eliminate
one of the three explanations that page offers ("the sub-step count is not
three"), which leaves a wrong magnitude or a genuinely marginal original.
Confidence **88**: the loop structure is unambiguous, and the arithmetic follows
from it. Not higher because it is decompilation without a trace, and because the
inertia handling between `body+0x130` and `body+0x160` was not read - if the
rotation at `+0xc0` is not orthonormal, the effective `k` and `c` both scale and
the *ratio* still does not move, but the argument deserves that caveat stated.

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
- **Where the torque sign is compensated.** See above. This is the highest-value
  thing left, and confirming the `vopmsub` operand convention against a
  reference would settle it in minutes.
- **The remaining `Ship_Update*` control terms** - engine, brakes, steering,
  pitch, airbrakes, drag, weathervane. `Ship_UpdateCraft` was read only far
  enough to find the hover and damping calls; the ordering table on
  [engine.md](../psp-pulse/engine.md) is **not** corroborated here.
- **The inertia handling**, and whether the angular accumulators hold torque or
  angular acceleration. `Body_Integrate` divides the force accumulator by a mass
  term at `body+0x370+8` but applies the angular accumulator through the `+0xc0`
  matrix with no visible inertia division, which hints at angular acceleration -
  not enough to claim it.
- **Nothing here was verified at runtime.**

## History

- 2026-07-27: first pass, driven by the physics investigation's stability
  question. `-400` and the damping triple confirmed at 92 in a second binary;
  the frozen-accumulator sub-step finding at 88; the `F x r` observation
  recorded as a systematic-sign clue at 85 without a claimed resolution.
