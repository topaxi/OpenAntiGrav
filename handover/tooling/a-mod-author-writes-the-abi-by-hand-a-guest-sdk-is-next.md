---
status: ready-to-wire
titles: [all]
updated: 2026-10-30
summary: "SDK rev 3 + track module + Hillside rain + track props landed locally; open: Blender-side props run, check/run tool, C header, C++ example, template, git dep, MODDED rule"
categories: [gameplay]
size: M (55)
difficulty: medium (60)
---

# A mod author writes the ABI by hand: a guest SDK is next

**Maintainer, 2026-10-29:** the API a modder has to code against is "quite cumbersome";
build a cleaner, more ergonomic one, Rust first, friendly to other languages too. This
thread is the review of what the two example mods write today and the design the lane
builds. Branch rule unchanged: everything here lands on `wasm-extensions`, never on
main, never pushed.

The wire ABI itself (one I/O buffer, three exports, core wasm, no component model) is
settled by `docs/architecture/wasm-extensions/README.md` section 2 and
`determinism-and-state.md` ("Web and iOS") and is **not** what this thread changes. The
problem is that the buffer's bytes are the *only* interface a mod author has.

## Landed since (2026-10-30, lane `hillside-rain`, local `wasm-extensions` branch)

The `[extension.track]` kind (a module bound to one track file of its mod), the craft
record's `lap` and `laps_target` (`Craft::lap()`, `laps_target()`), three race-wide output
commands (`set-weather`, `set-sun`, `grade-sky`) and the third example, Hillside's rain;
ABI 1 is revision 3. Open items below are unchanged. New, for whoever takes the tooling
lane: an outputs-only module perhaps should not mark a race MODDED, and a track module in
a `.zip` mod has no path to match (`Scope::Track(None)`), so it never loads.

## Landed (2026-10-29, lane `ext-sdk`)

Layers 0 and 1 below, and the examples on them; the README's "And the SDK" entry
has the pieces, sizes and fuel:

- ~~`oag-ext-abi`: the layout moved out of `oag_modhost::abi`, `REVISION` with its
  folded-layout test, `oag_abi() = (ABI << 8) | REVISION` refused by name, optional
  `oag_state_len`.~~ `state_bytes` stays **required** in the manifest (the lane's
  brief over this thread's "optional"); the export, when present, must equal it.
- ~~`oag-ext`: `Observation`, `Craft`, `Reply`, `blob!`, `Extension`, `export!`,
  `test::Race`.~~ Deviation: `Observation::craft` returns `Option<Craft<'_>>` by
  value, a view over the record, not `Option<&Craft>`: decoding every slot up front
  put Wall Penalty at 2.25x its old fuel; the view brought it under.
- ~~Wall Penalty and Energy Turbo rewritten (mod-examples branch `ext-sdk`); ground
  truths unchanged; sizes 627 to 638 and 1,774 to 2,395 bytes.~~
- ~~Decision 4's committed-examples test~~: `oag_raceplay`'s
  `tests::extension_examples` compiles each committed `.wasm` in
  `just check-extensions` and runs a full grid under the fuel budget.
- ~~Docs: README "What is built", `docs/tools/mods.md`, `hooks.md`'s command table.~~

Findings 1 to 8 below are closed by it; finding 9 is half closed (a mod's logic runs
under `cargo test`; no tool runs a built `.wasm` yet).

## What a mod author writes today (the review, 2026-10-29)

Reviewed: `mod-examples` at `43d45b9` (branch `wasm-extensions`), `wall-penalty/guest/src/lib.rs`
(119 lines) and `energy-turbo/guest/src/lib.rs` (271 lines), against the host's
`crates/modhost/src/abi.rs` on this branch.

1. **Every layout constant is copied by hand from `oag_modhost::abi`.** Wall Penalty carries
   13 lines of them (`IO_LEN`, `ACTIVE_AT`, `EVENT_COUNT_AT`, `EVENTS_AT`, `STATE_AT`,
   `REPLY_AT`, codes); Energy Turbo 21, and derives `CRAFTS_AT` with its own arithmetic
   (`REPLY_AT + 4 + 12 * MAX_COMMANDS`). Nothing checks the copies. The ABI is edited in
   place before the first release (README 4.3), and it already moved on this branch
   (craft records and six commands were added after Wall Penalty was built). The 627-byte
   Wall Penalty module kept working only because every edit appended. The host checks
   `oag_abi() == 1` and nothing else; a module built against a stale layout runs and
   misreads silently. The gate would not notice either: `oag_raceplay`'s
   `tests::extensions` use WAT guests, and the committed `.wasm` files are loaded only by
   the `#[ignore]`d ground-truth tests.
2. **Raw byte plumbing in every mod.** `static mut IO`, `addr_of_mut`, `read_volatile`,
   `write_volatile`, and `get_u8`/`put_u8`/`get_u32`/`put_u32`/`get_f32` are written out
   in both files (24 to 28 lines each, identical). `unsafe` in a mod that needs none.
3. **The state blob is serialised by hand.** Wall Penalty's `countdown`/`set_countdown`
   split a `u16` into bytes; Energy Turbo's `load`/`store` lay a five-field struct out
   at offsets 0, 4, 6, 7, 8 with "2 spare", and `oag-mod.toml`'s `state_bytes = 96`
   repeats `12 x 8` in a comment. Nothing ties the struct to the manifest number.
4. **Commands are positional tuples.** `reply.push(kind, slot, arg, value, word)`: a
   `u8` code plus three fields whose meaning depends on the code. `pickup-roll` puts the
   weapon in the `u16` and the period in the `u32`; `pickup-tint` puts the colour in the
   `u32`; `scale-thrust` uses the `f32`. The host decodes them into a typed `Command`
   enum; the guest sees none of that.
5. **Magic numbers.** `TURBO = 5` ("`Weapon::ALL`'s Turbo, plus one"), `EXTENSION = 255`,
   cue codes `0..6`, effect codes `0..2`, flag bits `1`, `2`, `4`, the colour as a packed
   `u32` with red low.
6. **The host's rules are the author's job.** A command for an inactive slot, two
   `scale-thrust` for one slot, a `shield-delta` past the pool, a non-finite `f32`, or a
   65th command faults the module mid-race. Wall Penalty has a comment explaining why it
   masks `active` before sending. The guest gets no help; a mistake is a race-time
   fault, not a compile error.
7. **The reply count is a trailing side effect.** `put_u32(REPLY_AT, count)` at the end
   of `tick`; forgetting it sends no commands and nothing says so.
8. **Entry-point ceremony per mod.** `#![no_std]`, three `#[unsafe(no_mangle)] extern "C"`
   exports, a `#[panic_handler]`, `crate-type = ["cdylib"]`, a `.cargo/config.toml` with
   `-zstack-size=16384`, `panic = "abort"`, and a `build.sh` that copies the artefact.
   The two mods' `Cargo.toml`, `config.toml` and `build.sh` differ by the package name
   only.
9. **No way to run a mod outside the game.** The author's loop is build, copy, `just
   play-mods`, race. `oag_modhost::Module::compile` + `tick` exist but are not reachable
   as a tool, and the mod's logic cannot be unit-tested natively because it is welded
   to the buffer.

Roughly **half of Wall Penalty and two fifths of Energy Turbo** is ABI plumbing that is
identical between the two files. The logic a modder actually wants to write (a
countdown per slot; a pool, a burst and a payback) is 35 and 150 lines.

## What a mod should look like

The target, as the Wall Penalty author would write it (names are a sketch; the lane
settles them):

```rust
#![no_std]
use oag_ext::prelude::*;

const CURSE_TICKS: u16 = 90;
const CURSE_FACTOR: f32 = 0.25;

oag_ext::blob! {
    /// One countdown per grid slot. Laid out in field order, little-endian, no padding.
    struct Curses { countdown: [u16; MAX_SHIPS] }
}

struct WallPenalty;

impl Extension for WallPenalty {
    type State = Curses;

    fn tick(obs: &Observation, state: &mut Curses, reply: &mut Reply) {
        for event in obs.events() {
            if let Event::WallContact { slot } = event {
                state.countdown[slot] = CURSE_TICKS;
            }
        }
        for slot in Slot::all() {
            let left = &mut state.countdown[slot];
            if *left == 0 {
                continue;
            }
            *left -= 1;
            // A command needs a `&Craft`: proof the slot holds one this tick.
            if let Some(craft) = obs.craft(slot) {
                reply.scale_thrust(craft, CURSE_FACTOR);
            }
        }
    }
}

oag_ext::export!(WallPenalty);
```

What changed: no constants copied, no `unsafe`, no bytes, the blob is a struct, a
command cannot name an empty slot, the reply count is written by the glue, and the
whole `impl` runs natively under `cargo test` because nothing in it is wasm-specific.
Energy Turbo's `reply.push(PICKUP_TINT, slot, 0, 0.0, RED)` becomes
`reply.pickup_tint(craft, Rgba::new(0xE0, 0x20, 0x20, 0xFF))`,
`reply.push(PICKUP_ROLL, 0, TURBO, 0.0, 2)` becomes `reply.pickup_roll(Weapon::Turbo, 2)`,
`flags & CRAFT_FIRE_HELD != 0` becomes `craft.fire_held()`, and
`pickup == EXTENSION` becomes `craft.pickup() == Pickup::Ours`.

## The shape: three layers over the same bytes

**Layer 0, the layout, defined once: `oag-ext-abi`** (new workspace crate, `no_std`, zero
dependencies, `#![forbid(unsafe_code)]`). The one source of truth for ABI 1: offsets,
sizes, caps, codes, flag bits, and `Craft`, `Event`, `Command` with their
encode/decode over `&[u8]`/`&mut [u8]`. `oag-modhost` **depends on it** and loses its
own `abi.rs` (the `write_observation`/`read_reply` bodies move; the host keeps the
checks that need the observation, like "slot holds a craft"). The guest SDK depends on
it too. Then the layout cannot drift between host and guest: there is one definition
and both sides compile against it. The doc table at the top of today's `abi.rs` is
generated from the same constants (a test compares them, as `check-status` does for
`names.tsv`).

- **A layout revision.** `oag_ext_abi::REVISION: u32`, bumped by hand on any edit to the
  layout, with a unit test that folds every offset, size and code into a committed
  constant so a forgotten bump fails the gate. The guest returns it: `oag_abi()`
  returns `(ABI << 8) | REVISION` (pre-release in-place edit of ABI 1; `[extension] api`
  stays the major). The host refuses a differing revision **by name** in the load
  report ("built for ABI 1 rev 3, this host is rev 4"). This closes finding 1 and
  makes a stale committed example `.wasm` a visible failure.
- **The state length travels with the module.** A fourth export, `oag_state_len() ->
  u32`, emitted by the glue from `State::BYTES`. The manifest's `state_bytes` becomes
  optional; when present it must equal the export (a mismatch is refused by name).
  Closes finding 3's second half.

**Layer 1, the Rust guest SDK: `oag-ext`** (`no_std`, no `alloc`, depends on
`oag-ext-abi` only; builds on every target, exports only under
`cfg(target_arch = "wasm32")`). Not `oag-mod`: that name is taken by the mod-package
crate (`crates/mod`, ADR-0066). Pieces:

- `Observation`: `tick()`, `crafts()` (active slots only, each with its `Slot`),
  `craft(slot) -> Option<&Craft>`, `events()` (an iterator; unknown kinds skipped, count
  clamped at `MAX_EVENTS`), `Craft::{is_person, fire_held, is_racing, shield,
  shield_max, speed_kmh, pickup}`.
- Typed vocabulary: `Slot` (indexes arrays; `Slot::all()`), `Weapon` (the title's list
  as an enum, `Turbo` and not `5`), `Pickup::{None, Weapon(Weapon), Ours}`, `Cue`,
  `Effect`, `Rgba`. All in `oag-ext-abi`, shared with the host.
- `Reply`: one method per command, taking `&Craft` where a slot must hold a craft;
  `pickup_roll` and the other race-wide ones take none. Returns `Result<(), Refused>`
  where the host would fault (full reply, second `scale_thrust` on one craft, factor
  out of `[0, 2]`, `shield_delta` past `craft.shield_max()`, non-finite). The host's
  fault path stays the backstop; the SDK makes the common mistakes unrepresentable or
  an `Err` the author sees in a native test. The count is written on return.
- `blob!`: a declarative macro (no proc-macro crate yet; see "Decisions") defining a
  plain-data struct with `State::BYTES`, `from_bytes`, `to_bytes`, little-endian, field
  order, no padding, and a `const` assert that it fits `MAX_STATE_BYTES`. Fields:
  integers, `f32`, `bool`, fixed arrays of those, nested `blob!` structs.
- `Extension` trait: `type State: Blob; fn tick(obs, state, reply)`. The trait docs carry
  the two rules an author must know and the bytes hide: the instance is fresh every tick
  (nothing but `State` survives) and events are one tick late.
- `export!`: the `IO` static, `memory`, `oag_abi`, `oag_io`, `oag_state_len`, `tick`, the
  panic handler (`unreachable`), all behind `cfg(target_arch = "wasm32")`.
- A `test` feature (std): `oag_ext::test::Race` builds an `Observation` from typed parts
  (`with_craft(slot, Craft {..})`, `event(Event::WallContact { slot })`), runs
  `WallPenalty::tick` **natively**, and returns the typed commands and the next state. No
  wasm toolchain needed for `cargo test` in the mod's own crate.

**Layer 2, tooling for authors of any language.**

- `oag-tools`: `oag-ext check <mod dir>`: reads `oag-mod.toml`, compiles the module with
  the real `oag_modhost::Module::compile`, reports exports, ABI revision, state length
  against the manifest, memory pages, module size, and fuel for a zero-event tick on a
  full grid against `FUEL_PER_TICK`. The first thing a C or Zig author runs.
- `oag-tools`: `oag-ext run <mod dir> --ticks N [--events ...]` (or a flag on `check`):
  the same harness fed a scripted observation, printing the typed commands per tick.
  Replaces "build, copy, `just play-mods`, race" for the first ten minutes of a mod.
- A template in `mod-examples` (`template/` or `cargo generate`): `Cargo.toml`,
  `.cargo/config.toml` (the 16 KiB stack), `build.sh`, `oag-mod.toml`, `src/lib.rs` on
  `oag-ext`. Finding 8.
- Generated bindings from layer 0: `just gen-ext-abi` emits `oag_ext.h` (offsets, codes,
  packed structs, inline `oag_obs_craft(i)`/`oag_reply_scale_thrust(...)`) for C, C++
  and Zig, and the doc table; `just check-ext-abi` fails when either is stale (the
  `gen-status`/`check-status` pattern). An AssemblyScript or TypeScript layout module
  from the same generator once a JS-side example exists.

## Other languages

The owed multi-language examples (Rust, C++, JavaScript through Javy, Python through
Pyodide, Lua through Wasmoon or lua2wasm; `custom-race-modes-and-campaigns.md`,
"WebAssembly extensions design") split in two. **Compiled languages** (C, C++, Zig,
AssemblyScript, Go through TinyGo) consume the generated header or module from layer 0
and need nothing else; the C++ example is this lane's second example after the Rust
rewrite, as proof the header is enough. **Interpreted runtimes** (Javy, Pyodide,
Wasmoon) are a separate feasibility finding (they bring WASI or an Emscripten host
and are tens of MB) and stay in that thread; the SDK makes no concession to them.

What makes the ABI "friendly" to other languages is already true of its bytes (fixed
offsets, little-endian, no component model) and is kept: the layer 0 generator just
stops every author re-deriving the table. One refinement worth taking while the ABI
is in-place editable: **every command record carries its arguments in named positions
per kind in the header**, so `oag_ext.h` can expose `struct oag_cmd_pickup_roll { u8
kind; u8 zero; u16 weapon; u32 zero; u32 every; }` over the same 12 bytes instead of
one generic record, and the documentation table names them per kind.

## Decisions for the lane (recommendation first)

1. **Where the SDK lives.** In the engine workspace (`crates/ext-abi`, `crates/ext`), so
   the host and the guest compile against one definition and the gate covers both.
   `mod-examples` takes `oag-ext` as a **git dependency on the engine repository with a
   pinned `rev`** (the submodule layout would allow a `path`, but a stand-alone clone
   must build). The pin is bumped with each ABI revision; the committed `.wasm` is
   refused by name until it is.
2. **`blob!` as a `macro_rules!` first**, a `#[derive(Blob)]` proc macro only when a
   second need for a proc-macro crate shows up (the maintainer's two-enums rule for
   macros). The declarative form covers both examples.
3. **Refusals: `Result`, not panics.** A `Reply` method returns `Err(Refused)` and sends
   nothing; the author decides. A panic in the guest is `unreachable` and faults the
   module for the race, which is the right outcome for a bug but the wrong one for a
   bounds case the author can handle.
4. **The gate builds the examples' guests?** No: `wasm32-unknown-unknown` is not in CI's
   toolchain. Instead a non-ignored test in `oag-raceplay` (behind `extensions`) loads
   each committed `assets/mods/examples/*/ext/*.wasm` with `Module::compile` and
   asserts the ABI revision matches, so a stale example fails `just check-extensions`.
5. **`oag_modhost::abi` after the move.** Items move to `oag-ext-abi` and call sites
   follow; no `pub use` compatibility re-exports (unpublished crates, maintainer's rule).
   Renaming `oag-modhost` to `oag-ext-host` is optional and not worth the churn now.
6. **Dependency rules.** `oag-ext-abi` has no dependencies and may be reached by any
   crate; `oag-ext` is a guest crate and **no engine crate depends on it** (add it to
   `scripts/check-dependency-rules.py` beside the `oag-game` rule); `oag-ext` builds
   natively for its own tests only. `cargo shear`, `check-size`, `check-determinism` (the
   `f32` handling is bit copies, nothing transcendental) apply as usual.

## Acceptance

- Wall Penalty and Energy Turbo rewritten on `oag-ext` in `mod-examples`, each under 60
  and 160 lines of logic with **no `unsafe`, no offsets and no codes**; module sizes
  within 2x of today's 627 and 1,774 bytes; fuel per tick on a full grid within 2x of
  today's (the `Dev` overlay's `mods` block shows both).
- `wall_penalty_ground_truth` and `energy_turbo_ground_truth` pass unchanged against the
  rewritten modules. Wall Penalty's blob (`[u16; 8]`) is byte-identical, so its race
  hashes are too; Energy Turbo's record drops its two spare bytes (12 to 10 per slot)
  unless the rewrite keeps a `_spare: u16`, in which case its hashes hold as well.
  Either way the vanilla fold in `tests::extensions` does not move.
- `oag-modhost` has no layout of its own; `oag-ext-abi`'s revision test and the host's
  refusal-by-name test exist; `oag-ext`'s native `test::Race` runs both examples' logic
  under plain `cargo test`.
- `oag-ext check` runs on both examples and on a C module built from `oag_ext.h` (the
  C++ example, the third mod in `mod-examples`).
- `docs/architecture/wasm-extensions/README.md` "What is built" gains an "And the SDK"
  entry; `docs/tools/mods.md` points a mod author at `oag-ext` and `oag-ext check`
  first; `hooks.md`'s command table names the SDK method beside each command.

## Open

- **Track properties' Blender side never ran in Blender** (lane `ext-track-props`,
  2026-10-30: this machine's `blender` fails to start, `libsycl.so.9` missing). The
  add-on's pure-Python writer and reader (`trackfile.py`: `[props]`, pad `oag_props`,
  `oag/triggers`) were run outside Blender and the engine reads what they write; the
  scene paths (`scene.mod_props` on pad and trigger Empties and the scene, trigger
  collect and import, the panel rows) are unexercised. Next: add a trigger and a few
  `mod_*` properties to `roundtrip.py loop`, run `just blender-fixtures --update`, and
  check `crates/trackfile/tests/blender.rs` reads them.
- **The examples' SDK dependency is a `path`** into the engine checkout the
  submodule sits in (`../../../../../crates/ext`, five levels from `guest/`). A
  stand-alone clone of `mod-examples` does not build until it is a git dependency on
  the engine with a pinned `rev`, which waits until the `wasm-extensions` branch is
  public. Bump the pin with each `REVISION`.
- **No `oag-ext check` / `run` tool** (layer 2): exports, revision, state length,
  memory, size and fuel for a built module, and a scripted run printing the typed
  commands. `tests::extension_examples` holds the fuel harness to start from.
- **No generated C header, no C++ example.** `oag_ext_abi` is the source to generate
  `oag_ext.h` from (`just gen-ext-abi` / `check-ext-abi`, the `gen-status` pattern),
  with per-kind command structs; the doc table at the top of `oag_ext_abi` is still
  written by hand.
- **No template** in `mod-examples` (the two `guest/` directories differ by name only).
- Whether the manifest keeps `memory_pages` as a budget while the module's own memory
  type carries the need: a note for the manifest thread, not the SDK.
- A generated TypeScript/AssemblyScript layout module waits for a JS-side example.

## Next Steps

1. `oag-tools`: `oag-ext check <mod dir>` and `oag-ext run`, on `oag_modhost::Module`
   and the full-grid observation in `tests::extension_examples`.
2. `just gen-ext-abi` / `check-ext-abi`: `oag_ext.h` and the doc table from
   `oag_ext_abi`; then the C++ example in `mod-examples` from the header, with its size
   and fuel beside the Rust ones, checked by `oag-ext check`.
3. The `mod-examples` template on `oag-ext`.
4. When the branch is public: the examples' git dependency, pinned.
5. Move what is left of this thread into the README and delete it.
