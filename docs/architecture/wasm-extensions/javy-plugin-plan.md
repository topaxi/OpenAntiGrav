# A JavaScript extension's fixed fuel floor: the plan to cut it

**Status: a plan with a measured prototype, nothing built** (2026-11-17). It follows
[the JavaScript section of the README](README.md#and-a-language-other-than-rust-javascript-through-javy-2026-10-30-ported-to-abi-1-revision-6-on-2026-11-16),
which ends on "evaluating the script inside the Wizer snapshot, or a custom plugin, is the
next lever". This page decides between the levers, with numbers, and lists the steps a wire
lane executes without re-deriving anything.

**Outcome.** The floor is not QuickJS and it is not the streams alone: it is what Javy does on
every call. A guest that (1) evaluates the script once, inside the Wizer snapshot, and (2)
maps the host's I/O buffer into JavaScript as an `ArrayBuffer`, costs **421 fuel for an empty
tick function** (Javy: 29,842) and runs Wall Penalty's empty tick in **13,426** (Javy:
110,770), with no host change, no ABI change and no new import. The prototype passes the
differential test on 20,000 observations and all five `wall_penalty_ground_truth` tests.
The target was a floor under 50,000.

The prototype is not a Javy plugin and does not use Javy at all. Section 2 says why that is
the better trade; the page keeps its name because the question was asked as "a custom Javy
plugin".

## 1. Where the floor goes

### Today (Javy 9.1.0, branch `javy-rebase`, 110,770 fuel on an empty tick)

The parts were measured one at a time on 2026-11-16 (the README's "Start-up per tick"); the
last column is new.

| Part | Fuel | How it is known |
| --- | --- | --- |
| Javy's `invoke` of an empty exported function | 29,842 | measured: a script that is only `function jsTick() {}` |
| of which the QuickJS call itself | at most 421 | measured here: the prototype's whole empty tick, `oag_io` included |
| `Javy.IO.readSync`, 88 bytes | about 18,000 | measured, one at a time |
| `Javy.IO.writeSync`, 220 bytes | about 20,500 | measured, one at a time |
| Top level: two `new Uint8Array` | about 10,800 | measured at about 5,400 each |
| Logic: eight idle slots | about 14,400 | measured at about 1,800 a slot |
| Not separately measured | about 17,200 | the remainder: `out.set` of an eight-element literal, the two header reads, the count write |

So about 29,400 of the "30,000 to enter and leave QuickJS" is not QuickJS. Javy's
`invoke` (`crates/plugin-api/src/lib.rs` at tag `v9.1.0`) does, on every call:
`Module::load(bytecode)`, `module.eval()`, a promise check, `module.get(fn_name)`, the call,
then the pending-job loop. Javy's Wizer snapshot is taken of the plugin before the bytecode
is attached, so none of that can be in it. (Confidence 90: the source reads so, and the
prototype's 421 is the same engine, rquickjs 0.12, without those steps.)

### With the prototype (2026-11-17)

Measured with `oag-modhost` as it stands on this branch (wasmtime `=50.0.2`, the host's own
engine configuration, quick profile), through a copy of
`crates/modhost/tests/javascript_module.rs` reading the module's path from the environment,
and `wall_penalty_ground_truth` with the module put in the example's place.

| | Javy (`javy-rebase`) | Prototype, `DataView` script | Prototype, typed-array views |
| --- | --- | --- | --- |
| Empty tick function | 29,842 | 421 | 421 |
| Wall Penalty, nothing to do | 110,770 | 30,870 | **13,426** |
| Contact tick (one new curse) | 121,833 | 45,756 | 25,250 |
| Cursed tick (one craft) | not printed | 42,182 | 21,925 |
| Race, 900 ticks: peak / mean | 121,833 / 114,571 | not run | **31,426 / 17,862** |
| 12-craft field: peak | 131,543 | not run | **40,281**, no fault, no divergence |
| Every slot cursed and puffing (16 commands) | 188,451 | 116,085 | **95,854** |
| 16 events, 8 new curses, 8 commands | not measured | 120,966 | 98,731 |
| Differential, 20,000 observations | equal | equal | equal |
| `wall_penalty_ground_truth` | 5 of 5 | not run | 5 of 5 |
| Module size | 1,371,786 bytes | 1,150,692 | 1,149,592 |
| Memory at instantiation | 22 pages | 20 pages | 20 pages, none grown |
| Call time in the race, debug host, mean | 105 us | not run | 42.9 us (Rust 19.8 us) |

What is left is the script's own work, at QuickJS's rate under wasm: an idle slot about
1,600, a command written about 5,000. The two script columns differ only in how the buffer
is read: `view.getUint16(at, true)` is a method call, `curses[slot]` on a `Uint16Array`
made at the top level is one indexed read. Views are free now (they are made once, in the
snapshot), so a script should make one per region it reads.

## 2. The options, ranked

| # | Option | Empty tick function | Wall Penalty, nothing to do | Work | Risk | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| A | **Our own QuickJS guest runtime: script evaluated in the snapshot, I/O buffer mapped** | 421 (measured) | 13,426 (measured) | small to medium: about 80 lines of Rust and a build script, both prototyped | low: prototyped end to end | **do this** |
| B | Custom Javy plugin with direct buffer access, script still evaluated per call (NOTES 1c as first written) | about 29,800 (Javy's `invoke` stays) | estimated 45,000 to 60,000 | medium: a plugin on `javy-plugin-api`, a rebuilt no-SIMD Javy CLI (7.5 min a build), the shim kept for the WASI imports | medium: lands near the target, not under it with room | no |
| C | Fork Javy's code generation so the bytecode is evaluated before the snapshot, streams kept | a few hundred | estimated 50,000 or more (read about 18,000 and write about 20,500 stay) | large: a fork of `javy-codegen` and the plugin API to carry | high: a fork to keep in step with upstream | no |
| D | A per-module fuel budget (`[extension] fuel = n`) | unchanged | unchanged | medium, needs an ADR | low | not needed for this; it does not cut the floor |
| E | Keep one instance alive across ticks | small | small | host change | breaks rollback and the "nothing crosses a tick but the state blob" rule | refused |

B and C's Wall Penalty numbers are estimates from the measured parts above, not runs.

**Why not stay a Javy plugin.** Javy gave three things. QuickJS built for wasm: that is
`rquickjs`, the crate Javy itself builds on (Javy 9.1.0 pins `rquickjs 0.12.0`; the
prototype uses the same). The snapshot: that is Wizer, which now ships in wasmtime
(`wasmtime wizer`, crate `wasmtime-wizer`), the engine the host already pins. The streams:
those are what option A removes. A Javy plugin would add a rebuilt Javy CLI to every build
and still need a second Wizer pass over Javy's output to get the script into the snapshot,
because Javy attaches the bytecode after its own snapshot. Nothing of Javy would be left
doing work.

**The host's model stays exactly as it is.** One fresh pooled instance per tick, reset by
copy-on-write from the module's memory image. The snapshot is that image, so every tick
starts from the same bytes: the evaluated top level, with the I/O buffer zeroed. Whatever a
tick writes to a top-level variable is gone on the next tick, as it is today (today because
the top level runs again, with A because the image is restored). The state blob is still
the only thing that crosses a tick. The one visible difference: top-level code runs **once,
at build time**, not once per tick. A script that computed something at the top level from
`Math.random` or `Date` got the snapshot's frozen values per tick before, and gets the same
frozen values now.

## 3. The design

### The runtime (a guest crate)

One Rust crate, `cdylib`, target `wasm32-wasip1`, depending on `rquickjs` and
`oag-ext-abi` (for `IO_LEN` and `ABI_WORD`). It owns a `static` I/O buffer of `IO_LEN`
bytes and exports what the ABI names:

| Export | What it does |
| --- | --- |
| `wizer-initialize` | Build time only, removed by Wizer. Reads the script from standard input, builds the QuickJS runtime and context, sets the global `oag.buffer` to an `ArrayBuffer` **over the static I/O buffer** (`JS_NewArrayBuffer` with no free function, so no copy), evaluates the script as a module, and keeps the module's exported `tick` function and its `stateBytes` number in statics |
| `oag_abi` | `oag_ext_abi::ABI_WORD` |
| `oag_io` | the static buffer's address |
| `oag_state_len` | the script's `stateBytes` |
| `tick` | `JS_Call` of the kept function; 0, or 1 when it threw |
| `memory` | the linker's |

The host writes the observation into the buffer, calls `tick`, and reads the reply and the
state from the same buffer. The script reads and writes those bytes in place through views
it made at the top level. No stream, no copy, no shim logic.

### The script's contract

```js
const curses = new Uint16Array(oag.buffer, 544, 8);   // STATE_AT, made once
export const stateBytes = 16;
export function tick() { /* read and write oag.buffer in place */ }
```

- `oag.buffer` is the whole I/O buffer, at the ABI's own offsets. The generated typings
  (`types/oag-abi.d.ts`) describe exactly this layout, so they finally apply as written.
- Top-level `const`s cost nothing per tick. The `// abi NAME = n` comment form the port
  needed can go back to plain `const NAME = n;`, which `scripts/gen-ext-typings.py --check`
  also compares with the ABI crate.
- The reply's string pool must be written inside `tick`: the host overwrites the whole
  buffer before each call, so nothing written at the top level survives in it.
- `tick` is synchronous. The prototype runs no promise job loop.

### The build

Four steps, all prototyped (`wasmtime` CLI 50.0.1, binaryen's `wasm-dis`/`wasm-as`, rustc
1.99.0 with `wasm32-wasip1`; the runtime builds in 30 s from cold):

1. `cargo build --release --target wasm32-wasip1` of the runtime. `rquickjs-sys` fetches
   wasi-sdk 24 into its `OUT_DIR` on the first build unless `WASI_SDK` points at one.
2. **Before the snapshot**, turn the `clock_time_get` and `random_get` imports into
   functions inside the module (0 and zero bytes). Without this the build is not
   reproducible: two builds of one script differed from byte 993,014 on. With it they are
   byte-identical.
3. `wasmtime wizer -S cli=y runtime.wasm -o snapshot.wasm < script.js`.
4. **After the snapshot**, turn the remaining eight WASI imports into functions that fail
   (`proc_exit` is `unreachable`), as `tools/shim.py` does today, minus all its stream
   logic. The result imports nothing and the host loads it as it loads any module.

Steps 1 and 2 depend on the runtime alone, so their output can be built once and shipped;
a mod author's build is steps 3 and 4.

## 4. Determinism and identity

- **Identity is unchanged.** The module is one self-contained file, so the SHA-256 of its
  bytes, already in the replay header's `mods` list
  ([determinism-and-state.md](determinism-and-state.md#replays)) and due to join the lobby
  fingerprint, covers the runtime, the script and the snapshot together. Nothing new joins
  either. (Confidence 90.)
- **The build is reproducible** once the clock and the randomness are fixed before the
  snapshot (measured: two builds, `cmp` equal). A replay names the module by hash, so two
  machines that build the same script must get the same bytes; step 2 above is what makes
  that true. (Confidence 85: measured on one machine and one toolchain; a different rustc
  or wasi-sdk gives other bytes, as for any guest.)
- **No SIMD.** The host's engine, with `wasm_simd(false)`, compiled the prototype. `rquickjs`
  built for `wasm32-wasip1` with default flags emits none; no `-simd128` flag was needed.
  (Confidence 90: the compile is the proof; a later rquickjs could change its flags.)
- **NaN canonicalisation still holds.** The prototype ran under the host's
  `cranelift_nan_canonicalization(true)` and matched Rust on 20,000 observations and a
  900-tick race. QuickJS's value boxing on wasm32 moves bit patterns through integer
  operations and memory, which canonicalisation does not touch. (Confidence 85: shown by
  the tests for this script; a script doing heavy float work is not covered by them.)
- **Fuel is a function of the bytes and the observation**, as before: the instance is the
  same image every tick, and QuickJS's allocator and collector are driven by the
  instruction stream alone. (Confidence 85.)
- **`Math.random` and `Date`.** Seeded and read once, at build time, from a clock that step
  2 fixes at 0: every tick sees the same sequence. Deterministic, and useless to a script.
  The wire lane removes both from the global object in `wizer-initialize` so a script fails
  loudly instead of drawing the same "random" number every tick. Chosen, not measured.
- **JavaScript numbers are `f64`.** Unchanged from the README's finding: a factor computed
  in `f64` and stored as `f32` is deterministic, but the SDK's "no `f64` that feeds a
  command" rule cannot be enforced on a script.
- **The browser.** Not tested, as before: nothing here ran a JavaScript module on the web
  build's engine. Unsettled; a run of the differential test under `just test-wasm` would
  settle it.

## 5. How this fits "ship Javy from our side" (the 1b list)

The runtime and the script contract above are the parts 1b keeps. Three ways to deliver
them, in order of how much they change; each builds on the one before.

1. **Tooling only: no host change, no ADR.** The engine ships the runtime already through
   build steps 1 and 2, and one command does steps 3 and 4. In Rust that is the
   `wasmtime-wizer` crate at the host's pinned version (`wasmtime-wizer 50.0.2` exists on
   crates.io, checked with `cargo info`) with our own instantiation, so the clock, the
   randomness and the script's bytes come from host functions we write, and a rewrite that
   drops the import section and puts the stub bodies first in the function and code
   sections (every import is replaced, so no function index moves). A mod author then needs
   neither Rust, wasi-sdk, binaryen nor Javy. The mod still ships a 1.15 MB module.
2. **Compile QuickJS once per process.** Measured on two snapshots of different scripts:
   **every section but the data is byte-identical** (code 932,274 bytes, the same SHA-256;
   also type, function, table, memory, global, export and element; data 203,372 against
   213,806 bytes). So "the provider" of 1b is this runtime's code, and what differs per mod
   is a memory image of about 200 KB plus the script's own heap, not bytecode. Either a
   compiled-code cache keyed on the non-data sections, or a split into a code module that
   imports its memory and an image module that exports it, shares the 1 s to 2 s Cranelift
   compile across every JavaScript mod. The global section is equal too, so the split needs
   no imported global. Both change the host (an import the host satisfies, two instances
   per call, or a cache where README 4.1 says "Compilation cache: off"): **an ADR**.
   (Confidence 60 that the split works as sketched: the section equality is measured, the
   split itself is not built.)
3. **A mod ships only its script.** The host runs build steps 3 and 4 at race load.
   Identity becomes the runtime's hash plus the script's hash, which is where 1b's
   "provider identity joins the fingerprint" applies. It puts a JavaScript parser in the
   load path and a second identity component in the replay header: **an ADR**. Not
   prototyped.

Javy's own dynamic linking (`-C dynamic`) is not needed by any of the three. The per-module
fuel budget (1b item 5) is independent and still needs its ADR if it is wanted; with the
floor at 421 it is no longer what stands between a JavaScript mod and the budget.

## 6. Steps for the wire lane

In order. The lane works on the Javy branch and the `assets/mods/examples` submodule branch
of the same name. It changes no host code: the only `.rs` file it touches is a test.

| # | Step | Files | Acceptance |
| --- | --- | --- | --- |
| 1 | The runtime crate, from the listing in section 8, hardened: a build-time error (script does not parse, no `tick` export, `stateBytes` over `MAX_STATE_BYTES`) prints the reason and fails the build; `Date` and `Math.random` removed; `tick` returns 1 on a thrown exception; `Cargo.lock` committed; `oag-ext-abi` as the git dependency the other guests use (`assets/mods/.cargo/config.toml` patches it to this tree) | `assets/mods/examples/js-runtime/` (new: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`). Not named `guest/`, so `just check-guests` (which builds `wasm32-unknown-unknown`) leaves it alone, as it leaves the Javy example alone today | it builds for `wasm32-wasip1`; a script with only `export function tick() {}` costs under 1,000 fuel a tick on `oag-modhost` (measured 421), from a scratch build, not a committed module |
| 2 | The build script: section 3's four steps, taking a script and an output path | `assets/mods/examples/js-runtime/build.sh`, `js-runtime/tools/stub_imports.py` (section 8) | the output has no import and exports `memory`, `oag_abi`, `oag_io`, `oag_state_len`, `tick`; two builds of one script are `cmp`-equal; two different scripts differ only in the data and data-count sections |
| 3 | Port the example to the buffer: `src/main.js` from section 8's script, ABI numbers as `const NAME = n;`; `build.sh` calls step 2; delete `tools/shim.py`, `tools/abi.py`, `tools/abi_word.py` and `wit.wit`; rebuild `ext/wall_penalty_js.wasm` (LFS); the manifest's comment about "55 % to 95 % of the fuel budget" restated; `memory_pages = 24` stays (20 at instantiation) | `assets/mods/examples/wall-penalty-js/` | `OAG_JS_DIFFERENTIAL_TICKS=20000 cargo nextest run -p oag-modhost --test javascript_module`: green, and red again when one byte of the pool's name is changed (the mutation check the port lane used); `cargo nextest run -p oag-raceplay --features extensions --run-ignored only -E 'test(/wall_penalty_ground_truth::/)'`: 5 of 5; fuel printed: nothing to do under 20,000 (13,426), race peak under 50,000 (31,426), field peak under 60,000 (40,281), every slot cursed under 120,000 (95,854) |
| 4 | Hold the floor in the test: the empty tick asserted under 50,000 (the claim), the doc comments no longer say "Javy" or "shim" | `crates/modhost/tests/javascript_module.rs` | `just gate-affected` under the lock (a `.rs` file changed) |
| 5 | The typings and their check: `scripts/gen-ext-typings.py` declares the `oag` global (`buffer: ArrayBuffer`) and the module's two exports in `types/oag-abi.d.ts`, and `--check` still compares the script's `const NAME = n;` lines with the ABI crate | `scripts/gen-ext-typings.py`, `assets/mods/examples/types/oag-abi.d.ts` | `just check-extensions`; a wrong number in `main.js` fails it |
| 6 | The words: the README's JavaScript section gains the new column and a rewritten verdict, this page's status line says built, the mod author skill's "A mod in JavaScript" section and the examples' README are rewritten for the new contract and build (CLAUDE.md: a change to how a mod is built updates the skill in the same change), the handover threads updated or deleted | `docs/architecture/wasm-extensions/README.md`, this page, `assets/mods/examples/.claude/skills/oag-mod-author/SKILL.md`, `assets/mods/examples/wall-penalty-js/README.md`, the two handover threads on JavaScript extensions | `just check-docs`, `just check-handover`, `just check-schemas` |

**Needs on the machine.** A Rust toolchain with `wasm32-wasip1` (the pinned 1.100.0 has
only `wasm32-unknown-unknown` here; 1.99.0 and stable have it; the lane asks before adding a
target to the pinned toolchain), network once for wasi-sdk 24 (or `WASI_SDK`), the
`wasmtime` CLI (50.0.1 is installed; it has `wizer`), binaryen. No Javy.

**Size.** Steps 1 to 3 are the prototype made tidy: small (confidence 75). Steps 4 to 6 are
small and mechanical (confidence 80). The whole lane is S to M (confidence 70), and needs
no judgement a sonnet lane lacks; the one place to stop and ask is a differential failure,
which would mean the script port is wrong, not the runtime.

**ADR.** None for this lane: the host, the ABI, the manifest and the import rule are
untouched, and the module meets
[ADR-0079](../adr/0079-extensions-are-sandboxed-webassembly-guests-behind-two-worlds.md)'s
rules as any guest does. Section 5's items 2 and 3 and the per-module fuel budget each need
one before they are built.

## 7. Confidence

| Claim | Confidence | Evidence, and what would raise it |
| --- | --- | --- |
| The fixed floor falls to about 421 fuel | 90 | measured on the real host with the prototype; the hardening in step 1 adds a few instructions |
| Wall Penalty's empty tick falls to about 13,400, under the 50,000 target | 90 | measured; the script in section 8 |
| Output equals Rust | 90 | differential on 20,000 observations, 5 of 5 ground-truth tests, with the prototype module |
| Javy's per-call cost is bytecode load and module evaluation, not QuickJS entry | 90 | `invoke` in Javy's source at `v9.1.0`; 29,842 against 421 on the same engine |
| The build is byte-reproducible with the clock and randomness fixed before the snapshot | 85 | two builds equal on one machine; not tried on a second machine |
| No host change and no ADR for the wire lane | 90 | the prototype loaded through `Module::compile` unchanged |
| Every non-data section is identical across scripts | 90 | two scripts compared section by section; more scripts would raise it |
| The code and image split of section 5 item 2 works | 60 | not built |
| The Rust tool of section 5 item 1 (`wasmtime-wizer` 50.0.2 with our own host functions) works | 70 | the crate exists at that version; its API was not exercised here, the CLI was |
| Compile time at race load is unchanged (1 s to 2 s) | 60 | not measured; the code section is 932 KB against a module of similar size before |

## 8. The prototype, as measured

Kept here so the wire lane starts from what produced the numbers. Built with
`cargo +1.99.0 build --release --target wasm32-wasip1`; `rquickjs` resolved to 0.12.0 with
`rquickjs-core` and `rquickjs-sys` 0.12.2.

`Cargo.toml`:

```toml
[package]
name = "oag-js-runtime"
version = "0.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
rquickjs = { version = "=0.12.0", default-features = false, features = ["std"] }

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true

[workspace]
```

`src/lib.rs` (the prototype wrote `ABI_WORD` and `IO_LEN` as literals; the wire lane takes
them from `oag-ext-abi`):

```rust
#![allow(static_mut_refs)]
use std::io::Read;
use std::ptr::null_mut;

use rquickjs::{qjs, Context, Function, Module, Object, Runtime, Value};

const ABI_WORD: u32 = (1 << 8) | 6;
const IO_LEN: usize = 4096;

static mut IO: [u8; IO_LEN] = [0; IO_LEN];
static mut CTX: *mut qjs::JSContext = null_mut();
static mut TICK: Option<qjs::JSValue> = None;
static mut STATE_LEN: u32 = 0;

#[export_name = "wizer-initialize"]
pub extern "C" fn init() {
    let mut source = String::new();
    std::io::stdin().read_to_string(&mut source).unwrap();
    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    context.with(|ctx| {
        let raw = unsafe {
            qjs::JS_NewArrayBuffer(
                ctx.as_raw().as_ptr(),
                IO.as_mut_ptr(),
                IO_LEN as _,
                None,
                null_mut(),
                false,
            )
        };
        let buffer = unsafe { Value::from_raw(ctx.clone(), raw) };
        let oag = Object::new(ctx.clone()).unwrap();
        oag.set("buffer", buffer).unwrap();
        ctx.globals().set("oag", oag).unwrap();
        let (module, promise) = Module::declare(ctx.clone(), "main", source)
            .unwrap()
            .eval()
            .unwrap();
        promise.finish::<()>().unwrap();
        let tick: Function = module.get("tick").unwrap();
        let state_len: u32 = module.get("stateBytes").unwrap_or(0);
        unsafe {
            STATE_LEN = state_len;
            TICK = Some(qjs::JS_DupValue(ctx.as_raw().as_ptr(), tick.as_raw()));
            CTX = ctx.as_raw().as_ptr();
        }
    });
    std::mem::forget(context);
    std::mem::forget(runtime);
}

#[no_mangle]
pub extern "C" fn tick() -> u32 {
    unsafe {
        let Some(function) = TICK else { return 2 };
        let result = qjs::JS_Call(CTX, function, qjs::JS_UNDEFINED, 0, null_mut());
        u32::from(qjs::JS_IsException(result))
    }
}

#[no_mangle]
pub extern "C" fn oag_io() -> u32 {
    unsafe { IO.as_mut_ptr() as u32 }
}

#[no_mangle]
pub extern "C" fn oag_abi() -> u32 {
    ABI_WORD
}

#[no_mangle]
pub extern "C" fn oag_state_len() -> u32 {
    unsafe { STATE_LEN }
}
```

The script that gave the right-hand column (13,426):

```js
const EVENTS_AT = 32;
const EVENT_LEN = 16;
const REPLY_AT = 1568;
const COMMAND_LEN = 12;
const POOL_AT = 2724;
const EVENT_WALL_CONTACT = 1;
const COMMAND_SCALE_THRUST = 1;
const COMMAND_FIRE_EFFECT = 6;
const SLOTS = 8;
const CURSE_TICKS = 90;
const PUFF_EVERY = 6;
const SPUTTER = [7, 115, 112, 117, 116, 116, 101, 114];
const io = new Uint8Array(oag.buffer);
const words = new Uint32Array(oag.buffer);
const curses = new Uint16Array(oag.buffer, 544, SLOTS); // STATE_AT
const view = new DataView(oag.buffer);
export const stateBytes = 16;

export function tick() {
  const events = words[6]; // EVENT_COUNT_AT / 4
  for (let i = 0; i < events; i++) {
    const at = EVENTS_AT + i * EVENT_LEN;
    if (io[at] === EVENT_WALL_CONTACT && io[at + 1] < SLOTS) curses[io[at + 1]] = CURSE_TICKS;
  }
  const active = words[5]; // ACTIVE_AT / 4
  let at = REPLY_AT + 4;
  let named = false;
  for (let slot = 0; slot < SLOTS; slot++) {
    let left = curses[slot];
    if (left === 0) continue;
    left -= 1;
    curses[slot] = left;
    if (active & (1 << slot)) {
      io[at] = COMMAND_SCALE_THRUST;
      io[at + 1] = slot;
      view.setFloat32(at + 4, 0.25, true);
      at += COMMAND_LEN;
      if (left % PUFF_EVERY === 0) {
        io[at] = COMMAND_FIRE_EFFECT;
        io[at + 1] = slot;
        io[at + 3] = 0x80; // OWN_FLAG's high byte: the name at pool offset 0
        at += COMMAND_LEN;
        named = true;
      }
    }
  }
  if (named) io.set(SPUTTER, POOL_AT);
  words[REPLY_AT >> 2] = (at - REPLY_AT - 4) / COMMAND_LEN;
}
```

`stub_imports.py` (with names: only those imports, placed after the imports that stay;
without: every WASI import):

```python
"""usage: stub_imports.py in.wat out.wat [name ...]; no names: every WASI import."""
import re
import sys

text = open(sys.argv[1], encoding="utf-8").read()
names = sys.argv[3:]
BODY = {
    "random_get": "(memory.fill (local.get 0) (i32.const 0) (local.get 1))\n(i32.const 0)",
    "clock_time_get": "(i64.store (local.get 2) (i64.const 0))\n(i32.const 0)",
    "environ_sizes_get": "(i32.store (local.get 0) (i32.const 0))\n(i32.store (local.get 1) (i32.const 0))\n(i32.const 0)",
    "environ_get": "(i32.const 0)",
    "fd_close": "(i32.const 0)",
    "proc_exit": "(unreachable)",
}
pattern = re.compile(r'^ \(import "wasi_snapshot_preview1" "(\w+)" \(func (\$\S+) (.*)\)\)$', re.M)
stubs = []
for found in list(pattern.finditer(text)):
    name = found.group(1)
    if names and name not in names:
        continue
    body = BODY.get(name, "(i32.const 8)")
    text = text.replace(found.group(0) + "\n", "", 1)
    stubs.append(" (func %s %s\n%s\n )" % (found.group(2), found.group(3), body))
last = None
for last in re.finditer(r'^ \(import .*$', text, re.M):
    pass
at = last.end() if last else re.search(r'^ \((global|memory) ', text, re.M).start() - 1
text = text[:at] + "\n" + "\n".join(stubs) + text[at:]
open(sys.argv[2], "w", encoding="utf-8").write(text)
```

The commands, `runtime.wasm` being the cargo output:

```sh
wasm-dis runtime.wasm -o runtime.wat
python3 -I stub_imports.py runtime.wat fixed.wat random_get clock_time_get
wasm-as --all-features fixed.wat -o fixed.wasm
wasmtime wizer -S cli=y fixed.wasm -o snapshot.wasm < main.js
wasm-dis snapshot.wasm -o snapshot.wat
python3 -I stub_imports.py snapshot.wat final.wat
wasm-as --all-features final.wat -o wall_penalty_js.wasm
```

The raw module imports ten WASI functions: `random_get`, `environ_get`,
`environ_sizes_get`, `clock_time_get`, `fd_close`, `fd_fdstat_get`, `fd_read`, `fd_seek`,
`fd_write`, `proc_exit`.

## 9. Not settled

- **Compile time at load** with the new module: not measured. A release-profile timing of
  `Module::compile` on both modules settles it.
- **A leaner context.** The prototype uses `Context::full`. Leaving out intrinsics a mod
  never needs (`Date`, `eval`, `RegExp`, `Proxy`, promises) would shrink the 203 KB image
  and the module; not measured, and not needed for the fuel target.
- **Stubs at link time.** Defining the WASI import symbols in the runtime crate might
  remove both rewrites and binaryen from the build. Not tried; the rewrite is proven.
- **A promise job loop.** The prototype has none, so an `async` tick would do nothing after
  its first `await`. The wire lane refuses a `tick` that returns a promise (return 1) rather
  than running jobs; chosen, not measured.
- **The fuel of a thrown exception** and of a script that allocates in `tick` (objects,
  strings, closures): not measured. Wall Penalty allocates nothing per tick.
