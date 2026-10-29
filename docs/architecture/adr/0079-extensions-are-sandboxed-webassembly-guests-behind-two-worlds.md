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
[`docs/architecture/wasm-extensions/README.md`](../wasm-extensions/README.md).

## Context

The maintainer wants mods to carry **behaviour**, not only data: a craft with a
specialty (2048's prototype idea: Feisar faster on each speed pad), a track that
springs a jump or switches a craft to its underside, a race from A to B, a custom
weapon, a HUD widget. The decisions already taken (2026-10-12, the custom race modes
thread) are: custom modes come through mods, potentially WebAssembly; rules as data
first; wasmtime with Cranelift, Pulley on iOS, the browser's own engine on the web;
a phase with no simulation code, then the simulation hook marking the session
MODDED, then hashes in the replay header and the lobby. What was missing is the
**shape of the API**: who may extend what, through which functions, with what in and
what out, and the list of what is withheld, with reasons.

Four properties of this engine constrain any answer:

1. **Determinism is load-bearing** (`docs/architecture/determinism.md`): replays,
   golden tests, network play and the trace comparison all break on one
   non-deterministic bit. A guest is simulation code the moment it changes a
   `World` field.
2. **`World` is plain data with a layout that changes weekly** (ADR-0003,
   ADR-0055). Anything that reads it by offset or holds a pointer into it is broken
   by the next field.
3. **The simulation must not know a renderer exists**, and no gameplay crate may
   depend on `oag-render`, `oag-audio`, `oag-input` or `oag-game` (`just check-deps`).
4. **Never invent what the assets author**: the disc's tables, pads and splines are
   the law; a mod adds on top of them, it does not replace them.

## Decision

1. **Two WIT worlds, package `oag:ext@1.0.0`.** A **sim world** whose guest exports
   `race-start`, `tick` and `race-end` and imports a seeded random draw, its own
   save blob and a log line, and nothing else. A **HUD world** whose guest exports
   `frame` and imports the mod's strings, its own textures and a log line. No WASI,
   no other import exists; a module importing anything else fails at load by name.
2. **The sim world is copies in, bounded commands out.** The host builds one
   read-only `observation` per tick (every slot's pose, velocity, shield, timers, lap
   and progress; the person-flown slots' `InputSnapshot`; this tick's events such as
   `pad-crossed`, `lap-completed`, `weapon-fired`, `weapon-hit`, `volume-entered`)
   and hands it to each module in a fixed order. A module returns a list of
   commands; each command is **checked, clamped and applied by the host** into a
   field that is already an input to the tick (`pending_thrust_scale`,
   `pending_impulse`, the damage path, `Held`, the maglock's engage flag, the finish
   edge, a standing key, a HUD channel). No handle, pointer or shared memory crosses
   the boundary in either direction.
3. **Five extension kinds** bind modules to things that exist: a **craft** module to
   a team identity (optionally a variant) in `oag-mod.toml`; a **track** module to a
   track file, named by `track.extension` in its manifest; a **rules** module to a
   mod-declared mode (after rules as data); a **weapon** module to a pickup id the mod
   declares; a **HUD** module to a widget box in a HUD layout patch. All but HUD are
   sim-world and require `simulation = true`. One craft module per hull per race, one
   track module per track, one rules module per race; a second is left out and named,
   never stacked.
4. **The HUD world is not deterministic and reaches no state.** It reads a copy of
   `oag_hud::Readout` plus the mod's published keys, draws `fill`, `sprite` and
   `text` inside its box, and its output is appended to the HUD `Frame` after the
   title's own widgets. It is absent from every hash by construction, which is what
   lets it ship in phase 1.
5. **The non-capabilities are part of the contract**, each with its reason on the
   README: no files, network, clock, threads or WASI; no raw `World`; no renderer,
   GPU, camera or audio from the sim world; no input devices beyond the snapshot the
   tick received; no cross-mod memory; no persistence except the host-mediated blob;
   no effect on an original title's modes unless the mod is on and the session is
   MODDED; no rewriting of the disc's tables, pads or spline; no new projectile kinds
   in v1; no menu vocabulary; no AI control in this ADR; no unbounded work; no
   floating-point escape hatches; no access to the disc's archives; no grid changes.
6. **Limits are fixed and deterministic.** Fuel per `tick` (a number the spike
   settles), 16 MiB of memory, one table, a 4 MiB module, 64 commands per tick,
   NaN canonicalisation on, relaxed SIMD, threads, shared memory and SIMD off, no
   compilation cache, no parallel compilation. A trap or exhaustion turns the module
   off for the rest of the race on a tick that is itself hashed; the race goes on and
   the mod is named. A module that will not validate leaves its mod out of the race.
7. **The simulation crates gain no WebAssembly dependency.** The host is
   `oag-raceplay`, behind an `extensions` cargo feature; with zero modules loaded the
   host does nothing and a vanilla race's bytes are unchanged, which a test asserts.
   `oag-server` runs sim modules only in phase 3.
8. **Trust follows the networking decisions.** Online: the server runs the modules and
   its hash is the truth. LAN: the host is trusted. The content fingerprint folds in
   the SHA-256 of every sim module; the replay header's `options` carries
   `id@version:sha256` per mod; mod code never arrives through a lobby.
9. **Versioning is the pre-release rule**: `oag:ext@1.0.0` is edited in place until
   the first release; after it, additions stay in the package and a changed field is
   a second world beside the first. The manifest's `api` names the package version.
10. **Targets**: wasmtime with Cranelift on desktop and Android, Pulley on iOS; the
    web engine is the spike's choice between wasmtime-on-`wasm32` with Pulley (one
    proof) and the browser's `WebAssembly` with instrumented fuel (faster). Same
    guest bits everywhere is proven by the spike, not assumed.

## Alternatives considered

- **A scripting language (Lua, Rhai).** Easier to write; harder to keep
  bit-identical across four engines and three operating systems, and every host
  binding is a hand-written surface. The pilot-controller thread already recommended
  against it. WebAssembly gives the sandbox, the fuel meter and the deterministic
  float subset for free, and the same module runs on every target.
- **Native plugins (`dlopen`, cdylib).** No sandbox, no web, no iOS, and a crash in a
  mod is a crash in the game. Rejected outright.
- **One world for everything.** A HUD module would then need `simulation = true`
  and every HUD mod would mark the session MODDED, which is the opposite of phase 1's
  point. Two worlds let the harmless half ship first and keep the dangerous half
  small.
- **Give guests the `World` through a typed accessor API** (getters and setters per
  field). Finer-grained than commands and much larger: every field is a function to
  version and every setter a clamp to write. Commands name intents the host already
  has a safe path for, and the observation is one record to grow.
- **Let guests write the handling table at `race-start`** (a "Feisar is 5 % faster"
  without per-tick commands). Rejected for the asset rule: the disc's numbers stay
  the disc's, and a per-tick scale against them is both re-derivable and visible in
  the log. A one-shot table rewrite is also the first thing a cheat mod would do.
- **Callbacks per event instead of an event list in the observation.** More
  idiomatic in WIT, but N host-to-guest calls per tick cost fuel accounting and
  ordering guarantees; a list in a fixed engine order is one call and one order.
- **Run the sim world on a thread.** The simulation is single-threaded by rule;
  a guest tick inside the race tick is the only placement that keeps it so.
- **Epoch (wall-clock) interruption instead of fuel.** Not deterministic: the same
  module would trap on a slow machine and finish on a fast one, and the two peers
  would desync. Fuel costs a few percent of guest speed and is reproducible.

## Consequences

**Good.** A mod can carry the behaviours the maintainer listed without the engine
learning any of them by name; each is a few commands against inputs the tick already
takes. The sandbox is the same on every target. The vanilla race is untouched by
construction, so golden hashes and the determinism references do not move. The
harmless half (HUD) ships before the dangerous half (sim). The non-capability list
gives a mod author the "no" up front instead of at review.

**Bad.**

- **The command set is a ceiling.** A behaviour that needs a field the host does not
  expose waits for a host change, a schema edit and a release; mod authors will ask
  for more commands weekly. The README's open questions (a gravity frame, a
  projectile pool, sound) are the first three.
- **Guest memory becomes simulation state.** It must be hashed, snapshotted and
  restored with `RaceSim` for reconciliation and replays (the engine half's
  `determinism-and-state.md`); a 16 MiB cap on a 1.7 MiB snapshot is a real cost in
  the reconcile path, and the practical limit for a sim module will be far below the
  cap.
- **Fuel makes guests slower** and makes the tick budget on a Deck or a phone a
  design input for mod authors, who have never had one.
- **A custom weapon in v1 is a script, not a projectile.** It cannot fly, be drawn
  by `oag_fx` or play the disc's effects; players will notice.
- **Two more schema edits in place** (`mod.tosd`'s `[extension]`, `track.tosd`'s
  `track.extension`) and a new glb record; fine before the first release, a
  versioning event after it.
- **MODDED is coarse in phase 2**: one sim module anywhere disables replays, ghosts
  and multiplayer for the session until phase 3's hashes land.
- **Three engines to keep bit-identical** is a standing test burden, and the web
  candidate (a) makes the browser build larger and slower than (b) would.

**Revisit if:** the spike cannot show Cranelift and Pulley bit-identical with fuel
on (then the deterministic subset or the fuel scheme is wrong, and the sim world
waits); or if the command set grows past a few dozen entries (then the typed
accessor alternative deserves a second look); or if a controller API for AI pilots
lands (then it should share this host, the observation record and the limits, as
the pilot-controller thread intends).
