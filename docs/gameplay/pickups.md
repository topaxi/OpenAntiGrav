# Pickups

What a `Weapon Pad` hands out, what a craft does with it, and - separately -
the free Turbo a solo event is given once a lap.

Implemented in [`oag_gameplay::pickup`](../../crates/gameplay/src/pickup.rs)
(the draw and the inventory), `oag_game::race` (the trigger, the grant and
spending it) and `oag_physics::engine` (what a fired Turbo does). The pad
geometry is [pads.md](../formats/pads.md); the table is
[weapon-stats.md](../formats/weapon-stats.md); the runtime side of the pad is
[the Ghidra page](../ghidra/functions/psp-pulse-usa/pads.md).

## Read this first: what is recovered and what is ours

This subsystem is unusually mixed, so the split comes before anything else.

| Piece | Status | Confidence |
| --- | --- | --- |
| The pad trigger: swept test, distance cache, stamping `<WeaponPad refresh_time>` | **recovered** | 90 |
| `<Pickupodds>`: weights per class, per `ai`/`human`/`front`/`back` | **recovered** | 92 |
| `<WeaponPad refresh_time elimination_refresh_time>` | **recovered** | 90 |
| `SQUARE` fires, `CIRCLE` absorbs | **recovered** | 90 |
| The `1.2` thrust multiplier and where it lands in `Ship_UpdateEngine` | **recovered** | 85 |
| That the `1.2` pickup is the **Turbo** weapon | inferred | 80 |
| `Ship_SetShield`'s clamp, which absorb pays through | **recovered** | 84 |
| Weapons off hides the pads and empties the trigger list | **recovered** | 88 |
| The free Turbo once a lap in a time trial and a speed lap | **recovered**, from two shipped records | 85 |
| **That a crossing grants anything at all** | **ours** | - |
| **The weighted draw** | **ours** | - |
| **The inventory's shape** (one slot, `Option<Weapon>`) | **ours** | - |
| **Granting only into an empty slot** | **ours** | - |
| **When in the lap the free Turbo arrives** | **ours** | - |

The three "ours" rows in the middle are not a gap anyone can close by looking
harder in the obvious place. `WeaponPads_TestCraft` (`0x0888727c`) stamps the
pad's refresh timer on a hit and **that is the whole of what it does** - no
pickup-grant call site has been found anywhere, and the craft-side flag word
that would hold the result (`*(entity+0x4c) + 0x1b8`, and `craft+0x1c0`) has
one bit identified out of fourteen. So the machinery *around* the grant is
recovered in detail and the grant itself is this project's reading of what a
weapon pad is for.

**The PRNG bounds what can ever be checked.** The original's generator is an
open question on the [roadmap](../overview/roadmap.md), so the *sequence* of
draws cannot match the original even with a byte-exact algorithm. Only the
*distribution* is testable, and only against the authored weights. Every draw
goes through `World::rng` so that a replay of one of our own races is
reproducible - which is the guarantee that survives.

## Which modes hand out what

Three different things happen, and the disc distinguishes them in two
independent files.

| Mode | Weapon pads | Free Turbo |
| --- | --- | --- |
| Time trial | hidden, trigger list empty | **once per lap** |
| Speed lap | hidden, trigger list empty | **once per lap** |
| Zone | hidden, trigger list empty | no |
| Single race | armed | no |

The first column is `Race_ReadSetupOptions` (`0x08896b84`) and
`World_CollectNodeLists` (`0x088879d4`): a weapons-off race does not ignore a
crossing, it clears each pad node's visibility bits and zeroes the trigger
list's own count. `oag_race::Mode::weapons_enabled` is the port, and
`oag_game::race::Race::start` drops the volumes rather than testing the mode per
tick, which is the same layer the original decides it at.

### The free Turbo, and how the second record was found

`MSC_EVENT_TT` and `MSC_EVENT_SL` both read *"You will be given a free turbo
pickup once per lap"*. That is a shipped string and it was known; what was
missing was any second source, and a mode whose pads are hidden getting a
pickup from nowhere is exactly the kind of claim a manual makes loosely.

**The HUD layouts settle it.** Each of the four authors a different pickup set:

| Layout | Pickup widgets |
| --- | --- |
| `Arcade_HUD.xml` | the backdrop and all thirteen icons |
| `Elimination_HUD.xml` | the same |
| `TimeTrial_HUD.xml` | the backdrop and **`TurboIcon` alone** |
| `Zone_HUD.xml` | none |

A layout carrying exactly one weapon icon, for a mode with no armed pads, is
otherwise inexplicable - and it is the same weapon the event text names. A
string table and a HUD layout agreeing is two independent records of one rule,
the same shape of evidence that settled weapons-off energy recovery in
`oag_gameplay::damage_rules`. Confidence **85**.

This was not found by looking for it. `every_weapon_has_an_icon_widget_named_after_it`
was written asserting that the weapons-off layouts author *no* pickup widgets,
and failed. **The assumption it was written to confirm came from a grep whose
key was wrong** - these files are [shortened XML](../formats/fexml.md) with a
per-file dictionary, so `name` is `b=` in `Arcade_HUD.xml` and `c=` in
`TimeTrial_HUD.xml`, and grepping the first file's key against the second
returns nothing with exit code 0. Expand before grepping, or use the parser.

**What is not recovered is where in the lap it arrives.** It is granted on the
lap edge here, so the first one comes with the start of lap 2 rather than at the
start line. The original may hand it over at the start of a lap instead, which
is the same edge one lap earlier; nothing read says which. The choice made is
the one that cannot give a free boost before the clock starts.

## The icon is found by name, not by an id

[hud.md](../ui/hud.md) recorded the pickup widgets as "14 `*Icon` widgets" whose
"icon ids are numeric", with no id-to-weapon mapping known - which made drawing
the right icon look like it needed something unrecovered.

It does not. `Arcade_HUD.xml` authors **thirteen**, and names each after its
weapon's own `type` string: `TurboIcon`, `ShieldIcon`, `RocketIcon`,
`LeachBeamIcon` and `RepulserIcon` misspellings included. That is exactly
`oag_formats::weapons::Weapon::ALL`, so the lookup is
`format!("{}Icon", weapon.as_type())` and nothing else -
`oag_game::hud::pickup_icon_name`. Pinned against the shipped file for all
thirteen by `every_weapon_has_an_icon_widget_named_after_it`.

The numeric ids are still real - `0x0883b3b8` forces one, and it is `6` - they
are simply not needed to draw the right icon. That constant is, separately, a
reason to read the `1.2` pickup as the Turbo: `6` lands on `Turbo` if the ids
index the class-name pool from one.

## Only what has an effect is handed out

`oag_gameplay::pickup::IMPLEMENTED` is the pool a pad draws from, and it holds
**Turbo alone** today.

- Ten of the thirteen need a projectile, a target or both.
- **Shield**'s `time` joins to no recovered code path. `shield.md` is about the
  energy *pool*, which is a different thing that the filename invites confusing
  with it.
- **Autopilot** is the AI's own controller taking over: `Ai_Construct`
  (`0x088536bc`) names the local player's input source the literal
  `"autopilot input"`. It is AI work wearing a pickup's clothes, not pickup
  work.

**This is a departure and a deliberate one.** The authored table weights
thirteen weapons and the draw sees one of them, so what a player gets is the
authored distribution *conditioned on* the implemented set. It narrows to
nothing as weapons land - adding a variant to `IMPLEMENTED` is the whole change
- and it beats handing out a rocket that cannot be fired.

## What a fired Turbo does

`if (flags & 0x0004) T *= craft+0x2a0`, in `Ship_UpdateEngine` (`0x0884c5c8`),
with `craft+0x2a0` written as `1.2` by `0x0883b3b8` on the branch that also sets
the gating flag and forces the HUD icon. The multiply sits **after** the
`0.5 * speed + accelcap` clamp and before the fixed doubling, so a turbo pushes
the craft past the cap rather than being clipped by it. Confidence 85 on the
arithmetic, 80 on it being the Turbo weapon.

`oag_physics::engine::ENGINE_PICKUP_SPEEDUP` is the constant and
`ShipState::turbo_timer` is the gate. **The timer is ours** - the original keeps
a bit in `craft+0x1c0` armed from an equally undecoded pickup word - but its
duration is the file's own `<Weapon type="Turbo"><Stats time>`. The field is
hashed by the determinism gate, which is why it lives on `ShipState` rather than
beside the inventory.

**Not to be confused with `Engine.turbo`**, which is a flat *additive* thrust
under flag bits `0x200`/`0x400` and a mode enum. `0x400`'s only known writer is
the barrel roll, so that term belongs to a different mechanic and is still
unimplemented. Reading `<Engine turbo>` as the Turbo pickup's effect is the
obvious-looking mistake here.

## Absorbing

`CIRCLE`, and it pays the weapon's own `<Stats absorb>` into the energy pool
through `oag_physics::damage::add`, which clamps at the skill-indexed `<Misc>`
maximum. **The clamp is recovered**: `Ship_SetShield` (`0x0883e6f4`) takes
`min(amount, max)` and floors nothing, so a full pool gains nothing and the
`ShieldBar` cannot overfill.

Absorb works for every weapon, implemented or not, because `absorb` is decoded
for all thirteen - so a pickup this engine cannot fire is still worth
collecting. That is not a design choice; it is what happens when the file
authors one attribute for everything.

## The refresh timer is a debounce, not a respawn

`<WeaponPad refresh_time>` is `0.55` seconds for every speed class on both
shipped PSP discs, and `elimination_refresh_time` is `0.05`. A craft at racing
speed clears a pad about 9.6 units long in well under half a second, so what the
timer buys is that one crossing is one pickup - not that a collected pad goes
away for a while. Eliminator's being an order of magnitude shorter fits a mode
that hands weapons out far more freely.

Stamped on **any** hit, including one that grants nothing, because that is what
the original does: the stamp is unconditional and the grant is not.

## What is not built

- **Ten of the thirteen weapons.** No projectile, no target, no `source == 2`
  damage path.
- **The pad's ready-to-collect colour cycle.** `WeaponPad_UpdateRefreshTimer`
  (`0x0892c034`) packs a grey into `pad+0x6c` while cooling down and cross-fades
  a small colour table once it is collectable. Observed, not implemented, so a
  spent pad looks the same as a fresh one.
- **`SubWeapon`.** The layouts author it, which suggests a craft can hold two.
  Nothing read says so, and the inventory here is one slot.
- **The `front`/`back` odds columns**, which need race positions, which need
  opponents that move.
- **The per-pad refresh timers are not in the determinism hash.** They are
  genuine simulation state; the hash covers `ShipState` and the tick, and
  widening it is a change to the gate rather than to this feature. A replay of a
  single race would need them.

## See also

- [race modes](race-modes.md) - what a single race is, and what it does without
- [weapon stats](../formats/weapon-stats.md) - the table and its schema
- [pads](../formats/pads.md) - the geometry and the trigger volumes
- [the pad runtime](../ghidra/functions/psp-pulse-usa/pads.md) - the trigger
- [shield](../ghidra/functions/psp-pulse-usa/shield.md) - the pool absorb pays
- [the HUD](../ui/hud.md) - the widgets
