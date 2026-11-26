# DetonatorBomb: found chasing SortRoot's callers, not the render sort itself

2026-08-25, a side effect of the `SortRoot.cpp` sweep on
[renderer.md](renderer.md#what-was-deliberately-not-read), chasing HD's open
see-through-surfaces depth-sort question. That sweep needed to know what
subsystem owned the six real callers of
`SortRoot`'s base constructor; this is as far as that chase got before it
stopped being about draw order. Read [race-manager.md](race-manager.md) and
[collision.md](collision.md) first for the `__FILE__`-at-offset-`0x30` anchor
this page reuses, and [memory.md](memory.md) for the per-function TOC defect.

## Why this was findable

`"DetonatorBomb.cpp\0"` lives at `0x007845d8`, reached through TOC slot
`0x008aa7a8`. Both candidate constructors write that pointer to object offset
`0x30` (`param_1[0xc]`, the same word index `RaceManager_Construct` and
`Collision`'s own constructor write their filename to) - the anchor that
attributes the whole class, same as everywhere else this pattern has been
used on this binary.

Both functions' own OPD entries declare TOC `0x008ad4d8` - the standard value
for this address range per [collision.md](collision.md)'s range table - so
their decompiles are not subject to the TOC defect [memory.md](memory.md)
documents.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x00135328` | `DetonatorBomb_Construct` | 80 |
| `0x00134b48` | `DetonatorBomb_ConstructComplete` | 80 |

Both decompile identically apart from which further constructor they are
reached from - the same base/complete split `RenderManager_Construct`
(`renderer.md`) and `RaceManager_Construct` (`race-manager.md`) use. This one
is resolved by caller, the same rule [mode-manager.md](mode-manager.md)
establishes: `0x00135328` has two real callers, `0x0012f088` and `0x0012f430`
(unnamed, unread - a further derived class two levels up from
`SortRoot.cpp`), the shape of a base-subobject call. `0x00134b48` has none -
only its own `.opd` self-entry - the shape of a complete object built
directly at a spawn site, not yet located.

**80, not 84**: decompilation and a consistent call-site pattern is this
sweep's whole evidentiary basis, same class of evidence as
`RaceManager_Construct`. No second binary - Pulse and Pure do not carry this
class - and no runtime trace.

## What this does and does not establish

Both constructors call `RaceManager_GetInstance()` and, when it returns
non-null, allocate a 0x380-byte object and construct it as `SortRoot`'s
derived class "X" from `renderer.md`'s `SortRoot` note (`0x00137690`'s
complete form) - i.e. a `DetonatorBomb` owns one instance of that class as a
plain member, not as a base. That confirms `SortRoot`'s derived classes sit
in gameplay/weapon code, tied to the race manager, not the renderer - the
evidence `renderer.md` uses to walk back the "draw-order root" guess.

**What it does not establish**: anything about render draw order. What
`DetonatorBomb` does and what class "X" is were read 2026-11-22, below:
X is the bomb's blast visual, `DetonatorBombBlast`.

**Correction, 2026-11-22**: the two "unnamed" callers above are
`BombManager`'s own constructors. The calls are at `0x0012f3b4` (inside
`0x0012f088`) and `0x0012f75c` (inside `BombManager_Construct`, `0x0012f430`),
each in the mode-14 arm after `cmpwi r0,14` (`0x0012f520`/`0x0012f558`); the
other arm builds a `NormalBomb` (`bl 0x00144a48`, `0x0012f20c`/`0x0012f5b4`).
The pool is two objects in mode 14, sixteen otherwise
([detonator-placement.md](detonator-placement.md#pools)). Confidence 85.

## What a Detonator bomb does (2026-11-22)

Static reading of `EBOOT.elf`, every function on TOC `0x008ad4d8` ("exact"),
bodies read off `llvm-objdump -d --mattr=+altivec,+vsx` where Ghidra truncates
them. The mine's half, the shared craft-side law (damage, the destroyed bit)
and the weapon-table lookup are on [detonator-mine.md](detonator-mine.md);
this section reuses them.

### Names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0012ebf8` | `BombManager_Update` | 80 |
| `0x0012e798` | `BombManager_TestCraftContacts` | 78 |
| `0x0012e598` | `BombManager_ApplyHit` | 78 |
| `0x0012df90` | `BombManager_DetonateAll` | 75 |
| `0x0012e260` | `BombManager_DestroyAtLane` | 72 |
| `0x001335d0` | `DetonatorBomb_PlaceInLane` | 72 |
| `0x00133c00` | `DetonatorBomb_Place` | 72 |
| `0x00133660` | `DetonatorBomb_IsInOwnerGrace` | 75 |
| `0x00134960` | `DetonatorBomb_Update` | 80 |
| `0x00134540` | `DetonatorBomb_Detonate` | 78 |
| `0x001340d8` | `DetonatorBomb_AnimateToMusic` | 76 |
| `0x00137690` | `DetonatorBombBlast_Construct` | 78 |
| `0x00137150` | `DetonatorBombBlast_Start` | 78 |
| `0x00136420` | `DetonatorBombBlast_Update` | 65 |
| `0x00135b90` | `DetonatorBombBlast_Reset` | 65 |

**Class "X" is `DetonatorBombBlast`.** `0x00137690` (called by both
`DetonatorBomb` constructors, `0x00135028`, `0x00135808`) and its twin
`0x00137e88` (unnamed: same body, no caller found) call `SortRoot`'s base
constructor (`0x00137704`: `bl 0x002dbe50`) and load four models through TOC
`-0x2c58`..`-0x2c4c`: `Data\Weapons\detonator_bomb_sphere.vex`,
`detonator_bomb_glowedges.vex`, `detonator_bomb_shockwave.vex`,
`detonator_bomb_lightrays.vex`, binding the material parameters
`Alpha_Scalar`, `Rainbow_Colour`, `V_Offset`, `Range`, `Distortion`, `Speed`,
`Ring Speed`, `Brightness` (TOC `-0x2c38`..`-0x2c1c`). It is the Detonator's
counterpart of `NormalBombBlast` (class "Y", [weapons.md](weapons.md)), owned
at `bomb+0x158`. `DetonatorBombBlast_Update` and `_Reset` are named from their
call shape (called while `+0x370` is set and the bomb retires on a `0` return;
called first by the start and the constructor), their bodies unread.

The `DetonatorBomb` vtable is `0x008641d8` (TOC `-0x2d8c` in
`DetonatorBomb_Construct`): `+0x38` `DetonatorBomb_Place`, `+0x40`
`DetonatorBomb_PlaceInLane` (the slot `Detonator_SpawnObject` calls), `+0x44`
`DetonatorBomb_Update`, `+0x48` `DetonatorBomb_IsInOwnerGrace`, `+0x4c` the
blast reset. `BombManager`'s vtable is `0x00864040` (`+0x0c` the update).

### The weapon table the bomb reads

`WeaponStats_ParseBomb` (`0x0002c690`, [detonator-mine.md](detonator-mine.md)
for the table and the Mine block), HD's Detonator values from `DATA00.PSARC`
`/data/xml/weaponstats_detonator.xml`:

| Attribute | Field | Type | Value | Reader |
| --- | --- | --- | ---: | --- |
| `damage` | `+0xe8` | float | `20` | `BombManager_ApplyHit` |
| `damageradius` | `+0xec` | float | `30` | **none found** |
| `blastradius` | `+0xf0` | float | `40` | `BombManager_ApplyHit` |
| `blastforce` | `+0xf4` | float | `30` | `BombManager_ApplyHit` |
| `absorb` | `+0xf8` | float | `25` | not read here |
| `slowdown_time` | `+0xfc` | float | `0.0` | `BombManager_ApplyHit` |
| `trigger_radius` | `+0x100` | float | `6` | `BombManager_TestCraftContacts` |
| `timetodie` | `+0x104` | float | `20` | **none found** on the Detonator path |
| `number_of_shots_to_destroy` | `+0x108` | int | `4` | `DetonatorBombBlast_Start`, `BombManager_DestroyAtLane` |
| `energy_recharge_per_shot` | `+0x10c` | int | `2` | not read here |
| `points_for_cannon_bullet_hit` | `+0x110` | int | `100` | not read here |
| `points_for_destroying` | `+0x114` | int | `500` | not read here |

Confidence **88** for the mapping. "None found": a displacement scan of
`BombManager`'s span (`0x0012d7b0`-`0x0012f500`), `Bomb.cpp`'s, `DetonatorBomb`'s
and `DetonatorBombBlast`'s (`0x001335d0`-`0x00138680`) finds no load at `+0xec`
or `+0x104` off the table (confidence 55).

### `BombManager_Update(mgr, dt)` (`0x0012ebf8`)

```text
for i in 0..live (+0x98):
    b := mgr.bombs[i] (+0x58 + 4i)
    if (b.flags & 1) and !(b.flags & 0x80):
        BombManager_TestCraftContacts(mgr, i)            ; 0x0012eca8-0x0012ecc8, 0x0012ed60
    b.vtable[+0x44](b, dt)                               ; DetonatorBomb_Update, 0x0012ecdc-0x0012ecfc
    if b.flags & 4:                                      ; leaves the pool
        b+0x4c = 255; flags &= ~8; live -= 1; swap with the last slot   ; 0x0012ec40-0x0012ec84
```

The contact test runs **before** the update, so a bomb hit on tick `N`
detonates on tick `N` (the mine is the other way round). Confidence **80**.

### `BombManager_TestCraftContacts(mgr, i)` (`0x0012e798`)

The mine's test with the bomb's numbers:

```text
r := Bomb.trigger_radius (+0x100)                         ; 0x0012e800
p := bomb position (0x0010efc8)
for k in 0..g_GameState.count, s := mgr.craftSlots[k] (+0xa0 + 4k):
    if b.owner (+0x4c) == k and DetonatorBomb_IsInOwnerGrace(b): skip   ; 0x0012e858-0x0012e860, 0x0012e9c0-0x0012e9e8
    if !(b.flags & 8): skip                              ; armed
    if role(s+0xf0) not in {0, 2}: skip                  ; 0x0012e878-0x0012e88c
    if |s+0x90 - p| < r (box early-out, then length):    ; 0x0012e88c-0x0012e964
        b.flags |= 0x80                                  ; 0x0012e980-0x0012e990
        (mode > 15: a network message)
        BombManager_ApplyHit(mgr, i, k)                  ; 0x0012e9b8
then the same wave-height probe as the mine (100.0, 0.1), counter +0x5c   ; 0x0012e9f8-0x0012eba8
```

`DetonatorBomb_IsInOwnerGrace` (`0x00133660`) is `age (+0x15c) < 0.5` (TOC
`-0x2db8`); every Detonator bomb is owned by slot `0`, the player, so **a bomb
cannot hit the player for its first 0.5 s**. A bomb has no darkening step.
Confidence **78**.

### `BombManager_ApplyHit(mgr, i, k)` (`0x0012e598`)

```text
if b.owner == *0x009943a8: s_k+0x124 = 1                 ; 0x0012e5c0-0x0012e5dc, 0x0012e774
b+0x58 = k; b+0x5c = *0x009943a8                         ; 0x0012e5f8, 0x0012e600
s_k+0x138 = 6 (hit kind: bomb); s_k+0x13c = b.owner
s_k+0x120 += Bomb.damage                                 ; 0x0012e614-0x0012e630
s_k+0x130 += Bomb.slowdown_time                          ; 0x0012e618-0x0012e63c
for every craft j, s_j := mgr.craftSlots[j]:             ; 0x0012e648-0x0012e770
    d := s_j+0x50 - p
    if |d| < Bomb.blastradius (+0xf0):
        s_j+0x110 += (d / |d|) * (1 - |d| / blastradius) * Bomb.blastforce (+0xf4)
```

So **the damage goes to the craft that touched the bomb only**, and the push
to every craft within `blastradius`, the toucher included, falling linearly to
zero at the edge. No `velocity_reduction` (that is the mine's, mode 14 only).
`*0x009943a8` is a global word read here as the local player's index (hypothesis,
confidence 55: it is compared with the owner and with the victim). The craft
spends `s+0x120` through `Ship_ApplyPendingDamage` exactly as for a mine
([detonator-mine.md](detonator-mine.md#what-the-craft-does-with-it)); who
spends `s+0x110` was not chased (the existing pending-impulse path in this
engine is the natural home). Confidence **80** for the law.

### `DetonatorBomb_Update(b, dt)` (`0x00134960`) and `DetonatorBomb_Detonate` (`0x00134540`)

```text
Update:
    0x0010f000(b); age (+0x15c) += dt
    if flags & 4: return
    if !(flags & 0x80):
        if b+0x150 != b+0x60: b+0x150 = b+0x60; b+0x154 = table+0x58   ; a hit flash
        DetonatorBomb_AnimateToMusic(b); return
    if !blast.active (+0x370):
        visible := any view v with 0x002d64d0(-1, &pos, &pos, v)       ; by position, unread
        DetonatorBomb_Detonate(b, visible)
        if !blast.active: flags |= 4                     ; leaves the pool this tick
    elif !DetonatorBombBlast_Update(blast, dt): flags |= 4

Detonate(b, visible):
    if visible:
        build the bomb's matrix from its up row (Gram-Schmidt)          ; 0x001346b8-0x00134878
        rm.entries[b+0x64].laneArray[b+0x66] = 3                        ; 0x0013487c-0x001348cc
        DetonatorBombBlast_Start(blast, &matrix, b+0x60)                ; 0x001348d0
        0x00677048(.., 50, ..)                                          ; unread, as the mine's
    stop the "~BOMBRADAR" loop (b+0x50)
    sound: "BOMBEXPL_PC" when b+0x58 == b+0x5c and (byte 0x009384e1 set or mode <= 14),
           else "BOMBEXPL"                                              ; 0x001345d4-0x001346b0
    b+0x50 = 0; b+0x58 = 0; b+0x34 &= ~4 (hidden)

DetonatorBombBlast_Start(blast, &matrix, hits):           ; 0x00137150
    DetonatorBombBlast_Reset(blast)
    if !(player craft+0x6a64 & 0x1000):
        spawn "WO_BOMB_EXPLO_DETONATOR" (TOC -0x2c6c), fourcc 'BMOB' (0x424d4f42), at matrix
    if hits < Bomb.number_of_shots_to_destroy: return    ; 0x001371dc
    copy the matrix to +0x1d0.., +0x370 = 1               ; the sphere, rays and shockwave run
```

`b+0x60` is the bomb's hit count: `DetonatorBomb_Place` zeroes it and
`BombManager_DestroyAtLane` sets it to `number_of_shots_to_destroy`. So:

| How the bomb went off | Effect | Blast visual | Leaves the pool | Confidence |
| --- | --- | --- | --- | ---: |
| A craft flew into it | `WO_BOMB_EXPLO_DETONATOR`, `BOMBEXPL(_PC)` | no (hits `0 < 4`) | the same tick | 72 |
| Destroyed (`BombManager_DestroyAtLane`, the EMP's path; or four cannon hits, unread) | the same | yes, until `DetonatorBombBlast_Update` returns `0` | when the blast ends | 68 |
| The next stage began (`BombManager_DetonateAll`) | the same, no damage to anyone | no, unless it had taken four hits | the same tick | 72 |

All three spawn the effect only when the bomb is visible and the player has
not left the race. The lane is freed only on the visible branch (the mine's
quirk again). The explosion's own lifetime and sizes (the `.pob`, the four
models) are presentation; nothing in the hit depends on them.

### Placement and the stage step

- `DetonatorBomb_PlaceInLane` (`0x001335d0`, vtable `+0x40`) stores the entry
  (`+0x64`, u16) and lane (`+0x66`, u8) and calls `DetonatorBomb_Place`
  (`0x00133c00`): serial, age `0`, owner, hits `0`, position `+0xa0`, up
  `+0xc0`, `+0x34 |= 6`, **flags `|= 9`** (active and armed at once), and a
  positioned `"~BOMBRADAR"` loop (TOC `-0x2d64`) at volume 1.0. Confidence 72.
- **Only one bomb lives at a time**: every stage step the placer's housekeeping
  (`table+0x54`, ELF default `1`, no attribute writes it) calls `0x0012ae10`,
  which is `BombManager_DetonateAll` (`0x0012df90`): every live bomb
  `flags |= 0x80` and its lane back to `3` (`0x0012dfd0`-`0x0012e040`). The
  placer then lays the new bomb. The old one goes off on its next update and
  leaves; that is why the pool is two, and it answers
  [detonator-placement.md](detonator-placement.md#pools)'s "nothing bounds a
  third bomb". The manual says the same: "there will only ever be one bomb".
  Confidence **75**.
- `BombManager_DestroyAtLane` (`0x0012e260`): the live bomb at
  `(entry, lane)` gets `flags = (flags & ~1) | 0x80`, hits =
  `number_of_shots_to_destroy`, `rm+0x6de40 += 1` (bombs destroyed) and a flat
  score add (`0x00082808`). Its callers `0x0013c0e8`, `0x0013c2a4` are in
  `EMP.cpp`'s span, beside `MineManager_DestroyAtLane`'s. Confidence 72.

### What the simulation needs

| Quantity | Value | Confidence |
| --- | --- | ---: |
| Contact | sphere, `|craft - bomb| < 6.0` (`trigger_radius`) | 80 |
| Owner grace | the player is immune to a bomb younger than `0.5` s | 78 |
| Damage | `20` energy to the craft that touched it, halved when the weapons flag is off | 78 |
| Push | every craft within `40`: `unit(craft - bomb) * 30 * (1 - d/40)` into the pending impulse | 78 |
| Slowdown, velocity cut | none | 80 |
| Delay | none: detonates on the tick of contact | 80 |
| Lifetime | none read; the stage step detonates it harmlessly | 72 |
| Effects | `WO_BOMB_EXPLO_DETONATOR` (visible, player still racing), `BOMBEXPL`/`BOMBEXPL_PC` | 72 |
| Destroyed craft | the mine's chain: energy `0` -> state 4 -> `ship+0x6a64 |= 0x1000` | 72 |

### Open

- `damageradius` and `timetodie` have no reader found. How the cannon counts
  hits into `b+0x60` (four hits, the fourth destroys the bomb harmlessly) and
  what it scores is read on [detonator-cannon.md](detonator-cannon.md):
  `points_for_cannon_bullet_hit`, `points_for_destroying` and
  `energy_recharge_per_shot` are not read; `DetonatorScoring.xml` is.
  `BombChainReactionDistance` is parsed and dropped, and the chain radius is the
  ELF's own `5000` (squared) ([detonator-placement.md](detonator-placement.md)).
- `DetonatorBombBlast_Update`'s timing and the `0x002d64d0(-1, ..)` visibility
  branch.
- Who spends `s+0x110`.

### Wired: the detonation effect (2026-11-22)

Every bomb that goes off (a craft's contact, a cannon's fourth hit, the stage step that replaces
it) plays `WO_BOMB_EXPLO_DETONATOR` at the bomb (`EventKind::{BombTouched, BombShot, BombReplaced}`,
`oag_raceplay::detonator_fx`). Since 2026-11-26 its camera-facing emitters draw the disc's own
sprites, its streaks the procedural strip, unpaired
([particle-fill.md](../../../rendering/particle-fill.md#which-effects-draw-a-sprite)). Chosen: it plays whether or not the bomb is in view or the player
is still racing (the original skips it when the player is out). `BOMBEXPL` plays with it (`Cue::BombExplosion`; `BOMBEXPL_PC`'s operand is unread, so never
chosen, [detonator-cannon.md](detonator-cannon.md), "Drawn and heard"); the HD Bomb's other blast
parts (`detonator_bomb_sphere`, `_shockwave`, `_lightrays`) are not drawn. Test:
`tests::detonator_fx::an_hd_detonator_race_plays_the_discs_effects_for_each_event`.

### Cross-check: 2048 and Omega

The weapon values and the three effects are identical on all three titles
([detonator-mine.md](detonator-mine.md#cross-check-2048-and-omega)). Both
executables name `DetonatorBomb.cpp`, `WO_BOMB_EXPLO_DETONATOR`,
`BOMBEXPL_PC` and `detonator_bomb_sphere`; 2048 also names `~BOMBRADAR`,
Omega does not (its sounds go through another bank, unread). Omega's one
reference to `WO_BOMB_EXPLO_DETONATOR` (`0x01815c6c`) is at `0x01337210`, unread. **Checked, applies, not
wired** for the data and the effect; the contact, hit and stage-step laws
**not read** on 2048 or Omega (no reader located; the work is a line-by-line
read of each title's `BombManager` update).

## See also

- [detonator-mine.md](detonator-mine.md) - the mine, the craft-side damage and
  the out-of-race bit
- [renderer.md](renderer.md#what-was-deliberately-not-read) - the
  `SortRoot.cpp` note this page grew out of
- [race-manager.md](race-manager.md) - the `__FILE__`-at-offset-`0x30`
  pattern and `RaceManager_GetInstance`
