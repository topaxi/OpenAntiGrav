# WebAssembly extensions: where the engine calls a mod

**Status: proposal (2026-10-29), nothing is built.** The overview, the capability
surface and the ABI sketch are in [README.md](README.md); the determinism rules,
the state model and the spike's measurements in
[determinism-and-state.md](determinism-and-state.md). This page is the engine half:
every hook a mod's WebAssembly module can export, the call site it would hang off in
today's code, and what the host does and refuses there.

Every `file.rs:line` below is pinned to commit `b4b4cb3aa` (main, 2026-10-29) and
named with its function, so a citation survives drift: search for the function if
the line moved.

## The shape every hook shares

- **Data in is a copy.** The host writes a fixed, versioned observation struct
  into the guest's I/O buffer before the call. The guest never holds a pointer into
  `World` or `RaceSim`; it cannot read what the host did not copy.
- **Data out is a bounded command.** The guest writes a command record (a few
  fixed fields, or a short list with a hard cap). The host validates every field
  (finite, in range, the slot exists, the command is one this hook accepts) before
  anything reaches the simulation. A NaN or out-of-range value is a fault, never
  clamped silently.
- **State is a host-owned blob.** Whatever must persist between calls lives in the
  mod's fixed-size blob inside the simulation state, copied in and out around each
  call (see [determinism-and-state.md](determinism-and-state.md#mod-state-is-a-fixed-size-blob-in-the-race-state)).
- **Failure has one answer.** A trap (out of fuel, `unreachable`, out-of-bounds
  memory, a stack overflow) or a refused command **faults the mod for the rest of
  the race**: the hook's outputs from that call are discarded, the blob keeps its
  pre-call value, and every later call to that mod is skipped as if it exported
  nothing. Same inputs, same fault tick on every machine (the spike measured the
  trap point identical on Cranelift and Pulley). The fault is logged naming the mod,
  the hook and the tick, and shown on the HUD and the results screen.
- **Order is fixed.** Within a hook, mods run in the order their ids sort; within a
  mod, slots run in slot order, the order the tick already steps them in.

### Where the hooks live

The race tick is `Race::tick` / `Race::tick_body`
(`crates/raceplay/src/tick.rs:48`, `:58`), in `oag-raceplay`'s simulation half (the
half `oag-server` links with `view` off). That is where every simulation hook is
called from, as `Race::tick` already calls `oag_physics::step`, `oag_weapons` and
`oag_race`. `oag-physics`, `oag-race`, `oag-weapons` and `oag-ai` never learn that a
mod exists: they see more `Environment` fields, more `RaceState` rules, or a
`Controller` implementation, exactly as they see Zone's auto-speed today
(`oag_physics::forces::Environment::auto_speed`, `crates/physics/src/forces.rs:260`).
Where the host itself lives, and why that respects the dependency rules, is in
[determinism-and-state.md](determinism-and-state.md#dependency-rules).

### The tick, with the hooks in it

`Race::tick_body` (`crates/raceplay/src/tick.rs:58`), in order. Hooks in **bold**.

| # | Phase | Today's call site | Hook |
| --- | --- | --- | --- |
| 0 | Grid flags, hover caps | `tick.rs:118` to `:131` | |
| 1 | Every person's controls | `Race::player_controls`, `tick.rs:137` to `:142` | **`craft.controls`** (person slots) |
| 2 | Pickups spent, autopilot | `Race::spend_pickup`, `tick.rs:166` to `:173` (`crates/raceplay/src/weapons.rs:148`) | **`weapon.fire`** |
| 3 | Mines, cannons | `tick.rs:180`, `:186` | |
| 4 | Each person's craft | `Race::step_player`, `tick.rs:196` (`crates/raceplay/src/player_step.rs:136`) | **`pad.enter`**, **`volume.enter`**, **`craft.environment`** |
| 5 | Projectiles fly, hit | `oag_weapons::projectile::step`, `tick.rs:235`; impacts `tick.rs:267` | **`weapon.hit`** |
| 6 | Rescue, AI craft, pairs | `tick.rs:415` to `:422`; `Race::step_opponents` (`crates/raceplay/src/field.rs:689`) | **`craft.controls`** (AI slots), and phase 4's hooks for AI craft |
| 7 | Clock | `self.sim.world.tick += 1`, `tick.rs:446` | |
| 8 | Race rules per person | `Race::advance_player_rules`, `tick.rs:493` (`player_step.rs:381`) | **`race.lap`**, **`race.finish_rule`** |
| 9 | Standings | `Race::update_standings`, `tick.rs:501` (`field.rs:1019`) | |
| 10 | Quake, leach beam, repulsers | `tick.rs:510` to `:530` | |
| 11 | (new) | between `:530` and `Race::capture_results`, `tick.rs:541` | **`race.tick_end`** |
| 12 | Results, camera, HUD state | `tick.rs:541` onwards (view half) | |

Outside the tick: **`race.spawn`** at race start (`Race::start`,
`crates/raceplay/src/start.rs:30`, through `spawn_pose`,
`crates/raceplay/src/spawn.rs:65`) and at a respawn (`Race::respawn`,
`crates/raceplay/src/respawn.rs:280`). On the render side, once a frame and never
in the simulation: **`hud.readout`** and **`hud.widget`**.

## Simulation hooks

### `craft.controls`: steer a craft

- **Where.** A person's slot: `Race::player_controls` (`player_step.rs:99`),
  called for every person-flown slot at `tick.rs:137` to `:142`, before anything
  moves. An AI slot: `Race::step_opponents` builds `controls` at `field.rs:738`
  (from `opponent_autopilot_controls` or the driver) before its own
  `oag_physics::step` at `field.rs:871`.
- **Two uses, one hook.** (a) **An assist** on a person's slot: the mod sees the
  player's controls and returns a correction, the shape of Pilot Assist
  (`oag_physics::pilot_assist`, fed by `crates/raceplay/src/pilot_assist.rs`).
  (b) **A controller** for an AI slot: the mod returns the controls outright, the
  pilot file's `controller = "mod:<name>"` proposal. For (b), `oag-ai` gains the
  `Controller` trait the pilot-controller design already calls for, and the host
  implements it.
- **Sees.** The slot's own pose, velocity, speed, shield, craft state, pickup; the
  person's `InputSnapshot` for (a) (`crates/gameplay/src/input.rs:28`); the nearest
  spline sample and a fixed number of samples ahead (index, position, frame,
  racing-line offset); the other craft's poses and speeds, a fixed array of
  `MAX_SHIPS`; the tick; the mod's blob.
- **Writes.** `ShipControls` (`oag_physics::ShipControls`): thrust, steer, pitch,
  airbrakes, each range checked; for an AI slot also the fire and absorb presses
  (a person's stay the person's own buttons).
- **Refuses.** Values outside the axis ranges, NaN; for (a), a correction on a slot
  the race did not grant an assist to (assists are per mode, as Pilot Assist is
  per title today); for (b), a slot no mod controller was assigned to.
- **Cost note.** Up to eight calls a tick; the dominant hook for a mod controller.

### `craft.environment`: change the forces on a craft this tick

- **Where.** The `Environment` the force law reads, built in `Race::step_player` at
  `player_step.rs:232` to `:250` and passed to `oag_physics::step` at
  `player_step.rs:258`; for AI craft built at `field.rs:851` and passed at
  `field.rs:871`. The hook runs after the pads (so it sees this tick's pad edge) and
  before the step.
- **Sees.** As `craft.controls`, plus this tick's pad and volume state (inside a
  speed pad, which one, entered this tick), the craft's handling class.
- **Writes.** Multipliers on terms the force law already takes as inputs, each
  bounded by the host: a thrust scale folded into
  `Environment::thrust_scale` (`forces.rs:292`, today the Leach Beam's slow-down
  input, neutral at 1.0), a gravity scale folded into `class_gravity_scale`
  (`forces.rs:245`), a speed-pad strength scale on `pad_hit`'s direction
  (`forces.rs:261` to `:270`: an unnormalised direction already scales the boost).
  New multipliers (top speed, grip, drag) are engine changes: each is one more
  `Environment` field defaulting to the identity, so a race without the mod passes
  `..Environment::default()` and stays bit-identical.
- **Refuses.** Any multiplier outside its declared band (a mod declares the band in
  its manifest, the host caps it at a hard ceiling); NaN; a write to a craft the
  mod is not bound to (a craft mod is bound to the slots flying that craft).
- **Not given.** A force, a velocity or a position. The guest scales the
  original's terms; it does not replace the integrator. A position write is
  `volume.enter`'s `impulse`, below, and is validated more strictly.

### `pad.enter`: a speed or weapon pad was entered

- **Where.** The entry edge of the speed pad, `Race::test_speedup_pads`
  (`crates/raceplay/src/pads.rs:73`), at `if entered != self.sim.pad_current[slot]`
  (`pads.rs:113`), the edge the Zone score and the exhaust flare already hang
  off; and the weapon pad's edge in `Race::test_weapon_pads` (`pads.rs:216`) at
  `pads.rs:258`. Both are called for a person's craft from `step_player`
  (`player_step.rs:210`, `:215`) and for an AI craft from `step_opponents`
  (`field.rs:824`, `:825`), so one hook site covers every craft.
- **Sees.** Slot, pad kind (speed or weapon), pad index, the craft's speed, the
  tick, the blob.
- **Writes.** Only the blob, plus at most one command from `race.tick_end`'s
  vocabulary (score, a message id). An event hook: what it changes takes effect
  through the blob on the next `craft.environment` call, which runs later in the
  same craft's step.
- **Refuses.** Commands outside that vocabulary. A weapon pad's grant is the
  title's own law and is not overridable here (`weapon.fire` is where a mod weapon
  acts).

### `volume.enter` / `volume.exit`: a track-authored trigger volume (engine change)

- **Where (to be built).** Today a track carries two kinds of trigger box: speed
  and weapon pads (`docs/formats/track-file.md`, the `oag/pads/speedup` and
  `oag/pads/weapon` nodes, each a matrix plus `_OAG_MIN`/`_OAG_MAX`), tested by the
  shared sweep (`Race::pad_sweep`, `pads.rs:45`) with `contains` and `distance`;
  and the reset zones `Race::reset_zone_contact` reads (`respawn.rs:235`). There is
  no generic "named volume a mod may react to". The proposal generalises the pad
  test: a third node kind, `oag/volumes/<name>`, the same box shape, tested in the
  same sweep after the weapon pads, with an entry and an exit edge per craft.
- **Sees.** Slot, volume name (an id the mod's track declares), entry or exit, the
  craft's pose and velocity, the tick, the blob.
- **Writes.** The blob; `set_gravity(direction)` on that craft (needs the gravity
  change below); `impulse(velocity_delta)` applied between the controls and the
  step, at the point `oag_physics::rail::apply` already writes a body
  (`player_step.rs:256`, `crates/physics/src/rail.rs:17`), capped in magnitude per
  tick; `respawn_at(volume or sample)`.
- **Refuses.** An impulse over the cap or not finite; a gravity direction that is
  not unit length within tolerance; any command on a volume the track did not
  author; any command from a mod that is not the track's own (a track's volumes
  belong to that track's mod).

### `weapon.fire`: a pickup was spent

- **Where.** A person's craft: `Race::spend_pickup` (`weapons.rs:148`), called at
  `tick.rs:168`, edge-triggered on fire/absorb. An AI craft:
  `Race::spend_opponent_pickup` (`crates/raceplay/src/field/opponent_weapons.rs:329`),
  called from `step_opponents` at `field.rs:826`, where the driver's weapon decision
  (`oag_ai::Driver::wants_to_fire`, `crates/ai/src/driver/weapons.rs:72`) is acted on.
  (`spend_pickup_under_autopilot`, `field.rs:357`, is the Autopilot pickup's path,
  not a fire decision.)
- **Sees.** Slot, the weapon held, fire or absorb, the craft's pose and speed, the
  other craft (as `craft.controls`), the blob.
- **Writes.** For a **mod weapon** only (see below): a timed effect on the firing
  craft (thrust scale and duration, the shape the Turbo has, `weapons.rs` sets
  `turbo_timer`), a shield timer, or one projectile from a host-defined template
  (speed, lifetime, damage, homing on or off) into the fixed pool
  (`oag_weapons::projectile::Projectiles`, `crates/gameplay/src/world.rs:378`).
- **Refuses.** Changing an original weapon's law (the disc's tables decide those;
  `docs/gameplay/weapon-lineage.md`); a projectile when the pool is full (the
  command is dropped, deterministically, as a full pool would drop an original
  shot); damage or speed outside the template's caps.
- **Engine change first.** `oag_tables::weapons::Weapon`
  (`crates/tables/src/weapons.rs:91`) is a closed enum of the titles' weapons; a
  mod weapon needs a `Weapon::Mod(index)` arm, a pickup icon from the mod's assets,
  a row in the weapon-lineage table, and the network weapons path of
  [ADR-0071](../adr/0071-weapons-are-predicted-simulation-with-a-discrete-state-correction-trigger.md)
  to treat it like any other weapon.
- **Open: AI craft and mod weapons.** Our driver's weapon decision knows the
  titles' weapons only; an AI craft holding a mod weapon would never fire it. Either
  a mod weapon declares which original weapon's AI use it borrows, or only mod
  controllers (`craft.controls` on AI slots) fire it. Not decided.

### `weapon.hit`: a projectile hit a craft or a wall

- **Where.** The impacts `oag_weapons::projectile::step` returns
  (`crates/weapons/src/projectile.rs:536`, called at `tick.rs:235`), walked at
  `tick.rs:267`, where `Race::credit_blast` (`crates/raceplay/src/eliminator.rs:498`)
  already credits kills.
- **Sees.** Weapon kind, owner slot, struck slot or wall, point, whether a shield
  absorbed it, the blob.
- **Writes.** The blob (a lives counter for the arena mode); `race.tick_end`'s
  commands. It cannot change the damage already applied: that is the weapon's law.
- **Refuses.** As `race.tick_end`.

### `race.lap`: a lap edge

- **Where: two sites, because a person and an AI craft count laps through two
  structs.**
  - A person's slot: `Race::advance_player_rules` (`player_step.rs:381`), run only
    for the slots stepped as people (`tick.rs:452` to `:494`), calls
    `RaceState::update` (`crates/race/src/state.rs:410`) and reads its `Outcome`
    (`state.rs:590`: `lap_completed`, `first_crossing`, `lap_ticks`, `finished`,
    `zone_advanced`, `perfect_zone`). The hook runs right after, with that outcome.
  - Every craft, AI included: `Race::update_standings` (`field.rs:1019`) runs
    `advance_standings` (`field.rs:1258`), which calls `Standing::update`
    (`crates/race/src/standing.rs:175`) per active craft; it returns whether a lap
    completed. An AI craft's `RaceState` is deliberately not advanced (the comment
    at `field.rs:1274` to `:1278`), so for AI slots the hook hangs off this return.
- **Sees.** The slot's `RaceState` (lap, best lap, splits, zone, score) and the
  outcome; the blob.
- **Writes.** `race.tick_end`'s commands.

### `race.finish_rule`: when a craft's race is over

- **Where.** Today the rule is in `oag-race`: `RaceState::complete_lap` finishes a
  craft when `lap > laps_target` (`state.rs:538`), Zone on `zone_target`
  (`state.rs:520`), elimination through `RaceState::eliminate`. Standings are
  `Standing` (`crates/race/src/standing.rs`): `Standing::update` finishes a craft
  on its own `laps_target` argument and records `finish_tick`; `finished()` at
  `:133` reads it. AI craft finish only there.
- **What it is.** Not a callback inside `oag-race`: a mod's finish rule is a
  **command** from `race.lap` or `race.tick_end` (`finish(slot)`), and a race whose
  mode declares `finish = "mod"` turns the lap-count rule off (`laps_target: None`,
  which already means "never ends on its own", `state.rs:154`). So `oag-race` keeps
  one rule set; the mod decides from outside. **`finish(slot)` sets both flags**:
  `Standing::finish_tick` for every craft, and `RaceState::finished` for a person's
  slot (whose results, HUD and records read it). With a lap target of `None`
  passed to both, neither finishes a craft on its own.
- **Refuses.** Finishing a craft twice (the flag is an edge, as `eliminate` is),
  un-finishing one.

### `race.tick_end`: the mode's own rules (engine change: the call site)

- **Where (to be added).** After the standings and the field-wide weapons
  (`tick.rs:530`) and before `Race::capture_results` (`tick.rs:541`), so a command
  that finishes or eliminates a craft is in this tick's results.
- **Sees.** Every slot's `RaceState`, standing, shield, craft state, position and
  progress along the course; the tick; the blob.
- **Writes, bounded.** Per slot: `finish`, `eliminate`, `add_score(i32)`,
  `set_position_override` (for a standings order a mod defines, such as lives then
  time), `message(id)` (an id from the mod's strings, shown by the HUD); a capped
  list, at most one of each per slot per tick.
- **Refuses.** Unknown command ids, a slot out of range, an override that is not a
  permutation of the field.

### `race.spawn`: where craft start and come back (engine change for arenas)

- **Where.** `spawn_pose` (`spawn.rs:65`, a pose from the track's authored start
  position or the spline's start sample) and the grid walk after it
  (`grid_poses`, `spawn.rs:274`; `oag_gameplay::grid_pose`, `crates/gameplay/src/spawn.rs:435`);
  `Race::respawn` (`respawn.rs:280`), which puts a craft back on a spline sample.
- **Sees.** Slot, grid rank, the spline start, the track's authored spawn points
  (to be added for arenas), the blob.
- **Writes.** A pose per slot from a finite set: an authored spawn point by index,
  or a spline sample index plus a lateral offset. Never a free coordinate: the host
  places the craft at the chosen point's ground height, as `spawn_pose` already
  does with `ground_under`.
- **Refuses.** An index the track did not author; two craft on one point.

## Render-side hooks

These run on the render thread, once a frame, and never touch the simulation. They
need no determinism (phase 1 of the plan in [README.md](README.md)).

### `hud.readout`: values a mode publishes to the HUD (data, no call)

- **Where.** `oag_hud::Readout` (`crates/hud/src/lib.rs:555`), filled by
  `Race::readout_for` (`crates/raceplay/src/telemetry.rs:122`), read by
  `oag_hud::draw::draw_list` (`crates/hud/src/draw.rs:796`).
- **What it is.** A mod declares named fields over its blob in its manifest (name,
  offset, type: `u32`, `i32`, `f32`, per slot or global). `readout_for` copies them
  into a fixed `mod_values` array on `Readout`. **No guest code runs**: a lives
  counter or a distance-to-finish is already in the blob the simulation hooks
  wrote. A HUD layout (the TOML HUD files the custom-modes design plans) binds an
  element to one of those names.

### `hud.widget`: a computed widget

- **Where.** After `draw_list` builds the frame in `HudOverlay::draw`
  (`crates/game/src/hud_overlay.rs:335`, `draw_list` at `:346`; split views at
  `:89` and `:94`), the widget's draws are appended to that `Frame`
  (`crates/hud/src/draw.rs:695`).
- **Sees.** A copy of the slot's `Readout` including the mod values, the viewport
  size and safe area, the frame's interpolation fraction.
- **Writes.** A capped list of draws in a subset of `oag_ui`'s `Draw` vocabulary
  (`crates/ui/src/frontend/draw.rs:28`): `Fill`, `Sprite` and `Text` with a
  sprite from the mod's own assets or the HUD sheet by name, positions in the
  layout's coordinate space.
- **Refuses.** Video, backdrops, model draws, anything outside the safe area, more
  than the cap. A widget that traps is hidden for the rest of the race; the race
  goes on.

## The maintainer's examples, mapped

| Example | Hooks | Works on today's engine? |
| --- | --- | --- |
| **A craft that gets faster on each speed pad** (2048's prototype craft as the inspiration) | `pad.enter` counts into the blob; `craft.environment` returns a thrust scale from the count | **Yes**, through `Environment::thrust_scale`. A true top-speed scale is one new `Environment` field. See the note on 2048 below. |
| **Spring jumps** on a custom track | `volume.enter` on a jump volume, command `impulse` | **No**: needs the `oag/volumes` node kind in the track file and the impulse write point. |
| **A gravity switch** (drive from the top of the track to the bottom) | `volume.enter`, command `set_gravity` | **No**: gravity acts on world `y` only (`oag_physics::passive::gravity`, `crates/physics/src/passive.rs:309`; `forces.rs:221` says so). Needs a gravity direction in `Environment` (default `-Y`, bit-identical for every race that does not set it) and a per-craft gravity state in `World`'s `Ship` (`crates/gameplay/src/world.rs:82`). The hover already casts along the craft's own `-up` (`crates/physics/src/hover.rs:610`) and the magstrip hold slaves attitude to the track (`oag_physics::maglock`), so the craft side follows; the camera and the AI's line do not and need their own check. |
| **A-to-B instead of a circuit** | `race.finish_rule` (`finish` command) or, better, rules as data | **Partly.** A sprint over part of a closed circuit needs only a finish rule (`finish` when progress passes a sample), which the hooks can do. A truly open track does not: the lap is a wrap of distance along a closed `Course` (`crates/race/src/course.rs:61`; `wrapped_forward`, `state.rs:35`), and the spawner, the respawn, the AI speed plan (a ring) and the flyby all assume a loop. Do the rules-as-data refactor first (finish by laps or by distance); a drag race then needs no code at all. |
| **Custom track rules** (lives, a time limit, checkpoints) | `race.tick_end`, `weapon.hit`, `volume.enter` | **Mostly**, once `race.tick_end`'s call site exists. Lives and kills read what `credit_blast` already credits. |
| **Custom weapons** | `weapon.fire`, `weapon.hit` | **No**: `Weapon` is a closed enum and the projectile templates are the titles'. Needs `Weapon::Mod`. Simple timed weapons (a Turbo-like boost, a shield) are the easy first case. |
| **Custom HUDs and extensions** | `hud.readout` (data), `hud.widget` (code) | **Mostly**: `Readout` needs the `mod_values` array; the HUD layout files are planned separately. |
| **A mod AI controller** (arena navigation) | `craft.controls` on AI slots | **No**: needs the `Controller` trait in `oag-ai` first (a refactor with identical hashes). |
| **Pilot Assist as a mod** on Pulse and Pure (the phase 2 proof) | `craft.controls` (a), on person slots | **Yes**, once the host exists; its oracle is the built-in law's bit-identical replay hashes. |

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
  the original's**, and belong to the engine: recovered in Ghidra, documented with
  confidence, wired as `Title` data and checked against Omega, like any other law.
  A mod is not the way to ship an original behaviour.
- **A mod is the way to bring such a specialty to a title that never had it**, or
  to a custom craft. That is the example above, and what this design is for.

## What this design does not expose

Listed so nobody adds it by accident; [README.md](README.md) carries the full
capability list.

- **Raw state.** No pointer into `World`, `RaceSim` or another mod's blob; no
  reading what the observation did not copy (another craft's blob, the RNG's
  state, a projectile's internals).
- **The integrator.** No force, velocity or position write outside `impulse` and
  `respawn_at`, both capped and both on authored volumes.
- **The originals' laws.** No hook replaces a recovered law or a disc table; a mod
  scales inputs to it or adds its own mode rules beside it.
- **The renderer from the simulation.** A simulation hook emits no draw and no
  sound; cues the race raises from mod commands (a message, a score) go through
  the existing cue and HUD paths.
- **Time and the outside.** No clock, no file, no network, no thread, no randomness
  but the host's seeded stream
  ([determinism-and-state.md](determinism-and-state.md)).
- **The original campaigns.** They stay vanilla: no simulation mod runs in a
  title's own campaign.

## What has to be built before the first hook

In order; each is its own merge.

1. The host itself (a wasmtime engine, the I/O buffer ABI, fuel, the fault path),
   with no hook wired: [determinism-and-state.md](determinism-and-state.md#dependency-rules).
2. `hud.readout` and `hud.widget` (phase 1, no simulation).
3. `craft.controls` (a) with Pilot Assist on Pulse and Pure as its oracle, and the
   mod blob in the race state and the hash (phase 2).
4. `pad.enter` and `craft.environment` (the craft-specialty example).
5. Rules as data in `oag-race`, then `race.lap`, `race.finish_rule`,
   `race.tick_end`.
6. Engine changes, each with its own hash-unchanged proof for vanilla races: track
   volumes plus `volume.enter`; a gravity direction; `Weapon::Mod`; the
   `Controller` trait and AI-slot `craft.controls`; authored spawn points and
   `race.spawn`.
