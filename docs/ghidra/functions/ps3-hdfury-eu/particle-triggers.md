# HD's particle effects: which ones the executable names, who spawns them, what is wired

2026-10-06, `hd-particle-triggers`. Binary `/hdfury/EBOOT-ps3-hdfury-eu.elf`. The
question was "HD authors 82 effects and only 2 are wired - find the rest of the
triggers". **The premise was out of date**: `docs/overview/status.md` section 4
said 2 of 88, but `oag_raceplay::RACE_EFFECTS` already held 33 names HD's disc
carries and a plain HD race **loads** all of them (the load report misses only
`WO_MAGSTRIP_SPARKS/ZONE`, `WO_RAIN`, `WO_RAIN_LENS`, `WO_SNOW`, which HD does not
ship); with the Cannon spark below it is 34. Loading is not firing: the LeachBeam hit
spark (`hit_sparks`) and the wreck trio (`wreck_fx`) are Pulse-gated and do not fire
on HD, and `WO_BLUE_WELDER` / `WO_MODESTO_STEAM_A` fire only if an HD circuit places
them as `ParticleSystem` nodes, which was not checked for HD. The rest are mostly
**Pulse-inherited** (title-neutral weapon visuals); what this page adds is which of
those have an HD function behind them, and the effects that have none wired.

## The scan (reproducible, validated)

For each name string in the image: find the string, find every 4-byte word holding
its address (a TOC slot), then every function with a `lwz/ld/addi d(r2)` whose
`r2 + d` is that slot, using each function's own `r2` from the program context
(`AssignPs3R2FromOpd.java`). This is `ship-collision-fx.md`'s technique as one
script over all 52 `WO_` strings the executable carries (`strings -a`).
**Validated on the known answer first**: `0x007a1c68` -> slot `0x008b3f6c` ->
`0x002d9730` (`ShipCollisionFx_Trigger`), nothing else. Results:
The TOC scan (scratch, not committed). A name
whose slot is in a data table (`0x008a9xxx`-`0x008abxxx`, the weapon tables) has no
function load: that is a third hop, listed as "table" below, not as "no trigger".

**The name-to-disc diff.** The disc ships 84 file names (82 internal names); the
executable carries 52 of them as plain strings. The other **31** have no
name-reachable trigger and are recorded as exactly that, not as "no trigger":
`WO_BLUE_WELDER`, `WO_BOMB_SHOCKWAVE_FLASH`, `WO_CANNON_HOTSPOT`,
`WO_CANNON_MUZZLEFLASH`, `WO_DAMAGE_PLUME`, `WO_DUSTMOTES`, `WO_ENGINE_FLARE`,
`WO_ENGINE_JETFLARE`, `WO_LEACHBEAM_BALL_SPARKS`, `WO_LEACHBEAM_CHARGING_SPARKS`,
`WO_LEACHBEAM_EMIT`, `WO_LEACHBEAM_ENERGY`, `WO_LEACHBEAM_ENERGY_SPRAY`,
`WO_LEACHBEAM_HITSHELL`, `WO_MAGSTRIP_LIGHTNING`, `WO_MISSILE_LAUNCH`,
`WO_MODESTO_STEAM_A`, `WO_PLASMA_FLASH`, `WO_PLAYER_DAMAGE_PLUME`,
`WO_PULSECANNON_MUZZLE`, `WO_QUAKE_DETONATOR_TRAILS`, `WO_SHIP_DEATH_SPARKS`,
`WO_SHIP_ENGINEFLARE`, `WO_SHIP_EXPLOSION_DEBRIS`, `WO_SHIP_EXPLOSION_LIGHTSHAFTS`,
`WO_SHIP_EXPLO_SMOKE`, `WO_SHIP_FXNODE_BIGEXPLO`, `WO_SHIP_FXNODE_EXPLO`,
`WO_UNDERWATER_GODRAYS`, and the three debug files `NUMBERS`, `STESPARKSTEST`,
`TEST_BOMBSPIKES`. Placed, attached and ambient effects leave no string
(`psys_inventory_ground_truth.rs`'s `mod hd` says why), and a name can be built from
a shared prefix. `WO_SHIP_ENGINEFLARE` (engine flare) and `WO_BLUE_WELDER` /
`WO_MODESTO_STEAM_A` (placed scenery) are wired by other means already.

## Census: Pulse's trigger against HD's

"By name" means the HD effect shares Pulse's name; "by role" would be a guess and
none is claimed. Confidence is the rubric's; "wired" is what an HD race fires
today after this change.

| Effect (HD name) | Pulse trigger | HD trigger read | Fires on HD |
| --- | --- | --- | --- |
| `WO_SHIP_COLL_SPARK_DAMAGE` | wall contact (attached, looping) | **created once per ship in the ship constructor**, `0x000ddd58` / twin `0x000dfd90`, attached (`FUN_002c9108(..., 1, 0)`), conf 80 | yes (attached mode), Pulse-inherited |
| `WO_SHIP_COLL_SPARK_DAMAGE_ZONE` | none | the same two constructors pick it when `mode < 22 && (1<<mode) & 0x206040` and the net flag is clear (modes 6, 13, 14, 21), conf 80 | **no**: this port has no map from those ids to its modes (`zone-sky.md`) |
| `WO_SHIP_SPARK_DAMAGE_WEAPON` | none (Pulse sparks from `Ship_Damage`) | `Cannon_ApplyCraftHit` -> `Ship_DispatchCollisionFx(kind 1)` -> `ShipCollisionFx_Trigger` kind 1, conf 72 | **yes, new** (`race::hit_sparks::throw_weapon_spark`) |
| `WO_SHIP_COLL_SPARK_NODAMAGE`, `WO_SHIP_SPARK_NODAMAGE_ZONE` | floor/magstrip contact, read, unwired | `ShipCollisionFx_Trigger` kind 2 (zone mask picks the `_ZONE` name); **read 2026-11-23**: called by `Ship_SpawnContactFx` (`0x000e6fd8`, sets kind 2 at `0x000e7730` on frictionless records while racing: floor, mag floor and reset), conf 85 ([collision-cues.md](collision-cues.md), "2026-11-23: which records carry no friction, and the spark they throw") | no |
| `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` | a LeachBeam drain's hit (`Ship_Damage`) | **resolved 2026-10-07:** `Ship_ApplyDamage` (`0x000e7760`) sets `object + 0x260` on a kind 7 hit and `0x002a06e0` (also the engine-flare enqueue) clears it and spawns the effect attached at `+0x1c0`, conf 78 | **yes, HD only** (`race::damage_fx`), once per hit |
| `WO_WEAPON_ABSORB` | `Ship_PlayAbsorbFeedback` | `ShipAbsorbNode_SpawnBurst` `0x002d8d10` (`absorb-feedback.md`) | yes (HD path) |
| `WO_TRAIL_HITSHIP`, `_RED` | (HD-only) | `0x002d9ec0` (`engine-trail.md`) | yes |
| `WO_ROCKET_EXPLO` | craft/track hit | `0x0014a7d8` | yes, Pulse-inherited; HD site found; the wall half (`_TRACK`) plays HD's own `.gtf` sprites since 2026-10-08, the craft half since 2026-11-26 ([particle-fill.md](../../../rendering/particle-fill.md#with-its-sprites)); the spawn (`Rocket_SpawnCraftBlast`) copies a point and hands the blast the struck craft's velocity, wired 2026-11-26 ([rocket-craft-blast.md](rocket-craft-blast.md)) |
| `WO_MISSILE_EXPLO` | detonation | `0x00155568` | yes, Pulse-inherited |
| `WO_MINE_EXPLO` | detonation | `0x00140710` | yes, Pulse-inherited |
| `WO_SHURIKEN_EXPIRE` | teardown | `0x0014d4f0` | yes, Pulse-inherited |
| `WO_REPULSER`, `WO_REPULSER_BLAST` | `Repulser_SpawnWaves` / `_SpawnBlastEffect` | `0x001577d8`, `0x00157b20`, `0x00158288`; `0x00158818` | yes, Pulse-inherited |
| `WO_PLASMA_LIGHTNING_EXPAND/COLLAPSE` | (HD-only) | `0x00127cd0`, `0x00127770` (`plasma.md`) | yes |
| `WO_LEACHBEAM_ABSORB` | (HD-only) | `0x00114a00`/`0x00114c78` (`weapons.md`) | yes |
| `WO_BOMB_SMOKERING` | `NormalBombBlast_Start` `0x00151538` at the detonation, spawned through `RaceManager` with TOC `-0x2350` (`weapons.md`, 2026-10-07) | the blast's matrix | yes, Pulse-inherited; HD site read |
| `WO_BOMB_RAYS` | `NormalBombBlast_Update` `0x001503d8` once, the first tick `age > 0.5` (`0x150fc8`, tag `'BORS'`, string TOC `-0x2374`) | the blast's matrix | **yes, HD only** (`Trigger::BombRays`, `bomb_blast::hd`) |
| `WO_LEACHBEAM_CHARGING`, `WO_ROCKET_FLARE`, `WO_ROCKET_EXPLO_TRACK`, `WO_MISSILE_HEAD/BOUNCE`, `WO_SHURIKEN_HEAD/BOUNCE`, `WO_PLASMA_HEAD`, `WO_CANNON_SPARKS`, `WO_QUAKE` | weapon visuals | slot in a weapon data table (third hop); `WO_LEACHBEAM_CHARGING` also loaded at `0x001134f0` | yes, Pulse-inherited, HD site unread |
| `WO_LEACHBEAM_LAUNCH`, `_HIT_TARGET`, `_BREAK` | none | spawn functions named (`weapons.md`, 82/82/78); **no code xref reaches them**: their OPD entries `0x00875a90/80/78` are only referenced from the OPD run itself, so the caller is a vtable or table not found | no |
| `WO_DAMAGE_MILD/MODERATE/CRITICAL` | none | `ShipDamageFx_Update` `0x002a17e8` off a weapon hit; thresholds 70 and 40 read, conf 72 | **yes, HD only** (`race::damage_fx`) |
| `WO_DAMAGE_ELECTRIC`, `WO_SHIP_DEATH_DAMAGE_PLUME` | none | the Zone-mask and `param_4 != 0` branches of the same function, unwired | no |
| `WO_DEBRIS_FIRE` | none | one load, `0x0010c958`, unread | no |
| `WO_DEBRIS_SPARKS`, `WO_NITRO_DEBRIS_SPARKS` | none | table slot, unread | no |
| `WO_BOMB_EXPLO_DETONATOR`, `WO_MINE_EXPLO_DETONATOR(_DEAD)` | none | **read 2026-11-22**: `DetonatorBombBlast_Start` `0x00137150` on any bomb detonation; `DetonatorMine_Explode` `0x00138e58`, `_DEAD` for a darkened mine (every craft contact, every earlier-stage mine), the plain one for a live mine destroyed otherwise, conf 72 ([detonator-mine.md](detonator-mine.md), [detonator-bomb.md](detonator-bomb.md)) | no: the Detonator hit is not built in `oag-weapons` |
| `WO_QUAKE_DETONATOR` `0x0013afe8/0x0013b5e8/0x0013b930` (read 2026-11-24: the EMP's wave, [detonator-emp.md](detonator-emp.md#what-hd-draws-for-a-fired-emp-2026-11-24), wired on HD), `WO_CANNON_SPARKS_DETONATOR`, `WO_LIGHTBARRIER_EXPLO`, `WO_NITRO_SHIP_DEATH` | none | functions found, bodies unread, except `WO_CANNON_SPARKS_DETONATOR`: a pulse round's death on a wall, `PulseBullet_Update` `0x00122608` ([detonator-cannon.md](detonator-cannon.md)) | no: the Detonator weapon set is not built in `oag-weapons` (and `WO_NITRO_SHIP_DEATH` is refused at blend class 4) |
| `WO_SHIP_EXPLOSION` | a wreck (`wreck_fx`, Pulse-gated) | table slot `0x008a8e84` | no on HD: `FXNODE_EXPLO` and `DEATH_SPARKS` are not strings in the executable |

## What was recovered

### The wall spark is a per-ship attached effect, made in the ship constructor

`0x000ddd58` (and its near-copy `0x000dfd90`, which differs only in a local copy of
`param_6` and the skin-search loop) is the ship constructor: it installs the vtable
(`PTR_PTR_008a8cfc`), zeroes about 8,000 words of state, loads the skin
(`Ship_ReloadModelForSkin`), `ShipAbsorbShell_Load`, `Ship_LoadEngineLightData` and
`Ship_GatherAbsorbNodes`. Near its end it allocates a `0x180`-byte effect object and
calls `FUN_002c9108(obj, name, 0x44535053, matrix = ship + 0x7d40, 1, 0)`. `name` is
TOC `-0x4504` (`0x008a8fd4` -> `WO_SHIP_COLL_SPARK_DAMAGE_ZONE`) when
`!*net_flag && mode < 0x16 && (1 << mode) & 0x206040`, else `-0x4500` (`0x008a8fd8` ->
`WO_SHIP_COLL_SPARK_DAMAGE`). The trailing `1` is the attach flag: a kind 1 spawn in
`ShipCollisionFx_Trigger` passes `0`. Confidence **80**: both names, the mask, the
fourcc and the attach flag are direct reads at the instruction level; that this
object is what a wall contact toggles is the old inference and still is one. This
answers `ship-collision-fx.md`'s open "what names the plain
`WO_SHIP_COLL_SPARK_DAMAGE` on this binary": the constructor, not a sibling of
`ShipCollisionFx_Trigger`, and `_DAMAGE_ZONE` has the same owner.

### The Cannon's craft-hit spark: one burst, no cooldown

`ShipCollisionFx_Trigger` kind 1 (decompiled again 2026-10-06) draws two random
angles, `U(-pi/2, pi/2)` and `U(-pi, pi)` (the constants at TOC `0x008b3f58`:
`0xbfc90fdb 0x3fc90fdb`, `0xc0c90fdb 0x40c90fdb`), builds a spawn matrix, allocates
`0x180` bytes and spawns `WO_SHIP_SPARK_DAMAGE_WEAPON` with the attach flag `0`. The
only gate is `*(craft + 0x5f42) == 0`; unlike Pulse's `instance + 100` there is no
per-locator cooldown in this branch. The file is not looping (`flags 0x46000012`,
5 ticks, two child emitters), consistent with a one-shot. Confidence **72** (the
`Cannon_ApplyCraftHit` chain, the literal `1`, the attach flag). **Wired**: HD only,
the nearest of at most ten `Ship Collision Fx` locators to the contact, severity
`1.0`, the locator's own up axis. **Chosen, not measured:** the severity, and the
roll (the two angles' axes were not read). Live: `--race --opponents --give cannon
--hold cross --press square` on HD fires 114 bursts in 900 ticks;
A crop at tick 42 (not kept) shows the white spark streak
on the struck craft one tick after a hit.

### The damage smoke: `0x002a17e8`, read in full 2026-10-07

`engine-trail.md` called it the engine-sound state machine and the first reading
here "a damage-state effect updater" with the thresholds unread. Both are
corrected below (disassembly read, not only the decompile, which truncates twice
on AltiVec ops).

**Caller.** `Ship_ApplyDamage` (`0x000e7760`, `param_4` is the damage source, `2`
the weapon branch as in Pulse's `Ship_Damage`) calls it after the subtraction:
`if (craft+0x5edc->0x138 == 7) { object+0x260 = 1; FUN_00113aa0(...) }` (a
LeachBeam, below) `else if (0 < shield) ShipDamageFx_Update(shield, craft+0x5f70)`,
behind a per-viewport mask (`craft+0x7a3c[i]`, `craft+0x7a24`). `shield` is the
craft's absolute pool (`craft+0x5fa0`, the class maximum is `+0x90` of the same
table), so the thresholds are in pool units. The second caller (`0x000e8de0`,
`Ship_ApplyPendingDamage`) is a network-client path (`craft+0x628c == 1`) that
calls it on a changed value. Event driven: nothing replays it while a craft
flies, and nothing ends it on a recharge. Confidence **72**.

**State.** `shield > 70.0` (`0x008b2fb4`) is **MILD**; `> 40.0` (`0x008b2fb8`)
**MODERATE**; otherwise **CRITICAL**, which spawns `WO_DAMAGE_CRITICAL` **and
then** `WO_DAMAGE_MODERATE` (two `FUN_002c9108` calls, the second after the
sound handle). Each is `FUN_002c9108(obj, name, fourcc, object + 0x1c0, 1, 0)`,
attached, fourccs `IMAD`, `OMAD`, `RCAD` as written in the file. With the Zone
mask (`0x206040`, net flag clear) `WO_DAMAGE_ELECTRIC` at `+0x200` replaces them.

**The hold.** A state write sets `object + 0x240 = 3.3` (`0x008b2fb0`). While it
is positive a hit that finds the same state returns (`0x002a1988`); a different
state releases the old *sound* and writes the new one; once it is zero any hit
rewrites the current state. The three effects are 200-tick one-shots
(`flags 0x0e` / `0x4600020e`, no `LOOPING`): 3.33 s, which is what the hold is
for. **The `+0x1a4..+0x1b0` handles are sound handles** (`FUN_0027c288` on the
sound engine at `+0x468c`), not effect handles, correcting the earlier reading:
the smoke is not released, it finishes. The decrement of `+0x240` was not read.

**Anchor.** `0x002a0268` rebuilds `+0x1c0` each tick: the identity matrix
(`0x00769d70`, world-aligned) with its translation taken from the node's matrix
offset `1.5` (`2.0` with the death flag `+0x2b4`) along one row, sign and
meaning unsettled there. Settled and wired since: see "The smoke's anchor against a frame"
below.

**Wired**, `oag_raceplay::damage_fx`, HD only through `Trigger::DamageMild`,
`DamageModerate` and `DamageCritical`. `--force-shield` plus the new
`--force-hit TICK:SLOT` reaches it headless (a scratch directory, not kept,
`cmp_*.png`: the same tick with and without the smoke). Not wired: the Zone
variant, the death plume (`param_4 != 0`) and the damage sound cue.

### The LeachBeam hit spark: `0x002a06e0`, conflict resolved

`0x002a06e0` is the per-tick engine-flare enqueue (`engine-trail.md`, 84) **and**
it carries a one-off: `if (object + 0x260) { spawn WO_SHIP_SPARK_DAMAGE_LEACHBEAM
attached at +0x1c0 (fourcc 'LBDE'); object + 0x260 = 0 }`. The flag is set by
`Ship_ApplyDamage` on a weapon kind 7 hit, instead of the smoke. The two readings
agreed; there was no conflict. HD throws it **once per hit at one attached
point**, not Pulse's one or two of the hull's locators with a 0.8 s gate, so the
title carries `Trigger::LeachHitSpark` and no `HitSpark`, and
`oag_raceplay::damage_fx::throw_attached_leach_spark` plays it. Confidence 78.
Not seen in a frame (a headless run has no rival whose beam reaches the player);
covered by `a_leachbeam_hit_throws_the_attached_spark_where_the_title_has_no_locators`.

### The Rocket's wall burst: HD's own file, and sprites it was drawn without (2026-10-08)

**What the original draws** (RPCS3, Talon's Junction campaign event 01, craft at rest,
`scripts/rpcs3-hd-weapon.py`, two boots, burst at video frame 199 of 30 fps;
a scratch directory, not kept): a white flash for about 0.1 s, orange sparks to about
0.4 s, then dense dark **teal-grey** smoke bodies that last past 3 s. **The axis is game
time**: the HUD race clock reads 22.9, 23.9, 24.9, 25.9 at video frames 199, 229, 259,
289, so 30 video frames is 1.0 game second here (the 0.6x of earlier hosts did not
apply to these two recordings). Mean RGB of the pixels more than 30 darker than the
pre-burst frame, 1280x720:

| age (s) | 0.23 | 0.43 | 0.63 | 1.03 | 2.03 | 3.03 |
| --- | --- | --- | --- | --- | --- | --- |
| r4 | 133,167,180 | 107,141,154 | 95,131,146 | 85,125,143 | 91,131,151 | 102,141,163 |
| r5 | 129,166,183 | 114,148,162 | 91,129,146 | 84,124,143 | 89,129,150 | 102,142,165 |

The background there is about (180,212,218), so smoke/background is 0.47/0.59/0.66 at
1 s: blue-biased, not brown (the 2026-10-07 reading of "grey-brown" was the orange
sparks bleeding into a small crop). Confidence 80 (two boots, agree within 5 a channel;
one circuit and one pose).

**Which effect it is.** `WO_ROCKET_EXPLO_TRACK` is HD's own authoring: 33,680 bytes,
nine emitters (`WO_ROCKET_EXPLO_TRACK`, `FIREBALL_ADDITIVE`, `FIRESPIKES`,
`FIREBALL_PARENT`, `FIREBALL_ANIMSMOKE_TRAIL`, `SPARKS`, `DEBRIS_NO_TRAILS`,
`FIRE_TRAILS_ONEQUAD`, `TRAIL_SMOKE`), against Pulse's four. The craft-hit
`WO_ROCKET_EXPLO` is 48,256 bytes and 12 emitters. Both are HD files, not Pulse's
played by inheritance, and ours already played them.

**What was missing: the sprites.** Every emitter names a `.tga` at `record + 0x4c4`
(`Z:\WipeoutPSP\HD\Data\Psys\Tex\<stem>.tga`), and `/data/psys/tex/<stem>.gtf`
resolves and decodes for **all nine** track-burst emitters and all 12 of the craft
burst (a census with `Sprite::from_gtf` over each, the stem's case as authored). The
loader read sprites for 2048 (`.gxt`) and Omega (`.gnf`) only, so every HD emitter drew
`psys.wesl`'s procedural radial glow, a model of a Pulse spark, times its palette.
`smoke_new4_8x4` is a 1024x512 **8x4 flipbook**: read top-left first, row 1 is a fire
ball fading to grey smoke, and rows 2 to 4 are yellow, then white-yellow, then white
blobs. `TRAIL_SMOKE`'s palette is black for entries 1-31 of 256 and white after, so the
smoke's colour is mostly the sprite's. Confidence 85.

**What is wired.** `oag_title::Effects::sprites` lists the HD effects whose `.gtf`
sprites play; HD lists `WO_ROCKET_EXPLO_TRACK` alone, Pulse, Pure, 2048 and Omega list
none. `Sprite::from_gtf` is new. Test: `hd_effect_sprites` (nine sprites when listed,
none when not). Cell 0 is the authored top-left, and ours walks the flipbooks as HD does
([particle-render.md](particle-render.md#the-atlas-frame)). Without HD's hold frame
the walk ends on the white cells; this burst's smoke holds on 31.

**Before and after.** Ours at tick 513 onwards (a single square press at tick 420,
`--camera-view close`, per-pixel against a no-fire render; the burst lands on a far
gantry, so about a third of the original's size, so only ratios compare):
smoke/background at 1 s was 0.77/0.74/0.73 (thin pale streaks, gone by 1.5 s) and is now
0.69/0.61/0.58, with the real debris chunks and the sprite's puff. The original's is
0.47/0.59/0.66: ours is still neutral to warm, smaller and shorter-lived. Pulse PSP and
PS2 renders at four ticks across a rocket burst are byte-identical to main.

**Not settled.** (1) The blue bias. HD ships `psys_normal`, `psys_simplegeom` and
`psys_lit` programs (`renderer.md`, the shader groups): strings `psys_normal_vp/_fp` at
`0x007a0f60/70`, `psys_simplegeom_*` at `0x007a0fb0/c8`, `psys_lit_*` at
`0x007a1060/70`, registry slots `0x008b3a34` (normal) and `0x008b3ab8` (lit);
`psys_normal` is also referenced from `FUN_002c7b30` at `0x002c7ea0`, `psys_lit` only
from its table slot, so what selects lit is not found, and nothing says whether any
declares `fogFactors` or a light. Unread; nothing was tinted. (2) The pose: ours hits a far wall. (3) The
craft hit (`WO_ROCKET_EXPLO`) was captured 2026-11-26 and still draws the procedural profile in
ours, which is one reason its picture does not match
([particle-fill.md](../../../rendering/particle-fill.md), "Against the original at a near
blast"). Omega and 2048: **checked, applies,
not wired** - both author their own `WO_ROCKET_EXPLO` (`psys_omega_ground_truth.rs`)
and already read their sprites.

## 2026-11-15: the damage smoke re-read, and a film of a hit

Lane `hd-impact`. Four corrections to "The damage smoke" above, each from the disassembly of
`ShipDamageFx_Update` (`0x002a17e8`) and its anchor helper `0x002a0268`, and one negative.

1. **The handles are particle instances, and a state change kills the old smoke.**
   `object + 0x1ac` and `+0x1b0` take the handle of the effect just spawned (`*0x00ad5fd4`),
   `+0x1a4` and `+0x1a8` the instance `0x0027c288` finds for it on `*(manager) + 0x468c`:
   the lookup [engine-trail.md](engine-trail.md) ("What the float and the velocity do") reads
   as the particle instance, confidence 80. On a hit that changes the state the function calls
   `0x0027f7b0(manager, handle, 1)` for both (`0x002a1994` on), and with a non-zero third
   argument that is `0x00285868(instance)`: it zeroes the live count (`+0x148`), clears flag
   bits `0x11` and frees every particle on the instance's list (`+0x1b8`), then the same for
   its children (`+0x24`) and siblings (`+0x1b0`). So the previous state's smoke, particles
   and all, is gone the moment the next state's starts. "Sound handles ... the smoke is not
   released, it finishes" above is wrong on both counts. Confidence 82.
2. **The smoke carries the craft's velocity over 60.** After each spawn, when the instance is
   found (and outside the Zone mask), `0x002a1c18`..`0x002a1c4c` loads the body's velocity
   (`*(ship + 0x6944) + 0x190`), multiplies by the float at `0x008b2fac` (`1/60`) and hands it
   to `0x00283b40(instance, &v)`, the call the trail-hit effect and the contact spark make
   ([ship-collision-fx.md](ship-collision-fx.md)). Confidence 82.
3. **The anchor is `1.5` along the node's third row, plus.** `0x002a0268` reads the owner's
   node matrix `M = *(object + 0x38)` and builds
   `at = M.row3 - M.row2 * s`, `s = -1.5` (`0xbfc00000`), or `-2.0` once the death flag
   (`object + 0x2b4`) is set (`0x002a0308`..`0x002a033c`: `vmaddfp v0 = row2 * s`,
   `vsubfp v31 = row3 - v0`). So the point is the node's origin **plus** `1.5` (dead: `2.0`)
   of its third row. When `ship + 0x5f43` is set and a global object is absent or has flag
   bits `6` clear, it also subtracts `1.9` (`0x008b2f48`) of the second row
   (`0x002a0340`..`0x002a0378`). `object + 0x1c0` is then the identity with that translation,
   `object + 0x200` the node's own three rows with it. The object is the craft's
   `EngineFlare` (`craft + 0x5f70`) and `*(object + 0x38)` its node, whose row 2 is "the flare
   node's own Z axis" on [engine-trail.md](engine-trail.md): **the smoke leaves the engine
   flare's node, `1.5` along that node's Z**, not the craft's origin. Which way that axis
   points along the hull is the sign engine-trail.md leaves open. Confidence 80 for the
   arithmetic, 75 for the node.
4. **The hold is `3.3` and the thresholds are `70` and `40`**, as read before (`0x008b2fb0`,
   `0x008b2fb4`, `0x008b2fb8`); unchanged.
5. **`~SHIP_ONFIRE` is not this smoke's sound.** Its one reference is `Ship_SetState`
   (`0x000e1e20`, the string at `0x00782528` through TOC slot `0x008a9018`, loaded at
   `0x000e2358`): the state-4 branch (jump table at `0x000e1eec`, case 4 at `0x000e20e8`)
   plays it on the craft's emitter with the handle kept at `ship + 0x7a30`, after
   `0x000d96c8` (the only function that names `~SHIP_DESTROYED`). The state-5 branch
   (`0x000e2088`) is the one that calls `ShipDamageFx_Update` (`0x000e20d8`), the death
   plume's caller. Both belong to the wreck sequence, not to a weapon hit. Read to this
   depth only. Confidence 75.

**This engine** (`oag_raceplay::damage_fx`): kills the old state's smoke (1), carries the craft's
velocity (2) and anchors at the flare node (3); all three wired.

### A film of a hit

RPCS3 0.0.43-20574, `hd-racebox-talons-venom-weapons-grid`, `scripts/rpcs3-hd-missile-film.py
--ages 0.05,0.1,0.2,0.33,0.5,0.75,1.0,1.33,1.7,2.0,2.5,3.0`: the player's Missile into the
rival 12 units ahead, single game frames from the hit. One run:

| Age | What the frame shows |
| --- | --- |
| 0.05 to 0.2 s | the Missile's blast over the rival, the screen washed out by 0.2 s |
| 0.33 s | the rival clear of the blast, pale embers hanging in the air where it was |
| 0.5 to 0.75 s | the rival flying on with small bright sparks over its hull, no dark smoke |
| 1.0 to 2.0 s | the player has caught the slowed rival and the hulls touch: an orange spark shower beside them, white streaks on the player's wing at 1.33 s |
| 2.5, 3.0 s | the rival out of frame |

The run does not say which effect each spark is: a rival's first Missile hit leaves it above
70 shield (`WO_DAMAGE_MILD`), and from 1.0 s the contact's own spark overlaps it.
`scripts/hd-missile-film-pair.py` was run on the film (ages 0.33 to 1.0 s) and is no pair for
this question: it puts ours' camera where the film's was relative to the **blast**, in ours'
own scenario at the start line, so the struck craft is a few pixels in the distance. A pair
for the hit effect needs the struck craft's pose matched, not the blast's place.

### The smoke's anchor against a frame

Lane `hd-smoke-anchor`. Wired: `damage_fx::anchor` is the `Engine Flare` locator plus `1.5`
(`2.0` once the craft has no shield) of the locator's Z axis, carried through the pose the hull is
drawn at, for the damage smoke, its per-tick follow and the LeachBeam spark that shares `+0x1c0`.
Tests: `the_anchor_is_the_flare_nodes_origin_plus_one_and_a_half_of_its_z_row`,
`the_smoke_leaves_the_flare_node_and_follows_it` (red with the craft's origin).

**The sign.** Every HD hull's locator reads Z axis `(0, 0, 1)` with the nozzle at negative Z
(Feisar `-4.93`, Qirex `-7.35`, Piranha `-8.03`; load report, `Locators.vex`), so `+Z` is toward the
nose and the smoke starts `1.5` model units (`1.125` world, the hull's `0.75`) **toward the nose
from the nozzle, inside the engine bay**. It is the mirror of the engine light's
`node - Z * Distance` ("behind the nozzle", renderer.md, confidence 85; that law's node matches
ours on Assegai, Assegai_c1 and Harimau). Confidence 70: the arithmetic is read (80, above); the
direction rests on that node convention.

**Against the original.** `hit0` of `data/reference/hd-capture/missile-hit/` (the struck rival,
`craft[6]` of each frame's json, projected with the frame's own `view_proj`, row vector times
matrix): at 0.60 s and 0.67 s the engine glow, the node, is at about (483, 388) and (475, 378) in
1278x718, at the end of the hull the projected `+Z` points away from, and the first sparks sit
within about 15 px of it (the rival's team is not recorded, so the nozzle Z spans `-4.9` to `-8.0`:
the two signs project 15 px apart for one team and the nozzle's own spread is 30 px). **The film
cannot separate the two signs**, and it shows neither the other way: its hit leaves the rival
above 70 shield (`WO_DAMAGE_MILD`, small sparks, no dark smoke). Ours, `--force-shield 0:30
--force-hit 275:0`, tick 300, 1280x720: the plume rises from the rear of the hull about 100 px
above the nozzles, from the engine housing toward the nose. A frame of a CRITICAL rival (shield
poked under 40 before the hit) would settle the sign to a pixel and is not taken.

**Not wired: the `1.9` of the second row.** `0x002a0340..0x002a0378` subtracts `1.9` of the node's
Y row when `ship + 0x5f43` is set and a global object is absent or has its flag bits `6` clear.
`+0x5f43` is read elsewhere only as the gate of `MagstripWake_UpdateRibbons`'s pool call
(magstrip-wake.md), and no page names the global object, so which view flag it is stays open.
The brief's own rule: leave it open with the address.

**The death flag.** `object + 0x2b4` selects `2.0`; its writer is unread. A craft with no shield
left stands for it, **chosen, not measured**, and the follow is the only consumer (a smoke is not
started on a spent shield, the original's rule).

**Lineage: 2048 and Omega, checked, applies, not wired.** Both ship the same three
`WO_DAMAGE_*` effects and an `EngineFlare` with the same locator convention
([engine-trail.md](engine-trail.md)), so the anchor holds in shape; the callers' twin of
`0x002a0268` was not located (no `-1.5` constant read near either caller), and the trigger is
HD-gated (`Trigger::DamageMild`).

## 2026-11-25: the camera-mode flags, a second film, and a CRITICAL attempt

Lane `hd-contact-sparks`. Three results: what `ship + 0x5f42` and `+0x5f43` are (they gate the
smoke's `1.9` term and every collision spark), a Missile film with the hulls apart, and a
CRITICAL rival that could not be framed.

### `+0x5f42` and `+0x5f43` are the player's view flags

`PlayerStatus_Update` (`0x000cb928`) and `PlayerStatus_ApplyCameraOption` (`0x000cae68`) write
both bytes of the player's craft (`*(this + 0x44)`) from the view the player's camera is in
(`*(this + 0x40)`, the camera mode):

| `this + 0x40` | The view (tripod string loaded at the call) | `+0x5f42` | `+0x5f43` | Where |
| --- | --- | --- | --- | --- |
| 0 | `local player%d internal tripod` (`0x00781a10`), picked when the player's `CameraP%i` record is `OPT_INT` (`0x007819f0`) | 1 | 0 | `0x000cb1b0`..`0x000cb1c8` |
| 1 | `local player%d external close tripod` (`0x00781a30`), `OPT_CLOSE` | 0 | 0 | `0x000cb058` |
| 2 | `...external far tripod` (`0x00781a58`), `OPT_FAR` | 0 | 0 | `0x000cb058` |
| 3 | `local player%d backward tripod` (`0x00781b20`), the look-back, entered while the pad record's `+0x69` byte is set and the object `*0x009878c4` (TOC `-0x4b50`, beside the camera object `*0x009878c0`) has `+0x34` bit 1 clear (not read further) | 1 | 1 | `0x000cba2c`, `0x000cba30`, `0x000ccd48`; the hull's visible bits `+0x34 & 7` cleared |

The generic write is `0x000cc778`/`0x000cc784`: `+0x5f42 = (mode == 0)`, `+0x5f43 = (mode == 3)`.
Camera classes' own setters (`0x0003dc7c` and the others in the byte-pair list) clear both.
**Live, RPCS3 `hd-racebox-talons-venom-weapons-grid`, `scripts/rpcs3-hd-camera-flags.py --hold l3,r3,l1,r1`:**
all eight craft read `00 00` at `+0x5f42..43` in the chase view; holding **R1** read `01 01` on the
player's craft only, and `00 00` again on release; L3, R3 and L1 changed nothing. So **`+0x5f43` is
the look-back view, set on the player's craft alone, and `+0x5f42` is set in the internal view
and in the look-back** (the internal view itself was not selected live: its half is the static
table above). Confidence 90 for `+0x5f43`, 85 for `+0x5f42`.

What follows from it:

- **Row 8's `1.9` term** (`0x002a0340`..`0x002a0378`): `+0x5f43` set and the camera object
  `*0x009878c0` absent or its `+0x34 & 6` clear (the destroy camera sets `6`, ship-destroy.md,
  row 5). So the smoke anchor moves by `1.9` of the node's Y row **only for the player's own craft
  while the player looks back**; a rival's `+0x5f43` is never set, so its smoke is the flare node
  plus `1.5` of its Z row, which is what ours draws. Confidence 85.
- **The cockpit-view flag of the collision sparks** (`ShipCollisionFx_Trigger`'s first gate,
  `*(locator + 0xe4) + 0x5f42`): the gate is the internal view or the look-back, on the player's
  craft only; a rival's is always 0. Confidence 85.
- **This engine draws neither view** (no internal camera, no look-back), so `+0x5f42` and
  `+0x5f43` are always 0 for it: nothing to wire, and neither the `1.9` term nor the spark gate
  changes a frame. A look-back or internal camera added later has to carry both.

### A second Missile film: the hulls never touch

`scripts/rpcs3-hd-missile-film.py --ages 0.5,0.6,0.67,0.75,0.9,1.0,1.17,1.33,1.5,1.7,2.0`
(RPCS3 0.0.43-20574, the same state, the player held 12 units behind the rival, 1.0 to 2.0 s
photographed). The player-to-rival distance from the film's own craft poses, per frame:
`12.0, 13.0, 12.6, 12.0, 11.7, 12.1, 13.3, 14.6, 14.9, 17.5, 24.3, 33.7` at ages
`0, 0.5, 0.65, 0.7, 0.8, 0.91, 1.02, 1.19, 1.35, 1.51, 1.72, 2.02`. **The hulls never come
within 11 units of each other**, and POS going 8 to 7 at 1.0 s is a different pass. The "hulls
touch" window in the first film's table (above) was not reproduced; the orange pieces beside the
rival at 1.0 s are the rival's damage and the red tiles on its hull the shield-hit flash, not the
contact spark (row 1 has no frame yet). To photograph a contact the player must be held at a hull
width or less (`--back 3`), which this lane did not run.

### A posed pair for the hit, built and scored, not adopted

`scripts/hd-hit-posed-pair.py --film DIR --team T` renders ours on the film's frames: craft 0 is
posed on the struck rival (`--pose-from` a one-row trace with the film's rows and position, held at
rest every tick by `--pose-hold-ticks`), the camera is the film's pushbuffer camera (the trace's
camera columns carry the world axes as columns, `oag_trace::replay::camera_orientation_of`),
`--force-shield` puts the rival at 89 % and `--force-hit` lands the hit, and the run goes on to
the frame's pool age. The track, the camera and the red pad tiles under the rival line up to the
pixel; the box score is over 360x280 centred on the rival's projection. Frames with a pushbuffer
camera only (`a0500`, `a0600`, `a0670`, `a1000`, `a1500`, `a2000`; a frame with none is skipped,
never borrowed). Team Piranha, a guess (the film does not record the rival's team; its hull is red
and white):

| Window | Frame (age) | Mean abs. difference over the box | Bright pixels, original / ours |
| --- | --- | --- | --- |
| 0.5 to 0.75 s | `a0500` (0.50) | 40.9 | 3092 / 764 |
| | `a0600` (0.65) | 42.6 | 1202 / 264 |
| | `a0670` (0.70) | 23.2 | 605 / 18 |
| 1.0 to 2.0 s | `a1000` (1.02) | 17.0 | 299 / 881 (ours: the hull's gloss) |
| | `a1500`, `a2000` | not scored: the rival is out of the frame (box off the picture) | - |

**It does not score honestly, so it is not adopted as a compare pair.** Three things the number
does not separate: (1) the rival's team is unknown (ours' hull is the wrong livery); (2) ours
draws the hull a little above and short of the original's (the film's pose is the body's, the
frame draws the hull a tick or two behind it, `docs/tools/compare-page.md`, "The skew is the
pause"), which the pair's own tick-lag refusal would catch and this script does not apply; (3)
ours draws **no visible hit sparks or smoke** at the matched age while the original shows orange
embers around the hull at 0.5 to 1.0 s (the bright-pixel counts), but ours' hull is held at rest
every tick, so the smoke's carried velocity (row 7) and the sparks' drift are not exercised: the
absence is a property of the pose hold as much as of the effect. Neither window therefore
judges rows 1, 6 and 7. The 1.0 to 2.0 s window has one valid frame, because the hulls never
touch (above). Next: a team read (RAM, the rival's `+0x7d2c` variant and team words), the tick-lag
fix, a rival that flies (a second ship in ours placed at the film's pose and velocity with the
field warm-up, `--field-from --field-warmup`, so its smoke carries).

### A CRITICAL rival could not be framed

`--rival-shield S` (new) writes the rival's shield (`ship + 0x5fa0`) before the shot. The
Missile took a rival poked to 75 down to 59.4 (MODERATE, 15.6 damage), but a rival poked to 30,
50 and 54 reads `-1.0` from the first frame after the hit and draws the broken hull: the
Missile destroys it. The anchor's sign (`+1.5` or `-1.5` of the node's Z row) stays at the
confidence 70 the "smoke's anchor against a frame" section gave it; what makes a hit lethal
below roughly 55 (against 15.6 above) is unread. Next: poke the shield **after** the hit to 35
and let `0x002a17e8` run through a second weapon, or hit with a weaker weapon.

## Names applied

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x000ddd58` | `Ship_Construct` | 80 | vtable, state zero-fill, skin/absorb-shell/engine-light/node loads, the attached wall spark |
| `0x000dfd90` | `Ship_ConstructAlt` | 65 | the same body with a local copy of `param_6` and a different skin search; it is the twin, not separately identified |
| `0x002a17e8` | `ShipDamageFx_Update` | 72 | the `WO_DAMAGE_*` state switch, thresholds 70 and 40, and its two callers above |
| `0x000cae68` | `PlayerStatus_ApplyCameraOption` | 72 | picks the camera mode from the player's `CameraP%i` record (`OPT_INT`, `OPT_CLOSE`, `OPT_FAR`), loads the tripod string and writes `ship + 0x5f42` (1 in the internal view) |
| `0x000e7760` | `Ship_ApplyDamage` | 65 | the weapon-source damage function that calls it, kind 7 sets the LeachBeam flag; `_q` |
| `0x000e8de0` | `Ship_ApplyPendingDamage` | 65 | applies the pending amount and calls `0x000e7760`; the network branch calls the smoke itself; `_q` |

## Zone-mask variants

`0x206040` is modes 6, 13, 14, 21 (`zone-sky.md`). Three effect pairs depend on it:
the constructor's `_DAMAGE_ZONE`, `ShipCollisionFx_Trigger` kind 2's `_NODAMAGE_ZONE`
and `0x002a17e8`'s `WO_DAMAGE_ELECTRIC`. None is wired: swapping them needs this
port's mode to be mapped to those ids, and `zone-sky.md` records that the order of
6 and 13 is not established. Left as the existing (non-Zone) names, which is what an
HD Zone race already plays.

## Omega, 2048, and the other titles

- **Omega:** `ps4-omega-eu/ship-collision-fx.md` reads the same `kind` switch, with
  `WO_SHIP_SPARK_DAMAGE_WEAPON` at `kind == 1`. **Checked, applies, not wired.**
  Omega's disc ships it (re-read 2026-10-13 on all nine archives, 46,005 digest-matched
  entries): `Data/particles/WO_SHIP_SPARK_DAMAGE_WEAPON.pob` (12,560 bytes) and a
  `Data/particles2048/` copy (9,216 bytes), both in `data00.psarc`, among 422 `.pob`
  entries. The earlier "no `.pob` rows" was a listing failure, not a result. The
  asset side is therefore present; the trigger stays unwired.
- **2048:** the Vita executable carries the string too (`strings -a` on
  `PCSF00007/base/eboot.elf` and `patch-v104/eboot.elf`: 2 hits each, with
  `WO_SHIP_COLL_SPARK_DAMAGE` and `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` beside it).
  **Checked, applies, not wired**; 2048's own trigger was not read.
- **The damage smoke and the LeachBeam spark (2026-10-07):** `strings -a` on the
  2048 patch executable (`PCSF00007/patch-v104/eboot.elf`) and on both Omega
  executables finds `WO_DAMAGE_MILD`, `_MODERATE`, `_CRITICAL`, `_ELECTRIC`,
  `WO_SHIP_DEATH_DAMAGE_PLUME` and `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` (Omega's patch
  build carries `WO_DAMAGE_MILD` only as the `.POB` file name). **Checked, applies,
  not wired**: the same names are authored, their triggers in those executables
  are unread, and a string is not a trigger.
- Both stay off by construction: `hit_sparks::throws_weapon_spark` is true for HD
  only, `only_hd_throws_the_cannon_craft_hit_spark` asserts it false for Pulse, Pure,
  2048 and Omega (mutation-checked), and a title with no anchors fires nothing.
- Pulse's `Ship_Damage` spark and HD's Cannon spark are two mechanisms for one
  outcome; Pulse keeps its own.

## Open

- Whether the wall contact's attached effect is toggled by `+0x5f42` (the caller of
  kind 2, `WO_SHIP_COLL_SPARK_NODAMAGE`, is now read: `Ship_SpawnContactFx` sets
  kind 2 at `0x000e7730` on frictionless records, conf 85;
  [collision-cues.md](collision-cues.md), "2026-11-23: which records carry no friction,
  and the spark they throw").
- The three `WO_LEACHBEAM_LAUNCH/HIT_TARGET/BREAK` spawners have no code xref; the
  dispatcher behind OPD run `0x00875a40..` is unfound.
- `0x0010c958` is `WO_DEBRIS_FIRE`'s one load, unread. The decrement site of the
  damage hold (`object + 0x240`), the sign of the `1.5` anchor offset, the Zone
  `WO_DAMAGE_ELECTRIC` branch and the death plume (`param_4 != 0`) are unread.
- The wreck trio is still Pulse-gated on HD: `WO_SHIP_FXNODE_EXPLO` and
  `WO_SHIP_DEATH_SPARKS` are on the disc but not strings in the executable, so no
  HD trigger is read for them; they stay unwired rather than ungated on a guess.
- The Zone mode map, which unlocks all three Zone variants at once.

## 2026-11-17: items 1 and 2 of the damage smoke re-read are wired

A state change kills the slot's previous state smoke (`psys::Stage::kill`) and every damage smoke
carries the craft's velocity (`psys::Stage::carry`), in `crates/raceplay/src/damage_fx.rs`; tests
`a_state_change_kills_the_previous_smoke_with_its_particles` and
`the_damage_smoke_carries_the_crafts_velocity`. Velocity is passed in units a second like the
trail-hit burst (the original's 1/60 is its per-tick unit). 2048 and Omega: checked, applies,
not wired (they play no damage smoke yet). Item 3 (the anchor) is still open.
