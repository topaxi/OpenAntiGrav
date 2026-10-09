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
said. So the three arms above are: **no hit → fall; wall or craft → detonate;
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
(`crates/game/tests/rocket_launch_ground_truth.rs`, disc-backed), because our own
standing start lands the craft at z -199.4 heading 1.7 degrees toward the -Z wall where
the original's is at z -197.0, which alone moves the detonation by 30 ticks.

| Law | Source | Ours now |
| --- | --- | --- |
| cruise speed is the class's, 222.22 u/s on Venom | live, two probes | `rocket::launch` |
| launch speed is 0.75 x that until the first surface-probe hit | live (rows 0.7500) | `LAUNCH_SPEED_SCALE`; class speed rides in `Projectile::launch_speed_kmh` |
| spawn at the craft's own position | live, to 0.1 unit | `launch` |
| the stored normal `self+0x100` is seeded from the craft (`-(craft+0xb10)`), the first probe goes along it | decompiled `Rocket_Init`; live rows `(-0.03, 1.0, 0.05)` | `Projectiles::spawn_riding` seeds the craft's up (whether `+0xb10` is the craft's up or its contact normal was not separated) |
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
