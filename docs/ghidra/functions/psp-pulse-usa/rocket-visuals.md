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
| `0x0886ebdc` | `Rocket_HitCraft_q` | 78 |
| `0x0886ed34` | `Rocket_SpawnCraftExplosion_q` | 78 |
| `0x08915484` | `Psys_Spawn_q` | 72 |

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
(`PROJECTILE_SPRITE_HALF_SIZE`, `crates/game/src/race.rs`). The *scale* is close;
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

So the model is aligned to **where it is going and what it is flying over**, with
a fixed quarter-turn correction.

**The axis of that quarter-turn is not resolved here.** `func_0x002676b4`
resolves to `0x08a6b6b4`, which is outside the band the other resolved calls on
this page land in and just below the string table - it has the shape of an import
stub, and `scripts/resolve-psp-imports.py` is the tool for it. Until it is
resolved, the quarter-turn is recorded and **not** reproduced: forward alignment
is fully evidenced, the extra rotation is a model-space convention we cannot yet
name.

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
than inferred from the names.** `Rocket_HitCraft_q` (`0x0886ebdc`) is the
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

**Now implemented, in its own commit.** `oag_gameplay::projectile` probes toward
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

Until 2026-09-13 `oag_gameplay::projectile` decided wall-versus-floor from the
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
`oag_gameplay::projectile::launch` divides by
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

## Audio, in passing

`Rocket_Init` allocates a `0x70`-byte emitter, sets `+0x38 = 0x44160000`
(`600.0f`, a rolloff distance) and starts a looping sound at volume `1.0` named
by the string at `0x08a7c0d0`: **`~ROCKETTVL`**. The neighbouring `~PLASMATVL`
confirms the `<WEAPON>TVL` - travel - pattern.

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

## What is not verified

- **The quarter-turn's axis**, blocked on resolving `0x08a6b6b4` - see above.
- **`WO_ROCKET_FLARE`'s parameters**, blocked on the `.pob` payload layout.
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
