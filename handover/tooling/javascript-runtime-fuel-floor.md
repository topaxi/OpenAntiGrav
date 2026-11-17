---
status: ready-to-wire
titles: [all]
updated: 2026-11-17
summary: "Build the planned JS guest runtime (script in the snapshot, I/O buffer mapped): floor 110,770 to about 13,400 fuel; prototype measured, six steps"
categories: [gameplay]
size: M (70)
difficulty: easy (70)
---

# A JavaScript extension's fuel floor

The plan is [`docs/architecture/wasm-extensions/javy-plugin-plan.md`](../../docs/architecture/wasm-extensions/javy-plugin-plan.md):
where the floor goes, the options ranked, the design, determinism and identity, how it fits
"ship Javy from our side", the steps, the confidence of each claim, and the prototype's
source. Read it first; nothing below repeats it.

It belongs to the parked Javy work
([javascript-extensions-through-javy.md](javascript-extensions-through-javy.md)): the branch
is not merged and the maintainer reviews it first. This thread is ready to wire **on that
branch** once the maintainer says to continue.

## What is known

- A prototype (2026-11-17) ran on the unchanged host: 421 fuel for an empty tick function
  (Javy 29,842), Wall Penalty 13,426 with nothing to do (110,770), 31,426 race peak
  (121,833), 40,281 in a 12-craft field (131,543), 95,854 with every slot cursed and puffing
  (188,451). Differential equal on 20,000 observations; `wall_penalty_ground_truth` 5 of 5.
- It is not a Javy plugin: a small Rust guest on `rquickjs`, snapshotted with
  `wasmtime wizer`. No Javy in the build.
- No host, ABI or manifest change, so no ADR for this thread.

## Open

1. Nothing of the plan is built: the prototype was scratch and is listed in the plan's
   section 8.
2. The pinned toolchain (1.100.0) has no `wasm32-wasip1` target on the maintainer's
   machine; 1.99.0 and stable do. Ask before adding it.
3. Not settled, each named in the plan's section 9: compile time at load, a leaner QuickJS
   context, link-time WASI stubs, the browser engine.
4. The follow-ups that do need an ADR (compile QuickJS once per process, a mod that ships
   only its script, a per-module fuel budget) are the plan's section 5. Not part of this
   thread's steps.

## Next Steps

1. The plan's section 6, steps 1 to 6 in order, each with its acceptance test.
2. Report the fuel table again from the committed module, and whether the build is
   byte-identical twice.
3. Then delete this file and fold what stays open into the Javy thread.
