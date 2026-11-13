# What a Rocket looks like: the model, the three effects, and the flight

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse, PSP), image base `0x08804000` |
| **Subsystem** | weapons - the Rocket's presentation |
| **Related** | [`weapon-fire.md`](weapon-fire.md) (the fire word and the fan), [`particle-system.md`](particle-system.md) (the `.pob` interpreter these effects run through), [`exhaust.md`](exhaust.md) (the node classes), [`../../../formats/pob.md`](../../../formats/pob.md) (the particle container), [`../../../formats/vex.md`](../../../formats/vex.md) (the class-ID table) |

[`weapon-fire.md`](weapon-fire.md) answered *how many* rockets a press puts in the
air. This page answers what each one **is**: a textured model, three separately
authored particle systems, a looping sound, and a flight path that follows the
track rather than a straight line.

Verified live in PPSSPP as well as read statically - see
[Runtime verification](#runtime-verification) - so the claims here that the
emulator exercised sit above the decompilation-only ceiling of 84.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0885cc24` | `Rocket_Ctor` | 85 |
| `0x0885cdb8` | `Rocket_Init` | 85 |
| `0x0885d1b0` | `Rocket_SpeedForClass` | 84 |
| `0x0885d2a8` | `Rocket_Update` | 88 |
| `0x0886f038` | `Rocket_Spawn` | 88 |
| `0x0886ebdc` | `Rocket_HitCraft` | 88 |
| `0x0886ed34` | `Rocket_SpawnCraftExplosion_q` | 78 |
| `0x08915484` | `Psys_Spawn_q` | 72 |
| `0x0886e7ac` | `Rocket_SweepCraftHit` | 88 |
| `0x0886ee88` | `Rocket_ApplyBlastForce` | 90 |
| `0x0886f154` | `Rocket_SweepProjectiles` | 78 |
| `0x08a6b820` | `Math_BuildAxisAngleMatrix` | 92 |
| `0x08a6b6b4` | `Math_RotateByAxisAngle` | 90 |

`Rocket_HitCraft` lost its `_q` on 2026-09-16, when its credit and its
caller were read in full - see "What a rocket hit spends" below.

## First: `weapon-fire.md` has the spawn helper at the wrong address

**`Rocket_Spawn_q` is recorded there as `0x0886b038`. It is `0x0886f038`**, and
the cause is that page's own documented trap catching the page that documents
it. `Weapon_FireRocket` contains `jal 0x0006b038`, which is *relative*:

```text
0886e17c: jal 0x0006b038      ; 0x08804000 + 0x0006b038 = 0x0886f038
```

The two functions are not related:

- **`0x0886f038`** takes a pool slot at `world+0x64`, stamps the owner index and
  a monotonic serial, calls the constructor and bumps the live count. It takes
  four arguments, matching the call. This is the spawn.
- **`0x0886b038` is not even a function entry.** It is an address *inside*
  `FUN_0886afb8` (body `0x0886afb8`-`0x0886b457`), which is a
  segment-versus-craft sweep taking two arguments. It reads the same projectile
  pool, so it is weapons code, but it is not a spawn - and the name was hung on
  an interior address, which is why `scripts/audit-ghidra-names.py` reported the
  live name at `0x0886afb8` while `names.tsv` claimed `0x0886b038`. The row and
  the database disagreed and neither was right.

That function is left **unnamed**: what it is has not been established to 50, and
this project's rule is to write the hypothesis down rather than guess a name.

`search_instructions jal 0x0006b038` returns **exactly three call sites, all
inside `Weapon_FireRocket`** (`0x0886e17c`, `0x0886e2e8`, `0x0886e440`), which is
the "three spawns, one invocation" that page already established, now pointing at
the function that actually does it.

`names.tsv` row for `0x0886b038` is corrected by the same change as this page.

## The rocket is a model, not a billboard

`Rocket_Ctor` (`0x0885cc24`) allocates a `0x1d0`-byte scene node and hands it a
`.vex` file:

```c
func_0x0010eb80(node, 0x2780e8, 0x45000000, 0xfdb2, 0x3e9, 0);
//                    ^ string  ^ 2048.0f          ^ vex class id
*(int *)(self + 0x110) = node;     // the handle Rocket_Update drives
```

`0x2780e8` is relative; the string at `0x08a7c0e8` is **`Data\Weapons\Rocket.vex`**.
`0x3e9` is a `.vex` node class id, in the band [`vex.md`](../../../formats/vex.md)
already tabulates (`0x3bf` Engine Flare, `0x3c8` Trail, `0x3e2` Ship Muzzle).

**Confirmed against the disc, not only the executable.** The entry exists, and
our own parser decodes it:

```sh
cargo run -q -p oag-view -- \
  "data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad" \
  --mesh 'Data\Weapons\Rocket.vex' --screenshot /tmp/rocket-mesh.png
# Data\Weapons\Rocket.vex: 1 meshes, 84 vertices, 28 triangles, radius 1.34
```

It is a finned dart, white with red banding - a needle nose, a long tapered body,
one dorsal fin and one ventral fin.

This is the single largest mismatch with `oag_gameplay`/`oag_render`, which draw
a camera-facing additive billboard of half-size `1.5`
(`PROJECTILE_SPRITE_HALF_SIZE`, `crates/raceplay/src/lib.rs`). The *scale* is close;
the primitive is wrong.

## It is oriented to velocity and to the track, every tick

The tail of `Rocket_Update` (`0x0885d2a8`) rebuilds a basis and pushes it at the
node:

| Offset | What |
| --- | --- |
| `+0x80` | normalised velocity - forward |
| `+0x70` | the stored surface normal, re-orthogonalised against forward |
| `+0x60` | their cross product |
| `+0xa0`..`+0xdc` | the assembled matrix, then rotated |

```c
func_0x002676b4(0xbfc90fdb, m, m);          // 0xbfc90fdb = -1.5707964f = -pi/2
func_0x00141284(*(self + 0x110), m, 0);     // hand it to the node
```

So the model is aligned to **where it is going and what it is flying over**.

~~**The axis of that quarter-turn is not resolved here.**~~ **Resolved
2026-09-24, and the quarter-turn is not the model's at all** - it orients the
`WO_ROCKET_FLARE` emitter. `0x08a6b6b4` is not an import stub; it is
`Math_RotateByAxisAngle`, and the table above was reading the call's argument
order wrong. See
[2026-09-24: the quarter-turn belongs to the flare](#2026-09-24-the-quarter-turn-belongs-to-the-flare-and-the-basis-is-measured).

## Three particle systems, and which fires when

All three names are literals in `BOOT.BIN`, as bare names and as
`Data\Psys\*.POB` paths, and all three go through one helper -
`Psys_Spawn_q` (`0x08915484`) - whose second argument is the name and whose third
is a four-character tag, little-endian.

| Effect | String | Tag | Spawned by | When |
| --- | --- | --- | --- | --- |
| `WO_ROCKET_FLARE` | `0x08a7c100` | `ROFL` | `Rocket_Init` `0x0885cdb8` | at launch, carried by the rocket |
| `WO_ROCKET_EXPLO_TRACK` | `0x08a7c110` | `ROD2` | `Rocket_Update` `0x0885d2a8`, at both of its collision branches | the rocket hits track geometry |
| `WO_ROCKET_EXPLO` | `0x08a7ca74` | `ROEX` | `Rocket_SpawnCraftExplosion_q` `0x0886ed34` | the rocket hits a **craft** |

`0x4c464f52` is `ROFL`, `0x32444f52` is `ROD2`, `0x58454f52` is `ROEX`.

**The two explosions are separately authored, and the split is confirmed rather
than inferred from the names.** `Rocket_HitCraft` (`0x0886ebdc`) is the
craft-hit path - it credits `damage` and `slowdown_time`, then calls the `ROEX`
spawner - while both of `Rocket_Update`'s detonating branches call `ROD2`.

Two details of the craft-hit explosion worth not losing:

- **It is not drawn at the impact point.** The position is the *struck craft's*
  own (`craft+0x90`) with **`y - 2.5`**.
- **It is conditional.** Only spawned when `func_0x0003a37c(craft+0xf0)` is
  non-zero - an activity or visibility test that is not read here.

**Superseded 2026-08-12: the parameters are readable now.** When this page was
written, only `WO_ROCKET_FLARE`'s *name* was recovered, on the grounds that a
`.pob`'s payload was undecoded. That was too broad a claim even then - it is
the *slot-resolved* record that is unread, not the emitter record - and the
emitter tree is now parsed and played
([`pob.md`](../../../formats/pob.md), "The parser walks the tree"). All three
of the rocket's effects come out whole:

| effect | emitters |
| --- | --- |
| `WO_ROCKET_FLARE` | `WO_ROCKET_FLARE` (2 per tick for 100 ticks, 30-tick life, 4x4 atlas, random frame), `WO_ROCKET_SHAZZAM` |
| `WO_ROCKET_EXPLO_TRACK` | root glow, `fat_streaks`, `SMOKERING`, `Fire_Emitter`, and one more |
| `WO_ROCKET_EXPLO` | root, `SMOKEMUSHROOM` (per particle), `DEBRIS` (32 per tick), `SMOKERING`, `GLOW`, `FIREMUSHROOM_PARENT_GLOWS`, `FIREMUSHROOM` (per particle) |

What has **not** happened is wiring them into the renderer: `race.rs` still
draws the invented puffs described below. The blocker is no longer decoding
but pooling - see `HANDOVER.md`. The emulator observations that follow were
taken before any of this and stand on their own.

**One structural point the name list does settle**, though: the rocket has no
separate trail effect. The Shuriken authors both `WO_SHURIKEN_HEAD` and
`WO_SHURIKEN_TRAIL`, and the Missile a `WO_MISSILE_HEAD`; the rocket has only
`WO_ROCKET_FLARE`. So the one emitter draws both the glow at the nose and the
streak behind it, and a reimplementation that wants a smoke trail is filling in
that emitter rather than adding a second effect the original does not have.

## Flight follows the track

`Rocket_Update` runs two swept queries a tick through
`func_0x0002d98c(world, from, to, &point, &normal, self+0x114, 0)`:

1. a **surface probe** - from the projected position, `6.0` units along the
   stored normal `self+0x100` - run only while flag bit `4` is clear
2. the **flight sweep** - previous position `self+0xf0` to projected position

Both branch on the same return code:

| Code | What happens |
| --- | --- |
| `0x7f` | no hit: `velocity.y -= dt * 50.0`, so **the rocket falls** |
| `0` or `4` | detonate: flags `|= 0x14`, spawn `WO_ROCKET_EXPLO_TRACK` |
| anything else | **deflect**: adopt the hit normal, push out `3.0` along it, recompute velocity from the corrected position, renormalise and rescale to speed |

A rocket therefore skims the surface, glances off walls and drops when it runs
out of track - it is not a straight-line projectile.

**Now implemented, in its own commit.** `oag_weapons::projectile` probes toward
the surface each tick, rides `3.0` above what it finds, turns its velocity
parallel to it, falls at `50.0` when it finds nothing, and stops only on a craft
or a wall. It is weapon-agnostic, so the Missile and the rest inherit it.

**This was not a cosmetic gap.** Flying straight, a volley fired on a real track
**died in the tick it was fired** - measured, all three gone before the next
frame. That is what "they look like three dots and disappear" was, and it is why
the mesh work alone did not read as an improvement. After the change the same
volley flies for about half a second, climbing with the track as it goes, and
detonates on geometry.

**The code is the surface type, and this engine had it all along.**
`FUN_0883198c` - the query itself, `Collision_SweepSegment` below - read at
decompiler level 2026-09-13: it runs `Collision_RaycastWorld` over the segment
with the Reset skip on, and on a mesh hit returns **the struck collider's
`+0x6c` surface type** - the same `0` wall / `1` floor / `2` reset /
`3` mag floor enum `collision.md` recovered at 92 and `oag_physics::Surface`
carries. `0x7f` is its no-hit value, and `4` is a second query
(`FUN_08831948`, the craft test at `self+0x114`) overriding whatever the mesh
said. **Correction 2026-11-11:** the second query is the collision cage
on the segment's end, not a craft test, and `self+0x114` is the rocket's
AI-track cursor; see
["code `4` is the collision cage"](#2026-11-11-code-4-is-the-collision-cage-and-what-a-bank-does-to-every-flyer-weapon-bank-follow). So the three arms above are: **no hit → fall; wall or craft → detonate;
floor or mag floor → ride.** The same switch, verified in `Missile_Update`'s
decompile the same day, reads `0`/`4` on the *probe* as nothing at all - no
fall, no ride - and on the *travel segment* as bounce/detonate, with a floor
code pushing the missile to `hit + normal * g_ride_height` and nothing more.

Until 2026-09-13 `oag_weapons::projectile` decided wall-versus-floor from the
**angle** of the hit instead, under a comment claiming the raycaster returned no
code, and that was a bug on every title: a wall met at under ~15 degrees was
treated as floor clipping and ignored, so the rocket flew *through* the barrier
and off the circuit, and a floor met steeply after a crest was treated as a
wall and detonated. Measured and fixed in
[`../../../gameplay/projectile-floor.md`](../../../gameplay/projectile-floor.md).
One judgement call remains ours: on a floor hit across the travel segment the
velocity is turned parallel with its speed kept, where `Rocket_Update` writes
`(next - prev) / dt` and lets the next probe rescale it.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0883198c` | `Collision_SweepSegment` | 85 |

`Collision_SweepSegment(world, from, to, &point, &normal, bounds, flags)`: 85
rather than higher because the second query's "craft" reading is inherited
from `missile.md` at 55, and only the mesh-hit path (the `+0x6c` read through
`world + index * 0xc + 0x2454`, the collider table `collision.md` describes) is
read here at instruction level. Five callers, all the projectile updates plus
`Camera_UpdatePlayerView`, per `cannon-quake-leachbeam.md`'s xref sweep.

It moved the committed race hash, which is correct and was isolated in two steps
before the constants were touched - see `crates/gameplay/tests/determinism.rs`,
whose first step shows the **previous constants reproduce bit for bit** with the
change disabled.

## Authored projectile speeds are km/h

`Rocket_SpeedForClass` (`0x0885d1b0`) - a leaf Ghidra had not made a function -
is a pure table lookup with no arithmetic:

```text
class 0 -> lwc1 f0, 0x08(stats)    venomspeed
class 1 -> lwc1 f0, 0x0c(stats)    flashspeed
class 2 -> lwc1 f0, 0x10(stats)    rapierspeed
class 3 -> lwc1 f0, 0x14(stats)    phantomspeed
otherwise 0.0
```

Those offsets are exactly [`weapon-fire.md`](weapon-fire.md)'s, from an
independent read of the same binary. **That is corroboration but not a second
leg**: the rubric's 85-94 band wants a runtime trace or a *second binary*, and
two passes over one `BOOT.BIN` is the "consistent call sites" evidence already
counted. Nothing has measured what this function returns - rockets flying proves
it ran, not what came out - so it sits at the decompilation-only ceiling of
**84**.

**Both callers divide its result by 3.6** before it becomes a velocity -
`Rocket_Init` by the literal `0x3e8e38e4` (`0.2777778`), `Rocket_Update` by a
literal `3.6`. Nothing between the parse and the divide scales it. So the
authored speeds are **km/h**, and a reimplementation that spends them as
units-per-second flies the rocket **3.6x too fast**.

[`weapon-stats.md`](../../../formats/weapon-stats.md) records the attribute
names at confidence 92 but **does not state a unit**, so this adds to that page
rather than contradicting it.

**This one is now implemented**, in its own commit separate from the visuals:
`oag_weapons::projectile::launch` divides by
[`oag_core::math::SPEED_TO_KMH`](../../../../crates/core/src/math.rs) at the call
site, mirroring the original, which converts in the consumer rather than in the
lookup. A Venom rocket went from `1000` units/s - **3600 km/h** on our own HUD's
`* 3.6`, against a craft that tops out near 600 - to `278` units/s, or 1000
km/h.

It moves no committed hash. `crates/gameplay/tests/determinism.rs` spawns its
rocket with an explicit velocity rather than through `launch`, and
`crates/core/tests/determinism.rs` covers the generator, so **no reference
constant was touched** - which is the only acceptable outcome, per that test's
own standing instruction.

The flight path above is still **reported, not implemented**.

## What a rocket hit spends, and what a wall hit does not

Read 2026-09-16 from `RocketPool_Update` (`0x0886de60`) down, because the
port had been spending a uniform full-radius blast on every rocket ending
and the Plasma's reading ([plasma.md](plasma.md)) suggested that was wrong
for the Rocket too. It is.

`RocketPool_Update` runs `Rocket_Update` on every live rocket, then - only
while the rocket's flag bit `0` (in flight) is set - `Rocket_SweepCraftHit`
(`0x0886e7ac`): for every craft in the grid except the firer (`rocket+0x40`),
a hull-cylinder test between the rocket's previous and current positions,
**6.0 units** either side of the line of travel and bounded by the two
endpoints. A hit calls `Rocket_HitCraft(pool, rocket, craft)` (`0x0886ebdc`):

```c
rocket->flags |= 0x24;                                  // hit a craft, retire
if (Ship_HasActiveShield(craft)) craft->+0x124 = 1;     // absorbed marker, read by the shield path
craft->+0x120 += stats->damage;                         // +0x04 of the per-class rocket record
craft->+0x12c += 1.0;                                   // hits taken
craft->+0x130 += stats->slowdown_time;                  // +0x2c
craft->+0x138  = 0;
craft->+0x13c  = rocket->firer;
if (craft is visible) Rocket_SpawnCraftExplosion_q(...);  // ROEX, at craft y - 2.5
Rocket_ApplyBlastForce(pool, hit_point, firer);         // 0x0886ee88
```

`Rocket_ApplyBlastForce` is `Plasma_ApplyBlastForce`'s twin: every craft but
the firer within `blastradius` (`+0x1c`) receives `(1 - d/blastradius) *
blastforce` (`+0x20`) along the line from the hit point into
`entity+0x110`, the pending-impulse slot every blast function writes. It
carries no damage and no slowdown. So a rocket's damage lands on **the craft
it struck and nobody else**, and the radius is force only - the struck craft
takes both.

**A wall hit spends nothing.** `Rocket_Update`'s two detonating branches set
flag `0x14` (retire, wall); the pool's second pass then reaps any rocket with
bit `2` set - or older than **5.0 s** - by releasing its particle system
(`FUN_088f3298`), playing `ROCKEXPLWALL` (bit `0x10`) or `ROCKEXPLSHIP` (bit
`0x20`), clearing the trail, and swapping the slot out. No craft field is
touched on that path; `ROD2` was already spawned by `Rocket_Update` itself.
Confidence **90** for the split: both functions decompile cleanly, the
offsets are `WeaponStats_ParseRocket`'s own ([weapon-fire.md](weapon-fire.md)),
and the teardown is the same eleven-line shape as `Plasmas_Update`'s.

**2026-09-25: the timeout reap is genuinely silent, confirmed rather than
inferred.** `RocketPool_Update` itself (`0x0886de60`, confidence 90 - read
whole via `decompile_function` against `psp-pulse-usa`'s `BOOT.BIN`) is the
function both paragraphs above describe. Its despawn pass is:

```c
if (5.0 < age /* +0x48 */) flags |= 4;          // age-only reap sets *only* bit 0x4
if ((flags & 4) != 0) {
    FUN_088f3298(...);                          // release the particle system
    ...
    if ((flags & 0x10) == 0) {
        if ((flags & 0x20) != 0) Sound_Play(..., "ROCKEXPLSHIP", ...);
        // else: no Sound_Play call at all
    } else {
        Sound_Play(..., "ROCKEXPLWALL", ...);
    }
    ...
}
```

There is no third arm. The `if`/`else if` only ever chooses between
`ROCKEXPLWALL` and `ROCKEXPLSHIP`; a rocket that ages out without a hit sets
neither `0x10` nor `0x20`, so the whole `Sound_Play` block is skipped and the
teardown falls straight through to releasing the slot. The original plays
**nothing** on a Rocket time-out, matching what this port already does (see
`Cue::RocketHitWall`'s own doc comment in `crates/sound/src/sfx/cue.rs`).

**The teardown's own callees were read too, not just the branch that skips
`Sound_Play`.** `FUN_088f3298` (particle-system release) and
`FUN_08939bcc`/`FUN_08939460` (both reached off `iVar5 + 0x50`, the rocket's
own sound emitter - the same one `~ROCKETTVL` loops on) were each decompiled
in full: none of the three calls `Sound_Play` or reaches one transitively.
`FUN_08939460(emitter, 0)` copies the emitter's last-known world position out
of its attached node and clears the node pointer - a detach, not a stop
notification - so there is no cue hiding in the teardown's own call tree
either. **What actually silences `~ROCKETTVL` on a timeout is not a call in
this function at all**: `TravelVoices::tick`
(`crates/sound/src/sfx/travel.rs`) reads `flying`/`position` straight
off the world's own projectile array every tick, independent of whether an
`Impact` was written that tick, and `flight.rs`'s timeout branch resets the
slot to `Projectile::default()` - so the loop's own falling edge fires the
same tick the slot goes inactive, on the projectile's own liveness rather
than on any `Impact`. Confirmed by reading both sides, not assumed from the
port's own shape.

`Rocket_SweepProjectiles` (`0x0886f154`, confidence 78) is the third call
in the per-rocket loop: the same cylinder test against every live **mine**
(pool `DAT_08b3bf8c`, radius `+0x100` of the rocket record) and every live
**bomb** (pool `DAT_08b3bf90`, radius `+0xe0`), flagging both the rocket and
whatever it met for retirement - a rocket clears mines and bombs off the
track by flying into them - quietly, since neither pool's teardown credits
anyone ([mine.md](mine.md)'s 2026-09-16 section). **Ported the same day** as
`Projectiles::sweep_rockets_through_laid`: the mine or bomb reports an
ending with no blast, the rocket is spent without one.

**Ported the same day**: `oag_weapons::projectile::flight` gives the Rocket
the Plasma's two arms - a craft hit routes to `blast::blast_direct_hit`, a
wall hit carries `blast: false` - and `crates/gameplay/tests/determinism.rs`
moved its constants for it, with the scenario's target moved into the
rocket's own arc so the direct hit stays covered. The 5.0 s lifetime and
the mine/bomb sweep followed later the same day.

## Audio, in passing

`Rocket_Init` allocates a `0x70`-byte emitter, sets `+0x38 = 0x44160000`
(`600.0f`, a rolloff distance) and starts a looping sound at volume `1.0` named
by the string at `0x08a7c0d0`: **`~ROCKETTVL`**. The neighbouring `~PLASMATVL`
confirms the `<WEAPON>TVL` - travel - pattern.

**2026-09-23: wired.** `Cue::RocketTravel` (held, per projectile slot, the
measured `600.0` radius) and `Cue::RocketHitWall`/`Cue::RocketHitShip` (off
`Impact::struck`, on the same bolt emitter) are in
`crates/sound/src/sfx/cue.rs`. The 5.0 s pool-reap timeout this page's
own "What a rocket hit spends" section leaves silent turned out not to need
disambiguating: `crates/weapons/src/projectile/flight.rs`'s own Rocket
timeout branch resets the slot without writing an `Impact` at all, so the
port's own impacts loop never sees a `struck: None` from that path - only
from a real wall hit.

**2026-09-25: `ROCKET` itself (the launch, not the travel loop) wired too.**
This page's own text used to leave it "named only in passing" with no
address; `Ship_FireHeldWeapon` (`0x08844ae8`), decompiled whole for the
missile.md/autopilot.md conflict it settled, plays it - `Sound_Play(1.0,
*(param_1+0x50), weapons.bnk, 0, "ROCKET", 0)`, positional, on a held-id-0
press, local player only. See `autopilot.md`'s own "`Ship_FireHeldWeapon`
opens both cues" section for the switch body and cross-checks; `Cue::Rocket`
in `crates/sound/src/sfx/cue.rs`.

## Runtime verification

Done with `scripts/psp-fire-weapon.py`, written for this page, against PPSSPP
v1.20.4 under Xvfb and `pulse-psp-usa.chd`, in a live Single Race.

**The cheat is one word.** `Weapons_DispatchFire` reads a fire-request word per
craft and dispatches one handler per set bit, so setting bit `0x80` is exactly
equivalent to holding a Rocket and pressing fire - no need for the grant path,
which is still unread.

Two things had to be right, and each cost an attempt:

- **Writes only take while the CPU is stepping.** Written against a free-running
  emulator the word reads back set and no handler ever consumes it.
- **The craft the weapons code walks is not the craft `Ship_UpdateCraft` takes.**
  `psp-trace.py` learns a craft pointer from that function's `a0`; `+0x1b8` on
  *that* object is not the fire word. `Weapons_DispatchFire` walks an **inline
  array in the world**: `world + 0x70 + index * 0x1f0`, `_DAT_000577f8` entries,
  read straight off its loop induction. The world arrives in `a0` - the
  prototype's leading `float` rides in `f12` - confirmed live, `a1` is zero.

What the run established:

- Setting bit `0x80` on craft 0 was **consumed within the frame**, and
  `Weapon_FireRocket` is the only handler for that bit
  (`jal 0x0006a104` -> `0x0886e104` in the dispatch chain). This is the
  bit-to-handler mapping verified rather than read.
- A breakpoint at **`0x0885d2a8` then hit repeatedly**, with nothing else in the
  air. That is `Rocket_Update` running per rocket per tick, and it is a *direct*
  runtime leg - a breakpoint on the function itself - which is why that address
  sits at 88 rather than 82.
- **`Rocket_Spawn`'s runtime leg is indirect and is worth naming as such.** No
  breakpoint was set on `0x0886f038`. What the run shows is that rockets
  provably came into existence, and this is the only construction path, so it
  provably ran - an inference from a runtime observation rather than an
  observation of the function. Combined with three unambiguous call sites that
  supports 88; it would not support more.
- **What the frames show.** Rockets leave together and travel **low, hugging the
  track surface**, reading as small warm-orange elongated glows rather than
  white points - consistent with the surface probe above. On track impact they
  produce a **large orange fireball** sitting on the track surface, far bigger
  and far warmer than the white additive flash this engine draws.

**No frame from the original is committed.** Screenshots of the running game are
reproduction; they stayed outside the repository and `just audit-leakage` is what
enforces that. The description above is the deliverable, not the image.

To repeat it:

```sh
Xvfb :97 -screen 0 1280x720x24 &
printf '[General]\nRemoteDebuggerOnStartup = True\nRemoteDebuggerLocal = True\nRemoteISOPort = 47810\n' > /tmp/debugger.ini
DISPLAY=:97 SDL_VIDEODRIVER=x11 setsid PPSSPPSDL --appendconfig=/tmp/debugger.ini \
    --windowed data/images/pulse-psp-usa.chd < /dev/null &

uv run --with websocket-client scripts/psp-drive.py --port 47810 menu --single-race --any-track
uv run --with websocket-client scripts/psp-fire-weapon.py --port 47810 --shots /tmp \
    --freeze-at 0x0885d2a8 --freeze-hits 24 rocket
```

## The craft-hit blast against the original, measured 2026-09-24

A PPSSPP capture of the player's own Rocket hitting the craft ahead on
Talon's Junction's grid (51 units from the camera, `WO_ROCKET_EXPLO` via
`Rocket_SpawnCraftExplosion_q`) settles what the blast looks like at player
size, frame by frame after the spawn: a **full-screen yellow wash** for its
first frames (screen mean `(89, 96, 84)` to `(223, 225, 85)`), a
**white-hot core** about a quarter of the screen wide at its peak (8-12
frames), then a saturated orange, textured fireball with black debris
(16-25), a brown-then-black smoke mushroom (30-60) and a faint grey column
by 80. So the original's peak is white at the core: a white disc at that
moment is not by itself the bug. What was wrong in ours was everything
around it - no textured fire, because particles drew a procedural disc
instead of their own sprite. The wash is not a particle at all but
`ScreenFlash_Start` kind `0`, `(1, 1, 0, 0.6)` over 0.5 s. Its consumer
was read the same day and matches these frames' added colour frame by frame;
`oag_fx::flash` draws it on Pulse's PSP source. See
[particle-system.md](particle-system.md#the-screen-flashs-consumer-read-and-measured-2026-09-24). The frames stay outside the repository.

## What our own renderer has and has not been shown to do

The change that accompanies this page draws the model. Separating what was
checked from what was assumed, because the two got confused once already:

- **Checked.** The entry loads off a real disc in a real race, and the model's
  long axis is the one the engine aims down the velocity - `Rocket.vex` spans
  `x 1.071, y 0.929, z 2.684`, and the matrix maps model **+Z** onto the
  direction of travel. That is a ground-truth test
  (`the_rocket_model_is_longest_along_the_axis_it_is_flown_down`), and it is
  also the check that would catch the deferred quarter-turn mattering: if the
  nose ran along X, a fixed pre-rotation would be needed and the test would say
  so.
- **Not checked: a rendered frame with a rocket in it.** The headless capture
  path cannot make one - `--race` holds the throttle and does not steer, so the
  craft never reaches a `Weapon Pad` and never receives a pickup.

**And a mistake worth keeping.** Three pale chevrons in a `single_race` capture
were read as a fanned volley. They are **track scenery**: they render
identically in `time_trial`, where no rocket can exist. The lesson is the cheap
control that settles it - render the same tick with weapons off and diff, rather
than identify an effect by its shape.

## 2026-09-24: the quarter-turn belongs to the flare, and the basis is measured

Read on the bridge (`/pulse/BOOT-psp-pulse-usa.BIN`) and then measured live on
PPSSPP (`pulse-psp-usa.chd`, Single Race, the player's own record, index 7 of
the grid), with a breakpoint at `0x0885da00` - the instruction after the
rotation call returns.

**The two helpers.** `0x08a6b820` builds a 4x4 from an axis and an angle:
the angle in `$f12` (a float argument, so Ghidra's shown signature drops it),
the axis at `a1`, the result at `a0`. It wraps the angle to one turn with the
`(x * 2^25/2pi) << 7 >> 7` fixed-point trick, takes `vcos`/`vsin` (VFPU
angles are quarter-turns, hence the `2/PI` constant), and writes the Rodrigues
rows `(t x^2 + c, t x y - s z, t x z + s y)`, ... and `(0, 0, 0, 1)`.
`0x08a6b6b4` copies its `a1` vector to the stack, builds that matrix from it,
and `vmmul.t`s it into the 3x3 block at `a0`, keeping `a0`'s own `w` column.
`contact-response.md` and the PS2's `collision-shake.md` had already met the
same pair in the camera shake and read it the same way; neither had named it.

**The call site, instruction by instruction** (`Rocket_Update`,
`0x0885d8b4`..`0x0885d9fc`):

| Offset | Written | From |
| --- | --- | --- |
| `+0x80` row 2 | `f` | `+0xe0` (velocity) normalised |
| `+0x70` row 1 | `n` | `+0x100` (surface normal) minus its `f` component, normalised |
| `+0x60` row 0 | `n x f` | `vcrsp.t C230, C220, C200` at `0x0885d988` |
| `+0x90` row 3 | position | the stack, `w = 1.0` at `+0x9c` |

Then, in this order:

1. `jal 0x08945284` with `a1 = +0x60` - the model's scene node takes the
   matrix **before** anything rotates it. `0x08945284` copies all sixteen
   words into the node's own transform (`*(node + 0x3c) + 0x40`).
2. `+0x60..+0x9c` is copied to `+0xa0..+0xdc`.
3. `jal 0x08a6b6b4` with `$f12 = 0xbfc90fdb` (`-pi/2`) and **`a0 = a1 =
   +0xa0`** (the delay slot's `move a1, a0`): the copy is turned about **its
   own row 0**, in place.

And `+0xa0` is what `Rocket_Init` (`0x0885cfd0`) passed `Psys_Spawn_q` as
its frame, with flag `1` (`t0`). `PsysNode_Start` (`0x08916200`) stores a
flag-1 frame **as a pointer** (`node + 0x50 = frame`) instead of copying
it, so `WO_ROCKET_FLARE` reads `rocket+0xa0` live every frame - the
quarter-turned basis. The model never sees the quarter-turn.

**Measured**, twelve consecutive `Rocket_Update` calls across the three
rockets of one volley (`rocket-basis.json`,
not committed), every one agreeing to four places:

```text
node  (+0x60)  row0 ( 0.1672,  0.1428, -0.9755)   n x f
               row1 ( 0.0007,  0.9894,  0.1450)   n
               row2 ( 0.9859, -0.0249,  0.1653)   f = velocity / |velocity|
               det  +1.0000
flare (+0xa0)  row0 = node row0
               row1 = node row2      (the flare's +Y is the direction of travel)
               row2 = -node row1     (the flare's +Z points into the track)
               det  +1.0000
```

So the model is placed with a **rotation**, model `+X` onto `n x f`, `+Y` onto
the normal, `+Z` down the velocity - which is what
`Race::projectile_model_matrices` has built since this date, with
`Projectile::surface` (this engine's own analogue of `+0x100`) as `n`.
Before, that function built `forward x up` for the side axis, a reflection, and
drew every rocket mirrored. Confidence **92** on the node basis (read, and
measured to the element) and **90** on the flare frame (measured; which way
`vmmul.t` composes was not decoded, the live rows settle it).

**And the flare now rides that frame.** Until this date `WO_ROCKET_FLARE`
took a position only and `psys::Stage::advance` gave every riding effect world
`+Y` as its emitter up - so ours streaked straight up from each dart, where
the original's is a long orange glow stretched along the flight path.
`psys::Stage::orient` now carries a per-instance up, and the Rocket's flare
takes its velocity. Only `+Y` is carried: the psys port builds the other two
axes from `up` alone (`horizontal_basis`), so an emitter whose azimuth matters
would still differ from the original's `n x f`/`-n` pair.

## 2026-10-01: a matched-state comparison, and the flight law it measured

The 2026-09-24 side-by-side was not a matched state (the original at 140-237
km/h, ours at 48-100, so our volley met an opponent). This one is: the same
authored intent on both sides, the same ship (Venom, Assegai), the same
circuit (Talon's Junction, `16_Track`) and the same start straight.

**Method.** `scripts/psp-weapon-pair.py` (emulator side) holds `cross` through
the countdown (not a false start, see `race-modes.md`), calls the first frame
the player's throttle word (`craft+0x2b8`) is non-zero GO, and ORs the Rocket
bit `0x80` into the player's weapon record inside `Weapons_DispatchFire` 120
frames later - GO was stop frame 266-267 on every run, and the craft was at
**speed 106.2, x 124.5** at the fire on every restart (more than ten runs over three emulator boots, 106.05-106.22). Ours is
`verification/scenarios/weapon-after-go.inputs` with `oag-game --race --mode
time_trial --give rocket --size 480x272`. Ours reaches the same state (x
124.9, speed 105.8) on tick 403, not 392: our standing start is about ten
ticks slower off the line, which is a handling matter and is left alone. The
presented frame lags the stop by two frames (the first frame that differs from a
no-fire control is `fire+3` on both boots, RMSE 0.068 -> 0.11), so the original's
`k` is compared with our tick `fire + k - 2`. Three traps found on the way, each
now in the script: the race manager's player slot (`*(0x08b317b4) + 0x2c0`) is the
**ship entity**, not the craft `Ship_UpdateCraft` takes (`entity+0x94` is the
craft); a probe breakpoint nested inside the dispatch breakpoint's loop records
only its first frame; and an unmanaged Xvfb places the SDL window off-screen.

**Time Trial is a valid place to fire.** The original updates and detonates
rockets there like anywhere else (a first reading that they were frozen was the
nested-breakpoint trap above). It has no opponents and no weapon pickups, which
is exactly why it is the clean place to compare a free flight. The capture
frames and the probe JSON are under a scratch directory, not kept (not
committed).

**Measured, 3 rockets of one volley, two runs: identical to 0.1 unit.** (Ours: a temporary, uncommitted `eprintln!` of `world.projectiles.slots` after `race.tick` in `race/capture/tick.rs`, reverted; the original: `psp-weapon-pair.py --probe rocket`.) Every
`Rocket_Update` hit (`0x0885d2a8`) read `a0` = the rocket:

| | original | ours |
| --- | --- | --- |
| spawn position | the craft's own, to 0.1 (`124.5, -47.9, -196.9` against the body's `124.5, -47.9, -197.0`) | the nose: craft position plus the hull's extent |
| speed, frames 0-3 | **166.67 u/s (600 km/h)**, drifting to 166.59 | 277.78 u/s (1000 km/h) |
| speed, frame 4 on | **222.22 u/s (800 km/h) and held** | 277.78 |
| age at the speed step | `age` 0.0501 -> 0.0668 (the fourth update) | no step |
| ends | `WO_ROCKET_EXPLO_TRACK` at frame **51, 61, 70** after the fire, at **185, 221, 252 units** from the spawn, rising from y -47.9 to -39..-33 with the road | tick 14, 24, 32 after the fire, at 75-158 units |
| `WO_ROCKET_FLARE` spawns | three, at frame 0, nothing else at launch | three, at launch |

What this settles:

- **Cruise speed is the class speed alone: `venomspeed` 800 km/h = 222.22 u/s.**
  Ours flies `class + launchSpeed` = 1000 km/h. The source comment on
  `projectile::rocket::launch` already flagged the sum as ours; it is now
  measured against, twice (this, and the 2026-09-24 fx-brightness probe's
  "about 222"). `Rocket_SpeedForClass` is called by both `Rocket_Init` and
  `Rocket_Update` and is a pure function of two globals, so it returns the same
  800 at both.
- **The first four frames are slower, at exactly 0.75 x the class speed**
  (166.67 = 600 km/h), and the 0.75 is the craft's display-matrix scale, not
  `launchSpeed`. `Rocket_Init` copies the matrix `Weapon_FireRocket` hands it
  into `+0x60..+0x9c` and scales its row 2 (`a1+0x20`, the direction) by
  `SpeedForClass / 3.6`. The first `Rocket_Update` hit breaks at entry, before the
  rows are rebuilt, so those words are still Init's copy: read off all six rockets
  of two runs, **rows 0, 1 and 2 each have length 0.7500** (the same `g_craft_scale`
  the Mine probe found on the craft's anchor). So launch speed = class speed x
  0.75 for as long as the direction vector keeps that length, and the step to
  222.22 at the fourth update is the first surface-probe hit, where
  `Rocket_Update` renormalises to `SpeedForClass / 3.6` (the `0x4066 6666`
  divide at `0x0885d6c8`). `launchSpeed` plays no part. Confidence **88**: the
  mechanism is read and the 0.75 and both speeds are measured, but why the step
  lands on the fourth update (probe reach against the hover height) was not
  separated from the age, and the Flash/Rapier/Phantom values were not run.
- **The rocket is spawned at the craft's position, not at its nose.** A
  stationary Mine's `Mine_PoseNode` matrix was read at the craft's body position
  to the hundredth (mine.md, same date), and the Rocket agrees.

**Open rows - all four landed 2026-10-01, second pass (see the next section for the
commits, the third law the rows did not name, and what is still open):**

1. *Rocket cruise speed*: 222.22 u/s, not 277.78. Measured, 2 sources.
2. *Rocket launch speed*: the direction vector keeps the craft's display scale
   0.75, so the rocket leaves at 0.75 x class until its first surface hit sets
   the class speed. Measured on Venom; `launchSpeed` is not involved.
3. *Rocket spawn point*: the craft's position, not the nose. Measured.
4. *Rocket life against the track*: ours detonates 2-4 times sooner than the
   original's 51-70 frames. Not isolated: the speed, the spawn point and the
   fan's lateral drift each change it; re-measure after 1-3.

**What the picture shows at player size** (`pair-tt-*.png`, scratch): at fire+3
to fire+8 the original has a wide orange-white glow (about 40-60 px across at
480x272) lying on the craft's nose and along the road, which thins to a streak
by fire+20; ours has a small yellow spot (about 15 px) ahead of the nose and
three darts. Both sides draw `WO_ROCKET_FLARE` once per rocket with nothing else
at launch, so the difference is not a missing effect. It is consistent with rows
2 and 3 (the original's flare starts on top of the craft and travels slowly for
four frames, so the camera sits inside its first particles; ours starts nine
units ahead at 1.7 x the speed) but that is **not tested**: it needs the gameplay
rows changed first. Chosen, not measured: nothing here.

Two things in the frames that are not this lane's: the original's craft is
about 1.4 x larger on screen than ours at the same moment (**resolved
2026-10-01**: the original's fresh profile flies `OPT_CLOSE` and ours defaulted to
`far`; the default is now `close` and the sizes agree, see
[camera.md](camera.md#the-default-view-is-opt_close-measured-2026-10-01); the two
views do render differently, the flag does take effect),
and the held-weapon icon differs (the original's Time Trial shows its pad
indicator; ours shows the weapon, because `--give` refills the slot).

## 2026-10-01, second pass: the rows landed, the probe arm read, a volley matched

Rows 1-3 each landed in their own commit, and row 4 resolved into two more laws the
rows did not name. Method for the last: the craft is placed at the pose the original's
volley was measured at and the volley fired through `projectile::fire_rocket`
(`crates/raceplay/tests/rocket_launch_ground_truth.rs`, disc-backed), because our own
standing start lands the craft at z -199.4 heading 1.7 degrees toward the -Z wall where
the original's is at z -197.0, which alone moves the detonation by 30 ticks.

| Law | Source | Ours now |
| --- | --- | --- |
| cruise speed is the class's, 222.22 u/s on Venom | live, two probes | `rocket::launch` |
| launch speed is 0.75 x that until the first surface-probe hit | live (rows 0.7500) | `LAUNCH_SPEED_SCALE`; class speed rides in `Projectile::launch_speed_kmh` |
| spawn at the craft's own position | live, to 0.1 unit | `launch` |
| the stored normal `self+0x100` is seeded from the craft (`-(craft+0xb10)`), the first probe goes along it | decompiled `Rocket_Init`; live rows `(-0.03, 1.0, 0.05)`; `+0xb10` is the located track point's down, not the craft's up ("The launch normal is the track's, not the craft's") | `Projectiles::spawn_riding` seeds `Craft::launch_normal`: `Ship::track_up`, the up of the sample the step located the craft on, else the craft's up |
| **the probe-hit arm steers toward the ride point** | decompiled `Rocket_Update`, confidence **88** | `flight.rs`, Rocket only |

**The probe-hit arm, read at decompiler level 2026-10-01.** On a floor-class result the
rocket adopts the hit normal (`+0x10c = 0`), takes `to' = hit + 3.0 * n`, sets
`velocity = (to' - from) / dt`, **normalises it and scales it to
`Rocket_SpeedForClass / 3.6`**, then moves to `from + velocity * dt` - not onto `to'`
and not by turning the old velocity parallel to the surface, which is what this engine
did for every projectile. The travel-segment floor arm that follows writes
`(next - prev) / dt` unnormalised and leaves the rescale to the next tick's probe. Where
our port differed by turning the velocity parallel, a volley flew nearly straight and
met the side wall 20-30 ticks early; with the arm ported it follows the road.

**Result at the original's pose (same ship, same track, same start line):**

| shot | original, frames / units from spawn | ours before (rows 1-3 only) | ours now, ticks / units |
| --- | --- | --- | --- |
| -Z side | 51 / 185 | 31 / 110 | 49 / 177 |
| centre | 61 / 221 | 42 / 151 | 60 / 218 |
| +Z side | 70 / 252 | 52 / 187 | 69 / 251 |

The centre shot's last position is within about four units of the original's
(`341.9, -36.0, -174.9` against `345.4, -35.8, -172.5`). The test asserts three ticks
and twelve units. Confidence **85** on the arm and the result: read and matched on one
circuit, one ship, one start pose; the other three classes and any other track were not
run.

**What is still open, honestly.**

- **The first two or three updates are a probe miss in the original, cause unrecovered.**
  A live break on the instruction after `Rocket_Update`'s first `Collision_SweepSegment`
  (`0x0885d40c`, `psp-weapon-pair.py --probe sweep`) read `v0 = 0x7f` (no hit, the fall
  arm) for the first **two** updates in a capture fired at speed 57 and for the first
  **three** at the matched speed 106, then `1` (floor) every update after, with the floor
  about four units below the rocket and the probe six long. That is the 166.67 u/s phase:
  the arm that renormalises never runs, the rocket stays at 0.75 x class, and it falls
  at 50 u/s^2 for those updates (the live y rises 0.10, 0.08, 0.08 per frame, the fall
  arm's shape). Ours finds the floor on its first update, so it has no slow phase and its
  first three positions sit about 1.2 units lower than the original's. A temporary
  diagnostic that skipped the probe for the first three updates reproduced the original's
  first four positions and speeds to about 0.04 unit (before the craft-up seed; its
  ranges were not re-run with the seed), so the fall arm is the original's and the miss
  is real, but **that diagnostic was a timer and was not kept**: nothing here says it is
  age. `Collision_SweepSegment` returns `0x7f`
  when `Collision_RaycastWorld`'s hit kind (`local_38`) is not `1`; whether the start
  line's floor is authored as another kind in the original's collision world, or the
  query is gated on something the rocket copies from its owner (`self+0x114` is
  `*(craft+0xad8)`, the bounds `Collision_SweepSegment`'s second query takes), is not
  separated. Not reproduced and not chosen.
- **The wide orange glow at fire+3..+8 is still not matched.** (**Closed in size 2026-10-01**: see "the launch glow" below; the paragraph is what it said at the time.) Read at 480x272 beside the
  original: the original's is a
  large white-orange bloom over the craft's nose and the road ahead; ours is a smaller
  yellow glow at the nose in the middle two rows. Closer than the 15 px spot before the
  spawn moved, not equal. `WO_ROCKET_FLARE`'s parameters are the unread part (section
  above).
- **Our standing start differs from the original's** by 2.4 units laterally and 1.7
  degrees of heading at the same place on the same track (ours z -199.4, forward z
  -0.030; the original z -197.0, forward z -0.007). A spawn/handling matter outside this
  page, recorded because it moved a rocket's detonation by 30 ticks.

## 2026-10-01: the launch glow, the flare's 4-bit sprite and ageing

The wide orange glow at fire+3..+8 (the 2026-10-01 second pass left it open, ours
smaller) had two causes, both in how `WO_ROCKET_FLARE` was played, and neither in
the Rocket's own law. Same state as that pass: Venom, Assegai, Talon's Junction,
Time Trial, fire at speed 106.2, native 480x272, two boots of the original.

**What the original spawns at launch, read live** (`psp-weapon-pair.py --probe
flare|rolled`, the pool of each of the three rockets' instances, every frame):

| emitter | draw | per frame, per rocket | half-size | colour, alpha | sprite |
| --- | --- | --- | --- | --- | --- |
| `WO_ROCKET_FLARE` (root) | `ParticleSystem_DrawPoolSquares`, a square, no roll | 2 new a tick, 30-tick life | **1.05 at age 0, 1.28, 1.51 ... 2.67 at tick 7** = the size channel `lo 1 + (hi 8 - lo) * (0.0068 .. 1)` | (245,245,191,255) at age 0, to (179,105,0,195) at tick 7: the colour table, white to orange | a 4x4 atlas cell, random per particle |
| `WO_ROCKET_SHAZZAM` | `ParticleSystem_DrawRolledQuads`, rolled | **1 new a tick, drawn once or twice** (22 of 39 particles twice) | **5..15 random, 6.2 to 14.8 read**, held for its life | (255,255,255,255) | `muzzle_flare_pSprite_64x64_ADD`, a soft orange star |

Particles stay where they spawn (the emitter flies on, so they string out behind
the rocket: the oldest at x 122.6, the newest at 151.4 at tick 7 of a rocket
moving 3.7 units a tick), and the instance scale words `+0x28..` read `1.0`. The
quads are 10 to 30 units across at the rocket, which is where the wide glow comes
from: **`WO_ROCKET_SHAZZAM` is the glow**, one big soft flash a tick on every
rocket, and the ring sprites are the fine orange structure round it.

**Cause 1: the flare's sprite was not a disc.** `WO_ROCKET_FLARE`'s root has an
embedded sprite like the other 29, a 128x64, 4-level, **4 bits a pixel** header (a
4x4 atlas of ragged orange rings, a black-to-orange 16-entry palette, alpha 255), and
`oag_pob::texture::parse_at` refused every header whose depth was not 8, so the
root drew the procedural radial disc: white and bright at the core where the
original draws rings. Found independently on this lane and on `pulse-wreck-2` the same
day (from a Bomb's `FIRE`, there); the wreck lane's reader merged first and is the one
on main - see [`pob.md`](../../../formats/pob.md) for the layout, the twelve 4-bit
sprites and the six roots that looked textureless.

**Cause 2: a pool particle's spawn tick aged it.** Ours spawned a particle and
integrated it in the same tick, so its first draw was a tick old and a particle with a
life of one tick or less was never drawn at all. `WO_ROCKET_SHAZZAM` authors `1 +- 1`
ticks, so about half of every rocket's flashes never appeared and the rest showed on
irregular ticks (float rounding decided which). Read live: the first draw is the size
channel at age 0, the flash is drawn once or twice, and the original's death rule is
"dead the update its life runs out". The spawn tick no longer ages or moves a pool
particle (`Particle::fresh`, which templates already had) and death keeps its
after-ageing test, with a rounding epsilon. This is the shared
`psys` machinery, so every pool effect changes by one tick of age at its first draw
(the Rocket's own explosions and the craft-hit blast included); the two numbers the
original gives to check it against are the flare's `1.05` and the SHAZZAM draw count.
The craft-hit blast's frame series (`rocket-visuals.md`, 2026-09-24) was not re-run.

**Result** (a scratch directory, not kept, fire minus a no-fire control, warm light
only - red minus blue, summed over the frame - mean of fire+3..+10, ours at tick
`402 + k`, a one-tick alignment that moves ours by up to 8 %):

| | warm light added | within 120 px of the nose | within 90 px |
| --- | ---: | ---: | ---: |
| original, boot a / boot b | 2,805 / 2,612 | 90 % | 71 % |
| ours before | 2,655 | 62 % | 46 % |
| ours now | 2,859 | 69 % | 53 % |

**The total was never the gap** - ours before was inside the original's own boot-to-boot
spread, and the disc it drew was bright and white; what moved is the kind and the
cadence, which are the measured part. Read as a player, native size, fire+3..+6
(`pair-d1.png`, `pair-d2.png` in the scratch directory): the orange-yellow streak up
the road with a white core at the nose, and the wide orange halo, are both there now
where before there was a white blob with a small yellow spot. **Not matched**: where
the light sits. The original's lies closer to the nose (90 % within 120 px against
69 %), because ours' rockets are further along the road at the same frame; that
follows the pose differences (our standing start, row (c) above) and the fan's lateral
drift, which this page has not measured.

**Does it stamp the bloom mask? No - measured.** PPSSPP on the **software renderer**
(the OpenGL backend leaves EDRAM zero; `SoftwareRenderer = True` in the profile),
both framebuffers read at fire+2, 3, 4, 5, 6 and 8 beside a no-fire control
(`psp-weapon-pair.py --edram`): the alpha plane holds the same values as the control
(`4`, `51`-`60` animating, `76`, `104`, `247`, `255`), and **7 to 33 pixels** differ
across the frame, the rockets' own hulls at the edges. The flare's quads, 10 to 30
units across, change none. The halo a player sees is the bloom of the **road**, whose
mask is `255` under the flare (about 30,000 pixels of `255` in both the fire and the
control frame): the bright pass is `rgb * alpha`, so light added to a stamped road
glows and light added to a `4` sky does not. Ours stamps the same road (`OAG_DUMP_GLOW_MASK`,
side by side at fire+5, same shapes), so no mask change was made. Seen on **one
boot** (six frames), so scored low: **80** for the rocket's flare and SHAZZAM not stamping;
the particle path's protect call is `docs/rendering/glow-mask.md`'s.

## What is not verified

- ~~**The quarter-turn's axis**, blocked on resolving `0x08a6b6b4`~~ -
  resolved and measured, see the 2026-09-24 section above.
- ~~**`WO_ROCKET_FLARE`'s parameters**~~ - read live and played 2026-10-01, see the section above; what remains is where the light sits, which follows the pose.
- **`func_0x0003a37c`**, the test that gates the craft-hit explosion.
- **What class `0x3e9` is called.** The id is read off the constructor; the
  class table's name for it was not looked up.
- **Whether the PS2 build agrees.** Nothing here has a second-binary leg, which
  is what holds every row below 95.

## History


- **2026-08-11.** Written while answering "make the rockets look like the
  original's". Found `weapon-fire.md`'s spawn address wrong by the same
  relative-address trap that page warns about - a reminder that the trap catches
  people who know about it. The emulator leg was added after the static read,
  and changed one thing: it is what turned "the rocket probably follows the
  surface" into a picture of two rockets skimming the track a body-length off
  the racing line.
- **2026-10-01, pulse-fx-recheck: the craft-hit blast's pool census.** `WO_ROCKET_EXPLO`'s eight pools read live
  (`--probe rolled`, one boot) against ours played alone: the steady-state areas agree to `10 %` (ticks 18, 24,
  30: `4.2k, 4.8k, 4.5k` against `4.0k, 4.7k, 4.1k`), the `GLOW` is the same law (half-size `1 + 8.8` a tick, alpha `200`,
  life 11 here and 5 to 15 authored). The matrix is identity (the spawn's rotation rows `1, 0, 0 / 0, 1, 0 / 0, 0, 1`), so
  the `0.75` scale law of the ship explosion does not apply. The picture series was **not** re-run like for like: a
  Time Trial pair has nothing to hit, and a Single Race pair (`--mode single_race`) puts the grid's craft 20 units
  ahead in ours where the original's has left by the fire frame.
- **2026-10-01, the launch glow.** The flare's 4-bit sprite and the pool particle's first draw at age 0 found and fixed; the rocket's flare and SHAZZAM read live; the mask measured unstamped.
- **2026-10-01, second pass.** The four rows landed, the probe-hit arm and the craft-up
  seed read and ported, a volley from the original's pose matched to 1-2 ticks and
  8 units, and the first-updates probe miss isolated and left open.
- **2026-09-24.** The quarter-turn resolved and measured live: it is the
  flare's frame, the model's basis is a rotation, and ours was a reflection.
  `0x08a6b820`/`0x08a6b6b4` named.

## 2026-10-08: the launch glow re-measured - the pool is equal, and the earlier pairs were taken ten ticks late (pulse-weapon-look)

The thread's open item was "the wide orange glow at fire+3..+8 is larger on the original than ours,
`WO_ROCKET_FLARE`'s parameters unread". Both halves were already stale (the flare was read and played on
2026-10-01, "the launch glow" section above). Re-measured from scratch with the hypothesis written first:
**H1, ours is smaller, is false** if fire-minus-control warm light of ours falls inside the original's
two-restart spread (one PPSSPP boot, not two). Method: `psp-weapon-pair.py rocket` (fire at GO + 120, speed 106.2, Venom, Talon's Junction,
Time Trial, PPSSPP software renderer, native), a no-fire control per side, two race restarts of the original in one PPSSPP boot,
warm light = sum of `max(0, dR - dB)` over pixels above 20, fire minus control.

**Finding 1 - the scenario was ten ticks late.** `verification/scenarios/weapon-after-go.inputs` presses at
tick 403, written when our standing start was ten ticks slower. Ours now reaches the original's state at
**tick 393** (x 124.94, speed 107.3; the original fires at x 124.5, speed 106.2): at 403 the craft is
at x 143 and 111 u/s. Every pair taken with the old file fired later in a different scene
(a tank and a building at other screen positions in the contact sheets; the flare drew from x 146.7
where the original draws from 126.9). `verification/scenarios/weapon-after-go-matched.inputs` fires at 393
and the original's `fire + k` frame is ours at tick `391 + k`. The old file stays: two ground-truth tests
pin their pictures to it. Earlier warm-light tables (2026-10-01) were taken at `402 + k` and carry this offset.
For ours a no-fire control must still pass `--give rocket` and use a script that never presses `square` (`nofire.inputs`):
with no `--give` the frame differed everywhere (a translucent ship and a different camera, cause not looked into), so it
is no control.

**Finding 2 - the glow is not smaller.** Warm light, millions, mean over the frames (original restart A / B in one boot, ours):

| frames | original A | original B | ours |
| --- | ---: | ---: | ---: |
| fire+3..+8 | 2.46 | 2.53 | 2.20 |
| fire+9..+30 (eight sampled) | 2.18 | 1.93 | 3.09 |

The first window is inside the boot spread (ours 11 % under, the original's own two restarts 3 % apart); the original is
repeatable (A and B agree frame by frame to about 10 %). From fire+9 ours is **about 1.5x** the original, in spikes at
fire+9, 16, 20 and 25 (4.0, 4.2, 3.8, 3.5) with the ticks between at the original's level (2.7, 2.95, 1.9, 1.7
against 2.1, 2.0, 2.0, 2.0): the excess is episodic, not a steady surplus.

**Finding 3 - the pool is equal, so the excess is not the effect's law (confidence 80: one probe run in one boot).** `--probe flare` and `--probe rolled` on the original against a temporary
(reverted, `psys-debug-print.patch`) per-spec print in `Stage::extend_vertices`:

| quantity | original, per rocket | ours |
| --- | --- | --- |
| `WO_ROCKET_FLARE` root count by age | 2 a tick: 8 at 4.9, 14 at 7.9, 26 at 13.9, 50 at 25.9, 60 at the 30-tick life | `2 (k-2)`: 4, 10, 22, 46, 58 (two ticks of display lag) |
| size, oldest particle | 1.74, 2.44, 3.83, 6.62, 7.78 | 1.74 (n=8), 2.67 (n=16), 3.60 (n=24), 6.38 (n=48), 7.30 (n=56) at the same counts |
| alpha, oldest | 229, 203, 152, 50, 8 | 0.90, 0.77, 0.63, 0.23, 0.10 (x255: 229, 195, 161, 59, 25) |
| colour by age | (245,245,191) at 0, (179,105,0) at 7 | (233,233,119) at age 0.07, (196,156,0) at 0.20, (154,65,3) at 0.33 |
| positions | x 125.7..140.8 at n=10, 124.4..155.6 at n=18, 114..214 at n=50 | x 127.9..139.6 at n=8, 127.1..154.3 at n=16, 124..213 at n=48 |
| `WO_ROCKET_SHAZZAM` | 1 or 2 quads a tick per rocket, half-size 5.5 to 14.8 | 1 or 2 a tick, 5 to 14.8 |

The blend is the table's class 2 by the static read (`SRC_ALPHA`, `FIX 0xffffff`); a GE dump at fire+9 contains batches with blend
`src=2 dst=10 fixB=ffffff` (hand-decoded, **not attributed to the flare's draws**), and ours is `SrcAlpha`/`One`. Falsifier met: count, size, alpha, colour, position and blend
match, so the spikes are neither the pool nor the blend. What is left, **not isolated**: the SHAZZAM draw's per-tick random size and its
sprite shading at the spike ticks (the original's flashes are also random but its two restarts agree, so its sequence is
deterministic and ours is another sequence), and the road's bloom under the larger quads. No fix was made. The
`85 x 110 px` fireball at k=25 in the brief is the **craft-hit** blast (`WO_ROCKET_EXPLO`); this Time Trial pair has
no blast before tick ~49 and its wall hit is `WO_ROCKET_EXPLO_TRACK`, so nothing is reported about it from this pair.
Cross-title: Pure's `WO_ROCKET_FLARE` is one emitter (`pure-status.md`) and its rocket drew unchanged by this lane:
**checked, nothing changed, nothing to port**. `pure-weapon-gfx` fired at tick 404 of the same old file; the 393-against-403
offset is Pulse's standing start against Pulse's original, so whether Pure's timing is off **was not checked** (it has its own
handling and its own original).

## 2026-10-30: the Rocket matched against the original, and what differs (pulse-rocket-fidelity)

Method as the Plasma's ([plasma.md](plasma.md), "2026-10-30: the Plasma matched against the original"):
Talon's Junction, Time Trial, Venom, Assegai; `scripts/psp-weapon-pair.py rocket` on PPSSPP v1.20.4 software
renderer (Pulse PSP USA), fire 120 frames after GO at speed 106.21, x 124.5; ours is `oag-game --race --mode
time_trial --team Assegai --class venom --give rocket --input-script
verification/scenarios/weapon-after-go-matched.inputs` at tick `391 + k`. A no-fire control on each side, three
fire restarts of the original (a, b and c; a/b at 30 frames, c every second frame k46-98), scoring by
`scripts/weapon-pair-score.py` (HUD masked, 87,625 px). Pairs under `data/reference/pulse-rocket-pairs/`.
The original is repeatable (the floor column, restart against restart: 1.4-5.4 grey), the state is matched (speed
106.21 on every boot). Confidence **55** on the numbers: one PPSSPP boot.

**Detonation tick: the handover's "ours detonates a few ticks early" is false on this scenario.** The probe
(`--probe spawns`, two runs) puts the three rockets' `WO_ROCKET_FLARE` at fire+0 and `WO_ROCKET_EXPLO_TRACK`
(`Rocket_Update`, `ra 0x0885dad4`) at **fire+51.0, +61.0, +70.0** frames (second run 51.0, 61.03, 70.0); the
craft's `WO_SHIP_COLL_SPARK_DAMAGE` follows at +72-73. Ours (airborne count of the three, per tick) drops at ticks
446, 456, 465 after a fire at tick 394: **+52, +62, +71, one tick late** on all three, with the same 10 and 9 tick
gaps. The original's `Rocket_Update` positions at the end: rocket 2 (first to go) x 309.8, y -39.0, z -188.1,
velocity (218.8, 27.9, 27.0); launch speed 166.3 u/s for three frames, then 213-221 u/s. Which of the launch ramp
or the sweep length costs the tick is not isolated (ours' rocket positions were not read per tick). Confidence 55.

### Ranked gaps (excess = fire minus no-fire control, grey)

| # | Region, frames | Area (px) | Grey error | Area x error | Driven by | Fix estimate |
| ---: | --- | ---: | ---: | ---: | --- | --- |
| 1 | In-flight trail and flare, k12-24, whole frame (box 120,30,280,170: 12.2): ours throws a wide orange-white haze up the right wall and over the track by the rocket; the original is a tight orange band along the line of fire | 87,625 | 7.7 | 0.67 M | `WO_ROCKET_FLARE` (`ROFL`, carried by each rocket, `Rocket_Init` `0x0885cdb8`), drawn by `ParticleSystem_DrawRolledQuads`/`FUN_089194d0`. Either its size/colour law or the rockets' sideways drift differs (the original's three rockets carry +27 to +45 u/s on z) | re-first, conf 40: `--probe flare` on the original for per-particle size/alpha, and ours' per-tick rocket position, then diff |
| 2 | Fireball and smoke, k54-72, box 150,40,250,110: ours is a saturated yellow-orange solid with a hard edge; the original is brown smoke with orange mottling that darkens and thins by k80-98 | 24,680 | 9.3 | 0.23 M | `WO_ROCKET_EXPLO_TRACK` (`ROD2`) emitters, texture or colour ramp at age; k54 and k64 spikes (7.4, 5.1) are the one-tick-late onsets | re-first, conf 45: `--probe spawns` already gives the trigger; the smoke texture bind and ramp need a GE dump at k58 |
| 3 | Launch, k1-10, box 120,30,280,170 | 43,730 | 3.1 | 0.14 M | Same flare; the white launch blowout is within noise of the floor (4.2) from k1-3 | no action |
| 4 | Detonation one tick late, +52/+62/+71 against +51/+61/+70 | whole frame | k54: 7.4, k64: 5.1 | | Rocket launch ramp or sweep length (`Rocket_SpeedForClass` `0x0885d1b0`, `Rocket_Update` `0x0885d2a8`, read) | ready-to-wire after a per-tick position read on ours, conf 50 |
| 5 | Craft damage onset, k74: ours' red shield bar one sample earlier (k74 against k78 of every second frame) | HUD | 7.8 | | Blast force on the craft (`Rocket_ApplyBlastForce` `0x0886ee88`) follows gap 4 plus the original's 2-frame display lag; likely the same tick | follows gap 4 |
| 6 | Craft-hit blast `WO_ROCKET_EXPLO`, fireball size (85x110 at k=25) and the kind-0 wash | | | | **Not captured**: Time Trial has no opponent. A Single Race pair puts the grid craft in different places each boot | re-first: needs an aimed hit; recipe in the handover thread |

Not findings: the HUD weapon hexagon (ours keeps the held-pickup icon because `--give` refills the slot), the
song-title ticker, the scene gap with no weapon (6-8 grey in every frame, 12-22 after k74 from the craft's
slowdown differing, the `ctl` column), and the 2-frame display lag.

**Cross-title.** HD and Omega fire a different Rocket (`ps3-hdfury-eu/` pages; model, trail and blast are their own
and an HD pair is not run here): **checked, differs** by file, not re-measured. Pure's Rocket visual and 2048's:
not checked in this lane (`docs/gameplay/weapon-lineage.md` unchanged, no cell moved). Pulse PS2 inherits PSP's
measured law and has no pair (**not checkable** without a PCSX2 pair).

## 2026-10-31: the flare haze and the fireball read off GE dumps of the original (pulse-rocket-look-law)

Method as the Plasma's ([plasma.md](plasma.md), "2026-10-30: the blast and the head read off GE dumps"):
`scripts/psp-weapon-pair.py --ge-dump-k K --shots K rocket` (Talon's Junction, Time Trial, Venom, Assegai, fire at
GO + 120, speed 106.21 on every boot), `scripts/psp-ge-dump.py census`, replays of edited dumps
(`scripts/psp-ge-replay.py`), PPSSPP v1.20.4 software renderer. 14 dumps in 12 boots (k10, 16 x3, 22, 28, 56, 60 x2,
64, 72, plus no-fire controls k16 and k60). Ours is not dumped; "ours" is `oag-game --race ... --ticks 391 + k`
with temporary, reverted `eprintln!`s on the particle pool, plus screenshots with a draw switched off (reverted).

**Falsifiers written before the capture.** (1) "The original's flare haze is only a tight band": falsified. The
original's own GE list carries, per rocket, one additive triangle-strip of 20-33 flare quads (up to 10 units wide,
the nearest 1.1-3.7 units from the eye) and one to two SHAZZAM quads; removing the strips in a replay takes the
whole-frame grey excess over the no-fire control from 7.1 to 2.4, removing the SHAZZAM quads to 6.1, both to 0.8 (the
floor). The excess is the strips', not a band. Prims far from the rocket (a 75-unit additive quad at x 536-611, the
many 4-vertex rows at x 492) are scenery that differs between boots and are excluded by position. (2) "The brown
smoke is not additive and matches a `WO_ROCKET_EXPLO_TRACK` embedded texture": held (below).

### What each draw is (texture byte-matched to the `.pob`)

Texture identity is by byte match of each draw's texture chunks against the `.pob`'s embedded texels; the dump's
RAM-resident textures arrive as chunk types 16-19 (levels 0-3, no header), not 24-28 (VRAM-address uploads, 16-byte
header), which the `psp-ge-dump.py` docstring does not say. A match counts only with a mip chain (levels 1-3 at the
sizes the dimensions give: +0x400, +0x100 for 64x64 CLUT8 and 128x64 CLUT4).

| Draw (vertex type) | Texture | `.pob` and offset | Blend | State |
| --- | --- | --- | --- | --- |
| flare strip, one per rocket (`0x19f`: float uv, 8888, float xyz; 4 vertices a particle plus 2 stitch vertices) | 128x64 CLUT4, 4x4 atlas (uv on a 0.25 grid) | `WO_ROCKET_FLARE.pob` levels 1-3 at `+0x2a30`, `+0x2e30`, `+0x2f30` (level 0 `+0x1a30`) | `0xa2` additive | texfunc modulate, ztest on |
| SHAZZAM quads (`0x11e`: u16 uv, 8888, s16 xyz at `/8`, stride 16) | 64x64 CLUT8 | `WO_ROCKET_FLARE.pob` levels 1-3 at `+0x43b0`, `+0x47b0`, `+0x48b0` (level 0 `+0x33b0`) | `0xa2` | 1-2 quads a rocket, 14-30 wide |
| blast root glow, one quad (spec 0) | 64x64 CLUT8, whole texture | `WO_ROCKET_EXPLO_TRACK.pob` `+0x49a0`... | `0xa2` | 139-264 wide |
| blast sparks (spec 1) | 32x32 CLUT8 | `WO_ROCKET_EXPLO_TRACK.pob` `+0x5720`, `+0x5820` | `0xa2` | 10-36 wide, wide-flat |
| **blast smoke (spec 2)** | 64x64 CLUT8, 4x4 atlas | `WO_ROCKET_EXPLO_TRACK.pob` `+0x6ca0`, `+0x70a0`, `+0x71a0` (level 0 `+0x5ca0`) | **`0x32` alpha-over**, not additive | 5 quads a blast |
| blast fire (spec 3) | 128x64 CLUT8, 4x4 atlas | `WO_ROCKET_EXPLO_TRACK.pob` `+0x9620`, `+0x9e20`, `+0xa020` | `0xa2` | 4 quads a blast |

So **ours' texture and blend binds are not misread**: `psys` spec 0-3 of `WO_ROCKET_EXPLO_TRACK` are additive,
additive, alpha-over, additive, the same four. The smoke is the only non-additive draw, and it is what the pair shows
as brown. Confidence **85** (byte match with mip chain, three boots for the flare, two for the blast at k60).

### Gap 1: the flare law, per particle, original against ours

Particles of the flare strip; three boots of k16 agree (oldest particle age 12, 12, 11 ticks), sorted by age. Width is
the quad's side in view units; **width = 2.10 + 0.4583 x age ticks** (half-size `1.05 + 0.229 x age`, the size
channel's own line), two particles a tick. The same table for ours is read off its pool at tick 409 (`391 + 18`), alpha
printed to four places:

| Age (ticks) | Width | Original rgba | Ours rgb, alpha |
| ---: | ---: | --- | --- |
| 0 | 2.10 | 245, 245, 191, 255 | 255, 255, 255, 255 |
| 1 | 2.56 | 233, 233, 119, 246 | 245, 245, 191, 247 |
| 2 | 3.02 | 223, 223, 55, 237 | 233, 233, 119, 238 |
| 3 | 3.49 | 212, 208, 0, 229 | 223, 223, 55, 229 |
| 5 | 4.42 | 196, 156, 0, 212 | 204, 184, 0, 212 |
| 9 | 6.27 | 154, 65, 3, 178 | 172, 81, 0, 178 |
| 12 | 7.66 | 82, 47, 23, 152 | 107, 53, 16, 153 |
| 14 | 8.59 | 35, 35, 35, 135 | 60, 41, 29, 136 |

**Alpha agrees** (`255 x (1 - age / 30)` in both; an earlier draft of this section read ours' alpha as stepped, which was
the `{:.1}` of the debug print, not the pool). **Ours' palette index is one tick younger than its size**: at a given
width ours shows the colour the original shows one tick earlier (ours' age 1 colour (245, 245, 191) is the original's
age 0). Small (a tick of colour, brighter in ours) and not the haze. Confidence **80** (20 matched ages, ours
deterministic).

The SHAZZAM quads read as before (14 / 30 / 28 wide at k16, `(255, 255, 255)`, half-size 5-15): they carry about 1.0
of the 7.1 grey excess (replay), so they are not "the glow" (the 2026-10-01 reading above), the strips are.

### Gap 1: the haze is the root flare drawn at the particle's moving position; the original draws it at the spawn point

Ours at tick 409 (fire + 18, the pair's shot k18): temporarily skipping particles of the pool before they draw and
re-rendering, excess over ours' no-fire control (grey; mean over the frame above the HUD, over the upper-right box
`y 20-110, x 270-480`, over the central band `y 110-200, x 120-420`):

| Ours | Whole | Upper right | Central band | Error to the original's fire frame |
| --- | ---: | ---: | ---: | ---: |
| original (fire minus control) | 7.7 | 0.4 | 18.8 | 0 |
| ours, all draws | 15.2 | **17.1** | 27.3 | 16.6 |
| ours, without the root flare (spec 0) | 2.2 | 0.2 | 8.6 | 13.7 |
| ours, without the SHAZZAM quads (spec 1) | 14.5 | 17.0 | 24.7 | 16.6 |
| ours, root flare of age 7 ticks and over skipped | 2.6 | 0.2 | 10.2 | 13.4 |
| **ours, root flare spawned with velocity 0** | **8.0** | **0.2** | **19.1** | **10.7** |

The wash up the right wall and across the sky is the **old root particles (age 7-10, 2-14 units from the camera, 2.7-3.4
units across, so each covers most of the screen), and they are in the wrong place**: ours gives the root flare an
ejection velocity of `speed_per_tick = (0.0, 1.009)` over a full sphere (`Cone`, half-angle pi, `velocity_inherit 100`,
no drag, no gravity: the authored `+0x48`/`+0x4c` pair) and integrates it, so a particle of age 10 has wandered up to 10
units (view `y` from -11 to +3.4 at age 10, `x` up to +8); with the velocity set to zero at spawn the excess falls
to 8.0 / 0.2 / 19.1, the original's 7.7 / 0.4 / 18.8. In the original the strip's two particles of a tick sit at the same
view `(x, y)` to 0.03-0.1 units up to age 12, and the near ones (`z -1.1` to `-3.7`) all hang within 0.3 units of one
another under the eye line (centres `y -3.0 to -3.8`, `x -0.7 / +0.4 / +1.5` for the three rockets, three boots), so
they blanket the lower half of the screen and nothing above it.

**But the original's pool does move the particles** (`psp-weapon-pair.py --probe flare`, one boot, 22 frames, the pool
position at `particle + 0x40`): the nearest same-slot particle between consecutive frames is 0.38-0.99 units away, the
ceiling being the authored spread 1.009. So the pool carries the same ejection velocity as ours, and **the GE list
draws the root flare at a position that does not move: the spawn point** (`ParticleSystem_DrawPoolSquares`'s position
read; which field it reads, the particle's `origin` or the position before the velocity step, is **unread**). Ours
draws the integrated position. Confidence **85** that the draw is static (three boots of k16, ages 0-12, plus the
zero-velocity replay recovering the frame), **60** that the rule is the draw's alone and not the spawn (the pool
probe moves, one boot). Only the root flare (`WO_ROCKET_FLARE` spec 0, `ParticleSystem_DrawPoolSquares`) was measured:
**do not generalise** to the other emitters; a draw class that moves them (the blast's debris) was not dumped.

**Wire recipe (ready-to-wire, confidence 85 for the flare):** draw `WO_ROCKET_FLARE`'s root particles at their spawn
position (`Particle::origin` in `oag_fx::psys`, which already exists), keep the pool integrating (children and
the death-child anchor read the moving position); scope it to the one emitter the dumps measured, per this
lane's rule on shared readers. Not changed here: `psys` is shared by every effect. Also: shift the root flare's palette
index one tick (above). **Open:** ours' rocket 3 flies 15.4 u/s laterally at tick 2 against the original's 7.1, and has
no first-four-tick 166.6 u/s phase (flies 211.9 with a first-tick `y` velocity of -66.8 from its nose-point spawn): both
belong to the flight-timing lane's shared `projectile::advance` and are not changed here.

### Gap 2: the fireball and smoke, where ours' alpha is five to fifteen times the original's

The original blast (`WO_ROCKET_EXPLO_TRACK`, one per detonation at fire + 51, 61, 70; blast age `a` = dump k minus
detonation) draws five smoke quads (alpha-over), four fire quads and one root glow quad plus sparks. Smoke law, two
boots at k60 (identical to the byte) and single boots at the other ages:

| Blast age `a` | Smoke width | rgb | **Alpha** | Fire quad rgba, width |
| ---: | ---: | --- | ---: | --- |
| 2 | 8.1 | 246, 189, 2 | 2 | 255, 237, 210, 255; 12.2 |
| 3 | 8.8 | 242, 186, 2 | 3 | 255, 211, 124, 255; 14.8 |
| 5 | 10.5 | 233, 179, 2 | 5 | 255, 119, 7, 255; 22.2 |
| 9 | 14.0 | 213, 164, 2 | **44** | 255, 151, 48, 225; 26.8 |
| 11 | 15.8 | 204, 157, 1 | 64 | 255, 229, 172, 196; 30.1 |
| 13 | 17.6 | 196, 150, 1 | 84 | 255, 186, 92, 174; 33.8 |
| 21 | 25.5 | 160, 123, 1 | 125 | 255, 238, 214, 69; 29.9 |

Width is `6.3 + 0.9 a` (to 0.5). The five smoke quads of a blast are identical in size, colour and alpha and differ only
in position (a 3-8 unit scatter); the texture cell is one of sixteen, the same for all five.

Ours (`--ticks 453`, `457`, `463`, tick = 393 + dump k, alpha printed to five places): the smoke widths agree (12.1 at
age 0.07 of 100 ticks, 14.9 at 0.11) but **ours' alpha is 0.910 at 6 ticks and 0.775 at 7.5, the original's about 0.06
and 0.10 at those widths** (and the two agree at ticks 2-3, 0.010 against 0.008): the authored
alpha channel of spec 2 is `(0, 0.007), (0.041, 0.021), (0.0436, 1.0), (0.177, 0.548), (0.497, 0.178), (1, 0)`, a spike
to full opacity at tick 4.4 that decays; the original's alpha **rises** instead, from about nothing to 125 at tick 21
(`44 / 64 / 84 / 125` at `a = 9 / 11 / 13 / 21`, about +10 a tick from `a = 5` to 13, +5 after), in the same
yellow-brown that ours draws. Ours' smoke is therefore a saturated, nearly opaque yellow-orange disc for the
first ten ticks where the original's is a thin brown veil with the additive fire seen through it: this is the "saturated
yellow-orange solid" of gap 2. The fire quads (alpha 225 then falling, 255 -> 69 over 21 ticks) and root glow have the
same palettes and sizes in ours, so the smoke alpha is the one draw to fix; ours' root glow is dimmer at the same age
(width 102 against 139, rgb 41-50 against 51-152) and was not pursued.

**The palette's own alpha is not the missing factor**: `palette[idx][3]` (which `extend_vertices` discards) is 0.92 at tick
6, 0.77 at 11 and 0.57 at 17, the authored curve again, so channel x palette alpha would be lower than ours by
a third at most and cannot make 0.06. The original's alpha is not the authored channel read at the particle's age, and what it is was **not isolated**: a
fade-in multiplier of `(a - 5) / 16.5` (0 at tick 5, 1 at tick 21) reproduces 44, 64, 84 and 125 to 20 % against
the authored curve (0.84, 0.78, 0.70, 0.51 of 255 there), and a table that smears the one-tick spike across a coarse
cell would give the same rise; the data stop at `a = 21`, so the peak and the fall are unread. Confidence **60** for
the table, **35** for any closed form.

**Recipe to finish it (not run):** dumps at k80, k90, k110 (blast age 29, 39, 59 for rocket 1) and the smoke quads'
alpha from each, plus a breakpoint probe on the particle update's alpha write for `WO_ROCKET_EXPLO_TRACK` spec 2
(the particle's alpha field offset is unread; `FUN_089194d0` draws the pool).

### Detonation tick, and what the dumps add to it

Read only. The blast spawn is at fire + 51, 61, 70 (probe, two runs) and ours' +52, +62, +71; the dump series gives
nothing against it (blast age 9 at k60 on the original). Ours' rocket 3 early velocity (above) is the one more
datum for the flight-timing lane.

### Cross-title

HD and Omega fire a different Rocket (`ps3-hdfury-eu/` pages): **checked, differs**. Pure: `Data\Psys\WO_ROCKET_FLARE.POB`
ships (32,352 bytes against Pulse's 18,752, a different file) and `WO_ROCKET_EXPLO_TRACK.POB` does not: **checked,
differs**; whether Pure's draw law matches is unread. 2048 loads Pulse's `WO_ROCKET_FLARE`/`_EXPLO` from
`data/particles2048` through the same `psys`: **not checked** against 2048's own draw (no 2048 GE dump). Pulse PS2
inherits PSP's law and has no pair (**not checkable** without a PCSX2 dump).

## 2026-11-11: code `4` is the collision cage, and what a bank does to every flyer (weapon-bank-follow)

Maintainer report: "on tilted track our weapons (plasma and others) fall off the track along gravity,
instead of following the track as in the originals." **True, and it is one missing law**: the second
query inside `Collision_SweepSegment`, the one this page and [missile.md](missile.md) read as a craft
test, is the **collision cage** ([collision-cage.md](collision-cage.md)). In the original a projectile
whose probe or travel segment ends outside the cage is destroyed (or, for a Missile's and a Shuriken's
probe, ignored); ours has no such test, so a projectile that gets past the last floor triangle flies
its whole lifetime through the void, falling at `50.0`. The flight laws themselves agree: on an
85 degree bank ours reproduces the original's Rocket, Cannon and Shuriken positions to `0.000` unit per
update.

### Where and how

**The steepest sustained bank on the disc is Moa Therma White** (`03_Track` forward): path 0 is
rolled 40 to 86 degrees for 1,682 units of arc (715 to 2,397 from the path's start; `asin(lateral.y)`
of every `oag-trace track` sample, all 24 layouts censused: next are de Konstruct `05` with 1,393
units over 40 degrees and Basilico `01` with two 300 unit stretches; every other circuit peaks under
51 degrees). Shots were fired at **sample 1416** (roll 85 degrees, about 130 units before the helix
starts to untwist), craft at `(251.4, 124.2, -349.3)` doing 139 units/s, flown there by the original's own
Autopilot (a ride to the spot, not a physics reference). A Plasma was fired at sample 1330 so that
its release fell at the same place. Second place: Outpost 7's first ramp (`07_Track`, sample 186).
Third: player-driven shots 120 frames after GO on Moa Therma's start straight and from a parked
craft on Talon's Junction. PPSSPP v1.20.4, software renderer, Time Trial, Venom class, USA disc.

`scripts/psp-projectile-sweeps.py` stops at each weapon update's entry and at the return of each of
its `Collision_SweepSegment` calls (the probe, then the travel sweep), one breakpoint at a time. Per
update it records the entity before the update, each call's return code with the point and normal it
wrote, and what is left of `Collision_SweepSegment`'s own frame under the caller's stack pointer:
the point handed to the second query (`sp - 0x70`), the mesh raycast's hit point (`sp - 0x60`),
collider index (`sp - 0x40`) and kind (`sp - 0x38`). Two loads of the same state gave the same
volley to 0.2 unit and the same ending update.

Ours was then run two ways from those records (a scratch harness on `oag_weapons::projectile::step`
and `Race::collision`, not kept): **one update of ours from each of the original's states** with the
original's own `dt`, compared with the original's next state, which isolates a law; and a **free
flight** from the original's first state at `1/60`.

### Result by weapon

| Weapon | Original's updates | One update of ours from each state | The original ends | Ours from the same first state |
| --- | ---: | --- | --- | --- |
| Rocket (centre shot) | 49 | **48 of 48 at 0.000 unit, velocity 0.00, normal within 0.03 degrees** | update 49: mesh wall (code `0`) at `(420.0, 148.3, -291.1)` | tick 49, the same wall point |
| Plasma | 49 | 45 of 48 within 0.012; the first three differ (0.82, 0.13, 0.09), see "the own-craft box" | update 49: **code `4`** on the probe and on the sweep, boundary `(424.66, 153.08, -292.16)` | never: rides the kerb, misses at tick 49, still flying at tick 600 at `(1786, -296, 2275)` |
| Shuriken | 58 | 57 of 57 at 0.000 unit (the velocity it leaves differs, see [shuriken.md](shuriken.md)) | update 58: **code `4`**, boundary `(495.83, 144.73, -278.27)` | never: tick 600 at `(2815, -1062, 1492)` |
| Missile | 180 | first two differ by 1.0 (own-craft box); then 0.05 to 0.5 a tick ([missile.md](missile.md)); the wall bounce at update 67 agrees to 0.000 | update 180, its own 3 s fuse | tick 181, about 8 units away |
| Cannon | 25 | **24 of 24 at 0.000 unit** | update 25 (age 0.40 s): **code `4`**, boundary `(553.28, 143.17, -280.29)` | reaped at tick 61 (its 1 s life), 440 units past the cage |
| Rocket volley, Outpost 7 ramp | 17, 22, 21 | exact except the own-craft box updates and one update after a ramp-face landing | all three: **code `4`** at `y` 20.0 to 27.3 from a launch at `y` -1.2, 0.27 to 0.37 s after the fire | two of three fly their whole 5 s life over the lip; the third meets a wall at tick 31 |

So the first tick ours diverges *off the track* is the tick the original's query returns `4`:
update 49 for the Plasma and 58 for the Shuriken on the bank, 25 for a Cannon round, 17 to 22 for
rockets on the ramp. Before that tick the positions agree; after it ours has a projectile where the
original has a detonation.

**That also answers this project's oldest open flight question** (`projectile-floor.md`, "jump
lips": "whether the original's rockets sail off the same lips is a play-test question"): they do
not. They are destroyed at the cage's ceiling within about a third of a second.

### The law: `Collision_SweepSegment`, complete

```c
int Collision_SweepSegment(world, from, to, out_point, out_normal, cursor, flags) {
    code = 0x7f;
    end  = *to;
    if (world && Collision_RaycastWorld(world, from, &end, &hit, -1, 0, 0) && hit.kind == 1) {
        code        = world->colliders[hit.collider]->surface;   // + 0x6c: 0 wall, 1 floor, 3 mag floor
        *out_point  = hit.point;
        *out_normal = hit.normal;
        end         = hit.point;                                 // the cage is asked about the hit
    }
    if (Cage_QuerySweepEnd(from, &end, out_point, out_normal, cursor, flags))   // 0x08831948
        code = 4;
    return code;
}
```

`Cage_QuerySweepEnd` is `Cage_TestSweepEnd(g_cage_effect, ..)` (`0x08832134`), decompiled end to end:

1. `g_cage_enabled` clear: return 0.
2. `AiTrack_UpdateCursor(500.0, track, cursor, end, ..)`: locate **the segment's end** (the mesh hit
   when there was one) on the AI track, from the projectile's own cursor (`rocket+0x114`,
   `plasma+0x130`, `missile+0x170`, `shuriken+0x16c`, `cannon round+0x604`; the argument this page
   called `bounds`). No located point: return 0.
3. `AiTrack_LocateOnSiblingPath`, `Cage_BuildSection` for the point and for the sibling at a fork,
   `Cage_TestPoint` on the end against each: exactly the craft's cage steps 2 to 4
   ([collision-cage.md](collision-cage.md), "The law"), on one point instead of eight corners.
4. Inside either section: return 0. Outside both: write the nearer boundary point to `out_point`
   and its inward normal to `out_normal`, call `CageEffect_Spawn` there when `flags == 0` (every
   weapon passes 0), return 1.

There is no 200 unit or 15 unit limit here; those are `Cage_ResolveCraft`'s.

**Two consequences that are easy to miss.** A mesh hit whose point is outside the cage returns `4`,
not its surface class: a floor outside the tube is a wall to a projectile. And a mesh raycast whose
nearest hit is a **box** collider (`hit.kind == 3`, a craft) leaves `code` at `0x7f` and `end` at
the full segment: a craft's box makes the query answer "nothing" (below).

What each update does with `4`, read off the five decompiles:

| Weapon | Probe returns `4` | Travel sweep returns `4` |
| --- | --- | --- |
| Rocket (`0x0885d2a8`) | flags `\|= 0x14`, `WO_ROCKET_EXPLO_TRACK` at the boundary point (as `0`) | the same |
| Plasma (`0x0885c6cc`) | flags `\|= 4`, the pool detonates it (as `0`) | the same |
| Missile (`0x0885a918`) | nothing: no ride, no fall (as `0`) | flags `\|= 0x14`: retired, **no bounce** (a `0` bounces up to its budget) |
| Shuriken (`0x08877bdc`) | nothing (as `0`) | flags `\|= 0x14`: destroyed, **no bounce** (a `0` is `Shuriken_Bounce`) |
| Cannon (`0x0886593c`) | no probe | flags `\|= 0x14` and the sparks (as `0`) |

Live for the Rocket, Plasma, Shuriken and Cannon rows (each ended on a `4`), and for the two
"nothing" cells (a Missile's probe returned `4` at updates 122 and 170 and a Shuriken's at update
40; each flew on and bounced off the mesh wall its sweep found). The Missile's sweep cell is
decompile only.

### The cage test reproduced: 1,187 queries, 17 of them `4`

Every recorded call (probe and sweep, five weapons, Moa Therma and Outpost 7) was put to this
engine's own cage, the one that already holds the craft (`oag_gameplay::cage::CageTrack::locate` on
the recorded point, then `oag_physics::cage::Section::test`, outside when outside the section and
its sibling's): **1,187 of 1,187 agree** on whether the original returned `4`, with no false
positive and no miss, and on the 17 that did the boundary point agrees to **0.000 unit** and the
normal within 0.03 degrees.

| Circuit | Point handed to the cage | Depth outside | Boundary point the original wrote |
| --- | --- | ---: | --- |
| `03` | `(419.2, 147.8, -285.0)` | 0.44 | `(419.28, 147.49, -285.21)` |
| `03` | `(421.3, 149.1, -281.4)` | 4.41 | `(421.77, 145.73, -284.12)` |
| `03` | `(424.6, 153.6, -292.0)` | 0.49 | `(424.66, 153.08, -292.16)` |
| `03` | `(316.0, 156.4, -308.1)` | 8.19 | `(317.47, 149.45, -312.11)` |
| `03` | `(427.1, 98.7, -294.7)` | 5.35 | `(427.30, 104.01, -295.53)` |
| `03` | `(493.1, 138.1, -269.2)` | 2.27 | `(493.25, 136.77, -271.07)` |
| `03` | `(495.8, 146.6, -277.3)` | 2.11 | `(495.83, 144.73, -278.27)` |
| `03` | `(596.4, 107.8, -261.7)` | 4.24 | `(596.45, 109.34, -265.65)` |
| `03` | `(770.3, 103.9, -310.8)` | 7.14 | `(767.59, 104.24, -304.19)` |
| `03` | `(553.3, 143.6, -280.2)` | 0.40 | `(553.28, 143.17, -280.29)` |
| `03` | `(-399.7, -3.0, -195.9)` | 1.41 | `(-400.73, -2.55, -195.11)` |
| `03` | `(-403.7, -3.0, -199.8)` | 0.81 | `(-404.26, -2.74, -199.28)` |
| `03` | `(-408.2, -3.1, -207.3)` | 1.87 | `(-409.45, -2.45, -206.01)` |
| `03` | `(-414.7, -8.1, -211.1)` | 5.96 | `(-418.06, -5.22, -207.18)` |
| `07` | `(-1057.8, 26.0, 340.3)` | 2.70 | `(-1057.76, 23.50, 341.19)` |
| `07` | `(-1003.1, 27.7, 323.5)` | 1.26 | `(-1003.03, 27.32, 324.72)` |
| `07` | `(-995.4, 20.0, 315.0)` | 0.02 | `(-995.41, 20.03, 314.97)` |

Confidence **94** that code `4` is the cage test on the segment's end and nothing else: decompiled
end to end, and the live return code and both outputs reproduced on every recorded call. Not 95:
one binary (the PS2 build was not read).

### States a test can start from

The original's own entity at the entry of the named update, and what followed. Seeding
`Projectile { position, velocity, surface, lifetime: MAX_FLIGHT_SECONDS - age, .. }` and stepping
at the `dt` given reproduces the listed next positions today; the endings are what is missing.

| Shot (circuit) | Update | `dt` | Age | Position | Velocity | Carried normal | Then |
| --- | ---: | --- | --- | --- | --- | --- | --- |
| Plasma (`03`), launch 700.77 km/h | 45 | 0.01668 | 0.7338 | `(407.899, 148.454, -295.326)` | `(204.506, 60.023, 72.350)` | `(0.261, 0.239, -0.935)` | rides 4 updates (mag floor at 45, the kerb's floor class at 46 to 48), **`4` at update 49** from `(421.300, 153.036, -293.762)`, velocity `(200.454, 31.446, 103.645)`, normal `(0.276, 0.368, -0.888)` |
| Shuriken (`03`) | 50 | 0.01668 | 0.8177 | `(458.717, 123.838, -291.732)` | `(230.851, 125.619, 91.020)` | `(0.120, 0.428, -0.896)` | **`4` on the sweep at update 58** from `(491.137, 142.501, -277.876)`, velocity `(279.537, 244.619, 35.872)`, normal `(0.223, 0.707, -0.671)` |
| Cannon (`03`) | 1 | 0.01669 | 0 | `(256.346, 123.760, -347.127)` | `(687.304, 33.099, 299.866)` | none | straight for 24 updates, **`4` at update 25** from `(541.014, 142.697, -282.444)` |
| Rocket (`03`) | 1 | 0.01669 | 0 | `(251.355, 124.203, -349.262)` | `(152.543, 8.285, 66.631)` | `(0.394, 0.053, -0.917)` | 222.22 units/s from update 2; ends at update 49 (a `4` on the probe and the mesh wall on the sweep in one run, the wall alone in the other) |
| Rocket (`07`), third of the volley | 6 | 0.01710 | 0.0835 | `(-1050.274, -1.294, 343.232)` | `(164.477, -3.584, -26.922)` | `(0, 1, 0)` | rides the ramp, **`4` at update 21** from `(-1006.012, 26.012, 324.725)`, velocity `(176.258, 103.231, -74.087)` |

### The own-craft box: why the first probes miss

The "first two or three updates are a probe miss, cause unrecovered" of the 2026-10-01 section,
and the Plasma's unridden first updates ([plasma.md](plasma.md), 2026-10-31 row 3), are the
**firing craft's own box collider**. `Collision_RaycastWorld` is called with no collider to skip
(`-1`) and with boxes included (`0`), so a probe that starts inside the craft's box meets its floor
face first, `hit.kind` is `3`, and `Collision_SweepSegment` returns `0x7f`: the fall arm, at the
launch speed. Read directly, 28 probe records in 5 captures: mesh hit kind `3`, one collider index
per race (`149` on Moa Therma, `181` on Outpost 7, `196` on Talon's Junction: the player's box, the
only craft in a Time Trial), hit `1.30` to `1.39` below the probe's start while the projectile is
at the craft's height, and the floor (`kind 1`) on the first update whose start is clear.

| Capture | Weapon | Probe start ahead of the craft's body, by update | Box hit |
| --- | --- | --- | --- |
| Parked craft, Talon's Junction | Missile | 0.71, 1.42, 2.13, then 2.84 | first three |
| Parked craft, Talon's Junction | Rocket | 2.84 | none |
| Player at 100 units/s, Moa Therma start (floor) | Rocket | 2.81, 3.91, then 5.01 | first two |
| Autopilot at 140 units/s, Outpost 7 (floor) | Rocket | 2.7, 3.2, 3.6, 4.0, then 4.3 | first four |
| Autopilot at 125 units/s, Outpost 7 ramp (floor) | Plasma | 4.45, 4.64, 4.80, then 4.93 (1.1 below the body by then) | first three |
| Autopilot at 139 units/s, Moa Therma bank (mag floor) | Rocket | 2.83 | none |
| Autopilot at 139 units/s, Moa Therma bank (mag floor) | Missile | 0.7, then 0.8 behind, then 2.3 behind | first two |

So it is neither age nor place: it is how long the probe's start takes to clear the box. How far
ahead of the body's position (`body+0x30` read at the update's entry) the box reaches is **not one
number**: between 2.13 and 2.84 parked, 3.91 to 5.01 at 100 units/s, 4.80 to 4.93 at 125 units/s,
4.0 to 4.3 at 140 units/s, and under 2.83 at 139 units/s on the mag strip. Confidence **90** that a
craft box nearer than the floor makes the query return `0x7f`. **Why the reach is not one number is
settled 2026-11-12** in ["The own-craft box is a rotated frame"](#the-own-craft-box-is-a-rotated-frame-2026-11-12-pulse-own-craft-box)
below: the box sits exactly on `body+0x30` with the body's own axes, and the raycast's separating-axis
gate looks at it through the transposed rotation. (The two guesses this paragraph carried, a centre
one tick ahead of the body and a collider-sync order, are both wrong: centre and body agree to 0.001.)

One capture shows it is any update, not only the launch: the Outpost 7 centre rocket landed on the
ramp's face, was pushed back over its own craft, and its probe then met the craft's box on every
update until the cage ended it.

### The own-craft box is a rotated frame (2026-11-12, pulse-own-craft-box)

Question put to the original: where is the firing craft's box collider when the weapons update, so
that the first probes miss on it, and how far ahead of `body+0x30` does it reach? The claim under
test was "one box, centred some way from `body+0x30`, reaching X ahead at rest, at 100/125/140 units/s
and on the mag strip". What would have falsified a one-box answer: the reach differing between two
captures that the box's pose could not explain, which the section above already had (2.13 to 2.84
parked, 4.80 to 4.93 at 125 units/s). It did not survive: **there is no X**. (Order of work, plainly:
the second capture, a parked Missile on Talons Junction, already refuted any single reach; the
transposed-frame model was built from it and from the decompile, and the other 54 captures were
predictions made with it, not fits, which is why 0 errors in 2,302 probes carries weight.) The pose is exact; the reach
is not one number because `Collision_RaycastBox` tests the segment against the box through the
transposed rotation.

**Method.** `scripts/psp-projectile-sweeps.py --box` (PPSSPP v1.20.4, software renderer, the grid save
states in `data/saves/pulse-psp/`) snapshots, at each update's entry and at each query's return, the
body's 16-float pose and 0x1e0 bytes of the box collider (`body+0x3bc`), and reads
`world + index * 0xc + 0x2454` (world from the pointer at `0x08ab0820`) against the capture's
`mesh_collider`. `scripts/psp-craft-box-replay.py` replays every probe against the model below. 56
captures (Rocket, Missile, Plasma), each loaded twice from the same state. The two loads differ
(a player's held-button timing moves the pose at the fire by up to 0.1 unit, an Autopilot run by a few
tenths, one Missile run by 1.6 units a few updates in), so a pair is two independent fits of the law,
not one run copied.

**The pose is the body's, exactly** (confidence **95**):

| Read | Result |
| --- | --- |
| `table[mesh_collider]` against `body+0x3bc`, every probe that met a box | 352 of 352 equal; indices 149, 151, 175, 181, 190, 196, 201, 217 (the player's, one per circuit) |
| collider `+0x80/+0x90/+0xa0` (the rotation rows) against body rows `+0x00/+0x10/+0x20` | 1,712 snapshots, largest difference **0** |
| collider centre `+0xb0` against `body+0x30` at the update's entry | 82 % within 0.001; the rest by up to 0.24 unit, the largest on De Konstruct runs that end against a wall (a contact correction after the collider sync; not traced further) |
| collider at the update's entry against at the probe's return | identical in all 2,302 probe records |
| dimensions `+0x1d0/+0x1d4/+0x1d8` | `(4.125, 2.625, 9.75)` on every Venom capture and on one Assegai Zone state: the Misc width, height and length times `0.75` ([collision.md](collision.md), "The dimensions feeding the collider are scaled"); half dimensions `(2.0625, 1.3125, 4.875)` are the scaled axes at `+0xc0/+0xd0/+0xe0` |

So there is no lead, no lag and no offset to explain: the box is centred on `body+0x30` with the
body's axes, 9.75 long. The guesses in the section above (a centre one tick ahead; the collider-sync
order; the Autopilot's rail) are all closed: the same 0.000 relation holds for a parked craft, a
player at 100 units/s and an Autopilot craft at 139 and 151 units/s.

**`Collision_RaycastBox` (`0x08817b1c`) reads the box through the transposed rotation** (confidence
**92**). Disassembled end to end this pass:

1. `Collision_ClipSegmentToAabb` (`0x08815038`, was `FUN_08815038`): the segment against the collider's
   world AABB (`+0x00`, `+0x10`); no overlap, return. Never decided a probe in these captures.
2. Rows `+0x80/+0x90/+0xa0` and centre `+0xb0` are loaded with `lv.q C100..C130` (each memory row
   into a matrix column) and both segment ends, minus the centre, go through `vtfm4.q C000, E100, C200`.
3. A separating-axis test of that segment against the half dimensions (`+0x1d0..+0x1d8` times 0.5): three
   axis tests and the three cross products. Failing any one returns with nothing hit.
4. Only then the box's eight corners (`Collider_BoxSamplePoints`, `0x08818a00`, centre plus or minus
   the three scaled axes) and its twelve triangles (`Collider_BoxTriangleIndices`, `0x08818ab0`) go
   through `Collision_SegmentTriangle` in world space.

The frame of step 2 was read live: a breakpoint after each `vtfm4` on a parked De Konstruct
craft (yaw 25 degrees), 20 `Collision_RaycastBox` calls, 40 results against the matrix rows
(`sp + 0xa0`) and the input vector (`sp + 0x470`, `sp + 0x4d0`): **39 of 39 non-zero results equal
`R^T * v`**, the column-wise product `(v . col0, v . col1, v . col2)`, and none equals `R * v`
(the fortieth input is a zero vector). A frame that would put a point in the box's own axes is
`R * v` (each row dotted with `v`); the original builds the other one. The consequence is a gate
region that is the true box turned the other way: the gate box and the real box coincide when the
craft is aligned to the world axes (a multiple of 90 degrees of yaw, no roll) and are 2 times the
craft's angle apart otherwise.

**The model, and what it predicts** (confidence **90**): a probe meets the box iff

- the segment `start` to `start - normal * length` (`start` the projected position; length `6.0` for the
  Rocket, `12.0` for the Missile, Plasma and Shuriken) passes the separating-axis gate in the
  `R^T` frame, and
- it crosses a face of the true oriented box (`centre`, rows, half dimensions), and
- that face is nearer than the mesh hit, if the mesh was hit.

Replayed against all 2,302 probes (352 of which met the box, 1,950 did not), **0 mispredicted**.
The same replay with the correct frame (`--frame rows`) mispredicts **63**, every one on a craft yawed
or rolled off the world axes by 8 degrees or more:

| Captures | Probes | Meet the box | `R^T` frame wrong | `R` frame wrong |
| --- | ---: | ---: | ---: | ---: |
| Parked, six circuits (Talons Junction and Vertica Black 28.5 degrees off the world axes, the other four within 1.7) | 525 | 42 | 0 | 5 |
| Player, Moa Therma White start, 100.2 units/s | 73 | 6 | 0 | 0 |
| Player, De Konstruct White (25 degrees off the world axes), 71, 112, 66 and 3 units/s | 537 | 89 | 0 | 35 |
| Player, Moa Therma White and Arc Prime White (10 and 12 degrees off), 6 to 66 units/s | 590 | 166 | 0 | 10 |
| Autopilot, Moa Therma White bank (85 degree roll, mag floor), 139 units/s | 320 | 4 | 0 | 8 |
| Autopilot, Outpost 7 (floor, 8 and 13 degrees off the world axes), 151 to 160 units/s | 257 | 45 | 0 | 5 |

The discriminating records, three decimals, for a test (rows are `+0x80/+0x90/+0xa0`; `start` is the
probe's projected position; the same box twice, one apart, is the sharpest pair):

| Capture, update | Rows | Centre | Start | Carried normal | Length | Original |
| --- | --- | --- | --- | --- | ---: | --- |
| Talons Junction parked, Missile 3 | (-0.879, -0.012, -0.477) (-0.004, 1.000, -0.018) (0.477, -0.014, -0.879) | (5.643, -50.083, -196.670) | (6.655, -50.155, -198.533) | (-0.003, 1.000, -0.019) | 12 | box, `0x7f` |
| Talons Junction parked, Missile 4 | the same | (5.642, -50.083, -196.670) | (6.995, -50.208, -199.161) | the same | 12 | mesh, floor (1) |
| De Konstruct White, 112 units/s, Rocket 1 | (0.897, -0.101, -0.431) (0.127, 0.991, 0.033) (0.424, -0.085, 0.902) | (84.827, -26.447, 70.351) | (85.978, -26.678, 72.801) | (0.128, 0.991, 0.016) | 6 | box, `0x7f` |
| De Konstruct White, 112 units/s, Rocket 4 | (0.896, -0.107, -0.431) (0.133, 0.991, 0.030) (0.424, -0.084, 0.902) | (85.621, -26.590, 72.051) | (87.166, -26.930, 75.332) | (0.128, 0.991, 0.016) | 6 | mesh, floor (1) |
| Moa Therma White bank, 139 units/s, Rocket 1 | (0.064, -0.997, -0.032) (0.397, 0.055, -0.916) (0.916, 0.046, 0.399) | (251.772, 124.247, -349.078) | (254.322, 124.351, -347.969) | (0.393, 0.054, -0.918) | 6 | mesh, mag floor (3) |
| Moa Therma White bank, Missile 2 | (0.065, -0.997, -0.032) (0.394, 0.055, -0.917) (0.917, 0.047, 0.397) | (253.130, 124.310, -348.487) | (252.295, 124.236, -348.851) | (0.394, 0.053, -0.917) | 12 | box, `0x7f` |
| Outpost 7, 160 units/s, Plasma 1 | (-0.184, -0.060, -0.981) (-0.462, 0.886, 0.032) (0.868, 0.459, -0.191) | (-1031.705, 7.034, 347.084) | (-1028.295, 9.891, 345.924) | (-0.377, 0.926, -0.000) | 12 | box, `0x7f` |

**Every bracket, explained.** On an axis-aligned craft (Moa Therma's and Arc Prime's grid, Metropia,
Fort Gale, Basilico) a start dead ahead hits to 4.875 and not past it: the 100 units/s Rocket reads
2.83 (three rockets of the volley), 3.92 (three), then 5.02 clear, the bracket `3.91 to 5.01` on this
page. On Talons Junction (28.5 degrees off the world axes) the gate box is turned 57 degrees from the real
one and the parked Missile hits at 0.70, 1.41, 2.12 and misses at 2.84, the old page's `0.71, 1.42,
2.13, then 2.84` (re-measured, same numbers). The reach of a start dead ahead of the centre, straight down, by yaw off the world axes (this
model, a Venom): 4.87 at 0 to 10 degrees, 4.12 at 15, 3.20 at 20, 2.48 at 28, 2.19 at 35, **2.06 at 45**,
2.38 at 60, 4.87 again at 90. That is the `2.13 to 2.84` parked (Talons), the
`4.80 to 4.93` of an aligned craft (4.875 inside it), and the `4.0 to 4.3` of a craft 13 to 17
degrees off the world axes (this is a fit, not a re-read: that capture is gone, and the fresh Outpost 7
Autopilot craft sat at 8 and 13 degrees). On the
mag strip the craft is rolled 85 degrees; in the gate frame the start's height is `-2.49`, past the
half height 1.3125, so the gate rejects it and the Rocket rides on its first update (it did, in both of its
mag-strip captures, 139.1 units/s), while the Missile's first two starts (0.70 ahead, 0.91 behind) pass and its third
(2.55 behind, gate height 2.27) does not. The old records could not be re-read (they were scratch, wiped
after the lane that took them); the parked Talons sequence and the 100 units/s start were reproduced
to the unit, and the Autopilot rows are covered by their own fresh captures above.

**Also settled:**

- The gate is on the world axes, not the track's: a craft's box reach depends on its **world**
  heading. A wire must not approximate it with a forward distance.
- A probe that meets a box leaves `code` at `0x7f` and `end` at the full segment, so the cage is asked
  about the full end: 199 Missile probes (De Konstruct 63, Moa Therma 28, Arc Prime 108, all start straights)
  returned **code `4` with kind `3`** (the 12 unit end is outside the tube there), and the Missile does nothing on it. Our wired cage test already
  asks about `to - surface * length` when the mesh is not hit; keep that.
- **The travel sweep follows the same law, but a meeting cannot be told from a stale slot** (confidence
  **65**). It is the same `Collision_SweepSegment`. Its hit-kind slot is written only on a hit, so a
  sweep that hits nothing still reads the last value there: 442 sweeps read kind `3`, 346 of them with
  no predicted meeting and (345 of the 346) a probe that read `3` in the same update. Replayed from the
  entity's position (not the stale previous-position word read at the update's entry) to the projected
  position with the same gate, the model predicts a meeting on 96 sweeps and **none of them read
  anything but `3`**; nowhere does a predicted meeting read as a mesh hit. That cannot separate a hit
  from carry-over on those 96, so the sweep is wired by sharing the function, not by a measurement of
  its own.
- Any craft's box, not only the firer's, is in the world's collider table, so the same gate applies to
  an opponent's (confidence **75**: one craft per Time Trial, all 352 meetings were the player's
  index; the code path has no owner test, `Collision_RaycastWorld`'s `-1` skips nothing).
- One title build (Pulse PSP USA) and one box size; the dimensions of other classes are the
  Misc times 0.75, read from the collider, not assumed.

**Wired 2026-11-13** (lane `pulse-own-craft-box`: `projectile::own_box_blocks`, `OwnCraftProbe`, `Weapons::own_craft_box_hides_track`; tests in `docs/gameplay/projectile-floor.md`, "Wired: the own-craft box"). The recipe it followed:
The probe in `projectile::flight` asks `Raycaster::raycast(Ray::new(to, -surface, length))` and falls on
`None`. The Plasma's `age < UNPINNED_SECONDS` gate (chosen, 0.04 s; wrong on the mag
strip, right by luck on an aligned floor) was replaced by one function every flyer shares, so the Rocket, whose
first two to four probes fall in the original and ride in ours, gets it too:

1. For each craft in the race (the firer's included; the Cannon has no probe), the box is centre =
   the body's position, rows = the body's rotation rows in the **original's order and signs** (row 0
   is `(-1, 0, 0)` for a craft facing -z on Moa Therma White's grid; our `Body` basis must be checked against
   the records above before use, because the gate is not symmetric under flipping one row), half dimensions
   `(width, height, length) * 0.75 * 0.5` of its `<Misc>`.
2. Gate: `local = (d . (r0.x, r1.x, r2.x), d . (r0.y, r1.y, r2.y), d . (r0.z, r1.z, r2.z))` for `d` =
   segment end minus centre, the three-axis plus three-cross separating-axis test against the half
   dimensions (`scripts/psp-craft-box-replay.py::separating_axis_passes` is the reference), then the
   nearest face crossing of the true oriented box.
3. If the nearest crossing is nearer than the mesh hit (or there is no mesh hit), the probe is `None`
   (code `0x7f`): the Rocket and Plasma fall, the Missile and Shuriken do as they do on `None` today.
   The travel sweep (position to projected position) asks the same function: a box nearer than a wall
   hides the wall from a projectile launched inside the box.
4. Test: the eight records above (red on the `R` frame for the Talons and De Konstruct rows and the
   mag strip, red without the box for all box rows), then `rocket_floor_trace` and the Outpost 7 ramp
   test re-run; expect the Rocket's first one to three probes to fall on an aligned grid and the
   Plasma's `UNPINNED_SECONDS` constant to go.

Names applied (2026-11-12): `Collision_ClipSegmentToAabb` (`0x08815038`, 78: the signature
`(start, end, min, max, out0, out1)` and the AABB corners it is handed, the slab divide and clamp in
the VFPU body; never decided a probe here, so its behaviour is read, not exercised) and
`Collider_BoxTriangleIndices` (`0x08818ab0`, 90: writes twelve triples of corner indices
`(1,5,7) (7,3,1) (7,5,4) (4,6,7) (7,6,2) (2,3,7) (2,0,1) (1,3,2) (4,5,1) (1,0,4) (6,4,0) (0,2,6)`,
the box's triangle list). `FUN_08814e34`, called with the rows and not read afterwards, stays unnamed
(a matrix inverse by its shape, result unused).

**Cross-title.** Pure shares Pulse's engine: unread (**not checkable** this lane, no Pure capture),
the wire is Pulse and PS2 by inheritance. HD, 2048 and Omega are separate code with their own projectile
queries: unread; **checked, differs** is not claimed.

### The launch normal is the track's, not the craft's

`-(craft+0xb10)`, which `Rocket_Init`, `Missile_Init`, `Shuriken_Init` and `Plasma_Launch` store as
the carried normal, is **the down axis of the track point the craft is located on**, negated: on
the Outpost 7 volley the stored normal is `(-0.000, 1.000, 0.000)` where the craft's own up is
`(-0.014, 0.996, -0.090)`, 5.24 degrees off, and it is within 0.04 degrees of `-down` of the nearest
spline sample on every Rocket, Missile and Shuriken capture (0.29 to 0.86 for the two Plasmas,
whose normal is taken at the release, against a resampled spline). `scripts/psp-autopilot-trace.py`
already reads the same field as "the located sample's down". This engine seeds `body.up()`.
Confidence **88**: seven captures, one field; the located point's own record was not read beside it.

### By-catch in the flight arms

- **The travel-sweep floor arm can throw a rocket upward.** `Rocket_Update` writes
  `(hit + 3.0 * normal - prev) / dt` unnormalised there. Fired 14 units short of Outpost 7's ramp
  face, the centre rocket landed on the face at update 7 and left it at `(-65.0, 155.6, -7.1)`,
  168.7 units/s, straight up and slightly back; with its probe blocked by its own craft's box it was
  never rescaled and climbed until the cage ended it ten updates later. Ours turns the velocity
  parallel and keeps the speed (chosen, on this page since 2026-09-13), so ours rides the ramp.
  One-update error there: 4.26 units.
- **The Shuriken's speed is not constant** and **the Missile's floor arm is the Rocket's steer**:
  [shuriken.md](shuriken.md), [missile.md](missile.md).

### What this retires

- "`4` is a second query, the craft test" (this page, "Flight follows the track"; `missile.md`'s 55).
  It is the cage. A projectile meets a craft through its pool's own craft sweep
  (`Rocket_SweepCraftHit` and its siblings), never through this query.
- "Jump lips" and "a rocket that leaves the circuit flies its whole cap" as the recovered model's
  own behaviour (`projectile-floor.md`).
- `plasma::UNPINNED_SECONDS` as an age: the bracket it was chosen inside was measured on one craft
  at one speed. On the mag strip the original's bolt rides on its first update.

### Wired (2026-11-11, `projectile-cage-wire`)

`oag_weapons::projectile` asks the race's cage (`oag_physics::cage::Containment`, implemented by
`oag_gameplay::cage::CageTrack`; `None` where the title runs no cage) about the probe's end and the
sweep's end, per the table above: [projectile-floor.md](../../../gameplay/projectile-floor.md)
lists what each weapon does. The craft's per-corner rule is now `Located::outside`, the same one
the projectile's one-point query uses. The 17 rows of "The cage test reproduced" and the five
"States a test can start from" are tests with full-precision values
(`crates/raceplay/tests/projectile_cage_ground_truth.rs`): all 17 outside at the original's boundary
point (0.01 unit) and normal (cosine above 0.9999); the Plasma ends on update 49, the Cannon round
on 25, the Outpost 7 rocket on 21, each on the original's own per-update `dt` (a single listed `dt`
drifts a whole update by the Rocket's 15th: 0.0171 against a mean of 0.0166). **The Shuriken
could not be chained from update 50** (until the floor arms were wired, below): ours kept the blade's speed where the original's changes
(item 5 of the thread; 276.9 units/s at update 52, 298.9 at 53), so by update 54 it is 0.3 unit off
the original's line, meets the kerb's wall at update 58 and bounces instead of ending, and the test re-seeds each update from the original's own state
instead. `rocket_floor_trace` on `07` and `10`: the rockets that reached their five seconds went
from 14 and 7 to 0, and 78 and 101 of 174 now end at the cage (0.0 to 4.1 units outside, median
about 1), the rest on walls and hulls as before. HD and Omega, 2048: `Spline::cage()` is `None`,
unchanged.

### Wired: the launch normal (2026-11-12, `launch-normal-wire`)

`Ship::track_up` holds `-down` of the spline sample `step_player` and `step_opponents` hand the
force law (`record_track_up`: the nearest sample for a person, the driver's own for an opponent),
written once a step, so a weapon spent before the step reads the previous tick's, as the original
reads a `craft+0xb10` the last locate wrote. `Craft::launch_normal` is that, or the craft's up where
it is `None` (before the first step, a track with no samples, a title whose
`Weapons::launch_normal_is_track_up` is `false`). `rocket::fire`, `shuriken::fire`, the Missile's
`spawn_guided` and the Plasma charge's per-tick reseat (`flight.rs`) take it. Left on the craft's up:
the Cannon (`Cannon_Init` copies the muzzle anchor's basis; no probe reads its normal), the
Disruptor (its advance was never read; it is not one of the four inits), the Rocket's fan axis, the
Shuriken's turn axis and the Plasma's muzzle offset (positions and orientations, not the carried
normal). Test: `projectile_cage_ground_truth::outpost_7_ramp_launch_normal_is_the_tracks_up` (the
craft posed on sample 186 with the capture's up, a Plasma press through `spend_pickup`; the charging
bolt carries the sample's `-down` to 0.05 degrees, 5 degrees off the craft's up) and
`seed::a_plasma_bolt_is_released_riding_the_located_track_up`. **Sample 186's `-down` is 0.26
degrees from world up**, so the capture's `(0, 1, 0)` is the print's rounding, not an exact axis.
The Rocket, Missile and Shuriken are not observable in a raceplay test: their first update, in the
firing tick, replaces the seed with the floor's normal; the handed-in normal is pinned by
`rocket::tests::a_rocket_is_seeded_with_the_normal_it_is_handed`.

Per title (`Weapons::launch_normal_is_track_up`, `docs/gameplay/weapon-lineage.md`): Pulse
**measured** `true`; Pure **inherited** from Pulse (the affected Pure tests stayed green); **HD,
2048 and Omega measured `true`** since 2026-11-12: HD's `Rocket_Init` and its siblings, Omega's and
2048's `Plasma_Init` store the same located record's down negated
([ps3-hdfury-eu/projectile-launch-normal.md](../ps3-hdfury-eu/projectile-launch-normal.md), 86, 80
and 75). Turning it on there moved three tests at their margins; that page's "Landed" section has
the distributions they were re-based on.

### Wired: the floor arms (2026-11-12, `pulse-floor-arms`)

`Weapons::floor_arms_steer_to_ride_point` picks `projectile::FloorArms` per title. On Pulse and
Pure (`SteerToRidePoint`) the Shuriken's probe arm and the Rocket's and Shuriken's travel-sweep arm
write `(ride point - start) / dt` unrescaled (`start` is the update's entry position), the Missile's
probe arm steers to the ride point as the Rocket's does, and the Plasma's travel-sweep arm only moves.
`moa_therma_white_shuriken_ends_at_the_cage` chains the Shuriken from update 50: the velocity at
each next entry is within 2 units/s of the recorded one and the blade ends on the cage at update 58
(without the law: 278 against 299 units/s at update 52, and a bounce on the kerb). The other three arms
are unit tests on a flat floor (`crates/weapons/src/projectile/tests/floor_arms.rs`), not recorded states.
**Cross-title**: HD's `Plasma_Update` is its own function (ride height 4.0, above) and its travel
segment only destroys on `0`, `4`, `5` (the pseudocode above writes no floor arm): **checked,
differs** for the Plasma sweep, and HD, 2048 and Omega keep the parallel turn (**not checkable**
for their Rocket, Missile and Shuriken arms: unread, named in the thread).

### Cross-title

- **Pulse PS2**: inherits the PSP law (not read on the PS2 build; **not checkable** cheaply, no PS2
  page names these functions).
- **Pure**: the cage flag is clear on all 16 Pure tracks ([collision-cage.md](collision-cage.md)), so
  `g_cage_enabled` is cleared by the first craft pass and the query returns 0: **checked, differs**
  (no cage; whether Pure's projectiles leave the circuit is unmeasured).
- **HD/Fury**: the original runs no cage off the track header
  ([ps3-hdfury-eu/collision-cage.md](../ps3-hdfury-eu/collision-cage.md), 85), and HD's
  `Plasma_Update` still switches on `0`/`4` ([ps3-hdfury-eu/plasma.md](../ps3-hdfury-eu/plasma.md)).
  What returns `4` there is unread: **not checkable** in this lane (HD's sweep function is not
  named; its cage module decodes soft in Ghidra). A wire must take the cage from the race's own
  `Spline::cage()`, which is `None` on HD, so HD does not gain Pulse's containment by sharing code.
- **2048, Omega**: follow HD's lineage (no cage off the header): **not checked** against their own
  executables.

### Names applied (2026-11-11)

| Address | Kind | Name | Conf | Evidence |
| --- | --- | --- | ---: | --- |
| `0x08832134` | function | `Cage_TestSweepEnd` | 92 | decompiled end to end; 1,187 live calls reproduced through the ported cage, the 17 boundary points to 0.000 |
| `0x08831948` | function | `Cage_QuerySweepEnd` | 90 | one call: `Cage_TestSweepEnd(g_cage_effect, ..)`; one caller, `Collision_SweepSegment` |
| `0x0883198c` | function | `Collision_SweepSegment` | 92 | was 85 for the unread second query; now read, and every return code of 1,187 live calls accounted for |

History: `Collision_SweepSegment` 85 (2026-09-13, mesh path read, second query inherited at 55) to
92 (2026-11-11, second query decompiled and reproduced live; one binary).
