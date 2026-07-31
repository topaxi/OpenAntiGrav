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

### Re-measured 2026-07-29, on an independent capture

The recording above was lost with the rest of `data/traces/` and has been retaken
from `verification/scenarios/standing-start.inputs`. **It is not the same run**:
the craft meets the wall at tick **186** rather than 66, and does so at 119
units/s rather than 54. What it does to the prediction:

| | 2026-07-28 | 2026-07-29 |
| --- | ---: | ---: |
| Contact ticks | 230 | 114 |
| Loss on the impact tick | `5.21 %` | `3.835 %` |
| Loss at the end of the run | `3.560 %` | `3.633 %` |
| **Minimum loss anywhere in contact** | above `3.5 %` | **`3.534 %`**, at 104 units/s |
| Contact ticks above 5 units/s below the `0.035` floor | - | **0 of 114** |

The floor holds on data that did not exist when it was predicted, from a
different impact at twice the speed. The launch half of the same capture
reproduces the other leg too - see
[force-balance-ground-truth.md](../../../physics/force-balance-ground-truth.md).

**One condition on any future use of this test, learned the hard way.** It is
only a friction measurement while the craft is *moving*. Below a few units per
second the normal impulse dominates the ratio and restitution can push
`|velocity|` **above** `speed`, giving a negative "loss"; the whole-lap capture
of the same day has ticks reading `-380 %` for that reason, on a craft wedged at
0.15 units/s. Restrict the test to contact ticks at speed, as the last row above
does.

Confidence **90** for the whole chain: the combination rule is nine plain
instructions, the two constants are single literals on branch-free paths, the
resolver is read at 88, and the measurement is one-sided rather than a best fit.

## The angular half, read at instruction level

`Body_ResolveContact`'s denominator and `Body_ApplyImpulseAtPoint`'s angular
term were both named-but-unread until this pass. Written out, with `r` the lever
arm `contact+0x00 - body+0x30`:

```text
v_p  = body+0x140 + cross(r, M(body+0x00..0x30) * body+0x150)   ; 0884ea98
vn   = dot(v_p, n)                                             ; 0884eb50
D    = body+0x378 + dot(n, cross(M(body+0x40..0x70) * cross(r, n), r))
j    = -(1 + body+0x388) * vn / D          ; and D == 0 returns, touching nothing
p    = j*n            unless contact+0x34 > 0 and v_p - n*vn != 0,
p    = j*n - contact+0x34 * (v_p - n*vn)   in which case
Body_ApplyImpulseAtPoint(body, contact, p)
Body_Translate(body, n * contact+0x30)
Body_RecordContact(body, contact, p, contact+0x20)
```

and the applier is

```text
body+0x140 += p * body+0x378                                   ; 0884d6d0
body+0x160 -= body+0x394 * (M(body+0xc0..0xf0) * cross(r, p))   ; 0884d7c8
body+0x150  = M(body+0x80..0xb0) * body+0x160                   ; 0884d824
```

Two things fall out that a textbook solver would not do.

### `body+0x394` is `0.1`, and only the *application* is scaled by it

The field has exactly one writer in the whole image - `0x08841520` in the
ship-entity constructor `0x08840c74`, storing `0x3dcccccd` - and exactly one
reader, `0x0884d768` in `Body_ApplyImpulseAtPoint`. Both arms of the
friction-branch above the store converge on it, so a racing craft always gets it.
Confidence **90**.

The denominator `D` contains the *full* angular compliance, so `j` is solved as
though all of the resulting spin were going to be applied, and then a tenth of it
is. The net effect is a contact that is **soft in translation** (`D` reaches
three times `invMass` on a corner lever, so `j` is a third of what a point-mass
solve would give) **and very stiff in rotation**. Applying either half without
the other is worse than applying neither: the denominator alone under-bounces
without stabilising, and the angular term alone spins the craft out on contact.

### The denominator applies a body-space tensor to a world-space vector

`0x0884ebc0`-`0x0884ebe8` loads the matrix at `body+0x40..0x70` and transforms
`cross(r, n)` with it. Both `r` and `n` are world-space, and `body+0x40` is
unambiguously the **body-space** inverse inertia:
[rigid-body.md](rigid-body.md#answered-body_setboxinertia-0x0884e1ac-writes-body0x40)
establishes that `Body_SetBoxInertia` writes it from the hull box's own
dimensions and that nothing recomputes it, while the *world* copy the integrator
derives every sub-step lives at `body+0x80` and is exactly what
`Body_ApplyImpulseAtPoint` reads three instructions from the end. So the rotation
into the body frame is skipped here and only here.

`I` is `(15.6, 21.6, 15.6)`, so on a pitched or rolled craft the two forms differ
by up to 38 % - a real difference rather than a rounding one. `crates/physics`
reproduces the literal reading and says so, because writing the textbook form
instead would make the crate disagree with the disassembly on no evidence at all.
Confidence **88** on the field identification, **90** on the instructions.

### Nothing on this path arms the collision stun

`Ship_ApplyCollisionImpulse` (`0x0883f274`) is the only `craft+0x290 += 0.5` in
the image and it is gated on a **pending impulse vector** at
`entity->0x4c + 0x110` being non-zero, which it zeroes on the way out. Nothing in
the contact path writes that vector: `Body_ResolveContact` ends with
`Body_RecordContact`, and the only consumer of that ring - `FUN_088418e0` - reads
the same per-contact impulse magnitude into three separate reactions (below) and
writes no impulse anywhere.

The captures agree, and they are the stronger leg:
`data/traces/talons-junction-time-trial-lap.csv` is a complete lap, 3,146 ticks
with `4.8 %` of them in wall contact, and `stun_timer` reads `0.0` on **every
tick**; `data/traces/talons-junction-standing-start.csv` adds 300 more, 230 of
them one continuous scrape, also `0.0` throughout. So the stun belongs to being
hit by something - a rival, a weapon - and not to touching the track.
`crates/physics` used to arm it from the wall constraint and no longer does; see
`crates/physics/src/wall.rs`'s `STUN_PER_CONTACT`. Confidence **88**.

### `FUN_088418e0`'s contact loop drives three separate reactions, and one of them **is** a particle

**Read in full this pass**, prompted by planning the collision-spark visual
effect (`oag_render::sparks`) and finding no prior page said what actually
consumes a hit. The earlier phrase above - "computes camera and audio
amplitudes" - turns out to conflate two of these three, one of them wrongly;
corrected here rather than silently. A later pass (below) found the actual
spark trigger sitting inside reaction #1, which this page had mislabelled
"camera shake" - so the "one candidate left for a visual spark trigger"
paragraph that used to close this section is retired; see
[`ShipCollisionFx_Trigger`](#shipcollisionfx_trigger-0x089246b4-is-the-actual-spark-spawn-function)
below.

For each record in the contact ring (`body + 0x170 + n*0x40`, `n < body+0x370`),
the loop computes `|p|` - the impulse magnitude `Body_RecordContact` stored -
and feeds it, scaled three separate ways, to three reactions. **The scaling
for reaction #1 is a clamp, and this took two passes to get right** - see the
correction below the list, which replaces an earlier wrong reading of what
`Ship_DispatchCollisionFx` actually receives.

1. **`Ship_DispatchCollisionFx` (`0x0883de90`)**, called as
   `FUN_0883de90(fVar21, param_2, piVar17)` where **`fVar21 = min(|p| * 0.0125,
   1.0)`** - clamped, not raw - only when a squared distance to a stored world
   position (`DAT_08ab10b0 + 0x70`, read as a camera-ish anchor) is in `(0,
   10000)` - i.e. gated by proximity. **Decompiled in full.** It finds the
   nearest of up to 10 `Ship Collision Fx` (`0x3d0`) instances attached to the
   craft (`param_2 + 0xc80` list, distance to the hit point at `param_3+0x10`),
   then unconditionally calls `ShipCollisionFx_Trigger(fVar21, thatInstance, 0,
   depth > 0.0)` - **this is the spark spawn**, not a camera effect. Only
   *after* that, if the reacting craft is the local player
   (`FUN_0883e64c(param_2) == 1`), it separately plays an audio cue via
   `FUN_08878750(fVar21 * DAT_08ab0dfc, ...)` - `fVar21` scaled *again* by a
   second, still-unread constant, one of two sound variants depending on
   whether the hit point is in front of or behind the craft. Nothing in this
   function's body calls a camera API; the "camera shake" reading this page
   previously carried was a guess from the `0.0125` scale alone and is
   **retracted**. Confidence **80** on the control flow (nearest-instance
   search, unconditional spark dispatch, gated sound); **40** on what
   `FUN_08878750` actually does with its arguments (sound cue is the likely
   reading given `PTR_s_...COLLISIONS...`-shaped sound calls elsewhere on this
   path, but the function itself is unread).
2. **Hull damage**, `FUN_088439ac(|p| * 0.05 * 0.7, param_2, hitSide, 0, 0)`.
   **Decompiled in full this pass.** It decrements the ship's shield/energy
   (`FUN_0883e6f4(shield - amount, ...)`), accumulates per-kind telemetry
   buckets keyed by the fourth argument (weapon/collision type, `0` here),
   triggers a death-state transition (`FUN_08844100(param_2, 4)`) when the
   result reaches zero, and plays a sound cue on death. This is **not** camera
   or audio amplitude as the earlier phrase read - it is the collision damage
   path, the gameplay-facing twin of the recovered `0.05`/`0.7` scrape-friction
   pair documented above. Gated on the per-contact scalar at
   `record + 0x1a0` (the friction field `Body_RecordContact` stores) being
   positive, and on a per-frame arm check (`FUN_0883e64c(param_2) == 1`).
   Confidence **80**: an unambiguous, branch-clear decompile, but a single
   static reading with no runtime trace corroborating it. Left as
   `FUN_088439ac` for now rather than renamed - see the open thread on
   `HANDOVER.md` for the rename-and-document follow-up this confidence level
   would otherwise call for.
3. **Shield hit flash**, a sibling branch taken *instead of* #2 when a linked
   shield entity exists and its flag `0x10` (at `entity + 0x1b8`) is set:
   `FUN_0885eb04(shieldEntity)`. **Decompiled in full**: six instructions that
   write a stored world-space hit position (`DAT_08b3bf50`, presumably set
   just before the call) and a literal `1.1` into fields on the shield entity
   - a hit-position-and-duration arm for the separately-scoped, still
   unimplemented "Shield hit response" (`docs/overview/roadmap.md`, M5), not a
   hull spark. Confidence **75**.

Also written on this pass: the same loop arms flag bits `0x20` and
`0x400020` on `craft + 0x860` right after reaction #2, whenever the damage
gate fires. A later pass in this same session found the actual spark trigger
(reaction #1, above) instead, so these flag bits are no longer read as the
spark candidate; their consumer is still unlocated and still unimportant to
the spark question. Left as an open, low-priority thread rather than chased
further.

## `ShipCollisionFx_Trigger` (`0x089246b4`) is the actual spark-spawn function

Reached from reaction #1 above. **Decompiled in full.** It takes
`(float intensity, ShipCollisionFx *instance, int kind, char damaged)` and:

1. **Names all three spark variants as literal strings**, spawned through the
   same `FUN_08915484` resource-by-name path `Ship_DispatchCollisionFx` also
   uses:
   - `kind == 2` -> `WO_WEAPON_ABSORB` (shield-absorb effect - see
     `FUN_08840640` below, not a collision at all).
   - `kind` is 0 or 1, `damaged == 0` -> `WO_SHIP_COLL_SPARK_NODAMAGE`,
     regardless of which of 0/1 `kind` is.
   - `kind == 1`, `damaged != 0` -> `WO_SHIP_SPARK_DAMAGE_LEACHBEAM`.
   - `kind == 0`, `damaged != 0` -> `WO_SHIP_COLL_SPARK_DAMAGE`. This is the
     one `Ship_DispatchCollisionFx` reaches from a wall/track contact
     (`kind` is hardcoded `0` at that call site).
2. **A 0.8-second cooldown, but only for `kind` 0 and 1.** `instance + 100`
   holds "don't fire again before this time"; compared against a global clock
   read at `*(DAT_08ab0818 + 0x40)` on entry, and set to `now + 0.8` after a
   kind-0/1 spawn succeeds. The gate is `(kind != 0 && kind != 1) ||
   (instance+100 <= now)`, so `kind == 2` (absorb) **bypasses the cooldown
   entirely** - it has its own re-trigger discipline via the caller's
   0.1-second stagger loop (`FUN_08840640`, below).
3. **A severity value, `intensity * 2.0 + 0.4`, only for the two "damaged"
   variants** (LEACHBEAM and COLL_SPARK_DAMAGE) - written as the last field of
   a small stack buffer passed into `FUN_088f44d8` right after the spawned
   instance is looked back up by hash. `WO_SHIP_COLL_SPARK_NODAMAGE` and
   `WO_WEAPON_ABSORB` never receive this value at all. `intensity` here is
   `ShipCollisionFx_Trigger`'s own first argument, i.e. **the same
   already-clamped `fVar21 = min(|p| * 0.0125, 1.0)` reaction #1 receives, not
   raw `|p|`** - see the correction below the reaction list, which is the
   important part of this entry.
4. **A `"COLLISIONS"` sound cue** (`FUN_089392b0(1.0, ..., "COLLISIONS", ...)`),
   fired once per surviving kind-0/1 call, separate from and in addition to
   the proximity cue `Ship_DispatchCollisionFx` itself plays.

Confidence **85**: unambiguous, branch-clear decompile with three literal
string names pinning the variant selection and clean, self-contained
arithmetic for both the cooldown and the severity formula. Not runtime-traced.

**Follow-up pass, same session, in two parts - the second corrects the
first.**

*First part.* `FUN_088f443c`/`FUN_088f44d8` (read in full) are a symmetric
getter/setter pair copying seven words between a stack buffer and offsets
`+0x28..+0x40` of `iVar2` - the object `FUN_088f24d0` looks up **by the
handle the trigger just wrote from the freshly spawned instance**, not the
shared `.pob` resource. So the severity write is confirmed **instance
state**, at field `+0x34` specifically (buffer index 3, disassembly-confirmed:
`swc1 f20,0xc(sp)` writes the same `f20` the `intensity * 2.0 + 0.4`
computation produced). That object is the generic `ParticleSystem` (`0x3c4`)
node the `.pob` resource resolves into, reached through `FUN_08916200` - a
much larger, general-purpose constructor this pass did not fully read, since
doing so is the general-particle-system decode the project's own
collision-sparks plan scoped out. One read past that, `FUN_088f4910`, shows
`+0x34` used as a plain multiplier against six derived fields
(`+0x58`/`+0x5c`/`+0x60`/`+0x64`/`+0x68`/`+0x6c`), computed from a *different*
pointer (`iVar1`) together with two sibling instance fields (`+0x28`,
`+0x2c`), with **no clamp anywhere on that path**.

*This is where the first pass went wrong.* It reproduced `resolve_contact`'s
own formula against this crate's actual impulse magnitudes - `1.39` for a `1`
unit/s graze, `69`-`165` for a realistic `50`-`119` unit/s impact - and read
`intensity * 2.0 + 0.4` as therefore unbounded and unusable without the
resource's own base values. **That used the wrong `intensity`.** It assumed
`ShipCollisionFx_Trigger`'s `intensity` argument was the raw impulse
magnitude `Ship_DispatchCollisionFx` receives unscaled - but the correction
above the reaction list already establishes that argument is
`fVar21 = min(|p| * 0.0125, 1.0)`, clamped to `[0, 1]` **before**
`Ship_DispatchCollisionFx` or `ShipCollisionFx_Trigger` ever see it. Once that
clamp is accounted for, `intensity * 2.0 + 0.4` ranges over a sane, bounded
`[0.4, 2.4]` - nowhere near unbounded.

*What caught it.* Not more static reading - a live PPSSPP capture
(`docs/reverse-engineering/ppsspp-debugger.md`'s methodology), breakpointed
on `ShipCollisionFx_Trigger`'s own stack write during a real standing-start
crash into a wall on Talon's Junction. Six real hits, craft speed `21.7`-
`112.4` units/s, read real `intensity` values of `0.011`-`0.070` - an order of
magnitude below `1.0`, let alone the `69`-`165` the wrong premise predicted.
That contradiction is what sent this pass back to re-read `FUN_088418e0` from
scratch rather than trust the earlier paraphrase, and found the `min(...,
1.0)` clamp sitting right there in the decompile. The six live values are
also internally consistent - `intensity / speed` holds to `0.00052`-`0.00062`
across a 5x speed range - which is exactly the shape a clamped, roughly
linear-in-impulse quantity should have well below its own ceiling.

**So the severity formula is now genuinely ported**, not merely re-scoped:
`oag_render::sparks::SEVERITY_SLOPE` (`2.0`) and `SEVERITY_FLOOR` (`0.4`)
apply the recovered shape on top of the already-recovered
`SEVERITY_SCALE` (`0.0125`) clamp, replacing what was previously an authored
severity curve with the literal one. `COLLISION_COOLDOWN` was already ported
in the first pass for the same reason this now is: both are usable without
the still-undecoded `.pob` resource data
([`docs/formats/pob.md`](../../../formats/pob.md)), once the actual inputs
are read correctly.

### `FUN_088f4910` derives six instance fields from a plain scalar block

*Second part of the same follow-up, after the severity correction above.*
`iVar1` - the pointer `FUN_088f4910` reads its six base fields from
(`+0x34`/`+0x38`/`+0x40`/`+0x48`/`+0x4c`/`+0x74`, per the reaction-list entry
above) - is not a third indirection layer. A live capture read the first
`0x20` bytes at `iVar1` and got `"WO_SHIP_COLL_SPARK_DAMAGE\0"`: `iVar1` **is
`resource_base`**, the exact address the `.pob` resource's own pointer-fixup
table is relative to (see
[pob.md](../../../formats/pob.md#the-slot-table-is-a-pointer-fixup-table)).
Extracting the resource file and reading the same six offsets from raw bytes
(`resource_base = HEADER_LEN + slots.len() * SLOT_LEN`, computed with
[`ParticleSystem::parse`](../../../../crates/formats/src/pob.rs)) reproduced
the live-captured values exactly: `0.0144`, `-0.00035750002`, `0.0`,
`0.048`, `0.0`, `-0.011232` at `+0x34/+0x38/+0x40/+0x48/+0x4c/+0x74`. These
are plain authored floats sitting in the file, never touched by the
pointer-fixup pass - see
[pob.md](../../../formats/pob.md#a-second-fixed-offset-field-block-sits-right-after-the-name---no-fixup-needed)
for the format-level writeup.

The full decompile of `FUN_088f4910`:

```c
void FUN_088f4910(int param_1)  // param_1 = spawned ParticleSystem instance
{
  float fVar2 = *(float *)(param_1 + 0x34);           // severity, set by the trigger
  int   iVar1 = *(int   *)(param_1 + 0x20);           // = resource_base, confirmed above
  float fVar3 = *(float *)(param_1 + 0x2c) * fVar2 * *(float *)(param_1 + 0x48);
  float fVar4 = *(float *)(param_1 + 0x28) * fVar2 * *(float *)(param_1 + 0x44);
  *(float *)(param_1 + 0x58) = *(float *)(iVar1 + 0x34) * fVar3;
  *(float *)(param_1 + 0x5c) = *(float *)(iVar1 + 0x38) * fVar3;
  *(float *)(param_1 + 0x60) = *(float *)(iVar1 + 0x40) * fVar3;
  *(float *)(param_1 + 0x64) = *(float *)(iVar1 + 0x48) * fVar4;
  *(float *)(param_1 + 0x68) = *(float *)(iVar1 + 0x4c) * fVar4;
  *(float *)(param_1 + 0x6c) = *(float *)(iVar1 + 0x74) * fVar2;
  *(float *)(param_1 + 0x70) = *(float *)(iVar1 + 0x4cc) * *(float *)(param_1 + 0x3c);
}
```

`param_1+0x28`/`+0x2c`/`+0x44`/`+0x48` are **instance-side** co-factors, a
different address space from the resource-side offsets of the same name
(`iVar1+0x28` was probed once out of caution and is unrelated - flagged here
so it doesn't get misread as a correlation later). In the captured hit they
all read `1.0`, the evident default, so `fVar3 == fVar4 == fVar2` (severity)
for this capture and cannot yet be told apart from it alone.

Reconstructing the six output fields from the file's own six values times
the captured severity (`0.4250674545764923`) reproduces every one of the
live-captured outputs to the same printed precision:

| output | formula | predicted | captured |
| --- | --- | --- | --- |
| `+0x58` | `0.0144 * fVar3` | `0.006122` | `0.006120971404016018` |
| `+0x5c` | `-0.00035750002 * fVar3` | `-0.000152` | `-0.00015196161984931678` |
| `+0x60` | `0.0 * fVar3` | `0.0` | `0.0` |
| `+0x64` | `0.048 * fVar4` | `0.020403` | `0.020403238013386726` |
| `+0x68` | `0.0 * fVar4` | `0.0` | `0.0` |
| `+0x6c` | `-0.011232 * fVar2` | `-0.004774` | `-0.004774357657879591` |

This is an end-to-end, byte-exact chain from file bytes through to the
values the spawn actually produces - the strongest evidence tier available
without a second binary. What it is **not** is a decoded meaning: the
`(0x58, 0x5c, 0x60)` group reads like a small 3-vector (direction? velocity
bias?) and `(0x64, 0x68)` like a 2-component pair, both plausible fits for
`sparks.rs`'s still-authored ejection/spread constants, but the magnitude
(~0.014 for the largest component) doesn't self-evidently correspond to any
unit this project's own physics or render code uses, and nothing downstream
of `FUN_088f4910` - i.e. whatever actually turns these six numbers into a
drawn particle - has been traced yet. Per the correction earlier in this
same section, porting a number without a confirmed unit is exactly the
mistake to avoid twice in one session; these six fields are recorded as a
located, reproducible finding, not as new `sparks.rs` inputs. Confidence
**90** for the offsets, the `iVar1 == resource_base` identity and the
input-to-output arithmetic (live capture and static file read agree
byte-exact, and the full derivation reproduces six independently-captured
outputs) - all on a single file and a single captured hit, not corpus-wide.
**0** for what any of the twelve numbers (six inputs, six outputs) mean.

The two non-spark callers below are recorded for completeness, not chased
further:

- **`FUN_088439ac`** (hull damage, documented above) also calls
  `ShipCollisionFx_Trigger(1.0, ..., shieldFlag, 1)` with a **hardcoded**
  intensity of `1.0`, but only on the branch where shield/energy has just
  reached zero - a death/destruction burst, not a per-hit spark, and not
  scaled by damage magnitude.
- **`FUN_08840640`** is the shield-absorb ability's own effect: it plays an
  `"ABSORB"` sound once, then calls `ShipCollisionFx_Trigger(1.0, ..., 2, 0)`
  in a loop over up to 10 attached instances, staggering a global float
  (`DAT_08abf564`) by `0.1` per iteration - unrelated to wall/track contact.

**Also checked and ruled out**: the `Ship Collision Fx` `0x3d0` class's own
registration wrapper, `FUN_08924bf0` (`Vex_RegisterClass(&DAT_08b63f18,
0x3d0)`), installs `FUN_08a6ba70` into the class descriptor's constructor
slot. That function is not a constructor at all - it is a three-instruction
thunk (`lui`/`jr`/`addiu` in the delay slot) that returns its own entry
address, the same self-referential shape several other classes' descriptors
install, which reads as a type-identity token rather than a per-instance
constructor. So the authored scene-graph path for this class does not lead
anywhere either, at least not through the route this pass checked; per
HANDOVER's standing rule, an unregistered or trivially-registered class
constructor does not prove the visual is absent, only that this particular
lead is a dead end. Confidence **70** that `FUN_08a6ba70` is a type token
rather than a constructor, on the instruction pattern alone.

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
- **What posts the pending impulse at `entity->0x4c + 0x110`.** The consumer is
  read and the contact path is excluded (above), so the writer is somewhere in
  the weapon or rival-contact code and is the next step for anyone wiring the
  stun back up.
- **`0x08815ccc`** (box against box) and **`0x0881702c`** (mesh against mesh,
  gated on `world+0x5464`), the two narrowphase pairs `Collision_DispatchPair`
  can reach that a craft against track geometry does not.
