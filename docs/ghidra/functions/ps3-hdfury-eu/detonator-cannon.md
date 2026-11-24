# Detonator: the pulse cannon, its hits, the score and the destroyed count

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`). Every function below declares TOC
`r2 = 0x008ad4d8` in its own OPD entry, so the TOC defect [memory.md](memory.md) describes does
not apply. Static reading only, 2026-11-22: nothing here ran. Bodies Ghidra truncates on AltiVec
words (`CannonManager_TestMineBombHits`, `PulseBullet_Update`, `PulseCannon_AwardStreakBonuses`)
were read off `llvm-objdump -d --mattr=+altivec,+vsx`; the Cell-only `lvlx` (opcode 31, xo 519),
which llvm prints as `<unknown>`, was decoded by hand. TOC constants come from `scripts/ps3-toc.py
resolve`. Read first: [detonator-mine.md](detonator-mine.md) and [detonator-bomb.md](detonator-bomb.md)
(what a mine and a bomb do; their weapon-table blocks), [cannon.md](cannon.md) (the ordinary
Cannon's round and muzzle), [weapons.md](weapons.md) ("The `0x334` member").

## The answer in one table

| Quantity | Value | Confidence |
| --- | --- | ---: |
| Weapon | the craft's fixed cannon, armed from the start, no pickup | 75 |
| Magazine ("clip") | `rounds` = 15 volleys; a volley is two rounds, one from each `cannon_flash` locator | 80 |
| Fire | while the fire button is held: one volley every `1/rate` = 0.1 s, the first on the first held tick | 78 |
| Round speed | `500` units/s; the launch tick only, `500 + 3.6 * craft speed` | 75 |
| Round life | 1.0 s, or its last wall | 72 |
| Track following | a probe 9.0 along the surface normal; on a floor hit the round sits 3.5 above it | 68 |
| Walls | `num_rebounds` = 1 reflection `v - 2(v.n)n`; the next wall kills it | 75 |
| Hit test | the tick's swept segment against a sphere of the target's own `trigger_radius`: mine 3, bomb 6 | 80 |
| Mine shot | one hit destroys it (any mine, live or dark), no damage to anyone | 80 |
| Bomb shot | `number_of_shots_to_destroy` = 4 hits; the fourth destroys it, harmlessly | 78 |
| Chain reaction | a bomb the cannon destroys sets off every untriggered mine within `sqrt(5000)` ~ 70.7 | 75 |
| Reload | empty and trigger released: refill to 15, cannon locked 0.75 s; absorb button: the same at any time | 72 |
| Score | `DetonatorScoring.xml`, below; points go to `RaceManager+0x2df0` | 80 |
| Destroyed count | `rm+0x6de3c` += 1 per mine the cannon hits directly or the EMP kills; chain-reaction mines and bombs are not counted | 80 |
| End percentage | `rm+0x2ddc = rm+0x6de3c / (sum of MineCountStage1..14) * 100`, f32 | 82 |

## Names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x00138700` | `PulseCannonManager_Construct` | 65 |
| `0x00138850` | `PulseCannonManager_Update` | 72 |
| `0x00138978` | `PulseCannonManager_FireVolley` | 75 |
| `0x00138820` | `PulseCannonManager_RequestReload` | 60 |
| `0x00147720` | `PulseCannon_AwardStreakBonuses` | 80 |
| `0x001479b8` | `PulseCannon_ResetStreak` | 70 |
| `0x001115e8` | `CannonManager_Update` | 78 |
| `0x0010ffd0` | `CannonManager_TestMineBombHits` | 80 |
| `0x0010f2e8` | `CannonManager_ArmSlot` | 65 |
| `0x001235a8` | `PulseBullet_Construct` | 85 |
| `0x00123378` | `PulseBullet_Fire` (was `PulseBullet_Init`, 55) | 75 |
| `0x00121d68` | `PulseBullet_FireFromMuzzles` | 65 |
| `0x00122608` | `PulseBullet_Update` | 75 |
| `0x00121988` | `PulseBullet_GetSegment` | 78 |
| `0x0002d170` | `WeaponStats_ParseCannon` | 85 |
| `0x000e9cd0` | `Ship_UpdateCannonTrigger` | 68 |
| `0x000d5750` | `Ship_UpdateCannonReload` | 70 |
| `0x00083030` | `DetonatorScoring_Load` | 85 |
| `0x00082e28` | `DetonatorScoring_ParseBullet` | 85 |
| `0x00082d18` | `DetonatorScoring_ParseEmp` | 85 |
| `0x00083238` | `DetonatorScoring_Construct` | 75 |
| `0x00082a60` | `DetonatorScoring_AddBulletMine` | 82 |
| `0x000828a8` | `DetonatorScoring_AddBulletBomb` | 80 |
| `0x00082c30` | `DetonatorScoring_AddChainMine` | 78 |
| `0x00082b48` | `DetonatorScoring_AddEmpMine` | 75 |
| `0x00082808` | `DetonatorScoring_AddEmpBomb` | 75 |
| `0x00082798` | `DetonatorScoring_AddFlat` | 70 |
| `0x00082978` | `DetonatorScoring_AddBulletBombHit` | 55 |
| `0x0007e6d0` | `Hud_RaiseChainBonus` | 72 |
| `0x00067120` | `Detonator_UpdateHud` | 65 |
| `0x000642a0` | `Detonator_IsBombAhead` | 55 |
| `0x00987620` | `g_DetonatorScoring` (data) | 80 |

**The manager is the pulse cannon's** (65): it is the `0x334`-byte class `WeaponManager_Construct`
builds in mode 14 only (both copies call `0x00138700`, at `0x0012cc7c` and `0x0012d744`); it
chains to `CannonManager`'s base constructor (`0x00111a30`), and that constructor, in mode 14, fills
the pool with thirty `0x1c0`-byte `PulseBullet` objects (`0x00111b3c`ff, `bl 0x1235a8`) instead of
sixty `CannonBullet`s and sixty `0x00130830` companions. `PulseBullet.cpp` (`0x00783940`) is the
round's own tag, the HUD names its ammunition `PulseBullets` (`0x0077ea48`), and 2048's manual
calls the weapon the pulse cannon. It has no `__FILE__` tag of its own, hence 65, not more.
`0x00138778` is the same body with no caller (left unnamed).

Its vtable is `0x00864270` (`0x00138700` writes it through TOC `-0x2c08`). Against
`CannonManager`'s `0x008639e8`, three slots differ: `+0x0c` update `0x00138850` (base
`0x001115e8`), `+0x38` fire `0x00138978` (base `CannonManager_FireRound`), `+0x3c`/`+0x44`
`0x00138820`/`0x001387f0` (base: no-ops). The extra `0xcc` bytes at `+0x268` are the clip record
the streak law reads (below).

`PulseBullet`'s vtable is `0x00863d98` (`0x001235a8`, TOC `-0x3480`): `+0x14` `0x00122098` (render
queue), `+0x1c` `0x00121e68` (draw), `+0x38` `PulseBullet_FireFromMuzzles`, `+0x3c`
`PulseBullet_Fire`, `+0x40` `0x00123370` (`b 0x00122608`, `PulseBullet_Update`), `+0x44`
`PulseBullet_GetSegment`. This is why [projectile-launch-normal.md](projectile-launch-normal.md)
found `0x00123378` as "a third bullet class's init": it is the pulse round's fire.

## The Cannon block of the weapon table

`WeaponStats_ParseCannon` (`0x0002d170`, called from the table loader at `0x00030674` with the
table base) stores, for `weaponstats_detonator.xml`'s `<Weapon type="Cannon">`:

| Attribute | Field | Store | Disc | Reader |
| --- | --- | --- | ---: | --- |
| `rounds` | `+0x84` | int | `15` | `CannonManager_ArmSlot`, `Ship_UpdateCannonReload`, the manual reload |
| `absorb` | `+0x88` | float | `20` | not read here |
| `rate` | `+0x8c` | float, **stored as `1.0 / rate`** | `10` -> `0.1` | `Ship_UpdateCannonTrigger` (`0x000e9fac`) |
| `damage_per_bullet` | `+0x90` | float | `1` | the craft-hit path (`0x0010f410`, `0x00110e50`), not Detonator's |
| `slowdown_time` | `+0x94` | float | `0.1` | the same |
| `round_recharge_time` | `+0x98` | float | `0.1` | **none found** |
| `recharge_pause` | `+0x9c` | float | `1.0` | **none found** |
| `num_rebounds` | `+0xa0` | int | `1` | `SPDetonator`'s constructors (`0x00064808`, `0x00064d88`), the EMP fire (`0x000ea040`): copied to the pickup slot's `+0x17c` |

"None found": every function that loads the table global (`0x009389a4`, 119 of them) was scanned
for a load at a displacement off the register an `lwzx` from the table array produced; the scan
finds `+0x84`, `+0x8c`, `+0xa0`, `+0x108` (`number_of_shots_to_destroy`) and the Mine's `+0x130`,
and nothing at `+0x98`, `+0x9c`, the Bomb's `+0x10c`/`+0x110`/`+0x114`
(`energy_recharge_per_shot`, `points_for_cannon_bullet_hit`, `points_for_destroying`), the Mine's
`+0x134`/`+0x138` (`points_per_metre`, `max_distance`) or the `Global` block's
`energy_recharge_per_stage` (`+0x04`, `0x0002b8e8`). Confidence **60** that all of these are
parsed and never read. The gauge the manual calls the reload is the clip and its lockout below,
not a per-round recharge, and the score values that are read live in `DetonatorScoring.xml`.

## Fire: the trigger, the clip and the volley

The per-tick pickup step `0x000eabd0` calls `0x000ea0c8`, `Ship_UpdateCannonTrigger`
(`0x000e9cd0`), `Ship_UpdatePickup` (`0x000e9160`), then tail-branches to
`Ship_UpdateCannonReload` (`0x000d5750`, `0x000ead84`). `s` is the craft's pickup slot
(`*(ship+0x5edc)`), `in` the input record `*(*(ship+0x5fac)+0x84)`.

```text
Ship_UpdateCannonTrigger (person craft, +0x5fa4 == 1):
    [mode 14: the EMP fire on in+0x1c, the gauge rm+0x2dd8 against table+0x1bc; EMP's, unread here]
    if !in+0x1a (fire): s+0x168 = 0; return                      ; 0x000e9d78-0x000e9d88, 0x000e9f00
    if s+0x204 != 3 (Cannon): return
    if s+0x164 (rounds) > 0:
        if !s+0x175 (ready): return
        s+0x168 -= dt                                            ; 0x000e9db8-0x000e9dcc
        mode 14: rm+0x6de44 += 1                                 ; 0x000ea098-0x000ea0b4
        if !(s+0x168 > 0.0):                                     ; 0x000e9de8-0x000e9df4
            s+0x164 -= 1; s+0x200 |= 0x4000 (fire request); s+0x168 += table+0x8c (0.1)
        (other modes: a clip at 0 drops the weapon; Detonator keeps it)
```

The volley itself is `PulseCannonManager_FireVolley` (`0x00138978`, vtable `+0x38`), reached for a
slot whose `+0x4000` request is set (the dispatcher was not read; `CannonManager_FireRound`, which
the volley calls first, clears the bit and in mode 14 returns before firing anything,
`0x0010fbd0`):

```text
if live (+0x148) < pool (+0x264, 30) - 1:                        ; 0x001389c8-0x001389d8
    a, b := pool[live], pool[live + 1]; live += 2
    k := clip+0x04 (+0x26c); a.slot = &clip.hits[k]; b.slot = &clip.hits[(k+1) % 30]
    clip.hits[k] += 1; clip.hits[(k+1) % 30] += 1; k = (k + 2) % 30   ; 0x001389ec-0x00138ad4
    vtable +0x44 -> 0x001476b0 (a volley counter, capped 15; read by nothing below)
    speed := ship+0x6944 -> +0x4c4 * 3.6 (TOC -0x2bfc)            ; 0x001389e8, 0x00138a60
    a: FireFromMuzzles(speed, left cannon_flash ship+0x5f38, ship muzzle, side 1, ..., s+0x17c)
    b: FireFromMuzzles(speed, right cannon_flash ship+0x5f3c, ship muzzle, side 0, ..., s+0x17c)
```

So **a volley is two rounds, one from each wing**, and the clip of 15 volleys is the whole
thirty-round pool. `rounds` counts volleys, not rounds. The volley skips silently when 29 or 30
rounds are still in flight (with a 1 s life and one volley per 0.1 s this cannot happen at the
disc's values). Confidence **78** for the trigger, **75** for the volley.

`rm+0x6de44` counts the ticks the trigger was held with a loaded, ready cannon: it is the
"has fired" record the *Hold Your Fire* trophy reads (below).

## The round: `PulseBullet_Fire` and `PulseBullet_Update`

`PulseBullet_FireFromMuzzles` (`0x00121d68`) stores the slot's `num_rebounds` at `+0x12c`
(`0x00121d94`), the craft at `+0x128`, the two muzzle nodes (`CannonBullet_SetMuzzles`), builds
the muzzle matrix (`0x00121ba0`) and calls `PulseBullet_Fire` (`0x00123378`):

```text
age (+0x4c) = 0; rows +0x140..+0x170 = muzzle matrix (translation +0x170 = muzzle position)
prev (+0xe0) = muzzle position; surface normal (+0x180) = -(craft+0x7850)
launch speed (+0x124) = speed + 500.0 (block 0x008c1870 +0x00)   ; 0x00123428
velocity (+0xd0) = forward row (+0x160) * launch speed; flags (+0x40) |= 8
sound "CANNON" (TOC -0x33f8) at the craft's emitter (ship+0x5ee0), volume 1.0
```

`PulseBullet_Update(round, dt, world, visible)` (`0x00122608`; `CannonManager_Update` passes the
collision world and `1`):

```text
age += dt; prev := current (+0x170)                              ; 0x00122694-0x001226d0
next := prev + velocity * dt
kind := Collision_SweepSegment(world, next, next - normal * 9.0)  ; 9.0 at TOC -0x3448
if kind not in {4, 5, 127}: normal := hit normal; next := hit + normal * 3.5   ; block +0x2c
current := next                                                  ; 0x00122830
forward := normalize(velocity), made perpendicular to the up row (+0x150), right = up x forward
velocity := forward * 500.0                                      ; block +0x00, 0x00122a8c-0x00122ad0
kind := Collision_SweepSegment(world, prev, next)                ; 0x00122b00
if kind in {4, 5} (a wall):
    if +0x12c (rebounds left) > 0:                               ; 0x00122b14, 0x00122fb0
        +0x12c -= 1
        velocity := velocity - 2 (velocity . n) n                 ; -2.0 at TOC -0x3408
        next := hit + n * 3.0                                    ; 3.0 at 0x008aa0a4
    else:
        flags |= 0x14 (remove, wall hit)                          ; 0x00122b48-0x00122b88
        if visible: spawn WO_CANNON_SPARKS_DETONATOR (mode 14) or WO_CANNON_SPARKS, fourcc
                    'BDOR' (0x42444f52), at a matrix built on the hit   ; 0x001232b8, 0x00122e10
```

`kind` is `Collision_SweepSegment`'s return ([own-craft-box.md](own-craft-box.md)): a mesh's
surface code, `0x7f` for nothing; [plasma.md](plasma.md) reads `4`/`5` as walls on HD. So the
round **skims the track at 3.5 above whatever floor the 9.0 probe finds, flies straight on where
there is none (no gravity), bounces once off a wall and dies on the second**. `CannonManager_Update`
retires it when `age > 1.0` (`0x008c1834`, a constant no loader writes) or `flags & 4`, and plays
`CANNONEXPLWALL` (volume 0.6) for a wall death (`flags & 0x10`; `CANNONEXPLSHIP`, 0.4, for `0x20`,
a craft hit) at the round (`0x00111764`-`0x0011181c`).

The speed is **500 units/s from the first update on**: the update rebuilds the velocity from the
constant block, not from `+0x124`, so the craft's speed only adds to the launch tick's step. A
quirk, read as written (`500 + 3.6 * v` mixes a km/h figure into a units/s one). Confidence
**75** for the speed and the wall law, **68** for the probe and ride, **60** for the
re-orthogonalisation against `+0x150` (no writer of `+0x150` was found in the update; the up row
looks like the launch matrix's, kept).

`PulseBullet_GetSegment` (`0x00121988`, vtable `+0x44`) returns `current` (`+0x170`) and `prev`
(`+0xe0`): the segment the hit test sweeps.

## The hit test: `CannonManager_TestMineBombHits` (`0x0010ffd0`)

`CannonManager_Update` (`0x001115e8`) runs, for every round in flight: its update, the craft test
`0x00110e50`, then this test. It is the ordinary Cannon's too (mines and bombs can be shot in any
mode); the Detonator arms are gated on byte `0x009384e1 == 0` and mode `14`.

```text
A, B := round.current, round.prev; d := A - B
if d != 0: u := normalize(d)                                     ; 0x00110088-0x00110128
for every mine m in MineManager (+0x78, count +0x398) with !(m.flags & 0x80):
    r := table.Mine.trigger_radius (+0x130, 3.0); P := BaseMine_GetPosition(m)
    hit := |P - A|^2 <= r^2                                       ; 0x001102c8-0x001102e4
        or (u.(P - A) < 0 and u.(P - B) > 0 and |perp(P)| < r)    ; 0x001102e8-0x001103cc
    if hit:
        round.flags |= 4; m.flags |= 0x80                         ; 0x00110428-0x00110438
        mode 14: rm+0x6de3c += 1; *round.slot += 1                ; 0x00110488-0x001104a8
                 DetonatorScoring_AddBulletMine(g_DetonatorScoring, m.isLive, m.pos)
for every bomb b in BombManager (+0x58, count +0x98) with !(b.flags & 0x80):
    the same test with r := table.Bomb.trigger_radius (+0x100, 6.0)
    if hit:
        if b.hits (+0x60) < number_of_shots_to_destroy (+0x108, 4):
            b.hits += 1; mode 14: *round.slot += 1
        if b.hits >= 4:                                           ; 0x00110690
            b.flags = (b.flags & ~1) | 0x80
            mode 14: rm+0x6de40 += 1; *round.slot += 1
                     DetonatorScoring_AddBulletBomb(g_DetonatorScoring, b.pos)
                     g_DetonatorScoring+0x34 = 1.0
                     for every mine m with !(m.flags & 0x80):    ; 0x00110a90-0x00110b94
                         if |m.pos - b.pos|^2 < float(*0x008c14d0):   ; 5000
                             m.flags |= 0x80
                             DetonatorScoring_AddChainMine(g_DetonatorScoring, m.isLive, m.pos)
        round.flags |= 4
```

`perp(P)` is the distance from `P` to the line, built from two cross products with `u`
(`0x00110318`-`0x001103b8`); with the endpoint test it makes the test a capsule round the tick's
segment (the `B` end was the previous tick's `A`). `m.isLive` is the mine's vtable `+0x40`,
`DetonatorMine_IsLive`. Confidence **80** for the whole test.

What this means, with [detonator-mine.md](detonator-mine.md) and [detonator-bomb.md](detonator-bomb.md):

- **A shot mine goes in one hit, live or dark, and harms nobody.** The cannon only sets `0x80`; it
  neither darkens the mine nor calls `MineManager_ApplyBlast`. On the next `MineManager_Update`
  `BaseMine_Update` explodes it first (`WO_MINE_EXPLO_DETONATOR` if laid this stage,
  `WO_MINE_EXPLO_DETONATOR_DEAD` if dark, `MINEEXPL`) and clears the armed bit before the contact
  test runs. Confidence **75** (the order of the two managers in a tick is unread; a contact test
  on the same tick as the shot is the one window left).
- **A bomb takes four hits.** Hits one to three only raise `b+0x60` (the bomb's update flashes on
  a change, `DetonatorBomb_Update`); the fourth clears the active bit, so `BombManager_TestCraftContacts`
  never runs on it again, and sets `0x80`: it detonates on its next update with the full blast
  visual (`hits >= 4`, `DetonatorBombBlast_Start`) and **no damage or push** (only the contact
  path calls `BombManager_ApplyHit`). Confidence **78**.
- **The chain reaction** sets off every untriggered mine whose squared distance to the bomb is
  under `5000`, the int at `g_DetonatorGamePlayValues+0x58` (`0x008c14d0`, ELF default `0x1388`,
  converted with `fcfid` and compared with `fcmpu`, `0x00110ad4`-`0x00110b2c`). **No attribute
  writes `+0x58`**: `Detonator_LoadGamePlayValues` parses `BombChainReactionDistance` (disc
  `5000`) into a stack local it drops ([detonator-placement.md](detonator-placement.md)); the
  default and the disc agree, so the radius is `sqrt(5000) = 70.71`. Chain mines are scored but
  **not counted in `rm+0x6de3c`** and add nothing to the clip's hits. Confidence **75**.
- `PointsForChainReactionMines` (`g_DetonatorGamePlayValues+0x10`) has no reader: no function
  that loads the block reads `+0x10` (the readers' displacements were listed). Confidence 60.

## The score: `DetonatorScoring.xml`

`DetonatorScoring_Load` (`0x00083030`), from `SPDetonator`'s constructors through
`DetonatorScoring_Construct` (`0x00083238`, `0x000645ac`/`0x00064b2c`), reads
`Data\XML\DetonatorScoring.xml` (TOC `-0x5f34`) into `g_DetonatorScoring` (`*0x00987620`):

| Element / attribute | Field | Disc (HD `DATA00` `/data/xml/detonatorscoring.xml`) |
| --- | --- | ---: |
| `Bullet multiplier` | `+0x00` float | `1.0` |
| `Bullet new_mine_score` | `+0x04` int | `100` |
| `Bullet old_mine_score` | `+0x08` int | `50` |
| `Bullet bomb_score` | `+0x0c` int | `2000` |
| `Bullet bomb_hit_score` | `+0x10` int | `0` |
| `Bullet bomb_increment` | `+0x14` int | `500` |
| `Emp new_mine_score` | `+0x18` int | `50` |
| `Emp old_mine_score` | `+0x1c` int | `25` |
| `Emp bomb_score` | `+0x20` int | `1000` |
| (counter shown as `%04d`, no writer but resets) | `+0x24` int | `0` |
| live-mine multiplier `m_mine` | `+0x28` float | `1.0` |
| bomb-hit multiplier | `+0x2c` float | `1.0` |
| EMP live-mine multiplier | `+0x30` float | `1.0` |
| chain-mine multiplier `m_chain` | `+0x34` float | `1.0` |
| bombs destroyed by the cannon `n` | `+0x38` int | `0` |

Every add goes to `+0x14` of the record the race manager's vtable `+0x3c` returns (called with `0`); `SPDetonator`'s `+0x3c` (`0x0067dc10`) is
`return rm + 0x2ddc`, so **the Detonator score is the int `rm+0x2df0`**, the value the HUD shows
(`Detonator_UpdateHud` stores it at the HUD record's `+0x2c`) and the medals compare. Each add but
the flat one also raises `Hud_RaiseScoreFeedback` (`0x00083ba0`) with the amount.

| Event | Points | Then | Evidence | Confidence |
| --- | --- | --- | --- | ---: |
| Cannon hits a live mine | `trunc(float(new_mine_score) * m_mine)` | `m_mine += multiplier` | `0x00082a80`-`0x00082ab4` (`fmuls`, `fctidz`) | 82 |
| Cannon hits a dark mine | `old_mine_score` (50) | - | `0x00082b40` | 82 |
| Cannon destroys a bomb | `bomb_score + n * bomb_increment` (2000, 2500, 3000, ...) | `n += 1`, `+0x2c = 1.0` | `0x000828b8`-`0x000828e0` | 82 |
| Chain-reaction live mine | `trunc(float(new_mine_score) * m_chain)` | `m_chain += multiplier`; `m_chain = 1.0` at each cannon bomb kill | `0x00082c50`-`0x00082c84` | 78 |
| Chain-reaction dark mine | `old_mine_score` (50) | - | `0x00082d10` | 78 |
| EMP kills a live mine | `trunc(float(Emp new_mine_score) * m_emp)` | `m_emp += multiplier` | `0x00082b68`-`0x00082b9c` | 75 |
| EMP kills a dark mine | `Emp old_mine_score` (25) | - | `0x00082c28` | 75 |
| EMP kills a bomb | `Emp bomb_score` (1000) | - | `0x00082820` | 75 |
| Streak bonus | 1000 / 3000 / 5000 | below | `0x0007e6d0` | 78 |
| Bomb hit, not destroyed | nothing | `DetonatorScoring_AddBulletBombHit` (`bomb_hit_score * +0x2c * k`) has no caller | `0x00082978`, raw `bl` scan | 60 |

So with the disc's `multiplier = 1.0`, **the live mines of one clip score 100, 200, 300, ...**:
`m_mine` counts live mines shot since the last reload, and resets to `1.0` at the reload (below),
not on a miss. `n` and `m_emp` reset only with the race. The HUD function (`0x000803d8`) formats `m_mine` as
`x%02d` (`fctiwz`) and `+0x24` as `%04d`, but never has a widget to put them in (corrected on
[detonator-hud.md](detonator-hud.md): `PulseScore` and `PulseMultiplier` are unbound on HD). All arithmetic is f32 then truncation
toward zero (`fctidz`), integers exact at these sizes.

`MineManager_DestroyAtLane` (`0x0011a590`) and `BombManager_DestroyAtLane` (`0x0012e260`) are the
EMP adds' callers ([detonator-mine.md](detonator-mine.md), [detonator-bomb.md](detonator-bomb.md)).

## The streak bonus: `PulseCannon_AwardStreakBonuses` (`0x00147720`)

`PulseCannonManager_Update` (`0x00138850`) runs `CannonManager_Update`, the HUD's bullet icons
(`0x0007e950`), then this, every tick. The clip record at `mgr+0x268` holds `hits[30]` at `+0x08`
(`mgr+0x270`): each round's slot is `1` once fired and `2` or more once it hit something (a mine,
a bomb hit or a bomb kill). Volley `i` (`0..14`) is `hits[2i]`, `hits[2i+1]`.

```text
streak := 0
for i in 0..14:                                                  ; 0x00147810-0x00147808, r29 = 2i
    if hits[2i] > 1 or hits[2i+1] > 1:                           ; 0x00147818-0x00147824, 0x00147908
        streak += 1; at := i
        if streak >= 15 and !clip+0xbf: clip+0xbf = 1; Hud_RaiseChainBonus(1, 5000, at)
            announcer "BONUS_15"; trophy 39                      ; 0x00147958-0x001479b4
        elif streak >= 10 and !clip+0xbe: clip+0xbe = 1; Hud_RaiseChainBonus(2, 3000, at)
            announcer "BONUS_10"                                 ; 0x00147874-0x001478bc
        elif 5 <= streak <= 9:
            if !clip+0xbc: clip+0xbc = 1; clip+0xc0 = at; Hud_RaiseChainBonus(3, 1000, at); "BONUS_5"
            elif !clip+0xbd: clip+0xc4 = at; clip+0xbd = (at - clip+0xc0 > 5); if so, the same 1000
    else: streak := 0
```

A streak is **consecutive volleys of one clip in which at least one of the two rounds hit**.
Five pay 1000 (and a second five pays 1000 again when it ends more than five volleys after the
first award), ten pay 3000 once, all fifteen 5000 once. `Hud_RaiseChainBonus` (`0x0007e6d0`) adds
the amount through `DetonatorScoring_AddFlat` (no score feedback), shows `HUD_CHAIN_5`/`_10`/`_15`
with the amount, recolours the streak's bullet icons and counts the award in
`rm+0x2de8`/`+0x2dea`/`+0x2dec` (u16: fives, tens, fifteens). Confidence **80**.

**The reset**: `PulseCannonManager_RequestReload` (`0x00138820`, vtable `+0x3c`, called by both
reloads) sets `clip+0xc8 = 1` (`mgr+0x330`) and zeroes the clip's counters (`0x00147a70`). On the
first tick after it that the cannon is ready (`s+0x175`), `PulseCannonManager_Update` clears the
flag, zeroes `hits[]` and the award latches (`PulseCannon_ResetStreak`, `0x001479b8`), resets the
icons (`0x0007e498`) and sets `g_DetonatorScoring+0x24 = 0`, `+0x2c = 1.0`, `+0x28 = 1.0`
(`0x0013893c`-`0x00138948`). The ring index `k` (`clip+0x04`) is **not** reset: it stays even,
so volley pairs stay aligned, but after a manual reload mid-clip the next clip's volleys fill
pairs from `k/2` upward and wrap, and the scan, which runs from pair 0, splits a streak that
crosses the wrap. A shipped quirk, read, not observed (confidence 60).

## The reload

`Ship_UpdateCannonReload` (`0x000d5750`), mode 14 only:

```text
if s+0x164 == 0:
    if in+0x1a (fire held): s+0x194 = 0; start the "~CANNON_DRYFIRE" loop      ; 0x000d58ac-0x000d595c
    else: s+0x194 = 1; stop it
if s+0x164 == 0 and s+0x194:                      ; empty and the trigger released
    s+0x175 = 0 (not ready); s+0x178 = s+0x198 = CannonLockoutTimeRunOutBullets (0x008c183c)
    s+0x164 = table.rounds (15)
    PulseCannonManager_RequestReload(mgr); HUD "CANNONRELOAD" (0x0007eef8, lockout, auto)
```

The manual reload is in `Ship_UpdatePickup` (`0x000e9160`, `0x000e9204`-`0x000e9238`, then
`0x000e9af0`): a person craft in mode 14 with the cannon ready and the input's `+0x1b` (the absorb
button) set gets the same refill with `CannonLockoutTimeFromManualReload` (`0x008c1838`) and the
HUD call with `manual = 1`. It does not test the clip, so a full clip can be reloaded (and its
streak and `m_mine` reset). Whether `+0x1b` is a press edge or a level is unread.

`CannonManager_Update` counts the lockout down in mode 14 (its first block, for the player's slot
`mgr+0x14c`; `0.0` at TOC `-0x39e8`): `s+0x178 > 0`
-> `s+0x175 = 0`, `s+0x178 -= dt`; else `s+0x178 = 0`, `s+0x175 = 1`. **Both lockouts are 0.75 s
on the disc** (`GamePlayValues.xml`; ELF defaults 0.5 manual, 1.0 empty). While locked, the trigger
does nothing (`Ship_UpdateCannonTrigger` returns on `!s+0x175`). The HUD's reload gauge
(`Detonator_UpdateHud`, its first block; `0.0`, `1.0`, `15.0` at TOC `-0x68e8`, `-0x68ec`,
`-0x67c4`) is `(1 - s+0x178 / s+0x198) * 15` while
`s+0x178 > 0`, else the clip count. Confidence **72**.

`CannonManager_ArmSlot` (`0x0010f2e8`) sets the slot to Cannon (`+0x204 = 3`), ready, timers 0 and
`rounds = table.rounds`; both of its Detonator callers (the constructors, `0x000647e4`, and the
EMP fire, `0x000ea024`) save and restore `s+0x164` around it, so in Detonator it arms without
refilling. What `s+0x164` holds when the race is built is not read; if it is `0`, the first
`Ship_UpdateCannonReload` reloads at once, which reads as the cannon loading during the
countdown (hypothesis, 50).

## The destroyed count and the end of the run

- `rm+0x6de3c` (+1 per mine destroyed by a cannon round or the EMP; not chain-reaction mines, not
  bombs) and `rm+0x6de40` (+1 per bomb destroyed by the cannon or the EMP) are the only writers
  found. Confidence **80**.
- At the end (`Detonator_UpdateRace`, `0x00067d88`-`0x00067f68`, on stage `>= 15` or the
  out-of-race bit): `rm+0x2de0 = rm+0x6de40` and

  ```text
  rm+0x2ddc = float(rm+0x6de3c) / float(sum of g_DetonatorGamePlayValues+0x14..+0x48) * 100.0
  ```

  `fdivs` then `fmuls` by `100.0` (TOC `-0x6830`, `0x00067f5c`-`0x00067f68`). The denominator is
  the fourteen `MineCountStage` values, `80 + 90 + ... + 200 = 1860` on the disc, whatever stage
  the run reached; each is an exact float, so the sum's order does not matter. This is the
  result screen's `PERCENT_MINES_CLEARED`. Confidence **82**.
- So `rm+0x2ddc` is a record: `+0x00` percent (float), `+0x04` bombs, `+0x0c..+0x10` the three
  streak counts, `+0x12` the HUD's stage (u16 `rm+0x2dee`), `+0x14` the score.

## The top block: a bomb warning, the HUD and three trophies

`Detonator_UpdateRace` runs its top block only while the player's speed
(`ship+0x6944 -> +0x4c4`) is over **10.0** (`0x008a6d2c`, `0x00067b54`):

- `Detonator_UpdateHud` (`0x00067120`) is **not a score add**: the reload gauge above; on each line
  crossing a second stage counter `rm+0x2dee` (u16, capped 14) with its announcer list
  `rm+0x2e20`; the medal thresholds of the campaign cell against `rm+0x2df0`
  (`DetonatorGoldMedal`/`Silver`/`Bronze`, announcer cues, `gold_med`..`bronze_med` icons); and
  `rm+0x2df0 += 1000` while the craft's `+0x6a64 & 0x200000` is set. That bit is set by
  `Craft_UpdateLapProgress` on a crossing (`0x000e5238`) when the craft's `+0x6a64 & 0x20` and
  `+0x7002` are clear: a clean-lap bonus, hypothesis 50 (what `0x20` records and how long `0x200000`
  stays set are unread; more than one tick would pay more than 1000).
- `Detonator_IsBombAhead` (`0x000642a0`, `f1 = 500.0`): for each live bomb, a track measure from
  the craft (`0x000d2a30`, unread) in `[0, 500]` and a straight-line distance not above it return
  `0x000588f0(rm)` (unread). When it returns non-zero, once per stage (latch `rm+0x6de63`, cleared
  at the stage step) the HUD raises **message 8** (`0x00083318(hud, 8, r)`, `r` the return of the call before it, not the craft (corrected on [detonator-hud.md](detonator-hud.md)), which files it in
  a warning slot with `2.0` in the slot's timer) and the announcer says `"bomb"` (TOC `-0x67c0`). So message 8 is a **bomb-ahead warning,
  not a hit**. Confidence 55.
- **Trophies**, through `0x00676f08` (the trophy unlock: its ids match `TROPHY.TRP`, `NPWR00187_00`):

  | Id | Trophy (the disc's text) | Condition read | Site |
  | ---: | --- | --- | --- |
  | 38 | *Hold Your Fire*: "Reach stage 10 without firing" | `rm+0x2dee >= 10` (`0x00769a24`) and `rm+0x6de44 == 0` | `0x000674d8` |
  | 39 | *Sharpshooter*: "Score a hit with every round in an entire cannon clip" | the 15-streak (one hit per volley suffices) | `0x001479a8` |
  | 40 | *Da Bomb*: "Destroy all 14 bombs" | `rm+0x6de40 >= 14` (`0x00769a44`) | `0x00067428` |

  The trophy texts confirm the clip of 15 volleys, the bomb counter and the trigger counter.
  Confidence **80**.

## Effects and sounds

| Effect or cue | Fires when | Confidence |
| --- | --- | ---: |
| `"CANNON"` | each round's fire (two per volley), at the craft's emitter | 75 |
| `~CANNON_DRYFIRE` (loop) | fire held with an empty clip, until released | 70 |
| `CANNONRELOAD` (HUD) | either reload; the gauge shows the lockout | 72 |
| `WO_CANNON_SPARKS_DETONATOR`, fourcc `BDOR` | a round dies on a wall (no rebound left), in view, mode 14 (else `WO_CANNON_SPARKS`) | 72 |
| `CANNONEXPLWALL` (0.6) | the same wall death, at the round | 70 |
| mine / bomb explosions | the object's own, as on [detonator-mine.md](detonator-mine.md), [detonator-bomb.md](detonator-bomb.md); the cannon spawns nothing on a hit | 75 |
| `BONUS_5` / `_10` / `_15`, `HUD_CHAIN_*` | the streak awards | 80 |
| `"bomb"`, HUD message 8 | a bomb ahead, once per stage | 55 |

The round's model and streak are presentation: `Data\Weapons\detonator_cannonbolt.vex`/`.rcsmodel`
and `hd_muzzleflash_detonator` `.vex`/`.rcsmodel` (`PulseBullet_Construct`, TOC `-0x33f0`..`-0x33e4`), drawn through
`0x00122098`/`0x00121e68` (unread beyond the queue).

## What the simulation needs

| Quantity | Value |
| --- | --- |
| Clip | 15 volleys; volley = 2 rounds (left and right `cannon_flash`) |
| Trigger | held: `t -= dt`; `if !(t > 0)`: fire, clip -= 1, `t += 0.1`; released: `t = 0`; locked or empty: nothing |
| Round | life 1.0 s; launch speed `500 + 3.6 v`, then 500; probe `9.0` down the surface normal, ride `3.5`; one wall reflection, then gone |
| Hit | capsule of the tick's segment, radius 3 (mine) / 6 (bomb); untriggered objects only; mines tested first, then bombs; the loops do not stop at a hit (below) |
| Mine | destroyed by one hit; no damage |
| Bomb | 4 hits; the fourth destroys it harmlessly and sets off mines within `sqrt(5000)` |
| Reload | empty + released, or absorb: refill 15, locked 0.75 s, streak and `m_mine` reset |
| Score | live mine `100 * m_mine`, `m_mine += 1` (per clip); dark mine 50; bomb `2000 + 500 n`; chain live `100 * m_chain` (reset per bomb), dark 50; streaks 1000 / 3000 / 5000 |
| Count | mines hit directly (and EMP kills) -> destroyed; bombs -> bombs |
| Percentage | `destroyed / 1860 * 100` (f32) at the run's end |

**One round can hit several objects in the tick it hits**: neither loop leaves at a hit (the mine
arm returns to `0x00110220`, the next mine) and the bomb loop follows the mine loop; the round's
`flags & 4` is acted on only by `CannonManager_Update`'s retire pass afterwards. So every
untriggered mine and bomb within the capsule that tick is hit, counted and scored, and each
adds to the volley's slot. Read, not observed; confidence 65.

## Open

- The dispatcher that calls `PulseCannonManager_FireVolley` for a set `+0x4000` request, and the
  order of the weapon managers and the craft update within a tick.
- `0x000d2a30` and `0x000588f0` (the bomb-ahead measure), what sets `+0x6a64 & 0x20` and clears
  `0x200000`, `+0x24`'s writer (the HUD's `%04d`).
- `round_recharge_time`, `recharge_pause`, `energy_recharge_per_shot`, `points_for_cannon_bullet_hit`,
  `points_for_destroying`, `points_per_metre`, `max_distance`, the weapon table's
  `energy_recharge_per_stage`, `PointsForChainReactionMines`: parsed, no reader found (60).
- The round's up row and the draw (`0x00121e68`).
- The EMP itself (`Ship_UpdateCannonTrigger`'s `in+0x1c` arm, `EMPManager`): read on
  [detonator-emp.md](detonator-emp.md).

## The buttons: fire is SQUARE, reload is CIRCLE, the EMP is L1 (2026-11-22, `det-look-re`)

`Ship_UpdateCannonTrigger` reads three bytes of the controls record `*(*(ship+0x5fac)+0x84)`: `+0x1a`
fire (hold), `+0x1b` the absorb byte (the manual reload, `Ship_UpdatePickup`), `+0x1c` the EMP. The record is
`PlayerInput+0x4c`, so `+0x1a` is the *use* hold `PlayerInput+0x66`, `+0x1b` absorb `+0x67`, `+0x1c` action 8
([eliminator-flip.md](eliminator-flip.md), "The AI's flip"): the cannon is the ordinary pickup **use** button
and the reload the ordinary **absorb** button, the same two as every other weapon. The question is which
physical buttons the bindings `Options_GetButtonBinding(pad, action)` return for use (`7`) and absorb (`4`) are.

`Pad_Update` (`0x0033e460`, `Pad_Update` in the bridge) turns a DualShock 3 record (`*record == 5`) into the
per-index booleans at `pad+0x674+index`. Reading the decompile, each index takes one float of the record
(`P = record + 4`):

| Index | Source | Button | Evidence |
| ---: | --- | --- | --- |
| 10, 11 | `P+0x2c`, `P+0x30` | L2, R2 | airbrake bindings (`eliminator-flip.md`) |
| 8, 9 | `P+0x34`, `P+0x38` | L1, R1 | L1 = Flip, live; R1 = reverse camera |
| 5 | `P+0x3c` | TRIANGLE | next in the run |
| 4 | `P+0x40` | CIRCLE | next in the run |
| 6 | `P+0x44` | CROSS | thrust, live |
| 7 | `P+0x48` | SQUARE | last in the run |

The run `P+0x2c..P+0x48` is the DualShock digital-2 order (L2, R2, L1, R1, TRIANGLE, CIRCLE, CROSS, SQUARE), and
the four indices live-confirmed by other pages (L2/R2 airbrakes, L1 flip, CROSS thrust) sit on it in that
order; the remaining three are read by position. **So use (`7`) is SQUARE, absorb (`4`) is CIRCLE, change camera
(`5`) is TRIANGLE.** The disc's `controls_definition_ps3.xml` rows (`O`, `SQ`, `TR`) are not those buttons for
use and absorb: [eliminator-flip.md](eliminator-flip.md) already calls its airbrake row's `L1` a placeholder, and
the labels there are rotated by one. Confidence **80**: the decompile and the live bindings, with the
`P+0x3c`/`+0x40`/`+0x48` sources unread (the function that fills `P` from `cellPadGetData`'s bytes was not
located, `FUN_005b9570` only calls `cellPadGetData`).

**The pairs lane's "TRIANGLE fires, SQUARE changes the camera" is the harness, not the game.** The pad driver
sends `triangle` as `BTN_NORTH` and `square` as `BTN_WEST` (`scripts/rpcs3_pad.py`), and RPCS3's evdev default
binds Square to `BTN_X` (`BTN_NORTH`, 0x133) and Triangle to `BTN_Y` (`BTN_WEST`, 0x134); the two face buttons
reach the game swapped. A harness `triangle` is a game SQUARE (use, fires the cannon, 0.2 s hold took the clip
15 -> 12), a harness `square` is a game TRIANGLE (change camera, the view changed and the hull vanished).
The same swap is behind every HD page that fires a weapon "with TRIANGLE" (`hd-pickup-table.md`, `shield.md`,
`craft-flags.md`, `leach-beam-strips.md`): it is the use button. The RPCS3 default is from RPCS3's source as
remembered, not read in this tree (confidence 70 for the swap as the mechanism; the live result and the static
reading agree independently, which is what puts the mapping at 80).

**Ours is right.** `oag_raceplay::detonator` fires on `Button::Square` held and reloads on `Button::Circle`
pressed, which are the ordinary HD use and absorb of `weapons.rs` (`Button::Square`, `Button::Circle`), and the EMP
on `Button::Flip` (L1), which the original's controls `+0x1c` reads (live, [detonator-emp.md](detonator-emp.md)).
No change; only the label "chosen" on the cannon's mapping becomes "read, 80". The reload is a **press edge** in
ours (chosen): the level-or-edge question of `+0x1b` stays open. A pair script that drives ours' cannon
with SQUARE and the original's with the harness's `triangle` is correct as it is.

**Checked against 2048 and Omega.** Omega's `PlayerInput_Update` (`FUN_012e8fb0`) is the same class with the same
record layout ([player-input.md](../ps4-omega-eu/player-input.md)); its pad layer maps its own indices, so the
physical buttons are **not checked**: the index-to-button run above is HD's `Pad_Update`. 2048's
`PlayerInput_Update` (`0x811b3072`) is the same shape ([eliminator-flip.md](eliminator-flip.md)); its pad
mapping is **not checked**, and ours uses one mapping for every title.

## Wired (2026-11-22)

`oag_weapons::detonator::cannon::step_cannon` runs the trigger, both reloads, the lockout, the
streak reset, the volley, each round (update, hit test, retire) and the streak scan, on HD, with
`DetonatorScoring.xml` read through `DetonatorLayouts::scoring`. Chosen where the page says unread:
the order inside a tick (above), a full ready clip at the start, a press edge for the reload,
the squared chain distance
`5000` from the executable's static (the file's `BombChainReactionDistance` is parsed and dropped),
no firing before the green light. Not wired: the `0.5 s` owner grace, the `+0x200000` clean-lap
`+1000`, the bomb-ahead warning, `EnergyRechargePerStage`, the `rm+0x2de8` award counts.
Tests: `crates/weapons/src/detonator/cannon/tests.rs`, the hash test in `oag-gameplay`, and
`detonator_ground_truth.rs` (a craft standing before a laid mine, Square held).

### Drawn and heard (2026-11-22)

- **Muzzles: the hull's own.** A volley's first round leaves `cannon_flash_left` and its second
  `cannon_flash_right` of `/data/ships/detonator/locators.vex`, read in the simulation half
  (`load::muzzle::cannon_flash`, the file the livery reads the same pair from) and carried by the
  craft's rows through the one hull-to-world conversion the Plasma's weapon node uses
  (`projectile::plasma::hull_point`: model `+Z` forward, `+X` left, rows `0.75` long). Both
  locators carry no rotation (`z` axis `(0, 0, 1)`), so the round leaves along the craft's own
  forward, which is also the original's muzzle-matrix forward row (`+0x160`). A hull with none
  falls back to `projectile::cannon::launch`'s chosen offsets. Test:
  `detonator_hull_ground_truth::an_hd_detonator_volley_leaves_from_the_hulls_cannon_flash_locators`
  (the round's `prev` is the locator carried by the craft, within `1e-3`, and within `0.5` of the
  drawn hull's own matrix times the locator; fails with the locators dropped).
- **The round and its flash.** `detonator_cannonbolt` is drawn on each live round, nose along its
  velocity, up along its launch up (`oag_raceplay::detonator_fx`); `hd_muzzleflash_detonator` is
  re-placed at the firing craft's locator, carried by the live pose, for the round's first `0.1 s`.
  **Chosen, not measured:** the round's draw `0x00121e68` is unread; the placement and the `0.1 s`
  window are HD's ordinary Cannon's (`cannon.md`). The flash model is unscaled and un-rolled.

  **The flash against the original, read 2026-11-24 (RPCS3, Talon's Junction, Feisar; grabs at lap
  clock 2.000, 2.017, 2.033, 2.050, the clip 15 to 14 at the first).** The original's flash is not one
  fixed shape: two jagged blue-white bursts at the wing tips that change size and outline every tick
  and differ between the two sides on the same tick (crop mean grey over 800,640,1120,770 at 1920x1080:
  60.3, 79.4, 55.4, 54.9, mean 62.5; pixels with grey over 150 are 11 to 22 % of the crop and average
  about (205, 231, 243)), and the scene around the craft is tinted blue while it lasts. Ours holds one
  identical frame for its five ticks, both sides alike: grey 76.7 every tick, 22.9 %, bright pixels
  about (232, 245, 250), a blown-out white core. That is the original's largest tick (79.4) held for the
  whole window. Confidence **85** for the difference (four grabbed frames against five rendered ticks).

  **What the disc says, and what it does not** (`PulseBullet` render queue `0x00122098`, vtable `+0x14`,
  confidence 70):
  - The flash branch (`+0x1a8` under a constant, `0x001223b0` to `0x00122468`) builds the muzzle matrix
    (`0x00121ba0`) and places the flash model (`+0x1a4`) with **the model node's own three rotation rows
    and the muzzle's translation** (`0x00323760`, `0x00327500`). It makes no call to the float-range
    roll (`0x0028c660`) and applies no scale, so the size roll `CannonBullet_Update` does (`cannon.md`,
    0.25 to 1.0 on all axes, 0.7 to 1.3 on Z) is **not** read for this round: it must not be wired from
    the ordinary Cannon.
  - The same function calls `0x006778c8` with a vector and floats once in the flash branch and once for
    the bolt (`0x001225b4`, `0x001225e8`), as `cannon.md` records for the ordinary Cannon. The blue tint
    on the flash ticks supports the hypothesis there (a light the round adds); it stays unpinned
    (below 50) because `0x006778c8` is unread.
  - The flash material is `hd_muzzleflash.rcsmaterial`, a blended emissive with `V_Offset`
    (`0x336d2dcc`) authored `(0,0,0,0)`, and its texture `pulse_muzzleflash_blue_glow.gtf` is a
    128x512 strip of **eight** flash frames. Ours draws frame 0 on every tick. That a per-draw
    `V_Offset` picks the frame (the changing outline) is a hypothesis, 55: no writer of it was found
    (the executable's two `"V_Offset"` strings are only reached from a debug-print table).
  - The model's own node animation is one degenerate key, so it does not animate the size.

  **Not wired, because the law is unread.** To read it: set a write watchpoint
  (`just build-rpcs3-watchpoints`) on the flash model's material `V_Offset` and on its node matrix during
  a held-fire grab (`scripts/rpcs3-hd-detonator-shots.py --fire-clock 1.85 --fire-hold 0.25`, shots at
  1.95 to 2.10; the clip drops about 5 ticks after the press), decompile what writes them, and read
  `0x006778c8`'s body for the light's colour, radius and fall-off. Until then a random or stepped frame
  pick, a size roll or a light would be an invented look.
  **2048 and Omega: not checked.** Both ship the same `hd_muzzleflash_detonator` file
  (`detonator-hull.md` on each); their `PulseBullet` render queues are unread, and the frame strip and
  `V_Offset` are the same data in the same file, so the open question carries over unchanged.
- **The wall death.** `WO_CANNON_SPARKS_DETONATOR` at the hit point on a round's second wall hit
  (72). The original spawns it only when the round is in some view's potentially-visible set; here
  every one plays (**chosen**).
- **Sounds.** `CANNON` once per round (two a volley) on the firing craft, `CANNONEXPLWALL` at the
  wall death, both as per-tick cue outputs (ADR-0018). Which weapon bank a Detonator race plays
  (`weapons.bnk` or `weapons_det.bnk`) is unread ([cue-variables.md](cue-variables.md), "Not
  determined"); this engine reads `weapons.bnk`. A mine that goes off plays `MINEEXPL` and a bomb
  `BOMBEXPL` where it was (`Cue::MineExplosion`, `Cue::BombExplosion`, loaded beside `HD_CONTACT`
  on HD only; both resolve to timelines in HD's `weapons.bnk`,
  `sfx_timeline_ground_truth::hd_s_race_cues_play_the_lists_the_bank_authors`). **Chosen:** the
  bomb plays `BOMBEXPL`, never `BOMBEXPL_PC` (the choice hangs on `b+0x58 == b+0x5c`, unread).

## Cross-check: 2048 and Omega

- **Data: checked, applies, not wired.** `DetonatorScoring.xml` is byte-for-byte HD's values on
  2048 (`data.psarc` `data/xml/DetonatorScoring.xml`; the v1.04 `data1.psarc` adds a copy under
  `data/xml/2048_hud/`) and on Omega (`data00.psarc`, two copies, `Data/xml/detonatorscoring.xml`
  and `Data/xml/2048_hud/`). The Cannon block and `GamePlayValues.xml` were already equal
  ([detonator-mine.md](detonator-mine.md#cross-check-2048-and-omega),
  [detonator-placement.md](detonator-placement.md)).
- **The streak law: checked, applies, not wired.** 2048 v1.04 `FUN_8120169a` (the `BONUS_5`
  reference, `0x81201808`) has HD's loop over fifteen pairs, thresholds `5..9`, `10`, `15`, awards
  `1000`/`3000`/`5000` and the second-five rule, at HD's own clip offsets `+0xbc..+0xbf`. Omega
  `FUN_01362150` the same awards and thresholds, and its reset writes the scoring record's `+0x24 = 0`
  and `+0x28`/`+0x2c = 1.0` as HD does. Omega's chain HUD `FUN_01269990` adds the amount to the
  same `+0x14` of the race manager's record.
- **The round: checked, applies, not wired.** Omega's `PulseBullet` (tag at `0x01878a4c`,
  `FUN_013916a0` fire, `FUN_01391b30` update) stores `speed + 500.0`, probes `9.0` along the
  normal, rides `3.5`, renormalises to `500.0`, reflects with `-2.0` and spawns
  `WO_CANNON_SPARKS_DETONATOR`. 2048's update (`FUN_8120058c`, the `WO_CANNON_SPARKS_DETONATOR`
  reference) reflects with `-2.0`, sets `flags |= 0x14` on the last wall and spawns either sparks
  with HD's fourcc `0x42444f52`; its speed and probe constants were not traced to their block.
- **Wired as HD's, applies**: the scoring file resolves on both through the title axis
  (`the_detonator_layout_entries_resolve` on each) and the same code reads either title's own
  Detonator table; a 2048 and an Omega race fire it (`assert_the_cannon_fires` in `detonator_ground_truth.rs`). The hit test, the score adds, the reload and the percentage below are HD's, not
  re-read there.
- **Not read on 2048 or Omega**: the hit test, the score adds, the reload and the percentage. The
  strings are all there (`CannonLockoutTime*`, `CANNONRELOAD`, `~CANNON_DRYFIRE`, `PulseBullets`,
  `new_mine_score`, `bomb_increment`); the work is a line-by-line read of each title's cannon
  hit test and its scoring record, which this pass did not do.
