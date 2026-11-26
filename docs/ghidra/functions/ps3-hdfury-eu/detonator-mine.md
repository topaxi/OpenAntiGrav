# Detonator: what a mine does on contact, and how a craft leaves the race

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`). Every function below declares TOC
`r2 = 0x008ad4d8` in its own OPD entry (`scripts/ps3-toc.py toc`: "exact"), so the TOC defect
[memory.md](memory.md) describes does not apply. Static reading only, 2026-11-22: nothing here
ran. Ghidra cuts several of these bodies short on AltiVec words (`MineManager_TestCraftContacts`,
`MineManager_ApplyBlast`, `DetonatorMine_Place`), so the laws below were read off
`llvm-objdump -d --mattr=+altivec,+vsx` over the real extent; TOC constants through
`scripts/ps3-toc.py resolve`. Read [detonator-placement.md](detonator-placement.md) first: it lays
the mines this page detonates, and its track-entry lane array is what a detonation frees. The
bomb's half is [detonator-bomb.md](detonator-bomb.md).

## Names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0002b5e8` | `WeaponStats_ParseMine` | 85 |
| `0x0002c690` | `WeaponStats_ParseBomb` | 85 |
| `0x0011b110` | `MineManager_Update` | 80 |
| `0x0011ac90` | `MineManager_TestCraftContacts` | 78 |
| `0x0011aa88` | `MineManager_ApplyBlast` | 78 |
| `0x0011a590` | `MineManager_DestroyAtLane` | 72 |
| `0x0011a418` | `MineManager_RemoveAll` | 72 |
| `0x0011a360` | `MineManager_DarkenAll` | 70 |
| `0x0010ec98` | `BaseMine_Update` | 80 |
| `0x0010e968` | `BaseMine_PlaceAt` | 75 |
| `0x0010ea48` | `BaseMine_IsPastOwnerGrace` | 75 |
| `0x0010ea18` | `BaseMine_GetPosition` | 80 |
| `0x00139d18` | `DetonatorMine_Update` | 80 |
| `0x00139da0` | `DetonatorMine_Place` | 72 |
| `0x00138e58` | `DetonatorMine_Explode` | 82 |
| `0x00688280` | `DetonatorMine_SetDark` | 72 |
| `0x00688290` | `DetonatorMine_IsLive` | 72 |
| `0x001395f0` | `DetonatorMine_AnimateToMusic` | 78 |

`MineManager_*` live in `MineManager.cpp`'s span (constructors `0x0011b538`/`0x0011b8e8`,
`scripts/ps3-toc.py map`), `BaseMine_*` in `BaseMine.cpp`'s (`0x0010ec28`/`0x0010ee10`),
`DetonatorMine_*` in `DetonatorMine.cpp`'s (`0x0013a248`/`0x0013a7d8`). The two tiny
`DetonatorMine` methods at `0x00688280`/`0x00688290` sit in a pool of one-line methods; they are
attributed by the vtable that holds them, below.

## The three vtables

`DetonatorMine_Construct`'s `*this = TOC -0x2bc8` resolves to `0x008642f0`; `Mine.cpp`'s
constructor (`0x00140b80`) writes `0x008644d8`, `BaseMine.cpp`'s (`0x0010ec28`) `0x00863938`.
Slots `+0x00`..`+0x2c` are shared scene-object methods; the mine's own begin at `+0x30`:

| Slot | `BaseMine` | `Mine` | `DetonatorMine` | Reading |
| --- | --- | --- | --- | --- |
| `+0x38` | `0x0010ec98` | `0x00140078` | `0x00139d18` | per-tick update (called by the manager with `dt`) |
| `+0x3c` | `0x00686448` (`blr`) | same | `0x00688280`: `this+0x160 = 0` | "darken": no-op except on a Detonator mine |
| `+0x40` | `0x00686450` (`return 1`) | same | `0x00688290`: `return this+0x160` | "is live" |
| `+0x44` | pure | `0x001402e8` | `0x00139da0` | place at a matrix |
| `+0x48` | pure | `0x00140710` | `0x00138e58` | explode (effect, sound) |

`Mine`'s `+0x48` is the ordinary mine's `WO_MINE_EXPLO` site already in
[particle-triggers.md](particle-triggers.md). Confidence **85** for the table (read word by word
off `.data` through the OPD entries).

## Object fields used below

| Field | Meaning | Evidence |
| --- | --- | --- |
| `+0x40` | flags: `1` active, `8` armed, `0x80` triggered, `4` remove | `0x00064f70` sets `1`; `DetonatorMine_Place` ORs `8` (`0x00139f0c`-`0x00139f24`); the readers below |
| `+0x44` | owner craft index | `0` from `Detonator_SpawnObject` (`0x000652f4`-`0x000653e4`): the player owns every Detonator mine |
| `+0x80` | position | `BaseMine_GetPosition` copies it (`0x0010ea18`) |
| `+0xa8` | life left, s | `BaseMine_Update` `-= dt` (`0x0010ecb8`); `DetonatorMine_Place` writes `0` (`0x00139df8`) |
| `+0xb8` | PVS cluster of the laying section | `BaseMine_PlaceAt` stores `r10`, the placer's `(s8)sec+0x60` (`0x0010e988`) |
| `+0xc0` | age, s | `BaseMine_Update` `+= dt` (`0x0010ecb4`); `DetonatorMine_Place` writes `0` (`0x00139dfc`) |
| `+0xc4` | visible this tick | `BaseMine_Update`, below |
| `+0xc6`, `+0xc8` | track entry (u16), lane index `j` (u8) | `BaseMine_PlaceAt` `0x0010e998`, `0x0010e984` |
| `+0x160` | **live**: laid this stage | set `1` by `DetonatorMine_Place` (`0x00139e68`), cleared by slot `+0x3c` |

## The weapon table the mine reads

`MineManager_TestCraftContacts` and `MineManager_ApplyBlast` read the current mode's weapon
table, `table[*(0x009389a4 + 0xc)]` (the global behind TOC `-0x36c8`; the same lookup
[autopilot.md](autopilot.md) reads for the Autopilot's time). `WeaponStats_ParseMine`
(`0x0002b5e8`, the only function naming `points_per_metre` and `velocity_reduction`,
`scripts/ps3-toc.py attrib`) fills the `Mine` block of that table; each row is a `strcasecmp`
against a TOC string then a float parse (`0x006769b8`) and a store:

| Attribute | Field | Type | HD Detonator value | Reader on this page |
| --- | --- | --- | ---: | --- |
| `damage` | `+0x118` | float | `10` | `MineManager_ApplyBlast` |
| `blastradius` | `+0x11c` | float | `30` | `MineManager_TestCraftContacts`, `MineManager_ApplyBlast` |
| `blastforce` | `+0x120` | float | `0` | `MineManager_ApplyBlast` |
| `timetodie` | `+0x124` | float | `7` | none in Detonator (life is `0` and ignored, below) |
| `absorb` | `+0x128` | float | `20` | not read here |
| `slowdown_time` | `+0x12c` | float | `0.0` | `MineManager_ApplyBlast` |
| `trigger_radius` | `+0x130` | float | `3` | `MineManager_TestCraftContacts` |
| `points_per_metre` | `+0x134` | int | `1` | **none found** |
| `max_distance` | `+0x138` | int | `300` | **none found** |
| `velocity_reduction` | `+0x13c` | float | `0.5` | `MineManager_ApplyBlast` (mode 14 only) |

The `Bomb` block (`WeaponStats_ParseBomb`, `0x0002c690`) is on
[detonator-bomb.md](detonator-bomb.md#the-weapon-table-the-bomb-reads). Values are
`DATA00.PSARC`'s `/data/xml/weaponstats_detonator.xml` `<Weapon type="Mine">`. Confidence **88**
for the mapping (every store read against its attribute string).

**`points_per_metre` and `max_distance` have no reader found.** A displacement scan of every
function that loads the table global (`0x009389a4`, 118 functions) for `lwz`/`lfs` at `+0x134`
and `+0x138`, and of the whole weapon span `0x0010e000`-`0x00160000` and the `SPDetonator` span
`0x0005f000`-`0x0006f000`, finds nothing that reads the `Mine` block there (the hits are other
objects' fields). The score adds `MineManager_DestroyAtLane` feeds (`0x00082b48`, below) take a
separate score record. Confidence **55** that the two are parsed and never read; the score law is
the Detonator-cannon phase's.

## `MineManager_Update(mgr, dt)` (`0x0011b110`)

```text
if mgr.live (+0x398) == 0: mgr+0x34 &= ~2; return 1
for i in 0..live:
    m := mgr.slots[i] (+0x78 + 4i)
    m.vtable[+0x38](m, dt)                               ; 0x0011b1a4-0x0011b1bc
    if m.life (+0xa8) > 0.0 or !(m.flags & 1):           ; 0.0 is TOC -0x36e4; 0x0011b1c4-0x0011b1ec
        MineManager_TestCraftContacts(mgr, i)
    elif byte 0x009384e1 == 0 and g_GameState.mode == 14:  ; 0x0011b1f0-0x0011b204, 0x0011b2fc-0x0011b308
        MineManager_TestCraftContacts(mgr, i)            ; a Detonator mine never times out
    else:
        m.flags |= 0x80; MineManager_TestCraftContacts(mgr, i)   ; any other mode: life over = detonate
compact: every m with flags & 4 leaves the pool:
    flags &= ~0x8c; owner = -1; m+0x34 &= ~4; live -= 1; swap with the last slot   ; 0x0011b224-0x0011b2c8
```

`g_GameState` is `*(TOC -0x36ec)` = `0x00936fe8` (`+0` craft count, `+0xe0` mode;
[eliminator-score.md](eliminator-score.md)). `0x009384e1` is the byte [weapons.md](weapons.md)
reads as `0` in every ordinary race. `DetonatorMine_Place` writes life `0`, so in Detonator the
mode-14 arm is the one taken: **a Detonator mine never times out** (`timetodie="7"` is not used).
Confidence **80**.

## `BaseMine_Update(m, dt)` (`0x0010ec98`), reached through `DetonatorMine_Update`

`DetonatorMine_Update` (`0x00139d18`) calls `BaseMine_Update`, then, while the mine is not
triggered, spins it (`+0xfc = +0xc0 * k`) and, when visible, runs `DetonatorMine_AnimateToMusic`
(`0x001395f0`: on a live mine, size and colour pulse on `SoundSystem_GetBandLevel`; a dark mine
skips the pulse). Presentation only; the writes of `MineColour`, `MineBrightness` and the
visualiser's binds are read on [detonator-mine-material.md](detonator-mine-material.md).

```text
age += dt; life -= dt                                    ; 0x0010eca8-0x0010ece0
0x00126c38(m)                                            ; owner gone -> flags |= 4
visible := any view v < count(*(TOC -0x3a58)+0xe4) with 0x002d64d0(m.cluster, &pos, &pos, v)
                                                         ; 0x0010ecec-0x0010ed58
if flags & 0x80:                                         ; 0x0010ed60-0x0010ed6c
    if flags & 8:
        if visible: m.vtable[+0x48](m, &m.pos)           ; Explode, 0x0010eddc-0x0010ee04
        flags &= ~8; m+0x34 &= ~4                        ; 0x0010edb8-0x0010edd0
    flags |= 4                                           ; 0x0010ed7c
```

`0x002d64d0` tests a section's PVS cluster against a per-view bit mask (`+0x4b48 + view*8` of the
visibility block, decompile), so "visible" is "in some view's potentially-visible set", not a
frustum test. Confidence **70** for that reading, **82** for the control flow.

**A triggered mine leaves on the tick after it is triggered**: `MineManager_Update` runs the
update before the contact test, so the test that sets `0x80` on tick `N` is followed by the
update that explodes it and sets `4` on tick `N+1`, and the compaction at the end of that same
call frees the slot. Bit `8` is cleared before the test runs again, so a mine hits once.
Confidence **80**.

**A shipped quirk**: the lane is freed inside `DetonatorMine_Explode` (below), which only runs
when the mine is visible. A mine detonated in no view's PVS leaves its lane marked `0` for the
rest of the race, so the placer never lays there again. Confidence **65** (control flow read; the
consequence for placement is inferred, nothing observed).

## `MineManager_TestCraftContacts(mgr, i)` (`0x0011ac90`)

```text
r     := table.Mine.trigger_radius (+0x130)              ; 0x0011acd8-0x0011ad48
p_m   := BaseMine_GetPosition(m)                         ; 0x0011ad4c
for k in 0..g_GameState.count:                           ; 0x0011ad54-0x0011ada8
    if m.owner == k and !BaseMine_IsPastOwnerGrace(m): skip   ; 0x0011adac-0x0011adb4, 0x0011af90-0x0011afa4
    if !(m.flags & 8): skip                              ; armed, 0x0011adb8-0x0011adc4
    s := mgr.craftSlots[k] (+0x58 + 4k)
    if !0x0012d7e0(s): skip                              ; craft present and role 0 or 2
    d := s+0x90 - p_m
    if |d.x| < r and |d.y| < r and |d.z| < r and |d| < r:     ; 0x0011ade4-0x0011ae98
        m.flags |= 0x80                                  ; 0x0011ae9c-0x0011aea8
        m.vtable[+0x3c](m)                               ; darken: DetonatorMine_SetDark
        if |d| < table.Mine.blastradius (+0x11c):        ; 0x0011aecc-0x0011af2c
            MineManager_ApplyBlast(mgr, i, k)
        (mode > 15: a network message, 0x0011af40-0x0011af88)
then: a wave-height probe at p_m (0x000a8b20 with 100.0, then 0x002d2068 over g_quake):
      height > 0.1 -> m.flags |= 0x80 and a counter at 0x0004d7e0()+0x58 += 1   ; 0x0011afc0-0x0011b108
```

- **The contact law is a sphere of radius `trigger_radius = 3.0`** around the mine's position
  against the craft's weapon-slot vector `s+0x90` (read as the craft's position, confidence 60
  for which point that is); the per-axis test before it is the sphere's bounding-box early-out
  and changes nothing. Confidence **80**.
- **Owner grace**: `BaseMine_IsPastOwnerGrace` (`0x0010ea48`) is `age > 0.5` (`0.5` at TOC
  `-0x3a68`). Every Detonator mine is owned by slot `0`, the player, so **a fresh mine cannot hit
  the player for its first 0.5 s**. Confidence **78**.
- `0x0012d7e0(s)` is `s+0xf0 != 0 && (role == 0 || role == 2)` (role `craft+0x628c`, the person
  and an opponent, [eliminator-score.md](eliminator-score.md)). Confidence 75.
- **The contact darkens the mine before it explodes**: slot `+0x3c` is `DetonatorMine_SetDark`,
  so a mine a craft runs into always explodes as `WO_MINE_EXPLO_DETONATOR_DEAD` (below).
  Confidence **72**: the call and the slot are read; no frame has been compared.
- The wave probe at the end runs for every mine every tick; `g_quake` is the Quake's wave state
  ([quake.md](quake.md)). Detonator builds no `QuakeManager` ([weapons.md](weapons.md)); whether
  the EMP drives the same wave (`WO_QUAKE_DETONATOR` is spawned in `EMP.cpp`'s span,
  [particle-triggers.md](particle-triggers.md)) is unread. Hypothesis, confidence 45. **Answered
  2026-11-24: it does not.** No function of `EMP.cpp`'s span loads `g_quake`'s pointer; the EMP walks
  its own cursor and kills mines through `MineManager_DestroyAtLane` ([detonator-emp.md](detonator-emp.md#the-wave-a-repulser-wave-that-clears-mines)).

## `MineManager_ApplyBlast(mgr, i, k)` (`0x0011aa88`)

```text
s := mgr.craftSlots[k]
if role(owner craft of m) in {0, 2}: s+0x124 = 1               ; 0x0011aabc-0x0011ab00
d := s+0x50 - p_m;  u := d / |d| (zero-guarded)                 ; 0x0011ab14-0x0011abe0
s+0x120 += Mine.damage                                          ; 0x0011abd0-0x0011abe4
s+0x110 += u * (1 - |d| / Mine.blastradius) * Mine.blastforce   ; 0x0011abe8-0x0011ac24
s+0x138  = 4 (hit kind: mine);  s+0x13c = m.owner               ; 0x0011ac08-0x0011ac1c
s+0x130 += Mine.slowdown_time                                   ; 0x0011ac28-0x0011ac34
if byte 0x009384e1 == 0 and g_GameState.mode == 14:             ; 0x0011ac38-0x0011ac48
    s+0x140 = Mine.velocity_reduction                           ; 0x0011ac68-0x0011ac84
```

`s` is the craft's pickup slot (`*(ship+0x5edc)`, [autopilot.md](autopilot.md)): the same object
whose `+0x138`/`+0x13c` [eliminator-score.md](eliminator-score.md) reads as the hit kind and the
attacker, and whose `+0x120` `Ship_ApplyPendingDamage` spends. The other weapons' hits fill the
same fields (`0x00119108`, a `LightBarrier` hit: kind `10`, its own damage and slowdown).
Confidence **80** for the law; **70** that `s+0x50` is the point the push direction is taken
from. With the disc's `blastforce="0"` and `slowdown_time="0.0"` **a Detonator mine pushes
nothing and slows nothing**; its effect on the craft is the damage and the velocity cut.

## What the craft does with it

- **Damage**: `Ship_ApplyPendingDamage` (`0x000e8de0`, [particle-triggers.md](particle-triggers.md))
  runs when `s+0x120 > 0`: for a racing person craft (`ship+0x5fa4 == 1`) without the shield bit
  (`s+0x200 & 0x10`) it calls `Ship_ApplyDamage(ship, s+0x120, source 2, kind s+0x138, s+0x124)`
  (`0x000e8f70`-`0x000e8f80`), then clears `s+0x120` (`0x000e8ee4`-`0x000e8ef4`). Confidence 72
  (the shielded and network arms around it are not read).
- **`Ship_ApplyDamage` (`0x000e7760`)** returns at once for a craft in state `2`, `4`, `5` or `6`,
  with `+0x6a64 & 0x1000`, or with `+0x6a84 > 0.0`; **halves the blow when `g_GameState+0x18` is
  `0`** (`0.5` at TOC `-0x49d4`; Pulse's `Ship_Damage` has the same halving for a weapons-off race,
  [contact-response.md](../psp-pulse-usa/contact-response.md); confidence 60 that `+0x18` is
  HD's weapons flag, and Detonator's campaign cells author `Weapons="on"`); and subtracts it
  from the energy pool `ship+0x5fa0`. Confidence 75.
- **Destroyed**: the zero-energy branch calls `Ship_SetState(ship, 4)` (`0x000e8178`,
  `0x000e8354`, `0x000e885c`: `li r4,4; bl 0x000e1e20`). `Ship_SetState` (`0x000e1e20`) jumps on
  the state through the ten-word table at `0x000e1eec` (TOC `-0x44e4`); state `4` lands at
  `0x000e20e8`, counts the death (`+0x6f78`), and for mode 14 (`cmpwi cr2, mode, 13`, `bgt` at
  `0x000e2178` to `0x000e29e0`, which falls back to `0x000e2184`) takes the common path that
  ends in **`ship+0x6a64 |= 0x1000`** (`0x000e2418`-`0x000e2420`, reached from `0x000e23f0`,
  `0x000e29c8`, `0x000e2a7c`, `0x000e2a88`, `0x000e2abc`). State `7` sets the same bit at
  `0x000e2784`. `Detonator_UpdateRace` ends the run on that bit
  ([detonator-mode.md](detonator-mode.md)). Confidence **72** for the chain (every link read,
  not every branch of `Ship_SetState`'s state-4 path).
- **Velocity cut**: `Ship_UpdateEntity` (`0x000eadb8`) at `0x000eba90`-`0x000ebacc`: when
  `s+0x140 > 0.0`, the body's velocity (`*(ship+0x6944)+0x190`, [physics.md](physics.md)) is
  multiplied by it and `s+0x140` is cleared. **One mine contact halves the craft's velocity
  vector once** (`0.5`), shield or not (no shield test on this branch). Just above
  (`0x000eba54`-`0x000eba8c`), `s+0x130 > 0` calls `0x000ed540` unless shielded, then clears it:
  the slowdown, `0` here. Confidence **78**.

## `DetonatorMine_Explode(m, &pos)` (`0x00138e58`)

```text
0x00677048(*(TOC -0x2bc4), 0x30, pos)                    ; unread (a shake or rumble request?)
rm.entries[m+0xc6].laneArray[m+0xc8] = 3                 ; the lane is free again
if !DetonatorMine_IsLive(m):                             ; vtable +0x40
    spawn "WO_MINE_EXPLO_DETONATOR_DEAD" (TOC -0x2ba0), fourcc 'MIED' (0x4445494d), at pos
else:
    spawn "WO_MINE_EXPLO_DETONATOR" (TOC -0x2bb0), same fourcc, at pos
    recolour the blast just spawned: see "The blast's colour" (2026-11-26)
sound "MINEEXPL" (TOC -0x2b98) at volume 1.0 (TOC -0x2b90), positioned
```

Both effects go through `0x002c9108` with the race manager as parent, the call shape
[particle-triggers.md](particle-triggers.md) reads for every HD weapon effect. Confidence **82**
for which name fires when; **85** for the recolour, read to its definitions in "The blast's colour" below.

**When each fires**:

| Effect | Fires when | Confidence |
| --- | --- | ---: |
| `WO_MINE_EXPLO_DETONATOR_DEAD` | a triggered mine explodes (and is in some view's PVS) after it was darkened: every mine a craft runs into, and every mine laid on an earlier stage (the manual's "dark mines") | 72 |
| `WO_MINE_EXPLO_DETONATOR` | a live mine (laid this stage) detonated without contact: `MineManager_DestroyAtLane`, the EMP's path, below | 70 |

## Wired: the explosions (2026-11-22)

A mine a craft runs into plays `WO_MINE_EXPLO_DETONATOR_DEAD` (the contact darkens it first);
a mine a cannon round hits, or a destroyed bomb's chain reaction sets off, plays
`WO_MINE_EXPLO_DETONATOR` while it was laid this stage and `_DEAD` after
(`oag_weapons::detonator::EventKind`, `oag_raceplay::detonator_fx::effect_for`). The disc's own
`.pob`. Since 2026-11-26 their camera-facing emitters draw the disc's own sprites, their streaks
the procedural strip
([particle-fill.md](../../../rendering/particle-fill.md#which-effects-draw-a-sprite)); the live one's shards are paired against
the original's frame below. Chosen: a shot mine explodes the tick it is hit (the original
the next), the effect plays whether or not the mine is in view, and the recolour toward white
(`lerp(c, 255, 0.8)`, 55) is not applied. `MINEEXPL` plays with it, at volume 1.0 where the mine was (`Cue::MineExplosion`, see
[detonator-cannon.md](detonator-cannon.md), "Drawn and heard"). Test:
`tests::detonator_fx::an_hd_detonator_race_plays_the_discs_effects_for_each_event`.
**2048/Omega: checked, applies, not wired** - the effects ship and parse
(`psys_2048_ground_truth.rs`, `psys_omega_ground_truth.rs`); the contact law is not read there.

## The shards, against the original (2026-11-26, `det-mine-blast`)

**Claim checked:** the original's frame of a mine a cannon round destroyed (the `cannon-volley` pair, Talon's Junction,
its published RPCS3 grab) shows small glass shards, a dim pink puff and a `+100`; ours drew white puffs
and long dotted streaks. **The shards are the disc's**: `wo_mine_explo_detonator.pob` (`DATA00` `/data/psys/`) has six
emitters, and its `Debris` is 50 alpha-over billboards, life `152 +- 54` ticks, radial at `1.31 +- 0.76` units a tick,
gravity `0.01`, size `0.3` to `0.8`, picking a random frame of a 512 x 512, 4 x 4 atlas of glass triangles. The other
five: the root (3 additive glows, size 7.5 shrinking to 0 over `74 +- 15` ticks, each with a `Trail_Ribbon` child),
`streaks_fat` (10 additive streaks, `12 +- 3` ticks), `sparks` (10 thin streaks, `113 +- 29` ticks) and `Explosion_Core`
(3 additive billboards off a 1024 x 1024, 8 x 8 smoke atlas, 43 ticks). `_DEAD` has three: the same debris as its root,
and black `streaks_fat` and `sparks`. Every emitter's `.gtf` decodes. Confidence **85** that `Debris` is what the frame
shows (the atlas's shapes and the shards' size and spread on the frame agree; one frame).

**Wired:** both mine effects are in HD's sprite list (`oag_hd` `SPRITES`, with HD's other weapon explosions,
[particle-fill.md](../../../rendering/particle-fill.md#which-effects-draw-a-sprite)), so ours draws the disc's pictures
where it drew the procedural profile. Test: `check_a_shot_mine_throws_the_discs_shards` in
`tests::detonator_fx::an_hd_detonator_race_plays_the_discs_effects_for_each_event`.

**The blast's age.** In the original the round (fired at lap 1.917 s) retired at age 10 ticks (the grab's cannon reads at
each pause, [compare-page.md](../../../tools/compare-page.md), "The volley clocks": its `flags` read `5` from lap 2.083 s on), and the picture is 6.35 ticks older than the 2.65 s pause, so the blast is
about 27 ticks old in the frame. Ours' round, fired from the craft held at the picture's pose, hits that mine 1 tick
after the volley (tick 389 of 425), so ours' blast is 36 ticks old. Putting the round back where the original's moving
craft fired it (`102.26` units/s times its age, and `3.6 v` on the launch tick) makes ours hit at tick 396, 29 ticks
before the picture.

**Mean grey on `cannon-volley`** (published 11.89, the destroyed mine not carried):

| Ours | mean grey |
| --- | ---: |
| the destroyed mine carried (position `217.595, -44.266, -192.347`, words and age `54.28 s` from the grab's pause read), procedural pictures | 12.70 |
| the same, the disc's sprites | 12.53 |
| the same, and the round carried back by the craft's speed | 13.75 |
| the destroyed mine carried, after the HD weapon explosions' flipbook and size-ramp work (sprites listed, merged 2026-11-26) | 12.31 |

So the sprites lower the carried pair by 0.17, but the pair with the destroyed mine stays above the published 11.89,
and the speed carry raises it: the younger blast shows more of what ours still draws wrong. What is left:

- **The root's three glows and their `Trail_Ribbon`s.** Ours draws each glow as a large white disc and each ribbon as a
  chain of dots (the ribbon's particles have speed 0, a `Streak { from_spawn }` of zero length per tick). The original's
  frame shows neither the discs nor the dots, only a few thin lines. Unread: how HD draws a speed-0 streak child, and
  whether the root's glow is drawn at all. Drawing the effects' streaks as HD's far bar (`FAR_BARS`) turns the dots into
  thin lines, but brighter and more of them than the original's, and the carried pair goes 12.53 to 12.92: not kept.
- **`Explosion_Core`.** The original's is a dim, translucent pink smoke; ours was near-white. **Read and wired
  2026-11-26**: the whole blast is recoloured from the stage's mine colour ("The blast's colour" below).
- **The `+100`**: a HUD widget, not an effect ([detonator-hud.md](detonator-hud.md), "The floating score").

The destroyed mine is therefore **not carried** in the published pair, and the craft-speed carry is not kept.

**2048 and Omega: checked, applies, not wired** (the brief defers their Detonator lanes). Both mine effects ship with
HD's emitter trees (the same six and three emitters, counts, blends, classes and lives; the `.pob`s name
`Tex\Debris_DetonatorMines.tga`), and the shard atlas ships as `data/particles{,2048}/Tex/Debris_DetonatorMines.gxt` in
2048's `data.psarc` and `.gnf` in Omega's base archive (census run on `psys_2048_ground_truth.rs` and
`psys_omega_ground_truth.rs`'s archive walks, 2026-11-26). Neither title lists its effects' sprites yet
(`Effects::sprites` is HD's alone), so wiring them is a decode check of the `.gxt`/`.gnf` atlas plus the list entry.

## The blast's colour (2026-11-26, `det-mine-blast-glow`)

**Claim:** a live mine's blast (`WO_MINE_EXPLO_DETONATOR`) is not drawn in its authored white.
`DetonatorMine_Explode` overwrites every colour table of the tree it has just spawned with one
entry made from the stage's `Detonator Mine Colour`. Static, `EBOOT.elf`, and 2048's twin; confidence **85**.

| Step | Address | Read |
| --- | --- | --- |
| The blast's handle | `0x00139028`-`0x0013903c` | `Psys_FindInstanceByHandle(*(*(TOC -0x2bac)) + 0x468c, *(*(TOC -0x2ba8)))`: TOC `-0x2bac` holds `0x00ad5fd8` (the particle manager, whose `+0x468c` is its instance store, written by `0x0027ff58`/`0x00280290`), TOC `-0x2ba8` holds `0x00ad5fd4` |
| `0x00ad5fd4` is the last root instance made | `0x0027f610` | `Psys_CreateInstance`: for an instance with no parent it stores the new handle (`instance+0x18`) at `*(r2+0x5428) + 0x3418` = `0x00ad2bbc + 0x3418` = `0x00ad5fd4`. The spawn `0x002c9108` -> `0x002c8c70` -> `0x0027f610` runs on the line before, so the handle is the blast's own |
| The colour | `0x00139050`-`0x0013905c` | `Environment_GetStageMineColour` (through the stub `0x00677cf8`) for the player craft's `+0x6284`: the committed stage's `Detonator Mine Colour`, word `r << 24 \| g << 16 \| b << 8 \| a` ([detonator-mine-material.md](detonator-mine-material.md)) |
| Toward white | `0x001390c8`-`0x001390e8` | per channel `fmadds (255 - c) * 0.8 + c`, `fctidz` (truncate), clamped to `255` (255.0 at TOC `-0x2b94`, 0.8 at `-0x2ba4`) |
| The packing | `0x001390ac`, `0x00139110`, `0x00139138`, `0x00139160` | `a << 24 \| b' << 16 \| g' << 9 \| r' << 1` (`rlwinm` 24/0-7, 16/0-15, 1/0-30, 9/0-22) |
| The write | `0x0013916c`-`0x0013918c` | the word into all 256 entries at `instance+0x28` (the root record) `+0xc4` |
| To the whole tree | `0x00139194` -> `0x002834f8` | `Psys_CopyPaletteToTree`: copies the record's 1024 bytes at `+0xc4` into every instance reached by `+0x24` (child) and `+0x1b4` (sibling), recursively |

HD's tables are read as bytes `a, b, g, r` ([pob.md](../../../formats/pob.md), "The one field that is not a plain
re-read"), so the shifts of `9` and `1` (not `8` and `0`) put green and red one bit high: red keeps its low seven bits,
green takes red's top bit, blue takes green's. With every channel at `204` or more after the lerp, the drawn entry is
`r = 2 r' - 256`, `g = (2 g' - 256) | 1`, `b = b' | 1`. On the opening stage (`1`, `Sub Venom`, `122 25 255`) that is
**`200 163 255`**, a lavender. The blast's six emitters (root glows, `Trail_Ribbon`, `Debris`, `streaks_fat`, `sparks`,
`Explosion_Core`) are all mode-0 walks of their tables, so all six draw in that one colour; the dark mine's blast
(`_DEAD`) is not recoloured. The table's alpha byte is the authored colour's, and HD never reads it.

Why 85: static, every step read to its definition, and 2048's own explode carries the same law with the handle taken
from the spawn directly; no live palette dump.

**Wired:** `oag_title::Effects::mine_blast_recolour` (HD, measured); `oag_fx::psys::detonator_mine_blast_entry`
(the packing) and `Effect::with_one_colour`; `oag_raceplay::detonator_fx::mine_blasts` builds one recoloured blast per
stage at race start, and a shot, chained or EMP'd live mine plays the one for the Detonator stage the lap counter shows.
That stage is `oag_race::Mode::detonator_stage(laps)`, not the colour grade's committed stage: on a race whose grade
is pinned to another stage (`ZoneGrade::pin_stage`) the blast and the mine bodies would take different stages' colours.
No published pair pins a stage.
Tests: `a_live_mine_blast_entry_is_the_executables_packing_not_a_plain_lerp` and
`one_colour_fills_every_emitters_whole_table` (`oag-fx`); `check_a_live_mine_blast_takes_the_stage_colour` in
`tests::detonator_fx::an_hd_detonator_race_plays_the_discs_effects_for_each_event` (HD image).

**Against the original** (`cannon-volley` with the destroyed mine carried, the `det-mine-blast` setup above):

| Ours | mean grey |
| --- | ---: |
| before (main, 2026-11-26) | 12.31 |
| the blast recoloured | **12.12** |
| the same, `Trail_Ribbon` muted (a diagnostic, not wired) | 11.83 |
| the same, the root's glows muted (diagnostic) | 12.12 |
| recoloured, the effect's class 6 drawn as HD's far bar (`FAR_BARS`) | 12.41 |
| recoloured, the root's noise force (modifier `0x11`, `Effects::turbulence`) on | 12.15 |
| recoloured, and the round carried back by the craft's speed (`102.26` units/s, the prototype above) | 12.98 |

The other five Detonator pairs and two Rocket craft-hit pairs did not move (none plays a live mine's blast). So the
core is the original's colour now. Muting the root's glows had no measured effect in this frame (blast about 36 ticks
old), but their draw law is unread and a younger blast would show them larger. And **what keeps the pair above its
published 11.89 is the `Trail_Ribbon` dots** (+0.29). The far bar and the noise force both make the pair worse, so neither
is wired.

**The ribbon, open.** Each root particle carries a `Trail_Ribbon` child instance (`Psys_InitParticle` `0x00289890`:
the child spawns with probability `child+0x4d4` = 1, its instance stores the parent particle at `+0x140`, the parent's
position at `+0x130`, the parent's velocity times `child+0x4d0` (0.01) at `+0x160`, and sets `+0x170 = 1`). The ribbon
emits one speed-0 particle a tick, mode 5, flags `0x22000000`. As read so far, a mode-5 particle's second point is its own
previous position (`Psys_UpdateParticles` `0x0028cc10` copies `+0x20` to `+0x30` each tick under `ctx+8`), so each
ribbon particle would be a short bar of zero length, which is ours' dot. The original's frame shows thin lines instead.
Not read: who reads the child instance's `+0x130`/`+0x140`/`+0x170` (the instance update `0x00288d98` does not touch
`+0x130`), and what flag `0x20000000` without `0x4000` gates. Next: one RPCS3 capture of the blast's particle quads a
few ticks after a hit ([particle-render.md](particle-render.md), "Reproducing").

**Names** (`names.tsv`):

| Address | Kind | Name | Confidence | Why |
| --- | --- | --- | ---: | --- |
| `0x002834f8` | function | `Psys_CopyPaletteToTree` | 85 | copies `+0xc4`'s 1024 bytes down `+0x24`/`+0x1b4`, recursing; nothing else |
| `0x0027f610` | function | `Psys_CreateInstance` | 70 | allocates `0x1d0` bytes, constructs, links into the manager's list, stores a root's handle; the child-flag tests at its head are unread |
| `0x00ad5fd4` | data | `Psys_LastRootHandle` | 75 | written by `Psys_CreateInstance` for a parentless instance, read here; Ghidra lists a dozen more readers among the effect spawners (`EMP_SpawnLaunchEffect`, `Trail_HitShipEffect`), not followed |

**2048: checked, applies, not wired.** Its mine explode (`FUN_811e6ebc`, `eboot-vita-2048-eu-v104.elf`) spawns the
same two names with the same fourcc and recolours the live one with the same law: `trunc(c + (255 - c) * 0.8)`
clamped to 255, packed `a << 24 | b' << 16 | g' << 9 | r' << 1` into all 256 words at `instance+0x28` `+0xc4`, then
`FUN_812aa676`. It takes the instance by the spawn node's own handle (`node+0xec`, the field `0x002c8c70` fills on HD),
which confirms independently that the recoloured instance is the blast just spawned. The colour comes from
`FUN_81023fc4` on the player craft's `+0x58d4` (2048's stage colour getter, not followed). 2048's table is
little-endian, so the same word reads `r, g, b, a` from its low byte up; it draws HD's colour if that getter packs
red in the word's top byte as HD's does (not read). Not wired: 2048 answers no mine trigger yet (its Detonator lane is
deferred).

**Omega: checked, applies, not wired.** `FUN_0133ac10` (patch 1.07, the function holding `0x0133ad5f`) spawns the same
two systems (looked up by the CRC-32 of their names) and, for a live mine, takes the spawned instance by its own handle
(`+0xc0`), reads the committed stage's colour word (stage table stride `0x250`, the craft's `+0x68f4`), applies
`(int)((255 - c) * 0.8 + c)` clamped to 255 and writes `g' << 9 | b' << 16 | r' * 2 | a << 24` into all 256 entries at
`record+0xc4` (the record at `instance+0x90`), then `FUN_016e03a0` (the tree copy, not opened). Same law; not wired, as
Omega answers no mine trigger yet.

## Carrying the destroyed mine (2026-11-26, `det-mine-ribbon`)

**Why the pair lost it.** `rpcs3-hd-pair.py` carried only the armed mines (`flags & 9 == 9`). The mine the
`cannon-volley` round hit (cv1 grab, `ship.mines[1]`, `0x306da430`, entry 34, lane 2, position
`217.595, -44.266, -192.347`) reads `flags` `1` at the pause: the hit sets `0x80`, `BaseMine_Update` turns that into
`4` and drops `8`, and `MineManager_Update`'s compaction clears `0x8c` (above), so it has left the pool by the pause.
A mine gone long before reads the same `1`. What tells them apart is the age (`+0xc0`): this one's advanced through the
shot's own mine reads (`52.40 s` at lap clock 0.233 s) and stopped at `54.2836 s` from lap 2.217 s on, while the 79
armed mines read `54.82 s` at the 2.65 s pause. So it left the pool about 0.5 s before the pause, at the round's hit.
Without it ours' replayed round hits nothing: no blast, and the score reads 0 where the original's reads 100.

**Rule** (`compare_env.detonator_pair_mines`, used by `mine_flags`): carry the armed mines, plus any `flags & 1` mine whose
age at the pause is above its age in the shot's first mine read. Its envelope and age are the picture-time read's,
like the others' (`0.3313, 0.6430, 8.5001`, age `54.2836`, frozen since the hit). Of the other Detonator grabs
(`hot2`, `shots6`, `gC`) none holds a non-armed mine, so only `cannon-volley` changes.

**Measured** (ours re-rendered on scratch copies of the pairs, the same binary twice, byte-identical):

| `cannon-volley` | mean grey |
| --- | ---: |
| published, the destroyed mine not carried | 10.80 |
| **the destroyed mine carried (adopted)** | **10.53** |
| carried, `Trail_Ribbon` muted (diagnostic) | 10.22 |
| carried, `Trail_Ribbon` and the root glows muted (diagnostic) | 10.23 |

The other five Detonator pairs read the same before and after (`-hot` 7.70, `endrace-norecord` 8.88, `endrace-record`
8.38, `mines-ahead` 9.68, `pad` 4.44). With the mine carried ours shows the blast and `0000100`, as the original does.
The ribbon is still worth 0.31, but even a perfect ribbon leaves the pair above 10. Its segment law stays open ("The
ribbon, open" above).

**2048 and Omega: not checkable.** This is a capture input of the HD RPCS3 pair script; neither title has a Detonator
pair with mine objects read from memory.

## Live and dark mines

- `DetonatorMine_Place` (`0x00139da0`, vtable `+0x44`, reached through `BaseMine_PlaceAt`
  `0x0010e968` from `Detonator_SpawnObject`) writes `+0x160 = 1`, life and age `0`, sweeps a
  segment down to the track (`Collision_SweepSegment`, `0x00139ed0`) and, on most return codes,
  re-aligns the mine's up row to the hit normal (`0x0013a0b0`-`0x0013a230`), then ORs `8` into the
  flags and `4` into `+0x34`. Confidence 70 (codes `0`, `4`, `5`, `127` skip the re-align).
- On every stage step the placer calls `MineManager_DarkenAll` (`0x0011a360`, every live mine's
  slot `+0x3c`) while `DoMusicResponsePreviousStageMines` is `0` (the disc's value,
  [detonator-placement.md](detonator-placement.md)). So **a mine is live for the stage it is
  laid in and dark afterwards**, which is 2048's manual: "unexploded mines from previous laps
  will persist and appear as dark mines". Confidence **80**.
- `RemoveMinesEachStage` (disc `0`) would call `0x0012ae38`, which runs `MineManager_RemoveAll`
  (`0x0011a418`: every live mine `flags |= 4`, so it leaves without exploding) and
  `BombManager_DetonateAll`. Confidence 75.

## `MineManager_DestroyAtLane(mgr, entry, lane)` (`0x0011a590`)

Finds the live mine whose `+0xc6`/`+0xc8` match, sets `0x80`, passes its "is live" answer and its
position (`0x0010ea28`, `m+0x80`) to `0x00082b48` (a score add: a multiplier record, `score += int * float` when live,
a flat amount when dark, through the race manager's vtable `+0x3c` and `Hud_RaiseScoreFeedback`
`0x00083ba0`, [eliminator-score.md](eliminator-score.md)), and adds one to `rm+0x6de3c`, the
objects-destroyed count [detonator-mode.md](detonator-mode.md) reads for the end-of-run
percentage. Its two callers (`0x0013c0d4`, `0x0013c290`) sit in `EMP.cpp`'s span beside the
bomb's twin, so this is the EMP wave's kill. No darkening, so a live mine killed this way fires
the coloured effect. Confidence **72** for the function, **55** for the score reading (the
manual's "take mines out in the first lap they appear for extra points" fits the live branch;
the amounts are the Detonator-cannon phase's).

## What the simulation needs

| Quantity | Value | Source | Confidence |
| --- | --- | --- | ---: |
| Contact | sphere, `|craft - mine| < trigger_radius` | `0x0011ade4`-`0x0011ae98` | 80 |
| `trigger_radius` | `3.0` | `weaponstats_detonator.xml` Mine | 88 |
| Owner grace | the player is immune to a mine younger than `0.5` s | `0x0010ea48` | 78 |
| Damage | `10` energy, halved when the weapons flag is off | `0x0011abd0`, `0x000e7760` | 75 |
| Push | none (`blastforce 0`); the law is `u * (1 - d/30) * blastforce` | `0x0011abe8` | 80 |
| Velocity | multiplied by `0.5` once, Detonator only | `0x000eba98` | 78 |
| Slowdown | none (`slowdown_time 0.0`) | `0x0011ac28` | 80 |
| Lifetime | none in Detonator (never times out) | `0x0011b1f0`-`0x0011b204` | 80 |
| One hit per mine | armed bit cleared at the explosion | `0x0010edb8` | 80 |
| Leaves the pool | tick after contact; lane freed if visible | `0x0010ed7c`, `0x00138e58` | 75 |
| Effect on contact | `WO_MINE_EXPLO_DETONATOR_DEAD`, `MINEEXPL` | above | 72 |
| Destroyed | energy at `0` -> state 4 -> `ship+0x6a64 |= 0x1000` -> race over | `0x000e2418` | 72 |

Dark mines hit exactly as live ones: the contact test reads no `+0x160`. Confidence 80.

## Open

- **Points**: read since on [detonator-cannon.md](detonator-cannon.md): the score record is
  `g_DetonatorScoring` (`DetonatorScoring.xml`), `0x00082b48` is `DetonatorScoring_AddEmpMine`,
  and the cannon kills a mine in one hit through `CannonManager_TestMineBombHits` (`0x0010ffd0`).
  `points_per_metre`, `max_distance` and `PointsForChainReactionMines` still have no reader.
- **Which point of the craft** `s+0x90` (contact) and `s+0x50` (push direction) are.
- **The order** of the weapon managers' update against `Ship_UpdateEntity` and
  `Ship_ApplyPendingDamage` within a tick: the damage and the velocity cut land on the craft's
  next update after the contact, whether that is the same tick is unread.
- `0x00677048` (both explosions call it), the recolour's sources, `0x00126c38`'s owner test,
  the wave probe's role in Detonator.

## Where a mine stands, and what a laid mine object holds (2026-11-23, `det-mines`, RPCS3)

**Read live**, not from the disassembly: 80 `DetonatorMine` objects (vtable `0x008642f0`, found by a heap scan,
`rpcs3_hd_field.detonator_mines`) of an HD Detonator race on Talon's Junction, Feisar, stage 1, paused at lap clock
2.7 s and again at 3.4 s (two boots, a third with the objects' bytes kept). A laid object (`0x170` bytes):

| Offset | Holds | Seen |
| --- | --- | --- |
| `+0x00` | vtable | `0x008642f0` |
| `+0x40` | flags | `9` (active, armed) on a laid mine; `1` once it left the pool (the craft hit one by 3.4 s: 79 at `9`, one at `1`, `+0x160` read `0`) |
| `+0x50`, `+0x60`, `+0x70` | the basis, rows `(0.9993, 0.036, ..)`, `(-0.036, 0.986, 0.165)`, `(0.006, -0.165, 0.986)` | the up row is the section's up, tilted with the track |
| `+0x80` | position | static: 80 of 80 identical at 2.7 s and 3.4 s, and identical across boots |
| `+0x90` | velocity `(99.86, 2.82, 4.58)`, `+0x9c` its length `100.0` | `sec.v10 * 100` of the placer; the position does not move (above) |
| `+0xc0` | age, s | 58.37 at a world clock of 56.5 (not the lap clock) |
| `+0xc6` / `+0xc8` | track entry (u16) / lane index `j` (u8) | `j = lanes / 2 + lane` |
| `+0xd8`, `+0xf4`, `+0xf8`, `+0x100` | football size, envelope `s`, envelope `t`, `MineBrightness` | `1.3537`, `0.5886`, `0.568`, `1.1771`: `2.3 s = 1.3538`, `2 s = 1.1772` exactly as `detonator-mine-material.md` writes them |
| `+0x138` | `MineColour` | `(0.4784, 0.098, 1.0)` = `(122, 25, 255) / 255`, stage 1's `Detonator Mine Colour` in file order |
| `+0x154`, `+0x158` | band indices | the football's and its `+ 11` |
| `+0x160` | live | `0x01000000` (a byte `1`) |

**Height.** The object's position sits **3.00 along the section's up above** the point
`oag_weapons::detonator::Survey::point` gives for its entry and lane (70 of 80 within 0.1; the other ten 2.5 to 2.9 on
banked or sloped sections, where the object's up is the ground normal and ours the section's), and **0.06 across the
section on average (0.47 at most)** on all 80, so the lane lattice, `3.5` a lane, is the original's to the unit. That
is `oag_weapons::detonator::MINE_LIFT`; draw, contact (`trigger_radius 3`, from the object's position), the cannon
round's capsule test and the EMP's event point read it through `Survey::mine_point`. A bomb stays on `point` (unmeasured).
Confidence 75: the offset is measured on one circuit and one stage; why it is `3.0` is not read (a `Collision_SweepSegment`
down to the track in `DetonatorMine_Place`, `0x00139ed0`, is the candidate, which would also explain the ten that differ).

**Placement is deterministic from the state.** The same 80 `(entry, lane)` addresses and positions came out of three
boots of the same save state (`Libc_Rand`'s stream is seeded the same), so one read stands for a pair's frames at any
lap clock of that state. **The entry number is the original's, not ours':** its survey starts at the player's start
cursor (`Detonator_PlaceStageObjects`, "Walk", `detonator-placement.md`), ours at the spline's origin, **7 entries
apart on Talon's Junction** (original entry `n` is ours `n - 7`), so a carried mine is matched by position
(`RaceSim::carry_detonator_mines`).

**Open: the near mines are drawn further away than their objects.** In `mines-ahead` (lap 2.67 s) the three mines 12 to 18
units ahead of the camera project, with the pair's camera, 30 % further from the screen centre than the original draws
them (`666, 1075, 1703` against `770, 1040, 1490` across), as if 7 units further down the view; the far mines (185 to 208
units) and, in `pad` (2.0 s), all four near mines at 83 to 89 units, project to within 4 px of where the original draws
them, and the scenery's red strips agree to 8 to 25 px. In `cannon-volley` (2.63 s) the factor is 0.87. It is not time,
not a model offset (both models are unit spheres about the origin) and not the objects' motion. Undetermined.

**2048/Omega:** not checked (the object layout is HD's `DetonatorMine`; 2048 and Omega place with the same files,
`detonator-placement.md`). **Checked, applies, not wired**: the height and the carry are HD's.

## The near-mine displacement is the pair's camera (2026-11-24, `det-mine-look-re`)

The "Open" line above is answered on [detonator-mine-draw.md](detonator-mine-draw.md): the mine is drawn at its object's
position (`m+0x80`, no view term), and the pair's push-buffer camera is 1.5 to 4 ticks of travel newer than the picture
(`mines-ahead` 3.55, `cannon-volley` 2.30, `pad` 1.55, fitted on the near mines; one cross-track scenery edge agrees on
x). Confidence 70: not reconciled with the earlier lag measurements (that page, "The displacement").

## Cross-check: 2048 and Omega

- **Weapon table**: `weaponstats_Detonator.xml` (2048 v1.04 `data1.psarc`) and
  `Data/xml/weaponstats_detonator.xml` (Omega `data00.psarc`) author the same `Global`, `Mine`
  and `Bomb` values as HD's, attribute for attribute (read 2026-11-22). **Checked, applies, not
  wired.**
- **Effects**: all three `.pob` ship on all three titles (HD `DATA00` `/data/psys/`, 2048
  `data.psarc` `data/particles/` and `data/particles2048/`, Omega `data00.psarc` likewise), and
  every executable names `WO_MINE_EXPLO_DETONATOR`, `_DEAD`, `WO_BOMB_EXPLO_DETONATOR`,
  `DetonatorMine.cpp`, `trigger_radius`, `points_per_metre`, `velocity_reduction`.
- **2048 v1.04** (`eboot.elf`, `llvm-objdump --triple=thumbv7-none-eabi`): the explode's shape at
  `0x811e6f20`-`0x811e6fe4`: the lane set to `3` (`movs r5,#3; str r5,[r1,r3]`), a virtual call
  through vtable `+0x34` returning a byte, `0` -> `WO_MINE_EXPLO_DETONATOR_DEAD`
  (`0x81499ee0`), non-zero -> `WO_MINE_EXPLO_DETONATOR` (`0x81499ec8`), both with HD's fourcc
  `0x4445494d`. **Checked, applies, not wired**, confidence 70. The contact test, the blast and
  the velocity cut were not located.
- **Omega** (`eboot.bin`, segment 0 at image base `0x01000000`): `0x0133acba` calls vtable
  `+0x68` (8-byte slots), and `test al; je 0x133b02b` picks `WO_MINE_EXPLO_DETONATOR_DEAD`
  (hashed at `0x0133b280`) over `WO_MINE_EXPLO_DETONATOR` (`0x0133afc0`); Omega CRC-hashes the
  effect name (table `0x020fc850`) where HD passes the string. The `Mine` parser stores
  `velocity_reduction` at `+0x1a8` (`0x0165d14c`); its reader was not found. **Checked, applies,
  not wired** for the effect selector (confidence 60); the contact and damage law **not read**
  on Omega.
