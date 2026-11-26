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

`rm+0x6de44` counts the game frames (1 or 2 ticks each) the trigger was held with a loaded, ready
cannon (live, one RPCS3 boot, 2026-11-26: 0, 2, 4, 5, 6 at lap clock 1.917, 1.950, 2.017, 2.050 and 2.067): it is the
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
  re-placed at the firing craft's locator, carried by the live pose, for the round's first `0.1 s`
  (**read**, below), rolled every tick, with its two point lights. **Chosen, not measured:** the
  round's draw `0x00121e68` only pushes and pops a matrix (no flash geometry), so the round's own
  placement (nose along the velocity, up along the launch up) stays ours.

### The flash's per-tick roll, read 2026-11-24 (`det-muzzle-flash-re`)

**This retracts the 2026-11-24 `det-muzzle-flash` paragraph that said the size roll "is not read for
this round: it must not be wired from the ordinary Cannon".** The render queue `0x00122098` indeed
rolls nothing (it re-reads the model node's rows with `0x00323760` and moves the translation), but
`PulseBullet_Update` (`0x00122608`) does, and it is the ordinary Cannon's law. A byte scan of
`0x00120000`-`0x00124000` for `bl 0x0028c660` (the float-range roll) finds three sites in it, and the
disassembly (`bl 0x00327500`, the model placement, at `0x00123278`) reads:

- **The window.** Each update adds `dt` to `+0x1a8` (`0x00122eb0`) and, with the flash model at `+0x1a4`
  present, runs the block at `0x00123098` while `+0x1a8 < 0.1` (`0x008aa07c`, `0x3dcccccd`, the same
  constant the render queue's flash branch tests at `0x0012217c`); past it the model node's low three
  flag bits are cleared (`0x00122ed4`-`0x00122ee4`), which hides it, as `CannonBullet_Update` does
  (`cannon.md`). The `0.1 s` window is **read** (80), no longer HD's Cannon's by analogy.
- **The block.** The muzzle matrix (`0x00121ba0`) is orthonormalised (row 0 normalised, row 1 made
  orthogonal and normalised, row 2 their cross product), then:
  1. `roll(0, 2 pi)` (`0x0028c660`, `2 pi` = `0x008aa0c4`) at `0x001231bc`, kept at `+0x1b0`, turns the
     matrix about its row 2 with `Matrix_RotateRowsByAxisAngle` (`0x00677688` -> `0x006ca6b0`, 75;
     axis `v2` is the row-2 slot, `0x001231c4`). In `CannonBullet_Update` the same roll turns the flash
     *quad*; here there is no quad (the draw has none), so the **model** turns.
  2. `roll(0.25, 1.0)` (`0x008aa0d8`, `0x008aa080`) at `0x001231e0`, kept at `+0x1ac`, scales **all three rows**.
  3. `roll(0.7, 1.3)` (`0x008aa0c8`, `0x008aa0cc`) at `0x00123244` scales row 2 alone: the model is stretched
     along its own forward.
  4. The model is placed (`0x00327500`) with that matrix, and every update (inside the window or not)
     adds `roll(150, 255) << 24` (`0x0028c570`) to the colour word at `+0x130` (initial `0x00ffffff`,
     `PulseBullet_Construct` `0x001237c8`), so its top byte wraps. **Nothing in the render queue, the draw
     or the update reads `+0x130`**; the translucency it suggests is therefore **not wired** (the
     material is blended emissive on its own).
  Confidence **80** for sizes, stretch and window (the three rolls and their ranges equal
  `CannonBullet_Update`'s, `cannon.md`), **70** for the rotation being about the model's own row 2
  (the call's argument is the row 2 slot; its sign convention is unread and a random angle does not
  need it).
- **Wired** (`oag_raceplay::detonator_fx`, `detonator_flash_matrices`): per round and per tick, the
  three rolls above; left and right roll apart (each round is its own object). **Chosen, not
  measured:** the rolls are drawn from `oag_fx::weapon_quads::random`, seeded by the round's slot and the
  tick (the simulation's own stream is not advanced), one draw per 60 Hz tick where the original draws
  one per frame (30 Hz in the captures: the lap clock steps `0.0333`).

### The cannon's three point lights, read and polled live 2026-11-24

`0x006778c8` is the tail-call thunk to `SpuLight_AddCandidate` (`0x0040d990`; `weapons.md`, "Weapon point
lights", 90): `(v2 position, v3 colour, f1 D, f2 w)`, a vertex light summed as
`max(0, 1 - |d| / D)^w * max(0, N.L) * colour`. The three calls in the render queue (the earlier page
paired `0x001225b4` and `0x001225e8` as flash and bolt; that attribution is **retracted**):

| Call | Taken | Position | Colour | `D` | `w` |
| --- | --- | --- | --- | --- | --- |
| `0x001225b4` | flash branch (`+0x1a8 < 0.1`), once per round | the muzzle, plus a nudge by `0.3` and `0.3` (`0x008c187c`, `0x008c1880`) along the matrix, unapplied here | `(0, 30, 40)` (table `0x00993f70`, `+0x10`) | `2.7` (`0x008c1884`) | `1.0` (`0x008aa080`) |
| `0x00122234` | bolt branch, every live round | the round | `(0, 0.003, 0.03)` (`+0x30`) | `70` (`0x008c188c`) | `1.0` (`0x008c1890`) |
| `0x001225e8` | the flash with no bolt model (`+0x1a0 == 0`) | the muzzle | `(2, 2, 0.4)` (`+0x40`) | `5.0` (`0x008c1888`) | `1.0` |

Live (RPCS3, Talon's Junction, Feisar, `hd-detonator-talons-feisar-grid-late`, fire held 0.25 s;
the visible candidate list at `0x00f4b300 + 0x20a0` polled continuously, two boots): the two flash lights
`(0, 30, 40)`, `D = 2.7`, `w = 1`, 1.13 apart on the craft's width and moving with it, for 2 frames in boot 1
(lap clock 1.983, 2.017) and 5 in boot 2 (3.200 to 3.333), and the two bolt lights `(0, 0.003, 0.03)`,
`D = 70`, `w = 1`, from the first frame, riding the rounds at about `500` units a second until they
leave range. The colour table words read through the debugger on boot 2
(`0x00993f80 = (0, 30, 40)`, `0x00993fa0 = (0, 0.003, 0.03)`, `0x00993fb0 = (2, 2, 0.4)`) and the
constants block `0x008c1870` (`0.3, 0.3, 2.7, 5.0, 70, 1.0`) match the static reading. Confidence **85**
for the first two rows, **60** for the third (never seen live). The frame count differs between boots (the
original steps a variable `dt`); ours uses the `0.1 s` age window of the roll.

**Wired** (`oag_raceplay::weapon_light`, `detonator_cannon_lights`): the first two rows, under the same
switches as every weapon light. The third is not (no round fires without its bolt). The light's effect
is local and visible: on the hull's upper faces beside each muzzle, cyan; the flash is lit, and the
hull in the original's crops carries the same cyan.

**The blue tint of the scene** (first read as not this light; it is the round's light, on the Zone programs' own path: "The blue wash of a volley" below). Over the original's grabs at lap 1.817 (before the volley)
and 2.000 and 2.033 (lit), the mean RGB of the top-left screen quarter goes from `(9.0, 4.1, 6.8)` to
`(9.4, 9.5, 33.6)` and `(9.1, 8.0, 27.3)`, the top-right quarter from `(13.5, 4.5, 9.6)` to
`(10.9, 6.0, 23.4)`: a wash well beyond a `D = 2.7` light's reach and beyond the `D = 70` one's colour of
`0.03`. Ours, with the light wired, leaves the same quarter at `(13.5, 8.5, 13.2)` on every flash tick.
Its source is **not read** (below 50), but narrowed (`det-muzzle-flash-re`, second pass):

- **It is the flash, not the music.** Four unfired grabs from the same restored state (lap 2.167 to 2.233, held bass bands up to 0.7) leave the top-left quarter at `(7.2..8.0, 2.7..3.1, 3.4..4.5)`; only the fired frames carry the wash. (The fired grab at 2.000 had held bass bands of 0.85 to 0.97, above any unfired one, so a band threshold is not excluded outright; 0.7 gave nothing.)
- **It is the scenery that lights, not a screen wash.** The difference of two original frames (1.933 against 2.000) shows the overhead girders and side structures going blue, shape by shape, the red track edge strips unchanged; a uniform overlay or a bloom would not follow the geometry.
- **Not the wired lights in ours.** The only candidate in the visible list with a range past the craft is the round's `D = 70` light, whose colour `(0, 0.003, 0.03)` reaches at most `+8` of 255 in `lit_sum` and the wash is `+27`. Ours with that colour multiplied by ten leaves the quarter at `(13.5, 8.5, 13.2)`: ours' scenery draws (`shade.wesl`, the `spu` term inside the material classes that take it) are not receivers of a weapon light at all. So two things are unread: which scenery materials the original lights through its `SVC1` twins, and the gain between a record's colour and the picture.

### The blue wash of a volley, read 2026-11-24 (`det-flash-wash-re`)

**The cause (confidence 80).** A Detonator frame draws every scenery and track chunk on its material's
Zone program ([detonator-look.md](detonator-look.md), row 3; [zone-frame.md](zone-frame.md)), and a
chunk the `LightCulling` job finds a light on runs the `SVC1` twin of that program (bit `0x800`,
[renderer.md](renderer.md), "The `LightCulling` SPU job is identified"). The Zone `SVC1` twins do not
add the SPU vertex light where the plain ones do. The plain programs add it to the light before the
albedo multiply (renderer.md, "The `SVC1` combine is read", 88). The Zone programs add it **last, after
the fog lerp and outside `light * surface`, inside the closing `/2`**:

```text
out.rgb = ( lerp(fogColour, light * surface + glow, fog)  +  spu ) / 2
```

Read on `talons_junction/materials/bluemetal.rcsmaterial` (`scripts/ps3-sho.py variants`,
`scripts/ps3-microcode.py vp-file` / `fp-file`). Variant 12, `HalfBrightZoneModeAmbientSunSpot0SVC1`,
has vertex block `0x4ef0` and shares fragment block `0x3bf0` with its `SVC0` twin (variant 9):

```text
vp 0x4ef0 (SVC1)   attribute 0x868f8229 slot 3          (SpuVertexColours)
                   MAD R0.w, v[3].w, 255, -128 ; EX2      (the RGBE decode, renderer.md)
                   MUL o[TC1].xyz, v[3].xyz, R0.w         (the light, into TC1)
vp 0x1fc0 (SVC0)   MOV o[TC1].xyz, c[208].xxxx = 0
fp 0x3bf0 @0x02    MOV R1, f[TC1]
          @0x3b    ADD H1.xyz, R2, H1                     (light: TC0 + ambient + sat(N.L) sun; TC0 is 0 on both)
          @0x3e    MAD H5.xyz, H1, H7, H0                 (light * surface + glow)
          @0x48    MAD H0.xyz, fog, H4, (1 - fog) fogColour
          @0x4b    ADD /2 H0.xyz, R1, H0  END             (+ the SPU light, halved)
```

`track_surface.rcsmaterial` variant 20 (vp `0x9980`, the light from attribute slot 4 into `TC1`; fp `0x71a0`
ends `@0x5e ADD /2 H0.xyz, R2, H0` with `@0x40 MOV R2.xyz, f[TC1]`) and `bluemetal` variant 20 (`ZoneTrans`,
fp `0x6a70`: `@0x70 MOV H0.xyz, f[TC1]; @0x71 ADD /2 H0.xyz, H0, H1`) end the same way.

**Census, Talon's Junction** (all 62 files under `talons_junction/materials/`; every `Zone*SVC1` variant,
classified by whether the shared fragment program's closing `ADD /2` / `MAD /2` reads the register loaded
from the `TC` the `SVC1` vertex program writes the light to):

| Family (Static) | added after the fog | other |
| --- | ---: | ---: |
| `HalfBrightZoneMode*SunSpot0SVC1` | 117 | 2 |
| `HalfBrightZoneMode*SunShadowMapSpot0SVC1` | 55 | 2 |
| `HalfBrightZoneTrans*SunSpot0SVC1` | 81 | 38 |
| `HalfBrightZoneTrans*SunShadowMapSpot0SVC1` | 44 | 13 |
| `ShadowToAlphaZoneMode*Spot0SVC1` | 0 | 41 |

The `StaticQuake` and `RigidBody` rows split the same way. The classifier is a text match, and the
`HalfBright` "other" rows read by hand are the same law under another register: `diffuse_spec_fresnel`
variant 14 ends `ADD /2 H0.xyz, R2.xzwx, H0`, and `bluemetal` variant 20 is the `ZoneTrans` form above.
**`ShadowToAlpha` differs.** `cf_startbeam_glow` variant 16 adds the light inside the light term
(`@0x28 ADD H3.xyz, f[TC1], const`), beside a constant, and ends `MAD /2 H0.xyz, R0.z, H0, R2`. That is
the shadowed-track redraw ([shadow-track-redraw.md](shadow-track-redraw.md)), which this engine folds into
one pass (`shadowed`), so it is not wired separately.

**Why only this placement can draw the wash.** Detonator's stage 1 authors `Scene.Texture Colour`,
`Near Colour` and `Base Colour` as `0 0 0` (detonator-look.md, "What the file says at stage 1"), so a
scenery chunk's Zone surface is black. A light inside `light * surface` adds nothing there, whatever its
gain: that is why ours, with the light in the multiply, stayed at `(13.5, 8.5, 13.2)` with the colour
times ten. Only a term outside the multiply turns black girders blue, and the girders turn the light's
own hue.

**The gain agrees.** The round's light is `(0, 0.003, 0.03)`, range 70, two rounds a volley. On a girder
some 20 units off, with `N.L` near 0.5, it leaves about `0.5 * 0.03 * 0.7 * 0.5 * 2 = 0.0105` in linear
blue. HD's scene target stores the sRGB encode ([zone-frame.md](zone-frame.md), "The Zone surface's
light"), so that is about 27 of 255 in blue and `0.00105` is about 3.5 in green. The original's
top-left quarter moves by `+25.3` in blue and `+4.3` in green (below). The flash's own `(0, 30, 40)`
lights reach 2.7 units and light the hull, not the girders.

**Measured against the original** (top-left and top-right quarter mean RGB at 1920x1080). The original's
frames are lap 1.833 (`02b`, before the volley) and 2.000 to 2.050 (`03`, `03b`, `04`, `04b`), one boot.
Ours is the `cannon-volley` pair's camera and inputs at ticks 380 (before), 386 to 390 (the volley) and
400:

| | TL before | TL in the volley | TR before | TR in the volley |
| --- | --- | --- | --- | --- |
| original | `(18.2, 6.8, 7.3)` | `(18.1, 10.8, 31.1)`, `(18.2, 10.9, 32.6)`, `(18.3, 10.4, 28.5)`, `(17.8, 9.2, 23.0)` | `(24.2, 10.5, 12.1)` | `(23.8, 11.5, 21.0)` to `(25.4, 13.1, 26.9)` |
| ours, light in the multiply | `(18.4, 7.9, 11.9)` | `(18.5, 8.1, 12.1)`, `(18.6, 8.2, 12.2)`, `(18.7, 8.2, 12.2)` | `(21.1, 10.2, 14.9)` | `(21.0, 10.3, 14.9)` to `(21.1, 10.3, 15.0)` |
| ours, after the fog | `(18.4, 7.9, 11.9)` | `(17.7, 10.5, 29.2)`, `(18.0, 10.4, 26.7)`, `(18.2, 9.9, 24.0)` | `(21.1, 10.2, 14.9)` | `(20.0, 10.5, 20.4)` to `(20.5, 11.5, 24.8)` |

At tick 400, 0.23 s on, ours is back to `(19.3, 8.9, 13.2)`. The picture matches as well: the overhead
girders and side structures go blue shape by shape, the red edge strips stay red. The pair's own frame
(`cannon-volley`, 2.63 s, 0.75 s after its volley, the rounds some 375 units away) does not move: whole
frame 15.00 before and after, top-left `(16.9, 7.5, 10.0)` both (original `(20.3, 10.0, 13.7)`). `pad`
8.17 and `mines-ahead` 14.65 are unchanged. Neither `zone-vineta-k-z5` nor the plain race pair `talons`
changes a byte. No HD Rocket or Bomb pair exists. An ordinary race draws plain programs, where ours
already added the light, so the change cannot reach one.

**Wired.** In `shade.wesl`, the generic combine of a chunk on its Zone program records the light
twice in `SunTerms`. It needs the title's Zone frame to be read as one (`ZoneFrame::lit_by_stage_alone`,
HD only) and the Zone grade to apply (not a `zone_plain` or `ZONE_EXEMPT_RATE` chunk):

- `after_fog`, `output_scale * spu`;
- `redraw`, what the `ShadowToAlpha` redraw's `surface * spu` adds.

A pane's clear part takes neither (weighted by its cover). The special combines take neither either
(light cone, facing-ramp sheen, holo billboard, flame, absorb shell, rims), and neither does a draw off the
authored rig. `shadow.wesl`'s `shadowed` weights `after_fog` by the craft shadow's `1 - coverage` and adds
`coverage * redraw`, so under a covered texel only `surface * spu` is left, as the original's blend of its
redraw gives. `fog.wesl`'s `plus_after_fog` adds it after `fogged`, in linear light. A `zone_plain` chunk
runs its plain `SVC1` program, which takes the light in its sum. Tests: `crates/mesh/tests/zone_spu_light.rs`,
on HD's encoded race target:

- With the authored fog on and a grey Zone surface, blue is `fogged + spu / 2` within 2, and the test
  asserts that the three wrong placements (before the fog, twice, in the encoded domain) are each more
  than 3 off.
- Under full coverage it is `fog(surface * (ambient + spu) / 2)`.
- An exempt chunk is unchanged by the light.
- A black Zone surface turns the light's blue; a white plain chunk takes it, a black one does not, and
  without the marker nothing is added.

Shown red: with no add, with the coverage weighting missing (blue 156 against 86), and with the exempt
gate missing (160 against 71). The volley frames after this: ticks 386, 388 and 390 top-left blue 28.6,
26.6 and 24.0. The pairs are as above.

**2048 and Omega.** 2048: checked, differs. It has no SPU light record ([weapons.md](weapons.md)), and
`Detonator::cannon_lights` is false there. Omega: not checkable. Its programs are PS4 recompiles with no
SPU stage, and with no memory readout there is no way to see whether a light list feeds them. Neither title sets
`lit_by_stage_alone`, so nothing changes on either.

**Reproduce.** The census: for each `talons_junction/materials/*.rcsmaterial`, `scripts/ps3-sho.py
variants <image> <path>` for the `Zone*SVC1` rows, then `scripts/ps3-microcode.py vp-file` and `fp-file`
on the extracted file (`just psarc cat <image>:PS3_GAME/USRDIR/DATA00.PSARC <path>`). The frames:
`oag-game` with the `cannon-volley` pair's flags and `--ticks 386`.

**`V_Offset`.** `PulseBullet_Construct` (`0x001235a8`) makes no `Material_BindInstanceParamPointer`
(`0x00677018`) call and no hash-name call (`0x00676ff8`), so the flash material has no game-side `V_Offset`
bound to a variable; the hash `0x336d2dcc` occurs nowhere in the executable. The changing outline of
the original is accounted for by the roll above (rotation, size, stretch), so the 8-frame strip stays at frame
`0` as authored (`V_Offset = 0`). That the strip is never advanced is **inferred** (65), not watched: no
write watchpoint was run on the material.

**Measured against the original** (crop `800,640,1120,770` at 1920x1080; the original's four grabbed
frames at lap 2.000, 2.017, 2.033, 2.050, one boot, from `det-muzzle-flash`; ours the pair's own
camera and inputs, ticks 386-390 and a second volley at 401-405):

| | mean grey per tick | bright (grey over 150) | bright pixels' mean RGB |
| --- | --- | --- | --- |
| original | 60.3, 79.4, 55.4, 54.9 | 11 to 22 % | about (205, 231, 243) |
| ours before | 76.7 on every tick | 22.9 % | about (232, 245, 250) |
| ours after, volley 1 | 63.8, 54.6, 52.6, 58.9, 50.0 | 17, 12, 11, 14, 10 % | (173..204, 241..243, 246..248) |
| ours after, volley 2 | 56.6, 58.4, 53.7, 63.3, 50.5 | 13, 14, 12, 16, 10 % | |

Ours now changes size and outline every tick, differs left and right, and sits inside the original's
bright share; its mean over ten ticks (56) is a little under the original's four grabs (62.5), whose
largest tick (79.4) ours has not reached in ten. The whole-frame mean grey of the `cannon-volley` pair
(2.63 s, after the window) is 14.99 stored and 15.00 re-rendered on this branch, with the lights switched
off 15.00 as well: the change does not move it (the stored figure comes from an earlier binary).
**Open:** the `+0x130` alpha word's reader; the `0.3` nudges of the light's position;
the third light's live trigger.

- **The wall death.** `WO_CANNON_SPARKS_DETONATOR` on a round's second wall hit, in the frame and
  at the place of "The spark's matrix, read live" below (85). The original spawns it only when the round is in some view's potentially-visible set; here
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
- **The muzzle flash's roll and lights (`det-muzzle-flash-re`): checked, applies in data, wired, not
  matched.** Both ship `hd_muzzleflash_detonator` (`detonator-hull.md` on each), so the model and its material
  are the same data. The roll in `detonator_flash_matrices` is title-agnostic and so now runs on a 2048
  and an Omega Detonator race too. Their `PulseBullet` updates (Omega `PulseBullet` tag `0x01878a4c`, 2048
  `FUN_8120058c`) were **not read for the three rolls**: a byte search of both executables (Omega, 2048 v1.04
  and base) for the `[2.7, 5.0, 70.0, 1.0]` constants block found nothing, as the table may be literals
  there. The point lights are HD-only in this engine (`Detonator::cannon_lights` is `true` on HD alone; the
  trail flag is true on 2048 and Omega too, so it is not the gate), so 2048 and Omega draw the roll and no light; whether theirs submit one is unread.

## The round, polled live (2026-11-25, `det-cannon-ricochet`)

Method: RPCS3 restored from `hd-detonator-talons-feisar-grid`, the cannon held, every `PulseBullet` of the pool
(vtable `0x00863d98`, 30 objects) read through `/proc/<pid>/mem` without pausing (`scripts/rpcs3-hd-pulse-round-poll.py`);
for the bounce, each new round's velocity was turned 60 degrees left or right (only `+0xd0` written) to send it at a side
wall. One boot, 179 in-flight samples, rounds of three volleys; the game steps `dt = 1/30`.

| Quantity | Read | Confidence |
| --- | --- | ---: |
| Step | `500 / 30 = 16.67` units a tick, velocity constant on open ground, no gravity | 85 |
| `+0x150` (up row) | equals `+0x180` (surface normal) in every sample, within `3e-5`, also when the normal changes (floor hit): **the up row is the normal**, not the muzzle matrix's up | 85 |
| Velocity | `\|v\| = 500` within `1e-4`, perpendicular to the **current** normal (`v.n` under 0.01 of 500, nonzero only on a tick that bounced or was written): flattened against the normal each tick | 85 |
| Rows | `fwd` perpendicular to up within `1e-7`, `right = up x fwd` within `5e-8`; `fwd` is the velocity direction, except on a bounce tick | 85 |
| Bounce | `+0x12c` 1 to 0, velocity reflected, speed 500; the rows show the incoming heading on that tick and the reflected one the next (4 of 179 samples) | 80 |
| Second wall | flags `0x15` (`0x14 | 1`), the round is retired | 75 |

So the static reading above ("no writer of `+0x150` found; the up row looks like the launch matrix's") is corrected: the
up row follows the surface normal, and with it the flattening of the heading, so a round turns with a slope. Ours
flattened against the launch normal, so a round fired on the flat and ridden up an incline kept a horizontal heading;
now `Round::normal` is the up, and `Round::heading` carries the drawn forward row.

**The round's draw** (`0x00122098`, bolt branch `0x1221b4`, read): rows `+0x140/+0x150/+0x160` are turned half a turn
about the up row (`0x00677688`, angle pi), the width rows scaled by `0.75` (`0x008c1870 +0x28`), the forward row by
`0.75 * (0.5 + 0.5 * min(age / 0.1, 1))` (`+0x24` = 0.1), placed at `+0x170`. Static, 75; not yet watched live.
Wired: scale and growth, `oag_raceplay::detonator_fx::round_matrix`; the half turn and the look since 2026-11-25 (below).
**Before that** the half turn was not applied and the look differed: from behind, the original's bolt is a wide translucent
blue ring stack (about 100 px at 1080p, lap 2.000 of the `cannon-volley` capture), ours a narrow white teardrop (about
40 px). Why is not found (model axes, material or the turn), so the bolt's shape is **not matched**.

**2048 / Omega:** checked, applies for the law (their `PulseBullet` updates probe 9.0 and ride 3.5 as above; the up-row
rule was not re-read there); not wired separately (shared code). The draw read is HD's.

## The bolt's look and the loop (2026-11-25, `det-cannon-bolt-look`)

**The look: the model's own program, and the half turn.** The bolt is one model,
`Data\Weapons\detonator_cannonbolt`, one material, `hd_detonator_cannonbolt_halo`, two pictures (a blue band
with a cloud in its alpha, and a 4x128 strip whose alpha is eight rings), drawn once per round: no trail and no
second draw. The ring stack is the strip's alpha along the streak; the program is read instruction by instruction on
[hd-unlit-programs.md](../../../rendering/hd-unlit-programs.md#the-detonator-cannon-bolt-2026-11-25-det-cannon-bolt-look)
(90). Ours drew the material through the lit path with the strip unbound, hence the white teardrop; it now draws the
program (`oag_mesh::mesh::rcs::cannon_bolt`, `PROGRAM_CANNON_BOLT`).

The streak runs along the model's `+Z` from `-1.8` to `30.5`, so the render queue's half turn about up
(`0x00677688`, angle pi, read 75) puts it **behind** the round: a streak about 23 units long (after the `0.75` scale)
trailing from the round back toward the muzzle, which is why the original's bolt sits between the muzzle flash and the
round and looks wide from a chase camera. Ours drew it ahead of the round. Now `round_matrix` negates the right and
forward rows (`a_round_is_drawn_turned_half_about_up_so_the_streak_trails_it`). From behind, at the `pad` pair's
camera and ours' tick 387 of a held volley, ours' bolt is a blue ring stack between the muzzle flash and the round, of
the original's width and placement (D14's `cannon-volley` crop against ours: a player sees the same bolt). **Not
measured:** a side-on view of the original (its camera was not moved), and the unit-0 gamma nibble of this draw.

**Through the loop: the ride law holds, live.** Talon's Junction has a full loop (spline samples 1150 to 1350: the
climb reaches 81.6 degrees at sample 1243, the track is inverted at 1330). RPCS3, `hd-detonator-talons-feisar-grid`
restored, the craft set at rest at the loop's foot (sample 1090, aimed at sample 1200), one volley
(`scripts/rpcs3-hd-pulse-round-poll.py --craft-at ...`); the round was polled every frame (`dt = 1/30`) from age
`0.033` to its end:

| Age (s) | Where | Up row (= `+0x180`) | Heading | Confidence |
| ---: | --- | --- | --- | ---: |
| 0.033 | the foot, `(-259.6, -73.6, 55.5)` | `(0.018, 0.999, -0.028)` | level | 85 |
| 0.367 | climbing, `y -15.7` | `(0.896, 0.424, 0.130)` | pitched up 64 degrees | 85 |
| 0.467 | vertical, `y 30.9` | `(0.984, -0.013, 0.179)` | straight up | 85 |
| 0.733 | inverted, `(-342.3, 112.9, 101.8)` | `(-0.011, -1.000, -0.002)` | level, upside down | 85 |
| 0.867 | coming down | `(-0.675, -0.735, -0.067)` | pitched down 41 degrees | 85 |
| 0.900 | a side-wall rebound (`+0x12c` 1 to 0), then straight on to age 1.0 | | | 80 |

In all 30 samples the up row equals the surface normal and `|v| = 500` with `v.n = 0` (the flattening), so a round
rides a loop all the way round with no gravity: the same law as on the curve, now on every slope. An earlier attempt
that wrote a round straight into the loop with the craft 700 units away flew straight through it: the floor probe
finds nothing there, so **collision for the probe is only present near the craft** (observed once, 70). Ours seeded
with the original's round at age `0.033` follows it through the loop: up row within `0.06` degrees, heading within
`2.1`, position within `0.05` units at `0.2` s growing sideways to `4.8` at `0.87` s (ours steps at 60 Hz, the
original at 30 Hz: the likely cause, not proven). Test:
`detonator_loop_ground_truth::a_cannon_round_rides_the_talons_junction_loop_as_the_original_does` (red with the
normal update dropped).

## The spark's matrix, read live (2026-11-25, `det-cannon-spark`)

On the death branch (`flags |= 0x14`, `0x00122b60`) `PulseBullet_Update` builds a matrix and hands it to the
spawn (`0x002c9108`, `r6 = r1 + 0xe0`, fourcc `BDOR`, `0x00123338`). The spawn's constructor (`0x002c8c70`, flags
`0`) copies it into the particle instance: vtable `0x0086a070`, rows at `+0xf0`, `+0x100`, `+0x110`, translation
at `+0x120`, `+0xd0` pointing at `+0xf0`. That makes the matrix readable after the fact with no breakpoint.

**Method.** RPCS3 (recompiler) restored from `hd-detonator-talons-feisar-grid`, the cannon held, each new round
turned toward a wall (`+0xd0`) with its rebound count cleared (`+0x12c`) so the first wall kills it; on the
wall-death flags the game is paused and every instance with that vtable is read
(`scripts/rpcs3-hd-pulse-spark-poll.py`). Two restores, 7 wall deaths. Falsifier, written before the capture:
row 1 world-up, or more than 5 degrees from `v - 2 (v.n) n` with `n` from our collision on the original's own
segment.

**Result** (ours' `n` and hit point from the original's `prev`, velocity and surface normal):

| Hit | Row 1 against the reflection (deg) | Row 1 against world up (deg) | Translation - our hit, along surface normal | along wall normal |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 0.22 | 91.0 | -3.491 | -0.301 |
| 2 | 0.00 | 97.1 | -3.464 | -0.295 |
| 3 | 0.00 | 91.5 | -3.494 | -0.337 |
| 4 | 0.44 | 86.6 | -3.500 | -0.303 |
| 5 | 0.54 | 85.8 | -3.502 | -0.270 |
| 6 | 0.00 | 86.7 | -3.500 | -0.300 |
| 7 | 0.00 | 83.5 | -3.512 | -0.303 |

| Row | Read | Confidence |
| --- | --- | ---: |
| 0 (`+0xf0`) | `row1 x row2`; the matrix is orthonormal, determinant 1 | 85 |
| 1 (`+0x100`) | the velocity reflected off the wall, `v - 2 (v.n) n`, unit | 85 |
| 2 (`+0x110`) | the round's surface normal (`+0x180`), dot `1.0000` with it: the static read's `r29 + 0x20` is the floor record's vector | 85 |
| translation | `3.5` below the hit along that normal, so on the road, not at the round's ride height | 85 |
| the `-0.3` along the wall normal | our hit point against the original's; not explained. Statically the translation is `hit + n + k * V` (`0x00122d9c`), so the original's own hit sits about `1.3` short of ours | 50 |

Ours: `EventKind::RoundWall` carries `spawn = hit - 3.5 * surface normal` and `rebound`, and
`oag_raceplay::detonator_fx` plays the effect at `spawn` and turns its frame's `+Y` along `rebound`. Row 0
and row 2 are not set: every emitter of this effect that reads the frame draws its azimuth uniformly about
`+Y`. The `-0.3` is not applied. Test:
`tests::detonator_fx::an_hd_detonator_race_plays_the_discs_effects_for_each_event` (the seven segments above;
red with the frame left world-up, with the rebound wrong, or with either sprite list dropped).

**The pictures.** The emitters were always the disc's `.pob` (five emitters). What stood in was their look:
the effect was in neither `oag_hd::effects` list, so all five drew the procedural profile and its three class-6
streaks drew as procedural strips. It is now in `SPRITES` and `FAR_BARS` (both are needed: a sprite list alone
leaves the three streaks procedural). All five sprites ship under `/data/psys/tex/`; the copies in `DATA00`,
`DATA02`, `DATA03` and `DATA06` decode to identical top-level pixels. `EXTENTS` is not listed (spawn radii
0.1 to 0.8 units, unmatched). Compared by eye with the original's frames at the same craft position (ours with
the original's matrices replayed at their game times): both show the arcing blue ribbon, the white dashes along
the rebound and the small flash. Not tick-matched and not camera-matched, so size and brightness are not
measured, and by eye ours is fainter (60 for "the same elements"). **Open:** at the grab's own game time one of
the two frames of ours shows no spark where the original shows the arcs; they appear 11 to 33 ticks later. Either
the earlier hits sit behind ours' closer chase camera or ours' particles run late; not settled.

**2048 / Omega, the spark: ported** (2026-11-25). See "The spark on 2048 and Omega" below.

**2048 / Omega.** The half turn: **checked, applies; wired as HD's** (`round_matrix` is title-agnostic). Both ship
the same model with the same extent along `+Z` (`psp2_material_dump` with `DECL_SUB=0`: 2048 `-2..30`, Omega
`-1.80..30.50`); their own render-queue turn is unread. The program: **differs in the tint and the colour tap;
ported** ([hd-unlit-programs.md](../../../rendering/hd-unlit-programs.md#the-detonator-cannon-bolt-on-2048-and-omega-2026-11-25-det-cannon-bolt-2048-omega)). The loop law: not re-read live on either (no 2048 or
Omega memory poll); their `PulseBullet` updates probe and ride as HD's (above).


## The spark on 2048 and Omega (2026-11-25, `det-cannon-bolt-2048-omega`)

Static, 2048 v1.04 `FUN_8120058c` and Omega v1.00 `FUN_013644c0`, each title's `PulseBullet`
update. **Confidence 80.**

| | 2048 | Omega |
| --- | --- | --- |
| Wall codes that take the wall branch | 5, 6, 8 | 5, 6, 8 (mask `0x160`) |
| Rebounds left: counter | `+0x128`, decrement, reflect `v - 2(v.n)n`, return | `+0x188`, the same |
| Last wall | `flags \|= 0x14` | `flags \|= 0x14` (`+0x74`) |
| Effect | mode `0xe`: `WO_CANNON_SPARKS_DETONATOR`, else `WO_CANNON_SPARKS`, fourcc `0x42444f52` | the same |
| Row 1 | `v - 2(v.n)n` | the same |
| Row 2 | the negated surface record vector `+0x20` (`V`) | the same |
| Translation | probe from the hit toward `hit + n + 5V`; on surface types 2, 3, 10, 11, 12 the probe's hit minus `n`, else the wall hit | the same |

So the spark fires **only on the death on the last wall**, as HD's does, in HD's frame, with the
translation probed down to the floor. A flag the caller passes (`param_4`/`param_3`) gates the
spawn; its source is unread. Both are **ported**: each table answers
`Trigger::DetonatorCannonSparks` (`Origin::Measured`) and lists the effect in `far_bars` for
its class-6 streaks (HD's law; Omega's own pusher has the same mode-5 bar, 2048's is unread,
[engine-trail.md](engine-trail.md)). The five sprites read from each title's own files. Ours
spawns at HD's measured point (`hit - 3.5` along the surface normal) on all three; the live
offset is not measured on either. Tests:
`tests::detonator_fx::{a_2048,an_omega}_wall_death_plays_the_discs_spark` (red without the
trigger, and without the far bar). By eye, a camera beside a hit on each: a blue ring or flash,
white dashes along the rebound, green debris. No original frame of either title to compare.
