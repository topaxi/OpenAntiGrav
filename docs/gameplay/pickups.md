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
| `T += Engine.turbo`, uncapped, and where it lands in `Ship_UpdateEngine` | **recovered** | 84 |
| That flag bit `0x200` is the Turbo pickup's half of that gate | inferred from the pair with the barrel roll's `0x400` | 75 |
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
are simply not needed to draw the right icon.

### Drawing both widgets as authored gives an opaque white hexagon

Found by looking at it, after the first version of this shipped. **It is not a
bug in the reader; it is what the shipped data says**, and both halves are
measured:

- `PickupBackground` samples a **filled hexagon whose alpha is 255** on 2,424 of
  its 2,492 opaque pixels; `TurboIcon` samples a glyph in the same white with
  antialiased edges. Both are pure white masks, so all the colour is meant to
  come from the tint.
- Both are authored `Color="FEConst->HudColour1"`, and that constant is
  `0xFFFFFFFF` - **opaque white**.

So an opaque white hexagon is drawn and an opaque white glyph is drawn on top of
it, and the second is invisible against the first. The original does not look
like that, so **it must set at least one of the two colours at runtime**, and
that is unrecovered. `0x0883b3b8` is the known place the runtime reaches into
these widgets - it forces the icon id - and is where to look.

**What this build does instead is a substitution, not a recovery.** The backdrop
is drawn in `HudBGColour`, the only background colour the layout defines
(`0x40000000`, a quarter-alpha black), and the icon keeps its authored white.
Every value still comes off the player's own disc; what is ours is the choice of
which constant. The precedent is the front end's title colour, substituted the
same way while the widget behind it is unbuilt.

Two ground-truth assertions keep it honest, both against the shipped file: that
the backdrop and the icon really are authored in one colour - if that ever
stopped being true the substitution would no longer be needed - and that the
layout really defines `HudBGColour`, without which the substitution would
silently not happen and the icon would go back to being invisible.

**A reference frame of the original's own pickup box would settle it** and has
not been taken. It is the cheapest open thing on this page.

The forced id `6` was briefly read as evidence that the `1.2` pickup is the
Turbo. **It is not**: `6` lands on `Shield` or `Autopilot` depending on where
the class-name pool starts counting, and the turbo turned out to be a different
term entirely - see below.

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

**`T += Engine.turbo`, uncapped.** From `Ship_UpdateEngine` (`0x0884c5c8`), read
out of the decompiler on 2026-08-11:

```c
if ((flags & 0x200) || (flags & 0x400)) {
    if (craft+0x2a4 == 1) {
        T += Engine.turbo;                        // after the cap, before the * 2.0
        if (controls.buttons & 1)  lift = g_boost_lift * T;
    }
}
```

The add sits **after** the `0.5 * speed + accelcap` clamp and before the fixed
doubling, so it is not clipped by the acceleration cap - which is what makes a
turbo a turbo rather than a nudge. `ShipState::turbo_timer` is the gate and
`oag_physics::engine::engine` is the port.

**Two bits turn one term on, and that is the evidence for reading `0x200` as the
pickup.** `0x400` is held while the barrel roll's timer runs
([input-bindings](../ghidra/functions/psp-pulse-usa/input-bindings.md),
confidence 80), and `<Special>` authors `roll_turbotime` beside it - so a roll
grants a *turbo*, and the term has two sources. `0x200` is the other one.
**Its writer has not been found**, so this is an inference from the pair rather
than a traced path: confidence **75**.

Not reproduced, both stated where they live: the `craft+0x2a4 == 1` gate, whose
enum nothing decodes, and the **boost lift** along body up under the thrust
button, whose `g_boost_lift` global was never read.

### The `1.2` multiplier is a different pickup, and wiring it here was wrong

Worth keeping because it is an easy mistake and it shipped for a few hours.
`craft+0x2a0 = 1.2` under flag `0x0004` is real and recovered
(`oag_physics::engine::ENGINE_PICKUP_SPEEDUP`), and it is **not** the turbo:

- It multiplies a thrust that has *already been clamped*. On Venom the cap is
  `0.5 * speed + accelcap` with `accelcap` in the tens, so `1.2` buys a few
  units of force; `<Engine turbo>` is authored an order of magnitude above the
  same class's `accelcap`. Measured end to end on the disc's own tunables, a
  fired turbo leaves the craft at **222 units/s against a control's 52** after
  one second - the multiplier version was imperceptible, which is exactly how
  it was reported.
- The branch that arms it lives in the **HUD update** (`0x0883b3b8`), not the
  craft update, and it also forces HUD icon id `6` and drives a fill from the
  pickup's own `+0x148` timer. So `0x800` is some timed effect the HUD draws a
  bar for; **which pickup it is remains unidentified**, since id `6` lands on
  `Shield` or `Autopilot` depending on where the class-name pool starts
  counting, and neither is obviously a speed effect.

**The lesson for the tests**, written down because the unit tests all passed:
asserting the *ratio* between a boosted and an unboosted tick cannot tell a
turbo from a nudge, because a ratio is a ratio. `a_weapon_pad_on_the_disc_hands_out_a_pickup_in_a_single_race`
now compares speed after a second against a control on the shipped numbers,
which is a magnitude and would have caught it.

**The timer itself is ours.** The original holds a bit; this holds seconds,
because neither `craft+0x1c0`'s writer nor the pickup word behind it has been
read. Its duration is the file's own `<Weapon type="Turbo"><Stats time>` and its
magnitude the class's own `<Engine turbo>`, so both numbers are real. The field
is hashed by the determinism gate, which is why it lives on `ShipState` rather
than beside the inventory.

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
## The determinism gate does not see any of this, and it should

**The simulation is still deterministic.** Nothing here reads a wall clock, OS
entropy or a `HashMap` iteration order; every draw goes through the seeded
`World::rng`, so the same seed and the same inputs give the same race. That part
of [the rules](../architecture/determinism.md) holds.

**What does not hold is that the gate can see it.** The committed hash covers
`ShipState` and the tick
(`oag_physics::probe::hash_state`, `crates/physics/tests/determinism.rs`), and
three pieces of simulation state this feature added sit outside it:

- the per-pad refresh timers, on `Race`;
- the inventory, on `oag_gameplay::Ship`;
- the generator's own position, which nothing has ever hashed.

`ShipState::turbo_timer` *is* hashed, so what a fired Turbo does to the force law
is covered - but what decides whether it is ever fired is not.

**The consequence is sharper than "an uncovered field", and it is why this is
written down rather than left implicit.** A pad's refresh timer decides whether
`pickup::draw` is called, and `draw` consumes `World::rng`. So a change that
moved a refresh timer by one tick would shift the whole generator stream from
there on, changing every later draw in the race - and the gate would report
green, because none of the three fields above reaches the hash. That is exactly
the class of silent divergence the gate exists to catch.

Two things follow for whoever picks this up. A **snapshot-based replay** cannot
restore a single race today, because the pads' state is not in the snapshot the
determinism rules are shaped around. And **widening the hash is the fix**, not
moving the fields: they are on `Race` and `Ship` for good reasons -
`oag-physics` depends on nothing but `oag-core` and cannot name a `Weapon`, and
pad timers belong to the track rather than to a craft. What is needed is a
race-level hash beside the ship-level one. None exists yet; the only thing
shaped like one lives inside
`cycling_the_camera_changes_no_simulation_state`.

## See also

- [race modes](race-modes.md) - what a single race is, and what it does without
- [weapon stats](../formats/weapon-stats.md) - the table and its schema
- [pads](../formats/pads.md) - the geometry and the trigger volumes
- [the pad runtime](../ghidra/functions/psp-pulse-usa/pads.md) - the trigger
- [shield](../ghidra/functions/psp-pulse-usa/shield.md) - the pool absorb pays
- [the HUD](../ui/hud.md) - the widgets
