# The shield pool, and what damages it

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** the pool, its maximum, its one initialiser and its one damage
entry point are read end to end, and **every claim now has a runtime leg**
(2026-08-10, PPSSPP, extended 2026-08-25). Confidence **94** for where the
pool and its maximum live, for the damage coefficient, for the
one-call-per-contact loop and for `entity + 0x368` being `0` on the local
human's craft and `2` on every AI opponent's in a single-player Single Race;
**92** for the two race-option globals and the regeneration branch; **90**
for the weapons-off halving; **80** for `entity + 0x368` being
`Craft_Construct_q`'s own second argument (decompilation only); **75** for
what values `1` and `3` mean, from the constructor's own name formats
(2026-09-16): kind `0` names the craft `"player"` and `1` names it
`"player%d"`, both with the AI byte `+0x48` clear; kinds `2` and `3` name
it `"id%d"` with `+0x48 = 1`. So `1`/`3` are the multiplayer twins of
`0`/`2` - a numbered human and a numbered AI - and under a network mode
(`g_game_mode >= 0xe`) the constructor calls `FUN_0895ebf0` for `1`/`3`
and again for `0`/`2` before naming. Neither value was produced live.

What is measured and what is not is set out under
[the runtime leg](#the-runtime-leg-and-what-it-did-not-reach); the
unmeasured claims cap at **84** per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md).

This page exists because "shield and energy" was the M5 item blocking three
others at once: the HUD's `ShieldBar` had nothing to show,
[Zone](zone-mode.md) had no end condition, and weapon damage had nothing to
subtract from. [`contact-response.md`](contact-response.md) had already read
the call - reaction #2 of `FUN_088418e0`'s contact loop - and left the callee
unnamed pending exactly this pass.

## The pool is `entity + 0x88`, and its maximum is skill-indexed

**The object is the ship entity, not the craft the trace harness breaks on**,
and this page said otherwise for one commit. They are two structures and the
confusion is built into the decompiler's own parameter names: `Ship_Shield`,
`Ship_Damage`, `Ship_SetShield` and `FUN_088418e0` all take the entity, while
`Ship_UpdateCraft` - which `scripts/psp_trace_fields.py` indexes off - takes
the craft. `craft + 0x80..0xbc` is an **orientation matrix**, so a capture
taken at `craft+0x88` records a direction cosine: a smooth run of plausible
floats between -1 and 1, which is exactly what it looked like.

The walk is `entity = *(craft + 0x1c4)`, and it is checkable rather than
assumed: **the entity's `+0x94` points back at the craft**. `Ship_SetShield`
needs that edge anyway, reaching the stats base as
`*(*(entity + 0x94) + 0x6c)`. Measured live: craft `0x09a0c800` -> entity
`0x09a0b840` -> `+0x94` back to `0x09a0c800`, with `entity+0x88` holding a
real pool and `entity+0x8c` holding `1`, the racing state. Both capture
scripts now make and check that walk.

`Ship_Shield` (`0x0883e68c`) and `Ship_SetShield` (`0x0883e6f4`) were named by
the Zone pass on the strength of their call sites. Read in full they settle the
field:

```c
// Ship_SetShield(float amount, ShipEntity *entity)
max = *(float *)(*(int *)(entity + 0x94) + 0x6c) + g_skill_level * 4 + 0x84);
if (amount > max) amount = max;
...
*(float *)(entity + 0x88) = amount;
if (*(int *)(entity + 0x368) == 0)
    Hud_SetEnergyBar((*(float *)(entity + 0x88) / max) * 100.0, DAT_08ab0838);
```

Four things follow, and each is a behaviour rather than a layout note:

- **The pool is clamped above and not below.** `Ship_SetShield` takes
  `min(amount, max)` and stores it. Nothing floors it at zero, which is what
  makes `Ship_Damage`'s `<= 0.0` test below reachable.
- **The HUD reads a percentage, not the raw pool**, and only for the entity
  whose `+0x368` is zero. [`oag_hud`](../../../../crates/hud/src/lib.rs)
  already formats `ShieldBarText` as a percentage; this is the reading that
  makes that right rather than a guess.
- **`entity + 0x94` holds the craft, whose `+0x6c` is the stats base.** The
  same base [`handling-stats.md`](../../../formats/handling-stats.md) accounts
  for with no gap.
- **`Ship_Shield` has a second source in the multiplayer modes.** With
  `DAT_08b31048 >= 0xe` it reads `DAT_08b313dc[entity + 0x364] + 0x180` instead
  - a per-network-player table - and `Ship_SetShield` mirrors into the same
  slot. Nothing in this project reaches those modes; recorded so a future
  reader does not conclude the field moved.

### `+0x84 + skill * 4` is `<Misc>`'s three difficulty slots

This is the one claim on the page with two independent legs, and they point in
opposite directions - a **writer** on one binary and a **reader** on the other.

[`handling-stats.md`](../../../formats/handling-stats.md#misc-sits-on-the-stats-base-and-its-offsets-are-ps2-confirmed)
records `HandlingXml_ParseMisc` (`0x0014db08` in the PS2 `SCES_547.48`) writing
`easyshield` to `0x84`, `mediumshield` to `0x88`, `hardshield` to `0x8c`, and
plain `shield` to **all three at once** as a bulk default. The PSP's
`Ship_SetShield` indexes exactly that range, `0x84 + n * 4` for `n =
g_skill_level`. Three authored slots and a three-slot index, recovered
separately and never compared until this pass.

So the ladder is `easy`, `medium`, `hard`, in that order, and the shipped files
author the easy one explicitly and let `shield` cover the other two. That also
bounds `g_skill_level` at `0..=2` without needing to read its parser.
Confidence **88**: two binaries, and the range closes.

## `Ship_ResetShield` (`0x0883dd24`) fills it

```c
void Ship_ResetShield(ShipEntity *entity)
{
    if (g_game_mode < 0xe)
        Ship_SetShield(stats_base[0x84 + g_skill_level * 4], entity);
    else if (entity->0x364 == FUN_0895ebf0())   // this machine's own player
        Ship_SetShield(stats_base[0x84 + g_skill_level * 4], entity);
}
```

The whole function. It is the only thing on the disc that sets the pool to its
maximum, so a ship starts a race full and nothing else refills it outright -
`Ship_AddShield` (`0x0883ddc8`) adds a delta and is what Zone's clean-zone
recharge uses. **Not** called from `Ship_InitCraft` (`0x08849354`), which is
worth stating because that is where it was looked for first - and it is a
different object again, so `+0x88` was never going to be among the forty-odd
fields it writes.

Confidence **84**. The body is unambiguous; what is not read is every call
site, so "the only thing that fills the pool" is a statement about this
function, not a survey.

## `Ship_Damage` (`0x088439ac`) is the single subtraction

Named this pass. Signature, from its five call sites and its own body:

```c
void Ship_Damage(float amount, ShipEntity *entity, int source, int weapon_kind, char by_rival)
```

The order of operations, with the branches that matter:

1. **Refuse outright** when `Ship_State(entity)` is 4, 5, 6 or 2, or when
   `entity->0x860 & 0x1000` is set. Those are the destroyed, respawning and
   otherwise-not-racing states - a craft already blowing up takes no further
   damage.
2. **Halve the amount when weapons are off**: `if (g_weapons_enabled == 0)
   amount *= 0.5`. A real recovered rule, and a surprising one - the option
   that removes weapons also softens every *collision*.
3. Accumulate an integer copy of the amount into telemetry buckets selected by
   `source`, with a nine-way sub-bucket on `weapon_kind` for weapon damage.
4. **Subtract**, but only in states 1 and 3:
   `new = Ship_Shield(entity) - amount`. In every other state the pool is
   re-stored unchanged, which still pushes the HUD.
5. **Warn on the downward crossing of 20 %.** `DAT_08a7b6a4` is `20.0`, read
   from `.rodata`, and the test is on the percentage before *and* after -
   `before > 20 >= after` plays `energycritical`. A level test would re-fire
   every frame of a scrape; this one cannot.
6. `Ship_SetShield(new, entity)`.
7. **Destroy at zero or below**: `if (new <= 0.0) Ship_SetState(entity, 4)`,
   plus the placement and elimination bookkeeping that follows from it.
8. Throw the struck hull's **damage** sparks through `ShipCollisionFx_Trigger`
   on one or two random `Ship Collision Fx` locators, for `source == 2` only.
   These are not absorb sparks, which this line called them until 2026-09-24;
   see "`Ship_Damage`'s weapon branch throws the hit sparks" below.

`source` is a damage category, read off the buckets it selects rather than
guessed: **0** is track/wall contact, **1** is another craft, **2** is a
weapon. That agrees with `ShipCollisionFx_Trigger`'s own `kind` argument,
whose 0/1 split [`contact-response.md`](contact-response.md) already
documents, and it is the same value: the contact loop passes
`record->0x170 != 0` to both.

Confidence **84** on the control flow and the two constants, **75** on the
`source` categories, which rest on the bucket layout rather than on a string.

### Contact damage is `|p| * 0.035`

`FUN_088418e0`'s contact loop, per record in the ring:

```c
fVar22 = |p| * 0.05;
...
if (record->0x1a0 > 0.0 && Ship_State(entity) == 1)
    Ship_Damage(fVar22 * 0.7, entity, record->0x170 != 0, 0, 0);
```

`|p|` is the per-contact impulse magnitude `Body_RecordContact` stored, the
same quantity the sparks scale by `0.0125` and clamp at 1. So collision damage
is `|p| * 0.05 * 0.7` = **`|p| * 0.035`**, and it is `|p| * 0.0175` whenever
weapons are off - which includes a time trial.

The `0.05`/`0.7` pair is the same one
[`contact-response.md`](contact-response.md) recovered for the scrape friction,
which is why that page calls this the gameplay-facing twin.

**This is the number a runtime leg would settle**, and none exists yet. See
"What is not verified" below.

### `Ship_Damage`'s weapon branch throws the hit sparks

Read 2026-09-24, because a player sees sparks and smoke on a craft the Cannon
is hitting and nothing in the Cannon's own path spawns a particle
([cannon-quake-leachbeam.md](cannon-quake-leachbeam.md)). The spawn is on the
**victim's** side, in `Ship_Damage` itself, after the subtraction and after the
`<= 0.0` destroyed block has closed:

```c
// 0x08844000..0x08844074, inside `if (+0x368 == 0 || +0x368 == 2)`
if (FUN_0883e37c(entity) && source == 2 && entity->fx_count /* +0xca8 */ != 0) {
    leach = entity->craft /* +0x4c */ ->kind /* +0x138 */ == 7;
    i = Psys_RandIntRange(0, entity->fx_count - 1);
    j = Psys_RandIntRange(0, entity->fx_count - 1);
    ShipCollisionFx_Trigger(1.0, entity->fx[i] /* +0xc80 */, leach, 1);   // jal 0x08844048
    if (i != j)
        ShipCollisionFx_Trigger(1.0, entity->fx[j], leach, 1);            // jal 0x0884406c
}
```

- **Every weapon hit that gets through, the killing one included.** The block
  follows the destroyed test rather than sitting inside it, which corrects
  [contact-response.md](contact-response.md)'s older "only on the branch where
  shield has just reached zero". A shielded craft never gets here:
  `Ship_ApplyPendingWeaponDamage` (`0x0883f13c`) takes its `ShipShield_Hit`
  branch instead of calling `Ship_Damage` at all.
- **The effect.** `ShipCollisionFx_Trigger`'s own switch
  ([contact-response.md](contact-response.md)): kind 0 with `damaged` set is
  `WO_SHIP_COLL_SPARK_DAMAGE`, the same four-emitter tree a damaging wall
  contact throws - smoke puffs, a spark fountain, `bits` and embers
  ([pob.md](../../../formats/pob.md)). Kind 1 is
  `WO_SHIP_SPARK_DAMAGE_LEACHBEAM`, chosen when the pending hit's kind
  (`craft+0x138`) is `7`, the LeachBeam's own id.
- **Severity 2.4, always.** Intensity is the literal `1.0`, so the trigger's
  `intensity * 2.0 + 0.4` is fixed.
- **At most once per locator per 0.8 s.** The trigger's `instance + 100`
  cooldown applies to kinds 0 and 1, so a Cannon's stream of small hits
  sparks each of Assegai's six locators at most every 0.8 s.
- `FUN_0883e37c` is a display-mask test (see "Which ten nodes" below) and is
  not read further.

Confidence **88**: a clean decompile, both call sites read, and measured.

**Ported** as `oag_raceplay::hit_sparks`, Pulse only. `oag_weapons::projectile::WeaponHit::landed`
reports each hit that got through `Ship_Damage`'s gate as a per-tick output.
Three choices are not measured. The cooldown is per locator but not shared
with wall contacts. The display-mask gate is not modelled. The locator picks
come from a view-side generator.

**Measured on PPSSPP**, USA `BOOT.BIN`, a Single Race on Talon's Junction,
2026-09-24. A synthetic hit was posted into the player's pending-damage
channel at a `Weapons_DispatchFire` stop: `craft+0x120 = 2.0`, `+0x138 = 3`
(the Cannon's own tag), `+0x124 = 0`. Then breakpoints went one at a time on
`ShipCollisionFx_Trigger` and on `Psys_Spawn_q` (`0x08915484`):

| Posted kind | Trigger `ra` | Trigger `a1`, `a2`, `f12` | Next `Psys_Spawn_q` | Shield |
| --- | --- | --- | --- | --- |
| 3, four hits | `0x08844050` | 0, 1, 1.0 | `WO_SHIP_COLL_SPARK_DAMAGE`, `ra 0x089249b4` | -2.0 each |
| 7, two hits | `0x08844050` | 1, 1, 1.0 | `WO_SHIP_SPARK_DAMAGE_LEACHBEAM`, `ra 0x08924900` | -2.0 each |

Every `a0` was one of the player's six `+0xc80` locators, a different one on
most hits. In a live race the same `ra` was reached on AI craft: 29 stops
while the player fired Rockets into the grid across GO, 7 while it fired the
Cannon. The AI were firing too, and the weapon behind each stop was not
recorded, so none is attributed to one weapon. A wall contact reaches the
same trigger from `0x0883df68` (`Ship_DispatchCollisionFx`) and was seen
alongside.

**The smoke is per hit, not a shield state.** The player's pool was set to
`8.0` with no hit and photographed over three seconds: nothing drew on the
craft. Statically, the only ship-owned `Psys_Spawn_q` callers are this
trigger, the destruction spawners `FUN_0883e064` (`WO_SHIP_FXNODE_EXPLO`,
`WO_SHIP_DEATH_SPARKS`) and `FUN_088407b0` (`WO_SHIP_EXPLOSION`), and
`FUN_0883f540` (`WO_LEACHBEAM_CHARGING`). The complete caller list is 24
functions. None of them reads the shield level.

The frames are in `data/scratch/hit-sparks/` (gitignored): `cap-cannon/`
(a hit every 6 frames for 60 frames), `cap-lowshield/`, and
`cannon-orig-vs-ours.png`.

#### The player-only call beside it: `CockpitHitFx_Arm_q` (`0x088eeaf8`)

On a `source == 2` hit to the human craft (`+0x368 == 0`) outside game modes
2 and 12, `Ship_Damage` also calls
`CockpitHitFx_Arm_q(1.5, 1.5, 0.6, 1.0, DAT_08ab2120)` beside `Camera_ArmShake`.
`DAT_08ab2120` is a `0x68`-byte node that `InGame_Update` builds with
`FUN_088eea44`, right after the screen-flash node (`DAT_08ab2200`). The arm
does nothing while the node is already running (`+0x58`). Otherwise it stores
`alpha = 1.0 * 255`, `rate = 1 / 0.6`, and two amplitudes
`1.5 * DAT_08a84a30`.

Its update (`0x088eeb54`, vtable `0x08ad0d84`) decays both amplitudes
linearly to zero over the 0.6 s. Each frame it draws two fresh offsets in
eighths (`Psys_RandIntRange(0, 7) * 0.125`). Its enqueue (`0x088eec20`) only
queues a draw while the player craft's `+0x6d` is set: the internal
(cockpit) camera flag, per [camera.md](camera.md). **So it is not in any
chase-camera frame.** What it draws is unread. Confidence **60** for the name:
the arm, the decay and the cockpit gate are read, the picture is not.

## The two race options that reach the pool

`Race_ReadSetupOptions` (`0x08896b84`, confidence **88** as of 2026-08-10 -
was 80, the body is a flat run of key lookups on literal-string keys and no
caller has been read, but every mode-keyed branch its `g_weapons_enabled`
switch takes is now independently checked against the Custom Race screen
itself, below) reads the race setup as string key/value pairs - `Team`,
`Livery`, `Mode`, `Class`, `Track`, `Opponents`, `Weapons`, `Damage`,
`SkillLevel`, `Tournament` - and lowers three of them into globals this page
needs. `FUN_08973424` is a string compare returning zero on equal, and
`DAT_08a7e2c0` is the literal `"On"` (read from `.rodata`, not inferred):

| Global | Address | Set from | Polarity |
| --- | --- | --- | --- |
| `g_weapons_enabled` | `0x08b30fa8` | `Weapons` | `"On"` -> 1 |
| `g_damage_enabled` | `0x08b30fa9` | `Damage` | `"On"` -> 1 |
| `g_skill_level` | `0x08b31044` | `SkillLevel` | index, **default 1** when the key is absent |

Each also has a per-mode default taken before the string is consulted, keyed
on the game mode at `DAT_08b31048`.

`g_skill_level`'s default of 1 is the middle rung of the three-slot ladder
above, which is the cross-check that the ladder is ordered easy-first rather
than hard-first.

### `g_weapons_enabled`'s per-mode default, spelled out, and checked against every race type

**The switch, read in full (`FUN_08896b84`, i.e. this function):**

```c
switch (DAT_08b31048) {
    case 5: case 9: case 10: case 15: case 17:
        g_weapons_enabled = 0;   // time trial (5), speed lap (10), + three more
        break;
    default:                    // 6, 7, 8, 11-14, 16 and anything else
        g_weapons_enabled = 1;
}
// Whether the <Weapons> setup attribute is even consulted afterward is a
// SECOND, separate switch:
switch (DAT_08b31048) {
    case 5: case 8: case 9: case 10: case 15: case 17:
        override_allowed = false;   // the default above is final
        break;
    case 6: case 7: case 11: case 12: case 13: case 14: case 16:
        override_allowed = true;
        break;
    default:
        override_allowed = true;
}
if (override_allowed && <Weapons> is present)
    g_weapons_enabled = (<Weapons> == "On");
```

So three groups, not two: modes `5`/`9`/`10`/`15`/`17` are weapons-off and
**locked** (no setup string can turn them on); mode `8` is weapons-**on** and
equally locked; everything else defaults on but is a normal setup switch.

**Checked against the Custom Race screen directly, 2026-08-10** - PPSSPP
v1.20.4 (SDL, `--graphics=gles`, under Xvfb), one screenshot per RACE TYPE
entry, saturating left then stepping right through the whole list:

| Race type | `WEAPONS` shown | Editable | Reads as mode |
| --- | --- | --- | --- |
| Single Race | On | yes | not `5/8/9/10/15/17` |
| Head to Head | Off | **no** (greyed) | one of `5/9/10/15/17` |
| Time Trial | Off | **no** (greyed) | `5` (confirmed live below) |
| Speed Lap | Off | **no** (greyed) | `10` (confirmed live below) |
| Tournament | On | yes | not `5/8/9/10/15/17` |
| Zone | Off | **no** (greyed) | `6` (confirmed live: `Zone_Update`'s own selector) |
| Eliminator | On | **no** (greyed) | `8` |

Two things this settles that the switch alone could not:

- **Eliminator is mode `8`**, confirmed independently of the
  `<WeaponPad elimination_refresh_time>` naming hypothesis in
  [pads.md](pads.md) - both now point at the same mode number from opposite
  directions (a locked-on `WEAPONS` row here, a distinct
  `DAT_08b31048 == 8` branch in `WeaponPads_TestCraft` there).
- **Zone's `WEAPONS: Off` is the front end supplying `<Weapons>Off</Weapons>`
  explicitly, not the code default** - `6` is in the *on*-by-default,
  override-*allowed* group above, so without that explicit XML, Zone would
  race with weapons on. The greyed row means the front end also refuses to let
  the *player* override its own override, which the switch alone does not
  show - only the menu does.

`AI DIFFICULTY` greys to `N/A` on exactly the same three modes (Time Trial,
Speed Lap, Zone) plus Head to Head - screenshotted in the same pass - which is
the live confirmation behind `oag_race::Mode::has_opponents` returning `false`
for all three modes this crate implements. See
[race-modes.md](../../../gameplay/race-modes.md).

### Damage off does not mean no subtraction - it means regeneration

The other consumer of `g_damage_enabled` is in `FUN_088418e0`, and it is not a
gate on `Ship_Damage` at all:

```c
if (g_damage_enabled == 0) {
    s = Ship_Shield(entity) + dt * 4.0;
    if (s < 20.0) s = 20.0;
    Ship_SetShield(s, entity);
}
```

So with damage off the pool regenerates at **4 units a second** and is
**floored at 20**, while `Ship_Damage` keeps subtracting exactly as before.
The craft cannot be destroyed because the floor is above zero, not because
nothing hurts it. Worth writing down precisely: "damage off" reads like a
branch that skips the damage, and it is not one.

## `Ship_State` (`0x0883e64c`) and `Ship_SetState` (`0x08844100`)

Both named this pass, because the damage path is unreadable without them and
both were being called through unnamed wrappers on four other pages.

`Ship_State` returns `entity->0x8c`, unless `entity->0x860 & 0x800` is set - the
networked craft flag - in which case it asks `FUN_0894db4c(entity->0x364)` for
the remote player's state instead.

`Ship_SetState` writes `entity->0x8c` and runs the transition. The states this
page needs:

| State | What the setter does |
| --- | --- |
| 3 | Respawn. Plays `RESET`, and stores `clamp(Ship_Shield(entity) - 1, 0, 5)` into `entity->0x44` - **a respawn cost, capped at 5**, or a flat `10` in game mode 6 |
| 4 | Destroyed. Plays `_BLOWUP`, hides the HUD, puts the camera in mode 5 |

Confidence **84** for both names, **70** for the respawn-cost reading: the
value is computed and stored, and what consumes `entity->0x44` is not read.

### The full nine-arm table, and a race-start hypothesis for states 6 and 7

2026-09-02, read while chasing the race-start countdown / launch-boost handover
thread. `zone-mode.md`'s own account of `Ship_SetState`'s jump table (`0x08a7bc18`,
nine arms, `0x0884416c` through `0x088446ec`) only carries states 4 and 5 to
instruction level. The other seven, disassembled the same way (the decompiler
still gives up on the whole function, so every arm below is read as raw
instructions, not decompiled):

| State | Address | What it does |
| --- | --- | --- |
| 0, 1, 2 | `0x0884416c`, `0x0884417c`, `0x0884418c` | Identical: call `FUN_0003aae8(entity)`, then fall into the common tail at `0x08844720`. Three states sharing one body with no state-specific work of their own - plausibly grid/lights/racing, the three the countdown walks through before anything else diverges, but nothing here names which is which. |
| 6 | `0x088445a0` | **Sets `entity->0x874` to `2.0` if `entity->0x368 == 0` (the local player - confirmed field, see below), or `0.8` otherwise.** `entity->0x874` is the *same field* states 4/5 use as the destruction timer (`0.5` then `1.5` seconds, per the table above this section). Then zeroes a byte at `craft->0x794->0x3bc->0x68`, calls `FUN_0018dd94` on that sub-object, and calls `FUN_0003a624(entity)` - a shape (silence something, then call one more function on the entity itself) that reads as "stop the engine, arm a timer" rather than anything shield- or damage-related. **Hypothesis, confidence 55: this is the false-start engine stall** `race-modes.md`'s "What no mode has yet" section already asserts from the user's own play knowledge ("silently kills the engine if you hold thrust before the lights") but had never traced. The local/remote timer split (2.0s vs 0.8s) would make sense as a harsher, visible penalty for the human and a shorter one that only needs to desync an AI/remote craft's own thrust briefly. Not yet runtime-verified - nothing has watched `entity->0x874` or `entity->0x368` during an actual false start. |
| 7 | `0x088445ec` | Zeroes the same `craft->0x794->0x3bc->0x68` byte state 6 does, then reads a global byte at `0x08aae7e3` to pick between `0` and a value read off a global pointer's `+0xb8`, range-checks it against `19`, and indexes a *second* jump table at `0x08a7bc40` - **resolved** (see below), 18 of its 19 entries read this session, and they collapse to just **two** distinct targets, `0x0884463c` and `0x088446c0`, both still inside this same disassembled block. Both toggle the *same* bit this file's own `Ship_SetState` table already names - `entity->0x860` bit `0x1000`, the bit `zone-mode.md` confirms `Zone_UpdateRacing` reads as "this craft is destroyed" - but differently: `0x0884463c` conditionally sets or clears it based on `entity->0x368` (and, only when setting, calls `FUN_0003c7b0(entity)`); `0x088446c0` unconditionally clears it, no call. **Revised reading, confidence 55: state 7 is not a per-mode countdown dispatch - it reads as a "clear the destroyed flag" transition** (out of states 4/5, back to racing), where the *class* of the current game mode (index `0` invalid/unused, `1` and the last four indices route to the conditional-clear-plus-call path, the twelve in between to the plain unconditional clear) decides which of two clear-bit shapes runs. **This walked back the state-7-as-countdown hypothesis this thread's earlier pass made** - the per-mode *jump table* is real and resolved, but what it dispatches to is narrower and less countdown-shaped than first read. Still open: which twelve-vs-five-ish mode classes these two groups actually are, and what `FUN_0003c7b0` does. |
| 8 | `0x088446ec` | Sets `entity->0x874` to `1.0` when `entity->0x368` is zero (the local player) and **`0.8`** otherwise (**corrected 2026-10-02**: this row read `2.0` for the non-local craft, taking the `lui a0,0x3F4C` in the `bne`'s delay slot at `0x088446f4` for dead code; it executes on both paths, and the `ori 0xCCCD` at `0x08844704` completes `0x3F4CCCCD = 0.8`, the same constant state 6's case arms for a non-zero `entity->0x368`; an Eliminator opponent measured live at `0.8`, see "State 8 measured" below) - then calls `FUN_0018dd94` again on the `0x3bc`-relative sub-object. Falls into the same common tail. Not enough here to guess what distinguishes state 8 from state 6 beyond the timer and the inverted local/remote ratio - both look like "some kind of timed lockout, direction of the asymmetry depends on which state" rather than one being clearly the false start and the other something else. |

**The per-state *updates* were read 2026-09-16, and they retire the false-start
hypothesis on state 6.** Three of the states have their own per-tick update:

| State | Update | What it does |
| --- | --- | --- |
| 4 | `Ship_UpdateExploding` (`0x088404c8`) | `+0x874 -= dt`; at zero, `Ship_SetState(5)` |
| 5 | `Ship_UpdateDestroyed` (`0x08847650`) | `+0x874 -= dt`; at zero counts the kill (`craft+0x8d4`), announces `FIRST KILL` / `5 KILLS LEFT` / `3` / `1` / `cont_elim` in Elimination (modes `8`/`0x12`, against the target `DAT_08b30fb0`), then `Ship_SetState(6)` - or `8` in Elimination |
| 6 (**wrong, see the 2026-10-02 correction below: this is state 8's update**) | `Ship_UpdateRespawn` (`0x08847914`) | `+0x874 -= dt` (the `2.0` s the player waits, `0.8` s an AI); at zero plays `RESET` for the player, clears the boost/stun fields, **relocates the craft onto the spline** - at the AI-corridor midpoint, 5 units up, facing 40 units down the tangent - sets the hover height from the handling block (`+0x78c * 0.25 + 50`), `Ship_SetState(1)`, `Ship_ResetShield`, and clears the destroyed bit `0x1000` |

So state 6 is the **respawn delay after destruction**, not a false-start stall
(which [race-modes.md](../../../gameplay/race-modes.md) had already measured
does not exist), and the `2.0`/`0.8` split is how long a human against an AI
sits dead before reappearing; state 8 is Elimination's own respawn with the
opposite ratio. State 3's `RESET` cue is the same one this update plays.
State `2` is *finished*: `Race_FinishAllCrafts` (`0x08824e10`,
[grid.md](grid.md)) sets it on the whole field at the flag and hands each craft
to its autopilot; state `7` is a networked craft whose peer dropped
(`FUN_08847f54`). States `0` and `1` are grid and racing, set by
`Race_PlaceGrid` and `Race_StartRacing`.

**Correction, 2026-10-02 (pulse-wreck-3): the per-state update table says state 8, not 6, is
`Ship_UpdateRespawn`.** `FUN_088418e0` dispatches on `Ship_State` through the nine-entry table at
`0x08a7bb88` (`sltiu 9`, `lui 0x8a8`, `lw -0x4478`, `jr`; read headless, whole table below):

| State | Entry | Update |
| --- | --- | --- |
| 0, 1, 2, 3 | `0x08841d68`, `d7c`, `dc8`, `ddc` | `FUN_0883fde0`, `Ship_UpdateStartBoost` (when `craft+0x368` is 0 or 2), `FUN_0883ff64`, `FUN_0883ff6c` |
| 4 | `0x08841df0` | `Ship_UpdateExploding` |
| 5 | `0x08841e04` | `Ship_UpdateDestroyed` (call at `0x08841e08`) |
| 6 | `0x08841e18` | **`FUN_08840500`: `+0x874 -= dt`, and the frame it crosses zero plays `cont_elim` in modes other than 2, 8 and 18. Nothing else** |
| 7 | `0x08841e2c` | `FUN_088405c8` (back to state 1 when `DAT_08b313dc[craft+0x364]` is set, for a local or networked craft) |
| 8 | `0x08841e40` | **`Ship_UpdateRespawn`** (call at `0x08841e44`, its only caller) |

So the table above that reads state 6 as "the respawn delay" is half right: `Ship_SetState`'s case 6 does arm `2.0`/`0.8` s, but
nothing read counts it into a respawn. The relocation, the `RESET` cue, `Ship_ResetShield` and `Ship_SetState(entity, 1)` belong to
**state 8**, which only the Eliminator reaches (`Ship_UpdateDestroyed` picks 8 for modes 8 and 18, 6 for the rest). Consequences:
the Eliminator's return is state 5's `1.5` s then state 8's `1.0` s (the local player) or `0.8` s (anyone else - this said `2.0` s
until the same day's live measurement, "State 8 measured" below), ported as `eliminator::eliminator_respawn_delay`; and **what
brings a single race's wrecked AI craft back from state 6 is not found** (and was then measured: nothing does, "State 6 measured
on PPSSPP" below). **The search for another way in or out** (headless, all 20 `Ship_SetState` call sites, `0x08844100`'s xrefs): the only
call that passes `8` is `Ship_UpdateDestroyed`'s (`0x088478ec`); every other caller is a fixed state (`Race_PlaceGrid` 0,
`Race_StartRacing` 1, `Race_FinishAllCrafts`, `FUN_0882d578`, `FUN_088dfb90`, `FUN_08824a44`, `Race_ResetCraftBoosts_q` 2, the
`FUN_088418e0` sites 2 and 3, `Ship_Damage` 4, `Ship_UpdateDestroyed` 6 and 8, `FUN_088405c8` and `Ship_UpdateRespawn` 1,
`FUN_08847f54` 7). The two `FUN_088418e0` routes to state 3 need the destroyed bit `0x1000` clear, and only `Ship_UpdateRespawn`
clears it. So **no path read brings a state-6 craft back before `Race_StartRacing`** - which fits the announcer line the state-6
timer plays at expiry (`cont_elim`, "contender eliminated": a single race's destroyed AI craft may simply be out, as
`RaceState::eliminate` already argues for the player). Consistency, not proof: `Ship_UpdateRespawn`'s own `cont_elim` branch
(modes other than 2, 8, 18) is dead code if only the Eliminator reaches it, which suggests the update was once state 6's. A live
run on PPSSPP decides it. `Ship_UpdateDestroyed` also writes `4.0` into `+0x874` before `Ship_SetState` overwrites it (read, no effect seen).

**The address-resolution trap, and how state 7's table was actually read**:
both addresses state 7's own disassembly computes (`lui`/`lw`-offset pairs, not
`jal` targets) failed to read back through `inspect_memory_content` at their
literal decompiled values (`0x002ac7e3`, `0x00277c40`) and read cleanly once
`+ 0x08804000` was added - the same correction `workflow.md`'s known-imperfect
section already documents for `jal` call targets on this binary, now confirmed
on a `lui`/`lw` jump-table base too; see that page for the generalised note.
The corrected inner table sits at `0x08a7bc40`, immediately after (one null
word past) `Ship_SetState`'s own nine-entry table's end at `0x08a7bc3c` - the
same data region, which is itself confirmation the correction is right.

`entity->0x368` itself is not re-derived here - `shield.md`'s own
["`entity + 0x368` is `Craft_Construct_q`'s own second argument"](#entity--0x368-is-craft_construct_qs-own-second-argument)
section already puts it at confidence 80 as "zero for the local player, set
for everyone else", and states 6 and 8 above both read cleanly against that.

### Who ends a single race on the destroyed bit, and who comes back

Read 2026-09-16 alongside `Ship_AddShield`'s callers. State 6 runs in *every*
mode - `Ship_UpdateDestroyed` picks state 8 only for modes `8`/`0x12` - so
the question [race-modes.md](../../../gameplay/race-modes.md) settled off the
results strings ("a destroyed craft is out of a single race") needed a
reader of the bit. It has one. `ArcadeRace_Update` (`0x0882c48c`, the same
five-way dispatch on `mode+0x7c8` every mode object has) hands state `2`,
racing, to `ArcadeRace_UpdateRacing` (`0x0882c5c4`):

```c
if (0 < mode->state /* +0x7cc */ && mode->state < 2) {
    Entity *player = mode->player;                              // +0x2c0
    if ((player->flags /* +0x860 */ & 0x1000) || Ship_State(player) == 2) {
        Hud_Hide(g_hud);                                        // 0x0881a128
        Race_BuildEndRaceResult(mode, 0);                       // 0x08827b4c
        RaceMode_SetState(mode, 3);
    }
}
```

So the **player's** destruction ends an Arcade race the tick state 5 raises
the bit - before state 6's two seconds can put the craft back - and the
results row reads "Ship destroyed" because the race is already over. An
**AI craft** has no mode object watching it, and **(corrected 2026-10-02, measured live: "State 6 measured on PPSSPP" below) it
does not come back**: it sits out state 5's `1.5` s and state 6's `0.8` s and stays in state 6. (This paragraph read
`Ship_UpdateRespawn` as state 6's update, relocating and refilling the craft so it raced on, with its `cont_elim` line announcing
a rival going down and coming back; that update is state 8's, the Eliminator's.) `cont_elim`, *"contender eliminated"*, is
played by state 6's own timer at its zero crossing (`FUN_08840500`) in modes other than 2, 8 and 18. Confidence **82** for both names (the object is identified by
its address range and its HUD file, per [state-machine.md](state-machine.md),
and the body is a clean decompile); race-modes.md's 75 for the ending rises
to that with it.

**Ported** (changed 2026-10-02, pulse-state6): `Race::tick_destroyed_craft` brings a craft back in the Eliminator only
(state 8: `eliminator_respawn_delay`); a single race's wrecked opponent stays down like the player, whose own destruction ends the
race through `RaceState::eliminate`. This used to return the opponent after `1.5 + 0.8` s on the paragraph's old reading. Still
not raised: `cont_elim` at state 6's expiry (an announcer-bank line this build's cue table does not carry yet) and the player's
`RESET` cue. The Eliminator's respawn pose is `Race::respawn`'s racing-line one rather than state 8's corridor midpoint
(`pos + lateral * (bound_r - bound_l) * 0.5`, five up, facing forty down the tangent), a stated departure.

### State 6 measured on PPSSPP: a wrecked AI craft stays down (2026-10-02, pulse-state6)

PPSSPP v1.20.4 (software renderer), Pulse USA, Single Race / Venom on a track that was **not** Talon's Junction (the craft came up
139.9 units from that start line; Track Select was not identified). `scripts/psp-state6-watch.py` stops at `Ship_UpdateCraft`
(`0x08849618`) each frame, wrecks one AI craft a few game seconds into a live race and logs its `entity` fields; `--watch` arms a
write watchpoint on `entity+0x8C`. Two valid runs (**the same boot and the same track, one race restarted between them**), different grid slots and different ways in:

| Run | Slot | Call | State sequence (frames after the call) | State at +15 s | Writers of `+0x8C` |
| --- | --- | --- | --- | --- | --- |
| a2 | 1 | `Ship_SetState(entity, 4)` | 4 (0), 5 (30), 6 (120) | **6**, timer `-12.0` s, 780 frames past entry | `Ship_SetState` only: `pc 0x08844760`, `ra 0x08844598` (4 to 5) and `ra 0x088445e4` (5 to 6); none after |
| b1 | 2 | `Ship_Damage(1e6, entity, 0, 0, 0)` (shield read back `-999855`) | 4 (0), 5 (30), 6 (120) | **6**, timer `-12.0` s, 790 frames past entry | the same two, none after |

State 4 lasts `0.5` s, state 5 `1.5` s, and state 6's `0.8` s timer crosses zero at about frame 168 (`entity+0x368` is `2` here, the
non-zero case). The destroyed bit `0x1000` of `entity+0x860` sets at state 5 and stays; the wreck is the live model
(`+0x8B0 == +0x8B8`); the craft coasts and comes to rest where it landed (b1: speed `0.0` from about frame 410, at
`(-544.4, -2.05, 151.5)`, unchanged at frame 910) and is never moved onto the racing line. Nothing reappeared. Raw logs:
`data/scratch/pulse-state6/{a2,b1}/log.json`. A first run (a1) read the target already in state 2 (the race had ended while the
emulator ran free between commands, `Race_FinishAllCrafts` setting the whole field to 2); it held state 6 and logged the same two
writes but is **void**. One frame of b1's HUD read `pos 7/7` (unverified against a baseline).

Confidence **85** that nothing in a single race revives a state-6 AI craft: a runtime trace, twice, agreeing with the jump
table and the 20-caller `Ship_SetState` survey. Short of 90 because the craft was destroyed by a call from a debugger stop (the
`Ship_Damage` call runs the whole destroy path, but not a weapon's or a wall's own caller), the window was 13 s past entry rather than
a race run to the flag, and one track and one grid were seen. **Not measured**: what `Race_FinishAllCrafts` does to a wreck at the
flag (the voided run's state 2 hints the field is set to state 2 whatever it was in), and whether the standings drop a wreck.

### State 8 measured on PPSSPP: an Eliminator opponent waits 0.8 s (2026-10-02, pulse-state6)

The same script on an Eliminator (`scripts/psp-drive.py menu --race-type 6`, the `KILLS (5)` HUD up, same non-Talon's-Junction track),
`Ship_Damage(1e6, entity, 0, 0, 0)` on slot 2 (`entity+0x368 == 2`), write watchpoint on `entity+0x8C`, run `e1`:

| Frames after the call | State | `entity+0x874` | Writer of `+0x8C` |
| --- | --- | --- | --- |
| 0 | 4 | 0.5 | (the call) |
| 30 | 5 | 1.5 | `ra 0x08844598` |
| 120 | **8** | **0.8** | `ra 0x08844718` (`Ship_UpdateDestroyed`'s call at `0x088478ec` passes 8) |
| 168 | 1, shield `144.99` (refilled) | about 0 | `ra 0x08844184` (`Ship_UpdateRespawn`'s `Ship_SetState(1)`) |

So an opponent is out `1.5 + 0.8 = 2.3` s in the Eliminator, not `3.5`. Run `e2` (a second race on a fresh boot, slot 4) read the same
`4 (0), 5 (30), 8 at 0.8 (120), 1 (168)`, with the same three writers' `ra`s. Run `e3` wrecked the **player** (grid slot 7,
`entity+0x368 == 0`): `4 (0), 5 (30), 8 with +0x874 = 1.0 (120), 1 (180)`, shield refilled, so the player's wait is `1.0` s, `2.5` s out in all.
Confidence **92** for the opponent's `0.8` (two runs, two slots, the corrected disassembly), **90** for the player's `1.0` (one run, the disassembly).
All three Eliminator runs and both single-race runs were on one emulator profile and the same track (not Talon's Junction), and all five
destroyed the craft by a call from a debugger stop, not a weapon or a wall.

**What would raise every confidence number in this table**: a live PPSSPP
capture of an actual false start (hold thrust before the lights, per the
user's own description of the original) with a watchpoint on `entity->0x874`
and `entity->0x8c` (the state field `Ship_State`/`Ship_SetState` read and
write) for the player's own entity. That would show which state number the
engine actually enters, settle whether it is 6 or 8, and read the true timer
value off real hardware rather than a decompiled float bit-pattern. Also
still needed: what the ~19-entry mode-class value at `0x08aae7e3`/global
`+0xb8` actually enumerates, and what `FUN_0003c7b0` does - both would turn
state 7's "two groups of modes" reading into a named one.

### State 3 is the reset, and four things enter it (2026-09-29)

Read while chasing a craft that falls off `01_Track`'s lip and beaches, and the
airborne trigger was then **watched firing in PPSSPP**. All four callers are in
`FUN_088418e0`, the per-craft race update (see
[contact-response.md](contact-response.md) for its contact loop), and all four
call `Ship_SetState(craft, 3)`:

| Call | Condition | Confidence |
| --- | --- | --- |
| `0x08841cec` | `craft+0x788 > 0.5`, where `+0x788` accumulates `dt` while `\|body.pos - craft+0xaf0\|^2 > 40000` (more than **200 units** from `+0xaf0`) and is zeroed otherwise; skipped when the destroyed bit `0x1000` is set | 80 (decompile) |
| `0x08841d30` | `*(craft+0x94)+0x284 > 4.0`, the craft's **airborne clock**; skipped when the destroyed bit is set | 90 (decompile + live) |
| `0x0884239c` | `DAT_08ab0c8c != 0 && FUN_0883191c(body+0x3bc, body, craft+0xad8) == 2`, only for a craft whose `+0x368` is `0` or `2` and whose state is not `6` | 50 - what `FUN_0883191c` tests is not read |
| `0x08842724` | a hull contact against a `Reset` mesh collider ([collision.md](collision.md#every-mesh-surface-reaches-the-hull-narrowphase-2026-09-29)) | 86, as recorded there |

`craft+0xaf0` is the position `AiTrack_LocatePosition` maintains for the craft
(the same address the relocation below reads); read here, not traced.

The airborne clock is `Ship_UpdateCraft`'s pair at `0x08849df0`: when any hover
probe touched this tick (`craft+0x1c0 & 1`) it stores `0` to `+0x284`
(`0x08849e08`) and adds `dt` to `+0x288`; otherwise it zeroes `+0x288` and adds
`dt` to `+0x284` (`0x08849e28`). An upside-down craft's probes point at the sky,
so it counts as airborne.

**`Ship_SetState(3)`** (`0x08844100`, arm 3): only on entry from another state,
saves the current state to `+0x40`, zeroes `+0x79c`, and **for the player**
(`+0x368 == 0`) plays `RESET` and sets `+0x44 = min(5, shield - 1)`, floored at
`0`, or `10` in mode `6`. Every entry clears flag bits `0x80`/`0x100` and the
`+0x87c`/`+0x88c`-`+0x898` fields.

**The state-3 update, `FUN_0883ff6c`**: `+0x79c -= dt`; while it is under `0.3`
it relocates the craft - `AiTrack_LocatePosition(500.0, track, craft+0xaf0,
..)` from `+0xaf0`, five units up, facing forty units down the tangent, then
`FUN_0883db20(+0x78c * 0.25 + 50, craft, pose)` - and once it reaches `0`
returns to the saved state and, if `+0x44 > 0`, calls
`Ship_Damage(+0x44, craft, 0, 0, 0)`. So **a reset costs the player up to five
shield** and costs an AI craft nothing. Since `+0x79c` starts at `0`, the
relocation happens on the entry tick.

**Measured** (`01_Track`, Basilico Black, Time Trial, reached with the
dev-unlock byte in [ppsspp-debugger.md](../../../reverse-engineering/ppsspp-debugger.md)):
the player's craft placed on the upper deck at sample 10 and coasted off the
lip at 20 u/s landed upside down on the floor below. `+0x284` was `0` until the
tick it left the deck (133), counted `0.0167` a tick, read `3.99` on tick 371,
and on tick 372 the craft was relocated upright at sample 50 on the lower
floor, five units up and 12.5 off the line, moving at **53.5 u/s** along the
track. The same trial at 30 and 45 u/s never reached the clock's limit: the
craft rights itself (30) or lands upright (45). **What the `53.5` means** is
read but not settled: it matches `FUN_0883db20`'s first argument if `+0x78c`
was `14`, and `+0x78c` is written from `body+0x398` in `FUN_088418e0`, which
reads as a launch speed - not the "hover height" `Ship_UpdateRespawn`'s row
above calls the same expression. Confidence **50** on that reading; nothing
renamed.

**Ported** (the airborne trigger only): `oag_race::recovery::AIRBORNE_RESET_SECONDS`
and `RespawnCause::Airborne`, through the same `Race::respawn` a `Reset`
contact uses. Three departures, recorded rather than fixed: that respawn puts
the craft at rest on the racing line rather than at the corridor midpoint at
`FUN_0883db20`'s speed, charges the player no shield, and the `200`-unit
distance trigger at `0x08841cec` is not ported - this project's invented
`LostCircuit`/`OffTrack` dwells stand where it would.

## The runtime leg, and what it did not reach

Measured 2026-08-10 against the USA disc in PPSSPP, `steer-left.inputs`
(200 ticks of full lock, which ends in a wall by design), Talon's Junction,
Time Trial, with `scripts/psp_trace_fields.py`'s new `shield` column.

**What the pool did, and the experiment that made it do it.** A Time Trial
races with both options *off*, which was read straight out of the two globals
rather than inferred: `g_weapons_enabled` and `g_damage_enabled` both read
`0` in a live race. That is the per-mode default `Race_ReadSetupOptions` takes
for game modes 5 and 10, and it is the first direct confirmation that those
two are this project's time trial and speed lap. Writing the globals by hand
mid-race then gives a controlled A/B the front end cannot reach:

| `weapons` | `damage` | What the pool did over 200 ticks |
| ---: | ---: | --- |
| 0 | 0 | **pinned at its maximum**, dipping by thousandths and recovering, ending at exactly the maximum after 115 ticks of wall contact |
| 0 | 1 | **drained monotonically** - 0 of 199 ticks rose - spending `2.3653` |
| 1 | 1 | drained monotonically, spending `4.6846` |

Three things follow, and each was a claim this page previously made on a
decompile alone:

- **The regeneration branch is real and `g_damage_enabled`'s polarity is the
  way round this page says.** With it clear the pool cannot fall - 115 ticks
  of scraping end at the maximum - and with it set the pool never once
  recovers. Confidence **92**.
- **"Damage off" does not skip the subtraction.** The dips in row 1 are
  `Ship_Damage` still running; what hides them is the 4-a-second recovery
  clamping straight back to the maximum. A capture that only looked at the
  endpoints would have read this as "no damage" and pinned the wrong
  mechanism.
- **The weapons-off halving measures 1.98 against a predicted 2.00.** Total
  energy spent gives `4.6846 / 2.3653 = 1.9806`; the per-tick ratio over the
  114 damaging ticks has a median of `1.9897`. The two runs are not even on
  the same trajectory - their `pos_x` differs by about 30 units throughout,
  so the contacts themselves differ - which is why the residual 1 % is not
  worth chasing. Confidence **90**.

### The coefficient is exactly `0.035`, measured

**`amount / |p| = 0.035000` on 25 of 25 calls** - median, minimum and maximum
all identical to six figures, over amounts spanning `0.0117` to `2.63`, a
factor of 225. That is float precision, not a fit.

The probe is a breakpoint on `Ship_Damage` itself, and the reason it works is
worth stating: **the contact ring is still intact there**. `FUN_088418e0`'s
loop is the ring's only consumer and calls `Ship_Damage` from inside it, so the
record being processed has not been cleared. An earlier attempt read the ring at
`Ship_UpdateCraft`'s entry instead and found the count `0` on all 199 ticks,
because by then the ring has already been drained - recorded here so the cheap
probe is not tried a third time.

Two things fell out of the same 25 calls:

- **One call per ring record, confirmed.** A tick whose count was `4` produced
  **four** `Ship_Damage` calls, each matching a different record's `|p|` at
  exactly `0.035`. So a frame with several contacts is damaged several times,
  which is what `oag_physics::wall::WallResponse::impulse_sum` sums for.
- **`source` is `0` on every wall contact**, with `a1`, `a2` and `a3` all zero,
  which is what the contact loop passes and what fixes `0` as track/wall.

**The calling convention is not what o32 would predict, and this cost an
attempt**: the entity is in **`a0`** and the float amount in **`f12`** - the
leading float does not reserve `a0`. Reading the entity from `a1` gives a zero
and an "Invalid address" from the debugger, which at least fails loudly.

## `entity + 0x368` is `Craft_Construct_q`'s own second argument

**2026-08-25.** A program-wide search for every `sw <reg>, 0x368(<reg>)`
found nine matches; six are unrelated (three write a stack slot, not an
entity, and three land in a different, network-message-shaped structure
around `0x089ee000`/`0x08a74000` that is never the ship entity `Ship_Damage`
walks). The seventh is the one that matters:
`Craft_Construct_q` (`0x08840c74`), at `0x08840fec`:

```
08840d48: move s2, a1        ; s2 = the constructor's own 2nd argument
...
08840fec: sw s2, 0x368(s0)   ; entity->0x368 = s2, unmodified
```

`s2` is never written between those two instructions, so this is a verbatim
parameter store, not a computed value - the cleanest kind of writer a static
read can produce. **This closes the "find the writer" step**, but not what
the value *means*: no static caller of `Craft_Construct_q` exists anywhere in
the binary to read off - no `jal` targets it and no data reference holds its
address, so it is reached only through an indirect dispatch (a class
descriptor or vtable-style table) this pass could not resolve. Confidence
**80**: unambiguous decompilation and register tracing, but no call site to
cross-check against and no runtime trace, so it stays in the "probable" band
rather than "confident" per the
[rubric](../../../reverse-engineering/confidence-rubric.md).

The same parameter drives two more things in `Craft_Construct_q` itself,
both keyed on its numeric value:

- **Naming.** `-1` gets no name at all (skipped outright); `0` gets a fixed
  string with no format argument; `1` gets a string formatted with the
  constructor's third argument; `2` and `3` get a *different* string, also
  formatted with the third argument; `4` and above do nothing.
- **`entity + 0x48`**, set in the same branches: `0` for values `0`/`1`,
  `1` for values `2`/`3`. That byte is read immediately afterward and passed
  into a network-controller-binding call (`func_0x0003a4bc`), but only when
  the game mode is a multiplayer one (`>= 0xe`) - it plays no part in
  single-player construction otherwise.

**This refutes "local human player slot"** more directly than the previous
reading could: rereading `Ship_Damage` in full this pass shows its pool-math
branch treats `0` and `2` identically (`if (iVar2 == 0 || iVar2 == 2)`
reaches the real `Ship_SetShield` subtraction), while `1` takes a separate
branch that does **not** touch the pool at all - it only increments a
telemetry counter - and `3` reaches neither branch, so nothing in
`Ship_Damage` acts on it. A slot count would not group `0` and `2` together
while excluding `1`. The shape instead fits a controller/ownership-kind
selector: `0` is the only value that also gets a HUD update and detailed
telemetry, i.e. the local human; `2` is simulated fully (it takes real
damage) but silently, which fits an AI opponent; `1` skips the local pool
subtraction entirely, consistent with a remote network player whose
authoritative shield value is not this machine's to change; `3` is untouched
by `Ship_Damage` altogether, so it is not a value produced for anything that
takes contact damage in single-player.

### Confirmed live: `0` is the human, `2` is every AI craft, in a Single Race

**2026-08-25, PPSSPP v1.20.4, SDL build under Xvfb, `pulse-psp-usa.chd`,
SINGLE RACE / VENOM, a full eight-craft grid on Talon's Junction.** Rather
than catch `Craft_Construct_q` itself - it has no static caller, so nothing
says *when* to arm a breakpoint for it - this reads the field the settled
way: break at `Ship_UpdateCraft` (fires once per craft per tick, a different
craft in `a0` each time), collect all eight distinct craft addresses, then
for each one read `craft + 0x2b8` (throttle), walk `craft + 0x1c4` to the
entity, and read `entity + 0x368`
(`scripts/psp-watch-controller-kind.py`):

```
craft        throttle entity         entity+0x368
0x09b774b0     0.00 0x09b764f0              0
0x09b48160    61.30 0x09b471a0              2
0x09b12ee0    57.32 0x09b11f20              2
0x09ae3e20    34.60 0x09ae2e60              2
0x09aaf6a0    34.60 0x09aae6e0              2
0x09a82e70    42.21 0x09a81eb0              2
0x09a518c0    34.60 0x09a50900              2
0x09a0dfd0    48.87 0x09a0d010              2
```

One craft reads a flat, digital `0.00` throttle while the other seven read
continuously fractional values (`34.60` to `61.30`) - exactly the human/AI
split `ppsspp-debugger.md` already measured independently ("the player's
craft is the one whose throttle is 0 or 100 ... the AI's is fractional").
**That one craft is also the only one with `entity + 0x368 == 0`; all seven
fractional-throttle craft read `2`.** Holding `cross` for 1.5 s and reading
again confirms it rather than leaving it as a one-shot coincidence: the same
craft (`0x09b774b0`, entity `0x09b764f0`) jumps to a digital `100.00` while
every AI craft's throttle moves to a *different* fractional value (AI
control is continuous, not fixed) - and `entity + 0x368` stays `0` for the
human and `2` for all seven AI craft, unchanged, across both reads.

This is a runtime trace, not an inference: confidence **94** (the rubric's
ceiling until a second binary corroborates it) for `0` = local human and `2`
= AI in single-player. **`1` and `3` were never produced in this pass** - a
Single Race has no network player and, per the naming/`+0x48` reading above,
those two values only matter when the game mode (`DAT_08b31048`) is `>= 0xe`
(multiplayer), which this pass never entered. They stay at confidence **60**,
unverified hypothesis, until someone reaches a multiplayer mode with the
debugger attached.

## `Hud_UpdateEnergyBar` (`0x0881c638`) tints the bar from a 20% threshold, not a gradient

2026-09-05. [hud.md](../../../ui/hud.md#still-open-after-the-frame) found the
shield bar reading cyan at 100% and solid red at 79% in two captured frames
and could not tell a threshold model from a gradient from two samples alone.
Read the colour writer directly instead of gathering more frames.

`Hud_Update` (`0x0881bf50`) dispatches to one sub-updater per active widget
type through a bitmask at the HUD object's `+0x40`; bit `0x2` calls
`Hud_UpdateEnergyBar(dt, hud)` - previously `FUN_0881c638`, immediately
after `Hud_SetEnergyBar` in the binary and reachable only through the same
`jal`-rebasing correction (`pseudo + 0x08804000`) `workflow.md` documents,
since the call renders unrebased as `func_0x00018638`. Read in full, the
relevant part is:

```c
fVar8 = max((entity_value / entity_max) * 100.0, 0);   // 0..100 percentage
// ... fill-crop math (width, x-offset) uses fVar8, unrelated to colour ...

if (iVar1 == 0) {                       // iVar1: an external override flag
    if (0.0 < fade_timer || fVar8 <= 20.0) {
        colour = (blink_alpha << 24) | 0x00ff0000;   // forced solid red
        goto apply;
    }
}
colour = (widget_colour & 0xffffff) | (blink_alpha << 24);  // authored colour, alpha only
apply:
    set all four bar-quad corners to colour;
```

`fade_timer` (`hud+0x11c`) is a one-shot flash, and its mechanics are read
directly rather than approximated: it **counts up** by the frame's `dt`
every frame the current percentage is *below the previous frame's*
(`fVar8 < hud+0x118`, itself updated to `fVar8` every frame) **or** the
timer is already running, and it resets to `0` the instant it reaches
`1.0` - a roughly one-second window per drop, re-armed by any further drop
before it expires. It is a separate accumulator from the blink cycle at
`hud+0x1dc` (scaled by `8.0`, driving the alpha toggle), which the earlier
reading of this section conflated with it. So the widget is forced red in
exactly two situations - the pool is at or under **20%**, or the pool just
*dropped* and the ~1 s window from that drop has not yet elapsed - and
otherwise renders the widget's own stored `Color` with only the alpha byte
modulated for the low-shield blink.

**Why this settles threshold over gradient, and resolves the 79%/100%
frames without contradiction**: the 100% grid frame has no recent hit and
`fVar8` far above 20, so it takes the `authored colour` branch - `ShieldBar`'s
`Color="FEConst->HudColour3"` in `Arcade_HUD.xml` (checked directly:
`Arcade_HUD.xml`'s own `<Variable global="HudColour3">` declares
`0xFF0DDFDD`, the same value [`hud.md`](../../../ui/hud.md) already reads off
Time Trial's identical table) - `0xAARRGGBB`: R=`0x0D`, G=`0xDF`, B=`0xDD` -
a light cyan, exactly what the frame shows. The 79% frame was taken **fifty
seconds in, after wall contact** per the thread's own note. A single isolated
hit gives only a ~1 s red window, which a screenshot landing inside would be
a coincidence worth flagging rather than asserting - but `shield.md`'s own
runtime leg (below) measured **115 ticks of wall contact in one 200-tick
run**, one `Ship_Damage` call per contact record per tick: sustained scraping
drops the pool on most frames, which re-arms `fade_timer` before it can
reach `1.0` and pins the bar red for the whole contact, not by chance. So the
79% frame reads as mid-scrape, not as a one-in-sixty-frames coincidence.
`0x00ff0000` in the same `0xAARRGGBB` order is pure red, matching "solid
red" exactly, and confirmed to be the shield/energy percentage bar and not
the km/h speed bar sharing this update path: the digits this function
formats are followed by the literal at `0x275de0` (rebased `0x08a79de0`),
which reads back as `"%"`, not `"kmh"`. A gradient model has no way to
produce a *sudden* full-red at 79% one hit after a full-cyan 100%; the
threshold-plus-flash model predicts precisely that.

**The `20.0` constant is not read in isolation.** It is the same value
`Hud_SetEnergyBar` (`0x0881a1c8`) already uses for its own two-tier
low-shield icon (blink below 20%, faster/different blink below 10%), and the
same value `Ship_Damage`'s `DAT_08a7b6a4` uses to fire the `energycritical`
warning sound on the downward crossing of 20% (see above). Three independent
call sites in three different subsystems - the icon, the warning sound and
this bar tint - agree on the same literal, which is why this reads as the
game's one canonical "critical" threshold rather than a coincidence.

Confidence **82**: decompilation only, capped at 84 per the
[rubric](../../../reverse-engineering/confidence-rubric.md)'s ceiling for
that evidence class, not runtime-verified for this specific function. What
would raise it: a live breakpoint on `Hud_UpdateEnergyBar` reading `fVar8`
and the resulting colour word across a controlled damage sequence (idle at
100%, a scripted hit that drops it to e.g. 60%, then a further hit below
20%) - the same probe class already used elsewhere on this page. Short of
that, the decompiled branch structure is unambiguous and every value in it
(the `20.0` floor, the `0x00ff0000` constant, the `0xffffff` mask that
preserves RGB while only alpha is blink-modulated) is a plain read, not an
inference.

~~**What is still open**: `iVar1` ... is not identified~~ - **identified
2026-09-25**, see the next section: `iVar1` is
`HullOverlay_AbsorbWindowActive(*(g_race_manager + 0x2c0))`, the player's own
craft, not "some external override" - it does not change the
threshold-vs-gradient answer either way: whichever flag it is, the function
still never blends between two colours by percentage.

## `Hud_UpdateEnergyBar`: the absorb flash (2026-09-25)

Read to settle whether, and how, the energy bar flashes white and blinks
during the absorb window - visible in the original's own frames and not
reproduced here before this pass. Full decompile on `program=BOOT.BIN`,
`/pulse/BOOT-psp-pulse-usa.BIN`, `ghidra-mcp`'s
`decompile_function(0x0881c638)`, which resolves the section above's open
`iVar1` at the same time:

```c
iVar1 = HullOverlay_AbsorbWindowActive(*(undefined4 *)(g_race_manager + 0x2c0));
```

`g_race_manager + 0x2c0` is the player's own craft - the same slot
`Hud_UpdateEnergyBar`'s caller and `cannon-quake-leachbeam.md`'s "2026-09-23
(later)" probe both already read the gate off - so `iVar1` is
**[`Readout::shield_absorbing`]**, not an unread override. Confidence
**84**, this page's own ceiling for "decompilation only, consistent call
sites" (see the [confidence rubric](../../../reverse-engineering/confidence-rubric.md)):
a direct decompile, no VFPU, of an already-named callee against an
already-identified argument, plus the standing live corroboration that the
call fires continuously from `Hud_UpdateEnergyBar` specifically (the
"2026-09-23 (later)" probe's `0x0883e908` reads) - though that probe proved
only that the *call* happens every frame, not what the three effects below
*do* with its result, so it does not lift them past the same ceiling.

**Three effects follow from `iVar1`, all in the same function body, all
decompile-only and so also capped at 84:**

1. **It suppresses the forced-red branch entirely.** The `if (iVar1 == 0) {
   if (fade_timer > 0.0 || pool <= 20.0) { forced red } }` shape means
   absorbing skips the whole red test, even under the 20% floor or mid a
   fresh hit - not merely "ignored", as the pre-2026-09-25 reading of
   [`Readout::shield_forced_red`] had it, but an active override the
   original takes every time. Confidence 82, matching this page's own
   existing rating for the surrounding threshold-plus-flash rule.
2. **It still enters the blink loop.** The accumulator at `hud+0x1dc`
   (scaled by `8.0`, `& 1` for the alpha toggle - the same field the
   "still open" note above already named) is entered whenever `pool <= 20.0`
   unconditionally, **or** `pool > 20.0` and (`iVar1 != 0` **or**
   `fade_timer > 0.0`). So absorbing blinks the bar's own alpha at the same
   4 Hz on/off cycle (8 accumulator transitions/second) the low-shield icon
   uses, independent of whether it is forced red - the two reads of `iVar1`
   are separate branches over the same tick. Confidence 80, direct
   decompile, matching the accumulator `shield.md`'s earlier reading already
   named but did not trace to a second consumer.
3. **It writes a flat `0xff`/`0x00` onto a second field of the same
   widget, unconditionally and with no blink applied:**

   ```c
   uVar3 = 0;
   if (iVar1 != 0) { uVar3 = 0xff; }
   *(undefined4 *)(*(int *)(param_2 + 0x1b0) + 0xf4) = uVar3;
   ```

   `param_2 + 0x1b0` is the `ShieldBar` widget itself - `Hud_BindWidgets`
   (`0x0881fbec`) binds it by the literal name `"ShieldBar"` and caches it at
   that same offset, so this is not a second, separately-named widget.
   `+0xf4` is written on **every** frame `Hud_UpdateEnergyBar` runs, never
   conditionally skipped, and only ever takes the two values `0`/`0xff` - a
   hard cut in and a hard cut out at the window's own edges, no fade. It is
   a different field from the corner colours (`+0xac`/`+0xb0`/`+0xb4`/`+0xb8`,
   which take the blinked `iVar4` alpha and either the forced-red or the
   authored RGB) and from every other cached offset `Hud_BindWidgets` fills
   (`+0x9c`, `+0xbc`, `+0xc4`), so it drives something else on the same
   widget rather than duplicating one of those. Confidence 82 for the write
   itself (direct decompile, unconditional every frame); confidence **60**
   for what it drives - "Plausible: a reasonable inference from surrounding
   code, could be wrong" on the rubric's own scale, not "Probable"'s
   structural fit, because only the write side was read.

   **The search for the read side, and why it came up empty this pass.**
   `search_instructions(mnemonic="lw", operand_pattern="0xf4(")` across the
   whole binary returns 80[+] hits, but `+0xf4` is a common stack-frame and
   struct offset generally - `Collision_RaycastMesh`, `BombBlast_Construct`,
   `Shuriken_Init`, `CellSelection_PopulateGrid` and dozens of others share
   it by coincidence of layout, not by touching this widget - and none of
   the hits sit inside a function that also reads `+0xac`/`+0xb0`/`+0xb4`/
   `+0xb8` (this widget's corner colours), which is the signal that would
   confirm one as the draw method. The widget class's vtable at `+0x38` is
   the more direct route - `Hud_BindWidgets` already calls through it for
   `ShieldBarMark`'s `+0xd4` slot and others - but the draw slot itself was
   not identified this pass. **What would raise it**: decompiling that
   vtable's draw slot and finding a read of `+0xf4`.

   **The fourteen `pulse-absorb-probe` frames (0.11-1.09 s into the window,
   `~0.075 s` apart) settle that a white layer exists, and rule out the
   alternative that would have needed no port change at all.** Pixel-sampled
   with PIL across the fill's own horizontal extent (`y=495`,
   `x=625..900` step 25), against the *empty speed bar's* own background
   directly above it (`y=455` - `SpeedBarBg`, not `ShieldBarBg`, which sits
   under the shield fill and is fully covered at a non-zero percentage, so
   it cannot be read off these frames directly): frames 01, 02, 05, 06, 08,
   09, 11, 12 read a uniform cyan fill (`R≈35-57, G=255, B=248-255`,
   matching `HudColour3`, `0xFF0DDFDD`); frames 03, 04, 07, 10, 13 read a
   uniform near-white fill (`R/G/B` at or near `255` across nearly the
   whole sampled width, a few samples near the right edge landing on the
   end-cap rather than the fill and reading mixed); frame 14, past the
   window, is back to a steady, unblinking `(14, 224, 222)` - `HudColour3`
   almost exactly. **No frame shows a gradient or a partial-width patch** -
   every in-window frame is uniformly cyan or uniformly white across the
   fill, and `SpeedBarBg` is pixel-identical throughout, so this is not
   bloom bleeding in from the hull glow (which would have moved that row
   too).

   **`ShieldBarBg` itself is read from the XML** (`just wad cat --expand
   <image>:PSP_GAME/USRDIR/Data.wad 'Data\XML\Arcade_HUD.xml'` - a single
   backslash, per `hud.md`'s own corrected note on this exact call): `Color=
   "FEConst->HudColour3A"`, and `Arcade_HUD.xml`'s own `<Variable global="HudColour3A">`
   declares `0x60B5D7C8` - alpha `0x60` (38%), RGB `(0xB5, 0xD7, 0xC8)`, a
   pale desaturated green-cyan, composited at well under half opacity over
   the dark track. **That is nowhere near the near-`255,255,255` the
   off-phase frames read**, which rules out "the fill's alpha drops to `0`
   and `ShieldBarBg` shows through" as the mechanism - a translucent pale
   green-cyan over a dark track cannot read as bright white. What remains
   is a same-geometry white layer, which is what the port draws. Confidence
   **82** for "a white layer exists, not an alpha-reveal of the background" -
   pixel data plus the XML's own authored colour, still short of a runtime
   trace of the draw call itself; the vtable's draw slot is what would
   settle *how* it is drawn, not whether one exists.

   **The blink's phase does not read as zero at the absorb.** The thirteen
   in-window frames read `C C W W C C W C C W C C W` in order - a first
   "off" stretch spanning two samples (03, 04) starting around 0.26 s into
   the window, then single-sample "off" hits roughly every 0.23-0.30 s
   after (07, 10, 13), consistent with a period near the measured 4 Hz
   once sampling-rate aliasing against the `~0.075 s` capture interval is
   accounted for. **A zero-initialised accumulator at the absorb's own
   start would put the first "off" half at `0.125-0.25 s`**, one half-cycle
   earlier than what these frames show. That is not a contradiction of the
   port: `hud+0x1dc` is read as a shared, freezing (not resetting)
   accumulator with no connection to the absorb trigger at all - the same
   shape `Race::advance_shield_blink`/`shield_blink_step` already
   implement, see below - so an absorb beginning mid-cycle, with whatever
   phase the icon's own earlier blinking (or a prior post-hit flash) left
   it at, is exactly what the freeze-not-reset model predicts and a
   zero-reset model would not. It is not evidence the phase is right,
   only that the *shape* (freezes rather than resets on the absorb) is -
   this capture's own pre-absorb history is not known, so there is nothing
   to check the resulting phase against.

**Ported** as `oag_hud::Readout::shield_absorbing` (wired off
`Race::absorb_window_active`, which reuses `Race::view.absorb_overlay`
rather than a second timer - the same `0.0 <= elapsed <= WINDOW` test
`HullOverlay_AbsorbWindowActive` itself is), `Readout::shield_blinking`/
`shield_blink_phase_on` for point 2, and `oag_hud::draw::draw_list`'s
`ShieldBar` arm for point 3 - a flat white copy of the bar's own cropped
fill, painted underneath it, standing in for the unchased draw target named
above. **The blink's own accumulator is ported, not approximated**:
`Race::view.shield_blink_timer` (`crate::race::Race::advance_shield_blink`)
is `hud+0x1dc` itself - it advances only while
[`Readout::shield_blinking`]'s three conditions hold and freezes rather
than resets the rest of the time, the same shape `hud+0x11c`'s existing
`shield_flash_step` already carries for the post-hit flash - so two blinks
in the same race can start at different phases, same as the original. One
assumption stands in for a read: the timer starts at `0.0` on a fresh race,
which was not checked against `Craft_Construct_q` or the HUD object's own
constructor this pass - see `crate::race::view::View::shield_blink_timer`'s
doc comment. **Chosen, not measured**: only the flash layer's own geometry
and blend mode (point 3's own gap) remain so - the blink's phase is no
longer one of them, unlike the equivalent gap `hud::runtime::phase_on`
still carries for HD's own blink.

## `Ship_AddShield`'s four callers, and `Ship_RefillLapShield` (`0x0883de30`)

Read 2026-09-16, because the roadmap carried "the pit-lane recharge is the
one left" against this item and nothing on this page said where a pit lane
was. There is none in this title. `Ship_AddShield` (`0x0883ddc8`) is the only
way energy comes *back* into the pool short of `Ship_ResetShield`, and it has
exactly four callers:

| Caller | What it adds | Port |
| --- | --- | --- |
| `Zone_Update` (`0x0882f700`) | `g_zone_recharge` on a clean zone ([zone-mode.md](zone-mode.md)) | `oag_race::zone` |
| `Ship_ApplyPendingWeaponRepair` (`0x0883f228`) | the LeachBeam's repair ([cannon-quake-leachbeam.md](cannon-quake-leachbeam.md)) | `oag_weapons::projectile::leach_beam` |
| `FUN_08844ec4`, the absorb handler | the held weapon's own `<Stats absorb>`, thirteen arms ([pickups.md](../../../gameplay/pickups.md)) | `Race::spend_pickup` |
| **`Ship_RefillLapShield` (`0x0883de30`)** | **a fifth of the maximum, on a completed lap in an Eliminator** | `Race::eliminator_lap_health_refill` |

The fourth is two lines:

```c
void Ship_RefillLapShield(Entity *entity) {                         // 0x0883de30
    Ship_AddShield(stats_base[0x84 + g_skill_level * 4] * 0.2f, entity);
    Ship_PlayAbsorbFeedback(entity);                                // 0x08840640
}
```

The table cell is the same one `Ship_ResetShield` fills the pool from - the
skill-indexed maximum - so the refill is **20 % of full, clamped at full** by
`Ship_SetShield`. Its two callers are the two Eliminators. `Eliminator_UpdateKillTarget`
(`0x0882ce18`, [race-campaign.md](race-campaign.md)) keeps the player's last
crossing count at `mode+0x1a10` and calls it when the count has gone up:

```c
crossings = player->craft->crossings;                               // craft+0xac8
if (mode->last_crossings != 0 && Ship_State(player) == 1 && mode->last_crossings < crossings)
    Ship_RefillLapShield(player);                                   // mode+0x2c0
mode->last_crossings = crossings;
```

`FUN_08822a50`, the multiplayer Elimination object's racing-state update
(`FUN_08822918` dispatches on `mode+0x7c8`, and `2` is this case), is the same
five lines against `mode+0x1aa4`. Three things fall out. **It is the player's
craft only** - `mode+0x2c0`, the same slot `Race_CreatePlayer` fills
([race-progress.md](race-progress.md)); an AI craft's laps refill nothing,
which is consistent with the AI never absorbing either. **The first crossing
does not count** - `last_crossings != 0` skips the start-line crossing a craft
makes leaving the grid. **And it is gated on the racing state**, so a craft
that crosses while exploding gets nothing. Confidence **85** for the amount
and the gate (a clean decompile, no VFPU), **80** for the multiplayer twin
(its object is identified by its size and its field offsets, not by a name).

`Ship_PlayAbsorbFeedback` (`0x08840640`) was on
[contact-response.md](contact-response.md) as `FUN_08840640`, the absorb
effect: `Sound_Play(1.0, entity+0x50, bank, 0, "ABSORB", 0)` once, then
`ShipCollisionFx_Trigger(1.0, node, 2, 0)` - kind 2 is `WO_WEAPON_ABSORB` -
over up to ten of the craft's fx nodes with `DAT_08abf564 = i * 0.1` staggering
them a tenth of a second apart. Naming it here because its callers are what
settle where `ABSORB` is heard, and they are **four, all read**: this refill,
the absorb handler's tail (`0x088455b8`, skipped in an Eliminator, where the
absorb pays no energy and spends the weapon on a Shield instead - see
[race-modes.md](../../../gameplay/race-modes.md#eliminator)), and two arms of a network callback
(`FUN_0883d5c0`: message `'B'`, and `'Q'` for a Quake absorbed remotely).
**None is a wall contact.** The contact loop's shield branch takes
`ShipShield_Hit` (`0x0885eb04`) instead of `Ship_DispatchCollisionFx`, and
`ShipShield_Hit` writes four colour words and a `1.1` timer and plays nothing -
so a shielded contact is *silent*, and this project's `ABSORB` on that edge was
an invention, removed the same day. Confidence **88** for the callers (a
complete xref list) and the silence (both gates on
[shield-pickup.md](shield-pickup.md) read at instruction level).

**Ported**: `Race::eliminator_lap_health_refill` adds
`LAP_REFILL_FRACTION` (`0.2`) of the maximum through `oag_physics::damage::add`
on the racing gate and raises `Cue::Absorb`; the absorb handler's own `ABSORB`
is raised on the same cue. **The staggered `WO_WEAPON_ABSORB` burst is drawn
as of 2026-09-23**: both paths call `Race::play_absorb_feedback`
(`oag_raceplay::absorb`), which raises the cue and attaches one instance per
`Ship Collision Fx` node on the stagger below. One departure, stated on the port: it refills every craft's lap,
not the player's alone, so a field of opponents that the player never shoots
does not wear itself down; `slot` is what to narrow if that is the wrong
call.

### Which ten nodes, and what the stagger does (2026-09-23)

The previous pass stopped at "the craft's fx nodes". Read to the end:

```c
void Ship_PlayAbsorbFeedback(Entity *e) {                          // 0x08840640
    if (FUN_0883e37c() && e->fx_count /* +0xca8 */ != 0) {
        Sound_Play(1.0, e->emitter /* +0x50 */, bank, 0, "ABSORB", 0);
        for (i = 0; e->fx[i] /* +0xc80 */ && i < 10; i++) {
            DAT_08abf564 = (float)i * 0.1f;
            ShipCollisionFx_Trigger(1.0, e->fx[i], 2, 0);
        }
    }
}
```

- **The list is the hull's `Ship Collision Fx` nodes.**
  `Ship_GatherCollisionFxNodes` (`0x0883ea50`, renamed from `FUN_0883ea50`)
  zeroes the ten words at `+0xc80` and calls `Vex_CollectNodesOfType`
  (`0x08a6d79c`, renamed from `FUN_08a6d79c`) on the live hull root at
  `+0x8b0`. It stores the count at `+0xca8`. `Vex_CollectNodesOfType` is a
  pre-order walk (first child `+0x10`, next sibling `+0xc`) that keeps every
  node whose type word `+4` matches the query's, up to the cap. The query's type is
  `FUN_08a6ba70`. That is the derived-class token which
  `FUN_08924bf0`, the `0x3d0` `Ship Collision Fx` registration, installs
  second, after the generic base token `FUN_08a6ba64` that 25 other
  registrations share. `FUN_08a6ba70` has only five callers, all in
  `Ship Collision Fx` code. `Ship_DispatchCollisionFx` reads the same
  `+0xc80` list as those instances
  ([contact-response.md](contact-response.md)). The absorb therefore plays
  on every node the collision sparks pick the nearest of: six on Assegai.
  The list is re-gathered by `FUN_0883eae8`/`FUN_0883eb68` whenever the live
  hull at `+0x8b0` is swapped between the two models at `+0x8b4`/`+0x8b8`.
  Confidence **85**.
- **The gate:** `+0xca8 != 0`, so a hull with no nodes gets **no sound
  either**. `FUN_0883e37c` is a display-mask test (`FUN_0891e908(g_display)
  & FUN_0897bc08(...)`) that was not read further. The port raises the cue
  regardless, as it did before, and only the picture waits on the node
  count. This is recorded as a difference, not fixed.
- **`DAT_08abf564` is a start delay.** Its only reader is
  `PsysNode_Start_q` (`0x08916200`, renamed from `FUN_08916200`, 65). That is
  the particle-node set-up `ShipCollisionFx_Trigger`'s spawn reaches, and it
  moves a positive value into the node's `+0xb8` and zeroes the global.
  `PsysNode_Update` (`0x08915cdc`, renamed from `FUN_08915cdc`, 80) subtracts
  the frame's `dt` from `+0xb8` while it is positive. Only after that does it
  compose the node's world matrix from its parent (the locator) and run
  `ParticleSystem_Update`. So node `i` starts `i * 0.1` s late and then rides
  the hull. Confidence **80** for the delay, **65** for `PsysNode_Start_q`'s
  role as a whole (only this branch of a long set-up was followed).
- **Severity is not set** for kind 2 (see `ShipCollisionFx_Trigger` on
  [contact-response.md](contact-response.md)), so the effect plays as
  authored.

The port is `oag_raceplay::absorb::PULSE_ABSORB_BURST`. It plays one
`WO_WEAPON_ABSORB` per locator through `psys::Stage::play_riding`, follows
the locator while the emitters run, and lets go after. `crates/game/tests/absorb_ground_truth.rs` pins six
bursts on Assegai, the second starting a tenth of a second in. Pure's twin
(`FUN_08925e20`) loops eight times, per
[`psp-pure-usa/rocket-and-collision-fx.md`](../psp-pure-usa/rocket-and-collision-fx.md).
HD's is a different mechanism on a different node class; see
[`ps3-hdfury-eu/absorb-feedback.md`](../ps3-hdfury-eu/absorb-feedback.md).

## Contact damage is charged every tick of a sustained graze

Moved from the `CONTACT_DAMAGE_SCALE` doc comment in `crates/physics/src/damage.rs`
(checked 2026-09-06, prompted by `07_Track`'s Ace opponent ending at shield `0.00` at
every `Tuning::look_speed` tried; `docs/gameplay/ai.md`).

Charging `0.05 * 0.7` per `impulse_sum` on **every** tick of a scrape is what the original
does: `FUN_088418e0`'s contact loop runs every tick the ring holds a record and gates
nothing on contact duration (the `steer-left.inputs` leg above: 115 of ~200 ticks in wall
contact, pool drained monotonically). `Body_RecordContact`'s third argument is `p`, the
full normal-plus-tangential impulse that `wall::resolve` sums, so a tangential graze and a
hard impact follow one rule.

Two other candidates for an over-charge came back clean:

- **The ring cap.** `Body_RecordContact` rejects an append past 8 entries a tick
  (`body+0x370`, `n < 8`); this crate does not reproduce the cap. `resolved_count` never
  exceeded 5 on `07_Track` and `13_Track` (lone Ace, 18,000 ticks each), so it would be a
  no-op and was not implemented.
- **Fixed 60 Hz against the original's variable step**
  ([ADR-0007](../../../architecture/adr/0007-fixed-timestep-vs-original.md)).
  `docs/psp/frame-pacing.md` puts racing `dt` at `0.016396`-`0.016973`; an overrunning
  frame gives a larger `dt`, so *fewer* `Ship_Damage` calls per second, not more.

What varies the per-tick charge is the contact law: a flat-wall probe at a fixed 28-degree
approach with no thrust converges on **12.84 %** velocity loss a tick, 73 % the restitution
bounce and 27 % the `0.035` friction floor. Across both circuits' scrape ticks the
fractional loss has a **median of 3.55 %** (near-tangential, the friction floor) and a tail
past 15 % on steeper-incidence ticks. A lap grinding at a steep angle is charged more per
tick because the recovered law says so; the cadence and the constant stay, and `07_Track`'s
destruction is a driving-line question (`docs/gameplay/ai.md`), not a damage-model one.

## What is not verified

- **Whether `1` means "network human" and `3` is genuinely unused** is still
  an inference from `Ship_Damage`'s branching, not a read of an enum -
  confidence 60. Closing it wants the same probe
  (`scripts/psp-watch-controller-kind.py`) run against a multiplayer session
  (ad hoc or infrastructure), which nothing in this project currently reaches.
- **`weapon_kind`'s nine cases are unmapped.** They select telemetry buckets
  and nothing else here; naming them wants the weapon table, which is
  unstarted.
- ~~**The absorb-spark burst is not drawn**~~ - drawn 2026-09-23. See
  "Which ten nodes, and what the stagger does" above. Nothing here has been
  compared against a capture of the original's absorb.

## History

- 2026-09-24: **`Ship_Damage`'s weapon branch read and measured**: it throws the
  struck hull's damage sparks, not absorb sparks, on every landed weapon hit.
  `CockpitHitFx_Arm_q` named. Ported as `oag_raceplay::hit_sparks`.
- 2026-09-23: **the absorb burst's node list and delay read**:
  `Ship_GatherCollisionFxNodes`, `Vex_CollectNodesOfType`, `PsysNode_Start_q`
  and `PsysNode_Update` are named. The burst is drawn.

- 2026-09-16: **`ArcadeRace_Update` / `ArcadeRace_UpdateRacing` named** -
  the reader of the destroyed bit that ends a single race for the player,
  and the reason an AI craft, which nothing watches, respawns through state
  6 instead. Ported as the single race's opponent respawn.
- 2026-09-16: **`Ship_AddShield`'s four callers enumerated**, closing "the
  pit-lane recharge" (there is none) with the one caller the port had
  guessed at: `Ship_RefillLapShield` (`0x0883de30`) gives back 20 % of the
  maximum on a completed lap in an Eliminator, player only, racing state
  only. `Ship_PlayAbsorbFeedback` (`0x08840640`) named from its callers,
  which also settle that a shielded contact plays nothing.
- 2026-09-05: **`Hud_UpdateEnergyBar` (`0x0881c638`) named and read in
  full**, closing the handover thread on whether the shield bar's colour is
  a threshold or a gradient. It is a threshold: solid red at or under 20%
  or during a one-shot post-hit flash, the widget's own authored colour
  (cyan, `0xFF0DDFDD` off the XML) otherwise - never a blend. Confidence 82,
  decompilation only.
- 2026-08-25, live pass: `entity + 0x368` read off all eight craft in a
  breakpoint-driven Single Race, twice (idle, then holding thrust). `0` on
  the one craft with a digital throttle (the human), `2` on all seven
  fractional-throttle craft (AI), unchanged across both reads - 60 -> 94 for
  that mapping in single-player. `1` and `3` were not produced (no
  multiplayer session reached) and stay at 60.
- 2026-08-25: **the writer of `entity + 0x368` found**: `Craft_Construct_q`
  (`0x08840c74`) stores its own second argument there verbatim, 60 -> 80 for
  the writer. No static caller of `Craft_Construct_q` exists (no `jal`
  xref, no data reference to its address), so which value each real craft
  gets is still unread; "local human player slot" is refuted by `Ship_Damage`
  treating `0` and `2` identically while excluding `1` and `3`, replaced with
  an unverified controller-kind hypothesis, confidence 60.
- 2026-08-10, second pass: `Race_ReadSetupOptions`'s `g_weapons_enabled` switch
  spelled out in full (three groups, not two: locked-off, locked-on, and
  overridable) and checked against a live menu walk of all seven RACE TYPE
  entries, 65 -> 88. Settles `oag_race::Mode::weapons_enabled` and
  `Mode::has_opponents` for the three modes this crate implements (all
  `false`, all confirmed against the greyed `WEAPONS`/`AI DIFFICULTY` rows),
  and independently corroborates Eliminator as mode `8` from
  [pads.md](pads.md)'s `elimination_refresh_time` reading. See
  [race-modes.md](../../../gameplay/race-modes.md) for what changed in the
  reimplementation.
- 2026-08-10: **the pool's object corrected from the craft to the ship
  entity**, `*(craft + 0x1c4) + 0x88`. The first reading took the
  decompiler's `craft` parameter name for the object the trace harness breaks
  on; they are two structures, and a capture at `craft+0x88` records an
  orientation-matrix element. Caught by looking at the column, which ran
  smoothly between -1 and 1 - a wrong offset here does not produce garbage, it
  produces a plausible-looking quantity.
- 2026-08-10: runtime leg (above). The pool, the two globals, the regeneration
  branch and the weapons-off halving move to 90-94, and **the `0.035`
  coefficient measures exact on 25 of 25 calls**, so it moves to 94 too. Only
  `entity + 0x368` is left unmeasured.
- 2026-08-10: page created. `Ship_ResetShield`, `Ship_Damage`, `Ship_State`,
  `Ship_SetState`, `g_weapons_enabled`, `g_damage_enabled` and `g_skill_level`
  named; the `0x84 + skill * 4` maximum joined to the PS2 parser's three
  `<Misc>` slots.
