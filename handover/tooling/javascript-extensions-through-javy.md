---
status: parked
titles: [all]
updated: 2026-10-30
summary: "JS extensions via Javy load and run on branch wasm-javy (1.37 MB, 155k-178k of 200k fuel); parked by the maintainer before the fuel decision"
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

- A JavaScript Wall Penalty built with Javy, its WASI preview-1 imports rewritten onto the
  `oag` namespace by the examples' `tools/shim.py`, plus TypeScript typings generated from
  the ABI (`scripts/gen-ext-typings.py`).
- Measured: module 1.37 MB; 155k-178k fuel per tick of the 200k budget; the module faults
  once roughly five or six craft are cursed at once. Load takes about 1.1 s.
- Findings are in `docs/architecture/wasm-extensions/README.md` on the branch; tests in
  `crates/modhost/tests/javascript_module.rs` and `wall_penalty_ground_truth.rs`.

## Open

The fuel headroom is the blocker. Three options were offered, none chosen:

1. Measure a Wizer/Javy pre-initialised snapshot first (lead's recommendation): QuickJS
   start-up per tick is the likely bulk of the fuel, and a snapshot may remove it.
2. A per-module fuel budget declared in `oag-mod.toml`.
3. A compiled-module cache: fixes the 1.1 s load only, unrelated to fuel.

## Next Steps

1. Maintainer reviews `wasm-javy` and reopens or drops the thread.
2. If reopened: keep `wasm-javy` current with `wasm-extensions` (merge it in), then
   measure option 1 before building 2 or 3.
