# ADR-0079: Extensions are sandboxed WebAssembly guests behind two worlds, a sim world of copies-in and bounded commands-out and a HUD world that reaches no state

## Status

**Proposed** (2026-10-29). A design for the maintainer to accept, change or
reject; nothing is built. Builds on [ADR-0066](0066-the-menu-is-the-disc-plus-mods.md)
(mods), [ADR-0078](0078-a-mod-is-turned-off-by-id-live-and-the-built-in-never.md)
(the MODS page as the opt-in), [ADR-0063](0063-the-own-track-format-is-glb-plus-a-toml-manifest.md)
and [ADR-0076](0076-a-track-file-carries-its-art-as-an-optional-section.md)
(the track file; decision 11 routes triggers to this host),
[ADR-0077](0077-a-custom-campaign-compiles-to-the-titles-own-grid.md) (custom
campaigns are where custom content belongs), [ADR-0055](0055-replays-are-inputs-and-a-ghost-is-poses.md)
(replays are inputs and hashes), [ADR-0070](0070-the-network-transport-is-webtransport.md)
(inputs on the wire, a hash per tick) and [ADR-0003](0003-no-ecs.md) (the world
is plain data). The capability surface, the non-capability list, the limits,
targets, licences and the worked examples are
[`docs/architecture/wasm-extensions/README.md`](../wasm-extensions/README.md); the
hooks at their call sites are [`hooks.md`](../wasm-extensions/hooks.md); the
determinism and state model, with the spike's measurements, is
[`determinism-and-state.md`](../wasm-extensions/determinism-and-state.md).

## Context

The maintainer wants mods to carry **behaviour**, not only data: a craft with a
specialty (2048's prototype idea: Feisar faster on each speed pad), a track that
springs a jump or switches a craft to its underside, a race from A to B, a custom
weapon, a HUD widget. The decisions already taken (2026-10-12, the custom race modes
thread) are: custom modes come through mods, potentially WebAssembly; rules as data
first; wasmtime with Cranelift, Pulley on iOS, the browser's own engine on the web;
a phase with no simulation code, then the simulation hooks marking the session
MODDED, then hashes in the replay header and the lobby. What was missing is the
**shape of the API**: who may extend what, through which functions, with what in and
what out, and the list of what is withheld, with reasons.

Four properties of this engine constrain any answer:

1. **Determinism is load-bearing** (`docs/architecture/determinism.md`): replays,
   golden tests, network play and the trace comparison all break on one
   non-deterministic bit. A guest is simulation code the moment it changes a
   `World` field.
2. **`World` is plain data with a layout that changes weekly** (ADR-0003,
   ADR-0055), and network rollback restores `World`, never a guest's memory
   (`Race::snapshot_sim` / `restore_sim`). Anything a guest keeps outside what the
   host owns is lost on a rollback: the spike measured a guest holding a counter in
   its own memory diverging after one.
3. **The simulation must not know a renderer exists**, and no gameplay crate may
   depend on `oag-render`, `oag-audio`, `oag-input` or `oag-game` (`just check-deps`).
4. **Never invent what the assets author**: the disc's tables, pads and splines are
   the law; a mod adds on top of them, it does not replace them.

## Decision

1. **Two worlds, one call per tick.** A **sim world** whose guest exports
   `race-start`, `tick` and `race-end` and is called **once per tick at one fixed
   phase** of `Race::tick` in `oag-raceplay`'s simulation half (after the people's
   pickups are spent, before any craft steps): the engine records events where they
   happen (`pad-crossed`, `volume-entered`, `wall-contact`, `weapon-fired`,
   `weapon-hit`, `lap-completed`, `respawned`, `reset`, `eliminated`) and hands them
   to the guest in the next call's observation, a one-tick latency stated rather
   than hidden. A **HUD world** on the render side whose guest exports `frame` and
   draws inside a box. The sim guest imports one namespace, `oag`, holding a seeded
   random draw and a log line; the HUD guest the mod's strings and textures and a
   log line. No WASI; an import outside that fails at load by name.
2. **Core WebAssembly, one I/O buffer, no component model.** The browser cannot run
   components natively, and three engines must run the same bytes. The WIT on the
   README is documentation of the records and signatures; the wire layout is a fixed
   little-endian buffer.
3. **Copies in, bounded commands out, a host-owned blob between.** The host writes an
   observation record into the buffer before the call (every slot's pose, velocity,
   shield, timers, lap and progress; the person-flown slots' `InputSnapshot`; the
   events since the last call; the blob); the guest returns a reply of commands and
   the blob to keep. Every command (`scale-thrust`, `impulse`, `hold-mag-lock`,
   `shield-delta`, `give-pickup`, `assist`, `set-finished`, `set-standing-key`,
   `publish`, `message`) is checked (finite, in range, the slot allowed, the command
   one the mod's kind may send) and applied by the host at the point in the tick
   where its target is read, into a field that is already an input to the tick
   (`pending_thrust_scale`, the body at the rail write point, the maglock's engage
   input, the damage path, `Held`, `Environment::pilot_assist`, the finish edge, the
   key `places` sorts by). `publish` and `message` are outputs queued beside the
   cues, never state. No pointer, handle or shared memory crosses the boundary.
4. **The guest is stateless across ticks.** One pooled instance per mod per tick;
   the only state that crosses a tick boundary is the mod's fixed-size blob (a size
   the manifest declares, capped), stored with its own `Rng` stream and its fault
   flag in a fixed array in the race state, snapshotted, restored and hashed with
   everything else. With no sim mod loaded nothing is hashed for them, so every
   vanilla hash is unchanged.
5. **Five extension kinds** bind modules to things that exist: a **craft** module to
   a team identity (optionally a variant) in `oag-mod.toml`; a **track** module to a
   track file, named by `track.extension` in its manifest, with its volumes as a new
   optional glb node kind `oag/volumes/<name>`; a **rules** module to a mod-declared
   mode (after rules as data); a **weapon** module to a pickup id the mod declares
   (a custom arm of `oag_tables::weapons::Weapon`, an engine change); a **HUD**
   module to a widget box in a HUD layout patch, reading a copy of `Readout` plus
   the keys the mod's sim module `publish`ed. All but HUD are sim-world and require
   `simulation = true`. Modules are called in the order their ids sort; one craft
   module per hull, one track module per track, one rules module per race, a second
   left out and named, never stacked.
6. **The non-capabilities are part of the contract**, each with its reason on the
   README: no files, network, clock, threads or WASI; no guest state across ticks
   but the blob; no raw `World`; no force, velocity or position write outside the
   capped `impulse` and `respawn-at` on authored volumes; no replacing a recovered
   law or a disc table; no renderer, GPU, camera or audio from the sim world; no
   input devices beyond the snapshot the tick received; no cross-mod memory; no
   persistence except a host-mediated blob; no effect on an original title's modes
   unless the mod is on and the session is MODDED; no guest-shaped projectiles (a
   mod weapon fires a host template into the fixed pool); no menu vocabulary; no AI
   advantage from `oag-extras`; no unbounded work; no floating-point escape hatches;
   no access to the disc's archives; no grid changes.
7. **Limits are fixed and deterministic, and the failure has one answer.** Fuel per
   call, a declared memory maximum under the same `StoreLimits` everywhere, a
   call-depth cap, a capped command list and a capped event buffer, a 4 MiB module; NaN
   canonicalisation on; relaxed SIMD, SIMD, threads and shared memory refused; no
   compilation cache, no parallel compilation. Because the browser has neither fuel
   nor NaN canonicalisation, both also run as a load-time rewrite of the module,
   applied on every engine. A trap or a refused command **faults the mod for the
   rest of the race**: that call's outputs are discarded, the blob keeps its
   pre-call value, every later call is skipped, the fault flag and tick are hashed
   (the fuel total never is), and the HUD and the results name the mod. The race
   goes on.
8. **The host is its own crate**, proposed `oag-modhost` (wasmtime and `oag-core`,
   no game knowledge, in the determinism scan), called from `oag-raceplay`'s
   simulation half behind an `extensions` feature; the gameplay crates gain no
   dependency on it; `oag-server` links it and no renderer; the web build never
   links wasmtime.
9. **Trust follows the networking decisions.** Online: the server runs the modules
   and its hash is the truth. LAN: the host is trusted. The lobby already refuses
   differing mods by fingerprint; phase 3 folds each sim module's SHA-256 into it.
   The replay header gains a dedicated `mods` list (id, version, module SHA-256,
   blob layout version), editing format v1 in place; playback refuses a mismatch by
   name. Mod code never arrives through a lobby.
10. **Versioning is the pre-release rule**: ABI major 1 edited in place until the
    first release; after it, additions stay in major 1 and a changed field is a
    second layout beside the first. The manifest's `api` names the major.
11. **Targets**: wasmtime 50 with Cranelift on desktop and Android; on iOS Cranelift
    compiling to Pulley bytecode on the device (no native code); on the web the
    browser's own `WebAssembly` over the rewritten module. Same guest bits everywhere
    is proven by the conformance test (Cranelift, Pulley, a headless browser, an
    AArch64 machine), not assumed; the spike already shows Cranelift and Pulley
    bit-identical over a 36,000-tick race, fuel trap points included, on one x86 host.

## Alternatives considered

- **A scripting language (Lua, Rhai).** Easier to write; harder to keep
  bit-identical across engines and operating systems, and every host binding is a
  hand-written surface. The pilot-controller thread already recommended against it.
  WebAssembly gives the sandbox, the fuel meter and the deterministic float subset
  for free, and one module runs on every target.
- **Native plugins (`dlopen`, cdylib).** No sandbox, no web, no iOS, and a crash in a
  mod is a crash in the game. Rejected outright.
- **wasmtime in the browser** (built for `wasm32`, Pulley as its only backend). One
  host and wasmtime's fuel and NaN canonicalisation everywhere, but it interprets wasm
  inside wasm, far slower than the browser's own engine, and grows the web binary.
  Rejected by the maintainer (2026-10-29) unless a serious reason shows up, such as
  the load-time rewrite failing to match the native hosts.
- **The component model and WIT as the wire ABI.** Cleaner bindings; but browsers do
  not run components natively, so the web host would need a transpiler layer and the
  three engines would no longer run the same bytes. Core wasm with a buffer is what
  the spike's guest is, and WIT stays as documentation.
- **One world for everything.** A HUD module would then need `simulation = true`
  and every HUD mod would mark the session MODDED, which is the opposite of phase 1's
  point. Two worlds let the harmless half ship first and keep the dangerous half
  small.
- **A guest hook per call site** (`craft.controls`, `craft.environment`, `pad.enter`,
  `race.tick_end`, ... an earlier draft of `hooks.md`) instead of one call per tick.
  It removes the one-tick latency and puts each guest at exactly the right place; it
  also costs a fresh pooled instance per call (4.2 us on Cranelift, 5.2 us on Pulley,
  measured) up to a dozen times a tick, builds the observation several times,
  multiplies the ordering rules between modules, and multiplies a network client's
  rollback replay by the same count. One call with an event list is one instance,
  one observation, one order and one replay step per module per tick; the latency is
  stated, and a law that cannot bear it is a measured original's and belongs in the
  engine.
- **A persistent guest instance for the race** with state in its own memory. The
  cheapest call (0.36 us against 11.4 us a tick pooled) and the spike showed it
  diverging after a rollback. Snapshotting every guest's linear memory with `RaceSim`
  also works, but makes the snapshot scale with guest memory (a Rust guest's default
  stack alone is 1 MiB) and cannot capture non-exported mutable globals.
- **Give guests the `World` through a typed accessor API** (getters and setters per
  field). Finer-grained than commands and much larger: every field is a function to
  version and every setter a check to write. Commands name intents the host already
  has a safe path for.
- **Let guests write the handling table at `race-start`** (a "Feisar is 5 % faster"
  without per-tick commands). Rejected for the asset rule: the disc's numbers stay
  the disc's, and a per-tick scale inside a declared band is both re-derivable and
  visible. A one-shot table rewrite is also the first thing a cheat mod would do.
- **Epoch (wall-clock) interruption instead of fuel.** Not deterministic: the same
  module would trap on a slow machine and finish on a fast one, and two peers would
  desync. Fuel is reproducible; the spike measured the same trap iteration on both
  engines.
- **Clamp a bad command silently** instead of faulting the mod. Friendlier to the
  author, and a silent clamp is a behaviour nobody wrote down that differs by
  ceiling between builds. A fault is loud, deterministic and in the hash.

## Consequences

**Good.** A mod can carry the behaviours the maintainer listed without the engine
learning any of them by name; each is a few commands against inputs the tick already
takes. The sandbox is the same on every target and the fault path is one path. The
vanilla race is untouched by construction, so golden hashes and the determinism
references do not move. The harmless half (HUD) ships before the dangerous half
(sim). The non-capability list gives a mod author the "no" up front instead of at
review. The spike's numbers say the cost is affordable: about +19 % a tick on
Cranelift with 20 guest calls, against a 60 us tick.

**Bad.**

- **The command set is a ceiling.** A behaviour that needs a field the host does not
  expose waits for a host change, a schema edit and a release; mod authors will ask
  for more commands weekly. Three of the maintainer's own examples (a spring jump, a
  gravity switch, a custom weapon) need an engine change before their hook exists.
- **A one-tick latency on every event.** A craft that enters a pad on tick N boosts
  from tick N+1; a spring volume fires 17 ms after its edge. Stated, identical
  everywhere, and the reason an original's measured law can never be a mod.
- **The blob is small and laid out by the mod.** A layout change is a
  replay-breaking event, which is why the header carries a blob layout version.
- **A fresh pooled instance per tick costs 4.2 us (Cranelift) to 5.2 us (Pulley)**
  per module per tick, times the rollback window on a network client; a phone is
  slower on both sides and unmeasured.
- **The load-time rewrite is a compiler pass of our own** (fuel counter, NaN
  canonicalisation, call depth) that must be proven equivalent to wasmtime's built-in
  behaviour; until it exists the browser cannot match and a deeply recursive guest is
  non-deterministic in principle.
- **A custom weapon in v1 is a template shot, not a weapon of the mod's shape.** It
  cannot fly its own law or carry the disc's effects; an AI craft would not fire it.
- **Three schema edits in place** (`mod.tosd`'s `[extension]`, `track.tosd`'s
  `track.extension`, the replay header's `mods`) and a new glb node kind; fine before
  the first release, a versioning event after it.
- **MODDED is coarse in phase 2**: one sim module anywhere disables replays, ghosts
  and multiplayer for the session until phase 3's hashes land.
- **Cranelift adds about 11.3 MB to a binary**, hence the cargo feature; iOS is not
  checked at all until a macOS machine builds wasmtime for it.

**Revisit if:** the conformance test cannot show the browser or an AArch64 machine
matching Cranelift and Pulley (then the rewrite or the deterministic subset is
wrong, and the sim world waits); or if the command set grows past a few dozen
entries (then the typed accessor alternative deserves a second look); or if a
controller API for AI pilots lands (then it should share this host, the context
record and the limits, as the pilot-controller thread intends).
