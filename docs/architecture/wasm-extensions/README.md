# WebAssembly extensions: who may extend what, through which surface, and what stays closed

**Status: a design, nothing built** (2026-10-29). The decision record is
[ADR-0079](../adr/0079-extensions-are-sandboxed-webassembly-guests-behind-two-worlds.md),
status *Proposed*. This page is the shape of the API: the extension kinds, how
each is declared, the interface sketch (the `sim` and `hud` worlds, their events
and commands), the explicit non-capabilities, limits, trust, targets, licences,
and the maintainer's examples walked end to end. The engine half is beside it:
[hooks.md](hooks.md) (where each event is raised and each command lands, at its
`file:line`, and where the one guest call per tick sits) and
[determinism-and-state.md](determinism-and-state.md) (floats, fuel, the state
blob, the hash, replays, network play, web and iOS, the spike's measurements).
Where this page and those two disagree, they own the engine facts and this page
is to be corrected.

## What is already decided, and not reopened here

Quoted from the handover thread on custom race modes and the ADRs it cites; the
page builds on them rather than re-deriving them.

- **Custom race modes are implemented through mods, potentially WebAssembly**
  (maintainer, 2026-10-12). Rules a mod can say as data come first (`modes/*.toml`);
  the WebAssembly hook is for what data cannot express.
- **Two phases, then a third.** Phase 1 runs modules only where nothing reaches a
  race's state (HUD widgets). Phase 2 opens the simulation hook; a mod that touches
  the simulation declares `simulation = true` and marks the session **MODDED**: no
  multiplayer, no ghosts or replays recorded or played back, records in a separate
  modded table, MODDED on the HUD and the MODS page. Phase 3 lifts that once module
  hashes travel in the replay header and the lobby and the cross-engine conformance
  test holds.
- **Engine choice**: wasmtime with Cranelift on Linux, Windows, macOS and Android;
  wasmtime's Pulley interpreter on iOS (no JIT); the browser's own `WebAssembly` on
  the web build. Three engines must give bit-identical results: a deterministic
  subset checked at load (no relaxed SIMD, no threads), a NaN rule, one host trait
  with three backends, a conformance test.
- **Phase 2's first user is Pilot Assist** on Pulse and Pure, with bit-identical
  replay hashes against the built-in Rust law as the oracle.
- **The original campaigns stay vanilla.** A custom campaign
  ([ADR-0077](../adr/0077-a-custom-campaign-compiles-to-the-titles-own-grid.md)) is
  where custom modes, tracks, craft and controllers belong.
- **A mod supports only the titles it declares** (`titles = [...]` in `oag-mod.toml`).
- **A track file carries no trigger events in 1.x**; behaviour that reacts to the
  race is a mod behaviour, the WebAssembly rules host
  ([ADR-0076](../adr/0076-a-track-file-carries-its-art-as-an-optional-section.md)
  decision 11).
- **Mods may change anything, simulation and race modes included**; the server
  advertises its content fingerprint and the client shows it on join; mod code
  arriving through a lobby is never auto-run (`docs/networking/README.md`,
  decision 6). Online play is server-authoritative; a LAN host is trusted
  (decisions 2 and 3).
- **Modern saves**: progress persists automatically, no save slots.
- **Pre-release**: a wire or format change edits v1 in place; no migration paths.

The maintainer's ask this page answers (2026-10-29, verbatim intent): a wasmtime
extension design where custom crafts, custom tracks and mods define their own
behaviour, inspired by 2048's prototype craft with individual specialties (Feisar
gets faster on each speed pad); custom tracks that trigger or add physics (spring
jumps, a gravity switch to the underside), custom track rules (A to B instead of a
circuit), custom weapons, custom or extended HUDs. Output: where the engine is
hooked, what is exposed, what is not.

## 1. Who extends what

Five extension kinds, two worlds. A **world** is the set of functions a guest
module exports and the host functions it may import. The **sim world** is
deterministic and is called once per tick inside the race tick; the **HUD world**
runs on the render side and can reach nothing the simulation hashes.

| Kind | Bound to | World | Declared in | Runs when |
| --- | --- | --- | --- | --- |
| **Craft** | a team identity (`oag_title::team::TeamIdentity::identity`, the shared lower-case id, `crates/title/src/team.rs:18-27`), optionally a variant (2048's `feisar2048\4` is the prototype, `oag_2048::race::SHIP_TYPES`) | sim | `oag-mod.toml` `[[extension.craft]]` | every grid slot flying that hull, a person's or the AI's alike |
| **Track** | one track file ([ADR-0063](../adr/0063-the-own-track-format-is-glb-plus-a-toml-manifest.md)) | sim | the track manifest, `track.extension` (a new optional key of `schemas/v1/track.tosd`) | a race on that track |
| **Rules / race mode** | a mode the mod declares (`modes/*.toml`, the rules-as-data step the thread puts first) | sim | the mode file, `extension = "..."` | a race in that mode |
| **Weapon** | a pickup id the mod declares (a custom arm of `oag_tables::weapons::Weapon`, an engine change) | sim | `oag-mod.toml` `[[extension.weapon]]` | a race with weapons on, on the titles the mod is for |
| **HUD** | a widget box in a HUD layout patch (`hud/*.toml`, the thread's section 1c) | hud | `oag-mod.toml` `[[extension.hud]]` | the HUD of a race the mod is on for |

A **pilot controller** (a module flying an AI slot, `controller = "mod:<name>"` in
a pilot file) is the pilot-controller thread's ADR; it would share this host, this
observation and these limits, and is not one of the five kinds here.

### 1.1 Declaring an extension

A mod is a folder, `.zip` or `.oagmod` with `oag-mod.toml` at its root
([ADR-0066](../adr/0066-the-menu-is-the-disc-plus-mods.md) decision 6,
`crates/mod/src/manifest.rs`, `schemas/v1/mod.tosd`). The manifest gains one
`[extension]` table. **This edits `v1/mod.tosd` in place** (pre-release, no
migration): every key below is optional, so an existing manifest stays valid.

```toml
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }

[mod]
id = "topaxi.prototypes"
name = "Prototype craft"
version = "0.1.0"
authors = ["A. Modder"]
license = "MIT"
description = "Each team's craft gets one specialty."
titles = ["wipeout-pulse", "wipeout-2048"]
oag = ">=0.1"

[extension]
simulation = true                  # required when any sim-world module is listed;
                                   # a sim module without it is refused by name
api = 1                            # the ABI major the modules were built against
memory_pages = 16                  # the guest's maximum linear memory, 64 KiB pages (ceiling 256)
state_bytes = 256                  # the host-owned state blob (section 2.1), capped by the host

[[extension.craft]]
team = "feisar"                    # TeamIdentity::identity
module = "ext/feisar.wasm"         # a file in the mod, core wasm, sim world
# variant = "prototype"            # optional: one of a title's craft variants only
# titles = ["wipeout-2048"]        # optional: narrows [mod] titles further

[[extension.weapon]]
id = "emp"                         # the pickup id this mod adds
name_string_id = "WEAPON_EMP"      # strings/<language>.toml
icon = "ext/emp.png"               # the HUD icon, the mod's own file
module = "ext/emp.wasm"            # sim world

[[extension.hud]]
id = "lap-delta"
module = "ext/lap_delta.wasm"      # hud world
layout = "hud/lap-delta.toml"      # the widget box, per title layout (section 2.2)
```

A track-bound module is named by the **track manifest**, not by `oag-mod.toml`,
because a track travels on its own (another mod may race it:
`file = "oag.hillside:tracks/hillside.track.toml"`, `docs/tools/mods.md`):

```toml
[track]
id = "spring-loop"
title = "Spring Loop"
glb = "spring-loop.glb"
extension = "spring-loop.wasm"     # beside the manifest; sim world; optional
```

`track.extension` is a minor addition to `schemas/v1/track.tosd` (an optional key;
a reader of today reports it as unread, ADR-0063 decision 3). The volumes such a
module reacts to are a new optional glb node kind, `oag/volumes/<name>`, the same
box shape as a pad (`_OAG_TO_WORLD`, `_OAG_MIN`, `_OAG_MAX`), tested in the pad
sweep after the weapon pads: a generalisation of the pad test, not a second
trigger system ([hooks.md](hooks.md#event-sources), `volume-entered`).

A rules module is named by the mode file the thread's step 2 introduces
(`modes/*.toml`, `schemas/v1/mode.tosd`, not written yet):

```toml
[mode]
id = "a-to-b"
name_string_id = "MODE_A_TO_B"
finish = "distance"                # rules as data: laps | distance | mod
extension = "ext/a_to_b.wasm"      # what data cannot say; optional
```

### 1.2 Load order and conflicts

- **Which mods take part** is the MODS page's business and no other: the search
  path (`oag_mod::discover::mods_search_path`, `crates/mod/src/discover.rs:20`)
  filtered by the disabled set
  ([ADR-0078](../adr/0078-a-mod-is-turned-off-by-id-live-and-the-built-in-never.md);
  `oag_game::mods::Mods::player`, `crates/game/src/mods.rs:456`). Extensions are read
  through that one gate, so a mod that is off contributes none.
- **Modules are called in the order their ids sort** (not search-path order, which
  differs between machines and would desync a lobby); each sees the same observation
  and never another's commands ([hooks.md](hooks.md#the-shape-one-call-per-tick-events-in-commands-out)).
- **A module is compiled at race load, never per tick**, on the load thread the race
  already builds on (`docs/architecture/race-load-transition.md`); the browser
  compiles asynchronously (section 5). The spike compiled a 1.6 KB guest in about
  2 ms ([determinism-and-state.md](determinism-and-state.md#the-spike)).
- **One craft module per hull per race.** Two enabled mods binding the same team
  identity on the same title: the first by id wins, the second is left out **and
  named in the load report** (`mod B: craft extension for "feisar" already provided
  by mod A`). Never stacked: two modules each scaling thrust is a result nobody
  authored.
- **One track module per track file, one rules module per race**; the mode names
  its own. A track module and a rules module may both run in a race (a spring-jump
  track in an A-to-B mode). A track's volumes answer only to the track's own module.
- **Weapon modules stack by pickup id**; a repeated id is the craft rule again.
- **HUD modules stack**; each owns its box, and two widgets on one box draw in id
  order, the later over the earlier.
- **Commands from several modules in one tick** are applied in id order, each
  checked on its own; a command outside its bound is a fault (section 4.2), never a
  silent clamp.
- **A module that fails to compile or validate** leaves its mod out of the race
  with the reason (ADR-0066 decision 8: a broken mod never stops the game). The race
  runs vanilla and says so.
- **The original campaigns load no extension**, whatever is installed
  (settled: the campaigns stay vanilla). Race box, Race Remix, custom campaigns and
  multiplayer (phase 3) do.

## 2. The interface sketch

**The wire ABI is core WebAssembly**: one I/O buffer in the guest's memory, three
exports, one import namespace `oag`. The component model does not run natively in
browsers and three engines must run the same bytes
([determinism-and-state.md](determinism-and-state.md#web-and-ios)). The WIT below
is **documentation of the records and signatures**, written in the component-model
interface language because it is the clearest way to say what crosses the
boundary; the host writes each record into the buffer in a fixed little-endian
layout and reads the reply back.

**One call per tick, events in, commands out**
([hooks.md](hooks.md#the-shape-one-call-per-tick-events-in-commands-out)). The
engine does not call into a guest at every interesting point; it records events
where they happen and calls each sim module once per tick at one fixed phase, C,
after the people's pickups are spent and before any craft steps
(`Race::tick_body`, between `crates/raceplay/src/tick.rs:186` and `:193`;
[hooks.md](hooks.md#the-tick-with-the-call-the-events-and-the-commands-in-it)).
A command lands in the same tick's force law and race rules. **Events raised after
C** (pads, volumes, walls, hits, laps, respawns, eliminations, an AI craft's fire)
reach the guest in the **next** tick's observation: a one-tick latency, identical
on every machine, stated rather than hidden. A law that needs same-tick reaction
is a measured original's and belongs in the engine, not in a mod.

**State is the host's.** A guest runs on a pooled instance that is fresh every
tick, so nothing in its own memory survives a tick boundary; what does is the
module's fixed-size state blob, passed in with the observation and returned with
the commands, kept in the race state and hashed with it
([determinism-and-state.md](determinism-and-state.md#mod-state-is-a-fixed-size-blob-in-the-race-state)).

### 2.1 The sim world

```wit
package oag:ext@1.0.0;

interface types {
    record vec3 { x: f32, y: f32, z: f32 }
    record quat { x: f32, y: f32, z: f32, w: f32 }

    enum controller { local, remote, ai }          // oag_gameplay::Controller
    enum mode { time-trial, speed-lap, zone, single-race, eliminator, custom }
    enum pad-kind { speedup, weapon }
    enum weapon { rocket, missile, quake, cannon, turbo, shield, autopilot,
                  plasma, bomb, mine, leach-beam, repulser, shuriken, disruptor,
                  custom }                          // oag_tables::weapons::Weapon + a custom arm

    /// What a person is holding this tick: oag_gameplay::InputSnapshot
    /// (crates/gameplay/src/input.rs:28), as the tick received it.
    /// Person-flown slots only; an AI slot reads all-zero.
    record input {
        held: u32, pressed: u32, released: u32,
        stick-x: f32, stick-y: f32,
        airbrake-left: f32, airbrake-right: f32,
    }

    /// One grid slot, read-only: copies of fields the renderer already reads.
    record craft {
        slot: u8,
        active: bool,
        controller: controller,
        team: string,                   // TeamIdentity::identity
        variant: option<u8>,
        position: vec3,                 // oag_physics::Body
        orientation: quat,
        linear-velocity: vec3,
        angular-velocity: vec3,
        speed-kmh: f32,
        shield: f32, shield-max: f32,   // ShipState::shield, Dimensions::shield
        craft-state: u8,                // oag_physics::damage::CraftState
        thrust: f32,                    // ShipState::thrust
        grounded: f32,                  // ShipState::grounded (0..1)
        time-airborne: f32,
        pad-timer: f32,                 // ShipState::pad_timer
        turbo-timer: f32,
        mag-lock-blend: f32,            // ShipState::mag_lock_blend (0..1)
        lap: u32, laps-target: option<u32>,
        progress: option<f32>,          // RaceState::progress, along the course
        place: u32,
        finished: bool,
        pickup: option<weapon>,
        pickup-custom: option<string>,  // the id when pickup == custom
        input: input,
    }

    /// A spline sample near a craft, a fixed count ahead of each person-flown
    /// slot, for an assist or a controller.
    record sample { index: u32, position: vec3, tangent: vec3, down: vec3,
                    half-width: f32, racing-line: f32 }

    /// This tick's events, in engine order (hooks.md, "Event sources").
    variant event {
        pad-crossed(pad-crossed),       // the entry edge, raceplay pads.rs:113 and :258
        volume-entered(volume-event),   // a track file's oag/volumes/<name> (engine change)
        volume-left(volume-event),
        wall-contact(slot-event),       // per contact edge
        weapon-fired(weapon-use),       // a person's this tick, an AI craft's a tick later
        weapon-hit(weapon-hit),         // damage already applied
        lap-completed(lap-completed),
        respawned(slot-event),          // raceplay respawn.rs:280
        reset(slot-event),              // raceplay respawn.rs:322
        eliminated(slot-event),
    }
    record pad-crossed { slot: u8, kind: pad-kind, index: u32 }
    record volume-event { slot: u8, volume: u32, name: string }
    record slot-event { slot: u8 }
    record weapon-use { slot: u8, weapon: weapon, custom: option<string> }
    record weapon-hit { victim: u8, source: u8, weapon: weapon, custom: option<string> }
    record lap-completed { slot: u8, lap: u32, lap-ticks: u32 }

    /// What a tick shows the guest. Built by the host once per tick and
    /// handed to every sim module in turn.
    record observation {
        tick: u64,
        race-ticks: u64,                // oag_race::race_clock_ticks
        mode: mode,
        mode-id: option<string>,        // a mod mode's id when mode == custom
        speed-class: string,
        ship-count: u8,
        course-length: f32,
        crafts: list<craft>,            // MAX_SHIPS entries, inactive ones flagged
        ahead: list<list<sample>>,      // per person-flown slot, empty for the rest
        events: list<event>,            // raised since the last call, capped
        state: list<u8>,                // this module's blob, state_bytes long
    }

    /// What a guest may ask for. Every command is checked by the host
    /// (section 2.1, "commands"); an out-of-range one is a fault.
    variant command {
        scale-thrust(scale-thrust),
        impulse(impulse),
        hold-mag-lock(hold-ticks),
        shield-delta(shield-delta),
        give-pickup(give-pickup),
        clear-pickup(u8),
        assist(assist),
        set-finished(u8),
        set-standing-key(standing-key),
        publish(publish),
        message(message),
    }
    record scale-thrust { slot: u8, factor: f32, ticks: u32 }
    record impulse { slot: u8, delta-velocity: vec3 }
    record hold-ticks { slot: u8, ticks: u32 }
    record shield-delta { slot: u8, amount: f32 }
    record give-pickup { slot: u8, weapon: weapon, custom: option<string> }
    /// What Race::pilot_assist_input computes today: the correction the
    /// built-in law would feed Environment::pilot_assist (the phase 2 proof).
    record assist { slot: u8, steer: f32, pitch: f32, lateral: f32 }
    record standing-key { slot: u8, key: f32 }
    record publish { key: string, value: value }
    variant value { number(f32), integer(s32), text(string) }
    record message { slot: u8, string-id: string }

    /// The guest's answer: its commands and the blob to keep. A module that
    /// has nothing to say returns the blob it was given.
    record reply { commands: list<command>, state: list<u8> }

    record setup {
        title: string,                  // the title slug
        track-id: string,
        mode: mode, mode-id: option<string>,
        speed-class: string,
        laps-target: option<u32>,
        ship-count: u8,
        seed: u64,
        crafts: list<craft>,            // the grid as placed, tick 0
        volumes: list<string>,          // the track's volume names, by index
        state: list<u8>,                // zeroed
    }
    record result { finished: list<u8>, places: list<u8>, state: list<u8> }
}

/// The host functions a sim guest may import: the whole `oag` namespace.
interface sim-host {
    /// The next value of this module's own stream: an oag_core::Rng
    /// (xoshiro128**, crates/core/src/rng.rs) seeded from the race seed and the
    /// mod's id, stored beside the blob and hashed with it. A module that
    /// draws nothing never moves World::rng, which AI and pickup rolls read.
    rng-next: func() -> u32;
    /// A log line, rate-limited by the host and tagged with the mod id (proposed).
    log: func(level: u8, text: string);
}

world sim {
    import sim-host;
    use types.{setup, observation, reply, result};

    /// Once, after the grid is placed (Race::start) and before tick 0. Sees
    /// the grid; cannot move it. Returns the initial blob.
    export race-start: func(setup: setup) -> list<u8>;
    /// Once per tick at phase C.
    export tick: func(obs: observation) -> reply;
    /// Once, when the race ends for everyone. Nothing persists past it in v1
    /// (open question 9).
    export race-end: func(result: result);
}
```

**Commands: what each writes, and its bound.** Where each lands and what the host
refuses there is [hooks.md](hooks.md#command-sinks); this table is the API's side.
Every command names a slot the module may touch (a craft module: slots flying its
hull; a track module: any slot, on its own volumes; a rules module: any slot; a
weapon module: the firing slot and the slots its use names). Numbers marked
*proposed* are starting points, chosen, not measured.

| Command | Writes | Bound | Engine change needed |
| --- | --- | --- | --- |
| `scale-thrust{slot, factor, ticks}` | the slot's engine output for `ticks` ticks, through `Ship::pending_thrust_scale` (`crates/gameplay/src/world.rs:204`, consumed by the step at `player_step.rs:231`) into `Environment::thrust_scale` (`crates/physics/src/forces.rs:292`) | `factor` in `[0.25, 2.0]`, `ticks` in `[1, 3600]` (proposed); two live scales on one slot multiply and the product is bounded again | a host-side timer per slot (the field is re-armed every tick by the Leach Beam), and the force law accepting a factor above 1.0 there; a true top-speed scale is one more `Environment` field defaulting to the identity |
| `impulse{slot, delta-velocity}` | `Body::linear_velocity`, at the point between a craft's controls and its step where `oag_physics::rail::apply` already writes a body (`player_step.rs:256`, `crates/physics/src/rail.rs:17`). **Not** through `ShipState::pending_impulse`, which also arms the collision stun | magnitude at most the class's top speed (proposed); one per slot per tick | the velocity write point |
| `hold-mag-lock{slot, ticks}` | the maglock's engage input for the slot (`crates/physics/src/maglock.rs`), so `mag_lock_blend` ramps in as on a magstrip; holds only where a magnetic floor is under the craft | `ticks` in `[1, 600]` (proposed) | none beyond the input |
| `shield-delta{slot, amount}` | `ShipState::shield` through the damage path (`crates/physics/src/damage.rs`), so the shield flash, the absorb window and the wreck threshold behave as a hit does | `amount` in `[-shield-max, +shield-max]`; never above `shield-max` | none |
| `give-pickup{slot, weapon}` / `clear-pickup{slot}` | `Ship::pickup` (`world.rs:114`, `oag_weapons::pickup::Held`); `custom` requires an id this mod declares | refused in a mode with weapons off (`oag_race::Mode`) | a custom arm of `Weapon` for a mod weapon (section 7.5) |
| `assist{slot, ...}` | `Environment::pilot_assist` for a person's slot, before the step: what `Race::pilot_assist_input` (`crates/raceplay/src/pilot_assist.rs`, called at `player_step.rs:219`) computes today | each term in the built-in law's range; a slot the race did not grant an assist to is refused | the command's sink (the phase 2 proof has no other) |
| `set-finished{slot}` | `Standing::finish_tick` for every craft and `RaceState::finished` for a person's slot, at phases 8 and 9 of the same tick; a mode that finishes by mod passes `laps_target: None` to both | once per slot per race; never un-finished | none: `laps_target: None` already means "never ends on its own" (`crates/race/src/state.rs:154`) |
| `set-standing-key{slot, key}` | a per-slot key the ordering `oag_race::places` sorts before progress and time (`standing.rs:322`) | finite `f32` | `places` takes standings and the course only today |
| `publish{key, value}` | an **output**, not state: the module's HUD channel, queued beside `self.sim.cues` and drained by the view half, so a sim-only replay drops it and it never enters the hash | at most 32 keys per mod, keys at most 32 bytes, text at most 64 bytes (proposed) | the channel |
| `message{slot, string-id}` | an output likewise: the HUD message board of a person-flown slot (`self.view.huds[slot].messages`) | one per slot per tick; the id must exist in the mod's strings | the queue |

A reply is capped (64 commands per module per tick, proposed; at most one of a
kind per slot); past the cap the module faults. A non-finite `f32` anywhere in a
reply is a fault ([determinism-and-state.md](determinism-and-state.md#floating-point)).
The events list is capped too (a fixed-size per-tick buffer in the race state,
cleared at C; an overflow drops the newest and is counted, deterministically).

### 2.2 The HUD world

```wit
package oag:ext@1.0.0;

interface hud-types {
    record rect { x: f32, y: f32, w: f32, h: f32 }
    record rgba { r: f32, g: f32, b: f32, a: f32 }
    enum font { hud, hud-small, default }          // oag_hud::Font
    enum align { left, centre, right }
    variant value { number(f32), integer(s32), text(string) }

    /// A read-only copy of oag_hud::Readout (crates/hud/src/lib.rs:555),
    /// the values the title's own HUD draws, plus this mod's published keys.
    record readout {
        speed-kmh: f32,
        shield: f32, shield-max: f32,
        lap: u32, laps: u32,
        place: u32, ships: u32,
        race-ticks: u64, lap-ticks: u64,
        best-lap-ticks: option<u32>,
        lap-splits: list<option<u32>>,
        wrong-way: bool,
        zone: u32, score: s32,
        pickup: option<string>,
        thrust-percent: f32,
        mode: string,
        published: list<tuple<string, value>>,
    }

    /// What a widget draws: a subset of oag_ui::frontend::Draw
    /// (crates/ui/src/frontend/draw.rs: Fill, Sprite, Text), clipped to the box.
    variant draw { fill(fill), sprite(sprite), text(text) }
    record fill { rect: rect, color: rgba }
    record sprite { texture: u32, rect: rect, uv: rect, color: rgba }
    record text { font: font, text: string, x: f32, y: f32, align: align, scale: f32, color: rgba }
}

interface hud-host {
    /// A string from the mod's strings/<language>.toml, or the id itself.
    string: func(id: string) -> string;
    /// A texture handle: the mod's own file named in its [[extension.hud]] row,
    /// or an entry of the title's HUD sheet by name. Registered at load.
    texture: func(name: string) -> option<u32>;
    log: func(level: u8, text: string);
}

world hud {
    import hud-host;
    use hud-types.{readout, rect, draw};

    /// Once per frame the HUD draws, for the view the widget is on. `box` is
    /// the widget's rectangle in the layout's own grid, the space
    /// oag_hud::Sprite::rect is in (crates/hud/src/widget.rs:107-118), inside the
    /// safe area. `alpha` is the frame's interpolation fraction; `dt` the frame's
    /// wall-clock step, for animation only: nothing here reaches the simulation.
    export frame: func(readout: readout, box: rect, alpha: f32, dt: f32) -> list<draw>;
}
```

The host appends a widget's draws to the `oag_hud::Frame` lists the pass already
paints (`crates/hud/src/draw.rs:695-713`), after `draw_list` builds the title's own
(`HudOverlay::draw`, `crates/game/src/hud_overlay.rs:335`), scissored to `box` the
way a split-screen view's span is (`Span::clip`, `draw.rs:718`). The widget's box
comes from the mod's HUD layout patch, a `hud/*.toml` that adds, moves or hides
elements by stable id (the thread's section 1c; `hud.tosd` not written yet) and may
declare a `[[widget]]` with an `id`, a `rect` per title layout and the module that
draws it. On a title the mod is not for, the widget does not exist. The draw list is
capped (64 draws per widget per frame, proposed); video, backdrops, model draws and
anything outside the safe area are refused.

### 2.3 Where the two worlds run

- **The host is its own crate**, proposed `oag-modhost`: the engine and its
  configuration, the I/O buffer ABI, fuel, the fault path, the pooled per-tick
  instance, the load-time checks and the module rewrite. It knows no game, depends
  on wasmtime and `oag-core` (for `Rng`), and joins the determinism scan
  ([determinism-and-state.md](determinism-and-state.md#dependency-rules)).
- **The sim call is made from `oag-raceplay`'s simulation half**, the half
  `Race::tick` lives in (`crates/raceplay/src/tick.rs:48`) and `oag-server` links
  with `view` off. The gameplay crates (`oag-gameplay`, `oag-physics`, `oag-race`,
  `oag-weapons`, `oag-ai`) gain **no dependency** on the host: commands reach them
  as inputs they already take, the way Zone's auto-speed does
  (`Environment::auto_speed`, `forces.rs:260`). `just check-deps` and `just
  test-wasm` stay meaningful.
- **The HUD host is render side**: the `hud` world runs in `oag-game`'s HUD overlay
  with the same crate, without the determinism needs.
- **Behind a cargo feature** (`extensions`), because Cranelift adds about 11.3 MB
  to a binary; `oag-server` links the host (wasmtime draws nothing, rule 6 holds)
  and runs sim modules in phase 3; the web build never links wasmtime (section 5).
- **A race with any sim module is MODDED** (phase 2). With no sim module loaded the
  host writes nothing to the hash, so every golden hash and determinism reference is
  unchanged by construction ([determinism-and-state.md](determinism-and-state.md#the-state-hash)).

## 3. What is not exposed, and why

Each line is a decision, not an omission. A guest asking for one of these fails at
load (an import outside `oag`) or faults at the command check.

| Not exposed | Why |
| --- | --- |
| **No WASI, no files, no network, no environment, no clock, no threads.** The linker holds one import namespace, `oag`, with the functions of `sim-host` or `hud-host` and nothing else | Determinism: a clock or a file is an input the replay does not carry (`docs/architecture/determinism.md`). Safety: a mod is read in place from an untrusted archive; it gets no reach past its own files. The browser cannot grant them anyway ([determinism-and-state.md](determinism-and-state.md#no-threads-no-clock-no-wasi)) |
| **No state in the guest's own memory across a tick.** One pooled instance per module per tick; the blob is the only thing that crosses a tick boundary | Network rollback restores the race state, not a guest's linear memory; the spike measured a guest with a counter in its own memory **diverging after a rollback** with a persistent instance ([determinism-and-state.md](determinism-and-state.md#mod-state-is-a-fixed-size-blob-in-the-race-state)). Enforced by construction rather than by asking authors to behave |
| **No raw `World`, `Ship` or `Body` pointer, no shared memory with the host, no reading what the observation did not copy** (another module's blob, the game's `Rng` state, a projectile's internals) | `World` is plain data whose layout changes with nearly every simulation commit ([ADR-0003](../adr/0003-no-ecs.md), [ADR-0055](../adr/0055-replays-are-inputs-and-a-ghost-is-poses.md)). A guest reading it by offset breaks on the next field; a guest writing it bypasses every check. Copies in, commands out |
| **No force, torque or position write; a velocity write only through the capped `impulse`**; no replacing the integrator | The force law is the measured original; a guest scales its inputs (`scale-thrust`), it does not step the body |
| **No same-tick reaction to an event raised after the call** | The one-tick latency is the price of one call per tick (section 2); a law that needs same-tick reaction is a measured original's and belongs in the engine |
| **No replacing a recovered law or a disc table**: not an original weapon's law, not a weapon pad's grant, not a handling value, not a pad volume or the spline | Never invent what the assets author (CLAUDE.md). A craft specialty is commands against the disc's own numbers, never a replacement `Handling`; a track's volumes are authored in the track file. A one-shot table rewrite is also the first thing a cheat mod would do |
| **No renderer, GPU, window, camera, draw or sound from the sim world** | The simulation must not know a renderer exists (CLAUDE.md, dependency rule 1). `message` and `publish` are outputs queued beside the cues ([ADR-0018](../adr/0018-audio-mixer-architecture.md)) and drained by the view; a mod-provided sound is open question 6 |
| **No input devices; only the `InputSnapshot` the tick already received, for person-flown slots** | The snapshot is the one thing a replay and a network peer carry (ADR-0055, [ADR-0070](../adr/0070-the-network-transport-is-webtransport.md) decision 4). Reading a pad directly would be an un-replayed input |
| **No reading or writing another module's blob, commands or published keys** | One instance and one blob per module; composition between mods is open question 8 and must not happen by accident |
| **No persistence between races in v1** (a host-mediated save of the blob, at most 4 KiB per mod and title, written through `records::save_in_background` like every other save, is open question 9) | Modern saves: no slots, no files a player manages; a mod must not leave state on disk the MODS page does not know about. Turning a mod off would hide its blob; uninstalling would delete it |
| **No change to an original title's faithful modes unless the player opts into the mod** | The MODS page's ENABLED switch is the opt-in (ADR-0078). With a sim module on, the session is MODDED (phase 2): records go to the modded table, ghosts and replays are off, the HUD says so. The original campaigns never load an extension (settled) |
| **No projectile of the mod's own in v1** | `World::projectiles` is a fixed-size pool of plain `Copy` data (`crates/gameplay/src/world.rs:378`); the Quake, Leach Beam and Repulser each have their own single-slot pool. A guest-defined projectile would change the snapshot's size and every golden hash. A custom weapon is a timed self-effect first (section 7.5); a projectile pool for mods, with a visual from the mod's files, is open question 4 |
| **No menu rows beyond ADR-0066's closed vocabulary, no settings, no pages** | A mod cannot invent front-end behaviour (ADR-0066 decision 4); the extension kinds attach to a race, not to the menus |
| **No AI advantage.** A craft module runs for every slot flying its hull, the AI's included; `controller` is readable so a rules mod can handicap, but nothing in `oag-extras` ever favours the AI | The AI obeys the player's physics (maintainer's rule). A third-party mod that does otherwise is the player's choice and is MODDED |
| **No unbounded work**: fuel per call, a declared memory maximum, a call-depth cap, a command cap, an event cap, string caps | A tick has a budget on a Steam Deck and a phone; a guest that loops forever must be a named, deterministic fault, not a hang ([determinism-and-state.md](determinism-and-state.md#fuel-not-time)) |
| **No floating-point escape hatches**: no relaxed SIMD, no SIMD in v1, no threads, no shared memory, NaN canonicalised, a NaN in any reply a fault | The three-engine rule ([determinism-and-state.md](determinism-and-state.md#floating-point)) |
| **No access to the disc's archives, other mods' files or the asset layers** | A mod's module reads its own files, registered at load (`hud-host.texture`); the HUD sheet by name is the one exception, read-only. The disc is the player's; a mod never streams it |
| **No spawning or removing craft, no grid changes** | `MAX_SHIPS` is a real bound and the grid is placed once (`Race::start`); `race-start` sees the grid and cannot move it. Authored spawn points for an arena are an engine change in the track file first |

## 4. Sandboxing, limits, failure, versioning, trust

### 4.1 Engine configuration and the load-time rewrite

wasmtime 50 (the version the spike pinned; `just test-wasm` runs the simulation
suites under wasmtime already, `docs/tools/web.md`), configured so everything a
guest can observe is pinned. The browser has neither fuel nor NaN canonicalisation,
so both also run as **a rewrite of the module at load**, applied on every engine,
with wasmtime's own switches left on as a second line
([determinism-and-state.md](determinism-and-state.md#fuel-not-time)).

| Setting | Value | Why |
| --- | --- | --- |
| Fuel | on; a budget per call, set before `tick` (`Store::set_fuel`); plus the rewrite's own counter at function entries and loop headers, trapping at zero, identical on every engine; wasmtime's fuel a higher backstop | the only portable work bound; a wall-clock epoch traps at different instructions on different machines. The budget bounds loops and calls, not straight-line length (measured). The number is set with the first real hook; the spike's toy used about 168 units a tick |
| NaN canonicalisation | `cranelift_nan_canonicalization(true)`, plus the rewrite inserting the same canonicalisation after every float op | measured: on, every arithmetic NaN read `7fc00000` on both engines; off, `ffc00000` with a payload |
| Relaxed SIMD, threads, shared memory, memory64, SIMD | off / refused at load | the deterministic subset; `simd128` only after the conformance test passes with it |
| Memory | the manifest's `memory_pages`, the same `StoreLimits` on every engine, ceiling 256 pages (16 MiB, proposed) | `memory.grow` fails at the same size everywhere |
| Call depth | the rewrite's counter, trapping at a fixed depth well below any engine's native stack | a native stack overflow traps at different depths on Cranelift and Pulley; open risk until the pass exists |
| Instance | one pooled instance per module per tick (copy-on-write reset) | rollback safety; 11.4 us (Cranelift) / 27.3 us (Pulley) a tick for 20 calls against a 60 us race tick; one call per tick here |
| State blob | `state_bytes` from the manifest, capped by the host (a few hundred bytes per slot plus a global part covers every example here; the cap is set with the first sim hook) | a fixed-size field in the race state, no allocation |
| Module size | at most 4 MiB on disk (proposed) | a mod is small; a bigger module is a mistake |
| Compilation cache, parallel compilation | off | the host writes nothing a mod asked for; no thread pool in the simulation half |
| Features `wasi`, `async`, `profiling`, `coredump`, `addr2line`, `debug-builtins`, `component-model`, `threads` | off | not needed; `wasi` is the whole non-capability list; the ABI is core wasm |

### 4.2 Faults

- **One answer for every failure** ([hooks.md](hooks.md#the-shape-one-call-per-tick-events-in-commands-out)):
  a trap (out of fuel, `unreachable`, out-of-bounds memory, stack overflow) or a
  refused command **turns the module off for the rest of the race**. That call's
  commands are discarded, the blob keeps its value from before the call, every
  later call is skipped. The race goes on: a player two laps in keeps the race; the
  author gets the tick.
- **The fault edge is simulation state**: the fault flag and the tick are hashed;
  the fuel total is not (its per-operator costs are wasmtime's own and the browser
  has none). Same inputs, same fault tick on every machine: the spike's runaway loop
  trapped at the same iteration on Cranelift and Pulley.
- **Shown, not only logged**: the HUD's MODDED tag and the results screen name the
  mod and the tick.
- **A HUD module that faults** is hidden for the rest of the race; the title's own
  HUD is untouched.
- **A module that will not validate at load** (an import outside `oag`, a missing
  export, a refused feature, another `api` major) leaves its mod out of the race,
  named in the load report; the race runs vanilla.

### 4.3 Versioning

- The ABI is **major 1, edited in place until the first release** (pre-release
  rule: no migration code paths). A module built against a record layout the host no
  longer writes fails at load by name; that is the whole compatibility story before
  1.0. The manifest's `api` names the major.
- After the first release: a new field at the end of a record, a new event variant
  or a new command is a compatible addition inside major 1 (a guest reads the prefix
  it knows and ignores an event it does not); a change to an existing field's
  meaning or position is major 2, a second layout the host writes beside the first
  for a release.
- The replay header's `mods` entry carries the mod's blob layout version, so a
  replay refuses a mod build whose blob layout changed.

### 4.4 Trust

- **Online play is server-authoritative** (networking decision 2): the server runs
  the same modules, accepts inputs only, and its hash per tick is the truth (ADR-0070
  decision 4). A client with a different module mispredicts and is corrected; it
  cannot cheat the result.
- **A LAN host is trusted** (decision 3); the joiner sees the content fingerprint
  and joins or not.
- **The lobby already refuses differing mods**: `Edition::content`
  (`crates/net/src/wire/online.rs:34`) and `content_refusal`
  (`crates/net/src/online/rooms.rs:288`, "your mods differ from the server's"), over
  `oag_net::discovery::content_fingerprint` (`crates/net/src/discovery.rs:168`),
  FNV-1a of `id@version` of the mods loaded (`crates/game/src/mods.rs:446`). **The
  gap**: two builds of one mod version would be admitted together and desync. Phase
  3 folds each sim module's SHA-256 into that fingerprint
  ([determinism-and-state.md](determinism-and-state.md#network-play)). Mod code
  arriving through a lobby is never auto-run (decision 6): the joiner must have the
  mod installed.
- **Replays**: `oag_replay::Header` (`crates/replay/src/header.rs`) has no mod field
  today and `build` is informational, never compared. Proposed: a dedicated `mods`
  list (id, version, module SHA-256, blob layout version), not the opaque `options`
  map, editing format v1 in place. Playback refuses a replay whose modules are not
  present with those hashes, by name; the stored state hashes still decide validity
  (ADR-0055). In phase 2 a modded session records no replay at all
  ([determinism-and-state.md](determinism-and-state.md#replays)).
- **Records**: a modded session's results go to the separate modded table (phase 2),
  keyed with the fingerprint; the networking page already says a modified-sim run
  goes to a separate board tagged with its fingerprint.
- **Rollback cost is the guest's cost times the window**: at most `MAX_WINDOW = 30`
  ticks, typically 7 to 11; estimated +19 % a tick on Cranelift and +45 % on Pulley
  with 20 guest calls a tick (desktop; a phone is open). One call per module per
  tick here keeps it at the low end.

## 5. Targets: the same bits everywhere

| Target | Engine | How the module gets in | Status |
| --- | --- | --- | --- |
| Linux, Windows, macOS | wasmtime 50, Cranelift | compiled at race load on the load thread | spike: Cranelift and Pulley bit-identical over a 36,000-tick race, fuel trap points included; the native Rust oracle of the same law gives the same hash ([determinism-and-state.md](determinism-and-state.md#the-spike)) |
| Android | wasmtime, Cranelift (aarch64) | same | same host code; the mobile glue is `oag-mobile` ([ADR-0065](../adr/0065-one-mobile-crate-for-every-phone-os.md)). **Owed**: the spike on an AArch64 machine, where the default NaN differs; the x86 run says nothing about it |
| iOS | wasmtime, Cranelift compiling to **Pulley** bytecode on the device (no native code), +11.4 MB; the runtime-and-Pulley-only build (+0.8 MB) needs precompiled `.cwasm` per wasmtime build, brittle | same | **not checked on iOS**: `cargo check --target aarch64-apple-ios` of wasmtime 50 stops in a build script needing Apple's `xcrun`, the macOS-only toolchain `just ios-app` needs; whether `signals_based_traps` must be off there is owed to a macOS machine ([determinism-and-state.md](determinism-and-state.md#web-and-ios)) |
| Web | the browser's own `WebAssembly`, through `js-sys`; **the web build never links wasmtime** | compiled asynchronously at race load, never per tick (browsers limit synchronous compiles on the main thread); the race load already runs on a Web Worker (`docs/tools/web.md`, "Threads"); per-tick calls copy the observation into the module's memory as on native | needs the load-time rewrite (fuel, NaN canonicalisation, call depth) before it can match; not run yet |

**Same bits everywhere means the guest's results, not the host's speed.** The
engines agree because the module is the same bytes after the rewrite, the imports
are the same two functions, the limits are the same numbers and the fault edge is
hashed. The simulation itself is already proven deterministic on `wasm32` (`just
test-wasm`, debug and release, 2026-10-09), so the open proof is the nested guest
in the browser and on AArch64, which the conformance test covers
([determinism-and-state.md](determinism-and-state.md#the-conformance-test)).

## 6. Licences

The workspace is `MIT OR Apache-2.0` (`Cargo.toml` `[workspace.package]`,
`LICENSE-MIT`, `LICENSE-APACHE`). There is no `deny.toml`; the policy in practice is
permissive dependencies only, and the lock file already carries `Apache-2.0 WITH
LLVM-exception` (`rustix`, `target-lexicon` under wgpu and quinn), `ISC` (`ring`) and
one weak copyleft, `MPL-2.0` (`symphonia`, file-level, in `oag-music`). Every proposed
dependency fits inside that:

| Crate | Licence | Role | Note |
| --- | --- | --- | --- |
| `wasmtime` 50 (with `cranelift`, `pulley`, `runtime`, `pooling-allocator`; without `wasi`, `cache`, `parallel-compilation`, `async`, `profiling`, `coredump`, `addr2line`, `debug-builtins`, `component-model`, `threads`) | Apache-2.0 WITH LLVM-exception (read off the crate's own `Cargo.toml` in the spike) | the engine | the LLVM exception only loosens Apache's attribution clause for binaries; compatible with both halves of ours |
| `cranelift-codegen` 0.137 and family, `pulley-interpreter` 50 | Apache-2.0 WITH LLVM-exception | wasmtime's backends | transitive |
| `wasmparser`, `wasm-encoder` | Apache-2.0 WITH LLVM-exception | the load-time rewrite (fuel counter, NaN canonicalisation, call depth) and the feature check before compiling | the same bytes go to wasmtime and to the browser |
| `wasmtime-wasi` | Apache-2.0 WITH LLVM-exception | **not used** | listed to say so: no WASI |
| `wit-bindgen`, `wasm-tools` | Apache-2.0 WITH LLVM-exception | **not a runtime dependency**: the WIT on this page is documentation; guests are plain `wasm32-unknown-unknown` Rust over the I/O buffer, as the spike's is | an example mod may generate its bindings with them at its own build time |
| `js-sys`, `web-sys`, `wasm-bindgen` | MIT OR Apache-2.0 | the web host | already in the tree for the web build (`crates/game/Cargo.toml`) |
| `sha2` | MIT OR Apache-2.0 | module hashes in the fingerprint and the replay header | or `ring`'s digest, already linked by `oag-net` |

Two wasmtime optional dependencies to keep **off**: `ittapi` (BSD-3-Clause OR GPL-2.0,
the VTune profiler hook, behind `profiling`) and anything behind `cache` (it writes
under the user's cache directory). A `deny.toml` allowlist (MIT, Apache-2.0 with and
without the LLVM exception, BSD-2/3, ISC, Zlib, Unicode, MPL-2.0 for the existing
case) is proposed as part of phase 1 so this table stays true without re-reading the
lock file (open question 7).

## 7. The maintainer's examples, end to end

Each walk names the extension kind, the manifest row, the events read, the
commands returned, what the host does with them, and what a player sees.
[hooks.md](hooks.md#the-maintainers-examples-mapped) says which of these the engine
can do today and which need an engine change first.

### 7.1 Feisar gets faster on each speed pad

The 2048 prototype idea: each team's fourth craft has a specialty
(`oag_2048::race::SHIP_TYPES`, `"prototype"` the fourth; `docs/formats/2048-campaign.md`
"A prototype craft is not a fifth, independent class"). **The specialties themselves
are not recovered** from 2048; on 2048 and Omega they are the original's laws and
belong to the engine once read, not to a mod, and a one-tick-late mod law would not
match them ([hooks.md](hooks.md#the-2048-prototype-craft)). A mod is how such a
specialty reaches a title that never had it, which is this walk. **Nearly on
today's engine**: `pending_thrust_scale` exists; the per-slot timer and a factor
above 1.0 are the engine change.

1. **Declaration**: `[[extension.craft]] team = "feisar", module = "ext/feisar.wasm"`,
   `state_bytes = 32` (a `u32` per slot), `simulation = true`.
2. **Load**: the race is on Pulse with a slot flying Feisar (or, in Race Remix, any
   title the mod is for); the module compiles on the load thread; `race-start` gets
   the grid and returns a zeroed blob. The session is MODDED.
3. **Tick N**: the hull enters a speed pad; the host raises `pad-crossed{slot,
   speedup, index}` at the entry edge (`Race::test_speedup_pads`,
   `crates/raceplay/src/pads.rs:73`, the edge at `:113`, for a person's craft and an
   AI's alike) into the event buffer.
4. **Tick N+1, the call**: the guest finds the event, increments the slot's counter
   in the blob, and returns `scale-thrust{slot, factor: min(1.0 + 0.02 * n, 1.5),
   ticks: 3600}` with the blob.
5. **Host**: checks the factor against the bound, starts the slot's timer, and each
   tick while it runs multiplies the factor into `pending_thrust_scale`
   (`world.rs:204`), which the step consumes (`player_step.rs:231`) into
   `Environment::thrust_scale` (`forces.rs:292`). The AI slot flying the same hull
   gets the same specialty: the AI obeys the player's physics.
6. **Player sees**: the pad boost as always, then a hull that keeps more speed on the
   straights from the next tick; the speed bar, which reads `speed_kmh` from the
   body's velocity (`crates/raceplay/src/telemetry.rs:137`), shows it with no HUD
   change. `publish{"feisar.pads", n}` lets a HUD module (7.6) show the counter.
7. **Replay and records**: phase 2, none recorded; MODDED on the results. Phase 3: the
   module's hash in the header and the lobby.

### 7.2 A spring jump on a custom track

**Needs the engine change** in [hooks.md](hooks.md#event-sources): the volume node
kind and the velocity write point.

1. **Track**: the author tags a Blender object as a volume; the add-on writes
   `oag/volumes/spring-a` into the glb (the pad's box shape), and the manifest names
   `extension = "spring-loop.wasm"`.
2. **Load**: `setup.volumes = ["spring-a"]`. A build without the `extensions`
   feature reports the node unread and races the track flat.
3. **Tick N**: the host tests every active hull against the volumes in the pad sweep
   (`Race::pad_sweep`, `pads.rs:45`), after the weapon pads, and raises
   `volume-entered{slot, 0, "spring-a"}` on the entry edge.
4. **Tick N+1, the call**: the guest returns `impulse{slot, delta-velocity: up *
   18.0 + forward * 4.0}` (its own numbers).
5. **Host**: refuses a non-finite or over-cap delta (a fault), else adds it to the
   body between the craft's controls and its step, where `oag_physics::rail::apply`
   already writes one (`player_step.rs:256`), not through `pending_impulse` (that
   port arms the collision stun). The craft leaves the floor one tick after the
   volume's edge, a 17 ms offset the author places the volume to absorb; `grounded`
   falls, `time_airborne` runs, the airborne pitch law and the landing behave as
   after a disc jump (the engine already has `Handling::speedpad_jump` and
   `turbo_jump` as upward kicks, `crates/physics/src/params.rs:346-349`). The
   off-track rescue still applies: a jump into the void respawns as any other.
6. **Player sees**: the jump, with the camera and shake the engine already does on
   landing. No sound in v1 (open question 6).

### 7.3 A gravity switch to the underside

Two facts first. **Gravity in this engine acts on world `-Y` only**
(`oag_physics::passive::gravity`, `crates/physics/src/passive.rs:309`;
`crates/physics/src/forces.rs:221-222`: "Gravity still acts on world `.y` only;
neither is a gravity direction"), and **the original's way of racing an inverted
section is the magstrip**, a kinematic attitude hold that fades the hover out and
slaves the hull to the surface normal (`crates/physics/src/maglock.rs`, confidence 90
off `Ship_UpdateMagLock`; `docs/physics/cornering-ground-truth.md` measures a Talon's
Junction inverted section). So there are two routes, and the walk takes both:

1. **The faithful route, no code**: the underside is authored as `mag_floor`
   collision (`oag_kind = "mag_floor"`, `docs/formats/track-file.md`), exactly as a
   disc circuit authors a magstrip. A craft that drives onto it is held through the
   inversion; the hover already casts along the craft's own `-up`
   (`crates/physics/src/hover.rs:610`). This works on a track file today and is what
   a continuous top-to-underside loop should be built from.
2. **The switch moment** is what the extension adds today: a volume at the lip
   (`volume-entered`), and the guest returns `hold-mag-lock{slot, ticks: 120}` so the
   hold engages before the hull has a `mag_floor` sample under it (the blend ramps at
   `0.2` a frame, `maglock.rs`), and, the author choosing, an `impulse` toward the
   surface. The hold releases when `ticks` run out and no magnetic floor is under
   the craft.
3. **The switch the maintainer described** (gravity itself turns, and the craft
   falls to the underside) is **an engine change**, mapped in
   [hooks.md](hooks.md#the-maintainers-examples-mapped): a gravity direction in
   `Environment` defaulting to `-Y` (bit-identical for every race that does not set
   it), per-craft gravity state on `Ship`, and a `set-gravity{slot, direction}`
   command from the same event with a unit direction. The craft side then follows
   (hover and maglock already work in the craft's own frame); the camera and the
   AI's racing line do not, and need their own check before the command is offered
   (open question 3).
4. **Player sees**: routes 1 and 2 today, the hull rolling onto the underside with
   the maglock's own attitude blend; route 3 after the engine change.

### 7.4 A to B instead of a circuit

The thread's order stands: rules as data first, open splines later. The extension's
part is the piece data cannot say. **Partly on today's engine**: a sprint over part
of a closed circuit needs only `set-finished`; a truly open track does not fit the
loop every lap structure assumes ([hooks.md](hooks.md#the-maintainers-examples-mapped)).

1. **Mode**: `modes/a-to-b.toml` with `finish = "mod"` (the rules-as-data step; it
   passes `laps_target: None` to `RaceState` and `Standing`, which already means
   "never ends on its own", `crates/race/src/state.rs:154`) and a grid at a sample
   other than the lap line. A plain `finish = "distance"` needs no module at all.
2. **What the module adds**: the finish itself, checkpoints with time bonuses, or
   "last place every 30 s is out". Each tick the guest reads every slot's `progress`
   and `race-ticks`; it returns `set-finished{slot}` when a slot's progress first
   reaches the finish sample (reading `progress` before the course wraps it,
   `wrapped_forward`, `state.rs:35`), `set-standing-key{slot, key}` for a ranking of
   its own, and `publish{"a_to_b.remaining_m", f32}` for the HUD.
3. **Host**: `set-finished` sets `Standing::finish_tick` for every craft and
   `RaceState::finished` for a person's slot at phases 8 and 9 of the same tick,
   once; `set-standing-key` feeds the key `oag_race::places` sorts by (an engine
   change: `places` takes standings and the course only today). The results screen,
   the records key per mode and the replay header's mode are the mode file's.
4. **An open point-to-point track** stays "later" (the spline, the spawner, the
   rescue, the AI plan and the flyby all assume a loop); the ABI does not change when
   it lands, only the track format does.

### 7.5 A custom weapon

**Needs the engine change** in [hooks.md](hooks.md#command-sinks):
`oag_tables::weapons::Weapon` (`crates/tables/src/weapons.rs:91`) is a closed enum of
the titles' weapons, so a mod weapon needs a custom arm, a pickup icon from the mod's
file, a row in `docs/gameplay/weapon-lineage.md` (a new pickup is a lineage row, the
member rule), and the network weapons path of
[ADR-0071](../adr/0071-weapons-are-predicted-simulation-with-a-discrete-state-correction-trigger.md)
treating it like any other. The pad roll (`oag_weapons::pickup::Driver`,
`crates/weapons/src/pickup.rs:116`) gains the mod's ids with the mod's weight in the
modes the mod is on. **Timed self-effects first** (a Turbo-like boost, a shield),
the easy case; a projectile of the mod's own is open question 4.

1. **Declaration**: `[[extension.weapon]] id = "emp", icon = "ext/emp.png",
   name_string_id = "WEAPON_EMP", module = "ext/emp.wasm"`.
2. **Tick N**: a person spends the pickup (`Race::spend_pickup`,
   `crates/raceplay/src/weapons.rs:148`, at `tick.rs:168`, before the call), so
   `weapon-fired{slot, custom, "emp"}` is in **this** tick's observation; an AI
   craft's fire (`spend_opponent_pickup`) happens after the call and arrives a tick
   later.
3. **The call**: the guest reads `obs.crafts`, picks every other craft within 30
   units, and returns `scale-thrust{target, factor: 0.5, ticks: 120}` and
   `shield-delta{target, amount: -5.0}` for each, `message{slot, "EMP_HIT"}` for the
   person-flown ones, and `clear-pickup{slot}`.
4. **Host**: checks and applies before the step; the damage path raises the shield
   flash and the absorb window as a hit does, so the victim's HUD reacts; the
   victims' modules see `weapon-hit` only if the host raises it for a scripted hit
   (open question 5).
5. **What a scripted weapon cannot do**: fly a projectile, draw a beam, play the
   disc's weapon effects. Those are `oag_fx` visuals bound to the fixed kinds. An
   **AI craft holding a mod weapon** is open in [hooks.md](hooks.md#event-sources):
   our driver's weapon decision knows the titles' weapons only.

### 7.6 A HUD extension

**Mostly on today's engine**: needs the HUD module host and the published-keys
channel; the HUD layout files are planned separately.

1. **Declaration**: `[[extension.hud]] id = "lap-delta", module = "ext/lap_delta.wasm",
   layout = "hud/lap-delta.toml"`; the layout patch declares `[[widget]] id =
   "lap-delta", rect = { pulse = [300, 20, 160, 60], hd = [1480, 60, 400, 150] }`
   in each title layout's own grid, inside the safe area.
2. **Each frame**: `frame(readout, box, alpha, dt)` gets the slot's `Readout` copy
   (`lap-splits`, `best-lap-ticks`, `lap-ticks`) and this mod's published keys; it
   returns `fill` and `text` draws inside `box` (a bar per completed lap scaled by
   its split against the best). No simulation state is read or written; the widget
   runs in the HUD pass on the render side and is absent from every hash.
3. **Host**: appends the draws to the `Frame`'s lists after the title's own
   (`HudOverlay::draw`, `crates/game/src/hud_overlay.rs:335`), scissored to `box`,
   and the overlay rasterises them as it does every `Draw::Fill` and `Draw::Text`.
4. **Player sees**: the widget where the patch put it, on the titles the mod is for,
   gone when the mod is off. The vanilla layout is the default; a HUD mod is opt-in
   (the thread's 1c.4).

## 8. Open questions

1. **The web proof**: the load-time rewrite (fuel counter, NaN canonicalisation,
   call depth) is designed, not built; the conformance test's headless-browser run
   (`wasm-bindgen-test`) is what shows the browser matches.
2. **The fuel budget** per call, set with the first real hook; and whether the HUD
   world gets fuel or a wall-clock deadline (it is not deterministic, so a deadline
   is allowed there).
3. **A per-craft gravity direction** in `Environment` (7.3): a measured default of
   `-Y` proven bit-identical for every race that does not set it, then the camera and
   the AI line checked before a `set-gravity` command is offered.
4. **A projectile pool for mods**: a second fixed-size pool outside
   `World::projectiles` so the vanilla snapshot's size is unchanged, with a visual
   from the mod's files, or a disc weapon's visuals the mod names without a stand-in
   for what the disc authors.
5. **Event granularity**: `pad-crossed` on entry only (the edge that exists today)
   is settled; `wall-contact` per contact edge; whether a scripted hit
   (`shield-delta` from a mod weapon) raises `weapon-hit` for the victim's modules.
6. **Sound**: a cue command naming a disc cue by id, or a cue from the mod's own WAV;
   both need the audio architecture (ADR-0018) to accept a mod-provided bank.
7. **A `deny.toml`** allowlist for the workspace, so section 6 is checked by the gate.
8. **Composition**: whether a rules module may read a craft module's published keys,
   and in what order.
9. **Persistence between races**: whether the blob is saved per mod and title (the
   proposal in section 3), and what a mod may do with it (an unlock, a best score)
   without a save slot.
10. **The pilot-controller thread's three questions** (may a pilot fly `"original"`
    outside the campaign; mod controllers in multiplayer; WebAssembly as the mod
    ABI): this page assumes yes to the third and leaves the first two to the
    maintainer.
11. **Hot reload** of a module during a race for mod authors (a `--reload-extensions`
    debug flag): useful, and incompatible with the hash; debug builds only, if at all.
12. **iOS and AArch64**: the spike on an AArch64 machine and the wasmtime build on
    iOS (`xcrun`), both owed to hardware this machine does not have.

## 9. Phased rollout

In the thread's order, with the extension kinds slotted in; each step is its own
merge and each engine change carries its own proof that a vanilla race's hashes do
not move ([hooks.md](hooks.md#what-has-to-be-built-in-order)):

1. **The host itself** (`oag-modhost`: the engine configuration of section 4.1, the
   I/O buffer ABI, fuel, the fault path, the pooled per-tick instance, the load-time
   checks), no call site wired, behind the `extensions` feature. The ADR accepted;
   the spike's result on this page.
2. **Phase 1, no simulation code**: the `hud` world, `[[extension.hud]]` and the
   `hud/*.toml` widget box, one example widget (the lap-delta graph) in
   `mod-examples`. Proves the host on low-stakes code.
3. **Phase 2, the simulation call**: call site C, the event buffer, the state blob in
   the race state and the hash, and the `assist` command, proven by moving
   Pulse/Pure's Pilot Assist backport into a module with bit-identical replay hashes
   (`pilot_assist_ground_truth.rs`); `simulation = true` and the MODDED session.
4. **The craft kind**: `scale-thrust` with its timer, `pad-crossed`, then the Feisar
   example (7.1), the first player-facing use.
5. **Rules as data** in `oag-race`, then `set-finished` and `set-standing-key`
   (7.4); the drag race as the first mod mode, no module needed.
6. **Engine changes, each its own merge**: track volumes and `volume-entered`, the
   velocity write for `impulse` (7.2); a gravity direction (7.3); a custom `Weapon`
   arm and `give-pickup` (7.5); authored spawn points; the `Controller` trait for a
   pilot controller (the pilot-controller thread).
7. **Phase 3**: module hashes in the replay header's `mods` list and the lobby
   fingerprint, playback and join refusing a mismatch by name, the conformance test
   across Cranelift, Pulley and the browser, `oag-server` running sim modules;
   MODDED stops meaning "no replay, no multiplayer" and starts meaning "with these
   modules".
