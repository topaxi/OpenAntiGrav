# WebAssembly extensions: who may extend what, through which surface, and what stays closed

**Status: a design, nothing built** (2026-10-29). The decision record is
[ADR-0079](../adr/0079-extensions-are-sandboxed-webassembly-guests-behind-two-worlds.md),
status *Proposed*. This page is the shape of the API: the extension kinds, the
WIT-style interface sketch, the explicit non-capabilities, limits, trust, targets,
licences, and the maintainer's examples walked end to end. The engine half lives
beside it: `hooks.md` (every hook at its `file:line`, tick phase, inputs, outputs
and refusals) and `determinism-and-state.md` (what is hashed, snapshotted and
restored), with a spike under `spikes/wasm-extensions/` proving bit-identical guest
runs across engines. Where this page and `hooks.md` disagree, `hooks.md` owns the
engine facts and this page is to be corrected.

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

Five extension kinds, two worlds. A **world** is a WIT world: the set of functions
the guest exports and the host functions it may import. The sim world is
deterministic and runs inside the race tick; the HUD world runs on the render side
and can reach nothing the simulation hashes.

| Kind | Bound to | World | Declared in | Runs when |
| --- | --- | --- | --- | --- |
| **Craft** | a team identity (`oag_title::team::TeamIdentity::identity`, the shared lower-case id, `crates/title/src/team.rs:18-27`), optionally a variant (2048's `feisar2048\4` is the prototype, `oag_2048::race::SHIP_TYPES`) | sim | `oag-mod.toml` `[[extension.craft]]` | every grid slot flying that hull, a person's or the AI's alike |
| **Track** | one track file ([ADR-0063](../adr/0063-the-own-track-format-is-glb-plus-a-toml-manifest.md)) | sim | the track manifest, `track.extension` (a new optional key of `schemas/v1/track.tosd`) | a race on that track |
| **Rules / race mode** | a mode the mod declares (`modes/*.toml`, the rules-as-data step the thread puts first) | sim | the mode file, `extension = "..."` | a race in that mode |
| **Weapon** | a pickup id the mod declares | sim | `oag-mod.toml` `[[extension.weapon]]` | a race with weapons on, on the titles the mod is for |
| **HUD** | a widget box in a HUD layout patch (`hud/*.toml`, the thread's section 1c) | hud | `oag-mod.toml` `[[extension.hud]]` | the HUD of a race the mod is on for |

### 1.1 Declaring an extension

A mod is a folder, `.zip` or `.oagmod` with `oag-mod.toml` at its root
([ADR-0066](../adr/0066-the-menu-is-the-disc-plus-mods.md) decision 6,
`crates/mod/src/manifest.rs`, `schemas/v1/mod.tosd`). The manifest gains one
`[extension]` table. **This edits `v1/mod.tosd` in place** (pre-release, no
migration): every key below is optional, so an existing manifest stays valid.

```toml
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }

[mod]
id = "topaxi.protoypes"
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
api = "1.0.0"                      # the oag:ext package version the modules were built against

[[extension.craft]]
team = "feisar"                    # TeamIdentity::identity
module = "ext/feisar.wasm"         # a file in the mod, .wasm, sim world
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
layout = "hud/lap-delta.toml"      # the widget box, per title layout (section 2.3)
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
a reader of today reports it as unread, ADR-0063 decision 3). The trigger volumes
such a module reacts to are a new optional glb record, `oag/trigger/<n>` (section
2.1, "events"), added the way `oag/sun` and `oag/art/<n>` were
(`docs/formats/track-file.md`, "Glb").

A rules module is named by the mode file the thread's step 2 introduces
(`modes/*.toml`, `schemas/v1/mode.tosd`, not written yet):

```toml
[mode]
id = "a-to-b"
name_string_id = "MODE_A_TO_B"
finish = "distance"                # rules as data: laps | distance
extension = "ext/a_to_b.wasm"      # what data cannot say; optional
```

### 1.2 Load order and conflicts

- **Mods load in the order the search path already gives them**
  (`oag_mod::discover::mods_search_path`, `crates/mod/src/discover.rs:20`: `$OAG_MODS`
  alone when set, else `data/mods/`, the examples, `mods/` beside the program, the
  data directory), filtered by the MODS page's disabled set
  ([ADR-0078](../adr/0078-a-mod-is-turned-off-by-id-live-and-the-built-in-never.md);
  `oag_game::mods::Mods::player`, `crates/game/src/mods.rs:456`). Extensions are read
  through that one gate and no other, so a mod that is off contributes none.
- **A module is compiled at race load, never per tick**, on the load thread the race
  already builds on (`docs/architecture/race-load-transition.md`), and the browser
  compiles asynchronously (section 5).
- **One craft module per hull per race.** Two enabled mods binding the same team
  identity on the same title: the first in search order wins, the second is left out
  **and named in the load report** (`mod B: craft extension for "feisar" already
  provided by mod A`). Never stacked: two modules each scaling thrust is a result
  nobody authored.
- **One track module per track file, one rules module per race**; the mode names
  its own. A track module and a rules module may both run in a race (a spring-jump
  track in an A-to-B mode).
- **Weapon modules stack by pickup id**; a repeated id is the craft rule again
  (first wins, second named).
- **HUD modules stack**; each owns its box, and two widgets on the same box draw in
  mod order, the later over the earlier.
- **Command order within a tick** is fixed: craft, track, rules, weapon modules, each
  in mod order; each command is clamped on its own (section 2.1) and a command
  refused is named once per race in the log, never silently dropped.
- **An extension that fails to compile or validate** leaves its mod out of the race
  with the reason (the mod rule of ADR-0066 decision 8: a broken mod never stops the
  game). The race runs vanilla and says so.
- **The original campaigns load no extension**, whatever is installed
  (settled: the campaigns stay vanilla). Race box, Race Remix, custom campaigns and
  multiplayer (phase 3) do.

## 2. The interface sketch

Written as WIT (the component-model interface language), package `oag:ext@1.0.0`.
It is a sketch: field names follow the engine's types so `hooks.md` can cite them,
and the final text lands with the phase-1 ADR implementation. Hook names here are
the ones agreed with the engine half; see `hooks.md` for each one's `file:line`.

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
                  custom }                          // oag_tables::weapons::Weapon + custom

    /// What a person is holding this tick: oag_gameplay::InputSnapshot, as the
    /// tick received it. Person-flown slots only; an AI slot reads all-zero.
    record input {
        held: u32, pressed: u32, released: u32,
        stick-x: f32, stick-y: f32,
        airbrake-left: f32, airbrake-right: f32,
    }

    /// One grid slot, read-only. Every field is a copy of engine state the
    /// renderer already reads; nothing here is a handle.
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
        thrust: f32,                    // ShipState::thrust
        grounded: f32,                  // ShipState::grounded (0..1)
        time-airborne: f32,
        pad-timer: f32,                 // ShipState::pad_timer
        turbo-timer: f32,
        mag-lock-blend: f32,            // ShipState::mag_lock_blend (0..1)
        lap: u32, laps-target: option<u32>,
        progress: option<f32>,          // RaceState::progress, along the spline
        place: u32,
        finished: bool,
        pickup: option<weapon>,
        pickup-custom: option<string>,  // the id when pickup == custom
        input: input,
    }

    variant event {
        pad-crossed(pad-crossed),       // raceplay pads.rs: test_speedup_pads / test_weapon_pads
        lap-completed(lap-completed),
        weapon-fired(weapon-use),
        weapon-hit(weapon-hit),
        wall-contact(slot-event),
        respawned(slot-event),          // raceplay respawn.rs: respawn
        reset(slot-event),              // raceplay respawn.rs: reset
        eliminated(slot-event),
        volume-entered(volume-event),   // a track file's oag/trigger/<n>
        volume-left(volume-event),
    }
    record pad-crossed { slot: u8, kind: pad-kind, index: u32 }
    record lap-completed { slot: u8, lap: u32, lap-ticks: u32 }
    record weapon-use { slot: u8, weapon: weapon, custom: option<string> }
    record weapon-hit { victim: u8, source: u8, weapon: weapon, custom: option<string> }
    record slot-event { slot: u8 }
    record volume-event { slot: u8, volume: u32, name: string }

    /// What a tick shows the guest. Built by the host once per tick and
    /// handed to every sim module in turn; a module never sees another
    /// module's commands.
    record observation {
        tick: u64,
        race-ticks: u64,                // oag_race::race_clock_ticks
        mode: mode,
        mode-id: option<string>,        // a mod mode's id when mode == custom
        speed-class: string,
        ship-count: u8,
        spline-length: f32,
        crafts: list<craft>,
        events: list<event>,            // this tick's, in engine order
    }

    /// What a guest may ask for. Every command is checked and clamped by the
    /// host (section 2.1, "commands"); a refused one is named in the log.
    variant command {
        scale-thrust(scale-thrust),
        impulse(impulse),
        shield-delta(shield-delta),
        give-pickup(give-pickup),
        clear-pickup(u8),
        hold-mag-lock(hold-ticks),
        set-finished(u8),
        set-standing-key(standing-key),
        publish(publish),
        message(message),
    }
    record scale-thrust { slot: u8, factor: f32, ticks: u32 }
    record impulse { slot: u8, delta-velocity: vec3 }
    record shield-delta { slot: u8, amount: f32 }
    record give-pickup { slot: u8, weapon: weapon, custom: option<string> }
    record hold-ticks { slot: u8, ticks: u32 }
    record standing-key { slot: u8, key: f32 }
    record publish { key: string, value: value }
    variant value { number(f32), integer(s32), text(string) }
    record message { slot: u8, string-id: string }

    record setup {
        title: string,                  // the title slug
        track-id: string,
        mode: mode, mode-id: option<string>,
        speed-class: string,
        laps-target: option<u32>,
        ship-count: u8,
        seed: u64,
        crafts: list<craft>,            // the grid as placed, tick 0
        volumes: list<string>,          // the track's trigger volume names, by index
    }
    record result { finished: list<u8>, places: list<u8> }
}

/// Host functions a sim guest may import. This is the complete list.
interface sim-host {
    use types.{value};
    /// A draw from a seeded stream the host owns: xoshiro128** today
    /// (oag_core::Rng), a child stream seeded from the race seed and the
    /// module's load index, so a module that draws nothing leaves the
    /// vanilla hashes untouched. Part of the snapshot and the hash.
    random-u32: func() -> u32;
    random-below: func(n: u32) -> u32;
    /// The save blob this mod wrote last time, if any (section 3, persistence).
    /// Read once at race-start; writes go through the `save` export's return.
    saved: func() -> option<list<u8>>;
    /// A log line, rate-limited by the host and tagged with the mod id.
    log: func(level: u8, text: string);
}

world sim {
    import sim-host;
    use types.{setup, observation, command, result};

    /// Once, after the grid is placed and before tick 0.
    export race-start: func(setup: setup);
    /// Once per tick, at the phase hooks.md names: after the person-flown
    /// slots' controls are mapped and pickups spent, before oag_physics::step
    /// and the race rules read their inputs, so a command lands in the same
    /// tick's force law and standings.
    export tick: func(obs: observation) -> list<command>;
    /// Once, when the race ends for everyone; the return is the save blob.
    export race-end: func(result: result) -> option<list<u8>>;
}
```

**Commands: bounds and what each writes.** Every command targets a slot the module
is allowed to touch (a craft module: slots flying its hull; a track or rules module:
any slot; a weapon module: the firing slot and the slots its use names). Out of
range is refused by name.

| Command | Writes | Clamp (chosen, not measured; the numbers are starting points) |
| --- | --- | --- |
| `scale-thrust{slot, factor, ticks}` | the slot's engine output for `ticks` ticks, multiplied into `Ship::pending_thrust_scale` (`crates/gameplay/src/world.rs`, the field the Turbo and the slowdown already write) each tick the host-side timer runs | `factor` in `[0.25, 2.0]`, `ticks` in `[1, 3600]`; two live scales on one slot multiply and the product is clamped again |
| `impulse{slot, delta-velocity}` | `Body::linear_velocity += delta-velocity`, through the same pending impulse the wall response uses (`ShipState::pending_impulse`, `crates/physics/src/ship.rs`) | `|delta-velocity|` at most the class's top speed; at most one per slot per tick |
| `shield-delta{slot, amount}` | `ShipState::shield`, through the damage path so the shield flash, the absorb window and the wreck threshold behave as a weapon hit does (`crates/physics/src/damage.rs`) | `amount` in `[-shield-max, +shield-max]`; a positive amount never exceeds `shield-max` |
| `give-pickup{slot, weapon}` / `clear-pickup{slot}` | `Ship::pickup` (`oag_weapons::pickup::Held`); `custom` requires the id to be one this mod declares | refused in a mode with weapons off (`Mode` decides, `crates/race/src/mode.rs`) |
| `hold-mag-lock{slot, ticks}` | the maglock's engage flag for `ticks` ticks, so `ShipState::mag_lock_blend` ramps in as on a magstrip (`crates/physics/src/maglock.rs`: `blend + 0.2` per frame) | `ticks` in `[1, 600]`; needs a `mag_floor` surface under the craft to hold to, else it ramps and releases |
| `set-finished{slot}` | the slot's `RaceState` finish, the same edge a lap gate sets | once per slot per race; ignored after |
| `set-standing-key{slot, key}` | a per-slot ranking scalar a custom mode's standings sort by, before progress and time | finite `f32` |
| `publish{key, value}` | the mod's HUD channel, read by this mod's HUD modules next frame | at most 32 keys per mod, keys at most 32 bytes, text at most 64 bytes |
| `message{slot, string-id}` | the HUD message board of a person-flown slot (`oag_hud::messages`, the board `Race::tick_body` advances) | one per slot per tick; the id must exist in the mod's strings |

The list a `tick` returns is capped (64 commands per module per tick); past the cap
the rest are dropped and the module is named. A non-finite `f32` anywhere in a
command is the NaN rule's error: the module is turned off for the race and named.

### 2.2 The HUD world

```wit
package oag:ext@1.0.0;

interface hud-types {
    record rect { x: f32, y: f32, w: f32, h: f32 }
    record rgba { r: f32, g: f32, b: f32, a: f32 }
    enum font { hud, hud-small, default }          // oag_hud::Font
    enum align { left, centre, right }

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
    variant value { number(f32), integer(s32), text(string) }

    /// What a widget draws. Each maps onto one oag_ui::frontend::Draw variant
    /// (crates/ui/src/frontend/draw.rs: Fill, Sprite, Text) and is clipped to
    /// the widget's box.
    variant draw {
        fill(fill),
        sprite(sprite),
        text(text),
    }
    record fill { rect: rect, color: rgba }
    record sprite { texture: u32, rect: rect, uv: rect, color: rgba }
    record text { font: font, text: string, x: f32, y: f32, align: align, scale: f32, color: rgba }
}

interface hud-host {
    /// A string from the mod's strings/<language>.toml, or the id itself.
    string: func(id: string) -> string;
    /// A texture the mod ships, by the file name listed in its [[extension.hud]]
    /// row; returns the handle `sprite.texture` takes, registered at load.
    texture: func(file: string) -> option<u32>;
    log: func(level: u8, text: string);
}

world hud {
    import hud-host;
    use hud-types.{readout, rect, draw};

    /// Once per frame the HUD draws, for the view the widget is on.
    /// `box` is the widget's rectangle in the layout's own grid, the space
    /// oag_hud::Sprite::rect is in (crates/hud/src/widget.rs:107-118).
    /// `dt` is the frame's wall-clock step, for animation only: the HUD world
    /// is not deterministic and nothing it computes reaches the simulation.
    export frame: func(readout: readout, box: rect, dt: f32) -> list<draw>;
}
```

The host appends a widget's draws to the `oag_hud::Frame` lists the pass already
paints (`crates/hud/src/draw.rs:695-713`, `oag_hud::draw_list` at `:796`), after the
title's own widgets, scissored to `box` the way a split-screen view's span is
(`Span::clip`, `:718`). The widget's box comes from the mod's HUD layout patch, a
`hud/*.toml` that adds, moves or hides elements by stable id (the thread's section
1c; `hud.tosd` not written yet) and may declare a `[[widget]]` with an `id`, a
`rect` per title layout and the module that draws it. On a title the mod is not
for, the widget does not exist.

### 2.3 What the sim world runs on

- **Host**: `oag-raceplay`, the crate whose `Race::tick` already maps inputs,
  runs the pads, the force law, the rescue and the race rules per slot
  (`crates/raceplay/src/tick.rs`, `player_step.rs`). The simulation crates
  (`oag-gameplay`, `oag-physics`, `oag-race`, `oag-weapons`, `oag-ai`) gain **no
  dependency** on any WebAssembly crate: the host reads `World` and writes the
  fields listed above, all of them existing inputs to the tick. That keeps `just
  check-deps` and the determinism scan as they are.
- **Behind a cargo feature** (`extensions`), off in `oag-server`'s default build
  until phase 3 (rule 6: the server links no drawing crate; a wasmtime dependency is
  allowed there, but the server runs sim modules only when the lobby carries their
  hashes).
- **A race with any sim module is MODDED** (phase 2). Golden hashes, the determinism
  references and every ground-truth test run with no module loaded, so a vanilla
  race's bytes are unchanged by this work: the host code path with zero modules does
  nothing, which is checked by a test that hashes a race with and without the feature
  compiled in.

## 3. What is not exposed, and why

Each line is a decision, not an omission. A guest asking for one of these fails at
load (an import the host does not provide) or at the command check (named refusal).

| Not exposed | Why |
| --- | --- |
| **No WASI, no files, no network, no environment, no clock, no threads.** The host provides exactly `sim-host` or `hud-host` above | Determinism: a clock or a file is an input the replay does not carry (`docs/architecture/determinism.md`, "Time", "Threads"). Safety: a mod is read in place from an untrusted archive; it gets no reach past its own files. The web build cannot grant them anyway |
| **No raw `World`, `Ship` or `Body` pointer, no shared memory with the host** | `World` is plain data whose layout changes with nearly every simulation commit ([ADR-0003](../adr/0003-no-ecs.md), [ADR-0055](../adr/0055-replays-are-inputs-and-a-ghost-is-poses.md) "a whole-world snapshot"). A guest reading it by offset breaks on the next field; a guest writing it bypasses every clamp. Copies in, commands out |
| **No renderer, GPU, window or camera from the sim world** | The simulation must not know a renderer exists (CLAUDE.md, dependency rule 1). The camera is view state, outside the hash |
| **No audio device, no cue injection** | Cues are a per-tick output of the simulation, never state ([ADR-0018](../adr/0018-audio-mixer-architecture.md)); a mod that wants a sound names a cue id in a later minor (open question 6), it never opens the mixer |
| **No input devices; only the `InputSnapshot` the tick already received, for person-flown slots** | The snapshot is the one thing a replay and a network peer carry (ADR-0055, ADR-0070 decision 4). Reading a pad directly would be an un-replayed input |
| **No reading or writing another mod's memory, commands or published keys** | One `Store` per module; composition between mods is an unsolved ordering question (section 8) and must not happen by accident |
| **No persistence except the host-mediated save blob** (`sim-host.saved`, `race-end`'s return; at most 64 KiB, keyed by mod id and title, written through `records::save_in_background` like every other save) | Modern saves: no slots, no files a player manages. The blob is the mod's only memory between races, so a mod cannot leave state on disk the MODS page does not know about; turning a mod off hides its blob, uninstalling deletes it |
| **No change to an original title's faithful modes unless the player opts into the mod** | The MODS page's ENABLED switch is the opt-in (ADR-0078). With a sim module on, the session is MODDED (phase 2): records go to the modded table, ghosts and replays are off, the HUD says so. The original campaigns never load an extension (settled) |
| **No rewriting of handling tables, weapon tables, pad volumes or the spline** | Never invent what the assets author (CLAUDE.md). A craft specialty is expressed as commands against the disc's own numbers (`scale-thrust`), never as a replacement `Handling`; a track's volumes are authored in the track file, not conjured per tick |
| **No new projectile kinds in v1** | `World::projectiles` is a fixed-size pool of plain `Copy` data sized by `MAX_PROJECTILES`; the Quake, Leach Beam and Repulser each have their own single-slot pool (`crates/gameplay/src/world.rs`). A guest-defined projectile would change the snapshot's size and every golden hash. A custom weapon is scripted through commands (section 7.5); a projectile pool for mods is open question 4 |
| **No menu rows beyond ADR-0066's closed vocabulary, no settings, no pages** | A mod cannot invent front-end behaviour (ADR-0066 decision 4); the extension kinds attach to a race, not to the menus |
| **No AI control in v1** (a controller module is the pilot-controller thread's ADR, not this one) | The AI obeys the player's physics; an extension's commands apply per slot whatever flies it, and a craft module cannot tell its own slot from a person's except by `controller`, which it may read and must not use to favour the AI (a rule for `oag-extras`' own modules; a third-party mod is the player's choice) |
| **No unbounded work**: fuel per tick, memory cap, command cap, string caps | A tick has a budget on a Steam Deck and a phone; a guest that loops forever must be a named failure, not a hang (section 4) |
| **No floating-point escape hatches**: no relaxed SIMD, no threads, no shared memory, NaN canonicalised, a NaN in any output is an error | The three-engine rule (established above) |
| **No access to the disc's archives, other mods' files or the asset layers** | A mod's module reads its own files, registered at load (`hud-host.texture`). The disc is the player's; a mod never streams it |
| **No spawning or removing craft, no grid changes mid-race** | `MAX_SHIPS` is a real bound and the grid is placed once (`Race_PlaceGrid`); standings, cameras and the HUD assume the slot set is fixed |

## 4. Sandboxing, limits, failure, versioning, trust

### 4.1 Engine configuration

wasmtime, with the configuration below on every target that runs it (the browser
case is section 5). Everything a guest could observe is pinned:

| Setting | Value | Why |
| --- | --- | --- |
| `consume_fuel` | on; fuel per `tick` call fixed per world (sim: a number the spike settles, on the order of 1 million; hud: ten times that) | the only portable work bound; wall-clock epochs are not deterministic |
| `cranelift_nan_canonicalization` | on | NaN payloads differ between engines; the thread's NaN rule |
| `wasm_relaxed_simd`, `wasm_threads`, `shared_memory`, `wasm_memory64` | off | the deterministic subset |
| `wasm_simd` | off in v1 | avoids a second code path to prove bit-identical on Pulley and the browser; revisit when the conformance test exists |
| `StoreLimits` | memory 16 MiB (256 pages), 1 table of at most 4,096 elements, 1 instance, 1 memory | a widget or a rules module needs kilobytes; the cap is a fuse |
| module size | at most 4 MiB on disk | a mod is small text and meshes; a bigger module is a mistake |
| `cache` | off (no compilation cache on disk) | the host writes nothing a mod asked for |
| `parallel_compilation` | off | the race load thread compiles; no rayon pool in the simulation half |
| features `wasi`, `async`, `profiling`, `coredump`, `addr2line`, `debug-builtins` | off | not needed, and `wasi` is the whole non-capability list |
| component model | on (`wit-bindgen` guests) | the WIT above is the contract; a core-module ABI would be a second one |

### 4.2 Traps and failure

- **A trap, out-of-fuel, out-of-memory, a refused command past the cap, or a
  non-finite output** in a sim module **turns that module off for the rest of the
  race**, the tick continues without it, and the HUD's MODDED tag and the end-of-race
  log name the mod and the tick. The race is not aborted: a player two laps in keeps
  the race; the mod author gets the tick number.
- **The switch-off is itself simulation state** (the tick it happened on is hashed),
  so a replay or a network peer that runs the same module with the same fuel reaches
  the same switch-off on the same tick. Fuel makes a trap deterministic; a wall-clock
  deadline would not.
- **A HUD module that traps** stops drawing; the title's own HUD is untouched. Named
  once in the log.
- **A module that will not validate at load** (unknown import, a disallowed feature,
  wrong `api` version) leaves its mod out of the race, named in the load report; the
  race runs vanilla.

### 4.3 Versioning

- `oag:ext@1.0.0` is **edited in place until the first release** (pre-release rule:
  no migration code paths). A module built against an import the host no longer has
  fails at load by name; that is the whole compatibility story before 1.0.
- After the first release: an addition (a new event variant, a new command, a new
  readout field) is a minor the same package version absorbs, because WIT records
  and variants extend compatibly for a guest that ignores what it does not know; a
  change to an existing field is `oag:ext@2.0.0`, a second world the host exports
  beside the first.
- The manifest's `api` says which package version the modules were built against;
  the host refuses a major it does not export.

### 4.4 Trust

- **Online play is server-authoritative** (decision 2): the server runs the same
  modules, accepts inputs only, and its hash per tick is the truth
  ([ADR-0070](../adr/0070-the-network-transport-is-webtransport.md) decision 4). A
  client with a different module mispredicts and is corrected; it cannot cheat the
  result.
- **A LAN host is trusted** (decision 3); the joiner sees the content fingerprint and
  joins or not.
- **The fingerprint grows a hash per sim module.** Today it is FNV-1a over `id@version`
  of the loaded mods (`oag_net::discovery::content_fingerprint`,
  `crates/net/src/discovery.rs:168`); with extensions it also folds in the SHA-256 of
  each sim module's bytes, so two builds of the same mod version with different code
  do not match. Mod code arriving through a lobby is never auto-run (decision 6): the
  joiner must have the mod installed, and `OtherContent` stays the verdict otherwise.
- **Replays**: the header's `options` table (`oag_replay::Header`,
  `crates/replay/src/header.rs:58`) gains `mods = "<id>@<version>:<sha256>,..."`. In
  phase 2 a modded session records no replay at all; in phase 3 the `Verifier` refuses
  playback unless the identical modules are installed and on, by name. The stored
  hashes still decide validity (ADR-0055, "when the simulation changes").
- **Records**: a modded session's results go to the separate modded table (phase 2),
  keyed with the fingerprint; the networking page already says a modified-sim run goes
  to a separate board tagged with its fingerprint.

## 5. Targets: the same bits everywhere

| Target | Engine | How the module gets in | Status |
| --- | --- | --- | --- |
| Linux, Windows, macOS | wasmtime, Cranelift | compiled at race load on the load thread | the configuration above; `just test-wasm` already runs the simulation suites under wasmtime 49 (`docs/tools/web.md`) |
| Android | wasmtime, Cranelift (aarch64) | same | same host code; the mobile glue is `oag-mobile` ([ADR-0065](../adr/0065-one-mobile-crate-for-every-phone-os.md)) |
| iOS | wasmtime, **Pulley** interpreter | same API, no JIT page | the thread's choice; the spike confirms Pulley against Cranelift bit for bit |
| Web | two candidates, the spike decides | compiled asynchronously at race load, never per tick (browsers limit synchronous compiles on the main thread); the race load already runs on a Web Worker (`docs/tools/web.md`, "Threads") | **(a)** wasmtime built for `wasm32` with Pulley as its only backend, so the guest runs inside our module on the identical interpreter as iOS, fuel and limits included; **(b)** the browser's own `WebAssembly` API through `js-sys`, with fuel reproduced by instrumenting the module at load. (a) is the one that keeps one host and one proof; (b) is faster and is the thread's original wording. Open question 1 |

**Same bits everywhere means the guest's results, not the host's speed.** The proof
is the engine half's spike (`spikes/wasm-extensions/`): the same module, the same
inputs, on Cranelift and Pulley, with fuel on, must produce the same outputs and the
same fuel exhaustion tick. Its result goes here when it lands. The simulation itself
is already proven deterministic on `wasm32` (`just test-wasm`, debug and release,
2026-10-09), so the open proof is the nested guest, not the host.

## 6. Licences

The workspace is `MIT OR Apache-2.0` (`Cargo.toml` `[workspace.package]`,
`LICENSE-MIT`, `LICENSE-APACHE`). There is no `deny.toml`; the policy in practice is
permissive dependencies only, and the lock file already carries `Apache-2.0 WITH
LLVM-exception` (`rustix`, `target-lexicon` under wgpu and quinn), `ISC` (`ring`) and
one weak copyleft, `MPL-2.0` (`symphonia`, file-level, in `oag-music`). Every proposed
dependency fits inside that:

| Crate | Licence | Role | Note |
| --- | --- | --- | --- |
| `wasmtime` (with `cranelift`, `pulley`, `component-model`, `runtime`; without `wasi`, `cache`, `parallel-compilation`, `async`, `profiling`, `coredump`, `addr2line`, `debug-builtins`) | Apache-2.0 WITH LLVM-exception | the engine | the LLVM exception only loosens Apache's attribution clause for binaries; compatible with both halves of ours |
| `wasmtime-wasi` | Apache-2.0 WITH LLVM-exception | **not used** | listed to say so: no WASI |
| `wit-bindgen` (guest macro, for our own example mods) and `wit-component` / `wasm-tools` (build-time) | Apache-2.0 WITH LLVM-exception | the WIT bindings | guest-side only in the example mods and the Pilot Assist port; the host uses `wasmtime::component::bindgen!` |
| `wasmparser`, `wasm-encoder` | Apache-2.0 WITH LLVM-exception | validation (already wasmtime's own deps) | the feature check at load reads the module's declared features through `wasmparser` before compiling |
| `cranelift-codegen` and family, `pulley-interpreter` | Apache-2.0 WITH LLVM-exception | wasmtime's backends | transitive |
| `js-sys`, `web-sys`, `wasm-bindgen` | MIT OR Apache-2.0 | web candidate (b) | already in the tree for the web build (`crates/game/Cargo.toml`) |
| `sha2` | MIT OR Apache-2.0 | module hashes in the fingerprint | or `ring`'s digest, already linked by `oag-net` |

Two wasmtime optional dependencies to keep **off**: `ittapi` (BSD-3-Clause OR GPL-2.0,
the VTune profiler hook, behind `profiling`) and anything behind `cache` (it writes
under the user's cache directory). A `deny.toml` allowlist (MIT, Apache-2.0 with and
without the LLVM exception, BSD-2/3, ISC, Zlib, Unicode, MPL-2.0 for the existing
case) is proposed as part of phase 1 so this table stays true without re-reading the
lock file (open question 7).

## 7. The maintainer's examples, end to end

Each walk names the extension kind, the manifest row, the events read, the commands
returned, what the host does with them, and what a player sees. Hook names are the
agreed ones (section 2); phases are `hooks.md`'s.

### 7.1 Feisar gets faster on each speed pad

The 2048 prototype idea: each team's fourth craft has a specialty
(`oag_2048::race::SHIP_TYPES`, `"prototype"` the fourth; `docs/formats/2048-campaign.md`
"A prototype craft is not a fifth, independent class"). The specialties themselves are
**not recovered** from 2048 and not measured; this is a mod's invention, labelled so.

1. **Declaration**: `[[extension.craft]] team = "feisar", variant = "prototype",
   titles = ["wipeout-2048"], module = "ext/feisar.wasm"`, `simulation = true`.
2. **Load**: the race is on 2048 with a slot flying `feisar2048\4`; the module compiles
   on the load thread; its `race-start` receives the grid and remembers which slots
   fly its hull (`craft.team == "feisar"`, `variant == 3`). The session is MODDED.
3. **Each tick**: `tick(obs)` scans `obs.events` for `pad-crossed{slot, kind:
   speedup}` on one of its slots (the host raises it from `test_speedup_pads`,
   `crates/raceplay/src/pads.rs:73`, the first tick the hull enters a pad's volume,
   not every tick the pad keeps re-arming `pad_timer`). It increments a per-slot
   counter in its own memory and returns `scale-thrust{slot, factor: 1.0 + 0.02 *
   n, ticks: 3600}` each tick (re-issued, because the host timer is the bound, not
   the memory).
4. **Host**: clamps `factor` to `[0.25, 2.0]` (so the twentieth pad is the last that
   counts), multiplies it into the slot's `pending_thrust_scale` for the tick, which
   `step_player` takes with `mem::replace(.., 1.0)` and hands to the force law
   (`crates/raceplay/src/player_step.rs:231`), the same field the Turbo pickup and the
   slowdown write. The AI slot flying the same hull gets the same specialty: the
   AI obeys the player's physics.
5. **Player sees**: the pad boost as always, then a hull that keeps more speed on the
   straights; the HUD's speed bar, which reads `speed_kmh` from the body's velocity
   (`crates/raceplay/src/telemetry.rs:137`), shows it with no HUD change. A HUD
   module (7.6) could show the counter from `publish{"feisar.pads", n}`.
6. **Replay and records**: phase 2, none recorded; MODDED on the results. Phase 3: the
   module's hash in the header and the lobby.

### 7.2 A spring jump on a custom track

1. **Track**: the author tags a Blender object as a trigger volume; the add-on writes
   `oag/trigger/0` into the glb (a `TRIANGLES` or box record with extras
   `oag_kind = "trigger"`, `oag_name = "spring-a"`; a minor addition to the glb table
   in `docs/formats/track-file.md`), and the manifest names
   `extension = "spring-loop.wasm"`.
2. **Load**: `setup.volumes = ["spring-a"]`. The reader counts the record; a build
   without the `extensions` feature reports it unread and races the track flat.
3. **Each tick**: the host tests every active hull against the trigger volumes with
   the sweep it already runs for pads (`pad_sweep`, `crates/raceplay/src/player_step.rs:209`)
   and raises `volume-entered{slot, volume: 0}` on the entry tick. The module returns
   `impulse{slot, delta-velocity: up * 18.0 + forward * 4.0}` (its own numbers).
4. **Host**: clamps `|delta-velocity|` to the class's top speed, adds it to the slot's
   `pending_impulse`, which the force law applies this tick. The craft leaves the
   floor; `grounded` falls, `time_airborne` runs, the airborne pitch law and the
   landing both behave as after a disc jump, because the engine already has
   `Handling::speedpad_jump` and `turbo_jump` as upward kicks (`crates/physics/src/params.rs:346-349`).
   The off-track rescue still applies: a jump into the void respawns as any other
   (`lost_off_the_track`, `airborne_too_long`, `player_step.rs`).
5. **Player sees**: the jump, with the camera and shake the engine already does on
   landing. No sound in v1 (open question 6).

### 7.3 A gravity switch to the underside

Two honest facts first. **Gravity in this engine acts on world `-Y` only**
(`crates/physics/src/forces.rs:221-222`: "Gravity still acts on world `.y` only;
neither is a gravity direction"), and **the original's way of racing an inverted
section is the magstrip**, a kinematic attitude hold that fades the hover out and
slaves the hull to the surface normal (`crates/physics/src/maglock.rs`, measured at
confidence 90 off `Ship_UpdateMagLock`; `docs/physics/cornering-ground-truth.md`
measures a Talon's Junction inverted section). So:

1. **Track**: the underside is authored as `mag_floor` collision
   (`oag_kind = "mag_floor"`, `docs/formats/track-file.md`), exactly as a disc
   circuit authors a magstrip. **No code**: a craft that drives onto it is held. This
   is the faithful mechanism and it already works on a track file.
2. **The switch moment** is what the extension adds: a trigger volume at the lip
   (`volume-entered`), and the module returns `hold-mag-lock{slot, ticks: 120}` so
   the hold engages before the hull has a `mag_floor` sample under it (the blend
   ramps at `0.2` a frame, `maglock.rs`), and optionally `impulse` toward the surface.
   The host sets the engage flag the magstrip test would set; the hold releases when
   `ticks` run out and no `mag_floor` is under the craft.
3. **A true gravity flip** (free flight pulled toward the underside, no surface
   contact) is **not a capability**: it needs `Environment` to take a gravity frame
   per craft, a change to the measured force law for every title, and it is listed
   as open question 3, not promised. The underside drive the maintainer described is
   reachable without it.
4. **Player sees**: the hull roll onto the underside with the maglock's own
   attitude blend, the camera following as on Talon's Junction.

### 7.4 A to B instead of a circuit

The thread's order stands: rules as data first, open splines later. The extension's
part is the piece data cannot say.

1. **Mode**: `modes/a-to-b.toml` with `finish = "distance"` and a finish sample (the
   rules-as-data step; a sprint over part of an existing closed loop needs only the
   finish gate and a grid at a sample other than the lap line). On a closed loop,
   `progress` wraps (`crates/race/src/standing.rs`, `wrapped_forward`), so the finish
   gate is the data layer's, not the module's.
2. **What the module adds**: checkpoints with time bonuses, or a "last place every
   30 s is out" rule. `race-start` gets `spline-length` and the grid; each `tick`
   reads `craft.progress` and `race-ticks`; it returns `set-standing-key{slot, key}`
   (the mode sorts by key, then progress, then time), `set-finished{slot}` when its
   own rule says a slot is done, and `publish{"a_to_b.remaining_m", f32}` for the HUD.
3. **Host**: `set-finished` raises the slot's finish edge once; `set-standing-key`
   feeds the standings the mode's data rules sort with. Everything else (results
   screen, records key per mode, the replay header's mode) is the mode file's.
4. **An open point-to-point track** stays "later" (the spline, the spawner, the
   rescue, the AI plan and the flyby all assume a loop); the extension API does not
   change when it lands, only the track format does.

### 7.5 A custom weapon

The pools are fixed (section 3, "no new projectile kinds in v1"), so a v1 weapon is
**scripted**: a pickup the mod declares, whose use is a sequence of commands.

1. **Declaration**: `[[extension.weapon]] id = "emp", icon = "ext/emp.png",
   name_string_id = "WEAPON_EMP", module = "ext/emp.wasm"`.
2. **Host additions this needs** (not built): a `Custom(u8)` arm in
   `oag_weapons::pickup::Held` (the index into the mod's weapon list), a per-mode
   roll table the pad roll draws from (`oag_weapons::pickup::Driver`,
   `crates/weapons/src/pickup.rs:116`) that includes the custom ids with the
   mod's weight, and the HUD icon from the mod's file in place of the disc's
   (`oag_hud::pickup_icon_name`). Each is a change `docs/gameplay/weapon-lineage.md`
   must record (a new pickup is a weapon-lineage row, the member rule).
3. **Each tick**: the host raises `weapon-fired{slot, weapon: custom, custom: "emp"}`
   when the slot spends the pickup. The module reads `obs.crafts`, picks every other
   craft within 30 units, and returns `scale-thrust{target, factor: 0.5, ticks: 120}`
   and `shield-delta{target, amount: -5.0}` for each, `message{slot, "EMP_HIT"}` for
   the person-flown ones, and `clear-pickup{slot}`.
4. **Host**: clamps and applies; the damage path raises the shield flash and the
   absorb window as a weapon hit does, so the victim's HUD reacts; `weapon-hit`
   events are raised for the victims so a craft module (7.1) can react in turn.
5. **What a scripted weapon cannot do**: fly a projectile, draw a beam, play the
   disc's weapon effects. Those are `oag_fx` visuals bound to the fixed kinds. A
   projectile pool for mods, with a visual from the mod's own files, is open
   question 4.

### 7.6 A HUD extension

1. **Declaration**: `[[extension.hud]] id = "lap-delta", module = "ext/lap_delta.wasm",
   layout = "hud/lap-delta.toml"`; the layout patch declares `[[widget]] id =
   "lap-delta", rect = { pulse = [300, 20, 160, 60], hd = [1480, 60, 400, 150] }`
   in each title layout's own grid.
2. **Each frame**: `frame(readout, box, dt)` gets the slot's `Readout` copy
   (`lap-splits`, `best-lap-ticks`, `lap-ticks`) and this mod's published keys; it
   returns `fill` and `text` draws inside `box` (a bar per completed lap scaled by
   its split against the best). No simulation state is read or written; the widget
   runs in the HUD pass on the render side and is absent from every hash.
3. **Host**: appends the draws to the `Frame`'s `sprites` and `hud_text` lists after
   the title's own, scissored to `box`, and `oag_game::hud_overlay` rasterises them
   as it does every `Draw::Fill` and `Draw::Text`.
4. **Player sees**: the widget where the patch put it, on the titles the mod is for,
   gone when the mod is off. The vanilla layout is the default; a HUD mod is opt-in
   (the thread's 1c.4).

## 8. Open questions

1. **Web engine**: wasmtime-on-`wasm32` with Pulley (one proof, slower) or the
   browser's `WebAssembly` with instrumented fuel (faster, a second proof). The spike
   measures both on a HUD widget and the Pilot Assist port.
2. **The fuel number** per `tick` and per `frame`, and whether the HUD world gets fuel
   at all or a wall-clock budget (it is not deterministic, so a deadline is allowed
   there).
3. **A per-craft gravity frame** in `Environment`, for a true gravity flip. Changes the
   measured force law's inputs on every title; needs a measured default of "world
   `-Y`" proven bit-identical before anything else may set it.
4. **A projectile pool for mods**: a second fixed-size pool outside
   `World::projectiles` so the vanilla snapshot's size is unchanged, with a visual
   from the mod's files. Also: can a mod reuse a disc weapon's visuals (a rocket model
   with a different law) without a stand-in?
5. **Event granularity**: `pad-crossed` on entry only (proposed) or every tick the
   pad re-arms; `wall-contact` per contact or per tick; whether `weapon-hit` carries
   the damage amount.
6. **Sound**: a `cue{slot, id}` command naming a disc cue by id, or a cue from the
   mod's own WAV; both need the audio thread's architecture (ADR-0018) to accept a
   mod-provided bank.
7. **A `deny.toml`** allowlist for the workspace, so section 6 is checked by the gate.
8. **Composition**: whether two sim mods may read each other's published keys
   (a rules mod showing a craft mod's counter), and in what order.
9. **The pilot-controller thread's three questions** (may a pilot fly `"original"`
   outside the campaign; mod controllers in multiplayer; WebAssembly as the mod ABI):
   this page assumes yes to the third and leaves the first two to the maintainer.
10. **Hot reload** of a module during a race for mod authors (a `--reload-extensions`
    debug flag): useful, and incompatible with the hash; debug builds only, if at all.

## 9. Phased rollout

In the thread's order, with the extension kinds slotted in:

1. **Phase 1, no simulation code**: the `extensions` feature, the wasmtime host with
   the configuration of section 4.1, the HUD world, `[[extension.hud]]` and the
   `hud/*.toml` widget box, one example widget (the lap-delta graph) in
   `mod-examples`. Proves the host trait and the engines on low-stakes code. Lands
   with the ADR accepted and the spike's result on this page.
2. **Phase 2, the simulation hook**: the sim world, `simulation = true` and the MODDED
   session, `race-start`/`tick`/`race-end`, the observation and the command set with
   their clamps, hashed guest memory and host timers (`determinism-and-state.md`),
   proven by moving Pulse/Pure's Pilot Assist backport into a mod with bit-identical
   replay hashes (`pilot_assist_ground_truth.rs`). Then the craft kind (7.1) as the
   first player-facing use.
3. **Track triggers**: the `oag/trigger/<n>` record, the Blender add-on's tag,
   `track.extension`, `volume-entered`, `impulse` and `hold-mag-lock` (7.2, 7.3).
4. **Rules**: after the rules-as-data refactor and `modes/*.toml`; `set-finished`,
   `set-standing-key` (7.4); the drag race as the first mod mode, no module needed.
5. **Weapons**: `Held::Custom`, the roll table, the icon, `give-pickup` with a custom
   id, the weapon-lineage rows (7.5).
6. **Phase 3**: module hashes in the replay header and the lobby fingerprint, the
   `Verifier` refusing a mismatch by name, the conformance test across Cranelift,
   Pulley and the web engine, `oag-server` running sim modules; MODDED stops meaning
   "no replay, no multiplayer" and starts meaning "with these modules".
