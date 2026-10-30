# WebAssembly extensions: where the engine meets a mod

**Status: proposal (2026-10-29), nothing is built.** The overview, the capability
surface and the interface sketch (the `sim` and `hud` worlds, their events and
commands) are in [README.md](README.md); the determinism rules, the state model and
the spike's measurements in [determinism-and-state.md](determinism-and-state.md).
This page is the engine half: **where in today's code each event is raised, where
each command lands, and where the one guest call per tick sits**, with what the
host refuses at each.

Every `file.rs:line` below is pinned to commit `b4b4cb3aa` (main, 2026-10-29) and
named with its function, so a citation survives drift: search for the function if
the line moved.

## The shape: one call per tick, events in, commands out

A simulation module exports `race-start`, `tick` and `race-end` (README, "The sim
world"). The engine does not call into a guest at every interesting point; it
**records events where they happen** and **calls each module once per tick**, at
one fixed phase, with an observation that carries those events. The guest answers
with a capped list of commands, which the host validates and **applies at the
point in the tick where each command's target is read**.

Why one call and not one per hook: a call costs a fresh instance (4.2 us pooled on
Cranelift, 5.2 us on Pulley, measured; see
[determinism-and-state.md](determinism-and-state.md#mod-state-is-a-fixed-size-blob-in-the-race-state)),
the observation is built once, ordering between modules is one rule (modules in
id order, each sees the same observation and never another's commands), and a
client's rollback replays one call per module per tick.

- **Data in is a copy.** The observation is built from engine state the renderer
  already reads; the guest never holds a pointer into `World` or `RaceSim`.
- **Data out is a bounded command.** The host checks every field (finite, in range,
  a slot the module may touch, a command the mod's kind may send) before anything
  reaches the simulation. A NaN or out-of-range value turns the module off; it is
  never clamped silently.
- **State is host-owned.** What a module keeps between ticks is its declared
  fixed-size state blob, in the race state, snapshotted and hashed with it
  ([determinism-and-state.md](determinism-and-state.md#mod-state-is-a-fixed-size-blob-in-the-race-state)).
- **Failure has one answer.** A trap (out of fuel, `unreachable`, out-of-bounds
  memory, stack overflow) or a refused command turns the module off for the rest of
  the race; the tick it happened on is state, so every machine running the same
  module switches it off on the same tick (the spike measured the trap point
  identical on Cranelift and Pulley). Its commands from that call are discarded and
  its state blob keeps its value from before the call.

## Where the host sits

The race tick is `Race::tick` / `Race::tick_body`
(`crates/raceplay/src/tick.rs:48`, `:58`) in `oag-raceplay`'s simulation half (the
half `oag-server` links with `view` off). The host is called from there.
`oag-physics`, `oag-race`, `oag-weapons` and `oag-ai` never learn that a module
exists: commands reach them as inputs they already take, the way Zone's auto-speed
does (`oag_physics::forces::Environment::auto_speed`,
`crates/physics/src/forces.rs:260`). Crates and dependency rules:
[determinism-and-state.md](determinism-and-state.md#dependency-rules).

## The tick, with the call, the events and the commands in it

`Race::tick_body` (`crates/raceplay/src/tick.rs:58`), in order.

| # | Phase | Today's call site | Events raised | Commands applied |
| --- | --- | --- | --- | --- |
| 0 | Grid flags, hover caps | `tick.rs:118` to `:131` | | |
| 1 | Every person's controls | `Race::player_controls`, `tick.rs:137` to `:142` | | |
| 2 | Pickups spent, autopilot | `Race::spend_pickup`, `tick.rs:166` to `:173` (`crates/raceplay/src/weapons.rs:148`) | `weapon-fired` (person slots) | |
| 3 | Mines, cannons | `tick.rs:180`, `:186` | | |
| **C** | **The call** (to be added) | between `tick.rs:186` and `:193` | | commands validated and queued |
| 4 | Each person's craft | `Race::step_player`, `tick.rs:196` (`crates/raceplay/src/player_step.rs:136`) | `pad-crossed`, `volume-entered`/`-left`, `wall-contact` | `scale-thrust`, `impulse`, `hold-mag-lock`, `shield-delta`, `give-pickup`/`clear-pickup`, `give-shield`/`hold-shield`/`clear-shield`, `assist` |
| 5 | Projectiles fly, hit | `oag_weapons::projectile::step`, `tick.rs:235`; impacts `tick.rs:267` | `hit` (the Quake, Leach Beam and Repulser steps raise it later in the tick; every source runs after the call) | |
| 6 | Rescue, AI craft, pairs | `tick.rs:415` to `:422`; `Race::step_opponents` (`crates/raceplay/src/field.rs:689`) | `respawned`, `reset`; phase 4's events and `weapon-fired` for AI craft | phase 4's commands for AI craft |
| 7 | Clock | `self.sim.world.tick += 1`, `tick.rs:446` | | |
| 8 | Race rules per person | `Race::advance_player_rules`, `tick.rs:493` (`player_step.rs:381`); `eliminate_if_wrecked` at `tick.rs:476` | `lap-completed`, `eliminated` | `set-finished` |
| 9 | Standings | `Race::update_standings`, `tick.rs:501` (`field.rs:1019`) | `lap-completed` (AI craft) | `set-finished` (AI craft), `set-standing-key` |
| 10 | Quake, leach beam, repulsers | `tick.rs:510` to `:530` | | |
| 11 | Results | `Race::capture_results`, `tick.rs:541` | | |

**Why the call sits at C.** After phase 2, so this tick's `weapon-fired` for a
person is in the observation and a mod weapon fired this tick acts this tick;
before phase 4, so a command lands in this tick's force law (`scale-thrust` is
consumed by the step at `player_step.rs:231`,
`std::mem::replace(&mut ship.pending_thrust_scale, 1.0)`) and in this tick's race
rules.

**The one-tick latency, stated rather than hidden.** Events raised after C (pads,
volumes, walls, hits, laps, respawns, eliminations, AI fire) are delivered in the
**next** tick's observation. So a craft that enters a speed pad on tick N has its
mod's `scale-thrust` applied from tick N+1. That is a property of this ABI,
identical on every machine; a law that needs same-tick reaction (an original's,
measured) belongs in the engine, not in a mod.

## Event sources

Each event is recorded into a fixed-size per-tick event buffer in the race state
(cleared at C after the observation is built), so it is part of a snapshot and a
rollback replays it. Names are the README's `event` variants.

### `pad-crossed`

- **Raised at** the speed pad's entry edge in `Race::test_speedup_pads`
  (`crates/raceplay/src/pads.rs:73`), at `if entered != self.sim.pad_current[slot]`
  (`pads.rs:113`), the edge the Zone score and the exhaust flare already hang off;
  and at the weapon pad's edge in `Race::test_weapon_pads` (`pads.rs:216`), at
  `pads.rs:258`. Both run for a person's craft from `step_player`
  (`player_step.rs:210`, `:215`) and for an AI craft from `step_opponents`
  (`field.rs:824`, `:825`), so one site covers every craft.
- **Carries** slot, kind (speed or weapon), pad index.
- **Entry only**, not every tick the pad re-arms its boost (the README's open
  question on event granularity; the entry edge is what exists today).

### `volume-entered` / `volume-left` (engine change)

- **Today** a track carries two kinds of trigger box: speed and weapon pads (the
  `oag/pads/speedup` and `oag/pads/weapon` nodes of
  [the track file](../../formats/track-file.md), each a matrix plus
  `_OAG_MIN`/`_OAG_MAX`), tested by the shared sweep (`Race::pad_sweep`,
  `pads.rs:45`) with `contains` and `distance`; and the reset zones
  `Race::reset_zone_contact` reads (`crates/raceplay/src/respawn.rs:235`). There is
  no generic named volume.
- **Proposed:** a third node kind with the same box shape, tested in the same
  sweep after the weapon pads, with an entry and an exit edge per craft and the
  volume's index in the event. A generalisation of the pad test, not a second
  trigger system.

### `wall-contact`

- **Raised at** the wall contact a craft's step reports (`evaluated.wall.impact`),
  where `Race::count_wall_contact` (`field.rs:545`, called from `step_player` and
  `step_opponents`) already counts it and `raise_contact_cue`
  (`crates/raceplay/src/cues.rs:41`, called at `tick.rs:673`) raises the sound.
- **Per contact edge**, once per craft per tick at most.
- **Built (Wall Penalty, 2026-10-29) as every tick of contact**, not the edge: the
  event is raised on each tick `contacts > floor_contacts` (the same test that
  counts `wall_contact_ticks`), so a craft that stays on the wall keeps reporting
  it and a curse can be renewed while it scrapes. `evaluated.wall.impact` alone
  would end a long scrape's curse 1.5 s after its first tick.

### `hit`

- **Raised at** every weapon hit on a craft, one that a running Shield absorbed included:
  `WeaponHit` (`crates/weapons/src/projectile/hit.rs`) carries the `asked` amount and the
  weapon out of each `oag_physics::damage::apply_weapon` call site;
  `Extensions::weapon_hits` folds them in at `tick.rs` (`projectile::step`),
  `weapons/single_instance.rs` (Quake, Leach Beam) and `weapons/repulser.rs`.
- **One event per craft per tick**: `amount` summed, `weapon` the first. Delivered at the
  next call (N+1), which runs before step N+1; every hit source runs after the call.
- **A Quake wave or a Leach Beam link meeting a running Shield raises none** (they gate
  on the Shield before `Ship_Damage`). `shield-delta` raises none.
- **Companions:** the `shielded` craft flag, and `give-shield`, `hold-shield` and
  `clear-shield` ([README](README.md#breakable-armour-from-the-shield-2026-10-30)).

### `weapon-fired`

- **Raised at** a person's craft: `Race::spend_pickup` (`weapons.rs:148`), called at
  `tick.rs:168`, edge-triggered on fire. An AI craft:
  `Race::spend_opponent_pickup` (`crates/raceplay/src/field/opponent_weapons.rs:329`),
  called from `step_opponents` at `field.rs:826`, where the driver's decision
  (`oag_ai::Driver::wants_to_fire`, `crates/ai/src/driver/weapons.rs:72`) is acted
  on. `spend_pickup_under_autopilot` (`field.rs:357`) is the Autopilot pickup's
  path, not a fire decision.
- **AI craft fire after C**, so their `weapon-fired` arrives a tick later than a
  person's. **Open:** our driver's decision knows the titles' weapons only, so an AI
  craft holding a mod weapon would never fire it; either a mod weapon names which
  original weapon's AI use it borrows, or only people fire it.

### `weapon-hit`

- **Raised at** the impacts `oag_weapons::projectile::step` returns
  (`crates/weapons/src/projectile.rs:536`, called at `tick.rs:235`), walked at
  `tick.rs:267`, where `Race::credit_blast` (`crates/raceplay/src/eliminator.rs:498`)
  already credits kills and `last_damager` is set.
- **Carries** victim, source, weapon. The damage itself is the weapon's law and is
  already applied when the event is raised.

### `lap-completed`

Two sources, because a person and an AI craft count laps through two structs:

- **A person's slot:** `Race::advance_player_rules` (`player_step.rs:381`), run only
  for the slots stepped as people (`tick.rs:452` to `:494`), calls
  `RaceState::update` (`crates/race/src/state.rs:410`); its `Outcome`
  (`state.rs:590`) has `lap_completed` and `lap_ticks`.
- **Every craft, AI included:** `Race::update_standings` (`field.rs:1019`) runs
  `advance_standings` (`field.rs:1258`), which calls `Standing::update`
  (`crates/race/src/standing.rs:175`) per active craft and returns whether a lap
  completed. An AI craft's `RaceState` is deliberately not advanced (the comment at
  `field.rs:1274` to `:1278`), so for AI slots this is the source.

### `respawned`, `reset`, `eliminated`

- `respawned`: `Race::respawn` (`respawn.rs:280`); `reset`: `Race::reset`
  (`respawn.rs:322`); both from the rescue at phase 6 and the reset zones.
- `eliminated`: `Race::eliminate_if_wrecked` (`player_step.rs:366`, called at
  `tick.rs:476`) for people and `Race::tick_destroyed_craft`
  (`crates/raceplay/src/eliminator.rs:129`, `tick.rs:439`) for the field.

## Command sinks

What each README command writes, where, and what the host refuses there. The
clamps are the README's.

The commands built so far, the record the host decodes (`oag_ext_abi::Command`)
and the guest SDK's method for each (`oag_ext::Reply`; [README](README.md), "And
the SDK"). A method taking a `&Craft` can only name a slot that holds one; each
returns `Err(Refused)` where the host would fault:

| Command | `oag_ext_abi::Command` | `oag_ext::Reply` | The SDK refuses |
| --- | --- | --- | --- |
| `scale-thrust{slot, factor}` | `ScaleThrust` | `scale_thrust(&craft, factor)` | a factor outside `[0, 2]` or not finite; a second for the craft this tick |
| `shield-delta{slot, amount}` | `ShieldDelta` | `shield_delta(&craft, amount)` | not finite or past `craft.shield_max()`; a second for the craft this tick |
| `give-pickup{slot}` | `GivePickup` | `give_pickup(&craft)` | a craft holding anything |
| `clear-pickup{slot}` | `ClearPickup` | `clear_pickup(&craft)` | a craft not holding `Pickup::Ours` |
| `pickup-roll{weapon, every}` | `PickupRoll` | `pickup_roll(weapon, every)` | `every` 0 |
| `scale-pad{pad, factor}` | `ScalePad` | `scale_pad(pad, factor)` | a factor outside `[0, 2]` or not finite; a pad the track does not have; a second for the pad this tick |
| `scale-pad{slot, factor}` | `ScaleSlotPads` | `scale_slot_pads(&craft, factor)` | a factor outside `[0, 2]` or not finite; a second for the craft this tick |
| `play-cue{slot, cue}` | `PlayCue` | `play_cue(&craft, cue)` | |
| `fire-effect{slot, effect}` | `FireEffect` | `fire_effect(&craft, effect)` | a ninth `fire-effect` or `fire-mod-effect` in one tick (`Refused::Effects`; the host faults the module) |
| `hud-tint{slot, element, rgba}` | `HudTint` | `hud_tint(&craft, element, rgba)` | the `Pickup` element (`Refused::PickupTint`; use `pickup_look`); the host decodes one as malformed |
| `pickup-look{slot, icon, colour}` (22) | `PickupLook` | `pickup_look(&craft, icon, colour)` | |
| `hud-shake{slot, element, amplitude}` | `HudShake` | `hud_shake(&craft, element, amplitude)` | `amplitude` outside `[0, 16]` or not finite |
| `hud-bar-extra{slot, element, extra, rgba}` | `HudBarExtra` | `hud_bar_extra(&craft, element, extra, rgba)` | `extra` outside `(0, 4]` or not finite |
| `hud-text{slot, anchor, x, y, text, rgba, size}` | `HudText` | `hud_text(&craft, place, rgba, text)` | text over 48 bytes, an offset past 2047, a size outside 1..=15 quarters; a 33rd item for the craft (`Refused::HudItems`); `hud_string(&craft, place, rgba, key)` names a string id of the mod's `strings/<language>.toml` instead of carrying the text |
| `hud-icon{slot, anchor, x, y, sprite, rgba, size}` | `HudIcon` | `hud_icon(&craft, place, rgba, sprite)`, `hud_mod_icon(&craft, place, rgba, name)` for the mod's own PNG | as `hud-text` |
| `set-weather{kind, strength}` | `SetWeather` | `set_weather(kind, strength)` | `strength` outside `[0, 1]` or not finite |
| `set-sun{intensity}` | `SetSun` | `set_sun(intensity)` | outside `[0, 1]` or not finite |
| `grade-sky{grey}` | `GradeSky` | `grade_sky(grey)` | outside `[0, 1]` or not finite |
| `give-shield{slot}`, `hold-shield{slot}`, `clear-shield{slot}` | `GiveShield`, `HoldShield`, `ClearShield` | `give_shield(&craft)`, `hold_shield(&craft)`, `clear_shield(&craft)` | a second Shield command for the craft this tick |
| `prefer-route{slot, route}` | `PreferRoute` | `prefer_route(&craft, route)` | a route at or past the region's route count; a second for the craft this tick |
| `emit{slot, channel, value}` (23) | `Emit` | `emit(&craft, channel!("<mod id>/<name>"), value)` | a non-finite value (`Refused::Range`); a fifth in one tick (`Refused::Emits`; the host faults the module); a channel the sending mod's `[channels] publishes` does not name (the host faults it) |

### `pickup-look{slot, icon, colour}`

A mod that hands out its own pickup (`give-pickup`, `pickup-roll`) declares its look in
`oag-mod.toml`; a mod that does not is faulted for the race on its first such command, naming
the table:

```toml
[pickup]
icon = { title = "turbo" }          # or { mod = "<key in [icons]>" }
colour = { rgba = "E0202080" }      # RRGGBBAA, or { title = "shield" }
```

`pickup-look` overrides it for one tick and slot (record: `arg` the icon's weapon code, or
bit 15 and a pool offset for a mod icon; `value` 0 for an rgba or the colour weapon's code;
`word` the rgba). The SDK's `Icon::{Title(Weapon), Mod(ModIcon)}` and
`Colour::{Title(Weapon), Rgba(Rgba)}` name the sources, with the per-weapon constants
`icon::title::TURBO`, `colour::title::SHIELD` and so on for all fourteen weapons. A bad
weapon in a record is malformed by name. An output, never hashed; `hud-shake` of the
`Pickup` element still works.

Every method also refuses a 97th command (`Refused::Full`). The host's checks stay
the backstop for a guest written without the SDK.

### `scale-thrust{slot, factor}`

- **As built**: `factor` in `[0, MAX_THRUST_FACTOR]` (`[0, 2]`), sent each tick it should
  hold, no `ticks` field and no host timer. At or below 1.0 it is multiplied into
  `pending_thrust_scale` at phase C; above 1.0 it is the mod's own engine scale,
  multiplied across modules and capped. The timer and the true top-speed scale below
  are the Feisar walk's engine change, not built.

- **Lands in** `Ship::pending_thrust_scale` (`crates/gameplay/src/world.rs:204`),
  which the step consumes and resets to 1.0 (`player_step.rs:231`, and the same for
  AI craft before `field.rs:871`) and passes as `Environment::thrust_scale`
  (`forces.rs:292`, the Leach Beam's slow-down input, "at or above 1.0 is neutral").
  **Engine change:** that field is one value re-armed every tick by the beam; a mod
  scale needs a host-side timer per slot that multiplies into it before the step,
  and the force law must accept a factor above 1.0 there. A true top-speed scale,
  rather than thrust, is one more `Environment` field defaulting to the identity, so
  every race without the mod passes `..Environment::default()` and stays
  bit-identical.

### `impulse{slot, delta-velocity}`

- **Not through `ShipState::pending_impulse`.** That port
  (`oag_physics::wall::apply_pending_impulse`) also arms the collision stun
  (`STUN_PER_CONTACT`, see `ShipState::stun_timer`'s doc in
  `crates/physics/src/ship.rs`), so a spring jump through it would stun the craft.
- **Lands at** the point between a craft's controls and its step where
  `oag_physics::rail::apply` already writes a body (`player_step.rs:256`,
  `crates/physics/src/rail.rs:17`; the AI's at `field.rs:865`): a velocity add, one
  per slot per tick, magnitude capped.

### `hold-mag-lock{slot, ticks}`, `shield-delta{slot, amount}`

- `hold-mag-lock` sets the maglock's engage input for the slot (`oag_physics::maglock`)
  before the step; it holds only where a magnetic floor is under the craft.
- `shield-delta` goes through the damage path (`crates/physics/src/damage.rs`) so the
  shield flash, the absorb window and the wreck threshold behave as a hit does.
  Applied before the step, with this tick's commands.

### `give-pickup{slot, weapon}` / `clear-pickup{slot}`

- **Lands in** `Ship::pickup` (`world.rs:114`, `oag_weapons::pickup::Held`) before
  the step. Refused in a mode with weapons off (`oag_race::Mode`).
- **A mod weapon needs an engine change first:** `oag_tables::weapons::Weapon`
  (`crates/tables/src/weapons.rs:91`) is a closed enum of the titles' weapons: a
  custom arm, its icon from the mod's files, a row in
  [the weapon lineage](../../gameplay/weapon-lineage.md), and the network weapons
  path of
  [ADR-0071](../adr/0071-weapons-are-predicted-simulation-with-a-discrete-state-correction-trigger.md).
  Projectiles of a mod's own stay out of `World::projectiles` (`world.rs:378`) so
  the vanilla snapshot's size never changes (README, open question 4).

### `set-finished{slot}`

- **Today's rule** is in `oag-race`: `RaceState::complete_lap` finishes a craft when
  `lap > laps_target` (`state.rs:538`), Zone on `zone_target` (`state.rs:520`),
  elimination through `RaceState::eliminate`. `Standing::update` finishes a craft on
  its own `laps_target` argument and records `finish_tick`; `Standing::finished`
  (`standing.rs:133`) reads it. AI craft finish only there.
- **Lands in both:** `Standing::finish_tick` for every craft, and
  `RaceState::finished` for a person's slot (results, HUD and records read it), at
  phases 8 and 9 of the same tick. A mode that finishes by mod passes a lap target
  of `None` to both (`laps_target: None` already means "never ends on its own",
  `state.rs:154`), so `oag-race` keeps one rule set and the mod decides from
  outside. Refused: a second finish of one slot.

### `set-standing-key{slot, key}`

- **Lands in** the ordering `oag_race::places` computes (`standing.rs:322`, read by
  `Race::places` in `field.rs`): a per-slot key sorted before progress and time.
  **Engine change:** `places` takes standings and the course only today.

### Built for Energy Turbo (2026-10-29)

- **`pickup-fire{slot}` / `pickup-absorbed{slot}`**: raised in `Race::spend_pickup`
  (`crates/raceplay/src/weapons.rs`, a person's fire or absorb press) and
  `Race::spend_opponent_pickup` (`crates/raceplay/src/field/opponent_weapons.rs`, the
  driver's fire wherever the Turbo's gate passes; it never absorbs) for a slot holding
  the mod's own pickup (`Weapon::Extension`). Delivered at the same tick's call (phase
  2 is before C).
- **`speed-pad{slot, pad}`** / **`weapon-pad{slot, pad}`**: raised at the entry edge
  of `Race::test_speedup_pads` and `Race::test_weapon_pads`
  (`crates/raceplay/src/pads.rs`) for every craft, whatever the pad's factor or
  cooldown; delivered at the next tick's call. **`trigger-enter` / `trigger-exit`**:
  `Extensions::test_triggers`, called beside the pad tests in the people's step and the
  field's with the same sweep, keeps each slot's inside mask; the call reports the
  difference from the last call's ([README](README.md#and-track-properties-2026-10-30)).
- **`scale-pad`** is host state (`RaceSim::extensions`: each pad's held factor, each
  slot's factor this tick), hashed, read by `Race::test_speedup_pads` as
  `Extensions::pad_factor`: the product scales the direction handed to
  `Environment::pad_hit`; at 0 no push and no plume, pad cue, boost count or Zone
  score, the edge and the cache the pad's as ever.
- **`shield-delta`** lands at C: a loss through `oag_physics::damage::apply_weapon`, a
  gain through `damage::add`.
- **`give-pickup` / `clear-pickup`** write the slot's pickup at C. **`pickup-roll`**
  is host state (`RaceSim::extensions`: the rule, its count and the owning module),
  hashed, and is read at the weapon pad's grant in `Race::test_weapon_pads`
  (`crates/raceplay/src/pads.rs`).
- **`scale-thrust` above 1.0** lands in `Environment::extension_thrust`, taken by the
  step (`player_step.rs`, `field.rs`).
- **`play-cue`** pushes onto `RaceSim::cues`. **`fire-effect`** queues for
  `Race::draw_extension_effects` (`tick.rs`'s drawn-tick block, beside the hit sparks).
  The five **`hud-*`** commands are collected per tick in `Extensions` and become one
  `oag_hud::ext_draws::Outputs` per slot in `Race::readout_for`
  (`Readout::ext`); `draw_list` applies the recolours, shakes and bar shares where it
  builds each widget, and `oag_game::hud_overlay` appends the text and icons
  (`ext_draws::append`) in the layout's grid, as a view of their own per slot in a split
  screen. Guests format numbers and times with `oag_ext::Line`.
- **`set-weather`, `set-sun`, `grade-sky`** are race-wide (slot 0), held in
  `Extensions` (`Sky`) for the tick, cleared at the next call, and read by the view half:
  `Race::advance_weather` drives the track module's rain (`Weather::drive`), `Scene`'s lens
  flare placement scales the rings' alpha, and the sky draw writes `Uniforms::sky_grey`.
- None of the outputs is hashed. The README's "What is built" has the table.

### `prefer-route{slot, route}` (Checkpoint Circuit, 2026-10-30)

- **Lands in** `Extensions` (`crates/raceplay/src/extensions/route.rs`): the slot's route for
  this tick, cleared at each call, hashed after the no-module early return. `route` is the
  course's numbering (`Course::routes`, the region's route entities); the host faults a route
  at or past the count and two for one slot; two modules for one slot, the later in id order
  wins.
- **Read by** `Race::steer_branching` (`crates/raceplay/src/routes.rs`), run for every craft
  a driver flies before it drives (`Ai_ChooseBranch`'s states 0 to 2): the coin is drawn as
  before on entering the pre-fork path, then where the driver moves onto a route's line
  (within `SWITCH_SAMPLES` of the split) a held preference replaces the coin's draw, if the
  route leaves that fork and its speed plan verified. The coin's stream (the driver's seed and
  visit count) never moves.
- **One tick, not one fork.** Sent at C, read in phase 6 of the same tick; a guest re-sends it
  every tick until the switch. A person steering is not moved by it.

### Events between mods: `emit` and `mod`

- **Command** `emit{slot, channel, value}` (23, `Command::Emit`; record: `slot` byte, `value` at 4,
  the channel key at 8). `channel` is `channel_key("<mod id>/<name>")`, the FNV-1a 32 hash a property's
  name has (SDK `channel!`). The slot must hold a craft, like any slot-bearing command.
- **Event** `mod{slot, from, channel, value}` (kind 10, `Event::Mod`): `from` is `channel_key` of the
  sending mod's id, or `TRACK_SENDER` (0) when a track's trigger sent it. Events grew to **16 bytes**
  for it: kind `u8`, slot `u8`, index `u16`, `value` `f32` at 4, channel `u32` at 8, sender `u32` at 12
  (zero in every other kind).
- **Lands in** `Extensions` (`crates/raceplay/src/extensions/channel.rs`, `on.rs`): the queue
  `mail`, filled by the modules' emits (applied after every module has run, so each saw the same
  observation) and by `Extensions::test_triggers` for a trigger that sends a channel. At the
  **next** call it is taken, **sorted** by channel, slot, sender and value, held to
  `MAX_MOD_EVENTS` (16) for the modules' and `MAX_TRACK_EVENTS` (8) for the track's triggers, each class
  in its own cap so neither starves the other, and written **ahead of** the engine's own events, so a
  crowd of wall contacts cannot push one out of `MAX_EVENTS`. A drop past a cap (and past `MAX_EVENTS`)
  is deterministic (the later in sorted order) and counted in `Extensions::drops()`, with one `warn!`
  in the race log the first time each kind drops; the counters are outputs, never hashed. Every module hears every `mod` event, whoever sent it: the
  order modules run in (id order) changes nothing any of them sees. Module ids still differ in
  `from`, so a pair of mods that swap ids hear different senders, not different events.
- **Hashed.** The queue is race state, sorted and written in `On::hash_into` with the modules'
  state, so it is in the hash from the end of the tick that raised it (before any module has seen
  it); `a_race_with_no_module_hashes_as_before` and `the_queue_is_in_the_hash_before_anyone_has_heard_it`
  (`tests/extension_channels.rs`) hold both ends. The beacons' inside masks are hashed when a module
  runs and never otherwise.
- **Faults, each by name.** The module's mod must publish the channel (`oag-mod.toml` `[channels]`:
  `emit on channel 0x..., which its oag-mod.toml [channels] publishes does not name (it publishes
  [...])`), and a module may send at most `MAX_EMITS_PER_TICK` = **4** a tick (`emit past the cap
  of 4 a tick`). The module is off for the rest of the race and **none** of that reply lands (README section 4.2). Caps are **chosen, not measured**: 4 a module a tick, 16 module
  events and 8 track events a tick.
- **A track needs no module.** `Beacons` (always built, with or without the `extensions` feature)
  tests the track's emitting triggers against every craft's sweep each tick, notes when each channel
  last fired (`Extensions::fired`, an output, never hashed) and, when a module runs, queues a
  `mod` event from `TRACK_SENDER` (value `1.0`). `Race::channel_started(channel)` turns that tick
  into the scenery's clock for the view's `oag_on` animations (`Keyed::on`,
  `Model::sample_anim_nodes_started`): render only, collision never moves.

### `message{slot, string-id}` and `publish{key, value}`: outputs, not state

- Both are **outputs of the simulation**, like cues
  ([ADR-0018](../adr/0018-audio-mixer-architecture.md)): the HUD's message board
  lives in the view half (`self.view.huds[slot].messages`, advanced at `tick.rs:83`)
  and `oag-server` has no view. So the host queues them beside `self.sim.cues` and
  the view half drains them, as it drains cues; a sim-only replay drops them, and
  they never enter the hash.

### `assist{slot, ...}`: Pilot Assist as a module (the phase 2 proof)

Pilot Assist on Pulse and Pure is the planned oracle (bit-identical replay hashes
against the built-in law, `pilot_assist_ground_truth.rs`). The built-in law is fed
through `Environment::pilot_assist`, which `Race::pilot_assist_input`
(`crates/raceplay/src/pilot_assist.rs`, called at `player_step.rs:219`) builds from
the person's controls and the craft's pose. A module version needs a command that
lands there, for person slots, before the step, carrying what
`pilot_assist_input` computes today. The README's command list must carry it, or
the phase 2 proof has no sink.

## `race-start`: the grid

`race-start` runs once after the grid is placed: `Race::start`
(`crates/raceplay/src/start.rs:30`), through `spawn_pose`
(`crates/raceplay/src/spawn.rs:65`, a pose from the track's authored start position
or the spline's start sample) and `grid_poses` (`spawn.rs:274`). It sees the grid;
it cannot move it (README: no grid changes). An arena's authored spawn points, and a
respawn at a point rather than a spline sample (`Race::respawn` places a craft on a
sample), are engine changes in the track file and `respawn.rs` first.

## Render side: the `hud` world

These run on the render thread, once a frame, and never touch the simulation
(phase 1 of the README's rollout; no determinism needed).

- **The values a HUD module reads** are a copy of `oag_hud::Readout`
  (`crates/hud/src/lib.rs:555`), filled by `Race::readout_for`
  (`crates/raceplay/src/telemetry.rs:122`), plus the keys a sim module `publish`ed.
- **Its draws** are appended to the `Frame` (`crates/hud/src/draw.rs:695`) that
  `oag_hud::draw::draw_list` (`draw.rs:796`) builds, in `HudOverlay::draw`
  (`crates/game/src/hud_overlay.rs:335`, `draw_list` at `:346`; split views at
  `:89` and `:94`), in a subset of `oag_ui`'s `Draw` vocabulary
  (`crates/ui/src/frontend/draw.rs:28`).

## Menu side: the `frontend` world

A mod screen runs in the menus, never in a race, and touches none of the sites above
([README section 2.3](README.md#23-the-frontend-world)). Its engine meeting points:

- **The row**: a mod's `menu.toml` row with `action = "open-mod-screen"` and
  `screen = "<id>"`; the new `oag_ui::menu::vocabulary::Action` variant
  (`crates/ui/src/menu/vocabulary.rs:13`) handled in `oag-game`, which opens the
  screen model over `oag-ui-screens` (the `Input` and `Pointer` in, `Outcome` and
  `Draw` out shape of `TagEntry`, `crates/ui-screens/src/tag_entry.rs:361`, `:423`,
  `:472`). The host widgets take pad and pointer through the same `Input` and
  `oag_ui::pointer::Pointer` (`crates/ui/src/pointer.rs:46`).
- **The save**: a reply's `save` goes into the records `Store`
  (`crates/game/src/records.rs:539`) and out through `records::save_in_background`
  (`:1090`), off the render thread; inline on the web (`:1098-1102`).
- **Into a race**: the one place a saved blob reaches a race is `setup.tune` at
  `race-start`, built at `Race::start` (`crates/raceplay/src/start.rs:30`) from the
  load's copy of the blob, never read from disk during the race.
- **Not wired anywhere else**: the frontend host is not called from
  `Race::tick_body`, and the sim hook never sees a screen.

## No module loaded: no host

The `Host` (wasmtime `Engine`) is built at the first module load by a race or a
screen, not at start-up, and `Race::tick_body`'s call site (C) iterates the loaded
sim modules: with none, the loop body never runs, no observation is built and no
buffer is written, so a vanilla tick is unchanged and the hash gains nothing
([README section 4.5](README.md#45-with-no-mods-loaded-nothing-runs)). The test that
proves it is part of the first build step below.

## The maintainer's examples, mapped

The README walks each end to end; this is what each needs from the engine.

| Example | Events and commands | Works on today's engine? |
| --- | --- | --- |
| **A craft that gets faster on each speed pad** (2048's prototype craft as the inspiration) | `pad-crossed` counted in the state blob; `scale-thrust` from the count | **Yes for thrust**: `scale-thrust` above 1.0 is the mod's engine scale, re-sent each tick (no host timer). A true top-speed scale is one new `Environment` field, not built. See the note on 2048 below. |
| **Spring jumps** on a custom track | `volume-entered`, `impulse` | **No**: needs the volume node kind and the velocity write point. |
| **A gravity switch** (drive from the top of the track to the bottom) | `volume-entered`, a gravity command (README open question 3) | **No**: gravity acts on world `y` only (`oag_physics::passive::gravity`, `crates/physics/src/passive.rs:309`; `forces.rs:221` says so). Needs a gravity direction in `Environment` (default `-Y`, bit-identical for every race that does not set it) and per-craft gravity state in `World`'s `Ship` (`world.rs:82`). The hover already casts along the craft's own `-up` (`crates/physics/src/hover.rs:610`) and the magstrip hold slaves attitude to the track (`oag_physics::maglock`), so the craft side follows; the camera and the AI's line need their own check. |
| **A-to-B instead of a circuit** | `set-finished` from progress | **Partly.** A sprint over part of a closed circuit needs only `set-finished` when progress passes a sample. A truly open track does not work: the lap is a wrap of distance along a closed `Course` (`crates/race/src/course.rs:61`; `wrapped_forward`, `state.rs:35`), and the spawner, the respawn, the AI speed plan (a ring) and the flyby assume a loop. Rules as data in `oag-race` first (finish by laps or by distance); a drag race then needs no code. |
| **Custom track rules** (lives, a time limit, checkpoints) | `weapon-hit`, `volume-entered`, `set-finished`, `set-standing-key`, `message` | **Mostly**, once `set-standing-key` reaches `places`. Kills read what `credit_blast` already credits. |
| **Custom weapons** | `weapon-fired`, `hit`, `give-pickup`, `scale-thrust`, `shield-delta` | **No**: `Weapon` is a closed enum. Timed self-effects (a Turbo-like boost, a shield) are the easy first case; projectiles of a mod's own are open. |
| **Custom HUDs and extensions** | the `hud` world, `publish` | **Mostly**: needs the HUD module host and the published-keys channel. |
| **Pilot Assist as a mod** (the phase 2 proof) | `assist` | **Yes**, once the command exists (above). |

### The 2048 prototype craft

2048's four craft per team are fighter, agility, speed and **prototype**
(`oag_2048::race::SHIP_TYPES`, `crates/2048/src/race.rs:304`;
[2048-status.md](../../formats/2048-status.md)). Their individual specialties are
**not recovered**: the team-select panel draws two ability icons per craft but "the
prototype's description is open" ([2048-frontend.md](../../formats/2048-frontend.md),
the `team` row), and the HUD's `SpeedPad*` family is chosen once from a ship
definition float, with "the purple swoosh on a pad" unexplained
([2048-hud.md](../../formats/2048-hud.md)). So:

- **On 2048 (and Omega, which keeps 2048's ship directories) the prototype laws are
  the original's** and belong to the engine: recovered in Ghidra, documented with
  confidence, wired as `Title` data and checked against Omega. A mod is not the way
  to ship an original behaviour, and a one-tick-late mod law would not match it.
- **A mod is the way to bring such a specialty to a title that never had it**, or to
  a custom craft. That is the example above.

## What has to be built, in order

Each is its own merge, each with a proof that a vanilla race's hashes do not move.

1. The host (wasmtime engine and configuration, fuel, the fault path, the pooled
   per-tick instance) with no call site wired, built lazily at the first module
   load; with the test that a vanilla race with the `extensions` feature on matches
   the feature-off hashes and creates no `Engine`.
2. The `hud` world (phase 1), and the `frontend` world beside it
   ([README section 2.3](README.md#23-the-frontend-world)): `open-mod-screen`, host
   widgets, the saved blob. `setup.tune` waits for step 4.
3. Call site C, the event buffer, the state blob in the race state and the hash, and
   the `assist` command, proven by Pilot Assist on Pulse and Pure (phase 2).
4. `scale-thrust` with its timer, then the craft example.
5. Rules as data in `oag-race`; `set-finished`, `set-standing-key`.
6. Engine changes: track volumes; the velocity write for `impulse`; a gravity
   direction; a custom weapon arm; authored spawn points.
