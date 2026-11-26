# The Rocket's craft blast: where it stands and what it rides

2026-11-26. Read in `/hdfury/EBOOT-ps3-hdfury-eu.elf` (TOC `r2 = 0x008ad4d8` for every
function here, `scripts/ps3-toc.py toc`: "exact"), with the 2048 (`vita-2048-eu-v104`) and
Omega (`ps4-omega-eu`, patch 1.07) twins and Pulse's PSP spawn for contrast. Ghidra marks part
of the HD spawn as bad data (a Cell `lvlx` it cannot decode), so the carry block below is
quoted from `llvm-objdump -d --triple=powerpc64`.

**The law (confidence 80).** On a Rocket's craft hit the spawn copies the struck craft's
position, dropped 2.5, into the new `WO_ROCKET_EXPLO` once. It then writes the craft's body
velocity times 1/60 into the instance's inherited velocity. Every particle born afterwards adds
that times its emitter's `+0x4d0`. The blast is not parented: its emitters stay where it was
spawned. The velocity is set at the spawn; no other call to the setter reaches this instance,
but the effect node's own per-tick update is unread, so a later write by another path is not
excluded (below). The cloud
travels with the craft only because `WO_ROCKET_EXPLO` authors `+0x4d0 = 1.0` on every root, and
it falls behind because those roots author a drag of 0.92 a tick.

| Address | Name | Confidence | What it does |
| --- | --- | --- | --- |
| `0x0014aa88` | `Rocket_HitCraft` | 72 | the only caller of the spawn; takes the struck racer, spawns the blast once if any viewport sees the craft (`0x000d02f0`), then pushes nearby craft (`0x00149f70`) |
| `0x0014a7d8` | `Rocket_SpawnCraftBlast` | 80 | spawns `WO_ROCKET_EXPLO` (tag `ROEX`) at a copied matrix, hands it the struck craft's velocity, submits a point light |
| `0x00283b40` | `Psys_SetInheritedVelocity` | 82 | stores a vector at instance `+0x160` and recurses into `+0x24` and `+0x1b4` |
| `0x0027c288` | `Psys_FindInstanceByHandle` | 80 | the handle table on `*(manager) + 0x468c` (key mod table size, stride 12, match on `+4`) |
| `0x00149f70` | `Rocket_PushNearbyCraft` | 82 | for every racer but the firer within Rocket `blastradius` of the hit, adds `normalize(d) * (1 - |d| / blastradius) * blastforce` to its slot's pending impulse `+0x110` ([below](#the-push-2026-11-26-projectile-leftovers)) |
| `0x00287168` | `Psys_PrepareInstance_q` | 62 | walks the record's sprite template list (gate `+0x9a4`, list `+0x9a8`) and makes a `0xa0`-byte particle for each |
| `0x0028bb18` | `Psys_InitTemplateParticle_q` | 55 | initialises one template's particle from its record |

## The spawn copies a point

`Rocket_HitCraft` (`0x0014aa88`) reads the struck racer `*(this + struck * 4 + 0x58)`, takes its
position at `+0x90` and lowers y by the TOC float at `-0x2500` (`0x40200000`, 2.5), the drop
Pulse's spawn already carries (`oag_title::engine_effects::CRAFT_BLAST_DROP`). That vector is
the only argument `Rocket_SpawnCraftBlast` gets besides `this` and the struck index.

`Rocket_SpawnCraftBlast` builds an identity matrix on its stack (rows from `0x00769d70`) with
that position in row 3, allocates a `0x180`-byte effect node, registers it with
`RaceManager` (`0x003238f8`), and calls the effect constructor:

```
0014a8e0  lis   r5,0x5845 ; ori r5,r5,0x4f52   tag 'ROEX'
0014a8e4  lwz   r4,-0x2510(r2)                 "WO_ROCKET_EXPLO" (0x007854f0)
0014a8f0  addi  r6,r1,0xb0                     the stack matrix
0014a8f4  li    r7,0x0                         attach flags
0014a8f8  li    r8,0x0
0014a8fc  bl    0x002c9108
```

The constructor (`0x002c9108` -> `0x002c8c70`) keeps the caller's pointer only when bit 0 of
the flags is set (`param_1 + 0xd0 = param_4`, an attached effect, the shape
`WO_SHIP_COLL_SPARK_DAMAGE` uses with flags `1`). With flags `0` and a matrix it copies the four
rows into the node's own `+0xf0..+0x12f` and points `+0xd0` there. So the blast keeps a copy:
it does not ride the craft's transform.

## The carry

```
0014a92c  lwz   r3,0x468c(r9)                  the particle manager's handle table
0014a930  bl    0x0027c288                     instance of the handle just spawned (*0x00ad5fd4)
0014a93c  beq   0x0014a994                     none: skip
0014a94c  addi  r10,r2,-0x2504                 0x008aafd4 = 0x3c888889, 1/60
0014a960  lvlx  v13,0,r10                      (Cell; Ghidra: bad data)
0014a96c  vspltw v13,v13,0
0014a970  lwz   r10,8(r11)                     the struck racer
0014a974  lwz   r9,0xf0(r10)                   its craft
0014a978  lwz   r11,0x6944(r9)                 its body
0014a97c  lvx   v0,r11,r0                      r0 = 0x190: the body's velocity
0014a984  vmaddfp v0,v0,v13,v1                 velocity * 1/60 (+ 0)
0014a988  stvx  v0,r1,r0                       r0 = 0xa0
0014a98c  bl    0x00283b40                     Psys_SetInheritedVelocity(instance, &v)
```

Body `+0x190` is the velocity the physics pages already read (`detonator-mine.md`,
`zone-destroy.md`). `Psys_SetInheritedVelocity` writes instance `+0x160`, which
`Psys_InitParticle` (`0x00289890`, `0x002899ac`-`0x00289a18`) adds to each newborn's velocity
times the record's `+0x4d0` ([engine-trail.md](engine-trail.md), "What the float and the
velocity do"). A child instance takes the parent particle's velocity times its own `+0x4d0`
(`0x0028a344`-`0x0028a358`), which `oag_fx::psys` already does.

**Set at the spawn.** Every direct branch to `0x00283b40` in the executable (`llvm-objdump`,
`bl 0x283b40`): `0x000d99a8` (in `Ship_PlayDestroyExplosion`), `0x00124738`, `0x00124cec`,
`0x0014a98c` and its own recursion. Only `0x0014a98c` reaches the blast, and it runs at the
spawn. That does not cover every writer of `+0x160`: `Trail_HitShipEffect` (`0x002d9fb4`)
writes it with no call to this setter ([engine-trail.md](engine-trail.md)). The blast's
`0x180`-byte node takes the table at `0x008745a0` (`0x0014a910`), whose first entries are
constant getters (`0x00682b60`, `0x00681e60`, `0x0067d518`); its per-tick method was not
found, so whether anything rewrites the velocity after the spawn is unread. A craft at a
steady speed would look the same either way, so the capture below cannot settle it.

What `WO_ROCKET_EXPLO`'s twelve emitters author, read through `oag_fx::psys` from the disc:
`+0x4d0 = 1.0` on all eleven but `trail` (`0.0`); roots `WO_ROCKET_EXPLO`, `SMOKE_BLUCLD`,
`FIREBALL_FIJETS` and `FIREBALL_TUMULT` drag `0.92` a tick, `FIRESPIKES` and
`FIRE_TRAILS_ONEQUAD` none.

The spawn ends by submitting a point light at the same position: colour `(7, 5, 1)` from a
static it fills once (`0x00994510`), `D = 100.0` (TOC `-0x2538`), `w = 1.0` (TOC `-0x2570`),
through the `SpuLight_AddCandidate` thunk `0x006778c8` ([renderer.md](renderer.md)).

## Against the capture

The 2026-11-26 film ([particle-fill.md](../../../rendering/particle-fill.md), "Against the
original at a near blast") measured the original's large-particle centroid behind the struck
craft. Ours with the law, a forced hit on a rival at about 100 units a second
(`craft_blast_carry::hds_craft_blast_travels_with_the_struck_craft`, all live particles):

| Age | Original: craft moved | Original: cloud from craft | Ours: craft moved | Ours: cloud from craft |
| --- | --- | --- | --- | --- |
| 0.18 s | 23.9 | 6.1 | 19.3 | 5.8 |
| 0.23 s | 29.0 | 9.6 | 24.3 | 7.4 |

The cloud's lag over the craft's travel: original 0.26 and 0.33, ours 0.30 and 0.30. Without
the carry ours stays at the spawn point: 0.6 units along a craft that moved 34.

**Where each stands** (2026-11-26, three films, confidence 90): the blast's template quads hold
one world point that equals the struck craft's position, dropped 2.5, about 2 ticks before the
blast's birth tick (0.01 to 0.30 units off). A Rocket volley lands as two such points, one blast
and then two together 5 to 6 ticks later
([particle-fill.md](../../../rendering/particle-fill.md#posing-the-near-hits)). The 2-tick lag's
cause is unread.

## The other titles

- **2048: ported.** `Rocket_SpawnCraftBlast` (`0x81206928`, confidence 80) spawns `WO_ROCKET_EXPLO` (tag `ROEX`) with flags `0, 0` at
  a stack matrix of the passed position, looks the instance up by its handle (`+0xec`), and
  hands it `*(*(*(racer)+0xe8)+0x5f84)+0x90` times `0x3c888889` (1/60) through
  `Psys_SetInheritedVelocity` (`0x812aa644`, 80), which stores it at `+0x140` and recurses into `+0x24` and `+0x184`: the
  same setter. A light of `(7, 5, 1)` follows (`FUN_8103731c`).
- **Omega: ported.** `Rocket_HitCraft` (`0x0139a6b0`, 75; the only `ROEX` immediate in the 1.07 executable)
  lowers the racer's `+0x90` by 2.5, spawns through `FUN_0175ea50` with flags `0, 0`, looks
  the instance up by `+0xc0` and hands it `*(craft + 0x75c8) + 0xa0` times `0.016666668`
  through `Psys_SetInheritedVelocity` (`0x01728780`, 80; stores `+0x1d0`, recurses `+0x88` and `+0x228`). It queues a light
  of `D = 100`, colour `(7, 5, 1)`, and pushes nearby craft by `(1 - d / r) * strength`, as
  HD's `Rocket_PushNearbyCraft` does.
- **Pulse PSP: checked, differs.** `Rocket_SpawnCraftExplosion_q` (`0x0886ed34`) spawns at a
  copied matrix with flags `0, 0` and writes no velocity, then starts `ScreenFlash_Start(0)`.
  Pulse PS2 inherits it. Pure is not read.

Wired as `oag_title::Effects::craft_blast_carry` (HD, 2048 and Omega `Measured`);
`RaceView::craft_blast_velocity` hands the struck craft's velocity to `Stage::play_carried`.

## The screen wash: the blast's sprite templates

2026-11-26, `blast-wash-capture`. **The full-screen wash on a Rocket's craft hit is
`WO_ROCKET_EXPLO`'s sprite templates: the yellow wash is `glow_longer`, the template on the
root emitter, and the white flash of the hit frame is a `Glow`, the template on
`FIRE_TRAILS_ONEQUAD` (and `MOLTEN_DEBRIS`, which authors the same one).** Confidence 85 for
`glow_longer`, 70 for the `Glow`s. Both are drawn as single additive quads capped at exactly
twice their depth (measured 2.0, read nowhere).

A template is one particle an emitter's instance starts with, created whatever the emission
count says ([pob.md](../../../formats/pob.md), `oag_pob::initial`; read on Pulse first). On HD
`FUN_00287168` reads the record's gate at `+0x9a4` (`0x00287534`) and its list at `+0x9a8`
(`0x00287540`), allocates a `0xa0`-byte particle for each record and initialises it through
`FUN_0028bb18` (`0x0028757c`). That is Pulse's initialiser's shape, so both carry a name at
`_q` confidence: `Psys_PrepareInstance_q` (62; the function does more than this walk, which is
all that was read) and `Psys_InitTemplateParticle_q` (55; called once per template record with
the new particle, the record and the instance, body unread).

### What was filmed

RPCS3 0.0.43-20574, HD EU, the weapons-grid save state of
[particle-fill.md](../../../rendering/particle-fill.md) ("Against the original at a near
blast"), restored fresh for every film, the Rocket fired 12 units behind AG Systems with
`scripts/rpcs3-hd-rocket-film.py`. Each arm is a data write before the shot (`--arm`; the PPU
recompiler ignores writes to code, and RPCS3 applies no `patch.yml` on a save-state restore).
**Falsifier, written before the films:** a candidate is named only if its arm removes the
single-quad additive draws with half height = 2 x depth in both of its films, against two
controls that have them. Frames whose command ring was not caught (every other age) are not
evidence either way.

| Film | Arm | Struck | Wash quads at the ages with draws |
| --- | --- | --- | --- |
| control 1, 2 | none | AG Systems | 1, 2 (0.05, 0.18 s); 3, 2 (0.1, 0.23 s), yellow `(255, 240, 99)` to `(255, 222, 76)` |
| light off | the blast light's colour zeroed, its fill guard set | AG Systems | 3, 2: same yellows |
| `FIRESPIKES` off | all three records' per-emission count and live cap zeroed | AG Systems | 1, 3: same yellows |
| tag caps (2 films) | every loaded record with cap 2.0 retagged 1.1 to 1.6 | AG Systems once, another craft once | still exactly 2.0 |
| whole tree off | emission zeroed on every `WO_ROCKET_EXPLO` record | another craft | blast's own quads gone (no cap 0.5 quad, alpha-over fill 0.0 to 0.1 screens against 5.8), wash quads still there |
| both templates off (2 films) | gate `+0x9a4` zeroed on the root and `FIRE_TRAILS_ONEQUAD` | AG Systems | **none**, blast otherwise unchanged (374 and 1,152 quads against 399 and 1,176) |
| root template off (2 films) | gate zeroed on the root only | AG Systems | **no yellow**; white quads at 1.999 on the hit frame remain |
| trail template off | gate zeroed on `FIRE_TRAILS_ONEQUAD` only | AG Systems | yellow remains |

- **The light is not it.** `SpuLight_AddCandidate` is a per-vertex sum over geometry
  ([weapons.md](weapons.md), "Weapon point lights"), so a zero colour adds nothing, the same as
  removing the call. The quads stayed.
- **No emitter is it.** The whole-tree arm shows the record writes reach the draw (the blast
  collapsed), and the wash stayed. No loaded record capped at 2.0 drew it either.
- **The templates' tables are the wash's colours.** Read off the disc through `oag_pob`, with
  the bytes the live memory held: `glow_longer` 30 ticks, additive, a keyed size 0 to 40, colours
  `(255, 245, 105)`, `(255, 240, 99)`, `(255, 227, 83)`, `(255, 210, 62)` over its life; `Glow`
  3 ticks, size 100, white to `(255, 237, 197)`. Each of the eight non-white colours the 39 quads
  carry is an entry of `glow_longer`'s table. At these sizes every quad nearer than 20 units is capped, which is why
  each covers the screen.
- **Why 85 and 70.** `glow_longer` was removed by its own gate in two films and by both gates
  in two more, and its colour table matches. The white `Glow` rests on one root-only film that
  kept it, its table and the 2.0 cap; which of the two identical `Glow`s draws it is not
  separated.

Raw films: `data/reference/hd-capture/rocket-craft-hit/` holds the earlier hits; this lane's
arms were not kept. Reproduce with `--arm none`, `--arm templates-off:WO_ROCKET_EXPLO,FIRE_TRAILS_ONEQUAD`
and `--arm templates-off:WO_ROCKET_EXPLO` on the same state, then count quads with
`scripts/hd-particle-quads.py` (the `2.0` cap column).

### The cap

Over the ten films, 37 of the 39 single additive quads larger than 1.05 x depth sit at half
height 1.98 to 2.001 x depth, and the other two are smaller (the lowest 1.707), so they are capped,
not drawn at a fixed screen size. A template record ends at
`0x900` and has no `+0xc80`, and retagging the parent emitters' caps did not move them, so the
2.0 comes from somewhere else in the builder. Unread.

### Wired

`oag_hd` lists the craft blast in `Effects::templates`, so ours plays its three root templates
(`glow_longer` and both `Glow`s) with its own sprites, and `NearLaw::template_cap` caps a
template at 2.0 x depth (HD measured; 2048 and Omega inherited, below). Tests:
`near_law::hds_craft_blast_plays_the_screen_wash_templates` (disc),
`near::tests::a_template_is_capped_by_the_laws_template_cap`, and the disc check
`check_hd_craft_blast_templates` in `pob_ground_truth.rs`.

Pairs (`compare_env.overlap_score` RMSE x 255, same tree, HD's templates off and on): 0.1 s
119.7 to **118.0**, 0.233 s 126.2 to **125.2**.

**A template takes nothing from the rest of its effect.** Played first as one more emitter, the
templates moved the 0.233 s pair to 140.0. With the templates listed but muted it stayed at
126.2, so the change came from their particles being born, not from their look. Those particles took draws from the
instance's shared random stream, which moved every later particle of the blast, and slots from
a full 256-particle pool, which changed which smoke survived. The original allocates a
template's particle on its own (`0x00287560`, `0xa0` bytes) outside the emitter pool. Ours now
keeps eight template slots beside the 256 (`oag_fx::psys`, `MAX_TEMPLATE_PARTICLES`), and a
tick's templates draw from one copy of the shared stream, so each template keeps the values it
drew before and every other particle lands where it would without templates.

| Arm (two renders each, identical) | 0.1 s | 0.233 s |
| --- | --- | --- |
| templates off | 119.7 | 126.2 |
| listed, muted | 119.7 | 126.2 |
| on, shared stream and pool | 117.8 | 140.0 |
| on, shared stream, template reserve | 116.2 | 138.5 |
| on, stream copy and template reserve (wired) | 118.0 | 125.2 |

The 0.233 s pair swings by 14 with the arrangement of the blast alone, so a pair this close to the
original's white frame measures our random arrangement as much as the wash.

**`glow_longer` is a half disc, and it stands on the blast.** A hit 30 units ahead draws it as a
flat-bottomed dome on the original too; ours had HD's `.gtf` sprites upside down, which the
loader now turns ([particle-fill.md](../../../rendering/particle-fill.md), "HD's particle sprites
are bottom-up"; the pair figures above predate the turn).

### The other titles

- **2048: ported (inherited trigger).** Its race plays its own
  `particles2048/WO_ROCKET_EXPLO.pob` (3 emitters), whose template is a different one,
  `BANG_GLOW` (60 ticks, size 0 to 30, `(255, 221, 0)` at alpha 180), and that is what ours
  plays. The `particles` copy inherited from HD carries HD's three. That the craft blast plays
  its templates, and the 2.0 x depth cap, are HD's, taken by lineage (`Origin::InheritedFrom`),
  not measured on 2048.
- **Omega: ported (inherited trigger).** Ours reads the effect directory by circuit (a choice,
  [particle-paths.md](../ps4-omega-eu/particle-paths.md)):
  `Data/particles2048/WO_ROCKET_EXPLO.pob`, read on a 2048 circuit, carries HD's `glow_longer`
  and `Glow`, and `Data/particles/`, read elsewhere, none, in the base archive and the 1.07 patch
  alike. Ours plays each file's own: the two on a 2048 circuit, nothing on an HD circuit. The
  trigger and the cap are inherited from HD, as on 2048.

The files are pinned by `the_rocket_craft_blast_templates_on_2048_and_omega`; what each title
loads by `the_2048_craft_blast_loads_under_the_near_law` and
`the_omega_craft_blast_loads_under_the_near_law` (Altima for the 2048 directory).

## Open

- Where HD's template cap of 2.0 is read (above, "The cap").
- Which point of the craft the slot's `+0x50` is (the push's origin): Pulse's `T+0x50` is the
  nose, 4.875 ahead; HD's is taken to be the same point (below, "Open from the push").
- Omega's and 2048's own drains (below, "The other titles").

## The push (2026-11-26, `projectile-leftovers`)

Read in `llvm-objdump -d --mattr=+altivec,+vsx` over the decrypted `EBOOT.elf` (Ghidra stops at
the Cell `lvlx` at `0x00149fa8`), TOC `0x008ad4d8` for every function here
(`scripts/ps3-toc.py toc`: "exact"), strings resolved with `scripts/ps3-toc.py resolve`.

**The law (confidence 82).** A Rocket's craft hit posts, for every racer but the firer,

```text
d = slot+0x50 - P                          ; P = rocket+0xf0
if |d| < Rocket.blastradius:
    slot+0x110 += (d / |d|) * (1 - |d| / Rocket.blastradius) * Rocket.blastforce
```

and once a tick the craft's drain spends `slot+0x110`:

```text
if slot+0x110 != 0:
    if !(slot+0x200 & 0x10):                      ; the running Shield
        p    = slot+0x110
        side = body+0x1d0                         ; basis row 0, the right row
        p    = side * (dot(side, p) >= 0 ? |p| : -|p|)
        Body_ApplyImpulseAtPoint(body, body+0x200, p, 0)   ; at the centre: v += p * invMass
        craft+0x2d8 += 0.5                        ; the stun
    slot+0x110 = 0
```

`slot` is the craft's pickup slot (`*(ship+0x5edc)`, the object the Mine's and the Bomb's pushes
write, [detonator-mine.md](detonator-mine.md), [detonator-bomb.md](detonator-bomb.md)), `body`
its rigid body (`*(ship+0x6944)`), `craft` its engine object (`*(ship+0x5fac)`). So
**`+0x110` is a pending impulse, not a velocity**, and HD spends it as Pulse's
`Ship_ApplyCollisionImpulse` does (magnitude kept, a sign, one axis, `0.5` s of stun, nothing
under a Shield), **along the craft's side axis, not its forward axis.**

### `Rocket_PushNearbyCraft` (`0x00149f70`), 82

```text
00149f70  lwz  r9,-0x2578(r2) ; lwz r10,0(r9)    racer count, *0x00936fe8
00149f80  lwz  r12,-0x2568(r2)                   stats sets, *0x009389a4
00149fa0  lwz  r0,0xc(r12)                       the race's set index
0014a008  lwzx r8,r12,r0                         Rocket record of that set
00149fe8  cmpw cr7,r5,r10 ... bt 30,skip         r5 = firer: skipped
0014a014  lvx  v11,r11,r6                        r6 = 0x50: slot+0x50
0014a01c  vsubfp v11,v11,v0                      minus *r4, the point
0014a034  vrsqrtefp ... vmaddfp                  one Newton step (0.5 at TOC -0x256c)
0014a064  lfs  f0,0x20(r8) ; fcmpu ; bf 28,skip  |d| < blastradius
0014a074  lfs  f13,0x24(r8)                      blastforce
0014a078  fdivs ; fsubs f0,f11,f0 ; fmuls        (1.0 - |d| / R) * S (1.0 at TOC -0x2570)
0014a0a0  vmaddfp v0,v13,v0,v1 ; stvx v0,0,r9    slot+0x110 += dir * that
```

The record's fields are named by `WeaponStats_ParseRocket` (`0x0002d9f8`, the `Rocket` branch
of the stats walker `0x0002fab0`): `damage +0x08`, `venomspeed`..`phantomspeed +0x0c..+0x18`,
`launchspeed +0x1c`, **`blastradius +0x20`**, **`blastforce +0x24`**, `spread +0x28`,
`absorb +0x2c`, `slowdown_time +0x30`. `Rocket_HitCraft` reads the same record's `+0x08` as the
struck craft's damage and `+0x30` as its slowdown, two more readers that agree on the layout.

The call (`0x0014ac54`-`0x0014ac74`) passes `P` from `0x00123c80(rocket, &a, &b)`, which copies
`rocket+0x90` and `rocket+0xf0`; `b`, the argument, is `+0xf0`, the rocket's previous position in
the live table of [plasma.md](plasma.md) ("The objects, read live"). `r5` is `rocket+0x44`, the
firer's index: **the firer is skipped and the struck craft is pushed**, the shape of Pulse's
`Rocket_ApplyBlastForce` (`oag_weapons::projectile::blast`'s `blast_direct_hit`).

**Every race blast is this function.** The same instructions, with only the record offsets
changed, are the Plasma's `Plasma_PushNearbyCraft` (`0x00146470`, `+0xc4`/`+0xc8`, the
Plasma's `blastradius`/`blastforce` in [plasma.md](plasma.md)'s parser table) and the Missile's
`Missile_PushNearbyCraft` (`0x00142258`, in `MissileManager`'s span, `+0x4c`/`+0x50`, named
`blastradius`/`blastforce` by `WeaponStats_ParseMissile` at `0x0002d6a8`). The Detonator Bomb's
(`0x0012e598`) and Mine's (`0x0011aa88`) pushes write the same slot by the same falloff. So one
switch for the title is faithful: `oag_title::weapons::Weapons::blast_push`.

### `Ship_ApplyPendingImpulse` (`0x000d8a28`), 80

```text
000d8a38  lwz  r5,0x5edc(r3)                     the slot; none: return
000d8a4c  li r9,0x110 ; lvx v13,r10,r9            pending; each of x, y, z tested against 0
000d8a64  lwz r0,0x200(r10) ; rlwinm 0x10         the running Shield
000d8acc  lwz r3,0x6944(r31)                     the body
000d8ad4  vmaddfp ... vrsqrtefp ... (1.0, 0.5 at TOC -0x49c4, -0x49d4)   |p|
000d8b34  lvx  v12,r9,r0                         r0 = 0x1d0: body row 0
000d8b38  ... vcmpgefp. dot(row0, p) >= 0        sign
000d8b70  vmaddfp v0,v12,v0,v9 ; stvx            p = row0 * (+-|p|)
000d8b8c  bl   0x000f6348                        Body_ApplyImpulseAtPoint(body, r4, p, 0)
000d8ba0  lfs f0,0x2d8(r9) ; fadds 0.5 ; stfs    *(ship+0x5fac)+0x2d8 += 0.5
000d8a74  ... stvx (zero) to slot+0x110          consumed either way
```

The one caller (`0x000ead98`) passes `r4 = body+0x200`, the body's position
([physics.md](physics.md), "The body's pose fields"), so the lever arm is zero and only
`v += p * invMass` acts ([wall-response.md](wall-response.md), `Body_ApplyImpulseAtPoint`).
Body row 0 is `cross(row1, row2)`, the right row ([rpcs3-capture.md](../../../reverse-engineering/rpcs3-capture.md),
the pose fields; `wall-response.md` reads it as the right row too). `craft+0x2d8` is the stun:
`Craft_ApplyLateralGrip` returns while it is positive ([hover-four-point.md](hover-four-point.md)),
and [craft-flags.md](craft-flags.md) reads it as `stunned`. The caller skips the drain when the
craft's state word is `2` (`0x000ead88`); that state is not named here.

**Why 82 and 80, not higher.** Decompilation only, with consistent readers: three parsers' names
agree with three push clones and with `Rocket_HitCraft`'s own reads, and the drain is the only
reader of `slot+0x110` found among the 170 loads of `ship+0x5edc`. No breakpoint watched it
run. The cheapest proof is one RPCS3 break at `0x000d8b8c` in the weapons-grid state of
[particle-fill.md](../../../rendering/particle-fill.md), reading `slot+0x110` against body `+0x1d0`.

### Against ours

Pulse's law (`oag_physics::wall::post_blast_impulse` and `apply_pending_impulse`) is the same
posting, the same falloff, the same skip of the firer, the same shield gate and stun. **The
drain's axis** was ours' `body.forward()` on every title; HD's is the side row.

**Pulse has the same axis, measured 2026-11-26.** Pulse's `Ship_ApplyCollisionImpulse`
projects onto row 0 of `*(entity+0x794)`, the left row, and 11 live drains put every velocity
change on that row with the pending vector's full length
([contact-response.md](../psp-pulse-usa/contact-response.md#the-drain-pushes-along-the-side-row-measured-2026-11-26),
92). HD's drain row is the right row, Pulse's the left; the sign rule cancels the difference
except on an exactly zero dot.

Wired as `oag_physics::laws::Laws::pending_impulse_on_side` (`CraftLaws` per title: HD and
Pulse measured, Pure inherited from Pulse; 2048 and Omega `false`, chosen, below), and HD's
`blast_push` `Posted` (measured). The posting point is ours' nose point
(`oag_physics::wall::nose_point`, Pulse's 4.875): HD's slot `+0x50` is taken to be the same
point, as its `+0x90` is the centre `Rocket_HitCraft` spawns the blast at and Pulse's `T`
object has the nose at `+0x50` and the centre at `+0x90` (confidence 60 for the point, open).
Tests: `oag_physics` `wall::impulse::tests::on_side_a_pending_impulse_pushes_along_the_side_axis`,
`oag_raceplay` `launch_hover::tests::the_blast_drain_axis_follows_the_title` and
`tests::blast_push::a_posted_blast_shoves_the_player_sideways_on_pulse_and_hd` (Pulse's and
HD's title data against Omega's immediate push: reverting either title's axis, the laws
mapping or the axis choice turns it red).

What the HD field felt before and after (2026-11-26, `OAG_REQUIRE_GAME_DATA=1`): the novice Score
Eliminator pace test (`hd_eliminator_hud_ground_truth`), which the lineage page recorded red
under the earlier Pulse posting on HD, stays green, and `hd_original_ai_ground_truth` too. The
eliminator test does not tell the two axes apart: with HD's drain set to forward it is green as
well, so it is no evidence for the side row.

### The other titles

- **Omega: checked, applies (posting), not wired.** `Rocket_HitCraft` (`0x0139a6b0`) ends in
  the same loop: for every racer but the firer (`*(rocket + 0x78)`), `d = slot+0x50 - rocket+0x130`,
  `|d| < record+0x24`, `slot+0x120 += normalize(d) * (1 - |d| / record+0x24) * record+0x28`. The
  pending slot moved to `+0x120`. Its drain was not found (no reader of `slot+0x120` with a `0.5`
  add among `VUCOMISS`/`VMOVUPS` loads of `+0x120`). HD's posted law inherited turned
  `omega_event_xp_ground_truth::a_finished_omega_event_raises_the_stored_xp_by_the_authored_award`
  red ("no lap or place award paid"), green again with Omega's old immediate push, so Omega keeps
  the immediate push (chosen) until its drain is read.
- **2048: checked, applies (posting), not wired.** Its Rocket hit `FUN_81206a9e` (the only
  caller of `Rocket_SpawnCraftBlast` `0x81206928`) ends in `FUN_8120680e(this, P, firer)`: for
  every racer but the firer, `d = slot+0x48 - P`, `|d| < record+0x24`, a `(1 - |d| / record+0x24)
  * record+0x28` falloff along `d / |d|` written into `slot+0x110..+0x11f` through a
  multiply-accumulate (the decompile shows no load of the old value; whether it adds is not
  settled). Its drain is unread. HD's law inherited turned
  `sol_ground_truth::a_field_of_seven_laps_2048_sol_at_ace` red (a craft with no flying lap in
  12,000 ticks: flying laps `[1, 1, 2, 1, 0, 1, 1]`), green again without it, so 2048 keeps the
  immediate push (chosen).
- **Fury** shares HD's executable.

### Open from the push

- The point at slot `+0x50` (nose or centre) and a live run of the drain (above).
- Omega's and 2048's own drains, and why HD's law stalls their fields (a stun per blast is
  the likely cause: Omega's and 2048's tables or AI fire more blasts, not measured).
