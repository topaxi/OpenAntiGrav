# WebAssembly extensions: determinism and state

**Status: proposal (2026-10-29), nothing is built.** The hooks are in
[hooks.md](hooks.md), the overview and capability surface in [README.md](README.md).
This page says how a mod's code can run inside the simulation without breaking what
[determinism.md](../determinism.md) guarantees, where its state lives, what replays
and network play need, and what the spike under `spikes/wasm-extensions/` measured.

Code citations are pinned to commit `b4b4cb3aa` (main, 2026-10-29) and name the
function, so they survive drift.

## The requirement

A simulation mod is part of the simulation. Everything
[determinism.md](../determinism.md) asks of `oag-physics` it asks of a guest:
bit-identical results on Linux, Windows, macOS, Android, iOS and the browser, on
x86-64 and AArch64, debug and release, and across the three engines the maintainer
chose (2026-10-12): **wasmtime with Cranelift** on desktop and Android, **wasmtime's
Pulley interpreter** on iOS (no JIT allowed there), and **the browser's own
`WebAssembly`** on the web build.

WebAssembly is a good fit because its core instruction set is already almost
deterministic: integer arithmetic, memory and control flow are exact, and every
`f32`/`f64` operation is IEEE-754 with round-to-nearest. **The exceptions are few,
and each has an answer below:** NaN bit patterns, relaxed SIMD, threads, resource
exhaustion (fuel, stack, memory), and anything imported.

## Floating point

- **`f32` arithmetic is exact IEEE-754** in core WebAssembly, including `sqrt`,
  `min`, `max`, conversions and rounding: the same set `oag-physics` restricts
  itself to. There is no fused multiply-add instruction in core WebAssembly, so the
  `mul_add` rule holds by construction.
- **Transcendentals are safe inside a guest.** A guest that calls `sin` compiles a
  software `libm` into its own module; those are integer and IEEE operations, so
  the result is bit-identical on every engine. That is the opposite of native code,
  where `check-transcendentals` must forbid platform `sin`. The rule for guests is
  therefore about *imports*: the host imports no math function.
- **NaN bits are the one non-determinism**, and the specification allows it: a NaN
  made by arithmetic may have any sign and payload the CPU produces. x86 makes a
  negative default NaN, AArch64 a positive one, and x86 propagates payloads. A NaN
  that reaches state (the blob) or the hash would split machines.
  - **wasmtime: `Config::cranelift_nan_canonicalization(true)`** rewrites every
    arithmetic NaN to the canonical `0x7fc00000` (`0x7ff8000000000000` for `f64`).
    It applies to Pulley too, since Pulley's bytecode comes from Cranelift.
    Measured in the spike (below): on, every arithmetic NaN read `7fc00000` on both
    engines; off, this x86 host made `ffc00000` and kept a payload (`7fe00001`).
    Bit operations (`neg`, `abs`, `copysign`, reinterpret) keep their payload with
    it on or off, as the specification says; they are exact everywhere already.
  - **The browser has no such switch.** So the web host must get canonical NaNs
    another way: a **load-time rewrite of the module** that inserts the same
    canonicalisation after every float operation (the transformation Cranelift's
    pass does, applied to the bytes before any engine sees them). Applied on every
    platform, it makes the three engines run the same module; wasmtime's own switch
    stays on as a second line. Not built; it needs the conformance test below.
  - **Outputs are checked regardless.** A NaN in any command field is a fault
    ([hooks.md](hooks.md#the-shape-one-call-per-tick-events-in-commands-out)), never forwarded.
- **SIMD.** `relaxed_simd` is refused at load (its results are
  implementation-defined by design; it includes a fused multiply-add). Plain
  `simd128` is deterministic except NaN bits, which the canonicalisation also covers
  for `v128`. **Start with SIMD refused** (the spike ran with it off) and allow
  `simd128` only after the conformance test passes with it.

## Fuel, not time

A guest is bounded by **fuel**, never by wall time: a timeout would fire at a
different instruction on a faster machine. wasmtime's `Config::consume_fuel(true)`
counts WebAssembly operations; the host sets a budget before each call
(`Store::set_fuel`) and an exhausted budget traps `OutOfFuel`, which faults the mod
deterministically ([hooks.md](hooks.md#the-shape-one-call-per-tick-events-in-commands-out)).

Measured in the spike:

- **The trap point is identical on Cranelift and Pulley.** A runaway loop stopped
  at iteration 47 under 1,000 units and 475 under 10,000, on both engines.
- **The fuel total is identical too**: 6,059,436 units over a 36,000-tick race on
  both, about 168 a tick.
- **Fuel is checked at function entry and loop headers, not per instruction.**
  wasmtime's translator emits the check in `fuel_function_entry` and
  `translate_loop_header` (crate `wasmtime-internal-cranelift` 50.0.0,
  `src/func_environ.rs`); between checks it only accumulates the cost. The spike
  shows the consequence: the loop-free tick hook ran a whole race on budgets of 20
  and of 100 units without trapping (identically on both engines); a budget of 1
  trapped at entry. So the budget bounds loops and calls, and straight-line code may
  overrun it by its own length. Deterministic, but **the budget is not a precise
  instruction count**.
- **wasmtime's fuel count is not state.** Its per-operator costs are wasmtime's own
  (configurable through `Config::operator_cost`) and the browser has no equivalent,
  so a fuel total must never reach the state hash or a replay: only **the fault
  edge** does (the mod's fault flag and the tick it faulted on). The spike hashed
  its fuel total, which is fine for comparing two wasmtime engines and nothing
  more.
- **The browser has no fuel.** For the web host to fault at the same tick, the same
  metering has to run there, which means **instrumenting the module at load** (a
  counter decremented at function entries and loop headers, trapping at zero) rather
  than relying on wasmtime's built-in fuel. Using that instrumentation on every
  engine, with wasmtime's fuel as a backstop set higher, gives one metering scheme
  everywhere. Not built; it rides with the NaN rewrite.

Two more exhaustion cases:

- **Memory.** A guest declares its maximum memory in its manifest; the host sets the
  same `StoreLimits` on every engine, so `memory.grow` fails at the same size
  everywhere.
- **Stack depth is the open risk.** A stack overflow traps where the *native*
  stack runs out, which differs between Cranelift's code and Pulley's interpreter.
  Fuel does not bound recursion depth tightly enough to rule it out. The answer is
  the same instrumentation pass: a call-depth counter that traps at a fixed depth
  well below any engine's native limit. Until it exists, a guest that recurses
  deeply is non-deterministic in principle.

## No threads, no clock, no WASI

- **Threads.** The host's wasmtime is built without its `threads` feature, so a
  module declaring shared memory fails to compile; the spike's host was built this
  way. Atomics and `wasi-threads` are refused by name.
- **No clock, no files, no network, no environment.** The host gives the guest **no
  WASI at all**: the linker holds one import namespace, `oag`, and a module importing
  anything else fails to instantiate and is refused at load, naming the import. The
  spike's guest imports exactly `oag.rng_next`.
- **Randomness comes from the host.** The guest has no generator of its own and no
  entropy. `oag.rng_next` returns the next value of a host `oag_core::Rng`
  (`crates/core/src/rng.rs:28`, xoshiro128**, 16 bytes of state). **One stream per
  module** (README: a child stream seeded from the race seed and the module's load
  index; the load order is fixed by mod id), stored beside the module's blob, so a mod drawing numbers never shifts the game's own stream
  (`World::rng`, `crates/gameplay/src/world.rs:309`), which AI and weapon rolls read.
  (The spike shared its one toy stream; that is enough to show the draws are
  deterministic, not to show the separation.)

## Mod state is a fixed-size blob in the race state

[ADR-0003](../adr/0003-no-ecs.md): gameplay state is plain data in fixed-size
arrays, so a snapshot is one copy. A mod's state follows that rule:

- **Each loaded simulation mod gets one fixed-size blob** (a size the manifest
  declares, capped by the host: a few hundred bytes per craft slot plus a global
  part is enough for every example in [hooks.md](hooks.md)), its own `Rng` stream
  and its fault flag, in a fixed array in the simulation state:
  `[ModSlot; MAX_SIM_MODS]`. No `Vec`, no pointer, no host-side map.
- **Where:** in `World` (`crates/gameplay/src/world.rs:304`) when it is gameplay
  state every crate may read, or in `RaceSim` (`crates/raceplay/src/sim.rs:57`)
  beside the pad caches. Either way `Race::snapshot_sim` / `Race::restore_sim`
  (`crates/raceplay/src/reconcile.rs:32`, `:49`) clone and restore it with
  everything else, which is what network rollback already uses.
- **The guest's own memory is not state.** A rollback restores what the host owns,
  never a guest's linear memory. The spike proved the failure: a guest keeping a
  counter in its own memory (`on_tick_leaky`) **diverged after a rollback with a
  persistent instance** (`d843aa8f6084fbca` straight against `102b878134e1409c`
  replayed), and matched with a fresh instance. So the host enforces it by
  construction rather than by asking guest authors to behave:

  | Instance lifetime | Rollback-safe | Cost (Cranelift / Pulley) |
  | --- | --- | --- |
  | One for the race | **No**, unless the guest is stateless | 0.36 / 1.2 us a call |
  | Fresh per call | Yes | 9.5 / 9.8 us a call; 4.2 / 5.2 us pooled |
  | **Fresh per tick, pooled, every call that tick on it** | **Yes**: at a tick boundary the guest's memory is always the module's initial image | **4.2 / 5.2 us a tick for the one `tick` call**; 11.4 / 27.3 us for 20 calls |

  **Recommended: one pooled instance per module per tick** (wasmtime's pooling
  allocator resets memory copy-on-write). With the README's ABI, one `tick` call per
  module per tick ([hooks.md](hooks.md#the-shape-one-call-per-tick-events-in-commands-out)),
  that is a fresh pooled instance per call. What crosses a tick boundary is the
  declared state blob alone, passed in with the observation and returned with the
  commands; the between-races save blob (README) is a separate thing. The alternative, snapshotting
  each guest's whole linear memory with `RaceSim`, also works but makes a snapshot
  cost scale with guest memory (a Rust guest's default is 1 MiB of stack alone), and
  must also capture non-exported mutable globals, which wasmtime does not expose.

### The state hash

`oag_gameplay::hash::write_world` (`crates/gameplay/src/hash.rs:62`) destructures
`World` exhaustively, so a new field cannot be forgotten by the hash; the race adds
its own fields in `Race::state_hash` (`crates/raceplay/src/hash.rs:46`). The mod
slots are hashed there: blob bytes, `Rng` state, and the fault flag with its
tick. Not the fuel used (see [Fuel, not time](#fuel-not-time)).

**A race with no simulation mod writes nothing for them**, so every vanilla hash,
including `crates/core/tests/determinism.rs`'s committed references, stays
bit-identical. Those references are never edited (CLAUDE.md); a design that moved
them would be wrong.

## Replays

[ADR-0055](../adr/0055-replays-are-inputs-and-a-ghost-is-poses.md): a replay is the
inputs plus a state hash at an interval, our own format, verified by re-simulating.
A mod's code is part of what re-simulates, so a replay must name it:

- **Today the header has no mod field** (`oag_replay::Header`,
  `crates/replay/src/header.rs:20`); `options` is an opaque string map for the
  composition root and `build` is informational, never compared.
- **Proposal (agreed with the README): a `mods` key in `Header::options`**
  (`header.rs:58`), listing each simulation module's mod id, version and the SHA-256
  of its bytes. `options` is opaque to `oag-replay`, so the composition root writes
  it and refuses playback, by name, unless the identical modules are installed and
  on; the stored hashes still decide validity. Nothing is released, so format v1 is
  edited in place if a dedicated field is wanted later.
- **The phases decided on 2026-10-12** come first: in phase 2 a session with a
  simulation mod is **modded** and records no replays or ghosts at all; the header
  field is phase 3, once the conformance test below holds.

## Network play

[ADR-0070](../adr/0070-the-network-transport-is-webtransport.md) and
[docs/networking](../../networking/README.md): online play is server-authoritative,
the client predicts and reconciles by replay, and "mods may change anything,
simulation and race modes included" (the 2026-10-13 decisions; "Mod code arriving
through a lobby is never auto-run").

- **The server runs the mods.** It is authoritative, so it simulates with the same
  modules; `oag-server` links the host (wasmtime is not a renderer; rule 6 is
  unaffected).
- **The lobby already refuses differing mods**: `Edition::content`
  (`crates/net/src/wire/online.rs:34`) is a fingerprint of the mods a machine runs,
  and `content_refusal` (`crates/net/src/online/rooms.rs:288`) refuses "your mods
  differ from the server's". **But the fingerprint is FNV-1a over `id@version`**
  (`oag_net::discovery::content_fingerprint`, `crates/net/src/discovery.rs:168`, fed
  by `crates/game/src/mods.rs:446`), not over the module's bytes: two different
  builds of one mod version would be admitted together and desync. Phase 3 folds
  each simulation module's SHA-256 into that fingerprint.
- **Rollback multiplies the guest's cost.** A client replays from the server's
  confirmed tick, at most `MAX_WINDOW = 30` ticks (`crates/net/src/client.rs:71`),
  and measured corrections replay 7 to 11 ticks
  ([networking](../../networking/README.md), "Replay cost on this machine"). A race
  tick costs about 60 us sim-only on the desktop that measured it. With the
  recommended per-tick instance, one `tick` call per module, and **three simulation
  modules** (a craft, a track and a rules mod; 3 x 4.2 us on Cranelift, 3 x 5.2 us on
  Pulley, a light guest):

  | | Vanilla | + 3 modules, Cranelift | + 3 modules, Pulley |
  | --- | --- | --- | --- |
  | One tick | 60 us | 73 us (+21 %) | 76 us (+26 %) |
  | A typical correction, 11 ticks | 0.66 ms | 0.80 ms | 0.83 ms |
  | The worst case, 30 ticks | 1.8 ms | 2.2 ms | 2.3 ms |

  The guest's own work is on top: the spike's guest is a few hundred operations a
  call, and a heavy module (a controller reading every craft) runs 3.4 times slower
  on Pulley than on Cranelift (the persistent-instance row).

  Desktop numbers from two different runs on one shared machine, so an estimate,
  not a measurement of the combination. A phone is slower on both sides; the S24
  half of the networking measurements is still open, and a Pulley phone (iOS) is the
  case to measure first.

## Dependency rules

- **The host is its own crate**, proposed `oag-modhost`: the wasmtime engine and
  its configuration, the I/O buffer ABI, fuel, the fault path, the module checks at
  load. It knows no game and no title, and depends on wasmtime and `oag-core` (for
  `Rng`). It does no float arithmetic of its own beyond range checks, and it joins
  `check-transcendentals`' scanned crates (`scripts/check-transcendentals.py`,
  `SCANNED_CRATES`), which today do not include `oag-raceplay`.
- **The simulation call (and its event buffer and command sinks) is in
  `oag-raceplay`'s simulation half**, where `Race::tick` lives. No `GAMEPLAY_CRATES` member
  (`scripts/check-dependency-rules.py`) depends on the host: `oag-physics`,
  `oag-race`, `oag-weapons` and `oag-ai` see only plain data (`Environment` fields,
  rules), which keeps their determinism scan and their wasm32
  test run (`just test-wasm`) meaningful.
- **The HUD host is render side**: the `hud` world runs in `oag-game`'s HUD overlay,
  through the same crate (phase 1).
- **`oag-server` links the host and no renderer** (rule 6), from phase 3, when the
  lobby carries module hashes: the host depends on nothing that draws.
- **Behind a cargo feature** (the README's `extensions`). wasmtime with Cranelift
  adds about 11.3 MB to a binary (spike, below).

## Web and iOS

- **Web: open (the README's open question 1).** The game itself is a wasm32 module
  in the browser (`docs/tools/web.md`). Either (a) wasmtime built for `wasm32` with
  Pulley as its only backend, the guest interpreted inside our module: one host, one
  proof, wasmtime's NaN canonicalisation and fuel included, if wasmtime builds for
  that target (not checked); or (b) the browser's own `WebAssembly`, compiled
  asynchronously at race load, never per tick: faster, but the browser has neither
  NaN canonicalisation nor fuel, so both move into a load-time rewrite of the module
  (above), and the component model the README sketches does not run natively in a
  browser (a guest component would be transpiled to a core module first). The spike
  used a core-WebAssembly I/O buffer, which runs on all three; it does not decide
  the interface language.
- **iOS.** No executable memory, so no Cranelift-to-native. Two options:
  - **Cranelift compiling to Pulley bytecode on the device** (no native code is
    generated, which is what the spike's Pulley runs did on x86): one code path,
    +11.4 MB.
  - **Runtime and Pulley only, +0.8 MB**, with modules precompiled to `.cwasm`
    elsewhere. A `.cwasm` is tied to the exact wasmtime version and configuration,
    so mods would ship per build; brittle.

  Recommended: the first. Pulley costs 3.4 times Cranelift per call (spike).

  **Not checked on iOS.** `cargo check --target aarch64-apple-ios` of wasmtime 50
  (Pulley only, and with Cranelift) stops in a build script that needs Apple's
  `xcrun` (`cc-rs`: `failed to find tool "xcrun"`), the same macOS-only toolchain
  `just ios-app` needs. So whether the crates build for iOS, and whether
  `Config::signals_based_traps` must be off there (it registers signal handlers by
  default; wasmtime's own doc names it for environments that cannot), is owed to a
  macOS machine.

## The conformance test

The claim that three engines agree is only as good as a test that runs one module on
all three and compares. Proposed, as in the 2026-10-12 plan: a `#[test]` running the
same guest and input trace on Cranelift and Pulley, plus a headless-browser run
(`wasm-bindgen-test`), comparing the state hash at every tick, with **a NaN battery
and a fuel-exhaustion case in it** (the spike's two), and the native Rust law as an
oracle where one exists (Pilot Assist on Pulse and Pure). It must also run on an
AArch64 machine, where the default NaN differs.

## The spike

`spikes/wasm-extensions/` is a standalone cargo project with its own `[workspace]`
table: not a workspace member, its own lock file, and the repository's `Cargo.lock`
untouched. Run it with `flock -o "$HOME/.cache/oag/gate.lock" ./run.sh` from that
directory; its README carries the raw output.

**What it is.** A one-dimensional toy race (`f32` only, a speed, a distance on a
1,000-unit loop, four speed pads, throttle from `oag_core::Rng` seed 42, 36,000 ticks
= ten minutes at 60 Hz), hashed with `oag_core::StateHasher`. The guest is "+5 % top
speed per speed pad crossed, capped at +50 %", 1,654 bytes, compiled from Rust for
`wasm32-unknown-unknown`, core WebAssembly only: a `pad-crossed` event and a
`scale-thrust` command in miniature (the spike calls the pad edge as an export of its
own, the same tick), with a 64-byte host-owned blob. Engine: wasmtime **50.0.0**
(cranelift-codegen 0.137.0, pulley-interpreter 50.0.0), licence **Apache-2.0 WITH
LLVM-exception** (read from the crate's own `Cargo.toml`), with fuel on, NaN
canonicalisation on, SIMD and relaxed SIMD off, no `threads` feature, no WASI.

**Results** (2026-10-29, x86-64 Linux, rustc 1.99.0, load average about 2):

| Check | Result |
| --- | --- |
| Cranelift twice and Pulley twice, four separate processes | **All output identical**: race hash `9c9a152ab63dfbcf`, 354 pads, fuel 6,059,436 |
| Native oracle: the same law in host Rust | **Same hash** as the guest (`8749c44ffaca9fe8`, fuel left out of both) |
| Rollback at tick 2,500 of 6,000, stateless guest | Equal, whatever the instance lifetime |
| Rollback, guest with state in its own memory | **Diverges** with a persistent instance; equal per call and per tick |
| Out of fuel | Same trap iteration on both engines (47 / 475) |
| Starved race (1 unit a call) | Faults at the first call, race runs on; same hash on both |
| NaN battery, canonicalisation on | Every arithmetic NaN `7fc00000` (`f64`: `7ff80000`) on both |
| NaN battery, canonicalisation off | `ffc00000` and payload `7fe00001` on this x86 host, on both engines |

**Per call and per tick** (release; wall clock measured by the spike itself, not by
anything simulated):

| | Cranelift | Pulley |
| --- | --- | --- |
| Persistent instance, one call | 0.36 us | 1.2 us |
| Fresh instance, one call | 9.5 us | 9.8 us |
| Fresh pooled instance, one call | 4.2 us | 5.2 us |
| **Pooled instance per tick, 20 calls** | **11.4 us** | **27.3 us** |
| Compile the 1.6 KB guest | 1.6 to 2.1 ms | 1.5 to 1.9 ms |

**Binary size** (a trivial program, release, stripped): 336,704 bytes without
wasmtime; +0.8 MB with the runtime and Pulley only; **+11.3 MB with Cranelift**;
+11.4 MB with both.

**What it does not show.**

- **Cross-architecture agreement.** One x86 CPU ran all four. With
  canonicalisation off, Pulley still matched Cranelift here because its interpreter
  uses the host CPU's float instructions: that agreement says nothing about an
  AArch64 machine. Owed: the same run on AArch64 (the Android target), and a
  browser run.
- **The browser.** Not run; it needs the load-time rewrite above for NaNs and fuel.
- **The stack-depth case** (no recursive guest was run).
- **Real hook sites.** A toy tick, not `Race::tick`; the costs are of the guest call
  mechanism, not of copying a full observation.
