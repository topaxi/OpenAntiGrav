# The shield pool, and what damages it

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** the pool, its maximum, its one initialiser and its one damage
entry point are read end to end, and **every claim except one now has a
runtime leg** (2026-08-10, PPSSPP). Confidence **94** for where the pool and
its maximum live, for the damage coefficient and for the one-call-per-contact
loop; **92** for the two race-option globals and the regeneration branch;
**90** for the weapons-off halving; **60** for the entity field that gates
the whole path.

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
  whose `+0x368` is zero. [`oag_game::hud`](../../../../crates/game/src/hud.rs)
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
8. Spawn one or two absorb sparks through `ShipCollisionFx_Trigger`, for
   `source == 2` only.

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

## The two race options that reach the pool

`Race_ReadSetupOptions` (`0x08896b84`, named this pass at confidence 80 - the
body is a flat run of key lookups and the keys are literal strings, but no
caller was read) reads the race setup as string key/value pairs - `Team`,
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

## What is not verified

- **`entity + 0x368` is not identified**, and it gates the entire shield path:
  `Ship_Damage` touches the pool only when it is 0 or 2, `Ship_AddShield`
  counts telemetry only when it is 0, and several sites test `< 0`
  explicitly, so `-1` is a real value. "Local human player slot" fits every
  site read so far and would imply AI craft take no damage through this
  function, which is almost certainly wrong - so the reading is **60** and
  the field keeps its offset rather than a name. Whoever finds the writer
  closes it.
- **`weapon_kind`'s nine cases are unmapped.** They select telemetry buckets
  and nothing else here; naming them wants the weapon table, which is
  unstarted.
- **The absorb-spark branch is not implemented** and is weapon-only.

## History

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
