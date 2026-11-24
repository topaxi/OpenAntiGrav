# The vendored `wgpu-hal`

`vendor/wgpu-hal` is `wgpu-hal` 30.0.2 as published on crates.io with one change: the
Vulkan backend keeps the SPIR-V of every pipeline stage it writes, per device, and hands
it to the next pipeline built with the same stage. The root `Cargo.toml` puts it in place
of the published crate with `[patch.crates-io]`. Every renderer and every GPU test uses it.

## Why we carry it

A shader module with pipeline-overridable constants (`override` in WGSL) stays naga IR in
`wgpu-hal` until a pipeline is created. For every stage of every pipeline,
`Device::compile_stage` then resolves the override constants
(`naga::back::pipeline_constants::process_overrides`, which revalidates the module) and
writes SPIR-V (`naga::back::spv::write_vec`). Our mesh pipelines are many variants of one
module (`mesh.wesl`: blend, cull, depth and vertex-layout variants of a material, whose
constants come from the model and the player's settings), so most of that work repeats a
stage already written. A Pulse PSP race walk calls `compile_stage` 308 times for 126
distinct stages, and before the change the call was about 25 % of the walk's user
instructions.

Writing SPIR-V at build time cannot reach those stages (their constants are run-time data,
and naga 30's SPIR-V writer emits no specialization constants), and resolving the
constants in our own crate before `wgpu` sees the module reached only 6.2 %:
`docs/architecture/load-time.md`, "Fifth round" and "Sixth round". The memo is the change
that clears the 10 % bar, and it can only live inside `wgpu-hal`: the SPIR-V depends on
the device's writer options and on the pipeline layout's binding renumbering, both private
to the backend.

Measured (`load-time.md`, "Seventh round"): the race walk falls from 16.69 to 14.20 G
user instructions (-14.9 %), the bare race launch from 9.10 to 7.17 G (-21.3 %), a
race-spawning ground-truth test from 27.09 to 16.96 G (-37.4 %). Frames are
pixel-identical.

## What changed

All in `vendor/wgpu-hal/src/vulkan/`; `vendor/wgpu-hal/NOTICE.md` lists the same.

- `stage_memo.rs` (new): `StageMemo`, a mutex-guarded map from `StageKey` to SPIR-V words.
- `mod.rs`: `ShaderModule::Intermediate` gains `memo_id: u64`, and `Device` gains
  `stage_memo`.
- `adapter.rs`: the device starts with an empty memo.
- `device.rs`: the body that resolved the constants and wrote SPIR-V moved unchanged into
  `write_intermediate_stage`. `compile_stage` looks the stage up first, and a miss writes
  it and inserts the result. `create_shader_module` gives each intermediate module a fresh
  id, and `destroy_shader_module` drops that id's entries.

The Vulkan shader module is still created from the words and destroyed for each pipeline,
exactly as before. Only the naga work is skipped. No `unsafe` was added.

**The key** is everything that decides the words:
- the module's id, which is a per-device counter and never an address, because a freed
  module's address can be reused. The id also covers the module's `runtime_checks` and
  debug source;
- the shader stage and the entry point;
- the override constants, sorted by name, with values compared by bit pattern;
- `zero_initialize_workgroup_memory`;
- the layout's binding map, the `(group, binding)` to Vulkan binding renumbering.

The device's own writer options are fixed per device, and the memo lives on the device.
`StageKey::new` destructures `ProgrammableStage` exhaustively, so a field that `wgpu` adds
there later fails to compile instead of silently missing from the key.

**The bound**: entries go when their module is destroyed, and the whole memo goes with the
device. Total SPIR-V is capped at 32 MiB (`MAX_BYTES`); an insert past the cap is skipped,
never evicting. The Pulse PSP race walk produces 126 distinct keys over its whole run, front-end
modules included, which are freed with their modules.

**Inert elsewhere**: only the Vulkan backend changed. The browser build (`wasm32`,
WebGPU) does not link `wgpu-hal` at all (`cargo tree --target wasm32-unknown-unknown -p
oag-game -i wgpu-hal` prints nothing). Metal, Dx12 and GL compile unchanged code.

## How it is tested

- `oag-gpu`'s `stage_memo_tests` (run in the gate, on every adapter the machine has) build
  pipelines from one module with an `override` and read back what each one wrote:
  - two layouts that put the shader's binding 1 at different Vulkan bindings;
  - two override values on the same layout;
  - a module created after another was dropped.
  Each test fails when its part of the key is removed (checked by hand: binding map,
  constants, module id).
- `vendor/wgpu-hal`'s own unit tests in `stage_memo.rs` (key inequality, eviction by
  module, the cap) run standalone, not in the gate:
  `cp ~/.cargo/registry/src/index.crates.io-*/wgpu-hal-30.0.2/Cargo.lock vendor/wgpu-hal/ && cargo test --locked --manifest-path vendor/wgpu-hal/Cargo.toml --features vulkan --lib stage_memo`
  (the workspace's `exclude = ["vendor"]` is what lets it build on its own; delete the copied lockfile afterwards).

## When to drop it

Drop the vendored crate, the `[patch.crates-io]` line and the `exclude` entry once a
`wgpu` release contains an equivalent stage cache. When upgrading `wgpu` before then:
1. Copy the new `wgpu-hal` from the registry over `vendor/wgpu-hal` in a commit of its own.
2. Re-apply this change. `git show` of the commit that added `stage_memo.rs` is the patch.
3. Run `oag-gpu`'s `stage_memo_tests` and the measurement in `load-time.md`,
   "Seventh round".

If the upgrade has moved `compile_stage` too far for the patch to apply, measure without
it first: a newer naga may already make the cost smaller.

## Upstream pull request text

The project has not opened it. This is the text for the maintainer to send to
<https://github.com/gfx-rs/wgpu> with the diff of `vendor/wgpu-hal/src/vulkan/`:

> **vulkan: reuse the SPIR-V of identical pipeline stages per device**
>
> A shader module with override constants (or any module under
> `Workarounds::SEPARATE_ENTRY_POINTS`) stays IR in `wgpu-hal` until pipeline creation,
> and `Device::compile_stage` runs `process_overrides` (with its revalidation) and the
> SPIR-V writer for every stage of every pipeline. Applications that build many pipelines
> from one module (blend, depth or vertex-layout variants of one material) repeat that
> work for stages whose SPIR-V is identical.
>
> This adds a per-device memo of the written SPIR-V, keyed on everything the writer
> reads: a per-device module id (not an address), the stage, the entry point, the
> override constants (sorted, compared by bits), `zero_initialize_workgroup_memory` and
> the pipeline layout's binding map. Device-level writer options are fixed per device. A
> module's entries are dropped in `destroy_shader_module`, the memo is dropped with the
> device, and its total size is capped (32 MiB; inserts past the cap are skipped). The
> `vk::ShaderModule` is still created and destroyed per pipeline as before, so driver
> behaviour is unchanged. The lock is never held across naga work. Two threads that miss
> on the same key both write it, and the second insert replaces equal words.
>
> The existing body moved verbatim into `write_intermediate_stage`. `StageKey::new`
> destructures `ProgrammableStage` exhaustively, so a new field must be considered for
> the key.
>
> Measured on a game that builds about 300 pipeline stages from a few modules (126
> distinct; Linux, RADV, warm Mesa shader cache, `perf stat -e instructions:u`): the
> process's user instructions to its first race frame drop by 21 %, and
> `compile_stage` falls from 25 % to 11 % of a profile. The output is pixel-identical.
>
> Tests: unit tests for the key and the bound. An application-side test builds compute
> pipelines from one module with two layouts that renumber a binding differently, with
> two override values, and after replacing a module, and checks that each pipeline writes
> through its own bindings and constant.
>
> `ShaderModule::Intermediate` gains a `memo_id` field. That is a breaking change for
> code that constructs the variant outside `wgpu-hal`, which nothing in the repository
> does.

A related naga issue, separate from this PR: naga 30.0.2's `process_overrides` documents
its result as having "an empty `overrides` arena", but it restores the arena before
revalidating (`back/pipeline_constants.rs`). A caller that resolves first and hands over
the result is resolved again (`load-time.md`, "Sixth round").
