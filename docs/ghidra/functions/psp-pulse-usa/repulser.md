# The Repulser: a blast at the firer, then two shockwaves that run along the track

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** **read 2026-10-04** (pulse-repulser lane), decompile plus
instruction-level checks of every load-bearing constant. Not yet runtime-verified,
so every score here is capped at 84.

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
| `0x08875210` | `Repulser_Init` | 84 |
| `0x08875400` | `Repulser_Update` | 84 |
| `0x08875658` | `Repulser_Reset` | 80 |
| `0x08875864` | `Repulser_GetWavePoints` | 80 |
| `0x088758b0` | `Repulser_SetHitLatch` | 80 |
| `0x088758c0` | `Repulser_HitLatch` | 80 |
| `0x088758cc` | `Repulser_UpdateFieldModel` | 72 |
| `0x088761d8` | `Repulser_SpawnBlastEffect` | 84 |
| `0x08876300` | `Repulser_SpawnWaves` | 80 |
| `0x08876634` | `Repulser_ForkAtJunction` | 60 |
| `0x08876914` | `Repulser_AdvanceWave` | 80 |
| `0x0887e174` | `AiTrack_StepForward` | 80 |
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
into `+0x228`. `Repulser_UpdateFieldModel` (`0x088758cc`) builds that model's
matrix from the firer's node each tick: uniform scale `+0x204` while it is above
`0.61`, `+0x1f8` after, vertex alpha `+0x210 * 255`, and the `+0x21c` spin.
Confidence 72: the matrix arithmetic was read at shape level only.

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

## AI

[weapon-ai.md](weapon-ai.md)'s switch puts weapon id 11 (Repulser) with the
Rocket, Missile, Cannon, Plasma, LeachBeam and Shuriken: flagged "ahead" and
aimed (`+0x58`). No Repulser-specific rule exists.

## What is built (2026-10-04)

`oag_tables::weapons::RepulserStats`, `oag_race::Course::{centre, corridor_width}`,
`oag_gameplay::projectile::repulser` (the timeline, the walk, the sweep, the hit)
and, in `oag_game`, `Race::fire_repulser`, `advance_repulsers` (the craft sweep,
the Mine/Bomb sweep, `REPULSOR`/`REPULSORHIT`) and `advance_repulser_visual`
(`WO_REPULSER_BLAST`, two `WO_REPULSER`, screen flash kind 2). Chosen, not
measured: the waves follow the primary ring (no fork), step ring points
(four per control-point interval) from the firer's ring index, and take the
corridor width at the craft's nearest ring point; the effects' frame comes from
the nearest spline sample.

**Not drawn: the field model.** Deferred for time, not confidence - its easing
and frame are read above. Until it lands the 0.8 s blast phase shows almost
nothing: `WO_REPULSER_BLAST` is fifty sub-unit sparks on a 13.6-unit ring,
collapsing inward at 0.625 units a tick (read off the parsed `.pob`, 2026-10-04).
Its flag `0x200000` (evenly stepped ring angles) and selector-5 record are not
played by `oag_render::psys`.

**The wave-start frame is close to a whiteout** in this port: both waves'
5-to-17-unit sprites spawn at the firer, just ahead of the chase camera.
Unverified against the original.

**HD now hands it out on this law.** HD's `weaponstats_elimination.xml`
(`DATA00`/`DATA02.PSARC`) authors the same block and weights it `ai=8 human=8`;
HD's own Repulser law is unread.

## Not read

- `FUN_0886d474` (remote destroy by id, under 50, not renamed) and the network
  broadcast payloads.
- `FUN_088765e4`: "is the cursor past half its path", used by the fork tick;
  under 50, not renamed.
- The field model's exact matrix and whether it draws over the blast effect.
- Nothing here is runtime-verified. A PPSSPP watchpoint on a live Repulser's
  `+0x1ec` age and `+0x50` state, and a write breakpoint on a target's `+0x110`,
  would lift the timeline and the hit law to the 85-94 band.
