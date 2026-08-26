# The Missile: its stats, its lock, its guidance and its flight

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** read end to end. The Missile is the first weapon in this project
whose *whole* behaviour is recovered rather than partly invented - the two lock
distances are authored, and the lock rule, the guidance law, the speed ramp and
the flight model are all read at instruction level. What is left invented is
listed under [What is still ours](#what-is-still-ours), and it is short.

Ported in `crates/gameplay/src/projectile/missile.rs` and
`crates/gameplay/src/projectile.rs`.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0880c31c` | `WeaponStats_ParseMissile` | 90 |
| `0x088685cc` | `Weapon_FireMissile` | 90 |
| `0x088685bc` | `WeaponPickup_ArmMissile` | 80 |
| `0x0885a160` | `Missile_Init` | 90 |
| `0x0885a918` | `Missile_Update` | 92 |
| `0x0885a038` | `Missile_SpeedNow` | 90 |
| `0x08862d9c` | `Weapon_RequestFire` | 92 |
| `0x08844784` | `Ship_AcquireLock` | 88 |
| `0x08844ae8` | `Ship_FireHeldWeapon` | 85 |
| `0x08861d20` | `WeaponPickup_Grant` | 80 |
| `0x0886e0f8` | `WeaponPickup_ArmRocket` | 80 |
| `0x08869588` | `MissilePool_Update` - **renamed**, see below | 88 |
| `0x08868d50` | `Missile_SpawnExplosion` | 85 |
| `0x08868ea4` | `Missile_ApplyBlastForce` | 88 |
| `0x088690fc` | `MissilePool_TestCraftHits` | 85 |
| `0x088687c0` | `MissilePool_SpawnRemote` | 78 |
| `0x08868a10` | `MissilePool_DestroyRemote` | 78 |
| `0x0886de60` | `RocketPool_Update` | 82 |

**`0x08869588` was `Projectiles_Update_q` at 75** on
[weapon-fire.md](weapon-fire.md), where it was read as the projectile pool in
general. It is the **Missile's** pool specifically, and three things say so
rather than one: it calls `Missile_Update` on every slot, its teardown calls
`Missile_SpawnExplosion`, and the Rocket has a pool of its own at `+0x124` whose
update (`RocketPool_Update`) calls `Rocket_Update` instead. The `_q` comes off
with the rename.

**Read [weapon-fire.md](weapon-fire.md) first** for the fire-request word and the
Rocket; this page is the Missile's half and it corrects one claim on that page -
see [A correction](#a-correction-craft0x1bc-is-not-a-target).

**The addresses on this page are real, not image-relative.** Ghidra renders this
program's operands and `jal` targets as `real - 0x08804000`, which is why
`get_xrefs_to` returns nothing for most of them; `search_instructions` on the
relative value is what works. That trap is documented at length on
[weapon-fire.md](weapon-fire.md#read-this-first-the-disassemblys-addresses-are-image-base-relative).

## The `<Stats>` block

`WeaponStats_ParseMissile` (`0x0880c31c`) is the same shape as the Rocket's
parser - match an attribute name, `swc1` the float at a fixed offset - and its
twelve stores land in the per-speed-class stats struct immediately after the
Rocket's eleven. Confidence **90**: the store offsets are unambiguous and the
attribute strings sit in one contiguous run.

| Offset | Attribute | | Offset | Attribute |
| --- | --- | --- | --- | --- |
| `+0x30` | `damage` | | `+0x48` | `blastradius` |
| `+0x34` | `venomspeed` | | `+0x4c` | `blastforce` |
| `+0x38` | `flashspeed` | | **`+0x50`** | **`lock_min_dist`** |
| `+0x3c` | `rapierspeed` | | **`+0x54`** | **`lock_max_dist`** |
| `+0x40` | `phantomspeed` | | `+0x58` | `absorb` |
| `+0x44` | `launchspeed` | | `+0x5c` | `slowdown_time` |

The stats struct for the live speed class is
`*(&DAT_08b32420 + DAT_08b32428 * 4)` throughout. **`DAT_08b32428` is measured**:
`0` in a Single Race and `1` in Eliminator, both at Venom, so it selects the stats
*file* - `WeaponStats_Race.xml` against `WeaponStats_Elimination.xml` - and not the
speed class. The speed class is the separate `DAT_08b31040`, and the *race mode* is
a third global, `DAT_08b31048`, which reads `3` in a Single Race and `8` in
Eliminator. Three globals it is easy to conflate; see
[weapon-ai.md](weapon-ai.md), which spends the third.

**LeachBeam's lock distances are at `+0x114`/`+0x118`** in the same struct, read
by the same function as the Missile's - see the lock below. It is the only other
weapon that locks.

## A press puts exactly one missile in the air

`Weapons_DispatchFire` (`0x08861814`) dispatches sixteen bits of `craft+0x1b8`.
**Bit `0x40` is the Missile**, calling `FUN_088685cc(world+0x4c, craft, index)`.

That handler makes **one** spawn call through `Missile_Init` - no fan, no burst,
unlike the Rocket's three literal calls. It also clears its own bit and empties
the craft's held-weapon slot:

```c
world->flags     |= 2;
craft->held       = -1;          // craft + 0x1bc
craft->fire_flags &= ~0x40;      // craft + 0x1b8
if (world->live < 0x2e) {
    // one spawn, from the craft's velocity when it is moving and from a pose
    // row when it is not
    dir = craft->velocity;                          // craft + 0x10
    if (vdot_t(dir, dir) < 0.1) dir = craft->pose.row2;
    Missile_Init(slot, &craft->pose, &dir, craft_index,
                 craft->target,                     // craft + 0x160
                 ...);
}
```

**The target is passed through unchecked**, and that settles a question this
page used to leave open. `craft+0x160` is whatever `Ship_FireHeldWeapon` put
there, `Missile_Init` takes it as it comes, and nothing between the two tests it.

**Three independent things say bit `0x40` is the Missile**, which is why this is
at 90 rather than at a shrug:

1. `Missile_Init` builds **two** `Trail_InitPreset(_, 0)` trails. That is the
   twin-trail signature already recorded on [exhaust.md](exhaust.md), where the
   racing craft's single trail was settled by noting that only the missile
   constructor makes two.
2. It plays `WO_MISSILE_HEAD` at two anchors and the sound cues `MISSILE` and
   `_MISSILETVL`.
3. `Weapon_RequestFire`'s case for this weapon is one of exactly **two** that
   take a target argument, and the two weapons that author `lock_max_dist` /
   `lock_min_dist` are the Missile and the LeachBeam.

## `Weapon_RequestFire`, and the weapon-id to bit map

`Weapon_RequestFire` (`0x08862d9c`) switches on the held weapon id at
`craft+0x1bc` and sets the matching bit of `craft+0x1b8`. A thirteen-case switch
is not open to much interpretation; confidence **92**.

| id | bit | id | bit | id | bit |
| --- | --- | --- | --- | --- | --- |
| 0 Rocket | `0x80` | 5 | `0x20` | 10 LeachBeam | `0x8000` |
| **1 Missile** | **`0x40`** | 6 Autopilot | `0x1000` | 11 | `0x10000` |
| 2 Quake | `0x8` | 7 | `0x4` | 12 | `0x20000` |
| 3 | `0x2000` | 8 | `0x2` | | |
| 4 | `0x400` | 9 | `0x100` | | |

Ids 0, 2 and 6 are named from their own sound cues in `Ship_FireHeldWeapon`
(`0x08844ae8`): `ROCKET`, `QUAKELAUNCH`, and `_AUTOPILOT` plus `autopilot_eng`.
Id 10 is named from its lock distances. The rest are left as numbers.

**Only ids 1 and 10 take a target**, stored to `craft+0x160`/`+0x164` and
`craft+0x168`/`+0x16c` respectively.

**The whole id space is settled as of 2026-08-17, from a second direction**:
`WeaponAiStats_Load` (`0x08851d88`) dispatches thirteen named XML elements to the
same ids, giving `3 Turbo`, `4 Shield`, `5 Cannon`, `8 Bomb`, `9 Mines` for the
five left as numbers above. See
[ai-stats.md](ai-stats.md#it-confirms-the-weapon-id-space-from-a-second-direction),
which also records that this **disagrees** with
`oag_formats::weapons::Weapon::ALL`'s order at three positions - that enum follows
the string-pool layout, not the ids.

**And it opens a conflict rather than closing one.** Id 8 being the Bomb puts bit
`0x2` on the Bomb, and [weapon-fire.md](weapon-fire.md) reads that bit's handler
as the **Cannon** at confidence 72 because `rounds`/`rate` are the Cannon's
`<Stats>` alone. Both readings are left standing; three loose ends in the switch
(bit `0x2000` dispatched by nothing, three cases making no `FUN_08871ddc` call,
and that call's argument being a second id space that remaps 3/8/9) say the
id-to-bit relation is not the identity it looks like.

## The lock

`Ship_AcquireLock` (`0x08844784`) walks the entity table and writes the target
index to `entity+0x85c`, mirroring it to `weapon_record+0x1b4`.
`Ship_FireHeldWeapon` then passes `entity_table[+0x85c] + 0x794` as the target,
gated on `entity+0x860 & 1`. Confidence **88**.

```c
self->target = -1;  best = 999999.0;
for (i = 0; i < ship_count; i++) {
    other = entity_table[i];                        // &DAT_08b31788
    if (other == self || other == 0) continue;
    if (Ship_State(other) != 1) continue;           // must be racing
    if (other->weapon_record[0x12c] > 0.0) continue;    // pending hits, see below

    to_them = other->pose.position - origin;
    along   = dot(to_them, self_forward);            // LONGITUDINAL, not range

    held = self->weapon_record[0x1bc];
    if      (held == 1)  { min = stats[0x50];  max = stats[0x54];  }   // Missile
    else if (held == 10) { min = stats[0x114]; max = stats[0x118]; }   // LeachBeam
    else                 { min = 0; max = 0; }

    if (along < max && min < along
        && dot(normalize(to_them), self_forward) > 0.9      // code literal
        && along < best) {
        range = length(other.pos - origin);
        if (fabs(track_gap(self, other)) / range < 1.4)     // code literal
            { self->target = i; best = along; }
    }
}
```

Three things in there are worth stating plainly, because each is easy to get
wrong from the authored data alone:

- **`lock_min_dist` and `lock_max_dist` bound the *longitudinal* distance**,
  `dot(target - origin, forward)`, not the straight-line range. A craft directly
  alongside is near zero on that axis however close it is, and is excluded by the
  near bound rather than by the cone.
- **The cone is a code literal `0.9`**, about 26 degrees, and is not authored
  anywhere.
- **The winner is the nearest by longitudinal distance**, not by range and not by
  bearing.

### The along-track screen

The last test calls `FUN_0883dbf4(self, other, 0)`, which reads each craft's
along-track distance from `entity+0x91c`, subtracts them, and wraps the
difference around the lap:

```c
fVar6 = *(float *)(param_1 + 0x91c);      // self
fVar5 = *(float *)(param_2 + 0x91c);      // other
...
uVar1 = FUN_0883d3c0(fVar5 - fVar6, *puVar3);   // wrap by the course length
```

So the rule is **along-track gap over straight-line range must be under 1.4**.
Confidence 90 on the arithmetic. The *purpose* is a reading at 80: it rejects a
craft that is close in space and far away along the road, which is the hairpin
case - the craft coming the other way is thirty units from the nose and three
hundred units of tarmac away, and a missile locked to it flies into the barrier.

### The one condition not ported

`weapon_record+0x12c` must be `<= 0.0`. It is **a count of hits landed on a craft
but not yet resolved**, not a timer: the missile's own impact bookkeeping
(`FUN_08869054`) increments it by `1.0` while adding the weapon's `damage` into
`+0x120` and its `slowdown_time` into `+0x130`, and setting `+0x138`/`+0x13c`.
So the original refuses to lock a craft that is already taking a hit this frame.

This engine applies a blast the moment it lands, so it has no deferred-damage
queue for such a count to count. The condition is recorded and deliberately not
ported; `oag_gameplay::projectile::missile::lock` says so at its definition.

**That function is also the consumer `<Global slowdown_limit>` has been missing.**
`docs/gameplay/pickups.md` records the slowdown mechanic as having none;
`+0x130` is where `slowdown_time` accumulates. Not chased further here.

## The guidance law

`Missile_Update` (`0x0885a918`), called per tick from the missile pool's own
update (`FUN_08869588`). Confidence **95** on the arithmetic.

```text
to_target = target.pos - self.pos          // self.pos is start-of-tick
d    = normalize(to_target)
v    = normalize(self.velocity)
err  = d - v
step = normalize(err) * (dt * 4.0)         // the only steering constant
move = if |step|^2 <= |err|^2 { step } else { err }
dir  = v + move                            // NOT renormalised
self.velocity = dir * speed_kmh * 0x3e8e38e4
```

Five properties that a port gets wrong by default:

1. **No trigonometry at all.** Nothing in the guidance path calls `vsin_s`,
   `vcos_s` or `vcst_s(5)` - it is a chord-clamped move-towards on the unit
   direction. Convenient here for a reason the original never cared about:
   [determinism.md](../../../architecture/determinism.md) forbids platform
   transcendentals in simulation code.
2. **The clamp is on a chord, not an angle.** A missile nearly lined up takes the
   whole error in one tick (the `else` arm); one pointing away turns at a
   constant rate.
3. **`dir` is not renormalised, so a turning missile flies *slower* than its
   pinned speed.** `normalize(d - v)` has a dot product of `-sin(θ/2)` with `v`,
   so the correction always points partly backwards: `|dir|` is `0.954` at a
   right-angle turn and `0.933` at a reversal, returning to one as it lines up.
   **An earlier reading of this function recorded the opposite** - that a turning
   missile was about 0.2 % *faster* - and the arithmetic says otherwise;
   `a_turning_missile_flies_slower_than_its_pinned_speed` is that correction as a
   test.
4. **It is applied one tick late.** The block runs after the candidate position is
   already committed and writes only the velocity.
5. **It is skipped entirely on the tick the missile bounces.**

`0x3e8e38e4` is the correctly-rounded `f32` for `1.0 / 3.6`. An earlier reading
claimed it was not - that the nearest float was `0x3e8e3924` - and that is simply
wrong; `0x3e8e3924` is a *different, less accurate* value. What is genuinely two
things is the **operation**: the guidance path multiplies by the reciprocal and
the surface-contact path divides by `3.6`, and `x * (1/3.6)` and `x / 3.6` are
not the same `f32` function.

### The target is fixed at launch

`Missile_Init` copies the target pointer into `self+0xe0`. `Missile_Update` reads
it twice and **writes it never**: there is no re-targeting, no per-tick range
re-check, and no give-up when the target gets away. A zero pointer simply skips
the whole block and the missile flies ballistically.

## The speed ramp

`Missile_SpeedNow` (`0x0885a038`) returns the speed in **km/h**:

```text
age < 1.0  ->  launch_kmh * (1 - age) + class_kmh * age
age >= 1.0 ->  class_kmh
```

The `1.0` is a **code literal**, not the authored `slowdown_time` - the two both
read `1.0` on the shipped disc, which is a coincidence worth naming so nobody
wires them together.

`launch_kmh` is set once by `Missile_Init`:

```c
*(float *)(self + 0x48) = vsqrt_s(vdot_t(dir, dir)) * 3.6 + stats[+0x44];
```

that is, **the firing craft's own speed in km/h plus `launchSpeed`**, with a
`vmax_s(_, 14.4)` floor applied to the value that becomes the initial velocity
but *not* to the stored ramp base.

**This is the consumer `launchSpeed` was missing.**
`crates/gameplay/src/projectile.rs` records, of the Rocket, that "what
`launchSpeed` *is* for has not been found". For the Missile it is an additive
muzzle velocity over the launcher, and the class speed is where the ramp *ends*.
**Not propagated to the Rocket**, which has its own `Rocket_Init` and has not been
re-read.

**The speed is pinned every tick, never integrated.** Both velocity writes
normalise and rescale to this, which has a consequence that looks like a bug: the
`dt * 50.0` fall term contributes only a direction change, never a magnitude one.

## The flight model: the Rocket's, with one literal changed

Same probe, same ride height, same fall - and a **12.0** probe length where
`Rocket_Update` uses `6.0` (`0x41400000`). Confidence **92**.

The difference that matters is what happens at a wall. `Missile_Update` keeps a
counter at `self+0x6c`:

```c
if (code == 0) {                              // a wall
    if (++self->bounces < 5) {
        impulse = normal * (-2.0 * dot(velocity, normal));
        velocity += impulse;                  // perfect mirror, no restitution loss
        position  = hit_point + normal * 0.1;
        // homing suppressed this tick
    } else {
        self->flags |= 0x14;                  // give up
    }
} else if (code == 4) {
    self->flags |= 0x14;                      // detonate
}
```

So **a missile glances off walls up to five times where a rocket detonates on its
first**, and the bounce fires `WO_MISSILE_BOUNCE` with the `MISSILEEXPWALL` cue.
The mirror is lossless and the push-off is `0.1` (`0x3dcccccd`).

Collision code `4` comes from a second, separate query (`FUN_08831948`) rather
than from the surface's own material code; reading it as "hit a craft" fits every
site but is **inference at 55**, and Ghidra types that helper as returning `void`
while its caller consumes a value, so the code-4 path itself is only at 80.

## A press with no lock still fires

**Recovered, confidence 90**, and this page previously implied the opposite -
`crates/game/src/race/weapons.rs` declined the shot and kept the pickup, which
was ours.

`Ship_FireHeldWeapon` (`0x08844ae8`) branches on the lock and calls
`Weapon_RequestFire` on **both** arms:

```c
if ((self->target == -1) || ((self->lock_flags & 1) == 0)) {   // +0x85c, +0x860
    Weapon_RequestFire(craft, pose, 0, 0xffffffff);            // null target, index -1
} else {
    Weapon_RequestFire(craft, pose, entity_table[self->target] + 0x794);
}
```

`Weapon_FireMissile` then clears `craft+0x1bc` **before** its `live < 0x2e`
bounds check, so the pickup is spent even when the pool is full, and hands
`craft+0x160` to `Missile_Init` without testing it. `Missile_Update` skips its
whole guidance block on `self+0xe0 == 0` - already recorded under
[The target is fixed at launch](#the-target-is-fixed-at-launch) - so the missile
rides the floor, glances off walls up to five times, and otherwise flies
ballistically.

**`entity+0x85c` has exactly three consumers in the whole executable**, by
exhaustive operand search: `Craft_Construct_q` sets it to `-1`,
`Ship_AcquireLock` writes it, `Ship_FireHeldWeapon` reads it. So nothing else in
the game reads the lock - whatever draws the sight reads the mirror at
`weapon_record+0x1b4` instead, and that has not been chased.

## And it ends itself after three seconds

**Recovered, confidence 90.** `MissilePool_Update` (`0x08869588`) runs a second
pass over every live slot after updating it:

```c
if ((3.0 < missile->age) && ((missile->flags & 1) != 0)) {
    missile->flags |= 4;                    // the destroy bit
    ...                                     // trail released, a cue played
}
if ((missile->flags & 4) != 0) {
    Missile_SpawnExplosion(pool, &missile->position);   // FUN_08868d50, fourcc MIEX
    ...                                     // slot swapped out of the live range
}
```

`missile+0x50` is **the same field `Missile_SpeedNow` ramps on** - that function
reads `+0x50` and nothing else for its `age < 1.0` blend, and `Missile_Update`
accumulates it by `dt` at the top of its own body. One field, two consumers, so
the ramp's age and the timeout's age cannot drift apart. That is what puts this
at 90 rather than at "a float compared against 3.0".

Bit `4` is shared: the wall and craft paths reach the same teardown through
`flags |= 0x14` and `|= 0x24`. So the expiry is a **detonation**, not a reap.

The `flags & 1` half is *locally simulated*, not *alive*:
`MissilePool_SpawnRemote_q` (`0x088687c0`) initialises the same word to `2`
instead and then fast-forwards the missile by the message's latency, so a remote
craft's missile is ended by its owner rather than by this timer.

**The Rocket has the same shape with different numbers.** `RocketPool_Update`
(`0x0886de60`) tests `5.0 < rocket+0x48` and takes its destroy branch - but that
branch reaches the trail release and **no** explosion spawner, so a stale rocket
vanishes where a stale missile goes off.

## The blast, and what does not reach it

**Confidence 88, and the method matters more than the reading**: both halves of a
missile's damage have exactly two callers each, by exhaustive operand search over
all 524,719 instructions rather than by following the decompiler.

| Function | What it does | Called from |
| --- | --- | --- |
| `FUN_08869054` | damage bookkeeping - `weapon_record+0x120`, the `+0x12c` pending count, `slowdown_time` into `+0x130` | `MissilePool_TestCraftHits`, `MissilePool_DestroyRemote_q` |
| `Missile_ApplyBlastForce` (`0x08868ea4`) | the impulse, below | the same two |
| `Missile_SpawnExplosion` (`0x08868d50`) | the `MIEX` explosion object at a position | `MissilePool_Update`, `MissilePool_DestroyRemote_q` |

So **`MissilePool_Update`'s teardown reaches the explosion and neither the damage
nor the force**. A missile that runs out of time, or that gives up on its fifth
wall, *looks* like it went off and hurts nobody.

The force itself, which corrects a live claim in
[pickups.md](../../../gameplay/pickups.md):

```c
for (i = 0; i < ship_count; i++) {
    if (i == struck) continue;                       // the direct hit is excluded
    d = craft[i].position - point;
    if (length(d) < stats[+0x48]) {                  // blastradius
        craft[i].impulse += normalize(d)
                          * (1.0 - length(d)/stats[+0x48]) * stats[+0x4c];  // blastforce
    }
}
```

**Linear falloff to nothing at the radius**, and the *damage* does not go through
it at all. `oag_gameplay::projectile::blast` spends both flat and includes the
firer; both are recorded there as ours and are now known to be wrong rather than
merely unevidenced. Not changed in the same pass, because it moves how every
weapon lands.

### One thing that disagrees, left standing

`MissilePool_DestroyRemote_q` (`0x08868a10`) handles the "somebody else's missile
died" message. Its `message+8 == -1` branch - died on nothing, no craft struck -
**does** call `Missile_ApplyBlastForce`, where the local expiry does not. Either
the network path is compensating for something the local path does elsewhere, or
one of the two is a bug in the original. Nothing here decides it, and the local
path is what is ported.

### By-catch: a cue that reads wrong

The expiry branch plays the cue at `_DAT_00278950`, and that pointer resolves to
**`"SHURIKENEXPL"`** - checked twice against a wider read of `0x08a79b90`, where
the neighbouring pointers resolve correctly to `"MISSILEEXPWALL"` and
`"MISSILEEXPSHIP"`. Confidence 60 that this is a copy-paste in the original
rather than a misread. Not load-bearing: nothing here plays a cue on that path.

## The lock-on sight, and the lock flag's writer

By-catch of the same pass, chased to the end on its own page:
[lock-sight.md](lock-sight.md). In short - `BOOT.BIN` holds one contiguous run of
sight **widget** names at `0x08a79cd4`, nine of them over **three** models
(`missile_sight_1` ... `_4` all instance `missile_sight_outer.vex`), and
`HudSight_Update` (`0x0881dbcc`) both places them and **writes
`entity+0x860 & 1`** - the flag `Ship_FireHeldWeapon` gates the lock on. So the
lock is not instant: it needs `0.8` seconds of holding a target on screen.

The tone is `~ROCKLOCK`, one voice started once and switched between a seeking
and a locked variant by a parameter (`HudSight_UpdateTone`, `0x0881b34c`).

## A correction: `craft+0x1bc` is not a target

[weapon-fire.md](weapon-fire.md) reads `craft+0x1bc = -1` in `Weapon_FireRocket`
as `craft->target = -1`. **It is the held-weapon slot being emptied**, and `-1`
means "carrying nothing".

The evidence is two two-instruction functions that arm a pickup, writing a weapon
*id* to the same offset at the same base:

```text
0886e0f8: sw zero,0x1bc(a1)    ; WeaponPickup_ArmRocket  - id 0
0886e100: _sw zero,0x1c0(a1)

088685bc: li a0,0x1
088685c0: sw a0,0x1bc(a1)      ; WeaponPickup_ArmMissile - id 1
088685c8: _sw a0,0x1c0(a1)
```

and `Weapon_RequestFire` switching on `craft+0x1bc` to pick which bit to set.
`FUN_088629f8`, which resets a whole weapon record, sets both `+0x1bc` and
`+0x1c0` to `-1`.

**So the Missile's lock is not on the craft** at that offset - it is at
`entity+0x85c`, written by `Ship_AcquireLock`, and passed to the missile via
`craft+0x160`.

`+0x1c0` holds the same id and is *not* cleared on firing; the AI's grant path
reads it as "the last weapon this craft was given" to avoid handing out the same
one twice running. Note this is a **different base** from the `craft+0x1c0` flag
word [engine.md](engine.md) documents with eleven bits in use - a whole-word store
of `1` would clobber those - and the two must not be conflated.

## By-catch: the pickup grant, and it is `<Pickupodds>`

`WeaponPickup_Grant` (`0x08861d20`) is called from `Weapons_DispatchFire` on the
branch guarded by `craft+0x1c4`, right after a `WEAPONPICKUP` sound. It is the
call site [pickups.md](../../../gameplay/pickups.md) has recorded across three
passes as not existing.

**It walks `<Pickupodds>`, and that is measured rather than inferred.**
Confidence **92**. Two independent things say so:

1. `WeaponStats_ParsePickupOdds` (`0x0880e93c`) - the `<Pickupodds>` sub-parser,
   reached from `WeaponStats_Parse` after it matches the `Pickupodds` element and
   stores the class index to `+0x728` - writes every weight to
   `base + class*4 + weapon_base + attribute_offset`, and its thirteen
   `weapon_base` values are **exactly** the offsets the grant walks.
2. `WeaponStats_Parse`'s own tail sums those same thirteen offsets into `+0x4b8`,
   which is **the total the grant divides by**, and does it for four columns and
   four classes.

### The layout

Each weapon owns `0x40` bytes: four attributes `0x10` apart, each a four-float
array indexed by speed class. So a weight is
`stats + weapon_base + attribute*0x10 + class*4`.

| Attribute | Offset | | Weapon | Base | | Weapon | Base |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `ai` | `+0x00` | | Bomb | `0x178` | | Quake | `0x2b8` |
| `human` | `+0x10` | | Rocket | `0x1b8` | | Turbo | `0x2f8` |
| `front` | `+0x20` | | Cannon | `0x1f8` | | Shield | `0x338` |
| `back` | `+0x30` | | Mine | `0x238` | | Plasma | `0x378` |
| | | | Missile | `0x278` | | Autopilot | `0x3b8` |
| | | | LeachBeam | `0x3f8` | | Repulser | `0x438` |
| | | | Shuriken | `0x478` | | *totals* | `0x4b8` |

The four precomputed totals sit at `0x4b8`, `0x4c8`, `0x4d8`, `0x4e8` - one per
attribute column, each again a four-float array by class. `DAT_08b31040` is the
live speed class throughout this subsystem, the same global `Missile_SpeedNow`
indexes its four class speeds with.

### The draw, and which branch is whose

```c
roll = rand() % total;
// then a cumulative walk, first weight over the roll wins
```

`craft+0x23c` selects between two paths, and **the naming matters because the
obvious reading is backwards**. One path reads the `ai` column flat; the other
reads `human` and adds a race-position blend:

```c
t = (place - 1) / ship_count;
weight = human + back * t + front * (1 - t);
```

So the path that consults `front`/`back` is the **human's**, not the AI's - the
column it starts from is `human`. `t` is `0` for the leader, who therefore gets
`front` added, and approaches `1` for the tail, who gets `back`. The shipped
Venom table makes the intent plain: Shield authors `front="2" back="0"` and Turbo
`back="2" front="0"`, so a player in front is likelier to draw a Shield and a
player at the back likelier to draw a Turbo. **The pickup draw rubber-bands, and
it rubber-bands the player rather than the field.**

Confidence 92 on the arithmetic, 85 on `craft+0x23c != 0` meaning "human" - that
rests on the column names rather than on a read of the writer.

Both paths also refuse to hand out the same weapon twice running, comparing
against the last one at `craft+0x230` (which is `weapon_record+0x1c0`, the
second copy of the held id described above) and looping until they draw
something else.

### What this retires

Three rows of [pickups.md](../../../gameplay/pickups.md)'s recovered-versus-ours
table were **ours** on the grounds that no grant existed to read:

- that a pad crossing grants anything at all;
- the weighted draw;
- the inventory being one slot.

All three are recovered now. **The implementation in `oag_gameplay::pickup` was
not changed in the same pass** - matching the original means adding the
front/back blend, the no-repeat rule and the `ai`-versus-`human` split, and that
moves the world hash for a reason that has nothing to do with the Missile.

## What is still ours

Short, and each is labelled where it lives:

- **The launch offset.** The spawn point is pushed out to the nose by the hull's
  own extent; the original spawns at the craft's pose.
- **A slot index where the original keeps a pointer**, so the target survives
  being inside a `Copy` world snapshot.
- **The lifetime cap.** The original's missile has none - only the bounce budget -
  so `MAX_FLIGHT_SECONDS` is the same safety net the Rocket already had.
- **Wall versus floor as a geometric test.** Inherited from the Rocket's model;
  this engine's raycaster returns no collision code, so `WALL_FACING` and
  `RIDEABLE_COS` stand in for one. See `crates/gameplay/src/projectile.rs`.
- **Not firing at all when nothing locks.** The original's behaviour on an
  unlocked press has not been read.
- **The pending-hit condition of the lock**, dropped for the reason above.
