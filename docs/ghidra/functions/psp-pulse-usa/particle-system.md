# The runtime particle system (`Psys`, PSP `BOOT.BIN`)

The interpreter that turns a loaded `.pob`/`SYSP` resource
([pob.md](../../../formats/pob.md)) into moving, coloured, sized particles.
Traced end to end on 2026-08-01, starting from the one open question the
collision-sparks investigation had left - "what do the six derived fields
`FUN_088f4910` writes actually mean?" - and closing it, plus most of the
emitter model around it. Everything below was decompiled in full, and the
load-bearing claims were then checked against `WO_SHIP_COLL_SPARK_DAMAGE.POB`'s
own file bytes and three live PPSSPP captures (a memory read of the loaded
resource, and two breakpoint runs on the emit functions during real wall
crashes). Where a claim rests on the decompile alone, its confidence says so.

Format-level consequences (the emitter record layout, the four-emitter
sibling tree, the channel blocks and colour table) live in
[pob.md](../../../formats/pob.md); this page is the function-by-function
record. The consumer in this repository is `oag_render::sparks`, which
transcribes the collision-spark emitters.

## The call graph

```
ParticleSystem_Update            0x088f5b9c   per-instance tick
├─ ParticleSystem_CacheModeFlags 0x088f4dcc   per-system globals (channel modes)
├─ [attr-anim array res+0x93c]              → re-runs DeriveScaledParams per tick
│   └─ ParticleSystem_DeriveScaledParams 0x088f4910
├─ ParticleSystem_UpdateEmission 0x088f4f84   spawn schedule
│   └─ ParticleSystem_SpawnBurst 0x088f56c4   dispatch on emitter shape (res+0x30)
│       ├─ 0: ParticleSystem_EmitPoint  0x088fba30
│       ├─ 3: FUN_088fc634 (cone placement - unread)
│       ├─ 4/7: ParticleSystem_EmitSphere 0x088fd340
│       ├─ 6: ParticleSystem_EmitBox    0x088fc040
│       └─ each → ParticleSystem_InitParticle 0x088f6e6c
│                 ├─ ParticleSystem_ConeVelocity 0x088fc37c   (velocity mode 0/2)
│                 └─ ParticleSystem_AimedVelocity 0x088fc490 (velocity mode 1)
└─ ParticleSystem_UpdateParticles 0x088f635c  integrate, colour, size, expire
    └─ ParticleSystem_OnParticleDeath 0x088f4994 ("DEAT"/"DEAS" child spawn)
```

Random helpers, used everywhere: `Psys_RandIntRange` (`0x088f8c3c`,
uniform int in `[a, b]`), `Psys_RandFloatRange` (`0x088f8d94`, uniform float
in `[a, b]`), `Psys_RandSpread` (`0x088f8dd0`, **`a + b * U(-1, 1)`** - a
centre-and-spread, *not* a min/max; misreading this flips every "±" claim
on this page). `powf` (`0x08985e30`) is the plain C library power function
(sign-and-integer-exponent handling included) and keeps its library name.

## `ParticleSystem_DeriveScaledParams` (`0x088f4910`) - the question that started this

Confidence **90**. The six derived instance fields, previously recovered
byte-exact but meaningless
([contact-response.md](contact-response.md#fun_088f4910-derives-six-instance-fields-from-a-plain-scalar-block)),
are the **severity-scaled emitter parameters**:

| instance | from resource | means | consumer |
| --- | --- | --- | --- |
| `+0x58` | `+0x34 * sev` | emitter extent, axis 1 (units) | `DAT_08ab2290`, read by every emit function |
| `+0x5c` | `+0x38 * sev` | emitter extent, axis 2 | `DAT_08ab2294` |
| `+0x60` | `+0x40 * sev` | emitter extent, axis 3 | (same global family) |
| `+0x64` | `+0x48 * sev` | ejection speed **centre**, units/tick | `Psys_RandSpread(+0x64, +0x68)` in every velocity init |
| `+0x68` | `+0x4c * sev` | ejection speed **spread** | same call |
| `+0x6c` | `+0x74 * sev` | gravity, units/tick², **flag-gated** | `UpdateParticles`, only under resource flag `0x200` |
| `+0x70` | `+0x4cc * inst+0x3c` | playback-rate multiplier on the whole system's `dt` | `ParticleSystem_Update` |

"sev" is the instance's `+0x34` - for collision sparks, the trigger's
recovered `intensity * 2.0 + 0.4` - times the co-factor pairs
`+0x28/+0x44` and `+0x2c/+0x48`, all live-captured at their neutral `1.0`.
The co-factors are not dead weight: they are the targets of the
**animated-attribute array** at `res+0x93c/+0x940` (records of `0xec` bytes,
each a channel evaluator plus a target selector at `+0xe0` mapping
`1..6 → instance +0x44/+0x48/+0x50/+0x4c/+0x54/+0x34`), and when that array
is non-empty `ParticleSystem_Update` re-runs this function **every tick**.
`WO_SHIP_COLL_SPARK_DAMAGE` has zero such records, so its derived params are
constant per burst.

## `ParticleSystem_Update` (`0x088f5b9c`)

Confidence **85**. Per-instance tick. The pieces that matter:

- **`res+0x24` is the emitter duration in ticks.** `instance+0x138` counts
  *down* from it; at zero with no live particles the instance sets its dead
  flag (`+0x160 |= 8`) and returns 0. `instance+0x13c` is the normalized
  age `1 - remaining/duration` used by every emitter-side channel evaluator.
- `instance+0xc = global_timescale * 60 * dt`, clamped to `3.0`, then
  multiplied by the `+0x70` playback rate - the whole system runs on a
  tick-count `dt`, which is what makes "units per tick" the native unit
  everywhere. `DAT_08b6207c` is the same `dt` in seconds.
- Walks the `res+0x9b4` **modifier list** for a type-3 node and precomputes
  `pow(k_axis, dt)` per axis into `DAT_08b62090..98` - exponential drag
  (see `UpdateParticles`).
- Emission (`UpdateEmission`) is gated on instance flag `+0x160 & 1` and a
  positive remaining duration.
- Recurses into the death-effect instance (`+0x1a4`/`+0x1a8`) and the
  per-particle-child list (`+0x1ac`).

## `ParticleSystem_UpdateEmission` (`0x088f4f84`)

Confidence **85**. The spawn schedule, all fields now grounded in
`WO_SHIP_COLL_SPARK_DAMAGE`'s bytes and confirmed live:

- `res+0x64`/`+0x68`: emission **interval**, min/max integer ticks
  (`Psys_RandIntRange`).
- `res+0x6c`/`+0x70`: **particles per emission**, min/max integers.
- `res+0xa0`: **live-particle cap**; an emission that would exceed it is
  skipped whole.
- The countdown timer at `instance+0x04` spawns while `<= 0` then adds the
  interval - so an emitter emits on its **first** update tick.
- An emission-scale channel block at `res+0x858` (evaluated over emitter
  age) times `instance+0x58`/`+0x5c` feeds `DAT_08ab2290/94`, the extents
  the emit functions read; both are floored at `1e-05`.
- Resource flag `0x800000` turns the duration into a repeat *count*
  (`instance+0x08` decrements per burst).

## The emit functions

`ParticleSystem_SpawnBurst` (`0x088f56c4`, confidence **90**) is a bare
switch on `res+0x30`: 0 → point, 1 → `FUN_088fcfec`, 2 → `FUN_088fbcec`,
3 → `FUN_088fc634`, 4 and 7 → sphere (7 passes a hemisphere flag),
6 → box, 8 → `FUN_088fcb10`. The unnamed cases are unread.

`ParticleSystem_EmitSphere` (`0x088fd340`, confidence **85**): direction
uniform on the unit sphere by the standard construction (`z = U(-1,1)`,
azimuth `U(0, 2π)`, in-plane radius `sqrt(1-z²)`); **shape 7 forces
`abs()` on one component** - a hemisphere. Spawn radius comes from
`DAT_08ab2290` shaped by `res+0x3c`: 0 → exact, 1 → `±instance+0x60`
spread, 2 → `* sin(U(0, π/2))`. With velocity mode ≠ 2 the particle's
velocity is the **same direction** times `Psys_RandSpread(inst+0x64,
inst+0x68)` - a radial burst; mode 2 builds a random tangent instead.
Under resource flag `0x200000` the direction comes from a
Fibonacci-spiral (`3.6/sqrt(N·(1-z²))` azimuth increment) - a *uniform*
sphere covering, not a random one. `ParticleSystem_EmitPoint`
(`0x088fba30`, **75**) zeroes the offset; `ParticleSystem_EmitBox`
(`0x088fc040`, **75**) draws each axis `U(-r, r)` from the same global.
Every emit function distributes the burst's particles along the emitter's
per-frame motion delta (`instance+0xc0`, scaled by `i/n`) when the global
sub-frame-spread flag (`DAT_08b620a0`, resource flag `0x20`) is on.

Velocity inits: `ParticleSystem_ConeVelocity` (`0x088fc37c`, **80**) draws
an angle `U(-a, a)` with `a = res+0x58` **in degrees** (the `π/180` is in
the code), takes its sin/cos, and scales by the speed spread.
`ParticleSystem_AimedVelocity` (`0x088fc490`, **88** - the `_q` is
resolved, see "The emit frame and the velocity dispatch, settled" below)
builds `y = sin(res+0x50 + jitter)`, horizontal components scaled by the
matching cosine with their heading taken from the input direction's rotated
by `res+0x54 + jitter` - so **`+0x50` is an elevation over the emitter's
horizontal plane and `+0x54` an azimuth offset**, both radians, both
jittered by `±res+0x58` degrees.

## `ParticleSystem_InitParticle` (`0x088f6e6c`)

Confidence **85**. Runs once per new particle:

- **Lifetime**: `Psys_RandSpread((int)res+0x5c, (int)res+0x60)` - stored as
  **integer ticks** in the file - times `instance+0x54 / 60` into seconds;
  `1/lifetime` cached for normalized age. Resource flag `0x800` means
  immortal (`FLT_MAX`).
- **Velocity inherit**: particle velocity `+= instance+0x150`,
  unconditionally. `+0x150` is written when a *child* system spawns off a
  parent particle (scaled by the child resource's `+0x4d0`); neither
  `ShipCollisionFx_Trigger` nor the generic constructor writes it for a
  top-level instance (checked by instruction scan over both), so collision
  sparks inherit **no** craft velocity - the moving-anchor effect comes
  from the emitter node riding the hull instead.
- **Colour, mode 2** (`res+0xbc == 2`): one **random** entry of the
  256-colour table at `res+0xc4`, RGBA. (The over-life palette walk lives
  in `UpdateParticles` and runs only when the mode is *not* 2.)
- **Spawn-time channel values**: the three channel blocks - size
  (`res+0x4d8`), alpha (`res+0x5b8`), rotation speed (`res+0x698`) - are
  evaluated at age 0 (mode 0 = keyframed, 2 = constant, 3 =
  `Psys_RandFloatRange(lo, hi)`); alpha lands in the particle's colour
  byte, rotation speed is sign-flipped half the time under flag `0x8`.
- **Per-particle child**: if `res+0x948` is set, with probability
  `child+0x4d4` a whole child system spawns attached to this particle,
  inheriting the particle's velocity times `child+0x4d0` - this is how
  trail-per-spark effects are authored. (`WO_SHIP_COLL_SPARK_DAMAGE` has
  none.)
- Random initial billboard roll under flag `0x4`; random sprite-atlas frame
  from `res+0x9ac` under flag `0x4000000`; world-transform of spawn offset
  and velocity through the node matrix at `instance+0xf0` unless flag
  `0x2` (emit-in-world-space) is set.

## `ParticleSystem_UpdateParticles` (`0x088f635c`)

Confidence **85**. Per live particle, per tick:

- **Gravity**: `velocity.y += instance+0x6c * dt` **only under resource
  flag `0x200`** - which every emitter of the collision-spark file leaves
  clear, so its authored gravity values are dormant. Then
  `position += velocity * dt` with `dt` in ticks - velocities are
  **units per tick**.
- **Modifier list** (`res+0x9b4`, node type at `+0x24`, next at `+0x30`):
  type 3 multiplies velocity by the precomputed per-axis `pow(k, dt)` -
  exponential drag (root emitter `k = 0.98`, `WO_SHIP_COLL_SPARK` `0.85`,
  `_TRAIL` `0.95`). Types 4, 0xb, 0x11, 0x12, 0x13 dispatch to unread
  handlers (`FUN_088fa744`, `FUN_088fa818`, `FUN_088fab50`, `FUN_088faab8`,
  `FUN_088fb11c`) - other force/attractor kinds, confidence 40 on that
  characterization.
- **Colour over life** (mode ≠ 2): RGB from
  `palette[(int)(age * 255.999)]`, alpha byte preserved.
- **Channel animation**: the load-time-baked keyframe machinery - times at
  `res+0x9f0`, per-segment rates at `res+0xa70` (16 bytes each), start
  vector at `res+0xc70` `{alpha, size, rot-rate, frame-rate}` - integrates
  the particle's channel vector by `d(age)`, and per-mode globals from
  `CacheModeFlags` apply it: alpha byte = `clamp(alpha, 0, 255) *
  instance+0x40`, **drawn size = size-channel * `instance+0x34`** - the
  severity. This is the line that makes a harder hit's sparks bigger.
  *These tables are zero in the file and filled in by the loader*: a live
  read of the loaded resource showed `+0x9f0 = [0, 0.21539, 1.0, FLT_MAX]`
  (the alpha keyframe times) and `+0xc70 = [200, 0.5, 0, 0]` (the spawn
  values) where the file has zeros - the interpreter bakes the channel
  blocks into them at load. Confidence **80** on the baking attribution
  (the baked values match the blocks exactly; the baking *site* -
  presumably `FUN_088f58a4` - was not stepped through).
- Billboard roll advances by the rotation-speed channel; sprite-atlas frame
  advances by the frame-rate channel modulo `res+0x9ac`.
- Lifetime decrements in seconds; at zero `ParticleSystem_OnParticleDeath`
  (`0x088f4994`, confidence **75**) may spawn the `res+0x944` death-child
  system (`"DEAT"`/`"DEAS"` fourccs) before the slot is freed.

`ParticleSystem_CacheModeFlags` (`0x088f4dcc`, confidence **80**) fills the
per-system globals the loop reads: `DAT_08b62068/69/6a` are "the
alpha/size/rotation channel block is keyframed (mode 0)", `DAT_08b6206c`
keys off the blend-mode table index at `res+0xb8`, `DAT_08b62070` is the
atlas frame count as a float.

## The draw layer (added 2026-08-01, second pass)

Prompted by three visual mismatches the user reported against the running
original - invisible smoke, insufficiently spiky sparks, and bursts not
tracking the hull - the draw dispatch turned out to be findable through the
blend table's own xrefs, and it resolves all three.

### The blend table at `DAT_08ab2260`, decoded

Eight `u32` entries, `entry = (index + 1) << 28 | low_byte`. The top nibble
is the **render-mode class**; `ParticleSystem_CacheModeFlags` derives its
booleans from it (`DAT_08b62060` = mode 6 or 7, `DAT_08b6206c` = mode 3),
and the per-particle draw dispatch switches on it directly.

### `ParticleSystem_DrawParticle` (`0x089186bc`)

Confidence **85**. Per particle: pushes the view matrix, binds the emitter's
texture state, applies `Gu_TexScale`/`Gu_TexOffset` for the sprite-atlas
frame, calls `ParticleSystem_ApplyBlendClass`, transforms the particle's
position **and its second stored point** (the row-`+0x50` value) to view
space, then dispatches on the blend-table mode:

- **mode 7** → `FUN_08916d00(size, colour, &second_point, &position, ...)` -
  a two-point draw (unread internally; same signature as mode 6's);
- **mode 6** → `ParticleSystem_DrawStreak` (`0x08916820`, below) - the
  streak quad;
- **mode 3** → `FUN_08916610(roll, size, colour, &position, ...)` - a
  one-point rotating billboard (unread internally; takes the roll field
  streaks do not);
- **modes 1/2** → an inline camera-facing quad at `position ± size` in view
  space - which also settles the size channel's **unit**: view-space (and
  therefore world-space) half-extent.

Combined with `ParticleSystem_UpdateParticles`' handling of the second
point - refreshed to the current position every tick *unless* resource flag
`0x2000000` is set, in which case it stays at the spawn position - the two
streak classes fall out: `WO_SHIP_COLL_SPARK` (flag set) draws rays
radiating from the impact point, `bits`/`_TRAIL` (flag clear) draw
last-tick motion streaks.

### `ParticleSystem_DrawStreak` (`0x08916820`)

Confidence **80**. Both view-space points are first scaled onto the nearer
of the two depths (keeping the quad screen-parallel), then a triangle-strip
quad is built along the 2D direction between them: half-width
`size` perpendicular, and the ends extended along the axis by
`stretch * size` (the stretch factor comes from the draw record; a
zero-length streak still draws a glow). Colour is flat across all four
corners.

### `ParticleSystem_ApplyBlendClass` (`0x0891653c`)

Confidence **85**. Switches on the draw-state word copied from the
resource's `+0xc0`:

- class 2: `BlendFunc(ADD, SRC_ALPHA, FIX 0xffffff)` - **additive**;
- class 3: `BlendFunc(ADD, SRC_ALPHA, ONE_MINUS_SRC_ALPHA)` - **alpha
  over**;
- class 1: alpha-test path, no blend.

The factor decode leans on the sceGu blend-factor enum (`2 = SRC_ALPHA`,
`3 = ONE_MINUS_SRC_ALPHA`, `10 = FIX`) matching the GE wrapper's argument
shape; the collision-spark file uses class 3 for the smoke root and class 2
for its three siblings, which is why the smoke is a dark translucent puff
in the original and was invisible in this project's previously all-additive
port.

## The three live captures

1. **Loaded-resource read** (CPU paused, memory search for the resource
   name): every header field matches the extracted file byte-exact; the
   two fixed-up pointers land exactly at `base + baked`; the channel
   scratch tables hold the baked keyframe data described above.
2. **Cone-emit breakpoint during a wall crash**: `bits` and
   `WO_SHIP_COLL_SPARK_TRAIL` instances observed emitting with per-emission
   counts matching the file (2 and 1) and **severity `+0x34` identical
   across siblings of one burst** (`2.0282` on a hard hit, `0.527` on a
   gentle one - both inside the trigger's `[0.4, 2.4]` range), co-factors
   all `1.0`.
3. **Sphere-emit breakpoint during a sustained scrape**: the root
   (`res = resource_base`) and `WO_SHIP_COLL_SPARK`
   (`res = resource_base + 0xd20` - the **nested** record, not the
   standalone file of the same name) both emit, counts 1 and 3, the root
   on its 4-tick cadence.

## The emit frame and the velocity dispatch, settled (2026-08-10)

Chased down because `oag_render::sparks` aimed everything at the contact
normal and its wall hits read as a symmetric starburst where the original's
read as an upward fan with lines hugging the wall. Three readings, each at
instruction or byte level, two live-corroborated:

- **`ParticleSystem_EmitSphere`'s velocity dispatch is read whole**:
  velocity mode `2` builds a tangent (cross of the sphere direction with a
  second random direction, normalised); **modes `0` and `1` are both
  radial** - the sphere/hemisphere direction times `Psys_RandSpread(speed)`.
  A sphere-shaped emitter therefore never calls the aimed or cone velocity
  functions and its `+0x50`/`+0x54`/`+0x58` fields are **inert** - which a
  live breakpoint on `ParticleSystem_AimedVelocity`'s exit confirms: across
  a real crash, `bits` and `_TRAIL` hit it and the sphere-shaped bright
  spark emitter never does. The hemisphere's forced-positive axis is the
  **emitter-local `+Y`** (the `abs()` lands on the `y` lane of the
  direction before scaling).
- **`ParticleSystem_AimedVelocity`'s axis convention is pinned** (the old
  `_q`): `res+0x50` is elevation, `res+0x54` azimuth offset - see the
  velocity-inits paragraph above. Live-measured distributions at its exit
  match the authored angles exactly: `bits` (authored `0.6283` rad ± 30°)
  sampled elevations `+6.8°..+50.5°`; `_TRAIL` (authored `0` ± 21.82°)
  sampled `-18.8°..+21.3°`. Confidence **88**.
- **The emit frame is the hull locator's, and every locator is a pure
  translation.** `ShipCollisionFx_Trigger` parents the instance to the
  nearest `Ship Collision Fx` node; all six such nodes on Assegai's
  `Ship.vex` (and spot-checks elsewhere) author **identity rotations**. So
  "emitter-local `+Y`" is world up, and **the contact normal appears
  nowhere in the effect** - the wall-hugging look of a scrape's sparks is
  the near-horizontal half of a `+Y` hemisphere, not aiming.

Two file-byte corrections to the collision-spark table found on the way,
both fed back into `oag_render::sparks`: the **`bits` emitter has the
gravity flag set** (`flags 0x80000202`, `+0x74 = -0.015` units/tick² -
an earlier pass read the flag as clear on all four), and
`WO_SHIP_COLL_SPARK_NODAMAGE.POB` decodes as a two-emitter tree (the bright
fountain, byte-identical fields, plus `bits`) with no smoke and no
`_TRAIL` - the variant the trigger spawns when a contact dealt no damage.

## Open, deliberately

- `FUN_088fc634` (shape 3, cone *placement*) and shapes 1/2/8 are unread;
  `oag_render::sparks` approximates shape 3 spawn position as the anchor
  (the authored extents are at most 0.1 units - sub-pixel at any race
  camera distance).
- Inside the draw layer: `FUN_08916d00` (the mode-7 two-point draw) and
  `FUN_08916610` (the mode-3 rotating billboard) are dispatched with known
  arguments but unread internally, and the exact stretch factor
  `ParticleSystem_DrawStreak` receives is not traced back to a resource
  field.
- Modifier types other than 3, the `+0x9c8`/`+0x9cc` slot targets, and the
  emitter-local frame of the aimed cones.

## The two unread draw modes, read (2026-08-09)

Both were dispatched with known arguments but never opened. Both decompile
cleanly and completely. **Confidence 88** on the geometry each builds - the
vertex arithmetic is a direct read and the vertex *format* is confirmed
independently, see below. The one subsidiary question this section opened, which
of the two trig helpers is sine and which cosine, was **closed the same day at
90** and is written up under the mode-3 sprite.

### The vertex format both use, and why it corroborates the rest

Both end in `Gu_DrawArray(4, 0x19f, n, 0, scratch + 0x18)`. Decoding `0x19f`
against the standard PSP vertex-type packing:

| Field | Bits | Value |
| --- | --- | --- |
| texture | 0-1 | `3` = `GU_TEXTURE_32BITF`, 2 floats, 8 bytes |
| colour | 2-4 | `7` = `GU_COLOR_8888`, 4 bytes |
| normal | 5-6 | `0` = none |
| position | 7-8 | `3` = `GU_VERTEX_32BITF`, 3 floats, 12 bytes |

Total **24 bytes**, and both functions write their vertices at exactly `0x18`
spacing. The stride the decode predicts and the stride the code uses agree
without being fitted to each other, which is what makes `0x19f` safe to rely on
elsewhere. `4` is `GU_TRIANGLE_STRIP`.

Both begin with `iVar = Gfx_AllocDrawScratch(g_display, n)` and **return
silently when it yields zero** - `0xf0` (240 bytes) for the eight-vertex streak,
`0x90` (144) for the four-vertex sprite. Particle draws are therefore **dropped
without trace when the per-frame scratch arena is exhausted**, which is worth
knowing before attributing a missing effect to the simulation.

### `FUN_08916d00` - the mode-7 draw is a *capped* streak, not a plain quad

Its real signature, from the body rather than from the call site:

```c
FUN_08916d00(float half_width, float cap_ratio,
             vec4 *b, vec4 *a, u32 colour)
```

The page's earlier description of the arguments as `(size, colour, &second_point,
&position, ...)` mis-ordered them: **`param_5` is the colour**, and `param_2` is
a float that only ever multiplies `param_1`.

1. **Both endpoints are pushed to a common depth**, the mean of the two:
   `z = (a.z + b.z) * 0.5`, then `a *= z/a.z` and `b *= z/b.z`. Because a
   perspective projection divides by `z`, scaling all three components by the
   same factor **leaves the screen position exactly unchanged** while making the
   pair co-planar in view space. It is a deliberate trick, not a simplification:
   it lets a screen-space-thick streak rasterise without perspective skew along
   its length. (This page already described the mode-6 streak as scaling "onto
   the nearer of the two depths"; mode 7 uses the **mean**, so if that reading
   of mode 6 came from the same shape of code it is worth re-checking.)
2. `dir = normalize(b - a)`, falling back to `(0, 1, 0)` for a zero-length
   streak - so a degenerate streak still draws, as a blob at the sprite's own
   size.
3. `perp = (dir.y, -dir.x, 0) * half_width` and `cap = dir * (cap_ratio * half_width)`.
4. Eight vertices, one triangle strip:

   | # | position | `u` | `v` |
   | --- | --- | --- | --- |
   | 0 | `a - cap - perp` | 1 | 0 |
   | 1 | `a - cap + perp` | 0 | 0 |
   | 2 | `a - perp` | 1 | 0.5 |
   | 3 | `a + perp` | 0 | 0.5 |
   | 4 | `b - perp` | 1 | 0.5 |
   | 5 | `b + perp` | 0 | 0.5 |
   | 6 | `b + cap - perp` | 1 | 1 |
   | 7 | `b + cap + perp` | 0 | 1 |

   All eight take the same flat colour.

**The `v` layout is the finding.** Vertices 2-5 all sit at `v = 0.5`, so the
streak's *body* - the whole span from `a` to `b`, however long - samples a
**single texture row**, stretched. Only the two caps, which extend
`cap_ratio * half_width` beyond each endpoint, carry any `v` variation, running
`0 -> 0.5` and `0.5 -> 1`. So the sprite's top and bottom edges become the
**fade at each end of the streak** and its middle row becomes the body's
constant colour. A reimplementation that maps `v` linearly `0 -> 1` along the
whole streak will get a visibly different result: the falloff will smear over
the entire length instead of staying in two fixed-size caps.

### `FUN_08916610` - the mode-3 sprite is a rolled quad with an aspect ratio

```c
FUN_08916610(angle roll, float half_height, float aspect,
             float *position, u32 colour)
```

Again `param_5` is the colour. Writing `c` and `s` for the two trig helpers'
results (`FUN_0897e030` and `FUN_0897e300`), `h = half_height` and
`w = aspect * half_height`, the four corners are exactly

```
axis_u = w * ( c, -s)
axis_v = h * ( s,  c)

v0 = p - axis_u - axis_v   uv (0, 1)
v1 = p - axis_u + axis_v   uv (0, 0)
v2 = p + axis_u - axis_v   uv (1, 1)
v3 = p + axis_u + axis_v   uv (1, 0)
```

`z` is the particle's own view depth on all four - no depth equalisation is
needed for a single point. So it is a **screen-aligned quad rotated by `roll`**,
half-height `half_height` and half-width `aspect * half_height`. `axis_u` and
`axis_v` are perpendicular for any `c, s` on the unit circle, so the quad is a
true rectangle.

**The trig pairing is settled - it was a five-minute check and it came back the
natural way.** `FUN_0897e030` is **`cosf`** and `FUN_0897e300` is **`sinf`**,
confidence **90**, so `roll = 0` gives an axis-aligned quad and `axis_u`/`axis_v`
really are a rotation by `roll`.

The evidence is fdlibm's own structure rather than a guess. Both functions have
**byte-identical** prologues - mask the sign bit, compare `|x|` against
`0x3f490fd8` (`pi/4`), then against `0x7f800000` (infinity), then call the shared
argument reducer at `0x08984760` (`__rem_pio2f`). They diverge only on the
small-`|x|` path, and that is where fdlibm's two kernels differ in **arity**:

| Function | small-`|x|` call | arguments passed |
| --- | --- | --- |
| `0x0897e030` | `0x08984d14` | `(x, y=0)` - two |
| `0x0897e300` | `0x08985680` | `(x, y=0, iy=0)` - three, the third an **integer** |

`__kernel_cosf(x, y)` takes two arguments; `__kernel_sinf(x, y, iy)` takes three,
the last an int selecting whether `y` is used. The extra `li a0, 0` at
`0x0897e394` is that third argument and it is the discriminator. Library
functions keep their library names here, following `powf` (`0x08985e30`) already
on this page.

### The stretch factor: traced to `particle+0x64`, and it is one field feeding three modes

The open item asked where `ParticleSystem_DrawStreak`'s stretch comes from.
`ParticleSystem_DrawParticle` takes the **particle** struct itself (its caller
`FUN_089177e4` walks the live list by `particle+0x78`), so its `param_1[n]`
offsets are particle offsets. Read off the dispatch:

| Particle field | Passed as |
| --- | --- |
| `+0x30` | `half_width` / `size` to modes 6, 7; `half_height` to mode 3; the half-extent of the inline mode-1/2 quad |
| `+0x34` | the packed colour, to every mode |
| `+0x38` | sprite-atlas frame index |
| `+0x5c` | `roll`, mode 3 only |
| **`+0x64`** | **`cap_ratio` to mode 7, the stretch to mode 6, and `aspect` to mode 3** |
| `+0x68`, `+0x6c` | `Gu_TexScale` pair |
| `+0x70` | atlas columns, `u16` |

**One field drives all three.** This page previously treated the mode-6/7
"stretch" and the mode-3 "aspect" as separate quantities; they are the same
`particle+0x64`, which means whatever authored value feeds it has to make sense
read both ways - as a multiple of the half-width for the caps, and as a
width/height ratio for the sprite. That is a useful constraint on the search and
it was not available before.

**Closed the same day: `particle+0x64` is a hard-coded `1.0f`, not a resource
field at all.** Confidence **88**.

`ParticleSystem_InitParticleFields` (`0x088f79b4`) - the per-particle field
initialiser, sitting immediately after `ParticleSystem_InitParticle` in the
binary - writes it directly:

```c
particle[0x19] = 0x3f800000;      // +0x64 = 1.0f
```

It is one constant in a block of neighbours that **independently confirms the
offset map read off the draw side**, which is what lifts this to 88 rather than
leaving it a lone store:

| Write | Offset | Meaning |
| --- | --- | --- |
| `particle[0x11] = resource` | `+0x44` | the resource pointer the draw path reads |
| `particle[0x17] = RandFloatRange(0, 2*pi)` when `res+0x884 & 0x10`, else `0` | `+0x5c` | **roll** - exactly the field mode 3 takes |
| `particle[0x18]` | `+0x60` | roll *rate*, from the `res+0x394` channel, negated on alternate particles |
| **`particle[0x19] = 1.0f`** | **`+0x64`** | **the stretch / cap ratio / aspect** |
| `particle[0x1a] = 1.0 / u16 at +0x70` | `+0x68` | `Gu_TexScale` u |
| `particle[0x1b] = 1.0 / u16 at +0x72` | `+0x6c` | `Gu_TexScale` v |
| `*(u16 *)(particle+0x70) = res+0x880 >> 16`, `+0x72 = res+0x880 & 0xffff` | | atlas columns, rows |

Every one lands where the draw-side read predicted, including the
reciprocal-of-atlas-count shape of the `Gu_TexScale` pair. The two readings were
derived independently, from opposite ends.

**So all three modes get `1.0`:** mode 7's caps extend exactly one half-width past
each endpoint, mode 6's stretch is `1.0`, and the mode-3 sprite is **square**.
Nothing authored feeds it - which is why the "one field, three modes" constraint
resolved so cleanly. It is not modified later either:
`ParticleSystem_UpdateParticles` (`0x088f635c`) contains no store to `+0x64` in
any form.

**And that means `oag_render::sparks` is already exactly right.** Its cap is
`1.0 * half`, and its doc comment calls that a substitution for an unknown
per-system factor. It is not a substitution - it is the recovered value. **The
comment should be corrected; the code should not change.** That is the second
time in this sweep that a recovered mechanism turned out to be already
reproduced.

**A trap this cost time on, recorded so it does not cost it again.** The first
attempt concluded `+0x64` was written by neither `InitParticle` nor
`UpdateParticles`, from an instruction search scoped by *function name*. The live
Ghidra database has **two functions both named `ParticleSystem_InitParticle`** -
`0x088f635c` (which the docs call `ParticleSystem_UpdateParticles`) and
`0x088f6e6c` (the real one) - so the name resolved to the wrong body and the
"searched both functions" claim was false. Re-run **by address** the negative does
hold for both real bodies, and the answer was one function further on. This is
the same duplicate-name hazard [mesh-draw.md](mesh-draw.md) records for the mesh
module: **in these two modules, scope instruction searches by address, never by
name.** The whole-program float-store sweep was also run; every particle-module
hit belongs to a different struct, so that route is genuinely exhausted.

### Checked against the consumer: none of this is a code change today

Done before recommending anything, and the answer is that it should **not** be
implemented yet. `oag_render::sparks` builds **one** four-corner quad per streak,
`centre ± dir*(length/2 + half) ± perp*half`, with `u`,`v` spanning `0..1` across
the whole thing. Against the original's eight vertices that is two differences:

1. **Cap length - resolved, and ours is correct.** `particle+0x64` is `1.0f`
   (above), so the original's cap is `1.0 * half` and so is ours. Not a
   substitution and not an approximation: the same value. Only the module's
   comment needs correcting.
2. **Where the falloff sits.** The original confines it to two fixed-size caps
   and stretches a single texture row over the body; ours spreads it over the
   entire streak. That would be a real visual difference **if we were sampling
   the authored sprite** - and we are not. `sparks.rs` binds no texture at all
   and computes a **procedural radial falloff from the UV** as a stand-in,
   because the authored sprite has not been decoded. Re-mapping our `v` to the
   original's `0 / 0.5 / 0.5 / 1` layout would therefore fit one approximation
   to the *coordinate convention* of a texture we do not sample - which buys
   nothing and would look arbitrary to the next reader.

**So the correct order is: decode the sprite texture first, then adopt the `v`
layout and the cap ratio together.** Adopting the `v` layout alone is not a
partial fix, it is a change with no defined meaning. Recorded here so the
sequencing survives.

The one thing that *is* worth carrying into the code now is a comment
correction: `sparks.rs`'s note that the original "extends the quad past both
points by a per-system stretch factor" is right, and can now name the field
(`particle+0x64`) and say that the same field is the mode-3 sprite's aspect
ratio.

### Applied names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x08916d00` | `ParticleSystem_DrawCappedStreak` | 88 |
| `0x08916610` | `ParticleSystem_DrawRotatedSprite` | 88 |
| `0x0891eaec` | `Gfx_AllocDrawScratch` | 82 |
| `0x088f79b4` | `ParticleSystem_InitParticleFields` | 88 |
| `0x0897e030` | `cosf` | 90 |
| `0x0897e300` | `sinf` | 90 |
| `0x08984d14` | `__kernel_cosf` | 88 |
| `0x08985680` | `__kernel_sinf` | 88 |
| `0x08984760` | `__rem_pio2f` | 82 |

`Gfx_AllocDrawScratch` (`0x0891eaec`) is named from its use rather than its
body - it is called with a byte count, returns a pointer or zero, and both
callers immediately write vertices into it - so 82 rather than 88. The two draw
functions keep no `_q` suffix because the geometry they build is a direct read.
`__rem_pio2f` (`0x08984760`) is 82 rather than 88 because it is identified by
position in the call structure - the shared reducer both kernels' callers branch
to for `|x| > pi/4` - rather than by reading its body.

### Two smaller facts from the same read

- **The near-plane cull is `-1.0`.** `DAT_08a88500` is `0xbf800000`. Modes 6 and
  7 test **both** endpoints and skip the particle if either has
  `view_z > -1.0`; modes 1, 2 and 3 test the single position. So a particle
  within one unit of the camera is dropped entirely rather than clipped, and a
  two-point streak is dropped if *either* end is too near - it never draws
  partially.
- **The mode-1/2 inline quad ignores `+0x64`.** It is built at
  `position.xy ± particle+0x30` with no aspect term, confirming this page's
  reading that the size channel is a view-space half-extent, and confirming that
  those two modes are square by construction.
