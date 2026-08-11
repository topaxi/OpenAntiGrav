# `WeaponStats_*.xml`: the weapons are authored data

**Status: the schema is read end to end, confidence 92.** Every weapon's
tunables, the disturber effects and the pickup distribution are **authored XML on
the disc**, not a table compiled into the executable.

**Partly implemented since 2026-08-11.** `oag_formats::weapons` decodes the three
`absorb`/`time` weapons, `absorb` for all thirteen, `<Pickupodds>` in full and
`<Global> slowdown_limit`; a race reads the file, a `Weapon Pad` draws from the
odds and Turbo is the one weapon with an effect. See
[pickups](../gameplay/pickups.md), which carries what is recovered and what is
this project's. Everything else on this page is still decoded nowhere, for the
reason the parser's own docs give: a field with no consumer is a field nobody
has checked.

That is the headline, because it changes what the weapons item on the
[roadmap](../overview/roadmap.md) costs: it is a format decode plus behaviour,
the way `handlingstats.xml` was, rather than a from-scratch recovery of a code
table. The parser was found by following the class-name strings at `0x08a78c00`
out of `.rodata`.

Per [ADR-0006](../architecture/adr/0006-no-copyrighted-content.md) this page
records **attribute names and structure only**. The values are the game's tuning
data; read them off your own disc with:

```sh
just wad cat --expand <image>:PSP_GAME/USRDIR/Data.wad 'Data\XML\WeaponStats_Race.xml'
```

## Two files, one schema

| Entry | What it is |
| --- | --- |
| `Data\XML\WeaponStats_Race.xml` | the ordinary race modes |
| `Data\XML\WeaponStats_Elimination.xml` | Eliminator |

Both are read by the same parser - `WeaponStats_Parse` (`0x0880db7c`) - so a mode
swaps the whole weapon table rather than patching it. The two path strings sit at `0x08a78cf8` and `0x08a78d18`.

## Structure

```xml
<WeaponStats>
  <Weapon type="Global">  <Stats .../> </Weapon>
  <Weapon type="Rocket">  <Stats .../> </Weapon>
  ...
  <DisturberOdds>
    <Weapon type="Rocket">
      <Odds  .../>
      <Times .../>
    </Weapon>
    ...
  </DisturberOdds>
  <Pickupodds class="Venom">
    <Weapon type="Autopilot"> <Stats ai back front human/> </Weapon>
    ...
  </Pickupodds>
  ...
</WeaponStats>
```

Three sections, and the third is the one a pickup system needs.

### The weapons

Fourteen `type` values, in the order the class-name pool at `0x08a78c00` holds
them: `Weapon`, `Rocket`, `Missile`, `Quake`, `Cannon`, `Turbo`, `Shield`,
`Autopilot`, `Plasma`, `Bomb`, `Mine`, `LeachBeam`, `Repulser`, `Shuriken`, plus
`Global`. Each has its own parser, so the attribute set is per weapon rather than
a union:

| `type` | Parser | `<Stats>` attributes |
| --- | --- | --- |
| `Global` | `0x0880dab0` | `slowdown_limit` |
| **`Rocket`** | **`0x0880c058`** | **`absorb blastforce blastradius damage venomspeed flashspeed rapierspeed phantomspeed launchSpeed spread`**, plus `slowdown_time` |
| `Missile` | `0x0880c31c` | the rocket's, less `spread`, plus `lock_max_dist lock_min_dist` |
| `Quake` | `0x0880c60c` | `absorb damage radius slowdown_time` |
| `Cannon` | `0x0880c774` | `absorb rounds rate damage_per_bullet slowdown_time` |
| **`Turbo`** | **`0x0880c92c`** | **`absorb time`** |
| **`Shield`** | **`0x0880ca2c`** | **`absorb time`** |
| **`Autopilot`** | **`0x0880cb2c`** | **`absorb time`** |
| `Plasma` | `0x0880cc2c` | the rocket's, plus `charge_time` |
| `Bomb` | `0x0880cef0` | `absorb blastforce blastradius damage damageradius slowdown_time trigger_radius timetodie` |
| `Mine` | `0x0880d124` | the bomb's, less `damageradius` |
| `LeachBeam` | `0x0880d328` | `repair absorb damage lock_max_dist lock_min_dist slowShipFactor range active_time energy_multiplier` |
| `Repulser` | `0x0880d58c` | `blastforce blastradius absorb damage slowdown_time blast_time wave_time` |
| `Shuriken` | `0x0880d790` | `absorb rhicochetForce blastForce blastradius rhicochetdamage blastdamage slowdown_time <class>speed launchSpeed fuse` |

**Bold rows are decoded by `oag_formats::weapons`**; the rest are named here and
read no further, because [nothing consumes them](../gameplay/pickups.md) and a
field decoded with no consumer is a field nobody has checked. The Rocket is
partly bold for the same reason: `oag_gameplay::projectile` reads nine of its
eleven attributes, and the two left plain are `slowdown_time` - half of the
slowdown mechanic, whose other half is `<Global slowdown_limit>` and which has
no consumer - and `spread`.

**`spread` is the half-angle of a three-rocket fan, in radians**, and that is
recovered rather than inferred: `Weapon_FireRocket` (`0x0886e104`) spawns one
rocket straight ahead, one rotated by `+spread` and one by `-spread`. Its
absence from the Missile follows - a homing weapon has no use for a launch fan.
See [weapon-fire.md](../ghidra/functions/psp-pulse-usa/weapon-fire.md), which
also carries the **struct offsets** for the whole Rocket block.

**`absorb` is on every one of them**, which is what `Ship_Damage`'s absorb branch
spends: [`shield.md`](../ghidra/functions/psp-pulse-usa/shield.md) records
`ShipCollisionFx_Trigger`'s `WO_WEAPON_ABSORB` effect and this is the number
behind it - absorbing a weapon instead of firing it pays energy back.

**The three highlighted rows share one schema, `absorb` and `time`**, and they
are the three weapons that damage nobody, parsed by
`WeaponStats_ParseTurbo`, `WeaponStats_ParseShield` and
`WeaponStats_ParseAutopilot`. That is what makes them the cheap ones
to build: no projectile, no target, no blast. Their parsers are byte-identical
except for the two offsets they write, at `+0x84`/`+0x88`, `+0x8c`/`+0x90` and
`+0x94`/`+0x98` of the stats block - three adjacent eight-byte slots in the
class-name order.

**Projectile speed is per speed class**, spelled out rather than indexed:
`venomspeed`, `flashspeed`, `rapierspeed`, `phantomspeed`.

**And those four are authored in km/h, not units per second.** Confidence 84,
and it belongs here rather than only in the page that found it:
`Rocket_SpeedForClass` (`0x0885d1b0`) is a pure table lookup that returns the
authored float for the current class - `+0x08`/`+0x0c`/`+0x10`/`+0x14`, exactly
the offsets tabulated above - and **both** of its callers divide the result by
`3.6` before it becomes a velocity. Nothing between the parse and that divide
scales it. A reimplementation that spends these numbers as units per second
therefore flies the projectile **3.6x too fast**. See
[rocket-visuals.md](../ghidra/functions/psp-pulse-usa/rocket-visuals.md), which
reads the two call sites; this note adds a unit to the attribute names above and
does not revise them.

### `<DisturberOdds>`: what a hit does to you

Per weapon, two sibling elements with matching attribute sets - a weight and a
duration for each effect:

`afterburner_loss`, `distorted_vision`, `steering_impaired`, `loss_of_hud`,
`fire`, `smoke`, `weapon_system_failure`, and the same seven again in `<Times>`
with a `_time` suffix.

Seven authored ways to be inconvenienced by a hit, none of them implemented and
several of them (`loss_of_hud`, `distorted_vision`) presentation rather than
simulation.

### `<Pickupodds class="...">`: what a pad hands out

One block per speed class, each listing every weapon with
`<Stats ai back front human/>`.

Four weights, and their shape is the whole pickup design:

- **`ai` and `human`** weight the same weapon differently depending on who
  crossed the pad.
- **`front` and `back`** weight it by grid position, which is what stops the
  leader being handed the same thing as the tail.

### The fifth class: the code knows the name, the data does not use it

`WeaponStats_Parse` tests the `class` attribute against **`Vector`** and then
**discards the result**, before testing `Venom`, `Flash`, `Rapier` and `Phantom`,
each of which stores a class index of 0/1/2/3. So the executable carries a fifth
name with nothing behind it.

The shipped race table authors **four** blocks and no `Vector`, which is measured
rather than inferred: `crates/formats/tests/weapons_ground_truth.rs` asserts both
halves. This page said the opposite for the length of one draft, having read the
parser and not the file - the ground-truth test is what caught it, which is what
it is for.

That is evidence toward [handling-stats.md](handling-stats.md)'s open question of
whether Pulse has a fifth speed class, and it points **away** from one: a class
the code names, discards, and no shipped file authors is a leftover rather than a
rung.

## What this does not answer

- **What the original does on a pickup.** The *trigger* is recovered -
  `WeaponPads_TestCraft` stamps the pad's refresh timer, [pads.md](pads.md) -
  and **no grant call site has been found**, so which weapon a crossing hands
  over, and how the four weights become one, are both unread. This project draws
  its own; see [pickups](../gameplay/pickups.md).
- **How a weapon is fired**, and what `craft+0x1c0`'s bits mean. One bit is
  identified: `0x0004` gates the `1.2` engine multiplier that a Turbo pickup
  arms.
- **`Ship_Damage`'s `weapon_kind`**, whose nine telemetry buckets almost
  certainly index this table; the mapping is still unconfirmed.

## A note that belongs to the AI as much as to the weapons

`Ai_Construct` (`0x088536bc`) names its input source `"AI input %d"` for an
opponent and the literal **`"autopilot input"`** for the local player - see
[grid.md](../ghidra/functions/psp-pulse-usa/grid.md). So the player's craft is
driven through the same controller object the AI is, and the **autopilot pickup
is that controller taking over**. Whoever builds either should know they are
building most of the other.
