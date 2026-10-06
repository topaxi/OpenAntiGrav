# The Repulser: a blast at the firer, then two shockwaves that run along the track

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** **read 2026-10-04** (pulse-repulser lane), decompile plus
instruction-level checks of every load-bearing constant. **Partly runtime-verified
the same day** (pulse-repulser-2): one live Repulser on PPSSPP, sampled at every
`Repulser_Update` call - see [the live read](#2026-10-04-live-on-ppsspp---the-timeline-the-field-model-and-the-travel-emitter).
Rows the capture confirmed are lifted to 88-90; the rest stay at or under 84.

**It is not "a field the craft is in".** The earlier note on
[shuriken.md](shuriken.md#a-neighbour-read-on-the-way-the-repulser) read
`Weapon_FireRepulser`'s four copies onto the firing craft's record as the weapon's
state. They are written and **never read** (see
[the dead copies](#the-four-copies-onto-the-firers-record-are-never-read)). The
weapon is a pool entity that lives `blast_time + wave_time` seconds: a visual
blast around the firer for `blast_time`, then two waves that walk the AI track's
own control points, one forward and one backward, hitting every other craft they
sweep over exactly once and setting off any laid Mine or Bomb in their path.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0880d58c` | `WeaponStats_ParseRepulser` | 90 |
| `0x0886ce8c` | `Weapon_FireRepulser` | 88 |
| `0x0886c888` | `RepulserPool_Construct` | 80 |
| `0x0886cc70` | `RepulserPool_Update` | 82 |
| `0x0886d5c8` | `RepulserPool_SweepTargets` | 82 |
| `0x0886cfc8` | `Repulser_WaveSweepsPoint` | 80 |
| `0x0886d254` | `Repulser_HitCraft` | 84 |
| `0x0886d4f4` | `Repulser_SpawnRemote` | 60 |
| `0x08875008` | `Repulser_Construct` | 82 |
| `0x08875210` | `Repulser_Init` | 90 |
| `0x08875400` | `Repulser_Update` | 90 |
| `0x08875658` | `Repulser_Reset` | 80 |
| `0x08875864` | `Repulser_GetWavePoints` | 80 |
| `0x088758b0` | `Repulser_SetHitLatch` | 80 |
| `0x088758c0` | `Repulser_HitLatch` | 80 |
| `0x088758cc` | `Repulser_UpdateFieldModel` | 88 |
| `0x088761d8` | `Repulser_SpawnBlastEffect` | 84 |
| `0x08876300` | `Repulser_SpawnWaves` | 80 |
| `0x08876634` | `Repulser_ForkAtJunction` | 72 |
| `0x08876914` | `Repulser_AdvanceWave` | 88 |
| `0x0887e174` | `AiTrack_StepForward` | 85 |
| `0x088765e4` | `Repulser_CursorPastHalfPath` | 80 |
| `0x0887d37c` | `AiTrack_PathIndex` | 90 |
| `0x0887d4d4` | `AiTrack_LocateOnSiblingPath` | 65 |
| `0x0887d970` | `AiTrack_PathListsNeighbour` | 60 |
| `0x0887e2f0` | `AiTrack_StepBackward` | 80 |

`Repulser_SpawnWaves` was `Repulser_SpawnWaves_q` at 62 on
[screen-flash-callers.md](screen-flash-callers.md), held down because its caller
was unread. Its caller is `Repulser_Update`, read below, so it rises to 80.

## The `<Stats>` block, from the parser

`WeaponStats_ParseRepulser` (`0x0880d58c`) stores, case-insensitively (the reader
is `strcasecmp`, [xml-reader.md](xml-reader.md)):

| Offset | Attribute | String | Eliminator (both PSP regions) | Race file |
| --- | --- | --- | --- | --- |
| `+0x128` | `damage` | `0x08a78a34` | 30 | 20 |
| `+0x12c` | `blastRadius` | `0x08a78b80` | 40 | 40 |
| `+0x130` | `blastForce` | `0x08a78b8c` | 40 | 40 |
| `+0x134` | `slowdown_time` | `0x08a78a3c` | 0.8 | 0.8 |
| `+0x138` | `absorb` | `0x08a78aac` | 15 | 20 |
| `+0x13c` | `blast_time` | `0x08a78b98` | 0.8 | 0.8 |
| `+0x140` | `wave_time` | `0x08a78ba4` | 0.8 | 0.8 |

Confidence 90: seven direct stores, one per attribute. **This corrects the order
[weapon-stats.md](../../../formats/weapon-stats.md) and the shuriken.md note
implied**: `+0x128` is `damage`, not `blastforce`. The values are read off
`Data\XML\WeaponStats_Elimination.xml` and `WeaponStats_Race.xml` with
`oag-wad cat --expand`; the race file's odds for the weapon are zero, so only the
Eliminator column is reachable.

**`blastRadius` is spent nowhere.** A whole-program `lwc1 ..., 0x12c(...)` sweep
finds ten hits; every one is on a different object (`Ship_AcquireLock` and four
`*_HitCraft` functions read the struck record's own `+0x12c` hit counter, the
rest are unrelated subsystems). Control: the same sweep at `+0x134`
(`slowdown_time`) finds the three Repulser readers this page names. Same shape as
the Bomb's `damageradius`. Confidence 75 for "unspent": a pointer-plus-`addiu`
load would not match the pattern.

## `Weapon_FireRepulser` (`0x0886ce8c`) claims a slot and does nothing else that lasts

```c
void Weapon_FireRepulser(RepulserPool *pool, WeaponRecord *rec, int craft_index) {
    pool->flags |= 2;
    rec->held = -1;                    // +0x1bc
    rec->fire &= ~0x10000;             // +0x1b8
    if (pool->live < 16) {             // +0xa8, slots at +0x68
        Repulser *r = pool->slot[pool->live];
        Stats *s = active_weapon_stats();
        rec->0x170 = s->damage;        // +0x128
        rec->0x17c = s->blast_time;    // +0x13c, overwritten on the next line
        rec->0x17c = s->wave_time;     // +0x140
        rec->0x178 = s->slowdown_time; // +0x134
        r->flags = 1; r->owner = craft_index; r->id = ++g_next_projectile_id;
        Repulser_Init(r, craft_index, rec->node /*+0xf0*/, rec->node->emitter /*+0x50*/);
        if (g_game_mode > 13) broadcast(...);     // network modes only
        pool->live += 1;
    }
}
```

The double store to `+0x17c` is real: `0x0886cf0c` and `0x0886cf14` are both
`swc1 f12, 0x17c(a1)`. Sixteen live Repulsers at most, the same cap as the
Shuriken's pool. Fire bit `0x10000`. Confidence 88.

### The four copies onto the firer's record are never read

`rec` here is the per-craft weapon record (`WEAPON_SYSTEM %d`, the object
`craft+0x4c` points at, the same one `+0x110` impulse and `+0x120` damage live
on). The negative, measured:

- `lwc1`/`lw` with `0x170(`, `0x178(`, `0x17c(` across the program, excluding
  `sp`: every hit is another struct (the `Ai` record in `Ai_Construct`/`Ai_Update`,
  the HUD in `Hud_UpdateEnergyBar`, the stats object `DAT_08b31774` in
  `Ship_Damage`/`Ship_AddShield`, cloud groups, the movie player).
- VFPU loads in this module are spelled `addiu rX, base, off` then
  `lv.q C, 0x0(rX)` (`Repulser_HitCraft` reaches `+0x110` that way at
  `0x0886d304`). An `addiu ..., 0x170`/`0x178`/`0x17c` sweep finds only craft
  entity, Missile, LeachBeam, particle and movie-player functions, and the one in
  `Weapons_DispatchFire` (`0x088618bc`) walks a `0x1f0`-stride table in the
  weapons manager, not the record.
- `Repulser_HitCraft` takes damage, slowdown and force straight from the stats
  block, not from these copies.

So the copies are dead stores. Confidence 70 that nothing reads them: a
`memcpy`-style block copy of the record would not show in either sweep.

## `Repulser_Init` (`0x08875210`): the sound, the blast effect, the field model

- Field-model easing state: scale `+0x204` from `1.0` toward `0.6` at `0.1` per
  1/60 step, then `+0x1f8` from `0.7` toward `4.0` at `0.05`; alpha `+0x210`
  from `0` toward `1.0` at `0.2`; spin `+0x21c` from `0` toward `-2*pi`
  (`0xc0c90fdb`) at `0.02`, which only starts once the entity is `0.4` s old.
- `Repulser_SpawnBlastEffect` (`0x088761d8`): `WO_REPULSER_BLAST`
  (`0x08a7cd18`), fourcc `0x33504552` (`REP3`), anchored at the entity's own
  matrix `+0x1a0`, which is first copied from the firer's node.
- `Sound_Play(1.0, firer_emitter, ..., "REPULSOR", 0)` (pointer cell
  `0x08a7ccbc`): `weapons.bnk` cue 33, five waveforms.
- Then a **Repulser-owned emitter** (`FUN_08946ce4(0x70)` + `SoundEmitter_Init`)
  at `+0x48`, its position pointer at the entity's `+0x150`, its radius `+0x38`
  set to `0x44160000` = **600.0**, playing `~REPULSORTRAVEL` (cue 34) with the
  handle kept at `+0x4c`. Released by `RepulserPool_Update` when the entity dies.

Confidence 84.

`Repulser_Construct` (`0x08875008`) loads `Data\Weapons\pulse_repulsorwave.vex`
(format string `0x08a7ccdc`, the entry exists in `Data.wad`) into `+0x1e4` - the
field model the easing above scales and fades - and looks up `"AI track data"`
into `+0x228`. The model is a flat ring, radius `10.17` at scale 1, one mesh of
262 triangles on `noise2_ADD_GLOW` (`oag-view --mesh --draws`; material flags
`0x0292`, pass mask `0x12b2`, the Bomb shockwave's `0x0212`/`0x1232` plus bit
`0x80`, the per-batch alpha-test selector).

### The field model's matrix, to the instruction (2026-10-04, pulse-repulser-2)

`Repulser_UpdateFieldModel` (`0x088758cc`, listing in the lane's scratch), with
`P` the AI-track point under the **firer's own cursor** (`craft+0xad8..+0xae4`,
`point_ptr` at `+0xae4`) and `T` the firer node's world translation (`+0x30` of
its matrix):

```c
left   = P.pos - P.lateral * P.half_left;          // +0x00, +0x30, +0x44
right  = P.pos + P.lateral * P.half_right;         // +0x48
across = normalize(right - left);
up     = -P.row2;                                  // vneg.q of +0x20
fwd    = normalize(across x up);                   // vcrsp.t
up'    = normalize(up - fwd * dot(fwd, up));
side   = up' x fwd;
s      = (+0x204 > 0.61) ? ease(+0x204) : ease(+0x1f8);   // pre-step test
Node_SetLocalMatrix(model, [side*s, up'*s, fwd*s, T], 0);  // 0x08875f78, a1 = sp+0x60
Image_SetVertexColours(model, (int)(+0x210 * 255) << 24 | 0xffffff); // 0x088760f4
if (age > 0.4) ease(+0x21c);                                 // 0x088760fc
Math_RotateByAxisAngle(+0x21c, &[side, up', fwd, T], axis = &up');   // 0x08876168, a0 = sp+0x130, a1 = sp+0x140
entity+0x1a0..+0x1dc = that rotated, unscaled matrix;         // 0x08876170-0x0887619c
```

**The model does not spin.** `Node_SetLocalMatrix` with `0` puts the node in
mode `0x1000000`, which `Vex_UpdateNodeWorldMatrix` (`0x08944544`) composes with
the parent's world matrix (`vmmul.q`); the parent is the Repulser entity
(`Repulser_Construct`'s `Node_AttachChild`), a bare `Object_ConstructBase`
object whose `+0x2c` is `0x3006` - mode `0`, inherit the parent's - all the way
up. So the model's world matrix is the scaled basis. The spin turns only the
copy stored at `+0x1a0`, which is the matrix `WO_REPULSER_BLAST` is anchored to
(`Repulser_SpawnBlastEffect`). The spin's sense against glam's
`Quat::from_axis_angle` is not settled: the blast's ring angles are random as
played here, so it does not show. **The model is visible for the whole
lifetime**: `Repulser_Update` raises its bits `4|2` every call, and nothing
clears them before the slot dies. The alpha reaches the GE the way the Bomb
shockwave's does (`bomb_blast.rs`), white with that alpha. Confidence 88 (the
live read below confirms every ease value).

Whether it draws over the blast effect is not separately settled. Both are
additive (`_ADD_GLOW`, the psys's own blend), so the order does not change the
picture.

### The field model's texture plays on the Repulser's age (2026-10-05, fx-age-clocks)

`Repulser_Init` calls `Node_SetAnimTimeTree(0.0)` on the field model at `0x08875324` (`f12 = f22 = 0`) and
nothing calls it again, so the model's texture time is the entity's age at rate 1. Measured live on two
boots with a conditional breakpoint on `Mesh_UpdateTextureTransforms`: the one fresh mesh read `+0x40 =
0.000` on the fire frame (clocks 341.338 and 168.616), slope `1.0000` against `g_ingame->0x40` over 95
samples, and stopped at 1.569 s, the entity's `blast_time + wave_time`. Confidence 88. See
[`anim-transform.md`](anim-transform.md#a-meshs-texture-time-seeded-per-spawn-so-object-age-2026-10-05).

## `Repulser_Update` (`0x08875400`): two phases and a lifetime

```c
bool Repulser_Update(float dt, Repulser *r) {
    r->age += dt;                                          // +0x1ec
    ease(alpha);  Repulser_UpdateFieldModel(dt, r);
    Stats *s = active_weapon_stats();
    if (r->state == 1) {                                   // +0x50, waves running
        prev[0..2] = current[0..2];                        // +0x120/+0x130/+0x140 <- +0x90/+0xd0/+0x110
        Repulser_AdvanceWave(r, &r->cursor0 /*+0x22c*/, &r->wave0 /*+0x60*/, .., dir 0, .., 5, 0);
        Repulser_AdvanceWave(r, &r->cursor1 /*+0x23c*/, &r->wave1 /*+0xa0*/, .., dir 1, .., 2, 0);
    } else if (s->blast_time < r->age) {
        Repulser_SpawnWaves(r);                            // cursors <- firer's (+0xad8), WO_REPULSER x2, screen flash 2
        Repulser_AdvanceWave(.. dir 0, 5, init=1);  Repulser_AdvanceWave(.. dir 1, 2, init=1);
        prev[0..2] = current[0..2];
        r->state = 1;  alpha target 0 at 0.1;
    }
    return r->age < s->blast_time + s->wave_time;          // false -> destroyed
}
```

**The step counts are immediates, not decompiler guesses**: `0x08875518 li t2,0x5`
with `t0 = 0` (forward) and `0x0887553c li t2,0x2` with `t0 = 1` (backward), and
the same pair at `0x0887558c`/`0x088755b0` on the spawn tick. There is no `dt`
in the advance: each wave moves **5 control points forward and 2 backward per
update call**, which at the authored 1/60 s step
([frame-pacing.md](../../../psp/frame-pacing.md)) is 300 and 120 points a
second. Confidence 84.

`AiTrack_StepForward` (`0x0887e174`) and `AiTrack_StepBackward` (`0x0887e2f0`)
move a `{track, path, point, point_ptr}` cursor that many points up or down the
`0x70`-stride point array, crossing to the next path at a path's end. Return
`2` stops at a junction whose `+4` is set unless the caller passes mode `1`;
return `1` is a fork (`next_alternate` present). Confidence 80.

`Repulser_AdvanceWave` (`0x08876914`) puts the wave's matrix translation at the
**midpoint of the track's two edges** at the cursor's point: `pos - lateral *
half_width_left` and `pos + lateral * half_width_right` (`point+0x44`/`+0x48`,
the half-widths, not the corridor), averaged with `0x3f000000` (`0.5`). On a fork
(return `1`, with the branch slot `+0x25c` still free) it spawns a third
`WO_REPULSER` (`REP2`) on the alternate path and drives it as wave 2 from then
on (`Repulser_ForkAtJunction`, `0x08876634`, the init-tick variant, 60).

`Repulser_SpawnWaves` (`0x08876300`): two `WO_REPULSER` (`0x08a7cd2c`), fourccs
`REP0` (`0x30504552`) anchored at wave 0's matrix `+0x60` and `REP1`
(`0x31504552`) at wave 1's `+0xa0`; both cursors set to the firer's own AI-track
cursor (`craft+0xad8`); then `FUN_088f00c0(.., 2, firer_pos)`, the screen flash
kind 2 ([screen-flash-callers.md](screen-flash-callers.md)). Confidence 80.

## `RepulserPool_SweepTargets` (`0x0886d5c8`): who a wave hits

Called by `RepulserPool_Update` (`0x0886cc70`) right after `Repulser_Update`,
every tick, in both phases. Per other craft (`i != owner`, latch
`Repulser_HitLatch(r, i)` clear):

1. Locate the craft (`+0x90`) on the AI track with radius `500.0` into a record at
   `sp+0x60`; take `width = rec+0x50 - rec+0x4c` - the **AI corridor's** right
   minus left edge, read at instruction level (`0x0886d6e8 lwc1 f22,0xb0(sp)`,
   `0x0886d6ec lwc1 f12,0xac(sp)`, `sub.s`).
2. `Repulser_WaveSweepsPoint(width, A = wave0 now, B = wave0 last tick, P)`, else
   the same for wave 1; on a hit, `Repulser_HitCraft` with that wave's `A`.
3. If the fork wave is live and neither hit, the same for wave 2.

`Repulser_WaveSweepsPoint` (`0x0886cfc8`), confidence 80:

```c
n = normalize(A - B);                        // travel direction this tick
f = dot(n, P - A);  b = dot(n, P - B);
if (f <= 1.0 && b >= -1.0 && f != 0.0 && b != 0.0) {
    u = normalize(n x (n x (P - A)));        // perpendicular to the travel axis
    d = dot(u, P - A);
    return -width < d && d < width;
}
return 0;
```

So a craft is hit when the wave's centre passes it this tick (within one unit
either end) and it is within `width` of the travel axis. The `!= 0.0` terms are
what keep the blast phase harmless: before `Repulser_SpawnWaves`, `A` and `B` are
the same reset point and both dots are zero.

Then the **Mine pool** (`pool+0xb0`, count `+0x164`, slots `+0x64`) and the
**Bomb pool** (`pool+0xb4`, count `+0xc4`, slots `+0x44`), each entity located
with radius `100.0` and tested against waves 0 and 1 only. A swept Mine gets
destroy bit `4` on `+0x3c` (`MinePool_Update` then blows it up,
[mine.md](mine.md)); a swept Bomb gets bit `4` and loses bit `1`
(`BombPool_Update` detonates it). The pool shapes are the ones
[plasma.md](plasma.md) and [mine.md](mine.md) already identified. Rockets,
Missiles, Plasma bolts and Shuriken are **not** in the sweep: the Repulser does
not deflect or destroy projectiles. Confidence 82.

## `Repulser_HitCraft` (`0x0886d254`): full force toward the wave, full damage

Read at instruction level, confidence 84:

```c
WeaponRecord *v = pool->record[i];                          // pool+0x48
Vec3 dir = normalize(v->hit_anchor /*+0x50*/ - A);         // vsub.q at 0x0886d2a0
v->impulse /*+0x110*/ += dir * -s->blastForce;             // lwc1 0x130 ; neg.s ; vscl.q ; vadd.q
v->pending_damage   /*+0x120*/ += s->damage;               // +0x128
v->hits_taken       /*+0x12c*/ += 1.0;
v->pending_slowdown /*+0x130*/ += s->slowdown_time;        // +0x134
v->pending_kind     /*+0x138*/  = 8;
v->pending_attacker /*+0x13c*/  = r->owner;
emitter(r)->position = v->node matrix; radius = 300.0;     // 0x43960000
Sound_Play(1.0, emitter(r), .., "REPULSORHIT", 0);         // cue 35
Repulser_SetHitLatch(r, i, 1);                              // +0x1f0 + i
```

- **No falloff.** The impulse is the full `blastForce` whatever the distance,
  unlike every `blast()` in this engine.
- **Toward the wave's current point.** The craft sits between last tick's point
  and this tick's, so `-(P - A)` points along the wave's travel: a craft ahead of
  the firer is shoved forward, one behind is shoved back. Both are pushed away
  from the firer, as the name says.
- **Once per craft per Repulser** (the eight-byte latch at `+0x1f0`, cleared by
  `Repulser_Reset`).
- Shields are not tested here; the pending-damage drain does that, as for every
  weapon ([shield-pickup.md](shield-pickup.md)).

## 2026-10-04: live on PPSSPP - the timeline, the field model and the travel emitter

pulse-repulser-2. PPSSPP v1.20.4 software renderer, Pulse `UCUS98712`, Time
Trial on Talon's Junction, craft parked on the line. The Repulser was fired by
setting bit `0x10000` in world record 0's `+0x1b8` inside a
`Weapons_DispatchFire` (`0x08861814`) break. `Weapon_FireRepulser` (`0x0886ce8c`)
then broke with `a0` = pool `0x09b75ac0`, `live` 0, entity `0x09b75b90`.
`Repulser_Update` (`0x08875400`) was broken on every call until the entity
retired: **96 calls**. Three runs agree. The probe is `probe.py`, and
`live1.jsonl`..`live3.jsonl` are in the lane's scratch.

| Call | `dt` (`f12`) | age `+0x1ec` | state `+0x50` | `+0x204` | `+0x1f8` | `+0x210` | `+0x21c` | cursor 0 point | cursor 1 point |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | 0.016683 | 0 | 0 | 1.0 | 0.7 | 0 | 0 | - | - |
| 1 | 0.016968 | 0.016683 | 0 | 0.96 | 0.7 | 0.2 | 0 | - | - |
| 2 | 0.016471 | 0.033651 | 0 | 0.924 | 0.7 | 0.36 | 0 | - | - |
| 3 | 0.016677 | 0.050122 | 0 | 0.924 | 0.7 | 0.36 | 0 | - | - |
| 4 | 0.016831 | 0.066799 | 0 | 0.8916 | 0.7 | 0.488 | 0 | - | - |
| 44 | 0.016550 | 0.734266 | 0 | 0.609011 | 0.7 | 0.99968 | -1.8264 | - | - |
| 47 | 0.016607 | 0.784210 | 0 | 0.609011 | 0.865 | 0.99974 | -1.9155 | - | - |
| 48 | 0.016690 | 0.800817 | **1** | 0.609011 | 0.865 | **1.0** | -1.9155 | 416 (path 1) | 416 |
| 49 | 0.016723 | 0.817507 | 1 | 0.609011 | 1.02175 | 0.9 | -2.0029 | 421 | 414 |
| 50 | 0.016683 | 0.834230 | 1 | 0.609011 | 1.17066 | 0.81 | -2.0885 | 2 (path 0) | 412 |
| 95 | 0.016682 | 1.584987 | 1 | 0.609011 | 3.53009 | 0.02028 | -4.2149 | 227 | 322 |

Each row is read at the entry of that call, so it shows the state the previous
call left behind.

- **The waves step once per call, and the update runs once per 60 Hz frame**
  (`dt` = 1/60 to within 2 %). Cursor 0 moves `+5` points a call and wraps from
  path 1 to path 0 (416, 421, then 2 = 421 + 5 - 424). Cursor 1 moves `-2`.
  `Repulser_Update` and `Repulser_AdvanceWave`'s step counts rise to 90 and 88.
  This is one stationary Time Trial on PPSSPP. It confirms the steps-per-call
  law, not the frame pacing of real hardware.
- **The eases step `(int)(dt / (1/60))` times, so a call with `dt` under 1/60
  steps them zero times** (calls 3, 45 and 46). That is about a third of the
  calls on PPSSPP, the same artefact `ship-shockwave.md` measured. It is why the
  original is still shrinking at call 40 while a once-a-tick port has switched.
  Align frames by ease state, not by call count. Every stepped value matches the
  constants `Repulser_Init` writes: shrink `x 0.9 + 0.06`, the switch below
  `0.61` (0.610013 to 0.609011), grow `0.7 -> 0.865 -> 1.02175`, alpha
  `0.2, 0.36, 0.488` then `1.0, 0.9, 0.81` once the waves start. The wave start
  is at age 0.8008, just past `blast_time` 0.8.
- **`~REPULSORTRAVEL` is placed at the world origin.** The emitter's `+0x50`
  reads `0x09b75ce0` = entity `+0x150` (`Repulser_Init`, `0x088753a0`).
  `SoundEmitter_Update` (`0x08939720`) copies that pointer's `+0x30`, i.e.
  entity `+0x180`. That stayed `(0, 0, 0)` for all 96 calls, and so did the
  emitter's own position. Nothing in any Repulser function writes
  `+0x150..+0x18f`, and the pool allocates entities zeroed
  (`RepulserPool_Construct`: `Mem_Alloc(0x270)`, then a zero fill). Radius
  `600.0` confirmed (`+0x38`). Confidence 90.
- **And the cue binds no sound anyway.** `~REPULSORTRAVEL` (`weapons.bnk` cue
  34) is one command on both PSP pressings: opcode `0x14`, operand `0`, with no
  key-on and no child grain (`sblk_cue_audit`, pinned by
  `sfx_weapon_ground_truth::repulsortravel_binds_no_sound_on_pulse_psp`). The
  port plays nothing for it. PS2 was not checked; the audit example does not
  open the PS2 disc.

### Which ring is which, in the original's frames

The original's compact, beaded ring around the craft (calls 10-30) is **the
blast psys, not the field model**. The probe held `+0x210` and `+0x218` at 0 on
every call, which holds the field model's alpha at zero, and the bead ring
stayed (`shots/psp-zero-alpha.png`). The field model itself is a faint smooth
glow, which is also what this port draws (`shots/nbk.png`, blast skipped).

**Open, seen once:**

- The blast's bead ring differs. In this port it starts wide (about 13.6
  units) and collapses inward by call 20. In the original it is compact around
  the craft from call 10 to call 40, then whitens into a cloud. That is
  `WO_REPULSER_BLAST`'s own playback (`docs/formats/pob.md`), not the field
  model.
- At wave start the original shows a blue screen tint (flash kind 2) with the
  waves streaming off. This port whites out the whole frame
  (`shots/oag-still-047.png` against `psp-live2-049.png`). Still unexplained.

## The fork wave, read (2026-10-04, pulse-repulser-2)

`AiTrack_StepForward` (`0x0887e174`), read whole, with `mode` as its third
argument. A path record is `0x20` bytes: `+0x00` point count, `+0x08` points
(`0x70` stride), `+0x10` exit junction. A junction's four slots are
predecessors then successors (`oag_vex::track`): `+0x04` is `prev[1]`, so a set
one means the walk has **arrived at a merge**; `+0x08` is `next[0]` and `+0x0c`
is `next[1]`, the alternate.

```c
while (steps > 0) {
    room = path->count - point - 1;
    if (steps <= room) { point += steps; steps = 0; return 0; }
    point += room; steps -= room;                       // now on the path's last point
    if (exit->prev[1] && mode != 1) return 2;           // a merge: caller retries with mode 1
    if (exit->alternate && mode != 2 && mode != 3) return 1;   // a fork: steps left in *steps
    steps -= 1; point = 0;
    path = (mode == 3) ? exit->alternate : exit->next;  // 3 takes the branch, 2 the primary
    mode = 0;
}
```

`Repulser_AdvanceWave` (`0x08876914`), non-init call. It loops the stepper
until the steps are spent.

- **Return 1 with `+0x25c` clear:** spawn a third `WO_REPULSER` (`REP2`,
  `0x32504552`) anchored at `+0xe0`, keep its handle in `+0x5c`, reset the
  `+0xe0` matrix to identity, and copy **this wave's cursor** to `+0x24c`. Then
  `+0x260` = this wave's direction and `+0x25c` = 1. This wave carries on with
  mode 2, the primary.
- **Return 2:** retry with mode 1, which passes the merge.
- **After the loop:** if a fork wave is live, runs in this wave's direction,
  and is not the wave being advanced, advance it recursively:
  `AdvanceWave(r, +0x24c, +0xe0, +0x5c, +0x260, mode, steps, 0)`. On the call
  that spawned it, mode is 3 (take the alternate) and steps is what was left at
  the fork. After that it is the parent's mode and full step count. **On the
  spawn call the fork's previous point is set to its current one**
  (`+0x140 = +0x110`), so it sweeps nothing across the jump.
- **One fork per Repulser**, latched by `+0x25c` until `Repulser_Reset`.
- **The sweep tests it against craft only.** `RepulserPool_SweepTargets` tests
  Mines and Bombs against waves 0 and 1.

Confidence 88 for this in-run path, off the **forward** wave.

**`AiTrack_StepBackward` (`0x0887e2f0`) does not mirror it at an entry
junction.** Its listing reads the same `path+0x10` junction
(`0x0887e334 lw t1,0x10(t3)`) and the same slots (`+0x04`, `+0x0c`, `+0x08`).
On reaching point 0 it moves to that junction's `next[0]` (or `next[1]` under
mode 3) and lands on that path's **last** point (`count - 1`). What that means
for a backward wave at a path's start is not established: either the runtime
path record differs from the disc's, or the walk does something odd. So no fork
is built off the backward wave, and `AiTrack_StepBackward` stays at 80.

`Repulser_ForkAtJunction` (`0x08876634`) is the **init-tick** variant, called
from `AdvanceWave`'s `init != 0` branch when the slot is free. It runs once,
from the firer's own cursor:

1. `AiTrack_LocateOnSiblingPath` (`0x0887d4d4`) picks a sibling path. That is
   the other successor of this path's exit junction, else one of its entry
   junction's two predecessors. If one exists, it relocates the cursor with
   `AiTrack_UpdateCursor(500.0, .., excluded = this path)`.
2. If the relocated point is within **10 units** of the firer's (`dist^2 <
   100.0`), and `AiTrack_PathListsNeighbour` (`0x0887d970`) finds the new path
   in the old path's list (count at `track+0x0c+4i`, entries at
   `track+0x4c+0x10i`), the third wave spawns there.
3. Its direction is `Repulser_CursorPastHalfPath` (`0x088765e4`), i.e.
   `point > count / 2`. That sends it backward from the far half of the branch
   and forward from the near half.

Confidence 72. The list's meaning is inferred from this one use; hence the
`_q`s on the two helpers.

## AI

[weapon-ai.md](weapon-ai.md)'s switch puts weapon id 11 (Repulser) with the
Rocket, Missile, Cannon, Plasma, LeachBeam and Shuriken: flagged "ahead" and
aimed (`+0x58`). No Repulser-specific rule exists.

## What is built (2026-10-04)

`oag_tables::weapons::RepulserStats`, `oag_race::Course::{centre, corridor_width}`,
`oag_weapons::projectile::repulser` (the timeline, the walk, the sweep, the hit)
and, in `oag_game`, `Race::fire_repulser`, `advance_repulsers` (the craft sweep,
the Mine/Bomb sweep, `REPULSOR`/`REPULSORHIT`) and `advance_repulser_visual`
(`WO_REPULSER_BLAST`, two `WO_REPULSER`, screen flash kind 2). Chosen, not
measured: the waves follow the primary ring (no fork), step ring points
(four per control-point interval) from the firer's ring index, and take the
corridor width at the craft's nearest ring point; the effects' frame comes from
the nearest spline sample.

**2026-10-04, pulse-repulser-2:** the field model draws (`race::repulser_field`,
riding `bomb_blast::BombBlastModels`). Each `Repulser_Update` steps its eases
once, which is a chosen rate. Its basis comes from the nearest spline sample to
the firer, where the original reads the AI-track point under the firer's
cursor. `WO_REPULSER_BLAST` takes the field's basis turned by the `+0x21c` spin,
through `Stage::orient`/`stretch`. The third wave forks at a split
(`oag_weapons::projectile::repulser::fork` on `oag_race::Course::branches`;
05, 07, 14 and 23 carry a branch, `repulser_fork_ground_truth`), off the
forward wave only. The fork is
hashed only while live, so every committed reference reproduced unchanged.
`~REPULSORTRAVEL` stays unwired (see the live section). The blast's own
playback (flag `0x200000` and the selector-5 record, `docs/formats/pob.md`) and
the wave-start whiteout were the open picture gaps (the whiteout closed 2026-10-06, see [particle-system.md](particle-system.md), "the guard band is the law").

**2026-10-04, pulse-psys-ring: the blast's ring matches.** Four interpreter laws were
missing and are now played:

- the emitter's playback rate `4`;
- selector 5 as the newborn's lifetime co-factor `1.5`;
- flag `0x200000`'s even step, `7.2` degrees a bead;
- the ring's aimed azimuth sense, `+0.244`.

The blast also rides `+0x1a0`: `Repulser_SpawnBlastEffect` hands `Psys_Spawn_q` the
matrix by pointer (`param_5 = 1`), and the emitter's flag `0x2` draws the pool through
it live. With these, the ring is wide at update 10, compact at 20 and 30, a white puff
at 40, and gone at 47, as on the PSP. See `particle-system.md`, "The emitter's clock
and the burst laws". **The wave-start whiteout is `WO_REPULSER`, not the flash** (closed 2026-10-06: the GE
drops the out-of-range `shazzam` quad). The spin's sense is still unmeasured, and
it shows now that the beads ride the spin.

**HD now hands it out on this law.** HD's `weaponstats_elimination.xml`
(`DATA00`/`DATA02.PSARC`) authors the same block and weights it `ai=8 human=8`;
HD's own Repulser law is unread.

## Not read

- `FUN_0886d474` (remote destroy by id, under 50, not renamed) and the network
  broadcast payloads.
- What `AiTrack_StepBackward` meets at a path's start (see the fork section).
  It reads the exit junction's successors.
- Whether the branches on 07, 14 and 23 are real second routes or coincident
  duplicates of their primary path. Their sample counts match exactly; 05's do
  not.
- The hit law is still static: a write breakpoint on a victim's `+0x110`
  during a live Eliminator Repulser would lift `Repulser_HitCraft`.
- The spin's sense against `Quat::from_axis_angle`. It does not show while the
  blast's ring angles are random.

## The wave's extent and frame, measured (2026-10-04, pulse-psys-shape8)

`Repulser_AdvanceWave`'s co-factor write (`0x08876fc0..0x08877020`) sets the
`WO_REPULSER` instance's `+0x2c` to **the distance between the track's two edges over
100**, not the wave's step: `sp+0x50` is `pos - lateral * point+0x44`, `sp+0x60` is
`pos + lateral * point+0x48`, and neither is rewritten before the `vsub.q`. The root's
extent `50` is therefore half the track's width. Live on Talon's Junction both waves
read `0.395..0.401` while the forward wave stepped about 30 units an update.
Confidence 92. Earlier text on this thread that said `|step| / 100` was wrong.

The wave matrix's `Z` is `-(across x up)` for direction 0 and `across x up` for
direction 1, then `X = Y x Z`. Under [track.md](../../../formats/track.md)'s handedness
(`left x up = forward`, so `across x up` points **backward**) both waves' `Z` is their
travel, which the live matrices confirm. Shape 8 spawns on the `+Z` half ring, so the
spray bows ahead of each wave. `crate::race::repulser_field::field_basis` names
`across x up` "forward"; the label is wrong, the matrix it builds is the original's.

Played since this change: `race::weapons::repulser` hands every wave
`stretch(width / 100, -across)` (forward and fork) or `stretch(width / 100, across)`
(backward). Shape 8 and the class-6 bar are in
[particle-system.md](particle-system.md), "Shape 8, the class-6 bar and the wave's
width", with the GE evidence for the wave-start `shazzam` bar, which ours now drops as the GE does (2026-10-06).
