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
`Craft_Construct_q`'s own second argument (decompilation only); **60** still
for what values `1` and `3` mean, since neither was ever produced in this
pass's single-player race and multiplayer was not reached.

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
| 8 | `0x088446ec` | Sets `entity->0x874` to `1.0` (not networked) or `2.0` (networked, per the same `entity->0x368` test) - **the opposite ratio from state 6** (there the local player got the *longer* timer; here the non-local craft does) - then calls `FUN_0018dd94` again on the `0x3bc`-relative sub-object. Falls into the same common tail. Not enough here to guess what distinguishes state 8 from state 6 beyond the timer and the inverted local/remote ratio - both look like "some kind of timed lockout, direction of the asymmetry depends on which state" rather than one being clearly the false start and the other something else. |

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

**What is still open**: `iVar1` (`func_0x0003a904` called on
`*(_DAT_0005801c + 0x2c0)` - a global race-state pointer `Hud_Update` itself
also dereferences at that same offset, not a field of the HUD object) is not
identified - it is read as "some external override that suppresses the
forced-red branch entirely", plausibly a practice/ghost or no-damage-mode
flag, but nothing here names it. `func_0x0003a904` rebases to `0x0883e904`,
which falls in an undefined gap between two functions rather than inside
one, so it was not chased further this pass. It does not change the
threshold-vs-gradient answer either way: whichever flag it is, the function
still never blends between two colours by percentage.

## What is not verified

- **Whether `1` means "network human" and `3` is genuinely unused** is still
  an inference from `Ship_Damage`'s branching, not a read of an enum -
  confidence 60. Closing it wants the same probe
  (`scripts/psp-watch-controller-kind.py`) run against a multiplayer session
  (ad hoc or infrastructure), which nothing in this project currently reaches.
- **`weapon_kind`'s nine cases are unmapped.** They select telemetry buckets
  and nothing else here; naming them wants the weapon table, which is
  unstarted.
- **The absorb-spark branch is not implemented** and is weapon-only.

## History

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
