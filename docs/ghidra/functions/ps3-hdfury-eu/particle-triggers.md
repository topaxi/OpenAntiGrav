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
`data/scratch/hd-particle-triggers/toc_scan.txt` (scratch, not committed). A name
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
| `WO_SHIP_COLL_SPARK_NODAMAGE`, `WO_SHIP_SPARK_NODAMAGE_ZONE` | floor/magstrip contact, read, unwired | `ShipCollisionFx_Trigger` kind 2 (zone mask picks the `_ZONE` name); **no caller with kind 2 found**: `Ship_DispatchCollisionFx`'s only direct caller passes 1 | no |
| `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` | a LeachBeam drain's hit (`Ship_Damage`) | **resolved 2026-10-07:** `Ship_ApplyDamage` (`0x000e7760`) sets `object + 0x260` on a kind 7 hit and `0x002a06e0` (also the engine-flare enqueue) clears it and spawns the effect attached at `+0x1c0`, conf 78 | **yes, HD only** (`race::damage_fx`), once per hit |
| `WO_WEAPON_ABSORB` | `Ship_PlayAbsorbFeedback` | `ShipAbsorbNode_SpawnBurst` `0x002d8d10` (`absorb-feedback.md`) | yes (HD path) |
| `WO_TRAIL_HITSHIP`, `_RED` | (HD-only) | `0x002d9ec0` (`engine-trail.md`) | yes |
| `WO_ROCKET_EXPLO` | craft/track hit | `0x0014a7d8` | yes, Pulse-inherited; HD site found, not compared |
| `WO_MISSILE_EXPLO` | detonation | `0x00155568` | yes, Pulse-inherited |
| `WO_MINE_EXPLO` | detonation | `0x00140710` | yes, Pulse-inherited |
| `WO_SHURIKEN_EXPIRE` | teardown | `0x0014d4f0` | yes, Pulse-inherited |
| `WO_REPULSER`, `WO_REPULSER_BLAST` | `Repulser_SpawnWaves` / `_SpawnBlastEffect` | `0x001577d8`, `0x00157b20`, `0x00158288`; `0x00158818` | yes, Pulse-inherited |
| `WO_PLASMA_LIGHTNING_EXPAND/COLLAPSE` | (HD-only) | `0x00127cd0`, `0x00127770` (`plasma.md`) | yes |
| `WO_LEACHBEAM_ABSORB` | (HD-only) | `0x00114a00`/`0x00114c78` (`weapons.md`) | yes |
| `WO_LEACHBEAM_CHARGING`, `WO_BOMB_SMOKERING`, `WO_ROCKET_FLARE`, `WO_ROCKET_EXPLO_TRACK`, `WO_MISSILE_HEAD/BOUNCE`, `WO_SHURIKEN_HEAD/BOUNCE`, `WO_PLASMA_HEAD`, `WO_CANNON_SPARKS`, `WO_QUAKE` | weapon visuals | slot in a weapon data table (third hop); `WO_LEACHBEAM_CHARGING` also loaded at `0x001134f0` | yes, Pulse-inherited, HD site unread |
| `WO_LEACHBEAM_LAUNCH`, `_HIT_TARGET`, `_BREAK` | none | spawn functions named (`weapons.md`, 82/82/78); **no code xref reaches them**: their OPD entries `0x00875a90/80/78` are only referenced from the OPD run itself, so the caller is a vtable or table not found | no |
| `WO_DAMAGE_MILD/MODERATE/CRITICAL` | none | `ShipDamageFx_Update` `0x002a17e8` off a weapon hit; thresholds 70 and 40 read, conf 72 | **yes, HD only** (`race::damage_fx`) |
| `WO_DAMAGE_ELECTRIC`, `WO_SHIP_DEATH_DAMAGE_PLUME` | none | the Zone-mask and `param_4 != 0` branches of the same function, unwired | no |
| `WO_DEBRIS_FIRE` | none | one load, `0x0010c958`, unread | no |
| `WO_DEBRIS_SPARKS`, `WO_NITRO_DEBRIS_SPARKS` | none | table slot, unread | no |
| `WO_BOMB_EXPLO_DETONATOR` `0x00137150`, `WO_BOMB_RAYS` `0x001503d8`, `WO_MINE_EXPLO_DETONATOR(_DEAD)` `0x00138e58`, `WO_QUAKE_DETONATOR` `0x0013afe8/0x0013b5e8/0x0013b930`, `WO_CANNON_SPARKS_DETONATOR`, `WO_LIGHTBARRIER_EXPLO`, `WO_NITRO_SHIP_DEATH` | none | functions found, bodies unread | no: the Detonator weapon set is not built in `oag-weapons` (and `WO_NITRO_SHIP_DEATH` is refused at blend class 4) |
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
`data/scratch/hd-particle-triggers/shots/crop_t42.png` shows the white spark streak
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
meaning unsettled. This port anchors at the craft's origin, **chosen, not
measured**.

**Wired**, `oag_raceplay::damage_fx`, HD only through `Trigger::DamageMild`,
`DamageModerate` and `DamageCritical`. `--force-shield` plus the new
`--force-hit TICK:SLOT` reaches it headless (`data/scratch/hd-particles/shots/`,
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

## Names applied

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x000ddd58` | `Ship_Construct` | 80 | vtable, state zero-fill, skin/absorb-shell/engine-light/node loads, the attached wall spark |
| `0x000dfd90` | `Ship_ConstructAlt` | 65 | the same body with a local copy of `param_6` and a different skin search; it is the twin, not separately identified |
| `0x002a17e8` | `ShipDamageFx_Update` | 72 | the `WO_DAMAGE_*` state switch, thresholds 70 and 40, and its two callers above |
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
  Whether Omega's disc ships that `.pob` was not determined: `scripts/psarc.py list`
  on its `data0*.psarc` returned no `.pob` rows in this run (a listing failure, not a
  result), so the disc side is **not checkable here**.
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

- The caller of kind 2 (`WO_SHIP_COLL_SPARK_NODAMAGE`), and whether the wall
  contact's attached effect is toggled by `+0x5f42`.
- The three `WO_LEACHBEAM_LAUNCH/HIT_TARGET/BREAK` spawners have no code xref; the
  dispatcher behind OPD run `0x00875a40..` is unfound.
- `0x0010c958` is `WO_DEBRIS_FIRE`'s one load, unread. The decrement site of the
  damage hold (`object + 0x240`), the sign of the `1.5` anchor offset, the Zone
  `WO_DAMAGE_ELECTRIC` branch and the death plume (`param_4 != 0`) are unread.
- The wreck trio is still Pulse-gated on HD: `WO_SHIP_FXNODE_EXPLO` and
  `WO_SHIP_DEATH_SPARKS` are on the disc but not strings in the executable, so no
  HD trigger is read for them; they stay unwired rather than ungated on a guess.
- The Zone mode map, which unlocks all three Zone variants at once.
