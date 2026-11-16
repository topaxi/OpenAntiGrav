---
status: parked
titles: [all]
updated: 2026-11-16
summary: "JS extensions via Javy: Wall Penalty ported to current main on branch javy-rebase (1.37 MB, 111k-188k of 200k fuel after the port, was 155k-228k); maintainer review before any merge"
categories: [gameplay]
size: M (40)
difficulty: medium (40)
---

# JavaScript extensions through Javy

**Maintainer, 2026-10-30: parked.** The work lives on branch `wasm-javy` (engine and the
`assets/mods/examples` submodule), on top of `wasm-extensions`, and is not merged into it.
Nothing is drawn from this thread until the maintainer reopens it after their own review
of the branch.

## What landed on the branch

- A JavaScript Wall Penalty built with Javy 9.1.0 (still the newest release), its WASI preview-1
  imports rewritten in-module by the examples' `tools/shim.py`, plus TypeScript typings generated
  from the ABI (`scripts/gen-ext-typings.py`, which now reads every `crates/ext-abi/src` file and
  `CommandKind`).
- Ported onto `main` (wasmtime `=50.0.2`, ABI 1 revision 6) on branch `javy-rebase`, not merged:
  module 1.37 MB; 110,770 fuel with nothing to do, 121,833 peak in a race, 131,543 in a 12-craft
  field, 188,451 with every slot cursed and puffing, of the 200k budget. Before the port, on the old
  ABI: 155k, 168k, 178k, 228k (out of fuel). Load takes about 1 s to 2 s of Cranelift.
- Findings are in `docs/architecture/wasm-extensions/README.md`; tests in
  `crates/modhost/tests/javascript_module.rs` and `wall_penalty_ground_truth.rs`.

## Open

The fuel headroom is the blocker. Options, none chosen:

1. The script's top level runs again on every tick and is billed (a top-level `const` about 1,400
   fuel, a `Uint8Array` about 5,400): a Javy build that evaluates the script inside the Wizer
   snapshot, or a custom plugin (NOTES 1c), would remove that and QuickJS's 30k entry cost.
2. A per-module fuel budget declared in `oag-mod.toml`.
3. A compiled-module cache: fixes the 1 s to 2 s load only, unrelated to fuel.

## Next Steps

1. Maintainer reviews `javy-rebase` (engine and the `assets/mods/examples` branch of the same
   name) and reopens or drops the thread.
2. If reopened: measure option 1 before building 2 or 3.
