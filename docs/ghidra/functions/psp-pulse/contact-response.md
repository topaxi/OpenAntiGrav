# The contact response: where the friction coefficient comes from

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`. **The names here are applied**, from [names.tsv](names.tsv).

[rigid-body.md](rigid-body.md#the-contact-response-and-the-friction-law-the-force-balance-was-missing)
reads `Body_ResolveContact` (`0x0884e968`) and establishes the *shape* of the
response: a normal impulse plus a tangential impulse `-contact->friction * vt`
which, because `invMass == 1`, is a flat multiplicative loss on the tangential
velocity every frame a contact exists. It leaves one question open, and calls it
the critical one:

> What a reimplementation needs, in order: the writer of `contact+0x34` [...]

**That writer is `Collision_AddContact` (`0x08816864`), and the coefficient it
produces for a craft against a wall is `0.035`.** Both inputs are literals in the
binary, so the number is a *prediction*, not a fit - and
[force-balance-ground-truth.md](../../../physics/force-balance-ground-truth.md)'s
capture confirms it one-sidedly.

## `Collision_AddContact` fills `contact+0x34` by averaging the two colliders

The tail of the function, at `0x08816a14`-`0x08816a70`:

```text
08816a14  lw    a0,0x60(s2)      ; colliderA->ownerId  -> contact+0x24
08816a20  lw    a0,0x60(s3)      ; colliderB->ownerId  -> contact+0x2c
08816a28  lwc1  f13,0x64(s2)     ; colliderA->friction
08816a2c  c.lt.s f13,f12         ; f12 == 0.0
08816a34  bc1t  0x08816a54       ; A negative -> result 0.0
08816a3c  lwc1  f12,0x64(s3)     ; colliderB->friction
08816a44  c.lt.s f12,f13         ; f13 == 0.0 here
08816a4c  bc1fl 0x08816a5c       ; B not negative -> average the two
08816a54  b     0x08816a70       ; (either negative)
08816a58  _mtc1 zero,f12         ;   result 0.0
08816a5c  lwc1  f13,0x64(s3)
08816a60  lui   a0,0x3f00        ; 0.5
08816a64  add.s f12,f12,f13
08816a6c  mul.s f12,f12,f14      ; (fA + fB) * 0.5
08816a70  swc1  f12,0x34(s0)     ; -> contact+0x34
```

So `contact->friction = (A + B) * 0.5`, **forced to zero if either side is
negative**. Confidence **90**: nine instructions, no VFPU, no branch not shown.

`s2` is the mesh collider (the one carrying the sweep-and-prune index, the
vertices and the triangle indices) and `s3` is the box collider - the ship. Both
contribute.

**This is the rule [collision.md](collision.md) already records, and it was
recorded against the wrong field.** That page calls `collider+0x64` *restitution*
and describes exactly this averaging-with-a-negative-sentinel combination. The
combination rule is right; the field's identity was not. `Body_ResolveContact`
takes its restitution from `body+0x388` and uses the combined `collider+0x64`
**only** to scale the tangential relative velocity. The `-1.0` is still a
sentinel, and it still means "the response ignores this surface" - it just means
frictionless rather than non-bouncing.

## The two literals

| Collider | `+0x64` | Written by | Literal |
| --- | ---: | --- | --- |
| `Wall Collision` | `0.05` | `CollisionNode_ParseChunks` `0x08934c48` | `0x3d4ccccd` in `f22` |
| `Floor`, `Mag Floor`, `Reset` | `-1.0` | `CollisionNode_ParseChunks` `0x08934c04`/`0x08934c84` | `0xbf800000` in `f20`, set at `0x08934ad8` |
| the craft's box | `0.02` | `Ship_SetColliderFriction` `0x0884da7c`, from the ship-entity constructor `0x08840c74` | `0x3ca3d70a`, loaded at `0x088414e4` |

`Ship_SetColliderFriction` is eleven instructions: it writes `entity+0x390` and
then the same float into `collider+0x64` of whichever of the entity's two
colliders (`+0x3ac` or `+0x3bc`) the mode word at `+0x3a0` selects. Confidence
**85**; the name carries `Ship_` because the ship-entity constructor is its only
caller.

The constructor has a second branch that passes `0.0` instead
(`0x08841500`), taken when a global byte at `0x08ab07e3` is clear **and** the
entity's class word at `*(sp+0x1ac) + 0xb8` reads `6`. Which class that is has not
been read, so the `0.0` branch is recorded and not modelled. Confidence **80**
on the branch structure, **50** on it never applying to a racing craft - the
trace is what says a racing craft takes the `0.02` branch.

**So a craft against a wall gets `contact->friction = (0.05 + 0.02) / 2 = 0.035`,
and against a floor, a magstrip or a reset volume it gets `0.0`.** The zero is
why the standing start reads `speed / |velocity| == 1.0000` for its first
sixty-one ticks while sitting on the track: the only surface with friction is a
wall.

## `contact+0x10` is a unit normal, and `contact+0x30` is the penetration depth

Both matter, because the resolver's `j`, its denominator and its tangential split
all assume `|n| == 1`, and none of that is tested by the speed measurement below.

`Collision_AddContact` builds a contact only when the box sample point is
**behind** the triangle plane by less than `2.0`:

```text
088168e4  vdot.t S220,C200,C210   ; d = dot(normal, samplePoint - v0)
08816918  c.lt.s f22,f13          ; f13 == 0.0
08816920  bc1f  0x08816a7c        ;   d >= 0  -> no contact
08816930  c.le.s f22,f12          ; f12 == -2.0
08816938  bc1t  0x08816a7c        ;   d <= -2 -> no contact
```

and then `contact+0x30 = -d`, the depth, at `0x08816a08` (the `f20` alternative
in that `bc1fl` needs `d < -5.0` and is therefore dead). The resolver's
`Body_Translate(body, n * contact+0x30)` pushes the body toward the plane's front
face, which is the side the normal points at and the side the ship is not on.

The normal itself is the third out-parameter of `Collision_SegmentTriangle`
(`0x08818bdc`), and that function normalises it before storing:

```text
08818c68  vdot.t S001,C330,C330
08818c6c  vrsq.s S000,S001        ; 1 / |N|
08818c8c  vscl.t C330,C330,S000   ; N /= |N|
08818d48  sv.q   C330,0x0(t3)     ; third out-param
```

Confidence **90**. The other two out-parameters are the intersection point
(`t2`) and the normalised plane distance of the segment start (`t1`).

## `Body_RecordContact` (`0x0884dbf4`) is a record, not physics

The third call `Body_ResolveContact` makes, which
[rigid-body.md](rigid-body.md) leaves unread, appends to a per-body ring of eight
`0x40`-byte slots: the impulse at `body + 0x190 + n*0x40`, the contact point at
`+0x180`, the friction at `+0x1a0`, `contact+0x20` at `+0x1a4`, zero at `+0x170`,
with the count at `body+0x370` and a hard `n < 8` reject. `Body_StepWorld`
(`0x0884f70c`) zeroes `body+0x370` once per frame at `0x0884ff54`, before the
per-body update callback runs.

**The consumer is the ship entity, not the physics step.** `FUN_088418e0` loads
`body+0x370` at `0x088423ac` as a loop bound and walks `body + 0x170 + n*0x40`
from `0x0884242c`, taking `vdot.t S220,C200,C200` on the impulse slot at `+0x20`
of each record - a squared-magnitude test against literals `0.7`, `0.0125` and
`10000.0`. That is the gameplay reaction to a hit (which is where
`Ship_ApplyCollisionImpulse`'s pending vector at `entity->0x4c + 0x110` comes
from, and what the damage and audio paths need), and **nothing in the loop writes
a velocity back**.

So the ring is an **observation buffer**, and the friction impulse is applied
exactly once - inline, in `Body_ResolveContact`. Confidence **85** on the
structure, and the measurement forbids the alternative independently: a second
application would make the per-frame loss `1 - 0.965² = 6.88 %`, where the
capture never exceeds `5.21 %` and settles at `3.56 %`.

## Frame order, from `Body_StepWorld` (`0x0884f70c`)

Worth writing down, because the whole force-balance investigation turned on
*when* the velocity is touched relative to when the trace samples it:

1. Per body: a swept move through `Collision_RaycastWorld` and `Body_SetPosition`
   (`0x0884d840`).
2. Per body: the vtable `+0x78` callback.
3. Per body: `body+0x370 = 0`, then the vtable `+0x70` callback - **this is where
   `Ship_UpdateCraft` runs and the force accumulators are filled**.
4. Per body: `Body_Integrate(body, subSteps, dt)`, then `0x0884e478`. **`body+0x398`
   is written here**, as `|velocity|`.
5. Broadphase and narrowphase (`0x088159c0`), producing up to 128 contacts.
6. Per contact: `Body_ResolveContact` (one body) or `0x0884ef30` (two bodies).

Step 6 runs *after* step 4 and there is no second integration, so the velocity a
craft carries into the next frame's step 3 is the post-contact one while
`body+0x398` still holds the pre-contact magnitude. That is exactly the
`speed / |velocity|` ratio the capture records, and it is a **direct read of the
per-frame contact loss** with nothing else in it. Confidence **88**.

## What the trace says, and why it is a one-sided test

With `n` the wall normal, `e = 0.4` and `D` the resolver's denominator, one
contact leaves

```text
|v'|^2 = vn^2 * (1 - (1 + e)/D)^2 + (1 - f)^2 * |vt|^2
```

The tangential factor is fixed at `1 - f = 0.965`. The normal factor is smaller
than that for any `D` above about `0.7`, so **moving speed out of the tangent and
into the normal can only increase the total loss**. The per-frame loss therefore
has `f` as a hard *floor*, approached from above as the motion becomes purely
tangential - never crossed.

`data/traces/talons-junction-standing-start.csv`, from the tick the craft meets
the wall:

| Tick | `1 - \|v\| / speed` |
| ---: | ---: |
| 66 | `5.21 %` |
| 67 | `4.03 %` |
| 70 | `3.885 %` |
| 90 | `3.797 %` |
| 150 | `3.636 %` |
| 200 | `3.577 %` |
| 295 | `3.560 %` |

Monotone after the impact transient, and **above `3.5 %` on every one of the 230
contact ticks**, converging on it. A friction of `0.036` is falsified outright by
tick 295 alone; a friction of `0.03` would leave a `0.5 %` gap that never closes.
The prediction `0.035` is the only value in that neighbourhood the data admits,
and it was read out of two literals before the trace was consulted.

Confidence **90** for the whole chain: the combination rule is nine plain
instructions, the two constants are single literals on branch-free paths, the
resolver is read at 88, and the measurement is one-sided rather than a best fit.

## Where the PS2 build diverges, and why this crate follows the PSP

The PS2 twin was read independently, and
[ps2-pulse/craft-update.md](../ps2-pulse/craft-update.md) records it: six passes
per frame in `World_StepBodies` (`0x0015ded8`), with contact collection and
`Body_ResolveContactPair` (`0x0015e600`) in pass 6, after `Body_Integrate` and
before the next frame's `Ship_UpdateCraft`. **That is instruction-level
corroboration of the post-integrate velocity window from a different ISA and
compiler**, and it is the part that matters most, because the whole force-balance
investigation turned on it.

Three things differ, and `crates/physics` implements the **PSP** reading in each,
per the project's PSP-over-PS2 policy. Recorded here so the divergence is a
finding rather than a discrepancy someone rediscovers:

| | PSP (implemented) | PS2 (recorded) |
| --- | --- | --- |
| Restitution | `body+0x388`, a per-body field the ship ctor sets to **`0.4`** | a hardcoded `-1.1` numerator, i.e. **`e = 0.1`** |
| Separating contacts | **no test** - `Body_ResolveContact` applies the normal impulse whatever the sign of `vn`, so the contact is two-sided | **early return when `vn > 0`**, a one-sided push |
| Friction | folded **inline** into the same impulse, `j*n - f*vt`, applied once by `Body_ApplyImpulseAtPoint` | **deferred** through `Body_QueueDeferredImpulse` (`0x0015ce28`) into the 8-entry queue |

The PSP's queue - `Body_RecordContact` above - is the structural twin of the PS2's
deferred queue (same `body+0x370` index, same 8-entry cap, same `0x40` stride),
but it is a consumer-side record on this build rather than a work list, as the
loop in `FUN_088418e0` shows. **The `0.4`-versus-`0.1` restitution split is the
one a reader is most likely to trip on**: both are real, and only the PSP's is
what a Pulse-PSP reimplementation should carry.

The PSP's `Body_ResolveContact` is the single-body variant - its denominator is
`invMass + angularTerm`, where the PS2's pair resolver has
`invMassA + invMassB + angularTerm`. `0x0884ef30` is the PSP's pair twin and is
unread; a craft against track geometry takes the single-body path.

## This corrects two things on other pages

- **[collision.md](collision.md) and [`docs/formats/collision.md`](../../../formats/collision.md)
  call `collider+0x64` restitution.** It is friction. Both are updated; the
  sentinel rule and the values survive untouched, and the reasoning that
  established them was never wrong about anything but the name.
- **[rigid-body.md](rigid-body.md) reads the `4.2 % -> 3.67 %` decay as rising
  toward the coefficient.** It falls toward it. The coefficient is `0.035`, not
  `0.0367`, and the residual above it is the normal impulse, which adds loss
  rather than subtracting it. That page's confidence-80 note on the decay is
  superseded by the arithmetic above.

## Not determined

- **What entity class `+0xb8 == 6` is**, and what the global byte at `0x08ab07e3`
  gates - the branch that gives a collider `0.0` friction instead of `0.02`.
- **`Ship_ApplyCollisionImpulse`'s pickup-flag `0x10`.** Narrowed from
  "unread" to a concrete location: the word is at `*(entity+0x4c) + 0x1b8`, and
  bit `0x10` being set suppresses both the forward-projected impulse *and* the
  `craft+0x290 += 0.5` stun. What sets it is still unread, so the
  confidence-75 wall attribution in
  [force-balance-ground-truth.md](../../../physics/force-balance-ground-truth.md)
  stays capped where it is.
- **`0x0884ef30`**, the two-body contact resolver. The one-body path above is
  what a craft against track geometry takes; ship-to-ship goes through the other.
