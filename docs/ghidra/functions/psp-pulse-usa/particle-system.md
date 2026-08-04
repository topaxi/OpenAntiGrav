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
│                 └─ ParticleSystem_AimedVelocity_q 0x088fc490 (velocity mode 1)
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
`ParticleSystem_AimedVelocity_q` (`0x088fc490`, **65**) adds the same
±cone jitter to authored yaw/pitch base angles at `res+0x50`/`+0x54`
(radians) - `_q` because the axis convention of the two angles was not
pinned down.

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
