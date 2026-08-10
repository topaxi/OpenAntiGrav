# The shield pool, and what damages it

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** the pool, its maximum, its one initialiser and its one damage
entry point are read end to end. Confidence **88** for where the maximum
comes from (two binaries, writer and reader), **84** for the damage function
and the two race-option globals, **60** for the craft field that gates the
whole path.

Everything here except the maximum is decompilation plus instruction-stream
reading, with no runtime trace, so it caps at **84** per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md).

This page exists because "shield and energy" was the M5 item blocking three
others at once: the HUD's `ShieldBar` had nothing to show,
[Zone](zone-mode.md) had no end condition, and weapon damage had nothing to
subtract from. [`contact-response.md`](contact-response.md) had already read
the call - reaction #2 of `FUN_088418e0`'s contact loop - and left the callee
unnamed pending exactly this pass.

## The pool is `craft + 0x88`, and its maximum is skill-indexed

`Ship_Shield` (`0x0883e68c`) and `Ship_SetShield` (`0x0883e6f4`) were named by
the Zone pass on the strength of their call sites. Read in full they settle the
field:

```c
// Ship_SetShield(float amount, Craft *craft)
max = *(float *)(*(int *)(craft + 0x94) + 0x6c) + g_skill_level * 4 + 0x84);
if (amount > max) amount = max;
...
*(float *)(craft + 0x88) = amount;
if (*(int *)(craft + 0x368) == 0)
    Hud_SetEnergyBar((*(float *)(craft + 0x88) / max) * 100.0, DAT_08ab0838);
```

Four things follow, and each is a behaviour rather than a layout note:

- **The pool is clamped above and not below.** `Ship_SetShield` takes
  `min(amount, max)` and stores it. Nothing floors it at zero, which is what
  makes `Ship_Damage`'s `<= 0.0` test below reachable.
- **The HUD reads a percentage, not the raw pool**, and only for the craft
  whose `+0x368` is zero. [`oag_game::hud`](../../../../crates/game/src/hud.rs)
  already formats `ShieldBarText` as a percentage; this is the reading that
  makes that right rather than a guess.
- **`craft + 0x94` holds the object whose `+0x6c` is the stats base.** The
  same base [`handling-stats.md`](../../../formats/handling-stats.md) accounts
  for with no gap.
- **`Ship_Shield` has a second source in the multiplayer modes.** With
  `DAT_08b31048 >= 0xe` it reads `DAT_08b313dc[craft + 0x364] + 0x180` instead
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
void Ship_ResetShield(Craft *craft)
{
    if (g_game_mode < 0xe)
        Ship_SetShield(stats_base[0x84 + g_skill_level * 4], craft);
    else if (craft->0x364 == FUN_0895ebf0())   // this machine's own player
        Ship_SetShield(stats_base[0x84 + g_skill_level * 4], craft);
}
```

The whole function. It is the only thing on the disc that sets the pool to its
maximum, so a craft starts a race full and nothing else refills it outright -
`Ship_AddShield` (`0x0883ddc8`) adds a delta and is what Zone's clean-zone
recharge uses. **Not** called from `Ship_InitCraft` (`0x08849354`), which is
worth stating because that is where it was looked for first: `Ship_InitCraft`
writes forty-odd craft fields and `+0x88` is not among them.

Confidence **84**. The body is unambiguous; what is not read is every call
site, so "the only thing that fills the pool" is a statement about this
function, not a survey.

## `Ship_Damage` (`0x088439ac`) is the single subtraction

Named this pass. Signature, from its five call sites and its own body:

```c
void Ship_Damage(float amount, Craft *craft, int source, int weapon_kind, char by_rival)
```

The order of operations, with the branches that matter:

1. **Refuse outright** when `Ship_State(craft)` is 4, 5, 6 or 2, or when
   `craft->0x860 & 0x1000` is set. Those are the destroyed, respawning and
   otherwise-not-racing states - a craft already blowing up takes no further
   damage.
2. **Halve the amount when weapons are off**: `if (g_weapons_enabled == 0)
   amount *= 0.5`. A real recovered rule, and a surprising one - the option
   that removes weapons also softens every *collision*.
3. Accumulate an integer copy of the amount into telemetry buckets selected by
   `source`, with a nine-way sub-bucket on `weapon_kind` for weapon damage.
4. **Subtract**, but only in states 1 and 3:
   `new = Ship_Shield(craft) - amount`. In every other state the pool is
   re-stored unchanged, which still pushes the HUD.
5. **Warn on the downward crossing of 20 %.** `DAT_08a7b6a4` is `20.0`, read
   from `.rodata`, and the test is on the percentage before *and* after -
   `before > 20 >= after` plays `energycritical`. A level test would re-fire
   every frame of a scrape; this one cannot.
6. `Ship_SetShield(new, craft)`.
7. **Destroy at zero or below**: `if (new <= 0.0) Ship_SetState(craft, 4)`,
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
if (record->0x1a0 > 0.0 && Ship_State(craft) == 1)
    Ship_Damage(fVar22 * 0.7, craft, record->0x170 != 0, 0, 0);
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
    s = Ship_Shield(craft) + dt * 4.0;
    if (s < 20.0) s = 20.0;
    Ship_SetShield(s, craft);
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

`Ship_State` returns `craft->0x8c`, unless `craft->0x860 & 0x800` is set - the
networked craft flag - in which case it asks `FUN_0894db4c(craft->0x364)` for
the remote player's state instead.

`Ship_SetState` writes `craft->0x8c` and runs the transition. The states this
page needs:

| State | What the setter does |
| --- | --- |
| 3 | Respawn. Plays `RESET`, and stores `clamp(Ship_Shield(craft) - 1, 0, 5)` into `craft->0x44` - **a respawn cost, capped at 5**, or a flat `10` in game mode 6 |
| 4 | Destroyed. Plays `_BLOWUP`, hides the HUD, puts the camera in mode 5 |

Confidence **84** for both names, **70** for the respawn-cost reading: the
value is computed and stored, and what consumes `craft->0x44` is not read.

## What is not verified

- **No runtime leg on any of it.** The cheapest one is a breakpoint read of
  `craft + 0x88` across a wall hit, which would test `|p| * 0.035` directly
  against the impulse the same capture already records.
  `scripts/psp_trace_fields.py` carries the column as of this pass, so the
  capture is a `just autopilot` run away.
- **`craft + 0x368` is not identified**, and it gates the entire shield path:
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

- 2026-08-10: page created. `Ship_ResetShield`, `Ship_Damage`, `Ship_State`,
  `Ship_SetState`, `g_weapons_enabled`, `g_damage_enabled` and `g_skill_level`
  named; the `0x84 + skill * 4` maximum joined to the PS2 parser's three
  `<Misc>` slots.
