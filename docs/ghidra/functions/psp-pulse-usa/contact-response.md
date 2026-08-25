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
`data/traces/talons-junction-time-trial-lap.csv` is 3,146 ticks (not a complete
lap despite the name - it stalls and reverses somewhere in it, see
`docs/tools/oag-trace.md`) with `4.8 %` of them in wall contact, and
`stun_timer` reads `0.0` on **every tick**; `data/traces/talons-junction-standing-start.csv`
adds 300 more, 230 of them one continuous scrape, also `0.0` throughout. So the
stun belongs to being hit by something - a rival, a weapon - and not to
touching the track.
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
without a second binary. Confidence **90** for the offsets, the
`iVar1 == resource_base` identity and the input-to-output arithmetic (live
capture and static file read agree byte-exact, and the full derivation
reproduces six independently-captured outputs).

**The meanings, left at confidence 0 above, were recovered the next day
(2026-08-01)** by tracing every consumer of the derived fields -
[particle-system.md](particle-system.md) is the full record, and the
function is renamed `ParticleSystem_DeriveScaledParams`. In one line each:
`+0x58/+0x5c/+0x60` are the severity-scaled emitter extents,
`+0x64/+0x68` the severity-scaled ejection-speed centre and spread in
world units per tick, `+0x6c` a flag-gated gravity, `+0x70` a
playback-rate multiplier on the system's `dt`; severity also multiplies
each particle's drawn size per tick. The waiting-for-a-unit discipline
this paragraph recorded paid off exactly as intended - the numbers went
into `oag_render::psys` only after their consumers were read
and the units confirmed, and the file turned out to hold a four-emitter
tree ([pob.md](../../../formats/pob.md#the-collision-spark-file-is-a-four-emitter-tree))
of which these six fields describe only the root.

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

**Both cues are now played**, and they are the same `Sound_Play` calls this page
found rather than a reconstruction: `"COLLISIONS"` on a contact past the
0.8-second gate, `"ABSORB"` when the shield is up and the sparks are therefore
suppressed. Two caveats are recorded rather than papered over. `"COLLISIONS"`
is stored in `ship.bnk` as **`".COLLISIONS"`** and no bank on either disc holds
the undotted name, so the port looks up the dotted one - SCREAM's own
sound/child-sound distinction, at confidence 70. And `"ABSORB"` is re-armed on
the same 0.8 seconds, where `FUN_08840640` **bypasses** that gate and has its
own 0.1-second stagger; how often the game calls it is not recovered, so a
shared cooldown is a stated approximation and the alternative - no gate at all -
would fire it sixty times a second through a scrape. See
[psp-audio.md](../../../formats/psp-audio.md#a-cue-owns-a-run-of-the-command-table)
and `oag_game::audio::sfx`.

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
[ps2-pulse-eu/craft-update.md](../ps2-pulse-eu/craft-update.md) records it: six passes
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
- ~~**`Ship_ApplyCollisionImpulse`'s pickup-flag `0x10`.**~~ **Read
  2026-08-19: it is the Shield pickup**, armed by `Shield_Fire`
  (`0x08861568`) off fire bit `0x20` and cleared when its countdown expires.
  See [shield-pickup.md](shield-pickup.md), which also reads the four `& 0x10`
  gates in this function's own contact loop and the weapon-damage drain
  (`0x0883f13c`) that absorbs a hit into the shield's flash instead. The
  confidence-75 wall attribution in
  [force-balance-ground-truth.md](../../../physics/force-balance-ground-truth.md)
  stays capped where it is for its own reasons, but not for this one: the
  Shield cannot be up during a trace that never picks one up.
- ~~**`0x0884ef30`**, the two-body contact resolver.~~ **Read 2026-08-11** - see
  [the section below](#body_resolvecontactpair-0x0884ef30-the-two-body-path).
- ~~**What posts the pending impulse at `entity->0x4c + 0x110`.**~~ **Read
  2026-08-19** - a write breakpoint at the live address caught two writers.
  See [the section below](#two-writers-found-at-a-live-write-breakpoint-2026-08-19).
- **`0x08815ccc`** (box against box) and **`0x0881702c`** (mesh against mesh,
  gated on `world+0x5464`), the two narrowphase pairs `Collision_DispatchPair`
  can reach that a craft against track geometry does not.

## `Body_ResolveContactPair` (`0x0884ef30`): the two-body path

**Read 2026-08-11.** This page listed it under "Not determined" until then, as the
one thing standing between the engine and craft-to-craft collision. Confidence
**80**: the arithmetic is unambiguous and every VFPU step is accounted for, but it
is decompilation of one function with no runtime leg and no second binary, which
the [rubric](../../../reverse-engineering/confidence-rubric.md) caps at 84.

Signature, as the narrowphase calls it (see [collision.md](collision.md)):
`(contact, bodyB, bodyA)`. The contact record is read at three rows - the contact
**point**, the contact **normal**, and a row carrying a scalar plus a handle.

### The gate: a separating test the one-body path does not have

```text
if ((vA . n) - (vB . n)) - 0.5 <= 0.0   -> resolve, else return
```

`vA` and `vB` are each body's velocity **at the contact point** - the full rigid
term, `linear + omega x r`. This is the clearest divergence from
`Body_ResolveContact`: the one-body path applies its impulse whatever the sign of
`vn`, and this one refuses on a pair that is already separating faster than
`0.5`. The threshold is a code literal, not a body or collider field.

### The impulse

```text
j = -(1.1 * vn) / (angular + invMassA + invMassB)
```

with `vn` the relative normal velocity recomputed from the same two point
velocities, and `angular` the usual `n . ((I_A^-1 (r_A x n)) x r_A + (I_B^-1 (r_B x n)) x r_B)`,
built here from two `vtfm4_q` inverse-inertia transforms and two cross products.

Two things to take from it:

- **`invMassA + invMassB`, which is what makes it a pair resolver.** This page
  predicted exactly that from the PS2 twin, and it holds.
- **Restitution is a hardcoded `-1.1`, so `e = 0.1`** - and that is *not* the
  PSP's one-body value. `Body_ResolveContact` reads the per-body `body+0x388`,
  which the ship constructor sets to `0.4`. So a craft bounces off the **track**
  with `e = 0.4` and off **another craft** with `e = 0.1`, in the same build. It
  is also the PS2 pair resolver's own numerator, so the two builds agree with
  each other about pairs while the PSP disagrees with itself across the two
  paths. Worth stating plainly because the `0.4`-versus-`0.1` split was already
  flagged as the thing a reader trips on, and this is a third instance of it.

`invMass` is at `body+0x378`, read on both bodies.

### What it does with the impulse

Equal and opposite, at the contact point, through the named
`Body_ApplyImpulseAtPoint` (`0x0884d64c`): `+j*n` to A and `-j*n` to B.

Then a **symmetric positional correction**: each body is moved along the normal
by `±0.25 * s`, where `s` is the contact row-3 scalar (penetration depth by
position and use, unconfirmed). A quarter each way rather than a half, and the
same magnitude for both bodies regardless of mass.

**No friction.** The one-body path folds `- f*vt` into the same impulse; this one
has no tangential term at all.

### The two calls after the impulses, resolved - and both ruled out as the pending-impulse writer

**Read 2026-08-19, correcting the row above.** `Ship_UpdateCraft` was never
checked against these two addresses; its real span is
`0x08849618-0x08849ed0`, and `0x0884d9f8`/`0x0884dc60` both sit thousands of
bytes before it - the "lands inside `Ship_UpdateCraft`" claim was a research
error, not a real boundary trap. The rebasing rule holds fine, same as it
does for `Body_ApplyImpulseAtPoint`'s call two lines above in the same tail.

- **`0x0884d9f8` is `Body_Translate`**, already named and documented above.
  Nothing new here - it is the pair path calling the same routine the
  positional-correction section describes, once per body, with `∓0.25 * s`
  each way.
- **`0x0884dc60` had no function boundary at all** - a genuine, if narrow,
  auto-analysis gap, bracketed by `Body_RecordContact`'s own `jr ra` just
  before it and a `jr ra` of its own at `0x0884dcc4`. Boundary created
  2026-08-19 (`FUN_0884dc60`, confidence 95 for the boundary itself - the
  bracketing returns and both call-site xrefs agree). Its body is a
  near-twin of `Body_RecordContact` just above: the same ring append at
  `body+0x370`, the same `+0x180`/`+0x190`/`+0x1a0` writes, but it stores
  `1` at `+0x170` where `Body_RecordContact` stores `0`, and it never
  writes `+0x1a4` (it takes one fewer argument - no `contact+0x20`
  equivalent). Read directly against the sibling function's bytes,
  confidence 85. **Left unnamed rather than guessed at**: the call's `a1`
  argument is not a "point" in either invocation - the first call passes
  the *other body's own pointer* as `a1`, so the callee's `lv.q 0x0(a1)`
  reads that body's `+0x00..0x0c` (an orientation-matrix row, not a
  position), and the second call passes this function's own third
  parameter (set up at entry exactly like the single-body path's
  `contact`) as the record's *owner* (`a0`) rather than as a point source.
  Whether that third parameter is a real manifold contact struct being
  reused as a ring owner, or the pair path's actual second-craft data
  wearing the name this page has been calling "contact," is unread.
  Confidence on that question: 50, which is why the function stays
  `FUN_0884dc60` rather than `Body_RecordContactPair` or similar.

**Neither call posts the pending impulse.** `Body_Translate` only ever
touches `body+0x30/0x34/0x38`. `FUN_0884dc60` only ever touches
`body+0x170/0x180/0x190/0x1a0/0x370` on whatever it is handed as `a0`. There
*are* two `swc1 f12,0x15c(...)` writes inside `Body_ResolveContactPair`
itself (`0x0884eff8` on `s0`, `0x0884f190` on `s1`) that are tempting given
`0x4c + 0x110 = 0x15c` - checked directly and ruled out: `f12` is the
function's `0.0` constant at both sites, and each write clears the
w-component of that body's own `+0x150..0x15c` quad immediately before the
whole quad is reloaded (`lv.q`) for a `vtfm4.q` matrix transform two
instructions later. That is a clear-to-zero on the body pointer directly,
not a post to `*(entity+0x4c)+0x110` - a different object and a different
operation. Confidence 90 this is unrelated to the stun mechanism.

So the pending-impulse writer stays unfound by static reading alone, but the
exclusion now covers both of `Body_ResolveContactPair`'s tail calls, not just
an unresolved "lands somewhere odd" note. It is somewhere else in the weapon
or rival-contact code, which the next section confirms directly.

### Two writers, found at a live write breakpoint, 2026-08-19

The static sweep above exhausted the cheap candidates without finding a
literal `+0x110` store on an entity-derived pointer anywhere in
`Body_ResolveContact`'s or `Body_ResolveContactPair`'s call trees. The
question "does anything write here" is what a live watchpoint answers better
than more decompilation, so this was checked directly: a Single Race
(opponents on the grid, `psp-drive.py menu --single-race`), a write breakpoint
(`memory.breakpoint.add`, `size=16, write=true, log=true`) armed on all eight
craft's `*(entity+0x4c)+0x110` at once - the addresses read live off a
`Ship_ApplyCollisionImpulse` breakpoint hit rather than guessed - and the
player driven into the pack. `docs/reverse-engineering/ppsspp-debugger.md`'s
watchpoint section is the mechanism; a control watchpoint on a craft's own
rigid-body position (thousands of hits) ran alongside the whole time to prove
the instrument was live throughout, per that page's rule that a quiet
watchpoint and a working one that saw nothing look identical.

Three of the eight armed craft took a hit during one drive. Every hit logged
one of two program counters, immediately followed by a **write** from
`Ship_ApplyCollisionImpulse` itself (`0x0883f3d0`/`0x0883f398`/`0x0883f410`) -
the zero-on-consume this page already reads - landing on the same watched
address a beat later. That is the consumer this page already reads,
corroborating the target address rather than a coincidence.

#### `Weapon_PostBlastImpulse_q` (`0x0886794c`), confidence 68

The more frequent of the two (5 of 6 logged hits). A short, straight-line
function - no loop, disassembled and read instruction-by-instruction because
the decompiler would not complete on it (nor on `Body_ResolveContactPair` or
`FUN_08868ea4` below, checked directly against a function this page decompiled
cleanly on 2026-08-11, so this is the decompiler failing on this database
session rather than these three functions being unusual).

Signature as called: `(craftArray, sourceIndex, targetIndex)`, all three plain
values, not pointers to a manifold. `craftArray + sourceIndex*4 + 0x64` gives
a source entity, call it `S`. `craftArray + targetIndex*4 + 0x44` gives, after
one indirection, what this function reads as the target, call it `T` - and it
is `T+0x110` that gets written - **what is measured, and no more**: the write
instruction's operand computes `T + 0x110`, and the live watchpoint (armed at
`*(entity+0x4c)+0x110`, `Ship_ApplyCollisionImpulse`'s own addressing) fired at
exactly that address, so `T+0x110` really is the pending-impulse slot for
whatever entity `T` names, corroborated at runtime rather than only in the
static trace.

**Full instruction-level re-read, 2026-08-19** (`disassemble_bytes` over
`0x0886794c`-`0x08867b4f`, the whole function body), corrects and extends the
first pass:

1. `S->0x40` indexes a global table (`0x0885bff0`, itself image-base-relative
   in the raw bytes - `0x00057ff0` plus `0x08804000`) of pointers, one per
   weapon type; call the pointed-to block `stats`. Shaped exactly like a
   per-weapon stats block (compare `WeaponAIstats` in
   `docs/reverse-engineering/ppsspp-debugger.md`'s watchpoint section, a
   different table with the same "index through a global, read named floats"
   shape), read at `+0xe8`, `+0xec`, `+0xf0`, `+0xfc` and branched on
   at `+0x368`.
2. **Correction: the tri-state branch reads `stats->0x368`, not `source->0x368`.**
   The first pass misattributed this field to the source entity; the
   instruction sequence is `S->0x40` → table lookup → `stats` pointer → `lw
   a0, 0x368(a0)`, three loads deep from `S`, landing on `stats`, not on `S`
   itself. **This means the field
   [HANDOVER.md](../../../../HANDOVER.md) flags as "the shield path's last
   unmeasured field: `entity + 0x368`" is a different field from this one** -
   that entry stands unchanged, and this page's own prior claim that the two
   were the same field is withdrawn. What the tri-state actually gates,
   corrected: **`T->0x124 = 1` when `stats->0x368` is `0` or `2`, skipped only
   when it is `1`** (traced through the full three-way branch, not "when it is
   not `0`" as the first pass had it - a case dispatch that sets a boolean
   from `tri == 0 || tri == 2` is easy to misread as `tri != 0` from a partial
   trace). Still unread: what `stats->0x368`'s three states mean, and what
   `T->0x124` is consumed by. A per-weapon-type flag (rocket vs. mine vs.
   missile behaving differently) is a better fit now than a per-source-craft
   one, but that is still a guess.
3. `S->0x90` is the source position - found by resolving the call at
   `0886798c`'s `jal`: raw operand `0x00055cd4`, real address
   `0x08859cd4` after the same image-base correction, a **three-instruction
   leaf** (`a0 += 0x90; *a1 = *a0; jr ra`) that is exactly `Vec4
   GetPosition_q(entity, out)` and nothing more, unnamed. The first pass did
   not resolve this call at all.
4. `d = |T->0x50 - S->0x90|` (`vsub.q` then `vdot.t`/`vsqrt.s`) - the first
   pass's "target_position" is `T->0x50` specifically, and it is the **same**
   subtraction reused for the direction below, not two separate reads as
   steps 1 and 4 of the old list implied.
5. `T->0x120 += stats->0xe8` - an accumulator on `T`, read nowhere in this
   function; a shake or cooldown timer is the obvious guess, unread.
6. `falloff = 1.0 - d / stats->0xec` - a linear falloff over a radius, not
   clamped here (a hit outside the radius drives `falloff` negative; nothing
   in this function stops that, so either the caller gates on distance first
   or a far hit posts a negative-magnitude impulse - unread).
7. `direction = normalize(T->0x50 - S->0x90)` - the same difference vector
   step 4 already computed, guarded against a zero-length vector the way
   `FUN_08868ea4` below does too (`vrcp` on a `MaxFloat`-substituted zero
   denominator rather than a divide-by-zero).
8. `impulse = direction * (falloff * stats->0xf0)`.
9. `*(T+0x110) = *(T+0x110) + impulse` - **accumulated**, via `vadd.q`, not
   overwritten. Two blasts in the same tick stack. The write itself is `sv.q
   C400, 0x0(a2)` at **`0x08867b0c`**, the address every watchpoint hit
   logged.
10. **Three more writes the first pass missed entirely**, all after the
    pending-impulse accumulate and none touching it: `T->0x130 += stats->0xfc`
    (a second, distinct scalar accumulator on `T`, unread consumer);
    `T->0x138 = 4` (a literal, unread - a state or mode tag is the obvious
    guess); `T->0x13c = S->0x40` (records the *source's* weapon-stat index
    onto the target - "what hit me last" bookkeeping is the obvious guess,
    unread). None of these three feed `Ship_ApplyCollisionImpulse` or
    anything else this page has read, so they stay measured-and-unported.

**What `T` actually is - settled, 2026-08-19, at a live breakpoint.** The
static read above left this as an open contradiction: `T+0x50` is used exactly
the way a world position is used, but `rigid-body.md`'s own offset table puts
`body+0x40..0x70` as the inverse inertia tensor, body space - so if `T` were
the body pointer, `T+0x50` should be tensor storage, not a position, and the
two readings could not both hold.

A live check settles it in favour of `T` being the body pointer. Method: a
`Ship_ApplyCollisionImpulse` breakpoint enumerated all eight live craft
entities and each one's body pointer (`*(entity+0x4c)`), same as the original
two-writers hunt; a second, execution breakpoint at `0x08867aa8` (right after
`Weapon_PostBlastImpulse_q`'s `lw a2, 0x44(s0)` loads `T`, before the `+0x110`
`addiu`) caught a real AI-on-AI blast during a driven Single Race. At the hit:

```
T (a2)  = 0x09ba1a80  -- exactly the body pointer already enumerated for one
                         of the eight craft (entity 0x09b41790)
T+0x50  = (-668.16, 7.92, 139.64, 1.0)  -- track-scale, matches the craft
                                            positions read moments earlier
                                            (e.g. -132.30, -49.58, -175.27)
T+0x110 = (0.0, 0.0, 0.0, 1.0)          -- resting, consistent with a tick
                                            with no pending impulse yet
```

`T` is the body pointer, confirmed by direct pointer equality against the
enumerated list, not inference. `T+0x50` holds a value that is unambiguously
position-shaped and position-scaled for the craft it belongs to. So
`Weapon_PostBlastImpulse_q` really does read `body+0x50` as this craft's world
position - which sits in genuine, unresolved tension with `rigid-body.md`'s
own confidence-**88** reading of `body+0x40..0x70` as a single 4x4 inverse
inertia tensor, body space. That reading is not weak: it is backed by
`Body_Integrate` actively *consuming* the block every sub-step
(`vtfm3.t`/`vscl.t` off `body+0x40`, the result cached to `body+0x80..0xb0`
and read again by `Body_ApplyImpulseAtPoint`), not merely by
`Body_SetBoxInertia` zeroing it at construction.

**Checked further, same session, and the contradiction stands rather than
resolves.** `Body_SetBoxInertia`'s diagonal patch-back (`disassemble_bytes`
over `0x0884e218`-`0x0884e22c`) writes three scalars at `+0x40`, `+0x54`,
`+0x68` - stride `0x14` apart, exactly what a row-major 4x4 with `0x10`-byte
rows at `+0x40`/`+0x50`/`+0x60`/`+0x70` predicts for its own diagonal
(`row_base + column*4`: `0x40+0`, `0x50+4`, `0x60+8`). The constructor's own
arithmetic requires row 1 to start at `+0x50` for `+0x54` to be its diagonal
entry - so this does not weaken `rigid-body.md`'s tensor reading, it
independently confirms the construction-time layout. Which makes the runtime
observation sharper, not softer: for an axis-aligned box, the off-diagonal
terms of row 1 (`+0x50`, `+0x58`, the ones this session read as `-668.16` and
`139.64`) should be `0.0` forever after construction, and they visibly are
not. Something writes real, non-zero, per-craft data into that row at
runtime, and `Weapon_PostBlastImpulse_q` reads it as this craft's position.
Left as a genuinely unreconciled contradiction, not a range-label
imprecision: the next check is whether `Body_Integrate`'s `vtfm3.t` reads all
four quad-words as one matrix or only three (a `.t` transform is 3x3; which
three rows it actually addresses from a `+0x40` base is not yet read), and
what writes `body+0x50` every tick if it is not the tensor's own math - the
integrator's own position update is the obvious candidate, given what this
function reads there.

**Separately, and read with a caveat this time.** The same live check also
read `body+0x30` (`rigid-body.md`'s own `Body_AddForceAtPoint` comment, `a0 =
body+0x30 ; position`) for all eight craft - but **at the start line, before
the countdown**, not at the mid-race moment `T+0x50` was sampled. Every
craft's value clustered near `(0, 0.7-0.75, 0)`, unit-scale. That is not a
controlled comparison against the mid-race `T+0x50` reading - a parked grid
sitting close together in some local frame is also a plausible explanation for
tight clustering, and `rigid-body.md` already documents `Body_Translate` doing
`body->position(+0x30) += ...`, i.e. a field that does accumulate like a
position. So this is not evidence that `body+0x30` is not position, only a
recorded observation from one uncontrolled sample: worth a same-breakpoint,
mid-race re-read before anyone treats it as a finding.

**Why 68, and the `_q`, unchanged despite settling `T` and reading the
caller.** The write site, its accumulate-not-overwrite semantics and the
falloff formula are now traced across the whole function body, the
source-position helper is resolved, `T`'s identity is now measured rather
than disputed, and the caller (`FUN_08867b50`, below) is now fully read
rather than found-but-unread - genuine gains, and `targetIndex`'s origin (a
range-checked sweep over every craft) is settled, not guessed. What still
holds it under 70: the `stats` table's identity and its
`sourceIndex`/`targetIndex` calling convention are still read off this
function alone with no second independent site to cross-check; the
`stats->0x368` branch's three cases are not understood, only observed; and
the trigger two hops further up - what arms a craft's fuse timer and what
calls `FUN_08867370` each tick - is still open, so "a rocket, a mine or a
missile" is narrowed (see `FUN_08867370`'s own section for why a mine now
fits best) but not confirmed.

#### A second writer at `FUN_08868ea4` (`0x08868ea4`), not renamed

The other logged PC, once. Also decompiler-resistant, and only partially read
by hand: it loops (`t7` against a bound read through a `lui`/`lw` pair shaped like a
`$gp`-relative access - the same pattern [HANDOVER.md](../../../../HANDOVER.md)
already flags as unreliable to address literally from its displayed
immediate, so the loop's actual bound is unread), and inside the loop
computes `*(cursor+0x48)+0x110` as its target address, where `cursor` starts
at the function's own `a0` and advances by one word (`4` bytes) per loop
iteration - the same `+0x48` offset `Weapon_PostBlastImpulse_q` reaches
through `+0x44`, one word apart, which is consistent with either of two
readings this project has not distinguished: a cursor walking successive
*entities*' pending-impulse pointers (each entity's own `+0x48` slot, one per
iteration), or a cursor walking consecutive *fields* inside one entity
(`+0x48`, `+0x4c`, `+0x50`, ...) that happen to alias the pending-impulse
offset on one particular iteration. Nothing read here settles which, nor what
the loop is walking a list *of*. That shape - iterate something, post an
impulse to each entry - still reads like the rival-contact half of "weapon or
rival-contact code" the way `Weapon_PostBlastImpulse_q` reads like the weapon
half, but it is a guess about the shape, not a measurement. Confidence on that
reading: under 50, so no name - `FUN_08868ea4` stays as it is rather than
guessing `ShipContact_PostRivalImpulse` or similar. **Two callers found, both
unread**: `FUN_08868a10` (`0x08868ad4`) and `FUN_088690fc` (`0x08869450`) -
same image-base-relative search as `Weapon_PostBlastImpulse_q`'s caller
above. Left for whoever picks the loop bound, the list identity, and these two
callers apart.

#### `FUN_08868a10` and `FUN_088690fc`, `FUN_08868ea4`'s two callers, read 2026-08-25

Both decompile cleanly (unlike everything above on this page); read from that
output rather than disassembly. Both call sites resolve the same way the rest
of this page's `jal` targets do: `func_0x00064ea4`'s relative operand plus
this program's `0x08804000` image base is `0x08868ea4`, i.e.
`FUN_08868ea4` itself - the same check `search_instructions` on the
relative operand already used to find these two functions as callers in the
first place.

**`FUN_08868a10(craftArray, impulse)`**, `0x08868a10`-`0x08868b0b`: a fixed
`0x30` (48)-iteration loop over a pointer array based at `craftArray+0x68`
(`entity = *(craftArray + 0x68 + i*4)`, `i` from `0` to `47`) - a **third**
parallel entity-pointer array on `craftArray`, distinct from the `+0x64` one
`FUN_08867b50` sweeps and the `+0x48` one `FUN_088690fc` sweeps below; this
page has still not settled whether the three are independent lists or views
into the same one at different strides. For each `entity`, a gate
(`func_0x0005abdc(impulse, entity)`, unread, real address `0x0885abdc`)
decides whether this entry is hit at all. When it is: `impulse+0x8`, compared
against the sentinel `-1`, picks between two branches that both end up
building the same `local_30`/`local_2c`/`local_28` 12-byte vector from
`impulse+0xc`/`+0x10`/`+0x14` (so `impulse` itself is a small struct - a
target index or `-1` "no specific target" sentinel at `+0x8`, an xyz vector
at `+0xc..+0x18`) and both set a flag on `entity+0x3c` (`|= 0x4` on the `-1`
path, `|= 0x24` otherwise) - the same bit `Weapon_PostBlastImpulse_q` and
`FUN_08867b50` set on their own "this connected" flag, corroborating that
`entity+0x3c` is a shared flags word across all of this page's contact
paths, not a per-function coincidence. The non-`-1` branch additionally calls
`func_0x00065054(craftArray, *(impulse+0x8), i)` (unread, real address
`0x08869054`) before setting its flag - `FUN_088690fc` below calls the same
function at the equivalent point, so whatever it does is common to both
callers, not specific to either. Both branches then call
`func_0x00064d50(craftArray, &local_30)` (unread, real address
`0x08868d50`) and finally `FUN_08868ea4(craftArray, &local_30,
*(entity+0x4c))` - `entity+0x4c` is the same "gives a pointer" field
`Ship_ApplyCollisionImpulse` reads as the body pointer, so this caller hands
`FUN_08868ea4` a body, not an entity, as its third argument.

**`FUN_088690fc(craftArray, sourceIndex)`**, `0x088690fc`-`0x0886951b`: reads
`source = *(craftArray + sourceIndex*4 + 0x68)` (the same `+0x68` array
`FUN_08868a10` walks, this time indexed directly rather than swept), gates on
a validity check (`func_0x0005ed4c`, the same unread validity function
`FUN_08867b50` calls at its own step 1) applied to `*(craftArray +
*(source+0x40)*4 + 0x48)` - the **`+0x48`** array, a fourth entity-pointer
list. Reads two vectors off `source` (`func_0x0005601c`, unread, real address
`0x0885a01c`) and builds a normalised direction between them (the
zero-guarded normalise idiom: substitute `1.0` before reciprocal when the
length compares equal to zero, rather than dividing by it - the same shape
`FUN_08868ea4`'s own body was read as using, per the note above). Then
sweeps `0..*(0x00577f8)` - a *dynamic* global count, not `FUN_08868a10`'s
fixed `0x30` - over the `+0x48` array, **skipping `i == *(source+0x40)`**:
`source+0x40` read here as "this entity's own index, so the sweep does not
test itself against its own line" corroborates `FUN_08867b50`'s identical
`entity+0x40` field, read there as a per-entity counter/index compared
against its own loop variable the same way. For each candidate that is not
`source` itself: two dot-product bounds checks against the direction vector
(`<= 1.0` and `>= -1.0` - a unit-length slab test along the swept segment
between `source`'s two read vectors), then a cross product against the same
direction, normalised again, and a third dot product bounded to `(-6.0,
6.0)` - read together, this is a swept-capsule proximity test: "is this
candidate within one unit along the segment `source` moved through this
tick, and within six units of the segment itself" is the shape, not
confirmed against a live trace. A passing candidate gets `*(source+0x3c) |=
0x24` (the same combined flag `FUN_08868a10`'s non`-1` branch sets, this
time on `source` rather than the candidate), then the identical pair
`func_0x00065054(craftArray, i, sourceIndex)` and `FUN_08868ea4(craftArray,
&<direction-endpoint vector>, *(source+0x40))` that `FUN_08868a10` calls -
except this caller's third argument to `FUN_08868ea4` is `*(source+0x40)`,
an index, not `entity+0x4c`, a pointer; `FUN_08868ea4`'s own parameter is
therefore not consistently typed across its two callers on the evidence
read so far, which is itself worth flagging rather than silently reconciling.

Confidence on both functions' shape: **60** (the flag bits, the shared
`FUN_08868ea4`/`func_0x00065054` call pair, and the `+0x40` self-index
corroboration are read off two independent decompiles and agree with each
other and with `FUN_08867b50`'s established fields); **under 50** on what
either function is *for* semantically (a name like
`ShipContact_ApplyImpulseSweep` is a guess about the swept-capsule reading,
not a measurement), so neither is renamed. `func_0x0005abdc`,
`func_0x00065054`, `func_0x00064d50` and `func_0x0005601c` are all still
unread past what this pass needed to trace the vector/flag flow through
them.

### `Ship_ApplyCollisionImpulse` (`0x0883f274`), read at instruction level, 2026-08-19

`engine.md` already carried this function's behaviour as a confidence-85
summary (the forward-axis projection, the Shield gate, `craft+0x290 += 0.5`).
Read in full off its disassembly - the decompiler would not complete on it
either, same as every function on this page checked today - it comes apart
into six steps, `s0` holding the function's own `a0` (called `entity` below)
throughout:

1. **The zero test.** `*(entity+0x4c)` gives a pointer; `+0x110` off it is the
   pending vector. Read component-wise - `x != 0` short-circuits the other two
   checks, `y` next, `z` last - and if all three are `0.0`, jump straight to
   the function's cleanup with nothing touched. This is why a live watchpoint
   only firing on three of eight craft this session is not "five armed
   watchpoints were dead": most craft simply had nothing pending that tick.
2. **The Shield gate.** `*(*(entity+0x4c)+0x1b8) & 0x10` - if set, skip both
   the projection and the timer arm below and go straight to step 6. This is
   the field `engine.md` already read as `craft->0x1b8`; whether it is really
   the same struct `weapon-fire.md` calls `entity+0x1b8` (the fire-request
   word, read one indirection shallower) or a same-shaped field on a distinct
   struct is not settled here - see "Not determined" below.
3. **The projection, and it really is what the summary said.** `magnitude =
   |pending.xyz|`; `forward = *(*(entity+0x794))` (`+0x794` is already read
   elsewhere as the ship's scene node - [`camera.md`](camera.md),
   [`exhaust.md`](exhaust.md), [`missile.md`](missile.md) all cite it, and its
   first quad reads as an orientation row here, consistent with a scene node's
   transform); `sign = dot(forward, pending.xyz) < 0 ? -1 : +1`; the pending
   vector's storage is then **overwritten in place** with `forward * (sign *
   magnitude)`. The original direction is discarded entirely - only its
   magnitude and which way it pointed relative to the ship's own forward axis
   survive. "Scales the ship's own forward axis by the impulse magnitude
   rather than using the impulse direction" is exact, not a summary.
4. **The impulse is applied through `Body_ApplyImpulseAtPoint` (`0x0884d64c`)**
   - the same function [the pair path below](#what-it-does-with-the-impulse)
   uses, called `(body = *(entity+0x794), point = a1, impulse = &pending)`.
   `a1` is the whole function's own second parameter, passed through
   unexamined the entire way - what the caller supplies as a point is not
   read here.
5. **The timer arm.** `*(entity+0x94)` is already established as the
   entity-to-craft back-pointer ([`shield.md`](shield.md) reads a live example
   at the identical `0xfc0` offset this page's own watchpoint session
   measured between "entity" and "craft" independently). `craft->0x290 +=
   0.5` there - `STUN_PER_CONTACT` in `crates/physics/src/wall.rs`.
6. **The consume.** Whether or not the Shield gate fired, `*(entity+0x4c)+0x110`
   is overwritten with a constant quad read out of `.rodata` (read live as all
   zero, matching every idle craft this session's watchpoint saw as `(0, 0, 0,
   1)` before a hit and presumably `(0, 0, 0, 0)` or similar after - not
   independently confirmed which).

Confidence **85** on the mechanism (matches `engine.md`'s existing figure; the
instruction trace adds detail, not a different reading), lower on `a1`'s
identity (unread) and on whether step 2's struct is the same one
`weapon-fire.md` names (undetermined, see below).

#### Its call target needed the image base added by hand - Ghidra's own bytes are pre-relocation

**The trap that cost the most time today, and the diagnosis took a wrong turn
before landing right.** Ghidra's stored bytes for the `jal` at `0x0883f3e0`
(`read_memory`: `93 25 01 0c`) decode, by the ordinary MIPS J-type rule
(`target = ((addr+4) & 0xF0000000) | ((word & 0x03FFFFFF) << 2)` - the high
bits are `0` for every PSP text address in this build), to `0x0004964c` - a
small, sub-`0x08000000` address nowhere in this program's loaded text.
Reading the same address **live**, in the running emulator
(`Debugger.read(0x0883f3e0, 4)`), returns *different* bytes - `93 35 21 0e` -
decoding to `0x0884d64c`: `Body_ApplyImpulseAtPoint`, already named, already
the function this call site should sensibly reach.

The reconciliation: `0x0004964c + 0x08804000 = 0x0884d64c`, where
`0x08804000` is this program's own image base
(`get_current_program_info`'s figure at the top of every session). Checked
against a control - all **six** `jal`s inside `Body_ResolveContactPair`,
calls to `Body_Translate`, `Body_ApplyImpulseAtPoint` and `FUN_0884dc60` this
project has trusted since 2026-08-11 - and the same addition recovers the
live-verified target from Ghidra's static bytes at every one of them,
including the exact pair this page's tail-call row already cites
(`0x499f8 + 0x08804000 = 0x0884d9f8` is `Body_Translate`; `0x49c60 +
0x08804000 = 0x0884dc60` is `FUN_0884dc60`) - so that earlier finding was
right, not lucky.

**So this is not database corruption.** Ghidra's stored instruction bytes
hold the **pre-relocation** `R_MIPS_26` target field - the raw word-address
bits an `R_MIPS_26` relocation entry is meant to patch - while the bytes
PPSSPP's loader actually runs are **post-relocation**, patched at load time.
Ghidra's importer evidently set the image base correctly (every address this
page cites by name resolves fine) without applying that relocation to the
`jal` instructions' own encoded bits. **The rule going forward**: a `jal`
target read from Ghidra's *static* disassembly needs the image base added by
hand - `target = image_base + ((word & 0x03FFFFFF) << 2)`, no `(addr+4) &
0xF0000000` term, since the field itself is unrelocated. A `jal` target read
from *live* memory needs the ordinary hardware rule instead, since the
relocation is already applied there. Mixing the two - which is what the first
pass through this investigation did, computing `0x08800000 + 0x0004964c` by
hand and landing inside `Ship_UpdateCraft` instead of at a real function - is
the actual trap; the fix is not a health check, it is remembering which of
the two addition rules the bytes in front of you were produced by. Prior
static readings that cited a call target by number are mechanically
recoverable with this constant, not suspect.

**Left open**: whether this is a general PSP-ELF-relocation trait or specific
to how this project's importer step handled `BOOT.BIN`, and whether it
explains anything about the decompiler failing on every function this session
tried to decompile (including `Body_ResolveContactPair` itself, clean in
2026-08-11) - no evidence ties the two; unapplied `jal` relocations would not
on their own explain a decompilation failure, so treat the decompiler outage
as a separate, undiagnosed thing rather than the same bug.

### `FUN_0883f540` (`0x0883f540`-`0x0883f7b7`): `a1` is the ship's own position

**Read 2026-08-19.** `Ship_ApplyCollisionImpulse`'s caller, and it settles the
`a1` question: this function reads `s1 = *(entity+0x794) + 0x30` early
(`entity+0x794` is the ship's scene node, already established via
[`camera.md`](camera.md)/[`exhaust.md`](exhaust.md)/[`missile.md`](missile.md)),
carries `s1` unchanged through three other calls, and passes it as `a1` at the
`Ship_ApplyCollisionImpulse` call site (`0x0883f5a0`/`0x0883f5a4`). The scene
node's own layout, read off this same function's copy loop at
`0x0883f720`-`0x0883f758` (four `lv.q`/`sv.q` pairs at `+0x00`, `+0x10`,
`+0x20`, `+0x30` into `entity+0xf70..0xfa0`), is three 16-byte orientation
rows followed by a fourth: **`+0x30` is the scene node's world position.** So
`a1` is the ship's own current position - not a contact point, not anything
weapon- or rival-supplied. `Body_ApplyImpulseAtPoint`'s lever arm
(`point - body.position`) is therefore zero by construction for every call
this project has found, which is why [`crate::wall::apply_pending_impulse`]
applies the impulse as a pure linear push with no angular term: not an
approximation, a structural fact of the one caller that exists.

`FUN_0883f540` itself is a ship's per-tick combat-reaction dispatcher, not
collision-specific - worth recording so the next reader does not mistake it
for a second contact-response function. In call order: `FUN_0883efb4`
(unread), `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`, already named and
already cited by `crates/physics/src/damage.rs`'s own module doc), a third
"pending scalar, consumed once" function at `FUN_0883f228` (unread as a
function, but its own body is small and clean: `if (*(entity+0x4c)+0x128) >
0.0`, call `Ship_AddShield(entity)`, then zero that field - a *fourth*
pending-value slot on the same struct, this one arming a shield grant, unread
beyond that), `Ship_ApplyCollisionImpulse`, then weapon-fire bookkeeping
(`Ship_AcquireLock`/`Ship_FireHeldWeapon`, gated on `*(entity+0x4c)+0x1bc`
being `1` or `10`), `FUN_08844ec4` (unread), `FUN_0883f424` (unread), and
`Ship_State` (`0x0883e64c`, already named) - plus a large branch on the same
`+0x1bc` state building what reads like HUD hit-notification text, not
chased.

### `FUN_08867b50` (`0x08867b50`-`0x08867f1b`): `Weapon_PostBlastImpulse_q`'s caller, and it settles `targetIndex`

**Read 2026-08-19.** Disassembled in full (decompiler resistant, same as
everything else on this page today). Signature `(craftArray, sourceIndex)` -
no target argument at all. It sweeps `0..*(0x0885b7f8)` (a fixed global -
a different count source than `FUN_08867370`'s own `craftArray+0x164`, which
turned out live to be `1`, not a craft count at all - see that function's own
section) as *candidate targets* over `craftArray`; whether `*(0x0885b7f8)`
itself resolves to `8` is not independently measured here, but the *target*
half of this sweep is corroborated another way - the live check settling
`T`'s identity (two sections up) confirms at least one real candidate this
sweep selects lands on a genuine craft. For each candidate:

1. Skip if a validity check (`FUN_0005ed4c` real address, unread) rejects the
   candidate.
2. **A box pre-check**, axis by axis: reject unless the candidate's position
   lies within `±f20` of the source's position on every axis, where `f20` is
   `+0x100` off a per-weapon-type stats entry - looked up through a **fixed
   global weapon-type index** at `*(0x0885bff8)`, not through the source
   craft's own `+0x40` field the way `Weapon_PostBlastImpulse_q` itself
   indexes its `stats`. Whether these two lookups always agree (plausible if
   the global tracks "the weapon type of the blast currently being
   processed") is not shown here - left open rather than assumed.
3. **A sphere check** against the same `f20`, then a **second, separate**
   check against `+0xec` off the *same* fixed-global-indexed stats entry -
   `+0xec` being the exact field `Weapon_PostBlastImpulse_q` itself reads as
   `radius`. Two radii, coarse then precise, both from the same lookup.
4. If both pass: `*(source+0x3c) |= 0x4` (a "this blast connected" flag, set
   on the source, not the target), then **`Weapon_PostBlastImpulse_q(craftArray,
   sourceIndex, candidateIndex)`** - `candidateIndex` is exactly this loop's
   own counter. This settles `targetIndex` completely: it is not chosen by
   any targeting logic in `Weapon_PostBlastImpulse_q` itself, it is handed in
   by whichever candidate this sweep is currently on.

After the loop, unconditionally: a two-call side effect (`0x0885abc4` then
`0x0894c784`, both unread, called with a `100.0` literal and `-1`/`0`
sentinel-shaped trailing arguments - a camera shake or a screen/audio cue is
the obvious guess given the shape, not confirmed), gated on
`*(0x0885b7f8)->0xb8 < 14` skipping it. Measured, not chased further; nothing
here feeds the pending-impulse path.

### `FUN_08867370` (`0x08867370`-`0x0886759b`): the fuse that drives the sweep

**Read 2026-08-19**, following `FUN_08867b50`'s one confirmed caller
(`search_instructions` on the image-relative `jal` operand, same method as
every other caller found this session). Signature `(craftArray, dt)` - `dt`
in `f12`, the leading-float-does-not-reserve-`a0` convention this project has
already seen on `Ship_Damage`. It loops `0..craftArray->0x164` - **corrected
below: measured live at `1`, not `8`, so "every craft" was an unverified
extrapolation from the loop's shape and is withdrawn** - and for each one:

1. `craft->0x48 -= dt`, written back unconditionally - **a per-craft countdown
   timer**, decremented every call.
2. If the timer is now `<= 0.0`: an optional side effect identical in shape to
   `FUN_08867b50`'s own post-loop block (same two unread calls, same `< 14`
   gate, same `source+0x3c |= 0x4` flag - gated here on `*(craft+0x3c) & 0x1`
   rather than always), then **unconditionally** `FUN_08867b50(craftArray,
   thisCraftIndex)` - the expiring craft becomes the blast's `sourceIndex`.

So the chain from `FUN_08867370` down is: `FUN_08867370` counts down whatever
`craftArray->0x164` names (see the correction just below - not a craft
count), and fires `FUN_08867b50` sourced from whichever entry's timer
expires → `FUN_08867b50` sweeps its own candidate targets over `craftArray`
(a different, wider count - see that function's own section above), box-then-
sphere range-checks each one, and calls `Weapon_PostBlastImpulse_q` for every
candidate that qualifies → `Weapon_PostBlastImpulse_q` posts the impulse this
page has already fully read. What arms the timer, and what calls
`FUN_08867370` itself, are both still open - the next two sections narrow the
first without closing either.

**Correction, same session: `craftArray->0x164` is not "eight race craft" -
but `craftArray` the pointer is still the roster.** A live breakpoint at
`FUN_08867370` (`0x08867370`), read for its own `a0` and `*(a0+0x164)`,
measured the count at **`1`** during a driven Single Race with a full
eight-craft grid. "It loops every craft" - this page's own first attempt at
describing this loop - does not survive that measurement: whatever
`+0x164` off `craftArray` counts, it is not a fixed roster size, and the
loop only visits that many entries via `craftArray + i*4 + 0x64`, so it is
not visiting all eight craft as fuse-bearers either. That is a narrower claim
than it first looks, though: it says `+0x164` is not a craft count, not that
`craftArray` itself is some other base entirely. The pointer keeps flowing
unchanged into `FUN_08867b50` and then `Weapon_PostBlastImpulse_q`, and
*that* function's own target identity was independently settled at a live
breakpoint two sections up, by a completely different route (enumerating
craft off `Ship_ApplyCollisionImpulse`'s own `a0`, not through `craftArray`
at all) - `T` matched a real craft's body pointer by direct equality, which
would not hold if `craftArray` itself were some other pool. So the open
question is narrow and specific: what `craftArray->0x164` actually counts,
not what `craftArray` is a base of. **One guess it rules out rather than
supports: "mines currently armed."** The breakpoint was hit right after
loading into the race, before any craft had fired a weapon - no mine could
have been armed yet - and the count still read `1`, not `0`. A count of `1`
taken that early is at least as consistent with "the player's own craft" (the
watchpoint script that follows enumerates entity `0` at this same point and
it is the one whose `+0x48` gets hit repeatedly) as with any mine-related
count. Left open rather than guessed at.

### `FUN_08863a20` (`0x08863a20`-`0x08863ba3`): `Weapons_DispatchFire`'s `world+0x44` handler, and it looks like the Mine's own fire handler

**Read 2026-08-19**, chasing `weapon-fire.md`'s own hint: "`world+0x44`'s
handler ... fires *backwards* (`vneg_q` on the craft's forward row), which
reads as a Bomb or a Mine". Disassembled in full - the decompiler fails on it
too. Signature `(subsystem, craft, craftIndex)`, matching the dispatch call
`FUN_08863a20(world+0x44, craft, i)` `weapon-fire.md` already documents. In
order:

1. Sets bit `0x2` on `subsystem+0x2c` (an armed/active flag on the subsystem
   itself), clears `craft->held` (`craft+0x1bc = -1`) and its own dispatch bit
   (`craft+0x1b8 &= ~0x100`) - the same "one shot, cleared immediately" shape
   every other fire handler on this page uses.
2. **A pool-capacity gate**: skip spawning entirely if `subsystem+0xc4` (a
   cursor) is `>= 32`.
3. Allocates the pool slot at `subsystem + cursor*4 + 0x44` - **the exact same
   `+0x44` indexing every function on this page has been reading as
   "craftArray-style, index then dereference to an entity pointer"** - and
   sets, on the new entity: `+0x3c = 1` (an active/alive flag, the same
   offset `Weapon_PostBlastImpulse_q` and its own callers read/write as a
   tri-state or bit flag elsewhere on this page); `+0x40 = craftIndex`,
   **the owning craft's own index, not a weapon-type index**; `+0x44` = the
   next value of a global incrementing counter (a unique spawn id).
4. **`+0x40 = craftIndex` corrects an assumption this page was carrying
   forward uncritically.** `Weapon_PostBlastImpulse_q` reads `S->0x40` as
   "indexes a global per-weapon-type stats table" purely from its shape (index
   then dereference); this spawn site shows at least one entity's `+0x40` is
   populated with the *craft's own index*, not a weapon-type enum. The two
   are reconcilable if the table at `0x0885bff0` is keyed **by craft** (each
   of the eight craft's currently-armed weapon) rather than by a flat weapon
   type - consistent with everything read on this page, since a mine's
   source craft and its currently-held weapon type would coincide at the
   moment of firing - but that is an inference from one spawn site, not a
   read of the table's own contents. Left as a correction to the assumption's
   confidence, not a settled fact.
5. Spawns the entity via a call at `0x0885f188(entity, craft's own node+0x30,
   staged_direction)`, where `staged_direction` is the craft's forward row
   **negated** (`vneg.q`) before being passed in - **this is the "fires
   backwards" behaviour `weapon-fire.md` already flagged**, now traced to its
   exact mechanism: not a separate reversed-facing spawn, a literal negation
   of the same forward vector every other weapon fires along. Confirms this
   handler places something *behind* the craft, the shape a dropped mine
   wants and a forward-flying rocket or missile does not.
6. The same `< 14` side-effect gate and the same two unread calls
   (`0x0885abc4`/`0x0894c784`) every function on this page has now shown at
   least once, here with a `20`-literal parameter where `FUN_08867b50` and
   `FUN_08867370` both used `12` - a per-caller intensity/duration argument is
   the obvious guess, unread.
7. Increments the pool cursor.

**`craft->0x48` - the fuse `FUN_08867370` decrements - is written nowhere in
this function.** A clean negative result, not an oversight: every field this
function touches on the new entity is listed above, and `+0x48` is not among
them. The spawn call at `0x0885f188` was the next candidate this page
expected to chase - it did not pan out cleanly; see the next section for why
and for where the search actually landed instead.

**Confidence on "this is the Mine": informed guess, not measured.** The
"fires backwards" mechanism, the subsystem-pool-plus-craftIndex-plus-unique-id
spawn shape, and process of elimination against the Rocket
(`Weapon_FireRocket`), Missile (`Weapon_FireMissile`, `world+0x4c`'s handler)
and Cannon (`Weapon_UpdateBurstFire_q`, the likelier read for the bit-`0x2`
handler per `weapon-fire.md`) all point the same way, but nothing read this
session confirms it against a weapon-select UI string or inventory label.

### A fuse-arming candidate, found by searching for the write directly - unconfirmed

**Read 2026-08-19**, after the register-provenance chase into `0x0885f188`
(the previous section's own spawn-helper call) turned out too ambiguous to
resolve statically - two callers jump directly into that address, past the
function's real prologue at `0x0885efc4`, leaving its expected locals unset
the way normal entry would set them. Rather than force that open question,
`search_instructions` for every `swc1` storing to a `+0x48` offset off a
non-stack register found **`FUN_0885bf84`** (`0x0885bf84`-`0x0885c153`):

```
entity->0x48 = |a2| * 3.6 + weaponTypeRecord->0xbc
```

`a2` is this function's own third argument, a vector whose magnitude is taken
first; the per-type record comes from a **different** global table than the
one `Weapon_PostBlastImpulse_q` reads (indexed the same "count-then-lookup"
way, but a distinct base address and a distinct field, `+0xbc` rather than
`+0xe8`/`+0xec`/`+0xf0`/`+0xfc`). This is exactly the shape a fuse-arming
write should have - a distance- or launch-speed-scaled base time plus a
per-weapon-type offset - and it is a `=`, not a `-=`, consistent with arming
rather than the routine per-tick decrement `FUN_08867370` already reads.

**Its caller, `FUN_0886a920`, is its own pool-allocation function - and its
pool does not obviously match `FUN_08863a20`'s Mine pool.** Read for its
shape only: cap `16` (`FUN_08863a20`'s was `32`), allocates via
`subsystem + cursor*4 + 0x64` (`FUN_08863a20` used `+0x44`) - two real
differences pointing at a **separate pool for a separate weapon type**, not
the Mine's own path read in the previous section. (A third-looking signal
doesn't hold up: this function also tags the new entity `+0x3c = 2` where
`FUN_08863a20` set `1`, but both zero the field first and then set it in the
same two-step shape, which reads just as well as one shared entity layout
using `+0x3c` as a type discriminant as it does as two unrelated pools - the
tag value alone isn't independent evidence either way, so only the cap and
the indexing offset are counted above.) Reached through a two-instruction
trampoline,
`FUN_0886b458` (`0x0886b458`-`0x0886b473`, forwards its third argument into
the second slot too), whose own caller is not found by a static `jal` search
- the same indirect-dispatch shape as `FUN_08867370` itself.

**A static re-read the same session found a positive match that cuts the other
way, though.** `FUN_08867370`'s own decrement instruction - read in full this
time (`disassemble_bytes` over the whole function, not inferred from the
earlier partial trace) - resolves its per-slot entity with `lw s4, 0x64(s3)`,
where `s3` walks `craftArray + i*4` exactly like every pool on this page. That
is the identical `subsystem + i*4 + 0x64` indexing `FUN_0886a920` uses to
allocate the very entity `FUN_0885bf84` arms, and the decrement itself -
`lwc1 f12, 0x48(s4)` / `sub.s` / `swc1 f12, 0x48(s4)` at `0x088673fc`-
`0x08867410` - lands on the exact field `FUN_0885bf84` writes. Two matches,
not one: the slot arithmetic and the field offset both line up. What still
does not line up: `FUN_0886a920`'s own allocation cursor lives at
`subsystem+0xa4` (`sltiu a0, a2, 0x10` gates it against the cap `16` there),
while `FUN_08867370`'s loop bound is `craftArray+0x164` - a different offset,
so this is not proof the two functions read the very same field, only that
they index the very same *shape* of array off what could plausibly be the
same subsystem struct (a rotating write-cursor and a separate live-count on
one struct is an ordinary shape, not a stretch). Confidence moves from
"unconfirmed, pool shape doesn't match" to **"unconfirmed, but the slot
arithmetic and the written field now both match exactly"** - a real
upgrade, not a settled question.

**A live check followed up on this, properly controlled - and it neither
confirmed nor refuted it, for a new and specific reason.** A first attempt
used a halting write watchpoint with no control alongside it, which
`ppsspp-debugger.md`'s own "always arm a positive control" section already
warns reads as indistinguishable from a false negative - so it is not
reported on its own here, only folded into the second attempt below. Redone
per that page's actual recipe - `enabled: False, log: True` at full speed,
plus a positive control watchpoint over 156 bytes of a live craft body
(the same shape as that page's own worked example) running alongside:
caught `FUN_0885bf84` arming a fresh entity, armed the log watch on its
`+0x48` and the control on the craft body, ran free for 90 seconds. The
control counted **30,413 hits** - proof the watch mechanism, the log, and
the emulator were all healthy the whole window, not stalled or wedged. The
target counted exactly **one** hit, `FUN_0885bf84`'s own arming write,
logged correctly with its PC. And yet by the end of the same 90 seconds,
that entity's `+0x3c` tag had changed (`2` to `5`) and its `+0x48` field
held a new value neither `0.0` nor what the arm write had set - meaning
something wrote there, controls says the watch works, and the log has no
line for it. Repeated with a lighter, uncontrolled version of the same
setup on a separate entity, tag changed `9` to `5` inside three minutes;
both changes read as the pool slot being freed and recycled to a different
occupant faster than expected, well inside the two-to-three-minute windows
tried here, and whatever writes a freed slot's memory during that handoff
is not going through the path `CHK Write*(CPU)` logs. **New trap for
`ppsspp-debugger.md`, not just a dead end on this page**: a watchpoint with
a verified-working control can still miss a real write if the writer takes
a different path than an ordinary CPU store - a zero count is not proof of
absence the way the existing "always arm a control" section implies, only
proof the *ordinary* path stayed silent. See
[ppsspp-debugger.md](../../../reverse-engineering/ppsspp-debugger.md#a-controlled-watch-can-still-miss-a-write---the-pool-slot-outlives-the-log-does-not)
for the full account.

So: settling whether `FUN_0885bf84` is really the write `FUN_08867370` needs
is still open, but for a narrower reason now - not "the pool shape doesn't
match" (it mostly does), but "this weapon-instance's slot lifetime is
apparently too short to catch mid-flight with a write watch, and the
watch's blind spot for however it gets recycled is itself unexplained".
Reading what `+0xbc`'s per-type table actually enumerates, finding
`FUN_0886b458`'s own caller, or catching a slot at the *moment* it is
armed and reading its `+0x48` on every single tick thereafter (rather than
trusting a watchpoint to report absence) are the concrete next steps.

### Not determined, again

- **Whether step 2's Shield flag and `weapon-fire.md`'s `entity+0x1b8`
  fire-request word are the same struct or two different ones that happen to
  share an offset.** `weapon-fire.md` reads the flag word one indirection
  shallower (`craft+0x1b8` direct) than this function does
  (`*(*(entity+0x4c))+0x1b8`); reconciling the two needs either a struct dump
  at both addresses or a live comparison, neither done here.
- **What `*(entity+0x4c)+0x1bc`'s states mean**, beyond `1` and `10` gating a
  weapon-fire-bookkeeping call and the HUD-text branch above - a full case
  read of `FUN_0883f540` past what this pass needed.
- **`Weapon_PostBlastImpulse_q`'s caller chain is now read two hops up**:
  `FUN_08867b50` and its own caller `FUN_08867370` (the fuse driver), both
  above. Still unread: `FUN_08868a10` and `FUN_088690fc` (the second writer's,
  two call sites), `FUN_0883f540` (`Ship_ApplyCollisionImpulse`'s - now partly
  read above, but its own five unread callees are not), what arms
  `craft->0x48`'s fuse in the first place, and whatever calls
  `FUN_08867370` itself every tick (not found via a static `jal` search -
  see that section for why an indirect/table dispatch is the working guess).
  Reading the second writer's callers is what would identify its own trigger
  and confirm the consumer's calling context beyond the `a1` answer above.
- **What is still not wired.** `Ship_ApplyCollisionImpulse` has a Rust port -
  [`crate::wall::apply_pending_impulse`] - and so does
  `Weapon_PostBlastImpulse_q` - [`crate::wall::post_blast_impulse`], both
  correct and tested against directly-supplied inputs. But nothing in this
  crate calls `post_blast_impulse`: `FUN_08867b50` and `FUN_08867370` are read
  well enough now to know exactly *how* a blast is dispatched and *why*
  `targetIndex` is whatever it is, but not well enough to know *when* one
  actually fires - the fuse-arming site and the per-tick caller feeding
  `FUN_08867370` its `dt` are both still open. The second writer has no port
  at all. So `pending_impulse` is still never set by anything in this crate,
  and `apply_pending_impulse` stays a correct, tested, but fully inert no-op
  end to end.

### `0x08815ccc` is a stub, so craft-to-craft contact does not come from the narrowphase

**Read 2026-08-11, and it changes the picture.** `Collision_DispatchPair`
(`0x08816eac`) sends a box proxy against a box proxy to `0x08815ccc`, and
`0x08815ccc` is **two instructions**:

```
08815ccc  jr    ra
08815cd0  _nop
```

It reports nothing, ever. Ghidra does not even claim it as a function - there is
nothing there to claim. Confidence **90**: four bytes of `0x03e00008` need no
interpretation, and `collision.md`'s dispatch reading, which the table above
rests on, is independently at 85.

Craft are box proxies. So **no pair of craft can produce a contact through
`Collision_StepNarrowphase`**, and whatever reaches `Body_ResolveContactPair`
with two craft in it is somewhere this project has not found. That is consistent
with, and sharpens, the open item about the pending impulse at
`entity->0x4c + 0x110`: the note that its writer "is somewhere in the weapon or
rival-contact code" now has the narrowphase positively excluded rather than
merely unexamined.

**The consequence for `oag_physics::pair`**: its detection is not an
approximation of a recovered test, because there is no recovered test to
approximate. It uses an oriented box against an oriented box over the hull's own
`<Misc width height length>`, and says so.

### Still not determined

- **What feeds `Body_ResolveContactPair` a pair of craft**, given the above.
- **`0x0881702c`** (mesh against mesh, gated on `world+0x5464`).
