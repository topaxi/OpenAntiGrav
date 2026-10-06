# `WeaponStats_*.xml`: the weapons are authored data

**Status: the schema is read end to end, confidence 92.** Every weapon's
tunables, the disturber effects and the pickup distribution are **authored XML on
the disc**, not a table compiled into the executable.

**Partly implemented since 2026-08-11.** `oag_tables::weapons` decodes the three
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

**`just wad` now hands the entry name through unmangled.** It used to pass
`{{ARGS}}` through the shell unquoted, which swallowed the backslashes before
`oag-wad` ever saw them - `just wad cat --expand <image>:...Data.wad
'Data\XML\WeaponStats_Race.xml'` failed with `no entry named
DataXMLWeaponStats_Race.xml`, not a missing-file error, which read like the
wrong path rather than a quoting trap. `justfile` now sets
`positional-arguments` and this recipe references `"$@"` instead, so the
recipe above works as written; calling `cargo run -q -p oag-tools --bin
oag-wad -- cat --expand <image>:PSP_GAME/USRDIR/Data.wad
'Data\XML\WeaponStats_Race.xml'` directly still works too, unchanged.

## Two files, one schema

| Entry | What it is |
| --- | --- |
| `Data\XML\WeaponStats_Race.xml` | the ordinary race modes |
| `Data\XML\WeaponStats_Elimination.xml` | Eliminator |

Both are read by the same parser - `WeaponStats_Parse` (`0x0880db7c`) - so a mode
swaps the whole weapon table rather than patching it. The two path strings sit at `0x08a78cf8` and `0x08a78d18`.

**The swap is not only in the numbers - it zeroes some weapons out of one table
entirely.** Two weapons author `<Pickupodds>` of nothing but zero in every
speed class of `WeaponStats_Race.xml`, and three others author nothing but
zero in `WeaponStats_Elimination.xml` instead - so a race mode and Eliminator
each have weapons the other's pad can never hand out at all, per-mode gating
rather than a per-mode reweighting. See
[pickups.md](../gameplay/pickups.md#shuriken-and-repulser-are-gated-by-mode-not-by-the-pool)
for which weapons and the measured odds.

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
| **`Rocket`** | **`0x0880c058`** | **`absorb blastforce blastradius damage slowdown_time venomspeed flashspeed rapierspeed phantomspeed launchSpeed`**, plus `spread` |
| **`Missile`** | `0x0880c31c` | the rocket's, less `spread`, plus `lock_max_dist lock_min_dist` - **decoded**, offsets and all, on [missile.md](../ghidra/functions/psp-pulse-usa/missile.md) |
| **`Quake`** | `0x0880c60c` | **`absorb damage radius slowdown_time`** - offsets `+0x60`..`+0x6c` measured, see [engine.md](../ghidra/functions/psp-pulse-usa/engine.md) - **decoded** in full as `oag_tables::weapons::QuakeStats`, `radius` now the hit-latch's own proximity gate per [cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md) |
| **`Cannon`** | `0x0880c774` | **`absorb rounds rate damage_per_bullet slowdown_time`** - **decoded** in full, offsets read 2026-09-07: `rounds` `+0x70` **as an `int`**, `absorb` `+0x74`, `rate` `+0x78` **stored as `1.0 / value`**, `damage_per_bullet` `+0x7c`, `slowdown_time` `+0x80`. The reciprocal is the whole reason the weapon fired once every twenty seconds until then - see [cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md) |
| **`Turbo`** | **`0x0880c92c`** | **`absorb time`** |
| **`Shield`** | **`0x0880ca2c`** | **`absorb time`** |
| **`Autopilot`** | **`0x0880cb2c`** | **`absorb time`** |
| **`Plasma`** | **`0x0880cc2c`** | the rocket's, less `spread`, plus `charge_time` - **decoded**, offsets and all, on [plasma.md](../ghidra/functions/psp-pulse-usa/plasma.md) |
| `Bomb` | `0x0880cef0` | `absorb blastforce blastradius damage damageradius slowdown_time trigger_radius timetodie` |
| `Mine` | `0x0880d124` | the bomb's, less `damageradius` |
| **`LeachBeam`** | `0x0880d328` | `repair absorb damage lock_max_dist lock_min_dist slowShipFactor range active_time energy_multiplier` - **all nine decoded** as `oag_tables::weapons::LeachBeamStats` (2026-09-08), and all nine spent as of 2026-09-16: the transfer, the lifetime, the range and, last, `slowShipFactor` - the one-shot thrust scale at `craft+0x31c`, see [cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md). The window is the second pair `Ship_AcquireLock` reads (`stats+0x114`/`+0x118`, against the Missile's `+0x50`/`+0x54`) and its consumer is the reticle; see [lock-sight.md](../ghidra/functions/psp-pulse-usa/lock-sight.md). **This block authors no `slowdown_time`** - the only one of the seven that does not |
| **`Repulser`** | **`0x0880d58c`** | `damage` `+0x128`, `blastRadius` `+0x12c` (spent nowhere), `blastForce` `+0x130`, `slowdown_time` `+0x134`, `absorb` `+0x138`, `blast_time` `+0x13c`, `wave_time` `+0x140` - read off the parser 2026-10-04, see [repulser.md](../ghidra/functions/psp-pulse-usa/repulser.md) |
| **`Shuriken`** | **`0x0880d790`** | `absorb rhicochetForce blastForce blastradius rhicochetdamage blastdamage slowdown_time <class>speed launchSpeed fuse` - **decoded** but for the ricochet pair and `slowdown_time`, offsets and all, on [shuriken.md](../ghidra/functions/psp-pulse-usa/shuriken.md) |

**Bold rows are decoded by `oag_tables::weapons`**; the rest are named here and
read no further, because [nothing consumes them](../gameplay/pickups.md) and a
field decoded with no consumer is a field nobody has checked. The Rocket is
partly bold for the same reason: `oag_weapons::projectile` reads nine of its
eleven attributes, and the one left plain is `spread`.

**`slowdown_time` moved out of that list on 2026-09-06** and is decoded on
every decoded block that authors it. **The LeachBeam is the one that does
not**, on all four shipped tables - Pulse's race and Eliminator files, USA and
EU - which is why `LeachBeamStats` has no such field rather than a defaulted
zero. Measured 2026-09-07. It stayed out for
a year because the slowdown mechanic had
no consumer; the mechanic's law is now recovered end to end -
`Ship_AddSlowdown` (`0x08848690`) adds a hit's `slowdown_time` to a timer and
clamps the total to `<Global slowdown_limit>`, so the global is a **ceiling on
seconds of slowdown outstanding**, not a speed floor. See
[engine.md](../ghidra/functions/psp-pulse-usa/engine.md). The Quake's `+0x60`
and `+0x68` are measured there too, from that mechanic's own consumer, so the
Quake's four are no longer unchecked.

**The Shuriken's block is measured at `+0x144`..`+0x174`**, thirteen slots, and
the Repulser's seven are confirmed at `+0x128` from a consumer rather than a
parser - so the run `mine.md` derived from attribute counts alone now has six
measured anchors (the Quake's four at `+0x60` were read off
`WeaponStats_ParseQuake` itself on 2026-09-06) and only the Cannon's five
(`+0x70`) are unchecked.

**The Plasma's block is measured at `+0x9c`..`+0xc4`**, which is exactly where
[mine.md](../ghidra/functions/psp-pulse-usa/mine.md)'s attribute-count
arithmetic put it - the third measured anchor in the run between the Missile's
block and the Mine's, after Turbo's `+0x84` and Shield's `+0x8c`. Ten of its
eleven attributes are decoded; `charge_time` is the one left plain, and it is
left plain for an uncomfortable reason rather than a routine one - see
[plasma.md](../ghidra/functions/psp-pulse-usa/plasma.md#charge_time-is-authored-and-nothing-read-spends-it).

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

**On Pulse and Wipeout HD. Wipeout Pure authors a single `speed`.** Measured
2026-08-26 off Pure's own table: its Missile reads
`speed="950" lock_min_dist="10" lock_max_dist="800"` and its Rocket
`speed="1000" spread="0.05"`, with no per-class attribute anywhere in the file
and **no `launchSpeed` on either**. So a Pure craft flies every class's weapons
at the same speed, which is a property of the disc rather than of this reading -
there is no per-class figure to have lost.

`oag_tables::weapons` accepts both: the per-class spelling wins where present,
one `speed` folds into all four, and `launchSpeed` defaults to zero when the
file does not carry it. A file with neither spelling is still an error.

### And Pure ships one table, not two

Pulse and HD ship `WeaponStats_Race.xml` and `WeaponStats_Elimination.xml` and
pick between them by race mode - the global `DAT_08b32428` selects the *file*,
recovered on [missile.md](../ghidra/functions/psp-pulse-usa/missile.md). Pure
ships a single, lower-cased **`Data\XML\weaponstats.xml`**, named at
`0x08a445a0` in `/pure/BOOT-psp-pure-usa.BIN` three strings before the `"WeaponStats"`
and `"Weapon"` element names its parser matches.

Which file a title reads is `oag_title::weapons::Weapons`. Until it was an axis,
every caller reached for Pulse's spelling and a Pure race parsed no weapons at
all - no pickups, no missile, no lock - with one report line to say so.

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
rather than inferred: `crates/tables/tests/weapons_ground_truth.rs` asserts both
halves. This page said the opposite for the length of one draft, having read the
parser and not the file - the ground-truth test is what caught it, which is what
it is for.

That is evidence toward [handling-stats.md](handling-stats.md)'s open question of
whether Pulse has a fifth speed class, and it points **away** from one: a class
the code names, discards, and no shipped file authors is a leftover rather than a
rung.

## The Pure dialect: ten weapons, one Disruptor, no fuse on the Bomb

**Disc-measured 2026-09-15**, off `Data\XML\weaponstats.xml` on both Pure
pressings (byte-identical, USA and EU) against Pulse USA's
`WeaponStats_Race.xml`. Attribute **names** and block structure only, per
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md); the parser
that reads each row is on
[`psp-pure-usa/weapons.md`](../ghidra/functions/psp-pure-usa/weapons.md),
which also carries the struct it fills.

| `type` | Pure `<Stats>` | Pulse `<Stats>` | Difference |
| --- | --- | --- | --- |
| `Global` | `slowdown_limit` | same | none |
| `Rocket` | `absorb blastforce blastradius damage slowdown_time speed spread` | the same less `speed`, plus `venomspeed flashspeed rapierspeed phantomspeed launchSpeed` | one speed, no launch speed |
| `Missile` | `absorb blastforce blastradius damage lock_max_dist lock_min_dist slowdown_time speed` | as the Rocket's split, plus the lock pair | one speed, no launch speed |
| `Quake` | `absorb damage radius slowdown_time` | same | none |
| **`Disruptor`** | `absorb speed`, plus fourteen `<Effect type=...>` children | **absent** | Pure only |
| `Turbo` / `Shield` / `Autopilot` | `absorb time` | same | none |
| `Plasma` | `absorb blastforce blastradius charge_time damage slowdown_time speed` | the same split as the Rocket | one speed, no launch speed |
| `Bomb` | `absorb blastforce blastradius damage damageradius slowdown_time trigger_radius` | the same plus **`timetodie`** | **no fuse** |
| `Mine` | `absorb blastforce blastradius damage slowdown_time timetodie trigger_radius` | same | none |
| `Cannon`, `LeachBeam`, `Repulser`, `Shuriken` | **absent** | present | Pulse only |
| `<DisturberOdds>` | **absent** | present, seven effects | Pulse only |
| `<Pickupodds class>` | **five** blocks: `Vector Venom Flash Rapier Phantom`, ten weapons each, `ai back front human` | four blocks, thirteen weapons each | Pure authors the fifth class and its parser stores it |

So ten weapons on Pure against thirteen on Pulse, and the two rosters share
nine: Pure's tenth is the Disruptor and Pulse's extra four are the Cannon,
LeachBeam, Repulser and Shuriken. That agrees with `oag_pure::hud`'s ten
icons and with the ten `absorb` values the file authors - the handover thread
that first noticed the gap counted nine, which was one short.

### The Disruptor's `<Effect>` blocks

```xml
<Weapon type="Disruptor">
  <Stats absorb speed/>
  <Effect type="Stall">              <EffStats time/>               </Effect>
  <Effect type="Fire Weapon"/>
  <Effect type="Mirror Left Right">  <EffStats time/>               </Effect>
  <Effect type="No Airbrakes">       <EffStats time/>               </Effect>
  <Effect type="Turbo Now"/>
  <Effect type="Autopilot Slow">     <EffStats speed_percent time/> </Effect>
  <Effect type="Autopilot Fast">     <EffStats speed_percent time/> </Effect>
  <Effect type="HUD Flicker">        <EffStats time/>               </Effect>
  <Effect type="Drunk">              <EffStats amount time/>        </Effect>
  <Effect type="Steal Weapon"/>
  <Effect type="Rubber Ship">        <EffStats amount time/>        </Effect>
  <Effect type="Adjust Gravity">     <EffStats amount/>             </Effect>
  <Effect type="Drunk Camera">       <EffStats amount time/>        </Effect>
  <Effect type="Trippy">             <EffStats time/>               </Effect>
</Weapon>
```

Fourteen authored, and the executable sorts them into three layers
(confidence 90 on the first, 84 on the other two; evidence on
[`psp-pure-usa/weapons.md`](../ghidra/functions/psp-pure-usa/weapons.md)):

1. **Four are dead at the parser.** `Fire Weapon`, `Turbo Now`, `Steal Weapon`
   and `Adjust Gravity` match no branch of `WeaponStats_ParseDisruptor` and
   exist as no string anywhere in either Pure executable. The three
   attribute-less ones look like an authoring convention for "no tunable";
   `Adjust Gravity` authors an `amount` nothing reads.
2. **Two are parsed and never rolled.** `HUD Flicker` and `Trippy` are read
   into the table and no function reads their slots back:
   `Disruptor_RollEffect` is `rand() % 8` over the other eight.
3. **Eight are live**: Stall, Mirror Left Right, No Airbrakes, Autopilot
   Slow, Autopilot Fast, Drunk, Drunk Camera, Rubber Ship - one chosen
   uniformly **when the pad hands the weapon over**, carried on the firing
   craft, and applied to whichever craft the bolt hits for that effect's own
   `time`. What each does to the victim is tabulated on the evidence page.

`oag_tables::weapons::DisruptorStats` decodes `absorb`, `speed` and the ten
parsed effects; the four dead blocks are deliberately not decoded, on the
"no consumer" rule, and the two unrolled ones are decoded because the
original's parser does and their absence would read as a parse gap.

**`speed` is the one authored speed the code does make per-class.**
`Disruptor_SpeedForClass` returns `speed + 80.0 * class_index`, 0..4 for
Vector..Phantom, so a Pure Disruptor flies faster up the ladder where every
other Pure weapon flies at its one authored figure. The bolt also leaves the
rail at a literal 500 km/h for its first tick, before the floor probe rescales
it.

### Pure's Bomb has no `timetodie`, and `damageradius` still has no reader

The Bomb's parser on Pure matches seven attributes and `timetodie` is not one
of them; the Mine's matches the same seven with `timetodie` in place of
`damageradius`. `BombPool_Update` ages a laid Bomb and spends that age on the
model's spin alone - nothing compares it to anything - so a Pure Bomb **sits
until a craft enters `trigger_radius`**, for the whole race if nothing does.
Confidence 86.

`damageradius` is parsed into the table and its slot has **no
cross-reference** on Pure, exactly as on Pulse - where the six slots either
side of it each resolve to their one consumer. Authored, stored, never read,
on both titles. It stays undecoded here for the same reason it always has.

`oag_tables::weapons::BombStats::timetodie` is therefore an `Option<f32>`:
`Some` on Pulse and HD, `None` on Pure, and a `None` fuse is a Bomb that
never times out rather than one that goes off at once.

## Wipeout HD and Fury: their own tables, and which copy a race reads

**An HD race plays HD's own numbers, not Pulse's.** `oag_hd::TITLE.weapons`
names `Data\XML\WeaponStats_Race.xml` and `..._Elimination.xml`; both resolve
(the PSARC reader folds case and separators) and decode with the Pulse roster
and no `skipped` entries. `crates/tables/tests/hd_weapons_ground_truth.rs`
pins it against the disc (read on 2026-10-06): every shipped copy decodes, and
an HD race's served bytes are `DATA00`'s copy.

| Entry | `DATA00` (Fury) | `DATA02` (base HD) | `DATA05` (front-end patch) |
| --- | --- | --- | --- |
| `weaponstats_race.xml` | 13 weapons, 4 classes, plus a `LightBarrier` block | same roster, no `LightBarrier` | same roster |
| `weaponstats_elimination.xml` | yes | yes (differs in Shield `time` and the LeachBeam) | no |
| `weaponstats_detonator.xml` | Mine, Cannon, Bomb, `EMP` | no | yes, retuned |

**The three race copies disagree on what a race spends.** The Rocket,
Missile, Plasma, Quake, Repulser, Shuriken and every `<Pickupodds>` block are
equal across them; the LeachBeam (`repair`, `damage`, `slowShipFactor`,
`energy_multiplier`) is not: all three differ on it, and `DATA05` also lacks
the Bomb's `number_of_shots_to_destroy`. A race
reads **`DATA00`'s**, because `oag_assets::Archives` serves the first archive
that has a name and `DATA00` is the data archive. **That is this project's
mount order, not a measured PS3 one** - the same open question as
`skin.xml`'s six copies in [hd-status.md](hd-status.md). Fury is the later
build and `DATA00` is the fuller copy, which makes it the likelier winner; it
is not a measurement.

### Per field

- **HD authors it and a race spends it** (through Pulse's code): every
  `<Stats>` block that decodes - Rocket, Missile, Quake, Plasma, Mine, Bomb,
  Cannon, LeachBeam, Repulser, Shuriken, Turbo/Shield/Autopilot `absorb` and
  `time`, `<Pickupodds>`, the `Global` `slowdown_limit`. The **numbers** are
  HD's, measured. The **behaviour** around them is Pulse's law, inherited and
  unmeasured on HD (HD's own constructors are read, not ported; see
  `docs/ghidra/functions/ps3-hdfury-eu/weapons.md`).
- **HD authors it and nothing reads it:** the Cannon's `recharge_time`
  (race, elimination) and `round_recharge_time`/`recharge_pause`/`num_rebounds`
  (detonator); the Bomb's `number_of_shots_to_destroy`; the `LightBarrier`
  block (`Weapon::from_type` drops an unknown `type` without a `skipped`
  entry, so no loader line says so); the whole `weaponstats_detonator.xml`
  except as a table that parses; the `EMP` block; `energy_recharge_per_stage`
  and the Detonator Mine's `points_per_metre`/`max_distance`/
  `velocity_reduction`. These are HD-only mechanics with no Pulse law to
  inherit; wiring them is open work, not a gap in the reader.
- **HD authors no value:** a speed per class for the Cannon, as on Pulse;
  everything Pulse's measured law supplies stands.
- **Per mode:** the race table zeroes the Repulser and Shuriken out of all four
  classes and the Eliminator table does not - the same per-mode gating
  Pulse has, held by the ground-truth test.

### Omega checks against HD: checked, applies, not wired

`omega-ps4-eu`'s `data00.psarc` ships `weaponstats_race/_elimination/
_detonator.xml` under HD's names, plus `weaponstats_Race_2048.xml`,
`weaponstats_Elimination_2048.xml` and `WeaponAIStats2048.xml`. The reader
decodes all of them. Omega's three HD-named files are **not byte-identical**
to Fury's - each adds a fifth `<Pickupodds class="SuperPhantom">` block - and
every decoded weapon block (Rocket through Shuriken, `slowdown_limit`) and the
first four classes are equal to `DATA00`'s. The fifth class is unread: the
handling reader's class list stops at four. Omega racing is out of scope, so
nothing is wired. The two `_2048` files decode with all 13 weapons.

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
