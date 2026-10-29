# Spike: a WebAssembly guest called from a simulation tick

A throwaway measurement for
[`docs/architecture/wasm-extensions/determinism-and-state.md`](../../docs/architecture/wasm-extensions/determinism-and-state.md).
Not engine code, not a workspace member: its own `[workspace]` table and its own
`Cargo.lock`, so the repository's `Cargo.lock` never sees wasmtime
(`git diff main -- Cargo.lock` is empty).

```sh
cd spikes/wasm-extensions
flock -o "$HOME/.cache/oag/gate.lock" ./run.sh
```

`run.sh` builds `guest/` for `wasm32-unknown-unknown` and `host/` natively, runs
the host on **Cranelift** and on **Pulley**, each **twice in separate
processes**, and diffs every output line but the `time` ones. Exit status 0 means
all four runs are bit-identical.

## What it runs

- **The guest** (`guest/src/lib.rs`, 1,654 bytes of wasm): "+5 % top speed per
  speed pad crossed, capped at +50 %", the shape of a craft-specialty mod. Core
  WebAssembly only (no component model, no WASI); one import, `oag.rng_next`, the
  host's seeded `oag_core::Rng`. Exports `on_pad_enter` (an event hook: changes
  the state blob, returns nothing) and `on_tick` (returns a top-speed scale the
  host validates to `0.5..=2.0`). The guest's persistent state is a 64-byte blob
  the host owns: copied in before each call, copied out after.
- **The host** (`host/src/main.rs`): a one-dimensional toy race, `f32` only, a
  ten-minute race (36,000 ticks at 60 Hz), throttle drawn from `oag_core::Rng`
  seed 42, four pads on a 1,000-unit loop. The race state, the generator and the
  blob are hashed with `oag_core::StateHasher`.
- **`nan_battery.wat`**: NaNs made several ways (0/0, sqrt(-1), inf-inf, a payload
  NaN through add, min, neg, promote/demote, signed zeros), run with Cranelift's
  NaN canonicalisation on and off.
- **`sizeprobe/`**: one trivial program built four ways to size the dependency.

Engine settings: `consume_fuel(true)`, `cranelift_nan_canonicalization(true)`,
`wasm_simd(false)`, `wasm_relaxed_simd(false)`, no `threads` cargo feature (a
module declaring shared memory does not compile). Pulley is selected with
`Config::target("pulley64")`.

## Results (2026-10-29, x86-64 Linux, rustc 1.99.0, load average about 2)

wasmtime **50.0.0**, cranelift-codegen 0.137.0, pulley-interpreter 50.0.0.
Licence read from the fetched crate's own `Cargo.toml`:
**`Apache-2.0 WITH LLVM-exception`**.

All four runs (Cranelift twice, Pulley twice, separate processes) identical:

```text
race fresh ticks=36000 hash=9c9a152ab63dfbcf pads=354 speed=4314f820 fuel=6059436 faulted=false
race persistent hash=9c9a152ab63dfbcf fuel=6059436
race starved fuel-per-call=1 hash=ddf4246fe9fef074 fuel=1 faulted=true
oracle native=8749c44ffaca9fe8 wasm-without-fuel=8749c44ffaca9fe8 equal=true
rollback fresh/on_tick straight=826952efcd381325 replayed=826952efcd381325 equal=true
rollback persistent/on_tick straight=826952efcd381325 replayed=826952efcd381325 equal=true
rollback persistent/on_tick_leaky straight=d843aa8f6084fbca replayed=102b878134e1409c equal=false
rollback fresh/on_tick_leaky straight=6e9c07b1fc75c339 replayed=6e9c07b1fc75c339 equal=true
fuel budget=1000 trap=Some(OutOfFuel) stopped_at=47 left=0
fuel budget=10000 trap=Some(OutOfFuel) stopped_at=475 left=0
nan canon=true 7fc00000 7fc00000 7fc00000 7fc00000 7fc00000 00000000 80000000 00000000 00000000 7ff80000 ffa00001 7fc00000
nan canon=false ffc00000 ffc00000 ffc00000 7fe00001 7fe00001 00000000 80000000 00000000 00000000 fff80000 ffa00001 7fe00001
```

What each line shows:

1. **Bit-identical across engines and runs** under fuel: same race hash, same
   fuel total (6,059,436 units, about 168 a tick), same 354 pads.
2. **Native oracle**: the same law written in host Rust gives the same hash as the
   guest (fuel left out of both). The guest adds nothing a Rust law would not.
3. **Rollback**: snapshot at tick 2,500 of 6,000, restore, replay. Equal for the
   stateless guest whatever the instance lifetime. **The leaky guest** (a counter
   in its own linear memory) **diverges under a persistent instance** and is
   repaired only by a fresh instance per call. A rollback restores what the host
   owns, never a guest's memory.
4. **Fuel**: a runaway loop traps `OutOfFuel` at the same iteration on both
   engines (47 at 1,000 units, 475 at 10,000). **Fuel is checked at function
   entry and loop back-edges, not per instruction**: the loop-free tick hook ran a
   whole race on 20 and on 100 units a call without trapping, identically on both
   engines; 1 unit trapped it at entry and the mod was faulted (the fallback).
5. **NaN canonicalisation works on both engines**: on, every arithmetic NaN is
   `7fc00000` (`7ff80000` for f64). Off, x86's default NaN is negative
   (`ffc00000`) and payloads propagate (`7fe00001`); an AArch64 machine makes
   `7fc00000` there instead, which is the cross-machine difference the setting
   removes. `f32.neg` keeps the payload (`ffa00001`) with it on or off: a bit
   operation the specification does not canonicalise, identical everywhere.
   **Pulley matches Cranelift even with canonicalisation off on this host**,
   because its interpreter runs the host CPU's float instructions; that does not
   carry to another architecture. This run checked one CPU; a cross-architecture
   run (AArch64, and a browser) is still owed.

Per call (not compared across runs; this host, release):

| | Cranelift | Pulley |
| --- | --- | --- |
| persistent instance | 0.36 us | 1.2 to 1.3 us |
| fresh instance per call | 9.5 us | 9.8 us |
| fresh, pooling allocator | 4.3 us | 5.3 us |
| compile the 1.6 KB guest | 1.7 ms | 1.6 to 1.9 ms |

Pulley's 3.4x per-call cost against Cranelift is the evidence it really
interpreted (both compile through Cranelift; Pulley to its bytecode).

Binary size (`sizeprobe`, release, stripped):

| build | bytes | delta |
| --- | --- | --- |
| no wasmtime | 336,704 | - |
| runtime + Pulley, no compiler (loads precompiled `.cwasm`) | 1,170,328 | +0.8 MB |
| runtime + Cranelift | 11,591,768 | +11.3 MB |
| runtime + Cranelift + Pulley | 11,702,200 | +11.4 MB |
