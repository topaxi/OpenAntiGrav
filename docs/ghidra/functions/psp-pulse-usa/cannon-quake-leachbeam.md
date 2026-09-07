# The Cannon, the Quake and the LeachBeam: all three fire bodies read

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** the three fire handlers `plasma.md` (2026-09-02) named only from the
dispatch table and a bit map - "do not assume anything about what they do" -
are now read end to end, each down to its spawn, its per-tick behaviour and
(where one exists) its damage/slowdown application. One correction falls out
immediately: **the Cannon candidate `0x088537ac` this thread's own 2026-09-07
entry named is wrong.** It decompiles as `Ai_Construct`
(`0x088536bc`-`0x08853a7f`, already named in [pickups.md](../../../gameplay/pickups.md)
for the *Autopilot's* own reveal), not a weapon handler at all - a coincidence
of address, not a candidate. See [History](#history) for the arithmetic that
produced it and the one that replaces it.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x088577ac` | `Weapon_FireCannon` | 85 |
| `0x0883f424` | `Cannon_UpdateReload` | 85 |
| `0x088648ec` | `Cannon_Init` | 80 |
| `0x0886c600` | `Weapon_FireQuake` | 88 (was 82) |
| `0x08874b14` | `Quake_Init` | 82 |
| `0x08866658` | `Weapon_FireLeachBeam` | 88 (was 82) |
| `0x08873d3c` | `LeachBeam_InitLocked` | 82 |
| `0x08872da8` | `LeachBeam_InitUnlocked` | 80 |
| `0x08866b08` | `LeachBeam_UpdatePool` | 85 (was `FUN_08866b08` at 60-78, see [bad-memory-access-halt.md](bad-memory-access-halt.md)) |
| `0x08866804` | `LeachBeam_Drain` | 78 |

Read [weapon-fire.md](weapon-fire.md) first for the two traps every reading below
depends on: `entity+0x1b8` is the fire-request word, and every `jal` operand and
`func_0x000NNNNN` the decompiler prints in this binary is **image-base-relative**
(`real = 0x08804000 + operand`) - it is what produced this page's own correction.

## The Cannon: `0x088537ac` refuted, `0x088577ac` confirmed, and it does not use the bit system at all

### The address correction, from a clean decompile

`Weapons_DispatchFire` (`0x08861814`) was decompiled whole for this page. Its
bit-`0x4000` arm reads, verbatim:

```c
if ((uVar5 & 0x4000) != 0) {
    func_0x000537ac(*(undefined4 *)(param_2 + 0x58),iVar9,uVar6);
    uVar5 = *(uint *)(iVar9 + 0x1b8);
}
```

`0x08804000 + 0x000537ac = 0x088577ac`, **not** `0x088537ac` - a `0x4000`
arithmetic slip in `plasma.md`'s sixteen-bit table, the same magnitude as the
bit being read, which is presumably how it happened. `0x088537ac` decompiles
as a real function, but the wrong one: it falls inside `Ai_Construct`
(`get_function_by_address` gives `Entry: 088536bc, Body: 088536bc - 08853a7f`),
which builds an AI driver record and has nothing to do with weapons.
`0x088577ac` decompiles as a real weapon handler with the shape described below.
**Confidence 90** on the address itself - a direct decompile and a checked
addition, cross-checked against `weapon-fire.md`'s own independent (and
correct) citation of `FUN_088577ac` for this same bit, predating `plasma.md`'s
slip.

### `Weapon_FireCannon` (`0x088577ac`) is a round-robin burst spawn, not a single shot

```c
void Weapon_FireCannon(CannonPool *pool, Craft *craft, int craft_index) {
    craft->fire_flags &= ~0x4000;               // craft + 0x1b8, cleared unconditionally
    uint live = pool->live;                      // pool + 0x158
    pool->flags |= 2;
    if (live < 0x3c) {                           // cap 60
        Round *r = pool->slot[live];             // pool + 0x68 + live*4
        bool left = (craft->shots & 1) != 0;     // craft + 0x154, low bit
        Entity *emitter = left ? craft->entity->barrel[1]   // craft->entity + 0x68
                               : craft->entity->barrel[0];  // craft->entity + 0x64
        pool->live = live + 1;
        float speed_kmh = craft->entity->body->speed * 3.6;   // entity+0x794+0x398, |velocity|
        r->flags = 0; r->flags = 1;
        r->owner = craft_index;                  // + 0x40
        r->id    = ++g_next_projectile_id;        // + 0x44
        Cannon_Init(speed_kmh, r, emitter, craft->entity->stats_a /* +0x60 */,
                    !left, craft->entity->stats_b /* +0x50 */);
    }
}
```

**Two muzzles, alternating.** `craft+0x154`'s low bit picks between two emitter
anchors on the craft's own entity (`+0x64`/`+0x68`) - a twin-barrel cannon,
consistent with HD/Fury's own `CannonManager` (`ps3-hdfury-eu/weapons.md`)
being a real class in this lineage, though that binary settles nothing about
this one's mechanics.

**Every round inherits the firing craft's own current speed**, read off its
rigid body (`entity+0x794+0x398`, the same `|velocity|` field `contact-response.md`
already reads for the per-frame contact loop) and converted to km/h - the same
"carry the shooter's own speed" shape the Shuriken's throw already has.

**The pool cap (60) is a live-count guard, not a magazine size** - same shape as
`Weapon_FirePlasma`'s `pool->live < 0x10`. `Cannon_Init` (`0x088648ec`) is the
actual constructor: it copies the emitter matrix as the round's pose, computes
`speed = speed_kmh_arg + <a per-class base speed from 0x00060af4, unread>`,
builds velocity as `forward * speed`, and copies the *firing craft's own*
cached track-locator record (`entity+0xad8..+0xae4`, the same fields
[Quake_Init](#quake_init-0x08874b14-locates-the-firing-craft-on-the-track) reads)
onto the round before playing a fire cue. Confidence **80** - direct decompile,
the base-speed lookup and the hit/damage path are not chased.

### What actually fires it: `craft+0x1bc == 3` drives a reload countdown, not the pickup bit

**Bit `0x2000` (the Cannon's own request bit per `Weapon_RequestFire`'s
id-to-bit map) is dispatched by nothing, and that is expected, not broken.**
The Cannon does not fire through the fire-request-word system the other twelve
weapons share at all. `Cannon_UpdateReload` (`0x0883f424`) does:

```c
void Cannon_UpdateReload(float dt, Entity *ship) {
    if (ship->weapon_pad_flags->0x16 == 0) return;      // ship+0x94+0x78
    if (ShipState(ship) != 1) return;                   // must be racing
    Craft *craft = ship->craft;                          // ship + 0x4c
    if (craft->held != 3) return;                        // craft + 0x1bc - held-weapon id
    craft->reload -= dt;                                  // craft + 0x158
    if (craft->reload < 0.0) {
        craft->reload += ActiveCannonStats()->rate;       // + 0x78
        craft->shots  -= 1;                                // craft + 0x154
        craft->fire_flags |= 0x4000;                       // arms Weapon_FireCannon
    }
    if (craft->shots == 0) {
        <ammo bookkeeping>
        craft->held = -1;
    }
}
```

Called every frame per craft from `FUN_0883f540`, itself part of the main
per-craft update chain (it also drives the Missile/LeachBeam lock scan and a
craft's own scene-node cache refresh), gated only on the craft's *held weapon
id being 3* - not on any bit of `craft+0x1b8`. So the sequence is: the player
presses fire, `Weapon_RequestFire` sets bit `0x2000` (which nothing reads) and
leaves `craft+0x1bc == 3` in place; `Cannon_UpdateReload` runs every frame
regardless, and it is *that* function's own countdown - refilled from the
Cannon's own authored `rate` - that periodically arms bit `0x4000`, which *is*
dispatched, to `Weapon_FireCannon` above. **Three independent facts tie id 3 to
the Cannon**: this function is gated on it and reads/writes exactly the
`rounds` (`craft+0x154`) and `rate` (`ActiveCannonStats+0x78`) attributes
`mine.md` and `weapon-fire.md` already identified as authored by the Cannon's
`<Stats>` alone; `craft+0x154`'s low bit is *also* what
`Weapon_FireCannon` reads to alternate barrels, a cross-check from the spawn
side; and it is the only weapon whose fire mechanism does not route through
`craft+0x1b8` at all, matching "the Cannon's own request bit is dispatched by
nothing" being true and *irrelevant* rather than a sign of missing work.

**A trap for whoever builds this**: `Weapon_FireCannon`'s `pool->live` lives at
`world+0x58 + 0x158` (the *subsystem*), while `Cannon_UpdateReload`'s reload
timer lives at `craft + 0x158` (the *per-craft weapon record*) - same offset,
different base, no relation. Worth naming next to the `craft+0x1ac`/`+0x1ac`
trap the handover thread already records for the Mine.

**Buildable now**: the whole fire-rate-and-burst mechanism is read at
instruction level. Not chased: the per-class base speed
(`func_0x00060af4`), and what a round does on hitting a craft or wall (no
`Cannon_HitCraft`-style function was located this pass).

## The Quake: a travelling point on the track's own spline, and nothing that touches a mesh

### `Weapon_FireQuake` (`0x0886c600`) - one instance, gated on a busy flag

```c
void Weapon_FireQuake(QuakePool *pool, Craft *craft, int craft_index) {
    Instance *q = pool->instance;                 // pool + 0x60 - a SINGLE instance, no array
    if (q->active != 0) return;                    // + 0x48 - only one quake in flight, ever
    QuakeStats *s = ActiveQuakeStats();
    craft->held = -1;
    craft->fire_flags &= ~8;
    craft->cached_damage        = s->damage;        // craft + 0x18c  <- stats + 0x60
    craft->cached_radius        = s->radius;         // craft + 0x190  <- stats + 0x64
    craft->cached_slowdown_time = s->slowdown_time;  // craft + 0x194  <- stats + 0x68
    q->flags = 0; q->flags = 1;
    q->owner = craft_index;                          // + 0x40
    q->id    = ++g_next_projectile_id;                // + 0x44
    Quake_Init(q, craft_index, craft->entity, &craft->fwd /* craft + 0x40 */);
    <telemetry only in >=14-player sessions>
}
```

**The Quake copies its own stats onto the *firing craft*, not onto the
instance** - `craft+0x18c/+0x190/+0x194` - exactly the shape `pickups.md` and
this thread's Open list already flag as the Repulser's own, unique-looking
behaviour ("the field is a state the craft is in"). It is not unique: this is
the **second** weapon that does it. Confidence **88** on the handler as a
whole - a full, unambiguous decompile with no VFPU trap in the path read.

### `Quake_Init` (`0x08874b14`) locates the firing craft on the track

This is the function that decides what "the Quake" *is* as a piece of state,
and it never touches a mesh, a vertex buffer or any collision geometry - it
only resolves the firing craft's own already-cached position on the track's
spline (the `entity+0xad8..+0xae4`/`+0xaf0`/`+0xb30` fields
[engine.md](engine.md#0xb10-is-splineptdown-and-the-whole-record-is-a-located-spline-sample)
already identifies as `SplinePt` records `AiTrack_LocatePosition` fills) into a
travelling wave's initial state:

1. Reads the craft's current segment reference(s) and its arc-position float
   (`entity+0xb30`, a field inside the first `SplinePt` record).
2. Checks a small global list (`_DAT_...598a8`, entries keyed by segment id
   plus a `[t_start, t_end]` window) to see whether the launch point straddles
   two segments - a **track-gap/junction continuity check**, not a trigger for
   anything visual: when it finds a match it flags `q->active = 1` and nudges
   the parametric `t` across the gap by a fixed epoch (`_DAT_...78cac / segment
   length`), so a quake launched right at a gap does not get stuck astride it.
3. Dots the craft's own forward vector (the fourth argument) against the
   segment's own tangent to pick a **direction sign** (`+1`/`-1`) for which way
   along the spline the wave should travel from the launch point.
4. Stores the segment reference(s), the parametric `t`, the direction sign and
   a "span" kind (one segment or two) into a handful of small module-scope
   globals - and nothing else. No draw call, no vertex write, no mesh handle
   anywhere in this function.

Confidence **82**: every field read is one this project has already located
and named from an independent reading (`engine.md`'s `SplinePt`), and the
shape (locate-on-spline, pick a direction, stash a scalar position) has no
plausible alternative reading given what it touches. Not chased: the exact
symbol identity of the module globals it writes (their real, un-rebased-looking
addresses were not resolved to real Ghidra data symbols this pass), and -
critically - **the function that advances that stored position every
subsequent frame was not found.** See [Open](#open-1).

### The "wave branch" inside `FUN_088418e0`, located precisely

`engine.md` already read that "`+0x60`/`+0x68`" (Quake's `damage`/`slowdown_time`)
are read by "`FUN_088418e0`'s wave branch" without giving an address. It is
`0x08841e60`-`0x08842064`, and it is the *damage and slowdown application*, run
for **every racing craft, every frame** (not gated on distance in this
function - see below):

```c
// inside FUN_088418e0, once per craft per frame, s2 = this craft's entity
if (craft_is_shooter(s2)) goto skip;                 // s2->0x360 == quake owner index
if (s2->craft->fire_flags & 0x10) goto skip;         // shielded
if (s2->0x860 & 0x40) goto apply;                    // "quake reached me" latch - see below
goto skip;
apply:
    entity->pending_slowdown += QuakeStats->slowdown_time;  // entity+0x130 += stats+0x68
    entity->pending_kind      = 5;                            // entity+0x138 - Quake's own id
    entity->pending_attacker  = <shooter craft index>;         // entity+0x13c
    Ship_Damage(entity, QuakeStats->damage, 2);                // stats+0x60, mode 2
```

`entity+0x130`/`+0x138`/`+0x13c` is the **same shared pending-hit channel** the
Missile and the Mine/Bomb blast already use (`engine.md`'s nine-writer table),
consumed the same way by `Ship_AddSlowdown`. `Ship_Damage` is already a named
function (`0x088439ac`) - confirmed by `get_function_by_address`, not
inferred - and `engine.md` already recorded that it compares its `weapon_kind`
argument against the literal `7`, which is `LeachBeam_Drain`'s own tag (below),
not the Quake's `5`; the two tags share one consumer function but distinct
values. Confidence **85** for this block: `entity+0x138 = 5` and the
`damage`/`slowdown_time` reads land exactly where `WeaponStats_ParseQuake`
(`0x0880c60c`) stores them, cross-checking `engine.md`'s independent reading of
the parser.

**What this settles for question 4**: the Quake's effect on a craft is applied
through the identical generic "pending hit" mechanism every other weapon here
uses - a scalar damage number and a scalar slowdown-seconds number, credited
once a latch bit is set. Nothing in the whole chain (`Weapon_FireQuake` ->
`Quake_Init` -> this branch) reads or writes a vertex, a mesh handle, a track
collision triangle, or any per-segment geometry override. **The Quake is a
travelling impulse along the track's own path (a segment + parametric-`t`
position plus a direction sign), not a deformation of the track**, and nothing
found here needs new geometry-mutation machinery to build - the missing piece
is purely the wave's own per-frame position update (below).

### Open

- **The function that advances the wave's stored `t` each frame, and the one
  that sets `entity+0x860 & 0x40` (the "the wave has reached me" latch this
  page's damage branch reads) were not found.** `Quake_Init` writes the wave's
  launch state once and this page did not find where it is read again. Given
  the shape already established (a scalar spline position with a direction
  sign, and a generic per-craft damage application keyed off a plain flag bit),
  the missing function is very likely a short, dedicated "advance the quake
  along the spline and flag whichever craft's own spline position it currently
  overlaps" loop - plausibly indexed by the same craft-position `SplinePt`
  fields this page already reads, one per craft, but that comparison itself
  was not located. **Do not guess a travel speed or a hit-window width for
  it** - neither was measured.
- **Whether the Quake has an authored travel speed at all is unread.**
  `WeaponStats_ParseQuake`'s four attributes (`damage`, `radius`,
  `slowdown_time`, `absorb` - `engine.md`'s own table) have no obvious "speed"
  among them; `radius` is the only candidate left unexplained by this page and
  might govern the hit window's width along the spline rather than a 3D blast
  radius, but that is a guess, not a reading.
- **The Quake's own effect/trigger is unread.** No `Psys_Spawn`-style call was
  seen in `Weapon_FireQuake` or `Quake_Init`; if the original draws anything
  for the wave's passage, its call site is still unlocated.
- **The per-class base speed `Cannon_Init` reads (`0x00060af4`) is unread**, and
  so is whatever a Cannon round's collision does on a hit.

## The LeachBeam: a resolved link to a pre-locked target, drained every tick it holds

### `Weapon_FireLeachBeam` (`0x08866658`) fires world-wide, not per-craft

```c
void Weapon_FireLeachBeam(LeachBeamPool *pool, Craft *craft, int craft_index) {
    pool->flags |= 2;
    craft->held = -1;
    craft->fire_flags &= ~0x8000;
    if (pool->live != 0) return;          // pool + 0x68 - ONE beam in the WHOLE RACE at a time
    Instance *b = pool->instance;          // pool + 0x64
    b->flags = 0; b->flags = 1;
    b->owner = craft_index;                // + 0x40
    b->id    = ++g_next_projectile_id;      // + 0x44
    Entity *shooter = craft->entity;         // craft + 0xf0
    if (craft->lock_target == -1) {          // craft + 0x16c
        LeachBeam_InitUnlocked(b, shooter->pose, craft_index, shooter->emitter, shooter);
    } else {
        LeachBeam_InitLocked(b, shooter->pose, craft_index,
                              craft->lock_target,      // craft + 0x16c - target craft index
                              craft->lock_node,          // craft + 0x168 - target's own scene node
                              shooter->rate_or_cue,       // shooter + 0x50
                              shooter);
    }
    <telemetry only in >=14-player sessions>
    pool->live += 1;
}
```

**Only one LeachBeam can be in flight in the entire race at once** - `pool+0x68`
is a world cursor, not a per-craft cooldown, a stricter gate than any other
weapon here. Confidence **88** - direct decompile, no VFPU trap on this path.

### Victim selection is the Missile's own lock-on, reused whole

`craft+0x16c`/`craft+0x168` are filled **before** this handler ever runs, by
the same `Ship_AcquireLock` scan `missile.md` already documents in full: a
longitudinal cone ahead of the craft, nearest-along-forward wins, using the
LeachBeam's own `lock_min_dist`/`lock_max_dist` at `<Stats>+0x114`/`+0x118`
(`missile.md`'s own reading - "the two weapons that author `lock_max_dist`/
`lock_min_dist` are the Missile and the LeachBeam"). **Nothing new needs
building for target selection** - it is the Missile's lock, unmodified, already
buildable from `missile.md` alone.

### The two constructors: an unlocked beam fizzles, a locked one becomes a real link

`LeachBeam_InitUnlocked` (`0x08872da8`, `kind = 2`) copies the shooter's own
emitter matrix as the beam's pose and sets its "target" field to the
**shooter's own entity** - there is no external target at all. Its pool update
(below) gives kind-2 instances no distance or damage logic whatsoever: they
just run out a short expiry and retire. **Firing without a lock plays the cue
and does nothing else.**

`LeachBeam_InitLocked` (`0x08873d3c`, `kind = 1`) is the real weapon:

- `instance+0x48 = target_craft_index` (from `craft+0x16c`).
- `instance+0xa0 = target_scene_node` (from `craft+0x168`) - **this is the field
  `bad-memory-access-halt.md` found null and crashed PPSSPP on**, when that
  page's raw fire-bit test skipped the lock entirely. A legitimately fired
  locked beam always has this filled straight from the lock, closing that
  page's own "what fills `instance->0xa0`" Open item at confidence **85**: it
  is the target's own resolved scene node, copied verbatim from the lock's own
  `craft+0x168`, not computed or re-resolved here.
- Allocates a positional-audio-style handle (`instance+0x4c`) and zero-fills a
  **32-entry chain of `0x30`-byte transforms** (`instance+0x3c0`..) alternating
  between two initialisation patterns - this reads as the beam's own drawn
  geometry, a segmented link between the two craft rather than a single static
  bolt, consistent with a "beam" needing more than one quad to draw convincingly.

### `LeachBeam_UpdatePool` (`0x08866b08`) - the function `bad-memory-access-halt.md` read half of

That page named this function's ownership "confidence 78 weapon-instance pool
machinery, confidence 60 on anything narrower" - reading it whole this pass
settles the narrower question: **it is the LeachBeam's own pool**, not a
shared or Missile-owned one. Every field it touches (`+0x48` target index,
`+0xa0` target node, `+0x5c` owner-craft-entity, `+0x54` kind 1/2) matches
exactly what the two constructors above just wrote, with no other weapon's
constructor touching any of them. Confidence **85**, up from that page's 60.

For a live kind-1 instance not yet flagged disconnected (`instance+0x3c & 0x40`
clear):

1. Measures the floor-clamped distance between the beam's own resolved world
   position and the **target's** resolved world position, via
   `FUN_08872f54` - the exact distance function `bad-memory-access-halt.md`
   already read in full (`max(|posA - posB|, floor)`).
2. Compares it against **`ActiveLeachBeamStats+0x11c`** - a fourth LeachBeam
   attribute, immediately adjacent to `lock_min_dist`/`lock_max_dist`
   (`+0x114`/`+0x118`) and not previously identified; call it the beam's own
   maximum reach. Past it, or if the target is invulnerable
   (`target+0x860 & 0x1000`), shielded (`target->entity+0x1b8 & 0x10`), or
   either craft's race-state check fails, the link is marked disconnected
   (`instance+0x3c |= 0x40`) rather than destroyed outright - it can sit
   disconnected for a few ticks (a linger window) before the pool actually
   retires it (`instance+0x3c & 4`).
3. **While connected**, every tick calls `func_0x0006f020(instance)` for a
   rate float, writes it into a per-craft readout slot (presumably a HUD/audio
   amplitude on the shooter), and - whenever that rate is positive - calls
   `LeachBeam_Drain` (below).

### `LeachBeam_Drain` (`0x08866804`) - the transfer, both directions

```c
void LeachBeam_Drain(Pool *pool, int instance_index, int owner_idx, int target_idx) {
    Entity *owner  = pool->craft_entities[owner_idx];    // pool + owner*4 + 0x44
    Entity *target = pool->craft_entities[target_idx];
    if (owner->fire_flags & 0x10) return;                 // shielded - gates BOTH halves
    if (Ship_IsValid(target)) {
        float amount = <func_0x0006eedc>(instance);
        target->pending_leach   += amount;                 // target + 0x120
        target->pending_kind     = 7;                        // target + 0x138 - LeachBeam's own id
        target->pending_source   = <ActiveLeachBeamStats+0x110>;   // target + 0x134
        target->pending_attacker = owner_idx;                // target + 0x13c
    }
    if (Ship_IsValid(owner)) {
        float amount = <func_0x0006ef18>(pool->instance[instance_index]);
        owner->pending_kind     = 7;                          // owner + 0x138
        owner->pending_gain    += amount;                      // owner + 0x128
        owner->pending_attacker = <instance's own owner field>; // owner + 0x13c
    }
}
```

**This is the literal "leach"**: a victim accumulator (`entity+0x120`, distinct
from the shared `+0x130` pending-hit slot every other weapon uses) and a
*shooter* accumulator (`entity+0x128`) are both credited from the same
transfer, every tick the link survives the range/shield/state gate above -
draining is continuous, not a single hit, and both halves stop the instant
either craft is shielded. `entity+0x138 = 7` confirms LeachBeam's own weapon-
type tag in the same field the Quake writes `5` into and `engine.md` already
found `Ship_Damage` comparing against `7` - two independent paths converging on
the same tag value. Confidence **78**: the transfer's *shape* (two accumulators,
symmetric credit, shield-gated) is read at instruction level; the two rate
functions (`func_0x0006eedc`, `func_0x0006ef18`) and what actually consumes
`+0x120`/`+0x128` into visible health/shield state were not decompiled this
pass.

### Open

- **The two drain-rate functions** (`func_0x0006eedc` = `0x0886eedc`,
  `func_0x0006ef18` = `0x0886ef18`) are unread - they decide the actual
  numbers, likely off a `<Stats>` attribute this page has not identified.
- **What consumes `entity+0x120`/`entity+0x128`** into an actual health or
  shield change is unread. It is **not** `Ship_AddSlowdown`'s `+0x130` channel -
  these are two different fields on two different craft, credited by a
  different function, and their own consumer is the single largest remaining
  gap for porting the LeachBeam's actual effect.
- **`ActiveLeachBeamStats+0x11c`'s attribute name is unmeasured.** Its offset is
  now located (adjacent to `lock_min_dist`/`lock_max_dist`); no parser call for
  it was re-checked against `WeaponStats_ParseLeachBeam` (`0x0880d328`,
  `plasma.md`'s table) this pass.
- **The 32-entry `0x30`-byte transform chain's exact use** (which quad/segment
  of the beam it draws, and by which draw call) was not read past its
  zero-fill shape.
- **`func_0x0006f068`/`func_0x0006f090`/`func_0x0006f0a4`/`func_0x0006ffa0`** -
  the expiry/disconnect/retire helpers `LeachBeam_UpdatePool` calls - are
  unread past their call sites; their names are guesses in the prose above
  ("expiry", "disconnect callback") and are not in `names.tsv`.

## What is buildable now and what still is not

- **The Cannon is buildable.** Its whole fire-rate mechanism - the reload
  countdown gated on `craft+0x1bc == 3`, the round/rate attributes, the
  twin-muzzle alternation, the craft-speed-inherited round, the round-robin
  spawn pool - is read at instruction level end to end. Missing only the
  per-class base speed and the round's own hit/collision behaviour, neither of
  which blocks a first pass (every other weapon here ships with at least a
  placeholder blast/impact).
- **The Quake is buildable for its hit/damage/slowdown half**, which reuses the
  Missile's and Mine/Bomb's own shared pending-hit channel outright - nothing
  new to build there beyond wiring the Quake's own `damage`/`slowdown_time` and
  the self-exclusion/shield checks this page reads. **Not buildable yet is the
  travelling half**: the wave's own per-frame advance along the spline and the
  latch that flags "this craft is currently under it" were not found this
  pass, and inventing a travel rule (a speed, a hit-window width) would be
  exactly the kind of guess this project's rubric exists to prevent. **It does
  not need track deformation of any kind** - that establishes the shape of the
  work, even though the work itself is not finished.
- **The LeachBeam is buildable for target selection and the connect/disconnect
  gate** (all reused from the Missile's lock, plus the range/shield checks read
  above), **but not for the actual drain amount**, which needs the two rate
  functions and the `+0x120`/`+0x128` consumer this page leaves open.

## History

This page supersedes the three addresses `weapons-eight-of-thirteen-the-plasma-and-the.md`'s
2026-09-07 entry named as unread: `0x088537ac` (refuted, see above),
`0x0886c600` and `0x08866658` (both now read in full). See that thread for the
narrower, single-session account of how this pass came about.
