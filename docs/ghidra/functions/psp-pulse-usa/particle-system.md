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
  under flag `0x4000000`, drawn from the instance's grid count `+0x164`
  (corrected 2026-09-24: `res+0x9ac` feeds a different per-particle field -
  see "The atlas frame advances"); world-transform of spawn offset
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
  advances by the frame-rate channel, wrapping at the grid's frame count
  (read in full in "The atlas frame advances").
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

Confidence **80**. **Corrected 2026-09-24**: the strip is a wedge, not a
quad, and the common depth is the farther endpoint's - see "The streak
strips' texture coordinates" below. Both view-space points are first scaled onto the nearer
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

## The collision sprite is identified and measured (2026-08-10)

The texture every collision-spark draw binds is now pinned live: with
breakpoints on all three draw helpers during a wall crash, every hit's
recorded stream ends in the same `TBP0`/`TSIZE0` pair - one **64x64
texture**, and PPSSPP's `SaveNewTextures` dump of it is unmistakably the
`orange_glow2.tga` the `.pob`'s authoring path names: a radial glow with a
white-hot core (`(253, 235, 220)` at the centre) tightening to saturated
orange by `r ≈ 0.15` and dimming linearly outward at constant hue. Radially
averaged measurements: alpha fits `(1-r)^0.92` with a plateau inside
`r < 0.06`; the texel colour itself also dims essentially linearly while
staying at saturation `0.98`. `oag_render::sparks`'s shader carries exactly
that split (measured constants, not the texels - ADR-0006), which is what
keeps a spark's bright part small: at the same alpha curve, a flat-rgb quad
reads about twice as wide.

One honest caveat survives from the same session: the identification ran
during a low-speed grinding state and every sampled draw went through the
**mode-3 billboard** helper - no streak-helper hit was observed, so which
draw class each collision emitter takes in practice rests on the
blend-table nibbles (`rmode 5 -> 0x60000019`, class 6; `rmode 6 ->
0x70000019`, class 7) and the earlier static reads, not on a live dispatch
trace.

**The second caveat is resolved, 2026-09-17.** The texture sitting in main
RAM with no WAD entry answering any hashable `Data\Psys\Tex\...` name was
never a loader mystery: `.pob` embeds the pixels itself.
`WO_SHIP_COLL_SPARK_DAMAGE.POB`'s own file bytes carry a 32-byte texture
header directly after each emitter's fixed-size record - no WAD lookup
needed at all - and the three bright emitters' headers all point at one
shared 64x64/4-level palette+pixel pool that decodes to exactly this
radial glow: white-hot core, saturated orange, the same measurements this
section records. `oag_vex::pob::texture` has the header layout and the
positional-addressing rule; `docs/formats/pob.md`'s "The sprite pixels are
on the disc after all" section has the corpus-wide evidence. This was the
"unlocated texture-reference gap" `pob.md` used to record - it is located.

## A particle is its sprite times its colour, and the Quake stretches its emitter (2026-09-24)

Chased because the Quake's crest and a close Rocket blast drew as white
blowouts where the original draws orange fire. Bloom (off: 1.9% of pixels
move, the blowout stays) and the blend space (the Pulse target is already
8-bit `Rgba8Unorm` in gamma space, ADR-0020) were ruled out first; what was
left is how the effect itself is played. Four reads, all static, the first
checked against the disc's own bytes:

### `Texture_BindEmbedded` (`0x08928b10`) - the sprite a particle draws with

Confidence **85**. `ParticleSystem_DrawParticle` calls it on the emitter's
texture block before every quad (two other callers, `FUN_08918bf8` and
`FUN_0891ee98`, are not particle code, hence the generic name). It returns
without binding when either pointer word (`+0x10` pixels, `+0x14` palette)
is zero; otherwise it enables texturing, sets the texture mode from the
depth byte (`8` bpp -> `T8`) with **bit 0 of `+0x06` as the swizzle
argument**, walks the mip chain into `Gu_TexImage` level by level, and loads
a 256-entry `8888` CLUT from the palette pointer. Both pointer words are
fixup sites in the `.pob`'s own slot table on all 64 PSP sprites (`pob.md`,
"Correction: the two texture offsets are from the resource base"), which is
what makes them pointers at draw time.

Nothing on the particle path sets a texture function, and the whole binary
has only two `TexFunc` emitters, both accounted for in
[mesh-draw.md](mesh-draw.md): the frame runs under `GU_TFX_MODULATE`,
`GU_TCC_RGBA`, colour doubling off. **So a particle's fragment is its sprite
texel times its colour, alpha included.** `WO_QUAKE`'s `fireballs` walk a
colour table from `(255, 250, 252)` to orange over their life, and their
sprite is an orange fire ring: the product is orange from birth. Drawn
over a white procedural disc - what `oag_render::psys` did before this
pass - the same table is white.

### The atlas frame: `FUN_088f58a4` and `ParticleSystem_InitParticle`

Confidence **85**. The instance init `FUN_088f58a4` splits `+0x9a0` into
the grid's two halves (instance `+0xa0`/`+0xa2`), stores their product at
`+0x164` as the frame count and the reciprocals at `+0xa4`/`+0xa8`.
`ParticleSystem_InitParticle` draws the particle's frame as
`Psys_RandIntRange(0, count - 1)` under flag `0x4000000` and `0` otherwise.
`ParticleSystem_DrawParticle` then applies `Gu_TexScale` by the reciprocals
and `Gu_TexOffset` by `frame % columns`, `frame / columns`. Not read: how
the frame advances over life (the frame-rate channel) - read since, in "The
atlas frame advances" below.

### `ParticleSystem_EmitLine` (`0x088fcfec`) - shape 1

Confidence **85**, read at instruction level. The emit function
`ParticleSystem_SpawnBurst` dispatches for shape 1. While `+0x3c` is `0..=2`
it writes the spawn offset `(U(-e, e), 0, U(-z, z), 1)`: `e` is the scaled
extent global `DAT_08ab2290` (`0x088fd15c`), `z` the resource's **unscaled**
`+0x40` (`0x088fd174`). Velocity then follows `+0x44` as for the other
shapes (`1` aimed, `0`/`2` cone). All three `WO_QUAKE` emitters are shape 1
with extent `50` and `+0x40 = 0`: a line.

### `ParticleSystem_SetScaleParams` (`0x088f44d8`) - what the Quake scales

Confidence **80**. Writes seven words from its argument into instance
`+0x28..+0x40`, re-runs `ParticleSystem_DeriveScaledParams`, sets bit
`0x10000` of `+0x160`, and recurses into the sibling and child instances at
`+0x1a8`/`+0x1a4`. Its partner at `0x088f443c` (not a Ghidra function; it
falls between two, so it is not named here) copies the same seven words
out. `Quake_Update` calls the pair around one store: `edge_distance / 50.0`
into slot 1 of the buffer (`sp+0x1a4`, `0x0891dc60`), which is instance
**`+0x2c`**. `ParticleSystem_DeriveScaledParams` multiplies `+0x2c` into the
three extent fields and nothing else - size and speed take `+0x28` and
`+0x34`. So the Quake stretches its line emitter to the road's width and
leaves every fireball its authored size. Also read on the way:
`Quake_Update`'s basis starts from `normalize(B - A)`, the edge-to-edge
direction, as its first row (`0x0891da08`) - the frame's `X`, along which
the line lies.

### Applied to Pulse on the PSP only, by choice

Everything in the two sections above is read off this binary. The port
applies the extent law - line and sphere placement, and the Quake's `/ 50`
as the extent co-factor - **only to Pulse off a PSP disc**; every other
source keeps spawning at the anchor with the `/ 50` as severity
(`oag_render::psys::Effect::without_extents`). That is a choice made
2026-09-24, not a finding: the PS2 ELF and HD's `EBOOT.elf` have not been
read, and a law unmeasured there should not change what they draw. A future
read starts from the PS2/HD counterparts of `ParticleSystem_SpawnBurst`
(`0x088f56c4`), `ParticleSystem_EmitLine` (`0x088fcfec`),
`ParticleSystem_EmitSphere` (`0x088fd340`), `Quake_Update` (`0x0891d268`)
and `ParticleSystem_SetScaleParams` (`0x088f44d8`). The sprite sampling is
not gated: the texture offsets are a format fact, and only PSP `.pob`s embed
sprites at all.

### `ScreenFlash_Start` (`0x088f00c0`) - the wash that is not a particle

Confidence **88** (was 72; the consumer is read and measured in
"The screen flash's consumer, read and measured" below). Called with a kind
and a world position by every weapon
detonation (`Rocket_SpawnCraftExplosion_q` passes kind `0`, `Quake_Update`
kind `4`, and eleven more callers). It arms a state block on
`DAT_08ab2200` - `+0x1a0 = 1`, the position at `+0x60` - and fills it from a
per-kind table: a duration at `+0x44`, two distances at `+0x4c`/`+0x50`,
and two RGBA keys at `+0x70` and `+0x80` for times `+0xb0 = 0` and
`+0xb4 = 1`:

| kind | caller | duration | distances | key at 0 | key at 1 |
| ---: | --- | ---: | --- | --- | --- |
| 0 | Rocket, craft hit | 0.5 s | 75, 200 | (1, 1, 0, 0.6) | (1, 0, 0, 0) |
| 4 | Quake | 0.4 s | 0, 300 | (1, 0.2, 0, 0.5) | (1, 0.2, 0, 0) |

The kind-0 key is the **full-screen yellow wash** the 2026-09-24 PPSSPP
capture of a craft-hit Rocket shows for its first frames (screen mean
`(89, 96, 84)` -> `(223, 225, 85)`: red and green up, blue untouched), and
kind 4 is the orange tint the Quake capture opens with. That match is why
the name clears 70. The per-frame consumer - the blend, the distance
falloff, the key interpolation - is read below, and `oag_render::flash`
draws every kind with a caller on Pulse's PSP source.

## Open, deliberately

- `FUN_088fc634` (shape 3, cone *placement*) and shapes 1/2/8 are unread;
  `oag_render::sparks` approximates shape 3 spawn position as the anchor
  (the authored extents are at most 0.1 units - sub-pixel at any race
  camera distance).
- Inside the draw layer: `FUN_08916d00` (the mode-7 two-point draw) and
  `FUN_08916610` (the mode-3 rotating billboard) are dispatched with known
  arguments but unread internally, and the exact stretch factor
  `ParticleSystem_DrawStreak` receives is not traced back to a resource
  field. (All three closed since: see "The two unread draw modes, read" and
  "Streaks, atlas advance and the screen flash".)
- Modifier types other than 3 and the
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

> **Corrected 2026-09-30.** `1.0f` is only the *initial* store. The per-tick
> field update `ParticleSystem_UpdateParticleFields` (`0x088f7e64`, below)
> overwrites it every tick from the resource's `+0xf0` block, and a live
> struck craft read `shazam` at `1.5` and `glow` at `1.7`. The search this
> section ran for a store to `+0x64` covered `ParticleSystem_UpdateParticles`
> and the initialiser, and missed the routine both of them call for the field
> sampling. Whether an *emitter's* particles ever get a value other than `1.0`
> is unread - see "Open" at the end of the new section.

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

## Streaks, atlas advance and the screen flash (2026-09-24)

Three draw-side reads, taken because a Rocket's craft hit and a struck
craft's hull sparks both read far weaker in `oag_render` than in the
original. All three are static reads of this binary; the screen flash is
also measured against an existing PPSSPP capture. All three are applied to
Pulse off a PSP disc only, the same line the extent law draws.

### The streak strips' texture coordinates

`ParticleSystem_DrawParticle` calls both strip builders as
`(half, stretch, &second_point, &position, colour)`: at `0x08918b2c` the
first pointer is `sp+0x50`, loaded at `0x089187ec` from the particle's
`+0x10` (the spawn or last-tick point), and the second is `sp+0x40` from its
`+0x00` (the position), set at `0x08918ad8`. Both endpoints are view space.

**`ParticleSystem_DrawStreak` (`0x08916820`), class 6, is a wedge, not a
rectangle.** Confidence **88**, read at instruction level. With `s0` the
second point and `s1` the position, `dir = normalize(s0 - s1)` (`(0, 1, 0)`
when zero), `perp = (dir.y, -dir.x, 0) * half` and `cap = dir * stretch *
half`. Four strip vertices, `Gu_DrawArray(4, 0x19f, 4, ...)`:

| # | position | `u` | `v` | written at |
| --- | --- | --- | --- | --- |
| 0 | `position - cap - perp` | 1 | 0 | `0x08916aec` |
| 1 | `position - cap + perp` | 0 | 0 | `0x08916b80` |
| 2 | `position + cap - perp` | 1 | 1 | `0x08916c14` |
| 3 | `second + cap + perp` | 0 | 1 | `0x08916ca8` |

Vertex 2 loads `lv.q C200, 0x0(s1)` at `0x08916ba0`: the **position**, not
the second point, where vertex 3 loads `s0` at `0x08916c34`. So the strip is
a sprite-sized head at the particle and a sliver out to the second point,
and the sliver samples the half of the sprite on the `u < v` side of its
diagonal. A zero-length streak is a plain square.

**A correction to the section above**: both points are scaled onto
`min(z)` (`c.lt.s` at `0x08916880` picks the smaller of the two view `z`),
and since a visible point has `z < -1`, that is the **farther** endpoint,
not the nearer one. `ParticleSystem_DrawCappedStreak` uses the mean.

`ParticleSystem_DrawCappedStreak` (`0x08916d00`, class 7) was re-read and
matches the table in "The two unread draw modes, read" exactly, with
`a` the position and `b` the second point.

**Implemented** as `oag_render::psys::streak`. Class 7's eight vertices are
drawn as one quad whose along-coordinate `psys.wgsl` folds back into the
`0 / 0.5 / 0.5 / 1` `v`, which is exact because `v` is linear along each of
the three spans. **Chosen, not measured**: the strip is built in world
space with `perp = dir x (right x up)`, which equals the view-space
`(dir.y, -dir.x)` for a streak in the screen plane, and each end keeps its
own depth instead of the common one.

### The atlas frame advances

Confidence **85**.

- **The switch.** `ParticleSystem_CacheModeFlags` (`0x088f4dcc`) sets
  `DAT_08b6206e` when `res+0x788 > 0` and the instance's frame count
  (`+0x164`, columns times rows) is not 1. `+0x788` is `+0x10`, the `hi`, of
  a channel block at `+0x778`: the fourth of the per-particle blocks
  (`+0x4d8` size, `+0x5b8` alpha, `+0x698` roll, `+0x778` frame rate, all
  `0xe0` apart).
- **The step.** `ParticleSystem_UpdateParticles` (`0x088f635c`), under that
  flag, keeps a float frame at particle `+0x98`:
  - under resource flag `0x40`, `age * (frames - 0.01 - 0.01)`;
  - otherwise `+= channel[3] * DAT_08b62080`, where
    `ParticleSystem_Update` stores `DAT_08b62080` as the instance's tick
    count times its playback rate at `0x088f5cb8`.
  - Once it reaches `frames - 0.01` it loses that amount, once. The drawn
    frame at `+0x78` is its floor.
- **The seed.** `ParticleSystem_InitParticle` seeds `+0x98` with the random
  start frame, or 0.
- **The bake.** `channel[3]` comes from the load-time bake: `FUN_088f3b68`
  calls `FUN_088f9024(res+0x9f0, alpha, size, roll, frame_rate)`.
  - `FUN_088f9024` merges the four blocks' key times into one sorted,
    de-duplicated timeline at `+0x9f0`.
  - It evaluates each channel there through `FUN_088f8ec0` and writes the
    segment rates at `+0xa70` and the start vector at `+0xc70`.
  - It zeroes a random-mode channel, which is why a random frame rate never
    moves: nothing at spawn samples it either.
  - It gives up on every channel of the emitter when any keyframed one
    has a period.
- **What authors it.** On the PSP disc: every 4x4 smoke and fire emitter of
  `WO_ROCKET_EXPLO`, `_EXPLO_TRACK`, `WO_SHIP_EXPLOSION` and
  `WO_SHIP_FXNODE_EXPLO`, plus the 2x2 Shuriken head, trail and rings and
  `WO_PLASMA_HEAD`. None of the hull-spark emitters authors a grid.

**`+0x9ac` is not the frame count.** `ParticleSystem_InitParticle` ORs
`Psys_RandIntRange(1, +0x9ac) << 4` into a per-particle flag byte when it
is above 1. What reads those bits is not traced.

**Implemented** as `oag_render::psys::frames`, and the channel is parsed as
`oag_vex::pob::Emitter::frame_rate`.

### The screen flash's consumer, read and measured (2026-09-24)

`DAT_08ab2200` points at a `0x1b0`-byte scene node. `InGame_Update` allocates
it and zeroes it at `0x08813d6c`/`0x08813d80`, then calls
`ScreenFlash_Construct` (`0x088ef234`), which installs the vtable
`0x08ad0df4` and sets the key count at `+0xd0` (and its copy at `+0x170`)
to 4. Three of the vtable's slots are its own:

- `0x088ef398`, a two-instruction store of the frame's `dt` into `+0x180`.
  It is not a Ghidra function, so it is not named here.
- `ScreenFlash_Update` (`0x088ef3a4`).
- `ScreenFlash_Draw` (`0x088efb1c`).

`ScreenFlash_Destroy` (`0x088ef334`) clears `DAT_08ab2200`.

**`ScreenFlash_Start` fills a pending block; `ScreenFlash_Update` decides.**
Confidence **88** for all three functions, each read in full:

1. **The pending check.** If `+0x1a0` is set, Update clears it, evaluates the
   pending keys at `t = 0`, and weighs them by the distance falloff. It then
   compares `(alpha * falloff)^2 * |rgb|^2` with the same product of the
   colour it last drew (`+0x190..+0x19c`, stale when nothing runs). Only a
   stronger one is copied over the running block (`+0x40..+0xdc` to
   `+0xe0..+0x17c`); that restarts the clock at `+0x184` and raises `+0x1a1`.
   A weaker flash is dropped.
2. **The running flash.** `+0x184 += dt` unless `DAT_08ab0628` (pause) is set,
   then `t = elapsed / duration`. At `t >= 1` it stops. Otherwise:
   - the keys are interpolated linearly over the times at `+0x150`. Every
     kind but 7 authors times 0 and 1, so this is one lerp;
   - the alpha is multiplied by the falloff, re-measured this frame;
   - the node is queued at layer `0x4f000000` unless `g_display+0x5dec` is
     set.
3. **`ScreenFlash_DistanceFalloff` (`0x088effb8`).** It returns 1 when `far`
   (`+0x50`) is not above zero; otherwise
   `clamp(1 - (d - near) / (far - near), 0, 1)`. `d` is the distance from
   `-(camera + 0x70)`, the eye as [camera.md](camera.md) reads it, to the
   flash's position.
4. **`ScreenFlash_Draw`.** Identity projection, view and model matrices. The
   glow mask is protected (`Bloom_SetGlowMaskWritable(g_bloom, 0)`). Depth
   test, texturing, culling and lighting are disabled, and blending enabled.
   `BlendFunc(ADD, SRC_ALPHA, FIX 0xffffff)`, additive, when `+0xe9`, the
   running copy of `+0x49`, is set; alpha-over otherwise. `Gu_Color` takes
   the colour times 255, truncated to bytes, and one untextured triangle
   strip covers the screen. The strip is `DAT_08ab2210`: `(+-1, +-1, -0)`,
   vertex type `0x180`.

**The full kind table**, read off `ScreenFlash_Start`. Distances are in world
units. `+0x48` is copied but never read by Update or Draw.

| kind | duration | near, far | key at 0 | key at 1 | blend |
| ---: | ---: | --- | --- | --- | --- |
| 0 | 0.5 | 75, 200 | (1, 1, 0, 0.6) | (1, 0, 0, 0) | additive |
| 1 | 1.25 | 100, 300 | (0.7, 0.1, 1, 0.7) | (0, 0, 1, 0) | additive |
| 2 | 0.4 | 100, 300 | (0, 0.2, 1, 0.5) | (0, 0.2, 1, 0) | additive |
| 3 | 0.75 | 100, 300 | (1, 0.5, 0, 0.7) | (1, 0, 0, 0) | additive |
| 4 | 0.4 | 0, 300 | (1, 0.2, 0, 0.5) | (1, 0.2, 0, 0) | additive |
| 5 | 0.4 | none | (1, 0.7, 0, 0.5) | (0.3, 0, 0, 0) | additive |
| 6 | 1.0 | none | (1, 1, 1, 1) | (0, 0, 0, 0) | additive |
| 7 | 2.0 | none | (1, 1, 1, 1) | (1, 1, 0, 0.8) at 0.1, (1, 0, 0, 0) at 1 | additive |
| 8 | 0.4 | 50, 200 | (1, 1, 0, 0.4) | (0, 0, 0, 0) | additive |
| 9 | 2.0 | none | (1, 1, 1, 1) | (0, 0, 0, 0) | additive |
| 10 | 0.7 | none | (1, 1, 1, 1) | (0, 0, 0, 0) | additive |
| 11 | 0.5 | none | (0.8, 0.8, 0.8, 1) | (0.01, 0.01, 0.01, 0.01) | alpha-over |

All fifteen callers are read in [screen-flash-callers.md](screen-flash-callers.md)
(2026-09-30): which weapon or craft event passes each kind and what gates it. Kinds
5 and 11 have no caller. The Quake calls `ScreenFlash_Start(4, A)` **every frame**
its wave's instance exists (`0x0891dc7c`, inside the per-span loop). `A` is the first
of `Quake_SampleSpan`'s two edge points, which makes the replacement rule
what keeps the tint up. Kind 7 authors three keys, not two - white, yellow at
`t = 0.1`, red to nothing - which the table above cannot show in one row.

**Measured.** The PPSSPP capture of a craft-hit Rocket 51 units from the eye
is `data/scratch/fx-brightness/ppsspp-rocket/scenarioB`, gitignored. Kind 0
predicts:

- nothing added to blue;
- `153 (1 - t)` added to red and `153 (1 - t)^2` to green, since the
  falloff is 1 inside 75 units;
- so green over red is `1 - t`.

Measured over dark pixels (below 120) in two track regions away from the
fireball, against the pre-flash frame:

| frame `k` | R added | G added | B added | G/R | `t` from G/R | `153 (1 - t)` |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 3 | 147-150 | 145-146 | 0 | 0.98 | 0.02 | 150 |
| 6 | 134-136 | 118 | 0 | 0.87-0.88 | 0.13 | 134 |
| 12 | 100-102 | 64-66 | 0 | 0.64 | 0.36 | 98 |
| 20 | 63-66 | 25-27 | 0-1 | 0.40 | 0.60 | 61 |
| 25 | 32 | 7 | 0 | 0.21-0.22 | 0.79 | 32 |

`t` steps by 0.031-0.035 a frame, which is `(1/60) / 0.5`: a 60 Hz `dt`
against the 0.5 s duration. Red follows `153 (1 - t)` to within 10 at every
frame; the frames around the fireball's peak (`k` 8-16) run a few high,
which is its own light. Blue never moves. That confirms the additive blend, the lerp, the
`0.6` alpha and the timebase together.

The HUD's cyan bar gains only 16 red under the full wash, against 150 on the
track. So the HUD draws over the flash, as `oag_game` composites it.

**Implemented** as `oag_render::flash`. It is drawn last in the scene pass,
before the bloom, colour-only. Our own run of the same shot adds
`141 -> 24` red over ticks 18 to 42 with the same green-over-red slope, `t`
stepping 1/30 a tick.

### Applied names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x088ef234` | `ScreenFlash_Construct` | 80 |
| `0x088ef334` | `ScreenFlash_Destroy` | 80 |
| `0x088ef3a4` | `ScreenFlash_Update` | 88 |
| `0x088efb1c` | `ScreenFlash_Draw` | 88 |
| `0x088effb8` | `ScreenFlash_DistanceFalloff` | 88 |
| `0x088f9024` | `ParticleSystem_BakeChannels` | 82 |
| `0x088f8ec0` | `ParticleSystem_EvalChannel` | 85 |
| `0x088f3b68` | `ParticleSystem_PrepareResource_q` | 65 |

`ScreenFlash_Start` (`0x088f00c0`) moves from 72 to **88**: its consumer is
read and the kind-0 prediction matches the capture frame by frame.
`ScreenFlash_Construct` and `_Destroy` stay at 80 because they are named from
the vtable they install and the global they set and clear, not from a
behaviour checked against anything. `ParticleSystem_BakeChannels` is 82: the
merge, the evaluation and the rate division are read, and the baked values
match the live read of `+0x9f0`/`+0xc70` in "The three live captures", but
the segment walk was not stepped. `ParticleSystem_PrepareResource_q` bakes the
channels, prepares the textures, registers the atlas grid in a global list
and recurses into the child, death and sibling records; the list's reader is
not traced, hence the `_q`.

### The instance initialiser and the sprite templates (2026-09-30)

`FUN_088f58a4` (`0x088f58a4`, **`ParticleSystem_InitInstance`**, confidence 80)
is what turns a resource into a live emitter instance: it copies the matrix,
sets `dt`-to-ticks, splits `+0x9a0` into the atlas grid, calls
`ParticleSystem_DeriveScaledParams`, recurses into the `+0x94c` sibling, and -
the part nothing had read - **makes one particle per sprite template** in the
list at `+0x9a8`, gated on `+0x9a4`. The record layout, the corpus and what is
played are in [pob.md](../../../formats/pob.md), "The sprite templates at
`+0x9a4`/`+0x9a8`". Severity multiplies a template's size like any particle's
(`shazam` `3.9 * 2.4 = 9.36`, live).

`ParticleSystem_DrawParticle` (`0x089186bc`) reads a template's render mode at
record `+0x874` and its blend class at `+0x878`, and binds the record's own
texture words at `+0x890`.

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x088f58a4` | `ParticleSystem_InitInstance` | 80 |

### Releasing an instance by handle (2026-09-30)

What the original does with the handle it is given when an effect's owner is
done with it. Read from disassembly and decompiles, all five functions
directly; no live capture.

`FUN_088f3298(manager, handle, now)` (`Psys_ReleaseHandle`) resolves the
handle through `Psys_LookupHandle` (`0x088f24d0`: slot `handle % capacity` of
the table at `manager+0x4690`, records of `0xc` bytes, valid only when the
stored id equals the handle, else `0`) and then does one of two things to the
instance:

- **`now == 0`** (every caller this page follows: `LeachBeam_Advance`'s
  `WO_LEACHBEAM_ENERGY` re-spawn passes `0`): sets bit `8` of `+0x160`. That
  is the same "dead" flag `ParticleSystem_Update` sets on an instance that
  has run out. Nothing in `ParticleSystem_Update` tests it on the way in, so
  the instance carries on until the manager's next tick: `FUN_088f3510`
  calls `Psys_ReapDead` (`0x088f32f8`), which walks the active list
  (`manager+0x48`), unlinks every instance with bit `8` set and passes it to
  `ParticleSystem_Destroy` (`0x088f42b8`).
- **`now != 0`**: `ParticleSystem_StopAndClear` (`0x088f4790`) zeroes the
  remaining duration (`+0x138`), clears `+0x160` bits `1` and `0x10`
  (emitting, and the looping mode), frees the **template** list at `+0x1b0`
  (`FUN_088f48c0`) and recurses into the death and sibling instances
  (`+0x1a8`, `+0x1a4`). The instance then reports itself dead (bit `8`) on the
  first `ParticleSystem_Update` after its pool count (`+0x140`), its child
  instances and its template list (`+0x1b0`) are all empty and the duration
  (`+0x138`) is spent - the tail of `0x088f5b9c`, read 2026-09-30 - so the
  emitters' own particles live out their lives first.

`ParticleSystem_Destroy` releases the handle (`FUN_088f2460`), frees the
**template list** (`FUN_088f7e38` on each entry of `+0x1b0`) and the pool (`FUN_088f45a0`), destroys the
death, sibling and per-particle-child instances, and returns the instance to
the pool (`FUN_08946d00`).

**So the release is a kill, not a detach.** Bit `8` takes the instance and
every particle it still holds out at the next manager tick; nothing lets the
particles finish. The node that owns the effect (`FUN_08915cdc`) follows the
same contract from the other side: when `Psys_LookupHandle` returns `0` for
its handle it destroys itself. Confidence **82** for the kill (every link is a
direct read; the manager tick's ordering against the draw was not checked, so
a frame of the old particles may draw first).

**Which callers take which arm** (every caller of `Psys_ReleaseHandle`, read
2026-09-30 from the decompiles of all ten):

| Caller | `now` | Releases |
| --- | --- | --- |
| `LeachBeam_Advance` (`0x08873fa0`), at the pulse block's `WO_LEACHBEAM_ENERGY` re-spawn | `0` | the previous ENERGY instance |
| `FUN_08872e64` (the teardown `LeachBeam_UpdatePool` calls on retire) | `0` | the ENERGY instance |
| `RocketPool_Update`, `MissilePool_Update` (two handles), `Plasmas_Update`, `Quake_Update` | `1` | the projectile's flares, the quake's effect, when it ends |
| `FUN_0883d664`, `FUN_0883f540`, `FUN_08875658`, `FUN_08877210` | `1` | per-craft and per-weapon handles, not matched to a port site |

**The two arms are not the same as a detach, and neither is a detach.** The
page above had `now != 0` zeroing "the live particle list"; that list (`+0x1b0`)
is the instance's **template** list, not its pool (see "An emitter's own
particles"), so the `now != 0` arm stops the emitters and frees the templates
and leaves every emitter particle to live out its life. `now == 0` destroys
the lot (`ParticleSystem_Destroy` walks the pool at `+0x74` in `FUN_088f45a0` as well, unread past its loop header).
In `oag_render::psys::Stage`: `now == 0` is **`kill`** (the ENERGY, both
sites), `now != 0` is **`release`** (the rocket, missile and plasma flares and
the quake), and `detach` - nothing freed early - is for an instance whose
emitters ran out on their own, which no caller here releases. Confidence
**82**; the one **chosen, not measured** part is that `kill` empties the
instance in the same tick, where the original lets one more draw through if the
draw runs before the manager's tick.

**It matters more than it looks, because templates live long.** The missile's
`WO_MISSILE_HEAD` carries a `redbar` (6000 ticks) and a `glow` (3600, 34 units),
the plasma head's `WO_PLASMA_HEAD` three (up to 65535). With the port's old
detach they outlived the projectile by a minute or more, **and** they were
never moved: `ParticleSystem_UpdateParticleFields` copies the owning instance's
node position (`instance + 0x120`) into a template every update (its
`FUN_088f24d0` lookup, the same handle resolution), so a flare's `glow` rides
the rocket, where the port spawned it once at the muzzle and left it there.
Both are fixed the same day (`psys::template::ride`, `Stage::release`), and
pinned on the real `WO_MISSILE_HEAD` in `psys_emitter_roll_ground_truth`.

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x088f24d0` | `Psys_LookupHandle` | 85 |
| `0x088f3298` | `Psys_ReleaseHandle` | 82 |
| `0x088f32f8` | `Psys_ReapDead` | 82 |
| `0x088f42b8` | `ParticleSystem_Destroy` | 82 |
| `0x088f4790` | `ParticleSystem_StopAndClear` | 78 |

### The per-tick field update, and what `particle+0x64` is (2026-09-30)

`FUN_088f7e64` (**`ParticleSystem_UpdateParticleFields`**, `0x088f7e64`, confidence
**85**) is the routine that samples a particle's channels. `ParticleSystem_InitParticleFields`
calls it once with `dt = 0` as its last act, and the particle update calls it
with the tick's `dt` for every live particle, *before* it takes the tick off the
particle's life. So the state a frame draws was sampled at the age the particle
had going **into** that frame: a particle born during the game logic is drawn
at age 0 that frame, and at `1 / life` the next. (Live: `glow` half-size
`0.75` then `3.18`; `shazam` `9.36` twice.) Per call, off the resource record
(`res`, the derived layout every template is stored in):

| Field written | Offset | From |
| --- | --- | --- |
| drawn half-size | `+0x30` | `res+0x10` block at the normalised age, times the instance severity |
| colour RGB | `+0x34..0x36` | `res+0x470` table at `age * 255.999`, unless `res+0x870 == 2` |
| colour alpha | `+0x37` | `res+0x1d0` block |
| atlas frame | `+0x58` | the `res+0x2b0` block, accumulated (only when the grid has more than one cell) |
| roll | `+0x5c` | the `res+0x390` block; see below |
| **aspect** | **`+0x64`** | **the `res+0xf0` block**: `v > 0` stores `1 + v`; `v <= 0` multiplies the size by `1 - v` and stores `1 / (1 - v)` |

**The roll law.** `res+0x394 == 3` (random mode): the angle advances by the
rate chosen at spawn. Otherwise, with `res+0x884 & 0x20` **set** the channel's
value times `dt` *is* the angle this tick (an absolute keyed angle), and
**clear** it is a rate that accumulates into the angle, the other way for a
particle whose spawn-time coin (`res+0x884 & 0x08`) came up odd. Flag `0x10`
starts the angle at `Psys_RandFloatRange(0, 2 pi)`. The draw is
`ParticleSystem_DrawRotatedSprite`: half-height `size`, half-width
`aspect * size`, turned by the roll.

**Measured on a struck craft** (PPSSPP, 2026-09-30, `ParticleSystem_DrawParticle`
log, locators 0, 2 and 5, frame by frame):

| Particle | Template block | Drawn |
| --- | --- | --- |
| `shazam` | `+0xf0` constant `0.5`, roll `0..0`, flags `0` | aspect `1.500`, roll `0.000` |
| `glow` | `+0xf0` constant `0.7`, roll keyed `0..2 pi` over nine keys, flags `0x30` | aspect `1.700`, roll `6.28, 5.60, 4.81, 4.13, 3.35, 2.65, 1.91, 1.24` at ticks 0..7 |

The glow's angles are the roll channel's keys at `tick / 40` times `2 pi`
(`1.0` falling to `0.192` by key `0.174`, so `5.55` at tick 1 against the
`5.60` read on a variable timestep).

**Why it matters.** Both templates are drawn **stretched**: the `glow` is 1.7
times wider than tall and turning, the `shazam` 1.5 times. A square quad of the
same size shows 0.59 and 0.67 of the area, which is the brightness the port was
missing once the camera and the pick sequence were matched. Every one of the
31 PSP templates draws as class 3 and carries a `+0xf0` block.

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x088f7e64` | `ParticleSystem_UpdateParticleFields` | 85 |

**An emitter's own particles are not this routine's.** 45 of the 76 PSP
emitters draw as class 3 too, 23 of them author a roll, and they go through
neither `ParticleSystem_UpdateParticleFields` (its only callers are
`ParticleSystem_InitParticleFields` and `ParticleSystem_Update`, both for
templates) nor `ParticleSystem_DrawParticle`. See the next section.

### An emitter's own particles: the batched draw and the roll law (2026-09-30)

Two kinds of particle live in an instance and the page above had them folded
into one. A **template** is a `0x90`-byte record in the list at instance
`+0x1b0`, linked by `+0x78`, sampled by `ParticleSystem_UpdateParticleFields`
and drawn one at a time by `ParticleSystem_DrawParticle`. An **emitter's own
particle** is a slot of the pool at instance `+0x74` (`0xa0` bytes a slot, the
particle pointer is `slot + 0x10`, and every `particle+` offset below is from it), integrated by `ParticleSystem_UpdateParticles` and
drawn a whole instance at a time. The offsets are the same layout shifted by
`0x40` (template `+0x30` size, `+0x34` colour, `+0x4c` life; pool `+0x70`,
`+0x74`, `+0x8c`), which is how a read of one was mistaken for the other.

| Address | Name | What |
| --- | --- | --- |
| `0x089177e4` | `ParticleSystem_DrawInstanceTree` | recurses the child chain (`+0x1ac`, sibling `+0x18`), then draws the instance's pool and, in the order flag `0x400000` picks, walks the `+0x1b0` template list into `ParticleSystem_DrawParticle` |
| `0x08918bf8` | `ParticleSystem_DrawEmitterPool` | pushes the matrix, binds the sprite, applies the blend class (`res+0xc0`), then **switches on `res+0xb8`**: index `0` and `1` to `FUN_089194d0`, `2` (class 3) to `ParticleSystem_DrawRolledQuads`, `5` (class 6) to `FUN_08917c7c`, `6` (class 7) to `FUN_08918160`, and every other index draws nothing. Only when the atlas grid is at most 16 cells: a larger grid draws no particle here at all |
| `0x089178c0` | `ParticleSystem_DrawRolledQuads` | the class 3 batch: one quad a live particle, six vertices each, one `Gu_DrawArray` |

**`ParticleSystem_DrawRolledQuads`.** `h` is the particle's size (`particle+0x70`
as a float), `w = h * *(float *)(res + 0x4c8)`, and the roll is `particle+0x50`,
turned to radians by the VFPU's `2/pi`. The four vertices are built in
registers as `C700 = (-w, -h, w, h)` times `(cos, -sin, cos, -sin)` and
`(sin, cos, sin, cos)`, added to the view-space position, so the quad's two
half-edges are

```
a = (w cos, -h sin)      b = (w sin, h cos)       vertices  -a-b, -a+b, a-b, a+b
```

which is the turned *unit* square with the aspect applied to the screen's `x`
afterwards. **It is a parallelogram, not the template's rectangle, whenever
`w != h` and the roll is not a multiple of a quarter turn.** At `w = h` it is
the same rotated square `ParticleSystem_DrawRotatedSprite` draws, with the same
sense: `a` is that routine's `axis_u` direction. Confidence **80**: read in
full in the decompile and the disassembly, **not** measured live.

**`+0x4c8` is a constant per emitter record, and the corpus authors it.** Read
on all 76 PSP emitters (`pob_emitter_roll_ground_truth`): `1.0` on 43 of the 45
class 3 emitters; **`4.0` on `WO_SHURIKEN_HEAD` and `WO_SHURIKEN_TRAIL`**, which
have no roll; and `3` on the collision sparks' class 6 streaks (and
`WO_CANNON_SPARKS`'s `thin_streaks`, `WO_ROCKET_EXPLO_TRACK`'s `fat_streaks`),
`2` on the class 6 and 7 of the missile bounce, rain, Shuriken bounce and expiry
and the absorb, `0.05` on `WO_REPULSER`. The class 6 and 7 routines are not
read here; that the same word is a streak's stretch is the obvious reading and
it is **unread**.

**The roll, off `ParticleSystem_InitParticle` and `ParticleSystem_UpdateParticles`**
(particle offsets, from the pointer `UpdateParticles` walks, `0x10` past the slot; the first reads are the `0x88f6e6c` decompile's, confidence
**80**, static):

| What | Law |
| --- | --- |
| start angle, `particle+0x50` | `Psys_RandFloatRange(-pi, pi)` under emitter flag `0x4`, else `0`; unless the class is 6 or 7, whose `+0x50` is the streak's other end |
| the coin | flag `0x8` adds `2` to the byte at `particle+0x81` when `FUN_088f8e18` says so (bit 1 set: flipped) |
| rate, `particle+0x9c` | the `res+0x698` channel: mode `0` (keyframed) leaves `0`; mode `2` its constant (the `hi` word, `+0x6a8`); mode `3` `Psys_RandFloatRange(lo, hi)` **once, here**; all negated when the coin bit is set |
| per tick, `DAT_08b6206c` (class 3 only) | channel not keyframed (`DAT_08b6206a` clear): `roll += rate * dt`. Keyframed: `roll -= v * dt` when the coin bit is **clear** and `roll += v * dt` when it is **set**, `v` the baked rotation component of the channel vector **before** this tick's integration - the value at the age the tick began with |

`dt` is the instance's tick count (`instance+0xc`, the same `dt` velocity is
multiplied by). **There is no absolute mode for an emitter** and no aspect
channel; the keyframed sign being the reverse of the constant one is the
decompile's reading and is **unmeasured**: the four keyframed class 3 emitters
on the disc are `WO_LEACHBEAM_CHARGING`'s `RINGS`, `WO_MISSILE_EXPLO`'s
`drift_down`, `WO_MODESTO_STEAM_A` and `WO_QUAKE`'s `debris`, and only the last
pair's flags decide which way they turn without a coin. The port plays all of
the above in `oag_render::psys::roll`.

**Corrects the section above.** `FUN_089177e4` walks the template list, not the
pool, so "the `+0x64` of an emitter's particles is `1.0`" was never a statement
about them: an emitter's particle has no `+0x64` (its aspect is `res+0x4c8`).

**Still open.** Whether an emitter's first draw is at age 0 as a template's is
(unmeasured, not played); which of `ParticleSystem_Update`'s orderings puts a
pool particle's spawn before or after its first integration; the class 6 and 7
batch routines' use of `+0x4c8`; and a live read of `particle+0x50` over a few ticks
on a flipped and an unflipped keyframed particle, which would make the sign
measured.

### Applied names (the batched draw)

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x089177e4` | `ParticleSystem_DrawInstanceTree` | 80 |
| `0x08918bf8` | `ParticleSystem_DrawEmitterPool` | 80 |
| `0x089178c0` | `ParticleSystem_DrawRolledQuads` | 85 |
| `0x089194d0` | `ParticleSystem_DrawPoolSquares` | 88 |
| `0x088fc634` | `ParticleSystem_EmitRing` | 85 |

### Shape 3 is a ring or a disc: `ParticleSystem_EmitRing` (2026-10-01)

`FUN_088fc634(instance, count)`, `ParticleSystem_SpawnBurst`'s shape-3 emitter (the
"cone placement" this page listed as unread), decompiled whole - the first read of it
stopped inside mode 2's loop and the rest was read after. With `r` the scaled extent
(`DAT_08ab2290`) and `phi` drawn `Psys_RandFloatRange(0, 2 pi)` per particle (or, under
resource flag `0x200000`, a random start stepped by `2 pi / count`), the spawn offset in the
emitter frame is, by `+0x3c`:

| `+0x3c` | offset `(x, y, z)` |
| ---: | --- |
| `0` | `(r cos phi, 0, r sin phi)` - a **ring** of radius `r` in the frame's `XZ` plane, `Y` the cone's axis |
| `1` | the same at a radius `Psys_RandSpread(r, instance+0x60)` |
| `2` | `(x, 0, z)` with `x` and `z` each `U(-r, r)`, **redrawn until `x^2 + z^2 <= r^2`**: a uniform **disc** |

What follows the placement, in the same function: `+0x44` (the emitter's velocity mode)
`1` calls `ParticleSystem_AimedVelocity` with the **normalised `(x, 0, z)` it just wrote**
as the heading - the particle leaves the ring outward - and `0` or `2` call
`ParticleSystem_ConeVelocity` independent of the offset; then `ParticleSystem_InitParticle`;
then, under the sub-frame flag (`DAT_08b620a0`), the offset and the two stored points are
pushed forward by the emitter's motion times `i / count`. Every shape-3 emitter with an
extent authors velocity mode 1.

**The extent is animated.** `WO_BOMB_SMOKERING`'s emitter carries one animated-attribute
record (`+0x93c` count 1, selector `2`, a keyed channel `1 + 1 * (0.0068 .. 1)`), which
`ParticleSystem_Update` evaluates at the emitter's normalised age every tick and stores into
instance `+0x48`, the extent's co-factor, before `ParticleSystem_DeriveScaledParams` runs
again (`oag_vex::pob::attribute`). So the ring is `12.94` at the first tick and `23.3` at
the sixteenth.

**Live, two boots** (a Bomb moved 120 units ahead and run out,
`psp-weapon-pair.py --probe rolled --detonate-bomb-at`; the second boot read 13.2, 13.8, 15.1,
17.6 and, at tick 15, 22.7): the particles born at emitter ticks
0, 1, 3, 7 and 16 sat **13.1, 13.7, 15.0, 17.6 and 23.4** units from the blast centre on the
horizontal plane, a vertical offset under a unit, every azimuth; the formula gives 13.0,
13.7, 15.0, 17.5 and 23.3, and the instance's own scale words read `1.0`. Each drifted
outward about 0.09 units a tick with `y` flat. (The first reading of this section took the
ring for a constant `12.94`; the log it was written from said otherwise.) Confidence **88**
for the decompile and the extent's growth (read, and matched at five ticks on two boots);
the disc's loop is read and not measured; the aimed azimuth's sign is unmeasured.

The corpus's shape-3 emitters with an extent over 0.1: `WO_BOMB_SMOKERING` (12.9, mode 1),
`WO_SHIP_EXPLOSION`'s root (12.9, mode 1), `SHIP_DEBRIS` (10, mode 2) and `trail` (1.9,
mode 1, velocity mode 0), `WO_ROCKET_EXPLO`'s `DEBRIS` (5.1, mode 2), the missile's
`drift_down` (4.1 to 4.3, mode 2) and `WO_REPULSER_BLAST` (13.6, mode 0, flag `0x200000`,
selector 5 animated). The animated attributes on the disc: eleven records on eleven
emitters, ten of them selector 2 (`WO_BOMB_SMOKERING` and its `debris`,
`WO_SHIP_EXPLOSION` and its `FIREBALL`, `WO_ROCKET_EXPLO`'s two mushrooms and
`WO_ROCKET_EXPLO_TRACK`'s `Fire_Emitter`, the Shuriken's bounce and expiry and the absorb).
`oag_render::psys::spawn::Spawn::Ring`, `place` and `EmitterSpec::extent_animation` play
all of it but the even step, the sub-frame spread, the azimuth's sign and selector 5. The
"approximates shape 3 as the anchor" remark further down was true of the collision sparks'
`0.1` and was never true of these.

### The pool's square draw, and a pool particle read live (2026-10-01)

`ParticleSystem_DrawEmitterPool`'s render-mode indices `0` and `1` go to
`FUN_089194d0`, now `ParticleSystem_DrawPoolSquares` (confidence **88**: read in
full, and its per-particle reads were then logged live, below). `a0` is the
instance, `a1` the view matrix, `a2` the per-frame UV table. For every live pool
particle (`slot + 0x10`, a bitmask word at the pool's head, a next-block pointer
at `+0x1410`):

| Particle field | Use |
| --- | --- |
| `+0x40` | world position, through the view matrix to `(x, y, z)` |
| `+0x70` | **half-size**: the quad is `(x +- h, y +- h)`, square, no roll |
| `+0x74..+0x76` | RGB; `+0x77` alpha, **times a near fade** |
| `+0x78` | atlas frame: `a2 + frame * 0x30` is that frame's UV corners |

The near fade: `z` is view-space, negative in front. A particle with `z > -1.0`
(`DAT_08a88500`) is not drawn; one with `-2.0 < z <= -1.0` (`DAT_08a88504`) has
its alpha scaled by `-(z + 1)` (`DAT_08abf57c = -1.0`), so a particle comes in
over its first unit past the eye. All three words read live. It matters only for
a particle at the camera, not at a rocket's 11 units. The six words at
`DAT_08a907a0..` set the projection (`Gu_SetMatrix(1, ...)`), and a quad is six
vertices, `Gu_DrawArray(4, 0x19f, ...)`.

`ParticleSystem_DrawRolledQuads` (`0x089178c0`) has the same walk and the roll law
above. **Both are live-probed**: `scripts/psp-weapon-pair.py --probe flare`
(`0x089194d0`) and `--probe rolled` (`0x089178c0`) break on each call during a
Rocket launch and read the instance's pool, and the sizes, colours and positions
read are the ones these functions consume. What that run measured is in
[`rocket-visuals.md`](rocket-visuals.md#2026-10-01-the-launch-glow-the-flares-4-bit-sprite-and-ageing).

### The instance matrix scales a root emitter's spawn, and a run emits one tick short (2026-10-01)

Measured on `WO_SHIP_EXPLOSION` as a grid opponent's wreck threw it (PPSSPP, Talon's Junction,
`scripts/psp-wreck-capture.py --pools`, which breaks on every `ParticleSystem_DrawEmitterPool` and reads
the instance's pool, its resource, its scale words at `+0x28` and its node matrix at `+0xf0`; two boots
for the counts, one for the positions). Three laws, each read against the same effect played alone by
ours (`System::ignite`, one tick a step).

**1. The matrix's scale reaches a root emitter's spawn offset and its velocity, and nothing else.**
`Psys_Spawn_q` copies the matrix it is given into the instance (`+0xf0`) and the emit functions carry the
spawn offset and the velocity through it unless the emitter's flag `0x2` is set. The explosion's
matrix is the hull's own, rows `0.75` long (read live on all four of its resources, and on the eight
`WO_SHIP_FXNODE_EXPLO` / `WO_SHIP_DEATH_SPARKS` instances), the instance's scale words `+0x28..` all
`1.0`, and:

| Effect | Original / ours with the scale ignored | Original / ours with `0.75` |
| --- | --- | --- |
| smoke ring (root, 18 particles): RMS radius, ticks 4 to 72 | `0.76, 0.77, 0.73, 0.73, 0.73, 0.73` | **`1.02, 1.01, 1.01, 1.01, 1.01, 1.01`** |
| fireball: horizontal spread / centroid rise at tick 40 | `0.79` / `0.73` | `1.11` / `0.89` |
| debris: centroid rise at ticks 24, 40, 56 | `0.78, 0.72, 0.69` | **`1.03, 1.00, 0.98`** |

The smoke ring's RMS radius reads the same on two more boots (`poolB` ticks 8 to 14 `14.5, 14.7, 14.8, 14.9`, `poolC` ticks 8 to
12 `13.9, 14.0, 14.1`, against ours `13.8, 13.9, 14.1, 14.2`), three boots in all. Sizes are **not** scaled: the fireball's half-size is `2 + 9 * age` of its life (`2.06` at birth, then
`+0.1155` a frame, matching the authored channel) on both sides, and so are the lifetime and the alpha ramp
(`0.96 / 0.96, 0.89 / 0.90, 0.78 / 0.80` at ticks 28, 34, 40). A **child** instance (`trail`, the debris's
per-particle child, `0x91ae580`) reads a matrix of unit rows, so a child spawn takes no scale. `1.0` is every
other matrix read so far: the Rocket's three `WO_ROCKET_EXPLO` and `WO_ROCKET_EXPLO_TRACK` spawns pass an
identity rotation with only the translation set (`--probe spawns`, a3 rows `1, 0, 0 / 0, 1, 0 / 0, 0, 1`).
Ported as `System::set_frame_scale`, called for the wreck's node effects and the big explosion only. Confidence
**85** (the smoke ring and the debris rise land on `1.0`; the fireball's `1.1` and the debris's `1.2` horizontal
spread, and the spikes, are not isolated - the mode-2 emitters (`SHIP_DEBRIS`, `FIRESPIKES`) spread more
than the uniform law gives and their cause is open).

**2. A finite run emits one tick short.** The explosion's emitters author `duration` `10` (smoke, `2` a tick),
`20` (fireball, `1`), `4` (`FIRESPIKES`, `3`) and `8` (debris): the original holds at most **18, 19, 9** of them
(ours held 20, 20, 12), on two boots, and a particle count that stayed flat from tick 8 to 72 so none was
lost to age. The rule that fits all of them and every `duration 1` burst (`WO_ROCKET_EXPLO`'s `DEBRIS`, `32` at
once, drawn exactly once) is: emit on the first update unconditionally, then only while more than one tick of
the duration is left - `d - 1` emissions for `d >= 2`, `1` for `1`. Ported as `EmitterSpec::short_run`.
Confidence **80** (three emitters, two boots; the `duration 1` case is the corpus's behaviour rather than a
separate measurement). It agrees with the Bomb's ring: `WO_BOMB_SMOKERING` authors `duration 20`, so the rule's last round is
tick 18, and the two-boot measurement in the section above saw births at ticks `0, 1, 3, 7, 16` and none later.

**3. A sprite template dies with one tick of its life left.** The explosion's `Glow` (life `6`, size `30`
held to `0.287`, then to `0`) was logged by `ParticleSystem_DrawParticle` (`scripts/psp-wreck-capture.py
--templates`) at sizes `30.00, 30.00, 28.04, 21.00, 14.09` and colours `ffffffff, fff3d9ff, ffe7b2ff,
ffdc8ccd, ffd06689` - five draws, ages `0` to `4`; ours drew a sixth at `7.17`, alpha `0.27`. The five match the
size and colour channels to the last digit (`oag_vex::pob::initial`). Confidence **75** (one boot).

**Probably the same law, unmeasured:** the hull's own collision sparks and the other effects parented to a craft's node ride the same
`0.75`-row matrices and are still played at frame scale `1.0` (`oag_game::race::hit_sparks` and friends).

**Not a law, a warning:** a census of rolled quads by bounding box overstates a particle's size by up to
`sqrt(2)` (`4/pi` on average for a random roll) - the first reading of this section took that for a `0.79` size
ratio and nearly scaled sizes. Read a size from the pool's `+0x70`, or from a quad's edge, not its extent.

**What the pool probe does not see:** the sprite templates (drawn one at a time by `ParticleSystem_DrawParticle`)
and the `FIRESPIKES` streaks' length (the probe stores each streak's head and second point; the comparison used
centroids, which sit half-way along).
